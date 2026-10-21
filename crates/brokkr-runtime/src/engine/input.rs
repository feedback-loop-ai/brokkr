//! A seat's requested input: the one document the effect's digest covers
//! and recovery rebuilds, composed from the journal, the pinned bundle and
//! the feature alone (#288).
//!
//! [`Engine::seat_input`] resolves the seat and its body, then composes in
//! steps: [`Engine::seat_context`] the context the seat reads,
//! [`Engine::body_input`] the shape its body runs in, and the marks, the
//! sealed secret bindings and the house rules beside them.

use std::path::{Path, PathBuf};

use brokkr_core::fold::RunState;
use serde_json::{json, Map, Value};

use super::sequence::nearest_change;
use super::{Engine, EngineError, RESULTS};
use crate::bundle::{ExecutableBody, Invocation, PanelMember, Seat, SequenceStep, StepBody};
use crate::phase_name;

/// Each result file the input names: `<effect><suffix>.json` in the
/// run-local `results/`, and a panel's members each under theirs.
struct ResultFiles<'a> {
    dir: PathBuf,
    effect_id: &'a str,
}

impl ResultFiles<'_> {
    fn result(&self, suffix: String) -> String {
        let file = self.dir.join(format!("{}{suffix}.json", self.effect_id));
        file.to_string_lossy().into_owned()
    }

    fn panel(&self, members: &[PanelMember], prefix: String) -> Value {
        let entry = |member: &PanelMember| {
            let path = self.result(format!("{prefix}-{}", member.name));
            let role = member.role_path.to_string_lossy();
            (
                member.name.clone(),
                json!({"role_path": role, "result_path": path}),
            )
        };
        Value::Object(members.iter().map(entry).collect())
    }
}

impl Engine {
    /// Seat input is a pure function of (journal, pinned bundle, feature):
    /// recovery rebuilds it and the digest recorded at request time must
    /// match, or the run parks instead of running something else.
    pub(super) fn seat_input(
        &self,
        state: &RunState,
        phase: &str,
        effect_id: &str,
    ) -> Result<Value, EngineError> {
        let seat = self.bundle.seats.get(phase).ok_or_else(|| {
            let detail = format!("no seat for phase '{phase}' (compile enforces this)");
            EngineError::Invariant("phase has seat", detail)
        })?;
        let (body, _) = seat
            .body
            .selected(state.strategy.as_deref())
            .ok_or_else(|| {
                let detail = format!("seat '{phase}' selector has no resolved body");
                EngineError::Invariant("selector has body", detail)
            })?;
        let workdir = self.workdir();
        let files = ResultFiles {
            dir: self.run_local().join(RESULTS),
            effect_id,
        };
        let context = self.seat_context(state, phase);
        let has_change = nearest_change(&context).is_some();
        let mut input = self.body_input(phase, seat, body, &workdir, context, &files);
        // The mark is part of the requested input, so the digest covers it;
        // a panel or sequence seat has no hands of its own at this label.
        self.mark_hands(phase, &mut input);
        // Sealed secret bindings (decision 0012): the engine threads
        // exactly two facts to the driver — the declared NAMES and the
        // store PATH, both journal-safe. Values are resolved at spawn
        // time inside the exec driver; no store read exists anywhere in
        // brokkr-runtime. Absent when the seat binds nothing, so
        // pre-0012 bundles rebuild byte-identical seat inputs.
        if !seat.secrets.is_empty() {
            input["secrets"] = json!(seat.secrets);
            input["secrets_file"] = json!(self.secrets_store_path().to_string_lossy());
        }
        if let Some(world) = &self.world {
            if let Some(house) = world.house_for(&self.repo)? {
                input["house_rules"] = json!(house);
            }
        }
        self.marks().seat_dialect(phase, has_change, &mut input);
        Ok(input)
    }

