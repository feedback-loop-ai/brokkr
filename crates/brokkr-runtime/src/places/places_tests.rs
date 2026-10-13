use std::path::{Path, PathBuf};

use super::{folder, refuse_folder_base, run_local, Places};

#[test]
fn a_workspace_without_a_folder_keeps_the_library_layout() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    assert_eq!(
        Places::of(root),
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
        Places::of(root),
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
    assert_eq!(folder(root), Some(root.join(".brokkr")));
    std::fs::write(root.join("recipes/x/bundle.json"), "{}").unwrap();
    assert_eq!(folder(root), None);
    assert_eq!(Places::of(root).state, None);
}

/// The entry is judged by what stands at its name: a link to a
/// directory, or a regular file, is no folder.
#[test]
fn a_linked_or_file_folder_is_no_folder() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir(root.join("elsewhere")).unwrap();
    std::os::unix::fs::symlink(root.join("elsewhere"), root.join(".brokkr")).unwrap();
    assert_eq!(folder(root), None);
    std::fs::remove_file(root.join(".brokkr")).unwrap();
    std::fs::write(root.join(".brokkr"), "").unwrap();
    assert_eq!(folder(root), None);
}

#[test]
fn the_run_local_directory_is_the_state_handed_else_the_workdirs_forge() {
    let workdir = Path::new("/repo");
    assert_eq!(run_local(None, workdir), PathBuf::from("/repo/.forge"));
    let state = Path::new("/root/.brokkr/state");
    assert_eq!(run_local(Some(state), workdir), state);
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
