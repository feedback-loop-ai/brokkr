use serde_json::json;

use super::*;
use crate::envelope::{EventEnvelope, EventType};
use crate::seat::row::tests::journals::{self, at, checkpoint, journal};
use crate::seat::row::{facts, SeatFacts};
use crate::summary::Account;

/// Each JSON shape a record states a value in, shown as JavaScript's
/// string conversion shows it.
#[test]
fn a_value_is_shown_as_javascript_converts_it_to_a_string() {
    let shapes = [
        (None, "undefined"),
        (Some(json!(null)), "null"),
        (Some(json!(false)), "false"),
        (Some(json!("Read")), "Read"),
        (Some(json!(7)), "7"),
        (Some(json!(2.0)), "2"),
        (Some(json!(0.25)), "0.25"),
        (Some(json!([null, 1, "b", [2, null]])), ",1,b,2,"),
        (Some(json!({"tool": "Read"})), "[object Object]"),
    ];
    for (value, expected) in shapes {
        assert_eq!(shown(value.as_ref()), expected, "{value:?}");
    }
}

/// A `seat-turn`'s target is kept only where it says something as text.
#[test]
fn a_turn_keeps_its_target_only_where_it_names_one() {
    let target = |target: serde_json::Value| {
        Turned::of(&json!({"turn": 2, "tool": "Edit", "target": target})).target
    };
    assert_eq!(target(json!("src/a.rs")), Some("src/a.rs".to_string()));
    assert_eq!(target(json!("")), None);
    assert_eq!(target(json!(3)), None);
    assert_eq!(
        Turned::of(&json!({})),
        Turned {
            turn: "undefined".to_string(),
            tool: "undefined".to_string(),
            target: None,
        }
    );
}

/// A finish keeps its outcome as shown, an absent one `undefined` and a
/// JSON null as none, and pins only a word the view lets stand.
#[test]
fn a_finish_keeps_its_outcome_as_shown_and_a_null_as_none() {
    let finished = |outcome: Option<serde_json::Value>| {
        let mut record = json!({"step": "panel-member-finished"});
        if let Some(outcome) = outcome {
            record["outcome"] = outcome;
        }
        finished(&record, "t")
    };
    let finish = |outcome: Option<&str>| Finish {
        at: "t".to_string(),
        outcome: outcome.map(str::to_string),
    };
    assert_eq!(
        [
            finished(Some(json!("failed"))),
            finished(Some(json!("pass"))),
            finished(Some(json!(null))),
            finished(None),
            finished(Some(json!(["failed"]))),
        ],
        [
            (
                Pin::Pinned(crate::seat::row::Outcome::Failed),
                finish(Some("failed"))
            ),
            (Pin::Unpinned, finish(Some("pass"))),
            (Pin::Unpinned, finish(None)),
            (Pin::Unpinned, finish(Some("undefined"))),
            (Pin::Unpinned, finish(Some("failed"))),
        ]
    );
}

/// A seat's request, its start under `attempt`, and its checkpoints.
fn panel(attempt: &str, records: Vec<serde_json::Value>) -> Vec<(EventType, serde_json::Value)> {
    let mut events = vec![(
        EventType::EffectStarted,
        json!({"effect_id": "p", "attempt_id": attempt}),
    )];
    events.extend(
        records
            .into_iter()
            .map(|record| checkpoint("p", attempt, record)),
    );
    events
}

fn asked() -> (EventType, serde_json::Value) {
    (
        EventType::EffectRequested,
        json!({"effect_id": "p", "seat": "panel"}),
    )
}

fn turn(member: Option<&str>, turn: u64, tool: &str) -> serde_json::Value {
    json!({"member": member, "step": "seat-turn", "turn": turn, "tool": tool})
}

fn live_of(events: &[EventEnvelope]) -> Vec<(String, Option<(String, String)>)> {
    live(Account::whole("run", events).seats())
}

fn named(member: &str) -> Member {
    Member::Named(member.to_string())
}

fn turned(turn: &str, tool: &str) -> Option<(String, String)> {
    Some((turn.to_string(), tool.to_string()))
}

/// A live line labelled `label`, with `turn`.
fn line(label: &str, turn: Option<(String, String)>) -> (String, Option<(String, String)>) {
    (label.to_string(), turn)
}

/// The live block reads the attempt in flight: its participants in the
/// order of their first checkpoint in it, an earlier attempt's order
/// passed over, each with its last `seat-turn` in it, a later record of
/// another step passing none over, and none for one with no turn in it
/// though an earlier attempt had one.
#[test]
fn the_live_block_reads_the_attempt_in_flight_in_its_checkpoints_order() {
    let mut events = vec![asked()];
    events.extend(panel(
        "one",
        vec![turn(Some("a"), 1, "Read"), turn(Some("b"), 1, "Edit")],
    ));
    events.push((
        EventType::EffectFailed,
        json!({"effect_id": "p", "attempt_id": "one"}),
    ));
    events.extend(panel(
        "two",
        vec![
            json!({"member": "b", "step": "session-started"}),
            turn(None, 4, "Bash"),
            turn(Some("a"), 2, "Grep"),
            json!({"member": "a", "step": "item-completed"}),
        ],
    ));
    let events = journal(events);
    assert_eq!(
        live_of(&events),
        [
            line("panel:b", None),
            line("panel", turned("4", "Bash")),
            line("panel:a", turned("2", "Grep")),
        ]
    );
}

