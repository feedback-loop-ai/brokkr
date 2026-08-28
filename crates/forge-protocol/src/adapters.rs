//! Built-in driver adapters: `forge driver <claude|codex|dsh|exec>`.
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
//! FORGE_CODEX_BIN, FORGE_DSH_BIN, FORGE_EXEC_NAME.

use std::io::{BufRead, Read, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Map, Value};

use crate::secret;
use crate::{Body, Message, ResultStatus};

const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    Claude,
    Codex,
    Dsh,
    Exec,
}

impl AdapterKind {
    pub fn parse(name: &str) -> Option<AdapterKind> {
        match name {
            "claude" => Some(AdapterKind::Claude),
            "codex" => Some(AdapterKind::Codex),
            "dsh" => Some(AdapterKind::Dsh),
            "exec" => Some(AdapterKind::Exec),
            _ => None,
        }
    }

    fn driver_name(&self) -> String {
        match self {
            AdapterKind::Claude => "claude-code".to_string(),
            AdapterKind::Codex => "codex".to_string(),
            AdapterKind::Dsh => "deepseek-harness".to_string(),
            AdapterKind::Exec => {
                std::env::var("FORGE_EXEC_NAME").unwrap_or_else(|_| "exec".to_string())
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
        let value = std::str::from_utf8(binding.secret().expose_for_spawn())
            .map_err(|_| format!("secret '{}' is not valid UTF-8", binding.name()))?;
        invocation.env(binding.name(), value);
    }
    let mut child = invocation
        .spawn()
        .map_err(|e| format!("could not invoke the agent CLI: {e}"))?;
    if let Some(payload) = stdin_payload {
        let mut stdin = child.stdin.take().expect("piped");
        stdin
            .write_all(payload.as_bytes())
            .map_err(|e| format!("could not write the prompt: {e}"))?;
    } else {
        drop(child.stdin.take());
    }
    child
        .wait_with_output()
        .map_err(|e| format!("agent CLI did not conclude: {e}"))
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
            if let Some(session_id) = event.get("session_id") {
                session_meta.insert("session_id".into(), session_id.clone());
            }
        }
        Some("assistant") => {
            *assistant_turns += 1;
            let blocks = event
                .pointer("/message/content")
                .and_then(Value::as_array);
            let tool_uses = blocks.into_iter().flatten().filter(|b| {
                b.get("type").and_then(Value::as_str) == Some("tool_use")
            });
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
                let target = tool_use
                    .pointer("/input/file_path")
                    .and_then(Value::as_str);
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

fn invoke(
    kind: AdapterKind,
    extra: &[String],
    prompt: &str,
    input: &Value,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    let workdir = input
        .get("workdir")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    match kind {
        AdapterKind::Claude => {
            let bin =
                std::env::var("FORGE_CLAUDE_BIN").unwrap_or_else(|_| "claude".to_string());
            let mut command = vec![
                bin,
                "-p".into(),
                "--output-format".into(),
                "stream-json".into(),
                "--verbose".into(),
            ];
            command.extend(extra.iter().cloned());
            let (program, args) = command
                .split_first()
                .ok_or_else(|| "empty command".to_string())?;
            let mut child = Command::new(program)
                .args(args)
                .current_dir(if workdir.is_empty() { "." } else { &workdir })
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("could not invoke the agent CLI: {e}"))?;
            {
                let mut stdin = child.stdin.take().expect("piped");
                stdin
                    .write_all(prompt.as_bytes())
                    .map_err(|e| format!("could not write the prompt: {e}"))?;
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
            let status = child
                .wait()
                .map_err(|e| format!("agent CLI did not conclude: {e}"))?;
            Ok(Invocation {
                exit_code: status.code().unwrap_or(-1),
                session_meta,
                stderr: stderr_thread.join().unwrap_or_default(),
            })
        }
        AdapterKind::Codex => {
            let bin = std::env::var("FORGE_CODEX_BIN").unwrap_or_else(|_| "codex".to_string());
            let mut command = vec![
                bin,
                "exec".into(),
                "--json".into(),
                "-C".into(),
                workdir.clone(),
            ];
            command.extend(extra.iter().cloned());
            let (program, args) = command
                .split_first()
                .ok_or_else(|| "empty command".to_string())?;
            let mut child = Command::new(program)
                .args(args)
                .current_dir(if workdir.is_empty() { "." } else { &workdir })
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("could not invoke the agent CLI: {e}"))?;
            child
                .stdin
                .take()
                .expect("piped")
                .write_all(prompt.as_bytes())
                .map_err(|e| format!("could not write the prompt: {e}"))?;
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
            let status = child
                .wait()
                .map_err(|e| format!("agent CLI did not conclude: {e}"))?;
            Ok(Invocation {
                exit_code: status.code().unwrap_or(-1),
                session_meta,
                stderr: stderr_thread.join().unwrap_or_default(),
            })
        }
        AdapterKind::Dsh => {
            let bin = std::env::var("FORGE_DSH_BIN").unwrap_or_else(|_| "dsh".to_string());
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
                let mut file = tempfile::Builder::new()
                    .prefix("forge-prompt-")
                    .suffix(".md")
                    .tempfile()
                    .map_err(|e| format!("could not stage the prompt: {e}"))?;
                file.write_all(prompt.as_bytes())
                    .map_err(|e| format!("could not stage the prompt: {e}"))?;
                prompt_file = Some(file);
            }
            let prompt_path = prompt_file
                .as_ref()
                .map(|f| f.path().to_string_lossy().into_owned())
                .unwrap_or_default();
            let command: Vec<String> = extra
                .iter()
                .map(|part| resolve_exec_part(part, &workdir, &prompt_path, bindings))
                .collect();
            let stdin_payload = if prompt_file.is_none() { Some(prompt) } else { None };
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

fn run_seat(kind: AdapterKind, extra: &[String], start: &Value, send: &mut impl FnMut(Body)) {
    let input = start.get("input").cloned().unwrap_or_else(|| json!({}));
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
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
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
    let invocation = match invoke(kind, extra, &prompt, &input, &bindings, &mut |data: &Value| {
        send(Body::Checkpoint {
            effect_id: effect_id.clone(),
            attempt_id: attempt_id.clone(),
            data: data.clone(),
        });
    }) {
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
    let stderr_tail_start = {
        let mut start = invocation.stderr.len().saturating_sub(4000);
        while !invocation.stderr.is_char_boundary(start) {
            start -= 1;
        }
        start
    };
    eprint!("{}", &invocation.stderr[stderr_tail_start..]);
    let mut checkpoint = Map::new();
    checkpoint.insert(
        "step".into(),
        Value::String(format!("{}-session-finished", kind.driver_name())),
    );
    checkpoint.insert("exit_code".into(), Value::from(invocation.exit_code));
    checkpoint.extend(invocation.session_meta);
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
    let seat_result = serde_json::from_str::<Value>(&raw)
        .unwrap_or_else(|e| json!({"__unparseable_result_file__": e.to_string()}));
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
pub fn serve(kind: AdapterKind, extra: Vec<String>) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut send = |body: Body| {
        if let Ok(line) = serde_json::to_string(&Message::new(body)) {
            let _ = stdout.write_all(line.as_bytes());
            let _ = stdout.write_all(b"\n");
            let _ = stdout.flush();
        }
    };
    for line in stdin.lock().lines() {
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
            Some("start") => run_seat(kind, &extra, &message, &mut send),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(name: &str, value: &str) -> secret::BoundSecret {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("secrets.env");
        secret::store_set(&store, name, value).unwrap();
        secret::resolve_bindings(&store, &[name.to_string()])
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
    }

    #[test]
    fn exec_template_resolves_secret_refs_to_env_references_never_values() {
        let bindings = vec![binding("GH_TOKEN", "tok3n+v4lue!")];
        let resolved = resolve_exec_part(
            "curl -H 'auth: {{secret:GH_TOKEN}}' {workdir}/x",
            "/w",
            "",
            &bindings,
        );
        assert_eq!(resolved, "curl -H 'auth: $GH_TOKEN' /w/x");
        assert!(!resolved.contains("tok3n"), "the value never enters argv text");
    }

    #[test]
    fn exec_template_leaves_undeclared_refs_untouched() {
        // Compile refuses these in real bundles; standalone driver use
        // must still never invent a resolution.
        let resolved = resolve_exec_part("{{secret:OTHER}}", "/w", "", &[]);
        assert_eq!(resolved, "{{secret:OTHER}}");
    }

    #[test]
    fn claude_fold_journals_file_paths_only_and_bash_stays_targetless() {
        // The 0012 amendment leaves the claude fold untouched: a Bash
        // tool_use (model-authored command) journals NO target; only
        // input.file_path ever becomes one.
        let mut turns = 0;
        let mut meta = Map::new();
        let mut emitted: Vec<Value> = Vec::new();
        let event = json!({"type": "assistant", "message": {"content": [
            {"type": "tool_use", "name": "Bash",
             "input": {"command": "curl -H 'auth: hunter22' https://x"}},
            {"type": "tool_use", "name": "Edit",
             "input": {"file_path": "src/lib.rs"}},
        ]}});
        fold_stream_event(&event, &mut turns, &mut meta, &mut |c| emitted.push(c.clone()));
        assert_eq!(emitted.len(), 2);
        assert_eq!(emitted[0]["tool"], "Bash");
        assert!(emitted[0].get("target").is_none(), "{}", emitted[0]);
        assert!(
            !serde_json::to_string(&emitted[0]).unwrap().contains("hunter22"),
            "the command text never reaches the checkpoint"
        );
        assert_eq!(emitted[1]["target"], "src/lib.rs");
    }
}
