//! Decision 0042's addendum of 2026-09-06: a capability spec names the
//! changes that wrote it. The archive step appends one `## Provenance`
//! line per capability it touched, and this test keeps the trail two-way
//! after the one-time backfill: every living capability carries a
//! non-empty provenance list, every change it names resolves to a
//! directory under `openspec/changes/archive/`, and that change's deltas
//! actually touched the capability. An empty list, a dangling name, or a
//! provenance section that is not held at the end of the file fails here.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// The archived directory a provenance line names: the first backticked
/// token on the line. The archive instruction renders exactly one such
/// token per line, so a line with none is malformed.
fn named_change(line: &str, capability: &str) -> String {
    let after = line
        .strip_prefix("- ")
        .unwrap_or_else(|| panic!("{capability}: provenance line is not a list item: {line}"));
    let start = after
        .find('`')
        .unwrap_or_else(|| panic!("{capability}: provenance line names no change: {line}"));
    let rest = &after[start + 1..];
    let end = rest
        .find('`')
        .unwrap_or_else(|| panic!("{capability}: provenance line has no closing backtick: {line}"));
    let name = &rest[..end];
    assert!(
        !name.is_empty() && !name.contains(['/', '\\']) && name != "." && name != "..",
        "{capability}: '{name}' is not an archived directory name"
    );
    name.to_string()
}

/// The capabilities a change touched, read from its own delta directory.
fn touched_capabilities(change: &Path) -> BTreeSet<String> {
    let specs = change.join("specs");
    let mut touched = BTreeSet::new();
    let entries =
        std::fs::read_dir(&specs).unwrap_or_else(|error| panic!("{}: {error}", specs.display()));
    for entry in entries {
        let path = entry.expect("delta entry").path();
        if path.join("spec.md").is_file() {
            touched.insert(
                path.file_name()
                    .expect("delta directory name")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    touched
}

#[test]
fn every_capability_names_the_archived_changes_that_wrote_it() {
    let root = workspace();
    let specs = root.join("openspec/specs");
    let archive = root.join("openspec/changes/archive");

    let mut capabilities: Vec<PathBuf> = std::fs::read_dir(&specs)
        .unwrap_or_else(|error| panic!("{}: {error}", specs.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.join("spec.md").is_file())
        .collect();
    capabilities.sort();
    assert!(
        capabilities.len() >= 8,
        "the living capability tree is suspiciously small: {}",
        capabilities.len()
    );

    for capability_dir in capabilities {
        let capability = capability_dir
            .file_name()
            .expect("capability directory name")
            .to_string_lossy()
            .into_owned();
        let spec = capability_dir.join("spec.md");
        let text = std::fs::read_to_string(&spec)
            .unwrap_or_else(|error| panic!("{}: {error}", spec.display()));

        // The heading is held at the end of the file, so it is the last
        // `## ` section. Standing truth accumulates below it and is never
        // edited into a different meaning.
        let headings: Vec<&str> = text
            .lines()
            .filter(|line| line.starts_with("## "))
            .collect();
        assert_eq!(
            headings.last().copied(),
            Some("## Provenance"),
            "{capability}: the `## Provenance` heading is missing or not held at the end of {}",
            spec.display()
        );
        assert_eq!(
            headings
                .iter()
                .filter(|heading| **heading == "## Provenance")
                .count(),
            1,
            "{capability}: exactly one provenance section belongs in {}",
            spec.display()
        );

        let provenance: Vec<&str> = text
            .lines()
            .skip_while(|line| *line != "## Provenance")
            .skip(1)
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        assert!(
            !provenance.is_empty(),
            "{capability}: a promoted capability must name at least one change"
        );

        for line in provenance {
            let change = named_change(line, &capability);
            let change_dir = archive.join(&change);
            assert!(
                change_dir.is_dir(),
                "{capability}: provenance names '{change}', which is not under {}",
                archive.display()
            );
            assert!(
                touched_capabilities(&change_dir).contains(&capability),
                "{capability}: change '{change}' did not touch it, so the trail would be false"
            );
        }
    }
}
