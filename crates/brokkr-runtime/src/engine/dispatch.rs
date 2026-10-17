//! The dispatch of one requested effect: its rebuilt input checked against
//! the digest recorded at request time, the attempt planned from journaled
//! facts, started durably, provisioned, and its selected body run (#288).
//!
//! [`Engine::execute`] reads as that sequence, in order; the pieces it is
//! split into stand beside it here, in the order it reaches them.

use std::time::Duration;

use brokkr_core::envelope::EventType;
use brokkr_core::fold::RunState;
use brokkr_core::EventEnvelope;
use serde_json::{json, Value};
use uuid::Uuid;

use super::{
    argv_for, arms_effect_gate_head, git_head, phase_of, select_candidates, Engine, EngineError,
    Selection, SiteSpawn, RESULTS,
};
use crate::bundle::{ExecutableBody, Invocation};

/// One attempt of a requested effect, as [`Engine::execute`] names it
/// before anything spawns: the effect, the attempt, the seat and the case
/// its selector chose, the site label it is composed and marked under,
/// the selected body, and whether that body is a gate.
struct Dispatch<'a> {
    effect_id: &'a str,
    attempt_id: &'a str,
    seat_name: &'a str,
    case: Option<&'a str>,
    site_name: &'a str,
    body: ExecutableBody<'a>,
    gate: bool,
}

impl Engine {
    pub(super) fn execute(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat_name: &str,
    ) -> Result<(), EngineError> {
        let phase = phase_of(state, "effect without a phase")?;
        let requested_digest = requested_digest(events, effect_id)?;
        let input = self.seat_input(state, &phase, effect_id)?;
        if brokkr_core::canonical::sha256_hex(&input) != requested_digest {
            return self.refuse_changed_input(effect_id);
        }

        // The journal is an execution resource, not part of the durable seat
        // request: equivalent --db spellings on resume must rebuild the same
        // input digest. Only the deterministic ship script receives it.
        let mut input = input;
        if phase == "ship" {
            let journal =
                std::fs::canonicalize(self.store.path()).unwrap_or(self.store.path().to_path_buf());
            input["context"]["journal"] = Value::String(journal.to_string_lossy().into_owned());
        }

        let seat = self.bundle.seats[seat_name].clone();
        // `seat_input` above has already refused an unresolved selector
        // against this same folded state.
        let (body, selected_case) = seat
            .body
            .selected(state.strategy.as_deref())
            .expect("seat input resolved the selector");
        let site_name = selected_case
            .map(|case| format!("{seat_name}:{case}"))
            .unwrap_or_else(|| seat_name.to_string());
        let attempt_id = Uuid::new_v4().to_string();
        let at = Dispatch {
            effect_id,
            attempt_id: &attempt_id,
            seat_name,
            case: selected_case,
            site_name: &site_name,
            body,
            gate: seat
                .body
                .selected_is_gate(state.strategy.as_deref(), seat.has_gate),
        };
        let (mut selection, mut single, started) = self.plan_attempt(events, &at, &mut input)?;
        // started is durable BEFORE the driver spawns: a crash in between
        // recovers as indeterminate, never as a silent double-execution.
        if arms_effect_gate_head(&body, &seat, state.strategy.as_deref()) {
            self.active_gate_head = Some(git_head(&self.repo));
        }
        self.append(EventType::EffectStarted, started, Some(attempt_id.clone()))?;
        std::fs::create_dir_all(self.run_local().join(RESULTS)).ok();
        let mut deadline = Duration::from_secs(seat.limits.timeout_seconds);
        let ids = [effect_id, attempt_id.as_str(), site_name.as_str()];
        if !self.provision(ids, &mut selection, &mut deadline)? {
            return Ok(());
        }
        // Sealed once its plans are provisioned, as members and steps are
        // (U7d2): the brokers it is served with are their pins.
        let brokers = selection.brokers(&site_name);
        let link = selection.get(&None);
        self.mark_serving(&site_name, link, brokers, single.as_mut(), &mut input);
        self.run_body(&at, input, deadline, &selection, single)
    }

    /// The world changed between request and execution (bundle edit,
    /// repo move). Never run something other than what was requested:
    /// the attempt starts on no driver and fails, saying why.
    fn refuse_changed_input(&mut self, effect_id: &str) -> Result<(), EngineError> {
        let attempt_id = Uuid::new_v4().to_string();
        self.append(
            EventType::EffectStarted,
            json!({"effect_id": effect_id, "attempt_id": attempt_id, "driver": "none"}),
            Some(attempt_id.clone()),
        )?;
        self.append(
            EventType::EffectFailed,
            json!({
                "effect_id": effect_id,
                "attempt_id": attempt_id,
                "error": "rebuilt seat input does not match the digest recorded \
                          at request time; refusing to execute a different effect",
            }),
            Some(attempt_id),
        )?;
        Ok(())
    }

