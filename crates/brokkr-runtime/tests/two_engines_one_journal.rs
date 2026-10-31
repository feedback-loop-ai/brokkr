//! Two real engines, one journal — the scenario the bug was recorded in.
//!
//! On 2026-09-02 two `brokkr run` processes shared one realm journal.
//! One was mid-review, streaming its driver's checkpoints; the other had
//! just started and was driving its intake. The second died 2.5 minutes
//! in with `sqlite: database is locked` after `run/started` and eighteen
//! more events had already landed cleanly — an exit 1 indistinguishable
//! from any other defect, and a run simply gone.
//!
//! This is that scenario, staged: two ENGINES (not raw `Store` calls),
//! in separate PROCESSES, driving real driver processes against one
//! file, while a third party deliberately takes and releases the
//! database's write lock underneath them. Processes rather than threads
//! for the reason `brokkr-store`'s `tests/concurrent_processes.rs`
//! already documents: POSIX advisory locks are held per process, so
//! same-process contention is mediated by SQLite's own in-memory
//! bookkeeping and never reaches the real file locks. Same-realm
//! parallel burns are separate processes, so the claim is spent where it
//! will actually be spent.
//!
//! What is proved: both engines run to completion, both journals verify
//! and fold, and neither process exits on Busy.

use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use brokkr_core::envelope::EventType;
use brokkr_core::fold::Run;
use brokkr_protocol::{Body, Message, ResultStatus};
use brokkr_runtime::{Bundle, Engine};
use brokkr_store::Store;
use serde_json::json;

/// Set on every child, so a child never re-enters the parent test, and
/// so the driver — which inherits its engine's environment — knows it is
/// being run as one rather than by a bare `cargo test`.
const DB_VAR: &str = "BROKKR_TWO_ENGINES_DB";
const BUNDLE_VAR: &str = "BROKKR_TWO_ENGINES_BUNDLE";
const ROLE_VAR: &str = "BROKKR_TWO_ENGINES_ROLE";

/// How many checkpoints each seat's driver streams. Every one is an
/// `effect/checkpointed` append on the shared journal, so this is the
/// dial that says how hard the two engines lean on one write lock.
const CHECKPOINTS: usize = 24;

const POLICY: &str = r#"{
  "phases": ["intake", "review", "done", "stop"],
  "initial": "intake",
  "terminal": ["done", "stop"],
  "rules": [
    {"id": "INTAKE-OK", "from": "intake", "result": "resolved", "next": "review",
     "reason": "framed"},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "done",
     "reason": "clean"},
    {"id": "REVIEW-HOLD", "from": "review", "result": "security-hold", "next": "stop",
     "severity": "hard", "reason": "held"}
  ]
}"#;

/// The argv that re-executes this test binary as one driver session.
///
/// Through `grep` because the driver protocol owns stdout and libtest
/// does not know that: a re-executed test binary prints `running 1
/// test` there before the test function is even entered, and the engine
/// would meet that line where it expects a message. One line-buffered
/// filter for the protocol's own shape keeps the driver inside the test
/// binary — no second crate, no shipped helper bin — while giving the
/// engine the clean stream it is entitled to.
fn driver_argv() -> Vec<String> {
    // The path is single-quoted AND forward-slashed: sh on the Windows
    // runners eats bare backslashes before exec ever sees the path
    // (`D:\a\brokkr` arrived as `D:abrokkr` on CI), and quoting alone
    // is not enough for every sh that shells this. Forward slashes are
    // valid Windows paths; single quotes keep spaces intact everywhere.
    let exe = std::env::current_exe()
        .unwrap()
        .display()
        .to_string()
        .replace('\\', "/");
    vec![
        "sh".into(),
        "-c".into(),
        format!(
            "exec '{exe}' --exact two_engines_one_journal::two_engines_driver_child --nocapture | grep --line-buffered '^{{'"
        ),
    ]
}

fn write_bundle(dir: &Path) -> PathBuf {
    let bundle = dir.join("bundle");
    std::fs::create_dir_all(bundle.join("roles")).unwrap();
    std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
    std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
    // Its seats hire an office over that driver, beside the bundle (U9b1).
    let driver = driver_argv();
    let driver: Vec<&str> = driver.iter().map(String::as_str).collect();
    crate::office::hire(dir, "two-engines", &driver, None);
    let seat = |result: &str| json!({"results": [result], "agent": "two-engines"});
    std::fs::write(
        bundle.join("bundle.json"),
        serde_json::to_string(&json!({
            "name": "two-engines",
            "policy": "policy.json",
            "seats": {"intake": seat("resolved"), "review": seat("clean")},
        }))
        .unwrap(),
    )
    .unwrap();
    bundle
}

// ---------------------------------------------------------------- driver

