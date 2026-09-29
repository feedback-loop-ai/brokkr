//! The terminal driver: the five crossterm calls as injected function
//! pointers, the restoring guard and panic hook, the startup refusals,
//! and the bounded shell that draws, polls and applies.

use super::*;

// ---------------------------------------------------------- the terminal

/// The five process-global crossterm calls, held as **function-pointer
/// fields**. Their production values are crossterm's own function items,
/// never our wrappers and never closures: either would be a counted
/// function only production could execute.
pub(crate) struct TerminalOps {
    pub enter_raw: fn() -> std::io::Result<()>,
    pub leave_raw: fn() -> std::io::Result<()>,
    pub poll: fn(Duration) -> std::io::Result<bool>,
    pub read: fn() -> std::io::Result<Event>,
    pub size: fn() -> std::io::Result<(u16, u16)>,
}

pub(crate) fn production_ops() -> TerminalOps {
    TerminalOps {
        enter_raw: enable_raw_mode,
        leave_raw: disable_raw_mode,
        poll: ratatui::crossterm::event::poll,
        read: ratatui::crossterm::event::read,
        size: ratatui::crossterm::terminal::size,
    }
}

/// Restoration is RAII: a trailing `disable_raw_mode()` is skipped by
/// every `?` between setup and the end of the loop, which is precisely
/// the bug "prove restoration on the error path" exists to catch. A
/// guard makes `?` safe by construction.
struct Guard<W: Write> {
    out: W,
    leave_raw: fn() -> std::io::Result<()>,
}

impl<W: Write> Drop for Guard<W> {
    fn drop(&mut self) {
        // Errors are swallowed deliberately: a failing restore must not
        // replace the error the operator actually needs to read, and a
        // panicking `Drop` during unwinding aborts the process.
        let _ = execute!(self.out, LeaveAlternateScreen, Show);
        let _ = (self.leave_raw)();
    }
}

/// The panic hook's restore, as a bare `fn` so the hook holds no closure
/// over a terminal it does not own.
pub(super) fn restore_stdout() {
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen, Show);
    let _ = disable_raw_mode();
}

/// Restore **first**, chain to the previous hook **second**: a panic
/// message printed into a raw-mode alternate screen is unreadable, and
/// swallowing the message entirely is worse than an ugly one.
pub(crate) fn install_panic_hook(restore: fn()) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        previous(info);
    }));
}

/// The three startup refusals, as one pure rule. Both `brokkr inspect`
/// and `brokkr watch` are named: an operator who cannot have the console
/// must be told what they can have instead.
pub(crate) fn refuse(is_tty: bool, size: (u16, u16), db_is_file: bool) -> Option<String> {
    let instead = "use `brokkr inspect --run <id>` or `brokkr watch --run <id>` instead";
    if !db_is_file {
        return Some(format!(
            "no workspace database to read, and a read never creates one; {instead}"
        ));
    }
    if !is_tty {
        return Some(format!(
            "`brokkr tui` needs a terminal and stdout is not one; {instead}"
        ));
    }
    if size.0 < MIN_WIDTH || size.1 < MIN_HEIGHT {
        return Some(format!(
            "this terminal is {}×{}, below the {MIN_WIDTH}×{MIN_HEIGHT} `brokkr tui` needs; {instead}",
            size.0, size.1
        ));
    }
    None
}

/// How a refreshed frame relates the displayed transcript to the fresh one.
///
/// Invariant: in `drive`, `tui.reading_transcript` implies that
/// `views.transcript` is `Some` and readable. It starts false; it is set
/// true only inside the `is_readable` filter on `views.transcript`; every
/// other write clears it; and `views` changes only to a fresh frame whose
/// recompose keeps the door open only over a readable read. Given that
/// invariant, a missing or unavailable fresh read makes
/// `transcript_invalidates` return true and closes the door before any
/// recompose runs, so the former `None` arm of the door recompose could
/// never execute. The classification below carries the fresh read in the
/// one arm that is allowed to recompose the door.
enum Refresh<'a> {
    /// Neither the displayed frame nor the fresh one holds a transcript.
    Absent,
    /// The fresh read is present and not invalidated, so it is readable.
    Continuing(&'a TranscriptRead),
    /// The fresh read replaced, removed or reordered the displayed one.
    Invalidated,
}

fn classify_refresh<'a>(
    displayed: Option<&TranscriptRead>,
    fresh: Option<&'a TranscriptRead>,
) -> Refresh<'a> {
    match (displayed, fresh) {
        (None, None) => Refresh::Absent,
        (_, Some(read)) if !transcript_invalidates(displayed, fresh) => Refresh::Continuing(read),
        _ => Refresh::Invalidated,
    }
}

