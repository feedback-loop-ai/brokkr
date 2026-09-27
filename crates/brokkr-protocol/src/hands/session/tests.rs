use super::*;

/// A pid no process holds: a child that has exited and been waited for.
fn dead_pid() -> u32 {
    let mut child = std::process::Command::new("true").spawn().unwrap();
    child.wait().unwrap();
    child.id()
}

/// A tree named as a session names it, planted without an owner.
fn planted(tmp: &Path, label: &str, pid: &str) -> PathBuf {
    let tree = tmp.join(format!("{PREFIX}{label}-{pid}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tree).unwrap();
    tree
}

#[test]
fn a_session_is_locked_while_it_lives_and_removed_when_it_drops() {
    let tmp = tempfile::tempdir().unwrap();
    let session = Session::create_in(tmp.path(), "serve").unwrap();
    let tree = session.path().to_path_buf();
    let name = tree.file_name().unwrap().to_str().unwrap();
    assert_eq!(
        owner_pid(name).map(|pid| pid.as_raw_nonzero().get()),
        Some(i32::try_from(std::process::id()).unwrap())
    );
    assert!(!unlocked(&tree), "the owner holds the lock");
    assert_eq!(reap_dead_sessions_in(tmp.path()), Vec::<PathBuf>::new());
    assert!(tree.is_dir());
    drop(session);
    assert!(!tree.exists(), "dropping the session removes its tree");
    assert_eq!(
        Session::create_in(&tmp.path().join("absent/\0"), "serve")
            .unwrap_err()
            .to_string(),
        "hands session: file name contained an unexpected NUL byte"
    );
}

#[test]
fn only_a_tree_whose_owner_is_dead_and_unlocked_is_reaped() {
    let tmp = tempfile::tempdir().unwrap();
    let dead = dead_pid().to_string();
    let mine = std::process::id().to_string();
    let unowned = planted(tmp.path(), "serve", &dead);
    let unheld = planted(tmp.path(), "exec", &dead);
    std::fs::write(unheld.join(LOCK), "").unwrap();
    // A live owner whose pid reads as dead here: its lock still holds.
    let held = Session::create_in(tmp.path(), "serve").unwrap();
    let foreign = tmp
        .path()
        .join(format!("{PREFIX}serve-{dead}-{}", uuid::Uuid::new_v4()));
    std::fs::rename(held.path(), &foreign).unwrap();
    // A lock that cannot be opened is not an answer.
    let unreadable = planted(tmp.path(), "serve", &dead);
    std::os::unix::fs::symlink(LOCK, unreadable.join(LOCK)).unwrap();
    let kept = [
        foreign,
        unreadable,
        planted(tmp.path(), "serve", &mine),
        planted(tmp.path(), "serve", "1"),
        planted(tmp.path(), "serve", "0"),
        planted(tmp.path(), "serve", "pid"),
        tmp.path()
            .join(format!("{PREFIX}{dead}-{}", uuid::Uuid::new_v4())),
        tmp.path().join(format!("{PREFIX}serve-{dead}-not-a-uuid")),
        tmp.path()
            .join(format!("other-serve-{dead}-{}", uuid::Uuid::new_v4())),
        tmp.path().join(format!("{PREFIX}x")),
    ];
    for tree in &kept[6..] {
        std::fs::create_dir_all(tree).unwrap();
    }
    let file = tmp
        .path()
        .join(format!("{PREFIX}serve-{dead}-{}", uuid::Uuid::new_v4()));
    std::fs::write(&file, "").unwrap();

    let mut reaped = reap_dead_sessions_in(tmp.path());
    reaped.sort();
    let mut expected = vec![unowned, unheld];
    expected.sort();
    assert_eq!(reaped, expected);
    for tree in &expected {
        assert!(!tree.exists(), "{}", tree.display());
    }
    for tree in &kept {
        assert!(tree.is_dir(), "{}", tree.display());
    }
    assert!(file.is_file());
    assert_eq!(
        reap_dead_sessions_in(&tmp.path().join("absent")),
        Vec::<PathBuf>::new()
    );
}
