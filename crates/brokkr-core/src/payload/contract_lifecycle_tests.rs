//! Decision 0082 ruling 4: the `phase-entered-head/v1` row names this
//! module's private reader, and the reader admits an instance of the version.

use serde_json::json;

use super::PhaseEntered;
use crate::EventType;
#[test]
fn the_lifecycle_row_names_the_phase_entered_reader_and_a_v1_head_loads() {
    let row = crate::contract_lifecycle::row("contracts/phase-entered-head.v1.schema.json");
    assert_eq!(row.version, "phase-entered-head/v1");
    assert_eq!(row.reader, "load brokkr_core::payload::PhaseEntered::read");
    let head = "0123456789abcdef0123456789abcdef01234567";
    let event = crate::contract_lifecycle::event(
        EventType::PhaseEntered,
        json!({"phase": "implement", "head": head}),
        1,
    );
    let entered = PhaseEntered::read(&event).expect("a v1 head loads");
    assert_eq!(entered.phase, "implement");
    let value = serde_json::to_value(&entered).unwrap();
    assert_eq!(value["head"], head, "the v1 head was not kept");
}
