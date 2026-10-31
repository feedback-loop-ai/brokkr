//! The fleet a page at a time (decision 0089 ruling 6): a summary row
//! lists as its run read whole, each section reads the ranges it places
//! rows in, and a page continues where its cursor ended.

use super::*;
use crate::fleet::run_row;
use crate::tests::envelope_builder::EnvelopeBuilder;
use crate::{residual_findings, Refusal, RunEntry};
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::fold;
use brokkr_core::summary::{Account, Listing, Summary};
use serde_json::{json, Value};

const FEATURE: &str = "\n  Fix the flaky gate\nwith the detail below";
const CREATED: &str = "2026-01-01T00:00:00Z";
const NOW: &str = "2026-01-02T00:00:00Z";

/// `steps` as one run's journal, sealed, a minute apart past the hour.
fn journal(steps: &[(EventType, Value)]) -> Vec<EventEnvelope> {
    let stamped = |(seq, (kind, payload)): (u64, &(EventType, Value))| {
        let builder = EnvelopeBuilder::new(*kind, payload.clone()).seq(seq);
        let at = format!("2026-01-01T01:{seq:02}:00Z");
        builder.event_id(format!("v{seq}")).at(at).sealed()
    };
    (1..).zip(steps).map(stamped).collect()
}

/// A run verified into a park, its one residual finding superseded from
/// another realm.
fn parked() -> Vec<EventEnvelope> {
    let held = json!({"from": "verify", "result": "hold", "rule_id": "VERIFY-HOLD",
        "problem": "unproven", "inputs": {"has_security_residual": true}});
    let superseded = json!({"command_id": "k1", "command": "supersede",
        "operator": "an operator", "args": {"findings": [6], "reason": "proven there",
        "by": {"realm": "elsewhere", "run_id": "fixed", "seq": 9}}});
    journal(&[
        (
            EventType::RunStarted,
            json!({"feature": "v", "manifest": {}}),
        ),
        (EventType::PhaseEntered, json!({"phase": "verify"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "v1", "seat": "verifier"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "v1", "attempt_id": "t1"}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "v1", "result": {"result": "hold"}}),
        ),
        (EventType::TransitionDecided, held),
        (
            EventType::RunParked,
            json!({"reason": "VERIFY-HOLD: unproven"}),
        ),
        (EventType::OperatorCommanded, superseded),
    ])
}

/// The row a store reads back for `summary`, as a page reads it.
fn row_of(summary: &Summary) -> Row {
    let cut = |bounded: &Bounded| Bounded {
        cut: bounded.cut.clone(),
        whole: None,
    };
    Row {
        run_id: "run".to_string(),
        feature_line: summary.feature_line.clone(),
        created_at: cut(&summary.created_at),
        last_recorded_at: summary.last_recorded_at.as_ref().map(cut),
        order_at: summary.last_at_ms.unwrap_or(i64::MAX),
        status: summary.status,
        stop_accepted_seq: summary.stop_accepted_seq,
        phase: summary.phase.as_ref().map(cut),
        park_reason: summary.park_reason.as_ref().map(cut),
        ruled: summary.ruled.clone(),
        hired: summary.hired.clone(),
        residual_open: summary.residual_open,
        refused: summary.refused.clone(),
        folded: (summary.folded_seq, summary.folded_hash.clone()),
        acknowledged: None,
    }
}

/// What the row keeps whole, as a store reads it by the run's key.
fn whole_of(summary: &Summary, acknowledged: Option<Copied>) -> Whole {
    Whole {
        feature: summary.feature.clone(),
        residuals: serde_json::from_str(&summary.residuals).unwrap(),
        acknowledged,
    }
}

