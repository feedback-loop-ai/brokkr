//! A dsh seat's hands rows (#896): the measured set exactly, the bounded
//! reader's one shape for them, the version they bind to, the hands access
//! their sandbox mode follows, and a seat with no hands served as before.

use std::path::Path;

use serde_json::{json, Value};

use super::super::mcp::tests::{dispatched, reach};
use super::super::route_overlay;
use super::super::tests::{binding, dsh_version_shim};
use super::*;
use crate::env_guard::EnvGuard;
use crate::hands::{HandsSpec, SEALED_REACH};
use crate::native_controls::{SealedBoundary, SealedDialect, ARTIFACT_ROOT, SERVING_INPUTS};

/// The full text an unreadable composed dsh tree prints.
const UNREADABLE: &str =
    "refusing to invoke the dsh driver: the composed dsh tree cannot be read as \
                          the one sequence of id and package rows dsh writes, so the seat is \
                          admitted on no measured composition (#896 D1)";

/// The measured overlay of #896, as an independent literal: the ten tool
/// rows the controller disabled on 0.2.1-rc.2 and rc.3, in its order, and
/// the sandbox row up to its mode.
const MEASURED_HEAD: &str = "\
# Written by `brokkr driver dsh` for one seat whose office declares hands:
# every effectful tool row disabled, dsh's own sandbox under the box,
# and Brokkr's workspace server as the seat's only effectful tool.
- id: tool-bash
  disabled: true
- id: tool-pwsh
  disabled: true
- id: tool-jobs
  disabled: true
- id: tool-fs
  disabled: true
- id: tool-fs-search
  disabled: true
- id: tool-skill
  disabled: true
- id: tool-subagent
  disabled: true
- id: tool-subagent-fork
  disabled: true
- id: tool-workflow
  disabled: true
- id: tool-web
  disabled: true
- id: sandbox-policy
  config:
    mode: ";

/// Brokkr's workspace server as the engine's `dsh-mcp-client` row, for
/// `command` and `args`.
fn server_row(command: &str, args: &[String]) -> String {
    let quoted = |value: &str| value.replace('\'', "''");
    let mut row = format!(
        "- insert:\n    - id: brokkr-hands\n      name: '@deepseek-ai/dsh-mcp-client'\n      \
         config:\n        serverName: brokkr\n        transport: stdio\n        \
         failOnStartupError: true\n        command: '{}'\n        args:\n",
        quoted(command)
    );
    for arg in args {
        row.push_str(&format!("          - '{}'\n", quoted(arg)));
    }
    row
}

