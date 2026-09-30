//! Which boundaries this machine offers (decision 0046 ruling 2,
//! generalising decision 0043 ruling 7): one probe table, read by the
//! launch every run starts or continues through ([`crate::launch`]) and
//! by `doctor`'s one `boundaries` line, so the refusal and the diagnosis
//! cannot disagree (design DD17).
//!
//! A boundary is never simulated. `namespace` needs bubblewrap on the
//! search path (and 0.10 or newer for a spec with overlay binds, as
//! decision 0043 read); `seatbelt` needs `sandbox-exec` and `container`
//! a container engine, and both refuse on every machine until decision
//! 0046 ruling 6's slices (ii) and (iii) build them — their tool is
//! reported as a readiness fact; `harness` and `open` ask nothing of the
//! machine and are always offered.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::PathBuf;

use brokkr_core::realms::{Boundary, BOUNDARIES};
use thiserror::Error;

use crate::Bundle;

/// What this machine says about one boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offer {
    /// A run may start under it; the detail names what was found
    /// (`bwrap`'s path or version) or that nothing was needed.
    Offered(String),
    /// The tool the boundary needs is not on the search path.
    MissingTool(&'static str),
    /// Named, pinned and admitted at compile, built by a later slice:
    /// the tool it will need, and what the search path holds today.
    Unbuilt {
        slice: &'static str,
        needs: &'static str,
        found: Option<String>,
    },
}

/// Why a bundle whose seats box their hands cannot start on this machine
/// (decision 0046 ruling 2).
#[derive(Debug, Error)]
pub enum Unboxable {
    /// The boundary's tool is not on the search path.
    #[error(
        "the `{boundary}` boundary needs `{tool}` on PATH and none was found; the seats \
         {seats:?} declare hands and cannot run on this machine — the boundary is never \
         simulated (decision 0046 ruling 2)"
    )]
    MissingTool {
        boundary: Boundary,
        tool: &'static str,
        seats: Vec<String>,
    },
    /// The boundary is built by a later slice of decision 0046 ruling 6.
    #[error(
        "the `{boundary}` boundary is built by slice ({slice}) of decision 0046 ruling 6, \
         not by this engine ({readiness}); the seats {seats:?} declare hands and cannot run \
         under it here — a realm may declare `harness` today (decision 0046 ruling 2)"
    )]
    Unbuilt {
        boundary: Boundary,
        slice: &'static str,
        readiness: String,
        seats: Vec<String>,
    },
    /// A seat's overlay bind asks more of bubblewrap than the one found.
    #[error("seat '{site}': {reason}")]
    Overlay { site: String, reason: String },
}

/// The table, from one lookup: `look(tool)` answers with a detail when
/// the tool is available (its path on a search path, its version under
/// `doctor`'s probe) and `None` when it is not.
pub fn offered(look: &dyn Fn(&str) -> Option<String>) -> BTreeMap<Boundary, Offer> {
    BOUNDARIES
        .into_iter()
        .map(|boundary| {
            let offer = match boundary {
                Boundary::Namespace => match look("bwrap") {
                    Some(detail) => Offer::Offered(detail),
                    None => Offer::MissingTool("bwrap"),
                },
                Boundary::Seatbelt => Offer::Unbuilt {
                    slice: "ii",
                    needs: "sandbox-exec",
                    found: look("sandbox-exec").map(|detail| format!("sandbox-exec {detail}")),
                },
                Boundary::Container => Offer::Unbuilt {
                    slice: "iii",
                    needs: "docker or podman",
                    found: look("docker")
                        .map(|detail| format!("docker {detail}"))
                        .or_else(|| look("podman").map(|detail| format!("podman {detail}"))),
                },
                Boundary::Harness | Boundary::Open => {
                    Offer::Offered("nothing of Brokkr's stands".to_string())
                }
            };
            (boundary, offer)
        })
        .collect()
}

/// A lookup over one search path: the tool's path when a regular file of
/// that name is on it.
pub fn on_path(path: &OsStr) -> impl Fn(&str) -> Option<String> + '_ {
    move |tool: &str| {
        std::env::split_paths(path)
            .map(|dir| dir.join(tool))
            .find(|candidate| candidate.is_file())
            .map(|found| found.display().to_string())
    }
}

/// Decision 0046 ruling 2: a bundle whose seats box their hands refuses
/// to start under a boundary this machine cannot build, naming the
/// boundary, what it needs and what was found, and the seats — before
/// any journal row is written or a seat spawned. A bundle that boxes
/// nothing asks nothing of the machine.
pub fn refuse_unboxable(bundle: &Bundle, path: &OsStr) -> Result<(), Unboxable> {
    if bundle.hands.is_empty() {
        return Ok(());
    }
    let seats: Vec<String> = bundle.hands.keys().cloned().collect();
    let boundary = bundle.boundary;
    let offers = offered(&on_path(path));
    match &offers[&boundary] {
        Offer::Offered(found) if boundary == Boundary::Namespace => {
            let bwrap = PathBuf::from(found);
            for (site, spec) in &bundle.hands {
                brokkr_protocol::hands::overlay_supported(spec, &bwrap).map_err(|reason| {
                    Unboxable::Overlay {
                        site: site.clone(),
                        reason,
                    }
                })?;
            }
            Ok(())
        }
        Offer::Offered(_) => Ok(()),
        Offer::MissingTool(tool) => Err(Unboxable::MissingTool {
            boundary,
            tool,
            seats,
        }),
        Offer::Unbuilt {
            slice,
            needs,
            found,
        } => Err(Unboxable::Unbuilt {
            boundary,
            slice,
            readiness: readiness(needs, found.as_deref()),
            seats,
        }),
    }
}

/// The readiness fact beside an unbuilt boundary: the tool the slice
/// will need, and whether it is here already.
pub fn readiness(needs: &str, found: Option<&str>) -> String {
    match found {
        Some(found) => format!("{found} found"),
        None => format!("{needs} not on PATH"),
    }
}
