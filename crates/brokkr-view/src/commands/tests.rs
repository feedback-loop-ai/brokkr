//! The operator's choices (#568, design D2): each command admitted or
//! refused by its verb's own rule, and each disabled action shown under
//! its condition and nowhere else.

use super::*;
use crate::clarify::Clarify;
use crate::fleet::tests::{finding, ruled};
use crate::tests::state;
use serde_json::json;

fn retry(phase: Option<&str>) -> Command {
    Command::Retry {
        phase: phase.map(str::to_string),
    }
}

/// Each command beside its refusal, `None` where it is admitted.
fn ruled_on(state: &RunState) -> Vec<(Command, Option<CommandRefusal>)> {
    commands(state)
        .into_iter()
        .map(|offered| {
            let refusal = match offered.admission {
                Admission::Admitted => None,
                Admission::Refused { refusal, text } => {
                    assert_eq!(text, refusal.text("r1"), "the text is the refusal's own");
                    Some(refusal)
                }
            };
            (offered.command, refusal)
        })
        .collect()
}

fn operator(command: OperatorCommand, refusal: Refusal) -> Option<CommandRefusal> {
    Some(CommandRefusal::Operator { command, refusal })
}

#[test]
fn a_run_parked_at_review_admits_retry_stop_and_conclude_and_refuses_resume() {
    let parked = state(Some("review"), Status::AwaitingOperator, None);
    assert_eq!(
        ruled_on(&parked),
        [
            (retry(Some("review")), None),
            (Command::Stop, None),
            (Command::Resume, Some(CommandRefusal::Parked)),
            (Command::Conclude, None),
        ]
    );
}

#[test]
fn a_running_run_refuses_retry_as_not_awaiting_the_operator() {
    let running = state(Some("implement"), Status::Running, None);
    assert_eq!(
        ruled_on(&running),
        [
            (
                retry(Some("implement")),
                operator(OperatorCommand::Retry, Refusal::RunNotAwaitingOperator)
            ),
            (Command::Stop, None),
            (Command::Resume, None),
            (Command::Conclude, None),
        ]
    );
}

#[test]
fn a_concluded_run_refuses_all_four_with_the_terminal_reason() {
    for status in [Status::Completed, Status::Stopped] {
        let ended = state(Some("done"), status, None);
        assert_eq!(
            ruled_on(&ended),
            [
                (
                    retry(Some("done")),
                    operator(OperatorCommand::Retry, Refusal::AfterTerminal)
                ),
                (
                    Command::Stop,
                    operator(OperatorCommand::Stop, Refusal::AfterTerminal)
                ),
                (Command::Resume, Some(CommandRefusal::Ended(status))),
                (Command::Conclude, Some(CommandRefusal::Concluded(status))),
            ],
            "{status:?}"
        );
    }
}

#[test]
fn a_run_parked_before_any_phase_has_no_phase_to_retry() {
    let parked = state(None, Status::AwaitingOperator, None);
    assert_eq!(
        ruled_on(&parked)[0],
        (
            retry(None),
            operator(OperatorCommand::Retry, Refusal::NoPhaseToRetry)
        )
    );
}

/// The refusals' words and texts, pinned once: the verb's and the view's
/// are one text.
#[test]
fn each_refusal_says_the_words_its_verb_says() {
    for word in [
        "after_terminal",
        "run_not_awaiting_operator",
        "no_phase_to_retry",
    ] {
        assert_eq!(member_retry_refusal(word), format!("refused operator retry --member ({word}): the run is not in a state a retry can apply to; nothing was journaled"));
    }
    let retry = operator(OperatorCommand::Retry, Refusal::RunNotAwaitingOperator).unwrap();
    assert_eq!(retry.word(), "run_not_awaiting_operator");
    assert_eq!(
        retry.text("r1"),
        "refused operator retry (run_not_awaiting_operator): the run is not in a state this \
         command can apply to; the refusal is journaled. Read it with: brokkr inspect --run r1"
    );
    let concluded = CommandRefusal::Concluded(Status::Stopped);
    assert_eq!(concluded.word(), "already_concluded");
    assert_eq!(
        concluded.text("r1"),
        "run 'r1' is already concluded (stopped); conclude appends nothing"
    );
    assert_eq!(CommandRefusal::Parked.word(), "awaiting_operator");
    assert_eq!(
        CommandRefusal::Parked.text("r1"),
        "run 'r1' awaits the operator; resume drives it again only once a retry is accepted"
    );
    let ended = CommandRefusal::Ended(Status::Completed);
    assert_eq!(ended.word(), "after_terminal");
    assert_eq!(
        ended.text("r1"),
        "run 'r1' is already concluded (completed); resume drives nothing"
    );
}

