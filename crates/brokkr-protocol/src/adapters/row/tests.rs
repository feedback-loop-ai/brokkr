use super::*;
use serde_json::json;

/// A refusal decided on the raw row ends it before the fold reads the
/// masked copy; without one, the fold gets the masked row and identity.
#[test]
fn a_raw_refusal_ends_the_row_before_its_fold() {
    let masked = json!({"type": "assistant"});
    let refused = super::super::refusal::refusal_reason("rate_limit", None, None);
    let mut folded = Vec::new();
    let input = |refusal| FoldInput {
        row: &masked,
        refusal,
        identity: None,
    };
    let ended = input(Some(refused.clone())).fold(|row, _| {
        folded.push(row.clone());
        None
    });
    assert_eq!(ended, Some(refused));
    assert!(folded.is_empty());
    let ended = input(None).fold(|row, identity| {
        folded.push(row.clone());
        assert!(identity.is_none());
        None
    });
    assert_eq!(ended, None);
    assert_eq!(folded, std::slice::from_ref(&masked));
}

fn binding(value: &str) -> Vec<BoundSecret> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bindings");
    secret::store_set(&path, "S", value).unwrap();
    secret::resolve_bindings(&path, &["S".into()]).unwrap()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Answer,
}

#[test]
fn kind_matches_raw_vocabulary() {
    let raw = json!({"type":"assistant"});
    let bindings = binding("assi");
    let row = Row::new(&raw, &bindings);
    let table = &[("assistant", Kind::Answer)];
    assert_eq!(row.kind("/type", table), Some(Kind::Answer));
    assert_eq!(row.kind("/missing", table), None);
}

#[test]
fn is_compares_a_whole_raw_word() {
    let raw = json!({"type":"assistant"});
    let bindings = binding("assi");
    let row = Row::new(&raw, &bindings);
    assert!(row.is("/type", "assistant"));
    assert!(!row.is("/type", "assi"));
}

#[test]
fn flag_reads_only_booleans() {
    let raw = json!({"yes":true,"no":false,"text":"true"});
    let row = Row::new(&raw, &[]);
    assert_eq!(row.flag("/yes"), Some(true));
    assert_eq!(row.flag("/no"), Some(false));
    assert_eq!(row.flag("/text"), None);
}

#[test]
fn count_reads_only_signed_integers() {
    let raw = json!({"integer":-12,"float":12.5,"text":"12"});
    let row = Row::new(&raw, &[]);
    assert_eq!(row.count("/integer"), Some(-12));
    assert_eq!(row.count("/float"), None);
    assert_eq!(row.count("/text"), None);
}

#[test]
fn number_reads_numbers_without_text_coercion() {
    let raw = json!({"float":12.5,"integer":12,"text":"12.5"});
    let row = Row::new(&raw, &[]);
    assert_eq!(row.number("/float"), Some(12.5));
    assert_eq!(row.number("/integer"), Some(12.0));
    assert_eq!(row.number("/text"), None);
}

#[test]
fn token_normalizes_before_matching_raw_vocabulary() {
    let long = "a".repeat(REFUSAL_TOKEN_LIMIT);
    let raw = json!({"error":"\t rate_limit\n", "long":format!("{long}z")});
    let bindings = binding("limit");
    let row = Row::new(&raw, &bindings);
    assert_eq!(
        row.token("/error", &[("rate_limit", Kind::Answer)]),
        Some(Kind::Answer)
    );
    assert_eq!(
        row.token("/long", &[(&long, Kind::Answer)]),
        Some(Kind::Answer)
    );
}

#[test]
fn text_masks_the_whole_string() {
    let raw = json!({"text":"prefix-value-suffix","number":5});
    let bindings = binding("value");
    let row = Row::new(&raw, &bindings);
    assert_eq!(
        row.text("/text").as_deref(),
        Some("prefix-[secret:S]-suffix")
    );
    assert_eq!(row.text("/number"), None);
}

#[test]
fn items_wrap_array_members_in_order_with_the_same_bindings() {
    let raw = json!({"items":[{"word":"paragraph"}, {"word":"second"}], "object":{}});
    let bindings = binding("para");
    let row = Row::new(&raw, &bindings);
    let words = row
        .items("/items")
        .unwrap()
        .map(|item| item.text("/word"))
        .collect::<Vec<_>>();
    assert_eq!(
        words,
        vec![Some("[secret:S]graph".into()), Some("second".into())]
    );
    assert!(row.items("/object").is_none());
}

