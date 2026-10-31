//! Decision 0065 slice two, U8a2a, through the binary (CR2, MB3; R4 and R5
//! of run four's review): `brokkr run` dispatches boxed exec seats whose
//! composed command is the production one, `brokkr hands exec` around
//! `brokkr driver exec`, each launched by the dispatch door of the real
//! executable under its attempt's writer lease. With the compile fence
//! standing, no bundle retains, so evidence is put at stake as the fence
//! rule allows: a worktree's artifact root already holds published
//! artifacts, and once one such attempt is admitted its root is recorded
//! as historical for every later admission on this HOME.
//!
//! An admitted attacker strikes from inside its own box, while its
//! attempt holds its lease, at its own run's retained bytes, another
//! worktree's historical artifacts, the admission lock, a hard link and
//! its own artifact root, beside a legitimate worktree write; a second run
//! on its worktree, while it runs, is refused by its lease. Attackers whose
//! reach could replace a protected root — a bind of it, of its ancestor or
//! of a symlink alias, a hard link into the worktree, a bind of the HOME
//! holding the coordination root — are refused before anything of them
//! starts. Every protected file keeps its bytes, one link, and the digest
//! its name says. The coordination HOME lies outside every worktree, in
//! the target's own temporary directory, whose ancestors protect it as the
//! production walk asks. Linux only, needing bubblewrap's namespaces: a
//! seat's own box skips it with its reason, and the host runs it.

#![cfg(target_os = "linux")]

use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use brokkr_core::canonical;
use serde_json::{json, Value};

/// The coordination root below a HOME.
const COORDINATION: &str = ".local/state/brokkr/capabilities";

/// A work seat, then the review it leads to.
const POLICY: &str = r#"{"phases": ["work", "review", "done"], "initial": "work",
  "terminal": ["done"],
  "rules": [{"id": "WORK", "from": "work", "result": "complete", "next": "review", "reason": "w"},
            {"id": "CLEAN", "from": "review", "result": "clean", "next": "done", "reason": "r"}]}"#;

/// The longest the test waits on anything a run does.
const PATIENCE: Duration = Duration::from_secs(120);

/// The HOME every run here shares, owner-only, and the worktrees, bundles
/// and journals beside it. `machine_proof::release` shares it (U8e3).
pub(crate) struct Host {
    _dir: tempfile::TempDir,
    pub(crate) base: PathBuf,
}

