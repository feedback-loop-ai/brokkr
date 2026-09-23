use super::*;
use serde_json::json;

/// The shipped research lane's route, copied from
/// `recipes/research-dsh/drivers/research-web.yml` without its comments.
const SHIPPED: &str = "\
- id: llm-pi-ai
  config:
    providers:
      dashscope:
        displayName: Model Studio (Token Plan)
        apiKeyEnv: DASHSCOPE_API_KEY
        api: openai-completions
        baseURL: https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1
        compat:
          thinkingFormat: deepseek
        models:
          - id: qwen3.8-max
            reasoningEfforts:
              low: low
              medium: medium
              xhigh: xhigh
";

const PIN: &str = "dashscope/qwen3.8-max";

fn refused(body: &str, needle: &str) {
    let error = validate(body.as_bytes(), PIN).unwrap_err();
    assert!(error.contains(needle), "expected {needle:?} in {error:?}");
}

#[test]
fn the_shipped_route_and_its_comments_pass_the_closed_grammar() {
    // Comments and blank lines are the shipped file's own shape.
    let with_comments = format!("# Decision 0044 ruling 5.\n\n{}# trailing note\n", SHIPPED);
    assert!(validate(with_comments.as_bytes(), PIN).is_ok());
}

#[test]
fn the_route_row_and_provider_are_closed() {
    refused(
        "- id: session-persistence-jsonl\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        models:\n          - id: qwen3.8-max\n",
        "route entry id",
    );
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      openrouter:\n        apiKeyEnv: X\n        models:\n          - id: qwen3.8-max\n",
        "provider the seat did not pin",
    );
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        models:\n          - id: other\n",
        "model the seat did not pin",
    );
    // Two providers.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        models:\n          - id: qwen3.8-max\n      other:\n        apiKeyEnv: Y\n        models:\n          - id: qwen3.8-max\n",
        "exactly one provider",
    );
}

#[test]
fn a_credential_value_or_a_field_outside_the_set_is_refused() {
    // An inline apiKey is outside the closed set.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKey: sk-live\n        apiKeyEnv: DASHSCOPE_API_KEY\n        models:\n          - id: qwen3.8-max\n",
        "outside the closed set",
    );
    // A literal authorization header, even beside a valid apiKeyEnv.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: DASHSCOPE_API_KEY\n        headers:\n          Authorization: Bearer sk-live\n        models:\n          - id: qwen3.8-max\n",
        "outside the closed set",
    );
    // A missing or malformed apiKeyEnv.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        api: openai-completions\n        models:\n          - id: qwen3.8-max\n",
        "missing `apiKeyEnv`",
    );
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: 9LIVE\n        models:\n          - id: qwen3.8-max\n",
        "environment-variable name",
    );
    // modelOverrides is outside the set too.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: DASHSCOPE_API_KEY\n        modelOverrides:\n          x: y\n        models:\n          - id: qwen3.8-max\n",
        "outside the closed set",
    );
}

#[test]
fn the_endpoint_grammar_decides_the_positive_and_every_refusal() {
    assert_eq!(validate(SHIPPED.as_bytes(), PIN), Ok(()));
    for base in [
        "https://user:pass@host/x",
        "https://host/x?api_key=1",
        "https://host/x#frag",
        "https://host/percent%2fescape",
        "https://host\\x",
        "https://host/ space",
        "https://[::1]/x",
        "https://host//x",
        "http://host/x",
        "HTTPS://host/x",
        "host/x",
        "https://-host/x",
        "https://host:123456/x",
        // Non-ASCII, in the host and in a path segment: the grammar's
        // letters and digits are ASCII ones.
        "https://hóst/x",
        "https://host/pàth",
    ] {
        let body = SHIPPED.replace(
            "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1",
            base,
        );
        assert_eq!(
            validate(body.as_bytes(), PIN),
            Err(
                "refusing to invoke the dsh driver: baseURL leaves the closed endpoint grammar"
                    .to_string()
            ),
            "{base:?}"
        );
    }
}

