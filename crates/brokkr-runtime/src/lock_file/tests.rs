//! The shared lock-file home (decision 0095 ruling 8): a lock file opens
//! only as this user's own regular file of one link.

use std::os::unix::fs::PermissionsExt;

use rustix::fs::CWD;

use super::*;

/// A lock file is made owner-only where it is missing, and opens again as
/// it stands; a symlink, a second link, a file others may read, a
/// directory, or a file another user owns does not open as one.
#[test]
fn a_lock_file_opens_only_as_this_users_own_single_file() {
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join("x.lock");
    let made = opened(CWD, &lock).expect("made where it was missing");
    let mode = std::fs::metadata(&lock).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    assert!(single(&made, user()));
    assert!(!single(&made, user() + 1), "another user's");
    assert!(opened(CWD, &lock).is_some(), "opened again as it stands");

    let link = dir.path().join("link.lock");
    std::os::unix::fs::symlink(&lock, &link).unwrap();
    assert!(opened(CWD, &link).is_none(), "a symlink");
    let second = dir.path().join("second");
    std::fs::hard_link(&lock, &second).unwrap();
    assert!(opened(CWD, &lock).is_none(), "of two links");
    std::fs::remove_file(&second).unwrap();
    std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o640)).unwrap();
    assert!(opened(CWD, &lock).is_none(), "its group may read it");
    std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(opened(CWD, &lock).is_some(), "this user's alone again");
    let directory = dir.path().join("directory.lock");
    std::fs::create_dir(&directory).unwrap();
    assert!(opened(CWD, &directory).is_none(), "a directory");
}
