//! One run, one view (#508), at the shell: the session `brokkr run`,
//! `resume` and `watch` open is the one `brokkr tui --run` opens, and
//! differs only in what it watches — when its run has ended, and what
//! Ctrl+C means.

use super::snapshot_tests::settings;
use super::tests::{lines_of, script, test_ops, views, NOW, SCRIPT, TERMINAL};
use super::*;
use ratatui::backend::TestBackend;

/// The console's session over `source` as `brokkr tui` opens it on the
/// fleet: no run named, one hearth, its terminal calls `ops`, its
/// restoring writer `restore`, and its loop bounded at `max`.
pub(super) fn console<'a, R: Write>(
    db_is_file: bool,
    ops: TerminalOps,
    restore: R,
    source: &'a mut dyn FnMut(Ask) -> Result<Refreshed>,
    max: usize,
) -> Session<'a, TestBackend, R> {
    Session {
        db_is_file,
        run: None,
        tabs: Vec::new(),
        tab: 0,
        ops,
        is_tty: true,
        animate: false,
        backend: TestBackend::new(100, 30),
        restore,
        source,
        max_iterations: max,
        watched: Watched::Console,
    }
}

/// A session opened on `run` at its level, watching `watched`, driven
/// over `source` for `polls` polls of the scripted keys: how it closed,
/// the backend holding the frame it last drew, and the console's state.
fn opened(
    run: &str,
    watched: Watched,
    source: &mut dyn FnMut(Ask) -> Result<Refreshed>,
    polls: usize,
) -> (Closed, TestBackend, Tui) {
    let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
    let mut tui = Tui::over(Some(run.to_string()), Vec::new(), 0);
    tui.watched = watched;
    let closed = drive(&mut terminal, &test_ops(), source, &mut tui, polls).unwrap();
    (closed, terminal.backend().clone(), tui)
}

/// A flag the drive beside the view raises once it has returned.
fn driven(ended: bool) -> Watched {
    Watched::Driven(std::sync::Arc::new(AtomicBool::new(ended)))
}

/// The events the scripted terminal delivers next, as they are.
fn events(events: Vec<Event>) {
    *SCRIPT.lock().unwrap() = events;
}

/// Ctrl+C as raw mode delivers it: a key, never a signal.
fn ctrl_c() {
    let pressed = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    events(vec![Event::Key(pressed)]);
}

/// The view `brokkr run` opens draws, frame for frame, what `brokkr tui
/// --run` draws for the same run: over the fixture's models, pinned, and
/// over a journal on disk read through the shell's own source.
#[test]
fn the_run_view_is_the_frame_tui_run_draws() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    script(&[]);
    let mut source = |_: Ask| Ok(Some(views()));
    let (_, tui_run, _) = opened("run-7", Watched::Console, &mut source, 2);
    let (_, run_view, _) = opened("run-7", driven(false), &mut source, 2);
    assert_eq!(run_view.buffer(), tui_run.buffer());
    let frame = run_view.to_string();
    settings().bind(|| insta::assert_snapshot!("run_view_160x48", frame));

    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    crate::tests::running_store(&db, "run-on-disk");
    let mut head = None;
    let mut source =
        |ask: Ask| crate::tui_views(&db, true, ask, &mut head, &mut None, || NOW.to_string());
    let (_, tui_run, _) = opened("run-on-disk", Watched::Console, &mut source, 2);
    let (_, run_view, _) = opened("run-on-disk", driven(false), &mut source, 2);
    let drawn = tui_run.to_string();
    assert!(drawn.contains("run run-on-disk"), "{drawn}");
    assert_eq!(run_view.buffer(), tui_run.buffer());
}