/// A second top-level entry is refused whole, whether it repeats the
/// admitted route or carries a row the overlay may not name, so no
/// entry past the first is ever read or forwarded.
#[test]
fn a_route_document_carrying_a_second_entry_is_refused() {
    let second_entry =
        "refusing to invoke the dsh driver: route overlay must hold exactly one top-level entry";
    for body in [
        format!("{SHIPPED}{SHIPPED}"),
        format!("{SHIPPED}- id: settings\n  config:\n    theme: dark\n"),
    ] {
        assert_eq!(
            validate(body.as_bytes(), PIN),
            Err(second_entry.to_string()),
            "{body:?}"
        );
    }
}

#[test]
fn executable_or_unrecognized_syntax_is_refused_at_any_depth() {
    for (body, needle) in [
        ("- id: llm-pi-ai\n  config: !!js ctx\n", "reserved character"),
        ("- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        __jsExpr: 1\n        models:\n          - id: qwen3.8-max\n", "plain identifier"),
        ("- id: llm-pi-ai\n  config: {providers: x}\n", "reserved character"),
        ("- id: llm-pi-ai\n  config: &anchor x\n", "reserved character"),
        ("- id: llm-pi-ai\n  config: *alias\n", "reserved character"),
        ("- id: llm-pi-ai\n  config: |\n    x\n", "reserved character"),
        ("- id: 'llm-pi-ai'\n", "reserved character"),
        ("- id: llm-pi-ai\n  config:\n    <<: x\n", "plain identifier"),
    ] {
        let error = validate(body.as_bytes(), PIN).unwrap_err();
        assert!(error.contains(needle), "{body:?}: expected {needle:?} in {error:?}");
    }
}

#[test]
fn a_route_needs_a_model_pin_and_model_item() {
    assert!(validate(SHIPPED.as_bytes(), "deepseek-v4-flash").is_err());
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n",
        "missing `models`",
    );
}

/// A helper: write one file under a temporary working directory and
/// build the private context that names it.
fn binding(body: &[u8], value: &str) -> (tempfile::TempDir, Value) {
    let dir = tempfile::tempdir().unwrap();
    // Every caller names one file directly under the temporary
    // directory; the directory itself is the only parent to create.
    let path = dir.path().join(value);
    std::fs::write(&path, body).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(body);
    let digest = hex::encode(hasher.finalize());
    let input = json!({
        "resume_context": { "route_overlay": { "value": value, "digest": digest } }
    });
    (dir, input)
}

#[test]
fn claim_reads_the_bound_file_and_requires_the_digest_before_the_shape() {
    let (dir, input) = binding(SHIPPED.as_bytes(), "route.yml");
    let workdir = dir.path().to_string_lossy().into_owned();
    let bytes = claim(&input, &workdir, Some(PIN), Some("route.yml"))
        .unwrap()
        .expect("a bound route is admitted");
    assert_eq!(bytes, SHIPPED.as_bytes());

    // The binding disagrees with the argv.
    let error = claim(&input, &workdir, Some(PIN), Some("other.yml")).unwrap_err();
    assert!(error.contains("disagrees"), "{error}");

    // A `--patch` with no binding and a binding with no `--patch`.
    let error = claim(&json!({}), &workdir, Some(PIN), Some("route.yml")).unwrap_err();
    assert!(error.contains("no bound route overlay"), "{error}");
    let error = claim(&input, &workdir, Some(PIN), None).unwrap_err();
    assert!(error.contains("disagrees"), "{error}");

    // A digest mismatch refuses before any shape check, even though the
    // bytes are a valid route.
    let (_, wrong) = binding(b"route: changed\n", "route.yml");
    let error = claim(&wrong, &workdir, Some(PIN), Some("route.yml")).unwrap_err();
    assert!(error.contains("do not hash"), "{error}");

    // A bound, digest-matching member whose baseURL leaves the grammar
    // still refuses: binding decides bytes, the grammar decides content.
    let bad = SHIPPED.replace("https://token-plan", "http://token-plan");
    let (bad_dir, bad_input) = binding(bad.as_bytes(), "route.yml");
    let error = claim(
        &bad_input,
        &bad_dir.path().to_string_lossy(),
        Some(PIN),
        Some("route.yml"),
    )
    .unwrap_err();
    assert!(error.contains("endpoint"), "{error}");
}

