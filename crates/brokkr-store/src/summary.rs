//! The run summary (decision 0089, slice 1): one row per run, the
//! fleet's whole account of it, kept in the journal's own file and in the
//! transaction of every insert that moves it.
//!
//! Two keepers, split by what each needs (ruling 3). What the row itself
//! states, SQLite keeps by trigger, so every writer keeps it, an older
//! binary's included: the row's presence, the run's head and its last
//! stamp, and whether an acknowledgement stands at that head, with its
//! words copied cut. What the fold states, the writer keeps: an append
//! made while the store carries the writer's [`Account`] of the run, at
//! the head the append found, writes the [`Summary`] of the account
//! carried over what it sealed. Any other append writes nothing folded,
//! so the row falls behind its head, its `lane` reads `behind`, and the
//! next writable open refolds it ([`Store::catch_up`]).
//!
//! The row is derived, never exported and never hashed: the journal's
//! events stay the record (decisions 0001 and 0003), and no
//! `brokkr-runtime` decision reads a row (ruling 1).

use std::sync::LazyLock;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::summary::{
    Account, Bounded, Listing, Summary, ACK_AT_CHARS, ACK_OPERATOR_CHARS, ACK_REASON_CHARS,
    STAMP_BYTES,
};
use brokkr_core::EventEnvelope;
use rusqlite::types::Value;
use rusqlite::{params, Connection, OptionalExtension};

use crate::acknowledgement::standing;
use crate::event_index::{
    finished, first_unindexed, runless, write_all_available, write_checkpoints, write_parks,
    write_sealed, IndexReading,
};
use crate::schema::all_present;
use crate::{head_of, patiently, seat_summary, Acknowledgement, Store, StoreError};

/// The lane a row's range is read from (ruling 6): a row whose fold or any
/// stamped part is not its head's reads behind and whole, so no reader
/// trusts columns older than the head it stands at.
fn lane() -> String {
    format!(
        "CASE
      WHEN folded_seq IS NULL OR folded_seq <> head_seq OR {stale} THEN 'behind'
      WHEN refusal_words IS NOT NULL AND acknowledged = 0 THEN 'refused'
      WHEN refusal_words IS NOT NULL THEN 'acknowledged'
      WHEN status = 'awaiting_operator' THEN 'parked'
      WHEN status = 'running' AND stop_accepted_seq IS NOT NULL THEN 'stopping'
      WHEN status = 'running' THEN 'running'
      ELSE 'finished'
    END",
        stale = stale("")
    )
}

/// Any stamp off the folded head sends the row behind, whichever writer folded it.
fn stale(prefix: &str) -> String {
    let [stopping, ended] = [stop::PART, ending::PART].map(|part| part.stale(prefix));
    format!("{} OR {stopping} OR {ended}", aggregate::stale(prefix))
}

/// The meta key that records the backfill finished (ruling 8).
pub(crate) const BACKFILLED: &str = "run_summary_backfill";

/// A row's place in the page index's order: its `order_at` and run id.
type Place = (i64, String);

mod aggregate;
mod columns;
mod comparison;
mod disagreement;
mod ending;
mod stop;

pub use disagreement::Disagreement;

const TABLE: [&str; 1] = ["run_summary"];
const INDEX: [&str; 2] = ["run_summary_page", aggregate::INDEX];
const TRIGGERS: [&str; 7] = [
    "run_summary_of_a_run",
    columns::HEAD_TRIGGER,
    "run_summary_of_an_acknowledgement",
    "run_summary_stamp_bounded",
    stop::PART.trigger,
    ending::PART.trigger,
    aggregate::TRIGGER,
];

/// The stamp an envelope's text carries, NULL where it carries none as
/// text, and read only from text that is JSON, so an envelope no reader
/// parses never fails its insert. An envelope with two `recorded_at` keys
/// is one serde refuses, so which of them this reads moves no row a
/// reader is shown: such a run does not load.
pub(crate) fn stamp_of(envelope: &str) -> String {
    text_at(envelope, "$.recorded_at")
}

