//! Decision 0103 ruling 1(a): a provider's quota or usage limit is the
//! typed `rate_limit` pre-session refusal, carrying the reset the provider
//! stated. Codex reads `codex_error_info` (and an HTTP 429) on the stream
//! and, when it exits before its first turn, the error in the thread
//! record the adapter already locates; dsh reads its session record's
//! `turn/end`; claude's status-only 429 is the one shape it was missing.
//! Every other error keeps `api_error`, class `Other`, with its words
//! unchanged.

use super::refusal::{
    claude_refusal_at, codex_thread_refusal, codex_thread_refusal_since, dsh_refusal, local_time,
    LocalTime, Now,
};
use super::tests::{executable, Case};
use super::*;
use crate::env_guard::EnvGuard;
use crate::RefusalClass;

/// One thread-record line set, in the envelope codex writes.
fn record(rows: &[Value]) -> String {
    let mut body = String::new();
    for row in rows {
        body.push_str(&serde_json::to_string(row).unwrap());
        body.push('\n');
    }
    body
}

/// A thread-record error row: the payload is where codex puts every
/// field, and `task_complete` is the row the 2026-11-08 usage limit was
/// measured on.
fn task_complete(error: Value) -> Value {
    json!({"timestamp": "2026-11-08T00:00:00Z", "ordinal": 2,
           "type": "event_msg",
           "payload": {"type": "task_complete", "turn_id": "t",
                       "last_agent_message": null, "error": error,
                       "started_at": 1, "completed_at": 2, "duration_ms": 1}})
}

/// A thread-record `token_count` row carrying a structured reset.
fn token_count(resets_at: u64) -> Value {
    json!({"timestamp": "2026-11-08T00:00:00Z", "ordinal": 1,
           "type": "event_msg",
           "payload": {"type": "token_count",
                       "info": {"total_token_usage": {}, "last_token_usage": {},
                                "model_context_window": 1},
                       "rate_limits": {"limit_id": "l", "limit_name": null,
                                       "primary": {"used_percent": 100.0,
                                                   "window_minutes": 10080,
                                                   "resets_at": resets_at},
                                       "secondary": null,
                                       "credits": {"has_credits": false,
                                                   "unlimited": false,
                                                   "balance": "0"},
                                       "individual_limit": null,
                                       "spend_control_reached": null,
                                       "plan_type": "p"}}})
}

/// A fixed clock reading, so a reset's arithmetic is pinned exactly: the
/// provider's text names no zone, and a test that read the wall clock
/// would drift.
/// One invocation over the test's own harness override, reading the
/// typed refusal it reports.
fn invoke_typed(
    kind: AdapterKind,
    dir: &std::path::Path,
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    super::tests::invoke_case(
        kind,
        &[],
        "the prompt",
        &json!({"workdir": dir}),
        Case {
            session: None,
            probe: false,
        },
        &[],
        emit,
    )
}

fn now_at(epoch: i64, year: i32, month: u8, day: u8, hour: u8, minute: u8) -> Now {
    now_at_second(epoch, year, month, day, hour, minute, 0)
}

/// The same reading with a sub-minute second, for a reset only seconds
/// away. The epoch is the host's own reading of the civil fields, so the
/// expected interval does not depend on the zone the test host runs in.
fn now_at_second(
    epoch: i64,
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
) -> Now {
    let local = LocalTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
    };
    Now {
        epoch: local.epoch().unwrap_or(epoch),
    }
}

/// 2026-11-08 09:38 local, the moment the measured request was refused.
fn refused_at() -> Now {
    now_at(1_762_594_680, 2026, 11, 8, 9, 38)
}

/// Decision 0103 ruling 1(a): codex's usage limit is the provider's own
/// rate limit, marked by the machine field codex names it with, and an
/// HTTP 429 is its status. Every other error keeps `api_error`.
#[test]
fn codex_usage_limit_and_429_are_rate_limits_and_other_errors_are_not() {
    // `codex_refusal` may read the host clock for a reset; the guard keeps
    // another test's TZ write out of it.
    let _env = EnvGuard::lock();
    for (event, status) in [
        (
            json!({"type": "error", "message": "You've hit your usage limit.",
                   "codex_error_info": "usage_limit_exceeded"}),
            None,
        ),
        (
            json!({"type": "turn.failed",
                   "error": {"message": "quota", "codex_error_info": "rate_limit_exceeded"}}),
            None,
        ),
        (
            json!({"type": "error", "message": "slow down", "status": 429}),
            Some(429),
        ),
        (
            json!({"type": "turn.failed",
                   "error": {"message": "slow down", "status": 429}}),
            Some(429),
        ),
    ] {
        let reason = codex_refusal(&event).unwrap_or_else(|| panic!("{event}"));
        assert_eq!(reason.class(), RefusalClass::RateLimit, "{event}");
        assert!(
            reason.words().starts_with("rate_limit"),
            "{event}: {reason}"
        );
        assert_eq!(reason.status(), status, "{event}");
        assert!(reason.retry_after_seconds().is_none(), "{event}: {reason}");
    }
    // Prose that reads like a rate limit, with no machine mark, is not
    // one: the classifier reads fields, never the message.
    for event in [
        json!({"type": "error", "message": "rate limited, try again later"}),
        json!({"type": "turn.failed", "error": {"message": "quota exceeded"}}),
        json!({"type": "error", "message": "boom", "status": 500,
               "codex_error_info": "internal_server_error"}),
    ] {
        let reason = codex_refusal(&event).unwrap_or_else(|| panic!("{event}"));
        assert_eq!(reason.class(), RefusalClass::Other, "{event}");
        assert!(reason.words().starts_with("api_error"), "{event}");
    }
    // Only these two event types are pre-session refusals at all.
    assert!(codex_refusal(&json!({"type": "item.completed"})).is_none());
}

