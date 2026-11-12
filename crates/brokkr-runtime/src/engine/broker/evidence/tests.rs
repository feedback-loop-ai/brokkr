//! The evidence module's own seams, proved directly: the archive reader,
//! the no-follow copy, the digest listing, the destination rule, the
//! no-follow discard walk, and the conversions that keep CR2's cause
//! (decision 0105 rulings 2, 3(a) and 5).

use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use std::path::Path;

use brokkr_core::canonical;

use super::*;
use crate::engine::boxed::Unrooted;
use crate::engine::broker::admission::{Conflict, Unadmitted, Unknown};
use crate::engine::broker::inventory::Unread;
use crate::engine::broker::root::Step;
use crate::engine::broker::sources::observed;

/// A fresh directory of this test's own, in TMPDIR canonical, as
/// admission's fixtures are: a HOME handed to the protected walk must not
/// sit under the link macOS spells `/var` as (`/private/var`).
fn fresh() -> tempfile::TempDir {
    let base = fs::canonicalize(std::env::temp_dir()).unwrap();
    tempfile::tempdir_in(base).unwrap()
}

/// A fresh canonical base with a coordination root and a retired root
/// beside it, for a destination's comparison.
fn roots() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let dir = fresh();
    let base = fs::canonicalize(dir.path()).unwrap();
    let coordination = base.join("coordination");
    fs::create_dir(&coordination).unwrap();
    let retired = base.join("retired");
    fs::create_dir(&retired).unwrap();
    (dir, base, coordination, retired)
}

/// Five seconds from now.
fn soon() -> std::time::Instant {
    std::time::Instant::now() + std::time::Duration::from_secs(5)
}

/// A coordination root of this test's own, owner-only, under a fresh HOME.
fn coordination_root() -> (tempfile::TempDir, PathBuf, PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let dir = fresh();
    let home = dir.path().join("home");
    let root = home.join(".local/state/brokkr/capabilities");
    fs::create_dir_all(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    (dir, home, root)
}

/// The digest-named file `bytes` written under `root`'s `sha256/`.
fn published(root: &Path, bytes: &[u8]) -> (String, u64) {
    let at = root.join(DIGESTS);
    fs::create_dir_all(&at).unwrap();
    let name = canonical::sha256_bytes(bytes);
    fs::write(at.join(&name), bytes).unwrap();
    (name, bytes.len() as u64)
}

/// A regular file in `dir` holding one byte, and its path.
fn a_file(dir: &Path) -> PathBuf {
    let file = dir.join("file");
    fs::write(&file, b"x").unwrap();
    file
}

/// `text` written as an owner-only record, as a seal writes one.
fn record(at: &Path, text: &str) {
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = fs::OpenOptions::new();
    let opened = options.write(true).create_new(true).mode(0o600);
    std::io::Write::write_all(&mut opened.open(at).unwrap(), text.as_bytes()).unwrap();
}

/// The historical record of `artifacts` in the worktree directory `at`, as
/// admission seals one.
fn historical(at: &Path, artifacts: &Path) {
    let key = super::historical::key(artifacts);
    record(
        &at.join(format!("historical-{key}.json")),
        &serde_json::json!({"artifacts": artifacts}).to_string(),
    );
}

/// Whether `refused` is the io error a path through a file earns: not
/// absent, and not a directory.
fn not_a_directory<T>(refused: Result<T, EvidenceError>) -> bool {
    matches!(
        refused,
        Err(EvidenceError::Io(error)) if error.kind() == std::io::ErrorKind::NotADirectory
    )
}

/// Whether `refused` is the io error a permission the runner lacks earns.
fn denied<T>(refused: &Result<T, EvidenceError>) -> bool {
    matches!(
        refused,
        Err(EvidenceError::Io(error)) if error.kind() == std::io::ErrorKind::PermissionDenied
    )
}

/// `path` made readable but not writable, so a name below it cannot be
/// made, and the mode put back once the test is done with it.
fn read_only(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o500)).unwrap();
}

fn writable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

/// An empty archive destination in `base`, opened, and an empty evidence
/// root beside it.
fn opened_archive(
    base: &Path,
    coordination: &Path,
    retired: &Path,
) -> (PathBuf, destination::Destination, PathBuf) {
    let to = base.join("archive");
    fs::create_dir(&to).unwrap();
    let opened = destination::Destination::open(&to, coordination, retired).unwrap();
    let evidence = base.join("evidence");
    fs::create_dir(&evidence).unwrap();
    (to, opened, evidence)
}

/// The one archive inventory a test reads back.
fn archive_instance() -> EvidenceArchive {
    EvidenceArchive {
        format: EVIDENCE_ARCHIVE_FORMAT.to_string(),
        root: "/a/root".into(),
        engine: "0.13.1".into(),
        archived_at: "2026-11-08T00:00:00Z".into(),
        artifacts: vec![ArchiveArtifact {
            sha256: "a".repeat(64),
            bytes: 3,
        }],
        total_bytes: 3,
    }
}

#[test]
fn an_archive_reads_only_this_builds_format() {
    let archive = archive_instance();
    let bytes = serde_json::to_vec(&archive).unwrap();
    assert_eq!(read_archive(&bytes).unwrap(), archive);
    let mut other = archive.clone();
    other.format = "brokkr.evidence-archive.v2".into();
    let bytes = serde_json::to_vec(&other).unwrap();
    assert!(
        matches!(read_archive(&bytes), Err(EvidenceError::Format(found)) if found == other.format),
        "another format is refused by name"
    );
    assert!(matches!(
        read_archive(b"not json"),
        Err(EvidenceError::Inventory(_))
    ));
}

#[test]
fn copied_hashes_what_it_writes_and_refuses_a_link_or_a_wrong_name() {
    let dir = fresh();
    let source = dir.path().join("source");
    fs::write(&source, b"evidence").unwrap();
    let name = canonical::sha256_bytes(b"evidence");
    let target = opened(dir.path());
    copied(&source, &target, &name).unwrap();
    assert_eq!(fs::read(dir.path().join(&name)).unwrap(), b"evidence");

    let link = dir.path().join("link");
    symlink(&source, &link).unwrap();
    let refused_dir = fresh();
    let refused = opened(refused_dir.path());
    assert!(matches!(
        copied(&link, &refused, &name),
        Err(EvidenceError::Corrupt(at)) if at == link
    ));
    let wrong = canonical::sha256_bytes(b"other");
    let second_dir = fresh();
    let second = opened(second_dir.path());
    assert!(matches!(
        copied(&source, &second, &wrong),
        Err(EvidenceError::Corrupt(at)) if at == source
    ));

    let fifo = dir.path().join("fifo");
    let made = std::process::Command::new("mkfifo").arg(&fifo).status();
    assert!(made.is_ok_and(|made| made.success()));
    let third_dir = fresh();
    let third = opened(third_dir.path());
    assert!(matches!(
        copied(&fifo, &third, &name),
        Err(EvidenceError::Corrupt(at)) if at == fifo
    ));
}

