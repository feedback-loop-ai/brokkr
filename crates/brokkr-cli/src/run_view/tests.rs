//! One run, one view (#508), at its seams: whether a verb opens the
//! view, a real drive beside a view however it closes, what each close
//! says and does, Ctrl+C raised as SIGINT, and `brokkr watch`'s view.
//! No pseudo-terminal crate is in the lockfile, so the terminal is the
//! viewer's injected answer, and the shell's own keys are proved under
//! `TestBackend` in `tui::view_tests`.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicUsize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use brokkr_runtime::Bundle;

use super::*;
use crate::cli_args::ResumeArgs;
use crate::tests::{at, bundled, running_store, stage_hands_free_fast, stopped_mid_flight_copy};
use crate::tests::{watching, workspace};
use crate::verbs::{delivery, readouts};

/// The view opens on a terminal, stdin and stdout both, and nowhere
/// else; `--no-view` keeps today's output even there.
#[test]
fn the_view_opens_on_a_terminal_both_ways_and_never_under_no_view() {
    let every = [false, true];
    let opened: Vec<(bool, bool, bool)> = every
        .iter()
        .flat_map(|stdin| every.iter().map(move |stdout| (*stdin, *stdout)))
        .flat_map(|(stdin, stdout)| every.map(|no_view| (stdin, stdout, no_view)))
        .filter(|(stdin, stdout, no_view)| opens(*stdin, *stdout, *no_view))
        .collect();
    assert_eq!(opened, [(true, true, false)]);
}

fn refused() -> std::io::Result<()> {
    Err(std::io::Error::other("the signal was refused"))
}

static RAISED: AtomicUsize = AtomicUsize::new(0);

fn raised() -> std::io::Result<()> {
    RAISED.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

/// What each close of a view over a driven run says on stderr: nothing
/// once the run ended, how to open the view again after `q`, nothing
/// once Ctrl+C has been handed to the engine and why not when it could
/// not be, and why a view that did not stay open closed.
#[test]
fn each_close_says_what_became_of_the_run() {
    let said_on = |closed: Result<Closed>, interrupt: fn() -> std::io::Result<()>| {
        let mut out = Vec::new();
        said(closed, "run-1", interrupt, &mut out);
        String::from_utf8(out).unwrap()
    };
    assert_eq!(said_on(Ok(Closed::Ended), refused), "");
    assert_eq!(
        said_on(Ok(Closed::Quit), refused),
        "the view closed; run run-1 keeps driving here. Open it again with: \
         brokkr tui --run run-1\n"
    );
    assert_eq!(said_on(Ok(Closed::Interrupted), raised), "");
    assert_eq!(RAISED.load(Ordering::SeqCst), 1, "Ctrl+C was handed on");
    assert_eq!(
        said_on(Ok(Closed::Interrupted), refused),
        "Ctrl+C could not stop run run-1 (the signal was refused); it keeps driving \
         here. Stop it with: brokkr operator stop --run run-1 --reason <why>\n"
    );
    let small = Err(anyhow::anyhow!("this terminal is 50×10"));
    assert_eq!(
        said_on(small, refused),
        "the view did not stay open: this terminal is 50×10; run run-1 keeps driving here\n"
    );
}

/// The drive's ending comes back only once the view beside it has
/// closed, and the view is told when the drive has returned: so a view
/// holding its final frame holds the summary until its key. A view that
/// panics does not take the run's ending with it, and a drive that
/// panics has ended for its view too, its panic carried on once the view
/// has closed.
#[test]
fn the_drive_ends_before_its_view_and_the_summary_waits_for_it() {
    let order = Mutex::new(Vec::new());
    let end = beside(
        || {
            order.lock().unwrap().push("drove");
            7
        },
        |ended| {
            let closed = match awaited(&ended) {
                true => "view closed on the end",
                false => "view gave up waiting",
            };
            order.lock().unwrap().push(closed);
        },
    );
    order.lock().unwrap().push("summary");
    assert_eq!(end, 7);
    let wanted = ["drove", "view closed on the end", "summary"];
    assert_eq!(*order.lock().unwrap(), wanted);
    let end = beside(|| 8, |_| panic!("a view that panics"));
    assert_eq!(end, 8);
    let seen = AtomicBool::new(false);
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        beside(
            || -> u8 { panic!("a drive that panics") },
            |ended| seen.store(awaited(&ended), Ordering::SeqCst),
        )
    }));
    let panic = unwound.unwrap_err();
    assert_eq!(panic.downcast_ref::<&str>(), Some(&"a drive that panics"));
    assert!(seen.load(Ordering::SeqCst), "the view saw the drive end");
}

