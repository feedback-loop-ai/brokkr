//! The checkpoints' dated facts (decision 0089 ruling 7): each
//! `effect/checkpointed` row keeps its time, the run's phase, the cost the
//! record reported or none, the five token counts, whether it is one of its
//! seat's turns, whether it is its participant's last finishing session
//! record, and whether its turn repeats its site's last message, with the
//! mark that says they were written. The facts are
//! brokkr-core's one reading, so the writer, the backfill and ruling 4's
//! comparison derive them one way (decision 0071 ruling 5).
//!
//! What the record states the writer writes from the carried account at the
//! append it covers, one row per sealed checkpoint, so an append costs the
//! checkpoint it writes and never the run's earlier ones. A writer that
//! carries no account, as a second writable open does, writes the same
//! facts from the event alone, with the run's phase read by its key. The one
//! fact that moves, `session`, is brokkr-core's reading alone: the trigger
//! never decides it, it only sends behind the rows whose session could
//! change, and the writer refolds them from the participant's rows, as #912's
//! parks are sent behind and refolded. The refold reads the participant's
//! rows by their own key, never the attempt's starts: main keys the flag by
//! a participant, so the rows alone say which record is the last. `repeat` never moves, since it reads only
//! earlier records: a row whose flag is not written is NULL, and the
//! backfill's whole-run walk writes it like the other facts. `facts_seq` is the stamp that says a row's
//! session was computed; `facts_seq IS NOT seq` is the off-head check, so a
//! row the trigger sent behind, or one an older writer left, is a lag until
//! the next writable open refolds its site. A journal from before the facts
//! is altered in place and walked once, marked unfinished until the walk
//! writes every row; a row core read no facts for is still marked processed,
//! so a later open does not read its run again.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::event_index::Indexed;
use brokkr_core::summary::{checkpoints, Account, Bounded, Checkpoint, NAME_BYTES};
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};

use crate::event_index::{finished, mark_finished, CHECKPOINTED};
use crate::{head_of, patiently, Store, StoreError};

/// The meta key that records the checkpoint facts' backfill finished
/// (ruling 8): its own, so a journal from before them is walked once.
pub(super) const BACKFILLED: &str = "event_index_checkpoint_backfill";

/// The checkpoint triggers a writer from before the session refold
/// installed and this build retired. Such a writer makes them again
/// whenever it finds them gone, and while they stand they decide `session`
/// in SQL; the event index's repair drops them and sends the checkpoints
/// behind for the writer's refold (decision 0089 rulings 4 and 7). One of
/// them standing is enough: the repair asks after both in the one
/// `sqlite_master` read it already makes of the kept triggers, so an open
/// costs no statement for them.
pub(super) const RETIRED: [&str; 2] = [
    "event_index_checkpoint_moved",
    "event_index_checkpoint_finished",
];

/// The checkpoint facts' columns, in the order [`facts`] gives them: the
/// time, the run's phase, the cost and whether it was reported, the five
/// token counts, whether it is one of its seat's turns, whether it is its
/// site's session record, and the mark that the row's facts were written at
/// all. Each is NULL on a row that is not a checkpoint; `turn` is NULL on a
/// row a writer from before it left, and a lag until the next writable open
/// writes it (decision 0089 rulings 4 and 7).
pub(super) const COLUMNS: [&str; 13] = [
    "at_ms",
    "phase",
    "usd",
    "reported",
    "input_tokens",
    "output_tokens",
    "cache_read_tokens",
    "cache_write_tokens",
    "reasoning_output_tokens",
    "session",
    "turn",
    "repeat",
    "facts",
];

/// The two columns the event index already carries, which a checkpoint row
/// writes as a park row does.
const EXISTING: [&str; 2] = ["at_ms", "phase"];

/// The type of a column [`add_columns`] adds, the two it shares with a
/// park's left out.
fn kind(column: &str) -> &'static str {
    match column {
        "usd" => "REAL",
        _ => "INTEGER",
    }
}

/// The checkpoint facts' columns after the park's, the two it shares with
/// them left out, in [`facts`]' order: what a row's select adds.
pub(super) fn row_columns() -> Vec<&'static str> {
    COLUMNS
        .iter()
        .copied()
        .filter(|column| !EXISTING.contains(column))
        .collect()
}