/// The value at `path` in an envelope's text, NULL where it holds none as
/// text, read only from text that is JSON: the one guard every trigger
/// reads an envelope through, so a non-text value writes NULL, never a
/// coerced number or JSON.
pub(crate) fn text_at(envelope: &str, path: &str) -> String {
    format!(
        "CASE WHEN json_valid({envelope}) AND json_type({envelope}, '{path}') = 'text'
           THEN json_extract({envelope}, '{path}') END"
    )
}

/// `stamp` cut to at most [`STAMP_BYTES`]: past its bound, the cut steps
/// back past at most three continuation bytes, so it ends at a character
/// boundary.
pub(crate) fn cut_stamp(stamp: &str) -> String {
    let blob = format!("CAST({stamp} AS BLOB)");
    let start = |at: usize| format!("hex(substr({blob}, {at}, 1)) NOT BETWEEN '80' AND 'BF'");
    let n = STAMP_BYTES;
    format!(
        "CAST(substr({blob}, 1, CASE
             WHEN {} THEN {n} WHEN {} THEN {} WHEN {} THEN {} ELSE {} END) AS TEXT)",
        start(n + 1),
        start(n),
        n - 1,
        start(n - 1),
        n - 2,
        n - 3,
    )
}

/// The columns an acknowledgement standing at the row's head sets,
/// `acknowledgement` naming its row, its words copied cut.
fn acknowledged_by(acknowledgement: &str) -> String {
    format!(
        "acknowledged = 1,
         ack_operator = substr({acknowledgement}.operator, 1, {ACK_OPERATOR_CHARS}),
         ack_reason = substr({acknowledgement}.reason, 1, {ACK_REASON_CHARS}),
         ack_at = substr({acknowledgement}.recorded_at, 1, {ACK_AT_CHARS})"
    )
}

/// The table, its indexes and triggers, `IF NOT EXISTS` throughout but the
/// head trigger, which is dropped and made anew so an older build's
/// definition is replaced on the next writable open. The whole values sit
/// last, the citation's too, so a page that selects none reads none; a
/// stamp past its bound is cut by its own trigger and kept whole there. A
/// writer from before the stopping or ending columns leaves their stamp
/// off, and a trigger sends its row behind.
fn migration() -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS run_summary (
            run_id TEXT PRIMARY KEY,
            feature_line TEXT, created_at TEXT, status TEXT, {stopping}, {ended},
            phase TEXT,
            park_reason TEXT, rule_id TEXT, rule_from TEXT, rule_routes INTEGER,
            hire_seat TEXT, hire_failed_attempts INTEGER,
            residual_open TEXT, residual_count INTEGER,
            refusal_seq INTEGER, refusal_words TEXT,
            folded_seq INTEGER, folded_hash TEXT,
            head_seq INTEGER NOT NULL, head_hash TEXT NOT NULL, last_recorded_at TEXT,
            last_at_ms INTEGER,
            acknowledged INTEGER NOT NULL DEFAULT 0,
            ack_operator TEXT, ack_reason TEXT, ack_at TEXT,
            lane TEXT GENERATED ALWAYS AS ({lane}) VIRTUAL,
            order_at INTEGER GENERATED ALWAYS AS
                (COALESCE(last_at_ms, 9223372036854775807)) VIRTUAL,
            {aggregated}, feature TEXT, residuals TEXT, created_at_whole TEXT,
            last_recorded_at_whole TEXT, phase_whole TEXT, park_reason_whole TEXT,
            rule_id_whole TEXT, rule_from_whole TEXT, hire_seat_whole TEXT,
            refusal_words_whole TEXT, {ended_whole}
        );
        CREATE INDEX IF NOT EXISTS run_summary_page
            ON run_summary (lane, order_at DESC, run_id DESC);
        CREATE TRIGGER IF NOT EXISTS run_summary_of_a_run AFTER INSERT ON runs BEGIN
            INSERT OR IGNORE INTO run_summary (run_id, head_seq, head_hash)
                VALUES (NEW.run_id, 0, '{ZERO_HASH}');
        END;
        DROP TRIGGER IF EXISTS run_summary_of_an_event;
        {head_trigger};
        CREATE TRIGGER IF NOT EXISTS run_summary_of_an_acknowledgement
            AFTER INSERT ON quarantine_acknowledgements BEGIN
            UPDATE run_summary SET {acknowledged}
                WHERE run_id = NEW.run_id AND head_seq = NEW.head_seq
                  AND head_hash = NEW.head_hash;
        END;
        CREATE TRIGGER IF NOT EXISTS run_summary_stamp_bounded
            AFTER UPDATE OF last_recorded_at ON run_summary
            WHEN length(CAST(NEW.last_recorded_at AS BLOB)) > {STAMP_BYTES} BEGIN
            UPDATE run_summary SET last_recorded_at_whole = NEW.last_recorded_at,
                last_recorded_at = {cut}
                WHERE run_id = NEW.run_id;
        END;
        {stopping_trigger} {ended_trigger} {indexed}",
        lane = lane(),
        aggregated = aggregate::defined(),
        indexed = aggregate::migration(),
        head_trigger = columns::head_trigger(),
        cut = cut_stamp("NEW.last_recorded_at"),
        acknowledged = acknowledged_by("NEW"),
        stopping = stop::PART.defined(),
        stopping_trigger = stop::PART.migration(),
        ended = ending::PART.defined(),
        ended_trigger = ending::PART.migration(),
        ended_whole = ending::defined_whole(),
    )
}

