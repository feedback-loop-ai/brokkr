//! `brokkr probe harness`: measure one agent CLI and write the facts its
//! adapter must declare (proposed decision 0075 ruling 3, #484). The
//! measuring is `brokkr_protocol::probe`'s; this handler resolves the
//! adapter and the credentials, and writes the report.

use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{anyhow, Result};
use brokkr_protocol::adapters::AdapterKind;
use brokkr_protocol::probe::{self, Declared, ProbeInput, Report};
use brokkr_protocol::secret;
use brokkr_runtime::agents::{Adapter, Adapters, ResumeIdentity};

use crate::cli_args::{ProbeCmd, ProbeHarnessArgs};
use crate::now_rfc3339;

/// How long one launch may run. A turn that asks for one word ends well
/// inside it; a CLI waiting on a prompt nobody answers is killed at it.
const LAUNCH_DEADLINE: Duration = Duration::from_secs(300);

/// `brokkr probe`.
pub(crate) fn probe(command: ProbeCmd) -> Result<ExitCode> {
    match command {
        ProbeCmd::Harness(args) => harness(args),
    }
}

/// The adapter's fields the probe launches by and compares against. The
/// passthrough is what its driver hands the CLI after `--`.
fn declared(adapter: &Adapter) -> Declared {
    Declared {
        adapter: adapter.provider.clone(),
        model_flag: adapter.model_flag.clone(),
        effort_flag: adapter.effort_flag.clone(),
        efforts: adapter.efforts.clone(),
        hands: adapter.hands.clone(),
        hands_gap: adapter.hands_gap.clone(),
        passthrough: adapter
            .driver
            .iter()
            .skip_while(|part| *part != "--")
            .skip(1)
            .cloned()
            .collect(),
        resume_versions: adapter
            .resume
            .shapes()
            .map(|(shape, assessed)| (shape.clone(), identity(&assessed.identity)))
            .collect(),
    }
}

/// The CLI version a resume shape was measured on, or why none was.
fn identity(identity: &ResumeIdentity) -> String {
    match identity {
        ResumeIdentity::Measured { version, .. } => version.clone(),
        ResumeIdentity::Unknown { reason } => format!("unknown: {reason}"),
    }
}

/// `brokkr probe harness`: one report, to `--out` or stdout, with the
/// drift since the report `--out` already held.
fn harness(args: ProbeHarnessArgs) -> Result<ExitCode> {
    let adapters = Adapters::load(&args.adapters_dir)?;
    let adapter = adapters.adapter(&args.adapter).ok_or_else(|| {
        anyhow!(
            "no adapter '{}' under {}",
            args.adapter,
            args.adapters_dir.display()
        )
    })?;
    // A built-in driver is `{brokkr} driver <kind> -- …`; any other driver
    // has no launch grammar the probe knows.
    let kind = adapter
        .driver
        .get(2)
        .and_then(|word| AdapterKind::parse(word))
        .ok_or_else(|| {
            anyhow!(
                "adapter '{}' is not launched by a built-in driver, so the probe has no \
                 launch grammar for its CLI",
                args.adapter
            )
        })?;
    let bindings = secret::resolve_bindings(&args.secrets_file, &args.credentials)
        .map_err(anyhow::Error::msg)?;
    let previous = match &args.out {
        Some(out) if out.exists() => Some(Report::parse(&std::fs::read_to_string(out)?)?),
        _ => None,
    };
    let declared = declared(adapter);
    let cli = args.cli.unwrap_or_else(|| adapter.binary.clone());
    let brokkr = std::env::current_exe()?;
    let report = probe::run(&ProbeInput {
        kind,
        cli: &cli,
        declared: &declared,
        bindings: &bindings,
        brokkr: &brokkr,
        date: &now_rfc3339(),
        deadline: LAUNCH_DEADLINE,
    })?;
    let report = match &previous {
        Some(previous) => report.with_drift_from(previous),
        None => report,
    };
    let text = format!("{}\n", serde_json::to_string_pretty(&report)?);
    match &args.out {
        Some(out) => write_report(out, &text, &report)?,
        None => print!("{text}"),
    }
    Ok(ExitCode::SUCCESS)
}

fn write_report(out: &Path, text: &str, report: &Report) -> Result<()> {
    std::fs::write(out, text)?;
    for line in report.drift_lines() {
        eprintln!("{line}");
    }
    eprintln!("probe report written to {}", out.display());
    Ok(())
}
