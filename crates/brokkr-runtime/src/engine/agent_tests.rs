//! The engine half of decision 0016: per-invocation-site selection, the
//! structural fail-to-start predicate, and the proof that `fold` never
//! reads any of it.

use super::*;
use crate::agents::Candidate;
use crate::bundle::{PanelMember, SequenceStep};

fn event(event_type: EventType, payload: Value) -> EventEnvelope {
    EventEnvelope {
        run_id: "run".into(),
        seq: 1,
        event_id: "event".into(),
        event_schema_version: 1,
        event_type,
        payload,
        causation_id: None,
        correlation_id: "run".into(),
        attempt_id: None,
        recorded_at: "2026-08-29T00:00:00Z".into(),
        previous_hash: brokkr_core::canonical::ZERO_HASH.into(),
        event_hash: "a".repeat(64),
    }
}

fn candidate(agent: &str, model: &str) -> Candidate {
    Candidate {
        agent: agent.into(),
        model: model.into(),
        effort: Some("high".into()),
        provider: "provider".into(),
        argv: vec!["driver".into(), "--model".into(), model.into()],
    }
}

fn failure(effect_id: &str, sites: Value) -> EventEnvelope {
    let mut payload = json!({
        "effect_id": effect_id,
        "attempt_id": "attempt",
        "error": "did not spawn",
    });
    payload["start_failure"] = Value::Bool(true);
    payload["start_failure_sites"] = sites;
    event(EventType::EffectFailed, payload)
}

/// AC-15: the index is a pure function of journaled facts. Nothing here
/// consults memory, a probe, or the clock — which is exactly why a
/// restart between attempts cannot change which model runs next.
#[test]
fn the_chain_index_counts_journaled_start_failures_and_clamps() {
    let none: Site = None;
    let member: Site = Some("a".into());
    let events = vec![
        failure("effect", json!([null])),
        // A different effect's failures never move this effect's index.
        failure("other", json!([null])),
        // A failure that names only a member leaves the seat's own index.
        failure("effect", json!(["a"])),
        // A terminal failure with no fail-to-start fields is not a skip.
        event(
            EventType::EffectFailed,
            json!({"effect_id": "effect", "attempt_id": "x", "error": "mid-session"}),
        ),
    ];
    assert_eq!(chain_index(&events, "effect", &none, 3), 1);
    assert_eq!(chain_index(&events, "effect", &member, 3), 1);
    // Clamped to the last candidate: the chain is a fallback chain, not
    // an infinite one, and 0006's bound is what ends the attempt.
    assert_eq!(chain_index(&events, "effect", &none, 1), 0);
    assert_eq!(chain_index(&[], "effect", &none, 2), 0);

    assert!(site_matches(&Value::Null, &none));
    assert!(!site_matches(&json!("a"), &none));
    assert!(site_matches(&json!("a"), &member));
    assert!(!site_matches(&Value::Null, &member));
}

/// Every invocation site of every seat shape, tagged exactly as the
/// journal tags its checkpoints.
#[test]
fn invocation_sites_name_every_shape_the_engine_can_run() {
    let member = |name: &str, candidates: Vec<Candidate>| PanelMember {
        name: name.into(),
        role_path: PathBuf::from("role.md"),
        command: vec!["driver".into()],
        confine: None,
        candidates,
    };
    let single = SeatBody::Single {
        role_path: PathBuf::from("role.md"),
        command: vec!["driver".into()],
        confine: None,
        candidates: vec![candidate("worker", "opus")],
    };
    assert_eq!(
        invocation_sites(single.selected(None).unwrap().0)[0].0,
        None
    );

    let panel = SeatBody::Panel {
        members: vec![
            member("a", vec![candidate("left", "opus")]),
            member("b", Vec::new()),
        ],
        aggregate: Aggregate::UnanimousPass,
    };
    let sites = invocation_sites(panel.selected(None).unwrap().0);
    assert_eq!(sites[0].0, Some("a".into()));
    assert_eq!(sites[1].0, Some("b".into()));
    assert!(sites[1].1.is_empty(), "an inline member has no chain");

    let sequence = SeatBody::Sequence {
        steps: vec![
            SequenceStep {
                name: "one".into(),
                results: vec!["one-result".into()],
                class: SeatClass::Work,
                body: StepBody::Single {
                    role_path: PathBuf::from("role.md"),
                    command: vec!["driver".into()],
                    confine: None,
                    candidates: vec![candidate("worker", "opus")],
                },
            },
            SequenceStep {
                name: "two".into(),
                results: vec!["two-result".into()],
                class: SeatClass::Work,
                body: StepBody::Panel {
                    members: vec![member("m", vec![candidate("left", "opus")])],
                    aggregate: Aggregate::UnanimousPass,
                },
            },
        ],
    };
    let sites = invocation_sites(sequence.selected(None).unwrap().0);
    assert_eq!(sites[0].0, Some("one".into()));
    assert_eq!(sites[1].0, Some("two:m".into()));
}

