//! Decision 0071 ruling 1's gates that `cargo-deny` and clippy cannot
//! hold alone (#336). Each is closed: what it does not list, it refuses.
//!
//! `deny.toml` is the one home of the crate graph: each `[bans]` entry
//! names the only crates allowed to depend on it directly. cargo-deny
//! cannot tell a dev edge from a normal one, and Cargo accepts a dev edge
//! that cycles, so the test here holds every edge `cargo metadata`
//! reports, dev ones included, to the same table, read strictly. A pure
//! crate's own edges are held to a closed set, which the test keeps in
//! agreement with the table, and every resolved package outside the
//! workspace must come from crates.io. The purity half checks that core
//! and view share one `clippy.toml` which repeats the root's thresholds,
//! and lexes their production source, comments and literals stripped, so
//! that every path under `std`, `core` or `alloc` and every macro they
//! name falls under an allowlist.
//!
//! The threat model, as the operator ruled it on 2026-09-26: these gates
//! catch every realistic way an effect slips into core or view by
//! mistake, and fail closed on whatever they cannot read. Deliberate
//! evasion by a determined adversary is out of scope; a review rates an
//! exotic evasion as a low residual, not a security hold.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use serde::Deserialize;

mod scan;

#[path = "../support/test_paths.rs"]
mod test_paths;
#[path = "../support/workspace.rs"]
mod workspace_root;
use workspace_root::{read, workspace};

/// The part of `cargo metadata --format-version 1` this test reads. Cargo
/// adds fields to the format over time, so unknown fields are ignored; an
/// unknown dependency kind fails the parse.
#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    version: String,
    /// `None` for a path package, the registry or git URL otherwise.
    source: Option<String>,
    dependencies: Vec<Dependency>,
    /// Every root Cargo compiles for the package: lib, bins, tests,
    /// examples, benches and a build script alike.
    targets: Vec<Target>,
}

/// One compiled root: its kinds as Cargo names them (`lib`, `test`,
/// `custom-build`, …) and the file it starts from.
#[derive(Deserialize)]
struct Target {
    kind: Vec<String>,
    src_path: PathBuf,
}

/// One manifest edge as Cargo read it: `name` is the package's own name
/// even when the manifest renames it, and every key form, quoted or
/// dotted, and every target table arrives here the same way.
#[derive(Deserialize)]
struct Dependency {
    name: String,
    /// Cargo writes `null` for a normal edge.
    kind: Option<Kind>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Normal,
    Dev,
    Build,
}

/// The workspace's packages, their edges, their targets and every package
/// they resolve to, as Cargo reads the manifests and the lockfile, read
/// once per test binary. Cargo's stderr carries progress such as a wait
/// for the package cache lock, so only its exit status is judged.
fn metadata() -> &'static Metadata {
    static METADATA: OnceLock<Metadata> = OnceLock::new();
    METADATA.get_or_init(|| {
        let output = Command::new(env!("CARGO"))
            .args([
                "metadata",
                "--format-version",
                "1",
                "--all-features",
                "--locked",
            ])
            .arg("--manifest-path")
            .arg(workspace().join("Cargo.toml"))
            .output()
            .expect("cargo runs");
        assert_eq!(
            output.status.code(),
            Some(0),
            "cargo metadata reads the workspace: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("cargo metadata's format 1")
    })
}

/// The one registry every package outside the workspace comes from.
const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

/// Ruling 1's closed sets: every crate a pure crate may depend on
/// directly, of any kind. An edge outside its set is refused whether or
/// not `deny.toml` governs the crate, and the test holds the sets and the
/// table's wrappers to one another.
const PURE: [(&str, &[&str]); 2] = [
    (
        "brokkr-core",
        &[
            "hex",
            "serde",
            "serde_json",
            "sha2",
            "thiserror",
            "time",
            "url",
        ],
    ),
    ("brokkr-view", &["brokkr-core", "serde", "serde_json"]),
];

/// The `deny.toml` wrappers the tests alone take: each is a dev edge that
/// points down or sideways, never above its crate's layer. cargo-deny
/// admits a wrapper's edge of any kind, so this test refuses a normal or
/// build edge between these pairs.
const DEV_ONLY: [(&str, &str); 3] = [
    ("brokkr-runtime", "rusqlite"),
    ("brokkr-bridge", "rusqlite"),
    ("brokkr-cli", "rusqlite"),
];

/// The workspace crates `deny.toml` does not place in the graph: the
/// binary, which nothing may depend on, and the seatbelt probe (#341), a
/// `publish = false` test helper whose only workspace edge is a
/// dev-dependency on brokkr-protocol. A crate added here is one no other
/// crate may name, so cargo-deny has nothing to govern.
const UNGOVERNED: [&str; 2] = ["brokkr-cli", "brokkr-seatbelt-probe"];

