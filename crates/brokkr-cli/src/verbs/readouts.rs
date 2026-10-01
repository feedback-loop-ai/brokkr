//! The verbs that read journals and the world and write nothing: `costs`,
//! `ledger`, `ui`, `tui`, `inspect`, `seats`, `watch`, `replay`, `runs`,
//! `realms` and `compare` (decision 0014: every readout is read-only).

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use brokkr_core::fold::fold;
use brokkr_runtime::realms::Hearth;
use brokkr_store::Store;
use serde_json::json;

use crate::cli_args::{CompareArgs, CostsArgs, InspectArgs, LedgerArgs, RealmsArgs};
use crate::cli_args::{ReplayArgs, RunsArgs, SeatsArgs, TuiArgs, UiArgs, WatchArgs};
use crate::{compare, fleet, ledger, realms, render, run_view, selector};
use crate::{hearths_of, journal_of, now_rfc3339, open_journal, resolve_in_hearths};
use crate::{summarize, ui_journal, watch_loop, Access, Exit, Invocation};

/// `brokkr costs`: per-seat cost and session accounting.
pub(crate) fn costs(workspace: &Path, CostsArgs { run, journal }: CostsArgs) -> Result<ExitCode> {
    let store = open_journal(&journal.journal(workspace)?, Access::Read)?;
    let report = compare::costs(&store, &run)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(Exit::Completed.into())
}

/// `brokkr ledger`: the shipper's delivery ledger.
pub(crate) fn ledger(
    workspace: &Path,
    LedgerArgs { run, journal, repo }: LedgerArgs,
) -> Result<ExitCode> {
    let db = journal.journal(workspace)?;
    anyhow::ensure!(
        db.is_file(),
        "journal does not exist: {}; ledger reads never create one",
        db.display()
    );
    let store = Store::open_read_only(&db)?;
    let run = selector::resolve_run(&store, &run)?;
    let events = store.load(&run)?;
    match repo {
        Some(repo) => println!("{}", ledger::write(&run, &events, &repo)?.display()),
        None => print!("{}", ledger::render(&run, &events, workspace)?),
    }
    Ok(Exit::Completed.into())
}

/// `brokkr ui`: serve the read-only surface through `serve_ui`.
pub(crate) fn ui(
    workspace: &Path,
    UiArgs {
        journal,
        port,
        open,
    }: UiArgs,
    serve_ui: impl FnOnce(PathBuf, u16, bool) -> std::io::Result<()>,
) -> Result<ExitCode> {
    serve_ui(ui_journal(workspace, journal)?, port, open)?;
    Ok(Exit::Completed.into())
}

/// `brokkr tui`: the console, drawn by `run_tui` over the world's hearths.
pub(crate) fn tui(
    workspace: &Path,
    TuiArgs { run, realms, db }: TuiArgs,
    run_tui: impl FnOnce(Vec<Hearth>, Option<String>, usize) -> Result<ExitCode>,
) -> Result<ExitCode> {
    let hearths = hearths_of(workspace, realms, db)?;
    // Selectors resolve through decision 0015's one resolver —
    // but resolving needs a store, and `brokkr tui` refuses a
    // missing database *before* anything opens one, because
    // `Store::open` creates a file, a WAL and a meta row. So
    // resolution waits until a file is known to exist and
    // `tui::start` does the refusing. In a many-hearth world it
    // also says which hearth to open on: the one holding the run.
    let (tab, run) = match run {
        Some(run) => {
            let (tab, run) = resolve_in_hearths(&hearths, run)?;
            (tab, Some(run))
        }
        None => (0, None),
    };
    run_tui(hearths, run, tab)
}

/// The view `inspect` derives and `seats` paints a block of: the run the
/// selector names, in the journal the world names, folded once.
fn run_view_of(
    workspace: &Path,
    realms: Option<PathBuf>,
    db: Option<PathBuf>,
    run: &str,
) -> Result<brokkr_view::RunView> {
    let db = journal_of(workspace, realms, db)?;
    let store = open_journal(&db, Access::Read)?;
    let run = selector::resolve_run(&store, run)?;
    let events = store.load(&run)?;
    let state = fold(&events)?;
    Ok(brokkr_view::run_view(&events, Some(&state)))
}

