//! `brokkr muninn` — the standing overseer (decision 0020, whose
//! evidence-source and citation rulings are amended for the world's
//! crossings by decision 0059).
//!
//! One invocation, three steps and no fourth: derive a fleet dossier
//! from the journal, hand it to one bounded seat, record what that seat
//! proposed. Nothing here executes a proposal, and nothing here can:
//!
//! - The store is opened READ-ONLY, so this path cannot append an event
//!   to any run's journal even by defect. The engine stays the single
//!   writer of run journals (ruling 3).
//! - The seat runs through `brokkr_protocol::oneshot`, which owns its own
//!   scratch directory and knows nothing about the journal. No
//!   repository tree is named in its input, so none can be reached by
//!   name (ruling 1).
//! - Its input carries no bound values and names no store of them:
//!   decision 0012's bindings are for seats that do work, and this seat
//!   does none.
//! - Proposals land in this module's own append-only file, beside the
//!   run journals and inside none of them (ruling 3). A report that does
//!   not validate is not recorded at all — an unverifiable proposal is
//!   worse than no proposal, because a later reader cannot tell it from
//!   a real reading.
//!
//! The command carries the name; the output does not. Everything printed
//! and everything recorded here is plain mechanic language (0019 law 4).

mod document;
mod record;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, Result};
use brokkr_protocol::oneshot::{self, OneShot};
use brokkr_runtime::realms::{Hearth, World};
use brokkr_runtime::{argv::Executable, bundle::expand_command};
use brokkr_runtime::{Adapters, Availability, Library};
use brokkr_store::Store;
use serde_json::{json, Value};

use crate::realms::{Consumed, ConsumedView, Pin, Published};
use crate::render::Safe;
use document::{
    Agent, Cite, CrossingCite, CrossingEntry, CrossingFinding, DossierDoc, DossierPin, Fleet,
    Parked, QuarantinedRow, Queued, QueuedCrossing, QueuedRun, RecordLine, Row, RunRow, SeatRow,
    Usage,
};

/// The agent definition this command invokes.
pub(crate) const AGENT: &str = "muninn";

/// The seat name the driver is started with. There is no phase here —
/// this seat belongs to no run — but the protocol names a seat, and this
/// is the honest name for it.
pub(crate) const SEAT: &str = "muninn";

/// The one result the seat is allowed to reach.
pub(crate) const PROPOSED: &str = "proposed";

/// Wire version of the dossier handed to the seat.
pub(crate) const DOSSIER_VERSION: u32 = 1;

/// Wire version of one line in the record.
pub(crate) const RECORD_VERSION: u32 = 1;

/// The record's name in the run-local state: beside the workspace
/// journal, and deliberately not inside it.
pub(crate) const RECORD: &str = "muninn.ndjson";

/// The one-line task the seat's prompt carries.
const TASK: &str = "Read the fleet dossier below and propose operator actions. \
                    Propose only: issue no command and start no run.";

/// What the whole fleet looks like to the seat, plus the two things a
/// report is judged against: the facts the dossier states, and the
/// operator commands each run admits.
#[derive(Debug)]
pub(crate) struct Dossier {
    /// The dossier as the seat receives it.
    pub value: Value,
    /// Every (realm, run id, sequence number) the dossier states — the
    /// closed set a proposal may cite. Decision 0007's provenance
    /// discipline, applied to advice instead of to a seat input, and
    /// carrying the realm since decision 0026 ruling 3: in a many-hearth
    /// world a fact belongs to the journal it was read from, and a
    /// finding that cannot name that journal cannot be followed back.
    /// The realm is absent exactly when the world has one hearth, which
    /// is every world that never drew a second journal.
    pub facts: Vec<(Option<String>, String, u64)>,
    /// Every `(realm, crossing)` the dossier states as a FINDING — the
    /// closed set a proposal may cite for a contract, beside the closed
    /// set it may cite for a run.
    ///
    /// A crossing is a fact about the MAP, not about any run: it has no
    /// `run_id` and no sequence number to carry, and a realm the map names
    /// before its first run — which `muninn run` already reads and
    /// reports on — has no run whose head one could be borrowed from. So
    /// it carries the citation it honestly has, the realm whose run would
    /// refuse and the crossing that would refuse it, rather than a run
    /// citation a reader would follow to a ruling that never mentioned a
    /// contract (decision 0057; decision 0020 ruling 3, as amended by
    /// decision 0059; decision 0007's provenance discipline).
    pub crossings: Vec<CitedCrossing>,
    /// Per run, the operator commands `brokkr-view` derives as legal.
    /// Keyed by run id alone, as the report cites it: a run id is unique
    /// within the journal it lives in, and where two hearths hold one id
    /// the hearth the map names first answers for it — the same rule
    /// [`Dossier::realm_of`] follows, so the two never disagree.
    pub commands: BTreeMap<String, Vec<String>>,
}

