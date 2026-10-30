//! Decision 0082 ruling 4: the `panel-carry/v1` row names this module's
//! private record reader, and the reader admits an instance of the version.

use serde_json::json;

#[test]
fn the_lifecycle_row_names_the_record_reader_and_a_v1_record_loads() {
    let row = crate::contract_lifecycle::row("contracts/panel-carry.v1.schema.json");
    assert_eq!(row.version, "panel-carry/v1");
    assert_eq!(
        row.reader,
        "load brokkr_runtime::engine::carry::facts::terminal_record"
    );
    let head = "0123456789abcdef0123456789abcdef01234567";
    let payload = json!({
        "tree": {"at_start": "clean", "at_end": "clean"},
        "head": {"at_start": head, "at_end": head},
        "settled_members": {"m": {"result": 1}}
    });
    let event = crate::contract_lifecycle::event(brokkr_core::EventType::EffectFailed, payload, 1);
    let record = super::terminal_record(&event.payload, "attempt").expect("a v1 record loads");
    assert!(record.is_some(), "the v1 panel record was not read");
}
