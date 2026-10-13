//! A bound plan's one lifetime and its ledger (decision 0065 slice two,
//! U6d, U6e; CR3, MB3, MB4): the ledger is created beside the plan before
//! anything of the plan is checked further or started, handed to the
//! session for its calls' records, and closed once the session has ended,
//! Failed with its first cause: admission's refusal, or the fatal ending
//! of a server it started. An admitted plan whose server initialized has
//! SD3's incomplete serving for its cause, with which `serve` then
//! refuses, so no session yet closes Clean. The bindings a session looks
//! up are given to its ledger as soon as they are known (U6f1a).

use std::cell::OnceCell;
use std::ffi::OsStr;
use std::os::fd::OwnedFd;
use std::path::Path;
use std::time::Instant;

use brokkr_protocol::broker::ledger::{Failure, Ledger, Unrecorded};
use brokkr_protocol::broker::{Owner, Refusal};
use brokkr_protocol::secret::BoundSecret;

use super::{bound, observed, readiness, screened, Admitted, Startup, STARTUP};
use crate::broker::rpc::Fatal;

/// The plan `locator` names, bound to this attempt at `digest` and checked
/// in MB3's refusal order, its box's sources observed afresh by the
/// private observer and compared with what the plan sealed, then the box
/// launched on its waiting bootstrap, its readiness checked, its bindings
/// looked up, given to its ledger ([`Binder`]) and handed over, its
/// server's exec seen (U6c8) and the server initialized (U6e), all within
/// the startup deadline unless the attempt is cancelled, which ends the
/// broker's own starter; or the first cause. Once the plan is bound, all of
/// it runs inside its ledger's one lifetime ([`lived`]). The box is ended
/// before this returns, before any message reaches the harness.
pub(in crate::broker) fn admit(locator: &Path, digest: &str) -> Result<Admitted, Ended> {
    let deadline = Instant::now() + STARTUP;
    let parent = rustix::process::getppid();
    let cancelled = || rustix::process::getppid() != parent;
    let startup: Startup<'_> = (deadline, &cancelled);
    let (layout, plan, attempt) = bound(locator, digest)?;
    let ledger = (&attempt, layout.named(0));
    lived(ledger, (&plan.owner, digest), |binder| {
        screened(&plan)?;
        let (record, handles) = observed(&layout, &plan, (locator, digest), startup)?;
        let admitted = readiness::compared(&plan.intent, record, handles)?;
        let served = crate::broker::rpc::initialized(&plan, startup);
        let bind = |found| binder.bind(found);
        readiness::ready(&admitted, (&plan, digest), startup, bind, served)?;
        Ok(admitted)
    })
}

/// Why `serve` ends before serving: its plan refused, its ledger not
/// recorded, or its server's session ended, in the protocol's own words.
#[derive(Debug, PartialEq)]
pub(in crate::broker) enum Ended {
    Refused(Refusal),
    Unrecorded(Unrecorded),
    Failed(Fatal),
}

impl std::fmt::Display for Ended {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ended::Refused(refusal) => refusal.fmt(formatter),
            Ended::Unrecorded(unrecorded) => unrecorded.fmt(formatter),
            Ended::Failed(fatal) => fatal.fmt(formatter),
        }
    }
}

impl std::error::Error for Ended {}

impl From<Refusal> for Ended {
    fn from(refusal: Refusal) -> Ended {
        Ended::Refused(refusal)
    }
}

impl From<Unrecorded> for Ended {
    /// A record the ledger did not make: one its final guard refused ends
    /// the session with the masking cause, which the ledger latched, on
    /// every path (MB4, CR2); any other is unrecorded.
    fn from(unrecorded: Unrecorded) -> Ended {
        match unrecorded {
            Unrecorded::Masking => Ended::Failed(Fatal::Failed(Failure::Masking)),
            Unrecorded::Reopened | Unrecorded::Record | Unrecorded::Limit => {
                Ended::Unrecorded(unrecorded)
            }
        }
    }
}

