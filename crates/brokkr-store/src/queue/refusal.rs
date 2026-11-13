//! Why the queue refused, typed and with the operator's text. Nothing was
//! written when one is returned.

use thiserror::Error;

use super::claim::{Claim, Generation};
use super::entry::Latch;
use super::storage::QUEUE_SCHEMA;
use super::EntryId;

/// The key of a duplicated cool-down row (decision 0103, slice 4d1): the
/// run, event seq, site and round a lost unique index no longer holds
/// apart, boxed so the refusal stays small as the store's callers carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooldownDuplicate {
    pub run_id: String,
    pub event_seq: i64,
    pub site: String,
    pub rounds: i64,
}

/// Why the queue refused, with the operator's text. Nothing was written.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum QueueRefusal {
    #[error("queue entry {0} does not exist")]
    UnknownEntry(EntryId),
    #[error("a new queue entry cannot wait for entry {0}, which does not exist")]
    UnknownWait(EntryId),
    #[error("a new queue entry names entry {0} to wait for twice")]
    DuplicateWait(EntryId),
    #[error("queue entry {entry} cannot move to place {to}: the queue's places run 1 to {last}")]
    PastTheEnds { entry: EntryId, to: u32, last: u32 },
    #[error("queue entry {0} is already held")]
    AlreadyHeld(EntryId),
    #[error("queue entry {0} is not held")]
    NotHeld(EntryId),
    #[error("queue entry {0} is held; release it first")]
    Held(EntryId),
    #[error("queue entry {entry} is claimed by run '{run}'")]
    Claimed { entry: EntryId, run: String },
    #[error("queue entry {0} was dropped")]
    Dropped(EntryId),
    #[error("run '{run}' already started queue entry {entry}")]
    RunTaken { run: String, entry: EntryId },
    #[error("queue storage {found} unsupported (want {QUEUE_SCHEMA})")]
    SchemaMismatch { found: u32 },
    /// A re-pin over a latch that no longer stands: a peer latched again,
    /// or re-pinned, after the caller read it. `standing` is the latch
    /// that stands now, read in the transaction that refused.
    #[error("queue entry {entry}'s latched hold is not the one the re-pin was shown")]
    LatchMoved {
        entry: EntryId,
        standing: Option<Latch>,
    },
    /// A re-pin over a pin that no longer stands: a peer re-pinned after
    /// the caller read it.
    #[error("queue entry {entry} was re-pinned after the re-pin was shown")]
    PinMoved { entry: EntryId },
    /// A latch measured against a pin or a latch that no longer stands: a
    /// peer re-pinned or latched after the caller read the entry.
    #[error(
        "queue entry {entry} was re-pinned or latched after its finding was measured, and the \
         finding was not latched"
    )]
    Unmeasured { entry: EntryId },
    /// A journal that records its queue, and has lost one of its tables.
    #[error(
        "the journal records a queue but its {table} table is gone; what the queue held cannot \
         be read, and it is never recreated empty"
    )]
    TableLost { table: &'static str },
    /// A cool-down row whose stored instant this reader cannot spell, named
    /// by its row (decision 0103 ruling 2(a), slice 4d1): a whole number of
    /// seconds outside `time`'s range is never read as a wait.
    #[error("the journal's provider cool-down {seq} records a time it cannot read")]
    CooldownTime { seq: i64 },
    /// A lost unique cool-down index over duplicated rows (0103, slice 4d1):
    /// the run, event seq, site and round of the duplicate, boxed so the
    /// error stays small as the store's callers carry it.
    #[error("the journal's provider cool-downs hold run '{}' event {} site '{}' round {} more than once; the lost unique index is not rebuilt over them", .0.run_id, .0.event_seq, .0.site, .0.rounds)]
    CooldownDuplicate(Box<CooldownDuplicate>),
    /// A command on an entry the dispatcher has claimed and not yet
    /// linked, returned or held (decision 0095 ruling 1).
    #[error("queue entry {0} is launching; the dispatcher links, returns or holds it first")]
    Launching(EntryId),
    /// A claim while this other entry stands launching: one entry is
    /// claimed at a time (decision 0095 ruling 7).
    #[error(
        "queue entry {0} is launching, and no other entry is claimed until it is linked, \
         returned or held"
    )]
    InFlight(EntryId),
    /// A claim of an entry re-pinned or latched since the pass that
    /// admitted it read it (decision 0095 ruling 7).
    #[error(
        "queue entry {0} was re-pinned or latched after the pass that admitted it read it, and \
         it was not claimed"
    )]
    Unseen(EntryId),
    /// A dispatcher's step under a claim that does not stand for it: the
    /// entry was claimed again, returned, held or linked since, or the
    /// step was taken already (decision 0095 ruling 1).
    #[error(
        "queue entry {}'s claim {} does not stand for this step",
        .0.entry,
        .0.seq
    )]
    Stale(Claim),
    /// A dispatcher's step under a claim another generation took (decision
    /// 0095 ruling 1).
    #[error(
        "queue entry {}'s claim {} was not taken by generation {}",
        .0.entry,
        .0.seq,
        .1.0
    )]
    OtherGeneration(Claim, Generation),
    /// A dispatcher's step under a generation that is not the journal's
    /// latest holder: a newer dispatcher fenced it out (decision 0095 ruling 2).
    #[error("dispatcher generation {} is not the journal's latest holder; it writes nothing", .0.0)]
    Superseded(Generation),
    /// A start under a claim whose bundle is not the one the pass measured:
    /// the manifest digest the claim recorded, which the start's is not
    /// (decision 0095 ruling 7). The start's is its caller's own.
    #[error(
        "queue entry {}'s claim {} admitted the bundle of manifest digest {}, and this start's \
         is another; no run was written",
        .0.entry,
        .0.seq,
        .1
    )]
    BundleMoved(Claim, String),
    /// Runs whose manifests carry one claim, more than one, which no
    /// dispatcher writes (decision 0095 ruling 3): none is adopted.
    #[error(
        "queue entry {}'s claim {} is carried by runs '{}', and a claim starts one run; none is \
         adopted",
        .0.entry,
        .0.seq,
        .1.join("', '")
    )]
    Carried(Claim, Vec<String>),
}
