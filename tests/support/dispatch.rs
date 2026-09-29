//! The one place a test spells a forge-dispatch/v2 envelope. The engine's
//! tests and the CLI's both bind runs with it, so a field the contract
//! adds is added here once. Each test binary includes this file through
//! `#[path]`; it is test code by its path, so it stays outside the
//! coverage denominator.

use brokkr_core::dispatch::{DispatchEnvelopeV2, PRODUCER_EFFECTS};
use serde_json::json;
use time::format_description::well_known::Rfc3339;

/// A sealed envelope binding run `run_id` to the recipe `recipe`, compiled
/// to `compiled_sha256`: issued a minute ago, expiring in five, and called
/// back at `callback`.
pub(crate) fn dispatch_envelope(
    run_id: &str,
    recipe: &str,
    compiled_sha256: &str,
    callback: &str,
) -> DispatchEnvelopeV2 {
    let now = time::OffsetDateTime::now_utc();
    serde_json::from_value::<DispatchEnvelopeV2>(json!({
        "schema":"forge-dispatch/v2", "envelope_id":"envelope", "forge_run_id":run_id,
        "issued_at":(now-time::Duration::minutes(1)).format(&Rfc3339).unwrap(),
        "expires_at":(now+time::Duration::minutes(5)).format(&Rfc3339).unwrap(),
        "canonical_digest":"",
        "looper":{"organization_id":"org","product_id":"product","story_id":"story",
            "delivery_run_id":"delivery","request_grant_id":"grant","feature_path":"feature",
            "immutable_inputs_sha256":"a".repeat(64)},
        "actor":{"principal_kind":"api_key","principal_id":"key","actor_kind":"service",
            "actor_id":"brokkr","accountable_operator_id":"operator","authority_source":"looper-grant",
            "operating_profile":"bounded"},
        "repository":{"owner":"owner","name":"repo","base_sha":"b".repeat(64),
            "candidate_sha":null,"workspace_class":"isolated","target_environment":"dogfood"},
        "recipe":{"name":recipe,"compiled_sha256":compiled_sha256},
        "budget":{"lane_tally_run_id":"lane","reservation_id":null,"cost_state":"known",
            "ceiling_microunits":1000,"currency":"USD"},
        "producer":{"registration_id":"registration","token_reference":"key",
            "callback_audience":callback,"accepting_service_id":"looper-api",
            "runtime_id":"runtime","producer_release":"brokkr@test","protocol_version":1,
            "starting_cursor":0},
        "allowed_effects":PRODUCER_EFFECTS,"forbidden_actions":["grant_create","grant_widen",
            "artifact_decide","workflow_advance","release_promote"],
        "bounds":{"max_attempts":3,"max_parallel_effects":4,"max_event_bytes":65536,
            "max_events_per_ten_seconds":40,"replay_retention_seconds":604800,
            "safe_stop":"boundary","cancellation":"fenced"},
        "evidence_requirements":["ordered_hash_chain"],"attestation_requirement":"self_reported"
    }))
    .unwrap()
    .sealed()
}
