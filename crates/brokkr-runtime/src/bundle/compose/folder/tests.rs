//! A seat that is the folder's own (decision 0091 ruling 1): a folder
//! repository on disk, a recipe on its shelf whose verify says
//! `"from": "folder"`, and an assertion about the seat the compile makes of
//! it, the layer it pins, or the refusal it gives, by its variant and its
//! text, each pinned once here.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::{FolderSeatRefusal, Shape, Side};
use crate::argv::Executable;
use crate::bundle::{Bundle, CompileError, Invocation, SeatBody};
use crate::places::{Layout, Origin, Shelves};
use brokkr_core::realms::Boundary;

/// The verify script a folder and a recipe each hold, saying whose it is.
fn script(whose: &str) -> String {
    format!("#!/bin/sh\necho {whose}\n")
}

/// An exec gate dispatching `./scripts/verify-seat.sh`, as `init` writes
/// one, under `hands`.
fn verify_seat(limits: Value, hands: Value) -> Value {
    json!({
        "class": "gate",
        "results": ["pass", "fail"],
        "limits": limits,
        "driver": {"command": [
            "{brokkr}", "driver", "exec", "--", "bash", "./scripts/verify-seat.sh", "{prompt_file}"
        ]},
        "hands": hands,
    })
}

/// The folder's own hands, which bind nothing.
fn folder_hands() -> Value {
    json!({"kind": "workspace", "network": false, "binds": []})
}

/// The recipe's own hands, which bind the toolchain.
fn recipe_hands() -> Value {
    json!({"kind": "workspace", "network": false,
           "binds": [{"path": "~/.rustup", "mode": "ro", "mask": []}]})
}

/// The recipe's verify seat: the folder's own, with limits of its own.
pub(crate) fn from_folder() -> Value {
    let limits = json!({"max_attempts": 3, "timeout_seconds": 60});
    let mut seat = verify_seat(limits, recipe_hands());
    seat["from"] = json!("folder");
    seat
}

/// One verify phase, then stop: a table that ships nothing, so no path
/// bypasses the protected phase.
fn policy() -> Value {
    json!({
        "schema": "forge.phase-machine/v1",
        "phases": ["verify", "stop"],
        "initial": "verify",
        "terminal": ["stop"],
        "rules": [
            {"id": "PASS", "from": "verify", "result": "pass", "next": "stop", "reason": "p"},
            {"id": "FAIL", "from": "verify", "result": "fail", "next": "stop", "reason": "f"},
        ],
    })
}

/// A bundle named `name` over `seats`, with the one-phase table.
fn bundle(name: &str, seats: Value) -> Value {
    json!({"name": name, "policy": "policy.json", "protected_phase": "verify", "seats": seats})
}

/// Write `bundle` at `dir` with its table and its own verify script, and
/// return `dir`, canonical.
fn layer(dir: &Path, bundle: &Value, whose: &str) -> PathBuf {
    std::fs::create_dir_all(dir.join("scripts")).unwrap();
    std::fs::write(dir.join("scripts/verify-seat.sh"), script(whose)).unwrap();
    std::fs::write(dir.join("policy.json"), policy().to_string()).unwrap();
    std::fs::write(dir.join("bundle.json"), bundle.to_string()).unwrap();
    dir.canonicalize().unwrap()
}

/// A folder repository whose folder bundle, `starter`, declares its own
/// verify seat, and the personal shelf beside it, both canonical.
pub(crate) struct Folder {
    _dir: tempfile::TempDir,
    repo: PathBuf,
    personal: PathBuf,
}

impl Folder {
    pub(crate) fn new() -> Folder {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let (repo, personal) = (root.join("repo"), root.join("config/recipes"));
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(&personal).unwrap();
        let folder = Folder {
            _dir: dir,
            repo,
            personal,
        };
        let limits = json!({"max_attempts": 2, "timeout_seconds": 3600});
        folder.declare(verify_seat(limits, folder_hands()));
        folder
    }

    /// The folder bundle's directory.
    pub(crate) fn folder(&self) -> PathBuf {
        self.repo.join(".brokkr")
    }

    /// The folder bundle's declaring document.
    fn file(&self) -> PathBuf {
        self.folder().join("bundle.json")
    }

