//! Whether a seat may start (#372, #355): the refusals that come before any
//! launch, and so before an `accepted` could be sent.

use serde_json::Value;

use super::{render_prompt, AdapterKind, StartRefusal};
use crate::overrides::OverrideError;

/// The prompt a seat starts on, or why it launches nothing: a start with
/// no correlation (#372), an override set only by its retired spelling
/// (#355), or a model seat whose charter cannot be read.
pub(super) fn start_prompt(
    effect_id: &str,
    attempt_id: &str,
    gate: Result<(), OverrideError>,
    input: &Value,
    kind: AdapterKind,
) -> Result<String, StartRefusal> {
    if effect_id.is_empty() {
        return Err(StartRefusal::MissingCorrelation("effect_id"));
    }
    if attempt_id.is_empty() {
        return Err(StartRefusal::MissingCorrelation("attempt_id"));
    }
    gate.map_err(StartRefusal::RetiredOverride)?;
    render_prompt(input, kind)
}
