//! The compiler keeps the built `brokkr` path private to this builder
//! (#1137i, Z1). The command, argv and shell doors refuse probing verbs.
//! Tests can stage an executable copy. The scan pins the bare path and
//! the isolated door to their callers.
//!
//! `brokkr doctor` probes tools for their versions, and its probe delivers
//! real kills to the process group it started, so doctor runs only alone
//! in a PID namespace of its own, through `crate::isolated::brokkr`, and
//! [`brokkr`] here refuses it. `init` is not among such verbs: it spawns
//! nothing, and reaches neither doctor nor its probe (`src/verbs/setup.rs`).

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_brokkr");

/// The verbs whose run reaches doctor's version probe.
pub(crate) const PROBING: [&str; 1] = ["doctor"];

/// The built brokkr given `argv`, its verb first, for a verb that reaches
/// no probe; one that does is refused here, before anything runs.
pub(crate) fn brokkr<I, S>(argv: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    brokkr_at(Path::new(bare_path()), argv)
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

/// The built path for a product field or fixture pinned by the scan.
pub(crate) fn bare_path() -> &'static str {
    BIN
}

// The shared builder also compiles in standalone test targets. The data
// doors expand only in the targets that use them.

/// The built binary and its checked arguments, as owned strings.
#[macro_export]
macro_rules! built_argv {
    ($args:expr) => {{
        let command = $crate::built::brokkr($args);
        std::iter::once(command.get_program())
            .chain(command.get_args())
            .map(|word| word.to_str().expect("UTF-8 command words").to_owned())
            .collect::<Vec<String>>()
    }};
}

/// The checked command words, each single-quoted for sh.
#[macro_export]
macro_rules! built_shell_line {
    ($args:expr) => {
        $crate::built_argv!($args)
            .into_iter()
            .map(|word| format!("'{}'", word.replace('\'', "'\\''")))
            .collect::<Vec<_>>()
            .join(" ")
    };
}

/// Copy the executable and its mode in a child that closes its write
/// descriptor before the test can run the copy.
#[macro_export]
macro_rules! stage_brokkr {
    ($path:expr) => {
        std::process::Command::new("/bin/cp")
            .args(["-p", $crate::built::bare_path()])
            .arg($path)
            .status()
            .and_then(|status| {
                if status.success() {
                    Ok(())
                } else {
                    Err(std::io::Error::other(format!(
                        "copying the binary: {status}"
                    )))
                }
            })
    };
}

/// The built path for the isolated runner, pinned to its source.
#[macro_export]
macro_rules! isolated_brokkr_path {
    () => {
        $crate::built::bare_path()
    };
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
