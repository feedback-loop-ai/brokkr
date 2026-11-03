//! The typed refusal at the transport edge (decision 0103 ruling 1(a)):
//! a present object that cannot be read refuses by name, never as absent,
//! and a `probe` in `supports` is not the advertisement.

use super::*;

/// A failed result carrying `refusal` verbatim.
fn refusal_result(refusal: serde_json::Value) -> String {
    serde_json::json!({
        "proto": PROTO,
        "msg_id": "result",
        "type": "result",
        "effect_id": "effect",
        "attempt_id": "attempt",
        "status": "failed",
        "refusal": refusal,
    })
    .to_string()
}

/// Drive `result` through a real driver process and read the report.
fn report_of(result: String) -> AttemptReport {
    let script = format!(
        "read -r line; printf '%s\\n' '{}'; read -r line; printf '%s\\n' '{}'",
        capabilities(),
        result
    );
    run(&script)
}

/// Run `script` as a probe and return its report.
fn probe_report(script: &str) -> AttemptReport {
    DriverProcess::spawn(
        &command(script),
        std::path::Path::new("."),
        None,
        &SpawnEnv::Inherit,
    )
    .unwrap()
    .probing()
    .run_attempt("test", "effect", "attempt", "seat", json!({}), |_| {})
}

#[test]
fn a_readable_refusal_is_carried_as_the_typed_object() {
    let report = report_of(refusal_result(serde_json::json!({
        "class": "rate_limit",
        "provider": "claude",
        "route": null,
        "words": "rate_limit (HTTP 429): slow down",
        "status": 429,
        "retry_after_seconds": 30,
    })));
    let refusal = report.refusal.expect("the typed refusal");
    assert_eq!(refusal.class(), crate::RefusalClass::RateLimit);
    assert_eq!(refusal.provider(), "claude");
    assert_eq!(refusal.route(), None);
    assert_eq!(refusal.status(), Some(429));
    assert_eq!(refusal.retry_after_seconds(), Some(30));
    assert_eq!(refusal.words(), "rate_limit (HTTP 429): slow down");
    assert_eq!(
        report.outcome,
        AttemptOutcome::Failed {
            error: "driver reported failure".into()
        }
    );
}

#[test]
fn a_present_but_malformed_refusal_refuses_by_name_never_as_absent() {
    for (refusal, named) in [
        (serde_json::Value::Null, "not an object"),
        (
            serde_json::json!({"class": "invented", "provider": "claude", "route": null,
                "words": "w"}),
            "`class`",
        ),
        (
            serde_json::json!({"class": "rate_limit", "route": null, "words": "w"}),
            "names no provider",
        ),
        (
            serde_json::json!({"class": "rate_limit", "provider": "claude", "words": "w"}),
            "names no `route`",
        ),
        (
            serde_json::json!({"class": "rate_limit", "provider": "claude", "route": null,
                "words": "w", "retry_after_seconds": 0}),
            "`retry_after_seconds`",
        ),
    ] {
        let report = report_of(refusal_result(refusal));
        assert_eq!(report.refusal, None);
        let AttemptOutcome::Indeterminate { reason } = &report.outcome else {
            panic!("a malformed present refusal must park: {report:?}");
        };
        assert!(reason.contains("driver refusal is unreadable:"), "{reason}");
        assert!(reason.contains(named), "{reason}");
        assert!(
            !reason.contains("unreadable driver message"),
            "the field, not the bytes, is named: {reason}"
        );
    }
}

#[test]
fn a_supports_probe_is_not_the_advertisement_and_a_probe_is_refused() {
    let capabilities = wire(Body::Capabilities {
        driver: "test".into(),
        version: "1".into(),
        supports: vec!["probe".into()],
        probe: None,
    });
    let script = format!("read -r line; printf '%s\\n' '{capabilities}'");
    let report = probe_report(&script);
    assert_eq!(
        report.outcome,
        AttemptOutcome::Failed {
            error: "driver does not advertise the probe; refusing to probe it".into()
        }
    );
    assert!(!report.accepted, "no start was sent");
}

