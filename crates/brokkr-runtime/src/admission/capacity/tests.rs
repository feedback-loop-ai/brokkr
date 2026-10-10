//! The machine's room for an entry: each check waits at its limit and
//! admits below it, naming the limit and what was measured; the host
//! configuration's absence, unreadability and invalidity are three
//! outcomes; and the first unmet check is the one recorded.

use std::cell::RefCell;
use std::error::Error as _;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use brokkr_core::EventType;
use brokkr_store::{Attribution, EntryId, NewEntry, QueueCommand};
use serde_json::{json, Value};

use super::super::{judge_within, pass, pass_within, HostError, Reason, Standing};
use super::*;
use crate::launch::{BundleSource, LaunchRequest, NewRun, RunMap};

const BY: Attribution<'static> = Attribution {
    operator: "vy",
    reason: "operator's word",
};

/// A workspace whose library holds four agents on two providers:
/// `alpha` serves `a1`, whose id carries no route; `beta` serves `b-local`
/// on its `spark-glm` route, `b-cloud` on its `cloud` route and `b-plain`
/// on none. Its bundle seats `work` and `review` on the agents the test
/// names.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    store: Store,
}

impl Fixture {
    fn new() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        for sub in ["agents/charters", "adapters", "bundle/roles"] {
            std::fs::create_dir_all(root.join(sub)).unwrap();
        }
        std::fs::write(root.join("agents/charters/work.md"), "# work\n").unwrap();
        std::fs::write(root.join("bundle/roles/work.md"), "# work\n").unwrap();
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/exec.json");
        std::fs::copy(repo, root.join("adapters/exec.json")).unwrap();
        let adapter = |provider: &str, models: Value| {
            json!({"provider": provider, "binary": format!("{provider}-cli"),
                   "driver": [format!("{provider}-cli"), "run"], "models": models,
                   "model_flag": "-m", "efforts": [], "effort_flag": "unsupported",
                   "tool_permissions": "unsupported", "mcp": "unsupported",
                   "native_capabilities": {"known": {}}})
        };
        let fixture = Fixture {
            store: Store::open(&root.join("forge.db")).unwrap(),
            _dir: dir,
            root,
        };
        fixture.write(
            "adapters/alpha.json",
            adapter("alpha", json!({"a1": "a-1"})),
        );
        let beta = json!({"b-local": "spark-glm/b-1", "b-cloud": "cloud/b-2", "b-plain": "b-3"});
        fixture.write("adapters/beta.json", adapter("beta", beta));
        for (agent, models) in [
            ("one", json!(["a1"])),
            ("local", json!(["b-local"])),
            ("cloudy", json!(["b-cloud"])),
            ("both", json!(["a1", "b-cloud"])),
        ] {
            let body = json!({"description": agent, "charter": "charters/work.md",
                              "models": models});
            fixture.write(&format!("agents/{agent}.json"), body);
        }
        let rule = |from: &str, result: &str, next: &str| {
            json!({"id": from.to_uppercase(), "from": from, "result": result, "next": next,
                   "reason": from})
        };
        let rules = [
            rule("work", "complete", "review"),
            rule("review", "clean", "done"),
        ];
        let phases = ["work", "review", "done"];
        let policy = json!({"phases": phases, "initial": phases[0], "terminal": [phases[2]],
                            "rules": rules});
        fixture.write("bundle/policy.json", policy);
        fixture.seat("one", "one");
        fixture
    }

    fn write(&self, relative: &str, body: Value) {
        let bytes = serde_json::to_vec_pretty(&body).unwrap();
        std::fs::write(self.root.join(relative), bytes).unwrap();
    }

    /// The bundle seats `work` on agent `work` and `review` on agent
    /// `review`.
    fn seat(&self, work: &str, review: &str) {
        self.bundle(json!({"results": ["clean"], "agent": review}), work);
    }

    /// The bundle seats `work` on agent `work`, and `review` as written.
    fn bundle(&self, review: Value, work: &str) {
        self.write(
            "bundle/bundle.json",
            json!({"name": "fixture", "policy": "policy.json", "seats": {
                "work": {"results": ["complete"], "agent": work},
                "review": review}}),
        );
    }

    /// The launch `brokkr queue add --bundle bundle` makes here.
    fn launch(&self) -> QueuedLaunch {
        let request = LaunchRequest {
            workspace: self.root.clone(),
            bundle: BundleSource::Dir("bundle".into()),
            journal: self.root.join("forge.db"),
            repo: None,
            secrets: None,
            host_path: OsString::new(),
        };
        let run = NewRun {
            feature: "f".into(),
            map: RunMap::Unmapped,
            dispatch: None,
        };
        QueuedLaunch::of(&request, &run).unwrap()
    }

    /// Queue that launch in this fixture's journal.
    fn queue(&mut self) -> EntryId {
        let launch = self.launch();
        enqueue(&mut self.store, &launch)
    }

    /// A run that is running now on the bundle as it compiles here.
    fn started(&mut self, name: &str) {
        let pinned = self.launch().compiled().unwrap().manifest;
        self.running(name, &pinned);
    }

    /// The seat scratch an entry queued here measures by default.
    fn scratch(&self) -> PathBuf {
        self.root.join(".forge/scratch")
    }

    /// A run that is running now and whose manifest says `manifest`.
    fn running(&mut self, name: &str, manifest: &Value) {
        self.store.create_run(name, "f", "b", manifest).unwrap();
        let started = json!({"feature": "f"});
        (self.store)
            .append_next(name, EventType::RunStarted, started, None, None)
            .unwrap();
    }

    /// A running run whose compile pinned each `(provider, model id)`,
    /// boxed or not.
    fn seating(&mut self, name: &str, seats: &[(&str, &str)], boxed: bool) {
        let candidates: Vec<Value> = (seats.iter())
            .map(|(provider, id)| json!({"provider": provider, "model_pins": {"read": [id]}}))
            .collect();
        let mut manifest = json!({"capabilities": {"sites": {"work": {"candidates": candidates}}}});
        if boxed {
            manifest["boundary"] = json!({"work": "namespace"});
        }
        self.running(name, &manifest);
    }

    /// The host configuration this fixture declares, written.
    fn declare(&self, host: &Value) -> PathBuf {
        let file = self.root.join("host.json");
        std::fs::write(&file, host.to_string()).unwrap();
        file
    }

    /// Each entry's reasons under `pass_within`, the scratch filesystem
    /// reporting `free` bytes.
    fn reasons(&self, file: &Path, free: u64) -> Vec<Vec<Reason>> {
        let probe = |_: &Path| Ok(free);
        let host = Host { file, free: &probe };
        verdicts(pass_within(&self.store, &host).unwrap())
    }

    /// The one entry's first unmet capacity check, or `None` when the
    /// machine has room.
    fn room(&self, file: &Path) -> Option<Capacity> {
        let mut reasons = self.reasons(file, PLENTY).remove(0);
        reasons.pop().map(|reason| match reason {
            Reason::Capacity(capacity) => capacity,
            other => panic!("not a capacity reason: {other}"),
        })
    }
}

