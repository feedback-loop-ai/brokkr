//! `brokkr queue` (decision 0068 ruling 1): the dispatcher's queue in the
//! journal's own database. `add` queues the launch `brokkr run` would make
//! with the same arguments; `move`, `hold`, `release`, `repin` and `drop`
//! change an entry, each journaled with its reason; `list` reads it, with
//! admission's verdict on each entry that waits, and `judge` shows the
//! same once it has latched the realm drift admission finds. Nothing here
//! starts a run.

use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use brokkr_runtime::admission::{self, Host, Judged, Verdict};
use brokkr_runtime::launch::{BundleSource, QueuedLaunch};
use brokkr_store::{Attribution, NewEntry, QueueCommand, Wait};
use serde::Serialize;

use super::delivery::{new_run, operator_name};
use crate::cli_args::{QueueAddArgs, QueueCmd, QueueEntryArgs, QueueJudgeArgs, QueueListArgs};
use crate::render::Safe;
use crate::{open_journal, Access, Exit};

/// Run one queue command.
pub(crate) fn queue(workspace: &Path, command: QueueCmd) -> Result<ExitCode> {
    match command {
        QueueCmd::Add(args) => add(workspace, args),
        QueueCmd::List(args) => list(workspace, args),
        QueueCmd::Judge(args) => judge(workspace, args),
        QueueCmd::Move(args) => change(workspace, args.entry, QueueCommand::Move { to: args.to }),
        QueueCmd::Hold(args) => change(workspace, args, QueueCommand::Hold),
        QueueCmd::Release(args) => change(workspace, args, QueueCommand::Release),
        QueueCmd::Repin(args) => repin(workspace, args),
        QueueCmd::Drop(args) => change(workspace, args, QueueCommand::Drop),
    }
}

/// `brokkr queue add`: the launch, encoded, at the end of the queue in the
/// journal the launch would write. The entry is launched from wherever the
/// dispatcher stands, so it names its workspace absolutely, and every path
/// it holds is anchored to that workspace ([`QueuedLaunch::of`]).
fn add(
    workspace: &Path,
    QueueAddArgs {
        launch,
        priority,
        after,
        reason,
    }: QueueAddArgs,
) -> Result<ExitCode> {
    let workspace = std::path::absolute(workspace)?;
    let (request, new) = new_run(&workspace, launch)?;
    let payload = QueuedLaunch::of(&request, &new)?.encode()?;
    let mut store = open_journal(&request.journal, Access::Append)?;
    let entry = NewEntry {
        payload: &payload,
        priority,
        waits: &after,
    };
    let id = attributed(&reason, |by| store.queue_add(entry, by))?;
    eprintln!("queued entry {id}");
    Ok(Exit::Completed.into())
}

