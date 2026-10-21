use super::*;
use crate::engine::tests::git_commit;

#[test]
fn the_four_readings_keep_the_ship_decision() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    let head = git_commit(repo, "base");
    assert_eq!(git_head(repo), Some(head));
    assert_eq!(
        (read_tree(repo), git_dirty(repo)),
        (TreeReading::Clean, false)
    );
    std::fs::write(repo.join("base.txt"), "edited").unwrap();
    assert_eq!(
        (read_tree(repo), git_dirty(repo)),
        (TreeReading::Dirty, true)
    );
    std::fs::write(repo.join(".git/index"), "not an index").unwrap();
    assert_eq!(
        (read_tree(repo), git_dirty(repo)),
        (TreeReading::Refused, false)
    );
    let missing = repo.join("missing");
    assert_eq!(
        (read_tree(&missing), git_dirty(&missing)),
        (TreeReading::Unrun, true)
    );
    assert_eq!(git_head(&missing), None);
}

#[test]
fn no_repository_empty_repository_and_ignored_edits_read_gits_answer() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(read_tree(dir.path()), TreeReading::Refused);
    assert_eq!(git_head(dir.path()), None);
    crate::realms::tests::initialised(dir.path());
    assert_eq!(read_tree(dir.path()), TreeReading::Clean);
    std::fs::write(dir.path().join(".gitignore"), "ignored\n").unwrap();
    git_commit(dir.path(), "base");
    std::fs::write(dir.path().join("ignored"), "edited").unwrap();
    assert_eq!(read_tree(dir.path()), TreeReading::Clean);
}
