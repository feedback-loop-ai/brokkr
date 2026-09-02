//! Decision 0019's standing check over current prose and metadata.
//!
//! Historical records, frozen fixtures, contract bodies, reference heritage,
//! and archived proposals never enter this walk. Rust source contributes only
//! comments; protocol constants and tests that pin frozen bytes remain data.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
enum Surface {
    Prose,
    RustComments,
    JsonDescription,
    CargoDescription,
}

#[derive(Debug, PartialEq, Eq)]
struct Offense {
    file: String,
    line: usize,
    text: String,
}

const FIXED_PROSE: [&str; 9] = [
    "README.md",
    "ARCHITECTURE.md",
    "CONTRIBUTING.md",
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
    "docs/target-architecture.md",
    "docs/extension-model.md",
    "docs/decisions/README.md",
    "contracts/README.md",
];

const RETIRED_CRATES: [&str; 8] = [
    "forge-cli",
    "forge-core",
    "forge-store",
    "forge-view",
    "forge-runtime",
    "forge_core",
    "forge_store",
    "forge_runtime",
];
const RETIRED_PRODUCT: &str = concat!("For", "ge");
const RETIRED_REPOSITORY: &str = concat!("the-", "forge");

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn visit(
    dir: &Path,
    accept: fn(&Path) -> bool,
    surface: Surface,
    found: &mut Vec<(PathBuf, Surface)>,
) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
    {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        let kind = entry.file_type().expect("file type");
        if kind.is_dir() {
            visit(&path, accept, surface, found);
        } else if kind.is_file() && accept(&path) {
            found.push((path, surface));
        }
    }
}

fn extension(path: &Path, wanted: &str) -> bool {
    path.extension().is_some_and(|value| value == wanted)
}

fn file_name(path: &Path, wanted: &str) -> bool {
    path.file_name().is_some_and(|value| value == wanted)
}

fn living_surfaces(root: &Path) -> Vec<(PathBuf, Surface)> {
    let mut found = FIXED_PROSE
        .iter()
        .map(|relative| (root.join(relative), Surface::Prose))
        .collect::<Vec<_>>();

    for relative in [
        "docs/guides",
        "specs",
        "recipes",
        "bundles",
        "agents/charters",
    ] {
        visit(
            &root.join(relative),
            |path| extension(path, "md"),
            Surface::Prose,
            &mut found,
        );
    }
    for relative in ["recipes", "bundles"] {
        visit(
            &root.join(relative),
            |path| file_name(path, "policy.json"),
            Surface::JsonDescription,
            &mut found,
        );
    }
    visit(
        &root.join("agents"),
        |path| extension(path, "json"),
        Surface::JsonDescription,
        &mut found,
    );
    visit(
        &root.join("crates"),
        |path| extension(path, "rs"),
        Surface::RustComments,
        &mut found,
    );
    visit(
        &root.join("crates"),
        |path| file_name(path, "Cargo.toml"),
        Surface::CargoDescription,
        &mut found,
    );

    found.sort_by(|left, right| left.0.cmp(&right.0));
    found.dedup_by(|left, right| left.0 == right.0);
    found
}

fn word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn contains_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(start, _)| {
        let before = start
            .checked_sub(1)
            .and_then(|index| text.as_bytes().get(index));
        let end = start + word.len();
        let after = text.as_bytes().get(end);
        before.is_none_or(|byte| !word_byte(*byte)) && after.is_none_or(|byte| !word_byte(*byte))
    })
}

fn selected(surface: Surface, line: &str) -> bool {
    match surface {
        Surface::Prose => true,
        Surface::RustComments => line.contains("//"),
        Surface::JsonDescription => line.trim_start().starts_with("\"description\""),
        Surface::CargoDescription => {
            let line = line.trim_start();
            line.starts_with("description") || line.starts_with('#')
        }
    }
}

