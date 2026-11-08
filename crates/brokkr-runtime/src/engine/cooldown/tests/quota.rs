//! Decision 0103 rulings 1(a) and 3 at the engine edge: the typed codex
//! rate-limit refusal the adapter now reports from its thread record —
//! class, provider, route, words and the provider's own reset — is the
//! one the cool-down reads. The fake `forge-driver/v1` participant here
//! reports exactly that object; the record-to-refusal parse is pinned in
//! `brokkr-protocol`'s adapter suite, and this case pins the engine's
//! side of the seam: a `waits` entry carrying the reset as `until`, the
//! probe between the attempts, and no second attempt in seconds.

use super::*;

#[test]
fn a_codex_usage_limit_with_its_reset_waits_for_the_probe_then_retries() {
    let dir = tempfile::tempdir().unwrap();
    // The retry-after the codex adapter reports: seconds from the
    // refusal's own time to the provider's stated reset, already bounded
    // by the existing `u64` (decision 0103 ruling 3).
    let retry = 3_600;
    let command = driver(
        dir.path(),
        "work",
        &["refuse", "succeed"],
        Probed::Answer,
        Some(retry),
        "codex",
        "second",
    );
    let mut engine = started(dir.path(), single_body(command), work(2));
    // The reset is inside the route's declared maximum, so the wait is
    // taken rather than parked: the held-wait past the maximum is
    // decision 0106's, and this unit does not build it.
    engine.cooldown.host = Some(host(dir.path(), 1, 7_200));
    let (mut engine, now) = with_clock(engine);
    let refused_at = *now.lock().unwrap();
    engine.drive().unwrap();

    // The probe ran BETWEEN the refused attempt and the retry.
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    let started = starts(&engine);
    assert_eq!(started.len(), 2, "{started:?}");
    assert_eq!(started[1]["probes"][0]["provider"], "codex");
    assert_eq!(started[1]["probes"][0]["route"], "second");

    // The terminal event records the provider's own number and the
    // `until` it computes, which is the reset the run waits to.
    let failed = failed(&engine);
    let terminal = failed
        .iter()
        .find(|event| event.payload.get("waits").is_some())
        .expect("the refused attempt records its wait");
    let waits = &terminal.payload["waits"];
    assert!(waits.is_array(), "{}", terminal.payload);
    assert_eq!(waits[0]["provider"], "codex");
    assert_eq!(waits[0]["route"], "second");
    assert_eq!(waits[0]["retry_after_seconds"], retry);
    assert_eq!(waits[0]["seconds"], retry);
    assert_eq!(
        waits[0]["until"],
        (refused_at + time::Duration::seconds(retry as i64))
            .format(&Rfc3339)
            .unwrap()
    );
    // The clock crossed the reset before the retry: 0006 did not spend
    // the second attempt in seconds.
    assert!(*now.lock().unwrap() >= refused_at + time::Duration::seconds(retry as i64));
}
