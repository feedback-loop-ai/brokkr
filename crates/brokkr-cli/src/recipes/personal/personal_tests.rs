//! The personal shelf's resolution and the directories an install makes
//! for it (#697), each by its exact outcome.

use std::path::{Path, PathBuf};

use super::{resolving, Made, Onto, Origin, ShelfPath, Unshelved};

/// [`super::holding`], the project's shelf spelled as a path.
fn holding(recipe: Option<&str>, project: PathBuf) -> Result<PathBuf, Unshelved> {
    super::holding(recipe, ShelfPath::new(project))
}

/// [`super::holding_in`], both shelves spelled as paths.
fn holding_in(name: &str, project: PathBuf, personal: Option<&Path>) -> Result<PathBuf, Unshelved> {
    let personal = personal.map(ShelfPath::new);
    super::holding_in(name, ShelfPath::new(project), personal.as_ref())
}

/// [`Origin::place`], the shelf spelled as a path.
fn place(origin: Origin, dir: &Path) -> anyhow::Result<PathBuf> {
    origin.place(&ShelfPath::new(dir))
}

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
/// personal shelf reads the project's alone.
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

/// A name that is not one plain recipe name, judged by its spelling,
/// refuses by its own variant before any shelf is read, so a shelf that
/// holds `x` is never reached through `x/` or `./x`; no recipe named reads
/// the project's shelf.
#[test]
fn a_recipe_named_as_a_path_refuses_before_any_shelf_is_read() {
    let (_dir, project, _) = shelves(&["x"], &[]);
    for name in ["x/", "./x", "x/.", "../x", "a/b", "/abs/x", "", ".", ".."] {
        let refused = holding(Some(name), project.clone()).unwrap_err();
        assert!(
            matches!(&refused, Unshelved::Unplain { name: at } if at == name),
            "{name}: {refused:?}"
        );
    }
    let text = holding(Some("x/"), project.clone())
        .unwrap_err()
        .to_string();
    let unplain = "--recipe 'x/' is not one plain recipe name, so it would not name one recipe on \
                   a shelf (#697): name the recipe as it stands on its shelf, or give --bundle \
                   <dir> for a bundle elsewhere; nothing was run";
    assert_eq!(text, unplain);
    assert_eq!(holding(None, project.clone()).unwrap(), project);
}

/// What a shelf that cannot be read refuses with: the `origin` shelf at
/// `shelf`, judged at `at` for `cause`.
fn unread(origin: Origin, shelf: &Path, at: &Path, cause: &str) -> String {
    format!(
        "the {origin} shelf {} cannot be read (#697): {}: {cause}; a shelf that cannot be read is \
         never read as an empty one, so mend or remove it; nothing was run or installed",
        shelf.display(),
        at.display()
    )
}

const DANGLING: &str = "No such file or directory (os error 2)";

/// The personal shelf resolves where it stands, through a link above it,
/// and judges no mark there; a link to nothing above it, and a shelf that
/// is a file, refuse the install by the shelf's path, never read as
/// absent or as a place to write.
#[test]
fn a_personal_shelf_resolves_where_it_stands() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().canonicalize().unwrap().join("real");
    std::fs::create_dir(&real).unwrap();
    let linked = dir.path().join("linked");
    std::os::unix::fs::symlink(&real, &linked).unwrap();
    let placed = place(Origin::Personal, &linked.join("brokkr/recipes")).unwrap();
    assert_eq!(placed, real.join("brokkr/recipes"));
    let dangling = dir.path().join("dangling");
    std::os::unix::fs::symlink("absent", &dangling).unwrap();
    let shelf = dangling.join("recipes");
    let refused = place(Origin::Personal, &shelf).unwrap_err();
    let expected = unread(Origin::Personal, &shelf, &dangling, DANGLING);
    assert_eq!(refused.to_string(), expected);
    let file = real.join("file");
    std::fs::write(&file, "").unwrap();
    let refused = place(Origin::Personal, &file).unwrap_err();
    let expected = unread(Origin::Personal, &file, &file, "not a directory");
    assert_eq!(refused.to_string(), expected);
}