const PLENTY: u64 = 1 << 40;

/// Queue `launch` in `store`.
fn enqueue(store: &mut Store, launch: &QueuedLaunch) -> EntryId {
    let payload = launch.encode().unwrap();
    let entry = NewEntry {
        payload: &payload,
        priority: 0,
        waits: &[],
    };
    store.queue_add(entry, BY).unwrap()
}

/// A plenty-free host reading the configuration at `file`, and the
/// verdict of `pass_within` over `store` on it.
fn passed(store: &Store, file: &Path) -> Result<Vec<Judged>, AdmissionError> {
    let probe = |_: &Path| Ok(PLENTY);
    pass_within(store, &Host { file, free: &probe })
}

fn verdicts(judged: Vec<Judged>) -> Vec<Vec<Reason>> {
    judged
        .into_iter()
        .filter_map(|judged| Some(judged.verdict?.reasons))
        .collect()
}

/// A host configuration declaring `providers`, a scratch floor of a
/// kibibyte and two boxed builds.
fn host(providers: Value) -> Value {
    json!({"schema": "forge.host/v1", "providers": providers,
           "scratch": {"floor_bytes": 1024}, "boxed_builds": {"ceiling": 2}})
}

/// `alpha` and `beta` at `alpha` and `beta` runs, `beta`'s routes as
/// given.
fn ceilings(alpha: u32, beta: u32, routes: Value) -> Value {
    host(json!({"alpha": {"ceiling": alpha}, "beta": {"ceiling": beta, "routes": routes}}))
}

