//! The evidence an exec seat's box cannot change (decision 0065 slice two,
//! U8a; CR2, MB3): the engine makes the run's artifact root owner-only
//! before launch and hands `hands exec` that root beside the authored spec
//! alone, and the box protects the artifacts already published there, for
//! a seat granted nothing.

use std::os::unix::fs::PermissionsExt;

use super::super::boxed::Unrooted;
use super::*;

// What the namespace proofs alone use: they run on Linux only, and macOS
// clippy denies an unused import.
#[cfg(target_os = "linux")]
use super::super::boxed::Unnamed;
#[cfg(target_os = "linux")]
use brokkr_protocol::hands::{require_bwrap, run_boxed_in, GitFacts};
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;
#[cfg(target_os = "linux")]
use std::process::Stdio;

/// The engine makes a missing artifact root owner-only, with each
/// directory above it that it makes, keeps an existing one as it is, and
/// refuses with CR2's cause where the root cannot be made or is not a
/// directory of its own.
#[test]
fn the_engine_makes_the_artifact_root_owner_only_or_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let run_local = dir
        .path()
        .canonicalize()
        .unwrap()
        .join("adopter/.brokkr/state");
    let artifacts = artifact_root(&run_local);
    assert_eq!(artifacts, run_local.join("artifacts"));
    assert_eq!(make_root(&artifacts), Ok(()));
    for made in [&artifacts, &run_local] {
        let mode = std::fs::metadata(made).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "{} is owner-only", made.display());
    }
    std::fs::set_permissions(&artifacts, std::fs::Permissions::from_mode(0o750)).unwrap();
    assert_eq!(make_root(&artifacts), Ok(()));
    let kept = std::fs::metadata(&artifacts).unwrap().permissions().mode();
    assert_eq!(kept & 0o777, 0o750, "an existing root is kept as it is");
    let cause = "dispatch refused: capability evidence root cannot be protected from every \
                 managed writer: the artifact root";
    let file = dir.path().join("file");
    std::fs::write(&file, "").unwrap();
    let linked = dir.path().join("linked");
    std::os::unix::fs::symlink(&artifacts, &linked).unwrap();
    let blocked = file.join("artifacts");
    for (root, refused, text) in [
        (
            &linked,
            Unrooted::Foreign(linked.clone()),
            format!(
                "{cause} '{}' is not a directory of its own",
                linked.display()
            ),
        ),
        (
            &blocked,
            Unrooted::Unmade(blocked.clone(), std::io::ErrorKind::NotADirectory),
            format!(
                "{cause} '{}' cannot be made or observed: not a directory",
                blocked.display()
            ),
        ),
    ] {
        assert_eq!(make_root(root), Err(refused.clone()));
        assert_eq!(refused.to_string(), text);
    }
}

/// Under `namespace` a site with hands is composed only beside the root
/// the engine made before launch: one it cannot make refuses the spawn
/// with CR2's cause, and nothing is composed around a missing protection.
#[test]
fn a_box_whose_artifact_root_cannot_be_made_refuses_its_spawn() {
    let command = ["/usr/local/bin/brokkr", "driver", "exec", "--", "true"].map(String::from);
    let command = command.to_vec();
    let (_dir, mut engine) = super::super::tests::engine(single_body(command.clone()));
    engine.boundary = Boundary::Namespace;
    let (spec, artifacts) = (HandsSpec::default(), artifact_root(&engine.run_local()));
    let made = engine.compose(
        "attempt",
        false,
        command.clone(),
        Some(&spec),
        None,
        "r.json",
    );
    assert_eq!((made.refusal, artifacts.is_dir()), (None, true));
    std::fs::remove_dir(&artifacts).unwrap();
    std::fs::write(&artifacts, "").unwrap();
    let refused = engine.compose("attempt", false, command, Some(&spec), None, "r.json");
    let unmade = Unrooted::Unmade(artifacts, std::io::ErrorKind::AlreadyExists);
    assert_eq!(refused.refusal, Some(unmade.to_string()));
}

