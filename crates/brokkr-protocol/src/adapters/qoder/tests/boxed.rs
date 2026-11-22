//! A boxed qoder seat (#1235): its launch, its refusal of a grant of its
//! own, and the recorded boxed turns of the second measurement folded as
//! the driver folds them. The recordings are read from the record when a
//! test runs, so no crate includes a file outside itself.

use super::*;
use crate::adapters::qoder::hands::{Breach, Seen};

/// The engine's grant of the box's one tool.
const GRANT: &str = "mcp__brokkr__workspace";

/// The box's hands, granted the engine's one tool.
fn boxed() -> Hands {
    Hands::Boxed(vec![GRANT.to_string()])
}

/// The recording `file` of the record's directory, read now.
fn record(file: &str) -> String {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/evidence/adapters/qoder-cli-1-1");
    std::fs::read_to_string(dir.join(file)).unwrap()
}

/// `stdout` folded under the box's hands on `pinned`, as the driver folds it.
fn folded(stdout: &str, pinned: &str) -> Folded {
    fold_under(&boxed(), stdout.as_bytes(), pinned, "", &[])
}

/// The breach `stdout` stops at under the box's hands on `Fast`, if any:
/// the typed failure, before it is rendered.
fn breach_of(stdout: &str) -> Option<Breach> {
    let mut fold = QoderFold::new(&pin("Fast"), &boxed());
    let mut transcript = Transcript::resolve(TranscriptKind::None).unwrap();
    for line in stdout.lines() {
        let read = fold.read(
            line.as_bytes(),
            &[],
            &mut Map::new(),
            &mut transcript,
            &mut |_| {},
        );
        match read {
            Ok(_) => {}
            Err(Stop::Breached(breach)) => return Some(breach),
            Err(_) => panic!("a stop that is no breach"),
        }
    }
    None
}

/// `stdout` with its init row rewritten by `change`.
fn with_init(stdout: &str, change: impl Fn(&mut Value)) -> String {
    let rows = stdout.lines().map(|line| {
        let mut row: Value = serde_json::from_str(line).unwrap();
        if row["subtype"] == "init" {
            change(&mut row);
        }
        row.to_string()
    });
    rows.collect::<Vec<_>>().join("\n")
}

/// A boxed launch is level 1's head, `dont_ask`, the seat's own controls,
/// the engine's grants and nothing else as the one `--allowed-tools`
/// value, the isolation in its one home, and `--tools ""`: no tool of
/// Qoder's own, no meta-tool denied (C4; `box-engine-workspace-tool`).
#[test]
fn a_boxed_launch_holds_the_engines_grant_and_no_tool_of_qoders_own() {
    let hands = boxed();
    let served = Engine {
        file: "/tmp/e/mcp.json",
        hands: &hands,
    };
    let seat = ["-m", "Fast", "--reasoning-effort", "low"];
    assert_eq!(
        launch("qodercli", &argv(&seat), served).unwrap(),
        Launch {
            command: argv(&[
                "qodercli",
                "-p",
                "--output-format",
                "stream-json",
                "--no-session-persistence",
                "--permission-mode",
                "dont_ask",
                "-m",
                "Fast",
                "--reasoning-effort",
                "low",
                "--allowed-tools",
                GRANT,
                "--strict-mcp-config",
                "--mcp-config",
                "/tmp/e/mcp.json",
                "--setting-sources",
                "",
                "--tools",
                "",
            ]),
            pin: pin("Fast"),
            hands: boxed(),
        }
    );
}

