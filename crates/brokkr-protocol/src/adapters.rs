//! Built-in driver adapters:
//! `brokkr driver <claude|lanetally|codex|dsh|exec>`.
//!
//! The Rust port of the retired Python adapters (decision 0009) — same
//! protocol behavior, same prompt composition, same result-file
//! contract, byte-for-byte compatible with the existing charters. The
//! seat writes its typed result to the file named in the input; a
//! missing file fails the attempt, an unparseable one is forwarded so
//! the ENGINE parks with raw evidence (decision 0001). Adapters never
//! repair anything.
//!
//! Env overrides for conformance shims: FORGE_CLAUDE_BIN,
//! FORGE_LANETALLY_BIN, BROKKR_CODEX_BIN, BROKKR_DSH_BIN,
//! BROKKR_EXEC_NAME. The three `BROKKR_*` names answer to their old
//! `FORGE_*` spelling for one more release (decision 0019, `legacy`);
//! the two that were never renamed read one name only.

use std::io::{BufRead, Read, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Map, Value};

use crate::secret;
use crate::{Body, Message, ResultStatus};

const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    Claude,
    Lanetally,
    Codex,
    Dsh,
    Exec,
}

impl AdapterKind {
    pub fn parse(name: &str) -> Option<AdapterKind> {
        match name {
            "claude" => Some(AdapterKind::Claude),
            "lanetally" => Some(AdapterKind::Lanetally),
            "codex" => Some(AdapterKind::Codex),
            "dsh" => Some(AdapterKind::Dsh),
            "exec" => Some(AdapterKind::Exec),
            _ => None,
        }
    }

    fn driver_name(&self) -> String {
        match self {
            AdapterKind::Claude => "claude-code".to_string(),
            // The Claude Code harness through LaneTally's session-capture
            // wrapper; the ledger discriminator is the checkpoint's
            // `capture` field, never this (forgeable) step-name stem.
            AdapterKind::Lanetally => "claude-lanetally".to_string(),
            AdapterKind::Codex => "codex".to_string(),
            AdapterKind::Dsh => "deepseek-harness".to_string(),
            AdapterKind::Exec => {
                adapter_binary("BROKKR_EXEC_NAME", Some("FORGE_EXEC_NAME"), "exec")
            }
        }
    }
}

fn compose_prompt(input: &Value) -> String {
    let get = |key: &str| input.get(key).and_then(Value::as_str).unwrap_or("");
    let role = input
        .get("role_path")
        .and_then(Value::as_str)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();
    let context = serde_json::to_string_pretty(input.get("context").unwrap_or(&json!({})))
        .unwrap_or_default();
    let allowed = input
        .get("allowed_results")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    format!(
        "{role}\n\n---\n## Task\n\nFeature: {feature}\nPhase: {phase} (you are this \
         phase's only seat)\nWorking directory: {workdir}\n\nRun context \
         (journal-derived, read-only):\n```json\n{context}\n```\n\n## Result contract \
         — MANDATORY\n\nWhen your work is finished, write a JSON object to exactly \
         this file:\n\n    {result_path}\n\nwith the shape:\n\n    {{\"result\": \
         \"<one of: {allowed}>\",\n      \"inputs\": {{ ...optional typed facts for \
         the phase machine... }},\n      \"notes\": \"<short human summary of what \
         you did and why>\"}}\n\nThe file is the ONLY channel the engine reads. \
         Printing the JSON instead of writing the file counts as producing no \
         result. You never decide the next phase — the engine's policy table rules \
         on your typed result.\n",
        role = role,
        feature = get("feature"),
        phase = get("phase"),
        workdir = get("workdir"),
        context = context,
        result_path = get("result_path"),
        allowed = allowed,
    )
}

struct Invocation {
    exit_code: i32,
    session_meta: Map<String, Value>,
    stderr: String,
}

fn io_context<T>(result: std::io::Result<T>, context: &str) -> Result<T, String> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => Err(format!("{context}: {error}")),
    }
}

/// One reader for every override, so the one-release fallback and its
/// one-time note are wired once rather than per variable. `legacy` is
/// the old `FORGE_*` spelling where decision 0019 renamed the variable,
/// and `None` where it did not.
fn adapter_binary(primary: &str, legacy: Option<&str>, fallback: &str) -> String {
    crate::legacy::env(primary, legacy).unwrap_or_else(|| fallback.to_string())
}

fn write_prompt(writer: &mut impl Write, payload: &str) -> Result<(), String> {
    io_context(
        writer.write_all(payload.as_bytes()),
        "could not write the prompt",
    )
}

fn run_cli(
    command: &[String],
    stdin_payload: Option<&str>,
    workdir: &str,
    bindings: &[secret::BoundSecret],
) -> Result<std::process::Output, String> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| "empty command".to_string())?;
    let mut invocation = Command::new(program);
    invocation
        .args(args)
        .current_dir(if workdir.is_empty() { "." } else { workdir })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Injection discipline (decision 0012, layer 3): values reach the
    // child ONLY through its environment, resolved at spawn time — never
    // argv (/proc/*/cmdline is world-readable), never the template. This
    // is the sole production call site of expose_for_spawn, CI-grep
    // pinned. A declared name overrides any pre-existing env entry: the
    // declaration is in the reviewed charter, so a collision is visible
    // at review time.
    for binding in bindings {
        let value = match std::str::from_utf8(binding.secret().expose_for_spawn()) {
            Ok(value) => value,
            Err(_) => return Err(format!("secret '{}' is not valid UTF-8", binding.name())),
        };
        invocation.env(binding.name(), value);
    }
    let mut child = io_context(invocation.spawn(), "could not invoke the agent CLI")?;
    if let Some(payload) = stdin_payload {
        let mut stdin = child.stdin.take().expect("piped");
        write_prompt(&mut stdin, payload)?;
    } else {
        drop(child.stdin.take());
    }
    io_context(child.wait_with_output(), "agent CLI did not conclude")
}

