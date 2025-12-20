use icarus_board::board::Board;
use icarus_common::{bitboard::Bitboard, r#move::Move, piece::Piece};

pub struct Position {
    board: Board,
    /// Previously played boards. `history[0]` is the starting position.
    history: Vec<Board>,
}

impl Position {
    pub fn new(board: Board) -> Self {
        Self {
            board,
            history: vec![],
        }
    }

    pub fn make_move(&mut self, mv: Move) {
        self.history.push(self.board);
        self.board.make_move(mv);
    }

    pub fn unmake_move(&mut self) {
        self.board = self.history.pop().unwrap();
    }

    pub fn insufficient_material(&self) -> bool {
        match self.board.occupied().popcnt() {
            // Only kings are insufficient
            2 => true,
            // Only kings and one bishop/knight are insufficient
            3 => {
                (self.board.pieces(Piece::Bishop) | self.board.pieces(Piece::Knight)).is_non_empty()
            }
            // Only kings and bishops is insufficient if the bishops are all on the same color squares.
            n => {
                let bishops = self.board.pieces(Piece::Bishop);

                (bishops.popcnt() == n - 2)
                    && (bishops.is_subset_of(Bitboard::LIGHT_SQUARES)
                        || bishops.is_subset_of(Bitboard::DARK_SQUARES))
            }
        }
    }

    pub fn board(&self) -> &Board {
        &self.board
    }
}
