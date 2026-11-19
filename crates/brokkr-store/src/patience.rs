//! Patience (#346): the one place a `SQLITE_BUSY` is waited out, and so the
//! one place every store operation's error passes on its way out, where a
//! damaged journal is named (#1000).

use std::path::Path;

use crate::damage::classified;
use crate::StoreError;

fn is_busy(error: &rusqlite::Error) -> bool {
    matches!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
    )
}

/// How long [`patiently`] waits between retries. Short enough that the
/// case it exists for — a `SQLITE_BUSY` returned in microseconds,
/// without the busy handler ever being asked — does not turn a
/// millisecond of contention into a visible stall; long enough that
/// losing a whole patience to a genuinely held lock costs a bounded
/// number of wakeups rather than a spin.
const CONTENTION_PAUSE: std::time::Duration = std::time::Duration::from_millis(5);

/// The ONE place a `SQLITE_BUSY` is handled, and the only one. Every
/// store operation an engine can reach runs inside it; no call site
/// anywhere else in the workspace retries anything.
///
/// The connection's busy handler is the first line and covers almost
/// everything, but it is not a promise SQLite always keeps. Its own
/// documentation says so: *"If SQLite determines that invoking the busy
/// handler could result in a deadlock, it will go ahead and return
/// SQLITE_BUSY to the application instead of invoking the busy
/// handler."* The btree layer's retry loop is guarded by
/// `pBt->inTransaction==TRANS_NONE`, so a write that follows a read
/// still open on the same connection is refused the handler outright and
/// comes back busy in **microseconds** — measured here at 12µs against a
/// peer's held write lock, versus the 30.03s the same contention costs a
/// connection the handler does cover. That is the shape a bounded
/// timeout cannot see: the budget was never spent, so nothing waited,
/// and the caller met `database is locked` with 29.99 seconds of
/// patience left in its pocket.
///
/// So patience is spent to a DEADLINE rather than counted in attempts:
/// whichever line of defence does the waiting, the total is one
/// [`crate::Store::set_patience`] budget and never two. A handler that
/// spends the whole budget lands on the deadline and reports
/// [`StoreError::Contended`] at once; a busy the handler declined
/// returns instantly and this loop spends the rest.
///
/// What it retries is exactly one thing: a raw `SQLITE_BUSY` /
/// `SQLITE_LOCKED`. Every other error — including
/// [`StoreError::HeadMoved`] and [`StoreError::AppendConflict`], which
/// are refusals and not accidents — returns on the first attempt, and
/// only through [`classified`], which names `journal` damaged where the
/// error says it is and leaves every other error untouched. `attempt`
/// must therefore write nothing outside its own transaction, which is
/// what makes re-running it the same call rather than a second one.
pub(crate) fn patiently<T>(
    operation: &'static str,
    patience: std::time::Duration,
    journal: &Path,
    mut attempt: impl FnMut() -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    let started = std::time::Instant::now();
    let deadline = started + patience;
    loop {
        match attempt() {
            Err(StoreError::Sqlite(error)) if is_busy(&error) => {
                if std::time::Instant::now() >= deadline {
                    return Err(StoreError::Contended {
                        operation,
                        waited_ms: started.elapsed().as_millis(),
                    });
                }
                std::thread::sleep(CONTENTION_PAUSE);
            }
            settled => return settled.map_err(|error| classified(journal, error)),
        }
    }
}
