//! Decision 0006's 2026-10-11 addendum (#434), driven through the engine:
//! an attempt whose result was not read after `start` parks for the
//! operator with attempts left, rather than retrying, so an effect its
//! executor committed is never committed twice. Every seat here may make
//! two attempts, so a retry, were one allowed, would run.

use std::path::Path;

use serde_json::{json, Value};

/// Implement, then the review the house requires, which a run that parks
/// at implement never reaches.
fn policy() -> String {
    let rule = |id: &str, from: &str, result: &str, next: &str| json!({"id": id, "from": from, "result": result, "next": next, "reason": id});
    let rules = [
        rule("IMPL-OK", "implement", "complete", "review"),
        rule("IMPL-FAIL", "implement", "fail", "stop"),
        rule("REVIEW-CLEAN", "review", "clean", "done"),
    ];
    let phases = ["implement", "review", "done", "stop"];
    json!({"initial": "implement", "phases": phases, "terminal": ["done", "stop"], "rules": rules})
        .to_string()
}

/// What the operator reads for a driver that exited after `accepted`
/// without a result.
const LOST: &str =
    "driver exited after accepting, before a result — attempt cannot be established as complete";

/// What the operator reads for an attempt killed at its deadline.
fn killed() -> String {
    format!(
        "attempt exceeded its {}s deadline and was killed; its process tree is gone, and what \
         it did is unknown",
        crate::engine::HANG_DEADLINE_SECONDS
    )
}

