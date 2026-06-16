use icarus_board::{
    attack_generators::{bishop_moves, rook_moves},
    board::Board,
    r#move::{Move, MoveFlag},
};
use icarus_common::{
    bitboard::Bitboard,
    lookups::{bishop_rays, king_moves, knight_moves, pawn_attacks, rook_rays},
    piece::{Color, Piece},
    square::Square,
};

use crate::search::params::see_val;

pub struct SeeCache {
    pub cache: [u64; 65536],
}

impl SeeCache {
    #[unsafe(no_mangle)]
    fn see_key(board: &Board, mv: Move) -> u64 {
        let (from, to, flag) = (mv.from(), mv.to(), mv.flag());

        let mut pieces = *board.piece_bbs();
        let mut colors = *board.color_bbs();

        let from_piece = board.piece_on(mv.from()).unwrap();
        let to_piece = mv.promotes_to().unwrap_or(from_piece);

        pieces[from_piece] ^= from;
        pieces[to_piece] ^= to;
        colors[board.stm()] ^= from.bitboard() | to.bitboard();

        let victim_info = match flag {
            MoveFlag::Castle => unreachable!("Shouldn't call see_key() on castle moves"),
            MoveFlag::EnPassant => Some((Piece::Pawn, Square::new(to.file(), from.rank()))),
            _ => board.piece_on(to).map(|piece| (piece, to)),
        };

        if let Some((victim_piece, victim_sq)) = victim_info {
            pieces[victim_piece] ^= victim_sq;
            colors[!board.stm()] ^= victim_sq;
        }

        let mut bbs = [Bitboard::EMPTY; 8];
        bbs[..6].copy_from_slice(pieces.as_array());
        bbs[6..].copy_from_slice(colors.as_array());

        let xor_const = [
            7885887765656621641u64,
            2296521334875578217,
            15643834537774262581,
            17022334849020955124,
            15921415685405999243,
            6991202512435526477,
            7094698545887784981,
            5442028411590048733,
        ];

        let mul_const = [
            3687171055u32,
            3687444937,
            3475285192,
            1292872970,
            951319277,
            2166860593,
            2090071128,
            985307635,
            1696940181,
            345125433,
            1538984714,
            2976375808,
            3015917641,
            3917624446,
            3896687178,
            1474348572,
        ];

        let superpiece_rays = rook_rays(to) | bishop_rays(to) | knight_moves(to);

        let mut key = unsafe {
            use std::arch::x86_64::*;
            let raymask = _mm256_set1_epi64x(superpiece_rays.0 as i64);
            let lo = _mm256_movemask_epi8(_mm256_mullo_epi32(
                _mm256_xor_si256(
                    _mm256_and_si256(_mm256_loadu_si256(bbs.as_ptr().cast()), raymask),
                    _mm256_loadu_si256(xor_const.as_ptr().cast()),
                ),
                _mm256_loadu_si256(mul_const.as_ptr().cast()),
            ));
            let hi = _mm256_movemask_epi8(_mm256_mullo_epi32(
                _mm256_xor_si256(
                    _mm256_and_si256(_mm256_loadu_si256(bbs.as_ptr().add(4).cast()), raymask),
                    _mm256_loadu_si256(xor_const.as_ptr().add(4).cast()),
                ),
                _mm256_loadu_si256(mul_const.as_ptr().add(8).cast()),
            ));

            (lo as u64) | ((hi as u64) << 32)
        };

        // the key bits still have clear dependencies. do one round of xorshift to shuffle them more

        key ^= key << 13;
        key ^= key >> 7;
        key ^= key << 17;
        key
    }

    fn full_see(board: &Board, mv: Move) -> i32 {
        let (from, to, flag) = (mv.from(), mv.to(), mv.flag());

        if flag == MoveFlag::Castle {
            return 0;
        }

        let mut next_victim = mv
            .promotes_to()
            .or_else(|| board.piece_on(mv.from()))
            .unwrap();

        // 35 is the max popcnt of any superpiece rays
        let mut gain = [0; 35];
        gain[0] = mv.captures(board).map_or(0, see_val)
            + mv.promotes_to()
                .map_or(0, |promo| see_val(promo) - see_val(Piece::Pawn));

        let orth = board.pieces(Piece::Rook) | board.pieces(Piece::Queen);
        let diag = board.pieces(Piece::Bishop) | board.pieces(Piece::Queen);

        let mut occupied = board.occupied() ^ from | to;
        if flag == MoveFlag::EnPassant {
            occupied ^= Square::new(to.file(), from.rank());
        }

        #[rustfmt::skip]
        let mut attackers = (
            (pawn_attacks(to, Color::White) & board.occupied_by(Color::Black) & board.pieces(Piece::Pawn))
            | (pawn_attacks(to, Color::Black) & board.occupied_by(Color::White) & board.pieces(Piece::Pawn))
            | (knight_moves(to) & board.pieces(Piece::Knight))
            | (bishop_moves(to, occupied) & diag)
            | (rook_moves(to, occupied) & orth)
            | (king_moves(to) & board.pieces(Piece::King))
        ) & occupied;

        let mut stm = !board.stm();

        let mut d = 0;
        loop {
            let my_attackers = attackers & board.occupied_by(stm);
            if next_victim == Piece::King && my_attackers.is_non_empty() {
                break;
            }

            d += 1;
            stm = !stm;
            if my_attackers.is_empty() {
                break;
            }
            gain[d] = see_val(next_victim) - gain[d - 1];

            next_victim = Piece::all()
                .find(|&p| (my_attackers & board.pieces(p)).is_non_empty())
                .unwrap();

            occupied ^= (board.pieces(next_victim) & my_attackers).next();

            if [Piece::Pawn, Piece::Bishop, Piece::Queen].contains(&next_victim) {
                attackers |= bishop_moves(to, occupied) & diag;
            }

            if [Piece::Rook, Piece::Queen].contains(&next_victim) {
                attackers |= rook_moves(to, occupied) & orth;
            }

            attackers &= occupied;
        }

        while d > 1 {
            d -= 1;
            gain[d - 1] = gain[d - 1].min(-gain[d]);
        }

        gain[0]
    }

    fn get_see(&mut self, board: &Board, mv: Move) -> i32 {
        if mv.flag() == MoveFlag::Castle {
            return 0;
        }

        let key = Self::see_key(board, mv);

        let idx = key % 65536;
        let tag = key & !65535;

        let cached = self.cache[idx as usize];
        if cached & !65535 == tag {
            return cached as i16 as i32;
        }

        let see = Self::full_see(board, mv);
        self.cache[idx as usize] = tag | (see as i16 as u16 as u64);
        see
    }

    pub fn cmp_see(&mut self, board: &Board, mv: Move, threshold: i32) -> bool {
        self.get_see(board, mv) >= threshold
    }
}
