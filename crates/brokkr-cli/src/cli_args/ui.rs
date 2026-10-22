//! `brokkr ui`'s arguments, moved whole out of `cli_args.rs`, at its
//! 800-line ceiling, to make room for `inspect --capability-call`
//! (decision 0065 slice two, U8g; operator ruling of 2026-10-09).

use super::JournalArgs;

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct UiArgs {
    #[command(flatten)]
    pub(crate) journal: JournalArgs,
    /// The loopback port the surface is served on.
    #[arg(long, default_value_t = 8383)]
    pub(crate) port: u16,
    /// Open the system browser after binding.
    #[arg(long)]
    pub(crate) open: bool,
}