/// The columns [`add_columns`] adds, declared for a table made anew.
pub(super) fn declared() -> String {
    COLUMNS
        .iter()
        .filter(|column| !EXISTING.contains(column))
        .map(|column| format!("{column} {}", kind(column)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Add the columns a journal from before the checkpoint facts lacks, one
/// `ALTER` each as SQLite has no `ADD COLUMN IF NOT EXISTS`, presence asked
/// rather than an error swallowed. The two shared with a park's are already
/// there.
pub(super) fn add_columns(conn: &Connection) -> Result<(), StoreError> {
    for column in COLUMNS.iter().filter(|column| !EXISTING.contains(column)) {
        let present = crate::schema::holds(conn, "event_index", [*column])?;
        if !present {
            conn.execute(
                &format!(
                    "ALTER TABLE event_index ADD COLUMN {column} {}",
                    kind(column)
                ),
                [],
            )?;
        }
    }
    Ok(())
}

/// The checkpoint facts in [`COLUMNS`]' order: the phase cut where it is
/// stored, the cost and its word, the five token counts, whether it is one
/// of its seat's turns, the session flag, and the mark that says they were
/// written. `None` is a checkpoint row core read no facts for, whose stamp
/// did not parse or which named no attempt: it keeps no facts, but it is
/// still marked processed with no turn and no repeat, so a later open does
/// not read its run for it again. The writer's positional row and the
/// comparison's named row take both from here, so the cut has one home. A
/// token count past the signed 64-bit integer SQLite keeps is refused by
/// name, never wrapped into a negative fact (decision 0089 ruling 7).
pub(super) fn facts(checkpoint: Option<&Checkpoint>) -> Result<[Value; COLUMNS.len()], StoreError> {
    let Some(checkpoint) = checkpoint else {
        let mut values: [Value; COLUMNS.len()] = std::array::from_fn(|_| Value::Null);
        // A checkpoint core read no facts for is still processed: it keeps
        // no turn and no repeated turn, and its facts marker says it was
        // written.
        values[COLUMNS.len() - 3] = Value::Integer(0);
        values[COLUMNS.len() - 2] = Value::Integer(0);
        values[COLUMNS.len() - 1] = Value::Integer(1);
        return Ok(values);
    };
    let usage = checkpoint.meter.usage();
    let count = |column: &'static str, count: Option<u64>| match count {
        None => Ok(Value::Null),
        Some(count) => i64::try_from(count)
            .map(Value::Integer)
            .map_err(|_| StoreError::TokenCountOutOfRange(checkpoint.seq, column, count)),
    };
    let phase = checkpoint
        .phase
        .as_deref()
        .map(|phase| Bounded::within(phase, NAME_BYTES).cut);
    Ok([
        Value::Integer(checkpoint.at),
        phase.map_or(Value::Null, Value::Text),
        checkpoint.usd.map_or(Value::Null, Value::Real),
        Value::Integer(i64::from(checkpoint.reported)),
        count("input_tokens", usage.and_then(|usage| usage.input_tokens))?,
        count("output_tokens", usage.and_then(|usage| usage.output_tokens))?,
        count(
            "cache_read_tokens",
            usage.and_then(|usage| usage.cache_read_tokens),
        )?,
        count(
            "cache_write_tokens",
            usage.and_then(|usage| usage.cache_write_tokens),
        )?,
        count(
            "reasoning_output_tokens",
            usage.and_then(|usage| usage.reasoning_output_tokens),
        )?,
        Value::Integer(i64::from(checkpoint.session)),
        Value::Integer(i64::from(checkpoint.meter.turn())),
        checkpoint
            .repeat
            .map_or(Value::Null, |repeat| Value::Integer(i64::from(repeat))),
        Value::Integer(1),
    ])
}

/// The statement that sends behind a site's earlier session rows when a
/// checkpoint is appended, inside the event's own transaction: it searches
/// the session partial index, never the run's every event, so only the rows
/// whose `session` could change are marked for the writer's refold
/// (decision 0089 rulings 4 and 7).
pub(super) fn session_behind(event: &str) -> String {
    format!(
        "UPDATE event_index SET facts_seq = NULL
                WHERE run_id = {event}.run_id
                  AND seq IN (
                      SELECT seq FROM event_index INDEXED BY event_index_checkpoint_session
                      WHERE run_id = {event}.run_id
                        AND attempt_id IS {event}.attempt_id
                        AND member IS {event}.member
                        AND seq < {event}.seq
                        AND type = '{CHECKPOINTED}' AND session = 1)"
    )
}

/// The statement that sends behind the appended checkpoint row itself,
/// through the site's stream index: the row whose session the writer is
/// about to compute, never the run's every event.
pub(super) fn row_behind(event: &str) -> String {
    format!(
        "UPDATE event_index SET facts_seq = NULL
                WHERE run_id = {event}.run_id
                  AND seq IN (
                      SELECT seq FROM event_index INDEXED BY event_index_stream
                      WHERE run_id = {event}.run_id
                        AND attempt_id IS {event}.attempt_id
                        AND member IS {event}.member
                        AND seq = {event}.seq
                        AND type = '{CHECKPOINTED}')"
    )
}

/// The predicate that names a `phase/entered` event's row on `events`,
/// guarded so an envelope no reader parses is never extracted: the partial
/// index and the writer's one phase lookup both read it.
fn phase_predicate() -> String {
    format!(
        "CASE WHEN json_valid(envelope) THEN json_extract(envelope, '$.type') END = '{}'",
        brokkr_core::EventType::PhaseEntered.as_str()
    )
}

/// The index that keys a `phase/entered` event's row on `events`: the phase
/// lookup reads the run's phase rows alone, never its every event.
pub(super) fn phase_index() -> String {
    format!(
        "CREATE INDEX IF NOT EXISTS event_index_phase ON events (run_id, seq) WHERE {}",
        phase_predicate()
    )
}

/// The last `phase/entered` event before `seq`, by its key: the run's phase
/// when the event at `seq` was recorded, as brokkr-core's walk reads it.
pub(super) static PHASE_BEFORE: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT envelope FROM events
         WHERE run_id = ?1 AND seq < ?2 AND {}
         ORDER BY seq DESC LIMIT 1",
        phase_predicate()
    )
});

