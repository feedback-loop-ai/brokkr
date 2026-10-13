//! What the binary's tests cannot reach of folder discovery: the entries
//! that cannot be read, doctor's fallback to the library column, and the
//! state moves' every shape. Also the library mark the unit tests of the
//! run verbs plant in a bare temporary workspace.

use std::path::{Path, PathBuf};

use super::{state_moves, DoctorDefaults, FolderError, Layout, OldLayout};

/// Mark `dir` as a recipe library (decision 0083 ruling 4), so a run verb
/// standing in it keeps the library's own `.forge/` layout.
pub(crate) fn plant_library(dir: &Path) {
    std::fs::create_dir_all(dir.join("recipes/mark")).unwrap();
    std::fs::write(dir.join("recipes/mark/bundle.json"), "{}").unwrap();
}

#[test]
fn a_workspace_that_cannot_be_made_absolute_is_unreadable() {
    let refused = Layout::discover(Path::new("")).unwrap_err();
    assert!(matches!(refused, FolderError::Unreadable { ref path, .. } if path == Path::new("")));
    assert_eq!(
        refused.to_string(),
        "reading : cannot make an empty path absolute"
    );
}

/// An entry the walk cannot read is refused, never taken as absent.
#[test]
fn an_entry_that_cannot_be_read_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let locked = dir.path().join("locked");
    std::fs::create_dir_all(locked.join("inner")).unwrap();
    let mode = |bits| std::os::unix::fs::PermissionsExt::from_mode(bits);
    std::fs::set_permissions(&locked, mode(0o000)).unwrap();
    let refused = Layout::discover(&locked.join("inner"));
    std::fs::set_permissions(&locked, mode(0o755)).unwrap();
    let refused = refused.unwrap_err();
    let git = locked.join("inner/.git");
    assert!(matches!(refused, FolderError::Unreadable { ref path, .. } if *path == git));
    assert_eq!(
        refused.to_string(),
        format!("reading {}: Permission denied (os error 13)", git.display())
    );
}

#[test]
fn doctor_reads_the_folder_or_else_reports_no_journal() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join(".brokkr")).unwrap();
    let found = DoctorDefaults::of(root.clone(), None);
    let state = root.join(".brokkr/state");
    assert_eq!(
        (
            &found.workspace,
            found.journal.as_ref().ok(),
            &found.secrets
        ),
        (
            &root,
            Some(&state.join("forge.db")),
            &state.join("secrets.env")
        )
    );
    assert_eq!(
        (&found.agents, &found.adapters),
        (&root.join(".brokkr/agents"), &root.join(".brokkr/adapters"))
    );

    // A folder below the root is a refusal discovery gives and doctor
    // never does: the journal has no place, and it says why; what doctor
    // only reads takes the library column.
    std::fs::create_dir_all(root.join("sub/.brokkr")).unwrap();
    std::fs::create_dir(root.join(".git")).unwrap();
    let flagged = Some(PathBuf::from("named.env"));
    let fallen = DoctorDefaults::of(root.join("sub/deep"), flagged);
    assert!(
        matches!(fallen.journal, Err(FolderError::Nested { ref path, .. }) if *path == root.join("sub"))
    );
    assert_eq!(
        (&fallen.workspace, &fallen.secrets),
        (&root.join("sub/deep"), &PathBuf::from("named.env"))
    );
    assert_eq!(
        (&fallen.agents, &fallen.adapters),
        (&PathBuf::from("agents"), &PathBuf::from("adapters"))
    );
}

