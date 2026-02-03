use std::sync::Arc;

use arrayvec::ArrayVec;
use icarus_board::{board::Board, r#move::Move};
use rand::{Rng, SeedableRng, rngs::SmallRng};

use crate::{
    nnue::network::{Network, Nnue},
    position::Position,
    search::move_picker::MAX_MOVES,
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

fn try_generate_fen(
    rng: &mut SmallRng,
    dfrc: bool,
    random_moves: usize,
    network: Arc<Network>,
) -> Option<String> {
    let random_moves = random_moves + rng.random_bool(0.5) as usize;

    let mut board = startpos(rng, dfrc);

    for _ in 0..random_moves {
        let legal: ArrayVec<Move, MAX_MOVES> = board.gen_all_moves_to();
        if legal.is_empty() {
            return None;
        }
        let mv = legal[rng.random_range(0..legal.len())];
        board.make_move(mv);
    }

    let mut nnue = Nnue::new(&board, network);
    if Position::new(board).eval(&mut nnue).0.abs() > 1000 {
        return None;
    }

    if board.terminal_state().is_some() {
        return None;
    }

    Some(board.fen(dfrc))
}

pub fn genfens(n: usize, seed: u64, dfrc: bool, random_moves: usize, network: Arc<Network>) {
    let mut rng = SmallRng::seed_from_u64(seed);
    for fen in
        std::iter::repeat_with(|| try_generate_fen(&mut rng, dfrc, random_moves, network.clone()))
            .flatten()
            .take(n)
    {
        println!("info string genfens {fen}")
    }
}
