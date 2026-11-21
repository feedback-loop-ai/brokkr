use super::*;
use serde_json::json;

/// v8's transcript definition alone, from the published contract.
fn contract_admits(kind: &str) -> bool {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/seat-record.v8.schema.json"
    );
    let mut schema: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    schema.as_object_mut().unwrap().remove("oneOf");
    schema["$ref"] = json!("#/$defs/transcript");
    let validator = jsonschema::draft7::new(&schema).unwrap();
    validator.is_valid(&json!({"kind": kind, "locator": "", "home": ""}))
}

/// The byte check is the contract's pattern and bound, case by case.
#[test]
fn the_kind_grammar_is_the_contracts() {
    let at_bound = format!("k{}", "-9".repeat(19)) + "9";
    let over = format!("{at_bound}9");
    let cases = [
        ("claude-session", true),
        ("none", true),
        ("a", true),
        ("a1-b2-c3", true),
        (at_bound.as_str(), true),
        (over.as_str(), false),
        ("", false),
        ("Trail", false),
        ("9lives", false),
        ("-trail", false),
        ("trail-", false),
        ("trail--log", false),
        ("trail_log", false),
        ("trail.log", false),
        ("tráil", false),
    ];
    for (kind, admitted) in cases {
        assert_eq!(kind_word(kind), admitted, "{kind}");
        assert_eq!(contract_admits(kind), admitted, "{kind}");
    }
}

fn declaring(transcript: &[&str], root_session: &[&str]) -> DeclaredKinds {
    let set = |kinds: &[&str]| kinds.iter().map(ToString::to_string).collect();
    DeclaredKinds {
        transcript: set(transcript),
        root_session: set(root_session),
    }
}

/// A declared, a built-in driver's and `none` transcript kind are
/// admitted; an undeclared one in either member is refused naming the
/// seat, the member and the kind; a kind outside the grammar is left to
/// the schema. Each member is declared apart: a transcript kind does
/// not declare a root session kind.
#[test]
fn the_writer_admits_what_is_declared_and_refuses_the_rest_by_name() {
    let kinds = declaring(&["trellis-trail"], &["trellis-thread"]);
    let record = |transcript: &str, root: &str| {
        json!({"step": "s", "transcript": {"kind": transcript},
               "root_session": {"kind": root}})
    };
    for (transcript, root) in [
        ("trellis-trail", "trellis-thread"),
        ("codex-thread", "dsh-session"),
        ("none", "claude-session"),
        ("Not A Kind", "trellis-thread"),
    ] {
        assert_eq!(kinds.admit(&record(transcript, root), "work"), Ok(()));
    }
    let refused = |member, kind: &str| {
        Err(UndeclaredKind {
            seat: "work".into(),
            member,
            kind: kind.into(),
        })
    };
    assert_eq!(
        kinds.admit(&record("trellis-thread", "trellis-thread"), "work"),
        refused(Member::Transcript, "trellis-thread")
    );
    assert_eq!(
        kinds.admit(&record("trellis-trail", "trellis-trail"), "work"),
        refused(Member::RootSession, "trellis-trail")
    );
    assert_eq!(
        kinds.admit(
            &json!({"result": "complete", "transcript": {"kind": "stray"}}),
            "work"
        ),
        refused(Member::Transcript, "stray")
    );
    assert_eq!(
        DeclaredKinds::default().admit(&record("trellis-trail", "dsh-session"), "work"),
        refused(Member::Transcript, "trellis-trail")
    );
    let text = refused(Member::RootSession, "stray")
        .unwrap_err()
        .to_string();
    assert_eq!(
        text,
        "seat 'work' recorded root session kind 'stray', which no loaded adapter declares \
         (decision 0075 ruling 2)"
    );
}
