use crate::{
    common::{bitboard::Bitboard, square::Square},
    tagged_cell,
};

// Textbook hyperbola quintessence, calculates ray attacks in two opposite directions
// for a file, rank, diagonal or antidiagonal. Only used for initializing the magic LUT.
fn hq(blockers: Bitboard, sq: Square, ray: Bitboard) -> Bitboard {
    let o = (blockers & ray).0;
    let r = sq.bitboard().0;
    let o_rev = o.reverse_bits();
    let r_rev = r.reverse_bits();

    Bitboard((o.wrapping_sub(r << 1)) ^ (o_rev.wrapping_sub(r_rev << 1)).reverse_bits()) & ray
}

fn rook_moves_hq(blockers: Bitboard, sq: Square) -> Bitboard {
    hq(blockers, sq, sq.file().bitboard()) | hq(blockers, sq, sq.rank().bitboard())
}

fn bishop_moves_hq(blockers: Bitboard, sq: Square) -> Bitboard {
    hq(blockers, sq, Bitboard::main_diag_for(sq)) | hq(blockers, sq, Bitboard::anti_diag_for(sq))
}

struct Magic {
    factor: u64,
    offset: isize,
    mask: u64,
    shift: u64,
}

impl Magic {
    fn idx(&self, blockers: u64) -> usize {
        (((blockers | self.mask).wrapping_mul(self.factor) >> self.shift) as usize)
            .wrapping_add_signed(self.offset)
    }
}

// Compact variable shift black magics computed by Sp00ph

const TABLE_SIZE: usize = 76411;
tagged_cell!(
    static ATTACK_TABLE: TaggedCell<[u64; TABLE_SIZE], pub SliderTag> = TaggedCell::new();
);

pub fn rook_moves(sq: Square, blockers: Bitboard, tag: SliderTag) -> Bitboard {
    // SAFETY: The index is guaranteed to be in bounds by construction of the magics.
    unsafe {
        Bitboard(
            *ATTACK_TABLE
                .get(tag)
                .get_unchecked(ROOK_MAGICS[sq].idx(blockers.0)),
        )
    }
}

pub fn bishop_moves(sq: Square, blockers: Bitboard, tag: SliderTag) -> Bitboard {
    // SAFETY: same as above.
    unsafe {
        Bitboard(
            *ATTACK_TABLE
                .get(tag)
                .get_unchecked(BISHOP_MAGICS[sq].idx(blockers.0)),
        )
    }
}

pub fn init() -> SliderTag {
    ATTACK_TABLE.init_inplace(|table| {
        init_piece(table, rook_moves_hq, &ROOK_MAGICS);
        init_piece(table, bishop_moves_hq, &BISHOP_MAGICS);
    })
}

fn init_piece(
    table: &mut [u64; TABLE_SIZE],
    generator: fn(Bitboard, Square) -> Bitboard,
    magics: &[Magic; 64],
) {
    for &sq in Square::ALL {
        let magic = &magics[sq];

        // We use black magics, so we need to invert the mask to get the relevant blocker bits.
        let relevant_bits = !magic.mask;

        // Carry rippler loop to loop through all possible relevant blocker sets.
        let mut current = 0u64;
        loop {
            table[magic.idx(current)] = generator(Bitboard(current), sq).0;

            current = current.wrapping_sub(relevant_bits) & relevant_bits;
            if current == 0 {
                break;
            }
        }
    }
}

