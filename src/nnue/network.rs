use std::{ptr, sync::Arc};

use arrayvec::ArrayVec;
use icarus_board::{
    board::Board,
    r#move::{Move, MoveFlag},
};
use icarus_common::{
    piece::{Color, Piece},
    square::{File, Rank, Square},
    util::enum_map::enum_map,
};

use crate::{
    nnue::{
        accumulator::{
            Accumulator, Feature, Updates, acc_add, acc_add_sub, acc_add_sub2, acc_add2_sub2,
        },
        inference::forward,
    },
    util::MAX_PLY,
};

pub const INPUT: usize = 768;
pub const HL: usize = 128;

const DEFAULT_NET: &[u8; size_of::<Network>()] =
    include_bytes!(concat!(env!("OUT_DIR"), "/icarus.nnue"));

#[repr(C, align(64))]
pub struct Network {
    pub ft_weight: [i16; INPUT * HL],
    pub ft_bias: [i16; HL],
    pub out_weight: [i16; 2 * HL],
    pub out_bias: i16,
}

impl Network {
    pub fn load(bytes: &[u8]) -> Arc<Self> {
        assert_eq!(bytes.len(), size_of::<Self>());

        let mut this = Arc::<Self>::new_uninit();
        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                ptr::from_mut(Arc::get_mut(&mut this).unwrap()).cast(),
                bytes.len(),
            );

            this.assume_init()
        }
    }

    pub fn default_net() -> Arc<Self> {
        Self::load(DEFAULT_NET)
    }
}

pub struct Nnue {
    stack: Box<[Accumulator; MAX_PLY as usize + 1]>,
    idx: usize,
    network: Arc<Network>,
}

impl Nnue {
    pub fn new(board: &Board, network: Arc<Network>) -> Self {
        let mut this = Self {
            stack: vec![
                Accumulator {
                    values: enum_map! { _ => [0; HL] },
                    dirty: enum_map! { _ => false },
                    updates: Updates::default(),
                };
                MAX_PLY as usize + 1
            ]
            .into_boxed_slice()
            .try_into()
            .unwrap(),
            idx: 0,
            network,
        };

        this.full_reset(board);
        this
    }

    pub fn full_reset(&mut self, board: &Board) {
        self.idx = 0;
        self.reset(board, Color::White);
        self.reset(board, Color::Black);
    }

    pub fn reset(&mut self, board: &Board, perspective: Color) {
        self.stack[self.idx].values[perspective].copy_from_slice(&self.network.ft_bias);
        let king = board.king(perspective);

        let adds: ArrayVec<usize, 64> = Square::all()
            .filter(|&square| board.piece_on(square).is_some())
            .map(|square| {
                let piece = board.piece_on(square).unwrap();
                let color = Color::from_idx(board.occupied_by(Color::Black).contains(square) as u8);
                Feature {
                    square,
                    piece,
                    color,
                }
                .idx(perspective, king)
            })
            .collect();

        acc_add(
            &mut self.stack[self.idx].values[perspective],
            &self.network,
            &adds,
        );
        self.stack[self.idx].dirty[perspective] = false;
    }

    pub fn make_move(&mut self, old_board: &Board, new_board: &Board, mv: Move) {
        let mut updates = Updates::default();
        let (from, to) = (mv.from(), mv.to());
        let (piece, stm) = (mv.piece_type(old_board), old_board.stm());

        if let Some(dir) = mv.castling_dir() {
            let (king, rook) = (dir.king_dst(), dir.rook_dst());
            let rank = Rank::R1.relative_to(stm);

            updates.move_piece(from, Square::new(king, rank), Piece::King, stm);
            updates.move_piece(to, Square::new(rook, rank), Piece::Rook, stm);
        } else if let Some(promo) = mv.promotes_to() {
            updates.remove_piece(from, piece, stm);
            updates.add_piece(to, promo, stm);
        } else {
            updates.move_piece(from, to, piece, stm);
        }

        if mv.flag() == MoveFlag::EnPassant {
            let victim_sq = Square::new(to.file(), from.rank());
            updates.remove_piece(victim_sq, Piece::Pawn, !stm);
        } else if let Some(victim) = mv.captures(old_board) {
            updates.remove_piece(to, victim, !stm);
        }

        self.stack[self.idx].updates = updates;
        self.stack[self.idx + 1].dirty = enum_map! { _ => true };
        self.idx += 1;

        if piece == Piece::King && (from.file() > File::D) != (to.file() > File::D) {
            self.reset(new_board, stm);
        }
    }

    pub fn unmake_move(&mut self) {
        self.idx -= 1;
    }

    pub fn update(&mut self, board: &Board) {
        for perspective in Color::all() {
            if self.stack[self.idx].dirty[perspective] {
                self.update_color(perspective, board.king(perspective));
            }
        }
    }

    fn update_color(&mut self, perspective: Color, king: Square) {
        let clean_idx = (0..self.idx)
            .rev()
            .find(|&i| !self.stack[i].dirty[perspective])
            .unwrap();

        for idx in clean_idx..self.idx {
            let [clean, dirty] = self.stack.get_disjoint_mut([idx, idx + 1]).unwrap();
            let clean_acc = &clean.values[perspective];
            let dirty_acc = &mut dirty.values[perspective];
            let network = &self.network;
            match (&clean.updates.adds[..], &clean.updates.subs[..]) {
                (&[add], &[sub]) => acc_add_sub(
                    clean_acc,
                    dirty_acc,
                    network,
                    add.idx(perspective, king),
                    sub.idx(perspective, king),
                ),
                (&[add], &[sub1, sub2]) => acc_add_sub2(
                    clean_acc,
                    dirty_acc,
                    network,
                    add.idx(perspective, king),
                    sub1.idx(perspective, king),
                    sub2.idx(perspective, king),
                ),
                (&[add1, add2], &[sub1, sub2]) => acc_add2_sub2(
                    clean_acc,
                    dirty_acc,
                    network,
                    add1.idx(perspective, king),
                    add2.idx(perspective, king),
                    sub1.idx(perspective, king),
                    sub2.idx(perspective, king),
                ),
                _ => unreachable!("Invalid Updates"),
            };
            dirty.dirty[perspective] = false;
        }
    }

    pub fn eval(&self, stm: Color) -> i32 {
        let acc = &self.stack[self.idx];
        let (us, them) = (&acc.values[stm], &acc.values[!stm]);

        forward(us, them, &self.network)
    }
}
