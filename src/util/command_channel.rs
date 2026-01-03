//! One-capacity spmc broadcast channel, allowing the UCI thread to send a message
//! to all search threads. If a message has been sent but has not yet been handled by
//! all receivers, the next send operation will block until all receivers have handled
//! the previous message.

use std::sync::{Arc, Condvar, Mutex};

struct MsgState<M> {
    msg: M,
    handled: usize,
    generation: usize,
}

struct Shared<M> {
    msg: Mutex<Option<MsgState<M>>>,
    tx_condvar: Condvar,
    rx_condvar: Condvar,
    num_receivers: usize,
}

pub struct Sender<M> {
    shared: Arc<Shared<M>>,
}

#[derive(Clone)]
pub struct Receiver<M> {
    shared: Arc<Shared<M>>,
    generation: usize,
}

/// Creates a channel that expects exactly `num_receivers` receivers. Each message must therefore
/// be handled by exactly `num_receivers` clones of the returned receiver before the next message
/// can be sent. Calling `.recv()` on fewer receivers will block the next send indefinitely,
/// while calling it on more receivers may panic or lead to unexpected behavior.
pub fn channel<M>(num_receivers: usize) -> (Sender<M>, Receiver<M>) {
    let shared = Arc::new(Shared {
        msg: Mutex::new(None),
        tx_condvar: Condvar::new(),
        rx_condvar: Condvar::new(),
        num_receivers,
    });

    let tx = Sender {
        shared: shared.clone(),
    };
    let rx = Receiver {
        shared,
        generation: 1,
    };
    (tx, rx)
}

impl<M> Sender<M> {
    /// Sends a message to all receivers. Will block if a previously sent message has not yet been
    /// handled by all receivers.
    pub fn send(&mut self, m: M) {
        let shared = &*self.shared;
        let mut msg = shared.msg.lock().unwrap();

        while let Some(state) = &*msg
            && state.handled < shared.num_receivers
        {
            msg = shared.tx_condvar.wait(msg).unwrap();
        }

        let generation = msg.as_ref().map_or(1, |m| m.generation + 1);
        *msg = Some(MsgState {
            msg: m,
            handled: 0,
            generation,
        });

        shared.rx_condvar.notify_all();
        drop(msg);
    }
}

impl<M> Receiver<M> {
    /// Waits for a message from the sending thread, calls `handler` on it and returns the result.
    /// Note that a mutex will be held for the entire duration of the `handler` call.
    pub fn recv<R, F: FnOnce(&M) -> R>(&mut self, handler: F) -> R {
        let shared = &*self.shared;
        let mut msg = shared.msg.lock().unwrap();

        while msg
            .as_ref()
            .is_none_or(|msg| msg.generation < self.generation)
        {
            msg = shared.rx_condvar.wait(msg).unwrap();
        }

        let msg_inner = msg.as_mut().unwrap();
        assert!(
            msg_inner.handled < shared.num_receivers && msg_inner.generation == self.generation,
            "Used too many receivers"
        );
        let res = handler(&msg_inner.msg);

        msg_inner.handled += 1;
        if msg_inner.handled == shared.num_receivers {
            shared.tx_condvar.notify_one();
        }

        self.generation += 1;

        res
    }
}
