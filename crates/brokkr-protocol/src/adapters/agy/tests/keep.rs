//! Transcript retention tests for `brokkr driver agy` (#1227, decision 0075
//! level 2): UUID validation, no-symlink traversal, regular-file check,
//! owner-only parent and 0700/0600 permissions, copy cap, and keeping on
//! every ending that produced an `init` row.

use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::judged::{bound, driven_with};
use super::*;
use crate::adapters::agy::keep::{keep, keep_with_cap, owned_parent_with, ConvGuard, KeepNotice};
use crate::transcript::driver_kind;
use brokkr_core::seat::is_agy_uuid;

pub(super) const UUID: &str = "32786eb8-d649-4c35-9110-c8c87a1cdb7d";
/// `l2b-printed-value.transcript.ndjson`, read at test run time: the evidence
/// lives outside `crates/brokkr-protocol`, so no `include_str!` in `src/` reaches
/// it (`packaging::no_crate_includes_a_file_from_outside_itself`).
fn sample_transcript() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/evidence/adapters/agy-cli-1-5/l2b-printed-value.transcript.ndjson"),
    )
    .expect("committed l2b-printed-value evidence transcript")
}

/// Write a `transcript.jsonl` under `engine_home` at agy's exact path and
/// agy's measured 0644 mode.
fn plant_transcript(engine_home: &Path, id: &str, bytes: &[u8]) -> PathBuf {
    let logs = engine_home
        .join(".gemini/antigravity-cli/brain")
        .join(id)
        .join(".system_generated/logs");
    std::fs::create_dir_all(&logs).unwrap();
    let file = logs.join("transcript.jsonl");
    std::fs::write(&file, bytes).unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    file
}

/// #1227 U2(a): `agy-conversation` is an adapter-declared v8 kind, never a
/// built-in driver kind admitted without a declaring adapter.
#[test]
fn driver_kind_returns_none_for_agy_so_only_a_declaring_adapter_admits_it() {
    assert_eq!(driver_kind(crate::adapters::AdapterKind::Agy), None);
}

#[test]
fn non_uuid_conversation_id_is_refused_before_naming_any_path() {
    assert!(is_agy_uuid(UUID));
    let dir = tempfile::tempdir().unwrap();
    let engine_home = dir.path().join("engine");
    let op_home = dir.path().join("op");
    std::fs::create_dir_all(&engine_home).unwrap();
    std::fs::create_dir_all(&op_home).unwrap();

    // Plant a real file reachable via path traversal if validation were skipped.
    let escaped = engine_home.join(".gemini/antigravity-cli/.system_generated/logs");
    std::fs::create_dir_all(&escaped).unwrap();
    std::fs::write(escaped.join("transcript.jsonl"), sample_transcript()).unwrap();

    for bad in [
        "..",
        "../outside",
        "32786EB8-d649-4c35-9110-c8c87a1cdb7d",
        "32786eb8-d649-4c35-9110-c8c87a1cdb7",
        "32786eb8-d649-4c35-9110-c8c87a1cdb7de",
        "32786eb8_d649_4c35_9110_c8c87a1cdb7d",
        "not-a-uuid",
        "",
    ] {
        assert!(!is_agy_uuid(bad), "{bad}");
        let err = keep(bad, &engine_home, Some(&op_home), &[]).unwrap_err();
        assert_eq!(err, KeepNotice::InvalidId, "{bad}");
        let text = err.to_string();
        assert!(
            !text.contains(bad) || bad.is_empty(),
            "notice echoed id: {text}"
        );
    }
    assert!(!op_home
        .join(".local/state/brokkr/agy-conversations")
        .exists());
}

#[test]
fn kept_directory_is_0700_and_kept_file_is_0600_despite_agys_0644_source() {
    let dir = tempfile::tempdir().unwrap();
    let engine_home = dir.path().join("engine");
    let op_home = dir.path().join("op");
    std::fs::create_dir_all(&op_home).unwrap();
    let src = plant_transcript(&engine_home, UUID, sample_transcript().as_bytes());
    assert_eq!(std::fs::metadata(&src).unwrap().mode() & 0o777, 0o644);

    let kept = keep(UUID, &engine_home, Some(&op_home), &[]).unwrap();
    let parent = op_home.join(".local/state/brokkr/agy-conversations");
    assert_eq!(kept.home(), parent);
    assert_eq!(kept.locator(), UUID);

    let conv_dir = parent.join(UUID);
    let dest = conv_dir.join("transcript.jsonl");
    assert_eq!(std::fs::metadata(&parent).unwrap().mode() & 0o777, 0o700);
    assert_eq!(std::fs::metadata(&conv_dir).unwrap().mode() & 0o777, 0o700);
    assert_eq!(std::fs::metadata(&dest).unwrap().mode() & 0o777, 0o600);
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), sample_transcript());

    // Re-keeping the same UUID refuses without overwriting.
    assert_eq!(
        keep(UUID, &engine_home, Some(&op_home), &[]),
        Err(KeepNotice::AlreadyKept)
    );
}

#[test]
fn symlink_at_any_component_from_home_to_transcript_is_refused() {
    let target_dir = tempfile::tempdir().unwrap();
    let decoy = target_dir.path().join("decoy.jsonl");
    std::fs::write(&decoy, sample_transcript()).unwrap();

    // Test a symlink at the leaf and at every intermediate directory component.
    let components = [
        ".gemini",
        ".gemini/antigravity-cli",
        ".gemini/antigravity-cli/brain",
        &format!(".gemini/antigravity-cli/brain/{UUID}"),
        &format!(".gemini/antigravity-cli/brain/{UUID}/.system_generated"),
        &format!(".gemini/antigravity-cli/brain/{UUID}/.system_generated/logs"),
        &format!(".gemini/antigravity-cli/brain/{UUID}/.system_generated/logs/transcript.jsonl"),
    ];

    for link_rel in components {
        let dir = tempfile::tempdir().unwrap();
        let engine_home = dir.path().join("engine");
        let real_tree = dir.path().join("real");
        let op_home = dir.path().join("op");
        std::fs::create_dir_all(&engine_home).unwrap();
        std::fs::create_dir_all(&op_home).unwrap();
        plant_transcript(&real_tree, UUID, sample_transcript().as_bytes());

        let link_path = engine_home.join(link_rel);
        std::fs::create_dir_all(link_path.parent().unwrap()).unwrap();
        let target = if link_rel.ends_with("transcript.jsonl") {
            decoy.clone()
        } else {
            real_tree.join(link_rel)
        };
        symlink(&target, &link_path).unwrap();

        let err = keep(UUID, &engine_home, Some(&op_home), &[]).unwrap_err();
        assert_eq!(err, KeepNotice::UnsafeSource, "symlink at {link_rel}");
        assert!(
            !op_home
                .join(".local/state/brokkr/agy-conversations")
                .join(UUID)
                .exists(),
            "nothing kept for symlink at {link_rel}"
        );
    }
}

#[test]
fn non_regular_leaf_and_non_private_parent_and_oversized_source_are_refused() {
    // 1. Directory at transcript.jsonl -> UnsafeSource
    let dir = tempfile::tempdir().unwrap();
    let engine_home = dir.path().join("engine");
    let op_home = dir.path().join("op");
    std::fs::create_dir_all(&op_home).unwrap();
    let as_dir = engine_home
        .join(".gemini/antigravity-cli/brain")
        .join(UUID)
        .join(".system_generated/logs/transcript.jsonl");
    std::fs::create_dir_all(&as_dir).unwrap();
    assert_eq!(
        keep(UUID, &engine_home, Some(&op_home), &[]),
        Err(KeepNotice::UnsafeSource)
    );

    // 2. Copy bound: exact cap succeeds, cap + 1 fails and leaves no partial dir.
    let dir2 = tempfile::tempdir().unwrap();
    let engine2 = dir2.path().join("engine");
    let op2 = dir2.path().join("op");
    std::fs::create_dir_all(&op2).unwrap();
    plant_transcript(&engine2, UUID, b"12345");
    assert_eq!(
        keep_with_cap(UUID, &engine2, Some(&op2), 4),
        Err(KeepNotice::TooLarge(4))
    );
    assert!(!op2
        .join(".local/state/brokkr/agy-conversations")
        .join(UUID)
        .exists());
    assert!(keep_with_cap(UUID, &engine2, Some(&op2), 5).is_ok());

    // 3. Parent directory with group/other bits -> UnownedParent.
    let dir3 = tempfile::tempdir().unwrap();
    let engine3 = dir3.path().join("engine");
    let op3 = dir3.path().join("op");
    plant_transcript(&engine3, UUID, b"ok");
    let parent3 = op3.join(".local/state/brokkr/agy-conversations");
    std::fs::create_dir_all(&parent3).unwrap();
    std::fs::set_permissions(&parent3, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        keep(UUID, &engine3, Some(&op3), &[]),
        Err(KeepNotice::UnownedParent)
    );
}

