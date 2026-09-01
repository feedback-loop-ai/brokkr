use super::*;
use serde_json::json;

type DispatchMutation = Box<dyn Fn(&mut DispatchEnvelopeV2)>;

fn fixture(bundle_sha256: &str) -> DispatchEnvelopeV2 {
    DispatchEnvelopeV2 {
        schema: DISPATCH_SCHEMA_V2.into(),
        envelope_id: "11111111-1111-4111-8111-111111111111".into(),
        forge_run_id: "forge-bound-run".into(),
        issued_at: "2026-08-28T08:00:00Z".into(),
        expires_at: "2026-08-28T09:00:00Z".into(),
        canonical_digest: String::new(),
        looper: LooperBinding {
            organization_id: "22222222-2222-4222-8222-222222222222".into(),
            product_id: "33333333-3333-4333-8333-333333333333".into(),
            story_id: "44444444-4444-4444-8444-444444444444".into(),
            delivery_run_id: "55555555-5555-4555-8555-555555555555".into(),
            request_grant_id: "66666666-6666-4666-8666-666666666666".into(),
            feature_path: "084-f-live-sse-cost-terminal-evidence".into(),
            immutable_inputs_sha256: "a".repeat(64),
        },
        actor: ActorBinding {
            principal_kind: "api_key".into(),
            principal_id: "key-ref".into(),
            actor_kind: "service".into(),
            actor_id: "rust-forge".into(),
            accountable_operator_id: "77777777-7777-4777-8777-777777777777".into(),
            authority_source: "looper-grant".into(),
            operating_profile: "bounded-autonomy".into(),
        },
        repository: RepositoryBinding {
            owner: "feedback-loop-ai".into(),
            name: "looper".into(),
            base_sha: "b".repeat(64),
            candidate_sha: None,
            workspace_class: "isolated-worktree".into(),
            target_environment: "dogfood".into(),
        },
        recipe: RecipeBinding {
            name: "fast".into(),
            compiled_sha256: bundle_sha256.into(),
        },
        budget: BudgetBinding {
            lane_tally_run_id: "55555555-5555-4555-8555-555555555555".into(),
            reservation_id: Some("reservation-1".into()),
            cost_state: "known".into(),
            ceiling_microunits: Some(2_000_000),
            currency: Some("USD".into()),
        },
        producer: ProducerBinding {
            registration_id: "88888888-8888-4888-8888-888888888888".into(),
            token_reference: "key-ref".into(),
            callback_audience: "https://dogfood.feedback-loop.ai".into(),
            accepting_service_id: "looper-api".into(),
            runtime_id: "runtime-1".into(),
            producer_release: "forge@candidate".into(),
            protocol_version: 1,
            starting_cursor: 0,
        },
        allowed_effects: PRODUCER_EFFECTS
            .iter()
            .map(|value| (*value).into())
            .collect(),
        forbidden_actions: REQUIRED_FORBIDDEN_ACTIONS
            .iter()
            .map(|value| (*value).into())
            .collect(),
        bounds: DispatchBounds {
            max_attempts: 3,
            max_parallel_effects: 4,
            max_event_bytes: 65_536,
            max_events_per_ten_seconds: 40,
            replay_retention_seconds: 604_800,
            safe_stop: "nearest_phase_boundary".into(),
            cancellation: "fenced_operator_command".into(),
        },
        evidence_requirements: vec!["ordered_hash_chain".into(), "terminal_receipt".into()],
        attestation_requirement: "self_reported".into(),
    }
    .sealed()
}

fn now() -> OffsetDateTime {
    OffsetDateTime::parse("2026-08-28T08:30:00Z", &Rfc3339).unwrap()
}

fn bundle() -> Value {
    json!({
        "engine":"0.2.0", "event_schema":1, "database_schema":1,
        "driver_protocol":1, "bundle_name":"fast",
        "files":{"policy.json":"c".repeat(64)}
    })
}

fn verify_mutation(
    bundle_sha: &str,
    expected: DispatchError,
    mutate: impl FnOnce(&mut DispatchEnvelopeV2),
) {
    let mut dispatch = fixture(bundle_sha);
    mutate(&mut dispatch);
    dispatch = dispatch.sealed();
    assert_eq!(dispatch.verify(now(), bundle_sha), Err(expected));
}

