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

/// `checkpoint` as `seat_row` hands it to the engine: run through the
/// real mask with `value` bound to the name `K`.
pub(crate) fn masked_by(mut checkpoint: Value, value: &str) -> Value {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("secrets.env");
    brokkr_protocol::secret::store_set(&store, "K", value).unwrap();
    let bindings = brokkr_protocol::secret::resolve_bindings(&store, &["K".into()]).unwrap();
    brokkr_protocol::secret::mask_json(&mut checkpoint, &bindings);
    checkpoint
}

#[test]
fn named_reads_masked_keys_at_the_record_and_result_only() {
    let kinds = loaded().seat(Some("trellis"), &argv(&["sh"]));
    let transcript = json!({"kind": "trellis-trail"});
    let nested = json!({"result": {"transcript": transcript}});
    let top = json!({"transcript": transcript});
    let masked = UndeclaredKind::Masked {
        seat: "work".into(),
    };
    for record in [nested, top.clone()] {
        let record = masked_by(record, "transcript");
        assert_eq!(
            Named::places(&record, "work").map(drop),
            Err(masked.clone())
        );
    }
    assert_eq!(
        masked.to_string(),
        "seat 'work' recorded a masked key where kind places are read (decision 0075 ruling 2)"
    );
    let notes = json!({"result": {"notes": {"transcript": transcript}}});
    let deep = masked_by(notes, "transcript");
    assert_eq!(Named::places(&deep, "work").map(drop), Ok(()));
    assert_eq!(kinds.admit(&deep, "work"), Ok(()));
    assert_eq!(
        kinds.admit(&masked_by(top, "kind"), "work"),
        outside(Member::Transcript)
    );
}

/// An agy seat declares `agy-conversation` for its transcript and no root
/// session kind (#1227, T1 & T2): it admits `agy-conversation` and `none`
/// as transcript kinds, refuses every other transcript kind and every root
/// session kind (including `agy-conversation`), and never rejoins.
#[test]
fn an_agy_seat_records_agy_conversation_and_none_and_no_root_session() {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let shipped = workspace.join("docs/examples/adapters/agy.json");
    let shipped = serde_json::from_str(&std::fs::read_to_string(shipped).unwrap()).unwrap();
    let driver = argv(&["{brokkr}", "driver", "agy", "--"]);
    let agy = loaded_from(&[shipped]).seat(Some("agy"), &driver);
    assert!(agy.declares_its_kinds());
    assert!(!agy.rejoins_by_legacy());
    let standing = Some(Standing {
        boundary: "namespace",
        hands: "none",
    });
    assert!(!agy.rejoins("agy-conversation", standing));
    for allowed in ["agy-conversation", "none"] {
        let transcript = json!({"step": "s", "transcript": {"kind": allowed}});
        assert_eq!(agy.admit(&transcript, "work"), Ok(()), "{allowed}");
    }
    for kind in BUILT_IN {
        let transcript = json!({"step": "s", "transcript": {"kind": kind}});
        assert_eq!(
            agy.admit(&transcript, "work"),
            refused(Member::Transcript, kind),
            "{kind}"
        );
    }
    for kind in BUILT_IN.into_iter().chain(["agy-conversation"]) {
        let root = json!({"step": "s", "root_session": {"kind": kind}});
        assert_eq!(
            agy.admit(&root, "work"),
            refused(Member::RootSession, kind),
            "root {kind}"
        );
    }
}

