//! One choke point for the built `brokkr` (#1137i): no test source names
//! the binary but `support/built.rs`, the plain door refuses doctor, and
//! the isolated door's guard refuses, with its typed reason and before
//! its program starts, a namespace identity it cannot read or parse, or
//! its parent's (Z1 and Z2).

use std::path::Path;

/// The one file that names the binary.
const BUILDER: &str = "support/built.rs";

/// The lines of `source` that spawn the built binary or name it outside
/// the builder: the variable Cargo names it by, anywhere, or a
/// `Command::new` of the builder's path.
fn bypasses(source: &str) -> Vec<String> {
    let named = concat!("CARGO_BIN_", "EXE_brokkr");
    let code = crate::rust_source::code(source).expect("a test source lexes");
    let spawned = code
        .lines()
        .filter(|line| line.contains("Command::new(") && line.contains("BIN"));
    let named = source.lines().filter(|line| line.contains(named));
    named
        .chain(spawned)
        .map(|line| line.trim().to_owned())
        .collect()
}

/// Z1: every tracked test source of this crate spawns the built binary
/// through the builder, so every `doctor` it runs is alone in its
/// namespace.
#[test]
fn no_test_spawns_the_built_brokkr_but_through_its_builder() {
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut found = Vec::new();
    for relative in crate::tracked_files::tracked(&tests, &["*.rs"]) {
        if relative == BUILDER {
            continue;
        }
        let source = std::fs::read_to_string(tests.join(&relative)).unwrap();
        found.extend(
            bypasses(&source)
                .into_iter()
                .map(|line| format!("{relative}: {line}")),
        );
    }
    assert_eq!(found, Vec::<String>::new());
}

/// Z1: the scan sees both ways round the builder, and not the builder's
/// own words in a string or a comment.
#[test]
fn the_scan_catches_a_named_binary_and_a_spawned_path() {
    let named = concat!("let b = env!(\"CARGO_BIN_", "EXE_brokkr\");");
    let spawned = "let c = Command::new(crate::built::BIN);";
    let quoted = "let d = \"Command::new(BIN)\"; // Command::new(BIN)";
    let source = [named, spawned, quoted].join("\n");
    assert_eq!(bypasses(&source), [named, spawned]);
}

/// Z1: the plain door refuses doctor before anything runs.
#[test]
#[should_panic(
    expected = "`brokkr doctor` reaches doctor's version probe, which delivers real \
                           kills: run it through isolated::brokkr"
)]
fn the_plain_door_refuses_doctor() {
    crate::built::brokkr(["doctor", "--bundle", "."]);
}

/// Z1: a command is given its verb as it is built, so none is added after
/// the refusal was ruled.
#[test]
#[should_panic(expected = "the built brokkr is given its verb as it is built")]
fn the_plain_door_refuses_a_command_without_its_verb() {
    crate::built::brokkr::<[&str; 0], &str>([]);
}

/// The guard's verdicts (Z2), proved without a namespace: the guard runs
/// directly, reads the link a test planted, and runs, if it admits, an
/// inert seam that only records the delivery it stands for.
#[cfg(target_os = "linux")]
mod guard {
    use std::path::Path;
    use std::process::Command;

    use crate::isolated::{seconds, Guard};
    use crate::pidns::{identity_at, Isolation};

    /// The namespace the guard is told is its parent's.
    const OUTER: u64 = 4026531836;

    /// What the guard did, given a link that reads `target` (or none at
    /// all): its exit code, its refusal, and whether the seam ran.
    fn guarded(target: Option<&str>) -> (Option<i32>, Option<Isolation>, bool) {
        guarded_with(target, |_| {})
    }

    /// Let a second reader inspect the same link before the fixture drops.
    fn guarded_with(
        target: Option<&str>,
        inspect: impl FnOnce(&Path),
    ) -> (Option<i32>, Option<Isolation>, bool) {
        let dir = tempfile::tempdir().unwrap();
        let (link, delivered) = (dir.path().join("ns-pid"), dir.path().join("delivered"));
        if let Some(target) = target {
            std::os::unix::fs::symlink(target, &link).unwrap();
        }
        let guard = Guard::of(OUTER, &link);
        let mut launcher = Command::new("/usr/bin/env");
        guard.wrap(&mut launcher);
        launcher
            .args(["/bin/sh", "-c", ": > \"$0\""])
            .arg(&delivered);
        let status = launcher.status().unwrap();
        let outcome = (status.code(), guard.refused(), delivered.exists());
        inspect(&link);
        outcome
    }

