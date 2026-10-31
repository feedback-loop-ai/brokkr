//! Decision 0065 slice two, U7c (SD3, MB1, MB3, CR3): each brokered
//! holding of an attempt is sealed into one owner-only plan, pinned by the
//! attempt's one inventory, before anything spawns, from the same
//! selected projection the servers the seat is named are read from. Each
//! box is surveyed by `brokkr broker survey` under the startup deadline
//! ([`survey_tests`]); here a stand-in answers as the survey would.

use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

use brokkr_core::realms::GrantRetention;
use brokkr_protocol::broker::{
    Bootstrap, Clearance, Inventory, Network, Plan, Provisioned, Refusal, Sources, Survey, Tree,
    Unread,
};
use brokkr_protocol::hands::{Bind, BindMode, GitFacts, ServerProgram};

use super::super::broker::{
    left, owned, reach, share, startup, stored, Attempt, Commit, Host, Surveyor, Unprovisioned,
};
use super::super::tests::{execute_work, requested};
use super::call_tests::planned;
use super::*;
use crate::capabilities::{Admitted, Connection, Holding, Implementation, Retention, Strength};

pub(in crate::engine) mod brokered_tests;
mod discovery_tests;
pub(in crate::engine) mod survey_tests;

/// The instance reference the single site resolves to.
const INSTANCE: &str = "1a22000000000000000000000000000000000000000000000000000000000022";

/// The bootstrap a fixture's host names.
const BOOTSTRAP: &str = "/opt/brokkr/bin/brokkr";

/// An admitted MCP holding of `capability` launched by `argv`, its
/// responses retained as `retained` says.
pub(super) fn admitted(capability: &str, argv: &[&str], retained: bool) -> Holding {
    Holding {
        classes: vec!["reads".into()],
        dialect: format!("{capability}-mcp"),
        dialect_sha256: "d".repeat(64),
        definition_sha256: "e".repeat(64),
        tools: strings(&["lookup", "fetch"]),
        restrictions: Map::new(),
        implementation: Implementation::Mcp {
            server: format!("cap-{capability}"),
            connection: Connection::Stdio(strings(argv)),
            version: "1.0.0".into(),
            secrets: strings(&["DOCS_TOKEN"]),
        },
        retention: Retention {
            declared: retained,
            realm: GrantRetention::Inherit,
        },
        admitted: Some(Admitted {
            network: Network::Isolated,
            clearance: Clearance {
                dialect: "d".repeat(64),
                policy: "f".repeat(64),
            },
        }),
    }
}

/// A native holding, which no broker carries.
fn native_holding() -> Holding {
    Holding {
        implementation: Implementation::Native {
            provider: "codex".into(),
            adapter_key: "web-search".into(),
        },
        admitted: None,
        ..admitted("web-search", &[], false)
    }
}

/// The directory a fixture whose HOME is judged with its ancestors (D6)
/// is made in: the first of `/var/tmp` and the temporary directory that,
/// with every ancestor, protects what lies below it by the writer's own
/// rule (`owned`), or none where neither does — as in a seat's box whose
/// `/tmp` is this user's and group-writable.
pub(super) fn base() -> Option<PathBuf> {
    let user = brokkr_protocol::hands::ids().0;
    let judged = |at: &Path| rustix::fs::stat(at).is_ok_and(|stat| owned(&stat, user, false));
    let candidates = [PathBuf::from("/var/tmp"), std::env::temp_dir()];
    let canonical = candidates
        .into_iter()
        .filter_map(|dir| dir.canonicalize().ok());
    canonical
        .into_iter()
        .find(|dir| dir.ancestors().all(judged))
}

/// Where a fixture that provisions nothing is made: the protected base,
/// or the temporary directory where there is none.
pub(super) fn scratch() -> PathBuf {
    base().unwrap_or_else(std::env::temp_dir)
}

/// A fixture root: the engine over `<root>/work`, a HOME at `<root>/home`
/// and an installed server at `<root>/opt/docs/bin/server`, outside both.
pub(super) struct Fixture {
    _dir: tempfile::TempDir,
    pub(super) root: PathBuf,
    pub(super) engine: Engine,
}

impl Fixture {
    /// [`Fixture::inline`] under the protected base, or none where this
    /// host has none, the test `test` then skipped with its reason: what
    /// it proves is written below a HOME the broker would trust.
    pub(super) fn protected(test: &str) -> Option<Fixture> {
        let Some(base) = base() else {
            let required = brokkr_protocol::hands::boundary_evidence_required();
            let reason = format!("{test}: no temporary directory here protects a HOME (D6)");
            brokkr_protocol::hands::skip_boundary_proof(required, &reason);
            return None;
        };
        Some(Fixture::inline_in(&base))
    }

    /// An inline `work` site with boxed hands — one read-write bind under
    /// HOME, one read-only and one overlay bind — whose one outcome holds
    /// nothing yet.
    pub(super) fn inline() -> Fixture {
        Fixture::inline_in(&scratch())
    }

    /// [`Fixture::inline`] made in `base`.
    pub(super) fn inline_in(base: &Path) -> Fixture {
        let dir = tempfile::tempdir_in(base).unwrap();
        let (dir, mut engine) = canonical_engine_in(dir, single_body(vec!["driver".into()]));
        let root = std::fs::canonicalize(dir.path()).unwrap();
        for made in ["home/cache", "ro", "overlay", "opt/docs/bin"] {
            std::fs::create_dir_all(root.join(made)).unwrap();
        }
        let server = root.join("opt/docs/bin/server");
        std::fs::write(&server, "#!/bin/sh\n").unwrap();
        // HOME and its ancestors are this user's and writable by no one
        // else, whatever the umask made them.
        for path in [&server, &root, &root.join("home")] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let bind = |path: String, mode| Bind {
            path,
            mode,
            mask: Vec::new(),
        };
        let spec = HandsSpec {
            network: false,
            binds: vec![
                bind("~/cache".into(), BindMode::Rw),
                bind(root.join("ro").display().to_string(), BindMode::Ro),
                bind(
                    root.join("overlay").display().to_string(),
                    BindMode::Overlay,
                ),
            ],
        };
        let site = engine.bundle.sites.entry("work".into()).or_default();
        inline_site(site, crate::bundle::McpIntent::Hands);
        site.hands = crate::bundle::HandsState::Hands(spec);
        site.capabilities = Some(two_candidates());
        Fixture {
            _dir: dir,
            root,
            engine,
        }
    }

