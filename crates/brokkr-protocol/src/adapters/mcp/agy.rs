//! The engine's MCP set an agy launch is served (#1200), read from the
//! intent the launch's one admission checked ([`super::isolated`]) and
//! handed to Agy's arm of the set's builder
//! ([`crate::native_controls::mcp::agy`]). A child module because its parent
//! stands at the production file ceiling.

use std::path::Path;

use super::{engine_executable, sealed_reach, Edge, Isolated, McpRefusal};
use crate::native_controls::mcp::agy::{served, Served};

/// What the engine's set gives an agy launch in `workdir`, once `isolated`
/// admitted it from `edge`'s sealed inputs: the box's hands only where the
/// admission passed with the hands set ([`Isolated::admitted_hands`]), and
/// the empty set otherwise. A brokered set and a shape refused past the
/// fence never reach here: the admission refused them. Hands no document
/// value binds exactly refuse.
pub(in crate::adapters) fn agy_set(
    edge: &Edge<'_>,
    isolated: &Isolated,
    workdir: &str,
) -> Result<Served, McpRefusal> {
    agy_set_at(edge, isolated, workdir, &engine_executable())
}

/// [`agy_set`] for the engine's executable at `brokkr`, so a path no
/// document value represents is a plain test.
pub(super) fn agy_set_at(
    edge: &Edge<'_>,
    isolated: &Isolated,
    workdir: &str,
    brokkr: &Path,
) -> Result<Served, McpRefusal> {
    let reach = sealed_reach(edge.input);
    served(isolated.admitted_hands(edge, (brokkr, workdir), &reach))
        .ok_or(McpRefusal::NotSealed { provider: "agy" })
}