/// One driver session, spoken in Rust rather than in a shell script
/// because the effect and attempt ids it must echo back are the engine's
/// own uuids and are only known once the `start` message arrives.
///
/// Inert under a plain `cargo test`: without [`DB_VAR`] in the
/// environment there is no engine on the other end of stdin, and reading
/// for one would hang.
#[test]
fn two_engines_driver_child() {
    if std::env::var(DB_VAR).is_err() {
        return;
    }
    let stdin = std::io::stdin();
    let mut line = String::new();
    let say = |body: Body| {
        println!("{}", serde_json::to_string(&Message::new(body)).unwrap());
    };

    // Hello.
    if std::io::BufRead::read_line(&mut stdin.lock(), &mut line).unwrap() == 0 {
        return;
    }
    say(Body::Capabilities {
        driver: "two-engines".into(),
        version: "1".into(),
        supports: Vec::new(),
    });

    // Start — which carries the ids and the seat this session serves.
    line.clear();
    if std::io::BufRead::read_line(&mut stdin.lock(), &mut line).unwrap() == 0 {
        return;
    }
    let message: Message = serde_json::from_str(&line).unwrap();
    let Body::Start {
        effect_id,
        attempt_id,
        seat,
        ..
    } = message.body
    else {
        panic!("expected start, got {:?}", message.body);
    };

    say(Body::Accepted {
        effect_id: effect_id.clone(),
        attempt_id: attempt_id.clone(),
        session_ref: Some("session".into()),
    });
    // The live telemetry. Each one becomes an append on the shared
    // journal by the engine's own hand, paced the way a real seat paces
    // its progress lines — and each one a record the seat-record fence
    // admits (decision 0034, ruling 6), because a refused checkpoint
    // would end the attempt and prove nothing about the lock.
    for step in 0..CHECKPOINTS {
        say(Body::Checkpoint {
            effect_id: effect_id.clone(),
            attempt_id: attempt_id.clone(),
            data: json!({"step": "seat-turn", "turn": step + 1}),
        });
        std::thread::sleep(std::time::Duration::from_millis(4));
    }
    let result = match seat.as_str() {
        "intake" => "resolved",
        _ => "clean",
    };
    say(Body::Result {
        effect_id,
        attempt_id,
        status: ResultStatus::Succeeded,
        result: Some(json!({"result": result})),
        error: None,
    });
    // The engine closes the pipe; wait for it rather than exiting first.
    line.clear();
    let _ = std::io::BufRead::read_line(&mut stdin.lock(), &mut line);
}

// ---------------------------------------------------------------- engine

/// One whole `brokkr run`, in its own process, on the shared journal.
/// Inert without [`DB_VAR`], like the driver.
#[test]
fn two_engines_engine_child() {
    let (Ok(db), Ok(bundle_dir), Ok(role)) = (
        std::env::var(DB_VAR),
        std::env::var(BUNDLE_VAR),
        std::env::var(ROLE_VAR),
    ) else {
        return;
    };
    let bundle_dir = Path::new(&bundle_dir);
    let beside = bundle_dir.parent().unwrap();
    let (agents, adapters) = (beside.join("agents"), beside.join("adapters"));
    let bundle =
        Bundle::compile_with(bundle_dir, &agents, &adapters).expect("the shared bundle compiles");
    // The shipped patience, deliberately: this is the scenario as it
    // ran, so the budget is the one that ships. A budget shortened for
    // the test's convenience would prove a different discipline than the
    // one an operator gets — SQLite's busy handler is not a fair queue,
    // and the repo's own measurement puts the tail of a wait for the
    // write lock in the seconds. The budget running OUT is proved
    // deterministically elsewhere, with the lock held on purpose:
    // `brokkr-store`'s `a_lock_held_past_all_patience_...` and the
    // engine's `contention_tests`.
    let store = Store::open(Path::new(&db)).expect("a racing open still opens");
    let mut engine = Engine::start(store, bundle, &format!("Feature: {role}"), None)
        .expect("a racing start still starts");
    println!("run_id={}", engine.run_id);
    let end = engine.drive().expect("an engine never exits on Busy");
    assert_eq!(
        end.state.run,
        Run::Completed,
        "{role} ended {:?}",
        end.state.run
    );
}

// ---------------------------------------------------------------- parent

/// Take and release the journal's write lock, over and over, for as long
/// as the flag says. Deliberate contention: the two engines meet a held
/// lock on the great majority of their appends rather than on whichever
/// ones incidental timing happens to collide.
///
/// Each turn holds the lock for [`held_for`] of its number, then
/// releases it for 3 ms: 15 ms held on average, as before, but never on
/// a fixed period. A
/// fixed one locks into step with the waiter. Once SQLite's busy handler
/// has ramped it polls every 100 ms, so a churn turning every 20 ms (five
/// turns to a poll) shows each poll the same phase, and a waiter whose
/// phase falls inside the hold meets a held lock on every poll of its
/// patience. That is how an engine spent its whole 30 s on one append
/// under coverage on a hosted runner (#829): a fixed 15 ms hold turns in
/// about 19 ms on a quiet workstation, and an instrumented, loaded
/// build's overhead per turn is what closes the gap to 20. Measured on
/// 2026-10-24 against one poller with a 10 s budget: a fixed 20.0 ms
/// turn starved 3 of 100 waits for the whole budget, where this hold at
/// the same 20 ms mean turn waited at most 1.7 s in 400. The tail that
/// remains is the cost of meeting a 3 ms gap in every 18 at random, and
/// the shipped 30 s patience covers it many times over.
fn churn_the_write_lock(db: PathBuf, until: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.busy_timeout(std::time::Duration::from_secs(30))
        .unwrap();
    let mut turn = 0u64;
    while until.load(std::sync::atomic::Ordering::SeqCst) {
        turn += 1;
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        brokkr_store::test_support::plant_run(&conn, &format!("churn-{turn}"), "{}").unwrap();
        std::thread::sleep(held_for(turn));
        conn.execute_batch("ROLLBACK").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(3));
    }
}