/// Each governed crate and the crates allowed to depend on it directly.
type Graph = BTreeMap<String, BTreeSet<String>>;

/// A `deny.toml` line the strict reader below does not understand.
#[derive(Debug, PartialEq)]
struct NotUnderstood {
    line: usize,
    text: String,
}

/// `deny.toml`'s `[bans]` table as the strict reader understands it.
#[derive(Debug, Default, PartialEq)]
struct Bans {
    parents: Graph,
    /// `name@version` duplicates `multiple-versions` lets stand (#337).
    skips: Vec<(String, String)>,
    multiple_versions_denied: bool,
}

/// Which array of the `[bans]` table a line sits in.
#[derive(Clone, Copy, PartialEq)]
enum Array {
    Out,
    Deny,
    Skip,
}

/// `deny.toml`'s `[bans]` entries, read strictly: inside the table every
/// line is `multiple-versions = "deny"`, `deny = [`, `skip = [`, `]`, a
/// comment, or one whole entry of the open array in the house form, and
/// outside it no line names `bans` or could spell it through an escape or
/// a multi-line string. Anything else is refused, never skipped, so the
/// graph read here is the graph cargo-deny enforces.
fn read_bans(deny: &str) -> Result<Bans, NotUnderstood> {
    let mut bans = Bans::default();
    let (mut in_bans, mut seen_bans, mut array) = (false, false, Array::Out);
    for (index, line) in deny.lines().enumerate() {
        let refuse = || NotUnderstood {
            line: index + 1,
            text: line.to_string(),
        };
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && array == Array::Out {
            in_bans = line == "[bans]";
            if (in_bans && seen_bans) || (!in_bans && trimmed.contains("bans")) {
                return Err(refuse());
            }
            seen_bans |= in_bans;
            continue;
        }
        if !in_bans {
            if ["bans", "\\", "\"\"\"", "'''"]
                .iter()
                .any(|word| trimmed.contains(word))
            {
                return Err(refuse());
            }
            continue;
        }
        array = bans_line(&mut bans, line, array).ok_or_else(refuse)?;
    }
    Ok(bans)
}

/// One line inside `[bans]`, folded into `bans`: the array it leaves
/// open, or `None` for a line the house form does not have.
fn bans_line(bans: &mut Bans, line: &str, array: Array) -> Option<Array> {
    let trimmed = line.trim();
    match (array, trimmed) {
        (Array::Out, "multiple-versions = \"deny\"") if !bans.multiple_versions_denied => {
            bans.multiple_versions_denied = true;
            Some(Array::Out)
        }
        (Array::Out, "deny = [") => Some(Array::Deny),
        (Array::Out, "skip = [") => Some(Array::Skip),
        (Array::Deny | Array::Skip, "]") => Some(Array::Out),
        (Array::Deny, _) => {
            let (name, wrappers) = ban_entry(line)?;
            bans.parents
                .insert(name, wrappers)
                .is_none()
                .then_some(Array::Deny)
        }
        (Array::Skip, _) => {
            bans.skips.push(skip_entry(line)?);
            Some(Array::Skip)
        }
        _ => None,
    }
}

/// The `[bans]` graph alone, as [`read_bans`] reads it.
fn allowed_parents(deny: &str) -> Result<Graph, NotUnderstood> {
    read_bans(deny).map(|bans| bans.parents)
}

/// A crate name with no quoting, spacing or version in it.
fn plain_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// One `[bans]` entry in the house form, or `None`:
/// `  { crate = "name", wrappers = ["a", "b"], reason = "…" },`
fn ban_entry(line: &str) -> Option<(String, BTreeSet<String>)> {
    let rest = line.strip_prefix("  { crate = \"")?;
    let (name, rest) = rest.split_once("\", wrappers = [")?;
    let (list, rest) = rest.split_once("], reason = \"")?;
    let reason = rest.strip_suffix("\" },")?;
    if !plain_name(name) || reason.contains(['"', '\\']) {
        return None;
    }
    let wrappers = list
        .split(", ")
        .map(|wrapper| {
            let wrapper = wrapper.strip_prefix('"')?.strip_suffix('"')?;
            plain_name(wrapper).then(|| wrapper.to_string())
        })
        .collect::<Option<BTreeSet<String>>>()?;
    Some((name.to_string(), wrappers))
}

/// One `skip` entry in the house form, or `None`:
/// `  { crate = "name@version", reason = "…" },`
fn skip_entry(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("  { crate = \"")?;
    let (spec, rest) = rest.split_once("\", reason = \"")?;
    let reason = rest.strip_suffix("\" },")?;
    let (name, version) = spec.split_once('@')?;
    let exact = !version.is_empty()
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'));
    (plain_name(name) && exact && !reason.contains(['"', '\\']))
        .then(|| (name.to_string(), version.to_string()))
}

