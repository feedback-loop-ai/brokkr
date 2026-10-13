//! `brokkr` started by a test as an engine. A run's start reaps the dead
//! hands trees of its TMPDIR and says each on its stderr (#415), so an
//! engine on the shared TMPDIR says a tree another test left there, and a
//! test that reads its run's stderr exactly reads that line too (#646).
//! Each engine here gets a TMPDIR of its own.

use std::path::Path;
use std::process::Command;

/// `brokkr` run in `dir`, which is its TMPDIR too: its start reaps only
/// the trees of `dir`.
pub(crate) fn brokkr_in(dir: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_brokkr"));
    command.current_dir(dir).env("TMPDIR", dir);
    command
}

/// [`brokkr_in`] `dir` given `args`, run to its end: what it printed on
/// stdout and on stderr.
pub(crate) fn printed_in(dir: &Path, args: &[&str]) -> (String, String) {
    let out = brokkr_in(dir).args(args).output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// `brokkr run` in `dir` of its bundle `dir/bundle` as `feature`,
/// journaled in `db`: the run it said it started, and its whole stderr.
pub(crate) fn run_in(dir: &Path, feature: &str, db: &Path) -> (String, String) {
    let bundle = dir.join("bundle");
    let (bundle, db) = (bundle.to_str().unwrap(), db.to_str().unwrap());
    let args = ["run", "--bundle", bundle, "--feature", feature, "--db", db];
    let (_, stderr) = printed_in(dir, &args);
    let run = stderr
        .lines()
        .find_map(|line| line.strip_prefix("run started: "))
        .unwrap_or_else(|| panic!("run id on stderr: {stderr}"))
        .trim()
        .to_string();
    (run, stderr)
}

/// A pid no process holds: a child that has exited and been waited for.
pub(crate) fn dead_pid() -> u32 {
    let mut child = Command::new("true").spawn().unwrap();
    child.wait().unwrap();
    child.id()
}
