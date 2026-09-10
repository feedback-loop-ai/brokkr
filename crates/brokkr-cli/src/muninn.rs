//! `brokkr muninn` — the standing overseer (decision 0020).
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

mod record;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, Result};
use brokkr_core::fold::fold;
use brokkr_protocol::oneshot::{self, OneShot};
use brokkr_runtime::bundle::expand_command;
use brokkr_runtime::realms::{Hearth, World};
use brokkr_runtime::{Adapters, Availability, Library};
use brokkr_store::Store;
use serde_json::{json, Value};

use crate::realms::Pin;
use crate::render::Safe;

/// The agent definition this command invokes.
pub const AGENT: &str = "muninn";

/// The seat name the driver is started with. There is no phase here —
/// this seat belongs to no run — but the protocol names a seat, and this
/// is the honest name for it.
pub const SEAT: &str = "muninn";

/// The one result the seat is allowed to reach.
pub const PROPOSED: &str = "proposed";

/// Wire version of the dossier handed to the seat.
pub const DOSSIER_VERSION: u32 = 1;

/// Wire version of one line in the record.
pub const RECORD_VERSION: u32 = 1;

/// The record's default location: beside the workspace journal, and
/// deliberately not inside it.
pub const DEFAULT_RECORD: &str = ".forge/muninn.ndjson";

/// The one-line task the seat's prompt carries.
const TASK: &str = "Read the fleet dossier below and propose operator actions. \
                    Propose only: issue no command and start no run.";