/// No attempt in flight reads no live line: none started, or the one
/// started concluded; the one in flight with no checkpoint yet reads its
/// seat's bare line, and a start re-opening a concluded attempt puts it in
/// flight again.
#[test]
fn no_attempt_in_flight_reads_no_live_line() {
    let mut concluded = vec![asked()];
    concluded.extend(panel("one", vec![turn(None, 1, "Read")]));
    concluded.push((
        EventType::EffectSucceeded,
        json!({"effect_id": "p", "attempt_id": "one"}),
    ));
    let mut reopened = concluded.clone();
    reopened.push(panel("one", vec![]).remove(0));
    let mut quiet = vec![asked()];
    quiet.extend(panel("one", vec![]));
    assert_eq!(live_of(&journal(concluded)), []);
    assert_eq!(live_of(&journal(vec![asked()])), []);
    assert_eq!(live_of(&journal(quiet)), [line("panel", None)]);
    let reopened = live_of(&journal(reopened));
    assert_eq!(reopened, [line("panel", turned("1", "Read"))]);
}

/// A start before its effect's request is live once the effect is asked:
/// the records before the request are listed first under the seat `?`,
/// each line with its last turn; a later record under the seat its
/// request names; one after the effect is asked again under that
/// request's seat; and a terminal naming the start's effect and attempt
/// ends it, though no request's attempt in flight is it.
#[test]
fn a_start_before_its_request_is_live_with_the_records_before_it() {
    let early = [
        line("?", None),
        line("?:review:a", turned("2", "undefined")),
    ];
    let mut lines = early.to_vec();
    lines.push(line("work", None));
    assert_eq!(live_of(&journals::in_flight(false)), lines);
    lines.push(line("check:review:a", turned("3", "Read")));
    assert_eq!(live_of(&journals::in_flight(true)), lines);
    assert_eq!(live_of(&journals::unasked()), []);
    assert_eq!(live_of(&journal(journals::unasked_events(true))), []);
    let quiet = live_of(&journals::started_unasked());
    assert_eq!(quiet, [line("quiet", None)]);
}

/// A request naming no seat, or an empty one, labels its records `?`, as a
/// record before any request is: one line under both, its turn the last
/// stated under either, and none passed over for a later record with none.
#[test]
fn a_record_under_no_seat_shares_the_line_of_one_before_the_request() {
    let unseated = |seat: serde_json::Value, last: serde_json::Value| {
        let mut events = vec![checkpoint("p", "one", turn(Some("a"), 1, "Read"))];
        events.extend(panel("one", vec![]));
        let asked = json!({"effect_id": "p", "seat": seat});
        events.push((EventType::EffectRequested, asked));
        events.push(checkpoint("p", "one", last));
        live_of(&journal(events))
    };
    let completed = json!({"member": "a", "step": "item-completed"});
    let kept = [line("?:a", turned("1", "Read"))];
    assert_eq!(unseated(json!(null), completed.clone()), kept);
    assert_eq!(unseated(json!(""), completed), kept);
    let later = [line("?:a", turned("2", "Grep"))];
    assert_eq!(unseated(json!(""), turn(Some("a"), 2, "Grep")), later);
}

/// What a participant's facts carry of its records: its last checkpoint's
/// time, its last `seat-turn` over every attempt, a member's last finish
/// and its last session record's session.
#[test]
fn a_participant_s_facts_carry_its_last_turn_finish_and_session() {
    let mut events = vec![asked()];
    events.extend(panel(
        "one",
        vec![
            json!({"member": "a", "step": "session-started", "session_id": "s-1"}),
            json!({"member": "a", "step": "seat-turn", "turn": 3, "tool": "Edit", "target": "src/a.rs"}),
            json!({"member": "a", "step": "panel-member-finished", "outcome": null}),
            json!({"member": "a", "step": "item-completed"}),
        ],
    ));
    let events = journal(events);
    let found: Vec<SeatFacts> = facts(Account::whole("run", &events).seats());
    let member = &found[1];
    assert_eq!(member.member, named("a"));
    assert_eq!(
        (
            member.last_at.as_deref(),
            member.turn.as_deref(),
            member.tool.as_deref(),
            member.target.as_deref(),
            member.finish.as_ref(),
            member.session_id.as_deref(),
        ),
        (
            Some(at(6).as_str()),
            Some("3"),
            Some("Edit"),
            Some("src/a.rs"),
            Some(&Finish {
                at: at(5),
                outcome: None,
            }),
            Some("s-1"),
        )
    );
}

/// The rows keep the live block unless the live start, unended, names no
/// effect or one not yet asked: a request of its effect after it, a
/// terminal naming its effect and attempt, or a later start of an asked
/// effect keeps it again.
#[test]
fn a_live_start_no_row_is_of_leaves_the_rows_unkept() {
    let started = |payload: serde_json::Value| (EventType::EffectStarted, payload);
    let ended = |payload: serde_json::Value| (EventType::EffectSucceeded, payload);
    let one = json!({"effect_id": "p", "attempt_id": "one"});
    let journals = [
        vec![],
        vec![asked(), started(one.clone())],
        vec![started(one.clone())],
        vec![started(one.clone()), asked()],
        vec![started(one.clone()), ended(one.clone())],
        vec![started(json!({}))],
        vec![started(json!({})), ended(json!({}))],
        vec![started(json!({})), asked(), started(one.clone())],
        vec![asked(), started(one), started(json!({"effect_id": "q"}))],
    ];
    let kept: Vec<bool> = journals
        .into_iter()
        .map(|events| Account::whole("run", &journal(events)).seats_kept())
        .collect();
    assert_eq!(
        kept,
        [true, true, false, true, true, false, true, true, false]
    );
}