/// The statement that writes one checkpoint's facts, by its key, and the
/// stamp that says its session was computed: the row's own seq, so a later
/// checkpoint at its site cannot make a row another's behind.
pub(super) static WRITE: LazyLock<String> = LazyLock::new(|| {
    let assignments: Vec<String> = COLUMNS
        .iter()
        .enumerate()
        .map(|(at, column)| format!("{column} = ?{}", at + 3))
        .collect();
    format!(
        "UPDATE event_index SET {}, facts_seq = ?{} WHERE run_id = ?1 AND seq = ?2",
        assignments.join(", "),
        COLUMNS.len() + 3
    )
});

/// Write one checkpoint row's facts, inside the caller's transaction.
fn write(tx: &Connection, run_id: &str, seq: u64, facts: &[Value]) -> Result<(), StoreError> {
    let mut bound: Vec<Value> = vec![Value::Text(run_id.to_string()), Value::Integer(seq as i64)];
    bound.extend_from_slice(facts);
    bound.push(Value::Integer(seq as i64));
    let mut statement = tx.prepare_cached(&WRITE)?;
    statement.execute(params_from_iter(bound))?;
    Ok(())
}

/// The candidate rows at a site whose `session` could change, each read by
/// its own key: the site's session rows through the session partial index,
/// its behind rows through the site's stale partial index, and, at an
/// append, the rows from the append's first checkpoint at the site up
/// through the stream index. The union reads a row behind and still its
/// site's session once, and no arm scans the site's history: only the rows
/// that could change are read, never the run.
fn site(from: bool) -> String {
    let appended = if from {
        format!(
            "UNION
             SELECT seq FROM event_index INDEXED BY event_index_stream
             WHERE run_id = ?1 AND attempt_id IS ?2 AND member IS ?3
               AND type = '{CHECKPOINTED}' AND seq >= ?4"
        )
    } else {
        String::new()
    };
    format!(
        "SELECT seq FROM event_index INDEXED BY event_index_checkpoint_session
         WHERE run_id = ?1 AND attempt_id IS ?2 AND member IS ?3
           AND type = '{CHECKPOINTED}' AND session = 1
         UNION
         SELECT seq FROM event_index INDEXED BY event_index_checkpoint_site_stale
         WHERE run_id = ?1 AND attempt_id IS ?2 AND member IS ?3
           AND type = '{CHECKPOINTED}' AND facts_seq IS NOT seq
         {appended}
         ORDER BY seq"
    )
}

