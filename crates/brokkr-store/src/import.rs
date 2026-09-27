//! Adoption (#354): an exported run relocated into this journal whole,
//! and the offline verification it makes on the very bytes it adopts.

use std::path::Path;

use brokkr_core::envelope::{verify_chain, ChainError, EventEnvelope};
use brokkr_core::RunState;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use thiserror::Error;

use crate::{
    now_rfc3339, patiently, refuse_existing, seat_record, SeatRecordError, Store, StoreError,
};

/// Why an adoption refused. Every variant refuses the import WHOLE:
/// nothing is written until all of them have had their say, so a
/// destination journal never holds a prefix of a run it declined.
#[derive(Debug, Error)]
pub enum ImportError {
    #[error(
        "refused: '{0}' is a redacted derivative — redaction rewrites payload \
         bytes and leaves the recorded hashes behind, so its chain can never \
         verify; an import of one could only ever adopt unverifiable content"
    )]
    Redacted(String),
    #[error(transparent)]
    Verify(#[from] VerifyError),
    #[error(
        "refused: this journal already carries run '{0}' — the run_id is hashed \
         into every envelope of its chain, so an adoption can neither rename it \
         nor overwrite what is already here; the collision is the operator's to rule on"
    )]
    Collision(String),
    #[error(
        "refused: '{0}' is not a run id this journal will carry — event hashes are \
         unkeyed, so a chain proves its bytes were not altered and never that whoever \
         sealed them was entitled to the name, and the name does not stay in the \
         database: `brokkr export` composes '<out>/<run_id>.ndjson' from it and every \
         readout prints it. An adoptable id is 1 to {RUN_ID_MAX} characters of ASCII \
         letters, digits, '-' and '_'"
    )]
    UnadoptableRunId(String),
    #[error(
        "refused: the export's run/started event carries no {field} — an adoption \
         derives the destination's runs row from the verified chain, never from \
         the sidecar manifest that no hash covers, and a readout that shows this \
         run must not show a blank where the chain said nothing"
    )]
    Unattested { field: &'static str },
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// A run's arrival: store bookkeeping BESIDE the chain, never inside it.
/// Absent means the run was driven here natively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrival {
    /// When the run landed in THIS journal (RFC3339).
    pub imported_at: String,
    /// Whence it came — the export this journal adopted.
    pub imported_from: String,
}

/// What an adoption did, for the operator's readout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adoption {
    pub run_id: String,
    pub events: usize,
    /// The adopted chain's head — unchanged from the export, since the
    /// bytes it was computed over are unchanged.
    pub head_hash: String,
    pub arrival: Arrival,
}

/// The longest run id an adoption will carry. Native ids are a feature
/// slug of at most 32 characters and eight hex ones, so this is roomy
/// three times over; it exists because the id becomes a path component
/// on the next export, and a bound is cheaper than the error that would
/// otherwise come back from the filesystem.
pub(crate) const RUN_ID_MAX: usize = 128;

/// What a run id may contain for this journal to adopt it.
///
/// A native id is a lowercased feature slug and eight hex characters
/// (`Engine::start_in_world`), so this set is generous already. What it
/// excludes is what a *foreign* id could otherwise do here: `.` (and so
/// `..`), the path separators, and every control and formatting
/// character. Import is the first path by which a run id somebody else
/// authored reaches the `runs` table, and from there `brokkr export`'s
/// `<out>/<run_id>.ndjson` and the operator's terminal.
fn adoptable(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '-' || character == '_'
}

/// A rejected run id as it may be written into a refusal. Every
/// character the gate does not allow becomes its codepoint, so the one
/// line naming the refusal is ASCII by construction — an id that reached
/// here precisely by being unprintable does not get to reorder the
/// sentence that turns it away. The predicate is the gate's own, so the
/// two can never drift apart.
fn escaped(run_id: &str) -> String {
    run_id
        .chars()
        .map(|character| {
            if adoptable(character) {
                character.to_string()
            } else {
                format!("\\u{{{:04x}}}", character as u32)
            }
        })
        .collect()
}

/// An event row written verbatim, past the append fence: an adoption's
/// pre-sealed envelopes, and the rows `test_support` plants.
pub(crate) const INSERT_EVENT: &str =
    "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES (?1, ?2, ?3, ?4)";

/// The `runs` row an adoption writes, every column derived from the
/// verified chain.
struct RunsRow<'a> {
    run_id: &'a str,
    feature: String,
    bundle_name: &'a str,
    manifest: String,
    created_at: &'a str,
}

