use super::*;
use forge_core::canonical::{sha256_hex, ZERO_HASH};
use forge_core::dispatch::{build_run_manifest_v2, DispatchEnvelopeV2, PRODUCER_EFFECTS};
use forge_core::fold::Cursor;
use forge_core::EventType;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use time::format_description::well_known::Rfc3339;

/// `HOME` is process-global and the transcript lookup reads it, so the
/// tests that point it at a temp projects tree take turns. One lock for
/// the whole binary, named where both surfaces' test modules can see it.
pub(crate) static HOME: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn running_store(db: &std::path::Path, run_id: &str) {
    let mut store = Store::open(db).unwrap();
    store
        .create_run(run_id, "feature", "test", &json!({"files": {}}))
        .unwrap();
    store
        .append_next(
            run_id,
            EventType::RunStarted,
            json!({"feature":"feature", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            run_id,
            EventType::PhaseEntered,
            json!({"phase":"work"}),
            None,
            None,
        )
        .unwrap();
}

/// A journal that genuinely does not fold: an `operator/accepted` that
/// names a command this run never carried. The fold has no rule that
/// can read it at any cursor — the acceptance is unattached — so the
/// run is quarantined. The grace exists for exactly this: a journal the
/// fold cannot read must not blind a fleet read to every other run.
///
/// (The stop-accepted-mid-flight shape this helper used to build is no
/// longer unfoldable — see `an_operator_stop_mid_flight_lists_with_its_real_status`.)
pub(crate) fn poisoned_store(db: &std::path::Path, run_id: &str) {
    let mut store = Store::open(db).unwrap();
    store
        .create_run(
            run_id,
            "the acceptance that names no command",
            "test",
            &json!({}),
        )
        .unwrap();
    let mut append = |kind, payload| {
        store
            .append_next(run_id, kind, payload, None, None)
            .unwrap();
    };
    append(
        EventType::RunStarted,
        json!({"feature": "the acceptance that names no command", "manifest": {}}),
    );
    append(EventType::PhaseEntered, json!({"phase": "verify"}));
    append(
        EventType::EffectRequested,
        json!({"effect_id": "eff", "seat": "verify", "phase": "verify"}),
    );
    append(
        EventType::EffectStarted,
        json!({"effect_id": "eff", "attempt_id": "att"}),
    );
    append(
        EventType::OperatorCommanded,
        json!({"command_id": "cmd", "command": "stop", "args": {},
               "operator": "operator"}),
    );
    append(
        EventType::OperatorAccepted,
        json!({"command_id": "a-command-never-issued", "operator": "operator",
               "reason": "stop it now"}),
    );
}

/// The live journal the fold used to refuse, replayed into a store: the
/// verbatim fixture export (read-only evidence, 105 events) re-appended
/// `(type, payload)` pair by pair, so a fleet read meets the exact shape
/// the engine recorded — the operator's `stop` accepted at seq 93 while
/// the verify seat's effect was in flight — without fabricating an
/// operator's database.
pub(crate) fn stopped_mid_flight_store(db: &std::path::Path, run_id: &str) {
    stopped_mid_flight_run(db, run_id, &json!({}));
}

/// The same copy, created under a caller-chosen pinned manifest so a
/// `resume` can be aimed at it under its bundle. The fixture file itself
/// is never opened for writing and never edited.
pub(crate) fn stopped_mid_flight_run(db: &std::path::Path, run_id: &str, manifest: &Value) {
    let ndjson = std::fs::read_to_string(
        workspace().join("fixtures/journals/tui-graph-the-selection-box-gets-80f98deb.ndjson"),
    )
    .unwrap();
    let events: Vec<forge_core::envelope::EventEnvelope> = ndjson
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let mut store = Store::open(db).unwrap();
    store
        .create_run(run_id, "tui graph: the selection box", "test", manifest)
        .unwrap();
    for event in &events {
        store
            .append_next(run_id, event.event_type, event.payload.clone(), None, None)
            .unwrap();
    }
}

fn cli(command: Cmd) -> Cli {
    Cli { command }
}

#[test]
fn summaries_costs_inspect_export_and_error_closures_are_exercised() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "r1");

    let state = fold(&Store::open(&db).unwrap().load("r1").unwrap()).unwrap();
    assert_eq!(status_str(&Status::Running), "running");
    assert_eq!(finish(&state), ExitCode::from(1));
    running_store(&db, "ops");
    assert!(run(cli(Cmd::Operator {
        run: "ops".into(),
        command: "widen".into(),
        reason: "not allowed".into(),
        db: db.clone(),
    }))
    .is_err());
    for command in ["retry", "stop"] {
        assert_eq!(
            run(cli(Cmd::Operator {
                run: "ops".into(),
                command: command.into(),
                reason: "test".into(),
                db: db.clone(),
            }))
            .unwrap(),
            ExitCode::SUCCESS
        );
    }

    assert_eq!(
        run(cli(Cmd::Costs {
            run: "r1".into(),
            db: db.clone(),
        }))
        .unwrap(),
        ExitCode::SUCCESS
    );
    assert_eq!(
        run(cli(Cmd::Inspect {
            run: "r1".into(),
            db: db.clone(),
            json: false,
            phase: None,
            seat: None,
        }))
        .unwrap(),
        ExitCode::SUCCESS
    );
    let out = dir.path().join("export");
    assert_eq!(
        run(cli(Cmd::Export {
            run: "r1".into(),
            out: out.clone(),
            db: db.clone(),
        }))
        .unwrap(),
        ExitCode::SUCCESS
    );
    assert!(out.join("r1.manifest.json").is_file());
    let blocked = dir.path().join("blocked-export");
    std::fs::create_dir(&blocked).unwrap();
    std::fs::create_dir(blocked.join("r1.manifest.json")).unwrap();
    assert!(run(cli(Cmd::Export {
        run: "r1".into(),
        out: blocked,
        db: db.clone(),
    }))
    .is_err());

    let unknown_driver = run(cli(Cmd::Driver {
        kind: "unknown".into(),
        args: Vec::new(),
    }))
    .unwrap_err()
    .to_string();
    assert!(unknown_driver.contains("unknown driver"));
    // The known-driver list and the Driver help text both name the
    // whole fleet, lanetally included.
    for driver in ["claude", "lanetally", "codex", "dsh", "exec"] {
        assert!(unknown_driver.contains(driver), "{unknown_driver}");
    }
    use clap::CommandFactory;
    let driver_help = Cli::command()
        .find_subcommand("driver")
        .unwrap()
        .clone()
        .render_help()
        .to_string();
    assert!(driver_help.contains("lanetally"), "{driver_help}");

    let mut store = Store::open(&db).unwrap();
    store
        .create_run("missing-feature", "", "test", &json!({"files": {}}))
        .unwrap();
    store
        .append_next(
            "missing-feature",
            EventType::RunStarted,
            json!({"manifest": {}}),
            None,
            None,
        )
        .unwrap();
    assert!(run(cli(Cmd::Rerun {
        run: "missing-feature".into(),
        bundle: Some(workspace().join("recipes/fast")),
        recipe: None,
        recipes_dir: workspace().join("recipes"),
        db,
        repo: None,
        secrets_file: None,
    }))
    .unwrap_err()
    .to_string()
    .contains("has no run/started feature"));
}

