//! The evidence a workspace box reads and never changes (decision 0065
//! slice two, U8a; CR2, MB3): the artifact root the engine hands the box,
//! pinned and mounted read-only after every writable mount where the
//! worktree holds it and mounted nowhere where it does not, the aliases
//! and links that refuse the box, and, in a real namespace, the writes,
//! renames and links that cannot change a published artifact.

#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;

#[cfg(target_os = "linux")]
use super::super::evidence::{named, numbered, observed};
#[cfg(target_os = "linux")]
use super::super::evidence::{reached, shown_elsewhere, wanted};
#[cfg(target_os = "linux")]
use super::super::evidence::{unaliased_on, unaliased_with};
#[cfg(target_os = "linux")]
use super::super::evidence::{Block, Contained, Facts, Root, Seen, Source};
use super::super::evidence::{Protected, Unprotected};
#[cfg(target_os = "linux")]
use super::super::namespace::sources::host::Record;
use super::*;

/// A digest name the fixtures publish under; any name is a file here.
const HEX: &str = "9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa";

/// The artifact root `relative` below `base`, made as the engine makes it
/// before launch: a recipe library's `.forge/artifacts` in its worktree,
/// an adopter's `.brokkr/state/artifacts`, or one beside the worktree.
fn made(base: &Path, relative: &str) -> PathBuf {
    let artifacts = base.join(relative);
    std::fs::create_dir_all(&artifacts).unwrap();
    artifacts
}

/// A recipe library's artifact root in the worktree `work`.
fn library(work: &Path) -> PathBuf {
    made(work, ".forge/artifacts")
}

/// Publish `bytes` as the one artifact under the root `artifacts`.
fn publish(artifacts: &Path, bytes: &str) -> PathBuf {
    let digests = artifacts.join("sha256");
    std::fs::create_dir_all(&digests).unwrap();
    let artifact = digests.join(HEX);
    std::fs::write(&artifact, bytes).unwrap();
    artifact
}

/// The workspace box over `work` handed `artifacts`, its private tree and
/// session under `dir`, built from the reach sealed for it there.
fn boxed(
    spec: &HandsSpec,
    work: &Path,
    artifacts: &Path,
    dir: &Path,
) -> std::io::Result<Vec<String>> {
    boxed_from(
        spec,
        work,
        artifacts,
        &sealed(spec, work, &dir.join("home")),
        dir,
    )
}

/// The refusal a box build returned, as its own variant.
fn refused(built: std::io::Result<Vec<String>>) -> Unprotected {
    let error = built.expect_err("the box was built");
    *error
        .into_inner()
        .unwrap()
        .downcast::<Unprotected>()
        .unwrap()
}

fn rw(path: &Path, mode: &str) -> HandsSpec {
    spec_of(json!({"kind": "workspace", "binds": [{"path": path, "mode": mode}]}))
}

#[cfg(target_os = "linux")]
fn identity(path: &Path) -> (u64, u64) {
    let found = std::fs::symlink_metadata(path).unwrap();
    (found.dev(), found.ino())
}

/// The position of `flag path path` in `argv`, a mount at its own path.
fn mounted(argv: &[String], flag: &str, path: &Path) -> Option<usize> {
    let path = path.display().to_string();
    argv.windows(3).position(|at| at == [flag, &path, &path])
}

/// The flags that make a mount the box can write.
const WRITABLE: [&str; 5] = [
    "--bind",
    "--bind-try",
    "--overlay",
    "--tmp-overlay",
    "--tmpfs",
];

/// A root the worktree holds, a recipe library's or an adopter's state
/// inside it, has each directory between them mounted at its own path
/// before the declared binds and the root read-only after every writable
/// mount: nothing mounted later can shadow it. Nothing is made here.
#[test]
fn a_root_the_worktree_holds_is_pinned_and_mounted_read_only_after_every_writable_mount() {
    let (dir, root, work) = fixture();
    let (pad, tools) = (made(&root, "pad"), made(&root, "tools"));
    let spec = spec_of(json!({"kind": "workspace", "binds": [
        {"path": pad, "mode": "rw"}, {"path": tools, "mode": "overlay"}
    ]}));
    for (relative, pins) in [
        (".forge/artifacts", &[".forge"][..]),
        (".brokkr/state/artifacts", &[".brokkr", ".brokkr/state"][..]),
    ] {
        let artifacts = made(&work, relative);
        let argv = boxed(&spec, &work, &artifacts, dir.path()).unwrap();
        assert!(!artifacts.join("sha256").exists(), "the box makes nothing");
        let first = mounted(&argv, "--bind-try", &pad).unwrap();
        for pin in pins {
            let at = mounted(&argv, "--bind", &work.join(pin));
            assert!(at.unwrap() < first, "{pin} is pinned before the binds");
        }
        let protected = mounted(&argv, "--ro-bind", &artifacts).unwrap();
        let last = argv
            .iter()
            .rposition(|part| WRITABLE.contains(&part.as_str()));
        assert!(last.unwrap() < protected, "{argv:?}");
        let mounts = ["--ro-bind", "--ro-bind-try", "--overlay-src"];
        let after: Vec<&String> = (argv[protected + 3..].iter())
            .filter(|part| mounts.contains(&part.as_str()) || WRITABLE.contains(&part.as_str()))
            .collect();
        assert_eq!(after, Vec::<&String>::new());
    }
}

/// An adopter's state outside the worktree is mounted nowhere: no pin and
/// no read-only mount name it or a directory holding it, so the box has
/// no path to it, and a writable bind of a directory holding it still
/// refuses the box.
#[test]
fn a_root_outside_every_bind_is_mounted_nowhere_and_its_aliases_still_refuse() {
    let (dir, root, work) = fixture();
    let artifacts = made(&root, "adopter/.brokkr/state/artifacts");
    publish(&artifacts, "evidence\n");
    let argv = boxed(&HandsSpec::default(), &work, &artifacts, dir.path()).unwrap();
    let named: Vec<&String> = (argv.iter())
        .filter(|part| part.starts_with(&root.join("adopter").display().to_string()))
        .collect();
    assert_eq!(named, Vec::<&String>::new());
    let state = root.join("adopter/.brokkr/state");
    let built = boxed(&rw(&state, "rw"), &work, &artifacts, dir.path());
    assert_eq!(refused(built), Unprotected::Alias(state));
    let sibling = made(&root, "adopter/src");
    assert!(boxed(&rw(&sibling, "rw"), &work, &artifacts, dir.path()).is_ok());
}

/// A writable or overlay mount of the root, of a directory holding it or
/// of one inside it refuses the box, judged by identity however it is
/// spelled; the git common directory and the private directories are held
/// to the same rule. A read-only alias, a sibling and an absent path are
/// mounted as declared.
#[test]
fn a_writable_alias_of_the_artifact_root_refuses_the_box() {
    let (dir, root, work) = fixture();
    let artifacts = library(&work);
    publish(&artifacts, "evidence\n");
    let (link, inner) = (root.join("link"), root.join("inner"));
    std::os::unix::fs::symlink(&artifacts, &link).unwrap();
    std::os::unix::fs::symlink(artifacts.join("sha256"), &inner).unwrap();
    for (host, mode) in [
        (link.clone(), "rw"),
        (work.join(".forge"), "rw"),
        (root.clone(), "overlay"),
        (work.clone(), "rw"),
        (inner.clone(), "overlay"),
        (artifacts.join("sha256"), "rw"),
    ] {
        let built = boxed(&rw(&host, mode), &work, &artifacts, dir.path());
        assert_eq!(refused(built), Unprotected::Alias(host));
    }
    // A linked worktree whose Git common directory holds its root.
    let (main, linked) = (root.join("main"), root.join("linked"));
    linked_worktree(&main, &linked);
    let held = made(&main, ".git/evidence");
    let session = OverlayWrites::Session(&root);
    let spec = HandsSpec::default();
    let (home, scratch) = (root.join("home"), root.join("scratch"));
    let built = box_argv(
        &spec,
        &linked,
        &held,
        &sealed(&spec, &linked, &home),
        &scratch,
        session,
        &[],
        None,
        &one("true"),
    );
    assert_eq!(refused(built), Unprotected::Alias(main.join(".git")));
    let inside = artifacts.join("scratch");
    let built = box_argv(
        &spec,
        &work,
        &artifacts,
        &sealed(&spec, &work, &home),
        &inside,
        session,
        &[],
        None,
        &one("true"),
    );
    assert_eq!(refused(built), Unprotected::Alias(inside.join("home")));

    let plain = root.join("plain");
    std::fs::write(&plain, "").unwrap();
    let built = boxed(&rw(&plain.join("x"), "rw"), &work, &artifacts, dir.path());
    let kind = std::io::ErrorKind::NotADirectory;
    assert_eq!(
        refused(built),
        Unprotected::Unobserved(plain.join("x"), kind)
    );
    let pad = made(&root, "pad");
    for (host, mode) in [(link, "ro"), (pad, "rw"), (root.join("absent"), "rw")] {
        assert!(boxed(&rw(&host, mode), &work, &artifacts, dir.path()).is_ok());
    }
}

