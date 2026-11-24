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
        masked: &masked,
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
fn contains_matches_a_raw_substring() {
    let raw = json!({"message":"please retry later"});
    let bindings = binding("retry");
    let row = Row::new(&raw, &bindings);
    assert!(row.contains("/message", "retry"));
    assert!(!row.contains("/missing", "retry"));
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
