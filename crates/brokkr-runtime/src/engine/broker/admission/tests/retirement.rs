//! Decision 0105: a historical evidence root, what admission makes of it,
//! and the operator's retirement. Each test stands its own coordination
//! root in ([`Host`]), never the host's.

use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use brokkr_core::canonical;

use super::*;
use crate::engine::artifacts::DIGESTS;
use crate::engine::broker::evidence::{
    self, read_archive, Cause, EvidenceError, EvidenceRoot, RetireAction, RetireOptions,
};
use crate::engine::broker::historical::{self, retired, superseded};

/// The artifact root of the worktree `work`.
fn artifacts(work: &Path) -> PathBuf {
    work.join(".forge/artifacts")
}

/// A historical record for `work` settled and released, and its artifact
/// root.
fn historical_root(host: &Host, work: &Path, ids: [&str; 2]) -> PathBuf {
    let claim = claim(work, ids, Protects::Evidence, &[]);
    freed(&host.admit(&claim).unwrap());
    artifacts(work)
}

/// The host's `kept` worktree, and its historical root made absent: the
/// artifact root its record names is removed.
fn absent_root(host: &Host) -> (PathBuf, PathBuf) {
    let kept = host.worktree("kept");
    let root = historical_root(host, &kept, ["run-a", "a1"]);
    std::fs::remove_dir(&root).unwrap();
    (kept, root)
}

/// The retirement request for `root` with `action`.
/// An unboxed writer of another worktree refused by the historical root
/// `host` holds.
fn unboxed_refused(host: &Host, other: &Path) {
    let later = claim(
        other,
        ["run-b", "b1"],
        Protects::Nothing,
        &[unbounded("work")],
    );
    assert_eq!(host.admit(&later), cr2(unboxed("work", "run-b")));
}

/// The retirement request for `root` with `action`.
fn options<'a>(root: &'a Path, action: RetireAction<'a>, reason: &'a str) -> RetireOptions<'a> {
    RetireOptions {
        artifacts: root,
        action,
        operator: "operator",
        reason,
        now: "2026-11-08T00:00:00Z",
    }
}

/// The host's `kept` historical root with its evidence published: the
/// worktree and the artifact root.
fn published_root(host: &Host) -> (PathBuf, PathBuf) {
    let kept = host.worktree("kept");
    let root = historical_root(host, &kept, ["run-a", "a1"]);
    let _ = published(&root);
    (kept, root)
}

/// Archive `root` into `to` in `host`, answering the refusal or the
/// retirement.
fn archive(host: &Host, root: &Path, to: &Path) -> Result<evidence::Retirement, EvidenceError> {
    evidence::retire_in(
        &host.root,
        options(root, RetireAction::Archived { to }, "archived"),
        soon(),
    )
}

/// The refusal archiving `root` into `to` earns in `host`.
fn archive_refused(host: &Host, root: &Path, to: &Path) -> EvidenceError {
    archive(host, root, to).expect_err("the archive must refuse")
}

/// The root an `Absent` refusal names, archiving `root` into `to` in
/// `host`; any other answer is a failure.
fn absent_refused(host: &Host, root: &Path, to: &Path) -> PathBuf {
    match archive(host, root, to) {
        Err(EvidenceError::Absent(at)) => at,
        other => panic!("an absent root cannot be archived: {other:?}"),
    }
}

/// Archive the absent `root` into a fresh destination and assert the
/// refusal wrote nothing and sealed no retirement for the worktree `kept`.
fn a_refused_absent_archive_writes_nothing(host: &Host, kept: &Path, root: &Path) {
    let to = host.dir.path().join("archive");
    assert_eq!(absent_refused(host, root, &to), root);
    assert!(!to.exists(), "nothing was written");
    assert!(
        !retirement_path(host, kept, "run-a", root).exists(),
        "no retirement was sealed"
    );
}

/// Two digest-named artifacts under `root`, and what they hold.
fn published(root: &Path) -> Vec<(String, Vec<u8>)> {
    let digests = root.join(DIGESTS);
    std::fs::create_dir_all(&digests).unwrap();
    [b"one".as_slice(), b"two".as_slice()]
        .into_iter()
        .map(|bytes| {
            let name = canonical::sha256_bytes(bytes);
            std::fs::write(digests.join(&name), bytes).unwrap();
            (name, bytes.to_vec())
        })
        .collect()
}

/// The worktree directory's historical records beside the run's own
/// directory.
fn repo_dir(host: &Host, work: &Path, run: &str) -> PathBuf {
    host.reservation(work, run)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// The retirement's file for `root` in the worktree `work`'s directory.
fn retirement_path(host: &Host, work: &Path, run: &str, root: &Path) -> PathBuf {
    repo_dir(host, work, run).join(retired::name(&historical::key(root)))
}

#[test]
fn a_historical_record_stakes_the_host_and_an_unboxed_writer_refuses() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let _ = historical_root(&host, &kept, ["run-a", "a1"]);
    unboxed_refused(&host, &other);
}

