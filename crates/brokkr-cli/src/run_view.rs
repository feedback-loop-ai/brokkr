//! One run, one view (#508): on a terminal, `brokkr run`, `resume` and
//! `watch` open the console at their run's level — the session `brokkr
//! tui --run <id>` opens, through the same [`console`] — and off one they
//! print exactly what they always have.
//!
//! The view reads the journal read-only, as `brokkr tui` does. Beside a
//! drive it runs on a thread of its own while the engine drives on this
//! one: `q` closes it and leaves the run driving with its plain output,
//! Ctrl+C is handed to the engine's stop path as SIGINT, and a run that
//! ends holds its final frame until a key before the summary prints.

use std::io::{IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use brokkr_core::fold::fold;
use brokkr_runtime::realms::Hearth;
use brokkr_runtime::Engine;

use crate::tui::{self, Closed, Watched};
use crate::{drive_to_end, open_journal, render, tui_source, Access, Exit};

/// Whether a verb opens the run view: on a terminal, its stdin and its
/// stdout both, unless `--no-view` asked for today's output there. The
/// pipeline's lanes and other sessions run `brokkr run` with neither, and
/// must not notice the view exists.
pub(crate) fn opens(stdin: bool, stdout: bool, no_view: bool) -> bool {
    stdin && stdout && !no_view
}

/// What a run view reaches beyond this process's own logic, as function
/// pointers whose production values are [`PRODUCTION`]'s: the house's
/// closure seam, as `tui::TerminalOps` is, so a test drives a run beside
/// a view without a terminal.
pub(crate) struct Viewer {
    /// Whether stdin and stdout are each a terminal.
    pub terminal: fn() -> (bool, bool),
    /// The console over the hearths, opened on a run and watching it.
    pub console: fn(Vec<Hearth>, Option<String>, usize, Watched) -> Result<Closed>,
    /// Ctrl+C, which raw mode delivers as a key, handed to the engine's
    /// own stop path.
    pub interrupt: fn() -> std::io::Result<()>,
}

/// The viewer every verb opens in production.
pub(crate) const PRODUCTION: Viewer = Viewer {
    terminal,
    console,
    interrupt,
};

fn terminal() -> (bool, bool) {
    (
        std::io::stdin().is_terminal(),
        std::io::stdout().is_terminal(),
    )
}

/// The console over `hearths`, opened on `run` and watching what
/// `watched` names: `brokkr tui`'s session, and every run view's. The
/// environment facts are read once here and everything else is injected;
/// the refusals live inside `tui::start`, and the source opens a store
/// only when it is called, which is after that gate.
pub(crate) fn console(
    hearths: Vec<Hearth>,
    run: Option<String>,
    tab: usize,
    watched: Watched,
) -> Result<Closed> {
    let mut heads: Vec<Option<(u64, String)>> = vec![None; hearths.len()];
    let mut seen = None;
    let db_is_file = hearths.iter().any(|hearth| hearth.journal.is_file());
    // A world with one hearth names no tabs, and the console draws none.
    let tabs: Vec<String> = match hearths.len() {
        0 | 1 => Vec::new(),
        _ => hearths.iter().map(Hearth::label).collect(),
    };
    let mut source = tui_source(&hearths, &mut heads, &mut seen);
    tui::start(tui::Session {
        db_is_file,
        run,
        tabs,
        tab,
        ops: tui::production_ops(),
        is_tty: std::io::stdout().is_terminal(),
        // Animation is enabled exactly when colour is, through the same
        // pure rule `brokkr runs` uses: NO_COLOR, TERM=dumb and a
        // non-tty all yield a still graph. No new flag, no new env var.
        animate: render::Style::detect().color,
        backend: ratatui::backend::CrosstermBackend::new(std::io::stdout()),
        restore: std::io::stdout(),
        source: &mut source,
        max_iterations: usize::MAX,
        watched,
    })
}

/// SIGINT to this process: what a terminal's Ctrl+C sends outside raw
/// mode. The engine's stop handler ends every live attempt and exits 130;
/// before its first attempt the signal's default ends the process.
fn interrupt() -> std::io::Result<()> {
    use rustix::process::{getpid, kill_process, Signal};
    kill_process(getpid(), Signal::INT).map_err(std::io::Error::from)
}

/// The one hearth a run view reads: the journal its run is written to.
fn sole(journal: &Path) -> Vec<Hearth> {
    vec![Hearth {
        realms: Vec::new(),
        journal: journal.to_path_buf(),
    }]
}

/// Drive `engine`, which writes `journal`, to its ending: beside the run
/// view where [`opens`] rules a terminal, and exactly as it always has
/// where it does not.
pub(crate) fn drive(
    engine: &mut Engine,
    journal: &Path,
    no_view: bool,
    viewer: &Viewer,
) -> Result<ExitCode> {
    let (stdin, stdout) = (viewer.terminal)();
    if !opens(stdin, stdout, no_view) {
        return drive_to_end(engine, Engine::drive);
    }
    let run = engine.run_id.clone();
    drive_to_end(engine, |engine| {
        beside(
            || engine.drive(),
            |ended| {
                let watched = Watched::Driven(ended);
                let closed = (viewer.console)(sole(journal), Some(run.clone()), 0, watched);
                said(closed, &run, viewer.interrupt, &mut std::io::stderr());
            },
        )
    })
}

/// The drive's end, told to its view when the drive returns and when it
/// unwinds alike: a drive that panicked has ended too, and a view left
/// waiting on it would hold the scope's join, and the process, open.
struct Ends<'a>(&'a AtomicBool);

