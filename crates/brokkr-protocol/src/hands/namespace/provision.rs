//! The server box as provisioning observes it (decision 0065 slice two,
//! U7c0b; MB3): the same box [`ServerBox::prepare`] builds, observed
//! through the same path, its write exclusion judged against the writers
//! the plan seals, taken as confined as the plan seals them, and not
//! against the observing process chain. The engine is no managed writer,
//! so its own privilege proves nothing of theirs. This confers nothing at
//! admission: there the broker's observer reads the real managed writers,
//! adds the sealed ones, proves their confinement and compares what it
//! observed with the sealed sources and writers. It lies beside
//! namespace.rs, which stands at its 800-line ceiling.

use super::{ServerBox, ServerProfile, ServerProgram};
use crate::broker::{Refusal, Sources};

impl ServerBox {
    /// The bounded sources of the box `program` is prepared in under
    /// `profile`, as provisioning seals them, or the first of MB3's causes
    /// in [`ServerBox::prepare`]'s order: its write-exclusion proof holds
    /// against `profile.writers` alone, confined, beside the kernel's
    /// overflow uid, so none sealed leaves identity unprotected wherever
    /// a support file is linked or unreadable. The box itself, its handles
    /// and its scratch are let go: the broker observes again before any
    /// lookup. Off Linux no box stands, as `prepare` refuses.
    pub fn provision(
        program: &ServerProgram,
        profile: &ServerProfile<'_>,
    ) -> Result<Sources, Refusal> {
        #[cfg(target_os = "linux")]
        let server = {
            let host = super::sources::Host::provisioning(profile.writers);
            ServerBox::prepare_with(program, profile, &host)?
        };
        #[cfg(not(target_os = "linux"))]
        let server = ServerBox::prepare(program, profile)?;
        Ok(server.sources)
    }
}