/// Decision 0103 ruling 3: codex's own "try again at …" text is parsed
/// strictly, and the seconds are measured from the refusal's local time.
/// A shape the pattern does not admit gives no reset, never a guess.
#[test]
fn codex_stated_reset_is_parsed_strictly_against_the_refusal_time() {
    // `now_at` reads the host's own zone through `mktime`.
    let _env = EnvGuard::lock();
    let message = "You've hit your usage limit. Upgrade to Pro or try again at \
                   Nov 11th, 2026 10:38 AM.";
    let error = json!({"message": message, "codex_error_info": "usage_limit_exceeded"});
    let row = task_complete(error);
    let reason = codex_thread_refusal(&record(&[row]), None, Some(&refused_at())).unwrap();
    assert_eq!(reason.class(), RefusalClass::RateLimit);
    // 2026-11-08 09:38 -> 2026-11-11 10:38 is three days and one hour.
    assert_eq!(reason.retry_after_seconds(), Some(262_800));

    // Twelve AM is midnight, twelve PM is noon, and a PM hour is the
    // afternoon; each is measured from a clock the same day.
    for (now, at, expected) in [
        (
            now_at(0, 2026, 11, 7, 23, 0),
            "Nov 8th, 2026 12:00 AM",
            3_600,
        ),
        (
            now_at(0, 2026, 11, 8, 0, 0),
            "Nov 8th, 2026 12:00 PM",
            43_200,
        ),
        (
            now_at(0, 2026, 11, 8, 9, 38),
            "Nov 8th, 2026 9:38 PM",
            43_200,
        ),
        // A refusal at 09:38:45 and a reset at 09:39 is fifteen seconds,
        // not the 60 a minute-resolution reading gives.
        (
            now_at_second(0, 2026, 11, 8, 9, 38, 45),
            "Nov 8th, 2026 9:39 AM",
            15,
        ),
    ] {
        let message = format!("usage limit; try again at {at}.");
        let body = record(&[task_complete(
            json!({"message": message, "codex_error_info": "usage_limit_exceeded"}),
        )]);
        assert_eq!(
            codex_thread_refusal(&body, None, Some(&now))
                .unwrap()
                .retry_after_seconds(),
            Some(expected),
            "{at}"
        );
    }

    // Every month name the pattern admits, and a stated time without the
    // trailing period.
    for month in [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ] {
        let body = record(&[task_complete(json!({
            "message": format!("try again at {month} 1st, 2026 12:00 PM"),
            "codex_error_info": "usage_limit_exceeded"}))]);
        assert!(
            codex_thread_refusal(&body, None, Some(&now_at(0, 2025, 1, 1, 0, 0)))
                .unwrap()
                .retry_after_seconds()
                .is_some(),
            "{month}"
        );
    }
    // A reset exactly at the refusal's own time is no wait at all.
    let body = record(&[task_complete(json!({
        "message": "try again at Nov 8th, 2026 10:38 AM.",
        "codex_error_info": "usage_limit_exceeded"}))]);
    assert_eq!(
        codex_thread_refusal(&body, None, Some(&now_at(0, 2026, 11, 8, 10, 38)))
            .unwrap()
            .retry_after_seconds(),
        None
    );

    // Every shape the strict pattern does not admit: no ordinal, no
    // zone-less clock, a bad month, an hour outside 12, a minute
    // outside 59, an unknown meridiem, trailing words, and no phrase.
    for text in [
        "try again at Nov 11, 2026 10:38 AM.",
        "try again at Nov 11th, 2026 10:38",
        "try again at Nope 11th, 2026 10:38 AM.",
        "try again at Nov 11th, 2026 13:38 PM.",
        "try again at Nov 11th, 2026 10:61 AM.",
        "try again at Nov 11th, 2026 10:38 XM.",
        "try again at Nov 11th, 2026 10:38 AM. and tomorrow",
        "try again at Nov 0th, 2026 10:38 AM.",
        "try again at Nov 32nd, 2026 10:38 AM.",
        "try again at Nov 11th, 26 10:38 AM.",
        "try again at Nov 11th, 20x6 10:38 AM.",
        "try again at Nov 11th, 2026 :38 AM.",
        "try again at Nov 11th, 2026 100:38 AM.",
        "try again at Nov 11th, 2026 10:3 AM.",
        "try again later",
    ] {
        let body = record(&[task_complete(
            json!({"message": text, "codex_error_info": "usage_limit_exceeded"}),
        )]);
        let reason = codex_thread_refusal(&body, None, Some(&refused_at())).unwrap();
        assert_eq!(reason.class(), RefusalClass::RateLimit, "{text}");
        assert_eq!(reason.retry_after_seconds(), None, "{text}");
    }
}

/// Decision 0103 ruling 3: a structured `resets_at` (codex's Unix second)
/// is preferred over the text; a structured field already in the past is
/// no wait either.
#[test]
fn codex_prefers_the_structured_reset_over_its_words() {
    // `now_at` reads the host's own zone through `mktime`.
    let _env = EnvGuard::lock();
    let now = now_at(1_700_000_000, 2023, 11, 14, 22, 13);
    let body = record(&[
        token_count(now.epoch.unsigned_abs() + 300),
        task_complete(json!({"message": "no stated time here",
                             "codex_error_info": "usage_limit_exceeded"})),
    ]);
    assert_eq!(
        codex_thread_refusal(&body, None, Some(&now))
            .unwrap()
            .retry_after_seconds(),
        Some(300)
    );

    // A stale exhausted-window reset must not suppress the provider's
    // stated future reset: the words are still the provider's own.
    let past = record(&[
        token_count(now.epoch.unsigned_abs() - 1_000),
        task_complete(json!({"message": "try again at Nov 15th, 2023 10:38 AM.",
                             "codex_error_info": "usage_limit_exceeded"})),
    ]);
    assert_eq!(
        codex_thread_refusal(&past, None, Some(&now))
            .unwrap()
            .retry_after_seconds(),
        Some(44_700)
    );
}

