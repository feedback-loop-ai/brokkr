//! Hardening tests for agy rejoin admission, first-line confirmation, empty
//! databases, launch-row masking preflight, and closed `owned_target` (#1227).

use super::*;
use crate::adapters::agy::fold::{Event, Failed, Failure, Verdict};
use crate::adapters::agy::rejoin::keep_rejoin;
use crate::adapters::agy::restore::check_owned_target;

fn assert_declined_before_lock(
    mut world: World,
    conv: &Path,
    input: Value,
    bindings: &[secret::BoundSecret],
    expected_refusal: &str,
    expected_stderr: Option<&str>,
) -> (Invocation, Vec<(Step, Value)>) {
    let pair = assert_declined_with(
        &mut world,
        &input,
        UUID_A,
        ("1.5.18", 0),
        bindings,
        expected_refusal,
        expected_stderr,
    );
    assert!(
        !conv.join(LOCK_FILE).exists(),
        "pre-lock decline must not take lock"
    );
    assert_eq!(
        std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
        b"orig-db"
    );
    pair
}

#[test]
fn r1_missing_or_unreadable_originating_version_declines_cold_before_lock() {
    let cases: [(&str, Option<Value>); 4] = [
        ("absent", None),
        ("null", Some(json!(null))),
        ("integer", Some(json!(7))),
        ("empty", Some(json!(""))),
    ];
    for (label, replacement) in cases {
        let (world, conv, mut input) = seeded_rejoin_world(b"orig-db", b"orig-brain");
        let rc = input["resume_context"].as_object_mut().unwrap();
        match replacement {
            None => {
                rc.remove("originating_harness_version");
            }
            Some(v) => {
                rc.insert("originating_harness_version".into(), v);
            }
        }
        let (_, emitted) =
            assert_declined_before_lock(world, &conv, input, &[], "unverified-harness", None);
        assert_eq!(
            started_row(&emitted).unwrap()["root_session"]["id"],
            UUID_B,
            "{label}"
        );
    }
}

#[test]
fn r2_unconfirmed_rejoin_stops_on_any_non_confirming_first_line() {
    let valid_init = format!(
        "{{\"event\":\"init\",\"conversation_id\":\"{UUID_A}\",\"init\":{{\"model\":\"{FLASH}\"}}}}\n"
    );
    let ok_res = "{\"event\":\"result\",\"result\":{\"status\":\"SUCCESS\",\"response\":\"OK\"}}\n";
    let step_line = "{\"event\":\"step_update\",\"step_update\":{\"step_type\":\"agent_response\",\"state\":\"DONE\"}}\n";
    let mut non_utf8 = b"\xff\n".to_vec();
    non_utf8.extend_from_slice(valid_init.as_bytes());
    non_utf8.extend_from_slice(ok_res.as_bytes());

    let cases: [(&str, Vec<u8>); 9] = [
        ("non-utf8-before-init", non_utf8),
        (
            "malformed-init-other-model-type",
            format!("{{\"event\":\"init\",\"conversation_id\":\"{UUID_B}\",\"init\":{{\"model\":123}}}}\n{valid_init}{ok_res}").into_bytes(),
        ),
        (
            "malformed-init-other-null",
            format!("{{\"event\":\"init\",\"conversation_id\":\"{UUID_B}\",\"init\":null}}\n{valid_init}{ok_res}").into_bytes(),
        ),
        (
            "malformed-init-offered-model-type",
            format!("{{\"event\":\"init\",\"conversation_id\":\"{UUID_A}\",\"init\":{{\"model\":123}}}}\n{valid_init}{ok_res}").into_bytes(),
        ),
        ("non-json-before-init", format!("not json\n{valid_init}{ok_res}").into_bytes()),
        ("unknown-event-before-init", format!("{{\"event\":\"heartbeat\"}}\n{valid_init}{ok_res}").into_bytes()),
        (
            "duplicate-event-key",
            format!("{{\"event\":\"init\",\"event\":\"init\",\"conversation_id\":\"{UUID_A}\",\"init\":{{\"model\":\"{FLASH}\"}}}}\n{ok_res}").into_bytes(),
        ),
        ("result-ahead-of-init", format!("{ok_res}{valid_init}{ok_res}").into_bytes()),
        ("step-ahead-of-init", format!("{step_line}{valid_init}{ok_res}").into_bytes()),
    ];

    for (label, raw) in cases {
        let (mut world, conv, input) = seeded_rejoin_world(b"orig-db", b"orig-brain");
        let mut spec = FakeAgySpec::turn(UUID_A, "", Some("mutated-db"), ("note.txt", "mutated"));
        spec.raw_stdout = Some(&raw);
        spec.sleep_after_first = true;
        let (res, emitted) =
            drive_offer_result(&mut world, &spec, &input, Some(UUID_A), &[], |_| {});
        let inv = res.unwrap_or_else(|e| panic!("{label}: unsettled must return Ok, got {e:?}"));
        assert_eq!(inv.launch, LaunchTerminal::Unconfirmed, "{label}");
        assert!(inv.answer.is_none(), "{label}");
        assert!(!world.seat().join("after_sleep.marker").exists(), "{label}");
        assert!(started_row(&emitted).is_none(), "{label}");
        assert_eq!(
            std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
            b"orig-db",
            "{label}"
        );
        assert_lock_free(&conv);
    }

    let mut cold_fold = AgyFold::new(FLASH, &[]);
    let mut cold_hold = LaunchHold::new(
        "agy",
        LaunchPlan::cold(Vec::new(), "agy-conversation", None),
    );
    let mut meta = Map::new();
    assert!(matches!(
        cold_fold.read(b"\xff", &mut meta, &mut cold_hold, &mut |_| {}),
        Line::Read
    ));
    assert_eq!(
        cold_fold.verdict(0, false),
        Verdict::Failed(Failed {
            failure: Failure::NotText,
            seen: fold::Seen::default(),
        })
    );
}

