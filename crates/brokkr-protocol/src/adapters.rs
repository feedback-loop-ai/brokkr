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
                session_meta.insert(
                    "session_id".into(),
                    Value::String(thread_id.chars().take(128).collect()),
                );
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
                    session_meta.insert(target.into(), Value::from(value));
                }
            }
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
        AdapterKind::Dsh => {
            let bin = adapter_binary("BROKKR_DSH_BIN", Some("FORGE_DSH_BIN"), "dsh");
            emit(&json!({
                "step":"harness-started",
                "harness":"deepseek",
                "profile":"headless",
            }));
            let mut command = vec![bin, "--profile".into(), "headless".into()];
            command.extend(extra.iter().cloned());
            command.push(prompt.into());
            let out = run_cli(&command, None, &workdir, &[])?;
            let mut session_meta = Map::new();
            session_meta.insert("harness".into(), Value::String("deepseek".into()));
            session_meta.insert("profile".into(), Value::String("headless".into()));
            Ok(Invocation {
                exit_code: out.status.code().unwrap_or(-1),
                session_meta,
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            })
        }
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
