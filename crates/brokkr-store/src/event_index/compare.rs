//! Ruling 4's comparison of a run's index rows with its events: the
//! expected columns of one row at a time, the first seq where a row is
//! not its event's, and a park's or a checkpoint's dated facts left for the
//! refold that writes them.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::event_index::Indexed;
use brokkr_core::summary::{checkpoints, parks, Checkpoint, Park};
use rusqlite::types::{Value, ValueRef};
use rusqlite::Connection;

use super::checkpoint::{facts as checkpoint_facts, row_columns, COLUMNS as CHECKPOINT_COLUMNS};
use super::{park_facts, IndexFault, PARK_COLUMNS};
use crate::StoreError;

/// The columns `event` projects to after its key, by name: the event's
/// own, the dated facts a park's row keeps and then the dated facts a
/// checkpoint's row keeps. `park` is the whole run's reading of the event
/// where it is a `run/parked` row, `checkpoint` the reading where it is an
/// `effect/checkpointed` row, and `head` the seq the park's reading covers.
/// A row core read no facts for is one whose stamp did not parse or which
/// named no attempt: its facts are null, but it is still processed, so a
/// park's `facts_seq` is the head and a checkpoint's `facts` is 1.
pub(super) fn projects(
    event: &EventEnvelope,
    park: Option<&Park>,
    checkpoint: Option<&Checkpoint>,
    head: u64,
) -> Result<Vec<(&'static str, Value)>, StoreError> {
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
    let is_checkpoint = checkpoint.is_some() || event.event_type == EventType::EffectCheckpointed;
    let checkpoint_facts = match checkpoint {
        Some(checkpoint) => checkpoint_facts(Some(checkpoint))?,
        None if is_checkpoint => checkpoint_facts(None)?,
        None => std::array::from_fn(|_| Value::Null),
    };
    for (value, column) in checkpoint_facts.into_iter().zip(CHECKPOINT_COLUMNS) {
        if is_checkpoint && PARK_COLUMNS.contains(&column) {
            // The time and phase are one column each on the row, a park's
            // and a checkpoint's; a checkpoint's own facts fill them.
            if let Some((_, held)) = columns.iter_mut().find(|(name, _)| *name == column) {
                *held = value;
            }
        } else if !PARK_COLUMNS.contains(&column) {
            columns.push((column, value));
        }
    }
    Ok(columns)
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
/// behind its head or whose checkpoint facts are not written: its dated
/// columns are not compared, since the refold that writes them is what a
/// later open does.
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

/// Which written marker says a row's dated facts wait for the refold.
#[derive(Clone, Copy)]
enum Marker {
    /// A park's `facts_seq`: NULL, or another head than the one expected,
    /// which the trigger and the repair both send behind.
    Park,
    /// A checkpoint's `facts`: only NULL. Any other marker a writer did not
    /// write is an edit, named by its column rather than passed as a lag
    /// forever (decision 0089 ruling 4).
    Checkpoint,
}

/// Whether `row`'s marker `name` says its dated facts wait for the refold,
/// by the reading [`Marker`] names.
fn behind(
    expected: &[(&'static str, Value)],
    row: &rusqlite::Row<'_>,
    name: &str,
    kind: Marker,
) -> Result<bool, StoreError> {
    let Some(at) = expected.iter().position(|(column, _)| *column == name) else {
        return Ok(false);
    };
    if !matches!(expected[at].1, Value::Integer(_)) {
        return Ok(false);
    }
    let found = row.get_ref(at + 1)?;
    Ok(match kind {
        Marker::Park => found != ValueRef::from(&expected[at].1),
        Marker::Checkpoint => found == ValueRef::Null,
    })
}

/// Where `row` parts from the row expected at `key`, each the next of
/// its kind in key order. A row's key is SQLite's integer: one below
/// zero, which no writer gives, stands before every key and is a stray,
/// carried in its bits ([`place`] reads it back), as [`super::first_parted`]
/// carries one past every expected key. A column is compared as its
/// bytes, so a text that is not UTF-8 parts there as the column it is.
/// A park whose `facts_seq` is not its run's head, or a checkpoint whose
/// `facts` are not written, is a lag: its immutable columns are compared, its
/// dated columns are not, and `stale` is set. A checkpoint marker that is
/// neither NULL nor 1 is compared like any other column, so an invalid
/// marker is named, never silently treated as a lag.
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
    let park_behind = behind(expected, row, "facts_seq", Marker::Park)?;
    let checkpoint_behind = behind(expected, row, "facts", Marker::Checkpoint)?;
    if park_behind || checkpoint_behind {
        *stale = true;
    }
    for (at, (column, value)) in expected.iter().enumerate() {
        let skipped = (park_behind && PARK_COLUMNS.contains(column))
            || (checkpoint_behind && CHECKPOINT_COLUMNS.contains(column));
        if skipped {
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

/// A run's index rows in seq order, by its key: what ruling 4 compares,
/// the event's own columns, the park's dated ones and then the
/// checkpoint's, in the order [`compare::projects`] gives them.
pub(crate) static ROWS: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT seq, type, attempt_id, member, recorded_at, event_id, {}
         FROM event_index WHERE run_id = ?1 ORDER BY seq",
        PARK_COLUMNS
            .iter()
            .chain(row_columns().iter())
            .copied()
            .collect::<Vec<_>>()
            .join(", ")
    )
});

/// How a run's rows stand against its events (ruling 4): the first seq
/// where a row is not its event's, a park whose dated facts are not its
/// head's (a lag the next writable open refolds, never a disagreement),
/// or every row its event's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexReading {
    Agrees,
    Behind,
    Parted(u64, IndexFault),
}

/// The first seq where `run_id`'s index rows are not what `events`, the
/// run read whole, project to, and how: the rows streamed beside what is
/// expected, one row compared at a time. A park whose facts are not
/// written at the run's head or a checkpoint's unwritten facts are a lag:
/// its immutable columns are still compared, and its dated facts are left
/// for the refold that writes them. The expected rows are projected up
/// front because a token count past the signed 64-bit bound is a typed
/// refusal, not a value to compare.
pub(crate) fn first_unindexed(
    conn: &Connection,
    run_id: &str,
    events: &[EventEnvelope],
) -> Result<IndexReading, StoreError> {
    let mut statement = conn.prepare_cached(&ROWS)?;
    let rows = statement.query([run_id])?;
    let parks: BTreeMap<u64, Park> = parks(events)
        .into_iter()
        .map(|park| (park.seq, park))
        .collect();
    let checkpoints: BTreeMap<u64, Checkpoint> = checkpoints(events)
        .into_iter()
        .map(|checkpoint| (checkpoint.seq, checkpoint))
        .collect();
    let head = events.last().map_or(0, |event| event.seq);
    let mut stale = false;
    let expected = events
        .iter()
        .map(|event| {
            Ok((
                event.seq,
                projects(
                    event,
                    parks.get(&event.seq),
                    checkpoints.get(&event.seq),
                    head,
                )?,
            ))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let parted = first_parted_at(rows, expected.into_iter(), &mut stale);
    let parted = parted?;
    Ok(match parted {
        Some((seq, fault)) => IndexReading::Parted(seq, fault),
        None if stale => IndexReading::Behind,
        None => IndexReading::Agrees,
    })
}
