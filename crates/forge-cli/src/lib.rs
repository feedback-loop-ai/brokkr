//! `brokkr` — the one shipped binary (decision 0003, named by decision
//! 0019). No UI, no required services: one executable, one workspace
//! database. Two bin targets share this one entry point: `brokkr`, and
//! the `forge` shim that keeps the old name working for one release.

mod agents;
mod compare;
mod doctor;
mod init;
mod muninn;
mod realms;
mod recipes;
mod render;
mod selector;
mod tui;
mod ui;

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{ArgGroup, Parser, Subcommand};
use forge_core::fold::{fold, RunState, Status};
use forge_runtime::realms::World;
use forge_runtime::{operator_command, Bundle, Engine};
use forge_store::Store;
use serde_json::{json, Value};

/// The workspace journal a command opens when neither a map nor `--db`
/// says otherwise. Unchanged since the first release, and the reason a
/// world that never drew a map notices nothing.
pub const DEFAULT_DB: &str = ".forge/forge.db";

/// Exit codes: 0 completed/ok · 2 parked (operator needed) · 3 stopped ·
/// 1 error.
#[derive(Parser)]
// `bin_name` is pinned, not inferred from argv[0]: the `forge` shim
// prints the same usage as `brokkr`, so the only place the old name
// survives in output is the shim's own notice (decision 0019 ruling 9).
#[command(
    name = "brokkr",
    bin_name = "brokkr",
    version,
    about = "Deterministic delivery engine"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Scaffold a minimal reviewable bundle and prove it compiles.
    Init { dir: PathBuf },
    /// Per-seat cost and session accounting from journal checkpoints —
    /// the LaneTally join surface (stable seat ids, journal-derived).
    Costs {
        #[arg(long)]
        run: String,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
    },
    /// Anchor a run's journal head in refs/forge/<run> (tamper evidence),
    /// or verify the existing anchor with --check.
    Anchor {
        /// Full run id, a unique run-id prefix, or `latest`.
        #[arg(long)]
        run: String,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        /// Verify instead of writing a new anchor.
        #[arg(long)]
        check: bool,
    },
    /// Serve the embedded read-only surface on loopback.
    Ui {
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
        #[arg(long, default_value_t = 8383)]
        port: u16,
        /// Open the system browser after binding.
        #[arg(long)]
        open: bool,
    },
    /// Explore runs interactively in the terminal: the fleet as a
    /// navigable table, one run's phase graph, seats and decision
    /// trail, and one seat's own checkpoint and session stream.
    /// Read-only like every other readout (decision 0014) — it issues
    /// no operator commands and writes nothing to the journal.
    Tui {
        /// Full run id, a unique run-id prefix, or `latest`; opens
        /// directly at that run's level.
        #[arg(long)]
        run: Option<String>,
        /// The world's map — the journal it names is the one opened
        /// (default ./realms.json when present).
        #[arg(long)]
        realms: Option<PathBuf>,
        /// The workspace journal. Outranks the map's journal; without
        /// either, .forge/forge.db as always.
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// Verify tools, drivers, the workspace database, and optionally a
    /// bundle, without executing any agent.
    Doctor {
        #[arg(long)]
        bundle: Option<PathBuf>,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
    },
    /// Validate a bundle and print its pinned manifest and digest.
    Compile {
        #[arg(long)]
        bundle: PathBuf,
    },
    /// Start a new run and drive it until it parks or finishes.
    #[command(group(ArgGroup::new("delivery").required(true).args(["bundle", "recipe"])))]
    Run {
        #[arg(long)]
        bundle: Option<PathBuf>,
        /// Named recipe, resolved to <recipes-dir>/<name>.
        #[arg(long)]
        recipe: Option<String>,
        #[arg(long, default_value = "recipes")]
        recipes_dir: PathBuf,
        #[arg(long)]
        feature: String,
        /// The world's map: realms and the journal they share (decision
        /// 0023). Defaults to ./realms.json when there is one; a map
        /// named here and missing or malformed is a refusal, never a
        /// silent fallback.
        #[arg(long)]
        realms: Option<PathBuf>,
        /// The workspace journal. Outranks the map's journal; without
        /// either, .forge/forge.db as always.
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Canonical forge-dispatch/v2 JSON. When present the run id,
        /// Looper/grant correlation, recipe, repository, budget, and producer
        /// bounds are pinned into an immutable run-manifest/v2.
        #[arg(long)]
        dispatch: Option<PathBuf>,
        /// Operator-side secrets store for seats with declared bindings
        /// (default <workdir>/.forge/secrets.env).
        #[arg(long)]
        secrets_file: Option<PathBuf>,
    },
    /// Resume an existing run under its exact pinned bundle.
    #[command(group(ArgGroup::new("delivery").required(true).args(["bundle", "recipe"])))]
    Resume {
        #[arg(long)]
        bundle: Option<PathBuf>,
        /// Named recipe, resolved to <recipes-dir>/<name>.
        #[arg(long)]
        recipe: Option<String>,
        #[arg(long, default_value = "recipes")]
        recipes_dir: PathBuf,
        #[arg(long)]
        run: String,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Operator-side secrets store for seats with declared bindings
        /// (default <workdir>/.forge/secrets.env).
        #[arg(long)]
        secrets_file: Option<PathBuf>,
    },
    /// Re-run a past run's feature as a NEW run under another bundle or
    /// recipe, so outcomes can be compared by run id. No stored linkage.
    #[command(group(ArgGroup::new("delivery").required(true).args(["bundle", "recipe"])))]
    Rerun {
        /// The source run whose feature is re-run.
        #[arg(long)]
        run: String,
        #[arg(long)]
        bundle: Option<PathBuf>,
        /// Named recipe, resolved to <recipes-dir>/<name>.
        #[arg(long)]
        recipe: Option<String>,
        #[arg(long, default_value = "recipes")]
        recipes_dir: PathBuf,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Operator-side secrets store for seats with declared bindings
        /// (default <workdir>/.forge/secrets.env).
        #[arg(long)]
        secrets_file: Option<PathBuf>,
    },
    /// Compare two runs' aligned outcomes: decision trails, first
    /// divergence, phases visited, per-seat costs. Read-only.
    Compare {
        run_a: String,
        run_b: String,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
    },
    /// The recipe library: bundle directories as named, swappable
    /// delivery strategies.
    Recipes {
        #[command(subcommand)]
        command: RecipesCmd,
    },
    /// The agent library (decision 0016): one definition per agent —
    /// description, charter, an ordered chain of abstract model names,
    /// abstract tool/MCP configuration — that seats reference by name.
    Agents {
        #[command(subcommand)]
        command: AgentsCmd,
    },
    /// The standing overseer (decision 0020): read the fleet, propose to
    /// the operator, execute nothing. It opens the journal read-only,
    /// issues no operator command, starts no run, and records every
    /// proposal — with the run ids and sequence numbers it was derived
    /// from — in its own append-only file beside the journal.
    Muninn {
        #[command(subcommand)]
        command: MuninnCmd,
    },
    /// Manage the operator-side secrets store (decision 0012): bundles
    /// and journals carry NAMES only; values live in this env-format
    /// file outside version control. There is no value-printing verb.
    Secrets {
        #[command(subcommand)]
        command: SecretsCmd,
    },
    /// Record an operator command (retry | stop) as journal events.
    Operator {
        #[arg(long)]
        run: String,
        /// "retry" re-runs the current phase; "stop" ends the run.
        command: String,
        #[arg(long)]
        reason: String,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
    },
    /// Explain a run: header, ruling, seats, decision trail, and the
    /// phase graph as a tree. `--phase` and `--seat` are the scoping
    /// verbs the console's clicks became; `--json` emits the view model.
    #[command(group(ArgGroup::new("scope").args(["phase", "seat"])))]
    Inspect {
        /// Full run id, a unique run-id prefix, or `latest`.
        #[arg(long)]
        run: String,
        /// The world's map — the journal it names is the one opened
        /// (default ./realms.json when present).
        #[arg(long)]
        realms: Option<PathBuf>,
        /// The workspace journal. Outranks the map's journal; without
        /// either, .forge/forge.db as always.
        #[arg(long)]
        db: Option<PathBuf>,
        /// Emit the view model verbatim — this is what scripts read.
        #[arg(long)]
        json: bool,
        /// Scope the readout to one phase.
        #[arg(long)]
        phase: Option<String>,
        /// Scope the readout to one seat, by label or participant key.
        #[arg(long)]
        seat: Option<String>,
    },
    /// Watch a run live: redraw the graph, seats, last ruling and seat
    /// activity whenever the journal head moves; exit when the run
    /// reaches a terminal status. Read-only, like every other readout.
    Watch {
        /// Full run id, a unique run-id prefix, or `latest`.
        #[arg(long)]
        run: String,
        /// The world's map — the journal it names is the one opened
        /// (default ./realms.json when present).
        #[arg(long)]
        realms: Option<PathBuf>,
        /// The workspace journal. Outranks the map's journal; without
        /// either, .forge/forge.db as always.
        #[arg(long)]
        db: Option<PathBuf>,
        /// Print one frame and exit.
        #[arg(long)]
        once: bool,
        /// Poll interval in milliseconds (floored at 100).
        #[arg(long = "interval", default_value_t = 750)]
        interval_ms: u64,
    },
    /// Rebuild state from the journal twice and verify determinism.
    Replay {
        /// Full run id, a unique run-id prefix, or `latest`.
        #[arg(long)]
        run: String,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
    },
    /// Write the canonical NDJSON journal and pinned manifest.
    Export {
        /// Full run id, a unique run-id prefix, or `latest`.
        #[arg(long)]
        run: String,
        #[arg(long, default_value = ".")]
        out: PathBuf,
        /// The world's map — the journal it names is the one opened
        /// (default ./realms.json when present).
        #[arg(long)]
        realms: Option<PathBuf>,
        /// The workspace journal. Outranks the map's journal; without
        /// either, .forge/forge.db as always.
        #[arg(long)]
        db: Option<PathBuf>,
        /// Also write a sanitized copy for publishable fixtures —
        /// `<run>.redacted.ndjson` and `<run>.redacted.manifest.json` —
        /// with every absolute path in event payloads rewritten to a
        /// stable placeholder. The verbatim pair is written unchanged;
        /// the redacted copy's recorded hashes no longer verify, and its
        /// manifest says so.
        #[arg(long)]
        redact: bool,
    },
    /// Verify an exported journal offline: chain, envelopes, fold.
    VerifyRun { file: PathBuf },
    /// Synchronize a Looper-bound run over the authenticated producer API.
    /// The API key is read from an environment variable and is never stored.
    Bridge {
        #[arg(long)]
        run: String,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
        #[arg(long)]
        looper_url: String,
        #[arg(long, default_value = "LOOPER_API_KEY")]
        token_env: String,
        /// Keep tailing the verified journal and command feed.
        #[arg(long)]
        follow: bool,
        #[arg(long, default_value_t = 750)]
        interval_ms: u64,
    },
    /// List runs in the workspace database: one clamped line per run,
    /// newest first. `--json` emits the view model for scripts.
    Runs {
        /// The world's map — the journal it names is the one opened
        /// (default ./realms.json when present).
        #[arg(long)]
        realms: Option<PathBuf>,
        /// The workspace journal. Outranks the map's journal; without
        /// either, .forge/forge.db as always.
        #[arg(long)]
        db: Option<PathBuf>,
        /// Emit the view model verbatim — this is what scripts read.
        #[arg(long)]
        json: bool,
    },
    /// List the world (decision 0023): each realm with its path, default
    /// branch and current HEAD, and the journal the world writes.
    /// Read-only, like every other readout.
    Realms {
        /// The map to read (default ./realms.json).
        #[arg(long)]
        realms: Option<PathBuf>,
        /// The workspace journal. Outranks the map's journal; without
        /// either, .forge/forge.db as always.
        #[arg(long)]
        db: Option<PathBuf>,
        /// Emit the view model verbatim — this is what scripts read.
        #[arg(long)]
        json: bool,
    },
    /// Run a built-in forge-driver/v1 adapter (claude | lanetally | codex | dsh | exec).
    /// Bundles reference these as {forge} driver <kind> -- <extra args>.
    Driver {
        kind: String,
        /// Arguments after -- pass to the agent CLI
        /// (claude/lanetally/codex/dsh) or form the command template
        /// (exec).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// (internal) Scripted forge-driver/v1 driver for machine proof.
    #[command(hide = true)]
    FakeDriver {
        #[arg(long)]
        script: PathBuf,
        #[arg(long)]
        state: PathBuf,
        /// The concrete model an adapter pinned (decision 0016). Echoed
        /// back as a checkpoint so a proof can assert the pin actually
        /// reached the driver rather than trusting the composed argv.
        #[arg(long)]
        model: Option<String>,
    },
}

#[derive(Subcommand)]
enum SecretsCmd {
    /// Bind NAME to a value read from STDIN (never argv — the CLI obeys
    /// its own injection discipline). Creates the store 0600.
    Set {
        name: String,
        #[arg(long, default_value = ".forge/secrets.env")]
        secrets_file: PathBuf,
    },
    /// Print bound names, one per line — names, never values.
    List {
        #[arg(long, default_value = ".forge/secrets.env")]
        secrets_file: PathBuf,
    },
    /// Remove NAME from the store.
    Remove {
        name: String,
        #[arg(long, default_value = ".forge/secrets.env")]
        secrets_file: PathBuf,
    },
}

#[derive(Subcommand)]
enum AgentsCmd {
    /// One line per agent — name, model chain, description. A broken
    /// definition prints a warning line and never aborts the listing.
    List {
        #[arg(long, default_value = forge_runtime::bundle::DEFAULT_AGENTS_DIR)]
        agents_dir: PathBuf,
    },
    /// The definition as written, plus the per-chain-entry resolution
    /// the compiler would compute. An unknown name errors naming the
    /// known set.
    Show {
        name: String,
        #[arg(long, default_value = forge_runtime::bundle::DEFAULT_AGENTS_DIR)]
        agents_dir: PathBuf,
        #[arg(long, default_value = forge_runtime::bundle::DEFAULT_ADAPTERS_DIR)]
        adapters_dir: PathBuf,
    },
}

#[derive(Subcommand)]
enum MuninnCmd {
    /// Derive the fleet dossier, ask one bounded seat for proposals, and
    /// record them. Nothing is executed: a proposal becomes an action
    /// only when the operator issues the command themselves.
    Run {
        /// The world's map — the journal it names is the fleet this
        /// reading covers (default ./realms.json when present).
        #[arg(long)]
        realms: Option<PathBuf>,
        /// The workspace journal. Outranks the map's journal; without
        /// either, .forge/forge.db as always.
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, default_value = forge_runtime::bundle::DEFAULT_AGENTS_DIR)]
        agents_dir: PathBuf,
        #[arg(long, default_value = forge_runtime::bundle::DEFAULT_ADAPTERS_DIR)]
        adapters_dir: PathBuf,
        #[arg(long, default_value = muninn::DEFAULT_RECORD)]
        record: PathBuf,
    },
    /// Read the record back: every proposal, with the run ids and
    /// sequence numbers it cited.
    List {
        #[arg(long, default_value = muninn::DEFAULT_RECORD)]
        record: PathBuf,
        /// Emit the recorded entries verbatim — this is what scripts read.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum RecipesCmd {
    /// List recipes under --dir plus the built-in bundles; broken ones
    /// print a warning line, never abort the listing.
    List {
        #[arg(long, default_value = "recipes")]
        dir: PathBuf,
    },
    /// Install a recipe from a local path or a git URL into <dir>/<name>.
    Add {
        source: String,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "recipes")]
        dir: PathBuf,
    },
    /// Print one recipe's RESOLVED bundle and, when it extends another,
    /// the composition chain it was resolved from (decision 0017).
    Show {
        name: String,
        #[arg(long, default_value = "recipes")]
        dir: PathBuf,
    },
}

