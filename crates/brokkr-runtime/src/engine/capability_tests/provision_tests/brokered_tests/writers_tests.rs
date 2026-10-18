//! U8a2a's attack proofs (D6's last paragraph; CR2, MB3; M7): a retaining
//! panel member, its holding brokered on the fixture's protected HOME with
//! the compile fence lifted, beside a zero-grant attacker, a boxed exec,
//! an already-running writer that really dispatched, settled historical
//! evidence, an ancestor's replacement, a symlink alias, a bind alias and a
//! hard link, alongside legitimate worktree writes. Each attacker would
//! change the protected bytes if it started; each proof asserts those bytes
//! first, then that nothing of the attempt started.
//!
//! The matrix dispatches every attacker through the dispatch door with the
//! fence lifted. One the comparison admits starts under its attempt's
//! lease, beside the keeper, and builds its own box as `brokkr hands exec`
//! builds it — `hands::run_boxed` over the spec, workdir and artifact root
//! of the argv the engine composes for an exec seat of its hands — and runs
//! its attack in that box while the attempt still holds its reservation
//! ([`boxed_attacker`]). This test binary cannot itself stand in for
//! `brokkr`, so the attacker's driver runs it on that one test.

use std::path::PathBuf;

use brokkr_core::canonical;

use super::*;
use crate::bundle::{Invocation, PanelMember};
use crate::capabilities::McpFence;
use crate::engine::broker::admission::tests::{driver, hands, last};
use crate::engine::broker::admission::{Conflict, Unadmitted};
use crate::engine::tests::{engine, set_site_hands};

/// The panel seat: the retaining `keeper`, one boxed inline Claude site,
/// and the `attacker`, an inline site the test arms.
fn seat() -> Value {
    let attacker = json!({"role": "roles/role.md", "driver": {"command": ["sh", "-c", ":"]}});
    json!({"results": ["pass", "fail"], "aggregate": "unanimous-pass",
           "panel": {"keeper": site(), "attacker": attacker}})
}

/// The bytes a retained response was published as, in the run-local
/// artifact root of `engine`.
fn evidence(engine: &Engine) -> PathBuf {
    let root = crate::engine::boxed::artifact_root(&engine.run_local());
    let artifact = root
        .join("sha256")
        .join(canonical::sha256_bytes(b"retained"));
    std::fs::create_dir_all(artifact.parent().unwrap()).unwrap();
    std::fs::write(&artifact, "retained").unwrap();
    artifact
}

/// The attacker of `engine` armed to run `attack` first, with `spec` hands
/// or none: its site's facts are those hands alone, and it is told no
/// charter, as a shell, not a seat.
fn armed(engine: &mut Engine, attack: &str, spec: Option<HandsSpec>) {
    let attacker = member(engine, "attacker");
    (attacker.command, attacker.role_path) = (driver(attack), PathBuf::new());
    match spec {
        Some(spec) => set_site_hands(&mut engine.bundle, "work:attacker", spec),
        None => {
            engine.bundle.hands.remove("work:attacker");
            let facts = crate::bundle::SiteFacts::default();
            engine.bundle.sites.insert("work:attacker".into(), facts);
        }
    }
}

/// The panel member `name` of `engine`'s work seat.
fn member<'e>(engine: &'e mut Engine, name: &str) -> &'e mut PanelMember {
    let body = &mut engine.bundle.seats.get_mut("work").unwrap().body;
    let SeatBody::Invocation(Invocation::Panel { members, .. }) = body else {
        panic!("the seat is a panel");
    };
    members
        .iter_mut()
        .find(|member| member.name == name)
        .unwrap()
}

/// CR2's refusal of the attacker of `claude`'s run reaching `root`.
fn reaching(claude: &Claude, root: &Path) -> Conflict {
    Conflict::Reaches {
        site: "work:attacker".into(),
        run: claude.fixture.engine.run_id.clone(),
        root: root.to_path_buf(),
    }
}

