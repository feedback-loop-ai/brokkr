//! Decision 0104 ruling 2: which attempt the shared-local floor reaches.
//! Each test plants the host file and the attempt's sealed routes, reads
//! the deadline, and — for the words — drives one fake driver past it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use brokkr_core::envelope::EventType;
use brokkr_protocol::{AttemptOutcome, AttemptReport, Cleanup};
use serde_json::json;

use super::super::{DriverRun, Selection};
use super::deadline::{Cause, Deadline};
use crate::agents::{Candidate, HarnessHands, Lowering};
use crate::bundle::{Invocation, Limits, SeatBody, SeatClass, SiteFacts};
use crate::capabilities::manifest::{ModelPins, Served};
use crate::engine::tests::{bundle, templated};
use crate::engine::Engine;

fn link(provider: &str, model: &str) -> Candidate {
    Candidate {
        agent: "worker".into(),
        model: model.into(),
        effort: None,
        provider: provider.into(),
        argv: vec!["unused".into()],
        hands_fragment: Vec::new(),
        harness: HarnessHands::default(),
        resume: Default::default(),
        lowering: Lowering::Unavailable,
        hands_notice: None,
        route_overlay: None,
    }
}

fn served(route: Option<&str>) -> ModelPins {
    ModelPins::Read(vec![Served {
        id: "concrete".into(),
        route: route.map(str::to_string),
    }])
}

fn facts(chain: Vec<Candidate>, pins: Vec<ModelPins>) -> SiteFacts {
    SiteFacts {
        chain,
        pins,
        ..Default::default()
    }
}

fn selected(entries: &[(Option<&str>, Candidate)]) -> Selection {
    let mut selection = Selection::new();
    for (site, candidate) in entries {
        selection.insert(site.map(str::to_string), candidate.clone());
    }
    selection
}

/// A v3 host whose `dsh/spark-glm` route is shared-local with `floor`,
/// or without one, and whose `dsh/spark` route is cloud.
fn host(dir: &Path, floor: Option<u64>) -> PathBuf {
    let mut shared = json!({"ceiling": 1, "class": "shared-local"});
    if let Some(floor) = floor {
        shared["timeout_floor_seconds"] = json!(floor);
    }
    let host = json!({
        "schema": "forge.host/v3",
        "providers": {"dsh": {"ceiling": 1, "routes": {
            "spark": {"ceiling": 1, "class": "cloud"},
            "spark-glm": shared,
        }}},
        "floors": [{"place": "temp", "floor_bytes": 1}],
        "boxed_builds": {"ceiling": 1},
    });
    let file = dir.join("host.json");
    std::fs::write(&file, host.to_string()).unwrap();
    file
}

/// The one `work` single seat's deadline over `(sites, selection)`.
fn deadline(
    host: Option<&Path>,
    sites: &BTreeMap<String, SiteFacts>,
    selection: &Selection,
    authored: u64,
) -> Deadline {
    Deadline::of(host, sites, "work", selection, authored)
}

fn single_facts(route: &str) -> BTreeMap<String, SiteFacts> {
    let link = link("dsh", "glm-flash");
    let mut sites = BTreeMap::new();
    sites.insert("work".into(), facts(vec![link], vec![served(Some(route))]));
    sites
}

/// The one `work` single's deadline over the shared-local Spark route.
fn seat(host: Option<&Path>, authored: u64) -> Deadline {
    let sites = single_facts("spark-glm");
    let selection = selected(&[(None, link("dsh", "glm-flash"))]);
    deadline(host, &sites, &selection, authored)
}

const RAISED: &str = "the shared-local floor host.json declares for dsh/spark-glm; the seat's \
                      own limit is 3600 seconds";

#[test]
fn a_shared_local_floor_raises_a_shorter_authored_limit() {
    let dir = tempfile::tempdir().unwrap();
    let host = host(dir.path(), Some(7200));
    let deadline = seat(Some(&host), 3600);
    assert_eq!(deadline.seconds(), 7200);
    assert_eq!(deadline.explanation().as_deref(), Some(RAISED));
}