impl Ended {
    /// The cause a session that ended so latches: its refusal, or its
    /// fatal ending's; none where a record could not be made, which latches
    /// nothing here: the ledger latched its own limit where that was why,
    /// and after an append that failed it records nothing, its closure
    /// included. A record the ledger refused before appending it leaves
    /// the closure to what is latched; the call path's own order makes
    /// none so.
    fn failure(&self) -> Option<Failure> {
        match self {
            Ended::Refused(refusal) => Some(Failure::Refused(refusal.clone())),
            Ended::Failed(fatal) => Some(fatal.failure()),
            Ended::Unrecorded(_) => None,
        }
    }
}

/// `session` run inside the ledger of the plan named `plan` in the held
/// `attempt` directory, the plan sealed at `digest` for `owner`, and handed
/// that ledger's [`Binder`]: a ledger that cannot be opened ends the broker
/// before anything starts, a second lifetime's included. The session's
/// ending is then [`settled`] in the ledger, and ends the broker whether or
/// not the closure is recorded. The ledger opens before any binding is
/// looked up (CR3), so its Opened, which holds only the plan's owner and
/// digest, is guarded by none as it is written. The session gives the
/// bindings it looks up to the binder as soon as they are known, which
/// keeps them for the ledger's lifetime and guards the Opened's very line
/// by them then, and every record after, its Closed included (MB4, CR2;
/// operator ruling 2026-10-13).
pub(super) fn lived<T>(
    (attempt, plan): (&OwnedFd, &OsStr),
    sealed: (&Owner, &str),
    session: impl FnOnce(Binder<'_, '_>) -> Result<T, Ended>,
) -> Result<T, Ended> {
    let kept = OnceCell::new();
    let opened = Ledger::open(attempt, plan, sealed, &[]);
    let mut ledger = opened.map_err(Ended::from)?;
    let ended = session(Binder {
        ledger: &mut ledger,
        kept: &kept,
    });
    settled(ledger, ended)
}

/// A session's one hand on its ledger before serving: where the bindings
/// it looks up are kept while the ledger lives, and the ledger they guard.
pub(super) struct Binder<'l, 'b> {
    ledger: &'l mut Ledger<'b>,
    kept: &'b OnceCell<Vec<BoundSecret>>,
}

impl Binder<'_, '_> {
    /// Guard the ledger by `bindings` from now on (`Ledger::bind`): its
    /// Opened re-read by them, and where that refuses, the masking cause,
    /// which the ledger latched and its closure then meets too.
    pub(super) fn bind(self, bindings: Vec<BoundSecret>) -> Result<(), Ended> {
        let kept = self.kept.get_or_init(|| bindings);
        self.ledger.bind(kept).map_err(Ended::from)
    }
}

/// Close `ledger` once its session has `ended`: Failed with the session's
/// first cause, SD3's where it ended admitted; and how the session ended.
/// Where the closure is not recorded, recovery finds only the verified
/// prefix, and never a success, and the session ends as it did, unless the
/// ledger's final guard refused the closure, which ends it with the masking
/// cause (MB4, CR2).
pub(in crate::broker) fn settled<T>(
    mut ledger: Ledger<'_>,
    ended: Result<T, Ended>,
) -> Result<T, Ended> {
    let cause = match &ended {
        Ok(_) => Some(Failure::Refused(Refusal::ServingIncomplete)),
        Err(ended) => ended.failure(),
    };
    if let Some(cause) = cause {
        ledger.fail(cause);
    }
    match ledger.close() {
        Err(Unrecorded::Masking) => Err(Unrecorded::Masking.into()),
        Ok(_) | Err(Unrecorded::Reopened | Unrecorded::Record | Unrecorded::Limit) => ended,
    }
}

#[cfg(test)]
mod tests;
