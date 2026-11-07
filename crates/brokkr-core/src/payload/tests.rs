use super::*;
use crate::committed_journals::committed_journals;
use crate::envelope_builder::EnvelopeBuilder;
use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
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

/// Decision 0103 ruling 2(a): the `waits` extension reads into its typed
/// entry, carrying the entry's own `seconds` (decision 0103 ruling 3).
#[test]
fn the_provider_wait_extension_reads() {
    let valid = event(
        Type::EffectFailed,
        json!({"effect_id": "e", "waits": [{
            "site": "a",
            "provider": "claude",
            "route": "anthropic",
            "words": "rate_limit (HTTP 429)",
            "retry_after_seconds": 30,
            "seconds": 30,
            "until": "2026-11-01T00:00:30Z",
            "rounds": 2
        }]}),
    );
    let waits = provider_waits(&valid).unwrap().unwrap();
    assert_eq!(waits.len(), 1);
    assert_eq!(waits[0].site.as_deref(), Some("a"));
    assert_eq!(waits[0].provider, "claude");
    assert_eq!(waits[0].route, "anthropic");
    assert_eq!(waits[0].words, "rate_limit (HTTP 429)");
    assert_eq!(waits[0].retry_after_seconds, Some(30));
    assert_eq!(waits[0].seconds, 30);
    assert_eq!(waits[0].rounds, 2);
    assert_eq!(
        waits[0].until,
        OffsetDateTime::parse("2026-11-01T00:00:30Z", &Rfc3339).unwrap()
    );

    // An event with no `waits` is no wait, not a malformed one.
    let none = event(Type::EffectFailed, json!({"effect_id": "e"}));
    assert_eq!(provider_waits(&none).unwrap(), None);
}

/// The reader refuses `payload` naming `field`, never as no wait.
fn refuses_by_name(payload: Value, field: &str) {
    let error = provider_waits(&event(Type::EffectFailed, payload)).unwrap_err();
    assert!(
        matches!(&error, ProviderWaitError::Field { field: named, .. } if named == field),
        "{error:?} did not name {field}"
    );
}
/// A present `waits` entry that does not read refuses by the field's own
/// name, never as no wait. The reader carries the schema's own bounds
/// (decision 0103 ruling 2, step 9): a word past `maxLength`, a provider
/// or route outside its pattern, and the entry's required `seconds`.
#[test]
fn the_provider_wait_extension_refuses_by_name() {
    for (payload, field) in [
        (json!({"waits": {}}), "waits"),
        (json!({"waits": []}), "waits"),
        (
            json!({"waits": [{"route": "r", "words": "w", "seconds": 1,
                "until": "2026-11-01T00:00:00Z", "rounds": 1}]}),
            "provider",
        ),
        (
            json!({"waits": [{"provider": "p", "words": "w", "seconds": 1,
                "until": "2026-11-01T00:00:00Z", "rounds": 1}]}),
            "route",
        ),
        (
            json!({"waits": [{"provider": "p", "route": "r", "words": "",
                "seconds": 1, "until": "2026-11-01T00:00:00Z", "rounds": 1}]}),
            "words",
        ),
        (
            json!({"waits": [{"provider": "p/r", "route": "r", "words": "w",
                "seconds": 1, "until": "2026-11-01T00:00:00Z", "rounds": 1}]}),
            "provider",
        ),
        (
            json!({"waits": [{"provider": "p", "route": "r/x", "words": "w",
                "seconds": 1, "until": "2026-11-01T00:00:00Z", "rounds": 1}]}),
            "route",
        ),
        (
            json!({"waits": [{"provider": "p", "route": "r", "words": "w",
                "until": "2026-11-01T00:00:00Z", "rounds": 1}]}),
            "seconds",
        ),
        (
            json!({"waits": [{"provider": "p", "route": "r", "words": "w",
                "seconds": 0, "until": "2026-11-01T00:00:00Z", "rounds": 1}]}),
            "seconds",
        ),
        (
            json!({"waits": [{"provider": "p", "route": "r", "words": "w",
                "retry_after_seconds": 30, "seconds": 31,
                "until": "2026-11-01T00:00:00Z", "rounds": 1}]}),
            "seconds",
        ),
        (
            json!({"waits": [{"provider": "p", "route": "r", "words": "w",
                "seconds": 1, "until": "not a time", "rounds": 1}]}),
            "until",
        ),
        (
            json!({"waits": [{"provider": "p", "route": "r", "words": "w",
                "seconds": 1, "until": "2026-11-01T00:00:00Z"}]}),
            "rounds",
        ),
        (
            json!({"waits": [{"provider": "p", "route": "r", "words": "w",
                "seconds": 1, "until": "2026-11-01T00:00:00Z", "rounds": 0}]}),
            "rounds",
        ),
        (
            json!({"waits": [{"provider": "p", "route": "r", "words": "w",
                "seconds": 1, "until": "2026-11-01T00:00:00Z", "rounds": 1,
                "retry_after_seconds": 0}]}),
            "retry_after_seconds",
        ),
        (
            json!({"waits": [{"provider": "p", "route": "r", "words": "w",
                "seconds": 1, "until": "2026-11-01T00:00:00Z", "rounds": 1,
                "site": null}]}),
            "site",
        ),
        (
            json!({"waits": [{"provider": "p", "route": "r", "words": "w",
                "seconds": 1, "until": "2026-11-01T00:00:00Z", "rounds": 1,
                "other": 1}]}),
            "other",
        ),
        (
            json!({"waits": [{"provider": "p", "route": "r", "words": "w",
                "seconds": 1, "until": "2026-11-01T00:00:00Z", "rounds": 1}, 7]}),
            "waits",
        ),
    ] {
        refuses_by_name(payload, field);
    }
}

