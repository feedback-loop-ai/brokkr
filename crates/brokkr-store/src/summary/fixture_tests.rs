//! Every exported journal under `fixtures/journals/`, imported into a
//! fresh store, reads as its fold (decision 0089 ruling 4): the import
//! writes each run's summary from the verified run it folds, at its head.
//! A journal the import refuses is planted as a writer from before the
//! summary would leave it, and the next open's catch-up judges it. The
//! fixtures are read, never regenerated. The engine scenario tests are
//! held to the same check by every writer store's drop (`test_support`).

use brokkr_core::envelope::{ChainError, EventEnvelope, EventType};
use brokkr_core::summary::{aggregate, Aggregate, Ending};

use crate::test_support::{
    fixture_events, fixture_journals as fixtures, import_fixtures, lane,
    plant_older_writer as plant_as_older_writer,
};
use crate::{Disagreement, Store, StoreError};

#[test]
fn every_run_reads_as_its_fold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let mut store = Store::open(&path).unwrap();
    let (lanes, refused) = import_fixtures(&mut store);
    assert_eq!(
        refused,
        [
            "chain: event 11: event_hash does not match canonical content",
            "refused: the export's run/started event carries no manifest — an adoption \
             derives the destination's runs row from the verified chain, never from the \
             sidecar manifest that no hash covers, and a readout that shows this run must \
             not show a blank where the chain said nothing",
        ]
    );
    drop(store);
    let store = Store::open(&path).unwrap();
    let lanes: Vec<(&str, String)> = lanes
        .iter()
        .map(|run| (run.as_str(), lane(&store, run).unwrap()))
        .collect();
    // The broken chain does not load, so no writer judges it: it stays
    // behind, for a reader to read whole and refuse live, and the doctor's
    // sweep, which reads every run whole, names it.
    let lane = |run, lane: &str| (run, lane.to_string());
    assert_eq!(
        lanes,
        [
            lane("conclude-already-done-hand-built", "finished"),
            lane("conclude-broken-chain-hand-built", "behind"),
            lane("conclude-parked-hand-built", "parked"),
            lane("conclude-stopped-mid-effect-hand-built", "stopping"),
            lane("reforging-the-road-back-hand-built", "finished"),
            lane("tui-graph-the-selection-box-gets-80f98deb", "stopping"),
        ]
    );
    assert_eq!(
        crate::test_support::every_run_reads_as_its_fold(&store),
        ["conclude-broken-chain-hand-built: does not load: chain: event 11: event_hash does not \
          match canonical content"]
    );
}

/// What the aggregate readouts read of a run, by the column that keeps
/// each, as this test words them, apart from the writer's own words.
fn read_as_columns(read: &Aggregate) -> Vec<(&'static str, Option<String>)> {
    let ending = read.finished.as_ref().map(|(_, ending)| ending);
    let rule = match ending {
        Some(Ending::ByRule(rule)) => rule.clone(),
        Some(Ending::Shipped | Ending::ByOperator) | None => None,
    };
    let word = ending.map(|ending| match ending {
        Ending::Shipped => "shipped".to_string(),
        Ending::ByRule(_) => "by_rule".to_string(),
        Ending::ByOperator => "by_operator".to_string(),
    });
    let shown = |value: Option<i64>| value.map(|value| value.to_string());
    let flag = |set: bool| Some(u8::from(set).to_string());
    vec![
        ("realm", read.realm.clone()),
        ("recipe", read.recipe.clone()),
        ("started_ms", shown(read.started_ms)),
        (
            "finished_ms",
            shown(read.finished.as_ref().map(|(at, _)| *at)),
        ),
        ("ending", word),
        ("ending_rule", rule),
        ("implement_visits", Some(read.implement_visits.to_string())),
        ("review_first", flag(read.review_first)),
        ("review_second", flag(read.review_second)),
        ("review_back", flag(read.review_back)),
    ]
}

