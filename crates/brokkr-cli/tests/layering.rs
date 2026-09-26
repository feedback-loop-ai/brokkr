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

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use serde::Deserialize;

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
                "--offline",
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
/// binary, which nothing may depend on. A crate added here is one no other
/// crate may name, so cargo-deny has nothing to govern.
const UNGOVERNED: [&str; 1] = ["brokkr-cli"];

/// Each governed crate and the crates allowed to depend on it directly.
type Graph = BTreeMap<String, BTreeSet<String>>;

/// A `deny.toml` line the strict reader below does not understand.
#[derive(Debug, PartialEq)]
struct NotUnderstood {
    line: usize,
    text: String,
}

/// `deny.toml`'s `[bans]` entries, read strictly: inside the table every
/// line is `deny = [`, `]`, a comment, or one whole entry in the house
/// form, and outside it no line names `bans` or could spell it through an
/// escape or a multi-line string. Anything else is refused, never skipped,
/// so the graph read here is the graph cargo-deny enforces.
fn allowed_parents(deny: &str) -> Result<Graph, NotUnderstood> {
    let mut parents = Graph::new();
    let mut in_bans = false;
    let mut seen_bans = false;
    for (index, line) in deny.lines().enumerate() {
        let refuse = || NotUnderstood {
            line: index + 1,
            text: line.to_string(),
        };
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') {
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
        if trimmed == "deny = [" || trimmed == "]" {
            continue;
        }
        let (name, wrappers) = ban_entry(line).ok_or_else(refuse)?;
        if parents.insert(name, wrappers).is_some() {
            return Err(refuse());
        }
    }
    Ok(parents)
}

