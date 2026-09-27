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

/// A filesystem that refuses every lock.
fn refused(_: BorrowedFd<'_>) -> Result<(), Errno> {
    Err(Errno::NOLCK)
}

/// The reaping of `tmp`, or a panic when it has not finished in a second.
fn reaped_within_a_second(tmp: &Path) -> Reaped {
    let (done, answer) = std::sync::mpsc::channel();
    let tmp = tmp.to_path_buf();
    std::thread::spawn(move || done.send(reap_dead_sessions_in(&tmp, flock)));
    answer
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("the reaper answered inside a second")
}

#[test]
fn a_session_is_locked_while_it_lives_and_removed_when_it_drops() {
    let tmp = tempfile::tempdir().unwrap();
    let session = Session::create_in(tmp.path(), "serve", flock).unwrap();
    let tree = session.path().to_path_buf();
    let name = tree.file_name().unwrap().to_str().unwrap();
    assert_eq!(
        owner_pid(name).map(|pid| pid.as_raw_nonzero().get()),
        Some(i32::try_from(std::process::id()).unwrap())
    );
    assert_eq!(probe(&tree, flock), Probe::Held, "the owner holds the lock");
    assert_eq!(reap_dead_sessions_in(tmp.path(), flock), Reaped::default());
    assert!(tree.is_dir());
    drop(session);
    assert!(!tree.exists(), "dropping the session removes its tree");
    assert_eq!(
        Session::create_in(&tmp.path().join("absent/\0"), "serve", flock)
            .unwrap_err()
            .to_string(),
        "hands session: file name contained an unexpected NUL byte"
    );
}

/// A session that cannot take its lock refuses, names the lock and the
/// errno, and leaves no tree to read as unowned.
#[test]
fn a_session_that_cannot_lock_refuses_and_leaves_no_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let error = Session::create_in(tmp.path(), "serve", refused).unwrap_err();
    let SessionError::Lock { path, cause } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(cause.raw_os_error(), Some(Errno::NOLCK.raw_os_error()));
    assert_eq!(path.file_name().unwrap(), LOCK);
    assert_eq!(path.parent().unwrap().parent().unwrap(), tmp.path());
    assert_eq!(
        error.to_string(),
        format!(
            "hands session: cannot lock {}: {}",
            path.display(),
            said(Errno::NOLCK)
        )
    );
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 0);
}

/// A dead owner's tree whose lock cannot be read, opened or locked is
/// kept, and each is said: one line naming the tree and the reason.
#[test]
fn a_lock_that_cannot_be_probed_keeps_the_tree_and_says_so() {
    use std::os::unix::fs::PermissionsExt;
    let mode = |path: &Path, bits| {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(bits)).unwrap();
    };
    let tmp = tempfile::tempdir().unwrap();
    let dead = dead_pid().to_string();
    let refusing = planted(tmp.path(), "serve", &dead);
    std::fs::write(refusing.join(LOCK), "").unwrap();
    let reaped = reap_dead_sessions_in(tmp.path(), refused);
    assert_eq!(
        reaped.to_string(),
        format!(
            "hands: kept {}: its lock cannot be probed: locking .owner.lock: {}\n",
            refusing.display(),
            said(Errno::NOLCK)
        )
    );
    let unopenable = planted(tmp.path(), "exec", &dead);
    std::fs::write(unopenable.join(LOCK), "").unwrap();
    mode(&unopenable.join(LOCK), 0o000);
    let unsearchable = planted(tmp.path(), "exec", &dead);
    mode(&unsearchable, 0o000);
    let reaped = reap_dead_sessions_in(tmp.path(), flock);
    mode(&unsearchable, 0o700);
    let mut kept = reaped.kept;
    kept.sort_by(|a, b| a.0.cmp(&b.0));
    let mut expected = vec![
        (unopenable, Unprobed::Open(Errno::ACCESS)),
        (unsearchable, Unprobed::Stat(Errno::ACCESS)),
    ];
    expected.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!((reaped.removed, kept), (vec![refusing], expected));
    assert_eq!(
        [Unprobed::Stat(Errno::ACCESS), Unprobed::Open(Errno::ACCESS)].map(|why| why.to_string()),
        [
            format!("reading .owner.lock: {}", said(Errno::ACCESS)),
            format!("opening .owner.lock: {}", said(Errno::ACCESS)),
        ]
    );
}

/// A lock file that is a FIFO, or a symlink to one, neither hangs the
/// reaper nor reads as free.
#[test]
fn a_lock_that_is_not_a_regular_file_is_kept_without_blocking() {
    for through_a_symlink in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let tree = planted(tmp.path(), "serve", &dead_pid().to_string());
        let fifo = tmp.path().join("fifo");
        let at = if through_a_symlink {
            &fifo
        } else {
            &tree.join(LOCK)
        };
        let fifo_mode = Mode::from_raw_mode(0o600);
        rustix::fs::mknodat(rustix::fs::CWD, at, FileType::Fifo, fifo_mode, 0).unwrap();
        if through_a_symlink {
            std::os::unix::fs::symlink(&fifo, tree.join(LOCK)).unwrap();
        }
        let started = std::time::Instant::now();
        let reaped = reaped_within_a_second(tmp.path());
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        assert_eq!(
            reaped.to_string(),
            format!(
                "hands: kept {}: its lock cannot be probed: .owner.lock is not a regular file\n",
                tree.display()
            )
        );
    }
}

/// A zombie is dead: its parent has not waited for it, and it holds
/// nothing. Linux reads it from `/proc`.
#[cfg(target_os = "linux")]
#[test]
fn a_zombie_owner_is_dead() {
    let mut child = std::process::Command::new("true").spawn().unwrap();
    let pid = Pid::from_raw(i32::try_from(child.id()).unwrap()).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !zombie(pid) && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!alive(pid), "an unreaped child is a zombie, and dead");
    child.wait().unwrap();
    assert!(alive(
        Pid::from_raw(i32::try_from(std::process::id()).unwrap()).unwrap()
    ));
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
    let held = Session::create_in(tmp.path(), "serve", flock).unwrap();
    let foreign = tmp
        .path()
        .join(format!("{PREFIX}serve-{dead}-{}", uuid::Uuid::new_v4()));
    std::fs::rename(held.path(), &foreign).unwrap();
    // A lock that cannot be opened is not an answer.
    let unreadable = planted(tmp.path(), "serve", &dead);
    std::os::unix::fs::symlink(LOCK, unreadable.join(LOCK)).unwrap();
    let kept = [
        foreign,
        unreadable.clone(),
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

    let mut reaped = reap_dead_sessions_in(tmp.path(), flock);
    reaped.removed.sort();
    let mut expected = vec![unowned, unheld];
    expected.sort();
    assert_eq!(
        reaped,
        Reaped {
            removed: expected.clone(),
            kept: vec![(unreadable, Unprobed::NotRegular)],
        }
    );
    for tree in &expected {
        assert!(!tree.exists(), "{}", tree.display());
    }
    for tree in &kept {
        assert!(tree.is_dir(), "{}", tree.display());
    }
    assert!(file.is_file());
    assert_eq!(
        reap_dead_sessions_in(&tmp.path().join("absent"), flock),
        Reaped::default()
    );
}
