//! The verbs that read journals and the world and write nothing: `costs`,
//! `ledger`, `ui`, `tui`, `inspect`, `seats`, `watch`, `replay`, `runs`,
//! `realms` and `compare` (decision 0014: every readout is read-only).

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use brokkr_core::fold::fold;
use brokkr_runtime::realms::Hearth;
use brokkr_store::Store;
use brokkr_view::capability_calls::{CallEvidence, CapabilityCall, NO_RETAINED};
use serde_json::json;

use crate::cli_args::{CompareArgs, CostsArgs, InspectArgs, LedgerArgs, RealmsArgs};
use crate::cli_args::{ReplayArgs, RunsArgs, SeatsArgs, TuiArgs, UiArgs, WatchArgs};
use crate::{compare, fleet, ledger, realms, render, run_view, selector, tui};
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
    run_tui: impl FnOnce(Vec<Hearth>, Option<String>, usize) -> Result<tui::Closed>,
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
    // However the console closed, `brokkr tui` did what it was asked.
    run_tui(hearths, run, tab)?;
    Ok(Exit::Completed.into())
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

/// `inspect --json`'s bytes, which `seats --json` prints too: the view as
/// the browser is served it, every scope decided (#351). The readouts a
/// terminal prints never decide them (#624).
fn print_served(view: brokkr_view::RunView) -> Result<ExitCode> {
    println!("{}", serde_json::to_string_pretty(&view.with_scopes())?);
    Ok(Exit::Completed.into())
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
        capability_call,
        repo,
    }: InspectArgs,
) -> Result<ExitCode> {
    let view = run_view_of(workspace, realms, db, &run)?;
    if let Some(call_id) = capability_call {
        let repo = repo.as_deref().unwrap_or(workspace);
        return print_call(repo, &view, &call_id, json);
    }
    if json {
        return print_served(view);
    }
    // clap's ArgGroup already rules the two mutually exclusive.
    let scope = match (phase, seat) {
        (Some(phase), _) => Some(brokkr_view::Scope::Phase(phase)),
        (None, Some(seat)) => Some(brokkr_view::Scope::Seat(seat)),
        (None, None) => None,
    };
    let lens = brokkr_view::lens_for(&view, scope.as_ref())
        .map_err(|error| anyhow::Error::msg(render::scope_refusal(&error)))?;
    print!(
        "{}",
        render::inspect(&view, lens.as_ref(), true, &render::Style::detect())
    );
    Ok(Exit::Completed.into())
}

/// Why `inspect --capability-call` opens no call of the selected run's
/// (CR5), beside the runtime reader's `Unread`. Its `Display` is the
/// operator's text (decision 0071 ruling 8), by hand as this crate's
/// errors are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CallRefused {
    /// No checkpoint of the run records the call.
    Unrecorded,
    /// Two checkpoints of the run record it: choosing one would reconcile.
    RecordedTwice,
    /// The call cites no digest: the view's [`NO_RETAINED`].
    NoRetained,
    /// `--json`, and the verified bytes are not JSON.
    NotJson,
}

impl std::fmt::Display for CallRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CallRefused::Unrecorded => "capability call is not recorded in this run",
            CallRefused::RecordedTwice => "capability call is recorded more than once in this run",
            CallRefused::NoRetained => NO_RETAINED,
            CallRefused::NotJson => "retained response artifact is not JSON",
        })
    }
}

impl std::error::Error for CallRefused {}

/// The one call `call_id` names among the checkpoints of the run `view`
/// derives, beside the evidence the view derived for it.
fn call_of<'v>(
    view: &'v brokkr_view::RunView,
    call_id: &str,
) -> Result<(&'v CapabilityCall, &'v CallEvidence), CallRefused> {
    let mut named = (view.participants.iter())
        .flat_map(|participant| &participant.checkpoints)
        .map(|row| &row.capability_call)
        .filter_map(|evidence| Some((evidence.call.as_ref()?, evidence)))
        .filter(|(call, _)| call.call_id == call_id);
    let found = named.next().ok_or(CallRefused::Unrecorded)?;
    named
        .next()
        .map_or(Ok(found), |_| Err(CallRefused::RecordedTwice))
}

/// What `inspect --capability-call` writes of the verified `bytes`:
/// exactly them where its output is no terminal, as a script or a file
/// reads them (CR5); on a terminal, their text with every control and
/// reordering character taken out ([`render::Safe`]) and its line ended,
/// so a server's response can neither move the cursor nor reorder the
/// line it is read on.
fn shown(bytes: &[u8], terminal: bool) -> std::borrow::Cow<'_, [u8]> {
    match terminal {
        false => std::borrow::Cow::Borrowed(bytes),
        true => {
            let text = String::from_utf8_lossy(bytes);
            let line = format!("{}\n", render::Safe::new(&text).as_str());
            std::borrow::Cow::Owned(line.into_bytes())
        }
    }
}

