//! The byte-identity witnesses of the agent-library slice (decision
//! 0016, spec AC-4), pinned BEFORE any production edit so the claim is
//! measured across the change rather than asserted after it.
//!
//! `recipes/fast` and `bundles/verify` adopt no agent. Their pinned
//! manifest digest must not move, and their manifest must carry no
//! `agents` key at all — absence, not an empty object, is what keeps a
//! non-adopting bundle's identity exactly what it was.

use std::path::PathBuf;

use forge_runtime::Bundle;

/// The workspace root: this file lives at `crates/forge-runtime/tests/`.
fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// Recorded from this tree at the commit that introduced this test. A
/// move here is either an intended engine-version bump or the byte
/// identity this slice promised to keep — never a silent third thing.
const WITNESSES: [(&str, &str); 2] = [
    (
        "recipes/fast",
        "804bfd70d718d38cec6f03e53f54ca2d06f8561e3acac2f5ddb6e21ec64a226d",
    ),
    (
        "bundles/verify",
        "1d429e439d52ef8e2044535b0c08a15651a1411f37641fe38d3e4868ca2a32b2",
    ),
];

#[test]
fn non_adopting_bundles_keep_their_digest_and_grow_no_agents_key() {
    for (relative, digest) in WITNESSES {
        let bundle = Bundle::compile(&workspace().join(relative))
            .unwrap_or_else(|e| panic!("{relative} must compile: {e}"));
        assert_eq!(
            bundle.manifest_digest(),
            digest,
            "{relative} manifest digest moved"
        );
        assert!(
            bundle.manifest.get("agents").is_none(),
            "{relative} manifest gained an 'agents' key; a non-adopting \
             bundle carries none"
        );
    }
}

/// Every recipe and bundle in the tree still compiles — the other half
/// of AC-4, and the reason an adopting recipe cannot be left half-edited.
#[test]
fn every_bundle_in_the_tree_compiles() {
    let root = workspace();
    let mut dirs: Vec<PathBuf> = Vec::new();
    for parent in ["recipes", "bundles"] {
        let mut children: Vec<PathBuf> = std::fs::read_dir(root.join(parent))
            .unwrap_or_else(|e| panic!("{parent} must be readable: {e}"))
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.join("bundle.json").is_file())
            .collect();
        children.sort();
        dirs.append(&mut children);
    }
    assert!(dirs.len() >= 5, "expected the shipped recipes and bundles");
    for dir in dirs {
        // Against the in-tree library roots explicitly, rather than by
        // changing the process working directory: two tests share one
        // process, and a global `set_current_dir` would make this suite
        // order-dependent.
        Bundle::compile_with(&dir, &root.join("agents"), &root.join("adapters"))
            .unwrap_or_else(|e| panic!("{} must compile: {e}", dir.display()));
    }
}