/// One `[bans]` entry in the house form, or `None`:
/// `  { crate = "name", wrappers = ["a", "b"], reason = "…" },`
fn ban_entry(line: &str) -> Option<(String, BTreeSet<String>)> {
    let plain = |name: &str| {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    };
    let rest = line.strip_prefix("  { crate = \"")?;
    let (name, rest) = rest.split_once("\", wrappers = [")?;
    let (list, rest) = rest.split_once("], reason = \"")?;
    let reason = rest.strip_suffix("\" },")?;
    if !plain(name) || reason.contains(['"', '\\']) {
        return None;
    }
    let wrappers = list
        .split(", ")
        .map(|wrapper| {
            let wrapper = wrapper.strip_prefix('"')?.strip_suffix('"')?;
            plain(wrapper).then(|| wrapper.to_string())
        })
        .collect::<Option<BTreeSet<String>>>()?;
    Some((name.to_string(), wrappers))
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

/// The crates a path may be rooted at to reach the standard library.
const ROOTS: [&str; 3] = ["std", "core", "alloc"];

/// Ruling 1's allowlist: the only paths under `std`, `core` or `alloc`
/// the pure crates may name, each a whole module or a single item, and
/// each one they use today. Every other path under a root is refused,
/// the root itself included.
const PURE_STD: [&str; 8] = [
    "cell",
    "collections::BTreeMap",
    "collections::BTreeSet",
    "fmt",
    "iter",
    "mem",
    "rc",
    "str",
];

/// The macros the pure crates may invoke besides those they define with
/// `macro_rules!`. `thread_local` serves view's test-only counters, and
/// clippy refuses the `LocalKey` it makes anywhere else.
const PURE_MACROS: [&str; 5] = ["format", "json", "matches", "thread_local", "vec"];

/// Keywords that may stand before `!(`, where the `!` is a negation.
const NEGATING_KEYWORDS: [&str; 6] = ["break", "if", "in", "match", "return", "while"];

/// Every macro std and core define, reached through the prelude or a path,
/// stable and unstable. A pure crate may not define a macro by one of these
/// names, so a call by one always means the builtin: only those in
/// [`PURE_MACROS`] pass.
const BUILTIN_MACROS: [&str; 56] = [
    "addr_of",
    "addr_of_mut",
    "asm",
    "assert",
    "assert_eq",
    "assert_matches",
    "assert_ne",
    "cfg",
    "cfg_match",
    "cfg_select",
    "column",
    "compile_error",
    "concat",
    "concat_bytes",
    "concat_idents",
    "const_format_args",
    "dbg",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_matches",
    "debug_assert_ne",
    "env",
    "eprint",
    "eprintln",
    "file",
    "format",
    "format_args",
    "format_args_nl",
    "global_asm",
    "include",
    "include_bytes",
    "include_str",
    "is_aarch64_feature_detected",
    "is_x86_feature_detected",
    "line",
    "log_syntax",
    "matches",
    "module_path",
    "naked_asm",
    "offset_of",
    "option_env",
    "panic",
    "pin",
    "print",
    "println",
    "ready",
    "stringify",
    "thread_local",
    "todo",
    "trace_macros",
    "try",
    "unimplemented",
    "unreachable",
    "vec",
    "write",
    "writeln",
];

/// The lint names an `allow`, `expect` or `warn` in a pure crate may not
/// lower: the crate-local purity lints, the clippy groups that hold them,
/// and `warnings`. [`admitted_exemption`] names the one exception.
const PURITY_LINTS: [&str; 6] = [
    "warnings",
    "clippy::all",
    "clippy::style",
    "clippy::disallowed_macros",
    "clippy::disallowed_methods",
    "clippy::disallowed_types",
];

/// Clock and randomness sources from the crates the pure crates may use,
/// refused wherever they are named.
const SOURCES: [&str; 12] = [
    "SystemTime",
    "Instant",
    "now_utc",
    "now_local",
    "current_local_offset",
    "local_offset_at",
    "new_v4",
    "new_v7",
    "now_v1",
    "now_v6",
    "now_v7",
    "RandomState",
];

/// What every string, byte string, raw string and char literal lexes to.
const LITERAL: &str = "<literal>";

/// A token and the line it starts on.
type Token = (usize, String);

/// The token at `at`, or `""` past the end.
fn text(tokens: &[Token], at: usize) -> &str {
    tokens.get(at).map_or("", |(_, token)| token.as_str())
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_ident(token: &str) -> bool {
    token.starts_with(|c: char| c.is_alphabetic() || c == '_') && token.chars().all(is_word)
}

fn word_end(chars: &[char], at: usize) -> usize {
    at + chars[at..].iter().take_while(|c| is_word(**c)).count()
}

/// Rust source as tokens, lexed before anything is matched: line, block
/// (nested) and doc comments are dropped, and each literal reads as
/// [`LITERAL`], so neither can hide code or pose as it. Words (a raw
/// identifier's `r#` dropped), `::` and lifetimes are one token each,
/// every other non-space character is its own.
fn lex(source: &str) -> Vec<Token> {
    lex_with_literals(source).0
}

/// [`lex`], and the source text of every [`LITERAL`] token, keyed by the
/// token's index, quotes and prefixes included.
fn lex_with_literals(source: &str) -> (Vec<Token>, BTreeMap<usize, String>) {
    let chars: Vec<char> = source.chars().collect();
    let (mut tokens, mut literals) = (Vec::new(), BTreeMap::new());
    let (mut at, mut line) = (0, 1);
    while at < chars.len() {
        let (end, token) = next_token(&chars, at);
        if token.as_deref() == Some(LITERAL) {
            literals.insert(tokens.len(), chars[at..end].iter().collect());
        }
        tokens.extend(token.map(|token| (line, token)));
        line += chars[at..end].iter().filter(|c| **c == '\n').count();
        at = end;
    }
    (tokens, literals)
}

/// The token at `at`, if it is not a comment or space, and where it ends.
fn next_token(chars: &[char], at: usize) -> (usize, Option<String>) {
    match (chars[at], chars.get(at + 1)) {
        ('/', Some('/')) => {
            let length = chars[at..].iter().take_while(|c| **c != '\n').count();
            (at + length, None)
        }
        ('/', Some('*')) => (block_comment_end(chars, at), None),
        ('"', _) => (quoted_end(chars, at + 1, '"'), Some(LITERAL.into())),
        ('\'', _) => char_or_lifetime(chars, at),
        (':', Some(':')) => (at + 2, Some("::".into())),
        (c, _) if is_word(c) => word_or_literal(chars, at),
        (c, _) if c.is_whitespace() => (at + 1, None),
        (c, _) => (at + 1, Some(c.to_string())),
    }
}

/// The end of the block comment opening at `at`, which nests.
fn block_comment_end(chars: &[char], at: usize) -> usize {
    let (mut depth, mut index) = (0, at);
    while index < chars.len() {
        match (chars[index], chars.get(index + 1)) {
            ('/', Some('*')) => depth += 1,
            ('*', Some('/')) => depth -= 1,
            _ => {
                index += 1;
                continue;
            }
        }
        index += 2;
        if depth == 0 {
            return index;
        }
    }
    chars.len()
}

/// The end of a literal whose body starts at `from` and closes at an
/// unescaped `quote`.
fn quoted_end(chars: &[char], from: usize, quote: char) -> usize {
    let mut index = from;
    while index < chars.len() {
        match chars[index] {
            '\\' => index += 2,
            c if c == quote => return index + 1,
            _ => index += 1,
        }
    }
    chars.len()
}

/// A char literal (`'x'`, `'\n'`) or a lifetime or label (`'a`).
fn char_or_lifetime(chars: &[char], at: usize) -> (usize, Option<String>) {
    match (chars.get(at + 1), chars.get(at + 2)) {
        (Some('\\'), _) => (quoted_end(chars, at + 1, '\''), Some(LITERAL.into())),
        (Some(_), Some('\'')) => (at + 3, Some(LITERAL.into())),
        _ => {
            let end = word_end(chars, at + 1);
            (end, Some(chars[at..end].iter().collect()))
        }
    }
}

/// A word, or the literal or raw identifier a word prefix opens: `r"…"`,
/// `br#"…"#`, `cr"…"`, `b"…"`, `c"…"`, `b'…'` and `r#ident`.
fn word_or_literal(chars: &[char], at: usize) -> (usize, Option<String>) {
    let end = word_end(chars, at);
    let word: String = chars[at..end].iter().collect();
    let hashes = chars[end..].iter().take_while(|c| **c == '#').count();
    match (word.as_str(), hashes, chars.get(end + hashes)) {
        ("r" | "br" | "cr", _, Some('"')) => {
            let body = end + hashes + 1;
            let close = (body..chars.len()).find(|&index| {
                chars[index] == '"'
                    && chars[index + 1..]
                        .iter()
                        .take(hashes)
                        .filter(|c| **c == '#')
                        .count()
                        == hashes
            });
            (
                close.map_or(chars.len(), |index| index + 1 + hashes),
                Some(LITERAL.into()),
            )
        }
        ("r", 1, Some(c)) if is_word(*c) => {
            let ident_end = word_end(chars, end + 1);
            (ident_end, Some(chars[end + 1..ident_end].iter().collect()))
        }
        ("b" | "c", 0, Some('"')) => (quoted_end(chars, end + 1, '"'), Some(LITERAL.into())),
        ("b", 0, Some('\'')) => (quoted_end(chars, end + 1, '\''), Some(LITERAL.into())),
        _ => (end, Some(word)),
    }
}

/// A `use` tree in a form the reader does not know.
struct Unreadable;

/// One path a `use` tree imports, the line of its last segment, and
/// whether it is renamed.
struct Import {
    line: usize,
    path: Vec<String>,
    renamed: bool,
}

/// Resolves the `use` tree at `at` onto `path`, nested groups, `self`,
/// globs and renames included, and leaves `at` past it. `Err` for a form
/// this reader does not know.
fn use_tree(
    tokens: &[Token],
    at: &mut usize,
    mut path: Vec<String>,
    imports: &mut Vec<Import>,
) -> Result<(), Unreadable> {
    if text(tokens, *at) == "::" {
        *at += 1;
    }
    loop {
        let (line, token) = tokens.get(*at).ok_or(Unreadable)?.clone();
        *at += 1;
        if token == "{" {
            return use_group(tokens, at, &path, imports);
        }
        if token != "*" && !is_ident(&token) {
            return Err(Unreadable);
        }
        let glob = token == "*";
        path.push(token);
        match (glob, text(tokens, *at)) {
            (false, "::") => *at += 1,
            (false, "as") => {
                *at += 2;
                imports.push(Import {
                    line,
                    path,
                    renamed: true,
                });
                return Ok(());
            }
            _ => {
                imports.push(Import {
                    line,
                    path,
                    renamed: false,
                });
                return Ok(());
            }
        }
    }
}

/// The trees of a `{ … }` group, each resolved onto `path`.
fn use_group(
    tokens: &[Token],
    at: &mut usize,
    path: &[String],
    imports: &mut Vec<Import>,
) -> Result<(), Unreadable> {
    loop {
        if text(tokens, *at) == "}" {
            *at += 1;
            return Ok(());
        }
        use_tree(tokens, at, path.to_vec(), imports)?;
        match text(tokens, *at) {
            "," => *at += 1,
            "}" => {}
            _ => return Err(Unreadable),
        }
    }
}

/// Whether a path rooted at `std`, `core` or `alloc` falls under the
/// allowlist. The root alone never does.
fn pure_std(path: &[String]) -> bool {
    PURE_STD.iter().any(|entry| {
        let entry: Vec<&str> = entry.split("::").collect();
        path.len() > entry.len()
            && path[1..=entry.len()]
                .iter()
                .zip(&entry)
                .all(|(a, b)| a == b)
    })
}

/// The refusal for one import, if it reaches a root: from the first root
/// segment on (so `crate::std::fs` is `std::fs`), a rename of the root
/// itself, or a path outside the allowlist.
fn import_refusal(import: &Import) -> Option<(usize, String)> {
    let root = import
        .path
        .iter()
        .position(|segment| ROOTS.contains(&segment.as_str()))?;
    let mut path = import.path[root..].to_vec();
    if path.len() > 1 && path.last().is_some_and(|segment| segment == "self") {
        path.pop();
    }
    if path.len() == 1 && import.renamed {
        return Some((import.line, format!("a rename of {}", path[0])));
    }
    (!pure_std(&path)).then(|| {
        (
            import.line,
            format!("{}: outside the pure std allowlist", path.join("::")),
        )
    })
}

/// The path rooted at `tokens[at]` outside an import, and where it ends.
/// A segment that is not an identifier (a macro's `$`) ends the path and
/// is kept, so the path falls outside the allowlist; a turbofish ends it.
fn expression_path(tokens: &[Token], at: usize) -> (Vec<String>, usize) {
    let mut path = vec![tokens[at].1.clone()];
    let mut end = at + 1;
    while text(tokens, end) == "::" && !["", "<"].contains(&text(tokens, end + 1)) {
        path.push(text(tokens, end + 1).to_string());
        end += 2;
        if !is_ident(text(tokens, end - 1)) {
            break;
        }
    }
    (path, end)
}

/// Every import and every other path that reaches a root and falls
/// outside the allowlist, and every rename of a root.
fn std_refusals(tokens: &[Token]) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut at = 0;
    while at < tokens.len() {
        let line = tokens[at].0;
        if text(tokens, at) == "use" && text(tokens, at + 1) != "<" {
            let (mut end, mut imports) = (at + 1, Vec::new());
            if use_tree(tokens, &mut end, Vec::new(), &mut imports).is_ok()
                && text(tokens, end) == ";"
            {
                found.extend(imports.iter().filter_map(import_refusal));
                at = end + 1;
                continue;
            }
            found.push((line, "an import the scanner cannot read".to_string()));
        } else if ROOTS.contains(&text(tokens, at)) {
            let (path, end) = expression_path(tokens, at);
            if !pure_std(&path) {
                let path = path.join("::");
                found.push((line, format!("{path}: outside the pure std allowlist")));
            }
            at = end;
            continue;
        }
        at += 1;
    }
    found
}

/// Whether the `mod` at `at` is declared directly under `#[cfg(test)]`.
fn under_cfg_test(tokens: &[Token], at: usize) -> bool {
    let attribute = ["#", "[", "cfg", "(", "test", ")", "]"];
    at >= attribute.len()
        && tokens[at - attribute.len()..at]
            .iter()
            .map(|(_, token)| token.as_str())
            .eq(attribute)
}

/// Whether the attribute opening at `at` sets a `path`, which could make
/// a module read a file this scan never lists.
fn sets_a_path(tokens: &[Token], at: usize) -> bool {
    let open = at + 1 + usize::from(tokens.get(at + 1).is_some_and(|(_, t)| t == "!"));
    if tokens.get(open).is_none_or(|(_, token)| token != "[") {
        return false;
    }
    let mut depth = 0;
    for (index, (_, token)) in tokens.iter().enumerate().skip(open) {
        match token.as_str() {
            "[" => depth += 1,
            "]" if depth == 1 => return false,
            "]" => depth -= 1,
            "path" if tokens.get(index + 1).is_some_and(|(_, t)| t == "=") => return true,
            _ => {}
        }
    }
    false
}

/// Where each token sits among the brace blocks: `at[i]` is the `{` of
/// the innermost block open at token `i`, and `parent[&b]` the `{` of the
/// block around the one opening at `b`. `None` is the file's top level.
struct Blocks {
    at: Vec<Option<usize>>,
    parent: BTreeMap<usize, Option<usize>>,
}

impl Blocks {
    fn of(tokens: &[Token]) -> Self {
        let mut open: Vec<usize> = Vec::new();
        let mut blocks = Blocks {
            at: Vec::with_capacity(tokens.len()),
            parent: BTreeMap::new(),
        };
        for (index, (_, token)) in tokens.iter().enumerate() {
            blocks.at.push(open.last().copied());
            match token.as_str() {
                "{" => {
                    blocks.parent.insert(index, open.last().copied());
                    open.push(index);
                }
                "}" => {
                    open.pop();
                }
                _ => {}
            }
        }
        blocks
    }

    /// Whether token `inner` lies in the block `outer`, directly or in a
    /// block nested in it.
    fn within(&self, outer: Option<usize>, inner: usize) -> bool {
        let mut block = self.at[inner];
        loop {
            if block == outer {
                return true;
            }
            match block {
                Some(open) => block = self.parent[&open],
                None => return false,
            }
        }
    }
}

/// Whether a `macro_rules!` of `name` is in scope at token `at`: defined
/// earlier, in the block that holds `at` or one around it. A builtin name
/// is never in scope, because defining one is refused.
fn defined_before(tokens: &[Token], blocks: &Blocks, name: &str, at: usize) -> bool {
    !BUILTIN_MACROS.contains(&name)
        && (2..at).any(|index| {
            text(tokens, index - 2) == "macro_rules"
                && text(tokens, index - 1) == "!"
                && text(tokens, index) == name
                && blocks.within(blocks.at[index - 2], at)
        })
}

/// The index of the `]` closing the attribute, inner or outer, whose `#`
/// is at `at`. `None` when no `[` follows.
fn attribute_end(tokens: &[Token], at: usize) -> Option<usize> {
    let open = at + 1 + usize::from(text(tokens, at + 1) == "!");
    if text(tokens, open) != "[" {
        return None;
    }
    let mut depth = 0;
    for (index, (_, token)) in tokens.iter().enumerate().skip(open) {
        match token.as_str() {
            "[" => depth += 1,
            "]" if depth == 1 => return Some(index),
            "]" => depth -= 1,
            _ => {}
        }
    }
    None
}

/// Whether the attribute from `at` to `end` lowers a lint in
/// [`PURITY_LINTS`]: it holds `allow`, `expect` or `warn`, and names one.
fn lowers_purity(tokens: &[Token], at: usize, end: usize) -> bool {
    let words: Vec<&str> = (at..=end).map(|index| text(tokens, index)).collect();
    let lowers = words
        .iter()
        .any(|word| ["allow", "expect", "warn"].contains(word));
    let names = words.iter().enumerate().any(|(index, word)| {
        let path = match words.get(index + 1..index + 3) {
            Some(["::", lint]) if *word == "clippy" => format!("clippy::{lint}"),
            _ => (*word).to_string(),
        };
        PURITY_LINTS.contains(&path.as_str())
    });
    lowers && names
}

/// One thread-local counter as the admitted shape spells it.
const COUNTER: [&str; 22] = [
    "pub", "(", "super", ")", "static", "<name>", ":", "Cell", "<", "usize", ">", "=", "const",
    "{", "Cell", "::", "new", "(", "0", ")", "}", ";",
];

/// The one exemption from the purity lints a pure crate may carry: an
/// outer `#[expect(clippy::disallowed_types, reason = "…")]` on a module
/// that holds only `use std::cell::Cell;` and a `thread_local!` of
/// `Cell<usize>` counters, itself inside a `#[cfg(test)]` module. The
/// view's test-only projector counters take it; nothing else can.
fn admitted_exemption(tokens: &[Token], blocks: &Blocks, at: usize, end: usize) -> bool {
    let attribute = [
        "#",
        "[",
        "expect",
        "(",
        "clippy",
        "::",
        "disallowed_types",
        ",",
        "reason",
        "=",
        LITERAL,
        ")",
        "]",
    ];
    let head = [
        "mod",
        "<name>",
        "{",
        "use",
        "std",
        "::",
        "cell",
        "::",
        "Cell",
        ";",
        "thread_local",
        "!",
        "{",
    ];
    let fits = |from: usize, shape: &[&str]| {
        shape.iter().enumerate().all(|(offset, want)| {
            let got = text(tokens, from + offset);
            if *want == "<name>" {
                is_ident(got)
            } else {
                got == *want
            }
        })
    };
    if end + 1 - at != attribute.len() || !fits(at, &attribute) || !fits(end + 1, &head) {
        return false;
    }
    let enclosing = blocks.at[at].is_some_and(|open| {
        open >= 2
            && text(tokens, open - 2) == "mod"
            && is_ident(text(tokens, open - 1))
            && under_cfg_test(tokens, open - 2)
    });
    let open = end + head.len();
    let Some(close) = (open + 1..tokens.len())
        .find(|&index| text(tokens, index) == "}" && blocks.at[index] == Some(open))
    else {
        return false;
    };
    let length = close - open - 1;
    enclosing
        && length.is_multiple_of(COUNTER.len())
        && (0..length / COUNTER.len()).all(|n| fits(open + 1 + n * COUNTER.len(), &COUNTER))
        && text(tokens, close + 1) == "}"
        && blocks.at[close + 1] == Some(end + 3)
}

/// One lexed source file of a pure crate, and where it sits in the
/// workspace, which a `#[path]` resolves against and names the crate.
struct Scanned<'a> {
    tokens: &'a [Token],
    literals: &'a BTreeMap<usize, String>,
    blocks: &'a Blocks,
    file: &'a Path,
}

