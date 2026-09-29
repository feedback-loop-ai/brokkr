//! The harness conformance probe (proposed decision 0075 ruling 3, #484):
//! run one agent CLI against a scratch repository and write the facts
//! its adapter must declare. Each fact is measured, with its evidence;
//! `unmeasured`, with why; or `unsupported`, as the CLI answered.
//!
//! It measures from what the CLI itself does: its exit statuses, its
//! event stream, and the files it writes under a scratch HOME. It never
//! asks a model what it can do, and it changes no seat, adapter or recipe:
//! the report is its whole output.
//!
//! Observing is split from deciding (decision 0071 ruling 10):
//! - `plan` turns the harness's launch grammar and the adapter's declared
//!   argv into the launches to try;
//! - `observe` is the one module with effects. It builds the scratch
//!   world, runs each launch to an exit or a deadline, and masks what it
//!   captured;
//! - `measure` reads the observations into facts, and `judge` derives the
//!   seat eligibility, the adapter comparison and the drift. Both are pure.
//!
//! Live probing spends real credentials, bound by name through decision
//! 0012's store. It is an operator host step, never CI: the tests drive
//! recorded stream shapes through fake CLIs.

mod facts;
mod judge;
mod measure;
mod observe;
mod plan;

use std::path::Path;
use std::time::Duration;

pub use facts::Report;

use crate::adapters::AdapterKind;
use crate::secret::BoundSecret;
use facts::{Cli, Host, PROBE_VERSION};
use observe::{Runner, Scratch, Trial};

/// The adapter fields the probe launches by and compares against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// The adapter's name, which the report carries.
    pub adapter: String,
    pub model_flag: Option<String>,
    pub effort_flag: Option<String>,
    pub efforts: Vec<String>,
    /// The argv that puts the CLI's hands in the box, placeholders intact.
    pub hands: Option<Vec<String>>,
    pub hands_gap: Option<String>,
    /// What the adapter's driver passes the CLI on every launch.
    pub passthrough: Vec<String>,
    /// Each resume shape's name and the CLI version it was measured on.
    pub resume_versions: Vec<(String, String)>,
}

/// One probe run's inputs.
pub struct ProbeInput<'a> {
    pub kind: AdapterKind,
    /// The CLI to launch: a path, or a name `PATH` resolves.
    pub cli: &'a str,
    pub declared: &'a Declared,
    /// The credentials a turn is given; a turn without them measures the
    /// authentication refusal.
    pub bindings: &'a [BoundSecret],
    /// This binary, whose `hands serve` is the MCP server the hands argv
    /// supplies.
    pub brokkr: &'a Path,
    pub date: &'a str,
    /// How long one launch may run before it is killed.
    pub deadline: Duration,
}

/// Why the probe produced no report.
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error(
        "adapter '{adapter}' is not an agent harness the probe knows; it measures \
         claude, codex and dsh"
    )]
    NotAHarness { adapter: String },
    #[error("{what}: {source}")]
    Io {
        what: String,
        source: std::io::Error,
    },
    /// A bound credential the launch cannot be given, in the injector's
    /// own words.
    #[error("{0}")]
    Credential(String),
    /// A launch whose tree could not be proven over: something of it may
    /// still run, so nothing it printed is read (#403).
    #[error("the CLI's process tree was not proven over: {0}")]
    Unended(crate::process::Unsettled),
    #[error("not a {} report: {problem}", PROBE_VERSION)]
    Report { problem: String },
}

/// Measure one CLI: every launch the plan names, in one scratch world.
pub fn run(input: &ProbeInput<'_>) -> Result<Report, ProbeError> {
    let plan = plan::plan(input.kind, input.declared)?;
    let scratch = Scratch::create()?;
    scratch.plant(&plan.user_config)?;
    let runner = Runner {
        scratch: &scratch,
        cli: input.cli,
        brokkr: input.brokkr,
        bindings: input.bindings,
        deadline: input.deadline,
    };
    let no_credentials = if input.bindings.is_empty() {
        Trial::Untried(
            "no credential was bound, so a turn without one is the plain turn".to_string(),
        )
    } else {
        Trial::Observed(runner.launch(&plan.turn, false)?)
    };
    let observed = measure::Observed {
        version: runner.launch(&plan.version, true)?,
        turn: runner.launch(&plan.turn, true)?,
        no_credentials,
        bad_model: runner.trial(&plan.bad_model)?,
        bad_effort: runner.trial(&plan.bad_effort)?,
        boxed: runner.trial(&plan.boxed)?,
    };
    let bound: Vec<&str> = input.bindings.iter().map(BoundSecret::name).collect();
    let facts = measure::facts(&plan, &observed, &bound);
    let version = measure::version(&observed.version);
    Ok(Report {
        probe: PROBE_VERSION.to_string(),
        adapter: input.declared.adapter.clone(),
        adapter_fields: judge::adapter_fields(input.declared, &facts, &version),
        eligibility: judge::eligibility(&facts),
        cli: Cli {
            command: input.cli.to_string(),
            version,
        },
        host: Host {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        },
        date: input.date.to_string(),
        facts,
        drift: Vec::new(),
    })
}

#[cfg(test)]
mod tests;
