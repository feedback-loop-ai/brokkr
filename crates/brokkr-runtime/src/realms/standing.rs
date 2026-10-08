//! A world replayed from a pin, stood on the disk as it is now.

use std::path::Path;

use super::{first_crossing_failure, resolve_crossings, World, WorldError};

impl World {
    /// This world standing on the disk as it is NOW: its crossings
    /// resolved against `workspace` and refused exactly as
    /// [`World::verify_crossings`] refuses them, then carried as a world
    /// loaded off the disk carries them. A queued launch's world is
    /// replayed from its pin and resolves none (decision 0068; #430's
    /// C1), so it is stood on the disk at admission, and the run it starts
    /// pins the crossings it stood on, as a run started directly does.
    /// The disk is read once, so what is pinned is what was fenced.
    pub(crate) fn standing_on(mut self, workspace: &Path) -> Result<World, WorldError> {
        (self.crossings, self.reports) =
            resolve_crossings(&workspace.join(&self.source), &self.map);
        if let Some(failure) = first_crossing_failure(&self.reports) {
            return Err(failure);
        }
        Ok(self)
    }
}