/// A fresh directory of this test's own, opened for a copy's target.
fn opened(path: &Path) -> std::os::fd::OwnedFd {
    std::fs::File::open(path).unwrap().into()
}

/// The production copy path for one name, as `archived` drives it: a new
/// file in `dir`, then the digest check `written_into` makes (decision
/// 0105; 1020c).
fn copied(source: &Path, dir: &std::os::fd::OwnedFd, name: &str) -> Result<(), EvidenceError> {
    let mut out = destination::create(dir, name)?;
    destination::written_into(source, &mut out, name)
}

#[test]
fn digests_names_only_regular_files_named_by_a_digest() {
    let dir = fresh();
    let root = dir.path().join("root");
    fs::create_dir_all(&root).unwrap();
    let (one, one_bytes) = published(&root, b"one");
    let (two, two_bytes) = published(&root, b"two");
    let mut expected = vec![(one.clone(), one_bytes), (two.clone(), two_bytes)];
    expected.sort();
    assert_eq!(digests(&root).unwrap(), expected);
    assert_eq!(counts(&root).unwrap(), (true, 2, one_bytes + two_bytes));

    let notes = root.join(DIGESTS).join("notes");
    fs::write(&notes, b"notes").unwrap();
    assert!(matches!(
        digests(&root),
        Err(EvidenceError::Unfinished(at)) if at == notes
    ));
    fs::remove_file(&notes).unwrap();
    let linked = root.join(DIGESTS).join("b".repeat(64));
    symlink(root.join(DIGESTS).join(&one), &linked).unwrap();
    assert!(matches!(
        digests(&root),
        Err(EvidenceError::Unfinished(at)) if at == linked
    ));
}

#[test]
fn counts_reads_an_absent_root_and_one_that_is_no_directory() {
    let dir = fresh();
    assert_eq!(counts(&dir.path().join("absent")).unwrap(), (false, 0, 0));
    let file = dir.path().join("file");
    fs::write(&file, b"x").unwrap();
    assert_eq!(counts(&file).unwrap(), (true, 0, 0));
}

/// S1: a destination is resolved by the filesystem, made only where it
/// resolved, taken only where it is empty, and compared with both roots
/// before anything is written.
#[test]
fn a_destination_is_resolved_made_and_taken_only_where_it_is_empty() {
    let (_dir, base, coordination, retired) = roots();
    let missing = base.join("missing/nested");
    let opened = destination::Destination::open(&missing, &coordination, &retired).unwrap();
    assert_eq!(opened.at, missing);
    assert!(missing.is_dir());
    let empty = base.join("empty");
    fs::create_dir(&empty).unwrap();
    let opened = destination::Destination::open(&empty, &coordination, &retired).unwrap();
    assert_eq!(opened.at, empty);
    let full = base.join("full");
    fs::create_dir(&full).unwrap();
    fs::write(full.join("file"), b"x").unwrap();
    assert!(matches!(
        destination::Destination::open(&full, &coordination, &retired),
        Err(EvidenceError::DestinationNotEmpty(at)) if at == full
    ));
    let file = base.join("file");
    fs::write(&file, b"x").unwrap();
    assert!(matches!(
        destination::Destination::open(&file, &coordination, &retired),
        Err(EvidenceError::DestinationNotDirectory(at)) if at == file
    ));
    let unresolved = base.join("absent/../archive");
    assert!(matches!(
        destination::Destination::open(&unresolved, &coordination, &retired),
        Err(EvidenceError::Unresolved(at)) if at == unresolved
    ));
    let inside = coordination.join("archive");
    assert!(matches!(
        destination::Destination::open(&inside, &coordination, &retired),
        Err(EvidenceError::ArchiveInsideCoordination(at)) if at == inside
    ));
    let inside = retired.join("archive");
    assert!(matches!(
        destination::Destination::open(&inside, &coordination, &retired),
        Err(EvidenceError::ArchiveInsideRoot(at)) if at == inside
    ));
}

/// S1/1020b: a `.` below the deepest standing ancestor names the directory
/// it follows; the components walk drops it, so the recorded path is the
/// resolved one, and the directory is made.
#[test]
fn a_dot_below_the_standing_ancestor_is_dropped() {
    let (_dir, base, coordination, retired) = roots();
    let spelled = base.join("missing/./archive");
    let opened = destination::Destination::open(&spelled, &coordination, &retired).unwrap();
    let resolved = base.join("missing/archive");
    assert_eq!(opened.at, resolved, "`.` names the same directory");
    assert!(resolved.is_dir(), "the resolved directory stands");
}

#[test]
fn discarded_removes_a_no_follow_walk_and_an_absent_root() {
    let dir = fresh();
    let outside = dir.path().join("outside");
    fs::write(&outside, b"kept").unwrap();
    let root = dir.path().join("root");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("file"), b"one").unwrap();
    fs::write(root.join("nested/file"), b"two").unwrap();
    symlink(&outside, root.join("link")).unwrap();
    discard_with(&root, &device_of).unwrap();
    assert!(!root.exists(), "the root is gone");
    assert_eq!(
        fs::read(&outside).unwrap(),
        b"kept",
        "the link was not followed"
    );
    discard_with(&dir.path().join("absent"), &device_of).unwrap();
}

#[test]
fn discarded_refuses_a_link_a_file_and_another_filesystem() {
    let dir = fresh();
    let outside = dir.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let link = dir.path().join("link");
    symlink(&outside, &link).unwrap();
    assert!(matches!(
        discard_with(&link, &device_of),
        Err(EvidenceError::Link(at)) if at == link
    ));
    let file = dir.path().join("file");
    fs::write(&file, b"x").unwrap();
    assert!(matches!(
        discard_with(&file, &device_of),
        Err(EvidenceError::NotDirectory(at)) if at == file
    ));
    let root = dir.path().join("root");
    fs::create_dir(&root).unwrap();
    let other = |path: &Path| {
        let root = root.clone();
        Some(if path == root { 2 } else { 1 })
    };
    assert!(matches!(
        discard_with(&root, &other),
        Err(EvidenceError::OtherFilesystem(at)) if at == root
    ));
    assert!(root.exists(), "nothing was removed");
}

