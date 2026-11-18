//! An aggregate is named by one word, and read back from it; and a
//! panel's member findings are joined typed.

use super::*;

#[test]
fn every_aggregate_reads_back_from_its_word() {
    let words: Vec<&str> = Aggregate::ALL.map(Aggregate::word).to_vec();
    assert_eq!(words, ["unanimous-pass", "review-panel"]);
    for aggregate in Aggregate::ALL {
        assert_eq!(Aggregate::parse(aggregate.word()), Some(aggregate));
    }
    assert_eq!(Aggregate::parse("UnanimousPass"), None);
    assert_eq!(Aggregate::known(), "unanimous-pass, review-panel");
}

/// Fix round 0109a1r2, R6: the members' stamped findings are read typed
/// once and joined in declared order, each with its own member's site; a
/// member whose findings are not the engine's stamped records poisons the
/// aggregate with the member evidence, never joined as they stand.
#[test]
fn member_findings_join_typed_and_an_unstamped_one_poisons_the_aggregate() {
    let stamped = |site: char, id: &str| {
        json!({"site": site.to_string().repeat(64), "id": id, "file": "a.rs", "summary": "s",
               "severity": "low", "security": false})
    };
    let members = [
        (
            "a".to_string(),
            json!({"result": "residual", "findings": [stamped('a', "x")]}),
        ),
        ("b".to_string(), json!({"result": "clean"})),
        (
            "c".to_string(),
            json!({"result": "clean", "findings": [stamped('c', "x")]}),
        ),
    ];
    let joined = Aggregate::ReviewPanel.aggregate_results(&members);
    assert_eq!(joined["result"], "residual");
    assert_eq!(
        joined["findings"],
        json!([stamped('a', "x"), stamped('c', "x")])
    );
    let mut unstamped = members.clone();
    unstamped[2].1["findings"][0]
        .as_object_mut()
        .unwrap()
        .remove("site");
    let poisoned = Aggregate::ReviewPanel.aggregate_results(&unstamped);
    assert_eq!(poisoned["result"], Aggregate::MEMBER_SCHEMA_INVALID);
    assert_eq!(poisoned["notes"]["members"]["c"], unstamped[2].1);
    assert!(poisoned.get("findings").is_none(), "{poisoned}");
}
