//! A command's tokens expanded for this machine, moved out of `bundle.rs`
//! when the engine executable became a value the compile is handed (#349,
//! ask 8) rather than one it reads.

use std::path::Path;

use brokkr_protocol::native_controls::{Segment, TemplateExpectation};

use crate::agents::{Composition, Lowering};
use crate::argv::Executable;

/// `{brokkr}` is `engine`, the engine's own executable (built-in drivers),
/// and `./`-prefixed entries are bundle-relative. Composed argv is expanded
/// by this same function, which is why a resolved seat's command is an
/// inline seat's command by construction — and why the manifest record
/// carries names, never argv: the expansion is machine-local.
///
/// Public because a seat composed OUTSIDE a bundle — Muninn's, under
/// decision 0020 — must expand the same tokens from the same code
/// rather than from a second copy of this rule.
pub fn expand_command(engine: &Executable, dir: &Path, parts: &[String]) -> Vec<String> {
    parts
        .iter()
        .map(|part| {
            if part == "{brokkr}" {
                return brokkr_executable(engine.path());
            }
            match part.strip_prefix("./") {
                Some(rel) => dir.join(rel).to_string_lossy().into_owned(),
                None => part.clone(),
            }
        })
        .collect()
}

/// [`expand_command`] over a composition, one segment at a time (design
/// D5.7): each token keeps the origin of the segment that supplied it, and
/// because the expansion is token for token, the expanded segments are
/// exactly the expanded argv, in order. A lowering that never composed
/// has nothing to expand.
pub(crate) fn expand_lowering(engine: &Executable, dir: &Path, lowering: &Lowering) -> Lowering {
    match lowering {
        Lowering::Composed(composition) => Lowering::Composed(Composition {
            segments: composition
                .segments
                .iter()
                .map(|segment| Segment {
                    origin: segment.origin,
                    argv: expand_command(engine, dir, &segment.argv),
                })
                .collect(),
            // Rebuild unit 5c-fix2: the declared template is expanded as
            // its segment is, so the seal compares like with like.
            template: match &composition.template {
                TemplateExpectation::None => TemplateExpectation::None,
                TemplateExpectation::Declared(argv) => {
                    TemplateExpectation::Declared(expand_command(engine, dir, argv))
                }
            },
            ..composition.clone()
        }),
        other => other.clone(),
    }
}

/// `{brokkr}` as the compile spells it: the executable's path, or the bare
/// word `brokkr` when it could not be read.
pub(super) fn brokkr_executable(current: Option<&Path>) -> String {
    match current {
        Some(path) => path.to_string_lossy().into_owned(),
        None => "brokkr".to_string(),
    }
}