#[test]
fn discarded_refuses_a_child_on_another_filesystem_or_one_it_cannot_read() {
    let dir = fresh();
    let root = dir.path().join("root");
    let child = root.join("child");
    fs::create_dir_all(&child).unwrap();
    fs::write(child.join("file"), b"x").unwrap();
    let filesystems = |path: &Path| {
        let child = child.clone();
        Some(if path == child { 2 } else { 1 })
    };
    assert!(matches!(
        discard_with(&root, &filesystems),
        Err(EvidenceError::OtherFilesystem(at)) if at == child
    ));
    let unknown = |path: &Path| {
        let child = child.clone();
        if path == child {
            None
        } else {
            Some(1)
        }
    };
    assert!(matches!(
        discard_with(&root, &unknown),
        Err(EvidenceError::Device(at)) if at == child
    ));
    assert!(root.exists(), "nothing was removed");
}

#[test]
fn every_internal_refusal_keeps_its_cause_or_its_words() {
    assert!(matches!(
        EvidenceError::from(Unknown::Root),
        EvidenceError::Coordination
    ));
    assert!(matches!(
        EvidenceError::from(Unknown::Lock),
        EvidenceError::Lock
    ));
    assert!(matches!(
        EvidenceError::from(Unknown::Expired),
        EvidenceError::Expired
    ));
    assert!(matches!(
        EvidenceError::from(Unknown::Bounds),
        EvidenceError::Bounds
    ));
    let at = std::path::PathBuf::from("/participant");
    assert!(matches!(
        EvidenceError::from(Unknown::Participant(at.clone())),
        EvidenceError::Participant(found) if found == at
    ));
    match EvidenceError::from(Unknown::Unsealed) {
        EvidenceError::Refused(source) => assert!(
            matches!(source.downcast_ref::<Unknown>(), Some(Unknown::Unsealed)),
            "a refusal keeps its typed cause: {source:?}"
        ),
        other => panic!("an unsealed refusal must keep its cause: {other:?}"),
    }
    assert_eq!(
        EvidenceError::from(Unknown::Unsynced).to_string(),
        Unknown::Unsynced.to_string()
    );
    assert_eq!(
        EvidenceError::from(Unknown::Lost).to_string(),
        Unknown::Lost.to_string()
    );
    assert!(matches!(
        EvidenceError::from(Unread::Retirement(at.clone())),
        EvidenceError::Retirement(found) if found == at
    ));
    assert!(matches!(
        EvidenceError::from(Unread::Superseded(at.clone())),
        EvidenceError::Retirement(found) if found == at
    ));
    assert!(matches!(
        EvidenceError::from(Unread::Unknown(Unknown::Bounds)),
        EvidenceError::Bounds
    ));
    assert!(matches!(
        unadmitted(Unadmitted::Evidence(Conflict::Corrupt(at.clone()))),
        EvidenceError::Corrupt(found) if found == at
    ));
    let reaches = Conflict::Reaches {
        site: "work".into(),
        run: "run-a".into(),
        root: at.clone(),
    };
    assert_eq!(
        unadmitted(Unadmitted::Evidence(reaches.clone())).to_string(),
        reaches.to_string(),
        "a typed cause keeps its words"
    );
    assert_eq!(
        unadmitted(Unadmitted::Identity(Unknown::Expired)).to_string(),
        Unknown::Expired.to_string()
    );
    let unrooted = Unrooted::Foreign(at.clone());
    assert_eq!(
        unadmitted(Unadmitted::Unrooted(unrooted.clone())).to_string(),
        unrooted.to_string(),
        "an unrooted refusal keeps its own words"
    );
}

/// 1020b: each new refusal states its case in the operator's words.
#[test]
fn the_new_refusals_keep_their_words() {
    let at = PathBuf::from("/evidence/root");
    assert_eq!(
        EvidenceError::Absent(at.clone()).to_string(),
        concat!(
            "refused: the evidence root '/evidence/root' does not stand, so it cannot be archived; ",
            "--discarded is the record for an absent root"
        )
    );
    assert_eq!(
        EvidenceError::Reason.to_string(),
        concat!(
            "refused: a retirement needs the operator's reason, and the one given is empty; ",
            "nothing was written"
        )
    );
    assert_eq!(
        EvidenceError::Linked(at.clone()).to_string(),
        "the coordination root's path '/evidence/root' is a link, which is never followed"
    );
    assert_eq!(
        EvidenceError::NotPlain(at.clone()).to_string(),
        concat!(
            "the capability coordination root is not protected: '/evidence/root' is not a plain ",
            "name, which the walk never enters"
        )
    );
    assert_eq!(
        EvidenceError::Digests(at.clone()).to_string(),
        concat!(
            "the digest directory '/evidence/root' is a link or is not a directory, which neither ",
            "a readout nor an archive follows"
        )
    );
    assert_eq!(
        EvidenceError::Link(at.clone()).to_string(),
        concat!(
            "the evidence root '/evidence/root' is a link, which a retirement never follows; ",
            "remove the link, then --discarded records the root as absent"
        )
    );
    assert_eq!(
        EvidenceError::Unprotected(at).to_string(),
        "the coordination root is not protected: '/evidence/root' is not this user's alone"
    );
}

/// 12(e)/1020b: the public verb checks the reason before it makes the
/// coordination root, so its "nothing was written" is true of a fresh HOME
/// too, which a test of `retire_in` on an opened root cannot see.
#[test]
fn the_public_retire_refuses_an_empty_reason_before_making_the_root() {
    let dir = fresh();
    let home = dir.path().join("home");
    fs::create_dir(&home).unwrap();
    let refused = retire(
        &home,
        RetireOptions {
            artifacts: &home.join("missing"),
            action: RetireAction::Discarded,
            operator: "operator",
            reason: "   ",
            now: "2026-11-08T00:00:00Z",
        },
        soon(),
    );
    assert!(matches!(refused, Err(EvidenceError::Reason)), "{refused:?}");
    assert!(!home.join(".local").exists(), "the refusal made nothing");
}

/// I1: a standing attempt with an unbounded writer refuses a retirement by
/// name, through `protected`'s `Reaching::Unbounded` arm.
#[test]
fn a_standing_unbounded_writer_refuses_a_retirement() {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::engine::broker::admission::{
        Protects, Reach, Reservation, Reserved, Witnessed, Writer,
    };

    let standing = Standing::from(Reservation {
        repo: "repo".into(),
        run: "run-s".into(),
        workdir: "/work".into(),
        artifacts: "/other/artifacts".into(),
        journal: "/other/forge.db".into(),
        attempts: BTreeMap::from([(
            "s1".into(),
            Reserved {
                protects: Protects::Nothing,
                writers: vec![Writer {
                    site: "work".into(),
                    reach: Reach::Unbounded,
                }],
                cleanup: Witnessed::Settling,
                serves: Vec::new(),
                drivers: Vec::new(),
            },
        )]),
        settled: BTreeSet::new(),
    });
    match protected(&[standing], Path::new("/evidence/root")) {
        Err(EvidenceError::Protected { root, run }) => {
            assert_eq!(root, Path::new("/evidence/root"));
            assert_eq!(run, "run-s");
        }
        other => panic!("an unbounded standing writer must refuse: {other:?}"),
    }
}

