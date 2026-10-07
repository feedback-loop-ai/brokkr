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
//! recipe, bundle or guide runs must name a test the package's own `it`
//! binary carries: the gate asks each crate's built binary for its list
//! (`--list --format terse`), holds a filter to a module with a listed
//! test of its own, and holds an `--exact` name to a listed name
//! (#543). The list is the binary of the command's own build
//! configuration, default features in the dev profile; a command that
//! names any build-moving flag — another feature set, another profile,
//! `--config`, `-Z`, a `+toolchain` — or an assignment before `cargo`
//! whose name is `CARGO_*` or `RUST*`, is refused, named, and never held
//! to the list of a build it does not run. A filter under more than one
//! `-p` is refused too, because Cargo unifies the selected packages'
//! features. `run_commands` reads each command that may pass `--test it`
//! word by word in a closed grammar, and refuses the first character
//! outside it. The binary already reflects a `cfg`, a `cfg_attr` and an
//! inner `#![cfg]`, so a test those gate away from the compiling host is in
//! no list and refuses; the source is never read for tests. A binary that
//! cannot be built or listed refuses, named, and is never skipped.
//!
//! The text gate can refuse only the words it sees, so a workflow's or a
//! script's command must also run through `scripts/run-it-tests.sh`, the
//! checked entry point that fails a filtered run executing 0 tests
//! whatever the line, the environment or the profile did to the build. A
//! guide's command is read from its text alone: an operator copies it, and
//! no job here runs it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::Metadata;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use super::{metadata, workspace, Package};
use crate::tracked_files::tracked;
use crate::workspace_root::GUARD;
use run_commands::{filters_in, fixture_lists, Filter, Lists};
use serde::Deserialize;

#[path = "test_targets/run_commands.rs"]
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
    /// A `--test it` filter whose module no listed test of the package's
    /// binary begins with: the module holds no test, or is no module.
    TestlessModule {
        at: String,
        package: String,
        module: String,
    },
    /// An `--exact` name the package's binary does not list.
    UnknownTest {
        at: String,
        package: String,
        name: String,
    },
    /// A `--test it` command with a filter and no `-p`.
    NoPackage { at: String, command: String },
    /// A `--test it` command with a filter and more than one `-p`: Cargo
    /// unifies the selected packages' features, so no single listed build
    /// holds the filter.
    ManyPackages {
        at: String,
        packages: Vec<String>,
        command: String,
    },
    /// A `--test it` command in a workflow or script that bypasses the
    /// checked entry point, `scripts/run-it-tests.sh`, which fails a run
    /// that executes 0 tests.
    Unguarded { at: String, command: String },
    /// A `--test it` command with a word, flag or form the reader does not
    /// know, by that word, or with a character outside its grammar, by
    /// that character.
    UnreadFilter {
        at: String,
        word: String,
        command: String,
    },
    /// A package's `it` binary no list was read of: the workspace has no
    /// `tests/it` for it, or its build or its list failed, by `why`.
    UnreadList {
        at: String,
        package: String,
        why: String,
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
            Self::TestlessModule {
                at,
                package,
                module,
            } => write!(
                f,
                "{at}: `--test it {module}::` names no test of {package}'s it binary, \
                 so it would run 0 tests and pass"
            ),
            Self::UnknownTest { at, package, name } => write!(
                f,
                "{at}: `--exact {name}` names no test of {package}'s it binary, \
                 so it would run 0 tests and pass"
            ),
            Self::NoPackage { at, command } => write!(
                f,
                "{at}: a `--test it` command with a filter and no -p, so no crate's root holds \
                 it: {command}"
            ),
            Self::ManyPackages {
                at,
                packages,
                command,
            } => write!(
                f,
                "{at}: a `--test it` filter under more than one -p ({}), whose features Cargo \
                 unifies, so no listed build holds it: {command}",
                packages.join(", ")
            ),
            Self::Unguarded { at, command } => write!(
                f,
                "{at}: a `--test it` command must run through {} so a filtered run that \
                 executes 0 tests fails: {command}",
                GUARD
            ),
            Self::UnreadFilter { at, word, command } => write!(
                f,
                "{at}: a `--test it` command the reader cannot hold to a module, at `{word}`: \
                 {command}"
            ),
            Self::UnreadList { at, package, why } => write!(
                f,
                "{at}: {package}'s it binary was not listed ({why}), \
                 so a filter cannot be held to it"
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

/// The tests each workspace crate's `it` binary carries, by package, read
/// once: brokkr-cli's is the binary this gate runs in (`current_exe`),
/// whose package declares no features so no feature flag can change it;
/// each other crate's is built and located through cargo's own JSON
/// messages, then listed. The build is the one the scanned commands make,
/// default features in the dev profile; `run_commands` refuses a command
/// that names any other feature set or profile, so no filter is held to
/// the list of a build it does not run. One that cannot be built or listed
/// refuses, naming it; it is never skipped.
fn test_lists() -> &'static Lists {
    static LISTS: OnceLock<Lists> = OnceLock::new();
    LISTS.get_or_init(|| {
        let mut lists = Lists::new();
        for package in members() {
            let it = package.manifest_path.with_file_name("tests").join("it.rs");
            if !test_roots(package).contains(it.as_path()) {
                continue;
            }
            let executable = if package.name == env!("CARGO_PKG_NAME") {
                assert!(
                    package.features.is_empty(),
                    "{}: the gate lists its own `current_exe`, so the package must declare no \
                     features for the list to be the command's own build",
                    package.name
                );
                if let Some(why) = own_profile_refusal(cfg!(debug_assertions)) {
                    return refused(unread_list(&package.name, why));
                }
                std::env::current_exe()
                    .map_err(|error| error.to_string())
                    .unwrap_or_else(|why| refused(unread_list(&package.name, &why)))
            } else {
                built_it(package).unwrap_or_else(|why| refused(unread_list(&package.name, &why)))
            };
            let carried =
                listed(&executable).unwrap_or_else(|why| refused(unread_list(&package.name, &why)));
            lists.insert(package.name.clone(), carried);
        }
        lists
    })
}

fn unread_list(package: &str, why: &str) -> Refusal {
    Refusal::UnreadList {
        at: "the gate".into(),
        package: package.to_string(),
        why: why.to_string(),
    }
}

/// The reason the gate's own `current_exe` is not the build the scanned
/// commands run, when it is not: those commands use `cargo test` in the
/// dev profile, whose `debug_assertions` mark a release build lacks. The
/// list is the command's own build, so a release gate refuses rather than
/// hold filters to another build's names.
fn own_profile_refusal(debug_assertions: bool) -> Option<&'static str> {
    (!debug_assertions).then_some(
        "the gate lists its own `current_exe`, which is not the dev-profile build the \
         scanned commands run",
    )
}

