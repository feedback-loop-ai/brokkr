use super::*;
use serde_json::json;

const FOREIGN_ATTEMPT: &str = "sk-ant-api03-planted-secret";
const FOREIGN_EVIDENCE: &str = "27 bytes (first byte: letter; sha256: 90bb697bdad60c89)";
const PREVIOUS_EVIDENCE: &str = "16 bytes (first byte: letter; sha256: 36d7c6429aef67cb)";

fn refusal(said: &str, evidence: &str) -> AttemptOutcome {
    AttemptOutcome::Indeterminate {
        reason: format!(
            "{0}: its {1}attempt_id{1} field held {2}, want 'attempt'; no result was read after start, and what the attempt did is unknown",
            said,
            '\u{60}',
            evidence
        ),
    }
}

fn accepted_message(effect: &str, attempt: &str, session: Option<&str>) -> String {
    wire(Body::Accepted {
        effect_id: effect.into(),
        attempt_id: attempt.into(),
        session_ref: session.map(str::to_owned),
    })
}

fn checkpoint_message(effect: &str, attempt: &str, data: Value) -> String {
    wire(Body::Checkpoint {
        effect_id: effect.into(),
        attempt_id: attempt.into(),
        data,
    })
}

fn result_message(
    effect: &str,
    attempt: &str,
    status: ResultStatus,
    result: Option<Value>,
    error: Option<&str>,
) -> String {
    wire(Body::Result {
        effect_id: effect.into(),
        attempt_id: attempt.into(),
        status,
        result,
        error: error.map(str::to_owned),
    })
}

fn run_messages(lines: &[String]) -> (AttemptReport, Vec<Value>) {
    let frames = lines
        .iter()
        .map(|line| format!("printf '%s\\n' '{line}'"))
        .collect::<Vec<_>>()
        .join("; ");
    let script = format!(
        "read -r hello; printf '%s\\n' '{}'; read -r start; {}; read -r done",
        capabilities(),
        frames
    );
    let mut callbacks = Vec::new();
    let report = DriverProcess::spawn(
        &command(&script),
        std::path::Path::new("."),
        None,
        &SpawnEnv::Inherit,
    )
    .unwrap()
    .run_attempt("test", "effect", "attempt", "seat", json!({}), |value| {
        callbacks.push(value.clone())
    });
    (report, callbacks)
}

fn report_record(report: &AttemptReport) -> Value {
    json!({
        "outcome": report.outcome,
        "session_ref": report.session_ref,
        "checkpoints": report.checkpoints,
        "accepted": report.accepted
    })
}

#[test]
fn a_foreign_attempt_acceptance_cannot_set_acceptance_or_session() {
    let (report, callbacks) = run_messages(&[
        accepted_message("effect", FOREIGN_ATTEMPT, Some("foreign-session")),
        result_message(
            "effect",
            "attempt",
            ResultStatus::Succeeded,
            Some(json!({"result":"complete"})),
            None,
        ),
    ]);
    assert_eq!(
        report.outcome,
        refusal("driver accepted a different attempt", FOREIGN_EVIDENCE)
    );
    assert!(!report.accepted);
    assert_eq!(report.session_ref, None);
    assert_eq!(report.checkpoints, Vec::<Value>::new());
    assert_eq!(callbacks, Vec::<Value>::new());
    assert!(!report_record(&report).to_string().contains(FOREIGN_ATTEMPT));

    let (prior_acceptance, _) = run_messages(&[
        accepted_message("effect", "attempt", Some("legitimate-session")),
        accepted_message("effect", FOREIGN_ATTEMPT, Some("foreign-session")),
        result_message(
            "effect",
            "attempt",
            ResultStatus::Succeeded,
            Some(json!({"result":"complete"})),
            None,
        ),
    ]);
    assert_eq!(
        prior_acceptance.outcome,
        refusal("driver accepted a different attempt", FOREIGN_EVIDENCE)
    );
    assert!(prior_acceptance.accepted);
    assert_eq!(prior_acceptance.session_ref, None);
}

#[test]
fn a_foreign_attempt_checkpoint_is_neither_retained_nor_delivered() {
    let (report, callbacks) = run_messages(&[
        checkpoint_message(
            "effect",
            FOREIGN_ATTEMPT,
            json!({"step":"foreign-checkpoint"}),
        ),
        result_message(
            "effect",
            "attempt",
            ResultStatus::Succeeded,
            Some(json!({"result":"complete"})),
            None,
        ),
    ]);
    assert_eq!(
        report.outcome,
        refusal("checkpoint for foreign attempt", FOREIGN_EVIDENCE)
    );
    assert!(!report.accepted);
    assert_eq!(report.checkpoints, Vec::<Value>::new());
    assert_eq!(callbacks, Vec::<Value>::new());
    assert!(!report_record(&report).to_string().contains(FOREIGN_ATTEMPT));

    let legitimate = json!({"step":"legitimate"});
    let (prior_checkpoint, callbacks) = run_messages(&[
        accepted_message("effect", "attempt", Some("legitimate-session")),
        checkpoint_message("effect", "attempt", legitimate.clone()),
        checkpoint_message(
            "effect",
            FOREIGN_ATTEMPT,
            json!({"step":"foreign-checkpoint"}),
        ),
        result_message(
            "effect",
            "attempt",
            ResultStatus::Succeeded,
            Some(json!({"result":"complete"})),
            None,
        ),
    ]);
    assert_eq!(
        prior_checkpoint.outcome,
        refusal("checkpoint for foreign attempt", FOREIGN_EVIDENCE)
    );
    assert!(prior_checkpoint.accepted);
    assert_eq!(prior_checkpoint.session_ref, None);
    assert_eq!(prior_checkpoint.checkpoints, Vec::<Value>::new());
    assert_eq!(callbacks, vec![legitimate]);
}

