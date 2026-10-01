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
        (Exit::Signalled(15), 143),
    ];
    for (exit, code) in codes {
        assert_eq!(exit.code(), code, "{exit:?}");
        assert_eq!(ExitCode::from(exit), ExitCode::from(code), "{exit:?}");
    }
}

/// `hands serve` ended by a signal exits as a shell reports one.
#[test]
fn a_termination_signal_exits_128_plus_its_number() {
    assert_eq!(Exit::of_signal(15), 143);
    assert_eq!(Exit::of_signal(2), 130);
    assert_eq!(Exit::of_signal(-1), 1);
}

/// `hands exec` passes the box's code on, and a box a signal ended,
/// which `run_boxed` reports as `-1`, fails rather than succeeding.
#[test]
fn a_box_passes_its_code_on_and_a_signalled_box_fails() {
    assert_eq!(Exit::of_box(0), Exit::Boxed(0));
    assert_eq!(Exit::of_box(42), Exit::Boxed(42));
    assert_eq!(Exit::of_box(-1), Exit::Failed);
}

#[test]
fn a_run_status_exits_with_its_own_code() {
    assert_eq!(Exit::of_status(&Status::Completed), Exit::Completed);
    assert_eq!(Exit::of_status(&Status::AwaitingOperator), Exit::Parked);
    assert_eq!(Exit::of_status(&Status::Stopped), Exit::Stopped);
    assert_eq!(Exit::of_status(&Status::Running), Exit::Running);
    // A watch ends once its run has stopped running, and only then.
    assert_eq!(
        Exit::of_settled(Some(&Status::Stopped)),
        Some(Exit::Stopped)
    );
    assert_eq!(Exit::of_settled(Some(&Status::Running)), None);
    assert_eq!(Exit::of_settled(None), None);
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