/// The reader refuses the two schema `maxLength` bounds the old reader let
/// through: a wait's words past 300 characters and a site past 200.
#[test]
fn the_provider_wait_extension_bounds_words_and_site() {
    let long_words = "w".repeat(301);
    let long_site = "s".repeat(201);
    for (payload, field) in [
        (
            json!({"waits": [{"provider": "p", "route": "r", "words": long_words,
                "seconds": 1, "until": "2026-11-01T00:00:00Z", "rounds": 1}]}),
            "words",
        ),
        (
            json!({"waits": [{"provider": "p", "route": "r", "words": "w",
                "seconds": 1, "until": "2026-11-01T00:00:00Z", "rounds": 1,
                "site": long_site}]}),
            "site",
        ),
    ] {
        refuses_by_name(payload, field);
    }
}

/// Decision 0103 slice 4c1, this fix round: the `refused` record reads into
/// its typed fields, and a present one that does not read refuses by name,
/// never as absent.
#[test]
fn the_refused_record_reads_and_refuses_by_name() {
    let valid = json!({
        "cause": "past-maximum", "provider": "claude", "route": "first",
        "words": "rate_limit: slow down", "retry_after_seconds": 5,
        "seconds": 5, "max_seconds": 2
    });
    let recorded = event(
        Type::EffectFailed,
        json!({"effect_id": "e", "refused": valid}),
    );
    let refused = provider_refusal(&recorded).unwrap().unwrap();
    assert_eq!(refused.cause, RefusalCause::PastMaximum);
    assert_eq!(refused.provider, "claude");
    assert_eq!(refused.route.as_deref(), Some("first"));
    assert_eq!(refused.words, "rate_limit: slow down");
    assert_eq!(refused.retry_after_seconds, Some(5));
    assert_eq!(refused.seconds, Some(5));
    assert_eq!(refused.max_seconds, Some(2));
    assert_eq!(json!(refused), valid, "the record writes its own shape");
    // The invocation-site tag reads back where present, and is absent for
    // an inline seat, as `waits.site` is.
    let cased = event(
        Type::EffectFailed,
        json!({"effect_id": "e", "refused": {
            "cause": "past-maximum", "site": "work:chore", "provider": "claude",
            "route": "first", "words": "rate_limit: slow down",
            "seconds": 5, "max_seconds": 2
        }}),
    );
    let refused = provider_refusal(&cased).unwrap().unwrap();
    assert_eq!(refused.site.as_deref(), Some("work:chore"));
    assert_eq!(json!(refused)["site"], "work:chore");
    assert!(
        json!(provider_refusal(&recorded).unwrap().unwrap())
            .get("site")
            .is_none(),
        "an inline seat's refusal carries no site"
    );

    // An unresolved route names none; a host refusal names one and no
    // computed length. Absent is no refusal, never a malformed one.
    let unresolved = event(
        Type::EffectFailed,
        json!({"effect_id": "e", "refused": {
            "cause": "route-unresolved", "provider": "claude",
            "words": "rate_limit"
        }}),
    );
    let refused = provider_refusal(&unresolved).unwrap().unwrap();
    assert_eq!(refused.cause, RefusalCause::RouteUnresolved);
    assert_eq!(refused.route, None);
    let host = event(
        Type::EffectFailed,
        json!({"effect_id": "e", "refused": {
            "cause": "host-unreadable", "provider": "claude", "route": "first",
            "words": "rate_limit"
        }}),
    );
    assert_eq!(
        provider_refusal(&host).unwrap().unwrap().cause,
        RefusalCause::HostUnreadable
    );
    assert_eq!(
        provider_refusal(&event(Type::EffectFailed, json!({"effect_id": "e"}))).unwrap(),
        None
    );
}

