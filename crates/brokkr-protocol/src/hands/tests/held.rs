//! Every sealed source a workspace box mounts is mounted from the handle
//! opened at its last observation (decision 0065 slice two, U8k; SI1, MB3):
//! the argv names no sealed source by its path, the box carries the handles
//! it mounts and no other, a source that no longer opens as sealed refuses
//! with its exact cause, and in real namespaces a source replaced after
//! that observation is not the object the box mounts.

use super::super::sealed::Drifted;
use super::*;

/// The flags that mount a source, or read an overlay's, by resolving its
/// path.
const NAMED: [&str; 5] = [
    "--bind",
    "--bind-try",
    "--ro-bind",
    "--ro-bind-try",
    "--overlay-src",
];

/// A linked worktree of a repository under `root`, its Git common directory
/// outside it, mounts each sealed source by its handle: the worktree, the
/// common directory and a writable bind by `--bind-fd`, a read-only bind and
/// Git's config, opened beneath the common directory's handle, by
/// `--ro-bind-fd`, an overlay's lower layer through its handle's name, the
/// handle mounted read-only beneath it, a bind sealed absent not at all, and
/// none by a flag that resolves its path. The box holds the handles its argv
/// mounts, and not the Git directory's, which it opened and does not mount.
#[test]
fn every_sealed_source_is_mounted_from_its_handle_and_none_by_its_path() {
    let (dir, root, _) = fixture();
    let (main, linked) = (root.join("main"), root.join("linked"));
    linked_worktree(&main, &linked);
    let (tools, pad, absent) = (root.join("tools"), root.join("pad"), root.join("absent"));
    let layer = root.join("layer");
    for made in [&tools, &pad, &layer] {
        std::fs::create_dir(made).unwrap();
    }
    let spec = spec_of(json!({"kind": "workspace", "binds": [
        {"path": tools, "mode": "ro"}, {"path": pad, "mode": "rw"},
        {"path": absent, "mode": "ro"}, {"path": layer, "mode": "overlay"}
    ]}));
    let reach = sealed(&spec, &linked, &root.join("home"));
    let boxed = held_from(
        &spec,
        &linked,
        &artifact_root(dir.path()),
        &reach,
        dir.path(),
    );
    let boxed = boxed.unwrap();
    let common = main.join(".git");
    let config = common.join("config");
    let held: Vec<&PathBuf> = boxed.held().iter().map(|(path, _)| path).collect();
    assert_eq!(held, [&linked, &common, &tools, &pad, &layer, &config]);
    let raw = boxed.argv().join(" ");
    let n = boxed.held()[4].1.as_raw_fd();
    let lower = format!(
        "--ro-bind-fd {n} {l} --overlay-src /proc/self/fd/{n} ",
        l = layer.display()
    );
    assert!(raw.contains(&lower), "{lower}: {raw}");
    let git_dir = common.join("worktrees/linked");
    let sealed = [
        &linked, &git_dir, &common, &tools, &pad, &absent, &layer, &config,
    ];
    let named: Vec<PathBuf> = (boxed.argv().windows(2))
        .filter(|pair| NAMED.contains(&pair[0].as_str()))
        .map(|pair| PathBuf::from(&pair[1]))
        .filter(|source| sealed.contains(&source))
        .collect();
    assert_eq!(named, Vec::<PathBuf>::new(), "{raw}");
    let argv = described(boxed);
    let text = argv.join(" ");
    let mounts = [
        ("--bind-fd", &linked),
        ("--bind-fd", &common),
        ("--ro-bind-fd", &tools),
        ("--bind-fd", &pad),
        ("--ro-bind-fd", &config),
    ];
    for (flag, source) in mounts {
        let mounted = format!("{flag} {s} {s}", s = source.display());
        assert!(text.contains(&mounted), "{mounted}: {text}");
    }
    assert!(!text.contains(&absent.display().to_string()), "{text}");
}

/// An overlay over a source sealed where nothing stood refuses before any
/// process starts, its source not openable to be mounted, where a bind
/// sealed absent is not mounted at all.
#[test]
fn an_overlay_over_a_source_sealed_absent_refuses() {
    let (dir, root, work) = fixture();
    let layer = root.join("layer");
    let spec = spec_of(json!({"kind": "workspace", "binds": [{"path": layer, "mode": "overlay"}]}));
    let reach = sealed(&spec, &work, &root.join("home"));
    let refused = held_from(&spec, &work, &artifact_root(dir.path()), &reach, dir.path());
    let cause = refused
        .unwrap_err()
        .into_inner()
        .unwrap()
        .downcast::<Drifted>();
    let kind = std::io::ErrorKind::NotFound;
    assert_eq!(
        cause.map(|cause| *cause).ok(),
        Some(Drifted::Unopened(layer, kind))
    );
}

