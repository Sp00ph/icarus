use std::sync::atomic::{AtomicU8, AtomicU64, Ordering::Relaxed};

use icarus_board::r#move::Move;

use crate::score::Score;

pub const DEFAULT_TT_SIZE: u64 = 16;
pub const MAX_TT_SIZE: u64 = 1048576;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TTFlag {
    None = 0,
    Exact = 1,
    Lower = 2,
    Upper = 3,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct TTData {
    // Raw eval
    pub eval: Score,
    // Search result
    pub score: Score,
    pub mv: Option<Move>,
    pub depth: u8,
    pub flags: Flags,
}

impl TTData {
    fn pack(self) -> u64 {
        unsafe { std::mem::transmute(self) }
    }

    fn unpack(n: u64) -> Self {
        unsafe { std::mem::transmute(n) }
    }
}

#[derive(Clone, Copy)]
pub struct Flags(u8);

impl Flags {
    pub fn new(age: u8, pv: bool, tt_flag: TTFlag) -> Self {
        Self((age << 3) | ((pv as u8) << 2) | tt_flag as u8)
    }

    pub fn age(self) -> u8 {
        self.0 >> 3
    }

    pub fn pv(self) -> bool {
        (self.0 >> 2) & 1 != 0
    }

    pub fn tt_flag(self) -> TTFlag {
        unsafe { std::mem::transmute(self.0 & 3) }
    }
}

struct TTEntry {
    key: AtomicU64,
    data: AtomicU64,
}

impl TTEntry {
    pub fn empty() -> Self {
        Self {
            key: AtomicU64::new(0),
            data: AtomicU64::new(0),
        }
    }

    pub fn store(&self, hash: u64, data: TTData) {
        let data = data.pack();
        self.key.store(hash ^ data, Relaxed);
        self.data.store(data, Relaxed);
    }

    pub fn reset(&self) {
        self.key.store(0, Relaxed);
        self.data.store(0, Relaxed);
    }

    pub fn key_and_data(&self) -> (u64, TTData) {
        let key = self.key.load(Relaxed);
        let data = self.data.load(Relaxed);
        (key ^ data, TTData::unpack(data))
    }
}

pub struct TTable {
    entries: Box<[TTEntry]>,
    age: AtomicU8,
}

impl TTable {
    pub fn new(mb: u64) -> TTable {
        let size = (mb * 1024 * 1024 / size_of::<TTEntry>() as u64) as usize;

        TTable {
            entries: (0..size).map(|_| TTEntry::empty()).collect(),
            age: AtomicU8::new(0),
        }
    }

    fn score_to_tt(s: Score, ply: u16) -> Score {
        if !s.is_mate() {
            s
        } else if s < Score::ZERO {
            s - ply as i16
        } else {
            s + ply as i16
        }
    }

    fn tt_to_score(s: Score, ply: u16) -> Score {
        if !s.is_mate() {
            s
        } else if s < Score::ZERO {
            s + ply as i16
        } else {
            s - ply as i16
        }
    }

    pub fn fetch(&self, hash: u64, ply: u16) -> Option<TTData> {
        let idx = self.index(hash);
        let (key, mut data) = self.entries[idx].key_and_data();

        if key != hash || data.flags.tt_flag() == TTFlag::None {
            None
        } else {
            data.score = Self::tt_to_score(data.score, ply);
            Some(data)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn store(
        &self,
        hash: u64,
        depth: u8,
        ply: u16,
        eval: Score,
        score: Score,
        mv: Option<Move>,
        tt_flag: TTFlag,
        pv: bool,
    ) {
        let old = self.fetch(hash, ply);
        let new = TTData {
            depth,
            eval,
            score: Self::score_to_tt(score, ply),
            mv: mv.or(old.and_then(|d| d.mv)),
            flags: Flags::new(self.age.load(Relaxed), pv, tt_flag),
        };

        self.entries[self.index(hash)].store(hash, new);
    }

    pub fn clear(&self) {
        self.entries.iter().for_each(|e| e.reset());
        self.age.store(0, Relaxed)
    }

    pub fn age(&self) {
        self.age
            .fetch_update(Relaxed, Relaxed, |age| Some((age + 1) % 32))
            .ok();
    }

    fn index(&self, hash: u64) -> usize {
        ((hash as u128 * self.entries.len() as u128) >> 64) as usize
    }
}
