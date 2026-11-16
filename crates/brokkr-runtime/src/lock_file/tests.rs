//! The shared lock-file home (decision 0095 ruling 8): a lock file opens
//! only as this user's own regular file of one link, and a lock that does
//! not open says why (#1082).

use std::os::unix::fs::PermissionsExt;

use rustix::fs::CWD;
use rustix::process::{getrlimit, setrlimit, Resource};

use super::*;

/// A lock file is made owner-only where it is missing, and opens again as
/// it stands; a symlink, a second link, a file others may read, a
/// directory, or a file another user owns does not open as one, and each
/// is refused by the errno or the clause that is its own.
#[test]
fn a_lock_file_opens_only_as_this_users_own_single_file() {
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join("x.lock");
    let made = open(CWD, &lock).expect("made where it was missing");
    let mode = std::fs::metadata(&lock).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    assert_eq!(judged(&made, user()), Ok(()));
    assert_eq!(
        judged(&made, user() + 1),
        Err(Unopened::Owner(user())),
        "another user's"
    );
    assert!(opened(CWD, &lock).is_some(), "opened again as it stands");

    let link = dir.path().join("link.lock");
    std::os::unix::fs::symlink(&lock, &link).unwrap();
    assert!(opened(CWD, &link).is_none(), "a symlink");
    assert_eq!(open(CWD, &link).unwrap_err(), Unopened::Open(Errno::LOOP));
    let second = dir.path().join("second");
    std::fs::hard_link(&lock, &second).unwrap();
    assert!(opened(CWD, &lock).is_none(), "of two links");
    assert_eq!(open(CWD, &lock).unwrap_err(), Unopened::Links(2));
    std::fs::remove_file(&second).unwrap();
    std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o640)).unwrap();
    assert!(opened(CWD, &lock).is_none(), "its group may read it");
    assert_eq!(open(CWD, &lock).unwrap_err(), Unopened::Mode(0o640));
    std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(opened(CWD, &lock).is_some(), "this user's alone again");
    let directory = dir.path().join("directory.lock");
    std::fs::create_dir(&directory).unwrap();
    assert!(opened(CWD, &directory).is_none(), "a directory");
    assert_eq!(
        open(CWD, &directory).unwrap_err(),
        Unopened::Open(Errno::ISDIR)
    );
}

/// A descriptor that is not a regular file is refused as such wherever it
/// came from: an open that found a directory or a device, which `open`
/// itself meets as an errno, is judged by `judged` as the clause it is.
#[test]
fn a_file_that_is_not_regular_is_judged_so() {
    let dir = tempfile::tempdir().unwrap();
    let directory = std::fs::File::open(dir.path()).unwrap();
    assert_eq!(judged(&directory, user()), Err(Unopened::NotRegular));
    let device = std::fs::File::open("/dev/null").unwrap();
    assert_eq!(judged(&device, user()), Err(Unopened::NotRegular));
}

/// Each cause reads in the words its refusal carries after its sentence:
/// the errno by name where it has one and by number where it has not, and
/// each clause by what it found.
#[test]
fn each_cause_reads_in_its_own_words() {
    let words = [
        (Unopened::Open(Errno::MFILE), "its open failed with EMFILE"),
        (Unopened::Open(Errno::NFILE), "its open failed with ENFILE"),
        (Unopened::Open(Errno::INTR), "its open failed with EINTR"),
        (Unopened::Open(Errno::NOMEM), "its open failed with ENOMEM"),
        (Unopened::Open(Errno::ACCESS), "its open failed with EACCES"),
        (Unopened::Open(Errno::PERM), "its open failed with EPERM"),
        (Unopened::Open(Errno::LOOP), "its open failed with ELOOP"),
        (Unopened::Open(Errno::ISDIR), "its open failed with EISDIR"),
        (
            Unopened::Open(Errno::NOTDIR),
            "its open failed with ENOTDIR",
        ),
        (Unopened::Open(Errno::NOSPC), "its open failed with ENOSPC"),
        (Unopened::Open(Errno::ROFS), "its open failed with EROFS"),
        (Unopened::Open(Errno::IO), "its open failed with EIO"),
        (Unopened::Open(Errno::BUSY), "its open failed with errno 16"),
        (
            Unopened::Status(Errno::IO),
            "its status could not be read: EIO",
        ),
        (Unopened::NotRegular, "it is not a regular file"),
        (Unopened::Links(2), "it has 2 links"),
        (Unopened::Owner(1001), "it is another user's, uid 1001"),
        (
            Unopened::Mode(0o640),
            "its mode 0640 lets its group or others in",
        ),
    ];
    for (cause, said) in words {
        assert_eq!(cause.to_string(), said);
    }
}

/// This process's open-file limit lowered and every descriptor under it
/// held, so the next open meets EMFILE until the descriptors are dropped.
/// Only a test in an engine of its own may call it: the limit is the whole
/// process's.
pub(crate) fn exhausted() -> Vec<std::fs::File> {
    let mut limit = getrlimit(Resource::Nofile);
    limit.current = Some(limit.current.map_or(64, |current| current.min(64)));
    setrlimit(Resource::Nofile, limit).unwrap();
    let mut held = Vec::new();
    let error = loop {
        match std::fs::File::open("/dev/null") {
            Ok(file) => held.push(file),
            Err(error) => break error,
        }
    };
    assert_eq!(error.raw_os_error(), Some(Errno::MFILE.raw_os_error()));
    assert!(!held.is_empty(), "no descriptor was free to exhaust");
    held
}

/// An open that meets EMFILE is refused by that errno, not as a lock that
/// is not this user's own (#1082). It plays in an engine of its own: the
/// process's open-file limit is lowered, so it is this test's and not the
/// host's beside parallel tests, and filled until an open meets EMFILE.
#[test]
fn an_open_that_meets_emfile_names_it() {
    if !brokkr_protocol::process::in_its_own_engine(
        "lock_file::tests::an_open_that_meets_emfile_names_it",
    ) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let held = exhausted();
    let refused = open(CWD, dir.path().join("x.lock")).unwrap_err();
    assert_eq!(refused, Unopened::Open(Errno::MFILE));
    assert_eq!(refused.to_string(), "its open failed with EMFILE");
    drop(held);
    assert!(
        open(CWD, dir.path().join("x.lock")).is_ok(),
        "one free again"
    );
}
