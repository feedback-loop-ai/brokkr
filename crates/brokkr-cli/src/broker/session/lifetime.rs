//! A bound plan's one lifetime and its ledger (decision 0065 slice two,
//! U6d, U6e, U6f1b; CR3, MB3, MB4): the ledger is created beside the plan
//! before anything of the plan is checked further or started, given a
//! retaining plan's staging (U8b), handed to the session for its calls'
//! records, and closed once the session has ended: Failed with its first
//! cause, admission's refusal or the fatal ending of a server it started;
//! Clean only where the harness was served until it ended, nothing was
//! latched, and every byte the broker wrote on its standard output and its
//! standard error, its server's last lines among them, was taken whole, or
//! the stream's reader closed its end, seen as a broken pipe; else the
//! delivery failure (operator rulings 2026-10-14). The bindings a
//! session looks up are given to its ledger, once, as soon as they are
//! known (U6f1a, U6f1b), and kept past it ([`Held`]): what `serve` writes
//! to its own standard error is passed by them, a served session's
//! server's tail before its ledger is closed, a failed session's tail and
//! last line after, and only then are they dropped. Where they cannot be
//! known, the ledger records nothing more, and nothing is written.

use std::cell::{Cell, OnceCell, RefCell};
use std::ffi::OsStr;
use std::os::fd::OwnedFd;
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use brokkr_protocol::broker::ledger::{Failure, Ledger, Unrecorded};
use brokkr_protocol::broker::{Owner, Refusal};
use brokkr_protocol::secret::BoundSecret;

use super::readiness::Handed;
use super::retaining::{self, Retention};
use super::stderr::{self, Said};
use super::{bound, observed, readiness, screened, Startup, STARTUP};
use crate::broker::rpc::pieces::Put;
use crate::broker::rpc::{self, Fatal};
use crate::broker::Signalled;
use crate::exit::Exit;

/// The plan `locator` names, bound to this attempt at `digest` and checked
/// in MB3's refusal order, its box's sources observed afresh by the
/// private observer and compared with what the plan sealed, then the box
/// launched on its waiting bootstrap, its readiness checked, its bindings
/// looked up, given to its ledger ([`Binder`]) and handed over, its
/// server's exec seen (U6c8) and the server initialized (U6e), all within
/// the startup deadline unless the attempt is cancelled, which ends the
/// broker's own starter; then the harness served over the broker's own
/// stdio until it ends ([`rpc::serving`]); or the first cause. Once the
/// plan is bound, all of it runs inside its ledger's one lifetime
/// ([`lived`]). The box is ended before the lifetime is; a served
/// session's server's tail is written before its ledger closes
/// ([`Held::delivered`]), and what the broker says of a failed one after,
/// last, while its bindings are held ([`Held::reported`]): the exit code.
/// A harness that ends the session by a signal (U9b1e) cancels its startup,
/// which ends with that stage's own cause, or hangs up on its serving as a
/// closed input does ([`rpc::Ending`]); a second signal then cancels what
/// that ending still waits for, as the starter's end does.
pub(in crate::broker) fn serve(locator: &Path, digest: &str, signalled: &Signalled) -> ExitCode {
    let deadline = Instant::now() + STARTUP;
    let parent = rustix::process::getppid();
    let orphaned = || rustix::process::getppid() != parent;
    let cancelled = || orphaned() | signalled.at_least(1);
    let cut = || orphaned() | signalled.at_least(2);
    let hung_up = || signalled.at_least(1);
    let startup: Startup<'_> = (deadline, &cancelled);
    let tell = |said: &[Said]| stderr::told(said, &cut);
    let held = Held::default();
    let ended = bound(locator, digest).map_err(Ended::from);
    let ended = ended.and_then(|(layout, plan, attempt)| {
        let retention = retaining::retention(&plan, &attempt, layout.attempt)?;
        let ledger = (&attempt, layout.named(0), retention);
        lived(&held, ledger, (&plan.owner, digest), &tell, |binder| {
            screened(&plan)?;
            let (record, handles) = observed(&layout, &plan, (locator, digest), startup)?;
            let admitted = readiness::compared(&plan.intent, record, handles)?;
            let bind = |found| binder.bind(found);
            readiness::ready(
                &admitted,
                (&plan, digest),
                startup,
                bind,
                |link, errors, bound| {
                    let ending: rpc::Ending<'_> = (&cut, &hung_up);
                    rpc::serving(link, errors, bound, (&plan, startup, ending))
                },
            )
        })
    });
    held.reported(ended, &tell)
}

