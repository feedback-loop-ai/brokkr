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

fn refused(body: &str, reason: Overlay) {
    assert_eq!(
        validate(body.as_bytes(), PIN),
        Err(Refused(reason)),
        "{body:?}"
    );
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
        Overlay::NotTheRouteRow,
    );
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      openrouter:\n        apiKeyEnv: X\n        models:\n          - id: qwen3.8-max\n",
        Overlay::UnpinnedProvider,
    );
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        models:\n          - id: other\n",
        Overlay::UnpinnedModel,
    );
    // Two providers.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        models:\n          - id: qwen3.8-max\n      other:\n        apiKeyEnv: Y\n        models:\n          - id: qwen3.8-max\n",
        Overlay::ProviderCount,
    );
}

#[test]
fn a_credential_value_or_a_field_outside_the_set_is_refused() {
    let outside = || Overlay::OutsideClosedSet(Part::ProviderMapping);
    // An inline apiKey is outside the closed set.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKey: sk-live\n        apiKeyEnv: DASHSCOPE_API_KEY\n        models:\n          - id: qwen3.8-max\n",
        outside(),
    );
    // A literal authorization header, even beside a valid apiKeyEnv.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: DASHSCOPE_API_KEY\n        headers:\n          Authorization: Bearer sk-live\n        models:\n          - id: qwen3.8-max\n",
        outside(),
    );
    // A missing or malformed apiKeyEnv.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        api: openai-completions\n        models:\n          - id: qwen3.8-max\n",
        Overlay::Missing {
            part: Part::ProviderMapping,
            field: Field::ApiKeyEnv,
        },
    );
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: 9LIVE\n        models:\n          - id: qwen3.8-max\n",
        Overlay::ApiKeyEnv,
    );
    // modelOverrides is outside the set too.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: DASHSCOPE_API_KEY\n        modelOverrides:\n          x: y\n        models:\n          - id: qwen3.8-max\n",
        outside(),
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
            Err(Refused(Overlay::Endpoint)),
            "{base:?}"
        );
    }
}

/// A second top-level entry is refused whole, whether it repeats the
/// admitted route or carries a row the overlay may not name, so no
/// entry past the first is ever read or forwarded.
#[test]
fn a_route_document_carrying_a_second_entry_is_refused() {
    for body in [
        format!("{SHIPPED}{SHIPPED}"),
        format!("{SHIPPED}- id: settings\n  config:\n    theme: dark\n"),
    ] {
        refused(&body, Overlay::EntryCount);
    }
}

#[test]
fn executable_or_unrecognized_syntax_is_refused_at_any_depth() {
    for (body, reason) in [
        ("- id: llm-pi-ai\n  config: !!js ctx\n", Overlay::ReservedStart(2)),
        ("- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        __jsExpr: 1\n        models:\n          - id: qwen3.8-max\n", Overlay::KeyNotIdentifier(6)),
        ("- id: llm-pi-ai\n  config: {providers: x}\n", Overlay::ReservedStart(2)),
        ("- id: llm-pi-ai\n  config: &anchor x\n", Overlay::ReservedStart(2)),
        ("- id: llm-pi-ai\n  config: *alias\n", Overlay::ReservedStart(2)),
        ("- id: llm-pi-ai\n  config: |\n    x\n", Overlay::ReservedStart(2)),
        ("- id: 'llm-pi-ai'\n", Overlay::ReservedStart(1)),
        ("- id: llm-pi-ai\n  config:\n    <<: x\n", Overlay::KeyNotIdentifier(3)),
    ] {
        refused(body, reason);
    }
}

