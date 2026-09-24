//! Decision 0065 rulings 4, 5 and 8, proved along the whole chain a launch
//! takes: a seat compiled in a realm, its resolved native controls handed
//! over exactly as the engine hands them, and the argv the harness would
//! be spawned with.
//!
//! The adapters are the SHIPPED ones, so the OFF pair asserted here is the
//! one `adapters/codex.json` declares from the controller's measurement.
//! Everything here is composition evidence: what the argv says. Whether a
//! provider then honours it — cold beyond codex-cli 0.154.0, resumed at
//! all, or on Claude — is the controller's to measure and is not claimed.

use std::path::{Path, PathBuf};

use brokkr_core::realms::{Boundary, RealmMap};
use brokkr_runtime::capabilities::CapabilityContext;
use brokkr_runtime::{Bundle, SeatBody};
use serde_json::{json, Value};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

const OFF: [&str; 2] = ["-c", "web_search=\"disabled\""];

fn write(root: &Path, relative: &str, value: &Value) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

/// An operator's configuration directory in a temp dir: the shipped
/// `web-search` definition and Codex dialect, a second dialect serving the
/// same capability, a library with one searching office on a Claude →
/// Codex chain, and a bundle directory beside them.
/// Every fixture path derives from the canonicalised root, so it is the
/// one the filesystem resolves (macOS's `/var` is `/private/var`).
struct Operator {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Operator {
    fn new() -> Operator {
        let dir = tempfile::tempdir().unwrap();
        let operator = Operator {
            root: std::fs::canonicalize(dir.path()).unwrap(),
            _dir: dir,
        };
        let root = operator.root();
        for shipped in [
            "capabilities/web-search.json",
            "capabilities/web-fetch.json",
            "dialects/tools/codex-native-search.json",
            "dialects/tools/claude-native-search.json",
        ] {
            let body: Value =
                serde_json::from_slice(&std::fs::read(workspace().join(shipped)).unwrap()).unwrap();
            write(root, shipped, &body);
        }
        let mut second: Value = serde_json::from_slice(
            &std::fs::read(root.join("dialects/tools/codex-native-search.json")).unwrap(),
        )
        .unwrap();
        second["name"] = json!("codex-search-second");
        write(root, "dialects/tools/codex-search-second.json", &second);
        std::fs::create_dir_all(root.join("agents/charters")).unwrap();
        std::fs::write(root.join("agents/charters/searcher.md"), "# searcher\n").unwrap();
        for (name, models, efforts) in [
            ("searcher", json!(["astra"]), json!({"astra": "high"})),
            (
                "fallback",
                json!(["opus", "astra"]),
                json!({"astra": "high", "opus": "high"}),
            ),
        ] {
            write(
                root,
                &format!("agents/{name}.json"),
                &json!({
                    "description": "an office that may search",
                    "charter": "charters/searcher.md",
                    "models": models,
                    "efforts": efforts,
                    "hands": {"kind": "workspace", "network": false, "binds": []},
                    "capabilities": {"web-search": "wants"},
                }),
            );
        }
        std::fs::create_dir_all(root.join("bundle/roles")).unwrap();
        std::fs::write(root.join("bundle/roles/role.md"), "# role\n").unwrap();
        write(
            root,
            "bundle/policy.json",
            &json!({
                "phases": ["inline", "boxed", "agent", "chain", "review", "done"],
                "initial": "inline", "terminal": ["done"],
                "rules": [
                    {"id": "A", "from": "inline", "result": "complete", "next": "boxed", "reason": "r"},
                    {"id": "B", "from": "boxed", "result": "complete", "next": "agent", "reason": "r"},
                    {"id": "C", "from": "agent", "result": "complete", "next": "chain", "reason": "r"},
                    {"id": "D", "from": "chain", "result": "complete", "next": "review", "reason": "r"},
                    {"id": "E", "from": "review", "result": "clean", "next": "done", "reason": "r"},
                ],
            }),
        );
        operator
    }

    fn root(&self) -> &Path {
        &self.root
    }

    /// The realm `private` under a v6 map granting `capabilities`.
    fn context(&self, capabilities: Value) -> CapabilityContext {
        let map = json!({"schema": "forge.realms/v6", "journal": "forge.db", "realms": [
            {"name": "private", "path": "repo", "default_branch": "main",
             "capabilities": capabilities}]});
        let (map, _) = RealmMap::of("realms.json", map).unwrap();
        CapabilityContext {
            realm: "private".into(),
            grants: map.realms[0].grants.clone(),
            root: self.root().to_path_buf(),
        }
    }

    /// Four Codex seats: inline unboxed, inline boxed, agent-backed, and
    /// an agent whose Codex lane is the FALLBACK of a Claude primary.
    /// `asks` is what the two inline seats write; `seat` is what the two
    /// agent-backed seats write over their office's asks, if anything.
    fn compile(
        &self,
        context: &CapabilityContext,
        boundary: Boundary,
        asks: Option<Value>,
        seat: Option<Value>,
    ) -> Result<Bundle, String> {
        self.compile_against(&workspace().join("adapters"), context, boundary, asks, seat)
    }

    /// The same four seats against a stated adapters root: the shipped
    /// declarations, or a copy one axis of which a test has edited.
    fn compile_against(
        &self,
        adapters: &Path,
        context: &CapabilityContext,
        boundary: Boundary,
        asks: Option<Value>,
        seat: Option<Value>,
    ) -> Result<Bundle, String> {
        let codex = |extra: &[&str]| {
            let mut command = vec![
                "{brokkr}",
                "driver",
                "codex",
                "--",
                "--model",
                "gpt-6-astra",
                "--effort",
                "high",
            ];
            command.extend(extra);
            json!({"command": command})
        };
        let mut inline = json!({"results": ["complete"], "role": "roles/role.md",
            "driver": codex(&["--sandbox", "workspace-write"])});
        // An inline seat with hands authors NO box tokens: a recipe's argv
        // carries no capability server, the engine's own included (decision
        // 0066 ruling 4). The boxed seat that does get the workspace server
        // is the agent-backed one, whose adapter owns the fragment.
        let mut boxed = json!({"results": ["complete"], "role": "roles/role.md",
            "hands": {"kind": "workspace", "network": false, "binds": []},
            "driver": codex(&["--sandbox", "read-only"])});
        let mut agent = json!({"results": ["complete"], "agent": "searcher"});
        let mut chain = json!({"results": ["complete"], "agent": "fallback"});
        if let Some(asks) = asks {
            inline["capabilities"] = asks.clone();
            boxed["capabilities"] = asks;
        }
        if let Some(seat) = seat {
            agent["capabilities"] = seat.clone();
            chain["capabilities"] = seat;
        }
        let mut seats = json!({"inline": inline, "agent": agent, "chain": chain,
            "review": {"results": ["clean"], "role": "roles/role.md",
                       "driver": {"command": ["driver"]}}});
        // Under `harness` an inline model site with hands is refused, and a
        // Claude link declares no writable harness fragment: both are
        // decision 0046's law, not this one's, so the unboxed matrix keeps
        // the inline and the agent-backed Codex seats and seats a plain
        // custom driver in the other two phases.
        if !boundary.is_boxed() {
            let plain = json!({"results": ["complete"], "role": "roles/role.md",
                               "driver": {"command": ["driver"]}});
            boxed = plain.clone();
            seats["chain"] = plain;
        }
        seats["boxed"] = boxed;
        write(
            self.root(),
            "bundle/bundle.json",
            &json!({"name": "launch", "policy": "policy.json", "seats": seats}),
        );
        Bundle::compile_with_capabilities(
            &self.root().join("bundle"),
            &self.root().join("agents"),
            adapters,
            Some("private"),
            None,
            boundary,
            context,
        )
        .map_err(|error| error.to_string())
    }
}

/// The argv the harness would be spawned with for one site and candidate:
/// the compiled driver argv after `--`, and the input the engine writes.
fn launch(bundle: &Bundle, label: &str, candidate: usize) -> Vec<String> {
    try_launch(bundle, label, candidate)
        .unwrap_or_else(|refusal| panic!("{label}[{candidate}] refused: {refusal}"))
}

/// [`launch`], with the driver's refusal where it refuses.
fn try_launch(bundle: &Bundle, label: &str, candidate: usize) -> Result<Vec<String>, String> {
    let facts = &bundle.sites[label];
    let outcome = &facts.capabilities.as_ref().unwrap().outcomes[candidate];
    let argv: Vec<String> = match facts.chain.get(candidate) {
        Some(link) => link.argv.clone(),
        None => match &bundle.seats[label].body {
            SeatBody::Single { command, .. } => command.clone(),
            _ => panic!("{label} is a single seat"),
        },
    };
    // Composed by the engine's own function, so the boundary's fragment —
    // the box's tokens, or the harness's own sandbox under `harness` — is
    // in the argv exactly as it is at a real spawn, and an inline site's
    // lowered allow is the engine's own segment, as dispatch composes it.
    let built = match bundle.boundary.is_boxed() {
        true => brokkr_runtime::engine::BuiltBoundary::Namespace,
        false => brokkr_runtime::engine::BuiltBoundary::Harness,
    };
    let spawn = brokkr_runtime::engine::compose_site_at(
        Some(facts),
        built,
        brokkr_runtime::SeatClass::Work,
        argv,
        bundle.hands.get(label),
        facts.chain.get(candidate),
        Path::new("/w"),
        &[],
        "/w/result.json",
        None,
    );
    let argv = &spawn.argv;
    let extra = &argv[argv.iter().position(|part| part == "--").unwrap() + 1..];
    // The plan AND the argv's two parts, each exactly as the engine writes
    // it: the driver refuses a launch whose provenance it cannot reassemble.
    let input = json!({"workdir": "/w", "seat": label, "native_controls": outcome.controls(),
                       "launch_arguments": spawn.launch_arguments()});
    match outcome.provider.as_str() {
        "codex" => brokkr_protocol::adapters::codex_command("codex", extra, "/w", None, &input),
        _ => brokkr_protocol::adapters::claude_command("claude", extra, None, &input),
    }
}

/// One compiled site and candidate composed as [`try_launch`] composes it,
/// then sealed exactly as dispatch seals it — the site's facts and the
/// serving outcome, through the engine's own functions — with the input
/// the driver is handed, which the dispatch door admits.
fn sealed(
    bundle: &Bundle,
    label: &str,
    candidate: usize,
) -> (brokkr_runtime::engine::SiteSpawn, Value) {
    use brokkr_runtime::engine::{verify_record, LAUNCH_RECORD};
    let (spawn, sealing) = sealing(bundle, label, candidate, &bundle.sites[label]);
    assert_eq!(sealing, Ok(()), "{label}[{candidate}]");
    let input = json!({LAUNCH_RECORD: spawn.launch_record()});
    assert_eq!(
        verify_record(&spawn, &input),
        Ok(()),
        "{label}[{candidate}]"
    );
    (spawn, input)
}

/// [`sealed`] over `facts` — the site's own, or a copy a test has moved —
/// with what the seal said: the expected state is filled from those facts
/// and the spawn composed from them, through the engine's own functions.
fn sealing(
    bundle: &Bundle,
    label: &str,
    candidate: usize,
    facts: &brokkr_runtime::bundle::SiteFacts,
) -> (brokkr_runtime::engine::SiteSpawn, Result<(), String>) {
    sealing_moved(bundle, label, candidate, facts, |_| {})
}

/// [`sealing`], with `moved` applied to the composed spawn before it is
/// sealed — a contribution changed on its way into the command.
fn sealing_moved(
    bundle: &Bundle,
    label: &str,
    candidate: usize,
    facts: &brokkr_runtime::bundle::SiteFacts,
    moved: impl FnOnce(&mut brokkr_runtime::engine::SiteSpawn),
) -> (brokkr_runtime::engine::SiteSpawn, Result<(), String>) {
    use brokkr_runtime::engine::expected_state;
    let outcome = &facts.capabilities.as_ref().unwrap().outcomes[candidate];
    let link = facts.chain.get(candidate);
    let argv: Vec<String> = match link {
        Some(link) => link.argv.clone(),
        None => match &bundle.seats[label].body {
            SeatBody::Single { command, .. } => command.clone(),
            _ => panic!("{label} is a single seat"),
        },
    };
    let built = match bundle.boundary.is_boxed() {
        true => brokkr_runtime::engine::BuiltBoundary::Namespace,
        false => brokkr_runtime::engine::BuiltBoundary::Harness,
    };
    let mut spawn = brokkr_runtime::engine::compose_site_at(
        Some(facts),
        built,
        brokkr_runtime::SeatClass::Work,
        argv,
        bundle.hands.get(label),
        link,
        Path::new("/w"),
        &[],
        "/w/result.json",
        None,
    );
    assert_eq!(spawn.refusal, None, "{label}[{candidate}]");
    moved(&mut spawn);
    let sealing =
        expected_state(outcome, link, Some(facts)).and_then(|expected| spawn.seal(expected));
    (spawn, sealing)
}

/// Rebuild unit 5c-fix: the whole refusal of an agent-backed seat whose
/// composition emits a permission template, until unit 5c-fix2.
const AGENT_TEMPLATE_REFUSED: &str =
    "dispatch refused: the selected candidate's composition emits a permission template behind \
     its driver verb, or does not open with its driver template, and until rebuild unit 5c-fix2 \
     an agent-backed seat records its template only as none, never as the segment it emitted \
     (operator ruling 2 of 2026-09-23; rebuild unit 5c-fix), so no launch record can be sealed \
     for this site; a record is sealed from typed facts and never repaired into a default one \
     (decision 0065 slice one, design D5.7)";

/// Rebuild unit 5c-fix: the whole refusal of a seal whose emitted template
/// contradicts the expected state's.
const TEMPLATE_CONTRADICTED: &str =
    "dispatch refused: the permission template this spawn emits is not the one its expected \
     state records from the adapter's declaration; a template omitted, altered or added on its \
     way into the command is never sealed as the engine's (operator ruling of 2026-09-24, the \
     permission template at inline sites; rebuild unit 5c-fix)";

fn off_pairs(argv: &[String]) -> usize {
    argv.windows(2).filter(|pair| *pair == OFF).count()
}

const THREAD: &str = "0198c0de-5e55-7000-8000-000000000001";

/// A stand-in `codex` that answers the version probe with the version the
/// shipped assessment is qualified against, and nothing else: composing a
/// launch runs no model. Staged beside its name and renamed in, so the
/// file is never open for writing when it is executed.
#[cfg(unix)]
fn codex_reporting(dir: &Path, version: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let staged = dir.join("codex.staged");
    std::fs::write(
        &staged,
        format!("#!/bin/sh\nprintf 'codex-cli {version}\\n'\n"),
    )
    .unwrap();
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).unwrap();
    let shim = dir.join("codex");
    std::fs::rename(&staged, &shim).unwrap();
    shim
}

