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

/// A tree named as a session names it, planted with a lock file no process
/// holds: what a dead owner leaves when its lock file outlives it. The
/// reaper reads it as free; a tree with no lock file at all is no longer
/// reaped (0.13.0).
fn unheld(tmp: &Path, label: &str, pid: &str) -> PathBuf {
    let tree = planted(tmp, label, pid);
    std::fs::write(tree.join(LOCK), "").unwrap();
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
    let tree = unheld(tmp.path(), "x\r\nforged line", &dead_pid().to_string());
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
    let accessor: Vec<_> = reaped.kept().collect();
    assert_eq!(accessor.len(), 2);
    let mut kept = reaped.kept;
    kept.sort_by(|a, b| a.0.cmp(&b.0));
    let mut expected = vec![
        (unopenable, Kept::Unprobed(Unprobed::Open(Errno::ACCESS))),
        (unsearchable, Kept::Unprobed(Unprobed::Stat(Errno::ACCESS))),
    ];
    expected.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        (
            reaped
                .removed
                .iter()
                .map(|(tree, _)| tree.clone())
                .collect::<Vec<_>>(),
            kept,
        ),
        (vec![refusing], expected)
    );
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
    let serve = unheld(tmp.path(), "serve", &dead);
    let exec = unheld(tmp.path(), "exec", &dead);
    let (_held, foreign) = held_under(tmp.path(), &dead);
    // A live owner holds its lock; a lockless tree is said whoever its
    // pid names, so a live name alone is not a fixture.
    let (_mine_owner, mine_tree) = held_under(tmp.path(), &mine);
    let (_one_owner, one_tree) = held_under(tmp.path(), "1");
    // A lock that cannot be opened is not an answer.
    let unreadable = planted(tmp.path(), "serve", &dead);
    std::os::unix::fs::symlink(LOCK, unreadable.join(LOCK)).unwrap();
    let kept = [
        foreign,
        unreadable.clone(),
        mine_tree,
        one_tree,
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
    reaped.removed.sort_by(|a, b| a.0.cmp(&b.0));
    let mut expected = vec![serve, exec];
    expected.sort();
    assert_eq!(
        reaped
            .removed
            .iter()
            .map(|(tree, _)| tree.clone())
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        reaped.kept,
        vec![(unreadable, Kept::Unprobed(Unprobed::NotRegular))]
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

/// A live owner's tree is never removed, even when its lock answers free:
/// a server that writes its lock file between the reaper's lockless read
/// and its probe leaves exactly that, and no planted tree can stand in for
/// the race. Only a dead owner's free tree is reapable.
#[test]
fn a_live_owner_s_free_tree_is_never_reapable() {
    assert!(!reapable(&Probe::Free, false));
    assert!(!reapable(&Probe::Held, true));
    assert!(!reapable(&Probe::Held, false));
    assert!(reapable(&Probe::Free, true));
}

/// The lockless transition's end, the operator's ruling of 2026-09-28:
/// a tree with no lock file is kept and said, never reaped, whatever its
/// pid reads: dead here, live here, or a foreign namespace's whose pid
/// collides with a live one here. A tree whose pid reads dead but whose
/// lock is held is kept.
#[test]
fn a_lockless_tree_is_kept_and_said_and_a_held_one_kept() {
    let tmp = tempfile::tempdir().unwrap();
    let dead = dead_pid().to_string();
    let mine = std::process::id().to_string();
    let mut lockless = vec![
        planted(tmp.path(), "serve", &dead),
        planted(tmp.path(), "serve", &mine),
        // pid 1 is live here; a foreign namespace's tree can carry the
        // same number and be dead there, so the name is no evidence.
        planted(tmp.path(), "serve", "1"),
    ];
    let (_held, locked) = held_under(tmp.path(), &dead);
    for tree in &lockless {
        std::fs::write(tree.join("call"), "").unwrap();
    }
    let mut reaped = reap_dead_sessions_in(tmp.path(), flock);
    reaped.kept.sort_by(|a, b| a.0.cmp(&b.0));
    lockless.sort();
    let expected: Vec<_> = lockless
        .iter()
        .map(|tree| (tree.clone(), Kept::Unprobed(Unprobed::NoLock)))
        .collect();
    assert_eq!(
        reaped,
        Reaped {
            removed: Vec::new(),
            kept: expected,
        }
    );
    let lines: String = lockless
        .iter()
        .map(|tree| {
            format!(
                "hands: kept {}: its lock cannot be probed: it holds no lock file; \
                 trees from before #415 are no longer reaped (0.13.0)\n",
                tree.display()
            )
        })
        .collect();
    assert_eq!(reaped.to_string(), lines);
    for tree in &lockless {
        assert!(tree.join("call").is_file(), "{}", tree.display());
    }
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

fn set_mode(path: &Path, bits: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(bits)).unwrap();
}

/// A dead owner's unheld tree whose root cannot be unlinked from its
/// parent is kept and said, with the removal's error, so the start's
/// stderr names it (#620). The walk still unseals and empties the tree:
/// the error is the root's own unlink, and the empty root waits for the
/// next pass.
#[test]
fn a_free_tree_that_cannot_be_removed_is_kept_and_said() {
    let tmp = tempfile::tempdir().unwrap();
    let dead = dead_pid().to_string();
    let tree = unheld(tmp.path(), "exec", &dead);
    let sealed = tree.join("sealed");
    std::fs::create_dir(&sealed).unwrap();
    std::fs::write(sealed.join("call"), "").unwrap();
    set_mode(&sealed, 0o500);
    let unlistable = unheld(tmp.path(), "serve", &dead);
    set_mode(&unlistable, 0o300);
    set_mode(tmp.path(), 0o500);

    let mut reaped = reap_dead_sessions_in(tmp.path(), flock);
    set_mode(tmp.path(), 0o700);
    set_mode(&unlistable, 0o700);
    reaped.kept.sort_by(|a, b| a.0.cmp(&b.0));

    let denied = |path: &Path| {
        Unremovable(TreeError::Io {
            path: path.to_path_buf(),
            cause: said(Errno::ACCESS),
        })
    };
    let mut kept = vec![
        (tree.clone(), Kept::Unremovable(denied(&tree))),
        (unlistable.clone(), Kept::Unremovable(denied(&unlistable))),
    ];
    kept.sort_by(|a, b| a.0.cmp(&b.0));
    // The line is pinned exactly, never built from the reason's `Display`:
    // it is the words it had before the one walk, the cause alone after the
    // tree.
    let mut ordered = [tree.clone(), unlistable.clone()];
    ordered.sort();
    let lines: String = ordered
        .iter()
        .map(|tree| {
            format!(
                "hands: kept {}: it cannot be removed: {}\n",
                tree.display(),
                said(Errno::ACCESS)
            )
        })
        .collect();
    assert_eq!(reaped.to_string(), lines);
    assert_eq!(
        reaped,
        Reaped {
            removed: Vec::new(),
            kept
        }
    );
    assert!(!sealed.exists(), "the walk unsealed and emptied the tree");
    assert!(tree.is_dir() && unlistable.is_dir(), "the roots are left");
}

/// A layer's overlay work directory as overlayfs leaves it under `tree`:
/// `overlay/<n>/work/work` with no bit for its owner, holding a file.
fn sealed_overlay_work(tree: &Path, layer: &str) -> PathBuf {
    let work = tree.join("overlay").join(layer).join("work").join("work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(work.join("index"), "").unwrap();
    set_mode(&work, 0o000);
    work
}

/// A dead owner's tree whose overlay work directories were left at 0000
/// and 0200 is reaped whole (#620), and so is a live session's own tree
/// when it drops.
#[test]
fn a_tree_whose_overlay_work_is_sealed_is_removed() {
    let tmp = tempfile::tempdir().unwrap();
    let tree = unheld(tmp.path(), "exec", &dead_pid().to_string());
    sealed_overlay_work(&tree, "0");
    set_mode(&sealed_overlay_work(&tree, "1"), 0o200);
    std::fs::write(tree.join("overlay").join("2"), "").unwrap();

    assert_eq!(
        reap_dead_sessions_in(tmp.path(), flock).to_string(),
        format!(
            "hands: reaped {}: its owner is dead and holds no lock\n",
            tree.display()
        )
    );
    assert!(!tree.exists());

    let session = Session::create_under(tmp.path(), "serve").unwrap();
    let own = session.path().to_path_buf();
    sealed_overlay_work(&own, "0");
    drop(session);
    assert!(!own.exists(), "dropping the session removes its tree");
}

/// A dead owner's free tree is removed with the bytes its walk freed, and
/// a live owner's held tree is not.
#[test]
fn a_dead_owner_s_free_tree_reports_its_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let tree = unheld(tmp.path(), "exec", &dead_pid().to_string());
    std::fs::write(tree.join("payload"), vec![9u8; 12_345]).unwrap();
    let (_held, live) = held_under(tmp.path(), &dead_pid().to_string());
    let expected = crate::tree::count_tree(&tree).unwrap().unwrap().bytes;

    let reaped = reap_dead_sessions_in(tmp.path(), flock);
    let removed: Vec<_> = reaped.removed().collect();
    assert_eq!(removed, vec![(tree.as_path(), expected)]);
    assert!(!tree.exists());
    assert_eq!(reaped.kept().count(), 0);
    assert!(live.is_dir(), "a held tree is never removed");
}

/// The listing names the trees the reaping would remove, with the bytes a
/// removal would free, and removes nothing.
#[test]
fn the_listing_removes_nothing_and_counts_what_the_reaping_would() {
    let tmp = tempfile::tempdir().unwrap();
    let tree = unheld(tmp.path(), "exec", &dead_pid().to_string());
    std::fs::write(tree.join("payload"), vec![3u8; 9_000]).unwrap();
    let (_held, live) = held_under(tmp.path(), &dead_pid().to_string());
    let expected = crate::tree::count_tree(&tree).unwrap().unwrap().bytes;

    assert_eq!(
        list_reapable_under(tmp.path()),
        vec![Listed {
            tree: tree.clone(),
            count: Ok(Some(crate::tree::Counted {
                bytes: expected,
                unreadable: None,
            })),
        }]
    );
    assert_eq!(
        list_reapable_under(tmp.path())[0].line_with(|path| path.to_owned()),
        format!("hands: listed {}: {expected} bytes\n", tree.display())
    );
    assert!(
        tree.join("payload").is_file(),
        "the listing removes nothing"
    );
    assert!(live.is_dir());
    let reaped = reap_dead_sessions_in(tmp.path(), flock);
    assert_eq!(
        reaped.removed().collect::<Vec<_>>(),
        vec![(tree.as_path(), expected)]
    );
    assert!(!tree.exists());
}

/// The overlay walk follows no symlink: a link at `overlay/<n>/work/work`
/// to a directory outside the tree leaves that directory's mode alone,
/// and the link is removed with the tree, never its target.
#[test]
fn the_overlay_walk_follows_no_symlink_out_of_the_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = tmp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("kept"), "").unwrap();
    set_mode(&outside, 0o500);
    let tree = unheld(tmp.path(), "exec", &dead_pid().to_string());
    let work = tree.join("overlay").join("0").join("work");
    std::fs::create_dir_all(&work).unwrap();
    std::os::unix::fs::symlink(&outside, work.join("work")).unwrap();

    let reaped = reap_dead_sessions_in(tmp.path(), flock);
    let mode = std::fs::metadata(&outside).unwrap().permissions().mode() & 0o777;
    set_mode(&outside, 0o700);

    assert_eq!(
        reaped.to_string(),
        format!(
            "hands: reaped {}: its owner is dead and holds no lock\n",
            tree.display()
        )
    );
    assert_eq!(mode, 0o500, "the link's target was not changed");
    assert!(outside.join("kept").is_file(), "nor removed");
}

