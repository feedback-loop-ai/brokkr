//! The removal walk (decision 0104 ruling 8, slice 5c1): links are removed
//! as entries, a hard-linked inode is counted once, a read-only directory is
//! unsealed, an absent path is absent, a partial removal is an error naming
//! its path, and an entry on another device is never entered.

use std::collections::HashSet;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use super::*;

/// The bytes `tree` holds, counted the way the walk counts them, so a test
/// can hold the walk to the exact figure.
fn allocated(tree: &Path) -> u64 {
    fn go(path: &Path, seen: &mut HashSet<(u64, u64)>, total: &mut u64) {
        let meta = std::fs::symlink_metadata(path).unwrap();
        if seen.insert((meta.dev(), meta.ino())) {
            *total += meta.blocks() * 512;
        }
        if meta.file_type().is_dir() {
            for entry in std::fs::read_dir(path).unwrap() {
                go(&entry.unwrap().path(), seen, total);
            }
        }
    }
    let mut seen = HashSet::new();
    let mut total = 0;
    go(tree, &mut seen, &mut total);
    total
}

fn set_mode(path: &Path, bits: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(bits)).unwrap();
}

/// The path and errno of a removal's io refusal.
fn refused_io(refused: &TreeError) -> (&Path, i32) {
    let TreeError::Io { path, cause } = refused else {
        panic!("{refused:?}");
    };
    (path, cause.raw_os_error().expect("an errno"))
}

/// A tree holding one read-only `sealed/` directory with a file in it.
fn sealed_tree(tmp: &Path) -> (PathBuf, PathBuf) {
    let tree = tmp.join("tree");
    let sealed = tree.join("sealed");
    std::fs::create_dir_all(&sealed).unwrap();
    std::fs::write(sealed.join("call"), "").unwrap();
    (tree, sealed)
}

#[test]
fn a_hard_linked_inode_is_counted_once() {
    let tmp = tempfile::tempdir().unwrap();
    let tree = tmp.path().join("tree");
    std::fs::create_dir(&tree).unwrap();
    let file = tree.join("one");
    std::fs::write(&file, vec![7u8; 12_000]).unwrap();
    std::fs::hard_link(&file, tree.join("two")).unwrap();
    let expected = allocated(&tree);
    assert_eq!(remove_tree(&tree).unwrap(), Some(expected));
    assert!(!tree.exists());
}

#[test]
fn a_symlink_is_removed_as_a_link_and_its_target_stands() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = tmp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    let target = outside.join("target");
    std::fs::write(&target, "kept").unwrap();
    let tree = tmp.path().join("tree");
    std::fs::create_dir(&tree).unwrap();
    std::os::unix::fs::symlink(&target, tree.join("link")).unwrap();
    let expected = allocated(&tree);
    assert_eq!(remove_tree(&tree).unwrap(), Some(expected));
    assert!(!tree.exists());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "kept");
}

#[test]
fn a_read_only_directory_is_unsealed_and_removed() {
    let tmp = tempfile::tempdir().unwrap();
    let (tree, sealed) = sealed_tree(tmp.path());
    set_mode(&sealed, 0o500);
    let expected = allocated(&tree);
    assert_eq!(remove_tree(&tree).unwrap(), Some(expected));
    assert!(!tree.exists());
}

#[test]
fn a_missing_path_is_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let absent = tmp.path().join("absent");
    assert_eq!(remove_tree(&absent).unwrap(), None);
    assert_eq!(count_tree(&absent).unwrap(), None);
}

#[test]
fn a_partial_removal_names_the_path_it_could_not_remove_and_leaves_the_root() {
    let tmp = tempfile::tempdir().unwrap();
    // 0500 lets the walk empty the tree but not unlink its root; 0000 stops
    // the stat before the open. Both are a partial removal, both name the
    // root, and both leave it for the next pass.
    for (mode, emptied) in [(0o500, true), (0o000, false)] {
        let outer = tmp.path().join(format!("outer-{mode:o}"));
        let tree = outer.join("tree");
        std::fs::create_dir_all(&tree).unwrap();
        std::fs::write(tree.join("call"), "").unwrap();
        set_mode(&outer, mode);
        let refused = remove_tree(&tree).unwrap_err();
        set_mode(&outer, 0o700);
        let (path, errno) = refused_io(&refused);
        assert_eq!(path, tree);
        assert_eq!(errno, Errno::ACCESS.raw_os_error());
        assert!(tree.is_dir(), "the root is left for the next pass");
        if emptied {
            assert_eq!(std::fs::read_dir(&tree).unwrap().count(), 0);
            remove_tree(&tree).unwrap();
        } else {
            assert!(tree.join("call").is_file());
        }
    }
}

