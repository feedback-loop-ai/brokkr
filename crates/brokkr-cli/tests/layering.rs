//! Decision 0071 ruling 1's gates that `cargo-deny` and clippy cannot
//! hold alone (#336).
//!
//! `deny.toml` is the one home of the crate graph: each `[bans]` entry
//! names the only crates allowed to depend on it directly. cargo-deny
//! judges the shipped graph with dev-dependencies left out, and Cargo
//! accepts a dev edge that cycles, so the test here holds every manifest
//! edge, dev ones included, to the same table. The purity half checks
//! that core and view share one `clippy.toml` which repeats the root's
//! thresholds, and backs clippy's item paths with a `git grep` over
//! whole modules.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::Command;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = workspace().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Normal,
    Dev,
    Build,
}

/// Dev edges the tests take beyond the shipped graph. Each points down
/// or sideways, never above its crate's layer.
const DEV_ONLY: [(&str, &str); 3] = [
    ("brokkr-runtime", "rusqlite"),
    ("brokkr-bridge", "rusqlite"),
    ("brokkr-cli", "rusqlite"),
];

/// The kind of dependency table a manifest header opens, if any.
fn table_kind(header: &str) -> Option<Kind> {
    let kinds = [
        ("dev-dependencies", Kind::Dev),
        ("build-dependencies", Kind::Build),
        ("dependencies", Kind::Normal),
    ];
    kinds.into_iter().find_map(|(word, kind)| {
        assert!(
            !header.starts_with(&format!("{word}.")) && !header.contains(&format!(".{word}.")),
            "[{header}]: a one-dependency table is not read here; write it inline"
        );
        (header == word || header.ends_with(&format!(".{word}"))).then_some(kind)
    })
}

/// Every `(kind, name)` a manifest declares, target-specific tables
/// included. A renamed dependency would hide its real name, so it is
/// refused rather than read.
fn edges(manifest: &str) -> BTreeSet<(Kind, String)> {
    let mut kind = None;
    let mut edges = BTreeSet::new();
    for line in manifest.lines() {
        if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            kind = table_kind(header);
            continue;
        }
        let Some(kind) = kind else { continue };
        if !line.starts_with(|c: char| c.is_ascii_alphanumeric()) {
            continue;
        }
        assert!(
            !line.contains("package ="),
            "{line}: a renamed dependency is not read here"
        );
        let name = line
            .split(['=', '.', ' '])
            .next()
            .expect("a key names the dependency");
        edges.insert((kind, name.to_string()));
    }
    edges
}

/// `deny.toml`'s `[bans]` entries: each governed crate and the crates
/// allowed to depend on it directly.
fn allowed_parents(deny: &str) -> BTreeMap<String, BTreeSet<String>> {
    let bans = deny
        .split_once("\n[bans]\n")
        .expect("deny.toml has a [bans] table")
        .1;
    bans.lines()
        .filter_map(|line| line.trim().strip_prefix("{ crate = \""))
        .map(|entry| {
            let (name, rest) = entry.split_once('"').expect("a quoted crate name");
            let wrappers = rest
                .split_once("wrappers = [")
                .and_then(|(_, list)| list.split_once(']'))
                .unwrap_or_else(|| panic!("{name} names its wrappers"))
                .0;
            let parents = wrappers
                .split(',')
                .map(|parent| parent.trim().trim_matches('"'))
                .filter(|parent| !parent.is_empty())
                .map(str::to_string)
                .collect();
            (name.to_string(), parents)
        })
        .collect()
}

fn workspace_crates() -> BTreeSet<String> {
    std::fs::read_dir(workspace().join("crates"))
        .expect("crates/")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("Cargo.toml").is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// Ruling 1, the one-way graph: every workspace crate but the binary is
/// governed by `deny.toml`, and every manifest edge to a governed crate
/// comes from a parent its entry allows, dev edges included.
#[test]
fn every_manifest_edge_is_in_the_allowed_graph() {
    let mut parents = allowed_parents(&read("deny.toml"));
    let crates = workspace_crates();
    let ungoverned: Vec<&String> = crates
        .iter()
        .filter(|c| !parents.contains_key(*c))
        .collect();
    assert_eq!(
        ungoverned,
        ["brokkr-cli"],
        "deny.toml places every library crate in the graph"
    );
    parents.insert("brokkr-cli".to_string(), BTreeSet::new());

    let mut refused = Vec::new();
    let mut checked = 0;
    for krate in &crates {
        for (kind, name) in edges(&read(&format!("crates/{krate}/Cargo.toml"))) {
            let Some(allowed) = parents.get(&name) else {
                continue;
            };
            checked += 1;
            let dev_only = kind == Kind::Dev && DEV_ONLY.contains(&(krate.as_str(), name.as_str()));
            if !allowed.contains(krate) && !dev_only {
                refused.push(format!("{krate} -> {name} ({kind:?})"));
            }
        }
    }
    assert_eq!(
        refused,
        Vec::<String>::new(),
        "edges outside the one-way graph (decision 0071 ruling 1)"
    );
    assert_eq!(checked, 29, "the governed edges read from the manifests");
}

/// The threshold lines of a clippy config: every top-level key but the
/// crate-local disallowed lists.
fn thresholds(config: &str) -> BTreeSet<String> {
    config
        .lines()
        .filter(|line| line.starts_with(|c: char| c.is_ascii_alphabetic()))
        .filter(|line| !line.starts_with("disallowed-"))
        .map(str::to_string)
        .collect()
}

/// A crate-level `clippy.toml` replaces the root one rather than merging
/// with it, so the pure crates' config repeats every root threshold.
#[test]
fn the_pure_crates_share_one_clippy_config_that_repeats_the_root_thresholds() {
    let core = read("crates/brokkr-core/clippy.toml");
    assert_eq!(
        read("crates/brokkr-view/clippy.toml"),
        core,
        "core and view hold one purity config"
    );
    let root: String = ["clippy.toml", ".clippy.toml"]
        .iter()
        .map(
            |name| match std::fs::read_to_string(workspace().join(name)) {
                Ok(text) => text,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
                Err(error) => panic!("{name}: {error}"),
            },
        )
        .collect();
    assert_eq!(
        thresholds(&core),
        thresholds(&root),
        "the root's thresholds, repeated"
    );
}

/// Clippy's disallowed lists name items, never whole modules. This names
/// the modules and the clock and randomness sources, in non-test source.
const EFFECTFUL: &str = concat!(
    r"std::(fs|process|env|net|thread|io|os)([^a-z_]|$)",
    r"|std::\{([^}]*[ ,])?(fs|process|env|net|thread|io|os)([^a-z_]|$)",
    r"|SystemTime|Instant|now_utc|now_local|new_v4|RandomState",
    r"|(print|println|eprint|eprintln|dbg)!",
);

/// Ruling 1's backstop, in the style of `scripts/coverage-exact.sh`: a
/// `git grep` over core's and view's production source finds nothing.
#[test]
fn the_pure_crates_name_no_effectful_module() {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace())
        .args(["grep", "-n", "-E", EFFECTFUL, "--"])
        .args(["crates/brokkr-core/src", "crates/brokkr-view/src"])
        .args([":(exclude)*/tests.rs", ":(exclude)*/tests/*"])
        .output()
        .expect("git runs");
    assert_eq!(
        (
            output.status.code(),
            String::from_utf8_lossy(&output.stdout).as_ref()
        ),
        (Some(1), ""),
        "core and view are pure (decision 0071 ruling 1): effects live in store and runtime"
    );
}