fn s(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

/// A driver input as dispatch seals it for an office whose hands stand
/// under `stands`, the hands access marked `access` where one is.
fn sealed(stands: SealedBoundary, access: Option<&str>) -> Value {
    let dialect = SealedDialect {
        stands: Some(stands),
        ..Default::default()
    };
    let inputs = dispatched(dialect, Vec::new(), Some(HandsSpec::default()));
    let mut input = json!({"workdir": "/w", ARTIFACT_ROOT: "/a", SERVING_INPUTS: inputs});
    input[SEALED_REACH] = serde_json::to_value(reach()).unwrap();
    if let Some(access) = access {
        input[ACCESS] = json!(access);
    }
    input
}

/// The `hands serve` arguments the final check binds the box's hands to
/// in `workdir`, for the inputs [`sealed`] writes.
fn bound(workdir: &str) -> Vec<String> {
    let spec = HandsSpec::default();
    crate::hands::serve_args(Path::new(workdir), Path::new("/a"), &reach(), &spec)
}

fn refused(rows: &str) -> String {
    route_overlay::hands(rows.as_bytes())
        .expect_err("refused")
        .to_string()
}

/// The rows are the measured overlay exactly, under each mode, and the
/// server row binds the command and arguments it is handed, each quoted.
#[test]
fn the_hands_rows_are_the_measured_overlay_and_the_workspace_server() {
    let args = s(&["hands", "serve", "--workdir", "/w'it"]);
    for (mode, word) in [
        (Mode::ReadOnly, "read-only"),
        (Mode::WorkspaceWrite, "workspace-write"),
    ] {
        assert_eq!(
            hands_rows(Release::V0_2_1, mode, "/bin/brokkr", &args).unwrap(),
            format!(
                "{MEASURED_HEAD}{word}\n{}",
                server_row("/bin/brokkr", &args)
            ),
            "{mode:?}"
        );
    }
    assert_eq!(
        hands_rows(
            Release::V0_2_1,
            Mode::ReadOnly,
            "/bin/brokkr",
            &s(&["a\nb"])
        ),
        Err(Refusal::Multiline)
    );
    assert_eq!(
        hands_rows(Release::V0_2_1, Mode::ReadOnly, "/bin/\rbrokkr", &[]),
        Err(Refusal::Multiline)
    );
    assert_eq!(
        Refusal::Multiline.to_string(),
        "refusing to invoke the dsh driver: the box's hands server path spans more than one \
         line, so it cannot be written as one overlay scalar"
    );
}

/// The hands identity has one spelling: the row the overlay writer emits,
/// the id the composed tree's judge accepts, the row dsh's startup report
/// recognises, and the package beside it are all the same two constants,
/// and the measured dump spells them the same way.
#[test]
fn the_writer_the_composition_and_the_report_spell_one_hands_identity() {
    use super::composition::{judge, Row, MEASURED_COMPOSITION};
    use super::report::{checked, ReportRefusal};

    assert!(MEASURED_COMPOSITION.contains(&(HANDS_ID, MCP_CLIENT)));
    assert_eq!(HANDS_ID, "brokkr-hands");
    assert_eq!(MCP_CLIENT, "@deepseek-ai/dsh-mcp-client");

    let rows = hands_rows(
        Release::V0_2_1,
        Mode::ReadOnly,
        "/bin/brokkr",
        &s(&["hands"]),
    )
    .unwrap();
    assert!(rows.contains(&format!("- id: {HANDS_ID}\n")), "{rows}");
    assert!(rows.contains(&format!("name: '{MCP_CLIENT}'\n")), "{rows}");

    let row = Row {
        id: HANDS_ID.to_string(),
        name: MCP_CLIENT.to_string(),
        disabled: false,
    };
    assert_eq!(judge(Release::V0_2_1, &[row]), Ok(()));
    assert_eq!(
        checked(&format!(
            "{HANDS_ID} ({MCP_CLIENT}): Error: initial connection or tool synchronization failed\n"
        )),
        Err(ReportRefusal::NotActivated)
    );
}

/// The bounded reader admits the rows in their one shape and refuses every
/// other row, a repeat, and a measured row left out, each by name.
#[test]
fn the_reader_admits_the_hands_rows_alone_and_names_what_it_refuses() {
    let head = |mode: &str| format!("{MEASURED_HEAD}{mode}\n");
    for mode in ["read-only", "workspace-write"] {
        assert_eq!(route_overlay::hands(head(mode).as_bytes()), Ok(()));
    }
    let prefix = "refusing to invoke the dsh driver: ";
    let shape = |row: &str| {
        format!("{prefix}the hands rows carry row '{row}' in a shape the measured hands set does not name")
    };
    let cases = [
        (
            format!("{}- id: tool-mcp\n  disabled: true\n", head("read-only")),
            shape("tool-mcp"),
        ),
        (
            head("read-only").replacen("disabled: true", "disabled: false", 1),
            shape("tool-bash"),
        ),
        (
            head("read-only").replacen("  disabled: true\n", "  disabled: true\n  x: y\n", 1),
            shape("tool-bash"),
        ),
        (head("danger-full-access"), shape("sandbox-policy")),
        (
            head("read-only").replace("    mode:", "    mood:"),
            shape("sandbox-policy"),
        ),
        (
            "- id: tool-bash\n\tdisabled: true\n".to_string(),
            format!("{prefix}route overlay line 2 carries a tab or control character"),
        ),
        (
            "- id: tool-bash\n      disabled: true\n".to_string(),
            format!("{prefix}route overlay line 2 has an unexpected indentation"),
        ),
        (
            format!("{}    x: y\n", head("read-only")),
            shape("sandbox-policy"),
        ),
        (
            head("read-only").replace("- id: sandbox-policy", "- id: tool-web"),
            shape("tool-web"),
        ),
        (
            format!("{}- id: tool-fs\n  disabled: true\n", head("read-only")),
            format!("{prefix}the hands rows name row 'tool-fs' twice"),
        ),
        (
            format!("{}- insert: x\n", head("read-only")),
            format!("{prefix}a hands row is missing `id`"),
        ),
        (
            "id: tool-bash\n".to_string(),
            format!("{prefix}route overlay must be a sequence of one entry"),
        ),
    ];
    for (rows, text) in cases {
        assert_eq!(refused(&rows), text, "{rows}");
    }
    assert_eq!(
        route_overlay::hands(b"\xff").unwrap_err().to_string(),
        format!("{prefix}route overlay is not UTF-8")
    );
    for dropped in DISABLED_ROWS.into_iter().chain([SANDBOX_POLICY]) {
        let rows = without(dropped);
        assert_eq!(
            refused(&rows),
            format!(
                "{prefix}the hands rows carry no row '{dropped}', which the measured hands set \
                 names"
            ),
            "{rows}"
        );
    }
}

/// The hands rows with the row `dropped` left out.
fn without(dropped: &str) -> String {
    let mut rows = String::new();
    for row in DISABLED_ROWS.into_iter().filter(|row| *row != dropped) {
        rows.push_str(&format!("- id: {row}\n  disabled: true\n"));
    }
    if dropped != SANDBOX_POLICY {
        rows.push_str("- id: sandbox-policy\n  config:\n    mode: read-only\n");
    }
    rows
}

/// The drift check binds the rows to the releases they were measured on,
/// each matched whole: any other release, or none readable, is refused by
/// name, and the refusal names every measured release.
#[test]
fn only_a_measured_release_is_served_the_hands_rows() {
    // Each measured version names its own release exactly: rc.3 is not
    // classified by the position it happens to hold in the table.
    for (version, release) in [
        ("0.2.1-rc.2", Release::V0_2_1),
        ("0.2.1-rc.3", Release::V0_2_1),
        ("0.2.2-rc.1", Release::V0_2_2),
    ] {
        assert_eq!(measured(Some(version.into())), Ok(release), "{version}");
    }
    // Every version the refusal names is named by one variant: a version
    // appended to the table alone is measured nowhere, never inherited by
    // the variant it happens to sit beside.
    for version in MEASURED_VERSIONS {
        assert!(Release::named(version).is_some(), "{version}");
    }
    let tail = " is not a dsh release its hands rows were measured on (0.2.1-rc.2, \
         0.2.1-rc.3, 0.2.2-rc.1), and a tool row nobody measured could stay enabled beside the \
         box, so the seat is not served its native tools (#896)";
    let other = measured(Some("0.2.1-rc.4".into())).unwrap_err();
    assert_eq!(
        other.to_string(),
        format!("refusing to invoke the dsh driver: dsh '0.2.1-rc.4'{tail}")
    );
    // Exact, not a prefix: the release candidate must be named whole.
    assert_eq!(
        measured(Some("0.2.2".into())).unwrap_err().to_string(),
        format!("refusing to invoke the dsh driver: dsh '0.2.2'{tail}")
    );
    assert_eq!(
        measured(None).unwrap_err().to_string(),
        format!("refusing to invoke the dsh driver: an unreadable dsh version{tail}")
    );
}

/// The programmatic tool-calling executor is pinned disabled on 0.2.2-rc.1,
/// and is not a 0.2.1 row at all, so that release's overlay never names it.
#[test]
fn the_0_2_2_overlay_disables_the_programmatic_tool_row() {
    let args = s(&["hands"]);
    let newer = hands_rows(Release::V0_2_2, Mode::ReadOnly, "/bin/brokkr", &args).unwrap();
    assert!(
        newer.contains("- id: tool-executor-ptc\n  disabled: true\n"),
        "{newer}"
    );
    let older = hands_rows(Release::V0_2_1, Mode::ReadOnly, "/bin/brokkr", &args).unwrap();
    assert!(!older.contains(PTC_ROW), "{older}");
    assert!(disabled(Release::V0_2_2, PTC_ROW));
    assert!(!disabled(Release::V0_2_1, PTC_ROW));
}

/// 0.2.2-rc.1 is a measured release, and a seat on it is served its own
/// overlay: the ten common rows, the sandbox, the pinned programmatic
/// executor, and Brokkr's server.
#[cfg(unix)]
#[test]
fn a_0_2_2_release_is_measured_and_its_seat_is_served_its_rows() {
    assert_eq!(measured(Some("0.2.2-rc.1".into())), Ok(Release::V0_2_2));
    let dir = tempfile::tempdir().unwrap();
    let workdir = dir.path().to_str().unwrap();
    let facts = GitFacts::default();
    let dsh = dsh_version_shim(dir.path(), "dsh", "0.2.2-rc.1");
    let brokkr = std::env::current_exe().unwrap();
    let brokkr = brokkr.to_str().unwrap();
    let input = sealed(SealedBoundary::Namespace, Some("read-only"));
    let selection = sandbox(dsh.to_str().unwrap(), workdir, &facts, &input).unwrap();
    let server = server_row(brokkr, &bound(workdir));
    assert_eq!(
        selection.rows(),
        Some(
            format!(
                "{MEASURED_HEAD}read-only\n- id: tool-executor-ptc\n  disabled: true\n{server}"
            )
            .as_str()
        )
    );
    let (staged, mode) = selection.into_parts();
    assert!(staged.is_none());
    assert_eq!(mode, Some(Mode::ReadOnly));
}

/// A read-only office runs dsh's sandbox read-only and a writing office
/// workspace-write; an access the input does not name is refused.
#[test]
fn the_offices_access_selects_the_sandbox_mode() {
    let access = |value: Value| Mode::of(&json!({ACCESS: value}));
    assert_eq!(access(json!("read-only")), Ok(Mode::ReadOnly));
    assert_eq!(access(json!("writes")), Ok(Mode::WorkspaceWrite));
    for other in [json!("gate"), json!("work"), json!(1), Value::Null] {
        assert_eq!(access(other), Err(Refusal::NoAccess));
    }
    assert_eq!(Mode::of(&json!({})), Err(Refusal::NoAccess));
    assert_eq!(
        Refusal::NoAccess.to_string(),
        "refusing to invoke the dsh driver: the engine names no hands access for a seat whose \
         office declares hands, so dsh's own sandbox mode cannot be chosen (#896)"
    );
    assert_eq!(Mode::named("danger-full-access"), None);
}

/// A seat whose office declares boxed hands is served the hands rows and
/// stages no runner; one with no hands, unboxed hands or no sealed inputs
/// is served as before, and the hands are never probed for.
#[cfg(unix)]
#[test]
fn a_boxed_seat_is_served_the_hands_rows_and_any_other_as_before() {
    let dir = tempfile::tempdir().unwrap();
    let workdir = dir.path().to_str().unwrap();
    let facts = GitFacts::default();
    let dsh = dsh_version_shim(dir.path(), "dsh", "0.2.1-rc.2");
    let dsh = dsh.to_str().unwrap();
    let brokkr = std::env::current_exe().unwrap();
    let brokkr = brokkr.to_str().unwrap();
    for (access, mode, word) in [
        ("read-only", Mode::ReadOnly, "read-only"),
        ("writes", Mode::WorkspaceWrite, "workspace-write"),
    ] {
        let input = sealed(SealedBoundary::Namespace, Some(access));
        let selection = sandbox(dsh, workdir, &facts, &input).unwrap();
        let server = server_row(brokkr, &bound(workdir));
        assert_eq!(
            selection.rows(),
            Some(format!("{MEASURED_HEAD}{word}\n{server}").as_str())
        );
        let (staged, selected) = selection.into_parts();
        assert!(staged.is_none(), "{access}");
        assert_eq!(selected, Some(mode), "{access}");
    }
    let unprobed = "/nonexistent/dsh";
    for input in [
        json!({"workdir": workdir}),
        sealed(SealedBoundary::Harness, Some("read-only")),
        sealed(SealedBoundary::Open, Some("writes")),
        json!({SERVING_INPUTS: dispatched(SealedDialect::default(), Vec::new(), None)}),
    ] {
        let selection = sandbox(unprobed, workdir, &facts, &input).unwrap();
        assert!(selection.rows().is_none(), "{input}");
        let (staged, mode) = selection.into_parts();
        assert!(staged.is_none() && mode.is_none(), "{input}");
    }
}

/// A boxed seat is refused on an unmeasured release, without a seat
/// access, or with sealed inputs that cannot be read, before any row is
/// written.
#[cfg(unix)]
#[test]
fn a_boxed_seat_is_refused_before_its_rows_are_written() {
    let dir = tempfile::tempdir().unwrap();
    let workdir = dir.path().to_str().unwrap();
    let facts = GitFacts::default();
    let newer = dsh_version_shim(dir.path(), "newer", "0.2.2");
    let boxed = sealed(SealedBoundary::Seatbelt, Some("read-only"));
    let refused = |bin: &str, input: &Value| sandbox(bin, workdir, &facts, input).err();
    assert_eq!(
        refused(newer.to_str().unwrap(), &boxed),
        Some(Refusal::Unmeasured {
            observed: "dsh '0.2.2'".into()
        })
    );
    let measured = dsh_version_shim(dir.path(), "dsh", "0.2.1-rc.2");
    let measured = measured.to_str().unwrap();
    let spanning = sandbox(measured, "/w\nx", &facts, &boxed).err();
    assert_eq!(spanning, Some(Refusal::Multiline));
    let accessless = sealed(SealedBoundary::Container, None);
    assert_eq!(
        refused("/nonexistent/dsh", &accessless),
        Some(Refusal::NoAccess)
    );
    let unreadable = json!({SERVING_INPUTS: "garbage"});
    let refusal = refused("/nonexistent/dsh", &unreadable).unwrap();
    assert_eq!(refusal, Refusal::Unbound(McpRefusal::Unreadable));
    assert_eq!(
        refusal.to_string(),
        "refusing to invoke the agent CLI: the engine's MCP isolation intent cannot be read, so \
         no strict MCP configuration is built"
    );
    assert_eq!(
        String::from(Refusal::NoAccess),
        Refusal::NoAccess.to_string()
    );
}

/// The launch itself: a boxed seat's one overlay is its transcript row
/// and then the hands rows, with no runner staged; a seat with no hands
/// is launched with its transcript row and nothing beside it.
#[cfg(unix)]
#[test]
fn a_dsh_launch_carries_the_hands_rows_only_where_its_office_declares_them() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    env.set("DSH_HOME", dir.path());
    let workdir = dir.path().to_str().unwrap();
    let dsh = dsh_version_shim(dir.path(), "dsh", "0.2.1-rc.3");
    let launch = |input: &Value| {
        super::super::dsh_launch_with(dsh.to_str().unwrap(), &[], workdir, None, input, |_| {
            panic!("a cold seat recomputes no composite")
        })
    };
    let brokkr = std::env::current_exe().unwrap();
    let server = server_row(&brokkr.to_string_lossy(), &bound(workdir));
    let boxed = launch(&sealed(SealedBoundary::Namespace, Some("read-only"))).unwrap();
    assert!(boxed.staged.is_none());
    assert_eq!(
        std::fs::read_to_string(boxed.overlay.path()).unwrap(),
        format!(
            "{}{MEASURED_HEAD}read-only\n{server}",
            super::super::dsh_transcript_row(&boxed.root).unwrap()
        )
    );
    let plain = launch(&json!({"workdir": workdir})).unwrap();
    assert_eq!(
        std::fs::read_to_string(plain.overlay.path()).unwrap(),
        super::super::dsh_transcript_row(&plain.root).unwrap()
    );
    let unmeasured = dsh_version_shim(dir.path(), "old", "0.1.5-rc.1");
    let refused = super::super::dsh_launch_with(
        unmeasured.to_str().unwrap(),
        &[],
        workdir,
        None,
        &sealed(SealedBoundary::Namespace, Some("writes")),
        |_| panic!("refused before any composite"),
    )
    .err()
    .unwrap();
    let unmeasured = Refusal::Unmeasured {
        observed: "dsh '0.1.5-rc.1'".into(),
    };
    assert_eq!(refused, unmeasured.to_string());
}

