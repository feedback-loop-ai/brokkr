use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::*;
use crate::envelope::EventType;
use crate::envelope_builder::EnvelopeBuilder;

fn event(cause: Option<Value>) -> EventEnvelope {
    let mut payload = json!({"reason": "a park", "evidence": {}});
    if let Some(cause) = cause {
        payload["cause"] = cause;
    }
    EnvelopeBuilder::new(EventType::RunParked, payload)
        .seq(9)
        .build()
}

fn round_trip(cause: ParkCause) -> ParkCause {
    let value = serde_json::to_value(&cause).expect("a cause serializes");
    serde_json::from_value(value).expect("a cause reads back")
}

#[test]
fn each_word_round_trips_with_its_fields() {
    assert_eq!(
        round_trip(ParkCause::Ruled {
            rule_id: Some("IMPL-BLOCKED".into()),
            detail: "the rule said so".into(),
        }),
        ParkCause::Ruled {
            rule_id: Some("IMPL-BLOCKED".into()),
            detail: "the rule said so".into(),
        }
    );
    assert_eq!(
        round_trip(ParkCause::Ruled {
            rule_id: None,
            detail: "no rule matched".into(),
        }),
        ParkCause::Ruled {
            rule_id: None,
            detail: "no rule matched".into(),
        }
    );
    assert_eq!(
        round_trip(ParkCause::AttemptCap {
            failure_class: FailureClass::StartFailure,
            detail: "two attempts failed".into(),
        }),
        ParkCause::AttemptCap {
            failure_class: FailureClass::StartFailure,
            detail: "two attempts failed".into(),
        }
    );
    assert_eq!(
        round_trip(ParkCause::Infrastructure {
            retryable: true,
            detail: "the engine never spawned".into(),
        }),
        ParkCause::Infrastructure {
            retryable: true,
            detail: "the engine never spawned".into(),
        }
    );
    assert_eq!(
        serde_json::to_value(ParkCause::ProviderLimit {
            provider: "dsh".into(),
            detail: "a rate limit".into(),
            refusal: ProviderLimitRefusal::RouteUnresolved,
        })
        .unwrap(),
        json!({
            "kind": "provider-limit",
            "provider": "dsh",
            "detail": "a rate limit",
            "refusal": {"kind": "route-unresolved"}
        })
    );
}

#[test]
fn a_park_states_its_reason_and_its_cause_together() {
    let park = Park::new(
        "the engine parked",
        ParkCause::infrastructure(false, "an indeterminate end"),
    );
    assert_eq!(park.reason(), "the engine parked");
    assert_eq!(park.to_string(), "the engine parked");
    assert_eq!(
        park.cause(),
        &ParkCause::infrastructure(false, "an indeterminate end")
    );
    assert_eq!(
        park.payload(json!({"head_at_start": "a"})),
        json!({
            "reason": "the engine parked",
            "evidence": {"head_at_start": "a"},
            "cause": {"kind": "infrastructure", "retryable": false,
                      "detail": "an indeterminate end"},
        })
    );
    assert_eq!(
        Park::ruled("refused").cause(),
        &ParkCause::ruled(None, "refused")
    );
}

