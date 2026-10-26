//! A seat's result, decided: checked against its schema, its declared
//! inputs joined to the facts the journal and the repository supply, and
//! the policy table's ruling journaled as the one transition (#288).
//!
//! [`Engine::decide`] parks a result that fails [`schema_problem`], or a
//! clean review that reports a residual at the protected phase, and
//! otherwise journals [`Engine::ruling`] over [`Engine::decision_inputs`],
//! held to the [`VerifiedCandidate`] a passing verification bound.

use std::path::Path;

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{computed_inputs, RunState};
use brokkr_core::policy::{Outcome, CONSECUTIVE_FAILURES, VISIT_PREFIX};
use brokkr_core::realms::LEGACY_REALM_KEY;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::worktree::{git_candidate, read_tree_counting, TreeReading, Untracked};
use super::{
    artifact_failures, artifact_problem, docs_only_between, git_dirty, git_head, phase_of, Engine,
    EngineError,
};
use crate::bundle::{Seat, REALM_FACTS};
use crate::phase_name;

/// The engine-owned record a passing verification writes into its
/// decision's inputs, which the protected phase and ship are held to
/// (decision 0097 ruling 3): the name core's input table owns, as
/// `reviewed_heads` is read above by the name the fold owns.
const VERIFIED_CANDIDATE: &str = "verified_candidate";

/// The immutable Git candidate a verification passed: HEAD and the tree
/// it identifies, read from a worktree with nothing uncommitted.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifiedCandidate {
    head: String,
    tree: String,
}

/// What a ruling owes the verified candidate: a passing verification
/// binds it, the protected phase and ship are held to it, and any other
/// ruling owes nothing.
#[derive(Debug, Clone, Copy)]
enum CandidateDuty {
    Bind,
    Hold,
    None,
}

/// What the worktree answers for a candidate: one verification can bind,
/// a tree with something uncommitted, or no commit to name at all (no
/// repository, an empty one, or no git), which binds nothing and is held
/// to nothing, as `reviewed_heads` is recorded only where a repository
/// answers.
#[derive(Debug)]
enum CandidateReading {
    Verified(VerifiedCandidate),
    Unclean,
    NoCommit,
}

impl Engine {
    pub(super) fn decide(
        &mut self,
        state: &RunState,
        raw_result: Value,
    ) -> Result<(), EngineError> {
        let phase = phase_of(state, "decide without a phase")?;
        let seat = &self.bundle.seats[&phase];

        // Result schema (decision 0001): an object with a declared result
        // string. Violations park with the raw evidence attached — never
        // repaired, coerced, or handed to a model to fix.
        if let Some(problem) = schema_problem(seat, &raw_result).or_else(|| {
            (phase == self.bundle.protected_phase)
                .then(|| crate::bundle::Aggregate::review_problem(&raw_result))
                .flatten()
        }) {
            self.append(
                EventType::TransitionDecided,
                json!({
                    "from": phase,
                    "result": "__schema-invalid__",
                    "rule_id": null,
                    "next": null,
                    "severity": null,
                    "inputs": {"raw_result": raw_result},
                    "problem": format!("result failed schema validation: {problem}"),
                }),
                None,
            )?;
            return Ok(());
        }

        let object = raw_result.as_object().expect("checked above");
        let result = object["result"]
            .as_str()
            .expect("checked above")
            .to_string();
        let inputs = self.decision_inputs(state, &phase, seat, object, &result);
        let mut payload = self.ruling(&phase, &result, &inputs, object);
        self.bind_verified_candidate(state, &phase, &mut payload);
        self.append(EventType::TransitionDecided, payload, None)?;
        Ok(())
    }

    /// A passing verification binds the immutable Git candidate in the
    /// journal, not a seat-writable sidecar, and the protected phase and
    /// ship are held to it (decision 0097 ruling 3). The table determines
    /// what passes: no verifier result word is assumed.
    fn bind_verified_candidate(&self, state: &RunState, phase: &str, payload: &mut Value) {
        match self.candidate_duty(phase, payload) {
            CandidateDuty::Bind => match candidate_reading(&self.repo) {
                CandidateReading::Verified(candidate) => {
                    payload["inputs"][VERIFIED_CANDIDATE] = json!(candidate);
                }
                CandidateReading::Unclean => park(payload, "verification candidate is dirty"),
                CandidateReading::NoCommit => {}
            },
            CandidateDuty::Hold => self.hold_to_verified_candidate(state, payload),
            CandidateDuty::None => {}
        }
    }

