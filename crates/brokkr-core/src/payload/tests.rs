use super::*;
use crate::committed_journals::committed_journals;
use crate::envelope_builder::EnvelopeBuilder;
use serde_json::json;
use EventType as Type;

fn event(event_type: EventType, payload: Value) -> EventEnvelope {
    EnvelopeBuilder::new(event_type, payload).seq(7).build()
}

/// A struct as it serializes, whole and its verbatim keys alone.
fn both<T: Serialize>(typed: &T, rest: &Verbatim) -> (Value, Value) {
    (json!(typed), json!(rest))
}

/// What `event`'s struct reads back to, or `None` for a type the fold
/// reads nothing from. A decision keeps no verbatim keys: its contract
/// is closed.
fn read_back(event: &EventEnvelope) -> Option<Result<(Value, Value), FoldError>> {
    Some(match event.event_type {
        Type::RunStarted => {
            let read = RunStarted::read(event);
            Ok(both(&read, &read.rest))
        }
        Type::PhaseEntered => PhaseEntered::read(event).map(|read| both(&read, &read.rest)),
        Type::EffectRequested => EffectRequested::read(event).map(|read| both(&read, &read.rest)),
        Type::EffectStarted => EffectStarted::read(event).map(|read| both(&read, &read.rest)),
        Type::EffectCheckpointed => {
            EffectCheckpointed::read(event).map(|read| both(&read, &read.rest))
        }
        Type::EffectSucceeded => {
            EffectSucceeded::read(event, |_| Ok(())).map(|read| both(&read, &read.rest))
        }
        Type::EffectFailed => EffectFailed::read(event).map(|read| both(&read, &read.rest)),
        Type::EffectIndeterminate => {
            EffectIndeterminate::read(event).map(|read| both(&read, &read.rest))
        }
        Type::TransitionDecided => decided(event).map(|read| (json!(read), json!({}))),
        Type::OperatorCommanded => {
            OperatorCommanded::read(event).map(|read| both(&read, &read.rest))
        }
        Type::OperatorAccepted => OperatorAccepted::read(event).map(|read| both(&read, &read.rest)),
        Type::RunParked => RunParked::read(event).map(|read| both(&read, &read.rest)),
        Type::OperatorRejected | Type::RunCompleted | Type::RunStopped => return None,
    })
}

/// How the fold reads a typed field.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    /// A string, refused by name when it is not one.
    Required,
    /// Any value, `null` included, refused by name when it is absent.
    Present,
    /// A string when it is one, and nothing otherwise.
    Lenient,
}

/// Every typed field of every struct, in the order the fold reads it.
const FIELDS: [(EventType, &[(&str, Kind)]); 11] = {
    use Kind::*;
    [
        (Type::RunStarted, &[("feature", Lenient)]),
        (Type::PhaseEntered, &[("phase", Required)]),
        (
            Type::EffectRequested,
            &[("effect_id", Required), ("seat", Required)],
        ),
        (
            Type::EffectStarted,
            &[("effect_id", Required), ("attempt_id", Required)],
        ),
        (Type::EffectCheckpointed, &[("effect_id", Required)]),
        (
            Type::EffectSucceeded,
            &[("effect_id", Required), ("result", Present)],
        ),
        (Type::EffectFailed, &[("effect_id", Required)]),
        (
            Type::EffectIndeterminate,
            &[("effect_id", Required), ("reason", Lenient)],
        ),
        (
            Type::OperatorCommanded,
            &[("command_id", Required), ("command", Required)],
        ),
        (Type::OperatorAccepted, &[("command_id", Required)]),
        (Type::RunParked, &[("reason", Required)]),
    ]
};

/// Whether the fold's read before #345 took `value` for a field of
/// `kind`: `payload_str`'s string, the result's `get`, or the lenient
/// `as_str`.
fn took(kind: Kind, value: Option<&Value>) -> bool {
    match kind {
        Kind::Required | Kind::Lenient => value.and_then(Value::as_str).is_some(),
        Kind::Present => value.is_some(),
    }
}

/// A payload whose every typed field reads, beside two keys no struct
/// types: an extension recorded as `null`, and a float that must keep
/// its own bytes.
fn readable(fields: &[(&str, Kind)]) -> Map<String, Value> {
    let mut payload = Map::new();
    for (field, _) in fields {
        payload.insert(field.to_string(), json!(format!("{field}-1")));
    }
    payload.insert("extension".into(), Value::Null);
    payload.insert("measured".into(), json!(1.0));
    payload
}