/// `brokkr inspect`: explain a run, whole or scoped to a phase or a seat.
pub(crate) fn inspect(
    workspace: &Path,
    InspectArgs {
        run,
        realms,
        db,
        json,
        phase,
        seat,
    }: InspectArgs,
) -> Result<ExitCode> {
    let view = run_view_of(workspace, realms, db, &run)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&view)?);
        return Ok(Exit::Completed.into());
    }
    // clap's ArgGroup already rules the two mutually exclusive.
    let scope = match (phase, seat) {
        (Some(phase), _) => Some(render::Scope::Phase(phase)),
        (None, Some(seat)) => Some(render::Scope::Seat(seat)),
        (None, None) => None,
    };
    let lens = render::lens_for(&view, scope.as_ref()).map_err(anyhow::Error::msg)?;
    print!(
        "{}",
        render::inspect(&view, lens.as_ref(), true, &render::Style::detect())
    );
    Ok(Exit::Completed.into())
}

/// `brokkr seats`: the seats block of `inspect`'s readout.
pub(crate) fn seats(
    workspace: &Path,
    SeatsArgs {
        run,
        realms,
        db,
        json,
    }: SeatsArgs,
) -> Result<ExitCode> {
    // A thin verb (design DD11): the journal `inspect` would
    // open, the view `inspect` would derive, and either the
    // seats block of `inspect`'s readout or `inspect --json`'s
    // own bytes. Nothing is derived here and no wire object is
    // versioned for the verb.
    let view = run_view_of(workspace, realms, db, &run)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&view)?);
        return Ok(Exit::Completed.into());
    }
    print!("{}", render::seats(&view, &render::Style::detect()));
    Ok(Exit::Completed.into())
}

/// `brokkr watch`: redraw a run live until it reaches a terminal status,
/// or for `watch_iteration_limit` frames when a test bounds it. On a
/// terminal, and not `--once`, it is the run view instead (#508).
pub(crate) fn watch(
    workspace: &Path,
    WatchArgs {
        run,
        realms,
        db,
        once,
        interval_ms,
        no_view,
    }: WatchArgs,
    watch_iteration_limit: Option<usize>,
    viewer: &run_view::Viewer,
) -> Result<ExitCode> {
    let db = journal_of(workspace, realms, db)?;
    // Selectors resolve once, before the loop: a prefix that is
    // unique now stays this frame's run even if another run is
    // started while we watch.
    let run = selector::resolve_run(&open_journal(&db, Access::Read)?, &run)?;
    let (stdin, stdout) = (viewer.terminal)();
    if !once && run_view::opens(stdin, stdout, no_view) {
        return run_view::watch(&db, &run, viewer);
    }
    let style = render::Style::detect();
    let is_tty = std::io::stdout().is_terminal();
    let iterations = if once {
        1
    } else {
        watch_iteration_limit.unwrap_or(usize::MAX)
    };
    watch_loop(
        &db,
        &run,
        interval_ms,
        is_tty,
        &style,
        &mut std::io::stdout(),
        &mut now_rfc3339,
        &mut |ms| std::thread::sleep(std::time::Duration::from_millis(ms)),
        iterations,
    )
}

/// `brokkr replay`: rebuild state from the journal twice and compare.
pub(crate) fn replay(
    workspace: &Path,
    ReplayArgs { run, journal }: ReplayArgs,
) -> Result<ExitCode> {
    let store = open_journal(&journal.journal(workspace)?, Access::Read)?;
    let run = selector::resolve_run(&store, &run)?;
    let events = store.load(&run)?;
    let first = format!("{:?}", fold(&events)?);
    let second = format!("{:?}", fold(&events)?);
    anyhow::ensure!(first == second, "replay was not deterministic");
    let state = fold(&events)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "events": events.len(),
            "chain": "verified",
            "replay": "deterministic",
            "state": summarize(&state),
        }))?
    );
    Ok(Exit::Completed.into())
}