/// The row the driver writes is the measured stdio row: a server without
/// `transport` never started on 0.2.1-rc.2 and dsh said nothing.
#[test]
fn the_workspace_server_row_carries_the_measured_transport_and_startup_policy() {
    let rows = hands_rows(
        Release::V0_2_1,
        Mode::ReadOnly,
        "/bin/brokkr",
        &s(&["hands"]),
    )
    .unwrap();
    let server = rows
        .split("- insert:")
        .nth(1)
        .expect("the insert row is written");
    assert!(server.contains("transport: stdio"), "{server}");
    assert!(server.contains("failOnStartupError: true"), "{server}");
}

/// A complete stderr capture of `raw`, through the one capture loop.
fn captured(raw: Vec<u8>) -> super::super::stderr::CapturedStderr {
    super::super::stderr::capture_reader(std::io::Cursor::new(raw)).take(false)
}

/// dsh's measured startup report: an entry that did not activate, or a
/// sandbox that cannot confine, fails the seat closed; the launcher's
/// partial line is carried, and a clean start adds nothing.
#[test]
fn the_dsh_startup_report_fails_closed_and_carries_partial() {
    use super::report::{checked, Enforcement, ReportRefusal};
    let warning = "dsh: warning: 1 entry did not activate\n\
                   brokkr-hands (@deepseek-ai/dsh-mcp-client): Error: mcp-client(probe): initial \
                   connection or tool synchronization failed\n";
    assert_eq!(checked(warning), Err(ReportRefusal::NotActivated));
    assert_eq!(
        checked(
            "brokkr-hands (@deepseek-ai/dsh-mcp-client): Error: mcp-client(probe): initial \
                 connection or tool synchronization failed\n"
        ),
        Err(ReportRefusal::NotActivated)
    );
    assert_eq!(
        checked(
            "Error: sandbox mode \"read-only\" is requested but no sandbox backend is usable \
                 on this host; refusing to run the command unconfined.\n"
        ),
        Err(ReportRefusal::SandboxUnavailable)
    );
    assert_eq!(
        checked("HarnessError: SANDBOX_UNAVAILABLE\n"),
        Err(ReportRefusal::SandboxUnavailable)
    );
    assert_eq!(
        checked("landlock-run: partial enforcement (older Landlock ABI)\n"),
        Ok(Enforcement::Partial)
    );
    let mut meta = serde_json::Map::new();
    assert_eq!(
        startup_gate(
            startup_report(&captured(
                b"landlock-run: partial enforcement (older Landlock ABI)\n".to_vec(),
            )),
            &mut meta,
            Mode::ReadOnly
        ),
        Ok(())
    );
    assert_eq!(meta["sandbox"], json!("read-only"));
    assert_eq!(meta["step_name"], json!(report::PARTIAL));
    assert_eq!(
        startup_gate(
            startup_report(&captured(warning.as_bytes().to_vec())),
            &mut serde_json::Map::new(),
            Mode::WorkspaceWrite
        ),
        Err(ReportRefusal::NotActivated)
    );
    assert_eq!(checked("dsh: done\n"), Ok(Enforcement::Full));
    assert_eq!(
        checked("dsh: warning: 0 entries did not activate\n"),
        Ok(Enforcement::Full)
    );
    assert_eq!(
        checked("dsh: warning: 1\n"),
        Ok(Enforcement::Full),
        "a warning with no space after its count is no activation refusal"
    );
    // What the model reasons is session content, not dsh's startup report:
    // a marker word there cannot refuse a healthy seat (#896 M1).
    let reasoning = "dsh: reasoning:\n\
                     I should explain what SANDBOX_UNAVAILABLE means: no sandbox backend is \
                     usable on this host; refusing to run the command unconfined.\n\
                     brokkr-hands (@deepseek-ai/dsh-mcp-client): Error: initial connection or \
                     tool synchronization failed\ndsh: done\n";
    assert_eq!(checked(reasoning), Ok(Enforcement::Full));
    // A startup report before that block is still read and still refuses.
    assert_eq!(
        checked(&format!(
            "dsh: warning: 1 entry did not activate\n{reasoning}"
        )),
        Err(ReportRefusal::NotActivated)
    );
    assert_eq!(
        ReportRefusal::NotActivated.to_string(),
        "refusing the dsh seat: the hands row 'brokkr-hands' did not activate, so the seat's one \
         effectful tool was never served; dsh ran the session anyway (#896 D0)"
    );
    assert_eq!(
        ReportRefusal::SandboxUnavailable.to_string(),
        "refusing the dsh seat: dsh reported SANDBOX_UNAVAILABLE (no sandbox backend is usable on \
         this host), so a call it could not confine is never admitted (#896 D4)"
    );
}

/// One prepared step in the test reader below.
enum Step {
    Data(&'static [u8]),
    Error(std::io::ErrorKind),
}

/// A stderr seam that returns prepared steps, one read call each, so a
/// capture can meet an interrupted and a failed read without a real pipe.
struct ReadSteps(std::collections::VecDeque<Step>);

impl ReadSteps {
    fn new(steps: impl IntoIterator<Item = Step>) -> Self {
        Self(steps.into_iter().collect())
    }
}

impl std::io::Read for ReadSteps {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self.0.pop_front() {
            Some(Step::Data(bytes)) => {
                buffer[..bytes.len()].copy_from_slice(bytes);
                Ok(bytes.len())
            }
            Some(Step::Error(kind)) => Err(std::io::Error::new(kind, "stderr seam failed")),
            None => Ok(0),
        }
    }
}

/// Drive one seam capture through `finish_dsh` and hand back what the
/// startup gate reads beside the evidence stderr.
fn captured_run(steps: impl IntoIterator<Item = Step>) -> (Startup, String) {
    let capture = super::super::stderr::capture_reader(ReadSteps::new(steps));
    let run = super::super::finish_dsh(
        &mut serde_json::Map::new(),
        0,
        capture.take(false),
        None,
        &[],
    )
    .unwrap();
    (run.startup, run.invocation.stderr)
}

/// #430 H1 (decision 0098 ruling 4; decision 0071 rulings 3 and 9): an
/// interrupted stderr read is retried, not taken for EOF, so the startup
/// warning behind it still refuses the seat.
#[test]
fn review_an_interrupted_stderr_read_must_not_erase_dsh_startup_refusal() {
    let (startup, _) = captured_run([
        Step::Error(std::io::ErrorKind::Interrupted),
        Step::Data(b"dsh: warning: 1 entry did not activate\n"),
    ]);
    assert_eq!(
        startup_gate(startup, &mut serde_json::Map::new(), Mode::ReadOnly),
        Err(ReportRefusal::NotActivated)
    );
}

/// The fourth ending at the gate (430s4a6 R1): an interrupted read retried
/// into a clean, complete capture lets dsh start. The retry neither refuses
/// a clean start nor hides a warning (the case above).
#[test]
fn chief_review_interrupted_then_complete_stderr_must_allow_dsh_startup() {
    let (startup, _) = captured_run([
        Step::Error(std::io::ErrorKind::Interrupted),
        Step::Data(b"dsh: done\n"),
    ]);
    assert_eq!(
        startup_gate(startup, &mut serde_json::Map::new(), Mode::ReadOnly),
        Ok(())
    );
}

/// #430 H1: a non-interrupted read error ends the capture UNREADABLE, not
/// at EOF; dsh's start refuses, naming the error, even though the warning
/// the seam held next was never returned.
#[test]
fn review_an_unreadable_stderr_capture_must_refuse_dsh_startup() {
    let (startup, stderr) = captured_run([
        Step::Error(std::io::ErrorKind::PermissionDenied),
        Step::Data(b"dsh: warning: 1 entry did not activate\n"),
    ]);
    let refusal = startup_gate(startup, &mut serde_json::Map::new(), Mode::ReadOnly)
        .expect_err("an unreadable capture never passes dsh's startup gate");
    assert!(matches!(refusal, ReportRefusal::UnreadableCapture(_)));
    assert_eq!(
        refusal.to_string(),
        "refusing the dsh seat: its startup stderr capture ended unreadable (permission denied: \
         stderr seam failed), so a warning dsh wrote cannot be ruled out; failing closed on what \
         cannot be read (#430 H1)"
    );
    assert!(
        stderr.contains("stderr capture ended unreadable"),
        "the unreadable capture is named in the evidence: {stderr}"
    );
}