#[test]
fn the_listing_removes_nothing_and_counts_what_the_walk_removes() {
    let tmp = tempfile::tempdir().unwrap();
    let (tree, sealed) = sealed_tree(tmp.path());
    std::fs::write(tree.join("one"), vec![1u8; 4096]).unwrap();
    std::fs::write(sealed.join("two"), vec![2u8; 8192]).unwrap();
    set_mode(&sealed, 0o500);
    let expected = allocated(&tree);
    assert_eq!(count_tree(&tree).unwrap(), Some(expected));
    assert!(tree.join("one").is_file());
    assert!(sealed.join("two").is_file());
    assert_eq!(remove_tree(&tree).unwrap(), Some(expected));
    assert!(!tree.exists());
}

/// A link at the tree's own path is refused, never followed and never
/// unsealed: its target stands exactly as it was.
#[test]
fn a_link_at_the_root_is_refused_and_its_target_stands() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("target");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("kept"), "").unwrap();
    set_mode(&target, 0o500);
    let link = tmp.path().join("link");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    for refused in [
        remove_tree(&link).unwrap_err(),
        count_tree(&link).unwrap_err(),
    ] {
        let (path, _) = refused_io(&refused);
        assert_eq!(path, link);
    }
    assert_eq!(
        std::fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o500,
        "the link's target was not unsealed"
    );
    assert!(target.join("kept").is_file());
    set_mode(&target, 0o700);
}

/// Removal errors compare by path and cause, the equality a reaper's kept
/// tree relies on.
#[test]
fn removal_errors_compare_by_path_and_cause() {
    let denied = || TreeError::Io {
        path: PathBuf::from("/tree"),
        cause: std::io::Error::from_raw_os_error(Errno::ACCESS.raw_os_error()),
    };
    assert_eq!(denied(), denied());
    assert_ne!(
        denied(),
        TreeError::Io {
            path: PathBuf::from("/other"),
            cause: std::io::Error::from_raw_os_error(Errno::ACCESS.raw_os_error()),
        }
    );
    assert_ne!(
        denied(),
        TreeError::Io {
            path: PathBuf::from("/tree"),
            cause: std::io::Error::from_raw_os_error(Errno::NOENT.raw_os_error()),
        }
    );
    let foreign = TreeError::Foreign {
        path: PathBuf::from("/tree/mounted"),
        device: 2,
        tree: 1,
    };
    assert_eq!(
        foreign,
        TreeError::Foreign {
            path: PathBuf::from("/tree/mounted"),
            device: 2,
            tree: 1,
        }
    );
    assert_ne!(
        foreign,
        TreeError::Foreign {
            path: PathBuf::from("/tree/mounted"),
            device: 3,
            tree: 1,
        }
    );
    assert_ne!(
        foreign,
        TreeError::Foreign {
            path: PathBuf::from("/tree/mounted"),
            device: 2,
            tree: 9,
        }
    );
    assert_ne!(
        foreign,
        TreeError::Foreign {
            path: PathBuf::from("/tree/other"),
            device: 2,
            tree: 1,
        }
    );
    assert_ne!(foreign, denied());
}

/// A tmpfs is not mountable in an unprivileged test, so the crossing is
/// driven through the walk's device seam: it reports another device for the
/// planted entry, as a mount inside the tree would.
#[test]
fn an_entry_on_another_device_is_not_entered() {
    let tmp = tempfile::tempdir().unwrap();
    let tree = tmp.path().join("tree");
    let foreign = tree.join("mounted");
    std::fs::create_dir_all(&foreign).unwrap();
    std::fs::write(foreign.join("call"), "").unwrap();
    let refused = walk(&tree, true, |path, device| {
        if path == foreign {
            device + 1
        } else {
            device
        }
    })
    .unwrap_err();
    let TreeError::Foreign {
        path,
        device,
        tree: root,
    } = &refused
    else {
        panic!("{refused:?}");
    };
    assert_eq!(path, &foreign);
    assert_eq!(*device, *root + 1);
    assert!(
        foreign.join("call").is_file(),
        "the foreign entry is intact"
    );
    assert!(tree.is_dir());
}
