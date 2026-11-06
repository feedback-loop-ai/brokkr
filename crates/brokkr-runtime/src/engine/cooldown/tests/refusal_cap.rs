//! The cap's park for a rate-limit refusal (decision 0103 ruling 7(c), this
//! fix round): the 0006 text first, with the record's facts read from the
//! failed event or, where there is none, from the wait entry beside it.

use super::*;

/// The cap's 0006 sentence opens the park, naming the attempt it reached.
fn assert_cap_opens(reason: &str, attempts: &str) {
    assert!(reason.starts_with("effect "), "{reason}");
    assert!(reason.contains(attempts), "{reason}");
}

/// The cap's refusal facts for a park with no `until`: the route, the
/// provider's words, its retry-after and the plain no-until statement.
fn assert_cap_without_until(reason: &str, attempts: &str, route: &str, retry: u64) {
    assert_cap_opens(reason, attempts);
    assert!(reason.contains(route), "{reason}");
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
    assert!(
        reason.contains(&format!("retry-after {retry}s")),
        "{reason}"
    );
    assert!(reason.contains("no until"), "{reason}");
}

/// R1: the cap reads its facts from the wait entry where the failed event
/// carries no `refused` record. The provider's retry-after and the declared
/// maximum are both named, even when the wait is below that maximum; a cap
/// with no readable declaration says that instead of inventing one.
#[test]
fn a_cap_wait_keeps_the_retry_after_and_declared_maximum() {
    for retry in [Some(7u64), None] {
        let (dir, mut engine, _now) = seated!(&["refuse"], Probed::Answer, retry, 1);
        drove(&mut engine, dir.path(), &["attempt:refuse"]);
        assert_eq!(starts(&engine).len(), 1);
        let reason = park_reason(&engine);
        assert_cap_opens(&reason, "failed 1 of 1 attempt(s)");
        assert!(reason.contains("claude/first"), "{reason}");
        assert!(reason.contains("until"), "{reason}");
        assert!(reason.contains("rate_limit: slow down"), "{reason}");
        assert!(reason.contains("declared 10s maximum"), "{reason}");
        match retry {
            Some(retry) => assert!(
                reason.contains(&format!("retry-after {retry}s")),
                "{reason}"
            ),
            None => assert!(!reason.contains("retry-after"), "{reason}"),
        }
        let failed = failed(&engine);
        assert_eq!(failed.len(), 1);
        assert!(
            failed[0].payload.get("refused").is_none(),
            "the waits-only path carries no refused record: {}",
            failed[0].payload
        );
    }
    let payload = || {
        json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused",
               "start_failure": true, "start_failure_sites": [null],
               "waits": wait_value(base() + time::Duration::seconds(5), 5)})
    };
    // No declaration: no maximum exists to name.
    let dir = tempfile::tempdir().unwrap();
    let reason = assert_payload_parks(
        dir.path(),
        1,
        payload(),
        &["claude/first", "rate_limit: slow down"],
    );
    assert!(!reason.contains("declared"), "{reason}");
    // An unreadable declaration: the cap says why it cannot name one.
    let dir = tempfile::tempdir().unwrap();
    let mut engine = resumed_payload(dir.path(), 1, payload());
    invalid_host(dir.path(), &mut engine);
    engine.drive().unwrap();
    assert!(
        logs(dir.path(), "work").is_empty(),
        "a cap invoked the driver"
    );
    let reason = park_reason(&engine);
    assert_cap_opens(&reason, "failed 1 of 1 attempt(s)");
    assert!(reason.contains("is not a valid"), "{reason}");
}

/// R2: a route that could not be resolved parks at the cap in the process
/// that wrote the failed event. The cap's 0006 text speaks first, then the
/// record's provider, route, retry-after and words, and it invents no
/// `until`.
#[test]
fn a_live_route_unresolved_cap_speaks_first_with_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let refusal = a_rate_limit_with(None, Some(7));
    let (mut engine, _now) = terminal_case(dir.path(), 1, None, &refusal, None, TerminalMode::Live);
    drove(&mut engine, dir.path(), &[]);
    assert_eq!(starts(&engine).len(), 1);
    let reason = park_reason(&engine);
    assert_cap_without_until(
        &reason,
        "failed 1 of 1 attempt(s)",
        "claude/an unresolved route",
        7,
    );
    assert_eq!(
        failed(&engine)[0].payload["refused"]["cause"],
        "route-unresolved"
    );
}

/// R2: an unreadable host file parks at the cap in the process that wrote
/// the failed event, with 0006's text first and an explicit no-`until`.
#[test]
fn a_live_host_unreadable_cap_speaks_first_with_the_refusal() {
    let (dir, mut engine, now) = seated!(&["refuse"], Probed::Answer, Some(7), 1);
    invalid_host(dir.path(), &mut engine);
    let before = *now.lock().unwrap();
    drove(&mut engine, dir.path(), &["attempt:refuse"]);
    assert_eq!(starts(&engine).len(), 1);
    assert_eq!(*now.lock().unwrap(), before, "the cap waits for nothing");
    let reason = park_reason(&engine);
    assert_cap_without_until(&reason, "failed 1 of 1 attempt(s)", "claude/first", 7);
    assert_eq!(
        failed(&engine)[0].payload["refused"]["cause"],
        "host-unreadable"
    );
}

/// R2: an earlier wait that does not read parks at the cap in the process
/// that wrote the failed event, with 0006's text first and no `until`.
#[test]
fn a_live_episode_unreadable_cap_speaks_first_with_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let refusal = a_rate_limit_with(Some("first"), Some(7));
    let earlier = json!({"effect_id": "effect", "attempt_id": "a0",
            "error": "refused", "start_failure": true,
            "start_failure_sites": [null], "waits": malformed_earlier()});
    let (mut engine, _now) = terminal_case(
        dir.path(),
        2,
        Some(host(dir.path(), 1, 10)),
        &refusal,
        Some(earlier),
        TerminalMode::Live,
    );
    drove(&mut engine, dir.path(), &[]);
    assert_eq!(starts(&engine).len(), 2);
    let reason = park_reason(&engine);
    assert_cap_without_until(&reason, "failed 2 of 2 attempt(s)", "claude/first", 7);
    assert_eq!(
        failed(&engine)[1].payload["refused"]["cause"],
        "episode-unreadable"
    );
}