/// H1: an exec seat whose artifact root is not UTF-8 refuses its spawn
/// before any argv is built, with CR2's cause. Its lossy spelling, U+FFFD
/// in place of the byte, names a decoy directory beside it, which a lossy
/// argv had the box protect while the real root stayed writable. A bundle
/// root that is not UTF-8 refuses by its own path. APFS refuses a name
/// that is not UTF-8, so Linux alone builds the pair.
#[cfg(target_os = "linux")]
#[test]
fn an_exec_seat_whose_artifact_root_is_not_utf8_refuses_before_any_argv() {
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let work = base.join("work");
    let root = base.join(std::ffi::OsStr::from_bytes(b"artifacts\xff"));
    let decoy = base.join("artifacts\u{fffd}");
    for made in [&work, &root, &decoy] {
        std::fs::create_dir(made).unwrap();
    }
    assert_eq!(root.to_string_lossy(), decoy.to_str().unwrap());
    let seat = ["/usr/local/bin/brokkr", "driver", "exec", "--", "true"].map(String::from);
    let (seat, spec) = (seat.to_vec(), HandsSpec::default());
    let refused = hands_command(seat.clone(), Some(&spec), &work, &root, &[]);
    assert_eq!(refused, Err(Unnamed::Artifacts(root.clone())));
    let lossy = "is not UTF-8, so no argument names it exactly and a lossy path would name \
                 another (rebuild unit 13-fix-c, R2; rebuild unit 14a2; U8a, H1)";
    let text = format!(
        "dispatch refused: capability evidence root cannot be protected from every managed \
         writer: the artifact root '{}' {lossy}",
        root.display()
    );
    assert_eq!(Unnamed::Artifacts(root.clone()).to_string(), text);
    let spawn = compose_site(
        BuiltBoundary::Namespace,
        SeatClass::Work,
        seat.clone(),
        Some(&spec),
        None,
        &work,
        &root,
        &[],
        "",
        None,
    );
    assert_eq!((spawn.argv, spawn.refusal), (seat.clone(), Some(text)));
    let bundle = base.join(std::ffi::OsStr::from_bytes(b"bundle\xff"));
    let roots = std::slice::from_ref(&bundle);
    let refused = hands_command(seat, Some(&spec), &work, &decoy, roots);
    assert_eq!(refused, Err(Unnamed::Bundle(bundle)));
    for (unnamed, text) in [
        (
            Unnamed::Executable("/b".into()),
            format!("dispatch refused: the engine's executable '/b' {lossy}"),
        ),
        (
            Unnamed::Bundle("/r".into()),
            format!("dispatch refused: the bundle root '/r' {lossy}"),
        ),
        (
            Unnamed::Unexpanded,
            "dispatch refused: the box's hands cannot be expanded into the seat's command"
                .to_string(),
        ),
    ] {
        assert_eq!(unnamed.to_string(), text);
    }
}

/// A zero-grant exec seat's composed box carries the engine's artifact
/// root and no protection key, and a run after earlier ones keeps their
/// evidence: the seat's writes and renames leave a historical artifact's
/// bytes, identity and directory where they were, while its worktree
/// write lands.
#[cfg(target_os = "linux")]
#[test]
fn a_zero_grant_exec_box_keeps_historical_evidence_without_an_authored_key() {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().canonicalize().unwrap().join("work");
    let artifacts = artifact_root(&crate::places::run_local(None, &work));
    make_root(&artifacts).unwrap();
    let digests = artifacts.join("sha256");
    std::fs::create_dir_all(&digests).unwrap();
    let artifact = digests.join("0".repeat(64));
    std::fs::write(&artifact, "historical\n").unwrap();
    let tamper = format!(
        "echo forged > {a}; rm -f {a}; mv .forge/artifacts gone; mv .forge gone; echo w > made",
        a = artifact.display()
    );
    let seat = [
        "/usr/local/bin/brokkr",
        "driver",
        "exec",
        "--",
        "/bin/sh",
        "-c",
    ];
    let mut inner: Vec<String> = seat.map(String::from).to_vec();
    inner.push(tamper.clone());
    let spec = HandsSpec::default();
    let argv = hands_command(inner, Some(&spec), &work, &artifacts, &[]).unwrap();
    let (work_arg, root_arg) = (work.display().to_string(), artifacts.display().to_string());
    assert_eq!(
        argv[1..8],
        [
            "hands",
            "exec",
            "--workdir",
            &work_arg,
            "--artifacts",
            &root_arg,
            "--spec"
        ]
    );
    let authored: Value = serde_json::from_str(&argv[8]).unwrap();
    assert_eq!(
        authored,
        json!({"kind": "workspace", "network": false, "binds": []})
    );
    assert_eq!(argv[14..], ["/bin/sh", "-c", tamper.as_str()]);
    let probe = require_bwrap().ok().filter(|bwrap| {
        Command::new(bwrap)
            .args(["--ro-bind", "/", "/", "--", "true"])
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    });
    let Some(bwrap) =
        probe.filter(|_| std::env::var_os(brokkr_protocol::hands::HANDS_BOX_ENV).is_none())
    else {
        brokkr_protocol::hands::skip_boundary_proof(
            brokkr_protocol::hands::boundary_evidence_required(),
            "this environment cannot create a bubblewrap namespace",
        );
        return;
    };
    // A rename across the box's mounts falls back to a copy at best: the
    // directories and the artifact keep their identities where they were.
    let held = [&artifact, &artifacts, &work.join(".forge")];
    let identities = || -> Vec<(u64, u64, u64)> {
        let metadata = held.map(|path| std::fs::symlink_metadata(path).unwrap());
        metadata
            .map(|found| (found.dev(), found.ino(), found.nlink()))
            .to_vec()
    };
    let before = identities();
    let spec = HandsSpec::parse(&authored).unwrap();
    let (home, session) = (dir.path().join("home"), dir.path().join("session"));
    let none = GitFacts::default();
    let boxed = run_boxed_in(
        &bwrap,
        &spec,
        Path::new(&argv[4]),
        Path::new(&argv[6]),
        &home,
        &session,
        &none,
        None,
        &argv[14..],
    );
    assert_eq!(boxed, Ok(0));
    assert_eq!(std::fs::read_to_string(work.join("made")).unwrap(), "w\n");
    assert_eq!(std::fs::read_to_string(&artifact).unwrap(), "historical\n");
    assert_eq!(identities(), before);
    assert_eq!(before[0].2, 1);
}
