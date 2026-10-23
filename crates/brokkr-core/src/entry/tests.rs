use super::*;
use serde_json::json;

#[test]
fn entry_names_are_bounded_identifiers() {
    for (name, expected) in [
        ("verify".into(), true),
        ("".into(), false),
        ("v".repeat(64), true),
        ("v".repeat(65), false),
        ("-v".into(), false),
        ("Verify".into(), false),
        ("v v".into(), false),
        ("v\n".into(), false),
        ("é".into(), false),
    ] {
        assert_eq!(is_entry_name(&name), expected, "{name:?}");
    }
}

#[test]
fn native_ids_match_the_published_language() {
    for (id, expected) in [
        ("-0a1b2c3d".into(), true),
        (format!("{}-0a1b2c3d", "a".repeat(32)), true),
        (format!("{}-0a1b2c3d", "a".repeat(33)), false),
        ("story-120-0a1b2c3d".into(), true),
        ("a-0a1b2c3".into(), false),
        ("a-0A1b2c3d".into(), false),
        ("A-0a1b2c3d".into(), false),
        ("a_b-0a1b2c3d".into(), false),
        ("forge-bound-run".into(), false),
    ] {
        assert_eq!(is_native_run_id(&id), expected, "{id}");
    }
}

#[test]
fn records_read_whole_and_check_their_language() {
    assert_eq!(RunEntry::of_started(&json!({})), Ok(None));
    assert_eq!(
        RunEntry::of_started(&json!({"entry":{"phase":"review","continues":[]}})),
        Ok(Some(RunEntry {
            phase: "review".into(),
            continues: vec![]
        }))
    );
    for (expected, raw) in crate::entry_records::invalid_records() {
        let error = RunEntry::of_started(&json!({"entry":raw})).unwrap_err();
        let kind = match error {
            EntryRead::Shape { .. } => "shape",
            EntryRead::Phase { .. } => "phase",
            EntryRead::RunId { .. } => "run_id",
            EntryRead::Twice { .. } => "twice",
        };
        assert_eq!(kind, expected, "{raw}");
    }
    assert_eq!(
        check_record("VERIFY", []),
        Err(EntryRead::Phase {
            phase: "VERIFY".into()
        })
    );
    assert_eq!(
        check_record("verify", ["bad"]),
        Err(EntryRead::RunId {
            run_id: "bad".into()
        })
    );
    let twice = EntryRead::Twice {
        run_id: "a-12345678".into(),
    };
    assert_eq!(
        check_record("verify", ["a-12345678", "a-12345678"]),
        Err(twice.clone())
    );
    assert_eq!(
        RunEntry::of_started(&json!({"entry":{"phase":"verify","continues":[
            {"run_id":"a-12345678","feature":"a"}, {"run_id":"a-12345678","feature":"b"}
        ]}})),
        Err(twice)
    );
    assert_eq!(check_record("verify", ["a-12345678", "b-abcdef01"]), Ok(()));
}

#[test]
fn errors_keep_the_operator_text() {
    assert_eq!(
        EntryRead::Shape {
            detail: "detail".into()
        }
        .to_string(),
        "entry has an unreadable shape: detail"
    );
    assert_eq!(
        EntryRead::Phase { phase: "V".into() }.to_string(),
        "entry phase 'V' is outside the entry-name grammar"
    );
    assert_eq!(
        EntryRead::RunId {
            run_id: "bad".into()
        }
        .to_string(),
        "continued run 'bad' is outside the native run-id grammar"
    );
    assert_eq!(
        EntryRead::Twice { run_id: "a".into() }.to_string(),
        "continued run 'a' is named twice"
    );
}
