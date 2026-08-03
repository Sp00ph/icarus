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

    const TABLE_SIZE: usize = 87038;

    struct Magic {
        factor: u64,
        position: isize,
    }

    #[rustfmt::skip]
    const BISHOP_MAGICS: &[Magic; 64] = &[
        Magic { factor: 0x400346a194005002, position: 48941 },
        Magic { factor: 0x800b0480d0840a00, position: 71848 },
        Magic { factor: 0x02a4111050080008, position: 70683 },
        Magic { factor: 0x803a185410000080, position: 70945 },
        Magic { factor: 0x91013a820081000c, position: 34313 },
        Magic { factor: 0x0021244281200001, position: 78649 },
        Magic { factor: 0x000162418480c404, position: 67861 },
        Magic { factor: 0x0080893101018080, position: 33646 },
        Magic { factor: 0x5006090304e2a008, position: 70434 },
        Magic { factor: 0x014203068150cc02, position: 44763 },
        Magic { factor: 0x4000a42120481000, position: 69985 },
        Magic { factor: 0x80323a18600a0000, position:  5007 },
        Magic { factor: 0x8290a13a81810008, position: 50127 },
        Magic { factor: 0x0000012148810000, position: 65752 },
        Magic { factor: 0x08800122c1848088, position: 70071 },
        Magic { factor: 0x962197d0c3028050, position: 64165 },
        Magic { factor: 0x0124081043021022, position: 73972 },
        Magic { factor: 0x2a90103d62d19008, position: 70139 },
        Magic { factor: 0x000a010500282404, position: 84338 },
        Magic { factor: 0x0002000401f803c6, position: 84433 },
        Magic { factor: 0xc000c00063008000, position:  5030 },
        Magic { factor: 0x06406200c0580080, position: 34313 },
        Magic { factor: 0x169a00879182c043, position: 45277 },
        Magic { factor: 0x820100042f5e6016, position: 64288 },
        Magic { factor: 0x000a281205850020, position: 73078 },
        Magic { factor: 0x2045140005014020, position: 65502 },
        Magic { factor: 0x800a030206003008, position: 47729 },
        Magic { factor: 0x4011044001040001, position: 84560 },
        Magic { factor: 0x4600181810606002, position: 85055 },
        Magic { factor: 0x060020310180cc00, position:  5123 },
        Magic { factor: 0x4805014000512080, position: 67811 },
        Magic { factor: 0x5001414000289044, position: 36069 },
        Magic { factor: 0x4140827500145400, position: 39804 },
        Magic { factor: 0x48004144a00a0a17, position: 49471 },
        Magic { factor: 0x02000407e8040040, position: 85523 },
        Magic { factor: 0x40020020d001a00c, position: 85632 },
        Magic { factor: 0x000020303010c0c0, position: 86092 },
        Magic { factor: 0x800020505002a140, position: 86581 },
        Magic { factor: 0x2800812058014500, position: 64799 },
        Magic { factor: 0x010010604800a280, position: 46126 },
        Magic { factor: 0x801001056d002400, position: 64355 },
        Magic { factor: 0x0118020203a01200, position: 71195 },
        Magic { factor: 0x60c400f404080200, position: 86688 },
        Magic { factor: 0x2000080203f40201, position: 86751 },
        Magic { factor: 0x00810048a0510280, position: 86830 },
        Magic { factor: 0x0000412060a04140, position: 86910 },
        Magic { factor: 0x0000425220580240, position: 64216 },
        Magic { factor: 0x34004188a3d44120, position: 68190 },
        Magic { factor: 0x27000186047b3005, position: 66967 },
        Magic { factor: 0x0220014261454200, position: 43645 },
        Magic { factor: 0x2108000851242000, position: 77673 },
        Magic { factor: 0x0100000818460440, position: 47833 },
        Magic { factor: 0x0080000085414001, position: 67290 },
        Magic { factor: 0x000d008304212040, position: 67096 },
        Magic { factor: 0x0d0045061197a541, position: 82768 },
        Magic { factor: 0x000041810930b210, position: 84177 },
        Magic { factor: 0x040400806108939c, position: 65602 },
        Magic { factor: 0x0108000861048b18, position: 40136 },
        Magic { factor: 0x0082000008512444, position: 67992 },
        Magic { factor: 0x031200420180c300, position: 38057 },
        Magic { factor: 0x0040000400854142, position: 68221 },
        Magic { factor: 0x0840080082c44122, position: 42936 },
        Magic { factor: 0x84400006862147a1, position: 71861 },
        Magic { factor: 0x8000433800a03868, position: 40563 },
    ];

    #[rustfmt::skip]
    const ROOK_MAGICS: &[Magic; 64] = &[
        Magic { factor: 0x0050020428000230, position: -1406 },
        Magic { factor: 0x00300018008c0004, position:  2180 },
        Magic { factor: 0x00600060804c0003, position:  3994 },
        Magic { factor: 0x00600c0060060002, position:  5530 },
        Magic { factor: 0x0060030060060001, position:  7066 },
        Magic { factor: 0x0060034001800060, position:  8666 },
        Magic { factor: 0x0060018000c00060, position: 10202 },
        Magic { factor: 0x0150002410080004, position: 11333 },
        Magic { factor: 0x0100500202280014, position: 14678 },
        Magic { factor: 0x100090002400801b, position: 16460 },
        Magic { factor: 0x0400c01800084032, position: 17300 },
        Magic { factor: 0x0000a00a01032003, position: 18034 },
        Magic { factor: 0x0800a0050080a002, position: 18736 },
        Magic { factor: 0x2000a018a0028001, position: 19482 },
        Magic { factor: 0x0000a010254000a0, position: 20044 },
        Magic { factor: 0x0400300060c20030, position: 20599 },
        Magic { factor: 0x86a0003002180015, position: 22427 },
        Magic { factor: 0x200c003000980004, position: 24041 },
        Magic { factor: 0x4120006014000420, position: 24937 },
        Magic { factor: 0x000600600c006004, position: 25637 },
        Magic { factor: 0x0403006006006002, position: 26421 },
        Magic { factor: 0x0001806003106001, position: 27181 },
        Magic { factor: 0x043003000424002a, position: 28028 },
        Magic { factor: 0x000000d004e80018, position: 28855 },
        Magic { factor: 0x0a80402620100010, position: 30900 },
        Magic { factor: 0x2040600030300010, position: 32916 },
        Magic { factor: 0xc120600060140004, position: 33876 },
        Magic { factor: 0x4201d0001c001800, position: 34825 },
        Magic { factor: 0x060408006006e001, position: 35817 },
        Magic { factor: 0x31008a001c001d00, position: 36837 },
        Magic { factor: 0x21a0920015390218, position: 37833 },
        Magic { factor: 0xcba05b000ab08040, position: 38819 },
        Magic { factor: 0x0900123000600060, position: 40539 },
        Magic { factor: 0x1400037c00c00040, position: 42485 },
        Magic { factor: 0x010000af00600060, position: 43394 },
        Magic { factor: 0x0041c810001c0018, position: 44399 },
        Magic { factor: 0x0030006040600c00, position: 45373 },
        Magic { factor: 0x02000080c0c00c06, position: 46343 },
        Magic { factor: 0x14b0008280800201, position: 47316 },
        Magic { factor: 0xb0840ac1000aa581, position: 48224 },
        Magic { factor: 0x40000405d8005000, position: 49745 },
        Magic { factor: 0x22000c0098003004, position: 51484 },
        Magic { factor: 0x0d0008401800c030, position: 52288 },
        Magic { factor: 0x800002411400c018, position: 53213 },
        Magic { factor: 0x0c00014031814014, position: 53878 },
        Magic { factor: 0x200000c00300c006, position: 54726 },
        Magic { factor: 0x201c001140014005, position: 55381 },
        Magic { factor: 0x410a0014b000b001, position: 56237 },
        Magic { factor: 0x200000f804020120, position: 57163 },
        Magic { factor: 0x100001802a140050, position: 58725 },
        Magic { factor: 0x08014000403e0140, position: 59331 },
        Magic { factor: 0x03212000a0220120, position: 60000 },
        Magic { factor: 0x0e20a000404500a0, position: 60714 },
        Magic { factor: 0x0b18a000180280a0, position: 61352 },
        Magic { factor: 0x002800140d200120, position: 61911 },
        Magic { factor: 0x128a000a02900090, position: 62232 },
        Magic { factor: 0x0000051084102046, position: 64102 },
        Magic { factor: 0x0100048214210441, position: 68175 },
        Magic { factor: 0x0080012040088c12, position: 70189 },
        Magic { factor: 0xb82000206e404a07, position: 72227 },
        Magic { factor: 0x4100100004208853, position: 74217 },
        Magic { factor: 0x1192080008040362, position: 76263 },
        Magic { factor: 0x00000400021220cd, position: 78257 },
        Magic { factor: 0x4002000828884302, position: 80290 },
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

                    table[(ROOK_MAGICS[i].position as usize).wrapping_add(
                        ((blockers | !mask.0).wrapping_mul(ROOK_MAGICS[i].factor)
                            >> (!mask.0).count_ones()) as usize,
                    )] = moves.0;
                }
            }

            {
                // Bishops
                let mask = bishop_mask(sq);

                for j in 0..(1 << mask.popcnt()) {
                    let blockers = pdep(j, mask.0);
                    let moves = bishop_moves(sq, blockers);

                    table[(BISHOP_MAGICS[i].position as usize).wrapping_add(
                        ((blockers | !mask.0).wrapping_mul(BISHOP_MAGICS[i].factor)
                            >> (!mask.0).count_ones()) as usize,
                    )] = moves.0;
                }
            }
        }

        let preamble = stringify!(
            use icarus_common::{square::Square, bitboard::Bitboard, util::Align64};

            #[inline]
            pub const fn rook_moves(sq: Square, blockers: Bitboard) -> Bitboard {
                let sq_idx = sq.idx() as usize;
                Bitboard(unsafe {
                    *ROOK_MAGICS.0[sq_idx].data.wrapping_add(
                        (ROOK_MAGICS.0[sq_idx]
                            .factor
                            .wrapping_mul(blockers.0 | ROOK_MAGICS.0[sq_idx].mask)
                            >> ROOK_MAGICS.0[sq_idx].shift) as usize,
                    )
                })
            }

            #[inline]
            pub const fn bishop_moves(sq: Square, blockers: Bitboard) -> Bitboard {
                let sq_idx = sq.idx() as usize;
                Bitboard(unsafe {
                    *BISHOP_MAGICS.0[sq_idx].data.wrapping_add(
                        (BISHOP_MAGICS.0[sq_idx]
                            .factor
                            .wrapping_mul(blockers.0 | BISHOP_MAGICS.0[sq_idx].mask)
                            >> BISHOP_MAGICS.0[sq_idx].shift) as usize,
                    )
                })
            }

            #[repr(align(32))]
            struct Magic {
                factor: u64,
                data: *const u64,
                mask: u64,
                shift: u64,
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
                "    Magic {{ factor: {:#018x}, mask: {:#018x}, shift: {}, data: ATTACK_TABLE.0.as_ptr().wrapping_offset({:>5}) }},",
                m.factor,
                mask.0,
                mask.0.count_ones(),
                m.position,
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
                "    Magic {{ factor: {:#018x}, mask: {:#018x}, shift: {}, data: ATTACK_TABLE.0.as_ptr().wrapping_offset({:>5}) }},",
                m.factor,
                mask.0,
                mask.0.count_ones(),
                m.position,
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
