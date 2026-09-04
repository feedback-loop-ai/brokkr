//! The model's hands are one tool, and the tool runs in an empty root
//! (decision 0043).
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
//! what lets a pinned script hold a gate (ruling 3). The strategy is part
//! of the bundle, not the repository it operates on: an exec box binds the
//! bundle root read-only at [`SANDBOX_BUNDLE`] so a bundle-relative script
//! travels with that strategy without becoming writable project input.
//!
//! Every namespace carries [`HANDS_BOX_ENV`]. Tests which themselves open
//! a box refuse nesting from that marker instead of depending on a kernel's
//! user-namespace policy or eventual nesting limit.
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
/// The stable in-box mount point for the bundle that owns an exec seat.
pub const SANDBOX_BUNDLE: &str = "/runtime/bundle";
/// Set by the engine in every namespace so box-building tests do not recurse.
pub const HANDS_BOX_ENV: &str = "BROKKR_HANDS_BOX";
const OUTPUT_BYTES: usize = 262_144;
const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const MAX_TIMEOUT_MS: u64 = 600_000;

/// How a bound host path may be touched from inside the box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindMode {
    /// Read-only.
    Ro,
    /// Read-write: writes land on the host. For the worktree and for
    /// nothing a hostile command could turn into a program you run.
    Rw,
    /// An overlay: the host path is the read-only lower layer and every
    /// write goes to an upper layer that lives for the seat and never
    /// touches the host. A toolchain cache is bound this way.
    Overlay,
}

impl BindMode {
    fn name(self) -> &'static str {
        match self {
            BindMode::Ro => "ro",
            BindMode::Rw => "rw",
            BindMode::Overlay => "overlay",
        }
    }
}

/// One extra bind into the box. `path` may start with `~/`, resolved
/// against the host home at spawn; `mask` names files UNDER the bind
/// that are hidden behind `/dev/null` — a credentials file inside a
/// toolchain directory that must otherwise be readable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bind {
    pub path: String,
    pub mode: BindMode,
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
                "mode": bind.mode.name(),
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
        let mode = match object.get("mode").and_then(Value::as_str) {
            Some("ro") => BindMode::Ro,
            Some("rw") => BindMode::Rw,
            Some("overlay") => BindMode::Overlay,
            _ => {
                return Err(format!(
                    "hands.binds entry '{path}' needs mode \"ro\", \"rw\" or \"overlay\""
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
        Ok(Bind { path, mode, mask })
    }
}

/// `~/x` against the host home; anything else as written.
fn expand_home(path: &str, home: &Path) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None => PathBuf::from(path),
    }
}

/// What the host knows about the worktree's git that the box must be
/// told, gathered OUTSIDE the box before it is built (decision 0043
/// ruling 6): where the git directory is, and who the seat commits as.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitFacts {
    /// The git directory's absolute path — inside the worktree for a
    /// primary checkout, elsewhere for a `git worktree`. `None` when the
    /// workdir is not a git repository.
    pub git_dir: Option<PathBuf>,
    /// `user.name` and `user.email` as the host resolves them, as the
    /// environment entries git reads them from.
    pub identity: Vec<(String, String)>,
}

/// Read the git facts from the host. Never errors: a workdir that is not
/// a repository simply carries none, and the box then has no git to
/// mask.
pub fn git_facts(workdir: &Path) -> GitFacts {
    let git = |args: &[&str]| -> Option<String> {
        let out = Command::new("git")
            .args(args)
            .current_dir(workdir)
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .filter(|text| !text.is_empty())
    };
    let git_dir =
        git(&["rev-parse", "--path-format=absolute", "--git-common-dir"]).map(PathBuf::from);
    let mut identity = Vec::new();
    for (env, key) in [("NAME", "user.name"), ("EMAIL", "user.email")] {
        if let Some(value) = git(&["config", key]) {
            identity.push((format!("GIT_AUTHOR_{env}"), value.clone()));
            identity.push((format!("GIT_COMMITTER_{env}"), value));
        }
    }
    GitFacts { git_dir, identity }
}

