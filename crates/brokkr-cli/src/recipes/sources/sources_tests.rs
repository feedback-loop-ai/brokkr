use super::*;
use crate::recipes::tests::{add, folder_repository};

/// A folder bundle refuses as a source by the type, judged where it
/// resolves: spelled, through a link, and as the base a leaf beside it
/// extends. The destination's shelf is never made (#696).
#[test]
fn add_refuses_a_folder_bundle_by_type() {
    let (_source, source) = folder_repository();
    let (_destination, root) = folder_repository();
    let folder = source.join(FOLDER);
    let linked = source.join("linked");
    std::os::unix::fs::symlink(&folder, &linked).unwrap();
    let leaf = source.join("leaf");
    std::fs::create_dir(&leaf).unwrap();
    let layer = serde_json::json!({"name": "leaf", "extends": FOLDER});
    std::fs::write(leaf.join("bundle.json"), layer.to_string()).unwrap();
    let shelf = root.join(".brokkr/recipes");
    for from in [&folder, &linked, &leaf] {
        let refusal = add(&root, &shelf, from.to_str().unwrap(), "x", None).unwrap_err();
        let expected = FolderBundle {
            source: folder.clone(),
        };
        let refused = refusal.downcast_ref::<FolderBundle>();
        assert_eq!(refused, Some(&expected), "{refusal}");
        assert!(!shelf.exists(), "{}", from.display());
    }
}