#[test]
fn anchor_create_check_and_injected_ui_cover_command_boundaries() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "r1");
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    assert!(Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&repo)
        .status()
        .unwrap()
        .success());
    for (key, value) in [("user.name", "Forge Test"), ("user.email", "forge@test")] {
        assert!(Command::new("git")
            .args(["config", key, value])
            .current_dir(&repo)
            .status()
            .unwrap()
            .success());
    }
    for check in [false, true] {
        assert_eq!(
            run(cli(Cmd::Anchor {
                run: "r1".into(),
                db: db.clone(),
                repo: repo.clone(),
                check,
            }))
            .unwrap(),
            ExitCode::SUCCESS
        );
    }

    let mut seen = None;
    let code = run_with(
        cli(Cmd::Ui {
            db: db.clone(),
            port: 4321,
            open: true,
        }),
        |actual_db, port, open| {
            seen = Some((actual_db, port, open));
            Ok(())
        },
        None,
        None,
        run_tui,
    )
    .unwrap();
    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(seen, Some((db, 4321, true)));
}

fn dispatch_for(bundle: &Bundle, run_id: &str, callback: &str) -> DispatchEnvelopeV2 {
    let now = time::OffsetDateTime::now_utc();
    serde_json::from_value::<DispatchEnvelopeV2>(json!({
        "schema":"forge-dispatch/v2", "envelope_id":"envelope", "forge_run_id":run_id,
        "issued_at":(now-time::Duration::minutes(1)).format(&Rfc3339).unwrap(),
        "expires_at":(now+time::Duration::minutes(5)).format(&Rfc3339).unwrap(),
        "canonical_digest":"",
        "looper":{"organization_id":"org","product_id":"product","story_id":"story",
            "delivery_run_id":"delivery","request_grant_id":"grant","feature_path":"feature",
            "immutable_inputs_sha256":"a".repeat(64)},
        "actor":{"principal_kind":"api_key","principal_id":"key","actor_kind":"service",
            "actor_id":"forge","accountable_operator_id":"operator","authority_source":"looper-grant",
            "operating_profile":"bounded"},
        "repository":{"owner":"owner","name":"repo","base_sha":"b".repeat(64),
            "candidate_sha":null,"workspace_class":"isolated","target_environment":"dogfood"},
        "recipe":{"name":bundle.name,"compiled_sha256":bundle.manifest_digest()},
        "budget":{"lane_tally_run_id":"lane","reservation_id":null,"cost_state":"known",
            "ceiling_microunits":1000,"currency":"USD"},
        "producer":{"registration_id":"registration","token_reference":"key",
            "callback_audience":callback,"accepting_service_id":"looper-api",
            "runtime_id":"runtime","producer_release":"forge@test","protocol_version":1,
            "starting_cursor":0},
        "allowed_effects":PRODUCER_EFFECTS,"forbidden_actions":["grant_create","grant_widen",
            "artifact_decide","workflow_advance","release_promote"],
        "bounds":{"max_attempts":3,"max_parallel_effects":4,"max_event_bytes":65536,
            "max_events_per_ten_seconds":40,"replay_retention_seconds":604800,
            "safe_stop":"boundary","cancellation":"fenced"},
        "evidence_requirements":["ordered_hash_chain"],"attestation_requirement":"self_reported"
    }))
    .unwrap()
    .sealed()
}

