//! A read that leaves the journal's directory as it found it (decision
//! 0115 ruling 3). [`Store::open_read_only`] refuses every write, but
//! SQLite still creates a WAL journal's `-wal` and `-shm` beside it on a
//! read-only open, and a read-only connection cannot remove them, so a
//! reader that ran where no writer was leaves both behind. A shell's
//! completer reads the journal on every keystroke, and must not.
//!
//! Where no `-wal` file stands beside the journal, every page is in the
//! main file and nothing is being written, so it is opened `immutable`:
//! no lock, no WAL and no `-shm` is asked for. Where one stands, a writer
//! holds the journal or left it, and both side files are read where they
//! are, `readonly_shm`, so no new one is created.

use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, OptionalExtension};

use crate::schema::{schema_supported, RECORDED_SCHEMA};
use crate::{damage, literal, patiently, Damage, Store, StoreError, READER_CACHE_KIB};

impl Store {
    /// Open an EXISTING journal for reading only, creating no file beside
    /// it, and spending at most `patience` on a peer's lock. Every refusal
    /// [`Store::open_read_only`] makes, this makes too.
    pub fn open_read_only_untouched(path: &Path, patience: Duration) -> Result<Store, StoreError> {
        Store::open_untouched(path, patience).map_err(|error| damage::classified(path, error))
    }

    fn open_untouched(path: &Path, patience: Duration) -> Result<Store, StoreError> {
        let bound = literal::journal(path)?;
        let mut wal = OsString::from(bound.as_os_str());
        wal.push("-wal");
        let query = match Path::new(&wal).exists() {
            true => "mode=ro&readonly_shm=1",
            false => "mode=ro&immutable=1",
        };
        let uri = format!("file:{}?{query}", encoded(bound.as_os_str()));
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
        let conn = Connection::open_with_flags(&uri, flags)
            .map_err(|error| literal::as_given(error, path, &bound))?;
        Store::reading(conn, path, patience)
    }

    /// A store reading `conn`, a read-only connection that opened `path`,
    /// which spends `patience` on a peer's lock: refused unless the
    /// journal records the schema this build reads. Both read-only opens
    /// end here.
    pub(crate) fn reading(
        conn: Connection,
        path: &Path,
        patience: Duration,
    ) -> Result<Store, StoreError> {
        conn.busy_timeout(patience)?;
        conn.pragma_update(None, "cache_size", -READER_CACHE_KIB)?;
        let found: Option<String> = patiently("open_read_only", patience, path, || {
            Ok(conn
                .query_row(RECORDED_SCHEMA, [], |row| row.get(0))
                .optional()?)
        })?;
        let found = found.ok_or_else(|| StoreError::Damaged {
            journal: path.into(),
            damage: Damage::NoSchema,
        })?;
        schema_supported(found.parse().unwrap_or(0))?;
        let mut store = Store::holding(conn, path);
        store.patience = patience;
        Ok(store)
    }
}

/// `path` as a URI's path: every byte but an unreserved one and `/`
/// percent-encoded, so a `?`, `#` or `%` in a directory's name is a
/// character of the path and never the start of a query SQLite would read.
fn encoded(path: &std::ffi::OsStr) -> String {
    path.as_encoded_bytes()
        .iter()
        .map(|&byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                char::from(byte).to_string()
            }
            byte => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests;
