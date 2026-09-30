//! The engine half of decision 0016: per-invocation-site selection, the
//! structural fail-to-start predicate, and the proof that `fold` never
//! reads any of it.

use super::*;
use crate::agents::{Candidate, HarnessHands};
use crate::bundle::{PanelMember, SequenceStep};

use super::tests::{engine, single_body, templated};
use crate::envelope_builder::EnvelopeBuilder;

fn event(event_type: EventType, payload: Value) -> EventEnvelope {
    EnvelopeBuilder::new(event_type, payload)
        .at("2026-08-29T00:00:00Z")
        .hash("a".repeat(64))
        .build()
}

fn candidate(agent: &str, model: &str) -> Candidate {
    templated(Candidate {
        agent: agent.into(),
        model: model.into(),
        effort: Some("high".into()),
        provider: "provider".into(),
        argv: vec!["driver".into(), "--model".into(), model.into()],
        hands_fragment: Vec::new(),
        harness: HarnessHands::default(),
        resume: Default::default(),
        lowering: Lowering::Unavailable,
        hands_notice: None,
    })
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
        candidates,
    };
    let single = SeatBody::Single {
        role_path: PathBuf::from("role.md"),
        command: vec!["driver".into()],
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
        candidates: Vec::new(),
    };
    let (selection, provenance, chains) =
        select_candidates(&[], "effect", inline.selected(None).unwrap().0);
    assert!(
        selection.get(&None).is_none(),
        "an inline site selects none"
    );
    assert!(provenance.is_none());
    assert!(chains.is_empty(), "an inline site walks no chain");
    assert_eq!(
        argv_for(&selection, &None, &["inline-driver".to_string()]),
        ["inline-driver".to_string()]
    );

    let resolved = SeatBody::Single {
        role_path: PathBuf::from("role.md"),
        command: vec!["inline-driver".into()],
        candidates: vec![candidate("worker", "opus"), candidate("worker", "sonnet")],
    };
    let events = vec![failure("effect", json!([null]))];
    let (selection, provenance, chains) =
        select_candidates(&events, "effect", resolved.selected(None).unwrap().0);
    assert_eq!(
        chains.get(&None).copied(),
        Some(1),
        "the site walked to its second link, and that index is part of its owner identity"
    );
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
/// matched — the four facts the process layer already knows decide it.
#[test]
fn the_fail_to_start_predicate_reads_only_structure() {
    let failed = |accepted: bool, checkpoints: Vec<Value>| AttemptReport {
        outcome: AttemptOutcome::Failed {
            error: "model not found".into(),
        },
        refused: None,
        cleanup: Cleanup::Settled,
        session_ref: None,
        checkpoints,
        stderr: "provider says: unknown model".into(),
        accepted,
        deadline_killed: false,
    };
    assert!(failed_to_start(&failed(false, Vec::new())));
    // Accepted: the session opened, so this is 0006's territory.
    assert!(!failed_to_start(&failed(true, Vec::new())));
    // Checkpointed: work happened that another model does not inherit.
    assert!(!failed_to_start(&failed(false, vec![json!({"step": "x"})])));
    // Killed by our own watchdog: `Failed` because the kill made
    // non-completion determinate, but the driver never got to say
    // whether a session opened (decision 0053 ruling 5). A hung first
    // turn must not walk the chain down every link on the same vendor.
    assert!(!failed_to_start(&AttemptReport {
        deadline_killed: true,
        ..failed(false, Vec::new())
    }));
    // A tree not proven over (#403): whatever the driver said, something
    // of it may still be running, so no fallback starts beside it.
    assert!(!failed_to_start(&AttemptReport {
        cleanup: Cleanup::Unresolved {
            reason: brokkr_protocol::process::Unsettled::Stdout,
        },
        ..failed(false, Vec::new())
    }));
    // Succeeded and indeterminate are never fail-to-start.
    for outcome in [
        AttemptOutcome::Succeeded { result: json!({}) },
        AttemptOutcome::Indeterminate {
            reason: "lost".into(),
        },
    ] {
        assert!(!failed_to_start(&AttemptReport {
            outcome,
            refused: None,
            cleanup: Cleanup::Settled,
            session_ref: None,
            checkpoints: Vec::new(),
            stderr: String::new(),
            accepted: false,
            deadline_killed: false,
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
        refused: None,
        cleanup: Cleanup::Settled,
        session_ref: None,
        checkpoints: Vec::new(),
        stderr: String::new(),
        accepted,
        deadline_killed: false,
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
    let (before, provenance_before, chains_before) =
        select_candidates(&events, "effect", body.selected(None).unwrap().0);
    // A second, wholly independent selection: no memory, no re-probe.
    let (after, provenance_after, chains_after) =
        select_candidates(&events, "effect", body.selected(None).unwrap().0);
    assert_eq!(before.get(&None), after.get(&None));
    assert_eq!(provenance_before, provenance_after);
    assert_eq!(chains_before, chains_after);
    assert_eq!(chains_before.get(&None).copied(), Some(2));
    assert_eq!(before.get(&None).unwrap().model, "third");
    assert_eq!(provenance_before.unwrap()[0]["chain_index"], 2);
}

/// Decision 0053: a provider refusal the driver classified before the
/// first turn arrives as `Failed` with no `accepted` and no checkpoint —
/// the structural fail-to-start shape — so the chain advances to the next
/// candidate, and the refused attempt keeps its reason in the journal.
#[test]
fn a_pre_session_refusal_advances_the_chain_and_keeps_its_reason() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    let mut selection = Selection::new();
    selection.insert(
        None,
        templated(Candidate {
            agent: "implementer".into(),
            model: "fable".into(),
            effort: Some("high".into()),
            provider: "claude".into(),
            hands_fragment: Vec::new(),
            harness: HarnessHands::default(),
            resume: Default::default(),
            hands_notice: None,
            argv: vec!["driver".into(), "--model".into(), "fable".into()],
            lowering: Lowering::Unavailable,
        }),
    );
    let reason = "provider refused before the first turn: rate_limit (HTTP 429): \
                  You have reached your limit";
    engine
        .conclude_single(
            "effect",
            "attempt",
            DriverRun::Ran(AttemptReport {
                outcome: AttemptOutcome::Failed {
                    error: reason.into(),
                },
                refused: None,
                cleanup: Cleanup::Settled,
                session_ref: None,
                checkpoints: Vec::new(),
                stderr: String::new(),
                accepted: false,
                deadline_killed: false,
            }),
            &selection,
            None,
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let failed = events
        .iter()
        .find(|event| event.event_type == EventType::EffectFailed)
        .unwrap();
    assert_eq!(failed.payload["start_failure"], json!(true));
    assert_eq!(failed.payload["start_failure_sites"], json!([null]));
    assert!(failed.payload["error"]
        .as_str()
        .unwrap()
        .contains("rate_limit"));
    assert!(
        !events
            .iter()
            .any(|event| event.event_type == EventType::EffectCheckpointed),
        "a refused attempt writes no checkpoint"
    );
    // The chain index counts the journaled start failure, so the next
    // attempt is hired from the next link rather than the exhausted one.
    assert_eq!(chain_index(&events, "effect", &None, 2), 1);
}

/// Rebuild unit 27b (operator ruling 2026-09-30, point 3; unit 27's SQ3 and
/// SQ4): A DIALECT STEP IS COMPOSED AND MARKED AT ITS OWN SITE. The
/// generated validator's compiled site holds no charter and no inline
/// segment, so composing at its label adds nothing to its spawn today; it
/// does hold its own "holds nothing" outcome and a judged, unspecified
/// local declaration, from which its launch record is sealed. Planted at
/// that site, a charter binding rides the spawn to the door, which refuses
/// it (SQ3), and a local declaration taken away leaves no record to seal,
/// which refuses the spawn (SQ4). Each is refused before anything spawns.
#[test]
fn a_dialect_step_is_composed_and_marked_at_its_own_site() {
    use crate::bundle::StepBody;
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("bundle");
    std::fs::create_dir_all(dir.join("roles")).unwrap();
    std::fs::write(dir.join("roles/role.md"), "# role\n").unwrap();
    let rules = [("design", "drafted", "review"), ("design", "fail", "design")]
        .into_iter()
        .chain([("review", "clean", "done")])
        .map(|(from, result, next)| {
            json!({"id": result, "from": from, "result": result, "next": next, "reason": result})
        });
    let policy = json!({"phases": ["design", "review", "done"], "initial": "design",
        "terminal": ["done"], "rules": rules.collect::<Vec<_>>()});
    std::fs::write(dir.join("policy.json"), policy.to_string()).unwrap();
    let inline = |name: &str, result: &str| {
        json!({"name": name, "results": [result],
        "role": "roles/role.md", "driver": {"command": ["driver"]}})
    };
    let design = json!({"results": ["drafted", "fail"], "sequence": [
        inline("author", "drafted"), {"name": "validate", "dialect": "validate"}]});
    let mut review = inline("review", "clean");
    review.as_object_mut().unwrap().remove("name");
    let seats = json!({"design": design, "review": review});
    let config = json!({"name": "d", "policy": "policy.json", "seats": seats});
    std::fs::write(dir.join("bundle.json"), config.to_string()).unwrap();
    let dialect = crate::dialect::Dialect::load(&repository.join("dialects/openspec.json"));
    let (agents, adapters) = (repository.join("agents"), repository.join("adapters"));
    let (dialect, boundary) = (Some(&dialect.unwrap().0), Boundary::Namespace);
    let compiled = Bundle::compile_with_realm(&dir, &agents, &adapters, None, dialect, boundary);
    let compiled = compiled.unwrap();
    let SeatBody::Sequence { steps } = &compiled.seats["design"].body else {
        panic!("a sequence seat")
    };
    let validate = steps
        .iter()
        .filter(|step| matches!(step.body, StepBody::Dialect { .. }));
    let validate: Vec<_> = validate.cloned().collect();
    let site = &compiled.sites["design:validate"];
    let held = [
        site.charter.is_none(),
        site.inline_local.is_none(),
        site.inline_sandbox.is_none(),
    ];
    assert_eq!((held, site.inline_hands.is_none()), ([true; 3], true));
    assert_eq!(
        (site.local.is_some(), site.capabilities.is_some()),
        (true, true)
    );
    let author = compiled.sites["design:author"].charter.clone();
    type Plant<'a> = &'a dyn Fn(&mut SiteFacts);
    let plants: [(Plant<'_>, &str); 2] = [
        (
            &|facts| facts.charter = author.clone(),
            "a charter of layer 'd' moved since the compile (replaced: roles/role.md); what a \
             seat is told must be the bytes the bundle's identity names (decision 0066 ruling 5)",
        ),
        (
            &|facts| facts.local = None,
            "the site's local declaration was never judged, so no launch record can be sealed \
             for this site; a record is sealed from typed facts and never repaired into a default \
             one (decision 0065 slice one, design D5.7)",
        ),
    ];
    for (index, (plant, problem)) in plants.into_iter().enumerate() {
        let refusal =
            format!("sequence step 'validate': driver did not spawn: dispatch refused: {problem}");
        let mut bundle = compiled.clone();
        plant(bundle.sites.get_mut("design:validate").unwrap());
        let store = Store::open(&root.join(format!("{index}.db"))).unwrap();
        let mut engine = Engine::start(store, bundle, "f", Some(root.clone())).unwrap();
        let dispatch = json!({"feature": "f", "phase": "design", "workdir": root,
            "allowed_results": ["drafted", "fail"], "house_rules": "",
            "context": {"results": {"design": {"result": "drafted", "inputs": {"change": "c"}}}},
            "steps": [{"role_path": "", "result_path": "result.json",
                       "allowed_results": ["drafted", "fail"]}]});
        let deadline = std::time::Duration::from_secs(2);
        let selection = Selection::new();
        let ran = engine.execute_sequence(
            "e", "a", "design", &validate, &dispatch, deadline, &selection,
        );
        ran.unwrap();
        let events = engine.store.load(&engine.run_id).unwrap();
        assert_eq!(
            events.last().unwrap().payload["error"],
            json!(refusal),
            "{refusal}"
        );
    }
}