/// M2: a directory sync succeeds for this tree, and procfs's `EINVAL` is
/// the allowance macOS needs, not a refusal.
#[test]
fn synced_a_directory_and_the_einval_allowance() {
    let dir = fresh();
    sync_dir(&opened(dir.path())).unwrap();
    #[cfg(target_os = "linux")]
    assert!(
        sync_dir(&opened(Path::new("/proc"))).is_ok(),
        "procfs answers a directory fsync with EINVAL"
    );
}

/// H1: the retirement covering a response is the earliest sealed at or
/// after the response's recorded time; after every retirement it is live,
/// and an unreadable time falls back to the current retirement.
#[test]
fn a_retirement_covers_the_evidence_retained_before_it() {
    let retirement = |at: &str, reason: &str| Retirement {
        artifacts: "/evidence/root".into(),
        cause: Cause::Discarded,
        archive: None,
        inventory_sha256: None,
        operator: "operator".into(),
        reason: reason.into(),
        retired_at: at.into(),
        engine: "0.13.1".into(),
    };
    let first = retirement("2026-11-08T00:00:00Z", "first");
    let second = retirement("2026-11-09T00:00:00Z", "second");
    let history = RetirementHistory {
        current: Some(second.clone()),
        superseded: vec![Superseded::of(&first, "run-b", "2026-11-09T00:00:00Z")],
    };
    assert_eq!(
        history.covering("2026-11-07T00:00:00Z"),
        Some(first.clone())
    );
    assert_eq!(
        history.covering("2026-11-08T12:00:00Z"),
        Some(second.clone())
    );
    assert_eq!(history.covering("2026-11-10T00:00:00Z"), None);
    assert_eq!(history.covering("not a time"), Some(second));
}

/// I2: a coordination root that stands but is not this user's alone is
/// named, not read as no history.
#[test]
fn a_coordination_root_not_this_users_alone_is_named() {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, home, root) = coordination_root();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    match list(&home, deadline) {
        Err(EvidenceError::Unprotected(at)) => assert_eq!(at, root),
        other => panic!("a root not this user's alone must be named: {other:?}"),
    }
}

/// I2/1020b: the readouts take admission's ancestor walk. An ancestor that
/// is not this user's alone is named by its own path, the same problem
/// `retire` refuses with, and nothing is made.
#[test]
fn an_ancestor_not_this_users_alone_is_named_and_nothing_is_made() {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, home, root) = coordination_root();
    let state = home.join(".local/state");
    fs::set_permissions(&state, fs::Permissions::from_mode(0o777)).unwrap();
    let listed = list(&home, soon());
    let retired = retire(
        &home,
        RetireOptions {
            artifacts: &root,
            action: RetireAction::Discarded,
            operator: "operator",
            reason: "given up",
            now: "2026-11-08T00:00:00Z",
        },
        soon(),
    );
    fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
    match listed {
        Err(EvidenceError::Unprotected(at)) => assert_eq!(at, state),
        other => panic!("an ancestor not this user's alone must be named: {other:?}"),
    }
    assert!(
        matches!(retired, Err(EvidenceError::Coordination)),
        "retire refuses the same walk: {retired:?}"
    );
    assert!(
        !root.join("admission.lock").exists(),
        "the readout made no lock"
    );
}

/// I2/1020b: a link on the way to the coordination root is named, not read
/// as no history, and the readout makes nothing where it points.
#[test]
fn a_linked_coordination_root_is_named_and_nothing_is_made() {
    let (dir, home, _root) = coordination_root();
    let at = home.join(".local/state/brokkr");
    let elsewhere = dir.path().join("elsewhere");
    fs::create_dir(&elsewhere).unwrap();
    fs::remove_dir_all(&at).unwrap();
    symlink(&elsewhere, &at).unwrap();
    let listed = list(&home, soon());
    let retired = retire(
        &home,
        RetireOptions {
            artifacts: &home.join("missing"),
            action: RetireAction::Discarded,
            operator: "operator",
            reason: "given up",
            now: "2026-11-08T00:00:00Z",
        },
        soon(),
    );
    match listed {
        Err(EvidenceError::Linked(refused)) => assert_eq!(refused, at),
        other => panic!("a linked coordination root must be named: {other:?}"),
    }
    assert!(
        matches!(retired, Err(EvidenceError::Coordination)),
        "retire refuses the same walk: {retired:?}"
    );
    assert!(
        !elsewhere.join("capabilities").exists(),
        "the readout made nothing"
    );
}

/// LOW 2 (1020c): the readout takes admission's plain-name check. A HOME
/// with a `..` is refused by `list` naming the path walked so far and the
/// problem `retire` refuses with, and the readout makes nothing.
#[test]
fn a_name_that_is_not_plain_is_named_by_the_readout() {
    let (dir, _home, root) = coordination_root();
    let base = dir.path();
    fs::create_dir(base.join("a")).unwrap();
    let spelled = base.join("a/../home");
    let listed = list(&spelled, soon());
    let retired = retire(
        &spelled,
        RetireOptions {
            artifacts: &root,
            action: RetireAction::Discarded,
            operator: "operator",
            reason: "given up",
            now: "2026-11-08T00:00:00Z",
        },
        soon(),
    );
    let at = base.join("a/..");
    match listed {
        Err(EvidenceError::NotPlain(refused)) => {
            assert_eq!(refused, at);
            assert!(
                EvidenceError::NotPlain(at.clone())
                    .to_string()
                    .starts_with(&EvidenceError::Coordination.to_string()),
                "the readout names the problem retire refuses with"
            );
        }
        other => panic!("a name that is not plain must be named: {other:?}"),
    }
    assert!(
        matches!(retired, Err(EvidenceError::Coordination)),
        "retire refuses the same walk: {retired:?}"
    );
    assert!(
        !root.join("admission.lock").exists(),
        "the readout made no lock"
    );
}

/// I2/1020b and 1020c: a name on the way that is a file, not a link, is
/// unreadable with its kind and names the path that failed rather than the
/// coordination root.
#[test]
fn a_file_on_the_way_to_the_coordination_root_is_unreadable() {
    let (_dir, home, _root) = coordination_root();
    let at = home.join(".local/state/brokkr");
    fs::remove_dir_all(&at).unwrap();
    fs::write(&at, b"not a directory").unwrap();
    match list(&home, soon()) {
        Err(EvidenceError::Unreadable(refused, kind)) => {
            assert_eq!(refused, at);
            assert_eq!(kind, std::io::ErrorKind::NotADirectory);
        }
        other => panic!("a file on the way must be unreadable: {other:?}"),
    }
}