#[test]
fn r3_empty_database_declines_restore_and_refuses_keep() {
    let (dir, op, eng, conv_direct) = seeded_restore_dir(b"", b"non-empty-brain");
    assert_eq!(
        lock_and_restore(UUID_A, &eng, Some(&op), RejoinCaps::DEFAULT).err(),
        Some(RestoreNotice::Copy(RejoinNotice::EmptyDatabase))
    );
    assert!(!eng.join(".gemini/antigravity-cli/conversations").exists());
    assert!(!eng.join(".gemini/antigravity-cli/brain").exists());
    assert_lock_free(&conv_direct);

    let (mut world, conv, input) = seeded_rejoin_world(b"", b"non-empty-brain");
    assert_declined_with(
        &mut world,
        &input,
        UUID_A,
        ("1.5.18", 0),
        &[],
        "unverified-harness",
        Some("conversation database is empty"),
    );
    assert_lock_free(&conv);
    assert_eq!(
        std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
        b""
    );
    drop(world);

    let cold_stdout = recorded_box("l2c-01-cold.ndjson");
    let mut world = World::new();
    let input = rejoin_input(&world, "1.5.18", "not applicable", "none");
    let spec = FakeAgySpec::turn(UUID_A, &cold_stdout, Some(""), ("note.txt", "b1"));
    let (inv, emitted) = drive_offer(&mut world, &spec, &input, None, &[]);
    let started = started_row(&emitted).expect("harness-started emitted");
    assert_eq!(
        (inv.launch, &started["root_session"]["persistent"]),
        (LaunchTerminal::Cold, &json!(false))
    );
    assert!(
        !conv_dir(world.operator_home(), UUID_A)
            .join(REJOIN_DIR)
            .exists()
            && started.get("transcript").is_none()
            && inv.stderr.contains("conversation database is empty")
            && !inv.stderr.contains(UUID_A),
        "{}",
        inv.stderr
    );
    drop(world);

    let eng2 = dir.path().join("eng-empty-keep");
    let convs = eng2.join(".gemini/antigravity-cli/conversations");
    let brain = eng2.join(".gemini/antigravity-cli/brain").join(UUID_B);
    std::fs::create_dir_all(&convs).unwrap();
    std::fs::create_dir_all(&brain).unwrap();
    std::fs::write(convs.join(format!("{UUID_B}.db")), b"").unwrap();
    std::fs::write(brain.join("note.txt"), b"ok").unwrap();
    assert_eq!(
        keep_rejoin(UUID_B, &eng2, Some(&op), &[], false, RejoinCaps::DEFAULT),
        Err(RejoinNotice::EmptyDatabase)
    );
    assert!(!conv_dir(&op, UUID_B).exists());

    let l2c_02 = recorded_box("l2c-02-rejoin-kept.ndjson");
    let (mut world, conv, input) = seeded_rejoin_world(b"turn-1-db", b"t1");
    let spec = FakeAgySpec::turn(UUID_A, &l2c_02, Some(""), ("note.txt", "t2"));
    let (inv, emitted) = drive_offer(&mut world, &spec, &input, Some(UUID_A), &[]);
    let started = started_row(&emitted).expect("harness-started emitted");
    assert_eq!(
        (
            inv.launch,
            &started["launch"],
            &started["root_session"]["persistent"]
        ),
        (LaunchTerminal::Resumed, &json!("resumed"), &json!(false))
    );
    assert!(
        !conv.join(REJOIN_DIR).exists() && inv.stderr.contains("conversation database is empty")
    );
    assert_lock_free(&conv);
}

