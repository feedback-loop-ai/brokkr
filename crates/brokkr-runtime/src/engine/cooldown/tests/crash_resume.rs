//! A crash between a failed event and its park: the record the failed event
//! carries is the only evidence, so the resume parks on it with no wait,
//! probe or driver invocation, and states the cause that record names
//! (decision 0106 ruling 1).

use super::*;

/// Drive a crash-before-the-park journal and require the resume to park on
/// the record's facts with no wait, probe or driver invocation.
fn assert_crash_parks(
    dir: &Path,
    attempts: u64,
    host: Option<PathBuf>,
    refusal: &brokkr_protocol::PreSessionRefusal,
    earlier: Option<Value>,
    expected: &[&str],
) -> brokkr_core::ParkCause {
    let expected_starts = 1 + usize::from(earlier.is_some());
    let (mut engine, _now) =
        terminal_case(dir, attempts, host, refusal, earlier, TerminalMode::Crashed);
    engine.drive().unwrap();
    assert!(
        logs(dir, "work").is_empty(),
        "a crash-before-the-park resume invoked the driver: {:?}",
        logs(dir, "work")
    );
    assert_eq!(
        starts(&engine).len(),
        expected_starts,
        "the resume started an attempt"
    );
    let reason = park_reason(&engine);
    for fact in expected {
        assert!(reason.contains(fact), "{reason} omits {fact}");
    }
    park_cause_of(&engine)
}

/// H1: the failed event records the refusal it parks on, so a crash before
/// the separate `run/parked` does not turn a maximum refusal into a retry.
/// This is the no-end cause: the journal cannot spell the `until`.
#[test]
fn a_no_until_crash_resumes_the_maximum_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let refusal = a_rate_limit_with(Some("first"), Some(1_000_000_000_000));
    let cause = assert_crash_parks(
        dir.path(),
        2,
        Some(host(dir.path(), 1, 10)),
        &refusal,
        None,
        &[
            "has no representable end",
            "declared 10s maximum",
            "rate_limit: slow down",
        ],
    );
    let brokkr_core::ProviderLimitRefusal::NoEnd {
        route, max_seconds, ..
    } = refusal_of(cause)
    else {
        panic!("a journal that cannot spell the end parks as no-end");
    };
    assert_eq!((route.as_str(), max_seconds), ("first", 10));
}

#[test]
fn a_crash_before_a_route_unresolved_park_resumes_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let refusal = a_rate_limit_with(None, None);
    let cause = assert_crash_parks(
        dir.path(),
        2,
        None,
        &refusal,
        None,
        &["route could not be resolved", "rate_limit: slow down"],
    );
    assert_eq!(
        refusal_of(cause),
        brokkr_core::ProviderLimitRefusal::RouteUnresolved
    );
}

/// A cap reached after a crash keeps a refusal whose route never resolved:
/// the cap states the record's facts without inventing a length or `until`.
#[test]
fn a_cap_after_a_crash_keeps_a_route_unresolved_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let refusal = a_rate_limit_with(None, Some(7));
    let (mut engine, _now) =
        terminal_case(dir.path(), 1, None, &refusal, None, TerminalMode::Crashed);
    drove(&mut engine, dir.path(), &[]);
    assert_eq!(starts(&engine).len(), 1);
    let reason = park_reason(&engine);
    assert!(reason.starts_with("effect "), "{reason}");
    assert!(reason.contains("claude/an unresolved route"), "{reason}");
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
    assert!(reason.contains("retry-after 7s"), "{reason}");
    assert!(reason.contains("no until"), "{reason}");
}

#[test]
fn a_crash_before_a_host_unreadable_park_resumes_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let refusal = a_rate_limit_with(Some("first"), None);
    let cause = assert_crash_parks(
        dir.path(),
        2,
        Some(dir.path().to_path_buf()),
        &refusal,
        None,
        &[
            "claude/first",
            "host configuration could not be read",
            "rate_limit: slow down",
        ],
    );
    assert_eq!(
        refusal_of(cause),
        brokkr_core::ProviderLimitRefusal::HostUnreadable {
            route: "first".into()
        }
    );
}

#[test]
fn a_crash_before_an_episode_unreadable_park_resumes_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let refusal = a_rate_limit_with(Some("first"), None);
    let cause = assert_crash_parks(
        dir.path(),
        3,
        Some(host(dir.path(), 1, 10)),
        &refusal,
        Some(
            json!({"effect_id": "effect", "attempt_id": "a0", "error": "refused",
                    "start_failure": true, "start_failure_sites": [null],
                    "waits": malformed_earlier()}),
        ),
        &[
            "claude/first",
            "an earlier wait does not read",
            "rate_limit: slow down",
        ],
    );
    assert_eq!(
        refusal_of(cause),
        brokkr_core::ProviderLimitRefusal::EpisodeUnreadable {
            route: "first".into()
        }
    );
}

#[test]
fn a_crash_before_a_past_maximum_park_resumes_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    // An earlier 2s wait and a stated 1s reset: the episode's 3s passes the
    // 2s maximum, the engine's to bound (a stated reset alone past it is
    // held, decision 0106 ruling 2).
    let refusal = a_rate_limit_with(Some("first"), Some(1));
    let earlier = json!({"effect_id": "effect", "attempt_id": "a0", "error": "refused",
        "waits": wait_value(base() + time::Duration::seconds(2), 2)});
    let (mut engine, _now) = terminal_case(
        dir.path(),
        3,
        Some(host(dir.path(), 1, 2)),
        &refusal,
        Some(earlier),
        TerminalMode::Crashed,
    );
    engine.drive().unwrap();
    assert!(logs(dir.path(), "work").is_empty(), "a resume ran a driver");
    let reason = park_reason(&engine);
    assert!(reason.contains("would wait 1s"), "{reason}");
    assert!(
        reason.contains("episode 3s is past the declared 2s maximum"),
        "{reason}"
    );
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
    // The cause names the wait's own recorded end, read back from the
    // failed event the crash left, not a guess.
    let recorded = failed(&engine)[1].payload["waits"][0]["until"]
        .as_str()
        .unwrap()
        .to_string();
    let until = OffsetDateTime::parse(&recorded, &Rfc3339).unwrap();
    assert_eq!(
        refusal_of(park_cause_of(&engine)),
        brokkr_core::ProviderLimitRefusal::PastMaximum {
            route: "first".into(),
            until,
            seconds: 1,
            max_seconds: 2,
        }
    );
}
