//! The queue's entry vocabulary (decision 0068 ruling 1): what an entry
//! waits on, where it stands, the latch admission keeps on it, and the
//! operator commands that move it. Its words are read from the queue tables
//! into these types, and a word this binary does not know refuses the read.

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use thiserror::Error;

use super::Wait;

/// A word a queue table holds that this binary does not know. Only the
/// store's own writers write these columns, so reading one is a journal
/// written by something else, refused rather than guessed at.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("queue column {column} holds '{word}', which this brokkr does not know")]
pub(super) struct UnknownWord {
    pub(crate) column: &'static str,
    pub(crate) word: String,
}

/// Read one word of a closed vocabulary: the member whose `word` it is.
pub(super) fn read_word<T: Copy>(
    value: ValueRef<'_>,
    column: &'static str,
    all: &[T],
    word: fn(T) -> &'static str,
) -> FromSqlResult<T> {
    let text = value.as_str()?;
    all.iter()
        .copied()
        .find(|known| word(*known) == text)
        .ok_or_else(|| {
            FromSqlError::Other(Box::new(UnknownWord {
                column,
                word: text.to_string(),
            }))
        })
}

/// What an entry waits on in the entry it names (decision 0068 ruling 2).
/// A named ending (decision 0064) joins when #317 builds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WaitOn {
    /// The awaited entry's run completed.
    Completed,
    /// The awaited entry's run reached any terminal status.
    Ended,
}

impl WaitOn {
    pub const ALL: [WaitOn; 2] = [WaitOn::Completed, WaitOn::Ended];

    pub fn word(self) -> &'static str {
        match self {
            WaitOn::Completed => "completed",
            WaitOn::Ended => "ended",
        }
    }
}

impl FromSql for WaitOn {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        read_word(value, "queue_waits.condition", &WaitOn::ALL, WaitOn::word)
    }
}

/// Text that is not a `Wait`.
#[derive(Debug, Error, PartialEq, Eq)]
#[error(
    "'{0}' is not ENTRY:CONDITION, with CONDITION one of {words}",
    words = WaitOn::ALL.map(WaitOn::word).join(", ")
)]
pub struct NotAWait(pub String);

/// The stored state word. Claimed is not one: an entry is claimed exactly
/// when it names a run, and that column is its one home.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Standing {
    Queued,
    Held,
    Dropped,
    Launching,
}

impl Standing {
    const ALL: [Standing; 4] = [
        Standing::Queued,
        Standing::Held,
        Standing::Dropped,
        Standing::Launching,
    ];

    pub(super) fn word(self) -> &'static str {
        match self {
            Standing::Queued => "queued",
            Standing::Held => "held",
            Standing::Dropped => "dropped",
            Standing::Launching => "launching",
        }
    }
}

impl FromSql for Standing {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        read_word(value, "queue_entries.state", &Standing::ALL, Standing::word)
    }
}

/// Where an entry stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryState {
    /// Waiting its turn.
    Queued,
    /// Kept in its place, and not to be started until released.
    Held,
    /// Claimed by the dispatcher, in its place, before its run exists
    /// (decision 0095 ruling 1): linked to the run it starts, or returned
    /// to the queue or held, before any other entry is claimed.
    Launching,
    /// Started: the run it produced, recorded once.
    Claimed { run: String },
    /// Taken out of the queue by the operator.
    Dropped,
}

impl EntryState {
    /// The state as the queue's readouts name it.
    pub fn word(&self) -> &'static str {
        match self {
            EntryState::Queued => Standing::Queued.word(),
            EntryState::Held => Standing::Held.word(),
            EntryState::Launching => Standing::Launching.word(),
            EntryState::Claimed { .. } => "claimed",
            EntryState::Dropped => Standing::Dropped.word(),
        }
    }

    /// The run a claimed entry started.
    pub fn run(&self) -> Option<&str> {
        match self {
            EntryState::Claimed { run } => Some(run),
            EntryState::Queued | EntryState::Held | EntryState::Launching | EntryState::Dropped => {
                None
            }
        }
    }

    pub(super) fn of(standing: Standing, run: Option<String>) -> EntryState {
        match (run, standing) {
            (
                Some(run),
                Standing::Queued | Standing::Held | Standing::Dropped | Standing::Launching,
            ) => EntryState::Claimed { run },
            (None, Standing::Queued) => EntryState::Queued,
            (None, Standing::Held) => EntryState::Held,
            (None, Standing::Launching) => EntryState::Launching,
            (None, Standing::Dropped) => EntryState::Dropped,
        }
    }
}

/// A realm-drift hold admission latched on an entry (#430): the `latch`
/// command that recorded it, and what admission found, as the runtime
/// encoded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Latch {
    pub(crate) seq: i64,
    pub finding: String,
}

/// A new entry.
#[derive(Debug, Clone, Copy)]
pub struct NewEntry<'a> {
    pub payload: &'a str,
    pub priority: i64,
    pub waits: &'a [Wait],
}

/// The operator commands that change an entry already queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueCommand {
    /// Put the entry at this place, from 1.
    Move {
        to: u32,
    },
    Hold,
    Release,
    Drop,
}

impl QueueCommand {
    pub(super) fn word(self) -> &'static str {
        match self {
            QueueCommand::Move { .. } => "move",
            QueueCommand::Hold => "hold",
            QueueCommand::Release => "release",
            QueueCommand::Drop => "drop",
        }
    }
}