/// S2: a coordination root that cannot be opened is a named problem, never
/// read as no history.
#[test]
fn an_unreadable_coordination_root_is_named_not_read_as_absent() {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, home, root) = coordination_root();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o000)).unwrap();
    let listed = list(&home, soon());
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    match listed {
        Err(EvidenceError::Unreadable(at, kind)) => {
            assert_eq!(at, root);
            assert_eq!(kind, std::io::ErrorKind::PermissionDenied);
        }
        other => panic!("an unreadable root must be named: {other:?}"),
    }
}

/// S2: a lock file that is a directory refuses as the write path refuses
/// it, never read as no lock.
#[test]
fn a_lock_that_is_a_directory_is_not_read_as_no_lock() {
    let (_dir, home, root) = coordination_root();
    fs::create_dir(root.join("admission.lock")).unwrap();
    assert!(matches!(list(&home, soon()), Err(EvidenceError::Lock)));
}

/// S2: a lock file that is a link refuses as the write path refuses it,
/// never read as no lock.
#[test]
fn a_lock_that_is_a_link_is_not_read_as_no_lock() {
    let (_dir, home, root) = coordination_root();
    let outside = home.parent().unwrap().join("outside-lock");
    fs::write(&outside, b"").unwrap();
    symlink(&outside, root.join("admission.lock")).unwrap();
    assert!(matches!(list(&home, soon()), Err(EvidenceError::Lock)));
}

/// M4: a root whose own digest cannot be read still describes, carrying its
/// problem, so one bad root hides no other.
#[test]
fn a_bad_root_carries_its_own_problem() {
    let dir = fresh();
    let root = dir.path().join("root");
    fs::create_dir_all(root.join(DIGESTS)).unwrap();
    let unfinished = root.join(DIGESTS).join("unfinished");
    fs::write(&unfinished, b"x").unwrap();
    let held = HistoricalRoot {
        repo: "repo".into(),
        artifacts: root,
        retirement: None,
        history: Vec::new(),
    };
    let row = described(&held);
    let problem = row.problem.expect("the bad root carries its problem");
    assert!(
        problem.contains(&unfinished.display().to_string()),
        "{problem}"
    );
}

/// M4/1020b: a `sha256` that is a link is a problem on that root's row, and
/// every other root still prints. A digest directory is read without
/// following a link, as the archive refuses it.
#[test]
fn a_linked_digest_directory_is_a_problem_on_its_own_row() {
    use std::os::unix::fs::DirBuilderExt;

    let (dir, home, root) = coordination_root();
    let base = fs::canonicalize(dir.path()).unwrap();
    let work = base.join("work");
    let repo = canonical::sha256_bytes(work.as_os_str().as_bytes());
    let at = root.join(&repo);
    let mut made = fs::DirBuilder::new();
    made.mode(0o700).create(&at).unwrap();
    let linked = base.join("linked");
    let elsewhere = base.join("elsewhere");
    fs::create_dir(&linked).unwrap();
    fs::create_dir(&elsewhere).unwrap();
    symlink(&elsewhere, linked.join(DIGESTS)).unwrap();
    let good = base.join("good");
    fs::create_dir(&good).unwrap();
    let (_, bytes) = published(&good, b"one");
    historical(&at, &linked);
    historical(&at, &good);
    record(&root.join("admission.lock"), "");

    let listed = list(&home, soon()).unwrap();
    assert_eq!(listed.len(), 2, "both rows print");
    let row = |artifacts: &Path| {
        listed
            .iter()
            .find(|row| row.artifacts == artifacts)
            .expect("the root is listed")
    };
    let bad = row(&linked);
    let problem = bad
        .problem
        .as_deref()
        .expect("the linked sha256 is a problem");
    assert!(
        problem.contains(&linked.join(DIGESTS).display().to_string()),
        "{problem}"
    );
    let ok = row(&good);
    assert_eq!(ok.problem, None);
    assert_eq!(ok.digests, 1);
    assert_eq!(ok.bytes, bytes);
}

/// H1: a superseded record's name carries its key and its sequence, and a
/// name outside that shape is not one.
#[test]
fn a_superseded_name_carries_its_key_and_sequence() {
    use crate::engine::broker::historical::superseded;

    let key = "a".repeat(64);
    let named = superseded::name(&key, 2);
    assert_eq!(named, format!("superseded-{key}-2.json"));
    assert_eq!(superseded::named(&named), Some((key.as_str(), 2)));
    assert_eq!(superseded::named("superseded-short-1.json"), None);
    assert_eq!(superseded::named(&format!("superseded-{key}-0.json")), None);
    assert_eq!(
        superseded::named(&format!("superseded-{key}-01.json")),
        None
    );
    assert_eq!(
        superseded::named(&format!("superseded-{key}-+1.json")),
        None
    );
    assert_eq!(superseded::named(&format!("superseded-{key}.json")), None);
    assert_eq!(superseded::named(&format!("historical-{key}.json")), None);
}

/// S1/R2: the retirement is read for the root admission recorded, which is
/// the root's real path, even where the repository is named through a link.
#[test]
fn a_retirement_is_read_through_a_symlinked_repo() {
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

    let (dir, home, root) = coordination_root();
    // Admission records a repository by its canonical path, so the planted
    // records are keyed by it: macOS's temporary directory sits under /var,
    // a link to /private/var.
    let real = fs::canonicalize(dir.path()).unwrap().join("real");
    let artifacts = real.join(".forge/artifacts");
    fs::create_dir_all(&artifacts).unwrap();
    let link = dir.path().join("link");
    symlink(&real, &link).unwrap();
    let repo = canonical::sha256_bytes(real.as_os_str().as_bytes());
    let key = canonical::sha256_bytes(artifacts.as_os_str().as_bytes());
    let at = root.join(repo);
    let mut made = fs::DirBuilder::new();
    made.mode(0o700).create(&at).unwrap();
    let retirement = Retirement {
        artifacts: artifacts.clone(),
        cause: Cause::Discarded,
        archive: None,
        inventory_sha256: None,
        operator: "operator".into(),
        reason: "given up".into(),
        retired_at: "2026-11-08T00:00:00Z".into(),
        engine: "0.13.1".into(),
    };
    let seal = |at: &Path, bytes: &[u8]| {
        let mut options = fs::OpenOptions::new();
        let opened = options.write(true).create_new(true).mode(0o600);
        std::io::Write::write_all(&mut opened.open(at).unwrap(), bytes).unwrap();
    };
    seal(
        &at.join(format!("historical-{key}.json")),
        serde_json::json!({"artifacts": artifacts})
            .to_string()
            .as_bytes(),
    );
    seal(
        &at.join(format!("retired-{key}.json")),
        &serde_json::to_vec(&retirement).unwrap(),
    );
    seal(&root.join("admission.lock"), b"");

    let read = retirements(&home, &link.join(".forge/artifacts"), soon()).unwrap();
    assert_eq!(read.current, Some(retirement));
}

