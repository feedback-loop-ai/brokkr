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

/// #1277 V1 + R3 + R6: valid unpadded base64url UTF-8 that fails JSON parse or
/// document checks refuses with typed variants naming the argument and
/// failure class (serde `Category` or `SpecRefusalKind`) and holds no
/// decoded field name, value, or serde fragment in `Display` or `Debug`.
#[test]
fn hands_serve_refuses_decoded_documents_without_disclosing_decoded_text() {
    use serde_json::error::Category;
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

    for (bad_doc, expected_cat, cat_name) in [
        (format!("{{{marker}"), Category::Syntax, "Syntax"),
        (
            r#"{"home":"\uD800"}"#.to_string(),
            Category::Syntax,
            "Syntax",
        ),
        (String::new(), Category::Eof, "Eof"),
        (
            format!(r#"{{"{marker}":"\uD800"}}"#),
            Category::Data,
            "Data",
        ),
        (
            format!(r#"{{"{marker}_FIELD":"{marker}_VALUE"}}"#),
            Category::Data,
            "Data",
        ),
    ] {
        let err = resolve_serve_documents(None, Some(b64(bad_doc)), Some(valid_spec.clone()), None)
            .unwrap_err();
        assert!(matches!(
            err,
            DocumentError::EncodedReach { category } if category == expected_cat
        ));
        assert_redacted(
            &err,
            &format!(
                "--reach-base64url: {}: decoded document is not a valid sealed reach ({cat_name})",
                brokkr_protocol::broker::Refusal::Identity
            ),
        );
    }

    assert_eq!(json_category_name(Category::Io), "Io");

    for (bad_spec_json, expected_cat, cat_name) in [
        (format!("{{{marker}"), Category::Syntax, "Syntax"),
        (String::new(), Category::Eof, "Eof"),
    ] {
        let err = resolve_serve_documents(
            Some(valid_reach.clone()),
            None,
            None,
            Some(b64(bad_spec_json)),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            DocumentError::EncodedSpecJson { category } if category == expected_cat
        ));
        assert_redacted(
            &err,
            &format!("--spec-base64url: decoded document is not valid JSON ({cat_name})"),
        );
    }

    for (bad_spec, expected_kind, kind_name) in [
        (
            format!(r#"{{"kind":"workspace","{marker}_FIELD":"{marker}_VALUE"}}"#),
            SpecRefusalKind::UnknownKey,
            "UnknownKey",
        ),
        (
            format!(r#"{{"kind":"workspace","network":"{marker}_VALUE"}}"#),
            SpecRefusalKind::Network,
            "Network",
        ),
        (
            format!(r#"{{"kind":"workspace","binds":[{{"path":"/{marker}","mode":"{marker}"}}]}}"#),
            SpecRefusalKind::Bind(BindRefusalKind::Mode),
            "Bind::Mode",
        ),
    ] {
        let err =
            resolve_serve_documents(Some(valid_reach.clone()), None, None, Some(b64(bad_spec)))
                .unwrap_err();
        assert!(matches!(
            err,
            DocumentError::EncodedSpec { cause } if cause == expected_kind
        ));
        assert_redacted(
            &err,
            &format!("--spec-base64url: decoded document is not a valid hands spec ({kind_name})"),
        );
    }
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

/// #1277 V3 + R4 + R8: omitting `--reach` (and `--reach-base64url`) fails at
/// clap with exit code 2 before `--spec` is parsed, printing main's exact
/// usage order (`--reach <REACH>` before `--spec <SPEC>`) on both `hands serve`
/// and `hands exec`.
#[test]
fn hands_serve_requires_reach_at_clap_before_parsing_spec() {
    use clap::Parser;
    let tmp = tempfile::tempdir().unwrap();
    let work_arg = tmp.path().to_str().unwrap();
    let root_path = tmp.path().join("artifacts");
    let root_arg = root_path.to_str().unwrap();

    let cases: &[(&str, &[&str], &str)] = &[
        (
            "serve",
            &[],
            "Usage: brokkr hands serve --workdir <WORKDIR> --artifacts <ARTIFACTS> --reach <REACH>",
        ),
        (
            "serve",
            &["--spec", "{"],
            "Usage: brokkr hands serve --workdir <WORKDIR> --artifacts <ARTIFACTS> --reach <REACH> --spec <SPEC>",
        ),
        (
            "exec",
            &["--", "true"],
            "Usage: brokkr hands exec --workdir <WORKDIR> --artifacts <ARTIFACTS> --reach <REACH> <COMMAND>...",
        ),
        (
            "exec",
            &["--spec", "{", "--", "true"],
            "Usage: brokkr hands exec --workdir <WORKDIR> --artifacts <ARTIFACTS> --reach <REACH> --spec <SPEC> <COMMAND>...",
        ),
    ];

    for (verb, extra, expected_usage) in cases {
        let mut argv = vec![
            "brokkr",
            "hands",
            verb,
            "--workdir",
            work_arg,
            "--artifacts",
            root_arg,
        ];
        argv.extend_from_slice(extra);
        let Err(err) = crate::Cli::try_parse_from(argv) else {
            panic!("expected clap to refuse missing --reach on hands {verb}");
        };
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
        assert_eq!(err.exit_code(), 2);
        let expected = format!(
            "error: the following required arguments were not provided:\n  --reach <REACH>\n\n{expected_usage}\n\nFor more information, try '--help'.\n"
        );
        assert_eq!(err.to_string(), expected);
    }
}

/// #1277 V3 (decision 0071 ruling 8): `reformat_missing_serve_reach` selects
/// the missing-reach diagnostic from structured clap argument state, not from
/// rendered `--reach <REACH>` text: renaming the metavar still preserves
/// main's usage order, while `hands serve` with `--reach-base64url` present
/// (missing `--artifacts`) and non-`hands` missing-required errors pass
/// through unchanged.
#[test]
fn missing_serve_reach_reformat_uses_structured_parser_state_not_metavar_text() {
    use clap::CommandFactory;
    use std::ffi::OsString;

    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().to_str().unwrap();
    let artifacts = tmp.path().join("artifacts");
    let artifacts_arg = artifacts.to_str().unwrap();

    // Custom metavar on `--reach` proves selection does not match "--reach <REACH>".
    let renamed_cmd = || {
        crate::Cli::command().mut_subcommand("hands", |hands| {
            hands.mut_subcommand("serve", |serve| {
                serve.mut_arg("reach", |reach| reach.value_name("SEALED"))
            })
        })
    };
    let argv: Vec<OsString> = [
        "brokkr",
        "hands",
        "serve",
        "--workdir",
        work,
        "--artifacts",
        artifacts_arg,
        "--spec",
        "{",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    let raw_err = renamed_cmd().try_get_matches_from(&argv).unwrap_err();
    let reformatted = reformat_missing_serve_reach(raw_err, &argv, renamed_cmd);
    assert_eq!(
        reformatted.to_string(),
        concat!(
            "error: the following required arguments were not provided:\n  --reach <SEALED>\n\n",
            "Usage: brokkr hands serve --workdir <WORKDIR> --artifacts <ARTIFACTS> --reach <SEALED> --spec <SPEC>\n\n",
            "For more information, try '--help'.\n"
        )
    );

    // When `--reach-base64url` is present and another required flag is missing,
    // `reformat_missing_serve_reach` leaves the original error alone.
    let missing_artifacts: Vec<OsString> = [
        "brokkr",
        "hands",
        "serve",
        "--workdir",
        work,
        "--reach-base64url",
        "e30",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    let raw_artifacts = crate::Cli::command()
        .try_get_matches_from(&missing_artifacts)
        .unwrap_err();
    let raw_text = raw_artifacts.to_string();
    let kept = reformat_missing_serve_reach(raw_artifacts, &missing_artifacts, crate::Cli::command);
    assert_eq!(kept.to_string(), raw_text);
    assert!(
        kept.to_string().contains("--artifacts <ARTIFACTS>"),
        "got: {kept}"
    );
    assert!(!kept.to_string().contains("--reach <REACH>"), "got: {kept}");
}

/// #1277 Z5: when `hands serve` receives plain `--reach` and `--artifacts` is
/// absent, `is_missing_serve_reach` must short-circuit false on
/// `serve.value_source("reach").is_none()` so `reformat_missing_serve_reach`
/// leaves clap's own error untouched instead of marking `--reach`
/// unconditionally required.
#[test]
fn hands_serve_with_plain_reach_and_missing_artifacts_keeps_clap_usage() {
    use clap::Parser;

    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().to_str().unwrap();

    // Passing `--spec` alongside plain `--reach` distinguishes clap's natural
    // error (where `--reach` is conditionally required) from the reformatted
    // error (where `--reach` is forced unconditionally required).
    let Err(err) = crate::Cli::try_parse_from([
        "brokkr",
        "hands",
        "serve",
        "--workdir",
        work,
        "--spec",
        DEFAULT_SPEC,
        "--reach",
        "{}",
    ]) else {
        panic!("expected clap to refuse missing --artifacts on hands serve");
    };
    assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    assert_eq!(err.exit_code(), 2);
    assert_eq!(
        err.to_string(),
        concat!(
            "error: the following required arguments were not provided:\n  --artifacts <ARTIFACTS>\n\n",
            "Usage: brokkr hands serve --workdir <WORKDIR> --artifacts <ARTIFACTS> --spec <SPEC> --reach <REACH>\n\n",
            "For more information, try '--help'.\n"
        )
    );
}

/// #1277 X2 (proposed decision 0125 ruling 4): non-missing-reach clap errors
/// on `hands serve` (such as passing `--reach` twice) keep clap's usage
/// synopsis for the present argument set rather than being rewritten by
/// `reformat_missing_serve_reach`. Pin the exact output so the documents and
/// behaviour cannot part.
#[test]
fn repeated_reach_on_hands_serve_prints_clap_usage_for_present_argument_set() {
    use clap::Parser;

    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().to_str().unwrap();
    let artifacts = tmp.path().join("artifacts");
    let artifacts_arg = artifacts.to_str().unwrap();

    let Err(err) = crate::Cli::try_parse_from([
        "brokkr",
        "hands",
        "serve",
        "--workdir",
        work,
        "--artifacts",
        artifacts_arg,
        "--reach",
        "a",
        "--reach",
        "b",
    ]) else {
        panic!("expected clap to refuse repeated --reach on hands serve");
    };
    assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    assert_eq!(err.exit_code(), 2);
    assert_eq!(
        err.to_string(),
        concat!(
            "error: the argument '--reach <REACH>' cannot be used multiple times\n\n",
            "Usage: brokkr hands serve [OPTIONS] --workdir <WORKDIR> --artifacts <ARTIFACTS>\n\n",
            "For more information, try '--help'.\n"
        )
    );
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

#[derive(Debug, PartialEq, Eq)]
enum RefusalClass {
    Reach(serde_json::error::Category),
    SpecJson(serde_json::error::Category),
    Spec(SpecRefusalKind),
}

fn refusal_class(err: &DocumentError) -> RefusalClass {
    match err {
        DocumentError::PlainReach { cause } => RefusalClass::Reach(cause.classify()),
        DocumentError::EncodedReach { category } => RefusalClass::Reach(*category),
        DocumentError::PlainSpecJson { cause } => RefusalClass::SpecJson(cause.classify()),
        DocumentError::EncodedSpecJson { category } => RefusalClass::SpecJson(*category),
        DocumentError::PlainSpec { cause } => {
            RefusalClass::Spec(SpecRefusalKind::from(cause.clone()))
        }
        DocumentError::EncodedSpec { cause } => RefusalClass::Spec(*cause),
        other => panic!("unexpected document error variant in parity table: {other:?}"),
    }
}

/// Full valid sealed reach JSON with every optional nested struct (`git`,
/// `binds`, `store`) populated so V2 duplicates every field at every level.
fn full_reach_json() -> &'static str {
    concat!(
        r#"{"home":"/h","worktree":{"path":"/w","identity":[1,2]},"#,
        r#""git":{"dir":{"path":"/w/.git","identity":[1,3]},"#,
        r#""common":{"path":"/w/.git","identity":[1,4]}},"#,
        r#""binds":[{"path":"/b","identity":[1,5]}],"#,
        r#""store":{"place":"/s","spelling":"/s"}}"#
    )
}

fn dup_once(base: &str, needle: &str) -> String {
    assert!(base.contains(needle), "expected {needle:?} in base doc");
    base.replacen(needle, &format!("{needle},{needle}"), 1)
}

fn reach_duplicate_cases(base: &str) -> Vec<(&'static str, String)> {
    vec![
        ("dup home", dup_once(base, r#""home":"/h""#)),
        (
            "dup worktree",
            dup_once(base, r#""worktree":{"path":"/w","identity":[1,2]}"#),
        ),
        (
            "dup git",
            dup_once(
                base,
                r#""git":{"dir":{"path":"/w/.git","identity":[1,3]},"common":{"path":"/w/.git","identity":[1,4]}}"#,
            ),
        ),
        (
            "dup binds",
            dup_once(base, r#""binds":[{"path":"/b","identity":[1,5]}]"#),
        ),
        (
            "dup store",
            dup_once(base, r#""store":{"place":"/s","spelling":"/s"}"#),
        ),
        ("dup worktree.path", dup_once(base, r#""path":"/w""#)),
        (
            "dup worktree.identity",
            dup_once(base, r#""identity":[1,2]"#),
        ),
        (
            "dup git.dir",
            dup_once(base, r#""dir":{"path":"/w/.git","identity":[1,3]}"#),
        ),
        (
            "dup git.common",
            dup_once(base, r#""common":{"path":"/w/.git","identity":[1,4]}"#),
        ),
        (
            "dup git.dir.path",
            base.replacen(
                r#""dir":{"path":"/w/.git""#,
                r#""dir":{"path":"/w/.git","path":"/w/.git""#,
                1,
            ),
        ),
        (
            "dup git.dir.identity",
            dup_once(base, r#""identity":[1,3]"#),
        ),
        (
            "dup git.common.path",
            base.replacen(
                r#""common":{"path":"/w/.git""#,
                r#""common":{"path":"/w/.git","path":"/w/.git""#,
                1,
            ),
        ),
        (
            "dup git.common.identity",
            dup_once(base, r#""identity":[1,4]"#),
        ),
        ("dup binds[0].path", dup_once(base, r#""path":"/b""#)),
        (
            "dup binds[0].identity",
            dup_once(base, r#""identity":[1,5]"#),
        ),
        ("dup store.place", dup_once(base, r#""place":"/s""#)),
        ("dup store.spelling", dup_once(base, r#""spelling":"/s""#)),
    ]
}

fn assert_reach_spelling_parity() {
    use serde_json::error::Category;
    let base = full_reach_json();
    let expected_valid: Sealed = serde_json::from_str(base).unwrap();
    // Main's plain `serde_json::from_str::<Sealed>` also admits serde's
    // positional array form; both spellings must admit it with the exact same
    // `Sealed` value (#1277 V2).
    let positional = r#"["/h",["/w",[1,2]],null,[],null]"#;
    let expected_positional: Sealed = serde_json::from_str(
        r#"{"home":"/h","worktree":{"path":"/w","identity":[1,2]},"git":null,"binds":[],"store":null}"#,
    )
    .unwrap();

    let mut cases: Vec<(&str, String, Result<Sealed, RefusalClass>)> = vec![
        ("valid full reach", base.to_string(), Ok(expected_valid)),
        (
            "valid positional reach (main serde tuple form)",
            positional.to_string(),
            Ok(expected_positional),
        ),
    ];
    for (label, doc) in reach_duplicate_cases(base) {
        cases.push((label, doc, Err(RefusalClass::Reach(Category::Data))));
    }
    cases.extend([
        (
            "unknown top member",
            base.replacen('{', r#"{"extra":1,"#, 1),
            Err(RefusalClass::Reach(Category::Data)),
        ),
        (
            "unknown nested member",
            base.replacen(r#""place":"/s""#, r#""place":"/s","extra":1"#, 1),
            Err(RefusalClass::Reach(Category::Data)),
        ),
        (
            "missing store",
            r#"{"home":"/h","worktree":{"path":"/w","identity":null},"git":null,"binds":[]}"#
                .to_string(),
            Err(RefusalClass::Reach(Category::Data)),
        ),
        (
            "missing worktree",
            r#"{"home":"/h","git":null,"binds":[],"store":null}"#.to_string(),
            Err(RefusalClass::Reach(Category::Data)),
        ),
        (
            "wrong type home",
            base.replacen(r#""home":"/h""#, r#""home":123"#, 1),
            Err(RefusalClass::Reach(Category::Data)),
        ),
        (
            "trailing bytes",
            format!("{base} trailing"),
            Err(RefusalClass::Reach(Category::Syntax)),
        ),
        (
            "leading BOM",
            format!("\u{feff}{base}"),
            Err(RefusalClass::Reach(Category::Syntax)),
        ),
        (
            "unpaired surrogate in known field",
            base.replacen(r#""home":"/h""#, r#""home":"\uD800""#, 1),
            Err(RefusalClass::Reach(Category::Syntax)),
        ),
        (
            "unpaired surrogate in key",
            r#"{"\uD800":"x"}"#.to_string(),
            Err(RefusalClass::Reach(Category::Syntax)),
        ),
        // `deny_unknown_fields` refuses the unknown key before scanning its value,
        // so `from_str::<Sealed>` returns `Data` in BOTH spellings.
        (
            "unpaired surrogate in unknown field",
            r#"{"extra":"\uD800"}"#.to_string(),
            Err(RefusalClass::Reach(Category::Data)),
        ),
        (
            "empty document",
            String::new(),
            Err(RefusalClass::Reach(Category::Eof)),
        ),
        (
            "null",
            "null".to_string(),
            Err(RefusalClass::Reach(Category::Data)),
        ),
    ]);

    for (label, doc, expected) in cases {
        let encoded = hands::encode_base64url(doc.as_bytes());
        let plain_res = resolve_serve_documents(Some(doc), None, None, None)
            .map(|(_, sealed)| sealed)
            .map_err(|err| refusal_class(&err));
        let encoded_res = resolve_serve_documents(None, Some(encoded), None, None)
            .map(|(_, sealed)| sealed)
            .map_err(|err| refusal_class(&err));
        assert_eq!(plain_res, expected, "plain reach mismatch on {label}");
        assert_eq!(
            encoded_res, plain_res,
            "encoded vs plain reach mismatch on {label}"
        );
    }
}

fn spec_duplicate_cases() -> Vec<(&'static str, &'static str, HandsSpec)> {
    use brokkr_protocol::hands::{Bind, BindMode};
    let bind = |path: &str, mode: BindMode, mask: &[&str]| Bind {
        path: path.to_string(),
        mode,
        mask: mask.iter().map(|m| (*m).to_string()).collect(),
    };
    let spec_of = |network: bool, binds: Vec<Bind>| HandsSpec { network, binds };
    vec![
        (
            "dup kind (main last-wins)",
            r#"{"kind":"mitten","kind":"workspace"}"#,
            HandsSpec::default(),
        ),
        (
            "dup network (main last-wins)",
            r#"{"kind":"workspace","network":false,"network":true}"#,
            spec_of(true, Vec::new()),
        ),
        (
            "dup binds (main last-wins)",
            r#"{"kind":"workspace","binds":[],"binds":[{"path":"/b","mode":"ro"}]}"#,
            spec_of(false, vec![bind("/b", BindMode::Ro, &[])]),
        ),
        (
            "dup bind.path (main last-wins)",
            r#"{"kind":"workspace","binds":[{"path":"/a","path":"/b","mode":"ro"}]}"#,
            spec_of(false, vec![bind("/b", BindMode::Ro, &[])]),
        ),
        (
            "dup bind.mode (main last-wins)",
            r#"{"kind":"workspace","binds":[{"path":"/b","mode":"rw","mode":"ro"}]}"#,
            spec_of(false, vec![bind("/b", BindMode::Ro, &[])]),
        ),
        (
            "dup bind.mask (main last-wins)",
            r#"{"kind":"workspace","binds":[{"path":"/b","mode":"ro","mask":["a"],"mask":["b"]}]}"#,
            spec_of(false, vec![bind("/b", BindMode::Ro, &["b"])]),
        ),
    ]
}

fn spec_refusal_cases() -> Vec<(&'static str, &'static str, RefusalClass)> {
    use serde_json::error::Category;
    vec![
        (
            "unknown top member",
            r#"{"kind":"workspace","extra":1}"#,
            RefusalClass::Spec(SpecRefusalKind::UnknownKey),
        ),
        (
            "unknown nested bind member",
            r#"{"kind":"workspace","binds":[{"path":"/b","mode":"ro","extra":1}]}"#,
            RefusalClass::Spec(SpecRefusalKind::Bind(BindRefusalKind::UnknownKey)),
        ),
        (
            "missing kind",
            r#"{}"#,
            RefusalClass::Spec(SpecRefusalKind::Kind),
        ),
        (
            "missing bind path",
            r#"{"kind":"workspace","binds":[{"mode":"ro"}]}"#,
            RefusalClass::Spec(SpecRefusalKind::Bind(BindRefusalKind::NoPath)),
        ),
        (
            "wrong type network",
            r#"{"kind":"workspace","network":"yes"}"#,
            RefusalClass::Spec(SpecRefusalKind::Network),
        ),
        (
            "wrong type binds",
            r#"{"kind":"workspace","binds":"no"}"#,
            RefusalClass::Spec(SpecRefusalKind::BindsNotArray),
        ),
        (
            "trailing bytes",
            r#""workspace" trailing"#,
            RefusalClass::SpecJson(Category::Syntax),
        ),
        (
            "leading BOM",
            "\u{feff}\"workspace\"",
            RefusalClass::SpecJson(Category::Syntax),
        ),
        (
            "unpaired surrogate in value",
            r#"{"kind":"\uD800"}"#,
            RefusalClass::SpecJson(Category::Syntax),
        ),
        (
            "unpaired surrogate in unknown field",
            r#"{"kind":"workspace","extra":"\uD800"}"#,
            RefusalClass::SpecJson(Category::Syntax),
        ),
        ("empty document", "", RefusalClass::SpecJson(Category::Eof)),
        (
            "null",
            "null",
            RefusalClass::Spec(SpecRefusalKind::NotKindOrObject),
        ),
    ]
}

fn assert_spec_spelling_parity() {
    use brokkr_protocol::hands::{Bind, BindMode};

    let valid_reach = full_reach_json().to_string();
    let rich = HandsSpec {
        network: true,
        binds: vec![Bind {
            path: "/b".to_string(),
            mode: BindMode::Ro,
            mask: vec!["secret".to_string()],
        }],
    };
    let mut cases: Vec<(&str, &str, Result<HandsSpec, RefusalClass>)> = vec![
        ("bare workspace", r#""workspace""#, Ok(HandsSpec::default())),
        (
            "valid object",
            r#"{"kind":"workspace","network":true,"binds":[{"path":"/b","mode":"ro","mask":["secret"]}]}"#,
            Ok(rich),
        ),
    ];
    for (label, doc, expected) in spec_duplicate_cases() {
        cases.push((label, doc, Ok(expected)));
    }
    for (label, doc, expected) in spec_refusal_cases() {
        cases.push((label, doc, Err(expected)));
    }

    for (label, doc, expected) in cases {
        let encoded = hands::encode_base64url(doc.as_bytes());
        let plain_res =
            resolve_serve_documents(Some(valid_reach.clone()), None, Some(doc.to_string()), None)
                .map(|(spec, _)| spec)
                .map_err(|err| refusal_class(&err));
        let encoded_res =
            resolve_serve_documents(Some(valid_reach.clone()), None, None, Some(encoded))
                .map(|(spec, _)| spec)
                .map_err(|err| refusal_class(&err));
        assert_eq!(plain_res, expected, "plain spec mismatch on {label}");
        assert_eq!(
            encoded_res, plain_res,
            "encoded vs plain spec mismatch on {label}"
        );
    }
}

/// #1277 V2: one table-driven test feeding the same document bytes to both
/// spellings of `--reach` and `--spec`, asserting equal admitted values or the
/// exact same refusal class on every case (including duplicate members of
/// every `Sealed` field at top level and nested, unknown/missing/wrong-type
/// members, trailing bytes, leading BOM, unpaired surrogates, empty document,
/// `null`, and bare `"workspace"`). Main's plain `--spec` parses via
/// `serde_json::Value` (last-wins on duplicate keys); encoded matches it.
#[test]
fn plain_and_encoded_spellings_agree_on_every_document() {
    assert_reach_spelling_parity();
    assert_spec_spelling_parity();
}
