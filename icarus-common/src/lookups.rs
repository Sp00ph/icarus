use crate::{
    bitboard::Bitboard,
    direction::{DownLeft, DownRight, Up, UpLeft, UpRight},
    piece::Color,
    square::{Rank, Square},
};

#[inline]
pub const fn rook_rays(sq: Square) -> Bitboard {
    const RAYS: &[Bitboard; Square::COUNT] = &{
        let mut arr = [Bitboard::EMPTY; Square::COUNT];

        let mut i = 0;
        while i < Square::COUNT {
            let sq = Square::from_idx(i as u8);
            arr[i] = sq.rank().bitboard().xor(sq.file().bitboard());

            i += 1;
        }

        arr
    };

    RAYS[sq.idx() as usize]
}

#[inline]
pub fn bishop_rays(sq: Square) -> Bitboard {
    const RAYS: &[Bitboard; Square::COUNT] = &{
        let mut arr = [Bitboard::EMPTY; Square::COUNT];

        let mut i = 0;
        while i < Square::COUNT {
            let sq = Square::from_idx(i as u8);
            arr[i] = Bitboard::main_diag_for(sq).xor(Bitboard::anti_diag_for(sq));

            i += 1;
        }

        arr
    };

    RAYS[sq.idx() as usize]
}

#[inline]
pub const fn knight_moves(sq: Square) -> Bitboard {
    const MOVES: &[Bitboard; Square::COUNT] = &{
        let mut arr = [Bitboard::EMPTY; Square::COUNT];

        let offsets = [
            (1, 2),
            (2, 1),
            (2, -1),
            (1, -2),
            (-1, -2),
            (-2, -1),
            (-2, 1),
            (-1, 2),
        ];

        let mut i = 0;
        while i < Square::COUNT {
            let from = Square::from_idx(i as u8);
            let mut bb = Bitboard::EMPTY;
            let mut j = 0;
            while j < offsets.len() {
                let (df, dr) = offsets[j];
                j += 1;
                let Some(to) = from.try_offset(df, dr) else {
                    continue;
                };

                bb = bb.union(to.bitboard());
            }

            arr[i] = bb;
            i += 1;
        }

        arr
    };

    MOVES[sq.idx() as usize]
}

#[inline]
pub const fn king_moves(sq: Square) -> Bitboard {
    const MOVES: &[Bitboard; Square::COUNT] = &{
        let mut arr = [Bitboard::EMPTY; Square::COUNT];

        let offsets = [
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ];

        let mut i = 0;
        while i < Square::COUNT {
            let from = Square::from_idx(i as u8);
            let mut bb = Bitboard::EMPTY;
            let mut j = 0;
            while j < offsets.len() {
                let (df, dr) = offsets[j];
                j += 1;
                let Some(to) = from.try_offset(df, dr) else {
                    continue;
                };

                bb = bb.union(to.bitboard());
            }

            arr[i] = bb;
            i += 1;
        }

        arr
    };

    MOVES[sq.idx() as usize]
}

#[inline]
pub const fn pawn_pushes(sq: Square, color: Color, blockers: Bitboard) -> Bitboard {
    let (push_dir, start_rank) = (color.signum(), Rank::R2.relative_to(color));

    let mut to = sq.bitboard().shift::<Up>(push_dir).subtract(blockers);

    if sq.rank().idx() == start_rank.idx() {
        to = to.union(to.shift::<Up>(push_dir).subtract(blockers));
    }

    to
}

#[inline]
pub const fn pawn_attacks(sq: Square, color: Color) -> Bitboard {
    const MOVES: &[[Bitboard; Square::COUNT]; Color::COUNT] = &{
        let mut arr = [[Bitboard::EMPTY; Square::COUNT]; Color::COUNT];

        let mut i = 0;
        while i < Square::COUNT {
            let sq = Square::from_idx(i as u8).bitboard();
            let white = sq.shift::<UpLeft>(1).union(sq.shift::<UpRight>(1));
            let black = sq.shift::<DownLeft>(1).union(sq.shift::<DownRight>(1));

            arr[Color::White.idx() as usize][i] = white;
            arr[Color::Black.idx() as usize][i] = black;

            i += 1;
        }

        arr
    };

    MOVES[color.idx() as usize][sq.idx() as usize]
}