#[test]
fn run_dispatch_refuses_io_and_json_then_accepts_a_verified_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let bundle_path = workspace().join("recipes/fast");
    let recipes_dir = workspace().join("recipes");
    let base = |dispatch| Cmd::Run {
        bundle: Some(bundle_path.clone()),
        recipe: None,
        recipes_dir: recipes_dir.clone(),
        feature: "feature".into(),
        db: dir.path().join("dispatch.db"),
        repo: None,
        dispatch,
        secrets_file: None,
    };

    let missing = dir.path().join("missing.json");
    assert!(run(cli(base(Some(missing))))
        .unwrap_err()
        .to_string()
        .contains("reading dispatch"));
    let malformed = dir.path().join("malformed.json");
    std::fs::write(&malformed, "not json").unwrap();
    assert!(run(cli(base(Some(malformed))))
        .unwrap_err()
        .to_string()
        .contains("parsing forge-dispatch/v2"));

    let bundle = Bundle::compile(&bundle_path).unwrap();
    let dispatch = dispatch_for(&bundle, "bound-run", "https://dogfood.example");
    let path = dir.path().join("dispatch.json");
    std::fs::write(&path, serde_json::to_string(&dispatch).unwrap()).unwrap();
    let code = run(cli(base(Some(path)))).unwrap();
    assert_eq!(code, ExitCode::from(2));
}

fn loopback_server(responses: Vec<String>) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        for body in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0_u8; 4096];
            let header_end = loop {
                let read = stream.read(&mut chunk).unwrap();
                assert!(read > 0);
                bytes.extend_from_slice(&chunk[..read]);
                if let Some(index) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    break index + 4;
                }
            };
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            while bytes.len() < header_end + length {
                let read = stream.read(&mut chunk).unwrap();
                assert!(read > 0);
                bytes.extend_from_slice(&chunk[..read]);
            }
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    (format!("http://{address}"), handle)
}

#[test]
fn bridge_command_covers_credentials_one_shot_and_bounded_follow() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("bridge.db");
    let (base_url, server) = loopback_server(
        (0..3)
            .flat_map(|_| {
                [
                    json!({"data":{"registration_id":"registration","status":"active",
                        "last_forge_sequence":0,"last_event_hash":ZERO_HASH}})
                    .to_string(),
                    json!({"data":{"replayed":true}}).to_string(),
                    json!({"data":[]}).to_string(),
                ]
            })
            .collect(),
    );
    let base_manifest = json!({
        "engine":"0.2.0", "event_schema":1, "database_schema":1,
        "driver_protocol":1, "bundle_name":"fast", "files":{"bundle.json":"a".repeat(64)}
    });
    let mut shell_bundle = Bundle::compile(&workspace().join("recipes/fast")).unwrap();
    shell_bundle.name = "fast".into();
    shell_bundle.manifest = base_manifest.clone();
    let mut dispatch = dispatch_for(&shell_bundle, "bridge-run", &base_url);
    dispatch.recipe.compiled_sha256 = sha256_hex(&base_manifest);
    dispatch = dispatch.sealed();
    let manifest = build_run_manifest_v2(&base_manifest, dispatch).unwrap();
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("bridge-run", "feature", "fast", &manifest)
        .unwrap();
    store
        .append_next("bridge-run", EventType::RunCompleted, json!({}), None, None)
        .unwrap();

    let token_name = format!("FORGE_TEST_TOKEN_{}", std::process::id());
    std::env::remove_var(&token_name);
    let command = |follow| Cmd::Bridge {
        run: "bridge-run".into(),
        db: db.clone(),
        looper_url: base_url.clone(),
        token_env: token_name.clone(),
        follow,
        interval_ms: 0,
    };
    assert!(run(cli(command(true)))
        .unwrap_err()
        .to_string()
        .contains("reading producer credential"));
    std::env::set_var(&token_name, "   ");
    assert!(run(cli(command(true)))
        .unwrap_err()
        .to_string()
        .contains("credential is empty"));
    std::env::set_var(&token_name, "test-token");
    assert!(run_with(
        cli(Cmd::Bridge {
            run: "missing-run".into(),
            db: db.clone(),
            looper_url: "http://127.0.0.1:1".into(),
            token_env: token_name.clone(),
            follow: false,
            interval_ms: 0,
        }),
        ui::serve,
        Some(1),
        None,
        run_tui,
    )
    .is_err());
    assert_eq!(
        run_with(cli(command(false)), ui::serve, None, None, run_tui).unwrap(),
        ExitCode::SUCCESS
    );
    assert_eq!(
        run_with(cli(command(true)), ui::serve, Some(2), None, run_tui).unwrap(),
        ExitCode::SUCCESS
    );
    std::env::remove_var(&token_name);
    server.join().unwrap();
}

#[test]
fn state_constructor_keeps_the_running_cursor_shape_explicit() {
    assert_eq!(
        driver_extra_args(vec!["before".into(), "--".into(), "after".into()]),
        vec!["after"]
    );
    assert_eq!(driver_extra_args(vec!["plain".into()]), vec!["plain"]);
    let state = RunState {
        run_id: "run".into(),
        seq: 0,
        last_hash: ZERO_HASH.into(),
        status: Status::Running,
        phase: None,
        cursor: Cursor::EnterPhase {
            phase: "work".into(),
        },
        consecutive_failures: Default::default(),
        reviewed_heads: None,
        last_decision: None,
        park_reason: None,
        feature: Some("feature".into()),
        pending_command: None,
        riding_stop: false,
    };
    assert_eq!(summarize(&state)["status"], "running");
}

/// A stdout that always refuses: the frame writer's error path is real
/// (a closed pipe), not decoration.
struct ClosedPipe;

impl Write for ClosedPipe {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("closed pipe"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn fixed_clock() -> String {
    "2026-01-01T00:00:00Z".to_string()
}

#[test]
fn watch_redraws_on_seq_and_on_a_hash_only_change_and_leaves_the_journal_alone() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "r1");
    let before = Store::open(&db).unwrap().export_ndjson("r1").unwrap();
    let style = render::Style::plain(80);

