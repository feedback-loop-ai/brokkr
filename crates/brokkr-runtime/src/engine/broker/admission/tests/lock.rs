//! The admission lock's refusal says why (#1082): an open that met an
//! errno names it, and a file that fails a clause of `lock_file::judged`
//! names the clause, whether admission made the lock or a readout found
//! it. A lock this engine cannot prove its own refuses for every cause.

use std::os::unix::fs::symlink;

use rustix::io::Errno;

use super::*;
use crate::engine::broker::root::locked_existing;
use crate::lock_file::tests::exhausted;

/// The refusal `unadmitted` is, as its unknown cause.
fn cause(unadmitted: Result<Held, Unadmitted>) -> Unknown {
    match unadmitted.map(drop) {
        Err(Unadmitted::Identity(unknown)) => unknown,
        other => panic!("admission was not refused by its identity: {other:?}"),
    }
}

/// Both takes of the lock in `host`'s root refuse with `why`: the
/// admission that makes the lock, and the readout that only finds it.
fn refused_as(host: &Host, incoming: &Claim, why: Unopened) {
    let refusal = Unknown::Lock(why);
    assert_eq!(cause(host.admit(incoming)), refusal);
    let found = locked_existing(host.root.dir(), soon());
    assert_eq!(found.map(drop), Err(refusal));
}

/// Each clause a lock file can fail, planted in the protected root:
/// a directory, a symlink, a file its group may read, a second link. Each
/// refuses admission and the readout alike, by its own words after
/// today's sentence, and the lock admits again once it is this user's own
/// single file. Another user's file needs a uid the box cannot take, so
/// that clause is planted by judging a file of this user's as another's, in
/// lock_file's tests.
#[test]
fn a_lock_that_fails_a_clause_refuses_by_that_clause() {
    let host = Host::new();
    let work = host.worktree("work");
    let incoming = open(&work, ["run-a", "a1"]);
    let lock = host.root().join(LOCK);

    std::fs::create_dir(&lock).unwrap();
    refused_as(&host, &incoming, Unopened::Open(Errno::ISDIR));
    std::fs::remove_dir(&lock).unwrap();

    symlink(host.dir.path(), &lock).unwrap();
    refused_as(&host, &incoming, Unopened::Open(Errno::LOOP));
    std::fs::remove_file(&lock).unwrap();

    std::fs::write(&lock, b"").unwrap();
    std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o640)).unwrap();
    refused_as(&host, &incoming, Unopened::Mode(0o640));
    std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o600)).unwrap();

    let second = host.dir.path().join("second-link");
    std::fs::hard_link(&lock, &second).unwrap();
    refused_as(&host, &incoming, Unopened::Links(2));
    let words = host.admit(&incoming).unwrap_err().to_string();
    assert!(
        words.ends_with("the admission lock is not this user's own single file: it has 2 links"),
        "{words}"
    );
    std::fs::remove_file(&second).unwrap();

    freed(&host.admit(&incoming).unwrap());
}

/// An open that meets EMFILE refuses admission and the readout by that
/// errno, and not as a lock that is another user's or has a second link,
/// which it reads as in the words of the refusal before #1082. It plays
/// in an engine of its own, its open-file limit lowered so it is this
/// test's and not the host's beside parallel tests.
#[test]
fn an_open_that_meets_emfile_refuses_by_its_errno() {
    if !in_its_own_engine(
        "engine::broker::admission::tests::lock::an_open_that_meets_emfile_refuses_by_its_errno",
    ) {
        return;
    }
    let host = Host::new();
    let work = host.worktree("work");
    freed(&host.admit(&open(&work, ["run-a", "a1"])).unwrap());
    let incoming = open(&work, ["run-a", "a2"]);

    let held = exhausted();

    let admitted = host.admit(&incoming);
    let found = locked_existing(host.root.dir(), soon());
    let words = admitted.as_ref().map(drop).unwrap_err().to_string();
    drop(held);
    assert_eq!(cause(admitted), Unknown::Lock(Unopened::Open(Errno::MFILE)));
    assert_eq!(
        found.map(drop),
        Err(Unknown::Lock(Unopened::Open(Errno::MFILE)))
    );
    assert!(
        words.ends_with(
            "the admission lock is not this user's own single file: its open failed with EMFILE"
        ),
        "{words}"
    );
    freed(&host.admit(&incoming).unwrap());
}
