//! Composition across shelves (#697 rulings 3 and 5): a folder repository
//! and a personal shelf beside it, each recipe a small bundle on disk, and
//! an assertion about the chain the resolver reads or the refusal it gives.

use std::io::ErrorKind::NotFound;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::{Named, Origin, ShelfRefusal, Shelves};
use crate::bundle::compose::{resolve_unsealed, Resolved};
use crate::bundle::CompileError;
use crate::places::places_tests::clean_git_config;
use crate::places::{Layout, PlacesError};

/// A folder repository at `repo` and the personal shelf at `personal`,
/// beside it, both canonical.
struct Shelved {
    _dir: tempfile::TempDir,
    repo: PathBuf,
    personal: PathBuf,
}

impl Shelved {
    /// A repository whose folder's bundle extends `folder_extends`, if
    /// anything, and whose shelf holds `fast`.
    fn new(folder_extends: Option<&str>) -> Shelved {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let (repo, personal) = (root.join("repo"), root.join("config/brokkr/recipes"));
        crate::places::places_tests::test_repository(&repo);
        std::fs::create_dir_all(&personal).unwrap();
        recipe(&repo.join(".brokkr"), "folder", folder_extends);
        recipe(&repo.join(".brokkr/recipes/fast"), "fast", None);
        Shelved {
            _dir: dir,
            repo,
            personal,
        }
    }

    /// The shelves a compile from the repository reads, with the personal
    /// one placed.
    fn shelves(&self) -> Shelves {
        self.selecting(None)
    }

    /// [`Shelved::shelves`] with the `project` shelf selected, if any.
    fn selecting(&self, project: Option<&Path>) -> Shelves {
        let layout = Layout::discover(&self.repo).unwrap();
        Shelves::of(&layout, project, Some(self.personal.clone()))
    }

    fn project(&self, name: &str) -> PathBuf {
        self.repo.join(".brokkr/recipes").join(name)
    }

    /// The bundle at `leaf` resolved across the repository's shelves.
    fn resolve(&self, leaf: &Path) -> Result<Resolved, CompileError> {
        resolve_unsealed(leaf, &self.shelves())
    }

    /// The project layer at `layer` naming the personal recipe `mine`.
    fn naming_mine(&self, layer: &Path) -> Named {
        let (base, personal) = ("mine".to_string(), self.personal.join("mine"));
        let layer = layer.to_path_buf();
        Named {
            layer,
            base,
            personal,
        }
    }
}

/// The shelf's refusal `error` is.
fn shelf_refusal(error: &CompileError) -> &ShelfRefusal {
    match error {
        CompileError::Shelf(refusal) => refusal,
        other => panic!("{other}"),
    }
}

/// Whether `refusal` is the shadowing of `name`, standing on the project
/// shelf at `project` and on the personal one at `personal`.
fn shadows(refusal: &ShelfRefusal, (name, project, personal): (&str, &Path, &Path)) -> bool {
    matches!(refusal, ShelfRefusal::Shadowed { name: n, project: p, personal: m }
        if (n.as_str(), p.as_path(), m.as_path()) == (name, project, personal))
}

/// A bundle named `name` at `dir`, extending `extends` if anything, with
/// its own table and seats only where it extends nothing.
fn recipe(dir: &Path, name: &str, extends: Option<&str>) -> PathBuf {
    std::fs::create_dir_all(dir.join("roles")).unwrap();
    std::fs::write(dir.join("roles/role.md"), format!("# {name}\n")).unwrap();
    let rules = [
        json!({"id": "WORK", "from": "work", "result": "complete", "next": "done", "reason": "w"}),
    ];
    let table = json!({"schema": "forge.phase-machine/v1", "phases": ["work", "done"],
                       "initial": "work", "terminal": ["done"], "rules": rules});
    let seat = json!({"results": ["complete"], "role": "roles/role.md",
                      "driver": {"command": ["./drive"]}});
    let bundle = match extends {
        Some(base) => json!({"name": name, "extends": base}),
        None => json!({"name": name, "policy": "policy.json", "seats": {"work": seat}}),
    };
    std::fs::write(dir.join("policy.json"), table.to_string()).unwrap();
    std::fs::write(dir.join("bundle.json"), bundle.to_string()).unwrap();
    dir.to_path_buf()
}

