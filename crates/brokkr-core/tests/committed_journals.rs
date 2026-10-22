//! The committed journals, read once for every test that measures them:
//! the fold's integration tests, which reach it as a module of `it.rs`,
//! and the payload reader's unit tests, which include it through
//! `#[path]` (#345).

use brokkr_core::envelope::EventEnvelope;
use std::path::{Path, PathBuf};

/// The repository root, where the committed journals live.
pub(crate) fn repository() -> PathBuf {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    crates.parent().unwrap().to_path_buf()
}

/// Every committed journal, the fixtures and the evidence, in path order.
#[expect(
    clippy::disallowed_methods,
    reason = "the test reads the committed journals"
)]
pub(crate) fn committed_journals() -> Vec<(PathBuf, Vec<EventEnvelope>)> {
    let mut paths: Vec<PathBuf> = ["fixtures/journals", "docs/evidence"]
        .into_iter()
        .flat_map(|dir| std::fs::read_dir(repository().join(dir)).unwrap())
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "ndjson")
        })
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let ndjson = std::fs::read_to_string(&path).expect("a committed journal");
            let events = ndjson
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| serde_json::from_str(line).expect("canonical NDJSON"))
                .collect();
            (path, events)
        })
        .collect()
}