/// The engine's own sentence for each refusal, from its typed fields: the
/// provider's words are never an input to it.
#[test]
fn each_refusal_states_its_own_sentence_from_its_typed_fields() {
    let until = OffsetDateTime::parse("2026-11-01T00:00:05Z", &Rfc3339).unwrap();
    let sentence = |refusal: ProviderLimitRefusal| refusal.detail("claude");
    assert_eq!(
        sentence(ProviderLimitRefusal::RouteUnresolved),
        "claude's route could not be resolved, so no route-wide hold was recorded"
    );
    assert_eq!(
        sentence(ProviderLimitRefusal::HostUnreadable { route: "r".into() }),
        "claude/r: the host file's cool-down bounds could not be read"
    );
    assert_eq!(
        sentence(ProviderLimitRefusal::EpisodeUnreadable { route: "r".into() }),
        "claude/r: an earlier wait of this effect does not read"
    );
    assert_eq!(
        sentence(ProviderLimitRefusal::PastMaximum {
            route: "r".into(),
            until,
            seconds: 5,
            max_seconds: 1,
        }),
        "claude/r: a 5s wait until 2026-11-01T00:00:05Z goes past the declared 1s maximum, \
         alone or with the episode's earlier rounds"
    );
    assert_eq!(
        sentence(ProviderLimitRefusal::NoEnd {
            route: "r".into(),
            seconds: 9,
            max_seconds: 3,
        }),
        "claude/r: a 9s wait has no end the journal can spell, against the declared 3s maximum"
    );
    assert_eq!(
        ParkCause::refusal("claude", ProviderLimitRefusal::RouteUnresolved),
        ParkCause::ProviderLimit {
            provider: "claude".into(),
            detail: ProviderLimitRefusal::RouteUnresolved.detail("claude"),
            refusal: ProviderLimitRefusal::RouteUnresolved,
        }
    );
}

#[test]
fn each_failure_class_round_trips_by_its_word() {
    for (class, word) in [
        (FailureClass::ProviderLimit, "provider-limit"),
        (FailureClass::ProviderRefusal, "provider-refusal"),
        (FailureClass::StartFailure, "start-failure"),
        (FailureClass::Determinate, "determinate"),
    ] {
        let value = serde_json::to_value(ParkCause::AttemptCap {
            failure_class: class,
            detail: "cap".into(),
        })
        .unwrap();
        assert_eq!(value["failure_class"], json!(word));
        assert_eq!(value["kind"], json!("attempt-cap"));
    }
}

#[test]
fn each_provider_limit_variant_round_trips_with_exactly_its_fields() {
    let until = OffsetDateTime::parse("2026-01-02T03:04:05Z", &Rfc3339).unwrap();
    for (refusal, expected) in [
        (
            ProviderLimitRefusal::RouteUnresolved,
            json!({"kind": "route-unresolved"}),
        ),
        (
            ProviderLimitRefusal::HostUnreadable {
                route: "first".into(),
            },
            json!({"kind": "host-unreadable", "route": "first"}),
        ),
        (
            ProviderLimitRefusal::EpisodeUnreadable {
                route: "first".into(),
            },
            json!({"kind": "episode-unreadable", "route": "first"}),
        ),
        (
            ProviderLimitRefusal::PastMaximum {
                route: "first".into(),
                until,
                seconds: 7201,
                max_seconds: 7200,
            },
            json!({
                "kind": "past-maximum",
                "route": "first",
                "until": "2026-01-02T03:04:05Z",
                "seconds": 7201,
                "max_seconds": 7200
            }),
        ),
        (
            ProviderLimitRefusal::NoEnd {
                route: "first".into(),
                seconds: 7201,
                max_seconds: 7200,
            },
            json!({
                "kind": "no-end",
                "route": "first",
                "seconds": 7201,
                "max_seconds": 7200
            }),
        ),
    ] {
        let cause = ParkCause::ProviderLimit {
            provider: "claude".into(),
            detail: "refused".into(),
            refusal: refusal.clone(),
        };
        let value = serde_json::to_value(&cause).unwrap();
        assert_eq!(value["refusal"], expected, "{refusal:?}");
        assert_eq!(
            round_trip(cause),
            ParkCause::ProviderLimit {
                provider: "claude".into(),
                detail: "refused".into(),
                refusal,
            }
        );
    }
}

