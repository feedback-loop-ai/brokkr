#![cfg(unix)]

//! Conformance for the built-in adapters (`brokkr driver <kind>`) — the
//! Rust port of the retired Python suite. Shims stand in for the agent
//! CLIs; conformance means capabilities on hello, accepted + checkpoint
//! + exactly one result per start, and the result-file contract honored.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use brokkr_store::{validate_seat_record, SeatRecordVersion};
use serde_json::{json, Value};

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

const OBEDIENT_SHIM: &str = r#"#!/bin/sh
# honor the result contract: find the result path in the prompt
# (stdin or the last .md argument) and write a typed result there.
last=""
for a in "$@"; do last="$a"; done
case "$last" in
  *.md) prompt=$(cat "$last") ;;
  *Result?contract*) prompt=$last ;;
  *) prompt=$(cat) ;;
esac
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work", "model": "seat-claim"}' > "$target"
printf '{"type":"result","session_id":"s1","num_turns":1,"total_cost_usd":0.0}\n'
printf 'session id: deadbeef1234\n'
"#;

// The claude flavor speaks stream-json: an init with the session id, two
// tool-using assistant turns (the second carrying two tool_use blocks in
// one message), a noise line the adapter must drop, and a final result
// with the session totals — while still honoring the result-file contract.
//
// Each assistant record carries the harness's own top-level `effort`
// echo, and the RESULT — and only the result — carries the thinking
// tokens: that granularity is the measurement decision 0035 ruling 4
// records, and it is why a claude turn checkpoint has no reasoning count
// to report.
const CLAUDE_STREAM_SHIM: &str = r#"#!/bin/sh
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work", "model": "seat-claim"}' > "$target"
printf '{"type":"system","subtype":"init","session_id":"stream-1"}\n'
printf '{"type":"assistant","effort":"xhigh","message":{"model":"claude-fable-5-1","usage":{"input_tokens":10,"cache_creation_input_tokens":4,"cache_read_input_tokens":3,"output_tokens":2},"content":[{"type":"text","text":"looking"},{"type":"tool_use","name":"Read","input":{"file_path":"src/lib.rs"}}]}}\n'
printf 'not json, ignorable noise\n'
printf '{"type":"assistant","effort":"high","message":{"model":"claude-fable-5-1","usage":{"input_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":10,"output_tokens":3},"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"src/main.rs"}},{"type":"tool_use","name":"Write","input":{"file_path":"src/out.rs"}}]}}\n'
printf '{"type":"result","num_turns":2,"total_cost_usd":0.125,"usage":{"input_tokens":15,"cache_creation_input_tokens":4,"cache_read_input_tokens":13,"output_tokens":5,"output_tokens_details":{"thinking_tokens":4}}}\n'
"#;

// Codex meters its reasoning per turn and reports its cache creation as
// `cache_write_input_tokens`; it puts its effort NOT on this stream but
// in the thread record decision 0032's locator names, which this shim
// writes where a real codex files one — under `$CODEX_HOME/sessions`, in
// a dated directory, named for the thread it announced.
const CODEX_JSON_SHIM: &str = r#"#!/bin/sh
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work", "model": "seat-claim"}' > "$target"
thread="$CODEX_HOME/sessions/2026/09/03"
mkdir -p "$thread"
printf '{"type":"turn_context","payload":{"turn_context":{"effort":"xhigh"}}}\n' \
  > "$thread/rollout-2026-09-03T00-00-00-codex-thread-1.jsonl"
printf '{"type":"thread.started","thread_id":"codex-thread-1"}\n'
printf '{"type":"turn.started"}\n'
printf '{"type":"item.started","item":{"type":"command_execution","command":"secret command"}}\n'
printf '{"type":"item.completed","item":{"type":"command_execution","aggregated_output":"private output"}}\n'
printf '{"type":"turn.completed","usage":{"model":"gpt-5.6-sol","input_tokens":21,"cached_input_tokens":8,"output_tokens":5,"cache_write_input_tokens":6,"reasoning_output_tokens":3}}\n'
"#;

/// A dsh that behaves like the installed one: it prints nothing the
/// driver reads on stdout and writes its session transcript under the
/// root the seat overlay pins, naming the model that served each
/// message the way the JSONL backend does (`data.message.source.model`).
const DSH_USAGE_SHIM: &str = r#"#!/bin/sh
prompt=$*
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work", "model": "seat-claim"}' > "$target"
root=
prev=
for a in "$@"; do
  if [ "$prev" = --patch ]; then root=$(awk -F"'" '/^    root: /{print $2}' "$a"); fi
  prev=$a