/// How long the churn holds the lock on its `turn`th take: one of eight
/// lengths from 8 to 22 ms, picked by the top three bits of the turn's
/// splitmix64 hash. A hash rather than a golden-ratio step, because a
/// low-discrepancy sequence keeps the sum of any five turns nearly
/// constant, and that sum is the period a 100 ms poll sees.
fn held_for(turn: u64) -> std::time::Duration {
    let mut z = turn.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    let step = (z ^ (z >> 31)) >> 61;
    std::time::Duration::from_millis(8 + 2 * step)
}

/// The recorded scenario, proved: one engine mid-phase streaming
/// checkpoints while a second starts a run and drives its intake in the
/// same store, both under a peer that keeps taking the write lock away.
/// Both processes finish, both journals fold, and neither exits on Busy.
#[test]
fn two_real_engines_share_one_journal_and_neither_exits_on_busy() {
    // Guard against a child re-entering the parent.
    if std::env::var(DB_VAR).is_ok() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("realm.db");
    let bundle = write_bundle(dir.path());
    // Each child engine is given an isolated, owner-only HOME, so its
    // admission resolves its coordination root under this temporary tree
    // and never the operator's real HOME (decision 0065 slice two, U8a2a).
    let home = dir.path().join("home");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&home)
        .unwrap();
    // The journal must exist before the churn thread can lock it; the
    // engines then race each other on everything that follows.
    drop(Store::open(&db).unwrap());

    let churning = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let churn = std::thread::spawn({
        let (db, flag) = (db.clone(), std::sync::Arc::clone(&churning));
        move || churn_the_write_lock(db, flag)
    });

    let spawn = |role: &str| {
        // Each engine operates a worktree of its own beside the one
        // journal, as parallel lanes do: two runs on one worktree are
        // serialized by its writer lease (decision 0065 slice two, U8a2a).
        let worktree = dir.path().join(role);
        std::fs::create_dir_all(&worktree).unwrap();
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "two_engines_one_journal::two_engines_engine_child",
                "--nocapture",
            ])
            // The child starts without a repository, so it operates the
            // directory it stands in: the scratch one, never this tree.
            .current_dir(&worktree)
            .env("HOME", &home)
            .env(DB_VAR, &db)
            .env(BUNDLE_VAR, &bundle)
            .env(ROLE_VAR, role)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the test binary re-executes")
    };

    // The first engine gets a head start, so by the time the second one
    // opens the journal and lands its `run/started` the first is already
    // mid-phase with a driver streaming into it — the shape the bug was
    // recorded in.
    let first = spawn("mid-phase");
    std::thread::sleep(std::time::Duration::from_millis(300));
    let second = spawn("fresh-start");

    let mut run_ids = Vec::new();
    for (role, child) in [("mid-phase", first), ("fresh-start", second)] {
        let out = child.wait_with_output().expect("child engine is waitable");
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        assert!(
            out.status.success(),
            "the {role} engine exited {}: {stderr}",
            out.status
        );
        // The exact words the recorded death printed. Their absence is
        // the point of the whole exercise.
        assert!(
            !stderr.contains("database is locked"),
            "the {role} engine met an unhandled lock: {stderr}"
        );
        run_ids.push(
            stdout
                .lines()
                .find_map(|line| line.strip_prefix("run_id="))
                .unwrap_or_else(|| panic!("the {role} engine printed no run id: {stdout}"))
                .to_string(),
        );
    }
    churning.store(false, std::sync::atomic::Ordering::SeqCst);
    churn.join().unwrap();

    assert_ne!(run_ids[0], run_ids[1], "both engines drove the same run");
    let reader = Store::open(&db).unwrap();
    for run_id in &run_ids {
        // `load` verifies the chain and refuses to return a partial one.
        let events = reader.load(run_id).unwrap();
        let state = brokkr_core::fold(&events).expect("both journals fold cleanly");
        assert_eq!(state.run, Run::Completed, "{run_id} did not complete");
        assert_eq!(state.seq, events.len() as u64);
        let checkpoints = events
            .iter()
            .filter(|e| e.event_type == EventType::EffectCheckpointed)
            .count();
        assert_eq!(
            checkpoints,
            CHECKPOINTS * 2,
            "{run_id} lost checkpoints under contention"
        );
    }
}