/// The unboxed and boxed launches of one seat differ in exactly what C4
/// names (C7): the permission mode, the engine's grant behind the seat's
/// controls, and the tool lists, Qoder's six tools and the meta-tools'
/// denial giving way to `--tools ""`. Everything else is level 1's, byte
/// for byte.
#[test]
fn the_unboxed_and_boxed_launches_differ_in_exactly_the_mode_the_grant_and_the_tools() {
    let seat = ["-m", "Qwen4-Max", "--reasoning-effort", "high"];
    let unboxed = command(&seat, EMPTY);
    let hands = boxed();
    let served = Engine {
        file: "/tmp/e/mcp.json",
        hands: &hands,
    };
    let mut expected = unboxed.clone();
    let mode = expected.iter().position(|part| part == "accept_edits");
    expected[mode.unwrap()] = "dont_ask".to_string();
    let behind_seat = LEAD.len() + seat.len();
    expected.splice(behind_seat..behind_seat, argv(&["--allowed-tools", GRANT]));
    let tools = expected.iter().position(|part| part == "--tools").unwrap();
    expected.splice(tools.., argv(&["--tools", ""]));
    assert_eq!(command(&seat, served), expected);
    // The unboxed seat's tail is level 1's, as its own golden pins it.
    assert_eq!(
        unboxed[unboxed.len() - 9..],
        argv(
            &[
                &ISOLATED[5..],
                &["--disallowed-tools", "mcp_call,mcp_get,mcp_list"][..]
            ]
            .concat()
        )
    );
}

/// A boxed seat that grants tools of its own is refused by the control,
/// never its value, wherever it writes it, before any doubled or valueless
/// reading of it (C4).
#[test]
fn a_boxed_seat_with_a_grant_of_its_own_is_refused_naming_the_control() {
    let hands = boxed();
    let served = Engine {
        file: "/tmp/e/mcp.json",
        hands: &hands,
    };
    for seat in [
        &["-m", "Fast", "--allowed-tools", "Bash(ls:*)"][..],
        &["--allowed-tools", "mcp__brokkr__workspace", "-m", "Fast"],
        &["-m", "Fast", "--allowed-tools"],
    ] {
        let refused = launch("qodercli", &argv(seat), served).unwrap_err();
        assert!(matches!(refused, LaunchError::BoxedGrant), "{refused:?}");
    }
    assert_eq!(
        LaunchError::BoxedGrant.to_string(),
        "refusing to invoke the agent CLI: the seat's arguments carry '--allowed-tools', and a \
         qoder seat whose hands are boxed holds no tool of Qoder's own to grant: its one grant \
         is the engine's workspace tool (#1235)"
    );
}

/// Serving inputs that seal the box's hands, measured cold, past the
/// lifted fence: the engine's set serves them, so the seat is boxed.
#[cfg(unix)]
fn boxed_serving() -> Value {
    use crate::native_controls::SealedBoundary::Namespace;
    sealed(Namespace, "hands", json!("measured"), "lifted")
}

/// Driven through `run_seat`, a boxed seat with a grant of its own ends as
/// the typed refusal's words, and the fake `qodercli` never starts.
#[cfg(unix)]
#[test]
fn a_driven_boxed_seat_with_a_grant_of_its_own_never_starts_its_harness() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_qoder(dir.path(), TOOL_LESS, "", None, 0);
    let extra = ["-m", "Fast", "--allowed-tools", "Bash(ls:*)"];
    let messages = run_sealed(dir.path(), &bin, &extra, Some(boxed_serving()));
    assert!(!dir.path().join("argv").exists(), "{messages:?}");
    let (status, error) = error_of(&messages).unwrap();
    assert_eq!(status, &crate::ResultStatus::Failed);
    assert_eq!(error, Some(LaunchError::BoxedGrant.to_string()));
}

