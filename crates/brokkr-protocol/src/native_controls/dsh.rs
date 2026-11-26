//! The measured DSH tool-web keys and the plan that supplies both.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{has_selection, refused, unconsumed, Controls, Piece, Refusal, Why};

/// One config key in DSH's tool-web row, in its declared state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Overlay {
    DshWebFetch(bool),
    DshWebSearch(bool),
}

impl Overlay {
    /// The capability this key switches.
    pub fn capability(self) -> &'static str {
        match self {
            Self::DshWebFetch(_) => "web-fetch",
            Self::DshWebSearch(_) => "web-search",
        }
    }

    /// Whether this key switches its tool on.
    pub fn enabled(self) -> bool {
        match self {
            Self::DshWebFetch(enabled) | Self::DshWebSearch(enabled) => enabled,
        }
    }
}

impl std::fmt::Display for Overlay {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key = match self {
            Self::DshWebFetch(_) => "fetch",
            Self::DshWebSearch(_) => "search",
        };
        write!(output, "tool-web config {key}: {}", self.enabled())
    }
}

/// The complete tool-web config. A standalone driver has no grant and uses
/// the OFF default. A managed launch must explicitly supply both states.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Web {
    fetch: bool,
    search: bool,
}

impl Web {
    pub(crate) fn row(self) -> String {
        let Self { fetch, search } = self;
        format!("- id: tool-web\n  config:\n    fetch: {fetch}\n    search: {search}\n")
    }

    pub(crate) fn any_enabled(self) -> bool {
        self.fetch || self.search
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum PlanRefusal {
    #[error("'overlay' is not a list of DSH web keys")]
    Overlay,
    #[error("{0}")]
    Provenance(String),
}

/// Finish decoding a known plan, checking the overlay before provenance.
pub(super) fn decode(plan: &Value, mut controls: Controls) -> Result<Controls, PlanRefusal> {
    controls.overlay = plan
        .get("overlay")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|_| PlanRefusal::Overlay)?
        .unwrap_or_default();
    controls.provenance = super::provenance(plan).map_err(PlanRefusal::Provenance)?;
    Ok(controls)
}

pub(super) fn argv_free(harness: &str, controls: &Controls) -> Result<(), Refusal> {
    if has_selection(controls) {
        return Err(unconsumed(harness, "a tool selection"));
    }
    if !controls.argv.is_empty() {
        return Err(unconsumed(harness, "managed arguments"));
    }
    Ok(())
}

pub(super) fn compose(controls: &Controls) -> Result<Web, Refusal> {
    let [first, second] = controls.overlay.as_slice() else {
        return Err(unanswered());
    };
    let (fetch, search) = match (*first, *second) {
        (Overlay::DshWebFetch(fetch), Overlay::DshWebSearch(search))
        | (Overlay::DshWebSearch(search), Overlay::DshWebFetch(fetch)) => (fetch, search),
        (Overlay::DshWebFetch(_), Overlay::DshWebFetch(_))
        | (Overlay::DshWebSearch(_), Overlay::DshWebSearch(_)) => return Err(unanswered()),
    };
    for key in [*first, *second] {
        let capability = key.capability();
        let named = |names: &[String]| names.iter().filter(|name| *name == capability).count();
        let (held, denied) = (named(&controls.held), named(&controls.denied));
        if (held, denied) != (usize::from(key.enabled()), usize::from(!key.enabled())) {
            return Err(unanswered());
        }
    }
    Ok(Web { fetch, search })
}

fn unanswered() -> Refusal {
    refused(Why::Unready, vec![Piece::Words(
        "the DSH overlay must switch fetch and search exactly once each, in the states its capability plan holds or denies",
    )])
}

#[cfg(test)]
#[path = "dsh/tests.rs"]
pub(crate) mod fixtures;