done
d="$root/--conformance--/session-served"
mkdir -p "$d"
f="$d/session.jsonl"
printf '{"type":"session","version":0,"id":"session-conformance-1","cwd":"/w"}\n' > "$f"
printf '{"type":"assistant/message","data":{"turn":1,"step":1,"message":{"source":{"model":"deepseek-v4-flash"}},"usage":{"inputTokens":13,"outputTokens":3}}}\n' >> "$f"
"#;

const SILENT_SHIM: &str = "#!/bin/sh\ncat > /dev/null 2>&1 || true\necho did nothing\n";

fn make_shim(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("shim");
    std::fs::write(&path, body).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    use std::os::unix::fs::PermissionsExt;
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

fn drive(kind_args: &[&str], shim: &Path, workdir: &Path) -> Vec<Value> {
    // Every harness home is test-owned. Conformance must never make a
    // driver name (or create under) the operator's real transcript home.
    let operator_home = tempfile::tempdir().unwrap();
    let codex_home = tempfile::tempdir().unwrap();
    let dsh_home = tempfile::tempdir().unwrap();
    let result_path = workdir.join("results/fx.json");
    let input = json!({
        "feature": "conformance", "phase": "intake", "seat": "intake",
        "role_path": workdir.join("missing-role.md"),
        "workdir": workdir,
        "result_path": result_path,
        "allowed_results": ["resolved"], "context": {},
    });
    let messages = [
        json!({"proto": "forge-driver/v1", "msg_id": "m1", "type": "hello",
               "engine_version": "test"}),
        json!({"proto": "forge-driver/v1", "msg_id": "m2", "type": "start",
               "effect_id": "fx", "attempt_id": "a1", "seat": "intake",
               "input": input}),
        json!({"proto": "forge-driver/v1", "msg_id": "m3", "type": "shutdown"}),
    ];
    let mut child = Command::new(brokkr_bin())
        .arg("driver")
        .args(kind_args)
        .env("BROKKR_CLAUDE_BIN", shim)
        // Pinned unconditionally: no conformance test may ever spawn a
        // real claude-lanetally on a LaneTally-equipped machine.
        .env("BROKKR_LANETALLY_BIN", shim)
        // Deliberately split: codex is pinned through the new spelling
        // and dsh through the old one, so a conformance run proves both
        // reach the same adapter for the release the old names survive
        // (decision 0019). The new dsh spelling is REMOVED rather than
        // left inherited: it outranks the old one, so an operator who
        // has it exported would otherwise send this test at a real dsh.
        .env("BROKKR_CODEX_BIN", shim)
        .env_remove("BROKKR_DSH_BIN")
        .env("FORGE_DSH_BIN", shim)
        .env("HOME", operator_home.path())
        .env("CODEX_HOME", codex_home.path())
        .env("DSH_HOME", dsh_home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for message in &messages {
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_file(&result_path);
    let parsed: Vec<Value> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    // A transcript row proves the selected harness home, not merely the
    // vocabulary. Invalid invocations can refuse before a driver exists.
    if let Some(row) = parsed
        .iter()
        .find(|message| message.pointer("/data/step").and_then(Value::as_str) == Some("transcript"))
    {
        let transcript = &row["data"]["transcript"];
        let (kind, expected_home) = match kind_args[0] {
            "claude" | "lanetally" => (
                "claude-session",
                operator_home.path().join(".claude/projects"),
            ),
            "codex" => ("codex-thread", codex_home.path().to_path_buf()),
            "dsh" => ("dsh-session", dsh_home.path().to_path_buf()),
            "exec" => ("none", PathBuf::new()),
            other => panic!("unrecognized conformance driver {other}"),
        };
        assert_eq!(transcript["kind"], kind);
        assert_eq!(transcript["home"], expected_home.to_string_lossy().as_ref());
    }
    parsed
}

fn all_adapters(shim: &Path) -> Vec<(&'static str, Vec<String>)> {
    vec![
        (
            "claude",
            vec![
                "claude".into(),
                "--".into(),
                "--model".into(),
                "claude-fable-5-1".into(),
            ],
        ),
        (
            "lanetally",
            vec![
                "lanetally".into(),
                "--".into(),
                "--model".into(),
                "claude-fable-5-1".into(),
            ],
        ),
        (
            "codex",
            vec![
                "codex".into(),
                "--".into(),
                "--model".into(),
                "gpt-5.6-sol".into(),
            ],
        ),
        (
            "dsh",
            vec![
                "dsh".into(),
                "--".into(),
                "--model".into(),
                "deepseek/deepseek-v4-flash".into(),
            ],
        ),
        (
            "exec-stdin",
            vec![
                "exec".into(),
                "--".into(),
                shim.to_string_lossy().into_owned(),
            ],
        ),
        (
            "exec-promptfile",
            vec![
                "exec".into(),
                "--".into(),
                shim.to_string_lossy().into_owned(),
                "{prompt_file}".into(),
            ],
        ),
    ]
}

/// This engine writes seat-record v2, so conformance judges what it
/// writes against v2 — the version its own runs declare (decision 0035
/// ruling 7). A driver whose records only satisfied v1 would still pass
/// v2; the assertions below are what make the new fields non-optional
/// for a BUILT-IN, which is where ruling 3's completeness lives.
fn assert_seat_records_conform(messages: &[Value], label: &str, case: &str) {
    for message in messages {
        if message["type"] == "checkpoint" {
            let record = &message["data"];
            validate_seat_record(record, 0, SeatRecordVersion::V2)
                .unwrap_or_else(|error| panic!("{label}/{case}: {error}: {record}"));
            assert!(
                record.get("model").and_then(Value::as_str).is_some(),
                "{label}/{case}: every current checkpoint carries decision 0031 model evidence: {record}"
            );
            assert!(
                record.get("effort").and_then(Value::as_str).is_some(),
                "{label}/{case}: every current checkpoint carries decision 0035 \
                 effort configuration, sentinel included: {record}"
            );
        }
        if message["type"] == "result" && message["status"] == "succeeded" {
            let record = &message["result"];
            validate_seat_record(record, 0, SeatRecordVersion::V2)
                .unwrap_or_else(|error| panic!("{label}/{case}: {error}: {record}"));
            assert!(record.get("model").and_then(Value::as_str).is_some());
            assert!(record.get("effort").and_then(Value::as_str).is_some());
            assert!(record
                .get("transcript")
                .and_then(Value::as_object)
                .is_some());
        }
    }
}

#[test]
fn conformance_across_all_builtin_adapters() {
    for case in ["obedient", "silent"] {
        let dir = tempfile::tempdir().unwrap();
        let shim = make_shim(
            dir.path(),
            if case == "obedient" {
                OBEDIENT_SHIM
            } else {
                SILENT_SHIM
            },
        );
        let claude_dir = dir.path().join("claude-stream");
        std::fs::create_dir_all(&claude_dir).unwrap();
        let claude_shim = make_shim(&claude_dir, CLAUDE_STREAM_SHIM);
        let codex_dir = dir.path().join("codex-json");
        std::fs::create_dir_all(&codex_dir).unwrap();
        let codex_shim = make_shim(&codex_dir, CODEX_JSON_SHIM);
        let dsh_dir = dir.path().join("dsh-usage");
        std::fs::create_dir_all(&dsh_dir).unwrap();
        let dsh_shim = make_shim(&dsh_dir, DSH_USAGE_SHIM);
        for (label, args) in all_adapters(&shim) {
            // The claude adapter streams its session: its obedient shim
            // speaks stream-json and yields three seat-turn checkpoints
            // (one per tool_use block, the last two sharing a turn)
            // before the session-finished one.
            let claude = label == "claude";
            // The lanetally leg reuses CLAUDE_STREAM_SHIM verbatim: the
            // reuse IS the argv-compatibility proof — the wrapper is
            // driven exactly as the claude binary would be.
            let lanetally = label == "lanetally";
            let codex = label == "codex";
            let dsh = label == "dsh";
            let exec = label.starts_with("exec");
            let shim = if (claude || lanetally) && case == "obedient" {
                &claude_shim
            } else if codex && case == "obedient" {
                &codex_shim
            } else if dsh && case == "obedient" {
                &dsh_shim
            } else {
                &shim
            };
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            let out = drive(&args, shim, dir.path());
            let kinds: Vec<&str> = out.iter().map(|m| m["type"].as_str().unwrap()).collect();
            let expected: &[&str] = if (claude || lanetally) && case == "obedient" {
                // Transcript, three tool turns, session-finished.
                &[
                    "capabilities",
                    "accepted",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "result",
                ]
            } else if codex && case == "obedient" {
                // Harness launch, transcript, and the four live codex
                // fold rows before session-finished.
                &[
                    "capabilities",
                    "accepted",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "result",
                ]
            } else if dsh && case == "obedient" {
                // Transcript, harness-started, one seat-turn naming what
                // served, then the finishing checkpoint.
                &[
                    "capabilities",
                    "accepted",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "result",
                ]
            } else if dsh || exec || codex {
                // Each still emits an explicit transcript shape: dsh
                // has a retained root, exec says none, and a silent
                // codex has an empty locator.
                &[
                    "capabilities",
                    "accepted",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "result",
                ]
            } else {
                // A silent Claude-shaped stream still reports the common
                // transcript shape before its finishing checkpoint.
                &[
                    "capabilities",
                    "accepted",
                    "checkpoint",
                    "checkpoint",
                    "result",
                ]
            };
            assert_eq!(kinds, expected, "{label}/{case}: {kinds:?}");
            for message in &out {
                assert_eq!(message["proto"], "forge-driver/v1", "{label}");
            }
            assert_seat_records_conform(&out, label, case);
            let transcript_rows: Vec<&Value> = out
                .iter()
                .filter(|message| {
                    message.pointer("/data/step").and_then(Value::as_str) == Some("transcript")
                })
                .collect();
            assert_eq!(
                transcript_rows.len(),
                1,
                "{label}/{case}: exactly one common transcript row: {out:?}"
            );
            let transcript = &transcript_rows[0]["data"]["transcript"];
            let expected_kind = if claude || lanetally {
                "claude-session"
            } else if codex {
                "codex-thread"
            } else if dsh {
                "dsh-session"
            } else {
                "none"
            };
            assert_eq!(transcript["kind"], expected_kind, "{label}/{case}");
            let locator = transcript["locator"].as_str().unwrap();
            assert!(locator.chars().count() <= 80, "{label}/{case}: {locator}");
            assert!(
                transcript.get("content").is_none(),
                "{label}/{case}: paths or ids only: {transcript}"
            );
            if dsh {
                assert!(
                    locator.starts_with("sessions/brokkr/seat-"),
                    "{label}/{case}: {locator}"
                );
                assert!(!locator.starts_with('/'), "{label}/{case}: {locator}");
            } else if exec || case == "silent" {
                assert_eq!(locator, "", "{label}/{case}");
            } else if codex {
                assert_eq!(locator, "codex-thread-1", "{label}/{case}");
            } else {
                assert_eq!(locator, "stream-1", "{label}/{case}");
            }
            if claude && case == "obedient" {
                assert_eq!(
                    out[2]["data"]["step"], "transcript",
                    "{label}: the locator is journaled at init: {}",
                    out[2]
                );
                assert_eq!(
                    out[3]["data"],
                    json!({"step": "seat-turn", "turn": 1, "tool": "Read",
                           "target": "src/lib.rs", "model": "claude-fable-5-1",
                           "effort": "xhigh",
                           "input_tokens":13, "output_tokens":2,
                           "cache_read_tokens":3, "cache_write_tokens":4}),
                    "{label}: {}",
                    out[3]
                );
                assert_eq!(
                    out[4]["data"],
                    json!({"step": "seat-turn", "turn": 2, "tool": "Edit",
                           "target": "src/main.rs", "model": "claude-fable-5-1",
                           "effort": "high",
                           "input_tokens":15, "output_tokens":3,
                           "cache_read_tokens":10}),
                    "{label}: {}",
                    out[4]
                );
                assert_eq!(
                    out[5]["data"],
                    json!({"step": "seat-turn", "turn": 2, "tool": "Write",
                           "target": "src/out.rs", "model": "claude-fable-5-1",
                           "effort": "high"}),
                    "{label}: {}",
                    out[5]
                );
                let finished = &out[6]["data"];
                assert_eq!(finished["step"], "claude-code-session-finished", "{label}");
                assert_eq!(finished["transcript"], *transcript, "{label}");
                assert_eq!(finished["num_turns"], 2, "{label}");
                assert_eq!(finished["total_cost_usd"], 0.125, "{label}");
                assert_eq!(finished["exit_code"], 0, "{label}");
                assert_eq!(finished["model"], "claude-fable-5-1", "{label}");
                assert_eq!(finished["effort"], "high", "{label}");
                assert_eq!(finished["input_tokens"], 28, "{label}");
                assert_eq!(finished["output_tokens"], 5, "{label}");
                assert_eq!(finished["cache_read_tokens"], 13, "{label}");
                assert_eq!(finished["cache_write_tokens"], 4, "{label}");
                // Claude reports its thinking ONLY in the result, so the
                // session record carries it and not one turn does. That
                // absence is decision 0035 ruling 4 in the journal, not
                // an omission: never zero, never back-filled per turn.
                assert_eq!(finished["reasoning_output_tokens"], 4, "{label}");
                for turn in &out[3..6] {
                    assert!(
                        turn["data"].get("reasoning_output_tokens").is_none(),
                        "{label}: a claude turn invents no reasoning count: {turn}"
                    );
                }
                // The capture guard is kind-scoped: only lanetally's
                // finished checkpoint ever carries the marker.
                assert!(finished.get("capture").is_none(), "{label}: {finished}");
            } else if lanetally && case == "obedient" {
                // Same stream, same fold, same disciplines as claude —
                // plus the constant ledger-capture marker and the
                // list-price cost flowing through unchanged.
                assert_eq!(out[2]["data"]["step"], "transcript", "{label}: {}", out[2]);
                assert_eq!(
                    out[3]["data"],
                    json!({"step": "seat-turn", "turn": 1, "tool": "Read",
                           "target": "src/lib.rs", "model": "claude-fable-5-1",
                           "effort": "xhigh",
                           "input_tokens":13, "output_tokens":2,
                           "cache_read_tokens":3, "cache_write_tokens":4}),
                    "{label}: {}",
                    out[3]
                );
                assert_eq!(
                    out[4]["data"],
                    json!({"step": "seat-turn", "turn": 2, "tool": "Edit",
                           "target": "src/main.rs", "model": "claude-fable-5-1",
                           "effort": "high",
                           "input_tokens":15, "output_tokens":3,
                           "cache_read_tokens":10}),
                    "{label}: {}",
                    out[4]
                );
                assert_eq!(
                    out[5]["data"],
                    json!({"step": "seat-turn", "turn": 2, "tool": "Write",
                           "target": "src/out.rs", "model": "claude-fable-5-1",
                           "effort": "high"}),
                    "{label}: {}",
                    out[5]
                );
                let finished = &out[6]["data"];
                assert_eq!(
                    finished["step"], "claude-lanetally-session-finished",
                    "{label}"
                );
                assert_eq!(finished["effort"], "high", "{label}");
                assert_eq!(finished["reasoning_output_tokens"], 4, "{label}");
                assert_eq!(finished["capture"], "lanetally", "{label}: {finished}");
                assert_eq!(finished["transcript"], *transcript, "{label}");
                assert_eq!(finished["num_turns"], 2, "{label}");
                assert_eq!(finished["total_cost_usd"], 0.125, "{label}");
                assert_eq!(finished["exit_code"], 0, "{label}");
                assert_eq!(finished["model"], "claude-fable-5-1", "{label}");
                assert_eq!(finished["input_tokens"], 28, "{label}");
                assert_eq!(finished["output_tokens"], 5, "{label}");
                assert_eq!(finished["cache_read_tokens"], 13, "{label}");
                assert_eq!(finished["cache_write_tokens"], 4, "{label}");
            } else if codex && case == "obedient" {
                // Nobody offered this attempt a session, so the launch
                // is cold and says so with no reason to give: a reason
                // exists only where an offer could not be taken.
                assert_eq!(
                    out[2]["data"],
                    json!({"step":"harness-started", "harness":"codex", "launch":"cold",
                           "model":"not reported", "effort":"not reported"}),
                    "{label}: {}",
                    out[2]
                );
                assert_eq!(
                    out[3]["data"],
                    json!({"step":"transcript", "transcript": transcript,
                           "model":"not reported", "effort":"not reported"}),
                    "{label}: {}",
                    out[3]
                );
                assert_eq!(
                    out[4]["data"],
                    json!({"step":"turn-started", "turn":1, "harness":"codex",
                           "model":"not reported", "effort":"not reported"})
                );
                assert_eq!(out[5]["data"]["tool"], "command_execution");
                assert!(out[5]["data"].get("command").is_none());
                assert_eq!(out[7]["data"]["input_tokens"], 21);
                assert_eq!(out[7]["data"]["cache_read_tokens"], 8);
                assert_eq!(out[7]["data"]["model"], "gpt-5.6-sol");
                // Read from the thread record, not from the stream and
                // not from the pin the seat was launched with: the argv
                // above says `--model gpt-5.6-sol` and no effort at all,
                // and the record says what the harness echoed.
                assert_eq!(out[7]["data"]["effort"], "xhigh");
                // The two counts codex reported all along and the fold
                // dropped until decision 0035 ruling 4 asked for them.
                assert_eq!(out[7]["data"]["cache_write_tokens"], 6);
                assert_eq!(out[7]["data"]["reasoning_output_tokens"], 3);
                assert_eq!(out[8]["data"]["transcript"], *transcript);
                assert_eq!(out[8]["data"]["output_tokens"], 5);
                assert_eq!(out[8]["data"]["model"], "gpt-5.6-sol");
                assert_eq!(out[8]["data"]["effort"], "xhigh");
                assert_eq!(out[8]["data"]["cache_write_tokens"], 6);
                assert_eq!(out[8]["data"]["reasoning_output_tokens"], 3);
            } else if dsh {
                assert_eq!(out[2]["data"]["step"], "transcript");
                assert_eq!(out[3]["data"]["step"], "harness-started");
                assert_eq!(out[3]["data"]["harness"], "deepseek");
                let finished_index = if case == "obedient" {
                    assert_eq!(out[4]["data"]["step"], "seat-turn");
                    assert_eq!(out[4]["data"]["model"], "deepseek-v4-flash");
                    assert_eq!(out[4]["data"]["input_tokens"], 13);
                    5
                } else {
                    4
                };
                assert_eq!(
                    out[finished_index]["data"]["step"],
                    "deepseek-harness-session-finished"
                );
                assert_eq!(out[finished_index]["data"]["transcript"], *transcript);
                assert_eq!(
                    out[finished_index]["data"]["model"],
                    if case == "obedient" {
                        "deepseek-v4-flash"
                    } else {
                        "not reported"
                    }
                );
                // Decision 0035 ruling 3's dsh arm, on every row: the
                // lanes carry a real effort control and neither dsh nor
                // the providers behind it echo any value, so the record
                // says so rather than repeating the pin back.
                for row in out.iter().filter(|m| m["type"] == "checkpoint") {
                    assert_eq!(row["data"]["effort"], "not reported", "{label}: {row}");
                }
                assert!(
                    out.iter()
                        .all(|m| m.pointer("/data/reasoning_output_tokens").is_none()),
                    "{label}: the headless dsh profile reports no reasoning at all"
                );
            } else if exec {
                // Exec has no model usage or transcript. Its command is
                // deliberately not a seat-record target: targets are
                // file paths belonging to numbered tool turns.
                assert_eq!(out[2]["data"]["step"], "transcript", "{label}");
                assert_eq!(out[3]["data"]["step"], "exec-started", "{label}");
                assert!(out[3]["data"].get("target").is_none(), "{label}");
                assert_eq!(out[out.len() - 2]["data"]["transcript"], *transcript);
                assert_eq!(out[out.len() - 2]["data"]["model"], "not applicable");
                // No model turn, therefore no effort to configure for
                // one: the other sentinel, and the distinction between
                // the two is the one dsh above makes visible.
                for row in out.iter().filter(|m| m["type"] == "checkpoint") {
                    assert_eq!(row["data"]["effort"], "not applicable", "{label}: {row}");
                }
            }
            assert_eq!(
                out[out.len() - 2]["data"]["transcript"],
                *transcript,
                "{label}/{case}: finishing session_meta repeats the journal locator"
            );
            let result = out.last().unwrap();
            if case == "obedient" {
                assert_eq!(result["status"], "succeeded", "{label}: {result}");
                assert_eq!(result["result"]["result"], "resolved", "{label}");
                let expected_model = if claude || lanetally {
                    "claude-fable-5-1"
                } else if codex {
                    "gpt-5.6-sol"
                } else if dsh {
                    "deepseek-v4-flash"
                } else {
                    "not applicable"
                };
                assert_eq!(
                    result["result"]["model"], expected_model,
                    "{label}: {result}"
                );
                // The successful result carries the hire's effort beside
                // the model it claims (decision 0035 ruling 3), each
                // driver reporting what its own harness gave it.
                let expected_effort = if claude || lanetally {
                    "high"
                } else if codex {
                    "xhigh"
                } else if dsh {
                    "not reported"
                } else {
                    "not applicable"
                };
                assert_eq!(
                    result["result"]["effort"], expected_effort,
                    "{label}: {result}"
                );
                if claude || lanetally {
                    assert_eq!(result["result"]["input_tokens"], 28, "{label}");
                    assert_eq!(result["result"]["output_tokens"], 5, "{label}");
                    assert_eq!(result["result"]["cache_read_tokens"], 13, "{label}");
                    assert_eq!(result["result"]["cache_write_tokens"], 4, "{label}");
                    assert_eq!(result["result"]["reasoning_output_tokens"], 4, "{label}");
                }
                if codex {
                    assert_eq!(result["result"]["cache_write_tokens"], 6, "{label}");
                    assert_eq!(result["result"]["reasoning_output_tokens"], 3, "{label}");
                }
                if dsh {
                    assert!(
                        result["result"].get("reasoning_output_tokens").is_none(),
                        "{label}: {result}"
                    );
                }
            } else {
                assert_eq!(result["status"], "failed", "{label}: {result}");
                assert!(
                    result["error"].as_str().unwrap().contains("no result file"),
                    "{label}: {result}"
                );
            }
        }
    }
}

#[test]
fn adapters_name_themselves_and_exec_requires_template() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), OBEDIENT_SHIM);
    let expected = [
        ("claude", "claude-code"),
        ("lanetally", "claude-lanetally"),
        ("codex", "codex"),
        ("dsh", "deepseek-harness"),
        ("exec-stdin", "exec"),
        ("exec-promptfile", "exec"),
    ];
    for ((label, args), (_, name)) in all_adapters(&shim).iter().zip(expected) {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = drive(&args, &shim, dir.path());
        assert_eq!(out[0]["driver"], name, "{label}");
    }
    let out = drive(&["exec"], &shim, dir.path());
    let result = out.last().unwrap();
    assert_eq!(result["status"], "failed");
    assert!(result["error"]
        .as_str()
        .unwrap()
        .contains("command template"));
}

