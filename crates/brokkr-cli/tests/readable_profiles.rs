//! The exact coverage gate's profiles, each read before the merge
//! (operator ruling 2026-11-01): `scripts/readable-profiles.sh` moves aside
//! every profile of the run that `llvm-profdata show` cannot read and names
//! each, leaves the readable ones and another run's where they are, and
//! refuses a run that wrote no profile or none it can read. A stub stands in
//! for `llvm-profdata`: it fails, as the real reader does, on a file whose
//! header it cannot parse.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::workspace_root::workspace;

const SCRIPT: &str = "scripts/readable-profiles.sh";

/// A stand-in for `llvm-profdata show`: a file that starts `CORRUPT` fails
/// with the real reader's words, and any other is read.
const STUB: &str = r#"#!/usr/bin/env bash
[ "$1" = show ] || exit 2
if [ "$(head -c 7 "$2")" = CORRUPT ]; then
  printf 'error: %s: invalid instrumentation profile data (file header is corrupt)\n' "$2" >&2
  exit 1
fi
printf 'Instrumentation level: Front-end\n'
"#;

/// A target directory of profiles, the stub reader and where the script
/// puts what it leaves out.
struct Scratch {
    dir: tempfile::TempDir,
}

impl Scratch {
    fn new(profiles: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().expect("scratch");
        std::fs::create_dir(dir.path().join("target")).expect("target");
        let tool = dir.path().join("llvm-profdata");
        std::fs::write(&tool, STUB).expect("stub reader");
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).expect("mode");
        for (name, bytes) in profiles {
            std::fs::write(dir.path().join("target").join(name), bytes).expect("profile");
        }
        Self { dir }
    }

    fn path(&self, sub: &str) -> PathBuf {
        self.dir.path().join(sub)
    }

    /// The script on the run `forge-coverage.r1`.
    fn run(&self) -> Output {
        Command::new("bash")
            .arg(workspace().join(SCRIPT))
            .arg(self.path("llvm-profdata"))
            .arg(self.path("target"))
            .arg("forge-coverage.r1")
            .arg(self.path("aside"))
            .arg(self.path("unreadable.txt"))
            .output()
            .expect("the script runs")
    }
}

/// The names in `dir`, sorted, or none when it is not there.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .map(|entry| {
                    entry
                        .expect("entry")
                        .file_name()
                        .into_string()
                        .expect("name")
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn an_unreadable_profile_is_left_out_and_named_and_the_rest_stay() {
    let scratch = Scratch::new(&[
        ("forge-coverage.r1-11-a.profraw", "READABLE"),
        ("forge-coverage.r1-12-b.profraw", "CORRUPT header"),
        ("forge-coverage.r1-13-c.profraw", "READABLE"),
        ("forge-coverage.r0-14-d.profraw", "CORRUPT, another run's"),
    ]);
    let output = scratch.run();
    let text = stderr(&output);
    assert!(output.status.success(), "{text}");
    assert_eq!(
        names(&scratch.path("target")),
        [
            "forge-coverage.r0-14-d.profraw",
            "forge-coverage.r1-11-a.profraw",
            "forge-coverage.r1-13-c.profraw",
        ],
        "the readable profiles and another run's stay where the report reads them"
    );
    assert_eq!(
        names(&scratch.path("aside")),
        ["forge-coverage.r1-12-b.profraw"]
    );
    assert_eq!(
        std::fs::read_to_string(scratch.path("unreadable.txt")).expect("the list"),
        "forge-coverage.r1-12-b.profraw\terror: forge-coverage.r1-12-b.profraw: invalid \
         instrumentation profile data (file header is corrupt)\n"
    );
    assert_eq!(
        text,
        "readable profiles: left out forge-coverage.r1-12-b.profraw, which llvm-profdata cannot \
         read: error: forge-coverage.r1-12-b.profraw: invalid instrumentation profile data (file \
         header is corrupt)\nreadable profiles: 1 of 3 profiles left out; the report counts the \
         rest\n"
    );
}

#[test]
fn a_run_whose_profiles_all_read_leaves_every_one_and_an_empty_list() {
    let scratch = Scratch::new(&[
        ("forge-coverage.r1-11-a.profraw", "READABLE"),
        ("forge-coverage.r1-12-b.profraw", "READABLE"),
    ]);
    let output = scratch.run();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_eq!(names(&scratch.path("target")).len(), 2);
    assert_eq!(names(&scratch.path("aside")), Vec::<String>::new());
    assert_eq!(
        std::fs::read_to_string(scratch.path("unreadable.txt")).expect("the list"),
        ""
    );
}

#[test]
fn a_run_with_no_readable_profile_or_none_at_all_is_refused() {
    for (profiles, refusal) in [
        (
            &[
                ("forge-coverage.r1-11-a.profraw", "CORRUPT"),
                ("forge-coverage.r1-12-b.profraw", "CORRUPT"),
            ][..],
            "readable profiles: none of this run's 2 profiles can be read\n",
        ),
        (
            &[("forge-coverage.r0-11-a.profraw", "READABLE")][..],
            "readable profiles: this run wrote no profile under ",
        ),
    ] {
        let scratch = Scratch::new(profiles);
        let output = scratch.run();
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(
            stderr(&output).contains(refusal),
            "missing {refusal:?} in:\n{}",
            stderr(&output)
        );
    }
}

/// The gate reads its profiles after every shard has run and before its
/// first report, and the list of what it left out lands in the evidence the
/// coverage job uploads.
#[test]
fn the_exact_gate_reads_each_profile_between_the_shards_and_the_report() {
    let gate = std::fs::read_to_string(workspace().join("scripts/coverage-exact.sh"))
        .expect("the exact gate");
    let at = |what: &str| {
        gate.find(what)
            .unwrap_or_else(|| panic!("coverage-exact.sh has no {what:?}"))
    };
    let shards = at("bash scripts/test-shards.sh coverage");
    let read = at("bash scripts/readable-profiles.sh");
    let report = at("llvm-cov report");
    assert!(
        shards < read && read < report,
        "shards, then the read, then the report"
    );
    assert!(
        gate[read..report].contains("target/coverage/unreadable-profiles.txt"),
        "the list of profiles left out is in the uploaded evidence"
    );
}
