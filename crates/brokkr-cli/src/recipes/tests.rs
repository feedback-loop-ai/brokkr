use super::*;
/// The tree these compiles resolve against: the workspace, never the
/// process's directory (decision 0023, and since 0021 a compile reads
/// the adapter data even for a recipe that names no agent).
use crate::tests::workspace;
use brokkr_core::policy::Machine;
use brokkr_runtime::bundle::Limits;
use brokkr_runtime::launch::LaunchError;
use brokkr_runtime::{Seat, SequenceStep, StepBody};
use std::collections::BTreeMap;

fn bundle_with_sequence() -> Bundle {
    let step = |name: &str, result: &str| SequenceStep {
        name: name.into(),
        results: vec![result.into()],
        class: brokkr_runtime::SeatClass::Work,
        body: StepBody::Single {
            role_path: "role.md".into(),
            command: vec!["driver".into()],
            candidates: Vec::new(),
            written: Vec::new(),
        },
    };
    let mut seats = BTreeMap::new();
    seats.insert(
        "review".into(),
        Seat {
            has_gate: false,
            results: vec!["clean".into()],
            limits: Limits::default(),
            inputs: Vec::new(),
            secrets: Vec::new(),
            body: SeatBody::Sequence {
                steps: vec![step("draft", "drafted"), step("verify", "pass")],
            },
        },
    );
    Bundle {
        dialect_prompts: Default::default(),
        name: "test".into(),
        description: String::new(),
        cost: String::new(),
        dir: PathBuf::new(),
        roots: vec![PathBuf::new()],
        boundary: brokkr_core::realms::Boundary::Namespace,
        chain: Vec::new(),
        machine: Machine {
            phases: vec!["review".into()],
            initial: "review".into(),
            terminal: Vec::new(),
            shippable_from: Vec::new(),
            rules: Vec::new(),
        },
        seats,
        manifest: serde_json::json!({}),
        protected_phase: "review".into(),
        hands: std::collections::BTreeMap::new(),
        inline_resume: std::collections::BTreeMap::new(),
        sites: Default::default(),
        charters: Default::default(),
    }
}

#[test]
fn resolver_and_sequence_summary_cover_every_shape() {
    let dir = tempfile::tempdir().unwrap();
    let direct = dir.path().join("direct");
    let implicit = dir.path().join("implicit");
    let resolve = |bundle, recipe, recipes_dir: &Path| {
        let folder = || Ok::<_, std::io::Error>(implicit.clone());
        source(bundle, recipe, recipes_dir.to_path_buf(), folder)
            .unwrap()
            .resolve()
    };
    assert_eq!(
        resolve(Some(direct.clone()), None, dir.path()).unwrap(),
        direct
    );
    assert!(resolve(None, Some("missing".into()), dir.path()).is_err());
    // Neither flag: the discovered folder's implicit bundle (decision 0083).
    assert_eq!(resolve(None, None, dir.path()).unwrap(), implicit);
    assert_eq!(
        seat_summary(&bundle_with_sequence()),
        "review[draft>verify]"
    );

    let mut selected = bundle_with_sequence();
    let sequence = selected.seats["review"].body.clone();
    selected.seats.get_mut("review").unwrap().body = SeatBody::Select {
        cases: BTreeMap::new(),
        default: Some(Box::new(sequence.clone())),
    };
    assert_eq!(seat_summary(&selected), "review{default=draft>verify}");

    selected.seats.get_mut("review").unwrap().body = SeatBody::Select {
        cases: BTreeMap::from([(
            "nested".into(),
            SeatBody::Select {
                cases: BTreeMap::from([("leaf".into(), sequence)]),
                default: None,
            },
        )]),
        default: None,
    };
    assert_eq!(seat_summary(&selected), "review{nested=draft>verify}");

    selected.seats.get_mut("review").unwrap().body = SeatBody::Select {
        cases: BTreeMap::new(),
        default: Some(Box::new(SeatBody::Single {
            role_path: PathBuf::new(),
            command: Vec::new(),
            candidates: Vec::new(),
            written: Vec::new(),
            class: brokkr_runtime::SeatClass::Work,
        })),
    };
    assert_eq!(seat_summary(&selected), "review{default=inline}");
    selected.seats.get_mut("review").unwrap().body = SeatBody::Select {
        cases: BTreeMap::from([(
            "nested".into(),
            SeatBody::Select {
                cases: BTreeMap::new(),
                default: None,
            },
        )]),
        default: None,
    };
    assert_eq!(seat_summary(&selected), "review{nested=unresolved}");
}