/// A [`Read`] seam that returns `prefix`, then blocks its second read until
/// `release` is signalled, then returns `suffix` and EOF. `blocked` tells
/// the test the reader is inside that second read, so `take(true)` meets a
/// settled, incomplete capture rather than racing it.
struct BlockedAfterPrefix {
    prefix: &'static [u8],
    suffix: &'static [u8],
    blocked: std::sync::mpsc::Sender<()>,
    release: std::sync::mpsc::Receiver<()>,
    reads: u8,
}

impl std::io::Read for BlockedAfterPrefix {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self.reads {
            0 => {
                self.reads = 1;
                buffer[..self.prefix.len()].copy_from_slice(self.prefix);
                Ok(self.prefix.len())
            }
            1 => {
                self.reads = 2;
                let _ = self.blocked.send(());
                let _ = self.release.recv();
                buffer[..self.suffix.len()].copy_from_slice(self.suffix);
                Ok(self.suffix.len())
            }
            _ => Ok(0),
        }
    }
}

/// #430 H1, fifth hold on this class (decision 0098 ruling 4; decision 0071
/// rulings 3 and 9): the bounded settle keeps a descendant-held pipe from
/// hanging the driver, but the capture it takes is INCOMPLETE, and an
/// incomplete capture never lets dsh start. The seam returns the warning's
/// truncated prefix, blocks its next read, and holds the rest back until
/// released: the gate must refuse, naming the incomplete capture, exactly
/// as it refuses an unreadable one.
#[test]
fn chief_review_incomplete_stderr_must_refuse_dsh_startup() {
    use super::super::stderr::StderrEnd;

    let (blocked, blocked_rx) = std::sync::mpsc::channel();
    let (release, release_rx) = std::sync::mpsc::channel();
    let capture = super::super::stderr::capture_reader(BlockedAfterPrefix {
        prefix: b"dsh: warning: 1 entry did not ",
        suffix: b"activate\n",
        blocked,
        release: release_rx,
        reads: 0,
    });
    blocked_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the reader reached its blocked read");
    let captured = capture.take(true);
    let incomplete = matches!(captured.end(), StderrEnd::Incomplete);
    assert!(
        incomplete,
        "the settle takes an incomplete capture: {captured:?}"
    );
    assert_eq!(captured.bytes(), b"dsh: warning: 1 entry did not ");
    let refusal = startup_gate(
        startup_report(&captured),
        &mut serde_json::Map::new(),
        Mode::ReadOnly,
    )
    .expect_err("an incomplete capture never passes dsh's startup gate");
    assert!(matches!(refusal, ReportRefusal::IncompleteCapture));
    assert_eq!(
        refusal.to_string(),
        "refusing the dsh seat: its startup stderr capture ended incomplete (the reader had not \
         reached EOF when the settle bound elapsed), so a warning dsh wrote cannot be ruled out; \
         failing closed on what cannot be read (#430 H1)"
    );
    let evidence = captured.evidence(|_| String::new());
    assert!(
        evidence.contains("stderr capture ended incomplete"),
        "the stopped capture names its incompleteness: {evidence:?}"
    );
    // Let the detached reader finish rather than leaving it blocked.
    release.send(()).expect("the reader is still waiting");
}

/// #430 H1 (decision 0071 rulings 3 and 9): the evidence line for an
/// unreadable capture joins whatever partial bytes it kept, adding exactly
/// one newline only where the kept text does not already end a line. The
/// empty and already-terminated captures add none.
#[test]
fn an_unreadable_capture_joins_its_partial_bytes_with_one_line() {
    let note = "stderr capture ended unreadable: permission denied: stderr seam failed\n";
    let failed = || Step::Error(std::io::ErrorKind::PermissionDenied);
    for (case, steps, expected) in [
        ("no bytes", vec![failed()], note.to_string()),
        (
            "a partial line",
            vec![Step::Data(b"partial"), failed()],
            format!("partial\n{note}"),
        ),
        (
            "a complete line",
            vec![Step::Data(b"partial\n"), failed()],
            format!("partial\n{note}"),
        ),
    ] {
        let (_, stderr) = captured_run(steps);
        assert_eq!(stderr, expected, "{case}");
    }
}

/// The composed-tree allowlist is the controller's measured dump, row for
/// row, and an extra row refuses by its id and package.
#[test]
fn the_composed_tree_is_the_measured_dump_and_refuses_an_ambient_row() {
    use super::composition::{judge, rows, Composition, MEASURED_COMPOSITION};
    // Read at run time, not `include_str!`: the measured dump has one home
    // in `docs/evidence/`, and a crate must not include a file above itself
    // (the published tarball would not carry it).
    let measured = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/evidence/adapters/dsh-hands-round2/dump-stdio.txt"),
    )
    .expect("the controller's measured dump is in the workspace");
    let parsed = rows(measured.as_bytes()).unwrap();
    let mut found: Vec<(&str, &str)> = parsed
        .iter()
        .map(|row| (row.id.as_str(), row.name.as_str()))
        .collect();
    let mut expected = MEASURED_COMPOSITION.to_vec();
    found.sort_unstable();
    expected.sort_unstable();
    assert_eq!(found, expected);
    assert_eq!(parsed.len(), 99);
    // The measured dump itself is admitted: every effectful row is disabled
    // and the one MCP client is Brokkr's.
    assert_eq!(judge(Release::V0_2_1, &parsed), Ok(()));

    let base = "- id: brokkr-hands\n  name: '@deepseek-ai/dsh-mcp-client'\n";
    let cases = [
        (
            format!("{base}- id: ambient-tool\n  name: '@deepseek-ai/dsh-tool-bash'\n"),
            Some(Composition::Unmeasured {
                id: "ambient-tool".into(),
                name: "@deepseek-ai/dsh-tool-bash".into(),
                release: "0.2.1-rc.2",
            }),
        ),
        (
            format!("{base}- id: ambient-hands\n  name: '@deepseek-ai/dsh-mcp-client'\n"),
            Some(Composition::SecondClient {
                id: "ambient-hands".into(),
            }),
        ),
        (
            format!("{base}- id: tool-web\n  name: '@deepseek-ai/dsh-tool-web'\n"),
            Some(Composition::Enabled {
                id: "tool-web".into(),
            }),
        ),
        (base.to_string(), None),
        (
            format!("{base}{base}"),
            Some(Composition::Repeated {
                id: "brokkr-hands".into(),
            }),
        ),
    ];
    for (dump, expected) in cases {
        assert_eq!(
            judge(Release::V0_2_1, &rows(dump.as_bytes()).unwrap()).err(),
            expected,
            "{dump}"
        );
    }
    assert_eq!(rows(b"not a dump\n").err(), Some(Composition::Unreadable));
}

/// 0.2.2-rc.1's measured composition is its own set: 0.2.1's 99, less the
/// removed list-agents row, plus its native and programmatic executors. The
/// programmatic row is disabled by default; the native one is enabled.
#[cfg(unix)]
#[test]
fn the_0_2_2_measured_composition_is_admitted() {
    use super::composition::{judge, rows, ADDED_0_2_2, REMOVED_0_2_2};

    assert_eq!(
        ADDED_0_2_2,
        [
            (
                "tool-executor-native",
                "@deepseek-ai/dsh-tool-executor-native"
            ),
            ("tool-executor-ptc", "@deepseek-ai/dsh-tool-executor-ptc"),
        ]
    );
    assert_eq!(
        REMOVED_0_2_2,
        [(
            "tool-subagent-list-agents",
            "@deepseek-ai/dsh-tool-subagent-control/list-agents",
        )]
    );
    let parsed = rows(measured_dump_0_2_2().as_bytes()).unwrap();
    assert_eq!(parsed.len(), 100);
    assert!(parsed
        .iter()
        .any(|row| row.id == "tool-executor-native" && !row.disabled));
    assert!(parsed
        .iter()
        .any(|row| row.id == "tool-executor-ptc" && row.disabled));
    assert!(!parsed
        .iter()
        .any(|row| row.id == "tool-subagent-list-agents"));
    assert_eq!(judge(Release::V0_2_2, &parsed), Ok(()));
}

/// A 0.2.2-rc.1 tree carrying 0.2.1's removed row is refused by its id and
/// package: each release admits only its own measured set.
#[cfg(unix)]
#[test]
fn a_0_2_2_tree_carrying_the_removed_row_is_refused() {
    use super::composition::{judge, rows, Composition};

    let mut dump = measured_dump_0_2_2();
    dump.push_str(
        "- id: tool-subagent-list-agents\n  \
         name: '@deepseek-ai/dsh-tool-subagent-control/list-agents'\n",
    );
    assert_eq!(
        judge(Release::V0_2_2, &rows(dump.as_bytes()).unwrap()).err(),
        Some(Composition::Unmeasured {
            id: "tool-subagent-list-agents".into(),
            name: "@deepseek-ai/dsh-tool-subagent-control/list-agents".into(),
            release: "0.2.2-rc.1",
        })
    );
}