#[test]
fn opaque_id_matches_its_offer_and_whole_rollout_token() {
    let raw = json!({"id":"thread-1234","other":"thread-1235","bad":"[thread-1234]"});
    let bindings = binding("1234");
    let row = Row::new(&raw, &bindings);
    let id = row.ident("/id").expect("raw identifier");
    assert!(id == Ident::from_offered("thread-1234"));
    assert!(row.ident("/other").is_some_and(|other| id != other));
    assert!(id.is_offered("thread-1234"));
    assert!(!id.is_offered("thread-[secret:S]"));
    assert!(id.names_rollout("rollout-time-thread-1234.jsonl"));
    assert!(!id.names_rollout("rollout-time-thread-12345.jsonl"));
    assert!(!id.names_rollout("rollout-time-xthread-1234.jsonl"));
    assert!(!row
        .ident("/bad")
        .unwrap()
        .names_rollout("rollout-[thread-1234].jsonl"));
    assert_eq!(row.text("/id").as_deref(), Some("thread-[secret:S]"));
}

#[test]
fn unsigned_count_keeps_the_json_integer_range() {
    let raw = json!({"usage": u64::MAX, "zero": 0, "negative": -1, "fraction": 1.5, "text": "12"});
    let row = Row::new(&raw, &[]);
    assert_eq!(
        [
            "/usage",
            "/zero",
            "/negative",
            "/fraction",
            "/text",
            "/missing"
        ]
        .map(|path| row.unsigned_count(path)),
        [Some(u64::MAX), Some(0), None, None, None, None]
    );
}

#[test]
fn at_keeps_bindings_on_the_nested_value() {
    let raw = json!({"nested": {"type": "assistant"}, "null": null});
    let bindings = binding("assi");
    let row = Row::new(&raw, &bindings);
    let nested = row.at("/nested").unwrap();
    assert!(nested.is("/type", "assistant"));
    assert_eq!(nested.text("/type").as_deref(), Some("[secret:S]stant"));
    assert!(row.at("/null").is_some());
    assert!(row.at("/missing").is_none());
}

#[test]
fn tool_keeps_raw_boundaries_in_the_masked_name() {
    use crate::adapters::tests::binding;

    let cases = [
        (
            "mcp__srv__tool",
            "S",
            "v__t",
            Tool::McpUnidentified {
                reported: "mcp__sr[secret:S]ool".into(),
            },
        ),
        (
            "mcp__srv__tool",
            "S",
            "mcp_",
            Tool::McpUnidentified {
                reported: "[secret:S]_srv__tool".into(),
            },
        ),
        (
            "mcp__lefttail__probe",
            "S__CUT",
            "tail",
            Tool::Mcp {
                server: "left[secret:S__CUT]".into(),
                tool: "probe".into(),
            },
        ),
        // The cut moves by the run's growth only: a run strictly inside the
        // server, two runs before the separator, and a run wholly inside the
        // tool component each keep the raw separator's place (F1).
        (
            "mcp__alphabetagamma__probe",
            "S",
            "beta",
            Tool::Mcp {
                server: "alpha[secret:S]gamma".into(),
                tool: "probe".into(),
            },
        ),
        (
            "mcp__abetacbetad__probe",
            "S",
            "beta",
            Tool::Mcp {
                server: "a[secret:S]c[secret:S]d".into(),
                tool: "probe".into(),
            },
        ),
        (
            "mcp__srv__readtokenfile",
            "S",
            "token",
            Tool::Mcp {
                server: "srv".into(),
                tool: "read[secret:S]file".into(),
            },
        ),
        (
            "WebSearch",
            "S",
            "Search",
            Tool::Named {
                name: "Web[secret:S]".into(),
            },
        ),
    ];
    for (name, secret_name, value, expected) in cases {
        let raw = json!({"name": name});
        let bindings = [binding(secret_name, value)];
        assert_eq!(Row::new(&raw, &bindings).tool("/name"), expected);
    }
}

