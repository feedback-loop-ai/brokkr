//! Which agent CLI a scaffold's seats are hired from (#717): the one
//! `--cli` names, else the first of claude, codex and dsh on PATH. A host
//! whose CLIs share one directory, as volta installs codex and dsh beside
//! npm, could otherwise never be scaffolded for any but the first. File
//! presence is the whole of the evidence, as it is for the stack: nothing
//! is executed, and a named CLI that PATH lacks is scaffolded for all the
//! same, its note naming the binary to install.

use std::ffi::OsStr;

use super::{Cli, CLIS};

/// What a scaffold with no agent CLI on PATH and none named says.
const NONE_FOUND: &str = "No agent CLI (claude, codex or dsh) was found on PATH, so this \
     scaffold is written for `claude`. Install one before the first \
     run; to scaffold for codex or dsh instead, re-run `init` once it \
     is installed.";

/// The CLI the seats are hired from, and the notes that say which rule
/// chose it and what follows from it for the tool grants and the gate.
pub(super) fn chosen(path: &OsStr, named: Option<Cli>) -> (Cli, Vec<String>) {
    let look = crate::boundary::on_path(path);
    let on_path = |cli: Cli| look(cli.binary()).is_some();
    let (cli, said) = match named {
        Some(cli) if on_path(cli) => (
            cli,
            format!("Scaffolded for `{}`, as --cli named.", cli.binary()),
        ),
        Some(cli) => (
            cli,
            format!(
                "Scaffolded for `{0}`, as --cli named, but no `{0}` was found on \
                 PATH. Install `{0}` before the first run.",
                cli.binary()
            ),
        ),
        None => match CLIS.into_iter().find(|&cli| on_path(cli)) {
            Some(cli) => (
                cli,
                format!(
                    "Scaffolded for `{}`, the first of claude, codex and dsh found on PATH.",
                    cli.binary()
                ),
            ),
            None => (Cli::Claude, NONE_FOUND.to_string()),
        },
    };
    let mut notes = vec![said];
    if cli != Cli::Claude {
        notes.push(format!(
            "`{}` cannot express a tool name, so no `{}`-hired agent carries a \
             tool allowance; where this README speaks of tool grants, it speaks \
             of the claude-hired seats only.",
            cli.binary(),
            cli.binary()
        ));
    }
    if cli == Cli::Dsh {
        // PATH order reaches dsh only where claude is absent; `--cli dsh`
        // reaches it beside a claude the gate can already run.
        let claude = if on_path(Cli::Claude) {
            "."
        } else {
            ", which is not on PATH — install claude before a run reaches review."
        };
        notes.push(format!(
            "The review gate cannot be held by dsh: its adapter is untrusted and \
             names no judges, and a gate seat judges only on a trusted adapter's \
             judge (decision 0021). So intake and implement are hired from dsh \
             and `agents/reviewer.json` is hired from claude{claude}"
        ));
    }
    (cli, notes)
}