// Records its argv, then emits the claude stream shape with an
// adversarial `"capture":"evil"` smuggled into the result event: the
// run_seat source literal must still win on the finished checkpoint.
const LANETALLY_ADVERSARIAL_SHIM: &str = r#"#!/bin/sh
printf '%s\n' "$*" > lanetally-argv.txt
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work"}' > "$target"
printf '{"type":"system","subtype":"init","session_id":"adv-1"}\n'
printf '{"type":"result","num_turns":1,"total_cost_usd":0.5,"capture":"evil"}\n'
"#;

#[test]
fn lanetally_argv_is_claude_shaped_and_the_capture_constant_survives_adversarial_streams() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), LANETALLY_ADVERSARIAL_SHIM);
    let out = drive(
        &["lanetally", "--", "--model", "claude-fable-5-1"],
        &shim,
        dir.path(),
    );
    // The wrapper is invoked exactly as claude would be: the stream-json
    // argv, prompt on stdin (the shim finds the result path only if the
    // prompt arrived there — the succeeded result below proves it).
    let argv = std::fs::read_to_string(dir.path().join("lanetally-argv.txt")).unwrap();
    assert_eq!(
        argv.trim_end(),
        "-p --output-format stream-json --verbose --model claude-fable-5-1"
    );
    let finished = &out[out.len() - 2]["data"];
    assert_eq!(finished["step"], "claude-lanetally-session-finished");
    // The constant is inserted after the session_meta extend: no
    // stream-derived key can shadow it.
    assert_eq!(finished["capture"], "lanetally", "{finished}");
    assert_eq!(finished["total_cost_usd"], 0.5, "{finished}");
    let result = out.last().unwrap();
    assert_eq!(result["status"], "succeeded", "{result}");
}

