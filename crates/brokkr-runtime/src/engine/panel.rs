//! A panel seat's members, run concurrently INSIDE one attempt (decision
//! 0002's parallel form), and the declared aggregate that joins their
//! results into the one typed result the outer machine sees (#288).
//!
//! A panel is a seat body or a sequence step: [`Engine::member_runs`]
//! derives each member's invocation, [`Engine::run_panel`] runs them and
//! journals their live checkpoints, [`Engine::journal_panel_members`]
//! journals each member's outcome in declared order, and [`panel_outcome`]
//! joins the reports by the [`Aggregate`] the seat declares.

use std::path::Path;

use brokkr_core::envelope::EventType;
use brokkr_core::policy::SEVERITY_ORDER;
use brokkr_core::realms::Boundary;
use brokkr_protocol::{AttemptOutcome, AttemptReport, Cleanup};
use brokkr_store::Store;
use serde_json::{json, Map, Value};

use super::{
    argv_for, checkpoints, copy_secret_binding_facts, failed_to_start, resume, spawn_site,
    stamp_boundary, start_failure_fields, tag_member, Calls, Checkpoints, Engine, EngineError,
    Indeterminate, Selection, Site, SiteSpawn, Unproven,
};
use crate::bundle::{Aggregate, Bundle, PanelMember, ENGINE_VERSION};

impl Engine {
    /// Run a parallel panel INSIDE one effect (decision 0002): members
    /// execute concurrently, join as a barrier in declared order, each
    /// member's outcome is journaled as checkpoint evidence, and the
    /// declared deterministic aggregate produces the single typed result
    /// the outer machine sees. Any indeterminate member makes the whole
    /// attempt indeterminate (park); otherwise any failed member fails
    /// the attempt (retryable under 0006).
    #[expect(clippy::too_many_arguments, reason = "baseline 2026-09, #288")]
    pub(super) fn execute_panel(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        seat_name: &str,
        members: &[PanelMember],
        aggregate: Aggregate,
        panel_input: &Value,
        deadline: std::time::Duration,
        selection: &Selection,
        gate: bool,
    ) -> Result<(), EngineError> {
        let runs = self.member_runs(
            attempt_id,
            seat_name,
            members,
            &panel_input["members"],
            panel_input,
            &panel_input["context"],
            selection,
            "",
            gate,
        );
        let reports = self.run_panel(effect_id, attempt_id, &runs, deadline, "")?;
        self.journal_panel_members(effect_id, attempt_id, &reports, &runs, "")?;
        let start_failures = start_failure_sites(&reports, "");
        let unproven = Unproven::panel(&reports);
        match panel_outcome(aggregate, reports) {
            AttemptOutcome::Indeterminate { reason } => {
                self.append(
                    EventType::EffectIndeterminate,
                    json!(Indeterminate {
                        effect_id,
                        attempt_id,
                        reason,
                        unproven,
                    }),
                    Some(attempt_id.to_string()),
                )?;
            }
            AttemptOutcome::Failed { error } => {
                let mut payload = json!({
                    "effect_id": effect_id,
                    "attempt_id": attempt_id,
                    "error": error,
                });
                // Per member, not per attempt: a member that failed to
                // start advances its OWN chain index, and a member that
                // ran does not.
                start_failure_fields(&mut payload, selection, start_failures);
                self.append(
                    EventType::EffectFailed,
                    payload,
                    Some(attempt_id.to_string()),
                )?;
            }
            AttemptOutcome::Succeeded { result } => {
                // The aggregate is the engine's own record — `result`,
                // `inputs`, `notes` and nothing else, with every member's
                // evidence under `notes` — so it conforms by
                // construction and the seat-record fence has nothing to
                // refuse here. A refusal would be an engine defect, and
                // it propagates as one, like any other storage error;
                // it is not dressed as a member's failure.
                self.append(
                    EventType::EffectSucceeded,
                    json!({"effect_id": effect_id, "attempt_id": attempt_id, "result": result}),
                    Some(attempt_id.to_string()),
                )?;
            }
        }
        Ok(())
    }