    /// The context a seat reads: the run, the decision that brought it
    /// here, the results of the phases before it and, on a return, the
    /// result that sent it back.
    fn seat_context(&self, state: &RunState, phase: &str) -> Value {
        let mut context = Map::new();
        context.insert("run_id".into(), json!(self.run_id));
        // Triage is the fresh-and-blind chief's office (decision 0041
        // ruling 6). Even on the bounded oversized return it receives
        // the commission and current tree, not journal history.
        if phase != phase_name::TRIAGE {
            context.insert("last_decision".into(), json!(state.last_decision));
            if let Some(entry) = &self.entry {
                context.insert("entry".into(), json!(entry));
            }
        }
        let current = self
            .bundle
            .machine
            .phases
            .iter()
            .position(|known| known == phase)
            .unwrap_or_default();
        let results: Map<String, Value> = self.bundle.machine.phases[..current]
            .iter()
            .filter_map(|earlier| {
                state
                    .phase_results
                    .get(earlier)
                    .map(|result| (earlier.clone(), json!(result)))
            })
            .collect();
        if !results.is_empty() {
            context.insert("results".into(), Value::Object(results));
        }
        // Reforging (decision 0022): a seat the run RETURNS to receives
        // the result that sent it back — the review's findings,
        // severities and notes reach the implementer who has to answer
        // them, because a precise finding is only useful to whoever
        // reads it. A seat on its FIRST visit of the run gets nothing
        // new, so a run that never revisits builds the input, and the
        // digest, it always built.
        let returned = self.entry.is_some()
            && state.last_decision.as_ref().is_some_and(|decision| {
                brokkr_core::policy::Direction::of(
                    &self.bundle.machine.phases,
                    &decision.from,
                    phase,
                ) == Some(brokkr_core::policy::Direction::Return)
            });
        if phase != phase_name::TRIAGE
            && (state.visits.get(phase).copied().unwrap_or(0) > 1 || returned)
        {
            context.insert(
                "returned_from".into(),
                json!({
                    "phase": state.last_decision.as_ref().map(|d| &d.from),
                    "result": state.last_result,
                }),
            );
        }
        Value::Object(context)
    }

    /// The input in the shape the seat's selected body runs in: a single
    /// driver, a panel's members, or a sequence's steps.
    fn body_input(
        &self,
        phase: &str,
        seat: &Seat,
        body: ExecutableBody<'_>,
        workdir: &Path,
        context: Value,
        files: &ResultFiles<'_>,
    ) -> Value {
        match body {
            ExecutableBody::Invocation(Invocation::Single { role_path, .. }) => json!({
                "feature": self.feature,
                "phase": phase,
                "seat": phase,
                "role_path": role_path.to_string_lossy(),
                "workdir": workdir.to_string_lossy(),
                "result_path": files.result(String::new()),
                "allowed_results": seat.results,
                "context": context,
            }),
            ExecutableBody::Invocation(Invocation::Panel { members, aggregate }) => json!({
                "feature": self.feature,
                "phase": phase,
                "seat": phase,
                "aggregate": format!("{aggregate:?}"),
                "workdir": workdir.to_string_lossy(),
                "members": files.panel(members, String::new()),
                "allowed_results": seat.results,
                "context": context,
            }),
            ExecutableBody::Sequence { steps } => {
                // The requested-time input enumerates the WHOLE sequence;
                // per-step driver inputs are derived from it
                // deterministically at execution time. Step ORDER is
                // load-bearing (steps run serially, and the digest must
                // rebuild identically): an array carries it — a JSON
                // object would sort its keys.
                let step_values: Vec<Value> =
                    steps.iter().map(|step| step_input(step, files)).collect();
                json!({
                    "feature": self.feature,
                    "phase": phase,
                    "seat": phase,
                    "workdir": workdir.to_string_lossy(),
                    "steps": step_values,
                    "allowed_results": seat.results,
                    "context": context,
                })
            }
        }
    }
}

/// One sequence step's entry in the requested input, in its form.
fn step_input(step: &SequenceStep, files: &ResultFiles<'_>) -> Value {
    match &step.body {
        StepBody::Invocation(Invocation::Single { role_path, .. }) => json!({
            "name": step.name,
            "allowed_results": step.results,
            "role_path": role_path.to_string_lossy(),
            "result_path": files.result(format!("-{}", step.name)),
        }),
        StepBody::Invocation(Invocation::Panel { members, aggregate }) => json!({
            "name": step.name,
            "allowed_results": step.results,
            "aggregate": format!("{aggregate:?}"),
            "members": files.panel(members, format!("-{}", step.name)),
        }),
        StepBody::Dialect { execution } => json!({
            "name": step.name,
            "allowed_results": step.results,
            "role_path": "",
            "result_path": files.result(format!("-{}", step.name)),
            "dialect": {"argv": execution.argv, "state": execution.state},
        }),
    }
}