    // Two polls with nothing moving: one frame, one sleep at the 100ms
    // floor, and the run is still running so the loop simply ends.
    let mut frames = Vec::new();
    let mut ticks: Vec<u64> = Vec::new();
    let code = {
        let mut sleep = |ms: u64| ticks.push(ms);
        watch_loop(
            &db,
            "r1",
            5,
            false,
            &style,
            &mut frames,
            &mut fixed_clock,
            &mut sleep,
            2,
        )
        .unwrap()
    };
    assert_eq!(code, ExitCode::from(1), "a running run exhausts the poll");
    let text = String::from_utf8(frames).unwrap();
    assert_eq!(text.matches("── 2026-01-01T00:00:00Z ──").count(), 1);
    assert!(!text.contains('\x1b'), "a pipe gets no ANSI: {text:?}");
    assert!(text.contains("graph"), "{text}");
    assert!(!text.contains("trail"), "a frame carries no trail: {text}");
    assert_eq!(ticks, vec![100], "--interval is floored at 100ms");

    // A seq change redraws.
    let mut frames = Vec::new();
    let mut appended = false;
    {
        let mut sleep = |_: u64| {
            if !appended {
                appended = true;
                Store::open(&db)
                    .unwrap()
                    .append_next(
                        "r1",
                        EventType::EffectRequested,
                        json!({"effect_id": "eff1", "seat": "work", "phase": "work"}),
                        None,
                        None,
                    )
                    .unwrap();
            }
        };
        watch_loop(
            &db,
            "r1",
            1000,
            false,
            &style,
            &mut frames,
            &mut fixed_clock,
            &mut sleep,
            2,
        )
        .unwrap();
    }
    let text = String::from_utf8(frames).unwrap();
    assert_eq!(text.matches("── ").count(), 2, "seq moved, so redraw");

    // A rewritten journal at EQUAL seq is the tamper case `anchor`
    // exists for: watch compares hash as well as seq and redraws rather
    // than sitting blind.
    let tampered = dir.path().join("tampered.db");
    running_store(&tampered, "r1");
    let mut frames = Vec::new();
    let mut rewritten = false;
    {
        let mut sleep = |_: u64| {
            if !rewritten {
                rewritten = true;
                std::fs::remove_file(&tampered).unwrap();
                let mut store = Store::open(&tampered).unwrap();
                store
                    .create_run("r1", "other", "test", &json!({"files": {}}))
                    .unwrap();
                store
                    .append_next(
                        "r1",
                        EventType::RunStarted,
                        json!({"feature": "other", "manifest": {}}),
                        None,
                        None,
                    )
                    .unwrap();
                store
                    .append_next(
                        "r1",
                        EventType::PhaseEntered,
                        json!({"phase": "work"}),
                        None,
                        None,
                    )
                    .unwrap();
            }
        };
        watch_loop(
            &tampered,
            "r1",
            1000,
            true,
            &style,
            &mut frames,
            &mut fixed_clock,
            &mut sleep,
            2,
        )
        .unwrap();
    }
    let text = String::from_utf8(frames).unwrap();
    assert_eq!(
        text.matches("\x1b[2J\x1b[H").count(),
        2,
        "same seq, different hash: {text:?}"
    );

    // Read-only: nothing above appended to the journal this test owns.
    assert_eq!(
        Store::open(&tampered).unwrap().export_ndjson("r1").unwrap(),
        Store::open(&tampered).unwrap().export_ndjson("r1").unwrap()
    );
    let store = Store::open(&db).unwrap();
    let after = store.export_ndjson("r1").unwrap();
    assert!(after.starts_with(&before), "watch never rewrites history");
}

