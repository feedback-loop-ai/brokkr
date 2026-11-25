//! The record's own id (F4, R1): the header's id is this record's own, so
//! an announced thread is compared exactly and, without one, the id is the
//! full id the codex file name ends with. A fixed word or any fragment of
//! the real id correlates nothing, and the lost-result path stands.

use super::*;

/// F4: an empty id, an id the file name does not carry, and every fragment
/// of the real id correlate nothing. The zero-stdout fake writes its work
/// marker, exits 0 and leaves no result, so each case is driven on both
/// paths and must send no result and no refusal.
#[cfg(unix)]
#[test]
fn review_chief_1037f_record_header_id_must_be_the_records_own() {
    for (label, id) in [
        ("an empty id is no record's own", ""),
        (
            "an id the file name does not carry is no record's own",
            "not-this-thread",
        ),
        ("the fixed word before the id is no record's own", "rollout"),
        ("the leading group is no record's own", "01a0619c"),
        ("the trailing group is no record's own", "9eaa94c3aec1"),
        ("a middle fragment is no record's own", "928b-7ad3"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut header = correlated_header(dir.path());
        header["payload"]["id"] = json!(id);
        let [started, complete] = started_and_quota_complete();
        let rows = [header, started, complete];
        assert_lost_on_both_paths(dir.path(), &rows, label);
    }
}

/// F4's name reader: a name carries an id only in codex's full 8-4-4-4-12
/// lowercase `rollout-<timestamp>-<id>.jsonl` shape. A name that is not a
/// rollout `.jsonl`, whose trailing id is not that shape (a bad character
/// or a bad hyphen position), or whose trailing 36 bytes split a character
/// carries no id, even when the header spells those characters, so the
/// record correlates nothing.
#[test]
fn a_name_that_does_not_carry_a_full_codex_id_correlates_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().to_string_lossy();
    let mut bad_shape = THREAD.to_string();
    bad_shape.pop();
    bad_shape.push('z');
    let bad_hyphen = THREAD.replacen('-', "0", 1);
    let split = format!("é{}", "a".repeat(35));
    for (name, id) in [
        (format!("rollout-{bad_shape}.jsonl"), bad_shape.as_str()),
        (format!("rollout-{bad_hyphen}.jsonl"), bad_hyphen.as_str()),
        (format!("{THREAD}.jsonl"), THREAD),
        (format!("rollout-{THREAD}"), THREAD),
        (format!("rollout-{split}.jsonl"), split.as_str()),
    ] {
        let path = dir.path().join(&name);
        let header = json!({"type": "session_meta",
            "payload": {"id": id, "timestamp": "9999-12-31T00:00:00Z", "cwd": cwd}});
        std::fs::write(&path, record(&[header])).unwrap();
        assert!(
            thread_header(&path, None, &cwd, 0, &[]).is_none(),
            "a name that does not carry a full codex id carries no id: {name}"
        );
    }
}

fn bind(dir: &Path, value: &str) -> Vec<secret::BoundSecret> {
    let store = dir.join("bindings");
    secret::store_set(&store, "S", value).unwrap();
    secret::resolve_bindings(&store, &["S".into()]).unwrap()
}

#[test]
fn bound_reset_span() {
    let _env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let now = refused_at();
    let body = record(&[json!({"type":"error","status":429,
        "message":"usage limit; try again at Nov 8th, 2026 10:38 AM."})]);
    let read = |bindings: &[secret::BoundSecret]| {
        let refusal = codex_thread_refusal(&body, None, Some(&now), bindings).unwrap();
        assert_eq!(refusal.class(), RefusalClass::RateLimit);
        refusal.retry_after_seconds()
    };
    assert_eq!(read(&[]), Some(3600));
    for (value, retry) in [
        ("8th,", None),
        ("at Nov", None),
        ("usage", Some(3600)),
        ("again", Some(3600)),
    ] {
        assert_eq!(read(&bind(dir.path(), value)), retry, "{value}");
    }
}

#[test]
fn later_start_after_error_is_work() {
    let [started, _] = started_and_quota_complete();
    let error = json!({"type":"error","status":429,"message":"quota"});
    let class = |rows: &[Value]| {
        codex_thread_refusal(&record(rows), None, None, &[]).map(|refusal| refusal.class())
    };
    assert_eq!(
        class(std::slice::from_ref(&error)),
        Some(RefusalClass::RateLimit)
    );
    assert_eq!(class(&[error, started]), None);
}

#[test]
fn complete_row_present_not_has() {
    let mut complete = completion_at("2026-11-08T00:00:00Z");
    let class = |row: &Value| {
        codex_thread_refusal(&record(std::slice::from_ref(row)), None, None, &[])
            .map(|refusal| refusal.class())
    };
    assert_eq!(class(&complete), Some(RefusalClass::RateLimit));
    complete["payload"]["last_agent_message"] = json!("delivered");
    assert_eq!(class(&complete), None);
}

#[test]
fn bound_rollout_words_keep_the_correlated_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("rollout-now-{THREAD}.jsonl"));
    let [started, complete] = started_and_quota_complete();
    let rows = [
        correlated_header(dir.path()),
        json!({"type":"world_state","payload":{}}),
        json!({"type":"turn_context","payload":{}}),
        json!({"type":"response_item","payload":{"type":"message","role":"developer"}}),
        json!({"type":"response_item","payload":{"type":"message","role":"user"}}),
        json!({"type":"event_msg","payload":{"type":"item_started","item":{"type":"UserMessage"}}}),
        json!({"type":"event_msg","payload":{"type":"item_completed","item":{"type":"UserMessage"}}}),
        started,
        token_count(200),
        json!({"type":"event_msg","payload":{"type":"error","message":"earlier error"}}),
        complete,
    ];
    std::fs::write(path, record(&rows)).unwrap();
    let cwd = dir.path().to_str().unwrap();
    let now = Now { epoch: 100 };
    let read = |bindings: &[secret::BoundSecret]| {
        let refusal =
            codex_thread_refusal_since(dir.path(), 0, cwd, Some(&now), 0, bindings).unwrap();
        assert_eq!(refusal.class(), RefusalClass::RateLimit);
        assert_eq!(refusal.words(), "rate_limit: quota");
        assert_eq!(refusal.retry_after_seconds(), Some(100));
    };
    read(&[]);
    let terms = [
        "type",
        "payload",
        "timestamp",
        "session_meta",
        "world_state",
        "turn_context",
        "response_item",
        "message",
        "role",
        "developer",
        "user",
        "event_msg",
        "task_started",
        "token_count",
        "error",
        "task_complete",
        "last_agent_message",
        "item_started",
        "item_completed",
        "item",
        "UserMessage",
        "rate_limits",
        "primary",
        "secondary",
        "used_percent",
        "resets_at",
        "codex_error_info",
        "usage_limit_exceeded",
        "01a0",
    ];
    for word in super::super::tests::bound_words(&terms) {
        read(&bind(dir.path(), word));
    }
}