/// One parsed claude stream-json line folded into the seat's telemetry:
/// `system`/`init` and `result` feed `session_meta`; each `tool_use`
/// block of an `assistant` message becomes one seat-turn checkpoint,
/// in block order, all carrying the message's turn number.
/// Privacy invariant (journal is evidence, not transcript): checkpoints
/// carry turn index, a ≤80-char tool name, and a ≤80-char target only —
/// never message text, thinking, or full tool inputs.
fn fold_stream_event(
    event: &Value,
    assistant_turns: &mut u64,
    session_meta: &mut Map<String, Value>,
    emit: &mut impl FnMut(&Value),
) {
    match event.get("type").and_then(Value::as_str) {
        Some("system") if event.get("subtype").and_then(Value::as_str) == Some("init") => {
            if let Some(session_id) = event.get("session_id").and_then(Value::as_str) {
                let clamped: String = session_id.chars().take(128).collect();
                session_meta.insert("session_id".into(), Value::String(clamped.clone()));
                // Journaled NOW, not only at session end: the
                // transcript drilldowns can only locate — and live-
                // stream — a WORKING seat's prose if the id is known
                // from the first message. The id is display-guarded
                // downstream (hex and dash only) like every session id.
                let mut checkpoint = Map::new();
                checkpoint.insert("step".into(), Value::String("session-started".into()));
                checkpoint.insert("session_id".into(), Value::String(clamped));
                emit(&Value::Object(checkpoint));
            }
        }
        Some("assistant") => {
            *assistant_turns += 1;
            let blocks = event.pointer("/message/content").and_then(Value::as_array);
            let tool_uses = blocks
                .into_iter()
                .flatten()
                .filter(|b| b.get("type").and_then(Value::as_str) == Some("tool_use"));
            for tool_use in tool_uses {
                let mut checkpoint = Map::new();
                checkpoint.insert("step".into(), Value::String("seat-turn".into()));
                checkpoint.insert("turn".into(), Value::from(*assistant_turns));
                let tool = tool_use.get("name").and_then(Value::as_str).unwrap_or("");
                checkpoint.insert(
                    "tool".into(),
                    Value::String(tool.chars().take(80).collect()),
                );
                // file_path ONLY: commands and URLs can embed inline secrets,
                // and the journal is append-only — the forge-verify review
                // hard-stopped on exactly this (run verify-…-917996f5). Full
                // detail belongs to the resumable transcript, not the record.
                let target = tool_use.pointer("/input/file_path").and_then(Value::as_str);
                if let Some(target) = target {
                    checkpoint.insert(
                        "target".into(),
                        Value::String(target.chars().take(80).collect()),
                    );
                }
                emit(&Value::Object(checkpoint));
            }
        }
        Some("result") => {
            for key in ["num_turns", "total_cost_usd"] {
                if let Some(value) = event.get(key) {
                    session_meta.insert(key.to_string(), value.clone());
                }
            }
        }
        _ => {}
    }
}

/// Fold Codex's stable `exec --json` JSONL into bounded live telemetry.
/// Commands, item bodies, model output, reasoning, and filesystem paths are
/// intentionally ignored; the journal records progress and usage, not a
/// transcript.
fn fold_codex_event(
    event: &Value,
    turn: &mut u64,
    session_meta: &mut Map<String, Value>,
    emit: &mut impl FnMut(&Value),
) {
    match event.get("type").and_then(Value::as_str) {
        Some("thread.started") => {
            if let Some(thread_id) = event.get("thread_id").and_then(Value::as_str) {
                let clamped: String = thread_id.chars().take(128).collect();
                session_meta.insert("session_id".into(), Value::String(clamped.clone()));
                // Journaled the moment the thread opens, not only inside
                // the finishing checkpoint that carries session_meta at
                // exit. `thread.started` is the FIRST line codex-cli
                // 0.148.0 writes — verified on the installed binary,
                // `{"type":"thread.started","thread_id":"01a0619c-…"}`
                // ahead of `turn.started` — so until now `brokkr inspect
                // --seat` on a WORKING codex seat showed no session id
                // at all and the live transcript drilldown had nothing
                // to open. Same discipline as the claude and dsh folds.
                emit(&json!({
                    "step":"session-started",
                    "harness":"codex",
                    "session_id": clamped,
                }));
            }
        }
        Some("turn.started") => {
            *turn += 1;
            emit(&json!({"step":"turn-started", "turn": *turn, "harness":"codex"}));
        }
        Some(kind @ ("item.started" | "item.completed")) => {
            let item_type = event
                .pointer("/item/type")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            emit(&json!({
                "step": if kind == "item.started" { "item-started" } else { "item-completed" },
                "turn": *turn,
                "tool": item_type.chars().take(80).collect::<String>(),
                "harness":"codex",
            }));
        }
        Some("turn.completed") => {
            let usage = event.get("usage").unwrap_or(&Value::Null);
            let mut checkpoint = Map::new();
            checkpoint.insert("step".into(), Value::String("turn-completed".into()));
            checkpoint.insert("turn".into(), Value::from(*turn));
            checkpoint.insert("harness".into(), Value::String("codex".into()));
            for (source, target) in [
                ("input_tokens", "input_tokens"),
                ("cached_input_tokens", "cache_read_tokens"),
                ("output_tokens", "output_tokens"),
            ] {
                if let Some(value) = usage.get(source).and_then(Value::as_u64) {
                    checkpoint.insert(target.into(), Value::from(value));
                    add_usage(session_meta, target, value);
                }
            }
            // The turn count is journaled HERE, not only from `result`.
            // Verified against codex-cli 0.148.0: a real `codex exec
            // --json` run ends at `turn.completed` and emits no `result`
            // event at all, so a codex seat's num_turns never reached
            // `brokkr costs` — the aggregator saw per-turn token keys it
            // does not sum and nothing else. Cost in USD is reported
            // nowhere in that stream (`usage` is token counts only), so
            // total_cost_usd stays absent rather than invented.
            session_meta.insert("num_turns".into(), Value::from(*turn));
            emit(&Value::Object(checkpoint));
        }
        // Conformance shims and older clients may provide only final metadata.
        Some("result") => {
            if let Some(session_id) = event.get("session_id").and_then(Value::as_str) {
                session_meta.insert(
                    "session_id".into(),
                    Value::String(session_id.chars().take(128).collect()),
                );
            }
            for key in ["num_turns", "total_cost_usd"] {
                if let Some(value) = event.get(key) {
                    session_meta.insert(key.into(), value.clone());
                }
            }
        }
        _ => {}
    }
}

/// One harness-reported token count folded into the session totals.
/// Usage ACCUMULATES: a plain insert left a multi-turn session's meta
/// holding only its LAST turn's counts, which is the number every cost
/// surface then read as the session's.
fn add_usage(session_meta: &mut Map<String, Value>, key: &str, value: u64) {
    let total = session_meta
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(value);
    session_meta.insert(key.to_string(), Value::from(total));
}