/// Fake `agy` script that writes `transcript.jsonl` (mode 0644) inside its
/// engine HOME under `id` before emitting `stdout` and exiting `code`.
fn fake_agy_with_transcript(
    dir: &Path,
    id: &str,
    transcript: &[u8],
    stdout: &[u8],
    delivers: bool,
    code: i32,
) -> PathBuf {
    std::fs::write(dir.join("stdout"), stdout).unwrap();
    std::fs::write(dir.join("transcript.src"), transcript).unwrap();
    let result = if delivers {
        "target=$(printf '%s' \"$line\" | sed -n 's/.*\\\\n    \\(\\/[^\\\\\"]*\\.json\\)\\\\n.*/\\1/p' | head -1)\n\
         [ -n \"$target\" ] && printf '{\"result\":\"complete\"}' > \"$target\"\n"
    } else {
        ""
    };
    let script = format!(
        "#!/bin/sh\nd='{}'\nline=$(cat)\n\
         logs=\"$HOME/.gemini/antigravity-cli/brain/{id}/.system_generated/logs\"\n\
         mkdir -p \"$logs\"\n\
         cp \"$d/transcript.src\" \"$logs/transcript.jsonl\"\n\
         chmod 0644 \"$logs/transcript.jsonl\"\n\
         cat \"$d/stdout\"\n{result}exit {code}\n",
        dir.display()
    );
    let bin = dir.join("agy");
    std::fs::write(&bin, script).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

fn declared_input(world: &World) -> Value {
    let mut input = world.input(None);
    input["resume_context"] = json!({
        "assessment": Value::Null,
        "declared_transcript_kind": "agy-conversation",
    });
    input
}

fn run_seat_with_input(input: Value, probe: bool) -> Vec<crate::Body> {
    let mut start = json!({"attempt_id": "attempt", "effect_id": "effect", "input": input});
    if probe {
        start["probe"] = json!(true);
    }
    let mut messages = Vec::new();
    crate::adapters::run_seat(
        AdapterKind::Agy,
        &argv(&["--model", FLASH]),
        &start,
        None,
        &mut |body| messages.push(body),
    );
    messages
}

pub(super) fn run_seat_agy(
    world: &mut World,
    bin: &Path,
    input: Value,
    probe: bool,
) -> Vec<crate::Body> {
    world.set(bin);
    run_seat_with_input(input, probe)
}

fn init_row(model: &str) -> String {
    format!(
        "{{\"event\":\"init\",\"conversation_id\":\"{UUID}\",\"init\":{{\"model\":\"{model}\"}}}}"
    )
}

pub(super) const STEP_DONE: &str =
    r#"{"event":"step_update","step_update":{"step_type":"agent_response","state":"DONE"}}"#;
pub(super) const OK_RES: &str =
    r#"{"event":"result","result":{"status":"SUCCESS","response":"DONE","num_turns":1}}"#;

fn clean_turn_stdout() -> String {
    format!("{}\n{STEP_DONE}\n{OK_RES}\n", init_row(FLASH))
}

fn clean_world() -> (World, PathBuf) {
    let stdout = clean_turn_stdout();
    let world = World::new();
    let bin = fake_agy_with_transcript(
        &world.seat(),
        UUID,
        sample_transcript().as_bytes(),
        stdout.as_bytes(),
        true,
        0,
    );
    (world, bin)
}

fn no_model_ok_stdout() -> String {
    let init_no_model =
        format!("{{\"event\":\"init\",\"conversation_id\":\"{UUID}\",\"init\":{{}}}}");
    let ok_res =
        r#"{"event":"result","result":{"status":"SUCCESS","response":"OK","num_turns":1}}"#;
    format!("{init_no_model}\n{ok_res}\n")
}

pub(super) fn kept_parent(operator_home: &Path) -> PathBuf {
    operator_home.join(".local/state/brokkr/agy-conversations")
}

fn kept_file(operator_home: &Path) -> PathBuf {
    kept_parent(operator_home)
        .join(UUID)
        .join("transcript.jsonl")
}

fn expected_kept_record(expected_home: &Path) -> Value {
    json!({
        "kind": "agy-conversation",
        "locator": UUID,
        "home": expected_home.to_string_lossy(),
    })
}

fn fake_agy_empty_result(dir: &Path, name: &str, stdout: &str) -> (PathBuf, PathBuf) {
    let payload = dir.join(format!("{name}-transcript.jsonl"));
    std::fs::write(&payload, sample_transcript()).unwrap();
    let out_file = dir.join(format!("{name}-stdout.ndjson"));
    std::fs::write(&out_file, stdout).unwrap();
    let result_path = dir.join("result.json");
    let bin = dir.join(name);
    let script = format!(
        "#!/bin/sh\ncat >/dev/null\n         mkdir -p \"$HOME/.gemini/antigravity-cli/brain/{UUID}/.system_generated/logs\"\n         cp '{}' \"$HOME/.gemini/antigravity-cli/brain/{UUID}/.system_generated/logs/transcript.jsonl\"\n         : > '{}'\ncat '{}'\nexit 0\n",
        payload.display(),
        result_path.display(),
        out_file.display(),
    );
    std::fs::write(&bin, script).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    (bin, result_path)
}

fn run_masked_case(secret_value: &str) -> (World, PathBuf, Value, Vec<crate::Body>) {
    let (mut world, bin) = clean_world();
    let store = world.path("secrets.env");
    secret::store_set(&store, "MASK_TEST", secret_value).unwrap();
    let mut input = declared_input(&world);
    input["secrets"] = json!(["MASK_TEST"]);
    input["secrets_file"] = json!(store);
    let messages = run_seat_agy(&mut world, &bin, input.clone(), false);
    (world, bin, input, messages)
}

fn checkpoint_transcripts(messages: &[crate::Body]) -> Vec<Value> {
    messages
        .iter()
        .filter_map(|body| match body {
            crate::Body::Checkpoint { data, .. } => data.get("transcript").cloned(),
            _ => None,
        })
        .collect()
}

struct EndingCase {
    label: &'static str,
    stdout: String,
    delivers: bool,
    code: i32,
    probe: bool,
    expect_kept: bool,
    expect_notice: Option<KeepNotice>,
}

fn assert_ending_case(case: &EndingCase) {
    let mut world = World::new();
    let bin = fake_agy_with_transcript(
        &world.seat(),
        UUID,
        sample_transcript().as_bytes(),
        case.stdout.as_bytes(),
        case.delivers,
        case.code,
    );
    let input = declared_input(&world);
    let messages = run_seat_agy(&mut world, &bin, input, case.probe);
    assert!(world.homes_gone(), "{}: engine HOME removed", case.label);

    let expected_home = world
        .operator_home()
        .join(".local/state/brokkr/agy-conversations");
    let kept = expected_home.join(UUID).join("transcript.jsonl");
    let transcripts = checkpoint_transcripts(&messages);
    let expected_record = json!({
        "kind": "agy-conversation",
        "locator": UUID,
        "home": expected_home.to_string_lossy(),
    });
    let record_names_it = transcripts.iter().any(|t| t == &expected_record);

    assert_eq!(
        kept.exists(),
        case.expect_kept,
        "{}: kept existence",
        case.label
    );
    assert_eq!(
        record_names_it, case.expect_kept,
        "{}: record names file iff kept ({transcripts:?})",
        case.label
    );
    if case.expect_kept {
        assert_eq!(std::fs::metadata(&kept).unwrap().mode() & 0o777, 0o600);
        assert_eq!(std::fs::read_to_string(&kept).unwrap(), sample_transcript());
        return;
    }
    if let Some(notice) = &case.expect_notice {
        drop(world);
        assert_direct_notice(case, notice);
    }
}

fn assert_direct_notice(case: &EndingCase, notice: &KeepNotice) {
    let mut direct_world = World::new();
    let direct_bin = fake_agy_with_transcript(
        &direct_world.seat(),
        UUID,
        sample_transcript().as_bytes(),
        case.stdout.as_bytes(),
        case.delivers,
        case.code,
    );
    direct_world.set(&direct_bin);
    let direct_input = declared_input(&direct_world);
    let inv = super::drive_flash(
        &direct_bin,
        StartMode {
            offered: false,
            probe: case.probe,
        },
        &direct_input,
        &[],
    )
    .unwrap_or_else(|e| panic!("{}: direct drive returns Ok, got {e}", case.label));
    assert!(
        inv.stderr.contains(&notice.to_string()),
        "{}: expected notice in stderr: {}",
        case.label,
        inv.stderr
    );
    assert_eq!(
        inv.session_meta["transcript"],
        json!({"kind": "none", "locator": "", "home": ""}),
        "{}",
        case.label
    );
}

/// #1227 U4 invariant: after any ending through `run_seat`, a file exists in
/// `<operator_home>/.local/state/brokkr/agy-conversations/<uuid>/transcript.jsonl`
/// if and only if a sent `Body::Checkpoint` names it as `agy-conversation`.
#[test]
fn kept_file_exists_iff_seat_record_names_it_across_all_endings() {
    let init = init_row(FLASH);
    let denied_res = r#"{"event":"result","result":{"status":"SUCCESS","denied_actions":[{"action":"command"}]}}"#;
    let sub_init = init_row("gemini-4-neon");
    let timeout_res = r#"{"event":"result","result":{"status":"ERROR","error":"print timeout"}}"#;

    let cases = [
        EndingCase {
            label: "clean",
            stdout: clean_turn_stdout(),
            delivers: true,
            code: 0,
            probe: false,
            expect_kept: true,
            expect_notice: None,
        },
        EndingCase {
            label: "failed-denied",
            stdout: format!("{init}\n{STEP_DONE}\n{denied_res}\n"),
            delivers: false,
            code: 0,
            probe: false,
            expect_kept: true,
            expect_notice: None,
        },
        EndingCase {
            label: "late-print-timeout-after-work",
            stdout: format!("{init}\n{STEP_DONE}\n{timeout_res}\n"),
            delivers: false,
            code: 3,
            probe: false,
            expect_kept: true,
            expect_notice: None,
        },
        EndingCase {
            label: "refused-to-start-substituted",
            stdout: format!("{sub_init}\n{OK_RES}\n"),
            delivers: false,
            code: 0,
            probe: false,
            expect_kept: false,
            expect_notice: Some(KeepNotice::RefusedToStart),
        },
        EndingCase {
            label: "refused-to-start-print-timeout",
            stdout: format!("{init}\n{timeout_res}\n"),
            delivers: false,
            code: 3,
            probe: false,
            expect_kept: false,
            expect_notice: Some(KeepNotice::RefusedToStart),
        },
    ];
    for case in &cases {
        assert_ending_case(case);
    }
}

/// #1227 U2(b): when the launch does NOT carry
/// `resume_context.declared_transcript_kind: "agy-conversation"` (for example an
/// inline `{brokkr} driver agy -- --model ...` seat with no adapter), the driver
/// keeps nothing and records `none` with no notice, matching main.
#[test]
fn undeclared_launch_keeps_nothing_and_records_none_without_notice() {
    let (mut world, bin) = clean_world();
    world.set(&bin);
    let input = world.input(None);
    let inv = super::drive_flash(&bin, StartMode::default(), &input, &[])
        .expect("undeclared launch succeeds");
    assert_eq!(inv.stderr, "");
    assert_eq!(
        inv.session_meta["transcript"],
        json!({"kind": "none", "locator": "", "home": ""})
    );
    let kept = world
        .operator_home()
        .join(".local/state/brokkr/agy-conversations")
        .join(UUID)
        .join("transcript.jsonl");
    assert!(!kept.exists(), "undeclared launch must keep nothing");
}

/// #1227 U3 & U5:
/// - No `init` row: `none`, no keep notice.
/// - Signed-out WITH `init`: notice is reported beside `SIGN_IN_WITHHELD` via
///   `stderr::join_note` (decided by typed state, never by comparing stderr text).
/// - Stderr without a trailing newline: `stderr::join_note` inserts a newline
///   before the keep notice.
/// - `init` with numeric `conversation_id: 42` or absent `conversation_id`:
///   model confirmation and turn verdict match main, `none` is recorded, and
///   `KeepNotice::InvalidId` is reported.
#[test]
fn keep_notices_join_cleanly_and_invalid_ids_preserve_fold_verdict() {
    let drive_declared = |stdout: &[u8],
                          stderr: &str,
                          code: i32,
                          delivers: bool,
                          bindings: &[secret::BoundSecret]| {
        let mut world = World::new();
        let bin = fake_agy(
            &world.seat(),
            std::str::from_utf8(stdout).unwrap(),
            stderr,
            delivers,
            code,
        );
        world.set(&bin);
        let input = declared_input(&world);
        let turn = super::drive_flash(&bin, StartMode::default(), &input, bindings);
        assert!(world.homes_gone());
        turn
    };

    // 1. No init row (SIGNED_OUT): records none, NO keep notice in stderr.
    let inv = drive_declared(SIGNED_OUT.as_bytes(), "", 1, false, &[]).unwrap();
    assert_eq!(inv.stderr, SIGN_IN_WITHHELD);
    assert_eq!(
        inv.session_meta["transcript"],
        json!({"kind": "none", "locator": "", "home": ""})
    );

    // 2. Signed-out WITH an init row (U3): notice is joined beside SIGN_IN_WITHHELD.
    let signed_out_with_init = format!(
        "{{\"event\":\"init\",\"conversation_id\":\"{UUID}\",\"init\":{{\"model\":\"{FLASH}\"}}}}\n         {SIGNED_OUT}"
    );
    let inv = drive_declared(signed_out_with_init.as_bytes(), "", 1, false, &[]).unwrap();
    assert_eq!(
        inv.stderr,
        format!("{SIGN_IN_WITHHELD}{}\n", KeepNotice::RefusedToStart)
    );
    assert_eq!(
        inv.session_meta["transcript"],
        json!({"kind": "none", "locator": "", "home": ""})
    );

    // 3. Stderr without trailing newline + absent transcript (U3): join_note inserts newline.
    let inv = drive_declared(TOOL_LESS.as_bytes(), "partial line", 0, true, &[]).unwrap();
    assert_eq!(inv.answer, Some(Value::String("READY\n".into())));
    assert_eq!(
        inv.stderr,
        format!("partial line\n{}\n", KeepNotice::Absent)
    );

    // 4. Hostile non-UUID string, numeric id (42), and absent id (U5):
    //    model pin still confirms and turn ends cleanly as on main.
    let bindings = [bound("TOK", "unused")];
    let dup_num_str = format!(r#""conversation_id":42,"conversation_id":"{UUID}","#);
    let dup_str_str = format!(r#""conversation_id":"{UUID}","conversation_id":"{UUID}","#);
    for (label, id_member, forbidden) in [
        (
            "hostile-string",
            r#""conversation_id":"../../secret-seat-path","#,
            "../../secret-seat-path",
        ),
        ("numeric-42", r#""conversation_id":42,"#, "42"),
        ("absent-id", "", ""),
        ("duplicate-num-then-str", dup_num_str.as_str(), UUID),
        ("duplicate-str-twice", dup_str_str.as_str(), UUID),
    ] {
        let stdout = format!(
            "{{\"event\":\"init\",{id_member}\"init\":{{\"model\":\"{FLASH}\"}}}}\n             {{\"event\":\"result\",\"result\":{{\"status\":\"SUCCESS\",\"response\":\"OK\"}}}}\n"
        );
        let inv = drive_declared(stdout.as_bytes(), "", 0, true, &bindings)
            .unwrap_or_else(|e| panic!("{label}: expected clean completion, got {e}"));
        assert_eq!(inv.answer, Some(Value::String("OK".into())), "{label}");
        assert_eq!(inv.session_meta["model"], FLASH, "{label}");
        assert_eq!(
            inv.session_meta["transcript"],
            json!({"kind": "none", "locator": "", "home": ""}),
            "{label}"
        );
        assert_eq!(
            inv.stderr,
            format!("{}\n", KeepNotice::InvalidId),
            "{label}"
        );
        if !forbidden.is_empty() {
            assert!(!inv.stderr.contains(forbidden), "{label}: {}", inv.stderr);
        }
    }

    // Silence unused import warning when driven_with isn't called above.
    let _ = driven_with;
}

/// Build a relative path from `std::env::current_dir()` to `target` without
/// changing the process working directory (thread-safe under parallel tests).
fn relative_path_to(target: &Path) -> PathBuf {
    let cwd = std::env::current_dir().unwrap().canonicalize().unwrap();
    let abs = target.canonicalize().unwrap();
    let mut rel = PathBuf::new();
    for comp in cwd.components() {
        if matches!(comp, std::path::Component::Normal(_)) {
            rel.push("..");
        }
    }
    for comp in abs.components() {
        if let std::path::Component::Normal(part) = comp {
            rel.push(part);
        }
    }
    assert!(rel.is_relative(), "expected relative path: {rel:?}");
    rel
}

/// #1227 W3: nothing is kept unless the journaled record will name it.
/// Binding a value equal to the conversation id, its first hex group, the word
/// `conversation`, or a component of the kept place's path would rewrite
/// `(kind, locator, home)` during shared masking; each keeps nothing, records
/// `none`, and emits a typed notice naming no value.
#[test]
fn masked_reference_keeps_nothing_and_records_none() {
    for (label, secret_value) in [
        ("full-uuid", UUID),
        ("first-hex-group", &UUID[..8]),
        ("kind-word-conversation", "conversation"),
        ("kind-word-agy-conversation", "agy-conversation"),
        ("kept-parent-component", "agy-conversations"),
        ("operator-home-component", "operator"),
        ("row-key-and-step-transcript", "transcript"),
        ("row-key-kind", "kind"),
        ("row-key-step", "step"),
    ] {
        let (world, bin, input, messages) = run_masked_case(secret_value);
        assert!(world.homes_gone(), "{label}: engine HOME removed");
        let kept = kept_file(&world.operator_home());
        assert!(!kept.exists(), "{label}: must keep nothing when masked");
        let expected_transcript = if secret_value == "kind" {
            json!({"[secret:MASK_TEST]": "none", "locator": "", "home": ""})
        } else {
            json!({"kind": "none", "locator": "", "home": ""})
        };
        let transcript_key = if secret_value == "transcript" {
            "[secret:MASK_TEST]"
        } else {
            "transcript"
        };
        let transcripts: Vec<Value> = messages
            .iter()
            .filter_map(|body| match body {
                crate::Body::Checkpoint { data, .. } => data.get(transcript_key).cloned(),
                _ => None,
            })
            .collect();
        assert!(
            !transcripts.is_empty() && transcripts.iter().all(|t| t == &expected_transcript),
            "{label}: expected masked fallback none transcripts, got {transcripts:?}"
        );

        let bindings = [bound("MASK_TEST", secret_value)];
        let inv = super::drive_flash(&bin, StartMode::default(), &input, &bindings)
            .unwrap_or_else(|e| panic!("{label}: expected clean turn, got {e}"));
        assert_eq!(
            inv.session_meta["transcript"],
            json!({"kind": "none", "locator": "", "home": ""}),
            "{label}"
        );
        assert_eq!(
            inv.stderr,
            format!("{}\n", KeepNotice::ReferenceMasked),
            "{label}"
        );
        if !KeepNotice::ReferenceMasked
            .to_string()
            .contains(secret_value)
        {
            assert!(
                !inv.stderr.contains(secret_value),
                "{label}: {}",
                inv.stderr
            );
        }
        assert!(!kept.exists(), "{label}: direct drive must keep nothing");
    }
}

/// #1227 Y3: any operator HOME rejected by the shared run-view home rule
/// (relative path, or an absolute path containing a newline) keeps nothing,
/// records `none`, and emits `KeepNotice::InvalidReference`.
#[test]
fn home_rejected_by_reader_rule_keeps_nothing_and_records_none() {
    let stdout = clean_turn_stdout();
    for (label, newline) in [("relative", false), ("newline", true)] {
        let mut world = World::new();
        let bin = fake_agy_with_transcript(
            &world.seat(),
            UUID,
            sample_transcript().as_bytes(),
            stdout.as_bytes(),
            true,
            0,
        );
        let bad_home = if newline {
            let dir = world.operator_home().join("bad\nline");
            let token_rel = ".gemini/antigravity-cli/antigravity-oauth-token";
            let token_dst = dir.join(token_rel);
            std::fs::create_dir_all(token_dst.parent().unwrap()).unwrap();
            std::fs::copy(world.operator_home().join(token_rel), &token_dst).unwrap();
            dir
        } else {
            relative_path_to(&world.operator_home())
        };
        let input = declared_input(&world);
        world.env.set("BROKKR_AGY_BIN", &bin);
        world.env.set("HOME", &bad_home);
        let messages = run_seat_with_input(input.clone(), false);
        assert!(world.homes_gone(), "{label}");
        let kept = kept_file(&bad_home);
        assert!(!kept.exists(), "{label}: rejected HOME must keep nothing");
        assert!(
            checkpoint_transcripts(&messages)
                .iter()
                .all(|t| t == &json!({"kind": "none", "locator": "", "home": ""})),
            "{label}"
        );
        let inv = super::drive_flash(&bin, StartMode::default(), &input, &[])
            .expect("rejected HOME turn still completes");
        assert_eq!(
            inv.stderr,
            format!("{}\n", KeepNotice::InvalidReference),
            "{label}"
        );
        assert!(!kept.exists(), "{label}");
    }
}

/// #1227 Y3 (native-home rule): a drive-prefixed or UNC HOME passes portable
/// lexical validation in `brokkr-view` (`is_valid_transcript_home`) but is
/// rejected as a host-relative path by transcript discovery on Linux/macOS
/// (`is_native_transcript_home`). The keeper applies the same native-home check
/// before touching disk, keeps nothing, records `none`, and emits
/// `KeepNotice::InvalidReference`.
#[test]
fn foreign_drive_and_unc_homes_keep_nothing_and_record_none() {
    let dir = tempfile::tempdir().unwrap();
    let engine_home = dir.path().join("engine");
    plant_transcript(&engine_home, UUID, sample_transcript().as_bytes());
    let conv = super::super::fold::Conversation::Named(UUID.to_string());

    for (label, foreign_home, top_entry) in [
        (
            "drive-backslash",
            r"C:\brokkr-y3-drive",
            r"C:\brokkr-y3-drive",
        ),
        ("drive-slash", "C:/brokkr-y3-drive", "C:"),
        (
            "unc",
            r"\\brokkr-y3-server\share",
            r"\\brokkr-y3-server\share",
        ),
    ] {
        let home_path = Path::new(foreign_home);
        assert_eq!(
            keep(UUID, &engine_home, Some(home_path), &[]),
            Err(KeepNotice::InvalidReference),
            "{label}: direct keep must reject non-native HOME"
        );
        let kept_file = home_path
            .join(".local/state/brokkr/agy-conversations")
            .join(UUID)
            .join("transcript.jsonl");
        assert!(
            !kept_file.exists() && !Path::new(top_entry).exists(),
            "{label}: rejected non-native HOME must create no host-relative path"
        );

        let mut meta = serde_json::Map::new();
        let mut emitted = Vec::new();
        let notice = super::super::keep::publish_transcript(
            super::super::keep::KeepContext {
                declared: true,
                probe: false,
                refused_to_start: false,
                conversation: &conv,
                engine_home: &engine_home,
                operator_home: Some(home_path),
                bindings: &[],
            },
            &mut meta,
            &mut |row| emitted.push(row.row.clone()),
        );
        assert_eq!(notice, Some(KeepNotice::InvalidReference), "{label}");
        let expected_none = json!({"kind": "none", "locator": "", "home": ""});
        assert_eq!(meta.get("transcript"), Some(&expected_none), "{label}");
        assert!(
            emitted
                .iter()
                .all(|row| row.get("transcript") == Some(&expected_none)),
            "{label}: emitted={emitted:?}"
        );
        assert!(
            !kept_file.exists() && !Path::new(top_entry).exists(),
            "{label}: publish_transcript must leave no host-relative file"
        );
    }
}

fn assert_refused_probe_matches_main(declared: bool) {
    let label = if declared { "declaring" } else { "adapterless" };
    let sub_init = format!(
        "{{\"event\":\"init\",\"conversation_id\":\"{UUID}\",\"init\":{{\"model\":\"gemini-4-neon\"}}}}"
    );
    let ok_res =
        r#"{"event":"result","result":{"status":"SUCCESS","response":"DONE","num_turns":1}}"#;
    let refused_stdout = format!("{sub_init}\n{ok_res}\n");

    let mut world = World::new();
    let bin = fake_agy_with_transcript(
        &world.seat(),
        UUID,
        sample_transcript().as_bytes(),
        refused_stdout.as_bytes(),
        false,
        0,
    );
    let input = if declared {
        declared_input(&world)
    } else {
        world.input(None)
    };
    let messages = run_seat_agy(&mut world, &bin, input, true);
    assert!(world.homes_gone(), "{label} refused probe");
    let kept = world
        .operator_home()
        .join(".local/state/brokkr/agy-conversations")
        .join(UUID)
        .join("transcript.jsonl");
    assert!(!kept.exists(), "{label} refused probe must keep nothing");
    assert_eq!(messages.len(), 1, "{label}: {messages:?}");
    assert!(
        matches!(
            &messages[0],
            crate::Body::Result {
                status: crate::ResultStatus::Failed,
                refusal: Some(_),
                result: None,
                error: Some(err),
                ..
            } if err == "probe ended without an answer"
        ),
        "{label}: {messages:?}"
    );
}

fn assert_answering_probe_matches_main(declared: bool) {
    let label = if declared { "declaring" } else { "adapterless" };
    let clean_stdout = clean_turn_stdout();
    let mut world = World::new();
    let bin = fake_agy_with_transcript(
        &world.seat(),
        UUID,
        sample_transcript().as_bytes(),
        clean_stdout.as_bytes(),
        true,
        0,
    );
    let input = if declared {
        declared_input(&world)
    } else {
        world.input(None)
    };
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_seat_agy(&mut world, &bin, input.clone(), true)
    }));
    let payload = panicked.expect_err(&format!(
        "{label}: answering probe must leave inv.answer untouched as on main"
    ));
    let msg = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("");
    assert!(
        msg.contains("driver checkpoints are JSON objects"),
        "{label}: {msg}"
    );
    assert!(world.homes_gone(), "{label} answering probe");
    let kept = world
        .operator_home()
        .join(".local/state/brokkr/agy-conversations")
        .join(UUID)
        .join("transcript.jsonl");
    assert!(!kept.exists(), "{label} answering probe must keep nothing");

    let inv = super::drive_flash(
        &bin,
        StartMode {
            offered: false,
            probe: true,
        },
        &input,
        &[],
    )
    .expect("direct probe drive succeeds");
    assert_eq!(inv.answer, Some(Value::String("DONE".into())), "{label}");
    assert_eq!(inv.stderr, "", "{label}: probe must emit no keep notice");
    assert_eq!(
        inv.session_meta["transcript"],
        json!({"kind": "none", "locator": "", "home": ""}),
        "{label}"
    );
    assert!(!kept.exists(), "{label}: direct probe must keep nothing");
}

fn assert_probe_mode_matches_main(declared: bool) {
    assert_refused_probe_matches_main(declared);
    assert_answering_probe_matches_main(declared);
}

/// #1227 W4: a probe is main's to the byte — nothing kept, no notice, answer
/// untouched — tested through `run_seat_with` with `probe == true` for both a
/// declaring probe and an adapterless probe.
#[test]
fn probe_turns_match_main_through_run_seat_with_for_declaring_and_adapterless() {
    assert_probe_mode_matches_main(true);
    assert_probe_mode_matches_main(false);
}

/// #1227 W6: an unreadable `resume_context.declared_transcript_kind` value is
/// a refusal to start naming the member, never a silent `none`, and spawns no
/// child process and makes no engine HOME.
#[test]
fn unreadable_declared_transcript_kind_refuses_to_start_naming_the_member() {
    let bad_contexts = [
        ("number", json!({"declared_transcript_kind": 42})),
        ("null", json!({"declared_transcript_kind": null})),
        ("bool", json!({"declared_transcript_kind": true})),
        ("empty", json!({"declared_transcript_kind": ""})),
        ("none", json!({"declared_transcript_kind": "none"})),
        (
            "over-long",
            json!({"declared_transcript_kind": "a".repeat(41)}),
        ),
        (
            "uppercase",
            json!({"declared_transcript_kind": "AGY-CONVERSATION"}),
        ),
        (
            "spaces",
            json!({"declared_transcript_kind": "agy conversation"}),
        ),
        (
            "array",
            json!({"declared_transcript_kind": ["agy-conversation"]}),
        ),
        ("non-object-context", json!(42)),
    ];
    for (label, ctx) in bad_contexts {
        let mut world = World::new();
        let marker = world.path("spawned");
        let bin = world.seat().join("should-not-run");
        std::fs::write(&bin, format!("#!/bin/sh\ntouch {}\n", marker.display())).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        world.set(&bin);

        let mut input = world.input(None);
        input["resume_context"] = ctx;
        let err = super::drive_flash(&bin, StartMode::default(), &input, &[])
            .map(drop)
            .expect_err(&format!("{label}: must refuse"));
        assert!(
            err.contains("resume_context.declared_transcript_kind"),
            "{label}: error must name the member, got: {err}"
        );
        assert!(!marker.exists(), "{label}: child must never spawn");
        assert!(world.homes_gone(), "{label}: no engine HOME made");
    }
}

/// #1227 W7: `agy::invoke` and `run_seat_with` share one `refusal_to_start`
/// predicate. When `agy` exits 0 with an empty (0-byte) result file and a
/// pre-turn refusal (unconfirmed model), both agree it is NOT a refusal to
/// start: the transcript is kept, `Accepted` and the checkpoint naming it are
/// sent, and the empty result file reaches decision 0001 as unparseable.
#[test]
fn empty_result_file_gets_one_answer_in_agy_invoke_and_run_seat_with() {
    let stdout = no_model_ok_stdout();
    let mut world = World::new();
    let (bin, result_path) = fake_agy_empty_result(&world.seat(), "fake-agy-empty-result", &stdout);
    let input = declared_input(&world);
    let messages = run_seat_agy(&mut world, &bin, input, false);
    assert!(world.homes_gone());
    assert_eq!(std::fs::metadata(&result_path).unwrap().len(), 0);
    let expected_home = kept_parent(&world.operator_home());
    let kept = kept_file(&world.operator_home());
    assert!(
        kept.exists(),
        "empty result file on clean exit keeps transcript"
    );
    let expected_record = expected_kept_record(&expected_home);
    assert!(
        checkpoint_transcripts(&messages)
            .iter()
            .any(|t| t == &expected_record),
        "checkpoint must name kept transcript: {messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, crate::Body::Accepted { .. })),
        "run_seat_with must accept: {messages:?}"
    );
    assert!(
        matches!(
            messages.last(),
            Some(crate::Body::Result {
                status: crate::ResultStatus::Succeeded,
                result: Some(val),
                ..
            }) if val.get("__unparseable_result_file__").is_some()
        ),
        "empty result file must reach decision 0001 unparseable path: {messages:?}"
    );
}

/// #1227 W8: an owner-only parent (`mode & 0o077 == 0`) that the owner cannot
/// write (mode 0500) fails keeping with a typed keep notice (`KeepNotice::Io`)
/// and leaves the turn unchanged.
#[test]
fn non_writable_owner_only_parent_reports_typed_notice_and_leaves_turn_unchanged() {
    let mut world = World::new();
    let parent = world
        .operator_home()
        .join(".local/state/brokkr/agy-conversations");
    std::fs::create_dir_all(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o500)).unwrap();

    let stdout = clean_turn_stdout();
    let bin = fake_agy_with_transcript(
        &world.seat(),
        UUID,
        sample_transcript().as_bytes(),
        stdout.as_bytes(),
        true,
        0,
    );
    world.set(&bin);
    let input = declared_input(&world);
    let inv = super::drive_flash(&bin, StartMode::default(), &input, &[])
        .expect("turn completes unchanged");

    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();

    assert_eq!(inv.answer, Some(Value::String("DONE".into())));
    assert_eq!(
        inv.session_meta["transcript"],
        json!({"kind": "none", "locator": "", "home": ""})
    );
    assert_eq!(
        inv.stderr,
        format!("{}\n", KeepNotice::Io(std::io::ErrorKind::PermissionDenied))
    );
    assert!(!parent.join(UUID).exists());
}

