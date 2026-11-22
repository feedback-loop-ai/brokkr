//! The live completer (decision 0115 rulings 2-8): clap_complete's
//! env-driven protocol, answered before anything else in `main()`, with
//! each value an operator names offered from the journal, the libraries
//! or the map. Every read is read-only and leaves no file, reads one
//! bounded page, waits at most [`PATIENCE`] on a peer, and fails silent:
//! nothing a request does reaches the terminal the operator is typing in.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::{Command, CommandFactory};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use clap_complete::env::CompleteEnv;

use brokkr_store::Store;

/// The variable a shell's registered completer sets (ruling 2).
const VAR: &str = "COMPLETE";

/// The most candidates one read lists (ruling 4).
const PAGE: u32 = 50;

/// The most seat rows a phase or seat read takes from one run (ruling 4):
/// a run's participants in the order they opened, never its events, so
/// the read is bounded however many events the run holds.
const SEATS: u32 = 500;

/// What a read spends on a peer's lock, at most (ruling 8).
const PATIENCE: Duration = Duration::from_millis(50);

/// Answer a completion request and exit, when `COMPLETE` asks for one;
/// otherwise return having read that variable and nothing else.
pub(crate) fn answer_if_asked() {
    if std::env::var_os(VAR).is_none() {
        return;
    }
    // A panic's message would land on the operator's prompt (ruling 7).
    std::panic::set_hook(Box::new(|_| {}));
    let here = std::env::current_dir().ok();
    let answered =
        CompleteEnv::with_factory(completing).try_complete(std::env::args_os(), here.as_deref());
    let code = match answered {
        // `COMPLETE` was empty or `0`: no request, so the command runs.
        Ok(false) => return drop(std::panic::take_hook()),
        Ok(true) => 0,
        Err(_) => 1,
    };
    let _ = std::io::stdout().flush();
    std::process::exit(code)
}

/// The values an argument is completed with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Values {
    Runs,
    Phases,
    Seats,
    Recipes,
    Agents,
    Realms,
    Entries,
}

/// The ids of the arguments that name a run: `--run`, `--by-run` and
/// `runs compare`'s two.
const RUN_IDS: [&str; 4] = ["run", "by_run", "run_a", "run_b"];

/// Which values the argument `id` of the verb `verb`, under `parent`,
/// takes: one table, read by the tree's own ids (decision 0071 ruling 5).
fn values_of(parent: &str, verb: &str, id: &str) -> Option<Values> {
    if RUN_IDS.contains(&id) {
        return Some(Values::Runs);
    }
    match (parent, verb, id) {
        (_, _, "phase") => Some(Values::Phases),
        (_, _, "seat") => Some(Values::Seats),
        (_, _, "recipe") | ("recipes", "show" | "usage", "name") => Some(Values::Recipes),
        ("agents", "show", "name") => Some(Values::Agents),
        (_, _, "realm" | "by_realm") => Some(Values::Realms),
        ("queue", _, "entry") => Some(Values::Entries),
        _ => None,
    }
}

/// The tree the completer answers from: the parsed tree, each argument
/// the table names given its values. Only a completion request builds it.
fn completing() -> Command {
    with_values(crate::Cli::command(), "")
}

fn with_values(command: Command, parent: &str) -> Command {
    let verb = command.get_name().to_string();
    let takes: Vec<(String, Values)> = (command.get_arguments())
        .filter_map(|arg| {
            let id = arg.get_id().as_str();
            values_of(parent, &verb, id).map(|values| (id.to_string(), values))
        })
        .collect();
    let command = takes.into_iter().fold(command, |command, (id, values)| {
        let complete = move |current: &OsStr| candidates(values, current);
        command.mut_arg(id, |arg| arg.add(ArgValueCompleter::new(complete)))
    });
    let visible: Vec<String> = (command.get_subcommands())
        .filter(|sub| !sub.is_hide_set())
        .map(|sub| sub.get_name().to_string())
        .collect();
    visible.into_iter().fold(command, |command, sub| {
        command.mut_subcommand(sub, |sub| with_values(sub, &verb))
    })
}

/// The candidates for `current`, each value that starts with it, with its
/// description; none where the value is not text or the read fails.
fn candidates(values: Values, current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return Vec::new();
    };
    let line = Line::read(std::env::args_os());
    let listed = listed(values, &line).unwrap_or_default();
    listed
        .into_iter()
        .filter(|(value, _)| value.starts_with(current))
        .map(|(value, help)| CompletionCandidate::new(value).help(help.map(Into::into)))
        .collect()
}

/// The flags of the line being completed that choose where its values are
/// read (ruling 6): the words clap_complete hands after its `--`, the
/// whole line, each spelled `--flag value` or `--flag=value`, the last
/// one written winning.
#[derive(Debug, Default, PartialEq, Eq)]
struct Line {
    db: Option<PathBuf>,
    realms: Option<PathBuf>,
    recipes_dir: Option<PathBuf>,
    agents_dir: Option<PathBuf>,
    dir: Option<PathBuf>,
    run: Option<String>,
}