#[test]
fn dispatch_digest_scope_time_and_manifest_are_fenced() {
    let bundle = bundle();
    let bundle_sha = canonical::sha256_hex(&bundle);
    let dispatch = fixture(&bundle_sha);
    assert_eq!(dispatch.verify(now(), &bundle_sha), Ok(()));

    let manifest = build_run_manifest_v2(&bundle, dispatch.clone()).unwrap();
    assert_eq!(bundle_manifest_from_run(&manifest).unwrap(), bundle);
    assert_eq!(
        dispatch_from_run(&manifest).unwrap(),
        Some(dispatch.clone())
    );

    let mut changed = dispatch.clone();
    changed.looper.story_id = "other".into();
    assert_eq!(
        changed.verify(now(), &bundle_sha),
        Err(DispatchError::BadDigest)
    );
    let mut expired = dispatch.clone();
    expired.expires_at = "2026-08-28T08:20:00Z".into();
    expired = expired.sealed();
    assert_eq!(
        expired.verify(now(), &bundle_sha),
        Err(DispatchError::InvalidTime)
    );
    let mut widened = dispatch.clone();
    widened.allowed_effects.push("workflow_advance".into());
    widened = widened.sealed();
    assert_eq!(
        widened.verify(now(), &bundle_sha),
        Err(DispatchError::EffectScope)
    );
    let mut confused = dispatch;
    confused.producer.callback_audience = "http://localhost:80@evil.example".into();
    confused = confused.sealed();
    assert_eq!(
        confused.verify(now(), &bundle_sha),
        Err(DispatchError::UnsafeBounds)
    );
}

#[test]
fn scalar_fields_hashes_and_times_fail_closed() {
    let bundle_sha = canonical::sha256_hex(&bundle());

    verify_mutation(&bundle_sha, DispatchError::BadSchema, |d| {
        d.schema = "old".into()
    });
    verify_mutation(&bundle_sha, DispatchError::BadSchema, |d| {
        d.producer.protocol_version = 2
    });
    for bad in ["", "   ", "line\nbreak"] {
        verify_mutation(&bundle_sha, DispatchError::BadField("envelope_id"), |d| {
            d.envelope_id = bad.into()
        });
    }
    verify_mutation(&bundle_sha, DispatchError::BadField("envelope_id"), |d| {
        d.envelope_id = "x".repeat(513)
    });

    let mut bad_digest = fixture(&bundle_sha);
    bad_digest.canonical_digest = "A".repeat(64);
    assert_eq!(
        bad_digest.verify(now(), &bundle_sha),
        Err(DispatchError::BadField("sha256"))
    );
    verify_mutation(&bundle_sha, DispatchError::BadField("sha256"), |d| {
        d.looper.immutable_inputs_sha256 = "g".repeat(64)
    });
    verify_mutation(&bundle_sha, DispatchError::BadField("sha256"), |d| {
        d.repository.base_sha = "short".into()
    });
    verify_mutation(&bundle_sha, DispatchError::BadField("sha256"), |d| {
        d.repository.candidate_sha = Some("BAD".into())
    });
    verify_mutation(&bundle_sha, DispatchError::BadField("sha256"), |d| {
        d.recipe.compiled_sha256 = "z".repeat(64)
    });

    verify_mutation(&bundle_sha, DispatchError::InvalidTime, |d| {
        d.issued_at = "never".into()
    });
    verify_mutation(&bundle_sha, DispatchError::InvalidTime, |d| {
        d.expires_at = "never".into()
    });
    verify_mutation(&bundle_sha, DispatchError::InvalidTime, |d| {
        d.issued_at = d.expires_at.clone()
    });
    verify_mutation(&bundle_sha, DispatchError::InvalidTime, |d| {
        d.issued_at = "2026-08-28T08:31:00Z".into()
    });
    verify_mutation(&bundle_sha, DispatchError::InvalidTime, |d| {
        d.expires_at = "2026-08-28T08:30:00Z".into()
    });

    let mut tampered = fixture(&bundle_sha);
    tampered.actor.actor_id = "changed-after-seal".into();
    assert_eq!(
        tampered.verify(now(), &bundle_sha),
        Err(DispatchError::BadDigest)
    );
}

