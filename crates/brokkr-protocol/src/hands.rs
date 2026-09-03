//! The model's hands are one tool, and the tool runs in an empty root
//! (decision 0040).
//!
//! A harness keeps its credential and its network to the provider
//! OUTSIDE the box. What the model asks to RUN goes through one tool,
//! `workspace`, served over MCP on stdio by `brokkr hands serve`, and
//! every call executes `bash -lc <command>` inside a bubblewrap
//! namespace built from an empty root: the worktree bound read-write at
//! its own path, the host toolchain read-only, a private `HOME` and
//! `/tmp`, no host home, no host credential, no other process, and no
//! network unless the spec grants it. A tool allow-list bounded what the
//! model may run; the box bounds what running anything can touch.
//!
//! The same namespace boxes a deterministic `exec` seat whole, which is
//! what lets a pinned script hold a gate (ruling 3).
//!
//! Linux only, like the boundary it builds: `bwrap` is refused when
//! absent, never simulated.

use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

/// The one tool the model sees. Claude Code names it `mcp__brokkr__workspace`.
pub const SERVER_NAME: &str = "brokkr";
pub const TOOL_NAME: &str = "workspace";
/// The workdir is bound at its own path, so the paths a prompt names are
/// the paths the command sees.
const SANDBOX_HOME: &str = "/runtime/home";
const SANDBOX_PATH: &str = "/runtime:/usr/local/bin:/usr/bin:/bin";
const OUTPUT_BYTES: usize = 262_144;
const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const MAX_TIMEOUT_MS: u64 = 600_000;

/// One extra bind into the box. `path` may start with `~/`, resolved
/// against the host home at spawn; `mask` names files UNDER the bind
/// that are hidden behind `/dev/null` — a credentials file inside a
/// toolchain directory that must otherwise be writable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bind {
    pub path: String,
    pub writable: bool,
    pub mask: Vec<String>,
}

/// What a site declared under `hands`. The string `"workspace"` is the
/// default spec; an object refines it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HandsSpec {
    pub network: bool,
    pub binds: Vec<Bind>,
}

impl HandsSpec {
    /// Parse a site's `hands` value. Closed vocabulary: the kind is
    /// `workspace` and nothing else; unknown keys are refused, because a
    /// dropped key here would be a dropped boundary.
    pub fn parse(raw: &Value) -> Result<HandsSpec, String> {
        let object = match raw {
            Value::String(kind) if kind == "workspace" => return Ok(HandsSpec::default()),
            Value::String(other) => {
                return Err(format!(
                    "hands '{other}' is not a known kind; the vocabulary is: workspace"
                ))
            }
            Value::Object(object) => object,
            other => {
                return Err(format!(
                    "hands must be \"workspace\" or an object, got {other}"
                ))
            }
        };
        for key in object.keys() {
            if !["kind", "network", "binds"].contains(&key.as_str()) {
                return Err(format!(
                    "hands has unknown key '{key}'; known: kind, network, binds"
                ));
            }
        }
        match object.get("kind").and_then(Value::as_str) {
            Some("workspace") => {}
            _ => return Err("hands.kind must be \"workspace\"".to_string()),
        }
        let network = match object.get("network") {
            None => false,
            Some(Value::Bool(flag)) => *flag,
            Some(other) => return Err(format!("hands.network must be a boolean, got {other}")),
        };
        let mut binds = Vec::new();
        if let Some(raw_binds) = object.get("binds") {
            let entries = raw_binds
                .as_array()
                .ok_or_else(|| "hands.binds must be an array".to_string())?;
            for entry in entries {
                binds.push(Bind::parse(entry)?);
            }
        }
        Ok(HandsSpec { network, binds })
    }

    /// The spec as data again: what the manifest pins and what
    /// `brokkr hands serve --spec` is handed.
    pub fn to_value(&self) -> Value {
        json!({
            "kind": "workspace",
            "network": self.network,
            "binds": self.binds.iter().map(|bind| json!({
                "path": bind.path,
                "mode": if bind.writable { "rw" } else { "ro" },
                "mask": bind.mask,
            })).collect::<Vec<_>>(),
        })
    }
}

