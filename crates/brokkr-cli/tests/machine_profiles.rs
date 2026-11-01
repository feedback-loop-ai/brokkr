//! The exact coverage gate's machines hand their profiles to its report
//! (operator ruling 2026-11-01): `scripts/machine-profiles.sh` writes a
//! machine's run merged into one profile, with a manifest naming the
//! machine and the candidate, and reads every machine's into the report's
//! run, refusing a manifest it cannot read, a machine the layout does not
//! name, one measured twice or never, and another candidate's. A stub
//! stands in for `llvm-profdata merge`.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

use crate::readable_profiles::{exact_gate, first, names, stderr};
use crate::workspace_root::workspace;

const SCRIPT: &str = "scripts/machine-profiles.sh";

/// The candidate every planted manifest names unless a case says otherwise.
const CANDIDATE: &str = "4b825dc6-nightly-2026-09-05-0123456789abcdef-fedcba9876543210";

/// A stand-in for `llvm-profdata merge -sparse -o <out> <profiles...>`: it
/// writes the profiles' names to `<out>`, and fails on one that starts
/// `UNMERGEABLE`.
const STUB: &str = r#"#!/usr/bin/env bash
[ "$1 $2 $3" = "merge -sparse -o" ] || exit 2
out="$4"
shift 4
for profile in "$@"; do
  [ "$(head -c 11 "$profile")" != UNMERGEABLE ] || exit 1
  basename "$profile"
done >"$out"
"#;

struct Scratch {
    dir: tempfile::TempDir,
}

impl Scratch {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("scratch");
        for sub in ["target", "machines"] {
            std::fs::create_dir(dir.path().join(sub)).expect("scratch directory");
        }
        let tool = dir.path().join("llvm-profdata");
        std::fs::write(&tool, STUB).expect("stub");
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).expect("mode");
        Self { dir }
    }

    fn path(&self, sub: &str) -> PathBuf {
        self.dir.path().join(sub)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new("bash")
            .arg(workspace().join(SCRIPT))
            .args(args)
            .output()
            .expect("the script runs")
    }

    /// A machine's directory as the report downloads it, with its manifest
    /// (`None` writes none), its profile and its list of unreadable ones.
    fn machine(&self, at: &str, manifest: Option<&str>, unreadable: Option<&str>) {
        let dir = self.path("machines").join(at);
        std::fs::create_dir(&dir).expect("machine directory");
        if let Some(manifest) = manifest {
            std::fs::write(dir.join("machine.json"), manifest).expect("manifest");
        }
        std::fs::write(dir.join("machine.profdata"), format!("profile of {at}")).expect("profile");
        if let Some(lines) = unreadable {
            std::fs::write(dir.join("unreadable-profiles.txt"), lines).expect("list");
        }
    }

    /// The report's read of every machine the layout names.
    fn read(&self, laid_out: &[&str]) -> Output {
        let (dir, target, list) = (
            self.path("machines"),
            self.path("target"),
            self.path("unreadable.txt"),
        );
        let mut args = vec![
            "read",
            dir.to_str().expect("path"),
            CANDIDATE,
            target.to_str().expect("path"),
            "forge-coverage.r9",
            list.to_str().expect("path"),
        ];
        args.extend(laid_out);
        self.run(&args)
    }
}

/// A manifest for `machine` and `candidate`.
fn manifest(machine: &str, candidate: &str) -> String {
    format!(
        r#"{{"schema":"brokkr.coverage-machine/v1","machine":"{machine}","candidate":"{candidate}"}}"#
    )
}

/// A machine merges its run's profiles, and only its run's, into one, and
/// names itself and the candidate beside it; a run that wrote none, or
/// whose profiles do not merge, is refused.
#[test]
fn a_machine_writes_its_runs_profiles_merged_with_its_manifest() {
    let scratch = Scratch::new();
    for (name, bytes) in [
        ("forge-coverage.r1-11-a.profraw", "RAW"),
        ("forge-coverage.r1-12-b.profraw", "RAW"),
        ("forge-coverage.r0-13-c.profraw", "another run's"),
    ] {
        std::fs::write(scratch.path("target").join(name), bytes).expect("profile");
    }
    let (tool, target, out) = (
        scratch.path("llvm-profdata"),
        scratch.path("target"),
        scratch.path("out"),
    );
    let write = |run: &str| {
        scratch.run(&[
            "write",
            tool.to_str().expect("path"),
            target.to_str().expect("path"),
            run,
            "linux/2",
            CANDIDATE,
            out.to_str().expect("path"),
        ])
    };
    let output = write("forge-coverage.r1");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        std::fs::read_to_string(out.join("machine.profdata")).expect("merged"),
        "forge-coverage.r1-11-a.profraw\nforge-coverage.r1-12-b.profraw\n"
    );
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("machine.json")).expect("manifest"))
            .expect("JSON");
    assert_eq!(
        written,
        serde_json::json!({
            "schema": "brokkr.coverage-machine/v1",
            "machine": "linux/2",
            "candidate": CANDIDATE,
        })
    );

    let none = write("forge-coverage.r7");
    assert_eq!(none.status.code(), Some(1));
    assert!(
        stderr(&none).starts_with("machine profiles: machine linux/2 wrote no profile under "),
        "{}",
        stderr(&none)
    );
    std::fs::write(target.join("forge-coverage.r1-14-d.profraw"), "UNMERGEABLE").expect("profile");
    let broken = write("forge-coverage.r1");
    assert_eq!(broken.status.code(), Some(1));
    assert_eq!(
        stderr(&broken),
        "machine profiles: machine linux/2's 3 profiles do not merge\n"
    );
}

