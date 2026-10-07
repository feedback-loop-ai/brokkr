//! One derivation, two surfaces (decision 0013; decision 0071 ruling 7):
//! what `brokkr-view` exports, the surfaces only paint (#351). No file
//! under `crates/brokkr-cli/src`, its Rust and `ui.html` alike, restates
//! a derivation the view took over. That the engine admits nothing by
//! the view is `layering`'s to hold: it reads the crate graph.

use std::path::Path;

use crate::workspace_root::workspace;

/// Each derivation the view took over, by a line a restatement of it
/// cannot avoid, and the one home a surface calls instead.
const RESTATED: [(&str, &str); 9] = [
    ("fn seat_costs", "brokkr_view::seat_costs"),
    ("fn first_divergence", "brokkr_view::first_divergence"),
    (
        "fn resolution_divergence",
        "brokkr_view::resolution_divergence",
    ),
    ("fn status_str", "Status::as_str"),
    ("Status::Stopped => \"stopped\"", "Status::as_str"),
    ("format!(\"{:?}\", state.cursor)", "brokkr_view::summary"),
    (
        "=> LegacyProvenance::LaneTally",
        "Participant::legacy_provenance",
    ),
    ("status == \"working\"", "Participant::working"),
    ("legacy_id: part.session_id", "brokkr_view::Subject::of"),
];

/// Every restatement in the files under `root`, as `path: why`, sorted.
/// A file the scan cannot read as text is a finding too: what it cannot
/// read, it cannot clear.
fn restatements(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let shown = path.strip_prefix(root).expect("under the root").display();
            match std::fs::read_to_string(&path) {
                Ok(text) => out.extend(
                    RESTATED
                        .iter()
                        .filter(|(line, _)| text.contains(line))
                        .map(|(line, home)| format!("{shown}: restates `{line}`; call {home}")),
                ),
                Err(error) => out.push(format!("{shown}: not readable as text ({error})")),
            }
        }
    }
    out.sort();
    out
}

#[test]
fn the_surfaces_restate_no_derivation_the_view_exports() {
    let src = workspace().join("crates/brokkr-cli/src");
    assert_eq!(restatements(&src), Vec::<String>::new());
}

/// The scan bites: each restatement it exists to refuse, planted, is
/// named with its home, and a file it cannot read is refused by name.
#[test]
fn the_scan_names_every_restatement_and_every_file_it_cannot_read() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("tui");
    std::fs::create_dir(&nested).unwrap();
    let planted: String = RESTATED
        .iter()
        .map(|(line, _)| format!("{line}\n"))
        .collect();
    std::fs::write(nested.join("state.rs"), planted).unwrap();
    std::fs::write(dir.path().join("ui.html"), b"\xff\xfe").unwrap();
    std::fs::write(
        dir.path().join("lib.rs"),
        "brokkr_view::seat_costs(&events)",
    )
    .unwrap();
    let mut expected: Vec<String> = RESTATED
        .iter()
        .map(|(line, home)| format!("tui/state.rs: restates `{line}`; call {home}"))
        .collect();
    expected.push("ui.html: not readable as text (stream did not contain valid UTF-8)".into());
    expected.sort();
    // A call through the view is no restatement.
    assert_eq!(restatements(dir.path()), expected);
    assert!(
        expected.contains(&"tui/state.rs: restates `fn status_str`; call Status::as_str".into())
    );
}