/// `inspect --capability-call`: the one call `call_id` names in the run
/// `view` derives, with the provenance the view derived for it (decision
/// 0071 ruling 7), and the response its digest cites, opened and verified
/// in the run-local directory of the operated repository `repo`, resolved
/// as `ledger --repo` resolves it (operator ruling 2026-10-22), by the
/// runtime's one reader. With `json`, the view's call and the response's
/// own bytes, unparsed, in one envelope. No artifact path is taken from
/// the operator, nothing is fetched and nothing is written (CR5).
fn print_call(
    repo: &Path,
    view: &brokkr_view::RunView,
    call_id: &str,
    json: bool,
) -> Result<ExitCode> {
    let (call, evidence) = call_of(view, call_id)?;
    let sha256 = call.response_sha256.as_deref();
    let sha256 = sha256.ok_or(CallRefused::NoRetained)?;
    let run_local = crate::folder::run_local(repo)?;
    let bytes = brokkr_runtime::engine::artifacts::retained(&run_local, sha256)?;
    let response = shown(&bytes, std::io::stdout().is_terminal());
    let mut out = std::io::stdout().lock();
    if json {
        let parsed = serde_json::from_slice::<serde::de::IgnoredAny>(&bytes);
        parsed.map_err(|_| CallRefused::NotJson)?;
        let call = serde_json::to_string(call)?;
        write!(out, "{{\"call\":{call},\"response\":")?;
        out.write_all(&response)?;
        out.write_all(b"}\n")?;
    } else {
        let said = [&call.call_id, &evidence.cell.text, &evidence.response.text];
        let [id, line, digest] = said.map(|text| render::Safe::new(text));
        let (id, line, digest) = (id.as_str(), line.as_str(), digest.as_str());
        write!(
            out,
            "capability call {id}\n{line}\nresponse sha256 {digest}\n\n"
        )?;
        out.write_all(&response)?;
    }
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
        return print_served(view);
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

/// `brokkr runs`: one line per run, the fleet's sections in order and
/// each by last activity — grouped by realm when the world has several
/// hearths. One hearth is read a page at a time (decision 0089).
pub(crate) fn runs(workspace: &Path, args: RunsArgs) -> Result<ExitCode> {
    let RunsArgs {
        realms,
        db,
        json,
        limit,
        after,
    } = args;
    let hearths = hearths_of(workspace, realms.clone(), db.clone())?;
    // A world with several hearths lists each one under its own
    // realm; a world with one is byte-for-byte the listing it
    // always was, down to opening its journal the same way.
    if hearths.len() > 1 {
        anyhow::ensure!(
            limit.is_none() && after.is_none(),
            "--limit and --after page one hearth's runs, and this world has {} hearths: name \
             one journal with --db",
            hearths.len()
        );
        return runs_fleet(&hearths, json);
    }
    let called = fleet::Called {
        db: db.as_deref(),
        realms: realms.as_deref(),
    };
    let db = hearths
        .into_iter()
        .next()
        .expect("a world resolves to at least one hearth")
        .journal;
    let asked = (json, limit, after.as_ref());
    let style = render::Style::detect();
    hearth_runs(&db, asked, called, &style, &mut std::io::stdout().lock())?;
    Ok(Exit::Completed.into())
}

/// `brokkr runs` over one hearth's journal at `db`, written to `out`: every
/// run, or the page `limit` and `after` ask for, as JSON when asked, its
/// next page's command `called` as this one was.
pub(crate) fn hearth_runs(
    db: &Path,
    (json, limit, after): (bool, Option<u32>, Option<&brokkr_view::Cursor>),
    called: fleet::Called<'_>,
    style: &render::Style,
    out: &mut impl std::io::Write,
) -> Result<()> {
    let before = fleet::Journal::at(db);
    let store = open_journal(db, Access::Read)?;
    // A fleet listing survives one corrupt journal: that run
    // becomes a quarantined row carrying the refusal's own words,
    // and the rest of the fleet is still listed. The journal is
    // never touched — the refusal is reported. Read a page at a time
    // from the summary; a journal the summary does not page yet is read
    // the kept way (#744). A refresh reads at most a page and one ahead
    // of the runs behind the summary whole and counts the rest; the
    // listing of every run keys the rest and lists each at its place, all
    // of it one read of the journal (ruling 6).
    let now = now_rfc3339();
    let every = limit.is_none() && after.is_none();
    fleet::list_hearth(&store, (before, &now), every, |listing| {
        match (limit, after) {
            (None, None) if json => listing.every_json(out),
            (None, None) => listing.every_text(out, style),
            (limit, after) => {
                let limit = limit.map_or(fleet::PAGE_ROWS, |limit| limit as usize);
                listing.one(out, (after, limit), json, called, style)
            }
        }
    })
}

/// `brokkr runs` over several hearths: each journal's runs under its
/// realm, a hearth at a time and each a page at a time (decision 0089).
fn runs_fleet(hearths: &[Hearth], json: bool) -> Result<ExitCode> {
    let style = render::Style::detect();
    let now = now_rfc3339();
    fleet::list_world(hearths, json, (&now, &style), &mut std::io::stdout().lock())?;
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
            "no map: this workspace has no {}/{map} or ./{map} and none was named with --realms",
            brokkr_runtime::places::FOLDER,
            map = brokkr_core::realms::DEFAULT_MAP_FILE
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

#[cfg(test)]
mod tests;
