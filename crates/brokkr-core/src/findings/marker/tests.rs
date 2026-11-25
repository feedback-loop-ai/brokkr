//! Fix round 0109a1r4, K1, bound: a marker reads only by the engine's
//! authorship, and a driver's live checkpoint passes the admission a
//! standalone result passes.

use serde_json::json;

use super::*;
use crate::findings::EngineWritten;

const SITE: &str = "5a1e000000000000000000000000000000000000000000000000000000000000";

fn finding() -> Value {
    json!({"id": "planted", "file": "src/lib.rs", "summary": "s",
           "severity": "high", "security": true})
}

#[test]
fn a_marker_reads_by_the_engines_authorship_never_by_its_word() {
    let authored = EngineMarker::StepFinished.record(json!({"step_name": "judge"}));
    assert_eq!(authored["step"], "sequence-step-finished");
    assert_eq!(
        EngineMarker::authored(&authored),
        Some(EngineMarker::StepFinished)
    );
    let member = EngineMarker::MemberFinished.record(json!({"member": "a"}));
    assert_eq!(
        EngineMarker::authored(&member),
        Some(EngineMarker::MemberFinished)
    );
    let worded = json!({"step": "sequence-step-finished", "step_name": "judge"});
    assert_eq!(EngineMarker::authored(&worded), None);
    let forged = json!({"step": "sequence-step-finished", "authored": "driver"});
    assert_eq!(EngineMarker::authored(&forged), None);
    let other = EngineMarker::StepFinished.record(json!({}));
    let other = json!({"step": "seat-turn", "authored": other["authored"]});
    assert_eq!(EngineMarker::authored(&other), None);
}

/// Fix round 0109a1r6, W2: the one mark a settled member's payload reads
/// as a verdict by is the engine's own member marker naming it as
/// succeeded; a released engine's unauthored marker, a step marker and a
/// member that did not succeed mark none.
#[test]
fn a_member_is_settled_only_by_the_engines_own_succeeded_member_marker() {
    let marker = |outcome: &str| {
        EngineMarker::MemberFinished.record(json!({"member": "a", "outcome": outcome}))
    };
    assert_eq!(
        EngineMarker::settled_member(&marker("succeeded")),
        Some("a")
    );
    assert_eq!(EngineMarker::settled_member(&marker("failed")), None);
    let mut released = marker("succeeded");
    released.as_object_mut().unwrap().remove(AUTHORED);
    assert_eq!(EngineMarker::settled_member(&released), None);
    let step = EngineMarker::StepFinished.record(json!({"member": "a", "outcome": "succeeded"}));
    assert_eq!(EngineMarker::settled_member(&step), None);
    let unnamed = EngineMarker::MemberFinished.record(json!({"outcome": "succeeded"}));
    assert_eq!(EngineMarker::settled_member(&unnamed), None);
}

#[test]
fn a_live_checkpoint_that_claims_the_engines_record_is_refused() {
    for mut claimed in [
        json!({"step": "sequence-step-finished", "step_name": "judge"}),
        json!({"step": "panel-member-finished", "member": "a"}),
        json!({"step": "seat-turn", "authored": "engine"}),
    ] {
        assert_eq!(
            admit_live(&mut claimed, Some(Reporter::Gate(SITE))),
            Err(LiveRefusal::EngineAuthored)
        );
    }
    assert_eq!(
        LiveRefusal::EngineAuthored.to_string(),
        "a driver's checkpoint claims a record only the engine authors (decision 0109 ruling 1)"
    );
}

#[test]
fn a_live_checkpoints_result_passes_the_standalone_admission() {
    let head = "a".repeat(40);
    let mut covered = json!({"step": "seat-turn", "result":
        {"result": "pass", "covered": {"kind": "head", "head": head, "dirty": false}}});
    assert_eq!(
        admit_live(&mut covered, Some(Reporter::Gate(SITE))),
        Err(LiveRefusal::Result(AdmissionRefusal::EngineWritten(
            EngineWritten::Covered
        )))
    );
    let mut sited = json!({"step": "seat-turn", "result":
        {"result": "pass", "findings": [{"site": SITE, "id": "planted"}]}});
    assert_eq!(
        admit_live(&mut sited, Some(Reporter::Gate(SITE))),
        Err(LiveRefusal::Result(AdmissionRefusal::SeatSite { index: 0 }))
    );
    let mut work = json!({"step": "seat-turn", "result":
        {"result": "pass", "findings": [finding()]}});
    assert_eq!(
        admit_live(&mut work, Some(Reporter::Work)),
        Err(LiveRefusal::Result(AdmissionRefusal::WorkSite))
    );
    assert_eq!(
        LiveRefusal::Result(AdmissionRefusal::WorkSite).to_string(),
        "a driver's checkpoint result was refused: the result carries 'findings' from a work \
         site: only a gate reports findings (decision 0109 ruling 1)"
    );
}

/// Fix round 0109a1r5, V4: findings belong only to verdicts, so a gate
/// driver's live checkpoint that reports one is refused, never stamped;
/// an empty set, a result that reports none, and a checkpoint with no
/// result pass as they were sent.
#[test]
fn a_gate_drivers_live_finding_is_refused_and_the_rest_passes() {
    let mut gate = json!({"step": "seat-turn", "result":
        {"result": "pass", "findings": [finding()]}});
    assert_eq!(
        admit_live(&mut gate, Some(Reporter::Gate(SITE))),
        Err(LiveRefusal::Finding)
    );
    assert_eq!(
        LiveRefusal::Finding.to_string(),
        "a driver's checkpoint carries a finding: findings belong only to the seat's result, \
         its verdict (decision 0109 ruling 1)"
    );
    let empty = json!({"step": "seat-turn", "result": {"result": "pass", "findings": []}});
    let mut sent = empty.clone();
    assert_eq!(admit_live(&mut sent, Some(Reporter::Gate(SITE))), Ok(()));
    assert_eq!(sent, empty);
    let silent = json!({"step": "seat-turn", "result": {"result": "pass"}});
    let mut unreported = silent.clone();
    assert_eq!(admit_live(&mut unreported, Some(Reporter::Work)), Ok(()));
    assert_eq!(unreported, silent);
    let mut plain = json!({"step": "seat-turn", "turn": 1});
    assert_eq!(admit_live(&mut plain, Some(Reporter::Work)), Ok(()));
    assert_eq!(plain, json!({"step": "seat-turn", "turn": 1}));
}