/// A root that is absent, reached through a link or of another kind, a
/// directory between it and the worktree that is a link, an artifact that
/// is a link, a directory or one of two links, a worktree that is the root,
/// lies inside it or holds it under another spelling, and a workdir that
/// cannot be observed each refuse the box by their own variant and text,
/// as the public callers report it; one regular artifact does not.
#[test]
fn a_linked_or_shared_artifact_refuses_the_box_by_its_own_cause() {
    type Plant = fn(&Path, &Path) -> (PathBuf, PathBuf, Unprotected);
    let cases: [(&str, Plant); 10] = [
        ("absent", |_, work| {
            let artifacts = work.join(".forge/artifacts");
            let kind = std::io::ErrorKind::NotFound;
            (
                work.into(),
                artifacts.clone(),
                Unprotected::Unobserved(artifacts, kind),
            )
        }),
        ("forge", |root, work| {
            let elsewhere = made(root, "elsewhere/artifacts");
            std::os::unix::fs::symlink(elsewhere.parent().unwrap(), work.join(".forge")).unwrap();
            let forge = work.join(".forge");
            (
                work.into(),
                forge.join("artifacts"),
                Unprotected::Directory(forge),
            )
        }),
        ("root", |_, work| {
            std::fs::create_dir(work.join(".forge")).unwrap();
            let artifacts = work.join(".forge/artifacts");
            std::fs::write(&artifacts, "").unwrap();
            (
                work.into(),
                artifacts.clone(),
                Unprotected::Directory(artifacts),
            )
        }),
        ("digests", |root, work| {
            let artifacts = library(work);
            let digests = artifacts.join("sha256");
            std::os::unix::fs::symlink(root, &digests).unwrap();
            (work.into(), artifacts, Unprotected::Directory(digests))
        }),
        ("link", |root, work| {
            let artifacts = library(work);
            let artifact = publish(&artifacts, "evidence\n");
            std::fs::remove_file(&artifact).unwrap();
            std::os::unix::fs::symlink(root.join("forged"), &artifact).unwrap();
            (work.into(), artifacts, Unprotected::Artifact(artifact))
        }),
        ("hardlink", |_, work| {
            let artifacts = library(work);
            let artifact = publish(&artifacts, "evidence\n");
            std::fs::hard_link(&artifact, work.join("alias")).unwrap();
            (work.into(), artifacts, Unprotected::Artifact(artifact))
        }),
        ("directory", |_, work| {
            let artifacts = library(work);
            let artifact = publish(&artifacts, "evidence\n");
            std::fs::remove_file(&artifact).unwrap();
            std::fs::create_dir(&artifact).unwrap();
            (work.into(), artifacts, Unprotected::Artifact(artifact))
        }),
        ("worktree inside", |_, work| {
            let inner = made(work, "inner");
            (inner.clone(), work.into(), Unprotected::Alias(inner))
        }),
        ("spelled apart", |root, work| {
            let artifacts = library(work);
            std::os::unix::fs::symlink(work, root.join("spelled")).unwrap();
            let spelled = root.join("spelled");
            (spelled.clone(), artifacts, Unprotected::Alias(spelled))
        }),
        ("parent", |_, work| {
            let artifacts = library(work);
            let climbed = work.join(".forge/../.forge/artifacts");
            assert!(climbed.is_dir() && artifacts.is_dir());
            (
                work.into(),
                climbed.clone(),
                Unprotected::Directory(climbed),
            )
        }),
    ];
    for (name, plant) in cases {
        let (dir, root, work) = fixture();
        let (workdir, artifacts, expected) = plant(&root, &work);
        let built = boxed(&HandsSpec::default(), &workdir, &artifacts, dir.path());
        assert_eq!(refused(built), expected, "{name}");
    }
    let (dir, root, work) = fixture();
    let artifacts = library(&work);
    publish(&artifacts, "evidence\n");
    assert!(boxed(&HandsSpec::default(), &work, &artifacts, dir.path()).is_ok());
    let absent = root.join("absent");
    let built = boxed(&HandsSpec::default(), &absent, &artifacts, dir.path());
    let kind = std::io::ErrorKind::NotFound;
    assert_eq!(refused(built), Unprotected::Unobserved(absent, kind));
}

/// An artifact root this observer cannot search refuses: whether it has
/// published anything is unobserved, never taken for nothing published.
#[test]
fn an_unsearchable_artifact_root_refuses_unobserved() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, _root, work) = fixture();
    let artifacts = library(&work);
    let unsearchable = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(&artifacts, unsearchable).unwrap();
    let built = boxed(&HandsSpec::default(), &work, &artifacts, dir.path());
    let denied = std::io::ErrorKind::PermissionDenied;
    let digests = artifacts.join("sha256");
    assert_eq!(refused(built), Unprotected::Unobserved(digests, denied));
}

/// Each cause's operator text, pinned once, as the workspace tool and an
/// exec box report it before any bubblewrap is spawned; the empty root a
/// driver input without one binds refuses with the same cause.
#[test]
fn each_cause_states_the_managed_writer_refusal() {
    let at = Path::new("/w/.forge");
    let cause = "capability evidence root cannot be protected from every managed writer";
    assert_eq!(UNPROTECTED_EVIDENCE, cause);
    for (unprotected, text) in [
        (
            Unprotected::Directory(at.into()),
            "'/w/.forge' is not a directory of its own",
        ),
        (
            Unprotected::Artifact(at.join("a")),
            "artifact '/w/.forge/a' is not a regular file with a single link",
        ),
        (
            Unprotected::Alias(at.into()),
            "the writable mount '/w/.forge' is the artifact root, holds it or lies inside it",
        ),
        (
            Unprotected::Unobserved(at.into(), std::io::ErrorKind::NotFound),
            "'/w/.forge' cannot be observed: entity not found",
        ),
    ] {
        assert_eq!(unprotected.to_string(), format!("{cause}: {text}"));
    }
    let (_dir, root, work) = fixture();
    let artifacts = work.join(".forge/artifacts");
    std::fs::create_dir(work.join(".forge")).unwrap();
    std::os::unix::fs::symlink(&root, &artifacts).unwrap();
    let (spec, none, absent) = (HandsSpec::default(), &[], root.join("none"));
    let refusal = format!(
        "hands namespace: {cause}: '{}' is not a directory of its own",
        artifacts.display()
    );
    let (home, scratch, timeout) = (
        root.join("home"),
        root.join("scratch"),
        Duration::from_secs(1),
    );
    let reach = sealed(&spec, &work, &home);
    let called = execute_in(
        &absent, &spec, &work, &artifacts, &reach, &scratch, &root, none, "true", timeout,
    );
    assert_eq!(called, Err(refusal.clone()));
    let boxed = run_boxed_in(
        &absent,
        &spec,
        &work,
        &artifacts,
        &reach,
        &root,
        none,
        None,
        &one("true"),
    );
    assert_eq!(boxed, Err(refusal));
    let empty = run_boxed_in(
        &absent,
        &spec,
        &work,
        Path::new(""),
        &reach,
        &root,
        none,
        None,
        &one("true"),
    );
    assert_eq!(
        empty,
        Err(format!(
            "hands namespace: {cause}: '' cannot be observed: invalid input parameter"
        ))
    );
}

/// No author names the protection: the spec's vocabulary stays closed, so
/// a key that would switch it is refused, and the spec a box is handed
/// carries the authored keys alone.
#[test]
fn no_authored_key_names_the_protection() {
    let refused = HandsSpec::parse(&json!({"kind": "workspace", "artifacts": "rw"}));
    let known = "hands has unknown key 'artifacts'; known: kind, network, binds";
    assert_eq!(refused, Err(known.to_string()));
    let value = HandsSpec::default().to_value();
    assert_eq!(
        value,
        json!({"kind": "workspace", "network": false, "binds": []})
    );
}

