use super::*;
/// The tree these compiles resolve against: the workspace, never the
/// process's directory (decision 0023, and since 0021 a compile reads
/// the adapter data even for a recipe that names no agent).
use crate::tests::workspace;
use brokkr_core::policy::Machine;
use brokkr_runtime::bundle::Limits;
use brokkr_runtime::{Seat, SequenceStep, StepBody};
use std::collections::BTreeMap;

fn bundle_with_sequence() -> Bundle {
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
                steps: vec![
                    SequenceStep {
                        name: "draft".into(),
                        results: vec!["drafted".into()],
                        class: brokkr_runtime::SeatClass::Work,
                        body: StepBody::Single {
                            role_path: "role.md".into(),
                            command: vec!["driver".into()],
                            candidates: Vec::new(),
                        },
                    },
                    SequenceStep {
                        name: "verify".into(),
                        results: vec!["pass".into()],
                        class: brokkr_runtime::SeatClass::Work,
                        body: StepBody::Single {
                            role_path: "role.md".into(),
                            command: vec!["driver".into()],
                            candidates: Vec::new(),
                        },
                    },
                ],
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
    let resolve = |bundle, recipe, recipes_dir: &Path| {
        source(bundle, recipe, recipes_dir.to_path_buf()).resolve()
    };
    assert_eq!(
        resolve(Some(direct.clone()), None, dir.path()).unwrap(),
        direct
    );
    assert!(resolve(None, Some("missing".into()), dir.path()).is_err());
    assert!(std::panic::catch_unwind(|| resolve(None, None, dir.path())).is_err());
    assert_eq!(
        seat_summary(&bundle_with_sequence()),
        "review[draft>verify]"
    );

    let mut selected = bundle_with_sequence();
    let sequence = selected.seats["review"].body.clone();
    selected.seats.get_mut("review").unwrap().body = SeatBody::Select {
        cases: BTreeMap::new(),
        default: Some(Box::new(sequence.clone())),
        case_gates: BTreeMap::new(),
        default_gate: false,
    };
    assert_eq!(seat_summary(&selected), "review{default=draft>verify}");

    selected.seats.get_mut("review").unwrap().body = SeatBody::Select {
        cases: BTreeMap::from([(
            "nested".into(),
            SeatBody::Select {
                cases: BTreeMap::from([("leaf".into(), sequence)]),
                default: None,
                case_gates: BTreeMap::new(),
                default_gate: false,
            },
        )]),
        default: None,
        case_gates: BTreeMap::new(),
        default_gate: false,
    };
    assert_eq!(seat_summary(&selected), "review{nested=draft>verify}");

    selected.seats.get_mut("review").unwrap().body = SeatBody::Select {
        cases: BTreeMap::new(),
        default: Some(Box::new(SeatBody::Single {
            role_path: PathBuf::new(),
            command: Vec::new(),
            candidates: Vec::new(),
        })),
        case_gates: BTreeMap::new(),
        default_gate: false,
    };
    assert_eq!(seat_summary(&selected), "review{default=inline}");
    selected.seats.get_mut("review").unwrap().body = SeatBody::Select {
        cases: BTreeMap::from([(
            "nested".into(),
            SeatBody::Select {
                cases: BTreeMap::new(),
                default: None,
                case_gates: BTreeMap::new(),
                default_gate: false,
            },
        )]),
        default: None,
        case_gates: BTreeMap::new(),
        default_gate: false,
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
    list(&workspace(), &recipes_file).unwrap();

    let library = empty.path().join("library");
    std::fs::create_dir(&library).unwrap();
    std::fs::create_dir(library.join("already")).unwrap();
    assert!(add(&workspace(), "unused", "already", &library).is_err());
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
        source.to_str().unwrap(),
        "gated",
        library.path(),
    )
    .expect("a recipe whose gates the workspace vouches for installs");
    assert!(
        library.path().join("gated/bundle.json").is_file(),
        "the installed copy survives"
    );
    // …and the listing that follows reads the same tree, so the recipe
    // it just accepted is not reported broken one command later.
    list(&workspace(), library.path()).unwrap();
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
    add(&workspace(), node.to_str().unwrap(), "node", fresh.path())
        .expect("node installs into a library that holds nothing");
    assert_eq!(installed(fresh.path()), ["fast", "node"]);

    let holding = tempfile::tempdir().unwrap();
    let fast = workspace().join("recipes/fast");
    copy_dir(&fast, &holding.path().join("fast")).unwrap();
    let charter = holding.path().join("fast/roles/implementer.md");
    let ours = std::fs::read_to_string(&charter).unwrap() + "\nThe library's own.\n";
    std::fs::write(&charter, &ours).unwrap();
    let own = workspace().join("recipes/self");
    add(&workspace(), own.to_str().unwrap(), "own", holding.path())
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
        alone.to_str().unwrap(),
        "node",
        holding.path(),
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
    add(&workspace(), own.to_str().unwrap(), "own", library.path())
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
        let installing = install(&leaf, "leaf", &library).unwrap();
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
    let refusal = add(bare.path(), node.to_str().unwrap(), "node", library.path())
        .unwrap_err()
        .to_string();
    assert!(
        refusal.starts_with("recipe 'node' does not compile (removed): "),
        "{refusal}"
    );
    assert_eq!(installed(library.path()), Vec::<String>::new());
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