/// The strict reader refuses each form it does not understand by its
/// line, rather than skip it.
#[test]
fn the_bans_reader_refuses_what_it_does_not_understand() {
    let entry = r#"  { crate = "rusqlite", wrappers = ["brokkr-store"], reason = "r" },"#;
    let table = |line: &str| format!("[bans]\ndeny = [\n{entry}\n{line}\n]\n");
    let refused = |line: usize, text: &str| {
        Err(NotUnderstood {
            line,
            text: text.to_string(),
        })
    };
    assert_eq!(
        allowed_parents(&table("")),
        Ok(Graph::from([(
            "rusqlite".to_string(),
            BTreeSet::from(["brokkr-store".to_string()])
        )]))
    );
    for line in [
        r#"    { crate = "ureq", wrappers = ["brokkr-bridge"], reason = "r" },"#,
        r#"  { crate = "ureq", wrappers = ["brokkr-bridge"], reason = "r" }, # c"#,
        r#"  { crate = "ureq","#,
        r#"  { crate = "ureq@2", wrappers = ["brokkr-bridge"], reason = "r" },"#,
        r#"  { crate = "rusqlite", wrappers = ["brokkr-cli"], reason = "r" },"#,
        r#"allow = ["ureq"]"#,
    ] {
        assert_eq!(allowed_parents(&table(line)), refused(4, line), "{line}");
    }
    let escaped = format!("\"{}u0062ans\".deny = []", '\\');
    for (text, at, line) in [
        ("bans.deny = []", 1, "bans.deny = []"),
        ("[ bans ]", 1, "[ bans ]"),
        (escaped.as_str(), 1, escaped.as_str()),
        ("notes = \"\"\"\n[bans]\n\"\"\"", 1, "notes = \"\"\""),
        ("[bans]\n[graph]\n[bans]", 3, "[bans]"),
    ] {
        assert_eq!(allowed_parents(text), refused(at, line), "{text}");
    }
}

/// `multiple-versions` is denied, and every skipped duplicate is still one
/// in the lockfile, so the skip list can only shrink (#337).
#[test]
fn the_duplicates_gate_skips_only_what_the_lockfile_still_duplicates() {
    let bans = read_bans(&read("deny.toml")).expect("deny.toml's [bans] reads");
    assert!(
        bans.multiple_versions_denied,
        "deny.toml refuses a second version of a crate"
    );
    let mut versions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for block in read("Cargo.lock").split("[[package]]").skip(1) {
        let field = |key: &str| {
            block
                .lines()
                .find_map(|line| line.strip_prefix(key))
                .map(|value| value.trim_matches('"').to_string())
        };
        if let (Some(name), Some(version)) = (field("name = "), field("version = ")) {
            versions.entry(name).or_default().insert(version);
        }
    }
    let stale: Vec<String> = bans
        .skips
        .iter()
        .filter(|(name, version)| {
            versions
                .get(name)
                .is_none_or(|all| all.len() < 2 || !all.contains(version))
        })
        .map(|(name, version)| format!("{name}@{version}"))
        .collect();
    assert_eq!(
        stale,
        Vec::<String>::new(),
        "skips the lockfile no longer duplicates"
    );
}

/// The duplicate gate's two lines are read in their house form, and every
/// near miss is refused rather than skipped.
#[test]
fn the_bans_reader_reads_the_duplicate_gate_in_its_house_form() {
    let text = "[bans]\nmultiple-versions = \"deny\"\nskip = [\n  { crate = \"syn@3.0.3\", reason = \"r\" },\n]\ndeny = [\n]\n";
    let bans = read_bans(text).expect("the house form reads");
    assert!(bans.multiple_versions_denied);
    assert_eq!(bans.skips, [("syn".to_string(), "3.0.3".to_string())]);
    for bad in [
        "[bans]\nmultiple-versions = \"warn\"\n",
        "[bans]\nmultiple-versions = \"deny\"\nmultiple-versions = \"deny\"\n",
        "[bans]\nskip = [\n  { crate = \"syn\", reason = \"r\" },\n]\n",
        "[bans]\nskip = [\n  { crate = \"syn@3\", wrappers = [\"a\"], reason = \"r\" },\n]\n",
        "[bans]\nskip = [\n  \"syn@3.0.3\",\n]\n",
        "[bans]\n]\n",
    ] {
        assert!(read_bans(bad).is_err(), "{bad}");
    }
}

