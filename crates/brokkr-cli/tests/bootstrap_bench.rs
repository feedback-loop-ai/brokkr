//! The load-bearing half of `scripts/bootstrap-bench.sh`, proven here.
//!
//! The script times two paths and fails on a blown budget. Timing is
//! shell arithmetic and needs no proof; what does need proof is the
//! thing the script asserts BEFORE it reports a number — that a
//! pristine `brokkr init` scaffold can actually be driven to a first
//! completed effect with no agent session spawned and nothing billed.
//! A benchmark that timed a run which never ran would report a very
//! good number.
//!
//! The stand-in is the adapter's own `BROKKR_CLAUDE_BIN` override, and
//! it is deliberately the SMALLEST possible substitution: the bundle is
//! byte-identical to what an operator is handed, the compile runs with
//! its decision-0021 gate-class trust check intact against the
//! scaffold's own `adapters/claude.json`, the driver transport is the
//! real one, and the journal is the real one. Only the agent CLI at the
//! far end is a stub. Rewriting the seats to `brokkr fake-driver`
//! instead would not compile — `fake-driver` is not a `brokkr driver
//! <kind>` dispatch, so no adapter can declare its tier, and the three
//! gate seats would have had to be demoted to `work` to make it pass.
//! That would have been a benchmark of a different bundle.
//!
//! Unix only: the stub is a shell script, because the thing it stands
//! in for is a CLI on `PATH`.

#![cfg(unix)]

use std::path::Path;
use std::process::Command;

/// The stub the adapter spawns instead of Claude Code. It reads the seat
/// prompt on stdin — the same prompt a real session would — finds the
/// one file the engine reads a result from, and answers the phase it was
/// seated in. It writes no code: what is being measured is the ENGINE's
/// cost to reach a first completed effect.
///
/// `intake` resolves; `implement` reports `blocked`, which the starter
/// table rules a hard stop (`IMPL-BLOCKED`). So the run ends one seat
/// PAST its first completed effect, and the script's timed window
/// over-measures time-to-first-effect rather than under-measuring it.
const STUB: &str = r#"#!/usr/bin/env bash
set -uo pipefail
prompt="$(cat)"
# The result contract puts the one path the engine reads on its own
# four-space-indented line; the last such line is it, because a role
# charter's indented lines are commands and come first.
result_file="$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | tail -n 1)"
phase="$(printf '%s\n' "$prompt" | sed -n 's/^Phase: \([a-z-]*\).*/\1/p' | head -n 1)"
case "$phase" in
  intake) result=resolved ;;
  *) result=blocked ;;
esac
if [ -z "$result_file" ]; then
  echo "stub: no result path in the prompt" >&2
  exit 1
fi
mkdir -p "$(dirname "$result_file")"
printf '{"result":"%s","notes":"benchmark stub; no agent ran"}\n' "$result" >"$result_file"
printf '{"type":"result","subtype":"success","num_turns":1,"total_cost_usd":0.0}\n'
"#;

fn git(repo: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(repo)
        .status()
        .unwrap()
        .success());
}

#[test]
fn a_pristine_scaffold_reaches_a_first_completed_effect_with_no_agent() {
    use std::os::unix::fs::PermissionsExt;

    let repo = tempfile::tempdir().unwrap();
    let repo = repo.path();
    git(repo, &["init", "-q"]);
    git(repo, &["config", "user.name", "Bootstrap Bench"]);
    git(repo, &["config", "user.email", "bench@test"]);
    git(repo, &["config", "commit.gpgSign", "false"]);
    // A marker, so `init` writes a stack-aware charter rather than the
    // placeholder one — the scaffold an operator would actually get.
    std::fs::write(repo.join("package.json"), "{\"name\":\"bench-app\"}\n").unwrap();
    std::fs::write(repo.join(".gitignore"), ".forge/\n").unwrap();
    git(repo, &["add", "."]);
    git(repo, &["commit", "-q", "-m", "bench fixture"]);

    let stub = repo.join("claude-stub");
    std::fs::write(&stub, STUB).unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();

    // Step 2 of the spine, into the repository root itself.
    let init = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(["init", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    assert_eq!(
        init.status.code(),
        Some(0),
        "init: {}",
        String::from_utf8_lossy(&init.stderr)
    );

    // Step 3, with the agent stubbed and NOTHING else changed.
    let run = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args([
            "run",
            "--bundle",
            ".",
            "--repo",
            ".",
            "--db",
            ".forge/forge.db",
            "--feature",
            "bootstrap benchmark: reach one completed effect",
        ])
        .env("BROKKR_CLAUDE_BIN", &stub)
        .current_dir(repo)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&run.stderr);

    let inspect = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args([
            "inspect",
            "--run",
            "latest",
            "--db",
            ".forge/forge.db",
            "--json",
        ])
        .current_dir(repo)
        .output()
        .unwrap();
    let readout = String::from_utf8_lossy(&inspect.stdout);
    assert_eq!(
        inspect.status.code(),
        Some(0),
        "inspect: {}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    // 3 is a stopped run, which is what the scripted `blocked` rules.
    // Any other code and the script above would be timing a failure.
    assert_eq!(run.status.code(), Some(3), "run: {stderr}\n{readout}");

    // The effect that mattered actually completed, and the ruling that
    // ended the run is the one the stub scripted.
    assert!(
        readout.contains("effect/succeeded"),
        "no effect completed: {readout}"
    );
    assert!(
        readout.contains("IMPL-BLOCKED"),
        "unexpected ruling: {readout}"
    );

    // Nothing was billed, because nothing was spawned: the only binary
    // the adapter reached for was the stub in this tempdir.
    assert!(
        !stderr.contains("could not invoke the agent CLI"),
        "the real claude binary was reached for: {stderr}"
    );
}
