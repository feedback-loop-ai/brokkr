//! The dashboard's derivations (#503): the path a run took, the ruling it
//! last earned, its last seat's and its last review's notes, the
//! checkpoints of the seat at work.

use super::*;
use crate::run_view;
use crate::tests::ev;
use serde_json::json;

/// A journal under construction, its events numbered as they are added.
#[derive(Default)]
struct Journal(Vec<EventEnvelope>);

impl Journal {
    fn push(&mut self, kind: EventType, payload: Value, at: &str) -> &mut Journal {
        let seq = self.0.len() as u64 + 1;
        self.0.push(ev(seq, kind, payload, at));
        self
    }

    fn enter(&mut self, phase: &str, at: &str) -> &mut Journal {
        self.push(EventType::PhaseEntered, json!({"phase": phase}), at)
    }

    /// `seat` hired in `phase` as `effect`, its attempt `a1` started.
    fn hire(&mut self, effect: &str, seat: &str, phase: &str, at: &str) -> &mut Journal {
        let request = json!({"effect_id": effect, "seat": seat, "phase": phase});
        self.push(EventType::EffectRequested, request, at);
        let started = json!({"effect_id": effect, "attempt_id": "a1"});
        self.push(EventType::EffectStarted, started, at)
    }

    /// `effect`'s attempt `attempt` ends as `kind`, its result `result`.
    fn end(
        &mut self,
        kind: EventType,
        effect: &str,
        attempt: &str,
        result: Value,
        at: &str,
    ) -> &mut Journal {
        let payload = json!({"effect_id": effect, "attempt_id": attempt, "result": result});
        self.push(kind, payload, at)
    }

    fn decide(&mut self, ruling: Value, at: &str) -> &mut Journal {
        self.push(EventType::TransitionDecided, ruling, at)
    }
}

/// A run through intake, implement and a review that finds a medium
/// residual, stopped by a regression that fails.
fn reviewed() -> Vec<EventEnvelope> {
    let mut journal = Journal::default();
    journal
        .push(
            EventType::RunStarted,
            json!({"feature": "a run"}),
            "2026-01-01T00:00:00Z",
        )
        .enter("intake", "2026-01-01T00:00:00Z")
        .hire("e1", "intake", "intake", "2026-01-01T00:00:00Z")
        .end(
            EventType::EffectSucceeded,
            "e1",
            "a1",
            json!({"result": "intook", "notes": "read the issue", "model": "claude-opus-5-5"}),
            "2026-01-01T00:02:03Z",
        )
        .decide(
            json!({"rule_id": "INTAKE-OK", "severity": "normal", "from": "intake",
                   "next": "implement", "result": "intook"}),
            "2026-01-01T00:02:03Z",
        )
        .enter("implement", "2026-01-01T00:02:03Z")
        .hire("e2", "implementer", "implement", "2026-01-01T00:02:03Z")
        .push(
            EventType::EffectCheckpointed,
            json!({"effect_id": "e2", "attempt_id": "a1",
                   "checkpoint": {"step": "seat-turn", "model": "gpt-6.1-sol"}}),
            "2026-01-01T00:10:00Z",
        )
        .end(
            EventType::EffectSucceeded,
            "e2",
            "a1",
            json!({"result": "complete"}),
            "2026-01-01T00:44:03Z",
        )
        .decide(
            json!({"rule_id": "IMPL-OK", "severity": "normal", "from": "implement",
                   "next": "review", "result": "complete"}),
            "2026-01-01T00:44:05Z",
        )
        .enter("review", "2026-01-01T00:44:05Z")
        .hire("e3", "reviewer", "review", "2026-01-01T00:44:05Z")
        .end(
            EventType::EffectSucceeded,
            "e3",
            "a1",
            json!({"result": "residual", "model": "claude-fable-5-1", "notes": {
                "members": {"correctness": "one branch untested", "security": "clean"},
                "verdicts": {"correctness": "residual", "security": "clean"}}}),
            "2026-01-01T00:59:08Z",
        )
        .decide(
            json!({"rule_id": "REVIEW-RESIDUAL-OK", "severity": "flagged", "from": "review",
                   "next": "regression", "result": "residual",
                   "inputs": {"max_residual_severity": "medium"}}),
            "2026-01-01T00:59:08Z",
        )
        .enter("regression", "2026-01-01T00:59:08Z")
        .hire("e4", "regression", "regression", "2026-01-01T00:59:08Z")
        .end(
            EventType::EffectFailed,
            "e4",
            "a1",
            json!({"result": "fail"}),
            "2026-01-01T01:00:00Z",
        )
        .decide(
            json!({"rule_id": "REGRESSION-FAIL", "severity": "hard", "from": "regression",
                   "next": "stop", "result": "fail", "problem": "two tests fail"}),
            "2026-01-01T01:00:00Z",
        );
    journal.0
}