    /// Derive one driver invocation per panel member from the
    /// requested-time input: `members_meta` holds the per-member
    /// role/result paths, `driver_seat_prefix` is the seat name (or
    /// `<seat>:<step>` inside a sequence), and `context` already carries
    /// any accumulated prior step results.
    #[expect(clippy::too_many_arguments, reason = "baseline 2026-09, #288")]
    pub(super) fn member_runs(
        &mut self,
        attempt_id: &str,
        driver_seat_prefix: &str,
        members: &[PanelMember],
        members_meta: &Value,
        seat_input: &Value,
        context: &Value,
        selection: &Selection,
        tag_prefix: &str,
        gate: bool,
    ) -> Vec<MemberRun> {
        members
            .iter()
            .map(|member| {
                let site = Some(format!("{tag_prefix}{}", member.name));
                let label = format!("{driver_seat_prefix}:{}", member.name);
                let mut input = json!({
                    "feature": seat_input["feature"],
                    "phase": seat_input["phase"],
                    "seat": format!("{driver_seat_prefix}:{}", member.name),
                    "role_path": members_meta[&member.name]["role_path"],
                    "workdir": seat_input["workdir"],
                    "result_path": members_meta[&member.name]["result_path"],
                    "allowed_results": seat_input["allowed_results"],
                    "house_rules": seat_input["house_rules"],
                    "context": context,
                });
                self.marks()
                    .member_dialect(&member.name, seat_input, &mut input);
                copy_secret_binding_facts(&mut input, seat_input);
                self.marks()
                    .site(&label, gate, selection.get(&site), &mut input);
                let hands = self.hands_for(&label);
                let mut spawn = self.compose_at(
                    Some(&label),
                    attempt_id,
                    gate,
                    argv_for(selection, &site, &member.command).to_vec(),
                    hands.as_ref(),
                    selection.get(&site),
                    input["result_path"].as_str().unwrap_or_default(),
                );
                self.mark_capabilities(&label, selection.get(&site), Some(&mut spawn), &mut input);
                // Each member's OWN offer and stamps, never the panel's:
                // selection is per site, not per aggregate (proposed
                // decision 0056 ruling 1).
                let plan = selection.plan(&site);
                if let Some(plan) = plan {
                    input["resume_context"] = resume::start_context(
                        plan.assessment.clone(),
                        plan.originating.as_ref(),
                        plan.offer.as_ref(),
                        plan.route_overlay.as_ref(),
                    );
                }
                MemberRun {
                    name: member.name.clone(),
                    driver_seat: label.clone(),
                    boundary: self.site_boundary(&label),
                    offer: plan.and_then(|plan| plan.offer.clone()),
                    context: plan.map(|plan| plan.context.clone()),
                    spawn,
                    input,
                }
            })
            .collect()
    }

    /// Run panel members concurrently INSIDE one attempt, journaling
    /// live member checkpoints as they arrive, and return the reports in
    /// declared order. Appends NO terminal effect event. `tag_prefix` is
    /// empty for a seat-level panel and `<step>:` inside a sequence, so
    /// the journaled member tag reads `<member>` or `<step>:<member>`.
    pub(super) fn run_panel(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        runs: &[MemberRun],
        deadline: std::time::Duration,
        tag_prefix: &str,
    ) -> Result<Vec<(String, AttemptReport)>, EngineError> {
        let workdir = self.workdir();
        // Split the borrows: member threads run drivers, while the
        // main-thread receive loop below needs the store and causal cursor.
        let bundle = &self.bundle;
        let attempt = checkpoints::Attempt {
            run_id: &self.run_id,
            effect_id,
            attempt_id,
        };
        // Each member's own call authority, never a sibling's.
        let mut calls: Vec<Calls> = runs
            .iter()
            .map(|run| {
                let facts = bundle.sites.get(&run.driver_seat);
                Calls::of(facts, &run.spawn, attempt_id, run.context.as_ref())
            })
            .collect();
        // The member whose checkpoint the fence refused, if one was
        // (decision 0034, ruling 6), is the tagged member name the
        // checkpoint rode under, so the refusal lands on that member's
        // report alone.
        let mut sink = Checkpoints::new(&mut self.store, &mut self.current_cause, attempt);
        let reports: Vec<(String, AttemptReport)> = std::thread::scope(|scope| {
            let (sender, receiver) = checkpoints::arrivals::<(String, Value)>();
            let handles: Vec<_> = runs
                .iter()
                .map(|run| {
                    let name = run.name.clone();
                    let checkpoint_name = format!("{tag_prefix}{}", run.name);
                    let workdir = workdir.clone();
                    let sender = sender.clone();
                    // Live telemetry: hand each checkpoint to the main
                    // thread — the store has one writer.
                    let forward = move |data: &Value| {
                        let _ = sender.send((checkpoint_name.clone(), data.clone()));
                    };
                    scope.spawn(move || {
                        let report =
                            run.report(bundle, &workdir, deadline, effect_id, attempt_id, forward);
                        (name, report)
                    })
                })
                .collect();
            // The main thread journals live member checkpoints as they
            // arrive (wall-clock order — checkpoints are temporal evidence,
            // nothing aggregates from them). Dropping our sender makes the
            // loop end when the last member finishes emitting. The sink
            // latches on an append error and the loop keeps draining — an
            // abandoned channel must not deadlock the members.
            drop(sender);
            sink.take(&receiver, |sink, (member, checkpoint)| {
                offer_member_checkpoint(sink, runs, &mut calls, tag_prefix, &member, checkpoint);
            });
            handles
                .into_iter()
                .map(|h| h.join().expect("panel member thread"))
                .collect()
        });
        let settled = sink.settle()?;
        Ok(reports
            .into_iter()
            .map(|(name, mut report)| {
                settled.carry(&format!("{tag_prefix}{name}"), &mut report);
                (name, report)
            })
            .collect())
    }