/// Every edge of `members` the allowed graph refuses, as
/// `parent -> child (kind): why`. No edge is skipped: a pure crate's edge
/// must be in its closed set, a workspace crate must be governed, a
/// governed crate's wrappers must name the parent, and a dev-only wrapper
/// admits a dev edge alone. An external crate `deny.toml` does not govern
/// is open to every crate but the pure ones.
fn refused_edges(members: &[&Package], parents: &Graph) -> Vec<String> {
    let workspace: BTreeSet<&str> = members.iter().map(|member| member.name.as_str()).collect();
    let mut refused = Vec::new();
    for package in members {
        let closed = PURE
            .iter()
            .find(|(name, _)| *name == package.name)
            .map(|(_, set)| *set);
        for dependency in &package.dependencies {
            let kind = dependency.kind.unwrap_or(Kind::Normal);
            let pair = (package.name.as_str(), dependency.name.as_str());
            let allowed = parents.get(&dependency.name);
            let why = if closed.is_some_and(|set| !set.contains(&pair.1)) {
                "outside the pure crate's closed set"
            } else if allowed.is_none() && workspace.contains(pair.1) {
                "a workspace crate deny.toml does not govern"
            } else if allowed.is_some_and(|allowed| !allowed.contains(pair.0)) {
                "its [bans] wrappers do not name the parent"
            } else if DEV_ONLY.contains(&pair) && kind != Kind::Dev {
                "a dev-only wrapper"
            } else {
                continue;
            };
            refused.push(format!("{} -> {} ({kind:?}): {why}", pair.0, pair.1));
        }
    }
    refused
}

/// The refusal names each form of edge the graph does not allow, and
/// passes an external crate no entry governs only above the pure crates.
#[test]
fn the_graph_refuses_every_edge_it_does_not_allow() {
    let parents = allowed_parents(&read("deny.toml")).expect("deny.toml's [bans] reads");
    let package = |name: &str, dependencies: &[(&str, Option<Kind>)]| Package {
        id: name.to_string(),
        name: name.to_string(),
        version: "0.0.0".to_string(),
        source: None,
        dependencies: dependencies
            .iter()
            .map(|(name, kind)| Dependency {
                name: name.to_string(),
                kind: *kind,
            })
            .collect(),
        targets: Vec::new(),
    };
    let packages = [
        package(
            "brokkr-core",
            &[("time", None), ("tempfile", Some(Kind::Dev))],
        ),
        package("brokkr-view", &[("brokkr-core", None), ("uuid", None)]),
        package(
            "brokkr-store",
            &[("tempfile", None), ("brokkr-runtime", Some(Kind::Dev))],
        ),
        package("brokkr-runtime", &[("rusqlite", Some(Kind::Build))]),
        package("brokkr-bridge", &[("brokkr-cli", Some(Kind::Dev))]),
        package("brokkr-cli", &[("rusqlite", Some(Kind::Dev))]),
    ];
    assert_eq!(
        refused_edges(&packages.iter().collect::<Vec<_>>(), &parents),
        [
            "brokkr-core -> tempfile (Dev): outside the pure crate's closed set",
            "brokkr-view -> uuid (Normal): outside the pure crate's closed set",
            "brokkr-store -> brokkr-runtime (Dev): its [bans] wrappers do not name the parent",
            "brokkr-runtime -> rusqlite (Build): a dev-only wrapper",
            "brokkr-bridge -> brokkr-cli (Dev): a workspace crate deny.toml does not govern",
        ]
    );
}

/// Ruling 1, the one-way graph: every workspace crate but the binary is
/// governed by `deny.toml`, the pure crates' closed sets agree with its
/// wrappers, every edge Cargo reads is allowed, and every package the
/// workspace resolves to outside it comes from crates.io.
#[test]
fn every_manifest_edge_is_in_the_allowed_graph() {
    let parents = allowed_parents(&read("deny.toml")).expect("deny.toml's [bans] reads");
    let metadata = metadata();
    let members: Vec<&Package> = metadata
        .packages
        .iter()
        .filter(|package| metadata.workspace_members.contains(&package.id))
        .collect();
    let ungoverned: Vec<&str> = members
        .iter()
        .map(|package| package.name.as_str())
        .filter(|name| !parents.contains_key(*name))
        .collect();
    assert_eq!(
        ungoverned, UNGOVERNED,
        "deny.toml places every library crate in the graph"
    );
    let disagreements: Vec<String> = PURE
        .iter()
        .flat_map(|(pure, closed)| {
            parents
                .iter()
                .filter(|(child, wrappers)| {
                    wrappers.contains(*pure) != closed.contains(&child.as_str())
                })
                .map(move |(child, _)| format!("{pure} -> {child}"))
        })
        .collect();
    assert_eq!(
        disagreements,
        Vec::<String>::new(),
        "a pure crate's closed set and deny.toml's wrappers name the same governed crates"
    );
    assert_eq!(
        refused_edges(&members, &parents),
        Vec::<String>::new(),
        "edges outside the one-way graph (decision 0071 ruling 1)"
    );
    let dev_only_used: BTreeSet<(&str, &str)> = members
        .iter()
        .flat_map(|package| {
            package
                .dependencies
                .iter()
                .filter(|dependency| dependency.kind == Some(Kind::Dev))
                .map(|dependency| (package.name.as_str(), dependency.name.as_str()))
        })
        .filter(|pair| DEV_ONLY.contains(pair))
        .collect();
    assert_eq!(
        dev_only_used,
        BTreeSet::from(DEV_ONLY),
        "every dev-only wrapper is a dev edge in use"
    );
    let unknown: Vec<String> = metadata
        .packages
        .iter()
        .filter(|package| {
            let member = metadata.workspace_members.contains(&package.id);
            package.source.as_deref() != (!member).then_some(CRATES_IO)
        })
        .map(|package| format!("{} ({:?})", package.name, package.source))
        .collect();
    assert_eq!(
        unknown,
        Vec::<String>::new(),
        "every package is a workspace member or comes from crates.io"
    );
}