#[test]
fn watch_frames_a_transient_error_gives_up_on_a_persistent_one_and_reports_a_closed_pipe() {
    let dir = tempfile::tempdir().unwrap();
    let style = render::Style::plain(80);
    let corrupt = dir.path().join("corrupt.db");
    std::fs::write(&corrupt, "not sqlite").unwrap();

    // A transient store error is a frame that says so, not an exit.
    let mut frames = Vec::new();
    let code = watch_loop(
        &corrupt,
        "r1",
        1000,
        false,
        &style,
        &mut frames,
        &mut fixed_clock,
        &mut |_| {},
        1,
    )
    .unwrap();
    assert_eq!(code, ExitCode::from(1));
    let text = String::from_utf8(frames).unwrap();
    assert!(
        text.contains("the journal is not readable right now"),
        "{text}"
    );

    // A persistent one exits nonzero.
    let mut frames = Vec::new();
    let error = watch_loop(
        &corrupt,
        "r1",
        1000,
        false,
        &style,
        &mut frames,
        &mut fixed_clock,
        &mut |_| {},
        9,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("unreadable polls"), "{error}");

    // An unknown run: head_hash answers zero, the load does not.
    let db = dir.path().join("forge.db");
    running_store(&db, "r1");
    let missing = watch_loop(
        &db,
        "nobody",
        1000,
        false,
        &style,
        &mut Vec::new(),
        &mut fixed_clock,
        &mut |_| {},
        1,
    )
    .unwrap_err()
    .to_string();
    assert!(missing.contains("nobody"), "{missing}");

    // A journal that does not fold still draws: the frame says what it
    // can and never guesses a status.
    let unfoldable = dir.path().join("unfoldable.db");
    running_store(&unfoldable, "r1");
    Store::open(&unfoldable)
        .unwrap()
        .append_next(
            "r1",
            EventType::RunStarted,
            json!({"feature": "again", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    let mut frames = Vec::new();
    watch_loop(
        &unfoldable,
        "r1",
        1000,
        false,
        &style,
        &mut frames,
        &mut fixed_clock,
        &mut |_| {},
        1,
    )
    .unwrap();
    let text = String::from_utf8(frames).unwrap();
    assert!(text.contains("this journal does not fold"), "{text}");

    // A closed pipe is an error, not a silent no-op.
    let closed = watch_loop(
        &db,
        "r1",
        1000,
        false,
        &style,
        &mut ClosedPipe,
        &mut fixed_clock,
        &mut |_| {},
        1,
    );
    assert!(closed.is_err());
    let closed_tty = watch_loop(
        &db,
        "r1",
        1000,
        true,
        &style,
        &mut ClosedPipe,
        &mut fixed_clock,
        &mut |_| {},
        1,
    );
    assert!(closed_tty.is_err());
}

/// The verb, its selector, and the promise that a read never creates a
/// database (decision 0014's AC-1 and AC-2).
#[test]
fn the_tui_verb_resolves_its_run_and_never_opens_a_database_it_might_create() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "run-alpha");

    let mut seen = None;
    let code = run_with(
        cli(Cmd::Tui {
            run: Some("run-al".into()),
            db: db.clone(),
        }),
        ui::serve,
        None,
        None,
        |db, run| {
            seen = Some((db, run));
            Ok(ExitCode::SUCCESS)
        },
    )
    .unwrap();
    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(
        seen,
        Some((db.clone(), Some("run-alpha".to_string()))),
        "a prefix resolves through the one resolver, never a second copy"
    );

    // A missing database: the selector is not consulted, nothing is
    // opened, and `forge tui` does the refusing.
    let missing = dir.path().join("nowhere.db");
    let mut seen = None;
    run_with(
        cli(Cmd::Tui {
            run: Some("latest".into()),
            db: missing.clone(),
        }),
        ui::serve,
        None,
        None,
        |db, run| {
            seen = Some((db, run));
            Ok(ExitCode::SUCCESS)
        },
    )
    .unwrap();
    assert_eq!(seen, Some((missing.clone(), Some("latest".to_string()))));
    assert!(!missing.exists(), "a read never creates a database");
    assert!(!dir.path().join("nowhere.db-wal").exists());

    // No `--run` at all: the fleet is where the console opens.
    let mut seen = None;
    run_with(
        cli(Cmd::Tui {
            run: None,
            db: db.clone(),
        }),
        ui::serve,
        None,
        None,
        |db, run| {
            seen = Some((db, run));
            Ok(ExitCode::SUCCESS)
        },
    )
    .unwrap();
    assert_eq!(seen, Some((db, None)));

    use clap::CommandFactory;
    let help = Cli::command().render_help().to_string();
    assert!(help.contains("tui"), "the verb is listed: {help}");
}

/// The console's liveness, at the one place a store is opened on its
/// path: head-gated on both seq and hash, fleet on the slower cadence,
/// and one unfoldable run keeping its row.
#[test]
fn the_tui_refresh_is_head_gated_on_seq_and_hash_and_keeps_an_unfoldable_run() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "r1");
    let clock = || "2026-01-01T00:07:03Z".to_string();
    let ask = |run, force, fleet| tui::Ask {
        run,
        session: None,
        working: false,
        force,
        fleet,
    };
    let mut head = None;

    let first = tui_views(
        &db,
        ask(Some("r1"), true, false),
        &mut head,
        &mut None,
        clock,
    )
    .unwrap()
    .expect("the first frame is forced");
    assert_eq!(first.runs.runs.len(), 1);
    assert!(first.run.is_some());
    assert_eq!(first.now, "2026-01-01T00:07:03Z");
    assert!(first.transcript.is_none(), "no seat is open");

    assert!(
        tui_views(
            &db,
            ask(Some("r1"), false, false),
            &mut head,
            &mut None,
            clock
        )
        .unwrap()
        .is_none(),
        "nothing moved, so nothing is re-folded"
    );
    assert!(
        tui_views(
            &db,
            ask(Some("r1"), false, true),
            &mut head,
            &mut None,
            clock
        )
        .unwrap()
        .is_some(),
        "the fleet's slower cadence rebuilds anyway"
    );
    assert!(
        tui_views(&db, ask(None, false, false), &mut head, &mut None, clock)
            .unwrap()
            .is_some(),
        "leaving a run behind moves the head to None"
    );
    head = Some((2, String::new()));

    // A seq move redraws.
    Store::open(&db)
        .unwrap()
        .append_next(
            "r1",
            EventType::EffectRequested,
            json!({"effect_id": "eff", "seat": "work", "phase": "work"}),
            None,
            None,
        )
        .unwrap();
    assert!(
        tui_views(
            &db,
            ask(Some("r1"), false, false),
            &mut head,
            &mut None,
            clock
        )
        .unwrap()
        .is_some(),
        "a moved head redraws"
    );

    // A rewritten journal at EQUAL seq is the tamper case `anchor`
    // exists for: the head is compared on hash as well.
    let tampered = dir.path().join("tampered.db");
    running_store(&tampered, "r1");
    let mut head = None;
    assert!(tui_views(
        &tampered,
        ask(Some("r1"), true, false),
        &mut head,
        &mut None,
        clock
    )
    .unwrap()
    .is_some());
    let seq_before = head.clone().unwrap().0;
    std::fs::remove_file(&tampered).unwrap();
    running_store(&tampered, "r1");
    Store::open(&tampered)
        .unwrap()
        .append_next(
            "r1",
            EventType::PhaseEntered,
            json!({"phase": "other"}),
            None,
            None,
        )
        .unwrap();
    let mut head_at_equal_seq = Some((seq_before, "a different hash".to_string()));
    assert!(
        tui_views(
            &tampered,
            ask(Some("r1"), false, false),
            &mut head_at_equal_seq,
            &mut None,
            clock
        )
        .unwrap()
        .is_some(),
        "same seq, different hash: the console redraws rather than sitting blind"
    );

    // One run whose journal does not fold keeps its row with the
    // model's absence mark; the whole fleet does not vanish with it.
    let mixed = dir.path().join("mixed.db");
    running_store(&mixed, "good");
    running_store(&mixed, "bad");
    Store::open(&mixed)
        .unwrap()
        .append_next(
            "bad",
            EventType::RunStarted,
            json!({"feature": "again", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    let mut head = None;
    let views = tui_views(&mixed, ask(None, true, false), &mut head, &mut None, clock)
        .unwrap()
        .unwrap();
    assert_eq!(views.runs.runs.len(), 2, "both runs are listed");
    let broken = views
        .runs
        .runs
        .iter()
        .find(|row| row.run_id == "bad")
        .unwrap();
    assert!(broken.status.is_none() && !broken.status_known);

    // A missing session is an absent transcript, never an invented one.
    let mut head = None;
    let views = tui_views(
        &mixed,
        tui::Ask {
            run: None,
            session: Some("9999-9999"),
            working: false,
            force: true,
            fleet: false,
        },
        &mut head,
        &mut None,
        clock,
    )
    .unwrap()
    .unwrap();
    assert!(views.transcript.is_none());

    // An unreadable store is an error the shell frames, not a panic.
    let corrupt = dir.path().join("corrupt.db");
    std::fs::write(&corrupt, "not sqlite").unwrap();
    let mut head = None;
    assert!(tui_views(
        &corrupt,
        ask(None, true, false),
        &mut head,
        &mut None,
        clock
    )
    .is_err());

    // The production source binds the workspace clock, and reading
    // through it leaves the journal exactly as it was.
    let before = Store::open(&db).unwrap().export_ndjson("r1").unwrap();
    let mut head = None;
    let mut seen = None;
    let mut source = tui_source(&db, &mut head, &mut seen);
    let views = source(ask(Some("r1"), true, false)).unwrap().unwrap();
    assert!(views.now.ends_with('Z'));
    assert_eq!(
        Store::open(&db).unwrap().export_ndjson("r1").unwrap(),
        before
    );
}

/// The other half of the console's liveness: a seat's prose lands
/// BETWEEN journal checkpoints, so the same poll that compares the head
/// also asks the transcript file its length — while the seat is
/// working, and not once it has concluded. Nothing here writes, and the
/// pure core never learns a file exists.
#[test]
fn a_working_seats_transcript_growing_forces_the_shell_to_re_read_it() {
    let _home = HOME.lock().unwrap_or_else(|error| error.into_inner());
    let previous_home = std::env::var_os("HOME");
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "r1");
    let home = dir.path().join("home");
    let projects = home.join(".claude").join("projects").join("live-project");
    std::fs::create_dir_all(&projects).unwrap();
    std::env::set_var("HOME", &home);

    let clock = || "2026-01-01T00:07:03Z".to_string();
    let poll = |session, working| tui::Ask {
        run: Some("r1"),
        session,
        working,
        force: false,
        fleet: false,
    };
    let turn = |text: &str| {
        format!("{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"{text}\"}}}}\n")
    };
    let mut head = None;
    let mut seen = None;

    // The first frame is forced; it settles the head and the length.
    assert!(tui_views(
        &db,
        tui::Ask {
            run: Some("r1"),
            session: Some("abcd-1234"),
            working: true,
            force: true,
            fleet: false,
        },
        &mut head,
        &mut seen,
        clock,
    )
    .unwrap()
    .is_some());
    assert!(
        tui_views(
            &db,
            poll(Some("abcd-1234"), true),
            &mut head,
            &mut seen,
            clock
        )
        .unwrap()
        .is_none(),
        "no journal move and no transcript at all: the frame stands"
    );

    // The seat's transcript appears, then gains a turn. Neither moves
    // the journal head, and both must reach the operator's eye.
    let file = projects.join("abcd-1234.jsonl");
    std::fs::write(&file, turn("the first words")).unwrap();
    let views = tui_views(
        &db,
        poll(Some("abcd-1234"), true),
        &mut head,
        &mut seen,
        clock,
    )
    .unwrap()
    .expect("a transcript that appeared is prose the operator is waiting for");
    assert_eq!(views.transcript.unwrap().0.len(), 1);
    assert!(
        tui_views(
            &db,
            poll(Some("abcd-1234"), true),
            &mut head,
            &mut seen,
            clock
        )
        .unwrap()
        .is_none(),
        "an unchanged length is not a reason to re-read a multi-megabyte file"
    );
    std::fs::write(
        &file,
        format!("{}{}", turn("the first words"), turn("and the next")),
    )
    .unwrap();
    let views = tui_views(
        &db,
        poll(Some("abcd-1234"), true),
        &mut head,
        &mut seen,
        clock,
    )
    .unwrap()
    .expect("the file grew");
    assert_eq!(views.transcript.unwrap().0.len(), 2);

    // A different seat is a different watch: one transcript's length
    // never speaks for another's, however much longer it happens to be.
    // Switching seats is a level change, which forces its own refresh.
    std::fs::write(
        projects.join("0000-1111.jsonl"),
        format!(
            "{}{}{}",
            turn("a different seat"),
            turn("with more to say"),
            turn("than the first one had")
        ),
    )
    .unwrap();
    assert!(
        tui_views(
            &db,
            poll(Some("0000-1111"), true),
            &mut head,
            &mut seen,
            clock
        )
        .unwrap()
        .is_none(),
        "a longer file under a different id is not this seat's growth"
    );

    // Once the seat concludes the file is not asked about at all: the
    // journal's own head is the only thing left that can move.
    assert!(tui_views(
        &db,
        poll(Some("abcd-1234"), false),
        &mut head,
        &mut seen,
        clock
    )
    .unwrap()
    .is_none());
    std::fs::write(
        &file,
        format!(
            "{}{}{}",
            turn("the first words"),
            turn("and the next"),
            turn("written after the seat concluded")
        ),
    )
    .unwrap();
    assert!(
        tui_views(
            &db,
            poll(Some("abcd-1234"), false),
            &mut head,
            &mut seen,
            clock
        )
        .unwrap()
        .is_none(),
        "a concluded seat's file growing is not the console's business"
    );

    if let Some(previous_home) = previous_home {
        std::env::set_var("HOME", previous_home);
    } else {
        std::env::remove_var("HOME");
    }
}

#[test]
fn the_readouts_render_and_scope_from_the_one_derivation() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let db_path = db.clone();
    running_store(&db, "r1");

    for json in [false, true] {
        assert_eq!(
            run(cli(Cmd::Runs {
                db: db.clone(),
                json,
            }))
            .unwrap(),
            ExitCode::SUCCESS
        );
        assert_eq!(
            run(cli(Cmd::Inspect {
                run: "r1".into(),
                db: db.clone(),
                json,
                phase: None,
                seat: None,
            }))
            .unwrap(),
            ExitCode::SUCCESS
        );
    }
    // The scoping verbs the console's clicks became.
    assert_eq!(
        run(cli(Cmd::Inspect {
            run: "r1".into(),
            db: db.clone(),
            json: false,
            phase: Some("work".into()),
            seat: None,
        }))
        .unwrap(),
        ExitCode::SUCCESS
    );
    let unknown = run(cli(Cmd::Inspect {
        run: "r1".into(),
        db: db.clone(),
        json: false,
        phase: None,
        seat: Some("nobody".into()),
    }))
    .unwrap_err()
    .to_string();
    assert!(unknown.contains("no seat 'nobody'"), "{unknown}");

    // `--once` is one frame; the iteration limit stands in for the
    // operator's Ctrl-C in the looping form.
    assert_eq!(
        run_with(
            cli(Cmd::Watch {
                run: "r1".into(),
                db: db.clone(),
                once: true,
                interval_ms: 100,
            }),
            ui::serve,
            None,
            None,
            run_tui,
        )
        .unwrap(),
        ExitCode::from(1),
        "a running run is finish()'s exit 1"
    );
    assert_eq!(
        run_with(
            cli(Cmd::Watch {
                run: "r1".into(),
                db,
                once: false,
                interval_ms: 100,
            }),
            ui::serve,
            None,
            Some(2),
            run_tui,
        )
        .unwrap(),
        ExitCode::from(1),
        "two polls: the second one sleeps first"
    );

    // The migration, as a checkable equality: `forge inspect --json`
    // nests today's output under `summary`, all nine keys verbatim and
    // in the same order, so `| jq .summary` reproduces it byte for byte.
    let events = Store::open(&db_path).unwrap().load("r1").unwrap();
    let state = fold(&events).unwrap();
    let view = forge_view::run_view(&events, Some(&state));
    assert_eq!(
        serde_json::to_string_pretty(&view.summary).unwrap(),
        serde_json::to_string_pretty(&summarize(&state)).unwrap()
    );

    // The clock the derivation refuses to read.
    assert!(now_rfc3339().ends_with('Z'));
}

