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
/// it. It costs each hanging seat 30 s before its kill. `STARTUP` is
/// private to the runtime, so this is a copy, which
/// `engine::hang_deadline_is_the_runtime_startup` pins equal to the
/// runtime's own declaration (#803).
pub(crate) const HANG_DEADLINE_SECONDS: u64 = 30;

/// brokkr-runtime's `engine/broker.rs`, the one home of `STARTUP`.
/// `include_str!` reads a path from this file's directory: three levels
/// up from `crates/brokkr-cli/tests/support/` is `crates/`, where the
/// runtime is the cli's sibling member of the workspace, as every
/// member is.
const RUNTIME_BROKER: &str = include_str!("../../../brokkr-runtime/src/engine/broker.rs");

/// The seconds of the one `STARTUP` `source` declares, read off its line
/// `const STARTUP: Duration = Duration::from_secs(N);` in the code alone,
/// each comment and literal read as a space ([`crate::rust_source::code`]).
/// A line naming `const STARTUP` in any other shape, no such line, two, or
/// a source the lexer cannot read, fails the pin, never passes unread.
fn startup_seconds(source: &str) -> u64 {
    let code = crate::rust_source::code(source).unwrap_or_else(|unread| {
        panic!("brokkr-runtime's engine/broker.rs cannot be read as Rust source: {unread}")
    });
    let declared: Vec<&str> = code
        .lines()
        .filter(|line| {
            let words: Vec<&str> = line
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .filter(|word| !word.is_empty())
                .collect();
            words.windows(2).any(|pair| pair == ["const", "STARTUP"])
        })
        .collect();
    let [line] = declared[..] else {
        panic!(
            "brokkr-runtime's engine/broker.rs names `const STARTUP` on {} lines, where the \
             pin reads exactly one: {declared:?}",
            declared.len()
        );
    };
    line.strip_prefix("const STARTUP: Duration = Duration::from_secs(")
        .and_then(|rest| rest.strip_suffix(");"))
        .filter(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|digits| digits.parse().ok())
        .unwrap_or_else(|| {
            panic!(
                "brokkr-runtime's engine/broker.rs declares `STARTUP` in a shape the pin does \
                 not read, not `const STARTUP: Duration = Duration::from_secs(N);`: {line}"
            )
        })
}

#[test]
fn hang_deadline_is_the_runtime_startup() {
    let startup = startup_seconds(RUNTIME_BROKER);
    assert_eq!(
        HANG_DEADLINE_SECONDS, startup,
        "HANG_DEADLINE_SECONDS is {HANG_DEADLINE_SECONDS} s and brokkr-runtime's STARTUP is \
         {startup} s: a seat that hangs to its kill takes the whole startup bound (#794)"
    );
}

/// A declaration a comment holds is never read: a block comment, nested
/// or not, a line comment and a doc comment each hide their copy, and the
/// one live line is read (#803).
#[test]
fn a_commented_startup_is_never_read() {
    let source = "/* const STARTUP: Duration = Duration::from_secs(30);\n\
                  /* const STARTUP: Duration = Duration::from_secs(31); */\n\
                  const STARTUP: Duration = Duration::from_secs(32);\n*/\n\
                  // const STARTUP: Duration = Duration::from_secs(33);\n\
                  /// const STARTUP: Duration = Duration::from_secs(34);\n\
                  const STARTUP: Duration = Duration::from_secs(60);\n";
    assert_eq!(startup_seconds(source), 60);
}

/// A comment splitting the live declaration with a space either side
/// leaves it a shape the pin does not read: it is refused, never read past
/// for the commented copy beside it (#803).
#[test]
#[should_panic(expected = "declares `STARTUP` in a shape the pin does not read")]
fn a_declaration_a_comment_splits_is_refused() {
    startup_seconds(
        "/*\nconst STARTUP: Duration = Duration::from_secs(30);\n*/\n\
         const /* startup bound */ STARTUP: Duration = Duration::from_secs(60);\n",
    );
}

/// A comment with no space beside it reads as the one space it stands
/// for, so the live declaration it splits is read, and the commented copy
/// beside it never is (#803).
#[test]
fn a_comment_between_two_words_reads_as_a_space() {
    let source = "/*\nconst STARTUP: Duration = Duration::from_secs(30);\n*/\n\
                  const/* startup bound */STARTUP: Duration = Duration::from_secs(60);\n";
    assert_eq!(startup_seconds(source), 60);
}

/// A literal is never code: a comment marker inside a string opens no
/// comment, so it hides no live declaration and exposes no commented copy
/// (#803).
#[test]
fn a_literal_neither_hides_nor_exposes_a_declaration() {
    let source = "const PATTERN: &str = \"logs/*\";\n\
                  const STARTUP: Duration = Duration::from_secs(60);\n\
                  // */const STARTUP: Duration = Duration::from_secs(30);\n\
                  const COPY: &str = r#\"\nconst STARTUP: Duration = Duration::from_secs(31);\n\"#;\n";
    assert_eq!(startup_seconds(source), 60);
}

/// A source the lexer cannot read is refused, never read in part (#803).
#[test]
#[should_panic(
    expected = "brokkr-runtime's engine/broker.rs cannot be read as Rust source: an \
                unterminated block comment"
)]
fn an_unreadable_source_is_refused() {
    startup_seconds("const STARTUP: Duration = Duration::from_secs(30);\n/* never closed\n");
}

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
/// the operated tree, which an unboxed writer with nothing at stake may
/// share (ruling 21 of 2026-10-29); a test that boxes a writer of that tree,
/// or puts evidence at stake, gives a HOME beside it instead, as
/// `brokkr_folder.rs` does (the operator's ruling of 2026-10-30).
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
