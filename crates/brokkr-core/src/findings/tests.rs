//! Decision 0109 ruling 3, bound: the key is `(site, id)`, the locators
//! are the record, and every refusal the ruling names is its own variant.

use serde_json::{json, Value};

use super::*;

const SITE: &str = "5a1e000000000000000000000000000000000000000000000000000000000000";
const OTHER: &str = "07e4000000000000000000000000000000000000000000000000000000000000";

fn site(digest: &str) -> SiteDigest {
    SiteDigest::try_from(digest.to_string()).unwrap()
}

fn reported() -> Value {
    json!({"id": "unchecked-len", "file": "crates/a/src/lib.rs", "line": 12,
           "summary": "the length is not checked", "severity": "medium", "security": false})
}

fn with(mut finding: Value, field: &str, value: Value) -> Value {
    finding[field] = value;
    finding
}

fn without(mut finding: Value, field: &str) -> Value {
    finding.as_object_mut().unwrap().remove(field);
    finding
}

fn keys(findings: &[Value], at: &str) -> Vec<FindingKey> {
    stamp(&Value::from(findings.to_vec()), &site(at))
        .unwrap()
        .iter()
        .map(Finding::key)
        .collect()
}

#[test]
fn a_moved_line_a_reworded_summary_and_a_moved_file_are_the_same_key() {
    let first = keys(&[reported()], SITE);
    let moved_line = with(reported(), "line", json!(40));
    let reworded = with(reported(), "summary", json!("len() is trusted unchecked"));
    let moved_file = with(reported(), "file", json!("crates/b/src/len.rs"));
    let unlined = without(reported(), "line");
    for later in [moved_line, reworded, moved_file, unlined] {
        assert_eq!(keys(std::slice::from_ref(&later), SITE), first, "{later}");
    }
    assert_eq!(first[0].site().as_str(), SITE);
    assert_eq!(first[0].id().as_str(), "unchecked-len");
    let elsewhere = keys(&[reported()], OTHER);
    assert_ne!(
        elsewhere, first,
        "the same id at another site is another finding"
    );
    let renamed = keys(&[with(reported(), "id", json!("len.unchecked"))], SITE);
    assert_ne!(renamed, first, "another id is another finding");
}

#[test]
fn the_engine_stamps_the_reporting_site_and_keeps_the_record() {
    let mut result = json!({"result": "residual", "findings": [reported()]});
    admit(&mut result, Some(SITE)).unwrap();
    assert_eq!(
        result["findings"],
        json!([{"site": SITE, "id": "unchecked-len", "file": "crates/a/src/lib.rs", "line": 12,
                "summary": "the length is not checked", "severity": "medium",
                "security": false}])
    );
    let read = recorded(&result).unwrap();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].key(), keys(&[reported()], SITE)[0]);
    assert_eq!(read[0].file().as_str(), "crates/a/src/lib.rs");
    assert_eq!(read[0].line().map(NonZeroU64::get), Some(12));
    assert_eq!(read[0].summary().as_str(), "the length is not checked");
    assert_eq!(read[0].severity(), Severity::Medium);
    assert!(!read[0].security());
    let mut plain = json!({"result": "clean", "notes": "n"});
    admit(&mut plain, None).unwrap();
    assert_eq!(plain, json!({"result": "clean", "notes": "n"}));
}

