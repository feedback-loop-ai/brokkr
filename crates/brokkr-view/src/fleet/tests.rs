//! The fleet's derivations (#491): the title a row is called by, the
//! verdict it shows, and the section it is listed under.

use super::*;
use crate::tests::state;
use crate::{Superseded, SupersededBy};
use serde_json::json;

const NOW: &str = "2026-01-02T00:00:00Z";
/// A day before [`NOW`], to the second: the first moment a finished run
/// is older.
const DAY_BEFORE: &str = "2026-01-01T00:00:00Z";
const JUST_UNDER_A_DAY: &str = "2026-01-01T00:00:01Z";

/// A run the fold has settled: no park reason unless the test gives one.
fn settled(status: Status, phase: &str, decision: Option<Value>) -> RunState {
    let mut state = state(Some(phase), status, decision);
    state.park_reason = None;
    state
}

fn ruled(rule: &str, from: &str, next: Option<&str>) -> Option<Value> {
    let mut decision = json!({"rule_id": rule, "from": from, "result": "ruled"});
    if let Some(next) = next {
        decision["next"] = json!(next);
    }
    Some(decision)
}

fn finding(input: &str, value: &str, superseded: bool) -> ResidualFinding {
    ResidualFinding {
        run_id: "r1".to_string(),
        seq: 9,
        phase: "review".to_string(),
        rule_id: "REVIEW-RESIDUAL-OK".to_string(),
        input: input.to_string(),
        value: value.to_string(),
        line: format!("r1 seq 9 · review · {input}: {value}"),
        superseded: superseded.then(|| Superseded {
            seq: 12,
            by: SupersededBy {
                realm: None,
                run_id: "r2".to_string(),
                seq: 30,
            },
            reason: "closed by the fix".to_string(),
            operator: "operator".to_string(),
            recorded_at: DAY_BEFORE.to_string(),
        }),
    }
}

fn wanted(standing: Standing, rule: Option<&str>, residual: Option<&str>, text: &str) -> Verdict {
    Verdict {
        standing,
        rule: rule.map(str::to_string),
        residual: residual.map(str::to_string),
        reason: None,
        text: text.to_string(),
    }
}

// ------------------------------------------------------------ the title

#[test]
fn a_title_is_the_first_line_that_says_anything() {
    assert_eq!(title("fix the fleet\nand the detail pane"), "fix the fleet");
    assert_eq!(
        title("\n   \n  STORY #491: the fleet  \nrest"),
        "STORY #491: the fleet"
    );
    assert_eq!(title(""), "");
    let sixty = "a".repeat(60);
    assert_eq!(title(&sixty), sixty, "sixty columns is not clamped");
}

#[test]
fn a_long_title_is_cut_at_a_word_within_sixty_columns() {
    let feature = "The TUI's fleet view prints whole commissions for every run: \
                   show titles, sort by who must act";
    assert_eq!(
        title(feature),
        "The TUI's fleet view prints whole commissions for every…"
    );
    // A word that ends exactly where the columns run out is kept whole.
    let words = "abcd ".repeat(13);
    assert_eq!(title(&words), format!("{}abcd…", "abcd ".repeat(11)));
    // A line with nowhere to cut is cut where it stops fitting.
    assert_eq!(title(&"x".repeat(100)), format!("{}…", "x".repeat(59)));
}

/// Acceptance 1: the clamp counts display columns, so a wide character
/// counts two, a combining mark none, and a multi-byte character one,
/// and no title is wider than sixty columns in the string's own measure.
#[test]
fn a_title_is_clamped_by_display_width_not_by_chars() {
    let cases = [
        // Wide, with nowhere to cut: 29 of them fill 58 columns.
        ("漢".repeat(40), format!("{}…", "漢".repeat(29))),
        // Wide, cut at a word: eleven 5-column units and one more word.
        ("漢字 ".repeat(20), format!("{}漢字…", "漢字 ".repeat(11))),
        // Multi-byte and one column each.
        ("é".repeat(70), format!("{}…", "é".repeat(59))),
        // A combining mark takes no column of its own.
        ("e\u{301}".repeat(70), format!("{}…", "e\u{301}".repeat(59))),
        // An emoji presentation selector widens the character before it
        // in the string's measure alone: the prefix steps back to fit.
        (
            "☺\u{FE0F}".repeat(40),
            format!("{}☺…", "☺\u{FE0F}".repeat(29)),
        ),
    ];
    for (feature, clamped) in cases {
        let got = title(&feature);
        assert_eq!(got, clamped);
        assert!(got.width() <= TITLE_COLUMNS, "{got} is {}", got.width());
    }
    assert_eq!(title(&"漢".repeat(40)).width(), 59);
    assert_eq!(title(&"漢字 ".repeat(20)).width(), 60);
    assert_eq!(title(&"☺\u{FE0F}".repeat(40)).width(), 60);
}