#[test]
fn a_longer_authored_limit_keeps_its_length() {
    let dir = tempfile::tempdir().unwrap();
    let host = host(dir.path(), Some(1800));
    let deadline = seat(Some(&host), 5400);
    assert_eq!(deadline.seconds(), 5400);
    assert_eq!(deadline.explanation(), None);
}

#[test]
fn no_host_file_keeps_the_seats_limit() {
    let deadline = seat(None, 3600);
    assert_eq!(deadline.seconds(), 3600);
    assert_eq!(deadline.explanation(), None);
}

#[test]
fn a_placed_but_absent_host_file_keeps_the_seats_limit() {
    let dir = tempfile::tempdir().unwrap();
    let deadline = seat(Some(&dir.path().join("absent.json")), 3600);
    assert_eq!(deadline.seconds(), 3600);
    assert_eq!(deadline.explanation(), None);
}

#[test]
fn an_unreadable_host_file_keeps_the_seats_limit_and_names_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let deadline = seat(Some(dir.path()), 3600);
    assert_eq!(deadline.seconds(), 3600);
    let words = deadline.explanation().unwrap();
    assert!(
        words.contains(&format!(
            "the host configuration at {} cannot be read",
            dir.path().display()
        )),
        "{words}"
    );
}

#[test]
fn an_invalid_host_file_keeps_the_seats_limit_and_names_why() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("host.json");
    std::fs::write(&file, "not json").unwrap();
    let deadline = seat(Some(&file), 3600);
    assert_eq!(deadline.seconds(), 3600);
    let words = deadline.explanation().unwrap();
    assert!(
        words.contains("is not a valid host configuration file"),
        "{words}"
    );
}

#[test]
fn a_cloud_link_after_a_shared_local_one_keeps_the_seats_limit() {
    let dir = tempfile::tempdir().unwrap();
    let host = host(dir.path(), Some(7200));
    let (shared, cloud) = (link("dsh", "glm-flash"), link("dsh", "glm"));
    let mut sites = BTreeMap::new();
    sites.insert(
        "work".into(),
        facts(
            vec![shared, cloud.clone()],
            vec![served(Some("spark-glm")), served(Some("spark"))],
        ),
    );
    // The attempt fell back to the cloud link: only that candidate is
    // launched, and the shared-local floor on the link before it is not
    // its own.
    let selection = selected(&[(None, cloud)]);
    let deadline = deadline(Some(&host), &sites, &selection, 3600);
    assert_eq!(deadline.seconds(), 3600);
    assert_eq!(deadline.explanation(), None);
}

#[test]
fn a_panel_takes_the_highest_floor_of_its_launched_members() {
    let dir = tempfile::tempdir().unwrap();
    let host = host(dir.path(), Some(7200));
    let (cloud, shared) = (link("dsh", "glm-flash"), link("dsh", "glm"));
    let mut sites = BTreeMap::new();
    sites.insert(
        "work:cloud".into(),
        facts(vec![cloud.clone()], vec![served(Some("spark"))]),
    );
    sites.insert(
        "work:spark".into(),
        facts(vec![shared.clone()], vec![served(Some("spark-glm"))]),
    );
    // The cloud member is first, so taking the first member's floor would
    // keep the seat's limit; the attempt takes the highest instead.
    let selection = selected(&[(Some("cloud"), cloud), (Some("spark"), shared)]);
    let deadline = deadline(Some(&host), &sites, &selection, 3600);
    assert_eq!(deadline.seconds(), 7200);
    assert_eq!(deadline.explanation().as_deref(), Some(RAISED));
}

