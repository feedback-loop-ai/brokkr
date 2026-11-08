//! Decision 0103 ruling 1(a): a provider's quota or usage limit is the
//! typed `rate_limit` pre-session refusal, carrying the reset the provider
//! stated. Codex reads `codex_error_info` (and an HTTP 429) on the stream
//! and, when it exits before its first turn, the error in the thread
//! record the adapter already locates; dsh reads its session record's
//! `turn/end`; claude's status-only 429 is the one shape it was missing.
//! Every other error keeps `api_error`, class `Other`, with its words
//! unchanged.

use super::refusal::{codex_thread_refusal, dsh_refusal, local_time, LocalTime, Now};
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
    Now {
        epoch,
        local: LocalTime {
            year,
            month,
            day,
            hour,
            minute,
        },
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
    let message = "You've hit your usage limit. Upgrade to Pro or try again at \
                   Nov 11th, 2026 10:38 AM.";
    let error = json!({"message": message, "codex_error_info": "usage_limit_exceeded"});
    let row = task_complete(error);
    let reason = codex_thread_refusal(&record(&[row]), &refused_at()).unwrap();
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
    ] {
        let message = format!("usage limit; try again at {at}.");
        let body = record(&[task_complete(
            json!({"message": message, "codex_error_info": "usage_limit_exceeded"}),
        )]);
        assert_eq!(
            codex_thread_refusal(&body, &now)
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
            codex_thread_refusal(&body, &now_at(0, 2025, 1, 1, 0, 0))
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
        codex_thread_refusal(&body, &now_at(0, 2026, 11, 8, 10, 38))
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
        let reason = codex_thread_refusal(&body, &refused_at()).unwrap();
        assert_eq!(reason.class(), RefusalClass::RateLimit, "{text}");
        assert_eq!(reason.retry_after_seconds(), None, "{text}");
    }
}

/// Decision 0103 ruling 3: a structured `resets_at` (codex's Unix second)
/// is preferred over the text; a structured field already in the past is
/// no wait either.
#[test]
fn codex_prefers_the_structured_reset_over_its_words() {
    let now = now_at(1_700_000_000, 2023, 11, 14, 22, 13);
    let body = record(&[
        token_count(1_700_000_300),
        task_complete(json!({"message": "no stated time here",
                             "codex_error_info": "usage_limit_exceeded"})),
    ]);
    assert_eq!(
        codex_thread_refusal(&body, &now)
            .unwrap()
            .retry_after_seconds(),
        Some(300)
    );

    let past = record(&[
        token_count(1_699_999_000),
        task_complete(json!({"message": "try again at Nov 11th, 2026 10:38 AM.",
                             "codex_error_info": "usage_limit_exceeded"})),
    ]);
    assert_eq!(
        codex_thread_refusal(&past, &now)
            .unwrap()
            .retry_after_seconds(),
        None
    );
}

/// Decision 0053: after a first turn the boundary is decision 0016's, and
/// a thread record's ordinary completion is not a refusal at all.
#[test]
fn a_codex_thread_record_is_a_refusal_only_where_it_carries_an_error() {
    let typed = record(&[task_complete(
        json!({"message": "auth failed", "codex_error_info": "unauthorized"}),
    )]);
    let reason = codex_thread_refusal(&typed, &refused_at()).unwrap();
    assert_eq!(reason.class(), RefusalClass::Other);
    assert!(reason.words().starts_with("api_error"));
    assert_eq!(reason.retry_after_seconds(), None);

    let normal = record(&[json!({"timestamp": "t", "ordinal": 1,
        "type": "event_msg",
        "payload": {"type": "task_complete", "turn_id": "t",
                    "last_agent_message": "done"}})]);
    assert!(codex_thread_refusal(&normal, &refused_at()).is_none());

    // The record's own `error` event is the other shape read.
    let event = record(&[json!({"timestamp": "t", "ordinal": 1,
        "type": "event_msg",
        "payload": {"type": "error", "message": "out of quota",
                    "codex_error_info": "usage_limit_exceeded"}})]);
    assert_eq!(
        codex_thread_refusal(&event, &refused_at()).unwrap().class(),
        RefusalClass::RateLimit
    );
    for quiet in [
        json!({"ordinal": 1}),
        json!({"type": "event_msg"}),
        json!({"type": "event_msg", "payload": {}}),
        json!({"type": "turn_context", "payload": {"model": "m"}}),
    ] {
        assert!(
            codex_thread_refusal(&record(std::slice::from_ref(&quiet)), &refused_at()).is_none(),
            "{quiet}"
        );
    }
    let bare = record(&[json!({"type": "error", "message": "no session",
                               "codex_error_info": "usage_limit_exceeded"})]);
    assert_eq!(
        codex_thread_refusal(&bare, &refused_at()).unwrap().class(),
        RefusalClass::RateLimit
    );
    // A truncated row answers nothing, as the model/effort echo does.
    let truncated = format!("{}{{not json", record(&[token_count(1)]));
    assert!(codex_thread_refusal(&truncated, &refused_at()).is_none());
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

/// The 2026-11-08 shape: the CLI announced its thread, then died before
/// its first turn with no refusal on `exec --json`. The thread record's
/// `task_complete.error` is the refusal, read through the locator the
/// adapter already holds (decision 0035), with the reset it states.
#[cfg(unix)]
#[test]
fn a_codex_that_exits_before_its_first_turn_is_read_from_the_thread_record() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("codex-home");
    let mut env = EnvGuard::lock();
    let fake = executable(
        dir.path(),
        "codex",
        r#"#!/bin/sh
cat >/dev/null
mkdir -p "$CODEX_HOME/sessions/2026/11/08"
cat > "$CODEX_HOME/sessions/2026/11/08/rollout-01a0619c-928b-7ad3-8cc9-9eaa94c3aec1.jsonl" <<'JSON'
{"timestamp":"2026-11-08T00:00:00Z","ordinal":1,"type":"event_msg","payload":{"type":"task_complete","turn_id":"t","last_agent_message":null,"error":{"message":"You have hit your usage limit. try again at Jan 1st, 9999 12:00 AM.","codex_error_info":"usage_limit_exceeded"},"started_at":1,"completed_at":2,"duration_ms":1}}
JSON
printf '{"type":"thread.started","thread_id":"01a0619c-928b-7ad3-8cc9-9eaa94c3aec1"}\n'
exit 1
"#,
    );
    env.set("CODEX_HOME", &home);
    env.set("BROKKR_CODEX_BIN", &fake);

    let mut emitted: Vec<Value> = Vec::new();
    let invocation = invoke_typed(AdapterKind::Codex, dir.path(), &mut |event| {
        emitted.push(event.clone())
    })
    .unwrap();
    let refusal = invocation.refusal.expect("the record's error is typed");
    assert_eq!(refusal.class(), RefusalClass::RateLimit);
    assert!(
        refusal.retry_after_seconds().is_some(),
        "the stated reset is read: {refusal}"
    );
    assert!(refusal.words().contains("usage limit"), "{refusal}");
    // No turn ran, so no work row was folded.
    assert!(
        !emitted
            .iter()
            .any(|event| event["step"] == "turn-completed"),
        "{emitted:?}"
    );
}

/// The one branch a healthy clock never reaches: `localtime_r` refuses a
/// second whose year does not fit, and the reset is then absent rather
/// than read from a zeroed `tm`. glibc returns NULL for this value; the
/// test is Linux's because that is the behaviour the coverage host sees.
#[cfg(target_os = "linux")]
#[test]
fn a_second_whose_year_does_not_fit_has_no_local_time() {
    assert_eq!(local_time(i64::MAX), None);
}