#[inline]
pub const fn between_inclusive(a: Square, b: Square) -> Bitboard {
    const fn between_inclusive_comptime(mut a: Square, b: Square) -> Bitboard {
        let (fa, ra) = (a.file(), a.rank());
        let (fb, rb) = (b.file(), b.rank());

        let (df, dr) = (
            fb.idx() as i8 - fa.idx() as i8,
            rb.idx() as i8 - ra.idx() as i8,
        );

        let orth = df == 0 || dr == 0;
        let diag = df.abs() == dr.abs();

        let mut bb = a.bitboard().union(b.bitboard());
        if !orth && !diag {
            return bb;
        }

        let (df, dr) = (df.signum(), dr.signum());

        loop {
            bb = bb.union(a.bitboard());

            if a.idx() == b.idx() {
                return bb;
            }

            a = a.offset(df, dr);
        }
    }

    const BETWEEN: &[[Bitboard; Square::COUNT]; Square::COUNT] = &{
        let mut arr = [[Bitboard::EMPTY; Square::COUNT]; Square::COUNT];

        let mut i = 0;
        while i < Square::COUNT {
            let mut j = i;
            while j < Square::COUNT {
                let bb = between_inclusive_comptime(
                    Square::from_idx(i as u8),
                    Square::from_idx(j as u8),
                );

                arr[i][j] = bb;
                arr[j][i] = bb;
                j += 1;
            }
            i += 1;
        }

        arr
    };

    BETWEEN[a.idx() as usize][b.idx() as usize]
}

#[inline]
pub const fn between(a: Square, b: Square) -> Bitboard {
    const BETWEEN: &[[Bitboard; Square::COUNT]; Square::COUNT] = &{
        let mut arr = [[Bitboard::EMPTY; Square::COUNT]; Square::COUNT];

        let mut i = 0;
        while i < Square::COUNT {
            let mut j = i;
            while j < Square::COUNT {
                let bb = between_inclusive(Square::from_idx(i as u8), Square::from_idx(j as u8))
                    .subtract(
                        Square::from_idx(i as u8)
                            .bitboard()
                            .union(Square::from_idx(j as u8).bitboard()),
                    );

                arr[i][j] = bb;
                arr[j][i] = bb;
                j += 1;
            }
            i += 1;
        }

        arr
    };

    BETWEEN[a.idx() as usize][b.idx() as usize]
}

#[inline]
pub const fn line(a: Square, b: Square) -> Bitboard {
    const fn line_comptime(a: Square, b: Square) -> Bitboard {
        let (fa, ra) = (a.file(), a.rank());
        let (fb, rb) = (b.file(), b.rank());

        let (df, dr) = (
            fb.idx() as i8 - fa.idx() as i8,
            rb.idx() as i8 - ra.idx() as i8,
        );

        let mut bb = Bitboard::EMPTY;

        if fa.idx() == fb.idx() {
            bb = bb.union(fa.bitboard());
        }
        if ra.idx() == rb.idx() {
            bb = bb.union(ra.bitboard());
        }

        if df == dr {
            bb = bb.union(Bitboard::main_diag_for(a));
        }

        if df == -dr {
            bb = bb.union(Bitboard::anti_diag_for(a));
        }

        bb
    }

    const LINES: &[[Bitboard; Square::COUNT]; Square::COUNT] = &{
        let mut arr = [[Bitboard::EMPTY; Square::COUNT]; Square::COUNT];

        let mut i = 0;
        while i < Square::COUNT {
            let mut j = i;
            while j < Square::COUNT {
                let bb = line_comptime(Square::from_idx(i as u8), Square::from_idx(j as u8));

                arr[i][j] = bb;
                arr[j][i] = bb;
                j += 1;
            }
            i += 1;
        }

        arr
    };

    LINES[a.idx() as usize][b.idx() as usize]
}
