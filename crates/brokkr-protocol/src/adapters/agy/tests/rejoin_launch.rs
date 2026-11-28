//! Level 2 slice 2 tests (#1227, decision 0075): restoring kept rejoin state,
//! `--conversation` launch, confirmation and stray-conversation termination,
//! exclusive per-conversation locking, post-rejoin replace/remove/untouched
//! keeping, current-turn-only accounting from the `l2c-*` recordings, and
//! fresh restriction composition.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use rustix::fs::{flock, openat, FlockOperation, Mode, OFlags, CWD};
use serde_json::{json, Map, Value};

use super::super::keep::{ParentError, SourceErrno};
use super::super::rejoin::CopyPoint;
use super::super::rejoin::{
    RejoinCaps, RejoinNotice, REJOIN_BRAIN_DIR, REJOIN_DB_FILE, REJOIN_DIR,
};
use super::super::restore::{
    check_owned_target, lock_and_restore, open_existing_owned_parent, remove_entry_tree,
    replace_rejoin, replace_rejoin_with, stage_replace_transcript, RestoreNotice, LOCK_FILE,
};
use super::boxed::{boxed_input, recorded_box};
use super::judged::bound;
use super::*;
use crate::adapters::stream_turn::Line;
use crate::adapters::{LaunchHold, LaunchPlan, LaunchTerminal, Step, AGY_SHAPE};

const UUID_A: &str = "764607c1-0f06-4973-9fa7-1a1054494a3b";
const UUID_B: &str = "e7836a32-679d-4c0a-8a59-5f30096b1719";
const UUID_OTHER: &str = "00000000-0000-4000-8000-000000000000";

fn conv_dir(operator_home: impl AsRef<Path>, uuid: &str) -> PathBuf {
    operator_home
        .as_ref()
        .join(".local/state/brokkr/agy-conversations")
        .join(uuid)
}

fn enabled_assessment(version: &str, boundary: &str, hands: &str) -> Value {
    json!({
        AGY_SHAPE: {
            "status": "supported",
            "identity": { "version": version, "applies_to": version },
            "classes": ["work"],
            "boundaries": [boundary],
            "hands": hands,
            "evidence": {
                "interface": "l2c-01..03",
                "restrictions": "l2c-19..22",
                "root": "l2c-02",
                "accounting": "l2c-01..13"
            }
        }
    })
}

fn owned_target_for(operator_home: impl AsRef<Path>, uuid: &str) -> Value {
    let home = operator_home
        .as_ref()
        .join(".local/state/brokkr/agy-conversations");
    json!({
        "provider_id": uuid,
        "persistence_locator": uuid,
        "persistence_home": home,
    })
}

fn rejoin_input(world: &World, version: &str, boundary: &str, hands: &str) -> Value {
    rejoin_input_for(world, UUID_A, version, boundary, hands)
}

fn rejoin_input_for(
    world: &World,
    offered: &str,
    version: &str,
    boundary: &str,
    hands: &str,
) -> Value {
    let mut input = world.input(None);
    input["boundary"] = json!(boundary);
    input["hands"] = json!(hands);
    input["resume_context"] = json!({
        "assessment": enabled_assessment(version, boundary, hands),
        "declared_transcript_kind": "agy-conversation",
        "declared_root_session_kind": "agy-conversation",
        "originating_harness_version": version,
        "owned_target": owned_target_for(world.operator_home(), offered),
    });
    input
}

fn boxed_rejoin_input(world: &World, version: &str, resume_measured: bool) -> Value {
    let mut input = boxed_input(world);
    input["boundary"] = json!("namespace");
    input["hands"] = json!("boxed");
    input["resume_context"] = json!({
        "assessment": enabled_assessment(version, "namespace", "boxed"),
        "declared_transcript_kind": "agy-conversation",
        "declared_root_session_kind": "agy-conversation",
        "originating_harness_version": version,
        "owned_target": owned_target_for(world.operator_home(), UUID_A),
    });
    if resume_measured {
        input[crate::native_controls::SERVING_INPUTS]["isolation"]["resume"] =
            json!({ AGY_SHAPE: "measured" });
    }
    input
}

struct FakeAgySpec<'a> {
    version: (&'a str, i32),
    id: &'a str,
    next_db: Option<&'a str>,
    next_brain: Option<(&'a str, &'a str)>,
    next_transcript: Option<&'a str>,
    stdout: &'a str,
    raw_stdout: Option<&'a [u8]>,
    exit_code: i32,
    sleep_after_first: bool,
    remove_old_note: bool,
}

impl<'a> FakeAgySpec<'a> {
    fn turn(id: &'a str, stdout: &'a str, db: Option<&'a str>, brain: (&'a str, &'a str)) -> Self {
        Self {
            version: ("1.5.18", 0),
            id,
            next_db: db,
            next_brain: Some(brain),
            next_transcript: Some("{\"turn\":2}"),
            stdout,
            raw_stdout: None,
            exit_code: 0,
            sleep_after_first: false,
            remove_old_note: false,
        }
    }
}

fn fake_agy_actions(seat_dir: &Path, spec: &FakeAgySpec<'_>, stdout_file: &Path) -> String {
    let mut out = String::new();
    if spec.remove_old_note {
        out.push_str(&format!(
            "rm -f \"$HOME/.gemini/antigravity-cli/brain/{}/note.txt\"\n",
            spec.id
        ));
    }
    match spec.next_db {
        Some(data) => out.push_str(&format!(
            "mkdir -p \"$HOME/.gemini/antigravity-cli/conversations\"\nprintf '%s' '{data}' > \"$HOME/.gemini/antigravity-cli/conversations/{}.db\"\n",
            spec.id
        )),
        None => out.push_str(&format!(
            "rm -f \"$HOME/.gemini/antigravity-cli/conversations/{}.db\"\n",
            spec.id
        )),
    }
    if let Some((rel, data)) = spec.next_brain {
        out.push_str(&format!(
            "mkdir -p \"$HOME/.gemini/antigravity-cli/brain/{}/$(dirname '{rel}')\"\nprintf '%s' '{data}' > \"$HOME/.gemini/antigravity-cli/brain/{}/{rel}\"\n",
            spec.id, spec.id
        ));
    }
    if let Some(data) = spec.next_transcript {
        out.push_str(&format!(
            "mkdir -p \"$HOME/.gemini/antigravity-cli/brain/{}/.system_generated/logs\"\nprintf '%s\\n' '{data}' > \"$HOME/.gemini/antigravity-cli/brain/{}/.system_generated/logs/transcript.jsonl\"\n",
            spec.id, spec.id
        ));
    }
    if spec.sleep_after_first {
        out.push_str(&format!(
            "cat >/dev/null\nhead -n 1 '{}'\nsleep 5\n: > '{}/after_sleep.marker'\ntail -n +2 '{}'\n",
            stdout_file.display(), seat_dir.display(), stdout_file.display()
        ));
    } else {
        out.push_str(&format!(
            "cat >/dev/null\ncat '{}'\n",
            stdout_file.display()
        ));
    }
    out
}

fn write_fake_rejoin_agy(seat_dir: &Path, spec: &FakeAgySpec<'_>) -> PathBuf {
    let stdout_file = seat_dir.join("turn.stdout");
    std::fs::write(
        &stdout_file,
        spec.raw_stdout.unwrap_or(spec.stdout.as_bytes()),
    )
    .unwrap();
    let actions = fake_agy_actions(seat_dir, spec, &stdout_file);
    let (d, id) = (seat_dir.display(), spec.id);
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = \"--version\" ]; then printf '%s\\n' '{}'; exit {}; fi\n\
         printf '%s\\n' \"$@\" > '{d}/last.argv'\n\
         cp \"$HOME/.gemini/config/mcp_config.json\" '{d}/snap.mcp' 2>/dev/null || true\n\
         cp \"$HOME/.gemini/config/hooks.json\" '{d}/snap.hooks' 2>/dev/null || true\n\
         if [ -e \"$HOME/.gemini/antigravity-cli/conversations\" ] || [ -e \"$HOME/.gemini/antigravity-cli/brain\" ]; then : > '{d}/had_restored_dirs.marker'; fi\n\
         for p in \"conversations/{id}.db:restored.db\" \"brain/{id}/note.txt:restored.note\"; do\n\
           s=\"$HOME/.gemini/antigravity-cli/${{p%%:*}}\"; t=\"{d}/${{p##*:}}\"\n\
           if [ -f \"$s\" ]; then cp \"$s\" \"$t\"; else rm -f \"$t\"; fi\n\
         done\n\
         {actions}exit {}\n",
        spec.version.0, spec.version.1, spec.exit_code
    );
    super::super::super::tests::executable(seat_dir, "agy", &script)
}

