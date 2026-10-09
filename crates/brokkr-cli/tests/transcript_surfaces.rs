//! One local derivation across the surfaces (#222, proposed decision
//! 0055; design D11). A synthetic source of each kind is read once
//! through `brokkr_cli::read_local`; the command's text and JSON, the
//! TUI pane keys and both doors, the browser's transcript route (the
//! command's JSON less its trailing newline, #352) and the browser participant
//! presentation are then compared against that one result. The
//! presentation transport is asserted prose-free.

use std::path::{Path, PathBuf};
use std::process::Command;

use brokkr_core::envelope::EventType;
use brokkr_store::Store;
use brokkr_view::transcript::{LegacyProvenance, TranscriptRead};
use serde_json::{json, Value};

/// `HOME` is process-global and the in-process browser routes resolve it
/// (for legacy synthesis) even when the selected reference carries its own
/// home, so every test in this file holds the binary's environment guard.
#[path = "../../../tests/support/env_guard.rs"]
mod env_guard;
use env_guard::EnvGuard;

struct World {
    dir: tempfile::TempDir,
    db: PathBuf,
    home: PathBuf,
    reference: brokkr_view::Transcript,
    path: String,
}

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

/// Materialize one synthetic source of `kind` under `home` and return the
/// recorded reference and the confirmed path.
fn source(home: &Path, kind: &str, locator: &str, body: &str) -> (brokkr_view::Transcript, String) {
    match kind {
        "claude-session" => {
            let projects = home.join(".claude").join("projects");
            let dir = projects.join("project");
            std::fs::create_dir_all(&dir).unwrap();
            let file = dir.join(format!("{locator}.jsonl"));
            std::fs::write(&file, body).unwrap();
            (
                brokkr_view::Transcript {
                    kind: kind.to_string(),
                    locator: locator.to_string(),
                    home: projects.to_str().unwrap().to_string(),
                },
                format!(
                    "{}/project/{locator}.jsonl",
                    projects.canonicalize().unwrap().display()
                ),
            )
        }
        "codex-thread" => {
            let dir = home.join("sessions");
            std::fs::create_dir_all(&dir).unwrap();
            let file = dir.join(format!("rollout-{locator}.jsonl"));
            std::fs::write(&file, body).unwrap();
            (
                brokkr_view::Transcript {
                    kind: kind.to_string(),
                    locator: locator.to_string(),
                    home: home.to_str().unwrap().to_string(),
                },
                format!(
                    "{}/sessions/rollout-{locator}.jsonl",
                    home.canonicalize().unwrap().display()
                ),
            )
        }
        "dsh-session" => {
            let dir = home.join(locator).join("project").join("seat");
            std::fs::create_dir_all(&dir).unwrap();
            let file = dir.join("session.jsonl");
            std::fs::write(&file, body).unwrap();
            (
                brokkr_view::Transcript {
                    kind: kind.to_string(),
                    locator: locator.to_string(),
                    home: home.to_str().unwrap().to_string(),
                },
                format!(
                    "{}/{locator}/project/seat/session.jsonl",
                    home.canonicalize().unwrap().display()
                ),
            )
        }
        other => panic!("unknown kind {other}"),
    }
}

/// A one-participant run whose checkpoint carries the reference.
fn make_world(kind: &str, locator: &str, body: &str) -> World {
    make_world_named(kind, locator, body, "home")
}

/// The same world under a chosen (possibly hostile) home directory name.
fn make_world_named(kind: &str, locator: &str, body: &str, home_name: &str) -> World {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let home = dir.path().join(home_name);
    std::fs::create_dir_all(&home).unwrap();
    let (reference, path) = source(&home, kind, locator, body);
    journal(
        &db,
        json!({"step": "session-finished", "transcript": reference}),
    );
    World {
        dir,
        db,
        home,
        reference,
        path,
    }
}