#[test]
fn a_route_needs_a_model_pin_and_model_item() {
    assert_eq!(
        validate(SHIPPED.as_bytes(), "deepseek-v4-flash"),
        Err(Refused(Overlay::UnpinnedProvider))
    );
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n",
        Overlay::Missing {
            part: Part::ProviderMapping,
            field: Field::Models,
        },
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
    assert_eq!(error, Refused(Overlay::Disagrees));

    // A `--patch` with no binding and a binding with no `--patch`.
    let error = claim(&json!({}), &workdir, Some(PIN), Some("route.yml")).unwrap_err();
    assert_eq!(error, Refused(Overlay::PatchUnbound));
    let error = claim(&input, &workdir, Some(PIN), None).unwrap_err();
    assert_eq!(error, Refused(Overlay::Disagrees));

    // A digest mismatch refuses before any shape check, even though the
    // bytes are a valid route.
    let (_, wrong) = binding(b"route: changed\n", "route.yml");
    let error = claim(&wrong, &workdir, Some(PIN), Some("route.yml")).unwrap_err();
    assert_eq!(error, Refused(Overlay::DigestMismatch));

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
    assert_eq!(error, Refused(Overlay::Endpoint));
}

#[test]
fn claim_refuses_absolute_traversal_and_escaping_values() {
    let (dir, _) = binding(SHIPPED.as_bytes(), "route.yml");
    let workdir = dir.path().to_string_lossy().into_owned();
    // The refusal under test is the one `Path::is_absolute` decides.
    let absolute = "/etc/passwd";
    for (value, reason) in [
        (absolute, Overlay::Absolute),
        ("../escape.yml", Overlay::ParentComponent),
    ] {
        let input = json!({
            "resume_context": { "route_overlay": { "value": value, "digest": "a".repeat(64) } }
        });
        let error = claim(&input, &workdir, Some(PIN), Some(value)).unwrap_err();
        assert_eq!(error, Refused(reason), "{value:?}");
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
    // The refusal a claim of `dir`'s route earns, bound to `bytes`.
    let claimed = |dir: &tempfile::TempDir, bytes: &[u8]| {
        let workdir = dir.path().to_string_lossy();
        claim(
            &bound(&digest(bytes)),
            &workdir,
            Some(PIN),
            Some("route.yml"),
        )
        .unwrap_err()
    };

    // A symlink inside the working directory resolving outside it.
    let (dir, _) = binding(body, "route.yml");
    std::fs::remove_file(dir.path().join("route.yml")).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("elsewhere.yml"),
        dir.path().join("route.yml"),
    )
    .unwrap();
    assert_eq!(claimed(&dir, body), Refused(Overlay::Outside));

    // A directory where the route file must be a regular file.
    let (dir, _) = binding(body, "route.yml");
    std::fs::remove_file(dir.path().join("route.yml")).unwrap();
    std::fs::create_dir(dir.path().join("route.yml")).unwrap();
    assert_eq!(claimed(&dir, body), Refused(Overlay::NotRegular));

    // Bytes over the reader's finite bound.
    let oversized = vec![b'a'; MAX_BYTES + 1];
    let (dir, _) = binding(&oversized, "route.yml");
    assert_eq!(claimed(&dir, body), Refused(Overlay::FileOverBound));

    // Bytes that are not UTF-8, bound by their own digest.
    let raw = b"- id: llm-pi-ai\n\xff\n";
    let (dir, _) = binding(raw, "route.yml");
    assert_eq!(claimed(&dir, raw), Refused(Overlay::NotUtf8));
}

#[test]
fn a_bound_route_needs_a_pin_a_nonempty_value_and_a_readable_workdir() {
    let (dir, input) = binding(SHIPPED.as_bytes(), "route.yml");
    let workdir = dir.path().to_string_lossy().into_owned();
    // A binding with no pinned model has no provider or id to check.
    let error = claim(&input, &workdir, None, Some("route.yml")).unwrap_err();
    assert_eq!(error, Refused(Overlay::NoProviderPin));
    // An empty value that agrees with the argv refuses before any read.
    let empty = json!({
        "resume_context": { "route_overlay": { "value": "", "digest": "a".repeat(64) } }
    });
    let error = claim(&empty, &workdir, Some(PIN), Some("")).unwrap_err();
    assert_eq!(error, Refused(Overlay::EmptyValue));
    // An empty workdir is read as the current directory, not skipped.
    let error = claim(&input, "", Some(PIN), Some("route.yml")).unwrap_err();
    assert_eq!(error, Refused(Overlay::FileUnreadable));
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
    assert_eq!(error, Refused(Overlay::NoProviderPin));
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
    assert_eq!(error, Refused(Overlay::FileOverBound));
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
    assert_eq!(error, Refused(Overlay::FileUnreadable));
}

