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
    let payloads = [
        json!({"result": "residual", "findings": [stamped('a', "x")]}),
        json!({"result": "clean"}),
        json!({"result": "clean", "findings": [stamped('c', "x")]}),
    ];
    let members = |payloads: &[Value]| {
        ["a", "b", "c"]
            .into_iter()
            .zip(payloads)
            .map(|(name, payload)| MemberVerdict::journaled(name, payload.clone()))
            .collect::<Vec<_>>()
    };
    let read = members(&payloads);
    let typed = Vec::<Finding>::deserialize(json!([stamped('a', "x")])).unwrap();
    assert_eq!(read[0].findings, MemberFindings::Read(typed));
    assert_eq!(read[0].payload, json!({"result": "residual"}), "taken out");
    let review = PanelJoin::open(Aggregate::ReviewPanel, &["a", "b", "c"]);
    let joined = review.aggregate_results(&read);
    assert_eq!(joined["result"], "residual");
    assert_eq!(
        joined["findings"],
        json!([stamped('a', "x"), stamped('c', "x")])
    );
    let mut unstamped = payloads.clone();
    unstamped[2]["findings"][0]
        .as_object_mut()
        .unwrap()
        .remove("site");
    let poisoned = review.aggregate_results(&members(&unstamped));
    assert_eq!(poisoned["result"], Aggregate::MEMBER_SCHEMA_INVALID);
    assert_eq!(poisoned["notes"]["members"]["c"], unstamped[2]);
    // Fix round 0109a1r6, W4: a's admitted finding stays the record's
    // keyed finding, once, under its own site, never hidden in the notes.
    let bare = json!({"result": "residual"});
    assert_eq!(poisoned["notes"]["members"]["a"], bare, "{poisoned}");
    assert_eq!(
        poisoned["findings"],
        json!([stamped('a', "x")]),
        "{poisoned}"
    );
}

/// Decision 0109 ruling 11: a review-panel member's word outside its
/// compiled law poisons the aggregate, naming the member and the reason;
/// a member the join holds no law for is refused, never read as lawful;
/// and a partial retry joins its carried members under their own laws.
#[test]
fn a_review_panel_members_word_answers_to_its_law_and_a_lawless_one_poisons() {
    let narrow = MemberLaw::declaring(&["clean", "residual"]);
    let members = MemberVerdict::all(&[
        ("a", json!({"result": "clean"})),
        ("b", json!({"result": "security-hold"})),
    ]);
    let join = PanelJoin::under(
        Aggregate::ReviewPanel,
        [("a", narrow.clone()), ("b", narrow)],
    );
    let poisoned = join.aggregate_results(&members);
    assert_eq!(poisoned["result"], Aggregate::MEMBER_SCHEMA_INVALID);
    assert_eq!(
        poisoned["notes"]["refused"],
        json!({"b": "member result 'security-hold' is not among its declared results [clean, residual]"})
    );
    let lawless = PanelJoin::open(Aggregate::ReviewPanel, &["a"]).aggregate_results(&members);
    assert_eq!(
        lawless["notes"]["refused"],
        json!({"b": "the member has no compiled result set"})
    );
    let carried = PanelJoin::open(Aggregate::ReviewPanel, &["a"])
        .carrying(&[("b".to_string(), MemberLaw::open())].into());
    assert_eq!(
        carried.aggregate_results(&members)["result"],
        "security-hold"
    );
    let unanimous = PanelJoin::open(Aggregate::UnanimousPass, &[]);
    let passing = MemberVerdict::all(&[("a", json!({"result": "pass"}))]);
    assert_eq!(
        unanimous.aggregate_results(&passing)["result"],
        "pass",
        "keeps its own check"
    );
}
