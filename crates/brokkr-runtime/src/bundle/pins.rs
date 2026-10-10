//! What each compiled site's candidates are served on (`run-manifest/v13`,
//! #430): the concrete model ids, read where the compile reads them, each
//! with the route its driver serves it on, and pinned beside the
//! candidate's capability record, so admission counts a
//! running run by what its own compile seated and never resolves it again
//! through another workspace's adapters.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use super::tier::FALLBACK_FLAG;
use super::{
    built_in_model_driver, inline_route_pin, route_pin, CapabilityAdapters, ModelPin, SiteFacts,
};
use crate::agents::Candidate;
use crate::capabilities::manifest::{ModelPins, Served};
use crate::capabilities::{harness_of, SiteCapabilities};

impl SiteFacts {
    /// Seal the site's capability outcomes beside its candidates' model
    /// pins, one for each outcome.
    pub(super) fn seal(&mut self, site: SiteCapabilities, pins: Vec<ModelPins>) {
        self.capabilities = Some(site);
        self.pins = pins;
    }
}

/// An agent-backed site's pins, one for each candidate.
pub(super) fn agents(adapters: CapabilityAdapters<'_>, chain: &[Candidate]) -> Vec<ModelPins> {
    (chain.iter())
        .map(|candidate| agent(adapters, &candidate.provider, &candidate.model))
        .collect()
}

/// One agent candidate's pins: the id its own adapter maps its model to,
/// served on the route that adapter's driver serves it on. The chain was
/// resolved against these adapters, so each maps; one that did not reads
/// as unreadable, never as no model.
fn agent(adapters: CapabilityAdapters<'_>, provider: &str, model: &str) -> ModelPins {
    let adapter = (adapters.adapters).and_then(|adapters| adapters.adapter(provider));
    let served = adapter.and_then(|adapter| {
        let id = adapter.models.get(model)?.clone();
        Some(Served::on(harness_of(&adapter.driver), id))
    });
    served.map_or(ModelPins::Unreadable(Vec::new()), |served| {
        ModelPins::Read(vec![served])
    })
}

/// An inline site's one candidate's pins. A built-in model driver's are
/// its model pin, read as decision 0040 reads it, and its fallback pin; a
/// pin that cannot be read as one concrete id makes the whole unreadable,
/// naming its flags. `exec` and a custom command take no model.
pub(super) fn inline(raw: &Value, adapters: CapabilityAdapters<'_>) -> Vec<ModelPins> {
    let Some(kind) = built_in_model_driver(raw) else {
        return exec();
    };
    let own = adapters
        .adapters
        .and_then(|adapters| adapters.adapter(kind));
    let (mut read, mut flags) = (Vec::new(), Vec::new());
    for pin in [inline_route_pin(raw, own), route_pin(raw, FALLBACK_FLAG)] {
        match pin {
            ModelPin::Concrete(id) => read.push(Served::on(kind, id)),
            ModelPin::Unreadable(on) => flags.extend(on),
            ModelPin::Absent => {}
        }
    }
    vec![match flags.is_empty() {
        true => ModelPins::Read(read),
        false => ModelPins::Unreadable(flags),
    }]
}

/// The pins of a site the engine runs through `exec`: no model.
pub(super) fn exec() -> Vec<ModelPins> {
    vec![ModelPins::Read(Vec::new())]
}

/// The manifest's per-site capability records, each candidate's beside
/// its model pins.
pub(super) fn capability_sites(sites: &BTreeMap<String, SiteFacts>) -> Map<String, Value> {
    (sites.iter())
        .map(|(label, facts)| {
            let site = (facts.capabilities.as_ref())
                .expect("the capability walk gave every compiled site an outcome");
            (label.clone(), site.pinned(&facts.pins))
        })
        .collect()
}

#[cfg(test)]
mod tests;
