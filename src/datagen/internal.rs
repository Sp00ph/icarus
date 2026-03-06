use std::{
    fs::File,
    io::Write,
    num::NonZero,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering::Relaxed},
        mpsc::{Sender, channel},
    },
};

use clap::{Args, Parser};
use icarus_board::{board::TerminalState, r#move::MoveFlag, movegen::Abort};
use icarus_common::{
    piece::Color,
    util::enum_map::{EnumMap, enum_map},
};
use indicatif::{MultiProgress, ProgressBar, ProgressFinish, ProgressStyle};
use rand::{SeedableRng, rngs::SmallRng};
use viriformat::{
    chess::{
        board::{Board as ViriBoard, DrawType, GameOutcome, WinType},
        chessmove::{Move as ViriMove, MoveFlags as ViriFlags},
        piece::PieceType as ViriPiece,
        types::Square as ViriSquare,
    },
    dataformat::Game,
};

use crate::{
    datagen::genfens::try_generate_pos,
    position::Position,
    score::Score,
    search::{
        searcher::{GlobalCtx, SearchParams, ThreadCtx},
        transposition_table::{DEFAULT_TT_SIZE, TTable},
    },
    uci::SearchLimit,
};

#[derive(Parser)]
enum Cmd {
    Datagen(DatagenArgs),
}

#[derive(Args, Debug)]
struct DatagenArgs {
    #[clap(short, long, required = true)]
    output: PathBuf,
    /// Number of threads used for datagen. Defaults to `available_parallelism()`
    #[clap(short, long)]
    threads: Option<usize>,
    #[clap(flatten)]
    limits: Limits,
    /// Soft nodes per move.
    #[clap(short, long, default_value_t = 5000)]
    nodes: u64,
    #[clap(short, long)]
    dfrc: bool,
    /// Number of random plies played in openings
    #[clap(long, default_value_t = 8)]
    random_moves: usize,
    /// Size of the viriformat batches in KiB accumulated per thread before they are written to disk.
    #[clap(long, default_value_t = 32)]
    batch_size: usize,

    #[clap(long, default_value_t = 5)]
    win_adj_movecount: usize,
    #[clap(long, default_value_t = 2000)]
    win_adj_score: i16,

    #[clap(long, default_value_t = 32)]
    draw_adj_movenumber: usize,
    #[clap(long, default_value_t = 6)]
    draw_adj_movecount: usize,
    #[clap(long, default_value_t = 10)]
    draw_adj_score: i16,
}

#[derive(Args, Debug)]
#[group(required = true)]
struct Limits {
    /// Maximum number of games generated before datagen aborts.
    #[clap(short = 'g', long)]
    max_games: Option<usize>,
    /// Maximum number of positions generated before datagen aborts.
    #[clap(short = 'p', long)]
    max_positions: Option<usize>,
}

struct DatagenCtx {
    games: AtomicUsize,
    positions: AtomicUsize,
    white_wins: AtomicUsize,
    black_wins: AtomicUsize,
    draws: AtomicUsize,

    win_adj_movecount: usize,
    win_adj_score: i16,

    draw_adj_movenumber: usize,
    draw_adj_movecount: usize,
    draw_adj_score: i16,

    nodes: u64,
    dfrc: bool,
    random_moves: usize,
    batch_size_kb: usize,
    game_limit: Option<(usize, ProgressBar)>,
    pos_limit: Option<(usize, ProgressBar)>,
}

pub fn datagen() {
    let Cmd::Datagen(args) = Cmd::parse();

    let mut ctx = DatagenCtx {
        games: AtomicUsize::new(0),
        positions: AtomicUsize::new(0),
        white_wins: AtomicUsize::new(0),
        black_wins: AtomicUsize::new(0),
        draws: AtomicUsize::new(0),

        game_limit: None,
        pos_limit: None,

        win_adj_movecount: args.win_adj_movecount,
        win_adj_score: args.win_adj_score,

        draw_adj_movenumber: args.draw_adj_movenumber,
        draw_adj_movecount: args.draw_adj_movecount,
        draw_adj_score: args.draw_adj_score,

        nodes: args.nodes,
        dfrc: args.dfrc,
        random_moves: args.random_moves,
        batch_size_kb: args.batch_size,
    };
    let threads = args
        .threads
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, NonZero::get));

    let (tx, rx) = channel();
    // let mut outfile = File::create_new(&args.output).unwrap();
    let mut outfile = File::create(&args.output).unwrap();

    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "{msg:9} : {pos:>10}/{len:10} ({percent_precise:>7}%) elapsed: {elapsed_precise}, ETA: {eta_precise} [{wide_bar}]",
    )
    .unwrap()
    .progress_chars("=>.");

    if let Some(games) = args.limits.max_games {
        let game_pb = m.add(
            ProgressBar::new(games as u64)
                .with_style(sty.clone())
                .with_message("games")
                .with_finish(ProgressFinish::Abandon),
        );
        game_pb.tick();
        ctx.game_limit = Some((games, game_pb));
    }

    if let Some(pos) = args.limits.max_positions {
        let pos_pb = m.add(
            ProgressBar::new(pos as u64)
                .with_style(sty.clone())
                .with_message("positions")
                .with_finish(ProgressFinish::Abandon),
        );
        pos_pb.tick();
        ctx.pos_limit = Some((pos, pos_pb));
    }

    std::thread::scope(|s| {
        for _ in 0..threads {
            let tx_clone = tx.clone();
            s.spawn(|| worker_loop(&ctx, tx_clone));
        }
        drop(tx);

        for batch in rx.iter() {
            outfile.write_all(&batch).unwrap();
        }
    });
}

