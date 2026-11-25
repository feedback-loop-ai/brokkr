use std::os::unix::fs::symlink;

use super::*;
use crate::verbs::setup::host::tests::V4;

#[test]
fn init_leaves_a_host_file_created_during_publication_and_reports_write_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("brokkr/host.json");
    let draft = Draft::new(["exec"]);
    let notes = seed_with(&path, &draft, |staged, target| {
        fs::write(target, V4).unwrap();
        publish(staged, target)
    })
    .unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), V4);
    assert_eq!(
        notes,
        [format!(
            "Left existing host file {} as it was. Run `brokkr system host upgrade` to upgrade it.",
            path.display()
        )]
    );

    let denied = path.with_file_name("denied");
    let error = seed_with(&denied, &draft, |_, _| {
        Err(io::ErrorKind::PermissionDenied.into())
    })
    .unwrap_err();
    assert!(
        matches!(error, UpgradeError::Write(at, cause) if at == denied && cause.kind() == io::ErrorKind::PermissionDenied)
    );
    let unreachable = path.join("unreachable");
    let error = seed_with(&unreachable, &draft, publish).unwrap_err();
    assert!(
        matches!(error, UpgradeError::Read(at, cause) if at == unreachable && cause.kind() == io::ErrorKind::NotADirectory)
    );

    let link = path.with_file_name("link");
    symlink(&denied, &link).unwrap();
    let notes = seed_with(&link, &draft, publish).unwrap();
    assert_eq!(
        notes,
        [format!(
            "Left existing host file {} as it was.",
            link.display()
        )]
    );
    assert_eq!(fs::read_link(&link).unwrap(), denied);
    assert_eq!(
        fs::symlink_metadata(&denied).unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
}