// ------------------------------------------------------------------
// Sealed secret bindings (decision 0012): injection discipline and the
// masking choke point, exercised against the real exec adapter — and,
// for the result-payload surface, against the lanetally adapter through
// the same shared run_seat choke point.
// ------------------------------------------------------------------

const SECRET_VALUE: &str = "tok3n+v4lue!7";

/// Writes what it can see: the injected environment, the literal argv
/// it was handed, and a stderr leak of the value.
const SECRET_PROBE_SHIM: &str = r#"#!/bin/sh
cat > /dev/null
printf '%s' "$API_TOKEN" > env.txt
printf '%s' "$UNREF" > unref.txt
printf '%s' "$1" > argv.txt
printf 'stderr-leak %s\n' "$API_TOKEN" 1>&2
"#;

/// Honors the result contract but echoes the injected value into its
/// result notes — the leg of the choke point that would otherwise ride
/// EffectSucceeded into the append-only journal.
const NOTES_LEAK_SHIM: &str = r#"#!/bin/sh
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
printf '{"result": "resolved", "notes": "leaked %s"}' "$API_TOKEN" > "$target"
"#;

fn write_store(dir: &Path, lines: &str) -> PathBuf {
    let store = dir.join("secrets.env");
    std::fs::write(&store, lines).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o600)).unwrap();
    store
}