/// Every run the journal already carries, given the row the triggers
/// would have kept for it: present, at its head, its stamp bounded,
/// acknowledged where an acknowledgement stands there. A row already
/// folded at its head keeps the fold's values, so a replacement re-derives
/// no row; a row not folded there is left `behind` for the backfill to
/// refold. Each head is found by its seq on the `(run_id, seq)` key first
/// and then read by that whole key, so the seed reads one event a run,
/// never a run's every event.
fn seed() -> String {
    format!(
        "INSERT OR IGNORE INTO run_summary (run_id, head_seq, head_hash)
             SELECT run_id, 0, '{ZERO_HASH}' FROM runs;
         UPDATE run_summary SET head_seq =
             COALESCE((SELECT MAX(seq) FROM events WHERE run_id = run_summary.run_id), 0);
         UPDATE run_summary SET {moved} FROM events AS head
             WHERE head.run_id = run_summary.run_id AND head.seq = run_summary.head_seq
               AND run_summary.folded_seq IS NOT run_summary.head_seq;
         UPDATE run_summary SET {acknowledged} FROM quarantine_acknowledgements AS ack
             WHERE ack.run_id = run_summary.run_id AND ack.head_seq = run_summary.head_seq
               AND ack.head_hash = run_summary.head_hash;",
        moved = columns::moved_to("head"),
        acknowledged = acknowledged_by("ack"),
    )
}

/// Send behind every row a stamp says is not its folded head's, as the
/// triggers do: an older writer may have folded while they were gone. The
/// open's backfill refolds the rows this sends behind (rulings 4 and 8).
fn send_stale_behind() -> String {
    format!(
        "UPDATE run_summary SET folded_seq = NULL, folded_hash = NULL
         WHERE {stale}",
        stale = stale("")
    )
}

/// Does this journal carry the table with every column, index and trigger,
/// the head trigger with this build's definition? Pure reads, so the
/// steady-state open writes nothing.
pub(crate) fn summary_intact(conn: &Connection) -> Result<bool, StoreError> {
    Ok(all_present(conn, "table", &TABLE)?
        && aggregate::shaped(conn)?
        && stop::PART.shaped(conn)?
        && ending::shaped(conn)?
        && all_present(conn, "index", &INDEX)?
        && columns::triggers_intact(conn, &TRIGGERS)?)
}

/// Create the summary, anew where its table lacks a column, seed a row for
/// every run already here, and send behind any row a stamp says the repair
/// did not see written, holding the write lock: the fifth additive repair.
pub(crate) fn migrate_summary(conn: &Connection) -> Result<(), StoreError> {
    if !aggregate::shaped(conn)? || !stop::PART.shaped(conn)? || !ending::shaped(conn)? {
        conn.execute_batch(&aggregate::rebuild())?;
    }
    let unwatched = ending::unwatched(conn)?;
    conn.execute_batch(&(migration() + &unwatched))?;
    conn.execute_batch(&seed())?;
    Ok(conn.execute_batch(&send_stale_behind())?)
}

