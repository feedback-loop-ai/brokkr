//! Build each verb's arguments in its own function. A single Subcommand
//! derive over all inline fields keeps every builder temporary in one
//! debug stack frame; slice 0046's extra verb overflowed Windows' main
//! stack before parsing or walking any bundle. Args delegates the finite
//! command tree one verb at a time, without increasing the process stack.

mod member_retry;
mod queue;
pub(crate) use member_retry::MemberRetryArgs;
pub(crate) use queue::{QueueAddArgs, QueueCmd, QueueEntryArgs, QueueJudgeArgs, QueueListArgs};

use std::path::PathBuf;

/// Which journal a verb opens, asked the same way at every verb that
/// takes one (#374): `--db` outranks the journal the map names, and
/// with neither the discovered folder's (decision 0083). Resolved once,
/// by `JournalArgs::journal`, before the verb opens anything.
#[derive(clap::Args)]
#[group(skip)]
pub(super) struct JournalArgs {
    /// The world's map — the journal it names is the one opened (default the found folder's
    /// .brokkr/realms.json, or a library's ./realms.json, when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without either, the found folder's
    /// .brokkr/state/forge.db, or a library's .forge/forge.db.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct InitArgs {
    /// The repository root whose .brokkr/ folder is written; a folder that
    /// already holds more than moved state is refused, never overwritten.
    pub(super) dir: PathBuf,
}

