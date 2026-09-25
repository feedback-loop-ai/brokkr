//! Decision 0071 ruling 1's gates that `cargo-deny` and clippy cannot
//! hold alone (#336).
//!
//! `deny.toml` is the one home of the crate graph: each `[bans]` entry
//! names the only crates allowed to depend on it directly. cargo-deny
//! cannot tell a dev edge from a normal one, and Cargo accepts a dev edge
//! that cycles, so the test here holds every edge `cargo metadata`
//! reports, dev ones included, to the same table, read strictly. The
//! purity half checks that core and view share one `clippy.toml` which
//! repeats the root's thresholds, and backs clippy's item paths with a
//! token scan that refuses whole modules in every import form.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// The part of `cargo metadata --format-version 1 --no-deps` this test
/// reads. Cargo adds fields to the format over time, so unknown fields
/// are ignored; an unknown dependency kind fails the parse.
#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    dependencies: Vec<Dependency>,
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

/// The workspace's packages and their edges, as Cargo reads the manifests.
fn metadata() -> Metadata {
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
        ])
        .arg("--manifest-path")
        .arg(workspace().join("Cargo.toml"))
        .output()
        .expect("cargo runs");
    assert_eq!(
        (
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ),
        (Some(0), "".into()),
        "cargo metadata reads the workspace"
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata's format 1")
}

/// The `deny.toml` wrappers the tests alone take: each is a dev edge that
/// points down or sideways, never above its crate's layer. cargo-deny
/// admits a wrapper's edge of any kind, so this test refuses a normal or
/// build edge between these pairs.
const DEV_ONLY: [(&str, &str); 3] = [
    ("brokkr-runtime", "rusqlite"),
    ("brokkr-bridge", "rusqlite"),
    ("brokkr-cli", "rusqlite"),
];

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

