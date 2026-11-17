//! Decision 0106 ruling 1's causes for the parks a recorded wait ends in:
//! a probe that ended without an answer is an indeterminate end, and a
//! resume or an episode past the maximum is the provider's limit with its
//! typed refusal. The wait's park funnel carries the cause with the reason.

use brokkr_core::{ParkCause, ProviderLimitRefusal};

use super::park_cause_of as cause_of;
use super::*;

/// A probe that spawned a process and ended with no answer may have run, so
/// a budget retry as a new effect would evade the original wait and probe:
/// the cause is `infrastructure`, never retryable (rulings 1 and 4). Before
/// this round the funnel wrote `ruled` for it, which ruling 1's table gives
/// to a policy or engine refusal, not to an indeterminate end.
#[test]
fn a_cleanly_settled_unanswered_probe_park_is_not_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let script = "read -r hello\n\
         printf '%s\\n' '{\"proto\":\"forge-driver/v1\",\"msg_id\":\"cap\",\"type\":\"capabilities\",\"driver\":\"test\",\"version\":\"1\",\"supports\":[]}'\n\
         read -r start\n\
         exit 0\n";
    let engine = journaled(
        dir.path(),
        single_body(vec!["sh".into(), "-c".into(), script.into()]),
        wait_value(base(), 1),
    );
    let (mut engine, _now) = with_clock(engine);
    engine.drive().expect("the clean probe ends");
    let reason = park_reason(&engine);
    assert!(reason.contains("ended without an answer"), "{reason}");
    let cause = cause_of(&engine);
    assert_eq!(
        cause,
        ParkCause::infrastructure(
            false,
            "the cool-down probe for claude/first ended without an answer"
        )
    );
}

#[test]
fn a_probe_that_hangs_or_errors_is_infrastructure_and_keeps_the_drivers_words_out_of_the_cause() {
    for (probed, words) in [
        (Probed::Hang, None),
        (Probed::Error, Some("probe exploded")),
    ] {
        let (_dir, engine, _now) = parked_probe(probed);
        let cause = cause_of(&engine);
        assert_eq!(
            cause,
            ParkCause::infrastructure(
                false,
                "the cool-down probe for claude/first ended without an answer"
            ),
            "{cause:?}"
        );
        if let Some(words) = words {
            assert!(park_reason(&engine).contains(words));
        }
    }
}

/// A recorded wait that a resume finds past the declared maximum: the
/// provider's limit, with its `past-maximum` refusal and the facts the
/// resume already holds. The provider's words stay in the reason.
#[test]
fn a_resume_past_the_recorded_maximum_parks_as_a_provider_limit() {
    let until = base() + time::Duration::seconds(5);
    let (dir, mut engine, _now) = resumed(&["succeed"], until, 5);
    engine.cooldown.host = Some(host(dir.path(), 1, 1));
    engine.drive().unwrap();
    let ParkCause::ProviderLimit {
        provider,
        detail,
        refusal,
    } = cause_of(&engine)
    else {
        panic!("a resume past the maximum is the provider's limit");
    };
    assert_eq!(provider, "claude");
    assert_eq!(
        refusal,
        ProviderLimitRefusal::PastMaximum {
            route: "first".into(),
            until,
            seconds: 5,
            max_seconds: 1,
        }
    );
    assert!(!detail.contains("rate_limit: slow down"), "{detail}");
    assert!(park_reason(&engine).contains("rate_limit: slow down"));
}

/// An accumulated episode past the maximum, reached through the probe's
/// next round, parks the same way: a typed refusal beside the reason.
#[test]
fn an_accumulated_episode_past_the_maximum_parks_as_a_provider_limit() {
    let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], Probed::Refuse, None, 3);
    engine.cooldown.host = Some(host(dir.path(), 30, 100));
    engine.drive().unwrap();
    assert!(park_reason(&engine).contains("episode 210s"));
    let ParkCause::ProviderLimit {
        provider,
        detail,
        refusal,
    } = cause_of(&engine)
    else {
        panic!("an episode past the maximum is the provider's limit");
    };
    assert_eq!(provider, "claude");
    let ProviderLimitRefusal::PastMaximum {
        route,
        seconds,
        max_seconds,
        ..
    } = refusal
    else {
        panic!("the episode's refusal is past-maximum: {refusal:?}");
    };
    assert_eq!((route.as_str(), seconds, max_seconds), ("first", 120, 100));
    assert!(!detail.contains("rate_limit: slow down"), "{detail}");
}