impl Dossier {
    /// The realm this fact was read in, or `None` if the dossier states
    /// no such fact at all. Two `Option`s, and they mean different
    /// things: the outer is "is this stated?", the inner is "under which
    /// realm?" — absent in a one-hearth world.
    fn realm_of(&self, run_id: &str, seq: u64) -> Option<Option<&str>> {
        self.facts
            .iter()
            .find(|(_, known, at)| known == run_id && *at == seq)
            .map(|(realm, _, _)| realm.as_deref())
    }

    /// Whether the dossier states a fact at all, whatever realm it was
    /// read in. The validator asks [`Dossier::realm_of`], which answers
    /// both questions at once; this is how the closed set itself is
    /// asserted.
    #[cfg(test)]
    fn states(&self, run_id: &str, seq: u64) -> bool {
        self.realm_of(run_id, seq).is_some()
    }

    /// Whether the dossier states this crossing as a finding of this
    /// realm. The crossing half of [`Dossier::realm_of`]'s question, and
    /// the only thing a crossing citation is checked against.
    fn states_crossing(&self, realm: &str, crossing: &str) -> bool {
        self.crossings
            .iter()
            .any(|(known, named)| known == realm && named == crossing)
    }

    fn admits(&self, run_id: &str, command: &str) -> bool {
        match self.commands.get(run_id) {
            Some(commands) => commands.iter().any(|known| known == command),
            None => false,
        }
    }
}

/// One realm's crossings, as the dossier states them (decision 0057;
/// decision 0026 ruling 3, which makes every fact name the realm it was
/// read for). Built only for a realm that draws a crossing at all, so a
/// world that never drew one hands the seat the exact dossier it always
/// handed it.
///
/// The entries ARE the shared readout entries `brokkr realms` prints —
/// the same [`crate::realms::crossings_of`] derivation over the one
/// `World::crossings_report` — so the two surfaces cannot state a
/// crossing two ways. A publication keeps its declared
/// repository-relative path; a consumption keeps its publishing realm
/// and its pin's state.
///
/// The realm here is the realm the MAP names, never a hearth label: a
/// crossing is drawn between realms and is nothing to do with which
/// journal a realm's runs land in, so it is named for what it is even in
/// a one-hearth world, where the journal-derived facts beside it name no
/// realm at all.
pub(crate) struct RealmCrossings {
    pub realm: String,
    /// What this realm publishes, in map order — the identity a reader
    /// needs, and never a bare count.
    pub publishes: Vec<Published>,
    /// What this realm consumes, in map order, each entry naming the
    /// publishing realm and carrying its pin's state: `matching`, `moved`
    /// (with the refusal's own words) or `unchecked`.
    pub consumes: Vec<Consumed>,
}

/// A crossing citation: the realm that CONSUMES the crossing, and the
/// crossing's name.
type CitedCrossing = (String, String);

/// Every realm of this world that draws a crossing, read off the same
/// report `brokkr realms` and `brokkr doctor` read, through the same
/// derivation (`crate::realms::crossings_of`). Nothing is resolved,
/// hashed or compared here: a second computation is a second answer
/// waiting to disagree with the one the loader already gave.
///
/// A world with no map at all draws none, which is every workspace that
/// never wrote a `realms.json`.
pub(crate) fn world_crossings(world: Option<&World>) -> Vec<RealmCrossings> {
    let Some(world) = world else {
        return Vec::new();
    };
    let mut crossings = Vec::new();
    for realm in &world.map.realms {
        let (publishes, consumes) = crate::realms::crossings_of(world.crossings_report(), realm);
        if publishes.is_empty() && consumes.is_empty() {
            continue;
        }
        crossings.push(RealmCrossings {
            realm: realm.name.clone(),
            publishes,
            consumes,
        });
    }
    crossings
}

/// One hearth this command reads: the journal, and the realm its runs
/// belong to when the world holds more than one (decision 0026 ruling
/// 3). A one-hearth world names no realm, and its dossier is exactly the
/// dossier it always was.
pub(crate) struct Source<'a> {
    pub realm: Option<&'a str>,
    pub store: &'a Store,
}