/// #1227 U2(a) & U2(c): `agy-conversation` is admitted ONLY where the seat's
/// loaded adapter declares it in `record_kinds`. An adapterless inline
/// `{brokkr} driver agy` seat and an agy adapter without `record_kinds` both
/// refuse `agy-conversation` (and every `root_session` kind of agy's), while a
/// declaring adapter admits `agy-conversation` and refuses `claude-session` and
/// every `root_session` kind.
#[test]
fn agy_conversation_requires_declaring_adapter_and_adapterless_seat_keeps_v7_rule() {
    let driver = argv(&[
        "{brokkr}",
        "driver",
        "agy",
        "--",
        "--model",
        "gemini-4-neon",
    ]);
    let agy_transcript = json!({"step": "s", "transcript": {"kind": "agy-conversation"}});
    let none_transcript = json!({"step": "s", "transcript": {"kind": "none"}});
    let agy_root = json!({"step": "s", "root_session": {"kind": "agy-conversation"}});

    // 1. Adapterless inline `{brokkr} driver agy` seat: refuses `agy-conversation`,
    //    admits `none`, and preserves main's adapter-absent v7 rule untouched.
    let adapterless = loaded_from(&[]).seat(None, &driver);
    assert!(!adapterless.declares_its_kinds());
    assert_eq!(adapterless.declared_transcript_kind(), None);
    assert_eq!(
        adapterless.admit(&agy_transcript, "work"),
        refused(Member::Transcript, "agy-conversation")
    );
    assert_eq!(
        adapterless.admit(&agy_root, "work"),
        refused(Member::RootSession, "agy-conversation")
    );
    assert_eq!(adapterless.admit(&none_transcript, "work"), Ok(()));

    // 2. Loaded `agy` adapter WITHOUT `record_kinds`: refuses `agy-conversation`
    //    and every built-in kind; admits `none` alone.
    let bare_agy = adapter_file("agy", &["{brokkr}", "driver", "agy", "--"], json!({}));
    let undeclared = loaded_from(&[bare_agy]).seat(Some("agy"), &driver);
    assert!(!undeclared.declares_its_kinds());
    assert_eq!(undeclared.declared_transcript_kind(), None);
    assert_eq!(undeclared.admit(&none_transcript, "work"), Ok(()));
    assert_eq!(
        undeclared.admit(&agy_transcript, "work"),
        refused(Member::Transcript, "agy-conversation")
    );
    assert_eq!(
        undeclared.admit(&agy_root, "work"),
        refused(Member::RootSession, "agy-conversation")
    );
    for kind in BUILT_IN {
        let t = json!({"step": "s", "transcript": {"kind": kind}});
        assert_eq!(
            undeclared.admit(&t, "work"),
            refused(Member::Transcript, kind)
        );
        let r = json!({"step": "s", "root_session": {"kind": kind}});
        assert_eq!(
            undeclared.admit(&r, "work"),
            refused(Member::RootSession, kind)
        );
    }

    // 3. Declaring `agy` adapter carries `declared_transcript_kind == Some("agy-conversation")`.
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let shipped = workspace.join("docs/examples/adapters/agy.json");
    let shipped = serde_json::from_str(&std::fs::read_to_string(shipped).unwrap()).unwrap();
    let declared = loaded_from(&[shipped]).seat(Some("agy"), &driver);
    assert_eq!(
        declared.declared_transcript_kind(),
        Some(brokkr_protocol::DeclaredTranscriptKind::AgyConversation)
    );
}

#[cfg(unix)]
const SERVE_AGY_KINDS: &str = "BROKKR_KINDS_SERVE_AGY";

/// Not a test of its own: serves production's `agy` driver when re-entered by
/// [`agy_seat_masked_record_agrees_with_kind_admission`] so the agreement test
/// drives the real keeper and masking path without widening any crate's public
/// visibility (#1227 Q3).
#[cfg(unix)]
#[test]
fn agy_driver_child() {
    let Ok(model) = std::env::var(SERVE_AGY_KINDS) else {
        return;
    };
    let served = brokkr_protocol::adapters::serve(
        brokkr_protocol::adapters::AdapterKind::Agy,
        vec!["--model".into(), model],
        || Ok(()),
    );
    std::process::exit(if served.is_ok() { 0 } else { 70 });
}

