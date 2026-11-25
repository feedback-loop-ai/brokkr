//! Work orders in the journal (decision 0116 rulings 2, 9 and 10).

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use brokkr_core::work_order::{Trackers, WorkOrder, WorkOrderError, WorkOrderId, WorkOrderState};
use brokkr_store::{OrderRefusal, StoreError};
use serde::Serialize;

use super::queue::attributed;
use crate::cli_args::{
    OrderAddArgs, OrderEditArgs, OrderListArgs, OrderRuleArgs, OrderShowArgs, OrderWriteArgs,
    OrdersCmd,
};
use crate::folder::Layout;
use crate::render::Safe;
use crate::{open_journal, Access, Exit};

const SHIPPED: &[&str] = &[
    include_str!("../../trackers/azure-devops.json"),
    include_str!("../../trackers/github.json"),
    include_str!("../../trackers/gitlab.json"),
    include_str!("../../trackers/jira.json"),
    include_str!("../../trackers/linear.json"),
    include_str!("../../trackers/zenhub.json"),
];

#[derive(Debug, thiserror::Error)]
enum TrackerError {
    #[error("cannot read trackers at '{}'", path.display())]
    Read { path: PathBuf, source: io::Error },
    #[error("invalid tracker definition in '{}'", path.display())]
    Definition {
        path: PathBuf,
        source: WorkOrderError,
    },
}

fn tracker_paths(directory: &Path) -> Result<Vec<PathBuf>, TrackerError> {
    let read_error = |source| TrackerError::Read {
        path: directory.to_path_buf(),
        source,
    };
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            // A dangling directory link stands here and should be refused.
            let absent = directory
                .symlink_metadata()
                .err()
                .map(|missing| missing.kind())
                == Some(io::ErrorKind::NotFound);
            return if absent {
                Ok(Vec::new())
            } else {
                Err(read_error(source))
            };
        }
        Err(source) => return Err(read_error(source)),
    };
    let mut paths: Vec<_> = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<_>>()
        .map_err(read_error)?;
    paths.retain(|path| path.extension().is_some_and(|ext| ext == "json"));
    paths.sort_unstable();
    Ok(paths)
}

fn trackers(directory: Option<&Path>) -> Result<Trackers, TrackerError> {
    let paths = directory
        .map(tracker_paths)
        .transpose()?
        .unwrap_or_default();
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let text = fs::read_to_string(&path).map_err(|source| TrackerError::Read {
            path: path.clone(),
            source,
        });
        files.push((path, text?));
    }
    // The core reader stops on the failing definition, so its path is
    // the last one yielded. Shipped definitions are checked in tests.
    let mut current = None;
    let adopter = files.iter().map(|(path, text)| {
        current = Some(path);
        text.as_str()
    });
    let definitions = SHIPPED.iter().copied().chain(adopter);
    Trackers::from_definitions(definitions).map_err(|source| TrackerError::Definition {
        path: current
            .expect("the shipped tracker definitions are valid")
            .clone(),
        source,
    })
}

pub(crate) fn orders(
    workspace: &Path,
    command: OrdersCmd,
    out: &mut impl Write,
) -> Result<ExitCode> {
    let layout = Layout::discover(workspace)?;
    let trackers = trackers(layout.trackers().as_deref())?;
    match command {
        OrdersCmd::List(args) => list(workspace, args, &trackers, out)?,
        OrdersCmd::Show(args) => show(workspace, args, &trackers, out)?,
        OrdersCmd::Add(args) => add(workspace, args, &trackers, out)?,
        OrdersCmd::Edit(args) => edit(workspace, args, &trackers, out)?,
        OrdersCmd::Rule(args) => rule(workspace, args, &trackers, out)?,
    }
    Ok(Exit::Completed.into())
}

/// One listing row, shared by the text and JSON output.
#[derive(Serialize)]
struct Row<'a> {
    id: &'a WorkOrderId,
    state: WorkOrderState,
    open_asks: usize,
    title: &'a str,
}