/// The run that used to be the quarantine's own example: an operator
/// `stop` accepted while the verify seat's effect was in flight. The
/// fold now has that arm, so the fleet reads it as what it is — there
/// is nothing to quarantine here, and the verbs aimed at it work.
#[test]
fn an_operator_stop_mid_flight_lists_with_its_real_status() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    stopped_mid_flight_store(&db, "stopped-mid-flight");
    running_store(&db, "healthy");

    let store = Store::open(&db).unwrap();
    let events = store.load("stopped-mid-flight").unwrap();
    let folded = fold_or_quarantine(&events);
    let state = folded.as_ref().expect("the live journal folds");
    assert_eq!(state.seq, 105);
    assert_eq!(state.status, Status::Running);
    assert_eq!(state.phase.as_deref(), Some("verify"));
    assert_eq!(state.cursor, Cursor::Stop, "concluded per the operator");

    let entries = [
        forge_view::RunEntry {
            run_id: "stopped-mid-flight",
            feature: "tui graph: the selection box",
            created_at: "2026-08-30T22:20:34Z",
            state: folded.as_ref().ok(),
            detail: folded.as_ref().err().map(String::as_str),
        },
        forge_view::RunEntry {
            run_id: "healthy",
            feature: "feature",
            created_at: "2026-08-30T22:21:00Z",
            state: None,
            detail: None,
        },
    ];
    let view = serde_json::to_value(forge_view::run_rows(&entries)).unwrap();
    let row = &view["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["run_id"] == "stopped-mid-flight")
        .expect("the run is listed")
        .clone();
    assert_eq!(row["status"], "running", "its real status, not '?'");
    assert_eq!(row["status_known"], true);
    assert_eq!(row["seq"], 105);
    assert_eq!(row["detail"], Value::Null, "nothing to quarantine");

    // …and the single-run verb aimed at it reads it instead of refusing.
    assert_eq!(
        run(cli(Cmd::Inspect {
            run: "stopped-mid-flight".into(),
            db: db.clone(),
            json: true,
            phase: None,
            seat: None,
        }))
        .unwrap(),
        ExitCode::SUCCESS,
        "the verb reads the journal instead of refusing it"
    );
}