/// In a real namespace a published artifact in the worktree keeps its
/// bytes and identity through every write, replacement, link and rename
/// the box tries, from the workspace tool and from an exec box alike,
/// while the box reads it and writes the worktree, `.forge` and a
/// declared bind as before.
#[cfg(target_os = "linux")]
#[test]
fn a_box_cannot_change_the_evidence_it_reads() {
    if !can_create_namespace() {
        skip_boundary_proof(
            boundary_evidence_required(),
            "this environment cannot create a bubblewrap namespace",
        );
        return;
    }
    let (dir, root, work) = fixture();
    let artifacts = library(&work);
    let artifact = publish(&artifacts, "evidence\n");
    let pad = made(&root, "pad");
    let spec = rw(&pad, "rw");
    let owner = Session::create_under(dir.path(), "test").unwrap();
    let long = Duration::from_secs(30);
    let reach = sealed(&spec, &work, &root.join("home"));
    let run = |command: &str| {
        let ran = execute(
            &spec,
            &work,
            &artifacts,
            &reach,
            owner.path(),
            command,
            long,
        );
        ran.unwrap()
    };
    let held = [&artifact, &artifacts, &work.join(".forge")];
    let before: Vec<(u64, u64)> = held.iter().map(|path| identity(path)).collect();
    let a = format!(".forge/artifacts/sha256/{HEX}");
    for tamper in [
        format!("echo forged > {a}"),
        format!(": > {a}"),
        format!("rm -f {a}"),
        format!("mv {a} moved"),
        format!("ln {a} alias"),
        format!("ln -sf /etc/hosts {a}"),
        format!("ln -s {a} through && echo forged > through"),
        format!("chmod 666 {a}"),
        "touch .forge/artifacts/sha256/planted".to_string(),
        "mv .forge/artifacts .forge/old".to_string(),
        "rm -rf .forge/artifacts".to_string(),
        "mv .forge forge-old".to_string(),
    ] {
        let tried = run(&tamper);
        assert_ne!(tried.exit_code, 0, "{tamper}: {}", tried.render());
    }
    let read = run(&format!(
        "cat {a} && echo made > made && echo r > .forge/result && echo p > {}/p",
        pad.display()
    ));
    assert_eq!(
        (read.exit_code, read.stdout.as_str()),
        (0, "evidence\n"),
        "{}",
        read.render()
    );
    for (path, text) in [
        (work.join("made"), "made\n"),
        (work.join(".forge/result"), "r\n"),
        (pad.join("p"), "p\n"),
    ] {
        assert_eq!(std::fs::read_to_string(path).unwrap(), text);
    }
    let script =
        format!("echo forged > {a}; mv .forge forge-old; mv .forge/artifacts x; echo e > exec");
    let command = ["/bin/sh".to_string(), "-c".to_string(), script];
    let bwrap = require_bwrap().unwrap();
    let session = root.join("exec");
    let boxed = run_boxed_in(
        &bwrap,
        &spec,
        &work,
        &artifacts,
        &reach,
        &session,
        &[],
        None,
        &command,
    );
    assert_eq!(boxed, Ok(0));
    assert_eq!(std::fs::read_to_string(work.join("exec")).unwrap(), "e\n");
    assert_eq!(std::fs::read_to_string(&artifact).unwrap(), "evidence\n");
    let after: Vec<(u64, u64)> = held.iter().map(|path| identity(path)).collect();
    assert_eq!(after, before);
    let names = std::fs::read_dir(artifacts.join("sha256")).unwrap();
    let names: Vec<_> = names.map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(names, [HEX]);
    assert_eq!(std::fs::metadata(&artifact).unwrap().nlink(), 1);
}

/// In a real namespace an adopter's root outside the worktree is not
/// there at all: the box can neither read, list, write nor plant beside
/// it, its artifact keeps its bytes and identity, and the worktree write
/// lands.
#[cfg(target_os = "linux")]
#[test]
fn a_box_cannot_reach_a_root_outside_every_bind() {
    if !can_create_namespace() {
        skip_boundary_proof(
            boundary_evidence_required(),
            "this environment cannot create a bubblewrap namespace",
        );
        return;
    }
    let (dir, root, work) = fixture();
    let artifacts = made(&root, "adopter/.brokkr/state/artifacts");
    let artifact = publish(&artifacts, "evidence\n");
    let before = [identity(&artifact), identity(&artifacts)];
    let owner = Session::create_under(dir.path(), "test").unwrap();
    let (spec, long) = (HandsSpec::default(), Duration::from_secs(30));
    let reach = sealed(&spec, &work, &root.join("home"));
    let a = artifact.display();
    for tamper in [
        format!("cat {a}"),
        format!("ls {}", artifacts.display()),
        format!("echo forged > {a}"),
        format!("rm {a}"),
        format!("touch {}/sha256/planted", artifacts.display()),
    ] {
        let tried = execute(
            &spec,
            &work,
            &artifacts,
            &reach,
            owner.path(),
            &tamper,
            long,
        );
        let tried = tried.unwrap();
        assert_ne!(tried.exit_code, 0, "{tamper}: {}", tried.render());
    }
    let wrote = execute(
        &spec,
        &work,
        &artifacts,
        &reach,
        owner.path(),
        "echo w > made",
        long,
    );
    assert_eq!(wrote.unwrap().exit_code, 0);
    assert_eq!(std::fs::read_to_string(work.join("made")).unwrap(), "w\n");
    assert_eq!(std::fs::read_to_string(&artifact).unwrap(), "evidence\n");
    assert_eq!([identity(&artifact), identity(&artifacts)], before);
    let names = std::fs::read_dir(artifacts.join("sha256")).unwrap();
    let names: Vec<_> = names.map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(names, [HEX]);
}

/// A protection mount bubblewrap cannot make refuses the box: the root
/// gone between the build and the spawn fails the namespace, and the
/// command never runs without it.
#[cfg(target_os = "linux")]
#[test]
fn a_protection_mount_that_cannot_be_made_refuses_the_command() {
    if !can_create_namespace() {
        skip_boundary_proof(
            boundary_evidence_required(),
            "this environment cannot create a bubblewrap namespace",
        );
        return;
    }
    let (dir, _root, work) = fixture();
    let artifacts = library(&work);
    let ran = work.join("ran");
    let command = [
        "/bin/sh".to_string(),
        "-c".to_string(),
        format!("echo ran > {}", ran.display()),
    ];
    let spec = HandsSpec::default();
    let (home, scratch) = (dir.path().join("home"), dir.path().join("scratch"));
    let built = box_argv(
        &spec,
        &work,
        &artifacts,
        &sealed(&spec, &work, &home),
        &scratch,
        OverlayWrites::Ram,
        &[],
        None,
        &command,
    );
    let argv = built.unwrap();
    std::fs::remove_dir_all(&artifacts).unwrap();
    let status = Command::new(require_bwrap().unwrap())
        .args(&argv[1..])
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1));
    assert!(!ran.exists(), "the command ran without its protection");
}

/// What was observed is observed again as the argv is final (L1): a pinned
/// directory or the root replaced under its own name, a writable source
/// swapped for another directory, and one absent then that now resolves
/// to the root each refuse by the path that changed; nothing changed
/// confirms.
#[test]
fn a_directory_or_source_changed_after_its_observation_refuses() {
    type Change = fn(&Path, &Path, &Path) -> PathBuf;
    let cases: [(&str, Change); 4] = [
        ("pinned", |_, work, _| {
            let forge = work.join(".forge");
            std::fs::rename(&forge, work.join("forge-old")).unwrap();
            std::fs::create_dir_all(forge.join("artifacts")).unwrap();
            forge
        }),
        ("root", |_, _, artifacts| {
            let old = artifacts.with_file_name("old");
            std::fs::rename(artifacts, old).unwrap();
            std::fs::create_dir(artifacts).unwrap();
            artifacts.into()
        }),
        ("swapped", |root, _, _| {
            let pad = root.join("pad");
            std::fs::rename(&pad, root.join("pad-old")).unwrap();
            std::fs::create_dir(&pad).unwrap();
            pad
        }),
        ("appeared", |root, _, artifacts| {
            let absent = root.join("absent");
            std::os::unix::fs::symlink(artifacts, &absent).unwrap();
            absent
        }),
    ];
    for (name, change) in cases {
        let (_dir, root, work) = fixture();
        let artifacts = library(&work);
        let pad = made(&root, "pad");
        let mut protected = Protected::observe(&artifacts, &work).unwrap();
        for host in [&pad, &root.join("absent")] {
            protected.exclude(host).unwrap();
        }
        assert_eq!(protected.confirm(), Ok(()), "{name}");
        let changed = change(&root, &work, &artifacts);
        let refused = protected.confirm();
        assert_eq!(refused, Err(Unprotected::Changed(changed)), "{name}");
    }
    let at = Path::new("/w/.forge");
    assert_eq!(
        Unprotected::Changed(at.into()).to_string(),
        format!(
            "{UNPROTECTED_EVIDENCE}: '/w/.forge' changed between its observation and the \
             box's mounts"
        )
    );
}

/// One recorded mount table line: `id`, on `dev`, its filesystem's `root`
/// mounted at `point`, with octal escapes as the kernel writes them.
#[cfg(target_os = "linux")]
fn line(id: u64, dev: &str, root: &str, point: &str) -> String {
    format!("{id} 1 {dev} {root} {point} rw,relatime shared:1 - ext4 /dev/sda1 rw")
}

/// A recorded mount table of `lines`, each ended as the kernel ends it.
#[cfg(target_os = "linux")]
fn table(lines: &[String]) -> Vec<u8> {
    let ended = lines.iter().map(|line| format!("{line}\n"));
    ended.collect::<String>().into_bytes()
}

/// [`shown_elsewhere`]'s verdict alone, without the records it keeps.
#[cfg(target_os = "linux")]
fn shown(table: &[u8], mount: u64, canonical: &Path) -> Result<(), Unprotected> {
    shown_elsewhere(table, mount, canonical).map(drop)
}