#[test]
fn a_variant_with_a_forbidden_field_refuses_by_name() {
    let error = ParkCause::read(&event(Some(json!({
        "kind": "provider-limit",
        "provider": "dsh",
        "detail": "refused",
        "route": "dsh/a",
        "refusal": {"kind": "route-unresolved"}
    }))))
    .unwrap_err();
    assert_eq!(
        error,
        ParkCauseError::Forbidden {
            kind: "provider-limit",
            field: "route".into()
        }
    );

    let nested = ParkCause::read(&event(Some(json!({
        "kind": "provider-limit",
        "provider": "dsh",
        "detail": "refused",
        "refusal": {"kind": "route-unresolved", "until": "2026-01-01T00:00:00Z"}
    }))))
    .unwrap_err();
    assert_eq!(
        nested,
        ParkCauseError::Forbidden {
            kind: "route-unresolved",
            field: "until".into()
        }
    );
    assert_eq!(
        nested.to_string(),
        "the park cause does not read: a route-unresolved cause forbids `until`"
    );
}

#[test]
fn a_variant_missing_a_required_field_refuses_by_name() {
    let error = ParkCause::read(&event(Some(json!({"kind": "ruled"})))).unwrap_err();
    assert_eq!(
        error,
        ParkCauseError::Missing {
            kind: "ruled",
            field: "detail"
        }
    );
    let past = json!({"kind": "provider-limit", "provider": "claude", "detail": "d",
        "refusal": {"kind": "past-maximum", "route": "r", "seconds": 5, "max_seconds": 1}});
    assert_eq!(
        ParkCause::read(&event(Some(past))).unwrap_err(),
        ParkCauseError::Missing {
            kind: "past-maximum",
            field: "until"
        }
    );
}

/// Each way a present `cause` fails to read is its own typed fault.
#[test]
fn a_malformed_cause_refuses_by_name() {
    let fault = |cause: Value| ParkCause::read(&event(Some(cause))).unwrap_err();
    assert_eq!(
        fault(json!({"kind": "not-a-cause"})),
        ParkCauseError::UnknownKind(Some("not-a-cause".into()))
    );
    assert_eq!(fault(json!({})), ParkCauseError::UnknownKind(None));
    assert_eq!(fault(json!({"kind": 3})), ParkCauseError::UnknownKind(None));
    assert_eq!(fault(json!("ruled")), ParkCauseError::NotAnObject);
    assert_eq!(
        fault(json!({"kind": "attempt-cap", "failure_class": "nope", "detail": "d"})),
        ParkCauseError::UnknownFailureClass("nope".into())
    );
    assert_eq!(
        fault(json!({"kind": "infrastructure", "retryable": "yes", "detail": "d"})),
        ParkCauseError::WrongType {
            kind: "infrastructure",
            field: "retryable",
            expected: "a boolean"
        }
    );
    let limit = |refusal: Value| {
        fault(
            json!({"kind": "provider-limit", "provider": "claude", "detail": "d",
                     "refusal": refusal}),
        )
    };
    assert_eq!(
        limit(json!({"kind": "nope"})),
        ParkCauseError::UnknownRefusal(Some("nope".into()))
    );
    assert_eq!(limit(json!({})), ParkCauseError::UnknownRefusal(None));
    assert_eq!(
        limit(
            json!({"kind": "past-maximum", "route": "r", "until": "tomorrow",
                     "seconds": 5, "max_seconds": 1})
        ),
        ParkCauseError::Until("tomorrow".into())
    );
    assert_eq!(
        fault(
            json!({"kind": "provider-limit", "provider": "claude", "detail": "d",
                     "refusal": "route-unresolved"})
        ),
        ParkCauseError::WrongType {
            kind: "provider-limit",
            field: "refusal",
            expected: "an object"
        }
    );
    assert_eq!(
        fault(json!({"kind": "provider-limit", "provider": "claude", "detail": "d"})),
        ParkCauseError::Missing {
            kind: "provider-limit",
            field: "refusal"
        }
    );
    assert_eq!(
        fault(json!({"kind": "not-a-cause"})).to_string(),
        "the park cause does not read: kind 'not-a-cause' names none of the closed words"
    );
}