    /// A ruling that takes no transition, or a hard one, admits nothing,
    /// so it binds and is held to nothing. Verify binds when it hands the
    /// candidate to the protected phase, or advances at all where it is
    /// itself the protected phase. A bundle with no verify seat has no
    /// verification to bind.
    fn candidate_duty(&self, phase: &str, payload: &Value) -> CandidateDuty {
        if !self.bundle.seats.contains_key(phase_name::VERIFY)
            || payload["next"].is_null()
            || payload["severity"] == "hard"
        {
            return CandidateDuty::None;
        }
        let protected = self.bundle.protected_phase.as_str();
        if phase == phase_name::VERIFY {
            if payload["next"] == protected || phase == protected {
                CandidateDuty::Bind
            } else {
                CandidateDuty::None
            }
        } else if phase == protected || phase == phase_name::SHIP {
            CandidateDuty::Hold
        } else {
            CandidateDuty::None
        }
    }

    /// What is judged or shipped is what verification passed, byte for
    /// byte, or that candidate under a delta the engine itself classifies
    /// as the repository's docs class at the verified head: the smith's
    /// docs return re-enters the protected phase without verify (decision
    /// 0041 ruling 5e) on the same classification. A run the table routed
    /// past verify is the table's to route, and so is one whose worktree
    /// names no commit; one verify ruled on in a repository carries a
    /// readable witness, or it parks.
    fn hold_to_verified_candidate(&self, state: &RunState, payload: &mut Value) {
        let Some(verified) = state.phase_results.get(phase_name::VERIFY) else {
            return;
        };
        let expected = verified
            .inputs
            .as_ref()
            .and_then(|inputs| inputs.get(VERIFIED_CANDIDATE))
            .and_then(|witness| serde_json::from_value::<VerifiedCandidate>(witness.clone()).ok());
        match (expected, candidate_reading(&self.repo)) {
            (None, CandidateReading::NoCommit) => {}
            (None, _) => park(
                payload,
                "verified candidate witness is missing or unreadable",
            ),
            (Some(expected), CandidateReading::Verified(found))
                if found == expected
                    || docs_only_between(&self.repo, &expected.head, &found.head) == Some(true) => {
            }
            (Some(_), _) => park(
                payload,
                "candidate differs from the journaled verified HEAD/tree or is dirty",
            ),
        }
    }

    /// The inputs the table rules on: the seat's declared inputs, and
    /// over them the facts the journal computes and the repository
    /// answers, none of which a seat may claim.
    fn decision_inputs(
        &self,
        state: &RunState,
        phase: &str,
        seat: &Seat,
        object: &Map<String, Value>,
        result: &str,
    ) -> Map<String, Value> {
        // Seat-supplied facts: everything the engine owns is dropped, and
        // only the seat's DECLARED inputs survive (decision 0007) — an
        // undeclared claim never reaches the table or the journal record.
        let mut inputs: Map<String, Value> = object
            .get("inputs")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        inputs.retain(|key, _| !crate::bundle::is_engine_owned(key));
        inputs.retain(|key, _| seat.inputs.iter().any(|declared| declared == key));
        // Journal-computed inputs overlay (never accepted from the seat).
        let counter_read = self
            .bundle
            .machine
            .reads_counter(phase, CONSECUTIVE_FAILURES);
        for (key, value) in computed_inputs(state, phase, result, counter_read) {
            inputs.insert(key, value);
        }
        // The phase-visit predicate (decision 0022), supplied for exactly
        // the phases this phase's rules ask about — counted from
        // `phase/entered`, never claimed by a seat.
        for visited in self.bundle.machine.visit_phases(phase) {
            let count = state.visits.get(&visited).copied().unwrap_or(0);
            inputs.insert(format!("{VISIT_PREFIX}{visited}"), Value::from(count));
        }
        self.repository_inputs(state, phase, &mut inputs);
        inputs
    }

