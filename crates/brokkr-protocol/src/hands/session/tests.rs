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

/// A live session whose tree is renamed to record `pid`, as a live owner
/// whose pid reads dead here looks: its lock still holds. The session is
/// returned so the lock lives as long as the caller keeps it.
fn held_under(tmp: &Path, pid: &str) -> (Session, PathBuf) {
    let session = Session::create_in(tmp, "serve", flock, create_lock).unwrap();
    let tree = tmp.join(format!("{PREFIX}serve-{pid}-{}", uuid::Uuid::new_v4()));
    std::fs::rename(session.path(), &tree).unwrap();
    (session, tree)
}

thread_local! {
    /// Where `listing` looks, and every name it saw there.
    static LISTING: std::cell::RefCell<(PathBuf, Vec<String>)> = std::cell::RefCell::default();
}

/// A lock that lists the temporary directory the moment before it locks.
fn listing(fd: BorrowedFd<'_>) -> Result<(), Errno> {
    LISTING.with_borrow_mut(|(tmp, seen)| {
        let names = std::fs::read_dir(tmp).unwrap();
        seen.extend(names.map(|entry| entry.unwrap().file_name().into_string().unwrap()));
    });
    flock(fd)
}

/// A filesystem that refuses every lock.
fn refused(_: BorrowedFd<'_>) -> Result<(), Errno> {
    Err(Errno::NOLCK)
}

/// A filesystem that refuses the lock file's creation, as a full RAM
/// `/tmp` answers `ENOSPC`.
fn no_space(_: &Path) -> std::io::Result<File> {
    Err(std::io::Error::from_raw_os_error(
        Errno::NOSPC.raw_os_error(),
    ))
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
    let session = Session::create_in(tmp.path(), "serve", flock, create_lock).unwrap();
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
        Session::create_in(&tmp.path().join("absent/\0"), "serve", flock, create_lock)
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
    let error = Session::create_in(tmp.path(), "serve", refused, create_lock).unwrap_err();
    let SessionError::Lock { path, cause } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(cause.raw_os_error(), Some(Errno::NOLCK.raw_os_error()));
    assert_eq!(path.file_name().unwrap(), LOCK);
    assert_eq!(path.parent().unwrap().parent().unwrap(), tmp.path());
    // The tree it names is the one it would have become, never its
    // staged name.
    let tree = path
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    let final_name = format!("{PREFIX}serve-{}-", std::process::id());
    assert!(tree.starts_with(&final_name), "{tree}");
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

/// A session whose lock file cannot be created — `ENOSPC` on a full RAM
/// `/tmp`, `EMFILE`, a planted directory at `.owner.lock` — refuses with
/// that error and leaves no staging directory behind (#468): the tree
/// was made by this call, under a name `owner_pid` rejects, and would
/// otherwise sit there for want of an owner to collect.
#[test]
fn a_lock_file_that_cannot_be_created_leaves_no_staging_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let error = Session::create_in(tmp.path(), "serve", flock, no_space).unwrap_err();
    let SessionError::Io(cause) = &error else {
        panic!("{error:?}");
    };
    assert_eq!(cause.raw_os_error(), Some(Errno::NOSPC.raw_os_error()));
    assert_eq!(
        error.to_string(),
        format!("hands session: {}", said(Errno::NOSPC))
    );
    assert_eq!(
        std::fs::read_dir(tmp.path()).unwrap().count(),
        0,
        "the staging directory the create made is removed"
    );
}

/// A filesystem that plants a directory at the lock path first, as a
/// writer racing the create would leave it, and then refuses the create
/// the way the real one refuses a directory: `EISDIR`.
fn a_directory_at_the_lock(path: &Path) -> std::io::Result<File> {
    std::fs::create_dir(path)?;
    File::create(path)
}

/// A planted directory at `.owner.lock` leaves the staging tree
/// non-empty, so `remove_dir` refuses it: the removal falls back to the
/// walk, which still leaves nothing behind (#468).
#[test]
fn a_planted_directory_at_the_lock_leaves_no_staging_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let error =
        Session::create_in(tmp.path(), "serve", flock, a_directory_at_the_lock).unwrap_err();
    let SessionError::Io(cause) = &error else {
        panic!("{error:?}");
    };
    assert_eq!(cause.raw_os_error(), Some(Errno::ISDIR.raw_os_error()));
    assert_eq!(
        error.to_string(),
        format!("hands session: {}", said(Errno::ISDIR))
    );
    assert_eq!(
        std::fs::read_dir(tmp.path()).unwrap().count(),
        0,
        "the staging directory the create made is removed"
    );
}

/// The env a parent test sets on this re-executed binary, as #403's
/// `role` is played, to exhaust its descriptors and create a session.
const EMFILE_ROLE: &str = "BROKKR_HANDS_TEST_EMFILE";

/// Played in a child of the test binary, which keeps the descriptor
/// exhaustion away from parallel tests: with every descriptor taken, the
/// lock file's create fails `EMFILE` for real — the case an injected
/// double that leaves descriptors free cannot plant. The child lowers its
/// own ceiling first, so the exhaustion costs a few dozen descriptors and
/// not the host's whole table beside parallel tests. What is left in the
/// temporary directory the parent handed over, the parent reads.
#[test]
#[ignore = "played only when a parent test re-executes this binary"]
fn emfile_role() {
    if std::env::var_os(EMFILE_ROLE).is_none() {
        return;
    }
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    assert_eq!(
        unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) },
        0,
        "{}",
        std::io::Error::last_os_error()
    );
    limit.rlim_cur = limit.rlim_cur.min(64);
    assert_eq!(
        unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limit) },
        0,
        "{}",
        std::io::Error::last_os_error()
    );
    let mut held = Vec::new();
    loop {
        match File::open("/dev/null") {
            Ok(file) => held.push(file),
            Err(error) => {
                assert_eq!(error.raw_os_error(), Some(libc::EMFILE), "{error}");
                break;
            }
        }
    }
    assert!(!held.is_empty(), "no descriptor was free to exhaust");
    let error = Session::create("serve").unwrap_err();
    let SessionError::Io(error) = &error else {
        panic!("{error:?}");
    };
    assert_eq!(error.raw_os_error(), Some(libc::EMFILE), "{error}");
}