/// The dsh usage keys, as `@deepseek-ai/dsh-llm`'s `TokenUsage` spells
/// them, paired with the journal's own names.
///
/// dsh's own counts are DISJOINT: a cache read is NOT part of
/// `inputTokens`. Verified by arithmetic against 0.1.0-rc.6 — a step
/// whose provider reported `prompt_tokens: 101` with `cached_tokens: 7`
/// was written to the transcript as `inputTokens: 94, cacheReadTokens:
/// 7`, and the next step's 103/21 as 82/21.
///
/// The journal's `input_tokens` is INCLUSIVE, because that is what the
/// key already meant everywhere else: codex reports `input_tokens:
/// 14830` beside `cached_input_tokens: 11264` (codex-cli 0.148.0, one
/// real `codex exec --json` turn), and `brokkr-view::session_tokens`
/// sums `input_tokens` and `output_tokens` *only*, documenting that a
/// cache read arrives inside the input count and adding it again would
/// double-count. So `DSH_INPUT_CACHE_READ` is folded back into the
/// input count below rather than leaving one journal key meaning one
/// thing for codex and another for dsh.
const DSH_USAGE: [(&str, &str); 3] = [
    ("inputTokens", "input_tokens"),
    ("outputTokens", "output_tokens"),
    (DSH_INPUT_CACHE_READ, "cache_read_tokens"),
];

/// The dsh count that is a subset of the journal's `input_tokens` but a
/// sibling of dsh's own `inputTokens`.
const DSH_INPUT_CACHE_READ: &str = "cacheReadTokens";

/// One dsh session-log line folded into bounded live telemetry.
///
/// dsh 0.1.0-rc.6's headless profile offers no machine-readable stdout
/// stream — verified against the installed binary: `dsh --help` lists
/// `-V/--profile/--patch/--dump-config/--dump-default-config` and the
/// `web`/`plugin` commands, none of them an output format, and `dsh
/// --profile headless --help` lists `-h` alone under "Answer one task,
/// print the final assistant message, and exit". The live signal is
/// therefore the JSONL transcript
/// `@deepseek-ai/dsh-session-persistence-jsonl` appends as the session
/// runs (observed growing 14 → 45 → 71 → 89 lines while the child was
/// still alive).
///
/// The log's first line is the immutable session header, which names the
/// session; every later line is one `SessionEvent`. One real session,
/// driven end to end through 0.1.0-rc.6 with a tool call in it, wrote:
///
/// ```text
/// session · permission/preset · sandbox/mode · approval/policy
/// agent/inbox/spliced · turn/start · step/start · user/message
/// request/header · request/context · assistant/chunk × 9
/// assistant/message · tool/call · tool/result · step/end
/// step/start · assistant/chunk × 5 · assistant/message · step/end
/// turn/end
/// ```
///
/// So an `assistant/message` is one assembled assistant *step*, not one
/// dsh turn: dsh authors its own coarser `data.turn`, and the two
/// messages above both carry `turn: 1` with `step: 1` and `step: 2`. A
/// headless seat answers one task, so dsh's own turn index never leaves
/// 1 and would report no progress at all. The step is the unit that
/// advances, and it is the same unit the claude fold counts — one
/// assembled assistant message per model response. It carries that
/// step's `usage` when the adapter reported accounting; a `tool/call`
/// names the tool the step asked for.
///
/// Privacy invariant, same as `fold_stream_event`: a turn index, a
/// ≤80-char tool name and token counts only — never message text,
/// reasoning, or tool arguments. dsh's `arguments` is the model's own
/// unparsed JSON string, so no target is derived from it at all.
fn fold_dsh_event(
    event: &Value,
    turns: &mut u64,
    session_meta: &mut Map<String, Value>,
    emit: &mut impl FnMut(&Value),
) {
    match event.get("type").and_then(Value::as_str) {
        Some("session") => {
            if let Some(session_id) = event.get("id").and_then(Value::as_str) {
                let clamped: String = session_id.chars().take(128).collect();
                session_meta.insert("session_id".into(), Value::String(clamped.clone()));
                // Journaled the moment the header lands, not at exit:
                // the transcript drilldowns can only locate a WORKING
                // seat's session if the id is already in the journal.
                emit(&json!({
                    "step":"session-started",
                    "harness":"deepseek",
                    "session_id": clamped,
                }));
            }
        }
        Some("assistant/message") => {
            *turns += 1;
            session_meta.insert("num_turns".into(), Value::from(*turns));
            let mut checkpoint = Map::new();
            checkpoint.insert("step".into(), Value::String("seat-turn".into()));
            checkpoint.insert("turn".into(), Value::from(*turns));
            checkpoint.insert("harness".into(), Value::String("deepseek".into()));
            let usage = event.pointer("/data/usage").unwrap_or(&Value::Null);
            let cache_read = usage
                .get(DSH_INPUT_CACHE_READ)
                .and_then(Value::as_u64)
                .unwrap_or(0);
            for (source, target) in DSH_USAGE {
                if let Some(mut value) = usage.get(source).and_then(Value::as_u64) {
                    // The one place dsh's disjoint accounting is
                    // reconciled with the journal's inclusive
                    // `input_tokens` — see DSH_USAGE.
                    if source == "inputTokens" {
                        value = value.saturating_add(cache_read);
                    }
                    checkpoint.insert(target.into(), Value::from(value));
                    add_usage(session_meta, target, value);
                }
            }
            emit(&Value::Object(checkpoint));
        }
        Some("tool/call") => {
            let tool = event
                .pointer("/data/name")
                .and_then(Value::as_str)
                .unwrap_or("");
            emit(&json!({
                "step":"seat-turn",
                "turn": *turns,
                "harness":"deepseek",
                "tool": tool.chars().take(80).collect::<String>(),
            }));
        }
        _ => {}
    }
}

/// The fixed transcript filename the JSONL backend writes inside each
/// session-owned directory (`<root>/--<cwd>--/<id>/session.jsonl`).
const DSH_TRANSCRIPT: &str = "session.jsonl";