/// The fleet, derived from the same view models every read surface
/// consumes (decision 0013). Nothing is re-read from the journal by hand
/// here: a second derivation is a second answer waiting to disagree.
///
/// Several hearths are read in sequence and stated side by side, each
/// row and each finding carrying the realm it came from. Nothing folds
/// across a journal boundary and nothing is written (ruling 5): the
/// stores arrive already opened read-only.
///
/// `crossings` is the world's already-computed crossing report
/// ([`world_crossings`] over `World::crossings_report`), passed in rather
/// than resolved here: a second resolution would hash the publisher's
/// bytes again and could disagree with the one `World::load` stands on.
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
pub(crate) fn dossier_of(
    sources: &[Source],
    crossings: &[RealmCrossings],
    now: &str,
) -> Result<Dossier> {
    let mut rows: Vec<Row> = Vec::new();
    let mut findings: Vec<Value> = Vec::new();
    let mut facts: Vec<(Option<String>, String, u64)> = Vec::new();
    let mut commands: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    // The rulings the supersede annotations cite, gathered while the
    // runs are read and admitted to the closed set afterwards: a
    // superseding run may be read later in the fleet than the run whose
    // finding it closed.
    let mut superseded: Vec<(Option<String>, String, u64)> = Vec::new();
    for source in sources {
        let realm = source.realm.map(str::to_string);
        let store = source.store;
        let listed = crate::fleet::read_hearth(store)
            .listed()
            .map_err(anyhow::Error::msg)?;
        for crate::fleet::ListedRun {
            run_id,
            feature,
            created_at,
            events,
            state,
            residuals,
        } in listed
        {
            let state = match state {
                Ok(state) => state,
                Err(crate::fleet::Quarantine { detail, seq, .. }) => {
                    // One corrupt journal must not blind the aide to the
                    // fleet. The run is quarantined — listed as `?` with the
                    // refusal's own words — and raised as a finding, because
                    // a journal that will not load or fold is exactly what
                    // the operator needs surfaced, not hidden. Its citation
                    // is the sequence the fold refused at, so a proposal
                    // naming it validates.
                    let finding = brokkr_view::quarantine_finding(&run_id, seq, &detail);
                    facts.push((realm.clone(), run_id.clone(), seq));
                    findings.push(keyed(&realm, serde_json::to_value(&finding)?));
                    commands.entry(run_id.clone()).or_default();
                    *counts.entry("quarantined".to_string()).or_default() += 1;
                    rows.push(Row::Quarantined(QuarantinedRow {
                        age: brokkr_view::age(&created_at, now),
                        created_at,
                        feature,
                        fold_error: detail,
                        operator_commands: [],
                        phase: None,
                        realm: realm.clone(),
                        run_id,
                        seq,
                        status: "?",
                    }));
                    continue;
                }
            };
            let view = brokkr_view::run_view(&events, Some(&state));
            let summary = view
                .summary
                .as_ref()
                .expect("a folded state always summarizes");
            let admits = brokkr_view::operator_commands(&state);
            *counts.entry(summary.status_word().to_string()).or_default() += 1;
            facts.push((realm.clone(), run_id.clone(), summary.seq));
            // First hearth wins, matching [`Dossier::realm_of`]: where
            // two journals hold one run id, one answer is stated for it
            // and it is the same one on both sides.
            commands.entry(run_id.clone()).or_insert(admits.clone());
            for finding in residuals {
                facts.push((realm.clone(), run_id.clone(), finding.seq));
                // The annotation is a citable fact in its own right
                // (decision 0047 ruling 4), so a report may say "the
                // operator closed this at seq N" and have that checked.
                // The superseding run's own ruling is a candidate to
                // become one, keyed exactly as the annotation keyed it —
                // never widened to the annotated run's realm, which
                // would be a guess about which hearth was meant.
                if let Some(mark) = &finding.superseded {
                    facts.push((realm.clone(), run_id.clone(), mark.seq));
                    superseded.push((mark.by.realm.clone(), mark.by.run_id.clone(), mark.by.seq));
                    *counts.entry("superseded_findings".to_string()).or_default() += 1;
                }
                findings.push(keyed(&realm, serde_json::to_value(&finding)?));
            }
            let (seats, cost_usd) = seats_of(&view);
            rows.push(Row::Read(RunRow {
                age: brokkr_view::age(&created_at, now),
                consecutive_failures: summary.consecutive_failures.clone(),
                cost_usd,
                created_at,
                feature,
                last_ruling: serde_json::to_value(&view.ruling)?,
                operator_commands: admits,
                park_reason: summary.park_reason.clone(),
                phase: summary.phase.clone(),
                realm: realm.clone(),
                run_id,
                seats,
                seq: summary.seq,
                status: summary.status_word().to_string(),
            }));
        }
    }
    // A superseding run's ruling is stated only when this reading
    // covers the journal it lives in (decision 0047 ruling 4): every
    // run read has already put its own head in `facts`, so that is the
    // question asked here.
    let covered: Vec<(Option<String>, String, u64)> = superseded
        .into_iter()
        .filter(|(realm, run_id, _)| {
            facts
                .iter()
                .any(|(seen, known, _)| (seen, known) == (realm, run_id))
        })
        .collect();
    facts.extend(covered);
    facts.sort();
    facts.dedup();
    let count = |status: &str| counts.get(status).copied().unwrap_or(0);
    let fleet = Fleet {
        awaiting_operator: count("awaiting_operator"),
        completed: count("completed"),
        quarantined: count("quarantined"),
        realms: sources.iter().filter_map(|source| source.realm).collect(),
        running: count("running"),
        runs: rows.len(),
        stopped: count("stopped"),
        superseded_findings: count("superseded_findings"),
    };
    let (crossing_entries, cited_crossings) = crossing_section(crossings, &mut findings);
    let value = json!(DossierDoc {
        crossings: crossing_entries,
        dossier_version: DOSSIER_VERSION,
        fleet,
        generated_at: now,
        residual_findings: findings,
        runs: rows,
    });
    Ok(Dossier {
        value,
        facts,
        crossings: cited_crossings,
        commands,
    })
}

