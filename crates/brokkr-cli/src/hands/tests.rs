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

fn parse_serve_cli(work: &std::path::Path, extra: &[&str]) -> HandsCommand {
    use clap::Parser;
    let work_arg = work.to_str().unwrap();
    let root_path = work.join("artifacts");
    let root_arg = root_path.to_str().unwrap();
    let mut argv = vec![
        "brokkr",
        "hands",
        "serve",
        "--workdir",
        work_arg,
        "--artifacts",
        root_arg,
    ];
    argv.extend_from_slice(extra);
    let crate::Cmd::Hands { command } = crate::Cli::try_parse_from(argv)
        .expect("clap must parse hands serve")
        .command
    else {
        panic!("expected hands serve");
    };
    command
}

/// #1277 B3: `hands serve` takes each document in JSON or unpadded base64url,
/// and both spellings decode into the exact same `Sealed` and `HandsSpec`.
#[test]
fn hands_serve_accepts_base64url_spellings_and_parses_identically_to_json() {
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

    let mixed =
        resolve_serve_documents(Some(reach_json), None, None, Some(spec_b64.clone())).unwrap();
    assert_eq!(mixed, plain);

    assert!(matches!(
        parse_serve_cli(
            tmp.path(),
            &["--reach-base64url", &reach_b64, "--spec-base64url", &spec_b64]
        ),
        HandsCommand::Serve {
            reach: None,
            reach_base64url: Some(ref r),
            spec: None,
            spec_base64url: Some(ref s),
            ..
        } if r == &reach_b64 && s == &spec_b64
    ));
}

/// #1277 R1: omitting `--spec` defaults to main's `"workspace"`, parsed by the
/// same code, while explicit `--spec` plus `--spec-base64url` still refuses;
/// `brokkr hands serve --help` shows the default as main does.
#[test]
fn hands_serve_defaults_omitted_spec_to_workspace_and_shows_default_in_help() {
    use clap::CommandFactory;

    let tmp = tempfile::tempdir().unwrap();
    let reach_json = reach(tmp.path());
    let reach_b64 = hands::encode_base64url(reach_json.as_bytes());
    let spec_b64 = hands::encode_base64url(DEFAULT_SPEC.as_bytes());

    let main_default = resolve_serve_documents(
        Some(reach_json.clone()),
        None,
        Some(DEFAULT_SPEC.to_string()),
        None,
    )
    .unwrap();
    let omitted_plain =
        resolve_serve_documents(Some(reach_json.clone()), None, None, None).unwrap();
    let omitted_encoded = resolve_serve_documents(None, Some(reach_b64), None, None).unwrap();
    assert_eq!(omitted_plain, main_default);
    assert_eq!(omitted_encoded, main_default);
    assert_eq!(omitted_plain.0, HandsSpec::default());

    assert!(matches!(
        parse_serve_cli(tmp.path(), &["--reach", &reach_json]),
        HandsCommand::Serve {
            spec: Some(ref s),
            spec_base64url: None,
            ..
        } if s == DEFAULT_SPEC
    ));
    assert!(matches!(
        parse_serve_cli(
            tmp.path(),
            &[
                "--reach",
                &reach_json,
                "--spec",
                DEFAULT_SPEC,
                "--spec-base64url",
                &spec_b64,
            ]
        ),
        HandsCommand::Serve {
            spec: Some(ref s),
            spec_base64url: Some(ref b),
            ..
        } if s == DEFAULT_SPEC && b == &spec_b64
    ));

    let cmd = crate::Cli::command();
    let help = cmd
        .find_subcommand("hands")
        .unwrap()
        .find_subcommand("serve")
        .unwrap()
        .clone()
        .render_help()
        .to_string();
    assert!(
        help.contains("[default: \"workspace\"]"),
        "expected default in help:\n{help}"
    );
}

