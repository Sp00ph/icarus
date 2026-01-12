use std::{
    sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering::Relaxed},
    time::{Duration, Instant},
};

use icarus_common::{piece::Color, util::enum_map::enum_map};

use crate::{
    uci::SearchLimit,
    util::{MAX_PLY, atomic_instant::AtomicInstant, buffered_counter::BufferedCounter},
};

pub struct TimeManager {
    start: AtomicInstant,
    infinite: AtomicBool,
    stop: AtomicBool,

    // Supported limits for a `go` command. If these are not set by the command, the maximum values are used.
    max_depth: AtomicU16,
    max_nodes: AtomicU64,
    soft_time: AtomicU64,
    hard_time: AtomicU64,

    move_overhead: AtomicU16,
    // TODO: Implement soft nodes for datagen.
    // TODO: Implement deepening. Will require base time, prev move and stability, and perchance a dont_deepen flag.
}

pub const DEFAULT_MOVE_OVERHEAD: u16 = 20;

impl Default for TimeManager {
    fn default() -> Self {
        Self {
            start: AtomicInstant::now(),
            infinite: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            max_depth: AtomicU16::new(0),
            max_nodes: AtomicU64::new(0),
            soft_time: AtomicU64::new(0),
            hard_time: AtomicU64::new(0),
            move_overhead: AtomicU16::new(DEFAULT_MOVE_OVERHEAD),
        }
    }
}

impl TimeManager {
    pub fn init(&self, stm: Color, limits: &[SearchLimit]) {
        self.set_stop_flag(false);

        let mut time = enum_map! { _ => u64::MAX };
        let mut inc = enum_map! { _ => 0 };
        let mut movetime = u64::MAX;
        let mut max_depth = u16::MAX;
        let mut max_nodes = u64::MAX;
        let mut infinite = true;

        for limit in limits {
            use SearchLimit::*;

            match *limit {
                WhiteTime(t) => time[Color::White] = t,
                BlackTime(t) => time[Color::Black] = t,
                WhiteInc(t) => inc[Color::White] = t,
                BlackInc(t) => inc[Color::Black] = t,
                MoveTime(t) => movetime = t,
                Depth(d) => max_depth = d,
                Nodes(n) => max_nodes = n,

                SearchMoves(_) => {}
            }

            if matches!(
                limit,
                WhiteTime(..) | BlackTime(..) | MoveTime(..) | Depth(..) | Nodes(..)
            ) {
                infinite = false;
            }
        }

        self.infinite.store(infinite, Relaxed);

        self.max_depth.store(max_depth.min(MAX_PLY), Relaxed);
        self.max_nodes.store(max_nodes, Relaxed);

        let (time, inc) = (time[stm], inc[stm]);
        let move_overhead = self.move_overhead.load(Relaxed) as u64;

        let hard_time = (time / 2).min(time.saturating_sub(move_overhead));
        let soft_time = ((time / 64).saturating_sub(move_overhead) + inc).min(hard_time);

        self.soft_time.store(soft_time, Relaxed);
        self.hard_time.store(
            hard_time.min(movetime.saturating_sub(move_overhead)),
            Relaxed,
        );

        self.start.store(Instant::now(), Relaxed);
    }

    pub fn set_stop_flag(&self, stop: bool) {
        self.stop.store(stop, Relaxed);
    }

    pub fn set_move_overhead(&self, overhead: u16) {
        self.move_overhead.store(overhead, Relaxed);
    }

    pub fn stop_flag(&self) -> bool {
        self.stop.load(Relaxed)
    }

    pub fn infinite(&self) -> bool {
        self.infinite.load(Relaxed)
    }

    pub fn stop_search(&self, nodes: &BufferedCounter) -> bool {
        self.stop_flag()
            || nodes.global() >= self.max_nodes.load(Relaxed)
            || (nodes.local().is_multiple_of(1024)
                && self.elapsed().as_millis() as u64 > self.hard_time.load(Relaxed))
    }

    pub fn stop_id(&self, depth: u16, nodes: u64) -> bool {
        self.stop_flag()
            || depth >= self.max_depth.load(Relaxed)
            || nodes >= self.max_nodes.load(Relaxed)
            || self.elapsed().as_millis() as u64 > self.soft_time.load(Relaxed)
    }

    pub fn elapsed(&self) -> Duration {
        self.start.load(Relaxed).elapsed()
    }
}