/// A shelf that cannot be read, the project's or the personal one,
/// refuses by its own variant before either recipe is taken, with `x` on
/// the readable shelf alone and with `x` on both until the break: a link
/// to nothing, a file, and a link to nothing above the shelf. Nothing at
/// the personal shelf, or at a relative project shelf or above it, is an
/// empty shelf.
#[test]
fn a_shelf_that_cannot_be_read_refuses_before_either_recipe_is_taken() {
    let dangle: fn(&Path) = |at| std::os::unix::fs::symlink("gone", at).unwrap();
    let file: fn(&Path) = |at| std::fs::write(at, "").unwrap();
    // Each break: what is moved away and replaced, and the cause it reads.
    let breaks = [
        ("in/recipes", dangle, DANGLING),
        ("in/recipes", file, "not a directory"),
        ("in", dangle, DANGLING),
    ];
    let cases = [Origin::Project, Origin::Personal]
        .into_iter()
        .flat_map(|origin| breaks.map(|broken| (origin, broken)))
        .flat_map(|case| [true, false].map(|both| (case, both)));
    for ((origin, (judged, plant, cause)), both) in cases {
        let dir = tempfile::tempdir().unwrap();
        let sides = [dir.path().join("project"), dir.path().join("personal")];
        let broken = &sides[usize::from(origin == Origin::Personal)];
        let shelf = |side: &Path| side.join("in/recipes");
        for side in &sides {
            std::fs::create_dir_all(shelf(side)).unwrap();
            if side != broken || both {
                std::fs::create_dir(shelf(side).join("x")).unwrap();
            }
        }
        std::fs::rename(broken.join(judged), broken.join("moved")).unwrap();
        plant(&broken.join(judged));
        let refused = holding_in("x", shelf(&sides[0]), Some(&shelf(&sides[1]))).unwrap_err();
        let text = unread(origin, &shelf(broken), &broken.join(judged), cause);
        assert_eq!(refused.to_string(), text, "{both}");
        assert!(matches!(
            refused,
            Unshelved::Unread { origin: of, shelf: at, .. } if of == origin && at == shelf(broken)
        ));
    }
    let (_dir, project, personal) = shelves(&["x"], &[]);
    let absent = personal.join("brokkr/recipes");
    let held = |project: &Path| {
        holding_in("x", project.to_path_buf(), Some(&absent)).map_err(|e| e.to_string())
    };
    assert_eq!(held(&project), Ok(project));
    let relative = PathBuf::from("no-shelf-697/recipes");
    assert_eq!(held(&relative), Ok(relative));
}

/// Every spelling of one shelf is one shelf (#697): a trailing separator,
/// a `.` component and a doubled separator are gone, relative or
/// absolute, byte for byte; a `..` stays as spelled; `.` alone is the
/// current directory, and an empty spelling stays empty.
#[test]
fn every_spelling_of_a_shelf_is_one_shelf() {
    let one = |spellings: &[&str], normal: &str| {
        for spelled in spellings {
            let shelf = ShelfPath::new(spelled);
            assert_eq!(shelf.path().as_os_str(), normal, "{spelled}");
            assert_eq!(shelf, ShelfPath::new(normal), "{spelled}");
        }
    };
    let relative = [
        ".brokkr/recipes/",
        ".brokkr/recipes/.",
        "./.brokkr/recipes/",
        ".brokkr//recipes",
        "./.brokkr/./recipes//",
    ];
    one(&relative, ".brokkr/recipes");
    let absolute = [
        "/r/.brokkr/recipes/",
        "/r//.brokkr/recipes",
        "/r/./.brokkr/recipes/.",
    ];
    one(&absolute, "/r/.brokkr/recipes");
    one(&["a/../b/", "./a/../b"], "a/../b");
    one(&[".", "./", "./."], ".");
    assert_eq!(ShelfPath::new("").path().as_os_str(), "");
}

/// A shelf spelled with a trailing separator or a trailing `.` is judged
/// at its own entry, never through it: a link to nothing and a file each
/// refuse by the variant, at the shelf, for the cause the bare spelling
/// reads.
#[test]
fn a_shelf_spelled_through_its_entry_is_judged_at_it() {
    let dir = tempfile::tempdir().unwrap();
    let (link, file) = (dir.path().join("link"), dir.path().join("file"));
    std::os::unix::fs::symlink("gone", &link).unwrap();
    std::fs::write(&file, "").unwrap();
    for (entry, cause) in [(&link, DANGLING), (&file, "not a directory")] {
        for spelled in [
            format!("{}/", entry.display()),
            format!("{}/.", entry.display()),
        ] {
            let refused = resolving(Origin::Project, &ShelfPath::new(&spelled)).err();
            let judged = matches!(
                &refused,
                Some(Unshelved::Unread { origin: Origin::Project, shelf, at, .. })
                    if shelf.as_os_str() == entry.as_os_str() && at.as_os_str() == entry.as_os_str()
            );
            assert!(judged, "{spelled}: {refused:?}");
            let text = unread(Origin::Project, entry, entry, cause);
            assert_eq!(refused.unwrap().to_string(), text, "{spelled}");
        }
    }
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

/// What a shelf spelled through `..` beneath an absent part refuses with:
/// the `origin` shelf `shelf`, beneath `at`, the entry of it that stands.
fn climbing(origin: Origin, shelf: &Path, at: &Path) -> String {
    format!(
        "the {origin} shelf {} climbs by '..' out of a directory that does not stand beneath {} \
         (#726): a shelf spelled through what does not exist is never read as an empty one, so \
         spell it without the '..'; nothing was run or installed",
        shelf.display(),
        at.display()
    )
}

/// A shelf whose part beneath its nearest standing entry holds a `..`
/// refuses by its own variant on either origin, absolute and, beneath the
/// current directory, relative (#726); a `..` beneath a standing entry is
/// the kernel's, so the shelf resolves where it leads.
#[test]
fn a_shelf_climbing_out_of_what_does_not_stand_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::fs::create_dir_all(root.join("cfg/sub")).unwrap();
    let absolute = [
        (root.join("nope/../cfg/brokkr/recipes"), root.clone()),
        (root.join("cfg/gone/../brokkr/recipes"), root.join("cfg")),
        (root.join("cfg/sub/gone/.."), root.join("cfg/sub")),
    ];
    let relative = PathBuf::from("no-shelf-726/../recipes");
    let cases = absolute
        .into_iter()
        .chain([(relative, PathBuf::from("."))])
        .flat_map(|case| [Origin::Project, Origin::Personal].map(|origin| (origin, case.clone())));
    for (origin, (shelf, at)) in cases {
        let refused = resolving(origin, &ShelfPath::new(&shelf)).err();
        let judged = matches!(
            &refused,
            Some(Unshelved::Climbing { origin: of, shelf: spelled, at: stands })
                if *of == origin && *spelled == shelf && *stands == at
        );
        assert!(judged, "{}: {refused:?}", shelf.display());
        let text = climbing(origin, &shelf, &at);
        assert_eq!(refused.unwrap().to_string(), text);
    }
    let standing = ShelfPath::new(root.join("cfg/sub/../brokkr/recipes"));
    let resolved = resolving(Origin::Personal, &standing).unwrap();
    assert_eq!(resolved.place, root.join("cfg/brokkr/recipes"));
}