fn worker_loop(ctx: &DatagenCtx, tx: Sender<Vec<u8>>) {
    let mut rng = SmallRng::from_os_rng();

    let mut thread_ctxs = enum_map! {
        _ => {
            let global = Arc::new(GlobalCtx {
                time_manager: Default::default(),
                nodes: Default::default(),
                num_searching: Default::default(),
                ttable: TTable::new(DEFAULT_TT_SIZE),
            });
            ThreadCtx::new(global, 0, ctx.dfrc)
        }
    };

    let mut buffer: Vec<u8> = Vec::with_capacity(ctx.batch_size_kb * 2048);

    loop {
        let games = ctx.games.fetch_add(1, Relaxed) + 1;
        if let Some((max, pb)) = &ctx.game_limit {
            pb.inc(1);
            if *max < games {
                break;
            }
        }

        let game = play_game(&mut rng, ctx, &mut thread_ctxs);
        let n_pos = game.moves.len();

        game.serialise_into(&mut buffer).unwrap();
        if buffer.len() >= ctx.batch_size_kb * 1024 {
            tx.send(buffer.clone()).unwrap();
            buffer.clear();
        }

        let pos = ctx.positions.fetch_add(n_pos, Relaxed);
        if let Some((max, pb)) = &ctx.pos_limit {
            pb.inc(n_pos as u64);
            if *max < pos {
                break;
            }
        }
    }

    if !buffer.is_empty() {
        tx.send(buffer).unwrap();
    }
}