/// What a compiled bundle looks like to an operator. `forge compile` and
/// `forge recipes show` print this and nothing else, from here, so the
/// two surfaces can never drift. `composed_from` is omitted entirely
/// when nothing was composed, so a plain bundle's output is unchanged.
fn compiled_view(bundle: &Bundle) -> Value {
    let mut view = json!({
        "bundle": bundle.name,
        "digest": bundle.manifest_digest(),
        "phases": bundle.machine.phases,
        "seats": bundle.seats.keys().collect::<Vec<_>>(),
        "manifest": bundle.manifest,
    });
    if !bundle.chain.is_empty() {
        view["composed_from"] = Value::Array(
            bundle
                .chain
                .iter()
                .map(|ancestor| {
                    json!({
                        "recipe": ancestor.name,
                        "digest": ancestor.digest,
                        "dir": ancestor.dir.display().to_string(),
                    })
                })
                .collect(),
        );
    }
    view
}

fn status_str(status: &Status) -> &'static str {
    match status {
        Status::Running => "running",
        Status::AwaitingOperator => "awaiting_operator",
        Status::Completed => "completed",
        Status::Stopped => "stopped",
    }
}

fn summarize(state: &RunState) -> Value {
    json!({
        "run_id": state.run_id,
        "seq": state.seq,
        "status": status_str(&state.status),
        "phase": state.phase,
        "cursor": format!("{:?}", state.cursor),
        "park_reason": state.park_reason,
        "consecutive_failures": state.consecutive_failures,
        "last_decision": state.last_decision,
        "feature": state.feature,
    })
}

