use super::*;

fn bound(name: &str, value: &[u8]) -> BoundSecret {
    BoundSecret {
        name: name.to_string(),
        secret: Secret::new(value.to_vec()),
    }
}

// -------------------------------------------------- layer 4: type

#[test]
fn debug_prints_redacted_never_the_value() {
    let secret = Secret::new(b"hunter22".to_vec());
    assert_eq!(format!("{secret:?}"), "Secret(REDACTED)");
}

#[test]
fn wipe_zeroizes_the_buffer() {
    let mut buf = b"hunter22".to_vec();
    wipe(&mut buf);
    assert!(buf.iter().all(|b| *b == 0));
}

// ------------------------------------------- layer 1: name grammar

#[test]
fn name_grammar_vectors() {
    for good in ["A", "GH_TOKEN", "X9", "A_B_C", "TOKEN2"] {
        assert!(valid_name(good), "{good} must be valid");
    }
    for bad in ["", "a", "gh_token", "9X", "_A", "A-B", "A B", "TOKEn", "Ä"] {
        assert!(!valid_name(bad), "{bad:?} must be invalid");
    }
}

#[test]
fn denylist_covers_exact_names_and_the_harness_prefix() {
    // `BROKKR_` configures this release's binary: a code-loading
    // primitive in a seat's hands, so it is pinned here.
    for name in [
        "PATH",
        "IFS",
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "BROKKR_X",
        "BROKKR_",
        "BROKKR_CODEX_BIN",
    ] {
        assert!(denylisted(name), "{name} must be denylisted");
        assert!(validate_name(name).is_err(), "{name} must not validate");
    }
    for name in ["PATHS", "GH_TOKEN", "LD", "FORG_X", "BROKKR", "BROKK_X"] {
        assert!(!denylisted(name), "{name} must not be denylisted");
    }
}

// ------------------------------------------------ layer 1: scanner

#[test]
fn scanner_finds_well_formed_references() {
    assert_eq!(
        scan_secret_refs("secret:NAME").unwrap(),
        Vec::<String>::new()
    );
    assert_eq!(
        scan_secret_refs("curl -H 'auth: {{secret:GH_TOKEN}}' {{secret:API_KEY}}").unwrap(),
        vec!["GH_TOKEN", "API_KEY"]
    );
    assert_eq!(
        scan_secret_refs("no references here").unwrap(),
        Vec::<String>::new()
    );
    // "secret:" without opening braces is not a reference.
    assert_eq!(
        scan_secret_refs("https://x/secret:thing").unwrap(),
        Vec::<String>::new()
    );
}

#[test]
fn scanner_rejects_malformed_occurrences() {
    for (text, why) in [
        ("{{secret:gh_token}}", "lowercase"),
        ("{{secret:}}", "empty"),
        ("{{ secret:NAME }}", "interior whitespace"),
        ("{{\tsecret:NAME}}", "interior tab"),
        ("{{secret: NAME}}", "space after colon"),
        ("{{secret:NAME", "unclosed"),
        ("{{secret:NAMe}}", "partial lowercase tail"),
        ("{{secret:9NAME}}", "leading digit"),
    ] {
        assert!(
            scan_secret_refs(text).is_err(),
            "{why}: {text:?} must be malformed"
        );
    }
}

#[test]
fn scanner_error_never_silently_passes_typos_into_argv() {
    let error = scan_secret_refs("{{secret:GH_TOKEN").unwrap_err();
    assert!(
        error.contains("GH_TOKEN"),
        "error names the reference: {error}"
    );
}

// ---------------------------------------------- layer 5: encoders