/// A 0.2.1 tree carrying either 0.2.2-rc.1 executor row is refused: the
/// 0.2.1 set is the measured 99 and nothing another release added.
#[cfg(unix)]
#[test]
fn a_0_2_1_tree_carrying_an_executor_row_is_refused() {
    use super::composition::{judge, rows, Composition};

    for (id, name) in [
        (
            "tool-executor-native",
            "@deepseek-ai/dsh-tool-executor-native",
        ),
        ("tool-executor-ptc", "@deepseek-ai/dsh-tool-executor-ptc"),
    ] {
        let mut dump = measured_dump();
        dump.push_str(&format!("- id: {id}\n  name: '{name}'\n"));
        assert_eq!(
            judge(Release::V0_2_1, &rows(dump.as_bytes()).unwrap()).err(),
            Some(Composition::Unmeasured {
                id: id.into(),
                name: name.into(),
                release: "0.2.1-rc.2",
            }),
            "{id}"
        );
    }
}

/// An unmeasured row is still refused on 0.2.2-rc.1, by its id and
/// package.
#[cfg(unix)]
#[test]
fn an_unmeasured_row_is_refused_on_0_2_2() {
    use super::composition::{judge, rows, Composition};

    let mut dump = measured_dump_0_2_2();
    dump.push_str("- id: ambient-tool\n  name: '@deepseek-ai/dsh-tool-bash'\n");
    assert_eq!(
        judge(Release::V0_2_2, &rows(dump.as_bytes()).unwrap()).err(),
        Some(Composition::Unmeasured {
            id: "ambient-tool".into(),
            name: "@deepseek-ai/dsh-tool-bash".into(),
            release: "0.2.2-rc.1",
        })
    );
}

/// A 0.2.2-rc.1 tree whose programmatic executor is left enabled is
/// refused by its id: the gate is the check that a profile which enables
/// the code-running executor cannot survive the overlay.
#[cfg(unix)]
#[test]
fn a_0_2_2_tree_with_the_programmatic_executor_enabled_is_refused() {
    use super::composition::{judge, rows, Composition};

    let mut parsed = rows(measured_dump_0_2_2().as_bytes()).unwrap();
    let ptc = parsed
        .iter_mut()
        .find(|row| row.id == PTC_ROW)
        .expect("the measured 0.2.2 set names the programmatic executor");
    assert!(ptc.disabled, "the fixture starts with the row disabled");
    ptc.disabled = false;
    assert_eq!(
        judge(Release::V0_2_2, &parsed).err(),
        Some(Composition::Enabled {
            id: "tool-executor-ptc".into(),
        })
    );
}

/// The launch's composed-tree gate reads the release its own binary
/// answers, so the same dump is judged against that release's set: 0.2.2's
/// dump is admitted under 0.2.2 and refused under 0.2.1, and an unmeasured
/// release's dump is read and then refused against no set.
#[cfg(unix)]
#[test]
fn the_composition_gate_keys_on_the_release_it_reads() {
    use super::composition::Composition;

    let dir = tempfile::tempdir().unwrap();
    let dump = dir.path().join("dump.yml");
    std::fs::write(&dump, measured_dump_0_2_2()).unwrap();
    let shim = |version: &str| {
        super::super::tests::executable(
            dir.path(),
            &format!("dsh-{version}"),
            &format!(
                "#!/bin/sh\ncase \"$1\" in --version|-V|-v) printf '{version}\\n'; exit 0 ;; \
                 esac\ncat '{}'\n",
                dump.display()
            ),
        )
    };
    let gate = |version: &str| {
        let mut command = std::process::Command::new(shim(version));
        command.arg("--dump-config");
        composition_gate(command)
    };
    assert_eq!(gate("0.2.2-rc.1"), Ok(()));
    assert_eq!(
        gate("0.2.1-rc.2").err(),
        Some(Composition::Unmeasured {
            id: "tool-executor-native".into(),
            name: "@deepseek-ai/dsh-tool-executor-native".into(),
            release: "0.2.1-rc.2",
        })
    );
    let unknown = gate("0.3.0").unwrap_err();
    assert_eq!(unknown, Composition::UnknownRelease);
    assert_eq!(
        unknown.to_string(),
        "refusing to invoke the dsh driver: the composed dsh tree's release cannot be read as \
         one the hands rows were measured on (0.2.1-rc.2, 0.2.1-rc.3, 0.2.2-rc.1), so the seat \
         is admitted on no measured composition (#896 D1)"
    );
}

/// A fake dsh that records the environment and directory it was handed,
/// answers the drift probe, and composes a valid hands tree. The session
/// (any invocation without `--dump-config`) exits cleanly.
const RECORDING_DSH: &str = r#"#!/bin/sh
case "$1" in
  --version|-V|-v) printf '0.2.1-rc.2\n'; exit 0 ;;
esac
{
  printf 'cwd=%s\n' "$(pwd)"
  printf 'dsh_home=%s\n' "${DSH_HOME-<unset>}"
  printf 'git_key=%s\n' "${GIT_CONFIG_KEY_0-<unset>}"
  printf 'removed=%s\n' "${LIBRARY_DOCS_TOKEN-<unset>}"
  printf '%s\n' '--'
} >> "$BROKKR_TEST_DSH_OBSERVE"
for a in "$@"; do
  if [ "$a" = "--dump-config" ]; then
    printf '%s\n' "- id: brokkr-hands" "  name: '@deepseek-ai/dsh-mcp-client'"
    exit 0
  fi
done
exit 0
"#;

/// The measured fake dsh's pre-session prefix: its version banner and the
/// composition dump `--dump-config` reads. Each fake appends its own body.
#[cfg(unix)]
const DSH_PREFIX: &str = r#"#!/bin/sh
case "$1" in
  --version|-V|-v) printf '0.2.1-rc.2\n'; exit 0 ;;
esac
for a in "$@"; do
  if [ "$a" = "--dump-config" ]; then
    printf '%s\n' "- id: brokkr-hands" "  name: '@deepseek-ai/dsh-mcp-client'"
    exit 0
  fi
done
"#;

/// A fake dsh whose session prints the measured activation warning after a
/// valid composition, or the report `BROKKR_TEST_DSH_REPORT` names when a
/// test needs a marker a bound secret masks (#896 H1).
const WARNING_DSH_TAIL: &str = r#"if [ -n "${BROKKR_TEST_DSH_REPORT-}" ]; then
  printf '%s\n' "$BROKKR_TEST_DSH_REPORT" >&2
else
  printf '%s\n' 'dsh: warning: 1 entry did not activate' \
    'brokkr-hands (@deepseek-ai/dsh-mcp-client): Error: mcp-client(probe): initial connection or tool synchronization failed' >&2
fi
exit 0
"#;

/// The one invocation the launch-path tests drive: a hands seat under
/// `input`, through `dsh`, its task appended last, with `bindings` masked
/// out of the stderr it exposes. Returns the home the launch served and the
/// invocation's outcome.
#[cfg(unix)]
fn invoke_hands(
    dsh: &str,
    workdir: &str,
    mut input: Value,
    bindings: &[crate::secret::BoundSecret],
    probe: bool,
) -> (Option<String>, Result<super::super::Invocation, String>) {
    input["workdir"] = json!(workdir);
    let launch = super::super::dsh_launch_with(dsh, &[], workdir, None, &input, |_| {
        panic!("a cold seat recomputes no composite")
    })
    .unwrap();
    let child_home = launch.home.child_home().map(str::to_string);
    let mut command = launch.command.clone();
    command.push("the task".to_string());
    let result = super::super::dsh_launch::invoke_dsh_launch_observed(
        launch,
        command,
        bindings,
        &mut crate::adapters::probe::ProbeAnswer::new(probe),
        &mut |_| {},
        |child| child.wait().map(|status| Some(status.code().unwrap_or(0))),
        &mut |_| {},
    );
    (child_home, result)
}

/// The one launch the startup-report tests drive: the measured fake dsh
/// that prints `report` on stderr (or its measured activation warning when
/// `None`), as a cold hands seat under its own home, with `bindings` masked
/// out of the stderr it exposes.
#[cfg(unix)]
fn launch_reporting(
    env: &mut EnvGuard,
    dir: &tempfile::TempDir,
    report: Option<&str>,
    bindings: &[crate::secret::BoundSecret],
) -> Result<super::super::Invocation, String> {
    match report {
        Some(report) => env.set("BROKKR_TEST_DSH_REPORT", report),
        None => env.remove("BROKKR_TEST_DSH_REPORT"),
    }
    let fake = super::super::tests::executable(
        dir.path(),
        "dsh",
        &format!("{DSH_PREFIX}{WARNING_DSH_TAIL}"),
    );
    let workdir = dir.path().to_str().unwrap();
    invoke_hands(
        fake.to_str().unwrap(),
        workdir,
        sealed(SealedBoundary::Namespace, Some("read-only")),
        bindings,
        false,
    )
    .1
}

