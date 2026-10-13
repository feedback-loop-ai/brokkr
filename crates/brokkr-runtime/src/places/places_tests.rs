use std::path::{Path, PathBuf};

use super::{
    default_map, folder, library, refuse_folder_base, run_local, state_of, unpinned_top_level,
    Kind, Layout, Places, PlacesError,
};

/// Mark `dir` as a recipe library (decision 0083 ruling 4), so a launch
/// from it keeps the library's own `.forge/` layout: the one mark the
/// runtime's launch fixtures plant in a bare temporary workspace.
pub(crate) fn plant_library(dir: &Path) {
    let mark = dir.join("recipes/mark").join(super::BUNDLE);
    std::fs::create_dir_all(mark.parent().unwrap()).unwrap();
    std::fs::write(mark, "{}").unwrap();
}

#[test]
fn a_workspace_without_a_folder_keeps_the_library_layout() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    assert_eq!(
        Places::of(root).unwrap(),
        Places {
            agents: root.join("agents"),
            adapters: root.join("adapters"),
            map: root.join("realms.json"),
        }
    );
}

#[test]
fn a_workspace_with_a_folder_reads_it() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir(root.join(".brokkr")).unwrap();
    assert_eq!(
        Places::of(root).unwrap(),
        Places {
            agents: root.join(".brokkr/agents"),
            adapters: root.join(".brokkr/adapters"),
            map: root.join(".brokkr/realms.json"),
        }
    );
    assert_eq!(state_of(root).unwrap(), Some(root.join(".brokkr/state")));
}

/// The places are judged at the root the walk reaches (round 634i's
/// first finding): from a subdirectory of a folder repository they are
/// the root folder's, and a `.brokkr` below the root refuses in the
/// CLI's words, as the state does.
#[test]
fn places_are_the_root_folders_from_a_subdirectory() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::create_dir_all(root.join(".brokkr")).unwrap();
    let deep = root.join("src/deep");
    std::fs::create_dir_all(&deep).unwrap();
    assert_eq!(
        Places::of(&deep).unwrap(),
        Places {
            agents: root.join(".brokkr/agents"),
            adapters: root.join(".brokkr/adapters"),
            map: root.join(".brokkr/realms.json"),
        }
    );
    assert_eq!(default_map(&deep).unwrap(), None);

    std::fs::create_dir(root.join("src/.brokkr")).unwrap();
    let src = root.join("src");
    let nested = |refused: &PlacesError| matches!(refused, PlacesError::Nested { path, root: at } if *path == src && *at == root);
    assert!(nested(&state_of(&deep).unwrap_err()));
    let refused = Places::of(&deep).unwrap_err();
    assert!(nested(&refused), "{refused:?}");
    assert_eq!(
        refused.to_string(),
        format!(
            "{}/.brokkr is not at this repository's root {} (decision 0083); Brokkr reads only \
             {}/.brokkr: move or remove it",
            src.display(),
            root.display(),
            root.display()
        )
    );
}

/// A dispatch judges a workspace's state by the CLI's one rule (round
/// 634c's second finding): no folder, no library mark and no old layout
/// is no place, refused in the CLI's words rather than read as a
/// library's `.forge/`; an old layout's own entry takes the library
/// column, because a dispatch's journal is always named.
#[test]
fn a_workspace_with_no_place_refuses_its_state() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let refused = state_of(root).unwrap_err();
    assert!(
        matches!(refused, PlacesError::NoPlace { ref cwd, root: ref at } if cwd == root && at == root)
    );
    assert_eq!(
        refused.to_string(),
        format!(
            "no .brokkr/ folder from {} up to {}, and {} is not a recipe library (decision \
             0083); nothing was created: run `brokkr init .` from {}",
            root.display(),
            root.display(),
            root.display(),
            root.display()
        )
    );
    for old in [".forge", "bundle.json"] {
        std::fs::write(root.join(old), "").unwrap();
        assert_eq!(state_of(root).unwrap(), None);
        let unescaped = Kind::Bare.state(root, root, false).unwrap_err();
        assert!(matches!(unescaped, PlacesError::NoPlace { .. }));
        std::fs::remove_file(root.join(old)).unwrap();
    }
    plant_library(root);
    assert_eq!(state_of(root).unwrap(), None);

    // From below the repository's root, the walk reaches it as the CLI's
    // does, and a workspace that cannot be made absolute is unreadable.
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::create_dir_all(root.join("sub")).unwrap();
    assert_eq!(state_of(&root.join("sub")).unwrap(), None);
    let empty = state_of(Path::new("")).unwrap_err();
    assert_eq!(
        empty.to_string(),
        "reading : cannot make an empty path absolute"
    );
}