/// A run's seats and what they cost together: `None` where no seat
/// reported a cost, because no telemetry is no claim, not a zero. A panel
/// or sequence seat's row already aggregates its members' cost, so
/// counting members again would bill the run twice.
fn seats_of(view: &brokkr_view::RunView) -> (Vec<SeatRow>, Option<f64>) {
    let seats: Vec<&brokkr_view::Participant> = view
        .participants
        .iter()
        .filter(|part| part.member.is_none())
        .collect();
    let costs: Vec<f64> = seats.iter().filter_map(|part| part.cost).collect();
    let cost = match costs.is_empty() {
        true => None,
        false => Some(costs.iter().sum::<f64>()),
    };
    let rows = seats
        .iter()
        .map(|part| SeatRow {
            attempts: part.attempts,
            cost_usd: part.cost,
            phase: part.phase.clone(),
            seat: part.label.clone(),
            status: part.status.clone(),
        })
        .collect();
    (rows, cost)
}

/// Decision 0057 read into the dossier, on the terms decision 0020
/// ruling 1's amendment (0059) admits: what each mapped realm publishes
/// and consumes, entry for entry, and — when a consumed pin no longer
/// matches — a FINDING charged to the CONSUMING realm, because that is
/// the realm whose next run `World::load` would refuse. A matching pin is
/// stated in the per-realm entries and is not a finding; an unchecked pin
/// is stated beside it and is nobody's fault here (the publisher's line
/// carries it, `ca0c765`). The entries are the shared readout entries
/// `brokkr realms` prints, through the one [`ConsumedView::of`], so the
/// dossier and that readout render one derivation.
fn crossing_section<'a>(
    crossings: &'a [RealmCrossings],
    findings: &mut Vec<Value>,
) -> (Vec<CrossingEntry<'a>>, Vec<CitedCrossing>) {
    let mut entries = Vec::new();
    let mut cited = Vec::new();
    for realm_crossings in crossings {
        if realm_crossings.publishes.is_empty() && realm_crossings.consumes.is_empty() {
            continue;
        }
        entries.push(CrossingEntry {
            consumes: realm_crossings
                .consumes
                .iter()
                .map(ConsumedView::of)
                .collect(),
            publishes: &realm_crossings.publishes,
            realm: &realm_crossings.realm,
        });
        for consumed in &realm_crossings.consumes {
            let Pin::Moved(line) = &consumed.pin else {
                continue;
            };
            cited.push((realm_crossings.realm.clone(), consumed.name.clone()));
            findings.push(json!(CrossingFinding {
                crossing: &consumed.name,
                input: "crossing_pins",
                line,
                publisher: &consumed.publisher,
                realm: &realm_crossings.realm,
                value: "moved",
            }));
        }
    }
    (entries, cited)
}

/// A dossier fact, keyed by the realm it was read in — and left exactly
/// as it was when there is no realm to key it by, which is every
/// one-hearth world.
fn keyed(realm: &Option<String>, value: Value) -> Value {
    match (realm, value) {
        (Some(realm), Value::Object(mut fields)) => {
            fields.insert("realm".to_string(), json!(realm));
            Value::Object(fields)
        }
        (_, value) => value,
    }
}

// -------------------------------------------------------- the seat

/// One resolved invocation of the overseer: the command, the charter it
/// reads, the deadline it runs under, and the model actually serving it.
#[derive(Debug)]
pub(crate) struct Seat {
    pub command: Vec<String>,
    pub charter: PathBuf,
    pub deadline: Duration,
    pub model: String,
    pub provider: String,
}