/// `events` read whole, as the fleet reads a run today, and its summary.
fn both(events: &[EventEnvelope]) -> (RunRow, Summary) {
    let listing = Listing {
        feature: FEATURE,
        created_at: CREATED,
    };
    let summary = Summary::of(&Account::whole("run", events), listing);
    let state = fold(events);
    let residuals = residual_findings("run", events);
    let words = state.as_ref().err().map(ToString::to_string);
    let acknowledgement = Acknowledgement {
        seq: summary.folded_seq,
        event_hash: summary.folded_hash.clone(),
        operator: "op".to_string(),
        reason: "looked".to_string(),
        recorded_at: CREATED.to_string(),
    };
    let entry = RunEntry {
        run_id: "run",
        feature: FEATURE,
        created_at: CREATED,
        last_recorded_at: events.last().map(|event| event.recorded_at.as_str()),
        state: state.as_ref().ok(),
        detail: words.as_deref().map(|words| Refusal {
            acknowledgement: Some(&acknowledgement),
            ..Quarantine::DoesNotFold.in_words(words)
        }),
        residuals: &residuals,
    };
    (run_row(&entry), summary)
}

fn wire(row: &RunRow) -> Value {
    serde_json::to_value(row).unwrap()
}

/// Ruling 2: a summary row read with its whole values is the run read
/// whole, every field, its place included.
#[test]
fn a_summary_row_read_whole_is_its_run_read_whole() {
    let (whole, summary) = both(&parked());
    let paged = summary_row(&row_of(&summary), Some(&whole_of(&summary, None)));
    assert_eq!(wire(&paged), wire(&whole));
    assert_eq!(paged.order_at, whole.order_at);
    assert_eq!(paged.residuals.len(), 1);
    assert_eq!(paged.verdict.text, "HOLD");
    let refused = journal(&[(EventType::PhaseEntered, json!({"phase": "review"}))]);
    let (whole, summary) = both(&refused);
    let copied = Copied {
        operator: "op".to_string(),
        reason: "looked".to_string(),
        at: CREATED.to_string(),
    };
    let paged = summary_row(&row_of(&summary), Some(&whole_of(&summary, Some(copied))));
    assert_eq!(wire(&paged), wire(&whole));
    assert_eq!(paged.quarantine, Some(Quarantine::DoesNotFold));
}

/// A page lists a row within its bounds: the feature as its first line,
/// each value as its bound left it, the acknowledgement's copy, and no
/// finding; its title and verdict are the whole read's.
#[test]
fn a_paged_row_lists_its_bounded_values() {
    let (whole, summary) = both(&parked());
    let mut row = row_of(&summary);
    row.phase = Some(Bounded {
        cut: "rev".to_string(),
        whole: None,
    });
    let paged = summary_row(&row, None);
    assert_eq!(paged.feature, "Fix the flaky gate");
    assert_eq!(
        (paged.title.as_str(), paged.phase.as_deref()),
        (whole.title.as_str(), Some("rev"))
    );
    assert_eq!(paged.verdict, whole.verdict);
    assert!(paged.residuals.is_empty());
    let refused = journal(&[(EventType::PhaseEntered, json!({"phase": "review"}))]);
    let (_, summary) = both(&refused);
    let mut row = row_of(&summary);
    row.acknowledged = Some(Copied {
        operator: "o".to_string(),
        reason: "r".to_string(),
        at: "t".to_string(),
    });
    let paged = summary_row(&row, None);
    let acknowledged = paged.acknowledgement.unwrap();
    assert_eq!(acknowledged.line(), "acknowledged by o at t, event 1: r");
    assert_eq!((paged.seq, paged.status_known), (None, false));
}

/// Each section reads the lanes, at the bounds, it places a row in; a
/// clock that does not read bounds nothing.
#[test]
fn each_section_reads_the_ranges_it_places_rows_in() {
    let now = brokkr_core::stamp::parse_millis(NOW).unwrap();
    let (stale, day) = (now - 3 * 3_600_000, now - 86_400_000);
    let range = |lane, bound| Range { lane, bound };
    let ruled = [
        (
            Section::NeedsYou,
            vec![
                range(Lane::Refused, None),
                range(Lane::Parked, None),
                range(Lane::Stopping, None),
                range(Lane::Running, Some(Bound::Below(stale))),
            ],
        ),
        (
            Section::Running,
            vec![range(Lane::Running, Some(Bound::AtOrAbove(stale)))],
        ),
        (
            Section::Recent,
            vec![range(Lane::Finished, Some(Bound::Above(day)))],
        ),
        (
            Section::Older,
            vec![
                range(Lane::Finished, Some(Bound::AtOrBelow(day))),
                range(Lane::Acknowledged, None),
            ],
        ),
    ];
    for (section, ranges) in ruled {
        assert_eq!(super::ranges(section, NOW), ranges, "{section:?}");
    }
    let unbounded = [
        (
            Section::NeedsYou,
            vec![
                range(Lane::Refused, None),
                range(Lane::Parked, None),
                range(Lane::Stopping, None),
            ],
        ),
        (Section::Running, vec![range(Lane::Running, None)]),
        (Section::Recent, vec![range(Lane::Finished, None)]),
        (Section::Older, vec![range(Lane::Acknowledged, None)]),
    ];
    for (section, ranges) in unbounded {
        assert_eq!(super::ranges(section, "no clock"), ranges, "{section:?}");
    }
}