/// #1227 Y4: a bound value equal to the emitted transcript row's default
/// `model` sentinel (`not reported`, inserted by `probe::seat_row` after
/// `mask_json`) or to the turn's pinned model (`FLASH`) does not rewrite any
/// byte of the emitted transcript row, so keeping succeeds and the emitted
/// transcript checkpoint carries the exact `agy-conversation` reference.
#[test]
fn bound_value_matching_row_model_leaves_transcript_row_unchanged_and_keeps() {
    for (label, secret_value) in [
        ("row-default-model", crate::adapters::MODEL_NOT_REPORTED),
        ("pinned-model", FLASH),
    ] {
        let (world, _bin, _input, messages) = run_masked_case(secret_value);
        assert!(world.homes_gone(), "{label}: engine HOME removed");
        let expected_home = kept_parent(&world.operator_home());
        let kept = kept_file(&world.operator_home());
        assert!(
            kept.exists(),
            "{label}: row unaffected by seat_row keeps file"
        );
        let expected_record = expected_kept_record(&expected_home);
        assert!(
            checkpoint_transcripts(&messages)
                .iter()
                .any(|t| t == &expected_record),
            "{label}: checkpoint must name kept transcript, got {messages:?}"
        );
    }
}

fn run_single_observation_case(
    write_empty_before_exit: bool,
    mutate_after_drive: impl FnOnce(&Path),
) -> (World, Vec<crate::Body>, PathBuf, PathBuf) {
    let stdout = no_model_ok_stdout();
    let mut world = World::new();
    let dir = world.seat();
    let (bin, result_path) = if write_empty_before_exit {
        fake_agy_empty_result(&dir, "fake-agy-y5", &stdout)
    } else {
        let bin = fake_agy_with_transcript(
            &dir,
            UUID,
            sample_transcript().as_bytes(),
            stdout.as_bytes(),
            false,
            0,
        );
        (bin, dir.join("result.json"))
    };
    world.set(&bin);

    let input = declared_input(&world);
    let start = json!({"attempt_id": "attempt", "effect_id": "effect", "input": input});
    let (binary, extra) = (
        bin.to_string_lossy().into_owned(),
        argv(&["--model", FLASH]),
    );
    let mut messages = Vec::new();
    crate::adapters::run_seat_with(
        AdapterKind::Agy,
        Ok(()),
        &start,
        &mut |body| messages.push(body),
        |prompt, input, bindings, emit| {
            let pinned = super::super::super::Pinned {
                binary: &binary,
                extra: &extra,
            };
            let inv = drive(
                pinned,
                StartMode::default(),
                prompt,
                input,
                bindings,
                &mut |row| emit(row),
            );
            mutate_after_drive(&result_path);
            inv
        },
    );
    let expected_home = world
        .operator_home()
        .join(".local/state/brokkr/agy-conversations");
    let kept = expected_home.join(UUID).join("transcript.jsonl");
    assert!(world.homes_gone());
    (world, messages, expected_home, kept)
}