/// Every root a pure crate compiles that the purity scan would not read,
/// as `crate kind at path`: the scan walks `src/` from `src/lib.rs`, so the
/// crate must have exactly that one lib, and every other root must be a
/// test under `tests/`. A build script, a bin, an example, a bench, a lib
/// elsewhere or a second lib is refused.
fn target_refusals(package: &Package, dir: &Path) -> Vec<String> {
    let mut refused = Vec::new();
    let mut libs = 0;
    for target in &package.targets {
        let kind = target.kind.join(",");
        let reached = match kind.as_str() {
            "lib" => {
                libs += 1;
                target.src_path == dir.join("src").join("lib.rs")
            }
            "test" => target.src_path.starts_with(dir.join("tests")),
            _ => false,
        };
        if !reached {
            let at = target
                .src_path
                .strip_prefix(dir)
                .unwrap_or(&target.src_path);
            refused.push(format!("{} {kind} at {}", package.name, at.display()));
        }
    }
    if libs != 1 {
        refused.push(format!("{}: {libs} lib targets", package.name));
    }
    refused
}

/// The target check refuses each root outside the scan's reach.
#[test]
fn the_target_check_refuses_every_root_the_scan_does_not_read() {
    let dir = Path::new("/w/crates/brokkr-core");
    let target = |kind: &str, path: &str| Target {
        kind: vec![kind.to_string()],
        src_path: dir.join(path),
    };
    let package = |targets: Vec<Target>| Package {
        id: "brokkr-core".to_string(),
        name: "brokkr-core".to_string(),
        version: "0.0.0".to_string(),
        source: None,
        dependencies: Vec::new(),
        targets,
    };
    let clean = package(vec![
        target("lib", "src/lib.rs"),
        target("test", "tests/fold_test.rs"),
    ]);
    assert_eq!(target_refusals(&clean, dir), Vec::<String>::new());
    let planted = package(vec![
        target("lib", "other.rs"),
        target("custom-build", "build.rs"),
        target("bin", "src/main.rs"),
        target("example", "examples/e.rs"),
        target("bench", "benches/b.rs"),
        target("test", "src/pure_tests.rs"),
        target("lib", "src/lib.rs"),
    ]);
    assert_eq!(
        target_refusals(&planted, dir),
        [
            "brokkr-core lib at other.rs",
            "brokkr-core custom-build at build.rs",
            "brokkr-core bin at src/main.rs",
            "brokkr-core example at examples/e.rs",
            "brokkr-core bench at benches/b.rs",
            "brokkr-core test at src/pure_tests.rs",
            "brokkr-core: 2 lib targets",
        ]
    );
    assert_eq!(
        target_refusals(&package(vec![]), dir),
        ["brokkr-core: 0 lib targets"]
    );
}

/// Ruling 1: each pure crate compiles only roots the purity scan reads, as
/// Cargo reports them.
#[test]
fn the_pure_crates_compile_only_what_the_scan_reads() {
    let metadata = metadata();
    let root = workspace();
    let refused: Vec<String> = PURE
        .iter()
        .flat_map(|(name, _)| {
            let package = metadata
                .packages
                .iter()
                .find(|package| {
                    package.name == *name && metadata.workspace_members.contains(&package.id)
                })
                .unwrap_or_else(|| panic!("{name} is a workspace member"));
            target_refusals(package, &root.join("crates").join(name))
        })
        .collect();
    assert_eq!(
        refused,
        Vec::<String>::new(),
        "a pure crate compiles a root the purity scan never reads (decision 0071 ruling 1)"
    );
}