/// Record that the backfill finished; a journal created with the summary
/// has nothing to backfill.
pub(crate) fn mark_backfilled(conn: &Connection) -> Result<(), StoreError> {
    conn.execute(
        "INSERT OR IGNORE INTO meta (key, value) VALUES (?1, 'finished')",
        [BACKFILLED],
    )?;
    Ok(())
}

/// A bounded value as its two columns hold it.
fn bounded(value: Option<&Bounded>) -> [Value; 2] {
    let text = |text: Option<&String>| text.map_or(Value::Null, |text| Value::Text(text.clone()));
    [
        text(value.map(|value| &value.cut)),
        text(value.and_then(|value| value.whole.as_ref())),
    ]
}

fn integer(value: Option<u64>) -> Value {
    value.map_or(Value::Null, |value| Value::Integer(value as i64))
}

/// The columns the writer keeps, by name, with the values `summary`
/// gives them, in the order a check names the first that differs.
fn written(summary: &Summary) -> Vec<(&'static str, Value)> {
    let ruled = summary.ruled.as_ref();
    let hired = summary.hired.as_ref();
    let refused = summary.refused.as_ref();
    let [created_at, created_at_whole] = bounded(Some(&summary.created_at));
    let [phase, phase_whole] = bounded(summary.phase.as_ref());
    let [park_reason, park_reason_whole] = bounded(summary.park_reason.as_ref());
    let [rule_id, rule_id_whole] = bounded(ruled.and_then(|ruled| ruled.rule_id.as_ref()));
    let [rule_from, rule_from_whole] = bounded(ruled.map(|ruled| &ruled.from));
    let [hire_seat, hire_seat_whole] = bounded(hired.map(|hired| &hired.seat));
    let [refusal_words, refusal_words_whole] = bounded(refused.map(|refused| &refused.words));
    let word =
        |word: Option<&'static str>| word.map_or(Value::Null, |word| Value::Text(word.into()));
    vec![
        ("feature_line", Value::Text(summary.feature_line.clone())),
        ("created_at", created_at),
        ("status", word(summary.status.map(|status| status.as_str()))),
        ("stop_accepted_seq", integer(summary.stop_accepted_seq)),
        ("stop_folded_seq", integer(Some(summary.folded_seq))),
        ("phase", phase),
        ("park_reason", park_reason),
        ("rule_id", rule_id),
        ("rule_from", rule_from),
        (
            "rule_routes",
            integer(ruled.map(|ruled| u64::from(ruled.routes))),
        ),
        ("hire_seat", hire_seat),
        (
            "hire_failed_attempts",
            integer(hired.map(|hired| hired.failed_attempts)),
        ),
        (
            "residual_open",
            word(summary.residual_open.map(|open| open.name())),
        ),
        ("residual_count", integer(Some(summary.residual_count))),
        ("refusal_seq", integer(refused.map(|refused| refused.seq))),
        ("refusal_words", refusal_words),
        ("folded_seq", integer(Some(summary.folded_seq))),
        ("folded_hash", Value::Text(summary.folded_hash.clone())),
        (
            "last_at_ms",
            summary.last_at_ms.map_or(Value::Null, Value::Integer),
        ),
        ("feature", Value::Text(summary.feature.clone())),
        ("residuals", Value::Text(summary.residuals.clone())),
        ("created_at_whole", created_at_whole),
        ("phase_whole", phase_whole),
        ("park_reason_whole", park_reason_whole),
        ("rule_id_whole", rule_id_whole),
        ("rule_from_whole", rule_from_whole),
        ("hire_seat_whole", hire_seat_whole),
        ("refusal_words_whole", refusal_words_whole),
    ]
    .into_iter()
    .chain(aggregate::written(&summary.aggregate, summary.folded_seq))
    .chain(ending::written(summary))
    .collect()
}

