use super::*;
use brokkr_core::entry::{ContinuedRun, RunEntry};

#[test]
fn the_entry_is_one_sentence_led_by_each_title() {
    let entry = RunEntry {
        phase: "verify".into(),
        continues: vec![
            ContinuedRun {
                run_id: "a-12345678".into(),
                feature: "land the parked slice\nsecond line\nthird".into(),
            },
            ContinuedRun {
                run_id: "b-abcdef01".into(),
                feature: "other work".into(),
            },
        ],
    };
    assert_eq!(
        entry_sentence(&entry),
        "entered at verify, continuing land the parked slice (a-12345678), other work (b-abcdef01)"
    );
    let events = vec![ev(1, EventType::RunStarted, json!({"entry":entry}), T0)];
    let view = run_view(&events, None);
    assert_eq!(
        view.entry,
        Some(EntryView::Entered {
            phase: "verify".into(),
            text: entry_sentence(&entry),
            continues: vec![
                ContinuedView {
                    title: "land the parked slice".into(),
                    feature: "land the parked slice\nsecond line\nthird".into(),
                    run_id: "a-12345678".into()
                },
                ContinuedView {
                    title: "other work".into(),
                    feature: "other work".into(),
                    run_id: "b-abcdef01".into()
                },
            ],
        })
    );
    assert_eq!(
        entry_sentence(&RunEntry {
            phase: "review".into(),
            continues: vec![]
        }),
        "entered at review, continuing no earlier run"
    );
}

#[test]
fn an_absent_entry_is_absent_and_an_unreadable_entry_is_stated() {
    let ordinary = vec![ev(1, EventType::RunStarted, json!({}), T0)];
    assert_eq!(run_view(&ordinary, None).entry, None);
    let events = vec![ev(
        1,
        EventType::RunStarted,
        json!({"entry":entry_records::invalid_records()[1].1}),
        T0,
    )];
    let error = brokkr_core::entry::RunEntry::of_started(&events[0].payload)
        .unwrap_err()
        .to_string();
    let view = run_view(&events, None);
    assert_eq!(
        view.entry,
        Some(EntryView::Unreadable {
            text: format!("entry unreadable: {error}"),
            error
        })
    );
    let before = run_view(&ordinary, None);
    assert_eq!(
        serde_json::to_value(view.phases).unwrap(),
        serde_json::to_value(before.phases).unwrap()
    );
    assert_eq!(
        serde_json::to_value(view.participants).unwrap(),
        serde_json::to_value(before.participants).unwrap()
    );
}
