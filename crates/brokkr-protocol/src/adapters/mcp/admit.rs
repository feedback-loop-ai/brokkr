//! Each invocation shape's sealed assessment read for the harness it would
//! serve (SI2), moved here from `mcp.rs`, which sits at its production
//! ceiling, when Codex's declared verdicts came to be read as every other
//! adapter's are (decision 0065 slice two, U9c2).

use super::{Invocation, ServingShape, StrictCause};
use crate::native_controls::{SealedAssessment, SealedIsolation, SealedServers};

impl SealedIsolation {
    /// Whether `provider` may be served its launch's shape: cold, or where
    /// it is `offered` a rejoin, that rejoin's cold replacement
    /// ([`SealedIsolation::admit`]), with the set this intent seals. dsh's
    /// hands ride its overlay (#896), beside which no engine-only home is
    /// measured, so a set beside the empty one is never admitted for it.
    /// The one admission a launch's isolated configuration is built under
    /// ([`super::isolated`]) and a dsh launch's home is decided by
    /// ([`crate::dsh_served`], #717).
    pub(crate) fn admits(&self, provider: &'static str, offered: bool) -> Result<(), StrictCause> {
        let invocation = match offered {
            true => Invocation::Replacement,
            false => Invocation::Cold,
        };
        self.admit(provider, invocation)?;
        let hands = self.servers != SealedServers::Empty;
        match hands && ServingShape::of(provider) == Some(ServingShape::Dsh) {
            true => Err(StrictCause::Unmeasured {
                provider: provider.into(),
                shape: "hands".into(),
            }),
            false => Ok(()),
        }
    }

    /// Whether `provider` may be served as `invocation`: its assessment
    /// must be measured, and the engine must build a qualified mechanism
    /// for its harness: Claude's strict configuration, dsh's engine-only
    /// home, or Codex's isolation options and store-denying profile (U9c2).
    /// A measured claim for a harness with none is missing evidence, never a
    /// fallback. dsh's route is judged separately ([`crate::dsh_served`]),
    /// once its model pin is read.
    pub(super) fn admit(
        &self,
        provider: &'static str,
        invocation: Invocation<'_>,
    ) -> Result<(), StrictCause> {
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
        let unmeasured = || StrictCause::Unmeasured {
            provider: provider.into(),
            shape: shape.to_string(),
        };
        match assessment {
            SealedAssessment::Unsupported(reason) => {
                return Err(StrictCause::Unsupported {
                    provider: provider.into(),
                    reason: reason.clone(),
                })
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
