//! A read that leaves the journal's directory as it found it (decision
//! 0119 ruling 3). [`Store::open_read_only`] refuses every write, but
//! SQLite still creates a WAL journal's `-wal` and `-shm` beside it on a
//! read-only open, and a read-only connection cannot remove them, so a
//! reader that ran where no writer was leaves both behind. A shell's
//! completer reads the journal on every keystroke, and must not.
//!
//! The journal is first resolved as SQLite resolves it, every symbolic
//! link followed, because SQLite names the side files after the file it
//! opens: a `--db` that is a link has its `-wal` beside the link's
//! target, never beside the link. Then what stands there decides the
//! read:
//!
//! - A `-journal`: a rollback-mode writer holds the journal, or left it
//!   for the next writer to roll back. A reader that may not roll it back
//!   reads nothing (ruling 7: a locked journal completes nothing).
//! - A `-wal`: a writer holds the journal or left it, and both side files
//!   are read where they are, `readonly_shm`, so no new one is created.
//! - Neither, and the header names a WAL journal: every page is in the
//!   main file and nothing is being written, so it is opened `immutable`,
//!   with no lock, no WAL and no `-shm`. Nothing guards that read against
//!   a writer starting during it, so once the read is done the side files
//!   and the main file are looked at again, and a read across any change
//!   answers nothing.
//! - Neither, and any other header (a rollback-mode journal, or a file
//!   SQLite will refuse): opened read-only under SQLite's own locks, which
//!   create no file in rollback mode, so a peer's lock refuses the read
//!   once the patience is spent.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rusqlite::{Connection, OpenFlags, OptionalExtension};

use crate::schema::{schema_supported, RECORDED_SCHEMA};
use crate::{damage, literal, patiently, Damage, Store, StoreError, READER_CACHE_KIB};

impl Store {
    /// Run `read` on an EXISTING journal opened for reading only,
    /// creating no file beside it and spending at most `patience` on a
    /// peer's lock. Every refusal [`Store::open_read_only`] makes, this
    /// makes too; it also refuses a journal a rollback journal stands
    /// beside, and the answer of a lock-free read the journal changed
    /// under. The store lives only for `read`, so no read can outlast
    /// that last look.
    pub fn read_untouched<T, E: From<StoreError>>(
        path: &Path,
        patience: Duration,
        read: impl FnOnce(&Store) -> Result<T, E>,
    ) -> Result<T, E> {
        let (store, unlocked) = Store::open_untouched(path, patience)
            .map_err(|error| damage::classified(path, error))?;
        let answer = read(&store)?;
        drop(store);
        match unlocked {
            Some(before) if Sight::of(&before.real) != before => {
                let why = "it changed while it was read without a lock";
                Err(StoreError::from(unsettled(path, why)).into())
            }
            _ => Ok(answer),
        }
    }

    /// The store, and what stood when it was opened `immutable`; `None`
    /// where SQLite's locks or its WAL guard the read instead.
    fn open_untouched(
        path: &Path,
        patience: Duration,
    ) -> Result<(Store, Option<Sight>), StoreError> {
        let bound = literal::journal(path)?;
        // A missing file has nothing to follow: SQLite refuses it below.
        let real = std::fs::canonicalize(&bound).unwrap_or(bound);
        let before = Sight::of(&real);
        if before.rollback {
            return Err(unsettled(path, "a rollback journal stands beside it").into());
        }
        let (query, unlocked) = match (before.wal, wal_header(&real)) {
            (true, _) => ("mode=ro&readonly_shm=1", None),
            (false, true) => ("mode=ro&immutable=1", Some(before)),
            (false, false) => ("mode=ro", None),
        };
        let uri = format!("file:{}?{query}", encoded(real.as_os_str()));
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
        let conn = Connection::open_with_flags(&uri, flags)
            .map_err(|error| literal::as_given(error, path, &real))?;
        Ok((Store::reading(conn, path, patience)?, unlocked))
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

/// What a lock-free read looks at again once it is done: the side files
/// a writer makes, and the main file's identity, its size, modification
/// time and, on unix, its inode.
#[derive(Debug, PartialEq, Eq)]
struct Sight {
    real: PathBuf,
    wal: bool,
    rollback: bool,
    main: Option<(u64, Option<SystemTime>, u64)>,
}

impl Sight {
    fn of(real: &Path) -> Sight {
        let beside = |suffix: &str| {
            let mut side = OsString::from(real.as_os_str());
            side.push(suffix);
            Path::new(&side).exists()
        };
        let main = std::fs::metadata(real)
            .ok()
            .map(|meta| (meta.len(), meta.modified().ok(), inode(&meta)));
        Sight {
            real: real.to_path_buf(),
            wal: beside("-wal"),
            rollback: beside("-journal"),
            main,
        }
    }
}

#[cfg(unix)]
fn inode(meta: &std::fs::Metadata) -> u64 {
    std::os::unix::fs::MetadataExt::ino(meta)
}

#[cfg(not(unix))]
fn inode(_: &std::fs::Metadata) -> u64 {
    0
}

/// Does the file's header name a WAL journal? Bytes 18 and 19 of an
/// SQLite header are its write and read versions, both 2 in WAL mode.
/// A header that cannot be read names none.
fn wal_header(real: &Path) -> bool {
    let mut header = [0u8; 20];
    let read = std::fs::File::open(real).and_then(|mut file| file.read_exact(&mut header));
    read.is_ok() && header.starts_with(b"SQLite format 3\0") && header[18..20] == [2, 2]
}

/// The refusal of a journal that cannot be read untouched now: SQLite's
/// own busy, since a writer holds it, saying why.
fn unsettled(path: &Path, why: &str) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
        Some(format!(
            "the journal '{}' is not read: {why}",
            path.display().to_string().escape_debug()
        )),
    )
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
