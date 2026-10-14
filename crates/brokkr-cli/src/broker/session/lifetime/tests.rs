//! A bound plan's lifetime given its bindings as soon as they are known
//! (U6f1a; MB4, CR2; operator ruling 2026-10-13): from then on its ledger's
//! every record is guarded by them, the Opened already written re-read and
//! the Closed a later cause would write included.

use std::cell::RefCell;
use std::ffi::OsStr;
use std::io::PipeReader;
use std::os::fd::OwnedFd;
use std::process::ExitCode;
use std::time::Duration;

use brokkr_protocol::broker::ledger::{recover, Disposition, Failure, Invalid, Outcome};
use brokkr_protocol::broker::Owner;
use brokkr_protocol::secret::{self, BoundSecret};
use rustix::io::Errno;

use super::{lived, Ended, Handed, Held};
use crate::broker::rpc::pieces::tests::{refusing, sink, within, Sink};
use crate::broker::rpc::pieces::Put;
use crate::broker::rpc::{pieces, Fatal, ATOMIC};
use crate::broker::session::stderr::{told_through, told_to, Said, Voice};

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
        &Held::default(),
        (&attempt, OsStr::new("plan")),
        (&owner(), DIGEST),
        &|_| Put::Taken,
        |binder| {
            binder.bind(Handed::Published(found))?;
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

/// A lifetime whose session, its bindings given to its binder and its
/// server's tail left with it, ends as `ended` says, its standard error a
/// sink that takes at most `most` bytes of each of `writes` writes, written
/// as `serve` writes it ([`told_to`]): the exit code, what the sink took,
/// and the closure its ledger proves.
fn reported(
    ended: fn() -> Result<(), Ended>,
    (most, writes): (usize, usize),
) -> (ExitCode, String, Option<Disposition>) {
    reported_to(ended, Some(sink(most, writes, Duration::ZERO)))
}

/// [`reported`], its standard error `stream`, or none where that could not
/// be set up, written as `serve` writes it then ([`told_through`]).
fn reported_to(
    ended: fn() -> Result<(), Ended>,
    stream: Option<(Sink, PipeReader)>,
) -> (ExitCode, String, Option<Disposition>) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let attempt: OwnedFd = std::fs::File::open(&root).unwrap().into();
    let (sink, reader) = stream.unzip();
    let sink = RefCell::new(sink);
    let tell = |said: &[Said]| match &mut *sink.borrow_mut() {
        Some(sink) => told_to(said, sink, within(Duration::from_secs(1))),
        None => told_through(Err(Errno::MFILE.into()), said, &|| false),
    };
    let held = Held::default();
    let sealed = (&owner(), DIGEST);
    let lifetime = lived(
        &held,
        (&attempt, OsStr::new("plan")),
        sealed,
        &tell,
        |binder| {
            let bound = binder.bind(Handed::Published(state("unrelated")))?;
            let line = Voice::new(bound.bindings).said(b"server says", "");
            bound.said.borrow_mut().extend(line);
            ended()
        },
    );
    let exit = held.reported(lifetime, &tell);
    drop(sink);
    let wrote = reader.map_or(Vec::new(), pieces::tests::held);
    let wrote = String::from_utf8(wrote).unwrap();
    let bytes = std::fs::read(root.join("plan.ledger")).unwrap();
    let (lifecycle, invalid) = recover(&bytes, &owner(), DIGEST);
    assert_eq!(invalid, None);
    (exit, wrote, lifecycle.closed().cloned())
}

#[test]
fn a_session_closes_clean_only_once_its_standard_error_took_every_piece_whole() {
    // A served session's server's tail is written before its ledger is
    // closed, while its bindings are held: taken whole, it closes Clean;
    // taken short, as a terminal or a file may take it, the session fails
    // and its ledger closes with the delivery cause, and nothing more is
    // written; where its standard error has closed its end, as a harness's
    // output may, it keeps its ending. A failed session keeps its first
    // cause, and its tail and last line follow the closure, a short write
    // failing its exit (U6f1b; operator rulings 2026-10-13, 2026-10-14).
    let served: fn() -> Result<(), Ended> = || Ok(());
    let failed: fn() -> Result<(), Ended> = || Err(Ended::Failed(Fatal::Failed(Failure::Protocol)));
    let (whole, short, closed) = ((ATOMIC, usize::MAX), (3, usize::MAX), (ATOMIC, 0));
    let tail = "server says\n";
    let last = format!("{tail}error: MCP server protocol is invalid\n");
    let failure = |cause| Some(Disposition::Failed(cause));
    let (success, exit_failed) = (ExitCode::SUCCESS, ExitCode::from(1));
    for (case, ended, sink, expected) in [
        (
            "served",
            served,
            whole,
            (success, tail, Some(Disposition::Clean)),
        ),
        (
            "served",
            served,
            short,
            (exit_failed, "ser", failure(Failure::Undelivered)),
        ),
        (
            "served",
            served,
            closed,
            (success, "", Some(Disposition::Clean)),
        ),
        (
            "failed",
            failed,
            whole,
            (exit_failed, &*last, failure(Failure::Protocol)),
        ),
        (
            "failed",
            failed,
            short,
            (exit_failed, "ser", failure(Failure::Protocol)),
        ),
    ] {
        let (code, wrote, closure) = expected;
        let expected = (code, wrote.to_owned(), closure);
        assert_eq!((case, sink, reported(ended, sink)), (case, sink, expected));
    }
}

#[test]
fn only_a_closed_standard_error_keeps_a_served_sessions_ending() {
    // Its standard error refusing every write: a broken pipe, its reader
    // gone, keeps the served session's ending and closes Clean; a full
    // device, an I/O error, a write that takes nothing or a stream that
    // cannot be set up is the delivery failure, the session failed and its
    // ledger closed with that cause. A failed session keeps its first
    // cause, its exit failing either way (operator ruling 2026-10-14).
    let served: fn() -> Result<(), Ended> = || Ok(());
    let failed: fn() -> Result<(), Ended> = || Err(Ended::Failed(Fatal::Failed(Failure::Protocol)));
    let (success, exit_failed) = (ExitCode::SUCCESS, ExitCode::from(1));
    let undelivered = Some(Disposition::Failed(Failure::Undelivered));
    let protocol = Some(Disposition::Failed(Failure::Protocol));
    let streams = || {
        [
            ("EPIPE", Some(refusing(Errno::PIPE))),
            ("ENOSPC", Some(refusing(Errno::NOSPC))),
            ("EIO", Some(refusing(Errno::IO))),
            ("zero", Some(sink(0, usize::MAX, Duration::ZERO))),
            ("set-up", None),
        ]
    };
    for (case, stream) in streams() {
        let expected = match case {
            "EPIPE" => (success, Some(Disposition::Clean)),
            _ => (exit_failed, undelivered.clone()),
        };
        let (code, wrote, closure) = reported_to(served, stream);
        assert_eq!(
            (case, code, wrote, closure),
            (case, expected.0, String::new(), expected.1)
        );
    }
    for (case, stream) in streams() {
        let (code, wrote, closure) = reported_to(failed, stream);
        let expected = (case, exit_failed, String::new(), protocol.clone());
        assert_eq!((case, code, wrote, closure), expected);
    }
}
