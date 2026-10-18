//! Every value a Codex TOML assignment carries decodes to exactly itself
//! (rebuild unit 13-fix-c, R2), moved here from `native_controls/tests.rs`
//! when the box's hands gained the artifact root the engine carries
//! (decision 0065 slice two, U8a).

use super::super::mcp::toml_basic;
use super::*;

/// A TOML basic string's body decoded as TOML v1.0.0 reads it, written from
/// the specification's escape table and not from the encoder under test:
/// `\b \t \n \f \r \" \\`, `\uXXXX` and `\UXXXXXXXX`. Any other escape, an
/// unescaped quote and a control character other than tab are invalid.
fn toml_decoded(body: &str) -> Option<String> {
    let mut decoded = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                let escape = chars.next()?;
                let digits = match escape {
                    'u' => 4,
                    'U' => 8,
                    _ => 0,
                };
                decoded.push(match escape {
                    'b' => '\u{8}',
                    't' => '\t',
                    'n' => '\n',
                    'f' => '\u{c}',
                    'r' => '\r',
                    '"' => '"',
                    '\\' => '\\',
                    'u' | 'U' => {
                        let hex: String = chars.by_ref().take(digits).collect();
                        if hex.len() != digits {
                            return None;
                        }
                        char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?
                    }
                    _ => return None,
                });
            }
            '"' => return None,
            c if (c <= '\u{1f}' && c != '\t') || c == '\u{7f}' => return None,
            c => decoded.push(c),
        }
    }
    Some(decoded)
}

/// A TOML array of basic strings, `["a","b"]` with no whitespace, decoded
/// by [`toml_decoded`].
fn toml_strings(array: &str) -> Option<Vec<String>> {
    let mut rest = array.strip_prefix('[')?.strip_suffix(']')?;
    let mut strings = Vec::new();
    while !rest.is_empty() {
        let body = rest.strip_prefix('"')?;
        let mut end = None;
        let mut escaped = false;
        for (at, c) in body.char_indices() {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => {
                    end = Some(at);
                    break;
                }
                _ => {}
            }
        }
        let end = end?;
        strings.push(toml_decoded(&body[..end])?);
        rest = &body[end + 1..];
        rest = match rest.strip_prefix(',') {
            Some(tail) if !tail.is_empty() => tail,
            Some(_) => return None,
            None if rest.is_empty() => rest,
            None => return None,
        };
    }
    Some(strings)
}

