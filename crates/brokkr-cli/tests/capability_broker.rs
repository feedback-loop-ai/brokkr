//! `brokkr broker serve` (decision 0065 slice two U6b and U6c; MB3, MB4,
//! SC1, SD3), driven through the real binary. The command takes a bounded
//! plan locator and digest and nothing else: no server argv, grant or
//! secret value is an option. A plan is bound only where the attempt's
//! protected inventory pins it; a bound plan's every field and box intent
//! is checked in MB3's order, and even an admitted plan still refuses
//! before any lookup or start until the serving protections land. The
//! compile fence still refuses every MCP grant.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use brokkr_protocol::broker::Refusal;
use serde_json::{json, Value};

#[cfg(target_os = "linux")]
mod observer;

const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

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
fn brokkr() -> &'static Path {
    static COPY: std::sync::OnceLock<(PathBuf, std::fs::File)> = std::sync::OnceLock::new();
    let (copy, _live) = COPY.get_or_init(|| {
        if !boxed() {
            sweep();
        }
        let dir = tempfile::Builder::new().prefix(COPIES).tempdir_in(base());
        let dir = dir.unwrap().keep();
        // Locked before it takes the name a sweep looks for.
        let live = std::fs::File::create(dir.join("lock.new")).unwrap();
        assert!(lock(&live));
        std::fs::rename(dir.join("lock.new"), dir.join("lock")).unwrap();
        let copy = dir.join("brokkr");
        std::fs::copy(env!("CARGO_BIN_EXE_brokkr"), &copy).unwrap();
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

    fn brokkr(&self, args: &[&str]) -> Ran {
        self.ran(self.command(args))
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = confined();
        command.args(args).current_dir(&self.path);
        command
    }

    fn ran(&self, mut command: Command) -> Ran {
        let out = command.output().unwrap();
        Ran {
            code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }
    }
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

#[test]
fn the_grouped_library_verbs_still_dispatch() {
    let root = Root::new();
    let store = root.write("secrets.env", "DOCS_TOKEN=value\n");
    let private = std::os::unix::fs::PermissionsExt::from_mode(0o600);
    std::fs::set_permissions(&store, private).unwrap();
    let ran = root.brokkr(&["secrets", "list", "--secrets-file", "secrets.env"]);
    assert_eq!((ran.code, ran.stdout.as_str()), (Some(0), "DOCS_TOKEN\n"));
}

/// `brokkr init`'s workspace, whose map grants its one realm
/// `library-docs` through an MCP dialect that no seat asks for.
fn granting_mcp(root: &Root) {
    assert_eq!(root.brokkr(&["init", "."]).code, Some(0));
    let map = root.path.join("realms.json");
    let mut realms: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&map).unwrap()).unwrap();
    realms["schema"] = json!("forge.realms/v6");
    realms["realms"][0]["capabilities"] =
        json!({"library-docs": {"dialect": "docs-mcp", "offices": []}});
    root.write("realms.json", &realms.to_string());
    root.write(
        "capabilities/library-docs.json",
        &json!({"name": "library-docs", "classes": ["reads", "egress"]}).to_string(),
    );
    root.write(
        "dialects/tools/docs-mcp.json",
        &json!({"schema": "brokkr.tool-dialect/v1", "name": "docs-mcp",
                "serves": "library-docs", "kind": "mcp",
                "connection": {"argv": ["/nonexistent/docs-mcp"]}, "version": "1.4.2",
                "secrets": ["DOCS_TOKEN"], "tools": ["resolve", "read"], "retained": true,
                "sends": {"description": "a library name", "seat_composed": true}})
        .to_string(),
    );
}

#[test]
fn the_compile_fence_still_refuses_an_unused_mcp_grant() {
    let root = Root::new();
    granting_mcp(&root);
    let ran = root.brokkr(&["compile", "--bundle", "."]);
    assert_eq!(ran.code, Some(1));
    assert_eq!(
        ran.stderr,
        "error: bundle: realm 'starter' grants capability 'library-docs' through dialect 'docs-mcp' of \
         kind 'mcp', whose broker support is not implemented until decision 0065 slice two\n"
    );
    assert_eq!(ran.stdout, "");
}

/// Each refusal's text, pinned once: MB3 and MB4's exact causes and SD3's
/// temporary serving cause, listed in MB3's precedence, which is their
/// order.
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
            Refusal::ServingIncomplete,
            "broker serving protections are incomplete",
        ),
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

/// The answer to a plan the broker admits, still refused before serving.
fn admitted() -> (Option<i32>, String) {
    past_prepare(Refusal::ServingIncomplete)
}

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
}