    /// Everything the attempt decides before its durable start: the
    /// selection, the single driver's composed spawn, the executing
    /// site's marks on `input`, and the `started` payload, with each
    /// site's resume plan set on the selection.
    fn plan_attempt(
        &mut self,
        events: &[EventEnvelope],
        at: &Dispatch<'_>,
        input: &mut Value,
    ) -> Result<(Selection, Option<SiteSpawn>, Value), EngineError> {
        let driver_label = driver_label(at.body);
        // Which link of each agent's chain runs this attempt, decided
        // from journaled facts before anything spawns. The existing
        // `driver` label is untouched: a display string is not a control
        // channel, and five consumers plus the engine would otherwise
        // have to parse a packed grammar to make a control decision.
        let (mut selection, provenance, chains) = select_candidates(events, at.effect_id, at.body);
        // The single-driver argv is composed HERE, ahead of the durable
        // start, because the session question is asked of it: which
        // instance this attempt resolves to is part of what decides
        // whether a prior session may be handed back to it.
        let runtime_hands = self.runtime_hands(at.site_name);
        let single = match at.body {
            ExecutableBody::Invocation(Invocation::Single { command, .. }) => {
                Some(self.compose_at(
                    Some(at.site_name),
                    at.attempt_id,
                    at.gate,
                    argv_for(&selection, &None, command).to_vec(),
                    runtime_hands.as_ref(),
                    selection.get(&None),
                    input["result_path"].as_str().unwrap_or_default(),
                ))
            }
            _ => None,
        };
        // The delivery door is a spawn-time fact of the selected link
        // (decision 0046 ruling 4): it rides the input the driver reads,
        // beside the journal path a ship seat receives, and outside the
        // requested digest — a chain fallback moves the door, and a
        // digest that moved with it would refuse the retry as a
        // different effect.
        //
        // The executing single's confinement markers are composed HERE,
        // from the selected label rather than the seat's phase label
        // (design D10 F1): `seat_input` marked the phase, and a selector's
        // phase label owns no facts, so re-marking `site_name` is what
        // publishes the selected body's own boundary and hands. It sits
        // beside the delivery door, after the requested digest was checked,
        // so a chain fallback or a selector move cannot refuse the retry
        // as a different effect.
        self.marks()
            .site(at.site_name, at.gate, selection.get(&None), input);
        let mut started = json!({
            "effect_id": at.effect_id,
            "attempt_id": at.attempt_id,
            "driver": driver_label,
        });
        self.record_sites(&mut started, provenance, at.body, at.site_name, at.gate);
        // Which session — if any — each EXECUTING MODEL SITE of this
        // attempt may rejoin, decided from journaled facts before
        // anything spawns (decision 0030, generalised by proposed
        // decision 0056 ruling 1 to all four work topologies). The
        // `started` payload is what the legacy comparison is judged
        // against, which is why the question is asked here, after the
        // driver label and the provenance are in it.
        //
        // The offer itself adds no field to the payload. It is a pure
        // function of this journal and the pinned bundle, so recording
        // it would record a derivation, not a fact — and the fact, what
        // the driver DID with the offer, is the driver's own checkpoint
        // to journal. The engine widens no event vocabulary to say
        // something the record already answers.
        let started_here = self.store.started_here(&self.run_id)?;
        selection.plans = self.site_plans(
            events,
            at.body,
            at.seat_name,
            at.case,
            at.site_name,
            at.gate,
            &selection,
            &chains,
            &started,
            started_here,
        );
        Ok((selection, single, started))
    }

    /// Run the attempt's selected body: a panel's members, a sequence's
    /// steps, or the single driver [`Engine::plan_attempt`] composed.
    fn run_body(
        &mut self,
        at: &Dispatch<'_>,
        input: Value,
        deadline: Duration,
        selection: &Selection,
        single: Option<SiteSpawn>,
    ) -> Result<(), EngineError> {
        match at.body {
            ExecutableBody::Invocation(Invocation::Panel { members, aggregate }) => self
                .execute_panel(
                    at.effect_id,
                    at.attempt_id,
                    at.site_name,
                    members,
                    *aggregate,
                    &input,
                    deadline,
                    selection,
                    at.gate,
                ),
            ExecutableBody::Sequence { steps } => self.execute_sequence(
                at.effect_id,
                at.attempt_id,
                at.site_name,
                steps,
                &input,
                deadline,
                selection,
            ),
            // Agents choose the argv (model selection); composition
            // decides what is mounted — a composed bundle spans every
            // recipe directory in its chain, not one dir. Both were
            // settled above, where the session question needed them.
            ExecutableBody::Invocation(Invocation::Single { .. }) => {
                let spawn = single.expect("a single seat composed its command");
                let plan = selection.plan(&None);
                let run = self.run_driver(
                    at.effect_id,
                    at.attempt_id,
                    at.site_name,
                    &spawn,
                    input,
                    deadline,
                    None,
                    plan,
                )?;
                let boundary = self.site_boundary(at.site_name);
                self.conclude_single(at.effect_id, at.attempt_id, run, selection, boundary)
            }
        }
    }
}

/// The input digest recorded when `effect_id` was requested.
fn requested_digest(events: &[EventEnvelope], effect_id: &str) -> Result<String, EngineError> {
    events
        .iter()
        .rev()
        .find(|e| {
            e.event_type == EventType::EffectRequested
                && e.payload.get("effect_id").and_then(Value::as_str) == Some(effect_id)
        })
        .and_then(|e| e.payload.get("input_digest").and_then(Value::as_str))
        .map(str::to_string)
        .ok_or_else(|| {
            let detail = format!("no requested event for {effect_id}");
            EngineError::Invariant("effect requested", detail)
        })
}

/// The attempt's `driver` label, as every consumer displays it.
fn driver_label(body: ExecutableBody<'_>) -> String {
    match body {
        ExecutableBody::Invocation(Invocation::Single { command, .. }) => command[0].clone(),
        ExecutableBody::Invocation(Invocation::Panel { members, aggregate }) => {
            format!("panel[{}]:{aggregate:?}", members.len())
        }
        ExecutableBody::Sequence { steps } => format!("sequence[{}]", steps.len()),
    }
}