impl<'a> RunsRow<'a> {
    /// The row a verified, folded journal answers for, or the gate it
    /// fails.
    fn of(events: &'a [EventEnvelope], state: RunState) -> Result<RunsRow<'a>, ImportError> {
        // The fold proved the first event is `run/started`, so it
        // exists; what it does not prove is that the payload carries
        // everything a `runs` row needs.
        let started = &events[0];
        // Gate 4, before the name is derived from, stored under, or
        // printed with. `verify_chain` proved every envelope carries
        // this one run_id (`ChainError::ForeignRun`) and that no byte of
        // any of them was altered — so checking the first is checking
        // them all, and what is left to check is the name itself.
        if started.run_id.is_empty()
            || started.run_id.chars().count() > RUN_ID_MAX
            || !started.run_id.chars().all(adoptable)
        {
            return Err(ImportError::UnadoptableRunId(escaped(&started.run_id)));
        }
        let feature = state
            .feature
            .ok_or(ImportError::Unattested { field: "feature" })?;
        let run_manifest = started
            .payload
            .get("manifest")
            .ok_or(ImportError::Unattested { field: "manifest" })?;
        // `runs.bundle_name` is a denormalization of the manifest key,
        // and nothing ever selects the column — `Store::manifest` and
        // every reader of it go to `runs.manifest`, which carries the
        // chain's own manifest verbatim. So a manifest without the key
        // is copied as faithfully as it can be, not refused over a
        // column no readout would have shown.
        let bundle_name = run_manifest
            .get("bundle_name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        Ok(RunsRow {
            run_id: &started.run_id,
            feature,
            bundle_name,
            manifest: serde_json::to_string(run_manifest).map_err(StoreError::from)?,
            created_at: &started.recorded_at,
        })
    }
}