#[test]
fn the_reader_refuses_each_malformed_block_shape() {
    // A top-level sequence must hold exactly one entry.
    refused("- id: a\n- id: b\n", Overlay::EntryCount);
    // A top-level mapping is not a sequence of one entry.
    refused("id: llm-pi-ai\n", Overlay::NotASequence);
    // A bare sequence item carries no member.
    refused("-\n", Overlay::BareItem(1));
    // A line that is neither a member nor a closed block key.
    refused("justtext\n", Overlay::Unrecognized(1));
    // A block whose head does not consume every line.
    refused(
        "- id: llm-pi-ai\nfoo: bar\n",
        Overlay::UnexpectedIndentation(2),
    );
    // A `key:` with no child block.
    let empty_key = || Overlay::EmptyKeyBlock {
        line: 1,
        key: "key".into(),
    };
    refused("key:\n", empty_key());
    // A child indented more than one depth under a `key:`.
    refused(
        "- id: llm-pi-ai\n  config:\n      providers: x\n",
        Overlay::DepthJump(3),
    );
    // A provider field repeated.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        api: a\n        api: a\n        models:\n          - id: qwen3.8-max\n",
        Overlay::ProviderFieldRepeated,
    );
    // `models` must be a sequence of exactly one item.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        models: x\n",
        Overlay::ModelsNotASequence,
    );
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        models:\n          - id: a\n          - id: b\n",
        Overlay::ModelCount,
    );
}

#[test]
fn the_semantic_helpers_refuse_non_mappings_and_repeated_or_nested_fields() {
    // `config` must be a mapping.
    refused(
        "- id: llm-pi-ai\n  config: x\n",
        Overlay::NotAMapping(Part::Named(Field::Config)),
    );
    // `providers` must be a mapping.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers: x\n",
        Overlay::NotAMapping(Part::Named(Field::Providers)),
    );
    // A repeated route-entry field is refused by `field`, not collapsed.
    refused(
        "- id: llm-pi-ai\n  id: llm-pi-ai\n",
        Overlay::Repeats {
            part: Part::RouteEntry,
            field: Field::Id,
        },
    );
    // `id` must stay a scalar.
    refused(
        "- id:\n  x: y\n",
        Overlay::NonScalar {
            part: Part::RouteEntry,
            field: Field::Id,
        },
    );
    // `baseURL` must stay a scalar.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        baseURL:\n          x: y\n        models:\n          - id: qwen3.8-max\n",
        Overlay::NonScalar {
            part: Part::ProviderMapping,
            field: Field::BaseUrl,
        },
    );
    // `compat` must stay a mapping.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        compat: x\n        models:\n          - id: qwen3.8-max\n",
        Overlay::NonMapping {
            part: Part::ProviderMapping,
            field: Field::Compat,
        },
    );
}

#[test]
fn keys_and_scalars_are_closed_identifiers_and_unquoted_values() {
    // A key whose body carries a character outside the identifier set.
    refused("id$x: y\n", Overlay::KeyNotIdentifier(1));
    // A key may carry `_` and `-` after its first character.
    refused("a_b-c: value\n", Overlay::NotASequence);
    // An empty value is refused.
    refused("key: \n", Overlay::EmptyScalar(1));
    // A value holding a comment or a second mapping is refused.
    refused("key: a: b\n", Overlay::CommentOrMapping(1));
    refused("key: a # b\n", Overlay::CommentOrMapping(1));
}