fn route(ceiling: u32, class: &str) -> Value {
    json!({"ceiling": ceiling, "class": class})
}

fn full(provider: &str, running: usize, ceiling: u32) -> Option<Capacity> {
    Some(Capacity::ProviderFull {
        provider: provider.into(),
        running,
        ceiling,
    })
}

fn route_full(route: &str, class: RouteClass, running: usize, ceiling: u32) -> Option<Capacity> {
    routed_full("beta", route, class, running, ceiling)
}

fn routed_full(
    provider: &str,
    route: &str,
    class: RouteClass,
    running: usize,
    ceiling: u32,
) -> Option<Capacity> {
    Some(Capacity::RouteFull {
        provider: provider.into(),
        route: route.into(),
        class,
        running,
        ceiling,
    })
}

#[test]
fn no_host_configuration_waits_each_entry_capacity_judges_and_nothing_else_reads_it() {
    let mut fixture = Fixture::new();
    let (held, free) = (fixture.queue(), fixture.queue());
    (fixture.store)
        .queue_command(held, QueueCommand::Hold, BY)
        .unwrap();
    let file = fixture.root.join("config/brokkr/host.json");
    let reasons = fixture.reasons(&file, PLENTY);
    let undeclared = Reason::Capacity(Capacity::Undeclared(file.clone()));
    assert_eq!(
        reasons,
        vec![vec![Reason::OperatorHold], vec![undeclared.clone()]]
    );
    assert_eq!(
        (undeclared.to_string(), undeclared.holds()),
        (
            format!(
                "no host configuration at {} declares capacity",
                file.display()
            ),
            false
        )
    );

    // An entry something else stops is never measured, so a pass with
    // none to measure reads no host configuration, even an invalid one.
    (fixture.store)
        .queue_command(free, QueueCommand::Drop, BY)
        .unwrap();
    let invalid = fixture.declare(&json!({}));
    assert_eq!(
        fixture.reasons(&invalid, PLENTY),
        vec![vec![Reason::OperatorHold]]
    );
}

#[test]
fn a_host_configuration_that_cannot_be_read_holds_and_is_never_read_as_absent() {
    let mut fixture = Fixture::new();
    fixture.queue();
    let file = fixture.root.join("host.json");
    let unreadable = |kind: io::ErrorKind| {
        let reason = Reason::Capacity(Capacity::Unreadable {
            path: file.clone(),
            kind,
        });
        let says = format!(
            "the host configuration at {} cannot be read: {kind}",
            file.display()
        );
        assert_eq!(reason.to_string(), says);
        assert!(reason.holds(), "{says}");
        vec![vec![reason]]
    };
    std::os::unix::fs::symlink("host.json", &file).unwrap();
    let looping = std::fs::read(&file).unwrap_err().kind();
    assert_eq!(fixture.reasons(&file, PLENTY), unreadable(looping));
    std::fs::remove_file(&file).unwrap();
    std::os::unix::fs::symlink("gone.json", &file).unwrap();
    let dangling = unreadable(io::ErrorKind::NotFound);
    assert_eq!(fixture.reasons(&file, PLENTY), dangling);
    std::fs::remove_file(&file).unwrap();
    std::fs::create_dir(&file).unwrap();
    let directory = unreadable(io::ErrorKind::IsADirectory);
    assert_eq!(fixture.reasons(&file, PLENTY), directory);
    let standing = |file: &Path| {
        passed(&fixture.store, file).unwrap()[0]
            .verdict
            .as_ref()
            .unwrap()
            .standing()
    };
    assert_eq!(standing(&file), Standing::Held);

    // A directory on the path that is a dangling link leaves the file
    // unreachable, not absent.
    std::os::unix::fs::symlink("unmounted", fixture.root.join("stowed")).unwrap();
    let file = fixture.root.join("stowed/host.json");
    let unreached = Reason::Capacity(Capacity::Unreadable {
        path: file.clone(),
        kind: io::ErrorKind::NotFound,
    });
    assert_eq!(fixture.reasons(&file, PLENTY), vec![vec![unreached]]);
}

