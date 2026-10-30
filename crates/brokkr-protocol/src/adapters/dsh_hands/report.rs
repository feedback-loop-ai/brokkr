//! What dsh says about the seat it just ran (#896 round 2, D0 and D4): the
//! startup line that names a row which did not activate, the sandbox
//! report that says no backend could confine a call, and the launcher's
//! own partial-enforcement line.
//!
//! The measured 0.2.1-rc.2 behaviour is why this exists: a hands row that
//! cannot start its server leaves dsh running and exiting 0, printing
//! `dsh: warning: 1 entry did not activate` and the row's own error. So a
//! hand that did not activate fails the seat closed here, with the row
//! named, rather than being served a session with no workspace tool.
//! `SANDBOX_UNAVAILABLE` is dsh's own structured refusal to run
//! unconfined; a report that it happened is a refusal, and the launcher's
//! `partial enforcement` line is carried to the operator instead of being
//! dropped. The exact words are dsh-sandbox's and dsh-sandbox-local's
//! (docs/evidence/adapters/dsh-hands-round2/report-words.txt).

use super::super::dsh_stderr::before_reasoning;
use super::{HANDS_ID, MCP_CLIENT};
use thiserror::Error;

/// dsh-sandbox's structured error code for a host with no usable backend.
const SANDBOX_UNAVAILABLE: &str = "SANDBOX_UNAVAILABLE";

/// The tail of that error's message, bound beside the code.
const UNCONFINED: &str =
    "no sandbox backend is usable on this host; refusing to run the command unconfined";

/// The Landlock launcher's line on an older ABI, as dsh-sandbox-local
/// reports it at every confined run. It is carried on the finished row's
/// `step_name`, the contract's admitted text, while `sandbox` keeps the
/// mode (#896 round 3).
pub(super) const PARTIAL: &str = "landlock-run: partial enforcement (older Landlock ABI)";

/// How much of dsh's own sandbox dsh reported. `Partial` is recorded on the
/// seat's finished row; `Full` adds nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Enforcement {
    Full,
    Partial,
}

/// Why the seat fails closed on the startup dsh reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum ReportRefusal {
    #[error(
        "refusing the dsh seat: the hands row '{HANDS_ID}' did not activate, so the seat's one \
         effectful tool was never served; dsh ran the session anyway (#896 D0)"
    )]
    NotActivated,
    #[error(
        "refusing the dsh seat: dsh reported {SANDBOX_UNAVAILABLE} (no sandbox backend is usable \
         on this host), so a call it could not confine is never admitted (#896 D4)"
    )]
    SandboxUnavailable,
}

/// Read the stderr dsh wrote before its session: an unavailable sandbox
/// fails closed, a hands entry that did not activate fails closed, and
/// partial enforcement is reported to the caller. The model's reasoning is
/// session content, so it is never startup evidence (#896 M1; decision
/// 0098 ruling 4).
pub(crate) fn checked(stderr: &str) -> Result<Enforcement, ReportRefusal> {
    let startup = before_reasoning(stderr);
    if startup.contains(SANDBOX_UNAVAILABLE) || startup.contains(UNCONFINED) {
        return Err(ReportRefusal::SandboxUnavailable);
    }
    // The hands row's own failure prefix, built once from the one spelling
    // of the row id and the package; each line is matched against it
    // without allocating again.
    let hands_error = format!("{HANDS_ID} ({MCP_CLIENT}): Error:");
    if startup
        .lines()
        .any(|line| did_not_activate(line, &hands_error))
    {
        return Err(ReportRefusal::NotActivated);
    }
    match startup.contains(PARTIAL) {
        true => Ok(Enforcement::Partial),
        false => Ok(Enforcement::Full),
    }
}

/// dsh's warning that some row did not activate, or the hands row's own
/// failure line beneath it.
fn did_not_activate(line: &str, hands_error: &str) -> bool {
    let warning = line.strip_prefix("dsh: warning: ").is_some_and(|rest| {
        let Some((count, tail)) = rest.split_once(' ') else {
            return false;
        };
        let count = count.parse::<u32>().is_ok_and(|count| count > 0);
        count && matches!(tail, "entry did not activate" | "entries did not activate")
    });
    warning
        || (line.starts_with(hands_error)
            && line.contains("initial connection or tool synchronization failed"))
}
