use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

pub const FLUSH_THRESHOLD: u64 = 1024;

/// A local counter, doing buffered writes to a backing atomic counter. To prevent excessive writes to the
/// backing atomic, we only write to it every `FLUSH_THRESHOLD` increments, or when `flush()` is called.
pub struct BufferedCounter {
    global: Arc<AtomicU64>,
    local: u64,
    buffer: u64,
}

impl BufferedCounter {
    pub fn new(count: Arc<AtomicU64>) -> Self {
        Self {
            global: count,
            local: 0,
            buffer: 0,
        }
    }

    pub fn flush(&mut self) {
        self.global.fetch_add(self.buffer, Ordering::Relaxed);
        self.buffer = 0;
    }

    pub fn inc(&mut self) {
        self.local += 1;
        self.buffer += 1;
        if self.buffer >= FLUSH_THRESHOLD {
            self.flush();
        }
    }

    /// Returns the exact number of times that `inc()` has been called on this `BufferedCounter`
    pub fn local(&self) -> u64 {
        self.local
    }

    /// Returns an estimate for the number of times that `inc()` has been called across all `BufferedCounter`s
    /// with this backing global counter. The estimate is a lower bound.
    pub fn global(&self) -> u64 {
        self.global.load(Ordering::Relaxed) + self.buffer
    }

    pub fn reset_local(&mut self) {
        self.local = 0;
        self.buffer = 0;
    }
}

impl Drop for BufferedCounter {
    fn drop(&mut self) {
        self.flush();
    }
}