#[test]
fn retiring_an_archived_root_stops_it_staking_and_names_every_digest() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let root = historical_root(&host, &kept, ["run-a", "a1"]);
    let held = published(&root);
    let to = host.dir.path().join("archive");
    let retired = evidence::retire_in(
        &host.root,
        options(&root, RetireAction::Archived { to: &to }, "archived"),
        soon(),
    )
    .unwrap();
    assert_eq!(retired.cause, Cause::Archived);
    assert_eq!(retired.archive.as_deref(), Some(to.as_path()));
    assert_eq!(retired.operator, "operator");
    assert_eq!(retired.reason, "archived");
    assert_eq!(retired.retired_at, "2026-11-08T00:00:00Z");
    assert_eq!(retired.engine, env!("CARGO_PKG_VERSION"));
    let inventory_bytes = std::fs::read(to.join("inventory.json")).unwrap();
    assert_eq!(
        retired.inventory_sha256.as_deref(),
        Some(canonical::sha256_bytes(&inventory_bytes).as_str())
    );
    let inventory = read_archive(&inventory_bytes).unwrap();
    assert_eq!(inventory.root, root);
    assert_eq!(inventory.engine, env!("CARGO_PKG_VERSION"));
    assert_eq!(inventory.archived_at, "2026-11-08T00:00:00Z");
    let mut expected: Vec<_> = held
        .iter()
        .map(|(sha256, bytes)| (sha256.clone(), bytes.len() as u64))
        .collect();
    expected.sort();
    let named: Vec<_> = inventory
        .artifacts
        .iter()
        .map(|artifact| (artifact.sha256.clone(), artifact.bytes))
        .collect();
    assert_eq!(named, expected, "every digest and its bytes");
    assert_eq!(
        inventory.total_bytes,
        held.iter()
            .map(|(_, bytes)| bytes.len() as u64)
            .sum::<u64>()
    );
    for (sha256, bytes) in &held {
        let copy = std::fs::read(to.join(DIGESTS).join(sha256)).unwrap();
        assert_eq!(&copy, bytes, "the copy is the evidence");
    }
    assert!(root.exists(), "archiving keeps the root");
    assert!(
        retirement_path(&host, &kept, "run-a", &root).exists(),
        "the retirement is sealed beside the historical record"
    );
    let listed = evidence::list_in(&host.root, soon()).unwrap();
    let found = listed
        .iter()
        .find(|root| root.artifacts == artifacts(&kept));
    let found = found.expect("the root is listed");
    assert!(found.exists && found.digests == 2 && found.bytes > 0);
    assert_eq!(found.retirement.as_ref().unwrap().cause, Cause::Archived);

    // The historical record stays; the root no longer guards evidence.
    let record = repo_dir(&host, &kept, "run-a").join(historical::name(&artifacts(&kept)));
    assert!(record.exists(), "the historical record is never deleted");
    let later = claim(
        &other,
        ["run-b", "b1"],
        Protects::Nothing,
        &[unbounded("work")],
    );
    assert!(host.admit(&later).is_ok(), "a retired root does not stake");
    assert!(matches!(
        evidence::retire_in(
            &host.root,
            options(&root, RetireAction::Discarded, "again"),
            soon()
        ),
        Err(EvidenceError::AlreadyRetired(at)) if at == root
    ));
}

#[test]
fn discarding_a_root_removes_it_and_records_an_absent_one() {
    let host = Host::new();
    let (kept, absent) = (host.worktree("kept"), host.worktree("absent"));
    let root = historical_root(&host, &kept, ["run-a", "a1"]);
    published(&root);
    let retired = evidence::retire_in(
        &host.root,
        options(&root, RetireAction::Discarded, "discarded"),
        soon(),
    )
    .unwrap();
    assert_eq!(retired.cause, Cause::Discarded);
    assert!(retired.archive.is_none() && retired.inventory_sha256.is_none());
    assert!(!root.exists(), "the discard removed the root");
    let listed = evidence::list_in(&host.root, soon()).unwrap();
    let found = listed.iter().find(|held| held.artifacts == root).unwrap();
    assert!(!found.exists);
    assert_eq!(found.retirement.as_ref().unwrap().cause, Cause::Discarded);

    let gone = historical_root(&host, &absent, ["run-b", "b1"]);
    std::fs::remove_dir_all(&gone).unwrap();
    let retired = evidence::retire_in(
        &host.root,
        options(&gone, RetireAction::Discarded, "already absent"),
        soon(),
    )
    .unwrap();
    assert_eq!(retired.cause, Cause::Discarded);
    assert!(
        !gone.exists(),
        "an already-absent root is recorded as absent"
    );
}

/// Ruling 2/1020b: an absent root cannot be archived; `--discarded` is its
/// record, and nothing is written at the destination.
#[test]
fn archiving_an_absent_root_refuses_and_writes_nothing() {
    let host = Host::new();
    let (kept, root) = absent_root(&host);
    a_refused_absent_archive_writes_nothing(&host, &kept, &root);
}

/// Ruling 2/1020b: the chief's `kept/.forge` case — the destination is the
/// absent root's own parent — refuses the archive as well, so no empty
/// archive is written there.
#[test]
fn archiving_an_absent_root_into_its_own_parent_refuses_and_writes_nothing() {
    let host = Host::new();
    let (kept, root) = absent_root(&host);
    let to = root.parent().unwrap().to_path_buf();
    assert_eq!(absent_refused(&host, &root, &to), root);
    assert!(!to.join("inventory.json").exists(), "no inventory");
    assert!(!to.join(DIGESTS).exists(), "no empty archive");
    assert!(!retirement_path(&host, &kept, "run-a", &root).exists());
}