/// H2, over recorded tables: the root `/w/.forge/artifacts` lies on mount
/// 21. Any other mount of its device that shows it, a directory holding it
/// or anything inside it, a single file included, outside the root's own
/// path refuses, wherever it is mounted and however its name is escaped;
/// so does a mount of another device inside the root that is shown
/// elsewhere. A bind of an unrelated directory of the same device, the
/// same path on another device, and a bind of part of the root at its own
/// place are no alias. A table that does not parse, names no mount 21, or
/// whose mount 21 does not hold the root proves nothing.
#[cfg(target_os = "linux")]
#[test]
fn a_mount_that_shows_the_root_elsewhere_refuses_by_the_table() {
    let canonical = Path::new("/w/.forge/artifacts");
    let own = line(21, "8:1", "/", "/");
    let mounted = |place: &str| Err(Unprotected::Mounted(place.into()));
    let (digests, hex) = (
        "/w/.forge/artifacts/sha256",
        format!("/w/.forge/artifacts/sha256/{HEX}"),
    );
    for (name, extra, expected) in [
        (
            "directory",
            line(30, "8:1", digests, "/else"),
            mounted("/else"),
        ),
        ("file", line(31, "8:1", &hex, "/x/file"), mounted("/x/file")),
        (
            "nested",
            line(32, "8:1", canonical.to_str().unwrap(), "/w/alias"),
            mounted("/w/alias"),
        ),
        (
            "holder",
            line(33, "8:1", "/w", "/mnt/w"),
            mounted("/mnt/w/.forge/artifacts"),
        ),
        (
            "whole",
            line(34, "8:1", "/", "/d"),
            mounted("/d/w/.forge/artifacts"),
        ),
        (
            "escaped",
            line(35, "8:1", "/w/.forge", "/a\\040b"),
            mounted("/a b/artifacts"),
        ),
        ("unrelated", line(36, "8:1", "/w/src", "/else/src"), Ok(())),
        ("other device", line(37, "8:2", digests, "/x"), Ok(())),
        ("own place", line(38, "8:1", digests, digests), Ok(())),
    ] {
        let found = shown(&table(&[own.clone(), extra]), 21, canonical);
        assert_eq!(found, expected, "{name}");
    }
    let inner = line(50, "0:60", "/", digests);
    let elsewhere = line(51, "0:60", "/", "/scratch");
    let found = shown(&table(&[own.clone(), inner.clone()]), 21, canonical);
    assert_eq!(found, Ok(()));
    let found = shown(&table(&[own.clone(), inner, elsewhere]), 21, canonical);
    assert_eq!(found, mounted("/scratch"));
    // The worktree a bind of `/srv/w`: the path below the bind's own root
    // is the root's in the filesystem, and the whole filesystem mounted at
    // `/` shows it there too.
    let bound = line(40, "8:1", "/srv/w", "/w");
    let alias = line(41, "8:1", "/srv/w/.forge/artifacts/sha256", "/x");
    let found = shown(&table(&[bound.clone(), alias]), 40, canonical);
    assert_eq!(found, mounted("/x"));
    let found = shown(&table(&[own.clone(), bound]), 40, canonical);
    assert_eq!(found, mounted("/srv/w/.forge/artifacts"));

    let unmapped = Err(Unprotected::Unmapped(canonical.into()));
    let other = line(22, "8:1", "/", "/other");
    for (name, text, mount) in [
        ("unparsable", b"21 1 8:1 / /\n".to_vec(), 21),
        ("no mount", table(std::slice::from_ref(&own)), 99),
        ("not holding", table(&[own, other]), 22),
    ] {
        assert_eq!(shown(&text, mount, canonical), unmapped, "{name}");
    }
    let cause = UNPROTECTED_EVIDENCE;
    assert_eq!(
        Unprotected::Mounted("/x".into()).to_string(),
        format!(
            "{cause}: the mount at '/x' shows the artifact root or part of it outside the \
             root's own path"
        )
    );
    assert_eq!(
        Unprotected::Unmapped("/w".into()).to_string(),
        format!("{cause}: the mount table does not prove where '/w' is mounted")
    );
}

/// The live observation reads the table it is handed for the mount the
/// kernel names: a table that cannot be read, or a root the kernel cannot
/// answer for, refuses with the error kind kept; this host's own table
/// proves a fresh root. An answer that names no mount, as a kernel before
/// statx's mount id gives, refuses as unproved over that same table.
#[cfg(target_os = "linux")]
#[test]
fn a_root_whose_mount_is_unproved_refuses() {
    use std::io::ErrorKind::{NotFound, PermissionDenied};
    let (_dir, root, work) = fixture();
    let artifacts = library(&work);
    let live = || std::fs::read("/proc/self/mountinfo");
    assert_eq!(unaliased_with(&artifacts, live).map(drop), Ok(()));
    let unread = unaliased_with(&artifacts, || Err(PermissionDenied.into())).map(drop);
    let table = PathBuf::from("/proc/self/mountinfo");
    assert_eq!(
        unread,
        Err(Unprotected::Unobserved(table, PermissionDenied))
    );
    let absent = root.join("absent");
    let found = unaliased_with(&absent, live).map(drop);
    assert_eq!(found, Err(Unprotected::Unobserved(absent, NotFound)));
    let unnamed = unaliased_on(&artifacts, Contained::Unnamed, live).map(drop);
    assert_eq!(unnamed, Err(Unprotected::Unmapped(artifacts)));
}

/// Set for the copy of this test binary a mount namespace runs: the
/// fixture root it builds a box over from inside.
#[cfg(target_os = "linux")]
const ALIASES: &str = "BROKKR_TEST_EVIDENCE_ALIASES";

/// H2 and H3 in real mount namespaces: a bind mount that shows the
/// digests, one published file, or the worktree holding the root at
/// another path, or the digests inside the worktree, refuses the box built
/// there by the place it shows them at; an unrelated bind of the same
/// device builds it, and a bind of the digests at their own place, inside
/// the root, refuses it as any mount there does (H7). A
/// writable overlay inside the worktree whose upper directory is the root
/// refuses by its point and that upper directory, which this namespace
/// cannot observe (bubblewrap names its layers under `/oldroot`); a
/// read-only overlay there reading the root, and the writable one out of
/// the box's reach, build. Each runs
/// this test again in its own namespace, which writes what it observed.
#[cfg(target_os = "linux")]
#[test]
fn a_bind_mount_alias_of_the_evidence_refuses_the_box() {
    if let Some(root) = std::env::var_os(ALIASES) {
        let root = PathBuf::from(root);
        let (work, scratch) = (root.join("work"), tempfile::tempdir().unwrap());
        let artifacts = work.join(".forge/artifacts");
        let spec = rw(&root.join("rw"), "rw");
        let built = boxed(&spec, &work, &artifacts, scratch.path());
        let observed = built.map_or_else(|error| error.to_string(), |_| "built".to_string());
        return std::fs::write(root.join("observed"), observed).unwrap();
    }
    if !can_create_namespace() {
        skip_boundary_proof(
            boundary_evidence_required(),
            "this environment cannot create a bubblewrap namespace",
        );
        return;
    }
    let (_dir, root, work) = fixture();
    let artifacts = library(&work);
    let artifact = publish(&artifacts, "evidence\n");
    let digests = artifacts.join("sha256");
    for place in [
        "alias",
        "mirror",
        "pad",
        "view",
        "wk1",
        "wk2",
        "wk3",
        "rw/view",
        "work/nested",
        "work/view",
    ] {
        made(&root, place);
    }
    std::fs::write(root.join("file"), "").unwrap();
    let refused = |place: PathBuf| Unprotected::Mounted(place).to_string();
    // bubblewrap names an overlay's upper directory under its `/oldroot`,
    // which this namespace cannot see once the box stands.
    let upper = Path::new("/oldroot").join(artifacts.strip_prefix("/").unwrap());
    let not_found = std::io::ErrorKind::NotFound;
    let unfollowed =
        |at: PathBuf| Unprotected::Unfollowed(at, upper.clone(), not_found).to_string();
    let mirrored = root.join("mirror/.forge/artifacts");
    let bind = |from: &Path, to: &Path| vec!["--bind".into(), from.into(), to.into()];
    let (pad, view) = (root.join("pad"), work.join("view"));
    let overlay = |work: &str, at: &Path| -> Vec<std::ffi::OsString> {
        let flags = [
            "--overlay-src".into(),
            pad.clone().into(),
            "--overlay".into(),
        ];
        let layers = [artifacts.clone().into(), root.join(work).into(), at.into()];
        flags.into_iter().chain(layers).collect()
    };
    let read_only = ["--overlay-src", "--overlay-src", "--ro-overlay"];
    let read_only = read_only.into_iter().map(Into::into);
    let read_only = (read_only.zip([&artifacts, &pad, &view]))
        .flat_map(|(flag, path)| [flag, path.into()])
        .collect();
    for (args, expected) in [
        (
            bind(&digests, &root.join("alias")),
            refused(root.join("alias")),
        ),
        (
            bind(&artifact, &root.join("file")),
            refused(root.join("file")),
        ),
        (bind(&work, &root.join("mirror")), refused(mirrored)),
        (
            bind(&digests, &work.join("nested")),
            refused(work.join("nested")),
        ),
        (bind(&pad, &root.join("view")), "built".to_string()),
        (
            bind(&digests, &digests),
            Unprotected::Nested(digests.clone()).to_string(),
        ),
        (overlay("wk1", &view), unfollowed(view.clone())),
        (read_only, "built".to_string()),
        (overlay("wk2", &root.join("view")), "built".to_string()),
        (
            overlay("wk3", &root.join("rw/view")),
            unfollowed(root.join("rw/view")),
        ),
    ] {
        let mut command = Command::new(require_bwrap().unwrap());
        command.args(["--dev-bind", "/", "/"]).args(&args);
        let name = "hands::tests::evidence::a_bind_mount_alias_of_the_evidence_refuses_the_box";
        let exe = std::env::current_exe().unwrap();
        command.arg("--").arg(exe).args(["--exact", name]);
        let ran = command.env(ALIASES, &root).output().unwrap();
        let out = String::from_utf8_lossy(&ran.stdout);
        let err = String::from_utf8_lossy(&ran.stderr);
        assert_eq!(ran.status.code(), Some(0), "{out}{err}");
        let observed = std::fs::read_to_string(root.join("observed")).unwrap();
        assert_eq!(observed, expected, "{args:?}");
        std::fs::remove_file(root.join("observed")).unwrap();
    }
}

