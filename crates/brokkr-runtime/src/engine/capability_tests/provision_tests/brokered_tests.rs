//! U7d2 (MB1, MB4, SI2): a site's brokered server set is sealed from what
//! its attempt's provisioning wrote. A compiled boxed inline Claude seat,
//! whose one outcome holds `library-docs` through an installed server, is
//! composed before its durable start, provisioned on a protected HOME and
//! then marked, as dispatch orders a single seat; its hands segment is
//! expanded beside the provisioned broker, and the driver serves exactly the
//! whole set an independent literal names, cold and as the cold
//! replacement and rejoin of an offered session. Dispatch's own calls are
//! driven too, on a host a test stands in for its provisioning
//! ([`stood_in`]).

use brokkr_protocol::hands::SEALED_REACH;
use brokkr_protocol::native_controls::{SealedBroker, SealedServers, SERVING_INPUTS};

use super::*;
use crate::bundle::StepBody;
use crate::capabilities::CapabilityContext;

mod writers_tests;

thread_local! {
    /// The host `Engine::provision` reads on this thread, where a test
    /// stood one in ([`StoodIn`]).
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
    /// How many boxes this thread's stood-in provisioning surveyed.
    static SURVEYED: Cell<usize> = const { Cell::new(0) };
}

/// The host dispatch's own provisioning reads on this thread, with the
/// survey's stand-in ([`observing`]), each box it surveys counted, where a
/// test stood one in; none otherwise, and the real host is read.
pub(in crate::engine) fn stood_in() -> Option<(Host, Surveyor<'static>)> {
    let host = HOST.with_borrow(Clone::clone)?;
    Some((host, &counted))
}

/// [`observing`], counted.
fn counted(
    bootstrap: &Path,
    request: &Survey,
    deadline: Instant,
) -> Result<Provisioned, Unprovisioned> {
    SURVEYED.set(SURVEYED.get() + 1);
    observing(bootstrap, request, deadline)
}

/// A host stood in for this thread's provisioning until dropped, on return
/// or unwind.
struct StoodIn;

impl StoodIn {
    fn new(host: Host) -> StoodIn {
        HOST.set(Some(host));
        StoodIn
    }
}

impl Drop for StoodIn {
    fn drop(&mut self) {
        HOST.set(None);
    }
}

/// The inline Claude seat's authored command.
const COMMAND: [&str; 8] = [
    "{brokkr}",
    "driver",
    "claude",
    "--",
    "--model",
    "claude-opus-5-5",
    "--effort",
    "high",
];

/// A compiled run of one boxed inline Claude work seat under a protected
/// HOME, an installed server outside it: the provisioning fixture's, over
/// another bundle.
struct Claude {
    fixture: Fixture,
}

/// `body` written as JSON to `path` under `root`.
fn write(root: &Path, path: &str, body: &Value) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body.to_string()).unwrap();
}

/// One boxed inline Claude site.
fn site() -> Value {
    json!({"role": "roles/role.md", "hands": {"kind": "workspace", "network": false, "binds": []},
           "driver": {"command": COMMAND}})
}

/// The single seat: one [`site`].
fn single() -> Value {
    let mut seat = site();
    seat["results"] = json!(["complete"]);
    seat
}

impl Claude {
    /// The run of `work` seated as `seat`, the outcome of each site of
    /// `brokered` holding `library-docs`, or none where this host protects
    /// no HOME, the test `test` skipped.
    fn new(test: &str, seat: Value, brokered: &[&str]) -> Option<Claude> {
        Fixture::protected(test).map(|fixture| Claude::of(fixture, seat, brokered))
    }

    /// [`Claude::new`] on `fixture`.
    fn of(mut fixture: Fixture, seat: Value, brokered: &[&str]) -> Claude {
        let root = fixture.root.clone();
        let rule = |(id, result): (usize, &Value)| {
            json!({"id": format!("W{id}"), "from": "work", "result": result, "next": "review",
                   "reason": "r"})
        };
        let results = seat["results"].as_array().unwrap().iter().enumerate();
        let mut rules: Vec<Value> = results.map(rule).collect();
        rules.push(
            json!({"id": "R", "from": "review", "result": "clean", "next": "done",
                          "reason": "r"}),
        );
        write(
            &root,
            "solo/policy.json",
            &json!({"phases": ["work", "review", "done"],
            "initial": "work", "terminal": ["done"], "rules": rules}),
        );
        std::fs::create_dir_all(root.join("solo/roles")).unwrap();
        std::fs::write(root.join("solo/roles/role.md"), "# role\n").unwrap();
        write(
            &root,
            "solo/bundle.json",
            &json!({"name": "solo", "policy": "policy.json",
            "seats": {"work": seat, "review": {"results": ["clean"], "role": "roles/role.md",
                "driver": {"command": ["driver"]}}}}),
        );
        std::fs::create_dir_all(root.join("agents")).unwrap();
        let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters");
        let mut bundle = Bundle::compile_with_capabilities(
            &root.join("solo"),
            &root.join("agents"),
            &shipped,
            Some("private"),
            None,
            brokkr_core::realms::Boundary::Namespace,
            &CapabilityContext::no_grants("private", &root),
        )
        .unwrap();
        let docs = admitted("library-docs", &[&fixture.server()], true);
        for label in brokered {
            let site = bundle.sites.get_mut(*label).unwrap();
            let outcome = &mut site.capabilities.as_mut().unwrap().outcomes[0];
            outcome.held.insert("library-docs".into(), docs.clone());
        }
        measured_rejoin(&mut bundle);
        fixture.engine.bundle = bundle;
        Claude { fixture }
    }