#[test]
fn base64_matches_rfc4648_vectors() {
    // RFC 4648 test vectors.
    for (input, padded) in [
        (&b""[..], ""),
        (b"f", "Zg=="),
        (b"fo", "Zm8="),
        (b"foo", "Zm9v"),
        (b"foob", "Zm9vYg=="),
        (b"fooba", "Zm9vYmE="),
        (b"foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(enc_b64_std(input), padded.as_bytes());
        assert_eq!(
            enc_b64_std_nopad(input),
            padded.trim_end_matches('=').as_bytes()
        );
    }
    // URL-safe alphabet swaps +/ for -_.
    assert_eq!(enc_b64_std(&[0xfb, 0xff]), b"+/8=");
    assert_eq!(enc_b64_url(&[0xfb, 0xff]), b"-_8=");
    assert_eq!(enc_b64_url_nopad(&[0xfb, 0xff]), b"-_8");
}

#[test]
fn hex_and_percent_match_fixed_vectors() {
    assert_eq!(enc_hex_lower(b"\x00\xabZ"), b"00ab5a");
    assert_eq!(enc_hex_upper(b"\x00\xabZ"), b"00AB5A");
    assert_eq!(enc_pct_upper(b"a+b !"), b"a%2Bb%20%21");
    assert_eq!(enc_pct_lower(b"a+b !"), b"a%2bb%20%21");
    assert_eq!(
        enc_pct_upper(b"AZaz09-._~"),
        b"AZaz09-._~",
        "unreserved pass through"
    );
}

// ------------------------------------------------ layer 5: masker

#[test]
fn masker_replaces_every_listed_encoding() {
    let value = b"tok3n+v4lue!";
    let bindings = vec![bound("API_TOKEN", value)];
    for (label, encode) in NEEDLE_ENCODINGS {
        let leak = encode(value);
        let mut text = b"before ".to_vec();
        text.extend_from_slice(&leak);
        text.extend_from_slice(b" after");
        let masked = mask_bytes(&text, &bindings);
        assert_eq!(
            masked, b"before [secret:API_TOKEN] after",
            "{label} must mask"
        );
    }
}

#[test]
fn masker_handles_multiple_secrets_longest_needle_first() {
    // ALPHA's value is a strict prefix of BETA's: the longer needle
    // must win where both match.
    let bindings = vec![bound("ALPHA", b"abcdef"), bound("BETA", b"abcdefgh")];
    let masked = mask_bytes(b"x abcdefgh y abcdef z", &bindings);
    assert_eq!(masked, b"x [secret:BETA] y [secret:ALPHA] z");
}

#[test]
fn masker_passes_needle_free_text_through_byte_identical() {
    let bindings = vec![bound("API_TOKEN", b"tok3n+v4lue!")];
    let text = b"ordinary build output, no secrets \xff\xfe here".to_vec();
    assert_eq!(mask_bytes(&text, &bindings), text);
    assert_eq!(mask_bytes(b"anything", &[]), b"anything");
    assert_eq!(mask_bytes(b"anything", &[bound("EMPTY", b"")]), b"anything");
}

#[test]
fn masker_operates_on_bytes_so_invalid_utf8_neighbors_still_mask() {
    let bindings = vec![bound("API_TOKEN", b"tok3n+v4lue!")];
    let mut text = vec![0xff, 0xfe];
    text.extend_from_slice(b"tok3n+v4lue!");
    text.push(0xff);
    let masked = mask_bytes(&text, &bindings);
    let mut expected = vec![0xff, 0xfe];
    expected.extend_from_slice(b"[secret:API_TOKEN]");
    expected.push(0xff);
    assert_eq!(masked, expected);
}

#[test]
fn masker_masks_adjacent_and_repeated_needles() {
    let bindings = vec![bound("K", b"vvvv")];
    assert_eq!(mask_bytes(b"vvvvvvvv", &bindings), b"[secret:K][secret:K]");
}

// ------------------------------------------------- layer 2: store

#[test]
fn store_round_trip_set_list_remove() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secrets.env");
    assert_eq!(store_names(&path).unwrap(), Vec::<String>::new());
    store_set(&path, "GH_TOKEN", "tokenvalue1").unwrap();
    store_set(&path, "API_KEY", "keyvalue22").unwrap();
    assert_eq!(store_names(&path).unwrap(), vec!["API_KEY", "GH_TOKEN"]);
    store_set(&path, "GH_TOKEN", "rotated-value").unwrap();
    assert_eq!(store_names(&path).unwrap(), vec!["API_KEY", "GH_TOKEN"]);
    let bindings = resolve_bindings(&path, &["GH_TOKEN".to_string()]).unwrap();
    assert_eq!(bindings[0].secret().expose_for_spawn(), b"rotated-value");
    assert!(store_remove(&path, "GH_TOKEN").unwrap());
    assert!(
        !store_remove(&path, "GH_TOKEN").unwrap(),
        "second remove finds nothing"
    );
    assert_eq!(store_names(&path).unwrap(), vec!["API_KEY"]);
}

