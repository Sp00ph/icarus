use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use crate::tagged_cell;

/// Wrapper type to atomically store and load `Instant`s. Internally, we store
/// in a `AtomicU64` the duration in nanoseconds since `EPOCH`. Note that this
/// breaks after running the program for ~584 years at a time.
pub struct AtomicInstant {
    value: AtomicU64,
    tag: EpochTag,
}

tagged_cell!(
    /// The epoch used for `AtomicInstant`.
    static EPOCH: TaggedCell<Instant, pub EpochTag> = TaggedCell::new();
);

pub fn init() -> EpochTag {
    EPOCH.init(Instant::now)
}

fn instant_to_bits(instant: Instant, tag: EpochTag) -> u64 {
    (instant - *EPOCH.get(tag)).as_nanos().try_into().unwrap()
}

fn bits_to_instant(bits: u64, tag: EpochTag) -> Instant {
    *EPOCH.get(tag) + Duration::from_nanos(bits)
}

impl AtomicInstant {
    fn new(instant: Instant, tag: EpochTag) -> Self {
        Self {
            value: AtomicU64::new(instant_to_bits(instant, tag)),
            tag,
        }
    }

    pub fn now(tag: EpochTag) -> Self {
        Self::new(Instant::now(), tag)
    }

    pub fn load(&self, order: Ordering) -> Instant {
        bits_to_instant(self.value.load(order), self.tag)
    }

    pub fn store(&self, instant: Instant, order: Ordering) {
        self.value.store(instant_to_bits(instant, self.tag), order);
    }
}