/// Driven through `run_seat` on the box's sealed hands, the launch is the
/// boxed one, its file names the engine's hands server, and the recorded
/// boxed turn finishes on its pin (`box-engine-workspace-tool`).
#[cfg(unix)]
#[test]
fn a_driven_boxed_seat_launches_boxed_and_finishes_the_recorded_turn() {
    let dir = tempfile::tempdir().unwrap();
    let result = dir.path().join("result.json");
    let stdout = record("box-engine-workspace-tool.ndjson");
    let bin = fake_qoder(dir.path(), stdout, "", Some(&result), 0);
    let messages = run_sealed(dir.path(), &bin, &["-m", "Fast"], Some(boxed_serving()));
    let (status, refusal) = result_of(&messages).unwrap();
    assert_eq!((status, refusal), (&crate::ResultStatus::Succeeded, None));
    let launched = std::fs::read_to_string(dir.path().join("argv")).unwrap();
    let launched: Vec<&str> = launched.lines().collect();
    assert_eq!(launched[4..6], ["--permission-mode", "dont_ask"]);
    assert_eq!(launched[8..10], ["--allowed-tools", GRANT]);
    assert_eq!(launched[launched.len() - 2..], ["--tools", ""]);
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("mcp")).unwrap()).unwrap();
    let servers: Vec<&String> = file["mcpServers"].as_object().unwrap().keys().collect();
    assert_eq!(servers, ["brokkr"]);
}

/// The recorded boxed turns that reached the hands tool end as answers,
/// on Fast and on Qwen4-Max (`box-engine-workspace-tool`, its replacement,
/// `box-default-notice` and `-qwen4-max`).
#[test]
fn the_recorded_boxed_turns_that_reached_the_hands_tool_end_as_answers() {
    for (file, pinned) in [
        ("box-engine-workspace-tool.ndjson", "Fast"),
        ("box-engine-workspace-tool-replacement.ndjson", "Fast"),
        ("box-default-notice.ndjson", "Fast"),
        ("box-default-notice-qwen4-max.ndjson", "Qwen4-Max"),
    ] {
        let folded = folded(&record(file), pinned);
        assert_eq!(folded.outcome, Ok(None), "{file}");
        assert!(folded.fold.worked(), "{file}");
    }
}

/// A server init lists `disconnected` is no box (`box-server-does-not-start`):
/// the turn stops at init, before any of it is folded, naming the server
/// and the status it saw, though Qoder would run it to `success`.
#[test]
fn a_boxed_turn_whose_server_did_not_start_stops_at_init() {
    let stdout = record("box-server-does-not-start.ndjson");
    assert_eq!(
        breach_of(&stdout),
        Some(Breach::Unconnected {
            status: Seen::Plain("disconnected".into())
        })
    );
    let folded = folded(&stdout, "Fast");
    assert!(!folded.fold.worked());
    assert_eq!(
        folded.outcome.unwrap_err().words(),
        "box_unserved: Qoder's init lists the engine's server 'brokkr' with a status other than \
         connected: 'disconnected'; a boxed turn runs only with it connected \
         (box-server-does-not-start)"
    );
}

/// A call to a tool the box does not hold, Qoder's own (`box-other-natives`)
/// or an engine tool with no grant (`box-ungranted-mcp-tool`), is a denial
/// in Qoder's result, and the boxed turn fails on it. The same recordings
/// under level 1's hands end as answers, as level 1 folds a denial.
#[test]
fn a_boxed_turn_with_a_permission_denial_fails() {
    for file in ["box-other-natives.ndjson", "box-ungranted-mcp-tool.ndjson"] {
        let stdout = record(file);
        assert_eq!(breach_of(&stdout), Some(Breach::Denied), "{file}");
        let folded = folded(&stdout, "Fast");
        assert!(folded.fold.worked(), "{file}");
        assert_eq!(
            folded.outcome.unwrap_err().words(),
            "box_denied: Qoder's result records a permission denial: a boxed seat called a \
             tool it does not hold"
        );
        assert_eq!(fold(&stdout, "Fast").outcome, Ok(None), "{file}");
    }
}

/// A hands server that dies mid-turn (`box-server-dies`) shows only in
/// Qoder's words, a tool error saying the engine's server is not
/// connected: the boxed turn fails there.
#[test]
fn a_boxed_turn_whose_server_died_fails_at_qoders_words() {
    let stdout = record("box-server-dies.ndjson");
    assert_eq!(breach_of(&stdout), Some(Breach::Lost));
    let folded = folded(&stdout, "Fast");
    assert!(folded.fold.worked());
    assert_eq!(
        folded.outcome.unwrap_err().words(),
        "box_server_lost: Qoder reported mid-turn that the engine's server 'brokkr' is not \
         connected (box-server-dies)"
    );
}