#[test]
fn claim_refuses_absolute_traversal_and_escaping_values() {
    let (dir, _) = binding(SHIPPED.as_bytes(), "route.yml");
    let workdir = dir.path().to_string_lossy().into_owned();
    // An absolute path is spelled differently per platform, and the
    // refusal under test is the one `Path::is_absolute` decides, so the
    // proof holds on both rather than being gated to one. Selected by
    // `#[cfg]` and not by `cfg!`, so the arm this platform does not use
    // is never compiled — a runtime branch would leave a line the exact
    // coverage gate can never reach.
    #[cfg(windows)]
    let absolute = "C:\\Windows\\win.ini";
    #[cfg(not(windows))]
    let absolute = "/etc/passwd";
    for (value, needle) in [(absolute, "absolute"), ("../escape.yml", "`..`")] {
        let input = json!({
            "resume_context": { "route_overlay": { "value": value, "digest": "a".repeat(64) } }
        });
        let error = claim(&input, &workdir, Some(PIN), Some(value)).unwrap_err();
        assert!(error.contains(needle), "{value:?}: {error}");
    }
}

#[cfg(unix)]
#[test]
fn claim_refuses_symlink_escape_non_regular_oversized_and_non_utf8() {
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("elsewhere.yml"), SHIPPED).unwrap();
    let digest = |bytes: &[u8]| {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    };
    let body = SHIPPED.as_bytes();
    let bound = |digest: &str| json!({"resume_context": {"route_overlay": {"value": "route.yml", "digest": digest}}});

    // A symlink inside the working directory resolving outside it.
    let (dir, _) = binding(body, "route.yml");
    std::fs::remove_file(dir.path().join("route.yml")).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("elsewhere.yml"),
        dir.path().join("route.yml"),
    )
    .unwrap();
    let error = claim(
        &bound(&digest(body)),
        &dir.path().to_string_lossy(),
        Some(PIN),
        Some("route.yml"),
    )
    .unwrap_err();
    assert!(error.contains("outside the working directory"), "{error}");

    // A directory where the route file must be a regular file.
    let (dir, _) = binding(body, "route.yml");
    std::fs::remove_file(dir.path().join("route.yml")).unwrap();
    std::fs::create_dir(dir.path().join("route.yml")).unwrap();
    let error = claim(
        &bound(&digest(body)),
        &dir.path().to_string_lossy(),
        Some(PIN),
        Some("route.yml"),
    )
    .unwrap_err();
    assert!(error.contains("not a regular file"), "{error}");

    // Bytes over the reader's finite bound.
    let oversized = vec![b'a'; MAX_BYTES + 1];
    let (dir, _) = binding(&oversized, "route.yml");
    let error = claim(
        &bound(&digest(body)),
        &dir.path().to_string_lossy(),
        Some(PIN),
        Some("route.yml"),
    )
    .unwrap_err();
    assert!(error.contains("byte bound"), "{error}");

    // Bytes that are not UTF-8, bound by their own digest.
    let raw = b"- id: llm-pi-ai\n\xff\n";
    let (dir, _) = binding(raw, "route.yml");
    let error = claim(
        &bound(&digest(raw)),
        &dir.path().to_string_lossy(),
        Some(PIN),
        Some("route.yml"),
    )
    .unwrap_err();
    assert!(error.contains("not UTF-8"), "{error}");
}

#[test]
fn a_bound_route_needs_a_pin_a_nonempty_value_and_a_readable_workdir() {
    let (dir, input) = binding(SHIPPED.as_bytes(), "route.yml");
    let workdir = dir.path().to_string_lossy().into_owned();
    // A binding with no pinned model has no provider or id to check.
    let error = claim(&input, &workdir, None, Some("route.yml")).unwrap_err();
    assert!(error.contains("pinned `--model`"), "{error}");
    // An empty value that agrees with the argv refuses before any read.
    let empty = json!({
        "resume_context": { "route_overlay": { "value": "", "digest": "a".repeat(64) } }
    });
    let error = claim(&empty, &workdir, Some(PIN), Some("")).unwrap_err();
    assert!(error.contains("empty"), "{error}");
    // An empty workdir is read as the current directory, not skipped.
    let error = claim(&input, "", Some(PIN), Some("route.yml")).unwrap_err();
    assert!(error.contains("unreadable"), "{error}");
}