/// #1227 Y5: `result_path` is observed once in `agy::concluded` and carried on
/// `Invocation.result_exists` to `run_seat_with`, so mutating `result_path`
/// after `agy::invoke` never makes the two sides disagree.
#[test]
fn result_file_is_observed_once_across_agy_invoke_and_run_seat_with() {
    // Present at exit, then deleted before `run_seat_with`: both see `true`.
    {
        let (_world, messages, expected_home, kept) =
            run_single_observation_case(true, |path| std::fs::remove_file(path).unwrap());
        assert!(kept.exists(), "present at exit keeps transcript");
        assert!(
            messages
                .iter()
                .any(|m| matches!(m, crate::Body::Accepted { .. })),
            "single observation keeps Accepted even after post-invoke unlink: {messages:?}"
        );
        let expected_record = expected_kept_record(&expected_home);
        assert!(
            checkpoint_transcripts(&messages)
                .iter()
                .any(|t| t == &expected_record),
            "checkpoint must still be flushed: {messages:?}"
        );
    }

    // Absent at exit, then created before `run_seat_with`: both see `false`.
    let (_world, messages, _, kept) = run_single_observation_case(false, |path| {
        std::fs::write(path, r#"{"result":"complete"}"#).unwrap()
    });
    assert!(!kept.exists(), "absent at exit keeps nothing");
    assert!(
        !messages
            .iter()
            .any(|m| matches!(m, crate::Body::Accepted { .. })),
        "single observation refuses to start even after post-invoke create: {messages:?}"
    );
    assert!(
        matches!(
            messages.last(),
            Some(crate::Body::Result {
                status: crate::ResultStatus::Failed,
                refusal: Some(_),
                ..
            })
        ),
        "expected refusal-to-start Failed result: {messages:?}"
    );
}

/// #1227 H5: `DeclaredTranscriptKind` null `resume_context`,
/// `fold::read_conversation_id` `expecting`, `Transcript::resolve(AgyConversation)`,
/// and source errno classifiers (direct and real-FS) bind.
#[test]
fn keep_source_errno_declared_null_and_fold_expecting_bind() {
    use rustix::io::Errno;
    use std::io::ErrorKind;

    assert_eq!(
        crate::DeclaredTranscriptKind::read_from(&json!({"resume_context": null})),
        Ok(None)
    );
    assert_eq!(
        super::super::fold::read_conversation_id("123"),
        super::super::fold::Conversation::Invalid
    );
    assert_eq!(
        crate::transcript::Transcript::resolve(crate::transcript::Kind::AgyConversation),
        Err(crate::transcript::TranscriptRefusal::NoHome {
            kind: crate::transcript::Kind::AgyConversation
        })
    );
    assert_eq!(
        super::super::keep::classify_dir_errno(Errno::PERM),
        KeepNotice::Io(ErrorKind::PermissionDenied)
    );
    assert_eq!(
        super::super::keep::classify_file_errno(Errno::NOENT),
        KeepNotice::Absent
    );
    assert_eq!(
        super::super::keep::classify_file_errno(Errno::PERM),
        KeepNotice::Io(ErrorKind::PermissionDenied)
    );

    let dir = tempfile::tempdir().unwrap();
    let op = dir.path().join("op");
    std::fs::create_dir_all(&op).unwrap();

    let engine_no_leaf = dir.path().join("engine-no-leaf");
    std::fs::create_dir_all(
        engine_no_leaf
            .join(".gemini/antigravity-cli/brain")
            .join(UUID)
            .join(".system_generated/logs"),
    )
    .unwrap();
    assert_eq!(
        keep(UUID, &engine_no_leaf, Some(&op), &[]),
        Err(KeepNotice::Absent)
    );

    let engine_no_read = dir.path().join("engine-no-read");
    let unreadable = plant_transcript(&engine_no_read, UUID, b"data");
    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();
    let file_err = keep(UUID, &engine_no_read, Some(&op), &[]);
    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(file_err, Err(KeepNotice::Io(ErrorKind::PermissionDenied)));

    let engine_bad_dir = dir.path().join("engine-bad-dir");
    plant_transcript(&engine_bad_dir, UUID, b"data");
    let dot_gemini = engine_bad_dir.join(".gemini");
    std::fs::set_permissions(&dot_gemini, std::fs::Permissions::from_mode(0o000)).unwrap();
    let dir_err = keep(UUID, &engine_bad_dir, Some(&op), &[]);
    std::fs::set_permissions(&dot_gemini, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(dir_err, Err(KeepNotice::Io(ErrorKind::PermissionDenied)));
}

/// #1227 H5: `owned_parent` branches, `is_owned_private_dir` predicate arms,
/// `ConvGuard::drop` cleanup, and `copy_bounded` cap and write-error paths bind.
#[test]
fn owned_parent_guard_drop_and_copy_bounded_error_paths_bind() {
    use std::io::ErrorKind;
    use std::os::fd::OwnedFd;

    let dir = tempfile::tempdir().unwrap();
    let op = dir.path().join("op");
    std::fs::create_dir_all(&op).unwrap();

    assert_eq!(
        super::super::keep::owned_parent(Path::new("/")).unwrap_err(),
        KeepNotice::UnownedParent
    );

    let file_ancestor = dir.path().join("file-ancestor");
    std::fs::write(&file_ancestor, b"x").unwrap();
    assert!(matches!(
        super::super::keep::owned_parent(&file_ancestor.join("sub/parent")),
        Err(KeepNotice::ParentIo(_))
    ));

    let ro_ancestor = dir.path().join("ro-ancestor");
    std::fs::create_dir_all(&ro_ancestor).unwrap();
    std::fs::set_permissions(&ro_ancestor, std::fs::Permissions::from_mode(0o500)).unwrap();
    let create_err = super::super::keep::owned_parent(&ro_ancestor.join("child"));
    std::fs::set_permissions(&ro_ancestor, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        create_err.unwrap_err(),
        KeepNotice::ParentIo(ErrorKind::PermissionDenied)
    );

    let link_parent = dir.path().join("link-parent");
    std::os::unix::fs::symlink(&op, &link_parent).unwrap();
    assert_eq!(
        super::super::keep::owned_parent(&link_parent).unwrap_err(),
        KeepNotice::UnownedParent
    );

    let file_parent = dir.path().join("file-parent");
    std::fs::write(&file_parent, b"x").unwrap();
    assert_eq!(
        super::super::keep::owned_parent(&file_parent).unwrap_err(),
        KeepNotice::UnownedParent
    );

    let locked_parent = dir.path().join("locked-parent");
    std::fs::create_dir_all(&locked_parent).unwrap();
    std::fs::set_permissions(&locked_parent, std::fs::Permissions::from_mode(0o000)).unwrap();
    let open_err = super::super::keep::owned_parent(&locked_parent);
    std::fs::set_permissions(&locked_parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        open_err.unwrap_err(),
        KeepNotice::ParentIo(ErrorKind::PermissionDenied)
    );

    let uid = rustix::process::geteuid().as_raw();
    let dir_700: rustix::fs::RawMode = 0o040700;
    let file_600: rustix::fs::RawMode = 0o100600;
    let dir_755: rustix::fs::RawMode = 0o040755;
    assert!(super::super::keep::is_owned_private_dir(dir_700, uid, uid));
    assert!(!super::super::keep::is_owned_private_dir(
        file_600, uid, uid
    ));
    assert!(!super::super::keep::is_owned_private_dir(
        dir_700,
        uid.wrapping_add(1),
        uid
    ));
    assert!(!super::super::keep::is_owned_private_dir(dir_755, uid, uid));

    let guard_parent = dir.path().join("guard-parent");
    let parent_fd = super::super::keep::owned_parent(&guard_parent).unwrap();
    let created_dir = guard_parent.join(UUID);
    {
        let _guard = ConvGuard::create(&parent_fd, UUID).unwrap();
        assert!(created_dir.is_dir());
    }
    assert!(
        !created_dir.exists(),
        "dropping undefused ConvGuard removes partial directory"
    );

    let src_path = dir.path().join("copy-src");
    let dst_path = dir.path().join("copy-dst");
    std::fs::write(&src_path, b"0123456789").unwrap();
    let src_fd: OwnedFd = std::fs::File::open(&src_path).unwrap().into();
    let dst_fd: OwnedFd = std::fs::File::create(&dst_path).unwrap().into();
    assert_eq!(
        super::super::keep::copy_bounded(src_fd, dst_fd, 4),
        Err(KeepNotice::TooLarge(4))
    );

    let src_fd2: OwnedFd = std::fs::File::open(&src_path).unwrap().into();
    let ro_dst_fd: OwnedFd = std::fs::File::open(&dst_path).unwrap().into();
    assert!(matches!(
        super::super::keep::copy_bounded(src_fd2, ro_dst_fd, 64),
        Err(KeepNotice::Io(_))
    ));
}

fn owned_conv_parent() -> (tempfile::TempDir, PathBuf, std::os::fd::OwnedFd) {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("agy-conversations");
    let parent_fd = super::super::keep::owned_parent(&parent).unwrap();
    (dir, parent, parent_fd)
}

/// #1227: rollback stays bound to the acquired parent and conversation handles
/// when the parent directory is renamed before a copy failure, cleaning up the
/// partial file under the moved parent without touching a replacement UUID
/// directory created at the original pathname.
#[test]
fn conv_guard_rollback_follows_renamed_parent_and_spares_replacement() {
    use std::os::fd::OwnedFd;

    let (dir, original_parent, parent_fd) = owned_conv_parent();
    let moved_parent = dir.path().join("agy-conversations-moved");

    let src_path = dir.path().join("oversized.jsonl");
    let cap: u64 = 65_536;
    std::fs::write(&src_path, vec![b'a'; (cap as usize) + 16]).unwrap();
    let src_fd: OwnedFd = std::fs::File::open(&src_path).unwrap().into();

    let replacement_file = original_parent
        .join(UUID)
        .join(brokkr_core::seat::AGY_TRANSCRIPT_FILE);
    let moved_conv = moved_parent.join(UUID);
    let moved_file = moved_conv.join(brokkr_core::seat::AGY_TRANSCRIPT_FILE);

    {
        let mut guard = ConvGuard::create(&parent_fd, UUID).unwrap();
        let dest_fd = guard.open_dest().unwrap();

        std::fs::rename(&original_parent, &moved_parent).unwrap();
        std::fs::create_dir_all(original_parent.join(UUID)).unwrap();
        std::fs::write(&replacement_file, b"replacement-keep").unwrap();

        assert_eq!(
            super::super::keep::copy_bounded(src_fd, dest_fd, cap),
            Err(KeepNotice::TooLarge(cap))
        );
        assert!(
            moved_file.is_file(),
            "partial transcript exists under the renamed parent before rollback"
        );
    }

    assert!(
        !moved_conv.exists(),
        "rollback removes the partial transcript and UUID directory under the renamed parent"
    );
    assert_eq!(
        std::fs::read(&replacement_file).unwrap(),
        b"replacement-keep",
        "rollback never touches a replacement UUID directory at the original pathname"
    );
}

/// #1227: if opening the freshly created UUID directory fails (for instance if
/// the directory becomes mode 0300 before open), `ConvGuard::create_with`
/// removes the empty directory through `unlink_if_same` so a subsequent keep
/// is not blocked with `AlreadyKept`.
#[test]
fn conv_guard_rolls_back_unopened_directory_when_open_fails_after_mkdir() {
    use std::io::ErrorKind;

    let (_dir, parent, parent_fd) = owned_conv_parent();
    let conv_dir = parent.join(UUID);

    let err = ConvGuard::create_with(
        &parent_fd,
        UUID,
        |_, name| {
            std::fs::set_permissions(parent.join(name), std::fs::Permissions::from_mode(0o300))
                .unwrap();
        },
        noop_conv,
    );
    assert_eq!(err.err(), Some(KeepNotice::Io(ErrorKind::PermissionDenied)));
    assert!(
        !conv_dir.exists(),
        "failed openat after mkdirat must roll back the empty UUID directory"
    );

    let retry = ConvGuard::create(&parent_fd, UUID);
    assert!(
        retry.is_ok(),
        "retry succeeds instead of failing with AlreadyKept once unopened directory is rolled back"
    );
    drop(retry);
    assert!(!conv_dir.exists());
}

/// #1227: when exclusive transcript creation fails because a transcript was
/// concurrently restored, dropping `ConvGuard` does not unlink the unowned file.
#[test]
fn conv_guard_rollback_spares_concurrently_restored_transcript_when_excl_fails() {
    use std::io::ErrorKind;

    let (_dir, parent, parent_fd) = owned_conv_parent();
    let restored = parent
        .join(UUID)
        .join(brokkr_core::seat::AGY_TRANSCRIPT_FILE);

    {
        let mut guard = ConvGuard::create(&parent_fd, UUID).unwrap();
        std::fs::write(&restored, b"restored-transcript").unwrap();
        assert_eq!(
            guard.open_dest().unwrap_err(),
            KeepNotice::Io(ErrorKind::AlreadyExists)
        );
    }

    assert_eq!(
        std::fs::read(&restored).unwrap(),
        b"restored-transcript",
        "rollback must not delete a transcript file that open_dest did not create"
    );
}

/// #1227: if the UUID directory is renamed away and replaced by a new empty
/// directory under the same parent, rollback cleans up our partial transcript
/// inside the moved handle and spares the empty replacement directory.
#[test]
fn conv_guard_rollback_spares_replaced_uuid_directory_under_same_parent() {
    use std::io::Write;

    let (_dir, parent, parent_fd) = owned_conv_parent();
    let uuid_dir = parent.join(UUID);
    let moved_dir = parent.join("moved-uuid");
    let moved_file = moved_dir.join(brokkr_core::seat::AGY_TRANSCRIPT_FILE);

    {
        let mut guard = ConvGuard::create(&parent_fd, UUID).unwrap();
        let mut dest = std::fs::File::from(guard.open_dest().unwrap());
        dest.write_all(b"partial-transcript").unwrap();
        drop(dest);

        std::fs::rename(&uuid_dir, &moved_dir).unwrap();
        std::fs::create_dir(&uuid_dir).unwrap();
        assert!(moved_file.is_file());
    }

    assert!(
        !moved_file.exists(),
        "rollback removes the partial transcript from the original directory handle"
    );
    assert!(
        uuid_dir.is_dir(),
        "rollback spares an empty replacement directory now occupying the UUID name"
    );
}

fn noop_conv(_: &std::os::fd::OwnedFd, _: &str) {}

fn outside_target_dir(mode: u32) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("outside-target");
    std::fs::create_dir(&target).unwrap();
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(mode)).unwrap();
    (dir, target)
}