/// R2 (rebuild unit 13-fix-c; security S2): every value interpolated into a
/// Codex TOML assignment passes through one TOML basic-string encoder, and a
/// TOML reader decodes exactly the value. The chief's case — a literal
/// backslash-u002f in the executable's path — decodes as a slash when
/// interpolated raw, and as itself once encoded; so do quotes, backslashes,
/// control characters, C1 characters and non-ASCII text, in the executable
/// and in the workdir the server's arguments carry. The expected value and
/// the emitted one are the same encoder's.
#[test]
fn every_codex_transport_value_decodes_as_toml_to_exactly_itself() {
    // The encoder's exact text for the escapes the chief found.
    assert_eq!(
        toml_basic("/opt/a\\u002fb/brokkr"),
        "/opt/a\\\\u002fb/brokkr"
    );
    assert_eq!(
        toml_basic("q\"\n\t\r\u{8}\u{c}\u{0}\u{1f}\u{7f}\u{85}é\\"),
        "q\\\"\\n\\t\\r\\b\\f\\u0000\\u001F\\u007F\u{85}é\\\\"
    );
    // The raw interpolation decodes to another path.
    assert_eq!(
        toml_decoded("/opt/a\\u002fb/brokkr").as_deref(),
        Some("/opt/a/b/brokkr")
    );
    let values = [
        "/opt/a\\u002fb/brokkr",
        "/opt/a\\U0000002Fb",
        "/opt/q\"uote",
        "/opt/back\\slash",
        "/opt/new\nline",
        "/opt/tab\there",
        "/opt/nul\u{0}",
        "/opt/del\u{7f}",
        "/opt/unit\u{1f}",
        "/opt/c1\u{85}",
        "/opt/é😀",
        "/opt/\\\\u002f\\",
    ];
    for value in values {
        assert_eq!(
            toml_decoded(&toml_basic(value)).as_deref(),
            Some(value),
            "{value:?}"
        );
        let path = std::path::Path::new(value);
        let bound = Transport {
            brokkr: path,
            workdir: path,
            artifacts: path,
            sealed: &REACH,
            spec: &SPEC,
        };
        let expanded = bound
            .expand(&argv(&CODEX_HANDS))
            .expect("a UTF-8 transport");
        let command = expanded[3]
            .strip_prefix("mcp_servers.brokkr.command=\"")
            .and_then(|rest| rest.strip_suffix('"'))
            .expect("a quoted command");
        assert_eq!(toml_decoded(command).as_deref(), Some(value), "{value:?}");
        let arguments = expanded[5]
            .strip_prefix("mcp_servers.brokkr.args=")
            .expect("an arguments assignment");
        assert_eq!(
            toml_strings(arguments),
            Some(crate::hands::serve_args(path, path, &REACH, &SPEC)),
            "{value:?}"
        );
    }
    // The check binds the hands through the same encoder, so the encoded
    // command checks and the raw one lacks its binding (rebuild unit
    // 21-fix-a, R1).
    let escaped = Sealed {
        transport: Some(escaped_transport()),
        ..codex_hands(&CODEX_HANDS)
    };
    let cold = escaped.served(&CODEX_LEAD);
    assert_eq!(cold[12], ESCAPED_COMMAND);
    assert_eq!(escaped.cold(cold.clone()).unwrap().into_argv(), cold);
    let mut raw = cold.clone();
    raw[12] = "mcp_servers.brokkr.command=\"/opt/a\\u002fb/brokkr\"".into();
    assert_eq!(escaped.cold(raw), Err(servers::unsealed("codex")));
}

/// A transport whose executable, workdir or artifact root is not UTF-8 has
/// no exact provider value: it expands nothing and binds no hands at the
/// final check (rebuild unit 13-fix-c, R2; U8a).
#[test]
fn a_transport_path_that_is_not_utf8_binds_no_hands() {
    use std::os::unix::ffi::OsStrExt;
    let unrepresentable = std::path::Path::new(std::ffi::OsStr::from_bytes(b"/opt/\xff/brokkr"));
    let boxed = codex_hands(&CODEX_HANDS);
    let cold = boxed.served(&CODEX_LEAD);
    // U8a2c: a sealed reach naming such a path has no exact value either.
    let (work, none) = (
        std::path::Path::new("/w"),
        crate::hands::GitFacts::default(),
    );
    let lossy = crate::hands::seal(&SPEC, unrepresentable, work, &none);
    for bound in [
        Transport {
            brokkr: unrepresentable,
            ..transport()
        },
        Transport {
            workdir: unrepresentable,
            ..transport()
        },
        Transport {
            artifacts: unrepresentable,
            ..transport()
        },
        Transport {
            sealed: &lossy,
            ..transport()
        },
    ] {
        assert_eq!(bound.expand(&argv(&CODEX_HANDS)), None);
        assert_eq!(
            check_final(
                "codex",
                cold.clone(),
                &boxed.controls,
                &boxed.expected,
                boxed.dialect(),
                Serving {
                    program: "codex",
                    workdir: "/w",
                    pins: &boxed.pins,
                    hands: Some(bound),
                    ..Serving::default()
                },
            ),
            Err(final_refusal(
                "codex",
                "binds its hands to an executable or a workdir that is not UTF-8, which no \
                 provider value represents exactly (rebuild unit 13-fix-c, R2)"
            ))
        );
    }
}