    /// Rewrite the folder bundle with `verify` as its verify seat.
    fn declare(&self, verify: Value) {
        self.write_folder(&bundle("starter", json!({"verify": verify})));
    }

    /// Rewrite the folder bundle as `document`.
    fn write_folder(&self, document: &Value) {
        layer(&self.folder(), document, "folder");
    }

    /// The recipe `fixture` on the folder's shelf, over `seats`.
    pub(crate) fn recipe(&self, seats: Value) -> PathBuf {
        let dir = self.folder().join("recipes/fixture");
        layer(&dir, &bundle("fixture", seats), "recipe")
    }

    /// The shelves a compile from the repository reads.
    pub(crate) fn shelves(&self) -> Shelves {
        let layout = Layout::discover(&self.repo).unwrap();
        Shelves::of(&layout, None, Some(self.personal.clone()))
    }
}

/// The bundle at `leaf` compiled across `shelves` under `harness`, with the
/// shipped adapters, no agent library, and an engine that is not there, so
/// a spawn of it that passes every check fails to start rather than runs.
pub(crate) fn compile(leaf: &Path, shelves: &Shelves) -> Result<Bundle, CompileError> {
    let adapters = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters");
    let roots = (Path::new("/nonexistent/agents"), adapters.as_path());
    let unmapped = crate::capabilities::UNMAPPED;
    let none = crate::capabilities::CapabilityContext::no_grants(unmapped, leaf);
    let engine = Executable(Some(PathBuf::from("/nonexistent/brokkr")));
    let law = Boundary::Harness;
    Bundle::compile_as(&engine, (leaf, shelves), roots, None, None, law, &none)
}

/// The folder seat refusal a compile gave, with its whole text.
fn refusal(result: Result<Bundle, CompileError>) -> (FolderSeatRefusal, String) {
    let error = result.map(|bundle| bundle.roots).unwrap_err();
    let text = error.to_string();
    match error {
        CompileError::Folder(refusal) => (refusal, text),
        _ => panic!("not a folder seat refusal: {text}"),
    }
}

/// The argv of a compiled single seat.
fn argv(bundle: &Bundle, seat: &str) -> Vec<String> {
    match &bundle.seats[seat].body {
        SeatBody::Invocation(Invocation::Single { command, .. }) => command.clone(),
        other => panic!("{seat} is a single site: {other:?}"),
    }
}

/// The verify script under `dir`, as its argv names it.
fn script_at(dir: &Path) -> String {
    dir.join("scripts/verify-seat.sh").display().to_string()
}

/// In a folder, the seat runs the folder's script in the folder's box,
/// ruled by the recipe's results and limits, and the folder's layer is
/// pinned beside the leaf's: under `@folder/starter`, last in the roots,
/// and recorded as a layer of origin `folder`. Nothing is composed.
#[test]
fn a_folder_seat_takes_the_folders_driver_and_hands_and_pins_its_layer() {
    let folder = Folder::new();
    let leaf = folder.recipe(json!({"verify": from_folder()}));
    let bundle = compile(&leaf, &folder.shelves()).unwrap();
    let dir = folder.folder().canonicalize().unwrap();
    assert_eq!(argv(&bundle, "verify")[5], script_at(&dir));
    assert_eq!(bundle.manifest["hands"]["verify"], folder_hands());
    let seat = &bundle.seats["verify"];
    assert_eq!(seat.results, ["pass", "fail"]);
    assert_eq!(
        (seat.limits.max_attempts, seat.limits.timeout_seconds),
        (3, 60)
    );
    assert_eq!(bundle.roots, [leaf.clone(), dir.clone()]);
    assert_eq!(bundle.origins, [Origin::Project, Origin::Folder]);
    assert!(bundle.chain.is_empty());
    let pinned = bundle.folder.clone().expect("the folder layer is pinned");
    assert_eq!((pinned.name.as_str(), &pinned.dir), ("starter", &dir));
    let files = bundle.manifest["files"].as_object().unwrap();
    assert_eq!(files["@folder/starter"], json!(pinned.digest));
    assert!(pinned.files.contains_key("scripts/verify-seat.sh"));
    let reserved: Vec<&String> = files.keys().filter(|key| key.starts_with('@')).collect();
    assert_eq!(reserved, ["@folder/starter"]);
    let layers = bundle.layers().unwrap();
    let origins: Vec<(Origin, String)> = layers.into_iter().map(|l| (l.origin, l.path)).collect();
    let paths = [leaf.display().to_string(), dir.display().to_string()];
    assert_eq!(
        origins,
        [
            (Origin::Project, paths[0].clone()),
            (Origin::Folder, paths[1].clone())
        ]
    );
}