/// Resolve the agent through decision 0016's library, and refuse a
/// definition that gives this seat a retry ladder: ruling 4 says one
/// invocation produces its report or nothing, and `max_attempts` is
/// where that would quietly stop being true.
pub(crate) fn seat(agents_dir: &Path, adapters_dir: &Path) -> Result<Seat> {
    let library = Library::load(agents_dir)?;
    let adapters = Adapters::load(adapters_dir)?;
    let resolved =
        brokkr_runtime::resolve_agent(&library, &adapters, &Availability::unspecified(), AGENT)
            .map_err(|error| anyhow::anyhow!("{error}"))?;
    let limits = resolved.limits.unwrap_or_default();
    anyhow::ensure!(
        limits.max_attempts == 1,
        "agent '{AGENT}' declares max_attempts {}; this seat runs once per \
         invocation and has no retry ladder of its own",
        limits.max_attempts
    );
    let candidate = resolved
        .candidates
        .first()
        .expect("resolution yields at least one candidate or refuses");
    Ok(Seat {
        command: expand_command(&Executable::current(), agents_dir, &candidate.argv),
        charter: resolved.charter,
        deadline: Duration::from_secs(limits.timeout_seconds),
        model: candidate.model.clone(),
        provider: candidate.provider.clone(),
    })
}

/// The seat's input. `workdir` is the scratch directory and nothing
/// else, and there is no `secrets` or `secrets_file` key for a resolver
/// to act on — both absences are the guarantee, not an oversight.
fn seat_input(seat: &Seat, dossier: &Dossier, scratch: &Path) -> Value {
    json!({
        "role_path": seat.charter.to_string_lossy(),
        "feature": TASK,
        "phase": SEAT,
        "workdir": scratch.to_string_lossy(),
        "result_path": scratch.join("report.json").to_string_lossy(),
        "allowed_results": [PROPOSED],
        "context": dossier.value,
    })
}

// ------------------------------------------------------ the report

/// A validated report. The fields are the three proposal kinds v1
/// carries, plus the citations every entry in them stated.
pub(crate) struct Report {
    pub fleet_summary: String,
    pub parked_runs: Vec<Parked>,
    pub work_queue: Vec<Queued>,
    pub citations: Vec<Cited>,
    /// The `(consuming realm, crossing)` pairs a report cited — the
    /// crossing half of [`Report::citations`], kept apart because a
    /// crossing is not a run and carries no run id or sequence to cite.
    pub crossing_citations: Vec<CitedCrossing>,
}

fn string_field(what: &str, object: &Value, key: &str) -> Result<String, String> {
    match object.get(key).and_then(Value::as_str) {
        Some(text) if !text.is_empty() => Ok(text.to_string()),
        _ => Err(format!("{what} is missing a non-empty '{key}'")),
    }
}

fn array_field<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("the report is missing the '{key}' array"))
}

/// The citation check: a proposal may only name a run and a sequence
/// number the dossier actually stated. A reader who cannot follow a
/// citation back to a journal fact cannot tell advice from invention.
/// The realm comes back with the citation rather than being asked for:
/// in a many-hearth world every recorded proposal names the journal its
/// fact was read in (decision 0026 ruling 3), and a report that names a
/// realm the dossier does not state for that fact is refused rather than
/// quietly corrected.
type Cited = (Option<String>, String, u64);

fn citation(what: &str, dossier: &Dossier, entry: &Value) -> Result<Cited, String> {
    let run_id = string_field(what, entry, "run_id")?;
    let seq = entry
        .get("seq")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{what} is missing a numeric 'seq'"))?;
    let Some(realm) = dossier.realm_of(&run_id, seq) else {
        return Err(format!(
            "{what} cites {run_id} seq {seq}, which the dossier does not state"
        ));
    };
    if let Some(claimed) = entry.get("realm").and_then(Value::as_str) {
        if Some(claimed) != realm {
            return Err(format!(
                "{what} cites {run_id} seq {seq} in realm '{claimed}'; the dossier states \
                 that fact in {}",
                match realm {
                    Some(realm) => format!("realm '{realm}'"),
                    None => "no realm".to_string(),
                }
            ));
        }
    }
    Ok((realm.map(str::to_string), run_id, seq))
}

/// A crossing citation: a proposal about a contract names the realm that
/// CONSUMES the crossing and the crossing itself. Both are required, and
/// the pair must be one the dossier actually states as a finding — a
/// crossing that is matching, or a realm the dossier did not charge, is
/// not a fact a reader could follow back (decision 0057; decision 0007's
/// provenance discipline). A crossing finding has no run id and no
/// sequence, so this is its own citation shape and never a borrowed run
/// citation (decision 0026 ruling 3's realm key, unchanged).
fn crossing_citation(
    what: &str,
    dossier: &Dossier,
    entry: &Value,
) -> Result<CitedCrossing, String> {
    let realm = string_field(what, entry, "realm")?;
    let crossing = string_field(what, entry, "crossing")?;
    if !dossier.states_crossing(&realm, &crossing) {
        return Err(format!(
            "{what} cites crossing '{crossing}' in realm '{realm}', which the dossier \
             does not state as a finding"
        ));
    }
    Ok((realm, crossing))
}

