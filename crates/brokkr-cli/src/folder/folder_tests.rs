//! What the binary's tests cannot reach of folder discovery: the entries
//! that cannot be read, doctor's fallback to the library column, and the
//! state moves' every shape. Also the library mark the unit tests of the
//! run verbs plant in a bare temporary workspace.

use std::path::{Path, PathBuf};

use brokkr_runtime::places::PlacesError;

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
    let named = |refused: &PlacesError| matches!(refused, PlacesError::Unreadable { path, .. } if *path == git);
    assert!(matches!(refused, FolderError::Places(ref refused) if named(refused)));
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
        state_moves(root).unwrap(),
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
    let untracked = state_moves(root).unwrap();
    assert_eq!(untracked[7..9], ["rm .forge/.gitignore", "rmdir .forge"]);
    git(root, &["add", "-f", ".forge/.gitignore"]);
    assert_eq!(state_moves(root).unwrap()[7], "git rm .forge/.gitignore");
}

/// A fresh repository holding an empty `.forge/`.
fn repository_with_a_forge() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    std::fs::create_dir(dir.path().join(".forge")).unwrap();
    dir
}

/// A journal moves whole: its logs beside it whatever it is named, and
/// each member by its own verb, so a tracked journal's untracked log is
/// never handed to `git mv`.
#[test]
fn a_journal_moves_whole_whatever_its_name_and_tracking() {
    let dir = repository_with_a_forge();
    let root = dir.path();
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
        state_moves(root).unwrap()[1..6],
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
        state_moves(root).unwrap()[1..3],
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
    let no_place = |state| matches!(state, Err(FolderError::Places(PlacesError::NoPlace { .. })));
    assert!(no_place(bare.state(true)));
    std::fs::create_dir(dir.path().join(".forge")).unwrap();
    assert_eq!(bare.state(true).unwrap(), None);
    assert!(no_place(bare.state(false)));
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

/// The refusal an old layout's `.forge` gives in place of its moves.
fn refused(root: &Path) -> FolderError {
    Layout::discover(root).unwrap().refuse_old().unwrap_err()
}

/// A `.forge` whose entries cannot be listed refuses by name, with no
/// moves, never read as empty (round 634c's third finding).
#[test]
fn a_forge_that_cannot_be_listed_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let forge = dir.path().join(".forge");
    std::fs::create_dir(&forge).unwrap();
    std::fs::write(forge.join("forge.db"), "journal").unwrap();
    let mode = |bits| std::os::unix::fs::PermissionsExt::from_mode(bits);
    std::fs::set_permissions(&forge, mode(0o000)).unwrap();
    if std::fs::read_dir(&forge).is_ok() {
        std::fs::set_permissions(&forge, mode(0o755)).unwrap();
        eprintln!("skipped: this process reads a mode-000 directory, as root does");
        return;
    }
    let refusal = refused(dir.path());
    std::fs::set_permissions(&forge, mode(0o755)).unwrap();
    assert!(matches!(refusal, FolderError::Unreadable { ref path, .. } if *path == forge));
    assert_eq!(
        refusal.to_string(),
        format!(
            "reading {}: Permission denied (os error 13)",
            forge.display()
        )
    );
}

/// A `.forge` that is neither a directory nor a link refuses by name, with
/// no moves: no `rmdir` could remove it (round 634c's fourth finding).
#[test]
fn a_forge_that_is_a_file_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let forge = dir.path().join(".forge");
    std::fs::write(&forge, "").unwrap();
    for shape in ["state-only", "root"] {
        let refusal = refused(dir.path());
        assert!(
            matches!(refusal, FolderError::ForgeNotADirectory { ref path } if *path == forge),
            "{shape}"
        );
        assert_eq!(
            refusal.to_string(),
            format!(
                "{} is a file, not Brokkr's state directory: move it aside",
                forge.display()
            )
        );
        std::fs::write(dir.path().join("bundle.json"), "{}").unwrap();
    }
}

/// A linked `.forge` whose ignore file git tracks is still removed as the
/// link it is, after `git rm` (round 634c's fifth finding).
#[test]
fn a_linked_forge_with_a_tracked_ignore_file_is_removed_as_a_link() {
    let dir = repository_with_a_forge();
    let root = dir.path();
    std::fs::write(root.join(".forge/.gitignore"), "*\n").unwrap();
    git(root, &["add", "-f", ".forge/.gitignore"]);
    std::fs::rename(root.join(".forge"), root.join("elsewhere")).unwrap();
    std::os::unix::fs::symlink(root.join("elsewhere"), root.join(".forge")).unwrap();
    assert_eq!(
        state_moves(root).unwrap(),
        [
            "mkdir -p .brokkr/state",
            "git rm .forge/.gitignore",
            "# .forge is a link: rm removes the link, and the directory it names stays",
            "rm .forge",
            "printf 'state/\\n' > .brokkr/.gitignore",
        ]
    );
}

/// A repository git will not list refuses by name, never read as one
/// that tracks nothing, so no tracked entry is listed under `mv`.
#[test]
fn a_repository_git_cannot_list_refuses_the_moves() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join(".git"), "not a gitfile\n").unwrap();
    std::fs::create_dir(root.join(".forge")).unwrap();
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", ".forge"])
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stderr).trim_end().to_string();
    assert!(!out.status.success() && !said.is_empty(), "{out:?}");
    let refusal = refused(root);
    assert!(
        matches!(refusal, FolderError::Untracked { root: ref at, said: ref git } if at == root && *git == said)
    );
    assert_eq!(
        refusal.to_string(),
        format!(
            "git ls-files in {} failed, so the moves cannot tell what git tracks (decision 0083); \
             nothing was opened: {said}",
            root.display()
        )
    );
}
