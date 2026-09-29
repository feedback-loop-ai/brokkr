//! The console's source as the source-level proofs in `tests.rs` read
//! it. #288 split `tui.rs` into one module per seam, so the read is the
//! root and every production module, and a proof here refuses a module
//! the read leaves out.

use std::path::Path;

/// The files themselves are the evidence: a renderer that cannot name a
/// store cannot write to one, and a widget layer with exactly two
/// sanitized constructors cannot be handed raw journal text.
pub(super) const SOURCE: &str = concat!(
    include_str!("../tui.rs"),
    include_str!("footer.rs"),
    include_str!("glyphs.rs"),
    include_str!("keys.rs"),
    include_str!("layout.rs"),
    include_str!("movement.rs"),
    include_str!("painter.rs"),
    include_str!("panes.rs"),
    include_str!("participant.rs"),
    include_str!("seats.rs"),
    include_str!("state.rs"),
    include_str!("style.rs"),
    include_str!("terminal.rs"),
);

/// Every entry under `src/tui/` is a test file, the snapshot directory,
/// or a production module whose whole text [`SOURCE`] holds. Anything
/// else — a module added and not read, a nested directory, a file of a
/// kind this does not know — is named, rather than passed unread.
#[test]
fn the_tui_source_reads_every_production_module() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tui");
    let mut unread = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        match (path.is_dir(), name.strip_suffix(".rs")) {
            (true, _) if name == "snapshots" => {}
            (false, Some(stem)) if stem == "tests" || stem.ends_with("_tests") => {}
            (false, Some(_)) if SOURCE.contains(&std::fs::read_to_string(&path).unwrap()) => {}
            _ => unread.push(name),
        }
    }
    unread.sort();
    assert_eq!(
        unread,
        Vec::<String>::new(),
        "modules the source read skips"
    );
}