#[rustfmt::skip]
static ROOK_MAGICS: [Magic; 64] = [
    Magic { factor: 0x0050020428000230, mask: 0xfffefefefefefe81, shift: 52, offset:  1696 },
    Magic { factor: 0x00300018008c0004, mask: 0xfffdfdfdfdfdfd83, shift: 53, offset: 29542 },
    Magic { factor: 0x00600060804c0003, mask: 0xfffbfbfbfbfbfb85, shift: 53, offset: 37373 },
    Magic { factor: 0x00600c0060060002, mask: 0xfff7f7f7f7f7f789, shift: 53, offset: 19571 },
    Magic { factor: 0x0060030060060001, mask: 0xffefefefefefef91, shift: 53, offset: 18035 },
    Magic { factor: 0x0060034001800060, mask: 0xffdfdfdfdfdfdfa1, shift: 53, offset: 26651 },
    Magic { factor: 0x00b0017804c00008, mask: 0xffbfbfbfbfbfbfc1, shift: 53, offset: 31048 },
    Magic { factor: 0x0150002410080004, mask: 0xff7f7f7f7f7f7f81, shift: 52, offset:  -913 },
    Magic { factor: 0x0100500202280014, mask: 0xfffefefefefe81ff, shift: 53, offset: 35580 },
    Magic { factor: 0x100090002400801b, mask: 0xfffdfdfdfdfd83ff, shift: 54, offset: 69320 },
    Magic { factor: 0x0000c0180083c004, mask: 0xfffbfbfbfbfb85ff, shift: 54, offset: 64373 },
    Magic { factor: 0x94260015d5ce0002, mask: 0xfff7f7f7f7f789ff, shift: 55, offset: 70792 },
    Magic { factor: 0x823e000aaaa60001, mask: 0xffefefefefef91ff, shift: 55, offset: 70280 },
    Magic { factor: 0x2000a018a0028001, mask: 0xffdfdfdfdfdfa1ff, shift: 54, offset: 68047 },
    Magic { factor: 0x0000a010254000a0, mask: 0xffbfbfbfbfbfc1ff, shift: 54, offset: 65778 },
    Magic { factor: 0x0400300060c20030, mask: 0xff7f7f7f7f7f81ff, shift: 53, offset: 16528 },
    Magic { factor: 0x86a0003002180015, mask: 0xfffefefefe81feff, shift: 53, offset: 13593 },
    Magic { factor: 0x200c003000980004, mask: 0xfffdfdfdfd83fdff, shift: 54, offset: 56614 },
    Magic { factor: 0x4120006014000420, mask: 0xfffbfbfbfb85fbff, shift: 54, offset: 49760 },
    Magic { factor: 0x000600600c006004, mask: 0xfff7f7f7f789f7ff, shift: 54, offset: 58022 },
    Magic { factor: 0x0403006006006002, mask: 0xffefefefef91efff, shift: 54, offset: 57270 },
    Magic { factor: 0x0001806003106001, mask: 0xffdfdfdfdfa1dfff, shift: 54, offset: 54947 },
    Magic { factor: 0x043003000424002a, mask: 0xffbfbfbfbfc1bfff, shift: 54, offset: 54187 },
    Magic { factor: 0x000000d004e80018, mask: 0xff7f7f7f7f817fff, shift: 53, offset: 11745 },
    Magic { factor: 0x0a80402620100010, mask: 0xfffefefe81fefeff, shift: 53, offset:  9879 },
    Magic { factor: 0x2040600030300010, mask: 0xfffdfdfd83fdfdff, shift: 54, offset: 50685 },
    Magic { factor: 0xc120600060140004, mask: 0xfffbfbfb85fbfbff, shift: 54, offset: 52544 },
    Magic { factor: 0x4201d0001c001800, mask: 0xfff7f7f789f7f7ff, shift: 54, offset: 46736 },
    Magic { factor: 0x060408006006e001, mask: 0xffefefef91efefff, shift: 54, offset: 48736 },
    Magic { factor: 0x31008a001c001d00, mask: 0xffdfdfdfa1dfdfff, shift: 54, offset: 45728 },
    Magic { factor: 0x21a0920015390218, mask: 0xffbfbfbfc1bfbfff, shift: 54, offset: 59027 },
    Magic { factor: 0xcba05b000ab08040, mask: 0xff7f7f7f817f7fff, shift: 53, offset: 21604 },
    Magic { factor: 0x0900123000600060, mask: 0xfffefe81fefefeff, shift: 53, offset: 15021 },
    Magic { factor: 0x1400037c00c00040, mask: 0xfffdfd83fdfdfdff, shift: 54, offset: 62832 },
    Magic { factor: 0x010000af00600060, mask: 0xfffbfb85fbfbfbff, shift: 54, offset: 59958 },
    Magic { factor: 0x0041c810001c0018, mask: 0xfff7f789f7f7f7ff, shift: 54, offset: 47737 },
    Magic { factor: 0x0030006040600c00, mask: 0xffefef91efefefff, shift: 54, offset: 60920 },
    Magic { factor: 0x02000080c0c00c06, mask: 0xffdfdfa1dfdfdfff, shift: 54, offset: 61875 },
    Magic { factor: 0x14b0008280800201, mask: 0xffbfbfc1bfbfbfff, shift: 54, offset: 51662 },
    Magic { factor: 0xb0840ac1000aa581, mask: 0xff7f7f817f7f7fff, shift: 53, offset: 23586 },
    Magic { factor: 0x40000405d8005000, mask: 0xfffe81fefefefeff, shift: 53, offset: 28196 },
    Magic { factor: 0x22000c0098003004, mask: 0xfffd83fdfdfdfdff, shift: 54, offset: 55854 },
    Magic { factor: 0x0d0008401800c030, mask: 0xfffb85fbfbfbfbff, shift: 54, offset: 53416 },
    Magic { factor: 0x800002411400c018, mask: 0xfff789f7f7f7f7ff, shift: 54, offset: 66752 },
    Magic { factor: 0x0c00014031814014, mask: 0xffef91efefefefff, shift: 54, offset: 65161 },
    Magic { factor: 0x200000c00300c006, mask: 0xffdfa1dfdfdfdfff, shift: 54, offset: 63680 },
    Magic { factor: 0x201c001140014005, mask: 0xffbfc1bfbfbfbfff, shift: 54, offset: 67407 },
    Magic { factor: 0x410a0014b000b001, mask: 0xff7f817f7f7f7fff, shift: 53, offset: 25568 },
    Magic { factor: 0x50000200fb010120, mask: 0xff81fefefefefeff, shift: 53, offset: 39164 },
    Magic { factor: 0x800001816b306200, mask: 0xff83fdfdfdfdfdff, shift: 55, offset: 73350 },
    Magic { factor: 0x864002620254d600, mask: 0xff85fbfbfbfbfbff, shift: 55, offset: 72328 },
    Magic { factor: 0x024000d600cf1200, mask: 0xff89f7f7f7f7f7ff, shift: 55, offset: 71816 },
    Magic { factor: 0x2140002a0027ca00, mask: 0xff91efefefefefff, shift: 55, offset: 71304 },
    Magic { factor: 0x0b18a000180280a0, mask: 0xffa1dfdfdfdfdfff, shift: 54, offset: 68603 },
    Magic { factor: 0xf433ffbfc0b117c0, mask: 0xffc1bfbfbfbfbfff, shift: 55, offset: 72840 },
    Magic { factor: 0x128a000a02900090, mask: 0xff817f7f7f7f7fff, shift: 53, offset: 34415 },
    Magic { factor: 0xc00002420238b092, mask: 0x81fefefefefefeff, shift: 53, offset:  7834 },
    Magic { factor: 0xa500052205115252, mask: 0x83fdfdfdfdfdfdff, shift: 54, offset: 44708 },
    Magic { factor: 0x6014800156014c96, mask: 0x85fbfbfbfbfbfbff, shift: 54, offset: 42660 },
    Magic { factor: 0x4020006a0066410a, mask: 0x89f7f7f7f7f7f7ff, shift: 54, offset: 43684 },
    Magic { factor: 0x40c6000013a015e6, mask: 0x91efefefefefefff, shift: 54, offset: 41636 },
    Magic { factor: 0x1192080008040362, mask: 0xa1dfdfdfdfdfdfff, shift: 53, offset: 33090 },
    Magic { factor: 0x01100010d8114194, mask: 0xc1bfbfbfbfbfbfff, shift: 54, offset: 40612 },
    Magic { factor: 0x9b0800013401605a, mask: 0x817f7f7f7f7f7fff, shift: 53, offset:  5787 },
];

