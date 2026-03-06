use std::{
    mem::transmute,
    sync::atomic::{AtomicU8, AtomicU64, Ordering::Relaxed},
};

use icarus_board::{board::Board, r#move::Move};

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
pub struct TTEntry {
    // Raw eval
    pub eval: Score,
    // Search result
    pub score: Score,
    pub mv: Option<Move>,
    pub depth: u8,
    pub flags: Flags,
}

const _: () = assert!(size_of::<TTEntry>() == 8);

#[repr(C, align(32))]
struct TTClusterMemory {
    data: [AtomicU64; 4],
}

#[derive(Clone, Copy)]
#[repr(C, align(32))]
struct TTCluster {
    entries: [TTEntry; 3],
    keys: u64,
}

impl TTClusterMemory {
    fn load(&self) -> TTCluster {
        let a = self.data[0].load(Relaxed);
        let b = self.data[1].load(Relaxed);
        let c = self.data[2].load(Relaxed);
        let d = self.data[3].load(Relaxed);

        unsafe { transmute([a, b, c, d]) }
    }

    fn store(&self, cluster: TTCluster) {
        let [a, b, c, d]: [u64; 4] = unsafe { transmute(cluster) };
        self.data[0].store(a, Relaxed);
        self.data[1].store(b, Relaxed);
        self.data[2].store(c, Relaxed);
        self.data[3].store(d, Relaxed);
    }

    fn clear(&self) {
        self.data[0].store(0, Relaxed);
        self.data[1].store(0, Relaxed);
        self.data[2].store(0, Relaxed);
        self.data[3].store(0, Relaxed);
    }

    fn empty() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl TTCluster {
    fn key_idx(&self, key: u16) -> Option<usize> {
        let low_bits = 0x0001000100010001;
        let high_bits = low_bits << 15;

        let splat = (key as u64) * low_bits;
        let diff = splat ^ self.keys;

        let i = (!diff & (diff.wrapping_sub(low_bits)) & high_bits).trailing_zeros() / 16;
        if i < 3 { Some(i as usize) } else { None }
    }

    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn keys(&self) -> [u16; 3] {
        [
            (self.keys >> (0 * 16)) as u16,
            (self.keys >> (1 * 16)) as u16,
            (self.keys >> (2 * 16)) as u16,
        ]
    }

    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn set_keys(&mut self, keys: [u16; 3]) {
        self.keys = ((keys[0] as u64) << (0 * 16))
            | ((keys[1] as u64) << (1 * 16))
            | ((keys[2] as u64) << (2 * 16));
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

pub struct TTable {
    entries: Box<[TTClusterMemory]>,
    age: AtomicU8,
}

impl TTable {
    pub fn new(mb: u64) -> TTable {
        let size = (mb * 1024 * 1024 / size_of::<TTCluster>() as u64) as usize;

        TTable {
            entries: (0..size).map(|_| TTClusterMemory::empty()).collect(),
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

    pub fn fetch(&self, hash: u64, ply: u16) -> Option<TTEntry> {
        let idx = self.index(hash);
        let hash = Self::trunc_key(hash);

        let cluster = self.entries[idx].load();
        if let Some(idx) = cluster.key_idx(hash) {
            let mut entry = cluster.entries[idx];
            entry.score = Self::tt_to_score(entry.score, ply);
            Some(entry)
        } else {
            None
        }
    }

    pub fn prefetch(&self, board: &Board) {
        #[cfg(target_feature = "sse")]
        {
            use std::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};
            unsafe {
                _mm_prefetch(
                    self.entries.as_ptr().add(self.index(board.hash())).cast(),
                    _MM_HINT_T0,
                );
            }
        }

        let _ = board;
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
        let index = self.index(hash);
        let hash = Self::trunc_key(hash);

        let mut cluster = self.entries[index].load();
        let mut keys = cluster.keys();

        let age = self.age.load(Relaxed);

        let mut cluster_idx = 0;
        let mut min_value = i32::MAX;
        let mut old = None;

        #[allow(clippy::needless_range_loop)]
        for i in 0..3 {
            let entry = cluster.entries[i];

            if keys[i] == hash {
                old = Some(entry);
            }

            if keys[i] == hash || entry.flags.tt_flag() == TTFlag::None {
                cluster_idx = i;
                break;
            }

            let relative_age = (32 + entry.flags.age() - age) & 31;
            let entry_value = entry.depth as i32 - 2 * relative_age as i32;

            if entry_value < min_value {
                cluster_idx = i;
                min_value = entry_value;
            }
        }

        if tt_flag == TTFlag::Exact
            || old.is_none_or(|old| depth + 4 > old.depth || age != old.flags.age())
        {
            keys[cluster_idx] = hash;
            cluster.set_keys(keys);
            cluster.entries[cluster_idx] = TTEntry {
                eval,
                score: Self::score_to_tt(score, ply),
                mv: mv.or(old.and_then(|e| e.mv)),
                depth,
                flags: Flags::new(age, pv, tt_flag),
            };

            self.entries[index].store(cluster);
        }
    }

    pub fn clear(&self) {
        self.entries.iter().for_each(|e| e.clear());
        self.age.store(0, Relaxed)
    }

    pub fn age(&self) {
        self.age
            .fetch_update(Relaxed, Relaxed, |age| Some((age + 1) % 32))
            .ok();
    }

    pub fn hashfull(&self) -> usize {
        self.entries[..2000]
            .iter()
            .flat_map(|e| e.load().entries)
            .filter(|e| e.flags.tt_flag() != TTFlag::None)
            .count()
            / 6
    }

    fn index(&self, hash: u64) -> usize {
        ((hash as u128 * self.entries.len() as u128) >> 64) as usize
    }

    fn trunc_key(key: u64) -> u16 {
        // We use the top bits for the index, so we want the bottom bits in the entry
        key as u16
    }
}