#[test]
fn the_reader_refuses_every_lexical_shape_it_did_not_recognize() {
    // Document markers.
    refused("---\n", Overlay::DocumentMarker(1));
    refused("...\n", Overlay::DocumentMarker(1));
    // A tab and a control character.
    refused("- id:\tx\n", Overlay::Control(1));
    refused("- id: x\u{1}\n", Overlay::Control(1));
    // A comment line is checked for them too, and before it is
    // skipped: a lone CR inside one hides a second row from this
    // reader that a CR-tolerant YAML reader would see.
    refused("# route\rapiKey: leaked\n- id: x\n", Overlay::Control(1));
    refused("  # route\t\n- id: x\n", Overlay::Control(1));
    // Odd indentation.
    refused("- id: x\n   y: z\n", Overlay::OddIndentation(2));
    // A whitespace-only line is skipped; a document of them is empty.
    refused("  \n", Overlay::Empty);
    // Comments and blanks alone carry no block line.
    refused("# only a comment\n\n", Overlay::Empty);
    // The byte bound is refused before any parse.
    let huge = vec![b'a'; MAX_BYTES + 1];
    assert_eq!(validate(&huge, PIN), Err(Refused(Overlay::TextOverBound)));
    // The line bound is refused while lexing.
    let many = "- x: y\n".repeat(MAX_LINES + 1);
    refused(&many, Overlay::TooManyLines);
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
    for value in [
        "https://",
        "https://host:/x",
        "https:///x",
        "https://host/",
        "https://host..x",
        "https://host-/x",
    ] {
        let body = SHIPPED.replace(
            "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1",
            value,
        );
        refused(&body, Overlay::Endpoint);
    }
}

#[test]
fn the_reader_guards_hold_at_their_internal_call_boundaries() {
    let lines = lex("a: b\n").unwrap();
    // A block start beyond the line list or at the wrong depth is
    // refused rather than indexed.
    assert_eq!(parse_block(&lines, 1, 0), Err(Overlay::Inconsistent));
    assert_eq!(parse_block(&lines, 0, 1), Err(Overlay::Inconsistent));
    // A mapping or a sequence forced with no admitted line is refused.
    assert_eq!(parse_mapping(&lines, 0, 1), Err(Overlay::EmptyBlock));
    assert_eq!(parse_sequence(&lines, 0, 0), Err(Overlay::EmptySequence));
}

#[test]
fn validate_refuses_a_pinned_model_outside_its_grammar() {
    let error = validate(SHIPPED.as_bytes(), "bad model").unwrap_err();
    assert_eq!(error, Refused(Overlay::PinGrammar));
    // A model item whose `reasoningEfforts` nests a non-mapping is
    // refused at the mapping call rather than hashed.
    refused(
        "- id: llm-pi-ai\n  config:\n    providers:\n      dashscope:\n        apiKeyEnv: X\n        models:\n          - id: qwen3.8-max\n            reasoningEfforts: x\n",
        Overlay::NotAMapping(Part::Named(Field::ReasoningEfforts)),
    );
}

#[test]
fn claim_refuses_each_malformed_binding_shape() {
    let dir = tempfile::tempdir().unwrap();
    let workdir = dir.path().to_string_lossy().into_owned();
    // A binding that is not an object.
    let non_object = json!({"resume_context": {"route_overlay": 5}});
    let error = claim(&non_object, &workdir, Some(PIN), None).unwrap_err();
    assert_eq!(error, Refused(Overlay::BindingNotAnObject));
    // A binding with no value.
    let no_value = json!({"resume_context": {"route_overlay": {}}});
    let error = claim(&no_value, &workdir, Some(PIN), None).unwrap_err();
    assert_eq!(error, Refused(Overlay::NoValue));
    // A binding with no digest.
    let no_digest = json!({"resume_context": {"route_overlay": {"value": "route.yml"}}});
    let error = claim(&no_digest, &workdir, Some(PIN), None).unwrap_err();
    assert_eq!(error, Refused(Overlay::NoDigest));
    // An unreadable working directory.
    let (_, input) = binding(SHIPPED.as_bytes(), "route.yml");
    let error = claim(
        &input,
        "/definitely/not/a/dir",
        Some(PIN),
        Some("route.yml"),
    )
    .unwrap_err();
    assert_eq!(error, Refused(Overlay::WorkdirUnreadable));
}