/// Exit codes: 0 completed · 2 parked (operator needed) · 3 stopped ·
/// 1 still running. One mapping, shared by `finish` and `watch`.
fn status_exit(status: &Status) -> ExitCode {
    match status {
        Status::Completed => ExitCode::SUCCESS,
        Status::AwaitingOperator => ExitCode::from(2),
        Status::Stopped => ExitCode::from(3),
        Status::Running => ExitCode::from(1),
    }
}

fn finish(state: &RunState) -> ExitCode {
    println!(
        "{}",
        serde_json::to_string_pretty(&summarize(state)).unwrap()
    );
    status_exit(&state.status)
}

/// The one clock read that keeps the derivation pure: `forge-view` has
/// no clock, so `now` arrives as a parameter.
fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("the clock formats as RFC 3339")
}

/// One `watch` frame: clear-and-home on a tty, an appended timestamped
/// frame otherwise so pipes and CI logs read as a timeline. The redraw
/// is unconditional — the alternative needs `LINES`, which is usually
/// unset, so a height gate would mean never redrawing at all. A frame
/// taller than the terminal shows its tail.
fn write_frame(
    out: &mut dyn std::io::Write,
    frame: &str,
    is_tty: bool,
    clock: &mut dyn FnMut() -> String,
) -> Result<()> {
    // Every frame ends in a newline and `Stdout` is line-buffered, so
    // there is nothing here for an explicit flush to do.
    if is_tty {
        write!(out, "\x1b[2J\x1b[H{frame}")?;
    } else {
        write!(out, "── {} ──\n{frame}", clock())?;
    }
    Ok(())
}

