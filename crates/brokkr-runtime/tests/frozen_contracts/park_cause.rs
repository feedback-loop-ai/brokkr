use super::contract;

/// Decision 0106 ruling 1: the park-cause extension's four words, each
/// closed to exactly the fields its variant has, and the five provider
/// refusal variants with their conditional fields.
#[test]
fn the_park_cause_extension_is_closed_field_for_field() {
    let validator = contract("contracts/park-cause.v1.schema.json");
    for valid in [
        serde_json::json!({"kind": "ruled", "rule_id": "IMPL-BLOCKED", "detail": "blocked"}),
        serde_json::json!({"kind": "ruled", "detail": "no rule matched"}),
        serde_json::json!({"kind": "attempt-cap", "failure_class": "start-failure",
            "detail": "cap"}),
        serde_json::json!({"kind": "infrastructure", "retryable": false,
            "detail": "indeterminate"}),
        serde_json::json!({"kind": "provider-limit", "provider": "dsh", "detail": "refused",
            "refusal": {"kind": "route-unresolved"}}),
        serde_json::json!({"kind": "provider-limit", "provider": "dsh", "detail": "refused",
            "refusal": {"kind": "host-unreadable", "route": "dsh:a"}}),
        serde_json::json!({"kind": "provider-limit", "provider": "dsh", "detail": "refused",
            "refusal": {"kind": "episode-unreadable", "route": "dsh:a"}}),
        serde_json::json!({"kind": "provider-limit", "provider": "dsh", "detail": "refused",
            "refusal": {"kind": "past-maximum", "route": "dsh:a",
                "until": "2026-11-01T00:00:03Z", "seconds": 7201, "max_seconds": 7200}}),
        serde_json::json!({"kind": "provider-limit", "provider": "dsh", "detail": "refused",
            "refusal": {"kind": "no-end", "route": "dsh:a",
                "seconds": 7201, "max_seconds": 7200}}),
    ] {
        assert!(validator.is_valid(&valid), "the extension refused {valid}");
    }
    for invalid in [
        serde_json::json!({"kind": "unknown", "detail": "a fifth word"}),
        serde_json::json!({"kind": "ruled"}),
        serde_json::json!({"kind": "ruled", "detail": "x", "route": "dsh:a"}),
        serde_json::json!({"kind": "attempt-cap", "failure_class": "other", "detail": "x"}),
        serde_json::json!({"kind": "infrastructure", "detail": "x"}),
        serde_json::json!({"kind": "provider-limit", "provider": "dsh", "detail": "x",
            "refusal": {"kind": "route-unresolved", "route": "dsh:a"}}),
        serde_json::json!({"kind": "provider-limit", "provider": "dsh", "detail": "x",
            "refusal": {"kind": "host-unreadable"}}),
        serde_json::json!({"kind": "provider-limit", "provider": "dsh", "detail": "x",
            "refusal": {"kind": "past-maximum", "route": "dsh:a",
                "seconds": 7201, "max_seconds": 7200}}),
        serde_json::json!({"kind": "provider-limit", "provider": "dsh", "detail": "x",
            "refusal": {"kind": "no-end", "route": "dsh:a",
                "until": "2026-11-01T00:00:03Z", "seconds": 7201, "max_seconds": 7200}}),
        serde_json::json!({"kind": "provider-limit", "provider": "dsh", "detail": "x",
            "refusal": {"kind": "route-unresolved", "until": "2026-11-01T00:00:03Z"}}),
    ] {
        assert!(
            !validator.is_valid(&invalid),
            "the extension admitted {invalid}"
        );
    }
}