impl Bind {
    fn parse(raw: &Value) -> Result<Bind, String> {
        let object = raw
            .as_object()
            .ok_or_else(|| "hands.binds entry must be an object".to_string())?;
        for key in object.keys() {
            if !["path", "mode", "mask"].contains(&key.as_str()) {
                return Err(format!(
                    "hands.binds entry has unknown key '{key}'; known: path, mode, mask"
                ));
            }
        }
        let path = object
            .get("path")
            .and_then(Value::as_str)
            .filter(|path| !path.is_empty())
            .ok_or_else(|| "hands.binds entry needs a non-empty 'path'".to_string())?
            .to_string();
        let writable = match object.get("mode").and_then(Value::as_str) {
            Some("ro") => false,
            Some("rw") => true,
            _ => {
                return Err(format!(
                    "hands.binds entry '{path}' needs mode \"ro\" or \"rw\""
                ))
            }
        };
        let mut mask = Vec::new();
        if let Some(raw_mask) = object.get("mask") {
            let names = raw_mask
                .as_array()
                .ok_or_else(|| format!("hands.binds entry '{path}' mask must be an array"))?;
            for name in names {
                let name = name
                    .as_str()
                    .filter(|name| !name.is_empty() && !name.contains('/'))
                    .ok_or_else(|| {
                        format!(
                            "hands.binds entry '{path}' mask names one file each, \
                             without a slash"
                        )
                    })?;
                mask.push(name.to_string());
            }
        }
        Ok(Bind {
            path,
            writable,
            mask,
        })
    }
}

/// `~/x` against the host home; anything else as written.
fn expand_home(path: &str, home: &Path) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None => PathBuf::from(path),
    }
}

/// The bubblewrap argv for one boxed command: the namespace, the binds,
/// the environment, then `--` and the command. `scratch` holds the
/// generated identity files and the private home and tmp for this one
/// call; the caller removes it after the process exits.
pub fn box_argv(
    spec: &HandsSpec,
    workdir: &Path,
    home: &Path,
    scratch: &Path,
    command: &[String],
) -> std::io::Result<Vec<String>> {
    let etc = scratch.join("etc");
    let private_home = scratch.join("home");
    let private_tmp = scratch.join("tmp");
    std::fs::create_dir_all(&etc)?;
    std::fs::create_dir_all(&private_home)?;
    std::fs::create_dir_all(&private_tmp)?;
    // A deterministic identity and a files-only resolver: `localhost`
    // resolves inside a no-network namespace without exposing the host's
    // resolver or its network namespace.
    let (uid, gid) = ids();
    let passwd = format!("runner:x:{uid}:{gid}:brokkr hands:{SANDBOX_HOME}:/bin/sh\n");
    let group = format!("runner:x:{gid}:\n");
    let hosts = "127.0.0.1 localhost\n::1 localhost ip6-localhost ip6-loopback\n";
    for (name, text) in [
        ("passwd", passwd.as_str()),
        ("group", group.as_str()),
        ("hosts", hosts),
        ("nsswitch.conf", "hosts: files\n"),
    ] {
        std::fs::write(etc.join(name), text)?;
    }

    let s = |text: &str| text.to_string();
    let mut argv = vec![
        s("bwrap"),
        s("--die-with-parent"),
        s("--new-session"),
        s("--unshare-pid"),
        s("--unshare-ipc"),
        s("--unshare-uts"),
        s("--unshare-cgroup-try"),
        s("--cap-drop"),
        s("ALL"),
    ];
    if !spec.network {
        argv.push(s("--unshare-net"));
    }
    argv.extend([
        s("--clearenv"),
        s("--proc"),
        s("/proc"),
        s("--dev"),
        s("/dev"),
        s("--dir"),
        s("/runtime"),
        s("--dir"),
        s("/etc"),
        s("--dir"),
        s("/home"),
        s("--dir"),
        s("/root"),
        s("--dir"),
        s("/run"),
        s("--dir"),
        s("/usr"),
    ]);
    // The host toolchain, read-only, where it exists (`-try`: an absent
    // source is skipped, never an error — /lib64 is a Debian fact, not
    // a law).
    for host in [
        "/usr/bin",
        "/usr/lib",
        "/usr/lib64",
        "/usr/share",
        "/usr/local",
        "/usr/libexec",
        "/bin",
        "/sbin",
        "/lib",
        "/lib64",
        "/etc/ssl",
        "/etc/ca-certificates",
        "/etc/alternatives",
        "/etc/ld.so.cache",
        "/etc/ld.so.conf",
        "/etc/ld.so.conf.d",
    ] {
        argv.extend([s("--ro-bind-try"), s(host), s(host)]);
    }
    let text = |path: &Path| path.to_string_lossy().into_owned();
    for (name, target) in [
        ("passwd", "/etc/passwd"),
        ("group", "/etc/group"),
        ("hosts", "/etc/hosts"),
        ("nsswitch.conf", "/etc/nsswitch.conf"),
    ] {
        argv.extend([s("--ro-bind"), text(&etc.join(name)), s(target)]);
    }
    // The private home and tmp go in BEFORE the worktree, so a worktree
    // that itself lives under /tmp is mounted on top of the private /tmp
    // rather than hidden beneath it.
    argv.extend([
        s("--bind"),
        text(&private_home),
        s(SANDBOX_HOME),
        s("--bind"),
        text(&private_tmp),
        s("/tmp"),
        s("--bind"),
        text(workdir),
        text(workdir),
    ]);
    for bind in &spec.binds {
        let host = expand_home(&bind.path, home);
        let flag = if bind.writable {
            "--bind-try"
        } else {
            "--ro-bind-try"
        };
        argv.extend([s(flag), text(&host), text(&host)]);
        for name in &bind.mask {
            let masked = host.join(name);
            // A mask over a file that is not there would make bwrap
            // create it on the host to mount over; only what exists is
            // hidden.
            if masked.exists() {
                argv.extend([s("--ro-bind"), s("/dev/null"), text(&masked)]);
            }
        }
    }
    for (key, value) in [
        ("HOME", SANDBOX_HOME),
        ("USER", "runner"),
        ("LOGNAME", "runner"),
        ("TMPDIR", "/tmp"),
        ("PATH", SANDBOX_PATH),
        ("LANG", "C.UTF-8"),
        ("LC_ALL", "C.UTF-8"),
        ("CI", "true"),
        ("DISABLE_AUTOUPDATER", "1"),
        ("DISABLE_TELEMETRY", "1"),
    ] {
        argv.extend([s("--setenv"), s(key), s(value)]);
    }
    argv.extend([s("--chdir"), text(workdir), s("--")]);
    argv.extend(command.iter().cloned());
    Ok(argv)
}