/// Drive an adapter over one attempt with secret bindings in the start
/// input, returning the protocol messages and the driver's own stderr
/// (which carries the masked child-stderr re-emit). `driver_args` is
/// everything after `brokkr driver` — e.g. `["exec", "--", template…]`
/// or `["lanetally"]`.
fn drive_with_secrets(
    driver_args: &[&str],
    workdir: &Path,
    store: &Path,
    secrets: &[&str],
    parent_env: &[(&str, &str)],
) -> (Vec<Value>, String) {
    let result_path = workdir.join("results/fx.json");
    let input = json!({
        "feature": "conformance", "phase": "intake", "seat": "intake",
        "role_path": workdir.join("missing-role.md"),
        "workdir": workdir,
        "result_path": result_path,
        "allowed_results": ["resolved"], "context": {},
        "secrets": secrets,
        "secrets_file": store,
    });
    let messages = [
        json!({"proto": "forge-driver/v1", "msg_id": "m1", "type": "hello",
               "engine_version": "test"}),
        json!({"proto": "forge-driver/v1", "msg_id": "m2", "type": "start",
               "effect_id": "fx", "attempt_id": "a1", "seat": "intake",
               "input": input}),
        json!({"proto": "forge-driver/v1", "msg_id": "m3", "type": "shutdown"}),
    ];
    let operator_home = tempfile::tempdir().unwrap();
    let codex_home = tempfile::tempdir().unwrap();
    let dsh_home = tempfile::tempdir().unwrap();
    let mut command = Command::new(brokkr_bin());
    command.arg("driver").args(driver_args);
    command.env("HOME", operator_home.path());
    command.env("CODEX_HOME", codex_home.path());
    command.env("DSH_HOME", dsh_home.path());
    for (key, value) in parent_env {
        command.env(key, value);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for message in &messages {
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let parsed = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    (parsed, String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn exec_injects_via_env_only_and_masks_the_stderr_reemit() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), SECRET_PROBE_SHIM);
    let store = write_store(
        dir.path(),
        &format!("API_TOKEN={SECRET_VALUE}\nUNREF=unref-value-99\n"),
    );
    let (out, stderr) = drive_with_secrets(
        &["exec", "--", shim.to_str().unwrap(), "{{secret:API_TOKEN}}"],
        dir.path(),
        &store,
        &["API_TOKEN", "UNREF"],
        // A pre-existing child env entry: the declared secret must win.
        &[("API_TOKEN", "from-parent-env")],
    );
    // Env injection carried the value; the declared name overrode the
    // inherited entry.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("env.txt")).unwrap(),
        SECRET_VALUE
    );
    // Every DECLARED name injects, referenced in the template or not.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("unref.txt")).unwrap(),
        "unref-value-99"
    );
    // argv carries the literal shell reference, never the value.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("argv.txt")).unwrap(),
        "$API_TOKEN"
    );
    // Exec command templates stay in the pinned manifest, never in the
    // prose-free seat record.
    assert_eq!(out[2]["data"]["step"], "transcript");
    assert!(out[3]["data"].get("target").is_none());
    assert!(!serde_json::to_string(&out[3])
        .unwrap()
        .contains(SECRET_VALUE));
    // The child's stderr leak reaches the driver's re-emit masked.
    assert!(stderr.contains("[secret:API_TOKEN]"), "stderr: {stderr}");
    assert!(!stderr.contains(SECRET_VALUE), "stderr: {stderr}");
}

