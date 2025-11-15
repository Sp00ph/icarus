use std::{
    env, fs,
    io::{self, BufWriter, Write},
    path::Path,
};

use icarus_common::{
    bitboard::Bitboard,
    lookups::bishop_rays,
    square::{File, Rank, Square},
};

fn pdep(n: u64, mask: u64) -> u64 {
    #[cfg(target_feature = "bmi2")]
    return unsafe { std::arch::x86_64::_pdep_u64(n, mask) };
    #[cfg(not(target_feature = "bmi2"))]
    {
        let mut result = 0;
        let mut mask = mask;

        for k in 0..mask.count_ones() {
            let j = mask.trailing_zeros();
            result |= ((n >> k) & 1) << j;

            mask ^= 1 << j;
        }

        result
    }
}

// TODO: pdep compression for rooks
fn pext(n: u64, mask: u64) -> u64 {
    #[cfg(target_feature = "bmi2")]
    return unsafe { std::arch::x86_64::_pext_u64(n, mask) };
    #[cfg(not(target_feature = "bmi2"))]
    {
        let mut result = 0;
        let mut mask = mask;

        for k in 0..mask.count_ones() {
            let j = mask.trailing_zeros();
            result |= ((n >> j) & 1) << k;

            mask ^= 1 << j;
        }

        result
    }
}

fn walk(mut sq: Square, df: i8, dr: i8, blockers: u64) -> Bitboard {
    let mut bb = Bitboard::EMPTY;
    let blockers = Bitboard(blockers);

    while !blockers.contains(sq) {
        sq = match sq.try_offset(df, dr) {
            Some(sq) => sq,
            None => break,
        };

        bb |= sq;
    }

    bb
}

fn rook_moves(sq: Square, blockers: u64) -> Bitboard {
    walk(sq, 1, 0, blockers)
        | walk(sq, 0, 1, blockers)
        | walk(sq, -1, 0, blockers)
        | walk(sq, 0, -1, blockers)
}

fn rook_mask(sq: Square) -> Bitboard {
    let rank_inner = sq
        .rank()
        .bitboard()
        .subtract(File::A.bitboard() | File::H.bitboard());
    let file_inner = sq
        .file()
        .bitboard()
        .subtract(Rank::R1.bitboard() | Rank::R8.bitboard());

    (rank_inner | file_inner).subtract(sq.bitboard())
}

fn bishop_moves(sq: Square, blockers: u64) -> Bitboard {
    walk(sq, 1, 1, blockers)
        | walk(sq, 1, -1, blockers)
        | walk(sq, -1, 1, blockers)
        | walk(sq, -1, -1, blockers)
}

fn bishop_mask(sq: Square) -> Bitboard {
    bishop_rays(sq)
        .subtract(Rank::R1.bitboard())
        .subtract(Rank::R8.bitboard())
        .subtract(File::A.bitboard())
        .subtract(File::H.bitboard())
}

mod magic {
    use super::*;

    // Magics are the most compact white magics by Volker Annuss: <https://www.talkchess.com/forum/viewtopic.php?p=727500>

    const TABLE_SIZE: usize = 88772;

    struct Magic {
        factor: u64,
        position: u32,
    }

