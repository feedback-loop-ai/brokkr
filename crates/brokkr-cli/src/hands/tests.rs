use super::*;

/// The reach a default box in `work` is sealed as `--reach` carries it,
/// its HOME beside the worktree (U8a2c).
fn reach(work: &std::path::Path) -> String {
    let (spec, home) = (hands::HandsSpec::default(), work.join("home"));
    let sealed = hands::seal(&spec, &home, work, &hands::git_facts(work));
    serde_json::to_string(&sealed).unwrap()
}

/// U8a2c: a reach that does not read refuses both verbs with MB3's
/// identity cause before any session or box is made.
#[test]
fn a_reach_that_does_not_read_refuses_both_verbs_before_any_box() {
    let tmp = tempfile::tempdir().unwrap();
    let unreached = |_: &str| -> Result<Session, SessionError> { panic!("no session is made") };
    let (workdir, artifacts) = (tmp.path().to_path_buf(), tmp.path().join("artifacts"));
    let serve = HandsCommand::Serve {
        workdir: workdir.clone(),
        artifacts: artifacts.clone(),
        reach: "{".to_string(),
        spec: "\"workspace\"".to_string(),
    };
    let exec = HandsCommand::Exec {
        workdir,
        artifacts,
        reach: "{\"home\":\"/h\"}".to_string(),
        bundle_root: None,
        spec: "\"workspace\"".to_string(),
        command: vec!["true".to_string()],
    };
    let cause = "--reach: MCP server box filesystem identity is not protected";
    let serve = run_with(serve, unreached).unwrap_err().to_string();
    assert_eq!(
        serve,
        format!("{cause}: EOF while parsing an object at line 1 column 1")
    );
    let exec = run_with(exec, unreached).unwrap_err().to_string();
    let missing = "missing field `worktree` at line 1 column 13";
    assert_eq!(exec, format!("{cause}: {missing}"));
}

/// Decision 0043 through the in-process verb: a boxed command runs whole
/// and returns its own code, built from the reach `--reach` carries
/// (U8a2c), and a spec that does not parse is refused before anything
/// spawns. Linux only, like the boundary. Moved here from the crate's
/// root suite, which sits at its line baseline, by U8a2c.
#[cfg(target_os = "linux")]
#[test]
fn the_hands_exec_verb_boxes_a_command_and_refuses_a_bad_spec() {
    use crate::tests::cli;
    use crate::{run, Cmd};
    use hands::{boundary_evidence_required, skip_boundary_proof, HANDS_BOX_ENV};
    let required = boundary_evidence_required();
    if std::env::var_os(HANDS_BOX_ENV).is_some() {
        skip_boundary_proof(required, "this environment is already a box");
        return;
    }
    let namespace = std::process::Command::new("bwrap")
        .args(["--ro-bind", "/", "/", "--", "true"])
        .output();
    if !namespace.is_ok_and(|output| output.status.success()) {
        let reason = "this environment cannot create a bubblewrap namespace";
        skip_boundary_proof(required, reason);
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let (work, artifacts) = (dir.path().join("work"), dir.path().join("artifacts"));
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir_all(&artifacts).unwrap();
    let exec = |spec: &str, command: &[&str]| HandsCommand::Exec {
        workdir: work.clone(),
        artifacts: artifacts.clone(),
        reach: reach(&work),
        bundle_root: None,
        spec: spec.to_string(),
        command: command.iter().map(|part| part.to_string()).collect(),
    };
    let boxed = ["--", "bash", "-c", "echo boxed > out.txt; exit 3"];
    let code = run(cli(Cmd::Hands {
        command: exec("\"workspace\"", &boxed),
    }));
    assert_eq!(code.unwrap(), ExitCode::from(3));
    assert_eq!(
        std::fs::read_to_string(work.join("out.txt")).unwrap(),
        "boxed\n"
    );

    // Only the leading separator is the verb's: a command that carries
    // its own `--` keeps it, where the first pass stripped every one.
    let kept = [
        "--",
        "bash",
        "-c",
        "printf '%s\\n' \"$@\" > args.txt",
        "sh",
        "--",
        "kept",
    ];
    let code = run(cli(Cmd::Hands {
        command: exec("\"workspace\"", &kept),
    }));
    assert_eq!(code.unwrap(), ExitCode::SUCCESS);
    assert_eq!(
        std::fs::read_to_string(work.join("args.txt")).unwrap(),
        "--\nkept\n"
    );

    let refused = run(cli(Cmd::Hands {
        command: exec("{\"kind\":\"mitten\"}", &["true"]),
    }));
    let refused = refused.unwrap_err().to_string();
    assert!(refused.contains("--spec"), "{refused}");
}

/// A session that cannot lock stops `hands serve` before it serves, and
/// the one line the binary prints names the lock and the kernel's words.
#[test]
fn a_session_that_cannot_lock_prints_the_lock_and_the_errno() {
    let tmp = tempfile::tempdir().unwrap();
    let nolck = || std::io::Error::from_raw_os_error(rustix::io::Errno::NOLCK.raw_os_error());
    let tree = tmp.path().join("brokkr-hands-serve-tree");
    let refused = |label: &str| {
        assert_eq!(label, "serve");
        Err(SessionError::Lock {
            path: tree.join(".owner.lock"),
            cause: nolck(),
        })
    };
    let serve = HandsCommand::Serve {
        workdir: tmp.path().to_path_buf(),
        artifacts: tmp.path().join("artifacts"),
        reach: reach(tmp.path()),
        spec: "\"workspace\"".to_string(),
    };
    let error = run_with(serve, refused).unwrap_err();
    let mut stderr = Vec::new();
    assert_eq!(crate::report_to(&error, &mut stderr), ExitCode::from(1));
    assert_eq!(
        String::from_utf8(stderr).unwrap(),
        format!(
            "error: hands session: cannot lock {}/.owner.lock: {}\n",
            tree.display(),
            nolck()
        )
    );
}