/// #1277 R2: leading-hyphen values on `--reach-base64url` and
/// `--spec-base64url` (both space and `=` forms) reach the typed decoder
/// rather than failing at clap as an unknown flag.
#[test]
fn hands_serve_passes_leading_hyphen_encoded_values_to_decoder() {
    let tmp = tempfile::tempdir().unwrap();
    let valid_reach = reach(tmp.path());
    let hyphen = "-wNotADoc123";
    let reach_eq = format!("--reach-base64url={hyphen}");
    let spec_eq = format!("--spec-base64url={hyphen}");

    for (extra, expected_flag) in [
        (&["--reach-base64url", hyphen][..], REACH_BASE64URL_FLAG),
        (&[reach_eq.as_str()][..], REACH_BASE64URL_FLAG),
        (
            &["--reach", &valid_reach, "--spec-base64url", hyphen][..],
            SPEC_BASE64URL_FLAG,
        ),
        (
            &["--reach", &valid_reach, spec_eq.as_str()][..],
            SPEC_BASE64URL_FLAG,
        ),
    ] {
        let HandsCommand::Serve {
            reach,
            reach_base64url,
            spec,
            spec_base64url,
            ..
        } = parse_serve_cli(tmp.path(), extra)
        else {
            panic!("expected hands serve");
        };
        let err =
            resolve_serve_documents(reach, reach_base64url, spec, spec_base64url).unwrap_err();
        assert!(
            matches!(
                err,
                DocumentError::Encoded {
                    flag,
                    cause: Base64UrlError::NotUtf8
                } if flag == expected_flag
            ),
            "{err}"
        );
        assert!(!err.to_string().contains(hyphen));
    }
}

/// #1277 B3 + R1: missing reach or duplicate spellings refuse with typed
/// `DocumentError` variants naming the arguments and never echoing values.
#[test]
fn hands_serve_refuses_missing_reach_or_duplicate_spellings() {
    let tmp = tempfile::tempdir().unwrap();
    let valid_reach = reach(tmp.path());
    let valid_reach_b64 = hands::encode_base64url(valid_reach.as_bytes());
    let valid_spec = DEFAULT_SPEC.to_string();
    let valid_spec_b64 = hands::encode_base64url(valid_spec.as_bytes());

    let err = resolve_serve_documents(None, None, Some(valid_spec.clone()), None).unwrap_err();
    assert!(matches!(err, DocumentError::MissingReach));
    assert_eq!(
        err.to_string(),
        "--reach: missing; pass either --reach or --reach-base64url"
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
    let valid_spec = DEFAULT_SPEC.to_string();
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
        assert!(matches!(
            reach_err,
            DocumentError::Encoded {
                flag: "--reach-base64url",
                cause
            } if cause == expected
        ));
        assert_eq!(
            reach_err.to_string(),
            format!("--reach-base64url: {expected}")
        );
        assert!(!reach_err.to_string().contains(bad));

        let spec_err =
            resolve_serve_documents(Some(valid_reach.clone()), None, None, Some(bad.to_string()))
                .unwrap_err();
        assert!(matches!(
            spec_err,
            DocumentError::Encoded {
                flag: "--spec-base64url",
                cause
            } if cause == expected
        ));
        assert_eq!(
            spec_err.to_string(),
            format!("--spec-base64url: {expected}")
        );
        assert!(!spec_err.to_string().contains(bad));
    }
}