#[cfg(unix)]
fn ids() -> (u32, u32) {
    // SAFETY: getuid/getgid take no arguments, read process credentials
    // and cannot fail.
    unsafe { (libc_getuid(), libc_getgid()) }
}

#[cfg(not(unix))]
fn ids() -> (u32, u32) {
    (65_534, 65_534)
}

#[cfg(unix)]
extern "C" {
    #[link_name = "getuid"]
    fn libc_getuid() -> u32;
    #[link_name = "getgid"]
    fn libc_getgid() -> u32;
}

/// The bwrap binary, or a refusal: the boundary is never simulated.
pub fn require_bwrap() -> Result<PathBuf, String> {
    bwrap_on(&std::env::var_os("PATH").unwrap_or_default())
}

/// `bwrap` on a given search path — the testable half of the refusal.
pub fn bwrap_on(path: &std::ffi::OsStr) -> Result<PathBuf, String> {
    match std::env::split_paths(path)
        .map(|dir| dir.join("bwrap"))
        .find(|candidate| candidate.is_file())
    {
        Some(found) => Ok(found),
        None => Err(
            "hands need bubblewrap: no `bwrap` on PATH, and the boundary is never simulated"
                .to_string(),
        ),
    }
}

/// One I/O outcome named by what it was doing — the single shape every
/// spawn-side error takes, so each site is a call and not a closure.
pub fn io_context<T>(result: std::io::Result<T>, doing: &str) -> Result<T, String> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => Err(format!("hands {doing}: {error}")),
    }
}

/// The executor the server calls per tool invocation: production hands
/// it [`execute`]; a test hands it a stub.
pub type Executor = dyn Fn(&HandsSpec, &Path, &str, Duration) -> Result<Executed, String>;

/// One executed command, as the tool reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Executed {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

impl Executed {
    /// The rendering the model reads: stdout, then labelled stderr, then
    /// the verdict lines.
    pub fn render(&self) -> String {
        let mut sections = Vec::new();
        if !self.stdout.is_empty() {
            sections.push(self.stdout.clone());
        }
        if !self.stderr.is_empty() {
            sections.push(format!("[stderr]\n{}", self.stderr));
        }
        if self.timed_out {
            sections.push("[timed out]".to_string());
        }
        sections.push(format!("[exit code: {}]", self.exit_code));
        sections.join("\n")
    }
}

fn bounded(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    if text.len() <= OUTPUT_BYTES {
        return text.into_owned();
    }
    let mut cut = OUTPUT_BYTES;
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    format!(
        "{}\n[output truncated at {OUTPUT_BYTES} bytes]",
        &text[..cut]
    )
}

