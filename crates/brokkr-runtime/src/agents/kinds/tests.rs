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

/// Two fixture adapters, each declaring its own kinds, beside two that
/// declare none: the shipped LaneTally adapter and `plain`, a fixture
/// whose seats run a raw command.
fn loaded() -> DeclaredKinds {
    let adapter = |declared: Option<RecordKinds>| AdapterKinds {
        declared,
        rejoin: None,
    };
    let fixture = |transcript: &str, root: &str| {
        let root = RootKind {
            kind: root.into(),
            resume_shape: "headless".into(),
        };
        adapter(Some(RecordKinds {
            transcript: transcript.into(),
            root_session: Some(root),
        }))
    };
    DeclaredKinds(BTreeMap::from([
        ("trellis".into(), fixture("trellis-trail", "trellis-thread")),
        ("arbor".into(), fixture("arbor-log", "arbor-root")),
        ("lanetally".into(), adapter(None)),
        ("plain".into(), adapter(None)),
    ]))
}

/// The adapters `adapters` loaded from files, as a compile loads them.
fn loaded_from(adapters: &[Value]) -> DeclaredKinds {
    let dir = tempfile::tempdir().unwrap();
    for adapter in adapters {
        let name = format!("{}.json", adapter["provider"].as_str().unwrap());
        std::fs::write(dir.path().join(name), adapter.to_string()).unwrap();
    }
    DeclaredKinds::of(Some(&Adapters::load(dir.path()).unwrap()))
}

/// A loadable adapter of `provider` whose configured driver is `driver`,
/// with `extra` beside it.
fn adapter_file(provider: &str, driver: &[&str], extra: Value) -> Value {
    let mut adapter = json!({"provider": provider, "trust_tier": "trusted",
        "egress": "contracted", "binary": "sh", "driver": driver,
        "models": {provider: format!("{provider}-1")}, "model_flag": "--model",
        "efforts": ["high"], "effort_flag": "--effort",
        "tool_permissions": "unsupported", "mcp": "unsupported"});
    (adapter.as_object_mut().unwrap()).extend(extra.as_object().unwrap().clone());
    adapter
}

const BUILT_IN: [&str; 3] = ["claude-session", "codex-thread", "dsh-session"];

fn record(transcript: &str, root: &str) -> Value {
    json!({"step": "s", "transcript": {"kind": transcript}, "root_session": {"kind": root}})
}

fn refused(member: Member, kind: &str) -> Result<(), UndeclaredKind> {
    refused_as(member, Recorded::Word(kind.into()))
}

/// The refusal of seat `work`'s `member` recording a kind outside the
/// grammar.
fn outside(member: Member) -> Result<(), UndeclaredKind> {
    refused_as(member, Recorded::Outside)
}