/// Ruling 2/1020b: a dangling link at the artifact root does not stand, so
/// it refuses as absent rather than archiving an empty retirement, and the
/// link is left where it was.
#[test]
fn archiving_a_dangling_root_link_refuses_as_absent() {
    let host = Host::new();
    let (kept, root) = absent_root(&host);
    let nowhere = host.dir.path().join("nowhere");
    std::os::unix::fs::symlink(&nowhere, &root).unwrap();
    a_refused_absent_archive_writes_nothing(&host, &kept, &root);
    assert!(root.is_symlink(), "the link is left where it was");
}

/// Ruling 2/1020b: `--discarded` of an absent root still records it as
/// absent, unchanged.
#[test]
fn discarding_an_absent_root_still_records_it_as_absent() {
    let host = Host::new();
    let (_kept, root) = absent_root(&host);
    let retired = evidence::retire_in(
        &host.root,
        options(&root, RetireAction::Discarded, "already absent"),
        soon(),
    )
    .unwrap();
    assert_eq!(retired.cause, Cause::Discarded);
    assert!(retired.archive.is_none());
    assert!(!root.exists());
    let listed = evidence::list_in(&host.root, soon()).unwrap();
    let row = listed.iter().find(|held| held.artifacts == root).unwrap();
    assert!(!row.exists, "the record says the root is absent");
    assert_eq!(row.retirement.as_ref(), Some(&retired));
}

/// 12(e)/1020b: the record takes the operator's words, so an empty or
/// whitespace-only reason refuses before anything is written.
#[test]
fn an_empty_reason_refuses_before_anything_is_written() {
    let host = Host::new();
    let (_kept, root) = published_root(&host);
    let to = host.dir.path().join("archive");
    let refused = evidence::retire_in(
        &host.root,
        options(&root, RetireAction::Archived { to: &to }, "  "),
        soon(),
    );
    assert!(matches!(refused, Err(EvidenceError::Reason)));
    assert!(!to.exists(), "nothing was written");
    assert!(root.exists(), "the root stands");
}

/// 1020b: a stat of the root that fails for another reason than absence is
/// named as the io error, before the destination is opened.
#[test]
fn archiving_a_root_whose_stat_is_denied_names_the_io_error() {
    let host = Host::new();
    let kept = host.worktree("kept");
    let root = historical_root(&host, &kept, ["run-a", "a1"]);
    let forge = kept.join(".forge");
    std::fs::set_permissions(&forge, std::fs::Permissions::from_mode(0o000)).unwrap();
    let to = host.dir.path().join("archive");
    let refused = archive(&host, &root, &to);
    std::fs::set_permissions(&forge, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        matches!(&refused, Err(EvidenceError::Io(error)) if error.kind() == std::io::ErrorKind::PermissionDenied),
        "{refused:?}"
    );
    assert!(!to.exists(), "nothing was written");
}

/// 1020b: retire finds a root through admission's `beside`, so a worktree
/// named through a link retires the root inspect finds.
#[test]
fn retiring_through_a_symlinked_worktree_finds_the_root() {
    let host = Host::new();
    let kept = host.worktree("kept");
    let root = historical_root(&host, &kept, ["run-a", "a1"]);
    published(&root);
    let link = host.dir.path().join("link");
    std::os::unix::fs::symlink(&kept, &link).unwrap();
    let spelled = link.join(".forge/artifacts");
    let retired = evidence::retire_in(
        &host.root,
        options(&spelled, RetireAction::Discarded, "given up"),
        soon(),
    )
    .unwrap();
    assert_eq!(
        retired.artifacts, root,
        "the record's resolved root is named"
    );
    assert!(!root.exists(), "the root inspect finds was retired");
}

#[test]
fn a_standing_attempt_that_protects_a_root_refuses_its_retirement() {
    let host = Host::new();
    let kept = host.worktree("kept");
    let held = host
        .admit(&claim(&kept, ["run-a", "a1"], Protects::Evidence, &[]))
        .unwrap();
    let root = artifacts(&kept);
    let to = host.dir.path().join("archive");
    match evidence::retire_in(
        &host.root,
        options(&root, RetireAction::Archived { to: &to }, "archived"),
        soon(),
    ) {
        Err(EvidenceError::Protected { root: at, run }) => {
            assert_eq!(at, root);
            assert_eq!(run, "run-a");
        }
        other => panic!("a standing protecting attempt must refuse: {other:?}"),
    }
    assert!(!to.exists(), "nothing was written");
    freed(&held);
}

/// A boxed writer of another run that can write `kept` refuses its
/// retirement, naming that run. Linux only: a boxed writer beside evidence
/// passes the mount table's judgement, which only Linux reads.
#[cfg(target_os = "linux")]
#[test]
fn a_standing_writer_that_can_reach_a_root_refuses_its_retirement() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let root = historical_root(&host, &kept, ["run-a", "a1"]);
    let standing = claim(
        &other,
        ["run-s", "s1"],
        Protects::Nothing,
        &[boxed("work", &[&other])],
    );
    let _held = host.admit(&standing).unwrap();
    planted_writers(
        &host,
        &other,
        ["run-s", "s1"],
        vec![boxed("work", &[&other, &kept])],
    );
    match evidence::retire_in(
        &host.root,
        options(&root, RetireAction::Discarded, "discarded"),
        soon(),
    ) {
        Err(EvidenceError::Protected { root: at, run }) => {
            assert_eq!(at, root);
            assert_eq!(run, "run-s");
        }
        other => panic!("a standing writer that reaches must refuse: {other:?}"),
    }
    assert!(root.exists(), "nothing was removed");
}