/// A pin with no provider segment refuses like an absent one, before
/// the file is stat'd or read. The route is keyed to the provider
/// `parse_dsh_model` defaults the pin to, so no later check can refuse
/// it: this vector is admitted unless the claim boundary refuses it.
#[test]
fn a_bound_route_beside_a_segment_less_pin_refuses_before_any_read() {
    let body = "- id: llm-pi-ai\n  config:\n    providers:\n      deepseek-official:\n        \
                apiKeyEnv: DEEPSEEK_API_KEY\n        models:\n          - id: deepseek-v4-flash\n            \
                reasoningEfforts:\n              high: high\n";
    // The bytes on their own pass the grammar for that pin.
    assert_eq!(validate(body.as_bytes(), "deepseek-v4-flash"), Ok(()));
    let (dir, input) = binding(body.as_bytes(), "route.yml");
    let root = std::fs::canonicalize(dir.path()).unwrap();
    let workdir = root.to_string_lossy().into_owned();
    let error = claim_with(
        &input,
        &workdir,
        Some("deepseek-v4-flash"),
        Some("route.yml"),
        |_| panic!("a segment-less pin never reads the route"),
        |_| panic!("a segment-less pin never stats the route"),
    )
    .unwrap_err();
    assert_eq!(
        error,
        "refusing to invoke the dsh driver: a route overlay needs a pinned `--model` with a \
         provider segment"
    );
    // A pin with the segment reads the same bytes and is admitted.
    let admitted = body.replace("deepseek-official", "deepseek");
    let (dir, input) = binding(admitted.as_bytes(), "route.yml");
    let root = std::fs::canonicalize(dir.path()).unwrap();
    let bytes = claim(
        &input,
        &root.to_string_lossy(),
        Some("deepseek/deepseek-v4-flash"),
        Some("route.yml"),
    )
    .unwrap();
    assert_eq!(bytes, Some(admitted.into_bytes()));
}

#[test]
fn the_reader_refuses_bytes_that_exceed_the_bound_after_a_bound_metadata() {
    // The metadata check sees a file inside the bound; the one read
    // returns more than it. That TOCTOU window is what the post-read
    // check exists for, and no deterministic fixture can force it, so
    // the reader is injected.
    let (dir, input) = binding(SHIPPED.as_bytes(), "route.yml");
    let workdir = dir.path().to_string_lossy().into_owned();
    let error = claim_with(
        &input,
        &workdir,
        Some(PIN),
        Some("route.yml"),
        |_| Ok(vec![b'a'; MAX_BYTES + 1]),
        |path| std::fs::metadata(path),
    )
    .unwrap_err();
    assert!(error.contains("byte bound"), "{error}");
}

/// A canonicalized path whose metadata read then fails is a refusal, not
/// a panic or a skipped file. The injected `stat` makes the failure
/// deterministic; the real path is still canonicalized first.
#[test]
fn the_reader_refuses_a_metadata_failure_after_a_resolved_path() {
    let (dir, input) = binding(SHIPPED.as_bytes(), "route.yml");
    let workdir = dir.path().to_string_lossy().into_owned();
    let error = claim_with(
        &input,
        &workdir,
        Some(PIN),
        Some("route.yml"),
        read_route_file,
        |_| Err(std::io::Error::other("the metadata read failed")),
    )
    .unwrap_err();
    assert!(error.contains("file is unreadable"), "{error}");
}

