//! Decision 0082 ruling 4: the `effect-provenance/v1` row names this
//! module's private reader, and the reader admits an instance of the version.

use serde_json::json;

use super::{facts, Provider, Seats};
use crate::EventType;

#[test]
fn the_lifecycle_row_names_the_provenance_reader_and_a_v1_start_loads() {
    let row = crate::contract_lifecycle::row("contracts/effect-provenance.v1.schema.json");
    assert_eq!(row.version, "effect-provenance/v1");
    assert_eq!(row.reader, "load brokkr_core::seat::row::Seats::note");
    let mut seats = Seats::default();
    seats.note(&crate::contract_lifecycle::event(
        EventType::EffectRequested,
        json!({"effect_id": "e"}),
        1,
    ));
    seats.note(&crate::contract_lifecycle::event(
        EventType::EffectStarted,
        json!({
            "effect_id": "e",
            "attempt_id": "a",
            // Every field `contracts/effect-provenance.v1.schema.json`
            // requires; this crate's test build does no I/O, so the v1
            // instance is completed by hand against the frozen schema.
            "provenance": [{
                "member": "m",
                "agent": "agent",
                "model": "model",
                "provider": "p",
                "chain_index": 0
            }]
        }),
        2,
    ));
    let facts = facts(seats.rows());
    assert_eq!(
        facts
            .iter()
            .find(|facts| facts.provider == Some(Provider::Named("p".into())))
            .map(|facts| facts.member.clone()),
        Some(crate::seat::row::Member::Named("m".into())),
        "the v1 provenance entry was not read"
    );
}