/// The bubblewrap argv for one boxed command: the namespace, the binds,
/// the environment, then `--` and the command. `scratch` holds this
/// call's generated identity files and private home and tmp; `session`
/// holds what outlives a call — the upper layers of overlay binds — and
/// is the seat's to remove when it ends.
#[allow(clippy::too_many_arguments)]
pub fn box_argv(
    spec: &HandsSpec,
    workdir: &Path,
    home: &Path,
    scratch: &Path,
    session: &Path,
    git: &GitFacts,
    bundle_root: Option<&Path>,
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
    let text = |path: &Path| path.to_string_lossy().into_owned();
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
        s("--setenv"),
        s(HANDS_BOX_ENV),
        s("1"),
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
    if let Some(bundle_root) = bundle_root {
        argv.extend([s("--ro-bind"), text(bundle_root), s(SANDBOX_BUNDLE)]);
    }
    // Ruling 6: the git directory. A `git worktree`'s lives outside the
    // worktree and is bound so git works at all; either way its `hooks`
    // are hidden behind an empty tmpfs and its `config` is read-only, so
    // nothing a boxed command writes can become a program the host runs
    // on its next git invocation.
    if let Some(git_dir) = &git.git_dir {
        if !git_dir.starts_with(workdir) {
            argv.extend([s("--bind"), text(git_dir), text(git_dir)]);
        }
        argv.extend([s("--tmpfs"), text(&git_dir.join("hooks"))]);
        let config = git_dir.join("config");
        argv.extend([s("--ro-bind-try"), text(&config), text(&config)]);
    }
    for (index, bind) in spec.binds.iter().enumerate() {
        let host = expand_home(&bind.path, home);
        match bind.mode {
            BindMode::Ro => argv.extend([s("--ro-bind-try"), text(&host), text(&host)]),
            BindMode::Rw => argv.extend([s("--bind-try"), text(&host), text(&host)]),
            BindMode::Overlay => {
                let layer = session.join("overlay").join(index.to_string());
                let upper = layer.join("upper");
                let work = layer.join("work");
                std::fs::create_dir_all(&upper)?;
                std::fs::create_dir_all(&work)?;
                argv.extend([
                    s("--overlay-src"),
                    text(&host),
                    s("--overlay"),
                    text(&upper),
                    text(&work),
                    text(&host),
                ]);
            }
        }
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
    let cargo_home = home.join(".cargo");
    let rustup_home = home.join(".rustup");
    let npm_cache = home.join(".npm");
    let sandbox_path = format!(
        "/runtime:{}:/usr/local/bin:/usr/bin:/bin",
        cargo_home.join("bin").display()
    );
    let mut environment = vec![
        ("HOME", SANDBOX_HOME.to_string()),
        ("USER", "runner".to_string()),
        ("LOGNAME", "runner".to_string()),
        ("TMPDIR", "/tmp".to_string()),
        ("PATH", sandbox_path),
        ("LANG", "C.UTF-8".to_string()),
        ("LC_ALL", "C.UTF-8".to_string()),
        ("CI", "true".to_string()),
        ("DISABLE_AUTOUPDATER", "1".to_string()),
        ("DISABLE_TELEMETRY", "1".to_string()),
        // Seat commits are unsigned (CONTRIBUTING); the signing wrapper
        // and its key are not in the box, and the repository config that
        // names them is outranked by this environment entry.
        ("GIT_CONFIG_COUNT", "1".to_string()),
        ("GIT_CONFIG_KEY_0", "commit.gpgsign".to_string()),
        ("GIT_CONFIG_VALUE_0", "false".to_string()),
    ];
    // On rustup installations cargo and rustc are proxies below
    // ~/.cargo/bin. A declared toolchain bind must therefore be both
    // executable and discoverable; the private HOME must not redirect the
    // proxy to an empty ~/.rustup inside the box.
    if spec
        .binds
        .iter()
        .any(|bind| expand_home(&bind.path, home) == cargo_home)
    {
        environment.push(("CARGO_HOME", cargo_home.display().to_string()));
    }
    if spec
        .binds
        .iter()
        .any(|bind| expand_home(&bind.path, home) == rustup_home)
    {
        environment.push(("RUSTUP_HOME", rustup_home.display().to_string()));
    }
    if spec
        .binds
        .iter()
        .any(|bind| expand_home(&bind.path, home) == npm_cache)
    {
        environment.push(("NPM_CONFIG_CACHE", npm_cache.display().to_string()));
    }
    for (key, value) in environment {
        argv.extend([s("--setenv"), s(key), value]);
    }
    for (key, value) in &git.identity {
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

/// The bwrap binary able to build THIS spec: overlays need bubblewrap
/// 0.10 or newer (Ubuntu 24.04 ships 0.9), and a spec that binds one is
/// refused on an older bwrap rather than degraded to a writable bind.
pub fn require_bwrap_for(spec: &HandsSpec) -> Result<PathBuf, String> {
    let bwrap = require_bwrap()?;
    overlay_supported(spec, &bwrap)?;
    Ok(bwrap)
}

/// Refuse a spec with overlay binds on a bwrap older than 0.10.
pub fn overlay_supported(spec: &HandsSpec, bwrap: &Path) -> Result<(), String> {
    if !spec.binds.iter().any(|bind| bind.mode == BindMode::Overlay) {
        return Ok(());
    }
    let reported = Command::new(bwrap)
        .arg("--version")
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default();
    overlay_supported_by(&reported, bwrap)
}

/// The version rule on the string bwrap reported — the testable half.
pub fn overlay_supported_by(reported: &str, bwrap: &Path) -> Result<(), String> {
    match parse_version(reported) {
        Some(version) if version >= (0, 10, 0) => Ok(()),
        _ => Err(format!(
            "hands bind mode 'overlay' needs bubblewrap 0.10 or newer; {} reports {:?}",
            bwrap.display(),
            reported
        )),
    }
}

/// `bubblewrap 0.11.0` → `(0, 11, 0)`; anything else is unknown.
pub fn parse_version(reported: &str) -> Option<(u32, u32, u32)> {
    let digits = reported.split_whitespace().last()?;
    let mut parts = digits.split('.').map(|part| part.parse::<u32>().ok());
    Some((
        parts.next()??,
        parts.next()??,
        parts.next().flatten().unwrap_or(0),
    ))
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
pub type Executor = dyn Fn(&HandsSpec, &Path, &Path, &str, Duration) -> Result<Executed, String>;

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

/// Drain a pipe to its end, keeping at most `OUTPUT_BYTES` and noting
/// when more arrived: the bound is on host memory, not only on what the
/// model reads back.
pub fn drain_bounded<R: Read>(mut pipe: R) -> (Vec<u8>, bool) {
    let mut kept = Vec::new();
    let mut truncated = false;
    let mut chunk = [0_u8; 8192];
    loop {
        let read = match pipe.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => read,
        };
        let room = OUTPUT_BYTES.saturating_sub(kept.len());
        if read > room {
            truncated = true;
        }
        kept.extend_from_slice(&chunk[..read.min(room)]);
    }
    (kept, truncated)
}

fn rendered(bytes: &[u8], truncated: bool) -> String {
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    if truncated {
        text.push_str(&format!("\n[output truncated at {OUTPUT_BYTES} bytes]"));
    }
    text
}

/// A session directory for what outlives one call — overlay upper
/// layers — created for the server's or the exec verb's lifetime.
pub fn session_dir(label: &str) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!(
        "brokkr-hands-{label}-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    io_context(std::fs::create_dir_all(&dir), "session")?;
    Ok(dir)
}

/// Run one `bash -lc <command>` inside the box, bounded in time and
/// output. This call's scratch is removed afterwards; `session` is the
/// caller's.
pub fn execute(
    spec: &HandsSpec,
    workdir: &Path,
    session: &Path,
    command: &str,
    timeout: Duration,
) -> Result<Executed, String> {
    let bwrap = require_bwrap_for(spec)?;
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let scratch = session.join(format!("call-{}", uuid::Uuid::new_v4()));
    io_context(std::fs::create_dir_all(&scratch), "scratch")?;
    let git = git_facts(workdir);
    let result = execute_in(
        &bwrap, spec, workdir, &home, &scratch, session, &git, command, timeout,
    );
    let _ = std::fs::remove_dir_all(&scratch);
    result
}

#[allow(clippy::too_many_arguments)]
pub fn execute_in(
    bwrap: &Path,
    spec: &HandsSpec,
    workdir: &Path,
    home: &Path,
    scratch: &Path,
    session: &Path,
    git: &GitFacts,
    command: &str,
    timeout: Duration,
) -> Result<Executed, String> {
    let inner = vec![
        "/bin/bash".to_string(),
        "-lc".to_string(),
        command.to_string(),
    ];
    let built = box_argv(spec, workdir, home, scratch, session, git, None, &inner);
    let argv = io_context(built, "namespace")?;
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
    let stdout_pipe = child.stdout.take().expect("piped");
    let stderr_pipe = child.stderr.take().expect("piped");
    let stdout_thread = std::thread::spawn(move || drain_bounded(stdout_pipe));
    let stderr_thread = std::thread::spawn(move || drain_bounded(stderr_pipe));
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
    let (stdout, stdout_cut) = stdout_thread.join().unwrap_or_default();
    let (stderr, stderr_cut) = stderr_thread.join().unwrap_or_default();
    Ok(Executed {
        stdout: rendered(&stdout, stdout_cut),
        stderr: rendered(&stderr, stderr_cut),
        exit_code: status.code().unwrap_or(if timed_out { 124 } else { -1 }),
        timed_out,
    })
}

/// Run a whole command inside the box with its stdio passed through —
/// how a deterministic `exec` seat holds a gate (ruling 3). This very
/// binary is bound read-only so the command may be a `brokkr driver …`
/// dispatch. Returns the child's exit code.
pub fn run_boxed(
    spec: &HandsSpec,
    workdir: &Path,
    bundle_root: Option<&Path>,
    command: &[String],
) -> Result<i32, String> {
    let bwrap = require_bwrap_for(spec)?;
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let session = session_dir("exec")?;
    let git = git_facts(workdir);
    let result = run_boxed_in(
        &bwrap,
        spec,
        workdir,
        &home,
        &session,
        &git,
        bundle_root,
        command,
    );
    let _ = std::fs::remove_dir_all(&session);
    result
}

#[allow(clippy::too_many_arguments)]
pub fn run_boxed_in(
    bwrap: &Path,
    spec: &HandsSpec,
    workdir: &Path,
    home: &Path,
    session: &Path,
    git: &GitFacts,
    bundle_root: Option<&Path>,
    command: &[String],
) -> Result<i32, String> {
    let mut with_self = spec.clone();
    with_self
        .binds
        .extend(std::env::current_exe().ok().map(|exe| Bind {
            path: exe.to_string_lossy().into_owned(),
            mode: BindMode::Ro,
            mask: Vec::new(),
        }));
    let scratch = session.join("call");
    let built = box_argv(
        &with_self,
        workdir,
        home,
        &scratch,
        session,
        git,
        bundle_root,
        command,
    );
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
    session: &Path,
    spec: &HandsSpec,
    run: &Executor,
) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Some(reply) = handle(&line, workdir, session, spec, run) else {
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
pub fn handle(
    line: &str,
    workdir: &Path,
    session: &Path,
    spec: &HandsSpec,
    run: &Executor,
) -> Option<Value> {
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
            Ok((command, timeout)) => match run(spec, workdir, session, &command, timeout) {
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