/// One recorded mount table line: `id` at `point`, a `filesystem` from
/// `source` with the super options `supers`, written as the kernel writes
/// them, on a device of its own, `0:<id>`.
#[cfg(target_os = "linux")]
fn typed(id: u64, point: &str, filesystem: &str, source: &str, supers: &str) -> String {
    format!("{id} 1 0:{id} / {point} rw,relatime - {filesystem} {source} {supers}")
}

/// A recorded directory's identity: one per spelling, the same for the
/// same spelling, as a fixture of the edge's observation.
#[cfg(target_os = "linux")]
fn known(at: &Path) -> (u64, u64) {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    at.hash(&mut hasher);
    (7, hasher.finish())
}

/// What the recorded edge observes beyond its defaults: a source's mount
/// as named, a layer as seen, and a mount source as found. By default a
/// source lies on the mount whose point is its longest prefix, a layer is
/// the directory it names unless it lies under `/var/lib/docker` or
/// `/oldroot`, which this namespace cannot see, and a source is absent.
#[cfg(target_os = "linux")]
#[derive(Default)]
struct Observe<'o> {
    mounts: &'o [(&'o str, Contained)],
    layers: &'o [(&'o str, Seen)],
    blocks: &'o [(&'o str, Block)],
}

#[cfg(target_os = "linux")]
impl Observe<'_> {
    /// Where the recorded edge places the source `at` among `records`.
    fn mount(&self, records: &[Record], at: &Path) -> Contained {
        let named = self.mounts.iter().find(|(path, _)| Path::new(path) == at);
        let placed = (records.iter())
            .filter(|record| at.starts_with(&record.point))
            .max_by_key(|record| record.point.components().count());
        named.map_or(Contained::Mount(placed.unwrap().id), |(_, mount)| *mount)
    }

    /// What the recorded edge sees of the layer `at`.
    fn layer(&self, at: &Path) -> Seen {
        let named = self.layers.iter().find(|(path, _)| Path::new(path) == at);
        let unseen = at.starts_with("/var/lib/docker") | at.starts_with("/oldroot");
        let found = match unseen {
            true => Err(std::io::ErrorKind::NotFound),
            false => Ok((at.to_path_buf(), known(at))),
        };
        named.map_or(found, |(_, seen)| seen.clone())
    }
}

/// The general rule's verdict over a recorded table of `lines`, the root's
/// own mount 21 among them, beside the root `canonical` and the writable
/// `sources`, over the facts `observe` records for exactly what the
/// verdict wants.
#[cfg(target_os = "linux")]
fn judge(
    lines: &[String],
    canonical: &str,
    sources: &[&str],
    observe: &Observe<'_>,
) -> Result<(), Unprotected> {
    let records = super::super::namespace::sources::host::records(&table(lines), 64).unwrap();
    rule(&records, canonical, sources, observe)
}

/// [`judge`] over `records` already parsed.
#[cfg(target_os = "linux")]
fn rule(
    records: &[Record],
    canonical: &str,
    sources: &[&str],
    observe: &Observe<'_>,
) -> Result<(), Unprotected> {
    let canonical = Path::new(canonical);
    let holders = canonical.ancestors().map(known).collect();
    let root = Root {
        canonical,
        own: 21,
        holders: &holders,
    };
    let sources: Vec<Source> = (sources.iter().map(Path::new))
        .map(|path| Source {
            path: path.to_path_buf(),
            mount: observe.mount(records, path),
        })
        .collect();
    let (layers, blocks) = wanted(records, &root, &sources);
    let found = |at: &PathBuf| {
        observe
            .blocks
            .iter()
            .find(|(path, _)| Path::new(path) == at)
    };
    let absent = Block::Unobserved(std::io::ErrorKind::NotFound);
    let facts = Facts {
        layers: (layers.into_iter())
            .map(|at| (at.clone(), observe.layer(&at)))
            .collect(),
        blocks: (blocks.into_iter())
            .map(|at| (at.clone(), found(&at).map_or(absent, |(_, block)| *block)))
            .collect(),
        sources,
    };
    reached(records, &root, &facts)
}

/// [`judge`] over the root's own mount `own` then `extra`, observed by
/// default.
#[cfg(target_os = "linux")]
fn verdict(
    own: &str,
    extra: &[String],
    canonical: &str,
    sources: &[&str],
) -> Result<(), Unprotected> {
    let lines: Vec<String> = std::iter::once(own.to_string())
        .chain(extra.iter().cloned())
        .collect();
    judge(&lines, canonical, sources, &Observe::default())
}

/// The writable sources of the recorded box: its worktree, an authored rw
/// bind, an authored overlay bind, the bound git common directory, and the
/// private home and tmp.
#[cfg(target_os = "linux")]
const SOURCES: [&str; 6] = ["/w", "/s/rw", "/s/ov", "/g/common", "/p/home", "/p/tmp"];

/// The root of the recorded box, inside its worktree, on mount 21.
#[cfg(target_os = "linux")]
const ROOT: &str = "/w/.forge/artifacts";

/// An overlay reading `/l` that writes through `upper` and `work`.
#[cfg(target_os = "linux")]
fn writes(upper: &str, work: &str) -> String {
    format!("rw,lowerdir=/l,upperdir={upper},workdir={work}")
}

/// The overlay mount 30 at `point` with the super options `supers`.
#[cfg(target_os = "linux")]
fn overlay(point: &str, supers: &str) -> String {
    typed(30, point, "overlay", "overlay", supers)
}

/// H3 and every overlay the box reaches, over recorded tables: an overlay
/// at or under any writable source whose upper or work directory is the
/// root, lies inside it or holds it, by its path or by its identity,
/// refuses by its mount point, however the kernel escaped the path. The
/// same overlay out of reach, one writing elsewhere, and one that only
/// reads the root, its writes copied up, are no alias. An upper directory
/// is read as the kernel reads it, unescaped, never in a second spelling.
#[cfg(target_os = "linux")]
#[test]
fn an_overlay_the_box_reaches_that_writes_into_the_root_refuses() {
    let own = line(21, "8:1", "/", "/");
    let overlaid = |point: &str| Err(Unprotected::Overlaid(point.into()));
    let into = writes(ROOT, "/wk");
    let refused = [
        ("H3", "/w/view", into.clone()),
        (
            "upper inside",
            "/w/v",
            writes("/w/.forge/artifacts/sha256", "/wk"),
        ),
        ("upper holds", "/w/v", writes("/w/.forge", "/wk")),
        ("work is root", "/w/v", writes("/u", ROOT)),
        ("work inside", "/w/v", writes("/u", "/w/.forge/artifacts/t")),
        ("work holds", "/w/v", writes("/u", "/w")),
        (
            "backslash",
            "/w/v",
            writes("/w/.forge/\\134artifacts", "/wk"),
        ),
        ("rw bind", "/s/rw/v", into.clone()),
        ("overlay bind", "/s/ov", into.clone()),
        ("git common", "/g/common/v", into.clone()),
        ("private home", "/p/home/v", into.clone()),
        ("private tmp", "/p/tmp/v", into.clone()),
    ];
    for (name, point, supers) in refused {
        let found = verdict(&own, &[overlay(point, &supers)], ROOT, &SOURCES);
        assert_eq!(found, overlaid(point), "{name}");
    }
    let built = [
        ("out of reach", "/else/v", into),
        ("elsewhere", "/w/v", writes("/u", "/wk")),
        (
            "copy-up",
            "/w/v",
            format!("rw,lowerdir={ROOT}:/l,upperdir=/u,workdir=/wk"),
        ),
        ("read-only", "/w/v", format!("ro,lowerdir={ROOT}/sha256")),
        (
            "escaped lower",
            "/w/v",
            "rw,lowerdir=/a\\134:b:/l,upperdir=/u,workdir=/wk".into(),
        ),
    ];
    for (name, point, supers) in built {
        let found = verdict(&own, &[overlay(point, &supers)], ROOT, &SOURCES);
        assert_eq!(found, Ok(()), "{name}");
    }
    // The kernel unescapes an upper directory: `/w/a\b` shown is `/w/ab`.
    let unescaped = overlay("/w/v", &writes("/w/a\\134b", "/wk"));
    let found = verdict(&own, std::slice::from_ref(&unescaped), "/w/ab", &SOURCES);
    assert_eq!(found, overlaid("/w/v"));
    assert_eq!(verdict(&own, &[unescaped], "/w/a\\b", &SOURCES), Ok(()));
    // An upper directory another name for the root, by its identity.
    let alias = [(
        "/mnt/alias",
        Ok(("/mnt/alias".into(), known(Path::new(ROOT)))),
    )];
    let observe = Observe {
        layers: &alias,
        ..Observe::default()
    };
    let lines = [own, overlay("/w/v", &writes("/mnt/alias", "/wk"))];
    assert_eq!(judge(&lines, ROOT, &SOURCES, &observe), overlaid("/w/v"));
}