/// The refusal a resolve gave.
fn refused(result: Result<Resolved, CompileError>) -> CompileError {
    match result {
        Ok(resolved) => panic!("composed {:?}", resolved.roots),
        Err(error) => error,
    }
}

/// A personal recipe extends a recipe on the workspace's project shelf
/// (ruling 3), and its chain records both layers' origins (ruling 5).
/// Without the shelves it reads its own shelf alone, where `fast` is not.
#[test]
fn a_personal_recipe_composes_over_a_project_one() {
    clean_git_config!();
    let shelved = Shelved::new(None);
    let mine = recipe(&shelved.personal.join("mine"), "mine", Some("fast"));
    let resolved = shelved.resolve(&mine).unwrap();
    assert_eq!(resolved.roots, [mine.clone(), shelved.project("fast")]);
    assert_eq!(resolved.origins, [Origin::Personal, Origin::Project]);
    let alone = refused(resolve_unsealed(&mine, &Shelves::default()));
    let unshelved = format!(
        "bundle: {}: extends 'fast', which is not a recipe in {}",
        mine.join("bundle.json").display(),
        shelved.personal.display()
    );
    assert_eq!(alone.to_string(), unshelved);
}

/// A personal recipe extends the folder's own bundle through the reserved
/// name `.brokkr` (ruling 3), which no other layer may name, and refuses
/// it where the workspace has no folder.
#[test]
fn a_personal_recipe_composes_over_the_folders_bundle() {
    clean_git_config!();
    let shelved = Shelved::new(None);
    let mine = recipe(&shelved.personal.join("mine"), "mine", Some(".brokkr"));
    let resolved = shelved.resolve(&mine).unwrap();
    let folder = shelved.repo.join(".brokkr");
    assert_eq!(resolved.roots, [mine.clone(), folder]);
    assert_eq!(resolved.origins, [Origin::Personal, Origin::Folder]);

    let team = recipe(&shelved.project("team"), "team", Some(".brokkr"));
    let reserved = format!(
        "bundle: {}: 'extends' value '.brokkr' is not a recipe name; names match \
         ^[a-z0-9][a-z0-9-]*$ and are refused before any path is built",
        team.join("bundle.json").display()
    );
    assert_eq!(refused(shelved.resolve(&team)).to_string(), reserved);

    let bare = shelved.repo.with_file_name("bare");
    crate::places::places_tests::test_repository(&bare);
    let shelves = Shelves::of(
        &Layout::discover(&bare).unwrap(),
        None,
        Some(shelved.personal),
    );
    let unfoldered = refused(resolve_unsealed(&mine, &shelves));
    let refusal = shelf_refusal(&unfoldered);
    assert!(matches!(refusal, ShelfRefusal::Unfoldered { layer } if *layer == mine));
    let text = format!(
        "bundle: the personal recipe at {} extends '.brokkr', the folder's own bundle, but this \
         workspace has no .brokkr folder (#697): run it from a folder repository, or extend a \
         recipe on a shelf",
        mine.display()
    );
    assert_eq!(unfoldered.to_string(), text);
}

/// A project recipe naming a recipe only the personal shelf holds refuses
/// with its own text, naming both paths (ruling 3).
#[test]
fn a_project_recipe_naming_a_personal_one_refuses() {
    clean_git_config!();
    let shelved = Shelved::new(None);
    recipe(&shelved.personal.join("mine"), "mine", None);
    let team = recipe(&shelved.project("team"), "team", Some("mine"));
    let refusal = refused(shelved.resolve(&team));
    let named = shelved.naming_mine(&team);
    let project = shelf_refusal(&refusal);
    assert!(matches!(project, ShelfRefusal::ProjectNamesPersonal(by) if *by == named));
    let text = format!(
        "bundle: the project recipe at {} extends 'mine', which stands only on the personal \
         shelf, at {} (#697): a project recipe never extends a personal one, since teammates and \
         CI could not compile it, so put 'mine' on the project shelf or extend a recipe there",
        team.display(),
        shelved.personal.join("mine").display()
    );
    assert_eq!(refusal.to_string(), text);
}