#[test]
fn the_wire_names_each_command_and_its_admission() {
    let parked = state(Some("review"), Status::AwaitingOperator, None);
    let wire = serde_json::to_value(commands(&parked)).unwrap();
    assert_eq!(
        wire[0],
        json!({"command": "retry", "phase": "review", "admission": "admitted"})
    );
    assert_eq!(
        wire[2],
        json!({"command": "resume", "admission": "refused", "refusal": "awaiting_operator",
               "text": CommandRefusal::Parked.text("r1")})
    );
}

#[test]
fn the_bridge_door_offers_only_a_parked_runs_admitted_operator_commands() {
    let parked = state(Some("review"), Status::AwaitingOperator, None);
    assert_eq!(operator_commands(&parked), ["retry", "stop"]);
    let running = state(Some("review"), Status::Running, None);
    assert!(
        commands(&running)[1].admission == Admission::Admitted,
        "the view admits a running run's stop"
    );
    assert_eq!(operator_commands(&running), Vec::<String>::new());
}

/// The four reasons, pinned once, as the canvas writes them.
#[test]
fn the_unavailable_reasons_are_the_canvas_copy() {
    let reasons: Vec<&str> = Unavailable::ALL.map(Unavailable::reason).to_vec();
    assert_eq!(
        reasons,
        [
            "Brokkr only lets you dismiss findings once a run has finished.",
            "Reviews start fresh each time; your note can't reach the reviewer yet.",
            "A parked run already waits for you; it holds nothing else back.",
            "Not yet: Brokkr has no way to pass an answer to a run directly.",
        ]
    );
}

fn actions(state: &RunState, residuals: &[ResidualFinding], clarify: bool) -> Vec<Unavailable> {
    let questions = Clarify {
        seq: 5,
        questions: vec!["Which target?".to_string()],
    };
    let clarify = clarify.then_some(&questions);
    let disabled = unavailable(state, residuals, clarify);
    for shown in &disabled {
        assert_eq!(shown.reason, shown.action.reason());
    }
    disabled.into_iter().map(|shown| shown.action).collect()
}

#[test]
fn each_unavailable_action_is_shown_under_its_condition_and_nowhere_else() {
    let open = [finding("max_residual_severity", "high", false)];
    let closed = [finding("max_residual_severity", "high", true)];
    let review = state(Some("review"), Status::AwaitingOperator, None);
    assert_eq!(
        actions(&review, &open, false),
        [
            Unavailable::DismissFinding,
            Unavailable::TalkToReviewer,
            Unavailable::Hold
        ]
    );
    assert_eq!(
        actions(&review, &closed, false),
        [Unavailable::TalkToReviewer, Unavailable::Hold],
        "a superseded finding is not open"
    );
    let clarify = state(
        Some("clarify"),
        Status::AwaitingOperator,
        ruled("CLARIFY-AMBIGUOUS-EXHAUSTED", "clarify", None),
    );
    assert_eq!(
        actions(&clarify, &[], true),
        [Unavailable::Hold, Unavailable::AnswerHere]
    );
    let running = state(Some("review"), Status::Running, None);
    assert_eq!(
        actions(&running, &open, false),
        [Unavailable::DismissFinding]
    );
    for status in [Status::Completed, Status::Stopped] {
        let finished = state(Some("review"), status, None);
        assert_eq!(
            actions(&finished, &open, false),
            [],
            "a finished run's finding is dismissed by supersede"
        );
    }
}
