//! Muninn's documents (#346): the dossier the seat reads and the line
//! the record keeps. Every struct declares its fields in byte order, the
//! order their keys printed in when each was a `json!` map.
//!
//! Only the record line's declaration order reaches bytes: it is written
//! from the struct itself. The dossier reaches the seat as a `Value`
//! (`oneshot::run_once` takes one), whose keys print sorted whatever a
//! struct declares, so its structs hold what each part writes and where
//! a key is left out.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use super::{Cited, CitedCrossing};
use crate::realms::{ConsumedView, Published};

/// The dossier the seat reads.
#[derive(Serialize)]
pub(crate) struct DossierDoc<'a> {
    // Written only by a world that draws one, so a world that never did
    // hands the seat the exact dossier it always handed it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub crossings: Vec<CrossingEntry<'a>>,
    pub dossier_version: u32,
    pub fleet: Fleet<'a>,
    pub generated_at: &'a str,
    pub residual_findings: Vec<Value>,
    pub runs: Vec<Row>,
}

/// The fleet's counts, the whole world's.
#[derive(Serialize)]
pub(crate) struct Fleet<'a> {
    pub awaiting_operator: u64,
    pub completed: u64,
    // A run whose journal does not fold has no status to count as; it
    // is counted as what it is.
    pub quarantined: u64,
    // The world's shape, stated only by a world that has one to state: a
    // one-hearth dossier is the dossier it always was, byte for byte.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub realms: Vec<&'a str>,
    pub running: u64,
    pub runs: usize,
    pub stopped: u64,
    // How many findings the operator has closed by name (decision 0047
    // ruling 4). Counted, not hidden: the summary says how many were left
    // out of the queue.
    pub superseded_findings: u64,
}

/// One run of the fleet: a run whose journal folded, or one that is
/// quarantined because it did not.
#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum Row {
    Quarantined(QuarantinedRow),
    Read(RunRow),
}

/// A run whose journal does not fold, listed as `?` with the refusal's
/// own words, admitting no command and in no phase.
#[derive(Serialize)]
pub(crate) struct QuarantinedRow {
    pub age: Option<String>,
    pub created_at: String,
    pub feature: String,
    pub fold_error: String,
    pub operator_commands: [String; 0],
    pub phase: Option<String>,
    // The realm a many-hearth world read the run in, and nothing in a
    // one-hearth world.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realm: Option<String>,
    pub run_id: String,
    pub seq: u64,
    pub status: &'static str,
}

/// A run whose journal folded, as the view models derive it.
#[derive(Serialize)]
pub(crate) struct RunRow {
    pub age: Option<String>,
    pub consecutive_failures: BTreeMap<String, u64>,
    pub cost_usd: Option<f64>,
    pub created_at: String,
    pub feature: String,
    pub last_ruling: Value,
    pub operator_commands: Vec<String>,
    pub park_reason: Option<String>,
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realm: Option<String>,
    pub run_id: String,
    pub seats: Vec<SeatRow>,
    pub seq: u64,
    pub status: String,
}

/// One seat of a [`RunRow`]: a panel or sequence seat's own row, never
/// its members'.
#[derive(Serialize)]
pub(crate) struct SeatRow {
    pub attempts: u64,
    pub cost_usd: Option<f64>,
    pub phase: Option<String>,
    pub seat: String,
    pub status: String,
}

/// One realm's crossings: both lists always written, empty included.
#[derive(Serialize)]
pub(crate) struct CrossingEntry<'a> {
    pub consumes: Vec<ConsumedView<'a>>,
    pub publishes: &'a [Published],
    pub realm: &'a str,
}

/// A moved pin, as a finding charged to the realm that consumes it.
#[derive(Serialize)]
pub(crate) struct CrossingFinding<'a> {
    pub crossing: &'a str,
    // A crossing is not a run: it has no run id and no sequence, so the
    // finding says what it is instead of borrowing a run's citation. The
    // marker is how a reader tells it from a run finding.
    pub input: &'static str,
    // The refusal `run` would give, verbatim, so the readout and the
    // loader cannot word one contract twice.
    pub line: &'a str,
    pub publisher: &'a str,
    pub realm: &'a str,
    pub value: &'static str,
}

/// One line of the record.
#[derive(Serialize)]
pub(crate) struct RecordLine<'a> {
    pub agent: Agent<'a>,
    pub citations: Vec<Cite<'a>>,
    // Written only by a report that cited one, so a run-only record
    // keeps exactly the shape it always had.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub crossing_citations: Vec<CrossingCite<'a>>,
    pub dossier: DossierPin<'a>,
    pub fleet_summary: &'a str,
    pub parked_runs: &'a [Parked],
    pub record_version: u32,
    pub recorded_at: &'a str,
    pub usage: Option<Usage<'a>>,
    pub work_queue: &'a [Queued],
}

/// The agent that proposed, and the deadline it ran under.
#[derive(Serialize)]
pub(crate) struct Agent<'a> {
    pub deadline_seconds: u64,
    pub model: &'a str,
    pub name: &'static str,
    pub provider: &'a str,
}

/// What the proposal was derived from: the dossier's fleet summary, and
/// its crossing report where it drew one.
#[derive(Serialize)]
pub(crate) struct DossierPin<'a> {
    // Written only by a dossier that drew one, so a record over a world
    // with no crossing keeps exactly the shape it always had.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crossings: Option<&'a Value>,
    pub dossier_version: u32,
    pub fleet: &'a Value,
    pub generated_at: &'a str,
}

/// The seat's own cost and usage, each read as the checkpoint states it.
#[derive(Serialize)]
pub(crate) struct Usage<'a> {
    pub cost_usd: Option<&'a Value>,
    pub session_id: Option<&'a Value>,
    pub turns: Option<&'a Value>,
}

/// A run citation, realm first when there is one. Absent for a
/// one-hearth world, so its record keeps exactly the shape it had.
#[derive(Serialize)]
pub(crate) struct Cite<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realm: Option<&'a str>,
    pub run_id: &'a str,
    pub seq: u64,
}

impl<'a> Cite<'a> {
    pub(crate) fn of((realm, run_id, seq): &'a Cited) -> Self {
        Cite {
            realm: realm.as_deref(),
            run_id,
            seq: *seq,
        }
    }
}

/// A crossing citation: it carries no run id and no sequence, and its
/// realm is always stated.
#[derive(Serialize)]
pub(crate) struct CrossingCite<'a> {
    pub crossing: &'a str,
    pub realm: &'a str,
}

impl<'a> CrossingCite<'a> {
    pub(crate) fn of((realm, crossing): &'a CitedCrossing) -> Self {
        CrossingCite { crossing, realm }
    }
}

/// A proposal for a parked run: the command to issue and why.
#[derive(Serialize)]
pub(crate) struct Parked {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realm: Option<String>,
    pub reasoning: String,
    pub run_id: String,
    pub seq: u64,
}

/// A queued finding: a crossing's, cited by realm and crossing, or a
/// run's, cited by run id and sequence.
#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum Queued {
    Crossing(QueuedCrossing),
    Run(QueuedRun),
}

/// A queued crossing finding.
#[derive(Serialize)]
pub(crate) struct QueuedCrossing {
    pub crossing: String,
    pub finding: String,
    pub realm: String,
    pub reasoning: String,
}

/// A queued run finding.
#[derive(Serialize)]
pub(crate) struct QueuedRun {
    pub finding: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realm: Option<String>,
    pub reasoning: String,
    pub run_id: String,
    pub seq: u64,
}