fn swap_for_dir(target: &Path, moved: &Path, mode: u32) {
    std::fs::rename(target, moved).unwrap();
    std::fs::create_dir(target).unwrap();
    std::fs::set_permissions(target, std::fs::Permissions::from_mode(mode)).unwrap();
}

fn swap_for_outside_link(target: &Path, moved: &Path, outside: &Path) {
    std::fs::rename(target, moved).unwrap();
    std::fs::set_permissions(moved, std::fs::Permissions::from_mode(0o755)).unwrap();
    symlink(outside, target).unwrap();
}

/// #1227 S3: if the freshly created UUID directory is replaced before an open
/// failure, `ConvGuard::create_with` checks the identity recorded right after
/// `mkdirat` and spares the replacement directory instead of unlinking
/// unconditionally by name.
#[test]
fn conv_guard_failed_open_spares_replaced_uuid_directory() {
    use std::io::ErrorKind;

    let (_dir, parent, parent_fd) = owned_conv_parent();
    let uuid_dir = parent.join(UUID);
    let moved_dir = parent.join("moved-before-open");

    let err = ConvGuard::create_with(
        &parent_fd,
        UUID,
        |_, name| swap_for_dir(&parent.join(name), &moved_dir, 0o300),
        noop_conv,
    );
    std::fs::set_permissions(&uuid_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(err.err(), Some(KeepNotice::Io(ErrorKind::PermissionDenied)));
    assert!(
        uuid_dir.is_dir(),
        "failed-open cleanup must check identity and spare a replacement directory"
    );
    assert!(
        moved_dir.is_dir(),
        "the originally created directory moved aside stays untouched"
    );
}

/// #1227 Q2: if the freshly created UUID directory is swapped for another
/// directory between `mkdirat` and the open, `ConvGuard::create_with` refuses
/// with `ReplacedDirectory` and leaves the replacement untouched (no `fchmod`,
/// no file created inside, no unlink).
#[test]
fn conv_guard_refuses_replaced_directory_without_touching_it() {
    let (_dir, parent, parent_fd) = owned_conv_parent();
    let uuid_dir = parent.join(UUID);
    let moved_dir = parent.join("moved-conv");

    let err = ConvGuard::create_with(
        &parent_fd,
        UUID,
        |_, name| swap_for_dir(&parent.join(name), &moved_dir, 0o755),
        noop_conv,
    );
    let notice = err.err().expect("swapped directory must be refused");
    assert_eq!(notice, KeepNotice::ReplacedDirectory);
    assert_eq!(
        notice.to_string(),
        "agy transcript not kept: a newly created kept directory was replaced before it could be opened"
    );
    assert!(
        uuid_dir.is_dir(),
        "replacement directory must not be unlinked"
    );
    assert_eq!(
        dir_mode(&uuid_dir),
        0o755,
        "replacement directory mode must not be changed by fchmod"
    );
    assert!(
        std::fs::read_dir(&uuid_dir).unwrap().next().is_none(),
        "no file may be created inside a replacement directory"
    );
    assert!(moved_dir.is_dir());
}

/// #1227 Q1, Q2: if the freshly created UUID directory is swapped for a
/// symbolic link to a directory outside the kept tree between `mkdirat` and the
/// open, `ConvGuard::create_with` refuses the link without changing the outside
/// directory's mode and without unlinking the symlink.
#[test]
fn conv_guard_refuses_symlink_swap_without_changing_target_mode() {
    let (_dir, parent, parent_fd) = owned_conv_parent();
    let (_outside, outside_dir) = outside_target_dir(0o755);

    let uuid_dir = parent.join(UUID);
    let moved_dir = parent.join("moved-before-link");
    let err = ConvGuard::create_with(
        &parent_fd,
        UUID,
        |_, name| {
            let created = parent.join(name);
            std::fs::rename(&created, &moved_dir).unwrap();
            symlink(&outside_dir, &created).unwrap();
        },
        noop_conv,
    );
    assert_eq!(err.err(), Some(KeepNotice::ReplacedDirectory));
    assert_eq!(
        dir_mode(&outside_dir),
        0o755,
        "outside symlink target mode must never be changed by path-based chmod"
    );
    assert!(
        std::fs::symlink_metadata(&uuid_dir)
            .unwrap()
            .file_type()
            .is_symlink(),
        "symlink replacement must be spared by unlink_if_same"
    );
    assert!(moved_dir.is_dir());

    let same_inode_moved = parent.join("moved-same-inode");
    let err_same = ConvGuard::create_with(
        &parent_fd,
        SECOND_UUID,
        |_, name| {
            let created = parent.join(name);
            std::fs::rename(&created, &same_inode_moved).unwrap();
            std::fs::set_permissions(&same_inode_moved, std::fs::Permissions::from_mode(0o755))
                .unwrap();
            symlink(&same_inode_moved, &created).unwrap();
        },
        noop_conv,
    );
    assert_eq!(err_same.err(), Some(KeepNotice::ReplacedDirectory));
    assert_eq!(
        dir_mode(&same_inode_moved),
        0o755,
        "O_NOFOLLOW must reject a symlink even when it points to the moved same-inode directory"
    );
}

/// #1227 Q2: when `owned_parent` creates the kept-conversations parent itself,
/// swapping that entry for another directory between `mkdirat` and the open is
/// refused with `ReplacedDirectory` without changing the replacement's mode or
/// unlinking it.
#[test]
fn owned_parent_refuses_replaced_created_parent_without_touching_it() {
    let dir = tempfile::tempdir().unwrap();
    let brokkr_dir = dir.path().join(".local/state/brokkr");
    std::fs::create_dir_all(&brokkr_dir).unwrap();
    let parent = brokkr_dir.join("agy-conversations");
    let moved = brokkr_dir.join("moved-parent");

    let err = owned_parent_with(&parent, || swap_for_dir(&parent, &moved, 0o755), || {});
    assert_eq!(err.err(), Some(KeepNotice::ReplacedDirectory));
    assert!(parent.is_dir(), "replacement parent must not be unlinked");
    assert_eq!(
        dir_mode(&parent),
        0o755,
        "replacement parent mode must not be changed by fchmod"
    );
    assert!(moved.is_dir());
}

/// #1227 L2 (m2): swapping a conversation dir or created `agy-conversations`
/// for an outside symlink after verify leaves the outside mode unchanged.
#[test]
fn fchmod_acts_on_verified_handle_when_name_is_swapped_for_symlink_after_verify() {
    use std::io::Write;
    let (_outside, outside_dir) = outside_target_dir(0o755);
    let file_name = brokkr_core::seat::AGY_TRANSCRIPT_FILE;

    let (_dir, parent, parent_fd) = owned_conv_parent();
    let moved_conv = parent.join("moved-conv-after-verify");
    let mut guard = ConvGuard::create_with(&parent_fd, UUID, noop_conv, |_, name| {
        swap_for_outside_link(&parent.join(name), &moved_conv, &outside_dir);
    })
    .expect("conv keep proceeds through verified handle");
    assert_eq!(dir_mode(&outside_dir), 0o755);
    assert_eq!(dir_mode(&moved_conv), 0o700);
    std::fs::File::from(guard.open_dest().unwrap())
        .write_all(b"kept-conv\n")
        .unwrap();
    assert!(!outside_dir.join(file_name).exists() && moved_conv.join(file_name).is_file());
    drop(guard);
    assert_eq!(dir_mode(&outside_dir), 0o755);

    let root = tempfile::tempdir().unwrap();
    let kept_root = root.path().join(".local/state/brokkr/agy-conversations");
    let moved_parent = kept_root.with_file_name("moved-parent-after-verify");
    let created_parent_fd = owned_parent_with(
        &kept_root,
        || {},
        || {
            swap_for_outside_link(&kept_root, &moved_parent, &outside_dir);
        },
    )
    .expect("parent keep proceeds through verified handle");
    assert_eq!(dir_mode(&outside_dir), 0o755);
    assert_eq!(dir_mode(&moved_parent), 0o700);
    let mut nested = ConvGuard::create(&created_parent_fd, UUID).unwrap();
    std::fs::File::from(nested.open_dest().unwrap())
        .write_all(b"kept-parent\n")
        .unwrap();
    assert!(!outside_dir.join(UUID).exists() && moved_parent.join(UUID).join(file_name).is_file());
}

const OWN_UMASK: &str = "BROKKR_AGY_KEEP_OWN_UMASK";
const UMASK_TEST: &str =
    "adapters::agy::tests::keep::first_use_under_restrictive_umask_removes_created_dirs_and_keeps_nothing";
const SECOND_UUID: &str = "22222222-3333-4444-5555-666666666666";

struct UmaskGuard(rustix::fs::Mode);

impl UmaskGuard {
    fn set(mask: rustix::fs::RawMode) -> Self {
        Self(rustix::process::umask(rustix::fs::Mode::from_raw_mode(
            mask,
        )))
    }
}

impl Drop for UmaskGuard {
    fn drop(&mut self) {
        rustix::process::umask(self.0);
    }
}

fn dir_mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o7777
}