/// The columns the triggers keep, as `summary` and the acknowledgement
/// standing at its head give them.
fn kept(summary: &Summary, standing: Option<&Acknowledgement>) -> Vec<(&'static str, Value)> {
    let [last_recorded_at, last_recorded_at_whole] = bounded(summary.last_recorded_at.as_ref());
    let copied = |text: &str, chars: usize| Value::Text(text.chars().take(chars).collect());
    let copy = |field: fn(&Acknowledgement) -> &str, chars: usize| {
        standing.map_or(Value::Null, |standing| copied(field(standing), chars))
    };
    vec![
        ("head_seq", integer(Some(summary.head_seq))),
        ("head_hash", Value::Text(summary.head_hash.clone())),
        ("last_recorded_at", last_recorded_at),
        ("last_recorded_at_whole", last_recorded_at_whole),
        (
            "acknowledged",
            Value::Integer(i64::from(standing.is_some())),
        ),
        (
            "ack_operator",
            copy(|ack| &ack.operator, ACK_OPERATOR_CHARS),
        ),
        ("ack_reason", copy(|ack| &ack.reason, ACK_REASON_CHARS)),
        ("ack_at", copy(|ack| &ack.recorded_at, ACK_AT_CHARS)),
    ]
}

/// The statement that writes every column [`written`] names.
static WRITE: LazyLock<String> = LazyLock::new(|| {
    let empty = Summary::of(
        &Account::empty(""),
        Listing {
            feature: "",
            created_at: "",
        },
    );
    let set: Vec<String> = written(&empty)
        .iter()
        .enumerate()
        .map(|(at, (column, _))| format!("{column} = ?{}", at + 2))
        .collect();
    format!(
        "UPDATE run_summary SET {} WHERE run_id = ?1",
        set.join(", ")
    )
});

/// Write `summary` onto `run_id`'s row, inside the caller's transaction.
pub(crate) fn write(tx: &Connection, run_id: &str, summary: &Summary) -> Result<(), StoreError> {
    let values = written(summary);
    let mut bound: Vec<&dyn rusqlite::ToSql> = vec![&run_id];
    bound.extend(
        values
            .iter()
            .map(|(_, value)| value as &dyn rusqlite::ToSql),
    );
    tx.prepare_cached(&WRITE)?.execute(bound.as_slice())?;
    Ok(())
}

/// What a `runs` row lists, owned so a writer can keep it between appends.
#[derive(Debug, Clone)]
pub(crate) struct Listed {
    feature: String,
    created_at: String,
}

impl Listed {
    pub(crate) fn new(feature: &str, created_at: &str) -> Listed {
        Listed {
            feature: feature.to_string(),
            created_at: created_at.to_string(),
        }
    }

    pub(crate) fn of(tx: &Connection, run_id: &str) -> Result<Listed, StoreError> {
        Ok(tx.query_row(
            "SELECT feature, created_at FROM runs WHERE run_id = ?1",
            params![run_id],
            |row| {
                Ok(Listed {
                    feature: row.get(0)?,
                    created_at: row.get(1)?,
                })
            },
        )?)
    }

    pub(crate) fn listing(&self) -> Listing<'_> {
        Listing {
            feature: &self.feature,
            created_at: &self.created_at,
        }
    }
}

/// The account a writer carries, and what its run's `runs` row lists once read.
#[derive(Debug, Clone)]
pub(crate) struct Carried {
    pub(crate) account: Account,
    pub(crate) listed: Option<Listed>,
}

/// What an append leaves the carried account as, once it commits.
pub(crate) enum Settled {
    /// Unchanged: no account carried, one of another run, or nothing sealed.
    Kept,
    /// The account stood at another head than the append found: it is
    /// not this run's any more, and nothing folded was written.
    Lost,
    /// Carried over what was sealed, and written.
    Carried(Box<Carried>),
}