/// Whether a workspace-relative `path` is one the exact coverage gate
/// treats as a test: under a `tests/` directory, or named `tests.rs` or
/// `*_tests.rs` (`scripts/coverage-exact.sh`'s pattern).
fn is_gate_test_path(path: &Path) -> bool {
    let parts: Vec<&str> = path.iter().filter_map(|part| part.to_str()).collect();
    let Some((last, dirs)) = parts.split_last() else {
        return false;
    };
    dirs.contains(&"tests")
        || *last == "tests.rs"
        || last
            .strip_suffix("_tests.rs")
            .is_some_and(|stem| !stem.is_empty())
}

/// `relative` joined onto `dir` with `.` and `..` folded, or `None` when it
/// climbs out of the workspace.
fn resolve(dir: &Path, relative: &str) -> Option<PathBuf> {
    let mut resolved = Vec::new();
    for part in dir.iter().chain(Path::new(relative).iter()) {
        match part.to_str()? {
            "." => {}
            ".." => {
                resolved.pop()?;
            }
            part => resolved.push(part),
        }
    }
    Some(resolved.iter().collect())
}

/// Whether the `#` at `at` opens the one `#[path]` form test code may
/// take: `#[cfg(test)] #[path = "…"] mod name;` at the file's top level,
/// the path a plain string that resolves, from the file's directory, to a
/// gate test path inside the workspace.
fn test_path_module(scan: &Scanned, at: usize) -> bool {
    let shape = ["#", "[", "path", "=", LITERAL, "]", "mod", "<name>", ";"];
    let fits = shape.iter().enumerate().all(|(offset, want)| {
        let got = text(scan.tokens, at + offset);
        if *want == "<name>" {
            is_ident(got)
        } else {
            got == *want
        }
    });
    let target = scan
        .literals
        .get(&(at + 4))
        .and_then(|literal| literal.strip_prefix('"')?.strip_suffix('"'))
        .filter(|path| !path.contains(['\\', '"']))
        .and_then(|path| resolve(scan.file.parent()?, path));
    fits && scan.blocks.at[at].is_none()
        && under_cfg_test(scan.tokens, at)
        && target.is_some_and(|target| is_gate_test_path(&target))
}