/// How the broker's own standard error is written: each call a writing of
/// its own, [`stderr::told`]'s, or a test's in its place. What became of
/// the writing.
pub(super) type Tell<'t> = &'t dyn Fn(&[Said]) -> Put;

/// Why `serve` ends unserved: its plan refused, its ledger not recorded,
/// or its server's session ended, in the protocol's own words.
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
/// before anything starts, a second lifetime's included. A session that
/// ended served first has what it left to say written by `tell`
/// ([`Held::delivered`]); its ending is then [`settled`] in the ledger,
/// and ends the broker whether or not the closure is recorded. The ledger
/// opens before any binding is looked up (CR3), so its Opened, which holds
/// only the plan's owner and digest, is guarded by none as it is written.
/// The session gives the bindings it looks up to the binder as soon as
/// they are known, which keeps them in `held`, past the ledger, and guards
/// the Opened's very line by them then, and every record after, its Closed
/// included (MB4, CR2; operator ruling 2026-10-13). Where what the session
/// looked up is uncertain, nothing more is recorded, its closure included:
/// recovery finds the Opened alone, and never a success. A plan that
/// retains its responses is given its `retention`, so its ledger stages
/// them beside it (U8b).
pub(super) fn lived<'b, T>(
    held: &'b Held,
    (attempt, plan, retention): (&OwnedFd, &OsStr, Option<Retention>),
    sealed: (&Owner, &str),
    tell: Tell<'_>,
    session: impl FnOnce(Binder<'_, 'b>) -> Result<T, Ended>,
) -> Result<T, Ended> {
    let opened = Ledger::open(attempt, plan, sealed, &[]);
    let mut ledger = opened.map_err(Ended::from)?;
    if let Some((retaining, dir)) = retention {
        ledger.retain(dir, plan, retaining);
    }
    let ended = session(Binder {
        ledger: &mut ledger,
        held,
    });
    match held.withheld.get() {
        true => ended,
        false => settled(ledger, held.delivered(ended, tell)),
    }
}

/// What a lifetime holds past its ledger, until `serve` has written its
/// last byte: the bindings its session looked up, kept once known; whether
/// what it looked up is uncertain, which withholds every record and line
/// they would guard; the lines its server's standard error left, passed by
/// them ([`Said`]); and whether its standard error already stopped taking
/// them, so nothing more is written there.
#[derive(Default)]
pub(super) struct Held {
    kept: OnceCell<Vec<BoundSecret>>,
    withheld: Cell<bool>,
    said: RefCell<Vec<Said>>,
    stopped: Cell<bool>,
}

impl Held {
    /// How a session that `ended` served leaves before its ledger is
    /// closed: every line its server's standard error left written by
    /// `tell` while the bindings that passed them are held, its standard
    /// output already taken whole as it served ([`rpc::serving`]); served
    /// where its standard error took every piece whole, or a write met a
    /// broken pipe, its reader having closed its end, as a harness's output
    /// may, which alone keeps its ending; else, a write or poll error, a
    /// write that took nothing or part of a piece, a stream that could not
    /// be set up, or its bound passed, the delivery failure, and nothing
    /// more is written there (operator rulings 2026-10-14). A session that
    /// failed keeps its first cause, and what it says is written once its
    /// ledger is closed ([`Held::reported`]).
    fn delivered<T>(&self, ended: Result<T, Ended>, tell: Tell<'_>) -> Result<T, Ended> {
        let served = ended?;
        match tell(&self.said.take()) {
            Put::Taken | Put::Closed => Ok(served),
            Put::Failed | Put::Short | Put::Late | Put::Unready => {
                self.stopped.set(true);
                Err(Ended::Failed(rpc::UNDELIVERED))
            }
        }
    }