/// The effectful surface of every third-party crate a pure crate may name,
/// read from that crate's source at the version `Cargo.lock` pins: each
/// public item that reads a clock, the time zone, the network or other
/// process state. An empty row was read and found to have none: hex,
/// serde, sha2 and thiserror touch no effect, and serde_json's
/// reader- and writer-taking functions reach one only through a
/// `std::io` handle the std allowlist refuses. A version bump fails
/// [`the_dependency_surface_table_matches_the_lockfile`] until its row is
/// read again. Each item is refused by its last segment wherever the scan
/// meets it, and by its path in both `clippy.toml`s.
const EFFECTS: [(&str, &str, &[&str]); 7] = [
    ("hex", "0.4.3", &[]),
    ("serde", "1.0.229", &[]),
    ("serde_json", "1.0.151", &[]),
    ("sha2", "0.10.9", &[]),
    ("thiserror", "2.0.20", &[]),
    (
        "time",
        "0.3.55",
        &[
            "time::Duration::time_fn",
            "time::Instant::elapsed",
            "time::Instant::now",
            "time::OffsetDateTime::now_local",
            "time::OffsetDateTime::now_utc",
            "time::SignedDuration::time_fn",
            "time::Timestamp::now",
            "time::UtcDateTime::now",
            "time::UtcOffset::current_local_offset",
            "time::UtcOffset::local_offset_at",
            "time::util::local_offset::get_soundness",
            "time::util::local_offset::set_soundness",
            "time::util::refresh_tz",
            "time::util::refresh_tz_unchecked",
        ],
    ),
    ("url", "2.5.8", &["url::Url::socket_addrs"]),
];

/// The table row whose effectful item is named `name`, as `crate version`.
/// `now` is held by the `::now` rule instead: as a path segment it is
/// refused, and a bare `now` is the clock value a pure crate is handed.
fn effect_of(name: &str) -> Option<String> {
    if name == "now" {
        return None;
    }
    EFFECTS.iter().find_map(|(krate, version, items)| {
        items
            .iter()
            .any(|item| item.rsplit("::").next() == Some(name))
            .then(|| format!("{krate} {version}"))
    })
}

/// Every table row's version against the lockfile, as `crate: why`: a row
/// whose crate a pure crate does not resolve to, or resolves at another
/// version, and a crate in a pure crate's closed set with no row. The
/// workspace crates in the sets are held by the scan itself.
fn surface_refusals(packages: &[Package], members: &[String]) -> Vec<String> {
    let resolved = |name: &str| -> Vec<&str> {
        packages
            .iter()
            .filter(|package| package.name == name && !members.contains(&package.id))
            .map(|package| package.version.as_str())
            .collect()
    };
    let mut refused: Vec<String> = EFFECTS
        .iter()
        .filter(|(krate, version, _)| resolved(krate) != [*version])
        .map(|(krate, version, _)| {
            format!(
                "{krate}: the table reads {version}, the lockfile pins {:?}",
                resolved(krate)
            )
        })
        .collect();
    let members_named: BTreeSet<&str> = packages
        .iter()
        .filter(|package| members.contains(&package.id))
        .map(|package| package.name.as_str())
        .collect();
    for (_, closed) in &PURE {
        for dependency in *closed {
            let row = EFFECTS.iter().any(|(krate, _, _)| krate == dependency);
            if !row && !members_named.contains(dependency) {
                refused.push(format!("{dependency}: no row in the effect table"));
            }
        }
    }
    refused.sort();
    refused.dedup();
    refused
}

/// A dependency bump, or a new crate in a closed set, fails here until its
/// effectful surface is read again at the new version.
#[test]
fn the_dependency_surface_table_matches_the_lockfile() {
    let metadata = metadata();
    assert_eq!(
        surface_refusals(&metadata.packages, &metadata.workspace_members),
        Vec::<String>::new(),
        "each pure crate's third-party surface was read at the version it compiles"
    );
    let drifted: Vec<Package> = metadata
        .packages
        .iter()
        .map(|package| Package {
            id: package.id.clone(),
            name: package.name.clone(),
            version: if package.name == "url" && package.source.is_some() {
                "2.5.9".to_string()
            } else {
                package.version.clone()
            },
            source: package.source.clone(),
            dependencies: Vec::new(),
            targets: Vec::new(),
        })
        .collect();
    assert_eq!(
        surface_refusals(&drifted, &metadata.workspace_members),
        ["url: the table reads 2.5.8, the lockfile pins [\"2.5.9\"]"]
    );
    let config = read("crates/brokkr-core/clippy.toml");
    let unlisted: Vec<&str> = EFFECTS
        .iter()
        .flat_map(|(_, _, items)| items.iter().copied())
        .filter(|item| !config.contains(&format!("{{ path = \"{item}\",")))
        .collect();
    assert_eq!(
        unlisted,
        Vec::<&str>::new(),
        "every effectful item is a clippy.toml disallowed method as well"
    );
}

