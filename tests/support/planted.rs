//! Journals planted as an older writer leaves them (#753; decision 0089):
//! every committed journal, the fixtures and the evidence, each re-chained
//! under its own run id, and any journal of runs planted into a fresh
//! store and caught up by a writable open. brokkr-cli's seat view parity
//! and its run readers' parity read journals so.

use std::path::Path;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::EventEnvelope;
use brokkr_store::test_support::plant_older_writer;
use brokkr_store::Store;

/// `journals` planted into a fresh journal at `path` as an older writer
/// leaves them, each run under its own id, and caught up by a writable
/// open: their run ids.
pub(crate) fn planted(path: &Path, journals: Vec<(String, Vec<EventEnvelope>)>) -> Vec<String> {
    let store = Store::open(path).unwrap();
    let mut runs = Vec::new();
    for (run, events) in journals {
        plant_older_writer(&store, &run, &events);
        runs.push(run);
    }
    drop(store);
    drop(Store::open(path).unwrap());
    runs
}

/// A journal's events as run `run`'s, each with an id of its own, sealed
/// into one chain, as a writer's run verifies.
pub(crate) fn chained(run: &str, events: Vec<EventEnvelope>) -> Vec<EventEnvelope> {
    let mut previous = ZERO_HASH.to_string();
    let seal = |event: EventEnvelope| {
        let sealed = EventEnvelope {
            run_id: run.to_string(),
            correlation_id: run.to_string(),
            event_id: format!("{run}-{}", event.seq),
            previous_hash: previous.clone(),
            ..event
        }
        .sealed();
        previous.clone_from(&sealed.event_hash);
        sealed
    };
    events.into_iter().map(seal).collect()
}

/// Every committed journal, the fixtures and the evidence, each by its
/// file's name and the events its lines hold.
fn committed() -> Vec<(String, Vec<EventEnvelope>)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let listed = ["fixtures/journals", "docs/evidence"].map(|dir| root.join(dir).read_dir());
    let mut paths: Vec<_> = listed.into_iter().flat_map(Result::unwrap).collect();
    paths.sort_by_key(|entry| entry.as_ref().unwrap().path());
    let mut journals = Vec::new();
    for path in paths.into_iter().map(|entry| entry.unwrap().path()) {
        let name = path.file_name().unwrap().to_str().unwrap();
        let Some(name) = name.strip_suffix(".ndjson") else {
            continue;
        };
        let text = std::fs::read_to_string(&path).unwrap();
        let lines = text.lines().filter(|line| !line.trim().is_empty());
        let events = lines.map(|line| serde_json::from_str(line).unwrap());
        journals.push((name.to_string(), events.collect()));
    }
    journals
}

/// Every committed journal re-chained under its own id and planted into a
/// fresh journal at `path`: their run ids.
pub(crate) fn committed_at(path: &Path) -> Vec<String> {
    let committed = committed().into_iter();
    let committed = committed.map(|(name, events)| (name.clone(), chained(&name, events)));
    planted(path, committed.collect())
}
