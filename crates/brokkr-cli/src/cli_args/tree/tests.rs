use clap::{CommandFactory, FromArgMatches, Parser};

use crate::{Cli, Cmd};

/// The verbs `command` lists in its help: clap's `help` and every hidden
/// verb are left out.
fn listed(command: &clap::Command) -> Vec<&str> {
    command
        .get_subcommands()
        .filter(|verb| !verb.is_hide_set() && verb.get_name() != "help")
        .map(clap::Command::get_name)
        .collect()
}

#[test]
fn the_help_lists_twelve_roots_and_each_family_its_verbs() {
    let tree = Cli::command();
    // Decision 0113's twelve, and decision 0115's `completions` beside
    // `doctor`.
    assert_eq!(
        listed(&tree),
        [
            "init",
            "run",
            "resume",
            "watch",
            "tui",
            "doctor",
            "completions",
            "runs",
            "journal",
            "library",
            "queue",
            "stats",
            "system"
        ]
    );
    let family = |name: &str| listed(tree.find_subcommand(name).unwrap()).join(" ");
    assert_eq!(
        family("runs"),
        "list inspect seats transcript compare replay rerun conclude operator muninn"
    );
    assert_eq!(
        family("journal"),
        "export import verify backup restore anchor keep-refs evidence"
    );
    assert_eq!(family("library"), "recipes agents compile");
    assert_eq!(
        family("queue"),
        "add list judge move hold release repin drop dispatch sweep"
    );
    assert_eq!(family("stats"), "analytics graph costs ledger");
    assert_eq!(family("system"), "realms secrets probe ui");
}

#[test]
fn the_engines_verbs_stay_at_the_root_hidden() {
    let tree = Cli::command();
    for name in ["driver", "hands", "bridge", "broker", "fake-driver"] {
        let verb = tree.find_subcommand(name).unwrap();
        assert!(verb.is_hide_set(), "{name} is hidden");
    }
    let driven = Cli::try_parse_from(["brokkr", "driver", "exec", "--", "true"]).unwrap();
    assert!(matches!(driven.command, Cmd::Driver(args) if args.kind == "exec"));
    let line = ["brokkr", "bridge", "--run", "r", "--looper-url", "http://x"];
    let bridged = Cli::try_parse_from(line).unwrap();
    assert!(matches!(bridged.command, Cmd::Bridge(args) if args.run == "r"));
}

#[test]
fn a_bare_runs_lists_runs_as_runs_list_does() {
    for line in [
        &["brokkr", "runs", "--json", "--limit", "2"][..],
        &["brokkr", "runs", "list", "--json", "--limit", "2"],
    ] {
        let parsed = Cli::try_parse_from(line).unwrap();
        assert!(
            matches!(parsed.command, Cmd::Runs(args) if args.json && args.limit == Some(2)),
            "{line:?}"
        );
    }
    // A flag of the bare listing is not also taken beside a verb.
    let both = Cli::try_parse_from(["brokkr", "runs", "--json", "inspect", "--run", "r"]);
    assert_eq!(
        both.err().map(|error| error.kind()),
        Some(clap::error::ErrorKind::ArgumentConflict)
    );
}

#[test]
fn a_family_verb_reaches_the_command_its_flat_one_did() {
    let parsed = |line: &[&str]| Cli::try_parse_from(line).unwrap().command;
    assert!(matches!(
        parsed(&["brokkr", "journal", "verify", "x.ndjson"]),
        Cmd::VerifyRun(args) if args.file.to_str() == Some("x.ndjson")
    ));
    assert!(matches!(
        parsed(&["brokkr", "runs", "inspect", "--run", "r", "--json"]),
        Cmd::Inspect(args) if args.run == "r" && args.json
    ));
    assert!(matches!(
        parsed(&["brokkr", "queue", "list"]),
        Cmd::Queue {
            command: crate::QueueCmd::List(_)
        }
    ));
}

#[test]
fn an_update_from_matches_reads_the_tree_again() {
    let matches = Cli::command()
        .try_get_matches_from(["brokkr", "system", "realms", "--json"])
        .unwrap();
    let mut cli = Cli::try_parse_from(["brokkr", "doctor"]).unwrap();
    cli.update_from_arg_matches(&matches).unwrap();
    assert!(matches!(cli.command, Cmd::Realms(args) if args.json));
}
