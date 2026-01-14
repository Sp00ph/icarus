use crate::{
    attack_generators::{bishop_moves, rook_moves},
    board::Board,
    r#move::{Move, MoveFlag},
    movegen::Abort,
};
use icarus_common::{
    bitboard::Bitboard,
    lookups::{knight_moves, pawn_attacks, pawn_pushes},
    piece::{Color, Piece},
    square::{File, Rank, Square},
};

pub fn test_islegal(fens: &[&str]) {
    let mut all_moves = vec![];
    for from in Square::all() {
        let to = knight_moves(from)
            | rook_moves(from, Bitboard::EMPTY)
            | bishop_moves(from, Bitboard::EMPTY);
        all_moves.extend(to.into_iter().map(|to| Move::new(from, to, MoveFlag::None)));
    }

    for col in Color::all() {
        for file in File::all() {
            // EP
            let from = Square::new(file, Rank::R5.relative_to(col));
            let to = pawn_attacks(from, col);
            all_moves.extend(
                to.into_iter()
                    .map(|to| Move::new(from, to, MoveFlag::EnPassant)),
            )
        }
    }
    for col in Color::all() {
        for file in File::all() {
            // Promos
            let from = Square::new(file, Rank::R7.relative_to(col));
            let to = pawn_attacks(from, col) | pawn_pushes(from, col, Bitboard::EMPTY);
            all_moves.extend(to.into_iter().flat_map(|to| {
                [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen]
                    .map(|promo| Move::new_promotion(from, to, promo))
            }));
        }
    }

    all_moves.push(Move::new(Square::E1, Square::A1, MoveFlag::Castle));
    all_moves.push(Move::new(Square::E1, Square::H1, MoveFlag::Castle));
    all_moves.push(Move::new(Square::E8, Square::A8, MoveFlag::Castle));
    all_moves.push(Move::new(Square::E8, Square::H8, MoveFlag::Castle));

    for depth in 0.. {
        println!("Depth {depth}...");
        for fen in fens {
            let board = Board::read_fen(fen).unwrap();
            println!("{fen}");
            islegal_all(&board, depth, &all_moves);
        }
    }
}

pub fn islegal_all(board: &Board, depth: u8, all_moves: &[Move]) {
    let moves: Vec<_> = board.gen_all_moves_to();

    for &mv in all_moves {
        assert_eq!(
            board.is_legal(mv),
            moves.contains(&mv),
            "{} {} {:?}",
            mv.from(),
            mv.to(),
            mv.flag()
        );
    }

    if depth == 0 {
        return;
    }

    board.gen_moves(|moves| {
        for mv in moves {
            let mut board = *board;
            board.make_move(mv);

            islegal_all(&board, depth - 1, all_moves);
        }
        Abort::No
    });
}
