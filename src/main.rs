use std::time::Instant;

use icarus_board::board::Board;

fn perft(board: &Board, depth: u8) -> u64 {
    let mut nodes = 0;

    if depth == 0 {
        return 1;
    }
    if depth == 1 {
        let mut count = 0;
        board.gen_moves(|m| count += m.len() as u64);
        return count;
    }
    board.gen_moves(|moves| {
        for mv in moves {
            let mut board = *board;
            board.make_move(mv);

            nodes += perft(&board, depth - 1);
        }
    });

    nodes
}

fn main() {
    let b = Board::start_pos();

    for depth in 1u8.. {
        let t0 = Instant::now();
        let n = perft(&b, depth);
        println!("{:.2?}, {n}", t0.elapsed());
    }
}