impl Sealed {
    fn new() -> Sealed {
        Sealed::at(DIGEST, ATTEMPT)
    }

    /// An attempt sealed in the directory `name` of repository `repo`.
    fn at(repo: &str, name: &str) -> Sealed {
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
        // The server leaves a marker if it is ever started.
        let server = root.write(
            "opt/docs/bin/docs-mcp",
            &format!(
                "#!/bin/sh\ntouch '{}'\n",
                root.path.join("started").display()
            ),
        );
        chmod(&server, 0o755);
        Sealed {
            locator: attempt.join("cap-library-docs.json"),
            root,
            home,
            attempt,
            sources: std::sync::OnceLock::new(),
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
        let (ours, theirs) = observer::pair();
        let locator = self.locator.to_str().unwrap();
        let args = [
            "broker",
            "observe",
            "--plan",
            locator,
            "--plan-digest",
            digest,
        ];
        let mut command = self.root.command(&args);
        command
            .env("HOME", &self.home)
            .stdin(std::process::Stdio::null());
        command.stdout(theirs).stderr(std::process::Stdio::null());
        command.status().unwrap();
        drop(command);
        observer::received(&ours)
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
    /// its environment as `environment` leaves it.
    fn serve_in(
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
        let mut command = self.root.command(&args);
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
    /// can stand: first on its `PATH`, a bubblewrap older than descriptor
    /// mounts (MB3), as off Linux no launcher mounts one at all.
    fn serve_unboxed(&self, digest: &str) -> (Option<i32>, String) {
        let old = self
            .root
            .write("old/bwrap", "#!/bin/sh\necho bubblewrap 0.4.0\n");
        chmod(&old, 0o755);
        let path = std::env::var("PATH").unwrap();
        let path = format!("{}:{path}", self.path("old").display());
        self.serve_in(&self.locator, digest, |command| {
            command.env("HOME", &self.home).env("PATH", path);
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
fn a_sealed_plan_is_admitted_and_still_refused_before_any_lookup_or_start() {
    if let Some(reason) = unservable() {
        return skip(&reason);
    }
    let sealed = Sealed::new();
    let store = Store::new(sealed.path("store/secrets.env"));
    let answer = sealed.answer(|_| ());
    let ended = Instant::now();
    assert_eq!(answer, admitted());
    // Zero lookups and zero starts.
    assert_eq!(store.lookups(ended), 0);
    assert!(!sealed.path("started").exists());
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
    // Pinned exactly, among others, it binds.
    sealed.pin(&[(&other, DIGEST), (&sealed.locator, &digest)]);
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
/// the CLI's prefix and newline lie outside it.
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
    assert_eq!((code, stderr), (Some(1), format!("error: {cause}\n")));
}

/// An executable at `path`, its directories made.
fn plant(path: &Path) -> PathBuf {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "#!/bin/sh\n").unwrap();
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
    // Each program is sealed with the sources the observer finds for it.
    let tree = |argv0: Value, executable: &Path, tree: Value| {
        sealed.answer(|plan| {
            set(plan, "/connection/argv/0", argv0);
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
    // Writers unmapped, none, or another beside the one observed.
    let unmapped = [json!([]), json!([euid(), 65534]), json!([4_294_967_295u32])];
    for uids in unmapped.into_iter().chain([json!([euid(), euid() + 1])]) {
        let answer = sealed.with("/box/writers/uids", uids.clone());
        assert_eq!((&uids, answer), (&uids, identity.clone()));
    }
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
    // A box whose private scratch cannot be made generates no identity.
    let file = sealed.root.write("not-a-directory", "");
    let digest = sealed.seal_bytes(sealed.plan().to_string().as_bytes());
    let answer = sealed.serve_in(&sealed.locator, &digest, |command| {
        command.env("HOME", &sealed.home).env("TMPDIR", &file);
    });
    assert_eq!(answer, refused(Refusal::Identity));
}

#[test]
fn the_store_is_neither_in_reach_nor_in_the_box() {
    if let Some(reason) = unservable() {
        return skip(&reason);
    }
    let sealed = Sealed::new();
    let store = |path: PathBuf| sealed.with("/box/excluded/store", json!(path));
    let reachable = past_prepare(Refusal::StoreReachable);
    assert_eq!(store(sealed.path("work/.forge/secrets.env")), reachable);
    assert_eq!(store(sealed.path("cache/secrets.env")), reachable);
    let mounted = past_prepare(Refusal::StoreInBox);
    assert_eq!(store(sealed.path("opt/docs/secrets.env")), mounted);
    assert_eq!(store(bootstrap()), mounted);
}