/// Whether the `extern` at `at` is the one `extern crate` test code may
/// take: `#[cfg(test)] extern crate self as <this crate>;`, which lets
/// the workspace's shared test support name the crate as other crates do.
fn test_self_alias(scan: &Scanned, at: usize) -> bool {
    let own = scan
        .file
        .strip_prefix("crates")
        .ok()
        .and_then(|rest| rest.iter().next()?.to_str())
        .map(|name| name.replace('-', "_"));
    under_cfg_test(scan.tokens, at)
        && text(scan.tokens, at + 2) == "self"
        && text(scan.tokens, at + 3) == "as"
        && own.is_some_and(|own| text(scan.tokens, at + 4) == own)
        && text(scan.tokens, at + 5) == ";"
}

/// Every item form the pure crates may not take: an `extern crate`, a
/// `#[path]`, a `mod tests;` outside `#[cfg(test)]`, a lowered purity lint,
/// a `macro_rules!` shadowing a builtin, a macro outside the allowlist or
/// out of its definition's scope, and a clock or randomness source. Code
/// under exactly `#[cfg(test)]` may take two of them: a `#[path]` to a
/// gate test path, and an alias of the crate itself.
fn form_refusals(scan: &Scanned) -> Vec<(usize, String)> {
    let (tokens, blocks) = (scan.tokens, scan.blocks);
    let mut found = Vec::new();
    for (at, (line, token)) in tokens.iter().enumerate() {
        let what = match (token.as_str(), text(tokens, at + 1), text(tokens, at + 2)) {
            ("extern", "crate", _) if !test_self_alias(scan, at) => "an extern crate".to_string(),
            ("mod", "tests", ";") if !under_cfg_test(tokens, at) => {
                "a `mod tests;` outside #[cfg(test)]".to_string()
            }
            ("#", _, _) if sets_a_path(tokens, at) && !test_path_module(scan, at) => {
                "a #[path] attribute".to_string()
            }
            ("#", _, _)
                if attribute_end(tokens, at).is_some_and(|end| {
                    lowers_purity(tokens, at, end) && !admitted_exemption(tokens, blocks, at, end)
                }) =>
            {
                "an attribute that lowers a purity lint".to_string()
            }
            ("macro_rules", "!", name) if BUILTIN_MACROS.contains(&name) => {
                format!("macro_rules! {name}: shadows a builtin macro")
            }
            (name, "!", "(" | "[" | "{")
                if is_ident(name)
                    && !NEGATING_KEYWORDS.contains(&name)
                    && !PURE_MACROS.contains(&name)
                    && !defined_before(tokens, blocks, name, at) =>
            {
                format!("{name}!: outside the pure macro allowlist")
            }
            ("::", "now", _) => "::now: a clock or randomness source".to_string(),
            (name, _, _) if SOURCES.contains(&name) => {
                format!("{name}: a clock or randomness source")
            }
            _ => continue,
        };
        found.push((*line, what));
    }
    found
}

