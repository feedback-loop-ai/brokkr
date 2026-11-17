//! A seat's result admitted as a verdict's record (decision 0109 slice
//! 0109a1): the findings a seat reports are stamped with the digest of
//! the compiled site that reported them, before the result is joined
//! into a panel's aggregate or journaled, and a result the admission
//! refuses fails its attempt with the refusal named, never repaired
//! (decision 0001). No word is refused here and no routing moves.

use brokkr_protocol::{AttemptOutcome, AttemptReport};

use super::resume::SiteContext;

/// `report` with its successful result admitted at `site`, the site whose
/// driver produced it: its findings stamped in place, or the refusal put
/// in place of the outcome acted on, as a refused checkpoint's is
/// (decision 0034 ruling 6), so the attempt fails, is retried and parks
/// under decision 0006. A report already refused is left as it stands.
pub(super) fn admitted(mut report: AttemptReport, site: Option<&SiteContext>) -> AttemptReport {
    if report.refused.is_some() {
        return report;
    }
    let AttemptOutcome::Succeeded { result } = &mut report.outcome else {
        return report;
    };
    let site = site.map(|site| site.site_ref.as_str());
    if let Err(refusal) = brokkr_core::findings::admit(result, site) {
        report.refused = Some(AttemptOutcome::Failed {
            error: format!("the result was refused: {refusal}"),
        });
    }
    report
}

#[cfg(test)]
mod tests;
