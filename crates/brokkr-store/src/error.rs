//! Typed store errors (decision 0071 ruling 8).

use std::path::PathBuf;

use brokkr_core::envelope::{ChainError, EventType};
use brokkr_core::fold::Status;
use thiserror::Error;

use crate::{
    Damage, ImportError, ManifestRefusal, QueueRefusal, SeatRecordError, SeatTreeRefusal,
    DATABASE_SCHEMA, RUN_ID_MAX,
};

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// A journal its own bytes say is damaged, the one refusal every read
    /// surface prints (#1000); see [`damage`].
    #[error(
        "the journal '{}' is damaged ({damage}); nothing was read from it; restore it from a \
         backup with `brokkr journal restore` and see the guide's \"A damaged journal\" (#1000)",
        journal.display().to_string().escape_debug()
    )]
    Damaged { journal: PathBuf, damage: Damage },
    /// A stored envelope that is not JSON, before [`damage`] names the
    /// journal it is in as [`StoreError::Damaged`].
    #[error("run '{run_id}' holds an envelope at seq {seq} that is not JSON")]
    Unparsed { run_id: String, seq: i64 },
    #[error("run '{0}' not found")]
    RunNotFound(String),
    #[error("run '{0}' already exists")]
    RunExists(String),
    /// A run id longer than a journal carries (decision 0089 ruling 2).
    #[error(
        "a run id is at most {RUN_ID_MAX} bytes; this one is {length}, and nothing was recorded"
    )]
    RunIdTooLong { length: usize },
    /// A run id no surface could name back, escaped (#820).
    #[error(
        "a run id is not empty and holds no control character; this one is '{0}', and nothing was recorded"
    )]
    RunIdUnnameable(String),
    #[error("chain: {0}")]
    Chain(#[from] ChainError),
    #[error(transparent)]
    SeatRecord(#[from] SeatRecordError),
    #[error("database schema {found} unsupported (want {DATABASE_SCHEMA})")]
    SchemaMismatch { found: u32 },
    #[error("append conflict: seq {seq} already written by another writer")]
    AppendConflict { seq: u64 },
    #[error("head moved: expected seq {expected_seq}, found {found_seq}")]
    HeadMoved { expected_seq: u64, found_seq: u64 },
    /// A non-annotation on a run that reached a terminal event (#440): nothing is written.
    #[error(
        "run '{run_id}' is already concluded ({}); {} is not an operator \
         annotation, and nothing was written",
        status.as_str(),
        event_type.as_str()
    )]
    AfterTerminal {
        run_id: String,
        status: Status,
        event_type: EventType,
    },
    /// The seq, column and count of a token count past the index's signed 64-bit range (decision 0089 ruling 7).
    #[error("checkpoint {0} reports {1} {2} past the index's 64-bit bound; nothing was written")]
    TokenCountOutOfRange(u64, &'static str, u64),
    /// [`Store::load_after`] was handed a head the run never had: no
    /// event at that seq, or one with another hash.
    #[error("unknown head: the run holds no event {seq} with the hash given")]
    UnknownHead { seq: u64 },
    /// [`Store::load_after`] found a row below seq 1, which no suffix read
    /// selects; [`Store::load`] refuses the same journal whole.
    #[error("run '{run_id}' holds a row at seq {seq}, before its chain's first event")]
    RowBeforeChain { run_id: String, seq: i64 },
    /// The queue refused an operator command or a claim (decision 0068).
    #[error(transparent)]
    Queue(#[from] QueueRefusal),
    /// A seat-tree append or repair refused (decision 0104 ruling 6).
    #[error(transparent)]
    SeatTree(#[from] SeatTreeRefusal),
    /// The head an operator acknowledged a quarantined run at already carries an acknowledgement (#511).
    #[error("run '{run_id}' is already acknowledged at event {seq}; nothing was recorded")]
    AlreadyAcknowledged { run_id: String, seq: u64 },
    #[error("the clock '{now}' is not an RFC 3339 instant; nothing was read at a guessed one")]
    Clock { now: String },
    /// A peer still held the journal's write lock when this operation's
    /// whole patience ran out. **Nothing was written.**
    ///
    /// This is a third thing, and the distinction is the point. An
    /// [`StoreError::AppendConflict`] means a peer took the seq; a
    /// [`StoreError::HeadMoved`] means a peer moved the head the caller
    /// decided against — both are *refusals*, verdicts about content,
    /// and neither may ever be retried into place. Contention is an
    /// accident of timing on a lock: the same call, made again, is the
    /// same call. It is typed separately so a caller can end lawfully on
    /// it instead of dying as if the journal had refused it something.
    #[error(
        "contended: a peer still held the journal's write lock after {waited_ms}ms of \
         {operation}; nothing was written"
    )]
    Contended {
        operation: &'static str,
        waited_ms: u128,
    },
    /// The event log committed envelopes for `run_id`, but updating derived
    /// projections failed. The event is durable; callers must NOT treat this
    /// as contention (`is_contention() == false`).
    #[error(
        "projection lag for run '{run_id}': event log committed, but projection update failed ({source})"
    )]
    ProjectionLag {
        run_id: String,
        #[source]
        source: Box<StoreError>,
    },
    /// An event log or projection update diverges from this store's local
    /// `runs`/`events` state.
    #[error("event log for run '{run_id}' diverges from local store state")]
    DivergentEventLog { run_id: String },
    /// A run manifest (or embedded `RunStarted` manifest) passed to
    /// [`ports::Journal`] or [`ports::EventLog`] failed validation: the
    /// manifest must be a JSON object whose `"engine"`, `"feature"`, and
    /// `"bundle_name"` fields (when present) are JSON strings (see
    /// [`ManifestRefusal`]); nothing was recorded.
    #[error("invalid manifest for run '{run_id}': {reason}; nothing was recorded")]
    InvalidManifest {
        run_id: String,
        reason: ManifestRefusal,
    },
    /// An import operation failed an adoption gate (such as an unadoptable
    /// run id or a fold refusal); nothing was recorded.
    #[error(transparent)]
    Import(Box<ImportError>),
    /// A filesystem I/O error from a file-backed event log or store operation.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// A run id that is not a safe single path segment (empty, `.`, `..`, or
    /// containing `/`, `\`, or NUL).
    #[error(
        "unsafe run id '{0}': must be a single path segment without '/', '\\', NUL, '.' or '..'"
    )]
    UnsafeRunId(String),
    /// A symlink was encountered where a regular file or directory was required.
    #[error("refusing symlink at '{}'", .0.display())]
    SymlinkRefused(PathBuf),
    /// A storage path was empty or held a NUL byte and could not be bound.
    #[error("invalid storage path '{}'", .0.display())]
    InvalidPath(PathBuf),
    /// A standalone split database path points to an incompatible or monolithic
    /// SQLite database (such as `forge.db` holding `runs`/`events` tables).
    #[error("refusing foreign database at '{}' (expected {expected})", path.display())]
    ForeignDatabase {
        path: PathBuf,
        expected: &'static str,
    },
    /// A `waits` payload on an effect event failed provider-wait validation.
    #[error("provider waits: {0}")]
    ProviderWait(#[from] brokkr_core::ProviderWaitError),
    /// A provider cool-down field (`rounds` or `retry_after_seconds`) exceeds
    /// SQLite's signed 64-bit integer range; nothing was written.
    #[error(
        "provider cool-down field '{field}' value {value} exceeds 64-bit integer bound; nothing was written"
    )]
    CooldownFieldOutOfRange { field: &'static str, value: u64 },
}

impl StoreError {
    /// Is this the contention accident rather than a refusal or a
    /// defect? The one predicate callers branch on — no error-text
    /// matching anywhere, and [`StoreError::HeadMoved`] answers `false`
    /// here forever.
    pub fn is_contention(&self) -> bool {
        matches!(self, StoreError::Contended { .. })
    }
}