/// The seat's own transcript under a per-seat root. NEVER "the newest
/// file": the root is fresh and belongs to this invocation alone, so no
/// directory scan can lose a race against a concurrent seat.
///
/// One root is not the same claim as one session, though. dsh's session
/// header carries a `delegationDepth`, so a session this seat delegates
/// writes a SECOND transcript under the same root, and `read_dir` yields
/// entries in whatever order the filesystem likes. The seat's own
/// session is therefore chosen by what its header says — depth 0 — and
/// not by which entry happened to come back first.
fn find_dsh_transcript(root: &std::path::Path) -> Option<std::path::PathBuf> {
    for project in std::fs::read_dir(root).ok()?.flatten() {
        for session in std::fs::read_dir(project.path())
            .into_iter()
            .flatten()
            .flatten()
        {
            let candidate = session.path().join(DSH_TRANSCRIPT);
            if names_the_seats_own_session(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

/// Whether a transcript's first line is the header of the session this
/// seat booted, rather than one it delegated.
///
/// A file whose header has not landed yet — dsh creates it before its
/// first append, and a half-written line is not JSON — is not the answer
/// *yet*: the poll loop simply asks again on its next pass, which is the
/// same discipline as a root that does not exist at boot.
fn names_the_seats_own_session(candidate: &std::path::Path) -> bool {
    let Ok(file) = std::fs::File::open(candidate) else {
        return false;
    };
    let mut header = String::new();
    if std::io::BufReader::new(file)
        .read_line(&mut header)
        .is_err()
    {
        return false;
    }
    let Ok(event) = serde_json::from_str::<Value>(&header) else {
        return false;
    };
    event.get("type").and_then(Value::as_str) == Some("session")
        && event
            .get("delegationDepth")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0
}

/// What the driver has read of the growing transcript so far: the open
/// handle and the bytes after its last complete line.
#[derive(Default)]
struct DshTail {
    file: Option<std::fs::File>,
    pending: Vec<u8>,
}

/// Fold whatever the child has appended since the last pass. Called
/// while the process still runs, so a partly-written last line stays in
/// `pending` until its newline arrives; a line that is not JSON is
/// noise, dropped, never repaired (decision 0001).
///
/// TRUST BOUNDARY, and it is weaker than claude's. Claude's stream-json
/// arrives on a pipe only the child holds; this is an ordinary file in a
/// directory the seat's own agent can write to, and the driver publishes
/// its path in `harness-started.target`. An agent with filesystem tools
/// can therefore append lines this fold will believe — a forged session
/// header renames the seat's session, forged `assistant/message` lines
/// inflate the turn and token counts. That is accepted, not overlooked:
/// such an agent is already trusted with the working tree, the fold
/// clamps every value it takes (≤128-char id, ≤80-char tool name, u64
/// counts) and derives nothing executable from any of them, so the worst
/// case is a wrong number and a wrong drilldown in the journal, never an
/// injection or a control-flow decision. A driver whose harness offers a
/// real stdout stream should use it rather than inherit this.
fn drain_dsh_transcript(
    tail: &mut DshTail,
    root: &std::path::Path,
    turns: &mut u64,
    session_meta: &mut Map<String, Value>,
    emit: &mut impl FnMut(&Value),
) {
    if tail.file.is_none() {
        tail.file = find_dsh_transcript(root).and_then(|path| std::fs::File::open(path).ok());
    }
    let Some(file) = tail.file.as_mut() else {
        return;
    };
    let mut chunk = Vec::new();
    let _ = file.read_to_end(&mut chunk);
    tail.pending.extend_from_slice(&chunk);
    while let Some(index) = tail.pending.iter().position(|byte| *byte == b'\n') {
        let line: Vec<u8> = tail.pending.drain(..=index).collect();
        let Ok(event) = serde_json::from_slice::<Value>(&line) else {
            continue;
        };
        fold_dsh_event(&event, turns, session_meta, emit);
    }
}

/// How long the poll loop idles between passes over a running seat's
/// transcript.
const DSH_POLL_IDLE: std::time::Duration = std::time::Duration::from_millis(25);

/// Carry a running seat to its exit code: sample the child's state, then
/// drain whatever it has appended. Exit is sampled BEFORE the drain, so
/// the pass that follows the last one reads a settled file — nothing the
/// child wrote can be missed by the driver losing a race with its own
/// child's exit.
///
/// A `wait` that ERRORS is terminal, never "not finished yet". Folding
/// that error back into the loop is exactly how a seat goes silent
/// forever: a persistent `waitpid` failure — ECHILD, if anything in the
/// process reaps children out from under this one — would spin here at
/// `DSH_POLL_IDLE` with no result, no error and no exit, which is the
/// invisible seat this whole driver change exists to end. The buffered
/// path this loop replaced surfaced such a failure as `agent CLI did not
/// conclude`; so does this one.
fn poll_until_exit(
    mut wait: impl FnMut() -> std::io::Result<Option<i32>>,
    mut drain: impl FnMut(),
) -> Result<i32, String> {
    loop {
        let finished = io_context(wait(), "agent CLI did not conclude")?;
        drain();
        if let Some(code) = finished {
            return Ok(code);
        }
        std::thread::sleep(DSH_POLL_IDLE);
    }
}

fn stage_prompt_with<Create, WritePrompt>(
    prompt: &str,
    create: Create,
    write_prompt: WritePrompt,
) -> Result<tempfile::NamedTempFile, String>
where
    Create: FnOnce() -> std::io::Result<tempfile::NamedTempFile>,
    WritePrompt: FnOnce(&mut tempfile::NamedTempFile, &[u8]) -> std::io::Result<()>,
{
    let mut file = io_context(create(), "could not stage the prompt")?;
    io_context(
        write_prompt(&mut file, prompt.as_bytes()),
        "could not stage the prompt",
    )?;
    Ok(file)
}

fn stage_prompt(prompt: &str) -> Result<tempfile::NamedTempFile, String> {
    stage_prompt_with(
        prompt,
        || {
            tempfile::Builder::new()
                .prefix("forge-prompt-")
                .suffix(".md")
                .tempfile()
        },
        |file, bytes| file.write_all(bytes),
    )
}

/// The claude stream-json invocation, parameterized ONLY by the harness
/// binary (never an `AdapterKind` — the claude and lanetally arms must
/// stay byte-identical in behavior, so kind-specific drift is
/// unrepresentable here). Three invariants, each load-bearing:
/// stdout is folded LIVE line-by-line (checkpoints are streamed
/// telemetry — converging on `wait_with_output` buffering is a rejected
/// design); stderr drains on its own thread so a chatty session — or a
/// chatty wrapper layer — cannot deadlock the stdout fold; unparseable
/// stream lines are noise, never repaired (decision 0001), which is
/// what makes wrapper-interleaved non-JSON output safe.
fn invoke_stream_json(
    bin: String,
    extra: &[String],
    prompt: &str,
    workdir: &str,
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    let mut command = vec![
        bin,
        "-p".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--verbose".into(),
    ];
    command.extend(extra.iter().cloned());
    let (program, args) = (&command[0], &command[1..]);
    let child = Command::new(program)
        .args(args)
        .current_dir(if workdir.is_empty() { "." } else { workdir })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = io_context(child, "could not invoke the agent CLI")?;
    {
        let mut stdin = child.stdin.take().expect("piped");
        io_context(
            stdin.write_all(prompt.as_bytes()),
            "could not write the prompt",
        )?;
    }
    // stderr drains on its own thread so a chatty session cannot
    // deadlock the stdout stream we are folding live.
    let stderr_pipe = child.stderr.take().expect("piped");
    let stderr_thread = std::thread::spawn(move || {
        let mut captured = Vec::new();
        let mut pipe = stderr_pipe;
        let _ = pipe.read_to_end(&mut captured);
        String::from_utf8_lossy(&captured).into_owned()
    });
    let stdout = child.stdout.take().expect("piped");
    let mut session_meta = Map::new();
    let mut assistant_turns = 0u64;
    for line in std::io::BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        // Unparseable stream lines are noise, never repaired
        // (decision 0001).
        let Ok(event) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        fold_stream_event(&event, &mut assistant_turns, &mut session_meta, emit);
    }
    let status = io_context(child.wait(), "agent CLI did not conclude")?;
    Ok(Invocation {
        exit_code: status.code().unwrap_or(-1),
        session_meta,
        stderr: stderr_thread.join().unwrap_or_default(),
    })
}

/// The dsh headless invocation. It does NOT converge on `run_cli`'s
/// buffered `wait_with_output`: that is what left a dsh seat silent from
/// `harness-started` to process exit — the wager challenger run
/// scaffold-tool-grants-per-stack-b-58476f86 sat at journal seq 5 for an
/// entire implement seat with no evidence it was alive but a process
/// table. Instead the child's own session transcript is followed as it
/// grows, so each assistant turn advances the journal while the seat
/// works, the way the claude fold does with stream-json.
///
/// stdin and stdout are `/dev/null`: the headless runner reads no stdin
/// and prints only its final answer, which this driver has never
/// journaled. stderr is piped and drained on its own thread so a chatty
/// session cannot deadlock the poll loop.
fn invoke_dsh(
    extra: &[String],
    prompt: &str,
    workdir: &str,
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    invoke_dsh_with(extra, prompt, workdir, emit, |child| {
        child
            .try_wait()
            .map(|status| status.map(|status| status.code().unwrap_or(-1)))
    })
}

/// The same invocation with the one question the OS answers — "is the
/// child still running?" — injectable, the way `stage_prompt_with` and
/// `dsh_transcript_root_in` make their own syscalls injectable. A real
/// `waitpid` failure cannot be provoked from a test, and the arm that
/// handles it is the difference between a seat that reports a refusal
/// and a seat that spins in silence forever, so it is reachable here.
fn invoke_dsh_with(
    extra: &[String],
    prompt: &str,
    workdir: &str,
    emit: &mut impl FnMut(&Value),
    mut wait: impl FnMut(&mut std::process::Child) -> std::io::Result<Option<i32>>,
) -> Result<Invocation, String> {
    let bin = adapter_binary("BROKKR_DSH_BIN", Some("FORGE_DSH_BIN"), "dsh");
    let (model, passthrough) = split_dsh_model(extra)?;
    // Bound for the whole invocation: dropping it removes the seat's
    // transcript, so it must outlive the poll loop that reads it.
    let root_dir = dsh_transcript_root()?;
    let root = root_dir.path();
    let overlay = dsh_seat_overlay(model.as_deref(), root)?;
    emit(&json!({
        "step":"harness-started",
        "harness":"deepseek",
        "profile":"headless",
        "model": model,
        // A directory path, within the same 80-char clamp every target
        // takes: while the seat works, the journal alone says where its
        // live transcript is, which is what a `brokkr watch` drilldown
        // needs. It is the seat's own root and goes with the seat.
        "target": root.to_string_lossy().chars().take(80).collect::<String>(),
    }));
    let mut command = vec![
        bin,
        "--profile".into(),
        "headless".into(),
        "--patch".into(),
        overlay.path().to_string_lossy().into_owned(),
    ];
    command.extend(passthrough);
    command.push(prompt.into());
    let child = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(if workdir.is_empty() { "." } else { workdir })
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = io_context(child, "could not invoke the agent CLI")?;
    let stderr_pipe = child.stderr.take().expect("piped");
    let stderr_thread = std::thread::spawn(move || {
        let mut captured = Vec::new();
        let mut pipe = stderr_pipe;
        let _ = pipe.read_to_end(&mut captured);
        String::from_utf8_lossy(&captured).into_owned()
    });
    let mut session_meta = Map::new();
    let mut turns = 0u64;
    let mut tail = DshTail::default();
    let exit_code = poll_until_exit(
        || wait(&mut child),
        || drain_dsh_transcript(&mut tail, root, &mut turns, &mut session_meta, emit),
    )?;
    session_meta.insert("harness".into(), Value::String("deepseek".into()));
    session_meta.insert("profile".into(), Value::String("headless".into()));
    if let Some(model) = model {
        session_meta.insert("model".into(), Value::String(model));
    }
    Ok(Invocation {
        exit_code,
        session_meta,
        stderr: stderr_thread.join().unwrap_or_default(),
    })
}

fn invoke(
    kind: AdapterKind,
    extra: &[String],
    prompt: &str,
    input: &Value,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    invoke_with_stager(kind, extra, prompt, input, bindings, emit, stage_prompt)
}

#[allow(clippy::too_many_arguments)]
fn invoke_with_stager(
    kind: AdapterKind,
    extra: &[String],
    prompt: &str,
    input: &Value,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Value),
    mut stage: impl FnMut(&str) -> Result<tempfile::NamedTempFile, String>,
) -> Result<Invocation, String> {
    let workdir = input
        .get("workdir")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    match kind {
        AdapterKind::Claude => invoke_stream_json(
            adapter_binary("FORGE_CLAUDE_BIN", None, "claude"),
            extra,
            prompt,
            &workdir,
            emit,
        ),
        // Same harness, same stream: LaneTally's wrapper is
        // argv-compatible with claude (including stream-json), so the
        // only difference IS the binary. No spawn-time fallback to plain
        // `claude` when the wrapper is missing — that would silently
        // un-capture sessions; doctor is the advisory surface.
        AdapterKind::Lanetally => invoke_stream_json(
            adapter_binary("FORGE_LANETALLY_BIN", None, "claude-lanetally"),
            extra,
            prompt,
            &workdir,
            emit,
        ),
        AdapterKind::Codex => {
            let bin = adapter_binary("BROKKR_CODEX_BIN", Some("FORGE_CODEX_BIN"), "codex");
            let mut command = vec![
                bin,
                "exec".into(),
                "--json".into(),
                "-C".into(),
                workdir.clone(),
            ];
            command.extend(extra.iter().cloned());
            let (program, args) = (&command[0], &command[1..]);
            let child = Command::new(program)
                .args(args)
                .current_dir(if workdir.is_empty() { "." } else { &workdir })
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();
            let mut child = io_context(child, "could not invoke the agent CLI")?;
            let mut stdin = child.stdin.take().expect("piped");
            io_context(
                stdin.write_all(prompt.as_bytes()),
                "could not write the prompt",
            )?;
            drop(stdin);
            let stderr_pipe = child.stderr.take().expect("piped");
            let stderr_thread = std::thread::spawn(move || {
                let mut captured = Vec::new();
                let mut pipe = stderr_pipe;
                let _ = pipe.read_to_end(&mut captured);
                String::from_utf8_lossy(&captured).into_owned()
            });
            let mut session_meta = Map::new();
            let mut turn = 0;
            for line in std::io::BufReader::new(child.stdout.take().expect("piped")).lines() {
                let Ok(line) = line else { break };
                if let Ok(event) = serde_json::from_str::<Value>(&line) {
                    fold_codex_event(&event, &mut turn, &mut session_meta, emit);
                }
            }
            let status = io_context(child.wait(), "agent CLI did not conclude")?;
            Ok(Invocation {
                exit_code: status.code().unwrap_or(-1),
                session_meta,
                stderr: stderr_thread.join().unwrap_or_default(),
            })
        }
        AdapterKind::Dsh => invoke_dsh(extra, prompt, &workdir, emit),
        AdapterKind::Exec => {
            if extra.is_empty() {
                return Err("exec driver needs a command template after '--'".to_string());
            }
            // Checkpoint-target amendment (decision 0012): the
            // UNRESOLVED template — the exact artifact the compile lint
            // proved value-free — is journalable within the 80-char
            // clamp. Resolved command lines, URLs, and prose never are.
            emit(&json!({
                "step": "exec-started",
                "target": extra.join(" ").chars().take(80).collect::<String>(),
            }));
            let mut prompt_file: Option<tempfile::NamedTempFile> = None;
            if extra.iter().any(|part| part.contains("{prompt_file}")) {
                prompt_file = Some(stage(prompt)?);
            }
            let prompt_path = prompt_file
                .as_ref()
                .map(|f| f.path().to_string_lossy().into_owned())
                .unwrap_or_default();
            let command: Vec<String> = extra
                .iter()
                .map(|part| resolve_exec_part(part, &workdir, &prompt_path, bindings))
                .collect();
            let stdin_payload = if prompt_file.is_none() {
                Some(prompt)
            } else {
                None
            };
            let out = run_cli(&command, stdin_payload, &workdir, bindings)?;
            // Known-plaintext masking choke point (decision 0012, layer
            // 5), on RAW captured bytes before any string conversion.
            // stdout is captured-and-dropped today; stderr is re-emitted
            // by run_seat and journaled as the stderr tail on failure —
            // the path that fires exactly when a credentialed command
            // prints the offending header.
            let stderr = secret::mask_bytes(&out.stderr, bindings);
            Ok(Invocation {
                exit_code: out.status.code().unwrap_or(-1),
                session_meta: Map::new(),
                stderr: String::from_utf8_lossy(&stderr).into_owned(),
            })
        }
    }
}

/// The provider row dsh's headless profile boots its agent on when the
/// pinned model names none. A patch overlay replaces the targeted row's
/// WHOLE config (dsh-base's own words), so the overlay that pins a
/// model must restate the provider or the boot loses it.
const DSH_PROVIDER: &str = "deepseek-official";

/// One pinned model, as dsh addresses it: a provider route in the
/// profile tree and a model id that route serves. `<id>` alone is the
/// official DeepSeek route; `<provider>/<id>` names another route the
/// profile declares — `dashscope/qwen3.8-max` for Model Studio. The
/// split is on the first slash and the id keeps none, so a route name
/// and a model id are each one plain identifier.
struct DshModel<'a> {
    provider: &'a str,
    model: &'a str,
}

fn parse_dsh_model(pinned: &str) -> Result<DshModel<'_>, String> {
    let plain = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
    };
    let (provider, model) = match pinned.split_once('/') {
        Some((provider, model)) => (provider, model),
        None => (DSH_PROVIDER, pinned),
    };
    if !plain(provider) || !plain(model) {
        return Err(format!(
            "dsh driver: model {pinned:?} is not `<id>` or `<provider>/<id>` of plain identifiers"
        ));
    }
    Ok(DshModel { provider, model })
}

