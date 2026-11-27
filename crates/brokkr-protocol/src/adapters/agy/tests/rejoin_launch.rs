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
use super::super::rejoin::{
    RejoinCaps, RejoinNotice, REJOIN_BRAIN_DIR, REJOIN_DB_FILE, REJOIN_DIR,
};
use super::super::restore::{
    lock_and_restore, open_existing_owned_parent, remove_kept_rejoin, replace_rejoin,
    RestoreNotice, LOCK_FILE,
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

fn rejoin_input(world: &World, version: &str, boundary: &str, hands: &str) -> Value {
    let mut input = world.input(None);
    input["boundary"] = json!(boundary);
    input["hands"] = json!(hands);
    input["resume_context"] = json!({
        "assessment": enabled_assessment(version, boundary, hands),
        "declared_transcript_kind": "agy-conversation",
        "declared_root_session_kind": "agy-conversation",
        "originating_harness_version": version,
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
    lock_probe: Option<&'a Path>,
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
            lock_probe: None,
            sleep_after_first: false,
            remove_old_note: false,
        }
    }
}

fn fake_agy_actions(seat_dir: &Path, spec: &FakeAgySpec<'_>, stdout_file: &Path) -> String {
    let mut out = String::new();
    if let Some(lock_path) = spec.lock_probe {
        out.push_str(&format!(
            "if flock -n '{}' true 2>/dev/null; then printf UNLOCKED > '{}/lock.state'; else printf LOCKED > '{}/lock.state'; fi\n",
            lock_path.display(), seat_dir.display(), seat_dir.display()
        ));
    }
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
    std::fs::write(&stdout_file, spec.stdout).unwrap();
    let actions = fake_agy_actions(seat_dir, spec, &stdout_file);
    let (d, id) = (seat_dir.display(), spec.id);
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" = \"--version\" ]; then printf '%s\\n' '{}'; exit {}; fi\n\
         printf '%s\\n' \"$@\" > '{d}/last.argv'\n\
         cp \"$HOME/.gemini/config/mcp_config.json\" '{d}/snap.mcp' 2>/dev/null || true\n\
         cp \"$HOME/.gemini/config/hooks.json\" '{d}/snap.hooks' 2>/dev/null || true\n\
         for p in \"conversations/{id}.db:restored.db\" \"brain/{id}/note.txt:restored.note\"; do\n\
           s=\"$HOME/.gemini/antigravity-cli/${{p%%:*}}\"; t=\"{d}/${{p##*:}}\"\n\
           if [ -f \"$s\" ]; then cp \"$s\" \"$t\"; else rm -f \"$t\"; fi\n\
         done\n\
         {actions}exit 0\n",
        spec.version.0, spec.version.1
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

fn assert_lock_free(conv: &Path) {
    let fd = openat(
        CWD,
        conv.join(LOCK_FILE),
        OFlags::RDWR | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .unwrap();
    flock(&fd, FlockOperation::NonBlockingLockExclusive).expect("conversation lock must be free");
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

fn drive_offer(
    world: &mut World,
    spec: &FakeAgySpec<'_>,
    input: &Value,
    session: Option<&str>,
    bindings: &[secret::BoundSecret],
) -> (Invocation, Vec<(Step, Value)>) {
    let bin = write_fake_rejoin_agy(&world.seat(), spec);
    world.set(&bin);
    let mut emitted = Vec::new();
    let mode = StartMode {
        session,
        probe: false,
    };
    let inv = drive_flash_ended(&bin, mode, input, bindings, &mut |e| {
        emitted.push((e.step, e.row.clone()))
    })
    .expect("drive_offer succeeds");
    (inv, emitted)
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

    let lock_path = conv.join(LOCK_FILE);
    let mut t2 = FakeAgySpec::turn(
        UUID_A,
        &l2c_02,
        Some("db-turn-2"),
        ("step2.txt", "brain-turn-2"),
    );
    t2.lock_probe = Some(&lock_path);
    t2.remove_old_note = true;
    let (inv2, emitted) = drive_offer(&mut world, &t2, &input, Some(UUID_A), &[]);
    assert_eq!(inv2.launch, LaunchTerminal::Resumed);
    assert_eq!(inv2.session_meta["input_tokens"], json!(9_801));
    assert!(world.homes_gone());

    let mut expected_argv = cold_argv;
    expected_argv.extend(["--conversation".to_string(), UUID_A.to_string()]);
    assert_eq!(read_argv(&seat_dir), expected_argv);
    assert_eq!(
        std::fs::read_to_string(seat_dir.join("snap.mcp")).unwrap(),
        cold_mcp
    );
    assert_eq!(
        std::fs::read_to_string(seat_dir.join("snap.hooks")).unwrap(),
        cold_hooks
    );
    assert_eq!(
        std::fs::read_to_string(seat_dir.join("restored.db")).unwrap(),
        "db-turn-1"
    );
    assert_eq!(
        std::fs::read_to_string(seat_dir.join("restored.note")).unwrap(),
        "brain-turn-1"
    );
    assert_eq!(
        std::fs::read_to_string(seat_dir.join("lock.state")).unwrap(),
        "LOCKED"
    );
    assert_lock_free(&conv);

    let started = started_row(&emitted).expect("harness-started emitted");
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

    let t3 = FakeAgySpec::turn(
        UUID_A,
        &l2c_03,
        Some("db-turn-3"),
        ("step3.txt", "brain-turn-3"),
    );
    let (inv3, _) = drive_offer(&mut world, &t3, &input, Some(UUID_A), &[]);
    assert_eq!(inv3.launch, LaunchTerminal::Resumed);
    assert_eq!(inv3.session_meta["input_tokens"], json!(10_049));
    assert_eq!(
        std::fs::read_to_string(seat_dir.join("restored.db")).unwrap(),
        "db-turn-2"
    );
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
        let input = rejoin_input(&world, "1.5.18", "not applicable", "none");
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
fn l3_l5_unconfirmed_rejoin_and_model_substitution_leave_kept_state_untouched() {
    let invalid = format!(
        "{{\"event\":\"init\",\"conversation_id\":\"not-a-uuid\",\"init\":{{\"model\":\"{FLASH}\"}}}}\n"
    );
    let wrong_model = format!(
        "{{\"event\":\"init\",\"conversation_id\":\"{UUID_A}\",\"init\":{{\"model\":\"other-model\"}}}}\n"
    );
    for (label, stream, expected_launch, expect_refusal) in [
        (
            "invalid-id-in-init",
            invalid.as_str(),
            LaunchTerminal::Unconfirmed,
            false,
        ),
        ("no-init-row", "", LaunchTerminal::Unconfirmed, false),
        (
            "model-substituted",
            wrong_model.as_str(),
            LaunchTerminal::Resumed,
            true,
        ),
    ] {
        let mut world = World::new();
        let conv = seed_kept(
            world.operator_home(),
            UUID_A,
            b"orig-db",
            &[("note.txt", b"orig-brain")],
        );
        let input = rejoin_input(&world, "1.5.18", "not applicable", "none");
        let spec = FakeAgySpec::turn(UUID_A, stream, Some("mutated-db"), ("note.txt", "mutated"));
        let (inv, emitted) = drive_offer(&mut world, &spec, &input, Some(UUID_A), &[]);
        assert_eq!(inv.launch, expected_launch, "{label}");
        assert_eq!(inv.refusal.is_some(), expect_refusal, "{label}");
        assert!(started_row(&emitted).is_none(), "{label}");
        assert_eq!(
            std::fs::read(conv.join(REJOIN_DIR).join(REJOIN_DB_FILE)).unwrap(),
            b"orig-db"
        );
        assert_lock_free(&conv);
    }
}

#[test]
fn l5_l4_failed_keep_after_confirmed_rejoin_removes_old_rejoin_and_releases_lock() {
    let l2c_02 = recorded_box("l2c-02-rejoin-kept.ndjson");
    let cases = [
        ("missing-db", None, None, true, "absent"),
        (
            "bound-secret",
            Some("db-with-rejoin-secret-9999"),
            Some(("TOK", "rejoin-secret-9999")),
            true,
            "bound secret value appeared",
        ),
        (
            "masked-row",
            Some("turn-2-db"),
            Some(("VER", "1.5.18")),
            false,
            "rewrite the root session row",
        ),
    ];
    for (label, next_db, binding, expect_row, stderr_sub) in cases {
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
        match (expect_row, started_row(&emitted)) {
            (true, Some(started)) => {
                assert_eq!(started["launch"], "resumed", "{label}");
                assert_eq!(
                    started["root_session"]["persistent"],
                    json!(false),
                    "{label}"
                );
                assert!(started.get("transcript").is_none(), "{label}");
            }
            (false, None) => {}
            other => panic!("{label}: unexpected harness-started {other:?}"),
        }
    }
}

fn assert_declined(
    mut world: World,
    input: Value,
    offered: &str,
    version: (&str, i32),
    expected_refusal: &str,
    expected_stderr: Option<&str>,
) {
    let cold_stdout = recorded_box("l2c-01-cold.ndjson").replace(UUID_A, UUID_B);
    let mut spec = FakeAgySpec::turn(
        UUID_B,
        &cold_stdout,
        Some("fresh-db"),
        ("note.txt", "fresh"),
    );
    spec.version = version;
    let (inv, emitted) = drive_offer(&mut world, &spec, &input, Some(offered), &[]);
    assert_eq!(inv.launch, LaunchTerminal::Cold);
    assert!(!read_argv(&world.seat())
        .iter()
        .any(|a| a == "--conversation"));
    assert!(
        !world.seat().join("restored.db").exists() && !world.seat().join("restored.note").exists()
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
        seed_kept(world.operator_home(), UUID_A, b"db", &[("note.txt", b"b")]);
        let input = rejoin_input(&world, "1.5.18", "not applicable", "none");
        (world, input)
    };
    let (w, mut inp) = base();
    inp["resume_context"]["assessment"] = shipped["resume"].clone();
    assert_declined(w, inp, UUID_A, ("1.5.18", 0), "unsupported-resume", None);

    let (w, mut inp) = base();
    inp["resume_context"]
        .as_object_mut()
        .unwrap()
        .remove("declared_root_session_kind");
    assert_declined(w, inp, UUID_A, ("1.5.18", 0), "unsupported-resume", None);

    let (w, mut inp) = base();
    inp["boundary"] = json!("namespace");
    assert_declined(
        w,
        inp,
        UUID_A,
        ("1.5.18", 0),
        "restrictions-unavailable",
        None,
    );

    let (w, _) = base();
    let binp = boxed_rejoin_input(&w, "1.5.18", false);
    assert_declined(
        w,
        binp,
        UUID_A,
        ("1.5.18", 0),
        "restrictions-unavailable",
        None,
    );

    let (w, inp) = base();
    assert_declined(
        w,
        inp,
        "not-a-uuid",
        ("1.5.18", 0),
        "invalid-session-id",
        None,
    );

    let (w, mut inp) = base();
    let other = w.path("other-home");
    std::fs::create_dir_all(&other).unwrap();
    inp["resume_context"]["owned_target"] =
        json!({"provider_id": UUID_A, "persistence_locator": UUID_A, "persistence_home": other});
    assert_declined(w, inp, UUID_A, ("1.5.18", 0), "instance-changed", None);

    let (w, mut inp) = base();
    let home = conv_dir(w.operator_home(), UUID_A)
        .parent()
        .unwrap()
        .to_path_buf();
    inp["resume_context"]["owned_target"] =
        json!({"provider_id": UUID_B, "persistence_locator": UUID_A, "persistence_home": home});
    assert_declined(w, inp, UUID_A, ("1.5.18", 0), "unverified-harness", None);

    let (w, inp) = base();
    assert_declined(w, inp, UUID_A, ("1.5.19", 0), "unverified-harness", None);

    let (w, inp) = base();
    assert_declined(w, inp, UUID_A, ("", 1), "unverified-harness", None);

    let (w, mut inp) = base();
    inp["resume_context"]["originating_harness_version"] = json!("1.5.14");
    assert_declined(w, inp, UUID_A, ("1.5.18", 0), "unverified-harness", None);
}

#[test]
fn l2_absent_missing_and_locked_kept_state_declines_run_cold() {
    let w = World::new();
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
        RestoreNotice::Io(std::io::ErrorKind::PermissionDenied),
        RestoreNotice::Parent(ParentError::NoOperatorHome),
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
    remove_kept_rejoin(UUID_A, Some(&op));
    assert!(!conv.join(REJOIN_DIR).exists());
    remove_kept_rejoin(UUID_A, Some(&op));
    std::fs::write(conv.join(REJOIN_DIR), b"stray").unwrap();
    remove_kept_rejoin(UUID_A, Some(&op));
    assert!(!conv.join(REJOIN_DIR).exists());
}
