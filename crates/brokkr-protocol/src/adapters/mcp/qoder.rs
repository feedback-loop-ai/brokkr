//! The engine's MCP set a qoder launch is served (#1178), read from the
//! intent the launch's one admission checked ([`super::isolated`]) and
//! handed to Qoder's arm of the set's builder
//! ([`crate::native_controls::mcp::qoder`]). A child module because its
//! parent stands at the production file ceiling.

use std::path::Path;

use super::{
    engine_executable, hands_transport, sealed_reach, sealed_servers, Edge, Isolated, McpRefusal,
};
use crate::native_controls::mcp::qoder::{served, Served};
use crate::native_controls::SealedServers;

/// What the engine's set gives a qoder launch in `workdir`, once
/// `isolated` admitted it from `edge`'s sealed inputs: the box's hands
/// only where the admission passed with the hands set, which it proved to
/// be the set those inputs declare, and the empty set otherwise: a driver
/// run by hand, an empty intent, or a shape without hands the standing
/// fence serves as before, which is never served ambient MCP or hands. A
/// brokered set and a shape refused past the fence never reach here: the
/// admission refused them. Hands no document value binds exactly refuse,
/// and so do boxed hands the standing fence served unadmitted: Qoder's
/// launch is composed whole by its driver, so no fragment of the compile's
/// keeps that seat boxed, and it is never launched with Qoder's own tools
/// in their place (#1235).
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
    let admitted = isolated
        .isolation
        .as_ref()
        .map(|isolation| &isolation.servers);
    let hands = match (edge.sealed(), admitted) {
        (Some(Ok(sealed)), Some(SealedServers::Hands)) => {
            hands_transport(sealed, (brokkr, workdir), edge.input, &reach)
        }
        (Some(Ok(sealed)), None) if sealed_servers(sealed) == SealedServers::Hands => {
            return Err(McpRefusal::Unboxed { provider: "qoder" });
        }
        _ => None,
    };
    served(hands).ok_or(McpRefusal::NotSealed { provider: "qoder" })
}