impl Line {
    fn read(args: impl IntoIterator<Item = OsString>) -> Line {
        let words: Vec<OsString> = (args.into_iter())
            .skip_while(|word| word != "--")
            .skip(1)
            .collect();
        let mut line = Line::default();
        for (at, word) in words.iter().enumerate() {
            let Some(word) = word.to_str() else {
                continue;
            };
            let (flag, value) = match word.split_once('=') {
                Some((flag, value)) => (flag, Some(value)),
                None => (word, words.get(at + 1).and_then(|next| next.to_str())),
            };
            line.set(flag, value.filter(|value| !value.is_empty()));
        }
        line
    }

    fn set(&mut self, flag: &str, value: Option<&str>) {
        let path = || value.map(PathBuf::from);
        match flag {
            "--db" => self.db = path(),
            "--realms" => self.realms = path(),
            "--recipes-dir" => self.recipes_dir = path(),
            "--agents-dir" => self.agents_dir = path(),
            "--dir" => self.dir = path(),
            "--run" => self.run = value.map(str::to_string),
            _ => {}
        }
    }
}

/// A value and the description a shell may show beside it.
type Listed = anyhow::Result<Vec<(String, Option<String>)>>;

/// Every value of `values` the line's journal, libraries or map hold.
fn listed(values: Values, line: &Line) -> Listed {
    match values {
        Values::Runs => journal(line, |store| Ok(store.recent_runs(PAGE)?)),
        Values::Phases => named(line, |names| names.phase.clone()),
        Values::Seats => named(line, |names| {
            let seat = names.seat.as_deref()?;
            Some(brokkr_view::participant_label(
                seat,
                names.member.as_deref(),
            ))
        }),
        Values::Recipes => recipes(line),
        Values::Agents => agents(line),
        Values::Realms => realms(line),
        Values::Entries => {
            let waiting = journal(line, |store| Ok(store.queue_waiting(PAGE)?))?;
            let listed = waiting
                .into_iter()
                .map(|(entry, word)| (entry.to_string(), Some(word)));
            Ok(listed.collect())
        }
    }
}

/// `read` on the journal the completed command opens, resolved as it
/// resolves it, opened read-only and left as it was (rulings 3 and 6): a
/// read the journal changed under answers nothing.
fn journal<T>(line: &Line, read: impl FnOnce(&Store) -> anyhow::Result<T>) -> anyhow::Result<T> {
    let invocation =
        crate::Invocation::inspect(Path::new("."), line.realms.clone(), line.db.clone())?;
    Store::read_untouched(&invocation.journal, PATIENCE, read)
}

/// The names `of` reads from the first [`SEATS`] seat rows of the run
/// `--run` names, never its events, the run resolved from at most two of
/// a prefix's matches (ruling 4); none without a `--run` on the line.
fn named(line: &Line, of: fn(&brokkr_store::SeatNames) -> Option<String>) -> Listed {
    let Some(run) = &line.run else {
        return Ok(Vec::new());
    };
    let rows = journal(line, |store| {
        let run = crate::selector::resolve_run_within(store, run)?;
        Ok(store.run_seats(&run, SEATS)?)
    })?;
    let mut names: Vec<String> = rows.iter().filter_map(of).collect();
    names.sort();
    names.dedup();
    Ok(names.into_iter().map(|name| (name, None)).collect())
}

/// The recipes `--recipe` resolves: the project shelf's, then the personal
/// shelf's where this machine places one, each name once. A shelf that
/// cannot be read offers nothing, and the other still offers its own.
fn recipes(line: &Line) -> Listed {
    let layout = crate::folder::Layout::discover(Path::new("."))?;
    let flag = line.recipes_dir.clone().or_else(|| line.dir.clone());
    let shelves = [
        Some(layout.recipes(flag)),
        crate::recipes::personal::placed(),
    ];
    let mut names: Vec<String> = (shelves.iter().flatten())
        .flat_map(|shelf| crate::recipes::listed(shelf).unwrap_or_default())
        .map(|(name, _)| name)
        .collect();
    names.sort();
    names.dedup();
    Ok(names.into_iter().map(|name| (name, None)).collect())
}

/// The agent library seats hire from.
fn agents(line: &Line) -> Listed {
    let layout = crate::folder::Layout::discover(Path::new("."))?;
    let (library, _) = brokkr_runtime::Library::scan(&layout.agents(line.agents_dir.clone()))?;
    let agents = library.agents();
    Ok(agents
        .map(|agent| (agent.name.clone(), Some(agent.description.clone())))
        .collect())
}

/// The realms the map names.
fn realms(line: &Line) -> Listed {
    let invocation =
        crate::Invocation::inspect(Path::new("."), line.realms.clone(), line.db.clone())?;
    let realms = invocation
        .world
        .map(|world| world.map.realms)
        .unwrap_or_default();
    Ok(realms.into_iter().map(|realm| (realm.name, None)).collect())
}

#[cfg(test)]
mod tests;