/// A cursor reads back as the cursor that wrote it, a run id with colons
/// whole and the empty id a writer admits, and text that names no place
/// is refused in its own words.
#[test]
fn a_cursor_reads_back_and_refuses_what_names_no_place() {
    for (section, run_id) in Section::ORDER.into_iter().zip(["a:b", "", "a\0b", "r"]) {
        let cursor = Cursor {
            section,
            order_at: -5,
            run_id: run_id.to_string(),
        };
        let text = cursor.to_string();
        assert_eq!(text.parse::<Cursor>(), Ok(cursor.clone()));
        assert_eq!(serde_json::to_value(&cursor).unwrap(), json!(text));
    }
    let refused = [
        (
            "older:5",
            "cursor 'older:5' is not <section>:<order>:<run id>, as a page's `next` states it",
        ),
        (
            "later:5:r",
            "cursor section 'later' is none of needs-you, running, recent or older",
        ),
        (
            "older:soon:r",
            "cursor order 'soon' is not a whole number of milliseconds",
        ),
    ];
    for (text, words) in refused {
        assert_eq!(text.parse::<Cursor>().unwrap_err().to_string(), words);
    }
}

/// A row of `section` at `order_at` named `run_id`.
fn listed(run_id: &str, order_at: i64) -> RunRow {
    let (mut row, _) = both(&parked());
    row.run_id = run_id.to_string();
    row.order_at = order_at;
    row
}

/// Up to `wanted` of `fleet`'s rows of `section` below `after`, as a
/// page's read hands them.
fn read_held(
    fleet: &[(Section, Vec<RunRow>)],
    section: Section,
    after: Option<Place<'_>>,
    wanted: usize,
) -> Result<Vec<RunRow>, ()> {
    let rows = fleet.iter().find(|(held, _)| *held == section);
    let rows = rows.map_or_else(Vec::new, |(_, rows)| rows.clone());
    let below = rows
        .into_iter()
        .filter(|row| after.is_none_or(|after| row.place() < after));
    Ok(below.take(wanted).collect())
}

/// What `page` lists, by section and id.
fn ids(page: &Page) -> Vec<(Section, String)> {
    page.rows
        .iter()
        .map(|(section, row)| (*section, row.run_id.clone()))
        .collect()
}

/// A page fills from the sections in order, one row past it says there
/// is a next, and the page after its cursor carries on in the cursor's
/// section, below its place.
#[test]
fn a_page_carries_on_where_its_cursor_ended() {
    let fleet = [
        (Section::NeedsYou, vec![listed("a", 9), listed("b", 5)]),
        (Section::Recent, vec![listed("c", 7)]),
    ];
    let read = |section: Section, after: Option<Place<'_>>, wanted: usize| {
        read_held(&fleet, section, after, wanted)
    };
    let first = page(None, (1, NOW), read).unwrap();
    assert_eq!(ids(&first), [(Section::NeedsYou, "a".to_string())]);
    let next = first.next.unwrap();
    assert_eq!(next.to_string(), "needs-you:9:a");
    let second = page(Some(&next), (2, NOW), read).unwrap();
    let expected = [
        (Section::NeedsYou, "b".to_string()),
        (Section::Recent, "c".to_string()),
    ];
    assert_eq!(ids(&second), expected);
    assert_eq!(second.next, None);
    let empty = page(None, (0, NOW), |_, _, _| Ok::<_, ()>(Vec::new())).unwrap();
    assert_eq!((empty.rows.len(), empty.next), (0, None));
    assert_eq!(
        page(None, (5, NOW), |_, _, _| Err::<Vec<RunRow>, _>("refused")).err(),
        Some("refused")
    );
}

