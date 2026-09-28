//! Every code the `brokkr` binary exits with, held once (#362). A verb
//! answers with an [`Exit`], never with a number of its own, and the
//! table in `docs/reference/cli.md` is rendered from this enum by the
//! reference test, so the code and the page cannot disagree.

use std::process::ExitCode;

use brokkr_core::fold::Status;

/// How an invocation ended, as its exit code says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Exit {
    /// The command did what it was asked; a driven run completed.
    Completed,
    /// The command failed: an error, a refused operator command, an
    /// unhealthy `doctor`, an unreadable transcript, a Muninn reading
    /// with nothing usable to record.
    Failed,
    /// The run was still running when the command stopped following it.
    Running,
    /// The run parked and awaits the operator.
    Parked,
    /// The run stopped.
    Stopped,
    /// A peer held the shared journal's write lock when this process ran
    /// out of patience for it. Its own code because it is its own thing:
    /// nothing was written, nothing is wrong, and the same command run
    /// again is likely to land.
    Contended,
    /// The command line did not parse. clap's own code, which is also
    /// [`Exit::Parked`]'s: a script tells them apart by stderr.
    Usage,
    /// The dsh sandbox runner could not build or start bubblewrap.
    RunnerFailed,
    /// `brokkr hands exec`: the boxed command's own code, passed through.
    Boxed(u8),
}

impl Exit {
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
        }
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