/// The lints a pure crate's attributes (`allow`, `expect` or `warn`) and
/// the workspace manifest's lint tables may not lower: the purity lints
/// under their current and renamed names, the clippy groups that hold them,
/// `warnings`, and `renamed_and_removed_lints`, which would silence a
/// renamed one. [`admitted_exemption`] names the one exception in source.
const PURITY_LINTS: [&str; 9] = [
    "warnings",
    "clippy::all",
    "clippy::style",
    "clippy::disallowed_macros",
    "clippy::disallowed_methods",
    "clippy::disallowed_types",
    "clippy::disallowed_method",
    "clippy::disallowed_type",
    "renamed_and_removed_lints",
];

/// A manifest line as `line: text`, for a refusal.
fn manifest_line(index: usize, line: &str) -> String {
    format!("{}: {}", index + 1, line.trim())
}

/// A manifest's non-blank, non-comment lines with their index and the
/// table header they sit under, and every line that sets rustflags or
/// opts into an unstable Cargo feature, which could pass `--cfg test`.
fn manifest_lines(manifest: &str) -> (Vec<(usize, String, &str)>, Vec<String>) {
    let (mut section, mut lines, mut flags) = (String::new(), Vec::new(), Vec::new());
    for (index, line) in manifest.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.contains("rustflags") || trimmed.contains("cargo-features") {
            flags.push(manifest_line(index, line));
        }
        if trimmed.starts_with('[') {
            section = trimmed.to_string();
        }
        lines.push((index, section.clone(), trimmed));
    }
    (lines, flags)
}

/// Every line of a pure crate's manifest that sets its lints any way but
/// by inheriting the workspace's, or sets rustflags: its one `[lints]`
/// table is exactly `workspace = true`, and no other line names `lints`.
fn crate_manifest_refusals(manifest: &str) -> Vec<String> {
    let (lines, mut refused) = manifest_lines(manifest);
    let mut inherits = false;
    for (index, section, line) in lines {
        let key = line
            .split('=')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('"');
        match (section.as_str(), line) {
            ("[lints]", "[lints]") => {}
            ("[lints]", "workspace = true") => inherits = true,
            (_, _) if section.contains("lints") || key.starts_with("lints") => {
                refused.push(manifest_line(index, line));
            }
            _ => {}
        }
    }
    if !inherits {
        refused.push("no `[lints]` with `workspace = true`".to_string());
    }
    refused
}

/// A lint table key as rustc reads the lint it names: Cargo passes
/// `disallowed-methods`, `disallowed_methods.level` and a quoted key alike
/// as `disallowed_methods`, so quotes go, the key is cut at its first dot,
/// hyphens become underscores, and a `[workspace.lints.clippy]` key is
/// `clippy::`-scoped.
fn manifest_lint(section: &str, key: &str) -> String {
    let key = key.replace(['"', '\''], "");
    let name = key.strip_prefix("clippy::").unwrap_or(&key);
    let name = name.split('.').next().unwrap_or(name).replace('-', "_");
    if section == "[workspace.lints.clippy]" {
        format!("clippy::{name}")
    } else {
        name
    }
}

/// Every line of the workspace manifest that lowers a purity lint or sets
/// rustflags: lints live only in `[workspace.lints.rust]` and
/// `[workspace.lints.clippy]`, neither may name a lint of
/// [`PURITY_LINTS`], and no other line names `lints`.
fn workspace_manifest_refusals(manifest: &str) -> Vec<String> {
    let (lines, mut refused) = manifest_lines(manifest);
    for (index, section, line) in lines {
        let key = line
            .split('=')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('"');
        let governed = ["[workspace.lints.rust]", "[workspace.lints.clippy]"];
        let refuse = if line.starts_with('[') {
            line.contains("lints") && !governed.contains(&line)
        } else if governed.contains(&section.as_str()) {
            PURITY_LINTS.contains(&manifest_lint(&section, key).as_str())
        } else {
            section.contains("lints") || key.contains("lints")
        };
        if refuse {
            refused.push(manifest_line(index, line));
        }
    }
    refused
}

/// Every Cargo configuration file in the repository, tracked or not, as a
/// workspace-relative path. Such a file can set `build.rustflags`, which
/// could compile every `#[cfg(test)]` item into a pure crate.
fn cargo_config_files(root: &Path) -> Vec<String> {
    let mut git = Command::new("git");
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            git.env_remove(name);
        }
    }
    let output = git
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(root)
        .output()
        .expect("git runs");
    assert_eq!(output.status.code(), Some(0), "git lists the tree");
    let listed = String::from_utf8(output.stdout).expect("utf-8 paths");
    let mut found: BTreeSet<String> = listed
        .split('\0')
        .filter(|path| path.ends_with(".cargo/config") || path.ends_with(".cargo/config.toml"))
        .map(str::to_string)
        .collect();
    for dir in ["", "crates/brokkr-core/", "crates/brokkr-view/"] {
        for name in [".cargo/config", ".cargo/config.toml"] {
            let path = format!("{dir}{name}");
            if root.join(&path).exists() {
                found.insert(path);
            }
        }
    }
    found.into_iter().collect()
}

