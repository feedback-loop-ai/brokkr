//! The dsh harness's child reads the seat's git environment whole (#1109):
//! run through the credential scrub dsh 0.2.1 applies to every child it
//! spawns, its shell still reads `commit.gpgsign=false` and runs git
//! without unsetting anything, under the host's identity.

use super::super::*;
use super::{executable, invoke};
use crate::env_guard::EnvGuard;
use crate::hands::environment::tests::{repository, READS_UNSIGNED, SCRUB_THEN_GIT};

/// The seat's git setting and the host identity reach the dsh child, so a
/// seat's commit is unsigned and attributed to the operator, and git runs
/// in the child's shell after dsh's scrub drops every credential-shaped
/// name.
#[test]
fn the_dsh_seat_commits_unsigned_under_the_host_identity() {
    let mut env = EnvGuard::lock();
    // Only Brokkr's setting reaches the child: none the host's own seat
    // may carry, whole or broken.
    for name in [
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_KEY_0",
        "GIT_CONFIG_VALUE_0",
        "GIT_CONFIG_PARAMETERS",
    ] {
        env.remove(name);
    }
    let dir = tempfile::tempdir().unwrap();
    let repo = repository(dir.path());
    for args in [
        ["config", "user.name", "Host Operator"],
        ["config", "user.email", "host@example.invalid"],
    ] {
        let out = Command::new("git")
            .args(args)
            .current_dir(&repo)
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

    assert_eq!(std::fs::read_to_string(&read).unwrap(), READS_UNSIGNED);
    let seen = std::fs::read_to_string(&dump).unwrap();
    assert!(
        seen.lines()
            .any(|line| line == "GIT_CONFIG_PARAMETERS='commit.gpgsign'='false'"),
        "{seen}"
    );
    assert!(!seen.contains("GIT_CONFIG_COUNT="), "{seen}");
    assert!(seen.contains("GIT_AUTHOR_NAME=Host Operator"), "{seen}");
    assert!(
        seen.contains("GIT_COMMITTER_EMAIL=host@example.invalid"),
        "{seen}"
    );
}
