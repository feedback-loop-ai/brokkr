//! Round five of #1200: an agy seat's workspace is checked last, directly
//! before agy starts, and an empty answer names its status like every
//! other end.

use super::judged::{driven, stream};
use super::resolved::turn_rows;
use super::*;

/// The launch `prepared_with` builds for `world`'s seat from `input`, the
/// fake agy set, with `made` making the HOME.
fn prepared_by(
    world: &mut World,
    input: &Value,
    made: impl FnOnce(Homes<'_>, Written<'_>) -> Result<Home, LaunchError>,
) -> Result<Prepared, LaunchError> {
    let bin = fake_agy(&world.seat(), TOOL_LESS, "", true, 0);
    world.set(&bin);
    let (binary, extra) = (bin.to_string_lossy(), argv(&["--model", FLASH]));
    let (temp, operator) = (std::env::temp_dir(), world.path("operator"));
    let homes = Homes {
        temp: &temp,
        operator: Some(&operator),
    };
    prepared_with(&binary, &extra, input, (homes, Path::new("/b")), made)
}

/// The path `invoke` refuses `prepared` for, matched as the typed value.
fn refused_for(prepared: &Prepared, input: &Value) -> std::path::PathBuf {
    let invoked = invoke(prepared, input, "prompt", &[], &mut |_| {});
    let Err(Ended::Launch(LaunchError::Configured(path))) = invoked.map(drop) else {
        panic!("the planted path was admitted or ended otherwise")
    };
    path
}

/// A workspace path planted while the engine's HOME is made is refused:
/// the check runs after the HOME, directly before the spawn, so the fake
/// never starts, and the HOME goes with the refused launch (M1).
#[test]
fn a_workspace_path_planted_while_the_home_is_made_is_refused_and_agy_never_starts() {
    let mut world = World::new();
    let input = world.input(None);
    let seat = world.seat().canonicalize().unwrap();
    let mut planted = None;
    let plant_while_made = |homes: Homes<'_>, written: Written<'_>| {
        let home = home::made(homes, written);
        planted = Some(plant(&seat, ".agents/hooks.json"));
        home
    };
    let prepared = prepared_by(&mut world, &input, plant_while_made).unwrap();
    assert_eq!(Some(refused_for(&prepared, &input)), planted);
    drop(prepared);
    assert!(!world.started());
    assert!(world.homes_gone());
}

/// A workspace path present from the start is still refused through the
/// driver's whole path: the fake never starts, and no HOME is left (M1).
#[test]
fn a_workspace_path_present_from_the_start_is_refused_and_leaves_no_home() {
    let mut world = World::new();
    let bin = fake_agy(&world.seat(), TOOL_LESS, "", true, 0);
    // The driver names the path it checked, the resolved one, so a
    // temporary directory reached through a link is planted at its end.
    let planted = plant(
        &world.seat().canonicalize().unwrap(),
        ".agents/mcp_config.json",
    );
    let messages = world.run(&bin, &["--model", FLASH], world.input(None), None);
    let error = ended(&messages).and_then(|(_, error, _)| error);
    assert_eq!(error, Some(LaunchError::Configured(planted).to_string()));
    assert!(!world.started());
    assert!(world.homes_gone());
}

/// A workspace path present from the start does not stop the HOME's
/// making: the one check runs after it, so `prepared_with` reaches `made`
/// once, and the launch is refused naming the path, the fake never starts,
/// and the HOME goes with the refused launch (N1).
#[test]
fn a_workspace_path_present_from_the_start_is_checked_only_after_the_home_is_made() {
    let mut world = World::new();
    let input = world.input(None);
    let planted = plant(&world.seat().canonicalize().unwrap(), ".agents/plugins");
    let mut calls = 0;
    let counted = |homes: Homes<'_>, written: Written<'_>| {
        calls += 1;
        home::made(homes, written)
    };
    let prepared = prepared_by(&mut world, &input, counted);
    let prepared = prepared.expect("the HOME's making was not reached with the path present");
    assert_eq!(calls, 1);
    assert_eq!(refused_for(&prepared, &input), planted);
    assert!(!world.started());
    drop(prepared);
    assert!(world.homes_gone());
}

/// A pinned `SUCCESS` whose response is empty, or blanks only, fails through
/// the driver with the status first and then the empty response (M4).
#[test]
fn an_empty_or_blank_response_fails_through_the_driver_naming_its_status_first() {
    let (init, step, _) = turn_rows();
    for response in ["", "   "] {
        let row = json!({"event": "result", "result": {"status": "SUCCESS", "response": response}});
        let rows = stream(&[init, step, &row.to_string()]);
        let turn = driven((&rows, "", 0), FLASH, &[]).turn;
        assert_eq!(turn.map(drop), Err(EMPTY_WORDS.to_string()), "{response:?}");
    }
}