/// Decision 0053: after a first turn the boundary is decision 0016's, and
/// a thread record's ordinary completion is not a refusal at all.
#[test]
fn a_codex_thread_record_is_a_refusal_only_where_it_carries_an_error() {
    // `refused_at` reads the host's own zone through `mktime`.
    let _env = EnvGuard::lock();
    let typed = record(&[task_complete(
        json!({"message": "auth failed", "codex_error_info": "unauthorized"}),
    )]);
    let reason = codex_thread_refusal(&typed, None, Some(&refused_at())).unwrap();
    assert_eq!(reason.class(), RefusalClass::Other);
    assert!(reason.words().starts_with("api_error"));
    assert_eq!(reason.retry_after_seconds(), None);

    let normal = record(&[json!({"timestamp": "t", "ordinal": 1,
        "type": "event_msg",
        "payload": {"type": "task_complete", "turn_id": "t",
                    "last_agent_message": "done"}})]);
    assert!(codex_thread_refusal(&normal, None, Some(&refused_at())).is_none());

    // The record's own `error` event is the other shape read.
    let event = record(&[json!({"timestamp": "t", "ordinal": 1,
        "type": "event_msg",
        "payload": {"type": "error", "message": "out of quota",
                    "codex_error_info": "usage_limit_exceeded"}})]);
    assert_eq!(
        codex_thread_refusal(&event, None, Some(&refused_at()))
            .unwrap()
            .class(),
        RefusalClass::RateLimit
    );
    for quiet in [
        json!({"ordinal": 1}),
        json!({"type": "event_msg"}),
        json!({"type": "event_msg", "payload": {}}),
        json!({"type": "turn_context", "payload": {"model": "m"}}),
    ] {
        assert!(
            codex_thread_refusal(
                &record(std::slice::from_ref(&quiet)),
                None,
                Some(&refused_at())
            )
            .is_none(),
            "{quiet}"
        );
    }
    let bare = record(&[json!({"type": "error", "message": "no session",
                               "codex_error_info": "usage_limit_exceeded"})]);
    assert_eq!(
        codex_thread_refusal(&bare, None, Some(&refused_at()))
            .unwrap()
            .class(),
        RefusalClass::RateLimit
    );
    // A clock that could not be read leaves the refusal typed and
    // simply carries no reset (decision 0103 ruling 1(a)).
    let unreadable = record(&[task_complete(json!({
        "message": "quota", "codex_error_info": "usage_limit_exceeded"}))]);
    let reason = codex_thread_refusal(&unreadable, None, None).unwrap();
    assert_eq!(reason.class(), RefusalClass::RateLimit);
    assert_eq!(reason.retry_after_seconds(), None);

    // A truncated row answers nothing, as the model/effort echo does.
    let truncated = format!("{}{{not json", record(&[token_count(1)]));
    assert!(codex_thread_refusal(&truncated, None, Some(&refused_at())).is_none());
}

/// Decision 0103 ruling 1(a): dsh's `turn/end` error carries the code and
/// status; a quota is the provider's rate limit, and every other error
/// before the first turn is `Other`.
#[test]
fn dsh_quota_is_a_rate_limit_and_other_errors_stay_other() {
    let quota = json!({"type": "turn/end", "seq": 2, "data": {"turn": 1,
        "reason": {"kind": "error", "error": {
            "message": "Insufficient Balance (request_id: r)",
            "code": "QUOTA", "status": 402}}}});
    let reason = dsh_refusal(&quota).unwrap();
    assert_eq!(reason.class(), RefusalClass::RateLimit);
    assert_eq!(reason.status(), Some(402));
    assert!(reason.words().starts_with("rate_limit (HTTP 402)"));
    assert!(reason.words().contains("Insufficient Balance"));
    assert_eq!(reason.retry_after_seconds(), None);

    let other = json!({"type": "turn/end", "seq": 2, "data": {"turn": 1,
        "reason": {"kind": "error", "error": {
            "message": "internal failure", "code": "INTERNAL", "status": 500}}}});
    let reason = dsh_refusal(&other).unwrap();
    assert_eq!(reason.class(), RefusalClass::Other);
    assert!(reason.words().starts_with("INTERNAL (HTTP 500)"));

    // A quota code, a 402 and a 429 each mark the provider's rate limit
    // on their own; a code with neither status keeps `Other`.
    for error in [
        json!({"message": "quota", "code": "QUOTA"}),
        json!({"message": "payment", "code": "OTHER", "status": 402}),
        json!({"message": "slow", "code": "OTHER", "status": 429}),
    ] {
        let event = json!({"type": "turn/end", "data": {"turn": 1,
            "reason": {"kind": "error", "error": error}}});
        assert_eq!(
            dsh_refusal(&event).unwrap().class(),
            RefusalClass::RateLimit,
            "{event}"
        );
    }
    let bare = json!({"type": "turn/end", "data": {"turn": 1,
        "reason": {"kind": "error", "error": {"message": "boom"}}}});
    let reason = dsh_refusal(&bare).unwrap();
    assert_eq!(reason.class(), RefusalClass::Other);
    assert!(reason.words().starts_with("api_error"));

    // A completed turn is not an error, a reason without a kind is not
    // read, and a row with no reason is not read at all.
    for event in [
        json!({"type": "turn/end", "data": {"turn": 1,
            "reason": {"kind": "completed"}}}),
        json!({"type": "turn/end", "data": {"turn": 1, "reason": {}}}),
        json!({"type": "turn/end", "data": {"turn": 1}}),
    ] {
        assert!(dsh_refusal(&event).is_none(), "{event}");
    }
}

/// Decision 0103 ruling 1(a): claude already types its `rate_limit` token;
/// the missing mark is a record whose only rate-limit field is the HTTP
/// status. The words speak the class, and an ordinary error keeps its own.
#[test]
fn claude_status_only_429_is_a_rate_limit() {
    let reason = claude_refusal(&json!({
        "type": "assistant", "isApiErrorMessage": true, "apiErrorStatus": 429,
        "message": {"content": [{"type": "text", "text": "slow down"}]}
    }))
    .unwrap();
    assert_eq!(reason.class(), RefusalClass::RateLimit);
    assert_eq!(reason.status(), Some(429));
    assert!(reason.words().starts_with("rate_limit (HTTP 429)"));

    let reason = claude_refusal(&json!({
        "type": "result", "is_error": true, "apiErrorStatus": 500,
        "error": "authentication_error", "result": "bad key"
    }))
    .unwrap();
    assert_eq!(reason.class(), RefusalClass::Other);
    assert!(reason
        .words()
        .starts_with("authentication_error (HTTP 500)"));

    // `is_error`/`isApiErrorMessage` and the harness's own `error` token
    // are ALTERNATIVE marks, never a conjunction: a record carrying only
    // one of them is still a refusal. The conjunction mutant survives a
    // record that carries both, so each side is pinned alone.
    for event in [
        json!({"type": "result", "is_error": true, "apiErrorStatus": 429,
               "result": "slow down"}),
        json!({"type": "result", "error": "rate_limit", "result": "slow down"}),
        json!({"type": "assistant", "error": "rate_limit",
               "message": {"content": [{"type": "text", "text": "slow down"}]}}),
    ] {
        let reason = claude_refusal(&event).unwrap_or_else(|| panic!("{event}"));
        assert_eq!(reason.class(), RefusalClass::RateLimit, "{event}");
    }
}

