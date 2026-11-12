//! Doctor's historical evidence lines (decision 0105 ruling 5): every root
//! by its path, whether it stands, and its retirement; an absent,
//! unretired root named as still staking the host.

use std::fs;
use std::path::Path;

use super::{report as write_report, Report};
use crate::doctor::safe;
use crate::verbs::evidence::fixture;

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
    let host = fixture::Host::new();
    fs::create_dir_all(host.artifacts()).unwrap();
    host.record(None);
    let absent = host.home.join("absent-work");
    fs::create_dir_all(&absent).unwrap();
    fixture::record(&host.home, &absent, None);
    let archived = host.home.join("archived-work");
    fs::create_dir_all(&archived).unwrap();
    let retirement = serde_json::json!({
        "artifacts": fixture::artifacts(&archived),
        "cause": "archived",
        "archive": "/archive",
        "inventory_sha256": "a".repeat(64),
        "operator": "operator",
        "reason": "archived",
        "retired_at": "2026-11-08T00:00:00Z",
        "engine": "0.13.1",
    });
    fixture::record(&host.home, &archived, Some(&retirement.to_string()));
    let report = reported(Some(&host.home));
    let lines = report.lines.join("\n");
    for (work, word) in [
        (&host.work, "0 digests, 0 bytes; not retired"),
        (&absent, "still stakes the host until retired"),
        (&archived, "retired: archived at /archive"),
    ] {
        let at = safe(&fixture::artifacts(work).display().to_string());
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
    let host = fixture::Host::new();
    let report = reported(Some(&host.home));
    assert!(report
        .lines
        .join("\n")
        .contains("no historical evidence root"));
    let report = reported(None);
    assert!(report.lines.join("\n").contains("names no HOME"));
    // TMPDIR canonical, as admission's fixtures are, so the protected walk
    // reaches this HOME on macOS, where TMPDIR sits under a link.
    let base = fs::canonicalize(std::env::temp_dir()).unwrap();
    let absent = tempfile::tempdir_in(base).unwrap();
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
    let host = fixture::Host::new();
    let root = host.artifacts();
    fs::create_dir_all(root.join("sha256")).unwrap();
    fs::write(root.join("sha256/unfinished"), b"x").unwrap();
    host.record(None);
    let report = reported(Some(&host.home));
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

    let host = fixture::Host::new();
    let root = host.home.join(".local/state/brokkr/capabilities");
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    let report = reported(Some(&host.home));
    let lines = report.lines.join("\n");
    assert!(lines.contains("not this user's alone"), "{lines}");
}

/// I2/1020b: doctor takes the ancestor walk admission and retire take: an
/// ancestor that is not this user's alone is named by the path that is.
#[test]
fn doctor_names_an_ancestor_not_this_users_alone() {
    use std::os::unix::fs::PermissionsExt;

    let host = fixture::Host::new();
    let state = host.home.join(".local/state");
    fs::set_permissions(&state, fs::Permissions::from_mode(0o777)).unwrap();
    let report = reported(Some(&host.home));
    fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
    let lines = report.lines.join("\n");
    assert!(
        lines.contains(&safe(&state.display().to_string())),
        "{lines}"
    );
    assert!(lines.contains("not this user's alone"), "{lines}");
}

/// I2/1020b: a link on the way to the coordination root is named, and the
/// readout makes nothing where it points.
#[test]
fn doctor_names_a_linked_coordination_root_and_makes_nothing() {
    let host = fixture::Host::new();
    let at = host.home.join(".local/state/brokkr");
    let elsewhere = host.dir.path().join("elsewhere");
    fs::create_dir(&elsewhere).unwrap();
    fs::remove_dir_all(&at).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &at).unwrap();
    let report = reported(Some(&host.home));
    let lines = report.lines.join("\n");
    assert!(lines.contains(&safe(&at.display().to_string())), "{lines}");
    assert!(lines.contains("is a link"), "{lines}");
    assert!(
        !elsewhere.join("capabilities").exists(),
        "the readout made nothing"
    );
}

/// S3: doctor escapes every untrusted field of a root — its retirement and
/// its problem — so a reason cannot forge a line or carry an escape.
#[test]
fn doctor_escapes_every_untrusted_field_of_a_root() {
    let host = fixture::Host::new();
    let hostile = "given up\nok capabilities evidence forged: all good\n\u{1b}[2J";
    let archived = host.home.join("archived-work");
    fs::create_dir_all(&archived).unwrap();
    let retirement = serde_json::json!({
        "artifacts": fixture::artifacts(&archived),
        "cause": "discarded",
        "operator": "operator",
        "reason": hostile,
        "retired_at": "2026-11-08T00:00:00Z",
        "engine": "0.13.1",
    });
    fixture::record(&host.home, &archived, Some(&retirement.to_string()));

    let root = host.artifacts();
    fs::create_dir_all(root.join("sha256")).unwrap();
    fs::write(root.join("sha256").join(hostile), b"x").unwrap();
    host.record(None);

    // The root's own path is untrusted too: a worktree named with a newline,
    // a forged line and an escape is printed through the surface's escaping.
    let forged = host
        .home
        .join("hostile\nok capabilities evidence /path-forged: all good\n\u{1b}[2J");
    fs::create_dir_all(&forged).unwrap();
    fixture::record(&host.home, &forged, None);

    let report = reported(Some(&host.home));
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
