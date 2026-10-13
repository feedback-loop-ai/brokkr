//! `brokkr analytics`' own part (#568): the hearths it hands over, the
//! headline it paints, and clap's refusal of a range in the view's words.

use clap::Parser;

use super::*;
use crate::Cli;

const NOW: &str = "2026-10-13T12:00:00Z";

#[test]
fn a_hearth_that_will_not_load_is_handed_over_quarantined_and_painted() {
    let dir = tempfile::tempdir().unwrap();
    let ghost = Hearth {
        realms: vec!["ghost".to_string()],
        journal: dir.path().join("ghost.db"),
    };
    let view = derive(&[ghost], Range::Day, None, NOW).unwrap();
    let refusal = view.quarantined[0].refusal.clone();
    assert!(
        !dir.path().join("ghost.db").exists(),
        "a read created a journal"
    );
    assert_eq!(
        paint(&view),
        format!(
            "2026-10-12T12:00:00Z → {NOW} · all realms\n\
             shipped        0 (no change vs previous 24h)\n\
             start → end    —\n\
             waiting on you 0.0h (nothing parked)\n\
             stopped by rule 0\n\
             reported cost  not reported\n\
             quarantined    ghost: {refusal}\n\
             not recorded yet: machine capacity over time · cost limits · costs for Codex and \
             dsh · reasons findings were dismissed\n"
        )
    );
}

#[test]
fn a_named_realm_heads_the_painting_and_a_bad_clock_is_refused() {
    let view = derive(&[], Range::Week, Some("alpha"), NOW).unwrap();
    assert!(paint(&view).starts_with("2026-10-06T12:00:00Z → 2026-10-13T12:00:00Z · alpha\n"));
    let refused = derive(&[], Range::Week, None, "noon").unwrap_err();
    assert_eq!(
        refused.to_string(),
        AnalyticsError::Clock("noon".to_string()).to_string(),
        "the view's words, which its own tests pin"
    );
}

#[test]
fn clap_refuses_a_range_outside_the_three_with_the_views_error() {
    let refused = Cli::try_parse_from(["brokkr", "analytics", "--range", "2d"])
        .err()
        .expect("2d is no range");
    assert_eq!(refused.kind(), clap::error::ErrorKind::ValueValidation);
    let source = std::error::Error::source(&refused)
        .and_then(|source| source.downcast_ref::<RangeRefused>())
        .map(|refused| &refused.0);
    assert_eq!(
        source,
        Some(&AnalyticsError::UnknownRange("2d".to_string()))
    );
}
