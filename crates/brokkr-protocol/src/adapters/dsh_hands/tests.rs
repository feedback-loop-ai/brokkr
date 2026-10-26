//! A dsh seat's hands rows (#896): the measured set exactly, the bounded
//! reader's one shape for them, the version they bind to, the seat class
//! their sandbox mode follows, and a seat with no hands served as before.

use std::path::Path;

use serde_json::{json, Value};

use super::super::mcp::tests::{dispatched, reach};
use super::super::route_overlay;
use super::super::tests::dsh_version_shim;
use super::*;
use crate::env_guard::EnvGuard;
use crate::hands::{HandsSpec, SEALED_REACH};
use crate::native_controls::{SealedBoundary, SealedDialect, ARTIFACT_ROOT, SERVING_INPUTS};

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
         config:\n        serverName: brokkr\n        command: '{}'\n        args:\n",
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
/// under `stands`, the seat class marked `class` where one is.
fn sealed(stands: SealedBoundary, class: Option<&str>) -> Value {
    let dialect = SealedDialect {
        stands: Some(stands),
        ..Default::default()
    };
    let inputs = dispatched(dialect, Vec::new(), Some(HandsSpec::default()));
    let mut input = json!({"workdir": "/w", ARTIFACT_ROOT: "/a", SERVING_INPUTS: inputs});
    input[SEALED_REACH] = serde_json::to_value(reach()).unwrap();
    if let Some(class) = class {
        input[SEAT_CLASS] = json!(class);
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
            hands_rows(mode, "/bin/brokkr", &args).unwrap(),
            format!(
                "{MEASURED_HEAD}{word}\n{}",
                server_row("/bin/brokkr", &args)
            ),
            "{mode:?}"
        );
    }
    assert_eq!(
        hands_rows(Mode::ReadOnly, "/bin/brokkr", &s(&["a\nb"])),
        Err(Refusal::Multiline)
    );
    assert_eq!(
        hands_rows(Mode::ReadOnly, "/bin/\rbrokkr", &[]),
        Err(Refusal::Multiline)
    );
    assert_eq!(
        Refusal::Multiline.to_string(),
        "refusing to invoke the dsh driver: the box's hands server path spans more than one \
         line, so it cannot be written as one overlay scalar"
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

/// The drift check binds the rows to the releases they were measured on:
/// any other release, or none readable, is refused by name.
#[test]
fn only_a_measured_release_is_served_the_hands_rows() {
    for version in MEASURED_VERSIONS {
        assert_eq!(measured(Some(version.to_string())), Ok(()));
    }
    let tail = " is not a dsh release its hands rows were measured on (0.2.1-rc.2, \
         0.2.1-rc.3), and a tool row nobody measured could stay enabled beside the box, so \
         the seat is not served its native tools (#896)";
    let other = measured(Some("0.2.1-rc.4".into())).unwrap_err();
    assert_eq!(
        other.to_string(),
        format!("refusing to invoke the dsh driver: dsh '0.2.1-rc.4'{tail}")
    );
    assert_eq!(
        measured(None).unwrap_err().to_string(),
        format!("refusing to invoke the dsh driver: an unreadable dsh version{tail}")
    );
}

/// A gate runs dsh's sandbox read-only and a work seat workspace-write;
/// a class the input does not name is refused.
#[test]
fn the_seat_class_selects_the_sandbox_mode() {
    let class = |value: Value| Mode::of(&json!({SEAT_CLASS: value}));
    assert_eq!(class(json!("gate")), Ok(Mode::ReadOnly));
    assert_eq!(class(json!("work")), Ok(Mode::WorkspaceWrite));
    for other in [json!("review"), json!(1), Value::Null] {
        assert_eq!(class(other), Err(Refusal::NoClass));
    }
    assert_eq!(Mode::of(&json!({})), Err(Refusal::NoClass));
    assert_eq!(
        Refusal::NoClass.to_string(),
        "refusing to invoke the dsh driver: the engine names no seat class for a seat whose \
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
    for (class, word) in [("gate", "read-only"), ("work", "workspace-write")] {
        let input = sealed(SealedBoundary::Namespace, Some(class));
        let (staged, rows) = sandbox(dsh, workdir, &facts, &input).unwrap();
        assert!(staged.is_none(), "{class}");
        let server = server_row(brokkr, &bound(workdir));
        assert_eq!(rows, Some(format!("{MEASURED_HEAD}{word}\n{server}")));
    }
    let unprobed = "/nonexistent/dsh";
    for input in [
        json!({"workdir": workdir}),
        sealed(SealedBoundary::Harness, Some("gate")),
        sealed(SealedBoundary::Open, Some("work")),
        json!({SERVING_INPUTS: dispatched(SealedDialect::default(), Vec::new(), None)}),
    ] {
        let (staged, rows) = sandbox(unprobed, workdir, &facts, &input).unwrap();
        assert!(staged.is_none() && rows.is_none(), "{input}");
    }
}

/// A boxed seat is refused on an unmeasured release, without a seat
/// class, or with sealed inputs that cannot be read, before any row is
/// written.
#[cfg(unix)]
#[test]
fn a_boxed_seat_is_refused_before_its_rows_are_written() {
    let dir = tempfile::tempdir().unwrap();
    let workdir = dir.path().to_str().unwrap();
    let facts = GitFacts::default();
    let newer = dsh_version_shim(dir.path(), "newer", "0.2.2");
    let boxed = sealed(SealedBoundary::Seatbelt, Some("gate"));
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
    let classless = sealed(SealedBoundary::Container, None);
    assert_eq!(
        refused("/nonexistent/dsh", &classless),
        Some(Refusal::NoClass)
    );
    let unreadable = json!({SERVING_INPUTS: "garbage"});
    let refusal = refused("/nonexistent/dsh", &unreadable).unwrap();
    assert_eq!(refusal, Refusal::Unbound(McpRefusal::Unreadable));
    assert_eq!(
        refusal.to_string(),
        "refusing to invoke the agent CLI: the engine's MCP isolation intent cannot be read, so \
         no strict MCP configuration is built"
    );
    assert_eq!(String::from(Refusal::NoClass), Refusal::NoClass.to_string());
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
    let boxed = launch(&sealed(SealedBoundary::Namespace, Some("gate"))).unwrap();
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
        &sealed(SealedBoundary::Namespace, Some("work")),
        |_| panic!("refused before any composite"),
    )
    .err()
    .unwrap();
    let unmeasured = Refusal::Unmeasured {
        observed: "dsh '0.1.5-rc.1'".into(),
    };
    assert_eq!(refused, unmeasured.to_string());
}
