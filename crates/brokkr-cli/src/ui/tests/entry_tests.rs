use super::*;
use crate::tests::envelope_builder::EnvelopeBuilder;

#[test]
fn the_page_consumes_the_shared_entry_text_on_one_line() {
    for (entry, expected) in [
        (json!({"phase":"verify","continues":[{"run_id":"a-12345678",
            "feature":"land the parked slice\nsecond\nthird"}]}),
         "entered at verify, continuing land the parked slice (a-12345678)"),
        (json!("verify"),
         "entry unreadable: entry has an unreadable shape: invalid type: string \"verify\", expected struct RunEntry"),
    ] {
        let event = EnvelopeBuilder::new(EventType::RunStarted,json!({"entry":entry})).build();
        let view = brokkr_view::run_view(&[event],None);
        assert_eq!(serde_json::to_value(view).unwrap()["entry"]["text"],expected);
        assert!(!expected.contains('\n'));
        assert!(PAGE.contains("if (view.entry) detail.appendChild(el('p', null, view.entry.text));"));
    }
}