#[test]
fn an_invalid_host_configuration_refuses_the_pass_naming_the_problem() {
    let mut fixture = Fixture::new();
    fixture.queue();
    let valid = ceilings(1, 1, json!({}));
    let refused = |host: Value| {
        let file = fixture.declare(&host);
        let error = passed(&fixture.store, &file).unwrap_err();
        let says = format!(
            "the host configuration at {} is not a valid forge.host/v1 file",
            file.display()
        );
        assert!(
            matches!(&error, AdmissionError::Host(HostError::Invalid { path, .. })
            if *path == file)
        );
        assert_eq!(error.to_string(), says);
        error.source().unwrap().to_string()
    };
    let with = |pointer: &str, value: Value| {
        let mut host = valid.clone();
        *host.pointer_mut(pointer).unwrap() = value;
        host
    };
    let mut unknown = valid.clone();
    unknown["cooldowns"] = json!({});
    let mut keyed = valid.clone();
    keyed["providers"]["alpha"]["burst"] = json!(2);
    let cases = [
        (unknown, "unknown field `cooldowns`, expected one of `schema`, `providers`, `scratch`, `boxed_builds` at line 1 column 41"),
        (keyed, "unknown field `burst`, expected `ceiling` or `routes` at line 1 column 59"),
        (with("/providers/alpha/ceiling", json!(0)), "invalid value: integer `0`, expected a nonzero u32 at line 1 column 63"),
        (with("/providers/alpha/ceiling", json!(-1)), "invalid value: integer `-1`, expected a nonzero u32 at line 1 column 64"),
        (with("/providers/alpha/ceiling", json!(4_294_967_296_u64)), "invalid value: integer `4294967296`, expected a nonzero u32 at line 1 column 72"),
        (with("/boxed_builds/ceiling", json!(0)), "invalid value: integer `0`, expected a nonzero u32 at line 1 column 28"),
        (with("/scratch/floor_bytes", json!(0)), "invalid value: integer `0`, expected a nonzero u64 at line 1 column 150"),
        (with("/scratch", json!({"floor_bytes": 1, "path": "tmp"})), "the scratch path tmp is not absolute at line 1 column 164"),
        (with("/scratch", json!({"floor_bytes": 1, "path": null})), "invalid type: null, expected path string at line 1 column 162"),
        (with("/schema", json!("forge.host/v2")), "unknown variant `forge.host/v2`, expected `forge.host/v1` at line 1 column 123"),
        (with("/providers/beta/routes", json!({"spark-glm": route(1, "local")})), "unknown variant `local`, expected `cloud` or `shared-local` at line 1 column 135"),
    ];
    let said: Vec<String> = cases
        .iter()
        .map(|(host, _)| refused(host.clone()))
        .collect();
    assert_eq!(said, cases.map(|(_, says)| says));
}

#[test]
fn provider_and_cloud_route_concurrency_waits_at_each_ceiling_and_admits_below_it() {
    let mut fixture = Fixture::new();
    fixture.seat("one", "cloudy");
    fixture.queue();
    let cloud = |ceiling| json!({"cloud": route(ceiling, "cloud")});
    let file = fixture.declare(&ceilings(2, 3, cloud(2)));
    assert_eq!(fixture.room(&file), None);

    // A run counts once against each provider and route it seats: `r1`
    // seats beta on two lanes, and alpha's one lane twice.
    let seats = [
        ("beta", "b-3"),
        ("beta", "cloud/b-2"),
        ("alpha", "a-1"),
        ("alpha", "a-1"),
    ];
    fixture.seating("r1", &seats, false);
    assert_eq!(fixture.room(&file), None);
    fixture.seating("r2", &[("alpha", "a-1"), ("beta", "cloud/b-2")], false);
    assert_eq!(fixture.room(&file), full("alpha", 2, 2));
    let file = fixture.declare(&ceilings(3, 3, cloud(2)));
    assert_eq!(
        fixture.room(&file),
        route_full("cloud", RouteClass::Cloud, 2, 2)
    );
    let file = fixture.declare(&ceilings(3, 2, cloud(3)));
    assert_eq!(fixture.room(&file), full("beta", 2, 2));
    let file = fixture.declare(&ceilings(3, 3, cloud(3)));
    assert_eq!(fixture.room(&file), None);

    // The order is fixed: a provider at its ceiling is named before a
    // scratch filesystem below its floor.
    let file = fixture.declare(&ceilings(2, 3, cloud(3)));
    let first = fixture.reasons(&file, 0).remove(0);
    assert_eq!(first, vec![Reason::Capacity(full("alpha", 2, 2).unwrap())]);
    assert_eq!(
        first[0].to_string(),
        "provider alpha is full: 2 running against a ceiling of 2"
    );
    let routed = Reason::Capacity(route_full("cloud", RouteClass::Cloud, 1, 1).unwrap());
    assert_eq!(
        routed.to_string(),
        "cloud route cloud of provider beta is full: 1 running against a ceiling of 1"
    );
}

