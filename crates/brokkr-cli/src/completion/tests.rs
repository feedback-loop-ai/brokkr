//! The static scripts (decision 0115 ruling 1): every visible verb of the
//! tree and every flag in each shell's script, no hidden verb and no 0.16
//! alias. The paths are walked from `Cli::command()`, never listed by
//! hand, and each shell is read in the form its generator offers a verb.

use clap::{CommandFactory, Parser, ValueEnum};
use clap_complete::Shell;

use super::{script, CompletionsArgs};
use crate::Cli;

/// Every verb path of the tree, whether a hidden verb is on it, and the
/// long flags of the verbs no hidden verb is on, each with its verb's depth.
#[derive(Default)]
struct Walked {
    visible: Vec<Vec<String>>,
    hidden: Vec<Vec<String>>,
    flags: Vec<(usize, String)>,
}

fn walk(command: &clap::Command, path: &[String], hidden: bool, walked: &mut Walked) {
    for sub in command.get_subcommands() {
        let path = [path, &[sub.get_name().to_string()]].concat();
        let hidden = hidden || sub.is_hide_set();
        if !hidden {
            let longs = sub.get_arguments().filter_map(clap::Arg::get_long);
            walked
                .flags
                .extend(longs.map(|long| (path.len(), long.to_string())));
        }
        walk(sub, &path, hidden, walked);
        match hidden {
            true => walked.hidden.push(path),
            false => walked.visible.push(path),
        }
    }
}

fn walked() -> Walked {
    let mut walked = Walked::default();
    walk(&Cli::command(), &[], false, &mut walked);
    walked
}

fn script_for(shell: Shell) -> String {
    let mut out = Vec::new();
    script(CompletionsArgs { shell }, &mut out);
    String::from_utf8(out).unwrap()
}

/// Whether `text` offers the verb at `path`, in the form `shell`'s
/// generator writes one: bash's dispatch case, zsh's command list, fish's
/// `-a` under its parents' conditions, and the PowerShell and elvish keys.
fn offers(shell: Shell, text: &str, path: &[String]) -> bool {
    let (parents, name) = path.split_at(path.len() - 1);
    let name = &name[0];
    match shell {
        Shell::Bash => {
            let mangled = |part: &String| format!("__subcmd__{}", part.replace('-', "__subcmd__"));
            let parent: String = parents.iter().map(mangled).collect();
            text.contains(&format!("brokkr{parent},{name})"))
        }
        Shell::Zsh => {
            let parent: String = parents.iter().map(|p| format!("__subcmd__{p}")).collect();
            let head = format!("_brokkr{parent}_commands() {{\n");
            let block = text
                .split_once(&head)
                .map(|(_, rest)| rest.split("\n}\n").next());
            block
                .flatten()
                .is_some_and(|block| block.contains(&format!("\n'{name}:")))
        }
        Shell::Fish => {
            let condition = match parents {
                [] => "\"__fish_brokkr_needs_command\"".to_string(),
                [first] => {
                    format!("using_subcommand {first}; and not __fish_seen_subcommand_from ")
                }
                [first, second] => {
                    format!("using_subcommand {first}; and __fish_seen_subcommand_from {second}")
                }
                deeper => panic!("no verb is this deep: {deeper:?}"),
            };
            let offered = format!("-f -a \"{name}\"");
            text.lines()
                .any(|line| line.contains(&condition) && line.contains(&offered))
        }
        Shell::PowerShell => text.contains(&format!("'brokkr;{}' {{", path.join(";"))),
        Shell::Elvish => text.contains(&format!("&'brokkr;{}'= {{", path.join(";"))),
        other => panic!("a shell this test does not read: {other}"),
    }
}

#[test]
fn every_shell_offers_every_visible_verb_and_flag_and_no_hidden_verb_or_alias() {
    let walked = walked();
    assert!(
        !walked.hidden.is_empty(),
        "the tree hides the engine's verbs"
    );
    let aliases: Vec<Vec<String>> = crate::cli_args::old_spellings()
        .map(|old| vec![old.to_string()])
        .collect();
    assert_eq!(aliases.len(), 30, "0.16 holds 30 flat spellings");
    for shell in Shell::value_variants() {
        let text = script_for(*shell);
        for path in &walked.visible {
            assert!(offers(*shell, &text, path), "{shell} lacks {path:?}");
        }
        for path in walked.hidden.iter().chain(&aliases) {
            assert!(!offers(*shell, &text, path), "{shell} offers {path:?}");
        }
        // clap_complete's fish generator writes the flags of the verbs two
        // deep and no deeper; the live completer offers the rest.
        for (depth, flag) in &walked.flags {
            let written = match shell {
                Shell::Fish if *depth > 2 => continue,
                Shell::Fish => format!("-l {flag}"),
                _ => format!("--{flag}"),
            };
            assert!(text.contains(&written), "{shell} lacks --{flag}");
        }
    }
}

#[test]
fn a_shell_clap_does_not_know_is_refused_by_its_value_check() {
    let refused = Cli::try_parse_from(["brokkr", "completions", "nushell"])
        .map(|_| ())
        .unwrap_err();
    assert_eq!(refused.kind(), clap::error::ErrorKind::InvalidValue);
}