/// Whether this process reads what its mode bits forbid, as root does:
/// a permission plant does not bite then.
fn reads_anything(locked: &Path) -> bool {
    std::fs::read_dir(locked).is_ok()
}

fn mode(path: &Path, bits: u32) {
    let mode = std::os::unix::fs::PermissionsExt::from_mode(bits);
    std::fs::set_permissions(path, mode).unwrap();
}

/// A library mark that cannot be read refuses by name, never read as no
/// library (round 634c's first finding): an unlistable `recipes/`, and a
/// `bundle.json` whose presence cannot be read or that links to nothing.
#[test]
fn an_unreadable_library_mark_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (recipes, recipe) = (root.join("recipes"), root.join("recipes/x"));
    std::fs::create_dir_all(root.join(".brokkr")).unwrap();
    std::fs::create_dir_all(&recipe).unwrap();
    std::fs::write(recipe.join("bundle.json"), "{}").unwrap();
    assert_eq!(Kind::of(root).unwrap(), Kind::Library);
    mode(&recipes, 0o000);
    if reads_anything(&recipes) {
        mode(&recipes, 0o755);
        eprintln!("skipped: this process reads a mode-000 directory, as root does");
        return;
    }
    let listed = Places::of(root).map(|_| ()).unwrap_err().to_string();
    std::fs::remove_dir(root.join(".brokkr")).unwrap();
    let marked = Kind::of(root).map(|_| ()).unwrap_err();
    mode(&recipes, 0o755);
    let denied = |path: &Path| {
        format!(
            "reading {}: Permission denied (os error 13)",
            path.display()
        )
    };
    assert_eq!(listed, denied(&recipes));
    assert!(matches!(marked, PlacesError::Unreadable { ref path, .. } if *path == recipes));

    let bundle = recipe.join("bundle.json");
    mode(&recipe, 0o000);
    let looked_up = library(root).unwrap_err().to_string();
    mode(&recipe, 0o755);
    assert_eq!(looked_up, denied(&bundle));

    std::fs::remove_file(&bundle).unwrap();
    std::os::unix::fs::symlink(root.join("absent.json"), &bundle).unwrap();
    assert_eq!(
        library(root).unwrap_err().to_string(),
        format!(
            "reading {}: No such file or directory (os error 2)",
            bundle.display()
        )
    );
    std::fs::remove_file(&bundle).unwrap();
    assert!(!library(root).unwrap());
}

/// A recipe library keeps its own layout, a `.brokkr` directory beside
/// `recipes/` included (the operator's ruling 1 of 2026-10-13).
#[test]
fn a_library_holding_a_folder_keeps_the_library_layout() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join(".brokkr")).unwrap();
    std::fs::create_dir_all(root.join("recipes/x")).unwrap();
    assert_eq!(folder(root).unwrap(), Some(root.join(".brokkr")));
    std::fs::write(root.join("recipes/x/bundle.json"), "{}").unwrap();
    assert_eq!(folder(root).unwrap(), None);
    assert_eq!(state_of(root).unwrap(), None);
}

/// The entry is judged by what stands at its name: a link to a
/// directory, or a regular file, is no folder, and refuses rather than
/// falling back to the library layout (round 634b's fourth finding).
#[test]
fn a_linked_or_file_folder_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let refused = |root: &Path| match Places::of(root) {
        Err(PlacesError::NotADirectory { root }) => root,
        other => panic!("{other:?}"),
    };
    std::fs::create_dir(root.join("elsewhere")).unwrap();
    std::os::unix::fs::symlink(root.join("elsewhere"), root.join(".brokkr")).unwrap();
    assert_eq!(refused(root), root);
    std::fs::remove_file(root.join(".brokkr")).unwrap();
    std::fs::write(root.join(".brokkr"), "").unwrap();
    assert_eq!(refused(root), root);
    assert_eq!(
        folder(root).unwrap_err().to_string(),
        format!(
            "{}/.brokkr is not a directory (decision 0083); Brokkr's folder must be one: move \
             or remove it",
            root.display()
        )
    );
}

