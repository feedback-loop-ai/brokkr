//! Decision 0089 ruling 1: the engine decides on its own fold, never on a
//! summary row, an event index row or a seat row. The read model's readers
//! are every public function of the store's summary, event index and seat
//! summary modules but the writer's hand-over, read from those modules'
//! sources so a reader added later is held here without an edit, and the
//! tables themselves by name;
//! no `brokkr-runtime` production source names one. A test source may,
//! since no decision runs in it.

use std::path::{Path, PathBuf};

use crate::workspace_root::{read, workspace};

/// The one public function of the summary module that is the writer's:
/// the account a writer hands the store to carry into its appends.
const WRITER: &str = "carry";

/// The summary module's public functions, by name: `pub fn NAME`, its
/// generics or parameters after it.
fn summary_functions(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("pub fn "))
        .map(|rest| {
            let end = rest.find(['(', '<']).unwrap_or(rest.len());
            rest[..end].to_string()
        })
        .collect()
}

/// Is `path` a test source: a `tests.rs`, a `*_tests.rs`, or beneath a
/// directory named `tests`?
fn test_source(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    name == "tests.rs"
        || name.ends_with("_tests.rs")
        || path.components().any(|part| part.as_os_str() == "tests")
}

fn sources(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// Each line of each production source under `root` that names one of
/// `names`, as `path:line: text`.
fn naming(root: &Path, names: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    sources(root, &mut found);
    found.sort();
    let mut named = Vec::new();
    for path in found.iter().filter(|path| !test_source(path)) {
        let text = std::fs::read_to_string(path).unwrap();
        for (at, line) in text.lines().enumerate() {
            if names.iter().any(|name| line.contains(name.as_str())) {
                let shown = path.strip_prefix(root).unwrap().display();
                named.push(format!("{shown}:{}: {}", at + 1, line.trim()));
            }
        }
    }
    named
}

/// The read model's readers: the tables, and every public function of the
/// summary, event index and seat summary modules but the writer's, each
/// as a call.
fn readers() -> Vec<String> {
    let tables = ["run_summary", "event_index", "seat_summary", "seat_head"];
    let mut readers: Vec<String> = tables.iter().map(|table| table.to_string()).collect();
    for module in ["summary", "event_index", "seat_summary"] {
        let source = read(&format!("crates/brokkr-store/src/{module}.rs"));
        readers.extend(
            summary_functions(&source)
                .into_iter()
                .filter(|name| name != WRITER)
                .map(|name| format!("{name}(")),
        );
    }
    readers
}

#[test]
fn no_runtime_decision_names_a_projection_s_reader() {
    let readers = readers();
    assert_eq!(
        readers,
        [
            "run_summary",
            "event_index",
            "seat_summary",
            "seat_head",
            "summary_disagreements(",
            "reads_index(",
            "index_trail(",
            "index_checkpoints(",
            "index_causes(",
            "envelopes_at(",
            "seat_rows(",
        ]
    );
    let named = naming(&workspace().join("crates/brokkr-runtime/src"), &readers);
    assert_eq!(named, Vec::<String>::new());
}

/// The scan reads what it is pointed at: a planted call in a production
/// source is named, and the same call in a test source is not.
#[test]
fn the_projection_scan_names_a_planted_reader_and_passes_a_test() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("engine/tests")).unwrap();
    let planted = "let rows = store.summary_disagreements()?;";
    for file in [
        "engine.rs",
        "engine/tests.rs",
        "engine/a_tests.rs",
        "engine/tests/x.rs",
    ] {
        std::fs::write(dir.path().join(file), format!("// a decision\n{planted}\n")).unwrap();
    }
    std::fs::write(dir.path().join("engine/notes.txt"), planted).unwrap();
    assert_eq!(
        naming(dir.path(), &readers()),
        [format!("engine.rs:2: {planted}")]
    );
    assert_eq!(
        summary_functions("  pub fn carry(&mut self)\n pub fn page<'a>(x)\npub fn bare"),
        ["carry", "page", "bare"]
    );
}
