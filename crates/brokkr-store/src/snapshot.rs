//! One read snapshot around many reads (decision 0089 ruling 8's boundary,
//! slice 4's units U5b2a and U5b2b1): a hearth's row read asks the store a
//! dozen statements and opens each lagging run whole, and a writer may move
//! a run from one lane to another between any two. Read under one
//! snapshot, the statements agree run for run, so the caller holds no set
//! of run ids to make them agree.
//!
//! [`Store::snapshot`] pins the snapshot with one open read statement, not
//! with a `BEGIN`, and hands back a [`ReadSnapshot`] that holds the pin
//! until it is dropped, so a reader can pull a window statement's rows
//! long after the snapshot opened and still read its state. A method of
//! this store that opens a read transaction of its own (`run_read`,
//! `seat_rows`, `seat_read`, the summary comparison) begins and ends it
//! inside the pin, and SQLite keeps the pin's read open when it ends:
//! another active statement of the connection holds the read transaction
//! past a nested `COMMIT` or `ROLLBACK`. One rule follows: a snapshot
//! taken while one is held, or inside [`Store::as_one_read`], joins the one
//! already held and reads its state.

use rusqlite::Statement;

use crate::{Store, StoreError};

/// The statement that pins the snapshot: one row whatever the journal
/// holds, read through the schema's own table, so the statement opens the
/// read transaction and stays active after its one row is stepped.
const PIN: &str = "SELECT count(*) FROM sqlite_schema";

/// A held read snapshot of one store: every statement its store makes while
/// it is held reads the journal as it stood when it was taken, whatever a
/// writer commits meanwhile. Dropping it releases the pin.
///
/// On a WAL journal an open read pins its snapshot and holds back the
/// checkpoint from the frames after it, so the log grows, and a checkpoint
/// waits behind the reader, for as long as the snapshot is held. The
/// holder names the length: the analytics readout holds one a rows-read
/// hearth from the moment it reads that hearth's lagging runs whole until
/// its last figure is folded, so every hearth's snapshot is held for the
/// readout's length, and released when the readout ends.
#[must_use = "the snapshot is released when the guard is dropped"]
pub struct ReadSnapshot<'s> {
    /// The pin, stepped to its row and never reset: an active statement
    /// holds the read transaction until it is finalized, which dropping it
    /// does.
    _pin: Statement<'s>,
}

impl Store {
    /// Pin the journal's state as it stands now, until the guard is
    /// dropped. A read-only open is pinned as a writable one is, and a
    /// snapshot taken while one is held joins it.
    pub fn snapshot(&self) -> Result<ReadSnapshot<'_>, StoreError> {
        let mut pin = self.conn.prepare(PIN)?;
        let mut rows = pin.query([])?;
        rows.next()?;
        // Dropping the cursor would reset the statement and end the read;
        // forgetting it leaves the statement stepped, so the read stands
        // until the statement itself is finalized. The cursor owns no heap.
        std::mem::forget(rows);
        Ok(ReadSnapshot { _pin: pin })
    }
}

#[cfg(test)]
mod tests;