/// Where no folder stands, the recipe's own complete declaration is what
/// compiles: its script, its box, no folder layer and no `@folder/` key.
#[test]
fn with_no_folder_the_recipes_own_seat_stands() {
    let folder = Folder::new();
    let leaf = folder.recipe(json!({"verify": from_folder()}));
    let bundle = compile(&leaf, &Shelves::default()).unwrap();
    assert_eq!(argv(&bundle, "verify")[5], script_at(&leaf));
    assert_eq!(bundle.manifest["hands"]["verify"], recipe_hands());
    assert_eq!(bundle.roots, [leaf]);
    assert_eq!(bundle.folder, None);
    let files = bundle.manifest["files"].as_object().unwrap();
    assert!(files.keys().all(|key| !key.starts_with('@')), "{files:?}");
}

/// An edit to the folder's script or to its `bundle.json` moves the
/// recipe's digest through its `@folder/starter` key alone, which is what
/// resume's comparison names.
#[test]
fn an_edit_to_the_folder_moves_the_recipes_digest_by_its_folder_key() {
    let folder = Folder::new();
    let leaf = folder.recipe(json!({"verify": from_folder()}));
    let compiled = || compile(&leaf, &folder.shelves()).unwrap();
    let before = compiled();
    let moved = |after: &Bundle| -> Vec<String> {
        let (was, now) = (&before.manifest["files"], &after.manifest["files"]);
        let keys = now.as_object().unwrap().keys();
        keys.filter(|key| was[key.as_str()] != now[key.as_str()])
            .cloned()
            .collect()
    };
    std::fs::write(folder.folder().join("scripts/verify-seat.sh"), "edited\n").unwrap();
    let edited = compiled();
    assert_ne!(edited.manifest_digest(), before.manifest_digest());
    assert_eq!(moved(&edited), ["@folder/starter"]);
    std::fs::write(
        folder.folder().join("scripts/verify-seat.sh"),
        script("folder"),
    )
    .unwrap();
    assert_eq!(compiled().manifest_digest(), before.manifest_digest());
    folder.declare(verify_seat(json!({"max_attempts": 1}), folder_hands()));
    let declared = compiled();
    assert_ne!(declared.manifest_digest(), before.manifest_digest());
    assert_eq!(moved(&declared), ["@folder/starter"]);
}

/// A recipe file under the reserved `@folder/` name refuses, as one under
/// `@compose/` does: the key is computed, never read from disk.
#[test]
fn a_recipe_file_in_the_folder_namespace_refuses() {
    let folder = Folder::new();
    let leaf = folder.recipe(json!({"verify": from_folder()}));
    std::fs::create_dir_all(leaf.join("@folder")).unwrap();
    std::fs::write(leaf.join("@folder/x"), "forged\n").unwrap();
    let refused = compile(&leaf, &Shelves::default()).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "bundle: bundle file '@folder/x' uses the reserved '@folder/' namespace; composition \
         provenance is computed by the resolver, never supplied by a bundle"
    );
}

/// `from` is a closed vocabulary of one word: any other refuses, in a
/// folder or out of one.
#[test]
fn a_from_other_than_folder_refuses() {
    let folder = Folder::new();
    let mut seat = from_folder();
    seat["from"] = json!("repo");
    let leaf = folder.recipe(json!({"verify": seat}));
    for shelves in [folder.shelves(), Shelves::default()] {
        let (variant, text) = refusal(compile(&leaf, &shelves));
        let word = "\"repo\"".to_string();
        assert_eq!(
            variant,
            FolderSeatRefusal::Word {
                seat: "verify".into(),
                word
            }
        );
        assert_eq!(
            text,
            "bundle: seat 'verify' says \"from\": \"repo\", and \"folder\" is the one word it \
             takes (decision 0091): any other is refused rather than read as no source at all \
             (decision 0071 ruling 2)"
        );
    }
}