/// A `.brokkr` entry that cannot be looked up refuses with its path,
/// never read as no folder. Judged at the root itself, as `init`'s
/// [`Layout::of_root`] does, because a walk meets the loop first at its
/// `.git`.
#[test]
fn a_folder_that_cannot_be_read_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let looped = dir.path().join("loop");
    std::os::unix::fs::symlink(&looped, &looped).unwrap();
    let folder = looped.join(".brokkr");
    let unreadable = std::fs::symlink_metadata(&folder).unwrap_err();
    let refused = Layout::of_root(&looped).unwrap_err();
    assert!(matches!(refused, PlacesError::Unreadable { ref path, .. } if *path == folder));
    assert_eq!(
        refused.to_string(),
        format!("reading {}: {unreadable}", folder.display())
    );
}

/// The default map's one presence rule (round 634b's first finding):
/// nothing is no map, a file or a link to one is the map, and anything
/// else refuses with its path.
#[test]
fn the_default_map_is_a_file_or_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let map = root.join("realms.json");
    assert_eq!(default_map(root).unwrap(), None);
    std::fs::write(root.join("map.json"), "{}").unwrap();
    std::os::unix::fs::symlink(root.join("map.json"), &map).unwrap();
    assert_eq!(default_map(root).unwrap(), Some(map.clone()));
    std::fs::remove_file(&map).unwrap();
    std::fs::write(&map, "{}").unwrap();
    assert_eq!(default_map(root).unwrap(), Some(map.clone()));

    std::fs::remove_file(&map).unwrap();
    std::fs::create_dir(&map).unwrap();
    let refused = default_map(root).unwrap_err();
    assert!(matches!(refused, PlacesError::MapNotAFile { ref path } if *path == map));
    assert_eq!(
        refused.to_string(),
        format!(
            "{} is not a file (decision 0083): the default realms map is a file, or a link to \
             one; move or remove it",
            map.display()
        )
    );
    std::fs::remove_dir(&map).unwrap();
    std::os::unix::fs::symlink(root.join("absent.json"), &map).unwrap();
    let refused = default_map(root).unwrap_err();
    assert!(matches!(refused, PlacesError::MapDangling { ref path } if *path == map));
    assert_eq!(
        refused.to_string(),
        format!(
            "{} is a link to nothing (decision 0083 ruling 4: a linked entry is the entry); mend \
             or remove the link",
            map.display()
        )
    );
    std::fs::remove_file(&map).unwrap();
    std::os::unix::fs::symlink(&map, &map).unwrap();
    let refused = default_map(root).unwrap_err();
    assert!(matches!(refused, PlacesError::Unreadable { ref path, .. } if *path == map));
}

/// A workspace that is a file holds no map it can read: its place
/// refuses as unreadable, never as no map.
#[test]
fn a_map_place_that_cannot_be_read_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file");
    std::fs::write(&file, "").unwrap();
    let refused = default_map(&file).unwrap_err();
    let map = file.join("realms.json");
    let unreadable = std::fs::metadata(&map).unwrap_err();
    assert_eq!(
        refused.to_string(),
        format!("reading {}: {unreadable}", map.display())
    );
}

#[test]
fn the_run_local_directory_is_the_state_handed_else_the_workdirs_forge() {
    let workdir = Path::new("/repo");
    assert_eq!(run_local(None, workdir), PathBuf::from("/repo/.forge"));
    let state = Path::new("/root/.brokkr/state");
    assert_eq!(run_local(Some(state), workdir), state);
}

/// Only a folder bundle skips its top-level `state/`; every other bundle
/// pins it, as before decision 0083.
#[test]
fn only_a_folder_bundle_skips_its_state() {
    let (folder, library) = (Path::new("/root/.brokkr"), Path::new("/root/recipes/state"));
    assert!(unpinned_top_level(folder, "state"));
    assert!(!unpinned_top_level(library, "state"));
    assert!(unpinned_top_level(library, "dialects"));
    assert!(!unpinned_top_level(folder, "roles"));
}

#[test]
fn only_a_folder_bundle_refuses_a_base() {
    let refused = refuse_folder_base(Path::new("/root/.brokkr"), "shared").unwrap_err();
    assert_eq!(
        refused.to_string(),
        "bundle: /root/.brokkr/bundle.json extends 'shared' (decision 0083); a bundle in \
         .brokkr/ composes no base, because its library would be /root: remove extends"
    );
    assert!(matches!(refused, crate::CompileError::Invalid(_)));
    assert_eq!(
        refuse_folder_base(Path::new("/root/recipes/x"), "shared").ok(),
        Some(())
    );
}