#[test]
fn r4_masked_rejoin_declines_cold_before_lock() {
    let cases = [
        ("version", "VER", "1.5.18"),
        ("launch-word", "LCH", "resumed"),
        ("harness-word", "HRN", "agy"),
        ("kind-word", "KND", "agy-conversation"),
        ("offered-id", "OID", UUID_A),
        ("address-home", "ADDR_HOME", "agy-conversations"),
    ];
    for (label, name, secret_val) in cases {
        let (world, conv, input) = seeded_rejoin_world(b"orig-db", b"orig-brain");
        let bindings = [bound(name, secret_val)];
        let (inv, _) = assert_declined_before_lock(
            world,
            &conv,
            input,
            &bindings,
            "unverified-harness",
            Some("rewrite the launch row"),
        );
        assert!(!inv.stderr.contains(UUID_A), "{label}: {}", inv.stderr);
    }

    let l2c_02 = recorded_box("l2c-02-rejoin-kept.ndjson");
    let (mut world, conv, input) = seeded_rejoin_world(b"orig-db", b"orig-brain");
    let spec = FakeAgySpec::turn(UUID_A, &l2c_02, Some("t2-db"), ("note.txt", "t2"));
    let bindings = [bound("TOK", "unrelated-secret-xyz-9876")];
    let (inv, emitted) = drive_offer(&mut world, &spec, &input, Some(UUID_A), &bindings);
    let started = started_row(&emitted).expect("resumed harness-started");
    assert_eq!(
        (
            inv.launch,
            &started["launch"],
            &started["root_session"]["persistent"],
            &started["transcript"]["locator"],
        ),
        (
            LaunchTerminal::Resumed,
            &json!("resumed"),
            &json!(true),
            &json!(UUID_A),
        )
    );
    assert_lock_free(&conv);
}