#[test]
fn resolve_missing_name_names_the_secret_and_the_path_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secrets.env");
    store_set(&path, "PRESENT", "somevalue").unwrap();
    let error = resolve_bindings(&path, &["ABSENT".to_string()]).unwrap_err();
    assert!(error.contains("ABSENT"), "{error}");
    assert!(error.contains("secrets.env"), "{error}");
    assert!(!error.contains("somevalue"), "never the contents: {error}");
}

#[test]
fn corrupt_and_unavailable_store_paths_fail_without_exposing_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.env");
    assert!(read_store(&missing)
        .unwrap_err()
        .contains("cannot read secrets store"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for (name, bytes, expected) in [
            ("non-utf8.env", vec![0xff, b'=', b'x'], "non-UTF-8 name"),
            ("bad-name.env", b"lower=value".to_vec(), "ill-formed name"),
        ] {
            let path = dir.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
            assert!(read_store(&path).unwrap_err().contains(expected));
        }
    }

    let duplicate = dir.path().join("duplicate.env");
    std::fs::write(&duplicate, b"TOKEN=first-value\nTOKEN=second-value\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&duplicate, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let bindings = resolve_bindings(&duplicate, &["TOKEN".into()]).unwrap();
    assert_eq!(bindings[0].secret().expose_for_spawn(), b"second-value");
}

#[test]
fn store_write_remove_and_empty_resolve_cover_refusal_boundaries() {
    assert_eq!(store_parent(Path::new("secrets.env")), Path::new("."));
    assert_eq!(store_parent(Path::new("/")), Path::new("."));
    assert_eq!(
        store_parent(Path::new("/tmp/secrets.env")),
        Path::new("/tmp")
    );
    assert_eq!(store_io::<()>(Ok(()), "ok".into()), Ok(()));
    assert!(
        store_io::<()>(Err(std::io::Error::other("no")), "context".into())
            .unwrap_err()
            .contains("context: no")
    );
    let dir = tempfile::tempdir().unwrap();
    let injectable = dir.path().join("injectable.env");
    let entries: [(String, Secret); 1] = [("TOKEN".into(), Secret::new(b"long-enough".to_vec()))];
    #[cfg(unix)]
    assert!(write_store_with(
        &injectable,
        &entries,
        |_, _| Err(std::io::Error::other("mode")),
        write_store_entry,
    )
    .unwrap_err()
    .contains("cannot set secrets store mode"));
    #[cfg(unix)]
    assert!(
        write_store_with(&injectable, &entries, set_store_mode, |_, _, _| Err(
            std::io::Error::other("write")
        ),)
        .unwrap_err()
        .contains("cannot write secrets store")
    );
    let parent_file = dir.path().join("parent-file");
    std::fs::write(&parent_file, "not a directory").unwrap();
    assert!(store_set(&parent_file.join("secrets.env"), "TOKEN", "long-enough").is_err());

    let destination_directory = dir.path().join("destination-directory");
    std::fs::create_dir(&destination_directory).unwrap();
    assert!(write_store(
        &destination_directory,
        &[("TOKEN".into(), Secret::new(b"long-enough".to_vec()))]
    )
    .is_err());
    assert!(write_store(Path::new(""), &[]).is_err());
    #[cfg(unix)]
    assert!(write_store(
        Path::new("/proc/forge-secrets.env"),
        &[("TOKEN".into(), Secret::new(b"long-enough".to_vec()))]
    )
    .is_err());

    let directory = dir.path().join("read-as-directory");
    std::fs::create_dir(&directory).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    assert!(read_store(&directory).is_err());

    let missing = dir.path().join("absent.env");
    assert!(!store_remove(&missing, "TOKEN").unwrap());
    assert!(resolve_bindings(&missing, &[]).unwrap().is_empty());
}

#[test]
fn store_set_refuses_every_rejected_value_class() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secrets.env");
    for (value, why) in [
        ("", "empty"),
        ("two\nlines", "multi-line"),
        ("two\rlines", "carriage return"),
        ("nul\0byte", "NUL"),
        ("abc", "under 4 bytes"),
    ] {
        assert!(
            store_set(&path, "NAME", value).is_err(),
            "{why} must refuse"
        );
    }
    assert!(
        store_set(&path, "NAME", "short7c").unwrap().is_some(),
        "under 8 bytes warns"
    );
    assert!(store_set(&path, "NAME", "longenough").unwrap().is_none());
    for name in ["PATH", "BROKKR_X", "lower", "9BAD"] {
        assert!(
            store_set(&path, name, "longenough").is_err(),
            "{name} must refuse"
        );
    }
}