/// A sealed source whose path opens another object than the one sealed
/// refuses with MB3's identity cause by that path, and one whose path no
/// longer opens refuses by the path and why; the very object again opens
/// as sealed. The opening cause's words are pinned here, once.
#[test]
fn a_source_that_no_longer_opens_as_sealed_refuses_with_its_exact_cause() {
    let (_dir, root, work) = fixture();
    let (tools, old) = (root.join("tools"), root.join("tools.old"));
    std::fs::create_dir(&tools).unwrap();
    let spec = spec_of(json!({"kind": "workspace", "binds": [{"path": tools, "mode": "ro"}]}));
    let reach = sealed(&spec, &work, &root.join("home"));
    let opened = || {
        let held = reach.opened()?.into_iter();
        Ok(held
            .map(|(path, fd)| (path, fd.is_some()))
            .collect::<Vec<_>>())
    };
    let whole = vec![(work.clone(), true), (tools.clone(), true)];
    assert_eq!(opened(), Ok(whole.clone()));
    std::fs::rename(&tools, &old).unwrap();
    std::fs::create_dir(&tools).unwrap();
    assert_eq!(opened(), Err(Drifted::Moved(tools.clone())));
    std::fs::remove_dir(&tools).unwrap();
    let kind = std::io::ErrorKind::NotFound;
    assert_eq!(opened(), Err(Drifted::Unopened(tools.clone(), kind)));
    let text = format!(
        "MCP server box filesystem identity is not protected: '{}', which the box is built \
         from, cannot be opened to be mounted: entity not found",
        tools.display()
    );
    assert_eq!(Drifted::Unopened(tools.clone(), kind).to_string(), text);
    std::fs::rename(&old, &tools).unwrap();
    assert_eq!(opened(), Ok(whole));
}

/// A step putting a decoy at each of `paths` once the box holds its handles:
/// the object there renamed beside it to `<name>.old`, and a directory in
/// its place holding `file` that reads `decoy`.
#[cfg(target_os = "linux")]
pub(super) fn decoyed(paths: Vec<PathBuf>) -> Box<dyn FnOnce()> {
    Box::new(move || {
        for path in paths {
            std::fs::rename(&path, path.with_extension("old")).unwrap();
            std::fs::create_dir(&path).unwrap();
            std::fs::write(path.join("file"), "decoy\n").unwrap();
        }
    })
}

/// The object [`decoyed`] moved aside from `at` put back in the decoy's
/// place.
#[cfg(target_os = "linux")]
pub(super) fn restored(at: &Path) {
    std::fs::remove_dir_all(at).unwrap();
    std::fs::rename(at.with_extension("old"), at).unwrap();
}