/// Every place a pure crate's source leaves ruling 1, as `line: what`.
/// `file` is the source's path relative to the workspace.
fn impurities(source: &str, file: &Path) -> Vec<String> {
    let (tokens, literals) = lex_with_literals(source);
    let blocks = Blocks::of(&tokens);
    let scan = Scanned {
        tokens: &tokens,
        literals: &literals,
        blocks: &blocks,
        file,
    };
    let mut found = std_refusals(&tokens);
    found.extend(form_refusals(&scan));
    found.sort_by_key(|(line, _)| *line);
    found
        .into_iter()
        .map(|(line, what)| format!("{line}: {what}"))
        .collect()
}

/// The scanner's cases: a source, and exactly what it refuses there, by
/// line. Every form the allowlists do not name is refused; the last
/// rows are clean or unterminated sources that refuse nothing.
const SCANNER_CASES: [(&str, &[&str]); 38] = [
    (
        "use std::{collections::BTreeMap, /* c */ os::unix::process::parent_id};",
        &["1: std::os::unix::process::parent_id: outside the pure std allowlist"],
    ),
    (
        "use std::{\n    collections::BTreeMap,\n    // c\n    os::unix::process::parent_id,\n};",
        &["4: std::os::unix::process::parent_id: outside the pure std allowlist"],
    ),
    (
        "/* /* */ std::fs::read(p) */ std::process::id()",
        &["1: std::process::id: outside the pure std allowlist"],
    ),
    (
        "/// a \"quote\n//! std::fs\n/** std::env */\nuse std::io::Read;",
        &["4: std::io::Read: outside the pure std allowlist"],
    ),
    (
        "let s = \"/*\"; let t = \"std::fs\"; std::env::var(s)",
        &["1: std::env::var: outside the pure std allowlist"],
    ),
    (
        "let s = r#\"\\\"#; let t = br##\"\"#\"##; std::process::id()",
        &["1: std::process::id: outside the pure std allowlist"],
    ),
    (
        "let q = '\"'; let e = '\\''; let b = b'\"';\nfn f<'a>(x: &'a str) { std::fs::read(x) }",
        &["2: std::fs::read: outside the pure std allowlist"],
    ),
    (
        "use std as s;\nuse core::{self as c};\nuse std::collections as h;",
        &["1: a rename of std", "2: a rename of core", "3: std::collections: outside the pure std allowlist"],
    ),
    (
        "extern crate alloc as a;",
        &["1: alloc: outside the pure std allowlist", "1: an extern crate"],
    ),
    (
        "use std::*;\nuse std::collections::*;\nuse std::fmt::*;",
        &["1: std::*: outside the pure std allowlist", "2: std::collections::*: outside the pure std allowlist"],
    ),
    (
        "use ::std::{collections::{BTreeMap, hash_map::HashMap}, io::{self, Write}};",
        &["1: std::collections::hash_map::HashMap: outside the pure std allowlist", "1: std::io: outside the pure std allowlist", "1: std::io::Write: outside the pure std allowlist"],
    ),
    (
        "macro_rules! reach { ($m:ident) => { std::$m::id() }; }\nreach!(process);",
        &["1: std::$: outside the pure std allowlist"],
    ),
    (
        "macro_rules! at { ($r:ident) => { $r::fs::read(p) }; }\nat!(std);",
        &["2: std: outside the pure std allowlist"],
    ),
    (
        "use crate::std::fs;\nlet f = r#std::fs::read;",
        &["1: std::fs: outside the pure std allowlist", "2: std::fs::read: outside the pure std allowlist"],
    ),
    (
        "use std::{$m};",
        &["1: an import the scanner cannot read", "1: std::{: outside the pure std allowlist"],
    ),
    (
        "println!(\"x\");\ninclude!(\"f.rs\");\nif !(a) { format!(\"{}\", vec![1]) }",
        &["1: println!: outside the pure macro allowlist", "2: include!: outside the pure macro allowlist"],
    ),
    (
        "let now = Instant::now();\nlet t = time::OffsetDateTime::now_utc();\nlet u = time::UtcDateTime::now();\nlet c = clock.now();\nlet d = age(now);",
        &["1: Instant: a clock or randomness source", "1: ::now: a clock or randomness source", "2: now_utc: a clock or randomness source", "3: ::now: a clock or randomness source"],
    ),
    (
        "#[path = \"../x.rs\"]\nmod x;\n#[cfg_attr(test, path = \"y.rs\")]\nmod y;\n#![doc(hidden)]",
        &["1: a #[path] attribute", "3: a #[path] attribute"],
    ),
    (
        "mod tests;\n#[cfg(test)]\nmod tests;\n#[cfg(any(test))]\nmod tests;",
        &["1: a `mod tests;` outside #[cfg(test)]", "5: a `mod tests;` outside #[cfg(test)]"],
    ),
    (
        "#[cfg(test)]\nextern crate self as brokkr_core;\n#[cfg(test)]\n#[path = \"../../../tests/support/envelope.rs\"]\nmod envelope_builder;",
        &[],
    ),
    (
        "#[path = \"../../../tests/support/envelope.rs\"]\nmod envelope_builder;\nextern crate self as brokkr_core;",
        &["1: a #[path] attribute", "3: an extern crate"],
    ),
    (
        "#[cfg(test)]\n#[path = \"other.rs\"]\nmod a;\n#[cfg(test)]\n#[path = \"../../../../tests/b.rs\"]\nmod b;\n#[cfg(test)]\n#[path = r\"../../../tests/c.rs\"]\nmod c;",
        &["2: a #[path] attribute", "5: a #[path] attribute", "8: a #[path] attribute"],
    ),
    (
        "mod m {\n    #[cfg(test)]\n    #[path = \"../../../tests/support/envelope.rs\"]\n    mod d;\n}\n#[cfg(any(test))]\n#[path = \"../../../tests/support/envelope.rs\"]\nmod e;\n#[path = \"../../../tests/support/envelope.rs\"]\n#[cfg(test)]\nmod f;\n#[cfg(test)]\n#[cfg_attr(test, path = \"../../../tests/support/envelope.rs\")]\nmod g;",
        &["3: a #[path] attribute", "7: a #[path] attribute", "9: a #[path] attribute", "13: a #[path] attribute"],
    ),
    (
        "#[cfg(test)]\n#[path = \"fold_tests.rs\"]\nmod h;\n#[cfg(test)]\n#[path = \"inner/tests.rs\"]\nmod i;",
        &[],
    ),
    (
        "#[cfg(test)]\nextern crate self as brokkr_view;\n#[cfg(test)]\nextern crate self;\n#[cfg(test)]\nextern crate serde as brokkr_core;\n#[cfg(any(test))]\nextern crate self as brokkr_core;",
        &["2: an extern crate", "4: an extern crate", "6: an extern crate", "8: an extern crate"],
    ),
    (
        "mod m {\n    macro_rules! include { () => {}; }\n    include!();\n}\ninclude!(\"../../../outside/pid.rs\");",
        &["2: macro_rules! include: shadows a builtin macro", "3: include!: outside the pure macro allowlist", "5: include!: outside the pure macro allowlist"],
    ),
    (
        "m!();\nmacro_rules! m { () => {}; }\nm!();\nmod a {\n    macro_rules! n { () => {}; }\n}\nn!();",
        &["1: m!: outside the pure macro allowlist", "7: n!: outside the pure macro allowlist"],
    ),
    (
        "macro_rules! vec { () => {}; }\nmacro_rules! env { () => {}; }\nenv!();",
        &["1: macro_rules! vec: shadows a builtin macro", "2: macro_rules! env: shadows a builtin macro", "3: env!: outside the pure macro allowlist"],
    ),
    (
        "#[allow(clippy::disallowed_types)]\nfn f() {}\n#![expect(clippy::all)]\n#[cfg_attr(test, allow(warnings))]\nfn g() {}\n#[warn(clippy::style)]\nfn h() {}\n#[allow(clippy::too_many_arguments)]\nfn i() {}\n#[deny(warnings)]\nfn j() {}",
        &["1: an attribute that lowers a purity lint", "3: an attribute that lowers a purity lint", "4: an attribute that lowers a purity lint", "6: an attribute that lowers a purity lint"],
    ),
    (
        "#[cfg(test)]\nmod observe {\n    #[expect(clippy::disallowed_types, reason = \"r\")]\n    mod cells {\n        use std::cell::Cell;\n        thread_local! {\n            pub(super) static A: Cell<usize> = const { Cell::new(0) };\n            pub(super) static B: Cell<usize> = const { Cell::new(0) };\n        }\n    }\n}",
        &[],
    ),
    (
        "mod observe {\n    #[expect(clippy::disallowed_types, reason = \"r\")]\n    mod cells {\n        use std::cell::Cell;\n        thread_local! {\n            pub(super) static A: Cell<usize> = const { Cell::new(0) };\n            pub(super) static B: Cell<usize> = const { Cell::new(0) };\n        }\n    }\n}",
        &["2: an attribute that lowers a purity lint"],
    ),
    (
        "#[cfg(test)]\nmod observe {\n    #[expect(clippy::disallowed_types, reason = \"r\")]\n    mod cells {\n        use std::cell::Cell;\n        thread_local! {\n            pub(super) static A: Cell<usize> = const { Cell::new(0) };\n            pub(super) static B: Cell<usize> = const { Cell::new(0) };\n        }\n        pub fn leak() {}\n    }\n}",
        &["3: an attribute that lowers a purity lint"],
    ),
    (
        "#[cfg(test)]\nmod observe {\n    #[expect(clippy::disallowed_types, reason = \"r\")]\n    mod cells {\n        use std::cell::Cell;\n        thread_local! {\n            pub(super) static A: Cell<u64> = const { Cell::new(0) };\n            pub(super) static B: Cell<usize> = const { Cell::new(0) };\n        }\n    }\n}",
        &["3: an attribute that lowers a purity lint"],
    ),
    (
        "use std::collections::{BTreeMap, BTreeSet};\nuse std::{fmt, rc::Rc, str::FromStr};\nlet s = std::str::from_utf8(b).map(std::mem::take);\nstd::iter::once::<u8>(1);\nmacro_rules! m { () => {} }\nm!();\nfn f<'a>() -> impl Sized + use<'a> { m!() }\nmod inner { fn g() { m!(); } }\n#",
        &[],
    ),
    (
        "\"open",
        &[],
    ),
    (
        "/* open",
        &[],
    ),
    (
        "r#\"open",
        &[],
    ),
    (
        "'",
        &[],
    ),
];

