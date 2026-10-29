use std::path::{Path, PathBuf};

/// A real git repository at `dir`, for a fixture that marks a workspace
/// root with `.git`: the walk asks git only when it can answer, so a bare
/// marker directory is no longer the same fixture (#908).
pub(crate) fn test_repository(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let out = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .output()
        .expect("git init");
    assert!(
        out.status.success(),
        "git init -q at {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

use super::{
    base_library, default_map, folder, library, run_local, state_of, unpinned_reason, Kind, Layout,
    Places, PlacesError,
};

/// [`unpinned_reason`] for a fixture whose git questions are answered:
/// these tests plant no repository, so git says the paths are pinned.
fn skips(dir: &Path, name: &str) -> bool {
    unpinned_reason(dir, Path::new(name)).unwrap().is_some()
}

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
            recipes: root.join("recipes"),
            map: root.join("realms.json"),
        }
    );
}

#[test]
fn a_workspace_with_a_folder_reads_it() {
    let dir = tempfile::tempdir().unwrap();
    let root = &dir.path().canonicalize().unwrap();
    std::fs::create_dir(root.join(".brokkr")).unwrap();
    assert_eq!(Places::of(root).unwrap(), folder_places(root));
    assert_eq!(state_of(root).unwrap(), Some(root.join(".brokkr/state")));
}

/// The places of the folder at `root`.
fn folder_places(root: &Path) -> Places {
    Places {
        agents: root.join(".brokkr/agents"),
        adapters: root.join(".brokkr/adapters"),
        recipes: root.join(".brokkr/recipes"),
        map: root.join(".brokkr/realms.json"),
    }
}

/// The places are judged at the root the walk reaches (round 634i's
/// first finding): from a subdirectory of a folder repository they are
/// the root folder's, and a `.brokkr` below the root refuses in the
/// CLI's words, as the state does.
#[test]
fn places_are_the_root_folders_from_a_subdirectory() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    test_repository(&root);
    std::fs::create_dir_all(root.join(".brokkr")).unwrap();
    let deep = root.join("src/deep");
    std::fs::create_dir_all(&deep).unwrap();
    assert_eq!(Places::of(&deep).unwrap(), folder_places(&root));
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

