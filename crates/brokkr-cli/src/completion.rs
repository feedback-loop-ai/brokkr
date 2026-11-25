//! Shell completion (decision 0119). `brokkr completions <shell>` prints
//! the script clap_complete writes for the command tree (ruling 1), so a
//! shell offers every visible verb, flag and fixed value set. The tree
//! holds no 0.16 alias, which is a respelling made before the tree parses
//! (decision 0113 ruling 4). The live completer, which `COMPLETE` asks
//! for, is [`live`]'s.

use std::io::Write;
use std::process::ExitCode;

use clap::builder::Resettable;
use clap::{Command, CommandFactory};

use crate::exit::Exit;

mod live;
pub(crate) use live::answer_if_asked;

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct CompletionsArgs {
    /// The shell the script is written for.
    #[arg(value_enum)]
    pub(crate) shell: clap_complete::Shell,
}

/// `brokkr completions`: the script for `shell`, on `out`.
pub(crate) fn script(CompletionsArgs { shell }: CompletionsArgs, out: &mut dyn Write) -> ExitCode {
    let mut tree = offered(&crate::Cli::command());
    clap_complete::generate(shell, &mut tree, "brokkr", out);
    Exit::Completed.into()
}

/// `command` as a shell may offer it: every hidden subcommand left out, at
/// every depth. clap_complete 4.6.9's generators write a hidden
/// subcommand into the script, and clap cannot remove one from a command,
/// so each level is built again from what a script reads of it: its name,
/// texts, version, arguments, groups, visible aliases, the two settings
/// that say whether a verb's flags and its subcommands go together, and
/// its visible subcommands. clap is built without its `string` feature,
/// so a name it takes is `'static`: each is leaked once, in a process
/// that prints one script and exits.
fn offered(command: &Command) -> Command {
    let visible = command.get_subcommands().filter(|sub| !sub.is_hide_set());
    Command::new(leaked(command.get_name()))
        .about(kept(command.get_about().cloned()))
        .long_about(kept(command.get_long_about().cloned()))
        .version(command.get_version().map(leaked))
        .bin_name(kept(command.get_bin_name().map(str::to_string)))
        .args(command.get_arguments().cloned())
        .groups(command.get_groups().cloned())
        .visible_aliases(command.get_visible_aliases().map(leaked))
        .args_conflicts_with_subcommands(command.is_args_conflicts_with_subcommands_set())
        .subcommand_required(command.is_subcommand_required_set())
        .subcommands(visible.map(offered))
}

/// A setting the parsed command holds, or none: what the rebuilt one is
/// given, so an unset one stays unset.
fn kept<T>(value: Option<T>) -> Resettable<T> {
    value.map_or(Resettable::Reset, Resettable::Value)
}

fn leaked(text: &str) -> &'static str {
    Box::leak(text.to_string().into_boxed_str())
}

#[cfg(test)]
mod tests;