#[test]
fn root_discovery_listing_and_existing_destination_cover_refusals() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("bundle.json"), "{}").unwrap();
    assert_eq!(bundle_root(dir.path()).unwrap(), dir.path());

    let empty = tempfile::tempdir().unwrap();
    assert!(bundle_root(empty.path())
        .unwrap_err()
        .to_string()
        .contains("no bundle"));
    for child in ["a", "b"] {
        let child = empty.path().join(child);
        std::fs::create_dir(&child).unwrap();
        std::fs::write(child.join("bundle.json"), "{}").unwrap();
    }
    assert!(bundle_root(empty.path())
        .unwrap_err()
        .to_string()
        .contains("2 subdirectories"));

    let recipes_file = empty.path().join("not-a-directory");
    std::fs::write(&recipes_file, "x").unwrap();
    list(&workspace(), &recipes_file, false).unwrap();

    let library = empty.path().join("library");
    std::fs::create_dir(&library).unwrap();
    std::fs::create_dir(library.join("already")).unwrap();
    assert!(add(&workspace(), &library, "unused", "already", None).is_err());
}

/// Installing a recipe whose seats JUDGE resolves the trust tier from
/// the WORKSPACE (decision 0021 read through 0023), not from wherever
/// the process happens to stand — this test's own directory is the
/// crate, which has no `adapters/` at all. Before that, `add` refused
/// such a recipe and then DELETED the copy for failing a check it was
/// never given the data for.
#[test]
fn a_gate_bearing_recipe_installs_against_the_workspaces_adapters() {
    let library = tempfile::tempdir().unwrap();
    let source = workspace().join("recipes/verify");
    add(
        &workspace(),
        library.path(),
        source.to_str().unwrap(),
        "gated",
        None,
    )
    .expect("a recipe whose gates the workspace vouches for installs");
    assert!(
        library.path().join("gated/bundle.json").is_file(),
        "the installed copy survives"
    );
    // …and the listing that follows reads the same tree, so the recipe
    // it just accepted is not reported broken one command later.
    list(&workspace(), library.path(), false).unwrap();
}

