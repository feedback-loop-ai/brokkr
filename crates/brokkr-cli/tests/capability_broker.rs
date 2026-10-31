//! `brokkr broker serve` (decision 0065 slice two U6b and U6c; MB3, MB4,
//! SC1, SD3), driven through the real binary. The command takes a bounded
//! plan locator and digest and nothing else: no server argv, grant or
//! secret value is an option. A plan is bound only where the attempt's
//! protected inventory pins it; a bound plan's every field and box intent
//! is checked in MB3's order before any lookup or start. An admitted plan's
//! box is made ready, its bindings handed over and its server started
//! inside it (U6c8), initialized (U6e) and served (U6f1b); the fixture's
//! plan retains its responses, which are staged beside its ledger (U8b).
//! A bound plan's one lifetime has its private ledger (U6d). The compile
//! fence still refuses every MCP grant.

use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use brokkr_protocol::broker::Refusal;
use serde_json::{json, Value};

#[cfg(target_os = "linux")]
mod cleanup;
#[cfg(target_os = "linux")]
mod handoff;
mod ledger;
#[cfg(target_os = "linux")]
mod observer;
#[cfg(target_os = "linux")]
mod protocol;
#[cfg(target_os = "linux")]
mod readiness;
#[cfg(target_os = "linux")]
mod serving;
mod survey;

const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// The value a fixture's store binds to `DOCS_TOKEN`: a canary no other
/// byte of a run spells, in any of decision 0012's encodings.
const TOKEN: &str = "u6c8-canary-Qx7Zv";

/// What one invocation left: its exit code, stdout and stderr.
struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// A canonicalised temporary root every fixture is built under. It lies
/// under `/var/tmp`, whose ancestry no one else may write whatever the
/// umask, and not under `/tmp`: a server box mounts no source below its
/// private `/tmp` (MB4). A seat's hands box binds no `/var/tmp` and admits
/// no plan, so there its refusal paths build in the build's own temporary
/// directory, which the box binds and which is no server's private path.
struct Root {
    _dir: tempfile::TempDir,
    path: PathBuf,
}

/// Where fixtures and the binary's copy are made: `/var/tmp`, or in a
/// seat's hands box the build's own temporary directory.
fn base() -> &'static str {
    match boxed() {
        true => env!("CARGO_TARGET_TMPDIR"),
        false => "/var/tmp",
    }
}

/// What each test process's own directory for its copy is named first.
const COPIES: &str = "brokkr-capability-broker-";

/// The binary under test, as a singly linked copy: cargo's `brokkr` is a
/// second link to its build artifact, and a bootstrap with a second link
/// refuses (MB3). Each process copies it once into an owner-only directory
/// of its own, its path the process's alone, so no other process can
/// replace it, nor run it while it is written. The process holds the
/// directory's `lock` exclusively for as long as it runs: unlike a process
/// id, a lock means the same in every PID namespace sharing the directory.
/// The copy is written by a child `cp`, then renamed in (#255): a copy this
/// process wrote is `Text file busy` while a sibling test's fork still holds
/// the inherited write descriptor, and a rename keeps the inode's count.
fn brokkr() -> &'static Path {
    static COPY: std::sync::OnceLock<(PathBuf, std::fs::File)> = std::sync::OnceLock::new();
    let (copy, _live) = COPY.get_or_init(|| {
        if !boxed() {
            sweep();
        }
        let dir = tempfile::Builder::new().prefix(COPIES).tempdir_in(base());
        let dir = dir.unwrap().keep();
        chmod(&dir, 0o700);
        // Locked before it takes the name a sweep looks for.
        let live = std::fs::File::create(dir.join("lock.new")).unwrap();
        assert!(lock(&live));
        std::fs::rename(dir.join("lock.new"), dir.join("lock")).unwrap();
        let (staged, copy) = (dir.join("brokkr.staged"), dir.join("brokkr"));
        let cp = Command::new("/bin/cp")
            .arg(env!("CARGO_BIN_EXE_brokkr"))
            .arg(&staged)
            .status();
        assert_eq!(cp.unwrap().code(), Some(0), "copying the binary");
        chmod(&staged, 0o755);
        std::fs::rename(&staged, &copy).unwrap();
        (std::fs::canonicalize(copy).unwrap(), live)
    });
    copy
}

/// Whether `file`'s exclusive lock was taken here, without waiting.
fn lock(file: &std::fs::File) -> bool {
    rustix::fs::flock(file, rustix::fs::FlockOperation::NonBlockingLockExclusive).is_ok()
}

/// Remove each copy's directory whose process has ended: one whose lock
/// this process can take. A directory still being made has no lock yet,
/// and is left.
fn sweep() {
    let Ok(entries) = std::fs::read_dir(base()) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let ours = name.to_str().is_some_and(|name| name.starts_with(COPIES));
        let held = std::fs::File::open(entry.path().join("lock"));
        if ours && held.is_ok_and(|held| lock(&held)) {
            std::fs::remove_dir_all(entry.path()).ok();
        }
    }
}

impl Root {
    fn new() -> Root {
        let dir = tempfile::tempdir_in(base()).unwrap();
        Root {
            path: dir.path().canonicalize().unwrap(),
            _dir: dir,
        }
    }

    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.path.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    /// An owner-only store at `relative` holding the plan's one declared
    /// binding, [`TOKEN`], every directory it lies in below the root
    /// writable by its owner alone whatever the umask, as MB4 guards a
    /// store's route (U6c5c).
    fn store(&self, relative: &str) -> PathBuf {
        let path = self.write(relative, &format!("DOCS_TOKEN={TOKEN}\n"));
        chmod(&path, 0o600);
        let dirs = path.ancestors().skip(1);
        for dir in dirs.take_while(|dir| *dir != self.path) {
            chmod(dir, 0o755);
        }
        path
    }

    fn brokkr(&self, args: &[&str]) -> Ran {
        self.ran(self.command(args))
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = confined();
        command.args(args).current_dir(&self.path);
        command
    }

    /// The binary under test run with `args` as a seat's harness starts
    /// the broker: on Linux a shell under `no_new_privs` stands for the
    /// harness, whose credentials the observer reads too (U6c5c), and the
    /// broker is its child.
    fn harnessed(&self, args: &[&str]) -> Command {
        let mut command = match cfg!(target_os = "linux") {
            true => {
                let mut command = Command::new(setpriv());
                command.args(["--no-new-privs", "/bin/sh", "-c", HARNESS]);
                command.arg(brokkr());
                command
            }
            false => Command::new(brokkr()),
        };
        command.args(args).current_dir(&self.path);
        command
    }

    fn ran(&self, mut command: Command) -> Ran {
        command.stdin(Stdio::null()).stdout(Stdio::piped());
        let out = bounded(command.stderr(Stdio::piped()).spawn().unwrap(), b"");
        Ran {
            code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }
    }
}

/// How long a test waits for a broker it started, and for its output: past
/// the startup's thirty seconds and the thirty-one one test idles for, with
/// room on a loaded host.
const TALK_MAX: Duration = Duration::from_secs(90);

/// One of a child's standard streams, read on a thread of its own from its
/// start, so that it never fills: what it has said so far, and word of its
/// end.
type Apart = (Arc<Mutex<Vec<u8>>>, Receiver<()>);

/// `stream`, read apart ([`Apart`]).
fn apart(mut stream: impl Read + Send + 'static) -> Apart {
    let said = Arc::new(Mutex::new(Vec::new()));
    let (into, (ended, ending)) = (Arc::clone(&said), std::sync::mpsc::channel());
    std::thread::spawn(move || {
        let mut chunk = [0; 4096];
        while let Ok(read @ 1..) = stream.read(&mut chunk) {
            into.lock().unwrap().extend_from_slice(&chunk[..read]);
        }
        ended.send(()).ok();
    });
    (said, ending)
}

/// What `child` said on its standard output and error, each read apart
/// where it is piped, and its exit, once `said` is written to its input,
/// where that is piped, which is held open until it ends. Every wait is
/// bounded by [`TALK_MAX`]: a child that has not ended by then is killed
/// with its children, and it, or one whose streams something it left behind
/// holds open past then, fails the test, naming its status and its standard
/// error as far as it was read (the review of U6f1b's sixth run).
fn bounded(child: Child, said: &[u8]) -> Output {
    bounded_by(child, said, (TALK_MAX, None))
}

/// [`bounded`], the broker's own standard error redirected by its starter
/// to the file `apart`, which a failure names too (the review of U6f1b's
/// seventh run).
#[cfg(target_os = "linux")]
fn bounded_apart(child: Child, said: &[u8], apart: &Path) -> Output {
    bounded_by(child, said, (TALK_MAX, Some(apart)))
}