/// The walk is over the real directory's ancestors (round 634j's first
/// finding): a workspace reached through a link is judged where it
/// stands, so a `.brokkr` above the real directory refuses though the
/// link's own name has none above it; and a workspace that cannot be
/// resolved refuses by name.
#[test]
fn a_linked_workspace_is_walked_from_its_real_directory() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    test_repository(&root);
    plant_library(&root);
    let (real, link) = (root.join("other/deep"), root.join("src"));
    std::fs::create_dir_all(&real).unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let layout = Layout::discover(&link).unwrap();
    assert_eq!(
        (layout.cwd(), layout.root()),
        (real.as_path(), root.as_path())
    );
    assert_eq!(layout.workspace(), link);

    std::fs::create_dir(root.join("other/.brokkr")).unwrap();
    let refused = Layout::discover(&link).unwrap_err();
    let other = root.join("other");
    assert!(
        matches!(refused, PlacesError::Nested { ref path, root: ref at } if *path == other && *at == root),
        "{refused:?}"
    );
    assert_eq!(
        refused.to_string(),
        format!(
            "{}/.brokkr is not at this repository's root {} (decision 0083); Brokkr reads only \
             {}/.brokkr: move or remove it",
            other.display(),
            root.display(),
            root.display()
        )
    );

    let gone = root.join("gone");
    let refused = Layout::discover(&gone).unwrap_err();
    assert!(matches!(refused, PlacesError::Unreadable { ref path, .. } if *path == gone));
    assert_eq!(
        refused.to_string(),
        format!(
            "reading {}: No such file or directory (os error 2)",
            gone.display()
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
    let root = &dir.path().canonicalize().unwrap();
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
    test_repository(root);
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
    let root = &dir.path().canonicalize().unwrap();
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
    let real = dir.path().canonicalize().unwrap();
    let root = real.as_path();
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
/// never read as no folder. Judged at the root itself, with no walk,
/// because a walk meets the loop first at its `.git`.
#[test]
fn a_folder_that_cannot_be_read_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let looped = dir.path().join("loop");
    std::os::unix::fs::symlink(&looped, &looped).unwrap();
    let folder = looped.join(".brokkr");
    let unreadable = std::fs::symlink_metadata(&folder).unwrap_err();
    let refused = Layout::at(looped.clone(), looped.clone(), &looped).unwrap_err();
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

/// Every spelling of one entry is judged at the entry itself (#697): a
/// link to nothing spelled with a trailing separator, or a trailing `.`,
/// fails `NotFound` as the bare spelling does, never read as nothing
/// there, and a file so spelled resolves to the file.
#[test]
fn an_entry_is_resolved_whatever_its_spelling() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let (link, file) = (root.join("link"), root.join("file"));
    std::os::unix::fs::symlink("gone", &link).unwrap();
    std::fs::write(&file, "").unwrap();
    let spellings = |at: &Path| {
        let at = at.display();
        [
            format!("{at}"),
            format!("{at}/"),
            format!("{at}/."),
            format!("{at}//"),
        ]
    };
    for spelled in spellings(&link) {
        let unread = super::resolved(Path::new(&spelled)).unwrap_err();
        assert_eq!(unread.kind(), std::io::ErrorKind::NotFound, "{spelled}");
    }
    for spelled in spellings(&file) {
        let real = super::resolved(Path::new(&spelled)).unwrap();
        assert_eq!(real.as_deref().map(Path::as_os_str), Some(file.as_os_str()));
    }
    let absent = super::resolved(&root.join("absent/")).unwrap();
    assert_eq!(absent, None);
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

/// A repository root holding the folder, canonical, and the directory
/// that keeps it.
fn folder_root() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    test_repository(&root);
    std::fs::create_dir(root.join(".brokkr")).unwrap();
    (dir, root)
}

/// Only a folder bundle skips its top-level `state/`; every other bundle
/// pins it, as before decision 0083. A `.brokkr` is the folder by the
/// layout's judgement and never by its name (round 634k's second
/// finding): one below the root, a library's recipe, and a library
/// root's own `.brokkr` each pin their `state/`.
#[test]
fn only_a_folder_bundle_skips_its_state() {
    let (_dir, root) = folder_root();
    let (folder, library) = (root.join(".brokkr"), root.join("recipes/state"));
    assert!(skips(&folder, "state"));
    assert!(!skips(&library, "state"));
    assert!(skips(&library, "dialects"));
    assert!(!skips(&folder, "roles"));
    let (nested, recipe) = (root.join("sub/.brokkr"), root.join("recipes/.brokkr"));
    std::fs::create_dir_all(&nested).unwrap();
    assert!(!skips(&nested, "state"));
    plant_library(&root);
    std::fs::create_dir_all(&recipe).unwrap();
    assert!(!skips(&recipe, "state"));
    assert!(!skips(&folder, "state"));
}

/// Only a folder bundle skips its top-level `recipes/`, the shelf (#696);
/// a library's recipe, and a library root's own `.brokkr`, pin theirs.
#[test]
fn only_a_folder_bundle_skips_its_shelf() {
    let (_dir, root) = folder_root();
    let (folder, recipe) = (root.join(".brokkr"), root.join("recipes/x"));
    assert!(skips(&folder, "recipes"));
    assert!(!skips(&recipe, "recipes"));
    assert!(!skips(&root.join(".brokkr/recipes/x"), "recipes"));
    plant_library(&root);
    assert!(!skips(&folder, "recipes"));
    let holds = [
        super::Skip::State,
        super::Skip::Shelf,
        super::Skip::Workspace,
    ]
    .map(super::unpinned_holds);
    assert_eq!(
        holds,
        [
            "the folder's runtime state, which a run writes",
            "the folder's shelf, whose named recipes are bundles of their own (#696)",
            "operator configuration",
        ]
    );
}

/// A folder bundle composes its base from its shelf (#696, amending
/// decision 0083 ruling 1), never the repository root; every other layer,
/// a shelf recipe and a library root's own `.brokkr` included, from its
/// parent.
#[test]
fn only_a_folder_bundle_composes_from_its_shelf() {
    let (_dir, root) = folder_root();
    let folder = root.join(".brokkr");
    assert_eq!(base_library(&folder).unwrap(), root.join(".brokkr/recipes"));
    let shelved = root.join(".brokkr/recipes/x");
    assert_eq!(
        base_library(&shelved).unwrap(),
        root.join(".brokkr/recipes")
    );
    let recipe = root.join("recipes/x");
    assert_eq!(base_library(&recipe).unwrap(), root.join("recipes"));
    plant_library(&root);
    assert_eq!(base_library(&folder).unwrap(), root);
}

/// A layer whose layout cannot be read composes no base, refused by name,
/// and pins its `state/`. A process that reads a mode-000 directory, as
/// root does, skips the case.
#[test]
fn a_folder_bundle_whose_layout_cannot_be_read_refuses_a_base() {
    let (_dir, root) = folder_root();
    plant_library(&root);
    let (recipes, folder) = (root.join("recipes"), root.join(".brokkr"));
    mode(&recipes, 0o000);
    let judged = (!reads_anything(&recipes)).then(|| {
        let refused = base_library(&folder).unwrap_err();
        (refused, !skips(&folder, "state"))
    });
    mode(&recipes, 0o755);
    let Some((refused, pinned)) = judged else {
        eprintln!("skipped: this process reads a mode-000 directory, as root does");
        return;
    };
    assert!(matches!(refused, crate::CompileError::Invalid(_)));
    assert_eq!(
        refused.to_string(),
        format!(
            "bundle: reading {}: Permission denied (os error 13)",
            recipes.display()
        )
    );
    assert!(pinned);
}

/// The personal shelf stands beside `host.json`, in the one directory
/// `host_file` finds it in (#697): under `$XDG_CONFIG_HOME`, else
/// `$HOME/.config`, and refused in its words where neither is absolute.
#[test]
fn the_personal_shelf_stands_beside_the_host_file() {
    let placed = |xdg: Option<&str>, home: Option<&str>| {
        let (xdg, home) = (xdg.map(Into::into), home.map(Into::into));
        let shelf = super::personal_shelf(xdg.clone(), home.clone()).map_err(|e| e.to_string());
        let host = crate::admission::host_file(xdg, home).map_err(|e| e.to_string());
        (shelf, host.map(|file| file.with_file_name("recipes")))
    };
    for (xdg, home, shelf) in [
        (Some("/x"), Some("/h"), "/x/brokkr/recipes"),
        (None, Some("/h"), "/h/.config/brokkr/recipes"),
        (Some("relative"), Some("/h"), "/h/.config/brokkr/recipes"),
    ] {
        let (found, beside) = placed(xdg, home);
        assert_eq!(found, Ok(PathBuf::from(shelf)), "{xdg:?}");
        assert_eq!(found, beside, "{xdg:?}");
    }
    let unplaced = "no host configuration can be found: neither XDG_CONFIG_HOME nor HOME \
                    names an absolute directory";
    let (found, beside) = placed(None, Some("relative"));
    assert_eq!(found, Err(unplaced.to_string()));
    assert_eq!(found, beside);
}
