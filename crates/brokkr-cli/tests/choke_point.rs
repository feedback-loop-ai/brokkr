//! The builder keeps the binary's path private at compile time (#1137i,
//! Z1). Its command, argv and shell doors refuse doctor. Tests can stage
//! an executable copy. The scan pins the bare path and the isolated door,
//! whose guard refuses an unreadable, unparsable or shared PID namespace
//! before its program starts (Z2).

use std::path::Path;

/// The one file that names the binary.
const BUILDER: &str = "support/built.rs";

const PINS: &[(&str, &[&str])] = &[
    (
        "bare_path",
        &[
            // The adapter config names the available provider executable.
            "agent_fallback.rs",
            // The gate script chooses the verifier's journal command.
            "delivered_by_brokkr.rs",
            // The native dsh fixture is a symlink to the real built image.
            "doctor_dsh_selection.rs",
            // The engine and its captured inputs supply their own arguments.
            "driver_conformance.rs",
            // The compiler composed the engine's arguments before retargeting.
            "qoder_driver.rs",
            // The compile fixture substitutes only the composed engine path.
            "route_overlay_driver.rs",
            // The fake adapter config also names its provider executable.
            "support/fake_office.rs",
        ],
    ),
    // The PID namespace runner alone can use the isolated path.
    ("isolated_brokkr_path", &["support/isolated.rs"]),
];

/// Report the Cargo name in the source and check path door names in code.
fn bypasses(relative: &str, source: &str) -> Vec<String> {
    let cargo_name = concat!("CARGO_BIN_", "EXE_brokkr");
    let mut found: Vec<_> = source
        .lines()
        .filter(|line| line.contains(cargo_name))
        .map(|line| line.trim().to_owned())
        .collect();
    let code = crate::rust_source::code(source).expect("a test source lexes");
    for &(door, pins) in PINS {
        let uses = code.lines().zip(source.lines()).filter(|(code, _)| {
            code.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .any(|name| name == door)
        });
        if pins.contains(&relative) {
            if uses.count() == 0 {
                found.push(format!("unused pin for {door}"));
            }
        } else {
            found.extend(uses.map(|(_, line)| line.trim().to_owned()));
        }
    }
    found
}

/// Z1: every tracked source uses the builder and every pin has a user.
#[test]
fn no_test_spawns_the_built_brokkr_but_through_its_builder() {
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let tracked = crate::tracked_files::tracked(&tests, &["*.rs"]);
    for &(_, pins) in PINS {
        for &pin in pins {
            assert!(
                tracked.iter().any(|file| file == pin),
                "{pin}: pinned source is absent"
            );
        }
    }
    let mut found = Vec::new();
    for relative in tracked {
        if relative == BUILDER {
            continue;
        }
        let source = std::fs::read_to_string(tests.join(&relative)).unwrap();
        found.extend(
            bypasses(&relative, &source)
                .into_iter()
                .map(|line| format!("{relative}: {line}")),
        );
    }
    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn the_scan_catches_the_cargo_name_and_a_path_assigned_to_a_variable() {
    let named = concat!(r#"let b = env!("CARGO_BIN_"#, r#"EXE_brokkr");"#);
    let assigned = r#"let b = crate::built::bare_path(); Command::new(b).arg("doctor");"#;
    let source = [named, assigned].join("\n");
    assert_eq!(bypasses("unlisted.rs", &source), [named, assigned]);
}

#[test]
fn the_scan_catches_a_path_on_its_own_line() {
    let path = "crate::built::bare_path()";
    let source = ["Command::new(", path, r#").arg("doctor");"#].join("\n");
    assert_eq!(bypasses("unlisted.rs", &source), [path]);
}

#[test]
fn the_scan_catches_a_path_formatted_into_a_shell_command() {
    let supplied = r#".args(["-c", &format!("'{}' doctor", crate::built::bare_path())]);"#;
    let source = [r#"Command::new("sh")"#, supplied].join("\n");
    assert_eq!(bypasses("unlisted.rs", &source), [supplied]);
}

#[test]
fn the_scan_ignores_a_path_name_in_a_string_and_comment() {
    let source = "let longer_bare_path = \"bare_path\";\n// bare_path";
    assert_eq!(bypasses("unlisted.rs", source), Vec::<String>::new());
}

#[test]
fn the_scan_refuses_a_pin_without_a_path_use() {
    assert_eq!(
        bypasses("qoder_driver.rs", r#"let unused = "bare_path";"#),
        ["unused pin for bare_path"]
    );
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

/// The argv door shares the command door's refusal.
#[test]
#[should_panic(
    expected = "`brokkr doctor` reaches doctor's version probe, which delivers real \
                           kills: run it through isolated::brokkr"
)]
fn the_argv_door_refuses_doctor() {
    drop(crate::built_argv!(["doctor"]));
}

#[test]
#[should_panic(expected = "the built brokkr is given its verb as it is built")]
fn the_argv_door_refuses_empty_arguments() {
    drop(crate::built_argv!([] as [&str; 0]));
}

#[test]
#[should_panic(
    expected = "`brokkr doctor` reaches doctor's version probe, which delivers real \
                           kills: run it through isolated::brokkr"
)]
fn the_shell_line_refuses_doctor() {
    drop(crate::built_shell_line!(["doctor"]));
}

#[test]
fn the_shell_line_keeps_an_apostrophe_in_one_word() {
    let word = "operator's path";
    let line = crate::built_shell_line!([word]);
    let script = format!("set -- {line}; shift; printf '<%s>\\n' \"$@\"");
    let output = std::process::Command::new("sh")
        .args(["-c", &script])
        .output()
        .unwrap();
    assert_eq!(
        (output.status.code(), output.stdout.as_slice()),
        (Some(0), b"<operator's path>\n".as_slice())
    );
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
            ("pid:[100000000000000000000]".into(), None),
            ("pid:[18446744073709551616]".into(), None),
            (format!("pid:[{}]\n", OUTER + 1), None),
            (
                "pid:[17999999999999999999]".into(),
                Some(17999999999999999999),
            ),
            (
                "pid:[18446744073709551609]".into(),
                Some(18446744073709551609),
            ),
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
