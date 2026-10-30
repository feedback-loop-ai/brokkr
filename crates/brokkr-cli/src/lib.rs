//! `brokkr` — the one shipped binary (decision 0003, named by decision 0019). No UI, no
//! required services: one executable, one workspace database. The compatibility shim that
//! kept the old name working for a release is gone; one bin target enters here (ruling 9).

// A leaked fault-seam guard would leave its plan's entries unfired with
// nothing to fail the test (change `prove-transcript-reader-faults` D4.5).
// `mem::forget` of the `Drop`-implementing `Guard` fails the clippy gate
// anywhere in this crate; the D7 inspection's no-leak clause covers
// `ManuallyDrop`, `Box::leak` and a helper that swallows ownership.
#![deny(clippy::mem_forget)]

mod agents;
mod boundary;
mod broker;
mod budget_frame;
mod cli_args;
mod compare;
mod doctor;
mod dsh_key;
mod exit;
mod fleet;
mod folder;
mod hands;
mod hearths;
mod init;
mod ledger;
mod local_transcript;
mod muninn;
mod realms;
mod recipes;
mod render;
mod run_pages;
mod run_view;
mod selector;
mod tui;
mod tui_read;
mod ui;
mod verbs;

// Test seams, hidden from documentation and outside the CLI's supported
// surface: the renderers the binary serves, for 11.2's cross-surface proof,
// and the TUI frames and listing #342's CPU budget and decision 0089's
// fleet budgets read, and the bytes its `costs` and `compare` budgets do.
#[doc(hidden)]
pub use crate::budget_frame::{
    compare_for_budget, costs_for_budget, fleet_frame_for_budget, fleet_listing_for_budget,
    fleet_page_for_budget, fleet_window_for_budget, fleet_world_listing_for_budget,
    run_frame_for_budget, run_readout_for_budget, FleetScroll, FleetWindow,
};
#[doc(hidden)]
pub use crate::tui::transcript_surfaces_for_test;
#[doc(hidden)]
pub use local_transcript::read_local;
#[doc(hidden)]
pub use ui::{handle, Response};

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use brokkr_core::fold::RunState;
use brokkr_runtime::realms::{Hearth, World, WorldError};
use brokkr_runtime::{Bundle, DriveEnd, EngineError};
use brokkr_store::Store;
use brokkr_view::transcript::{TranscriptRead, Unavailable};
use clap::{ArgGroup, Parser, Subcommand};
use cli_args::*;
use exit::Exit;
use serde_json::{json, Value};

/// The command line. Its exit codes are [`exit::Exit`]'s.
#[derive(Parser)]
// `bin_name` is pinned, not inferred from argv[0]: a renamed or
// symlinked copy still prints the name this engine answers to
// (decision 0019 ruling 9).
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
    Init(InitArgs),
    /// Per-seat cost and session accounting from journal checkpoints —
    /// the LaneTally join surface (stable seat ids, journal-derived).
    Costs(CostsArgs),
    /// Render the shipper's delivery ledger from journal and repository evidence.
    Ledger(LedgerArgs),
    /// Anchor a run's journal head in refs/forge/<run> (tamper evidence),
    /// or verify the existing anchor with --check.
    Anchor(AnchorArgs),
    /// Keep-refs (decision 0028): the git objects a run's journal cites,
    /// held by refs/forge/keep/<run>/<sha> so a squash-merge, a branch
    /// delete and a gc cannot collect the evidence the journal names.
    /// Runs plant these themselves at conclusion; these verbs cover the
    /// runs that concluded before the mechanism existed, and the
    /// deliberate letting-go that is the operator's alone.
    KeepRefs {
        #[command(subcommand)]
        command: KeepRefsCmd,
    },
    /// Serve the embedded read-only surface on loopback.
    Ui(UiArgs),
    /// Explore runs interactively in the terminal: the fleet as a
    /// navigable table, one run's phase graph, seats and decision
    /// trail, and one seat's own checkpoint and session stream.
    /// Read-only like every other readout (decision 0014) — it issues
    /// no operator commands and writes nothing to the journal.
    Tui(TuiArgs),
    /// Verify tools, drivers, the workspace database, and optionally a
    /// bundle, without executing any agent.
    Doctor(DoctorArgs),
    /// Validate a bundle and print its pinned manifest and digest.
    Compile(CompileArgs),
    /// Start a new run and drive it until it parks or finishes.
    #[command(group(ArgGroup::new("delivery").args(["bundle", "recipe"])))]
    Run(RunArgs),
    /// Resume an existing run under its exact pinned bundle.
    #[command(group(ArgGroup::new("delivery").args(["bundle", "recipe"])))]
    Resume(ResumeArgs),
    /// Re-run a past run's feature as a NEW run under another bundle or
    /// recipe, so outcomes can be compared by run id. No stored linkage.
    #[command(group(ArgGroup::new("delivery").args(["bundle", "recipe"])))]
    Rerun(RerunArgs),
    /// Compare two runs' aligned outcomes: decision trails, first
    /// divergence, phases visited, per-seat costs. Read-only.
    Compare(CompareArgs),
    #[command(flatten)]
    Library(LibraryCmd),
    /// Close a stopped or parked run from its journal alone — no bundle,
    /// no recipe, no effect. `resume` compiles the exact pinned recipe
    /// and refuses on any drift, which is right for the branches that
    /// spend money but leaves a run from a moved engine with no lawful
    /// ending. This appends the operator stop conclusion and nothing
    /// else, so it needs no pinned recipe to be honest about what it
    /// wrote. It cannot retry: that re-enters the policy loop, and the
    /// policy loop needs the bundle by construction. For a run believed
    /// dead: every write is fenced, so a journal that moves beneath the
    /// conclusion — something still driving the run — refuses instead
    /// of being closed over (decision 0029). Check `brokkr runs` first.
    Conclude(ConcludeArgs),
    /// Record an operator command (retry | stop | supersede) as journal
    /// events, or acknowledge a quarantined run beside its journal.
    Operator(OperatorArgs),
    /// The dispatcher's queue (decision 0068): runs waiting to start, in
    /// the journal's own database. Add, list, judge, move, hold, release,
    /// re-pin and drop entries; each change is journaled with its reason.
    Queue {
        #[command(subcommand)]
        command: QueueCmd,
    },
    /// Explain a run: header, ruling, seats, decision trail, and the
    /// phase graph as a tree. `--phase` and `--seat` are the scoping
    /// verbs the console's clicks became; `--json` emits the view model,
    /// or with `--capability-call` the call and its verified response.
    #[command(group(ArgGroup::new("scope").args(["phase", "seat"])))]
    Inspect(InspectArgs),
    /// Read one participant's retained local transcript — Claude, Codex or
    /// DSH — through the same bounded local derivation the TUI uses. The
    /// verb never launches, retries or resumes a provider and writes
    /// nothing to the journal.
    Transcript(TranscriptArgs),
    /// The seats of a run: the seats block `inspect` renders — every
    /// seat's model with the boundary its hands stood behind beside it
    /// (decision 0046 ruling 3) — from the same view. `--json` prints
    /// that view verbatim, the bytes `inspect --json` prints.
    Seats(SeatsArgs),
    /// Watch a run live: redraw the graph, seats, last ruling and seat
    /// activity whenever the journal head moves; exit when the run
    /// reaches a terminal status. Read-only, like every other readout.
    Watch(WatchArgs),
    /// Rebuild state from the journal twice and verify determinism.
    Replay(ReplayArgs),
    /// Write the canonical NDJSON journal and pinned manifest.
    Export(ExportArgs),
    /// Back up a whole journal — one consistent snapshot taken while
    /// writers may run, plus the versioned inventory that declares its
    /// boundary and makes the backup complete.
    Backup(verbs::backup::BackupArgs),
    /// Restore a journal backup into a new location, after the inventory,
    /// the snapshot's digest, SQLite's integrity, every chain and the
    /// counts all pass. It starts nothing.
    Restore(verbs::backup::RestoreArgs),
    /// Adopt an exported run into this journal, byte-identically — the
    /// verb paired with `export`. Journals never merge; one run
    /// relocates. Nothing lands unless the whole chain verifies, the
    /// events fold, the run_id does not already exist here, and the
    /// export is not a redacted derivative.
    Import(ImportArgs),
    /// Verify an exported journal offline: chain, envelopes, fold.
    VerifyRun(VerifyRunArgs),
    /// Synchronize a Looper-bound run over the authenticated producer API.
    /// The API key is read from an environment variable and is never stored.
    Bridge(BridgeArgs),
    /// List runs in the workspace database: one clamped line per run, the
    /// fleet's sections in order and each by last activity, read a page at
    /// a time. `--json` emits the view model for scripts.
    Runs(RunsArgs),
    /// List the world (decision 0023): each realm with its path, default
    /// branch and current HEAD, and the journal the world writes.
    /// Read-only, like every other readout.
    Realms(RealmsArgs),
    #[command(flatten)]
    CrossRun(verbs::analytics::CrossRunCmd),
    /// Run a built-in forge-driver/v1 adapter (claude | lanetally | codex | dsh | exec).
    /// Bundles reference these as {brokkr} driver <kind> -- <extra args>.
    Driver(DriverArgs),
    /// The model's hands are one tool, and the tool runs in an empty root
    /// (decision 0043): serve the `workspace` tool over MCP on stdio, or
    /// run one command whole inside the same box.
    Hands {
        #[command(subcommand)]
        command: HandsCommand,
    },
    /// Measure an agent CLI for its adapter's facts, spending its bound credentials: host only, never CI.
    Probe {
        #[command(subcommand)]
        command: ProbeCmd,
    },
    /// (internal) The MCP capability broker the harness starts beside
    /// `hands` from the engine's configuration (decision 0077).
    #[command(hide = true)]
    Broker {
        #[command(subcommand)]
        command: BrokerCmd,
    },
    /// (internal) Scripted forge-driver/v1 driver for machine proof.
    #[command(hide = true)]
    FakeDriver(FakeDriverArgs),
}