/// A dsh seat whose session record ends in the measured quota refusal,
/// folded by the production adapter: the invocation carries the typed
/// refusal, no `accepted` and no checkpoint (decision 0053).
#[cfg(unix)]
#[test]
fn a_dsh_quota_record_reaches_the_invocation_as_a_typed_refusal() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let fake = executable(
        dir.path(),
        "dsh",
        r#"#!/bin/sh
# This seat's argv is the cold launch: <bin> --profile headless --patch
# <overlay> <prompt>, so the overlay is the fourth argument.
root=$(sed -n "s/^    root: '\(.*\)'/\1/p" "$4")
d="$root/--project--/session-fake"
mkdir -p "$d"
f="$d/session.v3.jsonl"
printf '%s\n' '{"type":"session","version":0,"id":"session-fake","cwd":"/w"}' > "$f"
printf '%s\n' '{"type":"turn/end","seq":1,"data":{"turn":1,"reason":{"kind":"error","error":{"message":"Insufficient Balance (request_id: r)","code":"QUOTA","status":402}}}}' >> "$f"
exit 1
"#,
    );
    env.set("BROKKR_DSH_BIN", &fake);
    env.set("DSH_HOME", dir.path());
    let mut emitted: Vec<Value> = Vec::new();
    let invocation = invoke_typed(AdapterKind::Dsh, dir.path(), &mut |event| {
        emitted.push(event.clone())
    })
    .unwrap();
    let refusal = invocation.refusal.expect("the quota record is typed");
    assert_eq!(refusal.class(), RefusalClass::RateLimit);
    assert_eq!(refusal.status(), Some(402));
    assert!(refusal.words().contains("Insufficient Balance"));

    // The same seat run through `run_seat` is a refusal to start: no
    // accepted, no checkpoint, one failed result naming the provider.
    let mut bodies = Vec::new();
    let start = json!({"effect_id": "effect", "attempt_id": "attempt",
        "input": {"workdir": dir.path(), "result_path": dir.path().join("r.json"),
                  "allowed_results": ["complete"], "feature": "f", "phase": "work",
                  "role_path": "/dev/null"}});
    run_seat(AdapterKind::Dsh, &[], &start, None, &mut |body| {
        bodies.push(body)
    });
    assert!(
        !bodies
            .iter()
            .any(|body| matches!(body, Body::Accepted { .. })),
        "a refusal must not accept: {bodies:?}"
    );
    let Body::Result {
        status: ResultStatus::Failed,
        error: Some(error),
        ..
    } = bodies.last().unwrap()
    else {
        panic!("expected one failed result: {bodies:?}")
    };
    assert!(error.contains("rate_limit"), "{error}");
}

/// A fake codex that writes one rollout and exits before any turn. With
/// `announce`, it also prints the `thread.started` that hands the adapter
/// its locator; without it, the exit is the zero-event case the locator
/// has to find from the file this invocation wrote.
fn codex_exit(dir: &std::path::Path, rows: &[Value], announce: bool) -> std::path::PathBuf {
    const THREAD: &str = "01a0619c-928b-7ad3-8cc9-9eaa94c3aec1";
    let announced = if announce {
        format!("printf '{{\"type\":\"thread.started\",\"thread_id\":\"{THREAD}\"}}\\n'\n")
    } else {
        String::new()
    };
    let body = format!(
        r#"#!/bin/sh
cat >/dev/null
mkdir -p "$CODEX_HOME/sessions/2026/11/08"
cat > "$CODEX_HOME/sessions/2026/11/08/rollout-{THREAD}.jsonl" <<'JSON'
{}JSON
{announced}exit 1
"#,
        record(rows)
    );
    executable(dir, "codex", &body)
}

/// Drive a fake codex over `rows` and return what the adapter reported.
/// The two early-exit tests differ only in whether the shim announces its
/// thread, so both the spawn and the assertions live here once.
fn codex_quota_exit(
    dir: &std::path::Path,
    rows: &[Value],
    announce: bool,
) -> (Invocation, Vec<Value>) {
    let home = dir.join("codex-home");
    let mut env = EnvGuard::lock();
    let fake = codex_exit(dir, rows, announce);
    env.set("CODEX_HOME", &home);
    env.set("BROKKR_CODEX_BIN", &fake);
    let mut emitted: Vec<Value> = Vec::new();
    let invocation = invoke_typed(AdapterKind::Codex, dir, &mut |event| {
        emitted.push(event.clone())
    })
    .unwrap();
    (invocation, emitted)
}

fn assert_codex_quota(invocation: &Invocation, emitted: &[Value], why: &str) {
    assert!(
        !emitted
            .iter()
            .any(|event| event["step"] == "turn-completed"),
        "no turn ran: {emitted:?}"
    );
    let refusal = invocation.refusal.as_ref().expect(why);
    assert_eq!(refusal.class(), RefusalClass::RateLimit);
    assert!(
        refusal.retry_after_seconds().is_some(),
        "the stated reset is read: {refusal}"
    );
    assert!(refusal.words().contains("usage limit"), "{refusal}");
}

/// The 2026-11-08 shape: the CLI announced its thread, then died before
/// its first turn with no refusal on `exec --json`. The thread record's
/// `task_complete.error` is the refusal, read through the locator the
/// adapter already holds (decision 0035), with the reset it states.
#[cfg(unix)]
#[test]
fn a_codex_that_exits_before_its_first_turn_is_read_from_the_thread_record() {
    let dir = tempfile::tempdir().unwrap();
    let mut row = task_complete(json!({
        "message": "You have hit your usage limit. try again at Dec 31st, 9999 11:00 PM.",
        "codex_error_info": "usage_limit_exceeded"}));
    row["timestamp"] = json!("9999-12-31T00:00:00Z");
    let (invocation, emitted) = codex_quota_exit(dir.path(), &[row], true);
    assert_codex_quota(&invocation, &emitted, "the record's error is typed");
}