/// In real namespaces: each sealed source replaced by a decoy at its path
/// after the box's last observation, once it holds their handles, is not
/// the object the box mounts. A workspace call reads the worktree's, a
/// read-only bind's and an overlay's lower layer's own files, writes into
/// the writable bind it observed and into its overlay's upper layer, the
/// decoys and the lower layer untouched, cannot write the retained
/// repository's Git config (H1), and its command holds no descriptor beyond
/// its standard streams and the one it lists them by; an exec reads the
/// worktree and the lower layer it observed.
#[cfg(target_os = "linux")]
#[test]
fn a_source_replaced_after_the_last_observation_is_not_the_object_mounted() {
    if !can_create_namespace() {
        let reason = "this environment cannot create a bubblewrap namespace";
        return skip_boundary_proof(boundary_evidence_required(), reason);
    }
    let (dir, root, work) = fixture();
    committed(&work);
    let (tools, pad, layer) = (root.join("tools"), root.join("pad"), root.join("layer"));
    for (made, file) in [(&tools, "observed\n"), (&pad, ""), (&layer, "layered\n")] {
        std::fs::create_dir(made).unwrap();
        std::fs::write(made.join("file"), file).unwrap();
    }
    let spec = spec_of(json!({"kind": "workspace", "binds": [
        {"path": tools, "mode": "ro"}, {"path": pad, "mode": "rw"},
        {"path": layer, "mode": "overlay"}
    ]}));
    let reach = sealed(&spec, &work, &root.join("home"));
    let (artifacts, session) = (artifact_root(dir.path()), dir.path().join("session"));
    let decoys = vec![work.clone(), tools.clone(), pad.clone(), layer.clone()];
    super::super::sealed::OPENED.set(Some(decoyed(decoys)));
    let (t, p, l) = (tools.display(), pad.display(), layer.display());
    let wrote = format!("echo written > {p}/w && echo upper > {l}/new");
    let config = "{ { echo planted >> .git/config; } 2>/dev/null || echo read-only; }";
    let command = format!("cat file {t}/file {l}/file && {wrote} && {config} && ls /proc/self/fd");
    let long = Duration::from_secs(30);
    let ran = execute(&spec, &work, &artifacts, &reach, &session, &command, long).unwrap();
    assert_eq!(
        ran.stdout,
        "base\nobserved\nlayered\nread-only\n0\n1\n2\n3\n",
        "{}",
        ran.render()
    );
    assert_eq!(
        std::fs::read_to_string(root.join("pad.old/w")).unwrap(),
        "written\n"
    );
    assert!(!pad.join("w").exists(), "the decoy is not written");
    let lower = [layer.join("new"), root.join("layer.old/new")];
    assert_eq!(
        lower.map(|at| at.exists()),
        [false, false],
        "the lower layers"
    );
    let config = std::fs::read_to_string(root.join("work.old/.git/config")).unwrap();
    assert!(!config.contains("planted"), "{config}");
    // The exec box, built from the reach the sources now stand at again.
    for path in [&work, &tools, &pad, &layer] {
        restored(path);
    }
    let decoys = vec![work.clone(), layer.clone()];
    super::super::sealed::OPENED.set(Some(decoyed(decoys)));
    let bwrap = require_bwrap().unwrap();
    let both = format!("test \"$(cat file {l}/file)\" = \"$(printf 'base\\nlayered')\"");
    let read = ["/bin/sh".to_string(), "-c".to_string(), both];
    let exec = dir.path().join("exec");
    let boxed = run_boxed_in(
        &bwrap,
        &spec,
        &work,
        &artifacts,
        &reach,
        &exec,
        &[],
        None,
        &read,
    );
    assert_eq!(boxed, Ok(0));
}

/// H1 (decision 0043 ruling 6; operator ruling 2026-10-26 (2)), in a real
/// namespace: a linked worktree whose Git common directory's parent path
/// is gone once the box holds its handles still has that directory as it
/// retained it. A workspace call cannot write its config, plants no hook on
/// the host, and commits into it as before.
#[cfg(target_os = "linux")]
#[test]
fn the_retained_common_directory_keeps_its_config_read_only_when_its_path_goes() {
    if !can_create_namespace() {
        let reason = "this environment cannot create a bubblewrap namespace";
        return skip_boundary_proof(boundary_evidence_required(), reason);
    }
    let (dir, root, _) = fixture();
    let (main, linked, moved) = (
        root.join("main"),
        root.join("linked"),
        root.join("main.old"),
    );
    linked_worktree(&main, &linked);
    let spec = HandsSpec::default();
    let reach = sealed(&spec, &linked, &root.join("home"));
    let (from, to) = (main.clone(), moved.clone());
    let gone = move || std::fs::rename(from, to).unwrap();
    super::super::sealed::OPENED.set(Some(Box::new(gone)));
    let common = main.join(".git").display().to_string();
    let who = "-c user.name=Seat -c user.email=seat@example.invalid";
    let command = format!(
        "{{ {{ echo planted >> {common}/config; }} 2>/dev/null || echo read-only; }} && \
         touch {common}/hooks/post-commit && git {who} commit -q --allow-empty -m boxed && \
         echo committed"
    );
    let (artifacts, session) = (artifact_root(dir.path()), dir.path().join("session"));
    let long = Duration::from_secs(30);
    let ran = execute(&spec, &linked, &artifacts, &reach, &session, &command, long).unwrap();
    assert_eq!(ran.stdout, "read-only\ncommitted\n", "{}", ran.render());
    let config = std::fs::read_to_string(moved.join(".git/config")).unwrap();
    assert!(!config.contains("planted"), "{config}");
    let hook = moved.join(".git/hooks/post-commit");
    assert!(!hook.exists(), "no hook reaches the host");
    let head = std::fs::read_to_string(moved.join(".git/worktrees/linked/HEAD")).unwrap();
    let log = ["log", "-1", "--format=%s", head.trim()];
    let subject = Command::new("git").args(log).current_dir(&moved).output();
    assert_eq!(subject.unwrap().stdout, b"boxed\n");
}

