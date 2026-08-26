//! Built-in driver adapters: `forge driver <claude|codex|exec>`.
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
//! FORGE_CODEX_BIN, FORGE_EXEC_NAME.

use std::io::{BufRead, Read, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Map, Value};

use crate::{Body, Message, ResultStatus};

const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    Claude,
    Codex,
    Exec,
}

impl AdapterKind {
    pub fn parse(name: &str) -> Option<AdapterKind> {
        match name {
            "claude" => Some(AdapterKind::Claude),
            "codex" => Some(AdapterKind::Codex),
            "exec" => Some(AdapterKind::Exec),
            _ => None,
        }
    }

    fn driver_name(&self) -> String {
        match self {
            AdapterKind::Claude => "claude-code".to_string(),
            AdapterKind::Codex => "codex".to_string(),
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
) -> Result<std::process::Output, String> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| "empty command".to_string())?;
    let mut child = Command::new(program)
        .args(args)
        .current_dir(if workdir.is_empty() { "." } else { workdir })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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
/// `system`/`init` and `result` feed `session_meta`; an `assistant`
/// message with a `tool_use` block becomes one seat-turn checkpoint.
/// Privacy invariant (journal is evidence, not transcript): checkpoints
/// carry turn index, tool name, and a ≤80-char target only — never
/// message text, thinking, or full tool inputs.
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
            let Some(tool_use) = blocks.and_then(|blocks| {
                blocks
                    .iter()
                    .find(|b| b.get("type").and_then(Value::as_str) == Some("tool_use"))
            }) else {
                return;
            };
            let mut checkpoint = Map::new();
            checkpoint.insert("step".into(), Value::String("seat-turn".into()));
            checkpoint.insert("turn".into(), Value::from(*assistant_turns));
            checkpoint.insert(
                "tool".into(),
                tool_use.get("name").cloned().unwrap_or(Value::String(String::new())),
            );
            let target = ["file_path", "command", "url"].iter().find_map(|key| {
                tool_use
                    .pointer(&format!("/input/{key}"))
                    .and_then(Value::as_str)
            });
            if let Some(target) = target {
                checkpoint.insert(
                    "target".into(),
                    Value::String(target.chars().take(80).collect()),
                );
            }
            emit(&Value::Object(checkpoint));
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

fn invoke(
    kind: AdapterKind,
    extra: &[String],
    prompt: &str,
    input: &Value,
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
                let mut captured = String::new();
                let mut pipe = stderr_pipe;
                let _ = pipe.read_to_string(&mut captured);
                captured
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
            let mut command = vec![bin, "exec".into(), "-C".into(), workdir.clone()];
            command.extend(extra.iter().cloned());
            let out = run_cli(&command, Some(prompt), &workdir)?;
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut session_meta = Map::new();
            for line in stdout.lines() {
                let lower = line.to_ascii_lowercase();
                if let Some(idx) = lower.find("session id") {
                    let tail: String = line[idx..]
                        .chars()
                        .skip_while(|c| !c.is_ascii_hexdigit())
                        .take_while(|c| c.is_ascii_hexdigit() || *c == '-')
                        .collect();
                    if tail.len() >= 8 {
                        session_meta.insert("session_id".into(), Value::String(tail));
                        break;
                    }
                }
            }
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
                .map(|part| {
                    part.replace("{workdir}", &workdir)
                        .replace("{prompt_file}", &prompt_path)
                })
                .collect();
            let stdin_payload = if prompt_file.is_none() { Some(prompt) } else { None };
            let out = run_cli(&command, stdin_payload, &workdir)?;
            Ok(Invocation {
                exit_code: out.status.code().unwrap_or(-1),
                session_meta: Map::new(),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            })
        }
    }
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

    let prompt = compose_prompt(&input);
    // Streamed telemetry: each seat-turn the claude arm folds out of
    // stream-json becomes a live protocol checkpoint on this attempt.
    let invocation = match invoke(kind, extra, &prompt, &input, &mut |data: &Value| {
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
    eprint!(
        "{}",
        &invocation.stderr[invocation.stderr.len().saturating_sub(4000)..]
    );
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
    let Ok(raw) = std::fs::read_to_string(&result_path) else {
        send(Body::Result {
            effect_id,
            attempt_id,
            status: ResultStatus::Failed,
            result: None,
            error: Some("seat wrote no result file (the result contract was not met)".into()),
        });
        return;
    };
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