/// Judge an append inside its transaction: when `carried` is the account
/// of the run `sealed` was appended to, at `found`, the head the append
/// chained onto, carry it over `sealed` and write the summary of the
/// result, and its seat rows the append moved.
pub(crate) fn judge(
    tx: &Connection,
    carried: Option<&Carried>,
    found: (u64, &str),
    sealed: &[EventEnvelope],
) -> Result<Settled, StoreError> {
    let Some(first) = sealed.first() else {
        return Ok(Settled::Kept);
    };
    let Some(carried) = carried.filter(|carried| carried.account.run_id() == first.run_id) else {
        // No account of this run: the checkpoint the append seals still
        // keeps the facts its event states, and its session flag moves off
        // the site's earlier one, so a valid journal is never left looking
        // corrupt (decision 0089 rulings 4 and 7).
        write_sealed(tx, &first.run_id, sealed)?;
        return Ok(Settled::Kept);
    };
    if carried.account.head() != found {
        write_sealed(tx, &first.run_id, sealed)?;
        return Ok(Settled::Lost);
    }
    let account = carried.account.clone().onto(sealed);
    let listed = match &carried.listed {
        Some(listed) => listed.clone(),
        None => Listed::of(tx, &first.run_id)?,
    };
    write(tx, &first.run_id, &Summary::of(&account, listed.listing()))?;
    seat_summary::write(tx, &account, Some(&carried.account))?;
    write_parks(tx, &account)?;
    write_checkpoints(tx, &account, sealed)?;
    Ok(Settled::Carried(Box::new(Carried {
        // The append has written the checkpoints it sealed; keep no earlier
        // one, so the next append's clone of this account holds none
        // (decision 0089 ruling 9).
        account: account.carried(),
        listed: Some(listed),
    })))
}

/// One run of a catch-up, in its own transaction, its summary row and its
/// seat rows: written only while the head is still the one `account` was
/// read at.
pub(crate) fn refold_once(conn: &mut Connection, account: &Account) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let (seq, hash) = head_of(&tx, account.run_id())?;
    if account.head() == (seq, hash.as_str()) {
        let listed = Listed::of(&tx, account.run_id())?;
        write(
            &tx,
            account.run_id(),
            &Summary::of(account, listed.listing()),
        )?;
        seat_summary::write(&tx, account, None)?;
        write_parks(&tx, account)?;
        // A legacy checkpoint count the index cannot hold refuses its own
        // run, not the journal's open.
        write_all_available(&tx, account)?;
    }
    tx.commit()?;
    Ok(())
}

/// How one run's row stands against its fold: the doctor keeps only what
/// disagrees, and the tests' binding names a row behind its head too.
#[derive(Debug)]
pub(crate) enum Compared {
    Agrees,
    /// The row is behind its head and the run loads: a lag, which the
    /// next writer refolds.
    Behind,
    /// The summary row agrees, and the seat rows are behind their head: a
    /// lag too.
    SeatsBehind,
    Disagrees(Disagreement),
}

impl Store {
    /// Carry `account` into every append this store makes to its run, so
    /// each writes the run's summary in its own transaction (ruling 3).
    /// The writer hands over the fold it decided on, at the head it read;
    /// an append that finds another head writes nothing folded, and the
    /// account is dropped until the writer hands over another. The
    /// checkpoint facts already read are dropped as the account is
    /// carried, so an append clones no earlier checkpoint (decision 0089
    /// ruling 9); the event it seals is read again.
    pub fn carry(&mut self, account: Account) {
        self.carried = Some(Carried {
            account: account.carried(),
            listed: None,
        });
        #[cfg(any(test, feature = "test-support"))]
        self.binding.carried();
    }

    /// The checkpoint row seqs the carried account holds, for the test
    /// that an append keeps none of the run's earlier ones (ruling 9).
    #[cfg(test)]
    pub(crate) fn carried_checkpoints(&self) -> Option<usize> {
        self.carried
            .as_ref()
            .map(|carried| carried.account.checkpoint_rows().len())
    }

    /// Keep what a committed append of `landed` events to `run_id` left
    /// the carried account as, and hold the run to its fold when a test's
    /// store drops.
    pub(crate) fn settle(&mut self, (run_id, landed): (&str, usize), settled: Settled) {
        match settled {
            Settled::Kept => {}
            Settled::Lost => self.carried = None,
            Settled::Carried(carried) => self.carried = Some(*carried),
        }
        #[cfg(any(test, feature = "test-support"))]
        self.binding.appended(run_id, landed);
        #[cfg(not(any(test, feature = "test-support")))]
        let _ = (run_id, landed);
    }