impl Claude {
    /// The retaining panel, its keeper the [`recording`] driver, on any
    /// TMPDIR (M5): admission opens the coordination root it is handed with
    /// no ancestor judged, and provisioning reaches the keeper's plans from
    /// that same handle.
    fn retaining() -> Claude {
        Claude::retaining_on(Fixture::inline_in(&scratch()))
    }

    /// [`Claude::retaining`] on `fixture`.
    fn retaining_on(fixture: Fixture) -> Claude {
        let mut claude = Claude::of(fixture, seat(), &["work:keeper"]);
        let root = claude.fixture.root.clone();
        member(&mut claude.fixture.engine, "keeper").command[0] = recording(&root);
        claude
    }

    /// The work seat's effect executed as dispatch executes it, with the
    /// fence lifted, on the fixture's stood-in host.
    fn attacked(&mut self) {
        let _lift = McpFence::lift();
        self.executed_as_is();
    }

    /// The protected `artifact` still holds `bytes`, and the attempt failed
    /// by CR2's `conflict` before anything of it started: no driver was
    /// spawned, and nothing was provisioned.
    fn refused(&self, conflict: Conflict, (artifact, bytes): (&Path, &[u8])) {
        let kept = std::fs::read(artifact).unwrap();
        assert_eq!(kept, bytes, "the protected bytes");
        let (event, payload) = last(&self.fixture.engine);
        assert_eq!(event, EventType::EffectFailed, "{payload}");
        assert_eq!(payload["error"], Unadmitted::Evidence(conflict).to_string());
        let spawned = self.fixture.root.join("driver.argv").exists();
        assert!(!spawned, "the keeper never started");
        let sealed = self.fixture.run_dir().exists();
        assert!(!sealed, "nothing was sealed or provisioned");
    }
}

/// Another worktree's run whose artifact root holds the evidence `bytes`,
/// run once with hands on the fixture's HOME, so its root is recorded and
/// its attempt settled: the evidence's path, and its engine.
#[cfg(target_os = "linux")]
fn settled(claude: &Claude, bytes: &[u8]) -> (PathBuf, (tempfile::TempDir, Engine)) {
    // Beside the fixture, on its filesystem, so a hard link can reach it.
    let dir = tempfile::tempdir_in(&claude.fixture.root).unwrap();
    let body = crate::engine::tests::single_body(driver(":"));
    let (dir, mut other) = crate::engine::capability_tests::canonical_engine_in(dir, body);
    let root = crate::engine::boxed::artifact_root(&other.run_local());
    let artifact = root.join("sha256").join(canonical::sha256_bytes(bytes));
    std::fs::create_dir_all(artifact.parent().unwrap()).unwrap();
    std::fs::write(&artifact, bytes).unwrap();
    set_site_hands(&mut other.bundle, "work", hands(&[]));
    let _lift = McpFence::lift();
    let _host = StoodIn::new(claude.fixture.host());
    let effect = requested(&other, "effect");
    execute_work(&mut other, &[effect]).unwrap();
    let (event, payload) = last(&other);
    assert_eq!(event, EventType::EffectSucceeded, "{payload}");
    (std::fs::canonicalize(artifact).unwrap(), (dir, other))
}

/// A real engine's run on a worktree of its own, dispatched on a thread of
/// its own on `claude`'s host: its one driver, outside any box, waits until
/// let go, its attempt's lease held all the while (M7).
struct Waiting {
    run: String,
    gate: PathBuf,
    writer: Option<std::thread::JoinHandle<EventType>>,
    _dir: tempfile::TempDir,
}

impl Waiting {
    /// The run started, once its driver began.
    fn on(claude: &Claude) -> Waiting {
        let (dir, mut running) = engine(crate::engine::tests::single_body(Vec::new()));
        let (started, gate) = (dir.path().join("started"), dir.path().join("gate"));
        let waits = format!(
            "touch '{}'; while [ ! -e '{}' ] && [ -d '{}' ]; do sleep 0.05; done",
            started.display(),
            gate.display(),
            dir.path().display()
        );
        running.bundle.seats.get_mut("work").unwrap().body =
            crate::engine::tests::single_body(driver(&waits));
        let (host, run) = (claude.fixture.host(), running.run_id.clone());
        let writer = std::thread::spawn(move || {
            let _host = StoodIn::new(host);
            let effect = requested(&running, "effect");
            execute_work(&mut running, &[effect]).unwrap();
            last(&running).0
        });
        let deadline = Instant::now() + Duration::from_secs(30);
        while !started.exists() {
            assert!(Instant::now() < deadline, "the writer never started");
            std::thread::sleep(Duration::from_millis(20));
        }
        let writer = Some(writer);
        Waiting {
            run,
            gate,
            writer,
            _dir: dir,
        }
    }

