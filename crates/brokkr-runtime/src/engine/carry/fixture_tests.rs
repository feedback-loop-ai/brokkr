//! Synthetic journal and protocol fixtures shared by carry and operator proofs.
use super::record::{PanelRecord, PanelSpan, Tag};
use crate::bundle::{SeatClass, SequenceStep};
use crate::engine::tests::{driver_command, engine, git_commit, member, step_body};
use crate::engine::*;

pub(crate) fn clean() -> AttemptOutcome {
    AttemptOutcome::Succeeded {
        result: json!({"result":"clean"}),
    }
}

pub(crate) fn lost() -> AttemptOutcome {
    AttemptOutcome::Indeterminate {
        reason: "vanished".into(),
    }
}

/// A review-panel step whose members report the outcomes given, in declared order.
pub(crate) fn panel_step(name: &str, members: Vec<(&str, AttemptOutcome)>) -> SequenceStep {
    SequenceStep {
        name: name.into(),
        class: SeatClass::Work,
        results: vec!["clean".into(), "residual".into(), "security-hold".into()],
        body: StepBody::Invocation(Invocation::Panel {
            members: members
                .into_iter()
                .map(|(name, outcome)| member(name, command(outcome)))
                .collect(),
            aggregate: Aggregate::ReviewPanel,
        }),
    }
}

/// A single-driver step reporting `outcome`.
pub(crate) fn single_step(name: &str, outcome: AttemptOutcome) -> SequenceStep {
    SequenceStep {
        name: name.into(),
        class: SeatClass::Work,
        results: vec![],
        body: step_body(command(outcome)),
    }
}

/// A sequence whose panel step, `positions`, follows a first step, `chief`.
pub(crate) fn panel_after_chief() -> SeatBody {
    SeatBody::Sequence {
        steps: vec![
            single_step("chief", clean()),
            panel_step("positions", vec![("a", clean()), ("b", clean())]),
        ],
    }
}

pub(crate) fn command(result: AttemptOutcome) -> Vec<String> {
    let mut argv = driver_command("EFFECT", "ATTEMPT", result);
    let parse = r#"; read -r line; effect_id=$(printf '%s' "$line" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p'); attempt_id=$(printf '%s' "$line" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p'); printf"#;
    argv[2] = argv[2]
        .replacen("; read -r line; printf", parse, 1)
        .replace("EFFECT", "'\"$effect_id\"'")
        .replace("ATTEMPT", "'\"$attempt_id\"'");
    argv
}

pub(crate) fn body() -> SeatBody {
    SeatBody::Invocation(Invocation::Panel {
        members: ["a", "b"]
            .into_iter()
            .map(|name| {
                member(
                    name,
                    command(AttemptOutcome::Succeeded {
                        result: json!({"result":"clean"}),
                    }),
                )
            })
            .collect(),
        aggregate: Aggregate::ReviewPanel,
    })
}

pub(crate) fn append(runtime: &mut Engine, kind: EventType, payload: Value) {
    runtime
        .store
        .append_next(&runtime.run_id, kind, payload, None, None)
        .unwrap();
}

pub(crate) fn events(runtime: &Engine) -> Vec<EventEnvelope> {
    runtime.store.load(&runtime.run_id).unwrap()
}

pub(crate) fn parked(body: SeatBody, secrets: bool) -> (tempfile::TempDir, Engine) {
    parked_record(body, secrets, |_| {})
}

/// `runtime` with its secrets store configured at `store`, as a later
/// command's process is launched with it: one engine keeps the place it
/// resolved for its run, and a new process resolves afresh (U8h, rulings 18
/// and 19).
pub(crate) fn relaunched_with(runtime: &mut Engine, store: Option<std::path::PathBuf>) {
    runtime.secrets_file = store;
    runtime.store_place = Default::default();
}

pub(crate) fn parked_record(
    body: SeatBody,
    secrets: bool,
    record: impl FnOnce(&mut Value),
) -> (tempfile::TempDir, Engine) {
    let (dir, mut runtime) = engine(body);
    if secrets {
        runtime.bundle.seats.get_mut("work").unwrap().secrets = vec!["TOKEN".into()];
        runtime.secrets_file = Some(dir.path().join("ops.env"));
    }
    // Runtime files are not changes to the reviewed subject.
    std::fs::write(runtime.repo.join(".gitignore"), ".forge/\n").unwrap();
    git_commit(&runtime.repo, "base");
    runtime.drive_once().unwrap();
    runtime.drive_once().unwrap();
    let requested = events(&runtime).last().unwrap().payload.clone();
    assert!(requested.get("input_digest").is_some());
    let mut terminal =
        json!({"effect_id":requested["effect_id"],"attempt_id":"source", "reason":"b vanished"});
    PanelRecord {
        span: PanelSpan::open(&runtime.repo),
        settled_members: [(Tag("a".into()), json!({"result":"clean"}))]
            .into_iter()
            .collect(),
    }
    .stamp(&mut terminal);
    append(
        &mut runtime,
        EventType::EffectStarted,
        json!({"effect_id":requested["effect_id"],"attempt_id":"source","driver":"panel"}),
    );
    record(&mut terminal);
    append(&mut runtime, EventType::EffectIndeterminate, terminal);
    let end = runtime.drive().unwrap();
    assert_eq!(
        end.state.run.status(),
        brokkr_core::fold::Status::AwaitingOperator
    );
    (dir, runtime)
}

pub(crate) fn retry(runtime: &mut Engine, names: &[&str]) -> MemberRetryOutcome {
    let read = Engine::read_member_retry(&mut runtime.store, &runtime.run_id)
        .unwrap()
        .unwrap();
    runtime
        .retry_members(
            read,
            &names
                .iter()
                .map(|name| name.to_string())
                .collect::<Vec<_>>(),
            "test",
            "recover",
        )
        .unwrap()
}
