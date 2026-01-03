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

    // Black magics found by Volker Annuss and Niklas Fiekas
    // http://talkchess.com/forum/viewtopic.php?t=64790

    const TABLE_SIZE: usize = 87988;

    struct Magic {
        factor: u64,
        position: u32,
    }

    #[rustfmt::skip]
    const BISHOP_MAGICS: &[Magic; 64] = &[
        Magic { factor: 0xa7020080601803d8, position: 60984 },
        Magic { factor: 0x13802040400801f1, position: 66046 },
        Magic { factor: 0x0a0080181001f60c, position: 32910 },
        Magic { factor: 0x1840802004238008, position: 16369 },
        Magic { factor: 0xc03fe00100000000, position: 42115 },
        Magic { factor: 0x24c00bffff400000, position:   835 },
        Magic { factor: 0x0808101f40007f04, position: 18910 },
        Magic { factor: 0x100808201ec00080, position: 25911 },
        Magic { factor: 0xffa2feffbfefb7ff, position: 63301 },
        Magic { factor: 0x083e3ee040080801, position: 16063 },
        Magic { factor: 0xc0800080181001f8, position: 17481 },
        Magic { factor: 0x0440007fe0031000, position: 59361 },
        Magic { factor: 0x2010007ffc000000, position: 18735 },
        Magic { factor: 0x1079ffe000ff8000, position: 61249 },
        Magic { factor: 0x3c0708101f400080, position: 68938 },
        Magic { factor: 0x080614080fa00040, position: 61791 },
        Magic { factor: 0x7ffe7fff817fcff9, position: 21893 },
        Magic { factor: 0x7ffebfffa01027fd, position: 62068 },
        Magic { factor: 0x53018080c00f4001, position: 19829 },
        Magic { factor: 0x407e0001000ffb8a, position: 26091 },
        Magic { factor: 0x201fe000fff80010, position: 15815 },
        Magic { factor: 0xffdfefffde39ffef, position: 16419 },
        Magic { factor: 0xcc8808000fbf8002, position: 59777 },
        Magic { factor: 0x7ff7fbfff8203fff, position: 16288 },
        Magic { factor: 0x8800013e8300c030, position: 33235 },
        Magic { factor: 0x0420009701806018, position: 15459 },
        Magic { factor: 0x7ffeff7f7f01f7fd, position: 15863 },
        Magic { factor: 0x8700303010c0c006, position: 75555 },
        Magic { factor: 0xc800181810606000, position: 79445 },
        Magic { factor: 0x20002038001c8010, position: 15917 },
        Magic { factor: 0x087ff038000fc001, position:  8512 },
        Magic { factor: 0x00080c0c00083007, position: 73069 },
        Magic { factor: 0x00000080fc82c040, position: 16078 },
        Magic { factor: 0x000000407e416020, position: 19168 },
        Magic { factor: 0x00600203f8008020, position: 11056 },
        Magic { factor: 0xd003fefe04404080, position: 62544 },
        Magic { factor: 0xa00020c018003088, position: 80477 },
        Magic { factor: 0x7fbffe700bffe800, position: 75049 },
        Magic { factor: 0x107ff00fe4000f90, position: 32947 },
        Magic { factor: 0x7f8fffcff1d007f8, position: 59172 },
        Magic { factor: 0x0000004100f88080, position: 55845 },
        Magic { factor: 0x00000020807c4040, position: 61806 },
        Magic { factor: 0x00000041018700c0, position: 73601 },
        Magic { factor: 0x0010000080fc4080, position: 15546 },
        Magic { factor: 0x1000003c80180030, position: 45243 },
        Magic { factor: 0xc10000df80280050, position: 20333 },
        Magic { factor: 0xffffffbfeff80fdc, position: 33402 },
        Magic { factor: 0x000000101003f812, position: 25917 },
        Magic { factor: 0x0800001f40808200, position: 32875 },
        Magic { factor: 0x084000101f3fd208, position:  4639 },
        Magic { factor: 0x080000000f808081, position: 17077 },
        Magic { factor: 0x0004000008003f80, position: 62324 },
        Magic { factor: 0x08000001001fe040, position: 18159 },
        Magic { factor: 0x72dd000040900a00, position: 61436 },
        Magic { factor: 0xfffffeffbfeff81d, position: 57073 },
        Magic { factor: 0xcd8000200febf209, position: 61025 },
        Magic { factor: 0x100000101ec10082, position: 81259 },
        Magic { factor: 0x7fbaffffefe0c02f, position: 64083 },
        Magic { factor: 0x7f83fffffff07f7f, position: 56114 },
        Magic { factor: 0xfff1fffffff7ffc1, position: 57058 },
        Magic { factor: 0x0878040000ffe01f, position: 58912 },
        Magic { factor: 0x945e388000801012, position: 22194 },
        Magic { factor: 0x0840800080200fda, position: 70880 },
        Magic { factor: 0x100000c05f582008, position: 11140 },
    ];

    #[rustfmt::skip]
    const ROOK_MAGICS: &[Magic; 64] = &[
        Magic { factor: 0x80280013ff84ffff, position: 10890 },
        Magic { factor: 0x5ffbfefdfef67fff, position: 50579 },
        Magic { factor: 0xffeffaffeffdffff, position: 62020 },
        Magic { factor: 0x003000900300008a, position: 67322 },
        Magic { factor: 0x0050028010500023, position: 80251 },
        Magic { factor: 0x0020012120a00020, position: 58503 },
        Magic { factor: 0x0030006000c00030, position: 51175 },
        Magic { factor: 0x0058005806b00002, position: 83130 },
        Magic { factor: 0x7fbff7fbfbeafffc, position: 50430 },
        Magic { factor: 0x0000140081050002, position: 21613 },
        Magic { factor: 0x0000180043800048, position: 72625 },
        Magic { factor: 0x7fffe800021fffb8, position: 80755 },
        Magic { factor: 0xffffcffe7fcfffaf, position: 69753 },
        Magic { factor: 0x00001800c0180060, position: 26973 },
        Magic { factor: 0x4f8018005fd00018, position: 84972 },
        Magic { factor: 0x0000180030620018, position: 31958 },
        Magic { factor: 0x00300018010c0003, position: 69272 },
        Magic { factor: 0x0003000c0085ffff, position: 48372 },
        Magic { factor: 0xfffdfff7fbfefff7, position: 65477 },
        Magic { factor: 0x7fc1ffdffc001fff, position: 43972 },
        Magic { factor: 0xfffeffdffdffdfff, position: 57154 },
        Magic { factor: 0x7c108007befff81f, position: 53521 },
        Magic { factor: 0x20408007bfe00810, position: 30534 },
        Magic { factor: 0x0400800558604100, position: 16548 },
        Magic { factor: 0x0040200010080008, position: 46407 },
        Magic { factor: 0x0010020008040004, position: 11841 },
        Magic { factor: 0xfffdfefff7fbfff7, position: 21112 },
        Magic { factor: 0xfebf7dfff8fefff9, position: 44214 },
        Magic { factor: 0xc00000ffe001ffe0, position: 57925 },
        Magic { factor: 0x4af01f00078007c3, position: 29574 },
        Magic { factor: 0xbffbfafffb683f7f, position: 17309 },
        Magic { factor: 0x0807f67ffa102040, position: 40143 },
        Magic { factor: 0x200008e800300030, position: 64659 },
        Magic { factor: 0x0000008780180018, position: 70469 },
        Magic { factor: 0x0000010300180018, position: 62917 },
        Magic { factor: 0x4000008180180018, position: 60997 },
        Magic { factor: 0x008080310005fffa, position: 18554 },
        Magic { factor: 0x4000188100060006, position: 14385 },
        Magic { factor: 0xffffff7fffbfbfff, position:     0 },
        Magic { factor: 0x0000802000200040, position: 38091 },
        Magic { factor: 0x20000202ec002800, position: 25122 },
        Magic { factor: 0xfffff9ff7cfff3ff, position: 60083 },
        Magic { factor: 0x000000404b801800, position: 72209 },
        Magic { factor: 0x2000002fe03fd000, position: 67875 },
        Magic { factor: 0xffffff6ffe7fcffd, position: 56290 },
        Magic { factor: 0xbff7efffbfc00fff, position: 43807 },
        Magic { factor: 0x000000100800a804, position: 73365 },
        Magic { factor: 0x6054000a58005805, position: 76398 },
        Magic { factor: 0x0829000101150028, position: 20024 },
        Magic { factor: 0x00000085008a0014, position:  9513 },
        Magic { factor: 0x8000002b00408028, position: 24324 },
        Magic { factor: 0x4000002040790028, position: 22996 },
        Magic { factor: 0x7800002010288028, position: 23213 },
        Magic { factor: 0x0000001800e08018, position: 56002 },
        Magic { factor: 0xa3a80003f3a40048, position: 22809 },
        Magic { factor: 0x2003d80000500028, position: 44545 },
        Magic { factor: 0xfffff37eefefdfbe, position: 36072 },
        Magic { factor: 0x40000280090013c1, position:  4750 },
        Magic { factor: 0xbf7ffeffbffaf71f, position:  6014 },
        Magic { factor: 0xfffdffff777b7d6e, position: 36054 },
        Magic { factor: 0x48300007e8080c02, position: 78538 },
        Magic { factor: 0xafe0000fff780402, position: 28745 },
        Magic { factor: 0xee73fffbffbb77fe, position:  8555 },
        Magic { factor: 0x0002000308482882, position:  1009 },
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
                        + ((blockers | !mask.0).wrapping_mul(ROOK_MAGICS[i].factor) >> 52)
                            as usize] = moves.0;
                }
            }

            {
                // Bishops
                let mask = bishop_mask(sq);

                for j in 0..(1 << mask.popcnt()) {
                    let blockers = pdep(j, mask.0);
                    let moves = bishop_moves(sq, blockers);

                    table[(BISHOP_MAGICS[i].position as usize)
                        + ((blockers | !mask.0).wrapping_mul(BISHOP_MAGICS[i].factor) >> 55)
                            as usize] = moves.0;
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
                            .wrapping_mul(blockers.0 | ROOK_MAGICS.0[sq_idx].mask)
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
                            .wrapping_mul(blockers.0 | BISHOP_MAGICS.0[sq_idx].mask)
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
            let mask = !rook_mask(Square::from_idx(i as u8));
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
            let mask = !bishop_mask(Square::from_idx(i as u8));
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
                        .add(
                            core::arch::x86_64::_pext_u64(blockers.0, BISHOP_MAGICS.0[sq_idx].pext)
                                as usize,
                        )
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
    println!("cargo::rerun-if-env-changed=CARGO_FEATURE_USE_BMI2");

    let use_bmi2 = env::var("CARGO_CFG_TARGET_FEATURE").is_ok_and(|s| s.contains("bmi2"))
        && env::var("CARGO_FEATURE_USE_BMI2").is_ok();
    let file_path = Path::new(&env::var("OUT_DIR").unwrap()).join("generated.rs");
    let mut writer = BufWriter::new(fs::File::create(file_path).unwrap());
    if use_bmi2 {
        println!("cargo::rustc-cfg=bmi2");
        bmi2::generate(&mut writer).unwrap();
    } else {
        magic::generate(&mut writer).unwrap();
    }

    #[cfg(feature = "perft-all-960")]
    generate_perft960_tests();
}
