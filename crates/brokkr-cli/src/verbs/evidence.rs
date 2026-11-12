//! `brokkr evidence` (decision 0105 ruling 1(a)): list the host's
//! historical artifact roots with whether each still stands and its
//! retirement, or retire one under the admission lock once its evidence is
//! archived or discarded. The retirement is a sealed record beside the
//! historical one, which stays; no journal event is written (rulings 3(a)
//! and 4(a)).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;
use brokkr_runtime::engine::evidence::{self, EvidenceRoot, RetireAction, RetireOptions};
use clap::ArgGroup;

use crate::render::Safe;
use crate::{now_rfc3339, Exit};

/// How long the verb waits for the admission lock and the inventory: a
/// copy of a root's evidence may take a while, and nothing of it runs
/// while another admission holds the lock.
const DEADLINE: Duration = Duration::from_secs(3600);

/// `brokkr evidence list` and `brokkr evidence retire`.
#[derive(clap::Subcommand)]
pub(crate) enum EvidenceCmd {
    /// List every historical evidence root: its path, whether it exists,
    /// its digest count and bytes, its retirement and its superseded
    /// history. A root whose own counts cannot be read still prints, with
    /// its problem, and makes the verb exit non-zero after every row.
    List(ListArgs),
    /// Retire one historical root, under the admission lock, after
    /// archiving its evidence or discarding it.
    Retire(RetireArgs),
}

/// `brokkr evidence list`.
#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct ListArgs {
    /// Print the roots as a JSON array.
    #[arg(long)]
    pub(crate) json: bool,
}

/// `brokkr evidence retire`.
#[derive(clap::Args)]
#[group(skip)]
#[command(group(ArgGroup::new("cause").required(true)))]
pub(crate) struct RetireArgs {
    /// The artifact root the historical record names, as `evidence list`
    /// prints it.
    #[arg(long)]
    pub(crate) root: PathBuf,
    /// Archive every artifact into this absent or empty directory, write
    /// the inventory last, and keep the root. The destination is resolved
    /// by the filesystem, so a link is followed before `..`, and it must
    /// resolve outside the coordination root and the root being retired;
    /// the resolved path is the one recorded.
    #[arg(long, group = "cause")]
    pub(crate) archived_to: Option<PathBuf>,
    /// Discard the evidence: remove the artifact root by a walk that
    /// follows no link and crosses no filesystem.
    #[arg(long, group = "cause")]
    pub(crate) discarded: bool,
    /// Why the operator retires it, in their words.
    #[arg(long)]
    pub(crate) reason: String,
    /// Print the retirement as JSON.
    #[arg(long)]
    pub(crate) json: bool,
}

/// `brokkr evidence <command>`.
pub(crate) fn run(command: EvidenceCmd) -> Result<std::process::ExitCode> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("this host names no HOME, so it holds no evidence root"))?;
    let operator = super::delivery::operator_name();
    let now = now_rfc3339();
    run_in(command, &home, &now, &operator, &mut std::io::stdout())
}

/// [`run`] on the host `home`, when `now`, by `operator`, on `out`: the
/// seam a test drives without the process environment.
pub(crate) fn run_in(
    command: EvidenceCmd,
    home: &Path,
    now: &str,
    operator: &str,
    out: &mut dyn Write,
) -> Result<std::process::ExitCode> {
    match command {
        EvidenceCmd::List(ListArgs { json }) => list(home, json, out),
        EvidenceCmd::Retire(args) => retire(home, now, operator, args, out),
    }
}

/// `brokkr evidence list`: every historical root, as text or one JSON
/// array. Every row is printed before a root's own problem makes the verb
/// exit non-zero (decision 0105, M4).
fn list(home: &Path, json: bool, out: &mut dyn Write) -> Result<std::process::ExitCode> {
    let roots = evidence::list(home, deadline())?;
    if json {
        writeln!(out, "{}", serde_json::to_string_pretty(&roots)?)?;
    } else {
        for root in &roots {
            write!(out, "{}", painted(root))?;
        }
    }
    let refused = roots.iter().any(|root| root.problem.is_some());
    Ok(if refused {
        Exit::Failed
    } else {
        Exit::Completed
    }
    .into())
}

/// One root's text block: its path, existence, count and bytes, any
/// retirement, its superseded retirements, and its own problem where it
/// has one.
fn painted(root: &EvidenceRoot) -> String {
    let found = if root.exists { "exists" } else { "absent" };
    let retirement = match &root.retirement {
        None => "not retired".to_string(),
        Some(retired) => Safe::new(&retired.words()).as_str().to_string(),
    };
    let history: String = root
        .history
        .iter()
        .map(|record| format!("  {}\n", Safe::new(&record.words()).as_str()))
        .collect();
    let problem = root.problem.as_deref().map_or_else(String::new, |problem| {
        format!("  problem: {}\n", Safe::new(problem).as_str())
    });
    format!(
        "{}  {found}, {} digests, {} bytes, {retirement}\n{history}{problem}",
        Safe::new(&root.artifacts.display().to_string()).as_str(),
        root.digests,
        root.bytes,
    )
}

/// `brokkr evidence retire`: archive or discard, then seal the retirement
/// and print it.
fn retire(
    home: &Path,
    now: &str,
    operator: &str,
    RetireArgs {
        root,
        archived_to,
        discarded,
        reason,
        json,
    }: RetireArgs,
    out: &mut dyn Write,
) -> Result<std::process::ExitCode> {
    let root = std::path::absolute(&root).unwrap_or(root);
    // A relative destination is made absolute before the checks and the
    // record, exactly as `--root` is (decision 0105, M3).
    let archived_to = archived_to.map(|at| std::path::absolute(&at).unwrap_or(at));
    let action = match (archived_to.as_deref(), discarded) {
        (Some(at), _) => RetireAction::Archived { to: at },
        (None, true) => RetireAction::Discarded,
        (None, false) => {
            return Err(anyhow::anyhow!(
                "retire needs --archived-to <dir> or --discarded"
            ))
        }
    };
    let options = RetireOptions {
        artifacts: &root,
        action,
        operator,
        reason: &reason,
        now,
    };
    let retired = evidence::retire(home, options, deadline())?;
    if json {
        writeln!(out, "{}", serde_json::to_string_pretty(&retired)?)?;
        return Ok(Exit::Completed.into());
    }
    writeln!(
        out,
        "retired {}: {}",
        Safe::new(&retired.artifacts.display().to_string()).as_str(),
        Safe::new(&retired.words()).as_str()
    )?;
    Ok(Exit::Completed.into())
}

/// When the verb's inventory read and seal stop waiting.
fn deadline() -> Instant {
    Instant::now() + DEADLINE
}

#[cfg(test)]
mod tests;