fn assert_kept_modes_and_bytes(op_home: &Path, id: &str, expected: &[u8]) {
    let parent = kept_parent(op_home);
    let conv = parent.join(id);
    let file = conv.join(brokkr_core::seat::AGY_TRANSCRIPT_FILE);
    assert_eq!(dir_mode(&parent), 0o700);
    assert_eq!(dir_mode(&conv), 0o700);
    assert_eq!(dir_mode(&file), 0o600);
    assert_eq!(std::fs::read(&file).unwrap(), expected);
}

fn setup_umask_fixture(
    ancestor_mode: u32,
) -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf, String, String) {
    let first_bytes = sample_transcript();
    let second_bytes = format!("{first_bytes}\n");
    let root = tempfile::tempdir().unwrap();
    let (engine_home, op_home) = (root.path().join("src-home"), root.path().join("op-home"));
    plant_transcript(&engine_home, UUID, first_bytes.as_bytes());
    plant_transcript(&engine_home, SECOND_UUID, second_bytes.as_bytes());
    let brokkr_dir = op_home.join(".local/state/brokkr");
    std::fs::create_dir_all(&brokkr_dir).unwrap();
    std::fs::set_permissions(&brokkr_dir, std::fs::Permissions::from_mode(ancestor_mode)).unwrap();
    (
        root,
        engine_home,
        op_home,
        brokkr_dir,
        first_bytes,
        second_bytes,
    )
}