#[test]
fn a_panel_keeps_the_higher_floor_when_a_later_member_ties_it() {
    let dir = tempfile::tempdir().unwrap();
    let host = host(dir.path(), Some(7200));
    let (first, second) = (link("dsh", "glm-flash"), link("dsh", "glm"));
    let mut sites = BTreeMap::new();
    for (member, candidate) in [("first", first.clone()), ("second", second.clone())] {
        sites.insert(
            format!("work:{member}"),
            facts(vec![candidate], vec![served(Some("spark-glm"))]),
        );
    }
    let selection = selected(&[(Some("first"), first), (Some("second"), second)]);
    let deadline = deadline(Some(&host), &sites, &selection, 3600);
    assert_eq!(deadline.seconds(), 7200);
}

#[test]
fn a_sequence_takes_its_one_shared_local_steps_floor() {
    let dir = tempfile::tempdir().unwrap();
    let host = host(dir.path(), Some(7200));
    let step = link("dsh", "glm-flash");
    let mut sites = BTreeMap::new();
    sites.insert(
        "work:step".into(),
        facts(vec![step.clone()], vec![served(Some("spark-glm"))]),
    );
    let selection = selected(&[(Some("step"), step)]);
    let deadline = deadline(Some(&host), &sites, &selection, 3600);
    assert_eq!(deadline.seconds(), 7200);
}

#[test]
fn an_unsealed_site_pin_or_route_keeps_the_seats_limit() {
    let dir = tempfile::tempdir().unwrap();
    let host = host(dir.path(), Some(7200));
    let candidate = link("dsh", "glm-flash");
    let selection = selected(&[(None, candidate.clone())]);
    // No sealed site at all.
    assert_eq!(
        deadline(Some(&host), &BTreeMap::new(), &selection, 3600).seconds(),
        3600
    );
    // A pin whose route is unreadable, and one that names no route.
    for pin in [ModelPins::Unreadable(vec!["--model".into()]), served(None)] {
        let mut sites = BTreeMap::new();
        sites.insert("work".into(), facts(vec![candidate.clone()], vec![pin]));
        assert_eq!(
            deadline(Some(&host), &sites, &selection, 3600).seconds(),
            3600
        );
    }
    // A chain that does not hold the launched candidate.
    let mut sites = BTreeMap::new();
    sites.insert(
        "work".into(),
        facts(vec![link("dsh", "other")], vec![served(Some("spark-glm"))]),
    );
    assert_eq!(
        deadline(Some(&host), &sites, &selection, 3600).seconds(),
        3600
    );
    // Pins shorter than the chain: the launched link has none to read.
    let mut sites = BTreeMap::new();
    sites.insert(
        "work".into(),
        facts(
            vec![link("dsh", "other"), candidate.clone()],
            vec![served(Some("spark-glm"))],
        ),
    );
    assert_eq!(
        deadline(Some(&host), &sites, &selection, 3600).seconds(),
        3600
    );
}

/// The `work` single's deadline when `candidate` launches over `chain`
/// with `pins`.
fn launched(
    host: &Path,
    chain: Vec<Candidate>,
    candidate: Candidate,
    pins: Vec<ModelPins>,
    authored: u64,
) -> Deadline {
    let mut sites = BTreeMap::new();
    sites.insert("work".into(), facts(chain, pins));
    let selection = selected(&[(None, candidate)]);
    deadline(Some(host), &sites, &selection, authored)
}

#[test]
fn a_floor_lookup_for_an_undeclared_provider_or_route_keeps_the_seats_limit() {
    let dir = tempfile::tempdir().unwrap();
    let file = host(dir.path(), Some(7200));
    // The host declares dsh's shared-local spark-glm route with a
    // 7200-second floor. A launched candidate on a provider or route the
    // file does not declare reads no floor, so the seat's own limit stays.
    for (provider, route) in [("undeclared", "spark-glm"), ("dsh", "undeclared")] {
        let candidate = link(provider, "glm-flash");
        let read = launched(
            &file,
            vec![candidate.clone()],
            candidate,
            vec![served(Some(route))],
            3600,
        );
        assert_eq!(read.seconds(), 3600, "{provider}/{route}");
        assert_eq!(read.explanation(), None, "{provider}/{route}");
    }
}

