//! The engine's seat-tree record (decision 0104 rulings 6 and 7, slice
//! 5b): the row lands before the tree, a refused record makes no tree and
//! carries its words, a panel and a sequence share one attempt's row, and
//! a boxed or hands-less site records none.

use std::path::PathBuf;

use brokkr_core::realms::Boundary;
use brokkr_protocol::hands::HandsSpec;
use serde_json::json;

use super::*;
use crate::engine::tests::{engine, member, panel_input, set_site_hands, single_body};
use crate::engine::Selection;

const EFFECT: &str = "effect-a";
const ATTEMPT: &str = "11111111-1111-4111-8111-111111111111";

/// The append's ids: one effect, one engine-minted attempt.
fn ids() -> AttemptIds<'static> {
    AttemptIds {
        effect: EFFECT,
        attempt: ATTEMPT,
    }
}

/// An unboxed exec dispatch: four tokens, the third `exec`.
fn exec() -> Vec<String> {
    ["/usr/local/bin/brokkr", "driver", "exec", "--", "true"]
        .map(String::from)
        .to_vec()
}

/// The root the engine joins an attempt id to.
fn root(engine: &Engine) -> PathBuf {
    engine.run_local().join(SEAT_SCRATCH)
}

/// The attempt's one row, as the engine's own journal reads it.
fn one_row(engine: &Engine) {
    let rows = engine.store.seat_trees(&engine.run_id).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].attempt_id, ATTEMPT);
}

/// The refusal is an unmade tree at `path`, with the sentence the spawn
/// carries around its cause.
fn assert_unmade(refused: &Unready, path: &std::path::Path) {
    assert!(matches!(
        refused,
        Unready::Unmade { attempt, path: named, .. }
            if attempt == ATTEMPT && named.as_path() == path
    ));
    let words = refused.to_string();
    assert!(
        words.starts_with(&format!(
            "the seat tree for attempt '{ATTEMPT}' could not be made at '{}': ",
            path.display()
        )),
        "{words}"
    );
}

#[test]
fn the_row_lands_before_the_tree() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    engine.network_prefix = Some(false);
    let root = root(&engine);
    std::fs::create_dir_all(&root).unwrap();
    // A file where the tree would go: the row's append has already
    // happened by the time `create_dir_all` fails.
    std::fs::write(root.join(ATTEMPT), "").unwrap();

    let refused = unboxed(&mut engine, ids(), &HandsSpec::default()).unwrap_err();
    let rows = engine.store.seat_trees(&engine.run_id).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].effect_id.as_str(), rows[0].attempt_id.as_str()),
        (EFFECT, ATTEMPT)
    );
    assert_eq!(rows[0].root, root);
    assert!(root.join(ATTEMPT).is_file());
    assert!(!root.join(ATTEMPT).join("home").exists());
    assert_unmade(&refused, &root.join(ATTEMPT).join("home"));

    // The tree's second directory failing leaves the same row and no tree.
    std::fs::remove_file(root.join(ATTEMPT)).unwrap();
    std::fs::create_dir(root.join(ATTEMPT)).unwrap();
    std::fs::write(root.join(ATTEMPT).join("tmp"), "").unwrap();
    let refused = unboxed(&mut engine, ids(), &HandsSpec::default()).unwrap_err();
    assert_unmade(&refused, &root.join(ATTEMPT).join("tmp"));
    assert_eq!(engine.store.seat_trees(&engine.run_id).unwrap().len(), 1);
}

#[test]
fn a_panel_of_two_unboxed_members_writes_one_row() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    engine.boundary = Boundary::Open;
    engine.network_prefix = Some(false);
    for label in ["review:left", "review:right"] {
        set_site_hands(&mut engine.bundle, label, HandsSpec::default());
    }
    let members = [member("left", exec()), member("right", exec())];
    let input = panel_input(&["left", "right"]);
    let runs = engine.member_runs(
        ids(),
        "review",
        &members,
        &input["members"],
        &input,
        &input["context"],
        &Selection::new(),
        "",
        false,
    );
    assert_eq!(runs.len(), 2);
    assert!(runs.iter().all(|run| run.spawn.refusal.is_none()));
    one_row(&engine);
}

#[test]
fn a_sequence_of_two_unboxed_steps_writes_one_row() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    engine.boundary = Boundary::Open;
    engine.network_prefix = Some(false);
    for label in ["review:a", "review:b"] {
        set_site_hands(&mut engine.bundle, label, HandsSpec::default());
    }
    let selection = Selection::new();
    for label in ["review:a", "review:b"] {
        let site: crate::engine::Site = None;
        let composition = crate::engine::sequence::StepComposition {
            label,
            effect_id: EFFECT,
            attempt_id: ATTEMPT,
            gate: false,
            command: &exec(),
            site: &site,
            selection: &selection,
        };
        let (spawn, _) = engine.compose_step(
            composition,
            json!({"role_path": "role.md", "result_path": "result.json"}),
        );
        assert_eq!(spawn.refusal, None, "{label}");
    }
    one_row(&engine);
}

#[test]
fn a_refused_record_makes_no_tree_and_names_the_record() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    engine.boundary = Boundary::Open;
    engine.network_prefix = Some(false);
    let root = root(&engine);
    engine
        .store
        .record_seat_tree(&engine.run_id, "another-effect", ATTEMPT, &root)
        .unwrap();
    let unready = unboxed(&mut engine, ids(), &HandsSpec::default()).unwrap_err();
    assert!(matches!(unready, Unready::Unrecorded { .. }));

    let spawn = engine.compose_at(
        None,
        ids(),
        false,
        exec(),
        Some(&HandsSpec::default()),
        None,
        "r.json",
    );
    let refused = spawn.refusal.expect("the spawn is refused");
    let expected = format!(
        "the seat tree record for attempt '{ATTEMPT}' was not written: run '{}' already records \
         a seat tree for attempt '{ATTEMPT}' with another effect or root; this one was not \
         written",
        engine.run_id
    );
    assert_eq!(refused, expected);
    assert!(!root.join(ATTEMPT).exists());
    assert_eq!(engine.store.seat_trees(&engine.run_id).unwrap().len(), 1);
}

#[test]
fn a_boxed_dispatch_and_a_site_with_no_hands_write_no_row() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    engine.network_prefix = Some(false);
    let _boxed = engine.compose_at(
        None,
        ids(),
        false,
        exec(),
        Some(&HandsSpec::default()),
        None,
        "r.json",
    );
    let _unhanded = engine.compose_at(None, ids(), false, exec(), None, None, "r.json");
    assert_eq!(engine.store.seat_trees(&engine.run_id).unwrap(), vec![]);
    assert!(!root(&engine).join(ATTEMPT).exists());
}

#[test]
fn an_unknown_queue_version_makes_the_tree_with_no_row() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    engine.network_prefix = Some(false);
    let journal = engine.store.path().to_path_buf();
    let peer = rusqlite::Connection::open(&journal).unwrap();
    peer.execute("UPDATE meta SET value = '6' WHERE key = 'queue_schema'", [])
        .unwrap();
    drop(peer);
    unboxed(&mut engine, ids(), &HandsSpec::default()).unwrap();
    let root = root(&engine);
    assert!(root.join(ATTEMPT).join("home").is_dir());
    assert!(root.join(ATTEMPT).join("tmp").is_dir());
    let peer = rusqlite::Connection::open(&journal).unwrap();
    let count: i64 = peer
        .query_row("SELECT COUNT(*) FROM seat_trees", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}