/// What the next lock probe does to its own tree before answering: a tree
/// can vanish or become a link between the candidate read and the count,
/// and neither may be dropped.
enum OnProbe {
    Vanish(PathBuf),
    Link { tree: PathBuf, target: PathBuf },
}

thread_local! {
    static ON_PROBE: std::cell::RefCell<Option<OnProbe>> =
        const { std::cell::RefCell::new(None) };
}

fn changing(fd: BorrowedFd<'_>) -> Result<(), Errno> {
    match ON_PROBE.with_borrow_mut(|slot| slot.take()) {
        Some(OnProbe::Vanish(tree)) => std::fs::remove_dir_all(tree).unwrap(),
        Some(OnProbe::Link { tree, target }) => {
            std::fs::remove_dir_all(&tree).unwrap();
            std::os::unix::fs::symlink(target, tree).unwrap();
        }
        None => {}
    }
    flock(fd)
}

/// The renderer's `safe`: every control character removed.
fn stripped(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_control())
        .collect()
}

/// Whether `line` is one line whose only control byte is its own newline.
fn one_clean_line(line: &str) -> bool {
    line.strip_suffix('\n')
        .is_some_and(|line| line.chars().all(|character| !character.is_control()))
}

/// A hostile reaping root holding one dead owner's tree, whose label is
/// hostile too.
fn hostile_listing_root(label: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("evil\n\u{1b}[31mlist");
    std::fs::create_dir(&root).unwrap();
    let tree = unheld(&root, label, &dead_pid().to_string());
    (tmp, root, tree)
}