/// A transient store error (SQLITE_BUSY while a `forge run` holds the
/// write lock) is a frame that says so, not an exit. A persistent one
/// exits nonzero.
pub(crate) const WATCH_TRANSIENT_FRAMES: usize = 5;

/// Poll the journal head and redraw when it moves, comparing **both**
/// seq and hash: a rewritten journal at equal seq is the tamper case
/// `anchor` exists for, and `watch` should redraw rather than sit blind.
#[allow(clippy::too_many_arguments)]
fn watch_loop(
    db: &std::path::Path,
    run: &str,
    interval_ms: u64,
    is_tty: bool,
    style: &render::Style,
    out: &mut dyn std::io::Write,
    clock: &mut dyn FnMut() -> String,
    sleep: &mut dyn FnMut(u64),
    max_iterations: usize,
) -> Result<ExitCode> {
    let mut last: Option<(u64, String)> = None;
    let mut failures = 0usize;
    for iteration in 0..max_iterations {
        if iteration > 0 {
            sleep(interval_ms.max(100));
        }
        let head = Store::open(db).and_then(|store| store.head_hash(run).map(|h| (h, store)));
        match head {
            Ok((head, store)) => {
                failures = 0;
                if last.as_ref() != Some(&head) {
                    last = Some(head);
                    let events = store.load(run)?;
                    let state = fold(&events).ok();
                    let view = forge_view::run_view(&events, state.as_ref());
                    let frame = render::inspect(&view, None, false, style);
                    write_frame(out, &frame, is_tty, clock)?;
                    if let Some(state) = state {
                        // A park admits no further events until a human
                        // acts, so "keep watching" is an unbounded CI
                        // hang. The park reason printed first is the
                        // frame's own header.
                        if state.status != Status::Running {
                            return Ok(status_exit(&state.status));
                        }
                    }
                }
            }
            Err(error) => {
                failures += 1;
                let frame = format!("the journal is not readable right now: {error}\n");
                write_frame(out, &frame, is_tty, clock)?;
                anyhow::ensure!(
                    failures < WATCH_TRANSIENT_FRAMES,
                    "giving up on {} after {failures} unreadable polls: {error}",
                    db.display()
                );
            }
        }
    }
    Ok(ExitCode::from(1))
}

fn driver_extra_args(args: Vec<String>) -> Vec<String> {
    if let Some(index) = args.iter().position(|arg| arg == "--") {
        args[index + 1..].to_vec()
    } else {
        args
    }
}

/// The one plain line the `forge` shim writes to stderr before it
/// proceeds — stderr only, so piped stdout and JSON consumers are
/// untouched. Decision 0019 ruling 9; law 4 keeps it plain.
pub const SHIM_NOTICE: &str =
    "notice: forge is now named brokkr; the forge name works for one more release.";

/// Both bins enter here: same parse, same commands, same exit codes.
pub fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}

/// The transcript's own liveness, asked beside the journal head's: a
/// seat's prose lands BETWEEN checkpoints, so a working seat's file
/// growing is a refresh reason in its own right. Size only, read at the
/// shell's existing tick — no watch, no dependency — and only while the
/// seat can still write: the state resets with the session, so one
/// seat's length never speaks for another's, and a concluded seat is
/// not stat'd at all.
fn transcript_moved(ask: &tui::Ask, seen: &mut Option<(String, u64)>) -> bool {
    let Some(session) = ask.session.filter(|_| ask.working) else {
        *seen = None;
        return false;
    };
    let size = ui::transcript_path(session).map_or(0, |file| ui::transcript_len(&file));
    let grew = match seen {
        Some((watched, previous)) => {
            watched == session && ui::transcript_grew(Some(*previous), size)
        }
        None => false,
    };
    *seen = Some((session.to_string(), size));
    grew
}

/// Fold one run of a FLEET read. A journal that does not fold
/// quarantines that row — its error text becomes the row's detail — so
/// one corrupt run cannot blind an operator to every other run. Single-
/// run verbs (`inspect`, `watch`, `resume`) keep their bare `fold(..)?`:
/// a command aimed at one run must fail loudly on that run.
pub(crate) fn fold_or_quarantine(events: &[forge_core::EventEnvelope]) -> Result<RunState, String> {
    fold(events).map_err(|error| error.to_string())
}