/// `SHORT_VALUE_WARN_BYTES` is the shortest value accepted without the
/// warning, not the longest warned about (#419).
#[test]
fn a_value_exactly_at_the_warning_bound_is_accepted_quietly() {
    assert_eq!(
        validate_value("1234567"),
        Ok(Some(
            "warning: secret value is shorter than 8 bytes; short values mask aggressively"
                .to_string()
        ))
    );
    assert_eq!(validate_value("12345678"), Ok(None));
}

#[cfg(unix)]
#[test]
fn store_created_0600_and_replace_never_widens() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secrets.env");
    store_set(&path, "GH_TOKEN", "tokenvalue1").unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "created 0600");
    // Tighten to 0400: a rewrite must not widen it back.
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400)).unwrap();
    store_set(&path, "API_KEY", "keyvalue22").unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o400, "atomic replace preserves the stricter mode");
}

#[cfg(unix)]
#[test]
fn broader_than_0600_store_refuses_on_read() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secrets.env");
    store_set(&path, "GH_TOKEN", "tokenvalue1").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    let error = store_names(&path).unwrap_err();
    assert!(error.contains("broader"), "{error}");
    assert!(error.contains("secrets.env"), "names the path: {error}");
    assert!(
        !error.contains("tokenvalue1"),
        "never the contents: {error}"
    );
}

#[test]
fn store_parse_skips_comments_and_refuses_garbage() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secrets.env");
    std::fs::write(&path, "# comment\n\nGH_TOKEN=abcd1234\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    assert_eq!(store_names(&path).unwrap(), vec!["GH_TOKEN"]);
    std::fs::write(&path, "no equals sign\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let error = store_names(&path).unwrap_err();
    assert!(error.contains("line 1"), "{error}");
    assert!(!error.contains("equals"), "never the contents: {error}");
}

#[test]
fn mask_json_masks_every_decoded_string_and_leaves_keys_and_scalars() {
    // A value with a quote and a backslash is JSON-escaped on the wire,
    // so only masking after parsing can find it: this is the stream-json
    // surface decision 0012's layer 5 covers for the model harnesses.
    let value = "p\"a\\ss-w0rd";
    let store_dir = tempfile::tempdir().unwrap();
    let store = store_dir.path().join("secrets.env");
    store_set(&store, "API_TOKEN", value).unwrap();
    let bindings = resolve_bindings(&store, &["API_TOKEN".to_string()]).unwrap();
    let wire = serde_json::to_string(&serde_json::json!({
        "target": format!("/work/{value}"),
        "nested": [{"note": format!("said {value}")}, 7, true, null],
        "tokens": 3,
    }))
    .unwrap();
    assert!(!wire.contains(value), "the wire form is escaped: {wire}");
    let mut parsed: serde_json::Value = serde_json::from_str(&wire).unwrap();
    mask_json(&mut parsed, &bindings);
    assert_eq!(
        parsed,
        serde_json::json!({
            "target": "/work/[secret:API_TOKEN]",
            "nested": [{"note": "said [secret:API_TOKEN]"}, 7, true, null],
            "tokens": 3,
        })
    );
}

#[test]
fn mask_projected_masks_a_value_a_projector_reserialised() {
    // A transcript projector renders a tool call's arguments object as
    // serialised JSON, so a value with a quote, a backslash and a tab
    // reaches the projected text escaped: `mask_bytes` misses it there
    // and `mask_projected` does not, while still masking the raw value.
    let value = "p\"a\\ss\tw0rd";
    let store_dir = tempfile::tempdir().unwrap();
    let store = store_dir.path().join("secrets.env");
    store_set(&store, "API_TOKEN", value).unwrap();
    let bindings = resolve_bindings(&store, &["API_TOKEN".to_string()]).unwrap();
    let arguments = serde_json::json!({"header": value}).to_string();
    assert_eq!(arguments, "{\"header\":\"p\\\"a\\\\ss\\tw0rd\"}");
    let projected = format!("Bash {arguments} then {value}");
    assert_eq!(
        String::from_utf8(mask_bytes(projected.as_bytes(), &bindings)).unwrap(),
        format!("Bash {arguments} then [secret:API_TOKEN]")
    );
    assert_eq!(
        mask_projected(&projected, &bindings),
        "Bash {\"header\":\"[secret:API_TOKEN]\"} then [secret:API_TOKEN]"
    );
}

#[test]
fn mask_json_masks_object_keys_and_leaves_numbers_as_numbers() {
    // A result file's keys are the seat's to choose, so a key that echoes
    // a bound value is masked like any string. A count stays a count.
    let store_dir = tempfile::tempdir().unwrap();
    let store = store_dir.path().join("secrets.env");
    store_set(&store, "API_TOKEN", "key-leak-value").unwrap();
    let bindings = resolve_bindings(&store, &["API_TOKEN".to_string()]).unwrap();
    let mut value = serde_json::json!({
        "key-leak-value": {"inner key-leak-value": 1},
        "tokens": 42,
    });
    mask_json(&mut value, &bindings);
    assert_eq!(
        value,
        serde_json::json!({
            "[secret:API_TOKEN]": {"inner [secret:API_TOKEN]": 1},
            "tokens": 42,
        })
    );
}

// ------------------------------------------- layer 2: store reader

/// A store file under a canonical temporary root, at `mode`.
fn store_at(dir: &Path, name: &str, bytes: &[u8], mode: u32) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.canonicalize().unwrap().join(name);
    std::fs::write(&path, bytes).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
    path
}

