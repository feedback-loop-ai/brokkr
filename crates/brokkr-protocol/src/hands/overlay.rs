//! Where a box's overlay binds write, and their bubblewrap argv (#504).

use std::path::Path;

use super::namespace_path;

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
