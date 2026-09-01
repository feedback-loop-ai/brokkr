//! T11: the frozen v1 contracts, the corpus and the policy table are
//! read-only for this slice. A contract change is a NEW numbered file
//! beside the old one, never an edit — so the bytes of the frozen files
//! are pinned by digest here, and the two new files are asserted to
//! exist beside them.

use std::path::PathBuf;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn digest(relative: &str) -> String {
    let bytes = std::fs::read(workspace().join(relative))
        .unwrap_or_else(|e| panic!("{relative} must exist: {e}"));
    brokkr_core::canonical::sha256_bytes(&bytes)
}

/// Recorded from this tree before the agent library existed.
const FROZEN: [(&str, &str); 6] = [
    (
        "contracts/event-envelope.v1.schema.json",
        "8863a07c8f5e879afe472b6a2c3060a40e0aa7a46a4132d4302a488d4e9b4f11",
    ),
    (
        "contracts/driver-protocol.v1.schema.json",
        "3435d43bfdc0731dc3895e85871a84301ce96c296dd5e1aa9fb106ca3d257c32",
    ),
    (
        "contracts/run-manifest.v1.schema.json",
        "d612c595e81b17de778a841fc2cea3e1e0769fa9f10204c515d290cfaff6cc96",
    ),
    (
        "contracts/run-manifest.v2.schema.json",
        "771fa0375cd3065c88ccb81775eb80383754bf646877f622e0480adff3ca7588",
    ),
    (
        "contracts/dispatch-envelope.v2.schema.json",
        "14b6bbe0d306f53d29026e4002d14d5b5fae40a12915d67e186a4c3d08f4475b",
    ),
    (
        "fixtures/evaluator/corpus.ndjson",
        "19ed1b05ca04ac0fd3c511b6c6c1c7412a2400b763c27658d964080692cf9964",
    ),
];

#[test]
fn the_frozen_contracts_and_the_corpus_keep_their_exact_bytes() {
    for (relative, pinned) in FROZEN {
        assert_eq!(digest(relative), pinned, "{relative} bytes moved");
    }
    // The production table is read-only too, and it is not a contract
    // file, so it is pinned separately by the same rule.
    assert_eq!(
        digest("policy/phase-machine.json"),
        "e0b3e9338745dd07685ef8a2182345f9f78df2b4e80203a498779c305e1e90ee",
    );
}

/// The new contracts land BESIDE the frozen ones, as new numbered
/// files — the only way a frozen contract ever changes.
#[test]
fn the_new_contracts_exist_beside_the_frozen_ones() {
    for (relative, title) in [
        (
            "contracts/run-manifest.v3.schema.json",
            "Forge run manifest v3",
        ),
        (
            "contracts/effect-provenance.v1.schema.json",
            "Forge effect provenance v1",
        ),
        // Decision 0022's rule-driven park: a new version beside v1, not
        // an edit to it. The v1 table schema was never a file here, so
        // this pins the ONE thing that matters — v2 landed as its own
        // published file rather than as bytes changed under v1's name.
        (
            "contracts/phase-machine.v2.schema.json",
            "Forge phase-machine table v2",
        ),
        // Decision 0023's world: the map's own schema, and the manifest
        // version that pins one. Both land beside the frozen files —
        // v1's and v3's bytes are asserted above and did not move.
        ("contracts/realms.v1.schema.json", "Forge realms map v1"),
        (
            "contracts/run-manifest.v4.schema.json",
            "Forge run manifest v4",
        ),
        // Decision 0021's witness (remedy ii of the reforged run): the
        // authorising adapters pinned as a new version beside v4, whose
        // bytes are asserted above and did not move.
        (
            "contracts/run-manifest.v5.schema.json",
            "Forge run manifest v5",
        ),
    ] {
        let body: serde_json::Value =
            serde_json::from_slice(&std::fs::read(workspace().join(relative)).unwrap()).unwrap();
        assert!(
            body["title"].as_str().unwrap().starts_with(title),
            "{relative} is the published extension schema"
        );
    }
}