    /// How `serve` leaves once its lifetime has `ended`, the ledger closed:
    /// what the server's standard error left that is not yet written, and
    /// the broker's own last line, passed by the kept bindings, or by none
    /// where none were looked up ([`stderr::last`]), written by `tell`, in
    /// production to its standard error within their bound on this thread
    /// while the attempt is not cancelled ([`stderr::told`]), and only once
    /// every write has returned the bindings dropped; nothing where they
    /// are uncertain, or where its standard error already stopped taking
    /// what it was given ([`Held::delivered`]). The session's exit code, or
    /// a failure's where a write took a piece short or nothing, or erred
    /// otherwise than by a broken pipe, or the stream could not be set up,
    /// which ends the session as a delivery failure, however it ended
    /// before (operator rulings 2026-10-13, 2026-10-14).
    fn reported(self, ended: Result<(), Ended>, tell: Tell<'_>) -> ExitCode {
        let bindings = self.kept.get().map_or(&[][..], Vec::as_slice);
        let known = (!self.withheld.get()).then_some(bindings);
        let (code, last) = stderr::last(ended, known);
        if self.stopped.get() {
            return code;
        }
        let mut said = self.said.take();
        said.extend(last);
        match tell(&said) {
            Put::Short | Put::Failed => Exit::Failed.into(),
            Put::Taken | Put::Unready | Put::Late | Put::Closed => code,
        }
    }
}

/// A session's one hand on its ledger before serving: the ledger, and what
/// its lifetime holds of it ([`Held`]). It is spent binding them, so they
/// are given once.
pub(super) struct Binder<'l, 'b> {
    ledger: &'l mut Ledger<'b>,
    held: &'b Held,
}

/// A session's ledger once its bindings guard it, those bindings, which
/// serving masks everything it lets out by, and where it leaves the lines
/// of its server's standard error it kept for the broker's own.
pub(in crate::broker) struct Bound<'l, 'b> {
    pub(in crate::broker) ledger: &'l mut Ledger<'b>,
    pub(in crate::broker) bindings: &'b [BoundSecret],
    pub(in crate::broker) said: &'b RefCell<Vec<Said>>,
}

impl<'l, 'b> Binder<'l, 'b> {
    /// Guard the ledger by the bindings `handed` from now on
    /// (`Ledger::bind`), none where none were looked up: its Opened re-read
    /// by them, and where that refuses, the masking cause, which the ledger
    /// latched and its closure then meets too; else the ledger they guard,
    /// and them. Where what was looked up is uncertain, no guard can hold,
    /// so the ledger's records are withheld from now on, its closure
    /// included, and so is every line the broker would write, and the
    /// session ends with the handoff cause.
    pub(super) fn bind(self, handed: Handed<Vec<BoundSecret>>) -> Result<Bound<'l, 'b>, Ended> {
        let bindings = match handed {
            Handed::Published(bindings) => bindings,
            Handed::Nothing => Vec::new(),
            Handed::Uncertain => {
                self.held.withheld.set(true);
                return Err(Refusal::Handoff.into());
            }
        };
        let kept = self.held.kept.get_or_init(|| bindings);
        self.ledger.bind(kept).map_err(Ended::from)?;
        Ok(Bound {
            ledger: self.ledger,
            bindings: kept,
            said: &self.held.said,
        })
    }
}

/// Close `ledger` once its session has `ended`: Failed with the session's
/// first cause; Clean where it ended served and no cause was latched
/// (U6f1b); and how the session ended. Where the closure is not recorded,
/// recovery finds only the verified prefix, and never a success, and the
/// session ends as it did, unless the ledger's final guard refused the
/// closure, which ends it with the masking cause (MB4, CR2).
pub(in crate::broker) fn settled<T>(
    mut ledger: Ledger<'_>,
    ended: Result<T, Ended>,
) -> Result<T, Ended> {
    if let Some(cause) = ended.as_ref().err().and_then(Ended::failure) {
        ledger.fail(cause);
    }
    match ledger.close() {
        Err(Unrecorded::Masking) => Err(Unrecorded::Masking.into()),
        Ok(_) | Err(Unrecorded::Reopened | Unrecorded::Record | Unrecorded::Limit) => ended,
    }
}

#[cfg(test)]
mod tests;