fn verify_keep_under_restrictive_umask(mask: rustix::fs::RawMode) {
    use std::io::ErrorKind;

    let (_root, engine_home, op_home, brokkr_dir, first_bytes, second_bytes) =
        setup_umask_fixture(0o755);

    let parent_err = {
        let _umask = UmaskGuard::set(mask);
        super::super::keep::keep_with_cap(UUID, &engine_home, Some(&op_home), 65_536)
    }
    .unwrap_err();
    assert_eq!(
        parent_err,
        KeepNotice::ParentIo(ErrorKind::PermissionDenied),
        "missing agy-conversations under umask {mask:04o}"
    );
    assert!(
        !kept_parent(&op_home).exists(),
        "unopenable agy-conversations must be removed under umask {mask:04o}"
    );
    assert!(!kept_file(&op_home).exists());
    assert_eq!(
        dir_mode(&brokkr_dir),
        0o755,
        "existing ancestor mode must remain unchanged (W8)"
    );

    let kept = super::super::keep::keep_with_cap(UUID, &engine_home, Some(&op_home), 65_536)
        .unwrap_or_else(|notice| panic!("keep after umask {mask:04o} restored failed: {notice}"));
    assert_eq!(kept.locator(), UUID);
    assert_kept_modes_and_bytes(&op_home, UUID, first_bytes.as_bytes());

    let conv_err = {
        let _umask = UmaskGuard::set(mask);
        super::super::keep::keep_with_cap(SECOND_UUID, &engine_home, Some(&op_home), 65_536)
    }
    .unwrap_err();
    assert_eq!(
        conv_err,
        KeepNotice::Io(ErrorKind::PermissionDenied),
        "missing conv dir under umask {mask:04o}"
    );
    let second_conv = kept_parent(&op_home).join(SECOND_UUID);
    assert!(
        !second_conv.exists(),
        "unopenable conversation directory must be removed under umask {mask:04o}"
    );

    let second_kept =
        super::super::keep::keep_with_cap(SECOND_UUID, &engine_home, Some(&op_home), 65_536)
            .unwrap_or_else(|notice| {
                panic!("follow-up keep after umask {mask:04o} failed: {notice}")
            });
    assert_eq!(second_kept.locator(), SECOND_UUID);
    assert_kept_modes_and_bytes(&op_home, SECOND_UUID, second_bytes.as_bytes());
}