/// The word a reader paints for `cause`: an absent one is `unknown`, never a
/// guessed `ruled`, and `unknown` is no writer's word. Held here with the
/// tests that read it: no readout paints the bare word, the line carries it
/// (0071 ruling 6).
fn word_of(cause: Option<&ParkCause>) -> &'static str {
    match cause {
        None => "unknown",
        Some(ParkCause::Ruled { .. }) => "ruled",
        Some(ParkCause::AttemptCap { .. }) => "attempt-cap",
        Some(ParkCause::Infrastructure { .. }) => "infrastructure",
        Some(ParkCause::ProviderLimit { .. }) => "provider-limit",
    }
}

/// Whether the dispatcher may spend an infrastructure budget on `cause`
/// (decision 0106 ruling 4): only an `infrastructure` cause whose positive
/// non-execution evidence is recorded; every other cause and an absent one
/// read false. The dispatcher of slice 0106d re-reads the journal's evidence
/// rather than this stored word, so nothing outside these tests reads it yet.
fn is_retryable(cause: Option<&ParkCause>) -> bool {
    matches!(
        cause,
        Some(ParkCause::Infrastructure {
            retryable: true,
            ..
        })
    )
}

#[test]
fn an_old_park_reads_unknown_and_is_not_retryable() {
    let cause = ParkCause::read(&event(None)).unwrap();
    assert_eq!(cause, None);
    assert_eq!(word_of(cause.as_ref()), "unknown");
    assert!(!is_retryable(cause.as_ref()));
}

#[test]
fn only_retryable_infrastructure_reads_retryable() {
    let infrastructure = ParkCause::Infrastructure {
        retryable: true,
        detail: "never spawned".into(),
    };
    assert!(is_retryable(Some(&infrastructure)));
    assert!(!is_retryable(Some(&ParkCause::Ruled {
        rule_id: None,
        detail: "refused".into(),
    })));
    assert!(!is_retryable(Some(&ParkCause::Infrastructure {
        retryable: false,
        detail: "indeterminate".into(),
    })));
}

/// Each refusal variant paints its own word: the cause line and any script
/// read the variant rather than a field it does not have.
#[test]
fn each_provider_limit_refusal_word_is_its_own() {
    let words = [
        (ProviderLimitRefusal::RouteUnresolved, "route-unresolved"),
        (
            ProviderLimitRefusal::HostUnreadable { route: "a".into() },
            "host-unreadable",
        ),
        (
            ProviderLimitRefusal::EpisodeUnreadable { route: "a".into() },
            "episode-unreadable",
        ),
        (
            ProviderLimitRefusal::PastMaximum {
                route: "a".into(),
                until: OffsetDateTime::parse("2026-01-02T03:04:05Z", &Rfc3339).unwrap(),
                seconds: 1,
                max_seconds: 1,
            },
            "past-maximum",
        ),
        (
            ProviderLimitRefusal::NoEnd {
                route: "a".into(),
                seconds: 1,
                max_seconds: 1,
            },
            "no-end",
        ),
    ];
    for (refusal, word) in words {
        assert_eq!(refusal.word(), word, "{refusal:?}");
    }
}

