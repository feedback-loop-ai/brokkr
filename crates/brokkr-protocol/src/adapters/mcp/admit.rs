//! Each invocation shape's sealed assessment read for the harness it would
//! serve (SI2), moved here from `mcp.rs`, which sits at its production
//! ceiling, when Codex's declared verdicts came to be read as every other
//! adapter's are (decision 0065 slice two, U9c2).

use super::{Invocation, McpRefusal, ServingShape, StrictCause};
use crate::native_controls::{SealedAssessment, SealedIsolation};

impl SealedIsolation {
    /// Whether `provider` may be served as `invocation`: its assessment
    /// must be measured, and the engine must build a qualified mechanism
    /// for its harness: Claude's strict configuration, dsh's engine-only
    /// home, or Codex's isolation options and store-denying profile (U9c2).
    /// A measured claim for a harness with none is missing evidence, never a
    /// fallback. dsh's route is judged separately ([`Isolated::dsh`]), once
    /// its model pin is read.
    ///
    /// [`Isolated::dsh`]: super::Isolated::dsh
    pub(super) fn admit(
        &self,
        provider: &'static str,
        invocation: Invocation<'_>,
    ) -> Result<(), McpRefusal> {
        let (assessment, shape) = match invocation {
            Invocation::Cold => (&self.cold, "cold"),
            Invocation::Replacement => (&self.replacement, "replacement"),
            Invocation::Resume(shape) => (
                self.resume
                    .get(shape)
                    .unwrap_or(&SealedAssessment::Unmeasured),
                shape,
            ),
        };
        let unmeasured = || {
            McpRefusal::Strict(StrictCause::Unmeasured {
                provider: provider.into(),
                shape: shape.to_string(),
            })
        };
        match assessment {
            SealedAssessment::Unsupported(reason) => {
                return Err(McpRefusal::Strict(StrictCause::Unsupported {
                    provider: provider.into(),
                    reason: reason.clone(),
                }))
            }
            SealedAssessment::Unmeasured => return Err(unmeasured()),
            SealedAssessment::Measured => {}
        }
        match ServingShape::of(provider) {
            Some(ServingShape::Claude | ServingShape::Codex | ServingShape::Dsh) => Ok(()),
            None => Err(unmeasured()),
        }
    }
}