/// One parked-run proposal: a stated fact, and an operator command the
/// dossier states that run admits.
fn parked(dossier: &Dossier, entry: &Value) -> Result<(Cited, Parked), String> {
    let what = "a parked-run proposal";
    let (realm, run_id, seq) = citation(what, dossier, entry)?;
    let command = string_field(what, entry, "command")?;
    if !dossier.admits(&run_id, &command) {
        return Err(format!(
            "{what} suggests '{command}' for {run_id}, which is not an operator \
             command the dossier states for it"
        ));
    }
    let reasoning = string_field(what, entry, "reasoning")?;
    let proposal = Parked {
        command,
        realm: realm.clone(),
        reasoning,
        run_id: run_id.clone(),
        seq,
    };
    Ok(((realm, run_id, seq), proposal))
}

/// One work-queue entry about a run fact, cited by run id and sequence.
fn queued_run(dossier: &Dossier, entry: &Value) -> Result<(Cited, Queued), String> {
    let what = "a work-queue entry";
    let (realm, run_id, seq) = citation(what, dossier, entry)?;
    let finding = string_field(what, entry, "finding")?;
    let reasoning = string_field(what, entry, "reasoning")?;
    let queued = QueuedRun {
        finding,
        realm: realm.clone(),
        reasoning,
        run_id: run_id.clone(),
        seq,
    };
    Ok(((realm, run_id, seq), Queued::Run(queued)))
}

/// One work-queue entry about a crossing finding, cited by realm and
/// crossing.
fn queued_crossing(dossier: &Dossier, entry: &Value) -> Result<(CitedCrossing, Queued), String> {
    let what = "a work-queue entry";
    let (realm, crossing) = crossing_citation(what, dossier, entry)?;
    let finding = string_field(what, entry, "finding")?;
    let reasoning = string_field(what, entry, "reasoning")?;
    let queued = QueuedCrossing {
        crossing: crossing.clone(),
        finding,
        realm: realm.clone(),
        reasoning,
    };
    Ok(((realm, crossing), Queued::Crossing(queued)))
}

/// Read the seat's result into a report, or say exactly what is wrong
/// with it. Nothing is repaired and nothing is partially accepted
/// (decision 0001): a report with one bad entry is a bad report.
pub(crate) fn validate(dossier: &Dossier, result: &Value) -> Result<Report, String> {
    let reached = result.get("result").and_then(Value::as_str);
    if reached != Some(PROPOSED) {
        return Err(format!(
            "the seat reached '{}', not '{PROPOSED}'",
            reached.unwrap_or(brokkr_view::ABSENT)
        ));
    }
    let inputs = result
        .get("inputs")
        .ok_or_else(|| "the report carries no 'inputs' object".to_string())?;
    let fleet_summary = string_field("the report", inputs, "fleet_summary")?;
    let mut citations: Vec<Cited> = Vec::new();
    let mut parked_runs = Vec::new();
    for entry in array_field(inputs, "parked_runs")? {
        let (cited, proposal) = parked(dossier, entry)?;
        citations.push(cited);
        parked_runs.push(proposal);
    }
    let mut work_queue = Vec::new();
    let mut crossing_citations: Vec<CitedCrossing> = Vec::new();
    for entry in array_field(inputs, "work_queue")? {
        // A crossing finding is cited by realm and crossing; every other
        // entry is a run fact cited by run id and sequence. The presence
        // of `crossing` is the discriminator, so a report that names both
        // a crossing and a run is read as a crossing and never silently
        // as a run.
        if entry.get("crossing").is_some() {
            let (cited, queued) = queued_crossing(dossier, entry)?;
            crossing_citations.push(cited);
            work_queue.push(queued);
            continue;
        }
        let (cited, queued) = queued_run(dossier, entry)?;
        citations.push(cited);
        work_queue.push(queued);
    }
    citations.sort();
    citations.dedup();
    crossing_citations.sort();
    crossing_citations.dedup();
    Ok(Report {
        fleet_summary,
        parked_runs,
        work_queue,
        citations,
        crossing_citations,
    })
}

/// The seat's own cost and usage, read from the session checkpoint its
/// driver reports. A driver that reports none leaves this null: the
/// record says nothing rather than claiming zero.
fn usage(checkpoints: &[Value]) -> Option<Usage<'_>> {
    let session = checkpoints.iter().rev().find(|checkpoint| {
        checkpoint
            .get("step")
            .and_then(Value::as_str)
            .is_some_and(|step| step.ends_with("-session-finished"))
    })?;
    Some(Usage {
        cost_usd: session.get("total_cost_usd"),
        session_id: session.get("session_id"),
        turns: session.get("num_turns"),
    })
}

