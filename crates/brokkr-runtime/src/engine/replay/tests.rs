use super::*;
use brokkr_core::envelope::EventType;
use brokkr_core::fold::{Cursor, Run};
use brokkr_store::test_support::{every_run_reads_as_its_fold, lags, lane};
use brokkr_store::{Store, StoreError};
use serde_json::json;

#[path = "../../../../../tests/support/seat_facts.rs"]
mod seat_facts;

/// What `seat_facts::disagreements` finds of the account `replay`
/// carried, held to the run view of the events it read.
fn carried_disagreements(replay: &Replay) -> Option<(Vec<String>, usize)> {
    let account = replay.account.as_ref()?;
    let facts = brokkr_core::seat::row::facts(account.seats());
    Some(seat_facts::disagreements(
        account.run_id(),
        &replay.events,
        &facts,
    ))
}

/// After every engine scenario, the account its replay carried turn by
/// turn groups its seat rows into the participants the run view shows of
/// the events it read (#753; decision 0089 ruling 4 at the projection's
/// level), as the store's drop binding holds a run's summary row to its
/// fold. A replay that holds no account read nothing to hold. Not while a
/// test is already failing: its own panic is the one to report.
impl Drop for Replay {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        if let Some((found, _)) = carried_disagreements(self) {
            assert_eq!(found, Vec::<String>::new(), "a carried seat row disagrees");
        }
    }
}

/// A run started and entered into `intake`: two events.
fn started(store: &mut Store, run_id: &str) {
    store
        .create_run(run_id, "feat", "self", &json!({}))
        .unwrap();
    let opened = json!({"feature": "feat", "manifest": {}});
    store
        .append_next(run_id, EventType::RunStarted, opened, None, None)
        .unwrap();
    let entered = json!({"phase": "intake"});
    store
        .append_next(run_id, EventType::PhaseEntered, entered, None, None)
        .unwrap();
}

fn requested(store: &mut Store, run_id: &str) {
    let payload = json!({"effect_id": "fx", "seat": "intake"});
    store
        .append_next(run_id, EventType::EffectRequested, payload, None, None)
        .unwrap();
}

/// A later turn reads, verifies and folds only what landed since the
/// last: a prefix spoiled after it was read goes unread, where a whole
/// load refuses it.
#[test]
fn a_turn_reads_only_what_landed_since_the_last() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    started(&mut store, "r1");
    let mut replay = Replay::default();
    assert_eq!(replay.caught_up(&mut store, "r1").unwrap().seq, 2);

    let raw = rusqlite::Connection::open(store.path()).unwrap();
    raw.execute_batch(
        "DROP TRIGGER events_append_only_update;
         UPDATE events SET envelope = replace(envelope, '\"feat\"', '\"spoiled\"') WHERE seq = 1;",
    )
    .unwrap();
    requested(&mut store, "r1");

    let state = replay.caught_up(&mut store, "r1").unwrap();
    assert_eq!(state.seq, 3);
    assert_eq!(
        state.run,
        Run::Running(Cursor::ExecuteEffect {
            effect_id: "fx".into(),
            seat: "intake".into(),
            failed_attempts: 0,
        })
    );
    assert_eq!(replay.events.len(), 3);
    let whole = store.load("r1").unwrap_err();
    assert!(
        matches!(
            whole,
            StoreError::Chain(brokkr_core::envelope::ChainError::BadHash { seq: 1 })
        ),
        "{whole}"
    );
    lags(&mut store, &["r1"]);
}

/// A replay is one run's: asked for another, it reads that run whole
/// rather than carrying the first run's state onto the second's suffix.
#[test]
fn a_replay_asked_for_another_run_reads_it_whole() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    started(&mut store, "long");
    requested(&mut store, "long");
    started(&mut store, "short");
    let mut replay = Replay::default();
    assert_eq!(replay.caught_up(&mut store, "long").unwrap().seq, 3);

    let state = replay.caught_up(&mut store, "short").unwrap();
    assert_eq!((state.run_id.as_str(), state.seq), ("short", 2));
    assert_eq!(state.run, Run::Running(Cursor::RequestEffect));
    assert_eq!(replay.events.len(), 2);
}

