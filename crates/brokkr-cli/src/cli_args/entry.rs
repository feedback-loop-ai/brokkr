/// Run-level recovery, deliberately absent from queued-launch/v1 (decision 0094).
#[derive(clap::Args, Default)]
#[group(skip)]
pub(crate) struct EntryArgs {
    /// Enter at an admitted gate over work already on the branch.
    #[arg(long)]
    pub(crate) enter: Option<String>,
    /// Earlier concluded run continued by this run (id, prefix, or latest); repeat in order.
    #[arg(long, requires = "enter")]
    pub(crate) continues: Vec<String>,
}

#[cfg(test)]
mod tests;
