//! `brokkr queue` (decision 0068 ruling 1): the dispatcher's queue in the
//! journal's own database. `add` queues the launch `brokkr run` would make
//! with the same arguments; `move`, `hold`, `release` and `drop` change an
//! entry, each journaled with its reason; `list` reads it. Nothing here
//! starts a run.

use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use brokkr_runtime::launch::{BundleSource, QueuedLaunch};
use brokkr_store::{Attribution, NewEntry, QueueCommand, QueueEntry, Wait};
use serde::Serialize;

use super::delivery::{new_run, operator_name};
use crate::cli_args::{QueueAddArgs, QueueCmd, QueueEntryArgs, QueueListArgs};
use crate::render::Safe;
use crate::{open_journal, Access, Exit};

/// Run one queue command.
pub(crate) fn queue(workspace: &Path, command: QueueCmd) -> Result<ExitCode> {
    match command {
        QueueCmd::Add(args) => add(workspace, args),
        QueueCmd::List(args) => list(workspace, args),
        QueueCmd::Move(args) => change(workspace, args.entry, QueueCommand::Move { to: args.to }),
        QueueCmd::Hold(args) => change(workspace, args, QueueCommand::Hold),
        QueueCmd::Release(args) => change(workspace, args, QueueCommand::Release),
        QueueCmd::Drop(args) => change(workspace, args, QueueCommand::Drop),
    }
}

/// `brokkr queue add`: the launch, encoded, at the end of the queue in the
/// journal the launch would write. The entry is launched from wherever the
/// dispatcher stands, so it names its workspace absolutely; every relative
/// path it holds is relative to that workspace.
fn add(
    workspace: &Path,
    QueueAddArgs {
        run,
        priority,
        after,
        reason,
    }: QueueAddArgs,
) -> Result<ExitCode> {
    let workspace = std::path::absolute(workspace)?;
    let (request, new) = new_run(&workspace, run)?;
    let payload = QueuedLaunch::of(&request, &new)?.encode()?;
    let mut store = open_journal(&request.journal, Access::Append)?;
    let entry = NewEntry {
        payload: &payload,
        priority,
        waits: &after,
    };
    let operator = operator_name();
    let by = Attribution {
        operator: &operator,
        reason: &reason,
    };
    let id = store.queue_add(entry, by)?;
    eprintln!("queued entry {id}");
    Ok(Exit::Completed.into())
}

/// `brokkr queue move|hold|release|drop`.
fn change(
    workspace: &Path,
    QueueEntryArgs {
        entry,
        reason,
        journal,
    }: QueueEntryArgs,
    command: QueueCommand,
) -> Result<ExitCode> {
    let mut store = open_journal(&journal.journal(workspace)?, Access::Append)?;
    let operator = operator_name();
    let by = Attribution {
        operator: &operator,
        reason: &reason,
    };
    store.queue_command(entry, command, by)?;
    match command {
        QueueCommand::Move { to } => eprintln!("moved queue entry {entry} to place {to}"),
        QueueCommand::Hold => eprintln!("held queue entry {entry}"),
        QueueCommand::Release => eprintln!("released queue entry {entry}"),
        QueueCommand::Drop => eprintln!("dropped queue entry {entry}"),
    }
    Ok(Exit::Completed.into())
}

/// `brokkr queue list`: a look, so the journal is opened read-only.
fn list(workspace: &Path, QueueListArgs { journal, json }: QueueListArgs) -> Result<ExitCode> {
    let store = open_journal(&journal.journal(workspace)?, Access::Read)?;
    let entries = store
        .queue_list()?
        .into_iter()
        .map(|entry| {
            let launch = QueuedLaunch::decode(&entry.payload)
                .with_context(|| format!("queue entry {}", entry.id))?;
            Ok((entry, launch))
        })
        .collect::<Result<Vec<_>>>()?;
    let rows = rows(&entries);
    match json {
        true => println!("{}", serde_json::to_string_pretty(&rows)?),
        false => print!("{}", table(&rows)),
    }
    Ok(Exit::Completed.into())
}

/// One entry as `list` shows it: the one derivation of its facts, which
/// the table and `--json` both paint. The queue has one surface today;
/// when a second shows it (#491's fleet view), this row moves to
/// brokkr-view (decision 0071 ruling 7).
#[derive(Serialize)]
struct Row<'a> {
    place: Option<u32>,
    entry: i64,
    state: &'static str,
    run: Option<&'a str>,
    priority: i64,
    #[serde(serialize_with = "each_wait")]
    waits: &'a [Wait],
    added_at: &'a str,
    /// The launch as it is encoded.
    launch: &'a QueuedLaunch,
}

/// A wait as scripts read it.
#[derive(Serialize)]
struct WaitRow {
    entry: i64,
    on: &'static str,
}

fn each_wait<S: serde::Serializer>(waits: &&[Wait], to: S) -> Result<S::Ok, S::Error> {
    to.collect_seq(waits.iter().map(|wait| WaitRow {
        entry: wait.entry.0,
        on: wait.on.word(),
    }))
}

fn rows(entries: &[(QueueEntry, QueuedLaunch)]) -> Vec<Row<'_>> {
    entries
        .iter()
        .map(|(entry, launch)| Row {
            place: entry.position,
            entry: entry.id.0,
            state: entry.state.word(),
            run: entry.state.run(),
            priority: entry.priority,
            waits: &entry.waits,
            added_at: &entry.added_at,
            launch,
        })
        .collect()
}

/// The listing as an operator reads it: one aligned line per entry.
fn table(rows: &[Row<'_>]) -> String {
    if rows.is_empty() {
        return "the queue is empty\n".to_string();
    }
    let header = [
        "PLACE", "ENTRY", "STATE", "PRIORITY", "AFTER", "RUN", "BUNDLE", "FEATURE",
    ];
    let mut lines = vec![header.map(Safe::new)];
    for row in rows {
        let dash = || "-".to_string();
        let waits: Vec<String> = row.waits.iter().map(ToString::to_string).collect();
        let bundle = match &row.launch.bundle {
            BundleSource::Dir(dir) => dir.display().to_string(),
            BundleSource::Recipe { name, .. } => format!("recipe {name}"),
        };
        let cells = [
            row.place.map_or_else(dash, |place| place.to_string()),
            row.entry.to_string(),
            row.state.to_string(),
            row.priority.to_string(),
            Some(waits.join(","))
                .filter(|w| !w.is_empty())
                .unwrap_or_else(dash),
            row.run.map_or_else(dash, str::to_string),
            bundle,
            row.launch.feature.clone(),
        ];
        lines.push(cells.map(|cell| Safe::new(&cell)));
    }
    let widths: Vec<usize> = (0..header.len())
        .map(|column| {
            lines
                .iter()
                .map(|line| line[column].width())
                .max()
                .unwrap_or(0)
        })
        .collect();
    lines
        .iter()
        .map(|line| {
            let padded: Vec<String> = line
                .iter()
                .zip(&widths)
                .map(|(cell, width)| cell.padded(*width))
                .collect();
            format!("{}\n", padded.join("  ").trim_end())
        })
        .collect()
}

#[cfg(test)]
mod tests;
