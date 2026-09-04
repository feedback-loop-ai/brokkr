//! The byte-identity witnesses of the agent-library slice (decision
//! 0016, spec AC-4), pinned BEFORE any production edit so the claim is
//! measured across the change rather than asserted after it.
//!
//! `recipes/fast`, `recipes/node`, `recipes/preflight`,
//! `recipes/wager-harness` and `bundles/verify` adopt no agent. The
//! other roster recipes (`ember`, `crucible`, and `night-shift`) now
//! seat library agents under decision 0041. Every pinned manifest must
//! move only when its recorded strategy or dependencies move; an inline
//! recipe must continue to carry no `agents` key at all.
//!
//! Adopting no agent is not the same as answering to nobody. The inline
//! recipes seat gates, and since decision 0021 a gate stands on an
//! adapter's declared tier — so they carry a `drivers` key naming the
//! adapter digest that authorised each judging seat. That key is the
//! witness the refusals were missing: without it a demoted tier would
//! change what the compiler allows while leaving the bundle's identity
//! untouched.

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

/// Decision 0041 moves every pinned bundle for its three enacted reasons:
/// ruling 1 advances the fable adapter mapping, ruling 2 moves model sites
/// onto the roster, and ruling 3 adds each adapter's judges declaration.
/// Decision 0043 previously moved every pinned bundle at once, and for one reason:
/// every adapter file gained `hands` — how the provider puts its hands in
/// the box, or the measured reason it cannot — and a bundle whose inline
/// gate pins the adapter declaration that authorised it (decision 0021)
/// carries that file's digest in its identity. The bundles hiring the
/// review agents moved further: those agents now chain fable@high →
/// opus@xhigh → sol@xhigh and declare boxed hands, so their resolution
/// records and the manifest's `hands` key changed. This review correction
/// moves only bundles that hire an intake or implementer: their Git work is
/// now named in the charter and expressible in the resolved tool grant;
/// ignored allow-lists beside boxed hands were removed at the same time.
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
/// or decision 0019's closing sweep moved living role or policy prose,
/// or the byte identity this slice promised to keep. Never a silent
/// extra thing.
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
///
/// Decision 0033 deliberately moved the seven recipe witnesses: each
/// bundle now carries the description and relative cost rendered in the
/// sixty-second contributing guide. `bundles/verify` did not move.
///
/// Decision 0035 ruling 5 moved all eight, and that movement IS the
/// ruling rather than a side effect of it: every model pin now carries
/// an effort pin, so every one of these bundles states in its argv the
/// effort it hires as well as the model. A hire that changed is a
/// bundle whose identity changed, and this is where that is said out
/// loud. Nothing else moved them — no charter, no policy, no role
/// prose; the diff on each is `--effort <level>` beside `--model`, and
/// each level is the driver's own default rather than a tuned choice.
/// Decision 0039 moved `recipes/fast` and the four recipes composed
/// from it, and nothing else: the table gained `REVIEW-CLEAN-DOCS-FIXES`,
/// which ships a clean review whose own commits lie entirely in the
/// repository's docs class instead of buying the whole verify again. A
/// table that rules differently is a different strategy, and this is
/// where that is said. `recipes/node`, `recipes/preflight` and
/// `bundles/verify` do not derive from `fast` and did not move.
/// Decision 0041 rulings 4 and 5 move every witness here: gates no longer
/// advertise or route on judge-authored fixes, implement gains its
/// reserved `oversized` verdict, and tables with an implement phase gain
/// bounded finding returns. The two verdict-only strategies move because
/// their reviewer charter and declared inputs become honestly read-only.
/// This slice moves all eight again for one named reason: verifier and
/// shipper are no longer model-backed agents. Their inline exec commands,
/// boxed hands and recipe-owned verifier scripts are bundle identity, and
/// every wager inherits the same gates from `fast` by construction.
const WITNESSES: [(&str, &str); 8] = [
    (
        "recipes/fast",
        "d1ec8b3ab21140fb23c3eaae5b9aec970f9c71d79aa159eda91e1dad09450fc6",
    ),
    (
        "recipes/node",
        "de7e2bf2c18f2ec3bf3b339c9704ee5569a9b9b2b20dcac0e87cc369511b4bdf",
    ),
    (
        "recipes/preflight",
        "17055e1c914f4455fb5e2dba908da75a99b8ab52f555bd89a3141f57e60845f7",
    ),
    (
        "recipes/ember",
        "fda02a55ce9b119f73c6257d9b5513b2ab5cf3a0136049ba6e3d31cb9cdf87cb",
    ),
    (
        "recipes/crucible",
        "b42f0118c8a9e755e03081a619c450b7b7a0fb972ae262e6d47f24a15f0d91c9",
    ),
    (
        "recipes/night-shift",
        "c807e27ad2086d90f38b5890da884d4269903fd00f5d86717d67ff240643fd52",
    ),
    (
        "recipes/wager-harness",
        "75b3dec5dfb7dd81830a154b69c9559fdb3c0b3d7172cc8d595e59d4bc9002e7",
    ),
    (
        "bundles/verify",
        "ba5428fc3bd48a35e3a2f8cc3ccb50dd3e0d32e843cab0e37ca45ff4d4324efb",
    ),
];

/// The INLINE gate seats of each, by name: exactly what a `drivers`
/// witness must account for, one entry per judging seat and none for a
/// working one. `bundles/verify` and `recipes/preflight` have no ship
/// phase to gate — and no working seat at all, so in those two every
/// seat appears here.
///
/// Library-backed gates carry their adapter witnesses through the agent
/// resolution record instead, so they do not belong in this inline-only
/// list. In particular, all of Crucible's review offices are gates now.
const INLINE_GATES: [(&str, &[&str]); 4] = [
    ("recipes/fast", &["review", "ship", "verify"]),
    ("recipes/node", &["review", "ship", "verify"]),
    ("recipes/preflight", &["review", "verify"]),
    ("recipes/wager-harness", &["review", "ship", "verify"]),
];

#[test]
fn pinned_bundles_keep_their_recorded_digest() {
    let root = workspace();
    for (relative, digest) in WITNESSES {
        // Explicit roots, as in the compile below: since decision 0021 a
        // compile reads adapter data for inline gates too, even though
        // they adopt no agent — a gate seat's trust tier is declared there.
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
    let exec = adapters
        .digest("exec")
        .expect("the deterministic adapter is declared");
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
            let (driver, digest) = if matches!(*seat, "verify" | "ship") {
                ("exec", exec)
            } else {
                ("claude", claude)
            };
            assert_eq!(
                witnessed[*seat],
                serde_json::json!({ (driver): digest }),
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
