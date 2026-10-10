//! The route a built-in driver serves a concrete model id on, one rule
//! for the driver that serves it and the compile that pins it (#430):
//! the id's own route, its prefix before the FIRST `/` (decision 0036
//! ruling 2), else the driver's default route, which only dsh has.

use super::AdapterKind;

/// The provider row dsh's headless profile boots its agent on when the
/// pinned model names none. A patch overlay replaces the targeted row's
/// WHOLE config (dsh-base's own words), so the overlay that pins a
/// model must restate the provider or the boot loses it.
const DSH_PROVIDER: &str = "deepseek-official";

/// A concrete model id split on its FIRST `/`: the route it names, if
/// any, and the id that route serves.
pub fn split_route(model_id: &str) -> (Option<&str>, &str) {
    match model_id.split_once('/') {
        Some((route, id)) => (Some(route), id),
        None => (None, model_id),
    }
}

/// The route dsh serves `model_id` on: its own, else [`DSH_PROVIDER`].
pub(super) fn dsh_route(model_id: &str) -> &str {
    split_route(model_id).0.unwrap_or(DSH_PROVIDER)
}

impl AdapterKind {
    /// The route this driver serves `model_id` on, where it has one.
    pub fn served_route(self, model_id: &str) -> Option<&str> {
        match self {
            AdapterKind::Dsh => Some(dsh_route(model_id)),
            AdapterKind::Claude
            | AdapterKind::Lanetally
            | AdapterKind::Codex
            | AdapterKind::Exec => split_route(model_id).0,
        }
    }
}

#[cfg(test)]
mod tests;