/// The field a malformed `refused` object is refused by, never as absent.
#[test]
fn the_refused_record_refuses_by_name() {
    let refuses = |refused: Value, field: &str| {
        let error = provider_refusal(&event(
            Type::EffectFailed,
            json!({"effect_id": "e", "refused": refused}),
        ))
        .unwrap_err();
        assert!(
            matches!(&error, ProviderRefusalError::Field { field: named, .. } if named == field),
            "{error:?} did not name {field}"
        );
    };
    let valid = json!({
        "cause": "past-maximum", "provider": "claude", "route": "first",
        "words": "w", "seconds": 1, "max_seconds": 1
    });
    let with = |key: &str, value: Value| {
        let mut refused = valid.clone();
        refused[key] = value;
        refused
    };
    let mut without = valid.clone();
    without.as_object_mut().unwrap().remove("seconds");
    let mut without_max = valid.clone();
    without_max.as_object_mut().unwrap().remove("max_seconds");
    refuses(json!(7), "refused");
    refuses(with("cause", "no-such-cause".into()), "cause");
    refuses(with("provider", "p/r".into()), "provider");
    refuses(with("route", "r/x".into()), "route");
    refuses(with("words", "".into()), "words");
    refuses(with("words", "w".repeat(301).into()), "words");
    refuses(with("retry_after_seconds", 0.into()), "retry_after_seconds");
    refuses(with("seconds", 0.into()), "seconds");
    refuses(with("extra", 1.into()), "extra");
    refuses(without, "seconds");
    refuses(without_max, "max_seconds");
    // A resolved cause without a route, and one whose route is absent
    // where the cause says none was resolved.
    let mut route_less = valid.clone();
    route_less.as_object_mut().unwrap().remove("route");
    refuses(route_less, "route");
    refuses(
        json!({"cause": "route-unresolved", "provider": "claude",
               "route": "first", "words": "w"}),
        "route",
    );
    // Only a cause that computed a length may carry one.
    refuses(
        json!({"cause": "host-unreadable", "provider": "claude", "route": "first",
               "words": "w", "seconds": 1}),
        "seconds",
    );
    refuses(
        json!({"cause": "host-unreadable", "provider": "claude", "route": "first",
               "words": "w", "max_seconds": 1}),
        "max_seconds",
    );
    refuses(
        json!({"cause": "host-unreadable", "provider": "claude", "route": "first",
               "words": "w", "seconds": 1, "max_seconds": 1}),
        "seconds",
    );
}

