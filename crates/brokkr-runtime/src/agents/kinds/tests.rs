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

fn argv(words: &[&str]) -> Vec<String> {
    words.iter().map(ToString::to_string).collect()
}

/// Two fixture harnesses on raw drivers, each declaring its own kinds,
/// beside the shipped LaneTally harness, which declares none and runs its
/// built-in driver.
fn loaded() -> DeclaredKinds {
    let fixture = |transcript: &str, root: &str| SeatKinds {
        declared: Some(RecordKinds {
            transcript: transcript.into(),
            root_session: Some(root.into()),
        }),
        driver: None,
        rejoins_work: false,
    };
    let lanetally = SeatKinds {
        driver: Some(AdapterKind::Lanetally),
        ..SeatKinds::default()
    };
    DeclaredKinds(BTreeMap::from([
        ("trellis".into(), fixture("trellis-trail", "trellis-thread")),
        ("arbor".into(), fixture("arbor-log", "arbor-root")),
        ("lanetally".into(), lanetally),
    ]))
}

fn record(transcript: &str, root: &str) -> Value {
    json!({"step": "s", "transcript": {"kind": transcript}, "root_session": {"kind": root}})
}

fn refused(member: Member, kind: &str) -> Result<(), UndeclaredKind> {
    Err(UndeclaredKind {
        seat: "work".into(),
        member,
        kind: kind.into(),
    })
}

/// Each seat is judged by its own harness alone (the controller's ruling
/// Q1): a fixture seat admits its own kinds and `none`, and refuses the
/// other fixture's and every built-in driver's; a LaneTally seat admits
/// claude's and no other driver's; a raw command no adapter declares
/// kinds for keeps v7's three. A transcript kind declares no root session
/// kind.
#[test]
fn each_seat_is_judged_by_its_own_harness_and_never_by_another() {
    let loaded = loaded();
    let raw = argv(&["sh", "-c", "true"]);
    let (trellis, arbor) = (
        loaded.seat(Some("trellis"), &raw),
        loaded.seat(Some("arbor"), &raw),
    );
    for (seat, transcript, root) in [
        (&trellis, "trellis-trail", "trellis-thread"),
        (&trellis, "none", "trellis-thread"),
        (&arbor, "arbor-log", "arbor-root"),
        (&trellis, "Not A Kind", "trellis-thread"),
    ] {
        assert_eq!(seat.admit(&record(transcript, root), "work"), Ok(()));
    }
    for (seat, transcript, root, member, kind) in [
        (
            &trellis,
            "arbor-log",
            "trellis-thread",
            Member::Transcript,
            "arbor-log",
        ),
        (
            &trellis,
            "trellis-trail",
            "arbor-root",
            Member::RootSession,
            "arbor-root",
        ),
        (
            &arbor,
            "trellis-trail",
            "arbor-root",
            Member::Transcript,
            "trellis-trail",
        ),
        (
            &arbor,
            "arbor-log",
            "trellis-thread",
            Member::RootSession,
            "trellis-thread",
        ),
        (
            &trellis,
            "codex-thread",
            "trellis-thread",
            Member::Transcript,
            "codex-thread",
        ),
        (
            &trellis,
            "trellis-trail",
            "claude-session",
            Member::RootSession,
            "claude-session",
        ),
        (
            &trellis,
            "trellis-thread",
            "trellis-thread",
            Member::Transcript,
            "trellis-thread",
        ),
    ] {
        let judged = seat.admit(&record(transcript, root), "work");
        assert_eq!(judged, refused(member, kind), "{transcript} {root}");
    }
    // An inline LaneTally seat finds the shipped adapter by its driver.
    let lanetally = loaded.seat(None, &argv(&["{brokkr}", "driver", "lanetally", "--"]));
    let claude = record("claude-session", "claude-session");
    assert_eq!(lanetally.admit(&claude, "work"), Ok(()));
    let codex = record("codex-thread", "claude-session");
    assert_eq!(
        lanetally.admit(&codex, "work"),
        refused(Member::Transcript, "codex-thread")
    );
    let unnamed = loaded.seat(None, &raw);
    assert_eq!(
        unnamed.admit(&record("dsh-session", "codex-thread"), "work"),
        Ok(())
    );
    assert_eq!(
        unnamed.admit(&record("trellis-trail", "codex-thread"), "work"),
        refused(Member::Transcript, "trellis-trail")
    );
    let text = refused(Member::RootSession, "stray")
        .unwrap_err()
        .to_string();
    assert_eq!(
        text,
        "seat 'work' recorded root session kind 'stray', which the seat's harness does not \
         declare (decision 0075 ruling 2)"
    );
}

/// The reader walks every place v8 admits a kind (the controller's ruling
/// Q2): the top level of a checkpoint or result, and a checkpoint's nested
/// result, whose members are refused when undeclared and, since the
/// schema does not judge an open nested result, when outside the grammar,
/// without echoing them. A member that does not read keeps no sibling
/// from being read.
#[test]
fn every_place_a_record_names_a_kind_is_read() {
    let trellis = loaded().seat(Some("trellis"), &argv(&["sh"]));
    let kind = |kind: &str| json!({"kind": kind});
    for (record, member) in [
        (
            json!({"step": "s", "transcript": kind("stray")}),
            Member::Transcript,
        ),
        (
            json!({"step": "s", "root_session": kind("stray")}),
            Member::RootSession,
        ),
        (
            json!({"result": "complete", "transcript": kind("stray")}),
            Member::Transcript,
        ),
        (
            json!({"step": "s", "result": {"result": "complete", "transcript": kind("stray")}}),
            Member::ResultTranscript,
        ),
        (
            json!({"step": "s", "result": {"root_session": kind("stray")}}),
            Member::ResultRootSession,
        ),
        (
            json!({"step": "s", "transcript": 7, "result": {"transcript": kind("stray")}}),
            Member::ResultTranscript,
        ),
        (
            json!({"step": "s", "result": [1], "root_session": kind("stray")}),
            Member::RootSession,
        ),
    ] {
        assert_eq!(
            trellis.admit(&record, "work"),
            refused(member, "stray"),
            "{record}"
        );
    }
    let outside = json!({"step": "s", "result": {"transcript": kind("Not A Kind")}});
    let refusal = trellis.admit(&outside, "work");
    assert_eq!(refusal, refused(Member::ResultTranscript, "Not A Kind"));
    assert_eq!(
        refusal.unwrap_err().to_string(),
        "seat 'work' recorded nested result's transcript kind outside seat-record v8's \
         grammar, which the seat's harness does not declare (decision 0075 ruling 2)"
    );
    let declared = json!({"step": "s", "result": {"transcript": kind("trellis-trail"),
                                                  "root_session": kind("trellis-thread")}});
    assert_eq!(trellis.admit(&declared, "work"), Ok(()));
}

/// A root of one of the three kinds a built-in driver writes rejoins as
/// today; any other only where the harness declares it as its root
/// session kind and declares a resume for a work site that is not
/// `unsupported` (the controller's ruling Q3).
#[test]
fn a_new_root_kind_rejoins_only_where_its_harness_declares_a_resume() {
    let mut trellis = loaded().seat(Some("trellis"), &argv(&["sh"]));
    for kind in ["claude-session", "codex-thread", "dsh-session"] {
        assert!(trellis.rejoins(kind), "{kind}");
    }
    assert!(!trellis.rejoins("trellis-thread"));
    trellis.rejoins_work = true;
    assert!(trellis.rejoins("trellis-thread"));
    assert!(!trellis.rejoins("trellis-trail"));
    assert!(!trellis.rejoins("arbor-root"));
}
