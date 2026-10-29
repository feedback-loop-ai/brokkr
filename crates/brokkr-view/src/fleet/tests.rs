//! The fleet's derivations (#491): the title a row is called by, the
//! verdict it shows, and the section it is listed under.

use super::*;
use crate::tests::state;
use crate::{Superseded, SupersededBy};
use brokkr_core::fold::{Cursor, ParkReason, Run};
use serde_json::{json, Value};

const NOW: &str = "2026-01-02T00:00:00Z";
/// A day before [`NOW`], to the second: the first moment a finished run
/// is older.
const DAY_BEFORE: &str = "2026-01-01T00:00:00Z";
const JUST_UNDER_A_DAY: &str = "2026-01-01T00:00:01Z";

/// A run the fold has settled, parked on `needs a human` when it parked.
fn settled(status: Status, phase: &str, decision: Option<Value>) -> RunState {
    state(Some(phase), status, decision)
}

/// `state` parked for `reason`.
fn parked_for(mut state: RunState, reason: &str) -> RunState {
    state.run = Run::AwaitingOperator {
        reason: ParkReason::read(reason.to_string()),
    };
    state
}

pub(crate) fn ruled(rule: &str, from: &str, next: Option<&str>) -> Option<Value> {
    let mut decision = json!({"rule_id": rule, "from": from, "result": "ruled"});
    if let Some(next) = next {
        decision["next"] = json!(next);
    }
    Some(decision)
}

