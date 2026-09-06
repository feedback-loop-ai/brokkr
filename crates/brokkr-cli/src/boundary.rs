//! Which boundaries this machine offers (decision 0046 ruling 2,
//! generalising decision 0043 ruling 7): one probe table read by the
//! three verbs that start a run — `run`, `resume`, `rerun` — and by
//! `doctor`'s one `boundaries` line, so the refusal and the diagnosis
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
pub fn refuse_unboxable(bundle: &brokkr_runtime::Bundle, path: &OsStr) -> anyhow::Result<()> {
    if bundle.hands.is_empty() {
        return Ok(());
    }
    let seats: Vec<&String> = bundle.hands.keys().collect();
    let boundary = bundle.boundary;
    let offers = offered(&on_path(path));
    match &offers[&boundary] {
        Offer::Offered(found) if boundary == Boundary::Namespace => {
            let bwrap = PathBuf::from(found);
            for (site, spec) in &bundle.hands {
                brokkr_protocol::hands::overlay_supported(spec, &bwrap)
                    .map_err(|reason| anyhow::anyhow!("seat '{site}': {reason}"))?;
            }
            Ok(())
        }
        Offer::Offered(_) => Ok(()),
        Offer::MissingTool(tool) => anyhow::bail!(
            "the `{boundary}` boundary needs `{tool}` on PATH and none was found; the seats \
             {seats:?} declare hands and cannot run on this machine — the boundary is never \
             simulated (decision 0046 ruling 2)"
        ),
        Offer::Unbuilt {
            slice,
            needs,
            found,
        } => anyhow::bail!(
            "the `{boundary}` boundary is built by slice ({slice}) of decision 0046 ruling 6, \
             not by this engine ({}); the seats {seats:?} declare hands and cannot run under \
             it here — a realm may declare `harness` today (decision 0046 ruling 2)",
            readiness(needs, found.as_deref())
        ),
    }
}

/// The readiness fact beside an unbuilt boundary: the tool the slice
/// will need, and whether it is here already.
fn readiness(needs: &str, found: Option<&str>) -> String {
    match found {
        Some(found) => format!("{found} found"),
        None => format!("{needs} not on PATH"),
    }
}

/// `doctor`'s one line: the boundaries a run can start under here, and
/// for each it does not offer, why.
pub fn doctor_line(offers: &BTreeMap<Boundary, Offer>) -> String {
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
/// `namespace` with bubblewrap and under `harness` or `open` always, a
/// warning under `namespace` without bubblewrap, a warning under an
/// unbuilt boundary naming its slice. Returns whether the line is
/// healthy and its text.
pub fn hands_line(boundary: Boundary, offer: &Offer, hands: &[&str]) -> (bool, String) {
    let seats = if hands.is_empty() {
        "boxed seats".to_string()
    } else {
        format!("seats {hands:?} declare hands and")
    };
    match offer {
        Offer::Offered(detail) if boundary == Boundary::Namespace => {
            (true, format!("{detail} · {seats} can run"))
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
