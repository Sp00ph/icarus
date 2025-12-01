use icarus_common::{
    bitboard::Bitboard,
    direction::{Down, DownLeft, DownRight, Up, UpLeft, UpRight},
    lookups::{
        between, between_inclusive, bishop_rays, king_moves, knight_moves, line, pawn_attacks,
        rook_rays,
    },
    r#move::{Move, MoveFlag, PieceMoves},
    piece::{Color, Piece},
    square::{File, Rank, Square},
};

use crate::{
    attack_generators::{bishop_moves, rook_moves},
    board::Board,
    castling::CastlingDirection,
    ep_file::EnPassantFile,
    zobrist::ZOBRIST,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Abort {
    #[default]
    No,
    Yes,
}

macro_rules! abort_if {
    ($e:expr) => {
        if let Abort::Yes = $e {
            return Abort::Yes;
        }
    };
}

#[inline(always)]
fn color<const WHITE: bool>() -> Color {
    Color::from_idx(!WHITE as u8)
}

impl Board {
    #[inline]
    fn targets<const IN_CHECK: bool, const WHITE: bool>(&self) -> Bitboard {
        let stm = color::<WHITE>();

        let b = if IN_CHECK {
            let checker = self.checkers().next();
            let king = self.king(stm);
            between_inclusive(checker, king)
        } else {
            Bitboard::ALL
        };

        b.subtract(self.colors[stm])
    }

    #[inline]
    fn pawn_noisies<const WHITE: bool, V: FnMut(PieceMoves) -> Abort>(
        &self,
        visitor: &mut V,
        targets: Bitboard,
    ) -> Abort {
        let stm = color::<WHITE>();
        let push_dir = stm.signum();
        let our_pawns = self.colored_pieces(Piece::Pawn, stm);
        let our_king = self.king(stm);

        let capture_targets = targets & self.colors[!stm];

        {
            // Up-left captures. Any pinned pawns that are not on the same
            // anti diagonal as the king may not make such captures.

            let pinned_pawns = self.pinned & !Bitboard::anti_diag_for(our_king);

            for from in capture_targets.shift::<DownRight>(push_dir) & our_pawns & !pinned_pawns {
                let flag = if from.rank() == Rank::R7.relative_to(stm) {
                    MoveFlag::Promotion
                } else {
                    MoveFlag::None
                };

                abort_if!(visitor(PieceMoves::new(
                    flag,
                    Piece::Pawn,
                    from,
                    from.bitboard().shift::<UpLeft>(push_dir),
                )));
            }
        }

        {
            // Up-right captures. Any pinned pawns that are not on the same
            // main diagonal as the king may not make such captures.

            let pinned_pawns = self.pinned & !Bitboard::main_diag_for(our_king);

            for from in capture_targets.shift::<DownLeft>(push_dir) & our_pawns & !pinned_pawns {
                let flag = if from.rank() == Rank::R7.relative_to(stm) {
                    MoveFlag::Promotion
                } else {
                    MoveFlag::None
                };

                abort_if!(visitor(PieceMoves::new(
                    flag,
                    Piece::Pawn,
                    from,
                    from.bitboard().shift::<UpRight>(push_dir),
                )));
            }
        }

        {
            // Promoting pawn pushes. Any pinned pawns that are not on the same file
            // as the king may not make such moves.

            let pinned_pawns = self.pinned & !our_king.file().bitboard();

            let promo_push_targets =
                Rank::R8.relative_to(stm).bitboard() & !self.colors[!stm] & targets;

            for from in promo_push_targets.shift::<Down>(push_dir) & our_pawns & !pinned_pawns {
                abort_if!(visitor(PieceMoves::new(
                    MoveFlag::Promotion,
                    Piece::Pawn,
                    from,
                    from.bitboard().shift::<Up>(push_dir),
                )));
            }
        }

        if let Some(ep) = self.en_passant {
            let to = Square::new(ep.file(), Rank::R6.relative_to(stm));

            // We already precalculated the legal en passants, so we don't any more checks here.
            for from in ep.attacker_bb(stm) {
                abort_if!(visitor(PieceMoves::new(
                    MoveFlag::EnPassant,
                    Piece::Pawn,
                    from,
                    to.bitboard(),
                )));
            }
        }

        Abort::No
    }

    #[inline]
    fn pawn_quiets<const WHITE: bool, V: FnMut(PieceMoves) -> Abort>(
        &self,
        visitor: &mut V,
        targets: Bitboard,
    ) -> Abort {
        let stm = color::<WHITE>();
        let push_dir = stm.signum();
        let our_pawns = self.colored_pieces(Piece::Pawn, stm);
        let our_king = self.king(stm);

        // We don't allow pushes to the 8th rank here, because they would have to promote,
        // and therefore not be quiet. And since pushes can't capture, we exclude the opponent's
        // bitboard.
        let push_targets = !Rank::R8.relative_to(stm).bitboard() & !self.occupied();

        // Any pinned pawns that are not on the same file as our king may not push.
        let pinned_pawns = self.pinned & !our_king.file().bitboard();

        // Pawns that can be pushed one square. There are some false positives in here
        // (e.g. pushes that don't break a check) that are filtered out later.
        let single_push_from = push_targets.shift::<Down>(push_dir) & our_pawns & !pinned_pawns;

        // Pawns on the starting rank, that can be pushed one square, that have a valid dst square 2
        // above them can be double pushed.
        let double_push_from = Rank::R2.relative_to(stm).bitboard()
            & single_push_from
            & push_targets.shift::<Down>(2 * push_dir);

        for from in single_push_from & targets.shift::<Down>(push_dir) {
            abort_if!(visitor(PieceMoves::new(
                MoveFlag::None,
                Piece::Pawn,
                from,
                from.bitboard().shift::<Up>(push_dir),
            )));
        }

        for from in double_push_from & targets.shift::<Down>(2 * push_dir) {
            abort_if!(visitor(PieceMoves::new(
                MoveFlag::None,
                Piece::Pawn,
                from,
                Square::new(from.file(), Rank::R4.relative_to(stm)).bitboard(),
            )));
        }

        Abort::No
    }

    #[inline]
    fn knight_moves<V: FnMut(PieceMoves) -> Abort>(
        &self,
        visitor: &mut V,
        targets: Bitboard,
    ) -> Abort {
        for from in self.colored_pieces(Piece::Knight, self.stm) & !self.pinned {
            let to = knight_moves(from) & targets;
            if to.is_non_empty() {
                abort_if!(visitor(PieceMoves::new(
                    MoveFlag::None,
                    Piece::Knight,
                    from,
                    to
                )));
            }
        }
        Abort::No
    }

    #[inline]
    fn sliders<V: FnMut(PieceMoves) -> Abort, F: Fn(Square, Bitboard) -> Bitboard>(
        &self,
        visitor: &mut V,
        targets: Bitboard,
        sliders: Bitboard,
        moves: F,
    ) -> Abort {
        let from = self.colors[self.stm] & sliders;
        let blockers = self.occupied();
        let our_king = self.king(self.stm);

        for unpinned in from & !self.pinned {
            let to = moves(unpinned, blockers) & targets;
            if to.is_non_empty() {
                abort_if!(visitor(PieceMoves::new(
                    MoveFlag::None,
                    self.piece_on(unpinned).unwrap(),
                    unpinned,
                    to,
                )));
            }
        }

        for pinned in from & self.pinned {
            let ray = line(our_king, pinned);
            let to = moves(pinned, blockers) & targets & ray;
            if to.is_non_empty() {
                abort_if!(visitor(PieceMoves::new(
                    MoveFlag::None,
                    self.piece_on(pinned).unwrap(),
                    pinned,
                    to,
                )));
            }
        }

        Abort::No
    }

    #[inline]
    fn diag_slider_moves<V: FnMut(PieceMoves) -> Abort>(
        &self,
        visitor: &mut V,
        targets: Bitboard,
    ) -> Abort {
        self.sliders(
            visitor,
            targets,
            self.pieces[Piece::Queen] | self.pieces[Piece::Bishop],
            bishop_moves,
        )
    }

    #[inline]
    fn orth_slider_moves<V: FnMut(PieceMoves) -> Abort>(
        &self,
        visitor: &mut V,
        targets: Bitboard,
    ) -> Abort {
        self.sliders(
            visitor,
            targets,
            self.pieces[Piece::Queen] | self.pieces[Piece::Rook],
            rook_moves,
        )
    }

    #[inline]
    fn king_moves<const IN_CHECK: bool, V: FnMut(PieceMoves) -> Abort>(
        &self,
        visitor: &mut V,
    ) -> Abort {
        let our_king = self.king(self.stm);

        {
            // Regular king moves.
            let to = king_moves(our_king) & !self.attacked & !self.colors[self.stm];
            if to.is_non_empty() {
                abort_if!(visitor(PieceMoves::new(
                    MoveFlag::None,
                    Piece::King,
                    our_king,
                    to
                )));
            }
        }

        if !IN_CHECK {
            // Castles.
            let rank = Rank::R1.relative_to(self.stm);

            for dir in [CastlingDirection::Long, CastlingDirection::Short] {
                if let Some(rook_file) = self.castling_rights[self.stm].get(dir) {
                    let king_dst = Square::new(dir.king_dst(), rank);
                    let rook_dst = Square::new(dir.rook_dst(), rank);
                    let rook_sq = Square::new(rook_file, rank);

                    // Only possible in Chess960
                    if self.pinned.contains(rook_sq) {
                        continue;
                    }

                    let must_be_safe = between_inclusive(our_king, king_dst);
                    let must_be_empty =
                        must_be_safe | between_inclusive(our_king, rook_sq) | rook_dst;

                    let blockers = self.occupied() ^ our_king ^ rook_sq;

                    if (must_be_empty & blockers).is_empty()
                        && (must_be_safe & self.attacked).is_empty()
                    {
                        abort_if!(visitor(PieceMoves::new(
                            MoveFlag::Castle,
                            Piece::King,
                            our_king,
                            rook_sq.bitboard(),
                        )));
                    }
                }
            }
        }

        Abort::No
    }

    fn gen_moves_impl<const IN_CHECK: bool, const WHITE: bool, V: FnMut(PieceMoves) -> Abort>(
        &self,
        visitor: &mut V,
    ) -> Abort {
        let targets = self.targets::<IN_CHECK, WHITE>();

        abort_if!(self.pawn_noisies::<WHITE, V>(visitor, targets));
        abort_if!(self.pawn_quiets::<WHITE, V>(visitor, targets));
        abort_if!(self.knight_moves::<V>(visitor, targets));
        abort_if!(self.orth_slider_moves::<V>(visitor, targets));
        abort_if!(self.diag_slider_moves::<V>(visitor, targets));
        abort_if!(self.king_moves::<IN_CHECK, V>(visitor));

        Abort::No
    }

    #[inline]
    pub fn gen_moves<V: FnMut(PieceMoves) -> Abort>(&self, mut visitor: V) -> Abort {
        match (self.stm, self.checkers.popcnt()) {
            (Color::White, 0) => self.gen_moves_impl::<false, true, V>(&mut visitor),
            (Color::Black, 0) => self.gen_moves_impl::<false, false, V>(&mut visitor),
            (Color::White, 1) => self.gen_moves_impl::<true, true, V>(&mut visitor),
            (Color::Black, 1) => self.gen_moves_impl::<true, false, V>(&mut visitor),
            _ => self.king_moves::<true, V>(&mut visitor),
        }
    }

    /// Recalculates the `checkers`, `pinned`, `xray`, and `attacked` bitboards.
    /// Should be called after making a move, and after toggling `self.stm`.
    #[inline]
    pub(crate) fn calc_threats(&mut self) {
        let our_king = self.king(self.stm);
        let blockers = self.occupied();
        let push_dir = self.stm.signum();
        let their_pawns = self.colored_pieces(Piece::Pawn, !self.stm);
        let their_orth = self.orth_sliders(!self.stm);
        let their_diag = self.diag_sliders(!self.stm);
        self.checkers = Bitboard::EMPTY;
        self.pinned = Bitboard::EMPTY;
        self.attacked =
            their_pawns.shift::<DownLeft>(push_dir) | their_pawns.shift::<DownRight>(push_dir);
        self.attacked |= king_moves(self.king(!self.stm));

        for knight in self.colored_pieces(Piece::Knight, !self.stm) {
            let moves = knight_moves(knight);
            if moves.contains(our_king) {
                self.checkers |= knight;
            }
            self.attacked |= moves;
        }

        for orth in their_orth {
            let moves = rook_moves(orth, blockers ^ our_king);
            if moves.contains(our_king) {
                self.checkers |= orth;
            }
            self.attacked |= moves;
        }

        for diag in their_diag {
            let moves = bishop_moves(diag, blockers ^ our_king);
            if moves.contains(our_king) {
                self.checkers |= diag;
            }
            self.attacked |= moves;
        }

        self.checkers |= pawn_attacks(our_king, self.stm) & their_pawns;

        // We're done calculating `self.attacked` and `self.checkers`.
        // Now we do `self.pinned`.
        for orth in rook_rays(our_king) & their_orth {
            let between = between(orth, our_king) & blockers;
            if between.popcnt() == 1 {
                self.pinned |= between;
            }
        }

        for diag in bishop_rays(our_king) & their_diag {
            let between = between(diag, our_king) & blockers;
            if between.popcnt() == 1 {
                self.pinned |= between;
            }
        }
    }

    /// Calcuates en-passant threats onto a nstm pawn that just double pushed on `file`.
    /// Should be called after making a move, and after toggling `self.stm`.
    #[inline]
    pub(crate) fn calc_ep_file(&mut self, file: File) {
        let victim = Square::new(file, Rank::R5.relative_to(self.stm));
        let attacker_dst = Square::new(file, Rank::R6.relative_to(self.stm));
        let our_pawns = self.colored_pieces(Piece::Pawn, self.stm);
        let our_king = self.king(self.stm);

        let attackers = our_pawns & pawn_attacks(attacker_dst, !self.stm);
        if attackers.is_empty() {
            return;
        }

        let (mut left, mut right) = (false, false);

        'attackers: for attacker in attackers {
            // For each potential attacker, we simulate the occupancy bitboard after the capture, and only allow
            // the capture if it doesn't put our king in check. Because the last move was a pawn double push, and
            // our king wasn't in check before that, the only possible attackers are the just pushed pawn (which would
            // be taken), and sliders discovering an attack through one of the moved pieces. Therefore, it's enough
            // to check whether an opposing slider can see the king on the updated occupancy bitboard.

            let blockers = self.occupied() ^ attacker ^ attacker_dst ^ victim;

            let orth = self.orth_sliders(!self.stm);
            let diag = self.diag_sliders(!self.stm);

            for orth in rook_rays(our_king) & orth {
                if (blockers & between(our_king, orth)).is_empty() {
                    continue 'attackers;
                }
            }

            for diag in bishop_rays(our_king) & diag {
                if (blockers & between(our_king, diag)).is_empty() {
                    continue 'attackers;
                }
            }

            // Now we know that `attacker` can take en passant. Update the corresponding flag.
            if attacker.file() < victim.file() {
                left = true
            } else {
                right = true;
            }
        }

        if left || right {
            self.set_en_passant(Some(EnPassantFile::new(file, left, right)));
        }
    }

    /// Makes the given move on the board. Does *not* check whether the move is legal. An illegal
    /// move may break the board, silently or loudly.
    pub fn make_move(&mut self, mov: Move) {
        let (from, to, flag, promotion) = (
            mov.from(),
            mov.to(),
            mov.flag(),
            mov.promotes_to_unchecked(),
        );
        debug_assert_ne!(from, to);

        self.halfmove_clock += 1;
        self.fullmove_count += u16::from(self.stm == Color::Black);

        self.set_en_passant(None);

        let piece = self.piece_on(from).expect("Move from empty square");
        let victim = self.piece_on(to);

        if piece == Piece::Pawn || (victim.is_some() && flag != MoveFlag::Castle) {
            self.halfmove_clock = 0;
        }

        if let Some(victim) = victim
            && flag != MoveFlag::Castle
        {
            // Toggle victim bitboard. The mailbox will be fixed once our piece moves onto the square.
            self.toggle_square(to, !self.stm, victim);

            // If we take an opponent's rook, we must update their castling rights.
            let their_back_rank = Rank::R8.relative_to(self.stm);
            if to.rank() == their_back_rank {
                for dir in [CastlingDirection::Long, CastlingDirection::Short] {
                    if Some(to.file()) == self.castling_rights[!self.stm].get(dir) {
                        self.set_castles(!self.stm, dir, None);
                    }
                }
            }
        }

        let mut double_push_file = None;

        match flag {
            MoveFlag::None => {
                self.toggle_square(from, self.stm, piece);
                self.toggle_square(to, self.stm, piece);

                self.mailbox[from] = None;
                self.mailbox[to] = Some(piece);

                // Update castling rights if needed.
                match piece {
                    Piece::King => {
                        self.set_castles(self.stm, CastlingDirection::Long, None);
                        self.set_castles(self.stm, CastlingDirection::Short, None);
                    }
                    Piece::Rook => {
                        let back_rank = Rank::R1.relative_to(self.stm);
                        if from.rank() == back_rank {
                            for dir in [CastlingDirection::Long, CastlingDirection::Short] {
                                if Some(from.file()) == self.castling_rights[self.stm].get(dir) {
                                    self.set_castles(self.stm, dir, None);
                                }
                            }
                        }
                    }
                    // Calculate the en passant squares if we double pushed.
                    Piece::Pawn if from.rank().idx() ^ to.rank().idx() == 2 => {
                        double_push_file = Some(from.file())
                    }

                    _ => {}
                }
            }
            MoveFlag::Castle => {
                let dir = if to.file() < from.file() {
                    CastlingDirection::Long
                } else {
                    CastlingDirection::Short
                };

                let rook_from = to;
                let king_to = Square::new(dir.king_dst(), from.rank());
                let rook_to = Square::new(dir.rook_dst(), from.rank());

                self.toggle_square(rook_from, self.stm, Piece::Rook);
                self.toggle_square(rook_to, self.stm, Piece::Rook);
                self.toggle_square(from, self.stm, Piece::King);
                self.toggle_square(king_to, self.stm, Piece::King);

                // Note that the order of mailbox updates here is important. Because the king sometimes lands on
                // the square that the rook started on, we need to first clear both entries and then set both.
                self.mailbox[rook_from] = None;
                self.mailbox[from] = None;
                self.mailbox[rook_to] = Some(Piece::Rook);
                self.mailbox[king_to] = Some(Piece::King);

                self.set_castles(self.stm, CastlingDirection::Long, None);
                self.set_castles(self.stm, CastlingDirection::Short, None);
            }
            MoveFlag::EnPassant => {
                let target_sq = Square::new(to.file(), from.rank());
                self.toggle_square(from, self.stm, Piece::Pawn);
                self.toggle_square(to, self.stm, Piece::Pawn);
                self.toggle_square(target_sq, !self.stm, Piece::Pawn);

                self.mailbox[from] = None;
                self.mailbox[to] = Some(Piece::Pawn);
                self.mailbox[target_sq] = None;
            }
            MoveFlag::Promotion => {
                self.toggle_square(from, self.stm, Piece::Pawn);
                self.toggle_square(to, self.stm, promotion);
                self.mailbox[from] = None;
                self.mailbox[to] = Some(promotion);
            }
        }

        self.stm = !self.stm;
        self.hash ^= ZOBRIST.black_to_move;
        if let Some(ep) = double_push_file {
            self.calc_ep_file(ep);
        }
        self.calc_threats();
    }
}