/// Run one `bash -lc <command>` inside the box, bounded in time and
/// output. The scratch directory lives beside the workdir's own
/// `.forge` state and is removed afterwards.
pub fn execute(
    spec: &HandsSpec,
    workdir: &Path,
    command: &str,
    timeout: Duration,
) -> Result<Executed, String> {
    let bwrap = require_bwrap()?;
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let scratch = std::env::temp_dir().join(format!(
        "brokkr-hands-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    io_context(std::fs::create_dir_all(&scratch), "scratch")?;
    let result = execute_in(&bwrap, spec, workdir, &home, &scratch, command, timeout);
    let _ = std::fs::remove_dir_all(&scratch);
    result
}

pub fn execute_in(
    bwrap: &Path,
    spec: &HandsSpec,
    workdir: &Path,
    home: &Path,
    scratch: &Path,
    command: &str,
    timeout: Duration,
) -> Result<Executed, String> {
    let inner = vec![
        "/bin/bash".to_string(),
        "-lc".to_string(),
        command.to_string(),
    ];
    let argv = io_context(box_argv(spec, workdir, home, scratch, &inner), "namespace")?;
    let spawned = Command::new(bwrap)
        .args(&argv[1..])
        .current_dir("/")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = io_context(spawned, "could not spawn bwrap")?;
    let mut stdout_pipe = child.stdout.take().expect("piped");
    let mut stderr_pipe = child.stderr.take().expect("piped");
    let stdout_thread = std::thread::spawn(move || {
        let mut captured = Vec::new();
        let _ = stdout_pipe.read_to_end(&mut captured);
        captured
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut captured = Vec::new();
        let _ = stderr_pipe.read_to_end(&mut captured);
        captured
    });
    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        let waited = io_context(child.try_wait(), "wait")?;
        if let Some(status) = waited {
            break status;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            let _ = child.kill();
            let reaped = io_context(child.wait(), "wait after kill");
            break reaped?;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let stdout = stdout_thread.join().unwrap_or_default();
    let stderr = stderr_thread.join().unwrap_or_default();
    Ok(Executed {
        stdout: bounded(&stdout),
        stderr: bounded(&stderr),
        exit_code: status.code().unwrap_or(if timed_out { 124 } else { -1 }),
        timed_out,
    })
}

/// Run a whole command inside the box with its stdio passed through —
/// how a deterministic `exec` seat holds a gate (ruling 3). This very
/// binary is bound read-only so the command may be a `brokkr driver …`
/// dispatch. Returns the child's exit code.
pub fn run_boxed(spec: &HandsSpec, workdir: &Path, command: &[String]) -> Result<i32, String> {
    let bwrap = require_bwrap()?;
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let scratch = std::env::temp_dir().join(format!(
        "brokkr-hands-exec-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    io_context(std::fs::create_dir_all(&scratch), "scratch")?;
    let result = run_boxed_in(&bwrap, spec, workdir, &home, &scratch, command);
    let _ = std::fs::remove_dir_all(&scratch);
    result
}

pub fn run_boxed_in(
    bwrap: &Path,
    spec: &HandsSpec,
    workdir: &Path,
    home: &Path,
    scratch: &Path,
    command: &[String],
) -> Result<i32, String> {
    let mut with_self = spec.clone();
    with_self
        .binds
        .extend(std::env::current_exe().ok().map(|exe| Bind {
            path: exe.to_string_lossy().into_owned(),
            writable: false,
            mask: Vec::new(),
        }));
    let built = box_argv(&with_self, workdir, home, scratch, command);
    let argv = io_context(built, "namespace")?;
    let ran = Command::new(bwrap)
        .args(&argv[1..])
        .current_dir("/")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .status();
    let status = io_context(ran, "could not spawn bwrap")?;
    Ok(status.code().unwrap_or(-1))
}

/// The tool as MCP lists it.
pub fn tool_definition() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": format!(
            "Run a bash command in the worktree. The command can read and write \
             the worktree, but cannot see host files outside it, host credentials, \
             host processes or the host HOME. Output is bounded to {OUTPUT_BYTES} \
             bytes; timeoutMs defaults to {DEFAULT_TIMEOUT_MS} and may not exceed \
             {MAX_TIMEOUT_MS}."
        ),
        "inputSchema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["command"],
            "properties": {
                "command": {"type": "string", "minLength": 1, "maxLength": 16384},
                "timeoutMs": {"type": "integer", "minimum": 100, "maximum": MAX_TIMEOUT_MS}
            }
        }
    })
}

