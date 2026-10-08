//! The judge's door: the last message. Moved here from `boundary_tests.rs`
//! (decision 0065 slice two, U1g2), its fixtures' paths one module deeper.

use super::*;

/// Under `harness` with a `last-message` door the seat's final message
/// reaches the engine as the result file the harness writes; a final
/// message that is not the bare object is a missing result exactly as a
/// malformed file is. The driver reads the file as today under both
/// doors, which the codex driver's own tests prove; here the composed
/// argv names the path and the input names the door.
#[test]
fn a_harness_gate_on_a_last_message_door_names_its_result_path() {
    let (_dir, mut engine) = super::super::tests::engine(single_body(vec!["driver".into()]));
    engine.boundary = Boundary::Harness;
    super::super::tests::set_site_hands(&mut engine.bundle, "work", HandsSpec::default());
    let codex = candidate("codex", CODEX_FRAGMENT.to_vec(), codex_harness());
    let spawn = engine.compose(
        "attempt",
        true,
        codex.argv.clone(),
        Some(&HandsSpec::default()),
        Some(&codex),
        "/r/p.json",
    );
    assert_eq!(
        &spawn.argv[spawn.argv.len() - 2..],
        ["--output-last-message", "/r/p.json"]
    );
    assert_eq!(spawn.env, SpawnEnv::Inherit);
    let mut input = json!({"result_path": "/r/p.json"});
    engine.mark_hands("work", &mut input);
    engine.marks().door("work", true, Some(&codex), &mut input);
    assert_eq!(input["result_delivery"], "last-message");
}
