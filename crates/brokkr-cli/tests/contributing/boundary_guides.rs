//! Decision 0046's guides, moved whole from contributing.rs, which stands
//! at its 2,000-line ceiling.

use super::workspace;

/// Decision 0046's guides: no page states that the network was off under
/// `harness` or `open`, and the decision carries its one-line erratum
/// under a heading of its own. Which sections and rows a page keeps is a
/// reviewer's call since #365: a false claim fails here, a lost section
/// does not.
#[test]
fn no_boundary_guide_says_the_network_was_off_and_0046_carries_its_erratum() {
    let root = workspace();
    for page in [
        "docs/guides/provider-adapters.md",
        "docs/guides/recipe-authoring.md",
        "docs/guides/quickstart.md",
        "docs/guides/journal-and-verification.md",
        "docs/guides/repository-layout.md",
        "docs/guides/driver-authoring.md",
        "ARCHITECTURE.md",
        "contracts/README.md",
    ] {
        let raw = std::fs::read_to_string(root.join(page)).unwrap();
        // Task 12.8 (design DD15): the prefix is a narrowing the engine
        // attempts on Linux, and no page says the network was off.
        let lower = raw
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        for claim in ["network was off", "network is off", "network off"] {
            assert!(
                !lower.contains(claim),
                "{page} states the network was off: {claim}"
            );
        }
    }

    // The erratum: one heading, one line, the decision otherwise untouched.
    // The accepted 2026-09-09 Seatbelt addendum follows the erratum, so the
    // erratum section is delimited by the next `##` heading, not the file end.
    let decision =
        std::fs::read_to_string(root.join("docs/decisions/0046-the-boundary-is-named.md")).unwrap();
    assert!(decision.starts_with("# 0046 — The boundary is named"));
    assert!(decision.contains("\nStatus: accepted (operator ruled in chat, 2026-09-05)\n"));
    let erratum: Vec<&str> = decision
        .split("\n## Erratum\n")
        .nth(1)
        .expect("the decision carries its erratum heading")
        .split("\n## ")
        .next()
        .expect("the erratum section is bounded by the next heading")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(
        erratum.len(),
        1,
        "the erratum is exactly one line: {erratum:?}"
    );
    for named in [
        "`seat-record.v3`",
        "`seat-record.v4`",
        "#202",
        "decision 0034 rulings 6 and 7",
        "nothing else is renumbered",
    ] {
        assert!(
            erratum[0].contains(named),
            "the erratum does not name {named}"
        );
    }
}