/// The MCP config a harness is handed to reach this server: one entry,
/// `brokkr`, running this very binary.
pub fn mcp_config(brokkr: &Path, workdir: &Path, spec: &HandsSpec) -> Value {
    json!({
        "mcpServers": {
            SERVER_NAME: {
                "command": brokkr.to_string_lossy(),
                "args": serve_args(workdir, spec),
            }
        }
    })
}

/// The `brokkr hands serve …` argument vector a harness spawns.
pub fn serve_args(workdir: &Path, spec: &HandsSpec) -> Vec<String> {
    vec![
        "hands".to_string(),
        "serve".to_string(),
        "--workdir".to_string(),
        workdir.to_string_lossy().into_owned(),
        "--spec".to_string(),
        spec.to_value().to_string(),
    ]
}

/// One JSON-RPC exchange of the MCP stdio transport: newline-delimited
/// messages, a response for every request, silence for a notification.
/// `run` is the executor, injected so the loop is testable without a
/// namespace; production hands it [`execute`].
pub fn serve<R: BufRead, W: Write>(
    input: R,
    mut output: W,
    workdir: &Path,
    spec: &HandsSpec,
    run: &Executor,
) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Some(reply) = handle(&line, workdir, spec, run) else {
            continue;
        };
        output.write_all(reply.to_string().as_bytes())?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
    Ok(())
}

fn rpc_error(id: Value, code: i64, message: String) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// One message in, at most one message out.
pub fn handle(line: &str, workdir: &Path, spec: &HandsSpec, run: &Executor) -> Option<Value> {
    let message: Value = match serde_json::from_str(line) {
        Ok(message) => message,
        Err(error) => {
            return Some(rpc_error(
                Value::Null,
                -32700,
                format!("parse error: {error}"),
            ))
        }
    };
    let id = message.get("id").cloned();
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let Some(id) = id else {
        // A notification: acknowledged by silence, whatever it says.
        return None;
    };
    Some(match method {
        "initialize" => rpc_result(
            id,
            json!({
                "protocolVersion": params
                    .get("protocolVersion")
                    .and_then(Value::as_str)
                    .unwrap_or("2025-06-18"),
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "brokkr-hands", "version": env!("CARGO_PKG_VERSION")},
            }),
        ),
        "ping" => rpc_result(id, json!({})),
        "tools/list" => rpc_result(id, json!({"tools": [tool_definition()]})),
        "tools/call" => match call_arguments(&params) {
            Err(problem) => rpc_error(id, -32602, problem),
            Ok((command, timeout)) => match run(spec, workdir, &command, timeout) {
                Ok(executed) => rpc_result(
                    id,
                    json!({
                        "content": [{"type": "text", "text": executed.render()}],
                        "isError": executed.exit_code != 0,
                    }),
                ),
                Err(problem) => rpc_result(
                    id,
                    json!({"content": [{"type": "text", "text": problem}], "isError": true}),
                ),
            },
        },
        other => rpc_error(id, -32601, format!("method not found: {other}")),
    })
}

/// The tool's arguments, refused when they are not exactly the two the
/// schema names.
fn call_arguments(params: &Value) -> Result<(String, Duration), String> {
    if params.get("name").and_then(Value::as_str) != Some(TOOL_NAME) {
        return Err(format!(
            "unknown tool; this server serves only '{TOOL_NAME}'"
        ));
    }
    let empty = Map::new();
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    for key in arguments.keys() {
        if key != "command" && key != "timeoutMs" {
            return Err(format!("{TOOL_NAME}: unknown argument '{key}'"));
        }
    }
    let command = arguments
        .get("command")
        .and_then(Value::as_str)
        .filter(|command| !command.trim().is_empty() && command.len() <= 16_384)
        .ok_or_else(|| format!("{TOOL_NAME}: command must contain 1-16384 characters"))?;
    let timeout_ms = match arguments.get("timeoutMs") {
        None => DEFAULT_TIMEOUT_MS,
        Some(value) => value
            .as_u64()
            .filter(|ms| (100..=MAX_TIMEOUT_MS).contains(ms))
            .ok_or_else(|| {
                format!(
                    "{TOOL_NAME}: timeoutMs must be an integer from 100 through {MAX_TIMEOUT_MS}"
                )
            })?,
    };
    Ok((command.to_string(), Duration::from_millis(timeout_ms)))
}

#[cfg(test)]
mod tests;