#[test]
fn a_short_or_empty_pin_keeps_the_seats_limit() {
    let dir = tempfile::tempdir().unwrap();
    let file = host(dir.path(), Some(7200));
    let candidate = link("dsh", "glm-flash");
    // An empty pins list, and one shorter than the chain whose launched
    // link is past its end: neither has a route for the candidate, so
    // neither reads a floor.
    for (chain, pins) in [
        (vec![candidate.clone()], Vec::new()),
        (
            vec![link("dsh", "other"), candidate.clone()],
            vec![served(Some("spark-glm"))],
        ),
    ] {
        let read = launched(&file, chain, candidate.clone(), pins, 3600);
        assert_eq!(read.seconds(), 3600);
        assert_eq!(read.explanation(), None);
    }
}

fn killed(reason: &str) -> AttemptReport {
    AttemptReport {
        outcome: AttemptOutcome::Indeterminate {
            reason: reason.into(),
        },
        refused: None,
        refusal: None,
        cleanup: Cleanup::Settled,
        session_ref: None,
        checkpoints: Vec::new(),
        stderr: String::new(),
        accepted: true,
        deadline_killed: true,
    }
}

#[test]
fn the_words_name_the_floor_and_the_seat_when_the_floor_ended_the_attempt() {
    let mut run = DriverRun::Ran(killed("attempt exceeded its 7200s deadline and was killed"));
    let deadline = Deadline {
        seconds: 7200,
        cause: Some(Cause::Raised {
            provider: "dsh".into(),
            route: "spark-glm".into(),
            authored: 3600,
        }),
    };
    deadline.name(&mut run);
    let DriverRun::Ran(report) = run else {
        panic!("a ran attempt stayed ran");
    };
    assert_eq!(
        report.outcome,
        AttemptOutcome::Indeterminate {
            reason: "the seat's 7200-second deadline: the shared-local floor host.json declares \
                     for dsh/spark-glm; the seat's own limit is 3600 seconds; its process tree \
                     is gone, and what it did is unknown"
                .into()
        }
    );
}

#[test]
fn an_attempt_the_seat_or_another_end_ended_is_not_renamed() {
    // No raised cause: the seat's own limit governed.
    let mut own = DriverRun::Ran(killed("attempt exceeded its 3600s deadline and was killed"));
    Deadline {
        seconds: 3600,
        cause: None,
    }
    .name(&mut own);
    let DriverRun::Ran(own) = own else {
        panic!("a ran attempt stayed ran");
    };
    assert_eq!(
        own.outcome,
        AttemptOutcome::Indeterminate {
            reason: "attempt exceeded its 3600s deadline and was killed".into()
        }
    );
    // A driver that did not spawn has no report to rename.
    let mut unspawned = DriverRun::SpawnFailed("no driver".into());
    let raised = Deadline {
        seconds: 7200,
        cause: Some(Cause::Raised {
            provider: "dsh".into(),
            route: "spark-glm".into(),
            authored: 3600,
        }),
    };
    raised.name(&mut unspawned);
    assert!(matches!(unspawned, DriverRun::SpawnFailed(_)));
    // A run the deadline did not end, and one whose outcome is already a
    // failure, read exactly as before.
    let mut finished = DriverRun::Ran(AttemptReport {
        deadline_killed: false,
        ..killed("driver exited before accepting the attempt")
    });
    raised.name(&mut finished);
    let mut failed = DriverRun::Ran(AttemptReport {
        outcome: AttemptOutcome::Failed {
            error: "provider refused".into(),
        },
        ..killed("ignored")
    });
    raised.name(&mut failed);
    assert!(matches!(
        failed,
        DriverRun::Ran(AttemptReport {
            outcome: AttemptOutcome::Failed { .. },
            ..
        })
    ));
}