fn verify_keep_under_openable_umask() {
    let (_root, engine_home, op_home, brokkr_dir, first_bytes, second_bytes) =
        setup_umask_fixture(0o755);

    let _umask = UmaskGuard::set(0o100);
    let first = super::super::keep::keep_with_cap(UUID, &engine_home, Some(&op_home), 65_536)
        .unwrap_or_else(|notice| panic!("keep under umask 0100 failed: {notice}"));
    assert_eq!(first.locator(), UUID);
    let second =
        super::super::keep::keep_with_cap(SECOND_UUID, &engine_home, Some(&op_home), 65_536)
            .unwrap_or_else(|notice| panic!("second keep under umask 0100 failed: {notice}"));
    assert_eq!(second.locator(), SECOND_UUID);
    drop(_umask);

    assert_eq!(dir_mode(&brokkr_dir), 0o755);
    assert_kept_modes_and_bytes(&op_home, UUID, first_bytes.as_bytes());
    assert_kept_modes_and_bytes(&op_home, SECOND_UUID, second_bytes.as_bytes());
}

/// #1227 K2: under umask 0400 and umask 0777, newly created `agy-conversations`
/// and `<uuid>` directories cannot be opened, so keeping keeps nothing, returns
/// a typed notice, removes the created directory while leaving existing
/// directories untouched, and allows a later keep once the umask is restored.
/// Under umask 0100, newly created directories are mode 0600 (openable by their
/// owner), so the keeper opens them, sets 0700 through the verified handles,
/// and keeps the transcript. Runs in its own child process so changing the
/// process-wide umask never affects parallel tests, and restores the umask on
/// every path via `UmaskGuard`.
#[test]
fn first_use_under_restrictive_umask_removes_created_dirs_and_keeps_nothing() {
    if std::env::var_os(OWN_UMASK).is_some() {
        verify_keep_under_restrictive_umask(0o400);
        verify_keep_under_restrictive_umask(0o777);
        verify_keep_under_openable_umask();
        return;
    }
    let _env = crate::env_guard::EnvGuard::lock();
    let rerun = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", UMASK_TEST, "--test-threads=1"])
        .env(OWN_UMASK, "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&rerun.stdout);
    let stderr = String::from_utf8_lossy(&rerun.stderr);
    assert!(rerun.status.success(), "stdout: {stdout}\nstderr: {stderr}");
    assert!(stdout.contains("1 passed"), "stdout: {stdout}");
}

struct RestoreMode(PathBuf);

impl Drop for RestoreMode {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o700));
    }
}

/// #1227 K3: an existing ancestor above `agy-conversations` that is writable
/// and searchable but not readable (mode 0300) is traversed without a read-only
/// open, so creating `agy-conversations`, creating missing sub-ancestors, and
/// keeping subsequent turns under it all succeed while leaving its mode 0300.
#[test]
fn keep_succeeds_under_existing_ancestor_of_mode_0300() {
    let (_root, engine_home, op_home, brokkr_dir, first_bytes, second_bytes) =
        setup_umask_fixture(0o300);
    let _restore_brokkr = RestoreMode(brokkr_dir.clone());

    let first = super::super::keep::keep_with_cap(UUID, &engine_home, Some(&op_home), 65_536)
        .expect("keep creating agy-conversations under mode-0300 ancestor must succeed");
    assert_eq!(first.locator(), UUID);
    let second =
        super::super::keep::keep_with_cap(SECOND_UUID, &engine_home, Some(&op_home), 65_536)
            .expect("second keep under mode-0300 ancestor must succeed");
    assert_eq!(second.locator(), SECOND_UUID);
    assert_eq!(dir_mode(&brokkr_dir), 0o300);
    assert_kept_modes_and_bytes(&op_home, UUID, first_bytes.as_bytes());
    assert_kept_modes_and_bytes(&op_home, SECOND_UUID, second_bytes.as_bytes());

    let nested_root = tempfile::tempdir().unwrap();
    let nested_home = nested_root.path().join("nested-home");
    let local_dir = nested_home.join(".local");
    std::fs::create_dir_all(&local_dir).unwrap();
    std::fs::set_permissions(&local_dir, std::fs::Permissions::from_mode(0o300)).unwrap();
    let _restore_local = RestoreMode(local_dir.clone());

    let nested = super::super::keep::keep_with_cap(UUID, &engine_home, Some(&nested_home), 65_536)
        .expect("keep creating missing sub-ancestors under mode-0300 .local must succeed");
    assert_eq!(nested.locator(), UUID);
    assert_eq!(dir_mode(&local_dir), 0o300);
    assert_kept_modes_and_bytes(&nested_home, UUID, first_bytes.as_bytes());
}
