//! The tool-less gate (decision 0021 addendum 2026-11-22): a gate-class dsh
//! seat is served every pinned row disabled, dsh's sandbox read-only and no
//! MCP server, whatever `DSH_PERMISSION_MODE` names, and its composed tree
//! may carry no MCP client.

#![cfg(unix)]

use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Output};

use serde_json::{json, Value};

use super::super::composition::{composed_with, Composition, MEASURED_COMPOSITION};
use super::super::{gate_rows, sandbox, Hands, Mode, Refusal, Release, Shape, GATE_SEAT};
use super::{composed::with_sandbox_mode, dump_of, measured_dump, sealed, MEASURED_HEAD};
use crate::adapters::tests::dsh_version_shim;
use crate::env_guard::EnvGuard;
use crate::hands::GitFacts;
use crate::native_controls::SealedBoundary;

/// The gate overlay's own header, an independent literal.
const GATE_HEADER: &str = "\
# Written by `brokkr driver dsh` for one gate seat, served tool-less:
# every effectful tool row disabled, dsh's own sandbox read-only,
# and no MCP server at all.
";

/// The tool-less overlay on 0.2.1: the measured ten rows of
/// [`MEASURED_HEAD`] under the gate's header, read-only, and nothing after.
fn gate_overlay_0_2_1() -> String {
    let (_, rows) = MEASURED_HEAD
        .split_once("only effectful tool.\n")
        .expect("the measured head opens with the hands header");
    format!("{GATE_HEADER}{rows}read-only\n")
}

/// The facts a tool-less gate launch carries on `release`.
fn tool_less(release: Release) -> Hands {
    Hands {
        release,
        mode: Mode::ReadOnly,
        shape: Shape::ToolLess,
    }
}

#[test]
fn the_gate_overlay_is_the_measured_disabled_rows_with_no_server() {
    assert_eq!(gate_rows(Release::V0_2_1), Ok(gate_overlay_0_2_1()));
    assert_eq!(
        gate_rows(Release::V0_2_2Rc5),
        Ok(format!(
            "{}- id: tool-executor-ptc\n  disabled: true\n- id: tool-agent-team\n  disabled: true\n",
            gate_overlay_0_2_1()
        ))
    );
}

/// The removal control of the addendum's ruling W1: a seat the engine marks
/// gate class is served the tool-less overlay and stages no runner under
/// every `DSH_PERMISSION_MODE`, `danger-full-access` included. A gate that
/// inherited the variable would be served the runner's rows instead.
#[test]
fn a_gate_seat_is_served_tool_less_whatever_dsh_permission_mode_names() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let workdir = dir.path().to_str().unwrap();
    let dsh = dsh_version_shim(dir.path(), "dsh", "0.2.1-rc.2");
    let input = json!({"workdir": workdir, GATE_SEAT: true});
    for mode in [None, Some("workspace-write"), Some("danger-full-access")] {
        match mode {
            Some(mode) => env.set("DSH_PERMISSION_MODE", mode),
            None => env.remove("DSH_PERMISSION_MODE"),
        }
        let selection = sandbox(dsh.to_str().unwrap(), workdir, &GitFacts::default(), &input)
            .unwrap_or_else(|refusal| panic!("{mode:?}: {refusal}"));
        assert_eq!(
            selection.rows(),
            Some(gate_overlay_0_2_1().as_str()),
            "{mode:?}"
        );
        let (staged, hands) = selection.into_launch_parts();
        assert!(staged.is_none(), "{mode:?}");
        assert_eq!(hands, Some(tool_less(Release::V0_2_1)), "{mode:?}");
    }
}

/// A gate whose office declares boxed hands is refused, in its own words,
/// and so is a gate on a release nobody measured, before any row is
/// written.
#[test]
fn a_gate_with_boxed_hands_or_on_an_unmeasured_release_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let workdir = dir.path().to_str().unwrap();
    let refused = |bin: &str, input: &Value| sandbox(bin, workdir, &GitFacts::default(), input);
    let mut boxed = sealed(SealedBoundary::Namespace, Some("read-only"));
    boxed[GATE_SEAT] = json!(true);
    let refusal = refused("/nonexistent/dsh", &boxed).err();
    assert_eq!(refusal, Some(Refusal::GateHands));
    assert_eq!(
        Refusal::GateHands.to_string(),
        "refusing to invoke the dsh driver: a dsh gate is served tool-less, with every native \
         tool row disabled and no MCP server, so a gate whose office declares boxed hands is not \
         launched (decision 0021 addendum 2026-11-22)"
    );
    let newer = dsh_version_shim(dir.path(), "newer", "0.2.2");
    let bare = json!({"workdir": workdir, GATE_SEAT: true});
    assert_eq!(
        refused(newer.to_str().unwrap(), &bare).err(),
        Some(Refusal::Unmeasured {
            observed: "dsh '0.2.2'".into()
        })
    );
}

/// Judge `dump` as the composed-tree gate judges a launch carrying `hands`.
fn judged(dump: String, hands: Hands) -> Result<(), Composition> {
    composed_with(Command::new("dsh"), hands, move |_| {
        Ok(Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: dump.into_bytes(),
            stderr: Vec::new(),
        })
    })
}

/// A tool-less gate's tree owes no hands row and may carry no MCP client:
/// the measured rows less `brokkr-hands` are admitted, the measured rows
/// with it are refused by the client's id, and the same tree without it
/// is still refused for a boxed seat.
#[test]
fn a_gate_tree_carries_no_mcp_client_and_owes_no_hands_row() {
    let without: Vec<(&str, &str)> = (MEASURED_COMPOSITION.into_iter())
        .filter(|(id, _)| *id != "brokkr-hands")
        .collect();
    let without = with_sandbox_mode(&dump_of(Release::V0_2_1, &without), "read-only");
    assert_eq!(judged(without.clone(), tool_less(Release::V0_2_1)), Ok(()));
    let with = with_sandbox_mode(&measured_dump(), "read-only");
    let refusal = judged(with, tool_less(Release::V0_2_1)).unwrap_err();
    assert_eq!(
        refusal,
        Composition::GateClient {
            id: "brokkr-hands".into()
        }
    );
    assert_eq!(
        refusal.to_string(),
        "refusing to invoke the dsh driver: the composed dsh tree carries dsh-mcp-client row \
         'brokkr-hands', and a dsh gate is served tool-less, with no MCP server at all (decision \
         0021 addendum 2026-11-22)"
    );
    let boxed = super::boxed(Release::V0_2_1, Mode::ReadOnly);
    assert_eq!(judged(without, boxed), Err(Composition::NoHands));
}
