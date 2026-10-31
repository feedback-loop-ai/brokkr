//! Task 46.2 (U9b1b; MB3): the two-worktree lifetime matrix through real
//! dispatch. A second repository under the same HOME runs a boxed exec seat
//! that may write the first's server package. Whichever arrives second is
//! refused; together exactly one is admitted; the reservation outlives its
//! engine's death and stands through an uncertain settlement; and a writer
//! that cannot reach the package runs beside the server throughout.

use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

use serde_json::json;

use super::{ended, put, Ended, World, REVIEWS};

/// How long anything here is waited for: a survey under load, and its bound.
const PATIENCE: Duration = Duration::from_secs(90);

/// What a writer's exec seat does until `release` stands: says it is live,
/// then waits; then its result, as `complete`.
fn writing(live: &Path, release: &Path) -> String {
    let answer = REVIEWS.replace("clean", "complete");
    format!(
        "touch '{}'\nwhile [ ! -e '{}' ]; do sleep 0.05; done\n{answer}",
        live.display(),
        release.display()
    )
}

/// The second worktree: its own repository and `.brokkr/` under the world's
/// HOME, whose `work` seat is a boxed exec seat with `hands`.
struct Second<'w> {
    world: &'w World,
    repo: PathBuf,
    live: PathBuf,
    release: PathBuf,
}

