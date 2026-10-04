//! Where a box's overlay binds write, their bubblewrap argv (#504), and
//! which bubblewrap can build them.

use std::path::Path;

use super::{namespace_path, overlay_supported_by, BindMode, HandsSpec};

/// The overlay floor on a spec, asking `reported` for bwrap's version
/// only when the spec binds an overlay: the launch runs the binary, and
/// `doctor` hands in what its probe found, so both judge by one rule.
pub fn overlay_supported_with(
    spec: &HandsSpec,
    bwrap: &Path,
    reported: impl FnOnce() -> String,
) -> Result<(), String> {
    if !spec.binds.iter().any(|bind| bind.mode == BindMode::Overlay) {
        return Ok(());
    }
    overlay_supported_by(&reported(), bwrap)
}

/// Where a box's overlay binds write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayWrites<'a> {
    /// Upper layers under this session directory, which outlives one
    /// call: a hands server runs one box per tool call, and a call reads
    /// what the last one wrote.
    Session(&'a Path),
    /// A tmpfs inside the box (`--tmp-overlay`), gone with it: a
    /// single-shot exec box's writes are discarded. An upper layer on the
    /// host's disk made the box's init, unmounting it, sync that whole
    /// disk, which on a loaded host outlasted the engine's settle bound.
    /// The tmpfs has no size of its own, so the kernel's default holds,
    /// half the host's RAM: past it a write fails with ENOSPC in the box,
    /// and nothing falls back to disk.
    Ram,
}

/// The argv of the overlay bind at `index` over `host`, its writes where
/// `writes` says. A session's layer is made before the box starts, and a
/// layer that cannot be made is an error, never a writable bind.
pub(super) fn overlay_argv(
    host: &Path,
    index: usize,
    writes: OverlayWrites<'_>,
) -> std::io::Result<Vec<String>> {
    let host_path = |path: &Path| path.to_string_lossy().into_owned();
    let mut argv = vec!["--overlay-src".to_string(), host_path(host)];
    match writes {
        OverlayWrites::Session(session) => {
            let layer = session.join("overlay").join(index.to_string());
            let upper = layer.join("upper");
            let work = layer.join("work");
            std::fs::create_dir_all(&upper)?;
            std::fs::create_dir_all(&work)?;
            argv.extend(["--overlay".to_string(), host_path(&upper), host_path(&work)]);
        }
        OverlayWrites::Ram => argv.push("--tmp-overlay".to_string()),
    }
    argv.push(namespace_path(host));
    Ok(argv)
}
