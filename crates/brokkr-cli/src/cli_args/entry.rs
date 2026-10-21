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
mod tests {
    use crate::Cli;
    use clap::{error::ErrorKind, Parser};

    #[test]
    fn entry_flags_belong_to_run_and_continues_requires_enter() {
        for args in [
            vec![
                "brokkr",
                "queue",
                "add",
                "--feature",
                "synthetic",
                "--enter",
                "verify",
            ],
            vec![
                "brokkr",
                "queue",
                "add",
                "--feature",
                "synthetic",
                "--continues",
                "latest",
            ],
        ] {
            assert_eq!(
                Cli::try_parse_from(args).err().unwrap().kind(),
                ErrorKind::UnknownArgument
            );
        }
        assert_eq!(
            Cli::try_parse_from([
                "brokkr",
                "run",
                "--feature",
                "synthetic",
                "--continues",
                "latest"
            ])
            .err()
            .unwrap()
            .kind(),
            ErrorKind::MissingRequiredArgument
        );
        assert!(Cli::try_parse_from([
            "brokkr",
            "run",
            "--feature",
            "synthetic",
            "--enter",
            "verify"
        ])
        .is_ok());
    }
}
