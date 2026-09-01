//! The seam the third review of run implement-decision-0021 named
//! (residual 2, the reason its residual 1 shipped green): nothing
//! compiled a REAL bundle and pushed its manifest through the
//! Looper-bound run-manifest/v2 round-trip, so the bundle manifest's
//! shape and the dispatch lineage's tolerance for it could drift apart
//! silently — as they did when the v5 `drivers` witness landed. This
//! test pins the seam whichever way it moves: every shipped bundle
//! either round-trips losslessly or is refused loudly, and a manifest
//! key the lineage has not learned can never be dropped in silence.

use brokkr_core::canonical;
use brokkr_core::dispatch::{
    build_run_manifest_v2, bundle_manifest_from_run, ActorBinding, BudgetBinding, DispatchBounds,
    DispatchEnvelopeV2, DispatchError, LooperBinding, ProducerBinding, RecipeBinding,
    RepositoryBinding, DISPATCH_SCHEMA_V2, PRODUCER_EFFECTS, REQUIRED_FORBIDDEN_ACTIONS,
};
use brokkr_runtime::Bundle;
use std::path::PathBuf;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn compile(relative: &str) -> Bundle {
    let root = workspace();
    Bundle::compile_with(
        &root.join(relative),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .unwrap_or_else(|e| panic!("{relative} must compile: {e}"))
}

/// The unit-test fixture's envelope, rebuilt here because the seam under
/// test spans two crates: a real compiled manifest (runtime) entering
/// the Looper lineage (core).
fn envelope(bundle_sha256: &str) -> DispatchEnvelopeV2 {
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

/// Lossless or loud, never silent: for every shipped bundle and recipe,
/// entering the v2 lineage either preserves the manifest byte-for-byte
/// through the round-trip or is refused by a named error that blames
/// the exact key the lineage cannot carry.
#[test]
fn every_shipped_manifest_round_trips_losslessly_or_is_refused_loudly() {
    for relative in [
        "bundles/self",
        "bundles/verify",
        "recipes/fast",
        "recipes/sdd",
        "recipes/sdd-paranoid",
        "recipes/panel-review",
    ] {
        let manifest = compile(relative).manifest;
        let sha = canonical::sha256_hex(&manifest);
        match build_run_manifest_v2(&manifest, envelope(&sha)) {
            Ok(run) => assert_eq!(
                bundle_manifest_from_run(&run).unwrap(),
                manifest,
                "{relative}: the v2 round-trip dropped part of the manifest \
                 it accepted — the silent unresumability this test exists \
                 to forbid"
            ),
            Err(
                DispatchError::AgentsUnsupportedByDispatchLineage
                | DispatchError::ManifestKeyUnsupportedByDispatchLineage(_),
            ) => {}
            Err(other) => panic!(
                "{relative}: refused, but not by a lineage-limit error \
                 that names what cannot be carried: {other}"
            ),
        }
    }
}

/// The two known witnesses land as the refusals the lineage names for
/// them today — `recipes/fast` seats gates whose authorising adapters
/// are pinned under `drivers`, and `bundles/self` adopts agents. The
/// refusal is the honest reading of decision 0021's witness meeting
/// decision 0016's frozen v2 contract; a jointly agreed v2-lineage
/// version that carries the pins lifts it.
#[test]
fn the_gate_witness_and_the_agent_pin_are_refused_by_name() {
    let fast = compile("recipes/fast").manifest;
    assert!(fast.get("drivers").is_some(), "recipes/fast seats gates");
    let sha = canonical::sha256_hex(&fast);
    assert_eq!(
        build_run_manifest_v2(&fast, envelope(&sha)),
        Err(DispatchError::ManifestKeyUnsupportedByDispatchLineage(
            "drivers".into()
        ))
    );

    let own = compile("bundles/self").manifest;
    assert!(own.get("agents").is_some(), "bundles/self adopts agents");
    let sha = canonical::sha256_hex(&own);
    assert_eq!(
        build_run_manifest_v2(&own, envelope(&sha)),
        Err(DispatchError::AgentsUnsupportedByDispatchLineage)
    );
}