#[test]
fn a_key_block_closed_by_a_sibling_line_is_an_empty_block() {
    // `key:` is followed by a line at the same depth, so it has no
    // child block: the reader refuses it rather than reading the
    // sibling as a child.
    refused(
        "key:\nsibling: x\n",
        Overlay::EmptyKeyBlock {
            line: 1,
            key: "key".into(),
        },
    );
}

#[test]
fn a_mapping_ends_when_a_sequence_item_follows_it() {
    // The mapping loop stops at an item at its own depth, and the
    // leftover item is then an inconsistent indentation.
    refused("id: x\n- item\n", Overlay::UnexpectedIndentation(2));
}

#[test]
fn a_sequence_item_nested_under_the_first_item_ends_the_sequence() {
    // The first item's mapping loop stops at a nested item, and the
    // leftover line is then an inconsistent indentation.
    refused("- id: x\n  - nested\n", Overlay::UnexpectedIndentation(2));
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
    assert_eq!(error, Refused(Overlay::FileUnreadable));
}

/// The one place each refusal's bytes are pinned (#353): the launch
/// refusal a seat journals is this text behind the shared prefix.
#[test]
fn every_route_refusal_reads_as_it_always_has() {
    let pins = [binding_pins(), grammar_pins(), line_pins()];
    for (reason, text) in pins.into_iter().flatten() {
        assert_eq!(
            Refused(reason).to_string(),
            format!("refusing to invoke the dsh driver: {text}")
        );
    }
    // Every field and part as a refusal spells it.
    let fields = [
        (Field::Id, "id"),
        (Field::Config, "config"),
        (Field::Providers, "providers"),
        (Field::DisplayName, "displayName"),
        (Field::Api, "api"),
        (Field::BaseUrl, "baseURL"),
        (Field::Compat, "compat"),
        (Field::Models, "models"),
        (Field::ApiKeyEnv, "apiKeyEnv"),
        (Field::ReasoningEfforts, "reasoningEfforts"),
    ];
    for (field, spelled) in fields {
        let named = Overlay::NotAMapping(Part::Named(field));
        assert_eq!(named.to_string(), format!("{spelled} is not a mapping"));
    }
    for (part, spelled) in [
        (Part::RouteEntry, "the route entry"),
        (Part::ProviderMapping, "the provider mapping"),
        (Part::ModelItem, "the model item"),
    ] {
        let outside = Overlay::OutsideClosedSet(part);
        let text = format!("{spelled} carries a field outside the closed set");
        assert_eq!(outside.to_string(), text);
    }
}

/// The binding's and the file's refusals, each with its bytes.
fn binding_pins() -> Vec<(Overlay, &'static str)> {
    vec![
        (
            Overlay::PatchUnbound,
            "a `--patch` with no bound route overlay",
        ),
        (
            Overlay::BindingNotAnObject,
            "route_overlay is not an object",
        ),
        (Overlay::NoValue, "route_overlay carries no value"),
        (Overlay::NoDigest, "route_overlay carries no digest"),
        (
            Overlay::Disagrees,
            "the route overlay binding disagrees with the `--patch` value",
        ),
        (
            Overlay::NoProviderPin,
            "a route overlay needs a pinned `--model` with a provider segment",
        ),
        (Overlay::EmptyValue, "route_overlay value is empty"),
        (Overlay::Absolute, "route_overlay value is an absolute path"),
        (
            Overlay::ParentComponent,
            "route_overlay value contains a `..` component",
        ),
        (
            Overlay::WorkdirUnreadable,
            "route_overlay working directory is unreadable",
        ),
        (Overlay::FileUnreadable, "route_overlay file is unreadable"),
        (
            Overlay::Outside,
            "route_overlay value resolves outside the working directory",
        ),
        (
            Overlay::NotRegular,
            "route_overlay value is not a regular file",
        ),
        (
            Overlay::FileOverBound,
            "route_overlay exceeds the reader's byte bound",
        ),
        (
            Overlay::DigestMismatch,
            "route_overlay bytes do not hash to the bound manifest digest",
        ),
    ]
}

