//! `brokkr dispatch` (decision 0110 rulings 3 to 6) in the process: the
//! standing loop's interval, its sleep and its reaping, the bound's
//! command, the holder's readout that writes nothing, a pass that holds
//! the journal's lock, a driver's spawn refused by its log, its entry or
//! its directory, and the session a driver leads. The built binary's
//! passes and its standing loop are `it`'s `dispatch` tests.

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use brokkr_runtime::dispatch::{take_dispatch, Dispatcher, DispatcherError};
use brokkr_store::{Attribution, Claim, Claiming, NewEntry, Store};
use clap::Parser;

use super::*;
use crate::tests::env_guard::EnvGuard;
use crate::{Cli, Cmd, GroupedCmd};

/// `brokkr` given `argv`, as its dispatch command.
fn parsed(argv: &[&str]) -> DispatchArgs {
    let argv = [&["brokkr", "dispatch"], argv].concat();
    let Cmd::Grouped(GroupedCmd::Dispatch(args)) = Cli::try_parse_from(argv).unwrap().command
    else {
        panic!("parsed as a dispatch command")
    };
    args
}

/// Run `brokkr dispatch` given `argv` in `dir`: its exit and its lines.
fn ran(dir: &Path, argv: &[&str]) -> (ExitCode, String) {
    let mut out = Vec::new();
    let exit = dispatch(dir, parsed(argv), &mut out).unwrap();
    (exit, String::from_utf8(out).unwrap())
}

/// A new journal in `dir`, and a store on it.
pub(super) fn journal(dir: &Path) -> (PathBuf, Store) {
    let path = dir.join("forge.db");
    let store = Store::open(&path).unwrap();
    (path, store)
}

/// One entry of `payload` queued and claimed by the dispatcher that takes
/// the journal: the dispatcher, still holding it, and the claim.
fn claimed_of(store: &mut Store, payload: &str) -> (Dispatcher, Claim) {
    let dispatcher = take_dispatch(store).unwrap().unwrap();
    let claim = claimed_by(store, &dispatcher, payload);
    (dispatcher, claim)
}

/// One more entry of `payload` queued and claimed by `dispatcher`, which
/// holds the journal `store` is open on.
pub(super) fn claimed_by(store: &mut Store, dispatcher: &Dispatcher, payload: &str) -> Claim {
    let (priority, waits) = (0, &[][..]);
    let (operator, reason) = ("vy", "queued");
    let added = store.queue_add(
        NewEntry {
            payload,
            priority,
            waits,
        },
        Attribution { operator, reason },
    );
    let (generation, now) = (dispatcher.generation, time::OffsetDateTime::UNIX_EPOCH);
    let entry = &store.queue_entry(added.unwrap()).unwrap();
    let (high_water, digest) = (0, "d");
    let over = Claiming {
        entry,
        high_water,
        digest,
        generation,
        now,
    };
    let judged = store.queue_claim_with(&over, "r", |_| Ok::<(), ()>(()));
    judged.unwrap().unwrap()
}

/// [`claimed_of`] an entry whose payload names no launch.
fn claimed(store: &mut Store) -> (Dispatcher, Claim) {
    claimed_of(store, "{}")
}

/// A journal in a directory of the test's own, holding one entry claimed
/// by the dispatcher that took it, which still holds it.
pub(super) struct OneClaim {
    pub(super) dir: tempfile::TempDir,
    pub(super) db: PathBuf,
    pub(super) _held: (Store, Dispatcher),
    pub(super) claim: Claim,
}

/// [`OneClaim`] of an entry of `payload`.
pub(super) fn one_claim(payload: &str) -> OneClaim {
    let dir = tempfile::tempdir().unwrap();
    let (db, mut store) = journal(dir.path());
    let (dispatcher, claim) = claimed_of(&mut store, payload);
    OneClaim {
        dir,
        db,
        _held: (store, dispatcher),
        claim,
    }
}