/// The journal of a run, in `dir`, of one implement seat whose driver is
/// `driver`, allowed two attempts of a hang's deadline each.
fn journal(dir: &Path, driver: &[String], feature: &str) -> Vec<Value> {
    std::fs::create_dir_all(dir.join("bundle/roles")).unwrap();
    std::fs::write(dir.join("bundle/policy.json"), policy()).unwrap();
    std::fs::write(dir.join("bundle/roles/work.md"), "# work\n").unwrap();
    let timeout = crate::engine::HANG_DEADLINE_SECONDS;
    let limits = json!({"max_attempts": 2, "timeout_seconds": timeout});
    let mut implement = json!({"results": ["complete", "fail"], "role": "roles/work.md",
                               "driver": {"command": driver}, "limits": limits});
    // The run parks at implement, so review, which the house requires,
    // never runs.
    let mut review = json!({"results": ["clean"], "role": "roles/work.md",
                            "driver": {"command": driver}});
    // A driver that is not the exec driver's serves a model: past the
    // lifted fence it is hired as an office with measured facts (U9b1).
    if driver.get(1).map(String::as_str) != Some("driver") {
        let names = ["replay-office", "replay"];
        crate::fake_office::hire_limited(dir, names, json!(driver), Some(limits));
        implement = json!({"results": ["complete", "fail"], "agent": names[0]});
        review = json!({"results": ["clean"], "agent": names[0]});
    }
    let bundle = json!({"name": "replay", "policy": "policy.json",
                        "seats": {"implement": implement, "review": review}});
    std::fs::write(dir.join("bundle/bundle.json"), bundle.to_string()).unwrap();
    let db = dir.join("forge.db").to_string_lossy().into_owned();
    let (run, _) = crate::engine::run_in(dir, feature, Path::new(&db));
    let out = dir.join("journal").to_string_lossy().into_owned();
    let export = ["export", "--run", &run, "--out", &out, "--db", &db];
    crate::engine::printed_in(dir, &export);
    let ndjson = std::fs::read_to_string(Path::new(&out).join(format!("{run}.ndjson"))).unwrap();
    ndjson
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// How many events of `kind` the journal holds.
fn count(events: &[Value], kind: &str) -> usize {
    events.iter().filter(|event| event["type"] == kind).count()
}

/// The run parked on one attempt, ended indeterminate for `reason`, with
/// the driver's stderr `tail` the engine journals beside it. A failure
/// prints each event with its payload, so an attempt that failed says why.
fn parked_after_one_attempt(events: &[Value], reason: &str, tail: &str) {
    let journaled: Vec<String> = (events.iter())
        .map(|event| format!("{} {}", event["type"], event["payload"]))
        .collect();
    let journaled = journaled.join("\n");
    assert_eq!(count(events, "effect/started"), 1, "{journaled}");
    assert_eq!(count(events, "effect/failed"), 0, "{journaled}");
    assert_eq!(count(events, "run/parked"), 1, "{journaled}");
    let ended = events
        .iter()
        .find(|event| event["type"] == "effect/indeterminate")
        .unwrap_or_else(|| panic!("an indeterminate end: {journaled}"));
    let journaled = format!("{reason}; stderr tail: {tail}");
    assert_eq!(ended["payload"]["reason"], journaled, "{ended}");
}

/// `script` as an exec seat's command.
fn exec(script: &str) -> Vec<String> {
    let brokkr = env!("CARGO_BIN_EXE_brokkr");
    [brokkr, "driver", "exec", "--", "sh", "-c", script]
        .map(str::to_string)
        .to_vec()
}

/// A hung seat is killed at its deadline. Its tree is gone, but what it
/// did is unknown, so with an attempt left it parks, and the scripted
/// success its retry would have reached never runs.
#[test]
fn a_deadline_kill_with_attempts_left_parks_after_one_attempt() {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("script.json");
    let entries =
        json!([{"behavior": "hang"}, {"behavior": "succeed", "result": {"result": "complete"}}]);
    std::fs::write(
        &script,
        json!({"seats": {"implement": entries}}).to_string(),
    )
    .unwrap();
    let state = dir.path().join("state");
    let driver = [
        env!("CARGO_BIN_EXE_brokkr"),
        "fake-driver",
        "--script",
        script.to_str().unwrap(),
        "--state",
        state.to_str().unwrap(),
    ];
    let driver: Vec<String> = driver.map(str::to_string).to_vec();
    let events = journal(dir.path(), &driver, "replay");
    parked_after_one_attempt(&events, &killed(), "");
}

/// The issue's acceptance: an executor commits a marker outside the
/// worktree and its result is lost, by a deadline kill, by an exit 0 with
/// no result file, and by a prompt write that fails after the spawn. Each
/// run parks after one attempt, and the marker was committed once.
#[test]
fn a_marker_committed_then_lost_parks_and_is_never_committed_twice() {
    let killed = killed();
    // Past a pipe's 64 KiB and within one argument's 128 KiB.
    let large = "x".repeat(100 * 1024);
    let unwritten = "could not write the prompt: Broken pipe (os error 32); the command was \
                     spawned, so no result is sent\n";
    let unread = "seat wrote no result file (the result contract was not met), so no result \
                  is sent\n";
    for (case, then, feature, reason, tail) in [
        (
            "killed",
            "cat >/dev/null; sleep 300",
            "replay",
            killed.as_str(),
            "",
        ),
        ("no result file", "cat >/dev/null", "replay", LOST, unread),
        (
            "prompt unread",
            "exec 0<&-; sleep 1",
            &large,
            LOST,
            unwritten,
        ),
    ] {
        let (dir, outside) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let marker = outside.path().join("commits");
        let script = format!("printf x >> '{}'; {then}", marker.display());
        let events = journal(dir.path(), &exec(&script), feature);
        parked_after_one_attempt(&events, reason, tail);
        let commits = std::fs::read(&marker).unwrap();
        assert_eq!(commits, b"x", "{case}: committed once");
    }
}

/// No string on the wire earns a retry (ruling 3): a driver that lists
/// replay-shaped `supports` and writes a replay-shaped checkpoint, then
/// is killed at its deadline, parks like any other.
#[test]
fn a_driver_advertising_replay_on_the_wire_parks() {
    let message = |body: &str| {
        format!("printf '%s\\n' '{{\"proto\":\"forge-driver/v1\",\"msg_id\":\"m\",{body}}}'")
    };
    let capabilities = message(
        r#""type":"capabilities","driver":"replay","version":"1","supports":["cancel","replay","deduplicates","idempotent"]"#,
    );
    let ids = r#""effect_id":"'"$eid"'","attempt_id":"'"$aid"'""#;
    let script = [
        "read -r hello".to_string(),
        capabilities,
        "read -r start".into(),
        r#"eid=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')"#.into(),
        r#"aid=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')"#.into(),
        message(&format!(r#""type":"accepted",{ids},"session_ref":null"#)),
        message(&format!(
            r#""type":"checkpoint",{ids},"data":{{"step":"working","replay":"deduplicates","idempotency_key":"k"}}"#
        )),
        "sleep 300".into(),
    ]
    .join("\n");
    let dir = tempfile::tempdir().unwrap();
    let driver = ["sh".to_string(), "-c".into(), script];
    let events = journal(dir.path(), &driver, "replay");
    parked_after_one_attempt(&events, &killed(), "");
}
