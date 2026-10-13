//! The queue of what the stdout reader has read and the engine has not
//! yet taken (#433): bounded in items, as a `sync_channel` is, and in the
//! bytes its items weigh. The sender waits while the bytes queued plus
//! the next item's would pass the byte bound, unless nothing is queued,
//! so an item heavier than the bound still passes alone. Its bytes are
//! counted when it is queued and released when the receiver takes it.
//! Nothing is dropped: past either bound the sender waits, until the
//! receiver takes an item or is gone.

use std::sync::mpsc::{self, RecvError, RecvTimeoutError, SendError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// The bytes queued, and whether the receiver is gone.
#[derive(Default)]
struct Held {
    bytes: usize,
    closed: bool,
}

/// What the two ends share: the bytes held, and the signal that some
/// were released or the receiver went.
#[derive(Default)]
struct Shared {
    held: Mutex<Held>,
    released: Condvar,
}

impl Shared {
    fn held(&self) -> MutexGuard<'_, Held> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub(super) struct Sender<T> {
    tx: mpsc::SyncSender<(T, usize)>,
    shared: Arc<Shared>,
    bytes: usize,
}

pub(super) struct Receiver<T> {
    rx: mpsc::Receiver<(T, usize)>,
    shared: Arc<Shared>,
}

/// A queue of at most `items` items and `bytes` bytes, but for one item
/// heavier than `bytes`, which passes alone.
pub(super) fn bounded<T>(items: usize, bytes: usize) -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = mpsc::sync_channel(items);
    let shared = Arc::new(Shared::default());
    let sender = Sender {
        tx,
        shared: Arc::clone(&shared),
        bytes,
    };
    (sender, Receiver { rx, shared })
}

impl<T> Sender<T> {
    /// Queue `item`, which weighs `bytes`, waiting while it would take a
    /// queue that holds any bytes past the bound, and while the queue
    /// holds its count of items. The item comes back once the receiver
    /// is gone.
    pub(super) fn send(&self, item: T, bytes: usize) -> Result<(), SendError<T>> {
        let mut held = self.shared.held();
        loop {
            if held.closed {
                return Err(SendError(item));
            }
            if held.bytes == 0 || held.bytes.saturating_add(bytes) <= self.bytes {
                break;
            }
            held = self
                .shared
                .released
                .wait(held)
                .unwrap_or_else(PoisonError::into_inner);
        }
        held.bytes = held.bytes.saturating_add(bytes);
        drop(held);
        self.tx
            .send((item, bytes))
            .map_err(|SendError((item, _))| SendError(item))
    }
}

impl<T> Receiver<T> {
    pub(super) fn recv(&self) -> Result<T, RecvError> {
        self.rx.recv().map(|taken| self.release(taken))
    }

    pub(super) fn recv_timeout(&self, wait: Duration) -> Result<T, RecvTimeoutError> {
        self.rx.recv_timeout(wait).map(|taken| self.release(taken))
    }

    /// The item taken, its bytes released to the sender.
    fn release(&self, (item, bytes): (T, usize)) -> T {
        self.shared.held().bytes -= bytes;
        self.shared.released.notify_one();
        item
    }
}

/// A sender waiting on the byte bound is woken to find the receiver
/// gone, as one waiting on the item bound finds its channel closed.
impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.shared.held().closed = true;
        self.shared.released.notify_one();
    }
}