    /// The single site's outcome holds `holding` as `capability`.
    pub(super) fn hold(&mut self, capability: &str, holding: Holding) {
        let site = self.engine.bundle.sites.get_mut("work").unwrap();
        let outcome = &mut site.capabilities.as_mut().unwrap().outcomes[0];
        outcome.held.insert(capability.into(), holding);
    }

    /// The single site asks for `capability` as `strength`.
    pub(super) fn ask(&mut self, capability: &str, strength: Strength) {
        let site = self.engine.bundle.sites.get_mut("work").unwrap();
        let asks = &mut site.capabilities.as_mut().unwrap().asks.asks;
        asks.insert(capability.into(), strength);
    }

    /// The single site's hands.
    fn spec(&mut self) -> &mut HandsSpec {
        let site = self.engine.bundle.sites.get_mut("work").unwrap();
        let crate::bundle::HandsState::Hands(spec) = &mut site.hands else {
            panic!("the fixture's site has hands");
        };
        spec
    }

    /// The single site's hands gain `path` in `mode`.
    fn bind(&mut self, path: &str, mode: BindMode) {
        let mask = Vec::new();
        self.spec().binds.push(Bind {
            path: path.into(),
            mode,
            mask,
        });
    }

    /// The installed server's launch.
    pub(super) fn server(&self) -> String {
        self.root.join("opt/docs/bin/server").display().to_string()
    }

    /// The fixture's host: its HOME, a bootstrap and this user.
    pub(super) fn host(&self) -> Host {
        Host {
            home: self.root.join("home"),
            bootstrap: PathBuf::from(BOOTSTRAP),
            writer: brokkr_protocol::hands::ids().0,
        }
    }

    /// The run's directory under the fixture HOME's protected root.
    pub(super) fn run_dir(&self) -> PathBuf {
        let work = self.root.join("work");
        let repo = brokkr_core::canonical::sha256_bytes(work.as_os_str().as_encoded_bytes());
        let root = self.root.join("home/.local/state/brokkr/capabilities");
        root.join(repo).join(&self.engine.run_id)
    }

    /// The attempt `attempt` of the single site, provisioned on the
    /// fixture's host, each box surveyed by `survey`.
    pub(super) fn provision(
        &self,
        attempt: &str,
        survey: Surveyor<'_>,
    ) -> Result<Option<PathBuf>, Unprovisioned> {
        self.provisioned_by(attempt, Ok(self.host()), survey)
    }

    /// The attempt `attempt` of the single site, provisioned on a host
    /// whose HOME could not be read, where surveying a box fails the test.
    fn unread(&self, attempt: &str) -> Result<Option<PathBuf>, Unprovisioned> {
        let survey = |_: &Path, _: &Survey, _: Instant| panic!("a box was surveyed");
        self.provisioned_by(attempt, Err(Unprovisioned::Root), &survey)
    }

    /// The attempt `attempt` of the single site, provisioned on `host`,
    /// each box surveyed by `survey` within a minute.
    pub(super) fn provisioned_by(
        &self,
        attempt: &str,
        host: Result<Host, Unprovisioned>,
        survey: Surveyor<'_>,
    ) -> Result<Option<PathBuf>, Unprovisioned> {
        let attempt = Attempt {
            effect: "effect",
            attempt,
            site: "work",
        };
        let deadline = Instant::now() + Duration::from_secs(60);
        let (surveyor, commits) = ((survey, deadline), &Cell::new(Commit::Synced));
        let sealed = (self.engine).provisioned(&attempt, &selected(), host, surveyor, commits);
        sealed.map(|made| made.map(|sealed| sealed.control))
    }

    /// Whether the attempt `attempt` of the single site may spawn once
    /// provisioned on `host`, provisioning having begun at `began` under
    /// `bound`, which is left holding what the driver gets; each box
    /// surveyed by `survey`.
    fn launch(
        &mut self,
        attempt: &str,
        (began, bound): (Instant, &mut Duration),
        host: Result<Host, Unprovisioned>,
        survey: Surveyor<'_>,
    ) -> bool {
        let (ids, timed) = (["effect", attempt, "work"], (began, bound));
        let engine = &mut self.engine;
        let launched = engine.provision_with(ids, &mut selected(), timed, host, survey);
        launched.unwrap()
    }
}

/// The single site's selection: its plan's instance, no link.
fn selected() -> Selection {
    let mut selection = Selection::new();
    let plan = planned(&"0".repeat(64), INSTANCE);
    selection.plans.insert(None, plan);
    selection
}

/// The survey's stand-in: the program the request names, resolved as the
/// survey resolves it against the request's HOME, the bootstrap it was
/// started as, and a box of seven entries over three mounts, whatever its
/// deadline.
pub(super) fn observing(
    bootstrap: &Path,
    request: &Survey,
    _: Instant,
) -> Result<Provisioned, Unprovisioned> {
    let program = ServerProgram::resolve(&request.launch, &request.home)?;
    let tree = match program.tree() {
        Tree::System {} => Tree::System {},
        Tree::Package { root } => Tree::Package { root: root.clone() },
    };
    Ok(Provisioned {
        executable: program.executable().to_path_buf(),
        tree,
        bootstrap: Bootstrap {
            path: bootstrap.to_path_buf(),
            digest: "b".repeat(64),
        },
        sources: Sources {
            entries: 7,
            mounts: 3,
            digest: "5".repeat(64),
        },
    })
}

/// The inventory under `control` and the plans it pins, each read back
/// through the protocol's closed types after its pin's digest was checked.
pub(super) fn sealed(control: &Path) -> (Inventory, Vec<Value>) {
    let inventory = std::fs::read(control.join("inventory.json")).unwrap();
    let inventory: Inventory = serde_json::from_slice(&inventory).unwrap();
    let plans = (inventory.plans.iter())
        .map(|pin| {
            let bytes = std::fs::read(&pin.locator).unwrap();
            assert_eq!(brokkr_core::canonical::sha256_bytes(&bytes), pin.digest);
            let plan: Plan = serde_json::from_slice(&bytes).unwrap();
            serde_json::to_value(plan).unwrap()
        })
        .collect();
    (inventory, plans)
}

/// A path's permission bits.
fn mode(path: &Path) -> u32 {
    std::fs::symlink_metadata(path)
        .unwrap()
        .permissions()
        .mode()
        & 0o7777
}