    /// The attempt `attempt` of `selection`'s sites provisioned on the
    /// fixture's host, which is left holding the brokers it sealed.
    fn provisioned(&mut self, attempt: &str, selection: &mut Selection) {
        let host = Ok(self.fixture.host());
        let (ids, mut bound) = (["effect", attempt, "work"], Duration::from_secs(60));
        let timed = (Instant::now(), &mut bound);
        let engine = &mut self.fixture.engine;
        let launched = engine.provision_with(ids, selection, timed, host, &observing);
        assert!(launched.unwrap(), "{attempt} provisions");
    }

    /// The single seat's spawn for the attempt `attempt`, composed as
    /// dispatch composes it before the durable start.
    fn composed(&mut self, attempt: &str) -> SiteSpawn {
        let engine = &mut self.fixture.engine;
        let hands = engine.runtime_hands("work");
        // Its reach sealed as its admission seals it (U8a2c).
        let home = self.fixture.root.join("home");
        (hands.iter()).for_each(|spec| engine.sealed_for(&home, spec));
        let seat = &engine.bundle.seats["work"].body;
        let SeatBody::Invocation(Invocation::Single { command, .. }) = seat else {
            panic!("the seat is single");
        };
        let (command, link) = (command.clone(), None);
        engine.compose_at(
            Some("work"),
            attempt,
            false,
            command,
            hands.as_ref(),
            link,
            "",
        )
    }

    /// `spawn` marked at the single seat with `brokers`; the input the door
    /// is handed.
    fn marked(&self, spawn: &mut SiteSpawn, brokers: &[SealedBroker]) -> Value {
        let mut input = json!({"context": {}});
        let engine = &self.fixture.engine;
        let hands = engine.runtime_hands("work");
        engine.mark_serving(
            "work",
            None,
            (brokers, hands.as_ref()),
            Some(spawn),
            &mut input,
        );
        input
    }

    /// The attempt `attempt` of the single seat as dispatch makes it:
    /// composed before the durable start, provisioned on the fixture's
    /// host, then marked with the brokers its provisioning sealed. The argv
    /// composed, the spawn and the input the door is handed.
    fn dispatched(&mut self, attempt: &str) -> (Vec<String>, SiteSpawn, Value) {
        let mut spawn = self.composed(attempt);
        let composed = spawn.argv.clone();
        let mut selection = selected();
        self.provisioned(attempt, &mut selection);
        let input = self.marked(&mut spawn, selection.brokers("work"));
        (composed, spawn, input)
    }

    /// The driver's command for the spawned `argv` at the site `label`,
    /// handed `input` with the site's plan at the engine's workdir, run as
    /// `bin` and offered `session`; the charter the door reads is not this
    /// suite's, so no role is named.
    fn served(
        &self,
        (label, argv): (&str, &[String]),
        mut input: Value,
        (bin, session): (&str, Option<&str>),
    ) -> Result<Vec<String>, String> {
        let facts = &self.fixture.engine.bundle.sites[label];
        let outcome = &facts.capabilities.as_ref().unwrap().outcomes[0];
        input["workdir"] = json!(self.fixture.engine.workdir());
        input["seat"] = json!(label);
        input["native_controls"] = outcome.controls();
        input.as_object_mut().unwrap().remove("role_path");
        let extra = &argv[argv.iter().position(|part| part == "--").unwrap() + 1..];
        brokkr_protocol::adapters::claude_command(bin, extra, session, &input)
    }

