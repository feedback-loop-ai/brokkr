//! The box's exit code and the host's git facts, read through stand-ins
//! for `bwrap` and `git` on a private PATH (#419). A real bubblewrap
//! cannot be told to die by a signal on cue, and a real git never prints
//! its directories and then fails; the stand-ins do both, so these tests
//! run on every host, namespaces or not. Before #419 only brokkr-cli's
//! `hands` suite held `run_boxed`'s code, and no suite held a signalled
//! box's `-1` or a failed `git rev-parse`.
//!
//! Every test here holds the environment guard for its whole body, which
//! also serialises them: no test forks while another is writing a
//! stand-in, so no stand-in is exec'd while a write handle is open on it.

#[path = "../../../tests/support/env_guard.rs"]
mod env_guard;

use brokkr_protocol::hands::{execute_in, git_facts, run_boxed, run_boxed_in, GitFacts, HandsSpec};
use env_guard::EnvGuard;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A `bwrap` whose box exits with the code its command names, or dies by
/// SIGKILL when the command is `signal`. The command is the last argument.
const BWRAP: &str = "#!/bin/sh\n\
for last do :; done\n\
case \"$last\" in\n\
  signal) kill -KILL $$ ;;\n\
  *) exit \"$last\" ;;\n\
esac\n";

/// A `git` that prints two absolute directories, as `rev-parse` does, and
/// then exits with `status`.
fn git(status: u8) -> String {
    format!("#!/bin/sh\nprintf '/stand-in/git-dir\\n/stand-in/common-dir\\n'\nexit {status}\n")
}

/// A directory holding the two stand-ins, for a PATH of its own.
fn stand_ins(root: &Path, git_status: u8) -> PathBuf {
    let dir = root.join(format!("bin-{git_status}"));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, text) in [("bwrap", BWRAP.to_string()), ("git", git(git_status))] {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    dir
}

fn command(text: &str) -> Vec<String> {
    vec![text.to_string()]
}

/// A box killed by a signal has no exit code, and says so as `-1`, both
/// through the bounded `execute_in` and the pass-through `run_boxed_in`.
#[test]
fn a_box_killed_by_a_signal_reports_minus_one() {
    let _env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let bwrap = stand_ins(dir.path(), 1).join("bwrap");
    let (spec, facts) = (HandsSpec::default(), GitFacts::default());
    let (home, session) = (dir.path().join("home"), dir.path().join("session"));
    let executed = execute_in(
        &bwrap,
        &spec,
        dir.path(),
        &home,
        &dir.path().join("scratch"),
        &session,
        &facts,
        "signal",
        Duration::from_secs(60),
    )
    .unwrap();
    assert_eq!((executed.exit_code, executed.timed_out), (-1, false));
    let run = |text: &str| {
        run_boxed_in(
            &bwrap,
            &spec,
            dir.path(),
            &home,
            &session,
            &facts,
            None,
            &command(text),
        )
    };
    assert_eq!(run("signal"), Ok(-1));
    assert_eq!(run("0"), Ok(0));
    assert_eq!(run("3"), Ok(3));
}

/// `run_boxed` finds bwrap on PATH and returns the box's own code.
#[test]
fn run_boxed_returns_the_exit_code_of_the_box() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    env.set("PATH", stand_ins(dir.path(), 1));
    let run = |text: &str| run_boxed(&HandsSpec::default(), dir.path(), None, &command(text));
    assert_eq!(run("0"), Ok(0));
    assert_eq!(run("3"), Ok(3));
    assert_eq!(run("signal"), Ok(-1));
}

/// A `git rev-parse` that fails names no directory, whatever it printed
/// before failing; the same output from a `git` that succeeds does.
#[test]
fn a_failed_rev_parse_names_no_git_directory() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    env.set("PATH", stand_ins(dir.path(), 0));
    let facts = git_facts(dir.path());
    assert_eq!(
        (facts.git_dir, facts.common_dir),
        (
            Some(PathBuf::from("/stand-in/git-dir")),
            Some(PathBuf::from("/stand-in/common-dir"))
        ),
        "the stand-in is the git the facts are read from"
    );
    env.set("PATH", stand_ins(dir.path(), 1));
    assert_eq!(git_facts(dir.path()), GitFacts::default());
}
