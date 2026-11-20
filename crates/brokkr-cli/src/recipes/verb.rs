//! The `recipes` family's verbs (#1169), and the one among them that reads
//! a journal: `recipes usage`, what each recipe's attempts used, per phase
//! (decision 0112 ruling 4). Its figures are `brokkr-view`'s
//! [`brokkr_view::recipe_usage`], which `--json` emits as they are; the
//! table only paints them (decision 0071 ruling 7).

use std::path::{Path, PathBuf};

use anyhow::Result;
use brokkr_view::{PhaseUsage, Spread};
use clap::Subcommand;

use crate::cli_args::JournalArgs;
use crate::{open_journal, Access};

#[derive(Subcommand)]
pub(crate) enum RecipesCmd {
    /// List recipes under --dir, then those on the personal shelf, each
    /// with its origin, project or personal; broken ones print a warning
    /// line, never abort the listing.
    List {
        /// The recipe library directory (default the found .brokkr/recipes,
        /// listing nothing while it is absent, else recipes).
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Install a recipe from a local path or a git URL into the shelf at
    /// the repository's root, .brokkr/recipes/<name> where the folder is,
    /// else recipes/<name>, with the agents and charters it names that
    /// the agent library lacks.
    Add {
        /// A local bundle directory or a git URL.
        source: String,
        /// The name the recipe is installed under.
        #[arg(long)]
        name: String,
        /// With a git URL, the bundle's directory inside the clone, a
        /// relative path of plain names (`recipes/fast`), walked without
        /// following a link; without it, the clone's root or its one
        /// subdirectory holding a bundle.json. The bases its recipe
        /// extends come from beside it.
        #[arg(long)]
        path: Option<String>,
        /// Install onto the personal shelf instead,
        /// $XDG_CONFIG_HOME/brokkr/recipes/<name>, else
        /// $HOME/.config/brokkr/recipes/<name>, beside host.json and in no
        /// repository; it copies no agent or charter.
        #[arg(long)]
        personal: bool,
    },
    /// Print one recipe's RESOLVED bundle and, when it extends another,
    /// the composition chain it was resolved from.
    Show {
        /// The recipe's name, resolved to <dir>/<name>, else to the
        /// personal shelf's; a name on both refuses.
        name: String,
        /// The recipe library directory (default the found .brokkr/recipes,
        /// else recipes).
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Print what each recipe's attempts used, per phase, as the engine
    /// measured their process trees: the minimum, average and maximum of
    /// peak and mean memory, peak and mean load, CPU time and wall time,
    /// over the drivers measured. An attempt journaled without usage is
    /// counted as unmeasured, never as zero.
    Usage {
        /// One recipe, by the name its runs started under (default every
        /// recipe the journal has runs of).
        name: Option<String>,
        #[command(flatten)]
        journal: JournalArgs,
        /// Print the rows as JSON.
        #[arg(long)]
        json: bool,
    },
}

/// `brokkr recipes usage`: the rows for `name`, or every recipe, over
/// every run of the journal `journal` names, opened for reading.
pub(crate) fn usage(
    workspace: &Path,
    name: Option<String>,
    journal: JournalArgs,
    json: bool,
) -> Result<()> {
    let store = open_journal(&journal.journal(workspace)?, Access::Read)?;
    let runs = store
        .list_runs()?
        .into_iter()
        .map(|(run_id, _, _)| store.load(&run_id))
        .collect::<Result<Vec<_>, _>>()?;
    let rows = brokkr_view::recipe_usage(&runs, name.as_deref());
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        print!("{}", table(&rows, name.as_deref()));
    }
    Ok(())
}

/// The rows as the operator reads them: each phase's counts, then each
/// measure's minimum, average and maximum in MiB, cores and seconds.
fn table(rows: &[PhaseUsage], name: Option<&str>) -> String {
    if rows.is_empty() {
        let whose = name.map_or_else(|| "any recipe".to_string(), |name| format!("'{name}'"));
        return format!("no attempt of {whose} is journaled\n");
    }
    let mut out = String::new();
    for row in rows {
        out += &format!(
            "{} / {}: {} attempts, {} unmeasured, {} drivers measured\n",
            row.recipe, row.phase, row.attempts, row.unmeasured, row.measured
        );
        let measures = [
            (
                "peak memory",
                row.peak_memory_bytes,
                mib as fn(u64) -> String,
            ),
            ("mean memory", row.mean_memory_bytes, mib),
            ("peak load", row.peak_load_millicores, cores),
            ("mean load", row.mean_load_millicores, cores),
            ("cpu time", row.cpu_ms, seconds),
            ("wall time", row.wall_ms, seconds),
        ];
        for (label, spread, unit) in measures {
            out += &spread.map_or_else(
                || format!("  {label:<12} unmeasured\n"),
                |spread| line(label, spread, unit),
            );
        }
    }
    out
}

fn line(label: &str, Spread { min, average, max }: Spread, unit: fn(u64) -> String) -> String {
    format!(
        "  {label:<12} min {}  average {}  max {}\n",
        unit(min),
        unit(average),
        unit(max)
    )
}

fn mib(bytes: u64) -> String {
    format!("{:.1} MiB", bytes as f64 / f64::from(1u32 << 20))
}

fn cores(millicores: u64) -> String {
    format!("{:.2} cores", millicores as f64 / 1000.0)
}

fn seconds(millis: u64) -> String {
    format!("{:.1} s", millis as f64 / 1000.0)
}

#[cfg(test)]
mod tests;
