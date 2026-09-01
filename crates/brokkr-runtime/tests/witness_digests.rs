//! The byte-identity witnesses of the agent-library slice (decision
//! 0016, spec AC-4), pinned BEFORE any production edit so the claim is
//! measured across the change rather than asserted after it.
//!
//! `recipes/fast`, `recipes/node`, `recipes/preflight` and
//! `bundles/verify` adopt no agent, and neither do the four roster
//! recipes that later joined them (`ember`, `crucible`, `night-shift`,
//! `wager-harness`, all composed from `fast`'s inline seats). Their
//! pinned manifest digest must not move, and their manifest must carry
//! no `agents` key at all —
//! absence, not an empty object, is what keeps a non-adopting bundle's
//! identity exactly what it was.
//!
//! Adopting no agent is not the same as answering to nobody. Both seat
//! INLINE gates, and since decision 0021 a gate stands on an adapter's
//! declared tier — so both now carry a `drivers` key naming the adapter
//! digest that authorised each judging seat. That key is the witness the
//! refusals were missing: without it a demoted tier would change what
//! the compiler allows while leaving the bundle's identity untouched.

use std::path::PathBuf;

use brokkr_runtime::Bundle;

/// The workspace root: this file lives at `crates/brokkr-runtime/tests/`.
fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// Recorded from this tree at the commit that introduced this test. A
/// move here is either an intended engine-version bump, an intended
/// policy change re-pinned as the identity change it is (decision 0022
/// moved `recipes/fast`, which gained the reforging back-edge, and left
/// `bundles/verify` — which has no implement phase to return to —
/// exactly where it was; decision 0019's rename moved both, because the
/// `{forge}` token in their argv became `{brokkr}`; decision 0021's
/// compile-time refusals moved both again, because every seat in them
/// now declares whether it works or judges, and once more because their
/// inline gates now pin the adapter declaration that authorises them),
/// or the byte identity this slice promised to keep. Never a silent
/// fourth thing.
///
/// `recipes/node` joined them when the Node reference recipe landed,
/// pinned from its first compile: it ships as reference material a
/// stranger copies, so an unreviewed edit to a charter or a driver's
/// tool list must fail a test here exactly as it would for `fast`.
///
/// `recipes/preflight` joined for the sharper version of the same
/// reason: a contributor runs it on their own branch expecting the
/// machine's own bar, and a charter quietly softened — a gate dropped
/// from the verifier's list, a tool added to a driver — would change
/// what that promise is worth without changing anything visible.
///
/// The four roster recipes joined on the same terms, pinned from their
/// first compile. Each is `fast` plus a stated difference, so each
/// carries `fast`'s digest transitively under its `@compose/0000/fast`
/// manifest entry: edit `recipes/fast` and all move together, which
/// is the honest reading — a derived recipe IS a different strategy when
/// its base changes.
const WITNESSES: [(&str, &str); 8] = [
    (
        "recipes/fast",
        "6324f76f7bfacdba7b6d93c1f07022344cafe5467cfab27a333e52aa9acb3ae7",
    ),
    (
        "recipes/node",
        "ed3c623bceaa7ae83d849cad38451f4b99500752c4369a92b448b0d849dc7b3b",
    ),
    (
        "recipes/preflight",
        "9e78d802db37f4d389beaf09f964fac7124af111eb5ee23d8b05adeac8d41d9c",
    ),
    (
        "recipes/ember",
        "aa6514b5bc24fd85acc201b11c65dbad3d8ffe9356aadabd79161069b951f860",
    ),
    (
        "recipes/crucible",
        "50edb5383dd2291a0f13a57cbee884cd0427043b654c91e899226f0fea571955",
    ),
    (
        "recipes/night-shift",
        "a3352f00db74e3abc20d7fe5df05aafef8f463a8bb8a29649e0c6fb441f78701",
    ),
    (
        "recipes/wager-harness",
        "44da266f85547bd4212085fc965088d0b174c2f1de06eb0a1b45ea771c9805f6",
    ),
    (
        "bundles/verify",
        "4a94d29f9058cc506535809ec660e2dfc5c34ad3d144c09ad72fba66ac2e479f",
    ),
];

/// The INLINE gate seats of each, by name: exactly what a `drivers`
/// witness must account for, one entry per judging seat and none for a
/// working one. `bundles/verify` and `recipes/preflight` have no ship
/// phase to gate — and no working seat at all, so in those two every
/// seat appears here.
///
/// `recipes/crucible` is the one that reads differently, and on purpose:
/// its review seat is a sequence whose panel of `positions` WORKS and
/// whose `chief` step JUDGES, so the witness names `review:chief` and
/// not `review`. If that entry ever reads plain `review`, the chief
/// stopped being the seat's gate — and the two positions, which are
/// work-class and admit any driver under decision 0021 ruling 7, would
/// be ruling the protected phase between them.
const INLINE_GATES: [(&str, &[&str]); 8] = [
    ("recipes/fast", &["review", "ship", "verify"]),
    ("recipes/node", &["review", "ship", "verify"]),
    ("recipes/preflight", &["review", "verify"]),
    ("recipes/ember", &["review", "ship", "verify"]),
    ("recipes/crucible", &["review:chief", "ship", "verify"]),
    ("recipes/night-shift", &["review", "ship", "verify"]),
    ("recipes/wager-harness", &["review", "ship", "verify"]),
    ("bundles/verify", &["review", "verify"]),
];

#[test]
fn non_adopting_bundles_keep_their_digest_and_grow_no_agents_key() {
    let root = workspace();
    for (relative, digest) in WITNESSES {
        // Explicit roots, as in the compile below: since decision 0021 a
        // compile reads the adapter data even for these two, which adopt
        // no agent — a gate seat's trust tier is declared there.
        let bundle = Bundle::compile_with(
            &root.join(relative),
            &root.join("agents"),
            &root.join("adapters"),
        )
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

/// What authorises an inline gate is pinned where the bundle's identity
/// can see it (decision 0021): one entry per gate-class seat, naming the
/// driver and the digest of the adapter file whose declared tier let it
/// judge — and no entry for a work-class seat, which consulted nothing.
#[test]
fn an_inline_gate_pins_the_adapter_declaration_that_authorised_it() {
    let root = workspace();
    let adapters = brokkr_runtime::agents::Adapters::load(&root.join("adapters"))
        .expect("the shipped adapters load");
    let claude = adapters
        .digest("claude")
        .expect("the incumbent adapter is declared");
    for (relative, gates) in INLINE_GATES {
        let bundle = Bundle::compile_with(
            &root.join(relative),
            &root.join("agents"),
            &root.join("adapters"),
        )
        .unwrap_or_else(|e| panic!("{relative} must compile: {e}"));
        let witnessed = bundle.manifest["drivers"]
            .as_object()
            .unwrap_or_else(|| panic!("{relative} witnesses no driver for its gates"));
        let seats: Vec<&str> = witnessed.keys().map(String::as_str).collect();
        assert_eq!(seats, gates, "{relative} witnessed the wrong seats");
        for seat in gates {
            assert_eq!(
                witnessed[*seat],
                serde_json::json!({ "claude": claude }),
                "{relative} seat '{seat}' pins the wrong adapter"
            );
        }
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