    /// `command`, the Claude command the driver served for the sealed
    /// `input`, judged again by the final check as the driver judges it,
    /// but handed `removed` as the names its spawn removed: the checked
    /// argv, or the refusal's cause.
    fn checked(
        &self,
        input: &Value,
        command: Vec<String>,
        removed: &[String],
    ) -> Result<Vec<String>, String> {
        use brokkr_protocol::native_controls::{self as controls, Dialect, Origin, Transport};
        let sealed = SealedServing::decode(Some(&input[SERVING_INPUTS])).unwrap();
        let record = LaunchRecord::decode(Some(&input[LAUNCH_RECORD])).unwrap();
        let managed = controls::managed(input).unwrap().unwrap();
        let authored: Vec<String> = (record.segments.iter())
            .filter(|segment| segment.origin == Origin::Authored)
            .flat_map(|segment| segment.argv.iter().cloned())
            .collect();
        let (brokkr, workdir) = (
            std::env::current_exe().unwrap(),
            self.fixture.engine.workdir(),
        );
        // The root the engine made and wrote beside the sealed inputs (U8a),
        // and the reach it sealed (U8a2c).
        let artifacts = PathBuf::from(input[controls::ARTIFACT_ROOT].as_str().unwrap());
        assert_eq!(artifacts, artifact_root(&self.fixture.engine.run_local()));
        assert!(artifacts.is_dir(), "made before launch");
        let reach = serde_json::from_value(input[SEALED_REACH].clone()).unwrap();
        let SealedServers::Brokered(brokers) = &sealed.isolation.servers else {
            panic!("the seat is brokered");
        };
        let dialect = &sealed.dialect;
        let checked = controls::check_final(
            "claude",
            command,
            &managed,
            &record.expected,
            Dialect {
                permissions: dialect.permissions.as_ref(),
                sandbox: &dialect.sandbox,
                hands: &dialect.hands,
                boundary: &dialect.boundary,
                stands: dialect.stands,
            },
            controls::Serving {
                program: "claude",
                workdir: workdir.to_str().unwrap(),
                authored: &authored,
                pins: &sealed.pins,
                hands: sealed.spec.as_ref().map(|spec| Transport {
                    brokkr: &brokkr,
                    workdir: &workdir,
                    artifacts: &artifacts,
                    sealed: &reach,
                    spec,
                }),
                brokers,
                removed,
                ..controls::Serving::default()
            },
        );
        checked
            .map(controls::Checked::into_argv)
            .map_err(|refusal| refusal.cause)
    }

    /// The broker the attempt `attempt`'s provisioning wrote, read back
    /// from its directory: its one plan's locator and the digest of its
    /// bytes.
    fn written(&self, attempt: &str) -> SealedBroker {
        let plan = self.fixture.run_dir().join(attempt).join("plan-0.json");
        let bytes = std::fs::read(&plan).unwrap();
        SealedBroker {
            capability: "library-docs".into(),
            plan: plan.to_str().unwrap().into(),
            plan_digest: brokkr_core::canonical::sha256_bytes(&bytes),
            removals: vec!["DOCS_TOKEN".into()],
        }
    }
}