#[test]
fn a_destination_that_holds_a_file_refuses() {
    let host = Host::new();
    let kept = host.worktree("kept");
    let root = historical_root(&host, &kept, ["run-a", "a1"]);
    let full = host.dir.path().join("full");
    std::fs::create_dir(&full).unwrap();
    std::fs::write(full.join("file"), b"x").unwrap();
    assert!(matches!(
        evidence::retire_in(
            &host.root,
            options(&root, RetireAction::Archived { to: &full }, "archived"),
            soon()
        ),
        Err(EvidenceError::DestinationNotEmpty(at)) if at == full
    ));
    let file = host.dir.path().join("file");
    std::fs::write(&file, b"x").unwrap();
    assert!(matches!(
        evidence::retire_in(
            &host.root,
            options(&root, RetireAction::Archived { to: &file }, "archived"),
            soon()
        ),
        Err(EvidenceError::DestinationNotDirectory(at)) if at == file
    ));
    assert!(!retirement_path(&host, &kept, "run-a", &root).exists());
}

#[test]
fn a_mismatched_digest_refuses_with_nothing_written() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let root = historical_root(&host, &kept, ["run-a", "a1"]);
    let digest = published(&root);
    let name = digest[0].0.clone();
    let source = root.join(DIGESTS).join(&name);
    let to = host.dir.path().join("missing/nested/archive");
    let between = || std::fs::write(&source, b"tampered with").unwrap();
    let refused = evidence::retire_in_with(
        &host.root,
        options(&root, RetireAction::Archived { to: &to }, "archived"),
        soon(),
        &between,
    );
    assert!(
        matches!(&refused, Err(EvidenceError::Corrupt(at)) if at == &source),
        "{refused:?}"
    );
    assert!(!to.exists(), "no copy, no inventory, no destination kept");
    assert!(
        !host.dir.path().join("missing").exists(),
        "the directories the resolution made are removed too"
    );
    assert!(!retirement_path(&host, &kept, "run-a", &root).exists());
    let listed = evidence::list_in(&host.root, soon()).unwrap();
    let found = listed.iter().find(|held| held.artifacts == root).unwrap();
    assert!(found.retirement.is_none(), "no retirement was written");
    // The evidence still stakes the host: nothing was laundered.
    std::fs::write(&source, &digest[0].1).unwrap();
    unboxed_refused(&host, &other);
}

#[test]
fn an_unreadable_retirement_refuses_admission_with_cr2s_cause() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let root = historical_root(&host, &kept, ["run-a", "a1"]);
    let at = retirement_path(&host, &kept, "run-a", &root);
    std::fs::write(&at, b"not a retirement").unwrap();
    refused_with(&host, &other, Conflict::Retirement(at));
}

#[test]
fn a_retirement_that_does_not_parse_refuses_admission_with_cr2s_cause() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let root = historical_root(&host, &kept, ["run-a", "a1"]);
    let at = retirement_path(&host, &kept, "run-a", &root);
    // This user's own single regular file, so the read succeeds and the
    // parse is the refusal that names `retired_at`'s cause.
    written(&at, b"not a retirement");
    refused_with(&host, &other, Conflict::Retirement(at));
}

#[test]
fn a_link_at_the_root_refuses_the_discard() {
    let host = Host::new();
    let kept = host.worktree("kept");
    let root = historical_root(&host, &kept, ["run-a", "a1"]);
    let outside = host.dir.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    std::os::unix::fs::symlink(&outside, &root).unwrap();
    assert!(matches!(
        evidence::retire_in(
            &host.root,
            options(&root, RetireAction::Discarded, "discarded"),
            soon()
        ),
        Err(EvidenceError::Link(at)) if at == root
    ));
    assert!(outside.exists(), "the link was not followed");
    assert!(!retirement_path(&host, &kept, "run-a", &root).exists());
}

#[test]
fn a_root_on_another_filesystem_refuses_the_discard() {
    let host = Host::new();
    let kept = host.worktree("kept");
    let root = historical_root(&host, &kept, ["run-a", "a1"]);
    let elsewhere = {
        let root = root.clone();
        move |path: &Path| Some(if path == root { 2 } else { 1 })
    };
    assert!(matches!(
        evidence::discard_with(&root, &elsewhere),
        Err(EvidenceError::OtherFilesystem(at)) if at == root
    ));
    assert!(root.exists(), "nothing was removed");
    // The injected equivalent is the only way to show a second filesystem
    // without mounting one; the production device read is this one.
    let devices = |path: &Path| Some(std::fs::symlink_metadata(path).unwrap().dev());
    assert!(
        evidence::discard_with(&root, &devices).is_ok(),
        "on this host the root is one filesystem"
    );
}

