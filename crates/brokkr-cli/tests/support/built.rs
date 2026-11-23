//! The one way these tests spawn the built `brokkr` (#1137i, Z1): every
//! command that runs it is made here or by `crate::isolated` beside it,
//! and `choke_point` refuses a test source that names the binary anywhere
//! else.
//!
//! `brokkr doctor` probes tools for their versions, and its probe delivers
//! real kills to the process group it started, so doctor runs only alone
//! in a PID namespace of its own, through `crate::isolated::brokkr`, and
//! [`brokkr`] here refuses it. `init` is not among such verbs: it spawns
//! nothing, and reaches neither doctor nor its probe (`src/verbs/setup.rs`).

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

/// The built binary, for a test that hands its path on as data: a
/// driver's argv, a staged copy, a symlink or a script that runs it.
pub(crate) const BIN: &str = env!("CARGO_BIN_EXE_brokkr");

/// The verbs whose run reaches doctor's version probe.
pub(crate) const PROBING: [&str; 1] = ["doctor"];

/// The built brokkr given `argv`, its verb first, for a verb that reaches
/// no probe; one that does is refused here, before anything runs.
pub(crate) fn brokkr<I, S>(argv: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    brokkr_at(Path::new(BIN), argv)
}

/// [`brokkr`], the binary at `program`: a copy of the built one.
pub(crate) fn brokkr_at<I, S>(program: &Path, argv: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let argv = verbed(argv);
    if let Some(verb) = PROBING.iter().find(|verb| argv[0] == **verb) {
        panic!(
            "`brokkr {verb}` reaches doctor's version probe, which delivers real kills: run it \
             through isolated::brokkr"
        );
    }
    let mut command = Command::new(program);
    command.args(argv);
    command
}

/// `argv` owned, refused when it names no verb: a verb added after the
/// command is built would escape [`brokkr`]'s refusal.
pub(crate) fn verbed<I, S>(argv: I) -> Vec<OsString>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let argv: Vec<OsString> = argv
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect();
    assert!(
        !argv.is_empty(),
        "the built brokkr is given its verb as it is built"
    );
    argv
}