    /// Refold every run whose row is behind its head, one run per
    /// transaction (ruling 3), and record the backfill finished. Every
    /// writable open makes it. It spends no patience: a peer's lock ends
    /// it, writing nothing more, and the next open carries on where it
    /// stopped, since a refolded row is no longer behind. A run that does
    /// not load stays behind, for a reader to read whole and refuse live,
    /// and for the doctor's sweep to name. The event index's backfill
    /// follows, on the same terms ([`Store::index_behind_with`]), and the
    /// seat rows' ([`Store::seat_behind_with`]).
    pub(crate) fn catch_up(&mut self) -> Result<(), StoreError> {
        self.quietly(|store| store.refold_behind_with(|_| {}))?;
        self.quietly(|store| store.index_behind_with(|_| {}))?;
        self.quietly(|store| store.park_behind_with(|_| {}))?;
        self.quietly(|store| store.checkpoint_behind_with(|_| {}))?;
        self.quietly(|store| store.seat_behind_with(|_| {}))
    }

    /// `catch`, spending no patience: a peer's lock ends it, quietly.
    pub(crate) fn quietly(
        &mut self,
        catch: impl FnOnce(&mut Store) -> Result<(), StoreError>,
    ) -> Result<(), StoreError> {
        let patience = self.patience;
        self.set_patience(std::time::Duration::ZERO)?;
        let caught = catch(self);
        self.set_patience(patience)?;
        match caught {
            Err(contended) if contended.is_contention() => Ok(()),
            caught => caught,
        }
    }

    /// The refold, with `between` run on this connection after each run is
    /// read and before its transaction: the instant a peer's append makes
    /// the read stale, which the tests stage there. The behind runs are
    /// read by their place one run at a time, no cursor held across a
    /// write, so the memory is the largest run's however many fell behind.
    fn refold_behind_with(
        &mut self,
        mut between: impl FnMut(&Connection),
    ) -> Result<(), StoreError> {
        let mut after: Option<Place> = None;
        while let Some(place) = self.behind_after(after.as_ref())? {
            self.refold(&place.1, &mut between)?;
            after = Some(place);
        }
        if !finished(&self.conn, BACKFILLED)? {
            let conn = &self.conn;
            patiently("catch_up", self.patience, || mark_backfilled(conn))?;
        }
        Ok(())
    }

    /// Every run read whole, one run at a time, so the memory is the
    /// largest run's (ruling 4), whose row disagrees with its fold, whose
    /// event index is not what its events project to, or which does not
    /// load, then every run the index or the seat summary holds rows or a
    /// seat head of and the journal does not: what `brokkr doctor` names
    /// as failures. A run that loads,
    /// whose index agrees and whose row is behind its head is a lag, not a
    /// disagreement, and is passed.
    pub fn summary_disagreements(&self) -> Result<Vec<Disagreement>, StoreError> {
        let mut found = Vec::new();
        self.each_run_compared(|_, compared| {
            if let Compared::Disagrees(disagreement) = compared {
                found.push(disagreement);
            }
        })?;
        found.extend(runless(&self.conn)?);
        found.extend(seat_summary::runless(&self.conn)?);
        Ok(found)
    }

    /// Compare every run's row with its fold, in run id order, handing
    /// each run id and its outcome to `each` as it is read: the runs are
    /// read by their key one id at a time, so nothing of a run outlives
    /// its comparison.
    pub(crate) fn each_run_compared(
        &self,
        mut each: impl FnMut(&str, Compared),
    ) -> Result<(), StoreError> {
        let mut after: Option<String> = None;
        while let Some(run_id) = self.run_after(after.as_deref())? {
            let compared = match self.compared(&run_id, || {}) {
                Ok(compared) => compared,
                // The run's own projection refusal is its finding, never the
                // sweep's end: the other runs are still compared and the
                // doctor still names it (decision 0089 rulings 4 and 8).
                Err(error @ StoreError::TokenCountOutOfRange(..)) => {
                    Compared::Disagrees(Disagreement::Unread {
                        run_id: run_id.clone(),
                        error,
                    })
                }
                Err(error) => return Err(error),
            };
            each(&run_id, compared);
            after = Some(run_id);
        }
        Ok(())
    }