/// Decision 0103 ruling 1(a): the measured early exit wrote no event at
/// all, only its rollout. The locator is empty, so the one rollout
/// written since the invocation began is read; the reset it states is
/// the provider's own. This is the zero-event case the announcement-based
/// locator cannot serve.
#[cfg(unix)]
#[test]
fn a_codex_that_exits_with_no_event_is_read_from_its_new_thread_record() {
    let dir = tempfile::tempdir().unwrap();
    let (invocation, emitted) = codex_quota_exit(
        dir.path(),
        &[
            json!({"timestamp": "9999-12-31T00:00:00Z", "type": "session_meta",
                   "payload": {"id": "01a0619c-928b-7ad3-8cc9-9eaa94c3aec1",
                               "timestamp": "9999-12-31T00:00:00Z",
                               "cwd": dir.path().to_string_lossy()}}),
            json!({"timestamp": "9999-12-31T00:00:00Z", "ordinal": 1,
                   "type": "event_msg", "payload": {"type": "task_started", "turn_id": "t"}}),
            json!({"timestamp": "9999-12-31T00:00:01Z", "ordinal": 2,
                   "type": "event_msg", "payload": {"type": "task_complete",
                       "turn_id": "t", "error": {
                           "message": "You have hit your usage limit. \
                               try again at Dec 31st, 9999 11:00 PM.",
                           "codex_error_info": "usage_limit_exceeded"}}}),
        ],
        false,
    );
    assert_codex_quota(
        &invocation,
        &emitted,
        "the zero-event exit's own rollout is typed",
    );
}

/// The zero-event fallback is correlated, not "the newest file": several
/// rollouts written in the same window are narrowed by the cwd codex
/// records, and two that both name this seat are ambiguous and answer
/// nothing (decision 0035).
#[test]
fn a_zero_event_exit_picks_only_the_rollout_recorded_for_its_cwd() {
    // `refused_at` reads the host's own zone through `mktime`.
    let _env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("sessions");
    let day = root.join("2026/11/08");
    std::fs::create_dir_all(&day).unwrap();
    let write = |name: &str, cwd: &str| {
        let body = record(&[
            json!({"type": "session_meta",
                   "payload": {"cwd": cwd, "timestamp": "2026-11-08T00:00:00Z"}}),
            task_complete(json!({"message": "quota",
                                 "codex_error_info": "usage_limit_exceeded"})),
        ]);
        std::fs::write(day.join(name), body).unwrap();
    };
    write("rollout-a.jsonl", "/one");
    write("rollout-b.jsonl", "/two");
    let reason = codex_thread_refusal_since(&root, 0, "/two", Some(&refused_at()), 6).unwrap();
    assert_eq!(reason.class(), RefusalClass::RateLimit);

    // A third file for the same cwd leaves two candidates: ambiguous.
    write("rollout-c.jsonl", "/two");
    assert!(codex_thread_refusal_since(&root, 0, "/two", Some(&refused_at()), 6).is_none());

    // A file that is not a rollout and a cwd-less rollout are read, never
    // guessed from: neither can become this seat's record.
    std::fs::write(day.join("notes.txt"), "not a rollout").unwrap();
    std::fs::write(day.join("rollout-broken.jsonl"), "not json\n").unwrap();
    assert!(codex_thread_refusal_since(&root, 0, "/three", Some(&refused_at()), 6).is_none());

    // No directory, no depth and a window that excludes every file each
    // answer nothing rather than scanning further.
    assert!(codex_thread_refusal_since(&root.join("missing"), 0, "/two", None, 6).is_none());
    assert!(codex_thread_refusal_since(&root, i64::MAX, "/two", None, 6).is_none());
    assert!(codex_thread_refusal_since(&root, 0, "/two", None, 0).is_none());
}

/// Decision 0053's boundary in the thread record: an error before the
/// last `task_started` is a past turn's, not this invocation's refusal.
/// A resumed thread that exits without appending a new error therefore
/// does not re-serve a stale usage limit.
#[test]
fn a_codex_thread_reads_only_the_errors_after_its_last_turn_start() {
    // `refused_at` reads the host's own zone through `mktime`.
    let _env = EnvGuard::lock();
    let stale = record(&[
        task_complete(json!({"message": "old usage limit",
                             "codex_error_info": "usage_limit_exceeded"})),
        json!({"type": "event_msg", "payload": {"type": "task_started"}}),
        json!({"type": "event_msg", "payload": {"type": "task_complete",
            "last_agent_message": "done"}}),
    ]);
    assert!(codex_thread_refusal(&stale, None, Some(&refused_at())).is_none());

    // An error after a later turn started is the current invocation's.
    let current = record(&[
        task_complete(json!({"message": "old", "codex_error_info": "usage_limit_exceeded"})),
        json!({"type": "event_msg", "payload": {"type": "task_started"}}),
        json!({"type": "event_msg", "payload": {"type": "task_complete",
            "last_agent_message": "done"}}),
        json!({"type": "event_msg", "payload": {"type": "task_started"}}),
        task_complete(json!({"message": "quota", "codex_error_info": "usage_limit_exceeded"})),
    ]);
    let reason = codex_thread_refusal(&current, None, Some(&refused_at())).unwrap();
    assert_eq!(reason.class(), RefusalClass::RateLimit);

    // The row's own timestamp is the other boundary: a refusal written
    // before this invocation began is history even when the record holds
    // no later turn start, and one written after it is current.
    let historical = record(&[
        json!({"type": "event_msg", "payload": {"type": "task_started"}}),
        task_complete(json!({"message": "old",
                             "codex_error_info": "usage_limit_exceeded"})),
    ]);
    let since = Some(refused_at().epoch * 1_000);
    assert!(codex_thread_refusal(&historical, since, Some(&refused_at())).is_none());
    let fresh = record(&[json!({
        "timestamp": "2026-11-08T10:00:00Z", "type": "event_msg",
        "payload": {"type": "task_complete", "error": {
            "message": "quota", "codex_error_info": "usage_limit_exceeded"}}})]);
    assert_eq!(
        codex_thread_refusal(&fresh, since, Some(&refused_at()))
            .unwrap()
            .class(),
        RefusalClass::RateLimit
    );
}