/// A frame that arrived, settled against the one it replaces: the
/// transcript and the doors onto it are reconciled with the fresh read
/// before the fresh models become the displayed ones, which this returns.
fn arrive(
    tui: &mut Tui,
    displayed: Option<&TranscriptRead>,
    mut fresh: Views,
    vanishing_key: Option<&str>,
) -> Views {
    // L6: when the fresh fold no longer lists the participant
    // this frame was reading — even when it lists none at all
    // — the stale transcript is cleared in this same frame.
    if let (Some(key), Some(run)) = (vanishing_key, fresh.run.as_ref()) {
        if !run.participants.iter().any(|part| part.key == key) {
            fresh.transcript = None;
        }
    }
    // A refreshed read that replaced, removed or reordered a
    // displayed turn clears the cursor and closes an open
    // overlay BEFORE the new indices are shown; a pure append
    // or a notice-only change leaves navigation alone. Classify
    // once, then recompose the door only in the continuing arm,
    // from that arm's fresh read (see `classify_refresh`).
    match classify_refresh(displayed, fresh.transcript.as_ref()) {
        Refresh::Absent => {}
        Refresh::Invalidated => {
            tui.turn = None;
            tui.reading = None;
            tui.reading_transcript = false;
            tui.read_offset = 0;
        }
        // A notice-only refresh leaves the turns alone but still
        // changes what a door must say: recompose an open
        // transcript door from the fresh shared read so the pane
        // and the door report the same notices.
        Refresh::Continuing(read) => {
            if tui.reading_transcript {
                tui.reading = Some(match selected_turn(tui, &fresh) {
                    Some((index, turn)) => turn_overlay_text(index + 1, turn, read),
                    None => transcript_text(read),
                });
            }
        }
    }
    // A frame that arrived says whatever it has to say — a
    // hearth with no journal yet says so — and a frame with
    // nothing to say clears the last sentence.
    tui.status = fresh.note.clone();
    fresh
}

/// The bounded shell: draw, poll, apply, repeat. Everything impure it
/// touches arrives as a parameter, so the whole loop — its quit arm, its
/// error arm and its transient-busy arms — runs under `TestBackend`.
pub(super) fn drive<B: Backend>(
    terminal: &mut Terminal<B>,
    ops: &TerminalOps,
    source: &mut dyn FnMut(Ask) -> Result<Refreshed>,
    tui: &mut Tui,
    max_iterations: usize,
) -> Result<ExitCode>
where
    // A backend's own error reaches the operator through `anyhow`.
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let mut views = Views::empty();
    let mut failures = 0usize;
    for _ in 0..max_iterations {
        let subject = subject_of(tui, &views);
        let vanishing_key = subject.as_ref().map(|subject| subject.key.clone());
        let ask = Ask {
            run: tui.run.as_deref(),
            subject,
            force: std::mem::take(&mut tui.force),
            fleet: tui.ticks.is_multiple_of(RUNS_REFRESH_TICKS),
            // Only the ACTIVE hearth is ever asked about, so an inactive
            // tab's journal is neither polled nor opened.
            tab: tui.tab,
        };
        match source(ask) {
            Ok(Some(fresh)) => {
                views = arrive(
                    tui,
                    views.transcript.as_ref(),
                    fresh,
                    vanishing_key.as_deref(),
                );
                failures = 0;
            }
            Ok(None) => failures = 0,
            Err(error) => {
                // A transient store error (SQLITE_BUSY while a `brokkr
                // run` holds the write lock) is a frame that says so,
                // with keys still live. A persistent one gives up.
                failures += 1;
                tui.status = Some(format!("the journal is not readable right now: {error}"));
                anyhow::ensure!(
                    failures < crate::WATCH_TRANSIENT_FRAMES,
                    "giving up after {failures} unreadable polls: {error}"
                );
            }
        }
        settle(tui, &views);
        terminal.draw(|frame| draw(frame, tui, &views))?;
        if (ops.poll)(TICK)? {
            if let Some(key) = from_crossterm((ops.read)()?) {
                if apply(tui, &views, key) == Flow::Quit {
                    return Ok(crate::Exit::Completed.into());
                }
            }
        }
        tui.ticks += 1;
    }
    Ok(crate::Exit::Completed.into())
}

/// Enter the terminal, run the console, leave the terminal — with every
/// environment fact and every terminal call arriving as a parameter, so
/// the whole of this function executes in tests as well as in
/// production. Nothing here exits the process outright — that would run
/// past the guard's `Drop` and leave a terminal in raw mode — so the TUI
/// returns an `ExitCode` like every other arm.
#[expect(clippy::too_many_arguments, reason = "baseline 2026-09, #288")]
pub(crate) fn start<B: Backend, R: Write>(
    db_is_file: bool,
    run: Option<String>,
    // The world's hearths as realm names (decision 0026 ruling 2).
    // Empty or one-long draws no tab bar. `tab` is the one to open on:
    // the hearth a named `--run` was found in.
    tabs: Vec<String>,
    tab: usize,
    ops: TerminalOps,
    is_tty: bool,
    // Animation is enabled exactly when colour is: the same line kind
    // as `is_tty`, read once at the call site and injected here, so a
    // test sets it directly and touches no environment.
    animate: bool,
    backend: B,
    restore: R,
    source: &mut dyn FnMut(Ask) -> Result<Refreshed>,
    max_iterations: usize,
) -> Result<ExitCode>
where
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let size = (ops.size)().unwrap_or((0, 0));
    if let Some(message) = refuse(is_tty, size, db_is_file) {
        anyhow::bail!("{message}");
    }
    (ops.enter_raw)()?;
    let mut guard = Guard {
        out: restore,
        leave_raw: ops.leave_raw,
    };
    execute!(guard.out, EnterAlternateScreen, Hide)?;
    install_panic_hook(restore_stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut state = Tui::over(run, tabs, tab);
    state.animate = animate;
    let code = drive(&mut terminal, &ops, source, &mut state, max_iterations);
    // Uninstalled on the normal path: a panic later in this process must
    // not restore a terminal this function has already left.
    let _ = std::panic::take_hook();
    code
}