/// Ruling 1: the provider's words stay in the park's reason and the typed
/// `refused` record, and the cause does not copy them. The cause's detail is
/// the engine's own sentence, built from the typed refusal.
#[test]
fn a_provider_limit_cause_never_copies_the_providers_words() {
    const MARKER: &str = "MARKER-the-providers-own-words";
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path(), single_body(vec!["unused".into()]), work(2));
    request_effect(&mut engine);
    let refusal: brokkr_protocol::PreSessionRefusal = serde_json::from_value(json!({
        "class": "rate_limit", "provider": "claude", "route": null, "words": MARKER
    }))
    .unwrap();
    let super::super::Terminal::Park { park, .. } = engine
        .rate_limit_terminal("effect", json!({"error": "refused"}), Some(&refusal))
        .unwrap()
    else {
        panic!("an unresolved route parks");
    };
    let (reason, cause) = (park.reason(), park.cause().clone());
    assert!(reason.contains(MARKER), "{reason}");
    assert_eq!(
        cause,
        ParkCause::refusal("claude", ProviderLimitRefusal::RouteUnresolved)
    );
    let ParkCause::ProviderLimit { detail, .. } = &cause else {
        panic!("a typed refusal is the provider's limit");
    };
    assert!(!detail.contains(MARKER), "{detail}");

    // The same through a park the run wrote: a wait past the maximum.
    let (dir, mut engine, _now) = seated!(&["refuse"], Probed::Answer, Some(5), 2);
    engine.cooldown.host = Some(host(dir.path(), 1, 2));
    engine.drive().unwrap();
    assert!(park_reason(&engine).contains("rate_limit: slow down"));
    let ParkCause::ProviderLimit {
        provider,
        detail,
        refusal,
    } = cause_of(&engine)
    else {
        panic!("a wait past the maximum is the provider's limit");
    };
    assert_eq!(detail, refusal.detail(&provider));
    assert!(!detail.contains("slow down"), "{detail}");
}

/// Ruling 1: a probe's refusal carries the provider's words in its text,
/// which stay in the park's reason; the cause is the engine's own sentence,
/// built from the variant's provider and route. An error that carries no
/// words is its own reason, and `park_of` lists every variant, so a new one
/// is a compile error rather than a silent `ruled`.
#[test]
fn a_probes_refusal_parks_with_the_engines_sentence_and_the_words_in_the_reason() {
    use super::super::{park_of, CooldownError as E};
    const MARKER: &str = "MARKER-the-providers-own-words";
    let (provider, route, words) = ("claude", "first", MARKER);
    let named = || (provider.to_string(), route.to_string(), words.to_string());
    let store = || {
        Box::new(brokkr_store::StoreError::Contended {
            operation: "load",
            waited_ms: 1,
        })
    };
    let nested = || Box::new(E::TooManyWaits { seq: 3, count: 2 });
    let (a, b) = (named(), named());
    let (c, d) = (named(), named());
    let (e, f) = (named(), named());
    let (g, h) = (named(), named());
    let probes = [
        (
            E::ProbeUndeclared {
                provider: a.0,
                route: a.1,
                words: a.2,
            },
            "was refused again, with no cool-down declared",
        ),
        (
            E::ProbeBoundsUnreadable {
                provider: b.0,
                route: b.1,
                source: nested(),
                words: b.2,
            },
            "was refused again, and its bounds cannot be read",
        ),
        (
            E::ProbeJournalUnreadable {
                provider: c.0,
                route: c.1,
                source: store(),
                words: c.2,
            },
            "was refused again, and its journal cannot be read",
        ),
        (
            E::ProbeNoTerminal {
                provider: d.0,
                route: d.1,
                words: d.2,
            },
            "was refused again with no terminal event to extend",
        ),
        (
            E::ProbeRoundsUnreadable {
                provider: e.0,
                route: e.1,
                source: store(),
                words: e.2,
            },
            "was refused again, and its rounds cannot be read",
        ),
        (
            E::ProbeEpisodeUnreadable {
                provider: f.0,
                route: f.1,
                source: nested(),
                words: f.2,
            },
            "was refused again, and its episode cannot be read",
        ),
        (
            E::RoundUnbounded {
                provider: g.0,
                route: g.1,
                words: g.2,
            },
            "would write a round with no cool-down declared",
        ),
        (
            E::RoundNoTable {
                provider: h.0,
                route: h.1,
                words: h.2,
            },
            "was refused again by the provider",
        ),
    ];
    for (error, what) in probes {
        let park = park_of(&error);
        assert!(park.reason().contains(MARKER), "{error:?}");
        assert_eq!(
            park.cause(),
            &ParkCause::ruled(
                None,
                format!("the cool-down probe for {provider}/{route} {what}")
            )
        );
    }
    let plain = E::TooManyWaits { seq: 3, count: 2 };
    let park = park_of(&plain);
    assert_eq!(park.cause(), &ParkCause::ruled(None, park.reason()));
}