/// Each retaining plan's share is the budget divided by the retaining
/// plans the inventory pins, so simultaneous slots never share one, and
/// more than 32 exhaust it.
#[test]
fn each_retaining_slot_gets_a_disjoint_fixed_share_and_a_33rd_exhausts_the_budget() {
    let mib = 1 << 20;
    assert_eq!(share(0), Ok(None));
    assert_eq!(share(1), Ok(Some(256 * mib)));
    assert_eq!(share(2), Ok(Some(128 * mib)));
    assert_eq!(share(3), Ok(Some(256 * mib / 3)));
    assert_eq!(share(32), Ok(Some(8 * mib)));
    assert_eq!(share(33), Err(Unprovisioned::Budget));
}

/// Every cause provisioning refuses with, in the operator's words, said
/// once for this module.
#[test]
fn each_provisioning_refusal_says_its_own_words() {
    let said = [
        (
            Unprovisioned::Budget,
            "capability evidence budget is exhausted",
        ),
        (
            Unprovisioned::Unsealed("library-docs".into()),
            "capability 'library-docs' carries no admitted MCP seal to provision a plan from",
        ),
        (
            Unprovisioned::Box(Refusal::ProgramTree),
            "MCP server box program tree cannot be resolved",
        ),
        (
            Unprovisioned::Box(Refusal::Linked),
            "MCP server box program tree contains a multiply-linked file",
        ),
        (
            Unprovisioned::Unread(Unread::Malformed),
            "MCP server survey request is not one closed record",
        ),
        (
            Unprovisioned::Expired,
            "MCP server box survey did not answer before its startup deadline",
        ),
        (
            Unprovisioned::Spent,
            "the attempt's bound was spent provisioning its broker plans",
        ),
        (
            Unprovisioned::Unanswered,
            "MCP server box survey gave no closed reply",
        ),
        (
            Unprovisioned::Store,
            "the secret store's path does not resolve",
        ),
        (
            Unprovisioned::Root,
            "capability evidence root is not protected",
        ),
        (
            Unprovisioned::Provisioned,
            "capability evidence for this attempt is already provisioned",
        ),
        (
            Unprovisioned::Write,
            "capability evidence could not be written",
        ),
        (
            Unprovisioned::Bootstrap,
            "the trusted bootstrap cannot be read",
        ),
    ];
    for (cause, words) in said {
        assert_eq!(cause.to_string(), words);
    }
}

/// The plan the fixture's retaining package server is sealed into, for
/// attempt `attempt` and the inventory directory `control`.
fn docs_plan(fixture: &Fixture, attempt: &str, control: &Path) -> Value {
    let root = &fixture.root;
    let server = fixture.server();
    let run = fixture.run_dir();
    let repo = run.parent().unwrap().file_name().unwrap().to_str().unwrap();
    json!({
        "owner": {"repo": repo, "run": fixture.engine.run_id, "effect": "effect",
                  "attempt": attempt, "site": "work", "instance": INSTANCE},
        "server": "cap-library-docs", "capability": "library-docs",
        "dialect": {"name": "library-docs-mcp", "digest": "d".repeat(64),
                    "definition": "e".repeat(64), "version": "1.0.0"},
        "connection": {"argv": [server, "--stdio"]},
        "tools": ["lookup", "fetch"], "restrictions": {}, "retained": true,
        "secrets": ["DOCS_TOKEN"],
        "clearance": {"dialect": "d".repeat(64), "policy": "f".repeat(64)},
        "box": {
            "reach": {"writable": [root.join("work"), root.join("home/cache")],
                      "readable": [root.join("ro"), root.join("overlay")]},
            "executable": server, "tree": {"kind": "package", "root": root.join("opt/docs")},
            "sources": {"entries": 7, "mounts": 3, "digest": "5".repeat(64)},
            "writers": {"uids": [brokkr_protocol::hands::ids().0], "privilege": "confined"},
            "network": "isolated",
            "environment": ["PATH", "HOME", "TMPDIR", "USER", "LOGNAME", "LANG", "LC_ALL",
                            "BROKKR_HANDS_BOX"],
            "bootstrap": {"path": BOOTSTRAP, "digest": "b".repeat(64)},
            "excluded": {"store": root.join("work/.forge/secrets.env"), "control": [control]},
        },
    })
}

/// The site's brokered holdings — a package server that retains and a
/// system entry that does not, beside a native holding no broker carries —
/// are sealed into owner-only plans in an owner-only attempt directory,
/// each plan's facts the selected holding's, the site's own hands and the
/// box the survey answered, and the inventory pins exactly them by locator
/// and digest. Each survey is asked for the launch, the engine's HOME, the
/// site's reach, the sealed network and the engine's uid.
#[test]
fn an_attempts_brokered_holdings_are_sealed_owner_only_and_pinned_by_its_inventory() {
    let test = "an_attempts_brokered_holdings_are_sealed_owner_only_and_pinned_by_its_inventory";
    let Some(mut fixture) = Fixture::protected(test) else {
        return;
    };
    let server = fixture.server();
    fixture.hold(
        "library-docs",
        admitted("library-docs", &[&server, "--stdio"], true),
    );
    fixture.hold("status", admitted("status", &["true"], false));
    fixture.hold("web-search", native_holding());
    let asked: RefCell<Vec<(PathBuf, Survey)>> = RefCell::new(Vec::new());
    let survey = |bootstrap: &Path, request: &Survey, deadline: Instant| {
        let copied = serde_json::from_value(serde_json::to_value(request).unwrap()).unwrap();
        asked.borrow_mut().push((bootstrap.to_path_buf(), copied));
        observing(bootstrap, request, deadline)
    };
    let run = fixture.run_dir();
    let control = run.join("attempt-1");
    let provisioned = fixture.provision("attempt-1", &survey);
    assert_eq!(provisioned, Ok(Some(control.clone())));
    let capabilities = run.parent().unwrap().parent().unwrap();
    for private in [capabilities, run.parent().unwrap(), &run, &control] {
        assert_eq!(mode(private), 0o700, "{}", private.display());
    }
    let (inventory, plans) = sealed(&control);
    let locators: Vec<&Path> = (inventory.plans.iter())
        .map(|pin| pin.locator.as_path())
        .collect();
    assert_eq!(
        locators,
        [control.join("plan-0.json"), control.join("plan-1.json")]
    );
    for file in ["plan-0.json", "plan-1.json", "inventory.json"] {
        assert_eq!(mode(&control.join(file)), 0o600, "{file}");
    }
    let docs = docs_plan(&fixture, "attempt-1", &control);
    let mut status = docs.clone();
    status["server"] = json!("cap-status");
    status["capability"] = json!("status");
    status["dialect"]["name"] = json!("status-mcp");
    status["connection"] = json!({"argv": ["true"]});
    status["retained"] = json!(false);
    let system = ServerProgram::resolve("true", &fixture.root).unwrap();
    status["box"]["executable"] = json!(system.executable());
    status["box"]["tree"] = json!({"kind": "system"});
    assert_eq!(plans, [docs.clone(), status]);
    let request = |launch: &str| Survey {
        launch: launch.into(),
        home: fixture.root.join("home"),
        reach: serde_json::from_value(docs["box"]["reach"].clone()).unwrap(),
        network: Network::Isolated,
        writers: vec![brokkr_protocol::hands::ids().0],
    };
    let bootstrap = PathBuf::from(BOOTSTRAP);
    assert_eq!(
        asked.into_inner(),
        [
            (bootstrap.clone(), request(&server)),
            (bootstrap, request("true"))
        ]
    );
}

