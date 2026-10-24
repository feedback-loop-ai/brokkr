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
use brokkr_store::{
    Attribution, EntryId, NewEntry, QueueCommand, QueueRefusal, StoreError, WaitOn,
};
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
        crate::places::places_tests::plant_library(&root);
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
            shelf: None,
            journal: self.root.join("forge.db"),
            repo: None,
            secrets: None,
            host_path: OsString::new(),
        };
        let run = NewRun {
            entry: None,
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

    /// A running run whose compile pinned each `(provider, model id)`, on
    /// the id's own route, boxed or not, written as the compile writes it.
    fn seating(&mut self, name: &str, seats: &[(&str, &str)], boxed: bool) {
        let candidates: Vec<Value> = (seats.iter())
            .map(|(provider, id)| {
                let route = crate::capabilities::manifest::route(id).map(str::to_string);
                let served = crate::capabilities::manifest::Served {
                    id: id.to_string(),
                    route,
                };
                json!({"provider": provider, "model_pins": {"read": [served]}})
            })
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

    /// Each entry's reasons under `pass_within`, every filesystem
    /// reporting `free` bytes.
    fn reasons(&self, file: &Path, free: u64) -> Vec<Vec<Reason>> {
        let probe = |_: &Path| Ok(free);
        let host = Host {
            file,
            temp: TEMP.as_ref(),
            free: &probe,
        };
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

/// The temp directory every test host injects; never the test process's.
const TEMP: &str = "/brokkr-test-temp";

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
    let host = Host {
        file,
        temp: TEMP.as_ref(),
        free: &probe,
    };
    pass_within(store, &host)
}

fn verdicts(judged: Vec<Judged>) -> Vec<Vec<Reason>> {
    judged
        .into_iter()
        .filter_map(|judged| Some(judged.verdict?.reasons))
        .collect()
}

/// A host configuration declaring `providers`, a seat-scratch floor of a
/// kibibyte and two boxed builds.
fn host(providers: Value) -> Value {
    json!({"schema": "forge.host/v1", "providers": providers,
           "floors": [{"place": "seat-scratch", "floor_bytes": 1024}],
           "boxed_builds": {"ceiling": 2}})
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

    // A FIFO with no writer, or a link to one, is refused by its kind and
    // never waited on; so is a device, which a read may never finish.
    std::fs::remove_dir(&file).unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(&file)
        .status()
        .unwrap();
    assert!(made.success(), "mkfifo {}", file.display());
    let unsupported = unreadable(io::ErrorKind::Unsupported);
    assert_eq!(fixture.reasons(&file, PLENTY), unsupported);
    std::fs::rename(&file, fixture.root.join("pipe")).unwrap();
    std::os::unix::fs::symlink("pipe", &file).unwrap();
    assert_eq!(fixture.reasons(&file, PLENTY), unsupported);
    std::fs::remove_file(&file).unwrap();
    std::os::unix::fs::symlink("/dev/null", &file).unwrap();
    assert_eq!(fixture.reasons(&file, PLENTY), unsupported);

    // A directory on the path that is a dangling link leaves the file
    // unreachable, not absent.
    std::os::unix::fs::symlink("unmounted", fixture.root.join("stowed")).unwrap();
    let file = fixture.root.join("stowed/host.json");
    let unreached = Reason::Capacity(Capacity::Unreadable {
        path: file.clone(),
        kind: io::ErrorKind::NotFound,
    });
    assert_eq!(fixture.reasons(&file, PLENTY), vec![vec![unreached]]);

    // A file on the path, where a directory belongs, is no absence either:
    // the path cannot be reached, so the file cannot be read.
    std::fs::write(fixture.root.join("plain"), "").unwrap();
    let file = fixture.root.join("plain/host.json");
    let through_a_file = Reason::Capacity(Capacity::Unreadable {
        path: file.clone(),
        kind: io::ErrorKind::NotADirectory,
    });
    assert_eq!(fixture.reasons(&file, PLENTY), vec![vec![through_a_file]]);
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
    let mut unfloored = valid.clone();
    unfloored.as_object_mut().unwrap().remove("floors");
    // Two floors naming one place or one path, each with its own number.
    let twice = |at: Value| {
        let floor = |bytes: u64| {
            let mut floor = at.clone();
            floor["floor_bytes"] = json!(bytes);
            floor
        };
        with("/floors", json!([floor(1), floor(2)]))
    };
    let cases = [
        (unknown, "unknown field `cooldowns`, expected one of `schema`, `providers`, `floors`, `boxed_builds` at line 1 column 41"),
        (keyed, "unknown field `burst`, expected `ceiling` or `routes` at line 1 column 114"),
        (with("/providers/alpha/ceiling", json!(0)), "invalid value: integer `0`, expected a nonzero u32 at line 1 column 118"),
        (with("/providers/alpha/ceiling", json!(-1)), "invalid value: integer `-1`, expected a nonzero u32 at line 1 column 119"),
        (with("/providers/alpha/ceiling", json!(4_294_967_296_u64)), "invalid value: integer `4294967296`, expected a nonzero u32 at line 1 column 127"),
        (with("/boxed_builds/ceiling", json!(0)), "invalid value: integer `0`, expected a nonzero u32 at line 1 column 28"),
        (unfloored, "missing field `floors` at line 1 column 124"),
        (with("/floors", json!([])), "no floor is declared at line 1 column 41"),
        (with("/floors/0/floor_bytes", json!(0)), "invalid value: integer `0`, expected a nonzero u64 at line 1 column 56"),
        (with("/floors/0", json!({"floor_bytes": 1, "path": "tmp"})), "the floor path tmp is not absolute at line 1 column 70"),
        (with("/floors/0", json!({"floor_bytes": 1, "path": null})), "invalid type: null, expected path string at line 1 column 68"),
        (with("/floors/0", json!({"floor_bytes": 1, "path": "/t", "place": "temp"})), "a floor names both a path and a place at line 1 column 85"),
        (with("/floors/0", json!({"floor_bytes": 1})), "a floor names neither a path nor a place at line 1 column 58"),
        (with("/floors/0", json!({"floor_bytes": 1, "place": "disk"})), "unknown variant `disk`, expected `seat-scratch` or `temp` at line 1 column 71"),
        (with("/floors/0", json!({"floor_bytes": 1, "place": "temp", "size": 1})), "unknown field `size`, expected one of `floor_bytes`, `path`, `place` at line 1 column 78"),
        (twice(json!({"place": "temp"})), "the floor at place 'temp' is written twice at line 1 column 106"),
        (twice(json!({"path": "/var/tmp"})), "the floor at path '/var/tmp' is written twice at line 1 column 112"),
        (with("/schema", json!("forge.host/v2")), "unknown variant `forge.host/v2`, expected `forge.host/v1` at line 1 column 178"),
        (with("/providers/beta/routes", json!({"spark-glm": route(1, "local")})), "unknown variant `local`, expected `cloud` or `shared-local` at line 1 column 190"),
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
fn a_run_that_is_not_running_counts_against_no_ceiling() {
    let mut fixture = Fixture::new();
    fixture.seat("one", "cloudy");
    fixture.queue();
    let file = fixture.declare(&ceilings(1, 3, json!({"cloud": route(3, "cloud")})));
    // Parked, completed and stopped, each seated alpha's one lane: none of
    // them is running now, so none takes alpha's one place.
    let ends = [
        ("r1", EventType::RunParked, json!({"reason": "lost"})),
        ("r2", EventType::RunCompleted, json!({})),
        ("r3", EventType::RunStopped, json!({})),
    ];
    for (name, end, payload) in ends {
        fixture.seating(name, &[("alpha", "a-1")], false);
        (fixture.store)
            .append_next(name, end, payload, None, None)
            .unwrap();
    }
    assert_eq!(fixture.room(&file), None);
    // The control: one run that is running fills it.
    fixture.seating("r4", &[("alpha", "a-1")], false);
    assert_eq!(fixture.room(&file), full("alpha", 1, 1));
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

/// `declared` with `floors` in place of its own.
fn floored(declared: &Value, floors: Value) -> Value {
    let mut declared = declared.clone();
    declared["floors"] = floors;
    declared
}

/// The one entry's reasons under the host configuration `declared`, the
/// probe answering each path from `free`, and each path it was asked, in
/// order.
fn measured(
    fixture: &Fixture,
    declared: &Value,
    free: impl Fn(&Path) -> io::Result<u64>,
) -> (Vec<Reason>, Vec<PathBuf>) {
    let file = fixture.declare(declared);
    let asked = RefCell::new(Vec::new());
    let probe = |path: &Path| {
        asked.borrow_mut().push(path.to_path_buf());
        free(path)
    };
    let host = Host {
        file: &file,
        temp: TEMP.as_ref(),
        free: &probe,
    };
    let reasons = verdicts(pass_within(&fixture.store, &host).unwrap()).remove(0);
    (reasons, asked.into_inner())
}

#[test]
fn each_floor_is_measured_where_it_names_in_the_files_order() {
    let mut fixture = Fixture::new();
    fixture.queue();
    let floors = json!([{"place": "seat-scratch", "floor_bytes": 1},
                        {"place": "temp", "floor_bytes": 1},
                        {"path": "/var/brokkr-floor", "floor_bytes": 1}]);
    let declared = floored(&ceilings(1, 1, json!({})), floors);
    let (reasons, asked) = measured(&fixture, &declared, |_| Ok(PLENTY));
    assert_eq!(reasons, vec![]);
    // The seat scratch is the entry's own operated repository's, the temp
    // directory the injected one, and a path is measured as written.
    let paths: [PathBuf; 3] = [fixture.scratch(), TEMP.into(), "/var/brokkr-floor".into()];
    assert_eq!(asked, paths);
}

#[test]
fn a_floor_waits_below_its_free_bytes_naming_it_and_admits_at_or_above() {
    let mut fixture = Fixture::new();
    fixture.queue();
    let declared = ceilings(1, 1, json!({}));
    let low = |at: Measured| {
        vec![Reason::Capacity(Capacity::FloorLow {
            at,
            free: 1023,
            floor: 1024,
        })]
    };
    let scratch = Measured::Place {
        place: Place::SeatScratch,
        path: fixture.scratch(),
    };
    assert_eq!(measured(&fixture, &declared, |_| Ok(1024)).0, vec![]);
    assert_eq!(
        measured(&fixture, &declared, |_| Ok(1023)).0,
        low(scratch.clone())
    );
    let temp = floored(&declared, json!([{"place": "temp", "floor_bytes": 1024}]));
    let temped = Measured::Place {
        place: Place::Temp,
        path: TEMP.into(),
    };
    assert_eq!(
        measured(&fixture, &temp, |_| Ok(1023)).0,
        low(temped.clone())
    );
    let path = floored(
        &declared,
        json!([{"path": "/var/tmp", "floor_bytes": 1024}]),
    );
    let pathed = Measured::Path("/var/tmp".into());
    assert_eq!(
        measured(&fixture, &path, |_| Ok(1023)).0,
        low(pathed.clone())
    );
    let says = |at: Measured| low(at).remove(0).to_string();
    assert_eq!(
        [says(scratch), says(temped), says(pathed)],
        [
            format!(
                "the seat-scratch filesystem at {} has 1023 bytes free, below its floor of 1024",
                fixture.scratch().display()
            ),
            "the temp filesystem at /brokkr-test-temp has 1023 bytes free, below its floor of \
             1024"
                .to_string(),
            "the filesystem at /var/tmp has 1023 bytes free, below its floor of 1024".to_string(),
        ]
    );
}

#[test]
fn of_two_floors_on_one_filesystem_the_second_is_judged_by_its_own_number() {
    let mut fixture = Fixture::new();
    fixture.queue();
    let floors = json!([{"place": "seat-scratch", "floor_bytes": 1024},
                        {"path": "/var/tmp", "floor_bytes": 2048}]);
    let declared = floored(&ceilings(1, 1, json!({})), floors);
    // One filesystem answers both: the first floor holds, the second not.
    let (reasons, asked) = measured(&fixture, &declared, |_| Ok(1500));
    let at = Measured::Path("/var/tmp".into());
    let low = Capacity::FloorLow {
        at,
        free: 1500,
        floor: 2048,
    };
    assert_eq!(reasons, vec![Reason::Capacity(low)]);
    assert_eq!(asked, [fixture.scratch(), "/var/tmp".into()]);
    assert_eq!(measured(&fixture, &declared, |_| Ok(2048)).0, vec![]);
}

#[test]
fn a_floor_whose_filesystem_cannot_be_measured_waits_unmeasured() {
    let mut fixture = Fixture::new();
    fixture.queue();
    let declared = floored(
        &ceilings(1, 1, json!({})),
        json!([{"path": "/var/tmp", "floor_bytes": 1}]),
    );
    let gone = |_: &Path| Err(io::Error::from(io::ErrorKind::NotFound));
    let unmeasured = Capacity::FloorUnmeasured {
        at: Measured::Path("/var/tmp".into()),
        kind: io::ErrorKind::NotFound,
    };
    let (reasons, _) = measured(&fixture, &declared, gone);
    assert_eq!(reasons, vec![Reason::Capacity(unmeasured.clone())]);
    assert_eq!(
        (unmeasured.to_string(), unmeasured.holds()),
        (
            "the filesystem at /var/tmp cannot be measured: entity not found".to_string(),
            false
        )
    );
}

#[test]
fn a_floor_path_through_a_dangling_link_is_unmeasured_and_never_measured_on_its_parent() {
    let mut fixture = Fixture::new();
    fixture.queue();
    let link = fixture.root.join("scratch-link");
    std::os::unix::fs::symlink(fixture.root.join("gone"), &link).unwrap();
    let measured = |path: PathBuf| {
        let floors = json!([{"path": path, "floor_bytes": 1}]);
        let file = fixture.declare(&floored(&ceilings(1, 1, json!({})), floors));
        let host = Host {
            file: &file,
            temp: TEMP.as_ref(),
            free: &host::free_bytes,
        };
        let mut reasons = verdicts(pass_within(&fixture.store, &host).unwrap()).remove(0);
        let reason = reasons.pop().unwrap();
        (reason.to_string(), reason)
    };
    for path in [link.clone(), link.join("scratch")] {
        let unmeasured = Capacity::FloorUnmeasured {
            at: Measured::Path(path.clone()),
            kind: io::ErrorKind::NotFound,
        };
        let says = format!(
            "the filesystem at {} cannot be measured: entity not found",
            path.display()
        );
        assert_eq!(measured(path), (says, Reason::Capacity(unmeasured)));
    }
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
    // An agent candidate no adapter mapped is unreadable on no flag.
    let unmapped = json!({"provider": "dsh", "model_pins": {"unreadable": []}});
    let flagless =
        |why: &Unmeasurable| matches!(why, Unmeasurable::Pin { flags, .. } if flags.is_empty());
    let unsaid = "running run 'unmapped' does not say what it seats: site 'work' pins a model \
                  that cannot be read as one concrete id";
    let unmapped = beside("unmapped", manifest(unmapped), flagless);
    assert_eq!(unmapped, (unsaid.into(), false));
    // A route written `null` is refused, as v13 refuses it, never read as none.
    let null = json!({"provider": "dsh", "model_pins": {"read": [{"id": "glm", "route": null}]}});
    let unsaid = "running run 'null' does not say what it seats: invalid type: null, expected a \
                  string";
    assert_eq!(
        beside("null", manifest(null), manifested),
        (unsaid.into(), false)
    );

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
        temp: TEMP.as_ref(),
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
fn a_bare_and_a_qualified_dsh_id_are_both_counted_against_the_route_dsh_serves_them_on() {
    // dsh serves a bare id on its default route, deepseek-official, so a
    // running run seating `flash` (`deepseek-flash`) fills that route for
    // an entry seating it qualified, and for one seating it bare.
    let mut fixture = Fixture::new();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/dsh.json");
    let mut dsh: Value = serde_json::from_slice(&std::fs::read(repo).unwrap()).unwrap();
    dsh["models"]["official"] = json!("deepseek-official/deepseek-v4-pro");
    fixture.write("adapters/dsh.json", dsh);
    for (agent, model) in [("bare", "flash"), ("qualified", "official")] {
        let body = json!({"description": agent, "charter": "charters/work.md",
                          "models": [model], "efforts": {model: "high"}});
        fixture.write(&format!("agents/{agent}.json"), body);
    }
    fixture.seat("bare", "bare");
    fixture.started("flash");
    fixture.seat("qualified", "qualified");
    fixture.queue();
    let dsh = |routes: Value| host(json!({"dsh": {"ceiling": 9, "routes": routes}}));
    let official = |ceiling| dsh(json!({"deepseek-official": route(ceiling, "cloud")}));
    let full = routed_full("dsh", "deepseek-official", RouteClass::Cloud, 1, 1);
    assert_eq!(fixture.room(&fixture.declare(&official(1))), full);
    assert_eq!(
        full.unwrap().to_string(),
        "cloud route deepseek-official of provider dsh is full: 1 running against a ceiling of 1"
    );
    assert_eq!(fixture.room(&fixture.declare(&official(2))), None);
    fixture.seat("bare", "bare");
    assert_eq!(fixture.room(&fixture.declare(&official(2))), None);
    let full = routed_full("dsh", "deepseek-official", RouteClass::Cloud, 1, 1);
    assert_eq!(fixture.room(&fixture.declare(&official(1))), full);

    // The route omitted from the host file waits as undeclared.
    let undeclared = Capacity::RouteUndeclared {
        provider: "dsh".into(),
        route: "deepseek-official".into(),
    };
    assert_eq!(
        fixture.room(&fixture.declare(&dsh(json!({})))),
        Some(undeclared.clone())
    );
    assert_eq!(
        undeclared.to_string(),
        "the host configuration declares no ceiling for route deepseek-official of provider dsh"
    );
}

#[test]
fn a_running_lane_on_a_provider_the_host_does_not_declare_is_counted_as_cloud() {
    let config: HostConfig = serde_json::from_value(ceilings(1, 1, json!({}))).unwrap();
    let lane = Lane {
        provider: "gamma".into(),
        route: Some("spark-glm".into()),
    };
    assert_eq!(class(&config, &lane), RouteClass::Cloud);
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
        Capacity::FloorUnmeasured {
            at: Measured::Path(path.clone()),
            kind,
        },
        Capacity::FloorLow {
            at: Measured::Path(path),
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
            ("floor_unmeasured", false),
            ("floor_low", false),
            ("boxed_full", false),
        ]
    );
}

/// The room a pass over a plenty-free host reading `file` found for the
/// first entry it listed, which it admitted.
fn admitted(fixture: &Fixture, file: &Path) -> Room {
    let judged = passed(&fixture.store, file).unwrap().remove(0);
    judged.room.expect("admitted")
}

/// A fixture with one entry queued under ceilings of 3, the host
/// configuration declaring them, and the room a pass found for it.
fn one_admitted() -> (Fixture, EntryId, PathBuf, Room) {
    let mut fixture = Fixture::new();
    let entry = fixture.queue();
    let file = fixture.declare(&ceilings(3, 3, json!({})));
    let room = admitted(&fixture, &file);
    (fixture, entry, file, room)
}

/// Claim the entry `room` was measured for, as generation 1.
fn claimed(fixture: &mut Fixture, room: &Room) -> Result<Claim, Reason> {
    room.claim(&mut fixture.store, Generation(1), "admitted")
        .unwrap()
}

/// An operator's `retry` of a run, as `brokkr operator` journals it.
fn retry() -> Value {
    json!({"command_id": "c1", "command": "retry", "args": {}, "operator": "vy"})
}

/// Append each of `events` to `run`.
fn appended(fixture: &mut Fixture, run: &str, events: Vec<(EventType, Value)>) {
    for (event, payload) in events {
        (fixture.store)
            .append_next(run, event, payload, None, None)
            .unwrap();
    }
}

/// Under a ceiling of 1, a parked run retried and a run started between
/// the pass and the claim each take the place the pass found, and the
/// claim is refused for it with nothing written (decision 0095 ruling 7).
#[test]
fn a_run_retried_or_started_after_the_pass_refuses_the_claim_on_capacity() {
    for retried in [true, false] {
        let mut fixture = Fixture::new();
        let entry = fixture.queue();
        let held = fixture.queue();
        (fixture.store)
            .queue_command(held, QueueCommand::Hold, BY)
            .unwrap();
        let file = fixture.declare(&ceilings(1, 1, json!({})));
        fixture.seating("r1", &[("alpha", "a-1")], false);
        // Parked where its seat's attempts ran out, so a retry returns it.
        let entered = (EventType::PhaseEntered, json!({"phase": "work"}));
        let requested = json!({"effect_id": "e", "seat": "work"});
        let requested = (EventType::EffectRequested, requested);
        let parked = (
            EventType::RunParked,
            json!({"reason": "attempts exhausted"}),
        );
        appended(&mut fixture, "r1", vec![entered, requested, parked]);
        let judged = passed(&fixture.store, &file).unwrap();
        let rooms: Vec<bool> = judged.iter().map(|judged| judged.room.is_some()).collect();
        assert_eq!(rooms, [true, false]);
        let room = admitted(&fixture, &file);
        let compiled = fixture.launch().compiled().unwrap();
        assert_eq!(room.digest, compiled.manifest_digest());
        match retried {
            true => {
                let accepted = json!({"command_id": "c1"});
                let events = vec![
                    (EventType::OperatorCommanded, retry()),
                    (EventType::OperatorAccepted, accepted),
                ];
                appended(&mut fixture, "r1", events);
            }
            false => fixture.started("r2"),
        }
        let before = (fixture.store.queue_list()).unwrap();
        let refused = claimed(&mut fixture, &room);
        assert_eq!(refused, Err(Reason::Capacity(full("alpha", 1, 1).unwrap())));
        assert_eq!(fixture.store.queue_list().unwrap(), before);
        let state = fixture.store.queue_entry(entry).unwrap().state;
        assert_eq!(state, brokkr_store::EntryState::Queued);
    }
}

/// A run running at the pass that moved and seats what it seated is
/// counted once, a run that ended since frees its place, and a run
/// started since takes one: under a ceiling of 3, with two running at the
/// pass and two after it, the claim lands.
#[test]
fn the_claim_counts_each_run_that_moved_once_as_it_stands_now() {
    let mut fixture = Fixture::new();
    let entry = fixture.queue();
    let file = fixture.declare(&ceilings(3, 3, json!({})));
    fixture.seating("ended", &[("alpha", "a-1")], false);
    fixture.seating("live", &[("alpha", "a-1")], false);
    let room = admitted(&fixture, &file);
    let completed = (EventType::RunCompleted, json!({}));
    appended(&mut fixture, "ended", vec![completed]);
    // Its writer carries `live`'s fold, not this one's.
    brokkr_store::test_support::lags(&mut fixture.store, &["ended"]);
    let commanded = (EventType::OperatorCommanded, retry());
    appended(&mut fixture, "live", vec![commanded]);
    fixture.seating("new", &[("alpha", "a-1")], false);
    claimed(&mut fixture, &room).unwrap();
    let state = fixture.store.queue_entry(entry).unwrap().state;
    assert_eq!(state, brokkr_store::EntryState::Launching);
}

/// A run that moved since the pass is read as the census reads one: one
/// whose manifest does not say what it seats refuses the claim on
/// capacity, and one whose fold fails refuses it with the fold's error.
#[test]
fn a_run_that_moved_and_cannot_be_read_refuses_the_claim() {
    let mut fixture = Fixture::new();
    fixture.queue();
    let file = fixture.declare(&ceilings(3, 3, json!({})));
    fixture.seating("r1", &[("alpha", "a-1")], false);
    let room = admitted(&fixture, &file);
    fixture.running("unpinned", &json!({"schema": "run-manifest/v1"}));
    let refused = claimed(&mut fixture, &room).unwrap_err();
    assert!(
        matches!(&refused, Reason::Capacity(Capacity::RunUnmeasured {
            run, why: Unmeasurable::Manifest(_) }) if run == "unpinned"),
        "{refused:?}"
    );
    let completed = || (EventType::RunCompleted, json!({}));
    appended(&mut fixture, "r1", vec![completed(), completed()]);
    // A run that does not fold stands behind its head on purpose.
    brokkr_store::test_support::lags(&mut fixture.store, &["r1"]);
    let error = (room.claim(&mut fixture.store, Generation(1), "admitted")).unwrap_err();
    assert!(
        matches!(&error, AdmissionError::Fold { run, .. } if run == "r1"),
        "{error:?}"
    );
    assert_eq!(error.to_string(), "folding run 'r1'");
}

/// A hold, a drop or a re-pin of the entry a room was measured for,
/// between the pass and the claim, refuses the claim by the store's
/// variant with nothing written: the room claims its entry as the pass
/// read it, never as it reads now (decision 0095 ruling 7).
#[test]
fn a_room_claims_its_entry_only_as_the_pass_read_it() {
    type Between = (fn(&mut Fixture, EntryId), QueueRefusal, &'static str);
    let between: [Between; 3] = [
        (
            |fixture, entry| {
                let store = &mut fixture.store;
                store.queue_command(entry, QueueCommand::Hold, BY).unwrap();
            },
            QueueRefusal::Held(EntryId(1)),
            "queue entry 1 is held; release it first",
        ),
        (
            |fixture, entry| {
                let store = &mut fixture.store;
                store.queue_command(entry, QueueCommand::Drop, BY).unwrap();
            },
            QueueRefusal::Dropped(EntryId(1)),
            "queue entry 1 was dropped",
        ),
        (
            |fixture, entry| {
                let payload = fixture.launch().encode().unwrap();
                let seen = fixture.store.queue_entry(entry).unwrap().seen();
                (fixture.store.queue_repin(entry, &payload, seen, BY)).unwrap();
            },
            QueueRefusal::Unseen(EntryId(1)),
            "queue entry 1 was re-pinned or latched after the pass that admitted it read it, \
             and it was not claimed",
        ),
    ];
    for (act, refusal, text) in between {
        let (mut fixture, entry, _, room) = one_admitted();
        act(&mut fixture, entry);
        let before = fixture.store.queue_entry(entry).unwrap();
        let error = (room.claim(&mut fixture.store, Generation(1), "admitted")).unwrap_err();
        let AdmissionError::Store(StoreError::Queue(refused)) = &error else {
            panic!("not a queue refusal: {error:?}");
        };
        assert_eq!((refused, error.to_string()), (&refusal, text.to_string()));
        assert_eq!(fixture.store.queue_entry(entry).unwrap(), before);
    }
}

/// Another journal, whose entry 1 stands as `one_admitted`'s does, and
/// for which its own pass finds no room: its one place under ceilings of
/// 1 is taken.
fn full_elsewhere() -> Fixture {
    let mut other = Fixture::new();
    other.queue();
    let full_file = other.declare(&ceilings(1, 1, json!({})));
    other.seating("r1", &[("alpha", "a-1")], false);
    assert_eq!(other.room(&full_file), full("alpha", 1, 1));
    other
}

/// Assert `error` refuses the claim through the store at `path` as
/// another journal than the one its pass measured, by its text.
fn refused_elsewhere(error: &AdmissionError, path: &Path) {
    let AdmissionError::OtherJournal(named) = error else {
        panic!("not another journal: {error:?}");
    };
    assert_eq!(named, path);
    let text = format!(
        "the journal at {} is not shown to be the one the pass that found this room measured, \
         and nothing was claimed",
        path.display()
    );
    assert_eq!(error.to_string(), text);
}

/// What a claim writes, as the journal at `store`'s path holds it: its
/// entries, and its count of queue commands and of claim rows.
fn written(store: &Store) -> (Vec<QueueEntry>, i64, i64) {
    let read = rusqlite::Connection::open(store.path()).unwrap();
    let query = "SELECT (SELECT COUNT(*) FROM queue_commands), (SELECT COUNT(*) FROM queue_claims)";
    let counts = |row: &rusqlite::Row| Ok((row.get(0)?, row.get(1)?));
    let (commands, claims) = read.query_row(query, [], counts).unwrap();
    (store.queue_list().unwrap(), commands, claims)
}

/// A room claims in no journal but the one its pass measured, by the id
/// it names itself by: another journal whose entry 1 stands as the room's
/// did, its own pass having found no room for it, is refused by
/// `OtherJournal` with nothing written in either journal. A byte copy of
/// the measured journal carries its id, so it is the same journal, as
/// ruled (#430, 2026-10-24), and the room claims its entry in the copy.
#[test]
fn a_room_claims_in_no_journal_but_the_one_its_pass_measured() {
    let (measured, entry, _, room) = one_admitted();
    let mut other = full_elsewhere();
    let copy = measured.root.join("copy.db");
    for suffix in ["", "-wal"] {
        let from = format!("{}{suffix}", measured.store.path().display());
        std::fs::copy(from, format!("{}{suffix}", copy.display())).unwrap();
    }
    let mut copied = Store::open(&copy).unwrap();
    let before = [written(&measured.store), written(&other.store)];
    let error = (room.claim(&mut other.store, Generation(1), "admitted")).unwrap_err();
    refused_elsewhere(&error, other.store.path());
    assert_eq!([written(&measured.store), written(&other.store)], before);

    (room.claim(&mut copied, Generation(1), "admitted"))
        .unwrap()
        .unwrap();
    let state = copied.queue_entry(entry).unwrap().state;
    assert_eq!(state, brokkr_store::EntryState::Launching);
    assert_eq!(written(&measured.store), before[0]);
}

/// A new store on the journal the pass measured, by its path or through a
/// symlink to it, claims the room's entry in that journal.
#[test]
fn a_room_claims_through_any_store_on_the_journal_its_pass_measured() {
    for linked in [false, true] {
        let (fixture, entry, _, room) = one_admitted();
        let path = fixture.store.path().to_path_buf();
        let path = match linked {
            false => path,
            true => {
                let link = fixture.root.join("link.db");
                std::os::unix::fs::symlink(&path, &link).unwrap();
                link
            }
        };
        let mut reconnected = Store::open(&path).unwrap();
        (room.claim(&mut reconnected, Generation(1), "admitted"))
            .unwrap()
            .unwrap();
        let state = fixture.store.queue_entry(entry).unwrap().state;
        assert_eq!(state, brokkr_store::EntryState::Launching);
    }
}

/// A store opened through a symlink is the journal its connection holds,
/// however the link is pointed since. With `current.db` naming journal B,
/// whose own pass finds no room for its entry 1, when a store opens
/// through it, and journal A after: the room A's pass found claims
/// through that store in neither journal, and the room a pass over that
/// store found, handed A's own store, claims in neither; each is refused
/// by `OtherJournal` naming the store it was handed, with nothing written
/// in either journal.
#[test]
fn a_store_opened_through_a_symlink_retargeted_since_is_the_journal_it_holds() {
    for before_the_pass in [false, true] {
        let (mut measured, _, file, mut room) = one_admitted();
        let other = full_elsewhere();
        let alias = measured.root.join("current.db");
        std::os::unix::fs::symlink(other.store.path(), &alias).unwrap();
        let mut through = Store::open(&alias).unwrap();
        std::fs::remove_file(&alias).unwrap();
        std::os::unix::fs::symlink(measured.store.path(), &alias).unwrap();
        if before_the_pass {
            room = (passed(&through, &file).unwrap().remove(0).room).expect("admitted");
        }
        let before = [written(&measured.store), written(&other.store)];
        let (handed, named) = match before_the_pass {
            false => (&mut through, alias),
            true => {
                let named = measured.store.path().to_path_buf();
                (&mut measured.store, named)
            }
        };
        let error = (room.claim(handed, Generation(1), "admitted")).unwrap_err();
        refused_elsewhere(&error, &named);
        let after = [written(&measured.store), written(&other.store)];
        assert_eq!(after, before);
    }
}

/// A store is the journal it opened, not the one its name holds now: with
/// the file A's pass measured renamed onto the path of journal B while a
/// store holds B, the room is refused through that store by
/// `OtherJournal`, and neither journal's entries move.
#[test]
fn a_store_whose_place_another_file_was_renamed_into_is_the_journal_it_opened() {
    let (measured, _, _, room) = one_admitted();
    let mut other = Fixture::new();
    other.queue();
    let before = [&measured.store, &other.store].map(|store| store.queue_list().unwrap());
    std::fs::rename(measured.store.path(), other.store.path()).unwrap();
    let error = (room.claim(&mut other.store, Generation(1), "admitted")).unwrap_err();
    refused_elsewhere(&error, other.store.path());
    let after = [&measured.store, &other.store].map(|store| store.queue_list().unwrap());
    assert_eq!(after, before);
}

/// A store is the journal its connection holds, however long its open
/// waited: journal B, from before journals named themselves, with entries
/// 1 and 2 queued, is opened while a peer holds its write lock, so the
/// open waits to name it, and A's file, whose pass found a room for its
/// entry 1, is renamed onto B's path before the lock is let go. The room
/// is refused through that store by `OtherJournal`, and neither journal's
/// entries, commands or claims move.
#[test]
fn a_store_whose_path_was_replaced_while_its_open_waited_is_the_journal_it_holds() {
    let (measured, _, _, room) = one_admitted();
    let mut other = Fixture::new();
    other.queue();
    other.queue();
    let path = other.store.path().to_path_buf();
    let mut peer = rusqlite::Connection::open(&path).unwrap();
    brokkr_store::test_support::unname(&peer).unwrap();
    let before = [written(&measured.store), written(&other.store)];
    // B keeps a name while A's file stands in its place.
    let kept = other.root.join("kept.db");
    std::fs::hard_link(&path, &kept).unwrap();
    let lock = (peer.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)).unwrap();
    let opening = std::thread::spawn({
        let path = path.clone();
        move || Store::open(&path).unwrap()
    });
    std::thread::sleep(std::time::Duration::from_millis(300));
    std::fs::rename(measured.store.path(), &path).unwrap();
    lock.rollback().unwrap();
    let mut waited = opening.join().unwrap();
    assert_eq!(waited.queue_list().unwrap(), before[1].0, "not B's store");

    let error = (room.claim(&mut waited, Generation(1), "admitted")).unwrap_err();
    refused_elsewhere(&error, &path);
    std::fs::rename(&path, measured.store.path()).unwrap();
    std::fs::rename(&kept, &path).unwrap();
    assert_eq!([written(&measured.store), written(&other.store)], before);
}

/// A journal with no id, read only by an open that may not write, claims
/// in nothing: the room a pass over that open found is refused through
/// it, and through a store that may write, which names the journal as it
/// opens, by `OtherJournal`, with nothing written.
#[test]
fn a_room_whose_pass_read_a_journal_with_no_id_claims_in_nothing() {
    let (measured, _, file, _) = one_admitted();
    let path = measured.store.path().to_path_buf();
    brokkr_store::test_support::unname(&rusqlite::Connection::open(&path).unwrap()).unwrap();
    let mut read = Store::open_read_only(&path).unwrap();
    assert_eq!(read.journal().unwrap(), None);
    let room = (passed(&read, &file).unwrap().remove(0).room).expect("admitted");
    let before = written(&measured.store);
    let mut named = Store::open(&path).unwrap();
    assert_ne!(named.journal().unwrap(), None);
    for store in [&mut read, &mut named] {
        let error = (room.claim(store, Generation(1), "admitted")).unwrap_err();
        refused_elsewhere(&error, &path);
    }
    assert_eq!(written(&measured.store), before);
}

/// `room` as though it had been measured for `entry`: the shape a room
/// handed to the wrong entry would take, which no pass builds.
fn bound_to(room: &Room, entry: QueueEntry) -> Room {
    Room {
        journal: room.journal,
        entry,
        high_water: room.high_water,
        digest: room.digest.clone(),
        lanes: room.lanes.clone(),
        boxed: room.boxed,
        config: room.config.clone(),
        census: Arc::clone(&room.census),
    }
}

/// The claim judges the entry's waits again inside its transaction: a
/// room for an entry whose wait does not hold, because its awaited entry
/// has not started or its run has not ended, is refused by that wait with
/// nothing written; once the wait holds, the pass's own room claims it.
#[test]
fn a_claim_is_refused_while_a_wait_of_its_entry_does_not_hold() {
    let mut fixture = Fixture::new();
    let awaited = fixture.queue();
    let wait = Wait {
        entry: awaited,
        on: WaitOn::Completed,
    };
    let payload = fixture.launch().encode().unwrap();
    let new = NewEntry {
        payload: &payload,
        priority: 0,
        waits: &[wait],
    };
    let waiting = fixture.store.queue_add(new, BY).unwrap();
    let file = fixture.declare(&ceilings(3, 3, json!({})));
    let room = admitted(&fixture, &file);
    let unmet = bound_to(&room, fixture.store.queue_entry(waiting).unwrap());
    let refused = |fixture: &mut Fixture, reason: Reason, text: &str| {
        let before = fixture.store.queue_list().unwrap();
        let refused = claimed(fixture, &unmet).unwrap_err();
        assert_eq!((refused.to_string(), &refused), (text.to_string(), &reason));
        assert_eq!(fixture.store.queue_list().unwrap(), before);
    };
    let unstarted = "waits on 1:completed: entry 1 has not started a run";
    refused(&mut fixture, Reason::NotStarted(wait), unstarted);

    let claim = claimed(&mut fixture, &room).unwrap();
    fixture.started("awaited");
    (fixture.store)
        .queue_link(claim, Generation(1), "awaited", "started")
        .unwrap();
    let run = "awaited".to_string();
    let unended = "waits on 1:completed: its run 'awaited' has not ended";
    refused(&mut fixture, Reason::Unended { wait, run }, unended);

    let completed = (EventType::RunCompleted, json!({}));
    appended(&mut fixture, "awaited", vec![completed]);
    let met = admitted(&fixture, &file);
    claimed(&mut fixture, &met).unwrap();
    let state = fixture.store.queue_entry(waiting).unwrap().state;
    assert_eq!(state, brokkr_store::EntryState::Launching);
}
