//! Every integration-test file compiles and runs (#423). Each crate sets
//! `autotests = false` and declares its tests: one binary, `tests/it.rs`,
//! whose modules are the files under tests/, and each file that needs a
//! process of its own as a `[[test]]`. Cargo then builds nothing it is not
//! told of, so a new `tests/foo.rs` nobody declares would silently never
//! compile. This refuses one: every file Cargo would find on its own,
//! `tests/*.rs` and `tests/*/main.rs`, is a test target's root, as Cargo
//! reports the targets, or a module the crate's `tests/it.rs` declares.
//!
//! The root is read as text, in the forms it is written in: a blank line,
//! a comment, `#[path = "…"]` and `mod name;`. Any other line is refused,
//! so a form the reader does not know cannot hide or invent a module. A
//! probe of the tree that fails for any reason but absence is refused too,
//! never read as an absence.
//!
//! One binary runs a file's tests by a filter, `--test it <file>::`, and a
//! filter that matches nothing runs 0 tests and passes, where the old
//! `--test <file>` failed. So every `--test it` filter a workflow, script,
//! recipe, bundle or guide runs must name a module of its crate's root,
//! and an `--exact` name a test of that module. `run_commands` reads each
//! command that may pass `--test it` word by word in a closed grammar, and
//! refuses the first character outside it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::Metadata;
use std::io;
use std::path::{Path, PathBuf};

use super::{metadata, workspace, Package};
use crate::tracked_files::tracked;
use run_commands::filters_in;

mod run_commands;

/// How Cargo reaches a file under a crate's tests/.
#[derive(Debug, PartialEq, Eq)]
enum Declared {
    /// The root of a test target.
    Target,
    /// A module of the crate's one-binary root.
    Module,
    /// Neither: Cargo never compiles it.
    Undeclared,
}

/// What the gate cannot vouch for. The operator's text is the `Display`,
/// pinned once by `each_refusal_reads_as_the_operator_sees_it`.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    /// A line of the root in no form the reader knows, by its number.
    UnknownLine {
        root: PathBuf,
        number: usize,
        line: String,
    },
    /// A `#[path]` that no `mod` follows.
    PathDeclaresNothing { root: PathBuf, path: String },
    /// A probe of the tree that failed other than by absence.
    Unreadable { path: PathBuf, kind: io::ErrorKind },
    /// A `--test it` filter whose first segment the root does not declare.
    UnknownModule {
        at: String,
        package: String,
        module: String,
    },
    /// An `--exact` name no `#[test]` of its module carries.
    UnknownTest {
        at: String,
        package: String,
        name: String,
    },
    /// A `--test it` command with a filter and no `-p`.
    NoPackage { at: String, command: String },
    /// A `--test it` command with a word, flag or form the reader does not
    /// know, by that word, or with a character outside its grammar, by
    /// that character.
    UnreadFilter {
        at: String,
        word: String,
        command: String,
    },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownLine { root, number, line } => {
                write!(
                    f,
                    "{}:{number}: a line the root reader does not know: {line}",
                    root.display()
                )
            }
            Self::PathDeclaresNothing { root, path } => {
                write!(
                    f,
                    "{}: #[path = \"{path}\"] declares no module",
                    root.display()
                )
            }
            Self::Unreadable { path, kind } => write!(
                f,
                "{}: {kind}, so what Cargo would compile there cannot be read",
                path.display()
            ),
            Self::UnknownModule {
                at,
                package,
                module,
            } => write!(
                f,
                "{at}: `--test it {module}::` names no module of {package}'s tests/it.rs, \
                 so it would run 0 tests and pass"
            ),
            Self::UnknownTest { at, package, name } => write!(
                f,
                "{at}: `--exact {name}` names no #[test] function of {package}'s tests/it.rs, \
                 so it would run 0 tests and pass"
            ),
            Self::NoPackage { at, command } => write!(
                f,
                "{at}: a `--test it` command with a filter and no -p, so no crate's root holds \
                 it: {command}"
            ),
            Self::UnreadFilter { at, word, command } => write!(
                f,
                "{at}: a `--test it` command the reader cannot hold to a module, at `{word}`: \
                 {command}"
            ),
        }
    }
}

