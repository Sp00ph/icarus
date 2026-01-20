use std::{array, fmt, mem};

use icarus_common::{
    bitboard::Bitboard,
    piece::{Color, Piece},
    square::{File, Rank, Square},
    util::enum_map::EnumMap,
};

use crate::{
    castling::{CastlingDirection, CastlingRights},
    ep_file::EnPassantFile,
    r#move::{Move, MoveFlag},
    movegen::Abort,
    zobrist::ZOBRIST,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalState {
    Checkmate(Color),
    Draw,
}

#[derive(Clone, Copy)]
pub struct Board {
    /// Bitboards per piece type, containing both white and black pieces.
    pub(crate) pieces: EnumMap<Piece, Bitboard>,
    /// Occupancy bitboards per color.
    pub(crate) colors: EnumMap<Color, Bitboard>,
    /// Piece type per square.
    pub(crate) mailbox: EnumMap<Square, Option<Piece>>,
    /// Castling rights for both sides.
    pub(crate) castling_rights: EnumMap<Color, CastlingRights>,
    /// If a pawn can be taken en passant in the next move, its file is stored in here.
    pub(crate) en_passant: Option<EnPassantFile>,
    /// Bitboard containing all stm pieces pinned to their king
    pub(crate) pinned: Bitboard,
    /// Bitboard containing all nstm pieces checking the stm king.
    pub(crate) checkers: Bitboard,
    /// Bitboard containing all squares attacked by a nstm piece.
    /// Note that nstm pieces can see through the stm king.
    pub(crate) attacked: Bitboard,
    /// Half move clock, that counts the plies since the last capture or pawn move.
    pub(crate) halfmove_clock: u8,
    /// Full move count. Gets incremented after every black move.
    pub(crate) fullmove_count: u16,
    /// The side to move next.
    pub(crate) stm: Color,
    /// Zobrist hash of the current board position.
    pub(crate) hash: u64,
    /// Zobrist hash of only the pawns.
    pub(crate) pawn_hash: u64,
    /// Zobrist hash of knights, bishops, and kings.
    pub(crate) minor_hash: u64,
    /// Zobrist hash of rooks and queens.
    pub(crate) major_hash: u64,
    /// Zobrist hash of all non-pawns of one color.
    pub(crate) nonpawn_hash: EnumMap<Color, u64>,
}

impl Board {
    #[inline]
    pub fn occupied(&self) -> Bitboard {
        self.colors[Color::White] | self.colors[Color::Black]
    }

    #[inline]
    pub fn occupied_by(&self, color: Color) -> Bitboard {
        self.colors[color]
    }

    #[inline]
    pub fn pieces(&self, piece: Piece) -> Bitboard {
        self.pieces[piece]
    }

    #[inline]
    pub fn colored_pieces(&self, piece: Piece, color: Color) -> Bitboard {
        self.pieces[piece] & self.colors[color]
    }

    #[inline]
    pub fn piece_on(&self, sq: Square) -> Option<Piece> {
        self.mailbox[sq]
    }

    #[inline]
    pub fn colored_piece_on(&self, sq: Square, color: Color) -> Option<Piece> {
        self.mailbox[sq].filter(|_| self.colors[color].contains(sq))
    }

    #[inline]
    pub fn king(&self, color: Color) -> Square {
        self.colored_pieces(Piece::King, color).next()
    }

    #[inline]
    pub fn orth_sliders(&self, color: Color) -> Bitboard {
        self.colors[color] & (self.pieces[Piece::Rook] | self.pieces[Piece::Queen])
    }

    #[inline]
    pub fn diag_sliders(&self, color: Color) -> Bitboard {
        self.colors[color] & (self.pieces[Piece::Bishop] | self.pieces[Piece::Queen])
    }

    #[inline]
    pub fn en_passant(&self) -> Option<EnPassantFile> {
        self.en_passant
    }

    #[inline]
    pub fn pinned(&self) -> Bitboard {
        self.pinned
    }

    #[inline]
    pub fn checkers(&self) -> Bitboard {
        self.checkers
    }

    #[inline]
    pub fn attacked(&self) -> Bitboard {
        self.attacked
    }

    #[inline]
    pub fn halfmove_clock(&self) -> u8 {
        self.halfmove_clock
    }

    #[inline]
    pub fn fullmove_count(&self) -> u16 {
        self.fullmove_count
    }

    #[inline]
    pub fn stm(&self) -> Color {
        self.stm
    }

    #[inline]
    pub fn hash(&self) -> u64 {
        self.hash
    }

    #[inline]
    pub fn pawn_hash(&self) -> u64 {
        self.pawn_hash
    }

    #[inline]
    pub fn minor_hash(&self) -> u64 {
        self.minor_hash
    }

    #[inline]
    pub fn major_hash(&self) -> u64 {
        self.major_hash
    }

    #[inline]
    pub fn nonpawn_hash(&self, color: Color) -> u64 {
        self.nonpawn_hash[color]
    }

    #[inline]
    pub fn castling_rights(&self) -> EnumMap<Color, CastlingRights> {
        self.castling_rights
    }

    #[inline]
    pub fn is_tactic(&self, mv: Move) -> bool {
        [MoveFlag::EnPassant, MoveFlag::Promotion].contains(&mv.flag())
            || self.occupied_by(!self.stm).contains(mv.to())
    }

    #[inline]
    pub fn is_quiet(&self, mv: Move) -> bool {
        !self.is_tactic(mv)
    }

    /// Returns whether the given move is legal on the current board. Note that this uses move generation internally, so
    /// it is rather slow. In return, it can handle _any_ kind of move, so it doesn't require any invariants of `Move` to hold.
    #[inline]
    pub fn is_legal_thorough(&self, mv: Move) -> bool {
        self.gen_moves(|moves| {
            if moves.into_iter().any(|legal| legal == mv) {
                Abort::Yes
            } else {
                Abort::No
            }
        }) == Abort::Yes
    }

    #[inline]
    pub fn terminal_state(&self) -> Option<TerminalState> {
        let any_legal = self.gen_moves(|moves| {
            if !moves.is_empty() {
                Abort::Yes
            } else {
                Abort::No
            }
        }) == Abort::Yes;
        if any_legal {
            if self.halfmove_clock >= 100 || self.insufficient_material() {
                Some(TerminalState::Draw)
            } else {
                None
            }
        } else if self.checkers.is_non_empty() {
            Some(TerminalState::Checkmate(!self.stm))
        } else {
            Some(TerminalState::Draw)
        }
    }

    /// If this returns true, then the board contains insufficient material to ever checkmate either king.
    pub fn insufficient_material(&self) -> bool {
        let pieces = &self.pieces;
        use Piece::*;

        // We check 4 conditions:
        // 1. Any pawns or rooks or queens => sufficient
        // 2. Different-colored bishops => sufficient
        // 3. At least one bishop and at least one knight => sufficient
        // 4. At least 2 knights => sufficient
        (pieces[Pawn] | pieces[Rook] | pieces[Queen]).is_empty()
            && ((pieces[Bishop].is_subset_of(Bitboard::LIGHT_SQUARES)
                | pieces[Bishop].is_subset_of(Bitboard::DARK_SQUARES))
                & !(pieces[Bishop].is_non_empty() & pieces[Knight].is_non_empty())
                & (pieces[Knight].popcnt() < 2))
    }

    #[inline]
    pub(crate) fn toggle_square(&mut self, sq: Square, color: Color, piece: Piece) {
        self.pieces[piece] ^= sq;
        self.colors[color] ^= sq;

        let key = ZOBRIST.piece(sq, piece, color);
        self.hash ^= key;
        match piece {
            Piece::Pawn => self.pawn_hash ^= key,
            Piece::Knight | Piece::Bishop | Piece::King => self.minor_hash ^= key,
            Piece::Rook | Piece::Queen => self.major_hash ^= key,
        }

        if piece != Piece::Pawn {
            self.nonpawn_hash[color] ^= key;
        }
    }

    #[inline]
    pub(crate) fn set_en_passant(&mut self, f: Option<EnPassantFile>) {
        if let Some(old) = mem::replace(&mut self.en_passant, f) {
            self.hash ^= ZOBRIST.en_passant(old.file());
        }
        if let Some(new) = f {
            self.hash ^= ZOBRIST.en_passant(new.file());
        }
    }

    #[inline]
    pub(crate) fn set_castles(&mut self, color: Color, dir: CastlingDirection, file: Option<File>) {
        if let Some(old) = self.castling_rights[color].get(dir) {
            self.hash ^= ZOBRIST.castles(old, color);
        }
        if let Some(new) = file {
            self.hash ^= ZOBRIST.castles(new, color);
        }
        self.castling_rights[color].set(dir, file);
    }

    pub fn start_pos() -> Self {
        Self::read_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub fn read_fen(fen: &str) -> Option<Self> {
        let mut parts = fen.trim().split(' ');

        let pieces = parts.next()?;
        let stm = parts.next()?;
        let castles = parts.next()?;
        let epts = parts.next()?;
        let hmc = parts.next()?;
        let fmc = parts.next()?;
        if parts.next().is_some() {
            return None;
        }

        let mut board = Board {
            pieces: Default::default(),
            mailbox: Default::default(),
            colors: Default::default(),
            castling_rights: Default::default(),
            en_passant: None,
            pinned: Bitboard::EMPTY,
            checkers: Bitboard::EMPTY,
            attacked: Bitboard::EMPTY,
            halfmove_clock: 0,
            fullmove_count: 0,
            stm: Color::White,
            hash: 0,
            pawn_hash: 0,
            minor_hash: 0,
            major_hash: 0,
            nonpawn_hash: Default::default(),
        };

        let mut rank = 8u8;
        for line in pieces.split('/') {
            rank = rank.checked_sub(1)?;
            if line == "8" {
                continue;
            }

            let mut file = 0;
            let chars = line.bytes();
            for ch in chars {
                if matches!(ch, b'1'..=b'7') {
                    file += ch - b'0';
                    continue;
                }
                if file >= 8 {
                    return None;
                }

                let piece = match ch.to_ascii_lowercase() {
                    b'p' => Piece::Pawn,
                    b'n' => Piece::Knight,
                    b'b' => Piece::Bishop,
                    b'r' => Piece::Rook,
                    b'q' => Piece::Queen,
                    b'k' => Piece::King,
                    _ => return None,
                };
                let color = Color::from_idx(ch.is_ascii_lowercase() as u8);

                let sq = Square::new(File::from_idx(file), Rank::from_idx(rank));
                board.toggle_square(sq, color, piece);
                if board.mailbox[sq].replace(piece).is_some() {
                    return None;
                }

                file += 1;
            }
        }

        // Sanity check that both sides have a king.
        if board.colored_pieces(Piece::King, Color::White).popcnt() != 1
            || board.colored_pieces(Piece::King, Color::Black).popcnt() != 1
        {
            return None;
        }

        if rank != 0 {
            return None;
        }

        board.stm = match stm {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return None,
        };

        if board.stm == Color::Black {
            board.hash ^= ZOBRIST.black_to_move;
        }

        if castles != "-" {
            for ch in castles.bytes() {
                let color = Color::from_idx(ch.is_ascii_lowercase() as u8);
                let king = board.king(color);

                let file = match ch.to_ascii_lowercase() {
                    b'a'..=b'h' => File::from_idx(ch.to_ascii_lowercase() - b'a'),
                    b'k' => (king.file().idx()..8).map(File::from_idx).find(|&f| {
                        board
                            .colored_pieces(Piece::Rook, color)
                            .contains(Square::new(f, king.rank()))
                    })?,
                    b'q' => (0..king.file().idx())
                        .rev()
                        .map(File::from_idx)
                        .find(|&f| {
                            board
                                .colored_pieces(Piece::Rook, color)
                                .contains(Square::new(f, king.rank()))
                        })?,
                    _ => return None,
                };

                let king_sq = board.king(color);
                let rook_sq = Square::new(file, king_sq.rank());
                if !board.colored_pieces(Piece::Rook, color).contains(rook_sq) {
                    return None;
                }

                let dir = if file > king_sq.file() {
                    CastlingDirection::Short
                } else {
                    CastlingDirection::Long
                };
                board.set_castles(color, dir, Some(file));
            }
        }

        if epts != "-" {
            let mut chars = epts.bytes();
            let file = match chars.next()? {
                ch @ b'a'..=b'h' => File::from_idx(ch - b'a'),
                _ => return None,
            };

            if !matches!(chars.next(), Some(b'3' | b'6')) {
                return None;
            }

            board.calc_ep_file(file);
        }

        board.halfmove_clock = hmc.parse().ok()?;
        if board.halfmove_clock >= 100 {
            return None;
        }

        board.fullmove_count = fmc.parse().ok()?;
        board.calc_threats();

        Some(board)
    }

    pub fn frc(n: usize) -> Self {
        Self::dfrc(n, n)
    }

    pub fn dfrc(w: usize, b: usize) -> Self {
        fn place_piece(b: &mut Board, sq: Square, piece: Piece, color: Color, free: &mut Bitboard) {
            assert!(b.mailbox[sq].is_none());
            assert!(free.contains(sq));
            b.mailbox[sq] = Some(piece);
            b.toggle_square(sq, color, piece);
            *free ^= sq;
        }

        fn write_color(b: &mut Board, n: usize, color: Color) {
            let (n, light_bishop) = (n / 4, n % 4);
            let (n, dark_bishop) = (n / 4, n % 4);
            let (knights, queen) = (n / 6, n % 6);

            let rank = Rank::R1.relative_to(color);
            let mut free = rank.bitboard();
            let light_bishop =
                Square::new([File::B, File::D, File::F, File::H][light_bishop], rank);
            let dark_bishop = Square::new([File::A, File::C, File::E, File::G][dark_bishop], rank);
            place_piece(b, light_bishop, Piece::Bishop, color, &mut free);
            place_piece(b, dark_bishop, Piece::Bishop, color, &mut free);

            let queen = free.into_iter().nth(queen).unwrap();
            place_piece(b, queen, Piece::Queen, color, &mut free);

            #[rustfmt::skip]
            let (knight1, knight2) = [
                (0, 1), (0, 2), (0, 3), (0, 4),
                (1, 2), (1, 3), (1, 4),
                (2, 3), (2, 4),
                (3, 4),
            ][knights];

            let knight1 = free.into_iter().nth(knight1).unwrap();
            let knight2 = free.into_iter().nth(knight2).unwrap();

            place_piece(b, knight1, Piece::Knight, color, &mut free);
            place_piece(b, knight2, Piece::Knight, color, &mut free);

            let mut free = free.into_iter();
            let [rook1, king, rook2] = array::from_fn(|_| free.next().unwrap());

            place_piece(b, rook1, Piece::Rook, color, &mut { Bitboard::ALL });
            place_piece(b, rook2, Piece::Rook, color, &mut { Bitboard::ALL });
            place_piece(b, king, Piece::King, color, &mut { Bitboard::ALL });

            b.set_castles(color, CastlingDirection::Long, Some(rook1.file()));
            b.set_castles(color, CastlingDirection::Short, Some(rook2.file()));

            let pawn_rank = Rank::R2.relative_to(color);
            for file in File::all() {
                place_piece(b, Square::new(file, pawn_rank), Piece::Pawn, color, &mut {
                    Bitboard::ALL
                });
            }
        }

        assert!(w < 960 && b < 960);
        let mut board = Board {
            pieces: Default::default(),
            mailbox: Default::default(),
            colors: Default::default(),
            castling_rights: Default::default(),
            en_passant: None,
            pinned: Bitboard::EMPTY,
            checkers: Bitboard::EMPTY,
            attacked: Bitboard::EMPTY,
            halfmove_clock: 0,
            fullmove_count: 1,
            stm: Color::White,
            hash: 0,
            pawn_hash: 0,
            minor_hash: 0,
            major_hash: 0,
            nonpawn_hash: Default::default(),
        };
        write_color(&mut board, w, Color::White);
        write_color(&mut board, b, Color::Black);
        board.calc_threats();

        board
    }

    pub fn fen(&self, chess960: bool) -> String {
        use std::fmt::Write;
        let mut res = String::new();

        for &rank in Rank::ALL.iter().rev() {
            let mut gap = 0;

            for &file in File::ALL {
                let sq = Square::new(file, rank);
                if let Some(pt) = self.piece_on(sq) {
                    if gap != 0 {
                        write!(res, "{gap}").unwrap();
                        gap = 0;
                    }
                    let color = Color::from_idx(self.colors[Color::Black].contains(sq) as u8);
                    res.push(pt.to_char(color));
                } else {
                    gap += 1;
                }
            }
            if gap != 0 {
                write!(res, "{gap}").unwrap();
            }
            res.push(if rank != Rank::R1 { '/' } else { ' ' });
        }

        write!(res, "{} ", self.stm).unwrap();

        let mut castles = String::new();
        if let Some(f) = self.castling_rights[Color::White].get(CastlingDirection::Short) {
            let ch = if !chess960 { 'K' } else { f.to_char() };
            castles.push(ch);
        }
        if let Some(f) = self.castling_rights[Color::White].get(CastlingDirection::Long) {
            let ch = if !chess960 { 'Q' } else { f.to_char() };
            castles.push(ch);
        }
        if let Some(f) = self.castling_rights[Color::Black].get(CastlingDirection::Short) {
            let ch = if !chess960 { 'K' } else { f.to_char() };
            castles.push(ch.to_ascii_lowercase());
        }
        if let Some(f) = self.castling_rights[Color::Black].get(CastlingDirection::Long) {
            let ch = if !chess960 { 'Q' } else { f.to_char() };
            castles.push(ch.to_ascii_lowercase());
        }

        if castles.is_empty() {
            castles.push('-');
        }

        write!(res, "{castles} ").unwrap();

        let ep = if let Some(f) = self.en_passant {
            format!("{:#}{:}", f.file(), Rank::R6.relative_to(self.stm))
        } else {
            "-".to_string()
        };

        write!(res, "{ep} {} {}", self.halfmove_clock, self.fullmove_count).unwrap();

        res
    }

    pub fn print(&self, chess960: bool) {
        println!("╔═══╤═══╤═══╤═══╤═══╤═══╤═══╤═══╗");

        for &rank in Rank::ALL.iter().rev() {
            print!("║");
            for &file in File::ALL {
                let sq = Square::new(file, rank);
                let mut ch = match self.piece_on(sq) {
                    None => ' ',
                    Some(Piece::Pawn) => 'P',
                    Some(Piece::Knight) => 'N',
                    Some(Piece::Bishop) => 'B',
                    Some(Piece::Rook) => 'R',
                    Some(Piece::Queen) => 'Q',
                    Some(Piece::King) => 'K',
                };

                if self.colors[Color::Black].contains(sq) {
                    ch = ch.to_ascii_lowercase();
                }

                print!(" {ch} ");
                print!("{}", if file == File::H { '║' } else { '│' });
            }
            if rank != Rank::R1 {
                println!(" {rank:?}\n╟───┼───┼───┼───┼───┼───┼───┼───╢");
            }
        }
        println!(" {:?}\n╚═══╧═══╧═══╧═══╧═══╧═══╧═══╧═══╝", Rank::R1);

        for file in File::ALL {
            print!("  {file:?} ");
        }

        println!("\n\nFEN: {}", self.fen(chess960));
        println!("Zobrist key: {:#018x}", self.hash)
    }

    #[inline]
    pub fn parse_move(&self, lan: &str, chess960: bool) -> Option<Move> {
        if !(4..=5).contains(&lan.len()) {
            return None;
        }

        let from = Square::parse(&lan[..2])?;
        let to = Square::parse(&lan[2..4])?;
        let promote_to = match lan.as_bytes().get(4) {
            Some(&b) => Some(Piece::from_char(b as char)?),
            None => None,
        };

        if let Some(pt) = promote_to {
            if pt == Piece::Pawn || pt == Piece::King {
                return None;
            }
            return Some(Move::new_promotion(from, to, pt));
        }

        let castle_file = 'castle: {
            if self.piece_on(from) != Some(Piece::King) {
                break 'castle None;
            }

            if !chess960 && from.file() == File::E && [File::C, File::G].contains(&to.file()) {
                let dir = if to.file() == File::C {
                    CastlingDirection::Long
                } else {
                    CastlingDirection::Short
                };
                break 'castle Some(self.castling_rights[self.stm].get(dir)?);
            }

            if self.colored_piece_on(to, self.stm) == Some(Piece::Rook) {
                break 'castle Some(to.file());
            }

            None
        };

        if let Some(cf) = castle_file {
            return Some(Move::new(
                from,
                Square::new(cf, to.rank()),
                MoveFlag::Castle,
            ));
        }

        let is_ep = self.piece_on(from) == Some(Piece::Pawn)
            && from.rank() == Rank::R5.relative_to(self.stm)
            && self.en_passant.map(|ep| ep.file()) == Some(to.file());

        Some(Move::new(
            from,
            to,
            if is_ep {
                MoveFlag::EnPassant
            } else {
                MoveFlag::None
            },
        ))
    }
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Board").field(&self.fen(true)).finish()
    }
}