/// The lawful way to finish a sentence the old engine left hanging: an
/// accepted-but-unconcluded operator stop, aimed at with `resume`. The
/// journal is a COPY of the fixture in a temp store — no operator
/// database is touched and the fixture is never edited — and the run it
/// leaves behind reads `stopped`, with the process exiting 3 (hard
/// stop), not 0.
#[test]
fn resume_concludes_an_accepted_but_unconcluded_operator_stop_and_exits_three() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let bundle_path = workspace().join("recipes/fast");
    let bundle = Bundle::compile(&bundle_path).unwrap();
    stopped_mid_flight_run(&db, "stopped-mid-flight", &bundle.manifest);

    assert_eq!(
        run(cli(Cmd::Resume {
            bundle: Some(bundle_path),
            recipe: None,
            recipes_dir: workspace().join("recipes"),
            run: "stopped-mid-flight".into(),
            db: db.clone(),
            repo: None,
            secrets_file: None,
        }))
        .unwrap(),
        ExitCode::from(3),
        "a stopped run reporting success would be a lie to the shell",
    );

    let events = Store::open(&db)
        .unwrap()
        .load("stopped-mid-flight")
        .unwrap();
    let tail = events.last().unwrap();
    assert_eq!(tail.seq, 106, "appended, never rewritten");
    assert_eq!(tail.event_type, EventType::RunStopped);
    let reason = tail.payload["reason"].as_str().unwrap();
    // The cause, named: the operator's own command as the journal
    // recorded it at seq 92/93.
    assert!(reason.starts_with("OPERATOR-STOP: "), "{reason}");
    assert!(reason.contains("vyanakiev"), "{reason}");
    assert!(
        reason.contains("78af2044-21f4-4ad7-91e0-f141d239f0ce"),
        "{reason}"
    );
    assert!(
        reason.contains("the fence post through the rail"),
        "{reason}"
    );

    // Round trip: the fold reads the tail the engine just wrote.
    let state = fold(&events).unwrap();
    assert_eq!(state.status, Status::Stopped);
    assert_eq!(finish(&state), ExitCode::from(3));
}

