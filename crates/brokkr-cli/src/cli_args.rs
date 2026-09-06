//! Build each verb's arguments in its own function. A single Subcommand
//! derive over all inline fields keeps every builder temporary in one
//! debug stack frame; slice 0046's extra verb overflowed Windows' main
//! stack before parsing or walking any bundle. Args delegates the finite
//! command tree one verb at a time, without increasing the process stack.

use super::{DEFAULT_DB, DEFAULT_SECRETS};
use std::path::PathBuf;

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct InitArgs {
    pub(super) dir: PathBuf,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct CostsArgs {
    #[arg(long)]
    pub(super) run: String,
    #[arg(long, default_value = DEFAULT_DB)]
    pub(super) db: PathBuf,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct LedgerArgs {
    #[arg(long)]
    pub(super) run: String,
    #[arg(long, default_value = DEFAULT_DB)]
    pub(super) db: PathBuf,
    /// Write `.forge/ledger/<run>.md` here; without it, print the ledger.
    #[arg(long)]
    pub(super) repo: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct AnchorArgs {
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    #[arg(long, default_value = DEFAULT_DB)]
    pub(super) db: PathBuf,
    #[arg(long, default_value = ".")]
    pub(super) repo: PathBuf,
    /// Verify instead of writing a new anchor.
    #[arg(long)]
    pub(super) check: bool,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct UiArgs {
    #[arg(long, default_value = DEFAULT_DB)]
    pub(super) db: PathBuf,
    #[arg(long, default_value_t = 8383)]
    pub(super) port: u16,
    /// Open the system browser after binding.
    #[arg(long)]
    pub(super) open: bool,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct TuiArgs {
    /// Full run id, a unique run-id prefix, or `latest`; opens
    /// directly at that run's level.
    #[arg(long)]
    pub(super) run: Option<String>,
    /// The world's map — the journal it names is the one opened
    /// (default ./realms.json when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without
    /// either, .forge/forge.db as always.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct DoctorArgs {
    #[arg(long)]
    pub(super) bundle: Option<PathBuf>,
    /// The world's map whose realm house declarations doctor checks
    /// (default ./realms.json when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    #[arg(long, default_value = DEFAULT_DB)]
    pub(super) db: PathBuf,
    /// Operator-side secrets store, so doctor can say which declared
    /// credentials a route is taking from the ambient environment
    /// instead (decision 0036 ruling 5).
    #[arg(long, default_value = DEFAULT_SECRETS)]
    pub(super) secrets_file: PathBuf,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct CompileArgs {
    #[arg(long)]
    pub(super) bundle: PathBuf,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct RunArgs {
    #[arg(long)]
    pub(super) bundle: Option<PathBuf>,
    /// Named recipe, resolved to <recipes-dir>/<name>.
    #[arg(long)]
    pub(super) recipe: Option<String>,
    #[arg(long, default_value = "recipes")]
    pub(super) recipes_dir: PathBuf,
    #[arg(long)]
    pub(super) feature: String,
    /// The world's map: realms and the journal they share (decision
    /// 0023). Defaults to ./realms.json when there is one; a map
    /// named here and missing or malformed is a refusal, never a
    /// silent fallback.
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without
    /// either, .forge/forge.db as always.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
    #[arg(long)]
    pub(super) repo: Option<PathBuf>,
    /// Canonical forge-dispatch/v2 JSON. When present the run id,
    /// Looper/grant correlation, recipe, repository, budget, and producer
    /// bounds are pinned into an immutable run-manifest/v2.
    #[arg(long)]
    pub(super) dispatch: Option<PathBuf>,
    /// Operator-side secrets store for seats with declared bindings
    /// (default <workdir>/.forge/secrets.env).
    #[arg(long)]
    pub(super) secrets_file: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct ResumeArgs {
    #[arg(long)]
    pub(super) bundle: Option<PathBuf>,
    /// Named recipe, resolved to <recipes-dir>/<name>.
    #[arg(long)]
    pub(super) recipe: Option<String>,
    #[arg(long, default_value = "recipes")]
    pub(super) recipes_dir: PathBuf,
    #[arg(long)]
    pub(super) run: String,
    #[arg(long, default_value = DEFAULT_DB)]
    pub(super) db: PathBuf,
    #[arg(long)]
    pub(super) repo: Option<PathBuf>,
    /// Operator-side secrets store for seats with declared bindings
    /// (default <workdir>/.forge/secrets.env).
    #[arg(long)]
    pub(super) secrets_file: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct RerunArgs {
    /// The source run whose feature is re-run.
    #[arg(long)]
    pub(super) run: String,
    #[arg(long)]
    pub(super) bundle: Option<PathBuf>,
    /// Named recipe, resolved to <recipes-dir>/<name>.
    #[arg(long)]
    pub(super) recipe: Option<String>,
    #[arg(long, default_value = "recipes")]
    pub(super) recipes_dir: PathBuf,
    #[arg(long, default_value = DEFAULT_DB)]
    pub(super) db: PathBuf,
    #[arg(long)]
    pub(super) repo: Option<PathBuf>,
    /// Operator-side secrets store for seats with declared bindings
    /// (default <workdir>/.forge/secrets.env).
    #[arg(long)]
    pub(super) secrets_file: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct CompareArgs {
    pub(super) run_a: String,
    pub(super) run_b: String,
    #[arg(long, default_value = DEFAULT_DB)]
    pub(super) db: PathBuf,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct ConcludeArgs {
    #[arg(long)]
    pub(super) run: String,
    #[arg(long)]
    pub(super) reason: String,
    #[arg(long, default_value = DEFAULT_DB)]
    pub(super) db: PathBuf,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct OperatorArgs {
    #[arg(long)]
    pub(super) run: String,
    /// "retry" re-runs the current phase; "stop" ends the run;
    /// "supersede" records that residual findings on a run that has
    /// already finished are closed by another run (decision 0047).
    pub(super) command: String,
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
    /// supersede only: the world's map, so a citation may name a
    /// run in another hearth.
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct InspectArgs {
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    /// The world's map — the journal it names is the one opened
    /// (default ./realms.json when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without
    /// either, .forge/forge.db as always.
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
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct SeatsArgs {
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    /// The world's map — the journal it names is the one opened
    /// (default ./realms.json when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without
    /// either, .forge/forge.db as always.
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
    /// The world's map — the journal it names is the one opened
    /// (default ./realms.json when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without
    /// either, .forge/forge.db as always.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
    /// Print one frame and exit.
    #[arg(long)]
    pub(super) once: bool,
    /// Poll interval in milliseconds (floored at 100).
    #[arg(long = "interval", default_value_t = 750)]
    pub(super) interval_ms: u64,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct ReplayArgs {
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    #[arg(long, default_value = DEFAULT_DB)]
    pub(super) db: PathBuf,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct ExportArgs {
    /// Full run id, a unique run-id prefix, or `latest`.
    #[arg(long)]
    pub(super) run: String,
    #[arg(long, default_value = ".")]
    pub(super) out: PathBuf,
    /// The world's map — the journal it names is the one opened
    /// (default ./realms.json when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without
    /// either, .forge/forge.db as always.
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
    /// The world's map — the journal it names is the one opened
    /// (default ./realms.json when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The destination journal. Outranks the map's journal; without
    /// either, .forge/forge.db as always.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct VerifyRunArgs {
    pub(super) file: PathBuf,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct BridgeArgs {
    #[arg(long)]
    pub(super) run: String,
    #[arg(long, default_value = DEFAULT_DB)]
    pub(super) db: PathBuf,
    #[arg(long)]
    pub(super) looper_url: String,
    #[arg(long, default_value = "LOOPER_API_KEY")]
    pub(super) token_env: String,
    /// Keep tailing the verified journal and command feed.
    #[arg(long)]
    pub(super) follow: bool,
    #[arg(long, default_value_t = 750)]
    pub(super) interval_ms: u64,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct RunsArgs {
    /// The world's map — the journal it names is the one opened
    /// (default ./realms.json when present).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without
    /// either, .forge/forge.db as always.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
    /// Emit the view model verbatim — this is what scripts read.
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct RealmsArgs {
    /// The map to read (default ./realms.json).
    #[arg(long)]
    pub(super) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without
    /// either, .forge/forge.db as always.
    #[arg(long)]
    pub(super) db: Option<PathBuf>,
    /// Emit the view model verbatim — this is what scripts read.
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(clap::Args)]
#[group(skip)]
pub(super) struct DriverArgs {
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
