use super::*;

/// The reach a default box in `work` is sealed as `--reach` carries it,
/// its HOME beside the worktree (U8a2c).
fn reach(work: &std::path::Path) -> String {
    let (spec, home) = (hands::HandsSpec::default(), work.join("home"));
    let sealed = hands::seal(&spec, &home, work, &hands::git_facts(work), None);
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
        reach: Some("{".to_string()),
        reach_base64url: None,
        spec: Some("\"workspace\"".to_string()),
        spec_base64url: None,
    };
    let exec = HandsCommand::Exec {
        workdir,
        artifacts,
        reach: "{\"home\":\"/h\"}".to_string(),
        bundle_root: None,
        spec: "\"workspace\"".to_string(),
        command: vec!["true".to_string()],
    };
    let cause = format!("--reach: {}", brokkr_protocol::broker::Refusal::Identity);
    let serve = run_with(serve, unreached).unwrap_err().to_string();
    assert_eq!(
        serve,
        format!("{cause}: EOF while parsing an object at line 1 column 1")
    );
    let exec = run_with(exec, unreached).unwrap_err().to_string();
    let missing = "missing field `worktree` at line 1 column 13";
    assert_eq!(exec, format!("{cause}: {missing}"));
}

/// U8h (operator ruling 1 of 2026-10-23): a reach sealed before the store
/// was sealed in it, its every other field as admission writes it, no
/// longer reads, and refuses both verbs with MB3's identity cause before
/// any session or box is made, never a box that hides nothing.
#[test]
fn a_reach_sealed_without_the_store_refuses_both_verbs_before_any_box() {
    let tmp = tempfile::tempdir().unwrap();
    let unreached = |_: &str| -> Result<Session, SessionError> { panic!("no session is made") };
    let (workdir, artifacts) = (tmp.path().to_path_buf(), tmp.path().join("artifacts"));
    let mut sealed: serde_json::Value = serde_json::from_str(&reach(&workdir)).unwrap();
    assert_eq!(sealed["store"], serde_json::Value::Null);
    sealed.as_object_mut().unwrap().remove("store");
    let legacy = sealed.to_string();
    let serve = HandsCommand::Serve {
        workdir: workdir.clone(),
        artifacts: artifacts.clone(),
        reach: Some(legacy.clone()),
        reach_base64url: None,
        spec: Some("\"workspace\"".to_string()),
        spec_base64url: None,
    };
    let exec = HandsCommand::Exec {
        workdir,
        artifacts,
        reach: legacy.clone(),
        bundle_root: None,
        spec: "\"workspace\"".to_string(),
        command: vec!["true".to_string()],
    };
    let refused = format!(
        "--reach: {}: missing field `store` at line 1 column {}",
        brokkr_protocol::broker::Refusal::Identity,
        legacy.len()
    );
    assert_eq!(run_with(serve, unreached).unwrap_err().to_string(), refused);
    assert_eq!(run_with(exec, unreached).unwrap_err().to_string(), refused);
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
        reach: Some(reach(tmp.path())),
        reach_base64url: None,
        spec: Some("\"workspace\"".to_string()),
        spec_base64url: None,
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

/// #1277 B3: `hands serve` takes each document in JSON or unpadded base64url,
/// and both spellings decode into the exact same `Sealed` and `HandsSpec`.
#[test]
fn hands_serve_accepts_base64url_spellings_and_parses_identically_to_json() {
    use clap::Parser;

    let tmp = tempfile::tempdir().unwrap();
    let reach_json = reach(tmp.path());
    let spec_json = serde_json::json!({
        "kind": "workspace",
        "network": false,
        "binds": [{"path": "~/.cargo", "mode": "overlay", "mask": ["credentials"]}]
    })
    .to_string();
    let reach_b64 = hands::encode_base64url(reach_json.as_bytes());
    let spec_b64 = hands::encode_base64url(spec_json.as_bytes());

    let plain = resolve_serve_documents(
        Some(reach_json.clone()),
        None,
        Some(spec_json.clone()),
        None,
    )
    .unwrap();
    let encoded =
        resolve_serve_documents(None, Some(reach_b64.clone()), None, Some(spec_b64.clone()))
            .unwrap();
    assert_eq!(encoded, plain);

    // Mixed spellings (one JSON, one base64url) also resolve identically.
    let mixed =
        resolve_serve_documents(Some(reach_json), None, None, Some(spec_b64.clone())).unwrap();
    assert_eq!(mixed, plain);

    // Clap parses the new flags under the hidden `hands` root verb.
    let parsed = crate::Cli::try_parse_from([
        "brokkr",
        "hands",
        "serve",
        "--workdir",
        tmp.path().to_str().unwrap(),
        "--artifacts",
        tmp.path().join("artifacts").to_str().unwrap(),
        "--reach-base64url",
        &reach_b64,
        "--spec-base64url",
        &spec_b64,
    ])
    .unwrap();
    assert!(matches!(
        parsed.command,
        crate::Cmd::Hands {
            command: HandsCommand::Serve {
                reach: None,
                reach_base64url: Some(ref r),
                spec: None,
                spec_base64url: Some(ref s),
                ..
            }
        } if r == &reach_b64 && s == &spec_b64
    ));
}

/// #1277 B3: both spellings given or neither given refuses with typed
/// `DocumentError` variants naming the arguments and never echoing values.
#[test]
fn hands_serve_refuses_missing_or_duplicate_spellings() {
    let tmp = tempfile::tempdir().unwrap();
    let valid_reach = reach(tmp.path());
    let valid_reach_b64 = hands::encode_base64url(valid_reach.as_bytes());
    let valid_spec = "\"workspace\"".to_string();
    let valid_spec_b64 = hands::encode_base64url(valid_spec.as_bytes());

    let err = resolve_serve_documents(None, None, Some(valid_spec.clone()), None).unwrap_err();
    assert!(matches!(
        err,
        DocumentError::Missing {
            plain: "--reach",
            encoded: "--reach-base64url"
        }
    ));
    assert_eq!(
        err.to_string(),
        "--reach: missing; pass either --reach or --reach-base64url"
    );

    let err = resolve_serve_documents(Some(valid_reach.clone()), None, None, None).unwrap_err();
    assert!(matches!(
        err,
        DocumentError::Missing {
            plain: "--spec",
            encoded: "--spec-base64url"
        }
    ));
    assert_eq!(
        err.to_string(),
        "--spec: missing; pass either --spec or --spec-base64url"
    );

    let err = resolve_serve_documents(
        Some(valid_reach.clone()),
        Some(valid_reach_b64.clone()),
        Some(valid_spec.clone()),
        None,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        DocumentError::BothSpellings {
            plain: "--reach",
            encoded: "--reach-base64url"
        }
    ));
    assert_eq!(
        err.to_string(),
        "--reach: both --reach and --reach-base64url were given; pass only one"
    );
    assert!(!err.to_string().contains(&valid_reach_b64));

    let err = resolve_serve_documents(
        Some(valid_reach),
        None,
        Some(valid_spec),
        Some(valid_spec_b64.clone()),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        DocumentError::BothSpellings {
            plain: "--spec",
            encoded: "--spec-base64url"
        }
    ));
    assert_eq!(
        err.to_string(),
        "--spec: both --spec and --spec-base64url were given; pass only one"
    );
    assert!(!err.to_string().contains(&valid_spec_b64));
}

