//! U10a2's repairs of the scenario map's authority rows through real dispatch
//! (SC1, row 2; SD3, row 32): what the engine sealed is what the broker
//! serves. Between the engine's admission and its broker's start, the harness
//! substitutes a sealed fact in the plan, or the filesystem under a correctly
//! bound plan drifts, or the plan reaches the broker by a locator the attempt
//! never pinned. Each refuses at the broker, past its harness's start, with
//! no lookup and no server start. A broker refusing an unbound plan opens no
//! ledger, so the engine's settlement parks on the ledger it misses.

use brokkr_protocol::broker::Refusal;

use super::World;

/// The engine's cause for an attempt whose broker opened no ledger.
const MISSING: &str = "broker ledger is missing for this attempt";

/// `world`'s run, its harness running `script` before its broker: parked,
/// with the harness started once, and no lookup, server or call. Its cause.
fn refused_at_broker(world: &World, script: &str) -> Option<String> {
    world.before_broker(script);
    let (looked, started) = (world.store_reads(), world.server_reads());
    let ended = world.run();
    assert_eq!(ended.status, "awaiting_operator", "{ended:?}");
    let unread = (looked.lookups(), world.starts(), started.lookups());
    assert_eq!(unread, (0, 1, 0));
    assert_eq!(world.calls(&ended.run), []);
    ended.cause
}

/// What the broker said it refused, and its exit.
fn broker_refused(world: &World) -> (String, Option<i32>) {
    let said = std::fs::read_to_string(world.bin("broker.stderr")).unwrap();
    (said, world.broker_code())
}

/// The unbound plan's refusal as the broker reports it.
fn unbound() -> (String, Option<i32>) {
    (format!("error: {}\n", Refusal::Unbound), Some(1))
}

/// Row 2 (SC1): the plan's network, sealed from its holding's contracted
/// egress, substituted for an isolated one after sealing, in a file of the
/// plan's own protection: the plan no longer hashes to its pinned digest. The
/// dialect retains nothing, so no retention share re-reads the pins and the
/// plan's own binding is the one check. The sealed bytes are put back once
/// the broker has ended, so the engine settles on what the broker did.
#[test]
fn a_sealed_fact_substituted_after_sealing_leaves_the_plan_unbound() {
    let Some(world) = World::new() else {
        return;
    };
    world.dialect(&serde_json::json!({"retained": false}));
    let script = r#"grep -q '"network":"shared"' "$5" || exit 9
cp -p "$5" "$5.sealed"
sed 's/"network":"shared"/"network":"isolated"/' "$5" > "$5.new"
chmod 600 "$5.new"
mv "$5.new" "$5"
"$@"
code=$?
mv "$5.sealed" "$5"
exit $code"#;
    assert_eq!(refused_at_broker(&world, script).as_deref(), Some(MISSING));
    assert_eq!(broker_refused(&world), unbound());
}

/// Row 2 (SC1): filesystem drift under a correctly bound plan. The server's
/// program loses its group and other bits after sealing, still its owner's
/// to run: MB3's identity cause, not the unbound plan's.
#[test]
fn filesystem_drift_under_a_bound_plan_takes_the_identity_cause() {
    let Some(world) = World::new() else {
        return;
    };
    let script = format!("chmod 700 '{}'", world.program().display());
    let cause = refused_at_broker(&world, &script);
    assert_eq!(cause, Some(Refusal::Identity.to_string()));
}

/// Row 32 (SD3): the very plan the engine sealed, its bytes and digest whole,
/// handed to the public command by a locator beside it that the attempt's
/// inventory never pinned: the unbound plan's refusal, before any message.
#[test]
fn a_sealed_plan_by_a_locator_never_pinned_is_refused_unbound() {
    let Some(world) = World::new() else {
        return;
    };
    let script = r#"copy="$(dirname "$5")/copied.json"
cp -p "$5" "$copy"
"$1" "$2" "$3" "$4" "$copy" "$6" "$7"
code=$?
rm "$copy"
exit $code"#;
    assert_eq!(refused_at_broker(&world, script).as_deref(), Some(MISSING));
    assert_eq!(broker_refused(&world), unbound());
}