    /// Its driver let go: the event its attempt ended with.
    fn ended(mut self) -> EventType {
        std::fs::write(&self.gate, "").unwrap();
        self.writer.take().unwrap().join().unwrap()
    }
}

impl Drop for Waiting {
    /// On unwind too, its driver is let go.
    fn drop(&mut self) {
        let _ = std::fs::write(&self.gate, "");
    }
}

/// A zero-grant attacker outside any box beside a retaining member never
/// starts, and the retained bytes stay as published.
#[test]
fn a_zero_grant_attacker_beside_a_retaining_member_never_starts() {
    let mut claude = Claude::retaining();
    let artifact = evidence(&claude.fixture.engine);
    let forge = format!("printf forged > '{}'", artifact.display());
    armed(&mut claude.fixture.engine, &forge, None);
    claude.attacked();
    let run = claude.fixture.engine.run_id.clone();
    let site = "work:attacker".into();
    claude.refused(Conflict::Unboxed { site, run }, (&artifact, b"retained"));
}

/// A boxed exec attacker whose box can write the coordination root never
/// starts: its reach would let it strike every reservation and the lock.
#[test]
fn a_boxed_exec_attacker_that_can_reach_the_coordination_root_never_starts() {
    let mut claude = Claude::retaining();
    let home = claude.fixture.root.join("home");
    let coordination = home.join(".local/state/brokkr/capabilities");
    let artifact = evidence(&claude.fixture.engine);
    armed(&mut claude.fixture.engine, ":", Some(hands(&[&home])));
    let exec = ["{brokkr}", "driver", "exec", "--", "sh", "-c"].map(String::from);
    let forge = format!("printf forged > '{}'", artifact.display());
    let command = exec.into_iter().chain([forge]).collect();
    member(&mut claude.fixture.engine, "attacker").command = command;
    claude.attacked();
    let conflict = reaching(&claude, &coordination);
    claude.refused(conflict, (&artifact, b"retained"));
    assert!(
        coordination.join("admission.lock").exists(),
        "the lock stands"
    );
}

/// An already-running writer outside any box, a real engine's dispatch,
/// holds off a retaining member: while it runs, the retaining attempt is
/// refused before anything of it starts; once its attempt settled, the
/// retaining panel runs beside a member writing its own worktree, and the
/// retained bytes stay as published.
#[test]
fn an_already_running_writer_holds_off_a_retaining_member() {
    let mut claude = Claude::retaining();
    let own = claude.fixture.root.join("work/own");
    let mine = format!("printf mine > '{}'", own.display());
    armed(&mut claude.fixture.engine, &mine, Some(hands(&[])));
    let artifact = evidence(&claude.fixture.engine);
    let waiting = Waiting::on(&claude);
    claude.attacked();
    let held_off = Conflict::Unboxed {
        site: "work".into(),
        run: waiting.run.clone(),
    };
    claude.refused(held_off, (&artifact, b"retained"));
    assert_eq!(waiting.ended(), EventType::EffectSucceeded);
    claude.attacked();
    let (_, payload) = last(&claude.fixture.engine);
    // Off Linux no mount table proves the evidence shown once to a boxed
    // writer (decision 0063): the panel is refused at the coordination
    // root there, and runs on Linux.
    #[cfg(target_os = "linux")]
    {
        let keeper = claude.fixture.root.join("driver.argv").exists();
        assert!(keeper, "the keeper ran: {payload}");
        let written = std::fs::read(&own).ok();
        assert_eq!(written.as_deref(), Some(&b"mine"[..]), "{payload}");
    }
    #[cfg(not(target_os = "linux"))]
    {
        let root = claude
            .fixture
            .root
            .join("home/.local/state/brokkr/capabilities");
        let shown = Unadmitted::Evidence(Conflict::Shown(root)).to_string();
        assert_eq!(payload["error"], shown);
    }
    assert_eq!(std::fs::read(&artifact).unwrap(), b"retained");
}