/// H4, over recorded tables: the mount a writable source lies on, as the
/// edge observed it, is judged like any other. An overlay at `/s` that
/// writes into the root refuses where `/s/rw` or `/s/sha256` lies on it,
/// and so does one at an unrelated point the edge names for a source; a
/// FUSE, NFS or 9p mount holding a source refuses. A source whose mount
/// was not observed, or names no record, refuses by that source.
#[cfg(target_os = "linux")]
#[test]
fn the_mount_a_writable_source_lies_on_is_judged() {
    let own = line(21, "8:1", "/", "/");
    let held = overlay("/s", &writes(ROOT, "/wk"));
    for source in ["/s/rw", "/s/sha256"] {
        let found = verdict(&own, std::slice::from_ref(&held), ROOT, &["/w", source]);
        assert_eq!(found, Err(Unprotected::Overlaid("/s".into())), "{source}");
    }
    let named = [("/x/rw", Contained::Mount(30))];
    let observe = Observe {
        mounts: &named,
        ..Observe::default()
    };
    let lines = [own.clone(), held];
    let found = judge(&lines, ROOT, &["/w", "/x/rw"], &observe);
    assert_eq!(found, Err(Unprotected::Overlaid("/s".into())));
    for (filesystem, source) in [("fuse.sshfs", "u@h:/x"), ("nfs4", "h:/x"), ("9p", "share")] {
        let held = typed(30, "/s", filesystem, source, "rw");
        let found = verdict(&own, &[held], ROOT, &["/w", "/s/rw"]);
        assert_eq!(
            found,
            Err(Unprotected::Stacked("/s".into())),
            "{filesystem}"
        );
    }
    let denied = std::io::ErrorKind::PermissionDenied;
    for (mount, expected) in [
        (
            Contained::Unobserved(denied),
            Unprotected::Unobserved("/s/rw".into(), denied),
        ),
        (Contained::Unnamed, Unprotected::Unmapped("/s/rw".into())),
        (Contained::Mount(99), Unprotected::Unmapped("/s/rw".into())),
    ] {
        let named = [("/s/rw", mount)];
        let observe = Observe {
            mounts: &named,
            ..Observe::default()
        };
        let found = judge(std::slice::from_ref(&own), ROOT, &["/w", "/s/rw"], &observe);
        assert_eq!(found, Err(expected), "{mount:?}");
    }
}

/// An overlay's upper and work directories must be observed wherever it is
/// judged, whatever kept the edge from observing it: each refuses by its
/// point and the layer as the kernel reads it. Its lower layers are only
/// read, so one unobserved there builds.
#[cfg(target_os = "linux")]
#[test]
fn an_overlay_layer_that_cannot_be_observed_refuses() {
    let own = line(21, "8:1", "/", "/");
    let unfollowed = |point: &str, layer: &str, kind| {
        Err(Unprotected::Unfollowed(point.into(), layer.into(), kind))
    };
    let not_found = std::io::ErrorKind::NotFound;
    let found = verdict(
        &own,
        &[overlay("/w/v", &writes("/oldroot/u", "/oldroot/wk"))],
        ROOT,
        &SOURCES,
    );
    assert_eq!(found, unfollowed("/w/v", "/oldroot/u", not_found));
    for kind in [
        std::io::ErrorKind::PermissionDenied,
        std::io::ErrorKind::InvalidData,
        std::io::ErrorKind::NotADirectory,
        std::io::ErrorKind::Other,
    ] {
        let seen = [("/wk", Err(kind))];
        let observe = Observe {
            layers: &seen,
            ..Observe::default()
        };
        let lines = [own.clone(), overlay("/w/v", &writes("/u", "/wk"))];
        let found = judge(&lines, ROOT, &SOURCES, &observe);
        assert_eq!(found, unfollowed("/w/v", "/wk", kind), "{kind:?}");
    }
    let lower = writes("/u", "/wk").replace("/l", "/oldroot/l");
    let found = verdict(&own, &[overlay("/w/v", &lower)], ROOT, &SOURCES);
    assert_eq!(found, Ok(()));
}

/// H7, the root's own mount, over recorded tables: it must be a local
/// filesystem, or one of no known type on its own block device (H5). An
/// overlay refuses, its layers observed and apart from every source,
/// additive or legacy, and so does a container's overlay root; so do a
/// FUSE and an NFS mount. `ext4` and `tmpfs` build.
#[cfg(target_os = "linux")]
#[test]
fn the_root_stands_only_on_a_local_filesystem() {
    let stacked = || Err(Unprotected::Stacked("/".into()));
    let docker = "rw,lowerdir=/var/lib/docker/overlay2/l/ABC,\
                  upperdir=/var/lib/docker/overlay2/x/diff,workdir=/var/lib/docker/overlay2/x/work";
    for supers in [
        writes("/u", "/wk").as_str(),
        "rw,lowerdir+=/l1,lowerdir+=/l2,datadir+=/d,upperdir=/u,workdir=/wk",
        "rw,lowerdir=/l1:/l2::/d1",
        docker,
    ] {
        let rooted = typed(21, "/", "overlay", "overlay", supers);
        assert_eq!(verdict(&rooted, &[], ROOT, &SOURCES), stacked(), "{supers}");
    }
    for (filesystem, source, expected) in [
        ("fuse.sshfs", "u@h:/x", stacked()),
        ("nfs4", "h:/x", stacked()),
        ("ext4", "/dev/sda1", Ok(())),
        ("tmpfs", "tmpfs", Ok(())),
    ] {
        let rooted = typed(21, "/", filesystem, source, "rw");
        let found = verdict(&rooted, &[], ROOT, &SOURCES);
        assert_eq!(found, expected, "{filesystem}");
    }
}

/// H7, inside the root, over recorded tables: a mount whose point lies
/// strictly inside the root refuses whatever its type, a `tmpfs` and a
/// bind of the root's own device alike, whether or not a source reaches
/// it. The root's own mount at the root itself builds.
#[cfg(target_os = "linux")]
#[test]
fn nothing_mounted_inside_the_root_builds() {
    let own = line(21, "8:1", "/", "/");
    let inside = "/w/.forge/artifacts/sha256";
    let nested = || Err(Unprotected::Nested(inside.into()));
    let tmpfs = typed(30, inside, "tmpfs", "tmpfs", "rw");
    let bind = line(31, "8:1", "/srv/x", inside);
    for (name, mounted) in [("tmpfs", tmpfs), ("bind", bind)] {
        let found = verdict(&own, std::slice::from_ref(&mounted), ROOT, &SOURCES);
        assert_eq!(found, nested(), "{name}");
        let found = verdict(&own, &[mounted], ROOT, &["/p/home"]);
        assert_eq!(found, nested(), "{name} unreached");
    }
    let whole = line(20, "8:1", "/", "/");
    let at_root = line(21, "8:1", "/srv/a", ROOT);
    assert_eq!(verdict(&whole, &[at_root], ROOT, &SOURCES), Ok(()));
}

