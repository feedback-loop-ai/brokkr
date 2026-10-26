//! A seat row's digest (#753, the operator's 2026-10-24 ruling): the one
//! encoding every writer seals a row with and every reader checks it by,
//! a value at a time.

use brokkr_core::seat::row::Datum;
use rusqlite::types::{Value, ValueRef};
use sha2::{Digest, Sha256};

pub(super) fn value(datum: Datum) -> Value {
    match datum {
        Datum::Null => Value::Null,
        Datum::Integer(value) => Value::Integer(value),
        Datum::Text(value) => Value::Text(value),
    }
}

/// A value a row holds, borrowed where it stands, a datum's or SQLite's.
#[derive(Clone, Copy)]
pub(super) enum Cell<'a> {
    Null,
    Integer(i64),
    Text(&'a str),
}

impl<'a> Cell<'a> {
    /// The value a column holds, read as its bytes: `None` for a real, a
    /// blob or a text that is not UTF-8, which no row writes.
    pub(super) fn of(value: ValueRef<'a>) -> Option<Cell<'a>> {
        match value {
            ValueRef::Null => Some(Cell::Null),
            ValueRef::Integer(value) => Some(Cell::Integer(value)),
            ValueRef::Text(text) => std::str::from_utf8(text).ok().map(Cell::Text),
            ValueRef::Real(_) | ValueRef::Blob(_) => None,
        }
    }

    pub(super) fn held(datum: &'a Datum) -> Cell<'a> {
        match datum {
            Datum::Null => Cell::Null,
            Datum::Integer(value) => Cell::Integer(*value),
            Datum::Text(value) => Cell::Text(value),
        }
    }

    /// The row's datum this value is, copied.
    pub(super) fn datum(self) -> Datum {
        match self {
            Cell::Null => Datum::Null,
            Cell::Integer(value) => Datum::Integer(value),
            Cell::Text(value) => Datum::Text(value.to_string()),
        }
    }
}

/// The digest of a row as its writer writes it: a SHA-256 over its run,
/// its place and each of its values in [`SeatRow::COLUMNS`]' order, each
/// tagged by its type, an integer by its eight bytes and a text by its
/// length and then its bytes, so no two rows encode alike. The one home
/// of the encoding ([`Seal`]): every writer writes the digest [`written`](super::written)
/// gives, and [`sealed_at`](super::sealed_at) recomputes it from the values it reads, a
/// value at a time.
pub(super) fn digest(run_id: &str, opened: i64, data: &[Datum]) -> String {
    let mut seal = Seal::of(run_id, opened);
    for datum in data {
        seal.put(Cell::held(datum));
    }
    seal.hex()
}

/// A row's digest ([`digest`]) as its values are fed to it, so a reader
/// checks a row without holding the values it does not keep.
pub(super) struct Seal(Sha256);

impl Seal {
    /// The digest of the row at the place `opened` of `run_id`'s rows,
    /// before its values.
    pub(super) fn of(run_id: &str, opened: i64) -> Seal {
        let mut seal = Seal(Sha256::new());
        seal.put(Cell::Text(run_id));
        seal.put(Cell::Integer(opened));
        seal
    }

    pub(super) fn put(&mut self, cell: Cell<'_>) {
        match cell {
            Cell::Null => self.0.update(b"N"),
            Cell::Integer(value) => {
                self.0.update(b"I");
                self.0.update(value.to_be_bytes());
            }
            Cell::Text(value) => {
                self.0.update(b"T");
                self.0.update((value.len() as u64).to_be_bytes());
                self.0.update(value.as_bytes());
            }
        }
    }

    /// The digest, written as every digest of this realm is.
    pub(super) fn hex(self) -> String {
        hex::encode(self.0.finalize())
    }
}
