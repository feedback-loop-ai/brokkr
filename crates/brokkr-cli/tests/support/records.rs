//! The tracked paths whose words are fixed when they are written, for the
//! tests that hold the repository's prose to a rule. Each includes this
//! through `#[path]`, so the list has one home (#362).

/// Paths whose words are records, each with the reason it is not
/// rewritten to today's rules.
pub(crate) const RECORDS: [(&str, &str); 10] = [
    ("contracts/", "frozen contract bodies"),
    ("reference/", "frozen heritage"),
    ("fixtures/", "frozen fixtures"),
    (
        "docs/decisions/",
        "a decision's text is fixed when it is ruled",
    ),
    ("docs/releases/", "release notes say what shipped"),
    ("docs/lore/", "lore is the history as it was told"),
    (
        "docs/essays/",
        "an essay reports what happened, as it happened",
    ),
    ("docs/evidence/", "evidence records work as it happened"),
    (
        "docs/research/",
        "a research entry reads an article as of its date",
    ),
    (
        "openspec/changes/",
        "a change records its proposal and its work",
    ),
];