/// A hostile name at the reaping root and below it never reaches a kept or
/// a removed line: the tree's path goes through `safe`, and the kept reason
/// carries the cause alone, never the removal's own path.
#[test]
fn a_hostile_name_never_reaches_a_kept_or_a_removed_line() {
    let tmp = tempfile::tempdir().unwrap();
    let dead = dead_pid().to_string();

    let removed_root = tmp.path().join("evil\n\u{1b}[31mremoved");
    std::fs::create_dir(&removed_root).unwrap();
    let removed = unheld(&removed_root, "exec\t\u{7}", &dead);
    std::fs::create_dir(removed.join("sub\n\u{1b}[32m")).unwrap();
    let removed_line = reap_dead_sessions_in(&removed_root, flock).lines_with(stripped);
    assert_eq!(
        removed_line,
        format!(
            "hands: reaped {}: its owner is dead and holds no lock\n",
            stripped(&removed.display().to_string())
        )
    );
    assert!(!removed.exists());

    let kept_root = tmp.path().join("evil\n\u{1b}[31mkept");
    std::fs::create_dir(&kept_root).unwrap();
    let kept = unheld(&kept_root, "serve\r\u{1b}[33m", &dead);
    std::fs::create_dir(kept.join("sub\n\u{1b}[34m")).unwrap();
    set_mode(&kept_root, 0o500);
    let kept_line = reap_dead_sessions_in(&kept_root, flock).lines_with(stripped);
    set_mode(&kept_root, 0o700);
    assert_eq!(
        kept_line,
        format!(
            "hands: kept {}: it cannot be removed: {}\n",
            stripped(&kept.display().to_string()),
            said(Errno::ACCESS)
        )
    );

    for line in [removed_line, kept_line] {
        assert!(one_clean_line(&line), "{line:?}");
    }
}