#[test]
fn exec_missing_secret_refuses_before_spawn_naming_name_and_path() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), SECRET_PROBE_SHIM);
    let store = write_store(dir.path(), "OTHER=some-other-value\n");
    let (out, _) = drive_with_secrets(
        &["exec", "--", shim.to_str().unwrap()],
        dir.path(),
        &store,
        &["API_TOKEN"],
        &[],
    );
    let kinds: Vec<&str> = out.iter().map(|m| m["type"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        vec!["capabilities", "accepted", "result"],
        "no checkpoint, no spawn: {kinds:?}"
    );
    let result = out.last().unwrap();
    assert_eq!(result["status"], "failed");
    let error = result["error"].as_str().unwrap();
    assert!(error.contains("API_TOKEN"), "{error}");
    assert!(error.contains("secrets.env"), "{error}");
    assert!(
        !error.contains("some-other-value"),
        "never the contents: {error}"
    );
    assert!(
        !dir.path().join("env.txt").exists(),
        "the child must never have spawned"
    );
}

#[test]
fn exec_masks_the_child_written_result_payload() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), NOTES_LEAK_SHIM);
    let store = write_store(dir.path(), &format!("API_TOKEN={SECRET_VALUE}\n"));
    let (out, _) = drive_with_secrets(
        &["exec", "--", shim.to_str().unwrap()],
        dir.path(),
        &store,
        &["API_TOKEN"],
        &[],
    );
    let result = out.last().unwrap();
    assert_eq!(result["status"], "succeeded", "{result}");
    assert_eq!(result["result"]["notes"], "leaked [secret:API_TOKEN]");
    assert!(
        !serde_json::to_string(result)
            .unwrap()
            .contains(SECRET_VALUE),
        "{result}"
    );
}

