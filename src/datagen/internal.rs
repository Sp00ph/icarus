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
    search::{
        searcher::{GlobalCtx, ThreadCtx, id_loop},
        transposition_table::TTable,
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
    /// Size of the viriformat batches accumulated per thread before they are written to disk.
    #[clap(long, default_value_t = 16)]
    batch_size: usize,
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

#[derive(Default)]
struct DatagenCtx {
    games: AtomicUsize,
    positions: AtomicUsize,
    white_wins: AtomicUsize,
    black_wins: AtomicUsize,
    draws: AtomicUsize,

    nodes: u64,
    dfrc: bool,
    random_moves: usize,
    batch_size_mb: usize,
    game_limit: Option<(usize, ProgressBar)>,
    pos_limit: Option<(usize, ProgressBar)>,
}

pub fn datagen() {
    let Cmd::Datagen(args) = Cmd::parse();

    let mut ctx = DatagenCtx::default();
    ctx.nodes = args.nodes;
    ctx.dfrc = args.dfrc;
    ctx.random_moves = args.random_moves;
    ctx.batch_size_mb = args.batch_size;
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
        Color::White | Color::Black => {
            let global = Arc::new(GlobalCtx {
                time_manager: Default::default(),
                nodes: Default::default(),
                num_searching: Default::default(),
                ttable: TTable::new(16),
            });
            ThreadCtx::new(global, 0, ctx.dfrc)
        }
    };

    let mut buffer: Vec<u8> = vec![];

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
        if buffer.len() >= ctx.batch_size_mb * (1 << 20) {
            tx.send(buffer.clone()).unwrap();
            buffer.clear();
        }

        let pos_now = ctx.positions.fetch_add(n_pos, Relaxed);
        if let Some((max, pb)) = &ctx.pos_limit {
            pb.inc(n_pos as u64);
            if *max < pos_now {
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
    let mut pos = std::iter::repeat_with(|| {
        try_generate_pos(
            rng,
            ctx.dfrc,
            ctx.random_moves,
            &mut thread_ctxs[Color::White],
        )
    })
    .flatten()
    .next()
    .unwrap();

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
    let mut stm = pos.board().stm();

    let result = loop {
        thread_ctxs[stm].global.nodes.store(0, Relaxed);
        thread_ctxs[stm].nodes.reset_local();
        thread_ctxs[stm]
            .global
            .time_manager
            .init(stm, &[SearchLimit::Nodes(ctx.nodes)], true, 0);
        thread_ctxs[stm].chess960 = ctx.dfrc;
        thread_ctxs[stm].search_stack.fill(Default::default());
        thread_ctxs[stm].root_move_nodes = [[0; 64]; 64];
        thread_ctxs[stm].abort_now = false;
        thread_ctxs[stm].nnue.full_reset(pos.board());
        thread_ctxs[stm].global.num_searching.store(2, Relaxed);

        let score = id_loop(pos.clone(), &mut thread_ctxs[stm], false);
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

        let eval = score * stm.signum() as i16;
        game.add_move(viri_mv, eval.0);
        stm = !stm;
        pos.make_move(mv, None);

        if eval.is_mate() {
            if eval.0 > 0 {
                break GameOutcome::WhiteWin(WinType::Mate);
            } else {
                break GameOutcome::BlackWin(WinType::Mate);
            }
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