/// `Listed::line_with` passes every path it names through `safe`, on every
/// arm: the tree on all four, and the first unreadable directory on the
/// lower-bound one. A seat can name a directory with a newline and an ESC,
/// and 5c2's `--dry-run` paints these lines.
#[test]
fn a_listing_line_passes_every_named_path_through_safe() {
    let (_whole_tmp, whole_root, whole) = hostile_listing_root("exec\t\u{7}");
    let whole_line = list_reapable_in(&whole_root, flock)[0].line_with(stripped);
    assert_eq!(
        whole_line,
        format!(
            "hands: listed {}: {} bytes\n",
            stripped(&whole.display().to_string()),
            crate::tree::count_tree(&whole).unwrap().unwrap().bytes
        )
    );

    let (_lower_tmp, lower_root, lower) = hostile_listing_root("exec\n\u{1b}[35m");
    let work = lower.join("overlay").join("work\n\u{1b}[32mwork");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(work.join("index"), "").unwrap();
    set_mode(&work, 0o000);
    let listed = list_reapable_in(&lower_root, flock);
    let counted = listed[0].count.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(counted.unreadable.as_deref(), Some(work.as_path()));
    let lower_line = listed[0].line_with(stripped);
    assert_eq!(
        lower_line,
        format!(
            "hands: listed {}: at least {} bytes; it cannot read {}\n",
            stripped(&lower.display().to_string()),
            counted.bytes,
            stripped(&work.display().to_string())
        )
    );

    let (_gone_tmp, gone_root, gone) = hostile_listing_root("exec\r\u{1b}[36m");
    ON_PROBE.set(Some(OnProbe::Vanish(gone.clone())));
    let gone_line = list_reapable_in(&gone_root, changing)[0].line_with(stripped);
    assert_eq!(
        gone_line,
        format!(
            "hands: listed {}: it was gone before the count\n",
            stripped(&gone.display().to_string())
        )
    );

    let (_failed_tmp, failed_root, failed) = hostile_listing_root("serve\n\u{1b}[37m");
    let outside = failed.parent().unwrap().join("outside");
    std::fs::create_dir(&outside).unwrap();
    ON_PROBE.set(Some(OnProbe::Link {
        tree: failed.clone(),
        target: outside,
    }));
    let failed_line = list_reapable_in(&failed_root, changing)[0].line_with(stripped);
    assert_eq!(
        failed_line,
        format!(
            "hands: listed {}: the count failed: {}\n",
            stripped(&failed.display().to_string()),
            said(Errno::NOTDIR)
        )
    );

    for line in [&whole_line, &lower_line, &gone_line, &failed_line] {
        assert!(one_clean_line(line), "{line:?}");
    }
}