/// The folder's bundle naming a recipe only the personal shelf holds
/// refuses with its own text, naming both paths (ruling 3).
#[test]
fn the_folders_bundle_naming_a_personal_recipe_refuses() {
    clean_git_config!();
    let shelved = Shelved::new(Some("mine"));
    recipe(&shelved.personal.join("mine"), "mine", None);
    let folder = shelved.repo.join(".brokkr");
    let refusal = refused(shelved.resolve(&folder));
    let named = shelved.naming_mine(&folder);
    let bundle = shelf_refusal(&refusal);
    assert!(matches!(bundle, ShelfRefusal::FolderNamesPersonal(by) if *by == named));
    let text = format!(
        "bundle: the folder's bundle at {} extends 'mine', which stands only on the personal \
         shelf, at {} (#697): the folder's bundle never extends a personal recipe, since \
         teammates and CI could not compile it, so put 'mine' on the project shelf or extend a \
         recipe there",
        folder.display(),
        shelved.personal.join("mine").display()
    );
    assert_eq!(refusal.to_string(), text);
}

/// A personal recipe's base on both shelves refuses as `--recipe` does
/// (ruling 2), before the base on either is read: the personal `fast`
/// holds no bundle, and it is still the shadowing that refuses. A project
/// recipe reads its own shelf alone, so the same name composes there.
#[test]
fn a_base_on_both_shelves_refuses_before_either_is_read() {
    clean_git_config!();
    let shelved = Shelved::new(None);
    std::fs::create_dir(shelved.personal.join("fast")).unwrap();
    let mine = recipe(&shelved.personal.join("mine"), "mine", Some("fast"));
    let refusal = refused(shelved.resolve(&mine));
    let both = (shelved.project("fast"), shelved.personal.join("fast"));
    assert!(shadows(shelf_refusal(&refusal), ("fast", &both.0, &both.1)));
    let text = format!(
        "bundle: recipe 'fast' stands on both the project shelf, at {}, and the personal shelf, \
         at {} (#697): neither shelf wins, so rename or remove one of them",
        shelved.project("fast").display(),
        shelved.personal.join("fast").display()
    );
    assert_eq!(refusal.to_string(), text);

    let team = recipe(&shelved.project("team"), "team", Some("fast"));
    let resolved = shelved.resolve(&team).unwrap();
    assert_eq!(resolved.roots, [team, shelved.project("fast")]);
    assert_eq!(resolved.origins, [Origin::Project, Origin::Project]);
}

/// A personal recipe reads its own shelf where the name stands there alone,
/// or where the workspace has no project shelf; a base on neither shelf is
/// no recipe on its own; and a project recipe's missing base, with no
/// personal shelf placed, refuses as it always did.
#[test]
fn a_personal_recipe_reads_its_own_shelf() {
    let shelved = Shelved::new(None);
    let own = recipe(&shelved.personal.join("own"), "own", None);
    let mine = recipe(&shelved.personal.join("mine"), "mine", Some("own"));
    clean_git_config!();
    let resolved = shelved.resolve(&mine).unwrap();
    assert_eq!(resolved.roots, [mine.clone(), own.clone()]);
    assert_eq!(resolved.origins, [Origin::Personal, Origin::Personal]);
    std::fs::remove_dir_all(shelved.repo.join(".brokkr/recipes")).unwrap();
    let resolved = shelved.resolve(&mine).unwrap();
    assert_eq!(resolved.roots, [mine, own]);

    let lost = recipe(&shelved.personal.join("lost"), "lost", Some("gone"));
    let nowhere = format!(
        "bundle: {}: extends 'gone', which is not a recipe in {}",
        lost.join("bundle.json").display(),
        shelved.personal.display()
    );
    assert_eq!(refused(shelved.resolve(&lost)).to_string(), nowhere);

    let team = recipe(&shelved.project("team"), "team", Some("gone"));
    let unplaced = Shelves::of(&Layout::discover(&shelved.repo).unwrap(), None, None);
    let missing = format!(
        "bundle: {}: extends 'gone', which is not a recipe in {}",
        team.join("bundle.json").display(),
        shelved.repo.join(".brokkr/recipes").display()
    );
    assert_eq!(
        refused(resolve_unsealed(&team, &unplaced)).to_string(),
        missing
    );
}