/// Decision 0053's boundary with H1 (the chief's high finding): an error
/// followed by a successful `task_complete`, a `last_agent_message`, or an
/// item or tool row is a notice, not a pre-session refusal. The reader
/// answers `None` so the seat's lost-or-indeterminate path stands instead
/// of a cool-down retrying work already performed (decisions 0006 and
/// 0103 ruling 1(a)).
#[test]
fn a_successful_completion_clears_the_recovered_error() {
    // `refused_at` reads the host's own zone through `mktime`.
    let _env = EnvGuard::lock();
    let started = || {
        json!({"timestamp": "2026-11-08T10:00:00Z", "ordinal": 1,
               "type": "event_msg",
               "payload": {"type": "task_started", "turn_id": "t"}})
    };
    let error = || {
        json!({"timestamp": "2026-11-08T10:00:01Z", "ordinal": 2,
               "type": "event_msg",
               "payload": {"type": "error", "message": "transient",
                           "codex_error_info": "usage_limit_exceeded"}})
    };
    let refusal = |rows: &[Value]| codex_thread_refusal(&record(rows), None, Some(&refused_at()));

    // The transient error alone, before any work, is the invocation's
    // refusal: this is the control the completion below clears.
    let reason = refusal(&[started(), error()]).expect("the transient error is a refusal");
    assert_eq!(reason.class(), RefusalClass::RateLimit);

    // A successful completion, with or without a last message, is work
    // after the error. So is a completion carrying both an error and a
    // last message, and one with an explicit null error.
    for completed in [
        json!({"type": "event_msg", "payload": {"type": "task_complete",
            "turn_id": "t", "last_agent_message": "done"}}),
        json!({"type": "event_msg", "payload": {"type": "task_complete",
            "turn_id": "t"}}),
        json!({"type": "event_msg", "payload": {"type": "task_complete",
            "turn_id": "t", "last_agent_message": "done",
            "error": {"message": "late", "codex_error_info": "usage_limit_exceeded"}}}),
        json!({"type": "event_msg", "payload": {"type": "task_complete",
            "turn_id": "t", "error": null}}),
    ] {
        assert!(
            refusal(&[started(), error(), completed.clone()]).is_none(),
            "a completion after the error is work: {completed}"
        );
    }

    // Item and tool activity after the error is work on the same terms,
    // in the rollout spelling and the stream spelling.
    for work in [
        json!({"type": "response_item",
               "payload": {"type": "message", "role": "assistant"}}),
        json!({"type": "item.started", "item": {}}),
        json!({"type": "item.completed", "item": {}}),
        json!({"type": "event_msg", "payload": {"type": "item_started", "item": {}}}),
        json!({"type": "event_msg", "payload": {"type": "item_completed", "item": {}}}),
        json!({"type": "event_msg", "payload": {"type": "user_message", "message": "hi"}}),
        json!({"type": "event_msg", "payload": {"type": "agent_message", "message": "done"}}),
        json!({"type": "event_msg", "payload": {"type": "agent_reasoning", "text": "hmm"}}),
        json!({"type": "event_msg", "payload": {"type": "exec_command_begin",
               "command": ["ls"]}}),
        json!({"type": "event_msg", "payload": {"type": "exec_command_end"}}),
        json!({"type": "event_msg", "payload": {"type": "mcp_tool_call_begin"}}),
        json!({"type": "event_msg", "payload": {"type": "mcp_tool_call_end"}}),
    ] {
        assert!(
            refusal(&[started(), error(), work.clone()]).is_none(),
            "item or tool activity after the error is work: {work}"
        );
    }

    // Work followed by a later error is a mid-session failure too: the
    // error cannot be a refusal to start a turn that already ran.
    assert!(refusal(&[
        started(),
        json!({"type": "response_item",
               "payload": {"type": "message", "role": "assistant"}}),
        error(),
    ])
    .is_none());

    // A completion that carries only the error is still the error row,
    // and quiet rows (a session header, a turn context, token usage) are
    // not work that clears it.
    let only_error = json!({"type": "event_msg", "payload": {"type": "task_complete",
        "turn_id": "t", "last_agent_message": null,
        "error": {"message": "transient", "codex_error_info": "usage_limit_exceeded"}}});
    assert!(refusal(&[started(), only_error]).is_some());
    for quiet in [
        json!({"type": "session_meta", "payload": {}}),
        json!({"type": "turn_context", "payload": {}}),
        json!({"type": "event_msg", "payload": {"type": "token_count"}}),
    ] {
        assert!(
            refusal(&[started(), error(), quiet.clone()]).is_some(),
            "a quiet row is not work: {quiet}"
        );
    }
}

/// Decision 0053's boundary at full precision (the chief's same-second
/// finding): a previous invocation's refusal written at `…:00.100Z` is
/// history once this invocation starts at `…:00.900Z`, even though both
/// fall in the same whole second. The boundary is inclusive at the exact
/// millisecond of the start.
#[test]
fn a_previous_refusal_in_the_start_second_is_history() {
    let since = brokkr_core::stamp::parse_millis("2026-11-08T10:00:00.900Z").unwrap();
    let at = |timestamp: &str| {
        record(&[json!({"timestamp": timestamp, "type": "event_msg",
            "payload": {"type": "task_complete", "turn_id": "t",
                        "last_agent_message": null,
                        "error": {"message": "quota",
                                  "codex_error_info": "usage_limit_exceeded"}}})])
    };
    assert!(
        codex_thread_refusal(&at("2026-11-08T10:00:00.100Z"), Some(since), None).is_none(),
        "a completion 800ms before the start is history"
    );
    assert!(
        codex_thread_refusal(&at("2026-11-08T10:00:00.900Z"), Some(since), None).is_some(),
        "a completion exactly at the start is current"
    );
    assert!(
        codex_thread_refusal(&at("2026-11-08T10:00:01.000Z"), Some(since), None).is_some(),
        "a completion after the start is current"
    );
}

/// A temp root with one dated sessions day, kept alive by the returned
/// `TempDir`, for the zero-event locator tests.
fn sessions(day: &str) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("sessions");
    let day = root.join(day);
    std::fs::create_dir_all(&day).unwrap();
    (dir, root, day)
}

