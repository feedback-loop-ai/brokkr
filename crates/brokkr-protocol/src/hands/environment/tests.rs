//! Every seat's git environment reaches its shell whole (#1109): the
//! workspace box's table and the unboxed dispatch's, run through the very
//! credential scrub dsh 0.2.1's subprocess layer applies to every child,
//! still read `commit.gpgsign=false`, and git runs in them unaided. The
//! dsh harness's own child is driven through the same shell body in
//! `adapters::tests::seat_git`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

use super::super::{unboxed_environment, HandsSpec};
use super::{box_environment, seat_git_environment};

/// The words dsh 0.2.1's `SENSITIVE_ENV_PATTERN`
/// (`/KEY|PASSWORD|SECRET|TOKEN/i`, `@deepseek-ai/dsh-subprocess`) drops
/// an inherited name for.
const SCRUBBED: [&str; 4] = ["KEY", "PASSWORD", "SECRET", "TOKEN"];

/// Whether a credential scrub of that pattern drops `name`.
pub(crate) fn scrubbed(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    SCRUBBED.iter().any(|word| name.contains(word))
}

/// A seat's shell: drop every name the scrub matches, as dsh drops it
/// from each child, then ask git for the signing setting and the status.
pub(crate) const SCRUB_THEN_GIT: &str = "\
for name in $(env | sed -n 's/^\\([A-Za-z_][A-Za-z0-9_]*\\)=.*/\\1/p' \
| grep -iE 'KEY|PASSWORD|SECRET|TOKEN'); do unset \"$name\"; done
git config --get commit.gpgsign 2>&1; echo \"get=$?\"
git status --porcelain 2>&1; echo \"status=$?\"
";

/// What [`SCRUB_THEN_GIT`] prints in an empty repository whose
/// environment carries the seat's git setting whole.
pub(crate) const READS_UNSIGNED: &str = "false\nget=0\nstatus=0\n";

/// An empty repository under `dir`, made with no setting of the host's.
pub(crate) fn repository(dir: &Path) -> PathBuf {
    let repo = dir.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let out = Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    repo
}

/// [`SCRUB_THEN_GIT`]'s output in `repo` under exactly `table`.
fn shell_under(repo: &Path, table: &BTreeMap<String, String>) -> String {
    let out = Command::new("/bin/sh")
        .args(["-c", SCRUB_THEN_GIT])
        .current_dir(repo)
        .env_clear()
        .envs(table)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

/// The workspace box's table and the unboxed dispatch's, for a spec
/// binding every toolchain home, with the bundle's git identity.
fn tables(dir: &Path) -> [BTreeMap<String, String>; 2] {
    let home = dir.join("home");
    let spec = HandsSpec::parse(&json!({
        "kind": "workspace",
        "binds": (["~/.cargo", "~/.rustup", "~/.npm"].map(|path| json!({"path": path, "mode": "ro"}))),
    }))
    .unwrap();
    let identity = [
        ("GIT_AUTHOR_NAME", "Seat"),
        ("GIT_AUTHOR_EMAIL", "seat@example.invalid"),
        ("GIT_COMMITTER_NAME", "Seat"),
        ("GIT_COMMITTER_EMAIL", "seat@example.invalid"),
    ]
    .map(|(key, value)| (key.to_string(), value.to_string()));
    let engine = BTreeMap::from([(
        "PATH".to_string(),
        std::env::var("PATH").unwrap_or_default(),
    )]);
    let boxed = box_environment(&spec, &home)
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .chain(identity.clone())
        .collect();
    let unboxed = unboxed_environment(&engine, &home, &spec, &identity, &home, dir);
    [boxed, unboxed]
}

/// The box's and the unboxed dispatch's shells, scrubbed as dsh scrubs a
/// child, read `commit.gpgsign=false` and run `git status` without
/// unsetting anything of Brokkr's.
#[test]
fn every_hands_environment_reads_unsigned_and_runs_git_through_the_scrub() {
    let dir = tempfile::tempdir().unwrap();
    let repo = repository(dir.path());
    for table in tables(dir.path()) {
        assert_eq!(shell_under(&repo, &table), READS_UNSIGNED, "{table:?}");
    }
}

/// No name Brokkr fixes for a seat is one a credential scrub drops: the
/// seat's git setting itself, and every name of the box's and the
/// unboxed dispatch's tables with the git identity beside them. A
/// declared secret binding is deliberate and reaches the seat through
/// the binding path, not through these tables.
#[test]
fn no_fixed_seat_name_is_one_a_credential_scrub_drops() {
    assert_eq!(
        seat_git_environment(),
        ("GIT_CONFIG_PARAMETERS", "'commit.gpgsign'='false'")
    );
    assert!(!scrubbed(seat_git_environment().0));
    let dir = tempfile::tempdir().unwrap();
    for table in tables(dir.path()) {
        let dropped: Vec<_> = table.keys().filter(|name| scrubbed(name)).collect();
        assert_eq!(dropped, Vec::<&String>::new(), "{table:?}");
    }
    // The pattern is case-insensitive, as dsh's `/i` is.
    assert!(scrubbed("GIT_CONFIG_KEY_0") && scrubbed("api_token") && scrubbed("PassWord"));
    assert!(!scrubbed("GIT_CONFIG_PARAMETERS"));
}
