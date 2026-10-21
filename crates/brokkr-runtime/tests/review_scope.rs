//! Decision 0094 ruling 7: a review judges the branch from its merge-base
//! with the default branch, the cut the shipping gate vouches, whichever
//! run or hand wrote it. A review scoped to "since the run began" would
//! rule clean over commits no seat of the vouching run read, which an
//! entered run, or any run started over work already on the branch, puts
//! in front of it.
//!
//! The seven files that read so are held to the clause by path; every
//! charter and every recipe role is held to the rule that none scopes its
//! read to the run's beginning, so a charter written later cannot reopen
//! the gap. The two copies stay their source byte for byte (`roster.rs`).
//! These live beside `library_data.rs`, which is at its file ceiling.

use std::path::{Path, PathBuf};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The file's words, each run of whitespace one space, so a clause reads
/// the same wherever its lines wrap.
fn words(path: &Path) -> String {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

const CLAUSED: [&str; 7] = [
    "agents/charters/reviewer.md",
    "agents/charters/review-correctness.md",
    "agents/charters/review-security.md",
    "agents/charters/review-adversarial.md",
    "agents/charters/node-reviewer.md",
    "recipes/review-first/roles/reviewer.md",
    "recipes/standby/roles/reviewer.md",
];

#[test]
fn the_seven_review_files_scope_the_branch_from_its_merge_base() {
    for file in CLAUSED {
        let text = words(&workspace().join(file));
        for clause in [
            "since its merge-base with the default branch",
            "`context.entry`",
        ] {
            assert!(text.contains(clause), "{file} does not name {clause}");
        }
    }
}

#[test]
fn no_charter_or_role_scopes_its_read_to_the_runs_beginning() {
    let root = workspace();
    let mut files: Vec<PathBuf> = std::fs::read_dir(root.join("agents/charters"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    for recipe in std::fs::read_dir(root.join("recipes")).unwrap() {
        let roles = recipe.unwrap().path().join("roles");
        if roles.is_dir() {
            files.extend(
                std::fs::read_dir(roles)
                    .unwrap()
                    .map(|entry| entry.unwrap().path()),
            );
        }
    }
    files.retain(|path| path.extension().is_some_and(|ext| ext == "md"));
    assert!(files.len() > CLAUSED.len(), "the trees were not read");
    for path in files {
        let text = words(&path).to_lowercase();
        for scope in ["since the run began", "pre-run commit"] {
            assert!(
                !text.contains(scope),
                "{} scopes its read by {scope:?}",
                path.display()
            );
        }
    }
}
