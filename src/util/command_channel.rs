//! One-capacity spmc broadcast channel, allowing the UCI thread to send a message
//! to all search threads. The sending thread blocks until all receiving threads have
//! handled a sent message.

use std::{
    cell::UnsafeCell,
    panic::RefUnwindSafe,
    sync::{
        Arc,
        atomic::{
            AtomicU32,
            Ordering::{Acquire, Relaxed, Release},
        },
    },
};

struct Shared<M> {
    msg: UnsafeCell<Option<M>>,
    futex: AtomicU32,
    num_receivers: u32,
}

unsafe impl<M: Sync> Sync for Shared<M> {}
impl<M> RefUnwindSafe for Shared<M> {}

pub struct Sender<M> {
    shared: Arc<Shared<M>>,
}

pub struct Receiver<M> {
    shared: Arc<Shared<M>>,
    generation: bool,
}

/// Creates a channel with exactly `num_receivers` receivers. The returned values are
/// the singular sender, and an iterator yielding the `num_receivers` receivers. Note that
/// sending a message will block the sender until all receivers have handled the message,
/// so dropping any of the receivers will lead to a deadlock in the sender thread.
pub fn channel<M>(num_receivers: u32) -> (Sender<M>, impl Iterator<Item = Receiver<M>>) {
    let shared = Arc::new(Shared {
        msg: UnsafeCell::new(None),
        futex: AtomicU32::new(0),
        num_receivers,
    });

    let tx = Sender {
        shared: shared.clone(),
    };
    let rx_iter = std::iter::repeat_n(shared, num_receivers as usize).map(|shared| Receiver {
        shared,
        generation: true,
    });

    (tx, rx_iter)
}

fn pack_futex(threads: u32, generation: bool) -> u32 {
    debug_assert!(threads < (u32::MAX >> 1));
    threads | (generation as u32) << 31
}

fn unpack_futex(futex: u32) -> (u32, bool) {
    let threads = futex & (u32::MAX >> 1);
    let generation = (futex >> 31) as u8;
    (threads, generation != 0)
}

impl<M> Sender<M> {
    /// Sends a message to all receivers. Will block until all receivers have handled the message.
    pub fn send(&mut self, m: M) {
        let shared = &*self.shared;
        let (threads, generation) = unpack_futex(shared.futex.load(Relaxed));
        // Because any previous `send` call waited until all receivers handled the message, there should be no outstanding receivers.
        debug_assert!(threads == 0);

        // SAFETY: Any previously sent message has been handled by all receivers, and they won't access `msg` again until we signal them to.
        unsafe { *shared.msg.get() = Some(m) };

        let next_gen = !generation;
        // After writing the message, we update the generation and wake the receivers. We use Release here, and Acquire in the receivers,
        // to make sure that writing the message happens-before the receivers read it.
        shared
            .futex
            .store(pack_futex(shared.num_receivers, next_gen), Release);
        atomic_wait::wake_all(&shared.futex);

        // Now we wait until the number of outstanding receivers reaches 0. The receivers all decrement using Release, and we load
        // using Acquire here, to ensure that any accesses of `msg` from receivers happen-before we return from `send`.
        let mut futex = shared.futex.load(Acquire);
        while unpack_futex(futex).0 != 0 {
            atomic_wait::wait(&shared.futex, futex);
            futex = shared.futex.load(Acquire);
        }
    }
}

impl<M> Receiver<M> {
    /// Waits for a message from the sending thread, calls `handler` on it and returns the result.
    pub fn recv<R, F: FnOnce(&M) -> R>(&mut self, handler: F) -> R {
        let shared = &*self.shared;

        // Wait until the message generation matches our local generation.
        let mut futex = shared.futex.load(Acquire);
        while unpack_futex(futex).1 != self.generation {
            atomic_wait::wait(&shared.futex, futex);
            futex = shared.futex.load(Acquire);
        }

        // SAFETY: The loop above, combined with the Release store in `send()` ensures that
        // the sender thread is done touching the message, so we can access it safely.
        let msg = unsafe { &*shared.msg.get() };
        let ret = handler(msg.as_ref().unwrap());

        self.generation = !self.generation;

        // We've handled the message, so we can decrement the outstanding receiver count.
        // If we're the last thread to do so, we wake the sender thread.
        if unpack_futex(shared.futex.fetch_sub(1, Release)).0 == 1 {
            atomic_wait::wake_all(&shared.futex);
        }

        ret
    }
}
