//! An actual Claude rejoin of a compiled site (rebuild unit 21), moved out
//! of the parent suite beside the shape it is served as since U1d3 recorded
//! U0b's measured Claude replacement (decision 0065 slice two; SI2): a
//! launch whose replacement is measured rejoins only a shape sealed
//! measured, and is served the empty set's strict configuration there too.

use super::*;

/// Rebuild unit 21: a compiled Claude site dispatched as the engine
/// dispatches it ([`dispatched`]) and offered `session` under a SUPPORTED
/// assessment qualified for the markers dispatch wrote. The shipped Claude
/// assessment is unmeasured and declines every offer, so an actual rejoin
/// is handed one, as rebuild unit 15's Claude rows are, with its MCP shape
/// sealed measured, as the replacement is since U1d3 ([`rejoin_isolated`]).
/// The final command the driver would spawn, or its whole refusal.
#[cfg(unix)]
pub(super) fn claude_rejoined(
    bundle: &Bundle,
    label: &str,
    bin: &Path,
    session: &str,
) -> Result<Vec<String>, String> {
    claude_rejoined_at(bundle, label, 0, bin, session)
}

/// [`claude_rejoined`] for the site's candidate `candidate`, a primary or
/// a selected fallback (rebuild unit 21-fix-b).
#[cfg(unix)]
pub(super) fn claude_rejoined_at(
    bundle: &Bundle,
    label: &str,
    candidate: usize,
    bin: &Path,
    session: &str,
) -> Result<Vec<String>, String> {
    let work = brokkr_runtime::SeatClass::Work;
    let (extra, mut input) = dispatched(bundle, label, candidate, work, "/w")?;
    input["resume_context"] = json!({"assessment": {"boxed-workspace": {
        "status": "supported",
        "identity": {"version": "2.1.266", "applies_to": "2.1.266"},
        "classes": ["work"], "boundaries": [input["boundary"].clone()],
        "hands": input["hands"].clone(),
        "evidence": {"interface": "i", "restrictions": "r", "root": "o", "accounting": "a"},
        "limitations": [], "reason": null}}});
    measured_rejoin(&mut input);
    brokkr_protocol::adapters::claude_command(bin.to_str().unwrap(), &extra, Some(session), &input)
}

/// `input`'s sealed intent with the `boxed-workspace` rejoin measured, the
/// shape a supported Claude assessment here is handed under.
pub(super) fn measured_rejoin(input: &mut Value) {
    input["serving_inputs"]["isolation"]["resume"]["boxed-workspace"] = json!("measured");
}

/// A Claude rejoin of `session` whose shape is sealed measured, beside the
/// replacement U0b measured (U1d3): the empty set, then the selector.
pub(super) fn rejoin_isolated(session: &str) -> Vec<&str> {
    [ISOLATED, &["--resume", session]].concat()
}
