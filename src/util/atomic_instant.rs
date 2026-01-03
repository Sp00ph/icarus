use std::{
    sync::{
        LazyLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

/// Wrapper type to atomically store and load `Instant`s. Internally, we store
/// in a `AtomicU64` the duration in nanoseconds since `EPOCH`. Note that this
/// breaks after running the program for ~584 years at a time.
pub struct AtomicInstant(AtomicU64);

/// The epoch used for `AtomicInstant`. Note that this MUST be initialized
/// to a value smaller than any `Instant`s ever stored in an `AtomicInstant`.
pub static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

fn instant_to_bits(instant: Instant) -> u64 {
    (instant - *EPOCH).as_nanos().try_into().unwrap()
}

fn bits_to_instant(bits: u64) -> Instant {
    *EPOCH + Duration::from_nanos(bits)
}

impl AtomicInstant {
    pub fn new(instant: Instant) -> Self {
        Self(AtomicU64::new(instant_to_bits(instant)))
    }

    pub fn now() -> Self {
        Self::new(Instant::now())
    }

    pub fn load(&self, order: Ordering) -> Instant {
        bits_to_instant(self.0.load(order))
    }

    pub fn store(&self, instant: Instant, order: Ordering) {
        self.0.store(instant_to_bits(instant), order);
    }
}