/// Ruling 5: `--every` takes 5 to 3600 seconds, 60 by default, and
/// never beside `--once`.
#[test]
fn every_is_five_to_3600_seconds_sixty_by_default_and_refused_with_once() {
    assert_eq!(parsed(&[]).every, 60);
    assert_eq!((parsed(&["--every", "5"]).every), 5);
    assert_eq!((parsed(&["--every", "3600"]).every), 3600);
    for (argv, kind) in [
        (
            &["--every", "4"][..],
            clap::error::ErrorKind::ValueValidation,
        ),
        (
            &["--every", "3601"],
            clap::error::ErrorKind::ValueValidation,
        ),
        (
            &["--every", "5", "--once"],
            clap::error::ErrorKind::ArgumentConflict,
        ),
    ] {
        let argv = [&["brokkr", "dispatch"], argv].concat();
        let refused = Cli::try_parse_from(&argv).err().expect("refused");
        assert_eq!(refused.kind(), kind, "{argv:?}");
    }
}

/// Ask `stop` as its signal handlers do: the flag, then a byte to `waker`.
fn ask(stop: &AtomicBool, mut waker: &UnixStream) {
    stop.store(true, Ordering::SeqCst);
    waker.write_all(&[0]).unwrap();
}

/// The standing loop's sleep with no signal: the whole of its rest, and
/// not less, and none at all once a stop is asked.
#[test]
fn a_nap_with_no_signal_sleeps_its_whole_rest() {
    let (stop, _waker) = Stop::unasked().unwrap();
    let rest = Duration::from_millis(300);
    let began = Instant::now();
    nap(&stop, rest);
    let slept = began.elapsed();
    assert!(
        slept >= rest && slept < rest + Duration::from_secs(2),
        "{slept:?}"
    );

    let (stop, waker) = Stop::unasked().unwrap();
    ask(&stop.asked, &waker);
    let began = Instant::now();
    nap(&stop, Duration::from_secs(60));
    assert!(began.elapsed() < Duration::from_millis(500), "woke at once");
}

/// Decision 0110 ruling 5: a stop asked while the loop sleeps a long rest
/// wakes it as the signal's byte lands, not at the rest's end or a beat's.
/// The asker and the timer share one origin, taken before either starts,
/// and the asker says when it asked, so a busy host cannot order them the
/// wrong way round. Removal control (ii), the nap sleeping its whole rest,
/// fails it after the rest's 60 s.
#[test]
fn a_stop_asked_mid_nap_wakes_it_promptly() {
    let (stop, waker) = Stop::unasked().unwrap();
    let asks = Arc::clone(&stop.asked);
    let began = Instant::now();
    let asker = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200).saturating_sub(began.elapsed()));
        let asking = began.elapsed();
        ask(&asks, &waker);
        asking
    });
    nap(&stop, Duration::from_secs(60));
    let slept = began.elapsed();
    let asking = asker.join().unwrap();
    assert!(
        slept >= asking && slept < asking + Duration::from_millis(500),
        "woke {slept:?} after the origin, the stop asked at {asking:?}"
    );
}

/// A child that runs `script` in a shell.
fn child(script: &str) -> Child {
    Command::new("sh").args(["-c", script]).spawn().unwrap()
}

/// Ruling 5: reaping takes out each driver that exited, without waiting on
/// one still running, which stays for the next tick.
#[test]
fn reaping_takes_each_exited_driver_and_keeps_one_still_running() {
    let mut exited = child("exit 3");
    let pid = exited.id();
    let running = child("sleep 30");
    let kept = running.id();
    exited.wait().unwrap();
    // A waited child's status is cached on its handle, which `try_wait`
    // reads back, so the reap does not race the exit.
    let OneClaim { claim, .. } = one_claim("{}");
    let mut drivers = Drivers::new();
    drivers.insert(pid, ("run-done".into(), claim, exited));
    drivers.insert(kept, ("run-on".into(), claim, running));
    let reaped = reaped(&mut drivers);
    let [Exited {
        run,
        claim: handed,
        pid: at,
        exit: Ok(exit),
    }] = reaped.as_slice()
    else {
        panic!("{reaped:?}");
    };
    assert_eq!(
        (run.as_str(), *handed, *at, exit.code()),
        ("run-done", claim, pid, Some(3))
    );
    assert_eq!(drivers.keys().copied().collect::<Vec<_>>(), [kept]);
    let (.., mut running) = drivers.remove(&kept).unwrap();
    running.kill().unwrap();
    running.wait().unwrap();
}

