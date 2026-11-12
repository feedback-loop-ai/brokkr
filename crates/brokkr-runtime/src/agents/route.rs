//! A route's overlay declaration, the fact the adapter owns and the
//! engine composes from (decision 0056's 2026-09-14 note as amended by
//! issue #1052 ask 2).

use std::path::{Path, PathBuf};

/// A route's overlay, declared once on the adapter that owns the route
/// (decision 0056's 2026-09-14 note as amended; issue #1052 ask 2). The
/// loader resolves and digests the file; `path` is the canonical file the
/// declaration names, `file` is the path as written relative to the
/// adapter's own directory, and `sha256` is the digest the driver's one
/// read must reproduce. The engine composes `--patch` from this fact and
/// binds it to the declared digest; the driver still reads the file once
/// and checks the digest itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclaredRouteOverlay {
    pub(crate) file: String,
    pub(crate) path: PathBuf,
    pub(crate) sha256: String,
}

impl DeclaredRouteOverlay {
    /// The `--patch` value for a run whose working directory is
    /// `workdir`: the declared file's path relative to it, or the
    /// declaration's own adapter-relative spelling when the file is
    /// outside that tree. The fallback still composes a `--patch`, so the
    /// driver's absent-binding refusal (`PatchUnbound`) is what stops the
    /// start, never a quiet seat with no overlay at all.
    ///
    /// `workdir` is expected canonical, as `Bundle::compile_as` makes it;
    /// this function reads nothing.
    pub(crate) fn patch_value(&self, workdir: &Path) -> String {
        // An empty working directory names no tree to be relative to; the
        // adapter-relative spelling is the fallback, as it is outside the
        // tree. `strip_prefix("")` would otherwise succeed on an absolute
        // path and return it with a doubled root.
        if workdir.as_os_str().is_empty() {
            return self.file.clone();
        }
        let relative = self.path.strip_prefix(workdir).ok().and_then(|path| {
            path.components()
                .map(|component| component.as_os_str().to_str())
                .collect::<Option<Vec<_>>>()
                .map(|parts| parts.join("/"))
        });
        relative.unwrap_or_else(|| self.file.clone())
    }
}

/// Whether one composed segment is the route overlay's `--patch` pair
/// (#1052 ask 2): exactly the flag and a non-empty value. It is a
/// template contribution that is neither a model/effort pin nor a
/// permission control, so the pin checks pass it by; any other shape is
/// still judged as one of those.
pub(crate) fn is_patch_pair(argv: &[String]) -> bool {
    matches!(argv, [flag, value] if flag == "--patch" && !value.is_empty())
}