impl Drop for Ends<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// `drive` on this thread and `view` on its own, the view told through
/// the flag once the drive has returned or unwound. What the drive
/// returned comes back once the view has closed, so a view holding the
/// final frame holds the summary until its key.
fn beside<T>(drive: impl FnOnce() -> T, view: impl FnOnce(Arc<AtomicBool>) + Send) -> T {
    let ended = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ended);
    std::thread::scope(|scope| {
        let viewing = scope.spawn(move || view(flag));
        let end = {
            let _ends = Ends(&ended);
            drive()
        };
        // A view that panicked has restored the terminal and printed its
        // message through the console's hook; the run's ending still
        // prints, as it would have without the view.
        let _ = viewing.join();
        end
    })
}

/// What a run view's close says, and does, on `out`: `q` leaves the run
/// driving here with its plain output and says how to open the view
/// again; Ctrl+C is handed to the engine's stop path and is never a
/// silent detach; a view that could not open, or failed, says why.
fn said(
    closed: Result<Closed>,
    run: &str,
    interrupt: fn() -> std::io::Result<()>,
    out: &mut dyn Write,
) {
    let line = match closed {
        Ok(Closed::Ended) => return,
        Ok(Closed::Quit) => format!(
            "the view closed; run {run} keeps driving here. Open it again with: \
             brokkr tui --run {run}"
        ),
        Ok(Closed::Interrupted) => match interrupt() {
            Ok(()) => return,
            Err(error) => format!(
                "Ctrl+C could not stop run {run} ({error}); it keeps driving here. \
                 Stop it with: brokkr operator stop --run {run} --reason <why>"
            ),
        },
        Err(error) => format!("the view did not stay open: {error}; run {run} keeps driving here"),
    };
    // Stderr closed under a detached run has no reader to tell.
    let _ = writeln!(out, "{line}");
}

/// `brokkr watch` on a terminal: the run view, watching the run's own
/// journal, which it never writes. Closed, it exits as watch's frames
/// would have ended: with the run's own code once it has stopped, and
/// running while it runs or does not fold.
pub(crate) fn watch(journal: &Path, run: &str, viewer: &Viewer) -> Result<ExitCode> {
    let watched = Watched::Journal(run.to_string());
    (viewer.console)(sole(journal), Some(run.to_string()), 0, watched)?;
    let events = open_journal(journal, Access::Read)?.load(run)?;
    let status = fold(&events).ok().map(|state| state.status);
    Ok(Exit::of_settled(status.as_ref())
        .unwrap_or(Exit::Running)
        .into())
}

#[cfg(test)]
mod tests;
