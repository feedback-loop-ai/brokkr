//! `doctor`'s two boundary lines, painted from the one probe table the
//! launch refuses by ([`brokkr_runtime::boundary`], decision 0046 ruling
//! 2): the refusal and the diagnosis read one table, so they cannot
//! disagree (design DD17).

use std::collections::BTreeMap;
use std::path::Path;

use brokkr_core::realms::Boundary;
use brokkr_protocol::hands::{overlay_supported_with, HandsSpec};
#[cfg(test)]
use brokkr_runtime::boundary::refuse_unboxable;
pub(crate) use brokkr_runtime::boundary::{offered, on_path, Offer};
use brokkr_runtime::boundary::{overlays_buildable, readiness};

/// `doctor`'s one line: the boundaries a run can start under here, and
/// for each it does not offer, why.
pub(crate) fn doctor_line(offers: &BTreeMap<Boundary, Offer>) -> String {
    let mut offered = Vec::new();
    let mut withheld = Vec::new();
    for (boundary, offer) in offers {
        match offer {
            Offer::Offered(detail) if *boundary == Boundary::Namespace => {
                offered.push(format!("{boundary} (bubblewrap {detail})"));
            }
            Offer::Offered(_) => offered.push(boundary.to_string()),
            Offer::MissingTool(tool) => {
                withheld.push(format!("{boundary} needs {tool} on PATH (not found)"));
            }
            Offer::Unbuilt {
                slice,
                needs,
                found,
            } => withheld.push(format!(
                "{boundary} built by slice ({slice}) of decision 0046 ruling 6 ({})",
                readiness(needs, found.as_deref())
            )),
        }
    }
    format!("{} offered; {}", offered.join(" · "), withheld.join("; "))
}

/// `doctor`'s `hands` line, judged against the boundary the discovered
/// realm declares rather than against bubblewrap alone: healthy under
/// `namespace` with a bubblewrap that builds every seat's overlays and
/// under `harness` or `open` always, a warning under `namespace` without
/// bubblewrap or with one older than the overlay floor the launch
/// refuses by ([`overlays_buildable`]), a warning under an unbuilt
/// boundary naming its slice. Returns whether the line is healthy and
/// its text.
pub(crate) fn hands_line(
    boundary: Boundary,
    offer: &Offer,
    hands: &BTreeMap<String, HandsSpec>,
) -> (bool, String) {
    let seats = if hands.is_empty() {
        "boxed seats".to_string()
    } else {
        format!("seats {:?} declare hands and", Vec::from_iter(hands.keys()))
    };
    match offer {
        Offer::Offered(detail) if boundary == Boundary::Namespace => {
            let reported = || detail.clone();
            match overlays_buildable(hands, |spec| {
                overlay_supported_with(spec, Path::new("bwrap"), reported)
            }) {
                Ok(()) => (true, format!("{detail} · {seats} can run")),
                Err(refusal) => (
                    false,
                    format!("{detail} · {seats} will refuse to spawn: {refusal}"),
                ),
            }
        }
        Offer::Offered(_) => (
            true,
            format!("{seats} can run under `{boundary}` — no box of Brokkr's is built there"),
        ),
        Offer::MissingTool(tool) => (
            false,
            format!(
                "bubblewrap ({tool}) not found — {seats} will refuse to spawn under `{boundary}`"
            ),
        ),
        Offer::Unbuilt { slice, .. } => (
            false,
            format!(
                "{seats} will refuse to spawn: `{boundary}` is built by slice ({slice}) of \
                 decision 0046 ruling 6, not by this engine"
            ),
        ),
    }
}

#[cfg(test)]
mod tests;