/// Run the real `brokkr-protocol` agy driver keep path with one bound secret
/// value, returning whether the kept transcript file exists on disk and every
/// driver-masked checkpoint row emitted over the protocol (#1227 Q3).
#[cfg(unix)]
fn install_fake_agy(
    dir: &std::path::Path,
    uuid: &str,
    model: &str,
    result_path: &std::path::Path,
) -> std::path::PathBuf {
    let script_src = dir.join("fake-agy.sh");
    let script = format!(
        "#!/bin/sh\ncat >/dev/null\n\
         mkdir -p \"$HOME/.gemini/antigravity-cli/brain/{uuid}/.system_generated/logs\"\n\
         printf '{{\"step_index\":0,\"role\":\"USER\",\"text\":\"hi\",\"created_at\":\"2026-11-24T00:00:00Z\"}}\\n' \
         > \"$HOME/.gemini/antigravity-cli/brain/{uuid}/.system_generated/logs/transcript.jsonl\"\n\
         printf '{{\"result\":\"complete\"}}' > '{}'\n\
         printf '{{\"event\":\"init\",\"conversation_id\":\"{uuid}\",\"init\":{{\"model\":\"{model}\"}}}}\\n'\n\
         printf '{{\"event\":\"result\",\"result\":{{\"status\":\"SUCCESS\",\"response\":\"DONE\",\"num_turns\":1}}}}\\n'\n",
        result_path.display(),
    );
    std::fs::write(&script_src, script).unwrap();
    let fake_agy = dir.join("fake-agy");
    let copy_status = std::process::Command::new("sh")
        .args(["-c", "cp \"$1\" \"$2\" && chmod 755 \"$2\"", "sh"])
        .arg(&script_src)
        .arg(&fake_agy)
        .status()
        .unwrap();
    assert!(copy_status.success());
    fake_agy
}

#[cfg(unix)]
fn drive_agy_keep_with_secret(secret_value: &str) -> (bool, Vec<Value>) {
    use std::io::Write;
    use std::process::{Command, Stdio};

    const UUID: &str = "11111111-2222-3333-4444-555555555555";
    const MODEL: &str = "gemini-3.8-flash-low";

    let dir = tempfile::tempdir().unwrap();
    let op_home = dir.path().join("operator");
    let seat_dir = dir.path().join("seat");
    std::fs::create_dir_all(op_home.join(".gemini/antigravity-cli")).unwrap();
    std::fs::create_dir_all(&seat_dir).unwrap();
    std::fs::write(
        op_home.join(".gemini/antigravity-cli/antigravity-oauth-token"),
        b"login",
    )
    .unwrap();
    let store = dir.path().join("secrets.env");
    brokkr_protocol::secret::store_set(&store, "K", secret_value).unwrap();
    let result_path = seat_dir.join("result.json");
    let fake_agy = install_fake_agy(dir.path(), UUID, MODEL, &result_path);

    let start = json!({
        "type": "start",
        "effect_id": "e1",
        "attempt_id": "a1",
        "input": {
            "role_path": "/dev/null",
            "phase": "work",
            "feature": "f",
            "allowed_results": ["complete"],
            "result_path": result_path,
            "workdir": seat_dir,
            "secrets": ["K"],
            "secrets_file": store,
            "resume_context": {
                "assessment": Value::Null,
                "declared_transcript_kind": "agy-conversation",
            },
        },
    });
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "agents::kinds::tests::agy_driver_child",
            "--nocapture",
            "--test-threads=1",
            "-q",
        ])
        .env(SERVE_AGY_KINDS, MODEL)
        .env("HOME", &op_home)
        .env("BROKKR_AGY_BIN", &fake_agy)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        writeln!(stdin, "{{\"type\":\"hello\"}}").unwrap();
        writeln!(stdin, "{start}").unwrap();
        writeln!(stdin, "{{\"type\":\"shutdown\"}}").unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "agy driver child failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut checkpoints = Vec::new();
    let mut saw_result = false;
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Ok(msg) = serde_json::from_str::<brokkr_protocol::Message>(line) else {
            continue;
        };
        match msg.body {
            brokkr_protocol::Body::Checkpoint { data, .. } => checkpoints.push(data),
            brokkr_protocol::Body::Result {
                status: brokkr_protocol::ResultStatus::Succeeded,
                ..
            } => saw_result = true,
            _ => {}
        }
    }
    assert!(saw_result, "driver did not complete: {checkpoints:?}");

    let kept_file = op_home
        .join(".local/state/brokkr/agy-conversations")
        .join(UUID)
        .join("transcript.jsonl");
    (kept_file.exists(), checkpoints)
}