/// Ruling 5: a row a page read whole lists in the section its fold
/// places it in, at its own key, on whichever page read it, past the
/// first page as on it; the page's `next` is where it was read, as is a
/// cut id's, its whole id; and a walk a row a page lists every row once.
#[test]
fn a_row_read_whole_lists_by_its_fold_on_every_page() {
    let mut whole = listed("w", 8);
    whole.paged_at = Some((7, "w".to_string()));
    let mut cut = listed("x…", 6);
    cut.paged_at = Some((6, "x".repeat(200)));
    let fleet = [
        (Section::NeedsYou, vec![listed("a", 9)]),
        (Section::Recent, vec![whole, cut, listed("c", 5)]),
    ];
    let read = |section: Section, after: Option<Place<'_>>, wanted: usize| {
        read_held(&fleet, section, after, wanted)
    };
    let first = page(None, (2, NOW), read).unwrap();
    let placed = [
        (Section::NeedsYou, "a".to_string()),
        (Section::NeedsYou, "w".to_string()),
    ];
    assert_eq!(ids(&first), placed);
    let next = first.next.unwrap();
    assert_eq!(next.to_string(), "recent:7:w");
    let second = page(Some(&next), (1, NOW), read).unwrap();
    assert_eq!(ids(&second), [(Section::NeedsYou, "x…".to_string())]);
    // Placed, a row read whole lists at its own key, a cut id at its whole.
    let long = "x".repeat(200);
    assert_eq!(first.rows[1].1.place(), (8, "w"));
    assert_eq!(second.rows[0].1.place(), (6, long.as_str()));
    let past = second.next.unwrap();
    assert_eq!(past.to_string(), format!("recent:6:{}", "x".repeat(200)));
    let begun = Cursor {
        section: Section::Recent,
        order_at: 100,
        run_id: "z".to_string(),
    };
    let late = page(Some(&begun), (5, NOW), read).unwrap();
    let by_fold = [
        (Section::NeedsYou, "w".to_string()),
        (Section::NeedsYou, "x…".to_string()),
        (Section::Recent, "c".to_string()),
    ];
    assert_eq!(ids(&late), by_fold);
    let mut walked = Vec::new();
    let mut after = None;
    loop {
        let one = page(after.as_ref(), (1, NOW), read).unwrap();
        walked.extend(ids(&one));
        match one.next {
            Some(next) => after = Some(next),
            None => break,
        }
    }
    let every = [(Section::NeedsYou, "a".to_string())].into_iter();
    assert_eq!(walked, every.chain(by_fold).collect::<Vec<_>>());
}

/// A fleet read whole pages as the summary does: its rows of one
/// section, in order, below the cursor.
#[test]
fn a_fleet_read_whole_lists_a_section_below_its_cursor() {
    let rows = [listed("a", i64::MAX), listed("b", 20), listed("c", 10)];
    let ids = |rows: Vec<RunRow>| rows.into_iter().map(|row| row.run_id).collect::<Vec<_>>();
    let every: &dyn Fn(&RunRow) -> bool = &|_| true;
    assert_eq!(
        ids(section_of_rows(
            &rows,
            Section::NeedsYou,
            NOW,
            None,
            (2, every)
        )),
        ["a", "b"]
    );
    let after = Some((20, "b"));
    assert_eq!(
        ids(section_of_rows(
            &rows,
            Section::NeedsYou,
            NOW,
            after,
            (5, every)
        )),
        ["c"]
    );
    assert!(section_of_rows(&rows, Section::Older, NOW, None, (5, every)).is_empty());
    let not_b: &dyn Fn(&RunRow) -> bool = &|row| row.run_id != "b";
    assert_eq!(
        ids(section_of_rows(
            &rows,
            Section::NeedsYou,
            NOW,
            None,
            (2, not_b)
        )),
        ["a", "c"]
    );
}