/// A public `ProviderRefusal` built by hand can contradict its own cause. The
/// cause constructor never panics at that value boundary: the contradiction is
/// a typed error from the record's own reader, and the park a writer states from
/// it is `ruled`, never retried, naming what the record lacks.
#[test]
fn a_refusal_record_that_contradicts_itself_is_a_typed_error_not_a_panic() {
    use super::record::{variant, InconsistentRefusal, Lacks};
    use crate::RefusalCause;
    let lacking = |cause, lacks| InconsistentRefusal { cause, lacks };
    let record = |cause, route: Option<&str>, lengths: Option<(u64, u64)>| ProviderRefusal {
        cause,
        site: None,
        provider: "claude".into(),
        route: route.map(str::to_string),
        words: "slow down".into(),
        retry_after_seconds: None,
        seconds: lengths.map(|(seconds, _)| seconds),
        max_seconds: lengths.map(|(_, max)| max),
    };
    let until = OffsetDateTime::parse("2026-01-02T03:04:05Z", &Rfc3339).unwrap();
    for (refused, expected) in [
        (
            record(RefusalCause::HostUnreadable, None, None),
            lacking("host-unreadable", Lacks::Route),
        ),
        (
            record(RefusalCause::EpisodeUnreadable, None, None),
            lacking("episode-unreadable", Lacks::Route),
        ),
    ] {
        assert_eq!(variant(&refused, Some(until)), Err(expected));
    }
    let mut no_seconds = record(RefusalCause::PastMaximum, Some("first"), Some((5, 1)));
    no_seconds.seconds = None;
    assert_eq!(
        variant(&no_seconds, Some(until)),
        Err(lacking("past-maximum", Lacks::Seconds))
    );
    let mut no_maximum = record(RefusalCause::NoEnd, Some("first"), Some((5, 1)));
    no_maximum.max_seconds = None;
    assert_eq!(
        variant(&no_maximum, None),
        Err(lacking("no-end", Lacks::Maximum))
    );
    // The writer's cause for each is the engine's own refusal.
    let cause = ParkCause::provider_limit(&no_seconds, Some(until));
    assert_eq!(
        cause,
        ParkCause::ruled(
            None,
            "a typed refusal record that contradicts itself: a past-maximum refusal records no seconds"
        )
    );
    assert!(!is_retryable(Some(&cause)));
    // A consistent record reads as the variant its cause names, a
    // past-maximum one with no wait to read as no-end.
    let past = record(RefusalCause::PastMaximum, Some("first"), Some((5, 1)));
    assert_eq!(
        variant(&past, Some(until)),
        Ok(ProviderLimitRefusal::PastMaximum {
            route: "first".into(),
            until,
            seconds: 5,
            max_seconds: 1
        })
    );
    assert_eq!(
        variant(&past, None),
        Ok(ProviderLimitRefusal::NoEnd {
            route: "first".into(),
            seconds: 5,
            max_seconds: 1
        })
    );
    assert_eq!(
        variant(&record(RefusalCause::RouteUnresolved, None, None), None),
        Ok(ProviderLimitRefusal::RouteUnresolved)
    );
}

/// The reader enforces the forms the frozen schema states, as the
/// provider-wait reader enforces its own: what the schema refuses does not
/// read, however the type could hold it, and what it admits does.
#[test]
fn the_reader_refuses_what_the_frozen_schema_refuses_and_admits_what_it_admits() {
    let provider_limit = |provider: &str, refusal: Value| {
        json!({"kind": "provider-limit", "provider": provider, "detail": "d",
               "refusal": refusal})
    };
    let past = |route: &str, seconds: u64, max: u64| {
        json!({"kind": "past-maximum", "route": route,
               "until": "2026-01-02T03:04:05Z", "seconds": seconds, "max_seconds": max})
    };
    let form = |form: FormError| ParkCauseError::Form(form);
    let refused = [
        (
            json!({"kind": "ruled", "rule_id": null, "detail": "d"}),
            ParkCauseError::WrongType {
                kind: "ruled",
                field: "rule_id",
                expected: "a string",
            },
        ),
        (
            json!({"kind": "ruled", "rule_id": "", "detail": "d"}),
            form(FormError::RuleIdLength(0)),
        ),
        (
            json!({"kind": "ruled", "rule_id": "R".repeat(65), "detail": "d"}),
            form(FormError::RuleIdLength(65)),
        ),
        (
            json!({"kind": "ruled", "detail": ""}),
            form(FormError::EmptyDetail),
        ),
        (
            json!({"kind": "attempt-cap", "failure_class": "determinate", "detail": ""}),
            form(FormError::EmptyDetail),
        ),
        (
            json!({"kind": "infrastructure", "retryable": false, "detail": ""}),
            form(FormError::EmptyDetail),
        ),
        (
            provider_limit("-claude", json!({"kind": "route-unresolved"})),
            form(FormError::Provider("-claude".into())),
        ),
        (
            provider_limit(&"p".repeat(81), json!({"kind": "route-unresolved"})),
            form(FormError::Provider("p".repeat(81))),
        ),
        (
            provider_limit(
                "claude",
                json!({"kind": "host-unreadable", "route": "claude/a"}),
            ),
            form(FormError::Route("claude/a".into())),
        ),
        (
            provider_limit("claude", json!({"kind": "episode-unreadable", "route": ""})),
            form(FormError::Route(String::new())),
        ),
        (
            provider_limit("claude", past("first", 0, 5)),
            form(FormError::Seconds("seconds")),
        ),
        (
            provider_limit("claude", past("first", 5, 0)),
            form(FormError::Seconds("max_seconds")),
        ),
        (
            provider_limit(
                "claude",
                json!({"kind": "no-end", "route": "first", "seconds": 0, "max_seconds": 5}),
            ),
            form(FormError::Seconds("seconds")),
        ),
    ];
    for (value, expected) in refused {
        let fault = ParkCause::read(&event(Some(value.clone())))
            .expect_err(&format!("the schema refuses {value}"));
        assert_eq!(fault, expected, "{value}");
    }
    let admitted = [
        json!({"kind": "ruled", "rule_id": "R".repeat(64), "detail": "d"}),
        provider_limit("claude.2", past("claude:first", 1, 1)),
        provider_limit(&"p".repeat(80), json!({"kind": "route-unresolved"})),
    ];
    for value in admitted {
        serde_json::from_value::<ParkCause>(value.clone())
            .unwrap_or_else(|error| panic!("the schema admits {value}: {error}"));
    }
}

