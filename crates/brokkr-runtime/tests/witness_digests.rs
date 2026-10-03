//! The byte-identity witnesses (decision 0016, spec AC-4; #358): the
//! manifest digest of every bundle under `recipes/` and `bundles/` and
//! the bytes of every shipped charter, held once as data in
//! `witnesses.json`.
//!
//! A pinned manifest moves only when its recorded strategy or its
//! dependencies move: a charter, a role, a table, an adapter declaration
//! it consults, a composed base, or the engine version. Adopting no agent
//! is not the same as answering to nobody: an inline gate stands on an
//! adapter's declared tier (decision 0021), so an inline recipe carries a
//! `drivers` key naming the adapter digest that authorised each seat, and
//! a demoted tier moves the bundle's identity.
//!
//! A move is re-pinned by one command, never by hand:
//!
//! ```text
//! BROKKR_BLESS=1 cargo test -p brokkr-runtime --test it witness_digests::
//! ```
//!
//! Without it the suite compares every witness and fails once with a
//! table of every old → new value. Why a value moved belongs in the commit
//! message that moves it; the reviewed diff of `witnesses.json` is the
//! witness.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use brokkr_runtime::Bundle;

use crate::witnesses;

use witnesses::{Witnesses, TABLE};

/// The workspace root: this file lives at `crates/brokkr-runtime/tests/`.
fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// The one command that rewrites the table.
const BLESS: &str = "BROKKR_BLESS=1 cargo test -p brokkr-runtime --test it witness_digests::";

/// What a run does with the measured table.
#[derive(Debug, PartialEq)]
enum Mode {
    /// Compare every witness and fail once, naming all that moved.
    Compare,
    /// Rewrite the table with what was measured.
    Bless,
    /// Neither: the variables ask for something this suite will not do.
    Refused(String),
}

/// The mode `BROKKR_BLESS` and `CI` select. The caller reads both once
/// and passes them in, so no test writes the process environment. Only
/// `1` blesses, and never where CI is set: CI compares the table and
/// never rewrites it.
fn mode(bless: Option<&OsStr>, ci: Option<&OsStr>) -> Mode {
    match (bless, ci) {
        (None, _) => Mode::Compare,
        (Some(_), Some(_)) => Mode::Refused(format!(
            "BROKKR_BLESS refuses to run where CI is set: CI compares {TABLE} and never rewrites it"
        )),
        (Some(value), None) if value == OsStr::new("1") => Mode::Bless,
        (Some(value), None) => Mode::Refused(format!(
            "BROKKR_BLESS must be 1 or unset, not '{}'",
            value.to_string_lossy()
        )),
    }
}

/// Every bundle in the tree, relative to the workspace and sorted: each
/// directory under `recipes/` and `bundles/` that holds a `bundle.json`.
/// The witness set is this, never the table's own keys, so a row dropped
/// from the table reads as a moved witness instead of an unchecked one.
fn bundles_in_tree(root: &Path) -> Vec<String> {
    let mut dirs = Vec::new();
    for parent in ["recipes", "bundles"] {
        for entry in std::fs::read_dir(root.join(parent))
            .unwrap_or_else(|e| panic!("{parent} must be readable: {e}"))
        {
            let name = entry.expect("a bundle entry").file_name();
            let relative = format!("{parent}/{}", name.to_string_lossy());
            if root.join(&relative).join("bundle.json").is_file() {
                dirs.push(relative);
            }
        }
    }
    dirs.sort();
    dirs
}

/// Measure every witness in the tree: each bundle's compiled manifest
/// digest, and the bytes of every charter the library ships, so a bundle
/// or charter added, removed or dropped from the table is a moved witness.
fn measure(root: &Path) -> Witnesses {
    let bundles = bundles_in_tree(root)
        .into_iter()
        .map(|relative| {
            // Explicit roots: since decision 0021 a compile reads adapter
            // data for inline gates too, even though they adopt no agent —
            // a gate seat's trust tier is declared there.
            let bundle = Bundle::compile_with(
                &root.join(&relative),
                &root.join("agents"),
                &root.join("adapters"),
            )
            .unwrap_or_else(|e| panic!("{relative} must compile: {e}"));
            (relative, bundle.manifest_digest())
        })
        .collect();
    let charters = std::fs::read_dir(root.join("agents/charters"))
        .expect("agents/charters must be readable")
        .map(|entry| {
            let path = entry.expect("a charter entry").path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let bytes = std::fs::read(&path).expect("a readable charter");
            (name, brokkr_core::canonical::sha256_bytes(&bytes))
        })
        .collect();
    Witnesses { bundles, charters }
}

/// One moved witness: its section and name, then its old and new value;
/// `None` is a witness absent on that side.
type Moved<'a> = (String, Option<&'a str>, Option<&'a str>);

/// Every witness whose pinned and measured values differ.
fn moved<'a>(pinned: &'a Witnesses, measured: &'a Witnesses) -> Vec<Moved<'a>> {
    let mut out = Vec::new();
    for (section, old, new) in [
        ("bundles", &pinned.bundles, &measured.bundles),
        ("charters", &pinned.charters, &measured.charters),
    ] {
        let names: BTreeSet<&String> = old.keys().chain(new.keys()).collect();
        for name in names {
            let (was, is) = (old.get(name), new.get(name));
            if was != is {
                out.push((
                    format!("{section} {name}"),
                    was.map(String::as_str),
                    is.map(String::as_str),
                ));
            }
        }
    }
    out
}