    /// A namespace apart from its parent's is admitted, and its program
    /// runs.
    #[test]
    fn a_namespace_apart_from_its_parents_runs_its_program() {
        let apart = format!("pid:[{}]", OUTER + 1);
        assert_eq!(guarded(Some(&apart)), (Some(0), None, true));
    }

    /// The parent's own namespace is refused, and nothing runs.
    #[test]
    fn the_parents_namespace_is_refused_before_anything_runs() {
        let shared = format!("pid:[{OUTER}]");
        assert_eq!(
            guarded(Some(&shared)),
            (Some(125), Some(Isolation::Shared(OUTER)), false)
        );
    }

    /// Z2: an identity the guard cannot read is refused, with its typed
    /// reason, and nothing runs.
    #[test]
    fn an_unreadable_identity_is_refused_before_anything_runs() {
        let (code, refused, ran) = guarded(None);
        assert_eq!(
            (code, ran),
            (Some(125), false),
            "delivered, refused as {refused:?}"
        );
        let Some(Isolation::Unreadable(what)) = refused else {
            panic!("refused as {refused:?}");
        };
        assert!(what.ends_with("/ns-pid"), "{what}");
    }

    /// Z2: an identity that is not `pid:[N]`, N decimal digits, is
    /// refused, with what it read, and nothing runs.
    #[test]
    fn an_unparsable_identity_is_refused_before_anything_runs() {
        for read in ["pid:[]", "pid:[+5]", "pid:[12a]", "pid:[5", "net:[5]"] {
            let refused = Some(Isolation::Unparsable(read.into()));
            assert_eq!(guarded(Some(read)), (Some(125), refused, false), "{read}");
        }
    }

    /// Z2: the parent and child read the same canonical namespace numbers.
    #[test]
    fn the_guard_and_parent_read_one_namespace_grammar() {
        for (read, number) in [
            (format!("pid:[0{OUTER}]"), None),
            ("pid:[00]".into(), None),
            ("pid:[99999999999999999999]".into(), None),
            ("pid:[18446744073709551616]".into(), None),
            (format!("pid:[{}]\n", OUTER + 1), None),
            ("pid:[18446744073709551615]".into(), Some(u64::MAX)),
            ("pid:[0]".into(), Some(0)),
        ] {
            let parsed = number.ok_or_else(|| Isolation::Unparsable(read.clone()));
            let outcome = guarded_with(Some(&read), |link| {
                assert_eq!(identity_at(link), parsed, "parent read {read:?}");
            });
            let expected = match parsed {
                Ok(_) => (Some(0), None, true),
                Err(refused) => (Some(125), Some(refused), false),
            };
            assert_eq!(outcome, expected, "guard read {read:?}");
        }
    }

    /// Z4: a deadline is whole seconds rounded up, never `timeout 0`.
    #[test]
    fn a_deadline_is_rounded_up_to_whole_seconds() {
        let limits = [0, 1, 500, 1000, 2100].map(std::time::Duration::from_millis);
        assert_eq!(limits.map(seconds), ["1", "1", "1", "1", "3"]);
    }

    /// The guard's record of a refusal is read in its own words only.
    #[test]
    #[should_panic(expected = "the guard refused in words it does not use: \"maybe 7\"")]
    fn a_refusal_in_other_words_fails_the_test() {
        let guard = Guard::of(OUTER, Path::new("/proc/self/ns/pid"));
        let mut launcher = Command::new("/bin/sh");
        launcher.args(["-c", "printf 'maybe 7\\n' > \"$0\""]);
        let mut recorder = Command::new("/usr/bin/env");
        guard.wrap(&mut recorder);
        let record = recorder.get_args().nth(3).unwrap().to_owned();
        launcher.arg(record).status().unwrap();
        guard.refused();
    }
}