fn read_argv(seat_dir: &Path) -> Vec<String> {
    std::fs::read_to_string(seat_dir.join("last.argv"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

fn is_lock_held(conv: &Path) -> bool {
    let fd = openat(
        CWD,
        conv.join(LOCK_FILE),
        OFlags::RDWR | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .expect("lock file must exist");
    match flock(&fd, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => {
            let _ = flock(&fd, FlockOperation::Unlock);
            false
        }
        Err(rustix::io::Errno::WOULDBLOCK) => true,
        Err(err) => panic!("unexpected flock errno: {err:?}"),
    }
}

fn assert_lock_free(conv: &Path) {
    assert!(!is_lock_held(conv), "conversation lock must be free");
}

fn seed_kept(
    operator_home: impl AsRef<Path>,
    uuid: &str,
    db: &[u8],
    files: &[(&str, &[u8])],
) -> PathBuf {
    let conv = conv_dir(operator_home.as_ref(), uuid);
    let brain = conv.join(REJOIN_DIR).join(REJOIN_BRAIN_DIR);
    std::fs::create_dir_all(&brain).unwrap();
    for dir in [
        conv.parent().unwrap(),
        &conv,
        &conv.join(REJOIN_DIR),
        &brain,
    ] {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let db_path = conv.join(REJOIN_DIR).join(REJOIN_DB_FILE);
    std::fs::write(&db_path, db).unwrap();
    std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(conv.join("transcript.jsonl"), b"{\"turn\":1}\n").unwrap();
    for (rel, bytes) in files {
        let path = brain.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
    }
    conv
}

fn drive_offer_result(
    world: &mut World,
    spec: &FakeAgySpec<'_>,
    input: &Value,
    session: Option<&str>,
    bindings: &[secret::BoundSecret],
    mut on_emit: impl FnMut(&crate::adapters::Emitted),
) -> (Result<Invocation, Ended>, Vec<(Step, Value)>) {
    let bin = write_fake_rejoin_agy(&world.seat(), spec);
    world.set(&bin);
    let mut emitted = Vec::new();
    let mode = StartMode {
        session,
        probe: false,
    };
    let res = drive_flash_ended(&bin, mode, input, bindings, &mut |e| {
        on_emit(e);
        emitted.push((e.step, e.row.clone()));
    });
    (res, emitted)
}

fn drive_offer(
    world: &mut World,
    spec: &FakeAgySpec<'_>,
    input: &Value,
    session: Option<&str>,
    bindings: &[secret::BoundSecret],
) -> (Invocation, Vec<(Step, Value)>) {
    let (res, emitted) = drive_offer_result(world, spec, input, session, bindings, |_| {});
    (res.expect("drive_offer succeeds"), emitted)
}

fn seeded_restore_dir(db: &[u8], note: &[u8]) -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let (op, eng) = (dir.path().join("op"), dir.path().join("eng"));
    let conv = seed_kept(&op, UUID_A, db, &[("note.txt", note)]);
    std::fs::create_dir_all(eng.join(".gemini/antigravity-cli")).unwrap();
    (dir, op, eng, conv)
}

fn seeded_rejoin_world(db: &[u8], note: &[u8]) -> (World, PathBuf, Value) {
    let world = World::new();
    let conv = seed_kept(world.operator_home(), UUID_A, db, &[("note.txt", note)]);
    let input = rejoin_input(&world, "1.5.18", "not applicable", "none");
    (world, conv, input)
}

fn rejoin_hold(offered: &str) -> LaunchHold {
    let mut plan = LaunchPlan::cold(Vec::new(), "agy-conversation", None);
    plan.rejoining = Some(offered.to_string());
    plan.harness_version = Some("1.5.18".to_string());
    LaunchHold::new("agy", plan)
}

fn started_row(emitted: &[(Step, Value)]) -> Option<&Value> {
    emitted
        .iter()
        .find_map(|(s, r)| (*s == Step::HarnessStarted).then_some(r))
}

fn fold_recording(name: &str, offered: Option<&str>) -> (Map<String, Value>, LaunchTerminal) {
    let text = recorded_box(name);
    let mut fold = AgyFold::new(FLASH, &[]);
    let mut plan = LaunchPlan::cold(Vec::new(), "agy-conversation", None);
    if let Some(id) = offered {
        plan.rejoining = Some(id.to_string());
        plan.harness_version = Some("1.5.18".to_string());
    }
    let mut hold = LaunchHold::new("agy", plan);
    let mut record = Map::new();
    for line in text.lines() {
        let action = fold.read(line.as_bytes(), &mut record, &mut hold, &mut |_| {});
        if !matches!(action, Line::Read) {
            break;
        }
    }
    (record, hold.terminal())
}

#[test]
fn l6_rejoin_accounting_sums_step_rows_only_and_cold_matches_result() {
    let cases = [
        (
            "l2c-02-rejoin-kept.ndjson",
            Some(UUID_A),
            LaunchTerminal::Resumed,
            9_801,
            31,
            None,
            None,
        ),
        (
            "l2c-03-rejoin-chain.ndjson",
            Some(UUID_A),
            LaunchTerminal::Resumed,
            10_049,
            13,
            None,
            None,
        ),
        (
            "l2c-11-iso-rejoin.ndjson",
            Some("faa7e769-0ab7-4a47-b134-1282c2144460"),
            LaunchTerminal::Resumed,
            12_870 + 8_724,
            49,
            Some(8_724),
            None,
        ),
        (
            "l2c-13-box-rejoin.ndjson",
            Some("eb33b726-8955-4cfd-bd92-80d433deb774"),
            LaunchTerminal::Resumed,
            12_722 + 8_530,
            55,
            Some(8_530),
            None,
        ),
        (
            "l2c-01-cold.ndjson",
            None,
            LaunchTerminal::Cold,
            9_575,
            2,
            None,
            Some(1),
        ),
        (
            "l2c-10-iso-cold.ndjson",
            None,
            LaunchTerminal::Cold,
            16_543 + 24_280,
            236,
            Some(24_280),
            Some(1),
        ),
    ];
    for (file, offered, expected_term, input, output, cache, turns) in cases {
        let (rec, term) = fold_recording(file, offered);
        assert_eq!(term, expected_term, "{file}");
        assert_eq!(rec["input_tokens"], json!(input), "{file}");
        assert_eq!(rec["output_tokens"], json!(output), "{file}");
        assert_eq!(
            rec.get("cache_read_tokens"),
            cache.map(|v| json!(v)).as_ref(),
            "{file}"
        );
        assert_eq!(
            rec.get("num_turns"),
            turns.map(|v| json!(v)).as_ref(),
            "{file}"
        );
    }
}

fn assert_turn2_kept(conv: &Path, emitted: &[(Step, Value)]) {
    let started = started_row(emitted).expect("harness-started emitted");
    assert_eq!(started["launch"], "resumed");
    assert!(started.get("resume_refusal").is_none());
    assert_eq!(
        started["root_session"],
        json!({"kind": "agy-conversation", "id": UUID_A, "harness_version": "1.5.18", "persistent": true})
    );
    assert_eq!(started["transcript"]["locator"], UUID_A);
    assert_eq!(
        std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
        b"db-turn-2"
    );
    assert!(!conv
        .join(REJOIN_DIR)
        .join(REJOIN_BRAIN_DIR)
        .join("note.txt")
        .exists());
    assert_eq!(
        std::fs::read_to_string(conv.join("transcript.jsonl")).unwrap(),
        "{\"turn\":2}\n"
    );
}

#[test]
fn l1_l4_l5_l7_unboxed_rejoin_chains_locks_replaces_and_composes_fresh() {
    let l2c_02 = recorded_box("l2c-02-rejoin-kept.ndjson");
    let l2c_03 = recorded_box("l2c-03-rejoin-chain.ndjson");
    let mut world = World::new();
    let seat_dir = world.seat();
    let conv = seed_kept(
        world.operator_home(),
        UUID_A,
        b"db-turn-1",
        &[
            ("note.txt", b"brain-turn-1"),
            ("mcp_config.json", b"{\"mcpServers\":{\"poison\":{}}}"),
            ("hooks.json", b"{\"poison\":true}"),
        ],
    );
    let input = rejoin_input(&world, "1.5.18", "not applicable", "none");
    let cold_stdout = recorded_box("l2c-01-cold.ndjson").replace(UUID_A, UUID_B);
    let cold_spec = FakeAgySpec::turn(UUID_B, &cold_stdout, Some("db-cold"), ("note.txt", "cold"));
    drive_offer(&mut world, &cold_spec, &input, None, &[]);
    let cold_argv = read_argv(&seat_dir);
    let cold_mcp = std::fs::read_to_string(seat_dir.join("snap.mcp")).unwrap();
    let cold_hooks = std::fs::read_to_string(seat_dir.join("snap.hooks")).unwrap();

    let mut t2 = FakeAgySpec::turn(
        UUID_A,
        &l2c_02,
        Some("db-turn-2"),
        ("step2.txt", "brain-turn-2"),
    );
    t2.remove_old_note = true;
    let (mut lock_at, mut live_during_turn) = (Vec::new(), None);
    let (res2, emitted) = drive_offer_result(&mut world, &t2, &input, Some(UUID_A), &[], |e| {
        if matches!(
            e.step,
            Step::SeatTurn | Step::Transcript | Step::HarnessStarted
        ) {
            lock_at.push((e.step, is_lock_held(&conv)));
        }
        if e.step == Step::SeatTurn {
            live_during_turn = Some(conv.join(REJOIN_DIR).exists());
        }
    });
    let inv2 = res2.expect("rejoin turn 2 succeeds");
    assert_eq!(inv2.launch, LaunchTerminal::Resumed);
    assert_eq!(inv2.session_meta["input_tokens"], json!(9_801));
    assert!(world.homes_gone());

    let mut expected_argv = cold_argv;
    expected_argv.extend(["--conversation".to_string(), UUID_A.to_string()]);
    assert_eq!(read_argv(&seat_dir), expected_argv);
    let text = |name| std::fs::read_to_string(seat_dir.join(name)).unwrap();
    assert_eq!(text("snap.mcp"), cold_mcp);
    assert_eq!(text("snap.hooks"), cold_hooks);
    assert_eq!(text("restored.db"), "db-turn-1");
    assert_eq!(text("restored.note"), "brain-turn-1");
    assert_eq!(
        lock_at,
        vec![
            (Step::SeatTurn, true),
            (Step::Transcript, true),
            (Step::HarnessStarted, true),
        ],
        "lock must stay held across turn, transcript keep, and rejoin keep"
    );
    assert_eq!(
        live_during_turn,
        Some(false),
        "live rejoin/ must be vacated before agy runs"
    );
    assert_lock_free(&conv);

    assert_turn2_kept(&conv, &emitted);

    let t3 = FakeAgySpec::turn(
        UUID_A,
        &l2c_03,
        Some("db-turn-3"),
        ("step3.txt", "brain-turn-3"),
    );
    let (inv3, _) = drive_offer(&mut world, &t3, &input, Some(UUID_A), &[]);
    assert_eq!(
        (inv3.launch, &inv3.session_meta["input_tokens"]),
        (LaunchTerminal::Resumed, &json!(10_049))
    );
    assert_eq!(text("restored.db"), "db-turn-2");
    assert_eq!(
        std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
        b"db-turn-3"
    );
}

#[test]
fn l1_l7_boxed_rejoin_restores_confirms_and_carries_no_mode() {
    let l2c_02 = recorded_box("l2c-02-rejoin-kept.ndjson");
    let mut world = World::new();
    seed_kept(
        world.operator_home(),
        UUID_A,
        b"boxed-db-1",
        &[("note.txt", b"boxed-brain-1")],
    );
    let input = boxed_rejoin_input(&world, "1.5.18", true);
    let spec = FakeAgySpec::turn(
        UUID_A,
        &l2c_02,
        Some("boxed-db-2"),
        ("note.txt", "boxed-brain-2"),
    );
    let (inv, emitted) = drive_offer(&mut world, &spec, &input, Some(UUID_A), &[]);
    assert_eq!(inv.launch, LaunchTerminal::Resumed);
    assert!(world.homes_gone());
    let argv = read_argv(&world.seat());
    assert!(
        !argv.iter().any(|a| a == "--mode"),
        "boxed carries no --mode"
    );
    assert_eq!(&argv[argv.len() - 2..], ["--conversation", UUID_A]);
    assert_eq!(
        std::fs::read_to_string(world.seat().join("restored.db")).unwrap(),
        "boxed-db-1"
    );
    let started = started_row(&emitted).expect("boxed harness-started");
    assert_eq!(started["launch"], "resumed");
    assert_eq!(started["root_session"]["persistent"], json!(true));
}

#[test]
fn l3_l5_stray_conversation_stops_child_leaves_no_root_and_spares_kept_state() {
    for (file, offered, stray) in [
        ("l2c-04-unknown-id.ndjson", UUID_OTHER, UUID_B),
        (
            "l2c-05-malformed-id.ndjson",
            UUID_A,
            "997e0ae7-5fc7-42a8-8d01-c4010d91854c",
        ),
        (
            "l2c-07-brain-only.ndjson",
            UUID_A,
            "04081128-ed2e-45a6-bc45-c01177066607",
        ),
    ] {
        let mut world = World::new();
        let conv = seed_kept(
            world.operator_home(),
            offered,
            b"original-db",
            &[("note.txt", b"original-brain")],
        );
        let stdout = recorded_box(file);
        let mut spec = FakeAgySpec::turn(stray, &stdout, Some("stray-db"), ("stray.txt", "stray"));
        spec.sleep_after_first = file == "l2c-04-unknown-id.ndjson";
        let input = rejoin_input_for(&world, offered, "1.5.18", "not applicable", "none");
        let (inv, emitted) = drive_offer(&mut world, &spec, &input, Some(offered), &[]);
        assert_eq!(inv.launch, LaunchTerminal::Mismatch, "{file}");
        assert!(inv.answer.is_none() && !world.seat().join("after_sleep.marker").exists());
        assert!(
            started_row(&emitted).is_none() && !conv_dir(world.operator_home(), stray).exists()
        );
        assert_eq!(
            std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
            b"original-db"
        );
        assert_eq!(
            std::fs::read_to_string(conv.join("transcript.jsonl")).unwrap(),
            "{\"turn\":1}\n"
        );
        assert_lock_free(&conv);
        assert!(world.homes_gone());
        let bin = world.seat().join("agy");
        assert!(ended(&world.run(&bin, &["--model", FLASH], input, Some(offered))).is_none());
    }
}

#[test]
fn l3_l5_unconfirmed_rejoin_puts_kept_state_back_and_confirmed_model_substitution_drops_it() {
    let invalid = format!(
        "{{\"event\":\"init\",\"conversation_id\":\"not-a-uuid\",\"init\":{{\"model\":\"{FLASH}\"}}}}\n"
    );
    let wrong_model = format!(
        "{{\"event\":\"init\",\"conversation_id\":\"{UUID_A}\",\"init\":{{\"model\":\"other-model\"}}}}\n"
    );
    for (label, stream, expected_launch, expect_refusal, keep_old) in [
        (
            "invalid-id-in-init",
            invalid.as_str(),
            LaunchTerminal::Unconfirmed,
            false,
            true,
        ),
        ("no-init-row", "", LaunchTerminal::Unconfirmed, false, true),
        (
            "model-substituted",
            wrong_model.as_str(),
            LaunchTerminal::Resumed,
            true,
            false,
        ),
    ] {
        let (mut world, conv, input) = seeded_rejoin_world(b"orig-db", b"orig-brain");
        let spec = FakeAgySpec::turn(UUID_A, stream, Some("mutated-db"), ("note.txt", "mutated"));
        let (inv, emitted) = drive_offer(&mut world, &spec, &input, Some(UUID_A), &[]);
        assert_eq!(inv.launch, expected_launch, "{label}");
        assert_eq!(inv.refusal.is_some(), expect_refusal, "{label}");
        assert!(started_row(&emitted).is_none(), "{label}");
        if keep_old {
            assert_eq!(
                std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
                b"orig-db",
                "{label}"
            );
        } else {
            assert!(
                !conv.join(REJOIN_DIR).exists(),
                "{label}: old rejoin/ must not return"
            );
        }
        assert_lock_free(&conv);
    }
}
#[test]
fn l5_l4_failed_keep_after_confirmed_rejoin_removes_old_rejoin_and_releases_lock() {
    let l2c_02 = recorded_box("l2c-02-rejoin-kept.ndjson");
    let cases = [
        ("missing-db", None, None, "absent"),
        (
            "bound-secret",
            Some("db-with-rejoin-secret-9999"),
            Some(("TOK", "rejoin-secret-9999")),
            "bound secret value appeared",
        ),
    ];
    for (label, next_db, binding, stderr_sub) in cases {
        let mut world = World::new();
        let conv = seed_kept(
            world.operator_home(),
            UUID_A,
            b"turn-1-db",
            &[("note.txt", b"turn-1-brain")],
        );
        let input = rejoin_input(&world, "1.5.18", "not applicable", "none");
        let spec = FakeAgySpec::turn(UUID_A, &l2c_02, next_db, ("note.txt", "turn-2-brain"));
        let bindings: Vec<_> = binding.map(|(k, v)| bound(k, v)).into_iter().collect();
        let (inv, emitted) = drive_offer(&mut world, &spec, &input, Some(UUID_A), &bindings);
        assert_eq!(inv.launch, LaunchTerminal::Resumed, "{label}");
        assert!(
            !conv.join(REJOIN_DIR).exists(),
            "{label}: old rejoin/ must be removed"
        );
        assert!(
            conv.join("transcript.jsonl").exists(),
            "{label}: transcript survives"
        );
        assert!(inv.stderr.contains(stderr_sub), "{label}: {}", inv.stderr);
        assert_lock_free(&conv);
        let started = started_row(&emitted)
            .unwrap_or_else(|| panic!("{label}: confirmed rejoin must publish launch: resumed"));
        assert_eq!(started["harness"], "agy", "{label}");
        assert_eq!(started["launch"], "resumed", "{label}");
        assert!(started.get("transcript").is_none(), "{label}");
        assert!(started.get("resume_refusal").is_none(), "{label}");
        assert_eq!(
            started["root_session"]["persistent"],
            json!(false),
            "{label}"
        );
    }
}

fn assert_declined_with(
    world: &mut World,
    input: &Value,
    offered: &str,
    version: (&str, i32),
    bindings: &[secret::BoundSecret],
    expected_refusal: &str,
    expected_stderr: Option<&str>,
) -> (Invocation, Vec<(Step, Value)>) {
    let cold_stdout = recorded_box("l2c-01-cold.ndjson").replace(UUID_A, UUID_B);
    let mut spec = FakeAgySpec::turn(
        UUID_B,
        &cold_stdout,
        Some("fresh-db"),
        ("note.txt", "fresh"),
    );
    spec.version = version;
    let (inv, emitted) = drive_offer(world, &spec, input, Some(offered), bindings);
    assert_eq!(inv.launch, LaunchTerminal::Cold);
    assert!(!read_argv(&world.seat())
        .iter()
        .any(|a| a == "--conversation"));
    assert!(
        !world.seat().join("restored.db").exists()
            && !world.seat().join("restored.note").exists()
            && !world.seat().join("had_restored_dirs.marker").exists()
    );
    let started = started_row(&emitted).expect("declined offer emits harness-started");
    assert_eq!(
        (&started["launch"], &started["resume_refusal"]),
        (&json!("cold"), &json!(expected_refusal))
    );
    if let Some(sub) = expected_stderr {
        assert!(
            inv.stderr.contains(sub),
            "expected {sub:?} in {:?}",
            inv.stderr
        );
    }
    assert!(world.homes_gone());
    (inv, emitted)
}

fn assert_declined(
    mut world: World,
    input: Value,
    offered: &str,
    version: (&str, i32),
    expected_refusal: &str,
    expected_stderr: Option<&str>,
) {
    assert_declined_with(
        &mut world,
        &input,
        offered,
        version,
        &[],
        expected_refusal,
        expected_stderr,
    );
}

#[test]
fn l2_gate_isolation_id_target_and_version_declines_run_cold() {
    let shipped: Value = serde_json::from_str(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/examples/adapters/agy.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let base = || {
        let world = World::new();
        let conv = seed_kept(world.operator_home(), UUID_A, b"db", &[("note.txt", b"b")]);
        let input = rejoin_input(&world, "1.5.18", "not applicable", "none");
        (world, conv, input)
    };
    let decline = |w, conv: PathBuf, inp, offered, ver, refusal| {
        assert_declined(w, inp, offered, ver, refusal, None);
        assert!(!conv.join(LOCK_FILE).exists());
    };

    let (w, conv, mut inp) = base();
    inp["resume_context"]["assessment"] = shipped["resume"].clone();
    decline(w, conv, inp, UUID_A, ("1.5.18", 0), "unsupported-resume");

    let (w, conv, mut inp) = base();
    inp["resume_context"]
        .as_object_mut()
        .unwrap()
        .remove("declared_root_session_kind");
    decline(w, conv, inp, UUID_A, ("1.5.18", 0), "unsupported-resume");

    let (w, conv, mut inp) = base();
    inp["boundary"] = json!("namespace");
    decline(
        w,
        conv,
        inp,
        UUID_A,
        ("1.5.18", 0),
        "restrictions-unavailable",
    );

    let (w, conv, _) = base();
    let binp = boxed_rejoin_input(&w, "1.5.18", false);
    decline(
        w,
        conv,
        binp,
        UUID_A,
        ("1.5.18", 0),
        "restrictions-unavailable",
    );

    let (w, conv, inp) = base();
    decline(
        w,
        conv,
        inp,
        "not-a-uuid",
        ("1.5.18", 0),
        "invalid-session-id",
    );

    let (w, conv, mut inp) = base();
    let other = w.path("other-home");
    std::fs::create_dir_all(&other).unwrap();
    inp["resume_context"]["owned_target"] =
        json!({"provider_id": UUID_A, "persistence_locator": UUID_A, "persistence_home": other});
    decline(w, conv, inp, UUID_A, ("1.5.18", 0), "instance-changed");

    let (w, conv, mut inp) = base();
    let home = conv_dir(w.operator_home(), UUID_A)
        .parent()
        .unwrap()
        .to_path_buf();
    inp["resume_context"]["owned_target"] =
        json!({"provider_id": UUID_B, "persistence_locator": UUID_A, "persistence_home": home});
    decline(w, conv, inp, UUID_A, ("1.5.18", 0), "unverified-harness");

    let (w, conv, inp) = base();
    decline(w, conv, inp, UUID_A, ("1.5.19", 0), "unverified-harness");

    let (w, conv, inp) = base();
    decline(w, conv, inp, UUID_A, ("", 1), "unverified-harness");

    let (w, conv, mut inp) = base();
    inp["resume_context"]["originating_harness_version"] = json!("1.5.14");
    decline(w, conv, inp, UUID_A, ("1.5.18", 0), "unverified-harness");
}

#[test]
fn l2_absent_missing_and_locked_kept_state_declines_run_cold() {
    let w = World::new();
    let parent = w
        .operator_home()
        .join(".local/state/brokkr/agy-conversations");
    std::fs::create_dir_all(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    let inp = rejoin_input(&w, "1.5.18", "not applicable", "none");
    assert_declined(
        w,
        inp,
        UUID_A,
        ("1.5.18", 0),
        "unverified-harness",
        Some("kept conversation state is absent"),
    );

    let w = World::new();
    let conv = seed_kept(w.operator_home(), UUID_A, b"db", &[("note.txt", b"b")]);
    std::fs::remove_file(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap();
    let inp = rejoin_input(&w, "1.5.18", "not applicable", "none");
    assert_declined(
        w,
        inp,
        UUID_A,
        ("1.5.18", 0),
        "unverified-harness",
        Some("absent"),
    );

    let w = World::new();
    let conv = seed_kept(w.operator_home(), UUID_A, b"db", &[("note.txt", b"b")]);
    let lock_path = conv.join(LOCK_FILE);
    std::fs::write(&lock_path, b"").unwrap();
    let held = openat(
        CWD,
        &lock_path,
        OFlags::RDWR | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .unwrap();
    flock(&held, FlockOperation::NonBlockingLockExclusive).unwrap();
    let inp = rejoin_input(&w, "1.5.18", "not applicable", "none");
    assert_declined(
        w,
        inp,
        UUID_A,
        ("1.5.18", 0),
        "unverified-harness",
        Some("another turn holds the conversation lock"),
    );
    let _ = flock(&held, FlockOperation::Unlock);
    drop(held);
}

fn assert_restore_fails(
    db: &[u8],
    files: &[(&str, &[u8])],
    caps: RejoinCaps,
    tamper: impl FnOnce(&Path),
    expected: RestoreNotice,
) {
    let dir = tempfile::tempdir().unwrap();
    let (op, eng) = (dir.path().join("op"), dir.path().join("eng"));
    std::fs::create_dir_all(eng.join(".gemini/antigravity-cli")).unwrap();
    let conv = seed_kept(&op, UUID_A, db, files);
    tamper(&conv);
    let err = lock_and_restore(UUID_A, &eng, Some(&op), caps)
        .err()
        .unwrap();
    assert_eq!(err, expected);
    let agy = eng.join(".gemini/antigravity-cli");
    assert!(!agy.join("conversations").exists() && !agy.join("brain").exists());
}

#[test]
fn l2_restore_bounds_types_and_rollback_leave_engine_home_clean() {
    assert_restore_fails(
        b"12345",
        &[("big.txt", b"1234567890")],
        RejoinCaps {
            bytes: 8,
            entries: 10,
            depth: 4,
        },
        |_| {},
        RestoreNotice::Copy(RejoinNotice::TooLarge(8)),
    );
    assert_restore_fails(
        b"db",
        &[("a.txt", b"1"), ("b.txt", b"2")],
        RejoinCaps {
            bytes: 1024,
            entries: 3,
            depth: 4,
        },
        |_| {},
        RestoreNotice::Copy(RejoinNotice::TooManyEntries(3)),
    );
    assert_restore_fails(
        b"db",
        &[("a/b/c.txt", b"x")],
        RejoinCaps {
            bytes: 1024,
            entries: 32,
            depth: 1,
        },
        |_| {},
        RestoreNotice::Copy(RejoinNotice::TooDeep(1)),
    );
    assert_restore_fails(
        b"db",
        &[],
        RejoinCaps::DEFAULT,
        |_| {},
        RestoreNotice::Copy(RejoinNotice::EmptyBrain),
    );
}

#[test]
fn l2_restore_unsafe_types_absent_and_unowned_rollback_cleanly() {
    let uns = RestoreNotice::Copy(RejoinNotice::Source(SourceErrno::Unsafe));
    type Tamper = (fn(&Path), RestoreNotice);
    let tampers: [Tamper; 7] = [
        (
            |c| {
                let db = c.join(REJOIN_DIR).join(REJOIN_DB_FILE);
                std::fs::remove_file(&db).unwrap();
                std::os::unix::fs::symlink("/etc/hosts", &db).unwrap();
            },
            uns.clone(),
        ),
        (
            |c| {
                let db = c.join(REJOIN_DIR).join(REJOIN_DB_FILE);
                std::fs::remove_file(&db).unwrap();
                std::fs::create_dir(&db).unwrap();
            },
            uns.clone(),
        ),
        (
            |c| {
                std::os::unix::fs::symlink(
                    "/etc/hosts",
                    c.join(REJOIN_DIR).join(REJOIN_BRAIN_DIR).join("link"),
                )
                .unwrap();
            },
            uns.clone(),
        ),
        (
            |c| std::os::unix::fs::symlink("/etc/hosts", c.join(LOCK_FILE)).unwrap(),
            uns.clone(),
        ),
        (|c| std::fs::create_dir(c.join(LOCK_FILE)).unwrap(), uns),
        (
            |c| std::fs::remove_dir_all(c.join(REJOIN_DIR)).unwrap(),
            RestoreNotice::Absent,
        ),
        (
            |c| std::fs::set_permissions(c, std::fs::Permissions::from_mode(0o750)).unwrap(),
            RestoreNotice::Parent(ParentError::UnownedParent),
        ),
    ];
    for (tamper, expected) in tampers {
        assert_restore_fails(
            b"db",
            &[("a.txt", b"1")],
            RejoinCaps::DEFAULT,
            tamper,
            expected,
        );
    }
}

#[test]
fn l2_restore_notice_display_owned_parent_and_replace_remove_edge_cases() {
    for notice in [
        RestoreNotice::Absent,
        RestoreNotice::LockHeld,
        RestoreNotice::ReferenceMasked,
        RestoreNotice::Io(std::io::ErrorKind::PermissionDenied),
        RestoreNotice::Parent(ParentError::NoOperatorHome),
        RestoreNotice::Copy(RejoinNotice::EmptyDatabase),
        RestoreNotice::Copy(RejoinNotice::EmptyBrain),
    ] {
        let text = notice.to_string();
        assert!(
            text.starts_with("agy rejoin skipped") && !text.contains(UUID_A),
            "{text}"
        );
    }
    let dir = tempfile::tempdir().unwrap();
    let (op, eng) = (dir.path().join("op"), dir.path().join("eng"));
    let conv = seed_kept(&op, UUID_A, b"old", &[("sub/deep.txt", b"old-deep")]);
    let parent = conv.parent().unwrap();
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o750)).unwrap();
    assert_eq!(
        open_existing_owned_parent(parent).unwrap_err(),
        ParentError::UnownedParent
    );
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

    let src_conv = eng.join(".gemini/antigravity-cli/conversations");
    let src_brain = eng.join(".gemini/antigravity-cli/brain").join(UUID_A);
    std::fs::create_dir_all(&src_conv).unwrap();
    std::fs::create_dir_all(src_brain.join("nested")).unwrap();
    std::fs::write(src_conv.join(format!("{UUID_A}.db")), b"new-db").unwrap();
    std::fs::write(src_brain.join("nested/new.txt"), b"new-deep").unwrap();
    std::fs::rename(conv.join(REJOIN_DIR), conv.join(".rejoin.aside.test")).unwrap();
    replace_rejoin(UUID_A, &eng, Some(&op), &[], RejoinCaps::DEFAULT).unwrap();
    assert_eq!(
        std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
        b"new-db"
    );
    assert!(conv
        .join(REJOIN_DIR)
        .join(REJOIN_BRAIN_DIR)
        .join("nested/new.txt")
        .exists());
    assert!(!conv
        .join(REJOIN_DIR)
        .join(REJOIN_BRAIN_DIR)
        .join("sub/deep.txt")
        .exists());

    let conv_fd = openat(
        CWD,
        &conv,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .unwrap();
    remove_entry_tree(
        std::os::fd::AsFd::as_fd(&conv_fd),
        std::ffi::OsStr::new(".rejoin.aside.test"),
        16,
    );
    assert!(!conv.join(".rejoin.aside.test").exists());
    remove_entry_tree(
        std::os::fd::AsFd::as_fd(&conv_fd),
        std::ffi::OsStr::new(".missing"),
        16,
    );
    std::fs::write(conv.join(".stray-file"), b"x").unwrap();
    remove_entry_tree(
        std::os::fd::AsFd::as_fd(&conv_fd),
        std::ffi::OsStr::new(".stray-file"),
        16,
    );
    assert!(!conv.join(".stray-file").exists());
    let zero_dir = conv.join(".budget-zero");
    std::fs::create_dir(&zero_dir).unwrap();
    std::fs::write(zero_dir.join("child"), b"x").unwrap();
    remove_entry_tree(
        std::os::fd::AsFd::as_fd(&conv_fd),
        std::ffi::OsStr::new(".budget-zero"),
        0,
    );
    assert!(zero_dir.join("child").exists());
    remove_entry_tree(
        std::os::fd::AsFd::as_fd(&conv_fd),
        std::ffi::OsStr::new(".budget-zero"),
        4,
    );
    assert!(!zero_dir.exists());
    let locked_dir = conv.join(".unreadable-dir");
    std::fs::create_dir(&locked_dir).unwrap();
    std::fs::write(locked_dir.join("inner"), b"x").unwrap();
    std::fs::set_permissions(&locked_dir, std::fs::Permissions::from_mode(0o000)).unwrap();
    remove_entry_tree(
        std::os::fd::AsFd::as_fd(&conv_fd),
        std::ffi::OsStr::new(".unreadable-dir"),
        4,
    );
    std::fs::set_permissions(&locked_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::remove_dir_all(&locked_dir).unwrap();
}

/// F1: every coordinate in `resume_context.owned_target` (`provider_id`,
/// `persistence_locator`, `persistence_home`) is required and compared, and any
/// absent, null, empty, wrong-type, unreadable, or mismatched value declines
/// cold. Also, a root session is `persistent: true` only when the kept
/// transcript address a later offer needs is published beside it.
#[test]
fn f1_owned_target_coordinates_fail_closed_and_persistent_requires_address() {
    let base = || {
        let world = World::new();
        let conv = seed_kept(world.operator_home(), UUID_A, b"db", &[("note.txt", b"b")]);
        let input = rejoin_input(&world, "1.5.18", "not applicable", "none");
        let home = world
            .operator_home()
            .join(".local/state/brokkr/agy-conversations");
        (world, conv, input, home)
    };

    let bad_fields: [(&str, Vec<Option<Value>>); 4] = [
        (
            "",
            vec![None, Some(json!(null)), Some(json!("str")), Some(json!([]))],
        ),
        (
            "provider_id",
            vec![
                None,
                Some(json!(null)),
                Some(json!("")),
                Some(json!(42)),
                Some(json!(UUID_B)),
            ],
        ),
        (
            "persistence_locator",
            vec![
                None,
                Some(json!(null)),
                Some(json!("")),
                Some(json!(7)),
                Some(json!(UUID_B)),
            ],
        ),
        (
            "persistence_home",
            vec![
                None,
                Some(json!(null)),
                Some(json!("")),
                Some(json!(true)),
                Some(json!("/no/such/agy-conversations-home-xyz")),
            ],
        ),
    ];
    for (field, cases) in bad_fields {
        for bad in cases {
            let (w, conv, mut inp, _) = base();
            let obj = if field.is_empty() {
                inp["resume_context"].as_object_mut().unwrap()
            } else {
                inp["resume_context"]["owned_target"]
                    .as_object_mut()
                    .unwrap()
            };
            let key = if field.is_empty() {
                "owned_target"
            } else {
                field
            };
            match bad {
                None => {
                    obj.remove(key);
                }
                Some(v) => {
                    obj.insert(key.into(), v);
                }
            }
            assert_declined(w, inp, UUID_A, ("1.5.18", 0), "unverified-harness", None);
            assert!(!conv.join(LOCK_FILE).exists());
        }
    }
    let (w, _conv, inp, home) = base();
    assert_eq!(
        check_owned_target(&inp, UUID_A, None),
        Err("unverified-harness")
    );
    let empty_op = tempfile::tempdir().unwrap();
    let mut inp_unreadable_admitted = inp.clone();
    inp_unreadable_admitted["resume_context"]["owned_target"]["persistence_home"] = json!(home);
    assert_eq!(
        check_owned_target(&inp_unreadable_admitted, UUID_A, Some(empty_op.path())),
        Err("unverified-harness")
    );
    drop(w);
}

#[test]
fn f1_root_session_persistent_requires_publishable_address() {
    let cold_stdout = recorded_box("l2c-01-cold.ndjson");
    for (label, declare_transcript, next_transcript) in [
        ("cold-missing-transcript", true, None),
        ("cold-undeclared-transcript", false, Some("{\"turn\":1}")),
    ] {
        let mut world = World::new();
        let mut input = rejoin_input(&world, "1.5.18", "not applicable", "none");
        if !declare_transcript {
            input["resume_context"]
                .as_object_mut()
                .unwrap()
                .remove("declared_transcript_kind");
        }
        let mut spec = FakeAgySpec::turn(UUID_A, &cold_stdout, Some("db-1"), ("note.txt", "b1"));
        spec.next_transcript = next_transcript;
        let (inv, emitted) = drive_offer(&mut world, &spec, &input, None, &[]);
        assert_eq!(inv.launch, LaunchTerminal::Cold, "{label}");
        assert!(
            !conv_dir(world.operator_home(), UUID_A)
                .join(REJOIN_DIR)
                .exists(),
            "{label}: rejoin/ must not be kept without a publishable address"
        );
        let started = started_row(&emitted).expect("harness-started emitted");
        assert_eq!(
            started["root_session"]["persistent"],
            json!(false),
            "{label}"
        );
        assert!(started.get("transcript").is_none(), "{label}");
        assert!(
            inv.stderr
                .contains("transcript address could not be published"),
            "{label}: {}",
            inv.stderr
        );
    }

    let l2c_02 = recorded_box("l2c-02-rejoin-kept.ndjson");
    let mut world = World::new();
    let conv = seed_kept(
        world.operator_home(),
        UUID_A,
        b"t1-db",
        &[("note.txt", b"t1")],
    );
    let input = rejoin_input(&world, "1.5.18", "not applicable", "none");
    let mut spec = FakeAgySpec::turn(UUID_A, &l2c_02, Some("t2-db"), ("note.txt", "t2"));
    spec.next_transcript = None;
    let (inv, emitted) = drive_offer(&mut world, &spec, &input, Some(UUID_A), &[]);
    assert_eq!(inv.launch, LaunchTerminal::Resumed);
    assert!(!conv.join(REJOIN_DIR).exists());
    let started = started_row(&emitted).expect("harness-started emitted");
    assert_eq!(started["launch"], "resumed");
    assert_eq!(started["root_session"]["persistent"], json!(false));
    assert!(started.get("transcript").is_none());
    assert!(inv
        .stderr
        .contains("transcript address could not be published"));
    assert_lock_free(&conv);
}

/// F2: workspace admission runs AFTER restore as the last check before spawn.
/// Planting workspace configuration during restore must refuse spawn, leave
/// agy unlaunched, and put the set-aside `rejoin/` back whole.
#[test]
fn f2_workspace_admission_runs_after_restore_before_spawn() {
    let l2c_02 = recorded_box("l2c-02-rejoin-kept.ndjson");
    let (mut world, conv, input) = seeded_rejoin_world(b"orig-db", b"orig-brain");
    let spec = FakeAgySpec::turn(UUID_A, &l2c_02, Some("t2-db"), ("note.txt", "t2"));
    let bin = write_fake_rejoin_agy(&world.seat(), &spec);
    world.set(&bin);
    let workdir = PathBuf::from(input["workdir"].as_str().unwrap());
    let planted = workdir.join(".agents/hooks.json");
    let mut planted_during_restore = false;
    let mode = StartMode {
        session: Some(UUID_A),
        probe: false,
    };
    let pinned = crate::adapters::Pinned {
        binary: bin.to_str().unwrap(),
        extra: &["--model".into(), FLASH.into()],
    };
    let res = super::super::turn_with(
        pinned,
        mode,
        ("hello", &input, &[]),
        &mut |_| {},
        &mut |point| {
            if matches!(point, CopyPoint::OpenEntry(_)) && !planted_during_restore {
                planted_during_restore = true;
                std::fs::create_dir_all(planted.parent().unwrap()).unwrap();
                std::fs::write(&planted, b"{}").unwrap();
            }
        },
    );
    assert!(planted_during_restore, "restore hook must have fired");
    let err = res
        .err()
        .expect("workspace config during restore must refuse spawn");
    assert!(
        matches!(
            err,
            Ended::Launch(LaunchError::Configured(ref path)) if path == &planted
        ),
        "expected LaunchError::Configured({planted:?}), got {err:?}"
    );
    assert!(
        !world.seat().join("last.argv").exists(),
        "agy must not be spawned when workspace config appears during restore"
    );
    assert_eq!(
        std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
        b"orig-db",
        "unspawned rejoin must put set-aside rejoin/ back whole"
    );
    assert_lock_free(&conv);
    assert!(world.homes_gone());
}

/// F3 & F4: after a confirmed rejoin, every failure outcome (`ERROR` result,
/// non-zero exit, malformed row, unconfirmed model pin) leaves the live path
/// holding either Turn 2's end state kept whole under slice 1's rules or
/// nothing — never Turn 1's stale state — and publishes `persistent: false`
/// whenever nothing was kept.
fn assert_confirmed_failure(
    label: &str,
    stream: &str,
    exit_code: i32,
    next_db: Option<&str>,
    expect_kept: bool,
    expected: &fold::Failed,
) {
    let (mut world, conv, input) = seeded_rejoin_world(b"turn-1-db", b"t1");
    let mut spec = FakeAgySpec::turn(UUID_A, stream, next_db, ("note.txt", "t2"));
    spec.exit_code = exit_code;
    let (res, emitted) = drive_offer_result(&mut world, &spec, &input, Some(UUID_A), &[], |_| {});
    match res {
        Err(Ended::Failed(actual)) => assert_eq!(&actual, expected, "{label}"),
        Err(other) => panic!("{label}: expected Ended::Failed({expected:?}), got {other:?}"),
        Ok(_) => panic!("{label}: expected Ended::Failed({expected:?}), got Ok"),
    }
    let started = started_row(&emitted).expect("harness-started emitted");
    assert_eq!(
        (&started["launch"], &started["root_session"]["persistent"]),
        (&json!("resumed"), &json!(expect_kept)),
        "{label}"
    );
    if expect_kept {
        let db = std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap();
        assert_eq!(db, b"turn-2-db", "{label}: live rejoin/ must hold Turn 2");
        assert_eq!(started["transcript"]["locator"], UUID_A, "{label}");
    } else {
        assert!(
            !conv.join(REJOIN_DIR).exists(),
            "{label}: stale rejoin/ survived"
        );
        assert!(started.get("transcript").is_none(), "{label}");
    }
    assert_lock_free(&conv);
}

#[test]
fn f3_f4_confirmed_rejoin_failures_never_leave_turn1_stale_state() {
    let init_no_model = format!(r#"{{"event":"init","conversation_id":"{UUID_A}","init":{{}}}}"#);
    let init_pinned =
        format!(r#"{{"event":"init","conversation_id":"{UUID_A}","init":{{"model":"{FLASH}"}}}}"#);
    let err_res = r#"{"event":"result","result":{"status":"ERROR","error":"boom"}}"#;
    let malformed = r#"{"event":"step_update","step_update":{"step_type":123}}"#;
    let err_fail = fold::Failed {
        failure: fold::Failure::Unsuccessful,
        seen: fold::Seen {
            status: Some("ERROR".into()),
            timed_out: false,
        },
    };
    let exit_fail = fold::Failed {
        failure: fold::Failure::Exited(4),
        seen: fold::Seen::default(),
    };
    let unread_fail = fold::Failed {
        failure: fold::Failure::Unread(fold::Event::StepUpdate),
        seen: fold::Seen::default(),
    };
    let cases = [
        (
            "unpinned-error",
            format!("{init_no_model}\n{err_res}\n"),
            0,
            Some("turn-2-db"),
            false,
            &err_fail,
        ),
        (
            "unpinned-nonzero",
            format!("{init_no_model}\n"),
            4,
            Some("turn-2-db"),
            false,
            &exit_fail,
        ),
        (
            "unpinned-malformed",
            format!("{init_no_model}\n{malformed}\n"),
            0,
            Some("turn-2-db"),
            false,
            &unread_fail,
        ),
        (
            "pinned-error-whole",
            format!("{init_pinned}\n{err_res}\n"),
            0,
            Some("turn-2-db"),
            true,
            &err_fail,
        ),
        (
            "pinned-nonzero-whole",
            format!("{init_pinned}\n"),
            4,
            Some("turn-2-db"),
            true,
            &exit_fail,
        ),
        (
            "pinned-malformed-whole",
            format!("{init_pinned}\n{malformed}\n"),
            0,
            Some("turn-2-db"),
            true,
            &unread_fail,
        ),
        (
            "pinned-error-empty",
            format!("{init_pinned}\n{err_res}\n"),
            0,
            None,
            false,
            &err_fail,
        ),
        (
            "pinned-nonzero-empty",
            format!("{init_pinned}\n"),
            4,
            None,
            false,
            &exit_fail,
        ),
        (
            "pinned-malformed-empty",
            format!("{init_pinned}\n{malformed}\n"),
            0,
            None,
            false,
            &unread_fail,
        ),
    ];
    for (label, stream, exit_code, next_db, expect_kept, expected) in cases {
        assert_confirmed_failure(label, &stream, exit_code, next_db, expect_kept, expected);
    }
}

/// F5: stopping between any two steps of take-out, staging, and rename leaves
/// the live `rejoin/` path either whole or empty — never torn and never old.
#[test]
fn f5_stop_at_every_step_leaves_live_path_whole_or_empty() {
    let (dir, op, eng, conv) = seeded_restore_dir(b"turn-1-db", b"turn-1");

    let mut lease = lock_and_restore(UUID_A, &eng, Some(&op), RejoinCaps::DEFAULT).unwrap();
    let aside = conv.join(lease.aside_name());
    assert!(
        !conv.join(REJOIN_DIR).exists(),
        "live path empty after take-out"
    );
    assert_eq!(
        std::fs::read(aside.join(REJOIN_DB_FILE)).unwrap(),
        b"turn-1-db"
    );

    let src_db = eng.join(format!(".gemini/antigravity-cli/conversations/{UUID_A}.db"));
    let src_note = eng.join(format!(".gemini/antigravity-cli/brain/{UUID_A}/note.txt"));
    std::fs::write(&src_db, b"turn-2-db").unwrap();
    std::fs::write(&src_note, b"turn-2").unwrap();

    let mut live_during_open = None;
    let mut live_during_verify = None;
    let err = replace_rejoin_with(
        UUID_A,
        &eng,
        Some(&op),
        &[],
        RejoinCaps::DEFAULT,
        &mut |pt| match pt {
            CopyPoint::OpenEntry(_) => live_during_open = Some(conv.join(REJOIN_DIR).exists()),
            CopyPoint::FinalVerify => {
                live_during_verify = Some(conv.join(REJOIN_DIR).exists());
                std::fs::write(&src_db, b"turn-2-db-mutated").unwrap();
            }
            _ => {}
        },
    )
    .unwrap_err();
    assert_eq!(err, RejoinNotice::ChangedUnderfoot);
    assert_eq!(live_during_open, Some(false), "live empty during copy");
    assert_eq!(live_during_verify, Some(false), "live empty before rename");
    assert!(
        !conv.join(REJOIN_DIR).exists(),
        "live still empty after failed stage"
    );

    std::fs::write(&src_db, b"turn-2-db-final").unwrap();
    replace_rejoin(UUID_A, &eng, Some(&op), &[], RejoinCaps::DEFAULT).unwrap();
    assert_eq!(
        std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
        b"turn-2-db-final"
    );

    lease.confirm();
    drop(lease);
    assert!(!aside.exists(), "confirmed drop removes set-aside copy");
    assert_eq!(
        std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
        b"turn-2-db-final"
    );
    assert_lock_free(&conv);

    let eng2 = dir.path().join("eng2");
    std::fs::create_dir_all(eng2.join(".gemini/antigravity-cli")).unwrap();
    let lease2 = lock_and_restore(UUID_A, &eng2, Some(&op), RejoinCaps::DEFAULT).unwrap();
    let aside2 = conv.join(lease2.aside_name());
    assert!(!conv.join(REJOIN_DIR).exists());
    assert!(aside2.exists());
    drop(lease2);
    assert!(!aside2.exists());
    assert_eq!(
        std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
        b"turn-2-db-final"
    );
    assert_lock_free(&conv);
}

/// F7: cold turns ignore malformed `step_update.usage` (including explicit
/// `null`) exactly as before this slice; rejoined turns accept absent
/// `step_update.usage` and fail `Unread(StepUpdate)` on malformed or `null`
/// `step_update.usage` under K1.
#[test]
fn f7_cold_ignores_malformed_step_usage_and_rejoin_fails_unread() {
    let init =
        format!(r#"{{"event":"init","conversation_id":"{UUID_A}","init":{{"model":"{FLASH}"}}}}"#);
    let bad_step_str = r#"{"event":"step_update","step_update":{"step_type":"agent_response","state":"DONE","usage":{"input_tokens":"not-a-number"}}}"#;
    let bad_step_dup = r#"{"event":"step_update","step_update":{"step_type":"agent_response","state":"DONE","usage":{"input_tokens":1,"input_tokens":2}}}"#;
    let bad_step_null = r#"{"event":"step_update","step_update":{"step_type":"agent_response","state":"DONE","usage":null}}"#;
    let absent_step =
        r#"{"event":"step_update","step_update":{"step_type":"agent_response","state":"DONE"}}"#;
    let ok_res = r#"{"event":"result","result":{"status":"SUCCESS","response":"COLD_OK","usage":{"input_tokens":100,"output_tokens":5},"num_turns":1}}"#;

    let run = |mut hold: LaunchHold, step: &str, tag: &str| {
        let mut fold = AgyFold::new(FLASH, &[]);
        let mut meta = Map::new();
        let mut turns = 0usize;
        for line in [init.as_str(), step, ok_res] {
            assert!(
                matches!(
                    fold.read(line.as_bytes(), &mut meta, &mut hold, &mut |_| turns += 1),
                    Line::Read
                ),
                "{tag}"
            );
        }
        (fold.verdict(0, false), meta, turns)
    };

    for (label, bad_step) in [
        ("usage-str", bad_step_str),
        ("usage-dup", bad_step_dup),
        ("usage-null", bad_step_null),
    ] {
        let cold_hold = LaunchHold::new(
            "agy",
            LaunchPlan::cold(Vec::new(), "agy-conversation", None),
        );
        let (verdict, meta, _) = run(cold_hold, bad_step, label);
        assert_eq!(
            verdict,
            super::super::fold::Verdict::Answer("COLD_OK".into()),
            "cold {label}"
        );
        assert_eq!(meta["input_tokens"], json!(100), "cold {label}");
        assert_eq!(meta["output_tokens"], json!(5), "cold {label}");

        let (verdict, _, _) = run(rejoin_hold(UUID_A), bad_step, label);
        assert_eq!(
            verdict,
            super::super::fold::Verdict::Failed(super::super::fold::Failed {
                failure: super::super::fold::Failure::Unread(super::super::fold::Event::StepUpdate),
                seen: super::super::fold::Seen {
                    status: Some("SUCCESS".into()),
                    timed_out: false,
                },
            }),
            "rejoin must reject malformed step_update.usage ({label})"
        );
    }

    let (verdict, meta, turns) = run(rejoin_hold(UUID_A), absent_step, "rejoin absent usage");
    assert_eq!(
        verdict,
        super::super::fold::Verdict::Answer("COLD_OK".into())
    );
    assert_eq!(turns, 1);
    assert_eq!(meta.get("input_tokens"), None);
    assert_eq!(meta.get("output_tokens"), None);
}

/// F10: exercise remaining reachable error branches in `restore.rs`.
#[test]
fn f10_restore_error_branches_and_transcript_staging_cleanup() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();

    let op_empty = dir.path().join("op-empty");
    std::fs::create_dir_all(&op_empty).unwrap();
    let eng_empty = dir.path().join("eng-empty");
    std::fs::create_dir_all(eng_empty.join(".gemini/antigravity-cli")).unwrap();
    assert_eq!(
        lock_and_restore(UUID_A, &eng_empty, Some(&op_empty), RejoinCaps::DEFAULT).err(),
        Some(RestoreNotice::Absent)
    );

    let op_tiny = dir.path().join("op-tiny-db");
    seed_kept(&op_tiny, UUID_A, b"0123456789", &[("a.txt", b"a")]);
    let eng_tiny = dir.path().join("eng-tiny-db");
    std::fs::create_dir_all(eng_tiny.join(".gemini/antigravity-cli")).unwrap();
    assert_eq!(
        lock_and_restore(
            UUID_A,
            &eng_tiny,
            Some(&op_tiny),
            RejoinCaps {
                bytes: 2,
                entries: 16,
                depth: 4
            },
        )
        .err(),
        Some(RestoreNotice::Copy(RejoinNotice::TooLarge(2)))
    );

    let file_parent = dir.path().join("not-a-dir");
    std::fs::write(&file_parent, b"x").unwrap();
    assert_eq!(
        open_existing_owned_parent(&file_parent).unwrap_err(),
        ParentError::UnownedParent
    );

    let op_file_conv = dir.path().join("op-file-conv");
    let parent = op_file_conv.join(".local/state/brokkr/agy-conversations");
    std::fs::create_dir_all(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(parent.join(UUID_A), b"not-a-dir").unwrap();
    let eng = dir.path().join("eng-ok");
    std::fs::create_dir_all(eng.join(".gemini/antigravity-cli")).unwrap();
    assert_eq!(
        lock_and_restore(UUID_A, &eng, Some(&op_file_conv), RejoinCaps::DEFAULT).err(),
        Some(RestoreNotice::Copy(RejoinNotice::Source(
            SourceErrno::Unsafe
        )))
    );

    let op_sym = dir.path().join("op-sym-lock");
    let conv_sym = seed_kept(&op_sym, UUID_A, b"db", &[("a.txt", b"a")]);
    symlink(conv_sym.join("transcript.jsonl"), conv_sym.join(LOCK_FILE)).unwrap();
    assert_eq!(
        lock_and_restore(UUID_A, &eng, Some(&op_sym), RejoinCaps::DEFAULT).err(),
        Some(RestoreNotice::Copy(RejoinNotice::Source(
            SourceErrno::Unsafe
        )))
    );

    let op_fifo = dir.path().join("op-fifo-lock");
    let conv_fifo = seed_kept(&op_fifo, UUID_A, b"db", &[("a.txt", b"a")]);
    let fifo_path = conv_fifo.join(LOCK_FILE);
    super::rejoin::make_fifo(&fifo_path);
    assert_eq!(
        lock_and_restore(UUID_A, &eng, Some(&op_fifo), RejoinCaps::DEFAULT).err(),
        Some(RestoreNotice::Copy(RejoinNotice::Source(
            SourceErrno::Unsafe
        )))
    );
}

#[test]
fn f10_restore_ro_caps_and_transcript_staging_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let eng = dir.path().join("eng-ok");
    std::fs::create_dir_all(eng.join(".gemini/antigravity-cli")).unwrap();
    let op_ro = dir.path().join("op-ro-lock");
    let conv_ro = seed_kept(&op_ro, UUID_A, b"db", &[("a.txt", b"a")]);
    std::fs::set_permissions(&conv_ro, std::fs::Permissions::from_mode(0o500)).unwrap();
    let ro_err = lock_and_restore(UUID_A, &eng, Some(&op_ro), RejoinCaps::DEFAULT).err();
    std::fs::set_permissions(&conv_ro, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        ro_err,
        Some(RestoreNotice::Io(std::io::ErrorKind::PermissionDenied))
    );

    std::fs::write(conv_ro.join(LOCK_FILE), b"").unwrap();
    std::fs::set_permissions(
        conv_ro.join(LOCK_FILE),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    std::fs::set_permissions(&conv_ro, std::fs::Permissions::from_mode(0o500)).unwrap();
    let eng_ro = dir.path().join("eng-ro");
    std::fs::create_dir_all(eng_ro.join(".gemini/antigravity-cli")).unwrap();
    let rename_err = lock_and_restore(UUID_A, &eng_ro, Some(&op_ro), RejoinCaps::DEFAULT).err();
    std::fs::set_permissions(&conv_ro, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        rename_err,
        Some(RestoreNotice::Io(std::io::ErrorKind::PermissionDenied))
    );
    assert!(!eng_ro
        .join(".gemini/antigravity-cli/conversations")
        .exists());
    assert!(!eng_ro.join(".gemini/antigravity-cli/brain").exists());

    let op_caps = dir.path().join("op-caps");
    seed_kept(&op_caps, UUID_A, b"db", &[("a.txt", b"a")]);
    let bad_eng = dir.path().join("eng-missing-agy");
    std::fs::create_dir_all(&bad_eng).unwrap();
    assert_eq!(
        lock_and_restore(UUID_A, &bad_eng, Some(&op_caps), RejoinCaps::DEFAULT).err(),
        Some(RestoreNotice::Copy(RejoinNotice::Source(
            SourceErrno::Absent
        )))
    );
    for cap_entries in [0usize, 2usize] {
        let op_cap = dir.path().join(format!("op-cap-{cap_entries}"));
        seed_kept(&op_cap, UUID_A, b"db", &[("a.txt", b"a")]);
        let e = dir.path().join(format!("eng-cap-{cap_entries}"));
        std::fs::create_dir_all(e.join(".gemini/antigravity-cli")).unwrap();
        assert_eq!(
            lock_and_restore(
                UUID_A,
                &e,
                Some(&op_cap),
                RejoinCaps {
                    bytes: 1024,
                    entries: cap_entries,
                    depth: 4,
                },
            )
            .err(),
            Some(RestoreNotice::Copy(RejoinNotice::TooManyEntries(
                cap_entries
            )))
        );
    }
}

#[test]
fn f10_stage_replace_transcript_unowned_and_oversized_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let op_srt = dir.path().join("op-srt");
    let conv_srt = seed_kept(&op_srt, UUID_A, b"db", &[("a.txt", b"a")]);
    let parent_fd = open_existing_owned_parent(conv_srt.parent().unwrap())
        .unwrap()
        .unwrap();
    let src_file = dir.path().join("big-transcript.jsonl");
    std::fs::write(&src_file, b"0123456789abcdef").unwrap();

    std::fs::set_permissions(&conv_srt, std::fs::Permissions::from_mode(0o750)).unwrap();
    let src_fd = openat(
        CWD,
        &src_file,
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .unwrap();
    assert_eq!(
        stage_replace_transcript(&parent_fd, UUID_A, src_fd, 64),
        Err(super::super::keep::KeepNotice::Parent(
            ParentError::UnownedParent
        ))
    );
    std::fs::set_permissions(&conv_srt, std::fs::Permissions::from_mode(0o700)).unwrap();

    let src_fd2 = openat(
        CWD,
        &src_file,
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .unwrap();
    assert_eq!(
        stage_replace_transcript(&parent_fd, UUID_A, src_fd2, 4),
        Err(super::super::keep::KeepNotice::TooLarge(4))
    );
    let staging_left = std::fs::read_dir(&conv_srt)
        .unwrap()
        .filter_map(Result::ok)
        .any(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(".transcript.next.")
        });
    assert!(
        !staging_left,
        "failed transcript stage must unlink temp file"
    );
}

mod guards;
