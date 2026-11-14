//! `brokkr sweep` (decision 0104 ruling 9, as R3 amends it): one pass
//! under the journal's dispatch lock, or a dry run that opens the journal
//! read-only, takes no hold and writes nothing. The verdicts and their
//! lines are brokkr-runtime's; this handler resolves the journal, takes
//! the lock for a pass, and paints each line through [`Safe`].

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{anyhow, Result};
use brokkr_runtime::sweep as sweep_run;

use crate::cli_args::SweepArgs;
use crate::render::Safe;
use crate::{open_journal, Access, Exit};

/// Run one sweep command.
pub(crate) fn sweep(
    workspace: &Path,
    SweepArgs { dry, journal }: SweepArgs,
    out: &mut impl Write,
) -> Result<ExitCode> {
    let db = journal.journal(workspace)?;
    let report = match dry {
        true => {
            let store = open_journal(&db, Access::Read)?;
            sweep_run::plan(&store)?
        }
        false => {
            let mut store = open_journal(&db, Access::Append)?;
            let held = match brokkr_runtime::take_dispatch(&mut store)? {
                Ok(held) => held,
                Err(refused) => return Err(anyhow!(refused)),
            };
            sweep_run::pass(&mut store, &held)?
        }
    };
    out.write_all(report.lines_with(safe, dry).as_bytes())?;
    Ok(Exit::Completed.into())
}

/// The safe renderer every operator line uses (#468).
fn safe(text: &str) -> String {
    Safe::new(text).as_str().to_string()
}

#[cfg(test)]
mod tests;