/// The scanner lexes before it matches, so no comment or literal hides a
/// path or poses as one, and it refuses every form the allowlists do not
/// name: an unlisted module or item, a rename or glob of a root, a nested
/// group, a macro's metavariable, an unlisted macro, a builtin shadowed or
/// called out of scope, an `extern crate`, a `#[path]`, an ungated
/// `mod tests;`, a lowered purity lint and a clock or randomness source.
#[test]
fn the_scanner_refuses_every_form_its_allowlists_do_not_name() {
    for (source, expected) in SCANNER_CASES {
        let file = Path::new("crates/brokkr-core/src/lib.rs");
        assert_eq!(impurities(source, file), expected, "{source}");
    }
}

/// Whether `module` declares `mod tests;` under `#[cfg(test)]`, which
/// makes its `tests.rs` or `tests/` test-only. A file that cannot be read
/// declares nothing, so its tests are scanned.
fn declares_test_only_tests(module: &Path) -> bool {
    std::fs::read_to_string(module).is_ok_and(|source| {
        let tokens = lex(&source);
        tokens.windows(3).enumerate().any(|(at, window)| {
            window[0].1 == "mod"
                && window[1].1 == "tests"
                && window[2].1 == ";"
                && under_cfg_test(&tokens, at)
        })
    })
}