impl<'w> Second<'w> {
    /// `writes` is what its seat's hands bind writable beside `signals`, the
    /// one place its seat and this test both see.
    fn new(world: &'w World, writes: &Path) -> Second<'w> {
        let repo = world.base.join("second");
        std::fs::create_dir(&repo).unwrap();
        crate::brokkr_folder::git(&repo, &["init", "-q"]);
        let signals = world.base.join("signals");
        std::fs::create_dir(&signals).unwrap();
        let (live, release) = (signals.join("second-live"), signals.join("second-go"));
        let hands = json!({"kind": "workspace", "network": false, "binds": [
            {"path": writes, "mode": "rw"}, {"path": signals, "mode": "rw"}]});
        let folder = repo.join(".brokkr");
        put(&folder.join("policy.json"), crate::writer_lease::POLICY);
        put(&folder.join("roles/role.md"), "# role\n");
        let seat = |script: &str, result: &str| {
            json!({"results": [result], "role": "roles/role.md", "hands": hands,
                   "driver": {"command": ["{brokkr}", "driver", "exec", "--", "sh", "-c", script,
                                          "seat", "{prompt_file}"]}})
        };
        let work = writing(&live, &release);
        let bundle = json!({"name": "second", "policy": "policy.json", "seats": {
            "work": seat(&work, "complete"), "review": seat(REVIEWS, "clean")}});
        put(&folder.join("bundle.json"), bundle.to_string());
        let realms = json!({"schema": "forge.realms/v6", "journal": "state/forge.db",
            "realms": [{"name": "second", "path": "..", "default_branch": "main"}]});
        put(&folder.join("realms.json"), realms.to_string());
        Second {
            world,
            repo,
            live,
            release,
        }
    }

    /// Its run, started.
    fn start(&self) -> Child {
        let mut command = self.world.brokkr(&["run", "--feature", "second"]);
        command
            .current_dir(&self.repo)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    }

    /// Its run, driven to its end or its park, let through once its seat is
    /// live.
    fn run(&self) -> Ended {
        std::fs::write(&self.release, "").unwrap();
        let mut command = self.world.brokkr(&["run", "--feature", "second"]);
        ended(command.current_dir(&self.repo).output().unwrap())
    }
}

fn awaited(what: &str, holds: impl Fn() -> bool) {
    let until = Instant::now() + PATIENCE;
    while !holds() {
        assert!(Instant::now() < until, "waited for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The refusal a reaching writer takes beside the first world's admission.
fn refused_for_reach(ended: &Ended, package: &Path) {
    let cause = ended.cause.clone().unwrap_or_default();
    assert_eq!(
        (
            ended.status.as_str(),
            cause.starts_with("dispatch refused:"),
            cause.contains(package.to_str().unwrap()),
        ),
        ("awaiting_operator", true, true),
        "{ended:?}"
    );
}

/// One start order: a server serving, then a writer that reaches its package.
/// The writer starts nothing, and the session it arrived beside completes.
#[test]
fn a_writer_arriving_beside_a_serving_broker_is_refused_and_the_session_stands() {
    let Some(world) = World::new() else {
        return;
    };
    let second = Second::new(&world, &world.base.join("opt/docs"));
    let first = world.held_serving();
    let refused = second.run();
    refused_for_reach(&refused, &world.base.join("opt/docs"));
    assert!(!second.live.exists(), "its seat never started");
    std::fs::remove_file(world.bin("hold")).unwrap();
    let out = first.wait_with_output().unwrap();
    let done = ended(out);
    assert_eq!(done.status, "completed", "{done:?}");
    assert_eq!(world.broker_code(), Some(0));
}

/// The other start order: a writer running, then a seat whose server's package
/// it reaches. The seat is refused before any lookup and its harness never
/// starts; the writer's run is untouched, and completes.
#[test]
fn a_server_arriving_beside_a_running_writer_is_refused_with_no_lookup_or_start() {
    let Some(world) = World::new() else {
        return;
    };
    let second = Second::new(&world, &world.base.join("opt/docs"));
    let mut writer = second.start();
    awaited("the writer's seat", || second.live.exists());
    let reads = world.store_reads();
    let refused = world.run();
    refused_for_reach(&refused, &world.base.join("opt/docs"));
    assert_eq!((reads.lookups(), world.starts()), (0, 0));
    std::fs::write(&second.release, "").unwrap();
    assert_eq!(writer.wait().unwrap().code(), Some(0));
}

/// Simultaneous admission is atomic: of a server and a writer that reaches it,
/// started together, never both are admitted.
#[test]
fn a_server_and_a_writer_started_together_are_never_both_admitted() {
    let Some(world) = World::new() else {
        return;
    };
    let second = Second::new(&world, &world.base.join("opt/docs"));
    std::fs::write(&second.release, "").unwrap();
    put(&world.bin("hold"), "");
    let args = world.run_args();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let (one, two) = std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            let out = world.brokkr(&args).output().unwrap();
            ended(out)
        });
        let writer = scope.spawn(|| {
            let mut command = world.brokkr(&["run", "--feature", "second"]);
            ended(command.current_dir(&second.repo).output().unwrap())
        });
        // The harness holds only where the server was admitted; let it on.
        let until = Instant::now() + PATIENCE;
        while !first.is_finished() && Instant::now() < until {
            if world.bin("harness.pid").exists() {
                std::fs::remove_file(world.bin("hold")).ok();
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        std::fs::remove_file(world.bin("hold")).ok();
        (first.join().unwrap(), writer.join().unwrap())
    });
    let admitted = (one.status == "completed", two.status == "completed");
    let refused = |run: &Ended| {
        run.cause
            .as_deref()
            .is_some_and(|cause| cause.starts_with("dispatch refused:"))
    };
    assert!(
        matches!(admitted, (true, false) | (false, true) | (false, false)),
        "{one:?} {two:?}"
    );
    assert!(refused(&one) || refused(&two), "{one:?} {two:?}");
}

/// The reservation outlives its engine: killed while its harness serves, the
/// attempt keeps it against a later writer, through the restarted engine's
/// uncertain settlement too, since nothing proves its processes settled.
#[test]
fn a_killed_engines_reservation_refuses_a_later_writer_through_an_uncertain_settlement() {
    let Some(world) = World::new() else {
        return;
    };
    let second = Second::new(&world, &world.base.join("opt/docs"));
    let run = super::killed(world.held_serving());
    let refused = second.run();
    refused_for_reach(&refused, &world.base.join("opt/docs"));
    assert!(!second.live.exists());
    std::fs::remove_file(world.bin("hold")).unwrap();
    awaited("the harness to end", || world.broker_code().is_some());
    let resumed = world.brokkr(&["resume", "--run", &run]).output().unwrap();
    let parked = ended(resumed);
    assert_eq!(parked.status, "awaiting_operator", "{parked:?}");
    let refused = second.run();
    refused_for_reach(&refused, &world.base.join("opt/docs"));
    assert!(!second.live.exists());
}

/// Recovery (U8e3): the run a killed engine left, settled indeterminate at its
/// restart, holds its worktree until the operator confirms it settled. Only
/// then does a fresh run admit, its own plan serving it, and its call the only
/// one it carries.
#[test]
fn a_killed_engines_run_released_by_the_operator_lets_a_fresh_run_be_served() {
    let Some(world) = World::new() else {
        return;
    };
    let run = super::killed(world.held_serving());
    std::fs::remove_file(world.bin("hold")).unwrap();
    awaited("the harness to end", || world.broker_code().is_some());
    let resumed = world.brokkr(&["resume", "--run", &run]).output().unwrap();
    assert_eq!(ended(resumed).status, "awaiting_operator");
    let held = world.run();
    let cause = held.cause.clone().unwrap_or_default();
    assert!(cause.contains(&run), "held by the first run: {held:?}");
    let db = world.repo.join(".brokkr/state/forge.db");
    let args = [
        "operator",
        "--run",
        &run,
        "--reason",
        "its processes are gone",
        "release",
        "--db",
        db.to_str().unwrap(),
    ];
    let out = world.brokkr(&args).output().unwrap();
    let said = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{said}");
    let fresh = world.completes();
    assert_eq!(world.calls(&fresh.run).len(), 1);
}

/// The positive control: a writer kept apart from the package runs while the
/// server serves, and both complete.
#[test]
fn a_disjoint_writer_runs_beside_a_serving_broker_and_both_complete() {
    let Some(world) = World::new() else {
        return;
    };
    let apart = world.base.join("apart");
    std::fs::create_dir(&apart).unwrap();
    super::chmod(&apart, 0o755);
    let second = Second::new(&world, &apart);
    let first = world.held_serving();
    let beside = second.run();
    assert_eq!(beside.status, "completed", "{beside:?}");
    std::fs::remove_file(world.bin("hold")).unwrap();
    let done = ended(first.wait_with_output().unwrap());
    assert_eq!(done.status, "completed", "{done:?}");
}