/// A shelf that cannot be read is never read as an empty one (#697 ruling
/// 2): the project shelf, read for a personal layer's base on its own
/// shelf, and the personal one, read for a project layer's missing base,
/// each a link to nothing, a looping link or a file, refuse in `--recipe`'s
/// words, typed, naming the shelf; so does a selected project shelf
/// climbing by `..` out of what does not stand (#726). A shelf nothing
/// stands at, no entry and no link, is an empty one: the personal base is
/// taken, or the project's missing base is no recipe.
#[test]
fn an_unreadable_shelf_refuses_every_read_of_it() {
    clean_git_config!();
    let shelved = Shelved::new(None);
    let own = recipe(&shelved.personal.join("own"), "own", None);
    let mine = recipe(&shelved.personal.join("mine"), "mine", Some("own"));
    let team = recipe(&shelved.project("team"), "team", Some("gone"));
    let project = shelved.repo.join(".brokkr/recipes");
    let missing = format!(
        "bundle: {}: extends 'gone', which is not a recipe in {}",
        team.join("bundle.json").display(),
        project.display()
    );
    let dangles: fn(&Path) = |shelf| std::os::unix::fs::symlink("gone", shelf).unwrap();
    let loops: fn(&Path) = |shelf| std::os::unix::fs::symlink("recipes", shelf).unwrap();
    let file: fn(&Path) = |shelf| std::fs::write(shelf, "").unwrap();
    let sides = [
        (
            Origin::Project,
            &project,
            &mine,
            Ok(vec![mine.clone(), own]),
        ),
        (Origin::Personal, &shelved.personal, &team, Err(missing)),
    ];
    for (origin, shelf, leaf, empty) in sides {
        let kept = shelf.with_extension("kept");
        std::fs::rename(shelf, &kept).unwrap();
        for plant in [dangles, loops, file] {
            plant(shelf);
            let cause = std::fs::canonicalize(shelf)
                .map_or_else(|cause| cause.kind(), |_| std::io::ErrorKind::NotADirectory);
            let refusal = refused(shelved.resolve(leaf));
            let unread = matches!(shelf_refusal(&refusal),
                ShelfRefusal::Unread(PlacesError::ShelfUnread { origin: of, shelf: named, at, cause: why })
                    if (*of, named, at, why.kind()) == (origin, shelf, shelf, cause));
            assert!(unread, "{refusal}");
            std::fs::remove_file(shelf).unwrap();
        }
        let read = shelved.resolve(leaf);
        let read = read
            .map(|resolved| resolved.roots)
            .map_err(|e| e.to_string());
        assert_eq!(read, empty, "{origin}");
        std::fs::rename(&kept, shelf).unwrap();
    }

    let (spelled, at) = (shelved.repo.join("none/../own"), shelved.repo.clone());
    let climbing = shelved.selecting(Some(&spelled));
    let refusal = refused(resolve_unsealed(&mine, &climbing));
    let climbs = matches!(shelf_refusal(&refusal),
        ShelfRefusal::Unread(PlacesError::ShelfClimbs { origin: Origin::Project, shelf, at: by })
            if (shelf, by) == (&spelled, &at));
    assert!(climbs, "{refusal}");
}

