//! The tracked paths whose words are fixed when they are written, for the
//! tests that hold the repository's prose to a rule. `tests/it.rs` includes
//! this once through `#[path]`, and house_prose and doc_truth read it as
//! `crate::records`, so the list has one home (#362).

/// Surfaces whose bytes never move (the house rules), each with the reason
/// it is read-only.
pub(crate) const FROZEN: [(&str, &str); 4] = [
    (
        "contracts/",
        "a contract changes by a new version beside the old one",
    ),
    (
        "fixtures/",
        "the evaluator corpus is versioned, never rewritten",
    ),
    ("reference/", "the reference and its schemas are read-only"),
    ("policy/", "the production table is read-only"),
];

/// Paths whose words are records, each with the reason it is not
/// rewritten to today's rules: the [`FROZEN`] surfaces among them.
pub(crate) const RECORDS: [(&str, &str); 12] = [
    FROZEN[0],
    FROZEN[1],
    FROZEN[2],
    FROZEN[3],
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
        "docs/history/",
        "a superseded blueprint is kept as it was last edited",
    ),
    (
        "docs/research/",
        "a research entry reads an article as of its date",
    ),
    (
        "openspec/changes/",
        "a change records its proposal and its work",
    ),
];
