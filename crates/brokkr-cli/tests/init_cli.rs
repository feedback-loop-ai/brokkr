//! `brokkr init --cli <claude|codex|dsh>` (#717): the agent CLI the
//! scaffold is hired from, named in place of the first of claude, codex
//! and dsh on PATH. A host whose CLIs share one directory, as volta
//! installs codex and dsh beside npm, could otherwise never be scaffolded
//! for dsh. What follows the CLI follows the named one — the seats'
//! hiring, the adapters written and the dsh key check — and what follows
//! the stack does not move. Without `--cli` nothing changes, which every
//! other `init` test holds.

use std::path::Path;

use serde_json::Value;

use crate::brokkr_folder::bins_holding;
use crate::init_stacks::{brokkr_with, repository_of, TOOL_HOMES};

/// `brokkr init . <extra>` in `repo` under `PATH` = `bins` alone, with
/// `HOME` at `home` and no tool's home variable: its exit code and stderr.
fn init_under(repo: &Path, bins: &Path, home: &Path, extra: &[&str]) -> (Option<i32>, String) {
    let args: Vec<&str> = ["init", "."].iter().chain(extra).copied().collect();
    let (code, _, stderr) = brokkr_with(&args, repo, |command| {
        command.env("PATH", bins).env("HOME", home);
        command.env("XDG_CONFIG_HOME", home.join(".config"));
        for name in TOOL_HOMES {
            command.env_remove(name);
        }
    });
    (code, stderr)
}

