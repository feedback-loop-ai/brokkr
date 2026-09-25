//! Decision 0046: hands sites now pin the realm boundary; codex
//! harness fragments also move every identity that consults that adapter.
//! Pins below are updated only from the tests' reported left/right pairs.
//! Merging main's research-dsh xhigh effort pin with this boundary slice
//! moves that recipe again; its witness covers both changes together.
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
/// `recipes/gpt-flash` joins as the tenth witness: it is a descendant of
/// `recipes/triage` whose scoped `gpt-flash-*` roster, four-strategy
/// Sol/Flash/Astra seats and inherited deterministic gates are its identity.
/// Decision 0058's citation in the recipe README moves its manifest digest
/// once more, because the README bytes are part of the recipe's identity.
/// Decision 0035's 2026-09-11 addendum moves it again: the dsh adapter
/// lists an effortless route, and every bundle resolving an agent
/// through that adapter pins its digest — the roster's Flash seats do.
/// The dsh pin to the installed 0.1.5-rc.1 moves it once more for one
/// named reason: `hands.unsupported` re-measured on that release, which
/// moves the dsh adapter digest every Flash resolution witnesses.
/// The review-first remediation corrects that reason once more for one
/// named reason: `DSH_TOOLS_MODE` is documented (`native|ptc|both` into
/// `tools.mode`), those are presentation modes rather than a capability
/// restriction, and the installed tools/headless components identify as
/// 0.1.5-rc.2 behind launcher 0.1.5-rc.1 — which moves the same digest.
/// Proposed decision 0056 moves every bundle whose sites resolve through
/// `adapters/claude.json`, `adapters/codex.json`, `adapters/dsh.json` or
/// `adapters/lanetally.json`: each now declares what has been MEASURED
/// about resuming it, and an adapter declaration is bundle data. That
/// first edit left `recipes/research-dsh` unmoved, because its lane
/// resolved through no declaration this change then pinned.
/// The returned F1 correction pins the declaration a work-class inline
/// driver reads its resume assessment from, beside the gate's authorising
/// digest: that assessment decides whether the seat rejoins, so an edit
/// to it must move the bundle identity the offer is compared against.
/// That moves `recipes/fast` and `recipes/node` (their inline Claude
/// implementers), `recipes/night-shift` and `recipes/research-dsh`
/// (inline work seats the first `drivers` witness did not yet pin) and
/// `recipes/wager-harness` (its inline Codex implementer). The gate-only
/// bundles, the agent-backed ones and `recipes/preflight` keep their
/// digests: a seat that consulted no inline model declaration is not
/// touched.
/// The 2026-09-16 Codex declaration reconciliation moves the four bundles
/// whose inline or composed work sites read `adapters/codex.json`'s resume
/// assessment: `recipes/night-shift`, `recipes/wager-harness`,
/// `recipes/triage` and `recipes/gpt-flash`. The other six are unchanged;
/// their measured digests below are the copies of that edit's actual
/// compiles, never recomputed guesses.
/// The 2026-09-21 roster addition moves the three bundles whose sites
/// resolve through `adapters/dsh.json`: the adapter gains the `spark-glm`
/// local route with its `glm-flash` alias (ruled 2026-09-16) and the
/// Model Studio aliases `glm53` and `studio-flash41`, and an adapter
/// declaration is bundle data. The other seven are unchanged.
/// Issue #307 (operator rulings 2026-09-20 and 2026-09-21) moves
/// `recipes/triage` and nothing else here: its `engine` case seats
/// `implementer-engine`, which now hires astra@high then fable@high and
/// declares decision 0043's workspace hands in place of its Cargo/Git tool
/// list. The agent digest, the adapters that chain consults and the
/// manifest's `hands` and `boundary` entries for that site are the
/// movement. No charter, adapter or engine version changed; the measured
/// value is this tree's actual compile, and the nine others — including
/// `recipes/night-shift` and `recipes/gpt-flash`, which derive from triage
/// but seat their own implementers — reported no movement.
/// The 2026-09-22 roster move re-points three aliases the operator named:
/// `opus` to `claude-opus-5-5`, `sol` to `gpt-6-sol` and `luna` to
/// `gpt-6-luna`, each probed live that day. An adapter's model map is
/// bundle data, so every witness that resolves a seat through the claude
/// or codex adapter moves; each digest below is the actual compile.
/// Proposed decision 0069 moves the four witnesses that pin
/// `adapters/codex.json` — `recipes/night-shift`, `recipes/wager-harness`
/// (its inline Codex implementer), `recipes/triage` and
/// `recipes/gpt-flash` — because the declaration gained `hands.notice`
/// and dated 2026-09-24 hands-discovery limitations. The six others
/// reported no movement; each value is measured before and after the edit.
/// The Opus 5.5 / Fable 5.1 prompt audit (acffed37) moves six witnesses:
/// the implementer charters and the recipe implementer roles gained a
/// scope paragraph, an evidence paragraph and a targeted-edit sentence.
/// `recipes/fast`, `recipes/node`, `recipes/night-shift` and
/// `recipes/wager-harness` carry an edited role; `recipes/triage` composes
/// `fast` and seats the SDD smith, and `recipes/gpt-flash` derives from
/// triage. `recipes/preflight`, both research recipes and `bundles/verify`
/// seat no implementer and reported no movement; each value is the
/// test's own reported digest.
const WITNESSES: [(&str, &str); 10] = [
    (
        "recipes/fast",
        "57d76d0308f251652925f8a3acb40c1b55e664e4880875f9207aea37eb5c3e54",
    ),
    (
        "recipes/node",
        "d1fa6d617f6b877d8c8bc2f41afbca5c3e42e5db879bf96fb463bc6c3e9b8602",
    ),
    (
        "recipes/preflight",
        "a99bb568b244c74efa99d9a01a7efb2de57fb2c3ed14607821b2ed6645da08f9",
    ),
    (
        "recipes/night-shift",
        "18ccb3c890fe361379327542e1cddcb2696d048002ceb0aeb3cbf35ac29fb34e",
    ),
    (
        "recipes/wager-harness",
        "390d1a592cc30a79a7602e6431220ec8c5c093cc015830ca6d13d46eed7208b1",
    ),
    (
        "recipes/triage",
        "dc83420d3b8369ae7e5ed441cef178f700ea32b930044979eb285945a810a344",
    ),
    (
        "recipes/research",
        "393ffd7c5c396d74ed68b23191751e6517ac691c344067d8abd6803037940a9b",
    ),
    (
        "recipes/research-dsh",
        "76b686a59f3de486c320cea4c575be07e4a13334f549c5a5571dc81593aa2e6c",
    ),
    (
        "recipes/gpt-flash",
        "6649f1bfe3ca48da0208900ff4dbebc5c86b7a25b93347dc5b89d27bfae29417",
    ),
    (
        "bundles/verify",
        "b10e1f471a874ff1a0e278e8f303bda9e3965345b509bde0de1287f4eeee9f03",
    ),
];