/// The executable of a package's `it` test binary, built and located
/// through cargo's own JSON messages: the `compiler-artifact` whose
/// target is the `it` test names it. The build is the default feature set
/// in the dev profile, the configuration the scanned commands make. Cargo's
/// stderr carries progress, so only a failed run or a missing executable is
/// an error.
fn built_it(package: &Package) -> Result<PathBuf, String> {
    let output = Command::new(env!("CARGO"))
        .args([
            "test",
            "--locked",
            "--offline",
            "--no-run",
            "--message-format=json",
            "-p",
            &package.name,
            "--test",
            "it",
        ])
        .current_dir(workspace())
        .output()
        .map_err(|error| format!("cargo could not run: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo test --no-run failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let lines = String::from_utf8_lossy(&output.stdout);
    (lines.lines())
        .find_map(|line| {
            let artifact: Artifact = serde_json::from_str(line).ok()?;
            let is_it = artifact.target.name == "it"
                && artifact.target.kind.iter().any(|kind| kind == "test");
            is_it.then_some(artifact.executable).flatten()
        })
        .ok_or_else(|| "cargo named no it test binary".to_string())
}

/// The part of cargo's JSON messages the gate reads: the artifact of a
/// test target and the executable it built. Cargo adds fields to its
/// messages over time, so unknown fields are ignored; a message without
/// a target is no artifact of a test.
#[derive(Deserialize)]
struct Artifact {
    target: ArtifactTarget,
    executable: Option<PathBuf>,
}

#[derive(Deserialize)]
struct ArtifactTarget {
    name: String,
    kind: Vec<String>,
}

/// The tests the binary at `executable` carries, as its own terse list
/// reports them: a line `name: test`. A bench is listed but is no test
/// `cargo test` would run; a line of any other kind is an error, never
/// an absence.
fn listed(executable: &Path) -> Result<BTreeSet<String>, String> {
    let output = Command::new(executable)
        .args(["--list", "--format", "terse"])
        .output()
        .map_err(|error| format!("{} could not run: {error}", executable.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{} --list failed: {}",
            executable.display(),
            output.status
        ));
    }
    let mut tests = BTreeSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some((name, kind)) = line.rsplit_once(": ") else {
            return Err(format!(
                "{} lists a line it does not read: {line}",
                executable.display()
            ));
        };
        match kind {
            "test" => {
                tests.insert(name.to_string());
            }
            "bench" => {}
            _ => {
                return Err(format!(
                    "{} lists a line it does not read: {line}",
                    executable.display()
                ))
            }
        }
    }
    Ok(tests)
}

