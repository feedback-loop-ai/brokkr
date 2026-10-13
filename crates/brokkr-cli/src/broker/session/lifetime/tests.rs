//! A bound plan's lifetime given its bindings as soon as they are known
//! (U6f1a; MB4, CR2; operator ruling 2026-10-13): from then on its ledger's
//! every record is guarded by them, the Opened already written re-read and
//! the Closed a later cause would write included.

use std::ffi::OsStr;
use std::os::fd::OwnedFd;

use brokkr_protocol::broker::ledger::{recover, Disposition, Failure, Invalid, Outcome};
use brokkr_protocol::broker::Owner;
use brokkr_protocol::secret::{self, BoundSecret};

use super::{lived, Ended};
use crate::broker::rpc::Fatal;

/// The plan digest every case's ledger opens for.
const DIGEST: &str = "5ea1ed0000000000000000000000000000000000000000000000000000000000";

/// The owner every case's ledger opens for.
fn owner() -> Owner {
    let named = |name: &str| name.to_owned();
    Owner {
        repo: "b".repeat(64),
        run: named("r"),
        effect: named("e"),
        attempt: named("a"),
        site: named("s"),
        instance: named("s#0"),
    }
}

/// The plan's one binding, STATE, looked up as `value` from an owner-only
/// store on a canonical temporary root.
fn state(value: &str) -> Vec<BoundSecret> {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join("store.env");
    secret::store_set(&path, "STATE", value).unwrap();
    let declared = [String::from("STATE")];
    secret::resolve_bindings(&path, &declared).unwrap()
}

/// What a recovered ledger proves: each call and its outcome, its closure
/// and why verification stopped, if it did.
type Proved = (
    Vec<(String, Option<Outcome>)>,
    Option<Disposition>,
    Option<Invalid>,
);

/// A lifetime whose session, once STATE is looked up as `value`, gives it
/// to its binder and then ends with the server's protocol cause, as a
/// startup's does: how the lifetime ended, and what its ledger proves.
fn ended_after_binding(value: &str) -> (Result<(), Ended>, Proved) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let attempt: OwnedFd = std::fs::File::open(&root).unwrap().into();
    let found = state(value);
    let ended = lived(
        (&attempt, OsStr::new("plan")),
        (&owner(), DIGEST),
        |binder| {
            binder.bind(found)?;
            Err(Ended::Failed(Fatal::Failed(Failure::Protocol)))
        },
    );
    let bytes = std::fs::read(root.join("plan.ledger")).unwrap();
    let (lifecycle, invalid) = recover(&bytes, &owner(), DIGEST);
    let proved = (
        lifecycle.calls().to_vec(),
        lifecycle.closed().cloned(),
        invalid,
    );
    (ended, proved)
}

#[test]
fn bindings_given_once_known_guard_the_closure_a_later_cause_writes() {
    // STATE=closed: the Closed the protocol cause would close with holds
    // it, so it is never appended, and the lifetime ends with the masking
    // cause; only the Opened verifies.
    let masking = || Err(Ended::Failed(Fatal::Failed(Failure::Masking)));
    let unclosed = (vec![], None, Some(Invalid::Lifecycle));
    assert_eq!(ended_after_binding("closed"), (masking(), unclosed));
    // STATE=opened: the Opened already written is re-read by it and
    // refused, so the masking cause is latched first, and the closure,
    // which holds no binding, carries it.
    let latched = Some(Disposition::Failed(Failure::Masking));
    assert_eq!(
        ended_after_binding("opened"),
        (masking(), (vec![], latched, None))
    );
    // The control: an unrelated binding leaves the protocol cause the
    // session's, and its closure.
    let protocol = Failure::Protocol;
    let closed = (vec![], Some(Disposition::Failed(protocol.clone())), None);
    let control = (Err(Ended::Failed(Fatal::Failed(protocol))), closed);
    assert_eq!(ended_after_binding("unrelated"), control);
}