/// The dsh launcher takes no model flag: the model is one row of the
/// composed profile tree (`agent-default-model`), and the launcher's
/// only override channel is `--patch <overlay.yml>`, applied last. So
/// the adapter data says `model_flag: "--model"` — the pinning grammar
/// every provider shares — and this driver is where `--model <id>`
/// becomes the overlay dsh actually reads. Everything after `--` that
/// is not that pair passes through to the launcher unchanged.
fn split_dsh_model(extra: &[String]) -> Result<(Option<String>, Vec<String>), String> {
    let mut model = None;
    let mut passthrough = Vec::with_capacity(extra.len());
    let mut parts = extra.iter();
    while let Some(part) = parts.next() {
        if part == "--model" {
            let id = parts
                .next()
                .ok_or_else(|| "dsh driver: --model needs a model id after it".to_string())?;
            if model.replace(id.clone()).is_some() {
                return Err("dsh driver: --model given twice".to_string());
            }
        } else {
            passthrough.push(part.clone());
        }
    }
    Ok((model, passthrough))
}

/// The pinned-model row of the seat overlay, in the loader-patch
/// grammar dsh composes after every bundle and profile layer, staged
/// over an injected file so the two ways staging can fail — no file, a
/// file that takes no bytes — are reachable from a test without a full
/// disk. The id is written into YAML verbatim, so it is confined to the
/// characters a model id is made of — a model name is data the operator
/// pinned, never a place to smuggle a second row into the tree.
fn dsh_model_overlay_in(
    model: &str,
    create: impl FnOnce() -> std::io::Result<tempfile::NamedTempFile>,
) -> Result<tempfile::NamedTempFile, String> {
    let DshModel { provider, model } = parse_dsh_model(model)?;
    let mut file = io_context(create(), "could not stage the dsh model overlay")?;
    let body = format!(
        "# Written by `brokkr driver dsh` for one seat: the pinned model, as the\n\
         # overlay dsh's launcher composes last. A patch replaces the targeted\n\
         # row's whole config, so the provider is restated beside the model.\n\
         - id: agent-default-model\n\
         \x20 config:\n\
         \x20   provider: {provider}\n\
         \x20   model: {model}\n"
    );
    io_context(
        file.write_all(body.as_bytes()),
        "could not write the dsh model overlay",
    )?;
    Ok(file)
}

