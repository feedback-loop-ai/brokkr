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

    /// What this adapter honours of the protocol's OPTIONAL vocabulary.
    /// Codex is the one built-in that can rejoin the session it opened
    /// (`codex exec resume`, codex-cli 0.148.0); every other arm answers
    /// the empty list it always did, and the engine's offer never
    /// reaches a driver that did not declare it.
    fn supports(&self) -> Vec<String> {
        match self {
            AdapterKind::Codex => vec!["resume".to_string()],
            _ => Vec::new(),
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
                // Journaled NOW, as the claude fold journals its own —
                // and for a second reason here: the thread id is what a
                // retry resumes (decision 0030), and an attempt killed
                // on its deadline never reaches the session-finished
                // checkpoint. Captured at `thread.started`, the id
                // survives exactly the failure a retry follows.
                emit(&json!({
                    "step":"session-started", "session_id": clamped, "harness":"codex",
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

/// The sandbox classes `codex exec -s|--sandbox` takes — read-only,
/// workspace-write, danger-full-access (verified against the installed
/// codex-cli 0.148.0, `codex exec --help`). A resume re-expresses a
/// class this list names, or it does not happen: an unknown spelling is
/// never translated on a guess (decision 0030 ruling 2).
const CODEX_SANDBOX_CLASSES: [&str; 3] = ["read-only", "workspace-write", "danger-full-access"];

/// The thread a launch rejoins and the class re-imposed on it. The two
/// travel together because neither is lawful without the other: a resume
/// without a re-expressed class is the silent escalation decision 0030
/// ruling 2 forbids, and a class without a thread is just a cold spawn.
struct CodexResume {
    thread: String,
    sandbox: String,
}

/// One codex launch, decided before anything spawns: the argv, the
/// session it rejoins (`None` is a cold spawn), and — when a session was
/// on offer and this is a cold spawn anyway — why it could not be taken.
struct CodexLaunch {
    command: Vec<String>,
    resumed: Option<CodexResume>,
    cold_reason: Option<String>,
}

/// The cold argv, exactly as it has always been: `codex exec --json -C
/// <workdir>` plus the seat's own passthrough.
fn codex_cold(bin: &str, extra: &[String], workdir: &str) -> Vec<String> {
    let mut command = vec![
        bin.to_string(),
        "exec".into(),
        "--json".into(),
        "-C".into(),
        workdir.to_string(),
    ];
    command.extend(extra.iter().cloned());
    command
}

/// The seat's declared sandbox class, taken out of its own passthrough.
/// `codex exec resume` accepts neither `-C/--cd` nor `-s/--sandbox`
/// (verified: `codex exec resume --help`, codex-cli 0.148.0), so the
/// class has to leave the argv here and go back in as
/// `-c sandbox_mode="<class>"`. A flag with nothing after it declares
/// nothing, and the resume refuses itself rather than inventing a class.
fn split_codex_sandbox(extra: &[String]) -> (Option<String>, Vec<String>) {
    let mut class = None;
    let mut passthrough = Vec::with_capacity(extra.len());
    let mut parts = extra.iter();
    while let Some(part) = parts.next() {
        if let Some(value) = part.strip_prefix("--sandbox=") {
            class = Some(value.to_string());
        } else if part == "--sandbox" || part == "-s" {
            match parts.next() {
                Some(value) => class = Some(value.clone()),
                None => passthrough.push(part.clone()),
            }
        } else {
            passthrough.push(part.clone());
        }
    }
    (class, passthrough)
}

/// A thread id as codex writes it and the journal displays it: one
/// plain identifier of ASCII alphanumerics and dashes, not leading with
/// one. The id reaches argv positionally, so a spelling that could be
/// read as a flag — or as anything but an id — is refused rather than
/// passed on, and the attempt spawns cold.
fn plain_thread_id(id: &str) -> bool {
    !id.starts_with('-')
        && !id.is_empty()
        && id.len() <= 128
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// How codex is launched for this attempt. A session on offer is taken
/// ONLY when the seat's declared sandbox class can travel with it: the
/// resume drops the class it was opened under (measured — a thread
/// opened `-s read-only` writes files on a bare resume), so a class that
/// cannot be re-expressed is a cold spawn with the reason journaled,
/// never a quiet escalation (decision 0030 ruling 2).
fn codex_launch(bin: &str, extra: &[String], workdir: &str, session: Option<&str>) -> CodexLaunch {
    let cold = |cold_reason: Option<String>| CodexLaunch {
        command: codex_cold(bin, extra, workdir),
        resumed: None,
        cold_reason,
    };
    let Some(session) = session else {
        return cold(None);
    };
    if !plain_thread_id(session) {
        return cold(Some(
            "the offered session id is not a plain thread id".into(),
        ));
    }
    let (class, passthrough) = split_codex_sandbox(extra);
    let Some(class) = class else {
        return cold(Some(
            "the seat declares no sandbox class, and a codex resume does not \
             inherit the class its thread was opened under"
                .into(),
        ));
    };
    if !CODEX_SANDBOX_CLASSES.contains(&class.as_str()) {
        return cold(Some(format!(
            "sandbox class {class:?} is not one codex exec resume can be handed"
        )));
    }
    // A second sandbox expression in the seat's own argv (a `-c
    // sandbox_mode=…` override, a bypass flag) could outrank the one
    // re-imposed here, and last-write-wins is not a thing to gamble a
    // restriction on.
    if passthrough.iter().any(|part| part.contains("sandbox")) {
        return cold(Some(
            "the seat's own argv carries a second sandbox expression, which \
             could outrank the re-imposed class"
                .into(),
        ));
    }
    let mut command = vec![
        bin.to_string(),
        "exec".into(),
        "resume".into(),
        "--json".into(),
        "-c".into(),
        format!("sandbox_mode=\"{class}\""),
    ];
    command.extend(passthrough);
    command.push(session.to_string());
    // The prompt still arrives on stdin, which `codex exec resume` reads
    // only when the prompt positional is `-` (verified against 0.148.0).
    command.push("-".into());
    CodexLaunch {
        command,
        resumed: Some(CodexResume {
            thread: session.to_string(),
            sandbox: class,
        }),
        cold_reason: None,
    }
}

/// The launch checkpoint: what this seat did with the session it was (or
/// was not) offered, in the shape the dsh arm established.
fn codex_started(launch: &CodexLaunch) -> Value {
    let mut checkpoint = Map::new();
    checkpoint.insert("step".into(), Value::String("harness-started".into()));
    checkpoint.insert("harness".into(), Value::String("codex".into()));
    match &launch.resumed {
        Some(resume) => {
            checkpoint.insert("launch".into(), Value::String("resumed".into()));
            checkpoint.insert("session_id".into(), Value::String(resume.thread.clone()));
            checkpoint.insert("sandbox".into(), Value::String(resume.sandbox.clone()));
        }
        None => {
            checkpoint.insert("launch".into(), Value::String("cold".into()));
        }
    }
    if let Some(reason) = &launch.cold_reason {
        checkpoint.insert("reason".into(), Value::String(reason.clone()));
    }
    Value::Object(checkpoint)
}

/// Spawn codex, write the prompt to its stdin, and fold its stable
/// `exec --json` JSONL live. Shared by the cold and resume argvs so the
/// two paths differ in exactly one thing: the command line.
fn invoke_codex(
    command: &[String],
    prompt: &str,
    workdir: &str,
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    let (program, args) = (&command[0], &command[1..]);
    let child = Command::new(program)
        .args(args)
        .current_dir(if workdir.is_empty() { "." } else { workdir })
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

fn invoke(
    kind: AdapterKind,
    extra: &[String],
    prompt: &str,
    input: &Value,
    session: Option<&str>,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    invoke_with_stager(
        kind,
        extra,
        prompt,
        input,
        session,
        bindings,
        emit,
        stage_prompt,
    )
}

#[allow(clippy::too_many_arguments)]
fn invoke_with_stager(
    kind: AdapterKind,
    extra: &[String],
    prompt: &str,
    input: &Value,
    session: Option<&str>,
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
            let launch = codex_launch(&bin, extra, &workdir, session);
            emit(&codex_started(&launch));
            let invocation = invoke_codex(&launch.command, prompt, &workdir, emit)?;
            // A resume codex would not take — an unknown or expired
            // thread — is a cold spawn with the refusal journaled, not an
            // attempt failure (decision 0030 ruling 3). The predicate is
            // structural, never a read of codex's prose: the process
            // ended non-zero having never announced a thread, so no part
            // of the seat's work had begun and starting it cold costs
            // nothing but the cache.
            if launch.resumed.is_some()
                && invocation.exit_code != 0
                && !invocation.session_meta.contains_key("session_id")
            {
                emit(&json!({
                    "step":"harness-started", "harness":"codex", "launch":"cold",
                    "reason":"codex refused the offered thread",
                }));
                return invoke_codex(&codex_cold(&bin, extra, &workdir), prompt, &workdir, emit);
            }
            Ok(invocation)
        }
        AdapterKind::Dsh => {
            let bin = adapter_binary("BROKKR_DSH_BIN", Some("FORGE_DSH_BIN"), "dsh");
            let (model, passthrough) = split_dsh_model(extra)?;
            let overlay = match &model {
                Some(model) => Some(dsh_model_overlay(model)?),
                None => None,
            };
            emit(&json!({
                "step":"harness-started",
                "harness":"deepseek",
                "profile":"headless",
                "model": model,
            }));
            let mut command = vec![bin, "--profile".into(), "headless".into()];
            if let Some(overlay) = &overlay {
                command.push("--patch".into());
                command.push(overlay.path().to_string_lossy().into_owned());
            }
            command.extend(passthrough);
            command.push(prompt.into());
            let out = run_cli(&command, None, &workdir, &[])?;
            let mut session_meta = Map::new();
            session_meta.insert("harness".into(), Value::String("deepseek".into()));
            session_meta.insert("profile".into(), Value::String("headless".into()));
            if let Some(model) = model {
                session_meta.insert("model".into(), Value::String(model));
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

/// One overlay file for one seat, in the loader-patch grammar dsh
/// composes after every bundle and profile layer. The id is written
/// into YAML verbatim, so it is confined to the characters a model id
/// is made of — a model name is data the operator pinned, never a
/// place to smuggle a second row into the tree.
fn dsh_model_overlay(model: &str) -> Result<tempfile::NamedTempFile, String> {
    dsh_model_overlay_in(model, || {
        tempfile::Builder::new()
            .prefix("brokkr-dsh-model-")
            .suffix(".yml")
            .tempfile()
    })
}

/// The overlay's body over an injected file, so the two ways staging
/// can fail — no file, a file that takes no bytes — are reachable from
/// a test without a full disk.
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

/// `session` is the prior session the engine handed back for this seat
/// (decision 0030), or `None` for the cold start every attempt was
/// before it. Only the codex arm knows what to do with one; the rest
/// ignore it exactly as they ignored the message that carried it.
fn run_seat(
    kind: AdapterKind,
    extra: &[String],
    start: &Value,
    session: Option<&str>,
    send: &mut impl FnMut(Body),
) {
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
        session,
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

/// The adapter main loop over stdio: hello/capabilities, an optional
/// resume offer, start→seat, cancel/shutdown. `extra` are the args after
/// `--` in the bundle's driver command.
///
/// `resume` arrives BEFORE the `start` it belongs to and carries no seat
/// and no input — it is the session handle for the attempt the next
/// `start` describes, and nothing else. It is consumed by that one seat:
/// a second start with no resume in front of it is a cold start.
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
    let mut offered: Option<String> = None;
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
                supports: kind.supports(),
            }),
            Some("resume") => {
                offered = message
                    .get("session_ref")
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
            Some("start") => run_seat(kind, extra, &message, offered.take().as_deref(), &mut send),
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