const ROLE: &str = "BROKKR_RUN_VIEW_TEST_ROLE";

/// The part this test binary plays when a test re-executes it. Run on
/// its own, it plays none.
#[test]
#[ignore = "played only when a test re-executes this binary"]
fn role() {
    if std::env::var(ROLE).as_deref() == Ok("interrupt") {
        interrupt().unwrap();
    }
}

/// Ctrl+C in the view is the SIGINT a terminal sends outside raw mode,
/// raised at this process, so the engine's stop handler meets it as it
/// would meet the operator's own: unhandled, it ends the process as the
/// signal does, and the shell reads 128 plus SIGINT. Ignored, as under
/// `nohup`, it returns, which is how its raise is measured at all.
#[test]
fn ctrl_c_is_raised_as_the_sigint_a_terminal_sends() {
    let played = |prelude: &str| {
        let script = format!("{prelude}\"$0\" --exact run_view::tests::role --ignored; exit $?");
        let status = Command::new("sh")
            .args(["-c", &script])
            .arg(std::env::current_exe().unwrap())
            .env(ROLE, "interrupt")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        status.code()
    };
    assert_eq!(played(""), Some(130));
    assert_eq!(played("trap '' INT; "), Some(0));
}

/// A view a driven run opened: the journal and the run it was opened on,
/// its hearth, and whether it saw the drive end.
type Opened = (PathBuf, Option<String>, usize, bool);

/// The views driven runs opened in these tests.
static OPENED: Mutex<Vec<Opened>> = Mutex::new(Vec::new());

fn on_a_terminal() -> (bool, bool) {
    (true, true)
}

/// A view opened on `run` over `hearths`, recorded with the flag a drive
/// beside it raises.
fn record(hearths: &[Hearth], run: Option<String>, tab: usize, watched: &Watched) {
    let ended = match watched {
        Watched::Driven(ended) => ended.load(Ordering::SeqCst),
        Watched::Console | Watched::Journal(_) => false,
    };
    let opened = (hearths[0].journal.clone(), run, tab, ended);
    OPENED.lock().unwrap().push(opened);
}

/// Whether `ended` is raised within a bound every drive in these tests
/// keeps: how a view holding its final frame waits for the drive.
fn awaited(ended: &AtomicBool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !ended.load(Ordering::SeqCst) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    ended.load(Ordering::SeqCst)
}

/// The final frame held: the view closes on the operator's key once the
/// drive beside it has returned.
fn holding(
    hearths: Vec<Hearth>,
    run: Option<String>,
    tab: usize,
    watched: Watched,
) -> Result<Closed> {
    if let Watched::Driven(ended) = &watched {
        awaited(ended);
    }
    record(&hearths, run, tab, &watched);
    Ok(Closed::Ended)
}

/// `q` at once.
fn quitting(hearths: Vec<Hearth>, run: Option<String>, tab: usize, _: Watched) -> Result<Closed> {
    record(&hearths, run, tab, &Watched::Console);
    Ok(Closed::Quit)
}

/// Ctrl+C at once.
fn interrupting(
    hearths: Vec<Hearth>,
    run: Option<String>,
    tab: usize,
    _: Watched,
) -> Result<Closed> {
    record(&hearths, run, tab, &Watched::Console);
    Ok(Closed::Interrupted)
}

static INTERRUPTS: AtomicUsize = AtomicUsize::new(0);

