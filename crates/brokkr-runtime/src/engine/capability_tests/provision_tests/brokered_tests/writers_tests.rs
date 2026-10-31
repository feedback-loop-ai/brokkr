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
#[cfg(target_os = "linux")]
use crate::engine::broker::admission::tests::{leased, staked, standing_writer, unsynced_next};
use crate::engine::broker::admission::{Conflict, Unadmitted, Unknown};
#[cfg(target_os = "linux")]
use crate::engine::broker::settlement::Unreleased;
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
    fn refused(&self, conflict: Conflict, kept: (&Path, &[u8])) {
        self.refused_by(Unadmitted::Evidence(conflict), kept);
    }

    /// [`Claude::refused`], by the refusal `refusal`.
    fn refused_by(&self, refusal: Unadmitted, (artifact, bytes): (&Path, &[u8])) {
        let kept = std::fs::read(artifact).unwrap();
        assert_eq!(kept, bytes, "the protected bytes");
        let (event, payload) = last(&self.fixture.engine);
        assert_eq!(event, EventType::EffectFailed, "{payload}");
        assert_eq!(payload["error"], refusal.to_string());
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
    let refusal = Unadmitted::Identity(Unknown::Serving {
        site: "work:attacker".into(),
        run: claude.fixture.engine.run_id.clone(),
        served: coordination.clone(),
    });
    claude.refused_by(refusal, (&artifact, b"retained"));
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
    let home = claude.fixture.root.join("home");
    let reach = brokkr_protocol::hands::seal(spec, &home, &work, &git, None);
    let boxed = (
        &*crate::argv::tests::ENGINE,
        artifacts.as_path(),
        Some(&reach),
    );
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

/// The package the fixture's server is installed in: the program tree its
/// box is built from (U8a2b).
#[cfg(target_os = "linux")]
fn package(claude: &Claude) -> PathBuf {
    claude.fixture.root.join("opt/docs")
}

impl Claude {
    /// The attempt failed by MB3's identity cause, `site` of `run` able to
    /// write the server's package, before anything of it started, naming
    /// the launch that lies in it, reserved before any survey
    /// ([`Claude::exposing`]).
    #[cfg(target_os = "linux")]
    fn exposed(&self, site: &str, run: &str) {
        self.exposing(site, run, Path::new(&self.fixture.server()));
    }

    /// The attempt failed by MB3's identity cause, `site` of `run` able to
    /// write the server's source `served`, before anything of it started: no
    /// member's driver spawned, no plan written for a broker to bind, so no
    /// secret looked up and no server started, and the program unchanged.
    #[cfg(target_os = "linux")]
    fn exposing(&self, site: &str, run: &str, served: &Path) {
        let (event, payload) = last(&self.fixture.engine);
        assert_eq!(event, EventType::EffectFailed, "{payload}");
        let served = served.to_path_buf();
        let (site, run) = (site.to_string(), run.to_string());
        let refused = Unadmitted::Identity(Unknown::Serving { site, run, served });
        assert_eq!(payload["error"], refused.to_string());
        let spawned = self.fixture.root.join("driver.argv").exists();
        assert!(!spawned, "the keeper never started");
        let names = std::fs::read_dir(self.fixture.run_dir()).unwrap();
        let names: Vec<_> = names.map(|entry| entry.unwrap().file_name()).collect();
        assert_eq!(names, ["reservation.json"], "no plan was written");
        let program = std::fs::read(self.fixture.server()).unwrap();
        assert_eq!(program, b"#!/bin/sh\n", "the server's program");
    }
}

/// A zero-grant panel sibling whose box could write the package the
/// retaining keeper's server is built from, through a link toward it, is
/// refused by MB3's identity cause as the keeper's lease is extended to its
/// server's sources, before any member starts; a sibling writing only its
/// own worktree runs beside the keeper. Linux only, as it admits a boxed
/// writer beside the mount table's judgement (decision 0063).
#[cfg(target_os = "linux")]
#[test]
fn a_zero_grant_sibling_that_can_write_the_servers_package_never_starts() {
    let mut claude = Claude::retaining();
    let link = claude.fixture.root.join("docs");
    std::os::unix::fs::symlink(package(&claude), &link).unwrap();
    let forge = format!("printf forged > '{}'", claude.fixture.server());
    armed(&mut claude.fixture.engine, &forge, Some(hands(&[&link])));
    claude.attacked();
    let run = claude.fixture.engine.run_id.clone();
    claude.exposed("work:attacker", &run);
    armed(&mut claude.fixture.engine, ":", Some(hands(&[])));
    claude.attacked();
    let (_, payload) = last(&claude.fixture.engine);
    let keeper = claude.fixture.root.join("driver.argv").exists();
    assert!(keeper, "the keeper ran: {payload}");
}

/// A system server's box is built from the system directories every
/// server box binds, which lie in the operating system's own roots: with
/// the keeper's server the system `true`, a zero-grant sibling whose box
/// could write, through a link, the directory holding that program, or
/// `/usr/share`, is refused by MB3's identity cause naming `/usr`, the root
/// it reaches (D6), before any member starts. Linux only, as above.
#[cfg(target_os = "linux")]
#[test]
fn a_zero_grant_sibling_that_can_write_a_system_servers_source_never_starts() {
    let home = PathBuf::from("/");
    let program = ServerProgram::resolve("true", &home).unwrap();
    let program = program.executable().to_path_buf();
    let bin = std::fs::canonicalize(program.parent().unwrap()).unwrap();
    let share = std::fs::canonicalize("/usr/share").unwrap();
    let usr = PathBuf::from("/usr");
    for (target, served) in [(&bin, &usr), (&share, &usr)] {
        let mut claude = Claude::retaining();
        claude.launched(&["true"]);
        let link = claude.fixture.root.join("system");
        std::os::unix::fs::symlink(target, &link).unwrap();
        armed(&mut claude.fixture.engine, ":", Some(hands(&[&link])));
        claude.attacked();
        let run = claude.fixture.engine.run_id.clone();
        claude.exposing("work:attacker", &run, served);
    }
}

/// Writer first (H2): another worktree's zero-grant run, already standing,
/// whose box could write, through a link, the bootstrap's directory or
/// `/usr/bin`, refuses the retaining keeper's attempt by MB3's identity
/// cause naming it, or `/usr`, the root it lies in (D6), before any box is
/// surveyed; so does one that could
/// write the package, naming the launch that lies in it. Either way
/// nothing of the attempt starts and no plan is written for a broker to
/// bind, so none looks a secret up or starts its server. Linux only, as
/// above.
#[cfg(target_os = "linux")]
#[test]
fn a_standing_writer_of_a_servers_input_refuses_the_broker_before_lookup() {
    let bin = std::fs::canonicalize("/usr/bin").unwrap();
    let boot = |claude: &Claude| claude.fixture.root.join("boot/brokkr");
    // What a fixture's writer links toward, and the input that names.
    type Named<'n> = &'n dyn Fn(&Claude) -> PathBuf;
    let cases: [(Named<'_>, Named<'_>, usize); 3] = [
        (&|claude| boot(claude).parent().unwrap().into(), &boot, 0),
        (&|_| bin.clone(), &|_| PathBuf::from("/usr"), 0),
        (&package, &|claude| claude.fixture.server().into(), 0),
    ];
    for (target, served, surveys) in cases {
        let mut claude = Claude::retaining();
        let boot = boot(&claude);
        std::fs::create_dir_all(boot.parent().unwrap()).unwrap();
        std::fs::write(&boot, "brokkr").unwrap();
        let link = claude.fixture.root.join("aliased");
        std::os::unix::fs::symlink(target(&claude), &link).unwrap();
        let other = claude.fixture.root.join("other");
        std::fs::create_dir(&other).unwrap();
        let home = claude.fixture.root.join("home");
        let _standing = standing_writer(&home, &other, &link);
        armed(&mut claude.fixture.engine, ":", Some(hands(&[])));
        SURVEYED.set(0);
        let host = Host {
            bootstrap: boot,
            ..claude.fixture.host()
        };
        let _lift = McpFence::lift();
        claude.executed_on(host);
        claude.exposing("work", "run-b", &served(&claude));
        assert_eq!(SURVEYED.get(), surveys, "{:?}", served(&claude));
    }
}

/// Writer first, by a link on a route (U8a2b, run three): another
/// worktree's standing zero-grant run confined to `srv/links`, which holds
/// only a link on the way of a server's input — the bootstrap's or the
/// keeper's launch, each refused before any box is surveyed (run four) —
/// refuses the retaining keeper's attempt by MB3's identity
/// cause naming that link, though neither the input as spelled nor where it
/// resolves lies in its reach: nothing starts and no plan is written for a
/// broker to bind, so none looks a secret up or starts its server. One
/// confined to a directory beside it is kept apart, and the keeper runs.
/// Linux only, as above.
#[cfg(target_os = "linux")]
#[test]
fn a_standing_writer_of_a_link_on_a_servers_route_refuses_the_broker_before_lookup() {
    let symlink = |target: &Path, link: &Path| std::os::unix::fs::symlink(target, link).unwrap();
    for (case, surveys) in [("bootstrap", 0), ("launch", 0), ("apart", 1)] {
        let mut claude = Claude::retaining();
        let root = claude.fixture.root.clone();
        let [links, apart, other] = ["srv/links", "srv/apart", "other"].map(|dir| root.join(dir));
        for made in [&links, &apart, &other] {
            std::fs::create_dir_all(made).unwrap();
        }
        let boot = root.join("opt/brokkr");
        std::fs::write(&boot, "brokkr").unwrap();
        symlink(&boot, &links.join("brokkr"));
        symlink(&links.join("brokkr"), &root.join("boot"));
        symlink(Path::new(&claude.fixture.server()), &links.join("server"));
        symlink(&links.join("server"), &root.join("launch"));
        claude.launched(&[&root.join("launch").display().to_string(), "--stdio"]);
        let reach = if case == "apart" { &apart } else { &links };
        let _standing = standing_writer(&root.join("home"), &other, reach);
        armed(&mut claude.fixture.engine, ":", Some(hands(&[])));
        SURVEYED.set(0);
        let bootstrap = if case == "bootstrap" {
            root.join("boot")
        } else {
            boot
        };
        let host = Host {
            bootstrap,
            ..claude.fixture.host()
        };
        let _lift = McpFence::lift();
        claude.executed_on(host);
        match case {
            "bootstrap" => claude.exposing("work", "run-b", &links.join("brokkr")),
            "launch" => claude.exposing("work", "run-b", &links.join("server")),
            _ => {
                let (_, payload) = last(&claude.fixture.engine);
                let keeper = root.join("driver.argv").exists();
                assert!(keeper, "a writer apart: the keeper ran: {payload}");
            }
        }
        assert_eq!(SURVEYED.get(), surveys, "{case}");
    }
}

/// A launch swapped while its survey ran (U8a2b, run four): with another
/// worktree's zero-grant run standing, confined to `srv/links`, the launch
/// is a link straight toward the server's program, so the extension before
/// the survey commits; the survey's stand-in then points it through
/// `srv/links`. The extension after the survey locates the launch again
/// and refuses the attempt by MB3's identity cause naming the link the
/// writer could replace, and no plan is written. Left as it was, the
/// attempt provisions. Linux only: the extension reads the mount table.
#[cfg(target_os = "linux")]
#[test]
fn a_launch_swapped_during_its_survey_refuses_the_extension_after_it() {
    for swapped in [true, false] {
        let mut claude = Claude::of(Fixture::inline(), single(), &["work"]);
        let root = claude.fixture.root.clone();
        let [links, other] = ["srv/links", "other"].map(|dir| root.join(dir));
        for made in [&links, &other] {
            std::fs::create_dir_all(made).unwrap();
        }
        let (server, launch) = (PathBuf::from(claude.fixture.server()), root.join("launch"));
        std::os::unix::fs::symlink(&server, links.join("server")).unwrap();
        std::os::unix::fs::symlink(&server, &launch).unwrap();
        let held = admitted("library-docs", &[&launch.display().to_string()], true);
        claude.fixture.hold("library-docs", held);
        let home = root.join("home");
        let _standing = standing_writer(&home, &other, &links);
        leased(&mut claude.fixture.engine, &home, "attempt-1");
        let survey = |bootstrap: &Path, request: &Survey, deadline: Instant| {
            if swapped {
                std::fs::remove_file(&launch).unwrap();
                std::os::unix::fs::symlink(links.join("server"), &launch).unwrap();
            }
            observing(bootstrap, request, deadline)
        };
        let (ids, mut bound) = (["effect", "attempt-1", "work"], Duration::from_secs(60));
        let (mut selection, host) = (selected(), Ok(claude.fixture.host()));
        let engine = &mut claude.fixture.engine;
        let timed = (Instant::now(), &mut bound);
        let launched = engine.provision_with(ids, &mut selection, timed, host, &survey);
        assert_eq!(launched.unwrap(), !swapped, "swapped: {swapped}");
        if swapped {
            let (_, payload) = last(engine);
            let (site, run, served) = ("work".into(), "run-b".into(), links.join("server"));
            let refused = Unadmitted::Identity(Unknown::Serving { site, run, served });
            assert_eq!(payload["error"], refused.to_string());
            assert!(
                !claude.fixture.run_dir().join("attempt-1").exists(),
                "no plan"
            );
        }
    }
}

impl Claude {
    /// The work seat's effect executed as dispatch executes it, on `host`
    /// stood in.
    #[cfg(target_os = "linux")]
    fn executed_on(&mut self, host: Host) {
        let _host = StoodIn::new(host);
        let effect = requested(&self.fixture.engine, "effect");
        execute_work(&mut self.fixture.engine, &[effect]).unwrap();
    }

    /// The retaining keeper's `library-docs` server launched by `argv`.
    #[cfg(target_os = "linux")]
    fn launched(&mut self, argv: &[&str]) {
        let keeper = self.fixture.engine.bundle.sites.get_mut("work:keeper");
        let outcome = &mut keeper.unwrap().capabilities.as_mut().unwrap().outcomes[0];
        let held = admitted("library-docs", argv, true);
        outcome.held.insert("library-docs".into(), held);
    }
}

/// L1: a seal of the lease that committed unsynced is kept on the lease
/// whatever provisioning refuses after it — a survey that does not answer,
/// after the extension to the fixed inputs, or an attempt directory that
/// already stands, after both extensions — so, though the attempt is then
/// proven settled and released, it fails its run as it settles under the
/// fence rule, evidence at stake. Linux only: the extension reads the mount table (decision
/// 0063).
#[cfg(target_os = "linux")]
#[test]
fn a_seal_committed_unsynced_is_kept_whatever_provisioning_refuses_after_it() {
    let silent = |_: &Path, _: &Survey, _: Instant| Err(Unprovisioned::Expired);
    let cases: [(Surveyor<'_>, Unprovisioned); 2] = [
        (&silent, Unprovisioned::Expired),
        (&observing, Unprovisioned::Provisioned),
    ];
    for (survey, refused) in cases {
        let mut claude = Claude::of(Fixture::inline(), single(), &["work"]);
        let home = claude.fixture.root.join("home");
        leased(&mut claude.fixture.engine, &home, "attempt-1");
        let made = claude.fixture.run_dir().join("attempt-1");
        if refused == Unprovisioned::Provisioned {
            std::fs::create_dir(&made).unwrap();
        }
        unsynced_next();
        let (ids, mut bound) = (["effect", "attempt-1", "work"], Duration::from_secs(60));
        let (mut selection, host) = (selected(), Ok(claude.fixture.host()));
        let timed = (Instant::now(), &mut bound);
        let engine = &mut claude.fixture.engine;
        let launched = engine.provision_with(ids, &mut selection, timed, host, survey);
        assert!(!launched.unwrap(), "{refused}");
        let (_, payload) = last(engine);
        assert_eq!(payload["error"], refused.to_string());
        staked(engine);
        let settled = engine.settle("attempt-1", Ok(())).unwrap_err();
        let EngineError::Unreleased(unreleased) = &settled else {
            panic!("{refused}: {settled}");
        };
        let unsynced = Unreleased::released(Unadmitted::Identity(Unknown::Unsynced));
        assert_eq!(*unreleased, unsynced, "{refused}: kept on the lease");
    }
}

/// With the fence lifted every attempt that brokers is admitted before it
/// is provisioned, so provisioning one that holds no lease is refused by
/// MB3's identity cause, naming the reservation it lacks, before any box
/// is surveyed and before anything is made (`sources::held_extended`).
/// While the fence stands a test's attempt it never admitted is
/// provisioned from HOME, as the provisioning suite's are.
#[test]
fn a_brokered_attempt_holding_no_lease_is_refused_once_the_fence_lifts() {
    let claude = Claude::of(Fixture::inline(), single(), &["work"]);
    let survey = |_: &Path, _: &Survey, _: Instant| panic!("a box was surveyed");
    let _lift = McpFence::lift();
    let refused = claude.fixture.provision("attempt-1", &survey);
    let lost = Unprovisioned::Unadmitted(Unknown::Lost.into());
    assert_eq!(refused, Err(lost));
    assert!(!claude.fixture.run_dir().exists(), "nothing was made");
}

/// Names the directory [`serving_dispatch`] makes its fixture in.
#[cfg(target_os = "linux")]
const SERVED: &str = "BROKKR_U8A2B_SERVED";

/// [`serving_dispatch`], as its test runs it.
#[cfg(target_os = "linux")]
const SERVER: &str =
    "engine::capability_tests::provision_tests::brokered_tests::writers_tests::serving_dispatch";

/// Names the directory [`serving_broker`] finds its fixture in.
#[cfg(target_os = "linux")]
const BROKER: &str = "BROKKR_U8A2B_BROKER";

/// [`serving_broker`], as the seat's driver runs it.
#[cfg(target_os = "linux")]
const BROKER_TEST: &str =
    "engine::capability_tests::provision_tests::brokered_tests::writers_tests::serving_broker";

/// What outlives the engine [`killed_while`] kills: the seat's driver
/// alone, or a real server box the seat's broker launched beneath it, its
/// harness confined by the `setpriv` found ([`setpriv`]).
#[cfg(target_os = "linux")]
#[derive(Clone, PartialEq, Eq)]
enum Payload {
    Seat,
    Server(PathBuf),
}

/// The `setpriv` on this process's `PATH`, which confines a harness under
/// `no_new_privs`; none where this host has none.
#[cfg(target_os = "linux")]
fn setpriv() -> Option<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let found = std::env::split_paths(&path).map(|dir| dir.join("setpriv"));
    found.into_iter().find(|found| found.is_file())
}

/// The engine [`killed_while`] kills, in a process of its own: the single
/// brokered seat's attempt dispatched with the fence lifted on the fixture
/// it makes in the directory [`SERVED`] names — admitted, its lease
/// extended to its server's inputs, its server's plan written — and its
/// driver, the seat whose broker serves that server, writing its pid and
/// waiting to be killed: as a shell, or, where the directory holds
/// `payload`, naming the `setpriv` its parent found, as [`serving_broker`],
/// its server the package's program, which waits. Inert without
/// [`SERVED`]; production reads no such variable.
#[cfg(target_os = "linux")]
#[test]
fn serving_dispatch() {
    let Some(dir) = std::env::var_os(SERVED) else {
        return;
    };
    let dir = PathBuf::from(dir);
    let mut claude = Claude::of(Fixture::inline_in(&dir), single(), &["work"]);
    let (source, pid) = (dir.join("seat.source"), dir.join("seat.pid"));
    let shown = pid.display();
    let waits = format!("echo $$ > '{shown}.new'\nmv '{shown}.new' '{shown}'\nexec sleep 600");
    // The broker's observer holds its write-exclusion proof against itself,
    // its parent and theirs (MB3): each confined under `no_new_privs`, as
    // `setpriv` sets it, here a harness shell, the shell it runs and the
    // broker that one runs, each a child, not in its parent's place.
    // Named by its parent, which skips where this host has none ([`setpriv`]).
    // The seat's own shell stays, holding the stdout its engine reads: a
    // driver whose stdout closes unaccepted is one the engine ends after its
    // grace, and a slow host's box can take longer than that to stand.
    let setpriv = std::fs::read_to_string(dir.join("payload")).ok();
    let harness = "'\"$0\" \"$@\"; exit $?'";
    let serves = |setpriv: &str| {
        format!(
            "{BROKER}='{0}' RUST_BACKTRACE=1 '{setpriv}' --no-new-privs /bin/sh -c {harness} \
             /bin/sh -c \
             {harness} '{1}' --exact {BROKER_TEST} --nocapture --test-threads=1 \
             > '{0}/broker.out' 2>&1",
            dir.display(),
            std::env::current_exe().unwrap().display(),
        )
    };
    if setpriv.is_some() {
        std::fs::write(claude.fixture.server(), "#!/bin/sh\nexec sleep 600\n").unwrap();
        // The package as installed: written by no one but its owner,
        // whatever the umask made its directories, or no box stands.
        for made in ["opt", "opt/docs", "opt/docs/bin"] {
            let shut = std::os::unix::fs::PermissionsExt::from_mode(0o755);
            std::fs::set_permissions(claude.fixture.root.join(made), shut).unwrap();
        }
    }
    let body = setpriv.as_deref().map_or(waits, serves);
    std::fs::write(&source, format!("#!/bin/sh\n{body}\n")).unwrap();
    let seat = super::super::survey_tests::installed(&dir, "seat", &source);
    let engine = &mut claude.fixture.engine;
    let body = &mut engine.bundle.seats.get_mut("work").unwrap().body;
    *program(body) = seat.display().to_string();
    let named = format!("{}\n{}\n", claude.fixture.root.display(), engine.run_id);
    std::fs::write(dir.join("fixture"), named).unwrap();
    claude.attacked();
    // The seat waits to be killed, so a dispatch that returns never stood it:
    // say why, for the parent to show.
    let (event, payload) = last(&claude.fixture.engine);
    panic!("the dispatch returned before its seat stood: {event:?} {payload}");
}

/// The fixture root and run [`serving_dispatch`] named in `dir`.
#[cfg(target_os = "linux")]
fn named(dir: &Path) -> (PathBuf, String) {
    let named = std::fs::read_to_string(dir.join("fixture")).unwrap();
    let (root, run) = named.trim().split_once('\n').unwrap();
    (PathBuf::from(root), run.to_string())
}

/// The directory of `run`'s lease on the fixture at `root`.
#[cfg(target_os = "linux")]
fn lease_dir(root: &Path, run: &str) -> PathBuf {
    let work = root.join("work");
    let repo = canonical::sha256_bytes(work.as_os_str().as_encoded_bytes());
    root.join("home").join(COORDINATION).join(repo).join(run)
}

/// The one plan the inventory of the one attempt in the run directory
/// `run` pins, its bytes checked against the pin, as a broker binds it.
#[cfg(target_os = "linux")]
fn pinned(run: &Path) -> brokkr_protocol::broker::Plan {
    let entries = std::fs::read_dir(run).unwrap();
    let entries = entries.map(|entry| entry.unwrap().path().join("inventory.json"));
    let inventories: Vec<PathBuf> = entries.filter(|path| path.is_file()).collect();
    let [inventory] = &inventories[..] else {
        panic!(
            "one attempt in {} holds an inventory: {inventories:?}",
            run.display()
        );
    };
    let inventory = std::fs::read(inventory).unwrap();
    let inventory: brokkr_protocol::broker::Inventory = serde_json::from_slice(&inventory).unwrap();
    let [pin] = &inventory.plans[..] else {
        panic!("one plan is pinned");
    };
    let bytes = std::fs::read(&pin.locator).unwrap();
    assert_eq!(canonical::sha256_bytes(&bytes), pin.digest, "as pinned");
    serde_json::from_slice(&bytes).unwrap()
}

/// [`serving_broker`]'s word on its box, `said`, written to `served` in
/// `dir`, then its own pid as the seat's.
#[cfg(target_os = "linux")]
fn standing(dir: &Path, said: &Value) {
    std::fs::write(dir.join("served"), said.to_string()).unwrap();
    let pid = dir.join("seat.pid");
    std::fs::write(dir.join("seat.pid.new"), std::process::id().to_string()).unwrap();
    std::fs::rename(dir.join("seat.pid.new"), pid).unwrap();
}

/// The seat's broker of [`serving_dispatch`], in the seat's own process,
/// inert without [`BROKER`]: the plan the attempt's inventory pins, its
/// server's box prepared on this host and launched as a broker launches
/// it ([`served_box`]). Its word: the box's first process, or why no box
/// stands.
#[cfg(target_os = "linux")]
#[test]
fn serving_broker() {
    let Some(dir) = std::env::var_os(BROKER).map(PathBuf::from) else {
        return;
    };
    let serving = std::panic::AssertUnwindSafe(|| {
        let (root, run) = named(&dir);
        let plan = pinned(&lease_dir(&root, &run));
        let Err(unserved) = served_box(&dir, &root, &plan);
        unserved.to_string()
    });
    // The child's one diagnostic, whether it refused or panicked: its
    // parent reads only this text.
    let said = match std::panic::catch_unwind(serving) {
        Ok(refused) => json!({ "refused": refused }),
        Err(panicked) => {
            let text = panicked.downcast_ref::<String>().cloned();
            let text =
                text.or_else(|| panicked.downcast_ref::<&str>().map(|text| text.to_string()));
            json!({ "panicked": text })
        }
    };
    standing(&dir, &said);
}

/// Why [`served_box`] stood no box: MB3's refusal, as the protocol made
/// it, or a box that never readied, with how its first process ended.
#[cfg(target_os = "linux")]
#[derive(Debug, thiserror::Error)]
enum Unserved {
    #[error(transparent)]
    Refused(#[from] brokkr_protocol::broker::Refusal),
    #[error("the box never readied: {0:?}")]
    Unready(std::io::Result<std::process::ExitStatus>),
}

/// The server box of `plan`, prepared from the plan's own intent on this
/// host and launched on its entry, the shell standing for its bootstrap as
/// in the protocol's own box test, which execs the server once the box
/// readied: its first process written ([`standing`]), then held until this
/// process is killed. Why it does not stand, otherwise.
#[cfg(target_os = "linux")]
fn served_box(
    dir: &Path,
    root: &Path,
    plan: &brokkr_protocol::broker::Plan,
) -> Result<std::convert::Infallible, Unserved> {
    use brokkr_protocol::hands::{ServerBox, ServerProfile, ServerProgram};
    use std::io::Read;
    use std::os::fd::AsFd;
    let home = root.join("home");
    let program = ServerProgram::resolve(&plan.connection.argv[0], &home)?;
    let (intent, bootstrap) = (&plan.intent, std::fs::canonicalize("/bin/sh").unwrap());
    let profile = ServerProfile {
        reach: &intent.reach,
        network: &intent.network,
        bootstrap: &bootstrap,
        writers: &intent.writers.uids,
    };
    let mut server = ServerBox::prepare(&program, &profile)?;
    let entry = server.entry(std::process::id())?;
    let handles: Vec<_> = server.handles().map(|(_, fd)| fd.as_fd()).collect();
    let executable = program.executable().display();
    let script = format!("printf ready > \"/proc/self/fd/$2\" && exec '{executable}'");
    let command = |control: i32, ready: i32, status: i32| {
        let numbers = [control, ready, status].map(|fd| fd.to_string());
        let args = ["-c", &script, "sh", &numbers[0], &numbers[1], &numbers[2]];
        args.map(String::from).to_vec()
    };
    let never: &(dyn Fn() -> bool + Sync) = &|| false;
    let startup = (Instant::now() + Duration::from_secs(30), never);
    let mut launched = ServerBox::launch(&entry, &handles, command, startup)?;
    let mut ready = [0; 5];
    if launched.ready.read_exact(&mut ready).is_err() {
        return Err(Unserved::Unready(launched.child.wait()));
    }
    let info: Value = serde_json::from_reader(&mut launched.info).unwrap();
    standing(dir, &json!({ "served": info["child-pid"] }));
    std::thread::sleep(Duration::from_secs(600));
    panic!("the seat was never killed");
}

/// Whether the server box whose first process is `pid` still serves: that
/// process holds one child, the package's program waiting as `sleep`,
/// within ten seconds.
#[cfg(target_os = "linux")]
fn serving(pid: u64) -> bool {
    let comm = |child: &str| std::fs::read_to_string(format!("/proc/{child}/comm"));
    within(Duration::from_secs(10), || {
        let children = std::fs::read_to_string(format!("/proc/{pid}/task/{pid}/children"));
        let children = children.unwrap_or_default();
        let children: Vec<&str> = children.split_whitespace().collect();
        matches!(children[..], [child] if comm(child).is_ok_and(|comm| comm == "sleep\n"))
    })
}

/// Whether `pid` is gone, or only a zombie, within five seconds.
#[cfg(target_os = "linux")]
fn ended(pid: u64) -> bool {
    within(Duration::from_secs(5), || {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
        let rest = stat.split(") ").nth(1);
        rest.is_none_or(|rest| rest.starts_with('Z'))
    })
}

/// Whether `holds` held at some reading within `bound`, read every 20 ms.
#[cfg(target_os = "linux")]
fn within(bound: Duration, holds: impl Fn() -> bool) -> bool {
    let until = Instant::now() + bound;
    let mut held = holds();
    while !held & (Instant::now() < until) {
        std::thread::sleep(Duration::from_millis(20));
        held = holds();
    }
    held
}

/// A process killed and reaped as its test unwinds, unless it already was.
#[cfg(target_os = "linux")]
struct Killed(std::process::Child);

#[cfg(target_os = "linux")]
impl Drop for Killed {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A seat outliving its engine, killed once its test kills it, or as its
/// test unwinds before then.
#[cfg(target_os = "linux")]
struct Seat(Option<rustix::process::Pid>);

#[cfg(target_os = "linux")]
impl Drop for Seat {
    fn drop(&mut self) {
        if let Some(pid) = self.0.take() {
            let _ = rustix::process::kill_process(pid, rustix::process::Signal::KILL);
        }
    }
}

/// Another worktree's zero-grant run, its one `work` site boxed to bind
/// `writable`, dispatched with the fence lifted on a host whose HOME is
/// `home`: its engine, and the file its driver touches if it starts.
#[cfg(target_os = "linux")]
fn incoming(home: &Path, writable: &Path) -> ((tempfile::TempDir, Engine), PathBuf) {
    let dir = tempfile::tempdir_in(scratch()).unwrap();
    let ran = std::fs::canonicalize(dir.path()).unwrap().join("ran");
    let touch = driver(&format!("touch '{}'", ran.display()));
    let body = crate::engine::tests::single_body(touch);
    let (dir, mut engine) = crate::engine::capability_tests::canonical_engine_in(dir, body);
    set_site_hands(&mut engine.bundle, "work", hands(&[writable]));
    let (home, bootstrap) = (home.to_path_buf(), PathBuf::from(BOOTSTRAP));
    let writer = brokkr_protocol::hands::ids().0;
    let _lift = McpFence::lift();
    let _host = StoodIn::new(Host {
        home,
        bootstrap,
        writer,
    });
    let effect = requested(&engine, "effect");
    execute_work(&mut engine, &[effect]).unwrap();
    ((dir, engine), ran)
}

/// A real engine, in a process of its own, dispatching the single brokered
/// seat with `payload` beneath it ([`serving_dispatch`]) on a fixture it
/// makes in `dir`, once its seat stands: the engine, the fixture's root and
/// run, and, where `payload` is a server box, the box's first process.
#[cfg(target_os = "linux")]
fn dispatched_engine(dir: &Path, payload: &Payload) -> (Killed, (PathBuf, String), Option<u64>) {
    let boxed = matches!(payload, Payload::Server(_));
    if let Payload::Server(setpriv) = payload {
        std::fs::write(dir.join("payload"), setpriv.as_os_str().as_encoded_bytes()).unwrap();
    }
    let mut engine = std::process::Command::new(std::env::current_exe().unwrap());
    engine.args(["--exact", SERVER, "--nocapture", "--test-threads=1"]);
    let said = std::fs::File::create(dir.join("engine.err")).unwrap();
    let engine = engine.env(SERVED, dir).stdout(std::process::Stdio::null());
    let mut engine = Killed(engine.stderr(said).spawn().unwrap());
    let (pid, deadline) = (
        dir.join("seat.pid"),
        Instant::now() + Duration::from_secs(60),
    );
    // What the engine and, where it served one, its broker said, for a
    // failure to name.
    let said = || {
        let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap_or_default();
        format!("{}{}", read("engine.err"), read("broker.out"))
    };
    while !pid.exists() {
        let ended = engine.0.try_wait().unwrap();
        assert!(ended.is_none(), "the engine ended ({ended:?}): {}", said());
        let started = Instant::now() < deadline;
        assert!(started, "the seat never started: {}", said());
        std::thread::sleep(Duration::from_millis(20));
    }
    let served = boxed.then(|| {
        let said = std::fs::read(dir.join("served")).unwrap();
        let said: Value = serde_json::from_slice(&said).unwrap();
        let first = said["served"].as_u64();
        first.unwrap_or_else(|| panic!("no server box stands: {said}"))
    });
    (engine, named(dir), served)
}

/// Engine death with its `payload` surviving, for a program source and a
/// system source: a real engine, in a process of its own, dispatches the
/// single brokered seat ([`dispatched_engine`]) and is killed while that
/// seat's driver lives on, and, where `payload` is one, the server box its
/// broker launched serves on beneath it. The lease it sealed stands byte
/// for byte; another worktree's dispatch whose box could write, through a
/// link, the server's package or `/usr/bin` is refused by MB3's identity
/// cause before its driver starts, the source's bytes unchanged, while the
/// payload lives and once it ended: a death proves no settlement. One whose
/// box writes only a directory apart runs beside it. Only the facts an
/// engine leaves once it proved the attempt settled — its terminal event
/// journaled and its positive settlement fact sealed, as a crash just
/// before its release leaves them — release it: the incoming dispatch's own
/// admission recovers them, and its driver runs.
#[cfg(target_os = "linux")]
fn killed_while(payload: &Payload) {
    let made = tempfile::tempdir_in(scratch()).unwrap();
    let dir = std::fs::canonicalize(made.path()).unwrap();
    let (mut engine, (root, run), served) = dispatched_engine(&dir, payload);
    let (home, work) = (root.join("home"), root.join("work"));
    let lease = lease_dir(&root, &run).join("reservation.json");
    let sealed = std::fs::read(&lease).unwrap();
    engine.0.kill().unwrap();
    engine.0.wait().unwrap();
    let pid = std::fs::read_to_string(dir.join("seat.pid")).unwrap();
    let mut seat = Seat(rustix::process::Pid::from_raw(pid.trim().parse().unwrap()));
    let alive = rustix::process::test_kill_process(seat.0.unwrap());
    assert_eq!(alive, Ok(()), "the seat outlived its engine");
    let serves = |when: &str| {
        let first = served.map(serving);
        assert_ne!(first, Some(false), "{when}: the server's box serves");
    };
    serves("once its engine died");
    let package = root.join("opt/docs");
    let system = std::fs::canonicalize("/usr/bin").unwrap();
    // Each the bytes kept, what a writer links toward and the input named:
    // the launch, reserved before the package it lies in (U8a2b, run four).
    let program = package.join("bin/server");
    let cases = [
        (program.clone(), package.clone(), program),
        // Named by the operating-system root it lies in (D6).
        (
            std::fs::canonicalize("/bin/sh").unwrap(),
            system,
            PathBuf::from("/usr"),
        ),
    ];
    let refused = |when: &str| {
        for (index, (bytes, target, served)) in cases.iter().enumerate() {
            let before = std::fs::read(bytes).unwrap();
            let link = dir.join(format!("{when}-{index}"));
            std::os::unix::fs::symlink(target, &link).unwrap();
            let ((_dir, engine), ran) = incoming(&home, &link);
            let (event, payload) = last(&engine);
            assert_eq!(event, EventType::EffectFailed, "{when}: {payload}");
            let (site, run, served) = ("work".into(), engine.run_id.clone(), served.clone());
            let refusal = Unadmitted::Identity(Unknown::Serving { site, run, served });
            assert_eq!(payload["error"], refusal.to_string(), "{when}");
            assert!(!ran.exists(), "{when}: its driver never started");
            assert_eq!(std::fs::read(bytes).unwrap(), before, "{when}");
        }
        assert_eq!(std::fs::read(&lease).unwrap(), sealed, "{when}: as sealed");
    };
    refused("while-it-lives");
    let apart = dir.join("apart");
    std::fs::create_dir(&apart).unwrap();
    let ((_dir, beside), ran) = incoming(&home, &apart);
    let (event, said) = last(&beside);
    assert_eq!(
        event,
        EventType::EffectSucceeded,
        "a disjoint worktree: {said}"
    );
    assert!(ran.exists(), "a disjoint worktree's driver ran beside it");
    serves("throughout");
    let pid = seat.0.take().unwrap();
    rustix::process::kill_process(pid, rustix::process::Signal::KILL).unwrap();
    let gone = served.map(ended);
    assert_ne!(gone, Some(false), "the server's box ended with its seat");
    refused("once-it-ended");
    let attempt = crate::engine::broker::admission::tests::proven_settled(&home, &work, &run);
    let mut journal = brokkr_store::Store::open(&root.join("forge.db")).unwrap();
    let ended =
        json!({"effect_id": "effect", "attempt_id": attempt, "result": {"result": "complete"}});
    let id = Some(attempt.clone());
    (journal.append_next(&run, EventType::EffectSucceeded, ended, None, id)).unwrap();
    let link = dir.join("settled");
    std::os::unix::fs::symlink(&package, &link).unwrap();
    let ((_dir, engine), ran) = incoming(&home, &link);
    let (event, payload) = last(&engine);
    assert_eq!(event, EventType::EffectSucceeded, "{payload}");
    assert!(ran.exists(), "once settled, its driver ran");
}

/// Names the payload [`killed_engine`] proves [`killed_while`] for: `seat`,
/// or the `setpriv` a server box's harness is confined by.
#[cfg(target_os = "linux")]
const KILLED: &str = "BROKKR_U8A2B_KILLED";

/// [`killed_engine`], as its parent runs it.
#[cfg(target_os = "linux")]
const KILLED_TEST: &str =
    "engine::capability_tests::provision_tests::brokered_tests::writers_tests::killed_engine";

/// [`killed_while`] for the payload [`KILLED`] names, alone in a test
/// process of its own ([`in_a_process_of_its_own`]). Inert without
/// [`KILLED`]; production reads no such variable.
#[cfg(target_os = "linux")]
#[test]
fn killed_engine() {
    let Some(payload) = std::env::var_os(KILLED) else {
        return;
    };
    let payload = match payload.to_str() {
        Some("seat") => Payload::Seat,
        _ => Payload::Server(PathBuf::from(payload)),
    };
    let own = rustix::process::getpid();
    rustix::process::set_child_subreaper(Some(own)).unwrap();
    killed_while(&payload);
    reaped(Duration::from_secs(10));
    println!("killed: its sources kept");
}

/// Every child this process adopted, reaped within `bound`, so none passes
/// on its exit to this binary's parent, a subreaper too, whose attempts
/// could read it as a stray.
#[cfg(target_os = "linux")]
fn reaped(bound: Duration) {
    use rustix::process::{wait, WaitOptions};
    let until = Instant::now() + bound;
    while Instant::now() < until {
        match wait(WaitOptions::NOHANG) {
            Ok(Some(_)) => {}
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return,
        }
    }
    panic!("a child of the killed engine outlived it");
}

/// [`killed_engine`] for `payload`, in a test process of its own, which
/// adopts the seat its killed engine orphans: an orphan goes to its nearest
/// subreaper, which a test binary is once any attempt has run in it, and
/// there an attempt of another test, begun before the seat was born, could
/// end it as its stray. Every attempt of this process begins after it.
#[cfg(target_os = "linux")]
fn in_a_process_of_its_own(payload: &std::ffi::OsStr) {
    let mut killed = std::process::Command::new(std::env::current_exe().unwrap());
    killed.args(["--exact", KILLED_TEST, "--nocapture", "--test-threads=1"]);
    let ran = killed.env(KILLED, payload).output().unwrap();
    let said = String::from_utf8_lossy(&ran.stdout);
    let told = String::from_utf8_lossy(&ran.stderr);
    assert!(ran.status.success(), "{said}{told}");
    assert!(said.contains("killed: its sources kept"), "{said}{told}");
}

/// [`killed_while`] its seat's driver lives, a shell that waits. Linux
/// only, as above.
#[cfg(target_os = "linux")]
#[test]
fn a_dispatched_engine_killed_while_its_seat_lives_keeps_its_servers_sources() {
    in_a_process_of_its_own("seat".as_ref());
}

/// [`killed_while`] its server's box lives: the seat's driver is the
/// seat's broker ([`serving_broker`]), which prepares the box of the plan
/// the engine pinned on this host and launches it, the package's program
/// serving inside it, as `brokkr broker serve` would (the shell standing
/// for the bootstrap). Needs a real box and `setpriv`: skipped, naming
/// which, where either is missing, as in a seat's own box (host result
/// pending there).
#[cfg(target_os = "linux")]
#[test]
fn a_dispatched_engine_killed_while_its_server_box_lives_keeps_its_sources() {
    let test = "a_dispatched_engine_killed_while_its_server_box_lives_keeps_its_sources";
    if !boxes_here(test) {
        return;
    }
    let Some(setpriv) = setpriv() else {
        let required = brokkr_protocol::hands::boundary_evidence_required();
        let reason = format!("{test}: no setpriv to confine the broker's harness here");
        return brokkr_protocol::hands::skip_boundary_proof(required, &reason);
    };
    in_a_process_of_its_own(setpriv.as_os_str());
}
