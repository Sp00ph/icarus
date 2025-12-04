use icarus_common::bitboard::Bitboard;

/// Incremental threats table, keeping track of all squares attacked by one side.
/// To make these incremental, we need to count the number of attackers per attacked square.
/// For this, we use four bitboards. The number of attackers on square `i` is then the
/// concatenation of the `i`-th bits of the four bitboards. `counters[0]` stores the LSBs.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct ThreatsTable {
    pub counters: [Bitboard; 4],
    pub aggregate: Bitboard,
}

impl ThreatsTable {
    /// Increments all counters whose corresponding bits are set in `inc`.
    #[inline]
    pub fn increment(&mut self, inc: Bitboard) {
        let mut carry = inc;
        for counter in &mut self.counters {
            let sum = *counter ^ carry;
            carry &= *counter;
            *counter = sum;
        }

        debug_assert!(carry == Bitboard::EMPTY, "Counter overflow!");
    }

    /// Decrements all counters whose corresponding bits are set in `inc`.
    #[inline]
    pub fn decrement(&mut self, dec: Bitboard) {
        let mut carry = dec;
        for counter in &mut self.counters {
            let sum = *counter ^ carry;
            carry &= !*counter;
            *counter = sum;
        }

        debug_assert!(carry == Bitboard::EMPTY, "Counter underflow!");
    }

    /// Updates the aggregate bitboard.
    #[inline]
    pub fn finish(&mut self) {
        self.aggregate = self.counters[0] | self.counters[1] | self.counters[2] | self.counters[3];
    }

    #[inline]
    pub fn get(&self) -> Bitboard {
        self.aggregate
    }
}
