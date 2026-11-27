//! Publication tests for agy rejoin state: exact driver refusal stderr and
//! isolated publication probe guard (#1227, decision 0075 level 2 slice 1).

use super::*;
use crate::adapters::agy::rejoin::{publish_turn, DeclaredKinds, TurnKeep};
use crate::adapters::{Invocation, LaunchTerminal, Step};

fn drive_declared_work(
    version: (&str, i32),
    db: Option<&[u8]>,
    bindings: &[secret::BoundSecret],
    stdout: &str,
) -> (World, Invocation, Vec<(Step, Value)>) {
    let mut world = World::new();
    let seat_dir = world.seat();
    let bin = write_rejoin_fake_agy(&seat_dir, version, UUID, db, Some(b"step-text"), stdout);
    world.set(&bin);
    let input = declared_work_input(&world);
    let mut emitted = Vec::new();
    let inv =
        super::super::drive_flash_ended(&bin, StartMode::default(), &input, bindings, &mut |e| {
            emitted.push((e.step, e.row.clone()))
        })
        .expect("declared work turn succeeds");
    (world, inv, emitted)
}

#[test]
fn work_turn_records_root_session_persistent_iff_kept_whole() {
    let stdout = format!("{}\n{STEP_DONE}\n{OK_RES}\n", init_line(UUID, FLASH));

    let (world, seat_dir, messages) =
        run_declared_work(("1.5.14\n", 0), Some(b"sqlite-db"), None, &stdout);
    assert!(world.homes_gone(), "engine HOME must be removed");
    let env_text = std::fs::read_to_string(seat_dir.join("version.env")).unwrap();
    let env_lines: Vec<&str> = env_text.lines().collect();
    assert_eq!(
        env_lines[0], "true",
        "AGY_CLI_DISABLE_AUTO_UPDATE=true on --version"
    );
    assert_ne!(Path::new(env_lines[1]), world.operator_home());

    let conv = conv_dir(world.operator_home(), UUID);
    assert!(conv.join("transcript.jsonl").exists());
    assert!(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE).exists());
    assert!(conv
        .join(REJOIN_DIR)
        .join(REJOIN_BRAIN_DIR)
        .join(".system_generated/steps/1/output.txt")
        .exists());
    let started = harness_started(&messages).expect("harness-started checkpoint");
    assert_eq!(started["harness"], "agy");
    assert_eq!(started["launch"], "cold");
    assert_eq!(
        started["root_session"],
        json!({"kind": "agy-conversation", "id": UUID, "harness_version": "1.5.14", "persistent": true})
    );
    assert_eq!(started["transcript"]["kind"], "agy-conversation");
    assert_eq!(started["transcript"]["locator"], UUID);
    drop(world);

    // Missing DB -> persistent: false, rejoin absent, transcript STILL kept, exact refusal stderr.
    let (world2, inv2, emitted2) = drive_declared_work(("1.5.14\n", 0), None, &[], &stdout);
    let conv2 = conv_dir(world2.operator_home(), UUID);
    assert!(conv2.join("transcript.jsonl").exists());
    assert!(!conv2.join(REJOIN_DIR).exists());
    let started2 = emitted2
        .iter()
        .find_map(|(step, row)| (*step == Step::HarnessStarted).then_some(row))
        .expect("harness-started on missing db");
    assert_eq!(started2["root_session"]["persistent"], json!(false));
    assert!(started2.get("transcript").is_none());
    assert_eq!(
        inv2.stderr,
        "agy rejoin state not kept: a required conversation file or directory is absent\n"
    );
}

#[test]
fn work_turn_handles_bound_secret_version_failure_and_masked_root_session() {
    let stdout = format!("{}\n{STEP_DONE}\n{OK_RES}\n", init_line(UUID, FLASH));

    // Bound value in conversation -> persistent: false, rejoin absent, transcript kept.
    let (w1, _, m1) = run_declared_work(
        ("1.5.14\n", 0),
        Some(b"db with secret-printed-value-42"),
        Some(("MY_KEY", "secret-printed-value-42")),
        &stdout,
    );
    assert!(conv_dir(w1.operator_home(), UUID)
        .join("transcript.jsonl")
        .exists());
    assert!(!conv_dir(w1.operator_home(), UUID).join(REJOIN_DIR).exists());
    assert_eq!(
        harness_started(&m1).unwrap()["root_session"]["persistent"],
        json!(false)
    );
    drop(w1);

    // agy --version fails even when printing a valid version -> no root_session, no rejoin, transcript kept.
    let (w2, _, m2) = run_declared_work(("1.5.14\n", 1), Some(b"db"), None, &stdout);
    assert!(conv_dir(w2.operator_home(), UUID)
        .join("transcript.jsonl")
        .exists());
    assert!(!conv_dir(w2.operator_home(), UUID).join(REJOIN_DIR).exists());
    assert_no_root_session(&m2);
    drop(w2);

    // Bound value masks root_session row ("1.5.14") -> no root_session, no rejoin, exact refusal stderr.
    let (w3, inv3, emitted3) = drive_declared_work(
        ("1.5.14\n", 0),
        Some(b"db"),
        &[bound("VER_S", "1.5.14")],
        &stdout,
    );
    assert!(conv_dir(w3.operator_home(), UUID)
        .join("transcript.jsonl")
        .exists());
    assert!(!conv_dir(w3.operator_home(), UUID).join(REJOIN_DIR).exists());
    assert!(emitted3
        .iter()
        .all(|(_, row)| row.get("root_session").is_none()));
    assert_eq!(
        inv3.stderr,
        "agy rejoin state not kept: masking the turn's secret bindings would rewrite the root session row\n"
    );
}

#[test]
fn direct_probe_with_eligible_version_is_stopped_by_publication_probe_guard() {
    let (_dir, engine, op) = setup_planted_homes(
        b"sqlite-db",
        &[
            (".system_generated/logs/transcript.jsonl", b"{}\n"),
            (".system_generated/steps/1/output.txt", b"step-text"),
        ],
    );
    let mut outcome = Ok(Invocation {
        exit_code: 0,
        session_meta: Map::new(),
        notes: String::new(),
        stderr: String::new(),
        state: None,
        refusal: None,
        answer: None,
        launch: LaunchTerminal::Cold,
        result_exists: Some(true),
    });
    let mut emitted = Vec::new();
    publish_turn(
        TurnKeep {
            declared: DeclaredKinds {
                transcript: true,
                root: true,
            },
            probe: true,
            refused_to_start: false,
            pin_confirmed: true,
            conversation: &fold::Conversation::Named(UUID.to_string()),
            harness_version: Some("1.5.14"),
            engine_home: &engine,
            operator_home: Some(&op),
            bindings: &[],
        },
        &mut outcome,
        &mut |e| emitted.push((e.step, e.row.clone())),
    );
    let inv = outcome.expect("publish_turn keeps Ok outcome");
    assert_eq!(inv.stderr, "");
    assert_eq!(
        inv.session_meta["transcript"],
        json!({"kind": "none", "locator": "", "home": ""})
    );
    assert!(
        emitted
            .iter()
            .all(|(step, row)| *step != Step::HarnessStarted && row.get("root_session").is_none()),
        "probe publication must emit no root_session: {emitted:?}"
    );
    assert!(
        !conv_dir(&op, UUID).exists(),
        "probe publication must keep nothing"
    );
}
