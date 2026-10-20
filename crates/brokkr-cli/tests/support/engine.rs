//! `brokkr` started by a test as an engine. A run's start reaps the dead
//! hands trees of its TMPDIR and says each on its stderr (#415), so an
//! engine on the shared TMPDIR says a tree another test left there, and a
//! test that reads its run's stderr exactly reads that line too (#646).
//! Each engine here gets a TMPDIR of its own, and a HOME of its own, in
//! which its dispatches admit their attempts, never the operator's
//! (decision 0065 slice two, U8a2a).

use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The attempt deadline, in seconds, of a seat a test lets hang until its
/// kill. An attempt's bound is also its admission's: dispatch admits it
/// before anything spawns, within `began + min(STARTUP, bound)`, and
/// `STARTUP` is brokkr-runtime's 30 s (`engine/broker.rs`). A bound of
/// 1 or 2 s left admission one or two seconds, which admission on a
/// loaded host outlasted. The attempt then failed before its driver
/// started and was retried, so the test counted two attempts (#794). At
/// `STARTUP`, admission gets the whole startup bound, as it does for any
/// seat of 30 s or more, and the deadline under test no longer shortens
/// it. It costs each hanging seat 30 s before its kill.
pub(crate) const HANG_DEADLINE_SECONDS: u64 = 30;

/// `brokkr` run in `dir`, which is its TMPDIR too: its start reaps only
/// the trees of `dir`. Its HOME is [`home_in`] `dir`, unless the test
/// gives it another. `dir` is marked a recipe library first ([`library`]).
pub(crate) fn brokkr_in(dir: &Path) -> Command {
    library(dir);
    let mut command = Command::new(env!("CARGO_BIN_EXE_brokkr"));
    command
        .current_dir(dir)
        .env("TMPDIR", dir)
        .env("HOME", home_in(dir));
    command
}

/// `dir/home`, owner-only, made where it is missing: the HOME a test gives
/// every `brokkr` it launches on a path that can dispatch (`run`,
/// `resume`, `rerun`), unless it gives it another, so its dispatches admit
/// their attempts in a coordination root of the test's own, never under
/// the operator's HOME (decision 0065 slice two, U8a2a). `dir` is the
/// test's own, but may be the tree a run operates: [`brokkr_in`] stands in
/// it, and some suites hand it as `--repo` too. That HOME is then inside
/// the operated tree, which only the fence standing with nothing at stake
/// admits (the operator's first ruling of 2026-10-18); a test that puts
/// evidence at stake gives a HOME beside its repository instead, as
/// `brokkr_folder.rs` does (L4 of run four's second review).
pub(crate) fn home_in(dir: &Path) -> PathBuf {
    let home = dir.join("home");
    let mut made = std::fs::DirBuilder::new();
    made.recursive(true).mode(0o700).create(&home).unwrap();
    home
}

/// [`home_in`] `dir`, each toolchain directory a shipped recipe's hands
/// bind from HOME (`~/.cargo`, `~/.rustup`, `~/.npm`) linked in it to the
/// one under this process's HOME where that stands: the box still binds
/// the toolchain, and the coordination root is still the test's own.
pub(crate) fn toolchain_home_in(dir: &Path) -> PathBuf {
    let home = home_in(dir);
    let operator = std::env::var_os("HOME").map(PathBuf::from);
    for tool in [".cargo", ".rustup", ".npm"] {
        let own = operator.as_ref().map(|operator| operator.join(tool));
        let linked = home.join(tool);
        let missing = std::fs::symlink_metadata(&linked).is_err();
        if let Some(own) = own.filter(|own| own.exists() & missing) {
            std::os::unix::fs::symlink(own, linked).unwrap();
        }
    }
    home
}

/// Run in the fixture's environment and preserve the complete failure
/// summary in libtest's captured output, including an unexpected park's
/// reason on stdout. Child stderr remains unchanged for protocol proofs.
pub(crate) fn completed_in(dir: &Path, args: &[&str]) -> std::process::Output {
    let out = brokkr_in(dir).args(args).output().unwrap();
    if !out.status.success() {
        eprintln!(
            "brokkr {args:?}: {}\nstdout: {}\nstderr: {}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    out
}

/// Mark `dir` a recipe library (decision 0083 ruling 4), so a run verb
/// standing in it keeps the library's own `.forge/` layout: a test's bare
/// temporary directory has neither a `.brokkr/` folder nor the mark, and
/// there a run's state has no place. `dir` comes back, to stand in.
pub(crate) fn library(dir: &Path) -> &Path {
    let mark = dir.join("recipes/mark");
    std::fs::create_dir_all(&mark).unwrap();
    std::fs::write(mark.join("bundle.json"), "{}").unwrap();
    dir
}

/// [`brokkr_in`] `dir` given `args`, run to its end: what it printed on
/// stdout and on stderr.
pub(crate) fn printed_in(dir: &Path, args: &[&str]) -> (String, String) {
    let out = completed_in(dir, args);
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