/// Set in the process a dispatched attacker's driver starts, naming the
/// request [`boxed_attacker`] runs.
#[cfg(target_os = "linux")]
const BOXED: &str = "BROKKR_U8A2A_BOXED";

/// [`boxed_attacker`], as the attacker's driver runs it.
#[cfg(target_os = "linux")]
const BOXER: &str =
    "engine::capability_tests::provision_tests::brokered_tests::writers_tests::boxed_attacker";

/// Whether this host builds a real box, and so runs the proofs that need
/// one; the test `test` skipped with its reason where it does not, as in a
/// seat's own box, which the controller's host run covers.
#[cfg(target_os = "linux")]
fn boxes_here(test: &str) -> bool {
    use brokkr_protocol::hands::{require_bwrap, HANDS_BOX_ENV};
    let probe = require_bwrap().ok().filter(|bwrap| {
        let probed = std::process::Command::new(bwrap)
            .args(["--ro-bind", "/", "/", "--", "true"])
            .stderr(std::process::Stdio::null())
            .status();
        probed.is_ok_and(|status| status.success())
    });
    let boxes = probe.is_some() & std::env::var_os(HANDS_BOX_ENV).is_none();
    if !boxes {
        let required = brokkr_protocol::hands::boundary_evidence_required();
        let reason = format!("{test}: no bubblewrap namespace here");
        brokkr_protocol::hands::skip_boundary_proof(required, &reason);
    }
    boxes
}

/// The attacker of `claude`'s run armed, boxed by `spec`, to run `attack`
/// in its own box once its attempt is dispatched: its driver runs this
/// test binary on [`boxed_attacker`] with the request `case` names, which
/// holds the argv the engine composes for an exec seat of that spec
/// ([`crate::engine::boxed::hands_command`]) and where its reservation
/// stands. The request's path; its box's mark is that path with `ran` for
/// its extension.
#[cfg(target_os = "linux")]
fn dispatched(claude: &mut Claude, case: &str, (spec, attack): (&HandsSpec, &str)) -> PathBuf {
    let engine = &claude.fixture.engine;
    let work = std::fs::canonicalize(engine.workdir()).unwrap();
    let artifacts = crate::engine::boxed::artifact_root(&engine.run_local());
    let exec = ["brokkr", "driver", "exec", "--", "/bin/sh", "-c", attack].map(String::from);
    // Its reach sealed as its admission seals it, under the fixture's HOME.
    let git = brokkr_protocol::hands::git_facts(&work);
    let reach = brokkr_protocol::hands::seal(spec, &claude.fixture.root.join("home"), &work, &git);
    let boxed = (artifacts.as_path(), Some(&reach));
    let argv = crate::engine::boxed::hands_command(exec.to_vec(), Some(spec), &work, boxed, &[]);
    let repo = canonical::sha256_bytes(work.as_os_str().as_encoded_bytes());
    let home = claude
        .fixture
        .root
        .join("home/.local/state/brokkr/capabilities");
    let reservation = home
        .join(repo)
        .join(&engine.run_id)
        .join("reservation.json");
    let request = claude.fixture.root.join(format!("{case}.json"));
    let written = json!({"argv": argv.unwrap(), "reservation": reservation});
    std::fs::write(&request, written.to_string()).unwrap();
    let exe = std::env::current_exe().unwrap();
    let script = format!(
        "{BOXED}='{0}' '{1}' --exact {BOXER} --nocapture --test-threads=1 > '{0}.out' 2>&1",
        request.display(),
        exe.display()
    );
    armed(&mut claude.fixture.engine, &script, Some(spec.clone()));
    request
}