fn installed(library: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(library)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// A derived recipe resolves its bases from the library it is installed
/// into, so `add` brings each base the library lacks beside it — the
/// documented `brokkr recipes add <brokkr>/recipes/node` into a fresh
/// library — and stops at the first base the library already holds.
#[test]
fn a_derived_recipe_installs_with_the_bases_its_library_lacks() {
    let fresh = tempfile::tempdir().unwrap();
    let node = workspace().join("recipes/node");
    add(
        &workspace(),
        fresh.path(),
        node.to_str().unwrap(),
        "node",
        None,
    )
    .expect("node installs into a library that holds nothing");
    assert_eq!(installed(fresh.path()), ["fast", "node"]);

    let holding = tempfile::tempdir().unwrap();
    let fast = workspace().join("recipes/fast");
    copy_dir(&fast, &holding.path().join("fast")).unwrap();
    let charter = holding.path().join("fast/shipper.md");
    let ours = std::fs::read_to_string(&charter).unwrap() + "\nThe library's own.\n";
    std::fs::write(&charter, &ours).unwrap();
    let own = workspace().join("recipes/self");
    add(
        &workspace(),
        holding.path(),
        own.to_str().unwrap(),
        "own",
        None,
    )
    .expect("self installs over the library's own fast");
    assert_eq!(installed(holding.path()), ["fast", "own", "panel-review"]);
    assert_eq!(std::fs::read_to_string(&charter).unwrap(), ours);

    // A leaf standing apart from its bases installs alone and composes
    // with the library's.
    let apart = tempfile::tempdir().unwrap();
    copy_dir(&node, &apart.path().join("node")).unwrap();
    let alone = apart.path().join("node");
    add(
        &workspace(),
        holding.path(),
        alone.to_str().unwrap(),
        "node",
        None,
    )
    .expect("a lone leaf composes with the library's base");
    assert_eq!(
        installed(holding.path()),
        ["fast", "node", "own", "panel-review"]
    );
}

/// The chain is walked against the target a layer at a time: a source
/// holding `self` and `panel-review` but not `fast` installs both over the
/// library's own `fast`, where resolving the source's whole chain first
/// refused it.
#[test]
fn a_partial_chain_installs_over_the_librarys_own_base() {
    let source = tempfile::tempdir().unwrap();
    for recipe in ["self", "panel-review"] {
        let from = workspace().join("recipes").join(recipe);
        copy_dir(&from, &source.path().join(recipe)).unwrap();
    }
    let library = tempfile::tempdir().unwrap();
    copy_dir(
        &workspace().join("recipes/fast"),
        &library.path().join("fast"),
    )
    .unwrap();
    let own = source.path().join("self");
    add(
        &workspace(),
        library.path(),
        own.to_str().unwrap(),
        "own",
        None,
    )
    .expect("self composes from its source's panel-review and the library's fast");
    assert_eq!(installed(library.path()), ["fast", "own", "panel-review"]);
}

/// A base that is not one plain name, or that the source's library holds
/// only as a symlink, is never read from or copied to outside the library;
/// the leaf is copied alone for the compile to refuse.
#[test]
fn a_base_outside_the_library_is_never_copied() {
    let source = tempfile::tempdir().unwrap();
    let outside = source.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::create_dir_all(source.path().join("lib")).unwrap();
    std::os::unix::fs::symlink(&outside, source.path().join("lib/linked")).unwrap();
    for base in ["../outside", "linked"] {
        let leaf = source.path().join("lib/leaf");
        std::fs::create_dir_all(&leaf).unwrap();
        let document = serde_json::json!({"name": "leaf", "extends": base});
        std::fs::write(leaf.join("bundle.json"), document.to_string()).unwrap();
        let target = tempfile::tempdir().unwrap();
        let library = target.path().join("lib");
        std::fs::create_dir(&library).unwrap();
        let installing = install(&planned(&leaf, "leaf", &library).unwrap(), &library).unwrap();
        let names: Vec<&str> = installing.0.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["leaf"], "{base}");
        assert!(!target.path().join("outside").exists(), "{base}");
    }
}

/// A refused compile removes the leaf AND every base copied with it.
#[test]
fn a_refused_derived_recipe_leaves_the_library_as_it_found_it() {
    let bare = tempfile::tempdir().unwrap();
    let library = tempfile::tempdir().unwrap();
    let node = workspace().join("recipes/node");
    let refusal = add(
        bare.path(),
        library.path(),
        node.to_str().unwrap(),
        "node",
        None,
    )
    .unwrap_err();
    let refused = refusal
        .downcast_ref::<Refused>()
        .expect("a compile refusal");
    assert_eq!(refused.recipe, "node");
    assert!(
        matches!(
            refused.cause,
            LaunchError::Compile(brokkr_runtime::CompileError::Invalid(_))
        ),
        "{:?}",
        refused.cause
    );
    assert_eq!(
        refusal.to_string(),
        format!(
            "recipe 'node' does not compile (removed): {}",
            refused.cause
        )
    );
    assert_eq!(installed(library.path()), Vec::<String>::new());
}