/// S1: a destination is resolved through the links the filesystem holds,
/// not by a lexical reading, before it is compared with either root.
#[test]
fn a_destination_resolves_through_a_link_before_it_is_compared() {
    let dir = fresh();
    let base = fs::canonicalize(dir.path()).unwrap();
    let link = base.join("link");
    symlink(&base, &link).unwrap();
    let at = destination::resolved(&link.join("..").join("archive")).unwrap();
    assert_eq!(
        at,
        base.parent().unwrap().join("archive"),
        "the link is followed and its `..` is the directory above where it points"
    );
}

/// M2: an absent directory cannot be opened, and the io error names it
/// before any sync.
#[test]
fn a_directory_that_cannot_be_opened_refuses_the_sync() {
    let dir = fresh();
    let absent = dir.path().join("absent");
    let refused = destination::directory(&absent, (0, 0));
    assert!(
        matches!(&refused, Err(EvidenceError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound),
        "{refused:?}"
    );
}

/// M2/1020b: only a name that is not a directory or is a link refuses as
/// `DestinationNotDirectory`; a permission denied opening one is the io
/// error it is.
#[test]
fn an_open_error_that_is_not_missing_or_a_link_is_named_as_io() {
    use std::os::unix::fs::PermissionsExt;

    let dir = fresh();
    let shut = dir.path().join("shut");
    fs::create_dir(&shut).unwrap();
    fs::set_permissions(&shut, fs::Permissions::from_mode(0o600)).unwrap();
    let below = shut.join("below");
    let refused = destination::directory(&below, (0, 0));
    fs::set_permissions(&shut, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(denied(&refused), "{refused:?}");
}

/// S1/R3: the directory opened is the one that was resolved; one swapped
/// between the comparison and the open refuses by name.
#[test]
fn a_directory_that_is_not_the_one_resolved_refuses() {
    use std::os::unix::fs::MetadataExt;

    let dir = fresh();
    let one = dir.path().join("one");
    let two = dir.path().join("two");
    fs::create_dir(&one).unwrap();
    fs::create_dir(&two).unwrap();
    let found = fs::metadata(&one).unwrap();
    let identity = (found.dev(), found.ino());
    match destination::directory(&one, identity) {
        Ok(_) => {}
        other => panic!("the resolved directory opens: {other:?}"),
    }
    match destination::directory(&two, identity) {
        Err(EvidenceError::Replaced(at)) => assert_eq!(at, two),
        other => panic!("a swapped directory must refuse by name: {other:?}"),
    }
}

/// R9: a destination that cannot be recorded as text refuses before a byte
/// is written.
#[test]
fn a_destination_that_cannot_be_recorded_refuses_before_it_is_made() {
    use std::os::unix::ffi::OsStringExt;

    let (_dir, base, coordination, retired) = roots();
    let at = base.join(std::ffi::OsString::from_vec(b"archive\xff".to_vec()));
    match destination::Destination::open(&at, &coordination, &retired) {
        Err(EvidenceError::Unnamed(refused)) => assert_eq!(refused, at),
        other => panic!("a non-UTF-8 destination must refuse by name: {other:?}"),
    }
    assert!(!at.exists(), "nothing was made");
}

/// L1: a lock file that is not this user's own single regular file refuses
/// as the write path refuses it, and a missing one is no lock at all.
#[test]
fn the_read_only_lock_judges_what_it_finds() {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, _home, root) = coordination_root();
    let lock = root.join("admission.lock");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let opened = || {
        let flags = rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::DIRECTORY
            | rustix::fs::OFlags::CLOEXEC;
        rustix::fs::open(&root, flags, rustix::fs::Mode::empty()).unwrap()
    };
    assert!(matches!(locked_existing(&opened(), deadline), Ok(None)));
    fs::write(&lock, b"").unwrap();
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        locked_existing(&opened(), deadline),
        Err(Unknown::Lock)
    ));
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::hard_link(&lock, root.join("second")).unwrap();
    assert!(matches!(
        locked_existing(&opened(), deadline),
        Err(Unknown::Lock)
    ));
}

/// S1: a name below the deepest standing ancestor that stands as a link is
/// opened with no follow, so the refusal comes from `child` and names the
/// link's own path.
#[test]
fn a_destination_below_a_link_refuses_through_the_child() {
    let (_dir, base, coordination, retired) = roots();
    let dangling = base.join("dangling");
    symlink(base.join("nowhere"), &dangling).unwrap();
    let at = dangling.join("inner");
    match destination::Destination::open(&at, &coordination, &retired) {
        Err(EvidenceError::DestinationNotDirectory(refused)) => assert_eq!(refused, dangling),
        other => panic!("a link below the destination must refuse by name: {other:?}"),
    }
    assert!(dangling.is_symlink(), "the link was not followed");
}

/// S1: a name that cannot be made below a parent without write permission
/// refuses and leaves no directory this verb made.
#[test]
fn a_destination_that_cannot_be_made_refuses_and_keeps_nothing() {
    let (_dir, base, coordination, retired) = roots();
    let locked = base.join("locked");
    fs::create_dir(&locked).unwrap();
    read_only(&locked);
    let at = locked.join("made/nested");
    let refused = destination::Destination::open(&at, &coordination, &retired);
    writable(&locked);
    assert!(denied(&refused), "{refused:?}");
    assert!(!locked.join("made").exists(), "nothing was made");
}

/// M2/1020b: a component this verb made before `mkdirat` refused the next
/// is removed, so the refusal leaves the destination as it found it.
#[test]
fn a_component_made_before_a_mkdir_refusal_is_removed() {
    let (_dir, base, coordination, retired) = roots();
    let too_long = "n".repeat(300);
    let at = base.join("made").join(&too_long);
    let refused = destination::Destination::open(&at, &coordination, &retired);
    assert!(matches!(&refused, Err(EvidenceError::Io(_))), "{refused:?}");
    assert!(!base.join("made").exists(), "the component made is gone");
}

