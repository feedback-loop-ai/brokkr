use std::path::{Path, PathBuf};

use super::{
    default_map, folder, refuse_folder_base, run_local, unpinned_top_level, Places, PlacesError,
};

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
            state: None,
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
            state: Some(root.join(".brokkr/state")),
        }
    );
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
    assert_eq!(Places::of(root).unwrap().state, None);
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
/// never read as no folder.
#[test]
fn a_folder_that_cannot_be_read_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let looped = dir.path().join("loop");
    std::os::unix::fs::symlink(&looped, &looped).unwrap();
    let folder = looped.join(".brokkr");
    let unreadable = std::fs::symlink_metadata(&folder).unwrap_err();
    let refused = Places::of(&looped).unwrap_err();
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
