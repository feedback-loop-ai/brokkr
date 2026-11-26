//! The command tree an operator types (decision 0113): the six daily verbs
//! at the root, families holding the rest by what each verb acts on,
//! and the engine's own verbs hidden beside them. Every verb's arguments
//! are still built by its own `Args` type, one builder per verb, as the
//! parent module says, and each family is one more builder of its own.
//!
//! A parsed tree folds into the one [`Cmd`] the dispatcher matches, so a
//! handler never knows which path reached it. Parsing what was typed is
//! kept apart from deciding what it does (decision 0071 ruling 10).

use clap::{ArgGroup, Parser, Subcommand};

use super::*;
use crate::verbs::{analytics, backup, evidence, graph, setup};
use crate::{AgentsCmd, Cli, Cmd, KeepRefsCmd, MuninnCmd, RecipesCmd};

/// The command line as typed. Its exit codes are [`crate::exit::Exit`]'s.
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
struct Tree {
    #[command(subcommand)]
    command: Root,
}

#[derive(Subcommand)]
enum Root {
    /// Scaffold a minimal reviewable bundle and prove it compiles.
    Init(InitArgs),
    /// Start a new run and drive it until it parks or finishes.
    #[command(group(ArgGroup::new("delivery").args(["bundle", "recipe"])))]
    Run(RunArgs),
    /// Resume an existing run under its exact pinned bundle.
    #[command(group(ArgGroup::new("delivery").args(["bundle", "recipe"])))]
    Resume(ResumeArgs),
    /// Watch a run live: redraw the graph, seats, last ruling and seat
    /// activity whenever the journal head moves; exit when the run
    /// reaches a terminal status. Read-only, like every other readout.
    Watch(WatchArgs),
    /// Explore runs interactively in the terminal: the fleet as a
    /// navigable table, one run's phase graph, seats and decision
    /// trail, and one seat's own checkpoint and session stream.
    /// Read-only like every other readout — it issues
    /// no operator commands and writes nothing to the journal.
    Tui(TuiArgs),
    /// Verify tools, drivers, the workspace database, and optionally a
    /// bundle, without executing any agent.
    Doctor(DoctorArgs),
    /// Print the shell completion script for bash, zsh, fish, powershell
    /// or elvish: every visible verb, flag and fixed value. `source
    /// <(COMPLETE=bash brokkr)` registers the live completer instead, which
    /// also offers run ids, phases, seats, recipes, agents, realms and
    /// queue entries.
    Completions(crate::completion::CompletionsArgs),
    /// The runs in the journal: list them (the default), read one,
    /// compare two, re-run, close or command one, and the overseer that
    /// reads them all.
    Runs(Box<RunsFamily>),
    /// The journal itself: export, import and verify one run, back up and
    /// restore the whole journal, and the evidence that outlives it.
    Journal {
        #[command(subcommand)]
        command: JournalCmd,
    },
    /// The libraries a bundle is built from: recipes and agents, and the
    /// compiler that validates a bundle.
    Library {
        #[command(subcommand)]
        command: LibraryCmd,
    },
    #[command(flatten)]
    QueueAndOrders(QueueAndOrders),
    /// What the runs did, read across them: analytics, a recipe's graph,
    /// a run's costs and its delivery ledger.
    Stats {
        #[command(subcommand)]
        command: StatsCmd,
    },
    /// This host and its world: the realms map, the secrets store, a
    /// harness probe and the read-only web surface.
    System {
        #[command(subcommand)]
        command: SystemCmd,
    },
    #[command(flatten)]
    Engine(EngineCmd),
}

// These adjacent roots share a fold so the root match stays within its ceiling.
#[derive(Subcommand)]
enum QueueAndOrders {
    /// The dispatcher's queue: runs waiting to start, in
    /// the journal's own database. Add, list, judge, move, hold, release,
    /// re-pin and drop entries; each change is journaled with its reason.
    /// `dispatch` starts what admission admits, and `sweep` frees what the
    /// finished runs left behind.
    Queue {
        #[command(subcommand)]
        command: QueueFamily,
    },
    /// Work orders in the journal: list them (the default), show one,
    /// propose or edit its text, or record the operator's ruling.
    Orders(OrdersFamily),
}

/// `brokkr runs`: bare, it lists runs, as it did before it was a family.
#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
struct RunsFamily {
    #[command(flatten)]
    list: RunsArgs,
    #[command(subcommand)]
    command: Option<RunsCmd>,
}

