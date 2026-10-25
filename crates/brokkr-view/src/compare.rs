//! The derivations `brokkr compare` prints beside its seats' accounting:
//! where two runs diverged. Derived here once, so no surface aligns two
//! trails its own way (decision 0013; #351). The seats' accounting both
//! `brokkr costs` and `brokkr compare` print is brokkr-core's
//! `seat_costs`, folded there once for the journal and for the writer's
//! seat rows alike (#753), and exported from here.

use std::collections::BTreeMap;

use brokkr_core::EventEnvelope;
use serde::Serialize;

use crate::{run_view, Participant};

/// What one invocation site resolved to: the served pair as siblings
/// (decision 0046 ruling 3; design DD12) and the agent selection that
/// chose it, null where the journal names none. Compared whole, so a
/// boundary difference diverges exactly as a model difference does.
#[derive(Serialize, Clone, PartialEq)]
pub struct Resolution {
    model: String,
    boundary: String,
    selected: Option<Selected>,
}

/// The distinct agent-selection provenance a site's resolution carries.
#[derive(Serialize, Clone, PartialEq)]
struct Selected {
    agent: String,
    model: String,
    provider: String,
    chain_index: u64,
    fallback: bool,
}

impl Resolution {
    fn of(part: Participant) -> Resolution {
        Resolution {
            model: part.served.model.text,
            boundary: part.served.boundary.text,
            selected: part.provenance.map(|provenance| Selected {
                agent: provenance.agent,
                model: provenance.model,
                provider: provenance.provider,
                chain_index: provenance.chain_index,
                fallback: provenance.fallback,
            }),
        }
    }
}

/// What each run's invocation sites resolved to, keyed by participant
/// label so a panel member and a sequence step line up across runs. Read
/// off the run view's participants, so `compare` cannot describe a
/// fallback differently from every other readout.
pub fn resolutions(events: &[EventEnvelope]) -> BTreeMap<String, Resolution> {
    run_view(events, None)
        .participants
        .into_iter()
        .filter(|part| part.carried_from.is_none())
        .map(|part| (part.label.clone(), Resolution::of(part)))
        .collect()
}

/// One site's two sides where two runs resolved it differently; a side
/// that names no entry prints as null.
#[derive(Serialize)]
pub struct Sides<T> {
    a: Option<T>,
    b: Option<T>,
}

/// Where two decision trails first part: the position, and each side's
/// rule id or "park", or "end" where that trail stopped first.
#[derive(Serialize)]
pub struct FirstDivergence {
    index: usize,
    a: String,
    b: String,
}

/// AC-18: a model difference is a FIRST-CLASS divergence, not a
/// footnote — reported unconditionally, including when `same_recipe` is
/// true. Comparing pinned plans instead of what actually ran would hide
/// precisely the fallback this exists to expose, and an absence on one
/// side is itself the finding: one run named an agent and the other did
/// not. The comparison is structural over the whole entry, so a
/// boundary difference diverges exactly as a model difference does
/// (decision 0046 ruling 3) with no code written for it.
pub fn resolution_divergence<T: Clone + PartialEq>(
    a: &BTreeMap<String, T>,
    b: &BTreeMap<String, T>,
) -> BTreeMap<String, Sides<T>> {
    let mut sites: Vec<&String> = a.keys().chain(b.keys()).collect();
    sites.sort();
    sites.dedup();
    let mut out = BTreeMap::new();
    for site in sites {
        let (left, right) = (a.get(site), b.get(site));
        if left == right {
            continue;
        }
        out.insert(
            site.clone(),
            Sides {
                a: left.cloned(),
                b: right.cloned(),
            },
        );
    }
    out
}

/// None when the trails are equal; else the first differing position,
/// each side the rule id or "park". When one trail is a strict prefix of
/// the other, the shorter side renders as "end" at index = its length.
pub fn first_divergence(a: &[String], b: &[String]) -> Option<FirstDivergence> {
    let step = |trail: &[String], i: usize| -> String {
        trail.get(i).map_or("end", String::as_str).to_string()
    };
    (0..a.len().max(b.len()))
        .find(|&i| a.get(i) != b.get(i))
        .map(|i| FirstDivergence {
            index: i,
            a: step(a, i),
            b: step(b, i),
        })
}

#[cfg(test)]
pub(crate) mod tests;