/// H7's three configurations, against the root itself: a writable bind
/// below a source that exposes the root's region, and a writable source
/// bound from the root's parent, are aliases the table shows outside the
/// root's path; another overlay in reach whose upper directory lies in
/// the root writes into it. Each refuses.
#[cfg(target_os = "linux")]
#[test]
fn every_alias_of_the_root_region_refuses() {
    let canonical = Path::new(ROOT);
    let own = line(21, "8:1", "/", "/");
    let exposed = line(30, "8:1", "/w/.forge/artifacts/sha256", "/s/rw/view");
    let found = shown(&table(&[own.clone(), exposed]), 21, canonical);
    assert_eq!(found, Err(Unprotected::Mounted("/s/rw/view".into())));
    let parent = line(31, "8:1", "/w/.forge", "/s/rw");
    let found = shown(&table(&[own.clone(), parent]), 21, canonical);
    assert_eq!(found, Err(Unprotected::Mounted("/s/rw/artifacts".into())));
    let upper = overlay("/g/common/v", &writes("/w/.forge/artifacts/up", "/wk"));
    let found = verdict(&own, &[upper], ROOT, &SOURCES);
    assert_eq!(found, Err(Unprotected::Overlaid("/g/common/v".into())));
}

/// Every stacking or remote filesystem refuses where the box reaches it
/// or where the root stands on it, and lies inside the root only to refuse
/// as any mount there does; local filesystems in reach or under the root,
/// and any of them out of reach, build.
#[cfg(target_os = "linux")]
#[test]
fn a_stacking_remote_or_unknown_filesystem_in_reach_refuses() {
    let own = line(21, "8:1", "/", "/");
    let stacked = |point: &str| Err(Unprotected::Stacked(point.into()));
    let families = [
        ("fuse", "dev"),
        ("fuseblk", "/dev/sdb1"),
        ("fuse.sshfs", "u@h:/x"),
        ("nfs", "h:/x"),
        ("nfs4", "h:/x"),
        ("cifs", "//h/x"),
        ("smb3", "//h/x"),
        ("9p", "share"),
        ("virtiofs", "share"),
        ("ceph", "h:/x"),
        ("glusterfs", "h:/x"),
        ("aufs", "none"),
        ("unionfs", "none"),
        ("zfs", "pool/w"),
    ];
    for (filesystem, source) in families {
        let at = |point: &str| typed(30, point, filesystem, source, "rw");
        assert_eq!(
            verdict(&own, &[at("/w/m")], ROOT, &SOURCES),
            stacked("/w/m"),
            "{filesystem}"
        );
        let inside = at("/w/.forge/artifacts/sha256");
        let found = verdict(&own, &[inside], ROOT, &["/p/home"]);
        let nested = Unprotected::Nested("/w/.forge/artifacts/sha256".into());
        assert_eq!(found, Err(nested), "{filesystem} inside");
        assert_eq!(
            verdict(&own, &[at("/else/m")], ROOT, &SOURCES),
            Ok(()),
            "{filesystem} apart"
        );
        let rooted = typed(21, "/", filesystem, source, "rw");
        assert_eq!(
            verdict(&rooted, &[], ROOT, &SOURCES),
            stacked("/"),
            "{filesystem} own"
        );
    }
    for (filesystem, source) in [("ext4", "/dev/mapper/root"), ("tmpfs", "tmpfs")] {
        let at = typed(30, "/w/m", filesystem, source, "rw");
        assert_eq!(verdict(&own, &[at], ROOT, &SOURCES), Ok(()), "{filesystem}");
        let rooted = typed(21, "/", filesystem, source, "rw");
        assert_eq!(
            verdict(&rooted, &[], ROOT, &SOURCES),
            Ok(()),
            "{filesystem} own"
        );
    }
}

/// H5, over recorded tables: a filesystem of no known type builds only
/// where the edge observed its source to be a block device of the mount's
/// own number. A regular file under `/dev/shm`, a block device of another
/// number, or a source absent here refuses, in reach and as the root's own
/// mount; a known local type needs no device here, as the CI root volume's
/// ext4 on an absent `/dev/mapper` source does not.
#[cfg(target_os = "linux")]
#[test]
fn an_unknown_filesystem_builds_only_on_its_own_block_device() {
    let own = line(21, "8:1", "/", "/");
    let absent = Block::Unobserved(std::io::ErrorKind::NotFound);
    // `None` is a block device of the mount's own number.
    for (source, block, expected) in [
        ("/dev/sdb1", None, Ok(())),
        ("/dev/sdb1", Some(Block::Device((8, 17))), Err(())),
        ("/dev/shm/backing", Some(Block::Other), Err(())),
        ("/dev/sdb1", Some(absent), Err(())),
    ] {
        for (id, point) in [(30, "/w/m"), (21, "/")] {
            let block = block.unwrap_or(Block::Device((0, id)));
            let blocks = [(source, block)];
            let observe = Observe {
                blocks: &blocks,
                ..Observe::default()
            };
            let mounted = typed(u64::from(id), point, "vfat", source, "rw");
            let lines = match id {
                21 => vec![mounted],
                _ => vec![own.clone(), mounted],
            };
            let found = judge(&lines, ROOT, &SOURCES, &observe);
            let refused = Unprotected::Stacked(point.into());
            assert_eq!(found, expected.map_err(|()| refused), "{source} {block:?}");
        }
    }
    let ci = typed(21, "/", "ext4", "/dev/mapper/ci--vg-root", "rw");
    assert_eq!(verdict(&ci, &[], ROOT, &SOURCES), Ok(()));
    // A device named empty, as the kernel writes it: an unknown type has no
    // block device to observe, and a known local one needs none.
    let unnamed = typed(30, "/w/m", "vfat", "", "rw");
    let found = verdict(&own, &[unnamed], ROOT, &SOURCES);
    assert_eq!(found, Err(Unprotected::Stacked("/w/m".into())));
    let unnamed = typed(30, "/w/m", "ext4", "", "rw");
    assert_eq!(verdict(&own, &[unnamed], ROOT, &SOURCES), Ok(()));
    // A stacking type proves nothing by its device.
    let blocks = [("/dev/sdb1", Block::Device((0, 30)))];
    let observe = Observe {
        blocks: &blocks,
        ..Observe::default()
    };
    let lines = [own, typed(30, "/w/m", "fuseblk", "/dev/sdb1", "rw")];
    let found = judge(&lines, ROOT, &SOURCES, &observe);
    assert_eq!(found, Err(Unprotected::Stacked("/w/m".into())));
}

/// The edge observes, once, each fact the verdict reads: the mount the
/// kernel names for each source, each wanted layer as the directory it
/// resolves to or why it does not, and a judged unknown mount's source as
/// what it is, here a character device, or why it cannot be observed. A
/// `statx` answer without a mount id names none, and a block device is
/// read by its major and minor numbers.
#[cfg(target_os = "linux")]
#[test]
fn the_edge_observes_each_fact_the_verdict_reads() {
    use rustix::fs::{statx, AtFlags, StatxFlags, CWD};
    let (_dir, root, work) = fixture();
    let (file, absent) = (root.join("file"), root.join("absent"));
    std::fs::write(&file, "").unwrap();
    let held = statx(CWD, &work, AtFlags::SYMLINK_NOFOLLOW, StatxFlags::MNT_ID);
    let own = held.unwrap().stx_mnt_id;
    let [lower, upper, worker] = [&work, &file, &absent].map(|at| at.display().to_string());
    // The upper directory a directory or absent, the work directory a file.
    let writing = |top: &str| format!("rw,lowerdir=/l,upperdir={top},workdir={upper}");
    let point = format!("{lower}/o");
    let lines = [
        typed(own, "/", "overlay", "overlay", &writing(&lower)),
        typed(own + 3, &point, "overlay", "overlay", &writing(&worker)),
        typed(own + 1, &format!("{lower}/m"), "vfat", "/dev/null", "rw"),
        typed(own + 2, &format!("{lower}/n"), "vfat", &worker, "rw"),
    ];
    let records = super::super::namespace::sources::host::records(&table(&lines), 64).unwrap();
    let canonical = work.join(".forge/artifacts");
    let holders = std::collections::BTreeSet::new();
    let at = Root {
        canonical: &canonical,
        own,
        holders: &holders,
    };
    let facts = observed(&records, &at, std::slice::from_ref(&work));
    assert_eq!(facts.sources[0].mount, Contained::Mount(own));
    let layers = std::collections::BTreeMap::from([
        (work.clone(), Ok((work.clone(), identity(&work)))),
        (file, Err(std::io::ErrorKind::NotADirectory)),
        (absent.clone(), Err(std::io::ErrorKind::NotFound)),
    ]);
    assert_eq!(facts.layers, layers);
    let blocks = std::collections::BTreeMap::from([
        ("/dev/null".into(), Block::Other),
        (absent, Block::Unobserved(std::io::ErrorKind::NotFound)),
    ]);
    assert_eq!(facts.blocks, blocks);
    let mask = StatxFlags::MNT_ID.bits();
    assert_eq!(
        [named(mask, 9), named(0, 9)],
        [Contained::Mount(9), Contained::Unnamed]
    );
    let number = rustix::fs::makedev(8, 17);
    let found = [numbered(true, number), numbered(false, number)];
    assert_eq!(found, [Block::Device((8, 17)), Block::Other]);
}