/// Refusal 4: a seat that is the folder's own still declares the driver a
/// compile with no folder runs, in a folder or out of one.
#[test]
fn a_folder_seat_with_no_driver_of_its_own_refuses() {
    let folder = Folder::new();
    let mut seat = from_folder();
    seat.as_object_mut().unwrap().remove("driver");
    let leaf = folder.recipe(json!({"verify": seat}));
    for shelves in [folder.shelves(), Shelves::default()] {
        let (variant, text) = refusal(compile(&leaf, &shelves));
        assert_eq!(
            variant,
            FolderSeatRefusal::NoOwnDriver {
                seat: "verify".into()
            }
        );
        assert_eq!(
            text,
            "bundle: seat 'verify' says \"from\": \"folder\" and declares no driver of its own, \
             which a compile with no folder runs (decision 0091)"
        );
    }
}

/// The sixth refusal: a key the seat is not read for, written by the
/// recipe (in a folder or out of one) or by the folder's declaration.
#[test]
fn a_key_a_folder_seat_is_not_read_for_refuses_on_either_side() {
    let folder = Folder::new();
    let mut seat = from_folder();
    seat["role"] = json!("roles/verify.md");
    let leaf = folder.recipe(json!({"verify": seat}));
    for shelves in [folder.shelves(), Shelves::default()] {
        let (variant, text) = refusal(compile(&leaf, &shelves));
        let (seat, side, key) = ("verify".into(), Side::Recipe, "role".into());
        assert_eq!(variant, FolderSeatRefusal::Unread { seat, side, key });
        assert_eq!(
            text,
            "bundle: seat 'verify' is the folder's own (decision 0091), and the recipe's \
             declaration writes 'role', which a seat the folder supplies is never read for: it \
             takes class, results, limits, driver and hands alone, so any other key is refused \
             rather than ignored (decision 0004)"
        );
    }
    let leaf = folder.recipe(json!({"verify": from_folder()}));
    let mut declared = verify_seat(json!({}), folder_hands());
    declared["inputs"] = json!(["notes.md"]);
    folder.declare(declared);
    let (variant, text) = refusal(compile(&leaf, &folder.shelves()));
    let (seat, side, key) = ("verify".into(), Side::Folder, "inputs".into());
    assert_eq!(variant, FolderSeatRefusal::Unread { seat, side, key });
    assert!(
        text.contains("and the folder's declaration writes 'inputs'"),
        "{text}"
    );
}

/// Refusal 1: the folder bundle declares no such seat.
#[test]
fn a_folder_that_declares_no_such_seat_refuses() {
    let folder = Folder::new();
    folder.write_folder(&json!({"name": "starter", "policy": "policy.json", "seats": {}}));
    let leaf = folder.recipe(json!({"verify": from_folder()}));
    let (variant, text) = refusal(compile(&leaf, &folder.shelves()));
    let file = folder.file();
    let undeclared = FolderSeatRefusal::Undeclared {
        seat: "verify".into(),
        file: file.clone(),
    };
    assert_eq!(variant, undeclared);
    let expected = format!(
        "bundle: seat 'verify' is the folder's own (decision 0091), and {} declares no seat \
         'verify'",
        file.display()
    );
    assert_eq!(text, expected);
}

/// Refusal 3: the folder's seat runs something other than its own pinned
/// script, whether a raw process or an exec of text.
#[test]
fn a_folder_seat_that_is_not_an_exec_of_its_own_script_refuses() {
    let folder = Folder::new();
    let leaf = folder.recipe(json!({"verify": from_folder()}));
    let raw = json!(["bash", "./scripts/verify-seat.sh"]);
    let text = json!(["{brokkr}", "driver", "exec", "--", "bash", "-c", "true"]);
    for command in [raw, text] {
        let mut declared = verify_seat(json!({}), folder_hands());
        declared["driver"]["command"] = command;
        folder.declare(declared);
        let (variant, text) = refusal(compile(&leaf, &folder.shelves()));
        assert_eq!(
            variant,
            FolderSeatRefusal::NotItsOwnScript {
                seat: "verify".into()
            }
        );
        assert_eq!(
            text,
            "bundle: seat 'verify' is the folder's own (decision 0091), and the folder's is not \
             a `{brokkr} driver exec` dispatch of its own pinned script"
        );
    }
}