/// One refresh for `forge tui`: the only place a store is opened on that
/// path, and the reason `tui.rs` can name none. The head is compared on
/// **both** seq and hash — a rewritten journal at equal seq is the
/// tamper case `anchor` exists for, and a console should redraw rather
/// than sit blind.
fn tui_views(
    db: &std::path::Path,
    ask: tui::Ask,
    head: &mut Option<(u64, String)>,
    seen: &mut Option<(String, u64)>,
    clock: fn() -> String,
) -> Result<tui::Refreshed> {
    let store = Store::open(db)?;
    let current = match ask.run {
        Some(run) => Some(store.head_hash(run)?),
        None => None,
    };
    // Unconditional, and before the gate: the size that was observed is
    // the size the next tick compares against, whatever the gate rules.
    let grew = transcript_moved(&ask, seen);
    let moved = current != *head;
    if !(ask.force || ask.fleet || moved || grew) {
        // Nothing has moved: the console keeps the frame it has, and
        // nothing is re-folded at four polls a second.
        return Ok(None);
    }
    *head = current;
    let mut folded = Vec::new();
    for (run_id, feature, created_at) in store.list_runs()? {
        // Deliberately not `?`: a console would otherwise lose the
        // operator's whole fleet table because one old run is corrupt.
        // The absence mark is `RunRow.status_known`'s job (0001), and
        // the fold's own words now ride along as the row's detail
        // instead of being discarded.
        let folded_run = store
            .load(&run_id)
            .ok()
            .map(|events| fold_or_quarantine(&events));
        folded.push((run_id, feature, created_at, folded_run));
    }
    let entries: Vec<forge_view::RunEntry> = folded
        .iter()
        .map(
            |(run_id, feature, created_at, folded_run)| forge_view::RunEntry {
                run_id,
                feature,
                created_at,
                state: folded_run.as_ref().and_then(|folded| folded.as_ref().ok()),
                detail: folded_run
                    .as_ref()
                    .and_then(|folded| folded.as_ref().err())
                    .map(String::as_str),
            },
        )
        .collect();
    let run = ask.run.and_then(|run| store.load(run).ok()).map(|events| {
        let state = fold(&events).ok();
        forge_view::run_view(&events, state.as_ref())
    });
    Ok(Some(tui::Views {
        now: clock(),
        runs: forge_view::run_rows(&entries),
        run,
        // The seat's own session, located by the SAME lookup the
        // console's /api/session endpoint uses.
        transcript: ask.session.and_then(ui::session_turns),
    }))
}

/// The refresh source the console runs on, with the workspace clock
/// bound in. Built by a named function so it is reachable from a test
/// as well as from `forge tui`.
fn tui_source<'a>(
    db: &'a std::path::Path,
    head: &'a mut Option<(u64, String)>,
    seen: &'a mut Option<(String, u64)>,
) -> impl FnMut(tui::Ask) -> Result<tui::Refreshed> + 'a {
    move |ask| tui_views(db, ask, head, seen, now_rfc3339)
}

/// `forge tui`'s impure entry: the environment facts are read once here
/// and everything else is injected. The refusals live inside
/// `tui::start`, and the source above opens a store only when it is
/// called — which is after that gate.
fn run_tui(db: PathBuf, run: Option<String>) -> Result<ExitCode> {
    let mut head: Option<(u64, String)> = None;
    let mut seen: Option<(String, u64)> = None;
    let db_is_file = db.is_file();
    let mut source = tui_source(&db, &mut head, &mut seen);
    tui::start(
        db_is_file,
        run,
        tui::production_ops(),
        std::io::stdout().is_terminal(),
        // Animation is enabled exactly when colour is, through the same
        // pure rule `forge runs` uses: NO_COLOR, TERM=dumb and a
        // non-tty all yield a still graph. No new flag, no new env var.
        render::Style::detect().color,
        ratatui::backend::CrosstermBackend::new(std::io::stdout()),
        std::io::stdout(),
        &mut source,
        usize::MAX,
    )
}

fn run(cli: Cli) -> Result<ExitCode> {
    run_with(
        cli,
        std::path::Path::new("."),
        ui::serve,
        None,
        None,
        run_tui,
    )
}

/// What an invocation resolved before it opens anything: the world it
/// reads in, whether the operator NAMED that map, and the journal.
///
/// Three rules, in this order (decision 0023 ruling 3): a map named with
/// `--realms` is loaded or the command refuses; otherwise `./realms.json`
/// is the map when it exists; and `--db` outranks whatever journal the
/// map names. With neither map nor `--db` the journal is the default it
/// has always been — a world that never drew a map notices nothing.
struct Invocation {
    world: Option<World>,
    /// True only when `--realms` named the map. A map merely lying in
    /// the workspace is ambient: a surface that cannot honour a world
    /// may say so and carry on, but none may ignore one the operator
    /// typed.
    named: bool,
    journal: PathBuf,
    /// What an ambient map moved without being asked to. Ruling 3 wants
    /// a map found to be adopted; it does not want the adoption silent,
    /// so a journal that is somewhere else BECAUSE of a map nobody typed
    /// is said out loud — once, on stderr, before anything opens it.
    notice: Option<String>,
}

/// One journal, however it is written. A relative path means what it
/// means to the process that would open it, so a map naming
/// `.forge/forge.db` beside the working directory has moved nothing —
/// and a path that cannot be located at all compares as written.
fn same_journal(left: &std::path::Path, right: &std::path::Path) -> bool {
    let located =
        |path: &std::path::Path| std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    located(left) == located(right)
}

impl Invocation {
    fn resolve(
        workspace: &std::path::Path,
        realms: Option<PathBuf>,
        db: Option<PathBuf>,
    ) -> Result<Invocation> {
        let named = realms.is_some();
        let overridden = db.is_some();
        let world = World::discover(workspace, realms.as_deref())?;
        let mapped = world.as_ref().map(World::journal);
        let journal = db
            .or_else(|| mapped.clone())
            .unwrap_or_else(|| PathBuf::from(DEFAULT_DB));
        // A note is owed whenever a map nobody typed is adopted at all
        // (the run's own review caught the quiet arm: a map that keeps
        // the journal in place still decides realm paths and fact
        // keys). A map the operator named is one the operator knows
        // about; everything ambient is said out loud — once, on
        // stderr, before anything opens.
        let notice = mapped
            .zip(
                world
                    .as_ref()
                    .map(|world| world.source.display().to_string()),
            )
            .filter(|_| !named)
            .map(|(mapped, source)| {
                if overridden || same_journal(&mapped, std::path::Path::new(DEFAULT_DB)) {
                    format!(
                        "note: the map {source} found in this workspace is adopted \
                         (journal unchanged); --realms names it explicitly"
                    )
                } else {
                    format!(
                        "note: the journal is {}, named by the map {source} found in this \
                         workspace rather than typed with --realms; --db outranks it",
                        mapped.display(),
                    )
                }
            });
        Ok(Invocation {
            world,
            named,
            journal,
            notice,
        })
    }