/// A present `cause` that does not read is carried as its named refusal,
/// never as a park that stated none (decision 0106 ruling 1).
#[test]
fn a_present_cause_that_does_not_read_is_a_named_refusal_not_an_absent_one() {
    assert_eq!(CauseRead::of_event(&event(None)), None);
    let stated = json!({"kind": "infrastructure", "retryable": false, "detail": "d"});
    assert_eq!(
        CauseRead::of_event(&event(Some(stated))),
        Some(CauseRead::Stated(ParkCause::infrastructure(false, "d")))
    );
    for (malformed, words) in [
        (json!({"kind": "not-a-cause"}), "not-a-cause"),
        (
            json!({"kind": "provider-limit", "provider": "claude", "detail": "d",
                   "refusal": {"kind": "past-maximum", "route": "r",
                               "seconds": 5, "max_seconds": 1}}),
            "until",
        ),
        (json!(null), "not an object"),
    ] {
        let read = CauseRead::of_event(&event(Some(malformed.clone())));
        let Some(CauseRead::Refused(why)) = read else {
            panic!("{malformed} must be refused by name, not read as absent: {read:?}");
        };
        assert!(why.starts_with("the park cause does not read: "), "{why}");
        assert!(why.contains(words), "{why}");
    }
}

/// The one derivation: the journal's newest `run/parked`, whatever it
/// states, and nothing from an older park once a newer one states none.
#[test]
fn the_latest_park_states_the_cause_the_readouts_paint() {
    let ruled = event(Some(json!({"kind": "ruled", "detail": "first"})));
    let bad = event(Some(json!({"kind": "not-a-cause"})));
    let silent = event(None);
    assert_eq!(CauseRead::latest(&[]), None);
    assert_eq!(CauseRead::latest(&[ruled.clone(), silent.clone()]), None);
    assert!(matches!(
        CauseRead::latest(&[ruled.clone(), bad.clone()]),
        Some(CauseRead::Refused(_))
    ));
    assert_eq!(
        CauseRead::latest(&[bad, ruled]),
        Some(CauseRead::Stated(ParkCause::ruled(None, "first")))
    );
}