/// A dispatched attacker's box (M7), inert unless [`BOXED`] names its
/// request: `hands::run_boxed` over the spec, workdir and artifact root its
/// argv carries, as `brokkr hands exec` runs it, running the command that
/// argv's `driver exec` would start. Its mark: whether the attempt's
/// reservation held an attempt standing as the box began, and whether the
/// box was built and ran.
#[cfg(target_os = "linux")]
#[test]
fn boxed_attacker() {
    let Some(request) = std::env::var_os(BOXED).map(PathBuf::from) else {
        return;
    };
    let read: Value = serde_json::from_slice(&std::fs::read(&request).unwrap()).unwrap();
    let argv: Vec<String> = serde_json::from_value(read["argv"].clone()).unwrap();
    let flag = |name: &str| {
        let at = argv.iter().position(|part| part == name).unwrap();
        argv[at + 1].clone()
    };
    let spec = HandsSpec::parse(&serde_json::from_str(&flag("--spec")).unwrap()).unwrap();
    let (work, artifacts) = (
        PathBuf::from(flag("--workdir")),
        PathBuf::from(flag("--artifacts")),
    );
    let inner = argv.iter().rposition(|part| part == "--").unwrap() + 1;
    let reservation = std::fs::read(read["reservation"].as_str().unwrap()).unwrap();
    let reservation: Value = serde_json::from_slice(&reservation).unwrap();
    let standing = reservation["attempts"]
        .as_object()
        .is_some_and(|map| !map.is_empty());
    let reach = serde_json::from_str(&flag("--reach")).unwrap();
    let ran =
        brokkr_protocol::hands::run_boxed(&spec, &work, &artifacts, &reach, None, &argv[inner..]);
    let mark = json!({"standing": standing, "ran": ran.is_ok()});
    std::fs::write(request.with_extension("ran"), mark.to_string()).unwrap();
}

/// The attack matrix (D6's last paragraph, U8a2a's Proof line; M7), beside
/// a retaining panel member with the fence lifted, every attacker
/// dispatched through the door and, where admitted, run in its own real
/// box inside its attempt ([`boxed_attacker`]): an already-running writer
/// that really dispatched; another worktree's settled, historical
/// artifacts, reached by a bind of their run-local directory, by a bind of
/// their worktree that could replace an ancestor, by a symlink alias of the
/// run-local directory, and through a hard link in the attacker's own
/// worktree; a source another writer could swap; and, last, a zero-grant
/// attacker that admission admits, whose box keeps both its own run's
/// retained bytes and the historical ones while its legitimate worktree
/// write lands. The protected bytes are compared after each, so an
/// admission that failed open shows in them. The bind alias, which needs a
/// mount namespace, is [`a_bind_alias_attacker_is_refused_on_the_dispatch_path`].
/// Linux only: off Linux no box is built (decision 0063).
#[cfg(target_os = "linux")]
#[test]
fn the_attack_matrix_runs_every_admitted_attacker_in_its_own_box() {
    let test = "the_attack_matrix_runs_every_admitted_attacker_in_its_own_box";
    if !boxes_here(test) {
        return;
    }
    let mut claude = Claude::retaining();
    let retained = evidence(&claude.fixture.engine);
    let forge = |at: &Path| format!("printf forged > '{}'", at.display());
    let kept = [(retained.as_path(), &b"retained"[..])];
    let waiting = Waiting::on(&claude);
    let held_off = Conflict::Unboxed {
        site: "work".into(),
        run: waiting.run.clone(),
    };
    let armed = (hands(&[]), forge(&retained));
    struck(&mut claude, "running", armed, Some(held_off), &kept);
    assert_eq!(waiting.ended(), EventType::EffectSucceeded);
    let (artifact, _other) = settled(&claude, b"historical");
    let root = artifact.parent().unwrap().parent().unwrap().to_path_buf();
    let state = root.parent().unwrap().to_path_buf();
    let worktree = state.parent().unwrap().to_path_buf();
    let alias = claude.fixture.root.join("alias");
    std::os::unix::fs::symlink(&state, &alias).unwrap();
    let linked = claude.fixture.root.join("work/linked");
    let replace = format!(
        "mv '{0}' '{0}.old' && mkdir -p '{1}' && printf forged > '{2}'",
        state.display(),
        artifact.parent().unwrap().display(),
        artifact.display()
    );
    let kept = [kept[0], (artifact.as_path(), &b"historical"[..])];
    let cases = [
        (
            "bind",
            hands(&[&state]),
            forge(&artifact),
            reaching(&claude, &root),
        ),
        (
            "ancestor",
            hands(&[&worktree]),
            replace,
            reaching(&claude, &root),
        ),
        (
            "symlink",
            hands(&[&alias]),
            forge(&artifact),
            reaching(&claude, &root),
        ),
        (
            "hardlink",
            hands(&[]),
            forge(&linked),
            Conflict::Corrupt(artifact.clone()),
        ),
    ];
    for (case, spec, attack, refused) in cases {
        if case == "hardlink" {
            std::fs::hard_link(&artifact, &linked).unwrap();
        }
        struck(&mut claude, case, (spec, attack), Some(refused), &kept);
        std::fs::remove_file(&linked).ok();
    }
    let lock = swapped_source(&mut claude, &kept);
    let with_lock = [(lock.as_path(), &b""[..]), kept[0], kept[1]];
    // A disjoint source outside every writable root is admitted, and its
    // box runs inside its attempt.
    let apart = claude.fixture.root.join("apart");
    std::fs::create_dir_all(&apart).unwrap();
    let zero_grant = format!(
        "{}; {}; mv .forge gone; echo w > made",
        forge(&retained),
        forge(&artifact)
    );
    let armed = (hands(&[&apart]), zero_grant);
    let mark = struck(&mut claude, "zero-grant", armed, None, &with_lock);
    assert_eq!(mark, Some(json!({"standing": true, "ran": true})));
    let made = std::fs::read(claude.fixture.root.join("work/made")).ok();
    assert_eq!(
        made.as_deref(),
        Some(&b"w\n"[..]),
        "the worktree write landed"
    );
    let digest = |at: &Path| canonical::sha256_bytes(&std::fs::read(at).unwrap());
    let named = |at: &Path| at.file_name().unwrap().to_str().unwrap().to_string();
    assert_eq!(
        (digest(&artifact), digest(&retained)),
        (named(&artifact), named(&retained))
    );
}