    /// The facts the repository answers at decide time: the reviewed
    /// heads and the docs-only shortcut where a phase asks for them, and
    /// a ship's worktree and drift facts.
    fn repository_inputs(&self, state: &RunState, phase: &str, inputs: &mut Map<String, Value>) {
        let repo = self.repo.as_path();
        // The realm this repository IS, when a map named it
        // (decision 0023): repository facts are recorded under the
        // realm's name — the shape the heritage protocol recorded and
        // the shape multi-realm runs will need. Unmapped, they are
        // recorded exactly as they always were.
        let realm = self
            .world
            .as_ref()
            .and_then(|world| world.realm_for(repo))
            .map(|realm| realm.name.clone());
        let key = realm
            .clone()
            .unwrap_or_else(|| LEGACY_REALM_KEY.to_string());
        if phase == self.bundle.protected_phase {
            if let Some(head) = git_head(repo) {
                inputs.insert("reviewed_heads".into(), json!({ key: &head }));
                if let Some(docs_only) = self.fixes_docs_only(repo, phase, &head) {
                    inputs.insert("fixes_docs_only".into(), Value::Bool(docs_only));
                }
            }
        }
        // Decision 0041 ruling 5: the smith owns review findings. Expose
        // the docs-only shortcut only when review sent this implement
        // visit back; verify failures and implement self-loops must still
        // pass through verify even when their delta happens to be prose.
        let returned_from_review = phase == phase_name::IMPLEMENT
            && state
                .last_decision
                .as_ref()
                .map(|decision| decision.from.as_str())
                == Some(self.bundle.protected_phase.as_str());
        if returned_from_review {
            if let Some(head) = git_head(repo) {
                if let Some(docs_only) = self.fixes_docs_only(repo, phase, &head) {
                    inputs.insert("fixes_docs_only".into(), Value::Bool(docs_only));
                }
            }
        }
        if phase == phase_name::SHIP {
            ship_inputs(state, repo, realm.as_deref(), inputs);
        }
    }

    /// The decision payload: the table's ruling on (`phase`, `result`)
    /// over `inputs`, an advancing ruling held to its artifact gate.
    fn ruling(
        &self,
        phase: &str,
        result: &str,
        inputs: &Map<String, Value>,
        object: &Map<String, Value>,
    ) -> Value {
        // No ruling: the pair matched no rule, or the machine refused one.
        let unruled = |problem: String| {
            json!({"from": phase, "result": result, "rule_id": null, "next": null,
                   "severity": null, "inputs": inputs, "problem": problem})
        };
        match self.bundle.machine.evaluate(phase, result, inputs) {
            Outcome::Ruling {
                rule_id,
                next_phase,
                severity,
                requires_artifacts,
                ..
            } => {
                // Artifact gate: an advancing ruling that names artifacts
                // advances only if each exists as a non-empty regular file
                // in the workdir, probed exactly once, here, at decide
                // time. A miss fails closed through the park path — no
                // seat is asked, no seat attests (decision 0001).
                let failures = artifact_failures(&self.workdir(), &requires_artifacts);
                if failures.is_empty() {
                    json!({
                        "from": phase,
                        "result": result,
                        "rule_id": rule_id,
                        "next": next_phase,
                        "severity": severity,
                        "inputs": inputs,
                        "problem": null,
                    })
                } else {
                    // Severity is a property of a taken transition; none
                    // is taken. rule_id stays: the rule DID match, and
                    // that identity distinguishes a gate block from
                    // an unmatched pair in the journal.
                    json!({
                        "from": phase,
                        "result": result,
                        "rule_id": rule_id,
                        "next": null,
                        "severity": null,
                        "inputs": inputs,
                        "problem": artifact_problem(&rule_id, &failures),
                    })
                }
            }
            // A rule-driven park (decision 0022). The v1 event vocabulary
            // already spells it: a decision with a rule_id and no next is
            // exactly "a rule matched and no transition was taken" — the
            // shape the artifact gate writes. `severity` stays null
            // because severity is a property of a taken transition.
            Outcome::Park { rule_id, reason } => json!({
                "from": phase,
                "result": result,
                "rule_id": rule_id,
                "next": null,
                "severity": null,
                "inputs": inputs,
                "problem": park_problem(object, &reason),
            }),
            Outcome::Unmatched => unruled("no rule matched".to_string()),
            Outcome::Refused { problem } => unruled(problem.to_string()),
        }
    }
}