/// Write one synthetic rollout under `day`'s dated directory. `started` is
/// the session's own start, under `payload`, or the top-level envelope
/// timestamp when `top_level` is set; an empty string omits it.
fn rollout(day: &std::path::Path, name: &str, cwd: &str, started: &str, top_level: bool) {
    let mut header = json!({"type": "session_meta", "payload": {"cwd": cwd}});
    if !started.is_empty() {
        if top_level {
            header["timestamp"] = json!(started);
        } else {
            header["payload"]["timestamp"] = json!(started);
        }
    }
    let body = record(&[
        header,
        json!({"timestamp": "2026-11-08T10:00:02.000Z", "type": "event_msg",
               "payload": {"type": "task_complete", "turn_id": "t",
                           "last_agent_message": null,
                           "error": {"message": "quota",
                                     "codex_error_info": "usage_limit_exceeded"}}}),
    ]);
    std::fs::write(day.join(name), body).unwrap();
}

/// M1: a singleton candidate is correlated to this invocation exactly as a
/// crowd is. A foreign seat's only recent rollout is not this seat's
/// refusal (the chief's foreign-record finding), and a rollout that fails
/// either correlation fact is never read.
#[test]
fn a_single_foreign_rollout_is_not_this_invocations_refusal() {
    let (_dir, root, day) = sessions("2026/11/08");
    let since = brokkr_core::stamp::parse_millis("2026-11-08T10:00:00.900Z").unwrap();
    rollout(
        &day,
        "rollout-a.jsonl",
        "/foreign-seat",
        "2026-11-08T10:00:01.000Z",
        false,
    );
    assert!(
        codex_thread_refusal_since(&root, since, "/this-seat", None, 6).is_none(),
        "a singleton foreign rollout is not this invocation's"
    );

    // Every other way a header can fail correlation: not JSON, readable
    // but empty, no cwd, no start, and a body this process may not read.
    std::fs::write(day.join("rollout-broken.jsonl"), "not json\n").unwrap();
    std::fs::write(day.join("rollout-empty.jsonl"), "").unwrap();
    rollout(
        &day,
        "rollout-no-cwd.jsonl",
        "",
        "2026-11-08T10:00:01Z",
        false,
    );
    rollout(&day, "rollout-no-start.jsonl", "/this-seat", "", false);
    let unreadable = day.join("rollout-unreadable.jsonl");
    std::fs::write(&unreadable, "not json\n").unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();
    }

    // The same singleton naming this seat's cwd and a current start is
    // read; a start exactly at the boundary counts.
    rollout(
        &day,
        "rollout-b.jsonl",
        "/this-seat",
        "2026-11-08T10:00:00.900Z",
        false,
    );
    let reason = codex_thread_refusal_since(&root, since, "/this-seat", None, 6)
        .expect("the correlated rollout is this invocation's");
    assert_eq!(reason.class(), RefusalClass::RateLimit);
}

/// M1's other half: a rollout whose session began before this invocation
/// started is history even with this seat's cwd. The session start is read
/// from the header's payload, or from the envelope when that is where the
/// record spells it.
#[test]
fn a_session_that_began_before_this_invocation_is_history() {
    let (_dir, root, day) = sessions("2026/11/08");
    let since = brokkr_core::stamp::parse_millis("2026-11-08T10:00:00.900Z").unwrap();
    rollout(
        &day,
        "rollout-old.jsonl",
        "/this-seat",
        "2026-11-08T10:00:00.100Z",
        false,
    );
    assert!(
        codex_thread_refusal_since(&root, since, "/this-seat", None, 6).is_none(),
        "a session that began 800ms before the start is history"
    );
    rollout(
        &day,
        "rollout-new.jsonl",
        "/this-seat",
        "2026-11-08T10:00:00.900Z",
        true,
    );
    let reason = codex_thread_refusal_since(&root, since, "/this-seat", None, 6)
        .expect("the current session's rollout is read");
    assert_eq!(reason.class(), RefusalClass::RateLimit);
}

/// A fake codex that records one recovered turn and exits 0 with no result:
/// it starts a turn, optionally writes a transient usage-limit error, then
/// completes the turn successfully and leaves a work marker. With the error
/// this is the chief's H1 case; without it, it is the control the earlier
/// behaviour already handled.
#[cfg(unix)]
fn codex_recovered_completion(dir: &std::path::Path, with_error: bool) -> std::path::PathBuf {
    const THREAD: &str = "01a0619c-928b-7ad3-8cc9-9eaa94c3aec1";
    let mut rows = vec![
        json!({"timestamp": "9999-12-31T00:00:00Z", "type": "session_meta",
               "payload": {"id": THREAD, "timestamp": "9999-12-31T00:00:00Z",
                           "cwd": dir.to_string_lossy()}}),
        json!({"timestamp": "9999-12-31T00:00:01Z", "ordinal": 1,
               "type": "event_msg",
               "payload": {"type": "task_started", "turn_id": "t"}}),
    ];
    if with_error {
        rows.push(json!({"timestamp": "9999-12-31T00:00:02Z", "ordinal": 2,
               "type": "event_msg",
               "payload": {"type": "error", "message": "transient",
                           "codex_error_info": "usage_limit_exceeded"}}));
    }
    rows.push(json!({"timestamp": "9999-12-31T00:00:03Z", "ordinal": 3,
               "type": "event_msg",
               "payload": {"type": "task_complete", "turn_id": "t",
                           "last_agent_message": "done"}}));
    let body = format!(
        r#"#!/bin/sh
cat >/dev/null
printf '%s' done > '{marker}'
mkdir -p "$CODEX_HOME/sessions/2026/11/08"
cat > "$CODEX_HOME/sessions/2026/11/08/rollout-{THREAD}.jsonl" <<'JSON'
{rows}JSON
exit 0
"#,
        marker = dir.join("worked").display(),
        rows = record(&rows),
    );
    executable(dir, "codex", &body)
}