#[derive(Subcommand)]
enum RunsCmd {
    /// List runs in the workspace database: one clamped line per run, the
    /// fleet's sections in order and each by last activity, read a page at
    /// a time. `--json` emits the view model for scripts.
    List(RunsArgs),
    /// Explain a run: header, ruling, seats, decision trail, and the
    /// phase graph as a tree. `--phase` and `--seat` are the scoping
    /// verbs the console's clicks became; `--json` emits the view model,
    /// or with `--capability-call` the call and its verified response.
    #[command(group(ArgGroup::new("scope").args(["phase", "seat"])))]
    Inspect(InspectArgs),
    /// The seats of a run: the seats block `runs inspect` renders —
    /// every seat's model with the boundary its hands stood behind beside
    /// it — from the same view. `--json` prints
    /// that view verbatim, the bytes `runs inspect --json` prints.
    Seats(SeatsArgs),
    /// Read one participant's retained local transcript — Claude, Codex or
    /// DSH — through the same bounded local derivation the TUI uses. The
    /// verb never launches, retries or resumes a provider and writes
    /// nothing to the journal.
    Transcript(TranscriptArgs),
    /// Compare two runs' aligned outcomes: decision trails, first
    /// divergence, phases visited, per-seat costs. Read-only.
    Compare(CompareArgs),
    /// Rebuild state from the journal twice and verify determinism.
    Replay(ReplayArgs),
    /// Re-run a past run's feature as a NEW run under another bundle or
    /// recipe, so outcomes can be compared by run id. No stored linkage.
    #[command(group(ArgGroup::new("delivery").args(["bundle", "recipe"])))]
    Rerun(RerunArgs),
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
    /// of being closed over. Check `brokkr runs` first.
    // The fenced write is decision 0029's.
    Conclude(ConcludeArgs),
    /// Record an operator command (retry | stop | release | supersede |
    /// classify | repair) as journal events, or acknowledge a quarantined
    /// run beside its journal.
    Operator(Box<OperatorArgs>),
    /// The standing overseer: read the fleet, propose to
    /// the operator, execute nothing. It opens the journal read-only,
    /// issues no operator command, starts no run, and records every
    /// proposal — with the run ids and sequence numbers it was derived
    /// from — in its own append-only file beside the journal.
    Muninn {
        #[command(subcommand)]
        command: MuninnCmd,
    },
}

#[derive(Subcommand)]
enum JournalCmd {
    /// Write the canonical NDJSON journal and pinned manifest.
    Export(ExportArgs),
    /// Adopt an exported run into this journal, byte-identically — the
    /// verb paired with `journal export`. Journals never merge; one run
    /// relocates. Nothing lands unless the whole chain verifies, the
    /// events fold, the run_id does not already exist here, and the
    /// export is not a redacted derivative.
    Import(ImportArgs),
    /// Verify an exported journal offline: chain, envelopes, fold.
    Verify(VerifyRunArgs),
    /// Back up a whole journal — one consistent snapshot taken while
    /// writers may run, plus the versioned inventory that declares its
    /// boundary and makes the backup complete.
    Backup(backup::BackupArgs),
    /// Restore a journal backup into a new location, after the inventory,
    /// the snapshot's digest, SQLite's integrity, every chain and the
    /// counts all pass. It starts nothing.
    Restore(backup::RestoreArgs),
    /// Anchor a run's journal head in refs/forge/<run> (tamper evidence),
    /// or verify the existing anchor with --check.
    Anchor(AnchorArgs),
    /// Keep-refs: the git objects a run's journal cites,
    /// held by refs/forge/keep/<run>/<sha> so a squash-merge, a branch
    /// delete and a gc cannot collect the evidence the journal names.
    /// Runs plant these themselves at conclusion; these verbs cover the
    /// runs that concluded before the mechanism existed, and the
    /// deliberate letting-go that is the operator's alone.
    KeepRefs {
        #[command(subcommand)]
        command: KeepRefsCmd,
    },
    /// List every historical evidence root, or retire one under the
    /// admission lock once its evidence is archived or discarded. The
    /// sealed retirement sits beside the historical record, which stays.
    Evidence {
        #[command(subcommand)]
        command: evidence::EvidenceCmd,
    },
}

#[derive(Subcommand)]
enum LibraryCmd {
    /// The recipe library: bundle directories as named, swappable
    /// delivery strategies.
    Recipes {
        #[command(subcommand)]
        command: RecipesCmd,
    },
    /// The agent library: one definition per agent —
    /// description, charter, an ordered chain of abstract model names,
    /// abstract tool/MCP configuration — that seats reference by name.
    Agents {
        #[command(subcommand)]
        command: AgentsCmd,
    },
    /// Validate a bundle and print its pinned manifest and digest.
    Compile(CompileArgs),
}

#[derive(Subcommand)]
enum QueueFamily {
    #[command(flatten)]
    Entries(QueueCmd),
    /// Start what admission admits from the journal's queue, each run
    /// handed to a driver of its own: one pass with
    /// `--once` under the journal's dispatch lock, the bound's command and
    /// the holder's readout.
    Dispatch(DispatchArgs),
    /// Remove the run-local seat scratch trees the journal proves free and
    /// record what each complete removal freed, and reap the hands session
    /// trees whose owners are dead. One pass under the journal's dispatch
    /// lock; `--dry-run` lists what a pass would remove, with sizes, and
    /// writes nothing.
    Sweep(SweepArgs),
}