/// The retry's `probes` array reads back: two entries, each typed, and the
/// site tag present only where the probe was asked at one. A present array
/// that does not read refuses by name, never as no answer; the retired
/// single `probe` object is refused too, never read as absent.
#[test]
fn the_probes_read_and_refuse_the_retired_single_probe() {
    let answered = json!({"provider": "claude", "route": "first",
                          "answered_at": "2026-11-01T00:00:31Z"});
    let two = event(
        Type::EffectStarted,
        json!({"probes": [
            answered.clone(),
            {"site": "work:chore", "provider": "codex", "route": "second",
             "answered_at": "2026-11-01T00:00:32Z"}
        ]}),
    );
    let probes = provider_probes(&two).unwrap().unwrap();
    assert_eq!(probes.len(), 2);
    assert_eq!(probes[0].site, None);
    assert_eq!(probes[0].provider, "claude");
    assert_eq!(probes[1].site.as_deref(), Some("work:chore"));
    assert_eq!(probes[1].route, "second");
    assert_eq!(
        probes[1].answered_at,
        OffsetDateTime::parse("2026-11-01T00:00:32Z", &Rfc3339).unwrap()
    );
    assert_eq!(
        provider_probes(&event(Type::EffectStarted, json!({}))).unwrap(),
        None,
        "an effect/started with no probes answered"
    );
    let refuses = |payload: Value, field: &str| {
        let error = provider_probes(&event(Type::EffectStarted, payload)).unwrap_err();
        assert!(
            matches!(&error, ProviderProbeError::Field { field: named, .. } if named == field),
            "{error:?} did not name {field}"
        );
    };
    refuses(json!({"probes": []}), "probes");
    refuses(json!({"probes": {}}), "probes");
    refuses(
        json!({"probes": [{"provider": "claude", "route": "first"}]}),
        "answered_at",
    );
    refuses(
        json!({"probes": [{"provider": "claude", "route": "first",
                           "answered_at": "not a time"}]}),
        "answered_at",
    );
    refuses(
        json!({"probes": [{"provider": "p/r", "route": "first",
                           "answered_at": "2026-11-01T00:00:31Z"}]}),
        "provider",
    );
    refuses(
        json!({"probes": [{"provider": "claude", "route": "r/x",
                           "answered_at": "2026-11-01T00:00:31Z"}]}),
        "route",
    );
    refuses(
        json!({"probes": [{"site": "s".repeat(201), "provider": "claude",
                           "route": "first",
                           "answered_at": "2026-11-01T00:00:31Z"}]}),
        "site",
    );
    refuses(
        json!({"probes": [{"provider": "claude", "route": "first",
                           "answered_at": "2026-11-01T00:00:31Z", "extra": 1}]}),
        "extra",
    );
    refuses(json!({"probe": answered}), "probe");
    let error = provider_probes(&event(Type::EffectStarted, json!({"probes": []}))).unwrap_err();
    assert_eq!(
        error.to_string(),
        "event 7: the probes entry does not read: probes"
    );
    // The serializer writes an answer's site where it has one, and omits
    // it for an inline seat.
    let cased = ProbeAnswer {
        site: Some("work:chore".into()),
        provider: "claude".into(),
        route: "first".into(),
        answered_at: OffsetDateTime::parse("2026-11-01T00:00:31Z", &Rfc3339).unwrap(),
    };
    assert_eq!(json!(cased)["site"], "work:chore");
    let inline = ProbeAnswer {
        site: None,
        ..cased
    };
    assert!(json!(inline).get("site").is_none());
}

/// A `probes` entry that is not an object — a string, number, array or
/// null — is a present entry that does not read, refused by the array's
/// name, never skipped as no answer (decision 0103 ruling 4).
#[test]
fn a_probes_entry_that_is_not_an_object_refuses_by_the_arrays_name() {
    for entry in [json!("claude"), json!(7), json!(["claude"]), Value::Null] {
        let error = provider_probes(&event(Type::EffectStarted, json!({"probes": [entry]})))
            .expect_err("a non-object probe entry was read");
        assert!(
            matches!(&error, ProviderProbeError::Field { field, .. } if field == "probes"),
            "{error:?} did not name the entry's array"
        );
    }
}

/// The probe answer's own serializer writes its optional site and refuses a
/// time RFC 3339 cannot spell: the arm the production clock never reaches
/// is reachable, and says why.
#[test]
fn a_probe_answer_the_rfc3339_formatter_cannot_spell_refuses_to_serialize() {
    let answer = ProbeAnswer {
        site: None,
        provider: "claude".into(),
        route: "first".into(),
        answered_at: time::OffsetDateTime::new_utc(
            time::Date::from_calendar_date(-1, time::Month::January, 1).unwrap(),
            time::Time::MIDNIGHT,
        ),
    };
    let error = serde_json::to_value(&answer).unwrap_err();
    assert!(error.to_string().contains("year"), "{error}");
}