/// A list the gate cannot read is an error naming it, never an absence:
/// a binary that exits badly, and one whose lines are not a list.
#[test]
fn a_list_the_gate_cannot_read_is_refused() {
    assert_eq!(
        listed(Path::new("/usr/bin/false")),
        Err("/usr/bin/false --list failed: exit status: 1".to_string())
    );
    assert_eq!(
        listed(Path::new("/bin/echo")),
        Err("/bin/echo lists a line it does not read: --list --format terse".to_string())
    );
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
/// comment's, in each crate that has a binary.
#[test]
fn every_test_it_filter_names_a_test_of_its_crates_binary() {
    let lists = test_lists();
    let root = workspace();
    let (mut read, mut refused) = (BTreeSet::new(), Vec::new());
    for file in tracked(&root, &RUN_COMMANDS) {
        let bytes =
            std::fs::read(root.join(&file)).unwrap_or_else(|error| panic!("{file}: {error}"));
        match filters_in(&file, &String::from_utf8_lossy(&bytes), lists) {
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

/// #543 A3 and A5: a name is held to a test the binary carries and a
/// filter to a module whose tests the list records. A name that exists
/// only as the text of `hands.rs`'s raw string literal is in no list,
/// each support module of a root holds no listed test, and a filter into
/// a directory module still reads: `layering` carries tests of its own.
#[test]
fn a_filter_is_held_to_a_module_that_holds_tests_and_a_name_to_a_test() {
    let lists = fixture_lists();
    for (package, module) in [
        ("brokkr-cli", "numbered"),
        ("brokkr-cli", "rust_source"),
        ("brokkr-cli", "test_paths"),
        ("brokkr-cli", "tracked_files"),
        ("brokkr-cli", "workflow"),
        ("brokkr-cli", "workspace_root"),
        ("brokkr-runtime", "witnesses"),
    ] {
        let text = format!("cargo test --locked -p {package} --test it {module}::\n");
        let refused = Refusal::TestlessModule {
            at: "f:1".into(),
            package: package.into(),
            module: module.into(),
        };
        assert_eq!(filters_in("f", &text, &lists), Err(refused), "{text}");
    }
    for name in ["hands::named_pass", "hands::named_fail_when_requested"] {
        let text = format!("cargo test --locked -p brokkr-cli --test it -- --exact {name}\n");
        let refused = Refusal::UnknownTest {
            at: "f:1".into(),
            package: "brokkr-cli".into(),
            name: name.into(),
        };
        assert_eq!(filters_in("f", &text, &lists), Err(refused), "{text}");
    }
    let held = Ok(vec![Filter {
        at: "f:1".into(),
        package: "brokkr-cli".into(),
        module: "layering".into(),
    }]);
    assert_eq!(
        filters_in(
            "f",
            "cargo test --locked -p brokkr-cli --test it layering::test_targets\n",
            &lists
        ),
        held
    );
    let held = Ok(vec![Filter {
        at: "f:1".into(),
        package: "brokkr-cli".into(),
        module: "hands".into(),
    }]);
    assert_eq!(
        filters_in(
            "f",
            "cargo test --locked -p brokkr-cli --test it hands::\n",
            &lists
        ),
        held,
        "hands:: names a module the list carries tests of"
    );
}

/// #543: the binary's own list is the fact, on this host. The gate's own
/// test is in it, so a list that silently read as empty could not pass
/// here; the two names `hands.rs`'s raw string literal shows are in no
/// list; the tests `hands.rs`'s own `#![cfg(target_os = "linux")]` and the
/// two macOS `#[cfg]` gates `doctor_dsh_selection`'s and `init_doctor`'s
/// are in this host's list exactly when the predicate holds, the binary
/// having resolved them by construction; and no support module's name is
/// in any list.
#[test]
fn the_binarys_own_list_is_what_the_gate_holds() {
    let lists = test_lists();
    let cli = &lists["brokkr-cli"];
    assert!(
        cli.contains("layering::test_targets::the_binarys_own_list_is_what_the_gate_holds"),
        "the gate's own test is listed: {cli:?}"
    );
    for name in ["hands::named_pass", "hands::named_fail_when_requested"] {
        assert!(!cli.contains(name), "{name} is the text of a literal");
    }
    assert!(
        cli.iter().any(|test| test.starts_with("hands::")) == cfg!(target_os = "linux"),
        "hands.rs is #![cfg(target_os = \"linux\")]"
    );
    for name in [
        "doctor_dsh_selection::apple_default_search_excludes_confstr_only_directories",
        "init_doctor::a_macos_scaffold_declares_harness_and_asks_nothing_of_bubblewrap",
    ] {
        assert_eq!(
            cli.contains(name),
            cfg!(target_os = "macos"),
            "the macOS-gated test {name} is the binary's exactly on macOS"
        );
    }
    for module in [
        "numbered",
        "rust_source",
        "test_paths",
        "tracked_files",
        "workflow",
        "workspace_root",
    ] {
        assert!(
            !cli.iter()
                .any(|test| test.starts_with(&format!("{module}::"))),
            "{module} holds no test of the binary"
        );
    }
    let runtime = &lists["brokkr-runtime"];
    assert!(
        !runtime.iter().any(|test| test.starts_with("witnesses::")),
        "witnesses is a support module: {runtime:?}"
    );
    assert!(
        runtime
            .iter()
            .any(|test| test.starts_with("witness_digests::")),
        "witness_digests holds tests of the binary"
    );
    for package in ["brokkr-core", "brokkr-store"] {
        assert!(
            lists.get(package).is_some_and(|tests| !tests.is_empty()),
            "{package}'s it binary was listed and carries tests"
        );
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
            Refusal::TestlessModule {
                at: "ci.yml:9".into(),
                package: "brokkr-cli".into(),
                module: "gone".into(),
            },
            "ci.yml:9: `--test it gone::` names no test of brokkr-cli's it binary, \
             so it would run 0 tests and pass",
        ),
        (
            Refusal::UnknownTest {
                at: "ci.yml:9".into(),
                package: "brokkr-cli".into(),
                name: "gone::a".into(),
            },
            "ci.yml:9: `--exact gone::a` names no test of brokkr-cli's it binary, \
             so it would run 0 tests and pass",
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
            Refusal::UnreadList {
                at: "ci.yml:9".into(),
                package: "brokkr-core".into(),
                why: "cargo test --no-run failed: the linker could not find -lobjc".into(),
            },
            "ci.yml:9: brokkr-core's it binary was not listed (cargo test --no-run failed: \
             the linker could not find -lobjc), so a filter cannot be held to it",
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
        (
            Refusal::ManyPackages {
                at: "ci.yml:9".into(),
                packages: vec!["brokkr-cli".into(), "brokkr-core".into()],
                command: "cargo test -p brokkr-cli -p brokkr-core --test it x::".into(),
            },
            "ci.yml:9: a `--test it` filter under more than one -p (brokkr-cli, brokkr-core), \
             whose features Cargo unifies, so no listed build holds it: cargo test -p \
             brokkr-cli -p brokkr-core --test it x::",
        ),
        (
            Refusal::Unguarded {
                at: "ci.yml:9".into(),
                command: "cargo test --test it x::".into(),
            },
            "ci.yml:9: a `--test it` command must run through scripts/run-it-tests.sh so a \
             filtered run that executes 0 tests fails: cargo test --test it x::",
        ),
    ] {
        assert_eq!(refusal.to_string(), text);
    }
}

/// #543: the gate's own list is the dev-profile binary the scanned
/// commands run, and a release build is refused rather than held to
/// another build's names. `debug_assertions` is the mark a release build
/// lacks; the real gate reads it from its own build, and this pins both
/// halves reachable.
#[test]
fn the_gates_own_list_is_bound_to_the_dev_profile() {
    assert_eq!(own_profile_refusal(true), None);
    assert_eq!(
        own_profile_refusal(false),
        Some(
            "the gate lists its own `current_exe`, which is not the dev-profile build the \
             scanned commands run"
        )
    );
}