fn refused_as(member: Member, kind: Recorded) -> Result<(), UndeclaredKind> {
    Err(UndeclaredKind::Named {
        seat: "work".into(),
        member,
        kind,
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
    ] {
        assert_eq!(seat.admit(&record(transcript, root), "work"), Ok(()));
    }
    let stray = record("Not A Kind", "trellis-thread");
    assert_eq!(trellis.admit(&stray, "work"), outside(Member::Transcript));
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
/// result, whose members are refused when undeclared and when outside the
/// grammar, without echoing them. A nested result that is not an object
/// names no kind place and keeps no sibling from being read.
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
            json!({"step": "s", "result": [kind("trellis-trail")], "transcript": kind("stray")}),
            Member::Transcript,
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
    let stray = json!({"step": "s", "result": {"transcript": kind("Not A Kind")}});
    let refusal = trellis.admit(&stray, "work");
    assert_eq!(refusal, outside(Member::ResultTranscript));
    assert_eq!(
        refusal.unwrap_err().to_string(),
        "seat 'work' recorded nested result's transcript kind outside seat-record v8's \
         grammar, which the seat's harness does not declare (decision 0075 ruling 2)"
    );
    let declared = json!({"step": "s", "result": {"transcript": kind("trellis-trail"),
                                                  "root_session": kind("trellis-thread")}});
    assert_eq!(trellis.admit(&declared, "work"), Ok(()));
}

/// A kind place is a declared kind or absent, and nothing else passes
/// (the controller's ruling R13 of 2026-11-22): a string outside the
/// grammar, a kind that is no string, a place that is not an object, an
/// array read as nothing by position, and `null` are each refused at the
/// top level as in a nested result, never echoed; a record that is not
/// an object is refused as unread.
#[test]
fn a_kind_place_that_is_not_a_declared_kind_is_refused_whatever_it_holds() {
    let trellis = loaded().seat(Some("trellis"), &argv(&["sh"]));
    let held = [
        json!({"kind": "trellis--stray"}),
        json!({"kind": 7}),
        json!({"locator": "t-1"}),
        json!("trellis-thread"),
        json!(["trellis-trail"]),
        json!(null),
    ];
    for place in held {
        for (record, member) in [
            (json!({"transcript": place}), Member::Transcript),
            (json!({"root_session": place}), Member::RootSession),
            (
                json!({"result": {"transcript": place}}),
                Member::ResultTranscript,
            ),
            (
                json!({"result": {"root_session": place}}),
                Member::ResultRootSession,
            ),
        ] {
            let refusal = trellis.admit(&record, "work");
            assert_eq!(refusal, outside(member), "{record}");
            assert!(!refusal.unwrap_err().to_string().contains("stray"));
        }
    }
    for record in [json!(7), json!("trellis-trail"), json!([{"kind": "x"}])] {
        let refusal = trellis.admit(&record, "work");
        let unread = UndeclaredKind::Unread {
            seat: "work".into(),
        };
        assert_eq!(refusal, Err(unread), "{record}");
    }
    assert_eq!(
        trellis.admit(&json!(7), "work").unwrap_err().to_string(),
        "seat 'work' recorded its record's root outside seat-record v8's grammar, so the kinds \
         it names cannot be read (decision 0075 ruling 2)"
    );
    assert_eq!(trellis.admit(&json!({"result": 7}), "work"), Ok(()));
}

/// The driver is the one the seat's own command runs (the controller's
/// ruling R1): an inline codex seat beside a `codex` adapter whose
/// configured driver dispatches claude records codex's kind, and never
/// claude's.
#[test]
fn a_seats_driver_is_the_one_its_own_command_runs_never_its_adapters() {
    let claude_driver = ["{brokkr}", "driver", "claude", "--"];
    let loaded = loaded_from(&[adapter_file("codex", &claude_driver, json!({}))]);
    let codex = loaded.seat(None, &argv(&["{brokkr}", "driver", "codex", "--"]));
    let thread = record("codex-thread", "codex-thread");
    assert_eq!(codex.admit(&thread, "work"), Ok(()));
    for (record, member) in [
        (record("claude-session", "codex-thread"), Member::Transcript),
        (
            record("codex-thread", "claude-session"),
            Member::RootSession,
        ),
    ] {
        let judged = codex.admit(&record, "work");
        assert_eq!(judged, refused(member, "claude-session"), "{record}");
    }
}

/// An adapter that declares no kinds grants none (the controller's ruling
/// R2): a seat of `plain`, whose command is raw, is refused each of the
/// three built-in kinds, as a transcript kind and as a root session kind,
/// and records `none`.
#[test]
fn an_adapter_that_declares_no_kinds_grants_none() {
    let plain = loaded().seat(Some("plain"), &argv(&["sh", "-c", "true"]));
    for kind in BUILT_IN {
        let transcript = json!({"step": "s", "transcript": {"kind": kind}});
        let judged = plain.admit(&transcript, "work");
        assert_eq!(judged, refused(Member::Transcript, kind));
        let root = json!({"step": "s", "root_session": {"kind": kind}});
        let judged = plain.admit(&root, "work");
        assert_eq!(judged, refused(Member::RootSession, kind));
    }
    let none = json!({"step": "s", "transcript": {"kind": "none"}});
    assert_eq!(plain.admit(&none, "work"), Ok(()));
    assert!(!plain.declares_its_kinds());
}

/// A Qoder seat writes no kind (the controller's ruling Y1 of 2026-11-22):
/// its level 1 runs `--no-session-persistence` and its shipped adapter
/// declares no `record_kinds`, so a seat answering to it records `none`
/// and is refused each of the three built-in kinds and a fourth, as a
/// transcript kind and as a root session kind.
#[test]
fn a_qoder_seat_records_none_and_no_kind_of_its_own() {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let shipped = workspace.join("docs/examples/adapters/qoder.json");
    let shipped = serde_json::from_str(&std::fs::read_to_string(shipped).unwrap()).unwrap();
    let driver = argv(&["{brokkr}", "driver", "qoder", "--"]);
    let qoder = loaded_from(&[shipped]).seat(Some("qoder"), &driver);
    let none = json!({"step": "s", "transcript": {"kind": "none"}});
    assert_eq!(qoder.admit(&none, "work"), Ok(()));
    for kind in BUILT_IN.into_iter().chain(["qoder-session"]) {
        let transcript = json!({"step": "s", "transcript": {"kind": kind}});
        let judged = qoder.admit(&transcript, "work");
        assert_eq!(judged, refused(Member::Transcript, kind));
        let root = json!({"step": "s", "root_session": {"kind": kind}});
        let judged = qoder.admit(&root, "work");
        assert_eq!(judged, refused(Member::RootSession, kind));
    }
}

/// Which adapter a site answers to (the controller's ruling R3): an inline
/// exec site consults none, so a loaded `exec` adapter's declaration buys
/// it nothing; an inline raw command answers to none and keeps v7's three
/// kinds, and no fourth.
#[test]
fn an_inline_exec_site_and_a_raw_command_answer_to_no_adapter() {
    let exec_driver = ["{brokkr}", "driver", "exec", "--"];
    let declares = json!({"record_kinds": {"transcript": "exec-log"}});
    let loaded = loaded_from(&[adapter_file("exec", &exec_driver, declares)]);
    let exec = loaded.seat(None, &argv(&exec_driver));
    let log = json!({"step": "s", "transcript": {"kind": "exec-log"}});
    let judged = exec.admit(&log, "work");
    assert_eq!(judged, refused(Member::Transcript, "exec-log"));
    assert!(!exec.declares_its_kinds());
    let raw = loaded.seat(None, &argv(&["sh", "-c", "true"]));
    for kind in BUILT_IN {
        assert_eq!(raw.admit(&record(kind, kind), "work"), Ok(()), "{kind}");
    }
    let fourth = record("exec-log", "codex-thread");
    let judged = raw.admit(&fourth, "work");
    assert_eq!(judged, refused(Member::Transcript, "exec-log"));
}
