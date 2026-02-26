use std::sync::{Arc, atomic::Ordering};

use arrayvec::ArrayVec;
use icarus_board::{board::Board, r#move::Move};
use rand::{Rng, SeedableRng, rngs::SmallRng};

use crate::{
    position::Position,
    search::{
        move_picker::MAX_MOVES,
        searcher::{GlobalCtx, SearchParams, ThreadCtx},
        transposition_table::TTable,
    },
    uci::SearchLimit,
};

fn startpos(rng: &mut SmallRng, dfrc: bool) -> Board {
    if dfrc {
        let white = rng.random_range(0..960);
        let black = rng.random_range(0..960);
        Board::dfrc(white, black)
    } else {
        Board::start_pos()
    }
}

pub fn try_generate_pos(
    rng: &mut SmallRng,
    dfrc: bool,
    random_moves: usize,
    thread: &mut ThreadCtx,
) -> Option<Board> {
    let random_moves = random_moves + rng.random_bool(0.5) as usize;

    let mut pos = Position::new(startpos(rng, dfrc));

    for _ in 0..random_moves {
        let legal: ArrayVec<Move, MAX_MOVES> = pos.board().gen_all_moves_to();
        if legal.is_empty() {
            return None;
        }
        let mv = legal[rng.random_range(0..legal.len())];
        pos.make_move(mv, None);
    }

    let board = *pos.board();

    if board.terminal_state().is_some() {
        return None;
    }

    thread.global.nodes.store(0, Ordering::Relaxed);
    thread.global.num_searching.store(1, Ordering::Relaxed);
    thread.nodes.reset_local();
    thread.global.time_manager.init(
        board.stm(),
        &[SearchLimit::Nodes(1000), SearchLimit::Depth(10)],
        true,
        false,
        0,
    );
    let score = thread.do_search(SearchParams {
        pos,
        root_moves: None,
        chess960: dfrc,
        print_info: false,
    });

    let limit = 1000;
    if score.0.abs() >= limit {
        return None;
    }

    Some(board)
}

pub fn genfens(n: usize, seed: u64, dfrc: bool, random_moves: usize) {
    let global = Arc::new(GlobalCtx {
        time_manager: Default::default(),
        nodes: Default::default(),
        num_searching: Default::default(),
        ttable: TTable::new(16),
    });
    let mut thread = ThreadCtx::new(global, 0, dfrc);

    let mut rng = SmallRng::seed_from_u64(seed);
    for pos in
        std::iter::repeat_with(|| try_generate_pos(&mut rng, dfrc, random_moves, &mut thread))
            .flatten()
            .take(n)
    {
        println!("info string genfens {}", pos.fen(dfrc))
    }
}