/// H1 end to end (the chief's recovered-work finding): a zero-stdout fake
/// codex writes a rollout that errors and then completes, writes a work
/// marker, exits 0 and leaves no result. The error is a notice, so
/// `run_seat` sends no result at all — the lost-or-indeterminate path
/// stands — instead of a `failed` result with a `rate_limit` refusal that a
/// cool-down could retry. The control without the transient error already
/// sent no result and continues to.
#[cfg(unix)]
#[test]
fn recovered_work_without_a_result_stays_lost() {
    let mut env = EnvGuard::lock();
    for with_error in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("codex-home");
        let fake = codex_recovered_completion(dir.path(), with_error);
        env.set("CODEX_HOME", &home);
        env.set("BROKKR_CODEX_BIN", &fake);
        let start = json!({"effect_id": "effect", "attempt_id": "attempt",
            "input": {"workdir": dir.path(),
                      "result_path": dir.path().join("result.json"),
                      "allowed_results": ["complete"], "feature": "f",
                      "phase": "work", "role_path": "/dev/null"}});
        let mut bodies = Vec::new();
        run_seat(AdapterKind::Codex, &[], &start, None, &mut |body| {
            bodies.push(body)
        });
        assert!(
            dir.path().join("worked").exists(),
            "the fake wrote its work marker (with_error={with_error})"
        );
        assert!(
            !bodies
                .iter()
                .any(|body| matches!(body, Body::Result { .. })),
            "a recovered turn with no result is lost, not a refusal (with_error={with_error}): \
             {bodies:?}"
        );
        assert!(
            bodies
                .iter()
                .any(|body| matches!(body, Body::Accepted { .. })),
            "the lost attempt is still accepted (with_error={with_error}): {bodies:?}"
        );
    }
}

/// Decision 0103 ruling 3: the structured reset belongs to the window
/// that refused. A half-empty primary's reset must not shorten a full
/// secondary's wait, and two full windows take the later one.
#[test]
fn a_codex_structured_reset_belongs_to_the_exhausted_window() {
    // `now_at` reads the host's own zone through `mktime`.
    let _env = EnvGuard::lock();
    let now = now_at(1_700_000_000, 2023, 11, 14, 22, 13);
    let limits = |primary: (f64, i64), secondary: (f64, i64)| {
        json!({"timestamp": "2023-11-14T22:13:00Z", "ordinal": 1,
               "type": "event_msg",
               "payload": {"type": "token_count",
                   "rate_limits": {
                       "primary": {"used_percent": primary.0, "resets_at": primary.1},
                       "secondary": {"used_percent": secondary.0, "resets_at": secondary.1}}}})
    };
    let error = json!({"message": "no stated time",
                       "codex_error_info": "usage_limit_exceeded"});
    let body = record(&[
        limits((10.0, now.epoch + 300), (100.0, now.epoch + 3_600)),
        task_complete(error.clone()),
    ]);
    assert_eq!(
        codex_thread_refusal(&body, None, Some(&now))
            .unwrap()
            .retry_after_seconds(),
        Some(3_600)
    );
    // Two exhausted windows: the later reset cannot retry too early.
    let body = record(&[
        limits((100.0, now.epoch + 300), (100.0, now.epoch + 3_600)),
        task_complete(error),
    ]);
    assert_eq!(
        codex_thread_refusal(&body, None, Some(&now))
            .unwrap()
            .retry_after_seconds(),
        Some(3_600)
    );
}

/// Decision 0103 ruling 3: a structured reset on the successful quota
/// record is the provider's own number; without a clock the refusal is
/// still typed and simply carries no retry-after.
#[test]
fn a_claude_rate_limit_keeps_its_structured_reset() {
    let now = 1_700_000_000_i64;
    let event = json!({"type": "rate_limit_event",
        "rate_limit_info": {"status": "rejected", "resetsAt": now + 600,
            "rateLimitType": "five_hour", "utilization": 1.0,
            "isUsingOverage": false}});
    let reason = claude_refusal_at(&event, Some(now)).unwrap();
    assert_eq!(reason.class(), RefusalClass::RateLimit);
    assert_eq!(reason.retry_after_seconds(), Some(600));
    // A pass that names no readable reset, and a clock that could not be
    // read, both leave the refusal typed with no wait.
    let stale = json!({"type": "rate_limit_event",
        "rate_limit_info": {"status": "rejected", "resetsAt": now - 1}});
    assert_eq!(
        claude_refusal_at(&stale, Some(now))
            .unwrap()
            .retry_after_seconds(),
        None
    );
    assert_eq!(
        claude_refusal_at(&event, None)
            .unwrap()
            .retry_after_seconds(),
        None
    );
}

/// Decision 0103 ruling 3, measured against the host's own zone: on
/// 2026-10-25 London falls back at 02:00, so a refusal at 01:30 BST and
/// a stated 02:30 AM reset is 7200 seconds, not the 3600 a zone-blind
/// subtraction gives.
#[test]
fn a_text_reset_across_the_host_fall_back_waits_the_real_seconds() {
    let mut env = EnvGuard::lock();
    env.set("TZ", "Europe/London");
    let now = Now {
        epoch: 1_792_888_200,
    };
    let body = record(&[task_complete(json!({
        "message": "try again at Oct 25th, 2026 2:30 AM.",
        "codex_error_info": "usage_limit_exceeded"}))]);
    assert_eq!(
        codex_thread_refusal(&body, None, Some(&now))
            .unwrap()
            .retry_after_seconds(),
        Some(7_200)
    );
}

/// Decision 0103 ruling 3's round trip refuses a civil date the host's
/// zone does not have: `mktime` normalises Feb 30 to Mar 2, and the round
/// trip sees the difference rather than returning a guessed reset.
#[test]
fn an_impossible_civil_date_has_no_epoch() {
    // `LocalTime::epoch` reads the host's own zone through `mktime` and
    // `localtime_r`.
    let _env = EnvGuard::lock();
    let impossible = LocalTime {
        year: 2026,
        month: 2,
        day: 30,
        hour: 12,
        minute: 0,
        second: 0,
    };
    assert_eq!(impossible.epoch(), None);
}

/// The one branch a healthy clock never reaches: `localtime_r` refuses a
/// second whose year does not fit, and the reset is then absent rather
/// than read from a zeroed `tm`. glibc returns NULL for this value; the
/// test is Linux's because that is the behaviour the coverage host sees.
#[cfg(target_os = "linux")]
#[test]
fn a_second_whose_year_does_not_fit_has_no_local_time() {
    // `local_time` reads the host's own zone through `localtime_r`.
    let _env = EnvGuard::lock();
    assert_eq!(local_time(i64::MAX), None);
}
