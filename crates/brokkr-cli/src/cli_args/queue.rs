//! The queue verb's arguments (decision 0068 ruling 1).

use super::{JournalArgs, LaunchArgs};

/// The queue's operator commands (decision 0068 ruling 1). Each one that
/// changes the queue is journaled with its reason.
#[derive(clap::Subcommand)]
pub(crate) enum QueueCmd {
    /// Queue a new run, taking `brokkr run`'s arguments, last among the
    /// entries of its priority. Nothing starts: the entry waits for the
    /// dispatcher.
    #[command(group(clap::ArgGroup::new("delivery").args(["bundle", "recipe"])))]
    Add(QueueAddArgs),
    /// The queue in order (priority, higher first, then place): each
    /// waiting entry's place, state, priority,
    /// waits, launch and admission (admissible, or why it waits or is
    /// held), then the entries that started a run, with it. `--json`
    /// emits the view model for scripts.
    List(QueueListArgs),
    /// Judge the queue as `list` shows it, and first latch on each waiting
    /// entry the realm drift found: from then on it is held until the
    /// operator re-pins, re-queues or drops it, whatever the map comes to.
    /// Each latch is journaled with the reason.
    Judge(QueueJudgeArgs),
    /// Put an entry at another place in the queue, among the entries of
    /// its own priority: priority always wins, so a place above a higher
    /// priority or below a lower one leaves it first or last among its
    /// own. Says the place it holds after.
    Move(QueueMoveArgs),
    /// Keep an entry in its place, not to be started until released.
    Hold(QueueEntryArgs),
    /// Let a held entry be started again.
    Release(QueueEntryArgs),
    /// Re-pin a waiting entry to the realms map that would govern it now:
    /// how the operator releases an entry `judge` latched a realm-drift
    /// hold on, accepting the differences it found. Refused when the map
    /// on disk is not the one the latch found; judge the queue again.
    Repin(QueueEntryArgs),
    /// Take an entry out of the queue. One that started a run cannot be.
    Drop(QueueEntryArgs),
}

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct QueueAddArgs {
    #[command(flatten)]
    pub(crate) launch: LaunchArgs,
    /// The entry's priority, which orders the queue first: a higher
    /// priority is judged and started first, and `move` orders an entry
    /// only among those of its own priority.
    #[arg(long, default_value_t = 0, allow_negative_numbers = true)]
    pub(crate) priority: i64,
    /// An earlier entry this one waits for, and on what: `3:completed`
    /// (its run completed), `3:ended` (its run ended at all), or a named
    /// ending, as `3:shipped` (its run is over and ended so; one that
    /// ended otherwise holds this entry). The named endings are shipped,
    /// shipped-flagged, refused, failed, blocked, exhausted, superseded,
    /// infrastructure, delivered-elsewhere, withdrawn and
    /// operator-stopped. Repeatable.
    #[arg(long, value_name = "ENTRY:CONDITION")]
    pub(crate) after: Vec<brokkr_store::Wait>,
    /// Why, journaled with the command.
    #[arg(long)]
    pub(crate) reason: String,
}

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct QueueListArgs {
    #[command(flatten)]
    pub(crate) journal: JournalArgs,
    /// Emit the view model verbatim — this is what scripts read.
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct QueueJudgeArgs {
    /// Why, journaled with each latch.
    #[arg(long)]
    pub(crate) reason: String,
    #[command(flatten)]
    pub(crate) list: QueueListArgs,
}

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct QueueMoveArgs {
    #[command(flatten)]
    pub(crate) entry: QueueEntryArgs,
    /// The place to put it at, from 1, as `brokkr queue list` shows it.
    #[arg(long)]
    pub(crate) to: u32,
}

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct QueueEntryArgs {
    /// The entry's id, as `brokkr queue list` prints it.
    pub(crate) entry: brokkr_store::EntryId,
    /// Why, journaled with the command.
    #[arg(long)]
    pub(crate) reason: String,
    #[command(flatten)]
    pub(crate) journal: JournalArgs,
}