// ---------------------------------------------------------- the verdict

/// Acceptance 4, the stops: a policy stop names its rule without the
/// phase that ruled it, an exhausted bound among them; an operator stop
/// names who stopped it, and no rule.
#[test]
fn a_stop_says_who_stopped_it_and_by_which_rule() {
    let hold = settled(
        Status::Stopped,
        "stop",
        ruled("REVIEW-SECURITY-HOLD", "review", Some("stop")),
    );
    assert_eq!(
        verdict(
            Some(&hold),
            &[finding("max_residual_severity", "high", false)]
        ),
        wanted(
            Standing::Stopped,
            Some("REVIEW-SECURITY-HOLD"),
            Some("high"),
            "SECURITY-HOLD"
        )
    );
    let exhausted = settled(
        Status::Stopped,
        "stop",
        ruled("VERIFY-FAIL-EXHAUSTED", "verify", Some("stop")),
    );
    assert_eq!(
        verdict(Some(&exhausted), &[]),
        wanted(
            Standing::Stopped,
            Some("VERIFY-FAIL-EXHAUSTED"),
            None,
            "FAIL-EXHAUSTED"
        )
    );
    let debt = settled(
        Status::Stopped,
        "stop",
        ruled("REVIEW-REFORGE-EXHAUSTED-DEBT", "review", Some("stop")),
    );
    assert_eq!(verdict(Some(&debt), &[]).text, "REFORGE-EXHAU…");
    let unruled = settled(Status::Stopped, "stop", None);
    assert_eq!(
        verdict(Some(&unruled), &[]),
        wanted(Standing::Stopped, None, None, ABSENT)
    );
    let operator = settled(
        Status::Stopped,
        "review",
        ruled("IMPL-OK", "implement", Some("review")),
    );
    assert_eq!(
        verdict(Some(&operator), &[]),
        wanted(Standing::OperatorStopped, None, None, "by operator")
    );
}

/// Acceptance 4, shipped: the cell is the worst OPEN residual, `clean`
/// when there is none; a superseded finding is closed, and a boolean
/// flag names no severity.
#[test]
fn a_shipped_run_shows_its_worst_open_residual() {
    let shipped = settled(
        Status::Completed,
        "done",
        ruled("SHIP-COMPLETE", "ship", Some("done")),
    );
    assert_eq!(
        verdict(Some(&shipped), &[]),
        wanted(Standing::Shipped, Some("SHIP-COMPLETE"), None, "clean")
    );
    let residuals = [
        finding("max_residual_severity", "low", false),
        finding("max_residual_severity", "medium", true),
        finding("has_security_residual", "true", false),
    ];
    assert_eq!(
        verdict(Some(&shipped), &residuals),
        wanted(Standing::Shipped, Some("SHIP-COMPLETE"), Some("low"), "low")
    );
}

/// Acceptance 4, the runs that need the operator: a park a ruling made
/// names the rule, one it did not make names none, and both carry the
/// fold's reason; a quarantined run says its journal does not fold.
#[test]
fn a_run_that_needs_you_says_why() {
    let mut held = settled(
        Status::AwaitingOperator,
        "review",
        ruled("REVIEW-UNVERIFIED-SECURITY", "review", None),
    );
    held.park_reason = Some("REVIEW-UNVERIFIED-SECURITY for (review, clean)".to_string());
    let mut wanted_held = wanted(
        Standing::Parked,
        Some("REVIEW-UNVERIFIED-SECURITY"),
        None,
        "UNVERIFIED-SE…",
    );
    wanted_held.reason = held.park_reason.clone();
    assert_eq!(verdict(Some(&held), &[]), wanted_held);
    let mut spent = settled(
        Status::AwaitingOperator,
        "implement",
        ruled("ARCH-OK", "architect", Some("implement")),
    );
    spent.park_reason = Some("attempt limit exhausted".to_string());
    let mut wanted_spent = wanted(Standing::Parked, None, None, ABSENT);
    wanted_spent.reason = spent.park_reason.clone();
    assert_eq!(verdict(Some(&spent), &[]), wanted_spent);
    assert_eq!(
        verdict(None, &[]),
        wanted(Standing::Quarantined, None, None, "does not fold")
    );
    let running = settled(
        Status::Running,
        "review",
        ruled("IMPL-OK", "implement", Some("review")),
    );
    assert_eq!(
        verdict(Some(&running), &[]),
        wanted(Standing::Running, None, None, "")
    );
}

