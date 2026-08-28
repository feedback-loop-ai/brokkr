use super::*;
use forge_core::canonical::{sha256_hex, ZERO_HASH};
use forge_core::dispatch::{build_run_manifest_v2, DispatchEnvelopeV2, PRODUCER_EFFECTS};
use forge_core::fold::Cursor;
use forge_core::EventType;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use time::format_description::well_known::Rfc3339;

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
    )
    .is_err());
    assert_eq!(
        run_with(cli(command(false)), ui::serve, None, None).unwrap(),
        ExitCode::SUCCESS
    );
    assert_eq!(
        run_with(cli(command(true)), ui::serve, Some(2), None).unwrap(),
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
