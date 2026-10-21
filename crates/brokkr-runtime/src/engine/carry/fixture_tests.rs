//! Synthetic journal and protocol fixtures shared by carry and operator proofs.
use super::record::{PanelRecord, PanelSpan, Tag};
use crate::engine::tests::{driver_command, engine, git_commit, member};
use crate::engine::*;

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