#[test]
fn r5_rejoin_fails_on_malformed_result_usage_or_num_turns() {
    let init =
        format!(r#"{{"event":"init","conversation_id":"{UUID_A}","init":{{"model":"{FLASH}"}}}}"#);
    let step = r#"{"event":"step_update","step_update":{"step_type":"agent_response","state":"DONE","usage":{"input_tokens":10,"output_tokens":2}}}"#;
    let bad_usage = r#"{"event":"result","result":{"status":"SUCCESS","response":"OK","usage":{"input_tokens":"bad"},"num_turns":2}}"#;
    let bad_turns = r#"{"event":"result","result":{"status":"SUCCESS","response":"OK","usage":{"input_tokens":10,"output_tokens":2},"num_turns":"two"}}"#;

    for (label, bad_result) in [("bad-usage", bad_usage), ("bad-num-turns", bad_turns)] {
        let mut fold = AgyFold::new(FLASH, &[]);
        let mut hold = rejoin_hold(UUID_A);
        let mut meta = Map::new();
        for line in [init.as_str(), step, bad_result] {
            let step_line = fold.read(line.as_bytes(), &mut meta, &mut hold, &mut |_| {});
            assert!(matches!(step_line, Line::Read), "{label}");
        }
        assert_eq!(hold.terminal(), LaunchTerminal::Resumed, "{label}");
        assert_eq!(
            fold.verdict(0, false),
            Verdict::Failed(Failed {
                failure: Failure::Unread(Event::Result),
                seen: fold::Seen::default(),
            }),
            "{label}"
        );
    }
}

#[test]
fn r7_declined_offer_masking_judges_resume_refusal_token() {
    let cold_stdout = recorded_box("l2c-01-cold.ndjson").replace(UUID_A, UUID_B);
    let (mut world, conv, mut input) = seeded_rejoin_world(b"orig-db", b"orig-brain");
    input["resume_context"]["assessment"][AGY_SHAPE]["status"] = json!("unsupported");
    let spec = FakeAgySpec::turn(
        UUID_B,
        &cold_stdout,
        Some("fresh-db"),
        ("note.txt", "fresh"),
    );
    let bindings = [bound("REF_TOK", "unsupported-resume")];
    let (inv, emitted) = drive_offer(&mut world, &spec, &input, Some(UUID_A), &bindings);
    assert_eq!(inv.launch, LaunchTerminal::Cold);
    assert!(!conv_dir(world.operator_home(), UUID_B)
        .join(REJOIN_DIR)
        .exists());
    let started = started_row(&emitted).expect("declined offer still emits harness-started");
    assert_eq!(
        (&started["launch"], &started["resume_refusal"]),
        (&json!("cold"), &json!("unsupported-resume"))
    );
    assert!(started.get("root_session").is_none() && started.get("transcript").is_none());
    assert!(
        inv.stderr.contains("rewrite the root session row"),
        "{}",
        inv.stderr
    );
    assert!(!conv.join(LOCK_FILE).exists());
}

#[test]
fn r8_owned_target_rejects_unknown_member() {
    let (world, conv, mut input) = seeded_rejoin_world(b"orig-db", b"orig-brain");
    input["resume_context"]["owned_target"]
        .as_object_mut()
        .unwrap()
        .insert("extra".into(), json!("unexpected"));
    assert_eq!(
        check_owned_target(&input, UUID_A, Some(&world.operator_home())),
        Err("unverified-harness")
    );
    assert_declined_before_lock(world, &conv, input, &[], "unverified-harness", None);
}

fn assert_fold_unread(hold_init: fn() -> LaunchHold, lines: &[&str], expected: Event, label: &str) {
    let mut fold = AgyFold::new(FLASH, &[]);
    let mut hold = hold_init();
    let mut meta = Map::new();
    for line in lines {
        assert!(
            matches!(
                fold.read(line.as_bytes(), &mut meta, &mut hold, &mut |_| {}),
                Line::Read
            ),
            "{label}"
        );
    }
    assert_eq!(
        fold.verdict(0, false),
        Verdict::Failed(Failed {
            failure: Failure::Unread(expected),
            seen: fold::Seen::default(),
        }),
        "{label}"
    );
}

#[test]
fn t1_array_owned_target_and_array_init_first_line_are_rejected() {
    let (world, conv, mut input) = seeded_rejoin_world(b"orig-db", b"orig-brain");
    let home = conv.parent().unwrap().to_path_buf();
    input["resume_context"]["owned_target"] = json!([UUID_A, UUID_A, home]);
    assert_eq!(
        check_owned_target(&input, UUID_A, Some(&world.operator_home())),
        Err("unverified-harness")
    );
    assert_declined_before_lock(world, &conv, input, &[], "unverified-harness", None);

    let valid_init = format!(
        "{{\"event\":\"init\",\"conversation_id\":\"{UUID_A}\",\"init\":{{\"model\":\"{FLASH}\"}}}}\n"
    );
    let ok_res = "{\"event\":\"result\",\"result\":{\"status\":\"SUCCESS\",\"response\":\"OK\"}}\n";
    let array_init_stream = format!(
        "{{\"event\":\"init\",\"conversation_id\":\"{UUID_A}\",\"init\":[\"{FLASH}\"]}}\n{valid_init}{ok_res}"
    )
    .into_bytes();
    let (mut world, conv, input) = seeded_rejoin_world(b"orig-db", b"orig-brain");
    let mut spec = FakeAgySpec::turn(UUID_A, "", Some("mutated-db"), ("note.txt", "mutated"));
    spec.raw_stdout = Some(&array_init_stream);
    spec.sleep_after_first = true;
    let (res, emitted) = drive_offer_result(&mut world, &spec, &input, Some(UUID_A), &[], |_| {});
    let inv = res.expect("array init on rejoin must end unsettled");
    assert_eq!(inv.launch, LaunchTerminal::Unconfirmed);
    assert!(inv.answer.is_none());
    assert!(!world.seat().join("after_sleep.marker").exists());
    assert!(started_row(&emitted).is_none());
    assert_eq!(
        std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
        b"orig-db"
    );
    assert_lock_free(&conv);
}

#[test]
fn t1_array_in_place_of_object_fails_fold_rows_cold_and_rejoin() {
    let init_ok =
        format!(r#"{{"event":"init","conversation_id":"{UUID_A}","init":{{"model":"{FLASH}"}}}}"#);
    let init_arr = format!(r#"{{"event":"init","conversation_id":"{UUID_A}","init":["{FLASH}"]}}"#);
    let cold_hold = || {
        LaunchHold::new(
            "agy",
            LaunchPlan::cold(Vec::new(), "agy-conversation", None),
        )
    };
    let rejoin = || rejoin_hold(UUID_A);
    let result_arr = r#"{"event":"result","result":["SUCCESS","OK",[1,2,3,4],1,[],null]}"#;
    let result_usage_arr =
        r#"{"event":"result","result":{"status":"SUCCESS","response":"OK","usage":[1,2,3,4]}}"#;
    let result_denied_arr = r#"{"event":"result","result":{"status":"SUCCESS","response":"OK","denied_actions":[["run_command"]]}}"#;

    assert_fold_unread(
        cold_hold,
        &[init_arr.as_str()],
        Event::Init,
        "cold-init-arr",
    );
    for (label, bad_line, expected) in [
        (
            "cold-step-arr",
            r#"{"event":"step_update","step_update":["agent_response","DONE"]}"#,
            Event::StepUpdate,
        ),
        ("cold-result-arr", result_arr, Event::Result),
        ("cold-result-usage-arr", result_usage_arr, Event::Result),
        ("cold-result-denied-arr", result_denied_arr, Event::Result),
    ] {
        assert_fold_unread(cold_hold, &[init_ok.as_str(), bad_line], expected, label);
    }

    for (label, bad_line, expected) in [
        (
            "rejoin-step-arr",
            r#"{"event":"step_update","step_update":["agent_response","DONE",null]}"#,
            Event::StepUpdate,
        ),
        (
            "rejoin-step-usage-arr",
            r#"{"event":"step_update","step_update":{"step_type":"agent_response","state":"DONE","usage":[1,2,3,4]}}"#,
            Event::StepUpdate,
        ),
        ("rejoin-result-arr", result_arr, Event::Result),
        ("rejoin-result-usage-arr", result_usage_arr, Event::Result),
        ("rejoin-result-denied-arr", result_denied_arr, Event::Result),
    ] {
        assert_fold_unread(rejoin, &[init_ok.as_str(), bad_line], expected, label);
    }
}

#[test]
fn t2_sealed_standing_fence_unbuilt_intent_declines_cold_before_lock() {
    use crate::native_controls::{SealedBoundary::Open, SERVING_INPUTS};

    for (label, resume_map) in [
        ("no-resume-assessment", json!({})),
        (
            "measured-resume-unbuilt-replacement",
            json!({ AGY_SHAPE: "measured" }),
        ),
    ] {
        let (world, conv, mut input) = seeded_rejoin_world(b"orig-db", b"orig-brain");
        let mut serving = sealed(Open, "empty", json!("measured"), "standing");
        serving["isolation"]["replacement"] = json!("unmeasured");
        serving["isolation"]["resume"] = resume_map;
        input[SERVING_INPUTS] = serving;
        let (_, emitted) =
            assert_declined_before_lock(world, &conv, input, &[], "restrictions-unavailable", None);
        assert_eq!(
            started_row(&emitted).unwrap()["resume_refusal"],
            json!("restrictions-unavailable"),
            "{label}"
        );
    }
}

#[test]
fn t4_lease_drop_unlocks_before_close_even_when_dup_fd_outlives_it() {
    let (_dir, op, eng, conv) = seeded_restore_dir(b"turn-1-db", b"turn-1");

    let lease = lock_and_restore(UUID_A, &eng, Some(&op), RejoinCaps::DEFAULT).unwrap();
    let dup_fd = rustix::io::dup(lease.lock_fd()).unwrap();
    assert!(is_lock_held(&conv), "lock must be held while lease lives");
    drop(lease);
    assert_lock_free(&conv);
    drop(dup_fd);
}