/// The adapters a scaffold wrote, by file name, sorted.
fn adapters(repo: &Path) -> Vec<String> {
    let dir = repo.join(".brokkr/adapters");
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Each model office's chain, as the scaffold's agents hire it.
fn chains(repo: &Path) -> Vec<(&'static str, Value)> {
    ["intake", "implementer", "reviewer"]
        .into_iter()
        .map(|agent| {
            let file = repo.join(format!(".brokkr/agents/{agent}.json"));
            let definition: Value =
                serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
            (agent, definition["models"].clone())
        })
        .collect()
}

/// A dsh scaffold's chains: intake and implement on dsh, the gate on
/// claude, which dsh cannot hold (decision 0021).
fn dsh_chains() -> Vec<(&'static str, Value)> {
    vec![
        ("intake", serde_json::json!(["flash", "pro"])),
        ("implementer", serde_json::json!(["pro", "flash"])),
        ("reviewer", serde_json::json!(["fable", "opus"])),
    ]
}

/// What each of the dsh key check's lines in `stderr` is about: the line
/// up to its first colon, ahead of the detail that names this host's paths.
fn key_lines(stderr: &str) -> Vec<&str> {
    let check = ["dsh key ", "dsh home ", "dsh workdir:"];
    let lines = stderr.lines();
    let lines = lines.filter(|line| check.iter().any(|lead| line.starts_with(lead)));
    lines.map(|line| line.split(':').next().unwrap()).collect()
}

/// claude, codex and dsh all on PATH, whose order chooses claude:
/// `--cli dsh` writes a dsh scaffold, says `--cli` chose it, and ends with
/// the dsh key check.
#[test]
fn a_named_dsh_beside_claude_and_codex_scaffolds_for_dsh() {
    let repo = crate::brokkr_folder::repository();
    let aside = tempfile::tempdir().unwrap();
    let bins = bins_holding(aside.path(), &["claude", "codex", "dsh"]);
    let (code, stderr) = init_under(repo.path(), &bins, aside.path(), &["--cli", "dsh"]);
    assert_eq!(code, Some(0), "{stderr}");
    let said = "Scaffolded for `dsh`, as --cli named.";
    assert_eq!(
        stderr.lines().filter(|line| *line == said).count(),
        1,
        "{stderr}"
    );
    assert!(
        stderr.contains("and `agents/reviewer.json` is hired from claude.\n"),
        "{stderr}"
    );
    assert_eq!(
        adapters(repo.path()),
        ["claude.json", "dsh.json", "exec.json"]
    );
    assert_eq!(chains(repo.path()), dsh_chains());
    let routes = ["dsh key deepseek-official", "dsh home deepseek-official"];
    assert_eq!(key_lines(&stderr), routes, "{stderr}");
}

/// `--cli dsh` where PATH holds no dsh: the scaffold is written for dsh
/// all the same, and its note names the binary to install.
#[test]
fn a_named_dsh_absent_from_path_is_scaffolded_for_and_named_to_install() {
    let repo = crate::brokkr_folder::repository();
    let aside = tempfile::tempdir().unwrap();
    let bins = bins_holding(aside.path(), &["claude"]);
    let (code, stderr) = init_under(repo.path(), &bins, aside.path(), &["--cli", "dsh"]);
    assert_eq!(code, Some(0), "{stderr}");
    let said = "Scaffolded for `dsh`, as --cli named, but no `dsh` was found on PATH. \
                Install `dsh` before the first run.";
    assert_eq!(
        stderr.lines().filter(|line| *line == said).count(),
        1,
        "{stderr}"
    );
    assert_eq!(
        adapters(repo.path()),
        ["claude.json", "dsh.json", "exec.json"]
    );
    assert_eq!(chains(repo.path()), dsh_chains());
}

/// A CLI `init` does not scaffold for is clap's refusal, and nothing is
/// written.
#[test]
fn an_unknown_cli_is_refused_by_clap() {
    let repo = crate::brokkr_folder::repository();
    let aside = tempfile::tempdir().unwrap();
    let bins = bins_holding(aside.path(), &["claude"]);
    let (code, stderr) = init_under(repo.path(), &bins, aside.path(), &["--cli", "gemini"]);
    let refused = "error: invalid value 'gemini' for '--cli <CLI>'\n  \
                   [possible values: claude, codex, dsh]\n\n\
                   For more information, try '--help'.\n";
    assert_eq!((code, stderr.as_str()), (Some(2), refused));
    assert!(!repo.path().join(".brokkr").exists());
}

/// The acceptance shape of decision 0091's slice 3d: an npm folder on a
/// host whose volta home holds npm, node, codex and dsh in one `bin`.
/// `--cli dsh` scaffolds for dsh and still binds the volta home for npm's
/// verify; without `--cli`, PATH order scaffolds for codex.
#[test]
fn a_volta_bin_holding_codex_and_dsh_scaffolds_for_the_named_one() {
    let named: (&[&str], &[&str]) = (&["--cli", "dsh"], &["claude.json", "dsh.json", "exec.json"]);
    let ordered: (&[&str], &[&str]) = (&[], &["codex.json", "exec.json"]);
    for (extra, written) in [named, ordered] {
        let repo = repository_of("node-npm");
        let aside = tempfile::tempdir().unwrap();
        let home = aside.path().canonicalize().unwrap().join("home");
        let bin = home.join(".volta/bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(home.join(".volta/tools/image/node/20.11.0/bin")).unwrap();
        for file in ["volta-shim", "codex", "dsh"] {
            std::fs::write(bin.join(file), "").unwrap();
            let runs = std::os::unix::fs::PermissionsExt::from_mode(0o755);
            std::fs::set_permissions(bin.join(file), runs).unwrap();
        }
        for shim in ["npm", "node"] {
            std::os::unix::fs::symlink("volta-shim", bin.join(shim)).unwrap();
        }
        crate::brokkr_folder::plant_git(&bin);
        let (code, stderr) = init_under(repo.path(), &bin, &home, extra);
        assert_eq!(code, Some(0), "{stderr}");
        assert_eq!(adapters(repo.path()), written, "{extra:?}: {stderr}");
        let bundle = std::fs::read_to_string(repo.path().join(".brokkr/bundle.json")).unwrap();
        let bundle: Value = serde_json::from_str(&bundle).unwrap();
        assert_eq!(
            bundle["seats"]["verify"]["hands"]["binds"],
            serde_json::json!([{"path": "~/.volta", "mode": "ro"}]),
            "{extra:?}"
        );
    }
}
