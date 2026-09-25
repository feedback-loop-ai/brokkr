//! The heap budget for projecting a DshSession at the reader's source cap (#342).

mod heap_support;

/// 4,197,436 bytes at its peak on 2026-09-26, with a tenth of headroom.
const BUDGET: u64 = 4_617_180;

#[test]
fn projecting_the_largest_admitted_source_stays_within_the_heap_budget() {
    heap_support::assert_within(brokkr_view::transcript::TranscriptKind::DshSession, BUDGET);
}
