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

    // Variable shift black magics, self generated. Without any overlap, they require 79225 entries.
    // By taking advantage of gaps, we overlap the sections for a final table size of 77519 entries
    // for bishops and rooks combined.

    const TABLE_SIZE: usize = 77519;

    struct Magic {
        factor: u64,
        position: isize,
        shift: u32,
    }

    #[rustfmt::skip]
    const BISHOP_MAGICS: &[Magic; 64] = &[
        Magic { factor: 0x400346a194005002, shift: 58, position: 23618 },
        Magic { factor: 0x800b0480d0840a00, shift: 59, position: 25914 },
        Magic { factor: 0x02a4111050080008, shift: 59, position: 24668 },
        Magic { factor: 0x803a185410000080, shift: 59, position: 24907 },
        Magic { factor: 0x91013a820081000c, shift: 59, position: 25377 },
        Magic { factor: 0x0021244281200001, shift: 59, position: 24583 },
        Magic { factor: 0x000162418480c404, shift: 59, position: 25883 },
        Magic { factor: 0x0080893101018080, shift: 58, position: 23854 },
        Magic { factor: 0x5006090304e2a008, shift: 59, position: 25878 },
        Magic { factor: 0x014203068150cc02, shift: 59, position: 25853 },
        Magic { factor: 0x4000a42120481000, shift: 59, position: 24420 },
        Magic { factor: 0x80323a18600a0000, shift: 59, position: 24651 },
        Magic { factor: 0x8290a13a81810008, shift: 59, position: 25353 },
        Magic { factor: 0x0000012148810000, shift: 59, position: 24327 },
        Magic { factor: 0x08800122c1848088, shift: 59, position: 25642 },
        Magic { factor: 0x962197d0c3028050, shift: 59, position: 26175 },
        Magic { factor: 0x0124081043021022, shift: 59, position: 24219 },
        Magic { factor: 0x2a90103d62d19008, shift: 59, position: 24989 },
        Magic { factor: 0x000a010500282404, shift: 57, position: 77312 },
        Magic { factor: 0x0002000401f803c6, shift: 57, position: 76877 },
        Magic { factor: 0xc000c00063008000, shift: 57, position: 40436 },
        Magic { factor: 0x06406200c0580080, shift: 57, position: 54018 },
        Magic { factor: 0x169a00879182c043, shift: 59, position: 24729 },
        Magic { factor: 0x820100042f5e6016, shift: 59, position: 23912 },
        Magic { factor: 0x000a281205850020, shift: 59, position: 25626 },
        Magic { factor: 0x2045140005014020, shift: 59, position: 25610 },
        Magic { factor: 0x800a030206003008, shift: 57, position: 53134 },
        Magic { factor: 0x4011044001040001, shift: 55, position: 74969 },
        Magic { factor: 0x4600181810606002, shift: 55, position: 75955 },
        Magic { factor: 0x060020310180cc00, shift: 57, position: 40529 },
        Magic { factor: 0x4805014000512080, shift: 59, position: 25505 },
        Magic { factor: 0x5001414000289044, shift: 59, position: 25254 },
        Magic { factor: 0x4140827500145400, shift: 59, position: 24931 },
        Magic { factor: 0x48004144a00a0a17, shift: 59, position: 24849 },
        Magic { factor: 0x02000407e8040040, shift: 57, position: 76973 },
        Magic { factor: 0x40020020d001a00c, shift: 55, position: 75460 },
        Magic { factor: 0x000020303010c0c0, shift: 55, position: 76398 },
        Magic { factor: 0x800020505002a140, shift: 57, position: 77172 },
        Magic { factor: 0x2800812058014500, shift: 59, position: 24481 },
        Magic { factor: 0x010010604800a280, shift: 59, position: 24158 },
        Magic { factor: 0x801001056d002400, shift: 59, position: 24081 },
        Magic { factor: 0x0118020203a01200, shift: 59, position: 23963 },
        Magic { factor: 0x60c400f404080200, shift: 57, position: 77439 },
        Magic { factor: 0x2000080203f40201, shift: 57, position: 65206 },
        Magic { factor: 0x00810048a0510280, shift: 57, position: 77067 },
        Magic { factor: 0x0000412060a04140, shift: 57, position: 77232 },
        Magic { factor: 0x0000425220580240, shift: 59, position: 23706 },
        Magic { factor: 0x34004188a3d44120, shift: 59, position:  5747 },
        Magic { factor: 0x27000186047b3005, shift: 59, position: 26147 },
        Magic { factor: 0x0220014261454200, shift: 59, position: 26007 },
        Magic { factor: 0x2108000851242000, shift: 59, position: 23563 },
        Magic { factor: 0x0100000818460440, shift: 59, position: 24394 },
        Magic { factor: 0x0080000085414001, shift: 59, position: 23816 },
        Magic { factor: 0x000d008304212040, shift: 59, position: 23303 },
        Magic { factor: 0x0d0045061197a541, shift: 59, position: 26130 },
        Magic { factor: 0x000041810930b210, shift: 59, position: 25389 },
        Magic { factor: 0x040400806108939c, shift: 58, position:   -33 },
        Magic { factor: 0x0108000861048b18, shift: 59, position: 25091 },
        Magic { factor: 0x0082000008512444, shift: 59, position: 23195 },
        Magic { factor: 0x031200420180c300, shift: 59, position: 24138 },
        Magic { factor: 0x0040000400854142, shift: 59, position: 23452 },
        Magic { factor: 0x0840080082c44122, shift: 59, position:    32 },
        Magic { factor: 0x84400006862147a1, shift: 59, position: 26131 },
        Magic { factor: 0x8000433800a03868, shift: 58, position: 23361 },
    ];

    #[rustfmt::skip]
    const ROOK_MAGICS: &[Magic; 64] = &[
        Magic { factor: 0x0050020428000230, shift: 52, position:  1695 },
        Magic { factor: 0x00300018008c0004, shift: 53, position: 28253 },
        Magic { factor: 0x00600060804c0003, shift: 53, position: 39423 },
        Magic { factor: 0x00600c0060060002, shift: 53, position: 21121 },
        Magic { factor: 0x0060030060060001, shift: 53, position: 19585 },
        Magic { factor: 0x0060034001800060, shift: 53, position: 26713 },
        Magic { factor: 0x0060018000c00060, shift: 53, position: 18049 },
        Magic { factor: 0x0150002410080004, shift: 52, position:  -914 },
        Magic { factor: 0x0100500202280014, shift: 53, position: 36441 },
        Magic { factor: 0x100090002400801b, shift: 54, position: 70428 },
        Magic { factor: 0x0400c01800084032, shift: 54, position: 56110 },
        Magic { factor: 0x94260015d5ce0002, shift: 55, position: 71900 },
        Magic { factor: 0x823e000aaaa60001, shift: 55, position: 71388 },
        Magic { factor: 0x2000a018a0028001, shift: 54, position: 68296 },
        Magic { factor: 0x0000a010254000a0, shift: 54, position: 66888 },
        Magic { factor: 0x0400300060c20030, shift: 53, position: 14991 },
        Magic { factor: 0x86a0003002180015, shift: 53, position: 16819 },
        Magic { factor: 0x200c003000980004, shift: 54, position: 59308 },
        Magic { factor: 0x4120006014000420, shift: 54, position: 47802 },
        Magic { factor: 0x000600600c006004, shift: 54, position: 59948 },
        Magic { factor: 0x0403006006006002, shift: 54, position: 58425 },
        Magic { factor: 0x0001806003106001, shift: 54, position: 56870 },
        Magic { factor: 0x043003000424002a, shift: 54, position: 55246 },
        Magic { factor: 0x000000d004e80018, shift: 53, position: 11744 },
        Magic { factor: 0x0a80402620100010, shift: 53, position:  9878 },
        Magic { factor: 0x2040600030300010, shift: 54, position: 51744 },
        Magic { factor: 0xc120600060140004, shift: 54, position: 53603 },
        Magic { factor: 0x4201d0001c001800, shift: 54, position: 48751 },
        Magic { factor: 0x060408006006e001, shift: 54, position: 50751 },
        Magic { factor: 0x31008a001c001d00, shift: 54, position: 46787 },
        Magic { factor: 0x21a0920015390218, shift: 54, position: 60953 },
        Magic { factor: 0xcba05b000ab08040, shift: 53, position: 23154 },
        Magic { factor: 0x0900123000600060, shift: 53, position: 13484 },
        Magic { factor: 0x1400037c00c00040, shift: 54, position: 64758 },
        Magic { factor: 0x010000af00600060, shift: 54, position: 61884 },
        Magic { factor: 0x0041c810001c0018, shift: 54, position: 49752 },
        Magic { factor: 0x0030006040600c00, shift: 54, position: 62846 },
        Magic { factor: 0x02000080c0c00c06, shift: 54, position: 63801 },
        Magic { factor: 0x14b0008280800201, shift: 54, position: 52721 },
        Magic { factor: 0xb0840ac1000aa581, shift: 53, position: 25136 },
        Magic { factor: 0x40000405d8005000, shift: 53, position: 31284 },
        Magic { factor: 0x22000c0098003004, shift: 54, position: 57777 },
        Magic { factor: 0x0d0008401800c030, shift: 54, position: 54475 },
        Magic { factor: 0x800002411400c018, shift: 54, position: 69145 },
        Magic { factor: 0x0c00014031814014, shift: 54, position: 66271 },
        Magic { factor: 0x200000c00300c006, shift: 54, position: 65606 },
        Magic { factor: 0x201c001140014005, shift: 54, position: 67656 },
        Magic { factor: 0x410a0014b000b001, shift: 53, position: 30260 },
        Magic { factor: 0x200000f804020120, shift: 53, position: 40839 },
        Magic { factor: 0x800001816b306200, shift: 55, position: 74458 },
        Magic { factor: 0x864002620254d600, shift: 55, position: 73436 },
        Magic { factor: 0x024000d600cf1200, shift: 55, position: 72924 },
        Magic { factor: 0x2140002a0027ca00, shift: 55, position: 72412 },
        Magic { factor: 0x0b18a000180280a0, shift: 54, position: 69711 },
        Magic { factor: 0xf433ffbfc0b117c0, shift: 55, position: 73948 },
        Magic { factor: 0x128a000a02900090, shift: 53, position: 37770 },
        Magic { factor: 0xc00002420238b092, shift: 53, position:  7833 },
        Magic { factor: 0xa500052205115252, shift: 54, position: 45767 },
        Magic { factor: 0x6014800156014c96, shift: 54, position: 43719 },
        Magic { factor: 0x4020006a0066410a, shift: 54, position: 44743 },
        Magic { factor: 0x40c6000013a015e6, shift: 54, position: 42695 },
        Magic { factor: 0x1192080008040362, shift: 53, position: 33139 },
        Magic { factor: 0x00000400021220cd, shift: 53, position: 35131 },
        Magic { factor: 0x9b0800013401605a, shift: 53, position:  5786 },
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
                    let idx = (ROOK_MAGICS[i].position as usize).wrapping_add(
                        ((blockers | !mask.0).wrapping_mul(ROOK_MAGICS[i].factor)
                            >> ROOK_MAGICS[i].shift) as usize,
                    );

                    let old = std::mem::replace(&mut table[idx], moves.0);
                    assert!(old == moves.0 || old == 0);
                }
            }

            {
                // Bishops
                let mask = bishop_mask(sq);

                for j in 0..(1 << mask.popcnt()) {
                    let blockers = pdep(j, mask.0);
                    let moves = bishop_moves(sq, blockers);
                    let idx = (BISHOP_MAGICS[i].position as usize).wrapping_add(
                        ((blockers | !mask.0).wrapping_mul(BISHOP_MAGICS[i].factor)
                            >> BISHOP_MAGICS[i].shift) as usize,
                    );

                    let old = std::mem::replace(&mut table[idx], moves.0);
                    assert!(old == moves.0 || old == 0);
                }
            }
        }

        let preamble = stringify!(
            use icarus_common::{square::Square, bitboard::Bitboard, util::Align64};

            #[inline(never)]
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
                m.factor, mask.0, m.shift, m.position,
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
                m.factor, mask.0, m.shift, m.position,
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