/// [`bounded`] within `max`, a failure naming what `file`, where there is
/// one, holds after the standard error.
fn bounded_by(mut child: Child, said: &[u8], (max, file): (Duration, Option<&Path>)) -> Output {
    let until = Instant::now() + max;
    let told = || file.and_then(|file| std::fs::read_to_string(file).ok());
    let mut input = child.stdin.take();
    if let Some(input) = input.as_mut() {
        input.write_all(said).unwrap();
    }
    let (stdout, stderr) = (
        child.stdout.take().map(apart),
        child.stderr.take().map(apart),
    );
    let held = |stream: &Option<Apart>| {
        let said = stream
            .as_ref()
            .map(|(said, _)| said.lock().unwrap().clone());
        said.unwrap_or_default()
    };
    let failed = |why: &str, status: std::process::ExitStatus| -> ! {
        panic!(
            "{why}: {status:?}\n{}{}",
            String::from_utf8_lossy(&held(&stderr)),
            told().unwrap_or_default()
        )
    };
    let status = loop {
        match child.try_wait().unwrap() {
            Some(status) => break status,
            None if Instant::now() >= until => {
                killed(child.id());
                failed("the broker did not end", child.wait().unwrap())
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    drop(input);
    // Ended and reaped, its process id is no longer its own to kill.
    for (_, ending) in [&stdout, &stderr].into_iter().flatten() {
        if ending
            .recv_timeout(until.saturating_duration_since(Instant::now()))
            .is_err()
        {
            failed("the broker's output did not end", status);
        }
    }
    let (stdout, stderr) = (held(&stdout), held(&stderr));
    Output {
        status,
        stdout,
        stderr,
    }
}

/// Kill the process `pid`, a harness shell or the broker, and its
/// children.
fn killed(pid: u32) {
    let pid = pid.to_string();
    let run = |command: &str, args: &[&str]| {
        Command::new(command).args(args).status().ok();
    };
    run("pkill", &["-KILL", "-P", &pid]);
    run("kill", &["-KILL", &pid]);
}

/// The binary under test as a confined managed writer runs it: on Linux
/// under `no_new_privs`, without which its observer refuses a host's
/// multiply-linked system file (MB3). Setting it here would bind every
/// later test in this process, so `setpriv` sets it for the child alone.
fn confined() -> Command {
    match cfg!(target_os = "linux") {
        true => {
            let mut command = Command::new("setpriv");
            command.arg("--no-new-privs").arg(brokkr());
            command
        }
        false => Command::new(brokkr()),
    }
}

/// The harness shell's script: run its arguments as a child, not in its
/// own place, and exit as that child did.
const HARNESS: &str = "\"$0\" \"$@\"; exit $?";

/// `setpriv` where this process's `PATH` finds it, so that a broker's
/// own `PATH` need not.
fn setpriv() -> PathBuf {
    let path = std::env::var_os("PATH").unwrap();
    let mut found = std::env::split_paths(&path).map(|dir| dir.join("setpriv"));
    found.find(|setpriv| setpriv.is_file()).unwrap()
}

/// The first line of a refusal clap printed, and the usage exit.
fn usage(ran: &Ran) -> (Option<i32>, &str) {
    (ran.code, ran.stderr.lines().next().unwrap_or_default())
}

/// A plan file whose server would leave a marker if it were ever started.
fn planted(root: &Root) -> (PathBuf, PathBuf) {
    let marker = root.path.join("started");
    let server = root.write(
        "server.sh",
        &format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    );
    let plan = root.write(
        "plan.json",
        &json!({"server": {"argv": [server]}, "tools": ["lookup"]}).to_string(),
    );
    (plan, marker)
}

#[test]
fn a_manual_invocation_with_a_real_file_and_its_digest_is_unbound() {
    let root = Root::new();
    let (plan, marker) = planted(&root);
    let digest = brokkr_core::canonical::sha256_bytes(&std::fs::read(&plan).unwrap());
    let ran = root.brokkr(&[
        "broker",
        "serve",
        "--plan",
        plan.to_str().unwrap(),
        "--plan-digest",
        &digest,
    ]);
    assert_eq!(ran.code, Some(1));
    assert_eq!(
        ran.stderr,
        "error: broker plan is not bound to this attempt\n"
    );
    assert_eq!(ran.stdout, "");
    // The locator's server was never started.
    assert!(!marker.exists());
}

#[test]
fn the_plan_locator_and_digest_are_bounded_where_they_are_parsed() {
    let root = Root::new();
    let longest = format!("/{}", "a".repeat(1023));
    let over = format!("/{}", "a".repeat(1024));
    let refused = |plan: &str, digest: &str| {
        let ran = root.brokkr(&["broker", "serve", "--plan", plan, "--plan-digest", digest]);
        let (code, line) = usage(&ran);
        (code, line.to_string())
    };
    let invalid = |value: &str, arg: &str, why: &str| {
        (
            Some(2),
            format!("error: invalid value '{value}' for '{arg}': {why}"),
        )
    };
    assert_eq!(
        refused("plan.json", DIGEST),
        invalid(
            "plan.json",
            "--plan <PLAN>",
            "a plan locator is an absolute path"
        )
    );
    assert_eq!(
        refused("/run/../plan.json", DIGEST),
        invalid(
            "/run/../plan.json",
            "--plan <PLAN>",
            "a plan locator names no parent directory"
        )
    );
    assert_eq!(
        refused(&over, DIGEST),
        invalid(
            &over,
            "--plan <PLAN>",
            "a plan locator is at most 1024 bytes"
        )
    );
    for digest in [
        &DIGEST.to_uppercase(),
        &DIGEST[1..],
        &format!("sha256:{DIGEST}"),
    ] {
        assert_eq!(
            refused("/plan.json", digest),
            invalid(
                digest,
                "--plan-digest <PLAN_DIGEST>",
                "a plan digest is 64 lowercase hex characters"
            )
        );
    }
    // At the bound the locator parses, and the plan is still unbound.
    assert_eq!(
        refused(&longest, DIGEST),
        (
            Some(1),
            "error: broker plan is not bound to this attempt".to_string()
        )
    );
}

#[test]
fn no_server_argv_grant_or_secret_value_is_an_option() {
    let root = Root::new();
    let serve = |extra: &[&str]| {
        let mut args = vec![
            "broker",
            "serve",
            "--plan",
            "/plan.json",
            "--plan-digest",
            DIGEST,
        ];
        args.extend(extra);
        let ran = root.brokkr(&args);
        let (code, line) = usage(&ran);
        (code, line.to_string())
    };
    for option in [
        "--server", "--argv", "--grant", "--tools", "--secret", "--env",
    ] {
        assert_eq!(
            serve(&[option, "x"]),
            (
                Some(2),
                format!("error: unexpected argument '{option}' found")
            )
        );
    }
    assert_eq!(
        serve(&["--", "/usr/bin/docs-mcp"]),
        (
            Some(2),
            "error: unexpected argument '/usr/bin/docs-mcp' found".to_string()
        )
    );
    // Neither half of the locator may be left out.
    let bare = root.brokkr(&["broker", "serve", "--plan", "/plan.json"]);
    assert_eq!(
        usage(&bare),
        (
            Some(2),
            "error: the following required arguments were not provided:"
        )
    );
    assert_eq!(
        bare.stderr.lines().nth(1).map(str::trim),
        Some("--plan-digest <PLAN_DIGEST>")
    );
}

/// The binary's waiting bootstrap started as a box starts it (U6c6a,
/// U6c8): `frame` arrives on an anonymous pipe at descriptor 3, stdin is
/// closed, the status pipe is a third anonymous pipe at descriptor 5, and
/// `ready` redirects descriptor 4, the ready channel; `$2` is a marker
/// file beside the frame. The shell exits as the bootstrap did.
fn bootstrapped(root: &Root, frame: &[u8], ready: &str) -> Ran {
    let framed = root.path.join("frame");
    std::fs::write(&framed, frame).unwrap();
    let script = format!(
        "exec 6>&1; {{ cat \"$1\" | \"$0\" broker bootstrap --control 3 --ready 4 --status 5 \
         3<&0 0</dev/null 5>&7 7>&- 6>&- {ready}; echo $? >\"$1.exit\"; }} 7>&1 1>&6 \
         | cat >/dev/null; exit \"$(cat \"$1.exit\")\""
    );
    let mut command = Command::new("/bin/sh");
    command.args(["-c", &script]).arg(brokkr()).arg(&framed);
    command
        .arg(root.path.join("marker"))
        .current_dir(&root.path);
    root.ran(command)
}

/// A sealed intent for an isolated box, as one control frame.
fn intent_frame() -> Vec<u8> {
    let host = json!({"mnt": 1, "pid": 2, "net": 3, "ipc": 4, "uts": 5});
    let intent = json!({
        "plan": DIGEST, "sources": DIGEST, "network": "isolated", "host": host, "mounts": [],
    });
    let body = intent.to_string().into_bytes();
    let length = u32::try_from(body.len()).unwrap().to_be_bytes();
    [&length[..], &body].concat()
}

#[test]
fn the_bootstrap_without_its_private_control_context_is_not_established() {
    let root = Root::new();
    let cases: [&[&str]; 12] = [
        &[],
        &["--control", "3"],
        &["--ready", "4"],
        &["--control", "x", "--ready", "4"],
        &["--control", "1", "--ready", "2"],
        &["--control", "7", "--ready", "8"],
        &["--control", "-1", "--ready", "4"],
        &["--control", "3", "--ready", "-1"],
        &["--control"],
        &["--ready"],
        &["--control", "--ready", "4"],
        &["--control=", "--ready", "4"],
    ];
    for args in cases {
        let ran = root.brokkr(&[&["broker", "bootstrap"], args].concat());
        let answer = ((ran.code, ran.stderr), ran.stdout);
        let established = (refused(Refusal::Establishment), String::new());
        assert_eq!((args, answer), (args, established));
    }
}

/// A host is no box, and neither stdout nor a marker file is a ready
/// channel: each refuses before any ready byte. No plan, store or grant
/// is an option, so nothing is looked up or started.
#[test]
fn neither_a_host_nor_stdout_nor_a_marker_carries_readiness() {
    let root = Root::new();
    for ready in ["4>&1 1>&2", "4>&1", "4>\"$2\""] {
        let ran = bootstrapped(&root, &intent_frame(), ready);
        let answer = ((ran.code, ran.stderr), ran.stdout);
        let established = (refused(Refusal::Establishment), String::new());
        assert_eq!((ready, answer), (ready, established));
    }
    assert_eq!(std::fs::read(root.path.join("marker")).unwrap(), b"");
    for option in ["--plan", "--store", "--grant", "--exec"] {
        let ran = root.brokkr(&["broker", "bootstrap", option, "/x"]);
        let expected = format!("error: unexpected argument '{option}' found");
        assert_eq!(usage(&ran), (Some(2), expected.as_str()));
    }
}

#[test]
fn the_grouped_library_verbs_still_dispatch() {
    let root = Root::new();
    let store = root.write("secrets.env", "DOCS_TOKEN=value\n");
    let private = std::os::unix::fs::PermissionsExt::from_mode(0o600);
    std::fs::set_permissions(&store, private).unwrap();
    let ran = root.brokkr(&["secrets", "list", "--secrets-file", "secrets.env"]);
    assert_eq!((ran.code, ran.stdout.as_str()), (Some(0), "DOCS_TOKEN\n"));
}

/// `brokkr init`'s folder, in a repository of its own whatever checkout
/// the root sits in, whose map grants its one realm `library-docs`
/// through an MCP dialect that no seat asks for.
fn granting_mcp(root: &Root) {
    crate::brokkr_folder::git(&root.path, &["init", "-q"]);
    assert_eq!(root.brokkr(&["init", "."]).code, Some(0));
    let map = root.path.join(".brokkr/realms.json");
    let mut realms: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&map).unwrap()).unwrap();
    realms["schema"] = json!("forge.realms/v6");
    realms["realms"][0]["capabilities"] =
        json!({"library-docs": {"dialect": "docs-mcp", "offices": []}});
    root.write(".brokkr/realms.json", &realms.to_string());
    root.write(
        ".brokkr/capabilities/library-docs.json",
        &json!({"name": "library-docs", "classes": ["reads", "egress"]}).to_string(),
    );
    root.write(
        ".brokkr/dialects/tools/docs-mcp.json",
        &json!({"schema": "brokkr.tool-dialect/v1", "name": "docs-mcp",
                "serves": "library-docs", "kind": "mcp",
                "connection": {"argv": ["/nonexistent/docs-mcp"]}, "version": "1.4.2",
                "secrets": ["DOCS_TOKEN"], "tools": ["resolve", "read"], "retained": true,
                "sends": {"description": "a library name", "seat_composed": true}})
        .to_string(),
    );
}

/// The fence stands on macOS, where an MCP grant no seat asks for still
/// refuses exactly; on Linux, where U9b1 lifts it, the same folder
/// compiles, its grant held by no office.
#[test]
fn the_compile_fence_refuses_an_unused_mcp_grant_only_where_it_stands() {
    let root = Root::new();
    granting_mcp(&root);
    let ran = root.brokkr(&["compile", "--bundle", ".brokkr"]);
    if cfg!(target_os = "linux") {
        let compiled: serde_json::Value = serde_json::from_str(&ran.stdout).unwrap();
        return assert_eq!(
            (ran.code, &compiled["bundle"]),
            (Some(0), &json!("starter"))
        );
    }
    assert_eq!(ran.code, Some(1));
    assert_eq!(
        ran.stderr,
        "error: bundle: realm 'starter' grants capability 'library-docs' through dialect 'docs-mcp' of \
         kind 'mcp', whose broker support is not implemented until decision 0065 slice two\n"
    );
    assert_eq!(ran.stdout, "");
}

/// Each refusal's text, pinned once: MB3 and MB4's exact causes, listed in
/// MB3's precedence, which is their order.
#[test]
fn each_refusal_reads_in_mb3_and_mb4s_words() {
    let texts = [
        (Refusal::Unbound, "broker plan is not bound to this attempt"),
        (
            Refusal::Name("secret name 'x' is bad".into()),
            "secret name 'x' is bad",
        ),
        (
            Refusal::StartupInputs,
            "MCP server startup inputs are not protected from seat writes",
        ),
        (
            Refusal::ProgramTree,
            "MCP server box program tree cannot be resolved",
        ),
        (
            Refusal::LaunchInReach,
            "MCP server launch resolves inside seat-writable reach",
        ),
        (
            Refusal::BindOverlapsReach,
            "MCP server box bind overlaps seat reach",
        ),
        (
            Refusal::Linked,
            "MCP server box program tree contains a multiply-linked file",
        ),
        (
            Refusal::Identity,
            "MCP server box filesystem identity is not protected",
        ),
        (Refusal::Unavailable, "MCP server box is unavailable"),
        (
            Refusal::StoreReachable,
            "MCP secret store is reachable by workspace hands",
        ),
        (
            Refusal::StoreInBox,
            "MCP secret store would be mounted in the server box",
        ),
        (
            Refusal::Establishment,
            "MCP server box could not be established",
        ),
        (
            Refusal::Handoff,
            "MCP server secret environment could not be delivered",
        ),
        (Refusal::Exec, "MCP server could not start inside its box"),
        (
            Refusal::Response,
            "MCP server response exceeds the broker limit",
        ),
        (Refusal::Timeout, "MCP server call timed out"),
        (Refusal::Masking, "MCP response cannot be safely masked"),
    ];
    let precedence: Vec<&Refusal> = texts.iter().map(|(refusal, _)| refusal).collect();
    assert!(precedence.is_sorted(), "{precedence:?}");
    for (refusal, text) in texts {
        assert_eq!(refusal.to_string(), text);
    }
}

/// The exit and stderr of a refusal.
fn refused(refusal: Refusal) -> (Option<i32>, String) {
    (Some(1), format!("error: {refusal}\n"))
}

/// The answer to a plan whose box the observer prepared: `refusal`, the
/// cause after it, on Linux; elsewhere no launcher mounts a checked
/// descriptor, so the box is unavailable (MB3).
fn past_prepare(refusal: Refusal) -> (Option<i32>, String) {
    match cfg!(target_os = "linux") {
        true => refused(refusal),
        false => refused(Refusal::Unavailable),
    }
}

/// The answer to the fixture's plan, which retains its responses, once the
/// broker admits it: its server initialized, and its harness, which says
/// nothing and ends at once, served to that end (U8b). Off Linux no
/// launcher mounts a checked descriptor, so the box is unavailable (MB3).
fn admitted() -> (Option<i32>, String) {
    match cfg!(target_os = "linux") {
        true => (Some(0), String::new()),
        false => refused(Refusal::Unavailable),
    }
}

/// The exit and stderr of a session that ends with `failure` once its
/// server has started (U6e).
#[cfg(target_os = "linux")]
fn ended(failure: brokkr_protocol::broker::ledger::Failure) -> (Option<i32>, String) {
    (Some(1), format!("error: {failure}\n"))
}

/// What the fixture's server says once started, run by the box's own
/// shell: `serve`'s startup in MCP and nothing more, the dialect's pinned
/// version and both granted tools beside one the plan does not grant,
/// each request read whole as one line; then it waits for its input's end.
const SPEAKS: &str = r#"read -r line
echo '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"docs","version":"1.4.2"}}}'
read -r line
read -r line
echo '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"resolve","inputSchema":{"type":"object"}},{"name":"admin","inputSchema":{"type":"object"}},{"name":"read","inputSchema":{"type":"object"}}]}}'
read -r line
"#;

/// Whether this run stands in a seat's hands box, where no box is prepared.
fn boxed() -> bool {
    // An unprivileged bubblewrap user namespace maps root-owned host files to the overflow uid, so MB3 rightly refuses their filesystem identity.
    std::env::var_os(brokkr_protocol::hands::HANDS_BOX_ENV).is_some()
}

/// Skip a proof this run cannot give, for `reason`: a run that declared
/// boundary evidence fails instead of passing on it.
fn skip(reason: &str) {
    let required = brokkr_protocol::hands::boundary_evidence_required();
    brokkr_protocol::hands::skip_boundary_proof(required, reason);
}

/// Why a boxed run skips: a seat's hands box prepares no server box.
const BOXED: &str = "a seat's hands box prepares no server box";

/// Why no live server box stands for this run, if none does: a seat's
/// hands box prepares none. A host's root-only system files are no reason
/// (operator ruling 2026-10-07): MB3 admits them, so a box stands beside
/// them.
fn unservable() -> Option<String> {
    boxed().then(|| BOXED.to_string())
}

/// Check that `answer`, labelled `case`, is a sealed plan's that binds:
/// admitted where a live box stands; where none can, refused past binding
/// with the observer's own identity cause, and the rest declared skipped.
/// Its caller has checked that the base is protected ([`bindable`]).
fn binds(case: &str, answer: (Option<i32>, String)) {
    match unservable() {
        None => assert_eq!((case, answer), (case, admitted())),
        Some(reason) => {
            let observed = past_prepare(Refusal::Identity);
            assert_eq!((case, answer), (case, observed));
            skip(&reason);
        }
    }
}

/// Whether the broker's guard accepts every directory from `/` down to
/// [`base`]: each this user's or root's, and written by no one else but
/// where root's and sticky. Where it does not, as under a hands box's
/// build directory on an umask-002 host, no plan built there binds.
fn protected_base() -> bool {
    use std::os::unix::fs::MetadataExt;
    let euid = rustix::process::geteuid().as_raw();
    let base = std::fs::canonicalize(base()).unwrap();
    base.ancestors().all(|dir| {
        let held = std::fs::metadata(dir).unwrap();
        let (uid, mode) = (held.uid(), held.mode());
        let sticky = (uid, mode & 0o1000) == (0, 0o1000);
        [euid, 0].contains(&uid) & ((mode & 0o022 == 0) | sticky)
    })
}

/// Why a run with no protected base skips a plan that must bind.
const UNPROTECTED: &str = "no directory whose ancestry the broker's guard accepts holds fixtures";

/// Whether a plan can bind here: where no base is protected, the guard
/// refuses every plan before it reads one, so no refusal after binding is
/// told apart from that one, and the whole proof is declared skipped.
fn bindable() -> bool {
    let protected = protected_base();
    if !protected {
        skip(UNPROTECTED);
    }
    protected
}

/// A sealed fixture and the answer to a plan that does not bind, where a
/// plan can bind here ([`bindable`]); none, declared skipped, where not.
fn unbound_fixture() -> Option<(Sealed, (Option<i32>, String))> {
    bindable().then(|| (Sealed::new(), refused(Refusal::Unbound)))
}

/// Set the plan field at `pointer` to `value`.
fn set(plan: &mut Value, pointer: &str, value: Value) {
    *plan.pointer_mut(pointer).unwrap() = value;
}

/// The canonical bootstrap: the binary under test.
fn bootstrap() -> PathBuf {
    brokkr().to_path_buf()
}

/// This process's effective uid, every uid it holds: the one managed
/// writer the observer finds while the broker stands for every writer.
fn euid() -> u32 {
    rustix::process::geteuid().as_raw()
}

fn chmod(path: &Path, mode: u32) {
    let mode = std::os::unix::fs::PermissionsExt::from_mode(mode);
    std::fs::set_permissions(path, mode).unwrap();
}

/// The engine's identities: UUID strings for the effect and attempt.
const EFFECT: &str = "3f6c2a0e-8d1b-4c5e-9a7f-1b2c3d4e5f60";
const ATTEMPT: &str = "9b1e7c42-5d3a-4f8e-b6a1-0c2d4e6f8a9b";

/// The attempt directory `name` of repository `repo` under `base`'s
/// protected layout, owner-only from the layout's root down.
fn attempt_under(base: &Path, repo: &str, name: &str) -> PathBuf {
    let attempt = base.join(repo).join("run-1").join(name);
    std::fs::create_dir_all(&attempt).unwrap();
    for dir in attempt.ancestors().take(4) {
        chmod(dir, 0o700);
    }
    attempt
}

/// An attempt the engine sealed: the owner-only protected tree under a host
/// HOME, the attempt's inventory, the plan it pins, and the sources the
/// observer found for that plan, once observed.
struct Sealed {
    root: Root,
    home: PathBuf,
    attempt: PathBuf,
    locator: PathBuf,
    sources: std::sync::OnceLock<Value>,
    _slot: Slot,
}

/// The sealed fixtures, and so the real boxes, the suite holds at once.
/// Every box's admission observes the whole system set against the
/// broker's fixed startup deadline, which no test may lengthen; on a small
/// runner many started together starve one another past it.
const BOXES: usize = 3;

/// The suite's places among [`BOXES`]: how many are taken, and the signal
/// that one was given back.
static SLOTS: (std::sync::Mutex<usize>, std::sync::Condvar) =
    (std::sync::Mutex::new(0), std::sync::Condvar::new());

std::thread_local! {
    /// The places this test's own thread holds, so a fixture built inside
    /// another takes none and no test waits on itself.
    static HELD: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// A place among [`BOXES`], given back when its fixture is dropped.
struct Slot;

impl Slot {
    fn take() -> Slot {
        if HELD.get() == 0 {
            let (taken, freed) = &SLOTS;
            let mut taken = taken
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            while *taken >= BOXES {
                taken = freed
                    .wait(taken)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
            *taken += 1;
        }
        HELD.set(HELD.get() + 1);
        Slot
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        HELD.set(HELD.get() - 1);
        if HELD.get() == 0 {
            let (taken, freed) = &SLOTS;
            *taken
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) -= 1;
            freed.notify_one();
        }
    }
}

impl Sealed {
    fn new() -> Sealed {
        Sealed::at(DIGEST, ATTEMPT)
    }

    /// An attempt sealed in the directory `name` of repository `repo`.
    fn at(repo: &str, name: &str) -> Sealed {
        let slot = Slot::take();
        let root = Root::new();
        let home = root.path.join("home");
        let capabilities = home.join(".local/state/brokkr/capabilities");
        let attempt = attempt_under(&capabilities, repo, name);
        // The host HOME's ancestry is writable by no one else, whatever
        // the umask.
        chmod(&root.path, 0o700);
        for dir in attempt.ancestors().skip(4).take(4) {
            chmod(dir, 0o755);
        }
        // The server would leave a marker on the host if it could, and
        // speaks MCP's startup (U6e).
        let server = root.write(
            "opt/docs/bin/docs-mcp",
            &format!(
                "#!/bin/sh\ntouch '{}'\n{SPEAKS}",
                root.path.join("started").display()
            ),
        );
        chmod(&server, 0o755);
        // The operator's store, owner-only and outside both reach sets,
        // holding the declared binding (U6c5c, U6c8).
        root.store("store/secrets.env");
        Sealed {
            locator: attempt.join("cap-library-docs.json"),
            root,
            home,
            attempt,
            sources: std::sync::OnceLock::new(),
            _slot: slot,
        }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path.join(relative)
    }

    /// The plan the engine would seal for this attempt: its sources those
    /// the observer finds for it, where it finds any.
    fn plan(&self) -> Value {
        let mut plan = self.authored();
        let sources = self.sources.get_or_init(|| self.observed(&plan));
        set(&mut plan, "/box/sources", sources.clone());
        plan
    }

    /// `plan` with the sources the observer finds for it sealed, where it
    /// finds any; as it was where its observer refuses first.
    fn observing(&self, mut plan: Value) -> Value {
        let sources = self.observed(&plan);
        set(&mut plan, "/box/sources", sources);
        plan
    }

    /// The sources the observer finds for `plan`, sealed as it is, or the
    /// plan's own where it refuses before a box stands, as off Linux it
    /// always does.
    fn observed(&self, plan: &Value) -> Value {
        let digest = self.seal_bytes(plan.to_string().as_bytes());
        let (record, _) = self.observation(&digest);
        #[cfg(target_os = "linux")]
        observer::released(&record);
        match record.get("observed") {
            Some(observed) => {
                let fact = |name: &str| observed[name].clone();
                json!({"entries": fact("entries"), "mounts": fact("mounts"), "digest": fact("digest")})
            }
            None => plan["box"]["sources"].clone(),
        }
    }

    /// The record `broker observe` hands back for the plan sealed at
    /// `digest`, and the handles riding it: run as `serve` runs it, its
    /// stdout one end of a sequenced-packet pair this process made.
    #[cfg(target_os = "linux")]
    fn observation(&self, digest: &str) -> (Value, Vec<std::os::fd::OwnedFd>) {
        observer::relayed(self, digest, observer::Harness::Confined)
    }

    /// Off Linux no box stands, so no observer hands anything back.
    #[cfg(not(target_os = "linux"))]
    fn observation(&self, _: &str) -> (Value, Vec<std::os::fd::OwnedFd>) {
        (Value::Null, Vec::new())
    }

    /// The plan the engine would seal for this attempt, its sources as
    /// authored before any observation.
    fn authored(&self) -> Value {
        let path = |relative: &str| json!(self.path(relative));
        let digest = |byte: &str| json!(byte.repeat(64));
        let named = |dir: &Path| dir.file_name().unwrap().to_str().unwrap().to_owned();
        let attempt = named(&self.attempt);
        let repo = named(self.attempt.ancestors().nth(2).unwrap());
        json!({
            "owner": {"repo": repo, "run": "run-1", "effect": EFFECT, "attempt": attempt,
                      "site": "research", "instance": "research#0"},
            "server": "cap-library-docs", "capability": "library-docs",
            "dialect": {"name": "docs-mcp", "digest": digest("a"), "definition": digest("b"),
                        "version": "1.4.2"},
            "connection": {"argv": [path("opt/docs/bin/docs-mcp"), "--stdio"]},
            "tools": ["resolve", "read"], "restrictions": {}, "retained": true,
            "secrets": ["DOCS_TOKEN"],
            "clearance": {"dialect": digest("a"), "policy": digest("c")},
            "box": {
                "reach": {"writable": [path("work")], "readable": [path("cache")]},
                "executable": path("opt/docs/bin/docs-mcp"),
                "tree": {"kind": "package", "root": path("opt/docs")},
                "sources": {"entries": 12, "mounts": 3, "digest": digest("d")},
                "writers": {"uids": [euid()], "privilege": "confined"},
                "network": "shared",
                "environment": ["PATH", "HOME", "TMPDIR", "USER", "LOGNAME", "LANG", "LC_ALL",
                                "BROKKR_HANDS_BOX"],
                "bootstrap": {"path": bootstrap(), "digest": digest("e")},
                "excluded": {"store": path("store/secrets.env"), "control": [&self.attempt]}
            }
        })
    }

    /// Write `bytes` as the owner-only file `name` in the attempt.
    fn write(&self, name: &str, bytes: &[u8]) {
        let path = self.attempt.join(name);
        std::fs::write(&path, bytes).unwrap();
        chmod(&path, 0o600);
    }

    /// Write `pins` as the attempt's inventory.
    fn pin(&self, pins: &[(&Path, &str)]) {
        let plans: Vec<Value> = pins
            .iter()
            .map(|(locator, digest)| json!({"locator": locator, "digest": digest}))
            .collect();
        let inventory = json!({"plans": plans}).to_string();
        self.write("inventory.json", inventory.as_bytes());
    }

    /// Write `bytes` as the plan and pin them; their digest.
    fn seal_bytes(&self, bytes: &[u8]) -> String {
        let digest = brokkr_core::canonical::sha256_bytes(bytes);
        self.write("cap-library-docs.json", bytes);
        self.pin(&[(&self.locator, &digest)]);
        digest
    }

    /// The broker's exit and stderr for the plan at `locator` and `digest`,
    /// started by an engine whose HOME is `home`, or unset.
    fn serve_as(&self, home: Option<&Path>, locator: &Path, digest: &str) -> (Option<i32>, String) {
        self.serve_in(locator, digest, |command| {
            match home {
                Some(home) => command.env("HOME", home),
                None => command.env_remove("HOME"),
            };
        })
    }

    /// The broker's exit and stderr for the plan at `locator` and `digest`,
    /// its environment as `environment` leaves it: a fresh lifetime of the
    /// plan, as the engine provisions each, so its last ledger is gone.
    fn serve_in(
        &self,
        locator: &Path,
        digest: &str,
        environment: impl FnOnce(&mut Command),
    ) -> (Option<i32>, String) {
        std::fs::remove_file(ledger::of(locator)).ok();
        self.serve_again(locator, digest, environment)
    }

    /// The broker's exit and stderr for the plan at `locator` and `digest`
    /// as [`Sealed::serve_in`] has them, the plan's last ledger left.
    fn serve_again(
        &self,
        locator: &Path,
        digest: &str,
        environment: impl FnOnce(&mut Command),
    ) -> (Option<i32>, String) {
        let locator = locator.to_str().unwrap();
        let args = [
            "broker",
            "serve",
            "--plan",
            locator,
            "--plan-digest",
            digest,
        ];
        let mut command = self.root.harnessed(&args);
        environment(&mut command);
        let ran = self.root.ran(command);
        assert_eq!(ran.stdout, "");
        (ran.code, ran.stderr)
    }

    /// The broker's answer for the plan at `locator`, under this HOME.
    fn serve_at(&self, locator: &Path, digest: &str) -> (Option<i32>, String) {
        self.serve_as(Some(&self.home), locator, digest)
    }

    fn serve(&self, digest: &str) -> (Option<i32>, String) {
        self.serve_at(&self.locator, digest)
    }

    /// The broker's answer for the plan sealed at `digest` where no box
    /// can stand: no bubblewrap on its `PATH` (MB3), as off Linux no
    /// launcher mounts a descriptor at all. A launcher this run could plant
    /// would be a managed writer's, refused before it runs (U6c5c).
    fn serve_unboxed(&self, digest: &str) -> (Option<i32>, String) {
        let empty = self.path("unboxed");
        std::fs::create_dir_all(&empty).unwrap();
        self.serve_in(&self.locator, digest, |command| {
            command.env("HOME", &self.home).env("PATH", &empty);
        })
    }

    /// The broker's answer to `bytes`, sealed.
    fn answer_bytes(&self, bytes: &[u8]) -> (Option<i32>, String) {
        let digest = self.seal_bytes(bytes);
        self.serve(&digest)
    }

    /// The broker's answer to this attempt's plan once `edit` has changed
    /// it and the engine has sealed it.
    fn answer(&self, edit: impl FnOnce(&mut Value)) -> (Option<i32>, String) {
        let mut plan = self.plan();
        edit(&mut plan);
        self.answer_bytes(plan.to_string().as_bytes())
    }

    /// The broker's answer with the field at `pointer` sealed as `value`.
    fn with(&self, pointer: &str, value: Value) -> (Option<i32>, String) {
        self.answer(|plan| set(plan, pointer, value))
    }
}

/// A store the broker could only look a value up in by opening it: a FIFO
/// whose writer, a thread here, notes when a reader first opened it.
struct Store {
    path: PathBuf,
    writer: std::thread::JoinHandle<Instant>,
}

impl Store {
    fn new(path: PathBuf) -> Store {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // In place of the fixture's empty store, where it lies there.
        std::fs::remove_file(&path).ok();
        // Owner-only, as the store's own mode check requires.
        let made = Command::new("mkfifo")
            .args(["-m", "600"])
            .arg(&path)
            .status()
            .unwrap();
        assert_eq!(made.code(), Some(0));
        let fifo = path.clone();
        let writer = std::thread::spawn(move || {
            let mut opened = std::fs::OpenOptions::new().write(true).open(fifo).unwrap();
            let at = Instant::now();
            std::io::Write::write_all(&mut opened, b"DOCS_TOKEN=value\n").unwrap();
            at
        });
        Store { path, writer }
    }

    /// How many lookups opened the store before `ended`. A reader the
    /// broker opened released the writer then; otherwise only this
    /// release, after `ended`, does.
    fn lookups(self, ended: Instant) -> usize {
        let flags = rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NONBLOCK;
        let _release = rustix::fs::open(&self.path, flags, rustix::fs::Mode::empty()).unwrap();
        usize::from(self.writer.join().unwrap() < ended)
    }
}

#[test]
fn a_sealed_plan_is_admitted_and_a_store_without_identity_refused_unopened() {
    if let Some(reason) = unservable() {
        return skip(&reason);
    }
    // The protected store admits the plan, which still refuses before any
    // MCP message; nothing runs outside the box (MB4).
    let sealed = Sealed::new();
    assert_eq!(sealed.answer(|_| ()), admitted());
    assert!(!sealed.path("started").exists());
    // A store that is a FIFO has no identity, and is refused unopened.
    let store = Store::new(sealed.path("store/secrets.env"));
    let answer = sealed.answer(|_| ());
    let ended = Instant::now();
    assert_eq!(answer, past_prepare(Refusal::Identity));
    assert_eq!(store.lookups(ended), 0);
    assert!(!sealed.path("started").exists());
    // Nor has one that is absent, which admission does not make, or one
    // with a second link.
    let path = sealed.path("store/secrets.env");
    std::fs::remove_file(&path).unwrap();
    assert_eq!(sealed.answer(|_| ()), past_prepare(Refusal::Identity));
    assert!(!path.exists());
    sealed.root.store("store/secrets.env");
    std::fs::hard_link(&path, sealed.path("store/again")).unwrap();
    assert_eq!(sealed.answer(|_| ()), past_prepare(Refusal::Identity));
    std::fs::remove_file(sealed.path("store/again")).unwrap();
    assert_eq!(sealed.answer(|_| ()), admitted());
}

#[test]
fn only_the_protected_inventory_binds_a_plan() {
    let Some((sealed, unbound)) = unbound_fixture() else {
        return;
    };
    let bytes = sealed.plan().to_string();
    let digest = brokkr_core::canonical::sha256_bytes(bytes.as_bytes());
    sealed.write("cap-library-docs.json", bytes.as_bytes());
    // An authored copy with its true digest, outside the protected root.
    let authored = sealed.root.write("work/cap-library-docs.json", &bytes);
    assert_eq!(sealed.serve_at(&authored, &digest), unbound);
    // No inventory beside the plan: sealing it wrote one, so it goes. A
    // missing inventory is unbound, even where no box can stand either
    // (MB3: binding before the box).
    std::fs::remove_file(sealed.attempt.join("inventory.json")).unwrap();
    assert_eq!(sealed.serve(&digest), unbound);
    assert_eq!(sealed.serve_unboxed(&digest), unbound);
    // An inventory that pins another plan, another digest, or this one twice.
    let other = sealed.attempt.join("cap-other.json");
    sealed.pin(&[(&other, &digest)]);
    assert_eq!(sealed.serve(&digest), unbound);
    sealed.pin(&[(&sealed.locator, DIGEST)]);
    assert_eq!(sealed.serve(&digest), unbound);
    sealed.pin(&[(&sealed.locator, &digest), (&sealed.locator, &digest)]);
    assert_eq!(sealed.serve(&digest), unbound);
    // An inventory that does not parse closed.
    sealed.write("inventory.json", br#"{"plans": [], "ledgers": []}"#);
    assert_eq!(sealed.serve(&digest), unbound);
    // Pinned exactly, among others, it binds where each is another sealed
    // plan, which a retaining plan's share is counted over (U8b); a pin of
    // no plan leaves that share unknown, so the plan unbound.
    sealed.pin(&[(&other, DIGEST), (&sealed.locator, &digest)]);
    assert_eq!(sealed.serve(&digest), unbound);
    sealed.write("cap-other.json", bytes.as_bytes());
    sealed.pin(&[(&other, &digest), (&sealed.locator, &digest)]);
    binds("pinned", sealed.serve(&digest));
    // Where it binds, the box that cannot stand is the cause.
    if unservable().is_none() {
        let unavailable = refused(Refusal::Unavailable);
        assert_eq!(sealed.serve_unboxed(&digest), unavailable);
    }
}

#[test]
fn only_the_engines_home_roots_a_plan() {
    let Some((sealed, unbound)) = unbound_fixture() else {
        return;
    };
    let digest = sealed.seal_bytes(sealed.plan().to_string().as_bytes());
    let serve = |home: Option<&Path>| sealed.serve_as(home, &sealed.locator, &digest);
    // The root is compared by identity, not spelling: HOME reached through
    // a symlink is still the engine's.
    let link = sealed.path("link");
    std::os::unix::fs::symlink(&sealed.home, &link).unwrap();
    binds("linked home", serve(Some(&link)));
    // An engine with no HOME, or a relative one, roots nothing.
    assert_eq!(serve(None), unbound);
    assert_eq!(serve(Some(Path::new("home"))), unbound);
    // The same self-consistent, owner-only tree, pinned at its true
    // digest, is a lookalike under any other engine HOME.
    let host = sealed.path("host");
    let capabilities = host.join(".local/state/brokkr/capabilities");
    std::fs::create_dir_all(&capabilities).unwrap();
    chmod(&capabilities, 0o700);
    assert_eq!(serve(Some(&host)), unbound);
}

#[test]
fn a_plan_under_an_unprotected_root_or_file_is_unbound() {
    let Some((sealed, unbound)) = unbound_fixture() else {
        return;
    };
    let digest = sealed.seal_bytes(sealed.plan().to_string().as_bytes());
    let refuses = |path: &Path, mode: u32, restore: u32| {
        chmod(path, mode);
        let answer = sealed.serve(&digest);
        chmod(path, restore);
        assert_eq!((path, answer), (path, unbound.clone()));
        binds("restored", sealed.serve(&digest));
    };
    // The protected root and everything below it are this user's alone.
    refuses(
        &sealed.home.join(".local/state/brokkr/capabilities"),
        0o750,
        0o700,
    );
    refuses(&sealed.attempt, 0o701, 0o700);
    // An ancestor no one else may write.
    refuses(&sealed.home, 0o777, 0o755);
    // The plan and the inventory are owner-only files.
    refuses(&sealed.locator, 0o640, 0o600);
    refuses(&sealed.attempt.join("inventory.json"), 0o604, 0o600);
    // A second link to the plan.
    let alias = sealed.attempt.join("alias.json");
    std::fs::hard_link(&sealed.locator, &alias).unwrap();
    assert_eq!(sealed.serve(&digest), unbound);
    std::fs::remove_file(&alias).unwrap();
    // A symlinked plan, or a symlink on the way to it.
    let real = sealed.attempt.join("real.json");
    std::fs::rename(&sealed.locator, &real).unwrap();
    std::os::unix::fs::symlink(&real, &sealed.locator).unwrap();
    assert_eq!(sealed.serve(&digest), unbound);
    std::fs::remove_file(&sealed.locator).unwrap();
    std::fs::rename(&real, &sealed.locator).unwrap();
    binds("unlinked", sealed.serve(&digest));
    let run = sealed.attempt.parent().unwrap();
    std::fs::rename(run, run.with_file_name("run-0")).unwrap();
    std::os::unix::fs::symlink(run.with_file_name("run-0"), run).unwrap();
    assert_eq!(sealed.serve(&digest), unbound);
}

#[test]
fn a_plan_answers_for_the_attempt_it_lies_in_and_its_size() {
    let Some((sealed, unbound)) = unbound_fixture() else {
        return;
    };
    for (pointer, value) in [
        ("/owner/repo", json!("f".repeat(64))),
        ("/owner/run", json!("run-2")),
        (
            "/owner/attempt",
            json!("0d4c2b1a-6e5f-4a3b-8c7d-9e0f1a2b3c4d"),
        ),
        ("/owner/attempt", json!(ATTEMPT.to_uppercase())),
    ] {
        assert_eq!(
            (pointer, sealed.with(pointer, value)),
            (pointer, unbound.clone())
        );
    }
    // An attempt that is no portable path component binds nothing, even
    // where it names its own directory.
    assert_eq!(Sealed::at(DIGEST, "attempt 1").answer(|_| ()), unbound);
    // Nor does a repository that is no canonical sha256 (D6).
    for repo in ["not-a-digest", &DIGEST.to_uppercase()] {
        let answer = Sealed::at(repo, ATTEMPT).answer(|_| ());
        assert_eq!((repo, answer), (repo, unbound.clone()));
    }
    // A locator outside the protected layout binds nothing, even pinned.
    let other = sealed.home.join(".local/state/brokkr/other");
    let stray = attempt_under(&other, DIGEST, ATTEMPT);
    let mut plan = sealed.plan();
    set(&mut plan, "/box/excluded/control", json!([&stray]));
    let bytes = plan.to_string();
    let digest = brokkr_core::canonical::sha256_bytes(bytes.as_bytes());
    let locator = stray.join("cap-library-docs.json");
    for (name, text) in [
        ("cap-library-docs.json", bytes),
        (
            "inventory.json",
            json!({"plans": [{"locator": &locator, "digest": &digest}]}).to_string(),
        ),
    ] {
        std::fs::write(stray.join(name), text).unwrap();
        chmod(&stray.join(name), 0o600);
    }
    assert_eq!(sealed.serve_at(&locator, &digest), unbound);
    // MB3's request bound: one MiB of plan, and not a byte more.
    let mut bytes = sealed.plan().to_string().into_bytes();
    bytes.resize(1 << 20, b' ');
    binds("one MiB", sealed.answer_bytes(&bytes));
    bytes.push(b' ');
    assert_eq!(sealed.answer_bytes(&bytes), unbound);
}

#[test]
fn a_field_changed_after_sealing_acquires_no_authority() {
    let Some((sealed, unbound)) = unbound_fixture() else {
        return;
    };
    let digest = sealed.seal_bytes(sealed.plan().to_string().as_bytes());
    binds("as sealed", sealed.serve(&digest));
    let elsewhere = sealed.path("opt/other");
    for (pointer, value) in [
        ("/box/executable", json!(elsewhere.join("bin/docs-mcp"))),
        ("/box/tree", json!({"kind": "system"})),
        ("/box/tree/root", json!(elsewhere)),
        ("/box/reach/writable", json!([])),
        ("/box/sources/digest", json!("0".repeat(64))),
        // Another uid than the observed one on every host, whichever uid
        // the run has.
        ("/box/writers/uids", json!([euid() ^ 1])),
        ("/box/network", json!("isolated")),
        ("/box/bootstrap/digest", json!("0".repeat(64))),
        ("/clearance/policy", json!("0".repeat(64))),
        ("/connection/argv", json!(["/bin/sh", "-c", "true"])),
        ("/tools", json!(["resolve", "read", "admin"])),
        ("/retained", json!(false)),
    ] {
        let mut plan = sealed.plan();
        set(&mut plan, pointer, value);
        let bytes = plan.to_string();
        sealed.write("cap-library-docs.json", bytes.as_bytes());
        let altered = brokkr_core::canonical::sha256_bytes(bytes.as_bytes());
        // Every change moves the sealed bytes.
        assert_ne!((pointer, &altered), (pointer, &digest));
        // Neither the sealed digest nor the altered bytes' own is bound.
        assert_eq!((pointer, sealed.serve(&digest)), (pointer, unbound.clone()));
        assert_eq!(
            (pointer, sealed.serve(&altered)),
            (pointer, unbound.clone())
        );
    }
}

#[test]
fn a_plan_parses_closed_and_defaults_nothing() {
    let Some((sealed, unbound)) = unbound_fixture() else {
        return;
    };
    binds("as sealed", sealed.answer(|_| ()));
    let text = sealed.plan().to_string();
    let duplicated = text.replacen(
        "\"retained\":true",
        "\"retained\":true,\"retained\":false",
        1,
    );
    assert_ne!(duplicated, text);
    for bytes in [b"{\"owner\":".to_vec(), duplicated.into_bytes()] {
        assert_eq!(sealed.answer_bytes(&bytes), unbound);
    }
    for (pointer, value) in [
        ("/retained", json!("yes")),
        ("/restrictions", json!({"domains": ["docs.example"]})),
        ("/box/tree", json!({"kind": "workspace"})),
        ("/box/tree", json!({"kind": "system", "root": "/usr"})),
        ("/box/writers/privilege", json!("root")),
        // The network is a projection, never the egress word again.
        ("/box/network", json!("local")),
        ("/box/writers/uids", json!([-1])),
        // The engine's identities are strings, never a number.
        ("/owner/effect", json!(3)),
        ("/owner/attempt", json!(1)),
        ("/owner/instance", json!(0)),
    ] {
        assert_eq!(
            (pointer, sealed.with(pointer, value)),
            (pointer, unbound.clone())
        );
    }
    let unknown = |plan: &mut Value, at: &str| {
        plan.pointer_mut(at).unwrap()["mounts"] = json!(["/"]);
    };
    for at in ["", "/box", "/box/reach", "/box/excluded"] {
        assert_eq!(
            (at, sealed.answer(|plan| unknown(plan, at))),
            (at, unbound.clone())
        );
    }
    let missing = sealed.answer(|plan| {
        plan.as_object_mut().unwrap().remove("retained");
    });
    assert_eq!(missing, unbound);
}

/// Rewrite the record at `pointer` as the array of its fields in `order`.
fn positional(record: &mut Value, order: &[&str]) {
    let fields = order.iter().map(|field| record[*field].clone()).collect();
    *record = Value::Array(fields);
}

#[test]
fn every_record_is_an_object_never_a_positional_array() {
    let Some((sealed, unbound)) = unbound_fixture() else {
        return;
    };
    // Each record in its declared field order, which a positional reading
    // would take.
    let plan = [
        "owner",
        "server",
        "capability",
        "dialect",
        "connection",
        "tools",
        "restrictions",
        "retained",
        "secrets",
        "clearance",
        "box",
    ];
    let intent = [
        "reach",
        "executable",
        "tree",
        "sources",
        "writers",
        "network",
        "environment",
        "bootstrap",
        "excluded",
    ];
    let owner = ["repo", "run", "effect", "attempt", "site", "instance"];
    let records: [(&str, &[&str]); 13] = [
        ("", &plan),
        ("/owner", &owner),
        ("/dialect", &["name", "digest", "definition", "version"]),
        ("/connection", &["argv"]),
        ("/restrictions", &[]),
        ("/clearance", &["dialect", "policy"]),
        ("/box", &intent),
        ("/box/reach", &["writable", "readable"]),
        ("/box/tree", &["kind", "root"]),
        ("/box/sources", &["entries", "mounts", "digest"]),
        ("/box/writers", &["uids", "privilege"]),
        ("/box/bootstrap", &["path", "digest"]),
        ("/box/excluded", &["store", "control"]),
    ];
    for (pointer, order) in records {
        let answer = sealed.answer(|plan| positional(plan.pointer_mut(pointer).unwrap(), order));
        assert_eq!((pointer, answer), (pointer, unbound.clone()));
    }
    // The inventory and each pin, likewise.
    let digest = sealed.seal_bytes(sealed.plan().to_string().as_bytes());
    let pin = json!({"locator": &sealed.locator, "digest": &digest});
    let mut listed = pin.clone();
    positional(&mut listed, &["locator", "digest"]);
    for (name, inventory) in [
        ("inventory", json!([[pin]])),
        ("pin", json!({"plans": [listed]})),
    ] {
        sealed.write("inventory.json", inventory.to_string().as_bytes());
        assert_eq!((name, sealed.serve(&digest)), (name, unbound.clone()));
    }
    // The same pin as an object binds.
    sealed.write(
        "inventory.json",
        json!({"plans": [pin]}).to_string().as_bytes(),
    );
    binds("object pin", sealed.serve(&digest));
}

#[test]
fn no_public_decoder_reads_a_record_by_position() {
    use brokkr_protocol::broker::{Inventory, Owner, Restrictions, Tree};
    use serde::Deserialize;
    // Called as any caller outside the crate calls them, each record's
    // decoder refuses its fields by position.
    let message = |error: serde_json::Error| error.to_string();
    let positional = json!(["r", "run-1", EFFECT, ATTEMPT, "research", "research#0"]);
    let refusals = [
        Restrictions::deserialize(json!([])).map(drop),
        Owner::deserialize(positional).map(drop),
        Inventory::deserialize(json!([[]])).map(drop),
        Tree::deserialize(json!(["package", "/opt/docs"])).map(drop),
    ];
    let sequence = "invalid type: sequence, expected struct Fields";
    let tree = "invalid type: sequence, expected internally tagged enum Kind";
    assert_eq!(
        refusals.map(|refused| refused.map_err(message)),
        [sequence, sequence, sequence, tree].map(|text| Err(text.to_string()))
    );
    // The same records as objects are read, closed as ever.
    let owner = json!({"repo": "r", "run": "run-1", "effect": EFFECT, "attempt": ATTEMPT,
                       "site": "research", "instance": "research#0"});
    let read = Owner::deserialize(owner.clone()).unwrap();
    let fields = [
        read.repo,
        read.run,
        read.effect,
        read.attempt,
        read.site,
        read.instance,
    ];
    assert_eq!(
        fields,
        ["r", "run-1", EFFECT, ATTEMPT, "research", "research#0"]
    );
    let Restrictions {} = Restrictions::deserialize(json!({})).unwrap();
    assert_eq!(
        Inventory::deserialize(json!({"plans": []}))
            .unwrap()
            .plans
            .len(),
        0
    );
    let package = Tree::deserialize(json!({"kind": "package", "root": "/opt/docs"}));
    assert!(matches!(package.unwrap(), Tree::Package { root } if root == Path::new("/opt/docs")));
    let system = Tree::deserialize(json!({"kind": "system"})).unwrap();
    assert!(matches!(system, Tree::System {}));
    let mut unknown = owner.clone();
    unknown["mounts"] = json!([]);
    let mut missing = owner;
    missing.as_object_mut().unwrap().remove("instance");
    let closed = [
        Owner::deserialize(unknown).map(drop).map_err(message),
        Owner::deserialize(missing).map(drop).map_err(message),
        serde_json::from_str::<Tree>(r#"{"kind": "system", "kind": "system"}"#)
            .map(drop)
            .map_err(message),
    ];
    let expected = [
        "unknown field `mounts`, expected one of `repo`, `run`, `effect`, `attempt`, `site`, `instance`",
        "missing field `instance`",
        "duplicate field `kind` at line 1 column 25",
    ];
    assert_eq!(closed, expected.map(|text| Err(text.to_string())));
}

#[test]
fn a_sealed_plan_of_the_wrong_shape_is_unbound() {
    let Some((sealed, unbound)) = unbound_fixture() else {
        return;
    };
    let other = sealed.root.write("other-brokkr", "");
    for (pointer, value) in [
        ("/dialect/digest", json!("A".repeat(64))),
        ("/clearance/policy", json!("A".repeat(64))),
        ("/clearance/dialect", json!("c".repeat(64))),
        ("/box/bootstrap/path", json!(other)),
        (
            "/box/excluded/control",
            json!([sealed.attempt.parent().unwrap()]),
        ),
        ("/tools", json!([])),
        ("/tools", json!(["read", "read"])),
        ("/tools", json!([""])),
        ("/secrets", json!(["DOCS_TOKEN", "DOCS_TOKEN"])),
        ("/server", json!("")),
        ("/server", json!("cap-unrelated")),
        ("/owner/effect", json!("")),
        ("/owner/site", json!("")),
        ("/owner/instance", json!("")),
        ("/dialect/name", json!("")),
        ("/dialect/version", json!("")),
        ("/box/environment", json!([])),
        ("/box/environment", json!(["PATH", "PATH"])),
        ("/box/environment", json!(["PATH", "lower"])),
        // The server box's fixed names, in its order, and nothing else.
        (
            "/box/environment",
            json!([
                "HOME",
                "PATH",
                "TMPDIR",
                "USER",
                "LOGNAME",
                "LANG",
                "LC_ALL",
                "BROKKR_HANDS_BOX"
            ]),
        ),
        (
            "/box/environment",
            json!(["PATH", "HOME", "TMPDIR", "USER", "LOGNAME", "LANG", "LC_ALL"]),
        ),
        ("/connection/argv", json!([])),
        ("/connection/argv", json!([""])),
        ("/connection/argv", json!(["docs-mcp", ""])),
        ("/box/reach/readable", json!([sealed.path("cache/../opt")])),
        ("/box/excluded/store", json!("store/secrets.env")),
    ] {
        let case = format!("{pointer} = {value}");
        assert_eq!(
            (&case, sealed.with(pointer, value)),
            (&case, unbound.clone())
        );
    }
    // An empty capability, its server named after it.
    let unnamed = sealed.answer(|plan| {
        set(plan, "/capability", json!(""));
        set(plan, "/server", json!("cap-"));
    });
    assert_eq!(unnamed, unbound);
    // A plan with no secrets still binds.
    binds("no secrets", sealed.with("/secrets", json!([])));
}

#[test]
fn binding_names_are_checked_then_fixed_keys_refuse_before_lookup() {
    if !bindable() {
        return;
    }
    let sealed = Sealed::new();
    let named = |name: &str| sealed.with("/secrets", json!(["DOCS_TOKEN", name]));
    let cause = |name: &str| brokkr_protocol::secret::validate_name(name).unwrap_err();
    assert_eq!(
        cause("lower"),
        "secret name 'lower' does not match [A-Z][A-Z0-9_]*"
    );
    assert_eq!(named("lower"), refused(Refusal::Name(cause("lower"))));
    assert_eq!(
        named("LD_PRELOAD"),
        refused(Refusal::Name(cause("LD_PRELOAD")))
    );
    // A fixed key of the server box's own refuses before any lookup.
    let store = sealed.path("store/secrets.env");
    for fixed in ["HOME", "TMPDIR", "LANG", "USER"] {
        let opened = Store::new(store.clone());
        let answer = named(fixed);
        let lookups = opened.lookups(Instant::now());
        std::fs::remove_file(&store).unwrap();
        let startup = refused(Refusal::StartupInputs);
        assert_eq!((fixed, answer, lookups), (fixed, startup, 0));
    }
    // An invalid name outranks a fixed-key collision.
    let both = sealed.with("/secrets", json!(["HOME", "lower"]));
    assert_eq!(both, refused(Refusal::Name(cause("lower"))));
    // The plan's name cannot write a line of its own.
    let (code, stderr) = named("X\nerror: forged");
    assert_eq!(
        (code, stderr.as_str()),
        (
            Some(1),
            "error: secret name 'X\\nerror: forged' does not match [A-Z][A-Z0-9_]*\n"
        )
    );
    // NUL keeps its spelling of before the shared bound.
    let nul = concat!(
        "error: secret name 'X\\",
        "u{0}Y' does not match [A-Z][A-Z0-9_]*\n"
    );
    assert_eq!(named("X\0Y"), (Some(1), nul.to_string()));
}

/// The plan's name is escaped first, then the cause is cut to the shared
/// 512-scalar line bound, counted in scalars and not bytes, ending in `…`;
/// the CLI's prefix and newline lie outside it. The broker writes that
/// line cut into lines one write of POSIX's 512 bytes takes whole, their
/// newlines included, each on a character's boundary (U6f1b).
#[test]
fn a_long_binding_name_is_cut_to_one_bounded_line() {
    if !bindable() {
        return;
    }
    let sealed = Sealed::new();
    let name = format!("\n{}", "é".repeat(600));
    let (code, stderr) = sealed.with("/secrets", json!([name]));
    let cause = format!("secret name '\\n{}…", "é".repeat(496));
    assert_eq!(cause.chars().count(), 512);
    let (first, rest) = cause.split_at(cause.char_indices().nth(259).unwrap().0);
    assert_eq!(("error: ".len() + first.len(), rest.len()), (510, 507));
    let written = format!("error: {first}\n{rest}\n");
    assert_eq!((code, stderr), (Some(1), written));
}

/// An executable at `path` that speaks MCP's startup, its directories
/// made.
fn plant(path: &Path) -> PathBuf {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, format!("#!/bin/sh\n{SPEAKS}")).unwrap();
    chmod(path, 0o755);
    path.to_path_buf()
}

/// The program tree is resolved from the launch name on the host, never
/// read from the plan: the plan must have sealed the canonical executable
/// and MB3's tree of it that the server profile resolves (U6c4).
#[test]
fn the_program_tree_is_mb3s_layout_of_the_executable() {
    // Its refusals come after binding, which needs a protected base.
    if !bindable() {
        return;
    }
    let sealed = Sealed::new();
    // Each program is sealed with the sources the observer finds for it,
    // and told to speak MCP's startup: a shell runs it, a script ignores it.
    let tree = |argv0: Value, executable: &Path, tree: Value| {
        sealed.answer(|plan| {
            set(plan, "/connection/argv", json!([argv0, "-c", SPEAKS]));
            set(plan, "/box/executable", json!(executable));
            set(plan, "/box/tree", tree);
            *plan = sealed.observing(plan.take());
        })
    };
    let package = |root: &Path| json!({"kind": "package", "root": root});
    let system = || json!({"kind": "system"});
    let docs = sealed.path("opt/docs");
    let entry = docs.join("bin/docs-mcp");
    let link = sealed.path("docs-mcp");
    std::os::unix::fs::symlink(&entry, &link).unwrap();
    let sbin = plant(&docs.join("sbin/d"));
    let cargo = sealed.home.join(".cargo");
    let cargo_entry = plant(&cargo.join("bin/d"));
    let sh = std::fs::canonicalize("/bin/sh").unwrap();
    // A bare name is the first executable regular file on the fixed search
    // path, which need not be `/bin/sh`'s file where `/bin` and `/usr/bin`
    // are apart.
    let runnable =
        |file: &std::fs::Metadata| file.is_file() && file.permissions().mode() & 0o111 != 0;
    let bare = ["/usr/local/bin/sh", "/usr/bin/sh", "/bin/sh"]
        .into_iter()
        .find(|path| std::fs::metadata(path).is_ok_and(|file| runnable(&file)))
        .map(|path| std::fs::canonicalize(path).unwrap())
        .unwrap();
    // A system entry is its own tree, named bare on the fixed search path
    // or by a path; a package is its parent, or the parent of a `bin` or
    // `sbin`, whichever link the launch name took to it.
    for (argv0, executable, sealed_tree) in [
        (json!("sh"), &bare, system()),
        (json!("/bin/sh"), &sh, system()),
        (json!(entry), &entry, package(&docs)),
        (json!(link), &entry, package(&docs)),
        (json!(sbin), &sbin, package(&docs)),
        (json!(cargo_entry), &cargo_entry, package(&cargo)),
    ] {
        binds(
            &executable.display().to_string(),
            tree(argv0, executable, sealed_tree),
        );
    }
    let home_entry = plant(&sealed.home.join("d"));
    let root_entry = plant(&sealed.path("bin/d"));
    let missing = sealed.path("opt/none/bin/x");
    for (argv0, executable, sealed_tree) in [
        // A relative path holding a `/`; a bare name off the fixed path.
        (json!("bin/docs-mcp"), &entry, package(&docs)),
        (json!("docs-mcp"), &entry, package(&docs)),
        // A sealed executable that is not the canonical file, or a tree
        // that is not MB3's layout of it.
        (json!(link), &link, package(&docs)),
        (json!(entry), &entry, system()),
        (json!("sh"), &bare, package(bare.parent().unwrap())),
        (json!(entry), &entry, package(&docs.join("bin"))),
        // No file, or a root that is the host HOME or holds it.
        (json!(missing), &missing, package(missing.parent().unwrap())),
        (json!(home_entry), &home_entry, package(&sealed.home)),
        (json!(root_entry), &root_entry, package(&sealed.root.path)),
    ] {
        let answer = tree(argv0, executable, sealed_tree);
        let unresolved = refused(Refusal::ProgramTree);
        assert_eq!((executable, answer), (executable, unresolved));
    }
}

#[test]
fn the_box_neither_launches_from_nor_binds_over_the_seats_reach() {
    if !bindable() {
        return;
    }
    let sealed = Sealed::new();
    let inside = |executable: PathBuf| {
        plant(&executable);
        sealed.answer(|plan| {
            set(plan, "/connection/argv/0", json!(executable));
            set(plan, "/box/tree/root", json!(executable.parent().unwrap()));
            set(plan, "/box/executable", json!(executable));
        })
    };
    let overlapping = refused(Refusal::BindOverlapsReach);
    let launched = refused(Refusal::LaunchInReach);
    assert_eq!(inside(sealed.path("work/tool/server")), launched);
    assert_eq!(inside(sealed.path("cache/tool/server")), overlapping);
    // A launch name in writable reach, though it resolves outside it.
    let link = sealed.path("work/docs-mcp");
    std::os::unix::fs::symlink(sealed.path("opt/docs/bin/docs-mcp"), &link).unwrap();
    assert_eq!(sealed.with("/connection/argv/0", json!(link)), launched);
    // An explicit bind into a system source is reach the box keeps clear
    // of, and so is a root holding the box's generated identity: here the
    // broker's TMPDIR inside the seat's writable reach.
    let system = json!(["/usr/lib/u6c4-tool"]);
    assert_eq!(sealed.with("/box/reach/readable", system), overlapping);
    // Reach is cleared before the scratch is made: a TMPDIR in reach that
    // could hold no scratch still refuses as reach, not as identity.
    let tmp = sealed.path("work/tmp");
    std::fs::create_dir_all(&tmp).unwrap();
    let unusable = sealed.root.write("work/not-a-directory", "");
    let digest = sealed.seal_bytes(sealed.plan().to_string().as_bytes());
    for tmp in [tmp, unusable] {
        let answer = sealed.serve_in(&sealed.locator, &digest, |command| {
            command.env("HOME", &sealed.home).env("TMPDIR", &tmp);
        });
        assert_eq!((&tmp, answer), (&tmp, overlapping.clone()));
    }
    // A reach root inside the package, either kind, or over the bootstrap.
    for (pointer, root) in [
        ("/box/reach/writable", sealed.path("opt/docs/data")),
        ("/box/reach/readable", sealed.path("opt")),
        ("/box/reach/readable", bootstrap()),
    ] {
        let answer = sealed.with(pointer, json!([root]));
        assert_eq!((pointer, answer), (pointer, overlapping.clone()));
    }
}

#[test]
fn unprovable_or_unobserved_writers_and_sources_and_exposed_control_roots_refuse() {
    if let Some(reason) = unservable() {
        return skip(&reason);
    }
    let sealed = Sealed::new();
    // The plan's identity facts are checked once the box is prepared.
    let identity = past_prepare(Refusal::Identity);
    assert_eq!(sealed.answer(|_| ()), admitted());
    // Writers unmapped, none, or not the one observed (U6c5c: an observed
    // writer the plan does not list).
    let unmapped = [json!([]), json!([euid(), 65534]), json!([4_294_967_295u32])];
    for uids in unmapped.into_iter().chain([json!([euid() + 1])]) {
        let answer = sealed.with("/box/writers/uids", uids.clone());
        assert_eq!((&uids, answer), (&uids, identity.clone()));
    }
    // A sealed writer the observer cannot see joins the write-exclusion
    // proof beside the observed ones.
    let both = sealed.with("/box/writers/uids", json!([euid(), euid() + 1]));
    assert_eq!(both, admitted());
    // Each sealed source fact must be the observed one; the observer's own
    // bounds hold what it observes (MB3).
    let observed = sealed.plan()["box"]["sources"].clone();
    let one_more = |fact: &str| json!(observed[fact].as_u64().unwrap() + 1);
    for (pointer, value) in [
        ("/box/sources/entries", one_more("entries")),
        ("/box/sources/mounts", one_more("mounts")),
        ("/box/sources/digest", json!("0".repeat(64))),
    ] {
        let answer = sealed.with(pointer, value);
        assert_eq!((pointer, answer), (pointer, identity.clone()));
    }
    // The control roots lie outside the seat's reach and the box.
    assert_eq!(
        sealed.with("/box/reach/writable", json!([&sealed.home])),
        identity
    );
    let control = json!([&sealed.attempt, sealed.path("opt/docs/state")]);
    assert_eq!(sealed.with("/box/excluded/control", control), identity);
    // A box whose private scratch cannot be made generates no identity: it
    // is not established, the observer's cause crossing back whole.
    let file = sealed.root.write("not-a-directory", "");
    let digest = sealed.seal_bytes(sealed.plan().to_string().as_bytes());
    let answer = sealed.serve_in(&sealed.locator, &digest, |command| {
        command.env("HOME", &sealed.home).env("TMPDIR", &file);
    });
    assert_eq!(answer, past_prepare(Refusal::Establishment));
}

#[test]
fn the_store_is_neither_in_reach_nor_in_the_box() {
    if let Some(reason) = unservable() {
        return skip(&reason);
    }
    let sealed = Sealed::new();
    // Each store an empty owner-only file, so only where it lies differs;
    // the plan is sealed afresh with the sources it then observes.
    let store = |relative: Option<&str>| {
        let path = relative.map_or_else(bootstrap, |relative| sealed.root.store(relative));
        sealed.answer(|plan| {
            set(plan, "/box/excluded/store", json!(path));
            *plan = sealed.observing(plan.take());
        })
    };
    let reachable = past_prepare(Refusal::StoreReachable);
    assert_eq!(store(Some("work/.forge/secrets.env")), reachable);
    assert_eq!(store(Some("cache/secrets.env")), reachable);
    let mounted = past_prepare(Refusal::StoreInBox);
    assert_eq!(store(Some("opt/docs/secrets.env")), mounted);
    assert_eq!(store(None), mounted);
    // A store whose route passes through the hands' reach is theirs to
    // repoint, wherever it ends.
    std::os::unix::fs::symlink(sealed.path("store"), sealed.path("cache/store")).unwrap();
    assert_eq!(store(Some("cache/store/secrets.env")), reachable);
}

#[cfg(target_os = "linux")]
#[test]
fn every_managed_writer_is_observed_confined_and_sealed() {
    if let Some(reason) = unservable() {
        return skip(&reason);
    }
    let sealed = Sealed::new();
    let digest = sealed.seal_bytes(sealed.plan().to_string().as_bytes());
    assert_eq!(sealed.serve(&digest), admitted());
    // The harness that starts the broker is a managed writer too: one not
    // under `no_new_privs` leaves the writers unconfined, though the broker
    // and its observer are.
    let mut command = Command::new("/bin/sh");
    command.args(["-c", HARNESS, "setpriv", "--no-new-privs"]);
    command
        .arg(brokkr())
        .args(observer::serve_args(&sealed, &digest));
    command
        .env("HOME", &sealed.home)
        .current_dir(&sealed.root.path);
    let ran = sealed.root.ran(command);
    assert_eq!((ran.code, ran.stderr), refused(Refusal::Identity));
}
