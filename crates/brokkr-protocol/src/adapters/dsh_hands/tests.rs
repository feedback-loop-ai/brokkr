//! A dsh seat's hands rows (#896): the measured set exactly, the bounded
//! reader's one shape for them, the version they bind to, the hands access
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
    let rows = hands_rows(Mode::ReadOnly, "/bin/brokkr", &s(&["hands"])).unwrap();
    let server = rows
        .split("- insert:")
        .nth(1)
        .expect("the insert row is written");
    assert!(server.contains("transport: stdio"), "{server}");
    assert!(server.contains("failOnStartupError: true"), "{server}");
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
            "landlock-run: partial enforcement (older Landlock ABI)\n",
            &mut meta,
            Mode::ReadOnly
        ),
        Ok(())
    );
    assert_eq!(meta["sandbox"], json!("read-only"));
    assert_eq!(meta["step_name"], json!(report::PARTIAL));
    assert_eq!(
        startup_gate(warning, &mut serde_json::Map::new(), Mode::WorkspaceWrite),
        Err(ReportRefusal::NotActivated)
    );
    assert_eq!(checked("dsh: done\n"), Ok(Enforcement::Full));
    assert_eq!(
        checked("dsh: warning: 0 entries did not activate\n"),
        Ok(Enforcement::Full)
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

    let base = "- id: brokkr-hands\n  name: '@deepseek-ai/dsh-mcp-client'\n";
    let cases = [
        (
            format!("{base}- id: ambient-tool\n  name: '@deepseek-ai/dsh-tool-bash'\n"),
            Some(Composition::Unmeasured {
                id: "ambient-tool".into(),
                name: "@deepseek-ai/dsh-tool-bash".into(),
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
            judge(&rows(dump.as_bytes()).unwrap()).err(),
            expected,
            "{dump}"
        );
    }
    assert_eq!(rows(b"not a dump\n").err(), Some(Composition::Unreadable));
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

/// A fake dsh whose session prints the measured activation warning after a
/// valid composition.
const WARNING_DSH: &str = r#"#!/bin/sh
case "$1" in
  --version|-V|-v) printf '0.2.1-rc.2\n'; exit 0 ;;
esac
for a in "$@"; do
  if [ "$a" = "--dump-config" ]; then
    printf '%s\n' "- id: brokkr-hands" "  name: '@deepseek-ai/dsh-mcp-client'"
    exit 0
  fi
done
printf '%s\n' 'dsh: warning: 1 entry did not activate' \
  'brokkr-hands (@deepseek-ai/dsh-mcp-client): Error: mcp-client(probe): initial connection or tool synchronization failed' >&2
exit 0
"#;

/// The one invocation the two launch-path tests drive: a hands seat under
/// `input`, through `dsh`, its task appended last. Returns the home the
/// launch served and the invocation's outcome.
#[cfg(unix)]
fn invoke_hands(
    dsh: &str,
    workdir: &str,
    mut input: Value,
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
        workdir,
        &[],
        &mut |_| {},
        |child| child.wait().map(|status| Some(status.code().unwrap_or(0))),
        &mut |_| {},
    );
    (child_home, result)
}

/// The launch's one builder gives the composition probe the session's
/// working directory, environment and removals, so a probe can never read
/// a different tree than the session runs in (#896 round 3).
#[cfg(unix)]
#[test]
fn a_launch_composes_under_the_sessions_own_environment_and_directory() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let log = dir.path().join("observed");
    env.set("DSH_HOME", &home);
    env.set("BROKKR_TEST_DSH_OBSERVE", &log);
    env.set("LIBRARY_DOCS_TOKEN", "exported");
    let fake = super::super::tests::executable(dir.path(), "dsh", RECORDING_DSH);
    let workdir = dir.path().to_str().unwrap();
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
    let (child_home, invocation) = invoke_hands(fake.to_str().unwrap(), workdir, input);
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
    let fake = super::super::tests::executable(dir.path(), "dsh", WARNING_DSH);
    let workdir = dir.path().to_str().unwrap();
    let (_, invocation) = invoke_hands(
        fake.to_str().unwrap(),
        workdir,
        sealed(SealedBoundary::Namespace, Some("read-only")),
    );
    let problem = invocation
        .err()
        .expect("the measured activation warning refuses the invocation");
    assert_eq!(problem, ReportRefusal::NotActivated.to_string());
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

/// Every composed-tree refusal names itself in full, once (#896 round 3;
/// decision 0071 ruling 8).
#[test]
fn every_composition_refusal_names_itself_in_full() {
    use super::composition::Composition;
    let cases = [
        (
            Composition::Unmeasured {
                id: "ambient".into(),
                name: "@deepseek-ai/dsh-ambient".into(),
            },
            "refusing to invoke the dsh driver: the composed dsh tree carries row 'ambient' of \
             package '@deepseek-ai/dsh-ambient', which the measured hands composition (dsh \
             0.2.1-rc.2) does not name, so a seat would be served a row nobody measured (#896 D1)",
        ),
        (
            Composition::NoHands,
            "refusing to invoke the dsh driver: the composed dsh tree carries no 'brokkr-hands' \
             row, so the seat's one effectful tool would not be served (#896 D1)",
        ),
        (
            Composition::SecondClient {
                id: "ambient".into(),
            },
            "refusing to invoke the dsh driver: the composed dsh tree carries a second \
             dsh-mcp-client row 'ambient', so the seat would be served a second effectful server \
             beside Brokkr's (#896 D1)",
        ),
        (
            Composition::Enabled {
                id: "tool-web".into(),
            },
            "refusing to invoke the dsh driver: the composed dsh tree carries row 'tool-web' \
             enabled, which the hands rows disable, so the seat would keep an effectful tool \
             beside the box (#896 D1)",
        ),
        (
            Composition::Repeated {
                id: "brokkr-hands".into(),
            },
            "refusing to invoke the dsh driver: the composed dsh tree names row 'brokkr-hands' \
             twice, so no row it reads can be trusted (#896 D1)",
        ),
        (
            Composition::Unreadable,
            "refusing to invoke the dsh driver: the composed dsh tree cannot be read as the one \
             sequence of id and package rows dsh writes, so the seat is admitted on no measured \
             composition (#896 D1)",
        ),
        (
            Composition::DumpFailed {
                status: "exit 2".into(),
            },
            "refusing to invoke the dsh driver: `dsh --dump-config` failed (exit 2), so the \
             composed tree could not be read and the seat is not launched (#896 D1)",
        ),
    ];
    for (refusal, text) in cases {
        assert_eq!(refusal.to_string(), text);
    }
}