#[test]
fn a_root_with_no_historical_record_or_a_past_deadline_refuses() {
    let host = Host::new();
    let work = host.worktree("work");
    let root = artifacts(&work);
    std::fs::create_dir_all(&root).unwrap();
    assert!(matches!(
        evidence::retire_in(
            &host.root,
            options(&root, RetireAction::Discarded, "not recorded"),
            soon()
        ),
        Err(EvidenceError::NotRecorded(at)) if at == root
    ));
    let kept = host.worktree("kept");
    let recorded = historical_root(&host, &kept, ["run-a", "a1"]);
    assert!(matches!(
        evidence::retire_in(
            &host.root,
            options(&recorded, RetireAction::Discarded, "late"),
            std::time::Instant::now()
        ),
        Err(EvidenceError::Expired)
    ));
    assert!(recorded.exists(), "the late verb wrote nothing");
}

#[test]
fn list_names_every_root_its_existence_and_its_retirement() {
    let host = Host::new();
    let (kept, absent) = (host.worktree("kept"), host.worktree("absent"));
    let root = historical_root(&host, &kept, ["run-a", "a1"]);
    let gone = historical_root(&host, &absent, ["run-b", "b1"]);
    std::fs::remove_dir_all(&gone).unwrap();
    let listed: Vec<EvidenceRoot> = evidence::list_in(&host.root, soon()).unwrap();
    assert_eq!(listed.len(), 2);
    let kept_row = listed.iter().find(|held| held.artifacts == root).unwrap();
    let absent_row = listed.iter().find(|held| held.artifacts == gone).unwrap();
    assert!(kept_row.exists && !absent_row.exists);
    assert_eq!(
        absent_row.retirement, None,
        "an absent root is still unretired"
    );
    assert_eq!(
        evidence::retirement_in(&host.root, &root, soon()).unwrap(),
        None
    );
}

#[test]
fn a_retirement_is_read_back_by_its_root_and_only_a_deadline_can_hide_it() {
    let host = Host::new();
    let kept = host.worktree("kept");
    let root = historical_root(&host, &kept, ["run-a", "a1"]);
    let retired = evidence::retire_in(
        &host.root,
        options(&root, RetireAction::Discarded, "discarded"),
        soon(),
    )
    .unwrap();
    assert_eq!(
        evidence::retirement_in(&host.root, &root, soon()).unwrap(),
        Some(retired)
    );
    assert!(matches!(
        evidence::retirement_in(&host.root, &root, std::time::Instant::now()),
        Err(EvidenceError::Expired)
    ));
}

/// The superseded record's path for `root` in `work`'s worktree directory.
fn superseded_path(host: &Host, work: &Path, run: &str, root: &Path) -> PathBuf {
    repo_dir(host, work, run).join(superseded::name(&historical::key(root), 1))
}

/// The superseded records in the worktree `work`'s directory for `run`, by
/// name.
fn superseded_files(host: &Host, work: &Path, run: &str) -> Vec<String> {
    std::fs::read_dir(repo_dir(host, work, run))
        .unwrap()
        .filter_map(|entry| Some(entry.ok()?.file_name().to_str()?.to_string()))
        .filter(|name| name.starts_with("superseded-"))
        .collect()
}

/// The equal superseded record a crash between the two superseding steps
/// leaves beside the standing retirement.
fn planted_supersede(
    host: &Host,
    work: &Path,
    run: &str,
    retirement: &evidence::Retirement,
    root: &Path,
) {
    let record = superseded::Superseded::of(retirement, run, "2026-11-08T00:00:01Z");
    written(
        &superseded_path(host, work, run, root),
        &serde_json::to_vec(&record).unwrap(),
    );
}

/// A historical root of `kept` with its evidence published, retired as
/// discarded with `reason`.
fn retired_discarded(host: &Host, kept: &Path, reason: &str) -> (PathBuf, evidence::Retirement) {
    let root = historical_root(host, kept, ["run-a", "a1"]);
    let _ = published(&root);
    let retirement = evidence::retire_in(
        &host.root,
        options(&root, RetireAction::Discarded, reason),
        soon(),
    )
    .unwrap();
    (root, retirement)
}

/// A witness of `other` refused by admission with `conflict`.
fn refused_with(host: &Host, other: &Path, conflict: Conflict) {
    let later = claim(other, ["run-b", "b1"], Protects::Nothing, &[]);
    assert_eq!(host.admit(&later), cr2(conflict));
}

/// H1: a retirement covers the evidence its root held when it was retired.
/// A later evidence-protecting admission at the same root supersedes it,
/// the root stakes the host again, and an unbounded writer refuses.
#[test]
fn a_retired_root_that_protects_evidence_again_stakes_and_refuses_an_unboxed_writer() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let (root, _) = retired_discarded(&host, &kept, "given up");
    assert!(!root.exists(), "the discard removed the root");

    let again = claim(&kept, ["run-a", "a2"], Protects::Evidence, &[]);
    let held = host.admit(&again).unwrap();
    let second = published(&root);
    assert_eq!(second.len(), 2, "two digest-correct files were published");
    freed(&held);

    unboxed_refused(&host, &other);

    let listed = evidence::list_in(&host.root, soon()).unwrap();
    let row = listed.iter().find(|held| held.artifacts == root).unwrap();
    assert!(row.retirement.is_none(), "the old retirement is history");
    assert_eq!(row.history.len(), 1, "it is on record as superseded");
    assert_eq!(row.history[0].retirement.cause, Cause::Discarded);
    assert_eq!(row.history[0].run, "run-a");

    // The root can be retired again, and the second retirement stands.
    let retired = evidence::retire_in(
        &host.root,
        options(&root, RetireAction::Discarded, "again"),
        soon(),
    )
    .unwrap();
    assert_eq!(retired.cause, Cause::Discarded);
    let listed = evidence::list_in(&host.root, soon()).unwrap();
    let row = listed.iter().find(|held| held.artifacts == root).unwrap();
    assert_eq!(row.retirement.as_ref(), Some(&retired));
    assert_eq!(
        row.history.len(),
        1,
        "the first retirement is still history"
    );
}