/// H6: whatever a judged record spells that does not parse strictly
/// refuses by its point, its super options whatever its type: an empty
/// option, an escape that is not whole or not the kernel's, a bare `=` in
/// a layer, a decoded NUL, a layer key with no value, a relative, empty,
/// dotted or deleted layer, a `\` ending an unescaped layer, a second
/// `::`, an overlay that reads no layer, an upper directory without its
/// work directory or twice, and a point the kernel marks deleted. Out of
/// reach they are never read; a point with a `.` or `..` component
/// refuses wherever it is, since no prefix places it.
#[cfg(target_os = "linux")]
#[test]
fn options_that_do_not_parse_refuse() {
    let own = line(21, "8:1", "/", "/");
    for supers in [
        "rw,,lowerdir=/l",
        "rw,lowerdir=/l\\9",
        "rw,lowerdir=/arti\\146acts",
        "rw,lowerdir=/l=x",
        "rw,lowerdir=/l\\000",
        "rw,lowerdir=/l\\134",
        "rw,lowerdir",
        "rw,lowerdir=l",
        "rw,lowerdir=:/l",
        "rw,lowerdir=/l:",
        "rw,lowerdir=/a:::/b",
        "rw,lowerdir=/a::/b::/c",
        "rw,lowerdir+=/l/../x",
        "rw,lowerdir+=/l/./x",
        "rw,lowerdir+=/l\\040(deleted)",
        "rw,lowerdir=/l,upperdir=/u\\134,workdir=/wk",
        "rw,upperdir=/u,workdir=/wk",
        "rw,lowerdir=/l,upperdir=/u",
        "rw,lowerdir=/l,upperdir=/u,workdir=/wk,upperdir=/u2,workdir=/wk2",
    ] {
        let found = verdict(&own, &[overlay("/w/v", supers)], ROOT, &SOURCES);
        assert_eq!(found, Err(Unprotected::Unparsed("/w/v".into())), "{supers}");
        assert_eq!(
            verdict(&own, &[overlay("/else", supers)], ROOT, &SOURCES),
            Ok(()),
            "{supers}"
        );
    }
    let broken = typed(30, "/w/m", "ext4", "/dev/sda2", "rw,,broken\\9");
    let found = verdict(&own, &[broken], ROOT, &SOURCES);
    assert_eq!(found, Err(Unprotected::Unparsed("/w/m".into())));
    let deleted = typed(30, "/w/m\\040(deleted)", "ext4", "/dev/sda2", "rw");
    let found = verdict(&own, &[deleted], ROOT, &SOURCES);
    assert_eq!(found, Err(Unprotected::Unparsed("/w/m (deleted)".into())));
    let dotted = typed(30, "/else/../w/m", "ext4", "/dev/sda2", "rw");
    let found = verdict(&own, &[dotted], ROOT, &SOURCES);
    assert_eq!(found, Err(Unprotected::Unparsed("/else/../w/m".into())));
}

/// H6's probe cases, each a record of the root's device whose root field
/// or optional field the kernel never writes, through the alias check: a
/// root with a `.` or `..` component or the deleted mark refuses by that
/// record's point; a literal NUL in the root, or an optional field other
/// than `shared:N`, `master:N`, `propagate_from:N` and `unbindable`, does
/// not parse, so the table proves nothing.
#[cfg(target_os = "linux")]
#[test]
fn a_field_the_kernel_never_writes_refuses_the_alias_check() {
    let canonical = Path::new(ROOT);
    let own = line(21, "8:1", "/", "/");
    let unparsed = || Err(Unprotected::Unparsed("/else".into()));
    let unmapped = || Err(Unprotected::Unmapped(canonical.into()));
    let tagged = line(30, "8:1", "/tree", "/else").replace("shared:1", "shared:\\9");
    for (name, extra, expected) in [
        ("dot", line(30, "8:1", "/tree/./root", "/else"), unparsed()),
        (
            "dot-dot",
            line(30, "8:1", "/tree/../root", "/else"),
            unparsed(),
        ),
        (
            "deleted",
            line(30, "8:1", "/tree\\040(deleted)", "/else"),
            unparsed(),
        ),
        (
            "deleted root",
            line(30, "8:1", "/tree//deleted", "/else"),
            unparsed(),
        ),
        ("empty", line(30, "8:1", "/a//b", "/else"), unparsed()),
        ("trailing", line(30, "8:1", "/a/", "/else"), unparsed()),
        ("nul", line(30, "8:1", "/tree\0root", "/else"), unmapped()),
        ("optional", tagged, unmapped()),
    ] {
        let found = shown(&table(&[own.clone(), extra]), 21, canonical);
        assert_eq!(found, expected, "{name}");
    }
    let kept = ["shared:7 master:2 propagate_from:3 unbindable", ""];
    for optional in kept.map(|fields| own.replace("shared:1", fields).replace("  ", " ")) {
        let found = shown(&table(std::slice::from_ref(&optional)), 21, canonical);
        assert_eq!(found, Ok(()), "{optional}");
    }
}

/// H6's probe cases through [`reached`]: a judged record whose root has a
/// `.` or `..` component, the deleted mark or a NUL refuses by its point,
/// whatever its device. A literal NUL or a malformed optional field never
/// parses, so no record holding one reaches the verdict: the parser
/// refuses the table, and the NUL is planted in a parsed record instead.
#[cfg(target_os = "linux")]
#[test]
fn a_field_the_kernel_never_writes_refuses_the_verdict() {
    use super::super::namespace::sources::host::records;
    use std::os::unix::ffi::OsStringExt;
    let own = line(21, "8:1", "/", "/");
    let unparsed = || Err(Unprotected::Unparsed("/w/m".into()));
    let loose = ["/tree/./root", "/tree/../root", "/tree\\040(deleted)"];
    for root in loose.into_iter().chain(["/tree//deleted", "/a//b", "/a/"]) {
        let judged = line(30, "8:2", root, "/w/m");
        assert_eq!(
            verdict(&own, &[judged], ROOT, &SOURCES),
            unparsed(),
            "{root}"
        );
    }
    let whole = line(30, "8:2", "/", "/w/m");
    assert_eq!(verdict(&own, &[whole], ROOT, &SOURCES), Ok(()));
    let mut parsed = records(&table(&[own.clone(), line(30, "8:2", "/t", "/w/m")]), 64);
    let nul = std::ffi::OsString::from_vec(b"/tree\0root".to_vec());
    parsed.as_mut().unwrap()[1].root = nul.into();
    let found = rule(&parsed.unwrap(), ROOT, &SOURCES, &Observe::default());
    assert_eq!(found, unparsed());
    let tagged = own.replace("shared:1", "shared:\\9");
    for unparsable in [line(30, "8:2", "/tree\0root", "/w/m"), tagged] {
        let found = records(&table(std::slice::from_ref(&unparsable)), 64);
        assert_eq!(found, Err(crate::broker::Refusal::Identity), "{unparsable}");
    }
}

/// The operator text of each verdict of the general rule, pinned once.
#[cfg(target_os = "linux")]
#[test]
fn each_mount_verdict_states_the_managed_writer_refusal() {
    let cause = UNPROTECTED_EVIDENCE;
    let at = PathBuf::from("/w/v");
    let kind = std::io::ErrorKind::PermissionDenied;
    for (unprotected, text) in [
        (
            Unprotected::Unparsed(at.clone()),
            "the options of the mount at '/w/v' do not parse",
        ),
        (
            Unprotected::Overlaid(at.clone()),
            "the overlay at '/w/v' writes into the artifact root, inside it or into a directory \
             holding it",
        ),
        (
            Unprotected::Unfollowed(at.clone(), "/oldroot/u".into(), kind),
            "the overlay at '/w/v' names the layer '/oldroot/u', which cannot be observed: \
             permission denied",
        ),
        (
            Unprotected::Nested(at.clone()),
            "the mount at '/w/v' lies inside the artifact root",
        ),
        (
            Unprotected::Stacked(at),
            "the mount at '/w/v' is a stacking, remote or unknown filesystem whose backing cannot \
             be proved apart from the artifact root",
        ),
    ] {
        assert_eq!(unprotected.to_string(), format!("{cause}: {text}"));
    }
}

/// L1 through `box_argv` itself: a root replaced under its own name at the
/// last step before the argv is final refuses the box by that name, which
/// only the production call to `confirm` can see; untouched, it builds.
#[test]
fn the_box_confirms_its_observation_as_its_argv_is_final() {
    let (dir, _root, work) = fixture();
    let artifacts = library(&work);
    let spec = HandsSpec::default();
    boxed(&spec, &work, &artifacts, dir.path()).unwrap();
    let replaced = artifacts.clone();
    let step = move || {
        std::fs::rename(&replaced, replaced.with_file_name("old")).unwrap();
        std::fs::create_dir(&replaced).unwrap();
    };
    super::super::evidence::CONFIRMING.set(Some(Box::new(step)));
    let built = boxed(&spec, &work, &artifacts, dir.path());
    assert_eq!(refused(built), Unprotected::Changed(artifacts));
}