/// #1277 R3 + R6: valid unpadded base64url UTF-8 that fails JSON parse or
/// document checks refuses with typed variants naming the argument and
/// failure class (plus `SpecRefusalKind`'s variant name) and holds no
/// decoded field name, value, or serde fragment in `Display` or `Debug`.
#[test]
fn hands_serve_refuses_decoded_documents_without_disclosing_decoded_text() {
    let tmp = tempfile::tempdir().unwrap();
    let valid_reach = reach(tmp.path());
    let valid_spec = DEFAULT_SPEC.to_string();
    let marker = "SECRET_DECODED_MARKER_1277";
    let b64 = |text: String| hands::encode_base64url(text.as_bytes());
    let assert_redacted = |err: &DocumentError, expected: &str| {
        assert_eq!(err.to_string(), expected);
        assert!(!err.to_string().contains(marker));
        assert!(!format!("{err:?}").contains(marker));
    };

    for bad_json in [
        format!("{{{marker}"),
        format!(r#"{{"{marker}":"\uD800"}}"#),
        r#"{"home":"\uD800"}"#.to_string(),
    ] {
        let err =
            resolve_serve_documents(None, Some(b64(bad_json)), Some(valid_spec.clone()), None)
                .unwrap_err();
        assert!(matches!(
            err,
            DocumentError::EncodedNotJson {
                flag: "--reach-base64url"
            }
        ));
        assert_redacted(
            &err,
            "--reach-base64url: decoded document is not valid JSON",
        );
    }

    let reach_bad = b64(format!(r#"{{"{marker}_FIELD":"{marker}_VALUE"}}"#));
    let err = resolve_serve_documents(None, Some(reach_bad), Some(valid_spec), None).unwrap_err();
    assert!(matches!(err, DocumentError::EncodedReach));
    assert_redacted(
        &err,
        &format!(
            "--reach-base64url: {}: decoded document is not a valid sealed reach",
            brokkr_protocol::broker::Refusal::Identity
        ),
    );

    let spec_not_json = b64(format!("{{{marker}"));
    let err = resolve_serve_documents(Some(valid_reach.clone()), None, None, Some(spec_not_json))
        .unwrap_err();
    assert!(matches!(
        err,
        DocumentError::EncodedNotJson {
            flag: "--spec-base64url"
        }
    ));
    assert_redacted(&err, "--spec-base64url: decoded document is not valid JSON");

    let spec_wrong_key = b64(format!(
        r#"{{"kind":"workspace","{marker}_FIELD":"{marker}_VALUE"}}"#
    ));
    let err = resolve_serve_documents(Some(valid_reach.clone()), None, None, Some(spec_wrong_key))
        .unwrap_err();
    assert!(matches!(
        err,
        DocumentError::EncodedSpec {
            cause: SpecRefusalKind::UnknownKey
        }
    ));
    assert_redacted(
        &err,
        "--spec-base64url: decoded document is not a valid hands spec (UnknownKey)",
    );

    let spec_wrong_val = b64(format!(
        r#"{{"kind":"workspace","network":"{marker}_VALUE"}}"#
    ));
    let err = resolve_serve_documents(Some(valid_reach.clone()), None, None, Some(spec_wrong_val))
        .unwrap_err();
    assert!(matches!(
        err,
        DocumentError::EncodedSpec {
            cause: SpecRefusalKind::Network
        }
    ));
    assert_redacted(
        &err,
        "--spec-base64url: decoded document is not a valid hands spec (Network)",
    );

    let spec_wrong_bind = b64(format!(
        r#"{{"kind":"workspace","binds":[{{"path":"/{marker}","mode":"{marker}"}}]}}"#
    ));
    let err =
        resolve_serve_documents(Some(valid_reach), None, None, Some(spec_wrong_bind)).unwrap_err();
    assert!(matches!(
        err,
        DocumentError::EncodedSpec {
            cause: SpecRefusalKind::Bind(BindRefusalKind::Mode)
        }
    ));
    assert_redacted(
        &err,
        "--spec-base64url: decoded document is not a valid hands spec (Bind::Mode)",
    );
}

/// #1277 R3 + R6: every `SpecRefusal` and `BindRefusal` variant reduces to its
/// payload-free `SpecRefusalKind` with its exact variant name and drops all
/// decoded strings from both `Display` and `Debug`.
#[test]
fn spec_refusal_kind_drops_decoded_fields_for_every_variant() {
    let marker = "SECRET_SPEC_MARKER_1277";
    let cases = [
        (
            SpecRefusal::UnknownKind {
                kind: marker.to_string(),
            },
            SpecRefusalKind::UnknownKind,
            "UnknownKind",
        ),
        (
            SpecRefusal::NotKindOrObject {
                got: marker.to_string(),
            },
            SpecRefusalKind::NotKindOrObject,
            "NotKindOrObject",
        ),
        (SpecRefusal::Boundary, SpecRefusalKind::Boundary, "Boundary"),
        (
            SpecRefusal::UnknownKey {
                key: marker.to_string(),
            },
            SpecRefusalKind::UnknownKey,
            "UnknownKey",
        ),
        (SpecRefusal::Kind, SpecRefusalKind::Kind, "Kind"),
        (
            SpecRefusal::Network {
                got: marker.to_string(),
            },
            SpecRefusalKind::Network,
            "Network",
        ),
        (
            SpecRefusal::BindsNotArray,
            SpecRefusalKind::BindsNotArray,
            "BindsNotArray",
        ),
        (
            SpecRefusal::Bind(BindRefusal::NotObject),
            SpecRefusalKind::Bind(BindRefusalKind::NotObject),
            "Bind::NotObject",
        ),
        (
            SpecRefusal::Bind(BindRefusal::UnknownKey {
                key: marker.to_string(),
            }),
            SpecRefusalKind::Bind(BindRefusalKind::UnknownKey),
            "Bind::UnknownKey",
        ),
        (
            SpecRefusal::Bind(BindRefusal::NoPath),
            SpecRefusalKind::Bind(BindRefusalKind::NoPath),
            "Bind::NoPath",
        ),
        (
            SpecRefusal::Bind(BindRefusal::Mode {
                path: marker.to_string(),
            }),
            SpecRefusalKind::Bind(BindRefusalKind::Mode),
            "Bind::Mode",
        ),
        (
            SpecRefusal::Bind(BindRefusal::MaskNotArray {
                path: marker.to_string(),
            }),
            SpecRefusalKind::Bind(BindRefusalKind::MaskNotArray),
            "Bind::MaskNotArray",
        ),
        (
            SpecRefusal::Bind(BindRefusal::MaskName {
                path: marker.to_string(),
            }),
            SpecRefusalKind::Bind(BindRefusalKind::MaskName),
            "Bind::MaskName",
        ),
    ];
    for (refusal, expected_kind, expected_name) in cases {
        let kind = SpecRefusalKind::from(refusal);
        assert_eq!(kind, expected_kind);
        assert_eq!(kind.to_string(), expected_name);
        let err = DocumentError::EncodedSpec { cause: kind };
        assert_eq!(
            err.to_string(),
            format!(
                "--spec-base64url: decoded document is not a valid hands spec ({expected_name})"
            )
        );
        assert!(!err.to_string().contains(marker));
        assert!(!format!("{err:?}").contains(marker));
    }
}

/// #1277 R4 + R8: omitting `--reach` (and `--reach-base64url`) fails at clap
/// with exit code 2 before `--spec` is parsed, matching main's exit code,
/// diagnostic, and precedence.
#[test]
fn hands_serve_requires_reach_at_clap_before_parsing_spec() {
    use clap::Parser;
    let tmp = tempfile::tempdir().unwrap();
    let work_arg = tmp.path().to_str().unwrap();
    let root_path = tmp.path().join("artifacts");
    let root_arg = root_path.to_str().unwrap();

    for extra in [&[][..], &["--spec", "{"][..]] {
        let mut argv = vec![
            "brokkr",
            "hands",
            "serve",
            "--workdir",
            work_arg,
            "--artifacts",
            root_arg,
        ];
        argv.extend_from_slice(extra);
        let Err(err) = crate::Cli::try_parse_from(argv) else {
            panic!("expected clap to refuse missing --reach");
        };
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
        assert_eq!(err.exit_code(), 2);
        let rendered = err.to_string();
        assert!(
            rendered
                .contains("the following required arguments were not provided:\n  --reach <REACH>"),
            "{rendered}"
        );
    }
}

/// #1277 R4 + R6: plain JSON `--reach` and `--spec` refusals keep main's
/// diagnostics byte for byte on both `hands serve` and `hands exec`.
#[test]
fn plain_json_refusals_match_main_byte_for_byte_on_both_verbs() {
    let tmp = tempfile::tempdir().unwrap();
    let unreached = |_: &str| -> Result<Session, SessionError> { panic!("no session is made") };
    let (workdir, artifacts) = (tmp.path().to_path_buf(), tmp.path().join("artifacts"));
    let valid_reach = reach(&workdir);

    let run_pair = |spec: &str, reach_arg: &str| {
        let serve = HandsCommand::Serve {
            workdir: workdir.clone(),
            artifacts: artifacts.clone(),
            reach: Some(reach_arg.to_string()),
            reach_base64url: None,
            spec: Some(spec.to_string()),
            spec_base64url: None,
        };
        let exec = HandsCommand::Exec {
            workdir: workdir.clone(),
            artifacts: artifacts.clone(),
            reach: reach_arg.to_string(),
            bundle_root: None,
            spec: spec.to_string(),
            command: vec!["true".to_string()],
        };
        (
            run_with(serve, unreached).unwrap_err().to_string(),
            run_with(exec, unreached).unwrap_err().to_string(),
        )
    };

    let (serve_err, exec_err) = run_pair("{", &valid_reach);
    assert_eq!(serve_err, "EOF while parsing an object at line 1 column 1");
    assert_eq!(exec_err, "EOF while parsing an object at line 1 column 1");

    assert!(matches!(
        parse_spec_plain(r#"{"kind":"mitten"}"#),
        Err(DocumentError::PlainSpec {
            cause: SpecRefusal::Kind
        })
    ));
    let (serve_err, exec_err) = run_pair(r#"{"kind":"mitten"}"#, &valid_reach);
    assert_eq!(serve_err, r#"--spec: hands.kind must be "workspace""#);
    assert_eq!(exec_err, r#"--spec: hands.kind must be "workspace""#);

    let (serve_err, exec_err) = run_pair(DEFAULT_SPEC, r#"{"home":"/h"}"#);
    let expected_reach = format!(
        "--reach: {}: missing field `worktree` at line 1 column 13",
        brokkr_protocol::broker::Refusal::Identity
    );
    assert_eq!(serve_err, expected_reach);
    assert_eq!(exec_err, expected_reach);
}
