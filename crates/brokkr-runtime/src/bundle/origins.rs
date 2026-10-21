//! Each composed layer's origin as a run's start records it and `recipes
//! show` prints it (#697 ruling 5): one derivation from the compile, so
//! the journal and the CLI cannot tell two chains.

use serde_json::{json, Value};

use super::Bundle;
use crate::places::{ComposedLayer, Origin};

impl Bundle {
    /// Every layer's origin and directory, leaf first, where the bundle
    /// composed more than one layer or stands on the personal shelf; `None`
    /// for one layer elsewhere, whose start records what it always did.
    pub fn layers(&self) -> Option<Vec<ComposedLayer>> {
        let personal = self.origins.contains(&Origin::Personal);
        let layered = self
            .roots
            .iter()
            .zip(&self.origins)
            .map(|(dir, origin)| ComposedLayer {
                origin: *origin,
                path: dir.display().to_string(),
            });
        (self.roots.len() > 1 || personal).then(|| layered.collect())
    }

    /// The `run/started` payload of a run of this bundle: its `feature`, the
    /// `manifest` it pins and the `charters` its start checked, and its
    /// [`Bundle::layers`] where it has them. The resume pin covers every
    /// layer's bytes through the manifest's chain; the origins are evidence.
    pub(crate) fn run_started(
        &self,
        feature: &str,
        manifest: &Value,
        charters: Value,
        entry: Option<&brokkr_core::entry::RunEntry>,
    ) -> Value {
        let mut started = json!({"feature": feature, "manifest": manifest, "charters": charters, "phases": self.machine.phases});
        if let Some(layers) = self.layers() {
            started["layers"] = json!(layers);
        }
        if let Some(entry) = entry {
            started["entry"] = json!(entry);
        }
        started
    }
}
