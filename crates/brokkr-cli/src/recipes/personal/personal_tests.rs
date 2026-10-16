//! The personal shelf's resolution and the directories an install makes
//! for it (#697), each by its exact outcome.

use std::path::{Path, PathBuf};

use super::{holding_in, Made, Origin, Unshelved};

/// A project shelf and a personal one, side by side, with the recipes
/// `project` and `personal` name made on each.
fn shelves(project: &[&str], personal: &[&str]) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let (on_project, on_personal) = (dir.path().join("project"), dir.path().join("personal"));
    for (shelf, names) in [(&on_project, project), (&on_personal, personal)] {
        for name in names {
            std::fs::create_dir_all(shelf.join(name)).unwrap();
        }
    }
    (dir, on_project, on_personal)
}

/// The project's shelf, else the personal one, else the project's, whose
/// launch refuses the name; a name on both refuses with both paths; no
/// personal shelf, or a name that is not one plain component, reads the
/// project's alone.
#[test]
fn a_recipe_is_read_from_the_one_shelf_that_holds_it() {
    let (_dir, project, personal) = shelves(&["fast", "both"], &["mine", "both"]);
    let held = |name: &str, personal: Option<&Path>| {
        holding_in(name, project.clone(), personal).map_err(|refused| refused.to_string())
    };
    assert_eq!(held("fast", Some(&personal)), Ok(project.clone()));
    assert_eq!(held("mine", Some(&personal)), Ok(personal.clone()));
    assert_eq!(held("absent", Some(&personal)), Ok(project.clone()));
    assert_eq!(held("mine", None), Ok(project.clone()));
    assert_eq!(
        held("../personal/mine", Some(&personal)),
        Ok(project.clone())
    );
    let shadowed = format!(
        "recipe 'both' stands on both the project shelf, at {}, and the personal shelf, at {} \
         (#697): neither shelf wins, so rename or remove one of them",
        project.join("both").display(),
        personal.join("both").display()
    );
    assert_eq!(held("both", Some(&personal)), Err(shadowed));
    assert!(matches!(
        holding_in("both", project.clone(), Some(&personal)),
        Err(Unshelved::Shadowed { name, .. }) if name == "both"
    ));
}

/// An entry at the name that cannot be read refuses by its path, never
/// read as absent. A process that reads a mode-000 directory skips.
#[test]
fn a_shelf_that_cannot_be_read_refuses() {
    let (_dir, project, personal) = shelves(&[], &["mine"]);
    std::fs::create_dir(&project).unwrap();
    let mode = |bits| std::os::unix::fs::PermissionsExt::from_mode(bits);
    std::fs::set_permissions(&project, mode(0o000)).unwrap();
    let readable = std::fs::symlink_metadata(project.join("mine")).is_ok();
    let held = holding_in("mine", project.clone(), Some(&personal));
    std::fs::set_permissions(&project, mode(0o755)).unwrap();
    if readable {
        eprintln!("skipped: this process reads a mode-000 directory, as root does");
        return;
    }
    let refused = held.unwrap_err();
    assert!(matches!(refused, Unshelved::Unreadable(_)));
    let denied = format!(
        "reading {}: Permission denied (os error 13)",
        project.join("mine").display()
    );
    assert_eq!(refused.to_string(), denied);
}

/// The origin column's words and doctor's.
#[test]
fn each_origin_is_named() {
    assert_eq!(
        [Origin::Project, Origin::Personal].map(|origin| origin.to_string()),
        ["project", "personal"]
    );
    assert_eq!(Origin::Project.doctor(), ("recipe", "shelf", "#696"));
    assert_eq!(
        Origin::Personal.doctor(),
        ("personal recipe", "personal shelf", "#697")
    );
}

/// The directories above a personal shelf an install made are removed
/// unless kept, and one that stood before is never removed.
#[test]
fn the_directories_made_for_a_shelf_go_unless_kept() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    let shelf = config.join("brokkr/recipes");
    drop(Made::above(&shelf).unwrap());
    assert!(!config.exists());
    Made::above(&shelf).unwrap().keep();
    assert!(config.join("brokkr").is_dir());
    drop(Made::above(&shelf).unwrap());
    assert!(config.join("brokkr").is_dir());
}
