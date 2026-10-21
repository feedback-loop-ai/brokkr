//! The engine executable a run composes with (#349, ask 8): a box's
//! `hands exec` prefix, its hands server, a harness fragment's `{brokkr}`
//! and a dialect step's dispatch each name the one value they are handed —
//! the compiled bundle's — and never a binary read again. An unreadable one
//! keeps the empty spelling the engine has always given it.

use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;

use super::super::*;
use super::REACH;
use crate::argv::Executable;
use crate::engine::boxed::Unnamed;

fn strings(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

/// An engine executable that is not this test binary.
fn injected() -> Executable {
    Executable(Some(PathBuf::from("/opt/brokkr-test")))
}

/// `command`'s box at `/w`, handed `engine`, the fixtures' artifact root
/// and reach.
fn boxed_by(engine: &Executable, command: &[String]) -> Result<Vec<String>, Unnamed> {
    let spec = HandsSpec::default();
    let handed = (engine, Path::new("/a"), Some(&*REACH));
    hands_command(command.to_vec(), Some(&spec), Path::new("/w"), handed, &[])
}

#[test]
fn a_box_and_its_hands_server_name_the_executable_they_are_handed() {
    let exec = exec_dispatch(Path::new("/bundle/x.sh"));
    let prefix = |engine: &str| {
        let spec = r#"{"binds":[],"kind":"workspace","network":false}"#;
        let words = [
            engine,
            "hands",
            "exec",
            "--workdir",
            "/w",
            "--artifacts",
            "/a",
        ];
        [
            strings(&words),
            strings(&["--reach", REACH_ARG, "--spec", spec, "--"]),
        ]
        .concat()
    };
    let boxed = |engine: &str| [prefix(engine), exec.clone()].concat();
    assert_eq!(boxed_by(&injected(), &exec), Ok(boxed("/opt/brokkr-test")));
    assert_eq!(boxed_by(&Executable(None), &exec), Ok(boxed("")));
    let model = strings(&[
        "/b",
        "driver",
        "codex",
        "--",
        "-c",
        "mcp_servers.brokkr.command=\"{brokkr}\"",
    ]);
    let served = |engine: &str| {
        let command = format!("mcp_servers.brokkr.command=\"{engine}\"");
        [model[..5].to_vec(), vec![command]].concat()
    };
    assert_eq!(
        boxed_by(&injected(), &model),
        Ok(served("/opt/brokkr-test"))
    );
    assert_eq!(boxed_by(&Executable(None), &model), Ok(served("")));
    let lossy = PathBuf::from(std::ffi::OsStr::from_bytes(b"/opt/brokkr\xff"));
    let unnamed = Executable(Some(lossy.clone()));
    assert_eq!(boxed_by(&unnamed, &exec), Err(Unnamed::Executable(lossy)));
}

#[test]
fn a_harness_fragment_names_the_executable_it_is_handed() {
    let harness = HarnessHands {
        work: Some(strings(&["--mcp", "{brokkr}"])),
        ..codex_harness()
    };
    let link = candidate("codex", Vec::new(), harness);
    let fragment = |engine: &Executable| {
        let (spec, handed) = (
            HandsSpec::default(),
            (engine, Path::new("/a"), Some(&*REACH)),
        );
        let spawn = compose_site(
            BuiltBoundary::Harness,
            SeatClass::Work,
            link.argv.clone(),
            Some(&spec),
            Some(&link),
            Path::new("/w"),
            handed,
            &[],
            "/r/p.json",
            None,
        );
        spawn.argv[link.argv.len()..].to_vec()
    };
    assert_eq!(
        (fragment(&injected()), fragment(&Executable(None))),
        (
            strings(&["--mcp", "/opt/brokkr-test"]),
            strings(&["--mcp", ""])
        )
    );
}

/// A dialect step dispatches `<engine> driver exec --` through the
/// executable its bundle carries: a recording stand-in, staged beside and
/// renamed in, writes the argv it was spawned with. An unreadable one
/// spawns the empty spelling, which never starts, and the stand-in records
/// nothing.
#[test]
fn a_dialect_step_dispatches_through_the_executable_its_bundle_carries() {
    let (stand_in, spawned, _) = dialect_step_through(|stand_in| Some(stand_in.into()));
    let stand_in = stand_in.to_str().unwrap();
    let argv = strings(&[stand_in, "driver", "exec", "--", "true"]);
    assert_eq!(spawned, Some(argv));
    let (_, spawned, error) = dialect_step_through(|_| None);
    let failed = "sequence step 'validate': driver did not spawn: failed to spawn driver  driver \
                  exec -- true: No such file or directory (os error 2)";
    assert_eq!((spawned, error), (None, json!(failed)));
}

/// The stand-in, the argv it recorded if it was reached, and the last
/// journaled error, of a dialect step whose bundle carries the executable
/// `carried` names given the stand-in's path.
fn dialect_step_through(
    carried: impl FnOnce(&Path) -> Option<PathBuf>,
) -> (PathBuf, Option<Vec<String>>, Value) {
    let (kept, mut engine) = crate::engine::tests::engine(single_body(strings(&["driver"])));
    let record = kept.path().join("argv");
    let staged = kept.path().join("brokkr-test.staged");
    let program = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$0\" \"$@\" > '{}'\n",
        record.display()
    );
    std::fs::write(&staged, program).unwrap();
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).unwrap();
    let stand_in = kept.path().join("brokkr-test");
    std::fs::rename(&staged, &stand_in).unwrap();
    engine.bundle.executable = Executable(carried(&stand_in));
    let step = SequenceStep {
        name: "validate".into(),
        results: strings(&["drafted", "fail"]),
        class: SeatClass::Work,
        body: StepBody::Dialect {
            execution: crate::bundle::DialectExecution {
                argv: strings(&["true"]),
                state: None,
            },
        },
    };
    let input = json!({
        "feature": "feature", "phase": "design", "workdir": ".",
        "allowed_results": ["drafted", "fail"], "context": {},
        "steps": [{"role_path": "", "result_path": "result.json", "allowed_results": ["drafted", "fail"]}],
    });
    let deadline = std::time::Duration::from_secs(5);
    let at = ("effect", "attempt", "work");
    engine
        .execute_sequence(
            at.0,
            at.1,
            at.2,
            &[step],
            &input,
            deadline,
            &Selection::new(),
        )
        .unwrap();
    // Any other executable is not the stand-in, and records nothing.
    let recorded = std::fs::read_to_string(&record).ok();
    let spawned = recorded.map(|argv| argv.lines().map(str::to_string).collect());
    let events = engine.store.load(&engine.run_id).unwrap();
    let error = events.last().unwrap().payload["error"].clone();
    (stand_in, spawned, error)
}