/// Two `add`s into one library never interleave. The first is stood in for
/// by a held shelf with its `fast` copy in place, about to be refused and
/// removed: the second waits rather than stop its walk at that copy, so it
/// never keeps a `review-first` whose base the first then removes. When
/// the first made the library, it removes it empty before the second
/// wakes, which then makes its own rather than install into a removed one.
#[test]
fn a_second_add_waits_for_the_library_another_holds() {
    for standing in [true, false] {
        let parent = tempfile::tempdir().unwrap();
        let dir = parent.path().join("library");
        if standing {
            std::fs::create_dir(&dir).unwrap();
        }
        let held = Shelf::make(&dir, &dir).unwrap();
        assert_eq!(held.made.is_some(), !standing, "the first made it");
        let in_flight = dir.join("fast");
        copy_dir(&workspace().join("recipes/fast"), &in_flight).unwrap();
        let library = dir.clone();
        let second = std::thread::spawn(move || {
            let review_first = workspace().join("recipes/review-first");
            let source = review_first.to_str().unwrap();
            add(&workspace(), &library, source, "review-first", None)
        });
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert!(
            !second.is_finished(),
            "the second add ran in a held library"
        );
        std::fs::remove_dir_all(&in_flight).unwrap();
        drop(held);
        second
            .join()
            .unwrap()
            .expect("the second add installs once the library is released");
        assert_eq!(installed(&dir), ["fast", "review-first"]);
    }
}

/// An entry the library already holds is never removed by an install that
/// did not make it, a dangling link included: as the leaf it is refused
/// before any copy, as a base it ends the walk, and a copy that meets it
/// fails without registering it.
#[test]
fn a_dangling_link_in_the_library_is_never_removed() {
    let missing = tempfile::tempdir().unwrap().path().join("missing");
    let node = workspace().join("recipes/node");
    let linked = |name: &str| {
        let library = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(&missing, library.path().join(name)).unwrap();
        library
    };
    let kept = |library: &Path, name: &str| {
        assert_eq!(std::fs::read_link(library.join(name)).unwrap(), missing);
        assert_eq!(installed(library), [name]);
    };

    let base = linked("fast");
    let refusal = add(
        &workspace(),
        base.path(),
        node.to_str().unwrap(),
        "node",
        None,
    )
    .unwrap_err();
    assert_eq!(refusal.downcast_ref::<Refused>().unwrap().recipe, "node");
    kept(base.path(), "fast");

    let leaf = linked("node");
    let refusal = add(
        &workspace(),
        leaf.path(),
        node.to_str().unwrap(),
        "node",
        None,
    )
    .unwrap_err();
    let dest = leaf.path().join("node");
    assert_eq!(
        refusal.to_string(),
        format!(
            "recipe 'node' already exists at {}; remove it first",
            dest.display()
        )
    );
    kept(leaf.path(), "node");

    let planned = planned(&node, "node", leaf.path()).unwrap();
    let refusal = install(&planned, leaf.path()).err().unwrap();
    assert_eq!(
        refusal.downcast_ref::<std::io::Error>().map(|e| e.kind()),
        Some(std::io::ErrorKind::AlreadyExists)
    );
    kept(leaf.path(), "node");
}

#[test]
fn copy_skips_nested_git_metadata() {
    let source = tempfile::tempdir().unwrap();
    std::fs::create_dir(source.path().join(".git")).unwrap();
    std::fs::write(source.path().join(".git/config"), "secret").unwrap();
    std::fs::write(source.path().join("kept"), "plain").unwrap();
    let destination = tempfile::tempdir().unwrap().keep();
    copy_dir(source.path(), &destination).unwrap();
    assert!(destination.join("kept").is_file());
    assert!(!destination.join(".git").exists());
}

/// A folder repository planted by hand: a `.git` and a `.brokkr` directory
/// at its root (decision 0083), resolved as the walk resolves it.
pub(super) fn folder_repository() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    for entry in [".git", FOLDER] {
        std::fs::create_dir(root.join(entry)).unwrap();
    }
    (dir, root)
}