#[test]
fn a_duplicate_id_in_one_verdict_is_refused() {
    let moved = with(reported(), "line", json!(99));
    let refused = stamp(&json!([reported(), moved]), &site(SITE)).unwrap_err();
    let id = FindingId::try_from("unchecked-len".to_string()).unwrap();
    assert_eq!(refused, AdmissionRefusal::Duplicate { id });
    assert_eq!(
        refused.to_string(),
        "finding 'unchecked-len' is reported twice by one site (decision 0109 ruling 3)"
    );
    let other = with(reported(), "id", json!("other"));
    assert_eq!(
        stamp(&json!([reported(), other]), &site(SITE))
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn an_id_outside_the_pattern_is_refused() {
    let longest = "a".repeat(ID_MAX_BYTES);
    assert_eq!(
        keys(&[with(reported(), "id", json!(longest))], SITE).len(),
        1
    );
    for id in [
        json!("Upper"),
        json!("-lead"),
        json!(".lead"),
        json!("has space"),
        json!(""),
        json!("a".repeat(ID_MAX_BYTES + 1)),
        json!(7),
    ] {
        let finding = with(reported(), "id", id.clone());
        let refused = stamp(&json!([reported(), finding]), &site(SITE)).unwrap_err();
        assert_eq!(refused, AdmissionRefusal::Id { index: 1 }, "{id}");
    }
    let refused = stamp(&json!([without(reported(), "id")]), &site(SITE)).unwrap_err();
    assert_eq!(refused, AdmissionRefusal::Id { index: 0 });
    assert_eq!(
        refused.to_string(),
        "finding 0 has no id matching ^[a-z0-9][a-z0-9._-]*$ of at most 64 bytes \
         (decision 0109 ruling 3)"
    );
}

#[test]
fn a_seat_supplied_site_is_refused_even_when_it_is_the_right_one() {
    for supplied in [json!(SITE), json!(OTHER), json!(null)] {
        let finding = with(reported(), "site", supplied.clone());
        let mut result = json!({"result": "residual", "findings": [finding]});
        let refused = admit(&mut result, Some(SITE)).unwrap_err();
        assert_eq!(
            refused,
            AdmissionRefusal::SeatSite { index: 0 },
            "{supplied}"
        );
        assert_eq!(
            result["findings"][0]["site"], supplied,
            "nothing is repaired"
        );
    }
    assert_eq!(
        AdmissionRefusal::SeatSite { index: 0 }.to_string(),
        "finding 0 carries a site: the engine stamps each finding's site (decision 0109 ruling 3)"
    );
}

#[test]
fn a_record_that_is_not_a_finding_is_refused() {
    let malformed = [
        with(reported(), "extra", json!(1)),
        without(reported(), "file"),
        with(reported(), "file", json!("/etc/passwd")),
        with(reported(), "file", json!("a/../../etc")),
        with(reported(), "file", json!("")),
        with(reported(), "line", json!(0)),
        with(reported(), "line", json!(-3)),
        with(reported(), "summary", json!("")),
        without(reported(), "summary"),
        with(reported(), "severity", json!("severe")),
        without(reported(), "severity"),
        without(reported(), "security"),
        json!("a prose finding"),
    ];
    for finding in malformed {
        let refused = stamp(&json!([finding]), &site(SITE)).unwrap_err();
        assert_eq!(
            refused,
            AdmissionRefusal::Malformed { index: 0 },
            "{finding}"
        );
    }
    for findings in [json!({"id": "x"}), json!("none")] {
        assert_eq!(
            stamp(&findings, &site(SITE)).unwrap_err(),
            AdmissionRefusal::NotAnArray
        );
    }
}

#[test]
fn findings_with_no_site_to_stamp_are_refused_not_journaled_unkeyed() {
    for unsited in [None, Some("not-a-digest"), Some(&SITE[1..])] {
        let mut result = json!({"result": "residual", "findings": []});
        assert_eq!(
            admit(&mut result, unsited),
            Err(AdmissionRefusal::Unsited),
            "{unsited:?}"
        );
    }
}

/// Decision 0109 ruling 6: a v6 result, which has no `findings`, reads as
/// the empty set and builds no key; a record that does not read is none,
/// never a guessed key; and no key reads without both its site and id.
#[test]
fn a_v6_result_reads_unkeyed_and_no_key_exists_without_a_site_and_an_id() {
    let v6 = json!({"result": "residual", "inputs": {"max_residual_severity": "high"},
                    "notes": "unchecked-len at crates/a/src/lib.rs:12"});
    assert_eq!(recorded(&v6), Some(Vec::new()));
    let unstamped = json!({"result": "residual", "findings": [reported()]});
    assert_eq!(recorded(&unstamped), None);
    let key = json!({"site": SITE, "id": "unchecked-len"});
    let read: FindingKey = serde_json::from_value(key.clone()).unwrap();
    assert_eq!(read, keys(&[reported()], SITE)[0]);
    for partial in [
        json!({"site": SITE}),
        json!({"id": "unchecked-len"}),
        json!({}),
    ] {
        assert!(
            serde_json::from_value::<FindingKey>(partial.clone())
                .unwrap_err()
                .to_string()
                .starts_with("missing field"),
            "{partial}"
        );
    }
    let foreign = with(key, "file", json!("a.rs"));
    assert!(serde_json::from_value::<FindingKey>(foreign)
        .unwrap_err()
        .to_string()
        .starts_with("unknown field `file`"));
}