/// A seat with no agent-resolved site produces NO provenance and no
/// selection: an inline run's journal gains nothing at all.
#[test]
fn an_inline_seat_selects_nothing_and_journals_nothing() {
    let inline = SeatBody::Single {
        role_path: PathBuf::from("role.md"),
        command: vec!["inline-driver".into()],
        confine: None,
        candidates: Vec::new(),
    };
    let (selection, provenance) =
        select_candidates(&[], "effect", inline.selected(None).unwrap().0);
    assert!(selection.is_empty());
    assert!(provenance.is_none());
    assert_eq!(
        argv_for(&selection, &None, &["inline-driver".to_string()]),
        ["inline-driver".to_string()]
    );

    let resolved = SeatBody::Single {
        role_path: PathBuf::from("role.md"),
        command: vec!["inline-driver".into()],
        confine: None,
        candidates: vec![candidate("worker", "opus"), candidate("worker", "sonnet")],
    };
    let events = vec![failure("effect", json!([null]))];
    let (selection, provenance) =
        select_candidates(&events, "effect", resolved.selected(None).unwrap().0);
    assert_eq!(
        provenance.unwrap(),
        // The effort pin is journaled beside the model it was hired
        // with (decision 0035 ruling 5), so the view can carry the plan's
        // ask and the harness's echo as two separate facts.
        json!([{
            "member": null, "agent": "worker", "model": "sonnet",
            "effort": "high",
            "provider": "provider", "chain_index": 1,
        }])
    );
    assert_eq!(
        argv_for(&selection, &None, &["inline-driver".to_string()]),
        ["driver".to_string(), "--model".into(), "sonnet".into()]
    );
}

/// AC-14: the predicate is structural. No stderr is read, no message is
/// matched — the three facts the process layer already knows decide it.
#[test]
fn the_fail_to_start_predicate_reads_only_structure() {
    let failed = |accepted: bool, checkpoints: Vec<Value>| AttemptReport {
        outcome: AttemptOutcome::Failed {
            error: "model not found".into(),
        },
        session_ref: None,
        checkpoints,
        stderr: "provider says: unknown model".into(),
        accepted,
    };
    assert!(failed_to_start(&failed(false, Vec::new())));
    // Accepted: the session opened, so this is 0006's territory.
    assert!(!failed_to_start(&failed(true, Vec::new())));
    // Checkpointed: work happened that another model does not inherit.
    assert!(!failed_to_start(&failed(false, vec![json!({"step": "x"})])));
    // Succeeded and indeterminate are never fail-to-start.
    for outcome in [
        AttemptOutcome::Succeeded { result: json!({}) },
        AttemptOutcome::Indeterminate {
            reason: "lost".into(),
        },
    ] {
        assert!(!failed_to_start(&AttemptReport {
            outcome,
            session_ref: None,
            checkpoints: Vec::new(),
            stderr: String::new(),
            accepted: false,
        }));
    }
}

/// The fail-to-start fields are absent unless an AGENT-RESOLVED site
/// failed to start: an inline seat that fails to spawn journals exactly
/// what it always did.
#[test]
fn start_failure_fields_are_absent_for_inline_sites() {
    let mut payload = json!({"error": "did not spawn"});
    start_failure_fields(&mut payload, &Selection::new(), vec![None]);
    assert_eq!(payload, json!({"error": "did not spawn"}));

    let mut selection = Selection::new();
    selection.insert(Some("a".into()), candidate("left", "opus"));
    let mut payload = json!({"error": "did not spawn"});
    start_failure_fields(
        &mut payload,
        &selection,
        vec![Some("a".into()), Some("b".into())],
    );
    assert_eq!(payload["start_failure"], json!(true));
    assert_eq!(payload["start_failure_sites"], json!(["a"]));
}