    #[rustfmt::skip]
    const BISHOP_MAGICS: &[Magic; 64] = &[
        Magic { factor: 0x007fbfbfbfbfbfff, position:  5378 },
        Magic { factor: 0x0000a060401007fc, position:  4093 },
        Magic { factor: 0x0001004008020000, position:  4314 },
        Magic { factor: 0x0000806004000000, position:  6587 },
        Magic { factor: 0x0000100400000000, position:  6491 },
        Magic { factor: 0x000021c100b20000, position:  6330 },
        Magic { factor: 0x0000040041008000, position:  5609 },
        Magic { factor: 0x00000fb0203fff80, position: 22236 },
        Magic { factor: 0x0000040100401004, position:  6106 },
        Magic { factor: 0x0000020080200802, position:  5625 },
        Magic { factor: 0x0000004010202000, position: 16785 },
        Magic { factor: 0x0000008060040000, position: 16817 },
        Magic { factor: 0x0000004402000000, position:  6842 },
        Magic { factor: 0x0000000801008000, position:  7003 },
        Magic { factor: 0x000007efe0bfff80, position:  4197 },
        Magic { factor: 0x0000000820820020, position:  7356 },
        Magic { factor: 0x0000400080808080, position:  4602 },
        Magic { factor: 0x00021f0100400808, position:  4538 },
        Magic { factor: 0x00018000c06f3fff, position: 29531 },
        Magic { factor: 0x0000258200801000, position: 45393 },
        Magic { factor: 0x0000240080840000, position: 12420 },
        Magic { factor: 0x000018000c03fff8, position: 15763 },
        Magic { factor: 0x00000a5840208020, position:  5050 },
        Magic { factor: 0x0000020008208020, position:  4346 },
        Magic { factor: 0x0000804000810100, position:  6074 },
        Magic { factor: 0x0001011900802008, position:  7866 },
        Magic { factor: 0x0000804000810100, position: 32139 },
        Magic { factor: 0x000100403c0403ff, position: 57673 },
        Magic { factor: 0x00078402a8802000, position: 55365 },
        Magic { factor: 0x0000101000804400, position: 15818 },
        Magic { factor: 0x0000080800104100, position:  5562 },
        Magic { factor: 0x00004004c0082008, position:  6390 },
        Magic { factor: 0x0001010120008020, position:  7930 },
        Magic { factor: 0x000080809a004010, position: 13329 },
        Magic { factor: 0x0007fefe08810010, position:  7170 },
        Magic { factor: 0x0003ff0f833fc080, position: 27267 },
        Magic { factor: 0x007fe08019003042, position: 53787 },
        Magic { factor: 0x003fffefea003000, position:  5097 },
        Magic { factor: 0x0000101010002080, position:  6643 },
        Magic { factor: 0x0000802005080804, position:  6138 },
        Magic { factor: 0x0000808080a80040, position:  7418 },
        Magic { factor: 0x0000104100200040, position:  7898 },
        Magic { factor: 0x0003ffdf7f833fc0, position: 42012 },
        Magic { factor: 0x0000008840450020, position: 57350 },
        Magic { factor: 0x00007ffc80180030, position: 22813 },
        Magic { factor: 0x007fffdd80140028, position: 56693 },
        Magic { factor: 0x00020080200a0004, position:  5818 },
        Magic { factor: 0x0000101010100020, position:  7098 },
        Magic { factor: 0x0007ffdfc1805000, position:  4451 },
        Magic { factor: 0x0003ffefe0c02200, position:  4709 },
        Magic { factor: 0x0000000820806000, position:  4794 },
        Magic { factor: 0x0000000008403000, position: 13364 },
        Magic { factor: 0x0000000100202000, position:  4570 },
        Magic { factor: 0x0000004040802000, position:  4282 },
        Magic { factor: 0x0004010040100400, position: 14964 },
        Magic { factor: 0x00006020601803f4, position:  4026 },
        Magic { factor: 0x0003ffdfdfc28048, position:  4826 },
        Magic { factor: 0x0000000820820020, position:  7354 },
        Magic { factor: 0x0000000008208060, position:  4848 },
        Magic { factor: 0x0000000000808020, position: 15946 },
        Magic { factor: 0x0000000001002020, position: 14932 },
        Magic { factor: 0x0000000401002008, position: 16588 },
        Magic { factor: 0x0000004040404040, position:  6905 },
        Magic { factor: 0x007fff9fdf7ff813, position: 16076 }
    ];