    /// The first run id past `after`, or the first of all.
    pub(crate) fn run_after(&self, after: Option<&str>) -> Result<Option<String>, StoreError> {
        let next = match after {
            None => self.conn.query_row(
                "SELECT run_id FROM runs ORDER BY run_id LIMIT 1",
                [],
                |row| row.get(0),
            ),
            Some(after) => self.conn.query_row(
                "SELECT run_id FROM runs WHERE run_id > ?1 ORDER BY run_id LIMIT 1",
                [after],
                |row| row.get(0),
            ),
        };
        Ok(next.optional()?)
    }

    /// The first behind run past `after` in the page index's own order,
    /// `(order_at DESC, run_id DESC)`, or the first of all: each a search
    /// of the index (ruling 6's cursor), never a sort of the lane, so the
    /// walk reads the lane once.
    fn behind_after(&self, after: Option<&Place>) -> Result<Option<Place>, StoreError> {
        let place = |row: &rusqlite::Row<'_>| Ok((row.get(0)?, row.get(1)?));
        let next = match after {
            None => self.conn.query_row(
                "SELECT order_at, run_id FROM run_summary WHERE lane = 'behind'
                 ORDER BY order_at DESC, run_id DESC LIMIT 1",
                [],
                place,
            ),
            Some((order_at, run_id)) => self.conn.query_row(
                "SELECT order_at, run_id FROM run_summary
                 WHERE lane = 'behind' AND (order_at, run_id) < (?1, ?2)
                 ORDER BY order_at DESC, run_id DESC LIMIT 1",
                params![order_at, run_id],
                place,
            ),
        };
        Ok(next.optional()?)
    }

    /// How `run_id`'s row stands against its fold, read in one read
    /// transaction, so every read sees one snapshot of the journal and a
    /// peer's append between two of them cannot pass for a disagreement.
    /// The run is read whole before its row's place is judged, so a run
    /// that does not load is named whether its row stands at its head or
    /// behind it (ruling 7: the doctor's sweep reads every run whole). The
    /// same read holds the run's event index to its events, wherever the
    /// row stands, since a trigger's row never lags, and its seat rows to
    /// the fold's wherever the summary row stands, a seat head that lags
    /// read as a lag. `between` runs after the run is loaded and before
    /// its row is compared: the instant such an append lands, which the
    /// tests stage.
    pub(crate) fn compared(
        &self,
        run_id: &str,
        between: impl FnOnce(),
    ) -> Result<Compared, StoreError> {
        let _snapshot = self.conn.unchecked_transaction()?;
        let heads: Option<(Option<i64>, i64)> = self
            .conn
            .query_row(
                "SELECT folded_seq, head_seq FROM run_summary WHERE run_id = ?1",
                params![run_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let run_id = run_id.to_string();
        let Some((folded, head)) = heads else {
            return Ok(Compared::Disagrees(Disagreement::Missing { run_id }));
        };
        let events = match self.load(&run_id) {
            Ok(events) => events,
            Err(error) => return Ok(Compared::Disagrees(Disagreement::Unread { run_id, error })),
        };
        let stale = match first_unindexed(&self.conn, &run_id, &events)? {
            IndexReading::Parted(seq, fault) => {
                return Ok(Compared::Disagrees(Disagreement::Indexed {
                    run_id,
                    seq,
                    fault,
                }));
            }
            IndexReading::Behind => true,
            IndexReading::Agrees => false,
        };
        let account = Account::whole(&run_id, &events);
        let seats = seat_summary::compared(&self.conn, &account)?;
        if let Compared::Disagrees(_) = seats {
            return Ok(seats);
        }
        if folded != Some(head) {
            return Ok(Compared::Behind);
        }
        between();
        self.columns_compared(run_id, &account, stale, seats)
    }
}

#[cfg(test)]
mod fixture_tests;
#[cfg(test)]
mod tests;