#[test]
fn an_unreadable_host_names_itself_in_a_failed_attempts_words() {
    let mut run = DriverRun::Ran(AttemptReport {
        outcome: AttemptOutcome::Failed {
            error: "driver failed".into(),
        },
        ..killed("ignored")
    });
    Deadline {
        seconds: 3600,
        cause: Some(Cause::Unreadable {
            path: PathBuf::from("/var/tmp/host.json"),
            kind: std::io::ErrorKind::NotFound,
        }),
    }
    .name(&mut run);
    let DriverRun::Ran(report) = run else {
        panic!("a ran attempt stayed ran");
    };
    let AttemptOutcome::Failed { error } = report.outcome else {
        panic!("a failed attempt stayed failed");
    };
    assert_eq!(
        error,
        "driver failed; the host configuration at /var/tmp/host.json cannot be read: entity not \
         found"
    );
}

#[test]
fn an_invalid_host_names_itself_however_the_attempt_ended() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("host.json");
    std::fs::write(&file, "not json").unwrap();
    let cause = Cause::Invalid {
        error: crate::admission::read_host(&file).unwrap_err(),
    };
    let deadline = Deadline {
        seconds: 3600,
        cause: Some(cause),
    };
    // A spawn that never happened names the file too.
    let mut unspawned = DriverRun::SpawnFailed("driver did not spawn".into());
    deadline.name(&mut unspawned);
    let DriverRun::SpawnFailed(error) = unspawned else {
        panic!("a spawn failure stayed one");
    };
    assert!(
        error.contains(&format!(
            "driver did not spawn; the host configuration at {}",
            file.display()
        )),
        "{error}"
    );
    assert!(
        error.contains("is not a valid host configuration file"),
        "{error}"
    );
    // A success's words are not failure words, and are left alone.
    let mut succeeded = DriverRun::Ran(AttemptReport {
        outcome: AttemptOutcome::Succeeded {
            result: json!({"result": "complete"}),
        },
        ..killed("ignored")
    });
    deadline.name(&mut succeeded);
    let DriverRun::Ran(report) = succeeded else {
        panic!("a ran attempt stayed ran");
    };
    assert_eq!(
        report.outcome,
        AttemptOutcome::Succeeded {
            result: json!({"result": "complete"})
        }
    );
    // An indeterminate attempt names it too, after its own reason.
    let mut lost = DriverRun::Ran(killed("the driver vanished"));
    deadline.name(&mut lost);
    let DriverRun::Ran(report) = lost else {
        panic!("a ran attempt stayed ran");
    };
    let AttemptOutcome::Indeterminate { reason } = report.outcome else {
        panic!("an indeterminate attempt stayed indeterminate");
    };
    assert!(reason.starts_with("the driver vanished; "), "{reason}");
    assert!(
        reason.contains("is not a valid host configuration file"),
        "{reason}"
    );
}

#[test]
fn an_unplaced_host_error_is_its_own_words_without_a_source() {
    let deadline = Deadline {
        seconds: 60,
        cause: Some(Cause::Invalid {
            error: crate::admission::HostError::Unplaced,
        }),
    };
    assert_eq!(
        deadline.explanation().as_deref(),
        Some(
            "no host configuration can be found: neither XDG_CONFIG_HOME nor HOME names an \
             absolute directory"
        )
    );
}

/// The shell every test driver starts with: read the hello, declare the
/// capabilities, read the start, take its ids as `$eid` and `$aid`, and
/// accept. What the driver then does is the caller's tail.
const DRIVER_PRELUDE: &str = r#"read -r hello
printf '%s\n' '{"proto":"forge-driver/v1","msg_id":"c","type":"capabilities","driver":"test","version":"1","supports":[]}'
read -r start
case "$start" in *'"type":"start"'*) ;; *) exit 0 ;; esac
eid=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
aid=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
printf '%s,"type":"accepted","effect_id":"%s","attempt_id":"%s"}\n' '{"proto":"forge-driver/v1","msg_id":"a"' "$eid" "$aid"
"#;

