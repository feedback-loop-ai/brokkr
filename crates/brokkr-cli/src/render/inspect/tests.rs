//! Decision 0064 ruling 6 in `inspect` and `watch`: both words once a run
//! has ended, the citation beside a cited ending, and the reason a policy
//! ending was ruled; `watch`'s frame is `inspect`'s without the trail.

use brokkr_core::{EventType, Status};
use serde_json::{json, Value};

use crate::render::{inspect, Style};
use crate::tests::envelope_builder::EnvelopeBuilder;
use crate::tests::run_state::{run_state, standing};

/// The head of the readout of run `run-7`, stopped in `phase` after
/// `decision` with `events` journaled: every line before the first blank
/// one, alike in `inspect` and in a `watch` frame.
fn head(phase: &str, decision: Option<Value>, events: &[(EventType, Value)]) -> String {
    let mut state = run_state("run-7", 14, standing(Status::Stopped, ""));
    state.phase = Some(phase.to_string());
    state.last_decision = decision.map(|it| serde_json::from_value(it).unwrap());
    let events: Vec<_> = (1..)
        .zip(events)
        .map(|(seq, (event_type, payload))| {
            EnvelopeBuilder::new(*event_type, payload.clone())
                .run("run-7")
                .seq(seq)
                .event_id(format!("ev{seq}"))
                .build()
        })
        .collect();
    let view = brokkr_view::run_view(&events, Some(&state));
    let style = Style::plain(100);
    let inspected = inspect(&view, None, true, &style);
    let watched = inspect(&view, None, false, &style);
    let lines = |text: &str| text.split("\n\n").next().unwrap().trim_end().to_string();
    assert_eq!(
        lines(&inspected),
        lines(&watched),
        "watch paints inspect's head"
    );
    lines(&inspected)
}

/// A policy ending reads both words and the reason its ruling recorded;
/// one whose ruling recorded no problem names the rule and its phase.
#[test]
fn a_policy_ending_reads_both_words_and_the_rulings_reason() {
    let hold = json!({"rule_id": "REVIEW-SECURITY-HOLD", "from": "review", "next": "stop",
                      "result": "hold", "problem": "the security hold stands"});
    assert_eq!(
        head("stop", Some(hold), &[]),
        "run  run-7\n     \
         status stopped · ending refused · phase stop · seq 14\n\
         stop  ruled by REVIEW-SECURITY-HOLD from review — the security hold stands\n\
         ruling  REVIEW-SECURITY-HOLD  review → stop · hold\n        \
         the security hold stands"
    );
    let failed = json!({"rule_id": "VERIFY-FAIL", "from": "verify", "next": "stop",
                        "result": "fail"});
    assert_eq!(
        head("stop", Some(failed), &[]),
        "run  run-7\n     \
         status stopped · ending failed · phase stop · seq 14\n\
         stop  ruled by VERIFY-FAIL from verify\n\
         ruling  VERIFY-FAIL  verify → stop · fail"
    );
}

/// A cited ending reads `ending superseded by <run>`, its citation's
/// journal text through `Safe`, and names no ruling: an operator ended it.
#[test]
fn a_cited_ending_reads_by_what_it_cites() {
    let classify = json!({"command_id": "c1", "command": "classify",
        "args": {"cause": "superseded", "reason": "re-commissioned",
                 "by": {"realm": "br\x1b]0;x\x07okkr", "run_id": "run\r\x1b[2J-8"}}});
    assert_eq!(
        head(
            "implement",
            None,
            &[(EventType::OperatorCommanded, classify)]
        ),
        "run  run-7\n     \
         status stopped · ending superseded by br]0;xokkr/run[2J-8 · phase implement · seq 14"
    );
}

/// An unclassified run reads both words and no reason: an operator stop
/// that named no cause, one whose fold keeps the ruling before it (an
/// operator ended it, so no rule did), and a stop from the policy's stop
/// phase whose ruling the fold does not hold.
#[test]
fn an_unclassified_ending_reads_both_words_and_no_reason() {
    assert_eq!(
        head("implement", None, &[]),
        "run  run-7\n     status stopped · ending unclassified · phase implement · seq 14"
    );
    let earlier = json!({"rule_id": "IMPL-OK", "from": "implement", "next": "verify",
                         "result": "complete"});
    assert_eq!(
        head("implement", Some(earlier), &[]),
        "run  run-7\n     \
         status stopped · ending unclassified · phase implement · seq 14\n\
         ruling  IMPL-OK  implement → verify · complete"
    );
    assert_eq!(
        head("stop", None, &[]),
        "run  run-7\n     status stopped · ending unclassified · phase stop · seq 14"
    );
}

/// With colour, the ending is painted in its tone; the status beside it
/// is the fold's word, unpainted.
#[test]
fn the_ending_is_painted_in_its_tone() {
    let mut state = run_state("run-7", 14, standing(Status::Stopped, ""));
    state.phase = Some("stop".to_string());
    let hold = json!({"rule_id": "R", "from": "review", "next": "stop", "result": "hold"});
    state.last_decision = Some(serde_json::from_value(hold).unwrap());
    let view = brokkr_view::run_view(&[], Some(&state));
    let coloured = Style {
        color: true,
        width: 100,
    };
    let out = inspect(&view, None, false, &coloured);
    let line = "     status stopped · ending \x1b[33mrefused\x1b[0m · phase stop · seq 14\n";
    assert!(out.contains(line), "{out:?}");
}
