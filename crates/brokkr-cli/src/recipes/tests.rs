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
            results: vec!["clean".into()],
            limits: Limits::default(),
            inputs: Vec::new(),
            secrets: Vec::new(),
            body: SeatBody::Sequence {
                steps: vec![
                    SequenceStep {
                        name: "draft".into(),
                        body: StepBody::Single {
                            role_path: "role.md".into(),
                            command: vec!["driver".into()],
                            confine: None,
                            candidates: Vec::new(),
                        },
                    },
                    SequenceStep {
                        name: "verify".into(),
                        body: StepBody::Single {
                            role_path: "role.md".into(),
                            command: vec!["driver".into()],
                            confine: None,
                            candidates: Vec::new(),
                        },
                    },
                ],
            },
        },
    );
    Bundle {
        name: "test".into(),
        description: String::new(),
        cost: String::new(),
        dir: PathBuf::new(),
        roots: vec![PathBuf::new()],
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
    }
}

#[test]
fn resolver_and_sequence_summary_cover_every_shape() {
    let dir = tempfile::tempdir().unwrap();
    let direct = dir.path().join("direct");
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
    let source = workspace().join("bundles/verify");
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
