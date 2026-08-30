//! Decision 0019 ruling 9 through the binaries themselves: the shipped
//! binary is `brokkr`, the old `forge` name survives one more release as
//! a shim, and the shim's whole difference is one plain line on stderr —
//! stdout is byte-identical, so pipes and JSON consumers never see it.
//! The last test is the standing guard: no user-facing CLI string names
//! the binary `forge` any more.

use std::process::{Command, Output};

fn brokkr(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(args)
        .output()
        .unwrap()
}

fn shim(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Both bins are built and both run the same CLI: same version, same
/// usage, same exit code. `CARGO_BIN_EXE_*` resolves the platform's own
/// file name, so this holds on Windows without naming `.exe` anywhere.
#[test]
fn both_bins_exist_and_run_the_same_cli() {
    for args in [["--version"], ["--help"]] {
        let new = brokkr(&args);
        let old = shim(&args);
        assert_eq!(new.status.code(), Some(0), "{}", text(&new.stderr));
        assert_eq!(old.status.code(), Some(0), "{}", text(&old.stderr));
        assert_eq!(text(&new.stdout), text(&old.stdout));
    }

    // The name clap prints is the new one under both bins.
    let usage = text(&brokkr(&["--help"]).stdout);
    assert!(usage.contains("Usage: brokkr"), "{usage}");
    assert!(text(&brokkr(&["--version"]).stdout).starts_with("brokkr "));
}

/// The shim's one notice: a single plain line, on stderr, before the
/// command proceeds — and nothing added to stdout, which is what a pipe
/// or a `--json` consumer reads.
#[test]
fn the_shim_writes_one_plain_line_to_stderr_and_nothing_to_stdout() {
    assert!(!forge_cli::SHIM_NOTICE.contains('\n'), "one line only");
    assert!(forge_cli::SHIM_NOTICE.contains("brokkr"));
    assert!(forge_cli::SHIM_NOTICE.contains("forge"));
    assert!(forge_cli::SHIM_NOTICE.contains("one more release"));

    // A command with real stdout: the machine-readable surface is the
    // one that must not move.
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let db = db.to_str().unwrap();
    let new = brokkr(&["runs", "--json", "--db", db]);
    let old = shim(&["runs", "--json", "--db", db]);
    assert_eq!(new.status.code(), old.status.code());
    assert_eq!(text(&new.stdout), text(&old.stdout));
    assert!(
        !text(&old.stdout).contains("brokkr"),
        "notice reached stdout"
    );

    // Exactly one notice, and it is the first thing written.
    let stderr = text(&old.stderr);
    assert_eq!(
        stderr.matches(forge_cli::SHIM_NOTICE).count(),
        1,
        "{stderr}"
    );
    assert_eq!(
        stderr.lines().next(),
        Some(forge_cli::SHIM_NOTICE),
        "{stderr}"
    );

    // Everything after it is what the primary binary already writes: the
    // shim adds one line and changes nothing.
    let rest: Vec<&str> = stderr.lines().skip(1).collect();
    assert_eq!(rest, text(&new.stderr).lines().collect::<Vec<&str>>());
}

/// Mechanism keeps its plain names (ruling 3): the state directory, the
/// database, the git ref namespace, the wire protocols and the `{forge}`
/// token are not the binary's name and do not move in this slice.
const MECHANISM: [&str; 6] = [
    ".forge/",
    "forge.db",
    "refs/forge/",
    "forge-driver/",
    "forge-dispatch/",
    "{forge}",
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

    // The shim's notice is the sole sanctioned exception, and it is not
    // help text: it is one line on stderr.
    assert_eq!(binary_named_forge(forge_cli::SHIM_NOTICE).len(), 1);
}