/// H1: a crash between sealing the superseded record and removing the
/// retirement leaves both; the next admission finishes the move and writes
/// no second copy of the same retirement. The first supersede is a real
/// one, interrupted at its seal ([`Late::UnstageSuperseded`]), so a
/// mutation that removes the retirement before the seal is substantiated
/// fails here. Since R7 the supersede runs after the reservation is sealed,
/// so the interrupted attempt's reservation stands.
#[test]
fn a_crash_between_superseding_steps_finishes_with_one_copy() {
    let host = Host::new();
    let kept = host.worktree("kept");
    let (root, _retirement) = retired_discarded(&host, &kept, "given up");
    let key = historical::key(&root);

    LATE.set(Some(Late::UnstageSuperseded));
    let again = claim(&kept, ["run-a", "a2"], Protects::Evidence, &[]);
    assert_eq!(host.admit(&again), mb3(Unknown::Unsealed));
    let (standing, settled) = host.attempts(&kept, "run-a");
    assert!(
        standing.contains(&"a2".to_string()),
        "the attempt's reservation stands: {standing:?} settled {settled:?}"
    );
    assert!(
        retirement_path(&host, &kept, "run-a", &root).exists(),
        "the retirement still stands after the interrupted seal"
    );
    assert!(
        !superseded_path(&host, &kept, "run-a", &root).exists(),
        "no superseded copy was kept"
    );

    let held = host.admit(&again).unwrap();
    freed(&held);

    assert_eq!(
        superseded_files(&host, &kept, "run-a"),
        vec![superseded::name(&key, 1)],
        "one copy, no second"
    );
    assert!(!retirement_path(&host, &kept, "run-a", &root).exists());
    let listed = evidence::list_in(&host.root, soon()).unwrap();
    let row = listed.iter().find(|held| held.artifacts == root).unwrap();
    assert_eq!(row.history.len(), 1);
}

/// H1: a crash that sealed the superseded record but did not remove the
/// retirement leaves both; the next admission finishes the move and writes
/// no second copy of the very same retirement, the both-stand state a real
/// interruption reaches only after the seal.
#[test]
fn a_sealed_superseded_copy_is_not_sealed_again() {
    let host = Host::new();
    let kept = host.worktree("kept");
    let (root, retirement) = retired_discarded(&host, &kept, "given up");
    planted_supersede(&host, &kept, "run-a", &retirement, &root);
    assert!(
        retirement_path(&host, &kept, "run-a", &root).exists(),
        "the retirement and its copy stand together"
    );

    let again = claim(&kept, ["run-a", "a2"], Protects::Evidence, &[]);
    let held = host.admit(&again).unwrap();
    freed(&held);

    let key = historical::key(&root);
    assert_eq!(
        superseded_files(&host, &kept, "run-a"),
        vec![superseded::name(&key, 1)],
        "the equal copy is reused, never sealed a second time"
    );
    assert!(!retirement_path(&host, &kept, "run-a", &root).exists());
}

/// S5: a reservation seal that fails after the comparison leaves the
/// retirement standing, never moved into history by a refused admission.
#[test]
fn a_reservation_seal_that_fails_keeps_the_retirement() {
    let host = Host::new();
    let kept = host.worktree("kept");
    let (root, retirement) = retired_discarded(&host, &kept, "given up");
    // The retirement is already on record as history, so the supersede
    // step makes no seal of its own and the reservation's is the next one.
    planted_supersede(&host, &kept, "run-a", &retirement, &root);

    LATE.set(Some(Late::Unstage));
    let again = claim(&kept, ["run-a", "a2"], Protects::Evidence, &[]);
    assert_eq!(host.admit(&again), mb3(Unknown::Unsealed));
    assert_eq!(
        evidence::retirement_in(&host.root, &root, soon()).unwrap(),
        Some(retirement),
        "the retirement still stands after the reservation seal failed"
    );
}

/// H1: a superseded record that does not read refuses admission with CR2's
/// cause, never read as absent history.
#[test]
fn an_unreadable_superseded_record_refuses_admission_with_cr2s_cause() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let (root, _) = retired_discarded(&host, &kept, "given up");
    let at = superseded_path(&host, &kept, "run-a", &root);
    std::fs::write(&at, b"not a superseded record").unwrap();
    refused_with(&host, &other, Conflict::Retirement(at));
}

