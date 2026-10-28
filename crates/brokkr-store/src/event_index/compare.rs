//! Ruling 4's comparison of a run's index rows with its events: the
//! expected columns of one row at a time, the first seq where a row is
//! not its event's, and the park facts left for the refold that writes
//! them.

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::event_index::Indexed;
use brokkr_core::summary::Park;
use rusqlite::types::{Value, ValueRef};

use super::{park_facts, IndexFault, PARK_COLUMNS};
use crate::StoreError;

/// The columns `event` projects to after its key, by name: the event's
/// own, and the dated facts a park's row keeps, which the `run/parked`
/// event itself does not state, so `park` is the whole run's reading of
/// it and `head` the seq that reading covers. A `run/parked` event core
/// read no park for is one whose stamp did not parse: its row keeps no
/// dated facts, but it is still processed at `head`, so its `facts_seq`
/// is the head rather than NULL.
pub(super) fn projects(
    event: &EventEnvelope,
    park: Option<&Park>,
    head: u64,
) -> Vec<(&'static str, Value)> {
    let indexed = Indexed::of(event);
    let text = |text: Option<&str>| text.map_or(Value::Null, |text| Value::Text(text.into()));
    let mut columns = vec![
        ("type", text(Some(indexed.event_type.as_str()))),
        ("attempt_id", text(indexed.attempt_id)),
        ("member", text(indexed.member)),
        ("recorded_at", text(Some(indexed.recorded_at))),
        ("event_id", text(Some(&event.event_id))),
    ];
    columns.extend(PARK_COLUMNS.iter().copied().zip(match park {
        Some(park) => park_facts(Some(park), head),
        None if event.event_type == EventType::RunParked => park_facts(None, head),
        None => std::array::from_fn(|_| Value::Null),
    }));
    columns
}

/// The first key where `rows`, each its key and then its columns, part
/// from `expected`, each a key and the columns its row should hold, both
/// in key order, and how: the rows streamed beside what is expected, so a
/// comparison holds one row at a time.
pub(crate) fn first_parted(
    rows: rusqlite::Rows<'_>,
    expected: impl Iterator<Item = (u64, Vec<(&'static str, Value)>)>,
) -> Result<Option<(u64, IndexFault)>, StoreError> {
    first_parted_at(rows, expected, &mut false)
}

/// [`first_parted`], also marking in `stale` a row whose park facts stand
/// behind its head: its dated columns are not compared, since the refold
/// that writes them is what a later open does.
pub(super) fn first_parted_at(
    mut rows: rusqlite::Rows<'_>,
    mut expected: impl Iterator<Item = (u64, Vec<(&'static str, Value)>)>,
    stale: &mut bool,
) -> Result<Option<(u64, IndexFault)>, StoreError> {
    loop {
        let found = match (expected.next(), rows.next()?) {
            (None, None) => return Ok(None),
            (Some((key, _)), None) => Some((key, IndexFault::Missing)),
            (None, Some(row)) => Some((row.get::<_, i64>(0)? as u64, IndexFault::Stray)),
            (Some((key, columns)), Some(row)) => parted(key, &columns, row, stale)?,
        };
        if found.is_some() {
            return Ok(found);
        }
    }
}

/// The dated columns a park's row keeps, [`PARK_COLUMNS`], which a stale
/// stamp leaves for the refold.
fn dated(column: &str) -> bool {
    PARK_COLUMNS.contains(&column)
}

/// Where `row` parts from the row expected at `key`, each the next of
/// its kind in key order. A row's key is SQLite's integer: one below
/// zero, which no writer gives, stands before every key and is a stray,
/// carried in its bits ([`place`] reads it back), as [`super::first_parted`]
/// carries one past every expected key. A column is compared as its
/// bytes, so a text that is not UTF-8 parts there as the column it is.
/// A park whose `facts_seq` is not its run's head is a lag: its immutable
/// columns are compared, its dated columns are not, and `stale` is set.
fn parted(
    key: u64,
    expected: &[(&'static str, Value)],
    row: &rusqlite::Row<'_>,
    stale: &mut bool,
) -> Result<Option<(u64, IndexFault)>, StoreError> {
    let found: i64 = row.get(0)?;
    if found != place(key) {
        return Ok(Some(if found > place(key) {
            (key, IndexFault::Missing)
        } else {
            (found as u64, IndexFault::Stray)
        }));
    }
    let facts = expected
        .iter()
        .position(|(column, _)| *column == "facts_seq");
    let behind = match facts {
        // Only a park's expected stamp is a seq; every other row's is NULL.
        Some(at) if matches!(expected[at].1, Value::Integer(_)) => {
            row.get_ref(at + 1)? != ValueRef::from(&expected[at].1)
        }
        Some(_) | None => false,
    };
    if behind {
        *stale = true;
    }
    for (at, (column, value)) in expected.iter().enumerate() {
        if behind && dated(column) {
            continue;
        }
        if row.get_ref(at + 1)? != ValueRef::from(value) {
            return Ok(Some((key, IndexFault::Column(column))));
        }
    }
    Ok(None)
}

/// A seq or a place as SQLite holds it, an integer: a key [`parted`]
/// carried in its bits reads back as the value its row holds, a stray
/// below zero included.
pub(crate) fn place(key: u64) -> i64 {
    key as i64
}