pub(crate) fn finding(input: &str, value: &str, superseded: bool) -> ResidualFinding {
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

fn wanted(
    standing: Standing,
    rule: Option<&str>,
    residual: Option<Severity>,
    text: &str,
) -> Verdict {
    Verdict {
        standing,
        rule: rule.map(str::to_string),
        residual,
        reason: None,
        text: text.to_string(),
        detail: None,
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
            Some(Severity::High),
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
    let unnamed = settled(
        Status::Stopped,
        "stop",
        Some(json!({"rule_id": null, "from": "review", "result": "ruled"})),
    );
    assert_eq!(
        verdict(Some(&unnamed), &[]),
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

/// #909: an accepted operator stop with no ending written stands as
/// stopping, not running, under the section that needs the operator; its
/// detail names the accepted seq and the command that closes it. A
/// `run/stopped` after the stop reads exactly as it always did.
#[test]
fn a_stop_accepted_without_an_ending_stands_as_stopping() {
    let detail = "stop accepted at seq 93; no ending written. If no driver is running it, \
                  `brokkr conclude` closes it.";
    let mut accepted = settled(Status::Running, "verify", None);
    accepted.run = Run::Running(Cursor::Stop);
    accepted.stop_accepted_seq = Some(93);
    let row = silent_since(&accepted, Some(NOW));
    assert_eq!(row.verdict.standing, Standing::Stopping);
    assert_eq!(row.status_word(), "stopping");
    assert_eq!(row.verdict.detail.as_deref(), Some(detail));
    assert_eq!(
        need(&row, NOW),
        Some(Need::Stopping {
            detail: detail.to_string()
        })
    );
    assert_eq!(section_of(&row, NOW), Section::NeedsYou);

    // A stop riding an in-flight effect is the same sentence: the cursor
    // still holds the attempt, and the accepted seq is what its detail
    // names.
    let mut riding = settled(Status::Running, "verify", None);
    riding.run = Run::Running(Cursor::EffectInFlight {
        effect_id: "e1".to_string(),
        attempt_id: "a1".to_string(),
        seat: "verify".to_string(),
        failed_attempts: 0,
        stop_rides: true,
    });
    riding.stop_accepted_seq = Some(92);
    let riding = silent_since(&riding, Some(NOW));
    assert_eq!(riding.verdict.standing, Standing::Stopping);
    assert!(riding
        .verdict
        .detail
        .as_deref()
        .is_some_and(|detail| detail.starts_with("stop accepted at seq 92;")));
    assert_eq!(section_of(&riding, NOW), Section::NeedsYou);

    // The driver's ending lands: stopped by the operator, as today.
    let mut ended = accepted;
    ended.run = Run::Stopped;
    let ended = silent_since(&ended, Some(NOW));
    assert_eq!(ended.verdict.standing, Standing::OperatorStopped);
    assert_eq!(ended.status_word(), "stopped");
    assert_eq!(ended.verdict.detail, None);
    assert_eq!(section_of(&ended, NOW), Section::Recent);
}

/// Acceptance 4, shipped: the cell names the ruling that shipped it, and
/// the residual is the worst OPEN one; a superseded finding is closed, a
/// boolean flag names no severity, and `none` is no residual.
#[test]
fn a_shipped_run_shows_its_ruling_and_its_worst_open_residual() {
    let shipped = settled(
        Status::Completed,
        "done",
        ruled("SHIP-COMPLETE", "ship", Some("done")),
    );
    assert_eq!(
        verdict(Some(&shipped), &[]),
        wanted(Standing::Shipped, Some("SHIP-COMPLETE"), None, "COMPLETE")
    );
    let residuals = [
        finding("max_residual_severity", "low", false),
        finding("max_residual_severity", "medium", true),
        finding("has_security_residual", "true", false),
    ];
    assert_eq!(
        verdict(Some(&shipped), &residuals),
        wanted(
            Standing::Shipped,
            Some("SHIP-COMPLETE"),
            Some(Severity::Low),
            "COMPLETE"
        )
    );
    let none = [finding("max_residual_severity", "none", false)];
    assert_eq!(verdict(Some(&shipped), &none).residual, None);
}

/// Acceptance 4, the runs that need the operator: a park a ruling made
/// names the rule, one it did not make names none, and both carry the
/// fold's reason; a quarantined run says its journal does not fold.
#[test]
fn a_run_that_needs_you_says_why() {
    let held = parked_for(
        settled(
            Status::AwaitingOperator,
            "review",
            ruled("REVIEW-UNVERIFIED-SECURITY", "review", None),
        ),
        "REVIEW-UNVERIFIED-SECURITY for (review, clean)",
    );
    let mut wanted_held = wanted(
        Standing::Parked,
        Some("REVIEW-UNVERIFIED-SECURITY"),
        None,
        "UNVERIFIED-SE…",
    );
    wanted_held.reason = Some("REVIEW-UNVERIFIED-SECURITY for (review, clean)".to_string());
    assert_eq!(verdict(Some(&held), &[]), wanted_held);
    let spent = parked_for(
        settled(
            Status::AwaitingOperator,
            "implement",
            ruled("ARCH-OK", "architect", Some("implement")),
        ),
        "attempt limit exhausted",
    );
    let mut wanted_spent = wanted(Standing::Parked, None, None, ABSENT);
    wanted_spent.reason = Some("attempt limit exhausted".to_string());
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
        Standing::Stopping,
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
            "stopping",
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
        last_recorded_at: None,
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
            (Section::NeedsYou, vec!["quarantined", "parked"]),
            (Section::Running, vec!["running"]),
            (Section::Recent, vec!["undated", "under-a-day"]),
            (Section::Older, vec!["a-day-old"]),
        ]
    );
    assert_eq!(
        ids(sections(&view.runs, "no clock")),
        [
            (Section::NeedsYou, vec!["quarantined", "parked"]),
            (Section::Running, vec!["running"]),
            (Section::Recent, vec!["undated", "under-a-day", "a-day-old"]),
            (Section::Older, vec![]),
        ]
    );
}

/// A finished run is dated by when its journal last moved, not by when
/// it was created: one that waited weeks on a ruling and ended within
/// the day is in the last 24 hours. A journal that never moved falls
/// back to the run's creation.
#[test]
fn a_finished_run_is_dated_by_its_last_event() {
    let shipped = settled(Status::Completed, "done", None);
    let entry = |run_id, last_recorded_at| RunEntry {
        run_id,
        feature: "a run",
        created_at: "2025-12-01T00:00:00Z",
        last_recorded_at,
        state: Some(&shipped),
        detail: None,
        residuals: &[],
    };
    let view = run_rows(&[
        entry("ended-a-day-ago", Some(DAY_BEFORE)),
        entry("ended-today", Some(JUST_UNDER_A_DAY)),
        entry("never-moved", None),
    ]);
    assert_eq!(
        ids(sections(&view.runs, NOW)),
        [
            (Section::NeedsYou, vec![]),
            (Section::Running, vec![]),
            (Section::Recent, vec!["ended-today"]),
            (Section::Older, vec!["ended-a-day-ago", "never-moved"]),
        ]
    );
}

/// #511: a quarantine the operator acknowledged needs nothing and lists
/// under "older" however recent it is, keeping its `?`, its refusal and
/// what refused it; the same quarantine unacknowledged still needs them.
#[test]
fn an_acknowledged_quarantine_needs_nothing_and_lists_under_older_even_when_recent() {
    let acknowledgement = Acknowledgement {
        seq: 539,
        event_hash: "ab12".to_string(),
        operator: "valentin".to_string(),
        reason: "history from an engine since changed".to_string(),
        recorded_at: JUST_UNDER_A_DAY.to_string(),
    };
    let refused = |run_id, quarantine: Quarantine, acknowledged| RunEntry {
        detail: Some(Refusal {
            acknowledgement: acknowledged,
            ..quarantine.in_words("event 539: event after terminal status")
        }),
        ..crate::tests::listed(run_id, "a run", NOW, None)
    };
    let view = run_rows(&[
        refused("fold", Quarantine::DoesNotFold, Some(&acknowledgement)),
        refused("load", Quarantine::DoesNotLoad, Some(&acknowledgement)),
        refused("open", Quarantine::DoesNotFold, None),
    ]);
    assert_eq!(
        view.runs
            .iter()
            .map(|row| need(row, NOW))
            .collect::<Vec<_>>(),
        [Some(Need::Quarantined(Quarantine::DoesNotFold)), None, None]
    );
    assert_eq!(
        ids(sections(&view.runs, NOW)),
        [
            (Section::NeedsYou, vec!["open"]),
            (Section::Running, vec![]),
            (Section::Recent, vec![]),
            (Section::Older, vec!["load", "fold"]),
        ]
    );
    let load = &view.runs[1];
    assert_eq!(load.verdict.standing, Standing::Quarantined);
    assert_eq!(
        (load.status.as_deref(), load.quarantine),
        (None, Some(Quarantine::DoesNotLoad))
    );
    assert_eq!(
        load.detail.as_deref(),
        Some("event 539: event after terminal status")
    );
    assert_eq!(load.acknowledgement.as_ref(), Some(&acknowledgement));
    assert_eq!(view.runs[0].acknowledgement, None);
    assert_eq!(
        acknowledgement.line(),
        "acknowledged by valentin at 2026-01-01T00:00:01Z, event 539: history from an engine \
         since changed"
    );
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(
        json["runs"][2]["acknowledgement"],
        json!({"seq": 539, "event_hash": "ab12", "operator": "valentin",
               "reason": "history from an engine since changed",
               "recorded_at": JUST_UNDER_A_DAY})
    );
    assert_eq!(json["runs"][2]["verdict"]["standing"], "quarantined");
}

// ------------------------------------------------------------- the wrap

/// The detail pane's lines: broken at the last space that fits, by
/// display columns, a word wider than the width cut where it stops
/// fitting, indentation kept on the line it starts, and a blank line
/// kept as a line.
#[test]
fn a_text_wraps_at_a_word_by_display_columns() {
    assert_eq!(
        wrap("one two three four\n\nfive", 9),
        ["one two", "three", "four", "", "five"]
    );
    assert_eq!(wrap("abcdefghij klm", 4), ["abcd", "efgh", "ij", "klm"]);
    assert_eq!(wrap("   abcdefgh ij", 6), ["   abc", "defgh", "ij"]);
    assert_eq!(wrap("漢字 漢字漢字", 5), ["漢字", "漢字", "漢字"]);
    assert_eq!(
        wrap("e\u{301}e\u{301}e\u{301} x", 3),
        ["e\u{301}e\u{301}e\u{301}", "x"]
    );
    // Narrower than one wide character: a character a line, and it ends.
    assert_eq!(wrap("漢字", 1), ["漢", "字"]);
    assert_eq!(wrap("", 9), Vec::<String>::new());
    for line in wrap(&"word ".repeat(300), 98) {
        assert!(line.width() <= 98, "{line}");
    }
}

/// The wire: a row carries its title and its verdict beside the whole
/// feature, so `--json` stays lossless, and when its journal last moved.
#[test]
fn a_run_row_carries_its_title_and_verdict_beside_the_whole_feature() {
    let shipped = settled(
        Status::Completed,
        "done",
        ruled("SHIP-COMPLETE", "ship", Some("done")),
    );
    let feature = "fleet titles\n\nthe whole commission, every line of it";
    let residuals = [finding("max_residual_severity", "medium", false)];
    let view = run_rows(&[RunEntry {
        run_id: "r1",
        feature,
        created_at: DAY_BEFORE,
        last_recorded_at: Some(JUST_UNDER_A_DAY),
        state: Some(&shipped),
        detail: None,
        residuals: &residuals,
    }]);
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(json["view_version"], 28);
    assert_eq!(json["runs"][0]["hire"], Value::Null);
    assert_eq!(json["runs"][0]["acknowledgement"], Value::Null);
    assert_eq!(json["runs"][0]["feature"], feature);
    assert_eq!(json["runs"][0]["title"], "fleet titles");
    assert_eq!(json["runs"][0]["last_recorded_at"], JUST_UNDER_A_DAY);
    assert_eq!(
        json["runs"][0]["verdict"],
        json!({"standing": "shipped", "rule": "SHIP-COMPLETE", "residual": "medium",
               "reason": null, "text": "COMPLETE", "detail": null})
    );
}

// ------------------------------------------------------ what it needs

/// A run listed under `state` with its journal's last event at `last`.
fn silent_since(state: &RunState, last: Option<&str>) -> RunRow {
    let mut entry = crate::tests::listed("quiet", "LANDING PR #420", DAY_BEFORE, Some(state));
    entry.last_recorded_at = last;
    run_rows(&[entry]).runs.remove(0)
}

/// #503 item 3: a running run whose journal has been silent for longer
/// than an attempt's deadline and the margin past it — three hours — is
/// stale and needs the operator; at exactly three hours it is still
/// running. A stale run is never listed as running.
#[test]
fn a_running_run_silent_past_the_deadline_and_its_margin_is_stale() {
    let running = settled(Status::Running, "land", None);
    let at_bound = silent_since(&running, Some("2026-01-01T21:00:00Z"));
    assert_eq!(need(&at_bound, NOW), None);
    let listed = ids(sections(std::slice::from_ref(&at_bound), NOW));
    assert_eq!(listed[1], (Section::Running, vec!["quiet"]));
    let past = silent_since(&running, Some("2026-01-01T20:59:59Z"));
    let stale = Need::Stale {
        silent: "3h00m".to_string(),
    };
    assert_eq!(need(&past, NOW), Some(stale));
    assert_eq!(
        ids(sections(std::slice::from_ref(&past), NOW)),
        [
            (Section::NeedsYou, vec!["quiet"]),
            (Section::Running, vec![]),
            (Section::Recent, vec![]),
            (Section::Older, vec![]),
        ]
    );
    // Days silent, as the operator found one.
    let dead = silent_since(&running, Some("2025-12-28T04:00:00Z"));
    let silent = "116h00m".to_string();
    assert_eq!(need(&dead, NOW), Some(Need::Stale { silent }));
}

/// A run is stale only on evidence: a journal that states no last
/// event, a last event whose time does not read, and a clock that does
/// not read leave a running run running. A finished run needs nothing
/// however long it has been quiet.
#[test]
fn a_run_is_stale_only_when_its_silence_reads() {
    let running = settled(Status::Running, "land", None);
    let long_ago = Some("2025-12-01T00:00:00Z");
    assert_eq!(need(&silent_since(&running, None), NOW), None);
    assert_eq!(need(&silent_since(&running, Some("then")), NOW), None);
    assert_eq!(need(&silent_since(&running, long_ago), "no clock"), None);
    let shipped = settled(Status::Completed, "done", None);
    assert_eq!(need(&silent_since(&shipped, long_ago), NOW), None);
}

/// #503 items 3 and 4: what each need prints on its row and names in
/// the detail, the commands whole.
#[test]
fn each_need_says_what_the_operator_can_do() {
    let stale = Need::Stale {
        silent: "116h00m".to_string(),
    };
    let unfolded = Need::Quarantined(Quarantine::DoesNotFold);
    let unloaded = Need::Quarantined(Quarantine::DoesNotLoad);
    assert_eq!(
        [&Need::Parked, &unfolded, &unloaded, &stale].map(Need::label),
        ["parked", "quarantined", "quarantined", "stale"]
    );
    assert_eq!(Need::Parked.prompt(), None);
    assert_eq!(
        unfolded.prompt().as_deref(),
        Some("quarantined: export and inspect")
    );
    assert_eq!(
        unloaded.prompt().as_deref(),
        Some("quarantined: the journal does not load; inspect it by hand")
    );
    assert_eq!(
        stale.prompt().as_deref(),
        Some("stale: no event since 116h00m; resume or conclude")
    );
    assert_eq!(Need::Parked.way_out("r1"), Vec::<String>::new());
    assert_eq!(Need::Parked.commands("r1"), Vec::<String>::new());
    let acknowledge = "brokkr operator acknowledge --run r1 --reason <why>";
    assert_eq!(
        unfolded.commands("r1"),
        ["brokkr export --run r1", acknowledge]
    );
    // The store refuses a journal it does not load to every verb that
    // reads it, export included (review M1); an acknowledgement reads
    // neither its events nor its fold (#511).
    assert_eq!(unloaded.commands("r1"), [acknowledge]);
    assert_eq!(
        stale.commands("r1"),
        [
            "brokkr resume --run r1 --bundle <its-bundle>",
            "brokkr conclude --run r1 --reason <why>",
        ]
    );
    assert_eq!(
        unfolded.way_out("r1"),
        [
            "way out   brokkr export --run r1",
            "          and inspect the journal it writes",
            "          then brokkr operator acknowledge --run r1 --reason <why> lists it under \
             older",
        ]
    );
    assert_eq!(
        unloaded.way_out("r1"),
        [
            "way out   none in brokkr: the store refuses this journal, export included;",
            "          inspect the hearth's journal file by hand",
            "          then brokkr operator acknowledge --run r1 --reason <why> lists it under \
             older",
        ]
    );
    assert_eq!(
        stale.way_out("r1"),
        [
            "stale     no event since 116h00m, past the 3h bound: an attempt's 2h deadline \
             and a 1h margin",
            "way out   brokkr resume --run r1 --bundle <its-bundle>",
            "          drives it again under its pinned bundle (or --recipe <name>);",
            "          or brokkr conclude --run r1 --reason <why> closes it",
        ]
    );
    let parked = settled(Status::AwaitingOperator, "review", None);
    assert_eq!(need(&silent_since(&parked, None), NOW), Some(Need::Parked));
    let refused = |detail| RunEntry {
        detail,
        ..crate::tests::listed("q", "a run", NOW, None)
    };
    let fold = Quarantine::DoesNotFold.in_words("event 4: refused");
    let load = Quarantine::DoesNotLoad.in_words("hash chain broken at seq 3");
    let needs = [Some(fold), Some(load), None].map(|detail| {
        let view = run_rows(&[refused(detail)]);
        (need(&view.runs[0], NOW), view.runs[0].quarantine)
    });
    // A quarantine that names no refusal is read as the one no verb gets
    // past, so it never names `export` on a journal the store refuses.
    assert_eq!(
        needs,
        [
            (Some(unfolded), Some(Quarantine::DoesNotFold)),
            (Some(unloaded.clone()), Some(Quarantine::DoesNotLoad)),
            (Some(unloaded), Some(Quarantine::DoesNotLoad)),
        ]
    );
}

/// Review M2: the seat at work is derived once. A stale run's fold still
/// holds its effect in flight, and nothing drives it, so none is at work;
/// a live run's seat is the fold's hire.
#[test]
fn a_stale_run_has_no_seat_at_work() {
    let mut running = settled(Status::Running, "review", None);
    running.run = Run::Running(Cursor::EffectInFlight {
        effect_id: "e1".to_string(),
        attempt_id: "a1".to_string(),
        seat: "reviewer".to_string(),
        failed_attempts: 0,
        stop_rides: false,
    });
    let live = silent_since(&running, Some(NOW));
    let hire = Hire {
        seat: "reviewer".to_string(),
        attempt: 1,
    };
    assert_eq!(at_work(&live, NOW), Some(&hire));
    let dead = silent_since(&running, Some("2025-12-27T04:00:00Z"));
    assert_eq!(dead.hire, Some(hire), "the fold still names it");
    assert_eq!(at_work(&dead, NOW), None);
}

/// #503 item 6: a row names the seat its current effect is hired to and
/// which attempt, from the fold's cursor; a run between effects has none.
#[test]
fn a_run_row_names_the_seat_at_work_and_its_attempt() {
    let mut running = settled(Status::Running, "review", None);
    running.run = Run::Running(Cursor::EffectInFlight {
        effect_id: "e1".to_string(),
        attempt_id: "a2".to_string(),
        seat: "reviewer".to_string(),
        failed_attempts: 1,
        stop_rides: false,
    });
    let hire = |seat: &str, attempt| Hire {
        seat: seat.to_string(),
        attempt,
    };
    assert_eq!(silent_since(&running, None).hire, Some(hire("reviewer", 2)));
    running.run = Run::Running(Cursor::ExecuteEffect {
        effect_id: "e1".to_string(),
        seat: "reviewer".to_string(),
        failed_attempts: 0,
    });
    assert_eq!(silent_since(&running, None).hire, Some(hire("reviewer", 1)));
    let json = serde_json::to_value(silent_since(&running, None)).unwrap();
    assert_eq!(json["hire"], json!({"seat": "reviewer", "attempt": 1}));
    for run in [
        Run::Stopped,
        Run::Running(Cursor::RequestEffect),
        Run::Running(Cursor::Stop),
    ] {
        running.run = run;
        assert_eq!(silent_since(&running, None).hire, None);
    }
}

/// #503 item 2: a title takes the columns it is given, clamped at a word.
#[test]
fn a_title_takes_the_columns_it_is_given() {
    let first =
        "#403 macOS fix, round 9: PR #483's test (macos-latest) job fails four protocol tests";
    let feature = format!("{first}\nmore");
    let feature = feature.as_str();
    assert_eq!(title_within(feature, 200), first, "wider than sixty");
    assert_eq!(title_within(feature, 30), "#403 macOS fix, round 9: PR…");
    assert_eq!(title(feature), title_within(feature, TITLE_COLUMNS));
}

/// The summary row keeps a feature's first line cut to 1,024 bytes
/// (decision 0089 ruling 2), and the title is derived from that cut as it
/// is from the whole feature: at every width up to 256 columns, save for
/// a line made mostly of zero-width marks, whose cut keeps only marks.
/// Its removal control is a cut at a byte rather than a character
/// boundary, which fails on the three-byte line.
#[test]
fn a_cut_first_line_titles_as_its_feature() {
    let mebibyte = 1 << 20;
    let words = "a fleet title of ordinary words ".repeat(mebibyte / 32);
    let wide = "€".repeat(mebibyte / 3);
    let features = [
        "#753 a short feature\nwith a second line".to_string(),
        words,
        wide,
        "\n   \n\t\nthe first line that says anything\nand then more".to_string(),
    ];
    for feature in &features {
        let cut = brokkr_core::summary::feature_line(feature);
        for columns in [TITLE_COLUMNS, 256] {
            assert_eq!(
                title_within(cut, columns),
                title_within(feature, columns),
                "{columns} columns of {}",
                feature.chars().take(12).collect::<String>()
            );
        }
    }
    let joiner = char::from_u32(0x200d).expect("the zero-width joiner");
    let marked = format!("{}the words after them", joiner.to_string().repeat(2000));
    let cut = brokkr_core::summary::feature_line(&marked);
    assert_eq!(
        title_within(cut, TITLE_COLUMNS),
        joiner.to_string().repeat(341)
    );
    assert_eq!(title_within(&marked, TITLE_COLUMNS), marked);
}
