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

/// The run, event seq, site and rounds of a probe round that is not the
/// next one for its site (decision 0103 ruling 2(a), slice 4d2b), boxed so
/// the store's error stays small as its callers carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooldownRoundMismatch {
    pub run_id: String,
    pub event_seq: u64,
    pub site: String,
    pub rounds: u64,
    pub standing: u64,
}

/// The cool-down row a [`QueueRefusal::CooldownRowUnwritten`] names: the
/// run, event seq, site and round of a terminal's row the table did not
/// write (fix round 0106b1r6, D3), boxed so the store's error stays small.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnwrittenRow {
    pub run_id: String,
    pub event_seq: u64,
    pub site: String,
    pub rounds: u64,
}

/// The probe hold a [`QueueRefusal::ProbeHoldUnwritten`] names: the run
/// whose probe asked, and the provider and route it would have held (fix
/// round 0106b1r6, D2), boxed so the store's error stays small.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnwrittenHold {
    pub run_id: String,
    pub provider: String,
    pub route: String,
}

/// The probe hold a [`QueueRefusal::ProbeHoldUnsettled`] names: the run and
/// the settled attempt whose hold the table kept standing, boxed so the
/// store's error stays small.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsettledHold {
    pub run_id: String,
    pub attempt_id: String,
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
    /// A probe round for an event that is absent or is not the terminal
    /// effect event whose `waits` an episode appends to (decision 0103
    /// ruling 2(a), slice 4d2b). Nothing was written.
    #[error(
        "the journal's event '{run_id}' {event_seq} is not a terminal effect event a probe round \
         can append to"
    )]
    CooldownRoundEvent { run_id: String, event_seq: u64 },
    /// A probe round for a site the terminal event carries no readable
    /// `waits` entry for (decision 0103 ruling 2(a), slice 4d2b). Nothing
    /// was written.
    #[error(
        "the journal's event '{run_id}' {event_seq} carries no readable wait for site '{site}', \
         so it has no probe round"
    )]
    CooldownRoundWait {
        run_id: String,
        event_seq: u64,
        site: String,
    },
    /// A probe round that is not the next one for its site (decision 0103
    /// ruling 2(a), slice 4d2b): a gap or a duplicate would be written, so
    /// nothing was.
    #[error(
        "the journal's provider cool-downs for run '{}' event {} site '{}' stand at round {}; \
         round {} is not the next one",
        .0.run_id, .0.event_seq, .0.site, .0.standing, .0.rounds
    )]
    CooldownRoundNotNext(Box<CooldownRoundMismatch>),
    /// A probe round the table took without writing its row, as a trigger
    /// that ignores the insert leaves it (fix round 0106b1r5, H3): the
    /// probe's hold is not ended over a row that was never written, and
    /// nothing was. It carries the round and the round its site stood at,
    /// boxed as [`QueueRefusal::CooldownRoundNotNext`]'s are.
    #[error(
        "the journal's provider cool-downs wrote no row for run '{}' event {} site '{}' round \
         {}, so the probe's hold on its route stands and nothing was written",
        .0.run_id, .0.event_seq, .0.site, .0.rounds
    )]
    CooldownRoundUnwritten(Box<CooldownRoundMismatch>),
    /// A terminal event's cool-down row, held or cooling, the table took
    /// without writing, as a trigger that ignores the insert leaves it (fix
    /// round 0106b1r6, D3): the event is rolled back with it, so no wait is
    /// journaled that the store does not hold, and nothing was written.
    #[error(
        "the journal's provider cool-downs wrote no row for run '{}' event {} site '{}' round \
         {}, so the event that records its wait was not written either",
        .0.run_id, .0.event_seq, .0.site, .0.rounds
    )]
    CooldownRowUnwritten(Box<UnwrittenRow>),
    /// A probe's hold on its route the table took without writing its row,
    /// as a trigger that ignores the insert leaves it (fix round 0106b1r6,
    /// D2): no hold stands, so the probe is not dispatched, and nothing
    /// was written.
    #[error(
        "the journal's probe holds wrote no row for run '{}' on {}/{}, so no probe runs on it \
         and nothing was written",
        .0.run_id, .0.provider, .0.route
    )]
    ProbeHoldUnwritten(Box<UnwrittenHold>),
    /// A probe's hold whose end cannot be shown strictly after its probe's
    /// deadline, because the first whole second after it is past the last
    /// one the journal can spell (fix round 0106b1r9, E2): a hold clamped
    /// to that second would show an end before the deadline, so none is
    /// written and the probe is not dispatched.
    #[error(
        "the journal's probe holds cannot show run '{}''s hold on {}/{} ending after its \
         deadline: the first whole second after it is past 9999-12-31T23:59:59Z, the last the \
         journal can spell, so no probe runs on it and nothing was written",
        .0.run_id, .0.provider, .0.route
    )]
    ProbeHoldUnending(Box<UnwrittenHold>),
    /// A probe's hold on a queue whose storage holds no table this binary
    /// may write, an older version's or a newer binary's (fix round
    /// 0106b1r9, E3): no hold can be written, so the probe is not
    /// dispatched, and nothing was.
    #[error(
        "the journal's queue storage is not one this binary writes, so it takes no probe hold \
         for run '{}' on {}/{}, no probe runs on it, and nothing was written",
        .0.run_id, .0.provider, .0.route
    )]
    ProbeHoldUnstorable(Box<UnwrittenHold>),
    /// A settlement's end of a probe's hold the table took without writing
    /// it, as a trigger that ignores the update leaves it (the return of fix
    /// round 0106b1r12): the hold still stands, so the settlement is not
    /// reported as written, and none of its ends was.
    #[error(
        "the journal's probe holds kept run '{}''s hold for attempt '{}' standing through its \
         settlement, so its route stays held and none of the settlement's ends was written",
        .0.run_id, .0.attempt_id
    )]
    ProbeHoldUnsettled(Box<UnsettledHold>),
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