#[derive(clap::Subcommand)]
pub(super) enum SecretsCmd {
    /// Bind NAME to a value read from STDIN (never argv — the CLI obeys
    /// its own injection discipline). Creates the store 0600.
    Set {
        /// The name a seat's declared binding asks for.
        name: String,
        /// The store file, created 0600 when absent (default .brokkr/state/secrets.env, or a library's .forge/secrets.env).
        #[arg(long)]
        secrets_file: Option<PathBuf>,
    },
    /// Print bound names, one per line — names, never values.
    List {
        /// The store file to read (default .brokkr/state/secrets.env, or a library's .forge/secrets.env).
        #[arg(long)]
        secrets_file: Option<PathBuf>,
    },
    /// Remove NAME from the store.
    Remove {
        /// The bound name to remove.
        name: String,
        /// The store file to remove it from (default .brokkr/state/secrets.env, or a library's .forge/secrets.env).
        #[arg(long)]
        secrets_file: Option<PathBuf>,
    },
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct CostsArgs {
    /// The run whose seats are accounted: a full run id, a unique run-id
    /// prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    #[command(flatten)]
    pub(super) journal: JournalArgs,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct LedgerArgs {
    /// The run whose ledger is rendered: a full run id, a unique run-id
    /// prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    #[command(flatten)]
    pub(super) journal: JournalArgs,
    /// Write `<run>.md` under this repository's `.brokkr/state/ledger/`, or
    /// `.forge/ledger/` with no folder; without it, print the ledger.
    #[arg(long)]
    pub(super) repo: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct AnchorArgs {
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    #[command(flatten)]
    pub(super) journal: JournalArgs,
    /// The git repository whose refs/forge/<run> holds the anchor.
    #[arg(long, default_value = ".")]
    pub(super) repo: PathBuf,
    /// Verify instead of writing a new anchor.
    #[arg(long)]
    pub(super) check: bool,
}

mod ui;
pub(super) use ui::UiArgs;

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct TuiArgs {
    /// Full run id, a unique run-id prefix, or `latest`; opens
    /// directly at that run's level.
    #[arg(long)]
    pub(super) run: Option<String>,
    /// The world's map — the journal it names is the one opened (default the found folder's
    /// .brokkr/realms.json, or a library's ./realms.json, when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without either, the found folder's
    /// .brokkr/state/forge.db, or a library's .forge/forge.db.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct DoctorArgs {
    /// A bundle directory to compile and check as well (default the found .brokkr/
    /// folder, compiled once for the whole report and its folder line).
    #[arg(long)]
    pub(super) bundle: Option<PathBuf>,
    /// The world's map whose realm house declarations doctor checks (default the found
    /// folder's .brokkr/realms.json, or a library's ./realms.json, when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal whose database doctor opens. Outranks the map's journal; without
    /// either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
    /// Operator-side secrets store, so doctor can say which declared
    /// credentials a route is taking from the ambient environment
    /// instead (decision 0036 ruling 5; default .brokkr/state/secrets.env, else .forge/secrets.env).
    #[arg(long)]
    pub(super) secrets_file: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct CompileArgs {
    /// The bundle directory to validate, compiled against the workspace.
    #[arg(long)]
    pub(super) bundle: PathBuf,
}

/// What a delivering verb runs and what its seats are given, asked the
/// same way by `run`, `resume` and `rerun`.
#[derive(clap::Args, Default)]
#[group(skip)]
pub(super) struct DeliveryArgs {
    /// The bundle directory to deliver under; without it or `--recipe`,
    /// the discovered .brokkr/ folder. `resume` compiles it to the run's
    /// pinned manifest and refuses any drift.
    #[arg(long)]
    pub(super) bundle: Option<PathBuf>,
    /// Named recipe, resolved to <recipes-dir>/<name>.
    #[arg(long)]
    pub(super) recipe: Option<String>,
    /// The recipe library `--recipe` is resolved in (default the found
    /// .brokkr/recipes, else recipes).
    #[arg(long)]
    pub(super) recipes_dir: Option<PathBuf>,
    /// Operator-side secrets store for seats with declared bindings
    /// (default .brokkr/state/secrets.env, or <workdir>/.forge/secrets.env).
    #[arg(long)]
    pub(super) secrets_file: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct RunArgs {
    #[command(flatten)]
    pub(super) launch: LaunchArgs,
    /// On a terminal, print the plain lines and the summary instead of
    /// opening the run view. Off a terminal they always print.
    #[arg(long)]
    pub(super) no_view: bool,
}

/// The launch a new run is: what `brokkr run` starts and `brokkr queue
/// add` queues.
#[derive(clap::Args)]
#[group(skip)]
pub(super) struct LaunchArgs {
    #[command(flatten)]
    pub(super) delivery: DeliveryArgs,
    /// The feature the run delivers, as text: recorded when the run
    /// starts and handed to its seats.
    #[arg(long)]
    pub(super) feature: String,
    /// The world's map: realms and the journal they share (decision
    /// 0023). Defaults to .brokkr/realms.json, else ./realms.json; a map
    /// named here and missing or malformed is a refusal, never a
    /// silent fallback.
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without either, the found folder's
    /// .brokkr/state/forge.db, or a library's .forge/forge.db.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
    /// The repository the run operates on: the bundle is compiled
    /// against its realm and the engine works in it. Without it, the
    /// workspace (the current directory) is compiled against, and the
    /// engine gets no repository override, so it works there too.
    #[arg(long)]
    pub(super) repo: Option<PathBuf>,
    /// Canonical forge-dispatch/v2 JSON. When present the run id,
    /// Looper/grant correlation, recipe, repository, budget, and producer
    /// bounds are pinned into an immutable run-manifest/v2.
    #[arg(long)]
    pub(super) dispatch: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct ResumeArgs {
    #[command(flatten)]
    pub(super) delivery: DeliveryArgs,
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    #[command(flatten)]
    pub(super) journal: JournalArgs,
    /// The repository the resumed run operates on. Without it, the
    /// engine gets no repository override and works in the current
    /// directory.
    #[arg(long)]
    pub(super) repo: Option<PathBuf>,
    /// On a terminal, print the plain lines and the summary instead of
    /// opening the run view. Off a terminal they always print.
    #[arg(long)]
    pub(super) no_view: bool,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct RerunArgs {
    /// The source run whose feature is re-run.
    #[arg(long)]
    pub(super) run: String,
    #[command(flatten)]
    pub(super) delivery: DeliveryArgs,
    #[command(flatten)]
    pub(super) journal: JournalArgs,
    /// The repository the new run operates on: the bundle is compiled
    /// against its realm and the engine works in it. Without it, the
    /// workspace (the current directory) is compiled against, and the
    /// engine gets no repository override, so it works there too.
    #[arg(long)]
    pub(super) repo: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct CompareArgs {
    /// The first run: a full run id, a unique run-id prefix, or `latest`.
    pub(super) run_a: String,
    /// The second run, named the same ways.
    pub(super) run_b: String,
    #[command(flatten)]
    pub(super) journal: JournalArgs,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct ConcludeArgs {
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    /// Why the run is closed, recorded with the stop conclusion.
    #[arg(long)]
    pub(super) reason: String,
    #[command(flatten)]
    pub(super) journal: JournalArgs,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct OperatorArgs {
    #[command(flatten)]
    pub(super) member: MemberRetryArgs,
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    /// "retry" re-runs the current phase; "stop" ends the run; "release"
    /// attests that every process the run's ended attempts owned has ended,
    /// and frees the worktree their writer lease holds; "supersede" records
    /// that residual findings on a run that has already finished are closed
    /// by another run (decision 0047); "acknowledge" lists a quarantined
    /// run under older until its journal moves (#511).
    pub(super) command: String,
    /// Why the operator issued the command, recorded with it.
    #[arg(long)]
    pub(super) reason: String,
    /// supersede only: the residual findings this closes, by the
    /// sequence number of the ruling each was read from. Repeatable,
    /// or one comma-separated list.
    #[arg(long, value_delimiter = ',')]
    pub(super) findings: Vec<u64>,
    /// supersede only: the run that closed them.
    #[arg(long)]
    pub(super) by_run: Option<String>,
    /// supersede only: the `transition/decided` in that run which
    /// closed them.
    #[arg(long)]
    pub(super) by_seq: Option<u64>,
    /// supersede only: the realm that run was read in. Omitted for
    /// the workspace journal, which is every one-hearth world.
    #[arg(long)]
    pub(super) by_realm: Option<String>,
    /// The journal the command is written to — and, for supersede, the
    /// map a citation into another hearth is read through.
    #[command(flatten)]
    pub(super) journal: JournalArgs,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct InspectArgs {
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    /// The world's map — the journal it names is the one opened (default the found folder's
    /// .brokkr/realms.json, or a library's ./realms.json, when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without either, the found folder's
    /// .brokkr/state/forge.db, or a library's .forge/forge.db.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
    /// Emit the view model verbatim — this is what scripts read.
    #[arg(long)]
    pub(super) json: bool,
    /// Scope the readout to one phase.
    #[arg(long)]
    pub(super) phase: Option<String>,
    /// Scope the readout to one seat, by label or participant key.
    #[arg(long)]
    pub(super) seat: Option<String>,
    /// Open the retained response the call this id names in the run cites, verified against
    /// its recorded digest, never a path; with --json, beside the call's provenance.
    #[arg(long, value_name = "CALL_ID", conflicts_with_all = ["phase", "seat"])]
    pub(super) capability_call: Option<String>,
    /// The operated repository whose run-local directory holds the call's retained response, as
    /// `ledger --repo` resolves it; without it, the workspace's own.
    #[arg(long, requires = "capability_call")]
    pub(super) repo: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct TranscriptArgs {
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    /// A participant key, or a label that is unique within the run.
    #[arg(long)]
    pub(super) seat: String,
    /// One-based displayed-turn index; omitted reads the whole transcript.
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    pub(super) turn: Option<u64>,
    /// Emit the `brokkr.transcript/v1` document.
    #[arg(long)]
    pub(super) json: bool,
    /// The world's map — the journal it names is the one opened (default the found folder's
    /// .brokkr/realms.json, or a library's ./realms.json, when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without either, the found folder's
    /// .brokkr/state/forge.db, or a library's .forge/forge.db.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct SeatsArgs {
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    /// The world's map — the journal it names is the one opened (default the found folder's
    /// .brokkr/realms.json, or a library's ./realms.json, when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without either, the found folder's
    /// .brokkr/state/forge.db, or a library's .forge/forge.db.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
    /// Emit the view model verbatim — `inspect --json`'s own bytes.
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct WatchArgs {
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    /// The world's map — the journal it names is the one opened (default the found folder's
    /// .brokkr/realms.json, or a library's ./realms.json, when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without either, the found folder's
    /// .brokkr/state/forge.db, or a library's .forge/forge.db.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
    /// Print one frame and exit.
    #[arg(long)]
    pub(super) once: bool,
    /// Poll interval in milliseconds (floored at 100).
    #[arg(long = "interval", default_value_t = 750)]
    pub(super) interval_ms: u64,
    /// On a terminal, redraw the text frames instead of opening the run
    /// view. Off a terminal, and with `--once`, they always print.
    #[arg(long)]
    pub(super) no_view: bool,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct ReplayArgs {
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    #[command(flatten)]
    pub(super) journal: JournalArgs,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct ExportArgs {
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    /// The directory `<run>.ndjson` and `<run>.manifest.json` are
    /// written into, created when absent.
    #[arg(long, default_value = ".")]
    pub(super) out: PathBuf,
    /// The world's map — the journal it names is the one opened (default the found folder's
    /// .brokkr/realms.json, or a library's ./realms.json, when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without either, the found folder's
    /// .brokkr/state/forge.db, or a library's .forge/forge.db.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
    /// Also write a sanitized copy for publishable fixtures —
    /// `<run>.redacted.ndjson` and `<run>.redacted.manifest.json` —
    /// with every absolute path in event payloads rewritten to a
    /// stable placeholder. The verbatim pair is written unchanged;
    /// the redacted copy's recorded hashes no longer verify, and its
    /// manifest says so.
    #[arg(long)]
    pub(super) redact: bool,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct ImportArgs {
    /// The exported `<run>.ndjson`. Its `<run>.manifest.json`
    /// sidecar is read from beside it and must be there.
    #[arg(long)]
    pub(super) from: PathBuf,
    /// The world's map — the journal it names is the one opened (default the found folder's
    /// .brokkr/realms.json, or a library's ./realms.json, when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The destination journal. Outranks the map's journal; without either, the found folder's
    /// .brokkr/state/forge.db, or a library's .forge/forge.db.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct VerifyRunArgs {
    /// The exported `<run>.ndjson` journal to verify.
    pub(super) file: PathBuf,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct BridgeArgs {
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    #[command(flatten)]
    pub(super) journal: JournalArgs,
    /// The base URL of the Looper producer API.
    #[arg(long)]
    pub(super) looper_url: String,
    /// The environment variable the API key is read from; unset or
    /// empty is refused.
    #[arg(long, default_value = "LOOPER_API_KEY")]
    pub(super) token_env: String,
    /// Keep tailing the verified journal and command feed.
    #[arg(long)]
    pub(super) follow: bool,
    /// With `--follow`, the pause between syncs in milliseconds
    /// (floored at 100).
    #[arg(long, default_value_t = 750)]
    pub(super) interval_ms: u64,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct RunsArgs {
    /// The world's map — the journal it names is the one opened (default the found folder's
    /// .brokkr/realms.json, or a library's ./realms.json, when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without either, the found folder's
    /// .brokkr/state/forge.db, or a library's .forge/forge.db.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
    /// Emit the view model verbatim — this is what scripts read.
    #[arg(long)]
    pub(super) json: bool,
    /// Print one page of at most N runs, and where the next page starts (`next`).
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..))]
    pub(super) limit: Option<u32>,
    /// Print the page after this cursor: a page's `next`.
    #[arg(long, value_name = "CURSOR", value_parser = super::fleet::cursor)]
    pub(super) after: Option<brokkr_view::Cursor>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct RealmsArgs {
    /// The map to read (default .brokkr/realms.json, else ./realms.json).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without either, the found folder's
    /// .brokkr/state/forge.db, or a library's .forge/forge.db.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
    /// Emit the view model verbatim — this is what scripts read.
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct DriverArgs {
    /// The adapter to run: claude, lanetally, codex, dsh or exec.
    pub(super) kind: String,
    /// Arguments after -- pass to the agent CLI
    /// (claude/lanetally/codex/dsh) or form the command template
    /// (exec).
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub(super) args: Vec<String>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct FakeDriverArgs {
    #[arg(long)]
    pub(super) script: PathBuf,
    #[arg(long)]
    pub(super) state: PathBuf,
    /// The concrete model an adapter pinned (decision 0016). Echoed
    /// back as a checkpoint so a proof can assert the pin actually
    /// reached the driver rather than trusting the composed argv.
    #[arg(long)]
    pub(super) model: Option<String>,
    /// The concrete effort an adapter pinned (decision 0035 ruling
    /// 5), taken and echoed on exactly the terms `--model` is: the
    /// other half of the hire travels the same argv, so a fake seat
    /// that could not be told its effort would prove the pin
    /// reached the composed command and nothing further.
    #[arg(long)]
    pub(super) effort: Option<String>,
}

#[derive(clap::Subcommand)]
pub(super) enum ProbeCmd {
    /// Run one agent CLI headless against a scratch repository and HOME,
    /// and report each fact its adapter must declare as measured,
    /// unmeasured or unsupported, beside the adapter's own fields and the
    /// seat eligibility the facts derive.
    Harness(ProbeHarnessArgs),
}

/// The operator's libraries and stores, flattened into the top-level
/// verbs so the command line reads as it always has, and dispatched as
/// one group (`lib::library`).
#[derive(clap::Subcommand)]
pub(super) enum LibraryCmd {
    /// The recipe library: bundle directories as named, swappable
    /// delivery strategies.
    Recipes {
        #[command(subcommand)]
        command: super::RecipesCmd,
    },
    /// The agent library (decision 0016): one definition per agent —
    /// description, charter, an ordered chain of abstract model names,
    /// abstract tool/MCP configuration — that seats reference by name.
    Agents {
        #[command(subcommand)]
        command: super::AgentsCmd,
    },
    /// The standing overseer (decision 0020): read the fleet, propose to
    /// the operator, execute nothing. It opens the journal read-only,
    /// issues no operator command, starts no run, and records every
    /// proposal — with the run ids and sequence numbers it was derived
    /// from — in its own append-only file beside the journal.
    Muninn {
        #[command(subcommand)]
        command: super::MuninnCmd,
    },
    /// Manage the operator-side secrets store (decision 0012): bundles
    /// and journals carry NAMES only; values live in this env-format
    /// file outside version control. There is no value-printing verb.
    Secrets {
        #[command(subcommand)]
        command: SecretsCmd,
    },
}

/// The MCP capability broker's one verb (decision 0077), and its private
/// observer.
#[derive(clap::Subcommand)]
pub(super) enum BrokerCmd {
    /// Serve one held MCP capability on stdio from the engine's plan
    /// bound to this attempt. The locator and digest name a plan; they
    /// confer no authority, and an unbound plan is refused.
    Serve(BrokerServeArgs),
    /// `serve`'s own observer (U6c5b): rebind the same plan, observe its
    /// server box's sources, and hand the record and the checked handles
    /// back over the private socket `serve` gave it as stdout. It holds no
    /// authority `serve` does not.
    #[command(hide = true)]
    Observe(BrokerServeArgs),
    /// `serve`'s own waiting bootstrap (U6c6a, U6c8): inside the server box,
    /// read the broker's sealed intent from one inherited pipe, check the
    /// box it stands in, write one ready message on another, then receive
    /// the binding frame on the first and exec the sealed server, reporting
    /// a failure on a third. It takes no plan, store or grant.
    #[command(hide = true)]
    Bootstrap(BrokerBootstrapArgs),
    /// The engine's provisioning observer (U7c0b): survey one server box
    /// from one request on stdin and write one reply on stdout.
    #[command(hide = true)]
    Survey,
}

/// The bootstrap's three private control descriptors, by number. They are
/// taken as text, a negative number or no value included, so that an
/// absent or malformed one is the box's establishment cause rather than a
/// usage error.
#[derive(clap::Args)]
#[group(skip)]
pub(super) struct BrokerBootstrapArgs {
    /// The inherited pipe the broker's sealed intent arrives on.
    #[arg(long, num_args = 0..=1, allow_negative_numbers = true)]
    pub(super) control: Option<String>,
    /// The inherited pipe the one ready message leaves on.
    #[arg(long, num_args = 0..=1, allow_negative_numbers = true)]
    pub(super) ready: Option<String>,
    /// The inherited pipe a failed handoff or exec is reported on.
    #[arg(long, num_args = 0..=1, allow_negative_numbers = true)]
    pub(super) status: Option<String>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct BrokerServeArgs {
    /// The engine-written plan, as an absolute path.
    #[arg(long, value_parser = super::broker::plan_locator)]
    pub(super) plan: PathBuf,
    /// The plan's sha256, 64 lowercase hex characters.
    #[arg(long, value_parser = super::broker::plan_digest)]
    pub(super) plan_digest: String,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct ProbeHarnessArgs {
    /// The adapter whose harness is probed: claude, codex or dsh.
    #[arg(long)]
    pub(super) adapter: String,
    /// The CLI to launch (default: the adapter's binary, found on PATH).
    #[arg(long)]
    pub(super) cli: Option<String>,
    /// Write the report here. A report already here is read first, and
    /// every reading that moved since it is reported as drift.
    #[arg(long)]
    pub(super) out: Option<PathBuf>,
    /// A credential every credentialed launch is given, bound by name
    /// from the secrets store (decision 0012). Repeatable.
    #[arg(long = "credential")]
    pub(super) credentials: Vec<String>,
    /// The secrets store each `--credential` is read from (default .brokkr/state/secrets.env, or a library's .forge/secrets.env).
    #[arg(long)]
    pub(super) secrets_file: Option<PathBuf>,
    /// The directory holding the adapter files `--adapter` names (default .brokkr/adapters, else adapters).
    #[arg(long)]
    pub(super) adapters_dir: Option<PathBuf>,
}