#[test]
fn the_path_names_every_visit_its_ruling_residual_duration_and_model() {
    let board = dashboard(&reviewed());
    assert_eq!(board.run_id.as_deref(), Some("r1"));
    let visits: Vec<_> = board
        .path
        .iter()
        .map(|visit| {
            (
                visit.phase.as_str(),
                visit.ruled,
                visit.residual,
                visit.duration.as_deref(),
                visit.model.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        visits,
        [
            (
                "intake",
                Ruled::Passed,
                None,
                Some("2m03s"),
                Some("claude-opus-5-5")
            ),
            (
                "implement",
                Ruled::Passed,
                None,
                Some("42m02s"),
                Some("gpt-6.1-sol")
            ),
            (
                "review",
                Ruled::Flagged,
                Some(Severity::Medium),
                Some("15m03s"),
                Some("claude-fable-5-1")
            ),
            ("regression", Ruled::Stopped, None, Some("52s"), None),
        ]
    );
    let rules: Vec<_> = board
        .path
        .iter()
        .map(|visit| visit.rule.as_deref())
        .collect();
    assert_eq!(
        rules,
        [
            Some("INTAKE-OK"),
            Some("IMPL-OK"),
            Some("REVIEW-RESIDUAL-OK"),
            Some("REGRESSION-FAIL")
        ]
    );
    assert_eq!(
        board.decision,
        Some(Decision {
            rule: "REGRESSION-FAIL".to_string(),
            severity: Some(RuleSeverity::Hard),
            from: Some("regression".to_string()),
            next: Some("stop".to_string()),
            result: Some("fail".to_string()),
            residual: None,
            problem: Some("two tests fail".to_string()),
        })
    );
    let severities = [
        RuleSeverity::Normal,
        RuleSeverity::Flagged,
        RuleSeverity::Hard,
    ];
    assert_eq!(
        severities.map(RuleSeverity::label),
        ["normal", "flagged", "hard"]
    );
}

#[test]
fn the_last_seat_and_the_last_review_report_their_notes_whole() {
    let board = dashboard(&reviewed());
    let review = Concluded {
        seat: "reviewer".to_string(),
        phase: Some("review".to_string()),
        outcome: Outcome::Succeeded,
        result: Some("residual".to_string()),
        notes: Some(
            "correctness (residual): one branch untested\nsecurity (clean): clean".to_string(),
        ),
    };
    assert_eq!(board.last_review, Some(review));
    let last = board.last_seat.unwrap();
    assert_eq!(
        (
            last.seat.as_str(),
            last.outcome.label(),
            last.result.as_deref(),
            last.notes
        ),
        ("regression", "failed", Some("fail"), None)
    );
    assert_eq!(Outcome::Succeeded.label(), "succeeded");
    // An attempt whose outcome the engine could not tell concludes so.
    let mut journal = Journal::default();
    journal.enter("implement", "2026-01-01T00:00:00Z").hire(
        "e1",
        "implementer",
        "implement",
        "2026-01-01T00:00:00Z",
    );
    let lost = json!({"result": "complete", "notes": "the session ended unread"});
    journal.end(
        EventType::EffectIndeterminate,
        "e1",
        "a1",
        lost,
        "2026-01-01T00:00:09Z",
    );
    let last = dashboard(&journal.0).last_seat.unwrap();
    assert_eq!(
        (last.outcome, last.outcome.label()),
        (Outcome::Indeterminate, "indeterminate")
    );
}

/// A ruling to no phase parks, one back to a visited phase returns, a
/// visit nobody ruled is open, and a ruling's word outside phase-event/v1
/// is no severity at all.
#[test]
fn each_ruling_marks_its_visit() {
    let mut journal = Journal::default();
    journal
        .decide(
            json!({"rule_id": "BEFORE-ANY-PHASE"}),
            "2026-01-01T00:00:00Z",
        )
        .enter("implement", "2026-01-01T00:00:00Z")
        .decide(
            json!({"rule_id": "IMPL-BROKEN-RETRY", "severity": null, "from": "implement",
                   "next": "implement"}),
            "2026-01-01T00:00:05Z",
        )
        .enter("implement", "2026-01-01T00:00:05Z")
        .decide(
            json!({"rule_id": "IMPL-PARK", "from": "implement", "next": null}),
            "2026-01-01T00:00:09Z",
        )
        .enter("review", "2026-01-01T00:00:09Z")
        .decide(
            json!({"rule_id": "REVIEW-CLEAN", "severity": "loud", "from": "review",
                   "next": "regression"}),
            "2026-01-01T00:00:10Z",
        )
        .enter("regression", "2026-01-01T00:00:10Z");
    let board = dashboard(&journal.0);
    let ruled: Vec<Ruled> = board.path.iter().map(|visit| visit.ruled).collect();
    assert_eq!(
        ruled,
        [Ruled::Returned, Ruled::Parked, Ruled::Passed, Ruled::Open]
    );
    assert_eq!(board.decision.unwrap().severity, None);
    assert_eq!(board.path[3].duration.as_deref(), Some("0s"));
}

/// A terminal for an attempt that is not the open one says nothing; a
/// failed attempt with no notes reports its error; empty notes are none;
/// notes of no panel's shape print as written; and an event before any
/// phase, or a terminal for an effect never requested, places nothing.
#[test]
fn a_concluded_attempt_reports_what_it_wrote_and_a_stale_one_nothing() {
    let mut journal = Journal::default();
    journal
        .push(
            EventType::EffectStarted,
            json!({"effect_id": "ghost", "attempt_id": "a1"}),
            "2026-01-01T00:00:00Z",
        )
        .end(
            EventType::EffectSucceeded,
            "ghost",
            "a1",
            json!({"result": "pass"}),
            "2026-01-01T00:00:00Z",
        )
        .push(
            EventType::EffectRequested,
            json!({"effect_id": "early"}),
            "2026-01-01T00:00:00Z",
        )
        .push(
            EventType::EffectCheckpointed,
            json!({"effect_id": "early",
              "checkpoint": {"model": "m0"}}),
            "2026-01-01T00:00:00Z",
        )
        .push(
            EventType::EffectStarted,
            json!({"effect_id": "early"}),
            "2026-01-01T00:00:00Z",
        )
        .push(
            EventType::RunParked,
            json!({"effect_id": "early"}),
            "2026-01-01T00:00:00Z",
        )
        .enter("verify", "2026-01-01T00:00:00Z")
        .hire("e1", "verifier", "verify", "2026-01-01T00:00:00Z")
        .end(
            EventType::EffectSucceeded,
            "e1",
            "stale",
            json!({"result": "pass"}),
            "2026-01-01T00:00:01Z",
        )
        .push(
            EventType::EffectFailed,
            json!({"effect_id": "e1", "attempt_id": "a1", "error": "exit 1"}),
            "2026-01-01T00:00:02Z",
        )
        .end(
            EventType::EffectSucceeded,
            "e1",
            "a0",
            json!({"result": "pass"}),
            "2026-01-01T00:00:03Z",
        );
    let board = dashboard(&journal.0);
    let last = board.last_seat.clone().unwrap();
    assert_eq!(
        (last.seat.as_str(), last.notes.as_deref()),
        ("verifier", Some("exit 1"))
    );
    assert_eq!((last.result, board.last_review), (None, None));
    assert_eq!(board.path[0].model, None);
    let notes = |result: Value| notes_of(&json!({"result": result}));
    assert_eq!(notes(json!({"notes": "  "})), None);
    assert_eq!(
        notes(json!({"notes": {"why": 1}})).as_deref(),
        Some(r#"{"why":1}"#)
    );
    let members = json!({"notes": {"members": {"solo": "fine"}}});
    assert_eq!(notes(members).as_deref(), Some("solo: fine"));
    assert_eq!(dashboard(&[]), Dashboard::default());
}

/// The live column's stream: the working seat's checkpoints and its
/// working members', newest first, and no other seat's.
#[test]
fn the_seat_at_work_streams_its_checkpoints_newest_first() {
    let mut journal = Journal::default();
    let turn = |effect: &str, member: Option<&str>, tool: &str| {
        let mut checkpoint = json!({"step": "seat-turn", "tool": tool});
        if let Some(member) = member {
            checkpoint["member"] = json!(member);
        }
        json!({"effect_id": effect, "attempt_id": "a1", "checkpoint": checkpoint})
    };
    journal
        .enter("review", "2026-01-01T00:00:00Z")
        .hire("e1", "reviewer", "review", "2026-01-01T00:00:00Z")
        .hire("e2", "reviewer-two", "review", "2026-01-01T00:00:00Z")
        .push(
            EventType::EffectCheckpointed,
            turn("e1", Some("security"), "Read"),
            "2026-01-01T00:00:01Z",
        )
        .push(
            EventType::EffectCheckpointed,
            turn("e2", None, "Bash"),
            "2026-01-01T00:00:02Z",
        )
        .push(
            EventType::EffectCheckpointed,
            turn("e1", None, "Grep"),
            "2026-01-01T00:00:03Z",
        )
        .push(
            EventType::EffectCheckpointed,
            turn("e1", Some("correctness"), "Edit"),
            "2026-01-01T00:00:04Z",
        );
    let view = run_view(&journal.0, None);
    let streamed: Vec<(&str, &str)> = working_checkpoints(&view, "reviewer")
        .into_iter()
        .map(|(label, row)| (label, row.step.as_str()))
        .collect();
    let wanted = [
        ("reviewer:correctness", "Edit"),
        ("reviewer", "Grep"),
        ("reviewer:security", "Read"),
    ];
    assert_eq!(streamed, wanted);
    journal.end(
        EventType::EffectSucceeded,
        "e1",
        "a1",
        json!({"result": "clean"}),
        "2026-01-01T00:00:05Z",
    );
    let view = run_view(&journal.0, None);
    assert_eq!(
        working_checkpoints(&view, "reviewer").len(),
        0,
        "a concluded seat streams nothing"
    );
}