#[test]
fn a_shared_local_route_is_judged_by_its_own_ceiling_in_place_of_its_providers() {
    let mut fixture = Fixture::new();
    fixture.seat("local", "local");
    fixture.queue();
    let routes = json!({"spark-glm": route(2, "shared-local"), "cloud": route(1, "cloud")});
    let file = fixture.declare(&ceilings(1, 1, routes));
    // Beta's one cloud run fills beta's ceiling, and the shared-local
    // route does not count against it.
    fixture.seating("cloud", &[("beta", "cloud/b-2")], false);
    fixture.seating("local", &[("beta", "spark-glm/b-1")], false);
    assert_eq!(fixture.room(&file), None);
    fixture.seating("local-2", &[("beta", "spark-glm/b-1")], false);
    let shared = route_full("spark-glm", RouteClass::SharedLocal, 2, 2);
    assert_eq!(fixture.room(&file), shared);
    assert_eq!(
        shared.unwrap().to_string(),
        "shared-local route spark-glm of provider beta is full: 2 running against a ceiling of 2"
    );

    // Two shared-local runs leave beta's provider ceiling with the one
    // cloud run, so a cloud seat is judged by that one.
    fixture.seat("cloudy", "cloudy");
    let routes = json!({"spark-glm": route(2, "shared-local"), "cloud": route(5, "cloud")});
    let file = fixture.declare(&ceilings(1, 2, routes));
    assert_eq!(fixture.room(&file), None);
}

#[test]
fn a_provider_or_route_the_host_does_not_declare_waits_as_undeclared() {
    let mut fixture = Fixture::new();
    fixture.seat("one", "local");
    fixture.queue();
    let file = fixture.declare(&host(json!({"beta": {"ceiling": 1}})));
    let undeclared = Capacity::ProviderUndeclared("alpha".into());
    assert_eq!(fixture.room(&file), Some(undeclared.clone()));
    let file = fixture.declare(&ceilings(1, 1, json!({"cloud": route(1, "cloud")})));
    let route = Capacity::RouteUndeclared {
        provider: "beta".into(),
        route: "spark-glm".into(),
    };
    assert_eq!(fixture.room(&file), Some(route.clone()));
    // A fallback the chain may seat is measured as the first is.
    fixture.seat("both", "one");
    let file = fixture.declare(&host(json!({"alpha": {"ceiling": 1}})));
    let fallback = Capacity::ProviderUndeclared("beta".into());
    assert_eq!(fixture.room(&file), Some(fallback));
    assert_eq!(
        [undeclared, route].map(|reason| (reason.to_string(), reason.holds())),
        [
            (
                "the host configuration declares no ceiling for provider alpha".to_string(),
                false
            ),
            (
                "the host configuration declares no ceiling for route spark-glm of provider beta"
                    .to_string(),
                false
            ),
        ]
    );
}

