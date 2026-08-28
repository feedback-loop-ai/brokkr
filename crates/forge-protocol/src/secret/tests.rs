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
fn denylist_covers_exact_names_and_forge_prefix() {
    for name in [
        "PATH",
        "IFS",
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "FORGE_X",
        "FORGE_",
    ] {
        assert!(denylisted(name), "{name} must be denylisted");
        assert!(validate_name(name).is_err(), "{name} must not validate");
    }
    for name in ["PATHS", "GH_TOKEN", "LD", "FORG_X"] {
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
    for name in ["PATH", "FORGE_X", "lower", "9BAD"] {
        assert!(
            store_set(&path, name, "longenough").is_err(),
            "{name} must refuse"
        );
    }
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