/// The launch's one builder gives the composition probe the session's
/// working directory, environment and removals, so a probe can never read
/// a different tree than the session runs in (#896 round 3).
#[cfg(unix)]
#[test]
fn a_launch_composes_under_the_sessions_own_environment_and_directory() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    // The fake's `pwd` prints the kernel's spelling of its directory, and
    // macOS's temporary root `/var` is a link to `/private/var`: the root
    // is canonical once, here, so the expected `cwd=` is that spelling too.
    let root = dir.path().canonicalize().unwrap();
    let home = root.join("home");
    std::fs::create_dir(&home).unwrap();
    let log = root.join("observed");
    env.set("DSH_HOME", &home);
    env.set("BROKKR_TEST_DSH_OBSERVE", &log);
    env.set("LIBRARY_DOCS_TOKEN", "exported");
    let fake = super::super::tests::executable(&root, "dsh", RECORDING_DSH);
    let workdir = root.to_str().unwrap();
    let input = sealed(SealedBoundary::Namespace, Some("read-only"));
    // An attempt's removal, in force exactly as the invocation holds it.
    let mut removals = input.clone();
    removals[SERVING_INPUTS]["isolation"]["servers"] = json!({"brokered": [{
        "capability": "library-docs",
        "plan": "/state/plans/library-docs.json",
        "plan_digest": "a".repeat(64),
        "removals": ["LIBRARY_DOCS_TOKEN"],
    }]});
    let _removing = super::super::mcp::Removing::attempt(&removals);
    let (child_home, invocation) = invoke_hands(fake.to_str().unwrap(), workdir, input, &[], false);
    invocation.unwrap();
    let expected_home = child_home.unwrap_or_else(|| home.to_string_lossy().into_owned());
    let observed = std::fs::read_to_string(&log).unwrap();
    let blocks = (observed.split("--\n"))
        .filter(|block| !block.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(blocks.len(), 2, "{observed}");
    assert_eq!(
        blocks[0], blocks[1],
        "the probe and the session ran under the same effective context"
    );
    for block in blocks {
        assert!(block.contains(&format!("cwd={workdir}\n")), "{block}");
        assert!(
            block.contains(&format!("dsh_home={expected_home}\n")),
            "{block}"
        );
        assert!(block.contains("git_key=commit.gpgsign\n"), "{block}");
        assert!(block.contains("removed=<unset>\n"), "{block}");
    }
}

/// A launch whose composition succeeds but whose measured startup warning
/// reports the hands row did not activate fails the INVOCATION, not only a
/// helper (#896 round 3).
#[cfg(unix)]
#[test]
fn a_launch_fails_closed_when_the_measured_activation_warning_arrives() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    env.set("DSH_HOME", dir.path());
    let problem = launch_reporting(&mut env, &dir, None, &[])
        .err()
        .expect("the measured activation warning refuses the invocation");
    assert_eq!(problem, ReportRefusal::NotActivated.to_string());
}

/// A healthy launch whose model reasons about the refusal words stays
/// healthy: only the stderr dsh wrote before its session is classified, so
/// session content can neither hide nor manufacture a refusal (#896 M1;
/// decision 0098 ruling 4).
#[cfg(unix)]
#[test]
fn a_healthy_launch_is_not_refused_by_its_models_reasoning() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    env.set("DSH_HOME", dir.path());
    let invocation = launch_reporting(
        &mut env,
        &dir,
        Some(
            "dsh: reasoning:\n\
             I should explain what SANDBOX_UNAVAILABLE means: no sandbox backend is usable on \
             this host; refusing to run the command unconfined.\n\
             brokkr-hands (@deepseek-ai/dsh-mcp-client): Error: initial connection or tool \
             synchronization failed\ndsh: done",
        ),
        &[],
    )
    .expect("the model's reasoning refuses no healthy launch");
    assert_eq!(invocation.exit_code, 0);
    assert_eq!(
        invocation.stderr,
        "dsh: reasoning: [not journaled — decision 0034]\ndsh: done\n"
    );
}

/// A bound secret whose value sits inside dsh's marker text cannot hide the
/// refusal: the outcome is classified from the RAW stderr once, before the
/// exposed text is masked, and the stderr that reaches the invocation
/// carries only the mask, never the value (#896 H1; decision 0098 ruling 4).
#[cfg(unix)]
#[test]
fn a_bound_secret_never_hides_a_startup_refusal_or_appears_in_the_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let mut env = EnvGuard::lock();
    env.set("DSH_HOME", dir.path());
    let cases = [
        (
            "ENTRY",
            "entry",
            "dsh: warning: 1 entry did not activate",
            ReportRefusal::NotActivated,
        ),
        (
            "HANDS",
            "brokkr-hands",
            "brokkr-hands (@deepseek-ai/dsh-mcp-client): Error: mcp-client(probe): initial \
             connection or tool synchronization failed",
            ReportRefusal::NotActivated,
        ),
        (
            "SANDBOX",
            "SANDBOX_UNAVAILABLE",
            "HarnessError: SANDBOX_UNAVAILABLE",
            ReportRefusal::SandboxUnavailable,
        ),
    ];
    for (name, value, report, expected) in cases {
        let bound = binding(name, value);
        let problem = launch_reporting(&mut env, &dir, Some(report), std::slice::from_ref(&bound))
            .err()
            .expect("the masked marker still refuses the launch");
        assert_eq!(problem, expected.to_string(), "{report}");

        // The same raw report through the one capture edge the launch uses:
        // the typed outcome is the refusal and the exposed stderr is masked.
        let raw = report.as_bytes().to_vec();
        let run = super::super::finish_dsh(
            &mut serde_json::Map::new(),
            0,
            captured(raw),
            None,
            std::slice::from_ref(&bound),
        )
        .unwrap();
        assert_eq!(run.startup, Err(expected), "{report}");
        let mask = format!("[secret:{name}]");
        assert!(
            run.invocation.stderr.contains(&mask),
            "the exposed stderr carries the mask: {}",
            run.invocation.stderr
        );
        assert!(
            !run.invocation.stderr.contains(value),
            "the exposed stderr never carries the value: {}",
            run.invocation.stderr
        );
    }
}

/// A fake dsh the H1 case drives: its measured composition on
/// `--dump-config`, dsh's startup report on stderr from
/// `BROKKR_TEST_DSH_REPORT`, and then a valid session header plus one
/// `assistant/message` answer under the seat overlay's own root. It exits
/// after the answer, so the probe's own stop is what the run proves.
#[cfg(unix)]
const PROBE_ANSWER_DSH_TAIL: &str = r#"printf '%s\n' "${BROKKR_TEST_DSH_REPORT-dsh: warning: 1 entry did not activate}" >&2
while [ "$1" != --patch ] && [ $# -gt 0 ]; do shift; done
shift
root=$(awk -F"'" '/^    root: /{print $2}' "$1")
d="$root/--project--/session-fake"
mkdir -p "$d"
f="$d/session.v3.jsonl"
printf '{"type":"session","version":0,"id":"session-fake-1","cwd":"/w"}\n' > "$f"
printf '{"type":"assistant/message","data":{"turn":1,"step":1,"message":{"source":{"model":"served-by-dsh"}},"usage":{"inputTokens":10,"outputTokens":2}}}\n' >> "$f"
"#;

/// #430 H1 (decision 0098 ruling 4; decision 0071 rulings 3 and 9): a
/// probe's answer must not erase dsh's startup refusal. The stopped path
/// used to hand `finish_dsh` no stderr, which `startup_report` classified
/// as full enforcement, so the startup gate passed over the warning dsh
/// had already written. The same case is refused through the one gate for
/// both refusal classes.
#[cfg(unix)]
#[test]
fn review_a_dsh_probe_answer_must_not_erase_its_startup_refusal() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    env.set("DSH_HOME", dir.path());
    for (report, expected) in [
        (
            "dsh: warning: 1 entry did not activate",
            ReportRefusal::NotActivated,
        ),
        (
            "HarnessError: SANDBOX_UNAVAILABLE",
            ReportRefusal::SandboxUnavailable,
        ),
    ] {
        env.set("BROKKR_TEST_DSH_REPORT", report);
        let fake = super::super::tests::executable(
            dir.path(),
            "dsh",
            &format!("{DSH_PREFIX}{PROBE_ANSWER_DSH_TAIL}"),
        );
        let workdir = dir.path().to_str().unwrap();
        let problem = invoke_hands(
            fake.to_str().unwrap(),
            workdir,
            sealed(SealedBoundary::Namespace, Some("read-only")),
            &[],
            true,
        )
        .1
        .err()
        .expect("the stopped probe's startup report still refuses the launch");
        assert_eq!(problem, expected.to_string(), "{report}");
    }
}

