//! `brokkr probe harness`: measure one agent CLI and write the facts its
//! adapter must declare (proposed decision 0075 ruling 3, #484). The
//! measuring is `brokkr_protocol::probe`'s; this handler resolves the
//! adapter and the credentials, and writes the report.

use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{anyhow, Result};
use brokkr_protocol::adapters::AdapterKind;
use brokkr_protocol::native_controls::harness_arguments;
use brokkr_protocol::probe::{
    self, Declared, DeclaredOff, Native, NativePower, OffControl, ProbeInput, Report,
};
use brokkr_protocol::secret;
use brokkr_runtime::agents::{Adapter, Adapters, ResumeIdentity};
use brokkr_runtime::capabilities::{
    harness_of, Authority, Denial, NativeInventory, NativePlan, Requests, ADAPTER_SEAT, UNMAPPED,
};

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
/// passthrough is what its driver hands the CLI, read by the engine's own
/// [`harness_arguments`], with or without a `--` before it.
fn declared(adapter: &Adapter) -> Declared {
    Declared {
        adapter: adapter.provider.clone(),
        model_flag: adapter.model_flag.clone(),
        effort_flag: adapter.effort_flag.clone(),
        efforts: adapter.efforts.clone(),
        hands: adapter.hands.clone(),
        hands_gap: adapter.hands_gap.clone(),
        passthrough: harness_arguments(&adapter.driver).to_vec(),
        resume_versions: adapter
            .resume
            .0
            .iter()
            .map(|(shape, assessed)| (shape.clone(), identity(&assessed.identity)))
            .collect(),
        native: native(adapter),
    }
}

/// The adapter's native capabilities as the probe exercises them
/// (decision 0065 ruling 4): each declared power, and the OFF argv the
/// engine composes for a seat granted nothing, or why it composes none.
fn native(adapter: &Adapter) -> Native {
    let known = match &adapter.native {
        NativeInventory::Known { known, .. } => known,
        NativeInventory::Unmeasured(reason) => return Native::Unmeasured(reason.clone()),
    };
    let powers = known
        .iter()
        .map(|(key, power)| NativePower {
            key: key.clone(),
            capability: power.capability.clone(),
            tools: power.tools.clone(),
            off: match power.declared_denial() {
                Denial::Delivered => DeclaredOff::Switched,
                Denial::Impossible(_) => DeclaredOff::Unsupported,
                Denial::Unmeasured(_) => DeclaredOff::Unmeasured,
            },
        })
        .collect();
    let assessed =
        Authority::nothing(UNMAPPED, Path::new("")).assess(adapter, ADAPTER_SEAT, Requests::new());
    let off = match assessed.as_ref().map(|outcome| (outcome, &outcome.native)) {
        // The launch's own lowering refuses a plan its harness cannot
        // consume, and the probe reports that as the OFF control it is not.
        Ok((
            outcome,
            NativePlan::Known {
                contribution,
                expected,
                ..
            },
        )) => match contribution.segment(&outcome.provider, &outcome.harness, expected) {
            Ok(segment) => OffControl::Argv(segment.argv),
            Err(refusal) => OffControl::Refused(refusal.cause),
        },
        Ok((_, NativePlan::Unmeasured { reason, .. })) | Err(reason) => {
            OffControl::Refused(reason.clone())
        }
    };
    Native::Known { powers, off }
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
    // What the adapter declares is read as the engine reads it, whatever
    // launches it, so a driver the probe cannot launch is refused after.
    let declared = declared(adapter);
    let kind = launch_kind(adapter, &args.adapter)?;
    let bindings = secret::resolve_bindings(&args.secrets_file, &args.credentials)
        .map_err(anyhow::Error::msg)?;
    let previous = previous_report(args.out.as_deref())?;
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

/// The built-in driver kind that launches `adapter`'s CLI, as the engine's
/// own [`harness_of`] reads it; any other driver has no launch grammar the
/// probe knows.
fn launch_kind(adapter: &Adapter, name: &str) -> Result<AdapterKind> {
    AdapterKind::parse(harness_of(&adapter.driver)).ok_or_else(|| {
        anyhow!(
            "adapter '{name}' is not launched by a built-in driver, so the probe has no \
                 launch grammar for its CLI"
        )
    })
}

/// The report `--out` already holds, which the new one's drift is read
/// against, or none when `--out` is absent or names no file yet.
fn previous_report(out: Option<&Path>) -> Result<Option<Report>> {
    match out {
        Some(out) if out.exists() => Ok(Some(Report::parse(&std::fs::read_to_string(out)?)?)),
        _ => Ok(None),
    }
}

fn write_report(out: &Path, text: &str, report: &Report) -> Result<()> {
    std::fs::write(out, text)?;
    for line in report.drift_lines() {
        eprintln!("{line}");
    }
    eprintln!("probe report written to {}", out.display());
    Ok(())
}