/// The engine's stop path, standing in: the run keeps driving, so the
/// test reads its ending.
fn counted() -> std::io::Result<()> {
    INTERRUPTS.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

/// Runs stopped mid-flight under the hands-free fast bundle, each with
/// an operator stop its resume concludes before any seat is spawned: the
/// bundle staged in `dir`, and the journal holding them.
fn stopped_runs(dir: &Path, runs: &[&str]) -> (PathBuf, PathBuf) {
    let staged = stage_hands_free_fast(dir, "viewed-hands-free", false);
    let agents = workspace().join("agents");
    let compiled = Bundle::compile_with(&staged, &agents, &workspace().join("adapters")).unwrap();
    let charters = brokkr_runtime::bundle::charters_intact(&compiled).unwrap();
    let db = dir.join("forge.db");
    for run in runs {
        stopped_mid_flight_copy(&db, run, &compiled.manifest, Some(&charters));
    }
    (staged, db)
}

/// `brokkr resume --run <run>` of a run [`stopped_runs`] staged in `dir`.
fn resumed(dir: &Path, run: &str, no_view: bool, viewer: &Viewer) -> ExitCode {
    let (staged, db) = (dir.join("viewed-hands-free"), dir.join("forge.db"));
    let args = ResumeArgs {
        delivery: bundled(staged),
        run: run.into(),
        journal: at(&db),
        repo: Some(dir.to_path_buf()),
        no_view,
    };
    delivery::resume(&workspace(), args, viewer).unwrap()
}

/// `brokkr resume` on a terminal opens the view on its run's journal and
/// drives beside it to the run's own exit, whichever way the view
/// closes: held on its final frame until the drive has returned, closed
/// by `q` with the run left driving, or by Ctrl+C handed to the stop
/// path. `--no-view` there opens none.
#[test]
fn resume_on_a_terminal_drives_beside_its_view_whichever_way_it_closes() {
    let dir = tempfile::tempdir().unwrap();
    let runs = ["held", "quit", "interrupted", "plain"];
    let (_, db) = stopped_runs(dir.path(), &runs);
    let viewing = |console| Viewer {
        terminal: on_a_terminal,
        console,
        interrupt: counted,
    };
    let stopped = ExitCode::from(3);
    assert_eq!(
        resumed(dir.path(), "held", false, &viewing(holding)),
        stopped
    );
    assert_eq!(
        resumed(dir.path(), "quit", false, &viewing(quitting)),
        stopped
    );
    let viewer = viewing(interrupting);
    assert_eq!(resumed(dir.path(), "interrupted", false, &viewer), stopped);
    assert_eq!(
        resumed(dir.path(), "plain", true, &viewing(holding)),
        stopped
    );
    let opened: Vec<_> = OPENED.lock().unwrap().clone();
    let opened: Vec<_> = opened.into_iter().filter(|view| view.0 == db).collect();
    let on = |run: &str, ended| (db.clone(), Some(run.to_string()), 0, ended);
    assert_eq!(
        opened,
        [
            on("held", true),
            on("quit", false),
            on("interrupted", false)
        ]
    );
    assert_eq!(INTERRUPTS.load(Ordering::SeqCst), 1);
}

/// The runs `brokkr watch`'s view was opened on, and what it watched.
static WATCHED: Mutex<Vec<(Option<String>, String)>> = Mutex::new(Vec::new());

fn watched_view(_: Vec<Hearth>, run: Option<String>, _: usize, watched: Watched) -> Result<Closed> {
    let watching = match watched {
        Watched::Journal(run) => run,
        Watched::Console | Watched::Driven(_) => String::new(),
    };
    WATCHED.lock().unwrap().push((run, watching));
    Ok(Closed::Quit)
}

/// `brokkr watch` on a terminal opens the view watching its run's own
/// journal, and exits as its frames would have: with a stopped run's
/// own code, and as running while the run runs. `--once` and
/// `--no-view` keep the frames there.
#[test]
fn watch_on_a_terminal_is_the_run_view_and_exits_as_its_frames_would() {
    let dir = tempfile::tempdir().unwrap();
    let (_, db) = stopped_runs(dir.path(), &["ended"]);
    resumed(dir.path(), "ended", true, &PRODUCTION);
    running_store(&db, "running");
    let viewer = Viewer {
        terminal: on_a_terminal,
        console: watched_view,
        interrupt: refused,
    };
    let watch = |run: &str, once: bool, no_view: bool| {
        let mut args = watching(run, &db, once, 100);
        args.no_view = no_view;
        readouts::watch(&workspace(), args, Some(1), &viewer).unwrap()
    };
    assert_eq!(watch("ended", false, false), ExitCode::from(3));
    assert_eq!(watch("running", false, false), ExitCode::from(1));
    assert_eq!(watch("running", true, false), ExitCode::from(1));
    assert_eq!(watch("running", false, true), ExitCode::from(1));
    let named = |run: &str| (Some(run.to_string()), run.to_string());
    assert_eq!(*WATCHED.lock().unwrap(), [named("ended"), named("running")]);
}