/// The role a parent plays: this binary re-executed with `env` set and one
/// exact ignored test, which must run and pass. libtest's `--exact` with no
/// match exits 0 with nothing run, so the "1 passed" line is the proof it
/// ran (1020b).
fn ran(env: &str, exact: &str) {
    let seen = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", exact, "--ignored"])
        .env(env, "1")
        .output()
        .unwrap();
    assert!(
        seen.status.success(),
        "{}",
        String::from_utf8_lossy(&seen.stderr)
    );
    let out = String::from_utf8_lossy(&seen.stdout);
    assert!(out.contains("1 passed"), "the role ran: {out}");
}

/// `run` with the process umask at 0o777, restored before returning, so a
/// directory made under it lacks the bits `child` needs.
fn under_hostile_umask<T>(run: impl FnOnce() -> T) -> T {
    let previous = rustix::process::umask(rustix::fs::Mode::from_bits_truncate(0o777));
    let out = run();
    rustix::process::umask(previous);
    out
}

/// M2/1020b: a component this verb made before `child` refused the next is
/// removed too. Played in a child process under a umask that leaves a fresh
/// directory unreadable, so no parallel test reads the process umask.
#[test]
fn a_component_made_before_a_child_refusal_is_removed() {
    ran(
        CHILD_ROLE,
        "engine::broker::evidence::tests::a_component_made_before_a_child_refusal_under_a_hostile_umask",
    );
}

/// The role the child-arm control plays: this binary re-executed, so the
/// process-wide umask is changed where no parallel test reads it.
const CHILD_ROLE: &str = "BROKKR_EVIDENCE_CHILD_ROLE";

#[test]
#[ignore = "played only when a test re-executes this binary"]
fn a_component_made_before_a_child_refusal_under_a_hostile_umask() {
    if std::env::var_os(CHILD_ROLE).is_none() {
        return;
    }
    let (_dir, base, coordination, retired) = roots();
    let at = base.join("made/nested");
    let refused =
        under_hostile_umask(|| destination::Destination::open(&at, &coordination, &retired));
    assert!(denied(&refused), "{refused:?}");
    assert!(!base.join("made").exists(), "the component made is gone");
}

/// M2: the digest directory this verb makes is removed when it cannot be
/// opened, so a refusal leaves the destination as empty as it found it.
/// Played in a child process, whose umask no parallel test reads.
#[test]
fn a_digest_directory_that_cannot_be_opened_is_removed() {
    ran(
        UMASK_ROLE,
        "engine::broker::evidence::tests::a_digest_directory_is_removed_under_a_hostile_umask",
    );
}

/// The role the umask control plays: it is this binary re-executed, so the
/// process-wide umask is changed where no parallel test reads it. It keeps
/// the parent's own order: the directories exist before the umask is set,
/// and only `sha256/` is made under it.
const UMASK_ROLE: &str = "BROKKR_EVIDENCE_UMASK_ROLE";

#[test]
#[ignore = "played only when a test re-executes this binary"]
fn a_digest_directory_is_removed_under_a_hostile_umask() {
    if std::env::var_os(UMASK_ROLE).is_none() {
        return;
    }
    let (_dir, base, coordination, retired) = roots();
    let (to, opened, evidence) = opened_archive(&base, &coordination, &retired);
    let refused = under_hostile_umask(|| {
        opened.archive(
            &evidence,
            &mut Budget::new(soon()),
            &|| {},
            "2026-11-08T00:00:00Z",
        )
    });
    assert!(denied(&refused), "{refused:?}");
    assert!(!to.join(DIGESTS).exists(), "the digest directory is gone");
}

/// M2: `sha256/` cannot be made below a destination that is readable but
/// not writable, and the refusal is the io error, not a panic.
#[test]
fn a_digest_directory_that_cannot_be_made_refuses() {
    let (_dir, base, coordination, retired) = roots();
    let (to, opened, evidence) = opened_archive(&base, &coordination, &retired);
    read_only(&to);
    let refused = opened.archive(
        &evidence,
        &mut Budget::new(soon()),
        &|| {},
        "2026-11-08T00:00:00Z",
    );
    writable(&to);
    assert!(denied(&refused), "{refused:?}");
    assert!(!to.join(DIGESTS).exists());
}