/// The argv of an ACTUAL rejoin of one compiled Codex site: the same
/// composition [`launch`] makes, with what the engine writes for a site
/// whose session is offered — the site's confinement markers exactly as
/// `mark_hands` states them, the resume assessment the bundle COMPILED for
/// the site (the shipped adapter's, never a test's), and the capability
/// plan its outcome resolved to.
#[cfg(unix)]
fn rejoin(bundle: &Bundle, label: &str, shim: &Path) -> Vec<String> {
    let facts = &bundle.sites[label];
    let outcome = &facts.capabilities.as_ref().unwrap().outcomes[0];
    let (argv, assessment, boundary) = match facts.chain.first() {
        Some(link) => (link.argv.clone(), link.resume.value(), "harness"),
        None => match &bundle.seats[label].body {
            SeatBody::Single { command, .. } => (
                command.clone(),
                facts.inline_resume.clone().expect("an inline codex seat"),
                "not applicable",
            ),
            _ => panic!("{label} is a single seat"),
        },
    };
    let spawn = brokkr_runtime::engine::compose_site(
        brokkr_runtime::engine::BuiltBoundary::Harness,
        brokkr_runtime::SeatClass::Work,
        argv,
        bundle.hands.get(label),
        facts.chain.first(),
        Path::new("/w"),
        &[],
        "/w/result.json",
        None,
    );
    let argv = &spawn.argv;
    let extra = &argv[argv.iter().position(|part| part == "--").unwrap() + 1..];
    let input = json!({
        "workdir": "/w", "seat": label, "boundary": boundary, "hands": "none",
        "resume_context": {"assessment": assessment},
        "native_controls": outcome.controls(),
        "launch_arguments": spawn.launch_arguments(),
    });
    brokkr_protocol::adapters::codex_command(
        shim.to_str().unwrap(),
        extra,
        "/w",
        Some(THREAD),
        &input,
    )
    .unwrap_or_else(|refusal| panic!("{label} refused: {refusal}"))
}

/// The seat's own controls survive beside the managed one.
fn assert_intact(argv: &[String], label: &str) {
    for expected in [
        ["--model", "gpt-6-astra"],
        ["-c", "model_reasoning_effort=\"high\""],
    ] {
        assert!(
            argv.windows(2).any(|pair| pair == expected),
            "{label} lost {expected:?}: {argv:?}"
        );
    }
    assert!(
        argv.iter().any(|part| part == "--sandbox"),
        "{label}: {argv:?}"
    );
}

/// Every Codex seat in a realm that has not granted `web-search` is
/// composed with the measured OFF pair — inline and agent-backed, boxed
/// and unboxed, the primary lane and a serving fallback — whatever the
/// reason the seat does not hold it.
#[test]
fn a_codex_seat_that_does_not_hold_search_is_launched_with_it_switched_off() {
    let operator = Operator::new();
    let granted = json!({"web-search": {"dialect": "codex-native-search"}});
    let elsewhere =
        json!({"web-search": {"dialect": "codex-native-search", "offices": ["someone-else"]}});
    let wants = Some(json!({"web-search": "wants"}));
    for (case, context, asks, seat) in [
        // Nothing asked, nothing granted: the legacy-realm reading.
        (
            "no ask",
            CapabilityContext::no_grants("private", operator.root()),
            None,
            Some(json!({})),
        ),
        (
            "a lost want",
            operator.context(json!({})),
            wants.clone(),
            None,
        ),
        (
            "out of scope",
            operator.context(elsewhere),
            wants.clone(),
            None,
        ),
        // The realm grants it to everyone, and these seats do not ask.
        (
            "granted but unasked",
            operator.context(granted.clone()),
            None,
            Some(json!({})),
        ),
    ] {
        for boundary in [Boundary::Namespace, Boundary::Harness] {
            let bundle = operator
                .compile(&context, boundary, asks.clone(), seat.clone())
                .unwrap_or_else(|error| panic!("{case} under {boundary}: {error}"));
            let mut sites = vec![("inline", 0), ("agent", 0)];
            if boundary.is_boxed() {
                sites.extend([("boxed", 0), ("chain", 1)]);
            }
            for (label, candidate) in sites {
                let argv = launch(&bundle, label, candidate);
                assert_eq!(
                    &argv[argv.len() - 2..],
                    OFF,
                    "{case}, {label} under {boundary}: {argv:?}"
                );
                assert_eq!(off_pairs(&argv), 1, "{case}, {label}: {argv:?}");
                assert_intact(&argv, label);
                let site = bundle.sites[label].capabilities.as_ref().unwrap();
                assert!(site.outcomes[candidate].held.is_empty(), "{case}, {label}");
            }
            if !boundary.is_boxed() {
                continue;
            }
            // The Claude primary of the chain is denied by name too, and
            // keeps its own outcome: not the Codex fallback's.
            let claude = launch(&bundle, "chain", 0);
            assert!(
                claude
                    .windows(2)
                    .any(|pair| pair == ["--disallowedTools", "WebFetch,WebSearch"]),
                "{case}: {claude:?}"
            );
            assert_eq!(off_pairs(&claude), 0);
        }
    }
    // A box with `network: false` does not stand in for the switch: the
    // boxed seats above carried the pair all the same.
}

/// The other way round: a seat that asks, in a realm that grants it to
/// its office, is launched on the declared ON mechanism — the measured
/// cold default — with no OFF pair. An always-OFF implementation fails
/// here.
#[test]
fn a_codex_seat_that_holds_search_is_launched_without_the_off_pair() {
    let operator = Operator::new();
    let context = operator.context(json!({"web-search": {"dialect": "codex-native-search"}}));
    let asks = Some(json!({"web-search": "requires"}));
    for boundary in [Boundary::Namespace, Boundary::Harness] {
        let bundle = operator
            .compile(&context, boundary, asks.clone(), None)
            .unwrap();
        let mut sites = vec![("inline", 0), ("agent", 0)];
        if boundary.is_boxed() {
            sites.extend([("boxed", 0), ("chain", 1)]);
        }
        for (label, candidate) in sites {
            let argv = launch(&bundle, label, candidate);
            assert_eq!(off_pairs(&argv), 0, "{label} under {boundary}: {argv:?}");
            assert_intact(&argv, label);
            let outcome = &bundle.sites[label].capabilities.as_ref().unwrap().outcomes[candidate];
            assert_eq!(outcome.held["web-search"].tools, ["web_search"], "{label}");
            assert_eq!(outcome.held["web-search"].dialect, "codex-native-search");
        }
        if !boundary.is_boxed() {
            continue;
        }
        // The Codex grant is no grant to the chain's Claude primary: its
        // want is dropped for THAT candidate alone, with the reason.
        let chain = bundle.sites["chain"].capabilities.as_ref().unwrap();
        assert!(chain.outcomes[0].held.is_empty());
        assert_eq!(
            chain.outcomes[0].notices[0].1,
            "seat 'chain' (office 'fallback') in realm 'private': dropped wanted capability \
             'web-search' through dialect 'codex-native-search' because provider 'claude' \
             cannot carry a binding to provider 'codex'; native capability remains OFF"
        );
        assert!(chain.outcomes[1].notices.is_empty());
    }
}

/// Unit 4 (design D5.7), from production-compiled seats: a link's
/// template, model and effort emissions and the engine's hands reach the
/// sealed launch record by who supplied them — through the compile's
/// `{brokkr}` expansion, the harness fragment and the box's placeholder
/// expansion, for a primary and its fallback alike — beside the serving
/// outcome's expected state. An inline seat whose arguments spell the very
/// same bytes stays the author's.
#[test]
fn a_compiled_links_origins_reach_its_sealed_record_and_copied_bytes_stay_authored() {
    use brokkr_protocol::native_controls::{flatten, Origin, Segment};
    use brokkr_runtime::agents::Lowering;
    let operator = Operator::new();
    let context = CapabilityContext::no_grants("private", operator.root());
    let strings = |parts: &[&str]| {
        parts
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>()
    };
    let origins = |record: &Value| -> Vec<Value> {
        record["segments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|segment| segment["origin"].clone())
            .collect()
    };
    let head =
        |list: &Value, count: usize| Value::Array(list.as_array().unwrap()[..count].to_vec());
    let unboxed = operator
        .compile(&context, Boundary::Harness, None, Some(json!({})))
        .unwrap();

    // The compile expanded `{brokkr}` segment by segment: the link's
    // template names this binary, and its segments are its argv.
    let link = &unboxed.sites["agent"].chain[0];
    let Lowering::Composed(composition) = &link.lowering else {
        panic!("a resolved link carries its composition");
    };
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        composition.segments[0],
        Segment::new(Origin::Template, &strings(&[&exe, "driver", "codex", "--"]))
    );
    assert_eq!(flatten(&composition.segments), link.argv);

    // `harness`: the agent-backed link's model and effort are the
    // adapter's template, its work fragment the engine's hands.
    let (agent, input) = sealed(&unboxed, "agent", 0);
    let record = &input["launch_record"];
    assert_eq!(
        record["segments"],
        json!([
            {"origin": "template", "argv": ["--model", "gpt-6-astra"]},
            {"origin": "template", "argv": ["--effort", "high"]},
            {"origin": "hands", "argv": ["--sandbox", "workspace-write"]},
        ])
    );
    assert_eq!(
        record["expected"],
        json!({
            "identity": {"provider": "codex", "harness": "codex",
                         "model": {"kind": "named", "name": "astra"}},
            "native": {"kind": "known", "held": [], "denied": ["web-search"]},
            "local": {"allow": {"kind": "unspecified"}, "sandbox": {"kind": "unspecified"},
                      "application": {"kind": "dormant"}},
            "hands": {"kind": "required"},
            "template": {"kind": "none"},
        })
    );
    // The inline seat spells the same arguments byte for byte, and every
    // one of them is authored.
    let (inline, input) = sealed(&unboxed, "inline", 0);
    let copied = &input["launch_record"];
    assert_eq!(
        copied["segments"],
        json!([{"origin": "authored",
                "argv": ["--model", "gpt-6-astra", "--effort", "high",
                         "--sandbox", "workspace-write"]}])
    );
    assert_eq!(inline.argv[1..], agent.argv[1..]);
    assert_eq!(
        copied["expected"]["local"],
        json!({"allow": {"kind": "unspecified"}, "sandbox": {"kind": "unspecified"},
               "application": {"kind": "unrestricted"}})
    );
    assert_eq!(copied["expected"]["hands"], json!({"kind": "none"}));
    // An inline site whose allow does not lower emits no template and
    // expects none (rebuild unit 5c-fix).
    assert_eq!(copied["expected"]["template"], json!({"kind": "none"}));

    // `namespace`: the Claude primary and its Codex fallback, each under
    // its own identity, with the box's workspace fragment as hands.
    let boxed = operator
        .compile(&context, Boundary::Namespace, None, Some(json!({})))
        .unwrap();
    // The boxed inline seat's arguments are all its author's, its hands
    // the site's own declaration.
    let (_, input) = sealed(&boxed, "boxed", 0);
    assert_eq!(
        input["launch_record"]["segments"],
        json!([{"origin": "authored",
                "argv": ["--model", "gpt-6-astra", "--effort", "high",
                         "--sandbox", "read-only"]}])
    );
    assert_eq!(
        input["launch_record"]["expected"]["hands"],
        json!({"kind": "required"})
    );
    // Rebuild unit 5c-fix: the Claude primary's composition emits the
    // adapter's permission template behind its verb, which an agent-backed
    // seat cannot yet record (unit 5c-fix2), so nothing is sealed for it.
    // Its spawn's extras still carry every origin.
    let (spawn, refused) = sealing(&boxed, "chain", 0, &boxed.sites["chain"]);
    assert_eq!(refused, Err(AGENT_TEMPLATE_REFUSED.to_string()));
    assert_eq!(spawn.launch_record(), Value::Null);
    let primary = json!({"segments": spawn.extras().iter().map(|segment| json!({
        "origin": segment.origin.word(), "argv": segment.argv})).collect::<Vec<_>>()});
    assert_eq!(
        origins(&primary),
        [
            json!("template"),
            json!("template"),
            json!("template"),
            json!("hands")
        ]
    );
    assert_eq!(
        head(&primary["segments"], 3),
        json!([
            {"origin": "template", "argv": ["--permission-mode", "acceptEdits"]},
            {"origin": "template", "argv": ["--model", "claude-opus-5-5"]},
            {"origin": "template", "argv": ["--effort", "high"]},
        ])
    );
    assert_eq!(
        head(&primary["segments"][3]["argv"], 3),
        json!(["--tools", "", "--strict-mcp-config"])
    );
    let (_, input) = sealed(&boxed, "chain", 1);
    let fallback = &input["launch_record"];
    assert_eq!(fallback["expected"]["template"], json!({"kind": "none"}));
    assert_eq!(
        origins(fallback),
        [json!("template"), json!("template"), json!("hands")]
    );
    assert_eq!(
        head(&fallback["segments"][2]["argv"], 3),
        json!(["--sandbox", "read-only", "-c"])
    );
    assert_eq!(
        fallback["expected"]["identity"],
        json!({"provider": "codex", "harness": "codex",
               "model": {"kind": "named", "name": "astra"}})
    );
    assert_eq!(
        fallback["expected"]["native"],
        json!({"kind": "known", "held": [], "denied": ["web-search"]})
    );
}

/// Unit 4 (design D5.7): an office's direct allow list, compiled for an
/// unboxed Claude seat, reaches the spawn as the adapter's exact mapped
/// limits under the `local` origin — beside the template it was composed
/// after — with the ordered names and limits in its composition's local
/// expectation, independently of the joined flag value. Rebuild unit
/// 5c-fix: until unit 5c-fix2, the seat's emitted permission template
/// refuses the seal with the whole agent-backed cause.
#[test]
fn a_compiled_direct_allow_list_reaches_the_record_as_exact_local_limits() {
    let operator = Operator::new();
    write(
        operator.root(),
        "agents/limited.json",
        &json!({
            "description": "an office limited to two local commands",
            "charter": "charters/searcher.md",
            "models": ["opus"],
            "efforts": {"opus": "high"},
            "tools": {"allow": ["pytest", "cargo"]},
        }),
    );
    one_inline_seat(&operator, &["driver"]);
    write(
        operator.root(),
        "solo/bundle.json",
        &json!({"name": "solo", "policy": "policy.json", "seats": {
            "work": {"results": ["complete"], "agent": "limited"},
            "review": {"results": ["clean"], "role": "roles/role.md",
                       "driver": {"command": ["driver"]}}}}),
    );
    let bundle = solo_bundle(
        &operator,
        &workspace().join("adapters"),
        &CapabilityContext::no_grants("private", operator.root()),
    )
    .unwrap();
    // Rebuild unit 5c-fix: the shipped Claude adapter's composition emits
    // its permission template, which an agent-backed seat cannot yet record
    // from a typed fact (unit 5c-fix2), so the seal refuses whole and the
    // spawn carries no record. The local limits it was composed with are
    // still the composition's own, and its extras still carry every origin.
    let (spawn, refused) = sealing(&bundle, "work", 0, &bundle.sites["work"]);
    let brokkr_runtime::agents::Lowering::Composed(composition) =
        &bundle.sites["work"].chain[0].lowering
    else {
        panic!("a resolved link carries its composition");
    };
    assert_eq!(
        json!({
            "sealing": format!("{refused:?}"),
            "record": spawn.launch_record(),
            "extras": spawn.extras().iter().map(|segment| json!({
                "origin": segment.origin.word(), "argv": segment.argv})).collect::<Vec<_>>(),
            "local": format!("{:?}", composition.local()),
        }),
        json!({
            "sealing": format!("{:?}", Err::<(), _>(AGENT_TEMPLATE_REFUSED.to_string())),
            "record": null,
            "extras": [
                {"origin": "template", "argv": ["--permission-mode", "acceptEdits"]},
                {"origin": "template", "argv": ["--model", "claude-opus-5-5"]},
                {"origin": "template", "argv": ["--effort", "high"]},
                {"origin": "local",
                 "argv": ["--allowedTools", "Bash(.venv/bin/pytest:*),Bash(cargo:*)"]},
            ],
            "local": format!("{:?}", brokkr_protocol::native_controls::LocalExpectation {
                allow: brokkr_protocol::native_controls::AllowIntent::Listed(vec![
                    "pytest".into(), "cargo".into()]),
                sandbox: brokkr_protocol::native_controls::SandboxIntent::Unspecified,
                application: brokkr_protocol::native_controls::Application::Direct(vec![
                    "Bash(.venv/bin/pytest:*)".into(), "Bash(cargo:*)".into()]),
            }),
        })
    );
}