/// A fresh transcript root for one seat. The JSONL backend's `root` has
/// no default and one root belongs to one encoding, so this is also what
/// makes `compression: none` legal without disturbing the operator's
/// zstd-encoded `$DSH_HOME/sessions`.
///
/// The directory lives exactly as long as the invocation does. It is
/// what `brokkr watch` and the tui can follow WHILE the seat works, and
/// the checkpoints folded out of it are what survives: session id, step
/// count, per-step usage, tool names. The raw file is not evidence to
/// keep — it holds the prompt text, tool arguments and tool results that
/// the journal's privacy invariant deliberately excludes, so retaining
/// it would put precisely the excluded prose in an unmanaged directory
/// outside the operator's own `$DSH_HOME` retention, one per seat,
/// forever. It is dropped with the seat instead.
fn dsh_transcript_root() -> Result<tempfile::TempDir, String> {
    dsh_transcript_root_in(|| {
        tempfile::Builder::new()
            .prefix("brokkr-dsh-session-")
            .tempdir()
    })
}

/// The root over an injected directory, so the one way staging can fail
/// is reachable from a test without a full disk — and without a test
/// moving `TMPDIR` out from under every other test in the process.
fn dsh_transcript_root_in(
    create: impl FnOnce() -> std::io::Result<tempfile::TempDir>,
) -> Result<tempfile::TempDir, String> {
    io_context(create(), "could not stage the dsh session transcript root")
}