/// A turn hands the store the account it caught up to, so the turn's
/// appends write the run's summary (decision 0089 ruling 3): an append
/// made with none leaves the row behind its head, one made after a turn
/// leaves it at its head. Its removal control is a replay that keeps its
/// account to itself, whose append leaves the row behind.
#[test]
fn a_turn_hands_the_store_the_account_it_caught_up_to() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    started(&mut Store::open(&path).unwrap(), "r1");
    let mut store = Store::open(&path).unwrap();
    requested(&mut store, "r1");
    assert_eq!(lane(&store, "r1").as_deref(), Some("behind"));

    let mut replay = Replay::default();
    assert_eq!(replay.caught_up(&mut store, "r1").unwrap().seq, 3);
    let opened = json!({"effect_id": "fx", "attempt_id": "a1"});
    store
        .append_next("r1", EventType::EffectStarted, opened, None, None)
        .unwrap();
    assert_eq!(lane(&store, "r1").as_deref(), Some("running"));
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// A driven seat's turns, one repeating the other's message and each
/// priced, reach the account the engine carried turn by turn, which
/// groups its seat rows into the participants its run view shows: what
/// the drop check above holds every engine scenario to.
#[test]
fn a_driven_seat_s_carried_rows_read_as_its_run_view_s_participants() {
    let script = r#"
read -r hello
printf '%s\n' '{"proto":"forge-driver/v1","msg_id":"cap","type":"capabilities","driver":"test","version":"1","supports":[]}'
read -r start
effect_id=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
attempt_id=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
printf '{"proto":"forge-driver/v1","msg_id":"accepted","type":"accepted","effect_id":"%s","attempt_id":"%s","session_ref":null}\n' "$effect_id" "$attempt_id"
for turn in 1 2; do
printf '{"proto":"forge-driver/v1","msg_id":"t%s","type":"checkpoint","effect_id":"%s","attempt_id":"%s","data":{"step":"seat-turn","turn":%s,"model":"m","input_tokens":100,"output_tokens":10,"total_cost_usd":0.%s}}\n' "$turn" "$effect_id" "$attempt_id" "$turn" "$turn"
done
printf '{"proto":"forge-driver/v1","msg_id":"result","type":"result","effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{"result":"complete"},"error":null}\n' "$effect_id" "$attempt_id"
read -r done
"#;
    let body = super::super::tests::single_body(vec!["sh".into(), "-c".into(), script.into()]);
    let (_dir, mut engine) = super::super::tests::engine(body);
    engine
        .append(EventType::PhaseEntered, json!({"phase": "work"}), None)
        .unwrap();
    // Turn by turn, to the first whose carried account holds a priced row.
    let facts = loop {
        let turn = engine.drive_once().unwrap();
        assert!(turn.is_none(), "the run concluded before its seat ran");
        let account = engine.replay.account.as_ref().unwrap();
        let facts = brokkr_core::seat::row::facts(account.seats());
        if facts.iter().any(|facts| facts.cost.is_some()) {
            break facts;
        }
    };
    let spent: Vec<_> = facts
        .iter()
        .map(|facts| (facts.cost, facts.usage.clone()))
        .collect();
    assert_eq!(
        spent,
        [(
            Some(0.30000000000000004),
            Some(brokkr_core::seat::TokenUsage {
                input_tokens: Some(100),
                output_tokens: Some(10),
                total_tokens: Some(110),
                ..Default::default()
            })
        )]
    );
    assert_eq!(carried_disagreements(&engine.replay), Some((Vec::new(), 1)));
}

/// An engine driven to its conclusion leaves its run's summary at the
/// head it concluded on, so every engine scenario's drop check compares
/// the row with the fold rather than passing a row behind.
#[test]
fn a_driven_run_leaves_its_summary_at_its_head() {
    let body = super::super::tests::single_body(vec!["driver".into()]);
    let (_dir, mut engine) = super::super::tests::engine(body);
    for (event_type, payload) in [
        (EventType::PhaseEntered, json!({"phase": "work"})),
        (EventType::RunCompleted, json!({})),
    ] {
        engine.append(event_type, payload, None).unwrap();
    }
    let end = engine.drive().unwrap();
    assert_eq!(end.state.seq, 3);
    assert_eq!(
        lane(&engine.store, &engine.run_id).as_deref(),
        Some("finished")
    );
}