/// Ruling 5: a driver whose handle cannot say how it exited, here because
/// it was waited on behind the handle's back, as the system does when
/// SIGCHLD is inherited ignored, is reaped with why rather than kept as
/// running for ever.
#[test]
fn a_driver_whose_handle_does_not_read_is_reaped_with_why() {
    let exited = child("exit 0");
    let pid = exited.id();
    let waited = rustix::process::Pid::from_raw(pid as i32);
    rustix::process::waitpid(waited, rustix::process::WaitOptions::empty()).unwrap();
    let OneClaim { claim, .. } = one_claim("{}");
    let mut drivers = Drivers::new();
    drivers.insert(pid, ("run-gone".into(), claim, exited));
    let reaped = reaped(&mut drivers);
    let [Exited {
        run,
        pid: at,
        exit: Err(error),
        ..
    }] = reaped.as_slice()
    else {
        panic!("{reaped:?}");
    };
    assert_eq!(
        (run.as_str(), *at, error.raw_os_error()),
        (
            "run-gone",
            pid,
            Some(rustix::io::Errno::CHILD.raw_os_error())
        )
    );
    assert!(drivers.is_empty());
}

#[test]
fn set_bound_refuses_zero_and_what_is_not_a_whole_number() {
    for bound in ["0", "x", "1.5", "4294967296"] {
        let argv = ["brokkr", "dispatch", "set-bound", bound, "--reason", "r"];
        let refused = Cli::try_parse_from(argv).err().expect("refused");
        assert_eq!(
            refused.kind(),
            clap::error::ErrorKind::ValueValidation,
            "{bound}"
        );
    }
}

/// Ruling 6: `set-bound 2` is journaled as the operator's command, and the
/// next pass reads it.
#[test]
fn set_bound_two_governs_the_next_pass() {
    let dir = tempfile::tempdir().unwrap();
    let (db, mut store) = journal(dir.path());
    let argv = [
        "set-bound",
        "2",
        "--reason",
        "two refusals",
        "--db",
        db.to_str().unwrap(),
    ];
    let (exit, said) = ran(dir.path(), &argv);
    assert_eq!(exit, ExitCode::from(Exit::Completed));
    assert_eq!(said, "bound: 2 counted refusals hold an entry\n");
    let held = take_dispatch(&mut store).unwrap().unwrap();
    let absent = dir.path().join("absent-host.json");
    let host = brokkr_runtime::admission::Host {
        file: &absent,
        temp: dir.path(),
        free: &|_: &Path| Ok(1 << 40),
    };
    let seams = Seams {
        stands: || Ok(()),
        claims: || true,
        spawn: |_: &Spawn| Ok(Spawned { pid: 1 }),
    };
    let passed = dispatch::pass(&mut store, &held, &host, OsString::new(), seams).unwrap();
    assert!(matches!(
        passed.bound,
        Some(dispatch::BoundOutcome::Read(bound)) if bound == NonZeroU32::new(2).unwrap()
    ));
    let conn = rusqlite::Connection::open(&db).unwrap();
    let setting: (i64, String, String) = conn
        .query_row(
            "SELECT bound, operator, reason FROM dispatch_settings",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(setting, (2, operator_name(), "two refusals".into()));
}

/// The journal's bytes, as a digest.
fn digest(db: &Path) -> String {
    brokkr_core::canonical::sha256_bytes(&std::fs::read(db).unwrap())
}

/// `brokkr dispatch status` of `db`, run in `dir`, which leaves the
/// journal's bytes as they were.
fn status_of(dir: &Path, db: &Path) -> String {
    let before = digest(db);
    let (exit, said) = ran(dir, &["status", "--db", db.to_str().unwrap()]);
    assert_eq!(exit, ExitCode::from(Exit::Completed));
    assert_eq!(digest(db), before, "status wrote nothing");
    said
}

/// The holder row `store` records, as status names it.
fn recorded(store: &Store) -> String {
    let (generation, pid, since) = store.dispatch_holder().unwrap().unwrap();
    format!(
        "generation {} (pid {pid} on this host since {since})",
        generation.0
    )
}

#[test]
fn status_names_a_live_holder_held_and_a_dead_ones_journal_not_held() {
    let dir = tempfile::tempdir().unwrap();
    let (db, mut store) = journal(dir.path());
    let bound = "bound: 3 counted refusals hold an entry\n";
    assert_eq!(
        status_of(dir.path(), &db),
        format!("not held; no dispatcher has taken this journal\n{bound}")
    );

    let (dispatcher, claim) = claimed(&mut store);
    let (entry, seq) = claim.parts();
    let holder = recorded(&store);
    assert_eq!(
        status_of(dir.path(), &db),
        format!("held by {holder}\n{bound}launching: claim {seq} of entry {entry}\n")
    );

    store
        .create_run("run-1", "f", "b", &serde_json::json!({}))
        .unwrap();
    store
        .queue_link(claim, dispatcher.generation, "run-1", "r")
        .unwrap();
    drop(dispatcher);
    assert_eq!(
        status_of(dir.path(), &db),
        format!(
            "not held; last held by {holder}\n{bound}linked with no spawn row: run run-1 under \
             claim {seq} of entry {entry}\n"
        )
    );
}

#[test]
fn status_names_a_lock_held_before_its_holder_row_is_recorded() {
    let dir = tempfile::tempdir().unwrap();
    let (db, _store) = journal(dir.path());
    // Owner-only, as the shared lock-file home makes one: a lock file a
    // dispatcher would refuse is no dispatcher's to hold.
    let lock = std::os::unix::fs::OpenOptionsExt::mode(
        std::fs::OpenOptions::new().write(true).create(true),
        0o600,
    )
    .open(dir.path().join("forge.db.dispatch.lock"))
    .unwrap();
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive).unwrap();
    assert_eq!(
        status_of(dir.path(), &db),
        "held by a dispatcher that has not recorded itself yet\nbound: 3 counted refusals \
         hold an entry\n"
    );
}

