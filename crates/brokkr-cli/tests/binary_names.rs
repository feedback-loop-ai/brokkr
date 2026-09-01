//! Decision 0019 rulings 2 and 9 through the binaries and the manifests
//! themselves: there is one binary and it is `brokkr`, the one-release
//! `forge` shim is gone, no crate in the workspace is named `forge-*`
//! any more — and the standing guard stays: no user-facing CLI string
//! names the binary `forge`.

use std::path::PathBuf;
use std::process::{Command, Output};

fn brokkr(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// This file lives at `crates/brokkr-cli/tests/`.
fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// The one bin builds, runs, and names itself. `CARGO_BIN_EXE_*`
/// resolves the platform's own file name, so this holds on Windows
/// without naming `.exe` anywhere.
#[test]
fn the_one_binary_is_brokkr_and_names_itself_that() {
    for args in [["--version"], ["--help"]] {
        let run = brokkr(&args);
        assert_eq!(run.status.code(), Some(0), "{}", text(&run.stderr));
    }
    let usage = text(&brokkr(&["--help"]).stdout);
    assert!(usage.contains("Usage: brokkr"), "{usage}");
    assert!(text(&brokkr(&["--version"]).stdout).starts_with("brokkr "));

    // A command with real stdout: the machine-readable surface says
    // nothing about names at all, and no shim writes to stderr in front
    // of it any more.
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let run = brokkr(&["runs", "--json", "--db", db.to_str().unwrap()]);
    assert!(!text(&run.stdout).contains("notice:"), "{run:?}");
    assert!(!text(&run.stderr).contains("one more release"), "{run:?}");
}

/// Mechanism keeps its plain names (ruling 3): the state directory, the
/// database, the git ref namespace and the wire protocols are not the
/// binary's name and do not move in this slice.
const MECHANISM: [&str; 5] = [
    ".forge/",
    "forge.db",
    "refs/forge/",
    "forge-driver/",
    "forge-dispatch/",
];

/// Mechanism struck out, any surviving `forge` is the binary naming
/// itself by the old name. Returns the offending lines, so a failure
/// reads as the text an operator would have seen.
fn binary_named_forge(printed: &str) -> Vec<String> {
    let mut stripped = printed.to_string();
    for token in MECHANISM {
        stripped = stripped.replace(token, " ");
    }
    stripped
        .lines()
        .filter(|line| line.contains("forge"))
        .map(str::to_string)
        .collect()
}

/// The whole help surface — the top level and every subcommand — names
/// the binary `brokkr` and nothing else.
#[test]
fn no_user_facing_help_text_names_the_binary_forge() {
    let top = text(&brokkr(&["--help"]).stdout);
    assert_eq!(binary_named_forge(&top), Vec::<String>::new());

    let commands: Vec<String> = top
        .lines()
        .skip_while(|line| !line.starts_with("Commands:"))
        .skip(1)
        .take_while(|line| !line.trim().is_empty())
        .filter(|line| line.starts_with("  ") && !line.starts_with("   "))
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| *name != "help")
        .map(str::to_string)
        .collect();
    assert!(commands.len() > 20, "{commands:?}");

    for command in &commands {
        let help = text(&brokkr(&[command, "--help"]).stdout);
        assert!(help.contains(&format!("Usage: brokkr {command}")), "{help}");
        assert_eq!(binary_named_forge(&help), Vec::<String>::new(), "{command}");
    }
}

/// Every `Cargo.toml` in the tree, walked rather than listed, so a crate
/// added later cannot reintroduce the old name unnoticed.
fn manifests(dir: &std::path::Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            // Build output and dot-directories (`.git`, `.forge`) carry
            // no manifest this workspace owns.
            let skip = path
                .file_name()
                .is_some_and(|name| name == "target" || name.to_string_lossy().starts_with('.'));
            if skip {
                continue;
            }
            manifests(&path, found);
        } else if path.file_name().is_some_and(|name| name == "Cargo.toml") {
            found.push(path);
        }
    }
}

/// Ruling 2 as a standing condition: no `forge-*` crate name survives in
/// any manifest, and the CLI declares exactly one binary.
#[test]
fn no_manifest_names_a_forge_crate_and_the_cli_declares_one_binary() {
    let root = workspace();
    let mut found = Vec::new();
    manifests(&root, &mut found);
    assert!(found.len() >= 8, "{found:?}");
    for path in &found {
        let manifest = std::fs::read_to_string(path).expect("readable manifest");
        for line in manifest.lines() {
            // A `description` legitimately names a frozen wire protocol
            // (`forge-driver/v1`): protocol strings are archaeology, not
            // branding (ruling 3). Everything else in a manifest is a
            // name, a path or a dependency key.
            if line.trim_start().starts_with("description") {
                continue;
            }
            assert!(
                !line.contains("forge-") && !line.contains("forge_"),
                "{}: {line}",
                path.display()
            );
        }
    }

    let cli = std::fs::read_to_string(root.join("crates").join("brokkr-cli").join("Cargo.toml"))
        .expect("the cli manifest");
    assert_eq!(cli.matches("[[bin]]").count(), 1, "{cli}");
    assert!(cli.contains("name = \"brokkr\""), "{cli}");
    assert!(!cli.contains("name = \"forge\""), "{cli}");
}