#[test]
fn the_scratch_floor_measures_the_seat_scratch_or_the_declared_path_and_waits_below_it() {
    let mut fixture = Fixture::new();
    fixture.queue();
    let declared = ceilings(1, 1, json!({}));
    let asked = RefCell::new(Vec::new());
    let reasons = |host: &Value, free: io::Result<u64>| {
        let file = fixture.declare(host);
        let free = RefCell::new(Some(free));
        let probe = |path: &Path| {
            asked.borrow_mut().push(path.to_path_buf());
            free.borrow_mut().take().unwrap()
        };
        let host = Host {
            file: &file,
            free: &probe,
        };
        verdicts(pass_within(&fixture.store, &host).unwrap()).remove(0)
    };
    assert_eq!(reasons(&declared, Ok(1024)), vec![]);
    let scratch = fixture.scratch();
    let low = Capacity::ScratchLow {
        path: scratch.clone(),
        free: 1023,
        floor: 1024,
    };
    assert_eq!(
        reasons(&declared, Ok(1023)),
        vec![Reason::Capacity(low.clone())]
    );
    let mut elsewhere = declared.clone();
    elsewhere["scratch"]["path"] = json!("/var/tmp");
    let gone = io::Error::from(io::ErrorKind::NotFound);
    let unmeasured = Capacity::ScratchUnmeasured {
        path: "/var/tmp".into(),
        kind: io::ErrorKind::NotFound,
    };
    assert_eq!(
        reasons(&elsewhere, Err(gone)),
        vec![Reason::Capacity(unmeasured.clone())]
    );
    assert_eq!(
        asked.into_inner(),
        [scratch.clone(), scratch.clone(), "/var/tmp".into()]
    );
    assert_eq!(
        [low, unmeasured].map(|reason| (reason.to_string(), reason.holds())),
        [
            (
                format!(
                    "the scratch filesystem at {} has 1023 bytes free, below its floor of 1024",
                    scratch.display()
                ),
                false
            ),
            (
                "the scratch filesystem at /var/tmp cannot be measured: entity not found"
                    .to_string(),
                false
            ),
        ]
    );
}

#[test]
fn a_boxed_entry_waits_while_the_boxed_builds_running_reach_their_ceiling() {
    let mut fixture = Fixture::new();
    let boxed = json!({"results": ["clean"], "role": "roles/work.md",
        "driver": {"command": ["{brokkr}", "driver", "exec", "--", "true"]},
        "hands": {"kind": "workspace", "network": false, "binds": []}});
    fixture.bundle(boxed, "one");
    fixture.queue();
    let declared = host(json!({"alpha": {"ceiling": 9}, "exec": {"ceiling": 9}}));
    let file = fixture.declare(&declared);
    fixture.seating("boxed", &[("alpha", "a-1")], true);
    fixture.seating("open", &[("alpha", "a-1")], false);
    assert_eq!(fixture.room(&file), None);
    fixture.seating("boxed-2", &[("alpha", "a-1")], true);
    let full = Capacity::BoxedFull {
        running: 2,
        ceiling: 2,
    };
    assert_eq!(fixture.room(&file), Some(full.clone()));
    assert_eq!(
        full.to_string(),
        "boxed builds are full: 2 running against a ceiling of 2"
    );

    // An entry that builds in no box is not judged by the boxed ceiling.
    fixture.seat("one", "one");
    assert_eq!(fixture.room(&file), None);
}

/// The one entry's reason under `file`, its word and whether it holds,
/// where it is that `RunUnmeasured` or `Unmeasured` and its measurement
/// error is of the kind `of` names.
fn unmeasured(fixture: &Fixture, file: &Path, of: fn(&Unmeasurable) -> bool) -> (String, bool) {
    let reason = fixture.room(file).unwrap();
    let why = match &reason {
        Capacity::RunUnmeasured { why, .. } | Capacity::Unmeasured(why) => why,
        other => panic!("not unmeasured: {other}"),
    };
    assert!(of(why), "{why:?}");
    // A reason's error is shared by its clones, and equal to nothing else.
    assert_eq!(reason.clone(), reason);
    (reason.to_string(), reason.holds())
}