#[derive(Subcommand)]
enum StatsCmd {
    /// What the runs did across every hearth, for a range and an optional
    /// realm: shipped, start to end, waiting on you, rework, cost and
    /// served models. `--json` emits the view model.
    Analytics(analytics::AnalyticsArgs),
    /// One recipe's state machine and its traffic over a range. `--json` emits the view model.
    Graph(graph::GraphArgs),
    /// Per-seat cost and session accounting from journal checkpoints —
    /// the LaneTally join surface (stable seat ids, journal-derived).
    Costs(CostsArgs),
    /// Render the shipper's delivery ledger from journal and repository evidence.
    Ledger(LedgerArgs),
}

#[derive(Subcommand)]
enum SystemCmd {
    /// List the world: each realm with its path, default
    /// branch and current HEAD, and the journal the world writes.
    /// Read-only, like every other readout.
    Realms(RealmsArgs),
    #[command(flatten)]
    Configuration(setup::ConfigurationCmd),
    /// Measure an agent CLI for its adapter's facts, spending its bound credentials: host only, never CI.
    Probe {
        #[command(subcommand)]
        command: ProbeCmd,
    },
    /// Serve the embedded read-only surface on loopback.
    Ui(UiArgs),
}

/// The verbs the engine and its adapters spawn (decision 0113 ruling 2):
/// hidden from `--help`, and spelled at the root exactly as before, so
/// every bundle, adapter and running engine keeps reaching them.
#[derive(Subcommand)]
enum EngineCmd {
    /// Run a built-in forge-driver/v1 adapter (claude | lanetally | codex | dsh | qoder | agy | exec).
    /// Bundles reference these as {brokkr} driver <kind> -- <extra args>.
    #[command(hide = true)]
    Driver(DriverArgs),
    /// The model's hands are one tool, and the tool runs in an empty root:
    /// serve the `workspace` tool over MCP on stdio, or run one command
    /// whole inside the same box.
    #[command(hide = true)]
    Hands {
        #[command(subcommand)]
        command: crate::HandsCommand,
    },
    /// Synchronize a Looper-bound run over the authenticated producer API.
    /// The API key is read from an environment variable and is never stored.
    #[command(hide = true)]
    Bridge(BridgeArgs),
    /// (internal) The MCP capability broker the harness starts beside
    /// `hands` from the engine's configuration.
    #[command(hide = true)]
    Broker {
        #[command(subcommand)]
        command: BrokerCmd,
    },
    /// (internal) Scripted forge-driver/v1 driver for machine proof.
    #[command(hide = true)]
    FakeDriver(FakeDriverArgs),
}

impl From<Root> for Cmd {
    fn from(root: Root) -> Cmd {
        match root {
            Root::Init(args) => Cmd::Init(args),
            Root::Run(args) => Cmd::Run(args),
            Root::Resume(args) => Cmd::Resume(args),
            Root::Watch(args) => Cmd::Watch(args),
            Root::Tui(args) => Cmd::Tui(args),
            Root::Doctor(args) => Cmd::Doctor(args),
            Root::Completions(args) => Cmd::Grouped(GroupedCmd::Completions(args)),
            Root::Runs(family) => {
                let RunsFamily { list, command } = *family;
                command.map_or(Cmd::Runs(list), Cmd::from)
            }
            Root::Journal { command } => command.into(),
            Root::Library { command } => command.into(),
            Root::QueueAndOrders(command) => command.into(),
            Root::Stats { command } => command.into(),
            Root::System { command } => command.into(),
            Root::Engine(command) => command.into(),
        }
    }
}

impl From<QueueAndOrders> for Cmd {
    fn from(command: QueueAndOrders) -> Cmd {
        match command {
            QueueAndOrders::Queue { command } => command.into(),
            QueueAndOrders::Orders(OrdersFamily { list, command }) => Cmd::Grouped(command.map_or(
                GroupedCmd::Orders(OrdersCmd::List(list)),
                GroupedCmd::Orders,
            )),
        }
    }
}

impl From<RunsCmd> for Cmd {
    fn from(command: RunsCmd) -> Cmd {
        match command {
            RunsCmd::List(args) => Cmd::Runs(args),
            RunsCmd::Inspect(args) => Cmd::Inspect(args),
            RunsCmd::Seats(args) => Cmd::Seats(args),
            RunsCmd::Transcript(args) => Cmd::Transcript(args),
            RunsCmd::Compare(args) => Cmd::Compare(args),
            RunsCmd::Replay(args) => Cmd::Replay(args),
            RunsCmd::Rerun(args) => Cmd::Rerun(args),
            RunsCmd::Conclude(args) => Cmd::Conclude(args),
            RunsCmd::Operator(args) => Cmd::Operator(*args),
            RunsCmd::Muninn { command } => Cmd::Grouped(GroupedCmd::Muninn { command }),
        }
    }
}

