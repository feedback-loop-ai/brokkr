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
