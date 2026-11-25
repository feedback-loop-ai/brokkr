//! `spec_refusal.rs`'s own unit tests: each check of a site's `hands`
//! value names its own variant, in today's order, and every variant reads
//! as the text the operator has always seen (#353).

use serde_json::{json, Map, Value};

use super::*;
use crate::hands::{Bind, BindMode, HandsSpec};

/// A `workspace` object with `fields` set over its kind; a null field is
/// dropped, so `{"kind": null}` is an object without a kind.
fn spec(fields: Value) -> Value {
    let mut object = Map::new();
    object.insert("kind".to_string(), json!("workspace"));
    object.extend(fields.as_object().unwrap().clone());
    object.retain(|_, value| !value.is_null());
    Value::Object(object)
}

#[test]
fn each_spec_check_refuses_in_its_own_variant() {
    let refused = |raw: &Value, refusal: SpecRefusal| {
        assert_eq!(HandsSpec::checked(raw), Err(refusal), "{raw}");
    };
    let mitts = json!("mitts");
    refused(
        &mitts,
        SpecRefusal::UnknownKind {
            kind: "mitts".to_string(),
        },
    );
    for got in [json!(7), Value::Null] {
        refused(
            &got,
            SpecRefusal::NotKindOrObject {
                got: got.to_string(),
            },
        );
    }
    refused(&spec(json!({"boundary": "x"})), SpecRefusal::Boundary);
    refused(
        &spec(json!({"artifacts": "rw"})),
        SpecRefusal::UnknownKey {
            key: "artifacts".to_string(),
        },
    );
    for kind in [Value::Null, json!("mitten"), json!(7)] {
        refused(&spec(json!({"kind": kind})), SpecRefusal::Kind);
    }
    let yes = json!("yes");
    refused(
        &spec(json!({"network": yes})),
        SpecRefusal::Network {
            got: yes.to_string(),
        },
    );
    refused(&spec(json!({"binds": {}})), SpecRefusal::BindsNotArray);
    refused(
        &spec(json!({"binds": [7]})),
        SpecRefusal::Bind(BindRefusal::NotObject),
    );
}

#[test]
fn the_spec_checks_fire_in_today_s_order() {
    let first = |raw: Value, refusal: SpecRefusal| {
        assert_eq!(HandsSpec::checked(&raw), Err(refusal), "{raw}");
    };
    first(
        spec(json!({"kind": null, "mittens": 1})),
        SpecRefusal::UnknownKey {
            key: "mittens".to_string(),
        },
    );
    first(
        spec(json!({"boundary": 1, "mittens": 1})),
        SpecRefusal::Boundary,
    );
    let got = json!(1);
    first(
        spec(json!({"network": got, "binds": "all"})),
        SpecRefusal::Network {
            got: got.to_string(),
        },
    );
}

#[test]
fn an_admitted_spec_reads_as_its_exact_hands() {
    let admitted = |raw: Value, hands: HandsSpec| {
        assert_eq!(HandsSpec::checked(&raw), Ok(hands), "{raw}");
    };
    admitted(json!("workspace"), HandsSpec::default());
    admitted(spec(json!({})), HandsSpec::default());
    let bind = json!({"path": "/srv/cache", "mode": "rw"});
    admitted(
        spec(json!({"network": true, "binds": [bind]})),
        HandsSpec {
            network: true,
            binds: vec![Bind {
                path: "/srv/cache".to_string(),
                mode: BindMode::Rw,
                mask: Vec::new(),
            }],
        },
    );
}

#[test]
fn every_spec_refusal_reads_as_it_always_has() {
    for (refusal, text) in [
        (
            SpecRefusal::UnknownKind {
                kind: "mitts".to_string(),
            },
            "hands 'mitts' is not a known kind; the vocabulary is: workspace",
        ),
        (
            SpecRefusal::NotKindOrObject {
                got: "[1]".to_string(),
            },
            "hands must be \"workspace\" or an object, got [1]",
        ),
        (
            SpecRefusal::Boundary,
            "hands names 'boundary'; the boundary is declared by the realm \
             (realms.json, forge.realms/v4) and never by a bundle or an agent, \
             because the machine a realm runs on is the realm's fact \
             (decision 0046 ruling 1)",
        ),
        (
            SpecRefusal::UnknownKey {
                key: "artifacts".to_string(),
            },
            "hands has unknown key 'artifacts'; known: kind, network, binds",
        ),
        (SpecRefusal::Kind, "hands.kind must be \"workspace\""),
        (
            SpecRefusal::Network {
                got: "\"on\"".to_string(),
            },
            "hands.network must be a boolean, got \"on\"",
        ),
        (SpecRefusal::BindsNotArray, "hands.binds must be an array"),
        (
            SpecRefusal::Bind(BindRefusal::NoPath),
            "hands.binds entry needs a non-empty 'path'",
        ),
    ] {
        assert_eq!(refusal.to_string(), text);
    }
}