/// The report takes every laid-out machine's profile under its own run's
/// name, and each machine's unreadable profiles into its evidence, led by
/// the machine.
#[test]
fn the_report_reads_every_machine_once_under_its_own_runs_name() {
    let scratch = Scratch::new();
    scratch.machine(
        "coverage-machine-0",
        Some(&manifest("linux/1", CANDIDATE)),
        Some("forge-coverage.a-1-x.profraw\terror: corrupt\n"),
    );
    scratch.machine(
        "coverage-machine-1",
        Some(&manifest("linux/2", CANDIDATE)),
        None,
    );
    let output = scratch.read(&["linux/1", "linux/2"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        names(&scratch.path("target")),
        [
            "forge-coverage.r9-machine-linux-1.profraw",
            "forge-coverage.r9-machine-linux-2.profraw",
        ]
    );
    assert_eq!(
        std::fs::read_to_string(
            scratch
                .path("target")
                .join("forge-coverage.r9-machine-linux-2.profraw")
        )
        .expect("profile"),
        "profile of coverage-machine-1"
    );
    assert_eq!(
        std::fs::read_to_string(scratch.path("unreadable.txt")).expect("list"),
        "linux/1\tforge-coverage.a-1-x.profraw\terror: corrupt\n"
    );
}

/// Machine directories as the report downloads them, each with its
/// manifest, or none.
type Planted<'a> = &'a [(&'a str, Option<String>)];

/// A machine never measured, one measured twice, one the layout does not
/// name, another candidate's, a manifest that is not a machine's and no
/// manifest at all are each refused by name.
#[test]
fn a_missing_doubled_unknown_or_foreign_machine_is_refused() {
    let other = "0000000a-nightly-2026-09-05-0123456789abcdef-fedcba9876543210";
    let cases: [(Planted, &str); 6] = [
        (
            &[("m0", Some(manifest("linux/1", CANDIDATE)))],
            "machine profiles: machine linux/2 was never measured\n",
        ),
        (
            &[
                ("m0", Some(manifest("linux/1", CANDIDATE))),
                ("m1", Some(manifest("linux/1", CANDIDATE))),
            ],
            "machine profiles: machine linux/1 was measured twice\n",
        ),
        (
            &[("m0", Some(manifest("linux/7", CANDIDATE)))],
            "names linux/7, which the layout does not\n",
        ),
        (
            &[("m0", Some(manifest("linux/1", other)))],
            "machine profiles: machine linux/1 measured 0000000a-nightly-2026-09-05-\
             0123456789abcdef-fedcba9876543210, not this candidate, 4b825dc6-nightly-\
             2026-09-05-0123456789abcdef-fedcba9876543210\n",
        ),
        (
            &[(
                "m0",
                Some(r#"{"schema":"another/v1","machine":"linux/1"}"#.to_string()),
            )],
            "is not a machine's manifest\n",
        ),
        (
            &[("m0", None)],
            "machine profiles: no machine's manifest is under ",
        ),
    ];
    for (machines, refusal) in cases {
        let scratch = Scratch::new();
        for (at, written) in machines {
            scratch.machine(at, written.as_deref(), None);
        }
        let output = scratch.read(&["linux/1", "linux/2"]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{refusal}: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains(refusal),
            "missing {refusal:?} in:\n{}",
            stderr(&output)
        );
    }
}

/// The gate measures on a machine, or reports over every machine, through
/// this script: a machine writes after its shards and reads its profiles
/// first, and the report reads every Linux machine the layout names before
/// its first report.
#[test]
fn the_exact_gate_hands_machine_profiles_through_this_script() {
    let gate = exact_gate();
    let at = |what: &str| first(&gate, what);
    let shards = at(r#"bash scripts/test-shards.sh coverage "$nightly""#);
    let write = at("bash scripts/machine-profiles.sh write");
    let read = at("bash scripts/machine-profiles.sh read");
    let report = at("llvm-cov report");
    assert!(shards < write && write < read && read < report);
    assert!(gate[write..read].contains("bash scripts/test-shards.sh machines coverage"));
}
