//! Each invocation shape's sealed assessment read for the harness it would
//! serve (SI2), moved here from `mcp.rs`, which sits at its production
//! ceiling, when Codex's declared verdicts came to be read as every other
//! adapter's are (decision 0065 slice two, U9c2).

use std::path::Path;

use super::{hands_transport, Edge, Invocation, Isolated, ServingShape, StrictCause};
use crate::hands::Sealed;
use crate::native_controls::{SealedAssessment, SealedIsolation, SealedServers, Transport};

impl SealedIsolation {
    /// Whether `provider` may be served its launch's shape: cold, or where
    /// it is `offered` a rejoin, that rejoin's cold replacement
    /// ([`SealedIsolation::admit`]), with the set this intent seals. dsh's
    /// hands ride its overlay (#896), and an engine-only home was measured
    /// with the empty set and with its one boxed hands server; any other set
    /// beside that home is refused, because none is measured with it. The
    /// one admission a launch's isolated configuration is built under
    /// ([`super::isolated`]) and a dsh launch's home is decided by
    /// ([`crate::dsh_served`], #717).
    pub(crate) fn admits(&self, provider: &'static str, offered: bool) -> Result<(), StrictCause> {
        let invocation = match offered {
            true => Invocation::Replacement,
            false => Invocation::Cold,
        };
        self.admit(provider, invocation)?;
        if ServingShape::of(provider) == Some(ServingShape::Dsh) {
            match self.servers {
                SealedServers::Empty | SealedServers::Hands => {}
                SealedServers::Brokered(_) | SealedServers::NoModelSurface => {
                    return Err(StrictCause::Unmeasured {
                        provider: provider.into(),
                        shape: "hands".into(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Whether `provider` may be served as `invocation`: its assessment
    /// must be measured, and the engine must build a qualified mechanism
    /// for its harness: Claude's strict configuration, dsh's engine-only
    /// home, Codex's isolation options and store-denying profile (U9c2),
    /// Qoder's three-flag fragment (#1178), or Agy's engine HOME beside a
    /// workspace that carries none of Agy's configuration (#1200).
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
            Some(
                ServingShape::Claude
                | ServingShape::Codex
                | ServingShape::Dsh
                | ServingShape::Qoder
                | ServingShape::Agy,
            ) => Ok(()),
            None => Err(unmeasured()),
        }
    }
}

impl Isolated {
    /// The box's hands a launch is served from `edge`'s sealed inputs, bound
    /// to `brokkr`, `workdir` and the sealed `reach`: only where this
    /// admission passed with the hands set, which it proved to be the set
    /// those inputs declare, and none otherwise: a driver run by hand, an
    /// empty intent, or a shape the standing fence serves as before. The one
    /// home Qoder's set and Agy's read it from (#1178, #1200).
    pub(super) fn admitted_hands<'a>(
        &self,
        edge: &'a Edge<'_>,
        (brokkr, workdir): (&'a Path, &'a str),
        reach: &'a Sealed,
    ) -> Option<Transport<'a>> {
        let admitted = self.isolation.as_ref().map(|isolation| &isolation.servers);
        match (edge.sealed(), admitted) {
            (Some(Ok(sealed)), Some(SealedServers::Hands)) => {
                hands_transport(sealed, (brokkr, workdir), edge.input, reach)
            }
            _ => None,
        }
    }
}