fn read_handle(path: &Path) -> Result<Vec<(String, Secret)>, store::StoreError> {
    store::read_store_file(std::fs::File::open(path).unwrap(), path)
}

/// The operator's text, pinned once: every cause renders at the legacy
/// `String` edge exactly as the pre-extraction reader wrote it.
#[test]
fn store_causes_render_the_legacy_text_at_the_string_edge() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let absent = root.join("absent.env");
    assert_eq!(store_names(&absent), Ok(Vec::<String>::new()));
    assert_eq!(
        resolve_bindings(&absent, &["TOKEN".into()]).unwrap_err(),
        format!(
            "cannot read secrets store {}: No such file or directory (os error 2)",
            absent.display()
        )
    );
    let broad = store_at(&root, "broad.env", b"TOKEN=abcd1234\n", 0o644);
    assert_eq!(
        store_names(&broad),
        Err(format!(
            "refusing secrets store {}: permissions 644 are broader than 0600",
            broad.display()
        ))
    );
    for (name, bytes, cause) in [
        (
            "garbage.env",
            &b"# c\nTOKEN=abcd\nno equals\n"[..],
            "line 3 is not NAME=value",
        ),
        (
            "non-utf8.env",
            b"\xff=abcd\n",
            "line 1 has a non-UTF-8 name",
        ),
        (
            "ill-formed.env",
            b"\nlower=abcd\n",
            "line 2 has an ill-formed name",
        ),
    ] {
        let path = store_at(&root, name, bytes, 0o600);
        assert_eq!(
            store_remove(&path, "TOKEN"),
            Err(format!("secrets store {} {cause}", path.display()))
        );
    }
    let present = store_at(&root, "present.env", b"PRESENT=somevalue\n", 0o600);
    assert_eq!(
        resolve_bindings(&present, &["PRESENT".into(), "ABSENT".into()]).unwrap_err(),
        format!(
            "secret 'ABSENT' is not in the store at {} (brokkr secrets set ABSENT)",
            present.display()
        )
    );
}