#[test]
fn start_failure_sites_names_the_members_that_never_started() {
    let report = |accepted: bool| AttemptReport {
        outcome: AttemptOutcome::Failed {
            error: "boom".into(),
        },
        session_ref: None,
        checkpoints: Vec::new(),
        stderr: String::new(),
        accepted,
    };
    let reports = vec![("a".to_string(), report(false)), ("b".into(), report(true))];
    assert_eq!(start_failure_sites(&reports, ""), vec![Some("a".into())]);
    assert_eq!(
        start_failure_sites(&reports, "step:"),
        vec![Some("step:a".into())]
    );
}

/// AC-13: `fold` never reads a provenance field. An adopting run's
/// journal and the same journal with every extension field stripped fold
/// to the identical `RunState` — which is what makes the amended
/// `contracts/README.md` rule honest rather than convenient.
#[test]
fn fold_is_blind_to_every_field_this_slice_adds() {
    let mut adopting = vec![
        event(
            EventType::RunStarted,
            json!({"feature": "f", "manifest": {"agents": {"work": {}}}}),
        ),
        event(EventType::PhaseEntered, json!({"phase": "work"})),
        event(
            EventType::EffectRequested,
            json!({"effect_id": "e", "phase": "work", "seat": "work",
                   "idempotency_key": "k", "input_digest": "d"}),
        ),
        event(
            EventType::EffectStarted,
            json!({"effect_id": "e", "attempt_id": "a1", "driver": "d",
                   "provenance": [{"member": null, "agent": "worker",
                                   "model": "opus", "provider": "p",
                                   "chain_index": 0}]}),
        ),
        failure("e", json!([null])),
        event(
            EventType::EffectStarted,
            json!({"effect_id": "e", "attempt_id": "a2", "driver": "d",
                   "provenance": [{"member": null, "agent": "worker",
                                   "model": "sonnet", "provider": "p",
                                   "chain_index": 1}]}),
        ),
        event(
            EventType::EffectSucceeded,
            json!({"effect_id": "e", "attempt_id": "a2", "result": {"result": "complete"}}),
        ),
    ];
    for (index, envelope) in adopting.iter_mut().enumerate() {
        envelope.seq = index as u64 + 1;
    }
    let mut stripped = adopting.clone();
    for envelope in &mut stripped {
        if let Some(payload) = envelope.payload.as_object_mut() {
            for field in ["provenance", "start_failure", "start_failure_sites"] {
                payload.remove(field);
            }
        }
    }
    let with = fold(&adopting).unwrap();
    let without = fold(&stripped).unwrap();
    assert_eq!(format!("{with:?}"), format!("{without:?}"));
}

/// AC-15: the chain index survives a restart because it is never held
/// across one. The engine keeps no cross-attempt state — the selection
/// is a function of the effect's journaled events and nothing else — so
/// two independent selections over the same journal choose the same
/// candidate, which is exactly what a fresh process does when it
/// resumes into an effect that already has a fail-to-start behind it.
#[test]
fn the_chain_index_survives_a_restart_because_nothing_holds_it() {
    let body = SeatBody::Single {
        role_path: PathBuf::from("role.md"),
        command: vec!["inline-driver".into()],
        confine: None,
        candidates: vec![
            candidate("worker", "first"),
            candidate("worker", "second"),
            candidate("worker", "third"),
        ],
    };
    let events = vec![
        failure("effect", json!([null])),
        failure("effect", json!([null])),
    ];
    let (before, provenance_before) =
        select_candidates(&events, "effect", body.selected(None).unwrap().0);
    // A second, wholly independent selection: no memory, no re-probe.
    let (after, provenance_after) =
        select_candidates(&events, "effect", body.selected(None).unwrap().0);
    assert_eq!(before, after);
    assert_eq!(provenance_before, provenance_after);
    assert_eq!(before[&None].model, "third");
    assert_eq!(provenance_before.unwrap()[0]["chain_index"], 2);
}