/// `add` refuses a library inside a folder repository anywhere but its
/// shelf, where nothing stands yet, by the type and with nothing created;
/// the folder's own shelf passes, and an install into it that the compile
/// refuses removes the agents it copied, the directories they needed and
/// the shelf it made (#675, #696).
#[test]
fn add_refuses_a_folder_repository_by_type() {
    let (_dir, root) = folder_repository();
    let node = workspace().join("recipes/node");
    for dir in [root.join("recipes"), root.join("deep/recipes")] {
        let refusal = add(&root, &dir, node.to_str().unwrap(), "node", None).unwrap_err();
        let refused = refusal.downcast_ref::<InFolder>();
        let expected = InFolder {
            root: root.clone(),
            place: dir.clone(),
        };
        assert_eq!(refused, Some(&expected), "{refusal}");
        assert!(!dir.exists(), "{}", dir.display());
    }
    let shelf = root.join(".brokkr/recipes");
    let refusal = add(&root, &shelf, node.to_str().unwrap(), "node", None).unwrap_err();
    let refused = refusal.downcast_ref::<Refused>().map(|r| r.recipe.as_str());
    assert_eq!(refused, Some("node"), "{refusal}");
    assert_eq!(std::fs::read_dir(root.join(FOLDER)).unwrap().count(), 0);
}

/// `add` of a source that holds the library refuses by the type before
/// anything is copied, and the library it made is removed: the folder's own
/// bundle holds its shelf, and so does a base named `recipes` beside a leaf
/// in the folder, which is the shelf itself (#696).
#[test]
fn add_refuses_a_source_that_holds_the_library_by_type() {
    let (_dir, root) = folder_repository();
    let (folder, shelf) = (root.join(FOLDER), root.join(".brokkr/recipes"));
    let refusal = add(&root, &shelf, folder.to_str().unwrap(), "own", None).unwrap_err();
    let expected = Within {
        source: folder,
        place: shelf.clone(),
    };
    assert_eq!(refusal.downcast_ref::<Within>(), Some(&expected));
    assert!(!shelf.exists());
    let custom = root.join(".brokkr/custom");
    std::fs::create_dir(&custom).unwrap();
    let layer = serde_json::json!({"name": "custom", "extends": "recipes"});
    std::fs::write(custom.join("bundle.json"), layer.to_string()).unwrap();
    let refusal = add(&root, &shelf, custom.to_str().unwrap(), "x", None).unwrap_err();
    let expected = Within {
        source: shelf.clone(),
        place: shelf.clone(),
    };
    assert_eq!(refusal.downcast_ref::<Within>(), Some(&expected));
    assert!(!shelf.exists());
}

/// A shelf recipe's base is planned from its shelf, beside it, the library
/// the compiler composes it from (`places::base_library`), never from a
/// decoy at the repository's root (#696).
#[test]
fn a_shelf_recipes_base_is_planned_from_its_shelf() {
    let (_dir, root) = folder_repository();
    let shelf = root.join(".brokkr/recipes");
    for at in [shelf.join("leaf"), shelf.join("base"), root.join("base")] {
        std::fs::create_dir_all(&at).unwrap();
        let layer = serde_json::json!({"name": "leaf", "extends": "base"});
        std::fs::write(at.join("bundle.json"), layer.to_string()).unwrap();
    }
    let library = tempfile::tempdir().unwrap();
    let plan = planned(&shelf.join("leaf"), "leaf", library.path()).unwrap();
    let expected = [
        ("leaf".to_string(), shelf.join("leaf")),
        ("base".to_string(), shelf.join("base")),
    ];
    assert_eq!(plan, expected);
}

/// A chain that comes back to a name already planned ends there, so a
/// cycle is planned once and the compile names why it does not compose; a
/// library whose parent does not stand is never made, nor one its parent
/// cannot hold.
#[test]
fn a_chain_ends_at_a_name_it_already_plans() {
    let source = tempfile::tempdir().unwrap();
    for (name, base) in [("a", "b"), ("b", "a")] {
        let at = source.path().join(name);
        std::fs::create_dir(&at).unwrap();
        let layer = serde_json::json!({"name": name, "extends": base});
        std::fs::write(at.join("bundle.json"), layer.to_string()).unwrap();
    }
    let library = tempfile::tempdir().unwrap();
    let leaf = source.path().join("a");
    let names: Vec<String> = planned(&leaf, "a", library.path())
        .unwrap()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, ["a", "b"]);
    let unmade = library.path().join("missing/recipes");
    let refusal = add(&workspace(), &unmade, leaf.to_str().unwrap(), "a", None).unwrap_err();
    let kind = refusal.downcast_ref::<std::io::Error>().map(|e| e.kind());
    assert_eq!(kind, Some(std::io::ErrorKind::NotFound));
    let unheld = library.path().join("x".repeat(300));
    let refusal = Shelf::make(&unheld, &unheld).err().map(|e| e.kind());
    assert_eq!(refusal, Some(std::io::ErrorKind::InvalidFilename));
}