/// Every typed field, absent, `null`, mistyped and present, in a payload
/// whose other fields read: the struct refuses exactly what the fold's
/// old read refused, by the same name, and otherwise reads back to the
/// payload it was read from, its verbatim keys exactly those it did not
/// take.
#[test]
fn every_typed_field_reads_as_the_fold_always_read_it() {
    let variants = [None, Some(Value::Null), Some(json!(7)), Some(json!("word"))];
    for (event_type, fields) in FIELDS {
        for &(field, kind) in fields {
            for variant in &variants {
                let mut payload = readable(fields);
                match variant {
                    Some(value) => payload.insert(field.into(), value.clone()),
                    None => payload.remove(field),
                };
                let mut rest = payload.clone();
                rest.retain(|key, value| {
                    !fields
                        .iter()
                        .any(|&(typed, kind)| typed == key && took(kind, Some(value)))
                });
                let expected = match took(kind, variant.as_ref()) || kind == Kind::Lenient {
                    true => Ok((Value::Object(payload.clone()), Value::Object(rest))),
                    false => Err(FoldError::BadPayload {
                        seq: 7,
                        field: field.into(),
                    }),
                };
                let read = read_back(&event(event_type, Value::Object(payload)));
                assert_eq!(read, Some(expected), "{event_type:?} {field} {variant:?}");
            }
        }
    }
}

/// A payload with no field at all is refused for the first field the
/// fold reads, and a `transition/decided` names `from` and `result` in
/// turn before it reads as a closed decision.
#[test]
fn an_empty_payload_is_refused_for_the_first_field_read() {
    for (event_type, fields) in FIELDS {
        let read = read_back(&event(event_type, json!({}))).unwrap();
        match fields.iter().find(|&&(_, kind)| kind != Kind::Lenient) {
            Some(&(field, _)) => assert_eq!(
                read,
                Err(FoldError::BadPayload {
                    seq: 7,
                    field: field.into(),
                }),
                "{event_type:?}"
            ),
            None => assert_eq!(read, Ok((json!({}), json!({}))), "{event_type:?}"),
        }
    }
    let decided = |payload| read_back(&event(Type::TransitionDecided, payload)).unwrap();
    let refused = |field: &str| {
        Err(FoldError::BadPayload {
            seq: 7,
            field: field.into(),
        })
    };
    assert_eq!(decided(json!({"result": "done"})), refused("from"));
    assert_eq!(
        decided(json!({"from": "work", "result": 1})),
        refused("result")
    );
    assert_eq!(
        decided(json!({"from": "work", "result": "done", "case": 1})),
        Err(FoldError::BadDecision {
            seq: 7,
            detail: "unknown field `case`, expected one of `from`, `inputs`, `next`, \
                     `problem`, `result`, `rule_id`, `severity`"
                .into(),
        })
    );
    let ruling = json!({"from": "work", "inputs": null, "result": "done"});
    assert_eq!(decided(ruling.clone()), Ok((ruling, json!({}))));
}

/// A payload outside the contract's object refuses every field it must
/// carry, and reads as an empty object where nothing is required.
#[test]
fn a_payload_that_is_not_an_object_reads_as_an_empty_one() {
    let listed = json!(["feature", "phase"]);
    assert_eq!(
        read_back(&event(Type::RunStarted, listed.clone())),
        Some(Ok((json!({}), json!({}))))
    );
    assert_eq!(
        read_back(&event(Type::PhaseEntered, listed)),
        Some(Err(FoldError::BadPayload {
            seq: 7,
            field: "phase".into(),
        }))
    );
}

/// An effect the fold cannot place is refused for its place once its
/// target reads, before its result is read: a succeeded event with no
/// result is out of place, not malformed.
#[test]
fn an_unplaced_success_is_refused_before_its_result_is_read() {
    let refusal = FoldError::OutOfPlace {
        seq: 7,
        event: "EffectSucceeded".into(),
        cursor: "Stop".into(),
    };
    let named = event(Type::EffectSucceeded, json!({"effect_id": "e"}));
    assert_eq!(unplaced(&named, refusal.clone()), refusal);
    let unnamed = event(Type::EffectSucceeded, json!({"result": {}}));
    assert_eq!(
        unplaced(&unnamed, refusal),
        FoldError::BadPayload {
            seq: 7,
            field: "effect_id".into(),
        }
    );
}

/// Every payload the committed journals hold, of every type the fold
/// reads, reads and serializes back to exactly the payload it was read
/// from: no journal byte moves (#345). The three types the fold reads
/// nothing from (12 events) are left out of the count.
#[test]
fn every_committed_payload_reads_back_to_its_own_bytes() {
    let mut read = 0;
    for (path, events) in committed_journals() {
        for event in &events {
            let place = format!("{} seq {}", path.display(), event.seq);
            let Some(typed) = read_back(event) else {
                continue;
            };
            let (whole, _) = typed.expect(&place);
            assert_eq!(whole, event.payload, "{place}");
            read += 1;
        }
    }
    assert_eq!(
        read, 3_237,
        "every committed payload the fold reads was read"
    );
}
