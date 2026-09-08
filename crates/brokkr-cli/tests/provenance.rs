//! Decision 0042's addendum of 2026-09-06: a capability spec names the
//! changes that wrote it. The archive step appends one `## Provenance`
//! line per capability it touched, and these tests keep the trail two-way
//! after the one-time backfill. They walk it in both directions, because
//! one direction alone catches only half the ways it can break: from the
//! specs, a capability with an empty list or a dangling name fails; from
//! the archive, a fold that promoted a delta and forgot to append its
//! line fails. The line's shape is held too — it is a pointer, not a
//! summary, so a line that says more than which change and which day is
//! rationale leaking into standing truth.

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

/// The one shape the archive instruction renders, held literally:
///
/// ```text
/// - `<archived-directory-name>` — folded <YYYY-MM-DD>
/// ```
///
/// The archived directory a provenance line names is returned. Anything
/// else on the line is a refusal: `dialects/openspec/archive.md` tells
/// the smith the line names the change and nothing else, and a test that
/// accepted a trailing clause would leave that rule to prose alone.
fn named_change(line: &str, capability: &str) -> String {
    let after = line
        .strip_prefix("- `")
        .unwrap_or_else(|| panic!("{capability}: provenance line does not open `- \\``: {line}"));
    let (name, folded) = after
        .split_once('`')
        .unwrap_or_else(|| panic!("{capability}: provenance line has no closing backtick: {line}"));
    assert!(
        !name.is_empty() && !name.contains(['/', '\\']) && name != "." && name != "..",
        "{capability}: '{name}' is not an archived directory name"
    );
    let date = folded.strip_prefix(" — folded ").unwrap_or_else(|| {
        panic!("{capability}: provenance line does not read ` — folded <YYYY-MM-DD>`: {line}")
    });
    let day: Vec<char> = date.chars().collect();
    assert!(
        day.len() == 10
            && day[4] == '-'
            && day[7] == '-'
            && day
                .iter()
                .enumerate()
                .all(|(at, c)| c.is_ascii_digit() || at == 4 || at == 7),
        "{capability}: '{date}' is not a bare YYYY-MM-DD day, so the line says more \
         than which change wrote the capability: {line}"
    );
    name.to_string()
}

/// The capabilities a change touched, read from its own delta directory.
/// A change that promoted no truth — a docs-only fold — has no `specs/`
/// at all, and touched nothing.
fn touched_capabilities(change: &Path) -> BTreeSet<String> {
    let specs = change.join("specs");
    if !specs.is_dir() {
        return BTreeSet::new();
    }
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

/// The directories directly under a tree, sorted.
fn subdirectories(tree: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(tree)
        .unwrap_or_else(|error| panic!("{}: {error}", tree.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    found.sort();
    found
}

fn directory_name(path: &Path) -> String {
    path.file_name()
        .expect("directory name")
        .to_string_lossy()
        .into_owned()
}

/// The provenance lines a capability carries, with the section's own
/// rules checked: exactly one `## Provenance` heading, held at the end of
/// the file, so standing truth accumulates below it and is never edited
/// into a different meaning.
fn provenance_lines(spec: &Path, capability: &str) -> Vec<String> {
    let text =
        std::fs::read_to_string(spec).unwrap_or_else(|error| panic!("{}: {error}", spec.display()));
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
    text.lines()
        .skip_while(|line| *line != "## Provenance")
        .skip(1)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn every_capability_names_the_archived_changes_that_wrote_it() {
    let root = workspace();
    let archive = root.join("openspec/changes/archive");

    let capabilities: Vec<PathBuf> = subdirectories(&root.join("openspec/specs"))
        .into_iter()
        .filter(|path| path.join("spec.md").is_file())
        .collect();
    assert!(
        capabilities.len() >= 8,
        "the living capability tree is suspiciously small: {}",
        capabilities.len()
    );

    for capability_dir in capabilities {
        let capability = directory_name(&capability_dir);
        let provenance = provenance_lines(&capability_dir.join("spec.md"), &capability);
        assert!(
            !provenance.is_empty(),
            "{capability}: a promoted capability must name at least one change"
        );

        for line in provenance {
            let change = named_change(&line, &capability);
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

/// The other direction, and the one that catches a fold which archived a
/// change and skipped the append: every capability an archived change
/// promoted, and which still lives under `openspec/specs/`, names that
/// change. Walking only specs-to-archive would pass such a fold, because
/// the capability's list is already non-empty from an earlier change.
///
/// A delta whose capability no longer lives is not a failure: truth is
/// promoted and later retired, and a retired capability has no spec to
/// carry the line.
#[test]
fn every_archived_change_is_named_by_the_capabilities_it_promoted() {
    let root = workspace();
    let specs = root.join("openspec/specs");
    let archive = root.join("openspec/changes/archive");

    let changes = subdirectories(&archive);
    assert!(
        !changes.is_empty(),
        "no archived change under {}, so this walk proves nothing",
        archive.display()
    );

    let mut walked = 0usize;
    for change_dir in changes {
        let change = directory_name(&change_dir);
        for capability in touched_capabilities(&change_dir) {
            let spec = specs.join(&capability).join("spec.md");
            if !spec.is_file() {
                continue;
            }
            let named: BTreeSet<String> = provenance_lines(&spec, &capability)
                .iter()
                .map(|line| named_change(line, &capability))
                .collect();
            assert!(
                named.contains(&change),
                "change '{change}' promoted '{capability}', but {} names only {named:?}: \
                 the fold archived the change and never appended its line",
                spec.display()
            );
            walked += 1;
        }
    }
    assert!(
        walked >= 8,
        "the archive-to-specs walk covered {walked} capabilities, which is too few to prove it"
    );
}