/// The one-participant run every world records at `db`, its seat's last
/// checkpoint being `checkpoint`.
fn journal(db: &Path, checkpoint: Value) {
    let mut store = Store::open(db).unwrap();
    store
        .create_run("r222", "feat", "self", &json!({"files": {}}))
        .unwrap();
    let opening = [
        (EventType::RunStarted, r#"{"feature":"feat","manifest":{}}"#),
        (EventType::PhaseEntered, r#"{"phase":"intake"}"#),
        (
            EventType::EffectRequested,
            r#"{"effect_id":"eff1","seat":"review","phase":"intake"}"#,
        ),
        (
            EventType::EffectStarted,
            r#"{"effect_id":"eff1","attempt_id":"att0"}"#,
        ),
    ];
    let checkpointed = json!({"effect_id": "eff1", "attempt_id": "att0", "checkpoint": checkpoint});
    let opening =
        opening.map(|(event_type, payload)| (event_type, serde_json::from_str(payload).unwrap()));
    for (event_type, payload) in opening
        .into_iter()
        .chain([(EventType::EffectCheckpointed, checkpointed)])
    {
        store
            .append_next("r222", event_type, payload, None, None)
            .unwrap();
    }
}

/// The shared reader's result for `world`'s recorded reference. A
/// recorded reference needs no projects home, so the shell passes none.
fn shared_read(world: &World) -> TranscriptRead {
    let subject = brokkr_view::Subject {
        run: "r222".to_string(),
        key: "eff1".to_string(),
        reference: Some(world.reference.clone()),
        provenance: LegacyProvenance::Absent,
        legacy_id: None,
        working: false,
        tab: 0,
        realm: None,
    };
    brokkr_cli::read_local(&subject, None)
}

fn command(world: &World, extra: &[&str]) -> std::process::Output {
    command_under(world, &world.home, extra)
}

/// The command run with `home` as its `HOME`.
fn command_under(world: &World, home: &Path, extra: &[&str]) -> std::process::Output {
    let mut args = vec!["transcript", "--run", "r222", "--seat", "eff1"];
    args.extend_from_slice(extra);
    Command::new(brokkr_bin())
        .args(args)
        .arg("--db")
        .arg(&world.db)
        .env("HOME", home)
        .current_dir(world.dir.path())
        .output()
        .unwrap()
}

fn parse(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is not one JSON document: {error}; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// The browser's transcript route serves `brokkr transcript --json`'s
/// document byte for byte, for every kind (#352): the command prints the
/// same bytes and a newline. A refused read keeps its document under a 404.
fn same_bytes_in_the_browser(world: &World, command: &std::process::Output, status: &str) {
    let response = brokkr_cli::handle(&world.db, "/api/transcript/r222/eff1");
    assert_eq!(response.status, status, "{}", response.body);
    assert_eq!(
        format!("{}\n", response.body),
        String::from_utf8_lossy(&command.stdout),
        "{}",
        world.reference.kind
    );
}

/// A `brokkr ui` child serving `world`'s journal on an ephemeral port. It is
/// killed when dropped, so a failed assertion leaves no server behind; its
/// stderr stays open for the child's lifetime.
struct Server {
    child: std::process::Child,
    _stderr: std::io::BufReader<std::process::ChildStderr>,
    port: u16,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Server {
    fn start(world: &World) -> Self {
        use std::io::BufRead;
        let mut child = Command::new(brokkr_bin())
            .args(["ui", "--port", "0", "--db"])
            .arg(&world.db)
            .env("HOME", &world.home)
            .current_dir(world.dir.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut stderr = std::io::BufReader::new(child.stderr.take().unwrap());
        // "brokkr ui: http://127.0.0.1:<port>/ (read-only; Ctrl-C to stop)"
        let mut line = String::new();
        let _ = stderr.read_line(&mut line);
        let port = line
            .split_once("127.0.0.1:")
            .and_then(|(_, rest)| rest.split_once('/'))
            .and_then(|(port, _)| port.parse().ok());
        let server = Server {
            child,
            _stderr: stderr,
            port: port.unwrap_or_default(),
        };
        assert!(port.is_some(), "no bound url: {line}");
        server
    }

    /// The raw response to one GET, split at the blank line.
    fn get(&self, path: &str) -> (String, Vec<u8>) {
        use std::io::{Read, Write};
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        write!(stream, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        let split = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("a head and a body");
        let head = String::from_utf8(response[..split].to_vec()).unwrap();
        (head, response[split + 4..].to_vec())
    }
}

/// Decision 0073 ruling 1 on the wire: `/api/transcript/<run>/<key>`,
/// served by `brokkr ui` over TCP, carries exactly the document `brokkr
/// transcript --json` prints, which ends in the one newline the command
/// adds, readable (Claude, 200) or refused (non-UTF-8 Codex, 404).
#[test]
fn the_wire_body_is_the_commands_stdout_without_its_newline() {
    let _env = EnvGuard::lock();
    let claude = make_world(
        "claude-session",
        "abcd-1234",
        "{\"type\":\"assistant\",\"message\":{\"content\":\"on the wire\"}}\n",
    );
    let codex = make_world("codex-thread", "0199wire", "");
    std::fs::write(&codex.path, [0xff, 0xfe]).unwrap();
    for (world, exit, status) in [(&claude, 0, "200 OK"), (&codex, 1, "404 Not Found")] {
        let stdout = command(world, &["--json"]);
        assert_eq!(stdout.status.code(), Some(exit), "{}", world.reference.kind);
        let (head, body) = Server::start(world).get("/api/transcript/r222/eff1");
        assert!(
            head.starts_with(&format!("HTTP/1.1 {status}\r\n")),
            "{head}"
        );
        assert_eq!([&body[..], b"\n"].concat(), stdout.stdout, "{head}");
        assert!(
            head.contains(&format!("\r\nContent-Length: {}\r\n", body.len())),
            "{head}"
        );
    }
}

/// Compare every surface against one shared read, for one source.
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn compare(world: &World, read: &TranscriptRead, selected: Option<usize>) {
    // No secrets store sits beside the world's journal, so every surface
    // that masks says it masked nothing and names where it looked (#380).
    let mut read = read.clone();
    if !read.turns.is_empty() {
        read.notices.push(format!(
            "secrets not masked: no values found in {}; a value bound from another store, or \
             removed since the run, is shown as written",
            world.dir.path().join("secrets.env").display()
        ));
    }
    let read = &read;

    // The browser participant presentation carries only the shared
    // metadata, never transcript prose.
    let response = brokkr_cli::handle(&world.db, "/api/presentation/r222/eff1");
    assert_eq!(response.status, "200 OK");
    let presentation: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(presentation["admitted"], read.is_readable());
    assert_eq!(presentation["path"], json!(read.path));
    assert_eq!(presentation["hint"], json!(read.full_session));
    assert_eq!(
        presentation["reason"],
        json!(read.unavailable.map(|reason| reason.as_str()))
    );
    assert!(presentation.get("turns").is_none(), "{presentation}");
    assert!(presentation.get("blocks").is_none(), "{presentation}");
    for turn in &read.turns {
        for block in &turn.blocks {
            // Only text long enough to BE evidence is searched for. A one- or
            // two-character block is a substring of almost any JSON document —
            // including the temporary path this response legitimately carries —
            // so asserting its absence tests the platform's mkdtemp, not the
            // transport. macOS and Windows failed here where Linux passed for
            // exactly that reason: their temp paths happened to contain the
            // letter.
            if block.text.chars().count() < 4 {
                continue;
            }
            assert!(
                !response.body.contains(&block.text),
                "the browser transport must not carry prose: {presentation}"
            );
        }
    }

    // The TUI pane enumerates the same turns; both doors carry the same
    // turn text, one-based numbering, notices and hint through the real
    // renderers.
    let (keys, selected_door, whole_door) =
        brokkr_cli::transcript_surfaces_for_test(read, selected);
    assert_eq!(keys.len(), read.turns.len());
    for index in 0..read.turns.len() {
        assert!(keys.contains(&index.to_string()));
    }
    for (index, turn) in read.turns.iter().enumerate() {
        let header = format!("#{} {}  {}", index + 1, turn.role, turn.ts);
        assert!(whole_door.contains(&header), "{whole_door}");
        for block in &turn.blocks {
            assert!(whole_door.contains(&block.text), "{whole_door}");
        }
    }
    for notice in &read.notices {
        assert!(whole_door.contains(notice), "{whole_door}");
    }
    if let Some(hint) = &read.full_session {
        assert!(whole_door.contains(hint), "{whole_door}");
    }
    if let Some(index) = selected.filter(|index| *index < read.turns.len()) {
        let turn = &read.turns[index];
        let header = format!("#{} {}  {}", index + 1, turn.role, turn.ts);
        assert!(selected_door.contains(&header), "{selected_door}");
        for block in &turn.blocks {
            assert!(selected_door.contains(&block.text), "{selected_door}");
        }
        for notice in &read.notices {
            assert!(selected_door.contains(notice), "{selected_door}");
        }
    }

    // The whole command agrees with the same fields.
    let whole = command(world, &["--json"]);
    assert!(
        whole.status.success(),
        "{}",
        String::from_utf8_lossy(&whole.stderr)
    );
    let document = parse(&whole);
    assert_eq!(
        document["turns"],
        serde_json::to_value(&read.turns).unwrap()
    );
    assert_eq!(document["truncated"], read.truncated);
    assert_eq!(document["skipped_lines"], read.skipped_lines);
    assert_eq!(document["unrecognized_records"], read.unrecognized_records);
    assert_eq!(
        document["notices"],
        serde_json::to_value(&read.notices).unwrap()
    );
    assert_eq!(document["path"], json!(read.path));
    assert_eq!(document["full_session"], json!(read.full_session));
    // A readable derivation is never an unavailable one.
    assert_eq!(document["unavailable"], Value::Null);
    same_bytes_in_the_browser(world, &whole, "200 OK");

    // The text command carries the same one-based numbering, roles,
    // stamps, blocks, notices and hint as the pane and the JSON face.
    let whole_text = command(world, &[]);
    assert!(
        whole_text.status.success(),
        "{}",
        String::from_utf8_lossy(&whole_text.stderr)
    );
    let rendered = String::from_utf8_lossy(&whole_text.stdout);
    for (index, turn) in read.turns.iter().enumerate() {
        let header = format!("turn {} · {} · {}", index + 1, turn.role, turn.ts);
        assert!(rendered.contains(header.trim_end()), "{rendered}");
        for block in &turn.blocks {
            assert!(rendered.contains(&block.text), "{rendered}");
        }
    }
    for notice in &read.notices {
        assert!(rendered.contains(notice), "{rendered}");
    }
    if let Some(hint) = &read.full_session {
        assert!(rendered.contains(hint), "{rendered}");
    }

    // The selected command changes only the turn and keeps the whole
    // read's numbering, metadata and notices; its text face carries only
    // the requested turn's prose.
    if let Some(index) = selected.filter(|index| *index < read.turns.len()) {
        let selected_command = command(world, &["--json", "--turn", &(index + 1).to_string()]);
        assert!(selected_command.status.success());
        let document = parse(&selected_command);
        assert_eq!(
            document["turns"],
            serde_json::to_value(vec![read.turns[index].clone()]).unwrap()
        );
        assert_eq!(document["turn"], index + 1);
        assert_eq!(document["truncated"], read.truncated);
        assert_eq!(
            document["notices"],
            serde_json::to_value(&read.notices).unwrap()
        );

        let selected_text = command(world, &["--turn", &(index + 1).to_string()]);
        assert!(selected_text.status.success());
        let rendered = String::from_utf8_lossy(&selected_text.stdout);
        let turn = &read.turns[index];
        let header = format!("turn {} · {} · {}", index + 1, turn.role, turn.ts);
        assert!(rendered.contains(header.trim_end()), "{rendered}");
        for block in &turn.blocks {
            assert!(rendered.contains(&block.text), "{rendered}");
        }
        for notice in &read.notices {
            assert!(rendered.contains(notice), "{rendered}");
        }
        for (other_index, other) in read.turns.iter().enumerate() {
            if other_index == index {
                continue;
            }
            for block in &other.blocks {
                if !turn.blocks.iter().any(|own| own.text == block.text) {
                    assert!(
                        !rendered.contains(&block.text),
                        "the selected read leaked turn {}: {rendered}",
                        other_index + 1
                    );
                }
            }
        }
    }
}

/// One unavailable read reaches the TUI seam and the command with the
/// same unavailability, path, hint, notices and counts — the refusal half
/// of the one-derivation proof (task 11.2).
fn compare_refusal(world: &World, read: &TranscriptRead, expected: &str) {
    assert_eq!(
        read.unavailable.map(|reason| reason.as_str()),
        Some(expected),
        "the shared read carries the expected refusal"
    );

    // The TUI seam: no turn key and no selected door, and the openable
    // whole-transcript explanation carries the retained notices and hint
    // without any refused source prose.
    let (keys, selected_door, whole_door) = brokkr_cli::transcript_surfaces_for_test(read, None);
    assert!(keys.is_empty(), "{expected}: a refusal has no turn keys");
    assert!(
        selected_door.is_empty(),
        "{expected}: a refusal has no selected door"
    );
    for notice in &read.notices {
        assert!(whole_door.contains(notice), "{expected}: {whole_door}");
    }
    if let Some(hint) = &read.full_session {
        assert!(whole_door.contains(hint), "{expected}: {whole_door}");
    }

    // The command's whole read agrees on every failure-state field.
    let whole = command(world, &["--json"]);
    assert!(
        !whole.status.success(),
        "{expected}: a refusal exits nonzero"
    );
    assert_eq!(whole.status.code(), Some(1), "{expected}: exact exit code");
    let document = parse(&whole);
    assert_eq!(document["unavailable"], expected);
    assert_eq!(document["turns"], json!([]));
    same_bytes_in_the_browser(world, &whole, "404 Not Found");
    assert_eq!(document["path"], json!(read.path));
    assert_eq!(document["full_session"], json!(read.full_session));
    assert_eq!(document["truncated"], read.truncated);
    assert_eq!(document["skipped_lines"], read.skipped_lines);
    assert_eq!(document["unrecognized_records"], read.unrecognized_records);
    assert_eq!(
        document["notices"],
        serde_json::to_value(&read.notices).unwrap()
    );

    // Text mode writes no body and names the same sanitized reason.
    let text = command(world, &[]);
    assert!(!text.status.success());
    assert_eq!(text.status.code(), Some(1), "{expected}: exact exit code");
    assert!(
        text.stdout.is_empty(),
        "{expected}: text refusals write no body"
    );
    let stderr = String::from_utf8_lossy(&text.stderr);
    assert!(stderr.contains(expected), "{expected}: {stderr:?}");
}

/// One synthetic source of each kind flows through every surface, plus a
/// readable zero-turn source, a truncated source, a counted omission and
/// each DSH refusal.
#[test]
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn one_derivation_reaches_every_surface() {
    let mut env = EnvGuard::lock();
    for (kind, locator, body) in [
        (
            "claude-session",
            "abcd-1234",
            "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"claude says hi\"}}\n\
             {\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"claude says bye\"}}\n",
        ),
        (
            "codex-thread",
            "0199mine",
            "{\"timestamp\":\"t1\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"codex asks\"}]}}\n\
             {\"timestamp\":\"t2\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"codex answers\"}]}}\n",
        ),
        (
            "dsh-session",
            "sessions/one",
            "{\"type\":\"session\",\"version\":0}\n\
             {\"type\":\"user/message\",\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"dsh asks\"}]},\"time\":1000}\n\
             {\"type\":\"assistant/message\",\"data\":{\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"dsh answers\"}]}},\"time\":2000}\n",
        ),
    ] {
        let world = make_world(kind, locator, body);
        // The in-process handler and the child process must agree on the
        // local projects root for the Claude drill.
        env.set("HOME", &world.home);
        let read = shared_read(&world);
        assert!(read.is_readable(), "{kind}: {read:?}");
        assert_eq!(read.path.as_deref(), Some(world.path.as_str()));
        let selected = (read.turns.len() >= 2).then_some(1);
        compare(&world, &read, selected);
    }

    // A readable zero-turn source.
    let world = make_world("codex-thread", "0199zero", "{\"type\":\"turn_context\"}\n");
    env.set("HOME", &world.home);
    let read = shared_read(&world);
    assert!(read.is_readable() && read.turns.is_empty());
    compare(&world, &read, None);

    // A truncated source: three retained turns, then one over the display
    // budget, and one counted unknown record.
    let huge = "x".repeat(4_000_001);
    let body = format!(
        "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"one\"}}}}\n\
         {{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"two\"}}}}\n\
         {{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"three\"}}}}\n\
         {{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"{huge}\"}}}}\n\
         {{\"type\":\"future-record\"}}\n"
    );
    let world = make_world("claude-session", "abcd-1234", &body);
    env.set("HOME", &world.home);
    let read = shared_read(&world);
    assert!(read.truncated && read.unrecognized_records == 1, "{read:?}");
    compare(&world, &read, Some(1));

    // A safe counted omission that still reads.
    let world = make_world(
        "dsh-session",
        "sessions/one",
        "{\"type\":\"session\",\"version\":0}\n\
         {\"type\":\"future/event\",\"ignorable\":true}\n",
    );
    env.set("HOME", &world.home);
    let read = shared_read(&world);
    assert!(
        read.is_readable() && read.unrecognized_records == 1,
        "{read:?}"
    );
    compare(&world, &read, None);

    // A DSH header-version refusal is a body-stage refusal: the browser
    // presentation's discovery-stage facts stay honest while no surface
    // invents prose, and the command refuses.
    let world = make_world(
        "dsh-session",
        "sessions/one",
        "{\"type\":\"session\",\"version\":1}\n\
         {\"type\":\"user/message\",\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"hidden\"}]}}\n",
    );
    env.set("HOME", &world.home);
    let read = shared_read(&world);
    assert_eq!(
        read.unavailable,
        Some(brokkr_view::transcript::Unavailable::UnsupportedFormat)
    );
    let response = brokkr_cli::handle(&world.db, "/api/presentation/r222/eff1");
    let presentation: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(presentation["admitted"], true, "discovery owns the file");
    assert_eq!(presentation["path"], json!(read.path));
    assert_eq!(presentation["hint"], json!(read.full_session));
    assert!(presentation["reason"].is_null());
    assert!(!response.body.contains("hidden"), "{presentation}");
    compare_refusal(&world, &read, "unsupported-format");

    // A DSH ownership refusal is a lookup refusal every surface shares.
    let world = make_world(
        "dsh-session",
        "sessions/one",
        "{\"type\":\"session\",\"delegationDepth\":1,\"version\":0}\n",
    );
    env.set("HOME", &world.home);
    let read = shared_read(&world);
    assert_eq!(
        read.unavailable,
        Some(brokkr_view::transcript::Unavailable::NotFound)
    );
    let response = brokkr_cli::handle(&world.db, "/api/presentation/r222/eff1");
    let presentation: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(presentation["admitted"], false);
    assert_eq!(presentation["reason"], "not-found");
    assert_eq!(
        presentation["explanation"],
        "no valid depth-zero DSH session header"
    );
    assert!(presentation["path"].is_null());
    assert!(presentation.get("turns").is_none(), "{presentation}");
    compare_refusal(&world, &read, "not-found");
}

/// Operator ruling 2026-09-27 (decision 0073 rulings 3 and 4): the browser
/// reads what the command reads. A Claude reference recorded under a home
/// that is not the local projects home gets the command's bytes.
#[test]
fn a_foreign_home_claude_reference_reads_the_same_bytes_in_the_browser() {
    let mut env = EnvGuard::lock();
    let world = make_world(
        "claude-session",
        "abcd-1234",
        "{\"type\":\"assistant\",\"message\":{\"content\":\"from another home\"}}\n",
    );
    let local = world.dir.path().join("local");
    std::fs::create_dir_all(local.join(".claude").join("projects")).unwrap();
    env.set("HOME", &local);
    let whole = command_under(&world, &local, &["--json"]);
    assert_eq!(whole.status.code(), Some(0), "the command reads it");
    assert_eq!(
        parse(&whole)["turns"][0]["blocks"][0]["text"],
        "from another home"
    );
    same_bytes_in_the_browser(&world, &whole, "200 OK");
}

/// The same ruling for a legacy flat id whose local projects home does not
/// exist: the command refuses the read with its document, and the browser
/// serves that document, not a refusal of its own.
#[test]
fn a_legacy_id_under_a_missing_projects_home_reads_the_same_bytes_in_the_browser() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    // No provider provenance and no common reference: the pre-provider
    // Claude era, whose flat id is synthesized under the local projects
    // home, which this home does not hold.
    let db = dir.path().join("forge.db");
    journal(
        &db,
        json!({"step": "session-started", "session_id": "abcd-1234"}),
    );
    let projects = home.join(".claude").join("projects");
    let world = World {
        reference: brokkr_view::Transcript {
            kind: "claude-session".into(),
            locator: "abcd-1234".into(),
            home: projects.to_str().unwrap().into(),
        },
        path: String::new(),
        dir,
        db,
        home,
    };
    env.set("HOME", &world.home);
    let whole = command(&world, &["--json"]);
    assert_eq!(whole.status.code(), Some(1), "the command refuses the read");
    let document = parse(&whole);
    assert_eq!(
        (&document["legacy"], &document["unavailable"]),
        (&json!(true), &json!("not-found"))
    );
    same_bytes_in_the_browser(&world, &whole, "404 Not Found");
}

/// A real packed DSH source reaches CLI text/JSON, the TUI pane and both
/// doors as the same coalesced chunks: consecutive text and reasoning
/// members become one chunk each at the first member's stamp, so packed and
/// ordinary indices deliberately differ (command delta).
#[test]
fn packed_dsh_coalescing_reaches_every_surface() {
    let mut env = EnvGuard::lock();
    let body = "{\"type\":\"session\",\"version\":0}\n\
        {\"type\":\"text-chunks\",\"seq0\":10,\"time0\":1000,\"data\":{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[1,1],\"texts\":[\"Prose-Alpha \",\"Prose-Bravo \",\"Prose-Charlie\"]}}\n\
        {\"type\":\"reasoning-chunks\",\"seq0\":20,\"time0\":2000,\"data\":{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[1],\"texts\":[\"Reason-Delta \",\"Reason-Echo\"]}}\n";
    let world = make_world("dsh-session", "sessions/one", body);
    env.set("HOME", &world.home);
    let read = shared_read(&world);
    assert!(read.is_readable(), "{read:?}");
    assert_eq!(read.turns.len(), 2);
    assert_eq!(
        read.turns[0].blocks[0].text,
        "Prose-Alpha Prose-Bravo Prose-Charlie"
    );
    assert_eq!(read.turns[0].ts, "1000");
    assert_eq!(read.turns[1].blocks[0].text, "Reason-Delta Reason-Echo");
    assert_eq!(read.turns[1].ts, "2000");
    compare(&world, &read, Some(0));
}

/// The shared structural charge is one more surface agreement: 10,000 tiny
/// `tool/call` rows with absent data retain exactly 7,797 one-byte turns,
/// and the CLI, its selected read, the TUI pane, both doors and the JSON
/// document all carry the same truncation notice.
#[test]
fn structural_cap_notices_reach_every_surface() {
    let mut env = EnvGuard::lock();
    let mut body = String::from("{\"type\":\"session\",\"version\":0}\n");
    for seq in 1..=10_000 {
        body.push_str(&format!(
            "{{\"type\":\"tool/call\",\"seq\":{seq},\"time\":1000,\"data\":{{}}}}\n"
        ));
    }
    let world = make_world("dsh-session", "sessions/one", &body);
    env.set("HOME", &world.home);
    let read = shared_read(&world);
    assert!(read.is_readable(), "{read:?}");
    assert_eq!(read.turns.len(), 7_797);
    assert!(read.truncated);
    assert_eq!(
        read.notices,
        vec!["transcript truncated (size cap)".to_string()]
    );
    compare(&world, &read, Some(7_796));
}

/// A DSH semantic refusal after a readable-looking prefix, and the
/// ambiguity between two safe roots, travel the same TUI seam as every
/// other refusal: no reading door, the retained counts/notices/hint and
/// the same command document.
#[test]
fn dsh_semantic_refusals_reach_the_tui_seam() {
    let mut env = EnvGuard::lock();
    // A DSH semantic refusal after a readable-looking prefix is the same
    // unavailability through every surface, with its counts and notices
    // intact and no refused prose anywhere.
    let world = make_world(
        "dsh-session",
        "sessions/one",
        "{\"type\":\"session\",\"version\":0}\n\
         not json\n\
         {\"type\":\"future/required\"}\n",
    );
    env.set("HOME", &world.home);
    let read = shared_read(&world);
    assert_eq!(
        read.unavailable,
        Some(brokkr_view::transcript::Unavailable::UnsupportedFormat)
    );
    assert_eq!(read.skipped_lines, 1, "{read:?}");
    assert_eq!(read.unrecognized_records, 1, "{read:?}");
    compare_refusal(&world, &read, "unsupported-format");

    // A present common reference with two safe roots of different
    // versions is ambiguous, not a preference for the supported one.
    let world = make_world(
        "dsh-session",
        "sessions/one",
        "{\"type\":\"session\",\"version\":0}\n",
    );
    let other = world.home.join("sessions/one/other/seat");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(
        other.join("session.jsonl"),
        "{\"type\":\"session\",\"version\":1}\n",
    )
    .unwrap();
    env.set("HOME", &world.home);
    let read = shared_read(&world);
    assert_eq!(
        read.unavailable,
        Some(brokkr_view::transcript::Unavailable::AmbiguousSource)
    );
    compare_refusal(&world, &read, "ambiguous-source");
}

/// The complete recorded tool identity and MCP/dynamic/DSH context reach
/// every surface in the one centralized block text (design D5/D11), for
/// every readable kind.
#[test]
fn recorded_tool_identity_and_context_reach_every_surface() {
    let _env = EnvGuard::lock();
    let cases: [(&str, &str, &str, Vec<&str>); 3] = [
        (
            "claude-session",
            "abcd-1234",
            "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"tool_use\",\"name\":\"Read\",\"input\":{\"file_path\":\"src/lib.rs\"}}]}}\n",
            vec!["Read · src/lib.rs"],
        ),
        (
            "codex-thread",
            "0199mine",
            concat!(
                "{\"timestamp\":\"t1\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"F\",\"call_id\":\"c1\",\"arguments\":\"{}\"}}\n",
                "{\"timestamp\":\"t2\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"F\",\"call_id\":\"c2\",\"arguments\":\"{}\"}}\n",
                "{\"timestamp\":\"t3\",\"type\":\"event_msg\",\"payload\":{\"type\":\"mcp_tool_call_begin\",\"call_id\":\"m1\",\"invocation\":{\"server\":\"srv\",\"tool\":\"search\",\"arguments\":\"{}\"}}}\n",
                "{\"timestamp\":\"t4\",\"type\":\"event_msg\",\"payload\":{\"type\":\"dynamic_tool_call_response\",\"call_id\":\"d1\",\"content_items\":[{\"type\":\"inputText\",\"text\":\"done\"}],\"error\":\"boom\"}}\n",
            ),
            vec!["[c1]", "[c2]", "srv", "search", "[m1]", "[d1]", "done", "boom"],
        ),
        (
            "dsh-session",
            "sessions/one",
            concat!(
                "{\"type\":\"session\",\"version\":0}\n",
                "{\"type\":\"tool/call\",\"data\":{\"name\":\"bash\",\"arguments\":{\"cmd\":\"ls\"},\"callId\":\"dshtool1\",\"turn\":1,\"step\":1},\"time\":1000}\n",
                "{\"type\":\"tool/result\",\"data\":{\"message\":{\"content\":[{\"type\":\"tool-result\",\"toolCallId\":\"dshtool1\",\"content\":\"tool output here\"}]},\"callId\":\"dshtool1\",\"turn\":1,\"step\":1},\"time\":2000}\n",
            ),
            vec!["bash", "dshtool1", "tool output here"],
        ),
    ];

    for (kind, locator, body, needles) in cases {
        let world = make_world(kind, locator, body);
        // The common reference needs no ambient HOME, but the in-process
        // browser route still resolves it, so this test holds the guard.
        let read = shared_read(&world);
        assert!(read.is_readable(), "{kind}: {read:?}");
        let texts: Vec<&str> = read
            .turns
            .iter()
            .flat_map(|turn| turn.blocks.iter().map(|block| block.text.as_str()))
            .collect();
        for needle in &needles {
            assert!(
                texts.iter().any(|text| text.contains(needle)),
                "{kind}: the centralized block text lost {needle}: {texts:?}"
            );
        }

        // The command's JSON document serializes that exact block text.
        let document = parse(&command(&world, &["--json"]));
        let serialized: Vec<String> = document["turns"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|turn| {
                turn["blocks"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|block| block["text"].as_str().unwrap().to_string())
            })
            .collect();
        for needle in &needles {
            assert!(
                serialized.iter().any(|text| text.contains(needle)),
                "{kind}: the command document lost {needle}: {serialized:?}"
            );
        }

        // The text face renders the same context.
        let text = command(&world, &[]);
        assert!(
            text.status.success(),
            "{kind}: {}",
            String::from_utf8_lossy(&text.stderr)
        );
        let rendered = String::from_utf8_lossy(&text.stdout);
        for needle in &needles {
            assert!(
                rendered.contains(needle),
                "{kind}: the text face lost {needle}: {rendered}"
            );
        }

        // The TUI pane and whole door carry the same centralized text.
        let (_, _, whole_door) = brokkr_cli::transcript_surfaces_for_test(&read, None);
        for needle in &needles {
            assert!(
                whole_door.contains(needle),
                "{kind}: the TUI door lost {needle}: {whole_door}"
            );
        }

        // The browser presentation transports no tool prose.
        let response = brokkr_cli::handle(&world.db, "/api/presentation/r222/eff1");
        for text in &texts {
            assert!(
                !response.body.contains(*text),
                "{kind}: the browser transport leaked {text:?}"
            );
        }
    }
}

/// R25: one completed portable-display hint is byte-identical across the
/// view result, the command's text and decoded JSON, the TUI pane and whole
/// door, the presentation response and the served page's text node — with no
/// surface reconstructing a fragment.
#[test]
fn r25_portable_hint_is_identical_across_every_surface() {
    let _env = EnvGuard::lock();
    let hostile = "home $(x) `t` ;a&b|c<d>e%f!g \"q\" \\ é😀";
    for (kind, locator, body) in [
        ("codex-thread", "0199mine", "{\"type\":\"turn_context\"}\n"),
        (
            "dsh-session",
            "sessions/one",
            "{\"type\":\"session\",\"version\":0}\n",
        ),
    ] {
        let world = make_world_named(kind, locator, body, hostile);
        // The common reference needs no ambient HOME, but the in-process
        // browser route still resolves it, so this test holds the guard.
        let read = shared_read(&world);
        assert!(read.is_readable(), "{kind}: {read:?}");
        let hint = read
            .full_session
            .clone()
            .expect("a confirmed path has a hint");
        for raw in ["$(", "`", ";", "&", "|", "<", ">", "%", "!", "é", "😀"] {
            assert!(
                !hint.contains(raw),
                "{kind}: {raw:?} survived raw in {hint:?}"
            );
        }
        assert!(hint.contains("\\u0020"), "{kind}: {hint}");
        assert!(hint.contains("\\ud83d\\ude00"), "{kind}: {hint}");

        // The presentation transports the exact decoded hint with no prose.
        let response = brokkr_cli::handle(&world.db, "/api/presentation/r222/eff1");
        assert_eq!(response.status, "200 OK");
        let presentation: Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(presentation["hint"], json!(read.full_session), "{kind}");
        assert!(presentation.get("turns").is_none(), "{presentation}");

        // The TUI pane and whole door carry the same completed value; the
        // command's text face does too; its decoded JSON recovers it exactly
        // while the raw document carries the doubled outer escaping.
        let (_, _, whole) = brokkr_cli::transcript_surfaces_for_test(&read, None);
        assert!(whole.contains(&hint), "{kind}: {whole}");
        let text = command(&world, &[]);
        assert!(text.status.success());
        let text = String::from_utf8_lossy(&text.stdout);
        assert!(text.contains(&hint), "{kind}: {text}");
        let json_output = command(&world, &["--json"]);
        let document = parse(&json_output);
        assert_eq!(document["full_session"], json!(read.full_session), "{kind}");
        let raw = String::from_utf8_lossy(&json_output.stdout);
        assert!(raw.contains("\\\\u0020"), "{kind}: {raw}");
        assert!(raw.contains("\\\\ud83d\\\\ude00"), "{kind}: {raw}");

        // The served page paints the transported hint through `el`'s
        // textContent; no route or page quotes a path/home fragment.
        let page = brokkr_cli::handle(&world.db, "/");
        assert!(
            page.body.contains("el('p', 'cause', view.hint)"),
            "the page paints the shared hint verbatim"
        );
    }
}

/// #380: a value the model echoed into its Codex rollout reaches
/// `brokkr transcript` as `[secret:NAME]`, masked against the store beside
/// the journal. A value rotated away since the run is no longer in the
/// store and stays as written, and the command says that masking covers
/// only the store's current values rather than passing it silently.
#[test]
fn the_command_masks_a_bound_value_and_states_what_a_rotation_leaves() {
    let body = "{\"timestamp\":\"t1\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"now ghp-bound-7f3a9c, before ghp-rotated-0c11\"}]}}\n";
    let world = make_world("codex-thread", "0199mine", body);
    brokkr_protocol::secret::store_set(
        &world.dir.path().join("secrets.env"),
        "GH_TOKEN",
        "ghp-bound-7f3a9c",
    )
    .unwrap();
    let notice = "secrets masked against the store's current values for GH_TOKEN; a value \
                  rotated or removed since the run is not masked";

    let json_face = command(&world, &["--json"]);
    assert!(json_face.status.success());
    let document = parse(&json_face);
    assert_eq!(
        document["turns"][0]["blocks"][0]["text"],
        "now [secret:GH_TOKEN], before ghp-rotated-0c11"
    );
    assert_eq!(document["notices"], json!([notice]));

    let text_face = command(&world, &[]);
    assert!(text_face.status.success());
    let rendered = String::from_utf8_lossy(&text_face.stdout);
    assert!(
        rendered.contains("  text: now [secret:GH_TOKEN], before ghp-rotated-0c11"),
        "{rendered}"
    );
    assert!(rendered.contains(&format!("notice {notice}")), "{rendered}");
    assert!(!rendered.contains("ghp-bound-7f3a9c"), "{rendered}");
}

/// #380: a projector renders a Codex shell action and a DSH tool call's
/// arguments as serialised JSON, so a bound value holding a quote and a
/// backslash reaches the text re-escaped. `brokkr transcript` masks that
/// escaped form too, never printing the value in either spelling.
#[test]
fn the_command_masks_a_bound_value_a_structured_call_reserialised() {
    let value = "tok\"en\\7f3a9c";
    let escaped = "tok\\\"en\\\\7f3a9c";
    let arguments = json!({"command": ["curl", "-H", value], "type": "exec"});
    let codex = format!(
        "{}\n",
        json!({"timestamp": "t1", "type": "response_item", "payload": {
            "type": "local_shell_call", "call_id": "c1", "action": arguments}})
    );
    let dsh = format!(
        "{}\n{}\n",
        json!({"type": "session", "version": 0}),
        json!({"type": "tool/call", "seq": 5, "time": 1001, "data": {
            "callId": "c1", "name": "Bash", "arguments": arguments, "turn": 1, "step": 1}})
    );
    let masked = "{\"command\":[\"curl\",\"-H\",\"[secret:GH_TOKEN]\"],\"type\":\"exec\"}";
    for (world, text) in [
        (
            make_world("codex-thread", "0199mine", &codex),
            format!("{masked} [c1]"),
        ),
        (
            make_world("dsh-session", "sessions/one", &dsh),
            format!("Bash {masked} [c1]"),
        ),
    ] {
        assert!(std::fs::read_to_string(&world.path)
            .unwrap()
            .contains(escaped));
        brokkr_protocol::secret::store_set(
            &world.dir.path().join("secrets.env"),
            "GH_TOKEN",
            value,
        )
        .unwrap();
        let json_face = command(&world, &["--json"]);
        assert!(json_face.status.success());
        let document = parse(&json_face);
        assert_eq!(document["turns"][0]["blocks"][0]["text"], text.as_str());
        let text_face = command(&world, &[]);
        assert!(text_face.status.success());
        let rendered = String::from_utf8_lossy(&text_face.stdout);
        assert!(rendered.contains(&format!("  tool: {text}")), "{rendered}");
        assert!(!rendered.contains("7f3a9c"), "{rendered}");
    }
}