/// Rebuild unit 5c-fix, the agent-backed arm until unit 5c-fix2: an agent's
/// seat records the template `none` only where its composition emits no
/// permission template, and is otherwise refused whole with nothing
/// sealed. Over copies of the shipped adapters, canonicalised once, the
/// same office on the Claude adapter whose driver declares the template
/// refuses; one whose driver ends at the terminator, or at the verb, seals
/// `none`; and an opaque driver, which dispatches through no verb and whose
/// argv the engine composes whole and never parses, emits no permission
/// template behind one and seals `none`.
#[test]
fn an_agent_backed_seat_records_no_template_only_where_its_composition_emits_none() {
    let operator = Operator::new();
    write(
        operator.root(),
        "agents/plain.json",
        &json!({
            "description": "an office that declares no local tools",
            "charter": "charters/searcher.md",
            "models": ["opus"],
            "efforts": {"opus": "high"},
        }),
    );
    one_inline_seat(&operator, &["driver"]);
    write(
        operator.root(),
        "solo/bundle.json",
        &json!({"name": "solo", "policy": "policy.json", "seats": {
            "work": {"results": ["complete"], "agent": "plain"},
            "review": {"results": ["clean"], "role": "roles/role.md",
                       "driver": {"command": ["driver"]}}}}),
    );
    let context = CapabilityContext::no_grants("private", operator.root());
    let adapters = copied_adapters();
    let root = std::fs::canonicalize(adapters.path()).unwrap();
    let rows = [
        (
            "the shipped template",
            json!([
                "{brokkr}",
                "driver",
                "claude",
                "--",
                "--permission-mode",
                "acceptEdits"
            ]),
            format!(
                "{:?} null",
                Err::<(), _>(AGENT_TEMPLATE_REFUSED.to_string())
            ),
        ),
        (
            "nothing behind the terminator",
            json!(["{brokkr}", "driver", "claude", "--"]),
            format!("{:?} {}", Ok::<(), String>(()), json!({"kind": "none"})),
        ),
        (
            "nothing behind the verb",
            json!(["{brokkr}", "driver", "claude"]),
            format!("{:?} {}", Ok::<(), String>(()), json!({"kind": "none"})),
        ),
        (
            "an opaque driver",
            json!(["claude-wrapper", "--permission-mode", "acceptEdits"]),
            format!("{:?} {}", Ok::<(), String>(()), json!({"kind": "none"})),
        ),
    ];
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(|(label, driver, expected)| {
            edit_adapter(&root, "claude", |adapter| adapter["driver"] = driver);
            let observed = match solo_bundle(&operator, &root, &context) {
                Ok(bundle) => {
                    let (spawn, sealing) = sealing(&bundle, "work", 0, &bundle.sites["work"]);
                    let record = spawn.launch_record();
                    let template = match record.is_null() {
                        true => Value::Null,
                        false => record["expected"]["template"].clone(),
                    };
                    format!("{sealing:?} {template}")
                }
                Err(refusal) => refusal,
            };
            (observed != expected)
                .then(|| format!("row {label}:\n  left:  {observed}\n  right: {expected}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Rebuild unit 5b (design D5.3, D5.7): an inline Claude seat that declares
/// a typed allow and writes no capability flag compiles, and the engine
/// appends the adapter's exact mapped limits behind the authored command as
/// its own `local` segment. Rebuild unit 5c: between them, the adapter's
/// declared permission template as the engine's own `template` segment,
/// read from the shipped adapter data. The authored command is never
/// rewritten, the sealed record carries all three origins and the typed
/// expectation, and the whole ordered final Claude command carries the
/// template and the lowered list beside the native OFF.
#[test]
fn an_inline_claude_seats_typed_allow_reaches_its_final_command_as_the_engines_local_limits() {
    let operator = Operator::new();
    let authored = [
        "{brokkr}",
        "driver",
        "claude",
        "--",
        "--model",
        "claude-opus-5-5",
        "--effort",
        "high",
    ];
    one_inline_seat(&operator, &authored);
    typed_allow(&operator, json!(["pytest", "cargo"]));
    let context = CapabilityContext::no_grants("private", operator.root());
    let bundle = solo_bundle(&operator, &workspace().join("adapters"), &context).unwrap();
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let SeatBody::Single { command, .. } = &bundle.seats["work"].body else {
        panic!("work is a single seat");
    };
    // Every observed fact at once, so a mutation reports each one it moves.
    let (spawn, input) = sealed(&bundle, "work", 0);
    let record = &input["launch_record"];
    assert_eq!(
        json!({
            "authored command": command,
            "spawn": spawn.argv,
            "segments": record["segments"],
            "expected": record["expected"],
            "final": solo(&operator, &workspace().join("adapters"), &context),
        }),
        json!({
            "authored command": [&exe, "driver", "claude", "--",
                                 "--model", "claude-opus-5-5", "--effort", "high"],
            "spawn": [&exe, "driver", "claude", "--", "--model", "claude-opus-5-5",
                      "--effort", "high", "--permission-mode", "acceptEdits",
                      "--allowedTools", "Bash(.venv/bin/pytest:*),Bash(cargo:*)"],
            "segments": [
                {"origin": "authored",
                 "argv": ["--model", "claude-opus-5-5", "--effort", "high"]},
                {"origin": "template", "argv": ["--permission-mode", "acceptEdits"]},
                {"origin": "local",
                 "argv": ["--allowedTools", "Bash(.venv/bin/pytest:*),Bash(cargo:*)"]},
            ],
            "expected": {
                "identity": {"provider": "claude", "harness": "claude",
                             "model": {"kind": "none"}},
                "native": {"kind": "known", "held": [], "denied": ["web-fetch", "web-search"]},
                "local": {"allow": {"kind": "listed", "names": ["pytest", "cargo"]},
                          "sandbox": {"kind": "unspecified"},
                          "application": {"kind": "direct",
                                          "limits": ["Bash(.venv/bin/pytest:*)",
                                                     "Bash(cargo:*)"]}},
                "hands": {"kind": "none"},
                "template": {"kind": "declared",
                             "argv": ["--permission-mode", "acceptEdits"]},
            },
            "final": format!(
                "launched {:?}",
                [
                    "claude",
                    "-p",
                    "--output-format",
                    "stream-json",
                    "--verbose",
                    "--model",
                    "claude-opus-5-5",
                    "--effort",
                    "high",
                    "--permission-mode",
                    "acceptEdits",
                    "--allowedTools",
                    "Bash(.venv/bin/pytest:*),Bash(cargo:*)",
                    "--disallowedTools",
                    "WebFetch,WebSearch"
                ]
            ),
        })
    );
}

/// Rebuild unit 5c: a seat whose adapter declares no permission template
/// gets none. A copy of the shipped Claude adapter whose driver ends at its
/// verb composes the same inline seat with no `template` segment, and its
/// whole ordered final command carries the pins, the lowered list and the
/// native OFF, and no permission mode.
#[test]
fn an_inline_claude_seat_whose_adapter_declares_no_template_gets_none() {
    let operator = Operator::new();
    let adapters = copied_adapters();
    let root = std::fs::canonicalize(adapters.path()).unwrap();
    edit_adapter(&root, "claude", |adapter| {
        adapter["driver"] = json!(["{brokkr}", "driver", "claude", "--"]);
    });
    one_inline_seat(
        &operator,
        &[
            "{brokkr}",
            "driver",
            "claude",
            "--",
            "--model",
            "claude-opus-5-5",
            "--effort",
            "high",
        ],
    );
    typed_allow(&operator, json!(["cargo"]));
    let context = CapabilityContext::no_grants("private", operator.root());
    let bundle = solo_bundle(&operator, &root, &context).unwrap();
    let (spawn, input) = sealed(&bundle, "work", 0);
    assert_eq!(
        json!({
            "spawn": spawn.argv[1..],
            "segments": input["launch_record"]["segments"],
            "template": input["launch_record"]["expected"]["template"],
            "final": solo(&operator, &root, &context),
        }),
        json!({
            "spawn": ["driver", "claude", "--", "--model", "claude-opus-5-5", "--effort", "high",
                      "--allowedTools", "Bash(cargo:*)"],
            "segments": [
                {"origin": "authored",
                 "argv": ["--model", "claude-opus-5-5", "--effort", "high"]},
                {"origin": "local", "argv": ["--allowedTools", "Bash(cargo:*)"]},
            ],
            "template": {"kind": "none"},
            "final": format!(
                "launched {:?}",
                [
                    "claude",
                    "-p",
                    "--output-format",
                    "stream-json",
                    "--verbose",
                    "--model",
                    "claude-opus-5-5",
                    "--effort",
                    "high",
                    "--allowedTools",
                    "Bash(cargo:*)",
                    "--disallowedTools",
                    "WebFetch,WebSearch"
                ]
            ),
        })
    );
}

/// Rebuild unit 5c-fix (operator ruling of 2026-09-24, item 2; ruling 2 of
/// 2026-09-23): at the production-compiled inline Claude seat, the expected
/// state records the adapter's declared template from the compiler's typed
/// fact, never from the segment emitted, and a seal whose template-origin
/// segments contradict it refuses whole and seals nothing. Each row moves
/// one fact: the emitted segment omitted, altered or relabelled in the
/// site's facts; a template segment added to, or relabelled in, the
/// composed spawn; the declaration contradicting an unchanged emission;
/// and a declaration never recorded, which refuses the expected state.
#[test]
fn an_inline_seal_whose_emitted_template_contradicts_the_declared_one_refuses() {
    use brokkr_protocol::native_controls::{Origin, Segment, TemplateExpectation};
    use brokkr_runtime::bundle::SiteFacts;
    use brokkr_runtime::engine::SiteSpawn;
    let operator = Operator::new();
    one_inline_seat(
        &operator,
        &[
            "{brokkr}",
            "driver",
            "claude",
            "--",
            "--model",
            "claude-opus-5-5",
            "--effort",
            "high",
        ],
    );
    typed_allow(&operator, json!(["cargo"]));
    let context = CapabilityContext::no_grants("private", operator.root());
    let bundle = solo_bundle(&operator, &workspace().join("adapters"), &context).unwrap();
    let site = &bundle.sites["work"];
    let strings = |parts: &[&str]| {
        parts
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>()
    };
    let acceptance = strings(&["--permission-mode", "acceptEdits"]);
    let facts = |edit: &dyn Fn(&mut SiteFacts)| {
        let mut facts = site.clone();
        edit(&mut facts);
        facts
    };
    // Insert `segment` into the spawn at segment `at`, its argv with it.
    let inserted = |at: usize, segment: Segment| {
        move |spawn: &mut SiteSpawn| {
            let start: usize = spawn.segments[..at]
                .iter()
                .map(|segment| segment.argv.len())
                .sum();
            spawn
                .argv
                .splice(start..start, segment.argv.iter().cloned());
            spawn.segments.insert(at, segment);
        }
    };
    let never_declared = "dispatch refused: the inline site's lowered allow carries no recorded \
                          declaration of its adapter's permission template, so no launch record \
                          can be sealed for this site; a record is sealed from typed facts and \
                          never repaired into a default one (decision 0065 slice one, design \
                          D5.7)";
    type Row = (
        &'static str,
        SiteFacts,
        Box<dyn FnOnce(&mut SiteSpawn)>,
        Result<(), String>,
    );
    let contradicted = || Err(TEMPLATE_CONTRADICTED.to_string());
    let rows: Vec<Row> = vec![
        ("as compiled", site.clone(), Box::new(|_| {}), Ok(())),
        (
            "the emitted template omitted",
            facts(&|facts| facts.inline_template = None),
            Box::new(|_| {}),
            contradicted(),
        ),
        (
            "the emitted template altered",
            facts(&|facts| {
                facts.inline_template = Some(Segment::new(
                    Origin::Template,
                    &[
                        "--permission-mode".to_string(),
                        "bypassPermissions".to_string(),
                    ],
                ))
            }),
            Box::new(|_| {}),
            contradicted(),
        ),
        (
            "the emitted template relabelled as authored",
            facts(&|facts| {
                facts.inline_template = Some(Segment::new(Origin::Authored, &acceptance))
            }),
            Box::new(|_| {}),
            contradicted(),
        ),
        (
            "a second template segment added to the spawn",
            site.clone(),
            Box::new(inserted(2, Segment::new(Origin::Template, &acceptance))),
            contradicted(),
        ),
        (
            "the local segment relabelled as template in the spawn",
            site.clone(),
            Box::new(|spawn: &mut SiteSpawn| spawn.segments[2].origin = Origin::Template),
            contradicted(),
        ),
        (
            "the declaration none beside an emitted template",
            facts(&|facts| facts.declared_template = Some(TemplateExpectation::None)),
            Box::new(|_| {}),
            contradicted(),
        ),
        (
            "the declaration altered beside an unchanged emission",
            facts(&|facts| {
                facts.declared_template = Some(TemplateExpectation::Declared(
                    ["--permission-mode", "plan"].map(String::from).to_vec(),
                ))
            }),
            Box::new(|_| {}),
            contradicted(),
        ),
        (
            "the declaration never recorded",
            facts(&|facts| facts.declared_template = None),
            Box::new(|_| {}),
            Err(never_declared.to_string()),
        ),
    ];
    assert_eq!(rows.len(), 9);
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(|(label, facts, moved, expected)| {
            let (spawn, observed) = sealing_moved(&bundle, "work", 0, &facts, moved);
            // A refused seal leaves no record behind; an admitted one
            // records the declared template.
            let record = spawn.launch_record();
            let recorded = match &expected {
                Ok(()) => json!({"kind": "declared", "argv": acceptance}),
                Err(_) => Value::Null,
            };
            let observed_record = match record.is_null() {
                true => Value::Null,
                false => record["expected"]["template"].clone(),
            };
            (observed != expected || observed_record != recorded).then(|| {
                format!(
                    "row {label}:\n  left:  {observed:?} {observed_record}\n  right: \
                     {expected:?} {recorded}"
                )
            })
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));

    // A spawn sealed once and then refused keeps no record: the refusal
    // clears the earlier one rather than leaving it to be handed over.
    let (mut spawn, sealed) = sealing(&bundle, "work", 0, site);
    let mut expected = spawn.record.clone().map(|record| record.expected).unwrap();
    expected.template = TemplateExpectation::None;
    let resealed = spawn.seal(expected);
    assert_eq!(
        json!([
            format!("{sealed:?}"),
            format!("{resealed:?}"),
            spawn.launch_record()
        ]),
        json!([
            format!("{:?}", Ok::<(), String>(())),
            format!("{:?}", contradicted()),
            Value::Null
        ])
    );
}

/// What compiling a solo bundle whose inline `driver` seat pins the shipped
/// model and effort, authors `written` behind them and declares
/// `tools.allow: ["cargo"]` said, on the shipped adapters, in a realm that
/// grants nothing.
fn inline_typed_allow_beside(driver: &str, written: &[&str]) -> String {
    let operator = Operator::new();
    let mut authored = vec![
        "{brokkr}",
        "driver",
        driver,
        "--",
        "--model",
        "claude-opus-5-5",
        "--effort",
        "high",
    ];
    authored.extend(written);
    one_inline_seat(&operator, &authored);
    typed_allow(&operator, json!(["cargo"]));
    let context = CapabilityContext::no_grants("private", operator.root());
    match solo_bundle(&operator, &workspace().join("adapters"), &context) {
        Ok(_) => "compiled".to_string(),
        Err(refusal) => refusal,
    }
}

/// Rebuild units 5b-fix and 5b-fix2 (chief S1): the typed allow is never
/// lowered beside a permission mode or an additional directory its author
/// wrote, which could approve tools or reach files the engine's list does
/// not name. On the shipped adapters, the seats earlier units admitted —
/// `tools.allow` with an authored `bypassPermissions` or `--add-dir` beside
/// it — refuse at compile for both drivers in every spelling the chief
/// probed and the repeated and variadic ones, naming the option and its
/// position and never the mode or the directory. Rebuild unit 5c: with the
/// engine now emitting the adapter's `acceptEdits` template, an authored
/// `acceptEdits` — the template's own bytes — still refuses; the template is
/// the engine's because the adapter supplied it, never because a recipe's
/// text matches it.
#[test]
fn an_inline_typed_allow_beside_an_authored_capability_option_refuses_the_compile() {
    let carries = |canonical: &str, at: usize, kind: &str| {
        format!(
            "bundle: seat 'work' declares 'tools.allow' while its authored command carries \
             '{canonical}' (argument {at}), {kind}; the engine composes the typed list as its \
             own contribution and a recipe authors no capability-bearing option beside it, so the \
             site is refused rather than reconciled (operator ruling 1 of 2026-09-23; decision \
             0065 slice one, design D5.3)"
        )
    };
    let added = "an additional directory, which grants file access";
    for driver in ["claude", "lanetally"] {
        for (written, canonical, at, kind) in [
            (
                &["--permission-mode", "bypassPermissions"][..],
                "--permission-mode",
                5,
                "a permission mode",
            ),
            (
                &["--permission-mode=bypassPermissions"][..],
                "--permission-mode",
                5,
                "a permission mode",
            ),
            (
                &["--permission-mode", "acceptEdits"][..],
                "--permission-mode",
                5,
                "a permission mode",
            ),
            (
                &["--permission-mode=acceptEdits"][..],
                "--permission-mode",
                5,
                "a permission mode",
            ),
            (&["--add-dir", "/SENTINEL"][..], "--add-dir", 5, added),
            (&["--add-dir=/SENTINEL"][..], "--add-dir", 5, added),
            (
                &["--add-dir", "/SENTINEL-a", "/SENTINEL-b"][..],
                "--add-dir",
                5,
                added,
            ),
            (
                &[
                    "--verbose",
                    "--add-dir=/SENTINEL-a",
                    "--add-dir",
                    "/SENTINEL-b",
                ][..],
                "--add-dir",
                6,
                added,
            ),
        ] {
            assert_eq!(
                inline_typed_allow_beside(driver, written),
                carries(canonical, at, kind),
                "{driver} {written:?}"
            );
        }
    }
}

/// Rebuild unit 5b-fix2 (chief S2): a grammar failure on the inline typed
/// path never echoes the authored token. The chief's 2048-character web
/// value, an unknown option in its `=` spelling, is refused on the shipped
/// adapters by position, a bounded label and the grammar's cause, and the
/// whole refusal is the same length whatever the value's length.
#[test]
fn an_inline_typed_allow_beside_an_unreadable_long_value_refuses_without_echoing_it() {
    let long = format!("--web-search={}", "S".repeat(2048));
    for driver in ["claude", "lanetally"] {
        let refusal = inline_typed_allow_beside(driver, &[&long]);
        assert_eq!(
            refusal,
            format!(
                "bundle: seat 'work' declares 'tools.allow' while its authored command cannot be \
                 read: the '{driver}' command grammar cannot place argument 5 (an option the \
                 '{driver}' grammar does not model), whose token is not echoed because it can \
                 carry a value: it names no option, or names one that has no equals-joined \
                 spelling. A control nobody can read is a control nobody can rule on, so it is \
                 refused rather than passed through (decision 0066 ruling 6; operator ruling 1 \
                 of 2026-09-23)"
            )
        );
        assert_eq!(
            refusal.len(),
            inline_typed_allow_beside(driver, &["--web-search=S"]).len()
        );
    }
}

/// Rebuild unit 5b-fix3 (review R1 and R2): an authored effort beside a
/// typed allow stands only as one of the CLI reference's plain levels, by a
/// fixed classification rather than the adapter's declaration. Copies of
/// the shipped adapters that add `ultracode` and a 2055-character name to
/// their vocabulary still refuse `ultracode` and an undeclared name, for
/// both drivers and both spellings, under a fixed cause that is the same
/// length whatever the adapters declare; a plain level still compiles.
#[test]
fn an_inline_typed_allow_beside_an_unplain_effort_refuses_whatever_the_adapter_declares() {
    let long = "u".repeat(2055);
    let adapters = copied_adapters();
    for provider in ["claude", "lanetally"] {
        edit_adapter(adapters.path(), provider, |adapter| {
            adapter["efforts"] =
                json!(["low", "medium", "high", "xhigh", "max", "ultracode", long]);
        });
    }
    let beside = |adapters: &Path, driver: &str, written: &[&str]| {
        let operator = Operator::new();
        let mut authored = vec![
            "{brokkr}",
            "driver",
            driver,
            "--",
            "--model",
            "claude-opus-5-5",
        ];
        authored.extend(written);
        one_inline_seat(&operator, &authored);
        typed_allow(&operator, json!(["cargo"]));
        let context = CapabilityContext::no_grants("private", operator.root());
        match solo_bundle(&operator, adapters, &context) {
            Ok(_) => "compiled".to_string(),
            Err(refusal) => refusal,
        }
    };
    let refused = "bundle: seat 'work' declares 'tools.allow' while its authored command carries \
                   '--effort' (argument 3), an effort other than the reference's plain levels \
                   (low, medium, high, xhigh, max), which can turn on more than effort; the \
                   engine composes the typed list as its own contribution and a recipe authors \
                   no capability-bearing option beside it, so the site is refused rather than \
                   reconciled (operator ruling 1 of 2026-09-23; decision 0065 slice one, design \
                   D5.3)";
    for driver in ["claude", "lanetally"] {
        for written in [
            &["--effort", "ultracode"][..],
            &["--effort=ultracode"][..],
            &["--effort", "unknown"][..],
            &["--effort=unknown"][..],
        ] {
            let refusal = beside(adapters.path(), driver, written);
            assert_eq!(refusal, refused, "{driver} {written:?}");
            assert_eq!(
                refusal.chars().count(),
                beside(&workspace().join("adapters"), driver, written)
                    .chars()
                    .count(),
                "{driver} {written:?}"
            );
        }
    }
    assert_eq!(
        beside(adapters.path(), "claude", &["--effort=max"]),
        "compiled"
    );
}

/// Rebuild unit 5b at a LaneTally seat, which shares Claude's composition
/// path: its typed allow is lowered onto LaneTally's own tool permissions
/// and reaches the spawn and its sealed record as the engine's `local`
/// segment, behind LaneTally's own declared permission template as the
/// engine's `template` segment (rebuild unit 5c). The shipped LaneTally
/// inventory is unmeasured, which refuses every LaneTally seat on its own
/// terms, so this fixture copies Claude's measured native declarations into
/// it — the lowering is under test, not the wrapper's confinement.
#[test]
fn an_inline_lanetally_seats_typed_allow_reaches_its_spawn_as_the_engines_local_limits() {
    let operator = Operator::new();
    let adapters = copied_adapters();
    let claude: Value =
        serde_json::from_slice(&std::fs::read(adapters.path().join("claude.json")).unwrap())
            .unwrap();
    edit_adapter(adapters.path(), "lanetally", |adapter| {
        adapter["native_capabilities"] = claude["native_capabilities"].clone();
    });
    one_inline_seat(
        &operator,
        &[
            "{brokkr}",
            "driver",
            "lanetally",
            "--",
            "--model",
            "claude-opus-5-5",
            "--effort",
            "high",
        ],
    );
    typed_allow(&operator, json!(["git", "gh-pr-view"]));
    let context = CapabilityContext::no_grants("private", operator.root());
    let bundle = solo_bundle(&operator, adapters.path(), &context).unwrap();
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let (spawn, input) = sealed(&bundle, "work", 0);
    let record = &input["launch_record"];
    assert_eq!(
        json!({
            "spawn": spawn.argv,
            "segments": record["segments"],
            "local": record["expected"]["local"],
            "template": record["expected"]["template"],
        }),
        json!({
            "spawn": [&exe, "driver", "lanetally", "--", "--model", "claude-opus-5-5",
                      "--effort", "high", "--permission-mode", "acceptEdits",
                      "--allowedTools", "Bash(git:*),Bash(gh pr view:*)"],
            "segments": [
                {"origin": "authored",
                 "argv": ["--model", "claude-opus-5-5", "--effort", "high"]},
                {"origin": "template", "argv": ["--permission-mode", "acceptEdits"]},
                {"origin": "local", "argv": ["--allowedTools", "Bash(git:*),Bash(gh pr view:*)"]},
            ],
            "local": {"allow": {"kind": "listed", "names": ["git", "gh-pr-view"]},
                      "sandbox": {"kind": "unspecified"},
                      "application": {"kind": "direct",
                                      "limits": ["Bash(git:*)", "Bash(gh pr view:*)"]}},
            "template": {"kind": "declared", "argv": ["--permission-mode", "acceptEdits"]},
        })
    );
}

/// Rebuild unit 5b-fix (finding C1): the same LaneTally seat through the
/// driver's SERVING branch — `brokkr driver lanetally`, which composes the
/// wrapper's command under LaneTally's own shape — read off the command
/// the wrapper was actually spawned with. The whole ordered argv: the
/// stream shape, the authored pins, the adapter's permission template
/// (rebuild unit 5c), the engine's lowered list and the native OFF, and
/// nothing else. Provider-free: the wrapper is a recording
/// shim, and the driver is this test binary re-entered as
/// [`lanetally_serving_child`], because the serving branch is reached only
/// through the driver's own stdin protocol.
#[cfg(unix)]
#[test]
fn an_inline_lanetally_seats_typed_allow_reaches_the_wrappers_final_command_with_native_off() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    let operator = Operator::new();
    let adapters = copied_adapters();
    let claude: Value =
        serde_json::from_slice(&std::fs::read(adapters.path().join("claude.json")).unwrap())
            .unwrap();
    edit_adapter(adapters.path(), "lanetally", |adapter| {
        adapter["native_capabilities"] = claude["native_capabilities"].clone();
    });
    one_inline_seat(
        &operator,
        &[
            "{brokkr}",
            "driver",
            "lanetally",
            "--",
            "--model",
            "claude-opus-5-5",
            "--effort",
            "high",
        ],
    );
    typed_allow(&operator, json!(["git", "gh-pr-view"]));
    let context = CapabilityContext::no_grants("private", operator.root());
    let bundle = solo_bundle(&operator, adapters.path(), &context).unwrap();
    let (spawn, sealed_input) = sealed(&bundle, "work", 0);
    let outcome = &bundle.sites["work"].capabilities.as_ref().unwrap().outcomes[0];

    // The wrapper: answers the version probe, records any other argv.
    let root = operator.root();
    let recorded = root.join("wrapper-argv.txt");
    let staged = root.join("claude-lanetally.staged");
    std::fs::write(
        &staged,
        [
            "#!/bin/sh\n",
            "case \"$1\" in --version) printf '2.1.266 (Claude Code)\\n'; exit 0;; esac\n",
            "printf '%s\\n' \"$0\" \"$@\" > '",
            recorded.to_str().unwrap(),
            "'\n",
        ]
        .concat(),
    )
    .unwrap();
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).unwrap();
    let wrapper = root.join("claude-lanetally");
    std::fs::rename(&staged, &wrapper).unwrap();

    // What the engine hands the driver: its extras, and the input.
    let extra: Vec<String> =
        spawn.argv[spawn.argv.iter().position(|part| part == "--").unwrap() + 1..].to_vec();
    std::fs::create_dir_all(root.join("work")).unwrap();
    let input = json!({
        "feature": "serving", "phase": "work", "seat": "work",
        "role_path": root.join("solo/roles/role.md"), "role_text": "# role\n",
        "workdir": root.join("work"),
        "result_path": root.join("work/result.json"),
        "allowed_results": ["complete"], "context": {},
        "native_controls": outcome.controls(),
        "launch_arguments": spawn.launch_arguments(),
        "launch_record": sealed_input["launch_record"],
    });
    let home = root.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "lanetally_serving_child",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(SERVE_LANETALLY, serde_json::to_string(&extra).unwrap())
        .env("BROKKR_LANETALLY_BIN", &wrapper)
        .env_remove("FORGE_LANETALLY_BIN")
        .env("HOME", &home)
        .env("PATH", "/usr/bin:/bin")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for message in [
        json!({"proto": "forge-driver/v1", "msg_id": "m1", "type": "hello",
               "engine_version": "test"}),
        json!({"proto": "forge-driver/v1", "msg_id": "m2", "type": "start",
               "effect_id": "fx", "attempt_id": "a1", "seat": "work", "input": input}),
        json!({"proto": "forge-driver/v1", "msg_id": "m3", "type": "shutdown"}),
    ] {
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    let said = String::from_utf8_lossy(&out.stdout).into_owned();
    let spawned = std::fs::read_to_string(&recorded)
        .unwrap_or_else(|_| panic!("the wrapper was never spawned; the driver said: {said}"));
    assert_eq!(
        spawned.lines().collect::<Vec<_>>(),
        [
            wrapper.to_str().unwrap(),
            "-p",
            "--output-format",
            "stream-json",
            "--verbose",
            "--model",
            "claude-opus-5-5",
            "--effort",
            "high",
            "--permission-mode",
            "acceptEdits",
            "--allowedTools",
            "Bash(git:*),Bash(gh pr view:*)",
            "--disallowedTools",
            "WebFetch,WebSearch",
        ],
        "the driver said: {said}"
    );
}

/// The variable that turns [`lanetally_serving_child`] into the LaneTally
/// driver, carrying the extras the engine hands it as a JSON array.
const SERVE_LANETALLY: &str = "BROKKR_TEST_SERVE_LANETALLY";

/// Not a test of its own: the driver half of the serving test above, run
/// only when that test re-enters this binary with [`SERVE_LANETALLY`] set.
/// It serves the driver protocol on this process's stdin and stdout, as
/// `brokkr driver lanetally -- <extras>` does, then exits before the test
/// harness can report on it.
#[test]
fn lanetally_serving_child() {
    let Ok(extra) = std::env::var(SERVE_LANETALLY) else {
        return;
    };
    let extra: Vec<String> = serde_json::from_str(&extra).unwrap();
    brokkr_protocol::adapters::serve(brokkr_protocol::adapters::AdapterKind::Lanetally, extra)
        .unwrap();
    std::process::exit(0);
}

/// Declare `allow` as the solo work seat's typed local list.
fn typed_allow(operator: &Operator, allow: Value) {
    let path = operator.root().join("solo/bundle.json");
    let mut bundle: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    bundle["seats"]["work"]["tools"] = json!({"allow": allow});
    write(operator.root(), "solo/bundle.json", &bundle);
}

/// Ruling 5's launch on the RESUME argv, from compiled seats rather than a
/// hand-written plan: an inline and an agent-backed unboxed Codex work
/// seat, each way round, offered the session they opened. What comes back
/// is an actual `exec resume` — the offered thread, the stdin positional,
/// the re-imposed sandbox class and effort — and it carries the OFF pair
/// exactly when the seat does not hold search, placed before the two
/// positionals. A cold fallback would have no `resume` in it and fails
/// here; it is never counted as this proof. The assessment is the shipped
/// adapter's, so the eligibility being exercised is production's.
///
/// Composition evidence only: whether a RESUMED codex session honours the
/// switch is unmeasured and owed to the controller.
#[cfg(unix)]
#[test]
fn an_eligible_rejoin_of_a_compiled_codex_seat_carries_the_control_either_way_round() {
    let operator = Operator::new();
    let shim = codex_reporting(operator.root(), "0.154.0");
    let shim_path = shim.to_str().unwrap().to_string();
    for (case, context, denied) in [
        (
            "denied",
            CapabilityContext::no_grants("private", operator.root()),
            true,
        ),
        (
            "held",
            operator.context(json!({"web-search": {"dialect": "codex-native-search"}})),
            false,
        ),
    ] {
        let bundle = operator
            .compile(
                &context,
                Boundary::Harness,
                Some(json!({"web-search": "wants"})),
                None,
            )
            .unwrap();
        for label in ["inline", "agent"] {
            let argv = rejoin(&bundle, label, &shim);
            let mut expected = vec![
                shim_path.clone(),
                "exec".into(),
                "resume".into(),
                "--json".into(),
                "-c".into(),
                "sandbox_mode=\"workspace-write\"".into(),
                "-c".into(),
                "model_reasoning_effort=\"high\"".into(),
                "--model".into(),
                "gpt-6-astra".into(),
            ];
            if denied {
                expected.extend(OFF.map(String::from));
            }
            expected.extend([THREAD.to_string(), "-".to_string()]);
            assert_eq!(argv, expected, "{case} {label}: the whole resumed argv");
            assert_eq!(off_pairs(&argv), usize::from(denied), "{case} {label}");
        }
    }
}

/// An authored control that reaches the capability is refused at compile,
/// naming the seat — `--search` and the OFF pair itself alike — while an
/// unrelated `-c` (the boxed seat's own MCP configuration) is no conflict.
#[test]
fn an_authored_search_control_is_refused_at_compile_naming_the_seat() {
    let operator = Operator::new();
    let context = CapabilityContext::no_grants("private", operator.root());
    // Second council H1: all five spellings of the one config assignment,
    // the ATTACHED one included, earn the same realm-only refusal, because
    // the grammar parses them into the same assignment before anything
    // judges it.
    for (authored, written) in [
        (vec!["--search"], "--search"),
        (vec!["-c", "web_search=\"disabled\""], "-c web_search"),
        (vec!["-cweb_search=\"disabled\""], "-c web_search"),
        (vec!["-c=web_search=\"disabled\""], "-c web_search"),
        (
            vec!["--config", "web_search=\"disabled\""],
            "--config web_search",
        ),
        (
            vec!["--config=web_search=\"disabled\""],
            "--config web_search",
        ),
    ] {
        // The sound bundle compiles; then one authored control is added.
        operator
            .compile(&context, Boundary::Namespace, None, None)
            .unwrap();
        let mut bundle: Value = serde_json::from_slice(
            &std::fs::read(operator.root().join("bundle/bundle.json")).unwrap(),
        )
        .unwrap();
        let command = bundle["seats"]["inline"]["driver"]["command"]
            .as_array_mut()
            .unwrap();
        command.extend(authored.iter().map(|part| json!(part)));
        write(operator.root(), "bundle/bundle.json", &bundle);
        let refusal = Bundle::compile_with_capabilities(
            &operator.root().join("bundle"),
            &operator.root().join("agents"),
            &workspace().join("adapters"),
            Some("private"),
            None,
            Boundary::Namespace,
            &context,
        )
        .unwrap_err()
        .to_string();
        assert_eq!(
            refusal,
            format!(
                "bundle: seat 'inline' (office 'inline') in realm 'private': its \
                 arguments carry '{written}', which controls native capability 'web-search' of \
                 provider 'codex'. Only the realm grants a capability, and the engine composes \
                 the one control the grant resolves to; request 'web-search' by name under \
                 'capabilities' instead (decision 0065 rulings 3 and 4)"
            )
        );
    }
    // An option `codex exec` does not have never reaches the guard at
    // all: the grammar refuses it first, at the compiler, naming the
    // token (decision 0066 ruling 6).
    operator
        .compile(&context, Boundary::Namespace, None, None)
        .unwrap();
    let mut bundle: Value =
        serde_json::from_slice(&std::fs::read(operator.root().join("bundle/bundle.json")).unwrap())
            .unwrap();
    bundle["seats"]["inline"]["driver"]["command"]
        .as_array_mut()
        .unwrap()
        .extend([json!("--enable"), json!("web_search_request")]);
    write(operator.root(), "bundle/bundle.json", &bundle);
    assert_eq!(
        Bundle::compile_with_capabilities(
            &operator.root().join("bundle"),
            &operator.root().join("agents"),
            &workspace().join("adapters"),
            Some("private"),
            None,
            Boundary::Namespace,
            &context,
        )
        .unwrap_err()
        .to_string(),
        "bundle: seat 'inline' (office 'inline') in realm 'private': its arguments do not \
         parse: the 'codex' command grammar cannot place argument 7 ('--enable'): it names no \
         option. A harness brokkr launches is parsed against a model of its options, and a \
         token that grammar cannot place is refused rather than passed through, because a \
         control nobody can read is a control nobody can rule on (decision 0066 ruling 6)"
    );
}

/// A requirement the realm does not grant refuses compilation naming the
/// seat, the office, the capability and the realm — at every executable
/// form, in the same voice.
#[test]
fn an_ungranted_requirement_refuses_compilation_at_every_site_form() {
    let operator = Operator::new();
    let nothing = operator.context(json!({}));
    assert_eq!(
        operator
            .compile(
                &nothing,
                Boundary::Namespace,
                Some(json!({"web-search": "requires"})),
                None
            )
            .unwrap_err(),
        "bundle: seat 'boxed' (office 'boxed') in realm 'private': requires capability \
         'web-search' but the realm does not grant it to this office"
    );
    // A seat may not add to its office's asks, and a container is no site.
    assert_eq!(
        operator
            .compile(
                &nothing,
                Boundary::Namespace,
                None,
                Some(json!({"web-fetch": "wants"}))
            )
            .unwrap_err(),
        "bundle: seat 'agent' (office 'searcher') adds capability 'web-fetch', which \
         its office does not ask for; a seat may subtract from its office's asks and never add \
         to them"
    );
    // An MCP grant refuses the whole compile, asked for or not.
    write(
        operator.root(),
        "capabilities/library-docs.json",
        &json!({"name": "library-docs", "classes": ["reads", "egress"]}),
    );
    write(
        operator.root(),
        "dialects/tools/docs-mcp.json",
        &json!({"schema": "brokkr.tool-dialect/v1", "name": "docs-mcp",
                "serves": "library-docs", "kind": "mcp",
                "connection": {"url": "https://docs.invalid/mcp"}, "version": "1.0.0",
                "secrets": [], "tools": ["read"],
                "sends": {"description": "a library name", "seat_composed": true}}),
    );
    assert_eq!(
        operator
            .compile(
                &operator.context(json!({"library-docs": {"dialect": "docs-mcp", "offices": []}})),
                Boundary::Namespace,
                None,
                None
            )
            .unwrap_err(),
        "bundle: realm 'private' grants capability 'library-docs' through dialect \
         'docs-mcp' of kind 'mcp', whose broker support is not implemented until decision 0065 \
         slice two"
    );
}

/// Ruling 8: the grant is part of the bundle's identity. Identical inputs
/// give one digest; each authority axis, changed alone, gives another —
/// including a grant no seat uses and a definition whose ask was dropped.
/// And what is written is the contract it claims: run-manifest/v11.
#[test]
fn every_authority_axis_moves_the_manifest_digest_and_identical_inputs_do_not() {
    let operator = Operator::new();
    let schema: Value = serde_json::from_slice(
        &std::fs::read(workspace().join("contracts/run-manifest.v11.schema.json")).unwrap(),
    )
    .unwrap();
    let v11 = jsonschema::draft7::new(&schema).unwrap();
    let wants = Some(json!({"web-search": "wants"}));
    let digest = |context: &CapabilityContext, asks: Option<Value>| {
        let bundle = operator
            .compile(context, Boundary::Namespace, asks, Some(json!({})))
            .unwrap();
        assert!(v11.is_valid(&bundle.manifest), "{}", bundle.manifest);
        bundle.manifest_digest()
    };
    let granted = operator.context(json!({"web-search": {"dialect": "codex-native-search"}}));
    let base = digest(&granted, wants.clone());
    assert_eq!(
        base,
        digest(&granted, wants.clone()),
        "identical inputs, one digest"
    );

    let mut seen = vec![base.clone()];
    for (axis, capabilities) in [
        ("no grant", json!({})),
        (
            "office scope",
            json!({"web-search": {"dialect": "codex-native-search",
                                               "offices": ["inline"]}}),
        ),
        (
            "tools",
            json!({"web-search": {"dialect": "codex-native-search", "tools": []}}),
        ),
        (
            "dialect selection",
            json!({"web-search": {"dialect": "codex-search-second"}}),
        ),
    ] {
        let moved = digest(&operator.context(capabilities), wants.clone());
        assert!(!seen.contains(&moved), "{axis} did not move the digest");
        seen.push(moved);
    }
    // A grant NO seat asks for is pinned all the same, and moves with it.
    let unused = digest(&granted, None);
    let unused_scoped = digest(
        &operator.context(json!({"web-search": {"dialect": "codex-native-search",
                                                "offices": ["nobody"]}})),
        None,
    );
    assert_ne!(unused, unused_scoped, "an unused grant is still authority");
    // The serving dialect's bytes, and the definition's, authored changes
    // and nothing else: whitespace is enough, because the pin is raw.
    for file in [
        "dialects/tools/codex-native-search.json",
        "capabilities/web-search.json",
    ] {
        let path = operator.root().join(file);
        let original = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, format!("{original}\n")).unwrap();
        assert_ne!(
            digest(&granted, wants.clone()),
            base,
            "{file} bytes moved nothing"
        );
        std::fs::write(&path, &original).unwrap();
        assert_eq!(digest(&granted, wants.clone()), base, "{file} restored");
    }
    // A consulted definition is pinned even where its ask was DROPPED and
    // no dialect exists: the want below is lost in a realm granting nothing.
    let nothing = operator.context(json!({}));
    let dropped = digest(&nothing, wants.clone());
    let path = operator.root().join("capabilities/web-search.json");
    let original = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{original}\n")).unwrap();
    assert_ne!(digest(&nothing, wants.clone()), dropped);
    // An UNCONSULTED definition is no second source of identity.
    std::fs::write(&path, &original).unwrap();
    write(
        operator.root(),
        "capabilities/unrelated.json",
        &json!({"name": "unrelated", "classes": ["writes"]}),
    );
    assert_eq!(digest(&nothing, wants), dropped);
}

/// What the `capabilities` section is made of, and what it is not (design
/// D7). A RECIPE cannot define or grant: a definition and a dialect copied
/// into the bundle's own directory satisfy nothing, because the operator's
/// directory is the only place either is read. And the section pins
/// authority by relative source and digest — no host path, no expanded
/// argv, no temp directory — so the same authority compiles to the same
/// identity on any machine.
#[test]
fn a_recipe_defines_nothing_and_the_pinned_section_names_no_host_path_or_argv() {
    let operator = Operator::new();
    let wants = Some(json!({"web-search": "wants"}));
    let granted = operator.context(json!({"web-search": {"dialect": "codex-native-search"}}));

    // The operator's directory holds NO definition; the recipe brings its
    // own copy of both files. The ask is still undefined.
    let elsewhere = tempfile::tempdir().unwrap();
    for file in [
        "capabilities/web-search.json",
        "dialects/tools/codex-native-search.json",
    ] {
        let body: Value =
            serde_json::from_slice(&std::fs::read(workspace().join(file)).unwrap()).unwrap();
        write(&operator.root().join("bundle"), file, &body);
    }
    // The LIBRARY is judged first and whole (finding M3): every loaded
    // agent that asks, in library order, before any seat is resolved.
    let bare = CapabilityContext::no_grants("private", elsewhere.path());
    let undefined = |agent: &str| {
        format!(
            "agent '{agent}': capability 'web-search' has no abstract definition at \
             'capabilities/web-search.json' in the operator configuration; declare its classes \
             before requesting it"
        )
    };
    assert_eq!(
        operator
            .compile(&bare, Boundary::Namespace, wants.clone(), None)
            .unwrap_err(),
        format!(
            "bundle: {}; {}",
            undefined("fallback"),
            undefined("searcher")
        )
    );
    // Nor can the recipe's copy stand in for a granted dialect.
    let mut borrowed = granted.clone();
    borrowed.root = elsewhere.path().to_path_buf();
    assert_eq!(
        operator
            .compile(&borrowed, Boundary::Namespace, wants.clone(), None)
            .unwrap_err(),
        "bundle: realm 'private': capability 'web-search' has no abstract definition at \
         'capabilities/web-search.json' in the operator configuration; declare its classes \
         before granting it"
    );

    // Under the operator's own directory it compiles, holding the grant.
    let bundle = operator
        .compile(&granted, Boundary::Namespace, wants, None)
        .unwrap();
    let section = &bundle.manifest["capabilities"];
    assert_eq!(
        section["definitions"]["web-search"]["source"],
        "capabilities/web-search.json"
    );
    assert_eq!(
        section["dialects"]["codex-native-search"]["source"],
        "dialects/tools/codex-native-search.json"
    );
    // A held site records the plan by native KEY; the argv that plan
    // expands to belongs to the launch and is not in the manifest.
    let held = &section["sites"]["inline"]["candidates"][0];
    assert_eq!(held["held"]["web-search"]["dialect"], "codex-native-search");
    assert_eq!(held["native"]["on"], json!(["web-search"]));
    let denied = &section["sites"]["chain"]["candidates"][0]["native"];
    assert_eq!(denied["off"], json!(["web-fetch", "web-search"]));
    let text = section.to_string();
    // Both spellings of every host root: the loaders canonicalise what they
    // are given, and on macOS a temporary directory's canonical name
    // (`/private/var/…`) is not its lexical one — a leak of the canonical
    // path must not slip past a lexical needle.
    let canonical = |path: &Path| path.canonicalize().unwrap().to_str().unwrap().to_string();
    for absent in [
        operator.root().to_str().unwrap(),
        elsewhere.path().to_str().unwrap(),
        workspace().to_str().unwrap(),
        std::env::temp_dir().to_str().unwrap(),
        canonical(operator.root()).as_str(),
        canonical(elsewhere.path()).as_str(),
        canonical(&workspace()).as_str(),
        canonical(&std::env::temp_dir()).as_str(),
        "web_search=",
        "--disallowedTools",
        "--model",
        "{brokkr}",
    ] {
        assert!(!text.contains(absent), "the section carries '{absent}'");
    }
}

/// Ruling 8, one axis at a time and apart from the rest: a RESTRICTION's
/// value is identity even where it is inactive — Codex declares no
/// restriction transport, so the want is dropped whole (CQ1) and the
/// restriction still rides the pinned grant, authored order included.
#[test]
fn a_restriction_value_moves_the_manifest_digest_even_where_it_is_inactive() {
    let operator = Operator::new();
    let mut hosts: Value = serde_json::from_slice(
        &std::fs::read(
            operator
                .root()
                .join("dialects/tools/codex-native-search.json"),
        )
        .unwrap(),
    )
    .unwrap();
    hosts["name"] = json!("codex-search-hosts");
    hosts["restrictions"] = json!({"type": "object", "additionalProperties": false,
        "properties": {"allow": {"type": "object", "additionalProperties": false,
            "properties": {"hosts": {"type": "array", "items": {"type": "string"}}}}}});
    write(
        operator.root(),
        "dialects/tools/codex-search-hosts.json",
        &hosts,
    );
    let compiled = |allow: Option<Value>| {
        let mut grant = json!({"dialect": "codex-search-hosts"});
        if let Some(allow) = allow {
            grant["allow"] = json!({"hosts": allow});
        }
        operator
            .compile(
                &operator.context(json!({"web-search": grant})),
                Boundary::Namespace,
                Some(json!({"web-search": "wants"})),
                Some(json!({})),
            )
            .unwrap()
    };
    let digests: Vec<String> = [
        None,
        Some(json!(["a.example", "b.example"])),
        Some(json!(["b.example", "a.example"])),
        Some(json!(["a.example"])),
    ]
    .into_iter()
    .map(|allow| compiled(allow).manifest_digest())
    .collect();
    for (index, digest) in digests.iter().enumerate() {
        assert!(
            !digests[..index].contains(digest),
            "restriction value {index} did not move the digest: {digests:?}"
        );
    }
    // Inactive, and pinned all the same: the want was dropped with the
    // native search OFF, and the grant still carries what was written.
    let restricted = compiled(Some(json!(["a.example"])));
    assert_eq!(
        restricted.manifest["capabilities"]["grants"]["web-search"],
        json!({"dialect": "codex-search-hosts", "allow": {"hosts": ["a.example"]}})
    );
    assert_eq!(off_pairs(&launch(&restricted, "inline", 0)), 1);
    assert_eq!(
        compiled(Some(json!(["a.example"]))).manifest_digest(),
        restricted.manifest_digest(),
        "identical restrictions, one digest"
    );
}

/// And the last axis: an adapter's NATIVE-CONTROL declaration. A
/// byte-identical copy of the shipped adapters compiles to the shipped
/// digest; one edited word in Codex's declaration — an evidence limitation,
/// nothing that changes an argv — moves it, because what a harness is
/// declared able to do is what every denial in the bundle rests on.
#[test]
fn a_native_control_declaration_moves_the_manifest_digest() {
    let operator = Operator::new();
    let copied = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(workspace().join("adapters")).unwrap() {
        let path = entry.unwrap().path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            std::fs::copy(&path, copied.path().join(path.file_name().unwrap())).unwrap();
        }
    }
    let context = operator.context(json!({}));
    let digest = |adapters: &Path| {
        operator
            .compile_against(adapters, &context, Boundary::Namespace, None, None)
            .unwrap()
            .manifest_digest()
    };
    let shipped = digest(&workspace().join("adapters"));
    assert_eq!(digest(copied.path()), shipped, "identical declarations");

    // Edited as TEXT, so the one word is the only byte that moves: a
    // re-serialised file would move the digest by its whitespace alone.
    let codex = copied.path().join("codex.json");
    let written = std::fs::read_to_string(&codex).unwrap();
    let measured = "codex-cli versions other than 0.154.0 were not measured";
    assert_eq!(written.matches(measured).count(), 1, "{measured}");
    std::fs::write(
        &codex,
        written.replace(
            measured,
            "codex-cli versions other than 0.154.0 were never measured",
        ),
    )
    .unwrap();
    assert_ne!(digest(copied.path()), shipped);
    std::fs::write(&codex, written).unwrap();
    assert_eq!(digest(copied.path()), shipped, "restored");
}

/// Ruling 4's other half, at the compiler and both ways round: were Codex
/// measured UNABLE to remove its search tool, a Codex seat could not be
/// seated in a realm that has not granted search — boxed or unboxed, and
/// whether the seat asks for nothing, wants it, or belongs to an office
/// that subtracted it. The box is no substitute for the switch: the power
/// is the provider's server-side tool, which no namespace contains. Held
/// through a grant, the same seat compiles. The refusal names the seat,
/// the realm, the capability, the provider, the reason and the evidence.
#[test]
fn a_codex_that_could_not_switch_search_off_is_unseatable_boxed_and_unboxed() {
    let operator = Operator::new();
    let copied = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(workspace().join("adapters")).unwrap() {
        let path = entry.unwrap().path();
        std::fs::copy(&path, copied.path().join(path.file_name().unwrap())).unwrap();
    }
    let codex = copied.path().join("codex.json");
    let mut declared: Value = serde_json::from_slice(&std::fs::read(&codex).unwrap()).unwrap();
    let native = &mut declared["native_capabilities"]["known"]["web-search"];
    native["off"] = json!({"unsupported": "a hypothetical CLI with no key that removes the tool"});
    native["evidence"]["source"] = json!("a hypothetical measurement");
    native["evidence"]["scope"] = json!("hypothetical-cli 9.9");
    std::fs::write(&codex, serde_json::to_vec_pretty(&declared).unwrap()).unwrap();

    let refusal = |seat: &str, office: &str| {
        format!(
            "bundle: seat '{seat}' (office '{office}') in realm 'private': provider 'codex' \
             cannot switch off its native capability 'web-search', which this seat does not hold \
             (a hypothetical CLI with no key that removes the tool; evidence: a hypothetical \
             measurement, scope: hypothetical-cli 9.9); an ungranted native capability that \
             cannot be disabled cannot be seated in this realm (decision 0065 ruling 4)"
        )
    };
    let nothing = operator.context(json!({}));
    for boundary in [Boundary::Namespace, Boundary::Harness] {
        // No ask, a want, and an office's ask subtracted by its seat: the
        // first Codex site the walk reaches refuses, every time.
        for (asks, seat) in [
            (None, None),
            (Some(json!({"web-search": "wants"})), None),
            (None, Some(json!({}))),
        ] {
            assert_eq!(
                operator
                    .compile_against(copied.path(), &nothing, boundary, asks, seat)
                    .unwrap_err(),
                refusal("agent", "searcher"),
                "{boundary:?}"
            );
        }
    }
    // A grant to ANOTHER office excuses nobody.
    let elsewhere = operator.context(json!({"web-search": {"dialect": "codex-native-search",
                                              "offices": ["nobody"]}}));
    assert_eq!(
        operator
            .compile_against(copied.path(), &elsewhere, Boundary::Harness, None, None)
            .unwrap_err(),
        refusal("agent", "searcher")
    );
    // Held by every Codex site, the very same declaration seats.
    let granted = operator.context(json!({"web-search": {"dialect": "codex-native-search"}}));
    let wants = Some(json!({"web-search": "wants"}));
    let bundle = operator
        .compile_against(copied.path(), &granted, Boundary::Harness, wants, None)
        .unwrap();
    for label in ["inline", "agent"] {
        assert_eq!(off_pairs(&launch(&bundle, label, 0)), 0, "{label}");
    }
}

/// Every shipped bundle, as it compiles in this repository's own realm —
/// which grants NOTHING. Every executable site of every form has an
/// outcome; no seat holds anything; every Codex candidate is composed with
/// the OFF pair and every Claude candidate with both native tools denied;
/// DSH, LaneTally and exec stay unmeasured and claim no denial; and the
/// dialect wrapper moved the wrapped verify seat's outcome with the rest of
/// its facts while the generated validator got an explicit empty one.
#[test]
fn every_site_of_every_shipped_bundle_holds_nothing_and_has_its_native_powers_denied() {
    let root = workspace();
    let mut dirs: Vec<PathBuf> = ["bundles", "recipes"]
        .iter()
        .flat_map(|parent| std::fs::read_dir(root.join(parent)).unwrap())
        .map(|entry| entry.unwrap().path())
        .filter(|dir| dir.join("bundle.json").is_file())
        .collect();
    dirs.sort();
    let (mut codex, mut claude, mut unmeasured, mut nested) = (0, 0, 0, 0);
    for dir in &dirs {
        let bundle = Bundle::compile_with(dir, &root.join("agents"), &root.join("adapters"))
            .unwrap_or_else(|error| panic!("{} must compile: {error}", dir.display()));
        assert_eq!(bundle.manifest["capabilities"]["grants"], json!({}));
        for (label, facts) in &bundle.sites {
            let site = facts
                .capabilities
                .as_ref()
                .unwrap_or_else(|| panic!("{}: {label} has no outcome", dir.display()));
            nested += usize::from(label.contains(':'));
            assert!(!site.outcomes.is_empty(), "{label}");
            for outcome in &site.outcomes {
                assert!(
                    outcome.held.is_empty(),
                    "{}: {label} holds something",
                    dir.display()
                );
                let controls = outcome.controls();
                match outcome.provider.as_str() {
                    "codex" => {
                        codex += 1;
                        assert_eq!(controls["argv"], json!(OFF), "{}: {label}", dir.display());
                    }
                    "claude" => {
                        claude += 1;
                        assert_eq!(
                            controls["selection"]["deny"],
                            json!(["WebFetch", "WebSearch"]),
                            "{}: {label}",
                            dir.display()
                        );
                        assert_eq!(controls["selection"]["include"], json!([]));
                    }
                    _ => {
                        unmeasured += 1;
                        assert_eq!(controls["inventory"], "unmeasured", "{label}");
                        // The seat is told so, rather than told nothing.
                        assert!(outcome.prompt()["native"].is_string(), "{label}");
                    }
                }
            }
        }
        if let Some(wrapped) = bundle.sites.get("verify:checks") {
            // The wrapper moved the facts, not the office's identity.
            assert_eq!(wrapped.capabilities.as_ref().unwrap().asks.office, "verify");
            let validator = bundle.sites["verify:dialect-verify"]
                .capabilities
                .as_ref()
                .unwrap();
            assert_eq!(validator.asks.office, "verify:dialect-verify");
            assert!(validator.asks.asks.is_empty());
            assert_eq!(validator.outcomes[0].provider, "exec");
        }
    }
    assert!(codex > 0 && claude > 0 && unmeasured > 0 && nested > 0);
    // The researcher is the one shipped office that asks, and in this
    // realm it loses both wants, visibly, on every link of its chain.
    let research = Bundle::compile_with(
        &root.join("recipes/research"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .unwrap();
    let site = research.sites["research"].capabilities.as_ref().unwrap();
    assert_eq!(site.asks.office, "researcher");
    for outcome in &site.outcomes {
        let lost: Vec<&str> = outcome
            .notices
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(lost, ["web-fetch", "web-search"]);
    }
    assert_eq!(
        research.manifest["agents"]["research"]["notices"][0]["message"],
        "seat 'research' (office 'researcher') in realm '<unmapped>': dropped wanted capability \
         'web-fetch' because the realm does not grant it to this office"
    );
    // Both consulted definitions are pinned beside the dropped asks.
    assert_eq!(
        research.manifest["capabilities"]["definitions"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["web-fetch", "web-search"]
    );
}

/// A panel member and a sequence step are sites exactly as a seat is: each
/// resolves its own asks under its own label, and a request written on the
/// CONTAINER — which executes nothing — is refused rather than dropped.
#[test]
fn a_panel_member_and_a_sequence_step_resolve_under_their_own_labels() {
    let operator = Operator::new();
    let site = |asks: Option<Value>| {
        let mut site = json!({"role": "roles/role.md", "driver": {"command": [
            "{brokkr}", "driver", "codex", "--", "--model", "gpt-6-astra", "--effort", "high",
            "--sandbox", "read-only"]}});
        if let Some(asks) = asks {
            site["capabilities"] = asks;
        }
        site
    };
    write(
        operator.root(),
        "bundle/policy.json",
        &json!({"phases": ["judges", "steps", "review", "done"], "initial": "judges",
            "terminal": ["done"], "rules": [
                {"id": "A", "from": "judges", "result": "pass", "next": "steps", "reason": "r"},
                {"id": "B", "from": "judges", "result": "fail", "next": "steps", "reason": "r"},
                {"id": "C", "from": "steps", "result": "complete", "next": "review", "reason": "r"},
                {"id": "D", "from": "review", "result": "clean", "next": "done", "reason": "r"}]}),
    );
    let wants = json!({"web-search": "wants"});
    let compile = |judges: Value| {
        let mut one = site(Some(wants.clone()));
        one["name"] = json!("one");
        one["results"] = json!(["complete"]);
        let mut two = site(None);
        two["name"] = json!("two");
        write(
            operator.root(),
            "bundle/bundle.json",
            &json!({"name": "forms", "policy": "policy.json", "seats": {
                "judges": judges,
                "steps": {"results": ["complete"], "sequence": [one, two]},
                "review": {"results": ["clean"], "role": "roles/role.md",
                           "driver": {"command": ["driver"]}}}}),
        );
        Bundle::compile_with_capabilities(
            &operator.root().join("bundle"),
            &operator.root().join("agents"),
            &workspace().join("adapters"),
            Some("private"),
            None,
            Boundary::Namespace,
            &operator.context(json!({"web-search": {"dialect": "codex-native-search",
                                                    "offices": ["judges:search"]}})),
        )
        .map_err(|error| error.to_string())
    };
    let panel = json!({"results": ["pass", "fail"], "aggregate": "unanimous-pass", "panel": {
        "search": site(Some(wants.clone())), "plain": site(None)}});
    let bundle = compile(panel.clone()).unwrap();
    let held = |label: &str| {
        let site = bundle.sites[label].capabilities.as_ref().unwrap();
        (site.asks.office.clone(), !site.outcomes[0].held.is_empty())
    };
    // The grant names ONE office: the member that asks and is named holds
    // it; the step that asks and is not named loses it with the reason.
    assert_eq!(held("judges:search"), ("judges:search".to_string(), true));
    assert_eq!(held("judges:plain"), ("judges:plain".to_string(), false));
    assert_eq!(held("steps:one"), ("steps:one".to_string(), false));
    assert_eq!(held("steps:two"), ("steps:two".to_string(), false));
    assert_eq!(
        bundle.sites["steps:one"]
            .capabilities
            .as_ref()
            .unwrap()
            .outcomes[0]
            .notices[0]
            .1,
        "seat 'steps:one' (office 'steps:one') in realm 'private': dropped wanted capability \
         'web-search' because the realm grants it only to offices [judges:search], not to \
         this office"
    );
    let step = &bundle.sites["steps:one"]
        .capabilities
        .as_ref()
        .unwrap()
        .outcomes[0];
    assert_eq!(step.controls()["argv"], json!(OFF));
    let member = &bundle.sites["judges:search"]
        .capabilities
        .as_ref()
        .unwrap()
        .outcomes[0];
    assert_eq!(member.controls()["argv"], json!([]));

    let mut container = panel;
    container["capabilities"] = wants.clone();
    assert_eq!(
        compile(container).unwrap_err(),
        "bundle: seat 'judges' declares 'capabilities' beside a panel, sequence or select; a \
         request belongs to the site that executes — the member, step or case body — because \
         that is the office the realm grants to (decision 0065 ruling 5)"
    );
}

/// One office — `scholar`, which REQUIRES `web-search` and WANTS
/// `web-fetch` — seated in a panel, a sequence, a select and an inherited
/// bundle. Wherever it sits, writing no map inherits the office's asks, a
/// map is an unchanged-strength subset whose omissions are subtracted
/// (a requirement included), `{}` subtracts everything, and the office
/// keeps its name under every execution label (ruling 5; design D2).
#[test]
fn an_office_is_inherited_subset_and_emptied_the_same_way_in_every_body() {
    let operator = Operator::new();
    let root = operator.root();
    write(
        root,
        "agents/scholar.json",
        &json!({
            "description": "an office that must search and may fetch",
            "charter": "charters/searcher.md",
            "models": ["astra"],
            "efforts": {"astra": "high"},
            "hands": {"kind": "workspace", "network": false, "binds": []},
            "capabilities": {"web-search": "requires", "web-fetch": "wants"},
        }),
    );
    std::fs::create_dir_all(root.join("nested/roles")).unwrap();
    std::fs::write(root.join("nested/roles/role.md"), "# role\n").unwrap();
    write(
        root,
        "nested/policy.json",
        &json!({"phases": ["judges", "steps", "pick", "review", "done"], "initial": "judges",
            "terminal": ["done"], "rules": [
                {"id": "A", "from": "judges", "result": "pass", "next": "steps", "reason": "r"},
                {"id": "B", "from": "judges", "result": "fail", "next": "steps", "reason": "r"},
                {"id": "C", "from": "steps", "result": "complete", "next": "pick", "reason": "r"},
                {"id": "D", "from": "pick", "result": "complete", "next": "review", "reason": "r"},
                {"id": "E", "from": "review", "result": "clean", "next": "done", "reason": "r"}]}),
    );
    let scholar = |written: Option<Value>| {
        let mut site = json!({"agent": "scholar"});
        if let Some(written) = written {
            site["capabilities"] = written;
        }
        site
    };
    let fetch_only = json!({"web-fetch": "wants"});
    let search_only = json!({"web-search": "requires"});
    let seats = |subset: Value, emptied: Value| {
        let mut step = scholar(Some(emptied));
        step["name"] = json!("none");
        step["results"] = json!(["complete"]);
        let mut last = scholar(None);
        last["name"] = json!("all");
        json!({
            "judges": {"results": ["pass", "fail"], "aggregate": "unanimous-pass", "panel": {
                "inherits": scholar(None), "subset": scholar(Some(subset))}},
            "steps": {"results": ["complete"], "sequence": [step, last]},
            "pick": {"results": ["complete"], "select": {"on": "strategy",
                "cases": {"engine": scholar(None)},
                "default": scholar(Some(search_only.clone()))}},
            "review": {"results": ["clean"], "role": "roles/role.md",
                       "driver": {"command": ["driver"]}},
        })
    };
    let compile_typed = |recipe: &str, offices: Option<Value>| {
        let mut grant = json!({"dialect": "codex-native-search"});
        if let Some(offices) = offices {
            grant["offices"] = offices;
        }
        Bundle::compile_with_capabilities(
            &root.join(recipe),
            &root.join("agents"),
            &workspace().join("adapters"),
            Some("private"),
            None,
            Boundary::Namespace,
            &operator.context(json!({"web-search": grant})),
        )
    };
    let compile = |recipe: &str, offices: Option<Value>| {
        compile_typed(recipe, offices).map_err(|error| error.to_string())
    };
    let nested = |subset: Value, emptied: Value| {
        write(
            root,
            "nested/bundle.json",
            &json!({"name": "nested", "policy": "policy.json", "seats": seats(subset, emptied)}),
        );
        compile("nested", None)
    };
    let bundle = nested(fetch_only.clone(), json!({})).unwrap();
    // (subtracted, held, launched with the OFF pair) for one site.
    let read = |bundle: &Bundle, label: &str| {
        let site = bundle.sites[label].capabilities.as_ref().unwrap();
        assert_eq!(site.asks.office, "scholar", "{label} keeps its office");
        let outcome = &site.outcomes[0];
        (
            site.asks.subtracted.join(","),
            outcome.held.keys().cloned().collect::<Vec<_>>().join(","),
            off_pairs(&launch(bundle, label, 0)),
        )
    };
    let all = [
        ("judges:inherits", ("", "web-search", 0)),
        ("judges:subset", ("web-search", "", 1)),
        ("steps:none", ("web-fetch,web-search", "", 1)),
        ("steps:all", ("", "web-search", 0)),
        ("pick:engine", ("", "web-search", 0)),
        ("pick:default", ("web-fetch", "web-search", 0)),
    ];
    for (label, (subtracted, held, off)) in all {
        assert_eq!(
            read(&bundle, label),
            (subtracted.to_string(), held.to_string(), off),
            "{label}"
        );
    }
    // A subtracted REQUIREMENT is not a refusal and not a drop: the seat
    // says why it does not hold it, and no notice is recorded for it.
    let subset = &bundle.sites["judges:subset"]
        .capabilities
        .as_ref()
        .unwrap()
        .outcomes[0];
    assert_eq!(
        subset.not_held["web-search"],
        "this seat subtracted it from its office's asks"
    );
    assert_eq!(
        subset.notices,
        [(
            "web-fetch".to_string(),
            "seat 'judges:subset' (office 'scholar') in realm 'private': dropped wanted \
             capability 'web-fetch' because the realm does not grant it to this office"
                .to_string()
        )]
    );
    assert!(bundle.sites["steps:none"]
        .capabilities
        .as_ref()
        .unwrap()
        .outcomes[0]
        .notices
        .is_empty());

    // A seat never adds and never re-rates, in a nested body as anywhere.
    assert_eq!(
        nested(fetch_only.clone(), json!({"library-docs": "wants"})).unwrap_err(),
        "bundle: seat 'steps:none' (office 'scholar') adds capability 'library-docs', which its \
         office does not ask for; a seat may subtract from its office's asks and never add to \
         them"
    );
    assert_eq!(
        nested(json!({"web-search": "wants"}), json!({})).unwrap_err(),
        "bundle: seat 'judges:subset' (office 'scholar') changes capability 'web-search' from \
         requires to wants; a seat may subtract from its office's asks and never change their \
         strength"
    );

    // INHERITED: a recipe that extends `nested` and changes nothing
    // resolves every site exactly as its base does.
    nested(fetch_only, json!({})).unwrap();
    write(
        root,
        "derived/bundle.json",
        &json!({"name": "derived", "extends": "nested"}),
    );
    let derived = compile("derived", None).unwrap();
    assert_eq!(
        derived.manifest["capabilities"]["sites"],
        bundle.manifest["capabilities"]["sites"]
    );
    // And an inherited REQUIREMENT the realm does not reach refuses the
    // derived recipe, naming the inherited seat — while the seats that
    // subtracted it would have compiled.
    // (The doubled prefix is how every failure on a composed bundle has
    // always read: the chain note wraps the displayed error.)
    assert_eq!(
        compile("derived", Some(json!(["nobody"]))).unwrap_err(),
        "bundle: bundle: seat 'judges:inherits' (office 'scholar') in realm 'private': requires \
         capability 'web-search' but the realm grants it only to offices [nobody], not to this \
         office (composed: derived -> nested)"
    );
    // The chain note keeps the refusal a CAPABILITY one, which is what lets
    // `brokkr resume` send it through the manifest-mismatch door.
    assert!(matches!(
        compile_typed("derived", Some(json!(["nobody"]))),
        Err(brokkr_runtime::bundle::CompileError::Capability(_))
    ));
    // A seat's own declaration fault is the bundle's, not authority's.
    assert!(matches!(
        {
            nested(json!({"web-search": "wants"}), json!({})).unwrap_err();
            compile_typed("nested", None)
        },
        Err(brokkr_runtime::bundle::CompileError::Invalid(_))
    ));
}

// ------------------------------------------------- decision 0066

/// A copy of the shipped adapters a test may break one file of.
fn copied_adapters() -> tempfile::TempDir {
    let copied = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(workspace().join("adapters")).unwrap() {
        let path = entry.unwrap().path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            std::fs::copy(&path, copied.path().join(path.file_name().unwrap())).unwrap();
        }
    }
    copied
}

fn edit_adapter(root: &Path, provider: &str, edit: impl FnOnce(&mut Value)) {
    let path = root.join(format!("{provider}.json"));
    let mut adapter: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    edit(&mut adapter);
    std::fs::write(path, serde_json::to_vec_pretty(&adapter).unwrap()).unwrap();
}

/// One work seat and nothing else that could open the adapters: an inline
/// model driver with no asks, under a bare custom review gate.
fn one_inline_seat(operator: &Operator, command: &[&str]) {
    write(
        operator.root(),
        "solo/policy.json",
        &json!({"phases": ["work", "review", "done"], "initial": "work", "terminal": ["done"],
            "rules": [
                {"id": "W", "from": "work", "result": "complete", "next": "review", "reason": "r"},
                {"id": "R", "from": "review", "result": "clean", "next": "done", "reason": "r"}]}),
    );
    std::fs::create_dir_all(operator.root().join("solo/roles")).unwrap();
    std::fs::write(operator.root().join("solo/roles/role.md"), "# role\n").unwrap();
    write(
        operator.root(),
        "solo/bundle.json",
        &json!({"name": "solo", "policy": "policy.json", "seats": {
            "work": {"results": ["complete"], "role": "roles/role.md",
                     "driver": {"command": command}},
            "review": {"results": ["clean"], "role": "roles/role.md",
                       "driver": {"command": ["driver"]}}}}),
    );
}

/// What compiling the solo bundle said, or the final argv of its work seat.
fn solo(operator: &Operator, adapters: &Path, context: &CapabilityContext) -> String {
    match solo_bundle(operator, adapters, context) {
        Ok(bundle) => match try_launch(&bundle, "work", 0) {
            Ok(argv) => format!("launched {argv:?}"),
            Err(refusal) => format!("compiled, and the driver said: {refusal}"),
        },
        Err(refusal) => refusal,
    }
}

fn solo_bundle(
    operator: &Operator,
    adapters: &Path,
    context: &CapabilityContext,
) -> Result<Bundle, String> {
    Bundle::compile_with_capabilities(
        &operator.root().join("solo"),
        &operator.root().join("agents"),
        adapters,
        Some(context.realm.as_str()).filter(|realm| *realm != "<unmapped>"),
        None,
        Boundary::Harness,
        context,
    )
    .map_err(|refusal| refusal.to_string())
}

const CODEX_SEAT: [&str; 10] = [
    "{brokkr}",
    "driver",
    "codex",
    "--",
    "--model",
    "gpt-6-astra",
    "--effort",
    "high",
    "--sandbox",
    "workspace-write",
];

/// Finding H1: DENIAL IS SOMETHING THE LAUNCH PROVES, NEVER SOMETHING
/// ABSENCE IMPLIES. An inline Codex work seat that asks for nothing, in a
/// realm that grants nothing, used to compile with NO search OFF whenever
/// the adapter data was absent, legacy, emptied or unreadable — an
/// unrelated broken adapter file was enough — because the load error was
/// swallowed and "nothing declared" read as "nothing to deny". Every such
/// state now refuses the seat, naming the site, the office, the realm, the
/// provider, the capability and the ORIGINAL cause; and with valid data the
/// same seat launches with the OFF pair, exactly once, last. The realm
/// contexts are the ones every map version reduces to at the compiler: no
/// map at all, a realm that grants nothing (what v1 to v5 mean, and v6 with
/// the field omitted), and v6 with an explicit empty map.
#[test]
fn a_known_native_power_with_no_valid_denial_refuses_the_seat() {
    let operator = Operator::new();
    one_inline_seat(&operator, &CODEX_SEAT);
    let contexts = [
        CapabilityContext::no_grants("<unmapped>", operator.root()),
        CapabilityContext::no_grants("private", operator.root()),
        operator.context(json!({})),
    ];
    let refusal = |realm: &str, provider: &str, capability: &str, cause: &str| {
        format!(
            "bundle: seat 'work' (office 'work') in realm '{realm}': provider '{provider}' is \
             known to carry native capability '{capability}', which this seat does not hold, \
             and no valid control denies it: {cause}. A known native power is launched only \
             with a delivered denial, never on what absence implies; repair the adapter data \
             (decision 0066 ruling 1)"
        )
    };
    let unloaded = |root: &Path| {
        format!(
            "the adapter data could not be loaded ({})",
            brokkr_runtime::Adapters::load(root).unwrap_err()
        )
    };
    let legacy = "its adapter declares its native capabilities unmeasured (the adapter declares \
                  no native_capabilities assessment)";

    for context in &contexts {
        let realm = context.realm.as_str();
        // The control: the shipped declaration delivers the denial.
        let sound = solo(&operator, &workspace().join("adapters"), context);
        assert!(sound.starts_with("launched "), "{realm}: {sound}");
        assert!(
            sound.ends_with(r#""-c", "web_search=\"disabled\""]"#),
            "{realm}: {sound}"
        );
        assert_eq!(sound.matches("web_search").count(), 1, "{realm}: {sound}");

        // No adapters root at all.
        let absent = operator.root().join("no-adapters-here");
        assert_eq!(
            solo(&operator, &absent, context),
            refusal(realm, "codex", "web-search", &unloaded(&absent))
        );
        // A root with no declaration for the provider.
        let others = copied_adapters();
        std::fs::remove_file(others.path().join("codex.json")).unwrap();
        assert_eq!(
            solo(&operator, others.path(), context),
            refusal(
                realm,
                "codex",
                "web-search",
                "no adapter declares provider 'codex'"
            )
        );
        // A declaration written before the ruling.
        let older = copied_adapters();
        edit_adapter(older.path(), "codex", |adapter| {
            adapter
                .as_object_mut()
                .unwrap()
                .remove("native_capabilities");
        });
        assert_eq!(
            solo(&operator, older.path(), context),
            refusal(realm, "codex", "web-search", legacy)
        );
        // An inventory declared unmeasured, and one emptied of the power.
        let unmeasured = copied_adapters();
        edit_adapter(unmeasured.path(), "codex", |adapter| {
            adapter["native_capabilities"] = json!({"unmeasured": "nobody looked"});
        });
        assert_eq!(
            solo(&operator, unmeasured.path(), context),
            refusal(
                realm,
                "codex",
                "web-search",
                "its adapter declares its native capabilities unmeasured (nobody looked)"
            )
        );
        let emptied = copied_adapters();
        edit_adapter(emptied.path(), "codex", |adapter| {
            adapter["native_capabilities"] = json!({"known": {}});
        });
        assert_eq!(
            solo(&operator, emptied.path(), context),
            refusal(
                realm,
                "codex",
                "web-search",
                "its adapter declares no native capability serving 'web-search'"
            )
        );
        // An OFF control nobody measured.
        let untried = copied_adapters();
        edit_adapter(untried.path(), "codex", |adapter| {
            adapter["native_capabilities"]["known"]["web-search"]["off"] =
                json!({"unmeasured": "nobody has tried"});
        });
        assert_eq!(
            solo(&operator, untried.path(), context),
            refusal(
                realm,
                "codex",
                "web-search",
                "its OFF control is unmeasured (nobody has tried)"
            )
        );
        // The provider's own file unreadable; then a SOUND one beside an
        // unrelated broken neighbour, which is all it took.
        let malformed = copied_adapters();
        std::fs::write(malformed.path().join("codex.json"), "{not json").unwrap();
        assert_eq!(
            solo(&operator, malformed.path(), context),
            refusal(realm, "codex", "web-search", &unloaded(malformed.path()))
        );
        let neighbour = copied_adapters();
        std::fs::write(neighbour.path().join("broken.json"), "{not json").unwrap();
        let said = solo(&operator, neighbour.path(), context);
        assert_eq!(
            said,
            refusal(realm, "codex", "web-search", &unloaded(neighbour.path()))
        );
        assert!(
            said.contains("broken.json"),
            "the original cause is named: {said}"
        );
    }

    // Claude's floor is both of its powers: a declaration that keeps one
    // and omits the other refuses for the one it omits.
    one_inline_seat(
        &operator,
        &[
            "{brokkr}",
            "driver",
            "claude",
            "--",
            "--model",
            "claude-opus-5",
            "--effort",
            "high",
        ],
    );
    let half = copied_adapters();
    edit_adapter(half.path(), "claude", |adapter| {
        adapter["native_capabilities"]["known"]
            .as_object_mut()
            .unwrap()
            .remove("web-fetch");
    });
    assert_eq!(
        solo(&operator, half.path(), &contexts[1]),
        refusal(
            "private",
            "claude",
            "web-fetch",
            "its adapter declares no native capability serving 'web-fetch'"
        )
    );
    // DSH inherits nobody's floor: its declared uncertainty still seats.
    one_inline_seat(
        &operator,
        &[
            "{brokkr}",
            "driver",
            "dsh",
            "--",
            "--model",
            "deepseek-v4-flash",
            "--effort",
            "high",
        ],
    );
    let seated = solo_bundle(&operator, &workspace().join("adapters"), &contexts[1]).unwrap();
    let outcome = &seated.sites["work"].capabilities.as_ref().unwrap().outcomes[0];
    assert_eq!(outcome.controls()["inventory"], "unmeasured");
    assert_eq!(outcome.controls()["harness"], "dsh");
}

/// Finding H1, agent-backed: the same law where the adapters were opened
/// for an agent. Legacy metadata loads cleanly there — it is the inventory
/// that reads unmeasured — and used to compile a Codex link with no OFF.
#[test]
fn an_agent_backed_link_on_legacy_adapter_data_refuses_too() {
    let operator = Operator::new();
    let older = copied_adapters();
    edit_adapter(older.path(), "codex", |adapter| {
        adapter
            .as_object_mut()
            .unwrap()
            .remove("native_capabilities");
    });
    assert_eq!(
        operator
            .compile_against(
                older.path(),
                &operator.context(json!({})),
                Boundary::Namespace,
                None,
                None
            )
            .unwrap_err(),
        "bundle: seat 'agent' (office 'searcher') in realm 'private': provider 'codex' is known \
         to carry native capability 'web-search', which this seat does not hold, and no valid \
         control denies it: its adapter declares its native capabilities unmeasured (the adapter \
         declares no native_capabilities assessment). A known native power is launched only \
         with a delivered denial, never on what absence implies; repair the adapter data \
         (decision 0066 ruling 1)"
    );
}

/// Finding H2, the council's reproduction at the compiler: an unboxed
/// inline seat that supplies a concrete server, under `grants: {}` and a
/// perfectly sound inventory. Refused by name, at every site form, with no
/// value copied — while the ENGINE's hands, in an agent-backed boxed seat
/// of the same realm, reach the final command exactly as they did.
#[test]
fn an_authored_capability_server_refuses_the_compile_and_the_engines_hands_still_launch() {
    let operator = Operator::new();
    let context = operator.context(json!({}));
    let adapters = workspace().join("adapters");
    let refusal = |site: &str, written: &str, provider: &str| {
        format!(
            "bundle: seat '{site}' (office '{site}') in realm 'private': its arguments carry \
             '{written}', which configures a capability server or admits a server's tools for \
             provider '{provider}'. A recipe's driver arguments are recipe data, and only the \
             realm grants a capability (decision 0065 ruling 3); the workspace hands are the \
             engine's own to compose and need no authored configuration (decision 0066 ruling 4)"
        )
    };
    fn codex(extra: &[&'static str]) -> Vec<&'static str> {
        [&CODEX_SEAT[..], extra].concat()
    }
    fn claude(extra: &[&'static str]) -> Vec<&'static str> {
        const SEAT: [&str; 8] = [
            "{brokkr}",
            "driver",
            "claude",
            "--",
            "--model",
            "claude-opus-5",
            "--effort",
            "high",
        ];
        [&SEAT[..], extra].concat()
    }
    for (command, written, provider) in [
        (
            codex(&[
                "-c",
                "mcp_servers.ungranted.command=\"npx\"",
                "-c",
                "mcp_servers.ungranted.args=[\"fetch-mcp\"]",
            ]),
            "-c mcp_servers",
            "codex",
        ),
        (
            codex(&["--config=mcp_servers={ungranted={command=\"npx\"}}"]),
            "--config mcp_servers",
            "codex",
        ),
        // Second council H1, verbatim: the ATTACHED spelling the first
        // repair's scanner passed straight through into the final
        // `codex exec` command earns the same realm-only refusal.
        (
            codex(&["-cmcp_servers.ungranted.command=\"/bin/false\""]),
            "-c mcp_servers",
            "codex",
        ),
        (
            codex(&["-c=mcp_servers.ungranted.command=\"/bin/false\""]),
            "-c mcp_servers",
            "codex",
        ),
        (
            codex(&["--config", "mcp_servers.ungranted.command=\"/bin/false\""]),
            "--config mcp_servers",
            "codex",
        ),
        // Second council H2: the plugin channel, and a later admission
        // value, at the compiler as at the launch.
        (
            claude(&["--plugin-dir", "/etc/ungranted-plugins"]),
            "--plugin-dir",
            "claude",
        ),
        (
            claude(&["--allowedTools", "Read", "mcp__ungranted__fetch"]),
            "--allowedTools mcp__*",
            "claude",
        ),
        // Counterfeit hands: the engine's server name, authored.
        (
            codex(&["-c", "mcp_servers.brokkr.command=\"{brokkr}\""]),
            "-c mcp_servers",
            "codex",
        ),
        (
            claude(&[
                "--mcp-config",
                "/etc/ungranted.json",
                "--allowedTools",
                "mcp__ungranted__fetch",
            ]),
            "--mcp-config",
            "claude",
        ),
        (
            claude(&["--allowedTools", "Bash(git:*),mcp__ungranted__fetch"]),
            "--allowedTools mcp__*",
            "claude",
        ),
        (claude(&["--allowed-tools=*"]), "--allowedTools *", "claude"),
    ] {
        one_inline_seat(&operator, &command);
        assert_eq!(
            solo(&operator, &adapters, &context),
            refusal("work", written, provider),
            "{command:?}"
        );
    }
    // Nested: a panel member and a sequence step are sites like any other.
    for (body, site) in [
        (
            json!({"results": ["complete"], "sequence": [
            {"name": "first", "aggregate": "unanimous-pass", "panel": {
                "member": {"role": "roles/role.md", "driver": {"command": codex(&[
                    "-c", "mcp_servers.ungranted.command=\"npx\""])}},
                "peer": {"role": "roles/role.md", "driver": {"command": ["driver"]}}}},
            {"name": "second", "role": "roles/role.md", "driver": {"command": ["driver"]}}]}),
            "work:first:member",
        ),
        (
            json!({"results": ["complete"], "sequence": [
            {"name": "before", "results": ["drafted"], "role": "roles/role.md",
             "driver": {"command": ["driver"]}},
            {"name": "step", "role": "roles/role.md", "driver": {"command": codex(&[
                "-c", "mcp_servers.ungranted.command=\"npx\""])}}]}),
            "work:step",
        ),
    ] {
        one_inline_seat(&operator, &CODEX_SEAT);
        let path = operator.root().join("solo/bundle.json");
        let mut bundle: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        bundle["seats"]["work"] = body;
        std::fs::write(&path, serde_json::to_vec(&bundle).unwrap()).unwrap();
        assert_eq!(
            solo(&operator, &adapters, &context),
            refusal(site, "-c mcp_servers", "codex"),
            "{site}"
        );
    }
    // The engine's OWN hands, under the same empty grants: the agent-backed
    // boxed Codex seat launches with the adapter's workspace server and the
    // OFF pair last; the Claude primary of the chain keeps its strict MCP
    // configuration and its one allowed workspace tool.
    let bundle = operator
        .compile(&context, Boundary::Namespace, None, None)
        .unwrap();
    let boxed = launch(&bundle, "agent", 0);
    assert!(
        boxed
            .iter()
            .any(|part| part.starts_with("mcp_servers.brokkr.command=")),
        "{boxed:?}"
    );
    assert_eq!(&boxed[boxed.len() - 2..], OFF, "{boxed:?}");
    assert_eq!(off_pairs(&boxed), 1, "{boxed:?}");
    let primary = launch(&bundle, "chain", 0);
    for kept in [
        "--strict-mcp-config",
        "--mcp-config",
        "mcp__brokkr__workspace",
    ] {
        assert!(
            primary.iter().any(|part| part.contains(kept)),
            "{kept}: {primary:?}"
        );
    }
    assert_eq!(
        primary
            .iter()
            .filter(|part| *part == "--disallowedTools")
            .count(),
        1,
        "{primary:?}"
    );
}

/// Finding H3, the council's reproduction along the whole chain: Claude's
/// search OFF declared as ARGV `--disallowedTools WebSearch`, its fetch OFF
/// as a SELECTION. The compiler accepted both and the launch consumed only
/// the selection, so search was recorded OFF and never denied. Both now
/// reach the final command as ONE deny list under one flag — boxed beside
/// the engine's hands, and unboxed — and a representation no launch
/// consumes refuses at COMPILE, naming the site, the provider and the form.
#[test]
fn a_native_control_declared_as_argv_reaches_the_final_claude_command() {
    let operator = Operator::new();
    let context = operator.context(json!({}));
    let mixed = copied_adapters();
    edit_adapter(mixed.path(), "claude", |adapter| {
        adapter["native_capabilities"]["known"]["web-search"]["off"] =
            json!({"argv": ["--disallowedTools", "WebSearch"]});
    });
    // Boxed, the chain's Claude primary: the hands fragment intact, and the
    // two denials in one list.
    let bundle = operator
        .compile_against(mixed.path(), &context, Boundary::Namespace, None, None)
        .unwrap();
    let primary = launch(&bundle, "chain", 0);
    let deny = primary
        .iter()
        .position(|part| part == "--disallowedTools")
        .unwrap();
    assert_eq!(primary[deny + 1], "WebFetch,WebSearch", "{primary:?}");
    assert_eq!(
        primary
            .iter()
            .filter(|part| *part == "--disallowedTools")
            .count(),
        1,
        "{primary:?}"
    );
    assert!(
        primary.contains(&"mcp__brokkr__workspace".to_string()),
        "{primary:?}"
    );
    // Unboxed and inline: the whole final command.
    one_inline_seat(
        &operator,
        &[
            "{brokkr}",
            "driver",
            "claude",
            "--",
            "--model",
            "claude-opus-5",
            "--effort",
            "high",
            "--permission-mode",
            "acceptEdits",
        ],
    );
    assert_eq!(
        solo(&operator, mixed.path(), &context),
        format!(
            "launched {:?}",
            [
                "claude",
                "-p",
                "--output-format",
                "stream-json",
                "--verbose",
                "--model",
                "claude-opus-5",
                "--effort",
                "high",
                "--permission-mode",
                "acceptEdits",
                "--disallowedTools",
                "WebFetch,WebSearch"
            ]
        )
    );
    // A form no launch consumes is refused where it is compiled: Codex
    // takes no tool selection.
    let selecting = copied_adapters();
    edit_adapter(selecting.path(), "codex", |adapter| {
        let native = &mut adapter["native_capabilities"];
        native["known"]["web-search"]["off"] =
            json!({"selection": {"include": [], "allow": [], "deny": ["web_search"]}});
        native["selection"] = json!({
            "include": {"flag": "--tools", "separator": ","},
            "allow": {"flag": "--allow", "separator": ","},
            "deny": {"flag": "--deny", "separator": ","}});
    });
    one_inline_seat(&operator, &CODEX_SEAT);
    assert_eq!(
        solo(&operator, selecting.path(), &context),
        "bundle: seat 'work' (office 'work') in realm 'private': the capability plan carries a \
         tool selection for provider 'codex', which its launch does not consume; a control that \
         cannot reach the final command is refused rather than recorded and dropped (decision \
         0066 ruling 3)"
    );
}