    #[rustfmt::skip]
    const ROOK_MAGICS: &[Magic; 64] = &[
        Magic { factor: 0x00280077ffebfffe, position: 26304 },
        Magic { factor: 0x2004010201097fff, position: 35520 },
        Magic { factor: 0x0010020010053fff, position: 38592 },
        Magic { factor: 0x0040040008004002, position:  8026 },
        Magic { factor: 0x7fd00441ffffd003, position: 22196 },
        Magic { factor: 0x4020008887dffffe, position: 80870 },
        Magic { factor: 0x004000888847ffff, position: 76747 },
        Magic { factor: 0x006800fbff75fffd, position: 30400 },
        Magic { factor: 0x000028010113ffff, position: 11115 },
        Magic { factor: 0x0020040201fcffff, position: 18205 },
        Magic { factor: 0x007fe80042ffffe8, position: 53577 },
        Magic { factor: 0x00001800217fffe8, position: 62724 },
        Magic { factor: 0x00001800073fffe8, position: 34282 },
        Magic { factor: 0x00001800e05fffe8, position: 29196 },
        Magic { factor: 0x00001800602fffe8, position: 23806 },
        Magic { factor: 0x000030002fffffa0, position: 49481 },
        Magic { factor: 0x00300018010bffff, position:  2410 },
        Magic { factor: 0x0003000c0085fffb, position: 36498 },
        Magic { factor: 0x0004000802010008, position: 24478 },
        Magic { factor: 0x0004002020020004, position: 10074 },
        Magic { factor: 0x0001002002002001, position: 79315 },
        Magic { factor: 0x0001001000801040, position: 51779 },
        Magic { factor: 0x0000004040008001, position: 13586 },
        Magic { factor: 0x0000006800cdfff4, position: 19323 },
        Magic { factor: 0x0040200010080010, position: 70612 },
        Magic { factor: 0x0000080010040010, position: 83652 },
        Magic { factor: 0x0004010008020008, position: 63110 },
        Magic { factor: 0x0000040020200200, position: 34496 },
        Magic { factor: 0x0002008010100100, position: 84966 },
        Magic { factor: 0x0000008020010020, position: 54341 },
        Magic { factor: 0x0000008020200040, position: 60421 },
        Magic { factor: 0x0000820020004020, position: 86402 },
        Magic { factor: 0x00fffd1800300030, position: 50245 },
        Magic { factor: 0x007fff7fbfd40020, position: 76622 },
        Magic { factor: 0x003fffbd00180018, position: 84676 },
        Magic { factor: 0x001fffde80180018, position: 78757 },
        Magic { factor: 0x000fffe0bfe80018, position: 37346 },
        Magic { factor: 0x0001000080202001, position:   370 },
        Magic { factor: 0x0003fffbff980180, position: 42182 },
        Magic { factor: 0x0001fffdff9000e0, position: 45385 },
        Magic { factor: 0x00fffefeebffd800, position: 61659 },
        Magic { factor: 0x007ffff7ffc01400, position: 12790 },
        Magic { factor: 0x003fffbfe4ffe800, position: 16762 },
        Magic { factor: 0x001ffff01fc03000, position:     0 },
        Magic { factor: 0x000fffe7f8bfe800, position: 38380 },
        Magic { factor: 0x0007ffdfdf3ff808, position: 11098 },
        Magic { factor: 0x0003fff85fffa804, position: 21803 },
        Magic { factor: 0x0001fffd75ffa802, position: 39189 },
        Magic { factor: 0x00ffffd7ffebffd8, position: 58628 },
        Magic { factor: 0x007fff75ff7fbfd8, position: 44116 },
        Magic { factor: 0x003fff863fbf7fd8, position: 78357 },
        Magic { factor: 0x001fffbfdfd7ffd8, position: 44481 },
        Magic { factor: 0x000ffff810280028, position: 64134 },
        Magic { factor: 0x0007ffd7f7feffd8, position: 41759 },
        Magic { factor: 0x0003fffc0c480048, position:  1394 },
        Magic { factor: 0x0001ffffafd7ffd8, position: 40910 },
        Magic { factor: 0x00ffffe4ffdfa3ba, position: 66516 },
        Magic { factor: 0x007fffef7ff3d3da, position:  3897 },
        Magic { factor: 0x003fffbfdfeff7fa, position:  3930 },
        Magic { factor: 0x001fffeff7fbfc22, position: 72934 },
        Magic { factor: 0x0000020408001001, position: 72662 },
        Magic { factor: 0x0007fffeffff77fd, position: 56325 },
        Magic { factor: 0x0003ffffbf7dfeec, position: 66501 },
        Magic { factor: 0x0001ffff9dffa333, position: 14826 }
    ];

