//! Doctor's historical evidence lines (decision 0105 ruling 5): every root
//! by its path, whether it stands, and its retirement; an absent,
//! unretired root named as still staking the host.

use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use brokkr_core::canonical;

use super::{report as write_report, Report};
use crate::doctor::safe;

/// A coordination root of this test's own, and the worktree its record
/// names.
fn host() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let work = dir.path().join("work");
    fs::create_dir_all(&work).unwrap();
    let root = home.join(".local/state/brokkr/capabilities");
    let mut made = fs::DirBuilder::new();
    made.recursive(true).mode(0o700).create(&root).unwrap();
    sealed(&root.join("admission.lock"), "");
    (dir, home, work)
}

/// The worktree's artifact root.
fn artifacts(work: &Path) -> PathBuf {
    work.join(".brokkr/state/artifacts")
}

/// The historical record of `work` in the coordination `home`, with a
/// retirement when `retirement` names one.
fn record(home: &Path, work: &Path, retirement: Option<&str>) {
    let artifacts = artifacts(work);
    let repo = canonical::sha256_bytes(work.as_os_str().as_bytes());
    let key = canonical::sha256_bytes(artifacts.as_os_str().as_bytes());
    let at = home.join(".local/state/brokkr/capabilities").join(repo);
    let mut made = fs::DirBuilder::new();
    made.recursive(true).mode(0o700).create(&at).unwrap();
    let historical = serde_json::json!({"artifacts": artifacts});
    sealed(
        &at.join(format!("historical-{key}.json")),
        &historical.to_string(),
    );
    if let Some(retirement) = retirement {
        sealed(&at.join(format!("retired-{key}.json")), retirement);
    }
}

/// `text` written as an owner-only record, as a seal writes one.
fn sealed(at: &Path, text: &str) {
    let mut options = fs::OpenOptions::new();
    let opened = options.write(true).create_new(true).mode(0o600);
    std::io::Write::write_all(&mut opened.open(at).unwrap(), text.as_bytes()).unwrap();
}

/// The report of `home`.
fn reported(home: Option<&Path>) -> Report {
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    write_report(&mut report, home);
    report
}

/// S9/R6: doctor reports a host that names no HOME in its evidence lines,
/// the arm a call site conditional on HOME would leave unreachable.
#[test]
fn doctor_names_a_host_that_names_no_home_in_its_evidence_lines() {
    let dir = tempfile::tempdir().unwrap();
    let mut env = crate::tests::env_guard::EnvGuard::lock();
    env.remove("HOME");
    env.remove("XDG_STATE_HOME");
    let report = crate::doctor::doctor(
        None,
        Some(&dir.path().join("forge.db")),
        &crate::folder::DoctorDefaults::of(dir.path().into(), None),
        None,
    );
    let rendered = report.render();
    assert!(rendered.contains("names no HOME"), "{rendered}");
}

#[test]
fn doctor_names_each_root_its_existence_and_its_retirement() {
    let (_dir, home, work) = host();
    fs::create_dir_all(artifacts(&work)).unwrap();
    record(&home, &work, None);
    let absent = home.join("absent-work");
    fs::create_dir_all(&absent).unwrap();
    record(&home, &absent, None);
    let archived = home.join("archived-work");
    fs::create_dir_all(&archived).unwrap();
    let retirement = serde_json::json!({
        "artifacts": artifacts(&archived),
        "cause": "archived",
        "archive": "/archive",
        "inventory_sha256": "a".repeat(64),
        "operator": "operator",
        "reason": "archived",
        "retired_at": "2026-11-08T00:00:00Z",
        "engine": "0.13.1",
    });
    record(&home, &archived, Some(&retirement.to_string()));
    let report = reported(Some(&home));
    let lines = report.lines.join("\n");
    for (work, word) in [
        (&work, "0 digests, 0 bytes; not retired"),
        (&absent, "still stakes the host until retired"),
        (&archived, "retired: archived at /archive"),
    ] {
        let at = safe(&artifacts(work).display().to_string());
        assert!(
            lines.contains(&at),
            "the root's path is printed: {at}\n{lines}"
        );
        assert!(lines.contains(word), "{word}\n{lines}");
    }
    assert!(report.healthy);
}