/// The real `EMFILE` case, in a child of this test binary so the
/// exhaustion does not reach parallel tests: a lock file that cannot be
/// created for want of a descriptor must leave no staging directory,
/// though the walk `remove_dir_all` needs would fail the same way —
/// `remove_dir` needs none.
#[test]
fn a_lock_create_that_fails_for_want_of_a_descriptor_leaves_no_staging_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let played = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "hands::session::tests::emfile_role", "--ignored"])
        .env("TMPDIR", tmp.path())
        .env(EMFILE_ROLE, "1")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(
        played.status.success(),
        "{}{}",
        String::from_utf8_lossy(&played.stdout),
        String::from_utf8_lossy(&played.stderr)
    );
    assert_eq!(
        std::fs::read_dir(tmp.path()).unwrap().count(),
        0,
        "the staging directory the create made is removed"
    );
}

/// Each tree's path is through `safe` before the lines are joined, so a
/// newline inside it cannot pose as one of the renderer's own (#468).
#[test]
fn a_hostile_name_is_sanitized_before_the_lines_are_joined() {
    let tmp = tempfile::tempdir().unwrap();
    let tree = planted(tmp.path(), "x\r\nforged line", &dead_pid().to_string());
    let reaped = reap_dead_sessions_in(tmp.path(), flock);
    let said: String = tree
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .chars()
        .filter(|c| !c.is_control())
        .collect();
    let safe = |name: &str| name.chars().filter(|c| !c.is_control()).collect::<String>();
    assert_eq!(
        reaped.lines_with(safe),
        format!(
            "hands: reaped {}: its owner is dead and holds no lock\n",
            tmp.path().join(said).display()
        )
    );
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
        // mkfifo(1), because rustix offers no FIFO call on macOS.
        let made = std::process::Command::new("mkfifo").arg(at).status();
        assert!(made.unwrap().success());
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
    let (_held, foreign) = held_under(tmp.path(), &dead);
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

/// The one-release lockless transition, the operator's ruling of
/// 2026-09-28 that 0.13.0 ends: a tree with no lock file whose pid reads
/// dead is reaped, and a tree whose pid reads dead but whose lock is held
/// is kept.
#[test]
fn until_0_13_0_a_lockless_dead_tree_is_reaped_and_a_held_one_kept() {
    let tmp = tempfile::tempdir().unwrap();
    let dead = dead_pid().to_string();
    let lockless = planted(tmp.path(), "serve", &dead);
    let (_held, locked) = held_under(tmp.path(), &dead);
    assert_eq!(
        reap_dead_sessions_in(tmp.path(), flock).to_string(),
        format!(
            "hands: reaped {}: its owner is dead and holds no lock\n",
            lockless.display()
        )
    );
    assert!(!lockless.exists());
    assert!(locked.join(LOCK).is_file());
}

/// A tree mid-creation is never one a reaper reads: until its lock is
/// held it carries a name `owner_pid` rejects, and it takes its own name
/// only once locked.
#[test]
fn a_tree_mid_creation_is_never_reapable() {
    let tmp = tempfile::tempdir().unwrap();
    LISTING.set((tmp.path().to_path_buf(), Vec::new()));
    let session = Session::create_in(tmp.path(), "serve", listing, create_lock).unwrap();
    let (_, seen) = LISTING.take();
    let name = session.path().file_name().unwrap().to_str().unwrap();
    assert_eq!(seen, [format!("{STAGING}{name}")]);
    assert_eq!(owner_pid(&seen[0]), None);
    assert_eq!(probe(session.path(), flock), Probe::Held);
    let now: Vec<_> = std::fs::read_dir(tmp.path()).unwrap().collect();
    assert_eq!(now.len(), 1);
    assert_eq!(now[0].as_ref().unwrap().file_name(), name);
}

/// A dead owner's unheld tree that cannot be removed is neither removed
/// nor reported kept: the next start tries it again. Until #504 an exec
/// box's overlay `work` directory (mode 000, made by overlayfs) was what
/// made a tree unremovable; an exec box now writes its overlays to RAM,
/// so the refusal is planted here as a directory its owner cannot write.
#[cfg(unix)]
#[test]
fn a_free_tree_that_cannot_be_removed_is_left_for_the_next_start() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let tree = planted(tmp.path(), "exec", &dead_pid().to_string());
    let sealed = tree.join("sealed");
    std::fs::create_dir(&sealed).unwrap();
    std::fs::write(sealed.join("call"), "").unwrap();
    std::fs::set_permissions(&sealed, std::fs::Permissions::from_mode(0o500)).unwrap();

    let reaped = reap_dead_sessions_in(tmp.path(), flock);
    std::fs::set_permissions(&sealed, std::fs::Permissions::from_mode(0o700)).unwrap();

    assert_eq!(reaped, Reaped::default());
    assert!(sealed.join("call").is_file(), "the tree is left whole");
}
