//! The engine's MCP set a qoder launch is served (#1178), read from the
//! intent the launch's one admission checked ([`super::isolated`]) and
//! handed to Qoder's arm of the set's builder
//! ([`crate::native_controls::mcp::qoder`]). A child module because its
//! parent stands at the production file ceiling.

use std::path::Path;

use super::{engine_executable, sealed_reach, Edge, Isolated, McpRefusal};
use crate::native_controls::mcp::qoder::{served, Served};

/// What the engine's set gives a qoder launch in `workdir`, once
/// `isolated` admitted it from `edge`'s sealed inputs: the box's hands
/// only where the admission passed with the hands set, which it proved to
/// be the set those inputs declare, and the empty set otherwise: a driver
/// run by hand, an empty intent, or a shape the standing fence serves as
/// before, which is never served ambient MCP or hands. A brokered set and
/// a shape refused past the fence never reach here: the admission refused
/// them. Hands no document value binds exactly refuse.
pub(in crate::adapters) fn qoder_set(
    edge: &Edge<'_>,
    isolated: &Isolated,
    workdir: &str,
) -> Result<Served, McpRefusal> {
    qoder_set_at(edge, isolated, workdir, &engine_executable())
}

/// [`qoder_set`] for the engine's executable at `brokkr`, so a path no
/// document value represents is a plain test.
pub(super) fn qoder_set_at(
    edge: &Edge<'_>,
    isolated: &Isolated,
    workdir: &str,
    brokkr: &Path,
) -> Result<Served, McpRefusal> {
    let reach = sealed_reach(edge.input);
    let hands = isolated.admitted_hands(edge, (brokkr, workdir), &reach);
    served(hands).ok_or(McpRefusal::NotSealed { provider: "qoder" })
}
