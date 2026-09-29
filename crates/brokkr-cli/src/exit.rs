//! Every code the `brokkr` binary exits with, held once (#362). A verb
//! answers with an [`Exit`], never with a number of its own, and the
//! table in `docs/reference/cli.md` is rendered from this enum by the
//! reference test, so the code and the page cannot disagree.

use std::process::ExitCode;

use brokkr_core::fold::Status;

/// How an invocation ended, as its exit code says it. What each variant
/// means is written once, in [`Exit::meaning`], the text the reference
/// page's table prints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Exit {
    Completed,
    Failed,
    Running,
    Parked,
    Stopped,
    Contended,
    Usage,
    RunnerFailed,
    Boxed(u8),
    /// `hands serve` ended by a termination signal, carrying its number.
    Signalled(u8),
}

impl Exit {
    /// What the exit tells its caller. Only the reference page reads it,
    /// so it is built with the tests that render the page.
    #[cfg(test)]
    pub(crate) const fn meaning(self) -> &'static str {
        match self {
            Exit::Completed => "The command did what it was asked; a driven run completed.",
            Exit::Failed => "An error, a refused operator command, an unhealthy `doctor`, an unreadable transcript, or a Muninn reading with nothing usable to record.",
            Exit::Running => "The run was still running when the command stopped following it.",
            Exit::Parked => "The run parked and awaits the operator.",
            Exit::Stopped => "The run stopped.",
            Exit::Contended => "A peer held the shared journal's write lock. Nothing was written; the same command run again is likely to land.",
            Exit::Usage => "The command line did not parse (clap's own code, shared with `parked`: stderr tells them apart).",
            Exit::RunnerFailed => "The dsh sandbox runner could not build or start bubblewrap.",
            Exit::Boxed(_) => "`hands exec`: the boxed command's own exit code, passed through. A box a signal ended has no code of its own, and exits 1 (`failed`).",
            Exit::Signalled(_) => "`hands serve` ended by a termination signal: its session tree is removed, and it exits 128 plus the signal's number, as a shell reports a signal death (143 for SIGTERM).",
        }
    }

    /// The number the process exits with.
    pub(crate) const fn code(self) -> u8 {
        match self {
            Exit::Completed => 0,
            Exit::Failed | Exit::Running => 1,
            Exit::Parked | Exit::Usage => 2,
            Exit::Stopped => 3,
            Exit::Contended => 4,
            Exit::RunnerFailed => 127,
            Exit::Boxed(code) => code,
            Exit::Signalled(signal) => 128u8.saturating_add(signal),
        }
    }

    /// The code `hands serve` exits with when `signal` ends it, as
    /// `Session::remove_on_termination` asks for it. A signal number is
    /// never negative; one that were would read as a failure.
    pub(crate) fn of_signal(signal: i32) -> i32 {
        u8::try_from(signal)
            .map_or(Exit::Failed, Exit::Signalled)
            .code()
            .into()
    }

    /// The exit `hands exec` passes on for the box's `code`: its own, or
    /// a failure for the `-1` of a box a signal ended, which has none.
    pub(crate) fn of_box(code: i32) -> Exit {
        u8::try_from(code).map_or(Exit::Failed, Exit::Boxed)
    }

    /// A run's status as the code its driving or watching verb exits with.
    pub(crate) const fn of_status(status: &Status) -> Exit {
        match status {
            Status::Completed => Exit::Completed,
            Status::AwaitingOperator => Exit::Parked,
            Status::Stopped => Exit::Stopped,
            Status::Running => Exit::Running,
        }
    }

    /// How a command line that clap refused leaves: its message printed
    /// the way `Cli::parse` would print it, and `--help` or `--version`,
    /// which clap also reports as an error, still a success.
    pub(crate) fn of_parse(error: &clap::Error) -> Exit {
        // A closed stdout or stderr leaves nothing to report the failure
        // on; the exit code still says what happened.
        let _ = error.print();
        if error.use_stderr() {
            Exit::Usage
        } else {
            Exit::Completed
        }
    }
}

impl From<Exit> for ExitCode {
    fn from(exit: Exit) -> ExitCode {
        ExitCode::from(exit.code())
    }
}

#[cfg(test)]
mod tests;
