//! A writable open's backfill walks, cut short by a peer's lock (decision
//! 0089 ruling 8): the one staging the event index's and the seat rows'
//! resumption tests share.

use std::path::Path;

use rusqlite::Connection;

use super::write_lock_on;
use crate::{Store, StoreError};

/// `walk` made on `store`, spending no patience, with a peer taking
/// `path`'s write lock as the walk reaches its second run, after its
/// first run's transaction: the peer, still holding the lock.
pub(crate) fn cut_short(
    store: &mut Store,
    path: &Path,
    walk: impl FnOnce(&mut Store, &mut dyn FnMut(&Connection)) -> Result<(), StoreError>,
) -> Connection {
    let mut peer = None;
    let mut reached = 0;
    let mut stage = |_: &Connection| {
        reached += 1;
        if reached == 2 {
            peer = Some(write_lock_on(path));
        }
    };
    store.quietly(|store| walk(store, &mut stage)).unwrap();
    peer.expect("the walk reached its second run")
}