    pub fn generate(w: &mut impl Write) -> io::Result<()> {
        let mut table = vec![0u64; TABLE_SIZE];

        for i in 0..64 {
            let sq = Square::from_idx(i as u8);

            {
                // Rooks
                let mask = rook_mask(sq);

                for j in 0..(1 << mask.popcnt()) {
                    let blockers = pdep(j, mask.0);
                    let moves = rook_moves(sq, blockers);

                    table[(ROOK_MAGICS[i].position as usize)
                        + (blockers.wrapping_mul(ROOK_MAGICS[i].factor) >> 52) as usize] = moves.0;
                }
            }

            {
                // Bishops
                let mask = bishop_mask(sq);

                for j in 0..(1 << mask.popcnt()) {
                    let blockers = pdep(j, mask.0);
                    let moves = bishop_moves(sq, blockers);

                    table[(BISHOP_MAGICS[i].position as usize)
                        + (blockers.wrapping_mul(BISHOP_MAGICS[i].factor) >> 55) as usize] =
                        moves.0;
                }
            }
        }

        let preamble = stringify!(
            use icarus_common::{square::Square, bitboard::Bitboard, util::Align64};

            #[inline]
            pub const fn rook_moves(sq: Square, blockers: Bitboard) -> Bitboard {
                let sq_idx = sq.idx() as usize;
                Bitboard(unsafe {
                    *ROOK_MAGICS.0[sq_idx].data.add(
                        (ROOK_MAGICS.0[sq_idx]
                            .factor
                            .wrapping_mul(blockers.0 & ROOK_MAGICS.0[sq_idx].mask)
                            >> 52) as usize,
                    )
                })
            }

            #[inline]
            pub const fn bishop_moves(sq: Square, blockers: Bitboard) -> Bitboard {
                let sq_idx = sq.idx() as usize;
                Bitboard(unsafe {
                    *BISHOP_MAGICS.0[sq_idx].data.add(
                        (BISHOP_MAGICS.0[sq_idx]
                            .factor
                            .wrapping_mul(blockers.0 & BISHOP_MAGICS.0[sq_idx].mask)
                            >> 55) as usize,
                    )
                })
            }

            #[repr(align(32))]
            struct Magic {
                factor: u64,
                data: *const u64,
                mask: u64,
            }

            unsafe impl Sync for Magic {}
        );

        writeln!(w, "{preamble}")?;

        writeln!(
            w,
            "#[rustfmt::skip]\nstatic ROOK_MAGICS: Align64<[Magic; 64]> = Align64(["
        )?;
        for (i, m) in ROOK_MAGICS.iter().enumerate() {
            let mask = rook_mask(Square::from_idx(i as u8));
            writeln!(
                w,
                "    Magic {{ factor: {:#018x}, data: ATTACK_TABLE.0.as_ptr().wrapping_add({:#07x}), mask: {:#018x} }},",
                m.factor, m.position, mask.0
            )?;
        }
        writeln!(w, "]);\n\n")?;

        writeln!(
            w,
            "#[rustfmt::skip]\nstatic BISHOP_MAGICS: Align64<[Magic; 64]> = Align64(["
        )?;
        for (i, m) in BISHOP_MAGICS.iter().enumerate() {
            let mask = bishop_mask(Square::from_idx(i as u8));
            writeln!(
                w,
                "    Magic {{ factor: {:#018x}, data: ATTACK_TABLE.0.as_ptr().wrapping_add({:#07x}), mask: {:#018x} }},",
                m.factor, m.position, mask.0
            )?;
        }
        writeln!(w, "]);\n\n")?;

        writeln!(
            w,
            "#[rustfmt::skip]\nstatic ATTACK_TABLE: Align64<[u64; {TABLE_SIZE}]> = Align64(["
        )?;
        for ch in table.chunks(8) {
            write!(w, "    ")?;
            for i in ch {
                write!(w, "{i:#018x}, ")?;
            }
            writeln!(w)?;
        }
        writeln!(w, "]);")?;

        Ok(())
    }
}

