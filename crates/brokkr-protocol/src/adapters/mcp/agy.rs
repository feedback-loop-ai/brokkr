//! The engine's MCP set an agy launch is served (#1200), read from the
//! intent the launch's one admission checked ([`super::isolated`]) and
//! handed to Agy's arm of the set's builder
//! ([`crate::native_controls::mcp::agy`]). A child module because its parent
//! stands at the production file ceiling.

use std::path::Path;

use super::{engine_executable, intent, sealed_reach, Edge, Isolated, McpRefusal};
use crate::native_controls::mcp::agy::{served, Served};
use crate::native_controls::SealedBoundary;

/// The boundary an agy launch's sealed inputs on `edge` stand under, where
/// it boxes their hands, and `None` where they carry no boundary that does:
/// a driver run by hand, `harness` or `open`. What the sealed inputs
/// declare decides whether a launch's hands must be boxed, never what its
/// set was built with (#1224); inputs that cannot be read refuse as the
/// admission refuses them.
pub(in crate::adapters) fn agy_boxing(
    edge: &Edge<'_>,
) -> Result<Option<SealedBoundary>, McpRefusal> {
    let stands = intent(edge)?.and_then(|(sealed, _)| sealed.dialect.stands);
    Ok(stands.filter(|boundary| boundary.boxes()))
}

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

/// Whether an agy launch whose admission returned `isolated` from `edge` may
/// rejoin `shape`: allowed only when the start input carries no sealed intent
/// at all (a driver run by hand) or when `isolated` was built from that sealed
/// intent and admits `shape`. A sealed intent that the standing fence let
/// through unbuilt declines cold (`#1227`), because Antigravity CLI has no
/// pre-fence rejoin behaviour to fall back to.
pub(in crate::adapters) fn agy_resumes(edge: &Edge<'_>, isolated: &Isolated, shape: &str) -> bool {
    match &isolated.isolation {
        Some(_) => isolated.resumes("agy", shape),
        None => edge.sealed().is_none(),
    }
}