impl<'a> From<&'a WorkOrder> for Row<'a> {
    fn from(order: &'a WorkOrder) -> Self {
        Self {
            id: order.id().expect("a journaled work order has an id"),
            state: order.state(),
            open_asks: order.open_asks(),
            title: order.title(),
        }
    }
}

fn list(
    workspace: &Path,
    OrderListArgs { journal, json }: OrderListArgs,
    trackers: &Trackers,
    out: &mut impl Write,
) -> Result<()> {
    let store = open_journal(&journal.journal(workspace)?, Access::Read)?;
    let orders = match store.orders(trackers) {
        Err(StoreError::Orders(OrderRefusal::SchemaMismatch(0))) => Vec::new(),
        orders => orders?,
    };
    let rows: Vec<_> = orders.iter().map(Row::from).collect();
    if json {
        writeln!(out, "{}", serde_json::to_string_pretty(&rows)?)?;
    } else {
        for row in rows {
            let state = serde_json::to_string(&row.state)?;
            let state = state.trim_matches('"');
            let title = Safe::new(row.title);
            let title = title.as_str();
            writeln!(out, "{}  {state}  {}  {title}", row.id, row.open_asks)?;
        }
    }
    Ok(())
}

fn show(
    workspace: &Path,
    OrderShowArgs { id, journal }: OrderShowArgs,
    trackers: &Trackers,
    out: &mut impl Write,
) -> Result<()> {
    let store = open_journal(&journal.journal(workspace)?, Access::Read)?;
    let order = match store.order(&id, trackers) {
        Err(StoreError::Orders(OrderRefusal::SchemaMismatch(0))) => {
            return Err(StoreError::from(OrderRefusal::Unknown(id)).into());
        }
        order => order?,
    };
    writeln!(out, "digest {}", order.digest())?;
    for line in order.render().lines() {
        writeln!(out, "{}", Safe::new(line).as_str())?;
    }
    Ok(())
}

fn read_file(file: &Path, trackers: &Trackers) -> Result<WorkOrder> {
    let text = std::fs::read_to_string(file)
        .with_context(|| format!("cannot read work order '{}'", file.display()))?;
    Ok(WorkOrder::parse(&text, trackers)?)
}

fn add(
    workspace: &Path,
    OrderAddArgs { file, write }: OrderAddArgs,
    trackers: &Trackers,
    out: &mut impl Write,
) -> Result<()> {
    let order = read_file(&file, trackers)?;
    let state = order.state();
    let OrderWriteArgs { journal, reason } = write;
    let mut store = open_journal(&journal.journal(workspace)?, Access::Append)?;
    let id = attributed(&reason, |by| store.order_add(order, by))?;
    writeln!(out, "proposed {id}")?;
    if state != WorkOrderState::Proposed {
        const PROPOSAL_NOTE: &str = "the journal holds it as proposed until `brokkr orders rule`";
        writeln!(out, "{PROPOSAL_NOTE}")?;
    }
    Ok(())
}

fn edit(
    workspace: &Path,
    OrderEditArgs {
        id,
        file,
        write: OrderWriteArgs { journal, reason },
    }: OrderEditArgs,
    trackers: &Trackers,
    out: &mut impl Write,
) -> Result<()> {
    let text = read_file(&file, trackers)?;
    let mut store = open_journal(&journal.journal(workspace)?, Access::Append)?;
    attributed(&reason, |by| store.order_edit(&id, text, by))?;
    writeln!(out, "edited {id}, proposed until ruled again")?;
    Ok(())
}

fn rule(
    workspace: &Path,
    OrderRuleArgs {
        id,
        digest,
        write: OrderWriteArgs { journal, reason },
    }: OrderRuleArgs,
    trackers: &Trackers,
    out: &mut impl Write,
) -> Result<()> {
    let mut store = open_journal(&journal.journal(workspace)?, Access::Append)?;
    // The journal's ask count assumes the references were checked at
    // admission. A ruling also needs text this CLI can show.
    store.order(&id, trackers)?;
    attributed(&reason, |by| store.order_rule(&id, &digest, by))?;
    writeln!(out, "ruled {id}")?;
    Ok(())
}

#[cfg(test)]
mod tests;