#[test]
fn a_pass_is_refused_while_another_dispatcher_holds_the_journal() {
    let dir = tempfile::tempdir().unwrap();
    let (db, mut store) = journal(dir.path());
    let _held = take_dispatch(&mut store).unwrap().unwrap();
    let args = parsed(&["--once", "--db", db.to_str().unwrap()]);
    let refused = dispatch(dir.path(), args, &mut Vec::new()).unwrap_err();
    assert!(
        matches!(
            refused.downcast_ref::<DispatcherError>(),
            Some(DispatcherError::Dispatched(at, Some(_))) if *at == db
        ),
        "{refused:?}"
    );
}

/// A pass over an empty queue on a machine of the test's own: it sweeps,
/// reads the bound, finds no host and claims nothing, and exits 0.
#[test]
fn a_pass_over_an_empty_queue_says_what_it_read_and_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let places = std::fs::canonicalize(dir.path()).unwrap();
    let mut env = EnvGuard::lock();
    env.set("XDG_STATE_HOME", places.join("state"));
    env.set("XDG_CONFIG_HOME", places.join("config"));
    let db = places.join("forge.db");
    let (exit, said) = ran(&places, &["--once", "--db", db.to_str().unwrap()]);
    assert_eq!(exit, ExitCode::from(Exit::Completed));
    assert!(said.ends_with("queue: 0 waiting, 0 held\n"), "{said}");
    let (_, store) = journal(&places);
    assert_eq!(store.dispatch_holder().unwrap().unwrap().0 .0, 1);
}

/// A driver handed `claim`'s run `run-1` under `journal`.
pub(super) fn spawn_of(claim: Claim) -> Spawn {
    Spawn {
        entry: claim.parts().0,
        claim,
        generation: brokkr_store::Generation(1),
        run: "run-1".into(),
    }
}

/// This test binary, as the dispatcher's binary.
fn this_binary() -> Binary {
    Binary::started(&Executable::current()).unwrap()
}

#[test]
fn a_driver_whose_log_entry_or_directory_refuses_is_not_spawned() {
    let OneClaim { dir, db, claim, .. } = one_claim("{}");
    let spawn = spawn_of(claim);

    // The entry's payload names no workspace.
    let refused = driver(&this_binary(), &db, &spawn).unwrap_err();
    let SpawnFailed::Refused(why) = refused;
    assert!(why.starts_with("reading a queued launch"), "{why}");

    // A log in a directory others may enter.
    let runs = dir.path().join("forge.db.runs");
    let mode = std::os::unix::fs::PermissionsExt::from_mode(0o755);
    std::fs::set_permissions(&runs, mode).unwrap();
    let SpawnFailed::Refused(why) = driver(&this_binary(), &db, &spawn).unwrap_err();
    assert_eq!(
        why,
        format!(
            "dispatch: the driver log {} is not this user's own single file in a directory of \
             mode 0700, so the run's driver was not started (decision 0110)",
            runs.join("run-1.log").display()
        )
    );
}