/// What is at `path`, or `None` when nothing is. Every other error is a
/// refusal, never an absence: the gate fails closed on what it cannot read.
fn probe(
    path: &Path,
    stat: &dyn Fn(&Path) -> io::Result<Metadata>,
) -> Result<Option<Metadata>, Refusal> {
    match stat(path) {
        Ok(found) => Ok(Some(found)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(unreadable(path, &error)),
    }
}

fn unreadable(path: &Path, error: &io::Error) -> Refusal {
    Refusal::Unreadable {
        path: path.to_path_buf(),
        kind: error.kind(),
    }
}

/// The files Cargo's own discovery would make test targets: each `*.rs`
/// directly under `tests`, and each `main.rs` one directory down, every
/// probe through `stat`.
fn discoverable_with(
    tests: &Path,
    stat: &dyn Fn(&Path) -> io::Result<Metadata>,
) -> Result<Vec<PathBuf>, Refusal> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(tests).map_err(|error| unreadable(tests, &error))? {
        let path = entry.map_err(|error| unreadable(tests, &error))?.path();
        if probe(&path, stat)?.is_some_and(|probed| probed.is_dir()) {
            let main = path.join("main.rs");
            found.extend(probe(&main, stat)?.filter(Metadata::is_file).map(|_| main));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

/// The modules the one-binary root at `root` declares, by name, each
/// resolved as rustc resolves it from a crate root: `name.rs` beside the
/// root, or the `#[path]` that precedes the `mod`. A line of any other
/// form, or a `#[path]` no `mod` follows, is refused by line number.
fn modules_of(root: &Path, text: &str) -> Result<BTreeMap<String, PathBuf>, Refusal> {
    let dir = root.parent().expect("a root sits in tests/");
    let (mut modules, mut path) = (BTreeMap::new(), None);
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        let attribute = line
            .strip_prefix("#[path = \"")
            .and_then(|rest| rest.strip_suffix("\"]"));
        let module = line
            .strip_prefix("mod ")
            .and_then(|rest| rest.strip_suffix(';'))
            .filter(|name| is_identifier(name));
        if let (Some(attribute), None) = (attribute, &path) {
            path = Some(attribute.to_string());
        } else if let Some(name) = module {
            let file = path.take().unwrap_or_else(|| format!("{name}.rs"));
            modules.insert(name.to_string(), dir.join(file));
        } else if !(line.is_empty() || line.starts_with("//")) || path.is_some() {
            return Err(Refusal::UnknownLine {
                root: root.to_path_buf(),
                number: index + 1,
                line: line.to_string(),
            });
        }
    }
    match path {
        Some(path) => Err(Refusal::PathDeclaresNothing {
            root: root.to_path_buf(),
            path,
        }),
        None => Ok(modules),
    }
}

fn is_identifier(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// The roots of a package's test targets, as Cargo reports them.
fn test_roots(package: &Package) -> BTreeSet<&Path> {
    (package.targets.iter())
        .filter(|target| target.kind == ["test"])
        .map(|target| target.src_path.as_path())
        .collect()
}

/// The modules of a package's `tests/it.rs`, by name; none when the
/// package has no such target.
fn root_modules(package: &Package) -> BTreeMap<String, PathBuf> {
    let it = package.manifest_path.with_file_name("tests").join("it.rs");
    if !test_roots(package).contains(it.as_path()) {
        return BTreeMap::new();
    }
    let text =
        std::fs::read_to_string(&it).unwrap_or_else(|error| panic!("{}: {error}", it.display()));
    modules_of(&it, &text).unwrap_or_else(refused)
}

/// The workspace's own packages.
fn members() -> impl Iterator<Item = &'static Package> {
    let metadata = metadata();
    (metadata.packages.iter())
        .filter(move |package| metadata.workspace_members.contains(&package.id))
}

/// A refusal on the real tree fails the gate with the operator's text.
fn refused<T>(refusal: Refusal) -> T {
    panic!("{refusal}")
}

/// Every discoverable file of every workspace crate, by its path from the
/// workspace root, and how Cargo reaches it.
fn declarations() -> BTreeMap<String, Declared> {
    let root = workspace();
    let mut declared = BTreeMap::new();
    let stat = |path: &Path| std::fs::metadata(path);
    for package in members() {
        let tests = package.manifest_path.with_file_name("tests");
        let probed = probe(&tests, &stat).unwrap_or_else(refused);
        if !probed.is_some_and(|found| found.is_dir()) {
            continue;
        }
        let (targets, modules) = (test_roots(package), root_modules(package));
        let modules: BTreeSet<&PathBuf> = modules.values().collect();
        for file in discoverable_with(&tests, &stat).unwrap_or_else(refused) {
            let how = if targets.contains(file.as_path()) {
                Declared::Target
            } else if modules.contains(&file) {
                Declared::Module
            } else {
                Declared::Undeclared
            };
            let name = file.strip_prefix(&root).unwrap_or(&file);
            declared.insert(name.display().to_string(), how);
        }
    }
    declared
}

/// The tracked files whose commands are run: by CI, by a recipe's or a
/// bundle's seat, by a measuring script, or by an operator following a
/// guide.
const RUN_COMMANDS: [&str; 10] = [
    ".github",
    "ARCHITECTURE.md",
    "CONTRIBUTING.md",
    "README.md",
    "bundles",
    "docs",
    "packaging",
    "quality",
    "recipes",
    "scripts",
];

/// #423: no file under a crate's tests/ escapes the build. The walk must
/// first show it read the tree: a module, a target of its own and a
/// directory root, each reached the way it is declared.
#[test]
fn every_integration_test_file_is_a_module_of_the_root_or_its_own_target() {
    let declared = declarations();
    for (file, how) in [
        ("crates/brokkr-cli/tests/hands.rs", Declared::Module),
        ("crates/brokkr-cli/tests/layering/main.rs", Declared::Module),
        ("crates/brokkr-cli/tests/heap_dsh.rs", Declared::Target),
        ("crates/brokkr-runtime/tests/it.rs", Declared::Target),
    ] {
        assert_eq!(declared.get(file), Some(&how), "{file}");
    }
    let undeclared: Vec<&String> = (declared.iter())
        .filter(|(_, how)| **how == Declared::Undeclared)
        .map(|(file, _)| file)
        .collect();
    assert_eq!(
        undeclared,
        Vec::<&String>::new(),
        "Cargo never compiles these: add each to its crate's tests/it.rs as a \
         `mod`, or give it a [[test]] of its own in Cargo.toml that names why"
    );
}

/// #423: no command runs 0 tests by a stale filter. The walk must also
/// show it read the tree, by file, package and module: a workflow's plain
/// and `--exact` filters, a recipe's, a script's, a guide's and a
/// comment's, in each crate that has a root.
#[test]
fn every_test_it_filter_names_a_module_of_its_crates_root() {
    let roots: BTreeMap<String, BTreeMap<String, PathBuf>> = members()
        .map(|package| (package.name.clone(), root_modules(package)))
        .collect();
    let root = workspace();
    let module_file = |path: &Path| std::fs::read_to_string(path);
    let (mut read, mut refused) = (BTreeSet::new(), Vec::new());
    for file in tracked(&root, &RUN_COMMANDS) {
        let bytes =
            std::fs::read(root.join(&file)).unwrap_or_else(|error| panic!("{file}: {error}"));
        match filters_in(
            &file,
            &String::from_utf8_lossy(&bytes),
            &roots,
            &module_file,
        ) {
            Ok(filters) => read.extend(filters.into_iter().map(|filter| {
                let (at, _) = filter.at.rsplit_once(':').expect("file:line");
                (at.to_string(), filter.package, filter.module)
            })),
            Err(refusal) => refused.push(refusal.to_string()),
        }
    }
    assert_eq!(refused, Vec::<String>::new());
    for (file, package, module) in [
        (".github/workflows/ci.yml", "brokkr-cli", "packaging"),
        (".github/workflows/ci.yml", "brokkr-cli", "suppressions"),
        (
            "recipes/research/roles/verify-seat.sh",
            "brokkr-cli",
            "research_registry",
        ),
        ("scripts/measure-budgets.sh", "brokkr-runtime", "budgets"),
        (
            "docs/guides/provider-adapters.md",
            "brokkr-runtime",
            "witness_digests",
        ),
        ("quality/suppressions.txt", "brokkr-cli", "suppressions"),
    ] {
        let held = (file.to_string(), package.to_string(), module.to_string());
        assert!(read.contains(&held), "{held:?} is read: {read:?}");
    }
}

/// A probe that fails for any reason but absence is refused, never read
/// as an absence: an unreadable directory would hide the file in it.
#[test]
fn discovery_refuses_a_probe_it_cannot_read() {
    let tests = tempfile::tempdir().expect("a scratch tests/");
    let (planted, main) = (
        tests.path().join("planted"),
        tests.path().join("planted/main.rs"),
    );
    std::fs::create_dir(&planted).expect("planted/");
    std::fs::write(&main, "").expect("planted/main.rs");
    std::fs::write(tests.path().join("a.rs"), "").expect("a.rs");
    let failing = |kind: io::ErrorKind| {
        let main = main.clone();
        move |path: &Path| match path == main {
            true => Err(io::Error::from(kind)),
            false => std::fs::metadata(path),
        }
    };
    let found = discoverable_with(tests.path(), &|path| std::fs::metadata(path));
    assert_eq!(found, Ok(vec![tests.path().join("a.rs"), main.clone()]));
    let absent = discoverable_with(tests.path(), &failing(io::ErrorKind::NotFound));
    assert_eq!(absent, Ok(vec![tests.path().join("a.rs")]));
    assert_eq!(
        discoverable_with(tests.path(), &failing(io::ErrorKind::PermissionDenied)),
        Err(Refusal::Unreadable {
            path: main,
            kind: io::ErrorKind::PermissionDenied
        })
    );
}

/// The root reader takes the forms the roots are written in and refuses
/// every other line, and a `#[path]` that declares nothing.
#[test]
fn the_root_reader_refuses_a_line_it_does_not_know() {
    let root = Path::new("/w/tests/it.rs");
    let read = "//! doc\n\n// note\nmod a_1;\n  #[path = \"d/main.rs\"]\nmod d;\n";
    assert_eq!(
        modules_of(root, read),
        Ok(BTreeMap::from([
            ("a_1".into(), "/w/tests/a_1.rs".into()),
            ("d".into(), "/w/tests/d/main.rs".into())
        ]))
    );
    let unknown = |number, line: &str| Refusal::UnknownLine {
        root: root.into(),
        number,
        line: line.into(),
    };
    for (text, refused) in [
        ("mod a;\npub mod b;\n", unknown(2, "pub mod b;")),
        ("#[cfg(unix)]\nmod a;\n", unknown(1, "#[cfg(unix)]")),
        ("mod a {}\n", unknown(1, "mod a {}")),
        ("mod a::b;\n", unknown(1, "mod a::b;")),
        ("mod ;\n", unknown(1, "mod ;")),
        ("#[path = \"x.rs\"]\n\nmod x;\n", unknown(2, "")),
        (
            "#[path = \"x.rs\"]\n#[path = \"y.rs\"]\nmod y;\n",
            unknown(2, "#[path = \"y.rs\"]"),
        ),
        ("mod a; mod b;\n", unknown(1, "mod a; mod b;")),
        (
            "#[path = \"x.rs\"]\n",
            Refusal::PathDeclaresNothing {
                root: root.into(),
                path: "x.rs".into(),
            },
        ),
    ] {
        assert_eq!(modules_of(root, text), Err(refused), "{text}");
    }
}

/// Each refusal's text, pinned once.
#[test]
fn each_refusal_reads_as_the_operator_sees_it() {
    for (refusal, text) in [
        (
            Refusal::UnknownLine {
                root: "/w/tests/it.rs".into(),
                number: 2,
                line: "pub mod b;".into(),
            },
            "/w/tests/it.rs:2: a line the root reader does not know: pub mod b;",
        ),
        (
            Refusal::PathDeclaresNothing {
                root: "/w/tests/it.rs".into(),
                path: "x.rs".into(),
            },
            "/w/tests/it.rs: #[path = \"x.rs\"] declares no module",
        ),
        (
            Refusal::Unreadable {
                path: "/w/tests/d/main.rs".into(),
                kind: io::ErrorKind::PermissionDenied,
            },
            "/w/tests/d/main.rs: permission denied, so what Cargo would compile there \
             cannot be read",
        ),
        (
            Refusal::UnknownModule {
                at: "ci.yml:9".into(),
                package: "brokkr-cli".into(),
                module: "gone".into(),
            },
            "ci.yml:9: `--test it gone::` names no module of brokkr-cli's tests/it.rs, \
             so it would run 0 tests and pass",
        ),
        (
            Refusal::UnknownTest {
                at: "ci.yml:9".into(),
                package: "brokkr-cli".into(),
                name: "gone::a".into(),
            },
            "ci.yml:9: `--exact gone::a` names no #[test] function of brokkr-cli's \
             tests/it.rs, so it would run 0 tests and pass",
        ),
        (
            Refusal::NoPackage {
                at: "ci.yml:9".into(),
                command: "cargo test --test it x::".into(),
            },
            "ci.yml:9: a `--test it` command with a filter and no -p, so no crate's root \
             holds it: cargo test --test it x::",
        ),
        (
            Refusal::UnreadFilter {
                at: "ci.yml:9".into(),
                word: "--workspace".into(),
                command: "cargo test --workspace --test it x::".into(),
            },
            "ci.yml:9: a `--test it` command the reader cannot hold to a module, at \
             `--workspace`: cargo test --workspace --test it x::",
        ),
    ] {
        assert_eq!(refusal.to_string(), text);
    }
}