/// Do `act` as the operator, for `reason`.
fn attributed<T>(reason: &str, act: impl FnOnce(Attribution<'_>) -> T) -> T {
    let operator = operator_name();
    act(Attribution {
        operator: &operator,
        reason,
    })
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
    attributed(&reason, |by| store.queue_command(entry, command, by))?;
    match command {
        QueueCommand::Move { to } => eprintln!("moved queue entry {entry} to place {to}"),
        QueueCommand::Hold => eprintln!("held queue entry {entry}"),
        QueueCommand::Release => eprintln!("released queue entry {entry}"),
        QueueCommand::Drop => eprintln!("dropped queue entry {entry}"),
    }
    Ok(Exit::Completed.into())
}

/// `brokkr queue repin`: the operator's release of an entry's latched
/// realm-drift hold ([`admission::release`]): its launch under the map
/// that would govern it now, written beside the one it was queued with
/// and journaled with its reason, saying the differences it accepted.
/// Admission compares the entry against the map it stands for from then
/// on.
fn repin(
    workspace: &Path,
    QueueEntryArgs {
        entry,
        reason,
        journal,
    }: QueueEntryArgs,
) -> Result<ExitCode> {
    let mut store = open_journal(&journal.journal(workspace)?, Access::Append)?;
    let accepted = attributed(&reason, |by| {
        admission::release(&mut store, entry, by).map_err(anyhow::Error::from)
    })?;
    eprintln!("re-pinned queue entry {entry} to the realms map on disk, accepting: {accepted}");
    Ok(Exit::Completed.into())
}

/// `brokkr queue list`: a look, so the journal is opened read-only. Each
/// waiting entry carries admission's verdict, judged as the journal, the
/// maps and this machine stand now; a drift no latch records yet is
/// shown, and not latched.
fn list(workspace: &Path, QueueListArgs { journal, json }: QueueListArgs) -> Result<ExitCode> {
    let store = open_journal(&journal.journal(workspace)?, Access::Read)?;
    let entries = on_this_host(|host| Ok(admission::pass_within(&store, host)?))?;
    show(&entries, json)
}

/// `brokkr queue judge`: admission's writing pass ([`admission::judge`]),
/// which latches the realm drift it finds, journaled with the reason, and
/// then the listing `list` shows.
fn judge(
    workspace: &Path,
    QueueJudgeArgs {
        reason,
        list: QueueListArgs { journal, json },
    }: QueueJudgeArgs,
) -> Result<ExitCode> {
    let mut store = open_journal(&journal.journal(workspace)?, Access::Append)?;
    let entries = on_this_host(|host| {
        attributed(&reason, |by| {
            admission::judge_within(&mut store, host, by).map_err(anyhow::Error::from)
        })
    })?;
    show(&entries, json)
}

/// Do `act` on this machine as admission measures it (decision 0068
/// ruling 3): its host configuration where XDG puts it, and the free space
/// statvfs reports.
fn on_this_host<T>(act: impl FnOnce(&Host<'_>) -> Result<T>) -> Result<T> {
    on_the_host_at(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
        act,
    )
}

/// [`on_this_host`], with the two directories that place the host
/// configuration given rather than read from the environment.
fn on_the_host_at<T>(
    xdg: Option<OsString>,
    home: Option<OsString>,
    act: impl FnOnce(&Host<'_>) -> Result<T>,
) -> Result<T> {
    let file = admission::host_file(xdg, home)?;
    act(&Host {
        file: &file,
        free: &admission::free_bytes,
    })
}

/// Print the judged queue, as a table or as `--json`.
fn show(entries: &[Judged], json: bool) -> Result<ExitCode> {
    let rows = rows(entries);
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
    /// Admission's word while the entry waits; `None` once it started.
    admission: Option<AdmissionRow>,
}

/// Admission's verdict as scripts read it: the entry's standing and each
/// reason it may not start now, typed by its kind.
#[derive(Serialize)]
struct AdmissionRow {
    standing: &'static str,
    reasons: Vec<ReasonRow>,
}

#[derive(Serialize)]
struct ReasonRow {
    kind: &'static str,
    says: String,
}

impl AdmissionRow {
    fn of(verdict: &Verdict) -> AdmissionRow {
        AdmissionRow {
            standing: verdict.standing().word(),
            reasons: verdict
                .reasons
                .iter()
                .map(|reason| ReasonRow {
                    kind: reason.kind(),
                    says: reason.to_string(),
                })
                .collect(),
        }
    }

    /// The cell a table shows: the standing, and why when it is not
    /// admissible.
    fn cell(&self) -> String {
        let says: Vec<&str> = self
            .reasons
            .iter()
            .map(|reason| reason.says.as_str())
            .collect();
        match says.is_empty() {
            true => self.standing.to_string(),
            false => format!("{}: {}", self.standing, says.join(" | ")),
        }
    }
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

fn rows(entries: &[Judged]) -> Vec<Row<'_>> {
    entries
        .iter()
        .map(
            |Judged {
                 entry,
                 launch,
                 verdict,
             }| Row {
                place: entry.position,
                entry: entry.id.0,
                state: entry.state.word(),
                run: entry.state.run(),
                priority: entry.priority,
                waits: &entry.waits,
                added_at: &entry.added_at,
                launch,
                admission: verdict.as_ref().map(AdmissionRow::of),
            },
        )
        .collect()
}

/// The listing as an operator reads it: one aligned line per entry.
fn table(rows: &[Row<'_>]) -> String {
    if rows.is_empty() {
        return "the queue is empty\n".to_string();
    }
    let header = [
        "PLACE",
        "ENTRY",
        "STATE",
        "PRIORITY",
        "AFTER",
        "RUN",
        "BUNDLE",
        "FEATURE",
        "ADMISSION",
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
            row.admission.as_ref().map_or_else(dash, AdmissionRow::cell),
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
