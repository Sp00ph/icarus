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
            Accumulator, Feature, KingBucketCache, Updates, acc_add, acc_add_sub, acc_add_sub2,
            acc_add2_sub2, acc_add4, acc_sub, acc_sub4,
        },
        inference::forward,
    },
    util::MAX_PLY,
};

pub const INPUT: usize = 768;
pub const HL: usize = 1024;
pub const NUM_KING_BUCKETS: usize = 8;
#[rustfmt::skip]
pub static KING_BUCKET_LAYOUT: [u8; 64] = [
    0, 1, 2, 3, 3, 2, 1, 0,
    4, 4, 5, 5, 5, 5, 4, 4,
    6, 6, 6, 6, 6, 6, 6, 6,
    6, 6, 6, 6, 6, 6, 6, 6,
    6, 6, 6, 6, 6, 6, 6, 6,
    7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7,
];

pub fn king_bucket(king: Square, perspective: Color) -> usize {
    let king = Square::new(king.file(), king.rank().relative_to(perspective));
    KING_BUCKET_LAYOUT[king] as usize
}

pub fn should_mirror(king: Square) -> bool {
    king.file() > File::D
}

pub static NET: Network =
    unsafe { std::mem::transmute(*include_bytes!(concat!(env!("OUT_DIR"), "/icarus.nnue"))) };

#[repr(C, align(64))]
pub struct Network {
    pub ft_weight: [[[i16; HL]; INPUT]; NUM_KING_BUCKETS],
    pub ft_bias: [i16; HL],
    pub out_weight: [[i16; HL]; 2],
    pub out_bias: i16,
}

pub struct Nnue {
    stack: Box<[Accumulator; MAX_PLY as usize + 1]>,
    idx: usize,
    cache: Box<KingBucketCache>,
}

impl Nnue {
    pub fn new(board: &Board) -> Self {
        let mut this = Self {
            stack: vec![
                Accumulator {
                    values: enum_map! { _ => [0; HL] },
                    dirty: Default::default(),
                    needs_refresh: Default::default(),
                    updates: Default::default(),
                };
                MAX_PLY as usize + 1
            ]
            .into_boxed_slice()
            .try_into()
            .unwrap(),
            idx: 0,
            cache: Default::default(),
        };

        this.full_reset(board);
        this
    }

    pub fn full_reset(&mut self, board: &Board) {
        self.idx = 0;
        *self.cache = Default::default();
        self.reset(board, Color::White);
        self.reset(board, Color::Black);
    }

    pub fn reset(&mut self, board: &Board, perspective: Color) {
        let king = board.king(perspective);
        let mirror = should_mirror(king);
        let bucket = king_bucket(king, perspective);

        let entry = &mut self.cache.entries[perspective][mirror as usize][bucket];

        let mut adds: ArrayVec<usize, 64> = ArrayVec::new();
        let mut subs: ArrayVec<usize, 64> = ArrayVec::new();

        for color in Color::all() {
            for piece in Piece::all() {
                let current = board.colored_pieces(piece, color);
                let cached = entry.colors[color] & entry.pieces[piece];

                for add in current & !cached {
                    adds.push(
                        Feature {
                            piece,
                            color,
                            square: add,
                        }
                        .idx(perspective, king),
                    );
                }

                for sub in cached & !current {
                    subs.push(
                        Feature {
                            piece,
                            color,
                            square: sub,
                        }
                        .idx(perspective, king),
                    );
                }
            }
        }

        let weights = &NET.ft_weight[bucket];
        let values = &mut entry.features;

        let (chunks, rem) = adds.as_chunks();
        for &[add1, add2, add3, add4] in chunks {
            acc_add4(values, weights, add1, add2, add3, add4);
        }
        for &add in rem {
            acc_add(values, weights, add);
        }

        let (chunks, rem) = subs.as_chunks();
        for &[sub1, sub2, sub3, sub4] in chunks {
            acc_sub4(values, weights, sub1, sub2, sub3, sub4);
        }
        for &sub in rem {
            acc_sub(values, weights, sub);
        }

        entry.pieces = *board.piece_bbs();
        entry.colors = *board.color_bbs();

        self.stack[self.idx].values[perspective].copy_from_slice(&entry.features);
        self.stack[self.idx].dirty[perspective] = false;
        self.stack[self.idx].needs_refresh[perspective] = false;
    }

