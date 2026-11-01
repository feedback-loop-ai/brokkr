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
use crate::places::places_tests::clean_git_config;
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
        crate::places::places_tests::test_repository(&repo);
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
    compile_under(Boundary::Harness, leaf, shelves)
}

/// [`compile`] under the boundary `law`.
fn compile_under(law: Boundary, leaf: &Path, shelves: &Shelves) -> Result<Bundle, CompileError> {
    compile_in(None, law, leaf, shelves)
}

/// [`compile_under`] hiring from the agent library and operator
/// configuration at `library`, or from none.
fn compile_in(
    library: Option<&Path>,
    law: Boundary,
    leaf: &Path,
    shelves: &Shelves,
) -> Result<Bundle, CompileError> {
    let none = || PathBuf::from("/nonexistent/agents");
    let agents = library.map_or_else(none, |root| root.join("agents"));
    let adapters = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters");
    let roots = (agents.as_path(), adapters.as_path());
    let unmapped = crate::capabilities::UNMAPPED;
    let operator = library.unwrap_or(leaf);
    let none = crate::capabilities::CapabilityContext::no_grants(unmapped, operator);
    let engine = Executable(Some(PathBuf::from("/nonexistent/brokkr")));
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
    clean_git_config!();
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

/// Refusal 7: a folder declaration with no hands refuses, as its script
/// would spawn with no walk of the folder's map, even where a hands-less
/// work seat would otherwise compile, and it is read last: a declaration
/// of another shape with no hands refuses as refusal 2.
#[test]
fn a_folder_seat_whose_declaration_has_no_hands_refuses() {
    clean_git_config!();
    let folder = Folder::new();
    let mut declared = verify_seat(json!({}), folder_hands());
    for key in ["class", "hands"] {
        declared.as_object_mut().unwrap().remove(key);
    }
    folder.declare(declared.clone());
    let mut seat = from_folder();
    seat.as_object_mut().unwrap().remove("class");
    let leaf = folder.recipe(json!({"verify": seat}));
    let (variant, text) = refusal(compile(&leaf, &folder.shelves()));
    let file = folder.file();
    let no_hands = FolderSeatRefusal::NoHands {
        seat: "verify".into(),
        file: file.clone(),
    };
    assert_eq!(variant, no_hands);
    assert_eq!(
        text,
        format!(
            "bundle: seat 'verify' is the folder's own (decision 0091), and the folder's declares \
             no hands, so its script would spawn without the walk that checks it: declare its \
             hands in {}",
            file.display()
        )
    );
    declared["class"] = json!("gate");
    folder.declare(declared);
    let (variant, _) = refusal(compile(&leaf, &folder.shelves()));
    let shape = |class: &str| {
        Box::new(Shape {
            class: class.into(),
            results: "[\"pass\",\"fail\"]".into(),
        })
    };
    let differs = FolderSeatRefusal::Differs {
        seat: "verify".into(),
        folder: shape("\"gate\""),
        recipe: shape("null"),
    };
    assert_eq!(variant, differs);
}

/// Where no folder stands, the recipe's own complete declaration is what
/// compiles: its script, its box, no folder layer and no `@folder/` key.
#[test]
fn with_no_folder_the_recipes_own_seat_stands() {
    clean_git_config!();
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

/// Decision 0091 ruling 3 (#717, slice 3c): the library's `fast`, compiled
/// where a folder stands, runs the folder's verify and ship. Each seat's
/// script and box are those the folder declares, as `init` writes them for
/// a stack whose suite binds nothing, its limits stay fast's, and the
/// folder's layer is pinned under `@folder/starter`.
#[test]
fn in_a_folder_fasts_verify_and_ship_are_the_folders_own() {
    clean_git_config!();
    let folder = Folder::new();
    let limits = json!({"max_attempts": 1, "timeout_seconds": 60});
    let mut ship = verify_seat(limits.clone(), folder_hands());
    ship["results"] = json!(["ready", "shipped"]);
    ship["driver"]["command"][5] = json!("./scripts/ship-seat.sh");
    let command = ship["driver"]["command"].as_array_mut().unwrap();
    command.push(json!("{brokkr}"));
    let seats = json!({"verify": verify_seat(limits, folder_hands()), "ship": ship});
    folder.write_folder(&bundle("starter", seats));
    std::fs::write(
        folder.folder().join("scripts/ship-seat.sh"),
        script("folder"),
    )
    .unwrap();
    let library = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fast = library.join("recipes/fast");
    let compiled = compile_in(Some(&library), Boundary::Harness, &fast, &folder.shelves());
    let bundle = compiled.unwrap();
    let dir = folder.folder().canonicalize().unwrap();
    let engine = "/nonexistent/brokkr";
    for (seat, script, timeout) in [
        ("verify", "verify-seat.sh", 3600),
        ("ship", "ship-seat.sh", 1800),
    ] {
        let path = dir.join("scripts").join(script).display().to_string();
        let mut expected = vec![
            engine,
            "driver",
            "exec",
            "--",
            "bash",
            &path,
            "{prompt_file}",
        ];
        expected.extend((seat == "ship").then_some(engine));
        assert_eq!(argv(&bundle, seat), expected);
        assert_eq!(bundle.manifest["hands"][seat], folder_hands());
        let limits = &bundle.seats[seat].limits;
        assert_eq!((limits.max_attempts, limits.timeout_seconds), (2, timeout));
    }
    let pinned = bundle.folder.clone().expect("the folder layer is pinned");
    assert_eq!(
        bundle.manifest["files"]["@folder/starter"],
        json!(pinned.digest)
    );
    assert_eq!(bundle.roots, [fast.canonicalize().unwrap(), dir]);
}

/// An edit to the folder's script or to its `bundle.json` moves the
/// recipe's digest through its `@folder/starter` key alone, which is what
/// resume's comparison names.
#[test]
fn an_edit_to_the_folder_moves_the_recipes_digest_by_its_folder_key() {
    clean_git_config!();
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
    clean_git_config!();
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

/// The word refusal of seat `verify` whose `from` is `word`, as JSON.
fn word_refusal(word: &str) -> (FolderSeatRefusal, String) {
    let seat = "verify".to_string();
    let refusal = FolderSeatRefusal::Word {
        seat,
        word: word.into(),
    };
    let text = format!(
        "bundle: seat 'verify' says \"from\": {word}, and \"folder\" is the one word it takes \
         (decision 0091): any other is refused rather than read as no source at all (decision \
         0071 ruling 2)"
    );
    (refusal, text)
}

/// `from` is a closed vocabulary of one word: any other refuses, in a
/// folder or out of one.
#[test]
fn a_from_other_than_folder_refuses() {
    clean_git_config!();
    let folder = Folder::new();
    let mut seat = from_folder();
    seat["from"] = json!("repo");
    let leaf = folder.recipe(json!({"verify": seat}));
    for shelves in [folder.shelves(), Shelves::default()] {
        assert_eq!(refusal(compile(&leaf, &shelves)), word_refusal("\"repo\""));
    }
}

/// The folder's own declaration is read for its word as the recipe's is:
/// while the recipe's seat says `"folder"`, a folder declaration saying any
/// other refuses in the recipe's compile as in the folder's own, before
/// its keys are read.
#[test]
fn a_folder_declaration_whose_from_is_another_word_refuses() {
    clean_git_config!();
    let folder = Folder::new();
    let leaf = folder.recipe(json!({"verify": from_folder()}));
    for (word, written) in [
        (json!("repo"), "\"repo\""),
        (json!(null), "null"),
        (json!(7), "7"),
    ] {
        let mut declared = verify_seat(json!({}), folder_hands());
        declared["from"] = word;
        declared["inputs"] = json!(["notes.md"]);
        folder.declare(declared);
        assert_eq!(
            refusal(compile(&leaf, &folder.shelves())),
            word_refusal(written)
        );
        assert_eq!(
            refusal(compile(&folder.folder(), &folder.shelves())),
            word_refusal(written)
        );
    }
}

/// A folder declaration that says `"from": "folder"` is the folder's own
/// seat (TAKE 2): the folder's own compile takes its own declaration, and
/// a recipe whose seat says it takes that declaration's driver and hands.
#[test]
fn a_folder_declaration_that_says_folder_is_taken_in_both_compiles() {
    clean_git_config!();
    let folder = Folder::new();
    let dir = folder.folder().canonicalize().unwrap();
    let limits = json!({"max_attempts": 2, "timeout_seconds": 3600});
    let mut declared = verify_seat(limits, folder_hands());
    declared["from"] = json!("folder");
    folder.declare(declared);
    let leaf = folder.recipe(json!({"verify": from_folder()}));
    for (leaf, limits) in [(&leaf, (3, 60)), (&dir, (2, 3600))] {
        let bundle = compile(leaf, &folder.shelves()).unwrap();
        assert_eq!(argv(&bundle, "verify")[5], script_at(&dir));
        assert_eq!(bundle.manifest["hands"]["verify"], folder_hands());
        let seat = &bundle.seats["verify"];
        assert_eq!(
            (seat.limits.max_attempts, seat.limits.timeout_seconds),
            limits
        );
    }
}

/// Refusal 4's text, pinned once.
const NO_OWN_DRIVER: &str = "bundle: seat 'verify' says \"from\": \"folder\" and declares no \
                             driver of its own, which a compile with no folder runs (decision \
                             0091)";

/// Refusal 4: a seat that is the folder's own still declares the driver a
/// compile with no folder runs, in a folder or out of one.
#[test]
fn a_folder_seat_with_no_driver_of_its_own_refuses() {
    clean_git_config!();
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
        assert_eq!(text, NO_OWN_DRIVER);
    }
}

/// Refusal 4 in a folder reads the recipe's own seat, as written before the
/// folder's driver replaced it, as a compile with no folder does: a driver
/// that compile refuses refuses here, a model's among them, as a seat with
/// no `role` runs `exec` alone, while out of a folder that compile's own
/// refusal stands, unchanged. A work seat with no hands may exec what it
/// likes, as it may with no folder.
#[test]
fn a_folder_seat_whose_own_driver_a_compile_with_no_folder_refuses_refuses() {
    clean_git_config!();
    let folder = Folder::new();
    let pinned = from_folder()["driver"]["command"].clone();
    let unpinned = json!(["{brokkr}", "driver", "exec", "--", "bash", "-c", "true"]);
    let model = json!(["{brokkr}", "driver", "claude"]);
    let unknown = "bundle: seat 'verify' driver has an unknown key, 'env'; known: command. A key \
                   the compiler does not read is a declaration that was never made — a \
                   capability placed there would compile, deliver nothing and run the seat at \
                   its harness default — so it is refused rather than ignored (decision 0004; \
                   decision 0065 slice one, rebuild unit 5e)";
    let boxless = "bundle: seat 'verify' declares hands under the `harness` boundary, where no \
                   box stands, so its exec command must name the bundle's own pinned script: \
                   '-c' is an option token before the script; interpreters take no options here \
                   (`bash -c` would run text, not pinned bytes) (decision 0046 ruling 4; \
                   decision 0021)";
    let untrusted = "bundle: seat 'verify' is gate class but seats driver 'exec', which does not \
                     hold the trusted tier; a gate seat IS the check, and nobody stands behind \
                     the judges (decision 0021 ruling 2 — an undeclared tier is untrusted)";
    let missing_role = "bundle: seat 'verify' missing 'role'";
    for (driver, hands, outside) in [
        (json!(null), true, missing_role),
        (json!({}), true, missing_role),
        (json!({"command": []}), true, missing_role),
        (json!({"command": model}), false, missing_role),
        (json!({"command": pinned, "env": {}}), true, unknown),
        (json!({"command": unpinned}), true, boxless),
        (json!({"command": pinned}), false, untrusted),
    ] {
        let mut seat = from_folder();
        seat["driver"] = driver;
        if !hands {
            seat.as_object_mut().unwrap().remove("hands");
        }
        let leaf = folder.recipe(json!({"verify": seat}));
        let (variant, text) = refusal(compile(&leaf, &folder.shelves()));
        let seat = "verify".into();
        assert_eq!(variant, FolderSeatRefusal::NoOwnDriver { seat });
        assert_eq!(text, NO_OWN_DRIVER);
        let refused = compile(&leaf, &Shelves::default()).map(|bundle| bundle.roots);
        assert_eq!(refused.unwrap_err().to_string(), outside);
    }
    let mut declared = verify_seat(json!({}), folder_hands());
    declared.as_object_mut().unwrap().remove("class");
    folder.declare(declared);
    let mut seat = from_folder();
    seat["driver"]["command"] = unpinned;
    for key in ["class", "hands"] {
        seat.as_object_mut().unwrap().remove(key);
    }
    let leaf = folder.recipe(json!({"verify": seat}));
    let dir = folder.folder().canonicalize().unwrap();
    let taken = compile(&leaf, &folder.shelves()).unwrap();
    assert_eq!(argv(&taken, "verify")[5], script_at(&dir));
    let own = compile(&leaf, &Shelves::default()).unwrap();
    assert_eq!(argv(&own, "verify")[4..], ["bash", "-c", "true"]);
}

/// Under a box, refusal 4 reads the recipe's own seat with the whole of the
/// compile with no folder: an exec of text under hands, which the box
/// admits for a gate, compiles in a folder as out of one, while a secret
/// lint or a hands reading that compile refuses refuses in a folder with
/// refusal 4's text, and out of one with its own.
#[test]
fn under_a_box_a_folder_seat_is_held_to_the_whole_compile_with_no_folder() {
    clean_git_config!();
    let folder = Folder::new();
    let boxed = |leaf: &Path, shelves: &Shelves| compile_under(Boundary::Namespace, leaf, shelves);
    let mut seat = from_folder();
    seat["driver"]["command"] = json!(["{brokkr}", "driver", "exec", "--", "bash", "-c", "true"]);
    let leaf = folder.recipe(json!({"verify": seat}));
    let dir = folder.folder().canonicalize().unwrap();
    let taken = boxed(&leaf, &folder.shelves()).unwrap();
    assert_eq!(argv(&taken, "verify")[5], script_at(&dir));
    let own = boxed(&leaf, &Shelves::default()).unwrap();
    assert_eq!(argv(&own, "verify")[4..], ["bash", "-c", "true"]);
    let undeclared = "bundle: seat 'verify' command template references undeclared secret \
                      'UNDECLARED'; declare it in the seat's 'secrets' list (undeclared names \
                      never resolve)";
    let malformed = "bundle: seat 'verify' command template: malformed secret reference \
                     '{{secret:': expected {{secret:NAME}} with NAME matching [A-Z][A-Z0-9_]*";
    let null = "bundle: seat 'verify' hands: hands must be \"workspace\" or an object, got null \
                (decision 0043)";
    let secret = |reference: &str| {
        let mut seat = from_folder();
        let command = seat["driver"]["command"].as_array_mut().unwrap();
        command.push(json!(reference));
        seat
    };
    let mut handless = from_folder();
    handless["hands"] = Value::Null;
    for (seat, outside) in [
        (secret("{{secret:UNDECLARED}}"), undeclared),
        (secret("{{secret:"), malformed),
        (handless, null),
    ] {
        let leaf = folder.recipe(json!({"verify": seat}));
        let (variant, text) = refusal(boxed(&leaf, &folder.shelves()));
        let seat = "verify".into();
        assert_eq!(variant, FolderSeatRefusal::NoOwnDriver { seat });
        assert_eq!(text, NO_OWN_DRIVER);
        let refused = boxed(&leaf, &Shelves::default()).map(|bundle| bundle.roots);
        assert_eq!(refused.unwrap_err().to_string(), outside);
    }
}

/// The recipe's verify seat with `driver` as its driver, or with none.
fn driven(driver: Option<Value>) -> Value {
    let mut seat = from_folder();
    let own = seat.as_object_mut().unwrap();
    match driver {
        None => own.remove("driver"),
        Some(driver) => own.insert("driver".into(), driver),
    };
    seat
}

/// The recipe `child` on the personal shelf, which extends `fixture` and
/// declares nothing of its own.
fn child(folder: &Folder) -> PathBuf {
    let document = json!({"name": "child", "extends": "fixture"});
    layer(&folder.personal.join("child"), &document, "child")
}

/// In a composed child, refusal 4 rises with its variant and its one text
/// whether the inherited seat's driver is absent, which the resolver
/// refuses, or present and one a compile with no folder refuses, which the
/// seats' compile does, while any other refusal of a composed child still
/// carries the chain's note.
#[test]
fn in_a_composed_child_refusal_four_rises_unwrapped_and_others_carry_the_note() {
    clean_git_config!();
    let folder = Folder::new();
    let leaf = child(&folder);
    for driver in [None, Some(Value::Null)] {
        folder.recipe(json!({"verify": driven(driver)}));
        let seat = "verify".into();
        let refused = (
            FolderSeatRefusal::NoOwnDriver { seat },
            NO_OWN_DRIVER.into(),
        );
        assert_eq!(refusal(compile(&leaf, &folder.shelves())), refused);
    }
    let mut seat = from_folder();
    seat.as_object_mut().unwrap().remove("from");
    seat["driver"] = Value::Null;
    folder.recipe(json!({"verify": seat}));
    match compile(&leaf, &folder.shelves()).map(|bundle| bundle.roots) {
        Err(CompileError::Invalid(text)) => assert_eq!(
            text,
            "bundle: seat 'verify' missing 'role' (composed: child -> fixture)"
        ),
        other => panic!("not an invalid bundle: {other:?}"),
    }
}

/// Refusal 4's place: a seat that declares no driver refuses with it
/// before the folder's declaration is read, while a seat whose driver is
/// present but which a compile with no folder refuses gives the folder's
/// own refusal first (1, 3, then 2) and refusal 4 only where the folder's
/// declaration is sound, as the seats compile after the resolver.
#[test]
fn refusal_four_follows_the_folders_refusals_unless_no_driver_is_declared() {
    clean_git_config!();
    let folder = Folder::new();
    let seatless = json!({"name": "starter", "policy": "policy.json", "seats": {}});
    let raw = || {
        let mut declared = verify_seat(json!({}), folder_hands());
        declared["driver"]["command"] = json!(["bash", "./scripts/verify-seat.sh"]);
        declared
    };
    let reversed = || {
        let mut declared = verify_seat(json!({}), folder_hands());
        declared["results"] = json!(["fail", "pass"]);
        declared
    };
    let sound = || verify_seat(json!({}), folder_hands());
    let undeclared = undeclared_text(&folder.file());
    let shape = |results: &str| {
        Box::new(Shape {
            class: "\"gate\"".into(),
            results: results.into(),
        })
    };
    let seat = || "verify".to_string();
    for (driver, declared, expected, text) in [
        (
            Some(Value::Null),
            None,
            FolderSeatRefusal::Undeclared {
                seat: seat(),
                file: folder.file(),
            },
            undeclared.as_str(),
        ),
        (
            Some(Value::Null),
            Some(raw()),
            FolderSeatRefusal::NotItsOwnScript { seat: seat() },
            NOT_ITS_OWN_SCRIPT,
        ),
        (
            Some(Value::Null),
            Some(reversed()),
            FolderSeatRefusal::Differs {
                seat: seat(),
                folder: shape("[\"fail\",\"pass\"]"),
                recipe: shape("[\"pass\",\"fail\"]"),
            },
            DIFFERS,
        ),
        (
            Some(Value::Null),
            Some(sound()),
            FolderSeatRefusal::NoOwnDriver { seat: seat() },
            NO_OWN_DRIVER,
        ),
        (
            None,
            None,
            FolderSeatRefusal::NoOwnDriver { seat: seat() },
            NO_OWN_DRIVER,
        ),
    ] {
        match declared {
            Some(declared) => folder.declare(declared),
            None => folder.write_folder(&seatless),
        }
        let leaf = folder.recipe(json!({"verify": driven(driver)}));
        assert_eq!(
            refusal(compile(&leaf, &folder.shelves())),
            (expected, text.to_string())
        );
    }
}

/// The sixth refusal: a key the seat is not read for, written by the
/// recipe (in a folder or out of one) or by the folder's declaration.
#[test]
fn a_key_a_folder_seat_is_not_read_for_refuses_on_either_side() {
    clean_git_config!();
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
    clean_git_config!();
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
    assert_eq!(text, undeclared_text(&file));
}

/// Refusal 1's text, for a folder bundle declared in `file`.
fn undeclared_text(file: &Path) -> String {
    format!(
        "bundle: seat 'verify' is the folder's own (decision 0091), and {} declares no seat \
         'verify'",
        file.display()
    )
}

/// Refusal 3's text, pinned once.
const NOT_ITS_OWN_SCRIPT: &str = "bundle: seat 'verify' is the folder's own (decision 0091), and \
                                  the folder's is not a `{brokkr} driver exec` dispatch of its \
                                  own pinned script";

/// Refusal 3: the folder's seat runs something other than its own pinned
/// script, whether a raw process or an exec of text.
#[test]
fn a_folder_seat_that_is_not_an_exec_of_its_own_script_refuses() {
    clean_git_config!();
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
        assert_eq!(text, NOT_ITS_OWN_SCRIPT);
    }
}

/// Refusal 2's text for a folder whose results are the recipe's reversed,
/// pinned once.
const DIFFERS: &str = "bundle: seat 'verify' is the folder's own (decision 0091), and the \
                       folder's is \"gate\" with results [\"fail\",\"pass\"] where the recipe's \
                       is \"gate\" with results [\"pass\",\"fail\"]";

/// Refusal 2: the folder's seat is another class, or has other results,
/// as written and in order.
#[test]
fn a_folder_seat_of_another_shape_refuses() {
    clean_git_config!();
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
    assert_eq!(text, DIFFERS);
}

/// Refusal 5: a folder bundle never takes a seat from itself through its
/// base, compiled as the leaf or as a personal recipe's `.brokkr` base,
/// with a folder or with none.
#[test]
fn a_folder_bundle_taking_its_bases_folder_seat_refuses() {
    clean_git_config!();
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
    clean_git_config!();
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