mod bmi2 {
    use icarus_common::lookups::rook_rays;

    use super::*;

    pub fn generate(w: &mut impl Write) -> io::Result<()> {
        let mut rook_offsets = [0usize; 65];
        let mut bishop_offsets = [0usize; 65];
        for sq in 0..64 {
            rook_offsets[sq + 1] =
                rook_offsets[sq] + (1 << rook_mask(Square::from_idx(sq as u8)).popcnt());
        }
        // bishop_offsets[0] = rook_offsets[64];
        for sq in 0..64 {
            bishop_offsets[sq + 1] =
                bishop_offsets[sq] + (1 << bishop_mask(Square::from_idx(sq as u8)).popcnt());
        }

        let rook_table_size = rook_offsets[64];
        let mut rook_table = vec![0u64; rook_table_size];

        let bishop_table_size = bishop_offsets[64];
        let mut bishop_table = vec![0u64; bishop_table_size];

        for i in 0..64 {
            let sq = Square::from_idx(i as u8);

            {
                // Rooks
                let mask = rook_mask(sq);

                for j in 0..(1 << mask.popcnt()) {
                    let blockers = pdep(j as u64, mask.0);
                    let moves = rook_moves(sq, blockers);

                    rook_table[rook_offsets[i] + j] = moves.0;
                }
            }

            {
                // Bishops
                let mask = bishop_mask(sq);

                for j in 0..(1 << mask.popcnt()) {
                    let blockers = pdep(j as u64, mask.0);
                    let moves = bishop_moves(sq, blockers);

                    bishop_table[bishop_offsets[i] + j] = moves.0;
                }
            }
        }

        let preamble = stringify!(
            use icarus_common::{square::Square, bitboard::Bitboard, util::Align64};

            #[repr(align(32))]
            struct RookMagic {
                pext: u64,
                pdep: u64,
                data: *const u16,
            }
            unsafe impl Sync for RookMagic {}

            #[repr(align(16))]
            struct BishopMagic {
                pext: u64,
                data: *const u64,
            }
            unsafe impl Sync for BishopMagic {}

            #[inline]
            pub fn rook_moves(sq: Square, blockers: Bitboard) -> Bitboard {
                let sq_idx = sq.idx() as usize;

                Bitboard(unsafe {
                    core::arch::x86_64::_pdep_u64(
                        *ROOK_MAGICS.0[sq_idx]
                            .data
                            .add(core::arch::x86_64::_pext_u64(
                                blockers.0,
                                ROOK_MAGICS.0[sq_idx].pext,
                            ) as usize) as u64,
                        ROOK_MAGICS.0[sq_idx].pdep,
                    )
                })
            }

            #[inline]
            pub fn bishop_moves(sq: Square, blockers: Bitboard) -> Bitboard {
                let sq_idx = sq.idx() as usize;

                Bitboard(unsafe {
                    *BISHOP_MAGICS.0[sq_idx]
                        .data
                        .add(core::arch::x86_64::_pext_u64(
                            blockers.0,
                            BISHOP_MAGICS.0[sq_idx].pext,
                        ) as usize)
                })
            }
        );

        writeln!(w, "{preamble}")?;

        writeln!(
            w,
            "#[rustfmt::skip]\nstatic ROOK_MAGICS: Align64<[RookMagic; 64]> = unsafe {{ Align64(["
        )?;
        for (i, o) in rook_offsets[..64].iter().enumerate() {
            let sq = Square::from_idx(i as u8);
            let mask = rook_mask(sq);
            let pdep = rook_rays(sq);
            writeln!(
                w,
                "    RookMagic {{ pext: {:#018x}, pdep: {:#018x}, data: ROOK_ATTACK_TABLE.0.as_ptr().add({:#07x}) }},",
                mask.0, pdep.0, o
            )?;
        }
        writeln!(w, "]) }};\n\n")?;

        writeln!(
            w,
            "#[rustfmt::skip]\nstatic BISHOP_MAGICS: Align64<[BishopMagic; 64]> = unsafe {{ Align64(["
        )?;
        for (i, o) in bishop_offsets[..64].iter().enumerate() {
            let sq = Square::from_idx(i as u8);
            let mask = bishop_mask(sq);
            writeln!(
                w,
                "    BishopMagic {{ pext: {:#018x}, data: BISHOP_ATTACK_TABLE.0.as_ptr().add({:#07x}) }},",
                mask.0, o
            )?;
        }
        writeln!(w, "]) }};\n\n")?;

        writeln!(
            w,
            "#[rustfmt::skip]\nstatic ROOK_ATTACK_TABLE: Align64<[u16; {rook_table_size}]> = Align64(["
        )?;

        for sq in 0..64 {
            for ch in rook_table[rook_offsets[sq]..rook_offsets[sq + 1]].chunks(16) {
                write!(w, "    ")?;
                for i in ch {
                    write!(
                        w,
                        "{:#06x}, ",
                        pext(*i, rook_rays(Square::from_idx(sq as u8)).0)
                    )?;
                }
                writeln!(w)?;
            }
        }
        writeln!(w, "]);\n")?;
        writeln!(
            w,
            "#[rustfmt::skip]\nstatic BISHOP_ATTACK_TABLE: Align64<[u64; {bishop_table_size}]> = Align64(["
        )?;

        for sq in 0..64 {
            for ch in bishop_table[bishop_offsets[sq]..bishop_offsets[sq + 1]].chunks(8) {
                write!(w, "    ")?;
                for i in ch {
                    write!(w, "{i:#018x}, ",)?;
                }
                writeln!(w)?;
            }
        }

        writeln!(w, "]);")?;

        Ok(())
    }
}