/// `add` refuses a library in a folder repository whose `recipes/` is a
/// link to a directory outside it or a nested repository, by the type: the
/// folder repository is judged where the library is spelled, not only
/// where the walk from it resolves to (#675).
#[test]
fn add_refuses_a_folder_repository_behind_a_link_or_a_nested_repository() {
    let (_dir, root) = folder_repository();
    let outside = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let dir = root.join("recipes");
    let plant_link = |dir: &Path| std::os::unix::fs::symlink(external.path(), dir).unwrap();
    let plant_repository = |dir: &Path| std::fs::create_dir_all(dir.join(".git")).unwrap();
    let real = external.path().canonicalize().unwrap();
    for (plant, place) in [
        (&plant_link as &dyn Fn(&Path), &real),
        (&plant_repository, &dir),
    ] {
        plant(&dir);
        refuses_in_folder(outside.path(), &dir, &root, place);
        assert!(!dir.join("node").exists());
        std::fs::remove_file(&dir)
            .or_else(|_| std::fs::remove_dir_all(&dir))
            .unwrap();
    }
    assert_eq!(std::fs::read_dir(external.path()).unwrap().count(), 0);
}

/// `add` refuses a library that a link outside a folder repository
/// resolves into a nested repository at its `recipes/`, at the link and
/// below it, by the type: the folder repository is judged where the
/// library resolves to, not only where it is spelled (#675).
#[test]
fn add_refuses_a_folder_repository_a_link_from_outside_resolves_into() {
    let (_dir, root) = folder_repository();
    let outside = tempfile::tempdir().unwrap();
    let nested = root.join("recipes");
    std::fs::create_dir_all(nested.join(".git")).unwrap();
    let alias = outside.path().join("alias");
    std::os::unix::fs::symlink(&nested, &alias).unwrap();
    for (dir, place) in [
        (alias.clone(), nested.clone()),
        (alias.join("sub"), nested.join("sub")),
    ] {
        refuses_in_folder(outside.path(), &dir, &root, &place);
    }
    assert_eq!(std::fs::read_dir(&nested).unwrap().count(), 1, ".git only");
}

/// `add` of a compiling recipe from `from` into `dir` refuses by the type,
/// naming the folder repository at `root` and the `place` `dir` resolves
/// to (#675).
fn refuses_in_folder(from: &Path, dir: &Path, root: &Path, place: &Path) {
    let node = workspace().join("recipes/node");
    let refusal = add(from, dir, node.to_str().unwrap(), "node", None).unwrap_err();
    let expected = InFolder {
        root: root.to_path_buf(),
        place: place.to_path_buf(),
    };
    assert_eq!(
        refusal.downcast_ref::<InFolder>(),
        Some(&expected),
        "{refusal}"
    );
}

/// A `name` that is not one plain component refuses by the variant before
/// anything is created: it would write somewhere other than the place
/// judged (#675).
#[test]
fn add_refuses_a_path_it_cannot_judge_by_type() {
    let outside = tempfile::tempdir().unwrap();
    let at = outside.path().to_path_buf();
    let node = workspace().join("recipes/node");
    let node = node.to_str().unwrap();
    let dir = at.join("recipes");
    for name in [
        "../node", "a/b", "", ".", "..", "/node", "x/", "x//", "x/.", "x/./",
    ] {
        let refusal = add(&at, &dir, node, name, None).unwrap_err();
        let expected = Unplain::Name {
            name: name.into(),
            dir: dir.clone(),
        };
        assert_eq!(refusal.downcast_ref::<Unplain>(), Some(&expected), "{name}");
    }
    let left = std::fs::read_dir(&at).unwrap().count();
    assert_eq!(left, 0, "nothing is created");
}