/// [`Store::import_run`]'s one transaction, as a body [`patiently`] may
/// run again: the row and every event, or nothing. An attempt that ends
/// busy committed nothing, and the next asks the collision afresh.
fn adopt_once(
    conn: &mut Connection,
    row: &RunsRow<'_>,
    arrival: &Arrival,
    events: &[EventEnvelope],
) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    // Gate 5 inside the transaction that would write, so a concurrent
    // adoption of the same run loses here rather than forking the
    // destination.
    refuse_existing(&tx, row.run_id)?;
    tx.execute(
        "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at,
                           imported_at, imported_from)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            row.run_id,
            row.feature,
            row.bundle_name,
            row.manifest,
            row.created_at,
            arrival.imported_at,
            arrival.imported_from,
        ],
    )?;
    for event in events {
        // Serialized the way `append_next` stores its own — the export
        // form is re-derived by `export_ndjson`, so a re-export of this
        // run reproduces the source bytes.
        tx.execute(
            INSERT_EVENT,
            params![
                row.run_id,
                event.seq as i64,
                event.event_hash,
                serde_json::to_string(event)?,
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

impl Store {
    /// Adopt an exported run into this journal byte-identically — the
    /// verb paired with [`Store::export_ndjson`]. Journals never merge,
    /// but one run can relocate.
    ///
    /// Nothing lands until every gate passes, in this order:
    ///
    /// 1. The export must not be a **redacted derivative**, refused by
    ///    its own markers before its content is read at all — the
    ///    `.redacted.` filename [`redact_export`](crate::redact_export) mandates, or a
    ///    `"redacted": true` sidecar manifest. Redaction rewrites
    ///    payload bytes and leaves the recorded hashes behind, so a
    ///    redacted export's chain can never verify; there is no import
    ///    of one that would not be an adoption of unverifiable content.
    /// 2. The **chain must verify whole**. One broken link refuses the
    ///    import — never a good prefix of it.
    /// 3. Every checkpoint and successful result must satisfy the frozen
    ///    **seat-record contract**. No malformed record is adopted as
    ///    evidence merely because its enclosing hash chain is intact.
    /// 4. The events must **fold**, and a `FoldError` refuses with the
    ///    same citation a quarantined run shows anywhere else.
    /// 5. The **run_id must be one this journal would carry**. A
    ///    verified chain proves its bytes were not altered; the hashes
    ///    are unkeyed, so it never proves the sealer was entitled to the
    ///    name. See [`adoptable`].
    /// 6. The destination must not already carry this **run_id**. The
    ///    run_id is hashed into every envelope, so a collision is
    ///    structurally not a rename-and-retry: it is the operator's to
    ///    rule on. A second import of the same export refuses here for
    ///    exactly the same reason a genuinely different run sharing a
    ///    run_id does — adoption is not idempotent, it is once.
    ///
    /// The adopted events are then written exactly as exported: same
    /// bytes, same hashes, same seqs, same `recorded_at`, same run_id.
    /// [`Store::append_next`] is the wrong primitive for that by
    /// design — it seals a *fresh* envelope from the destination's head
    /// — so this appends pre-sealed envelopes verbatim, all of them in
    /// one transaction or none of them.
    ///
    /// The `runs` row is derived from the **verified chain**: the run
    /// manifest rides inside `run/started`'s payload, so every column
    /// comes from bytes a hash covers. The sidecar manifest is consulted
    /// for one thing only — its redaction marker — where trusting an
    /// uncovered file can only ever cause a refusal. `created_at` is the
    /// first event's `recorded_at`, so the adopted run sorts into the
    /// fleet where it actually ran.
    ///
    /// Where the run arrived from and when is recorded in the `runs`
    /// table BESIDE the chain, never inside it. `brokkr_core::fold`
    /// cannot observe it, so `state = fold(events)` holds identically
    /// for a native run and an adopted one.
    pub fn import_run(
        &mut self,
        ndjson: &str,
        manifest: &Value,
        origin: &Path,
    ) -> Result<Adoption, ImportError> {
        let named = origin
            .file_name()
            .unwrap_or(origin.as_os_str())
            .to_string_lossy()
            .to_string();
        // Gate 1, twice — the two marks `export --redact` writes. The
        // filename catches the pair as published; the manifest flag
        // catches a copy somebody renamed back.
        if named.contains(".redacted.") {
            return Err(ImportError::Redacted(named));
        }
        if manifest.get("redacted") == Some(&Value::Bool(true)) {
            return Err(ImportError::Redacted(named));
        }

        // Gates 2 and 3: parse, chain, fold — the same three checks
        // `verify-run` makes offline, on the same bytes, before this
        // journal is touched.
        let (events, state) = verified_events(ndjson)?;
        let row = RunsRow::of(&events, state)?;
        let head_hash = events
            .last()
            .expect("a folded journal has a last event")
            .event_hash
            .clone();
        let arrival = Arrival {
            imported_at: now_rfc3339(),
            imported_from: origin.display().to_string(),
        };

        let conn = &mut self.conn;
        patiently("import_run", self.patience, || {
            adopt_once(conn, &row, &arrival, &events)
        })
        .map_err(|refused| match refused {
            StoreError::RunExists(run_id) => ImportError::Collision(run_id),
            refused => ImportError::Store(refused),
        })?;
        Ok(Adoption {
            run_id: row.run_id.to_string(),
            events: events.len(),
            head_hash,
            arrival,
        })
    }

    /// How a run got here: `None` for a run driven natively in this
    /// journal, `Some` for one adopted by [`Store::import_run`].
    /// Provenance is queryable without reading a single event, which is
    /// the whole point of recording it beside the chain.
    ///
    /// A journal opened read-only that predates the arrival columns has
    /// none to select; a read-write open migrates before anything asks.
    pub fn arrival(&self, run_id: &str) -> Result<Option<Arrival>, StoreError> {
        let row: Option<(Option<String>, Option<String>)> =
            patiently("arrival", self.patience, || {
                Ok(self
                    .conn
                    .query_row(
                        "SELECT imported_at, imported_from FROM runs WHERE run_id = ?1",
                        params![run_id],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()?)
            })?;
        let (imported_at, imported_from) =
            row.ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))?;
        Ok(match (imported_at, imported_from) {
            (Some(imported_at), Some(imported_from)) => Some(Arrival {
                imported_at,
                imported_from,
            }),
            _ => None,
        })
    }
}

/// Verify an exported NDJSON journal offline: parse, chain, seat record, fold.
pub fn verify_export(ndjson: &str) -> Result<brokkr_core::RunState, VerifyError> {
    verified_events(ndjson).map(|(_, state)| state)
}

/// The same four checks as [`verify_export`], handing back the
/// envelopes as well as the state they fold to. An adoption needs the
/// envelopes themselves — it writes their exact bytes — and must make
/// the checks on the very bytes it adopts, not on a second parse of
/// them.
pub fn verified_events(
    ndjson: &str,
) -> Result<(Vec<EventEnvelope>, brokkr_core::RunState), VerifyError> {
    let events = ndjson
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str::<EventEnvelope>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(VerifyError::Parse)?;
    verify_chain(&events).map_err(VerifyError::Chain)?;
    seat_record::validate_events(&events).map_err(VerifyError::SeatRecord)?;
    let state = brokkr_core::fold(&events).map_err(VerifyError::Fold)?;
    Ok((events, state))
}

#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("parse: {0}")]
    Parse(serde_json::Error),
    #[error("chain: {0}")]
    Chain(ChainError),
    #[error(transparent)]
    SeatRecord(SeatRecordError),
    #[error("fold: {0}")]
    Fold(brokkr_core::FoldError),
}
