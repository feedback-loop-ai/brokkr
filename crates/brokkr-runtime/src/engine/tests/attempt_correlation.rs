use super::*;
use brokkr_core::EventType;

const FOREIGN_ATTEMPT: &str = "foreign-attempt-correlation-marker";

fn driver_script(result: &str, foreign_attempt: bool) -> String {
    let wrong_attempt = if foreign_attempt {
        "reported_attempt='foreign-attempt-correlation-marker'"
    } else {
        ":"
    };
    format!(
        r#"
read -r hello
printf '%s\n' '{{"proto":"forge-driver/v1","msg_id":"cap","type":"capabilities","driver":"correlation-test","version":"1","supports":[]}}'
read -r start
effect_id=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
attempt_id=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
reported_attempt="$attempt_id"
{wrong_attempt}
printf '{{"proto":"forge-driver/v1","msg_id":"accepted","type":"accepted","effect_id":"%s","attempt_id":"%s","session_ref":"current-session"}}\n' "$effect_id" "$attempt_id"
printf '{{"proto":"forge-driver/v1","msg_id":"checkpoint","type":"checkpoint","effect_id":"%s","attempt_id":"%s","data":{{"step":"{result}"}}}}\n' "$effect_id" "$attempt_id"
printf '{{"proto":"forge-driver/v1","msg_id":"result","type":"result","effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{{"result":"{result}"}},"error":null}}\n' "$effect_id" "$reported_attempt"
read -r done
"#
    )
}

fn configured_engine(foreign_review: bool) -> (tempfile::TempDir, Engine) {
    let (dir, mut engine) = engine(single_body(vec!["unused".into()]));
    for (phase, result) in [
        ("work", "complete"),
        ("review", "clean"),
        ("ship", "shipped"),
    ] {
        let script = driver_script(result, phase == "review" && foreign_review);
        engine.bundle.seats.get_mut(phase).unwrap().body =
            single_body(vec!["sh".into(), "-c".into(), script]);
    }
    (dir, engine)
}

fn effect_for_seat(events: &[EventEnvelope], seat: &str) -> String {
    events
        .iter()
        .find(|event| {
            event.event_type == EventType::EffectRequested && event.payload["seat"] == seat
        })
        .unwrap_or_else(|| panic!("no request for {seat}: {events:#?}"))
        .payload["effect_id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn terminal_for_effect<'a>(events: &'a [EventEnvelope], effect: &str) -> Vec<&'a EventEnvelope> {
    events
        .iter()
        .filter(|event| {
            matches!(
                event.event_type,
                EventType::EffectSucceeded
                    | EventType::EffectFailed
                    | EventType::EffectIndeterminate
            ) && event.payload["effect_id"] == effect
        })
        .collect()
}

fn checkpoint_for_effect<'a>(events: &'a [EventEnvelope], effect: &str) -> &'a EventEnvelope {
    events
        .iter()
        .find(|event| {
            event.event_type == EventType::EffectCheckpointed
                && event.payload["effect_id"] == effect
        })
        .unwrap()
}

#[test]
fn foreign_clean_review_result_parks_and_replays_without_ship_authority() {
    let (dir, mut engine) = configured_engine(true);
    let run_id = engine.run_id.clone();
    let bundle = engine.bundle.clone();
    let db = dir.path().join("forge.db");
    let repo = dir.path().join("work");
    let end = engine.drive().unwrap();
    let events = engine.store.load(&run_id).unwrap();
    assert_eq!(
        end.state.run.status(),
        Status::AwaitingOperator,
        "{events:#?}"
    );
    let review_effect = effect_for_seat(&events, "review");
    let review_start = events
        .iter()
        .find(|event| {
            event.event_type == EventType::EffectStarted
                && event.payload["effect_id"] == review_effect
        })
        .unwrap();
    let review_attempt = review_start.payload["attempt_id"].clone();
    let review_terminals = terminal_for_effect(&events, &review_effect);
    assert_eq!(review_terminals.len(), 1);
    assert_eq!(
        review_terminals[0].event_type,
        EventType::EffectIndeterminate
    );
    assert_eq!(review_terminals[0].payload["attempt_id"], review_attempt);
    assert_eq!(
        review_terminals[0].payload["reason"],
        format!(
            "result for foreign attempt: its \u{60}attempt_id\u{60} field held 34 bytes \
             (first byte: letter; sha256: 1da73c5b10ca1dff), want '{}'; no result was \
             read after start, and what the attempt did is unknown; stderr tail: ",
            review_attempt.as_str().unwrap()
        )
    );
    assert!(!terminal_for_effect(&events, &review_effect)
        .iter()
        .any(|event| event.event_type == EventType::EffectSucceeded));
    assert!(!events.iter().any(|event| {
        event.event_type == EventType::TransitionDecided
            && event.payload["from"] == "review"
            && event.payload["next"] == "ship"
    }));
    assert!(!events.iter().any(|event| {
        event.event_type == EventType::EffectRequested && event.payload["seat"] == "ship"
    }));
    assert!(!events
        .iter()
        .any(|event| event.event_type == EventType::RunCompleted));

    let checkpoint = checkpoint_for_effect(&events, &review_effect)
        .payload
        .clone();
    assert_eq!(checkpoint["attempt_id"], review_attempt);
    assert_eq!(checkpoint["checkpoint"], json!({"step":"clean"}));
    let encoded = serde_json::to_string(&events).unwrap();
    assert!(!encoded.contains(FOREIGN_ATTEMPT));

    drop(engine);
    let mut resumed =
        Engine::resume(Store::open(&db).unwrap(), bundle, &run_id, Some(repo)).unwrap();
    let replayed_end = resumed.drive().unwrap();
    assert_eq!(replayed_end.state.run.status(), Status::AwaitingOperator);
    let replayed = resumed.store.load(&run_id).unwrap();
    let replayed_effect = effect_for_seat(&replayed, "review");
    assert_eq!(replayed_effect, review_effect);
    let replayed_terminal = terminal_for_effect(&replayed, &review_effect);
    assert_eq!(replayed_terminal.len(), 1);
    assert_eq!(
        replayed_terminal[0].event_type,
        EventType::EffectIndeterminate
    );
    assert_eq!(
        checkpoint_for_effect(&replayed, &review_effect).payload,
        checkpoint
    );
    assert!(!replayed.iter().any(|event| {
        event.event_type == EventType::EffectRequested && event.payload["seat"] == "ship"
    }));
    assert!(!replayed
        .iter()
        .any(|event| event.event_type == EventType::RunCompleted));
    assert!(!serde_json::to_string(&replayed)
        .unwrap()
        .contains(FOREIGN_ATTEMPT));
}

#[test]
fn matching_review_result_reaches_ship_and_completes() {
    let (_dir, mut engine) = configured_engine(false);
    let end = engine.drive().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(end.state.run.status(), Status::Completed, "{events:#?}");
    let review_effect = effect_for_seat(&events, "review");
    let review_terminal = terminal_for_effect(&events, &review_effect);
    assert_eq!(review_terminal.len(), 1);
    assert_eq!(review_terminal[0].event_type, EventType::EffectSucceeded);
    assert!(events.iter().any(|event| {
        event.event_type == EventType::EffectRequested && event.payload["seat"] == "ship"
    }));
    assert!(events.iter().any(|event| {
        event.event_type == EventType::TransitionDecided
            && event.payload["from"] == "review"
            && event.payload["next"] == "ship"
    }));
    assert!(events
        .iter()
        .any(|event| event.event_type == EventType::RunCompleted));
}