/// The descriptor reader's typed causes, each with its path and line.
#[test]
fn the_descriptor_reader_returns_typed_causes() {
    use store::StoreError;
    let dir = tempfile::tempdir().unwrap();
    let broad = store_at(dir.path(), "broad.env", b"TOKEN=abcd1234\n", 0o640);
    assert!(matches!(
        read_handle(&broad),
        Err(StoreError::BroadMode { path, mode: 0o640 }) if path == broad
    ));
    type Cause = fn(&StoreError) -> Option<(&Path, usize)>;
    let cases: [(&str, &[u8], Cause); 3] = [
        ("garbage.env", b"A=b\r\nno equals\n", |e| match e {
            StoreError::NotAssignment { path, line } => Some((path, *line)),
            _ => None,
        }),
        ("non-utf8.env", b"A=b\n\xfe=c\n", |e| match e {
            StoreError::NonUtf8Name { path, line } => Some((path, *line)),
            _ => None,
        }),
        ("ill-formed.env", b"A=b\n9A=c\n", |e| match e {
            StoreError::IllFormedName { path, line } => Some((path, *line)),
            _ => None,
        }),
    ];
    for (name, bytes, cause) in cases {
        let path = store_at(dir.path(), name, bytes, 0o600);
        let error = read_handle(&path).unwrap_err();
        assert_eq!(
            cause(&error),
            Some((path.as_path(), 2)),
            "{name}: {error:?}"
        );
    }
    let directory = dir.path().canonicalize().unwrap().join("directory");
    std::fs::create_dir(&directory).unwrap();
    std::fs::set_permissions(
        &directory,
        std::os::unix::fs::PermissionsExt::from_mode(0o700),
    )
    .unwrap();
    let opened = std::fs::File::open(&directory).unwrap();
    assert!(matches!(
        store::read_store_file(opened, &directory),
        Err(StoreError::Io { path, source })
            if path == directory && source.kind() == std::io::ErrorKind::IsADirectory
    ));
}

/// Comments, blank lines and CRLF are skipped, a line splits at its first
/// `=`, a later assignment overrides, and a value keeps its raw bytes,
/// which the injector, not the reader, refuses as non-UTF-8.
#[test]
fn the_descriptor_reader_parses_the_env_format_once_into_raw_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = store_at(
        dir.path(),
        "secrets.env",
        b"# comment\r\n\r\nURL=a=b\r\nRAW=\xff\xfe\nURL=later=c\n",
        0o600,
    );
    let entries = read_handle(&path).unwrap();
    let values: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(name, secret)| (name.as_str(), secret.expose_for_spawn()))
        .collect();
    assert_eq!(values, [("URL", &b"later=c"[..]), ("RAW", b"\xff\xfe")]);
    let bindings = store::bind_names(entries, &["RAW".into()], &path).unwrap();
    assert_eq!(
        bind_environment(&mut std::process::Command::new("true"), &bindings),
        Err(BindError::NotUtf8("RAW".into()))
    );
}

/// The mode check and the read both go through the supplied handle: a
/// path replaced after the open reaches neither.
#[test]
fn the_descriptor_reader_checks_and_reads_the_handle_not_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = store_at(dir.path(), "secrets.env", b"ADMITTED=first-value\n", 0o600);
    let admitted = std::fs::File::open(&path).unwrap();
    let swapped = store_at(dir.path(), "swapped.env", b"SWAPPED=other-value\n", 0o644);
    std::fs::rename(&swapped, &path).unwrap();
    let entries = store::read_store_file(admitted, &path).unwrap();
    let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["ADMITTED"]);
    assert_eq!(entries[0].1.expose_for_spawn(), b"first-value");
}

/// A missing name is classified from the one parse: with the store gone
/// after its read, the cause is still the missing name, not an I/O
/// failure from a second look.
#[test]
fn a_missing_name_is_classified_without_rereading_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let path = store_at(dir.path(), "secrets.env", b"PRESENT=somevalue\n", 0o600);
    let entries = read_handle(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let names = ["PRESENT".to_string(), "ABSENT".to_string()];
    assert!(matches!(
        store::bind_names(entries, &names, &path),
        Err(store::StoreError::MissingName { name, path: at }) if name == "ABSENT" && at == path
    ));
}

/// A declared name the store lacks refuses even when the process's own
/// environment carries it: there is no ambient fallback.
#[test]
fn a_missing_binding_never_falls_back_to_the_ambient_environment() {
    let ambient = std::env::vars_os()
        .filter_map(|(name, _)| name.into_string().ok())
        .find(|name| valid_name(name) && !denylisted(name))
        .expect("the test process carries an upper-case environment name");
    let dir = tempfile::tempdir().unwrap();
    let path = store_at(dir.path(), "secrets.env", b"PRESENT=somevalue\n", 0o600);
    assert!(matches!(
        store::bind_names(read_handle(&path).unwrap(), std::slice::from_ref(&ambient), &path),
        Err(store::StoreError::MissingName { name, .. }) if name == ambient
    ));
}