/// A rule-driven park's problem (decision 0092 ruling 3): the parked
/// seat's own non-empty notes, which say what the operator must answer
/// (the escalation's reasoning, the environment that blocked a seat),
/// else the rule's reason. The fold prefixes the rule's id either way.
fn park_problem<'a>(object: &'a Map<String, Value>, reason: &'a str) -> &'a str {
    object
        .get("notes")
        .and_then(Value::as_str)
        .filter(|notes| !notes.is_empty())
        .unwrap_or(reason)
}

/// Why `raw_result` fails the seat's result schema (decision 0001): an
/// object with a declared result string, and a declared `change` input
/// spelled as an identifier. `None` when it passes.
fn schema_problem(seat: &Seat, raw_result: &Value) -> Option<String> {
    match raw_result.as_object() {
        None => Some("seat result is not an object".to_string()),
        Some(object) => match object.get("result").and_then(Value::as_str) {
            None => Some("seat result has no 'result' string".to_string()),
            Some(r) if !seat.results.iter().any(|allowed| allowed == r) => Some(format!(
                "seat result '{r}' is not among declared results {:?}",
                seat.results
            )),
            Some(_) => object
                .get("inputs")
                .and_then(Value::as_object)
                .and_then(|inputs| inputs.get("change"))
                .filter(|_| seat.inputs.iter().any(|input| input == "change"))
                .and_then(|value| match value.as_str() {
                    Some(change) if brokkr_core::policy::is_identifier(change) => None,
                    _ => Some(format!(
                        "declared input 'change' must match ^[a-z0-9][a-z0-9._-]*$, got {value}"
                    )),
                }),
        },
    }
}

/// A ship's facts: whether its worktrees are dirty, and whether the head
/// drifted from the one the protected phase recorded, keyed by realm in
/// a mapped world.
fn ship_inputs(
    state: &RunState,
    repo: &Path,
    realm: Option<&str>,
    inputs: &mut Map<String, Value>,
) {
    let dirty = git_dirty(repo);
    let head = git_head(repo);
    inputs.insert("dirty_worktrees".into(), Value::Bool(dirty));
    // Fail-closed: when the protected phase RECORDED heads,
    // ship always answers the drift question. A repo that
    // no longer resolves to a recorded realm, or a realm
    // whose head was never recorded, is indistinct from
    // drift — silence here shipped where the old code
    // re-armed review (this run's own review caught it).
    let drifted = state
        .reviewed_heads
        .as_ref()
        .map(|recorded| match recorded.head_for(realm) {
            Some(reviewed) => head.as_deref() != Some(reviewed),
            None => true,
        });
    if let Some(drifted) = drifted {
        inputs.insert("drift_detected".into(), Value::Bool(drifted));
    }
    // The same facts, keyed by realm — one realm today,
    // several when multi-realm runs arrive. Recorded only in
    // a mapped world, so an unmapped run's decision payload
    // is byte-for-byte the one it always wrote.
    if let Some(realm) = realm {
        let mut facts = Map::new();
        if let Some(head) = &head {
            facts.insert("head".into(), Value::from(head.clone()));
        }
        facts.insert("dirty_worktrees".into(), Value::Bool(dirty));
        if let Some(drifted) = drifted {
            facts.insert("drift_detected".into(), Value::Bool(drifted));
        }
        inputs.insert(REALM_FACTS.into(), json!({ realm: Value::Object(facts) }));
    }
}

/// The candidate the worktree holds: HEAD and its tree where a commit
/// answers, verified only if git reads the tree clean counting every
/// untracked file whatever the repository's own status setting hides. A
/// status git refuses or cannot run is unclean. The two reads are one
/// pass; a writer racing them produces a pair no later clean pass
/// reproduces, which parks, never admits.
fn candidate_reading(repo: &Path) -> CandidateReading {
    let Some((head, tree)) = git_candidate(repo) else {
        return CandidateReading::NoCommit;
    };
    if read_tree_counting(repo, Untracked::All) == TreeReading::Clean {
        CandidateReading::Verified(VerifiedCandidate { head, tree })
    } else {
        CandidateReading::Unclean
    }
}

/// The candidate's refusal: the ruling keeps its rule, takes no
/// transition, and says why, the shape a blocked artifact gate writes.
fn park(payload: &mut Value, problem: &str) {
    payload["next"] = Value::Null;
    payload["severity"] = Value::Null;
    payload["problem"] = Value::String(problem.into());
}

#[cfg(test)]
#[path = "candidate_binding_tests.rs"]
mod candidate_binding_tests;
