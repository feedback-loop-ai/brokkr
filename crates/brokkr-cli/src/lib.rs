//! `brokkr` — the one shipped binary (decision 0003, named by decision
//! 0019). No UI, no required services: one executable, one workspace
//! database. The compatibility shim that kept the old name working for
//! a release is gone; one bin target enters here (ruling 9).

mod agents;
mod compare;
mod doctor;
mod init;
mod ledger;
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
use brokkr_core::fold::{fold, RunState, Status};
use brokkr_runtime::realms::{Hearth, World};
use brokkr_runtime::{conclude, operator_command, Bundle, Engine, FencedCommandOutcome};
use brokkr_store::Store;
use clap::{ArgGroup, Parser, Subcommand};
use serde_json::{json, Value};

/// The workspace journal a command opens when neither a map nor `--db`
/// says otherwise. Unchanged since the first release, and the reason a
/// world that never drew a map notices nothing.
///
/// It is a fallback now, not the steady state. Same-realm parallel burns
/// belong in the realm's one `journal` from `realms.json`, which several
/// `brokkr` processes driving different runs can share safely — measured
/// and fenced at the store layer, see `brokkr_store`'s module doc. A
/// worktree-local `.forge/forge.db` stays entirely legal and is what a
/// mapless world still gets, but reaching for one per worktree to keep
/// parallel burns apart is emergency isolation: it buys nothing the
/// shared journal does not already give, and it scatters one realm's
/// history across files that no single reader can fold together.
pub const DEFAULT_DB: &str = ".forge/forge.db";

/// The operator-side secrets store the engine threads to seats with
/// declared bindings (decision 0012), spelled here because `doctor` now
/// reads its NAMES to tell a bound credential from an ambient one
/// (decision 0036 ruling 5). The run verbs resolve the same path against
/// their workdir; doctor has no workdir of its own.
pub const DEFAULT_SECRETS: &str = ".forge/secrets.env";