/// A driver in `dir` named `name`, accepting and then running `tail`.
fn driver(dir: &Path, name: &str, tail: &str) -> Vec<String> {
    let path = dir.join(name);
    std::fs::write(&path, format!("{DRIVER_PRELUDE}{tail}")).unwrap();
    vec!["sh".into(), path.display().to_string()]
}

/// A driver that accepts and then sleeps until the watchdog kills it: the
/// attempt ends at its raised deadline with the engine's own words.
fn silent_driver(dir: &Path) -> Vec<String> {
    driver(dir, "slow-driver.sh", "sleep 30\n")
}

/// A driver that accepts and then fails normally, saying `driver failed`.
fn failed_driver(dir: &Path) -> Vec<String> {
    driver(
        dir,
        "failed-driver.sh",
        r#"printf '%s,"type":"result","effect_id":"%s","attempt_id":"%s","status":"failed","result":null,"error":"driver failed"}\n' '{"proto":"forge-driver/v1","msg_id":"r"' "$eid" "$aid"
"#,
    )
}

fn started(dir: &Path, body: SeatBody, limits: Limits) -> Engine {
    let store = brokkr_store::Store::open(&dir.join("forge.db")).unwrap();
    let mut compiled = bundle(dir, body);
    compiled.seats.get_mut("work").unwrap().limits = limits;
    std::fs::create_dir_all(dir.join("work")).unwrap();
    Engine::start(store, compiled, "Feature", Some(dir.join("work"))).unwrap()
}

/// The one `work` single seat running `script` under `timeout`, its
/// launched candidate pinned to the shared-local Spark route and its host
/// file `host` where one is placed (an absent or unreadable one keeps the
/// seat's limit).
fn attempt(dir: &Path, host: Option<PathBuf>, script: Vec<String>, timeout: u64) -> Engine {
    let link = templated(Candidate {
        agent: "worker".into(),
        model: "glm-flash".into(),
        effort: None,
        provider: "dsh".into(),
        argv: script.clone(),
        hands_fragment: Vec::new(),
        harness: HarnessHands::default(),
        resume: Default::default(),
        lowering: Lowering::Unavailable,
        hands_notice: None,
        route_overlay: None,
    });
    let body = SeatBody::Invocation(Invocation::Single {
        role_path: PathBuf::from("role.md"),
        command: script,
        candidates: vec![link.clone()],
        written: Vec::new(),
        class: SeatClass::Work,
    });
    let mut engine = started(
        dir,
        body,
        Limits {
            max_attempts: 1,
            timeout_seconds: timeout,
        },
    );
    engine.cooldown.host = host;
    engine.bundle.sites.insert(
        "work".into(),
        facts(vec![link], vec![served(Some("spark-glm"))]),
    );
    engine
}

/// The `field` of the first `event` the attempt journaled.
fn terminal(engine: &Engine, event: EventType, field: &str) -> String {
    engine
        .store
        .load(&engine.run_id)
        .unwrap()
        .iter()
        .find(|envelope| envelope.event_type == event)
        .unwrap_or_else(|| panic!("the attempt journaled no {event:?}"))
        .payload[field]
        .as_str()
        .unwrap()
        .to_string()
}

/// One slow-driver attempt against `floor`, end to end, and the reason
/// its terminal event carries.
fn slow_attempt(dir: &Path, floor: Option<u64>) -> String {
    let mut engine = attempt(dir, Some(host(dir, floor)), silent_driver(dir), 1);
    engine.drive().unwrap();
    terminal(&engine, EventType::EffectIndeterminate, "reason")
}

