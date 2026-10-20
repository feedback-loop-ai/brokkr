//! Composition across shelves (#697 rulings 3 and 5): a folder repository
//! and a personal shelf beside it, each recipe a small bundle on disk, and
//! an assertion about the chain the resolver reads or the refusal it gives.

use std::path::{Path, PathBuf};

use serde_json::json;

use super::{Named, Origin, ShelfRefusal, Shelves};
use crate::bundle::compose::{resolve_unsealed, Resolved};
use crate::bundle::CompileError;
use crate::places::Layout;

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
        std::fs::create_dir_all(repo.join(".git")).unwrap();
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
        Shelves::of(
            &Layout::discover(&self.repo).unwrap(),
            Some(self.personal.clone()),
        )
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
    std::fs::create_dir_all(bare.join(".git")).unwrap();
    let shelves = Shelves::of(&Layout::discover(&bare).unwrap(), Some(shelved.personal));
    let unfoldered = refused(resolve_unsealed(&mine, &shelves));
    let layer = mine.clone();
    assert_eq!(
        shelf_refusal(&unfoldered),
        &ShelfRefusal::Unfoldered { layer }
    );
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
    let shelved = Shelved::new(None);
    recipe(&shelved.personal.join("mine"), "mine", None);
    let team = recipe(&shelved.project("team"), "team", Some("mine"));
    let refusal = refused(shelved.resolve(&team));
    let named = ShelfRefusal::ProjectNamesPersonal(shelved.naming_mine(&team));
    assert_eq!(shelf_refusal(&refusal), &named);
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
    let shelved = Shelved::new(Some("mine"));
    recipe(&shelved.personal.join("mine"), "mine", None);
    let folder = shelved.repo.join(".brokkr");
    let refusal = refused(shelved.resolve(&folder));
    let named = ShelfRefusal::FolderNamesPersonal(shelved.naming_mine(&folder));
    assert_eq!(shelf_refusal(&refusal), &named);
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
    let shelved = Shelved::new(None);
    std::fs::create_dir(shelved.personal.join("fast")).unwrap();
    let mine = recipe(&shelved.personal.join("mine"), "mine", Some("fast"));
    let refusal = refused(shelved.resolve(&mine));
    let shadowed = ShelfRefusal::Shadowed {
        name: "fast".into(),
        project: shelved.project("fast"),
        personal: shelved.personal.join("fast"),
    };
    assert_eq!(shelf_refusal(&refusal), &shadowed);
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
    let unplaced = Shelves::of(&Layout::discover(&shelved.repo).unwrap(), None);
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

/// A project shelf that cannot be read is never read as an empty one: a
/// personal recipe's base there refuses with the entry's own error.
#[test]
fn an_unreadable_project_shelf_refuses_a_personal_base() {
    let shelved = Shelved::new(None);
    let shelf = shelved.repo.join(".brokkr/recipes");
    std::fs::remove_dir_all(&shelf).unwrap();
    std::os::unix::fs::symlink("recipes", &shelf).unwrap();
    let mine = recipe(&shelved.personal.join("mine"), "mine", Some("fast"));
    let fast = shelf.join("fast");
    let cause = std::fs::symlink_metadata(&fast).unwrap_err();
    let text = format!("bundle: reading {}: {cause}", fast.display());
    assert_eq!(refused(shelved.resolve(&mine)).to_string(), text);
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