/// An entry on the walk from the library that cannot be read refuses by
/// name, never taken as absent (fail closed).
#[test]
fn add_refuses_a_dir_it_cannot_read_the_way_to() {
    use std::os::unix::fs::PermissionsExt;
    let refused = |dir: &Path| {
        let refusal = add(&workspace(), dir, "unused", "node", None).unwrap_err();
        match refusal.downcast::<PlacesError>() {
            Ok(PlacesError::Unreadable { path, .. }) => path,
            other => panic!("an unreadable refusal: {other:?}"),
        }
    };
    if rustix::process::geteuid().is_root() {
        return;
    }
    let outside = tempfile::tempdir().unwrap();
    let locked = outside.path().join("locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let dir = locked.join("in/recipes");
    let path = refused(&dir);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(path, dir);
}

/// A `--path` that is not a relative path of plain names refuses by its
/// spelling before anything is cloned, and one given with a local source
/// refuses by the variant, both creating nothing (#695).
#[test]
fn add_refuses_a_path_by_its_spelling_or_its_local_source_by_type() {
    let outside = tempfile::tempdir().unwrap();
    let dir = outside.path().join("recipes");
    let never = "file:///never/cloned.git";
    for path in [
        "../x",
        "/abs",
        "recipes/./x",
        "recipes/x/",
        "",
        ".",
        "x//y",
        "x/..",
    ] {
        let refusal = add(&workspace(), &dir, never, "x", Some(path)).unwrap_err();
        let expected = Unplain::Path { path: path.into() };
        assert_eq!(refusal.downcast_ref::<Unplain>(), Some(&expected), "{path}");
    }
    let node = workspace().join("recipes/node");
    let source = node.to_str().unwrap();
    let refusal = add(&workspace(), &dir, source, "node", Some("fast")).unwrap_err();
    let expected = AtPath::Local {
        source: source.into(),
        path: "fast".into(),
    };
    assert_eq!(refusal.downcast_ref::<AtPath>(), Some(&expected));
    let left = std::fs::read_dir(outside.path()).unwrap().count();
    assert_eq!(left, 0, "nothing is created");
}

/// The walk to `--path` inside a clone reaches a bundle, and refuses by the
/// variant a part that is a link, a part that is absent or not a directory,
/// and a directory whose bundle.json is absent or a link (#695).
#[test]
fn reach_walks_to_a_bundle_and_never_through_a_link() {
    let clone = tempfile::tempdir().unwrap();
    let recipes = clone.path().join("recipes");
    for at in ["x", "empty", "linked"] {
        std::fs::create_dir_all(recipes.join(at)).unwrap();
    }
    std::fs::write(recipes.join("x/bundle.json"), "{}").unwrap();
    let linked = recipes.join("linked/bundle.json");
    std::os::unix::fs::symlink(recipes.join("x/bundle.json"), linked).unwrap();
    std::os::unix::fs::symlink(&recipes, clone.path().join("link")).unwrap();
    assert_eq!(reach(clone.path(), "recipes/x"), Ok(recipes.join("x")));
    let link = |path: &str, link: &str| AtPath::Link {
        path: path.into(),
        link: link.into(),
    };
    let missing = |path: &str, missing: &str| AtPath::Missing {
        path: path.into(),
        missing: missing.into(),
    };
    let unbundled = |path: &str| AtPath::Unbundled { path: path.into() };
    let file = "recipes/x/bundle.json";
    for (path, expected) in [
        ("link/x", link("link/x", "link")),
        (
            "recipes/absent/x",
            missing("recipes/absent/x", "recipes/absent"),
        ),
        (file, missing(file, file)),
        ("recipes/empty", unbundled("recipes/empty")),
        ("recipes/linked", unbundled("recipes/linked")),
    ] {
        assert_eq!(reach(clone.path(), path), Err(expected), "{path}");
    }
}