/// A slow-driver attempt whose provisioning spends `spent` seconds of its
/// budget (a stand-in, so the test need not broker a real plan), and the
/// protocol's words for the driver's deadline.
fn spent_attempt(dir: &Path, timeout: u64, spent: u64) -> String {
    let mut engine = attempt(dir, None, silent_driver(dir), timeout);
    // The start turn enters the phase; the next requests the effect. The
    // turn after that would execute it, so stop at the request.
    engine.drive_once().unwrap();
    engine.drive_once().unwrap();
    let run_id = engine.run_id.clone();
    let state = engine.replay.caught_up(&mut engine.store, &run_id).unwrap();
    let events = engine.replay.events.clone();
    let effect_id = events
        .iter()
        .find(|envelope| envelope.event_type == EventType::EffectRequested)
        .and_then(|envelope| envelope.payload["effect_id"].as_str())
        .expect("the turn requested the effect")
        .to_string();
    engine
        .execute_with(
            &events,
            &state,
            &effect_id,
            "work",
            Instant::now(),
            Box::new(
                |_: &mut Engine,
                 _: [&str; 3],
                 _: &mut Selection,
                 (_, bound): (Instant, &mut Duration),
                 _: crate::engine::broker::Observed| {
                    *bound = bound.saturating_sub(Duration::from_secs(spent));
                    Ok(true)
                },
            ),
        )
        .unwrap();
    terminal(&engine, EventType::EffectIndeterminate, "reason")
}

/// One normally failing attempt against the host path `host`, and the
/// error its terminal event carries.
fn failed_attempt(dir: &Path, host: PathBuf) -> String {
    let mut engine = attempt(dir, Some(host), failed_driver(dir), 3600);
    engine.drive().unwrap();
    terminal(&engine, EventType::EffectFailed, "error")
}

#[test]
fn a_fake_driver_that_outlives_a_raised_deadline_ends_with_both_numbers_and_the_source() {
    let dir = tempfile::tempdir().unwrap();
    let reason = slow_attempt(dir.path(), Some(2));
    assert!(reason.contains("the seat's 2-second deadline"), "{reason}");
    assert!(
        reason.contains("shared-local floor host.json declares for dsh/spark-glm"),
        "{reason}"
    );
    assert!(
        reason.contains("the seat's own limit is 1 seconds"),
        "{reason}"
    );
}

#[test]
fn a_single_seat_with_no_floor_keeps_todays_words() {
    let dir = tempfile::tempdir().unwrap();
    // The same driver, but the host declines to raise: the attempt stops
    // at the seat's own one second and the protocol's own words stand.
    let reason = slow_attempt(dir.path(), None);
    assert!(
        reason.starts_with("attempt exceeded its 1s deadline and was killed"),
        "{reason}"
    );
}

#[test]
fn the_driver_runs_on_what_provisioning_left_of_the_attempts_deadline() {
    let dir = tempfile::tempdir().unwrap();
    // The six-second attempt began three seconds before its provisioning,
    // so the driver gets about three seconds, not six. Truncation makes
    // the exact second vary; what binds is that the FULL bound is not
    // handed to the driver again.
    let reason = spent_attempt(dir.path(), 6, 3);
    assert!(
        reason.contains("deadline and was killed"),
        "the driver outlived its remaining budget: {reason}"
    );
    assert!(
        !reason.contains("its 6s deadline"),
        "the driver was handed the whole attempt bound, not what provisioning left: {reason}"
    );
}

#[test]
fn an_unreadable_host_names_itself_after_a_normal_failure() {
    let dir = tempfile::tempdir().unwrap();
    let error = failed_attempt(dir.path(), dir.path().to_path_buf());
    assert!(error.starts_with("driver failed;"), "{error}");
    assert!(
        error.contains(&format!(
            "the host configuration at {} cannot be read",
            dir.path().display()
        )),
        "{error}"
    );
}

#[test]
fn an_invalid_host_names_itself_after_a_normal_failure() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("host.json");
    std::fs::write(&file, "not json").unwrap();
    let error = failed_attempt(dir.path(), file);
    assert!(error.starts_with("driver failed;"), "{error}");
    assert!(
        error.contains("is not a valid host configuration file"),
        "{error}"
    );
}