/// The INLINE model-driver seats of each, by name and the adapter each
/// names: exactly what a `drivers` witness must account for. Since
/// proposed decision 0056 ruling 5 an inline WORK seat consults its
/// adapter's resume assessment just as an inline gate consults its tier,
/// and that declaration is pinned beside the gate's so an edit to it
/// moves the identity that decides whether the seat rejoins.
/// `bundles/verify` and `recipes/preflight` have no ship phase to gate —
/// and no working seat at all, so in those two every seat appears here.
///
/// Library-backed gates carry their adapter witnesses through the agent
/// resolution record instead, so they do not belong in this inline-only
/// list. In particular, all of Crucible's review offices are gates now.
const INLINE_ADAPTERS: [(&str, &[(&str, &str)]); 4] = [
    (
        "recipes/fast",
        &[
            ("implement", "claude"),
            ("review", "claude"),
            ("ship", "exec"),
            ("verify", "exec"),
        ],
    ),
    (
        "recipes/node",
        &[
            ("implement", "claude"),
            ("review", "claude"),
            ("ship", "exec"),
            ("verify", "exec"),
        ],
    ),
    (
        "recipes/preflight",
        &[("review", "claude"), ("verify", "exec")],
    ),
    (
        "recipes/wager-harness",
        &[
            ("implement", "codex"),
            ("review", "claude"),
            ("ship", "exec"),
            ("verify", "exec"),
        ],
    ),
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

/// What answered an inline seat is pinned where the bundle's identity
/// can see it: one entry per inline model-driver seat, naming the driver
/// and the digest of the adapter file whose declared tier let a gate
/// judge, or whose resume assessment a work seat would rejoin under
/// (decision 0021; proposed decision 0056 ruling 5). An edit to either
/// declaration moves the identity, which is what stops a changed rule
/// from reusing a root the old one opened.
#[test]
fn an_inline_gate_pins_the_adapter_declaration_that_authorised_it() {
    let root = workspace();
    let adapters = brokkr_runtime::agents::Adapters::load(&root.join("adapters"))
        .expect("the shipped adapters load");
    let digest = |provider: &str| {
        adapters
            .digest(provider)
            .unwrap_or_else(|| panic!("the {provider} adapter is declared"))
    };
    for (relative, seats) in INLINE_ADAPTERS {
        let bundle = Bundle::compile_with(
            &root.join(relative),
            &root.join("agents"),
            &root.join("adapters"),
        )
        .unwrap_or_else(|e| panic!("{relative} must compile: {e}"));
        let witnessed = bundle.manifest["drivers"]
            .as_object()
            .unwrap_or_else(|| panic!("{relative} witnesses no driver for its inline seats"));
        let names: Vec<&str> = witnessed.keys().map(String::as_str).collect();
        let expected: Vec<&str> = seats.iter().map(|(seat, _)| *seat).collect();
        assert_eq!(names, expected, "{relative} witnessed the wrong seats");
        for (seat, provider) in seats {
            assert_eq!(
                witnessed[*seat],
                serde_json::json!({ (*provider): digest(provider) }),
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