/// A refusal travels on the wire and in a summary row's column as
/// `{"refused": ...}`, which no cause's `kind`-tagged object can be, and
/// reads back as the same refusal; a value that is neither is refused.
#[test]
fn a_refusal_round_trips_on_the_wire_and_in_the_column() {
    let refused = CauseRead::Refused("the park cause does not read: x".into());
    let wire = serde_json::to_value(&refused).unwrap();
    assert_eq!(wire, json!({"refused": "the park cause does not read: x"}));
    assert_eq!(serde_json::from_value::<CauseRead>(wire).unwrap(), refused);
    let stated = CauseRead::Stated(ParkCause::ruled(Some("R".into()), "d"));
    let wire = serde_json::to_value(&stated).unwrap();
    assert_eq!(
        wire,
        json!({"kind": "ruled", "rule_id": "R", "detail": "d"})
    );
    assert_eq!(serde_json::from_value::<CauseRead>(wire).unwrap(), stated);
    assert!(serde_json::from_value::<CauseRead>(json!({"refused": 3})).is_err());
    assert!(serde_json::from_value::<CauseRead>(json!({"kind": "nope"})).is_err());
    assert_eq!(refused.line(), "refused (the park cause does not read: x)");
    assert_eq!(stated.line(), "ruled R: d");
}

/// Decision 0106 ruling 1 and the summary row's bound: a cause's sentence is
/// within `PARK_REASON_BYTES` where the park is built and where a journal's
/// cause is read, so no readout cuts it a second time and every readout
/// paints the same bytes; the reason beside it stays whole. The largest
/// sentence written here is 10,000 bytes of two-byte characters, which cuts
/// at a character boundary at 256 bytes. A refusal's echo of the journal's
/// own bytes is bounded as it is made.
#[test]
fn a_cause_is_bounded_where_it_is_built_and_where_it_is_read() {
    let long = "é".repeat(5_000);
    let causes = [
        ParkCause::ruled(None, long.clone()),
        ParkCause::attempt_cap(FailureClass::Determinate, long.clone()),
        ParkCause::infrastructure(false, long.clone()),
    ];
    for cause in causes {
        let park = Park::new(long.clone(), cause);
        assert_eq!(park.reason(), long);
        assert_eq!(park.cause().detail().len(), PARK_REASON_BYTES);
        let payload = park.payload(json!({}));
        assert_eq!(payload["reason"], json!(long));
        assert!(long.starts_with(payload["cause"]["detail"].as_str().unwrap()));
    }
    assert_eq!(
        Park::ruled(long.clone()).cause().detail().len(),
        PARK_REASON_BYTES
    );
    let held = json!({"kind": "attempt-cap", "failure_class": "determinate", "detail": long});
    let read = ParkCause::read(&event(Some(held))).unwrap().unwrap();
    assert_eq!(read.detail().len(), PARK_REASON_BYTES);
    let refused = CauseRead::of_event(&event(Some(json!({"kind": "x".repeat(10_000)}))));
    let Some(CauseRead::Refused(why)) = refused else {
        panic!("a cause that does not read is refused by name");
    };
    assert_eq!(why.len(), PARK_REASON_BYTES);
}

/// The cause line says an infrastructure cause's retryability in words,
/// each way: ruling 4's fact, painted where a script and an operator read it.
#[test]
fn an_infrastructure_line_says_whether_it_is_retryable() {
    assert_eq!(
        ParkCause::infrastructure(true, "never spawned").line(),
        "infrastructure (retryable): never spawned"
    );
    assert_eq!(
        ParkCause::infrastructure(false, "indeterminate").line(),
        "infrastructure (not retryable): indeterminate"
    );
}

/// A `kind` that is absent or not a string, the cause's own or its
/// refusal's, is named as such in the operator's words.
#[test]
fn an_absent_kind_is_named_as_absent_in_the_refusal_words() {
    let fault = |cause: Value| ParkCause::read(&event(Some(cause))).unwrap_err();
    let unkinded = fault(json!({"detail": "d"}));
    assert_eq!(unkinded, ParkCauseError::UnknownKind(None));
    assert_eq!(
        unkinded.to_string(),
        "the park cause does not read: kind is absent or not a string"
    );
    let refusal = fault(
        json!({"kind": "provider-limit", "provider": "claude", "detail": "d",
        "refusal": {"route": "first"}}),
    );
    assert_eq!(refusal, ParkCauseError::UnknownRefusal(None));
    assert_eq!(
        refusal.to_string(),
        "the park cause does not read: refusal kind is absent or not a string"
    );
}

