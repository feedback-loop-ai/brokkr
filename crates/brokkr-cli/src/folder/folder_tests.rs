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
    let named = |refused: &PlacesError| matches!(refused, PlacesError::Unreadable { path, .. } if path == Path::new(""));
    assert!(matches!(refused, FolderError::Places(ref refused) if named(refused)));
    assert_eq!(
        refused.to_string(),
        "reading : cannot make an empty path absolute"
    );
}

/// An entry the walk cannot read is refused, never taken as absent. The
/// workspace itself is locked, not its parent, so it still resolves to
/// the real directory the walk begins in (round 634j).
#[test]
fn an_entry_that_cannot_be_read_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let locked = dir.path().canonicalize().unwrap().join("locked");
    std::fs::create_dir_all(locked.join("inner")).unwrap();
    let mode = |bits| std::os::unix::fs::PermissionsExt::from_mode(bits);
    std::fs::set_permissions(locked.join("inner"), mode(0o000)).unwrap();
    let refused = Layout::discover(&locked.join("inner"));
    std::fs::set_permissions(locked.join("inner"), mode(0o755)).unwrap();
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
    let root = dir.path().canonicalize().unwrap();
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
    std::fs::create_dir_all(root.join("sub/deep")).unwrap();
    std::fs::create_dir(root.join(".git")).unwrap();
    let flagged = Some(PathBuf::from("named.env"));
    let fallen = DoctorDefaults::of(root.join("sub/deep"), flagged);
    assert!(
        matches!(fallen.journal, Err(FolderError::Places(PlacesError::Nested { ref path, .. })) if *path == root.join("sub"))
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

/// Commit what `root`'s index holds, so a tracked `.forge/` entry is
/// clean and the moves list it (round 634k's third finding).
fn commit(root: &Path) {
    let author = ["-c", "user.name=t", "-c", "user.email=t@example.invalid"];
    let commit = ["-c", "commit.gpgsign=false", "commit", "-q", "-m", "x"];
    git(root, &[&author[..], &commit[..]].concat());
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
    commit(root);
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
    commit(root);
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
    commit(root);
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

/// A `.forge/` entry whose name is not UTF-8 refuses by name, with no
/// moves, never listed under a name no shell could reach (round 634i's
/// second finding). Once git tracks it, an untracked name holding U+FFFD
/// in its place is never taken for it: the tracked one gone from the work
/// tree is a change git holds, and the moves are refused, naming it as
/// the refusal can (round 634k's third finding).
#[test]
fn a_name_that_is_not_utf8_refuses_and_tracking_matches_bytes() {
    use std::os::unix::ffi::OsStrExt;
    let dir = repository_with_a_forge();
    let root = &dir.path().canonicalize().unwrap();
    let odd = root
        .join(".forge")
        .join(std::ffi::OsStr::from_bytes(b"journal-\xff.db"));
    if let Err(refused) = std::fs::write(&odd, "journal") {
        eprintln!("skipped: this filesystem refuses a name that is not UTF-8: {refused}");
        return;
    }
    let refusal = refused(root);
    assert!(matches!(refusal, FolderError::NotUtf8 { ref path } if *path == odd));
    assert_eq!(
        refusal.to_string(),
        format!(
            "{}/.forge/journal-\u{FFFD}.db is not named in UTF-8, so no listed move could name \
             it (decision 0083); nothing was opened: rename it",
            root.display()
        )
    );

    git(root, &["add", ".forge"]);
    commit(root);
    std::fs::remove_file(&odd).unwrap();
    std::fs::write(root.join(".forge/journal-\u{FFFD}.db"), "journal").unwrap();
    let refusal = refused(root);
    assert!(matches!(refusal, FolderError::Uncommitted { ref changes, .. } if changes.len() == 1));
    assert_eq!(
        refusal.to_string().lines().last(),
        Some("   D .forge/journal-\u{FFFD}.db")
    );
}

/// A `.forge/` entry whose name holds a control character is valid UTF-8
/// but no move line could name it on one display line, so it refuses by
/// name, escaped, with no moves (round 634j's third finding).
#[test]
fn a_name_with_a_control_character_refuses() {
    let dir = repository_with_a_forge();
    let root = dir.path().canonicalize().unwrap();
    for (name, escaped) in [
        ("a\nb.db", r"a\nb.db"),
        ("a\tb", r"a\tb"),
        ("a\rb", r"a\rb"),
    ] {
        let odd = root.join(".forge").join(name);
        std::fs::write(&odd, "journal").unwrap();
        let refusal = refused(&root);
        assert!(matches!(refusal, FolderError::ControlCharacter { ref path } if *path == odd));
        assert_eq!(
            refusal.to_string(),
            format!(
                "{}/.forge/{escaped} holds a control character, so no listed move could name it \
                 on one line (decision 0083); nothing was opened: rename it",
                root.display()
            )
        );
        std::fs::remove_file(&odd).unwrap();
    }
}

/// The old ignore entry is judged as the entry itself (round 634j's second
/// finding): a link, even to nothing, goes by `rm`; a directory, which no
/// `rm` removes, refuses by name with no moves.
#[test]
fn an_ignore_entry_that_is_not_a_file_or_a_link_refuses() {
    let dir = repository_with_a_forge();
    let root = dir.path().canonicalize().unwrap();
    let ignore = root.join(".forge/.gitignore");
    std::os::unix::fs::symlink(root.join("absent"), &ignore).unwrap();
    assert_eq!(
        state_moves(&root).unwrap()[1..3],
        ["rm .forge/.gitignore", "rmdir .forge"]
    );
    std::fs::remove_file(&ignore).unwrap();
    std::fs::create_dir_all(ignore.join("keep")).unwrap();
    let refusal = refused(&root);
    assert!(matches!(refusal, FolderError::IgnoreNotAFile { ref path } if *path == ignore));
    assert_eq!(
        refusal.to_string(),
        format!(
            "{} is neither a file nor a link, so no listed move could remove it (decision 0083); \
             nothing was opened: move it aside",
            ignore.display()
        )
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

/// A library's journal, which a command there opens when neither a map
/// nor `--db` names one.
pub(crate) fn library_journal() -> PathBuf {
    Path::new(super::LIBRARY_STATE).join(super::JOURNAL)
}

/// A library's journal and secrets store are the names the folder's
/// state uses, in its own `.forge/`, as they always were (the public
/// copies of them retired in round 634i).
#[test]
fn the_public_defaults_are_the_library_state_files() {
    assert_eq!(library_journal(), Path::new(".forge/forge.db"));
    assert_eq!(super::in_state(None, super::JOURNAL), library_journal());
    assert_eq!(
        super::in_state(None, super::SECRETS),
        Path::new(".forge/secrets.env")
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
    let forge = dir.path().canonicalize().unwrap().join(".forge");
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
    let forge = dir.path().canonicalize().unwrap().join(".forge");
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

/// A linked `.forge` whose ignore file git tracks refuses by name, with no
/// moves: git refuses some paths beyond a symbolic link, so no move
/// through one is listed (round 634k's third finding; its review found
/// git 2.51 completes this shape's own list, so the reason names what git
/// refuses only in part).
#[test]
fn a_linked_forge_whose_entries_git_tracks_refuses() {
    let dir = repository_with_a_forge();
    let root = &dir.path().canonicalize().unwrap();
    std::fs::write(root.join(".forge/.gitignore"), "*\n").unwrap();
    git(root, &["add", "-f", ".forge/.gitignore"]);
    commit(root);
    std::fs::rename(root.join(".forge"), root.join("elsewhere")).unwrap();
    std::os::unix::fs::symlink(root.join("elsewhere"), root.join(".forge")).unwrap();
    let refusal = refused(root);
    let forge = root.join(".forge");
    assert!(matches!(refusal, FolderError::LinkedTracked { ref path } if *path == forge));
    assert_eq!(
        refusal.to_string(),
        format!(
            "{} is a link and git tracks entries under it; git refuses some paths beyond a \
             symbolic link, so no moves are listed (decision 0083); nothing was opened: put the \
             directory it names in its place, or untrack its entries, then run again",
            forge.display()
        )
    );
}

/// Each record of `git status --porcelain=v1 -z` is read typed: two
/// state letters git documents for a tracked path, a space and a path, a
/// rename's origin after it. Anything else refuses with its bytes, never
/// skipped (round 634k's third finding).
#[test]
fn a_status_record_this_cannot_read_refuses() {
    let root = Path::new("/r");
    let read = |status: &[u8]| super::tracked::changes(root, status);
    let status = b"A  .forge/a\0 M .forge/b c\0R  .forge/d\0.forge/e\0MM .forge/\xff\n\0";
    let listed = read(status).unwrap();
    let lines: Vec<String> = listed.iter().map(ToString::to_string).collect();
    assert_eq!(
        lines,
        [
            "A  .forge/a",
            " M .forge/b c",
            "R  .forge/d",
            "MM .forge/\u{FFFD}\\n"
        ]
    );
    assert_eq!(read(b"").unwrap(), []);
    for (status, record) in [
        (&b"?? .forge/x\0"[..], "?? .forge/x"),
        (b"A? .forge/x\0", "A? .forge/x"),
        (b"AM.forge/x\0", "AM.forge/x"),
        (b"A  \0", "A  "),
        (b"A\0", "A"),
        (b"R  .forge/d\0", "R  .forge/d"),
        (b"\xff\n .forge/x\0", "\u{FFFD}\\n .forge/x"),
    ] {
        let refusal = read(status).unwrap_err();
        let said = format!("it printed a record this cannot read: '{record}'");
        assert!(
            matches!(refusal, FolderError::Untracked { verb: "status", ref said, .. } if said.contains("cannot read")),
            "{record}"
        );
        assert_eq!(
            refusal.to_string(),
            format!(
                "git status in /r failed, so the moves cannot tell what git tracks (decision \
                 0083); nothing was opened: {said}"
            ),
            "{record}"
        );
    }
}

/// Each record of `git ls-files -s -v -z` is read typed: a file's or a
/// link's mode names the entry under `.forge/` it stands in, a path that
/// is `.forge` itself names none, a submodule refuses by its path, and a
/// record with no tab, an unknown mode or an unknown tag refuses with its
/// bytes (round 634k's review).
#[test]
fn an_index_record_this_cannot_read_refuses() {
    let root = Path::new("/r");
    let read = |listed: &[u8]| super::tracked::held(root, listed);
    let object = "0".repeat(40);
    let listed = format!(
        "H 100644 {object} 0\t.forge/a\0H 100755 {object} 0\t.forge/r/x.json\0M 120000 {object} \
         1\t.forge\0"
    );
    assert_eq!(
        read(listed.as_bytes()).unwrap(),
        [b"a".to_vec(), b"r".to_vec()]
    );
    let submodule = format!("H 100644 {object} 0\t.forge/a\0H 160000 {object} 0\t.forge/z\0");
    let refusal = read(submodule.as_bytes()).unwrap_err();
    assert!(
        matches!(refusal, FolderError::TrackedSubmodule { ref path } if path == Path::new("/r/.forge/z"))
    );
    assert_eq!(
        refusal.to_string(),
        "/r/.forge/z is a submodule git tracks, which `git mv` moves only together with \
         .gitmodules, so no moves are listed (decision 0083); nothing was opened: move it out of \
         .forge and commit, then run again"
    );
    for record in [
        "H 100644 x 0 .forge/a",
        "H 040000 x 0\t.forge/d",
        "100644 x 0\t.forge/a",
        "C 100644 x 0\t.forge/a",
        "H100644 x 0\t.forge/a",
    ] {
        let refusal = read(format!("{record}\0").as_bytes()).unwrap_err();
        assert!(matches!(
            refusal,
            FolderError::Untracked {
                verb: "ls-files",
                ..
            }
        ));
        assert_eq!(
            refusal.to_string(),
            format!(
                "git ls-files in /r failed, so the moves cannot tell what git tracks (decision \
                 0083); nothing was opened: it printed a record this cannot read: '{}'",
                record.escape_debug()
            )
        );
    }
}

/// Every tracked path git does not read in the work tree refuses by name
/// with its tag, whatever its depth under `.forge`: `S`, as a sparse
/// checkout sets it, and each lowercase, assume-unchanged tag. `git
/// status` states no change to one, and `git rm` refuses one a sparse
/// checkout leaves out (round 634k's chief).
#[test]
fn a_path_git_does_not_read_in_the_work_tree_refuses() {
    let root = Path::new("/r");
    let object = "0".repeat(40);
    let listed = format!(
        "H 100644 {object} 0\t.forge/a\0S 100644 {object} 0\t.forge/.gitignore\0h 100644 \
         {object} 0\t.forge/r/x\ny\0s 120000 {object} 0\t.forge/l\0m 100644 {object} 2\t.forge/u\0"
    );
    let refusal = super::tracked::held(root, listed.as_bytes()).unwrap_err();
    let lines = "\n  S .forge/.gitignore\n  h .forge/r/x\\ny\n  s .forge/l\n  m .forge/u";
    assert!(
        matches!(refusal, FolderError::Unwatched { root: ref at, ref paths } if at == root && paths.len() == 4)
    );
    assert_eq!(
        refusal.to_string(),
        format!(
            "/r/.forge holds paths git tracks but does not read in the work tree, so `git status` \
             cannot say whether `git mv` or `git rm` would complete, and `git rm` refuses a path a \
             sparse checkout leaves out; no moves are listed (decision 0083); nothing was opened: \
             run `git sparse-checkout disable`, or `git update-index --no-skip-worktree \
             --no-assume-unchanged` on each, then run again{lines}"
        )
    );
}

/// A repository git lists but cannot state, its HEAD naming a commit it
/// does not hold, refuses as one git will not list does. A git that
/// states such a repository after all skips the case.
#[test]
fn a_repository_git_cannot_state_refuses_the_moves() {
    let dir = repository_with_a_forge();
    let root = &dir.path().canonicalize().unwrap();
    std::fs::write(root.join(".forge/notes.md"), "").unwrap();
    git(root, &["add", ".forge/notes.md"]);
    commit(root);
    let head = std::fs::read_to_string(root.join(".git/HEAD")).unwrap();
    let branch = head.trim().strip_prefix("ref: ").unwrap();
    std::fs::write(
        root.join(".git").join(branch),
        format!("{}\n", "1".repeat(40)),
    )
    .unwrap();
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=no",
            "--",
            ".forge",
        ])
        .output()
        .unwrap();
    if out.status.success() {
        eprintln!("skipped: this git states a repository whose HEAD it does not hold");
        return;
    }
    let said = String::from_utf8_lossy(&out.stderr).trim_end().to_string();
    let refusal = refused(root);
    assert!(
        matches!(refusal, FolderError::Untracked { verb: "status", root: ref at, said: ref git } if at == root && *git == said)
    );
    assert_eq!(
        refusal.to_string(),
        format!(
            "git status in {} failed, so the moves cannot tell what git tracks (decision 0083); \
             nothing was opened: {said}",
            root.display()
        )
    );
}

/// A repository git will not list refuses by name, never read as one
/// that tracks nothing, so no tracked entry is listed under `mv`.
#[test]
fn a_repository_git_cannot_list_refuses_the_moves() {
    let dir = tempfile::tempdir().unwrap();
    let root = &dir.path().canonicalize().unwrap();
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
        matches!(refusal, FolderError::Untracked { verb: "ls-files", root: ref at, said: ref git } if at == root && *git == said)
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
