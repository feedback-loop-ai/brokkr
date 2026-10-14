//! An aggregate is named by one word, and read back from it.

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