fn git(root: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

/// Each `.forge/` entry moves: a journal with its `-wal` and `-shm`, a
/// log whose journal is gone on its own, a tracked entry by `git mv`, and
/// the ignore file out by `git rm` when git tracks it, else by `rm` and
/// an `rmdir`.
#[test]
fn the_state_moves_cover_every_entry() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-q"]);
    for name in [
        "forge.db",
        "forge.db-wal",
        "forge.db-shm",
        "old.db-wal",
        "results/r.json",
    ] {
        let path = root.join(".forge").join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "").unwrap();
    }
    std::fs::write(root.join(".forge/notes.md"), "").unwrap();
    git(root, &["add", ".forge/notes.md"]);
    assert_eq!(
        state_moves(root, true),
        [
            "mkdir -p .brokkr/state",
            "mv .forge/forge.db .brokkr/state/forge.db",
            "mv .forge/forge.db-wal .brokkr/state/forge.db-wal",
            "mv .forge/forge.db-shm .brokkr/state/forge.db-shm",
            "git mv .forge/notes.md .brokkr/state/notes.md",
            "mv .forge/old.db-wal .brokkr/state/old.db-wal",
            "mv .forge/results .brokkr/state/results",
            "rmdir .forge",
            "printf 'state/\\n' > .brokkr/.gitignore",
        ]
    );
    std::fs::write(root.join(".forge/.gitignore"), "*\n").unwrap();
    let untracked = state_moves(root, true);
    assert_eq!(untracked[7..9], ["rm .forge/.gitignore", "rmdir .forge"]);
    git(root, &["add", "-f", ".forge/.gitignore"]);
    assert_eq!(state_moves(root, true)[7], "git rm .forge/.gitignore");
}

/// A journal moves whole: its logs beside it whatever it is named, and
/// each member by its own verb, so a tracked journal's untracked log is
/// never handed to `git mv`.
#[test]
fn a_journal_moves_whole_whatever_its_name_and_tracking() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-q"]);
    std::fs::create_dir(root.join(".forge")).unwrap();
    for name in [
        "forge.db",
        "forge.db-wal",
        "j.sqlite",
        "j.sqlite-shm",
        "j.sqlite-wal",
    ] {
        std::fs::write(root.join(".forge").join(name), "").unwrap();
    }
    git(root, &["add", ".forge/forge.db"]);
    assert_eq!(
        state_moves(root, true)[1..6],
        [
            "git mv .forge/forge.db .brokkr/state/forge.db",
            "mv .forge/forge.db-wal .brokkr/state/forge.db-wal",
            "mv .forge/j.sqlite .brokkr/state/j.sqlite",
            "mv .forge/j.sqlite-wal .brokkr/state/j.sqlite-wal",
            "mv .forge/j.sqlite-shm .brokkr/state/j.sqlite-shm",
        ]
    );
}

/// A name a shell would split or quote is listed as one quoted word.
#[test]
fn a_moved_name_is_one_shell_word() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir(root.join(".forge")).unwrap();
    for name in ["run history.sqlite", "it's"] {
        std::fs::write(root.join(".forge").join(name), "").unwrap();
    }
    assert_eq!(
        state_moves(root, true)[1..3],
        [
            r"mv '.forge/it'\''s' '.brokkr/state/it'\''s'",
            "mv '.forge/run history.sqlite' '.brokkr/state/run history.sqlite'",
        ]
    );
}

/// The public defaults are a library's journal and secrets store, copies
/// held to the names the folder's state uses (decision 0071 ruling 5).
#[test]
fn the_public_defaults_are_the_library_state_files() {
    let library = Path::new(super::LIBRARY_STATE);
    assert_eq!(Path::new(crate::DEFAULT_DB), library.join(super::JOURNAL));
    assert_eq!(
        Path::new(crate::DEFAULT_SECRETS),
        library.join(super::SECRETS)
    );
}

/// `--db` lends a bare repository's state the library column only where
/// the root holds an old layout's own entry: `.forge` alone is one, and
/// nothing at all is none.
#[test]
fn the_db_escape_needs_an_old_layout() {
    let dir = tempfile::tempdir().unwrap();
    let bare = Layout::discover(dir.path()).unwrap();
    assert!(matches!(bare.state(true), Err(FolderError::NoPlace { .. })));
    std::fs::create_dir(dir.path().join(".forge")).unwrap();
    assert_eq!(bare.state(true).unwrap(), None);
    assert!(matches!(
        bare.state(false),
        Err(FolderError::NoPlace { .. })
    ));
}

/// A root layout with no `.forge` at all lists no state to move.
#[test]
fn a_root_layout_without_state_moves_no_state() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("bundle.json"), "{}").unwrap();
    let layout = Layout::discover(dir.path()).unwrap();
    let Some(OldLayout::Root { moves }) = layout.old().unwrap() else {
        panic!("the root layout");
    };
    assert_eq!(
        moves,
        [
            "mkdir -p .brokkr/state",
            "printf 'state/\\n' > .brokkr/.gitignore"
        ]
    );
    assert_eq!(layout.state(true).unwrap(), None);
    assert_eq!(layout.agents(None), PathBuf::from("agents"));
}