impl From<JournalCmd> for Cmd {
    fn from(command: JournalCmd) -> Cmd {
        match command {
            JournalCmd::Export(args) => Cmd::Export(args),
            JournalCmd::Import(args) => Cmd::Import(args),
            JournalCmd::Verify(args) => Cmd::VerifyRun(args),
            JournalCmd::Backup(args) => Cmd::Backup(args),
            JournalCmd::Restore(args) => Cmd::Restore(args),
            JournalCmd::Anchor(args) => Cmd::Anchor(args),
            JournalCmd::KeepRefs { command } => Cmd::KeepRefs { command },
            JournalCmd::Evidence { command } => Cmd::Evidence { command },
        }
    }
}

impl From<LibraryCmd> for Cmd {
    fn from(command: LibraryCmd) -> Cmd {
        match command {
            LibraryCmd::Recipes { command } => Cmd::Grouped(GroupedCmd::Recipes { command }),
            LibraryCmd::Agents { command } => Cmd::Grouped(GroupedCmd::Agents { command }),
            LibraryCmd::Compile(args) => Cmd::Compile(args),
        }
    }
}

impl From<QueueFamily> for Cmd {
    fn from(command: QueueFamily) -> Cmd {
        match command {
            QueueFamily::Entries(command) => Cmd::Queue { command },
            QueueFamily::Dispatch(args) => Cmd::Grouped(GroupedCmd::Dispatch(args)),
            QueueFamily::Sweep(args) => Cmd::Grouped(GroupedCmd::Sweep(args)),
        }
    }
}

impl From<StatsCmd> for Cmd {
    fn from(command: StatsCmd) -> Cmd {
        match command {
            StatsCmd::Analytics(args) => Cmd::CrossRun(analytics::CrossRunCmd::Analytics(args)),
            StatsCmd::Graph(args) => Cmd::CrossRun(analytics::CrossRunCmd::Graph(args)),
            StatsCmd::Costs(args) => Cmd::Costs(args),
            StatsCmd::Ledger(args) => Cmd::Ledger(args),
        }
    }
}

impl From<SystemCmd> for Cmd {
    fn from(command: SystemCmd) -> Cmd {
        match command {
            SystemCmd::Realms(args) => Cmd::Realms(args),
            SystemCmd::Configuration(command) => Cmd::Grouped(GroupedCmd::Configuration(command)),
            SystemCmd::Probe { command } => Cmd::Probe { command },
            SystemCmd::Ui(args) => Cmd::Ui(args),
        }
    }
}

impl From<EngineCmd> for Cmd {
    fn from(command: EngineCmd) -> Cmd {
        match command {
            EngineCmd::Driver(args) => Cmd::Driver(args),
            EngineCmd::Hands { command } => Cmd::Hands { command },
            EngineCmd::Bridge(args) => Cmd::Bridge(args),
            EngineCmd::Broker { command } => Cmd::Broker { command },
            EngineCmd::FakeDriver(args) => Cmd::FakeDriver(args),
        }
    }
}

// `Cli` is parsed through the tree and holds the command it folds into, so
// `Cli::try_parse_from` and `Cli::command()` answer with the tree's shape.
impl clap::CommandFactory for Cli {
    fn command() -> clap::Command {
        Tree::command()
    }

    fn command_for_update() -> clap::Command {
        Tree::command_for_update()
    }
}

impl clap::FromArgMatches for Cli {
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        Tree::from_arg_matches(matches).map(|tree| Cli {
            command: tree.command.into(),
        })
    }

    fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        *self = Cli::from_arg_matches(matches)?;
        Ok(())
    }
}

impl Parser for Cli {
    fn try_parse_from<I, T>(itr: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let argv: Vec<std::ffi::OsString> = itr.into_iter().map(Into::into).collect();
        if let Some(err) = crate::hands::refuse_encoded_option_as_value(
            &argv,
            <Self as clap::CommandFactory>::command,
        ) {
            return Err(err);
        }
        <Self as clap::CommandFactory>::command()
            .try_get_matches_from(&argv)
            .and_then(|mut matches| {
                <Self as clap::FromArgMatches>::from_arg_matches_mut(&mut matches)
            })
            .map_err(|err| {
                crate::hands::reformat_serve_clap_error(
                    err,
                    &argv,
                    <Self as clap::CommandFactory>::command,
                )
            })
    }
}

#[cfg(test)]
mod tests;
