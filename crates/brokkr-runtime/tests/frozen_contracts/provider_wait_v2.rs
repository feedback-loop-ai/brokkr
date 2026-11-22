//! Decision 0106 ruling 2: `provider-wait/v2` lands beside v1, which keeps
//! its bytes. A held entry is v2's alone: v1's closed entry refuses it by
//! the key it does not list, v2 reads a v1 entry as not held, and v2's
//! reader and schema agree on every held shape either can express.

use super::{a_failed_event, contract};

/// A held entry: a provider-stated reset of three days past a two-hour
/// maximum.
fn held_entry() -> serde_json::Value {
    serde_json::json!({
        "provider": "codex", "route": "first", "words": "usage limit",
        "retry_after_seconds": 259_200, "seconds": 259_200,
        "until": "2026-11-04T00:00:00Z", "rounds": 1,
        "held": true, "max_seconds": 7200
    })
}

/// The messages `validator` gives `instance`, joined: the schema's own
/// words for why it refused.
fn refusals(validator: &jsonschema::Validator, instance: &serde_json::Value) -> String {
    validator
        .iter_errors(instance)
        .map(|error| error.to_string())
        .collect::<Vec<_>>()
        .join("; ")
}

/// A reader that knows only v1 meets a held entry and refuses it by the
/// key v1 does not list, never reading it as an ordinary wait or as none.
#[test]
fn a_v1_reader_refuses_a_v2_held_wait_by_name() {
    let v1 = contract("contracts/provider-wait.v1.schema.json");
    let payload = serde_json::json!({"waits": [held_entry()]});
    let words = refusals(&v1, &payload);
    assert!(words.contains("'held'"), "{words}");
    assert!(words.contains("'max_seconds'"), "{words}");
    // And v2, beside it, admits the same entry.
    assert!(contract("contracts/provider-wait.v2.schema.json").is_valid(&payload));
}

/// A v1 entry satisfies v2's schema, and v2's reader reads it as an
/// ordinary wait.
#[test]
fn a_v2_reader_reads_a_v1_wait_as_not_held() {
    let v1_entry = serde_json::json!({
        "provider": "claude", "route": "first", "words": "rate_limit",
        "retry_after_seconds": 30, "seconds": 30,
        "until": "2026-11-01T00:00:30Z", "rounds": 1
    });
    let payload = serde_json::json!({"waits": [v1_entry]});
    assert!(contract("contracts/provider-wait.v1.schema.json").is_valid(&payload));
    assert!(contract("contracts/provider-wait.v2.schema.json").is_valid(&payload));
    let waits = brokkr_core::provider_waits(&a_failed_event(payload))
        .unwrap()
        .unwrap();
    assert_eq!(waits[0].held, None);
}

/// A held entry with no maximum is refused by the schema and by the reader,
/// which names `max_seconds`.
#[test]
fn a_held_entry_with_no_maximum_refuses_by_name() {
    let mut entry = held_entry();
    entry.as_object_mut().unwrap().remove("max_seconds");
    let payload = serde_json::json!({"waits": [entry]});
    assert!(!contract("contracts/provider-wait.v2.schema.json").is_valid(&payload));
    assert_eq!(
        brokkr_core::provider_waits(&a_failed_event(payload)).unwrap_err(),
        brokkr_core::ProviderWaitError::Field {
            seq: 1,
            field: "max_seconds".into(),
        }
    );
}

/// v2's reader and its schema agree on every held shape the schema can
/// express, with each verdict pinned so neither side passes by refusing or
/// admitting everything; the reader names the field each refusal is for.
#[test]
fn the_v2_reader_and_its_schema_agree_on_a_held_entry() {
    let v2 = contract("contracts/provider-wait.v2.schema.json");
    let with = |key: &str, value: serde_json::Value| {
        let mut entry = held_entry();
        entry[key] = value;
        entry
    };
    let without = |key: &str| {
        let mut entry = held_entry();
        entry.as_object_mut().unwrap().remove(key);
        entry
    };
    let refused = serde_json::json!({"cause": "past-maximum", "provider": "codex",
        "route": "first", "words": "usage limit", "seconds": 259_200, "max_seconds": 7200});
    for (case, payload, refusal) in [
        ("held", serde_json::json!({"waits": [held_entry()]}), None),
        (
            "held false",
            serde_json::json!({"waits": [with("held", false.into())]}),
            Some("held"),
        ),
        (
            "a maximum with no held",
            serde_json::json!({"waits": [without("held")]}),
            Some("max_seconds"),
        ),
        (
            "a held wait with no stated reset",
            serde_json::json!({"waits": [without("retry_after_seconds")]}),
            Some("retry_after_seconds"),
        ),
        (
            "a zero maximum",
            serde_json::json!({"waits": [with("max_seconds", 0.into())]}),
            Some("max_seconds"),
        ),
        (
            "a held wait beside a refused record",
            serde_json::json!({"waits": [held_entry()], "refused": refused}),
            Some("refused"),
        ),
    ] {
        let schema = v2.is_valid(&payload);
        let read = brokkr_core::provider_waits(&a_failed_event(payload));
        assert_eq!(
            schema,
            refusal.is_none(),
            "{case}: the schema said {schema}"
        );
        match refusal {
            None => assert_eq!(
                read.unwrap().unwrap()[0].held,
                Some(brokkr_core::Held { max_seconds: 7200 }),
                "{case}"
            ),
            Some(field) => assert_eq!(
                read.unwrap_err(),
                brokkr_core::ProviderWaitError::Field {
                    seq: 1,
                    field: field.into(),
                },
                "{case}"
            ),
        }
    }
    // Draft-07 cannot compare two sibling numbers, so the schema admits a
    // held length within its maximum; the reader refuses it by name.
    let within = serde_json::json!({"waits": [with("max_seconds", 259_200.into())]});
    assert!(
        v2.is_valid(&within),
        "draft-07 has no cross-property compare"
    );
    assert_eq!(
        brokkr_core::provider_waits(&a_failed_event(within)).unwrap_err(),
        brokkr_core::ProviderWaitError::Field {
            seq: 1,
            field: "max_seconds".into(),
        }
    );
}

/// Fix round 0106b1r10, H4: an unwritten wait's settlement marks itself
/// `wait_unwritten: true`, the one value v2 admits for the key.
#[test]
fn an_unwritten_waits_settlement_is_marked_true_and_only_true() {
    let v2 = contract("contracts/provider-wait.v2.schema.json");
    assert!(v2.is_valid(&serde_json::json!({"wait_unwritten": true})));
    let words = refusals(&v2, &serde_json::json!({"wait_unwritten": false}));
    assert!(words.contains("true was expected"), "{words}");
}
