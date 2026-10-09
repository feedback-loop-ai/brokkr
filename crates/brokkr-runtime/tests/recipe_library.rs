//! The recipe library says each fact once (decision 0071 ruling 5, #359).
//!
//! A recipe that differs from another by a seat, an entry or a reason is
//! an `extends` overlay (decision 0017), so two recipes whose resolved
//! tables are byte-identical must read that table from one layer. And a
//! script or charter decision 0048 makes a bundle directory carry for
//! itself is a copy of one canonical source, held to it byte for byte, so
//! a fix made there reaches every bundle that runs it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_runtime::bundle::compose::resolve;
use serde_json::Value;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// Every recipe directory in the library, by name, sorted.
fn recipes(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root.join("recipes"))
        .expect("recipes/ is readable")
        .map(|entry| entry.expect("a recipe entry").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| {
            root.join("recipes")
                .join(name)
                .join("bundle.json")
                .is_file()
        })
        .collect();
    names.sort();
    names
}

/// The layer whose own `policy` a composition's table was last written by:
/// the nearest layer, leaf first, whose `bundle.json` declares one.
fn table_author(roots: &[PathBuf]) -> String {
    roots
        .iter()
        .find(|root| {
            let bytes = std::fs::read(root.join("bundle.json")).expect("a layer's bundle.json");
            let document: Value = serde_json::from_slice(&bytes).expect("a layer's JSON");
            document.get("policy").is_some()
        })
        .and_then(|root| root.file_name())
        .expect("a resolved recipe has a table some layer declared")
        .to_string_lossy()
        .into_owned()
}

/// Every pair of recipes that resolve to one table but read it from two
/// layers, as the refusal names it.
fn copied_tables(root: &Path) -> Vec<String> {
    let mut by_table: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for name in recipes(root) {
        let resolved = resolve(&root.join("recipes").join(&name))
            .unwrap_or_else(|error| panic!("recipes/{name} resolves: {error}"));
        let digest = brokkr_core::canonical::sha256_hex(&resolved.table);
        let author = table_author(&resolved.roots);
        by_table.entry(digest).or_default().push((name, author));
    }
    let mut offenses = Vec::new();
    for group in by_table.values() {
        let (first, first_author) = &group[0];
        for (name, author) in &group[1..] {
            if author != first_author {
                offenses.push(format!(
                    "recipes/{first} and recipes/{name} resolve to byte-identical policy \
                     tables written twice, in recipes/{first_author} and recipes/{author}; \
                     extend one from the other rather than copy the table (decision 0017, #359)"
                ));
            }
        }
    }
    offenses
}

/// Acceptance of #359: no table is written twice. Recipes that inherit a
/// table share it with the layer that wrote it, and only those.
#[test]
fn no_two_recipes_carry_one_policy_table_written_twice() {
    assert_eq!(copied_tables(&workspace()), Vec::<String>::new());
}

/// The canonical source of each file a bundle directory carries for itself
/// (decision 0048 pins a script's bytes in the directory that runs it): a
/// file of this name anywhere in the library is a byte-for-byte copy of it.
/// Fast's implementer role became the library's intakeless charter when
/// fast moved onto library offices (#360); the inline smiths still read it.
const CANONICAL: [(&str, &str); 3] = [
    ("verify-seat.sh", "recipes/fast/scripts/verify-seat.sh"),
    ("ship-seat.sh", "recipes/fast/scripts/ship-seat.sh"),
    (
        "implementer.md",
        "agents/charters/implementer-intakeless.md",
    ),
];

/// Files that share a canonical's name and are a different office, each
/// with the reason it is its own.
const OWN_OFFICES: [(&str, &str); 5] = [
    (
        "recipes/night-shift/roles/implementer.md",
        "the unattended implementer's charter",
    ),
    (
        "recipes/node/roles/implementer.md",
        "Node's implementer charter",
    ),
    ("recipes/node/roles/verify-seat.sh", "Node's npm verifier"),
    (
        "recipes/preflight/roles/verify-seat.sh",
        "preflight's local CI gates on an unmerged branch",
    ),
    (
        "recipes/research/roles/verify-seat.sh",
        "the research registry's parse gate",
    ),
];

/// Every file under `dir`, relative to `root`, sorted.
fn files_under(root: &Path, dir: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).expect("a readable directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            files_under(root, &path, out);
        } else {
            let relative = path.strip_prefix(root).expect("under the workspace");
            out.push(relative.to_string_lossy().into_owned());
        }
    }
}

/// Every way the library's copies disagree with their canonical sources.
fn drifted_copies(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    for dir in ["recipes", "scripts"] {
        files_under(root, &root.join(dir), &mut files);
    }
    files.sort();
    let mut offenses = Vec::new();
    for (name, canonical) in CANONICAL {
        let source = std::fs::read(root.join(canonical)).expect("a canonical source");
        let named = files
            .iter()
            .filter(|path| path.rsplit('/').next() == Some(name) && *path != canonical);
        for path in named {
            let same = std::fs::read(root.join(path)).expect("a copy") == source;
            let own = OWN_OFFICES.iter().any(|(office, _)| office == path);
            match (path.starts_with("recipes/"), own, same) {
                (false, _, _) => offenses.push(format!(
                    "{path} is a copy of {canonical} outside the library, which no bundle runs; \
                     delete it"
                )),
                (true, false, false) => offenses.push(format!(
                    "{path} has drifted from {canonical}; copy the canonical over it, or name \
                     the office it is in OWN_OFFICES"
                )),
                (true, true, true) => offenses.push(format!(
                    "{path} is a copy of {canonical}, not an office of its own; drop it from \
                     OWN_OFFICES"
                )),
                (true, false, true) | (true, true, false) => {}
            }
        }
    }
    for (office, _) in OWN_OFFICES {
        if !files.iter().any(|path| path == office) {
            offenses.push(format!(
                "{office} is named in OWN_OFFICES but is not in the library"
            ));
        }
    }
    offenses
}

/// Item 3 of #359: every shipped copy is its canonical source's bytes, so
/// #287's fix to the verifier's failure filter, made once, runs in every
/// bundle; an office of its own is named with its reason.
#[test]
fn every_pinned_script_and_charter_copy_is_its_canonical_source() {
    assert_eq!(drifted_copies(&workspace()), Vec::<String>::new());
}