fn play_game(
    rng: &mut SmallRng,
    ctx: &DatagenCtx,
    thread_ctxs: &mut EnumMap<Color, ThreadCtx>,
) -> Game {
    let mut pos = Position::new(
        std::iter::repeat_with(|| try_generate_pos(rng, ctx.dfrc, ctx.random_moves, thread_ctxs))
            .flatten()
            .next()
            .unwrap(),
    );

    thread_ctxs.values_mut().for_each(|t| {
        t.history.clear();
        t.global.ttable.clear();
    });

    let mut game = {
        let mut board = ViriBoard::new();
        board
            .set_from_fen(&pos.board().fen(ctx.dfrc), ctx.dfrc)
            .unwrap();
        Game::new(&board)
    };

    let mut prev_score: Option<Score> = None;
    let mut draw_adj_count = 0;
    let mut win_adj_count = 0;
    let result = loop {
        let stm = pos.board().stm();

        thread_ctxs[stm].global.nodes.store(0, Relaxed);
        thread_ctxs[stm].global.num_searching.store(1, Relaxed);
        thread_ctxs[stm].global.time_manager.init(
            stm,
            &[SearchLimit::Nodes(ctx.nodes)],
            true,
            false,
            0,
        );

        let score = thread_ctxs[stm].do_search(SearchParams {
            pos: pos.clone(),
            root_moves: None,
            chess960: ctx.dfrc,
            print_info: false,
        });
        let mv = thread_ctxs[stm].search_stack[0].pv[0];

        let (from, to) = (
            ViriSquare::new_clamped(mv.from().idx()),
            ViriSquare::new_clamped(mv.to().idx()),
        );

        let viri_mv = match mv.flag() {
            MoveFlag::EnPassant => ViriMove::new_with_flags(from, to, ViriFlags::EnPassant),
            MoveFlag::Castle => ViriMove::new_with_flags(from, to, ViriFlags::Castle),
            MoveFlag::Promotion => ViriMove::new_with_promo(
                from,
                to,
                ViriPiece::new(mv.promotes_to_unchecked().idx()).unwrap(),
            ),
            _ => ViriMove::new(from, to),
        };

        let white_eval = score * stm.signum() as i16;

        game.add_move(viri_mv, white_eval.0);
        pos.make_move(mv, None);

        if let Some(prev) = prev_score {
            if white_eval.0.abs() >= ctx.win_adj_score && white_eval.0.signum() == prev.0.signum() {
                win_adj_count += 1;
            } else {
                win_adj_count = 0;
            }
        }

        if game.moves.len().div_ceil(2) >= ctx.draw_adj_movenumber
            && white_eval.0.abs() <= ctx.draw_adj_score
        {
            draw_adj_count += 1;
        } else {
            draw_adj_count = 0;
        }

        prev_score = Some(white_eval);

        if white_eval.is_mate() {
            if white_eval.0 > 0 {
                break GameOutcome::WhiteWin(WinType::Mate);
            } else {
                break GameOutcome::BlackWin(WinType::Mate);
            }
        }

        if win_adj_count >= ctx.win_adj_movecount * 2 {
            if white_eval.0 > 0 {
                break GameOutcome::WhiteWin(WinType::Adjudication);
            } else {
                break GameOutcome::BlackWin(WinType::Adjudication);
            }
        }

        if draw_adj_count >= ctx.draw_adj_movecount * 2 {
            break GameOutcome::Draw(DrawType::Adjudication);
        }

        match pos.board().terminal_state() {
            Some(TerminalState::Checkmate(stm)) => match stm {
                Color::White => break GameOutcome::WhiteWin(WinType::Mate),
                Color::Black => break GameOutcome::BlackWin(WinType::Mate),
            },
            Some(TerminalState::Draw) => {
                if pos.board().gen_moves(|moves| {
                    if !moves.is_empty() {
                        Abort::Yes
                    } else {
                        Abort::No
                    }
                }) == Abort::Yes
                {
                    break GameOutcome::Draw(DrawType::InsufficientMaterial);
                } else {
                    break GameOutcome::Draw(DrawType::Stalemate);
                }
            }
            None => {}
        }

        if pos.board().halfmove_clock() >= 100 {
            break GameOutcome::Draw(DrawType::FiftyMoves);
        }
    };

    match result {
        GameOutcome::WhiteWin(_) => {
            ctx.white_wins.fetch_add(1, Relaxed);
        }
        GameOutcome::BlackWin(_) => {
            ctx.black_wins.fetch_add(1, Relaxed);
        }
        _ => {
            ctx.draws.fetch_add(1, Relaxed);
        }
    }

    game.set_outcome(result);
    game
}
