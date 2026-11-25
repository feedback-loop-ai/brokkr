//! `brokkr queue dispatch`'s arguments (decision 0110 rulings 3, 5 and 6): the
//! standing loop's interval, one pass with `--once`, the hidden `drive`
//! the pass spawns for each run it starts, and the bound's command and the
//! holder's readout.

use std::num::NonZeroU32;

use super::JournalArgs;

#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct DispatchArgs {
    /// Run one pass and return without waiting on the drivers it started.
    #[arg(long)]
    pub(crate) once: bool,
    /// Without `--once`, the seconds from one pass's start to the next,
    /// from 5 to 3600. The dispatcher stands until SIGINT or SIGTERM.
    #[arg(
        long,
        value_name = "SECONDS",
        default_value_t = 60,
        value_parser = clap::value_parser!(u64).range(5..=3600),
        conflicts_with = "once"
    )]
    pub(crate) every: u64,
    #[command(flatten)]
    pub(crate) journal: JournalArgs,
    #[command(subcommand)]
    pub(crate) command: Option<DispatchCmd>,
}

#[derive(clap::Subcommand)]
pub(crate) enum DispatchCmd {
    /// A queued run's driver: resume the run the entry is linked to under
    /// the claim, once its `spawn` row has landed. The dispatcher spawns
    /// it, and a run whose driver is gone is resumed by the same command,
    /// which a dispatcher's line gives.
    #[command(hide = true)]
    Drive(DriveArgs),
    /// Set the journal's bound on the refusals counted against one entry
    /// before the dispatcher holds it, a whole number of at least 1,
    /// journaled as the operator's command.
    SetBound(SetBoundArgs),
    /// Say whether a dispatcher holds the journal, the bound in force, and
    /// each entry left launching or linked with no `spawn` row. Opens the
    /// journal read-only and writes nothing.
    Status(StatusArgs),
}

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct DriveArgs {
    #[command(flatten)]
    pub(crate) journal: JournalArgs,
    /// The queue entry whose run is driven.
    #[arg(long)]
    pub(crate) entry: i64,
    /// The seq of the claim the entry is linked under.
    #[arg(long)]
    pub(crate) claim: i64,
}

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct SetBoundArgs {
    /// The bound: counted refusals of one entry before it is held.
    pub(crate) bound: NonZeroU32,
    /// Why the bound is set, journaled with it.
    #[arg(long)]
    pub(crate) reason: String,
    #[command(flatten)]
    pub(crate) journal: JournalArgs,
}

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct StatusArgs {
    #[command(flatten)]
    pub(crate) journal: JournalArgs,
}