/// Exit codes: 0 completed/ok · 2 parked (operator needed) · 3 stopped ·
/// 1 error.
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
    Init { dir: PathBuf },
    /// Per-seat cost and session accounting from journal checkpoints —
    /// the LaneTally join surface (stable seat ids, journal-derived).
    Costs {
        #[arg(long)]
        run: String,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
    },
    /// Render the shipper's delivery ledger from journal and repository evidence.
    Ledger {
        #[arg(long)]
        run: String,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
        /// Write `.forge/ledger/<run>.md` here; without it, print the ledger.
        #[arg(long)]
        repo: Option<PathBuf>,
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
        /// The world's map whose realm house declarations doctor checks
        /// (default ./realms.json when present).
        #[arg(long)]
        realms: Option<PathBuf>,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
        /// Operator-side secrets store, so doctor can say which declared
        /// credentials a route is taking from the ambient environment
        /// instead (decision 0036 ruling 5).
        #[arg(long, default_value = DEFAULT_SECRETS)]
        secrets_file: PathBuf,
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
    Conclude {
        #[arg(long)]
        run: String,
        #[arg(long)]
        reason: String,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
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
    /// Adopt an exported run into this journal, byte-identically — the
    /// verb paired with `export`. Journals never merge; one run
    /// relocates. Nothing lands unless the whole chain verifies, the
    /// events fold, the run_id does not already exist here, and the
    /// export is not a redacted derivative.
    Import {
        /// The exported `<run>.ndjson`. Its `<run>.manifest.json`
        /// sidecar is read from beside it and must be there.
        #[arg(long)]
        from: PathBuf,
        /// The world's map — the journal it names is the one opened
        /// (default ./realms.json when present).
        #[arg(long)]
        realms: Option<PathBuf>,
        /// The destination journal. Outranks the map's journal; without
        /// either, .forge/forge.db as always.
        #[arg(long)]
        db: Option<PathBuf>,
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
    /// Bundles reference these as {brokkr} driver <kind> -- <extra args>.
    Driver {
        kind: String,
        /// Arguments after -- pass to the agent CLI
        /// (claude/lanetally/codex/dsh) or form the command template
        /// (exec).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// The model's hands are one tool, and the tool runs in an empty root
    /// (decision 0043): serve the `workspace` tool over MCP on stdio, or
    /// run one command whole inside the same box.
    Hands {
        #[command(subcommand)]
        command: HandsCommand,
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
        /// The concrete effort an adapter pinned (decision 0035 ruling
        /// 5), taken and echoed on exactly the terms `--model` is: the
        /// other half of the hire travels the same argv, so a fake seat
        /// that could not be told its effort would prove the pin
        /// reached the composed command and nothing further.
        #[arg(long)]
        effort: Option<String>,
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
        #[arg(long, default_value = brokkr_runtime::bundle::DEFAULT_AGENTS_DIR)]
        agents_dir: PathBuf,
    },
    /// The definition as written, plus the per-chain-entry resolution
    /// the compiler would compute. An unknown name errors naming the
    /// known set.
    Show {
        name: String,
        #[arg(long, default_value = brokkr_runtime::bundle::DEFAULT_AGENTS_DIR)]
        agents_dir: PathBuf,
        #[arg(long, default_value = brokkr_runtime::bundle::DEFAULT_ADAPTERS_DIR)]
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
        #[arg(long, default_value = brokkr_runtime::bundle::DEFAULT_AGENTS_DIR)]
        agents_dir: PathBuf,
        #[arg(long, default_value = brokkr_runtime::bundle::DEFAULT_ADAPTERS_DIR)]
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
enum KeepRefsCmd {
    /// Plant a keep-ref for every object the run's journal cites.
    /// Idempotent: replanting moves nothing.
    Plant {
        /// Full run id, a unique run-id prefix, or `latest`.
        #[arg(long)]
        run: String,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
    /// Which runs hold which exhibits — one `for-each-ref`, no journal
    /// needed. `--run` narrows the listing to one run.
    List {
        /// Full run id, a unique run-id prefix, or `latest`; without it,
        /// every run holding keep-refs in this repository.
        #[arg(long)]
        run: Option<String>,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
    /// Let one run's exhibits go: remove refs/forge/keep/<run>/*. The
    /// operator's decision alone — nothing in the engine ever deletes a
    /// keep-ref, and the objects are then as mortal as gc leaves them.
    Delete {
        #[arg(long)]
        run: String,
        #[arg(long, default_value = DEFAULT_DB)]
        db: PathBuf,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
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

/// What a compiled bundle looks like to an operator. `brokkr compile` and
/// `brokkr recipes show` print this and nothing else, from here, so the
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
        "strategy": state.strategy,
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
                    let view = brokkr_view::run_view(&events, state.as_ref());
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
fn keep_ref_run(db: &std::path::Path, run: &str) -> Result<String> {
    if db.is_file() {
        return selector::resolve_run(&Store::open(db)?, run);
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
fn keep_refs(command: KeepRefsCmd) -> Result<ExitCode> {
    match command {
        KeepRefsCmd::Plant { run, db, repo } => {
            let store = Store::open(&db)?;
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
        KeepRefsCmd::List { run, db, repo } => {
            let mut held = brokkr_runtime::list_keep_refs(&repo)?;
            if let Some(run) = run {
                let run = keep_ref_run(&db, &run)?;
                held.retain(|holder, _| *holder == run);
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({ "keep": held }))?
            );
        }
        KeepRefsCmd::Delete { run, db, repo } => {
            let run = keep_ref_run(&db, &run)?;
            let removed = brokkr_runtime::delete_keep_refs(&repo, &run)?;
            eprintln!("released {removed} exhibit(s) for {run}");
        }
    }
    Ok(ExitCode::SUCCESS)
}

#[derive(Subcommand, Debug)]
pub enum HandsCommand {
    /// Serve the one `workspace` tool over MCP (newline-delimited JSON-RPC
    /// on stdio); every call runs `bash -lc <command>` inside the box.
    Serve {
        /// The worktree, bound read-write at its own path.
        #[arg(long)]
        workdir: PathBuf,
        /// The box spec as JSON: {"kind":"workspace","network":…,"binds":[…]}.
        #[arg(long, default_value = "\"workspace\"")]
        spec: String,
    },
    /// Run one command whole inside the box with stdio passed through —
    /// how a deterministic `exec` seat holds a gate. Exits with the
    /// command's own code.
    Exec {
        #[arg(long)]
        workdir: PathBuf,
        /// Strategy root, bound read-only at /runtime/bundle.
        #[arg(long)]
        bundle_root: Option<PathBuf>,
        #[arg(long, default_value = "\"workspace\"")]
        spec: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        command: Vec<String>,
    },
}

fn hands(command: HandsCommand) -> anyhow::Result<ExitCode> {
    use brokkr_protocol::hands;
    let parse_spec = |spec: &str| -> anyhow::Result<hands::HandsSpec> {
        let raw: serde_json::Value = serde_json::from_str(spec)?;
        hands::HandsSpec::parse(&raw).map_err(|problem| anyhow::anyhow!("--spec: {problem}"))
    };
    match command {
        HandsCommand::Serve { workdir, spec } => {
            let spec = parse_spec(&spec)?;
            // The session outlives every call: overlay upper layers live
            // here until the harness closes the server's stdin.
            let session = hands::session_dir("serve").map_err(anyhow::Error::msg)?;
            let stdin = std::io::stdin();
            let served = hands::serve(
                stdin.lock(),
                std::io::stdout(),
                &workdir,
                &session,
                &spec,
                &hands::execute,
            );
            let _ = std::fs::remove_dir_all(&session);
            served?;
            Ok(ExitCode::SUCCESS)
        }
        HandsCommand::Exec {
            workdir,
            bundle_root,
            spec,
            command,
        } => {
            let spec = parse_spec(&spec)?;
            // Only the leading separator is ours; a command may carry
            // its own `--`.
            let command: Vec<String> = match command.first().map(String::as_str) {
                Some("--") => command[1..].to_vec(),
                _ => command,
            };
            let code = hands::run_boxed(&spec, &workdir, bundle_root.as_deref(), &command)
                .map_err(anyhow::Error::msg)?;
            Ok(ExitCode::from(
                u8::try_from(code.clamp(0, 255)).unwrap_or(1),
            ))
        }
    }
}

/// Decision 0043 ruling 7: a bundle whose seats box their hands refuses
/// to start without bubblewrap, naming the seats — before any journal is
/// opened or seat spawned. The boundary is Linux's and is never simulated.
fn refuse_unboxable(bundle: &brokkr_runtime::Bundle, path: &std::ffi::OsStr) -> anyhow::Result<()> {
    if bundle.hands.is_empty() {
        return Ok(());
    }
    match brokkr_protocol::hands::bwrap_on(path) {
        Ok(bwrap) => {
            for (site, spec) in &bundle.hands {
                brokkr_protocol::hands::overlay_supported(spec, &bwrap)
                    .map_err(|reason| anyhow::anyhow!("seat '{site}': {reason}"))?;
            }
            Ok(())
        }
        Err(reason) => anyhow::bail!(
            "{reason}; the seats {:?} declare hands and cannot run on this machine \
             (decision 0043 ruling 7 — hands are a Linux boundary)",
            bundle.hands.keys().collect::<Vec<_>>()
        ),
    }
}

fn driver_extra_args(args: Vec<String>) -> Vec<String> {
    if let Some(index) = args.iter().position(|arg| arg == "--") {
        args[index + 1..].to_vec()
    } else {
        args
    }
}

/// A peer still held the shared journal's write lock when this process
/// ran out of patience for it. Its own exit code because it is its own
/// thing: nothing was written, nothing is wrong, and the same command
/// run again is likely to land. Distinct from 1 (a defect), from 2 (a
/// park the run itself decided on) and from 3 (stopped).
pub const CONTENDED_EXIT: u8 = 4;

/// Did this error come from a peer holding the journal's lock?
///
/// Asked of the whole chain and answered by the store's own typed
/// predicate — never by matching error text. Both shapes it arrives in
/// are asked: a `StoreError` raised straight out of a store call, and
/// one an `EngineError` carries — the latter needs asking separately
/// because that variant is `transparent`, which puts the store error's
/// own source in the chain and the store error itself nowhere in it.
///
/// A contention that reached here wrote nothing, so there is no
/// half-done work to describe.
fn contention(error: &anyhow::Error) -> Option<&brokkr_store::StoreError> {
    error.chain().find_map(|link| {
        link.downcast_ref::<brokkr_store::StoreError>()
            .filter(|store| store.is_contention())
            .or_else(|| {
                link.downcast_ref::<brokkr_runtime::EngineError>()
                    .and_then(brokkr_runtime::EngineError::contention)
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
    match contention(error) {
        Some(store) => {
            eprintln!(
                "contended: {store}\nA peer is writing this journal. Nothing was \
                 written and nothing was lost — resume when it is done."
            );
            ExitCode::from(CONTENDED_EXIT)
        }
        None => {
            eprintln!("error: {error:#}");
            ExitCode::from(1)
        }
    }
}

/// The binary's entry: one parse, one command set, one set of exit
/// codes. There is one bin now — decision 0019 ruling 9's shim is gone.
pub fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(e) => report(&e),
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
pub(crate) fn fold_or_quarantine(
    events: &[brokkr_core::EventEnvelope],
) -> Result<RunState, String> {
    fold(events).map_err(|error| error.to_string())
}

/// One refresh for `brokkr tui`: the only place a store is opened on that
/// path, and the reason `tui.rs` can name none. The head is compared on
/// **both** seq and hash — a rewritten journal at equal seq is the
/// tamper case `anchor` exists for, and a console should redraw rather
/// than sit blind.
fn tui_views(
    db: &std::path::Path,
    // Whether this world has exactly one hearth. A one-hearth world
    // opens its journal exactly as it always has; a many-hearth world
    // opens each hearth READ-ONLY, because a console that CREATED the
    // journal of a realm the operator merely tabbed past would have
    // written to a world it came only to read (decision 0026 ruling 5).
    sole: bool,
    ask: tui::Ask,
    head: &mut Option<(u64, String)>,
    seen: &mut Option<(String, u64)>,
    clock: fn() -> String,
) -> Result<tui::Refreshed> {
    // A realm the map names before its first run has no journal yet, and
    // that is an ordinary state of the world, not a fault: the hearth is
    // EMPTY, and an empty hearth is a frame that says so with the keys
    // still live (decision 0026 ruling 2) — exactly what `brokkr runs`
    // already prints for the same realm. Refusing here would count
    // against the console's give-up bound and end the session over a
    // realm the operator merely tabbed to. Only in a many-hearth world:
    // a sole journal that is not on disk is refused by `tui::start`
    // before the loop begins, and that refusal is unchanged.
    if !sole && !db.is_file() {
        if !(ask.force || ask.fleet) {
            return Ok(None);
        }
        // Nothing was read, so nothing is remembered as read: the tick
        // that finds the journal finally there rebuilds from scratch.
        *head = None;
        return Ok(Some(tui::Views {
            now: clock(),
            note: Some(format!(
                "this realm has no journal yet, and a read never creates one: {}",
                db.display()
            )),
            ..tui::Views::empty()
        }));
    }
    let store = match sole {
        true => Store::open(db)?,
        false => Store::open_read_only(db)?,
    };
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
    let entries: Vec<brokkr_view::RunEntry> = folded
        .iter()
        .map(
            |(run_id, feature, created_at, folded_run)| brokkr_view::RunEntry {
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
        brokkr_view::run_view(&events, state.as_ref())
    });
    Ok(Some(tui::Views {
        now: clock(),
        runs: brokkr_view::run_rows(&entries),
        run,
        // The seat's own session, located by the SAME lookup the
        // console's /api/session endpoint uses.
        transcript: ask.session.and_then(ui::session_turns),
        // A journal that was read has nothing to say about itself.
        note: None,
    }))
}

/// The refresh source the console runs on, with the workspace clock
/// bound in. Built by a named function so it is reachable from a test
/// as well as from `brokkr tui`.
///
/// One hearth or many, the ACTIVE tab's journal is the only one this
/// ever opens: a tab nobody has visited has never been asked about, so
/// its store is never opened at all and its journal is never polled
/// (decision 0026 ruling 2). The laziness is the absence of a question,
/// not a cache that could go stale.
fn tui_source<'a>(
    hearths: &'a [Hearth],
    heads: &'a mut [Option<(u64, String)>],
    seen: &'a mut Option<(String, u64)>,
) -> impl FnMut(tui::Ask) -> Result<tui::Refreshed> + 'a {
    let sole = hearths.len() < 2;
    move |ask| {
        let tab = ask.tab.min(hearths.len().saturating_sub(1));
        tui_views(
            &hearths[tab].journal,
            sole,
            ask,
            &mut heads[tab],
            seen,
            now_rfc3339,
        )
    }
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
/// A hearth whose journal is not on disk yet is not consulted, because
/// resolving would open it and `Store::open` creates a file, a WAL and a
/// meta row. When NO hearth has one, the selector passes through
/// unresolved and `tui::start` does the refusing — a read must not create
/// the database it came to read.
///
/// A MANY-hearth world is walked READ-ONLY, the same way [`tui_views`]
/// reads it and for the same reason: the peer realms are journals the
/// operator did not name, and a lookup passing through one must not
/// migrate it (ruling 5). A world of ONE hearth is the journal the
/// operator DID name, opened exactly as `inspect` and `watch` open it —
/// the single-run path is untouched, down to the sidecars it leaves
/// behind.
fn resolve_in_hearths(hearths: &[Hearth], run: String) -> Result<(usize, String)> {
    let sole = hearths.len() < 2;
    let mut refusal: Option<anyhow::Error> = None;
    // The hearths that answered: index, the id it resolved to, and when
    // that run was created — which is what `latest` compares.
    let mut answered: Vec<(usize, String, String)> = Vec::new();
    for (index, hearth) in hearths.iter().enumerate() {
        if !hearth.journal.is_file() {
            continue;
        }
        let opened = match sole {
            true => Store::open(&hearth.journal),
            false => Store::open_read_only(&hearth.journal),
        };
        let listed = opened
            .map_err(anyhow::Error::from)
            .and_then(|store| Ok(store.list_runs()?));
        let runs = match listed {
            Ok(runs) => runs,
            Err(error) => {
                refusal.get_or_insert(error);
                continue;
            }
        };
        let refs: Vec<selector::RunRef<'_>> = runs
            .iter()
            .map(|(run_id, _feature, created_at)| selector::RunRef { run_id, created_at })
            .collect();
        match selector::resolve(&refs, &run) {
            Ok(id) => {
                let created = refs
                    .iter()
                    .find(|candidate| candidate.run_id == id)
                    .map_or(String::new(), |candidate| candidate.created_at.to_string());
                answered.push((index, id, created));
            }
            Err(error) => {
                refusal.get_or_insert(error);
            }
        }
    }
    if run == selector::LATEST {
        return match newest_answer(answered) {
            Some((index, id)) => Ok((index, id)),
            None => match refusal {
                Some(error) => Err(error),
                None => Ok((0, run)),
            },
        };
    }
    match answered.len() {
        1 => {
            let (index, id, _) = answered.swap_remove(0);
            Ok((index, id))
        }
        0 => match refusal {
            Some(error) => Err(error),
            None => Ok((0, run)),
        },
        // These strings reach a terminal through anyhow, so the selector
        // and every realm and id named back are sanitized.
        _ => Err(anyhow::anyhow!(
            "'{}' matches a run in {} realms: {}; name one journal with --db",
            render::Safe::new(&run).as_str(),
            answered.len(),
            answered
                .iter()
                .map(|(index, id, _)| format!(
                    "{} ({})",
                    render::Safe::new(&hearths[*index].label()).as_str(),
                    render::Safe::new(id).as_str()
                ))
                .collect::<Vec<String>>()
                .join(", ")
        )),
    }
}

/// `brokkr tui`'s impure entry: the environment facts are read once here
/// and everything else is injected. The refusals live inside
/// `tui::start`, and the source above opens a store only when it is
/// called — which is after that gate.
fn run_tui(hearths: Vec<Hearth>, run: Option<String>, tab: usize) -> Result<ExitCode> {
    let mut heads: Vec<Option<(u64, String)>> = vec![None; hearths.len()];
    let mut seen: Option<(String, u64)> = None;
    let db_is_file = hearths.iter().any(|hearth| hearth.journal.is_file());
    // A world with one hearth names no tabs, and the console draws none.
    let tabs: Vec<String> = match hearths.len() {
        0 | 1 => Vec::new(),
        _ => hearths.iter().map(Hearth::label).collect(),
    };
    let mut source = tui_source(&hearths, &mut heads, &mut seen);
    tui::start(
        db_is_file,
        run,
        tabs,
        tab,
        tui::production_ops(),
        std::io::stdout().is_terminal(),
        // Animation is enabled exactly when colour is, through the same
        // pure rule `brokkr runs` uses: NO_COLOR, TERM=dumb and a
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
    let overridden = db.is_some();
    let invocation = Invocation::resolve(workspace, realms, db)?.announce();
    let hearths = match (&invocation.world, overridden) {
        (Some(world), false) => world.hearths(),
        _ => Vec::new(),
    };
    Ok(match hearths.is_empty() {
        true => vec![Hearth {
            realms: Vec::new(),
            journal: invocation.journal,
        }],
        false => hearths,
    })
}

/// One hearth's runs, folded. A journal that will not open at all is the
/// hearth's own refusal, in its own words: a many-hearth listing survives
/// a realm whose journal is not there yet, the same way a fleet listing
/// already survives one unfoldable run.
///
/// Opened READ-ONLY, and deliberately: a reading surface that creates the
/// journal it came to read has written to a world it was only asked to
/// look at (decision 0026 ruling 5).
type FoldedRun = (String, String, String, Result<RunState, String>);

fn hearth_runs(journal: &std::path::Path) -> Result<Vec<FoldedRun>, String> {
    // One error voice for the three doors: what a hearth refuses with is
    // the store's own words, wherever in the read it refused.
    fn hearth_error(error: brokkr_store::StoreError) -> String {
        error.to_string()
    }
    let store = Store::open_read_only(journal).map_err(hearth_error)?;
    let mut folded = Vec::new();
    for (run_id, feature, created_at) in store.list_runs().map_err(hearth_error)? {
        let events = store.load(&run_id).map_err(hearth_error)?;
        folded.push((run_id, feature, created_at, fold_or_quarantine(&events)));
    }
    Ok(folded)
}

/// The pinned manifest an export wrote beside its journal, named the
/// way `Cmd::Export` names it: `<run>.ndjson` is paired with
/// `<run>.manifest.json`, and — since the stem carries the marker —
/// `<run>.redacted.ndjson` with `<run>.redacted.manifest.json`, so a
/// redacted pair can be named in full and refused by name.
fn manifest_beside(journal: &std::path::Path) -> PathBuf {
    let stem = journal
        .file_stem()
        .unwrap_or(journal.as_os_str())
        .to_string_lossy()
        .to_string();
    journal.with_file_name(format!("{stem}.manifest.json"))
}

/// Compile a bundle against the tree the invocation stands in. The
/// agent library and the adapters are read from the WORKSPACE for the
/// same reason `realms.json` is (decision 0023): what a command resolves
/// is a function of its arguments, not of where the caller happens to
/// stand. The default workspace is `.`, so an operator sees exactly the
/// roots they always did — but since decision 0021 a compile reads the
/// adapter data for a bundle that names no agent at all (a gate seat's
/// trust tier and a secret binding's grant live there), and a verb that
/// resolved one tree while compiling against another would be the machine
/// diagnosing itself wrong.
pub(crate) fn compile_in(workspace: &std::path::Path, dir: &std::path::Path) -> Result<Bundle> {
    Ok(Bundle::compile_with(
        dir,
        &workspace.join(brokkr_runtime::bundle::DEFAULT_AGENTS_DIR),
        &workspace.join(brokkr_runtime::bundle::DEFAULT_ADAPTERS_DIR),
    )?)
}

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
    run_tui: impl FnOnce(Vec<Hearth>, Option<String>, usize) -> Result<ExitCode>,
) -> Result<ExitCode> {
    match cli.command {
        Cmd::Init { dir } => {
            // The recipe lands in `dir`; the repository it describes is
            // the WORKSPACE, read for its manifests so the implement and
            // verify seats are told commands that would actually run
            // there. Same tree every other verb resolves (decision 0023),
            // for the same reason: what a command produces is a function
            // of its arguments, not of where the caller happens to stand.
            let digest = init::init(&dir, workspace)?;
            eprintln!(
                "initialized reviewable bundle at {} (digest {digest})",
                dir.display()
            );
            // The scaffold carries its own `adapters/` and `agents/`,
            // where the trust tier its gate seats compile against and the
            // tool grants its seats run under are declared (decisions
            // 0021 and 0016). Every other verb reads those trees from the
            // workspace, which is the directory brokkr is run in — so say
            // once, here, where to stand.
            eprintln!(
                "run brokkr from inside {} — its adapters/ and agents/ declare \
                 the trust tier and the tool grants its seats run under",
                dir.display()
            );
            if let Err(reason) =
                brokkr_protocol::hands::bwrap_on(&std::env::var_os("PATH").unwrap_or_default())
            {
                eprintln!(
                    "warning: {reason}; the scaffolded seats [\"ship\", \"verify\"] \
                     declare hands and will refuse to run here — the shipped gates \
                     require Linux with bubblewrap on PATH"
                );
            }
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
        Cmd::Ledger { run, db, repo } => {
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
                let report = brokkr_runtime::verify_anchor(&store, &repo, &run)?;
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                let sha = brokkr_runtime::anchor(&store, &repo, &run)?;
                eprintln!("anchored {run} at {sha}");
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::KeepRefs { command } => keep_refs(command),
        Cmd::Ui { db, port, open } => {
            serve_ui(db, port, open)?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Tui { run, realms, db } => {
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
        Cmd::Doctor {
            bundle,
            realms,
            db,
            secrets_file,
        } => {
            let report = doctor::doctor(bundle.as_deref(), &db, &secrets_file, realms.as_deref());
            println!("{}", report.render());
            Ok(if report.healthy {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        Cmd::Compile { bundle } => {
            let bundle = compile_in(workspace, &bundle)?;
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
            let bundle = compile_in(workspace, &recipes::resolve(bundle, recipe, &recipes_dir)?)?;
            refuse_unboxable(&bundle, &std::env::var_os("PATH").unwrap_or_default())?;
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
                let envelope: brokkr_core::dispatch::DispatchEnvelopeV2 =
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
            let bundle = compile_in(workspace, &recipes::resolve(bundle, recipe, &recipes_dir)?)?;
            refuse_unboxable(&bundle, &std::env::var_os("PATH").unwrap_or_default())?;
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
                .filter(|e| e.event_type == brokkr_core::EventType::RunStarted)
                .and_then(|e| e.payload.get("feature"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    anyhow::anyhow!("source run '{run}' has no run/started feature to re-run")
                })?
                .to_string();
            let bundle = compile_in(workspace, &recipes::resolve(bundle, recipe, &recipes_dir)?)?;
            refuse_unboxable(&bundle, &std::env::var_os("PATH").unwrap_or_default())?;
            let mut engine = Engine::start(store, bundle, &feature, repo)?;
            engine.secrets_file = secrets_file;
            eprintln!(
                "rerun of {run} as {} under {}",
                engine.run_id, engine.bundle.name
            );
            let end = engine.drive()?;
            Ok(finish(&end.state))
        }
        Cmd::Conclude { run, reason, db } => {
            let mut store = Store::open(&db)?;
            let operator = std::env::var("USER").unwrap_or("operator".into());
            let state = conclude(&mut store, &run, &operator, &reason)?;
            Ok(finish(&state))
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
            // The command is fenced against a concurrently-driving
            // engine, so it can come back refused. Saying "recorded"
            // there would tell the operator the opposite of what the
            // journal says.
            match operator_command(&mut store, &run, &command, &operator, &reason)? {
                FencedCommandOutcome::Accepted { .. } => {
                    eprintln!(
                        "recorded operator {command}; continue with: brokkr resume --run {run}"
                    );
                    Ok(ExitCode::SUCCESS)
                }
                FencedCommandOutcome::Rejected { reason, .. } => {
                    // The reason word carries which condition it was —
                    // `lost_fence` for a run that moved under the
                    // operator, `after_terminal` or
                    // `run_not_awaiting_operator` for a command the run
                    // was never in a state to take — so this line states
                    // the condition and passes the word through rather
                    // than paraphrasing it into one story.
                    eprintln!(
                        "refused operator {command} ({reason}): the run is not in a state \
                         this command can apply to; the refusal is journaled. Read it with: \
                         brokkr inspect --run {run}"
                    );
                    Ok(ExitCode::FAILURE)
                }
            }
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
            let view = brokkr_view::run_view(&events, Some(&state));
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
                let mut redactor = brokkr_store::Redactor::learn(&[&ndjson, &raw_manifest]);
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
        Cmd::Import { from, realms, db } => {
            let db = journal_of(workspace, realms, db)?;
            // The sidecar is required, not optional: it is where an
            // export declares itself redacted, and an import that
            // shrugged at a missing manifest would accept exactly the
            // pair whose declaration went missing.
            let manifest_path = manifest_beside(&from);
            let ndjson =
                std::fs::read_to_string(&from).context(format!("reading {}", from.display()))?;
            let raw = std::fs::read_to_string(&manifest_path)
                .context(format!("reading {}", manifest_path.display()))?;
            let manifest: Value = serde_json::from_str(&raw)
                .context(format!("parsing {}", manifest_path.display()))?;
            let mut store = Store::open(&db)?;
            let adoption = store.import_run(&ndjson, &manifest, &from)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "run_id": adoption.run_id,
                    "events": adoption.events,
                    "chain": "verified",
                    "adopted": "byte-identical",
                    "journal_head_hash": adoption.head_hash,
                    "imported_at": adoption.arrival.imported_at,
                    "imported_from": adoption.arrival.imported_from,
                }))?
            );
            // `import_run`'s run_id gate already refuses anything a
            // terminal could not print plainly, so this is belt over
            // braces — but the house rule is that a journal string
            // reaching a tty goes through `Safe`, and the one line
            // telling an operator the adoption happened is a poor place
            // to start making exceptions.
            eprintln!(
                "imported {} into {} ({} events)",
                render::Safe::new(&adoption.run_id).as_str(),
                db.display(),
                adoption.events
            );
            Ok(ExitCode::SUCCESS)
        }
        Cmd::VerifyRun { file } => {
            let ndjson =
                std::fs::read_to_string(&file).context(format!("reading {}", file.display()))?;
            let state = brokkr_store::verify_export(&ndjson)?;
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
            let transport = brokkr_bridge::HttpTransport::new(looper_url, token);
            let mut bridge = brokkr_bridge::Bridge::new(transport);
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
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Runs { realms, db, json } => {
            let hearths = hearths_of(workspace, realms, db)?;
            // A world with several hearths lists each one under its own
            // realm; a world with one is byte-for-byte the listing it
            // always was, down to opening its journal the same way.
            if hearths.len() > 1 {
                let read: Vec<Result<Vec<FoldedRun>, String>> = hearths
                    .iter()
                    .map(|hearth| hearth_runs(&hearth.journal))
                    .collect();
                let entries: Vec<Vec<brokkr_view::RunEntry>> = read
                    .iter()
                    .map(|folded| match folded {
                        Err(_) => Vec::new(),
                        Ok(runs) => runs
                            .iter()
                            .map(
                                |(run_id, feature, created_at, state)| brokkr_view::RunEntry {
                                    run_id,
                                    feature,
                                    created_at,
                                    state: state.as_ref().ok(),
                                    detail: state.as_ref().err().map(String::as_str),
                                },
                            )
                            .collect(),
                    })
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
                        detail: read[index].as_ref().err().map(String::as_str),
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
                return Ok(ExitCode::SUCCESS);
            }
            let db = hearths
                .into_iter()
                .next()
                .expect("a world resolves to at least one hearth")
                .journal;
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
            let entries: Vec<brokkr_view::RunEntry> = folded
                .iter()
                .map(
                    |(run_id, feature, created_at, folded_run)| brokkr_view::RunEntry {
                        run_id,
                        feature,
                        created_at,
                        state: folded_run.as_ref().ok(),
                        detail: folded_run.as_ref().err().map(String::as_str),
                    },
                )
                .collect();
            let view = brokkr_view::run_rows(&entries);
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
        Cmd::Hands { command } => hands(command),
        Cmd::Driver { kind, args } => {
            let kind = brokkr_protocol::adapters::AdapterKind::parse(&kind).ok_or_else(|| {
                anyhow::anyhow!(
                    "unknown driver '{kind}'; known: claude, lanetally, codex, dsh, exec"
                )
            })?;
            let extra = driver_extra_args(args);
            brokkr_protocol::adapters::serve(kind, extra)?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Compare { run_a, run_b, db } => {
            compare::compare(&run_a, &run_b, &db)?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Recipes { command } => {
            match command {
                RecipesCmd::List { dir } => recipes::list(workspace, &dir)?,
                RecipesCmd::Add { source, name, dir } => {
                    recipes::add(workspace, &source, &name, &dir)?
                }
                RecipesCmd::Show { name, dir } => {
                    let bundle = compile_in(workspace, &recipes::resolve(None, Some(name), &dir)?)?;
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
                let hearths = hearths_of(workspace, realms, db)?;
                muninn::run(
                    &hearths,
                    &agents_dir,
                    &adapters_dir,
                    &record,
                    &now_rfc3339(),
                )
            }
            MuninnCmd::List { record, json } => {
                muninn::list(&record, json)?;
                Ok(ExitCode::SUCCESS)
            }
        },
        Cmd::Secrets { command } => {
            use brokkr_protocol::secret;
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
            effort,
        } => {
            brokkr_protocol::fake::run_fake_driver(
                &script,
                &state,
                model.as_deref(),
                effort.as_deref(),
            )?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

#[cfg(test)]
mod tests;
