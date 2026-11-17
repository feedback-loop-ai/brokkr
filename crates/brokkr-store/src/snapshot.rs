//! One read snapshot around many reads (decision 0089 ruling 8's boundary,
//! slice 4's unit U5b2a): a hearth's row read asks the store a dozen
//! statements and opens each lagging run whole, and a writer may move a run
//! from one lane to another between any two. Read under one snapshot, the
//! statements agree run for run, so the caller holds no set of run ids to
//! make them agree.
//!
//! [`Store::read_snapshot`] pins the snapshot with one open read statement,
//! not with a `BEGIN`. A method of this store that opens a read
//! transaction of its own (`run_read`, `seat_rows`, `seat_read`, the
//! summary comparison) begins and ends it inside the pin, and SQLite keeps
//! the pin's read open when it ends: another active statement of the
//! connection holds the read transaction past a nested `COMMIT` or
//! `ROLLBACK`. One rule follows: a snapshot taken inside a snapshot, or
//! inside [`Store::as_one_read`], joins the one already held and reads its
//! state.

use crate::{Store, StoreError};

/// The statement that pins the snapshot: one row whatever the journal
/// holds, read through the schema's own table, so the statement opens the
/// read transaction and stays active after its one row is stepped.
const PIN: &str = "SELECT count(*) FROM sqlite_schema";

impl Store {
    /// Run `read`, every statement it makes reading the journal as it stood
    /// when the snapshot opened, whatever a writer commits meanwhile. The
    /// methods of this store that open a read transaction of their own
    /// read inside it too, and see the same state. `read`'s own refusal is
    /// its result, and a snapshot refuses nothing a read-only open reads.
    ///
    /// A snapshot inside a snapshot (or inside [`Store::as_one_read`])
    /// joins it: the outer's state is read, and the inner's end releases
    /// nothing.
    ///
    /// On a WAL journal an open read pins its snapshot and holds back the
    /// checkpoint from the frames after it, so the log grows for as long as
    /// the snapshot is held. The caller names the length: the analytics
    /// readout holds one for one hearth's whole row read, its window
    /// statements and the whole read of each run they leave, and releases
    /// it before the next hearth is read.
    pub fn read_snapshot<T, E: From<StoreError>>(
        &self,
        read: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        let mut pin = self.conn.prepare(PIN).map_err(StoreError::from)?;
        let mut rows = pin.query([]).map_err(StoreError::from)?;
        rows.next().map_err(StoreError::from)?;
        let read = read();
        // A read writes nothing, so its end is the same either way.
        drop(rows);
        read
    }
}

#[cfg(test)]
mod tests;