/// A native writing seat in a linked worktree is a runner seat, never a
/// hands seat: its one overlay carries the transcript row followed by the
/// scoped runner's staged row, exactly as main wrote it, on both the
/// shipped and the stream-json launch (#896 round 4). Hands stay selected
/// only by [`Selection::Hands`], so neither the composition probe nor the
/// startup gate runs for it. A stand-in bwrap whose one probe succeeds
/// keeps the test off this box's unusable bubblewrap branch.
#[cfg(target_os = "linux")]
#[test]
fn a_linked_worktree_runner_is_not_a_hands_launch() {
    use super::super::tests::{dsh_enabled_input, real_linked_worktree, synthetic_dsh_composite};
    let mut env = EnvGuard::lock();
    let (dir, worktree, _facts) = real_linked_worktree();
    std::os::unix::fs::symlink("/bin/true", dir.path().join("bwrap")).unwrap();
    let mut search = std::ffi::OsString::from(dir.path());
    search.push(":");
    search.push(std::env::var_os("PATH").unwrap_or_default());
    env.set("PATH", &search);
    // The seat process may inherit a broken `git -c` triple from the host;
    // this test needs `git_facts` to read the real linked worktree.
    env.set("GIT_CONFIG_COUNT", "1");
    env.set("GIT_CONFIG_KEY_0", "commit.gpgsign");
    env.set("GIT_CONFIG_VALUE_0", "false");
    env.set("DSH_PERMISSION_MODE", "workspace-write");
    env.set("DSH_HOME", dir.path());
    let workdir = worktree.to_str().unwrap();
    let dsh = dsh_version_shim(dir.path(), "dsh", "0.1.5-rc.1");
    let dsh = dsh.to_str().unwrap();
    let assert_runner = |launch: super::super::DshLaunch, stream: bool| {
        let (row, _, store) = launch
            .staged
            .as_ref()
            .expect("the linked worktree stages a scoped runner");
        assert_eq!(
            launch.mode, None,
            "a runner's overlay row is not a hands selection"
        );
        assert_eq!(launch.stream_json, stream);
        let mut expected = super::super::dsh_transcript_row(&launch.root).unwrap();
        expected.push_str(row);
        let overlay = std::fs::read_to_string(launch.overlay.path()).unwrap();
        assert_eq!(
            overlay, expected,
            "the runner overlay is main's: the transcript row then the scoped-runner row"
        );
        assert!(
            row.contains("runnerCommand:"),
            "the staged row names the runner command: {row}"
        );
        for binding in [store.store_path(), store.trusted_path()] {
            let binding = binding.to_str().unwrap();
            assert!(
                overlay.contains(binding),
                "the overlay binds the private store path {binding}: {overlay}"
            );
        }
    };
    let cold = super::super::dsh_launch_with(
        dsh,
        &[],
        workdir,
        None,
        &json!({"workdir": workdir}),
        |_| panic!("a cold seat recomputes no composite"),
    )
    .unwrap();
    assert_runner(cold, false);
    let digest = "b".repeat(64);
    let input = dsh_enabled_input("0.1.5-rc.1", &digest, &worktree);
    let warm = super::super::dsh_launch_with(dsh, &[], workdir, None, &input, |_| {
        Ok(synthetic_dsh_composite(&digest))
    })
    .unwrap();
    assert_runner(warm, true);
}

/// The judge refuses each row a seat may not be served, driven from the
/// dump text itself, and prints the operator's full words; a measured row
/// left disabled is admitted.
#[test]
fn the_composed_tree_judge_refuses_by_its_full_words() {
    use super::composition::{judge, rows, Composition};
    let hands = "- id: brokkr-hands\n  name: '@deepseek-ai/dsh-mcp-client'\n";
    let cases = [
        (
            format!("{hands}- id: ambient\n  name: '@deepseek-ai/dsh-ambient'\n"),
            Composition::Unmeasured {
                id: "ambient".into(),
                name: "@deepseek-ai/dsh-ambient".into(),
                release: "0.2.1-rc.2",
            },
            "refusing to invoke the dsh driver: the composed dsh tree carries row 'ambient' of \
             package '@deepseek-ai/dsh-ambient', which the measured hands composition (dsh \
             0.2.1-rc.2) does not name, so a seat would be served a row nobody measured (#896 D1)",
        ),
        (
            "- id: tool-bash\n  name: '@deepseek-ai/dsh-tool-bash'\n  disabled: true\n".to_string(),
            Composition::NoHands,
            "refusing to invoke the dsh driver: the composed dsh tree carries no 'brokkr-hands' \
             row, so the seat's one effectful tool would not be served (#896 D1)",
        ),
        (
            format!("{hands}- id: ambient-hands\n  name: '@deepseek-ai/dsh-mcp-client'\n"),
            Composition::SecondClient {
                id: "ambient-hands".into(),
            },
            "refusing to invoke the dsh driver: the composed dsh tree carries a second \
             dsh-mcp-client row 'ambient-hands', so the seat would be served a second effectful \
             server beside Brokkr's (#896 D1)",
        ),
        (
            format!("{hands}- id: tool-web\n  name: '@deepseek-ai/dsh-tool-web'\n"),
            Composition::Enabled {
                id: "tool-web".into(),
            },
            "refusing to invoke the dsh driver: the composed dsh tree carries row 'tool-web' \
             enabled, which the hands rows disable, so the seat would keep an effectful tool \
             beside the box (#896 D1)",
        ),
        (
            "- id: brokkr-hands\n  name: '@deepseek-ai/dsh-mcp-client'\n  disabled: true\n"
                .to_string(),
            Composition::HandsDisabled {
                id: "brokkr-hands".into(),
            },
            "refusing to invoke the dsh driver: the composed dsh tree carries row 'brokkr-hands' \
             disabled, so the seat's one effectful tool would not be served (#896 D1)",
        ),
        (
            format!("{hands}{hands}"),
            Composition::Repeated {
                id: "brokkr-hands".into(),
            },
            "refusing to invoke the dsh driver: the composed dsh tree names row 'brokkr-hands' \
             twice, so no row it reads can be trusted (#896 D1)",
        ),
    ];
    for (dump, expected, text) in cases {
        let refusal = judge(Release::V0_2_1, &rows(dump.as_bytes()).unwrap()).unwrap_err();
        assert_eq!(refusal, expected, "{dump}");
        assert_eq!(refusal.to_string(), text, "{dump}");
    }
    let admitted =
        format!("{hands}- id: tool-bash\n  name: '@deepseek-ai/dsh-tool-bash'\n  disabled: true\n");
    assert_eq!(
        judge(Release::V0_2_1, &rows(admitted.as_bytes()).unwrap()),
        Ok(())
    );
}

/// The reader refuses a row it cannot read as one id and one name, printing
/// the refusal's full words.
#[test]
fn the_composed_tree_reader_refuses_a_row_that_is_not_one_id_and_name() {
    use super::composition::{rows, Composition};
    for dump in [
        "- id: twice\n  name: first\n  name: second\n",
        "- id: stray\n  name: '@deepseek-ai/dsh-tool-bash'\nnot a row\n",
        "- id: nameless\n",
    ] {
        let refusal = rows(dump.as_bytes()).unwrap_err();
        assert_eq!(refusal, Composition::Unreadable, "{dump}");
        assert_eq!(refusal.to_string(), UNREADABLE, "{dump}");
    }
}

/// The dump path refuses a failed dump by its exit code, a dump killed by
/// a signal in its own words, and a dump over the bound as unreadable.
#[cfg(unix)]
#[test]
fn the_composed_tree_check_refuses_a_failed_or_oversized_dump() {
    use super::composition::{composed_with, Composition};
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, ExitStatus, Output};

    let run = |status: ExitStatus, stdout: Vec<u8>| {
        composed_with(Command::new("dsh"), Some(Release::V0_2_1), |_| {
            Ok(Output {
                status,
                stdout,
                stderr: Vec::new(),
            })
        })
    };
    let exit = run(ExitStatus::from_raw(2 << 8), Vec::new()).unwrap_err();
    assert_eq!(
        exit,
        Composition::DumpFailed {
            status: "exit 2".into()
        }
    );
    assert_eq!(
        exit.to_string(),
        "refusing to invoke the dsh driver: `dsh --dump-config` failed (exit 2), so the \
         composed tree could not be read and the seat is not launched (#896 D1)"
    );
    let signal = run(ExitStatus::from_raw(9), Vec::new()).unwrap_err();
    assert_eq!(
        signal,
        Composition::DumpFailed {
            status: "by signal".into()
        }
    );
    assert_eq!(
        signal.to_string(),
        "refusing to invoke the dsh driver: `dsh --dump-config` failed (by signal), so the \
         composed tree could not be read and the seat is not launched (#896 D1)"
    );
    // An otherwise admissible dump padded past the bound with comments: the
    // reader alone would admit it, so only the bound can refuse it.
    let mut padded = String::from("- id: brokkr-hands\n  name: '@deepseek-ai/dsh-mcp-client'\n");
    padded.push_str(&"#\n".repeat((1 << 20) / 2 + 1));
    let oversized = run(ExitStatus::from_raw(0), padded.into_bytes()).unwrap_err();
    assert_eq!(oversized, Composition::Unreadable);
    assert_eq!(oversized.to_string(), UNREADABLE);
    // A clean dump whose bytes are not rows is refused by the same reader.
    let shapeless = run(ExitStatus::from_raw(0), b"not a dump\n".to_vec()).unwrap_err();
    assert_eq!(shapeless, Composition::Unreadable);
    assert_eq!(shapeless.to_string(), UNREADABLE);
}

/// The masked `spark-glm` provider entry the driver's closed route grammar
/// admits: `https://` and a reasoning level, neither of which the real
/// route's endpoint uses. The launch test never dials it.
#[cfg(target_os = "linux")]
const SPARK_GLM_ROUTE: &str = "\
- id: llm-pi-ai
  config:
    providers:
      spark-glm:
        apiKeyEnv: SPARK_API_KEY
        api: openai-completions
        baseURL: https://spark.invalid:8888/v1
        models:
          - id: GLM-5.3-Flash-EXL3
            reasoningEfforts:
              low: low
";

/// The measured 99 rows as `--dump-config` writes them, with the ten
/// effectful rows disabled, so the composed-tree gate reads the measured
/// composition.
#[cfg(unix)]
fn measured_dump() -> String {
    use super::composition::MEASURED_COMPOSITION;
    MEASURED_COMPOSITION
        .iter()
        .map(|(id, name)| {
            let disabled = super::disabled(Release::V0_2_1, id);
            format!(
                "- id: {id}\n  name: '{name}'\n{}",
                if disabled { "  disabled: true\n" } else { "" }
            )
        })
        .collect()
}