#[test]
fn what_an_entry_or_a_running_run_seats_must_be_measured() {
    let file = |fixture: &Fixture| fixture.declare(&ceilings(9, 9, json!({})));
    let beside = |run: &str, pinned: Value, of| {
        let mut fixture = Fixture::new();
        fixture.queue();
        fixture.running(run, &pinned);
        unmeasured(&fixture, &file(&fixture), of)
    };
    let manifest = |candidate: Value| json!({"capabilities": {"sites": {"work": {"candidates": [candidate]}}}});
    let manifested = |why: &Unmeasurable| matches!(why, Unmeasurable::Manifest(_));
    let unsaid = "running run 'v1' does not say what it seats: missing field `capabilities`";
    assert_eq!(
        beside("v1", json!({"schema": "run-manifest/v1"}), manifested),
        (unsaid.into(), false)
    );

    // A run pinned before v13 names its abstract model and no pin: it is
    // never resolved again through the admitting workspace's adapters.
    let unpinned = manifest(json!({"provider": "beta", "model": "b-local"}));
    let unsaid = "running run 'v12' does not say what it seats: missing field `model_pins`";
    assert_eq!(beside("v12", unpinned, manifested), (unsaid.into(), false));
    let unread = json!({"provider": "dsh", "model_pins": {"unreadable": ["-m", "--model"]}});
    let pin = |why: &Unmeasurable| {
        *why == Unmeasurable::Pin {
            site: "work".into(),
            flags: vec!["-m".into(), "--model".into()],
        }
    };
    let unsaid = "running run 'odd' does not say what it seats: site 'work' pins a model that \
                  cannot be read as one concrete id on -m, --model";
    assert_eq!(beside("odd", manifest(unread), pin), (unsaid.into(), false));

    // An entry whose bundle does not compile is held, naming why, before
    // any running run is read.
    let mut fixture = Fixture::new();
    fixture.seat("one", "nobody");
    fixture.queue();
    let file = file(&fixture);
    let compiled = |why: &Unmeasurable| matches!(why, Unmeasurable::Compile(_));
    let detail = "what it seats cannot be measured: bundle: seat 'review': agent 'nobody' is not \
                  in the library; known agents: both, cloudy, local, one";
    assert_eq!(unmeasured(&fixture, &file, compiled), (detail.into(), true));
}

#[test]
fn a_running_run_that_does_not_fold_refuses_the_measure() {
    let mut fixture = Fixture::new();
    fixture.queue();
    let file = fixture.declare(&ceilings(9, 9, json!({})));
    let manifest = json!({"schema": "run-manifest/v1"});
    fixture.store.create_run("r0", "f", "b", &manifest).unwrap();
    let error = passed(&fixture.store, &file).unwrap_err();
    assert!(matches!(&error, AdmissionError::Fold { run, .. } if run == "r0"));
    assert_eq!(error.to_string(), "folding run 'r0'");
}

#[test]
fn the_writing_pass_judges_the_machine_after_it_latches() {
    let mut fixture = Fixture::new();
    fixture.queue();
    let file = fixture.root.join("host.json");
    let probe = |_: &Path| Ok(PLENTY);
    let host = Host {
        file: &file,
        free: &probe,
    };
    let judged = verdicts(judge_within(&mut fixture.store, &host, BY).unwrap());
    let undeclared = Reason::Capacity(Capacity::Undeclared(file.clone()));
    assert_eq!(judged, vec![vec![undeclared]]);
    assert_eq!(verdicts(pass(&fixture.store).unwrap()), vec![vec![]]);
}

#[test]
fn a_running_run_counts_against_the_route_its_own_compile_pinned_whichever_workspace_admits() {
    // Workspace `pinned` holds the journal, and a run of its bundle seats
    // `b-local`, which its adapters map onto the spark-glm route.
    let mut pinned = Fixture::new();
    pinned.seat("local", "local");
    pinned.started("spark");
    // Workspace `other` shares the journal, and its adapters map the same
    // alias onto no route at all and `b-cloud` onto spark-glm.
    let other = Fixture::new();
    let remapped = json!({"b-local": "b-1", "b-cloud": "spark-glm/b-2", "b-plain": "b-3"});
    let adapter = std::fs::read(pinned.root.join("adapters/beta.json")).unwrap();
    let mut adapter: Value = serde_json::from_slice(&adapter).unwrap();
    adapter["models"] = remapped;
    other.write("adapters/beta.json", adapter);
    other.seat("cloudy", "cloudy");
    enqueue(&mut pinned.store, &other.launch());
    let routes = |ceiling| json!({"spark-glm": route(ceiling, "shared-local")});
    let file = pinned.declare(&ceilings(9, 9, routes(1)));
    let shared = route_full("spark-glm", RouteClass::SharedLocal, 1, 1);
    assert_eq!(pinned.room(&file), shared);
    let file = pinned.declare(&ceilings(9, 9, routes(2)));
    assert_eq!(pinned.room(&file), None);
}

