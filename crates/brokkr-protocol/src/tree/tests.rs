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

/// A target directory holding `kept` at mode 0500, and a `link` to it.
fn linked_target(tmp: &Path) -> (PathBuf, PathBuf) {
    let target = tmp.join("target");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("kept"), "").unwrap();
    set_mode(&target, 0o500);
    let link = tmp.join("link");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    (target, link)
}

/// Both forms of the walk's refusal of `path`.
fn refused_both(path: &Path) -> [TreeError; 2] {
    [
        remove_tree(path).unwrap_err(),
        count_tree(path).unwrap_err(),
    ]
}

/// The target a link named stands exactly as it was, and is restored so
/// the temporary directory can be removed.
fn target_stands(target: &Path) {
    assert_eq!(
        std::fs::metadata(target).unwrap().permissions().mode() & 0o777,
        0o500,
        "the link's target was not unsealed"
    );
    assert!(target.join("kept").is_file());
    set_mode(target, 0o700);
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
    // A parent that is absent makes the root absent too, before either
    // walk opens it.
    let under_absent = absent.join("tree");
    assert_eq!(remove_tree(&under_absent).unwrap(), None);
    assert_eq!(count_tree(&under_absent).unwrap(), None);
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
    assert_eq!(
        count_tree(&tree).unwrap().map(|counted| counted.bytes),
        Some(expected)
    );
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
    let (target, link) = linked_target(tmp.path());
    for refused in refused_both(&link) {
        let (path, _) = refused_io(&refused);
        assert_eq!(path, link);
    }
    target_stands(&target);
}

/// Removal errors compare by path and cause, the equality a reaper's kept
/// tree relies on.
#[test]
fn removal_errors_compare_by_path_and_cause() {
    let denied = || TreeError::Io {
        path: PathBuf::from("/tree"),
        cause: std::io::Error::from_raw_os_error(Errno::ACCESS.raw_os_error()),
    };
    let access = std::io::Error::from_raw_os_error(Errno::ACCESS.raw_os_error());
    assert_eq!(denied().cause(), access.to_string());
    assert_eq!(denied().to_string(), format!("removing /tree: {access}"));
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
        foreign.cause(),
        "is on another filesystem (device 2, not 1); a tree is removed on its own filesystem"
    );
    assert_eq!(
        foreign.to_string(),
        "/tree/mounted is on another filesystem (device 2, not 1); a tree is removed on its \
         own filesystem"
    );
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

/// A root's own name is the only component the walk opens no-follow, so a
/// spelling that hides a link behind a trailing separator, `.` or `..` is
/// refused before anything opens. The link's target and the directory's
/// parent stand, modes and contents alike.
#[test]
fn a_root_spelled_with_a_separator_or_a_dot_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, link) = linked_target(tmp.path());
    let dir = tmp.path().join("dir");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join("kept"), "").unwrap();

    for (root, suffix) in [
        (&link, "/"),
        (&link, "/."),
        (&link, "/.."),
        (&dir, "/."),
        (&dir, "/.."),
    ] {
        let spelled = PathBuf::from(format!("{}{suffix}", root.display()));
        for refused in refused_both(&spelled) {
            let (named, errno) = refused_io(&refused);
            assert_eq!(named, spelled.as_path(), "{spelled:?}");
            assert_eq!(errno, Errno::INVAL.raw_os_error(), "{spelled:?}");
        }
    }

    target_stands(&target);
    assert!(dir.join("kept").is_file(), "the parent's contents stand");
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    // A root with no parent is refused too: it has no final name to open.
    for refused in refused_both(Path::new("relative")) {
        let (named, errno) = refused_io(&refused);
        assert_eq!(named, Path::new("relative"));
        assert_eq!(errno, Errno::INVAL.raw_os_error());
    }
    // A plain root is still removed.
    assert!(remove_tree(&dir).unwrap().is_some());
    assert!(!dir.exists());
}

/// A stat the removal cannot do before opening names its path and errno.
#[test]
fn a_root_name_the_filesystem_refuses_is_named() {
    let tmp = tempfile::tempdir().unwrap();
    let long = tmp.path().join("x".repeat(300));
    let refused = remove_tree(&long).unwrap_err();
    let (named, errno) = refused_io(&refused);
    assert_eq!(named, long);
    assert_eq!(errno, Errno::NAMETOOLONG.raw_os_error());
}

/// A count that meets a directory it cannot read is a lower bound naming
/// that directory; it changes no mode, and a removal still takes the tree
/// whole.
#[test]
fn an_unreadable_directory_makes_the_count_a_lower_bound() {
    let tmp = tempfile::tempdir().unwrap();
    let tree = tmp.path().join("tree");
    let work = tree.join("overlay").join("0").join("work").join("work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(work.join("index"), vec![3u8; 4096]).unwrap();
    let whole = allocated(&tree);
    set_mode(&work, 0o000);

    let counted = count_tree(&tree).unwrap().unwrap();
    assert_eq!(counted.unreadable.as_deref(), Some(work.as_path()));
    assert!(
        counted.bytes < whole,
        "{} is not below {whole}",
        counted.bytes
    );
    assert_eq!(
        std::fs::metadata(&work).unwrap().permissions().mode() & 0o777,
        0o000,
        "the count changed no mode"
    );
    assert_eq!(remove_tree(&tree).unwrap(), Some(whole));
    assert!(!tree.exists());

    // A root the count cannot read is a lower bound naming the root.
    let sealed = tmp.path().join("sealed-root");
    std::fs::create_dir(&sealed).unwrap();
    std::fs::write(sealed.join("kept"), vec![4u8; 4096]).unwrap();
    let whole = allocated(&sealed);
    set_mode(&sealed, 0o000);
    let counted = count_tree(&sealed).unwrap().unwrap();
    assert_eq!(counted.unreadable.as_deref(), Some(sealed.as_path()));
    assert!(counted.bytes < whole);
    set_mode(&sealed, 0o700);
    assert_eq!(remove_tree(&sealed).unwrap(), Some(whole));
}

/// A readdir the walk meets is the walk's error naming that directory, and
/// the count's first unreadable path: a regular file's descriptor is a
/// listing the kernel refuses.
#[test]
fn a_readdir_error_is_the_walks_error_and_the_counts_lower_bound() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("file");
    std::fs::write(&file, "").unwrap();
    let fd: OwnedFd = std::fs::File::open(&file).unwrap().into();
    let path = tmp.path().join("tree");

    let mut removal = Walk::new(true);
    let refused = descend(&fd, &path, 1, &mut removal, &same_device).unwrap_err();
    let (named, errno) = refused_io(&refused);
    assert_eq!(named, path);
    assert_eq!(errno, Errno::NOTDIR.raw_os_error());

    let mut listing = Walk::new(false);
    descend(&fd, &path, 1, &mut listing, &same_device).unwrap();
    assert_eq!(listing.unreadable.as_deref(), Some(path.as_path()));
}