/// Driven through `run_seat`, a boxed turn that fails behind its folded
/// turns ends lost with no result sent, though Qoder exited 0 with its
/// result written, as every late stop does.
#[cfg(unix)]
#[test]
fn a_driven_boxed_turn_that_fails_late_sends_no_result() {
    for file in ["box-server-dies.ndjson", "box-other-natives.ndjson"] {
        let dir = tempfile::tempdir().unwrap();
        let result = dir.path().join("result.json");
        let bin = fake_qoder(dir.path(), record(file), "", Some(&result), 0);
        let messages = run_sealed(dir.path(), &bin, &["-m", "Fast"], Some(boxed_serving()));
        assert!(result.exists(), "{file}");
        assert_eq!(result_of(&messages), None, "{file}: {messages:?}");
    }
}

/// An init that lists one tool of Qoder's own beside the meta-tools, as a
/// later Qoder that kept a native tool under `--tools ""` would, is no
/// box: the turn stops at init naming the tool. So are servers other than
/// the engine's alone, and an init whose lists are not lists.
#[test]
fn a_boxed_init_beside_a_native_tool_or_another_server_stops() {
    let recorded = record("box-engine-workspace-tool.ndjson");
    let native = with_init(&recorded, |init| {
        init["tools"].as_array_mut().unwrap().push(json!("Bash"));
    });
    assert_eq!(
        breach_of(&native),
        Some(Breach::Native {
            tool: Seen::Plain("Bash".into())
        })
    );
    let unplain = with_init(&recorded, |init| {
        init["tools"] = json!(["mcp_call", "-x y"])
    });
    assert_eq!(
        breach_of(&unplain),
        Some(Breach::Native {
            tool: Seen::Unplain
        })
    );
    let beside =
        |servers: Value| with_init(&recorded, |init| init["mcp_servers"] = servers.clone());
    let planted = json!([{"name": "brokkr", "status": "connected"},
                         {"name": "sentinel-user", "status": "connected"}]);
    let sentinel = Seen::Plain("sentinel-user".into());
    assert_eq!(
        breach_of(&beside(planted)),
        Some(Breach::Beside { server: sentinel })
    );
    let twice = json!([{"name": "brokkr", "status": "connected"},
                       {"name": "brokkr", "status": "connected"}]);
    let engine = Seen::Plain("brokkr".into());
    assert_eq!(
        breach_of(&beside(twice)),
        Some(Breach::Beside { server: engine })
    );
    assert_eq!(breach_of(&beside(json!([]))), Some(Breach::Unlisted));
    assert_eq!(breach_of(&beside(json!({}))), Some(Breach::Unread));
    let words = |breach: Breach| breach.refusal().words().to_string();
    assert_eq!(
        words(Breach::Native {
            tool: Seen::Plain("Bash".into())
        }),
        "box_native: Qoder's init lists a tool beside its MCP meta-tools: 'Bash'; a boxed seat \
         holds no tool of Qoder's own"
    );
    assert_eq!(
        words(Breach::Beside {
            server: Seen::Unplain
        }),
        "box_unserved: Qoder's init lists an MCP server beside the engine's server 'brokkr': a \
         value that is not a plain id; a boxed turn is served the engine's server alone"
    );
    assert_eq!(
        words(Breach::Unlisted),
        "box_unserved: Qoder's init does not list the engine's server 'brokkr', and a boxed turn \
         runs only with it connected (box-server-does-not-start)"
    );
    assert_eq!(
        words(Breach::Unread),
        "box_unserved: Qoder's init does not list its tools and MCP servers as every measured \
         Qoder stream does, so nothing shows the seat's hands are the box's"
    );
}