/// M1: a destination inside the coordination root, or inside the root being
/// retired, refuses by name before anything is written, and a `..`
/// spelling does not hide it.
#[test]
fn a_destination_inside_either_root_refuses_with_nothing_written() {
    let host = Host::new();
    let (kept, root) = published_root(&host);
    let inside_coordination = host.root.path().join("archive");
    assert!(matches!(
        archive_refused(&host, &root, &inside_coordination),
        EvidenceError::ArchiveInsideCoordination(at) if at == inside_coordination
    ));
    assert!(!inside_coordination.exists(), "nothing was written");
    let spelled = host
        .root
        .path()
        .join("..")
        .join(host.root.path().file_name().unwrap())
        .join("archive");
    assert!(matches!(
        archive_refused(&host, &root, &spelled),
        EvidenceError::ArchiveInsideCoordination(at) if at == spelled
    ));
    assert!(!spelled.exists(), "nothing was written");
    let inside_root = root.join("archive");
    assert!(matches!(
        archive_refused(&host, &root, &inside_root),
        EvidenceError::ArchiveInsideRoot(at) if at == inside_root
    ));
    assert!(!inside_root.exists(), "nothing was written");
    assert!(!retirement_path(&host, &kept, "run-a", &root).exists());
}

/// I1: `retire()` refuses a HOME whose ancestors are not protected, before
/// it reads any root.
#[test]
fn a_home_whose_ancestors_are_not_protected_refuses_the_retire_verb() {
    let host = Host::new();
    let open = host.dir.path().join("open");
    std::fs::create_dir(&open).unwrap();
    std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o777)).unwrap();
    let home = open.join("home");
    std::fs::create_dir(&home).unwrap();
    let root = PathBuf::from("/not/read");
    assert!(matches!(
        evidence::retire(
            &home,
            options(&root, RetireAction::Discarded, "not recorded"),
            soon()
        ),
        Err(EvidenceError::Coordination)
    ));
}

/// `bytes` written as the new owner-only record `at`, as a seal writes one.
fn written(at: &Path, bytes: &[u8]) {
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true).mode(0o600);
    std::io::Write::write_all(&mut options.open(at).unwrap(), bytes).unwrap();
}

/// S1: a link spelled before `..` does not hide a destination inside the
/// coordination root: the filesystem resolves it, and nothing is written.
#[test]
fn a_link_before_a_parent_does_not_hide_a_destination_inside_the_coordination_root() {
    let host = Host::new();
    let (kept, root) = published_root(&host);
    let link = host.dir.path().join("link");
    std::os::unix::fs::symlink(repo_dir(&host, &kept, "run-a"), &link).unwrap();
    let to = link.join("..").join("archive");
    assert!(matches!(
        archive_refused(&host, &root, &to),
        EvidenceError::ArchiveInsideCoordination(at) if at == to
    ));
    assert!(
        !host.root.path().join("archive").exists(),
        "no archive, no inventory was written under the coordination root"
    );
    assert!(!retirement_path(&host, &kept, "run-a", &root).exists());
}

/// S1: a link spelled before `..` does not hide a destination inside the
/// root being retired either.
#[test]
fn a_link_before_a_parent_does_not_hide_a_destination_inside_the_root() {
    let host = Host::new();
    let (kept, root) = published_root(&host);
    let nested = root.join("nested");
    std::fs::create_dir(&nested).unwrap();
    let link = host.dir.path().join("alias");
    std::os::unix::fs::symlink(&nested, &link).unwrap();
    let to = link.join("..").join("archive");
    assert!(matches!(
        archive_refused(&host, &root, &to),
        EvidenceError::ArchiveInsideRoot(at) if at == to
    ));
    assert!(!root.join("archive").exists(), "nothing was written");
    assert!(!retirement_path(&host, &kept, "run-a", &root).exists());
}

/// S1: the destination recorded is the path the filesystem resolved, not
/// the spelling the operator gave.
#[test]
fn the_resolved_destination_is_the_one_recorded() {
    let host = Host::new();
    let (_kept, root) = published_root(&host);
    let outside = host.dir.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    let link = host.dir.path().join("outside-link");
    std::os::unix::fs::symlink(&outside, &link).unwrap();
    let to = link.join("..").join("archive");
    let retired = archive(&host, &root, &to).unwrap();
    let expected = host.dir.path().join("archive");
    assert_ne!(to, expected, "the spelling and the resolution differ");
    assert_eq!(retired.archive.as_deref(), Some(expected.as_path()));
    assert!(expected.join("inventory.json").exists());
}

/// S5: a claim refused after the comparison leaves the retirement and the
/// history unchanged; only an admitted claim supersedes.
#[test]
fn chief_refused_admission_keeps_the_retirement() {
    let host = Host::new();
    let kept = host.worktree("kept");
    let (root, retirement) = retired_discarded(&host, &kept, "given up");
    // The claim protects evidence, so its own unbounded writer is compared
    // and refused against the coordination root after the comparison.
    let again = claim(
        &kept,
        ["run-a", "a2"],
        Protects::Evidence,
        &[unbounded("work")],
    );
    assert_eq!(host.admit(&again), cr2(unboxed("work", "run-a")));
    assert_eq!(
        evidence::retirement_in(&host.root, &root, soon()).unwrap(),
        Some(retirement)
    );
    let listed = evidence::list_in(&host.root, soon()).unwrap();
    let row = listed.iter().find(|held| held.artifacts == root).unwrap();
    assert!(
        row.history.is_empty(),
        "a refused admission writes no history"
    );
    assert!(!superseded_path(&host, &kept, "run-a", &root).exists());
}

