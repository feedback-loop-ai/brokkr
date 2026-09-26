//! The CPU budgets for Brokkr's hot paths (#342): instruction counts under
//! Callgrind, which do not vary with the machine's load the way wall clock
//! does. CI runs these on Linux against the merge-base in the same job and
//! fails a regression over 2% (`--callgrind-limits='ir=2%'`). Valgrind runs
//! nowhere else, so nothing here is compiled into a test run.

use std::hint::black_box;
use std::path::PathBuf;

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::{fold, RunState};
use brokkr_core::realms::Boundary;
use brokkr_runtime::dialect::Dialect;
use brokkr_runtime::Bundle;
use gungraun::{library_benchmark, library_benchmark_group, main};

/// A real exported run: 105 events, 88 of them checkpoints.
const FIXTURE: &str =
    include_str!("../../../fixtures/journals/tui-graph-the-selection-box-gets-80f98deb.ndjson");

/// The largest run in the operator's journal when #342 was filed. The fold
/// and the view are measured at the scale a long run makes them meet.
const EVENTS: usize = 7_510;

/// The clock the view derivation takes as an argument.
const NOW: &str = "2026-09-26T00:00:00Z";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The fixture grown to `EVENTS` events by repeating each checkpoint in
/// place and renumbering: a long-running seat's journal is mostly its
/// checkpoints. Fold checks protocol shape, not the hash chain (callers
/// verify the chain first), so the grown journal folds as the fixture does.
fn journal() -> Vec<EventEnvelope> {
    let fixture: Vec<EventEnvelope> = FIXTURE
        .lines()
        .map(|line| serde_json::from_str(line).expect("the fixture is an exported journal"))
        .collect();
    let is_checkpoint = |event: &EventEnvelope| event.event_type == EventType::EffectCheckpointed;
    let checkpoints = fixture.iter().filter(|event| is_checkpoint(event)).count();
    let spare = EVENTS - (fixture.len() - checkpoints);
    let mut remainder = spare % checkpoints;
    let mut grown = Vec::with_capacity(EVENTS);
    for event in fixture {
        let copies = match is_checkpoint(&event) {
            true => spare / checkpoints + usize::from(remainder > 0),
            false => 1,
        };
        remainder = remainder.saturating_sub(usize::from(is_checkpoint(&event)));
        for _ in 0..copies {
            let mut copy = event.clone();
            copy.seq = grown.len() as u64 + 1;
            grown.push(copy);
        }
    }
    assert_eq!(
        grown.len(),
        EVENTS,
        "the grown journal has the budget's size"
    );
    grown
}

fn folded() -> (Vec<EventEnvelope>, RunState) {
    let events = journal();
    let state = fold(&events).expect("the grown journal folds");
    (events, state)
}

/// `bundles/self` in the self realm, as `brokkr compile` reads it.
fn self_bundle() -> (PathBuf, Dialect) {
    let dialect = Dialect::load(&root().join("dialects/openspec.json"))
        .expect("the openspec dialect loads")
        .0;
    (root().join("bundles/self"), dialect)
}

#[library_benchmark]
#[bench::events_7510(setup = journal)]
fn fold_journal(events: Vec<EventEnvelope>) -> RunState {
    black_box(fold(black_box(&events)).expect("the grown journal folds"))
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
    .expect("bundles/self compiles in the self realm");
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

library_benchmark_group!(
    name = hot_paths;
    benchmarks = fold_journal, run_view, compile_bundle, tui_frame
);

main!(library_benchmark_groups = hot_paths);
