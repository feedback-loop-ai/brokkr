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

/// Both opens' refusal to open a relative journal names the path as it was
/// given, where SQLite's diagnostic names the absolute path it was handed:
/// the writer's parent is a file, so no directory is made, and the reader
/// finds no file. Nothing is created under the crate.
#[test]
fn a_journal_that_cannot_be_opened_is_named_as_given() {
    let given = Path::new("Cargo.toml/journal.db");
    for opened in [Store::open(given), Store::open_read_only(given)] {
        assert_eq!(
            opened.err().map(|error| error.to_string()),
            Some("sqlite: unable to open database file: Cargo.toml/journal.db".to_owned())
        );
    }
}

/// An open's error that carries no SQLite diagnostic passes as it is: a
/// path holding a nul byte is refused before SQLite sees it, at the nul in
/// the bound path.
#[test]
fn an_open_error_without_a_diagnostic_passes_as_it_is() {
    let before_nul = std::env::current_dir().unwrap().join("nul");
    match Store::open_read_only(Path::new("nul\0byte")).err() {
        Some(StoreError::Sqlite(rusqlite::Error::NulError(nul))) => {
            assert_eq!(nul.nul_position(), before_nul.as_os_str().len());
        }
        other => panic!("not refused as a nul byte: {other:?}"),
    }
}