/// The coordination root below a HOME.
#[cfg(target_os = "linux")]
const COORDINATION: &str = ".local/state/brokkr/capabilities";

/// The matrix's R1 case (U8a2a; R1): a source beneath `claude`'s
/// worktree, which another writer of it could swap for a symlink to the
/// coordination root before the box is rebuilt from the authored source,
/// is refused before anything spawns, the admission lock's bytes kept
/// beside every file of `kept`. The lock's path.
#[cfg(target_os = "linux")]
fn swapped_source(claude: &mut Claude, kept: &[(&Path, &[u8])]) -> PathBuf {
    let source = claude.fixture.root.join("work/src");
    let coordination = claude.fixture.root.join("home").join(COORDINATION);
    std::fs::create_dir_all(&source).unwrap();
    let lock = coordination.join("admission.lock");
    let redirect = format!(
        "printf forged > '{}'",
        source.join("admission.lock").display()
    );
    let replaceable = Conflict::Replaceable {
        site: "work:attacker".into(),
        run: claude.fixture.engine.run_id.clone(),
        bind: source.clone(),
    };
    let held = std::iter::once((lock.as_path(), &b""[..]));
    let held: Vec<(&Path, &[u8])> = held.chain(kept.iter().copied()).collect();
    let armed = (hands(&[&source]), redirect);
    struck(claude, "swapped", armed, Some(replaceable), &held);
    lock
}