#[cfg(feature = "perft-all-960")]
fn generate_perft960_tests() {
    println!("cargo::rerun-if-changed=perft960.txt");
    let results = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/perft960.txt")).unwrap();
    let file_path = Path::new(&env::var("OUT_DIR").unwrap()).join("perft_generated.rs");
    let mut writer = BufWriter::new(fs::File::create(file_path).unwrap());

    for line in results.lines() {
        let mut parts = line.split('\t');
        let number = parts.next().unwrap().trim();
        let fen = parts.next().unwrap().trim();

        let expected: Vec<_> = parts.map(|s| s.trim()).collect();
        let expected = expected.join(", ");

        writeln!(
            writer,
            "
        perft_test!(
            perft960_p{number}: {fen:?};
            {expected},
        );
        "
        )
        .unwrap();
    }
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=CARGO_CFG_TARGET_FEATURE");

    let has_bmi2 = env::var("CARGO_CFG_TARGET_FEATURE").is_ok_and(|s| s.contains("bmi2"));
    let file_path = Path::new(&env::var("OUT_DIR").unwrap()).join("generated.rs");
    let mut writer = BufWriter::new(fs::File::create(file_path).unwrap());
    if has_bmi2 {
        println!("cargo::rustc-cfg=bmi2");
        bmi2::generate(&mut writer).unwrap();
    } else {
        magic::generate(&mut writer).unwrap();
    }

    #[cfg(feature = "perft-all-960")]
    generate_perft960_tests();
}