pub(super) static SITE: LazyLock<String> = LazyLock::new(|| site(true));
static SITE_BEHIND: LazyLock<String> = LazyLock::new(|| site(false));

/// The seqs [`refold_site`] reads, bound to the participant and, at an
/// append, the first sealed seq at it.
fn candidates(
    conn: &Connection,
    run_id: &str,
    attempt_id: &str,
    member: Option<&str>,
    from: Option<u64>,
) -> Result<Vec<u64>, StoreError> {
    let mut bound = vec![
        Value::Text(run_id.to_string()),
        Value::Text(attempt_id.to_string()),
        member.map_or(Value::Null, |member| Value::Text(member.to_string())),
    ];
    let sql = match from {
        Some(seq) => {
            bound.push(Value::Integer(seq as i64));
            &*SITE
        }
        None => &*SITE_BEHIND,
    };
    let mut statement = conn.prepare_cached(sql)?;
    let found = statement.query_map(params_from_iter(bound), |row| row.get(0))?;
    Ok(found.collect::<Result<_, _>>()?)
}

/// The statement that writes a checkpoint's one moving fact and its stamp,
/// by its key.
pub(super) const WRITE_SESSION: &str =
    "UPDATE event_index SET session = ?3, facts_seq = seq WHERE run_id = ?1 AND seq = ?2";

/// Refold one participant's behind rows from brokkr-core's one reading:
/// read the candidate envelopes through the stream index, let core's fold
/// say which finishes a session, and write its flags. A row core cannot read
/// keeps no session, and is still stamped processed. The dated facts written
/// from the record alone are not touched, and the participant's session rows
/// are read by their own key, never the attempt's starts.
fn refold_site(
    conn: &Connection,
    run_id: &str,
    attempt_id: &str,
    member: Option<&str>,
    from: Option<u64>,
) -> Result<(), StoreError> {
    let seqs = candidates(conn, run_id, attempt_id, member, from)?;
    let mut events = Vec::with_capacity(seqs.len());
    {
        let mut statement = conn.prepare_cached(super::ENVELOPE)?;
        for seq in &seqs {
            let envelope: String =
                statement.query_row(params![run_id, *seq as i64], |row| row.get(0))?;
            events.push(serde_json::from_str(&envelope)?);
        }
    }
    let sessions: BTreeMap<u64, bool> = checkpoints(&events)
        .into_iter()
        .map(|checkpoint| (checkpoint.seq, checkpoint.session))
        .collect();
    let mut statement = conn.prepare_cached(WRITE_SESSION)?;
    for seq in seqs {
        let session = sessions
            .get(&seq)
            .map_or(Value::Null, |session| Value::Integer(i64::from(*session)));
        statement.execute(params![run_id, seq as i64, session])?;
    }
    Ok(())
}

/// Refold the participants the append `sealed` wrote checkpoints for,
/// inside its own transaction: one call for each participant, from the
/// first sealed seq it wrote there, so an append costs its own checkpoints
/// and the participant's session rows, never the run.
pub(crate) fn refold_sealed(
    tx: &Connection,
    run_id: &str,
    sealed: &[EventEnvelope],
) -> Result<(), StoreError> {
    let mut sites: BTreeMap<(String, Option<String>), u64> = BTreeMap::new();
    for event in sealed {
        if event.event_type != EventType::EffectCheckpointed {
            continue;
        }
        let indexed = Indexed::of(event);
        let Some(attempt_id) = indexed.attempt_id else {
            continue;
        };
        sites
            .entry((attempt_id.to_string(), indexed.member.map(str::to_string)))
            .or_insert(event.seq);
    }
    for ((attempt_id, member), from) in sites {
        refold_site(tx, run_id, &attempt_id, member.as_deref(), Some(from))?;
    }
    Ok(())
}