/// One line of the record: what was proposed, what it was derived from,
/// when, and what the seat cost.
///
/// The dossier's `fleet` summary and its crossing report are both
/// snapshotted here. The fleet line has always named the runs the
/// proposal stood on; the crossing entries are snapshotted beside it for
/// the same reason (decision 0020 ruling 3, as amended by decision 0059):
/// a crossing names mutable map and file state, so a citation alone would
/// not survive the next `realms.json` edit. The append-only record must
/// still say, later, what the pin was when the proposal was derived — and
/// it now does, repository-relative paths and all.
fn entry<'a>(
    now: &'a str,
    seat: &'a Seat,
    dossier: &'a Dossier,
    report: &'a Report,
    usage: Option<Usage<'a>>,
) -> RecordLine<'a> {
    RecordLine {
        agent: Agent {
            deadline_seconds: seat.deadline.as_secs(),
            model: &seat.model,
            name: AGENT,
            provider: &seat.provider,
        },
        citations: report.citations.iter().map(Cite::of).collect(),
        crossing_citations: report
            .crossing_citations
            .iter()
            .map(CrossingCite::of)
            .collect(),
        dossier: DossierPin {
            crossings: dossier.value.get("crossings"),
            dossier_version: DOSSIER_VERSION,
            fleet: &dossier.value["fleet"],
            generated_at: now,
        },
        fleet_summary: &report.fleet_summary,
        parked_runs: &report.parked_runs,
        record_version: RECORD_VERSION,
        recorded_at: now,
        usage,
        work_queue: &report.work_queue,
    }
}

// ------------------------------------------------------- rendering

/// Every string a reader sees here was written by the seat, so it gets
/// the same treatment every other journal-derived string reaching a
/// terminal gets: `Safe` strips the control and reordering characters
/// that would otherwise let a report overwrite the line above it or
/// reverse the ruling it just cited.
fn text(value: &Value, key: &str) -> String {
    Safe::new(
        value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or(brokkr_view::ABSENT),
    )
    .as_str()
    .to_string()
}

fn number(value: &Value, key: &str) -> String {
    match value.get(key).and_then(Value::as_u64) {
        Some(number) => number.to_string(),
        None => brokkr_view::ABSENT.to_string(),
    }
}

/// A cited run, named the way the record cites it: `realm/run-id` where
/// the world had several hearths to read, the bare run id where it had
/// one. A reader must be able to follow a proposal back to the journal
/// it was read in (decision 0026 ruling 3).
fn cited_run(value: &Value) -> String {
    match value.get("realm").and_then(Value::as_str) {
        Some(realm) => format!("{}/{}", Safe::new(realm).as_str(), text(value, "run_id")),
        None => text(value, "run_id"),
    }
}

/// A cited crossing, named the way the record cites it: `realm/crossing`
/// where the realm is the one that CONSUMES it. `None` for a value that
/// carries no crossing, which is every run fact — the one discriminator
/// the renderer and the validator share.
fn cited_crossing(value: &Value) -> Option<String> {
    let crossing = value.get("crossing").and_then(Value::as_str)?;
    match value.get("realm").and_then(Value::as_str) {
        Some(realm) => Some(format!(
            "{}/{}",
            Safe::new(realm).as_str(),
            Safe::new(crossing).as_str()
        )),
        None => Some(Safe::new(crossing).as_str().to_string()),
    }
}

fn rows<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// One recorded invocation, as a reader sees it: what was proposed, and
/// the run id and sequence number behind every line of it.
fn render(entry: &Value) -> String {
    let parked = rows(entry, "parked_runs");
    let queued = rows(entry, "work_queue");
    let mut out = format!(
        "{} · {} proposals for parked runs · {} findings queued\n",
        text(entry, "recorded_at"),
        parked.len(),
        queued.len()
    );
    out.push_str(&format!("  summary: {}\n", text(entry, "fleet_summary")));
    for proposal in parked {
        out.push_str(&format!(
            "  parked {} seq {} · suggest '{}' · {}\n",
            cited_run(proposal),
            number(proposal, "seq"),
            text(proposal, "command"),
            text(proposal, "reasoning")
        ));
    }
    for item in queued {
        match cited_crossing(item) {
            Some(crossing) => out.push_str(&format!(
                "  queue {} · {} · {}\n",
                crossing,
                text(item, "finding"),
                text(item, "reasoning")
            )),
            None => out.push_str(&format!(
                "  queue {} seq {} · {} · {}\n",
                cited_run(item),
                number(item, "seq"),
                text(item, "finding"),
                text(item, "reasoning")
            )),
        }
    }
    let mut cited: Vec<String> = rows(entry, "citations")
        .iter()
        .map(|fact| format!("{} seq {}", cited_run(fact), number(fact, "seq")))
        .collect();
    cited.extend(
        rows(entry, "crossing_citations")
            .iter()
            .filter_map(cited_crossing),
    );
    out.push_str(&format!("  cites: {}\n", cited.join(", ")));
    out
}

