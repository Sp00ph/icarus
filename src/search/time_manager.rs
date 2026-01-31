use std::{
    sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU64, Ordering::Relaxed},
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
    stop: AtomicU32,

    // Supported limits for a `go` command. If these are not set by the command, the maximum values are used.
    max_depth: AtomicU16,
    soft_nodes: AtomicU64,
    hard_nodes: AtomicU64,
    base_time: AtomicU64,
    soft_time: AtomicU64,
    hard_time: AtomicU64,

    move_overhead: AtomicU16,
    // TODO: Implement soft nodes for datagen.
}

pub const DEFAULT_MOVE_OVERHEAD: u16 = 20;

impl Default for TimeManager {
    fn default() -> Self {
        Self {
            start: AtomicInstant::now(),
            infinite: AtomicBool::new(false),
            stop: AtomicU32::new(0),
            max_depth: AtomicU16::new(0),
            soft_nodes: AtomicU64::new(0),
            hard_nodes: AtomicU64::new(0),
            base_time: AtomicU64::new(0),
            soft_time: AtomicU64::new(0),
            hard_time: AtomicU64::new(0),
            move_overhead: AtomicU16::new(DEFAULT_MOVE_OVERHEAD),
        }
    }
}

impl TimeManager {
    pub fn init(&self, stm: Color, limits: &[SearchLimit], use_soft_nodes: bool) {
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
        self.soft_nodes.store(max_nodes, Relaxed);
        let hard_nodes = max_nodes.saturating_mul(if use_soft_nodes { 200 } else { 1 });
        self.hard_nodes.store(hard_nodes, Relaxed);

        let (time, inc) = (time[stm], inc[stm]);
        let move_overhead = self.move_overhead.load(Relaxed) as u64;

        let hard_time = (time / 2).min(time.saturating_sub(move_overhead));
        let soft_time = ((time / 64).saturating_sub(move_overhead) + inc).min(hard_time);

        self.soft_time.store(soft_time, Relaxed);
        self.base_time.store(soft_time, Relaxed);
        self.hard_time.store(
            hard_time.min(movetime.saturating_sub(move_overhead)),
            Relaxed,
        );

        self.start.store(Instant::now(), Relaxed);
    }

    pub fn set_stop_flag(&self, stop: bool) {
        self.stop.store(stop as u32, Relaxed);
        if self.infinite() {
            atomic_wait::wake_all(&self.stop);
        }
    }

    pub fn set_move_overhead(&self, overhead: u16) {
        self.move_overhead.store(overhead, Relaxed);
    }

    pub fn stop_flag(&self) -> bool {
        self.stop.load(Relaxed) != 0
    }

    pub fn infinite(&self) -> bool {
        self.infinite.load(Relaxed)
    }

    pub fn stop_search(&self, nodes: &BufferedCounter) -> bool {
        self.stop_flag()
            || nodes.global() >= self.hard_nodes.load(Relaxed)
            || (nodes.local().is_multiple_of(1024)
                && self.elapsed().as_millis() as u64 > self.hard_time.load(Relaxed))
    }

    pub fn stop_id(&self, depth: u16, nodes: u64) -> bool {
        self.stop_flag()
            || depth >= self.max_depth.load(Relaxed)
            || nodes >= self.soft_nodes.load(Relaxed)
            || self.elapsed().as_millis() as u64 > self.soft_time.load(Relaxed)
    }

    pub fn elapsed(&self) -> Duration {
        self.start.load(Relaxed).elapsed()
    }

    pub fn wait_for_stop(&self) {
        while !self.stop_flag() {
            atomic_wait::wait(&self.stop, 0);
        }
    }

    pub fn deepen(&self, depth: u16, total_nodes: u64, best_move_nodes: u64, move_stability: u16) {
        if depth < 4 {
            return;
        }

        let ratio = (best_move_nodes as f64) / (total_nodes.max(1) as f64);

        let node_tm_factor = 2.5 - 1.5 * ratio;
        let move_stability_factor = (1.8 - 0.1 * (move_stability as f64)).max(0.9);

        let new_target =
            ((self.base_time.load(Relaxed) as f64 * node_tm_factor * move_stability_factor) as u64)
                .min(self.hard_time.load(Relaxed));
        self.soft_time.store(new_target, Relaxed);
    }
}