/// The rows a writable open must refold: the checkpoint rows behind their
/// own stamp, sought through the stale partial index.
static BEHIND: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT seq, attempt_id, member FROM event_index INDEXED BY event_index_checkpoint_stale
         WHERE run_id = ?1 AND type = '{CHECKPOINTED}'
           AND (facts IS NULL OR turn IS NULL OR repeat IS NULL OR facts_seq IS NOT seq)"
    )
});

/// The statement that says a run still has checkpoint facts no writer has
/// written, so its backfill reads the run whole rather than refolding sites.
static UNWRITTEN: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT EXISTS (SELECT 1 FROM event_index INDEXED BY event_index_checkpoint_stale
         WHERE run_id = ?1 AND type = '{CHECKPOINTED}'
           AND (facts IS NULL OR turn IS NULL OR repeat IS NULL))"
    )
});

/// Does `run_id` hold a checkpoint row whose record facts are not written?
fn unwritten(conn: &Connection, run_id: &str) -> Result<bool, StoreError> {
    Ok(conn.query_row(&UNWRITTEN, [run_id], |row| row.get(0))?)
}

/// Refold every behind checkpoint site of `run_id`, reading each site's rows
/// through the stream index and never the run: an older writer leaves the
/// rows it sent behind for the next writable open. A row no site names is
/// stamped processed with no session, since core reads no session for it.
fn refold_behind(conn: &Connection, run_id: &str) -> Result<(), StoreError> {
    let mut statement = conn.prepare_cached(&BEHIND)?;
    let rows = statement.query_map([run_id], |row| {
        Ok((
            row.get::<_, u64>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;
    let mut sites = BTreeSet::new();
    let mut named = Vec::new();
    for row in rows {
        let (seq, attempt_id, member) = row?;
        match attempt_id {
            Some(attempt_id) => {
                sites.insert((attempt_id, member));
            }
            None => named.push(seq),
        }
    }
    for seq in named {
        conn.execute(
            "UPDATE event_index SET facts_seq = seq WHERE run_id = ?1 AND seq = ?2",
            params![run_id, seq as i64],
        )?;
    }
    for (attempt_id, member) in sites {
        refold_site(conn, run_id, &attempt_id, member.as_deref(), None)?;
    }
    Ok(())
}

/// One run of the session refold, in its own transaction: the read and the
/// write are one immediate lock, so a peer's append appears whole or not at
/// all. A peer's unwritten facts found under the lock are left for the next
/// open, so the completion claim never stands over a row this pass skipped.
fn refold_once(conn: &mut Connection, run_id: &str) -> Result<bool, StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let wrote = !unwritten(&tx, run_id)?;
    if wrote {
        refold_behind(&tx, run_id)?;
    }
    tx.commit()?;
    Ok(wrote)
}

/// Write the facts of every checkpoint the append `sealed` carries, one row
/// each from the account carried over it: an append costs the checkpoints it
/// writes and never the run's earlier ones.
pub(crate) fn write_appended(
    tx: &Connection,
    account: &Account,
    sealed: &[brokkr_core::EventEnvelope],
) -> Result<(), StoreError> {
    for event in sealed {
        if event.event_type == brokkr_core::EventType::EffectCheckpointed {
            let facts = facts(account.checkpoint(event.seq))?;
            write(tx, account.run_id(), event.seq, &facts)?;
        }
    }
    refold_sealed(tx, account.run_id(), sealed)
}

/// The run's phase when the event at `seq` was recorded: the last
/// `phase/entered` event before it, read by its key through the phase index,
/// as brokkr-core's walk reads it from the events it carries.
fn phase_before(conn: &Connection, run_id: &str, seq: u64) -> Result<Option<String>, StoreError> {
    let found: Option<String> = conn
        .query_row(
            &PHASE_BEFORE,
            rusqlite::params![run_id, seq as i64],
            |row| row.get(0),
        )
        .optional()?;
    let Some(envelope) = found else {
        return Ok(None);
    };
    let event: brokkr_core::EventEnvelope = serde_json::from_str(&envelope)?;
    Ok(brokkr_core::summary::entered_phase(&event).map(str::to_string))
}

/// Write the facts of every checkpoint the append `sealed` carries from the
/// event alone, for a writer that carries no account of the run: the
/// record's own facts, with the run's phase before it read by its key. The
/// refold that follows writes the site's session flag from brokkr-core's
/// reading, so no append path leaves stale facts or a false disagreement
/// (decision 0089 rulings 4 and 7).
pub(crate) fn write_sealed(
    tx: &Connection,
    run_id: &str,
    sealed: &[brokkr_core::EventEnvelope],
) -> Result<(), StoreError> {
    for event in sealed {
        if event.event_type != brokkr_core::EventType::EffectCheckpointed {
            continue;
        }
        let phase = phase_before(tx, run_id, event.seq)?;
        let checkpoint = Checkpoint::read(event, phase.as_deref());
        write(tx, run_id, event.seq, &facts(checkpoint.as_ref())?)?;
    }
    refold_sealed(tx, run_id, sealed)
}

/// Write the facts of every checkpoint of `account`'s run, at the head the
/// account covers: the backfill's walk and an adoption's whole write.
pub(crate) fn write_all(tx: &Connection, account: &Account) -> Result<(), StoreError> {
    for seq in account.checkpoint_rows() {
        let facts = facts(account.checkpoint(*seq))?;
        write(tx, account.run_id(), *seq, &facts)?;
    }
    Ok(())
}

/// Write every checkpoint row of `account`'s run that brokkr-core's
/// reading can express, leaving a row whose count the index's signed
/// columns cannot hold a lag for a reader to read whole and the doctor to
/// name: a legacy run is refused by name, never by failing the journal's
/// open (decision 0089 rulings 4 and 8).
pub(crate) fn write_all_available(tx: &Connection, account: &Account) -> Result<(), StoreError> {
    match write_all(tx, account) {
        Ok(()) | Err(StoreError::TokenCountOutOfRange(..)) => Ok(()),
        Err(error) => Err(error),
    }
}

/// One run of the checkpoint backfill, in its own transaction, written only
/// while the head is still the one `account` was read at; `true` where it
/// wrote, `false` where a peer moved the head and the run is left for the
/// next open, so the backfill is not marked finished over it.
pub(super) fn once(conn: &mut Connection, account: &Account) -> Result<bool, StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let (seq, hash) = head_of(&tx, account.run_id())?;
    let wrote = account.head() == (seq, hash.as_str());
    if wrote {
        write_all(&tx, account)?;
    }
    tx.commit()?;
    Ok(wrote)
}

/// The first run whose checkpoint rows need a write past the walk's cursor,
/// sought through the partial index of rows still to write. A journal may
/// hold the empty run id, which `run_id > ''` cannot name, so the first
/// step has no cursor at all.
fn stale(after: bool) -> String {
    let cursor = if after { "AND run_id > ?1" } else { "" };
    format!(
        "SELECT run_id FROM event_index INDEXED BY event_index_checkpoint_stale
         WHERE type = '{CHECKPOINTED}'
           AND (facts IS NULL OR turn IS NULL OR repeat IS NULL OR facts_seq IS NOT seq) {cursor}
         ORDER BY run_id LIMIT 1"
    )
}

static STALE: LazyLock<String> = LazyLock::new(|| stale(true));
static STALE_FIRST: LazyLock<String> = LazyLock::new(|| stale(false));

/// One ascending walk of the runs whose checkpoint facts still need a
/// write, keyed by the runs' partial index so a journal whose facts are all
/// written reads one empty range and no run. It stops at the first run it
/// cannot load, leaving that run for a reader to read whole and the doctor
/// to name, and the backfill unfinished.
pub(super) struct Walk {
    after: Option<String>,
}

impl Walk {
    pub(super) fn new() -> Self {
        Self { after: None }
    }

    pub(super) fn next(&mut self, conn: &Connection) -> Result<Option<String>, StoreError> {
        let (sql, after) = match &self.after {
            Some(after) => (&*STALE, Some(after.as_str())),
            None => (&*STALE_FIRST, None),
        };
        let mut statement = conn.prepare_cached(sql)?;
        let found: Option<String> = statement
            .query_row(params_from_iter(after), |row| row.get(0))
            .optional()?;
        if let Some(run) = &found {
            self.after = Some(run.clone());
        }
        Ok(found)
    }
}

impl Store {
    /// Write every checkpoint's dated facts that a writer from before them,
    /// or a missing walk, left unwritten, one run per transaction (decision
    /// 0089 ruling 8), and let a run that does not load stay for a reader to
    /// read whole and the doctor to name. Every writable open makes it; the
    /// partial index holds only rows still to write, so a journal whose
    /// facts all stand finds none. A run whose head a peer moves while it is
    /// read writes nothing and leaves the backfill unfinished, so the
    /// completion claim never stands over a row it skipped. `between` runs
    /// on this connection after a run is read and before its transaction,
    /// the instant the tests stage a peer's lock at. The memory is the
    /// walk's cursor and the largest run's events.
    pub(crate) fn checkpoint_behind_with(
        &mut self,
        mut between: impl FnMut(&Connection),
    ) -> Result<(), StoreError> {
        let mut walk = Walk::new();
        let mut unfinished = false;
        let mut cleared = false;
        while let Some(run_id) = walk.next(&self.conn)? {
            if !cleared {
                let conn = &self.conn;
                patiently("checkpoints", self.patience, || clear_backfilled(conn))?;
                cleared = true;
            }
            if !self.checkpoint_run(&run_id, &mut between)? {
                unfinished = true;
            }
        }
        let done = !unfinished && (cleared || !finished(&self.conn, BACKFILLED)?);
        if done {
            let conn = &self.conn;
            patiently("checkpoints", self.patience, || {
                mark_finished(conn, BACKFILLED)
            })?;
        }
        Ok(())
    }
}

impl Store {
    /// One run's checkpoint facts, written in a transaction of its own:
    /// whether they now stand. A run whose rows lag only by their session
    /// is refolded from its sites' checkpoints without reading the run; any
    /// other run is read whole and written. A run that does not load, a
    /// legacy count the signed columns cannot hold (refused by name for a
    /// reader to read whole and the doctor to name, decision 0089 rulings 4
    /// and 8), and a head a peer moved while the run was read all leave the
    /// facts unwritten, so the backfill stays unfinished until an open
    /// writes them. `between` runs after the read and before the
    /// transaction.
    fn checkpoint_run(
        &mut self,
        run_id: &str,
        between: &mut impl FnMut(&Connection),
    ) -> Result<bool, StoreError> {
        if !unwritten(&self.conn, run_id)? {
            between(&self.conn);
            let conn = &mut self.conn;
            return patiently("checkpoints", self.patience, || refold_once(conn, run_id));
        }
        let Ok(events) = self.load(run_id) else {
            return Ok(false);
        };
        let account = Account::whole(run_id, &events);
        between(&self.conn);
        let conn = &mut self.conn;
        match patiently("checkpoints", self.patience, || once(conn, &account)) {
            Err(StoreError::TokenCountOutOfRange(..)) => Ok(false),
            written => written,
        }
    }
}

/// Mark the checkpoint facts' backfill unfinished, before a pass that has
/// rows to write: a run that does not load leaves it so.
fn clear_backfilled(conn: &Connection) -> Result<(), StoreError> {
    conn.execute("DELETE FROM meta WHERE key = ?1", [BACKFILLED])?;
    Ok(())
}

/// Does the table hold the checkpoint facts?
#[cfg(test)]
pub(super) fn checkpointed(conn: &Connection) -> Result<bool, StoreError> {
    crate::schema::holds(conn, "event_index", COLUMNS)
}

/// Has the checkpoint facts' backfill finished?
#[cfg(test)]
pub(super) fn backfilled(conn: &Connection) -> Result<bool, StoreError> {
    crate::event_index::finished(conn, BACKFILLED)
}