/// M2: the inventory is created new beneath the destination, so a name that
/// already stands there is the io error that creation reports.
#[test]
fn an_inventory_that_already_stands_refuses() {
    let (_dir, base, coordination, retired) = roots();
    let (to, opened, evidence) = opened_archive(&base, &coordination, &retired);
    fs::write(to.join("inventory.json"), b"raced").unwrap();
    let refused = opened.archive(
        &evidence,
        &mut Budget::new(soon()),
        &|| {},
        "2026-11-08T00:00:00Z",
    );
    assert!(
        matches!(&refused, Err(EvidenceError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists),
        "{refused:?}"
    );
    assert!(!to.join(DIGESTS).exists(), "the digest directory is gone");
}

/// M2/1020b and 1020c: an inventory this verb created is the one its
/// refusal removes, driven through archive's own refusal path once the
/// inventory is written, never a test-only item in the production file.
#[test]
fn a_refusal_after_an_inventory_this_verb_wrote_removes_it() {
    let (_dir, base, coordination, retired) = roots();
    let (to, opened, evidence) = opened_archive(&base, &coordination, &retired);
    let after = || Err(EvidenceError::NotDirectory(to.clone()));
    let refused = opened.archive_with(
        &evidence,
        &mut Budget::new(soon()),
        &|| {},
        &after,
        "2026-11-08T00:00:00Z",
    );
    assert!(
        matches!(&refused, Err(EvidenceError::NotDirectory(at)) if *at == to),
        "{refused:?}"
    );
    assert!(
        !to.join("inventory.json").exists(),
        "the inventory this verb wrote is removed"
    );
    assert!(!to.join(DIGESTS).exists(), "the digest directory is gone");
}

/// Archive `evidence` through `between`, expecting the planted inventory's
/// `AlreadyExists`: the refusal both ordering tests below read.
fn an_inventory_another_writer_planted_refuses(
    opened: &destination::Destination,
    evidence: &Path,
    between: &dyn Fn(),
) {
    let refused = opened.archive(
        evidence,
        &mut Budget::new(soon()),
        between,
        "2026-11-08T00:00:00Z",
    );
    assert!(
        matches!(&refused, Err(EvidenceError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists),
        "{refused:?}"
    );
}

/// M2/1020b: a refusal removes only what this verb wrote. A name another
/// writer planted after the destination was verified empty — the inventory
/// and a digest of the root's own name — survives, and the refusal still
/// stands. Played through `archive`'s `between` seam, after `sha256/` is
/// made and before the copies.
#[test]
fn an_archive_refusal_removes_only_what_the_verb_wrote() {
    let (_dir, base, coordination, retired) = roots();
    let (to, opened, evidence) = opened_archive(&base, &coordination, &retired);
    let (name, _) = published(&evidence, b"one");
    let between = || {
        fs::write(to.join("inventory.json"), b"raced").unwrap();
        fs::write(to.join(DIGESTS).join(&name), b"raced").unwrap();
    };
    an_inventory_another_writer_planted_refuses(&opened, &evidence, &between);
    assert_eq!(
        fs::read(to.join("inventory.json")).unwrap(),
        b"raced",
        "the planted inventory survives"
    );
    assert!(
        to.join(DIGESTS).join(&name).exists(),
        "the planted digest survives"
    );
}

/// M2/1020b and 1020c: an inventory another writer planted is not this
/// verb's to remove, even where no digest copy refused first: `written`
/// learns of an inventory only after `create` made it.
#[test]
fn an_archive_refusal_keeps_an_inventory_it_did_not_create() {
    let (_dir, base, coordination, retired) = roots();
    let (to, opened, evidence) = opened_archive(&base, &coordination, &retired);
    let between = || fs::write(to.join("inventory.json"), b"raced").unwrap();
    an_inventory_another_writer_planted_refuses(&opened, &evidence, &between);
    assert_eq!(
        fs::read(to.join("inventory.json")).unwrap(),
        b"raced",
        "the inventory another writer planted survives"
    );
}

/// `hook` for a root whose every seal fails: the archive is written, then
/// its retirement cannot be sealed.
fn unsealed(_: Step, _: &std::os::fd::OwnedFd, _: &str) -> std::io::Result<()> {
    Err(std::io::Error::other("the seal fails"))
}

/// M2/1020b: an archive whose retirement seal then fails stays in place,
/// and a retry into the same directory refuses `DestinationNotEmpty`; the
/// retry takes a fresh destination.
#[test]
fn an_archive_whose_seal_fails_stays_and_a_retry_needs_a_fresh_destination() {
    use std::os::unix::fs::DirBuilderExt;

    let (dir, _home, root_path) = coordination_root();
    let base = fs::canonicalize(dir.path()).unwrap();
    let work = base.join("work");
    let artifacts = work.join(".forge/artifacts");
    let (name, _) = published(&artifacts, b"one");
    let repo = canonical::sha256_bytes(work.as_os_str().as_bytes());
    let at = root_path.join(&repo);
    let mut made = fs::DirBuilder::new();
    made.mode(0o700).create(&at).unwrap();
    historical(&at, &artifacts);
    record(&root_path.join("admission.lock"), "");
    let root = Root::opened(opened(&root_path), root_path, (unsealed, observed)).unwrap();
    let to = base.join("archive");
    let refused = retire_in_with(
        &root,
        RetireOptions {
            artifacts: &artifacts,
            action: RetireAction::Archived { to: &to },
            operator: "operator",
            reason: "archived",
            now: "2026-11-08T00:00:00Z",
        },
        soon(),
        &|| {},
    );
    match refused {
        Err(EvidenceError::Refused(source)) => assert!(
            matches!(source.downcast_ref::<Unknown>(), Some(Unknown::Unsealed)),
            "the refusal keeps its typed cause: {source:?}"
        ),
        other => panic!("the retirement seal's refusal must be typed: {other:?}"),
    }
    assert!(
        to.join("inventory.json").exists(),
        "the archive stays in place"
    );
    assert!(to.join(DIGESTS).join(&name).exists(), "its copy stays");
    let again = retire_in(
        &root,
        RetireOptions {
            artifacts: &artifacts,
            action: RetireAction::Archived { to: &to },
            operator: "operator",
            reason: "archived",
            now: "2026-11-08T00:00:00Z",
        },
        soon(),
    );
    assert!(
        matches!(&again, Err(EvidenceError::DestinationNotEmpty(at)) if *at == to),
        "{again:?}"
    );
}

/// M4: a root whose metadata cannot be read, and a digest directory that is
/// not a directory, refuse with the io error rather than as absent.
#[test]
fn counts_and_digests_name_a_root_they_cannot_read() {
    use std::os::unix::fs::PermissionsExt;

    let dir = fresh();
    let below = a_file(dir.path()).join("below");
    assert!(not_a_directory(counts(&below)));
    let root = dir.path().join("root");
    fs::create_dir(&root).unwrap();
    let at = root.join(DIGESTS);
    fs::write(&at, b"not a directory").unwrap();
    assert!(matches!(digests(&root), Err(EvidenceError::Digests(refused)) if refused == at));
    fs::remove_file(&at).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o000)).unwrap();
    let refused = digests(&root);
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(denied(&refused), "{refused:?}");
}

/// M2: a discard refuses a stat that fails for a reason other than absence,
/// and a device read that answers nothing.
#[test]
fn discarded_names_a_failed_stat_and_an_unreadable_device() {
    let dir = fresh();
    let below = a_file(dir.path()).join("below");
    assert!(not_a_directory(discard_with(&below, &device_of)));
    let root = dir.path().join("root");
    fs::create_dir(&root).unwrap();
    assert!(matches!(
        discard_with(&root, &|_| None),
        Err(EvidenceError::Device(at)) if at == root
    ));
    assert!(root.exists(), "nothing was removed");
}

/// M2: the one directory sync answer the code allows is `EINVAL`; any other
/// error is a refusal, never read as a sync.
#[cfg(target_os = "linux")]
#[test]
fn a_directory_sync_refuses_an_error_that_is_not_einval() {
    let dir = fresh();
    let path = rustix::fs::open(
        dir.path(),
        rustix::fs::OFlags::PATH | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .unwrap();
    let refused = sync_dir(&path);
    assert!(
        matches!(
            &refused,
            Err(EvidenceError::Io(error))
                if error.raw_os_error() == Some(rustix::io::Errno::BADF.raw_os_error())
        ),
        "{refused:?}"
    );
}

/// M5: an archived retirement with no inventory digest still names the
/// archive it went to.
#[test]
fn an_archived_retirement_without_an_inventory_names_its_archive() {
    let retirement = Retirement {
        artifacts: "/evidence/root".into(),
        cause: Cause::Archived,
        archive: Some("/archive".into()),
        inventory_sha256: None,
        operator: "operator".into(),
        reason: "kept".into(),
        retired_at: "2026-11-08T00:00:00Z".into(),
        engine: "0.13.1".into(),
    };
    assert_eq!(retirement.outcome(), "archived at /archive");
    assert_eq!(
        retirement.words(),
        "archived at /archive on 2026-11-08T00:00:00Z by operator: kept"
    );
}