#[test]
fn authority_scope_budget_and_evidence_are_least_privilege() {
    let bundle_sha = canonical::sha256_hex(&bundle());
    let authority_cases: Vec<DispatchMutation> = vec![
        Box::new(|d| d.actor.principal_kind = "human".into()),
        Box::new(|d| d.actor.principal_id = "other-key".into()),
        Box::new(|d| d.actor.authority_source = "ambient".into()),
        Box::new(|d| d.actor.actor_kind = "invented".into()),
        Box::new(|d| d.producer.accepting_service_id = "other-api".into()),
        Box::new(|d| d.producer.starting_cursor = 1),
    ];
    for mutate in authority_cases {
        verify_mutation(
            &bundle_sha,
            DispatchError::BadField("producer_authority"),
            mutate,
        );
    }
    for actor_kind in [
        "accountable_human",
        "ai_agent",
        "service",
        "system_validator",
    ] {
        let mut dispatch = fixture(&bundle_sha);
        dispatch.actor.actor_kind = actor_kind.into();
        dispatch = dispatch.sealed();
        assert_eq!(dispatch.verify(now(), &bundle_sha), Ok(()));
    }
    verify_mutation(&bundle_sha, DispatchError::RecipeMismatch, |d| {
        d.recipe.compiled_sha256 = "d".repeat(64)
    });

    for mutate in [
        (|d: &mut DispatchEnvelopeV2| d.allowed_effects.clear()) as fn(&mut DispatchEnvelopeV2),
        |d| d.allowed_effects.push(d.allowed_effects[0].clone()),
        |d| d.allowed_effects = vec!["invented".into()],
    ] {
        verify_mutation(&bundle_sha, DispatchError::EffectScope, mutate);
    }
    for mutate in [
        (|d: &mut DispatchEnvelopeV2| d.forbidden_actions.clear()) as fn(&mut DispatchEnvelopeV2),
        |d| d.forbidden_actions.push(d.forbidden_actions[0].clone()),
        |d| d.forbidden_actions[0] = "invented".into(),
    ] {
        verify_mutation(&bundle_sha, DispatchError::ForbiddenScope, mutate);
    }

    for mutate in [
        (|d: &mut DispatchEnvelopeV2| d.budget.cost_state = "invented".into())
            as fn(&mut DispatchEnvelopeV2),
        |d| d.budget.ceiling_microunits = None,
        |d| d.budget.ceiling_microunits = Some(0),
        |d| d.budget.currency = None,
        |d| d.budget.currency = Some("EU".into()),
        |d| d.budget.currency = Some("usd".into()),
    ] {
        verify_mutation(&bundle_sha, DispatchError::Budget, mutate);
    }
    for cost_state in [
        "known",
        "evidenced-zero",
        "unknown",
        "not-applicable",
        "reconciliation-pending",
        "final",
    ] {
        let mut dispatch = fixture(&bundle_sha);
        dispatch.budget.cost_state = cost_state.into();
        dispatch = dispatch.sealed();
        assert_eq!(dispatch.verify(now(), &bundle_sha), Ok(()));
    }

    for mutate in [
        (|d: &mut DispatchEnvelopeV2| d.evidence_requirements.clear())
            as fn(&mut DispatchEnvelopeV2),
        |d| d.evidence_requirements.push("terminal_receipt".into()),
        |d| d.evidence_requirements[1] = "".into(),
        |d| d.attestation_requirement = "external".into(),
    ] {
        verify_mutation(
            &bundle_sha,
            DispatchError::BadField("evidence_requirements"),
            mutate,
        );
    }
}

#[test]
fn every_execution_bound_and_callback_origin_is_validated() {
    let bundle_sha = canonical::sha256_hex(&bundle());
    let bound_cases: Vec<DispatchMutation> = vec![
        Box::new(|d| d.bounds.max_attempts = 0),
        Box::new(|d| d.bounds.max_parallel_effects = 0),
        Box::new(|d| d.bounds.max_event_bytes = 0),
        Box::new(|d| d.bounds.max_event_bytes = 65_537),
        Box::new(|d| d.bounds.max_events_per_ten_seconds = 0),
        Box::new(|d| d.bounds.max_events_per_ten_seconds = 41),
        Box::new(|d| d.bounds.replay_retention_seconds = 604_799),
        Box::new(|d| d.bounds.safe_stop = "immediate".into()),
        Box::new(|d| d.bounds.cancellation = "ambient".into()),
    ];
    for mutate in bound_cases {
        verify_mutation(&bundle_sha, DispatchError::UnsafeBounds, mutate);
    }
    for (audience, valid) in [
        ("https://example.test", true),
        ("http://localhost", true),
        ("http://127.0.0.1", true),
        ("not-a-url", false),
        ("https://user@example.test", false),
        ("https://user:pass@example.test", false),
        ("https://:pass@example.test", false),
        ("https://example.test/path", false),
        ("https://example.test?query=1", false),
        ("https://example.test#fragment", false),
        ("http://example.test", false),
        ("ftp://example.test", false),
    ] {
        let mut dispatch = fixture(&bundle_sha);
        dispatch.producer.callback_audience = audience.into();
        dispatch = dispatch.sealed();
        assert_eq!(
            dispatch.verify(now(), &bundle_sha).is_ok(),
            valid,
            "{audience}"
        );
    }
}