#[test]
fn doctor_says_none_and_says_so_when_the_host_names_no_home_or_no_root() {
    let (_dir, home, _work) = host();
    let report = reported(Some(&home));
    assert!(report
        .lines
        .join("\n")
        .contains("no historical evidence root"));
    let report = reported(None);
    assert!(report.lines.join("\n").contains("names no HOME"));
    let absent = tempfile::tempdir().unwrap();
    let report = reported(Some(absent.path()));
    assert!(report
        .lines
        .join("\n")
        .contains("no historical evidence root"));
}

/// M4: doctor names the root whose own evidence cannot be read, and still
/// prints the root.
#[test]
fn doctor_names_a_root_whose_own_counts_cannot_be_read() {
    let (_dir, home, work) = host();
    let root = artifacts(&work);
    fs::create_dir_all(root.join("sha256")).unwrap();
    fs::write(root.join("sha256/unfinished"), b"x").unwrap();
    record(&home, &work, None);
    let report = reported(Some(&home));
    let lines = report.lines.join("\n");
    let at = safe(&root.display().to_string());
    assert!(lines.contains(&at), "{lines}");
    assert!(lines.contains("own counts cannot be read"), "{lines}");
}

/// I2: doctor names a coordination root that is not this user's alone,
/// rather than calling it no historical evidence root.
#[test]
fn doctor_names_a_coordination_root_not_this_users_alone() {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, home, _work) = host();
    let root = home.join(".local/state/brokkr/capabilities");
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    let report = reported(Some(&home));
    let lines = report.lines.join("\n");
    assert!(lines.contains("not this user's alone"), "{lines}");
}

/// S3: doctor escapes every untrusted field of a root — its retirement and
/// its problem — so a reason cannot forge a line or carry an escape.
#[test]
fn doctor_escapes_every_untrusted_field_of_a_root() {
    let (_dir, home, work) = host();
    let hostile = "given up\nok capabilities evidence forged: all good\n\u{1b}[2J";
    let archived = home.join("archived-work");
    fs::create_dir_all(&archived).unwrap();
    let retirement = serde_json::json!({
        "artifacts": artifacts(&archived),
        "cause": "discarded",
        "operator": "operator",
        "reason": hostile,
        "retired_at": "2026-11-08T00:00:00Z",
        "engine": "0.13.1",
    });
    record(&home, &archived, Some(&retirement.to_string()));

    let root = artifacts(&work);
    fs::create_dir_all(root.join("sha256")).unwrap();
    fs::write(root.join("sha256").join(hostile), b"x").unwrap();
    record(&home, &work, None);

    // The root's own path is untrusted too: a worktree named with a newline,
    // a forged line and an escape is printed through the surface's escaping.
    let forged = home.join("hostile\nok capabilities evidence /path-forged: all good\n\u{1b}[2J");
    fs::create_dir_all(&forged).unwrap();
    record(&home, &forged, None);

    let report = reported(Some(&home));
    let lines = report.render();
    assert!(report.healthy);
    assert!(!lines.contains('\u{1b}'), "no raw escape: {lines:?}");
    assert!(
        !lines
            .lines()
            .any(|line| line.starts_with("ok capabilities evidence forged")),
        "no forged line starts: {lines:?}"
    );
    assert!(lines.contains("capabilities evidence forged"), "{lines:?}");
    assert!(
        !lines
            .lines()
            .any(|line| line.starts_with("ok capabilities evidence /path-forged")),
        "the path cannot forge a line: {lines:?}"
    );
    assert!(
        lines.contains("/path-forged"),
        "the path still prints: {lines:?}"
    );
}