#[test]
fn the_reader_refuses_each_malformed_block_shape() {
    // A top-level sequence must hold exactly one entry.
    refused("- id: a\n- id: b\n", "exactly one top-level entry");
    // A top-level mapping is not a sequence of one entry.
    refused("id: llm-pi-ai\n", "sequence of one entry");
    // A bare sequence item carries no member.
    refused("-\n", "bare sequence item");
    // A line that is neither a member nor a closed block key.
    refused("justtext\n", "not a recognized block line");
    // A block whose head does not consume every line.
    refused("- id: llm-pi-ai\nfoo: bar\n", "unexpected indentation");
    // A `key:` with no child block.
    refused("key:\n", "empty `key:` block");
    // A child indented more than one depth under a `key:`.
    refused(
        "- id: llm-pi-ai\n  config:\n      providers: x\n",
        "jumps more than one depth",
    );
    // A provider field repeated.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        api: a\n        api: a\n        models:\n          - id: qwen3.8-max\n",
        "repeats a field",
    );
    // `models` must be a sequence of exactly one item.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        models: x\n",
        "models is not a sequence",
    );
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        models:\n          - id: a\n          - id: b\n",
        "exactly one item",
    );
}

#[test]
fn the_semantic_helpers_refuse_non_mappings_and_repeated_or_nested_fields() {
    // `config` must be a mapping.
    refused("- id: llm-pi-ai\n  config: x\n", "config is not a mapping");
    // A repeated route-entry field is refused by `field`, not collapsed.
    refused("- id: llm-pi-ai\n  id: llm-pi-ai\n", "repeats `id`");
    // `id` must stay a scalar.
    refused("- id:\n  x: y\n", "nests a non-scalar `id`");
    // `baseURL` must stay a scalar.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        baseURL:\n          x: y\n        models:\n          - id: qwen3.8-max\n",
        "nests a non-scalar `baseURL`",
    );
    // `compat` must stay a mapping.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        compat: x\n        models:\n          - id: qwen3.8-max\n",
        "nests a non-mapping `compat`",
    );
}

#[test]
fn keys_and_scalars_are_closed_identifiers_and_unquoted_values() {
    // A key whose body carries a character outside the identifier set.
    refused("id$x: y\n", "not a plain identifier");
    // A key may carry `_` and `-` after its first character.
    refused("a_b-c: value\n", "sequence of one entry");
    // An empty value is refused.
    refused("key: \n", "empty value");
    // A value holding a comment or a second mapping is refused.
    refused("key: a: b\n", "comment or an extra mapping");
    refused("key: a # b\n", "comment or an extra mapping");
}

#[test]
fn the_reader_refuses_every_lexical_shape_it_did_not_recognize() {
    // Document markers.
    refused("---\n", "document marker");
    refused("...\n", "document marker");
    // A tab and a control character.
    refused("- id:\tx\n", "tab or control");
    refused("- id: x\u{1}\n", "tab or control");
    // A comment line is checked for them too, and before it is
    // skipped: a lone CR inside one hides a second row from this
    // reader that a CR-tolerant YAML reader would see.
    refused("# route\rapiKey: leaked\n- id: x\n", "tab or control");
    refused("  # route\t\n- id: x\n", "tab or control");
    // Odd indentation.
    refused("- id: x\n   y: z\n", "odd indentation");
    // A whitespace-only line is skipped; a document of them is empty.
    refused("  \n", "route overlay is empty");
    // Comments and blanks alone carry no block line.
    refused("# only a comment\n\n", "route overlay is empty");
    // The byte bound is refused before any parse.
    let huge = vec![b'a'; MAX_BYTES + 1];
    assert!(validate(&huge, PIN).is_err());
    // The line bound is refused while lexing.
    let many = "- x: y\n".repeat(MAX_LINES + 1);
    assert!(validate(many.as_bytes(), PIN).is_err());
}

#[test]
fn the_endpoint_grammar_admits_a_bare_authority_and_refuses_its_edges() {
    let base = SHIPPED.replace(
        "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1",
        "https://host",
    );
    assert!(validate(base.as_bytes(), PIN).is_ok());
    // A credential-name beginning with `_` is admitted; Brokkr never
    // reads it.
    let underscore = SHIPPED.replace("DASHSCOPE_API_KEY", "_X");
    assert!(validate(underscore.as_bytes(), PIN).is_ok());
    for (value, needle) in [
        ("https://", "endpoint"),
        ("https://host:/x", "endpoint"),
        ("https:///x", "endpoint"),
        ("https://host/", "endpoint"),
        ("https://host..x", "endpoint"),
        ("https://host-/x", "endpoint"),
    ] {
        let body = SHIPPED.replace(
            "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1",
            value,
        );
        let error = validate(body.as_bytes(), PIN).unwrap_err();
        assert!(error.contains(needle), "{value:?}: {error}");
    }
}