#[test]
fn a_probe_start_carries_the_marker_to_an_advertising_driver() {
    let capabilities = wire(Body::Capabilities {
        driver: "test".into(),
        version: "1".into(),
        supports: vec![],
        probe: Some(Probe),
    });
    let answered = wire(Body::Result {
        refusal: None,
        effect_id: "effect".into(),
        attempt_id: "attempt".into(),
        status: ResultStatus::Succeeded,
        result: Some(serde_json::json!({"result": "complete"})),
        error: None,
    });
    let missed = wire(Body::Result {
        refusal: None,
        effect_id: "effect".into(),
        attempt_id: "attempt".into(),
        status: ResultStatus::Failed,
        result: None,
        error: Some("the start carried no probe marker".into()),
    });
    let script = format!(
        "read -r line; printf '%s\\n' '{capabilities}'; read -r start; \
         case \"$start\" in *'\"probe\":true'*) printf '%s\\n' '{answered}' ;; \
         *) printf '%s\\n' '{missed}' ;; esac"
    );
    let report = probe_report(&script);
    assert_eq!(
        report.outcome,
        AttemptOutcome::Succeeded {
            result: serde_json::json!({"result": "complete"})
        }
    );
}

/// The refusal object's closed shape is read field by field at the edge
/// (decision 0103 ruling 1(a); decision 0071 ruling 3): every malformed or
/// unknown member is refused by the field that carries it, never silently
/// dropped or read as an absence.
#[test]
fn the_refusal_reader_refuses_every_malformed_field_by_name() {
    use crate::{PreSessionRefusal, RefusalError, DRIVER_REFUSAL_SCHEMA};

    let base = || {
        serde_json::json!({
            "class": "rate_limit",
            "provider": "claude",
            "route": null,
            "words": "rate_limit",
        })
    };
    let with = |key: &str, value: serde_json::Value| {
        let mut object = base();
        object[key] = value;
        object
    };
    let cases = [
        (
            "a member outside the closed schema",
            with("extra", serde_json::json!(true)),
            RefusalError::Unknown(DRIVER_REFUSAL_SCHEMA),
        ),
        (
            "a route that is not an identifier",
            with("route", serde_json::json!("bad/route")),
            RefusalError::RouteForm,
        ),
        (
            "empty words",
            with("words", serde_json::json!("")),
            RefusalError::Words,
        ),
        (
            "words past the bound",
            with("words", serde_json::json!("w".repeat(301))),
            RefusalError::Words,
        ),
        (
            "a status below the HTTP range",
            with("status", serde_json::json!(99)),
            RefusalError::Status,
        ),
        (
            "a status above the HTTP range",
            with("status", serde_json::json!(600)),
            RefusalError::Status,
        ),
        (
            "a provider whose first character is not alphanumeric",
            with("provider", serde_json::json!("-claude")),
            RefusalError::Provider,
        ),
        (
            "a provider past the length bound",
            with("provider", serde_json::json!("c".repeat(81))),
            RefusalError::Provider,
        ),
        (
            "a provider with a character outside the grammar",
            with("provider", serde_json::json!("claude/v2")),
            RefusalError::Provider,
        ),
    ];
    for (case, object, expected) in cases {
        assert_eq!(
            PreSessionRefusal::read(&object),
            Err(expected.clone()),
            "{case}: {object}"
        );
    }
}

/// The probe marker is a typed `true`, never a doubly-optional boolean
/// (decision 0103 ruling 4(a)): a present `false` is refused where it is
/// read, and only an absent field means an ordinary attempt.
#[test]
fn a_present_false_probe_marker_is_refused() {
    let refused = serde_json::from_str::<Probe>("false").expect_err("false is not the marker");
    // The serde error is a data error at this edge, and its complete
    // diagnostic is pinned: a different message or category is a different
    // contract (#430 M1; decision 0071 rulings 8 and 9).
    assert_eq!(refused.classify(), serde_json::error::Category::Data);
    assert_eq!(
        refused.to_string(),
        "the probe marker is present but not true"
    );
}