/// The listing names a tree's lower bound and the directory it cannot read,
/// and a removal still takes that tree whole.
#[test]
fn the_listing_names_a_lower_bound_and_the_unreadable_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let tree = unheld(tmp.path(), "exec", &dead_pid().to_string());
    let work = tree.join("overlay").join("0").join("work").join("work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(work.join("index"), vec![3u8; 4096]).unwrap();
    let whole = crate::tree::count_tree(&tree).unwrap().unwrap().bytes;
    set_mode(&work, 0o000);

    let listed = list_reapable_under(tmp.path());
    assert_eq!(listed.len(), 1);
    let counted = listed[0].count.as_ref().unwrap().as_ref().unwrap();
    assert_eq!(counted.unreadable.as_deref(), Some(work.as_path()));
    assert!(
        counted.bytes < whole,
        "{} is not below {whole}",
        counted.bytes
    );
    assert_eq!(
        listed[0].line_with(|path| path.to_owned()),
        format!(
            "hands: listed {}: at least {} bytes; it cannot read {}\n",
            tree.display(),
            counted.bytes,
            work.display()
        )
    );

    let reaped = reap_dead_sessions_in(tmp.path(), flock);
    assert_eq!(
        reaped.removed().collect::<Vec<_>>(),
        vec![(tree.as_path(), whole)]
    );
    assert!(!tree.exists());
}

/// A tree whose count fails is listed with the refusal, never dropped; the
/// hostile link it became is not followed.
#[test]
fn the_listing_names_a_count_that_failed() {
    let tmp = tempfile::tempdir().unwrap();
    let tree = unheld(tmp.path(), "serve", &dead_pid().to_string());
    let outside = tmp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("kept"), "").unwrap();
    ON_PROBE.set(Some(OnProbe::Link {
        tree: tree.clone(),
        target: outside.clone(),
    }));

    let listed = list_reapable_in(tmp.path(), changing);
    assert_eq!(listed.len(), 1);
    match &listed[0].count {
        Err(TreeError::Io { path, cause }) => {
            assert_eq!(path, &tree);
            assert_eq!(cause.raw_os_error(), Some(Errno::NOTDIR.raw_os_error()));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        listed[0].line_with(|path| path.to_owned()),
        format!(
            "hands: listed {}: the count failed: {}\n",
            tree.display(),
            said(Errno::NOTDIR)
        )
    );
    assert!(outside.join("kept").is_file());
    assert!(tree.is_symlink());
}

/// A fresh root holding one session tree the next probe vanishes.
fn vanishing_tree() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let tree = unheld(tmp.path(), "exec", &dead_pid().to_string());
    ON_PROBE.set(Some(OnProbe::Vanish(tree.clone())));
    (tmp, tree)
}

/// A tree gone before the count is listed as gone, not dropped.
#[test]
fn the_listing_names_a_tree_gone_before_the_count() {
    let (tmp, tree) = vanishing_tree();

    let listed = list_reapable_in(tmp.path(), changing);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].count, Ok(None));
    assert_eq!(
        listed[0].line_with(|path| path.to_owned()),
        format!(
            "hands: listed {}: it was gone before the count\n",
            tree.display()
        )
    );
    assert!(!tree.exists());
}

/// A tree that vanishes between the probe and the removal is not kept and
/// not counted: the removal found it already absent.
#[test]
fn a_tree_gone_between_the_probe_and_the_removal_is_not_kept() {
    let (tmp, tree) = vanishing_tree();

    assert_eq!(
        reap_dead_sessions_in(tmp.path(), changing),
        Reaped::default()
    );
    assert!(!tree.exists());
}
