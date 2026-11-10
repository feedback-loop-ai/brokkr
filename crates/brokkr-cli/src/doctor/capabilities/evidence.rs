//! Doctor's historical evidence lines (decision 0105 ruling 5): every
//! historical artifact root the host holds, by its path, whether it still
//! stands, and its retirement. A root whose artifact root is absent and
//! which is not retired is named as still staking the host, so an operator
//! sees why unboxed writers refuse; no line refuses, and none is keyed to
//! where a path lies (ruling 6).

use std::path::Path;
use std::time::{Duration, Instant};

use brokkr_runtime::engine::evidence::{self, EvidenceError};

use super::super::{safe, Report};

/// The most doctor waits for the coordination root's lock and inventory.
const DEADLINE: Duration = Duration::from_secs(30);

/// The `capabilities evidence` lines: each root's path, existence and
/// retirement, or why none could be read.
pub(super) fn report(report: &mut Report, home: Option<&Path>) {
    let Some(home) = home else {
        report.warn(
            "capabilities evidence",
            "this host names no HOME, so its historical evidence roots cannot be listed".into(),
        );
        return;
    };
    let roots = match evidence::list(home, Instant::now() + DEADLINE) {
        Ok(roots) => roots,
        Err(refused) => {
            report.warn("capabilities evidence", words(&refused));
            return;
        }
    };
    if roots.is_empty() {
        report.ok(
            "capabilities evidence",
            "no historical evidence root".into(),
        );
        return;
    }
    for root in &roots {
        let at = safe(&root.artifacts.display().to_string());
        let what = format!("capabilities evidence {at}");
        if let Some(problem) = &root.problem {
            report.warn(
                &what,
                format!(
                    "the historical evidence root's own counts cannot be read: {}",
                    safe(problem)
                ),
            );
            continue;
        }
        match (&root.retirement, root.exists) {
            (Some(retired), _) => report.ok(&what, format!("retired: {}", safe(&retired.words()))),
            (None, false) => report.warn(
                &what,
                "the historical artifact root is absent; it still stakes the host until retired, \
                 so unboxed writers refuse"
                    .into(),
            ),
            (None, true) => report.ok(
                &what,
                format!(
                    "{} digests, {} bytes; not retired",
                    root.digests, root.bytes
                ),
            ),
        }
    }
}

/// Why no root could be listed, in doctor's words, escaped for a terminal.
fn words(refused: &EvidenceError) -> String {
    format!(
        "the historical evidence roots could not be read: {}",
        safe(&refused.to_string())
    )
}

#[cfg(test)]
mod tests;