/// `brokkr runs`: one line per run, newest first — grouped by realm when
/// the world has several hearths.
pub(crate) fn runs(workspace: &Path, RunsArgs { realms, db, json }: RunsArgs) -> Result<ExitCode> {
    let hearths = hearths_of(workspace, realms, db)?;
    // A world with several hearths lists each one under its own
    // realm; a world with one is byte-for-byte the listing it
    // always was, down to opening its journal the same way.
    if hearths.len() > 1 {
        return runs_fleet(&hearths, json);
    }
    let db = hearths
        .into_iter()
        .next()
        .expect("a world resolves to at least one hearth")
        .journal;
    let store = open_journal(&db, Access::Read)?;
    // A fleet listing survives one corrupt journal: that run
    // becomes a quarantined row carrying the refusal's own words,
    // and the rest of the fleet is still listed. The journal is
    // never touched — the refusal is reported.
    let listed = fleet::read_hearth(&store)
        .listed()
        .map_err(anyhow::Error::msg)?;
    let entries: Vec<brokkr_view::RunEntry> = listed.iter().map(fleet::ListedRun::entry).collect();
    let view = brokkr_view::run_rows(&entries);
    if json {
        println!("{}", serde_json::to_string_pretty(&view)?);
    } else {
        print!(
            "{}",
            render::runs(&view, &now_rfc3339(), &render::Style::detect())
        );
    }
    Ok(Exit::Completed.into())
}

/// `brokkr runs` over several hearths: each journal's runs under its realm.
fn runs_fleet(hearths: &[Hearth], json: bool) -> Result<ExitCode> {
    let read: Vec<fleet::HearthRead> = hearths
        .iter()
        .map(|hearth| fleet::read_journal(&hearth.journal))
        .collect();
    let entries: Vec<Vec<brokkr_view::RunEntry>> = read
        .iter()
        .map(|hearth| hearth.runs.iter().map(fleet::ListedRun::entry).collect())
        .collect();
    let labels: Vec<String> = hearths.iter().map(Hearth::label).collect();
    let journals: Vec<String> = hearths
        .iter()
        .map(|hearth| hearth.journal.display().to_string())
        .collect();
    let grouped: Vec<brokkr_view::HearthEntries> = (0..hearths.len())
        .map(|index| brokkr_view::HearthEntries {
            realm: &labels[index],
            journal: &journals[index],
            entries: &entries[index],
            detail: read[index].detail.as_deref(),
        })
        .collect();
    let view = brokkr_view::fleet_rows(&grouped);
    if json {
        println!("{}", serde_json::to_string_pretty(&view)?);
    } else {
        print!(
            "{}",
            render::fleet(&view, &now_rfc3339(), &render::Style::detect())
        );
    }
    Ok(Exit::Completed.into())
}

/// `brokkr realms`: the world, each realm with its path, branch and HEAD.
pub(crate) fn realms(
    workspace: &Path,
    RealmsArgs { realms, db, json }: RealmsArgs,
) -> Result<ExitCode> {
    // `inspect`, not `resolve`: `realms` is a read surface with no
    // writes (decision 0023 ruling 6), and a readout that refuses
    // to describe the world because one contract moved is at its
    // least useful in exactly the world an operator typed it in.
    // The crossing lines below say which pin moved instead.
    let Invocation { world, journal, .. } = Invocation::inspect(workspace, realms, db)?.announce();
    let world = world.ok_or_else(|| {
        anyhow::anyhow!(
            "no map: this workspace has no {} and none was named with --realms",
            brokkr_core::realms::DEFAULT_MAP_FILE
        )
    })?;
    let rows = realms::rows(&world);
    let source = world.source.display().to_string();
    let journal = journal.display().to_string();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&realms::view(&source, &journal, &rows))?
        );
    } else {
        print!(
            "{}",
            realms::render(&source, &journal, &rows, realms::per_realm(&world, &rows))
        );
    }
    Ok(Exit::Completed.into())
}

/// `brokkr compare`: two runs' aligned outcomes.
pub(crate) fn compare(
    workspace: &Path,
    CompareArgs {
        run_a,
        run_b,
        journal,
    }: CompareArgs,
) -> Result<ExitCode> {
    compare::compare(&run_a, &run_b, &journal.journal(workspace)?)?;
    Ok(Exit::Completed.into())
}