// -------------------------------------------------------- commands

/// `brokkr muninn run`. A refused invocation and an unusable report both
/// record nothing, print one plain line, and exit nonzero: the record is
/// evidence, and evidence nobody can check is not evidence.
pub(crate) fn run(
    hearths: &[Hearth],
    world: Option<&World>,
    agents_dir: &Path,
    adapters_dir: &Path,
    record_path: &Path,
    now: &str,
) -> Result<ExitCode> {
    // Every hearth the map names is read (decision 0026 ruling 3), and
    // each of them READ-ONLY: this path cannot append to any journal it
    // reads, however many it reads.
    //
    // A one-hearth world names no realm, and its dossier, its report and
    // its record are exactly what they were before many hearths existed
    // — including its refusal: the one journal it was pointed at must
    // open, or there is no dossier to write about. A MANY-hearth world
    // survives a realm whose journal is not there yet, the way `brokkr
    // runs` already does: that hearth states nothing, its absence is
    // said out loud rather than silently, and the rest of the world is
    // still read. A realm mapped before its first run must not withhold
    // the whole world's dossier.
    //
    // The world's crossings are computed FIRST, before any journal opens:
    // a map supplies crossing facts even in a world where no run exists
    // yet, and a fresh two-realm map must not be refused for the journals
    // it has not written. Only a world with neither a readable journal
    // nor a crossing to report has nothing to say.
    let sole = hearths.len() < 2;
    let crossings = world_crossings(world);
    let mut stores: Vec<Store> = Vec::new();
    let mut labels: Vec<Option<String>> = Vec::new();
    for hearth in hearths {
        let opened = Store::open_read_only(&hearth.journal)
            .with_context(|| format!("opening {} for reading", hearth.journal.display()));
        match opened {
            Ok(store) => {
                stores.push(store);
                labels.push(match sole {
                    true => None,
                    false => Some(hearth.label()),
                });
            }
            Err(error) => {
                // The one-hearth refusal is kept for a world whose map
                // draws no crossing: the single journal it was pointed at
                // must open. When the map DOES draw crossings, the
                // crossing report is a dossier in its own right and the
                // absent journal is said out loud below.
                if sole && crossings.is_empty() {
                    return Err(error);
                }
                eprintln!(
                    "realm {} states nothing: {}",
                    Safe::new(&hearth.label()).as_str(),
                    Safe::new(&error.to_string()).as_str()
                );
            }
        }
    }
    // Degrading per hearth is not the same as reporting on nothing: a
    // world where NOT ONE journal opened and the map draws no crossing
    // has no dossier to write about, and saying so beats sending an aide
    // to read an empty world. A map that DOES draw a crossing yields a
    // dossier with no journal at all.
    anyhow::ensure!(
        !stores.is_empty() || !crossings.is_empty(),
        "no journal in this world could be read; there is nothing to report on"
    );
    let sources: Vec<Source> = stores
        .iter()
        .zip(&labels)
        .map(|(store, realm)| Source {
            realm: realm.as_deref(),
            store,
        })
        .collect();
    let dossier = dossier_of(&sources, &crossings, now)?;
    let seat = seat(agents_dir, adapters_dir)?;
    let outcome = oneshot::run_once(&seat.command, SEAT, seat.deadline, |scratch| {
        seat_input(&seat, &dossier, scratch)
    });
    let (result, checkpoints) = match outcome {
        // The reason carries the driver's own words, so it is sanitized
        // like anything else that reaches a terminal from outside.
        OneShot::Refused { reason } => {
            eprintln!(
                "muninn produced no report and recorded nothing: {}",
                Safe::new(&reason).as_str()
            );
            return Ok(crate::Exit::Failed.into());
        }
        OneShot::Produced {
            result,
            checkpoints,
        } => (result, checkpoints),
    };
    let report = match validate(&dossier, &result) {
        Ok(report) => report,
        Err(problem) => {
            eprintln!(
                "muninn's report was not usable and was not recorded: {}",
                Safe::new(&problem).as_str()
            );
            return Ok(crate::Exit::Failed.into());
        }
    };
    let line = entry(now, &seat, &dossier, &report, usage(&checkpoints));
    record::append(record_path, &line)?;
    print!("{}", render(&json!(line)));
    eprintln!(
        "recorded in {}; nothing was executed — issue any command yourself",
        record_path.display()
    );
    Ok(crate::Exit::Completed.into())
}

/// `brokkr muninn list`. Reads the record back, citations included.
pub(crate) fn list(record_path: &Path, json: bool) -> Result<()> {
    let entries = record::read(record_path)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }
    if entries.is_empty() {
        println!("no proposals recorded in {}", record_path.display());
        return Ok(());
    }
    for entry in &entries {
        print!("{}", render(entry));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