/// #1277 B3: padded, non-alphabet, invalid-length, non-canonical trailing
/// bits, or non-UTF-8 values on `--reach-base64url` or `--spec-base64url`
/// refuse with typed `DocumentError::Encoded`, never echoing the input.
#[test]
fn hands_serve_refuses_malformed_base64url_documents() {
    let tmp = tempfile::tempdir().unwrap();
    let valid_reach = reach(tmp.path());
    let valid_spec = "\"workspace\"".to_string();
    let non_utf8 = hands::encode_base64url(&[0xff, 0xfe]);

    for (bad, expected) in [
        ("Zg==", Base64UrlError::Padding),
        ("Zm9v!", Base64UrlError::InvalidAlphabet),
        ("A", Base64UrlError::InvalidLength),
        ("AB", Base64UrlError::NonZeroTrailingBits),
        (non_utf8.as_str(), Base64UrlError::NotUtf8),
    ] {
        let reach_err =
            resolve_serve_documents(None, Some(bad.to_string()), Some(valid_spec.clone()), None)
                .unwrap_err();
        assert!(
            matches!(
                reach_err,
                DocumentError::Encoded {
                    flag: "--reach-base64url",
                    cause
                } if cause == expected
            ),
            "{reach_err}"
        );
        assert_eq!(
            reach_err.to_string(),
            format!("--reach-base64url: {expected}")
        );
        assert!(!reach_err.to_string().contains(bad));

        let spec_err =
            resolve_serve_documents(Some(valid_reach.clone()), None, None, Some(bad.to_string()))
                .unwrap_err();
        assert!(
            matches!(
                spec_err,
                DocumentError::Encoded {
                    flag: "--spec-base64url",
                    cause
                } if cause == expected
            ),
            "{spec_err}"
        );
        assert_eq!(
            spec_err.to_string(),
            format!("--spec-base64url: {expected}")
        );
        assert!(!spec_err.to_string().contains(bad));
    }
}

/// #1277 B3: valid unpadded base64url UTF-8 that is not a valid reach or
/// spec document runs through the same parse checks and never echoes the
/// encoded argument value.
#[test]
fn hands_serve_refuses_decoded_documents_that_fail_parse() {
    let tmp = tempfile::tempdir().unwrap();
    let valid_reach = reach(tmp.path());
    let valid_spec = "\"workspace\"".to_string();

    let bad_reach_b64 = hands::encode_base64url(b"{\"home\":\"/h\"}");
    let err = resolve_serve_documents(None, Some(bad_reach_b64.clone()), Some(valid_spec), None)
        .unwrap_err();
    assert!(matches!(
        err,
        DocumentError::Reach {
            flag: "--reach-base64url",
            ..
        }
    ));
    assert_eq!(
        err.to_string(),
        format!(
            "--reach-base64url: {}: missing field `worktree` at line 1 column 13",
            brokkr_protocol::broker::Refusal::Identity
        )
    );
    assert!(!err.to_string().contains(&bad_reach_b64));

    let bad_spec_json_b64 = hands::encode_base64url(b"{");
    let err = resolve_serve_documents(
        Some(valid_reach.clone()),
        None,
        None,
        Some(bad_spec_json_b64.clone()),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        DocumentError::SpecJson {
            flag: "--spec-base64url",
            ..
        }
    ));
    assert_eq!(
        err.to_string(),
        "--spec-base64url: EOF while parsing an object at line 1 column 1"
    );
    assert!(!err.to_string().contains(&bad_spec_json_b64));

    let bad_spec_schema_b64 = hands::encode_base64url(b"{\"kind\":\"mitten\"}");
    let err = resolve_serve_documents(
        Some(valid_reach),
        None,
        None,
        Some(bad_spec_schema_b64.clone()),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        DocumentError::Spec {
            flag: "--spec-base64url",
            ..
        }
    ));
    assert_eq!(
        err.to_string(),
        "--spec-base64url: hands.kind must be \"workspace\""
    );
    assert!(!err.to_string().contains(&bad_spec_schema_b64));
}
