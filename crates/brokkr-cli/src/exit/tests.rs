use super::*;
use clap::Parser;

#[test]
fn every_exit_has_its_one_code() {
    let codes = [
        (Exit::Completed, 0),
        (Exit::Failed, 1),
        (Exit::Running, 1),
        (Exit::Parked, 2),
        (Exit::Usage, 2),
        (Exit::Stopped, 3),
        (Exit::Contended, 4),
        (Exit::RunnerFailed, 127),
        (Exit::Boxed(0), 0),
        (Exit::Boxed(42), 42),
    ];
    for (exit, code) in codes {
        assert_eq!(exit.code(), code, "{exit:?}");
        assert_eq!(ExitCode::from(exit), ExitCode::from(code), "{exit:?}");
    }
}

#[test]
fn a_run_status_exits_with_its_own_code() {
    assert_eq!(Exit::of_status(&Status::Completed), Exit::Completed);
    assert_eq!(Exit::of_status(&Status::AwaitingOperator), Exit::Parked);
    assert_eq!(Exit::of_status(&Status::Stopped), Exit::Stopped);
    assert_eq!(Exit::of_status(&Status::Running), Exit::Running);
}

/// A refused command line is clap's usage error; the help and the
/// version clap also reports as errors leave with success.
#[test]
fn a_command_line_clap_refuses_is_a_usage_exit_and_help_is_not() {
    let refused = crate::Cli::try_parse_from(["brokkr", "no-such-verb"])
        .err()
        .expect("an unknown verb does not parse");
    assert_eq!(Exit::of_parse(&refused), Exit::Usage);
    for asked in ["--help", "--version"] {
        let answered = crate::Cli::try_parse_from(["brokkr", asked])
            .err()
            .expect("help and version stop the parse");
        assert_eq!(Exit::of_parse(&answered), Exit::Completed, "{asked}");
    }
}
