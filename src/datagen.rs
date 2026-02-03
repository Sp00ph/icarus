use arrayvec::ArrayVec;
use icarus_board::{board::Board, r#move::Move};
use rand::{Rng, SeedableRng, rngs::SmallRng};

use crate::{pesto::eval, search::move_picker::MAX_MOVES};

fn startpos(rng: &mut SmallRng, dfrc: bool) -> Board {
    if dfrc {
        let white = rng.random_range(0..960);
        let black = rng.random_range(0..960);
        Board::dfrc(white, black)
    } else {
        Board::start_pos()
    }
}

fn try_generate_fen(rng: &mut SmallRng, dfrc: bool, random_moves: usize) -> Option<String> {
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

    if eval(&board).0.abs() > 1000 {
        return None;
    }

    if board.terminal_state().is_some() {
        return None;
    }

    Some(board.fen(dfrc))
}

pub fn genfens(n: usize, seed: u64, dfrc: bool, random_moves: usize) {
    let mut rng = SmallRng::seed_from_u64(seed);
    for fen in std::iter::repeat_with(|| try_generate_fen(&mut rng, dfrc, random_moves)).flatten().take(n) {
        println!("info string genfens {fen}")
    }
}