/// The overlay row that points dsh's session persistence at this seat's
/// own root, in the encoding an external line reader can follow. The
/// path is written as a single-quoted YAML scalar with its quotes
/// doubled, and a path that could open a line of its own is refused
/// rather than written — same discipline as the model id.
///
/// Accepted by the installed 0.1.0-rc.6, not merely by a shim: `dsh
/// --profile headless --dump-config` composes this row over the
/// `session-persistence-jsonl` id that `@deepseek-ai/dsh-base` already
/// contributes (whose only default is `root: dshHomePath('sessions')`),
/// and a real headless session run with it wrote a plain-text
/// `<root>/--<cwd>--/session-<uuid>/session.jsonl` — the layout
/// `find_dsh_transcript` walks and `DSH_TRANSCRIPT` names.
fn dsh_transcript_row(root: &std::path::Path) -> Result<String, String> {
    let root = root.to_string_lossy();
    if root.contains('\n') || root.contains('\r') {
        return Err(format!(
            "dsh driver: transcript root {root:?} spans more than one line"
        ));
    }
    Ok(format!(
        "# Written by `brokkr driver dsh` for one seat: this seat's session\n\
         # transcript, raw and line-readable, under a root only this seat\n\
         # writes — so the driver follows the right session by construction\n\
         # and never by scanning a shared directory for the newest file.\n\
         - id: session-persistence-jsonl\n\
         \x20 config:\n\
         \x20   root: '{}'\n\
         \x20   compression: none\n\
         \x20   packChunks: false\n",
        root.replace('\'', "''")
    ))
}

/// One overlay per seat carrying every row this driver pins: the
/// transcript root always, and the model when one is pinned. It is ONE
/// file because `--patch` is the launcher's only override channel and
/// the seat's argv stays as narrow as it was.
fn dsh_seat_overlay(
    model: Option<&str>,
    root: &std::path::Path,
) -> Result<tempfile::NamedTempFile, String> {
    dsh_seat_overlay_in(model, root, || {
        tempfile::Builder::new()
            .prefix("brokkr-dsh-seat-")
            .suffix(".yml")
            .tempfile()
    })
}

/// The seat overlay over an injected file, so the ways staging can fail
/// are reachable from a test without a full disk.
fn dsh_seat_overlay_in(
    model: Option<&str>,
    root: &std::path::Path,
    create: impl FnOnce() -> std::io::Result<tempfile::NamedTempFile>,
) -> Result<tempfile::NamedTempFile, String> {
    let row = dsh_transcript_row(root)?;
    let mut file = match model {
        Some(model) => dsh_model_overlay_in(model, create)?,
        None => io_context(create(), "could not stage the dsh seat overlay")?,
    };
    io_context(
        file.write_all(row.as_bytes()),
        "could not write the dsh seat overlay",
    )?;
    Ok(file)
}

/// Resolve one exec template part: `{workdir}`, `{prompt_file}`, and
/// each declared `{{secret:NAME}}` → the literal shell env reference
/// `$NAME` — never the value. `$NAME` expands only when the template
/// itself invokes a shell (`bash -c '…'`); env injection is the
/// mechanism that always works, and no `sh -c` wrapping is added (it
/// would change quoting semantics for every existing exec bundle).
fn resolve_exec_part(
    part: &str,
    workdir: &str,
    prompt_path: &str,
    bindings: &[secret::BoundSecret],
) -> String {
    let mut part = part
        .replace("{workdir}", workdir)
        .replace("{prompt_file}", prompt_path);
    for binding in bindings {
        part = part.replace(
            &format!("{{{{secret:{}}}}}", binding.name()),
            &format!("${}", binding.name()),
        );
    }
    part
}