    /// Say the note, if there is one, before the caller opens anything.
    fn announce(self) -> Invocation {
        if let Some(notice) = &self.notice {
            eprintln!("{notice}");
        }
        self
    }
}

/// The journal alone, for the read surfaces that take a map only to know
/// which world's fleet they are reading.
fn journal_of(
    workspace: &std::path::Path,
    realms: Option<PathBuf>,
    db: Option<PathBuf>,
) -> Result<PathBuf> {
    Ok(Invocation::resolve(workspace, realms, db)?
        .announce()
        .journal)
}

fn run_with(
    cli: Cli,
    // The directory `realms.json` is discovered in. Injected rather than
    // read from the process, so what a command resolves is a function of
    // its arguments and not of where the caller happens to stand.
    workspace: &std::path::Path,
    serve_ui: impl FnOnce(PathBuf, u16, bool) -> std::io::Result<()>,
    bridge_iteration_limit: Option<usize>,
    watch_iteration_limit: Option<usize>,
    run_tui: impl FnOnce(PathBuf, Option<String>) -> Result<ExitCode>,
) -> Result<ExitCode> {
    match cli.command {
        Cmd::Init { dir } => {
            let digest = init::init(&dir)?;
            eprintln!(
                "initialized reviewable bundle at {} (digest {digest})",
                dir.display()
            );
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Costs { run, db } => {
            let store = Store::open(&db)?;
            let events = store.load(&run)?;
            let (report, total) = compare::seat_costs(&events);
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "run_id": run,
                    "seats": report,
                    "total_cost_usd": total,
                }))?
            );
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Anchor {
            run,
            db,
            repo,
            check,
        } => {
            let store = Store::open(&db)?;
            let run = selector::resolve_run(&store, &run)?;
            if check {
                let report = forge_runtime::verify_anchor(&store, &repo, &run)?;
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                let sha = forge_runtime::anchor(&store, &repo, &run)?;
                eprintln!("anchored {run} at {sha}");
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Ui { db, port, open } => {
            serve_ui(db, port, open)?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Tui { run, realms, db } => {
            let db = journal_of(workspace, realms, db)?;
            // Selectors resolve through decision 0015's one resolver —
            // but resolving needs a store, and `forge tui` refuses a
            // missing database *before* anything opens one, because
            // `Store::open` creates a file, a WAL and a meta row. So
            // resolution waits until the file is known to exist and
            // `tui::start` does the refusing.
            let run = match (run, db.is_file()) {
                (Some(run), true) => Some(selector::resolve_run(&Store::open(&db)?, &run)?),
                (run, _) => run,
            };
            run_tui(db, run)
        }
        Cmd::Doctor { bundle, db } => {
            let report = doctor::doctor(bundle.as_deref(), &db);
            println!("{}", report.render());
            Ok(if report.healthy {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        Cmd::Compile { bundle } => {
            let bundle = Bundle::compile(&bundle)?;
            println!("{}", serde_json::to_string_pretty(&compiled_view(&bundle))?);
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Run {
            bundle,
            recipe,
            recipes_dir,
            feature,
            realms,
            db,
            repo,
            dispatch,
            secrets_file,
        } => {
            // The map is read BEFORE anything is compiled, opened or
            // spawned: a named map that is missing or malformed ends the
            // invocation here, with no journal touched and no seat run.
            let Invocation {
                world,
                named,
                journal: db,
                ..
            } = Invocation::resolve(workspace, realms, db)?.announce();
            // A Looper-bound run pins a run-manifest/v2, whose bytes a
            // counterpart system reads and whose round-trip reconstructs
            // the manifest from six named keys. A world cannot be pinned
            // there, so a map the operator NAMED is refused rather than
            // half-honoured — and refused HERE, in the same breath as a
            // missing or malformed map, before a bundle is compiled or a
            // journal is created.
            anyhow::ensure!(
                !(named && dispatch.is_some()),
                "a run with --dispatch cannot pin the map named by --realms: the \
                 Looper-bound run-manifest/v2 lineage carries no world, and dropping \
                 the map silently would leave the run unable to say which one it \
                 believed in. Run without --dispatch, or without --realms, until a \
                 jointly agreed v2-lineage manifest version exists"
            );
            let bundle = Bundle::compile(&recipes::resolve(bundle, recipe, &recipes_dir)?)?;
            let store = Store::open(&db)?;
            let mut engine = if let Some(path) = dispatch {
                // A map merely lying in the workspace is a different
                // matter: it still names the journal this world's fleet
                // writes, so the run goes there — but it is not pinned,
                // and a dropped pin is said out loud rather than left to
                // be discovered in the manifest.
                if let Some(world) = &world {
                    eprintln!(
                        "note: {} is not pinned into this run: --dispatch writes a \
                         run-manifest/v2, which carries no world",
                        world.source.display()
                    );
                }
                let raw = std::fs::read_to_string(&path)
                    .with_context(|| format!("reading dispatch {}", path.display()))?;
                let envelope: forge_core::dispatch::DispatchEnvelopeV2 =
                    serde_json::from_str(&raw).context("parsing forge-dispatch/v2")?;
                envelope.verify(time::OffsetDateTime::now_utc(), &bundle.manifest_digest())?;
                Engine::start_with_dispatch(store, bundle, &feature, repo, envelope)?
            } else {
                Engine::start_in_world(store, bundle, &feature, repo, world)?
            };
            engine.secrets_file = secrets_file;
            eprintln!("run started: {}", engine.run_id);
            let end = engine.drive()?;
            Ok(finish(&end.state))
        }
        Cmd::Resume {
            bundle,
            recipe,
            recipes_dir,
            run,
            db,
            repo,
            secrets_file,
        } => {
            let bundle = Bundle::compile(&recipes::resolve(bundle, recipe, &recipes_dir)?)?;
            let store = Store::open(&db)?;
            let mut engine = Engine::resume(store, bundle, &run, repo)?;
            engine.secrets_file = secrets_file;
            let end = engine.drive()?;
            Ok(finish(&end.state))
        }
        Cmd::Rerun {
            run,
            bundle,
            recipe,
            recipes_dir,
            db,
            repo,
            secrets_file,
        } => {
            let store = Store::open(&db)?;
            let events = store
                .load(&run)
                .with_context(|| format!("loading source run '{run}'"))?;
            let feature = events
                .first()
                .filter(|e| e.event_type == forge_core::EventType::RunStarted)
                .and_then(|e| e.payload.get("feature"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    anyhow::anyhow!("source run '{run}' has no run/started feature to re-run")
                })?
                .to_string();
            let bundle = Bundle::compile(&recipes::resolve(bundle, recipe, &recipes_dir)?)?;
            let mut engine = Engine::start(store, bundle, &feature, repo)?;
            engine.secrets_file = secrets_file;
            eprintln!(
                "rerun of {run} as {} under {}",
                engine.run_id, engine.bundle.name
            );
            let end = engine.drive()?;
            Ok(finish(&end.state))
        }
        Cmd::Operator {
            run,
            command,
            reason,
            db,
        } => {
            anyhow::ensure!(
                command == "retry" || command == "stop",
                "operator command must be 'retry' or 'stop'"
            );
            let mut store = Store::open(&db)?;
            let operator = std::env::var("USER").unwrap_or("operator".into());
            operator_command(&mut store, &run, &command, &operator, &reason)?;
            eprintln!("recorded operator {command}; continue with: brokkr resume --run {run}");
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Inspect {
            run,
            realms,
            db,
            json,
            phase,
            seat,
        } => {
            let db = journal_of(workspace, realms, db)?;
            let store = Store::open(&db)?;
            let run = selector::resolve_run(&store, &run)?;
            let events = store.load(&run)?;
            let state = fold(&events)?;
            let view = forge_view::run_view(&events, Some(&state));
            if json {
                println!("{}", serde_json::to_string_pretty(&view)?);
                return Ok(ExitCode::SUCCESS);
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
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Watch {
            run,
            realms,
            db,
            once,
            interval_ms,
        } => {
            let db = journal_of(workspace, realms, db)?;
            // Selectors resolve once, before the loop: a prefix that is
            // unique now stays this frame's run even if another run is
            // started while we watch.
            let run = selector::resolve_run(&Store::open(&db)?, &run)?;
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
        Cmd::Replay { run, db } => {
            let store = Store::open(&db)?;
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
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Export {
            run,
            out,
            realms,
            db,
            redact,
        } => {
            let db = journal_of(workspace, realms, db)?;
            let store = Store::open(&db)?;
            let run = selector::resolve_run(&store, &run)?;
            std::fs::create_dir_all(&out)?;
            let ndjson = store.export_ndjson(&run)?;
            let manifest = store.manifest(&run)?;
            let journal_path = out.join(format!("{run}.ndjson"));
            std::fs::write(&journal_path, &ndjson)?;
            std::fs::write(
                out.join(format!("{run}.manifest.json")),
                serde_json::to_string_pretty(&manifest)?,
            )?;
            eprintln!("exported {}", journal_path.display());
            if redact {
                // A sanitized copy that could pass as verbatim evidence
                // would be a forgery, so the copy is marked twice: in
                // its filenames and in its manifest, which also names
                // the consequence — redaction breaks the recorded event
                // hashes, and hash verification applies only to the
                // verbatim export.
                //
                // Journal and manifest are scrubbed through ONE
                // redaction, in that order: the manifest states the
                // bundle a run was invoked with and, in a mapped world,
                // the map file and the realm paths it named — operator-
                // machine detail the journal beside it is published to
                // withhold. Sharing the table also keeps `[path-1]`
                // naming one path across the pair.
                let raw_manifest = serde_json::to_string(&manifest)?;
                let mut redactor = forge_store::Redactor::learn(&[&ndjson, &raw_manifest]);
                let redacted = redactor.journal(&ndjson)?;
                let redacted_path = out.join(format!("{run}.redacted.ndjson"));
                std::fs::write(&redacted_path, &redacted)?;
                let mut fields = redactor
                    .document(&manifest)
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                fields.insert("redacted".into(), json!(true));
                fields.insert(
                    "redaction".into(),
                    json!({
                        "scheme": "absolute filesystem paths — POSIX, drive-letter, \
                                   and UNC — in event payload string fields, and in \
                                   this manifest, rewritten to stable placeholders \
                                   ([path-N]), usernames to [user-N]; scheme URLs \
                                   survive as a declared bound",
                        "hashes": "recorded event hashes predate redaction and no \
                                   longer match; a pinned realms map's sha256 is \
                                   likewise the digest of the map as it was, not of \
                                   the scrubbed copy printed here; hash verification \
                                   applies only to the verbatim export",
                    }),
                );
                std::fs::write(
                    out.join(format!("{run}.redacted.manifest.json")),
                    serde_json::to_string_pretty(&Value::Object(fields))?,
                )?;
                eprintln!("exported {} (redacted)", redacted_path.display());
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::VerifyRun { file } => {
            let ndjson =
                std::fs::read_to_string(&file).context(format!("reading {}", file.display()))?;
            let state = forge_store::verify_export(&ndjson)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "chain": "verified",
                    "state": summarize(&state),
                }))?
            );
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Bridge {
            run,
            db,
            looper_url,
            token_env,
            follow,
            interval_ms,
        } => {
            let token = std::env::var(&token_env)
                .with_context(|| format!("reading producer credential from {token_env}"))?;
            anyhow::ensure!(!token.trim().is_empty(), "producer credential is empty");
            let transport = forge_bridge::HttpTransport::new(looper_url, token);
            let mut bridge = forge_bridge::Bridge::new(transport);
            let mut command_cursor = 0;
            let mut iteration = 0usize;
            loop {
                let mut store = Store::open(&db)?;
                let report = bridge.sync_once(
                    &mut store,
                    &run,
                    time::OffsetDateTime::now_utc(),
                    command_cursor,
                )?;
                command_cursor = report.last_command_cursor;
                println!(
                    "{}",
                    serde_json::to_string(&json!({
                        "run_id": run,
                        "registered": report.registered,
                        "submitted": report.submitted,
                        "replayed": report.replayed,
                        "commands": report.commands,
                        "last_forge_sequence": report.last_forge_sequence,
                    }))?
                );
                iteration += 1;
                if !follow || bridge_iteration_limit.is_some_and(|limit| iteration >= limit) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(interval_ms.max(100)));
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Realms { realms, db, json } => {
            let Invocation { world, journal, .. } =
                Invocation::resolve(workspace, realms, db)?.announce();
            let world = world.ok_or_else(|| {
                anyhow::anyhow!(
                    "no map: this workspace has no {} and none was named with --realms",
                    forge_core::realms::DEFAULT_MAP_FILE
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
                print!("{}", realms::render(&source, &journal, &rows));
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Runs { realms, db, json } => {
            let db = journal_of(workspace, realms, db)?;
            let store = Store::open(&db)?;
            let mut folded = Vec::new();
            for (run_id, feature, created_at) in store.list_runs()? {
                // A fleet listing survives one unfoldable journal: that
                // run becomes a quarantined row carrying the fold's own
                // words, and the rest of the fleet is still listed. The
                // journal is never touched — the refusal is reported.
                let folded_run = fold_or_quarantine(&store.load(&run_id)?);
                folded.push((run_id, feature, created_at, folded_run));
            }
            let entries: Vec<forge_view::RunEntry> = folded
                .iter()
                .map(
                    |(run_id, feature, created_at, folded_run)| forge_view::RunEntry {
                        run_id,
                        feature,
                        created_at,
                        state: folded_run.as_ref().ok(),
                        detail: folded_run.as_ref().err().map(String::as_str),
                    },
                )
                .collect();
            let view = forge_view::run_rows(&entries);
            if json {
                println!("{}", serde_json::to_string_pretty(&view)?);
            } else {
                print!(
                    "{}",
                    render::runs(&view, &now_rfc3339(), &render::Style::detect())
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Driver { kind, args } => {
            let kind = forge_protocol::adapters::AdapterKind::parse(&kind).ok_or_else(|| {
                anyhow::anyhow!(
                    "unknown driver '{kind}'; known: claude, lanetally, codex, dsh, exec"
                )
            })?;
            let extra = driver_extra_args(args);
            forge_protocol::adapters::serve(kind, extra)?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Compare { run_a, run_b, db } => {
            compare::compare(&run_a, &run_b, &db)?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Recipes { command } => {
            match command {
                RecipesCmd::List { dir } => recipes::list(&dir)?,
                RecipesCmd::Add { source, name, dir } => recipes::add(&source, &name, &dir)?,
                RecipesCmd::Show { name, dir } => {
                    let bundle = Bundle::compile(&recipes::resolve(None, Some(name), &dir)?)?;
                    println!("{}", serde_json::to_string_pretty(&compiled_view(&bundle))?);
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Agents { command } => {
            match command {
                AgentsCmd::List { agents_dir } => agents::list(&agents_dir)?,
                AgentsCmd::Show {
                    name,
                    agents_dir,
                    adapters_dir,
                } => agents::show(&name, &agents_dir, &adapters_dir)?,
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Muninn { command } => match command {
            MuninnCmd::Run {
                realms,
                db,
                agents_dir,
                adapters_dir,
                record,
            } => {
                let db = journal_of(workspace, realms, db)?;
                muninn::run(&db, &agents_dir, &adapters_dir, &record, &now_rfc3339())
            }
            MuninnCmd::List { record, json } => {
                muninn::list(&record, json)?;
                Ok(ExitCode::SUCCESS)
            }
        },
        Cmd::Secrets { command } => {
            use forge_protocol::secret;
            match command {
                SecretsCmd::Set { name, secrets_file } => {
                    let value = std::io::read_to_string(std::io::stdin())
                        .context("reading the secret value from stdin")?;
                    // stdin values arrive newline-terminated; the value
                    // itself must be single-line (validated in the store).
                    let value = value.strip_suffix('\n').unwrap_or(&value);
                    let value = value.strip_suffix('\r').unwrap_or(value);
                    let warning = secret::store_set(&secrets_file, &name, value)
                        .map_err(|e| anyhow::anyhow!(e))?;
                    if let Some(warning) = warning {
                        eprintln!("{warning}");
                    }
                    eprintln!("set {name} in {}", secrets_file.display());
                }
                SecretsCmd::List { secrets_file } => {
                    for name in secret::store_names(&secrets_file).map_err(anyhow::Error::msg)? {
                        println!("{name}");
                    }
                }
                SecretsCmd::Remove { name, secrets_file } => {
                    let removed =
                        secret::store_remove(&secrets_file, &name).map_err(anyhow::Error::msg)?;
                    anyhow::ensure!(
                        removed,
                        "no secret named '{name}' in {}",
                        secrets_file.display()
                    );
                    eprintln!("removed {name} from {}", secrets_file.display());
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::FakeDriver {
            script,
            state,
            model,
        } => {
            forge_protocol::fake::run_fake_driver(&script, &state, model.as_deref())?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

#[cfg(test)]
mod tests;