impl Host {
    /// A fresh one where every ancestor protects a HOME, as the production
    /// walk judges them: the target's temporary directory, else TMPDIR,
    /// `/tmp` or `/var/tmp`. None where none does, the test then skipped
    /// with its reason: this proof is of production's resolution from HOME.
    pub(crate) fn new() -> Option<Host> {
        let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        let candidates = [
            target,
            std::env::temp_dir(),
            "/tmp".into(),
            "/var/tmp".into(),
        ];
        let guarded = candidates.iter().find(|at| guards(at));
        let Some(dir) = guarded.and_then(|at| tempfile::tempdir_in(at).ok()) else {
            use brokkr_protocol::hands::{boundary_evidence_required, skip_boundary_proof};
            let reason = "writer_lease: no location here protects a HOME (D6)";
            skip_boundary_proof(boundary_evidence_required(), reason);
            return None;
        };
        let base = std::fs::canonicalize(dir.path()).unwrap();
        for at in [base.clone(), base.join("home"), base.join("tmp")] {
            std::fs::create_dir_all(&at).unwrap();
            std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        Some(Host { _dir: dir, base })
    }

    fn coordination(&self) -> PathBuf {
        self.base.join("home").join(COORDINATION)
    }

    /// The worktree `name`, a recipe library, and the artifact its root
    /// publishes holding `evidence` where given.
    pub(crate) fn worktree(
        &self,
        name: &str,
        evidence: Option<&[u8]>,
    ) -> (PathBuf, Option<PathBuf>) {
        let work = self.base.join(name);
        std::fs::create_dir_all(&work).unwrap();
        crate::engine::library(&work);
        let published = evidence.map(|bytes| {
            let at = work.join(".forge/artifacts/sha256");
            std::fs::create_dir_all(&at).unwrap();
            let at = at.join(canonical::sha256_bytes(bytes));
            std::fs::write(&at, bytes).unwrap();
            at
        });
        (work, published)
    }

    /// The bundle `name`: its work and review seats exec seats boxed by
    /// the read-write `binds`, the work seat running `script` first.
    fn bundle(&self, name: &str, binds: &[&Path], script: &str) -> PathBuf {
        let binds: Vec<Value> = (binds.iter())
            .map(|path| json!({"path": path, "mode": "rw"}))
            .collect();
        let hands = json!({"kind": "workspace", "network": false, "binds": binds});
        self.seated(name, Some(hands), script)
    }

    /// [`Host::bundle`], its seats unboxed: they hold no hands.
    pub(crate) fn unboxed(&self, name: &str, script: &str) -> PathBuf {
        self.seated(name, None, script)
    }

    /// The bundle `name`, its seats holding `hands` where given.
    fn seated(&self, name: &str, hands: Option<Value>, script: &str) -> PathBuf {
        let bundle = self.base.join("bundles").join(name);
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
        std::fs::write(bundle.join("role.md"), "# seat\n").unwrap();
        let seat = |result: &str, before: &str| {
            let mut seat = json!({"results": [result], "role": "role.md",
                   "driver": {"command": ["{brokkr}", "driver", "exec", "--", "sh", "-c",
                                          answered(before, result), "seat", "{prompt_file}"]}});
            if let Some(hands) = &hands {
                seat["hands"] = hands.clone();
            }
            seat
        };
        let (work, review) = (seat("complete", script), seat("clean", ":"));
        let document = json!({"name": name, "policy": "policy.json",
                              "seats": {"work": work, "review": review}});
        std::fs::write(
            bundle.join("bundle.json"),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();
        bundle
    }

    /// `brokkr run` of `bundle` on `work`, journaled in its own journal,
    /// started under this HOME, its output to files of its own.
    pub(crate) fn start(&self, work: &Path, bundle: &Path) -> Running {
        let name = bundle.file_name().unwrap().to_str().unwrap();
        let said = self.base.join(format!("{name}.stderr"));
        let child = Command::new(env!("CARGO_BIN_EXE_brokkr"))
            .args(["run", "--bundle", bundle.to_str().unwrap()])
            .args([
                "--feature",
                "writer lease",
                "--db",
                self.journal(name).to_str().unwrap(),
            ])
            .args(["--repo", work.to_str().unwrap()])
            .current_dir(work)
            .env("HOME", self.base.join("home"))
            .env("TMPDIR", self.base.join("tmp"))
            .stdout(Stdio::null())
            .stderr(std::fs::File::create(&said).unwrap())
            .spawn()
            .unwrap();
        Running { child, said }
    }

    pub(crate) fn journal(&self, name: &str) -> PathBuf {
        self.base.join(format!("{name}.db"))
    }

    /// `bundle` run on `work` to its end: its run, and the events it
    /// journaled.
    pub(crate) fn ran(&self, work: &Path, bundle: &Path) -> (String, Vec<Value>) {
        let run = self.start(work, bundle).ended();
        let events = self.events(bundle, &run);
        (run, events)
    }

    /// Every event `run` of `bundle` journaled, exported.
    pub(crate) fn events(&self, bundle: &Path, run: &str) -> Vec<Value> {
        let name = bundle.file_name().unwrap().to_str().unwrap();
        let (db, out) = (self.journal(name), self.base.join(format!("{name}.export")));
        let exported = Command::new(env!("CARGO_BIN_EXE_brokkr"))
            .args(["export", "--run", run, "--out", out.to_str().unwrap()])
            .args(["--db", db.to_str().unwrap()])
            .current_dir(&self.base)
            .env("HOME", self.base.join("home"))
            .output()
            .unwrap();
        assert!(exported.status.success(), "{exported:?}");
        let ndjson = std::fs::read_to_string(out.join(format!("{run}.ndjson"))).unwrap();
        ndjson
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    /// Where `run` of `work` keeps its reservation.
    fn reservation(&self, work: &Path, run: &str) -> PathBuf {
        let key = canonical::sha256_bytes(work.as_os_str().as_encoded_bytes());
        self.coordination()
            .join(key)
            .join(run)
            .join("reservation.json")
    }

    /// The attempts `run` of `work` holds standing.
    pub(crate) fn standing(&self, work: &Path, run: &str) -> usize {
        let bytes = std::fs::read(self.reservation(work, run)).unwrap();
        let held: Value = serde_json::from_slice(&bytes).unwrap();
        held["attempts"].as_object().unwrap().len()
    }
}

/// A run's `brokkr`, killed and reaped if its test unwinds first.
pub(crate) struct Running {
    pub(crate) child: Child,
    said: PathBuf,
}

impl Running {
    /// The run it said it started, once it says it.
    pub(crate) fn run(&self) -> String {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let said = std::fs::read_to_string(&self.said).unwrap_or_default();
            let run = said
                .lines()
                .find_map(|line| line.strip_prefix("run started: "));
            if let Some(run) = run {
                return run.trim().to_string();
            }
            assert!(Instant::now() < deadline, "no run started: {said}");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Its run, once `brokkr` exited, within the patience.
    fn ended(mut self) -> String {
        let run = self.run();
        let deadline = Instant::now() + PATIENCE;
        while self.child.try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "{run} never ended");
            std::thread::sleep(Duration::from_millis(20));
        }
        run
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

/// The seat's shell: `before`, then the result `result` written where the
/// prompt file, its first argument, names the result contract's file.
fn answered(before: &str, result: &str) -> String {
    let contract = r#"grep -o '/[^ ]*/results/[^ ]*\.json' "$1" | tail -n 1"#;
    format!(
        "{before}\nprintf '{{\"result\":\"{result}\",\"inputs\":{{}},\"notes\":\"n\"}}' \
         > \"$({contract})\""
    )
}

/// Wait up to the patience for `holds`.
pub(crate) fn until(what: &str, holds: impl Fn() -> bool) {
    let deadline = Instant::now() + PATIENCE;
    while !holds() {
        assert!(Instant::now() < deadline, "{what} never held");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The error every failed attempt of `events` journaled.
pub(crate) fn failures(events: &[Value]) -> Vec<String> {
    let failed = events
        .iter()
        .filter(|event| event["type"] == "effect/failed");
    failed
        .map(|event| event["payload"]["error"].as_str().unwrap().to_string())
        .collect()
}

/// Whether any attempt of `events` succeeded.
pub(crate) fn succeeded(events: &[Value]) -> bool {
    events
        .iter()
        .any(|event| event["type"] == "effect/succeeded")
}

/// CR2's cause, as dispatch journals it.
pub(crate) fn cr2(conflict: &str) -> String {
    format!(
        "dispatch refused: capability evidence root cannot be protected from every managed \
         writer: {conflict}"
    )
}

/// CR2's refusal of the work site of `run` reaching `root`.
fn reaching(run: &str, root: &Path) -> String {
    let root = root.display();
    cr2(&format!(
        "site 'work' of run '{run}' can reach '{root}', which evidence or coordination depends on"
    ))
}

/// Whether this host builds a real box; the test skipped with its reason
/// where it does not, as in a seat's own box.
fn boxes_here() -> bool {
    use brokkr_protocol::hands::{boundary_evidence_required, skip_boundary_proof, HANDS_BOX_ENV};
    let probe = Command::new("bwrap")
        .args(["--ro-bind", "/", "/", "true"])
        .output();
    let boxes =
        probe.is_ok_and(|ran| ran.status.success()) & std::env::var_os(HANDS_BOX_ENV).is_none();
    if !boxes {
        let reason = "writer_lease: no bubblewrap namespace here";
        skip_boundary_proof(boundary_evidence_required(), reason);
    }
    boxes
}

/// Whether `at`, canonical, and each directory above it is this user's or
/// root's and writable by no one else, unless root's and sticky.
fn guards(at: &Path) -> bool {
    let user = std::fs::metadata("/proc/self").map(|own| own.uid());
    let Ok(canonical) = std::fs::canonicalize(at) else {
        return false;
    };
    canonical.ancestors().all(|dir| {
        let Ok(found) = std::fs::metadata(dir) else {
            return false;
        };
        let owner = (found.uid() == 0) | user.as_ref().is_ok_and(|user| *user == found.uid());
        let sticky = (found.uid() == 0) & (found.mode() & 0o1000 != 0);
        owner & ((found.mode() & 0o022 == 0) | sticky)
    })
}

/// The device and inode of what stands at `at`, unfollowed.
fn identity(at: &Path) -> (u64, u64) {
    let found = std::fs::symlink_metadata(at).unwrap();
    (found.dev(), found.ino())
}

/// The file at `at` holds `bytes` in one link, and its name is their
/// digest.
fn intact(at: &Path, bytes: &[u8], what: &str) {
    assert_eq!(std::fs::read(at).unwrap(), bytes, "{what}: its bytes");
    assert_eq!(
        std::fs::metadata(at).unwrap().nlink(),
        1,
        "{what}: one link"
    );
    let name = at.file_name().unwrap().to_str().unwrap();
    assert_eq!(canonical::sha256_bytes(bytes), name, "{what}: its digest");
}

/// A box binding `home`, which holds the coordination root, refuses at the
/// door by MB3's identity cause, on every run (operator ruling of 2026-10-30),
/// its driver never started.
fn refused_at_the_door(host: &Host, home: &Path) {
    let (attacked, _) = host.worktree("coordination", None);
    let mark = attacked.join("ran");
    let touch = format!("touch '{}'", mark.display());
    let bundle = host.bundle("coordination", &[home], &touch);
    let (run, events) = host.ran(&attacked, &bundle);
    let refused = format!(
        "dispatch refused: MCP server box filesystem identity is not protected: site 'work' of \
         run '{run}' can write '{}', which an MCP server's box or lease rests on",
        host.coordination().display()
    );
    assert_eq!(failures(&events), [refused]);
    assert!(!mark.exists(), "its driver never started");
}

#[test]
fn a_real_boxed_exec_strikes_from_inside_its_lease_and_every_protected_byte_stands() {
    let Some(host) = boxes_here().then(Host::new).flatten() else {
        return;
    };
    // Another worktree's evidence, recorded historical by its admission.
    let (other, historical) = host.worktree("other", Some(b"historical"));
    let historical = historical.unwrap();
    let (_, events) = host.ran(&other, &host.bundle("recorder", &[], ":"));
    assert!(succeeded(&events), "the recorder ran: {events:?}");
    let lock = host.coordination().join("admission.lock");
    let locked = std::fs::read(&lock).unwrap();

    // The attacker, admitted, waits under its lease, then strikes.
    let (work, retained) = host.worktree("work", Some(b"retained"));
    let retained = retained.unwrap();
    let artifacts = work.join(".forge/artifacts");
    let root_was = identity(&artifacts);
    let forge = |at: &Path| format!("printf forged > '{}' 2>/dev/null", at.display());
    let strike = [
        format!("touch '{}'", work.join("started").display()),
        format!(
            "while [ ! -e '{}' ]; do sleep 0.05; done",
            work.join("go").display()
        ),
        forge(&retained),
        forge(&historical),
        forge(&lock),
        format!(
            "ln '{}' '{}' 2>/dev/null",
            retained.display(),
            work.join("linked").display()
        ),
        format!(
            "mv '{}' '{}' 2>/dev/null",
            artifacts.display(),
            work.join("gone").display()
        ),
        format!("echo w > '{}'", work.join("made").display()),
    ];
    let attacker = host.start(&work, &host.bundle("attacker", &[], &strike.join("; ")));
    let attacking = attacker.run();
    until("the attacker runs in its box", || {
        work.join("started").exists()
    });
    assert_eq!(host.standing(&work, &attacking), 1, "its lease stands");

    // An already-running writer refuses a second run on its worktree.
    let mark = work.join("second.ran");
    let second = host.bundle("second", &[], &format!("touch '{}'", mark.display()));
    let (_, events) = host.ran(&work, &second);
    let held = format!("run '{attacking}' holds this worktree, its writers not proven settled");
    assert_eq!(failures(&events), [cr2(&held)]);
    assert!(!mark.exists(), "the second run's driver never started");

    std::fs::write(work.join("go"), "").unwrap();
    let ended = attacker.ended();
    let events = host.events(&host.base.join("bundles/attacker"), &ended);
    assert!(succeeded(&events), "the attacker's box ran: {events:?}");
    intact(&retained, b"retained", "its own retained bytes");
    intact(&historical, b"historical", "the historical artifact");
    assert_eq!(std::fs::read(&lock).unwrap(), locked, "the admission lock");
    // A move across the root's read-only mount falls back to a copy, which
    // reads it; the root itself is the directory it was.
    assert_eq!(identity(&artifacts), root_was, "its artifact root");
    assert!(!work.join("linked").exists(), "no link was made");
    let made = std::fs::read(work.join("made")).ok();
    assert_eq!(
        made.as_deref(),
        Some(&b"w\n"[..]),
        "the worktree write landed"
    );
    assert_eq!(host.standing(&work, &attacking), 0, "released once settled");

    // Reach that could replace a protected root refuses before it starts.
    let root = historical.parent().unwrap().parent().unwrap().to_path_buf();
    let state = root.parent().unwrap().to_path_buf();
    let alias = host.base.join("alias");
    std::os::unix::fs::symlink(&state, &alias).unwrap();
    let home = host.base.join("home");
    let corrupt = cr2(&format!(
        "the artifact '{}' is not one file of one link holding the bytes its name digests",
        historical.display()
    ));
    // Each case's read-write bind, and the root it reaches; the hard link
    // binds nothing, and is refused as the artifact it corrupts.
    let cases: [(&str, Option<&Path>, Option<PathBuf>); 4] = [
        ("bind", Some(&state), Some(root.clone())),
        ("ancestor", Some(&other), Some(root.clone())),
        ("symlink", Some(&alias), Some(root.clone())),
        ("hardlink", None, None),
    ];
    refused_at_the_door(&host, &home);
    for (case, bind, reached) in cases {
        let (attacked, _) = host.worktree(case, None);
        if bind.is_none() {
            std::fs::hard_link(&historical, attacked.join("linked")).unwrap();
        }
        let mark = attacked.join("ran");
        let binds: Vec<&Path> = bind.into_iter().collect();
        let bundle = host.bundle(case, &binds, &format!("touch '{}'", mark.display()));
        let (run, events) = host.ran(&attacked, &bundle);
        let expected = reached.map_or_else(|| corrupt.clone(), |root| reaching(&run, &root));
        assert_eq!(failures(&events), [expected], "{case}");
        assert!(!mark.exists(), "{case}: its driver never started");
        std::fs::remove_file(attacked.join("linked")).ok();
        intact(&historical, b"historical", case);
        intact(&retained, b"retained", case);
    }
}
