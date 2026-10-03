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
//! so a form the reader does not know cannot hide or invent a module.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::{metadata, workspace};

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

/// The files Cargo's own discovery would make test targets: each `*.rs`
/// directly under `tests`, and each `main.rs` one directory down.
fn discoverable(tests: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in
        std::fs::read_dir(tests).unwrap_or_else(|error| panic!("{}: {error}", tests.display()))
    {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            found.extend(Some(path.join("main.rs")).filter(|main| main.is_file()));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// The files the one-binary root at `root` declares as modules, each
/// resolved as rustc resolves it from a crate root: `name.rs` beside the
/// root, or the `#[path]` that precedes the `mod`. A line of any other
/// form, or a `#[path]` no `mod` follows, is refused by line number.
fn modules_of(root: &Path, text: &str) -> Result<BTreeSet<PathBuf>, String> {
    let dir = root.parent().expect("a root sits in tests/");
    let (mut modules, mut path) = (BTreeSet::new(), None);
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        let attribute = line
            .strip_prefix("#[path = \"")
            .and_then(|rest| rest.strip_suffix("\"]"));
        let module = line
            .strip_prefix("mod ")
            .and_then(|rest| rest.strip_suffix(';'))
            .filter(|name| {
                !name.is_empty() && name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
            });
        if let (Some(attribute), None) = (attribute, &path) {
            path = Some(attribute.to_string());
        } else if let Some(name) = module {
            modules.insert(dir.join(path.take().unwrap_or_else(|| format!("{name}.rs"))));
        } else if !(line.is_empty() || line.starts_with("//")) || path.is_some() {
            return Err(format!("{}:{}: {line}", root.display(), index + 1));
        }
    }
    match path {
        Some(path) => Err(format!(
            "{}: #[path = \"{path}\"] declares no module",
            root.display()
        )),
        None => Ok(modules),
    }
}

/// Every discoverable file of every workspace crate, by its path from the
/// workspace root, and how Cargo reaches it.
fn declarations() -> BTreeMap<String, Declared> {
    let (metadata, root) = (metadata(), workspace());
    let mut declared = BTreeMap::new();
    let members = metadata
        .packages
        .iter()
        .filter(|package| metadata.workspace_members.contains(&package.id));
    for package in members {
        let tests = package.manifest_path.with_file_name("tests");
        if !tests.is_dir() {
            continue;
        }
        let targets: BTreeSet<&Path> = (package.targets.iter())
            .filter(|target| target.kind == ["test"])
            .map(|target| target.src_path.as_path())
            .collect();
        let it = tests.join("it.rs");
        let modules = match targets.contains(it.as_path()) {
            true => modules_of(&it, &std::fs::read_to_string(&it).expect("the root reads"))
                .unwrap_or_else(|line| panic!("a line the root reader does not know: {line}")),
            false => BTreeSet::new(),
        };
        for file in discoverable(&tests) {
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

/// The root reader takes the forms the roots are written in and refuses
/// every other line, and a `#[path]` that declares nothing.
#[test]
fn the_root_reader_refuses_a_line_it_does_not_know() {
    let root = Path::new("/w/tests/it.rs");
    let read = "//! doc\n\n// note\nmod a_1;\n  #[path = \"d/main.rs\"]\nmod d;\n";
    assert_eq!(
        modules_of(root, read),
        Ok(BTreeSet::from([
            "/w/tests/a_1.rs".into(),
            "/w/tests/d/main.rs".into()
        ]))
    );
    for (text, refused) in [
        ("mod a;\npub mod b;\n", "/w/tests/it.rs:2: pub mod b;"),
        ("#[cfg(unix)]\nmod a;\n", "/w/tests/it.rs:1: #[cfg(unix)]"),
        ("mod a {}\n", "/w/tests/it.rs:1: mod a {}"),
        ("mod a::b;\n", "/w/tests/it.rs:1: mod a::b;"),
        ("mod ;\n", "/w/tests/it.rs:1: mod ;"),
        ("#[path = \"x.rs\"]\n\nmod x;\n", "/w/tests/it.rs:2: "),
        (
            "#[path = \"x.rs\"]\n#[path = \"y.rs\"]\nmod y;\n",
            "/w/tests/it.rs:2: #[path = \"y.rs\"]",
        ),
        ("mod a; mod b;\n", "/w/tests/it.rs:1: mod a; mod b;"),
        (
            "#[path = \"x.rs\"]\n",
            "/w/tests/it.rs: #[path = \"x.rs\"] declares no module",
        ),
    ] {
        assert_eq!(modules_of(root, text), Err(refused.to_string()), "{text}");
    }
}
