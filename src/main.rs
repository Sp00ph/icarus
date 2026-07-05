use icarus_board::board::Board;

use crate::{engine::Engine, nnue::network::Nnue, position::Position};

pub mod bench;
pub mod datagen;
pub mod engine;
pub mod nnue;
pub mod position;
pub mod score;
pub mod search;
pub mod uci;
pub mod util;
pub mod wdl;

fn main() -> anyhow::Result<()> {
    // Engine::new().run()

    for fen in [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/P2P2PP/q2Q1R1K w kq - 0 2",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNB1KBNR w KQkq - 0 1",
        "rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "rn1qkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r1bqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        // "1nbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQka - 0 1",
    ] {
        let board = Board::read_fen(fen).unwrap();
        let eval = Position::new(board).eval(&mut Nnue::new(&Board::read_fen(fen).unwrap()), false);
        println!("FEN: {fen}");
        println!("EVAL: {}", eval.0);
    }

    Ok(())
}