/// One case of the attack matrix, named `case`: `claude`'s attacker boxed
/// by the hands and running the attack `armed` names, dispatched beside
/// the retaining keeper with the fence lifted ([`dispatched`]); then every
/// protected file of `kept` holding its bytes, and the attempt refused by
/// `refused` before anything of it started, its box never begun, or
/// admitted. The mark the admitted attacker's box left, if any.
#[cfg(target_os = "linux")]
fn struck(
    claude: &mut Claude,
    case: &str,
    (spec, attack): (HandsSpec, String),
    refused: Option<Conflict>,
    kept: &[(&Path, &[u8])],
) -> Option<Value> {
    let request = dispatched(claude, case, (&spec, &attack));
    claude.attacked();
    let mark = std::fs::read(request.with_extension("ran")).ok();
    let mark = mark.map(|mark| serde_json::from_slice(&mark).unwrap());
    for (at, bytes) in kept {
        let held = std::fs::read(at).unwrap();
        let shown = at.display();
        assert_eq!(held, *bytes, "{case}: the protected bytes of {shown}");
    }
    match refused {
        Some(conflict) => {
            claude.refused(conflict, kept[0]);
            assert_eq!(mark, None, "{case}: no box began");
        }
        None => assert!(mark.is_some(), "{case}: the admitted box ran"),
    }
    mark
}

/// Names the fixture of [`a_bind_alias_attacker_is_refused_on_the_dispatch_path`]
/// in the mount namespace it runs in.
#[cfg(target_os = "linux")]
const ALIASED: &str = "BROKKR_U8A2A_DISPATCH_ALIASED";

/// A bind alias of protected evidence on the dispatch path (M7): beside the
/// retaining keeper with the fence lifted, an attacker whose writable bind
/// is a second mount of another run's evidence directory is refused by the
/// identity it shares, before anything of it starts, the evidence's bytes
/// unchanged. The alias is a bind bubblewrap makes for the proof, in a
/// mount namespace of its own, where this test runs again.
#[cfg(target_os = "linux")]
#[test]
fn a_bind_alias_attacker_is_refused_on_the_dispatch_path() {
    let test = "a_bind_alias_attacker_is_refused_on_the_dispatch_path";
    if let Some(dir) = std::env::var_os(ALIASED) {
        return aliased(Path::new(&dir));
    }
    if !boxes_here(test) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(dir.path()).unwrap();
    let forge = root.join("kept/.forge");
    let digests = forge.join("artifacts/sha256");
    std::fs::create_dir_all(&digests).unwrap();
    std::fs::write(digests.join(canonical::sha256_bytes(b"kept")), "kept").unwrap();
    std::fs::create_dir_all(root.join("alias")).unwrap();
    std::fs::create_dir_all(root.join("fixture")).unwrap();
    let bwrap = brokkr_protocol::hands::require_bwrap().unwrap();
    let name =
        format!("engine::capability_tests::provision_tests::brokered_tests::writers_tests::{test}");
    let status = std::process::Command::new(bwrap)
        .args(["--dev-bind", "/", "/", "--bind"])
        .arg(&forge)
        .arg(root.join("alias"))
        .arg("--")
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", &name, "--nocapture", "--test-threads=1"])
        .env(ALIASED, &root)
        .status();
    assert!(
        root.join("entered").exists(),
        "the proof ran in its namespace"
    );
    assert!(
        status.unwrap().success(),
        "the proof in its namespace failed"
    );
}

/// [`a_bind_alias_attacker_is_refused_on_the_dispatch_path`] in its
/// namespace, on the fixture at `root`: the kept worktree's evidence held
/// by a standing reservation of its own, and the retaining panel's attacker
/// bound to the alias.
#[cfg(target_os = "linux")]
fn aliased(root: &Path) {
    std::fs::write(root.join("entered"), "").unwrap();
    let kept = root.join("kept");
    let evidence = kept.join(".forge/artifacts");
    let artifact = evidence
        .join("sha256")
        .join(canonical::sha256_bytes(b"kept"));
    let mut claude = Claude::retaining_on(Fixture::inline_in(&root.join("fixture")));
    let home = claude.fixture.root.join("home");
    let _standing = crate::engine::broker::admission::tests::protecting(&home, &kept);
    let alias = root.join("alias");
    armed(
        &mut claude.fixture.engine,
        &format!(
            "printf forged > '{}'",
            alias
                .join("artifacts/sha256")
                .join(canonical::sha256_bytes(b"kept"))
                .display()
        ),
        Some(hands(&[&alias])),
    );
    claude.attacked();
    claude.refused(reaching(&claude, &evidence), (&artifact, b"kept"));
}
