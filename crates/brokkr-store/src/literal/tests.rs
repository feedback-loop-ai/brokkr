//! The one home binds a journal's path before SQLite reads it (#954).

use std::path::{Path, PathBuf};

use super::journal;
use crate::{Store, StoreError};

/// A relative name that SQLite would read as a URI is bound to the literal
/// file of that name in the working directory, and an absolute path is
/// left as it was given.
#[test]
fn a_uri_shaped_name_is_bound_to_the_literal_file_of_that_name() {
    let here = std::env::current_dir().unwrap();
    let named = Path::new("file:other.db?mode=rwc");
    assert_eq!(journal(named).unwrap(), here.join(named));
    assert_eq!(
        journal(&here.join("forge.db")).unwrap(),
        here.join("forge.db")
    );
}

/// An empty path names no file: both opens refuse it as SQLite's invalid
/// path before SQLite sees it, where SQLite would have opened a private
/// temporary database for the writer.
#[test]
fn an_empty_path_is_refused_by_both_opens() {
    for opened in [
        Store::open(Path::new("")),
        Store::open_read_only(Path::new("")),
    ] {
        match opened.err() {
            Some(StoreError::Sqlite(rusqlite::Error::InvalidPath(path))) => {
                assert_eq!(path, PathBuf::new());
            }
            other => panic!("not refused as an invalid path: {other:?}"),
        }
    }
}

/// SQLite's cannot-open diagnostic naming `path`, the one text this module
/// pins.
fn cannot_open(path: &Path) -> String {
    format!("unable to open database file: {}", path.display())
}

/// SQLite's diagnostic names the absolute path it was handed; it is shown
/// as the path the operator gave, and its code is kept.
#[test]
fn a_diagnostic_naming_the_bound_path_names_the_path_as_given() {
    let code = rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN);
    let bound = Path::new("/work/blocked/journal.db");
    let given = Path::new("blocked/journal.db");
    let shown = super::as_given(
        rusqlite::Error::SqliteFailure(code, Some(cannot_open(bound))),
        given,
        bound,
    );
    assert_eq!(
        shown,
        rusqlite::Error::SqliteFailure(code, Some(cannot_open(given)))
    );
}

/// An error that carries no SQLite diagnostic passes as it is, even when
/// it names the bound path.
#[test]
fn an_error_without_a_diagnostic_passes_as_it_is() {
    let bound = Path::new("/work/blocked/journal.db");
    let given = Path::new("blocked/journal.db");
    let shown = super::as_given(
        rusqlite::Error::InvalidPath(bound.to_path_buf()),
        given,
        bound,
    );
    assert_eq!(shown, rusqlite::Error::InvalidPath(bound.to_path_buf()));
}

/// Both opens refuse a journal they cannot open as SQLite's cannot-open,
/// naming the path: the writer's parent is a file, so no directory is
/// made, and the reader finds no file.
#[test]
fn a_journal_that_cannot_be_opened_is_refused_as_cannot_open() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("blocked"), "").unwrap();
    let given = dir.path().join("blocked/journal.db");
    for opened in [Store::open(&given), Store::open_read_only(&given)] {
        match opened.err() {
            Some(StoreError::Sqlite(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: rusqlite::ErrorCode::CannotOpen,
                    ..
                },
                Some(message),
            ))) => assert_eq!(message, cannot_open(&given)),
            other => panic!("not refused as cannot-open: {other:?}"),
        }
    }
}

/// A path holding a nul byte is refused before it is bound or SQLite sees
/// it, at the nul in the path as given.
#[test]
fn a_nul_byte_is_refused_at_its_place_in_the_path_as_given() {
    for opened in [
        Store::open(Path::new("nul\0byte")),
        Store::open_read_only(Path::new("nul\0byte")),
    ] {
        match opened.err() {
            Some(StoreError::Sqlite(rusqlite::Error::NulError(nul))) => {
                assert_eq!(nul.nul_position(), 3);
            }
            other => panic!("not refused as a nul byte: {other:?}"),
        }
    }
}