fn stderr_tail_start(stderr: &str) -> usize {
    let mut start = stderr.len().saturating_sub(4000);
    while !stderr.is_char_boundary(start) {
        start -= 1;
    }
    start
}

fn run_seat(kind: AdapterKind, extra: &[String], start: &Value, send: &mut impl FnMut(Body)) {
    let input = start.get("input").cloned().unwrap_or(json!({}));
    let effect_id = start["effect_id"].as_str().unwrap_or("").to_string();
    let attempt_id = start["attempt_id"].as_str().unwrap_or("").to_string();
    send(Body::Accepted {
        effect_id: effect_id.clone(),
        attempt_id: attempt_id.clone(),
        session_ref: None,
    });
    let result_path = input
        .get("result_path")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if let Some(parent) = std::path::Path::new(&result_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Sealed secret bindings (decision 0012): every DECLARED name is
    // resolved before spawn — template references are only the optional
    // argv-side spelling. A missing name (or unreadable store) refuses
    // the attempt determinately, naming the secret and the store path,
    // never an empty-string injection.
    let declared: Vec<String> = input
        .get("secrets")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let bindings = if declared.is_empty() {
        Vec::new()
    } else {
        let store = input
            .get("secrets_file")
            .and_then(Value::as_str)
            .unwrap_or("");
        match secret::resolve_bindings(std::path::Path::new(store), &declared) {
            Ok(bindings) => bindings,
            Err(error) => {
                send(Body::Result {
                    effect_id,
                    attempt_id,
                    status: ResultStatus::Failed,
                    result: None,
                    error: Some(error),
                });
                return;
            }
        }
    };

    let prompt = compose_prompt(&input);
    // Streamed telemetry: each seat-turn the claude arm folds out of
    // stream-json becomes a live protocol checkpoint on this attempt.
    let invocation = match invoke(
        kind,
        extra,
        &prompt,
        &input,
        &bindings,
        &mut |data: &Value| {
            send(Body::Checkpoint {
                effect_id: effect_id.clone(),
                attempt_id: attempt_id.clone(),
                data: data.clone(),
            });
        },
    ) {
        Ok(invocation) => invocation,
        Err(error) => {
            send(Body::Result {
                effect_id,
                attempt_id,
                status: ResultStatus::Failed,
                result: None,
                error: Some(error),
            });
            return;
        }
    };
    let stderr_tail_start = stderr_tail_start(&invocation.stderr);
    eprint!("{}", &invocation.stderr[stderr_tail_start..]);
    let mut checkpoint = Map::new();
    checkpoint.insert(
        "step".into(),
        Value::String(format!("{}-session-finished", kind.driver_name())),
    );
    checkpoint.insert("exit_code".into(), Value::from(invocation.exit_code));
    checkpoint.extend(invocation.session_meta);
    // Ledger-capture marker: a source-literal CONSTANT, never
    // data-derived, inserted AFTER the session_meta extend so
    // last-write-wins guarantees no stream-derived key can ever shadow
    // it. "Priceable in the LaneTally ledger", not "priced" —
    // total_cost_usd above stays the harness-reported list price.
    if kind == AdapterKind::Lanetally {
        checkpoint.insert("capture".into(), Value::String("lanetally".into()));
    }
    send(Body::Checkpoint {
        effect_id: effect_id.clone(),
        attempt_id: attempt_id.clone(),
        data: Value::Object(checkpoint),
    });

    if invocation.exit_code != 0 {
        send(Body::Result {
            effect_id,
            attempt_id,
            status: ResultStatus::Failed,
            result: None,
            error: Some(format!("agent CLI exited {}", invocation.exit_code)),
        });
        return;
    }
    let Ok(raw) = std::fs::read(&result_path) else {
        send(Body::Result {
            effect_id,
            attempt_id,
            status: ResultStatus::Failed,
            result: None,
            error: Some("seat wrote no result file (the result contract was not met)".into()),
        });
        return;
    };
    // Masking choke point, third surface (decision 0012, layer 5): the
    // child-written result payload rides Body::Result into the
    // append-only journal via EffectSucceeded — a child that echoes
    // $TOKEN into its notes must not put plaintext there. Raw bytes
    // first, string conversion second.
    let raw = secret::mask_bytes(&raw, &bindings);
    let raw = String::from_utf8_lossy(&raw);
    // Typed-invalid on purpose when unparseable: the engine parks with
    // raw evidence (decision 0001); adapters repair nothing.
    let seat_result = match serde_json::from_str::<Value>(&raw) {
        Ok(result) => result,
        Err(error) => json!({"__unparseable_result_file__": error.to_string()}),
    };
    send(Body::Result {
        effect_id,
        attempt_id,
        status: ResultStatus::Succeeded,
        result: Some(seat_result),
        error: None,
    });
}

/// The adapter main loop over stdio: hello/capabilities, start→seat,
/// cancel/shutdown. `extra` are the args after `--` in the bundle's
/// driver command.
fn serve_io(
    kind: AdapterKind,
    extra: &[String],
    input: impl BufRead,
    mut output: impl Write,
) -> std::io::Result<()> {
    let mut send = |body: Body| {
        let line = serde_json::to_string(&Message::new(body))
            .expect("the closed protocol message vocabulary serializes");
        let _ = output.write_all(line.as_bytes());
        let _ = output.write_all(b"\n");
        let _ = output.flush();
    };
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue; // the engine speaks the protocol; ignore noise
        };
        match message.get("type").and_then(Value::as_str) {
            Some("hello") => send(Body::Capabilities {
                driver: kind.driver_name(),
                version: ADAPTER_VERSION.to_string(),
                supports: vec![],
            }),
            Some("start") => run_seat(kind, extra, &message, &mut send),
            Some("cancel") => {
                send(Body::Cancelled {
                    effect_id: message
                        .get("effect_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                });
                return Ok(());
            }
            Some("shutdown") => return Ok(()),
            _ => {}
        }
    }
    Ok(())
}

pub fn serve(kind: AdapterKind, extra: Vec<String>) -> std::io::Result<()> {
    serve_io(kind, &extra, std::io::stdin().lock(), std::io::stdout())
}

#[cfg(test)]
mod tests;