#[test]
fn lanetally_result_payload_reaches_the_shared_masking_choke_point() {
    // The claude/lanetally arm injects no secret env (bindings feed only
    // the exec arm's spawn), so a shim echoing $API_TOKEN would leak
    // nothing and prove nothing: the store's known plaintext is baked
    // literally into the result notes instead, and run_seat's shared
    // known-plaintext masking — zero lanetally-specific code — must
    // catch it on the way into the journal.
    let dir = tempfile::tempdir().unwrap();
    let leak_shim = format!(
        r#"#!/bin/sh
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
printf '{{"result": "resolved", "notes": "leaked {SECRET_VALUE}"}}' > "$target"
printf '{{"type":"result","num_turns":1,"total_cost_usd":0.0}}\n'
"#
    );
    let shim = make_shim(dir.path(), &leak_shim);
    let store = write_store(dir.path(), &format!("API_TOKEN={SECRET_VALUE}\n"));
    let (out, _) = drive_with_secrets(
        &["lanetally"],
        dir.path(),
        &store,
        &["API_TOKEN"],
        &[("BROKKR_LANETALLY_BIN", shim.to_str().unwrap())],
    );
    let result = out.last().unwrap();
    assert_eq!(result["status"], "succeeded", "{result}");
    assert_eq!(result["result"]["notes"], "leaked [secret:API_TOKEN]");
    assert!(
        !serde_json::to_string(result)
            .unwrap()
            .contains(SECRET_VALUE),
        "{result}"
    );
}
