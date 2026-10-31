//! The operator trail's rows: each names its command and seq, and a
//! disposition that names no command still gets a row.

use serde_json::json;

use super::*;
use crate::run_view;
use crate::tests::ev;

/// An accepted disposition whose payload names no command still gets a
/// trail row: it says the unknown command `?` and its own seq, never a
/// command it cannot name (decision 0071 ruling 9).
#[test]
fn an_operator_disposition_without_a_command_id_still_gets_a_row() {
    let at = "2026-01-01T00:00:00Z";
    let events = [
        ev(1, EventType::RunStarted, json!({"manifest": {}}), at),
        ev(2, EventType::OperatorAccepted, json!({}), at),
    ];
    let view = run_view(&events, None);
    let row = view
        .journal
        .iter()
        .find(|row| row.seq == 2)
        .expect("the disposition is on the trail");
    assert_eq!(row.what.text, "? · accepted at seq 2");
}