/// Ctrl+C stops a run this process drives, as SIGINT would; everywhere
/// else, `brokkr tui` and `brokkr watch` alike, it quits. `q` closes the
/// view over a driven run and leaves the run to its drive.
#[test]
fn ctrl_c_stops_a_driven_run_and_quits_everywhere_else() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let mut source = |_: Ask| Ok(Some(views()));
    let watching = [
        (driven(false), Closed::Interrupted),
        (Watched::Console, Closed::Quit),
        (Watched::Journal("run-7".to_string()), Closed::Quit),
    ];
    for (watched, wanted) in watching {
        ctrl_c();
        let (closed, _, _) = opened("run-7", watched.clone(), &mut source, 4);
        assert_eq!(closed, wanted, "{watched:?}");
    }
    script(&[Key::Char('q')]);
    let (closed, _, _) = opened("run-7", driven(false), &mut source, 4);
    assert_eq!(closed, Closed::Quit);
}

/// A run that ends holds its final frame, read afresh, until any key:
/// the footer says so, and the key closes the view rather than moving
/// it. Until then every key is the run level's own.
#[test]
fn a_run_that_ended_holds_its_final_frame_until_any_key() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    // The drive returns while the first frame is read.
    let flag = std::sync::Arc::new(AtomicBool::new(false));
    let mut forced: Vec<bool> = Vec::new();
    let mut asking = |ask: Ask| {
        forced.push(ask.force);
        flag.store(true, Ordering::SeqCst);
        Ok(Some(views()))
    };
    script(&[]);
    let watched = Watched::Driven(std::sync::Arc::clone(&flag));
    let (closed, frame, tui) = opened("run-7", watched, &mut asking, 3);
    assert_eq!((closed, tui.ended), (Closed::Quit, true));
    assert_eq!(
        forced,
        [true, true, false],
        "the final frame is read afresh"
    );
    let footer = lines_of(frame.buffer())[47].trim_end().to_string();
    assert_eq!(footer, "the run has ended · any key closes the view");
    let mut source = |_: Ask| Ok(Some(views()));
    script(&[Key::Char('?')]);
    let (closed, _, tui) = opened("run-7", driven(true), &mut source, 4);
    assert_eq!((closed, tui.help), (Closed::Ended, false));
    // A key the console binds nothing to closes it too; a resize and a
    // release, which are no key pressed, do not.
    let home = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
    let mut release = home;
    release.kind = KeyEventKind::Release;
    events(vec![
        Event::Resize(160, 48),
        Event::Key(release),
        Event::Key(home),
    ]);
    let (closed, _, _) = opened("run-7", driven(true), &mut source, 4);
    assert_eq!(closed, Closed::Ended);
    assert_eq!(SCRIPT.lock().unwrap().len(), 0, "the press closed it");
    script(&[Key::Char('?')]);
    let (closed, _, tui) = opened("run-7", driven(false), &mut source, 2);
    assert_eq!((closed, tui.help, tui.ended), (Closed::Quit, true, false));
}

/// `brokkr watch`'s view ends when the run's own fleet row folds to a
/// status other than running, and only then: another run's ending, a
/// running row, and a row whose journal does not fold end nothing.
#[test]
fn a_watched_journal_ends_when_its_run_stops_running() {
    let mut views = views();
    let watched = Watched::Journal("run-7".to_string());
    let stand = |views: &mut Views, run: &str, standing: Standing| {
        let row = views.runs.runs.iter_mut().find(|row| row.run_id == run);
        row.unwrap().verdict.standing = standing;
    };
    stand(&mut views, "run-old", Standing::Shipped);
    let every = [
        Standing::Running,
        Standing::Quarantined,
        Standing::Parked,
        Standing::Shipped,
        Standing::Stopped,
        Standing::OperatorStopped,
    ];
    let ended: Vec<bool> = every
        .into_iter()
        .map(|standing| {
            stand(&mut views, "run-7", standing);
            has_ended(&watched, &views)
        })
        .collect();
    assert_eq!(ended, [false, false, true, true, true, true]);
    assert!(!has_ended(&Watched::Console, &views));
}