/// With no personal shelf placed, a project shelf climbing out of what
/// does not stand still refuses before the recipe is taken (#726).
#[test]
fn a_climbing_project_shelf_refuses_with_no_personal_shelf() {
    let (_dir, project, _) = shelves(&["x"], &[]);
    let shelf = project.join("sub/..");
    let refused = holding_in("x", shelf.clone(), None).unwrap_err();
    let climbed = matches!(&refused, Unshelved::Climbing { origin: Origin::Project, shelf: of, at }
        if *of == shelf && *at == project);
    assert!(climbed, "{refused:?}");
    assert_eq!(
        refused.to_string(),
        climbing(Origin::Project, &shelf, &project)
    );
}

/// The directories an install makes for a shelf are each held from before
/// the first is made: a component too long to make, below one the install
/// made, refuses naming the shelf and the directory, and leaves the tree
/// as it was (#726).
#[test]
fn a_directory_that_cannot_be_made_removes_every_one_made() {
    let dir = tempfile::tempdir().unwrap();
    let long = "a".repeat(256);
    let at = dir.path().join("new").join(&long);
    let shelf = at.join("brokkr/recipes");
    let refused = Made::above(&shelf).err();
    let cause = std::fs::create_dir(dir.path().join(&long)).unwrap_err();
    let made = matches!(&refused, Some(Unshelved::Unmade { shelf: of, at: unmade, .. })
        if *of == shelf && *unmade == at);
    assert!(made, "{refused:?}");
    let text = format!(
        "the personal shelf {} cannot be made (#726): {}: {cause}; every directory made for it \
         was removed again, so nothing was installed",
        shelf.display(),
        at.display()
    );
    assert_eq!(refused.unwrap().to_string(), text);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

/// An install onto the personal shelf whose plan holds a name the project
/// shelf holds, the recipe's or a base's, refuses naming both paths
/// (#726); a plan of names it does not hold, and any plan onto the project
/// shelf, pass to the install, and a project shelf that cannot be read
/// refuses as resolution refuses it.
#[test]
fn an_install_of_a_name_the_project_shelf_holds_refuses() {
    let (_dir, project, personal) = shelves(&["x"], &[]);
    let on_project = ShelfPath::new(&project);
    let onto = Onto::Personal {
        project: &on_project,
    };
    let plan = |names: &[&str]| -> Vec<(String, PathBuf)> {
        names
            .iter()
            .map(|name| (name.to_string(), PathBuf::new()))
            .collect()
    };
    let unshadowing = |names: &[&str]| onto.unshadowing(&plan(names), &personal);
    let refused = unshadowing(&["y", "x"]).unwrap_err();
    assert!(matches!(&refused, Unshelved::Shadowing { name, .. } if name == "x"));
    let shadowing = format!(
        "recipe 'x' stands on the project shelf, at {}, so installing it onto the personal shelf, \
         at {}, would stand it on both (#726): neither shelf would win, so rename or remove one of \
         them; nothing was installed",
        project.join("x").display(),
        personal.join("x").display()
    );
    assert_eq!(refused.to_string(), shadowing);
    assert!(matches!(unshadowing(&["y", "z"]), Ok(())));
    let projected = Onto::Project.unshadowing(&plan(&["x"]), &project);
    assert!(matches!(projected, Ok(())));
    std::fs::rename(&project, project.with_extension("gone")).unwrap();
    std::os::unix::fs::symlink("gone", &project).unwrap();
    let refused = unshadowing(&["x"]).unwrap_err();
    let text = unread(Origin::Project, &project, &project, DANGLING);
    assert_eq!(refused.to_string(), text);
}
