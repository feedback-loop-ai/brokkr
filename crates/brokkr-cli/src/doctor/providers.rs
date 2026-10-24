//! One provider's `brokkr doctor` line (#288): what [`super::probe_providers`]
//! observes of each provider an adapter file declares, the line it prints,
//! and the availability fact it records.

use brokkr_runtime::agents::Adapter;
use brokkr_runtime::{Availability, Presence};

use super::{CompositeProbe, Observed, Report};
use crate::render::Safe;

/// Probe one provider and report its line: its version where the probe
/// answered, why it was not probed where selection refused, and the
/// binary it could not find otherwise.
pub(super) fn probe_provider(
    report: &mut Report,
    availability: &mut Availability,
    adapter: &Adapter,
    probe: fn(&str) -> Option<String>,
    composite: CompositeProbe,
) {
    let models: Vec<&str> = adapter.models.keys().map(String::as_str).collect();
    let serves = match models.is_empty() {
        true => "serves no abstract model yet".to_string(),
        false => format!("serves {}", models.join(", ")),
    };
    // DSH resolves its own executable through the adapter seam, so
    // its version and its composite describe one installation; every
    // other provider keeps the declared name and the bare probe.
    let observed = match adapter.provider.as_str() {
        "dsh" => composite(adapter, probe),
        _ => Observed {
            binary: adapter.binary.clone(),
            version: probe(&adapter.binary),
            warning: false,
            suffix: String::new(),
            cause: None,
        },
    };
    match &observed.version {
        Some(version) => {
            availability.record(&adapter.provider, Presence::Available);
            let detail = match observed.suffix.is_empty() {
                true => format!("{version} · {serves}"),
                false => format!("{version} · {serves} · {}", observed.suffix),
            };
            if observed.warning {
                report.warn(&adapter.provider, detail);
            } else {
                report.ok(&adapter.provider, detail);
            }
        }
        // Selected, and deliberately NOT probed: the refusal that
        // stopped the probe rides in the suffix. It is not a missing
        // binary, and saying so would send an operator holding a
        // malformed pnpm lock to reinstall an executable that is
        // there; nothing is recorded about an availability that was
        // never asked.
        None if !observed.suffix.is_empty() => {
            let binary = Safe::new(&observed.binary).as_str().to_string();
            report.warn(
                &adapter.provider,
                format!(
                    "binary '{binary}' selected and not probed · {serves} · {}",
                    observed.suffix
                ),
            );
        }
        None => {
            availability.record(&adapter.provider, Presence::Unavailable);
            report_not_found(report, adapter, observed, &serves);
        }
    }
}

/// The line for a provider whose binary was not found, with the cause
/// selection gave and the operator's hint.
fn report_not_found(report: &mut Report, adapter: &Adapter, observed: Observed, serves: &str) {
    // The advice, where the operator wrote one, comes from
    // the adapter file: it belongs beside the binary name,
    // not in a Rust constant that needs a release to fix.
    let hint = match &adapter.hint {
        Some(hint) => format!(" ({hint})"),
        None => String::new(),
    };
    // The binary is an operator's override or a seam's
    // selection and the cause came from a filesystem lookup:
    // neither is output doctor authored, so both go through
    // `Safe` at this, their only interpolation. Rendered raw,
    // a nonexistent override carrying a newline and a
    // clear-screen sequence reached the terminal verbatim
    // (security hold 2026-09-20, S2).
    let binary = Safe::new(&observed.binary).as_str().to_string();
    let cause = match observed.cause {
        Some(cause) => format!(": {}", Safe::new(&cause).as_str()),
        None => String::new(),
    };
    report.warn(
        &adapter.provider,
        format!(
            "binary '{binary}' not found{cause} — seats resolving to this \
             provider will fail to spawn{hint} · {serves}"
        ),
    );
}