#[test]
fn manifest_conversion_refuses_every_malformed_boundary() {
    let valid = bundle();
    let bundle_sha = canonical::sha256_hex(&valid);
    let dispatch = fixture(&bundle_sha);

    assert_eq!(bundle_manifest_from_run(&valid).unwrap(), valid);
    assert_eq!(dispatch_from_run(&valid).unwrap(), None);
    assert_eq!(
        build_run_manifest_v2(&Value::Null, dispatch.clone()),
        Err(DispatchError::BadManifest)
    );
    for key in [
        "engine",
        "event_schema",
        "database_schema",
        "driver_protocol",
        "bundle_name",
        "files",
    ] {
        let mut malformed = valid.clone();
        malformed.as_object_mut().unwrap().remove(key);
        let altered_sha = canonical::sha256_hex(&malformed);
        assert_eq!(
            build_run_manifest_v2(&malformed, fixture(&altered_sha)),
            Err(DispatchError::BadManifest),
            "{key}"
        );
    }
    let mut empty_files = valid.clone();
    empty_files["files"] = json!({});
    let empty_files_sha = canonical::sha256_hex(&empty_files);
    assert_eq!(
        build_run_manifest_v2(&empty_files, fixture(&empty_files_sha)),
        Err(DispatchError::BadManifest)
    );
    for (key, value) in [
        ("engine", json!("")),
        ("bundle_name", json!("")),
        ("event_schema", json!(2)),
        ("database_schema", json!(2)),
        ("driver_protocol", json!(2)),
    ] {
        let mut malformed = valid.clone();
        malformed[key] = value;
        let altered_sha = canonical::sha256_hex(&malformed);
        assert_eq!(
            build_run_manifest_v2(&malformed, fixture(&altered_sha)),
            Err(DispatchError::BadManifest),
            "{key}"
        );
    }
    // AC-19 (decision 0016): the Looper lineage REFUSES an adopting
    // bundle rather than truncating its pin. `bundle_manifest_from_run`
    // reconstructs from six named keys, so an `agents` key would be
    // silently dropped and the run would become unresumable with a diff
    // that blames no file. A loud refusal beats a quiet substitution.
    let mut adopting = valid.clone();
    adopting["agents"] = json!({"implement": {"agent": "implementer"}});
    let adopting_sha = canonical::sha256_hex(&adopting);
    let refusal = build_run_manifest_v2(&adopting, fixture(&adopting_sha));
    assert_eq!(
        refusal,
        Err(DispatchError::AgentsUnsupportedByDispatchLineage)
    );
    let message = refusal.unwrap_err().to_string();
    assert!(message.contains("unresumable"), "{message}");
    assert!(message.contains("v2-lineage manifest version"), "{message}");
    // The reforging of run implement-decision-0021 (operator ruled
    // remedy ii): the v5 `drivers` witness reached the manifest through
    // a key the named refusal above did not guard, so the lineage now
    // refuses EVERY key beyond the six it can round-trip — fail closed,
    // naming the key it cannot carry.
    let mut witnessed = valid.clone();
    witnessed["drivers"] = json!({"verify": {"claude": "e".repeat(64)}});
    let witnessed_sha = canonical::sha256_hex(&witnessed);
    let refusal = build_run_manifest_v2(&witnessed, fixture(&witnessed_sha));
    assert_eq!(
        refusal,
        Err(DispatchError::ManifestKeyUnsupportedByDispatchLineage(
            "drivers".into()
        ))
    );
    let message = refusal.unwrap_err().to_string();
    assert!(message.contains("'drivers'"), "{message}");
    assert!(message.contains("unresumable"), "{message}");
    // And the guard is the whole key space, not a longer list: a key
    // this crate has never heard of is refused the same way.
    let mut unknown = valid.clone();
    unknown["witness_of_2027"] = json!(true);
    let unknown_sha = canonical::sha256_hex(&unknown);
    assert_eq!(
        build_run_manifest_v2(&unknown, fixture(&unknown_sha)),
        Err(DispatchError::ManifestKeyUnsupportedByDispatchLineage(
            "witness_of_2027".into()
        ))
    );
    // Non-adopting bundles dispatch exactly as they did.
    assert!(build_run_manifest_v2(&valid, fixture(&bundle_sha)).is_ok());

    let mut wrong_recipe = dispatch.clone();
    wrong_recipe.recipe.compiled_sha256 = "d".repeat(64);
    assert_eq!(
        build_run_manifest_v2(&valid, wrong_recipe),
        Err(DispatchError::RecipeMismatch)
    );

    let manifest = build_run_manifest_v2(&valid, dispatch).unwrap();
    for key in ["dispatch_sha256", "bundle_sha256"] {
        let mut corrupted = manifest.clone();
        corrupted[key] = json!("0".repeat(64));
        assert_eq!(
            dispatch_from_run(&corrupted),
            Err(DispatchError::BadManifest),
            "{key}"
        );
    }
    let mut malformed = manifest;
    malformed.as_object_mut().unwrap().remove("dispatch");
    assert_eq!(
        bundle_manifest_from_run(&malformed),
        Err(DispatchError::BadManifest)
    );
    assert_eq!(
        dispatch_from_run(&malformed),
        Err(DispatchError::BadManifest)
    );
}