/// One poisoned journal in the fleet is a quarantined row, not a blind
/// operator. The fleet verb lists every other run and says why this one
/// carries no status; the single-run verbs aimed AT it still fail
/// loudly, because a command naming one run must not quietly succeed
/// against a journal it could not read.
#[test]
fn one_unfoldable_journal_is_quarantined_by_the_fleet_and_fatal_to_its_own_verbs() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "healthy");
    poisoned_store(&db, "poisoned");

    for json in [false, true] {
        assert_eq!(
            run(cli(Cmd::Runs {
                db: db.clone(),
                json,
            }))
            .unwrap(),
            ExitCode::SUCCESS,
            "the fleet listing survives the poisoned run"
        );
    }

    // The rows that listing built, checked at the model rather than
    // through the terminal: both runs present, one of them quarantined
    // in the fold's own words.
    let store = Store::open(&db).unwrap();
    let folded: Vec<(
        String,
        String,
        String,
        std::result::Result<RunState, String>,
    )> = store
        .list_runs()
        .unwrap()
        .into_iter()
        .map(|(run_id, feature, created_at)| {
            let folded = fold_or_quarantine(&store.load(&run_id).unwrap());
            (run_id, feature, created_at, folded)
        })
        .collect();
    let entries: Vec<forge_view::RunEntry> = folded
        .iter()
        .map(
            |(run_id, feature, created_at, folded)| forge_view::RunEntry {
                run_id,
                feature,
                created_at,
                state: folded.as_ref().ok(),
                detail: folded.as_ref().err().map(String::as_str),
            },
        )
        .collect();
    let view = serde_json::to_value(forge_view::run_rows(&entries)).unwrap();
    assert_eq!(view["count"], 2, "no run is dropped: {view}");
    let row = |run_id: &str| -> Value {
        view["runs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["run_id"] == run_id)
            .expect("every run is listed")
            .clone()
    };
    let poisoned = row("poisoned");
    assert_eq!(poisoned["status"], Value::Null, "rendered as '?'");
    assert_eq!(poisoned["status_known"], false);
    assert!(
        poisoned["detail"]
            .as_str()
            .unwrap()
            .starts_with("event 6: operator/accepted without a matching command"),
        "the fold's own words survive into the row: {poisoned}"
    );
    let healthy = row("healthy");
    assert_eq!(healthy["status"], "running");
    assert_eq!(healthy["detail"], Value::Null);

    // Aimed at that run, the same refusal is fatal: these verbs keep
    // their bare `fold(..)?` and are not softened by the fleet's grace.
    for command in [
        Cmd::Inspect {
            run: "poisoned".into(),
            db: db.clone(),
            json: false,
            phase: None,
            seat: None,
        },
        Cmd::Replay {
            run: "poisoned".into(),
            db: db.clone(),
        },
    ] {
        let error = run(cli(command)).unwrap_err().to_string();
        assert!(
            error.contains("without a matching command"),
            "the verb names the refusal: {error}"
        );
    }
    // `watch` keeps the console's own unchanged behaviour: it renders
    // the absence rather than a status, and never exits success on it.
    assert_eq!(
        run(cli(Cmd::Watch {
            run: "poisoned".into(),
            db: db.clone(),
            once: true,
            interval_ms: 100,
        }))
        .unwrap(),
        ExitCode::from(1)
    );
}