#[test]
fn a_rule_loses_only_the_prefix_of_the_phase_that_ruled_it() {
    assert_eq!(abbreviate("IMPL-BLOCKED", Some("implement")), "BLOCKED");
    assert_eq!(abbreviate("OPERATOR-STOP", Some("review")), "OPERATOR-STOP");
    assert_eq!(abbreviate("WORK", Some("work")), "WORK");
    assert_eq!(abbreviate("SHIP-COMPLETE", None), "SHIP-COMPLETE");
}

#[test]
fn each_standing_and_section_has_one_word() {
    let standings = [
        Standing::Quarantined,
        Standing::Parked,
        Standing::Running,
        Standing::Shipped,
        Standing::Stopped,
        Standing::OperatorStopped,
    ];
    assert_eq!(
        standings.map(Standing::label),
        [
            "quarantined",
            "parked",
            "running",
            "shipped",
            "stopped",
            "stopped"
        ]
    );
    assert_eq!(
        Section::ORDER.map(Section::label),
        ["needs you", "running", "last 24h", "older"]
    );
}

// --------------------------------------------------------- the sections

/// One run in each place a section can put it, oldest first as the
/// store lists them.
fn fleet(
    running: &RunState,
    parked: &RunState,
    shipped: &RunState,
    stopped: &RunState,
) -> RunsView {
    let entry = |run_id, created_at, state| RunEntry {
        run_id,
        feature: "a run",
        created_at,
        state,
        detail: None,
        residuals: &[],
    };
    run_rows(&[
        entry("quarantined", DAY_BEFORE, None),
        entry("parked", DAY_BEFORE, Some(parked)),
        entry("running", DAY_BEFORE, Some(running)),
        entry("a-day-old", DAY_BEFORE, Some(shipped)),
        entry("under-a-day", JUST_UNDER_A_DAY, Some(stopped)),
        entry("undated", "not a timestamp", Some(stopped)),
    ])
}

fn ids(sections: Vec<(Section, Vec<&RunRow>)>) -> Vec<(Section, Vec<&str>)> {
    sections
        .into_iter()
        .map(|(section, rows)| {
            (
                section,
                rows.iter().map(|row| row.run_id.as_str()).collect(),
            )
        })
        .collect()
}

/// Acceptance 3's derivation: the sections come in the order who must
/// act reads them, each keeps the rows newest first, a finished run
/// leaves the last 24 hours at exactly a day, and a run whose age cannot
/// be read is never hidden among the older ones.
#[test]
fn the_sections_are_ordered_by_who_must_act() {
    let running = settled(Status::Running, "review", None);
    let parked = settled(Status::AwaitingOperator, "review", None);
    let shipped = settled(Status::Completed, "done", None);
    let stopped = settled(Status::Stopped, "review", None);
    let view = fleet(&running, &parked, &shipped, &stopped);
    assert_eq!(
        ids(sections(&view.runs, NOW)),
        [
            (Section::NeedsYou, vec!["parked", "quarantined"]),
            (Section::Running, vec!["running"]),
            (Section::Recent, vec!["undated", "under-a-day"]),
            (Section::Older, vec!["a-day-old"]),
        ]
    );
    assert_eq!(
        ids(sections(&view.runs, "no clock")),
        [
            (Section::NeedsYou, vec!["parked", "quarantined"]),
            (Section::Running, vec!["running"]),
            (Section::Recent, vec!["undated", "under-a-day", "a-day-old"]),
            (Section::Older, vec![]),
        ]
    );
}

/// The wire: a row carries its title and its verdict beside the whole
/// feature, so `--json` stays lossless.
#[test]
fn a_run_row_carries_its_title_and_verdict_beside_the_whole_feature() {
    let shipped = settled(
        Status::Completed,
        "done",
        ruled("SHIP-COMPLETE", "ship", Some("done")),
    );
    let feature = "fleet titles\n\nthe whole commission, every line of it";
    let view = run_rows(&[RunEntry {
        run_id: "r1",
        feature,
        created_at: DAY_BEFORE,
        state: Some(&shipped),
        detail: None,
        residuals: &[],
    }]);
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(json["view_version"], 12);
    assert_eq!(json["runs"][0]["feature"], feature);
    assert_eq!(json["runs"][0]["title"], "fleet titles");
    assert_eq!(
        json["runs"][0]["verdict"],
        json!({"standing": "shipped", "rule": "SHIP-COMPLETE", "residual": null,
               "reason": null, "text": "clean"})
    );
}
