use std::ffi::OsString;

use clap::{CommandFactory, Parser};

use super::{respelled, Moved, MOVED};
use crate::Cli;

/// Every root main had before the tree (5864795d1), the hidden two among
/// them; decision 0113 places each.
const MAIN_ROOTS: [&str; 43] = [
    "init",
    "costs",
    "ledger",
    "anchor",
    "keep-refs",
    "ui",
    "tui",
    "doctor",
    "compile",
    "run",
    "resume",
    "rerun",
    "compare",
    "recipes",
    "agents",
    "muninn",
    "secrets",
    "sweep",
    "dispatch",
    "conclude",
    "operator",
    "queue",
    "inspect",
    "transcript",
    "seats",
    "watch",
    "replay",
    "export",
    "evidence",
    "backup",
    "restore",
    "import",
    "verify-run",
    "bridge",
    "runs",
    "realms",
    "analytics",
    "graph",
    "driver",
    "hands",
    "probe",
    "broker",
    "fake-driver",
];

fn words(argv: &[OsString]) -> Vec<&str> {
    argv.iter().map(|word| word.to_str().unwrap()).collect()
}

#[test]
fn an_old_spelling_takes_its_new_path_and_says_so_once() {
    let (argv, moved) = respelled(["brokkr", "verify-run", "x.ndjson", "--", "inspect"]);
    assert_eq!(
        words(&argv),
        ["brokkr", "journal", "verify", "x.ndjson", "--", "inspect"]
    );
    assert_eq!(
        moved.unwrap().to_string(),
        "brokkr: 'verify-run' is now 'brokkr journal verify'; the old spelling goes in 0.17"
    );
    let (argv, moved) = respelled(["brokkr", "inspect", "--run", "r"]);
    assert_eq!(words(&argv), ["brokkr", "runs", "inspect", "--run", "r"]);
    assert_eq!(
        moved,
        Some(Moved {
            old: "inspect",
            new: &["runs", "inspect"]
        })
    );
}

#[test]
fn help_with_an_old_spelling_prints_its_new_verbs_help() {
    let (argv, moved) = respelled(["brokkr", "help", "inspect"]);
    assert_eq!(words(&argv), ["brokkr", "help", "runs", "inspect"]);
    assert_eq!(
        moved.unwrap().to_string(),
        "brokkr: 'inspect' is now 'brokkr runs inspect'; the old spelling goes in 0.17"
    );
    let help = |argv: &[&str]| {
        let error = Cli::try_parse_from(argv).err().unwrap();
        (error.kind(), error.render().to_string())
    };
    let typed = help(&words(&argv));
    assert_eq!(typed, help(&["brokkr", "runs", "inspect", "--help"]));
    assert_eq!(typed.0, clap::error::ErrorKind::DisplayHelp);
    // Only the word after `help` is the verb's place.
    let (argv, moved) = respelled(["brokkr", "help", "runs", "inspect"]);
    assert_eq!(
        (words(&argv), moved),
        (vec!["brokkr", "help", "runs", "inspect"], None)
    );
}

#[test]
fn a_root_that_did_not_move_is_left_as_typed() {
    for verb in [
        "init",
        "run",
        "resume",
        "watch",
        "tui",
        "doctor",
        "runs",
        "queue",
        "driver",
        "hands",
        "bridge",
        "broker",
        "fake-driver",
        "journal",
    ] {
        let (argv, moved) = respelled(["brokkr", verb, "inspect"]);
        assert_eq!(
            (words(&argv), moved),
            (vec!["brokkr", verb, "inspect"], None)
        );
    }
    // A moved name anywhere but the verb's place is an argument.
    let (argv, moved) = respelled(["brokkr", "runs", "inspect"]);
    assert_eq!(
        (words(&argv), moved),
        (vec!["brokkr", "runs", "inspect"], None)
    );
    assert_eq!(
        respelled(["brokkr"]),
        (vec![OsString::from("brokkr")], None)
    );
}

#[test]
fn every_root_main_had_is_placed_once() {
    let tree = Cli::command();
    let roots: Vec<&str> = tree
        .get_subcommands()
        .map(clap::Command::get_name)
        .filter(|name| *name != "help")
        .collect();
    for (old, new) in MOVED {
        assert!(!roots.contains(&old), "{old} is no longer a root");
        let verb = new
            .iter()
            .try_fold(&tree, |at, name| at.find_subcommand(name))
            .unwrap_or_else(|| panic!("{old}'s new path {new:?} is a verb"));
        assert!(!verb.is_hide_set(), "{new:?} is listed in its family");
    }
    let mut placed: Vec<&str> = MOVED.iter().map(|(old, _)| *old).collect();
    placed.extend(
        roots
            .iter()
            .filter(|root| !["journal", "library", "stats", "system"].contains(root)),
    );
    placed.sort_unstable();
    let mut main = MAIN_ROOTS.to_vec();
    main.sort_unstable();
    assert_eq!(placed, main);
}

#[test]
fn every_old_spelling_parses_as_its_new_one() {
    for (old, new) in MOVED {
        let help = |argv: Vec<OsString>| {
            Cli::try_parse_from(argv)
                .err()
                .map(|help| (help.kind(), help.render().to_string()))
        };
        let typed = help(respelled(["brokkr", old, "--help"]).0);
        let path = std::iter::once("brokkr").chain(new.iter().copied());
        let spelled = help(path.chain(["--help"]).map(OsString::from).collect());
        assert_eq!(typed, spelled, "{old}");
        assert_eq!(
            typed.map(|(kind, _)| kind),
            Some(clap::error::ErrorKind::DisplayHelp)
        );
    }
}