#[test]
fn a_driver_whose_workspace_is_gone_is_not_spawned() {
    let gone = std::env::temp_dir().join("brokkr-dispatch-no-such-workspace");
    let payload = serde_json::json!({"encoding": "queued-launch/v1", "workspace": gone,
        "bundle": {"dir": "bundle"}, "repo": null, "secrets": null, "feature": "x",
        "map": "unmapped", "dispatch": null});
    let OneClaim { dir, db, claim, .. } = one_claim(&payload.to_string());
    let spawn = spawn_of(claim);
    let SpawnFailed::Refused(why) = driver(&this_binary(), &db, &spawn).unwrap_err();
    assert!(why.contains("No such file or directory"), "{why}");
    assert!(dir.path().join("forge.db.runs/run-1.log").exists());
}

#[test]
fn a_drivers_argv_names_the_journal_entry_and_claim() {
    let OneClaim { db, claim, .. } = one_claim("{}");
    let (entry, seq) = claim.parts();
    let argv = drive_args(&db, &spawn_of(claim));
    let mut journal = OsString::from("--db=");
    journal.push(&db);
    let expected: Vec<OsString> = [
        "dispatch".into(),
        "drive".into(),
        format!("--entry={entry}").into(),
        format!("--claim={seq}").into(),
        journal,
    ]
    .into();
    assert_eq!(argv, expected);
    let words = argv.iter().map(|word| word.to_str().unwrap());
    let parsed = Cli::try_parse_from(std::iter::once("brokkr").chain(words)).unwrap();
    assert!(matches!(
        parsed.command,
        Cmd::Grouped(GroupedCmd::Dispatch(DispatchArgs {
            command: Some(DispatchCmd::Drive(DriveArgs { entry: e, claim: c, .. })),
            ..
        })) if e == entry.0 && c == seq
    ));
}

#[test]
fn a_driver_of_a_claim_with_no_spawn_row_is_refused_by_the_handler() {
    let OneClaim { dir, db, claim, .. } = one_claim("{}");
    let (entry, seq) = claim.parts();
    let (entry, seq) = (entry.to_string(), seq.to_string());
    let argv = [
        "drive",
        "--db",
        db.to_str().unwrap(),
        "--entry",
        &entry,
        "--claim",
        &seq,
    ];
    let refused = dispatch(dir.path(), parsed(&argv), &mut Vec::new()).unwrap_err();
    assert!(
        matches!(
            refused.downcast_ref::<dispatch::DriverError>(),
            Some(dispatch::DriverError::Unspawned { .. })
        ),
        "{refused:?}"
    );
}

/// Set in the re-executed test binary that plays a driver's `lead`.
const LEAD: &str = "BROKKR_TEST_DISPATCH_LEAD";

/// A driver's `lead`, played between this binary's start and its normal
/// exit rather than between a fork and an exec, where its coverage would
/// be lost: it writes down the session it then reads, and its pid.
#[test]
#[ignore = "played by a_driver_leads_a_session_of_its_own in a child of its own"]
fn lead_role() {
    let Some(written) = std::env::var_os(LEAD) else {
        return;
    };
    lead().expect("a process that leads no group leaves its session");
    let session = rustix::process::getsid(None)
        .unwrap()
        .as_raw_nonzero()
        .get();
    std::fs::write(written, format!("{session} {}", std::process::id())).unwrap();
}

/// Ruling 3: `lead` leaves the dispatcher's session, so a driver leads one
/// of its own. Played under a shell that waits for it, so this process is
/// never its parent.
#[test]
fn a_driver_leads_a_session_of_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let written = dir.path().join("session");
    let role = "\"$0\" --exact verbs::dispatch::tests::lead_role --ignored; exit $?";
    let mut shell = Command::new("sh");
    shell
        .arg("-c")
        .arg(role)
        .arg(std::env::current_exe().unwrap());
    shell.env(LEAD, &written).stdin(Stdio::null());
    let quiet = shell.stdout(Stdio::null()).stderr(Stdio::null());
    assert!(quiet.status().unwrap().success());
    let said = std::fs::read_to_string(&written).unwrap();
    let (session, pid) = said.split_once(' ').unwrap();
    assert_eq!(session, pid, "the driver leads its own session");
}
