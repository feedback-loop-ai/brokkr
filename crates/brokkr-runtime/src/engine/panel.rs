//! A panel seat's members, run concurrently INSIDE one attempt (decision
//! 0002's parallel form), and the declared aggregate that joins their
//! results into the one typed result the outer machine sees (#288).
//!
//! A panel is a seat body or a sequence step: [`Engine::member_runs`]
//! derives each member's invocation, [`Engine::run_panel`] runs them and
//! journals their live checkpoints, [`Engine::journal_panel_members`]
//! journals each member's outcome in declared order, and [`panel_outcome`]
//! joins the reports by the [`Aggregate`] the seat declares.

use std::collections::BTreeMap;
use std::path::Path;

use brokkr_core::envelope::EventType;
use brokkr_core::findings::EngineMarker;
use brokkr_core::realms::Boundary;
use brokkr_protocol::{AttemptOutcome, AttemptReport, Cleanup};
use brokkr_store::Store;
use serde_json::{json, Value};

use super::broker::fold::Begun;
use super::broker::settlement::ended;
use super::broker::Held;
use super::carry::{
    outcome_under,
    record::{settled_members, PanelRecord, PanelSpan, Tag},
};
use super::cooldown::refused_members;
use super::dispatch::deadline::AttemptBound;
use super::{
    argv_for, checkpoints, copy_secret_binding_facts, failed_to_start, resume, spawn_site,
    stamp_boundary, start_failure_fields, tag_member, Calls, Checkpoints, Door, DriverRun, Engine,
    EngineError, Indeterminate, Selection, Site, SiteSpawn, Unproven,
};
use crate::agents::kinds::{SeatKinds, UNDECLARED};
use crate::bundle::{
    Aggregate, Bundle, MemberFindings, MemberVerdict, PanelJoin, PanelMember, ENGINE_VERSION,
};

pub(super) struct PanelAt<'a> {
    effect_id: &'a str,
    attempt_id: &'a str,
    seat_name: &'a str,
    gate: bool,
}

impl<'a> PanelAt<'a> {
    pub(super) fn new(
        effect_id: &'a str,
        attempt_id: &'a str,
        seat_name: &'a str,
        gate: bool,
    ) -> Self {
        Self {
            effect_id,
            attempt_id,
            seat_name,
            gate,
        }
    }
}