/// H1's refusals, before any process starts: Git's config gone from the
/// common directory the box retains refuses by why it does not open, a link
/// in its place as not one regular file, whose words are pinned here once,
/// and a worktree's own Git directory replaced once the box holds its
/// handles as no longer the directory sealed. Linux's path handle opens a
/// link as itself.
#[cfg(target_os = "linux")]
#[test]
fn git_s_config_the_box_cannot_hold_on_the_retained_directory_refuses() {
    let (dir, root, work) = fixture();
    committed(&work);
    let spec = HandsSpec::default();
    let reach = sealed(&spec, &work, &root.join("home"));
    let artifacts = artifact_root(dir.path());
    let (common, config, aside) = (
        work.join(".git"),
        work.join(".git/config"),
        root.join("aside"),
    );
    let built = || {
        let built = held_from(&spec, &work, &artifacts, &reach, dir.path()).map(drop);
        built.map_err(|error| *error.into_inner().unwrap().downcast::<Drifted>().unwrap())
    };
    assert_eq!(built(), Ok(()));
    std::fs::rename(&config, &aside).unwrap();
    let kind = std::io::ErrorKind::NotFound;
    assert_eq!(built(), Err(Drifted::Unopened(config.clone(), kind)));
    std::os::unix::fs::symlink(&aside, &config).unwrap();
    assert_eq!(built(), Err(Drifted::Unheld(config.clone())));
    let text = format!(
        "MCP server box filesystem identity is not protected: '{}', which the box holds \
         read-only, is not one regular file",
        config.display()
    );
    assert_eq!(Drifted::Unheld(config.clone()).to_string(), text);
    std::fs::remove_file(&config).unwrap();
    std::fs::rename(&aside, &config).unwrap();
    super::super::sealed::OPENED.set(Some(decoyed(vec![common.clone()])));
    assert_eq!(built(), Err(Drifted::Moved(common)));
}

/// L4: the descriptor hand-off needs `close_range`'s close-on-exec form,
/// Linux 5.11. A kernel that refuses it, with EINVAL for the flag before
/// 5.11 or ENOSYS for the call before 5.9, stood in for by a filter on the
/// one thread that launches, fails the launch before bubblewrap runs, by
/// the kernel's own error in the box's exact words.
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
fn a_kernel_without_close_range_s_close_on_exec_form_fails_the_launch() {
    let (dir, root, work) = fixture();
    let spec = HandsSpec::default();
    let reach = sealed(&spec, &work, &root.join("home"));
    let artifacts = artifact_root(dir.path());
    let (scratch, session) = (dir.path().join("scratch"), dir.path().join("session"));
    let refused = [
        (libc::EINVAL, "Invalid argument (os error 22)"),
        (libc::ENOSYS, "Function not implemented (os error 38)"),
    ];
    for (errno, words) in refused {
        let launch = || {
            refusing_close_range(errno);
            let long = Duration::from_secs(30);
            let bwrap = Path::new("/bin/true");
            let (spec, work) = (&spec, &work);
            execute_in(
                bwrap,
                spec,
                work,
                &artifacts,
                &reach,
                &scratch,
                &session,
                &[],
                "true",
                long,
            )
        };
        let launched = std::thread::scope(|scope| scope.spawn(launch).join().unwrap());
        let launched = launched.map(|ran| ran.exit_code);
        assert_eq!(
            launched,
            Err(format!("hands could not spawn bwrap: {words}"))
        );
    }
}

/// This thread, and every process it starts, sees `close_range` fail with
/// `errno`: a seccomp filter allowing every other call, which the thread
/// keeps until it ends.
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn refusing_close_range(errno: i32) {
    let step = |code: u32, k: u32, jt: u8, jf: u8| libc::sock_filter {
        code: u16::try_from(code).unwrap(),
        jt,
        jf,
        k,
    };
    let errno = u32::try_from(errno).unwrap() & libc::SECCOMP_RET_DATA;
    let called = u32::try_from(libc::SYS_close_range).unwrap();
    let mut filter = [
        step(libc::BPF_LD | libc::BPF_W | libc::BPF_ABS, 0, 0, 0),
        step(libc::BPF_JMP | libc::BPF_JEQ | libc::BPF_K, called, 0, 1),
        step(
            libc::BPF_RET | libc::BPF_K,
            libc::SECCOMP_RET_ERRNO | errno,
            0,
            0,
        ),
        step(libc::BPF_RET | libc::BPF_K, libc::SECCOMP_RET_ALLOW, 0, 0),
    ];
    let program = libc::sock_fprog {
        len: 4,
        filter: filter.as_mut_ptr(),
    };
    // SAFETY: both calls change this thread's own attributes; the second
    // reads `program`, which points at `filter`, both alive until it returns.
    let installed = unsafe {
        libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) == 0
            && libc::prctl(libc::PR_SET_SECCOMP, libc::SECCOMP_MODE_FILTER, &program) == 0
    };
    assert!(installed, "{}", std::io::Error::last_os_error());
}