/// The Claude command a cold launch of the seat is served, written by hand
/// with the MCP document `document`, whose broker it allows (U9b1d).
fn cold(program: &str, document: &str) -> Vec<String> {
    let brokered = usize::from(document.contains(r#""cap-library-docs""#));
    let allowed = ["mcp__brokkr__workspace", ",mcp__cap-library-docs"][..=brokered].concat();
    strings(&[
        program,
        "-p",
        "--output-format",
        "stream-json",
        "--verbose",
        "--model",
        "claude-opus-5-5",
        "--effort",
        "high",
        "--tools",
        "",
        "--strict-mcp-config",
        "--mcp-config",
        document,
        "--allowedTools",
        &allowed,
        "--disallowedTools",
        "WebFetch,WebSearch",
    ])
}

/// The whole set's document at `workdir`, by hand: the box's hands, with the
/// reach sealed here for them under the fixture's HOME (U8a2c) and the
/// run-local store its boxes hide (U8h), and beside them `broker` on its own
/// plan, both run by this test binary; `None` for the hands alone.
fn document(workdir: &Path, broker: Option<&SealedBroker>) -> String {
    let exe = std::env::current_exe().unwrap();
    let home = workdir.parent().unwrap().join("home");
    let git = brokkr_protocol::hands::git_facts(workdir);
    let store = crate::places::run_local(None, workdir).join(crate::places::SECRETS);
    let (spec, place) = (
        HandsSpec::default(),
        brokkr_protocol::hands::store_place(&store),
    );
    let sealed =
        brokkr_protocol::hands::seal(&spec, &home, workdir, &git, Some((&place.unwrap(), &store)));
    let reach = serde_json::to_string(&serde_json::to_string(&sealed).unwrap()).unwrap();
    let (exe, workdir) = (exe.to_str().unwrap(), workdir.to_str().unwrap());
    let spec = r#"{\"binds\":[],\"kind\":\"workspace\",\"network\":false}"#;
    let hands = format!(
        r#""brokkr":{{"args":["hands","serve","--workdir","{workdir}","--artifacts","{workdir}/.forge/artifacts","--reach",{reach},"--spec","{spec}"],"command":"{exe}"}}"#
    );
    let broker = broker.map_or_else(String::new, |broker| {
        format!(
            r#","cap-library-docs":{{"args":["broker","serve","--plan","{}","--plan-digest","{}"],"command":"{exe}"}}"#,
            broker.plan, broker.plan_digest
        )
    });
    format!(r#"{{"mcpServers":{{{hands}{broker}}}}}"#)
}

/// A Claude CLI in `dir` that reports version 2.1.266, installed by a
/// child and renamed in.
fn shim(dir: &Path) -> String {
    let source = dir.join("claude.source");
    std::fs::write(&source, "#!/bin/sh\nprintf '2.1.266 (Claude Code)\\n'\n").unwrap();
    let installed = super::survey_tests::installed(dir, "claude", &source);
    installed.to_str().unwrap().to_string()
}

/// Each site of `bundle` records its `boxed-workspace` rejoin as its
/// replacement is recorded, as the `supported` assessment [`offered`]
/// stipulates: the shipped Claude adapter has measured its boxed
/// replacement since U1d3 but no MCP record of that rejoin, and a launch
/// whose replacement is measured declines a rejoin whose shape is not.
fn measured_rejoin(bundle: &mut Bundle) {
    use crate::agents::{McpInvocation, McpShape};
    let sites = bundle.sites.values_mut();
    for record in sites.flat_map(|site| site.capabilities.iter_mut().flat_map(|c| &mut c.strict)) {
        let replacement =
            (record.iter()).find(|(shape, _)| shape.invocation == McpInvocation::Replacement);
        let rejoin = replacement.map(|(shape, ambient)| {
            let invocation = McpInvocation::Resume("boxed-workspace".into());
            (
                McpShape {
                    invocation,
                    ..shape.clone()
                },
                ambient.clone(),
            )
        });
        record.extend(rejoin);
    }
}

/// `input` offered `session` under the boxed workspace's markers, its
/// rejoin assessed `supported` for the shim's version where `supported`,
/// and unassessed otherwise.
fn offered(mut input: Value, supported: bool) -> Value {
    input["boundary"] = json!("namespace");
    input["hands"] = json!("boxed");
    input["resume_context"] = json!({"assessment": supported.then(|| json!({"boxed-workspace": {
        "status": "supported",
        "identity": {"version": "2.1.266", "applies_to": "2.1.266"},
        "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed",
        "evidence": {"interface": "i", "restrictions": "r", "root": "o", "accounting": "a"},
        "limitations": [], "reason": null}}))});
    input
}

/// The session the seat is offered.
const SESSION: &str = "019c4b7e-0000-7000-8000-000000000001";

/// MB1, MB4: the seat's serving inputs seal exactly the broker its
/// attempt's provisioning wrote, and the driver serves the hands and that
/// broker as the independent literal names them: cold, as the rejoin of an
/// offered session, and as the cold replacement of one it declines. The
/// dispatch door admits the sealed pair, and the next attempt is served its
/// own plan.
#[test]
fn a_provisioned_seat_is_served_the_whole_set_its_attempt_wrote() {
    let test = "a_provisioned_seat_is_served_the_whole_set_its_attempt_wrote";
    let Some(mut claude) = Claude::new(test, single(), &["work"]) else {
        return;
    };
    let bin = shim(&claude.fixture.root);
    for attempt in ["attempt-1", "attempt-2"] {
        let (_, spawn, input) = claude.dispatched(attempt);
        assert_eq!(spawn.refusal, None);
        assert_eq!(verify_record(&spawn, &input), Ok(()));
        claude.whole_set(("work", &spawn.argv[..]), &input, attempt, &bin);
    }
}

impl Claude {
    /// MB1, MB4 at the site `label` spawned with `argv` and handed `input`
    /// by the attempt `attempt`: its serving inputs seal exactly the broker
    /// that attempt wrote, and the driver serves the hands and that broker
    /// as the independent literal names them, cold, as the rejoin of a
    /// session offered on the shim `bin`, and as the cold replacement of
    /// one it declines.
    fn whole_set(&self, (label, argv): (&str, &[String]), input: &Value, attempt: &str, bin: &str) {
        let broker = self.written(attempt);
        let sealed = SealedServing::decode(Some(&input[SERVING_INPUTS])).unwrap();
        let brokered = SealedServers::Brokered(vec![broker.clone()]);
        assert_eq!(sealed.isolation.servers, brokered, "{label}");
        let whole = document(&self.fixture.engine.workdir(), Some(&broker));
        let site = (label, argv);
        let served = self.served(site, input.clone(), ("claude", None));
        assert_eq!(served, Ok(cold("claude", &whole)), "{label} {attempt}");
        let rejoined = [cold(bin, &whole), strings(&["--resume", SESSION])].concat();
        let rejoin = (offered(input.clone(), true), (bin, Some(SESSION)));
        assert_eq!(self.served(site, rejoin.0, rejoin.1), Ok(rejoined));
        let declined = (offered(input.clone(), false), (bin, Some(SESSION)));
        let replacement = self.served(site, declined.0, declined.1);
        assert_eq!(replacement, Ok(cold(bin, &whole)), "{label} {attempt}");
    }
}

/// A recording driver in `dir`, staged and renamed in: it writes the
/// arguments it was spawned with to `driver.argv`, each NUL-terminated, and
/// the start message it was handed to `driver.json`, and answers as the
/// capturing driver does.
fn recording(dir: &Path) -> String {
    let start = dir.join("driver.json");
    let result = json!({"result": "complete"});
    let answer = capturing_driver_command("effect", "attempt", &start, result);
    let source = dir.join("driver.source");
    let argv = dir.join("driver.argv");
    let script = format!(
        "#!/bin/sh\nprintf '%s\\0' \"$@\" > '{}'\n{}\n",
        argv.display(),
        answer[2]
    );
    std::fs::write(&source, script).unwrap();
    let installed = super::survey_tests::installed(dir, "driver", &source);
    installed.to_str().unwrap().to_string()
}

/// What [`recording`] in `dir` was spawned with and handed: its arguments,
/// and its start message.
#[cfg(target_os = "linux")]
fn recorded(dir: &Path) -> (Vec<String>, Value) {
    let argv = std::fs::read(dir.join("driver.argv")).expect("the driver was spawned");
    let argv = argv.strip_suffix(&[0]).unwrap().split(|byte| *byte == 0);
    let argv = argv.map(|field| String::from_utf8(field.to_vec()).unwrap());
    let start = std::fs::read(dir.join("driver.json")).unwrap();
    (argv.collect(), serde_json::from_slice(&start).unwrap())
}

/// The program `body`'s one inline driver is spawned as: the single seat's,
/// or its sequence's first step's.
fn program(body: &mut SeatBody) -> &mut String {
    let command = match body {
        SeatBody::Invocation(Invocation::Single { command, .. }) => command,
        SeatBody::Sequence { steps } => match &mut steps[0].body {
            StepBody::Invocation(Invocation::Single { command, .. }) => command,
            StepBody::Invocation(Invocation::Panel { .. }) | StepBody::Dialect { .. } => {
                panic!("the step is single")
            }
        },
        SeatBody::Invocation(Invocation::Panel { .. }) | SeatBody::Select { .. } => {
            panic!("the seat is single or a sequence")
        }
    };
    &mut command[0]
}

/// R1 of U7d2's first review (MB1, MB4): dispatch's own calls — `execute`'s
/// for a single seat and `single_step`'s for a sequence step — provision
/// the attempt on a stood-in host after its durable start, and the driver
/// they spawn is handed exactly the whole set that attempt wrote, served
/// cold, rejoined and as the cold replacement as the literal names it.
/// Linux only: off Linux no mount table is read, so the dispatch door
/// refuses every boxed writer beside evidence before anything spawns
/// (decision 0063; U8a2a).
#[cfg(target_os = "linux")]
#[test]
fn dispatch_serves_each_site_the_whole_set_its_attempt_wrote() {
    let test = "dispatch_serves_each_site_the_whole_set_its_attempt_wrote";
    let mut after = after();
    // A sibling step that retains nothing is still a managed writer, which
    // a workspace box keeps from the attempt's evidence (U8a2a, CR2).
    after["hands"] = site()["hands"].clone();
    for (seat, label) in [(single(), "work"), (beside(after), "work:only")] {
        let Some(mut claude) = Claude::new(test, seat, &[label]) else {
            return;
        };
        claude.executed();
        let root = claude.fixture.root.clone();
        let (argv, start) = recorded(&root);
        let attempt = start["attempt_id"].as_str().unwrap();
        let bin = shim(&root);
        claude.whole_set((label, &argv[..]), &start["input"], attempt, &bin);
    }
}

/// The step `after` a sequence's brokered step: inline, with no hands.
fn after() -> Value {
    json!({"name": "after", "role": "roles/role.md", "driver": {"command": ["true"]}})
}

/// A sequence seat of one brokered [`site`], `only`, and then `after`.
fn beside(after: Value) -> Value {
    let mut step = site();
    (step["name"], step["results"]) = (json!("only"), json!(["complete"]));
    json!({"results": ["complete"], "sequence": [step, after]})
}

impl Claude {
    /// The work seat's effect executed as dispatch executes it, on the
    /// fixture's stood-in host, its program the [`recording`] driver.
    fn executed(&mut self) {
        let root = self.fixture.root.clone();
        let engine = &mut self.fixture.engine;
        *program(&mut engine.bundle.seats.get_mut("work").unwrap().body) = recording(&root);
        self.executed_as_is();
    }

    /// The work seat's effect executed as dispatch executes it, on the
    /// fixture's stood-in host.
    fn executed_as_is(&mut self) {
        let _host = StoodIn::new(self.fixture.host());
        let effect = requested(&self.fixture.engine, "effect");
        execute_work(&mut self.fixture.engine, &[effect]).unwrap();
    }
}

/// U8a2a (CR2): a step whose holding retains protects the attempt's
/// evidence from its first writer on, so a zero-grant sibling step outside
/// any box refuses the whole attempt by CR2's cause at admission, before
/// anything is provisioned or any step's driver starts. On any TMPDIR (R3
/// of run four's review): admission opens the root it is handed with no
/// ancestor judged, and refuses before provisioning reads the HOME.
#[test]
fn a_retaining_step_beside_an_unboxed_sibling_starts_nothing() {
    let mut claude = Claude::of(Fixture::inline(), beside(after()), &["work:only"]);
    claude.executed();
    let root = claude.fixture.root.clone();
    let engine = &claude.fixture.engine;
    let events = engine.store.load(&engine.run_id).unwrap();
    let failed = events.last().unwrap();
    let (site, run) = ("work:after".into(), engine.run_id.clone());
    let unboxed = super::super::super::broker::admission::Conflict::Unboxed { site, run };
    let refusal = super::super::super::broker::admission::Unadmitted::Evidence(unboxed);
    assert_eq!(failed.event_type, EventType::EffectFailed);
    assert_eq!(failed.payload["error"], refusal.to_string());
    assert!(
        !root.join("driver.argv").exists(),
        "no step's driver started"
    );
    // Nothing was provisioned: no attempt directory, so no plan to serve.
    let provisioned = std::fs::read_dir(claude.fixture.run_dir()).map(|dir| dir.count());
    assert!(provisioned.is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound));
}

/// R2 of U7d2's first review (MB4): the names a spawn removed are judged
/// at the final check against the brokers' sealed removals, not taken from
/// them. The engine sealed `DOCS_TOKEN` for the attempt's broker, and its
/// served command is checked as served only where the spawn removed exactly
/// that name; one it did not remove, one it removed beside it, or a name
/// removed twice refuses.
#[test]
fn a_removal_the_spawn_did_not_make_refuses_at_the_final_check() {
    let test = "a_removal_the_spawn_did_not_make_refuses_at_the_final_check";
    let Some(mut claude) = Claude::new(test, single(), &["work"]) else {
        return;
    };
    let (_, spawn, input) = claude.dispatched("attempt-1");
    let served = claude.served(("work", &spawn.argv[..]), input.clone(), ("claude", None));
    let command = served.unwrap();
    let sealed = SealedServing::decode(Some(&input[SERVING_INPUTS])).unwrap();
    let SealedServers::Brokered(brokers) = &sealed.isolation.servers else {
        panic!("the seat is brokered");
    };
    let removals: Vec<&[String]> = (brokers.iter())
        .map(|broker| broker.removals.as_slice())
        .collect();
    assert_eq!(removals, [strings(&["DOCS_TOKEN"])]);
    let final_set =
        "provider 'claude' final MCP configuration is not the engine's sealed server set";
    for (removed, expected) in [
        (&["DOCS_TOKEN"][..], Ok(command.clone())),
        (&[], Err(final_set.to_string())),
        (&["OTHER_TOKEN"], Err(final_set.to_string())),
        (&["DOCS_TOKEN", "OTHER_TOKEN"], Err(final_set.to_string())),
        (&["DOCS_TOKEN", "DOCS_TOKEN"], Err(final_set.to_string())),
    ] {
        let checked = claude.checked(&input, command.clone(), &strings(removed));
        assert_eq!(checked, expected, "{removed:?}");
    }
}

/// A broker sealed beside the hands that is not the one provisioned — its
/// plan's locator, its digest or its removals altered after the seal —
/// refuses at the dispatch door; the plan or digest altered past the door
/// refuses at the driver's final check, whose command carries the plan the
/// attempt wrote. Removals altered past the door are the next test's.
#[test]
fn an_altered_sealed_broker_refuses_at_the_final_boundary() {
    let test = "an_altered_sealed_broker_refuses_at_the_final_boundary";
    let Some(mut claude) = Claude::new(test, single(), &["work"]) else {
        return;
    };
    let (_, spawn, input) = claude.dispatched("attempt-1");
    let final_set = "refusing to invoke the agent CLI: provider 'claude' final MCP configuration \
                     is not the engine's sealed server set";
    for (field, value, driver) in [
        ("plan", json!("/elsewhere/plan-0.json"), Some(final_set)),
        ("plan_digest", json!("c".repeat(64)), Some(final_set)),
        ("removals", json!(["OTHER_TOKEN"]), None),
    ] {
        let mut altered = input.clone();
        let broker = &mut altered[SERVING_INPUTS]["isolation"]["servers"]["brokered"][0];
        broker[field] = value;
        let door = verify_record(&spawn, &altered);
        assert_eq!(door, Err(super::super::serving_tests::NOT_SEALED.into()));
        if let Some(refusal) = driver {
            let served = claude.served(("work", &spawn.argv[..]), altered, ("claude", None));
            assert_eq!(served, Err(refusal.to_string()), "{field}");
        }
    }
}

/// #349, ask 8: the hands and the broker beside them are run by the engine
/// executable the bundle carries, which the box's hands were composed with
/// and the whole set is expanded under again — never a binary read anew.
#[test]
fn the_whole_set_is_run_by_the_executable_the_bundle_carries() {
    let test = "the_whole_set_is_run_by_the_executable_the_bundle_carries";
    // An unreadable executable keeps the empty spelling where the engine
    // path stands.
    for (engine, spelling) in [(Some("/opt/brokkr-test"), "/opt/brokkr-test"), (None, "")] {
        let Some(mut claude) = Claude::new(test, single(), &["work"]) else {
            return;
        };
        claude.fixture.engine.bundle.executable = crate::argv::Executable(engine.map(Into::into));
        let (_, spawn, _) = claude.dispatched("attempt-1");
        let broker = claude.written("attempt-1");
        let exe = std::env::current_exe().unwrap();
        let whole = document(&claude.fixture.engine.workdir(), Some(&broker));
        let whole = whole.replace(exe.to_str().unwrap(), spelling);
        let config = spawn.argv.iter().position(|part| part == "--mcp-config");
        let served = config.map(|at| spawn.argv[at + 1].clone());
        assert_eq!((spawn.refusal, served), (None, Some(whole)));
    }
}

/// A seat whose outcome holds no brokered capability seals the hands set
/// alone, provisions nothing, and keeps every byte of the command composed
/// for it: the driver serves the hands-only literal.
#[test]
fn a_seat_with_no_brokered_holding_is_served_its_hands_alone() {
    let test = "a_seat_with_no_brokered_holding_is_served_its_hands_alone";
    let Some(mut claude) = Claude::new(test, single(), &[]) else {
        return;
    };
    let (composed, spawn, input) = claude.dispatched("attempt-1");
    assert_eq!((spawn.refusal.as_deref(), &spawn.argv), (None, &composed));
    let sealed = SealedServing::decode(Some(&input[SERVING_INPUTS])).unwrap();
    assert_eq!(sealed.isolation.servers, SealedServers::Hands);
    assert!(!claude.fixture.run_dir().exists(), "nothing is provisioned");
    let alone = document(&claude.fixture.engine.workdir(), None);
    let served = claude.served(("work", &spawn.argv[..]), input, ("claude", None));
    assert_eq!(served, Ok(cold("claude", &alone)));
}

/// MB1 at each member: a panel's members are provisioned under their own
/// labels and each is sealed and served its own set, the member that
/// brokers `library-docs` beside its hands and the one that brokers nothing
/// its hands alone, never the other's.
#[test]
fn each_panel_member_is_served_the_set_its_own_site_was_provisioned() {
    let test = "each_panel_member_is_served_the_set_its_own_site_was_provisioned";
    let panel = json!({"results": ["pass", "fail"], "aggregate": "unanimous-pass",
                       "panel": {"docs": site(), "plain": site()}});
    let Some(mut claude) = Claude::new(test, panel, &["work:docs"]) else {
        return;
    };
    let current = state(Some("work"), Cursor::RequestEffect);
    let input = (claude.fixture.engine)
        .seat_input(&current, "work", "effect")
        .unwrap();
    let mut selection = Selection::new();
    for (index, member) in ["docs", "plain"].into_iter().enumerate() {
        let instance = index.to_string().repeat(64);
        let plan = planned(&"0".repeat(64), &instance);
        selection.plans.insert(Some(member.into()), plan);
    }
    claude.provisioned("attempt-1", &mut selection);
    let body = claude.fixture.engine.bundle.seats["work"].body.clone();
    let SeatBody::Invocation(Invocation::Panel { members, .. }) = &body else {
        panic!("the seat is a panel");
    };
    let (meta, context) = (&input["members"], &input["context"]);
    // Each member's reach sealed as its admission seals it (U8a2c).
    let home = claude.fixture.root.join("home");
    for label in ["work:docs", "work:plain"] {
        let hands = claude.fixture.engine.hands_for(label).unwrap();
        claude.fixture.engine.sealed_for(&home, &hands);
    }
    let runs = (claude.fixture.engine).member_runs(
        "attempt-1",
        "work",
        members,
        meta,
        &input,
        context,
        &selection,
        "",
        false,
    );
    let observed: Vec<_> = (runs.iter())
        .map(|run| {
            let sealed = SealedServing::decode(Some(&run.input[SERVING_INPUTS])).unwrap();
            let site = (run.driver_seat.as_str(), &run.spawn.argv[..]);
            let served = claude.served(site, run.input.clone(), ("claude", None));
            (run.driver_seat.clone(), sealed.isolation.servers, served)
        })
        .collect();
    let (broker, workdir) = (claude.written("attempt-1"), claude.fixture.engine.workdir());
    let whole = cold("claude", &document(&workdir, Some(&broker)));
    let alone = cold("claude", &document(&workdir, None));
    assert_eq!(
        observed,
        [
            (
                "work:docs".into(),
                SealedServers::Brokered(vec![broker]),
                Ok(whole)
            ),
            ("work:plain".into(), SealedServers::Hands, Ok(alone)),
        ]
    );
}

/// The removal control of `broker::brokers`' UTF-8 reading of each pinned
/// locator: an attempt whose directory is not UTF-8 is refused as a failed
/// write before any broker is sealed from it. Linux only: APFS refuses a
/// file name that is not UTF-8 (EILSEQ) before any engine code runs.
#[cfg(target_os = "linux")]
#[test]
fn an_attempt_directory_that_is_not_utf8_seals_no_broker() {
    let test = "an_attempt_directory_that_is_not_utf8_seals_no_broker";
    let Some(mut fixture) = Fixture::protected(test) else {
        return;
    };
    fixture.hold(
        "library-docs",
        admitted("library-docs", &[&fixture.server()], true),
    );
    let home = fixture.root.join(OsString::from_vec(b"h\xffme".to_vec()));
    std::fs::create_dir(&home).unwrap();
    std::fs::set_permissions(&home, std::fs::Permissions::from_mode(0o755)).unwrap();
    let host = Host {
        home,
        ..fixture.host()
    };
    let made = fixture.provisioned_by("attempt-1", Ok(host), &observing);
    assert_eq!(made, Err(Unprovisioned::Write));
}

/// Nothing is reconciled (U7d2): a serving that brokers `library-docs`
/// marked without its attempt's brokers, as `mark_capabilities` marks it,
/// and one that brokers nothing handed a broker, each refuse their spawn;
/// so does a spawn whose hands segment is not its fragment's hands-only
/// expansion, and a set the builder cannot expand. Each text is pinned here
/// once.
#[test]
fn a_broker_set_that_is_not_the_provisioned_one_refuses_its_spawn() {
    use super::super::super::isolation::{beside, Unsealed};
    let test = "a_broker_set_that_is_not_the_provisioned_one_refuses_its_spawn";
    let (Some(mut held), Some(mut none)) = (
        Claude::new(test, single(), &["work"]),
        Claude::new(test, single(), &[]),
    ) else {
        return;
    };
    let (_, spawn, input) = held.dispatched("attempt-1");
    let broker = held.written("attempt-1");
    let unprovisioned = |held: &str, provisioned: &str| {
        format!(
            "dispatch refused: the site's serving candidate brokers the MCP capabilities \
             [{held}], but this attempt provisioned plans for [{provisioned}] at its site, so \
             no broker set is sealed beside its launch; a broker is sealed only from its own \
             provisioned plan (decision 0065 slice two, U7d2)"
        )
    };
    let mut unmarked = held.composed("attempt-2");
    let mut ignored = json!({"context": {}});
    (held.fixture.engine).mark_capabilities("work", None, Some(&mut unmarked), &mut ignored);
    let mut handed = none.composed("attempt-1");
    none.marked(&mut handed, std::slice::from_ref(&broker));
    let mut altered = held.composed("attempt-3");
    let hands = (altered.segments.iter_mut()).rfind(|segment| segment.origin == Origin::Hands);
    hands.unwrap().argv[4] = "{}".into();
    held.marked(&mut altered, std::slice::from_ref(&broker));
    let unexpanded = Unsealed::Unexpanded.to_string();
    assert_eq!(
        [unmarked.refusal, handed.refusal, altered.refusal],
        [
            Some(unprovisioned("'library-docs'", "")),
            Some(unprovisioned("", "'library-docs'")),
            Some(unexpanded.clone()),
        ]
    );
    assert_eq!(
        unexpanded,
        "dispatch refused: the spawn does not carry its measured hands fragment expanded for \
         the box's hands exactly once, so the brokers its attempt provisioned cannot be \
         expanded beside them; no document is rebuilt around another (decision 0065 slice \
         two, U7d2)"
    );
    // The set the builder cannot name, and serving inputs without hands.
    let sealed = SealedServing::decode(Some(&input[SERVING_INPUTS])).unwrap();
    let workdir = held.fixture.engine.workdir();
    let unbare = SealedBroker {
        capability: "docs.v2".into(),
        ..broker.clone()
    };
    let handless = SealedServing {
        spec: None,
        ..sealed.clone()
    };
    // U8a2c: and a spawn composed with no sealed reach.
    let reach = serde_json::from_value(input[SEALED_REACH].clone()).unwrap();
    for (serving, broker, reach) in [
        (&sealed, &unbare, Some(&reach)),
        (&handless, &broker, Some(&reach)),
        (&sealed, &broker, None),
    ] {
        let mut again = spawn.clone();
        let worktree = (
            &*crate::argv::tests::ENGINE,
            workdir.as_path(),
            Path::new("/a"),
            reach,
        );
        let expanded = beside(&mut again, serving, worktree, std::slice::from_ref(broker));
        assert_eq!(expanded, Err(Unsealed::Unexpanded));
        assert_eq!(again.argv, spawn.argv);
    }
    // The public seal provisions nothing, so the brokering site refuses
    // there too, with the same text.
    let engine = &held.fixture.engine;
    let facts = engine.bundle.sites.get("work");
    let public = crate::engine::serving_inputs(None, facts, spawn.class, engine.boundary);
    assert_eq!(public.err(), Some(unprovisioned("'library-docs'", "")));
}