/// The readers refuse each form that lowers a purity lint or sets flags.
#[test]
fn the_manifest_readers_refuse_every_lowered_lint_and_flag() {
    let inherited = "[package]\nname = \"x\"\n\n[lints]\nworkspace = true\n";
    assert_eq!(crate_manifest_refusals(inherited), Vec::<String>::new());
    for (manifest, expected) in [
        (
            "[lints.clippy]\ndisallowed_methods = \"allow\"\n",
            &[
                "1: [lints.clippy]",
                "2: disallowed_methods = \"allow\"",
                "no `[lints]` with `workspace = true`",
            ][..],
        ),
        (
            "[lints]\nworkspace = true\nclippy = { disallowed_types = \"allow\" }\n",
            &["3: clippy = { disallowed_types = \"allow\" }"][..],
        ),
        (
            "lints.workspace = true\n[lints.rust]\nwarnings = \"allow\"\n",
            &[
                "1: lints.workspace = true",
                "2: [lints.rust]",
                "3: warnings = \"allow\"",
                "no `[lints]` with `workspace = true`",
            ][..],
        ),
        (
            "[profile.dev]\nrustflags = [\"--cfg\", \"test\"]\n[lints]\nworkspace = true\n",
            &["2: rustflags = [\"--cfg\", \"test\"]"][..],
        ),
    ] {
        assert_eq!(crate_manifest_refusals(manifest), expected, "{manifest}");
    }
    let workspace = "[workspace.lints.rust]\nunexpected_cfgs = { level = \"warn\" }\n";
    assert_eq!(workspace_manifest_refusals(workspace), Vec::<String>::new());
    for (manifest, expected) in [
        (
            "[workspace.lints.clippy]\ndisallowed_methods = \"allow\"\ndisallowed_method = \"allow\"\n\"clippy::disallowed_types\" = \"allow\"\nstyle = \"allow\"\ntoo_many_lines = \"warn\"\n",
            &["2: disallowed_methods = \"allow\"", "3: disallowed_method = \"allow\"", "4: \"clippy::disallowed_types\" = \"allow\"", "5: style = \"allow\""][..],
        ),
        (
            "[workspace.lints.clippy]\ndisallowed-methods = \"allow\"\ndisallowed_methods.level = \"allow\"\n\"disallowed-types\".priority = 1\n",
            &[
                "2: disallowed-methods = \"allow\"",
                "3: disallowed_methods.level = \"allow\"",
                "4: \"disallowed-types\".priority = 1",
            ][..],
        ),
        (
            "[workspace.lints.rust]\nrenamed_and_removed_lints = \"allow\"\n",
            &["2: renamed_and_removed_lints = \"allow\""][..],
        ),
        (
            "[workspace.lints]\nclippy.disallowed_types = \"allow\"\n[workspace]\nlints.clippy.all = \"allow\"\n",
            &["1: [workspace.lints]", "2: clippy.disallowed_types = \"allow\"", "4: lints.clippy.all = \"allow\""][..],
        ),
        (
            "cargo-features = [\"profile-rustflags\"]\n",
            &["1: cargo-features = [\"profile-rustflags\"]"][..],
        ),
    ] {
        assert_eq!(workspace_manifest_refusals(manifest), expected, "{manifest}");
    }
}

/// Ruling 1's clippy half cannot be lowered from a manifest, and nothing in
/// the repository passes rustc flags: each pure crate inherits the
/// workspace lints, the workspace lowers no purity lint, and no Cargo
/// configuration file exists.
#[test]
fn no_manifest_or_config_lowers_the_purity_gates() {
    let mut refused: Vec<String> = PURE
        .iter()
        .flat_map(|(name, _)| {
            crate_manifest_refusals(&read(&format!("crates/{name}/Cargo.toml")))
                .into_iter()
                .map(move |why| format!("crates/{name}/Cargo.toml:{why}"))
        })
        .collect();
    refused.extend(
        workspace_manifest_refusals(&read("Cargo.toml"))
            .into_iter()
            .map(|why| format!("Cargo.toml:{why}")),
    );
    refused.extend(
        cargo_config_files(&workspace())
            .into_iter()
            .map(|path| format!("{path}: a Cargo configuration file")),
    );
    assert_eq!(
        refused,
        Vec::<String>::new(),
        "the purity lints stand as clippy.toml sets them (decision 0071 ruling 1)"
    );
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
