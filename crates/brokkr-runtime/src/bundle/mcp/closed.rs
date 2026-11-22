//! A serving's SI2 verdict where its driver alone holds a boxed seat
//! closed (#1235; decision 0115). A child module because its parent nears
//! the production file ceiling.

use brokkr_protocol::adapters::{AdapterKind, StrictCause};

use super::McpIntent;
use crate::capabilities::McpFence;

/// `verdict` as `fence` admits it, except where `harness` serves `set`'s
/// boxed hands by a launch its driver composes whole: Qoder's driver serves
/// the box's hands only on a measured shape, and no compiled fragment keeps
/// its seat boxed in their place, so a missing measured shape refuses that
/// seat whichever way the fence stands. The standing fence (macOS) keeps
/// every other harness's verdict on its record as before, and every set
/// without boxed hands waits on the fence, Qoder's included.
pub(super) fn admitted(
    fence: McpFence,
    (set, harness): (Option<McpIntent>, &str),
    verdict: Option<StrictCause>,
) -> Result<(), StrictCause> {
    let boxed = match set {
        Some(McpIntent::Hands | McpIntent::NetworkedHands | McpIntent::Brokered) => true,
        Some(McpIntent::Empty | McpIntent::NoModelSurface) | None => false,
    };
    let composed_whole = match AdapterKind::parse(harness) {
        Some(AdapterKind::Qoder) => true,
        Some(
            AdapterKind::Claude
            | AdapterKind::Codex
            | AdapterKind::Dsh
            | AdapterKind::Lanetally
            | AdapterKind::Exec,
        )
        | None => false,
    };
    match (boxed && composed_whole, verdict) {
        (true, Some(cause)) => Err(cause),
        (true, None) => Ok(()),
        (false, verdict) => fence.admit(verdict),
    }
}