/// The first plan of attempt `attempt` with the fixture holding `holding`
/// as `library-docs` on `host`, surveyed by `survey`, its attempt and
/// control root, which every attempt has its own of, set aside.
fn first_plan(
    fixture: &mut Fixture,
    attempt: &str,
    holding: Holding,
    (host, survey): (Host, Surveyor<'_>),
) -> Value {
    fixture.hold("library-docs", holding);
    let control = fixture.provisioned_by(attempt, Ok(host), survey);
    let mut plan = sealed(&control.unwrap().unwrap()).1.remove(0);
    plan["owner"]["attempt"] = Value::Null;
    plan["box"]["excluded"]["control"] = Value::Null;
    plan
}

/// The fields, and box fields, in which two plans differ.
fn differing(one: &Value, other: &Value) -> Vec<String> {
    let fields = |plan: &Value, under: &str| -> Vec<(String, Value)> {
        let object = plan.as_object().unwrap().iter();
        let named = object.map(|(key, value)| (format!("{under}{key}"), value.clone()));
        named.collect()
    };
    let both = |plan: &Value| [fields(plan, ""), fields(&plan["box"], "box.")].concat();
    let pairs = both(one).into_iter().zip(both(other));
    let moved = pairs.filter(|((key, one), (_, other))| key != "box" && one != other);
    moved.map(|((key, _), _)| key).collect()
}

/// The plan field one change to a holding moves, and the change.
type Moving = (&'static str, fn(&mut Holding));

/// SC1: every fact a plan seals follows its own input and nothing else.
/// Each of the holding's sealed facts, the host's writer, the survey's
/// bootstrap and sources and the seat's hands, moved alone, moves exactly
/// its own field of the plan.
#[test]
fn each_sealed_fact_moves_exactly_its_own_field_of_the_plan() {
    let test = "each_sealed_fact_moves_exactly_its_own_field_of_the_plan";
    let Some(mut fixture) = Fixture::protected(test) else {
        return;
    };
    let server = fixture.server();
    let docs = || admitted("library-docs", &[&server, "--stdio"], true);
    let host = (fixture.host(), &observing as Surveyor<'_>);
    let base = first_plan(&mut fixture, "a-base", docs(), host);
    let mut moved = Vec::new();
    let held: [Moving; 10] = [
        ("box.network", |h| {
            h.admitted.as_mut().unwrap().network = Network::Shared
        }),
        ("clearance", |h| {
            h.admitted.as_mut().unwrap().clearance.policy = "a".repeat(64)
        }),
        ("retained", |h| h.retention.realm = GrantRetention::Veto),
        ("tools", |h| h.tools.push("other".into())),
        ("dialect", |h| h.dialect_sha256 = "c".repeat(64)),
        ("dialect", |h| h.definition_sha256 = "c".repeat(64)),
        ("dialect", |h| h.dialect = "other-mcp".into()),
        ("secrets", |h| {
            if let Implementation::Mcp { secrets, .. } = &mut h.implementation {
                secrets.push("OTHER_TOKEN".into());
            }
        }),
        ("dialect", |h| {
            if let Implementation::Mcp { version, .. } = &mut h.implementation {
                *version = "2.0.0".into();
            }
        }),
        ("connection", |h| {
            if let Implementation::Mcp {
                connection: Connection::Stdio(argv),
                ..
            } = &mut h.implementation
            {
                argv.push("--verbose".into());
            }
        }),
    ];
    for (index, (field, change)) in held.into_iter().enumerate() {
        let mut holding = docs();
        change(&mut holding);
        let host = (fixture.host(), &observing as Surveyor<'_>);
        let plan = first_plan(&mut fixture, &format!("a-held-{index}"), holding, host);
        moved.push((vec![field.to_string()], differing(&base, &plan)));
    }
    let system = admitted("library-docs", &["true"], true);
    let host = (fixture.host(), &observing as Surveyor<'_>);
    let plan = first_plan(&mut fixture, "a-system", system, host);
    let launch = ["connection", "box.executable", "box.tree"].map(String::from);
    moved.push((launch.to_vec(), differing(&base, &plan)));
    let rehashed = |bootstrap: &Path, request: &Survey, deadline: Instant| {
        let mut provisioned = observing(bootstrap, request, deadline)?;
        provisioned.bootstrap.digest = "a".repeat(64);
        Ok(provisioned)
    };
    let host = (fixture.host(), &rehashed as Surveyor<'_>);
    let plan = first_plan(&mut fixture, "a-bootstrap", docs(), host);
    moved.push((vec!["box.bootstrap".into()], differing(&base, &plan)));
    let mut writer = fixture.host();
    writer.writer += 1;
    let plan = first_plan(&mut fixture, "a-writer", docs(), (writer, &observing));
    moved.push((vec!["box.writers".into()], differing(&base, &plan)));
    let drifted = |bootstrap: &Path, request: &Survey, deadline: Instant| {
        let mut provisioned = observing(bootstrap, request, deadline)?;
        provisioned.sources.digest = "6".repeat(64);
        Ok(provisioned)
    };
    let host = (fixture.host(), &drifted as Surveyor<'_>);
    let plan = first_plan(&mut fixture, "a-sources", docs(), host);
    moved.push((vec!["box.sources".into()], differing(&base, &plan)));
    fixture.bind("/usr/share", BindMode::Ro);
    let host = (fixture.host(), &observing as Surveyor<'_>);
    let plan = first_plan(&mut fixture, "a-reach", docs(), host);
    moved.push((vec!["box.reach".into()], differing(&base, &plan)));
    // One assertion over every move, so a removal shows each fact it
    // unbinds.
    let (expected, observed): (Vec<_>, Vec<_>) = moved.into_iter().unzip();
    assert_eq!(observed, expected);
}

/// Nothing is reconciled, every holding wanted, and nothing is made: the
/// budget, and a holding without its admitted seal or a stdio launch,
/// refuse before the host is consulted, its HOME unread then refusing
/// nothing in their place; a program MB3 cannot resolve and a box
/// the survey refuses each refuse with their own cause; a protected root
/// that is not refuses too.
#[test]
fn every_refusal_comes_before_anything_is_made_and_spares_no_wanted_holding() {
    let mut fixture = Fixture::inline();
    let server = fixture.server();
    let docs = || admitted("library-docs", &[&server], true);
    for index in 0..33 {
        let held = format!("docs-{index}");
        fixture.hold(&held, admitted(&held, &[&server], true));
        fixture.ask(&held, Strength::Wants);
    }
    assert_eq!(fixture.unread("a-budget"), Err(Unprovisioned::Budget));
    let test = "every_refusal_comes_before_anything_is_made_and_spares_no_wanted_holding";
    let Some(fixture) = &mut Fixture::protected(test) else {
        return;
    };
    fixture.ask("library-docs", Strength::Wants);
    let mut unadmitted = docs();
    unadmitted.admitted = None;
    let mut url = docs();
    url.implementation = Implementation::Mcp {
        server: "cap-library-docs".into(),
        connection: Connection::Url("https://docs.example/mcp".into()),
        version: "1.0.0".into(),
        secrets: Vec::new(),
    };
    let unsealed = Unprovisioned::Unsealed("library-docs".into());
    for (index, holding) in [unadmitted, url].into_iter().enumerate() {
        fixture.hold("library-docs", holding);
        assert_eq!(fixture.unread(&format!("a-{index}")), Err(unsealed.clone()));
    }
    let refusing = |_: &Path, _: &Survey, _: Instant| Err(Unprovisioned::Box(Refusal::Linked));
    let cases: [(Holding, Surveyor<'_>, Refusal); 2] = [
        (
            admitted("library-docs", &["docs/server"], true),
            &observing,
            Refusal::ProgramTree,
        ),
        (docs(), &refusing, Refusal::Linked),
    ];
    for (index, (holding, survey, refusal)) in cases.into_iter().enumerate() {
        fixture.hold("library-docs", holding);
        let refused = fixture.provision(&format!("a-box-{index}"), survey);
        assert_eq!(refused, Err(Unprovisioned::Box(refusal)));
    }
    let made = fixture.root.join("home/.local");
    assert!(
        !made.exists(),
        "nothing was made before every plan was sealed"
    );
    // The protected root is a symlink to an owner-only directory, then a
    // directory others may read, each below protected ancestors.
    fixture.hold("library-docs", docs());
    let capabilities = fixture.root.join("home/.local/state/brokkr/capabilities");
    let elsewhere = fixture.root.join("elsewhere");
    std::fs::create_dir_all(capabilities.parent().unwrap()).unwrap();
    std::fs::create_dir(&elsewhere).unwrap();
    let owner_only = std::fs::Permissions::from_mode(0o700);
    for dir in [&elsewhere, &capabilities.parent().unwrap().to_path_buf()] {
        std::fs::set_permissions(dir, owner_only.clone()).unwrap();
    }
    for dir in ["home/.local", "home/.local/state"] {
        std::fs::set_permissions(fixture.root.join(dir), owner_only.clone()).unwrap();
    }
    std::os::unix::fs::symlink(&elsewhere, &capabilities).unwrap();
    let symlinked = fixture.provision("a-symlinked", &observing);
    assert_eq!(symlinked, Err(Unprovisioned::Root));
    std::fs::remove_file(&capabilities).unwrap();
    std::fs::create_dir(&capabilities).unwrap();
    std::fs::set_permissions(&capabilities, std::fs::Permissions::from_mode(0o750)).unwrap();
    let shared = fixture.provision("a-shared", &observing);
    assert_eq!(shared, Err(Unprovisioned::Root));
    assert_eq!(std::fs::read_dir(&capabilities).unwrap().count(), 0);
}

/// Only an attempt directory that already stands is provisioned: a run
/// directory the attempt's cannot be made in is a write that failed.
#[test]
fn an_attempt_directory_that_cannot_be_made_is_a_failed_write() {
    let test = "an_attempt_directory_that_cannot_be_made_is_a_failed_write";
    let Some(mut fixture) = Fixture::protected(test) else {
        return;
    };
    fixture.hold("status", admitted("status", &["true"], false));
    assert!(fixture
        .provision("attempt-1", &observing)
        .is_ok_and(|made| made.is_some()));
    let run = fixture.run_dir();
    std::fs::set_permissions(&run, std::fs::Permissions::from_mode(0o500)).unwrap();
    let refused = fixture.provision("attempt-2", &observing);
    std::fs::set_permissions(&run, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(refused, Err(Unprovisioned::Write));
    assert_eq!(
        fixture.provision("attempt-1", &observing),
        Err(Unprovisioned::Provisioned)
    );
}

/// A run id that is not one plain name refuses as an unprotected root
/// does, and nothing is made beside the repo's directory or in it: the run
/// is reached as one name, never walked through.
#[test]
fn a_run_id_that_is_not_one_plain_name_reaches_no_directory() {
    let test = "a_run_id_that_is_not_one_plain_name_reaches_no_directory";
    let Some(mut fixture) = Fixture::protected(test) else {
        return;
    };
    fixture.hold("status", admitted("status", &["true"], false));
    let repo = fixture.run_dir().parent().unwrap().to_path_buf();
    for run in ["../escaped", "a/b", ".", ".."] {
        fixture.engine.run_id = run.into();
        let refused = fixture.provision("attempt-1", &observing);
        assert_eq!(refused, Err(Unprovisioned::Root), "{run}");
    }
    let capabilities = std::fs::read_dir(repo.parent().unwrap()).unwrap();
    let made: Vec<PathBuf> = capabilities.map(|entry| entry.unwrap().path()).collect();
    assert_eq!(made, std::slice::from_ref(&repo));
    assert_eq!(std::fs::read_dir(&repo).unwrap().count(), 0);
}

/// The environment of one entry, `HOME` as `home`, beside one that is
/// not UTF-8.
fn homed(home: impl Into<OsString>) -> Vec<(OsString, OsString)> {
    let foreign = (OsString::from("OTHER"), OsString::from_vec(vec![0xff]));
    vec![(OsString::from("HOME"), home.into()), foreign]
}

/// The host is read from the values given, never this process's: HOME as
/// the broker reads it, from the environment's UTF-8 entries, absolute and
/// canonical, the bootstrap canonical; a missing, relative or non-UTF-8
/// HOME is no protected root, and a binary that does not read no
/// bootstrap.
#[test]
fn the_host_is_the_home_and_binary_it_is_given_canonical() {
    let fixture = Fixture::inline();
    let spelled = fixture.root.join("work/../home");
    let bootstrap = fixture.root.join("opt/docs/bin/../bin/server");
    let host = Host::of(homed(spelled), Ok(bootstrap));
    let server = PathBuf::from(fixture.server());
    let expected = Host {
        bootstrap: server.clone(),
        ..fixture.host()
    };
    assert_eq!(host, Ok(expected));
    let unread = || Err(std::io::Error::other("no binary"));
    let mut foreign = fixture.root.join("home").into_os_string().into_vec();
    foreign.extend_from_slice(b"/\xff");
    // Linux holds a directory of that name, so only its spelling refuses
    // it; APFS refuses a name that is not UTF-8 (EILSEQ), and holds none.
    #[cfg(target_os = "linux")]
    std::fs::create_dir(OsString::from_vec(foreign.clone())).unwrap();
    let homes = [
        Vec::new(),
        homed("home"),
        homed(OsString::from_vec(foreign)),
    ];
    for home in homes {
        assert_eq!(Host::of(home, Ok(server.clone())), Err(Unprovisioned::Root));
    }
    let home = homed(fixture.root.join("home"));
    assert_eq!(Host::of(home, unread()), Err(Unprovisioned::Bootstrap));
}

/// Operator ruling 4 of 2026-10-18 (U9b1): a host serves MCP servers only
/// where its legacy TIOCSTI setting reads `0`, as the kernel writes it or
/// bare; an absent, unreadable, enabled or any other setting is the one
/// startup cause, whose words are pinned here.
#[test]
fn a_host_whose_legacy_tiocsti_is_not_switched_off_serves_no_mcp_server() {
    let fixture = Fixture::inline();
    let served =
        |setting: std::io::Result<&[u8]>| fixture.host().served(setting.map(<[u8]>::to_vec));
    let failed = |kind: std::io::ErrorKind| Err(std::io::Error::from(kind));
    assert_eq!(
        [served(Ok(b"0\n")), served(Ok(b"0"))],
        [Ok(fixture.host()), Ok(fixture.host())]
    );
    let refused = [
        served(failed(std::io::ErrorKind::NotFound)),
        served(failed(std::io::ErrorKind::PermissionDenied)),
        served(Ok(b"1\n")),
        served(Ok(b"00\n")),
        served(Ok(b"0\n\n")),
        served(Ok(b"")),
    ];
    assert_eq!(refused, [(); 6].map(|()| Err(Unprovisioned::Terminal)));
    assert_eq!(
        Unprovisioned::Terminal.to_string(),
        "MCP servers cannot start: the host's /proc/sys/dev/tty/legacy_tiocsti is absent, \
         unreadable or not \"0\""
    );
}

/// Ruling 4's refusal is provisioning's (operator ruling of 2026-10-26):
/// an attempt that brokers nothing launches on a host that allows legacy
/// TIOCSTI, its bound whole; one that brokers a holding fails with the
/// startup cause before any survey starts, and nothing is provisioned.
#[test]
fn the_terminal_refusal_reaches_only_a_brokered_attempt_and_before_any_survey() {
    let test = "the_terminal_refusal_reaches_only_a_brokered_attempt_and_before_any_survey";
    let Some(mut fixture) = Fixture::protected(test) else {
        return;
    };
    let enabled = || fixture.host().served(Ok(b"1\n".to_vec()));
    let (none, brokered) = (enabled(), enabled());
    let whole = Duration::from_secs(10);
    let mut bound = whole;
    assert!(fixture.launch("a-none", (Instant::now(), &mut bound), none, &observing));
    assert_eq!(bound, whole);
    let server = fixture.server();
    fixture.hold("library-docs", admitted("library-docs", &[&server], true));
    let survey = |_: &Path, _: &Survey, _: Instant| panic!("a box was surveyed");
    let launched = fixture.launch("a-tty", (Instant::now(), &mut bound), brokered, &survey);
    let events = fixture.engine.store.load(&fixture.engine.run_id).unwrap();
    let failed = events.last().unwrap();
    let error = Unprovisioned::Terminal.to_string();
    let payload = json!({"effect_id": "effect", "attempt_id": "a-tty", "error": error});
    assert_eq!(
        (launched, failed.event_type, &failed.payload, bound),
        (false, EventType::EffectFailed, &payload, whole)
    );
    assert!(!fixture.run_dir().exists(), "nothing was provisioned");
}

/// M2: every survey of an attempt answers by MB3's startup deadline or
/// the attempt's bound, whichever comes first, and the driver is handed
/// only what provisioning left of the bound; nothing left is spent.
#[test]
fn the_surveys_answer_within_the_bound_and_the_driver_gets_only_what_is_left() {
    let began = Instant::now();
    let second = Duration::from_secs(1);
    assert_eq!(startup(began, 10 * second), began + 10 * second);
    assert_eq!(startup(began, 60 * second), began + 30 * second);
    assert_eq!(left(10 * second, began, began + 4 * second), Ok(6 * second));
    for spent in [10, 11] {
        let now = began + spent * second;
        assert_eq!(left(10 * second, began, now), Err(Unprovisioned::Spent));
    }
}

/// M2 in the engine: a provisioned attempt's bound is left holding what
/// its provisioning did not spend, its surveys asked to answer by the
/// bound where that comes before MB3's deadline; one with nothing left
/// fails with the refusal's words, its bound untouched; one that
/// provisions nothing keeps its bound whole, on a host whose HOME could
/// not be read too, which refuses only an attempt that brokers a holding.
#[test]
fn provisioning_leaves_the_driver_only_what_it_did_not_spend() {
    let test = "provisioning_leaves_the_driver_only_what_it_did_not_spend";
    let Some(mut fixture) = Fixture::protected(test) else {
        return;
    };
    let ago = |seconds| {
        Instant::now()
            .checked_sub(Duration::from_secs(seconds))
            .unwrap()
    };
    let whole = Duration::from_secs(10);
    let mut bound = whole;
    let homeless = || Err(Unprovisioned::Root);
    assert!(fixture.launch("a-none", (ago(4), &mut bound), homeless(), &observing));
    assert_eq!(bound, whole);
    let server = fixture.server();
    fixture.hold("library-docs", admitted("library-docs", &[&server], true));
    let refused = fixture.launch("a-homeless", (ago(4), &mut bound), homeless(), &observing);
    assert!(!refused);
    let deadlines = RefCell::new(Vec::new());
    let survey = |bootstrap: &Path, request: &Survey, deadline: Instant| {
        deadlines.borrow_mut().push(deadline);
        observing(bootstrap, request, deadline)
    };
    let began = ago(4);
    assert!(fixture.launch("a-left", (began, &mut bound), Ok(fixture.host()), &survey));
    assert_eq!(deadlines.into_inner(), [began + whole]);
    let unspent = Duration::from_secs(5)..=Duration::from_secs(6);
    assert!(unspent.contains(&bound), "{bound:?}");
    let mut spent = whole;
    let host = Ok(fixture.host());
    assert!(!fixture.launch("a-spent", (ago(10), &mut spent), host, &observing));
    assert_eq!(spent, whole);
    let events = fixture.engine.store.load(&fixture.engine.run_id).unwrap();
    let failed = events.last().unwrap();
    assert_eq!(failed.event_type, EventType::EffectFailed);
    let error = Unprovisioned::Spent.to_string();
    let payload = json!({"effect_id": "effect", "attempt_id": "a-spent", "error": error});
    assert_eq!(failed.payload, payload);
}

/// MB4: the store is sealed as the broker reads it, absolute with no `.`
/// or `..`: up to its last `..` resolved as the kernel resolves it, and
/// the rest as spelled, a symlink kept a symlink; a store whose `..` does
/// not resolve has no path.
#[test]
fn the_store_is_sealed_absolute_and_normal_its_symlinks_kept() {
    let fixture = Fixture::inline();
    let root = &fixture.root;
    std::os::unix::fs::symlink(root.join("ro"), root.join("linked")).unwrap();
    let through = root.join("work/./../linked/./private/secrets.env");
    assert_eq!(
        stored(&through),
        Ok(root.join("linked/private/secrets.env"))
    );
    let after = root.join("linked/../home/.secrets");
    assert_eq!(stored(&after), Ok(root.join("home/.secrets")));
    assert_eq!(stored(&root.join("a/./b")), Ok(root.join("a/b")));
    assert_eq!(
        stored(&root.join("missing/../secrets.env")),
        Err(Unprovisioned::Store)
    );
}

/// The reach a plan seals binds each of the site's roots where the hands
/// box binds it: a bind's `~/` is the host HOME the box is given.
#[test]
fn the_sealed_reach_names_each_hands_root_where_the_hands_box_binds_it() {
    let mut fixture = Fixture::inline();
    let home = fixture.root.join("home");
    let workdir = fixture.root.join("work");
    let spec = fixture.spec().clone();
    let git = GitFacts::default();
    let sealed = reach(&spec, &home, &workdir, &git);
    let scratch = fixture.root.join("scratch");
    let command = ["true".to_string()];
    let artifacts = fixture.root.join("artifacts");
    std::fs::create_dir_all(&artifacts).unwrap();
    let observed = brokkr_protocol::hands::git_facts(&workdir);
    let boxed = brokkr_protocol::hands::seal(&spec, &home, &workdir, &observed, None);
    // The launch that holds the box's handles (U8k), `echo` standing in
    // for bubblewrap: what it prints is the argv bubblewrap would read.
    let (echo, session) = (Path::new("/bin/echo"), fixture.root.join("session"));
    let echoed = brokkr_protocol::hands::execute_in(
        echo,
        &spec,
        &workdir,
        &artifacts,
        &boxed,
        &scratch,
        &session,
        &git.identity,
        &command.join(" "),
        Duration::from_secs(30),
    );
    let echoed = echoed.unwrap().stdout;
    let argv: Vec<String> = echoed.split_whitespace().map(String::from).collect();
    let roots = sealed.writable.iter().chain(&sealed.readable);
    let roots: Vec<String> = roots.map(|root| root.display().to_string()).collect();
    assert_eq!(roots.len(), 4);
    for root in roots {
        assert!(argv.contains(&root), "{root} is bound by {argv:?}");
    }
}

/// A site with no brokered holding — a native one, or an MCP holding its
/// candidate's own set does not broker — provisions nothing, its host's
/// unread HOME refusing nothing; the requested input and its digest do not
/// move with a holding.
#[test]
fn a_site_without_a_brokered_holding_provisions_nothing_and_the_requested_digest_holds() {
    let mut fixture = Fixture::inline();
    let current = state(Some("work"), Cursor::RequestEffect);
    let requested = fixture
        .engine
        .seat_input(&current, "work", "effect")
        .unwrap();
    fixture.hold("web-search", native_holding());
    assert_eq!(fixture.unread("a-native"), Ok(None));
    // A brokered holding moves neither the requested input nor its digest.
    let server = fixture.server();
    fixture.hold("library-docs", admitted("library-docs", &[&server], true));
    let input = fixture.engine.seat_input(&current, "work", "effect");
    assert_eq!(input.unwrap(), requested);
    let site = fixture.engine.bundle.sites.get_mut("work").unwrap();
    site.inline_mcp = Some(crate::bundle::McpIntent::Empty);
    assert_eq!(fixture.unread("a-empty"), Ok(None));
    assert!(!fixture.root.join("home/.local").exists());
}

/// The measured discovery declaration a Codex link carries.
fn discovering() -> brokkr_protocol::adapters::HandsNotice {
    let declared =
        json!({"workspace_tool": "mcp__brokkr__workspace", "discovery_tool": "tool_search"});
    brokkr_protocol::adapters::HandsNotice::parse(&declared).unwrap()
}

/// What the selected servers carrier names for the fixture's
/// `library-docs` holding, beside `discovery`.
fn named(discovery: Value) -> Value {
    json!({"servers": [{"server": "cap-library-docs", "capability": "library-docs",
                        "tools": ["lookup", "fetch"]}],
           "discovery_tool": discovery})
}

/// U7c (D8, MB1): the serving candidate is named exactly the servers its
/// own set brokers, their exact tools and its own adapter's discovery tool,
/// written at spawn and cleared first on every selection: a fallback with
/// no discovery declaration hears none, a dropped wanted holding, a native
/// holding and a set that brokers nothing leave no carrier, a reused input
/// keeps none, and an inline site is named its own. No plan, store or
/// ledger locator is among them.
#[test]
fn the_selected_servers_are_named_from_the_serving_candidates_own_set_and_cleared_first() {
    let mut fixture = Fixture::inline();
    let server = fixture.server();
    fixture.hold("library-docs", admitted("library-docs", &[&server], true));
    fixture.hold("web-search", native_holding());
    let marks = fixture.engine.marks();
    let carrier = |link: Option<&Candidate>| {
        let mut input = json!({"capability_servers": {"servers": "stale"}, "context": {}});
        marks.site("work", false, link, &mut input);
        input.get("capability_servers").cloned()
    };
    let primary = Candidate {
        hands_notice: Some(discovering()),
        ..codex_primary()
    };
    assert_eq!(carrier(Some(&primary)), Some(named(json!("tool_search"))));
    // A fallback whose adapter declares no discovery tool hears none.
    assert_eq!(carrier(Some(&codex_primary())), Some(named(Value::Null)));
    // A fallback whose own set carries no broker is named nothing.
    assert_eq!(carrier(Some(&dsh_fallback())), None);
    // The inline site is named from its own set and its own notice.
    let inline = (fixture.engine.bundle.sites.get_mut("work")).unwrap();
    inline.inline_hands_notice = Some(discovering());
    let marks = fixture.engine.marks();
    let mut input = json!({"context": {}});
    marks.servers("work", None, &mut input);
    assert_eq!(input["capability_servers"], named(json!("tool_search")));
    // A dropped wanted holding leaves only the native one: nothing is named.
    let site = fixture.engine.bundle.sites.get_mut("work").unwrap();
    let outcome = &mut site.capabilities.as_mut().unwrap().outcomes[0];
    outcome.held.remove("library-docs");
    outcome
        .not_held
        .insert("library-docs".into(), "dropped".into());
    let mut input = json!({"capability_servers": named(Value::Null)});
    fixture
        .engine
        .marks()
        .servers("work", Some(&primary), &mut input);
    assert_eq!(input, json!({}));
}

/// U7c (MB1): every executing site of a panel provisions its own selected
/// link's holdings — a fallback's, never its primary's — under its own
/// label and instance, beside the member whose set brokers nothing; the
/// inventory pins them in the sites' order.
#[test]
fn each_member_provisions_its_own_selected_links_holdings_under_its_own_site() {
    let test = "each_member_provisions_its_own_selected_links_holdings_under_its_own_site";
    let Some(mut fixture) = Fixture::protected(test) else {
        return;
    };
    let server = fixture.server();
    let docs = |capability: &str| admitted(capability, &[&server], true);
    let luna = Candidate {
        model: "luna".into(),
        ..codex_primary()
    };
    let work = fixture.engine.bundle.sites["work"].clone();
    for (member, held) in [("a", ["docs-a", "unused"]), ("b", ["stale-docs", "docs-b"])] {
        let mut site = work.clone();
        let capabilities = site.capabilities.as_mut().unwrap();
        let mut fallback = capabilities.outcomes[0].clone();
        fallback.model = Some("luna".into());
        capabilities.outcomes[0]
            .held
            .insert(held[0].into(), docs(held[0]));
        fallback.held.insert(held[1].into(), docs(held[1]));
        capabilities.outcomes.push(fallback);
        fixture
            .engine
            .bundle
            .sites
            .insert(format!("work:{member}"), site);
    }
    let mut selection = Selection::new();
    let members = [("a", codex_primary()), ("b", luna), ("c", dsh_fallback())];
    for (index, (member, link)) in members.into_iter().enumerate() {
        selection.insert(Some(member.into()), link);
        let instance = index.to_string().repeat(64);
        selection
            .plans
            .insert(Some(member.into()), planned(&"0".repeat(64), &instance));
    }
    let attempt = Attempt {
        effect: "effect",
        attempt: "a-panel",
        site: "work",
    };
    let surveyor = (
        &observing as Surveyor<'_>,
        Instant::now() + Duration::from_secs(60),
    );
    let host = Ok(fixture.host());
    let commits = &Cell::new(Commit::Synced);
    let control = (fixture.engine).provisioned(&attempt, &selection, host, surveyor, commits);
    let (_, plans) = sealed(&control.unwrap().unwrap().control);
    let owned: Vec<[&Value; 3]> = (plans.iter())
        .map(|plan| {
            [
                &plan["owner"]["site"],
                &plan["owner"]["instance"],
                &plan["capability"],
            ]
        })
        .collect();
    let (a, b) = (json!("0".repeat(64)), json!("1".repeat(64)));
    assert_eq!(
        owned,
        [
            [&json!("work:a"), &a, &json!("docs-a")],
            [&json!("work:b"), &b, &json!("docs-b")],
        ]
    );
}

/// Execute the fixture's attempt under a capturing driver and check it
/// fails after its durable start with exactly `error`, the driver never
/// started.
#[cfg(target_os = "linux")]
fn refused_at_launch(mut fixture: Fixture, error: &Unprovisioned) {
    let captured = fixture.root.join("started.json");
    let result = json!({"result": "complete"});
    let command = capturing_driver_command("effect", "attempt", &captured, result);
    let seat = fixture.engine.bundle.seats.get_mut("work").unwrap();
    seat.body = single_body(command);
    let effect = requested(&fixture.engine, "effect");
    execute_work(&mut fixture.engine, &[effect]).unwrap();
    let events = fixture.engine.store.load(&fixture.engine.run_id).unwrap();
    let [.., started, ended] = events.as_slice() else {
        panic!("the attempt journaled no start and end");
    };
    assert_eq!(started.event_type, EventType::EffectStarted);
    assert_eq!(ended.event_type, EventType::EffectFailed);
    let failure = json!({"effect_id": "effect", "attempt_id": started.payload["attempt_id"],
                         "error": error.to_string()});
    assert_eq!(ended.payload, failure);
    assert!(!captured.exists(), "no driver started");
}

/// U7c (SD3, CR3): an attempt whose plans cannot be provisioned fails after
/// its durable start with the refusal's own words, and no driver starts:
/// an oversized allocation, and a wanted holding without its seal, both
/// before the host is consulted. Linux only: off Linux no mount table is
/// read, so the dispatch door refuses every boxed writer beside evidence
/// before anything is provisioned (decision 0063; U8a2a).
#[cfg(target_os = "linux")]
#[test]
fn an_attempt_whose_plans_cannot_be_provisioned_fails_before_any_driver_starts() {
    let mut fixture = Fixture::inline();
    let server = fixture.server();
    for index in 0..33 {
        let held = format!("docs-{index}");
        fixture.hold(&held, admitted(&held, &[&server], true));
    }
    refused_at_launch(fixture, &Unprovisioned::Budget);
    let mut fixture = Fixture::inline();
    let mut unsealed = admitted("status", &["true"], false);
    unsealed.admitted = None;
    fixture.hold("status", unsealed);
    fixture.ask("status", Strength::Wants);
    refused_at_launch(fixture, &Unprovisioned::Unsealed("status".into()));
}