#[test]
fn pin_compares_raw_models_and_only_plain_ids_can_be_quoted() {
    let raw =
        json!({"pinned": "Qwen4-Max", "other": "Next4-Max", "unplain": "with space", "number": 4});
    let bindings = binding("4-Max");
    let row = Row::new(&raw, &bindings);
    let pin = ModelPin::parse("Qwen4-Max").unwrap();
    assert_eq!(row.pin("/pinned", &pin), Pin::Pinned);
    assert_eq!(row.pin("/other", &pin), Pin::Other { plain: true });
    assert_eq!(row.text("/other").as_deref(), Some("Next[secret:S]"));
    assert_eq!(row.pin("/unplain", &pin), Pin::Other { plain: false });
    assert_eq!(row.pin("/number", &pin), Pin::Absent);
    assert_eq!(row.pin("/absent", &pin), Pin::Absent);
}

#[test]
fn init_matches_raw_system_and_subtype_even_without_an_id() {
    let bindings = binding("init");
    let raw = json!({"type": "system", "subtype": "init"});
    assert!(Row::new(&raw, &bindings).is_init());
    for raw in [
        json!({"type": "system"}),
        json!({"type": "assistant", "subtype": "init"}),
    ] {
        assert!(!Row::new(&raw, &bindings).is_init());
    }
}

#[test]
fn equals_compares_the_raw_engine_value() {
    let raw = json!({"cwd":"/my/workdir"});
    let bindings = binding("work");
    let row = Row::new(&raw, &bindings);
    assert!(row.equals("/cwd", "/my/workdir"));
    assert!(!row.equals("/cwd", "/my/[secret:S]dir"));
    assert!(!row.equals("/missing", ""));
}

#[test]
fn since_compares_time_without_returning_it() {
    let raw = json!({"timestamp":"2026-11-08T10:00:00.900100Z"});
    let bindings = binding("9001");
    let row = Row::new(&raw, &bindings);
    let at = parse_nanos("2026-11-08T10:00:00.900100Z").unwrap();
    assert_eq!(row.since("/timestamp", at), Some(true));
    assert_eq!(row.since("/timestamp", at + 1), Some(false));
    assert_eq!(row.since("/missing", at), None);
}

#[test]
fn present_requires_a_non_null_value() {
    let raw = json!({"text":"", "null":null});
    let row = Row::new(&raw, &[]);
    assert!(row.present("/text"));
    assert!(!row.present("/null"));
    assert!(!row.present("/missing"));
}

#[test]
fn has_accepts_an_explicit_null() {
    let raw = json!({"null":null});
    let row = Row::new(&raw, &[]);
    assert!(row.has("/null"));
    assert!(!row.has("/missing"));
}

#[test]
fn reset_rejects_bindings_that_touch_the_consumed_time() {
    let raw = json!({"error":{"message":"usage limit; try again at Nov 11th, 2026 10:38 AM."}});
    let expected = LocalTime {
        year: 2026,
        month: 11,
        day: 11,
        hour: 10,
        minute: 38,
        second: 0,
    };
    for (value, reset) in [("at Nov", None), ("11th", None), ("usage", Some(expected))] {
        let bindings = binding(value);
        let row = Row::new(&raw, &bindings).with_fallback("/error");
        assert_eq!(row.reset("/message"), reset, "{value}");
    }
}

#[test]
fn fallback_is_used_only_for_absent_fields() {
    let raw = json!({"status":null,"error":{"status":429,"message":"private text","type":"inner"},
        "type":"outer"});
    let bindings = binding("private");
    let row = Row::new(&raw, &bindings).with_fallback("/error");
    assert_eq!(row.count("/status"), None);
    assert_eq!(row.text("/message").as_deref(), Some("[secret:S] text"));
    assert!(row.is("/type", "outer"));
    assert_eq!(row.text("/missing"), None);
}

#[test]
fn object_shape_distinguishes_records_from_other_and_missing_values() {
    let raw = json!({"object": {}, "array": [], "null": null});
    let row = Row::new(&raw, &[]);
    assert!(row.is_object("/object"));
    for path in ["/array", "/null", "/missing"] {
        assert!(!row.is_object(path), "{path}");
    }
}