/// The closed grammar's semantic refusals, each with its bytes.
fn grammar_pins() -> Vec<(Overlay, &'static str)> {
    let (part, field) = (Part::ProviderMapping, Field::Compat);
    vec![
        (Overlay::NotUtf8, "route overlay is not UTF-8"),
        (
            Overlay::EntryCount,
            "route overlay must hold exactly one top-level entry",
        ),
        (
            Overlay::NotASequence,
            "route overlay must be a sequence of one entry",
        ),
        (
            Overlay::NotTheRouteRow,
            "the route entry id is not the route row",
        ),
        (
            Overlay::ProviderCount,
            "providers must define exactly one provider",
        ),
        (
            Overlay::PinGrammar,
            "the pinned model is not `<id>` or `<provider>/<id>`",
        ),
        (
            Overlay::UnpinnedProvider,
            "the route names a provider the seat did not pin",
        ),
        (
            Overlay::ProviderFieldRepeated,
            "the provider mapping repeats a field",
        ),
        (
            Overlay::ApiKeyEnv,
            "apiKeyEnv is not an environment-variable name",
        ),
        (
            Overlay::Endpoint,
            "baseURL leaves the closed endpoint grammar",
        ),
        (Overlay::ModelCount, "models must hold exactly one item"),
        (Overlay::ModelsNotASequence, "models is not a sequence"),
        (
            Overlay::UnpinnedModel,
            "the route names a model the seat did not pin",
        ),
        (
            Overlay::Missing { part, field },
            "the provider mapping is missing `compat`",
        ),
        (
            Overlay::Repeats { part, field },
            "the provider mapping repeats `compat`",
        ),
        (
            Overlay::NonScalar { part, field },
            "the provider mapping nests a non-scalar `compat`",
        ),
        (
            Overlay::NonMapping { part, field },
            "the provider mapping nests a non-mapping `compat`",
        ),
    ]
}

/// The bounded line reader's refusals, each with its bytes.
fn line_pins() -> Vec<(Overlay, &'static str)> {
    vec![
        (
            Overlay::TextOverBound,
            "route overlay exceeds the reader's byte bound",
        ),
        (
            Overlay::Control(6),
            "route overlay line 6 carries a tab or control character",
        ),
        (
            Overlay::DocumentMarker(6),
            "route overlay line 6 is a document marker",
        ),
        (
            Overlay::OddIndentation(6),
            "route overlay line 6 has an odd indentation",
        ),
        (Overlay::TooManyLines, "route overlay has too many lines"),
        (Overlay::Empty, "route overlay is empty"),
        (
            Overlay::UnexpectedIndentation(6),
            "route overlay line 6 has an unexpected indentation",
        ),
        (
            Overlay::Inconsistent,
            "route overlay indentation is inconsistent",
        ),
        (Overlay::EmptyBlock, "route overlay block has no lines"),
        (
            Overlay::BareItem(6),
            "route overlay line 6 is a bare sequence item",
        ),
        (Overlay::EmptySequence, "route overlay sequence is empty"),
        (
            Overlay::DepthJump(6),
            "route overlay line 6 jumps more than one depth",
        ),
        (
            Overlay::EmptyKeyBlock {
                line: 6,
                key: "compat".into(),
            },
            "route overlay line 6 has an empty `compat:` block",
        ),
        (
            Overlay::Unrecognized(6),
            "route overlay line 6 is not a recognized block line",
        ),
        (
            Overlay::KeyNotIdentifier(6),
            "route overlay line 6 carries a key that is not a plain identifier",
        ),
        (
            Overlay::EmptyScalar(6),
            "route overlay line 6 carries an empty value",
        ),
        (
            Overlay::ReservedStart(6),
            "route overlay line 6 carries a value beginning with a reserved character",
        ),
        (
            Overlay::CommentOrMapping(6),
            "route overlay line 6 carries a value with a comment or an extra mapping",
        ),
    ]
}