/// 0.2.2-rc.1's measured 100 rows as `--dump-config` writes them: 0.2.1's
/// set less its removed row, plus the two executors, with every effectful
/// row that release's overlay disables marked disabled.
#[cfg(unix)]
fn measured_dump_0_2_2() -> String {
    use super::composition::{ADDED_0_2_2, MEASURED_COMPOSITION, REMOVED_0_2_2};
    let mut rows: Vec<(&str, &str)> = MEASURED_COMPOSITION
        .into_iter()
        .filter(|row| !REMOVED_0_2_2.contains(row))
        .collect();
    rows.extend(ADDED_0_2_2);
    rows.iter()
        .map(|(id, name)| {
            let disabled = super::disabled(Release::V0_2_2, id);
            format!(
                "- id: {id}\n  name: '{name}'\n{}",
                if disabled { "  disabled: true\n" } else { "" }
            )
        })
        .collect()
}

/// A provider shim that records the engine-only home, the working
/// directory and (for the session) the one `--patch` overlay, and prints
/// `dump` for the composition probe.
#[cfg(target_os = "linux")]
fn probe_shim(
    root: &std::path::Path,
    dump: &std::path::Path,
    observe: &std::path::Path,
) -> std::path::PathBuf {
    let script = r#"#!/bin/sh
case "$1" in
  --version|-V|-v) printf '0.2.1-rc.2\n'; exit 0 ;;
esac
probe=0
for a in "$@"; do
  [ "$a" = --dump-config ] && probe=1
done
{
  printf 'home=%s\n' "${DSH_HOME-<unset>}"
  printf 'cwd=%s\n' "$(pwd)"
  if [ "$probe" = 0 ]; then
    prev=
    for a in "$@"; do
      [ "$prev" = --patch ] && cat "$a"
      prev=$a
    done
  fi
  printf '%s\n' '--'
} >> 'OBSERVE'
[ "$probe" = 1 ] && cat 'DUMP'
exit 0
"#
    .replace("DUMP", dump.to_str().unwrap())
    .replace("OBSERVE", observe.to_str().unwrap());
    super::super::tests::executable(root, "dsh", &script)
}

/// The sealed inputs for the boxed hands office, with `servers` as the
/// intent and the route claim the driver reads.
#[cfg(target_os = "linux")]
fn boxed_hands_input(work: &std::path::Path, digest: &str, servers: Value) -> Value {
    let mut input = sealed(SealedBoundary::Namespace, Some("writes"));
    input["workdir"] = json!(work.to_str().unwrap());
    input["serving_inputs"]["isolation"]["fence"] = json!("lifted");
    input["serving_inputs"]["isolation"]["cold"] = json!("measured");
    input["serving_inputs"]["isolation"]["replacement"] = json!("measured");
    input["serving_inputs"]["isolation"]["servers"] = servers;
    input["resume_context"] = json!({"route_overlay": {"value": "route.yml", "digest": digest}});
    input
}

/// The server rows the overlay's one `- insert:` item carries, parsed from
/// the nested list's `id` and `name` lines. The launch test reads a row's
/// fields here rather than counting a word in the text.
#[cfg(target_os = "linux")]
fn inserted_server_rows(overlay: &str) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    let mut insert = false;
    let mut row: Option<(String, String)> = None;
    for line in overlay.lines() {
        let body = line.trim_start_matches(' ');
        if body.is_empty() {
            continue;
        }
        let indent = line.len() - body.len();
        match (indent, body) {
            (0, _) => {
                if let Some(row) = row.take() {
                    rows.push(row);
                }
                insert = body == "- insert:";
            }
            (4, body) if insert => {
                if let Some(id) = body.strip_prefix("- id: ") {
                    if let Some(row) = row.replace((id.to_string(), String::new())) {
                        rows.push(row);
                    }
                }
            }
            (6, body) if insert => {
                if let (Some((_, name)), Some(value)) = (row.as_mut(), body.strip_prefix("name: "))
                {
                    *name = value.trim_matches('\'').to_string();
                }
            }
            _ => {}
        }
    }
    if let Some(row) = row.take() {
        rows.push(row);
    }
    rows
}

/// The shim's two blocks: the composition probe and the session read the
/// same engine-only home and working directory, and the session's one
/// overlay carries the measured hands rows and the route entry.
#[cfg(target_os = "linux")]
fn assert_probe_and_session(observed: &str, staged: &Path, work: &Path) {
    let blocks: Vec<&str> = observed.split("--\n").filter(|b| !b.is_empty()).collect();
    assert_eq!(blocks.len(), 2, "the probe and the session: {observed}");
    for block in &blocks {
        let line = |prefix: &str| {
            block
                .lines()
                .find_map(|l| l.strip_prefix(prefix))
                .unwrap_or_else(|| panic!("{prefix} in {block}"))
        };
        assert_eq!(Path::new(line("home=")), staged);
        assert_eq!(Path::new(line("cwd=")), work);
    }
    let overlay = blocks[1];
    for row in [
        "- id: tool-bash",
        "- id: sandbox-policy",
        "mode: workspace-write",
        "spark-glm",
    ] {
        assert!(overlay.contains(row), "missing {row:?} in {overlay}");
    }
    assert_eq!(
        inserted_server_rows(overlay),
        vec![(super::HANDS_ID.to_string(), super::MCP_CLIENT.to_string())],
        "the overlay inserts Brokkr's one server row: {overlay}"
    );
}

/// #896 review finding 3: the boxed hands launch is served from an
/// engine-only home, and a set carrying a second sealed server is refused
/// at that launch by name. `dsh_launch_with` is the function the driver's
/// start runs before it spawns, so this exercises the home decision, the
/// route claim, the one composed overlay and the composed-tree gate
/// together; the provider is a shim, and the measured 99 rows are the
/// round-2 dump. The real dsh tool listing is the recorded run in
/// `docs/evidence/adapters/dsh-boxed-hands.md`. Only Linux seals this
/// shape: no engine-only home is measured on macOS, so the test runs where
/// its seal is real.
#[cfg(target_os = "linux")]
#[test]
fn a_boxed_dsh_hands_launch_is_served_from_the_engine_only_home() {
    use super::super::dsh_launch::invoke_dsh_launch_observed;
    use sha2::Digest as _;

    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let observe = root.join("observe");
    let home = root.join("home");
    let work = root.join("work");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(work.join("route.yml"), SPARK_GLM_ROUTE).unwrap();
    let digest = {
        let mut hasher = sha2::Sha256::new();
        hasher.update(SPARK_GLM_ROUTE.as_bytes());
        hex::encode(hasher.finalize())
    };
    let dump = root.join("measured.yml");
    std::fs::write(&dump, measured_dump()).unwrap();
    let shim = probe_shim(&root, &dump, &observe);
    env.set("BROKKR_DSH_BIN", &shim);
    env.set("HOME", &home);
    env.remove("DSH_HOME");
    let extra = s(&[
        "--model",
        "spark-glm/GLM-5.3-Flash-EXL3",
        "--patch",
        "route.yml",
    ]);

    let launch = super::super::dsh_launch_with(
        shim.to_str().unwrap(),
        &extra,
        work.to_str().unwrap(),
        None,
        &boxed_hands_input(&work, &digest, json!("hands")),
        |_| panic!("a cold seat recomputes no composite"),
    )
    .expect("the measured boxed hands shape is admitted cold");
    let staged = std::path::PathBuf::from(
        launch
            .home
            .child_home()
            .expect("a boxed hands seat is served an engine-only home"),
    );
    let base = home.join(".local/state/brokkr/dsh-homes");
    assert!(
        staged.starts_with(&base),
        "the engine-only home is not under {base:?}: {}",
        staged.display()
    );
    let mut command = launch.command.clone();
    command.push("the task".to_string());
    let mut emit = |_: &Value| {};
    let mut observer = |_: &crate::adapters::DshObservation| {};
    invoke_dsh_launch_observed(
        launch,
        command,
        &[],
        &mut crate::adapters::probe::ProbeAnswer::new(false),
        &mut emit,
        |child| child.wait().map(|status| Some(status.code().unwrap_or(0))),
        &mut observer,
    )
    .expect("the measured composition is admitted and the shim runs");

    let observed = std::fs::read_to_string(&observe).expect("the shim recorded its launch");
    assert_probe_and_session(&observed, &staged, &work);

    // A set carrying a second sealed server is refused by name before any
    // home is staged or any child spawns.
    std::fs::remove_file(&observe).unwrap();
    let refused = super::super::dsh_launch_with(
        shim.to_str().unwrap(),
        &extra,
        work.to_str().unwrap(),
        None,
        &boxed_hands_input(
            &work,
            &digest,
            json!({"brokered": [{
                "capability": "web-search",
                "plan": "/w/plan.json",
                "plan_digest": "00",
                "removals": [],
            }]}),
        ),
        |_| panic!("a refused launch recomputes no composite"),
    )
    .err()
    .expect("a second sealed server refuses the launch");
    // This is `isolated()`'s brokers check, which runs before `admits`.
    // The `admits` Brokered arm is bound by the unit test
    // `a_dsh_hands_set_is_the_only_non_empty_set_admitted` in
    // `adapters/mcp/tests/dsh_home.rs`.
    assert_eq!(
        refused,
        "refusing to invoke the agent CLI: provider 'dsh' has no measured strict MCP \
         configuration for 'brokers'",
        "the second server refuses by name"
    );
    assert!(!observe.exists(), "a brokered set spawns no child");
}
