//! What a command writes beside a waiting entry (#430): a latch admission
//! found, or the launch the operator re-pinned it to. Each is journaled
//! as its command, and each is written only over what its caller read of
//! the entry, checked in the transaction that writes it: a latch is never
//! measured against a pin or a latch that no longer stands, and a re-pin
//! never clears a latch, or replaces a pin, its caller was not shown.

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use super::storage::{latch_row, queue_stored, STANDING_LATCH, STANDING_PIN};
use super::{record, standing, Attribution, EntryId, EntryState, QueueRefusal};
use crate::StoreError;

/// What a caller read of a waiting entry before it writes beside it: the
/// pin the entry stands for and the latch that stands on it, each by the
/// seq of the command that wrote it, `None` for none.
/// [`QueueEntry::seen`](super::QueueEntry::seen) reads it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Seen {
    pub pin: Option<i64>,
    pub latch: Option<i64>,
}

/// What a command writes beside a waiting entry, keyed by the command.
#[derive(Debug, Clone, Copy)]
pub(super) enum Beside {
    /// A realm-drift hold admission found.
    Latch,
    /// The launch the operator re-pinned the entry to.
    Pin,
}

impl Beside {
    fn word(self) -> &'static str {
        match self {
            Beside::Latch => "latch",
            Beside::Pin => "repin",
        }
    }

    fn insert(self) -> &'static str {
        match self {
            Beside::Latch => {
                "INSERT INTO queue_latches (seq, entry_id, finding) VALUES (?1, ?2, ?3)"
            }
            Beside::Pin => "INSERT INTO queue_pins (seq, entry_id, payload) VALUES (?1, ?2, ?3)",
        }
    }
}

/// Refuse a re-pin or a latch of an entry that has left the queue.
fn left_the_queue(entry: EntryId, state: EntryState) -> Option<QueueRefusal> {
    match state {
        EntryState::Dropped => Some(QueueRefusal::Dropped(entry)),
        EntryState::Claimed { run } => Some(QueueRefusal::Claimed { entry, run }),
        EntryState::Queued | EntryState::Held => None,
    }
}

/// Refuse the write unless the pin and the latch `seen` read are the ones
/// standing now: a latch measured before a peer re-pinned or latched, and
/// a re-pin over a latch or a pin its caller was not shown.
fn fenced(
    tx: &Transaction<'_>,
    entry: EntryId,
    beside: Beside,
    seen: Seen,
) -> Result<(), StoreError> {
    let latch = tx
        .query_row(STANDING_LATCH, [entry.0], latch_row)
        .optional()?;
    let pin: Option<i64> = tx.query_row(STANDING_PIN, [entry.0], |row| row.get(0))?;
    let latch_stands = latch.as_ref().map(|latch| latch.seq) == seen.latch;
    let refusal = match (beside, latch_stands, pin == seen.pin) {
        (Beside::Latch | Beside::Pin, true, true) => return Ok(()),
        (Beside::Latch, ..) => QueueRefusal::Unmeasured { entry },
        (Beside::Pin, false, _) => QueueRefusal::LatchMoved {
            entry,
            standing: latch,
        },
        (Beside::Pin, true, false) => QueueRefusal::PinMoved { entry },
    };
    Err(refusal.into())
}

pub(super) fn beside_once(
    conn: &mut Connection,
    entry: EntryId,
    beside: Beside,
    seen: Seen,
    text: &str,
    by: Attribution<'_>,
) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    queue_stored(&tx)?;
    if let Some(refusal) = left_the_queue(entry, standing(&tx, entry)?) {
        return Err(refusal.into());
    }
    fenced(&tx, entry, beside, seen)?;
    let seq = record(&tx, entry, beside.word(), None, by)?;
    tx.execute(beside.insert(), params![seq, entry.0, text])?;
    tx.commit()?;
    Ok(())
}
