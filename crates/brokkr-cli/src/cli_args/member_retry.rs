use super::DeliveryArgs;
use std::path::PathBuf;

#[derive(clap::Args, Default)]
#[group(skip)]
pub(crate) struct MemberRetryArgs {
    /// retry only: run this panel member; repeat to run several. Sequence members use <step>:<member>.
    #[arg(long)]
    pub(crate) member: Vec<String>,
    /// With --member: the operated repository, defaulting as resume does.
    #[arg(long)]
    pub(crate) repo: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) delivery: DeliveryArgs,
}