#[test]
fn the_reader_guards_hold_at_their_internal_call_boundaries() {
    let lines = lex("a: b\n").unwrap();
    // A block start beyond the line list or at the wrong depth is
    // refused rather than indexed.
    assert!(parse_block(&lines, 1, 0).is_err());
    assert!(parse_block(&lines, 0, 1).is_err());
    // A mapping or a sequence forced with no admitted line is refused.
    assert!(parse_mapping(&lines, 0, 1).is_err());
    assert!(parse_sequence(&lines, 0, 0).is_err());
}

#[test]
fn validate_refuses_a_pinned_model_outside_its_grammar() {
    let error = validate(SHIPPED.as_bytes(), "bad model").unwrap_err();
    assert!(error.contains("not `<id>`"), "{error}");
    // A model item whose `reasoningEfforts` nests a non-mapping is
    // refused at the mapping call rather than hashed.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        models:\n          - id: qwen3.8-max\n            reasoningEfforts: x\n",
        "reasoningEfforts is not a mapping",
    );
}

#[test]
fn claim_refuses_each_malformed_binding_shape() {
    let dir = tempfile::tempdir().unwrap();
    let workdir = dir.path().to_string_lossy().into_owned();
    // A binding that is not an object.
    let non_object = json!({"resume_context": {"route_overlay": 5}});
    let error = claim(&non_object, &workdir, Some(PIN), None).unwrap_err();
    assert!(error.contains("not an object"), "{error}");
    // A binding with no value.
    let no_value = json!({"resume_context": {"route_overlay": {}}});
    let error = claim(&no_value, &workdir, Some(PIN), None).unwrap_err();
    assert!(error.contains("no value"), "{error}");
    // A binding with no digest.
    let no_digest = json!({"resume_context": {"route_overlay": {"value": "route.yml"}}});
    let error = claim(&no_digest, &workdir, Some(PIN), None).unwrap_err();
    assert!(error.contains("no digest"), "{error}");
    // An unreadable working directory.
    let (_, input) = binding(SHIPPED.as_bytes(), "route.yml");
    let error = claim(
        &input,
        "/definitely/not/a/dir",
        Some(PIN),
        Some("route.yml"),
    )
    .unwrap_err();
    assert!(error.contains("working directory is unreadable"), "{error}");
}

#[test]
fn a_key_block_closed_by_a_sibling_line_is_an_empty_block() {
    // `key:` is followed by a line at the same depth, so it has no
    // child block: the reader refuses it rather than reading the
    // sibling as a child.
    refused("key:\nsibling: x\n", "empty `key:` block");
}

#[test]
fn a_mapping_ends_when_a_sequence_item_follows_it() {
    // The mapping loop stops at an item at its own depth, and the
    // leftover item is then an inconsistent indentation.
    refused("id: x\n- item\n", "unexpected indentation");
}

#[test]
fn a_sequence_item_nested_under_the_first_item_ends_the_sequence() {
    // The first item's mapping loop stops at a nested item, and the
    // leftover line is then an inconsistent indentation.
    refused("- id: x\n  - nested\n", "unexpected indentation");
}

#[cfg(unix)]
#[test]
fn claim_refuses_a_bound_file_it_cannot_actually_read() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, input) = binding(SHIPPED.as_bytes(), "route.yml");
    let path = dir.path().join("route.yml");
    // `stat` succeeds on a mode-000 regular file; only the read is
    // refused, which is the second unreadable-file guard.
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    let error = claim(
        &input,
        &dir.path().to_string_lossy(),
        Some(PIN),
        Some("route.yml"),
    )
    .unwrap_err();
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    assert!(error.contains("file is unreadable"), "{error}");
}
