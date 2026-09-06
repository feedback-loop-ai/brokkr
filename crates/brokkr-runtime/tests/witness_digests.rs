//! Decision 0046: hands sites now pin the realm boundary; codex
//! harness fragments also move every identity that consults that adapter.
//! Pins below are updated only from the tests' reported left/right pairs.
//! The byte-identity witnesses of the agent-library slice (decision
//! 0016, spec AC-4), pinned BEFORE any production edit so the claim is
//! measured across the change rather than asserted after it.
//!
//! `recipes/fast`, `recipes/node`, `recipes/preflight`,
//! `recipes/wager-harness` and `bundles/verify` adopt no agent. The
//! routing and night-shift recipes seat library agents under decision
//! 0041. Every pinned manifest must
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

/// Decision 0045 moves the bundles that hire a review office, the triage
/// gate, the analyst or the chief: the codex adapter gained the `astra`
/// lane and a second judge, and those offices now chain across the vendor
/// line (`recipes/triage`, `recipes/night-shift`); the inline recipes and
/// `bundles/verify` pin only the claude adapter and did not move.
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
/// This review return moves them once more because deterministic ship gates
/// shed unused toolchain binds, and Node's verifier retains only its npm
/// cache. Those least-privilege hands declarations are also identity.
/// This correction moves them again because every deterministic gate
/// names its script bundle-relatively, making the strategy's read-only
/// script mount (rather than the operated-on repository) part of the
/// command the witness pins.
/// The returned implementation moves every script-owning witness again:
/// the verifier and shipper bytes now live inside those bundle roots and
/// therefore enter the manifest identity. Preflight already owned its
/// verifier under `roles/`, so it does not move.
/// Decision 0041 ruling 6 adds the routing witness: the routing recipe's
/// identity includes the triage office, routing table, current design
/// council, and Fast as its composed base.
/// Ruling 8 moves the routing descendants because triage now pins each
/// non-final sequence step's result vocabulary. House rules do not move a
/// bundle identity: they belong to the realm pin.
/// This review correction moves those descendants again because the compiler
/// now actually emits the promised vocabulary fields. Node moves separately:
/// its duplicated repository rules left the role and now live only in the
/// adopter's house file.
/// Decision 0042 moves the two design-bearing descendants: the old local
/// heading check is replaced by the realm dialect's validate command, verify
/// gains the dialect's archive check, and the chief now hands off `change` as
/// a typed input. Those commands and charter bytes are manifest identity.
/// Decision 0044 ruling 5 then moves every witness: the claude adapter's tool
/// vocabulary gains the explicit web-fetch and web-search grant, and every
/// bundle here pins that adapter through an agent or an inline gate. The two
/// design-bearing descendants therefore carry both legitimate movements.
/// The research recipe joins as the eighth witness because its researcher,
/// boxed registry gate and proposal-only table are its identity. Its dsh lane
/// is the ninth: it pins the same charter, the Qwen3.8-Max hire and the overlay
/// that enables page fetch and names its route.
/// Decision 0042's second slice moves the two triage descendants again: the
/// design route is now five phases, its new judges and validators are pinned,
/// and the dialect-free SDD smith replaces the generic design implementer.
/// The returned reviews move those descendants once more: spec compliance's
/// complete typed contract and intake's boxed hands are agent identity, then
/// the loop judges bind their deterministic checks and closed `drift_in`
/// vocabulary while the smith regains its complete result semantics.
/// Night-shift moves independently because its one-attempt override now names
/// every phase in the SDD route.
/// This correction moves the two triage descendants together: artifact
/// validation retries now bind to journal-counted failures instead of every
/// return into the phase.
const WITNESSES: [(&str, &str); 9] = [
    (
        "recipes/fast",
        "ff28a7b81e7e5c9083e32349f03b51bc98a9a240700e206ea3053a11c5519e3d",
    ),
    (
        "recipes/node",
        "9360691caceafd30cac9ad4fafe4825573be5880692c3600d47c7a6595364a60",
    ),
    (
        "recipes/preflight",
        "dd34ad1f6c982ba90cc13222bc291bca79fb25f6477409e0816c40a0f0aec652",
    ),
    (
        "recipes/night-shift",
        "5b9ab06abdc7f6e50e8389872a3507bada8b15d2b0837e501b2c25b46e23467b",
    ),
    (
        "recipes/wager-harness",
        "3a0bf5710c2c577439b814ebd96cd1f88a206e3f65f7c11603cf5c4cec111f3e",
    ),
    (
        "recipes/triage",
        "1b37f6d37c27ee43ea93feac54370524db8c52a8e8c9f41fd344097ff991e395",
    ),
    (
        "recipes/research",
        "c557bc4c0d7ef72e735cea59fa372f61b4c17f92c336e9710b7659e64bb17bd6",
    ),
    (
        "recipes/research-dsh",
        "0b40a3b54611c1e7900d340cd74414d75a0219790b5f7aac21bf889dde10f0fa",
    ),
    (
        "bundles/verify",
        "cf16320ccaeaccb32e820f911bca2e2ff686b680cce236679a1addd4c41add63",
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

/// Decision 0046 ruling 1: the contract a compiled manifest claims is
/// run-manifest/v9 — v8 plus the `boundary` map beside `hands`, present
/// exactly with it. Every witness validates, and the ones that box
/// something carry both keys over the same site labels.
#[test]
fn every_witness_manifest_satisfies_the_v9_contract_it_claims() {
    let root = workspace();
    let schema: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("contracts/run-manifest.v9.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::draft7::new(&schema).unwrap();
    for (relative, _) in WITNESSES {
        let bundle = Bundle::compile_with(
            &root.join(relative),
            &root.join("agents"),
            &root.join("adapters"),
        )
        .unwrap();
        assert!(
            validator.is_valid(&bundle.manifest),
            "{relative} emits a manifest outside run-manifest/v9"
        );
        let hands = bundle.manifest.get("hands").and_then(|v| v.as_object());
        let boundary = bundle.manifest.get("boundary").and_then(|v| v.as_object());
        assert_eq!(
            hands.map(|map| map.keys().collect::<Vec<_>>()),
            boundary.map(|map| map.keys().collect::<Vec<_>>()),
            "{relative}: boundary is keyed exactly as hands is"
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