/// S6: a history whose last sequence has no next refuses with a typed
/// cause, never an overflow panic.
#[test]
fn a_sequence_with_no_next_refuses_with_a_typed_cause() {
    let host = Host::new();
    let kept = host.worktree("kept");
    let (root, retirement) = retired_discarded(&host, &kept, "given up");
    let mut older = retirement;
    older.reason = "older".to_string();
    let record = superseded::Superseded::of(&older, "run-b", "2026-11-08T00:00:01Z");
    let key = historical::key(&root);
    let at = repo_dir(&host, &kept, "run-a").join(superseded::name(&key, u64::MAX));
    written(&at, &serde_json::to_vec(&record).unwrap());
    let again = claim(&kept, ["run-a", "a2"], Protects::Evidence, &[]);
    assert_eq!(host.admit(&again), mb3(Unknown::Overflow));
    assert!(
        retirement_path(&host, &kept, "run-a", &root).exists(),
        "nothing was removed"
    );
}

/// S7: a superseded record with a field its outer closed shape does not
/// know refuses with CR2's cause.
#[test]
fn a_superseded_record_with_an_unknown_field_refuses_with_cr2s_cause() {
    unknown_field_refused(false);
}

/// S7/1020b: an unknown field inside the retirement a superseded record
/// nests refuses too, closed at both levels.
#[test]
fn a_nested_retirement_with_an_unknown_field_refuses_with_cr2s_cause() {
    unknown_field_refused(true);
}

/// A superseded record with an unknown field at its own level or, where
/// `nested`, inside the retirement it nests, refuses with CR2's cause.
fn unknown_field_refused(nested: bool) {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let (root, retirement) = retired_discarded(&host, &kept, "given up");
    let at = superseded_with_extra(&host, &kept, &root, &retirement, |value| {
        if nested {
            value["retirement"]["extra"] = serde_json::json!(true);
        } else {
            value["extra"] = serde_json::json!(true);
        }
    });
    refused_with(&host, &other, Conflict::Retirement(at));
}

/// The superseded record `retirement` of `root` in `kept`, changed by
/// `edit` and written under the host's coordination root: the path it was
/// written at.
fn superseded_with_extra(
    host: &Host,
    kept: &Path,
    root: &Path,
    retirement: &evidence::Retirement,
    edit: impl Fn(&mut serde_json::Value),
) -> PathBuf {
    let at = superseded_path(host, kept, "run-a", root);
    let record = superseded::Superseded::of(retirement, "run-b", "2026-11-08T00:00:01Z");
    let mut value = serde_json::to_value(&record).unwrap();
    edit(&mut value);
    written(&at, &serde_json::to_vec(&value).unwrap());
    at
}

/// A host with two worktrees and a retirement that reads but names no
/// historical record: the host, the kept and other worktrees, the
/// retirement, and a key no record carries.
fn orphan() -> (Host, PathBuf, PathBuf, evidence::Retirement, String) {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let (_root, retirement) = retired_discarded(&host, &kept, "given up");
    (host, kept, other, retirement, "b".repeat(64))
}

/// CR2: a retirement that reads but names no historical record refuses by
/// its own path, never read as history for some other root.
#[test]
fn a_retirement_that_names_no_record_refuses_admission() {
    let (host, kept, other, retirement, key) = orphan();
    let at = repo_dir(&host, &kept, "run-a").join(retired::name(&key));
    written(&at, &serde_json::to_vec(&retirement).unwrap());
    refused_with(&host, &other, Conflict::Retirement(at));
}

/// H1: a superseded retirement that reads but names no historical record
/// refuses by its own path, as a bare retirement does.
#[test]
fn a_superseded_retirement_that_names_no_record_refuses_admission() {
    let (host, kept, other, retirement, key) = orphan();
    let at = repo_dir(&host, &kept, "run-a").join(superseded::name(&key, 1));
    let record = superseded::Superseded::of(&retirement, "run-b", "2026-11-08T00:00:01Z");
    written(&at, &serde_json::to_vec(&record).unwrap());
    refused_with(&host, &other, Conflict::Retirement(at));
}

/// M2: a source that verification read but the copy cannot open refuses
/// after `sha256/` stands, and the removal takes the digest directory and
/// every directory the resolution made.
#[test]
fn a_source_that_cannot_be_read_after_verification_removes_the_digest_directory() {
    let host = Host::new();
    let kept = host.worktree("kept");
    let root = historical_root(&host, &kept, ["run-a", "a1"]);
    let digest = published(&root);
    let source = root.join(DIGESTS).join(&digest[0].0);
    let to = host.dir.path().join("missing/nested/archive");
    let between = || {
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o000)).unwrap();
    };
    let refused = evidence::retire_in_with(
        &host.root,
        options(&root, RetireAction::Archived { to: &to }, "archived"),
        soon(),
        &between,
    );
    std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(
        matches!(&refused, Err(EvidenceError::Corrupt(at)) if at == &source),
        "{refused:?}"
    );
    assert!(!to.exists(), "the directories the resolution made are gone");
    assert!(
        !host.dir.path().join("missing").exists(),
        "the first directory made is gone too"
    );
    assert!(!retirement_path(&host, &kept, "run-a", &root).exists());
}
