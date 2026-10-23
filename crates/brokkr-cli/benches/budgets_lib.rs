//! The CPU budgets for Brokkr's hot paths (#342): instruction counts under
//! Callgrind, which do not vary with the machine's load the way wall clock
//! does. CI runs these on Linux against the merge-base in the same job and
//! fails a regression over 2% (`--callgrind-limits='ir=2%'`). Valgrind runs
//! nowhere else, so nothing here is compiled into a test run.

use std::hint::black_box;
use std::path::PathBuf;

use brokkr_core::envelope::EventEnvelope;
use brokkr_core::fold::{fold, fold_onto, RunState};
use brokkr_core::realms::Boundary;
use brokkr_runtime::dialect::Dialect;
use brokkr_runtime::Bundle;
use brokkr_store::test_support::Width;
use gungraun::{library_benchmark, library_benchmark_group, main};

#[path = "../tests/heap/grown.rs"]
mod grown;

use grown::{journal, EVENTS};

/// The clock the view derivation takes as an argument.
const NOW: &str = "2026-09-26T00:00:00Z";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The state the grown journal's first 7,509 events fold to, and its last
/// event: what one engine turn folds since #354, which carries the state
/// it holds over what landed since instead of folding the run again.
fn turn() -> (RunState, Vec<EventEnvelope>) {
    let mut events = journal();
    let last = events.split_off(EVENTS - 1);
    (fold(&events).expect("the grown journal folds"), last)
}

fn folded() -> (Vec<EventEnvelope>, RunState) {
    let events = journal();
    let state = fold(&events).expect("the grown journal folds");
    (events, state)
}

/// `recipes/self` in the self realm, as `brokkr compile` reads it.
fn self_bundle() -> (PathBuf, Dialect) {
    let dialect = Dialect::load(&root().join("dialects/openspec.json"))
        .expect("the openspec dialect loads")
        .0;
    (root().join("recipes/self"), dialect)
}

#[library_benchmark]
#[bench::events_7510(setup = journal)]
fn fold_journal(events: Vec<EventEnvelope>) -> RunState {
    black_box(fold(black_box(&events)).expect("the grown journal folds"))
}

#[library_benchmark]
#[bench::events_7510(setup = turn)]
fn fold_turn(turn: (RunState, Vec<EventEnvelope>)) -> RunState {
    let (state, suffix) = turn;
    black_box(fold_onto(black_box(state), black_box(&suffix)).expect("the last event folds"))
}

#[library_benchmark]
#[bench::events_7510(setup = folded)]
fn run_view(journal: (Vec<EventEnvelope>, RunState)) -> brokkr_view::RunView {
    let (events, state) = journal;
    black_box(brokkr_view::run_view(black_box(&events), Some(&state)))
}

#[library_benchmark]
#[bench::bundles_self(setup = self_bundle)]
fn compile_bundle(bundle: (PathBuf, Dialect)) -> usize {
    let (dir, dialect) = bundle;
    let root = root();
    let compiled = Bundle::compile_with_realm(
        black_box(&dir),
        &root.join("agents"),
        &root.join("adapters"),
        Some("brokkr"),
        Some(&dialect),
        Boundary::Namespace,
    )
    .expect("recipes/self compiles in the self realm");
    black_box(compiled.seats.len())
}

// One RUN-level frame at 160×48: the fold, the run view and the draw into
// `TestBackend`, which is what one console redraw of a long run costs.
#[library_benchmark]
#[bench::events_7510_160x48(setup = journal)]
fn tui_frame(events: Vec<EventEnvelope>) -> ratatui::buffer::Buffer {
    black_box(brokkr_cli::run_frame_for_budget(
        black_box(&events),
        NOW,
        160,
        48,
    ))
}

/// A generated journal of 2,000 runs of `events` each (decision 0089
/// ruling 9), and the directory that holds it while it is read.
fn fleet_journal(events: u64) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let db = dir.path().join("forge.db");
    let store = brokkr_store::Store::open(&db).expect("a fresh journal opens");
    let runs = brokkr_store::test_support::fleet(NOW, events, Width::Wide);
    brokkr_store::test_support::generate(&store, &runs).expect("the journal is generated");
    (dir, db)
}

fn fleet_10k() -> (tempfile::TempDir, PathBuf) {
    fleet_journal(5)
}

fn fleet_1m() -> (tempfile::TempDir, PathBuf) {
    fleet_journal(500)
}

// The console's first fleet page at 80×24, open to painted frame: the
// summary's index read a page and the one ahead, each row checked against
// its run's head, and drawn. Its counts are the same at 10,000 events as
// at 1,000,000 (decision 0089 ruling 9).
#[library_benchmark]
#[bench::events_10k(setup = fleet_10k)]
#[bench::events_1m(setup = fleet_1m)]
fn fleet_page(journal: (tempfile::TempDir, PathBuf)) -> String {
    let (_dir, db) = journal;
    black_box(
        brokkr_cli::fleet_frame_for_budget(black_box(&db), NOW, 80, 24)
            .expect("the first page reads"),
    )
}

/// A journal holding the grown run alone, its events chained, and the
/// directory that holds it while it is read.
fn chained_journal() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let db = dir.path().join("forge.db");
    drop(brokkr_store::Store::open(&db).expect("a fresh journal opens"));
    grown::planted(&db, &grown::chained(journal()));
    (dir, db)
}

// Opening the grown run (decision 0089 ruling 5): its read-only open, and
// its chain read in seq order and verified one envelope at a time, at
// ruling 7's rate of about 2.5 µs an event.
#[library_benchmark]
#[bench::events_7510(setup = chained_journal)]
fn open_run(journal: (tempfile::TempDir, PathBuf)) -> brokkr_store::Opened {
    let (_dir, db) = journal;
    let store = brokkr_store::Store::open_read_only(black_box(&db)).expect("the journal opens");
    black_box(
        store
            .open_run(black_box(grown::RUN))
            .expect("the grown run opens"),
    )
}

library_benchmark_group!(
    name = hot_paths;
    benchmarks = fold_journal, fold_turn, run_view, compile_bundle, tui_frame, fleet_page, open_run
);

main!(library_benchmark_groups = hot_paths);