/// Refusal 2: the folder's seat is another class, or has other results,
/// as written and in order.
#[test]
fn a_folder_seat_of_another_shape_refuses() {
    let folder = Folder::new();
    let leaf = folder.recipe(json!({"verify": from_folder()}));
    let mut declared = verify_seat(json!({}), folder_hands());
    declared["results"] = json!(["fail", "pass"]);
    folder.declare(declared);
    let (variant, text) = refusal(compile(&leaf, &folder.shelves()));
    let shape = |results: &str| Shape {
        class: "\"gate\"".into(),
        results: results.into(),
    };
    let (theirs, ours) = (shape("[\"fail\",\"pass\"]"), shape("[\"pass\",\"fail\"]"));
    let differs = FolderSeatRefusal::Differs {
        seat: "verify".into(),
        folder: Box::new(theirs),
        recipe: Box::new(ours),
    };
    assert_eq!(variant, differs);
    assert_eq!(
        text,
        "bundle: seat 'verify' is the folder's own (decision 0091), and the folder's is \"gate\" \
         with results [\"fail\",\"pass\"] where the recipe's is \"gate\" with results \
         [\"pass\",\"fail\"]"
    );
}

/// Refusal 5: a folder bundle never takes a seat from itself through its
/// base, compiled as the leaf or as a personal recipe's `.brokkr` base,
/// with a folder or with none.
#[test]
fn a_folder_bundle_taking_its_bases_folder_seat_refuses() {
    let folder = Folder::new();
    folder.recipe(json!({"verify": from_folder()}));
    folder.write_folder(&json!({"name": "starter", "extends": "fixture"}));
    let mine = folder.personal.join("mine");
    let mine = layer(
        &mine,
        &json!({"name": "mine", "extends": ".brokkr"}),
        "mine",
    );
    let file = folder.file();
    let expected = format!(
        "bundle: {}: seat 'verify' is the folder's own by its base 'fixture' (decision 0091), and \
         a folder declares its own: declare seat 'verify' in {}",
        file.display(),
        file.display()
    );
    let leaf = folder.folder();
    for (leaf, shelves) in [
        (&leaf, folder.shelves()),
        (&leaf, Shelves::default()),
        (&mine, folder.shelves()),
    ] {
        let (variant, text) = refusal(compile(leaf, &shelves));
        let (seat, base) = ("verify".into(), "fixture".into());
        let through = FolderSeatRefusal::ThroughItsBase {
            file: file.clone(),
            seat,
            base,
        };
        assert_eq!(variant, through);
        assert_eq!(text, expected);
    }
}

/// Where the folder is already a layer of the chain, its pin is reused:
/// a personal recipe over `.brokkr` whose seat says `from`, and the folder
/// bundle's own seat that says it, each take the folder's declaration with
/// no `@folder/` key and no layer added.
#[test]
fn a_folder_already_composed_is_pinned_as_its_layer() {
    let folder = Folder::new();
    let dir = folder.folder().canonicalize().unwrap();
    let mine = folder.personal.join("mine");
    let seats = json!({"override": {"seats": ["verify"]}, "seats": {"verify": from_folder()}});
    let mut document = bundle("mine", seats["seats"].clone());
    document["extends"] = json!(".brokkr");
    document["override"] = seats["override"].clone();
    for key in ["policy", "protected_phase"] {
        document.as_object_mut().unwrap().remove(key);
    }
    let mine = layer(&mine, &document, "mine");
    let mut own = from_folder();
    own["limits"] = json!({"max_attempts": 2, "timeout_seconds": 3600});
    own["hands"] = folder_hands();
    for (leaf, roots) in [
        (&mine, vec![mine.clone(), dir.clone()]),
        (&dir, vec![dir.clone()]),
    ] {
        let bundle = compile(leaf, &folder.shelves()).unwrap();
        assert_eq!(argv(&bundle, "verify")[5], script_at(&dir));
        assert_eq!(bundle.manifest["hands"]["verify"], folder_hands());
        assert_eq!(bundle.roots, roots);
        assert_eq!(bundle.folder, None);
        let files = bundle.manifest["files"].as_object().unwrap();
        assert!(
            files.keys().all(|key| !key.starts_with("@folder/")),
            "{files:?}"
        );
        folder.declare(own.clone());
    }
}
