use std::fs;
use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};

use super::*;

const V4: &str = r#"{
  "schema" : "forge.host/v4",
  "providers": {"exec": {"ceiling": 2e0}},
  "floors": [{"place": "temp", "floor_bytes": 1}],
  "boxed_builds": {"ceiling": 2}, "runs": {"ceiling": 3}
}
"#;

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let host = dir.path().join("host.json");
    fs::write(&host, V4).unwrap();
    (dir, host)
}

fn run_at(path: &Path, check: bool) -> (Exit, String) {
    let mut out = Vec::new();
    let code = upgrade_with(path, check, &mut out, persist).unwrap();
    (code, String::from_utf8(out).unwrap())
}

#[test]
fn host_check_previews_then_upgrade_keeps_bytes_mode_and_the_current_inode() {
    let (_dir, path) = fixture();
    fs::set_permissions(&path, Permissions::from_mode(0o640)).unwrap();
    let original = fs::metadata(&path).unwrap();
    let shown = path.display();
    let diff = format!(
        "--- {shown}\n+++ {shown}\n@@ -2 +2 @@\n-  \"schema\" : \"forge.host/v4\",\n+  \"schema\" : \"forge.host/v5\",\n"
    );
    assert_eq!(run_at(&path, true), (Exit::Failed, diff.clone()));
    assert_eq!(fs::read_to_string(&path).unwrap(), V4);
    assert_eq!(fs::metadata(&path).unwrap().ino(), original.ino());
    assert_eq!(run_at(&path, false), (Exit::Completed, diff));
    let changed = fs::metadata(&path).unwrap();
    assert_eq!(changed.mode() & 0o7777, 0o640);
    assert_ne!(changed.ino(), original.ino());
    let expected = V4.replace("forge.host/v4", "forge.host/v5");
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    for check in [true, false] {
        assert_eq!(
            run_at(&path, check),
            (
                Exit::Completed,
                format!("{shown} is current (forge.host/v5)\n")
            )
        );
        assert_eq!(fs::metadata(&path).unwrap().ino(), changed.ino());
        assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    }
    let mut diff = Vec::new();
    diff_line(&mut diff, '-', "no newline").unwrap();
    assert_eq!(diff, b"-no newline\n\\ No newline at end of file\n");
}

#[test]
fn host_upgrade_refuses_bad_input_and_never_follows_a_link() {
    let (dir, path) = fixture();
    let fail = |path: &Path| upgrade_with(path, false, &mut io::sink(), persist).unwrap_err();
    let missing = dir.path().join("missing");
    let missing_error = fail(&missing);
    assert_eq!(
        missing_error.to_string(),
        format!(
            "no host configuration at {}; `brokkr init` creates one",
            missing.display()
        )
    );
    assert!(matches!(missing_error, UpgradeError::Missing(at) if at == missing));
    assert!(
        matches!(fail(dir.path()), UpgradeError::Read(at, error) if at == dir.path() && error.kind() == io::ErrorKind::Other)
    );
    let link = dir.path().join("link.json");
    symlink(&path, &link).unwrap();
    assert!(matches!(fail(&link), UpgradeError::Link(at) if at == link));
    assert_eq!(fs::read_link(&link).unwrap(), path);
    assert_eq!(fs::read_to_string(&path).unwrap(), V4);

    let malformed = V4.replace("\"ceiling\": 3", "\"ceiling\": 0");
    fs::write(&path, &malformed).unwrap();
    assert!(
        matches!(fail(&path), UpgradeError::Invalid(HostError::Invalid { path: at, .. }) if at == path)
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), malformed);
}

#[test]
fn host_upgrade_with_a_failed_rename_leaves_the_original_and_removes_the_temp_file() {
    let (dir, path) = fixture();
    let error = upgrade_with(&path, false, &mut io::sink(), |staged, target| {
        assert_eq!(target, path);
        assert_eq!(staged.path().parent(), path.parent());
        assert_eq!(
            fs::read_to_string(staged.path()).unwrap(),
            V4.replace("forge.host/v4", "forge.host/v5")
        );
        persist(staged, &dir.path().join("missing/host.json"))
    })
    .unwrap_err();
    assert!(error.to_string().starts_with(&format!(
        "cannot write the host configuration at {}: ",
        path.display()
    )));
    assert!(
        matches!(error, UpgradeError::Write(at, source) if at == path && source.kind() == io::ErrorKind::NotFound)
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), V4);
    let remaining: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(remaining, [path]);
}
