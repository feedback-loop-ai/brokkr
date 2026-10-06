//! The charter an inline site is bound to (rebuild unit 17; design D7), and
//! GP2's DATA check of what that site asks for over the very bytes the
//! binding pins (decision 0065 slice two, design D8, U3c).
//!
//! Every inline executable site — a seat, a panel member, a sequence step,
//! a select case or default, and each of them nested in another — is bound
//! here, by the layer that wrote it. The check is the loaded office's
//! ([`crate::agents::charter_data::check_bound`]); nothing here reads the
//! paragraph grammar, and the engine's later DATA reminder declares nothing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{
    bounded_reference, bounded_site, compose, dispatch_driver, missing_clause, owned_input,
    site_facts, CharterOwner, CharterPin, Charters, CompileError, InputFault, SiteFacts,
};

pub(super) fn parse_role(
    dir: &Path,
    what: &str,
    raw: &Value,
    charters: &Charters,
    sites: &mut BTreeMap<String, SiteFacts>,
) -> Result<PathBuf, CompileError> {
    let Some(role_rel) = raw.get("role").and_then(Value::as_str) else {
        if raw
            .pointer("/driver/command")
            .and_then(Value::as_array)
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .as_deref()
            .and_then(dispatch_driver)
            .as_deref()
            == Some("exec")
        {
            // A deterministic exec site is the script it names. It has no
            // model to instruct and therefore no charter to load into a
            // prompt; the prompt remains the typed run context and result
            // contract the script reads.
            return Ok(PathBuf::new());
        }
        return Err(CompileError::Invalid(format!(
            "seat '{what}' missing 'role'"
        )));
    };
    // Decision 0066 ruling 5: `dir` is the layer that WROTE this seat, so
    // an inherited, selected or nested body is judged against its own
    // declaring layer and the refusal names that layer's file. The charter
    // is read through a handle bound to its contained, regular target
    // (operator ruling 3; design D7), so a link out of the layer, a FIFO
    // or a replacement mid-read refuses here rather than at the seat. Every
    // refusal names the declaring file, the seat and the reference, bounded.
    // The verified buffer's digest is what the declaring layer's walk takes
    // for the charter's keys (rebuild unit 16-fix-b, F3), so it is kept.
    // Rebuild unit 17: the site is bound here to that layer, the key its
    // map pins the reference under, the target the read resolved and the
    // buffer's digest, so no later reader has to guess its owner.
    // Rebuild unit 18-fix-b (council F1, F2): read from the layer's directory
    // as its owner, so the pin carries that read's binding and who the owner
    // was when it was read, and the seal compares both.
    let source = dir.join("bundle.json");
    let (site, reference) = (bounded_site(what), bounded_reference(role_rel));
    match owned_input(dir, role_rel) {
        Ok(bound) => {
            let office = format!("{}: seat {site}", source.display());
            check_asks(what, raw, &bound.bytes, office, &reference)?;
            let role = dir.join(role_rel);
            let owner = CharterOwner::Layer {
                dir: dir.to_path_buf(),
                key: bound.held.binding.key.clone(),
            };
            let pin = CharterPin::of(owner, role_rel, role.clone(), &bound);
            site_facts(sites, what).charter = Some(pin);
            let read = compose::CharterRead::of(dir, site, reference, bound);
            charters.borrow_mut().push(read);
            Ok(role)
        }
        Err(InputFault::Missing(error)) => Err(CompileError::Invalid(format!(
            "{}: seat {site} names role {reference}, {}",
            source.display(),
            missing_clause(&error)
        ))),
        Err(InputFault::Place(place)) => Err(CompileError::Invalid(format!(
            "{}: seat {site} names role {reference}, {place}. {}",
            source.display(),
            place.remedy().unwrap_or(
                "A charter there could change what the seat is told without moving the bundle's \
                 identity, so it is refused; move it to a path the bundle pins, such as 'roles/' \
                 (decision 0066 ruling 5)"
            )
        ))),
    }
}

/// GP2 at one inline site: every capability its own map asks for, of either
/// strength and whether or not the realm grants or a candidate holds it, is
/// declared by the charter `bytes` the site is bound to. An inline site's
/// map is its office's asks, read as the capability pass reads it, so a map
/// that pass would refuse is refused here in its words.
fn check_asks(
    what: &str,
    raw: &Value,
    bytes: &[u8],
    office: String,
    reference: &str,
) -> Result<(), CompileError> {
    let asks = raw
        .get("capabilities")
        .map(|written| crate::capabilities::parse_requests(&format!("seat '{what}'"), written))
        .transpose()
        .map_err(CompileError::Invalid)?
        .unwrap_or_default();
    crate::agents::charter_data::check_bound(bytes, &asks, office, reference.to_string())
        .map_err(CompileError::Charter)
}