    pub fn make_move(&mut self, board: &Board, mv: Move) {
        let mut updates = Updates::default();
        let (from, mut to) = (mv.from(), mv.to());
        let (piece, stm) = (mv.piece_type(board), board.stm());

        if let Some(dir) = mv.castling_dir() {
            let (king, rook) = (dir.king_dst(), dir.rook_dst());
            let rank = Rank::R1.relative_to(stm);

            updates.move_piece(from, Square::new(king, rank), Piece::King, stm);
            updates.move_piece(to, Square::new(rook, rank), Piece::Rook, stm);
            to = Square::new(dir.king_dst(), to.rank());
        } else if let Some(promo) = mv.promotes_to() {
            updates.remove_piece(from, piece, stm);
            updates.add_piece(to, promo, stm);
        } else {
            updates.move_piece(from, to, piece, stm);
        }

        if mv.flag() == MoveFlag::EnPassant {
            let victim_sq = Square::new(to.file(), from.rank());
            updates.remove_piece(victim_sq, Piece::Pawn, !stm);
        } else if let Some(victim) = mv.captures(board) {
            updates.remove_piece(to, victim, !stm);
        }

        self.stack[self.idx].updates = updates;
        self.idx += 1;
        self.stack[self.idx].dirty = enum_map! { _ => true };
        self.stack[self.idx].needs_refresh = self.stack[self.idx - 1].needs_refresh;

        if piece == Piece::King
            && (king_bucket(from, stm) != king_bucket(to, stm)
                || should_mirror(from) != should_mirror(to))
        {
            self.stack[self.idx].needs_refresh[stm] = true;
        }
    }

    pub fn unmake_move(&mut self) {
        self.idx -= 1;
    }

    pub fn update(&mut self, board: &Board) {
        for perspective in Color::all() {
            if self.stack[self.idx].needs_refresh[perspective] {
                self.reset(board, perspective);
            } else if self.stack[self.idx].dirty[perspective] {
                self.update_color(perspective, board);
            }
        }
    }

    fn update_color(&mut self, perspective: Color, board: &Board) {
        let mut clean_idx = None;

        for i in (0..self.idx).rev() {
            if self.stack[i].needs_refresh[perspective] {
                break;
            }
            if !self.stack[i].dirty[perspective] {
                clean_idx = Some(i);
                break;
            }
        }

        let king = board.king(perspective);
        let bucket = king_bucket(king, perspective);

        let Some(clean_idx) = clean_idx else {
            self.reset(board, perspective);
            return;
        };

        let weights = &NET.ft_weight[bucket];

        for idx in clean_idx..self.idx {
            let [clean, dirty] = self.stack.get_disjoint_mut([idx, idx + 1]).unwrap();
            let clean_acc = &clean.values[perspective];
            let dirty_acc = &mut dirty.values[perspective];
            match (&clean.updates.adds[..], &clean.updates.subs[..]) {
                (&[add], &[sub]) => acc_add_sub(
                    clean_acc,
                    dirty_acc,
                    weights,
                    add.idx(perspective, king),
                    sub.idx(perspective, king),
                ),
                (&[add], &[sub1, sub2]) => acc_add_sub2(
                    clean_acc,
                    dirty_acc,
                    weights,
                    add.idx(perspective, king),
                    sub1.idx(perspective, king),
                    sub2.idx(perspective, king),
                ),
                (&[add1, add2], &[sub1, sub2]) => acc_add2_sub2(
                    clean_acc,
                    dirty_acc,
                    weights,
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

        forward(us, them)
    }
}
