//! The dsh harness's child reads the seat's git environment whole (#1109):
//! run through the credential scrub dsh 0.2.1 applies to every child it
//! spawns, its shell still reads `commit.gpgsign=false` and runs git
//! without unsetting anything, under the host's identity, whatever
//! numbered git config group the engine itself inherited.

use super::super::*;
use super::{executable, invoke};
use crate::env_guard::EnvGuard;
use crate::hands::environment::tests::{repository, READS_UNSIGNED, SCRUB_THEN_GIT};

/// The numbered group and the parameters a host's own seat may carry.
const INHERITED: [&str; 4] = [
    "GIT_CONFIG_COUNT",
    "GIT_CONFIG_KEY_0",
    "GIT_CONFIG_VALUE_0",
    "GIT_CONFIG_PARAMETERS",
];

/// Launch a dsh seat whose fake harness runs [`SCRUB_THEN_GIT`] in a
/// repository carrying the host's identity, under the engine environment
/// `env` already holds; return what the shell printed and the child's
/// whole environment.
fn dsh_seat_shell(env: &mut EnvGuard) -> (String, String) {
    let dir = tempfile::tempdir().unwrap();
    let repo = repository(dir.path());
    for args in [
        ["config", "user.name", "Host Operator"],
        ["config", "user.email", "host@example.invalid"],
    ] {
        let out = Command::new("git")
            .args(args)
            .current_dir(&repo)
            .env_remove("GIT_CONFIG_COUNT")
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    }
    let dump = dir.path().join("env");
    let read = dir.path().join("git");
    let fake = executable(
        dir.path(),
        "dsh",
        &format!(
            "#!/bin/sh\nenv > '{}'\n{{\n{SCRUB_THEN_GIT}}} > '{}'\n",
            dump.display(),
            read.display()
        ),
    );
    env.set("BROKKR_DSH_BIN", &fake);
    env.set("DSH_HOME", dir.path());

    invoke(
        AdapterKind::Dsh,
        &[],
        "p",
        &json!({"workdir": repo}),
        None,
        &[],
        &mut |_| {},
    )
    .unwrap();

    (
        std::fs::read_to_string(&read).unwrap(),
        std::fs::read_to_string(&dump).unwrap(),
    )
}

/// The seat's git setting and the host identity reach the dsh child, so a
/// seat's commit is unsigned and attributed to the operator, and git runs
/// in the child's shell after dsh's scrub drops every credential-shaped
/// name.
#[test]
fn the_dsh_seat_commits_unsigned_under_the_host_identity() {
    let mut env = EnvGuard::lock();
    for name in INHERITED {
        env.remove(name);
    }
    let (read, seen) = dsh_seat_shell(&mut env);

    assert_eq!(read, READS_UNSIGNED);
    for line in [
        "GIT_CONFIG_COUNT=0",
        "GIT_CONFIG_PARAMETERS='commit.gpgsign'='false'",
        "GIT_AUTHOR_NAME=Host Operator",
        "GIT_COMMITTER_EMAIL=host@example.invalid",
    ] {
        assert!(seen.lines().any(|seen| seen == line), "{line}: {seen}");
    }
}

/// A whole numbered group the engine inherited (as a seat whose own
/// engine fixed the old triple does) is closed in the dsh child: dsh's
/// scrub drops its `GIT_CONFIG_KEY_0`, and git still reads the seat's
/// setting over the group's `true` and runs, rather than refusing over a
/// count with no key.
#[test]
fn an_inherited_numbered_group_does_not_break_the_dsh_seats_git() {
    let mut env = EnvGuard::lock();
    env.remove("GIT_CONFIG_PARAMETERS");
    env.set("GIT_CONFIG_COUNT", "1");
    env.set("GIT_CONFIG_KEY_0", "commit.gpgsign");
    env.set("GIT_CONFIG_VALUE_0", "true");
    let (read, seen) = dsh_seat_shell(&mut env);

    assert_eq!(read, READS_UNSIGNED);
    assert!(
        seen.lines().any(|line| line == "GIT_CONFIG_COUNT=0"),
        "{seen}"
    );
}