/// A personal layer's base whose entry cannot be read refuses with its
/// path and cause, never read as absent (#697 ruling 2): on the personal
/// shelf, a name too long for a directory entry, and on a readable project
/// shelf, a name whose path there is too long for the system to look up,
/// where beside the personal recipe it is not.
#[test]
fn an_unreadable_base_entry_refuses_on_either_shelf() {
    let shelved = Shelved::new(None);
    let unreadable = |leaf: &Path, shelves: &Shelves, at: &Path| {
        let cause = std::fs::symlink_metadata(at).unwrap_err();
        let text = format!("bundle: reading {}: {cause}", at.display());
        assert_eq!(refused(resolve_unsealed(leaf, shelves)).to_string(), text);
    };
    let long = "a".repeat(256);
    let mine = recipe(&shelved.personal.join("mine"), "mine", Some(&long));
    unreadable(&mine, &shelved.shelves(), &shelved.personal.join(&long));

    let base = "b".repeat(200);
    let deep = recipe(&shelved.personal.join("deep"), "deep", Some(&base));
    let mut shelf = shelved.repo.clone();
    while std::fs::symlink_metadata(shelf.join(&base)).is_err_and(|e| e.kind() == NotFound) {
        shelf.push("c".repeat(100));
    }
    unreadable(&deep, &shelved.selecting(Some(&shelf)), &shelf.join(&base));
}

/// A folder's bundle compiled by its directory, with no shelves, whose
/// root cannot be judged, a `recipes` beside it that loops, refuses with
/// that entry's path and cause, never read as a library layer (#697
/// ruling 5).
#[test]
fn a_folder_bundle_beside_an_unreadable_root_refuses() {
    let shelved = Shelved::new(None);
    let recipes = shelved.repo.join("recipes");
    std::os::unix::fs::symlink("recipes", &recipes).unwrap();
    let cause = std::fs::read_dir(&recipes).unwrap_err();
    let folder = shelved.repo.join(".brokkr");
    let text = format!("bundle: reading {}: {cause}", recipes.display());
    let unjudged = refused(resolve_unsealed(&folder, &Shelves::default()));
    assert_eq!(unjudged.to_string(), text);
}

/// The project shelf a compile is given is the one it composes across
/// (#697 ruling 3), in place of the workspace's own: a personal layer's
/// base is read there, a name on it and on the personal shelf refuses,
/// and a recipe on it naming a personal one refuses as a project's. On the
/// workspace's own shelf, which holds no `quick`, each reads otherwise.
#[test]
fn a_selected_project_shelf_is_the_one_composed_across() {
    clean_git_config!();
    let shelved = Shelved::new(None);
    let own = shelved.repo.join("own");
    let quick = recipe(&own.join("quick"), "quick", None);
    let mine = recipe(&shelved.personal.join("mine"), "mine", Some("quick"));
    let selected = shelved.selecting(Some(&own));
    let resolved = resolve_unsealed(&mine, &selected).unwrap();
    assert_eq!(resolved.roots, [mine.clone(), quick]);
    assert_eq!(resolved.origins, [Origin::Personal, Origin::Project]);
    let unselected = |leaf: &Path| refused(shelved.resolve(leaf)).to_string();
    let not_in = |leaf: &Path, base: &str, library: &Path| {
        let file = leaf.join("bundle.json");
        let (file, library) = (file.display(), library.display());
        format!("bundle: {file}: extends '{base}', which is not a recipe in {library}")
    };
    assert_eq!(unselected(&mine), not_in(&mine, "quick", &shelved.personal));

    recipe(&shelved.personal.join("only"), "only", None);
    let team = recipe(&own.join("team"), "team", Some("only"));
    let refusal = refused(resolve_unsealed(&team, &selected));
    let (base, personal) = ("only".into(), shelved.personal.join("only"));
    let layer = team.clone();
    let named = Named {
        layer,
        base,
        personal,
    };
    let project = shelf_refusal(&refusal);
    assert!(matches!(project, ShelfRefusal::ProjectNamesPersonal(by) if *by == named));
    assert_eq!(unselected(&team), not_in(&team, "only", &own));

    std::fs::create_dir(shelved.personal.join("quick")).unwrap();
    let refusal = refused(resolve_unsealed(&mine, &selected));
    let both = (own.join("quick"), shelved.personal.join("quick"));
    assert!(shadows(
        shelf_refusal(&refusal),
        ("quick", &both.0, &both.1)
    ));
}

/// Each origin's one word, which the journal records and every surface
/// prints.
#[test]
fn each_origin_has_one_word() {
    let words = [
        Origin::Personal,
        Origin::Project,
        Origin::Folder,
        Origin::Library,
    ]
    .map(|origin| origin.to_string());
    assert_eq!(words, ["personal", "project", "folder", "library"]);
}