#[test]
fn foreign_successful_and_failed_results_are_indeterminate() {
    for (status, result, error) in [
        (
            ResultStatus::Succeeded,
            Some(json!({"result":"foreign-success"})),
            None,
        ),
        (ResultStatus::Failed, None, Some("foreign failure")),
    ] {
        let checkpoint = json!({"step":"legitimate"});
        let (report, callbacks) = run_messages(&[
            accepted_message("effect", "attempt", Some("current-session")),
            checkpoint_message("effect", "attempt", checkpoint.clone()),
            result_message("effect", FOREIGN_ATTEMPT, status, result, error),
        ]);
        assert_eq!(
            report.outcome,
            refusal("result for foreign attempt", FOREIGN_EVIDENCE)
        );
        assert!(report.accepted);
        assert_eq!(report.session_ref, None);
        assert_eq!(report.checkpoints, Vec::<Value>::new());
        assert_eq!(callbacks, vec![checkpoint]);
        assert!(!report_record(&report).to_string().contains(FOREIGN_ATTEMPT));
    }
}

#[test]
fn foreign_attempt_bytes_never_enter_reported_diagnostics() {
    let (report, callbacks) = run_messages(&[result_message(
        "effect",
        FOREIGN_ATTEMPT,
        ResultStatus::Succeeded,
        Some(json!({"result":"complete"})),
        None,
    )]);
    let serialized = report_record(&report).to_string();
    assert!(!serialized.contains(FOREIGN_ATTEMPT), "{serialized}");
    assert_eq!(callbacks, Vec::<Value>::new());
}

#[test]
fn effect_mismatch_wins_when_both_identifiers_are_foreign() {
    let cases = [
        accepted_message("foreign", FOREIGN_ATTEMPT, None),
        checkpoint_message("foreign", FOREIGN_ATTEMPT, json!({"step":"x"})),
        result_message(
            "foreign",
            FOREIGN_ATTEMPT,
            ResultStatus::Succeeded,
            Some(json!({"result":"x"})),
            None,
        ),
    ];
    for (line, said) in cases.into_iter().zip([
        "driver accepted a different effect",
        "checkpoint for foreign effect",
        "result for foreign effect",
    ]) {
        let (report, _) = run_messages(&[line]);
        assert_eq!(
            report.outcome,
            lost_after_start(&format!(
                "{said}: its \u{60}effect_id\u{60} field held 7 bytes (first byte: letter; \
                 sha256: 656771905e1ef731), want 'effect'"
            ))
        );
        assert!(!report_record(&report).to_string().contains(FOREIGN_ATTEMPT));
    }
}

#[test]
fn matching_attempt_evidence_keeps_existing_success_and_checkpoint_behavior() {
    let checkpoint = json!({"step":"legitimate"});
    let (report, callbacks) = run_messages(&[
        accepted_message("effect", "attempt", Some("current-session")),
        checkpoint_message("effect", "attempt", checkpoint.clone()),
        result_message(
            "effect",
            "attempt",
            ResultStatus::Succeeded,
            Some(json!({"result":"complete"})),
            None,
        ),
    ]);
    assert_eq!(
        report.outcome,
        AttemptOutcome::Succeeded {
            result: json!({"result":"complete"})
        }
    );
    assert!(report.accepted);
    assert_eq!(report.session_ref.as_deref(), Some("current-session"));
    assert_eq!(report.checkpoints, vec![checkpoint.clone()]);
    assert_eq!(callbacks, vec![checkpoint]);
}

#[test]
fn a_resumed_session_does_not_admit_its_prior_attempt() {
    let capabilities = wire(Body::Capabilities {
        driver: "test".into(),
        version: "1".into(),
        supports: vec!["resume".into()],
    });
    let accepted = accepted_message("effect", "previous-attempt", Some("old-session"));
    let script = format!(
        "read -r hello; printf '%s\\n' '{}'; read -r resume; read -r start; \
         printf '%s\\n' '{}'; read -r done",
        capabilities, accepted
    );
    let report = DriverProcess::spawn(
        &command(&script),
        std::path::Path::new("."),
        None,
        &SpawnEnv::Inherit,
    )
    .unwrap()
    .run_attempt_resuming(
        "test",
        "effect",
        "attempt",
        "seat",
        json!({}),
        Some("old-session".into()),
        |_| {},
    );
    assert_eq!(
        report.outcome,
        refusal("driver accepted a different attempt", PREVIOUS_EVIDENCE)
    );
    assert!(!report.accepted);
    assert_eq!(report.session_ref, None);
}