/// #1227 A2, Q3: the real `agy` driver keep path and `SeatKinds::admit` agree
/// for bound values `transcript`, `kind`, and `agy-conversation`: each refuses
/// to keep a file and emits the fallback `none` transcript row (masked as the
/// driver masks it), and passing that emitted row to `admit` yields
/// `UndeclaredKind::Masked` for `transcript`, `outside(Member::Transcript)`
/// for `kind`, and `Ok(())` for `agy-conversation`. A control binding proves
/// the same harness keeps and admits `agy-conversation`.
#[cfg(unix)]
#[test]
fn agy_seat_masked_record_agrees_with_kind_admission() {
    let agy_adapter = adapter_file(
        "agy",
        &["{brokkr}", "driver", "agy", "--"],
        json!({"record_kinds": {"transcript": "agy-conversation"}}),
    );
    let agy =
        loaded_from(&[agy_adapter]).seat(Some("agy"), &argv(&["{brokkr}", "driver", "agy", "--"]));

    // Control: unrelated secret keeps the transcript and admits `agy-conversation`.
    let (kept_control, control_rows) = drive_agy_keep_with_secret("unrelated-secret-value");
    assert!(kept_control, "control run must keep the transcript file");
    let control_row = control_rows
        .iter()
        .find(|row| row["step"] == "transcript")
        .expect("control transcript checkpoint");
    assert_eq!(control_row["transcript"]["kind"], "agy-conversation");
    assert_eq!(agy.admit(control_row, "work"), Ok(()));

    // Bound `transcript`: nothing kept, fallback `none` emitted under the
    // masked record key, refused by `admit` as `UndeclaredKind::Masked`.
    let (kept_transcript, rows_transcript) = drive_agy_keep_with_secret("transcript");
    assert!(!kept_transcript, "transcript binding must keep no file");
    let row_transcript = rows_transcript
        .iter()
        .find(|row| row["step"] == "[secret:K]")
        .expect("masked transcript checkpoint");
    assert_eq!(
        row_transcript["[secret:K]"],
        json!({"kind": "none", "locator": "", "home": ""})
    );
    assert_eq!(
        agy.admit(row_transcript, "work"),
        Err(UndeclaredKind::Masked {
            seat: "work".into(),
        })
    );

    // Bound `kind`: nothing kept, fallback `none` emitted under the masked
    // `kind` key, refused by `admit` as outside seat-record v8's grammar.
    let (kept_kind, rows_kind) = drive_agy_keep_with_secret("kind");
    assert!(!kept_kind, "kind binding must keep no file");
    let row_kind = rows_kind
        .iter()
        .find(|row| row["step"] == "transcript")
        .expect("kind-masked transcript checkpoint");
    assert_eq!(
        row_kind["transcript"],
        json!({"[secret:K]": "none", "locator": "", "home": ""})
    );
    assert_eq!(agy.admit(row_kind, "work"), outside(Member::Transcript));

    // Bound `agy-conversation`: nothing kept, fallback `none` emitted intact,
    // admitted by `admit`.
    let (kept_kind_val, rows_kind_val) = drive_agy_keep_with_secret("agy-conversation");
    assert!(!kept_kind_val, "agy-conversation binding must keep no file");
    let row_kind_val = rows_kind_val
        .iter()
        .find(|row| row["step"] == "transcript")
        .expect("agy-conversation-masked transcript checkpoint");
    assert_eq!(
        row_kind_val["transcript"],
        json!({"kind": "none", "locator": "", "home": ""})
    );
    assert_eq!(agy.admit(row_kind_val, "work"), Ok(()));
}