#[derive(Subcommand)]
enum AgentsCmd {
    /// One line per agent — name, model chain, description. A broken
    /// definition prints a warning line and never aborts the listing.
    List {
        /// The agent library directory (default .brokkr/agents, else agents).
        #[arg(long)]
        agents_dir: Option<PathBuf>,
    },
    /// The definition as written, plus the per-chain-entry resolution
    /// the compiler would compute. An unknown name errors naming the
    /// known set.
    Show {
        /// The agent's name in the library.
        name: String,
        /// The agent library directory (default .brokkr/agents, else agents).
        #[arg(long)]
        agents_dir: Option<PathBuf>,
        /// The adapter library the chain's models are resolved against (default .brokkr/adapters, else adapters).
        #[arg(long)]
        adapters_dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum MuninnCmd {
    /// Derive the fleet dossier, ask one bounded seat for proposals, and
    /// record them. Nothing is executed: a proposal becomes an action
    /// only when the operator issues the command themselves.
    Run {
        /// The world's map — the journal it names is the fleet this
        /// reading covers (default .brokkr/realms.json, else ./realms.json, when present).
        #[arg(long)]
        realms: Option<PathBuf>,
        /// The workspace journal. Outranks the map's journal; without
        /// either, .brokkr/state/forge.db, or .forge/forge.db in a library.
        #[arg(long)]
        db: Option<PathBuf>,
        /// The agent library Muninn's own seat is hired from (default .brokkr/agents, else agents).
        #[arg(long)]
        agents_dir: Option<PathBuf>,
        /// The adapter library that seat's model is resolved against (default .brokkr/adapters, else adapters).
        #[arg(long)]
        adapters_dir: Option<PathBuf>,
        /// The append-only file each proposal is recorded in (default .brokkr/state/muninn.ndjson, or a library's .forge/muninn.ndjson).
        #[arg(long)]
        record: Option<PathBuf>,
    },
    /// Read the record back: every proposal, with the run ids and
    /// sequence numbers it cited.
    List {
        /// The record file to read (default .brokkr/state/muninn.ndjson, or a library's .forge/muninn.ndjson).
        #[arg(long)]
        record: Option<PathBuf>,
        /// Emit the recorded entries verbatim — this is what scripts read.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum KeepRefsCmd {
    /// Plant a keep-ref for every object the run's journal cites.
    /// Idempotent: replanting moves nothing.
    Plant {
        /// Full run id, a unique run-id prefix, or `latest`.
        #[arg(long)]
        run: String,
        #[command(flatten)]
        journal: JournalArgs,
        /// The git repository the keep-refs are planted in.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
    /// Which runs hold which exhibits — one `for-each-ref`, no journal
    /// needed. `--run` narrows the listing to one run.
    List {
        /// A run id; with the workspace journal there, also a unique
        /// prefix or `latest`, and without it the id is taken literally
        /// and `latest` is refused. Omitted, every run holding keep-refs
        /// in this repository.
        #[arg(long)]
        run: Option<String>,
        #[command(flatten)]
        journal: JournalArgs,
        /// The git repository whose keep-refs are listed.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
    /// Let one run's exhibits go: remove refs/forge/keep/<run>/*. The
    /// operator's decision alone — nothing in the engine ever deletes a
    /// keep-ref, and the objects are then as mortal as gc leaves them.
    Delete {
        /// A run id; with the workspace journal there, also a unique
        /// prefix or `latest`, and without it the id is taken literally
        /// and `latest` is refused.
        #[arg(long)]
        run: String,
        #[command(flatten)]
        journal: JournalArgs,
        /// The git repository the keep-refs are removed from.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
}

#[derive(Subcommand)]
enum RecipesCmd {
    /// List recipes under --dir, then those on the personal shelf, each
    /// with its origin, project or personal; broken ones print a warning
    /// line, never abort the listing.
    List {
        /// The recipe library directory (default the found .brokkr/recipes,
        /// listing nothing while it is absent, else recipes).
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Install a recipe from a local path or a git URL into the shelf at
    /// the repository's root, .brokkr/recipes/<name> where the folder is,
    /// else recipes/<name>, with the agents and charters it names that
    /// the agent library lacks.
    Add {
        /// A local bundle directory or a git URL.
        source: String,
        /// The name the recipe is installed under.
        #[arg(long)]
        name: String,
        /// With a git URL, the bundle's directory inside the clone, a
        /// relative path of plain names (`recipes/fast`), walked without
        /// following a link; without it, the clone's root or its one
        /// subdirectory holding a bundle.json. The bases its recipe
        /// extends come from beside it.
        #[arg(long)]
        path: Option<String>,
        /// Install onto the personal shelf instead,
        /// $XDG_CONFIG_HOME/brokkr/recipes/<name>, else
        /// $HOME/.config/brokkr/recipes/<name>, beside host.json and in no
        /// repository; it copies no agent or charter.
        #[arg(long)]
        personal: bool,
    },
    /// Print one recipe's RESOLVED bundle and, when it extends another,
    /// the composition chain it was resolved from (decision 0017).
    Show {
        /// The recipe's name, resolved to <dir>/<name>, else to the
        /// personal shelf's; a name on both refuses.
        name: String,
        /// The recipe library directory (default the found .brokkr/recipes,
        /// else recipes).
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

/// Every crossing the WORLD draws, per realm: what each realm publishes,
/// with the path it was read from and the digest of the bytes that were
/// there, and what each realm consumes, with the realm that publishes it
/// and the pin the map carries.
///
/// Printed beside the compiled bundle and never written INTO it. A
/// crossing is the realm's, exactly as the boundary is (decision 0046
/// ruling 1 prints one under each hands site and keeps it out of no
/// manifest it does not own; decision 0057 ruling 1), and what a run
/// STOOD ON is recorded at run start on the run manifest
/// (run-manifest/v10). Adding a `crossings` key to `bundle.manifest`
/// would move every affected bundle's `manifest_digest` for a fact no
/// bundle owns.
///
/// Omitted entirely when no realm draws one, so a world without
/// crossings prints exactly what it printed before this existed.
fn crossings_view(world: &World) -> Option<Value> {
    let mut realms = serde_json::Map::new();
    for realm in &world.map.realms {
        if realm.published().is_empty() && realm.consumed().is_empty() {
            continue;
        }
        let published = realm
            .published()
            .iter()
            .map(|crossing| {
                let mut row = json!({"name": crossing.name, "path": crossing.path});
                // Always resolved here: `compile` reads its world through
                // `World::discover`, which refuses a published file it
                // could not read before this view is built.
                if let Some(resolved) = world.crossing(&realm.name, &crossing.name) {
                    row["source"] = json!(resolved.source);
                    row["sha256"] = json!(resolved.sha256);
                }
                row
            })
            .collect::<Vec<_>>();
        let consumed = realm
            .consumed()
            .iter()
            .map(|crossing| {
                json!({
                    "name": crossing.name,
                    "realm": crossing.realm,
                    "sha256": crossing.sha256,
                })
            })
            .collect::<Vec<_>>();
        realms.insert(
            realm.name.clone(),
            json!({"publishes": published, "consumes": consumed}),
        );
    }
    (!realms.is_empty()).then_some(Value::Object(realms))
}

/// What a compiled bundle looks like to an operator. `brokkr compile` and
/// `brokkr recipes show` print this and nothing else, from here, so the
/// two surfaces can never drift. `composed_from` is omitted entirely
/// when nothing was composed, so a plain bundle's output is unchanged —
/// and so is `crossings`, which belongs to the world a verb discovered
/// and not to the bundle, which is why `recipes show` passes none.
fn compiled_view(bundle: &Bundle, world: Option<&World>) -> Value {
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
    // Each layer's origin, the chain a run's start records (#697 ruling 5).
    if let Some(layers) = bundle.layers() {
        view["layers"] = json!(layers);
    }
    if let Some(crossings) = world.and_then(crossings_view) {
        view["crossings"] = crossings;
    }
    view
}

fn summarize(state: &RunState) -> Value {
    json!(brokkr_view::summary(state))
}

/// Drive a started run to its ending. The start first reaps the scratch
/// trees of hands servers whose owners died and says each on stderr,
/// through the same safe renderer as every other operator-facing line —
/// a hostile tree name must not reach the terminal as written (#468) —
/// journaling nothing (#415). Then the conclusion's anchor and keep-ref
/// gaps on stderr, and the summary `finish` prints. `drive` drives it:
/// alone, or beside the run view (#508).
fn drive_to_end(
    engine: &mut brokkr_runtime::Engine,
    drive: impl FnOnce(&mut brokkr_runtime::Engine) -> Result<DriveEnd, EngineError>,
) -> Result<ExitCode> {
    let reaped = brokkr_protocol::hands::reap_dead_sessions();
    eprint!(
        "{}",
        reaped.lines_with(|tree| render::Safe::new(tree).as_str().to_owned())
    );
    let end = drive(engine)?;
    for gap in &end.gaps {
        eprintln!("{gap}");
    }
    Ok(finish(&end.state))
}

fn finish(state: &RunState) -> ExitCode {
    println!(
        "{}",
        serde_json::to_string_pretty(&summarize(state)).unwrap()
    );
    Exit::of_status(&state.run.status()).into()
}

/// The one clock read that keeps the derivation pure: `brokkr-view` has
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

/// A transient store error (SQLITE_BUSY while a `brokkr run` holds the
/// write lock) is a frame that says so, not an exit. A persistent one
/// exits nonzero.
pub(crate) const WATCH_TRANSIENT_FRAMES: usize = 5;

/// What a verb does to the journal it names, declared at the verb rather
/// than remembered by it (#375).
pub(crate) enum Access {
    /// A look: the journal must already exist and is opened
    /// `SQLITE_OPEN_READ_ONLY` — no file, WAL, migration or guard repair.
    Read,
    /// A writer: the journal is created if absent and brought to the
    /// current schema, as [`Store::open`] does.
    Append,
}

/// Open the journal a verb names, the way its [`Access`] declares.
pub(crate) fn open_journal(db: &std::path::Path, access: Access) -> Result<Store> {
    match access {
        Access::Read => {
            anyhow::ensure!(
                db.is_file(),
                "journal does not exist: {}; a read never creates one",
                db.display()
            );
            Ok(Store::open_read_only(db)?)
        }
        Access::Append => Ok(Store::open(db)?),
    }
}

/// Poll the journal head and redraw when it moves, comparing **both**
/// seq and hash: a rewritten journal at equal seq is the tamper case
/// `anchor` exists for, and `watch` should redraw rather than sit blind.
#[expect(clippy::too_many_arguments, reason = "baseline 2026-09, #288")]
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
        // Every poll is a look at a journal a live `brokkr run` may be
        // writing: it never takes the write path (#375).
        let head =
            open_journal(db, Access::Read).and_then(|store| Ok((store.head_hash(run)?, store)));
        match head {
            Ok((head, store)) => {
                failures = 0;
                if last.as_ref() != Some(&head) {
                    last = Some(head);
                    let read = run_pages::viewed(&store, run, run_pages::Shows::Frame)?;
                    let (view, state) = (read.view, read.folded.ok());
                    let frame = render::inspect(&view, None, false, style);
                    write_frame(out, &frame, is_tty, clock)?;
                    // A park admits no further events until a human acts,
                    // so "keep watching" is an unbounded CI hang. The park
                    // reason printed first is the frame's own header.
                    if let Some(exit) =
                        Exit::of_settled(state.map(|state| state.run.status()).as_ref())
                    {
                        return Ok(exit.into());
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
    Ok(Exit::Running.into())
}

/// `--run` for the reading and releasing verbs. Keep-refs outlive
/// journals by design — the exhibits of a run whose database has moved
/// on are still listable, and still the operator's to release — so the
/// selector goes through decision 0015's one resolver when there IS a
/// workspace database and is taken literally when there is not, rather
/// than refusing a run the refs themselves can name.
///
/// `latest` is the exception, and is refused rather than taken
/// literally: it is not a name, it is a question only the run table can
/// answer, and no repository holds a run called `latest`. Answering it
/// literally would report a released or listed nothing as if it were an
/// answer — the quiet outcome these verbs exist to prevent.
fn keep_ref_run(workspace: &std::path::Path, journal: JournalArgs, run: &str) -> Result<String> {
    let db = journal.journal(workspace)?;
    if db.is_file() {
        return selector::resolve_run(&Store::open(&db)?, run);
    }
    anyhow::ensure!(
        run != selector::LATEST,
        "'{}' is a question for the workspace database, and {} is not there; \
         name the run itself — `brokkr keep-refs list` prints the runs this \
         repository holds exhibits for",
        selector::LATEST,
        db.display()
    );
    Ok(run.to_string())
}

/// The keep-ref verbs. Planting reads the journal (so it resolves
/// strictly, through the store it must open anyway); listing and
/// deleting read only the repository.
fn keep_refs(workspace: &std::path::Path, command: KeepRefsCmd) -> Result<ExitCode> {
    match command {
        KeepRefsCmd::Plant { run, journal, repo } => {
            let store = Store::open(&journal.journal(workspace)?)?;
            let run = selector::resolve_run(&store, &run)?;
            let planted = brokkr_runtime::plant_keep_refs(&store, &repo, &run)?;
            eprintln!(
                "kept {} exhibit(s) for {run} in {}/{run}/",
                planted.kept.len(),
                brokkr_runtime::keep_refs::KEEP_PREFIX,
            );
            if !planted.absent.is_empty() {
                eprintln!(
                    "keep-ref gap: {} cited object(s) are not in this repository: {}",
                    planted.absent.len(),
                    planted.absent.join(", "),
                );
            }
        }
        KeepRefsCmd::List { run, journal, repo } => {
            let mut held = brokkr_runtime::list_keep_refs(&repo)?;
            if let Some(run) = run {
                let run = keep_ref_run(workspace, journal, &run)?;
                held.retain(|holder, _| *holder == run);
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({ "keep": held }))?
            );
        }
        KeepRefsCmd::Delete { run, journal, repo } => {
            let run = keep_ref_run(workspace, journal, &run)?;
            let removed = brokkr_runtime::delete_keep_refs(&repo, &run)?;
            eprintln!("released {removed} exhibit(s) for {run}");
        }
    }
    Ok(Exit::Completed.into())
}

pub use hands::HandsCommand;

#[cfg(test)]
use brokkr_runtime::boundary::refuse_unboxable;

/// The payload the LONG-STANDING adapters (claude, lanetally, codex,
/// exec) receive: everything the operator's command line carried past
/// the boundary, less one leading `--`.
///
/// Clap consumes the outer separator itself, so a `--` still standing at
/// the head of the collected arguments is NOT that boundary — it is a
/// second terminator the seat wrote. Dropping it is therefore a
/// normalisation, not a boundary; it is kept here only because these four
/// adapters have shipped with it, and their conformance suites measure
/// that behaviour. Searching the whole vector instead cut the payload at
/// an INTERIOR terminator and handed the adapter a shorter argv than was
/// typed: `-- --effort --model p/m high --` arrived EMPTY and launched
/// with no pin, and `-- --model p/m -- --model second` arrived as the
/// second pair alone. Nothing but that one head token is dropped.
///
/// The dsh arm does not come through here at all — see `driver_payload`.
fn driver_extra_args(args: Vec<String>) -> Vec<String> {
    match args.first().map(String::as_str) {
        Some("--") => args[1..].to_vec(),
        _ => args,
    }
}

/// The payload one named driver's own admission rule judges.
///
/// DSH opts OUT of the head normalisation above: its admission rule reads
/// EVERY token exactly as the operator's command line carried it — leading,
/// interior and trailing alike — because a residual `--` is an argument the
/// seat wrote and the adapter refuses it, as the option terminator, before
/// any route read, version probe or launch (proposed decision 0056 rulings
/// 4 and 6). While the head token was deleted here, `brokkr driver dsh --
/// --` and `brokkr driver dsh -- -- --model p/m` reached the adapter with
/// that terminator gone and were ADMITTED — they launched a child, staged
/// an overlay and allocated a retained root. An adapter can only refuse
/// what it receives. The other four keep their shipped behaviour, measured
/// by their own tests, rather than moving under a dsh repair.
fn driver_payload(kind: brokkr_protocol::adapters::AdapterKind, args: Vec<String>) -> Vec<String> {
    match kind {
        brokkr_protocol::adapters::AdapterKind::Dsh => args,
        _ => driver_extra_args(args),
    }
}

/// Did this error come from a peer holding the journal's lock?
///
/// Asked of the whole chain and answered by the store's own typed
/// predicate — never by matching error text. It asks all four shapes: a
/// `StoreError` straight out of a store call, and one an `EngineError`, an
/// `ImportError` or a `LaunchError` carries, asked separately because
/// their store variants are `transparent`, which puts the store error's
/// own source in the chain and the store error itself nowhere in it.
///
/// A contention that reached here wrote nothing, so there is no
/// half-done work to describe.
fn contention(error: &anyhow::Error) -> Option<&brokkr_store::StoreError> {
    error.chain().find_map(|link| {
        let store = link.downcast_ref::<brokkr_store::StoreError>().or_else(|| {
            match link.downcast_ref::<brokkr_store::ImportError>() {
                Some(brokkr_store::ImportError::Store(store)) => Some(store),
                _ => None,
            }
        });
        store
            .filter(|store| store.is_contention())
            .or_else(|| {
                link.downcast_ref::<brokkr_runtime::EngineError>()
                    .and_then(brokkr_runtime::EngineError::contention)
            })
            .or_else(|| {
                link.downcast_ref::<brokkr_runtime::launch::LaunchError>()
                    .and_then(brokkr_runtime::launch::LaunchError::contention)
            })
    })
}

/// How a failed command leaves: one line for an operator and one exit
/// code, chosen by what the error IS.
///
/// The bug this closes ended here. A `database is locked` used to arrive
/// as an anonymous error, print `error: {e:#}` and exit 1 —
/// indistinguishable from any other defect — and an engine that had
/// journaled nineteen good events simply vanished. Contention says its
/// own name now, says that nothing was lost, and carries its own code.
fn report(error: &anyhow::Error) -> ExitCode {
    report_to(error, &mut std::io::stderr().lock())
}

/// `report`, writing its line to `stderr`: the seam a test reads to pin
/// what the binary prints.
fn report_to(error: &anyhow::Error, stderr: &mut impl std::io::Write) -> ExitCode {
    // A stderr that cannot be written leaves the exit code to say it.
    match contention(error) {
        Some(store) => {
            let _ = writeln!(
                stderr,
                "contended: {store}\nA peer is writing this journal. Nothing was \
                 written and nothing was lost — resume when it is done."
            );
            Exit::Contended.into()
        }
        None => {
            let _ = writeln!(stderr, "{}", failure_line(error));
            Exit::Failed.into()
        }
    }
}

/// What an uncontended failure prints on stderr.
fn failure_line(error: &anyhow::Error) -> String {
    format!("error: {}", safe_lines(&format!("{error:#}")))
}

/// Sanitize an error display line by line, keeping the chain's own line
/// breaks: every rendered line loses control and directional characters
/// while the recorded error strings stay untouched.
pub(crate) fn safe_lines(text: &str) -> String {
    text.split('\n')
        .map(|line| render::Safe::new(line).as_str().to_string())
        .collect::<Vec<String>>()
        .join("\n")
}

/// The binary's entry: one parse, one command set, one set of exit
/// codes. There is one bin now — decision 0019 ruling 9's shim is gone.
///
/// The dsh sandbox runner is dispatched before clap: dsh invokes it with
/// bubblewrap's own argv, including the bare `--` that separates the
/// profile from the command, and clap cannot both parse a verb and
/// preserve that separator. It is not a user-facing command and never
/// appears in `--help`.
pub fn main() -> ExitCode {
    if let Some(args) = dsh_sandbox_runner_args() {
        return dsh_sandbox_runner(args);
    }
    match Cli::try_parse().map(run) {
        Ok(Ok(code)) => code,
        Ok(Err(e)) => report(&e),
        Err(usage) => Exit::of_parse(&usage).into(),
    }
}

fn dsh_sandbox_runner_args() -> Option<Vec<String>> {
    let mut argv = std::env::args();
    let _program = argv.next();
    argv.next()
        .filter(|verb| verb == brokkr_protocol::dsh_sandbox::RUNNER_VERB)
        .map(|_| argv.collect())
}

/// Run the dsh sandbox runner whole: it builds bubblewrap's argv from the
/// profile dsh handed it and `exec`s it, so the runner process is
/// replaced and the seat sees one process. Every refusal prints the
/// prefix dsh classifies as a runner failure, never a denied command.
fn dsh_sandbox_runner(args: Vec<String>) -> ExitCode {
    let signature = brokkr_protocol::dsh_sandbox::RUNNER_FAILURE_SIGNATURE;
    match brokkr_protocol::dsh_sandbox::runner_argv(&args) {
        Ok(argv) => exec_bwrap(&argv, signature),
        Err(problem) => {
            eprintln!("{signature}{problem}");
            Exit::RunnerFailed.into()
        }
    }
}

fn exec_bwrap(argv: &[String], signature: &str) -> ExitCode {
    use std::os::unix::process::CommandExt;
    let (program, rest) = (&argv[0], &argv[1..]);
    let error = std::process::Command::new(program).args(rest).exec();
    eprintln!("{signature}{error}");
    Exit::RunnerFailed.into()
}

/// The local Claude projects root a legacy flat id is synthesized
/// against, or `None` when there is no usable `HOME`. The shell resolves
/// it once per read and hands it to [`local_transcript::read_local`],
/// which reads no environment (#351).
pub(crate) fn local_projects_home() -> Option<String> {
    let home = std::env::var_os("HOME")?;
    std::path::Path::new(&home)
        .join(".claude")
        .join("projects")
        .to_str()
        .map(str::to_string)
}

/// Which of the hearths that answered `latest` actually holds it: the
/// newest run in the WORLD, by the recorded stamp — the same rule
/// `selector::resolve` applies inside one journal, applied across them.
/// A tie leaves the earlier hearth in place, because map order is the
/// world's own order and a tie broken by iteration order would answer
/// differently on maps that say the same thing.
///
/// Pure, over `(hearth, run id, created_at)`, so the rule is testable
/// without three journals whose stamps happen to differ.
fn newest_answer(answered: Vec<(usize, String, String)>) -> Option<(usize, String)> {
    answered
        .into_iter()
        .reduce(|best, next| match next.2 > best.2 {
            true => next,
            false => best,
        })
        .map(|(index, id, _)| (index, id))
}

/// Where a `--run` selector resolves in a world of many hearths: the
/// hearth that holds it, and the id it resolved to. A run id lives in
/// exactly ONE journal (decision 0026 ruling 3), so this is a lookup
/// across hearths and never a merge.
///
/// Every hearth is asked, and a selector that answers in SEVERAL of them
/// is refused by name rather than silently opening the first — the same
/// rule `selector::resolve` applies inside one journal, applied across
/// them: picking one for the operator would be a guess about which run
/// they meant, and a selector that is strict in a one-hearth world and
/// loose in a many-hearth one is a trap. `latest` is the exception the
/// selector already defines: it means the NEWEST run, so where several
/// hearths hold runs the recorded stamp decides between them, and the
/// earliest hearth in map order wins a tie.
///
/// A hearth whose journal is not on disk yet is not consulted: there is
/// nothing to read, and a read never creates one. When NO hearth has one, the selector passes through
/// unresolved and `tui::start` does the refusing — a read must not create
/// the database it came to read.
///
/// Every hearth is walked READ-ONLY, the same way [`tui_views`] reads
/// it: a peer realm is a journal the operator did not name, and a lookup
/// passing through one must not migrate it (ruling 5); a SOLE hearth is
/// the journal the operator did name, and a look is a look there too
/// (#375) — no WAL, migration or guard repair.
fn resolve_in_hearths(hearths: &[Hearth], run: String) -> Result<(usize, String)> {
    let answers = hearths::ask(hearths, &run);
    if run == selector::LATEST {
        return hearths::latest(answers, run);
    }
    hearths::named(hearths, answers, run)
}

/// `brokkr tui`'s impure entry: the console, watching nothing but its
/// keys, through the session every run view opens (#508).
fn run_tui(hearths: Vec<Hearth>, run: Option<String>, tab: usize) -> Result<tui::Closed> {
    run_view::console(hearths, run, tab, tui::Watched::Console)
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

/// An exact participant key wins; otherwise an exact label selects only
/// when unique. Prefixes and fuzzy labels never match, and an ambiguous
/// label names every matching key so the operator can choose.
fn select_transcript_participant<'a>(
    participants: &'a [brokkr_view::Participant],
    seat: &str,
) -> Result<&'a brokkr_view::Participant> {
    if let Some(exact) = participants.iter().find(|part| part.key == seat) {
        return Ok(exact);
    }
    let matched: Vec<&brokkr_view::Participant> = participants
        .iter()
        .filter(|part| part.label == seat)
        .collect();
    match matched.as_slice() {
        [only] => Ok(only),
        [] => Err(anyhow::anyhow!(
            "no participant key or label {} in this run",
            render::Safe::new(seat).as_str()
        )),
        many => {
            // Every candidate key is sanitized, not only the requested
            // label: a provider-recorded key can carry terminal controls.
            let keys: Vec<String> = many
                .iter()
                .map(|part| render::Safe::new(&part.key).as_str().to_string())
                .collect();
            Err(anyhow::anyhow!(
                "participant label {} is ambiguous; choose one of: {}",
                render::Safe::new(seat).as_str(),
                keys.join(", ")
            ))
        }
    }
}

/// Apply `--turn` only after every cap, diagnostic and refusal: a
/// retained index keeps its original turn and the whole read's metadata;
/// an index beyond the prefix becomes `turn-not-retained` unless the read
/// already refused for a stronger reason.
fn select_transcript_turn(read: TranscriptRead, turn: Option<u64>) -> TranscriptRead {
    let Some(index) = turn else {
        return read;
    };
    if !read.is_readable() {
        return read;
    }
    // Compare and index in the recorded width: a u64-to-usize narrowing
    // could wrap on a 32-bit target and select an unrelated retained turn.
    if index >= 1 && index <= read.turns.len() as u64 {
        let position = (index - 1) as usize;
        let mut selected = read;
        selected.turns = vec![selected.turns[position].clone()];
        return selected;
    }
    let explanation = if read.truncated {
        "the requested turn is outside the retained prefix; the bounded read does not establish whether it exists later"
    } else {
        "the requested turn is beyond the retained projection"
    };
    TranscriptRead::refused(
        read.reference.clone(),
        read.legacy,
        Unavailable::TurnNotRetained,
        explanation,
        read.path.clone(),
        read.truncated,
        read.skipped_lines,
        read.unrecognized_records,
        read.full_session.clone(),
    )
}

/// The complete `brokkr.transcript/v1` document, every member present
/// even when null or empty, serialized from the shared result alone: the
/// bytes `brokkr transcript --json` prints and the browser's transcript
/// route serves (#352).
fn transcript_document(run: &str, seat: &str, read: &TranscriptRead, turn: Option<u64>) -> String {
    let document = json!({
        "schema": brokkr_view::transcript::TRANSCRIPT_SCHEMA,
        "run_id": run,
        "seat": seat,
        "transcript": &read.reference,
        "legacy": read.legacy,
        "path": &read.path,
        "turn": turn,
        "turns": &read.turns,
        "truncated": read.truncated,
        "skipped_lines": read.skipped_lines,
        "unrecognized_records": read.unrecognized_records,
        "notices": &read.notices,
        "unavailable": read.unavailable.map(Unavailable::as_str),
        "full_session": &read.full_session,
    });
    serde_json::to_string_pretty(&document).expect("a JSON value serializes")
}

/// `brokkr transcript`: resolve the run read-only, select exactly one
/// participant, run the shared local derivation and render it. Nothing
/// is launched, written or resumed.
fn transcript_command(
    workspace: &std::path::Path,
    realms: Option<PathBuf>,
    db: Option<PathBuf>,
    run: String,
    seat: String,
    turn: Option<u64>,
    json: bool,
) -> Result<ExitCode> {
    let hearths = hearths_of(workspace, realms, db)?;
    // Consult every distinct existing hearth read-only and apply the one
    // established exact/prefix/`latest` rule; a read never opens a journal
    // read-write, even when `--db` names a sole hearth.
    let (hearth, run) = resolve_in_hearths(&hearths, run)?;
    let store = Store::open_read_only(&hearths[hearth].journal)?;
    let participants = run_pages::participants(&store, &run)??;
    let participant = select_transcript_participant(&participants, &seat)?;
    // Only the subject's reference, provenance and legacy id reach the
    // reader; the hearth index keeps it the console's shape.
    let subject = brokkr_view::Subject::of(hearth, None, run.clone(), participant);
    let read = local_transcript::read_local(&subject, local_projects_home().as_deref());
    let read = local_transcript::mask_secrets(
        read,
        &local_transcript::store_beside(&hearths[hearth].journal),
    );
    let read = select_transcript_turn(read, turn);
    if json {
        println!(
            "{}",
            transcript_document(&run, &participant.key, &read, turn)
        );
    }
    match read.unavailable {
        None => {
            if !json {
                print!(
                    "{}",
                    render::transcript(
                        &run,
                        &participant.key,
                        &read,
                        turn,
                        &render::Style::detect()
                    )
                );
            }
            Ok(Exit::Completed.into())
        }
        Some(reason) => {
            // The refusal explanation and notices reach stderr in both
            // modes: JSON still carries the document on stdout, and the
            // operator still needs the sanitized reason.
            let explanation = read.explanation.clone().unwrap_or_default();
            let mut line = format!(
                "transcript unavailable: {}: {}",
                reason.as_str(),
                render::Safe::new(&explanation).as_str()
            );
            for notice in &read.notices {
                line.push_str("; ");
                line.push_str(render::Safe::new(notice).as_str());
            }
            eprintln!("{line}");
            Ok(Exit::Failed.into())
        }
    }
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
    /// What an ambient map is doing without being asked to. Ruling 3
    /// wants a map found to be adopted; it does not want the adoption
    /// silent, so EVERY map nobody typed is said out loud — once, on
    /// stderr, before anything opens — whether it moved the journal or
    /// merely decides the realm paths and the fact keys.
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

/// How an invocation opens the map it found: refusing a moved crossing
/// ([`World::discover`]) or reporting one ([`World::inspect`]).
type OpenWorld =
    fn(&std::path::Path, Option<&std::path::Path>) -> Result<Option<World>, WorldError>;

impl Invocation {
    /// The world as every verb that STARTS or CONTINUES a run reads it: a
    /// crossing that has moved refuses here, before a store is opened
    /// (decision 0057, `b9b2ee6`).
    fn resolve(
        workspace: &std::path::Path,
        realms: Option<PathBuf>,
        db: Option<PathBuf>,
    ) -> Result<Invocation> {
        Invocation::open(workspace, realms, db, World::discover)
    }

    /// The world as a READ surface reads it: a crossing that has moved is
    /// a line in the readout rather than the end of it. `brokkr realms`
    /// and `brokkr muninn run` read this way for the same reason `brokkr
    /// doctor` does — a surface that only looks refuses nothing, and
    /// refusing here would blank the readout in exactly the world an
    /// operator opened it to see (decision 0046's Addendum; decision 0023
    /// ruling 6, which makes `realms` a read surface with no writes). The
    /// verbs that take the map only for its journal read it this way too
    /// ([`JournalArgs::journal`]).
    fn inspect(
        workspace: &std::path::Path,
        realms: Option<PathBuf>,
        db: Option<PathBuf>,
    ) -> Result<Invocation> {
        Invocation::open(workspace, realms, db, World::inspect)
    }

    /// Decision 0083 first: the folder is discovered and an old layout
    /// refused before any map or store opens, unless `--db` names the
    /// journal; the folder's map and journal are then the defaults.
    fn open(
        workspace: &std::path::Path,
        realms: Option<PathBuf>,
        db: Option<PathBuf>,
        open: OpenWorld,
    ) -> Result<Invocation> {
        let (named, overridden) = (realms.is_some(), db.is_some());
        let layout = folder::Layout::discover(workspace)?;
        if !overridden {
            layout.refuse_old()?;
        }
        let realms = realms.map(|path| layout.typed(path));
        let world = open(&layout.workspace(), realms.as_deref())?;
        let mapped = world.as_ref().map(World::journal);
        // The default is asked only where `--db` names nothing, so it is
        // never the `--db` escape's.
        let default = || layout.state_file(false, brokkr_runtime::places::JOURNAL);
        let journal = db.or_else(|| mapped.clone()).map_or_else(default, Ok)?;
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
                if overridden || default().is_ok_and(|default| same_journal(&mapped, &default)) {
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

/// `brokkr operator supersede` (decision 0047 ruling 1): one annotation
/// on a run that has already finished, saying which of its residual
/// findings another run closed. Every citation is verified before
/// anything is written and a refusal writes nothing (ruling 2), so this
/// reports a refusal as an error rather than as a journaled rejection.
///
/// The world is opened the way the fleet verbs open it — `--realms`, an
/// ambient `./realms.json`, `--db` outranking both — because `by` may
/// name a run in another hearth, and a citation into a journal this
/// process never opened is one nobody can follow back.
fn supersede(
    workspace: &std::path::Path,
    run: &str,
    reason: &str,
    findings: &[u64],
    by: (Option<String>, Option<u64>, Option<String>),
    realms: Option<PathBuf>,
    db: Option<PathBuf>,
) -> Result<ExitCode> {
    let (by_run, by_seq, by_realm) = by;
    anyhow::ensure!(
        !findings.is_empty(),
        "supersede needs --findings: the sequence numbers of the rulings whose \
         residual findings this closes"
    );
    let (Some(by_run), Some(by_seq)) = (by_run, by_seq) else {
        anyhow::bail!(
            "supersede needs --by-run and --by-seq: the run that closed the \
             findings, and the ruling in it that did"
        );
    };
    let invocation = Invocation::resolve(workspace, realms, db)?.announce();
    let cited_journal = match &by_realm {
        None => invocation.journal.clone(),
        Some(realm) => invocation
            .world
            .as_ref()
            .map(World::hearths)
            .unwrap_or_default()
            .into_iter()
            .find(|hearth| hearth.realms.iter().any(|known| known == realm))
            .map(|hearth| hearth.journal)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "--by-realm '{realm}' names no realm in this world; --realms \
                     names the map that would"
                )
            })?,
    };
    let mut store = Store::open(&invocation.journal)?;
    // The cited journal is opened READ-ONLY: a citation is checked
    // against a world this command only looks at (decision 0026 ruling
    // 5), and the one journal it writes to is the annotated run's.
    let cited = Store::open_read_only(&cited_journal)?;
    let run = &selector::resolve_run(&store, run)?;
    let operator = std::env::var("USER").unwrap_or("operator".into());
    let written = brokkr_runtime::operator_supersede(
        &mut store,
        &cited,
        run,
        &brokkr_runtime::Supersede {
            findings,
            by_realm: by_realm.as_deref(),
            by_run: &by_run,
            by_seq,
            reason,
            operator: &operator,
        },
    )?;
    eprintln!(
        "recorded supersede at seq {} on {run}; the findings stay in the journal, \
         marked, and the run's status is untouched. Read it with: brokkr inspect \
         --run {run}",
        written.seq
    );
    Ok(Exit::Completed.into())
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

impl JournalArgs {
    /// The journal a verb that takes the map ONLY for its journal opens
    /// (#374) — resolved on [`Invocation`]'s three rules, so `resume`,
    /// `conclude` and `operator stop` address the journal `run` wrote.
    ///
    /// Read the way [`Invocation::inspect`] reads it: none of these verbs
    /// pins the world it read, so a crossing that has moved is not theirs
    /// to refuse — and a recovery verb that refused there would fail in
    /// exactly the world it is needed in. `resume` still refuses one
    /// through its own pinned world's fence, before any seat spawns.
    fn journal(self, workspace: &std::path::Path) -> Result<PathBuf> {
        Ok(Invocation::inspect(workspace, self.realms, self.db)?
            .announce()
            .journal)
    }
}

/// The one journal `brokkr ui` serves: the fleet `brokkr tui` opens, read
/// the way `tui` reads it ([`hearths_of`]), so the two surfaces never show
/// different fleets (#374). The web view serves a single journal, so a
/// world whose realms name several hearths is refused, naming them, rather
/// than served one of them as though it were the whole fleet.
fn ui_journal(workspace: &std::path::Path, journal: JournalArgs) -> Result<PathBuf> {
    let mut hearths = hearths_of(workspace, journal.realms, journal.db)?;
    anyhow::ensure!(
        hearths.len() == 1,
        "brokkr ui serves one journal, and this world's realms name {}: {}; --db names \
         the one to serve (brokkr tui and brokkr runs read them all)",
        hearths.len(),
        hearths
            .iter()
            .map(|hearth| hearth.journal.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(hearths.remove(0).journal)
}

/// The journals a FLEET read opens (decision 0026 rulings 2 and 3): one
/// per DISTINCT hearth the world's realms name, in map order.
///
/// Two rules on top of [`Invocation::resolve`]'s three, and no third.
/// `--db` names one journal and outranks the map here exactly as it does
/// for a single-run verb — an operator who typed one journal reads one
/// journal. And a workspace with no map has the one journal it always
/// had. Either way the answer is a single hearth, which every surface
/// below renders exactly as it did before this existed: a world that
/// never drew two journals notices nothing.
fn hearths_of(
    workspace: &std::path::Path,
    realms: Option<PathBuf>,
    db: Option<PathBuf>,
) -> Result<Vec<Hearth>> {
    Ok(fleet_of(workspace, realms, db, Invocation::resolve)?.1)
}

/// The same fleet, with the WORLD it was read from kept — what a surface
/// needs when it states facts about the map itself and not only about the
/// journals the map names, which is `brokkr muninn run` and its crossings
/// (decision 0057; decision 0026 ruling 3).
///
/// Opened with [`Invocation::inspect`]: the raven reports a moved
/// crossing as a finding, so a moved crossing must not be the end of the
/// reading. `hearths_of` keeps [`Invocation::resolve`] — its callers
/// include verbs that go on to open a store the world named.
fn world_and_hearths(
    workspace: &std::path::Path,
    realms: Option<PathBuf>,
    db: Option<PathBuf>,
) -> Result<(Option<World>, Vec<Hearth>)> {
    fleet_of(workspace, realms, db, Invocation::inspect)
}

fn fleet_of(
    workspace: &std::path::Path,
    realms: Option<PathBuf>,
    db: Option<PathBuf>,
    open: fn(&std::path::Path, Option<PathBuf>, Option<PathBuf>) -> Result<Invocation>,
) -> Result<(Option<World>, Vec<Hearth>)> {
    let overridden = db.is_some();
    let invocation = open(workspace, realms, db)?.announce();
    let hearths = match (&invocation.world, overridden) {
        (Some(world), false) => world.hearths(),
        _ => Vec::new(),
    };
    let hearths = match hearths.is_empty() {
        true => vec![Hearth {
            realms: Vec::new(),
            journal: invocation.journal,
        }],
        false => hearths,
    };
    Ok((invocation.world, hearths))
}

/// Dispatch one parsed command to its verb's handler (decision 0071
/// ruling 10: a verb is a `Cmd` variant plus a handler). The match is
/// exhaustive and every arm only forwards, so a new verb cannot compile
/// without naming where it is handled.
fn run_with(
    cli: Cli,
    // The directory `realms.json`, `agents/` and `adapters/` are
    // discovered in. Injected rather than read from the process, so what
    // a command resolves is a function of its arguments and not of where
    // the caller happens to stand.
    workspace: &std::path::Path,
    serve_ui: impl FnOnce(PathBuf, u16, bool) -> std::io::Result<()>,
    bridge_iteration_limit: Option<usize>,
    watch_iteration_limit: Option<usize>,
    run_tui: impl FnOnce(Vec<Hearth>, Option<String>, usize) -> Result<tui::Closed>,
) -> Result<ExitCode> {
    use verbs::{backup, delivery, exchange, probe, queue, readouts, setup};
    match cli.command {
        Cmd::Init(args) => setup::init(args),
        Cmd::Costs(args) => readouts::costs(workspace, args),
        Cmd::Ledger(args) => readouts::ledger(workspace, args),
        Cmd::Anchor(args) => exchange::anchor(workspace, args),
        Cmd::KeepRefs { command } => keep_refs(workspace, command),
        Cmd::Ui(args) => readouts::ui(workspace, args, serve_ui),
        Cmd::Tui(args) => readouts::tui(workspace, args, run_tui),
        Cmd::Doctor(args) => setup::doctor(args),
        Cmd::Compile(args) => setup::compile(workspace, args),
        Cmd::Run(args) => delivery::run(workspace, args, &run_view::PRODUCTION),
        Cmd::Resume(args) => delivery::resume(workspace, args, &run_view::PRODUCTION),
        Cmd::Rerun(args) => delivery::rerun(workspace, args),
        Cmd::Conclude(args) => delivery::conclude(workspace, args),
        Cmd::Operator(args) => delivery::operator(workspace, args),
        Cmd::Queue { command } => queue::queue(workspace, command),
        Cmd::Inspect(args) => readouts::inspect(workspace, args, &mut std::io::stdout()),
        Cmd::Transcript(TranscriptArgs {
            run,
            seat,
            turn,
            json,
            realms,
            db,
        }) => transcript_command(workspace, realms, db, run, seat, turn, json),
        Cmd::Seats(args) => readouts::seats(workspace, args, &mut std::io::stdout()),
        Cmd::Watch(args) => readouts::watch(
            workspace,
            args,
            watch_iteration_limit,
            &run_view::PRODUCTION,
        ),
        Cmd::Replay(args) => readouts::replay(workspace, args),
        Cmd::Export(args) => exchange::export(workspace, args),
        Cmd::Backup(args) => backup::backup(workspace, args),
        Cmd::Restore(args) => backup::restore(args),
        Cmd::Import(args) => exchange::import(workspace, args),
        Cmd::VerifyRun(args) => exchange::verify_run(args),
        Cmd::Bridge(args) => exchange::bridge(workspace, args, bridge_iteration_limit),
        Cmd::Realms(args) => readouts::realms(workspace, args),
        Cmd::Runs(args) => readouts::runs(workspace, args),
        Cmd::CrossRun(command) => verbs::analytics::cross_run(workspace, command),
        Cmd::Hands { command } => hands::run(command),
        Cmd::Probe { command } => probe::probe(workspace, command),
        Cmd::Broker { command } => broker::run(command),
        Cmd::Driver(args) => setup::driver(args),
        Cmd::Compare(args) => readouts::compare(workspace, args),
        Cmd::Library(command) => library(workspace, command),
        Cmd::FakeDriver(args) => setup::fake_driver(args),
    }
}

/// The library verbs `Cmd::Library` groups, each to its handler.
fn library(workspace: &std::path::Path, command: LibraryCmd) -> Result<ExitCode> {
    use verbs::setup;
    match command {
        LibraryCmd::Recipes { command } => setup::recipes(workspace, command),
        LibraryCmd::Agents { command } => setup::agents(workspace, command),
        LibraryCmd::Muninn { command } => setup::muninn(workspace, command),
        LibraryCmd::Secrets { command } => setup::secrets(workspace, command),
    }
}

#[cfg(test)]
mod cli_reference_tests;
#[cfg(test)]
mod opened_tests;
#[cfg(test)]
mod tests;