fn explicitly_allowed(file: &str, line: &str) -> bool {
    if line.contains("SwarmForge") {
        return true;
    }
    if file == "README.md" && line.contains(concat!("\"For", "ge\" survives as the verb")) {
        return true;
    }

    let history_file = matches!(
        file,
        "README.md"
            | ".github/workflows/ci.yml"
            | ".github/workflows/release.yml"
            | "crates/brokkr-cli/Cargo.toml"
    );
    history_file && (line.contains("`forge` shim") || line.contains("`{forge}` token"))
}

fn retired_name(line: &str) -> bool {
    contains_word(line, RETIRED_PRODUCT)
        || line.contains(RETIRED_REPOSITORY)
        || contains_word(line, "the forge")
        || contains_word(line, "The forge")
        || RETIRED_CRATES.iter().any(|name| line.contains(name))
}

fn offenses_in(file: &str, contents: &str, surface: Surface) -> Vec<Offense> {
    contents
        .lines()
        .enumerate()
        .filter(|(_, line)| selected(surface, line))
        .filter(|(_, line)| retired_name(line) && !explicitly_allowed(file, line))
        .map(|(index, line)| Offense {
            file: file.to_string(),
            line: index + 1,
            text: line.trim().to_string(),
        })
        .collect()
}

fn render(offenses: &[Offense]) -> String {
    offenses
        .iter()
        .map(|offense| format!("{}:{}: {}", offense.file, offense.line, offense.text))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn allowed_history_mechanisms_and_verbs_pass() {
    let prose = concat!(
        "Brokkr forges each slice.\n",
        "A seat can forge a ruling line.\n",
        "The forged citation is left as the forging left it.\n",
        "The self-forge loop is bounded.\n",
        "Wire: forge-driver/v1; state: .forge/forge.db; ref: refs/forge/.\n",
        "SwarmForge is acknowledged.\n",
    );
    assert_eq!(offenses_in("README.md", prose, Surface::Prose), []);
    assert_eq!(
        offenses_in(
            "README.md",
            concat!(
                "**\"For",
                "ge\" survives as the verb.**\nThe `forge` shim is retired.\n"
            ),
            Surface::Prose,
        ),
        []
    );
    assert_eq!(
        offenses_in(
            ".github/workflows/ci.yml",
            "# The `{forge}` token remains a one-release fallback.\n",
            Surface::Prose,
        ),
        []
    );
}

#[test]
fn retired_names_fail_with_file_and_line() {
    let prose = concat!(
        "Brokkr is current.\n",
        "For",
        "ge was left on a living surface.\n",
        "the-",
        "forge is also retired.\n",
        "the forge is still a product noun here.\n",
        "The forge opens a sentence as the same noun.\n",
        "Dependencies: forge-core and forge_runtime.\n",
    );
    let offenses = offenses_in("docs/guides/example.md", prose, Surface::Prose);
    assert_eq!(offenses.len(), 5, "{}", render(&offenses));
    assert_eq!(offenses[0].line, 2);
    assert_eq!(offenses[4].line, 6);
    let failure = render(&offenses);
    assert!(failure.contains("docs/guides/example.md:2:"), "{failure}");
    assert!(failure.contains("docs/guides/example.md:6:"), "{failure}");

    let rust = concat!("let answer = 42; /", "/ For", "ge is retired here.\n");
    let offenses = offenses_in("crates/example/src/lib.rs", rust, Surface::RustComments);
    assert_eq!(offenses.len(), 1, "{}", render(&offenses));
    assert_eq!(offenses[0].line, 1);
}

#[test]
fn living_surfaces_have_no_retired_product_or_crate_names() {
    let root = workspace();
    let surfaces = living_surfaces(&root);
    assert!(
        surfaces.len() > 100,
        "living-surface walk is suspiciously small"
    );

    let mut offenses = Vec::new();
    for (path, surface) in surfaces {
        let relative = path.strip_prefix(&root).expect("workspace-relative path");
        let relative = relative.to_string_lossy().replace('\\', "/");
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        offenses.extend(offenses_in(&relative, &contents, surface));
    }

    assert!(
        offenses.is_empty(),
        "retired forge-to-Brokkr names remain:\n{}",
        render(&offenses)
    );
}