    /// Journal member outcomes as checkpoint evidence in declared
    /// (stable) order — never wall-clock completion order.
    pub(super) fn journal_panel_members(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        reports: &[(String, AttemptReport)],
        runs: &[MemberRun],
        tag_prefix: &str,
    ) -> Result<(), EngineError> {
        for (name, report) in reports {
            let boundary = runs
                .iter()
                .find(|run| run.name == *name)
                .and_then(|run| run.boundary);
            let settled = report.settled_outcome();
            let kind = match &settled {
                AttemptOutcome::Succeeded { .. } => "succeeded",
                AttemptOutcome::Failed { .. } => "failed",
                AttemptOutcome::Indeterminate { .. } => "indeterminate",
            };
            let model = match &settled {
                AttemptOutcome::Succeeded { result } => result
                    .get("model")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                _ => None,
            }
            .or_else(|| {
                report
                    .checkpoints
                    .iter()
                    .rev()
                    .find_map(|checkpoint| checkpoint.get("model").and_then(Value::as_str))
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "not reported".to_string());
            // The engine's own marker names the member's model, so it
            // carries the member's boundary beside it (design DD19).
            let marker = stamp_boundary(
                json!({
                    "step": "panel-member-finished",
                    "member": format!("{tag_prefix}{name}"),
                    "outcome": kind,
                    "model": model,
                    "session_ref": report.session_ref,
                    "inner_checkpoints": report.checkpoints.len(),
                }),
                boundary,
            );
            self.append(
                EventType::EffectCheckpointed,
                json!({
                    "effect_id": effect_id,
                    "attempt_id": attempt_id,
                    "checkpoint": marker,
                }),
                Some(attempt_id.to_string()),
            )?;
        }
        Ok(())
    }
}

/// Which panel members failed to start, tagged as the journal tags them.
pub(super) fn start_failure_sites(
    reports: &[(String, AttemptReport)],
    tag_prefix: &str,
) -> Vec<Site> {
    reports
        .iter()
        .filter(|(_, report)| failed_to_start(report))
        .map(|(name, _)| Some(format!("{tag_prefix}{name}")))
        .collect()
}

/// One panel member's derived driver invocation: the `seat` string the
/// driver sees, the composed spawn, the boundary that stands at the
/// member's site, and its input.
pub(super) struct MemberRun {
    pub(super) name: String,
    pub(super) driver_seat: String,
    pub(super) boundary: Option<Boundary>,
    /// This member's own eligible session and its persistence locator,
    /// if the engine offered one.
    pub(super) offer: Option<resume::ResumeTarget>,
    /// This member's own structural identity, handed beside its
    /// checkpoints to the single journal writer for stamping — the
    /// worker threads never touch the store.
    pub(super) context: Option<resume::SiteContext>,
    pub(super) spawn: SiteSpawn,
    pub(super) input: Value,
}

impl MemberRun {
    /// This member's attempt, on its own worker thread: its driver run
    /// to a report, each checkpoint handed to `on_checkpoint` as it
    /// arrives. A member whose driver did not spawn reports a failure
    /// that started nothing.
    fn report(
        &self,
        bundle: &Bundle,
        workdir: &Path,
        deadline: std::time::Duration,
        effect_id: &str,
        attempt_id: &str,
        on_checkpoint: impl FnMut(&Value),
    ) -> AttemptReport {
        match spawn_site(bundle, &self.spawn, &self.input, workdir, deadline) {
            Err(e) => AttemptReport {
                outcome: AttemptOutcome::Failed {
                    error: format!("member driver did not spawn: {e}"),
                },
                refused: None,
                // Nothing was spawned, so nothing is left.
                cleanup: Cleanup::Settled,
                session_ref: None,
                checkpoints: Vec::new(),
                stderr: String::new(),
                // Nothing ran, so nothing was accepted: the structural
                // fail-to-start predicate holds.
                accepted: false,
                // No process existed for a watchdog to kill.
                deadline_killed: false,
            },
            // The door's own input, carrying the charter text it verified
            // for this member.
            Ok((process, input)) => process.run_attempt_resuming(
                ENGINE_VERSION,
                effect_id,
                attempt_id,
                &self.driver_seat,
                input,
                // This member's own offer, decided in `site_plans` before
                // anything spawned. Only its provider ID crosses the wire;
                // the locator rides the member's private `resume_context`.
                self.offer.as_ref().map(|offer| offer.provider_id.clone()),
                on_checkpoint,
            ),
        }
    }
}

/// Offer one live member checkpoint to the panel's sink, on the main
/// thread: read by the call authority of the member it is tagged with,
/// stamped with that member's boundary and structural identity.
fn offer_member_checkpoint(
    sink: &mut Checkpoints<'_, fn(&mut Store)>,
    runs: &[MemberRun],
    calls: &mut [Calls],
    tag_prefix: &str,
    member: &str,
    mut checkpoint: Value,
) {
    let at = runs
        .iter()
        .position(|run| format!("{tag_prefix}{}", run.name) == member);
    let owner = at.map(|at| &runs[at]);
    let call = at.map_or(Ok(None), |at| calls[at].consume(&mut checkpoint));
    let boundary = owner.and_then(|run| run.boundary);
    let checkpoint = stamp_boundary(tag_member(checkpoint, member), boundary);
    // The member's own stamp, applied by the one journal
    // writer from the context its worker handed over.
    let checkpoint = match owner.and_then(|run| run.context.as_ref()) {
        Some(context) => context.stamp(checkpoint),
        None => resume::unstamped(checkpoint),
    };
    sink.offer(member, checkpoint, call);
}

/// Join member reports into the attempt's single outcome: any
/// indeterminate member parks the whole attempt; otherwise any failed
/// member fails it (retryable under 0006); otherwise the declared
/// aggregate produces the one typed result.
pub(super) fn panel_outcome(
    aggregate: Aggregate,
    reports: Vec<(String, AttemptReport)>,
) -> AttemptOutcome {
    let mut indeterminate = Vec::new();
    let mut failures = Vec::new();
    let mut member_results = Vec::new();
    for (name, report) in reports {
        match report.settled_outcome() {
            AttemptOutcome::Succeeded { result } => member_results.push((name, result)),
            AttemptOutcome::Failed { error } => failures.push(format!("{name}: {error}")),
            AttemptOutcome::Indeterminate { .. } => indeterminate.push(name),
        }
    }
    if !indeterminate.is_empty() {
        return AttemptOutcome::Indeterminate {
            reason: format!("panel members {indeterminate:?} could not establish completion"),
        };
    }
    if !failures.is_empty() {
        return AttemptOutcome::Failed {
            error: format!("panel members failed — {}", failures.join("; ")),
        };
    }
    AttemptOutcome::Succeeded {
        result: aggregate.aggregate_results(&member_results),
    }
}

impl Aggregate {
    /// Deterministic, order-independent aggregation of member results. A
    /// member payload without a readable result string poisons the
    /// aggregate into a marker the engine's own schema validation parks
    /// with the full member evidence attached — never coerced (law 0001).
    /// A member result outside the vocabulary ranks WORST and flows to
    /// decide(), whose declared-results check parks it with evidence.
    pub(super) fn aggregate_results(self, members: &[(String, Value)]) -> Value {
        let Some(read) = Verdicts::read(members) else {
            let evidence: Map<String, Value> = members.iter().cloned().collect();
            return json!({
                "result": "__member-schema-invalid__",
                "notes": {"members": Value::Object(evidence)},
            });
        };
        let meta = json!({"members": read.notes, "verdicts": read.verdicts});
        match self {
            Aggregate::UnanimousPass => unanimous_pass(&read.parsed, meta),
            Aggregate::ReviewPanel => review_panel(&read.parsed, meta),
        }
    }
}

/// Every member's result string, read once for the aggregate: by member,
/// its notes and its verdict as evidence, and beside its name the result
/// and the payload it came in.
struct Verdicts<'a> {
    notes: Map<String, Value>,
    verdicts: Map<String, Value>,
    parsed: Vec<(&'a str, &'a str, &'a Value)>,
}

impl<'a> Verdicts<'a> {
    /// The members' verdicts, or `None` when any member's payload has no
    /// readable result string.
    fn read(members: &'a [(String, Value)]) -> Option<Self> {
        let mut notes = Map::new();
        let mut verdicts = Map::new();
        let mut parsed: Vec<(&str, &str, &Value)> = Vec::new();
        for (name, payload) in members {
            if let Some(note) = payload.get("notes") {
                notes.insert(name.clone(), note.clone());
            }
            let result = payload.get("result").and_then(Value::as_str)?;
            verdicts.insert(name.clone(), Value::String(result.to_string()));
            parsed.push((name.as_str(), result, payload));
        }
        Some(Verdicts {
            notes,
            verdicts,
            parsed,
        })
    }
}

/// [`Aggregate::UnanimousPass`]: "pass" only when every member reports
/// "pass", otherwise "fail".
fn unanimous_pass(parsed: &[(&str, &str, &Value)], meta: Value) -> Value {
    // Only the declared vocabulary aggregates; anything else is
    // never coerced to "fail" (law 0001) — it poisons the result
    // so decide() parks with the member evidence.
    if parsed.iter().any(|(_, r, _)| *r != "pass" && *r != "fail") {
        return json!({
            "result": "__member-schema-invalid__",
            "notes": meta,
        });
    }
    let all_pass = parsed.iter().all(|(_, r, _)| *r == "pass");
    json!({
        "result": if all_pass { "pass" } else { "fail" },
        "notes": meta,
    })
}

/// [`Aggregate::ReviewPanel`]: the worst member's result wins, over
/// clean < residual < security-hold, with the members' residual inputs
/// joined beside it.
fn review_panel(parsed: &[(&str, &str, &Value)], meta: Value) -> Value {
    let rank = |r: &str| match r {
        "clean" => 0,
        "residual" => 1,
        "security-hold" => 2,
        _ => 3, // unknown ranks worst: fail closed via decide()
    };
    let worst = parsed
        .iter()
        .max_by_key(|(_, r, _)| rank(r))
        .expect("panels have members");
    let joined = ReviewInputs::join(parsed);
    let mut inputs = Map::new();
    // Backward-compatible evidence for frozen/third-party tables:
    // judges never fix, so the only lawful value is false. Shipped
    // tables no longer declare or read it and therefore drop it.
    inputs.insert("fixes_applied".into(), Value::Bool(false));
    inputs.insert("spec_defect".into(), Value::Bool(joined.spec_defect));
    if worst.1 == "residual" || joined.severity_rank > 0 {
        inputs.insert(
            "max_residual_severity".into(),
            Value::String(SEVERITY_ORDER[joined.severity_rank].to_string()),
        );
        inputs.insert(
            "has_security_residual".into(),
            Value::Bool(joined.has_security),
        );
    }
    json!({"result": worst.1, "inputs": inputs, "notes": meta})
}

/// What a review panel's members report in their own `inputs`, joined:
/// the highest residual severity any names (one outside
/// [`SEVERITY_ORDER`] counts as none), and whether any names a security
/// residual or a spec defect.
struct ReviewInputs {
    severity_rank: usize,
    has_security: bool,
    spec_defect: bool,
}

impl ReviewInputs {
    fn join(parsed: &[(&str, &str, &Value)]) -> Self {
        let mut severity_rank = 0usize;
        let mut has_security = false;
        let mut spec_defect = false;
        for (_, _, payload) in parsed {
            let Some(inputs) = payload.get("inputs").and_then(Value::as_object) else {
                continue;
            };
            if let Some(s) = inputs.get("max_residual_severity").and_then(Value::as_str) {
                if let Some(i) = SEVERITY_ORDER.iter().position(|x| *x == s) {
                    severity_rank = severity_rank.max(i);
                }
            }
            has_security |= inputs
                .get("has_security_residual")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            spec_defect |= inputs
                .get("spec_defect")
                .and_then(Value::as_bool)
                .unwrap_or(false);
        }
        ReviewInputs {
            severity_rank,
            has_security,
            spec_defect,
        }
    }
}