impl Engine {
    /// Run a parallel panel INSIDE one effect (decision 0002): members
    /// execute concurrently, join as a barrier in declared order, each
    /// member's outcome is journaled as checkpoint evidence, and the
    /// declared deterministic aggregate produces the single typed result
    /// the outer machine sees. Any indeterminate member makes the whole
    /// attempt indeterminate (park); otherwise any failed member fails
    /// the attempt (retryable under 0006). A panel's terminal words name
    /// the attempt's deadline once, at the joined outcome.
    pub(super) fn execute_panel(
        &mut self,
        at: &PanelAt<'_>,
        members: &[PanelMember],
        aggregate: Aggregate,
        panel_input: &Value,
        bound: AttemptBound<'_>,
        selection: &Selection,
    ) -> Result<(), EngineError> {
        let PanelAt {
            effect_id,
            attempt_id,
            seat_name,
            gate,
        } = *at;
        let join = PanelJoin::of(aggregate, members);
        let mut span = PanelSpan::open(&self.repo);
        let runs = self.member_runs(
            crate::engine::AttemptIds::new(effect_id, attempt_id),
            seat_name,
            members,
            &panel_input["members"],
            panel_input,
            &panel_input["context"],
            selection,
            "",
            gate,
        );
        let (reports, stamped) =
            self.run_panel(effect_id, attempt_id, &runs, bound.remaining, "")?;
        self.usage.panel(attempt_id, &reports);
        self.journal_panel_members(effect_id, attempt_id, &reports, &runs, "")?;
        span.close(&self.repo);
        let record = PanelRecord {
            span,
            settled_members: settled_members((&reports, &stamped), None),
        };
        let start_failures = start_failure_sites(&reports, "");
        // Every member that refused with a rate limit records its own wait
        // (decision 0103 ruling 2(a), slice 4c2a): one entry per refused
        // site, assembled here from the reports in declared order.
        let refused = refused_members(&reports, "");
        let events = self.store.load(&self.run_id)?;
        let unproven = Unproven::panel(&reports);
        let carry = selection.carry.as_ref();
        // One line, as `disposed`'s refusal callsites: the exact-coverage
        // gate reads a multi-line call's `?` edge as its own region.
        let name = bound.deadline.panel_namer(&reports);
        let members = (reports, stamped);
        let mut outcome = outcome_under(&join, carry, members, &record.span, attempt_id)?;
        name(&mut outcome);
        match outcome.outcome() {
            AttemptOutcome::Indeterminate { reason } => {
                let mut payload = json!(Indeterminate {
                    effect_id,
                    attempt_id,
                    reason,
                    unproven
                });
                record.stamp(&mut payload);
                let terminal =
                    self.panel_rate_limit_terminal(&events, effect_id, payload, &refused, false);
                self.append(
                    EventType::EffectIndeterminate,
                    terminal.payload,
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
                record.stamp(&mut payload);
                let terminal =
                    self.panel_rate_limit_terminal(&events, effect_id, payload, &refused, true);
                self.append(
                    EventType::EffectFailed,
                    terminal.payload,
                    Some(attempt_id.to_string()),
                )?;
                if let Some(park) = terminal.park {
                    self.park(&park)?;
                }
            }
            AttemptOutcome::Succeeded { result } => {
                // The aggregate is the engine's own record — `result`,
                // `inputs`, `notes`, the members' stamped `findings` and
                // nothing else, with every member's evidence under
                // `notes` — so it conforms by
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
        ids: crate::engine::AttemptIds<'_>,
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
                let site = Some(Tag::prefixed(tag_prefix, &member.name).0);
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
                    ids,
                    gate,
                    argv_for(selection, &site, &member.command).to_vec(),
                    hands.as_ref(),
                    selection.get(&site),
                    input["result_path"].as_str().unwrap_or_default(),
                );
                let (link, brokers) = (selection.get(&site), selection.brokers(&label));
                let served = (brokers, hands.as_ref());
                self.mark_serving(&label, link, served, Some(&mut spawn), &mut input);
                // Each member's OWN offer and stamps, never the panel's:
                // selection is per site, not per aggregate (proposed
                // decision 0056 ruling 1).
                let plan = selection.plan(&site);
                if let Some(plan) = plan {
                    plan.attach_resume_context(&mut input);
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
    /// declared order, each settled by its own broker calls
    /// ([`Engine::settle_members`]). Appends NO terminal effect event.
    /// `tag_prefix` is empty for a seat-level panel and `<step>:` inside a
    /// sequence, so the journaled member tag reads `<member>` or
    /// `<step>:<member>`.
    pub(super) fn run_panel(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        runs: &[MemberRun],
        deadline: std::time::Duration,
        tag_prefix: &str,
    ) -> Result<Members, EngineError> {
        let workdir = self.workdir();
        // Where each member's attempt began, before anything of it spawns.
        let begun =
            (runs.iter().map(|run| self.begun(&run.spawn))).collect::<Result<Vec<_>, _>>()?;
        // Split the borrows: member threads run drivers, while the
        // main-thread receive loop below needs the store and causal cursor.
        let (bundle, held) = (&self.bundle, self.held.as_ref());
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
        let reports: Vec<_> = std::thread::scope(|scope| {
            let (sender, receiver) = checkpoints::arrivals::<(String, Value)>();
            let handles: Vec<_> = (runs.iter().zip(begun))
                .map(|(run, begun)| {
                    let name = run.name.clone();
                    let checkpoint_name = Tag::prefixed(tag_prefix, &run.name).0;
                    let workdir = workdir.clone();
                    let sender = sender.clone();
                    // Live telemetry: hand each checkpoint to the main
                    // thread — the store has one writer.
                    let forward = move |data: &Value| {
                        let _ = sender.send((checkpoint_name.clone(), data.clone()));
                    };
                    scope.spawn(move || {
                        let door = (bundle, held, begun.as_ref());
                        // A member whose driver did not spawn started no
                        // broker: nothing of it is folded (CR3).
                        match run.report(door, &workdir, deadline, effect_id, attempt_id, forward) {
                            DriverRun::Ran(report) => (name, report, begun),
                            DriverRun::SpawnFailed(error) => (name, unspawned(&error), None),
                        }
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
                let at = (runs, &mut calls[..], tag_prefix);
                offer_member_checkpoint(sink, at, &member, checkpoint);
            });
            handles
                .into_iter()
                .map(|h| h.join().expect("panel member thread"))
                .collect()
        });
        let settled = sink.settle();
        self.settle_members([effect_id, attempt_id], runs, reports, settled, tag_prefix)
    }

    /// The members' `reports`, in declared order beside their `runs`, each
    /// with where its attempt began where its driver spawned, once the sink
    /// `settled`: every member's cleanup witnessed on the lease first, and
    /// each settled member's own settlement record sealed (U8e2), so no
    /// member's evidence is read while a sibling's broker may still write
    /// (#403; D7), then each report carrying what its checkpoints left of
    /// it and settled by its own broker calls ([`Engine::settled_by`])
    /// before any is admitted. Each member's admitted findings are handed
    /// back typed beside the reports, by member, never in its result's
    /// `Value`. A sink that failed is handed up once every member is
    /// witnessed.
    pub(super) fn settle_members(
        &mut self,
        ids: [&str; 2],
        runs: &[MemberRun],
        reports: Vec<(String, AttemptReport, Option<Begun>)>,
        settled: Result<checkpoints::Settled, EngineError>,
        tag_prefix: &str,
    ) -> Result<Members, EngineError> {
        for ((_, report, _), run) in reports.iter().zip(runs) {
            ended(&mut self.held, &run.driver_seat, &report.cleanup);
        }
        let settled = settled?;
        let mut members = Vec::with_capacity(reports.len());
        let mut stamped = Stamped::new();
        for ((name, mut report, begun), run) in reports.into_iter().zip(runs) {
            settled.carry(&Tag::prefixed(tag_prefix, &name).0, &mut report);
            // Each member's findings carry the member's own site.
            let report = self.settled_by(ids, &run.spawn, begun, report)?;
            let admitted = super::verdict::judged(report, run.context.as_ref(), &run.driver_seat);
            if let Some(findings) = admitted.findings {
                stamped.insert(name.clone(), MemberFindings::Read(findings));
            }
            members.push((name, admitted.report));
        }
        Ok((members, stamped))
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
            let run = runs.iter().find(|run| run.name == *name);
            let boundary = run.and_then(|run| run.boundary);
            // The member's structural site, the one its findings are
            // stamped with, so a reader files each finding under the
            // member that reported it even when the member journaled no
            // checkpoint of its own (fix round 0109a1r3, M3).
            let site = run.and_then(|run| run.context.as_ref());
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
            let mut marker = EngineMarker::MemberFinished.record(json!({
                "member": Tag::prefixed(tag_prefix, name),
                "outcome": kind,
                "model": model,
                "session_ref": report.session_ref,
                "inner_checkpoints": report.checkpoints.len(),
            }));
            if let Some(site) = site {
                marker["site_ref"] = Value::String(site.site_ref.clone());
            }
            let marker = stamp_boundary(marker, boundary);
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
        .map(|(name, _)| Some(Tag::prefixed(tag_prefix, name).0))
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
    /// arrives; why its driver did not spawn, where it did not
    /// ([`unspawned`]). The door is handed the bundle, the lease the
    /// attempt was admitted under (U8a2a) and where this member's fold
    /// begins, which its settlement record keeps (U8e2).
    fn report(
        &self,
        (bundle, held, begun): (&Bundle, Option<&Held>, Option<&Begun>),
        workdir: &Path,
        deadline: std::time::Duration,
        effect_id: &str,
        attempt_id: &str,
        on_checkpoint: impl FnMut(&Value),
    ) -> DriverRun {
        let door = held.map(|held| Door::of(held, &self.driver_seat, begun));
        let spawned = spawn_site(bundle, &self.spawn, &self.input, workdir, deadline, door);
        // The door's own input, carrying the charter text it verified for
        // this member.
        let (process, input) = match spawned {
            Ok(spawned) => spawned,
            Err(refused) => return DriverRun::SpawnFailed(refused),
        };
        DriverRun::Ran(super::received(process.run_attempt_resuming(
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
        )))
    }

    /// What this member's own harness declares (decision 0075 ruling 2).
    fn kinds(&self) -> &SeatKinds {
        (self.context.as_ref()).map_or(&UNDECLARED, |context| &context.kinds)
    }
}

/// The report of a member whose driver did not spawn, by `error`: a
/// failure that started nothing.
fn unspawned(error: &str) -> AttemptReport {
    AttemptReport {
        refusal: None,
        outcome: AttemptOutcome::Failed {
            error: format!("member driver did not spawn: {error}"),
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
        usage: None,
    }
}

/// Offer one live member checkpoint to the panel's sink, on the main
/// thread: read by the call authority of the member it is tagged with,
/// stamped with that member's boundary and structural identity, and
/// admitted by the kinds that member's own harness declares.
fn offer_member_checkpoint(
    sink: &mut Checkpoints<'_, fn(&mut Store)>,
    (runs, calls, tag_prefix): (&[MemberRun], &mut [Calls], &str),
    member: &str,
    mut checkpoint: Value,
) {
    let at = runs
        .iter()
        .position(|run| Tag::prefixed(tag_prefix, &run.name).0 == member);
    let owner = at.map(|at| &runs[at]);
    let seat = owner.map_or(member, |run| run.driver_seat.as_str());
    let kinds = owner.map_or(&UNDECLARED, MemberRun::kinds);
    let declared = checkpoints::Declared { kinds, seat };
    let call = at.map_or(Ok(None), |at| calls[at].consume(&mut checkpoint));
    let boundary = owner.and_then(|run| run.boundary);
    let checkpoint = stamp_boundary(tag_member(checkpoint, member), boundary);
    // The member's own stamp, applied by the one journal
    // writer from the context its worker handed over.
    let context = owner.and_then(|run| run.context.as_ref());
    let checkpoint = match context {
        Some(context) => context.stamp(checkpoint),
        None => resume::unstamped(checkpoint),
    };
    sink.offer_live(member, checkpoint, call, context, declared);
}

/// Join member reports into the attempt's single outcome: any
/// indeterminate member parks the whole attempt; otherwise any failed
/// member fails it (retryable under 0006); otherwise the declared
/// aggregate produces the one typed result.
pub(super) fn panel_outcome(
    join: &PanelJoin,
    reports: Vec<(String, AttemptReport)>,
    stamped: Stamped,
) -> AttemptOutcome {
    outcome_over(
        join,
        reports
            .into_iter()
            .map(|(name, report)| (name, report.settled_outcome()))
            .collect(),
        stamped,
    )
}

/// Each panel member's findings, typed, by member name: what admission
/// stamped for a fresh member, or what a carried member's journaled
/// result records. A member absent here reported none.
pub(crate) type Stamped = BTreeMap<String, MemberFindings>;

/// A panel's settled members: their reports in declared order, and their
/// findings, typed, beside them.
pub(crate) type Members = (Vec<(String, AttemptReport)>, Stamped);

pub(super) fn outcome_over(
    join: &PanelJoin,
    reports: Vec<(String, AttemptOutcome)>,
    mut stamped: Stamped,
) -> AttemptOutcome {
    let mut indeterminate = Vec::new();
    let mut failures = Vec::new();
    let mut member_results = Vec::new();
    for (name, report) in reports {
        match report {
            AttemptOutcome::Succeeded { result } => {
                let findings = stamped
                    .remove(&name)
                    .unwrap_or(MemberFindings::Read(Vec::new()));
                member_results.push(MemberVerdict {
                    name,
                    payload: result,
                    findings,
                });
            }
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
        result: join.aggregate_results(&member_results),
    }
}

impl super::SitePlan {
    pub(super) fn attach_resume_context(&self, input: &mut Value) {
        input["resume_context"] = resume::start_context(
            self.assessment.clone(),
            self.originating.as_ref(),
            self.offer.as_ref(),
            self.route_overlay.as_ref(),
            self.context.kinds.declared_transcript_kind().as_ref(),
        );
    }
}