/// The failure a drift prints: every moved witness in one Markdown table
/// of old → new values, ready to paste into the pull request that
/// re-pins them.
fn drift_report(moved: &[Moved]) -> String {
    let mut report = format!(
        "{} witness(es) moved. Say why in the commit, then re-pin with `{BLESS}`:\n\n\
         | witness | old | new |\n|---|---|---|\n",
        moved.len()
    );
    for (witness, old, new) in moved {
        let (old, new) = (old.unwrap_or("absent"), new.unwrap_or("absent"));
        writeln!(report, "| {witness} | {old} | {new} |").expect("a String accepts writes");
    }
    report
}

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

/// The whole table, bundles and charters, in one measurement: compared by
/// default, rewritten under `BROKKR_BLESS=1`. One test does both halves so
/// a bless run writes the file once.
#[test]
fn pinned_bundles_keep_their_recorded_digest() {
    let root = workspace();
    let pinned = Witnesses::load(&root);
    let measured = measure(&root);
    let moved = moved(&pinned, &measured);
    let bless = std::env::var_os("BROKKR_BLESS");
    let ci = std::env::var_os("CI");
    match mode(bless.as_deref(), ci.as_deref()) {
        Mode::Refused(why) => panic!("{why}"),
        Mode::Compare => assert!(moved.is_empty(), "{}", drift_report(&moved)),
        Mode::Bless => {
            let text = serde_json::to_string_pretty(&measured).expect("the table serialises");
            std::fs::write(root.join(TABLE), format!("{text}\n")).expect("the table is writable");
            eprintln!("re-pinned {} witness(es) in {TABLE}", moved.len());
        }
    }
}

/// Bless is a developer's command: where CI is set it refuses with the
/// reason, whatever `BROKKR_BLESS` says, and a value other than `1` is
/// refused rather than read as either mode.
#[test]
fn bless_refuses_where_ci_is_set_and_reads_only_one() {
    let (one, yes) = (Some(OsStr::new("1")), Some(OsStr::new("yes")));
    let ci = Some(OsStr::new("true"));
    assert_eq!(mode(None, None), Mode::Compare);
    assert_eq!(mode(None, ci), Mode::Compare);
    assert_eq!(mode(one, None), Mode::Bless);
    let refused = Mode::Refused(
        "BROKKR_BLESS refuses to run where CI is set: CI compares \
         crates/brokkr-runtime/tests/witnesses.json and never rewrites it"
            .to_string(),
    );
    assert_eq!(mode(one, ci), refused);
    assert_eq!(mode(yes, ci), refused);
    assert_eq!(
        mode(yes, None),
        Mode::Refused("BROKKR_BLESS must be 1 or unset, not 'yes'".to_string())
    );
}

/// A drift names every moved witness in one run, in section and name
/// order, and a witness that appeared or vanished reads as `absent` on
/// the side it is missing from.
#[test]
fn a_drift_reports_every_moved_witness_in_one_table() {
    let table = |bundles: &[(&str, &str)], charters: &[(&str, &str)]| Witnesses {
        bundles: bundles
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        charters: charters
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    };
    let pinned = table(
        &[
            ("recipes/a", "a0"),
            ("recipes/b", "b0"),
            ("recipes/c", "c0"),
        ],
        &[("gone.md", "g0"), ("kept.md", "k0")],
    );
    let measured = table(
        &[
            ("recipes/a", "a1"),
            ("recipes/b", "b0"),
            ("recipes/c", "c1"),
        ],
        &[("kept.md", "k0"), ("new.md", "n1")],
    );
    assert_eq!(
        drift_report(&moved(&pinned, &measured)),
        "4 witness(es) moved. Say why in the commit, then re-pin with \
         `BROKKR_BLESS=1 cargo test -p brokkr-runtime --test it witness_digests::`:\n\
         \n\
         | witness | old | new |\n\
         |---|---|---|\n\
         | bundles recipes/a | a0 | a1 |\n\
         | bundles recipes/c | c0 | c1 |\n\
         | charters gone.md | g0 | absent |\n\
         | charters new.md | absent | n1 |\n"
    );
    assert!(moved(&pinned, &pinned).is_empty());
}

/// Decision 0046 ruling 1: the contract a compiled manifest claims is
/// run-manifest/v9 — v8 plus the `boundary` map beside `hands`, present
/// exactly with it. Every witness validates, and the ones that box
/// something carry both keys over the same site labels.
///
/// Since decision 0065 the contract a compiled manifest claims is
/// run-manifest/v11: v9's `hands`/`boundary` clauses carried forward
/// unchanged, plus the REQUIRED `capabilities` section every compile now
/// writes. The test keeps its name and its boundary assertions; the file
/// it validates against is the version the manifests actually are.
#[test]
fn every_witness_manifest_satisfies_the_v9_contract_it_claims() {
    let root = workspace();
    let schema: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("contracts/run-manifest.v11.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::draft7::new(&schema).unwrap();
    for relative in bundles_in_tree(&root) {
        let bundle = Bundle::compile_with(
            &root.join(&relative),
            &root.join("agents"),
            &root.join("adapters"),
        )
        .unwrap();
        assert!(
            validator.is_valid(&bundle.manifest),
            "{relative} emits a manifest outside run-manifest/v11"
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
    let dirs = bundles_in_tree(&root);
    assert!(dirs.len() >= 5, "expected the shipped recipes and bundles");
    for dir in dirs {
        // Against the in-tree library roots explicitly, rather than by
        // changing the process working directory: two tests share one
        // process, and a global `set_current_dir` would make this suite
        // order-dependent.
        Bundle::compile_with(
            &root.join(&dir),
            &root.join("agents"),
            &root.join("adapters"),
        )
        .unwrap_or_else(|e| panic!("{dir} must compile: {e}"));
    }
}