#[rustfmt::skip]
static BISHOP_MAGICS: [Magic; 64] = [
    Magic { factor: 0x400346a194005002, mask: 0xffbfdfeff7fbfdff, shift: 58, offset: 22068 },
    Magic { factor: 0x800b0480d0840a00, mask: 0xffffbfdfeff7fbff, shift: 59, offset: 24364 },
    Magic { factor: 0x02a4111050080008, mask: 0xffffffbfdfeff5ff, shift: 59, offset: 23118 },
    Magic { factor: 0x803a185410000080, mask: 0xffffffffbfddebff, shift: 59, offset: 23357 },
    Magic { factor: 0x91013a820081000c, mask: 0xfffffffffdbbd7ff, shift: 59, offset: 23827 },
    Magic { factor: 0x0021244281200001, mask: 0xfffffffdfbf7afff, shift: 59, offset: 23033 },
    Magic { factor: 0x000162418480c404, mask: 0xfffffdfbf7efdfff, shift: 59, offset: 24333 },
    Magic { factor: 0x0080893101018080, mask: 0xfffdfbf7efdfbfff, shift: 58, offset: 22304 },
    Magic { factor: 0x5006090304e2a008, mask: 0xffdfeff7fbfdffff, shift: 59, offset: 24328 },
    Magic { factor: 0x014203068150cc02, mask: 0xffbfdfeff7fbffff, shift: 59, offset: 24303 },
    Magic { factor: 0x4000a42120481000, mask: 0xffffbfdfeff5ffff, shift: 59, offset: 22870 },
    Magic { factor: 0x80323a18600a0000, mask: 0xffffffbfddebffff, shift: 59, offset: 23101 },
    Magic { factor: 0x8290a13a81810008, mask: 0xfffffffdbbd7ffff, shift: 59, offset: 23803 },
    Magic { factor: 0x0000012148810000, mask: 0xfffffdfbf7afffff, shift: 59, offset: 22777 },
    Magic { factor: 0x08800122c1848088, mask: 0xfffdfbf7efdfffff, shift: 59, offset: 24092 },
    Magic { factor: 0x962197d0c3028050, mask: 0xfffbf7efdfbfffff, shift: 59, offset: 24625 },
    Magic { factor: 0x0124081043021022, mask: 0xffeff7fbfdfffdff, shift: 59, offset: 22669 },
    Magic { factor: 0x2a90103d62d19008, mask: 0xffdfeff7fbfffbff, shift: 59, offset: 23439 },
    Magic { factor: 0x000a010500282404, mask: 0xffbfdfeff5fff5ff, shift: 57, offset: 76204 },
    Magic { factor: 0x0002000401f803c6, mask: 0xffffbfddebffebff, shift: 57, offset: 75769 },
    Magic { factor: 0xc000c00063008000, mask: 0xfffffdbbd7ffd7ff, shift: 57, offset: 38386 },
    Magic { factor: 0x06406200c0580080, mask: 0xfffdfbf7afffafff, shift: 57, offset: 52959 },
    Magic { factor: 0x169a00879182c043, mask: 0xfffbf7efdfffdfff, shift: 59, offset: 23179 },
    Magic { factor: 0x820100042f5e6016, mask: 0xfff7efdfbfffbfff, shift: 59, offset: 22362 },
    Magic { factor: 0x000a281205850020, mask: 0xfff7fbfdfffdfbff, shift: 59, offset: 24076 },
    Magic { factor: 0x2045140005014020, mask: 0xffeff7fbfffbf7ff, shift: 59, offset: 24060 },
    Magic { factor: 0x800a030206003008, mask: 0xffdfeff5fff5efff, shift: 57, offset: 52075 },
    Magic { factor: 0x4011044001040001, mask: 0xffbfddebffebddff, shift: 55, offset: 73861 },
    Magic { factor: 0x4600181810606002, mask: 0xfffdbbd7ffd7bbff, shift: 55, offset: 74847 },
    Magic { factor: 0x060020310180cc00, mask: 0xfffbf7afffaff7ff, shift: 57, offset: 38479 },
    Magic { factor: 0x4805014000512080, mask: 0xfff7efdfffdfefff, shift: 59, offset: 23955 },
    Magic { factor: 0x5001414000289044, mask: 0xffefdfbfffbfdfff, shift: 59, offset: 23704 },
    Magic { factor: 0x4140827500145400, mask: 0xfffbfdfffdfbf7ff, shift: 59, offset: 23381 },
    Magic { factor: 0x48004144a00a0a17, mask: 0xfff7fbfffbf7efff, shift: 59, offset: 23299 },
    Magic { factor: 0x02000407e8040040, mask: 0xffeff5fff5efdfff, shift: 57, offset: 75865 },
    Magic { factor: 0x40020020d001a00c, mask: 0xffddebffebddbfff, shift: 55, offset: 74352 },
    Magic { factor: 0x000020303010c0c0, mask: 0xffbbd7ffd7bbfdff, shift: 55, offset: 75290 },
    Magic { factor: 0x800020505002a140, mask: 0xfff7afffaff7fbff, shift: 57, offset: 76064 },
    Magic { factor: 0x2800812058014500, mask: 0xffefdfffdfeff7ff, shift: 59, offset: 22931 },
    Magic { factor: 0x010010604800a280, mask: 0xffdfbfffbfdfefff, shift: 59, offset: 22608 },
    Magic { factor: 0x801001056d002400, mask: 0xfffdfffdfbf7efff, shift: 59, offset: 22531 },
    Magic { factor: 0x0118020203a01200, mask: 0xfffbfffbf7efdfff, shift: 59, offset: 22413 },
    Magic { factor: 0x60c400f404080200, mask: 0xfff5fff5efdfbfff, shift: 57, offset: 76331 },
    Magic { factor: 0x2000080203f40201, mask: 0xffebffebddbfffff, shift: 57, offset: 63280 },
    Magic { factor: 0x00810048a0510280, mask: 0xffd7ffd7bbfdffff, shift: 57, offset: 75959 },
    Magic { factor: 0x0000412060a04140, mask: 0xffafffaff7fbfdff, shift: 57, offset: 76124 },
    Magic { factor: 0x0000425220580240, mask: 0xffdfffdfeff7fbff, shift: 59, offset: 22156 },
    Magic { factor: 0x34004188a3d44120, mask: 0xffbfffbfdfeff7ff, shift: 59, offset:  5748 },
    Magic { factor: 0x27000186047b3005, mask: 0xfffffdfbf7efdfff, shift: 59, offset: 24597 },
    Magic { factor: 0x0220014261454200, mask: 0xfffffbf7efdfbfff, shift: 59, offset: 24457 },
    Magic { factor: 0x2108000851242000, mask: 0xfffff5efdfbfffff, shift: 59, offset: 22013 },
    Magic { factor: 0x0100000818460440, mask: 0xffffebddbfffffff, shift: 59, offset: 22844 },
    Magic { factor: 0x0080000085414001, mask: 0xffffd7bbfdffffff, shift: 59, offset: 22266 },
    Magic { factor: 0x000d008304212040, mask: 0xffffaff7fbfdffff, shift: 59, offset: 21753 },
    Magic { factor: 0x0d0045061197a541, mask: 0xffffdfeff7fbfdff, shift: 59, offset: 24580 },
    Magic { factor: 0x000041810930b210, mask: 0xffffbfdfeff7fbff, shift: 59, offset: 23839 },
    Magic { factor: 0x040400806108939c, mask: 0xfffdfbf7efdfbfff, shift: 58, offset:   -32 },
    Magic { factor: 0x0108000861048b18, mask: 0xfffbf7efdfbfffff, shift: 59, offset: 23541 },
    Magic { factor: 0x0082000008512444, mask: 0xfff5efdfbfffffff, shift: 59, offset: 21645 },
    Magic { factor: 0x031200420180c300, mask: 0xffebddbfffffffff, shift: 59, offset: 22588 },
    Magic { factor: 0x0040000400854142, mask: 0xffd7bbfdffffffff, shift: 59, offset: 21902 },
    Magic { factor: 0x0840080082c44122, mask: 0xffaff7fbfdffffff, shift: 59, offset:    33 },
    Magic { factor: 0x84400006862147a1, mask: 0xffdfeff7fbfdffff, shift: 59, offset: 24581 },
    Magic { factor: 0x8000433800a03868, mask: 0xffbfdfeff7fbfdff, shift: 58, offset: 21811 },
];