/// Core's and view's production source: every `.rs` under `dir` but a
/// `tests.rs` or `tests/` that one of `parents`, the files that declare
/// `dir`'s modules, declares under `#[cfg(test)]`.
fn production_sources(dir: &Path, parents: &[PathBuf], files: &mut Vec<PathBuf>) {
    let test_only = parents
        .iter()
        .any(|parent| declares_test_only_tests(parent));
    for entry in std::fs::read_dir(dir).expect("a source directory") {
        let path = entry.expect("a directory entry").path();
        let name = path.file_name().and_then(|name| name.to_str());
        if test_only && [Some("tests"), Some("tests.rs")].contains(&name) {
            continue;
        }
        if path.is_dir() {
            production_sources(
                &path,
                &[path.with_extension("rs"), path.join("mod.rs")],
                files,
            );
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

/// Ruling 1: core's and view's production source names nothing outside
/// the allowlists, and every `.rs` file in it is scanned.
#[test]
fn the_pure_crates_name_nothing_outside_the_allowlists() {
    let root = workspace();
    let mut files = Vec::new();
    for krate in ["brokkr-core", "brokkr-view"] {
        let src = root.join("crates").join(krate).join("src");
        production_sources(&src, &[src.join("lib.rs")], &mut files);
    }
    let found: Vec<String> = files
        .iter()
        .flat_map(|path| {
            let relative = path.strip_prefix(&root).expect("under the workspace");
            let source = std::fs::read_to_string(path).expect("readable source");
            impurities(&source, relative)
                .into_iter()
                .map(move |what| format!("{}:{what}", relative.display()))
        })
        .collect();
    assert_eq!(
        found,
        Vec::<String>::new(),
        "core and view are pure (decision 0071 ruling 1): effects live in store and runtime"
    );
    let crates = root.join("crates");
    assert!(files.contains(&crates.join("brokkr-core/src/lib.rs")));
    assert!(files.contains(&crates.join("brokkr-view/src/transcript.rs")));
    assert!(!files.contains(&crates.join("brokkr-view/src/transcript/tests.rs")));
    assert!(!files.contains(&crates.join("brokkr-view/src/tests.rs")));
}
