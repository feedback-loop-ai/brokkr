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

/// A fresh directory of this test's own.
fn fresh() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
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

/// Whether `refused` is the io error a path through a file earns: not
/// absent, and not a directory.
fn not_a_directory<T>(refused: Result<T, EvidenceError>) -> bool {
    matches!(
        refused,
        Err(EvidenceError::Io(error)) if error.kind() == std::io::ErrorKind::NotADirectory
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
    destination::copied(&source, &target, &name).unwrap();
    assert_eq!(fs::read(dir.path().join(&name)).unwrap(), b"evidence");

    let link = dir.path().join("link");
    symlink(&source, &link).unwrap();
    let refused_dir = fresh();
    let refused = opened(refused_dir.path());
    assert!(matches!(
        destination::copied(&link, &refused, &name),
        Err(EvidenceError::Corrupt(at)) if at == link
    ));
    let wrong = canonical::sha256_bytes(b"other");
    let second_dir = fresh();
    let second = opened(second_dir.path());
    assert!(matches!(
        destination::copied(&source, &second, &wrong),
        Err(EvidenceError::Corrupt(at)) if at == source
    ));

    let fifo = dir.path().join("fifo");
    let made = std::process::Command::new("mkfifo").arg(&fifo).status();
    assert!(made.is_ok_and(|made| made.success()));
    let third_dir = fresh();
    let third = opened(third_dir.path());
    assert!(matches!(
        destination::copied(&fifo, &third, &name),
        Err(EvidenceError::Corrupt(at)) if at == fifo
    ));
}

/// A fresh directory of this test's own, opened for a copy's target.
fn opened(path: &Path) -> std::os::fd::OwnedFd {
    std::fs::File::open(path).unwrap().into()
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
    assert!(matches!(
        EvidenceError::from(Unknown::Unsealed),
        EvidenceError::Refused(_)
    ));
    assert_eq!(
        EvidenceError::from(Unknown::Unsealed).to_string(),
        Unknown::Unsealed.to_string(),
        "a refusal keeps its cause's words"
    );
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

/// M2: a directory that cannot be opened refuses by name, before any
/// sync.
#[test]
fn a_directory_that_cannot_be_opened_refuses_the_sync() {
    let dir = fresh();
    let absent = dir.path().join("absent");
    match destination::directory(&absent, (0, 0)) {
        Err(EvidenceError::DestinationNotDirectory(at)) => assert_eq!(at, absent),
        other => panic!("an absent directory must refuse by name: {other:?}"),
    }
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
    assert!(
        matches!(&refused, Err(EvidenceError::Io(error)) if error.kind() == std::io::ErrorKind::PermissionDenied),
        "{refused:?}"
    );
    assert!(!locked.join("made").exists(), "nothing was made");
}

/// M2: the digest directory this verb makes is removed when it cannot be
/// opened, so a refusal leaves the destination as empty as it found it.
/// Played in a child process, whose umask no parallel test reads.
#[test]
fn a_digest_directory_that_cannot_be_opened_is_removed() {
    let seen = std::process::Command::new("sh")
        .args([
            "-c",
            "exec \"$0\" --exact engine::broker::evidence::tests::a_digest_directory_is_removed_under_a_hostile_umask --ignored",
            std::env::current_exe().unwrap().to_str().unwrap(),
        ])
        .env(UMASK_ROLE, "1")
        .output()
        .unwrap();
    assert!(
        seen.status.success(),
        "{}",
        String::from_utf8_lossy(&seen.stderr)
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
    let previous = rustix::process::umask(rustix::fs::Mode::from_bits_truncate(0o777));
    let refused = opened.archive(
        &evidence,
        &mut Budget::new(soon()),
        &|| {},
        "2026-11-08T00:00:00Z",
    );
    rustix::process::umask(previous);
    assert!(
        matches!(&refused, Err(EvidenceError::DestinationNotDirectory(at)) if *at == to.join(DIGESTS)),
        "{refused:?}"
    );
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
    assert!(
        matches!(&refused, Err(EvidenceError::Io(error)) if error.kind() == std::io::ErrorKind::PermissionDenied),
        "{refused:?}"
    );
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

/// M4: a root whose metadata cannot be read, and a digest directory that is
/// not a directory, refuse with the io error rather than as absent.
#[test]
fn counts_and_digests_name_a_root_they_cannot_read() {
    let dir = fresh();
    let below = a_file(dir.path()).join("below");
    assert!(not_a_directory(counts(&below)));
    let root = dir.path().join("root");
    fs::create_dir(&root).unwrap();
    fs::write(root.join(DIGESTS), b"not a directory").unwrap();
    assert!(not_a_directory(digests(&root)));
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
