//! The heap budget for projecting an AgyConversation at the reader's source
//! cap (#1227 U9). The budget lives in `quality/heap-bytes.json`.

#[path = "heap/support.rs"]
mod heap_support;

#[test]
fn projecting_the_largest_admitted_source_stays_within_the_heap_budget() {
    heap_support::assert_within(brokkr_view::transcript::TranscriptKind::AgyConversation);
}