/// Each typed reader refuses its field by name: a required string that is
/// not one, a `retryable` flag that is absent, and a refusal's seconds that
/// are absent or not a whole number (a word, a negative, a fraction).
#[test]
fn each_typed_field_refuses_by_name_when_absent_or_mistyped() {
    let fault = |cause: Value| ParkCause::read(&event(Some(cause))).unwrap_err();
    assert_eq!(
        fault(json!({"kind": "ruled", "detail": 5})),
        ParkCauseError::WrongType {
            kind: "ruled",
            field: "detail",
            expected: "a string"
        }
    );
    assert_eq!(
        fault(json!({"kind": "infrastructure", "detail": "d"})),
        ParkCauseError::Missing {
            kind: "infrastructure",
            field: "retryable"
        }
    );
    let no_end = |refusal: Value| {
        fault(
            json!({"kind": "provider-limit", "provider": "claude", "detail": "d",
            "refusal": refusal}),
        )
    };
    assert_eq!(
        no_end(json!({"kind": "no-end", "route": "first", "max_seconds": 1})),
        ParkCauseError::Missing {
            kind: "no-end",
            field: "seconds"
        }
    );
    for seconds in [json!("ten"), json!(-1), json!(1.5)] {
        let mistyped = no_end(json!({"kind": "no-end", "route": "first",
            "seconds": seconds, "max_seconds": 1}));
        let whole = ParkCauseError::WrongType {
            kind: "no-end",
            field: "seconds",
            expected: "a whole number",
        };
        assert_eq!(mistyped, whole, "{seconds}");
        assert_eq!(
            mistyped.to_string(),
            "the park cause does not read: `seconds` of a no-end cause is not a whole number"
        );
    }
}

/// The engine's own refusal for a record that contradicts its cause names
/// what the record lacks in its sentence: its route, or its maximum.
#[test]
fn a_contradicting_record_names_what_it_lacks_in_the_ruled_sentence() {
    use crate::RefusalCause;
    let mut refused = ProviderRefusal {
        cause: RefusalCause::HostUnreadable,
        site: None,
        provider: "claude".into(),
        route: None,
        words: "slow down".into(),
        retry_after_seconds: None,
        seconds: Some(5),
        max_seconds: None,
    };
    let contradicts = "a typed refusal record that contradicts itself";
    assert_eq!(
        ParkCause::provider_limit(&refused, None),
        ParkCause::ruled(
            None,
            format!("{contradicts}: a host-unreadable refusal records no route")
        )
    );
    refused.cause = RefusalCause::NoEnd;
    refused.route = Some("first".into());
    assert_eq!(
        ParkCause::provider_limit(&refused, None),
        ParkCause::ruled(
            None,
            format!("{contradicts}: a no-end refusal records no maximum")
        )
    );
}

/// An end RFC 3339 cannot spell (a year before 0) is named as such in the
/// engine's sentence, never a panic and never a guessed instant.
#[test]
fn an_end_rfc_3339_cannot_spell_is_named_in_the_sentence() {
    let date = time::Date::from_calendar_date(-1, time::Month::January, 1).unwrap();
    let refusal = ProviderLimitRefusal::PastMaximum {
        route: "r".into(),
        until: date.midnight().assume_utc(),
        seconds: 5,
        max_seconds: 1,
    };
    assert_eq!(
        refusal.detail("claude"),
        "claude/r: a 5s wait until an instant the journal cannot spell goes past the declared \
         1s maximum, alone or with the episode's earlier rounds"
    );
}