/// Every fixture's row keeps, column for column, what the readouts read of
/// its run read whole (`brokkr_core::summary::aggregate`, which
/// `brokkr analytics` and the recipe's graph read): the parity of decision
/// 0089 slice 4. The broken chain is read by no writer, so its row keeps
/// nothing.
#[test]
fn every_fixture_keeps_what_the_aggregate_readouts_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let mut store = Store::open(&path).unwrap();
    import_fixtures(&mut store);
    drop(store);
    let store = Store::open(&path).unwrap();
    let mut kept = Vec::new();
    for (_, _, events) in fixture_events() {
        let run_id = events[0].run_id.clone();
        let read = aggregate(&events, brokkr_core::fold(&events).ok().as_ref());
        for (column, expected) in read_as_columns(&read) {
            let select =
                format!("SELECT CAST({column} AS TEXT) FROM run_summary WHERE run_id = ?1");
            let found: Option<String> = store
                .conn
                .query_row(&select, [&run_id], |row| row.get(0))
                .unwrap();
            if found != expected && !(run_id == BROKEN && found.is_none()) {
                kept.push(format!("{run_id}: {column} {found:?}, read {expected:?}"));
            }
        }
    }
    assert_eq!(kept, Vec::<String>::new());
    let read = |column: &str, run_id: &str| -> Option<String> {
        let select = format!("SELECT CAST({column} AS TEXT) FROM run_summary WHERE run_id = ?1");
        store
            .conn
            .query_row(&select, [run_id], |row| row.get(0))
            .unwrap()
    };
    let reforged = "reforging-the-road-back-hand-built";
    let words = ["ending", "implement_visits", "review_first", "review_back"];
    let reforging: Vec<Option<String>> = words.iter().map(|word| read(word, reforged)).collect();
    // Three `phase/entered` of implement, read by `grep -o` over its lines.
    let shipped = |word: &str| Some(word.to_string());
    assert_eq!(
        reforging,
        [shipped("shipped"), shipped("3"), shipped("1"), shipped("1")]
    );
}

/// An import's inserts are indexed by the trigger as an append's are: a
/// fixture's trail is its every event but its checkpoints, read from the
/// journal's own lines.
#[test]
fn an_import_is_indexed_as_its_inserts_land() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    import_fixtures(&mut store);
    let run = "reforging-the-road-back-hand-built";
    let ndjson = std::fs::read_to_string(fixtures().join(format!("{run}.ndjson"))).unwrap();
    let trail: Vec<u64> = ndjson
        .lines()
        .map(|line| serde_json::from_str::<EventEnvelope>(line).unwrap())
        .filter(|event| event.event_type != EventType::EffectCheckpointed)
        .map(|event| event.seq)
        .collect();
    assert_eq!(trail.len(), 58);
    assert_eq!(store.index_trail(run).unwrap(), trail);
}

const BROKEN: &str = "conclude-broken-chain-hand-built";

/// The doctor's sweep names `BROKEN`, and it alone, as a run that does
/// not load at its bad event, its row standing at `place`.
fn assert_only_the_broken_chain_is_named(store: &Store, place: &str) {
    let found = store.summary_disagreements().unwrap();
    assert!(
        matches!(
            found.as_slice(),
            [Disagreement::Unread {
                run_id,
                error: StoreError::Chain(ChainError::BadHash { seq: 11 }),
            }] if run_id == BROKEN
        ),
        "{place}: {found:?}"
    );
}

/// The broken chain, planted as an older writer's events, is named by the
/// doctor's sweep (decision 0089 ruling 7) behind its head, where no
/// writer's catch-up can fold it, and at its head, as a row folded before
/// the chain broke beneath it would stand.
#[test]
fn a_run_that_does_not_load_is_named_behind_its_head_and_at_it() {
    let ndjson = std::fs::read_to_string(fixtures().join(format!("{BROKEN}.ndjson"))).unwrap();
    let events: Vec<EventEnvelope> = ndjson
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    plant_as_older_writer(&Store::open(&path).unwrap(), BROKEN, &events);
    let store = Store::open(&path).unwrap();
    assert_eq!(lane(&store, BROKEN).as_deref(), Some("behind"));
    assert_only_the_broken_chain_is_named(&store, "behind its head");
    store
        .conn
        .execute(
            "UPDATE run_summary SET folded_seq = head_seq, folded_hash = head_hash,
                 aggregated_seq = head_seq",
            [],
        )
        .unwrap();
    assert_ne!(lane(&store, BROKEN).as_deref(), Some("behind"));
    assert_only_the_broken_chain_is_named(&store, "at its head");
}