#[test]
fn an_inline_model_seat_is_judged_by_the_route_its_command_pins() {
    let mut fixture = Fixture::new();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/dsh.json");
    std::fs::copy(repo, fixture.root.join("adapters/dsh.json")).unwrap();
    let command = [
        "{brokkr}",
        "driver",
        "dsh",
        "--",
        "--model",
        "spark-glm/GLM-5.3-Flash-EXL3",
    ];
    let mut command = command.to_vec();
    command.extend(["--effort", "high"]);
    let inline = json!({"results": ["clean"], "role": "roles/work.md",
                        "driver": {"command": command}});
    fixture.bundle(inline, "one");
    fixture.started("spark");
    fixture.queue();
    let dsh = |routes: Value| {
        host(json!({"alpha": {"ceiling": 9}, "dsh": {"ceiling": 9, "routes": routes}}))
    };
    let shared = |ceiling| dsh(json!({"spark-glm": route(ceiling, "shared-local")}));
    let full = routed_full("dsh", "spark-glm", RouteClass::SharedLocal, 1, 1);
    assert_eq!(fixture.room(&fixture.declare(&shared(1))), full);
    assert_eq!(fixture.room(&fixture.declare(&shared(2))), None);
    let undeclared = Capacity::RouteUndeclared {
        provider: "dsh".into(),
        route: "spark-glm".into(),
    };
    assert_eq!(
        fixture.room(&fixture.declare(&dsh(json!({})))),
        Some(undeclared)
    );
}

#[test]
fn an_entry_that_seats_no_model_is_measured_without_adapters() {
    let mut fixture = Fixture::new();
    let exec = |result: &str| {
        json!({"results": [result], "role": "roles/work.md",
               "driver": {"command": ["{brokkr}", "driver", "exec", "--", "true"]}})
    };
    let seats = json!({"work": exec("complete"), "review": exec("clean")});
    let bundle = json!({"name": "fixture", "policy": "policy.json", "seats": seats});
    fixture.write("bundle/bundle.json", bundle);
    std::fs::remove_dir_all(fixture.root.join("adapters")).unwrap();
    fixture.queue();
    let file = fixture.declare(&host(json!({"exec": {"ceiling": 1}})));
    assert_eq!(fixture.room(&file), None);

    // A bundle that seats a model does not compile with no adapters, and
    // is held naming why.
    fixture.seat("one", "one");
    let compiled = |why: &Unmeasurable| matches!(why, Unmeasurable::Compile(_));
    let detail = format!(
        "what it seats cannot be measured: bundle: adapters {}: No such file or directory (os \
         error 2); the adapter data is where a driver's model mapping (decision 0016) and its \
         trust tier and binding grant (decision 0021) are declared, and this bundle names an \
         agent, seats a gate, declares a secret binding or declares typed tools",
        fixture.root.join("adapters").display()
    );
    assert_eq!(unmeasured(&fixture, &file, compiled), (detail, true));
}

#[test]
fn every_capacity_reason_has_its_word() {
    let path = PathBuf::from("/h");
    let kind = io::ErrorKind::NotFound;
    let pin = Unmeasurable::Pin {
        site: "s".into(),
        flags: vec![],
    };
    let reasons = [
        Capacity::Undeclared(path.clone()),
        Capacity::Unreadable {
            path: path.clone(),
            kind,
        },
        Capacity::Unmeasured(pin.clone()),
        Capacity::RunUnmeasured {
            run: "r".into(),
            why: pin,
        },
        Capacity::ProviderUndeclared("p".into()),
        Capacity::RouteUndeclared {
            provider: "p".into(),
            route: "r".into(),
        },
        full("p", 1, 1).unwrap(),
        route_full("r", RouteClass::Cloud, 1, 1).unwrap(),
        Capacity::ScratchUnmeasured {
            path: path.clone(),
            kind,
        },
        Capacity::ScratchLow {
            path,
            free: 0,
            floor: 1,
        },
        Capacity::BoxedFull {
            running: 1,
            ceiling: 1,
        },
    ];
    let words = reasons.map(|reason| {
        let reason = Reason::Capacity(reason);
        (reason.kind(), reason.holds())
    });
    assert_eq!(
        words,
        [
            ("host_undeclared", false),
            ("host_unreadable", true),
            ("unmeasured", true),
            ("run_unmeasured", false),
            ("provider_undeclared", false),
            ("route_undeclared", false),
            ("provider_full", false),
            ("route_full", false),
            ("scratch_unmeasured", false),
            ("scratch_low", false),
            ("boxed_full", false),
        ]
    );
}