/// Ruling 1, the one-way graph: every workspace crate but the binary is
/// governed by `deny.toml`, and every edge Cargo reads to a governed crate
/// comes from a parent its entry allows, dev edges included; a dev-only
/// wrapper admits a dev edge alone.
#[test]
fn every_manifest_edge_is_in_the_allowed_graph() {
    let mut parents = allowed_parents(&read("deny.toml")).expect("deny.toml's [bans] reads");
    let packages = metadata().packages;
    let ungoverned: Vec<&str> = packages
        .iter()
        .map(|package| package.name.as_str())
        .filter(|name| !parents.contains_key(*name))
        .collect();
    assert_eq!(
        ungoverned,
        ["brokkr-cli"],
        "deny.toml places every library crate in the graph"
    );
    parents.insert("brokkr-cli".to_string(), BTreeSet::new());

    let mut refused = Vec::new();
    let mut dev_only_used = BTreeSet::new();
    let mut checked = 0;
    for package in &packages {
        for dependency in &package.dependencies {
            let Some(allowed) = parents.get(&dependency.name) else {
                continue;
            };
            checked += 1;
            let kind = dependency.kind.unwrap_or(Kind::Normal);
            let pair = (package.name.as_str(), dependency.name.as_str());
            let dev_only = DEV_ONLY.contains(&pair);
            if dev_only && kind == Kind::Dev {
                dev_only_used.insert(pair);
            }
            if !allowed.contains(&package.name) || (dev_only && kind != Kind::Dev) {
                refused.push(format!("{} -> {} ({kind:?})", pair.0, pair.1));
            }
        }
    }
    assert_eq!(
        refused,
        Vec::<String>::new(),
        "edges outside the one-way graph (decision 0071 ruling 1)"
    );
    assert_eq!(
        dev_only_used,
        BTreeSet::from(DEV_ONLY),
        "every dev-only wrapper is a dev edge in use"
    );
    assert_eq!(checked, 29, "the governed edges Cargo reports");
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

/// Clippy's disallowed lists name items, never whole modules, so the
/// backstop refuses std's effectful modules by name.
const EFFECTFUL_MODULES: [&str; 7] = ["fs", "process", "env", "net", "thread", "io", "os"];

/// Clock and randomness sources, refused wherever they are named.
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

const TERMINAL_MACROS: [&str; 5] = ["print", "println", "eprint", "eprintln", "dbg"];

/// Rust source as `(line, token)`: words (a raw identifier's `r#`
/// dropped), `::` as one token, and every other non-space character
/// alone. Whitespace, newlines included, separates tokens and nothing
/// more, so an import split over lines reads as it would on one.
fn tokens(source: &str) -> Vec<(usize, String)> {
    let mut tokens = Vec::new();
    let mut line = 1;
    let mut chars = source.chars().peekable();
    let word = |c: &char| c.is_alphanumeric() || *c == '_';
    while let Some(c) = chars.next() {
        if c == '\n' {
            line += 1;
        } else if word(&c) {
            let mut text = String::from(c);
            while let Some(next) = chars.next_if(word) {
                text.push(next);
            }
            if text != "r" || chars.next_if_eq(&'#').is_none() {
                tokens.push((line, text));
            }
        } else if c == ':' && chars.next_if_eq(&':').is_some() {
            tokens.push((line, "::".to_string()));
        } else if !c.is_whitespace() {
            tokens.push((line, c.to_string()));
        }
    }
    tokens
}

/// The first root a `std::{ … }` group imports that reaches the world, at
/// any nesting of the group: an effectful module, or `self` and `*`,
/// which bring std itself in.
fn group_impurity(group: &[(usize, String)]) -> Option<(usize, String)> {
    let mut depth = 1;
    let mut item_start = true;
    for (line, token) in group {
        let token = token.as_str();
        if item_start && (EFFECTFUL_MODULES.contains(&token) || token == "self" || token == "*") {
            return Some((*line, format!("std::{{ {token} }}")));
        }
        match token {
            "{" => depth += 1,
            "}" if depth == 1 => return None,
            "}" => depth -= 1,
            _ => {}
        }
        item_start = depth == 1 && token == ",";
    }
    None
}

/// Every place a source names an effectful std module in any path or
/// import form, aliases std, or names a clock, randomness source or
/// terminal macro, as `line: what`.
fn impurities(source: &str) -> Vec<String> {
    let tokens = tokens(source);
    let text = |at: usize| tokens.get(at).map_or("", |(_, token)| token.as_str());
    let mut found = Vec::new();
    for (at, (line, token)) in tokens.iter().enumerate() {
        let what = match (token.as_str(), text(at + 1), text(at + 2)) {
            ("std", "as", _) => Some((*line, "an alias of std".to_string())),
            ("std", "::", "*") => Some((*line, "a glob import of std".to_string())),
            ("std", "::", "{") => group_impurity(&tokens[at + 3..]),
            ("std", "::", module) if EFFECTFUL_MODULES.contains(&module) => {
                Some((*line, format!("std::{module}")))
            }
            (name, _, _) if SOURCES.contains(&name) => Some((*line, name.to_string())),
            (name, "!", _) if TERMINAL_MACROS.contains(&name) => Some((*line, format!("{name}!"))),
            _ => None,
        };
        found.extend(what.map(|(line, what)| format!("{line}: {what}")));
    }
    found
}

/// The backstop reads the forms a line-scoped search misses: a group split
/// over lines, nested groups, an alias or glob of std, spaced paths and
/// raw identifiers.
#[test]
fn the_backstop_refuses_every_import_form() {
    let cases: [(&str, &[&str]); 9] = [
        (
            "use std::{\n    collections::BTreeMap,\n    fs,\n};\nfn f() { fs::hard_link(a, b); }",
            &["3: std::{ fs }"],
        ),
        (
            "use std::{collections::{BTreeMap}, io::{self, Write}};",
            &["1: std::{ io }"],
        ),
        (
            "use std as s;\nfn f() { s::fs::hard_link(a, b); }",
            &["1: an alias of std"],
        ),
        ("extern crate std as s;", &["1: an alias of std"]),
        ("use ::std::*;", &["1: a glob import of std"]),
        ("use std::{self as s};", &["1: std::{ self }"]),
        ("let p = std :: r#fs :: read(p);", &["1: std::fs"]),
        (
            "let now = Instant::now();\nprintln ! (\"{now:?}\");",
            &["1: Instant", "2: println!"],
        ),
        (
            "use std::{collections::BTreeMap, sync::{Arc, Mutex}, fmt};\nlet out = stdout_of(x);",
            &[],
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(impurities(source), expected, "{source}");
    }
}

/// Core's and view's production source: every `.rs` under `src/` but the
/// `tests.rs` modules and `tests/` directories.
fn production_sources(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a source directory") {
        let path = entry.expect("a directory entry").path();
        let name = path.file_name().and_then(|name| name.to_str());
        if path.is_dir() && name != Some("tests") {
            production_sources(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") && name != Some("tests.rs") {
            files.push(path);
        }
    }
}

/// Ruling 1's backstop: core's and view's production source names no
/// effectful module, clock, randomness source or terminal macro.
#[test]
fn the_pure_crates_name_no_effectful_module() {
    let root = workspace();
    let mut files = Vec::new();
    for krate in ["brokkr-core", "brokkr-view"] {
        production_sources(&root.join("crates").join(krate).join("src"), &mut files);
    }
    let found: Vec<String> = files
        .iter()
        .flat_map(|path| {
            let relative = path.strip_prefix(&root).expect("under the workspace");
            let source = std::fs::read_to_string(path).expect("readable source");
            impurities(&source)
                .into_iter()
                .map(move |what| format!("{}:{what}", relative.display()))
        })
        .collect();
    assert_eq!(
        found,
        Vec::<String>::new(),
        "core and view are pure (decision 0071 ruling 1): effects live in store and runtime"
    );
    for scanned in ["brokkr-core/src/lib.rs", "brokkr-view/src/transcript.rs"] {
        assert!(
            files.contains(&root.join("crates").join(scanned)),
            "{scanned} is scanned"
        );
    }
}
