use std::path::PathBuf;

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct CompileArgs {
    /// The bundle directory to validate, compiled against the workspace.
    #[arg(long)]
    pub(crate) bundle: PathBuf,
}