/// What the whole fleet looks like to the seat, plus the two things a
/// report is judged against: the facts the dossier states, and the
/// operator commands each run admits.
#[derive(Debug)]
pub struct Dossier {
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
    /// contract (decision 0057; decision 0007's provenance discipline).
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
/// The realm here is the realm the MAP names, never a hearth label: a
/// crossing is drawn between realms and is nothing to do with which
/// journal a realm's runs land in, so it is named for what it is even in
/// a one-hearth world, where the journal-derived facts beside it name no
/// realm at all.
pub struct RealmCrossings {
    pub realm: String,
    pub published: usize,
    pub consumed: usize,
    /// The consumed pins that no longer match the publisher's bytes —
    /// each one a finding, charged to THIS realm.
    pub moved: Vec<MovedPin>,
    /// How many of this realm's pins could not be checked at all, because
    /// the publisher's own file could not be read. Counted and never
    /// filed as a finding: nothing is broken about an unchecked pin, only
    /// unproven, and the publisher's own line is where that fault lives
    /// (`ca0c765`). Counted rather than dropped, so `consumed` is never
    /// read as "this many pins matched".
    pub unchecked: usize,
}

/// One pin that no longer matches, with the refusal's own words.
pub struct MovedPin {
    pub crossing: String,
    pub publisher: String,
    /// What `World::load` would have refused this realm's next run with,
    /// verbatim — never a second wording composed here.
    pub detail: String,
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
pub fn world_crossings(world: Option<&World>) -> Vec<RealmCrossings> {
    let Some(world) = world else {
        return Vec::new();
    };
    let mut crossings = Vec::new();
    for realm in &world.map.realms {
        let (published, consumed) = crate::realms::crossings_of(world.crossings_report(), realm);
        if published.is_empty() && consumed.is_empty() {
            continue;
        }
        let mut moved = Vec::new();
        let mut unchecked = 0;
        for entry in &consumed {
            match &entry.pin {
                Pin::Matching => {}
                Pin::Moved(detail) => moved.push(MovedPin {
                    crossing: entry.name.clone(),
                    publisher: entry.publisher.clone(),
                    detail: detail.clone(),
                }),
                Pin::Unchecked(_) => unchecked += 1,
            }
        }
        crossings.push(RealmCrossings {
            realm: realm.name.clone(),
            published: published.len(),
            consumed: consumed.len(),
            moved,
            unchecked,
        });
    }
    crossings
}

/// One hearth this command reads: the journal, and the realm its runs
/// belong to when the world holds more than one (decision 0026 ruling
/// 3). A one-hearth world names no realm, and its dossier is exactly the
/// dossier it always was.
pub struct Source<'a> {
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
pub fn dossier_of(sources: &[Source], crossings: &[RealmCrossings], now: &str) -> Result<Dossier> {
    let mut rows: Vec<Value> = Vec::new();
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
        for (run_id, feature, created_at) in store.list_runs()? {
            let events = store
                .load(&run_id)
                .with_context(|| format!("loading run '{run_id}'"))?;
            let state = match fold(&events) {
                Ok(state) => state,
                Err(error) => {
                    // One unfoldable journal must not blind the aide to the
                    // fleet. The run is quarantined — listed as `?` with the
                    // fold's own words — and raised as a finding, because an
                    // unfoldable journal is exactly what the operator needs
                    // surfaced, not hidden. Its citation is the sequence the
                    // fold refused at, so a proposal naming it validates.
                    let seq = error.seq();
                    let detail = error.to_string();
                    let finding = brokkr_view::quarantine_finding(&run_id, seq, &detail);
                    facts.push((realm.clone(), run_id.clone(), seq));
                    findings.push(keyed(&realm, serde_json::to_value(&finding)?));
                    commands.entry(run_id.clone()).or_default();
                    *counts.entry("quarantined".to_string()).or_default() += 1;
                    rows.push(keyed(
                        &realm,
                        json!({
                            "run_id": run_id,
                            "seq": seq,
                            "status": "?",
                            "phase": Value::Null,
                            "feature": feature,
                            "created_at": created_at,
                            "age": brokkr_view::age(&created_at, now),
                            "fold_error": detail,
                            "operator_commands": Vec::<String>::new(),
                        }),
                    ));
                    continue;
                }
            };
            let view = brokkr_view::run_view(&events, Some(&state));
            let summary = view
                .summary
                .as_ref()
                .expect("a folded state always summarizes");
            let admits = brokkr_view::operator_commands(&summary.status);
            *counts.entry(summary.status.clone()).or_default() += 1;
            facts.push((realm.clone(), run_id.clone(), summary.seq));
            // First hearth wins, matching [`Dossier::realm_of`]: where
            // two journals hold one run id, one answer is stated for it
            // and it is the same one on both sides.
            commands.entry(run_id.clone()).or_insert(admits.clone());
            for finding in brokkr_view::residual_findings(&run_id, &events) {
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
            // A panel or sequence seat's row already aggregates its members'
            // cost, so counting members again would bill the run twice.
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
            rows.push(keyed(
                &realm,
                json!({
                    "run_id": run_id,
                    "seq": summary.seq,
                    "status": summary.status,
                    "phase": summary.phase,
                    "feature": feature,
                    "created_at": created_at,
                    "age": brokkr_view::age(&created_at, now),
                    "park_reason": summary.park_reason,
                    "operator_commands": admits,
                    "consecutive_failures": summary.consecutive_failures,
                    "last_ruling": serde_json::to_value(&view.ruling)?,
                    "cost_usd": cost,
                    "seats": seats.iter().map(|part| json!({
                        "seat": part.label,
                        "phase": part.phase,
                        "status": part.status,
                        "attempts": part.attempts,
                        "cost_usd": part.cost,
                    })).collect::<Vec<Value>>(),
                }),
            ));
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
    // The world's shape, stated only by a world that has one to state: a
    // one-hearth dossier is the dossier it always was, byte for byte.
    let realms: Vec<&str> = sources.iter().filter_map(|source| source.realm).collect();
    let mut fleet = json!({
            "runs": rows.len(),
            "running": count("running"),
            "awaiting_operator": count("awaiting_operator"),
            "completed": count("completed"),
            "stopped": count("stopped"),
            // A run whose journal does not fold has no status to count
            // as; it is counted as what it is.
            "quarantined": count("quarantined"),
            // How many findings the operator has closed by name
            // (decision 0047 ruling 4). Counted, not hidden: the
            // summary says how many were left out of the queue.
            "superseded_findings": count("superseded_findings"),
    });
    if !realms.is_empty() {
        fleet["realms"] = json!(realms);
    }
    // Decision 0057 read into the dossier: what each mapped realm
    // publishes and consumes, and — when a consumed pin no longer matches
    // — a FINDING charged to the CONSUMING realm, because that is the
    // realm whose next run `World::load` would refuse. A matching pin is
    // stated in the per-realm summary and is not a finding; an unchecked
    // pin is counted beside it and is nobody's fault here (the publisher's
    // line carries it, `ca0c765`).
    let mut crossing_reports: Vec<Value> = Vec::new();
    let mut cited_crossings: Vec<CitedCrossing> = Vec::new();
    for realm_crossings in crossings {
        if realm_crossings.published == 0 && realm_crossings.consumed == 0 {
            continue;
        }
        let mut report = json!({
            "realm": realm_crossings.realm,
            "published": realm_crossings.published,
            "consumed": realm_crossings.consumed,
        });
        if realm_crossings.unchecked > 0 {
            report["unchecked"] = json!(realm_crossings.unchecked);
        }
        if !realm_crossings.moved.is_empty() {
            report["moved"] = json!(realm_crossings
                .moved
                .iter()
                .map(|moved| json!({
                    "crossing": moved.crossing,
                    "publisher": moved.publisher,
                    "detail": moved.detail,
                }))
                .collect::<Vec<Value>>());
        }
        crossing_reports.push(report);
        for moved in &realm_crossings.moved {
            cited_crossings.push((realm_crossings.realm.clone(), moved.crossing.clone()));
            findings.push(keyed(
                &Some(realm_crossings.realm.clone()),
                json!({
                    "crossing": moved.crossing,
                    "publisher": moved.publisher,
                    // A crossing is not a run: it has no run id and no
                    // sequence, so the finding says what it is instead of
                    // borrowing a run's citation. The marker is how a
                    // reader tells it from a run finding.
                    "input": "crossing_pins",
                    "value": "moved",
                    // The refusal `run` would give, verbatim, so the
                    // readout and the loader cannot word one contract
                    // twice.
                    "line": moved.detail,
                }),
            ));
        }
    }
    let mut value = json!({
        "dossier_version": DOSSIER_VERSION,
        "generated_at": now,
        "fleet": fleet,
        "runs": rows,
        "residual_findings": findings,
    });
    // Written only by a world that draws one, so a world that never did
    // hands the seat the exact dossier it always handed it.
    if !crossing_reports.is_empty() {
        value["crossings"] = json!(crossing_reports);
    }
    Ok(Dossier {
        value,
        facts,
        crossings: cited_crossings,
        commands,
    })
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
pub struct Seat {
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
pub fn seat(agents_dir: &Path, adapters_dir: &Path) -> Result<Seat> {
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
        command: expand_command(agents_dir, &candidate.argv),
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
pub struct Report {
    pub fleet_summary: String,
    pub parked_runs: Vec<Value>,
    pub work_queue: Vec<Value>,
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

/// One citation's fields, realm first when there is one. Absent for a
/// one-hearth world, so its record keeps exactly the shape it had.
fn cite(realm: &Option<String>, run_id: &str, seq: u64) -> serde_json::Map<String, Value> {
    let mut fields = serde_json::Map::new();
    if let Some(realm) = realm {
        fields.insert("realm".to_string(), json!(realm));
    }
    fields.insert("run_id".to_string(), json!(run_id));
    fields.insert("seq".to_string(), json!(seq));
    fields
}

/// Read the seat's result into a report, or say exactly what is wrong
/// with it. Nothing is repaired and nothing is partially accepted
/// (decision 0001): a report with one bad entry is a bad report.
pub fn validate(dossier: &Dossier, result: &Value) -> Result<Report, String> {
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
        let mut fields = cite(&realm, &run_id, seq);
        fields.insert("command".to_string(), json!(command));
        fields.insert("reasoning".to_string(), json!(reasoning));
        citations.push((realm, run_id, seq));
        parked_runs.push(Value::Object(fields));
    }
    let mut work_queue = Vec::new();
    let mut crossing_citations: Vec<CitedCrossing> = Vec::new();
    for entry in array_field(inputs, "work_queue")? {
        let what = "a work-queue entry";
        // A crossing finding is cited by realm and crossing; every other
        // entry is a run fact cited by run id and sequence. The presence
        // of `crossing` is the discriminator, so a report that names both
        // a crossing and a run is read as a crossing and never silently
        // as a run.
        if entry.get("crossing").is_some() {
            let (realm, crossing) = crossing_citation(what, dossier, entry)?;
            let finding = string_field(what, entry, "finding")?;
            let reasoning = string_field(what, entry, "reasoning")?;
            let mut fields = serde_json::Map::new();
            fields.insert("realm".to_string(), json!(realm));
            fields.insert("crossing".to_string(), json!(crossing));
            fields.insert("finding".to_string(), json!(finding));
            fields.insert("reasoning".to_string(), json!(reasoning));
            crossing_citations.push((realm, crossing));
            work_queue.push(Value::Object(fields));
            continue;
        }
        let (realm, run_id, seq) = citation(what, dossier, entry)?;
        let finding = string_field(what, entry, "finding")?;
        let reasoning = string_field(what, entry, "reasoning")?;
        let mut fields = cite(&realm, &run_id, seq);
        fields.insert("finding".to_string(), json!(finding));
        fields.insert("reasoning".to_string(), json!(reasoning));
        citations.push((realm, run_id, seq));
        work_queue.push(Value::Object(fields));
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
fn usage(checkpoints: &[Value]) -> Value {
    let session = checkpoints.iter().rev().find(|checkpoint| {
        checkpoint
            .get("step")
            .and_then(Value::as_str)
            .is_some_and(|step| step.ends_with("-session-finished"))
    });
    match session {
        None => Value::Null,
        Some(session) => json!({
            "cost_usd": session.get("total_cost_usd"),
            "turns": session.get("num_turns"),
            "session_id": session.get("session_id"),
        }),
    }
}

/// One line of the record: what was proposed, what it was derived from,
/// when, and what the seat cost.
fn entry(now: &str, seat: &Seat, dossier: &Dossier, report: &Report, usage: Value) -> Value {
    let mut entry = json!({
        "record_version": RECORD_VERSION,
        "recorded_at": now,
        "agent": {
            "name": AGENT,
            "model": seat.model,
            "provider": seat.provider,
            "deadline_seconds": seat.deadline.as_secs(),
        },
        "dossier": {
            "dossier_version": DOSSIER_VERSION,
            "generated_at": now,
            "fleet": dossier.value.get("fleet"),
        },
        "fleet_summary": report.fleet_summary,
        "parked_runs": report.parked_runs,
        "work_queue": report.work_queue,
        "citations": report.citations.iter()
            .map(|(realm, run_id, seq)| Value::Object(cite(realm, run_id, *seq)))
            .collect::<Vec<Value>>(),
        "usage": usage,
    });
    // Written only by a report that cited one, so a run-only record keeps
    // exactly the shape it always had. A crossing citation carries no run
    // id and no sequence; its realm is always stated.
    if !report.crossing_citations.is_empty() {
        entry["crossing_citations"] = json!(report
            .crossing_citations
            .iter()
            .map(|(realm, crossing)| json!({"realm": realm, "crossing": crossing}))
            .collect::<Vec<Value>>());
    }
    entry
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
pub fn run(
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
    let sole = hearths.len() < 2;
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
                if sole {
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
    // world where NOT ONE journal opened has no dossier to write about,
    // and saying so beats sending an aide to read an empty world.
    anyhow::ensure!(
        !stores.is_empty(),
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
    // The world's crossings, read off the SAME report `brokkr realms` and
    // `brokkr doctor` read (decision 0057): nothing is resolved, hashed or
    // compared again here, and a moved pin becomes a finding rather than
    // the end of the flight. A world with no map draws none.
    let crossings = world_crossings(world);
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
            return Ok(ExitCode::from(1));
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
            return Ok(ExitCode::from(1));
        }
    };
    let entry = entry(now, &seat, &dossier, &report, usage(&checkpoints));
    record::append(record_path, &entry)?;
    print!("{}", render(&entry));
    eprintln!(
        "recorded in {}; nothing was executed — issue any command yourself",
        record_path.display()
    );
    Ok(ExitCode::SUCCESS)
}

/// `brokkr muninn list`. Reads the record back, citations included.
pub fn list(record_path: &Path, json: bool) -> Result<()> {
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
