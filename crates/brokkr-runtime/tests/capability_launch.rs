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
        // A recipe authors no `--sandbox` (operator ruling 1 of 2026-09-23):
        // the inline seat declares its class, which the engine lowers as its
        // own segment (rebuild unit 5d; fixture migration of 2026-09-26).
        let mut inline = json!({"results": ["complete"], "role": "roles/role.md",
            "tools": {"sandbox": "workspace-write"}, "driver": codex(&[])});
        // An inline seat with hands authors NO box tokens: a recipe's argv
        // carries no capability server, the engine's own included (decision
        // 0066 ruling 4). The boxed seat that does get the workspace server
        // is the agent-backed one, whose adapter owns the fragment. Beside
        // hands no inline class is declared: the box confines it (D5.3).
        let mut boxed = json!({"results": ["complete"], "role": "roles/role.md",
            "hands": {"kind": "workspace", "network": false, "binds": []},
            "driver": codex(&[])});
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

/// [`try_launch`] through the dispatch door (rebuild unit 12-fix-c, C3):
/// the site's spawn sealed and its record verified ([`sealed`]), then the
/// driver handed the input dispatch writes — the plan, the argv's
/// provenance and the sealed record — and the actual command builder run on
/// the sealed spawn's own argv.
fn sealed_launch(bundle: &Bundle, label: &str, candidate: usize) -> Result<Vec<String>, String> {
    let (spawn, mut input) = sealed(bundle, label, candidate);
    let outcome = &bundle.sites[label].capabilities.as_ref().unwrap().outcomes[candidate];
    input["workdir"] = json!("/w");
    input["seat"] = json!(label);
    input["native_controls"] = outcome.controls();
    input["launch_arguments"] = spawn.launch_arguments();
    let argv = &spawn.argv;
    let extra = &argv[argv.iter().position(|part| part == "--").unwrap() + 1..];
    match outcome.provider.as_str() {
        "codex" => brokkr_protocol::adapters::codex_command("codex", extra, "/w", None, &input),
        _ => brokkr_protocol::adapters::claude_command("claude", extra, None, &input),
    }
}

/// [`solo`], launched through [`sealed_launch`].
fn solo_sealed(operator: &Operator, adapters: &Path, context: &CapabilityContext) -> String {
    match solo_bundle(operator, adapters, context) {
        Ok(bundle) => match sealed_launch(&bundle, "work", 0) {
            Ok(argv) => format!("launched {argv:?}"),
            Err(refusal) => format!("compiled, and the driver said: {refusal}"),
        },
        Err(refusal) => refusal,
    }
}

/// The authored command of the inline site `label`: a single seat's, or
/// the panel member `seat:member`'s.
fn inline_command(bundle: &Bundle, label: &str) -> Vec<String> {
    let (seat, member) = label.split_once(':').unwrap_or((label, ""));
    match &bundle.seats[seat].body {
        SeatBody::Single { command, .. } if member.is_empty() => command.clone(),
        SeatBody::Panel { members, .. } => members
            .iter()
            .find(|each| each.name == member)
            .map(|each| each.command.clone())
            .unwrap_or_else(|| panic!("{label} is no panel member")),
        _ => panic!("{label} is a single seat or a panel member"),
    }
}

/// Review return F2 of rebuild unit 14a4a: [`sealed_launch`]'s cold Codex
/// or Claude command (rebuild unit 14a4b), then judged by `check_final`
/// under its provider's harness as rebuild unit 14 serves it —
/// against the sealed record, its plan, and the serving inputs dispatch
/// seals beside them (the selected candidate's composition, or the inline
/// site's recorded dialect and hands, the boundary fragment its class
/// selects, and the boundary the site stood under), sealed by the engine's
/// own `serving_inputs` (rebuild unit 14a4c), with the box's transport
/// bound to the executable and workdir the engine composed the spawn with.
/// The checked argv, or the refusal.
fn checked_launch(bundle: &Bundle, label: &str, candidate: usize) -> Result<Vec<String>, String> {
    let facts = &bundle.sites[label];
    let (spawn, _) = sealed(bundle, label, candidate);
    let carried = brokkr_runtime::engine::serving_inputs(
        facts.chain.get(candidate),
        Some(facts),
        spawn.class,
        bundle.boundary,
    )?;
    checked_against(bundle, label, candidate, &carried)
}

/// [`checked_launch`] against the serving inputs `carried`, however they
/// were sealed.
fn checked_against(
    bundle: &Bundle,
    label: &str,
    candidate: usize,
    carried: &brokkr_protocol::native_controls::SealedServing,
) -> Result<Vec<String>, String> {
    use brokkr_protocol::native_controls::{
        check_final, managed, Checked, Dialect, LaunchRecord, Origin, Serving, Transport,
    };
    let facts = &bundle.sites[label];
    let (spawn, _) = sealed(bundle, label, candidate);
    let command = sealed_launch(bundle, label, candidate)?;
    let outcome = &facts.capabilities.as_ref().unwrap().outcomes[candidate];
    let harness = outcome.provider.as_str();
    let controls = managed(&json!({"native_controls": outcome.controls()}))?.unwrap();
    let record = LaunchRecord::decode(Some(&spawn.launch_record()))?;
    let authored: Vec<String> = record
        .segments
        .iter()
        .filter(|segment| segment.origin == Origin::Authored)
        .flat_map(|segment| segment.argv.iter().cloned())
        .collect();
    let brokkr = std::env::current_exe().unwrap();
    check_final(
        harness,
        command,
        &controls,
        &record.expected,
        Dialect {
            permissions: carried.dialect.permissions.as_ref(),
            sandbox: &carried.dialect.sandbox,
            hands: &carried.dialect.hands,
            boundary: &carried.dialect.boundary,
            stands: carried.dialect.stands,
        },
        Serving {
            program: harness,
            workdir: "/w",
            authored: &authored,
            pins: &carried.pins,
            hands: carried.spec.as_ref().map(|spec| Transport {
                brokkr: &brokkr,
                workdir: Path::new("/w"),
                spec,
            }),
            ..Default::default()
        },
    )
    .map(Checked::into_argv)
    .map_err(|refusal| refusal.cause)
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
        None => inline_command(bundle, label),
    };
    let built = match bundle.boundary {
        Boundary::Open => brokkr_runtime::engine::BuiltBoundary::Open,
        boxed if boxed.is_boxed() => brokkr_runtime::engine::BuiltBoundary::Namespace,
        _ => brokkr_runtime::engine::BuiltBoundary::Harness,
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
    // Composed from the site's facts, as dispatch composes it, so an inline
    // seat's lowered class is the engine's own segment (rebuild unit 5d).
    let spawn = brokkr_runtime::engine::compose_site_at(
        Some(facts),
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

/// The seat's own controls survive beside the managed one. A sandbox is
/// the engine's alone, exactly one: the inline seat's lowered class, an
/// agent's hands; the boxed inline seat authors none (fixture migration of
/// 2026-09-26) and is served its hands like an agent (operator ruling (B)
/// of 2026-09-27; rebuild unit 14a4a).
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
    assert_eq!(
        argv.iter().filter(|part| *part == "--sandbox").count(),
        1,
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
    // The inline seat's command is the same bytes: its pins are authored,
    // and the class it declares is the engine's `local` segment, never
    // hands (fixture migration of 2026-09-26; rebuild unit 5d).
    let (inline, sealed_inline) = sealing(&unboxed, "inline", 0, &unboxed.sites["inline"]);
    assert_eq!(sealed_inline, Ok(()));
    let copied = inline.launch_record();
    assert_eq!(
        copied["segments"],
        json!([{"origin": "authored",
                "argv": ["--model", "gpt-6-astra", "--effort", "high"]},
               {"origin": "local", "argv": ["--sandbox", "workspace-write"]}])
    );
    assert_eq!(inline.argv[1..], agent.argv[1..]);
    assert_eq!(
        copied["expected"]["local"],
        json!({"allow": {"kind": "unspecified"}, "sandbox": {"kind": "workspace-write"},
               "application": {"kind": "unrestricted"}})
    );
    assert_eq!(copied["expected"]["hands"], json!({"kind": "none"}));
    // An inline site whose adapter declares no template emits none and
    // expects none (rebuild unit 5c-fix).
    assert_eq!(copied["expected"]["template"], json!({"kind": "none"}));
    // Counterfeit origin (operator ruling 1 of 2026-09-23; rebuild unit
    // 12): a recipe that AUTHORS the engine's own class bytes is refused at
    // compile by origin, the bytes proving nothing.
    one_inline_seat(
        &operator,
        &[&CODEX_SEAT[..], &["--sandbox", "workspace-write"]].concat(),
    );
    assert_eq!(
        solo(&operator, &workspace().join("adapters"), &context),
        "bundle: seat 'work' (office 'work') in realm 'private': its arguments carry \
         '--sandbox' (argument 5), a capability-bearing option of harness 'codex'. A recipe \
         authors no capability-bearing option, whatever its value, polarity or grant: tools \
         come from typed declarations and the realm's grant, composed by the engine alone \
         (operator ruling 1 of 2026-09-23)"
    );

    // `namespace`: the Claude primary and its Codex fallback, each under
    // its own identity, with the box's workspace fragment as hands.
    let boxed = operator
        .compile(&context, Boundary::Namespace, None, Some(json!({})))
        .unwrap();
    // The boxed inline seat's arguments are its author's, then the box's
    // workspace fragment as the engine's hands (operator ruling (B) of
    // 2026-09-27; rebuild unit 14a4a); it authors no sandbox (fixture
    // migration of 2026-09-26).
    let (_, input) = sealed(&boxed, "boxed", 0);
    assert_eq!(
        input["launch_record"]["segments"],
        json!([{"origin": "authored",
                "argv": ["--model", "gpt-6-astra", "--effort", "high"]},
               {"origin": "hands", "argv": boxed_hands(&boxed, "boxed")}])
    );
    assert_eq!(
        input["launch_record"]["expected"]["hands"],
        json!({"kind": "required"})
    );
    // Rebuild unit 5c-fix2: the Claude primary's composition emits the
    // adapter's permission template behind its verb, and its expected state
    // records the adapter's declaration of it exactly, so it is sealed.
    let (_, input) = sealed(&boxed, "chain", 0);
    let primary = &input["launch_record"];
    assert_eq!(
        primary["expected"]["template"],
        json!({"kind": "declared", "argv": ["--permission-mode", "acceptEdits"]})
    );
    assert_eq!(
        origins(primary),
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

/// The shipped Codex adapter's `hands.workspace` fragment, expanded as the
/// box expands it for `label`'s hands at `/w`: the independent value an
/// emitted hands segment is compared with.
fn boxed_hands(bundle: &Bundle, label: &str) -> Vec<String> {
    let adapter: Value =
        serde_json::from_slice(&std::fs::read(workspace().join("adapters/codex.json")).unwrap())
            .unwrap();
    let fragment: Vec<String> =
        serde_json::from_value(adapter["hands"]["workspace"].clone()).unwrap();
    brokkr_protocol::native_controls::Transport {
        brokkr: &std::env::current_exe().unwrap(),
        workdir: Path::new("/w"),
        spec: &bundle.hands[label],
    }
    .expand(&fragment)
    .unwrap()
}

/// Rebuild unit 14a4a (operator ruling (B) of 2026-09-27): the boxed inline
/// Codex seat is served like its agent-backed Codex fallback. Its sealed
/// launch ends in its adapter's `hands.workspace` fragment as the engine's
/// `hands` segment, expanded for the box exactly as the fallback's is; its
/// plan types that segment's whole length as the box's hands, as the
/// fallback's does; and the driver composes the same launch from both,
/// which the final check passes as the exact cold command (review return
/// F2).
#[test]
fn a_boxed_inline_seats_hands_are_emitted_and_typed_as_an_agent_backed_seats_are() {
    let operator = Operator::new();
    let context = CapabilityContext::no_grants("private", operator.root());
    let bundle = operator
        .compile(&context, Boundary::Namespace, None, Some(json!({})))
        .unwrap();
    let served = |label: &str, candidate: usize| {
        let (_, input) = sealed(&bundle, label, candidate);
        let segments = input["launch_record"]["segments"].as_array().unwrap();
        let outcome = &bundle.sites[label].capabilities.as_ref().unwrap().outcomes[candidate];
        (
            segments.last().unwrap().clone(),
            outcome.controls()["hands"].clone(),
            checked_launch(&bundle, label, candidate),
        )
    };
    let expanded = boxed_hands(&bundle, "boxed");
    let (hands, typed, launched) = served("boxed", 0);
    assert_eq!(hands, json!({"origin": "hands", "argv": expanded}));
    assert_eq!(typed, json!(expanded.len()));
    assert_eq!(launched, Ok(checked_codex(&expanded)));
    assert_eq!((hands, typed, launched), served("chain", 1));
}

/// The cold command the final check passes for a boxed Codex site on
/// `gpt-6-astra` at `high` that holds nothing, with `hands` its box's
/// expanded `hands.workspace` fragment, and web search switched off
/// behind it: written out, never composed.
fn checked_codex(hands: &[String]) -> Vec<String> {
    let lead = [
        "codex",
        "exec",
        "--json",
        "-C",
        "/w",
        "-c",
        "model_reasoning_effort=\"high\"",
        "--model",
        "gpt-6-astra",
    ];
    [
        lead.map(String::from).to_vec(),
        hands.to_vec(),
        OFF.map(String::from).to_vec(),
    ]
    .concat()
}

/// Review return F2 of rebuild unit 14a4a, the path 14b serves: a compiled
/// inline Codex panel member with boxed hands passes the final check —
/// its cold command rebuilt from its sealed record, its plan and its
/// recorded dialect and hands, with the box's transport bound as the
/// engine bound it — as exactly the command an agent-backed Codex member
/// of the same panel passes.
#[test]
fn a_compiled_inline_codex_panel_member_with_hands_passes_the_final_check_as_an_agent_member_does()
{
    let operator = Operator::new();
    write(
        operator.root(),
        "bundle/policy.json",
        &json!({"phases": ["judges", "review", "done"], "initial": "judges",
            "terminal": ["done"], "rules": [
                {"id": "A", "from": "judges", "result": "pass", "next": "review", "reason": "r"},
                {"id": "B", "from": "judges", "result": "fail", "next": "review", "reason": "r"},
                {"id": "C", "from": "review", "result": "clean", "next": "done", "reason": "r"}]}),
    );
    write(
        operator.root(),
        "bundle/bundle.json",
        &json!({"name": "panel", "policy": "policy.json", "seats": {
            "judges": {"results": ["pass", "fail"], "aggregate": "unanimous-pass", "panel": {
                "inline": {"role": "roles/role.md",
                    "hands": {"kind": "workspace", "network": false, "binds": []},
                    "driver": {"command": ["{brokkr}", "driver", "codex", "--",
                                           "--model", "gpt-6-astra", "--effort", "high"]}},
                "agent": {"agent": "searcher"}}},
            "review": {"results": ["clean"], "role": "roles/role.md",
                       "driver": {"command": ["driver"]}}}}),
    );
    let bundle = Bundle::compile_with_capabilities(
        &operator.root().join("bundle"),
        &operator.root().join("agents"),
        &workspace().join("adapters"),
        Some("private"),
        None,
        Boundary::Namespace,
        &CapabilityContext::no_grants("private", operator.root()),
    )
    .unwrap();
    let expected = checked_codex(&boxed_hands(&bundle, "judges:inline"));
    assert_eq!(
        checked_launch(&bundle, "judges:inline", 0),
        Ok(expected.clone())
    );
    assert_eq!(checked_launch(&bundle, "judges:agent", 0), Ok(expected));
}

/// Unit 4 (design D5.7): an office's direct allow list, compiled for an
/// unboxed Claude seat, reaches the spawn as the adapter's exact mapped
/// limits under the `local` origin — beside the template it was composed
/// after — with the ordered names and limits in its composition's local
/// expectation, independently of the joined flag value. Rebuild unit
/// 5c-fix2: the seat's emitted permission template is sealed as the
/// adapter's declaration its expected state records.
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
    // Rebuild unit 5c-fix2: the shipped Claude adapter's composition emits
    // its permission template, and the expected state records the adapter's
    // declaration of it from the composition's typed fact, so the seat is
    // sealed beside its local limits.
    let (_, input) = sealed(&bundle, "work", 0);
    let record = &input["launch_record"];
    let brokkr_runtime::agents::Lowering::Composed(composition) =
        &bundle.sites["work"].chain[0].lowering
    else {
        panic!("a resolved link carries its composition");
    };
    assert_eq!(
        json!({
            "template": record["expected"]["template"],
            "local": record["expected"]["local"],
            "segments": record["segments"],
            "composed": format!("{:?}", composition.local()),
        }),
        json!({
            "template": {"kind": "declared", "argv": ["--permission-mode", "acceptEdits"]},
            "local": {"allow": {"kind": "listed", "names": ["pytest", "cargo"]},
                      "sandbox": {"kind": "unspecified"},
                      "application": {"kind": "direct",
                                      "limits": ["Bash(.venv/bin/pytest:*)", "Bash(cargo:*)"]}},
            "segments": [
                {"origin": "template", "argv": ["--permission-mode", "acceptEdits"]},
                {"origin": "template", "argv": ["--model", "claude-opus-5-5"]},
                {"origin": "template", "argv": ["--effort", "high"]},
                {"origin": "local",
                 "argv": ["--allowedTools", "Bash(.venv/bin/pytest:*),Bash(cargo:*)"]},
            ],
            "composed": format!("{:?}", brokkr_protocol::native_controls::LocalExpectation {
                allow: brokkr_protocol::native_controls::AllowIntent::Listed(vec![
                    "pytest".into(), "cargo".into()]),
                sandbox: brokkr_protocol::native_controls::SandboxIntent::Unspecified,
                application: brokkr_protocol::native_controls::Application::Direct(vec![
                    "Bash(.venv/bin/pytest:*)".into(), "Bash(cargo:*)".into()]),
            }),
        })
    );
}

/// Rebuild unit 5c-fix2 (operator ruling 2 of 2026-09-23; the ruling of
/// 2026-09-24, item 2): an agent's seat records the permission template its
/// adapter DECLARES, carried by its composition as a typed fact, and the
/// seal admits the template it emits against that record. Over copies of
/// the shipped adapters, canonicalised once, the same office on the shipped
/// Claude and LaneTally adapters records `acceptEdits` exactly; on the
/// shipped Codex adapter, which declares no template, `none`. A Claude
/// driver that declares its template directly behind the verb records it
/// too; one that ends at the terminator, or at the verb, records `none`;
/// and an opaque driver, which dispatches through no verb and whose argv
/// the engine composes whole and never parses, declares no template behind
/// one and records `none`.
#[test]
fn an_agent_backed_seat_records_the_permission_template_its_adapter_declares() {
    let operator = Operator::new();
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
    let sealed_with = |template: Value| format!("{:?} {template}", Ok::<(), String>(()));
    let acceptance = json!({"kind": "declared", "argv": ["--permission-mode", "acceptEdits"]});
    let none = json!({"kind": "none"});
    let rows = [
        (
            "the shipped claude template",
            "claude",
            "opus",
            None,
            sealed_with(acceptance.clone()),
        ),
        (
            "the shipped lanetally template",
            "lanetally",
            "opus-tallied",
            None,
            sealed_with(acceptance.clone()),
        ),
        (
            "the shipped codex adapter, no template",
            "codex",
            "astra",
            None,
            sealed_with(none.clone()),
        ),
        (
            "a template directly behind the verb",
            "claude",
            "opus",
            Some(json!([
                "{brokkr}",
                "driver",
                "claude",
                "--permission-mode",
                "plan"
            ])),
            sealed_with(json!({"kind": "declared", "argv": ["--permission-mode", "plan"]})),
        ),
        (
            "nothing behind the terminator",
            "claude",
            "opus",
            Some(json!(["{brokkr}", "driver", "claude", "--"])),
            sealed_with(none.clone()),
        ),
        (
            "nothing behind the verb",
            "claude",
            "opus",
            Some(json!(["{brokkr}", "driver", "claude"])),
            sealed_with(none.clone()),
        ),
        (
            "an opaque driver",
            "claude",
            "opus",
            Some(json!([
                "claude-wrapper",
                "--permission-mode",
                "acceptEdits"
            ])),
            sealed_with(none.clone()),
        ),
    ];
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(|(label, provider, model, driver, expected)| {
            if let Some(driver) = driver {
                edit_adapter(&root, provider, |adapter| adapter["driver"] = driver);
            }
            write(
                operator.root(),
                "agents/plain.json",
                &json!({
                    "description": "an office that declares no local tools",
                    "charter": "charters/searcher.md",
                    "models": [model],
                    "efforts": {model: "high"},
                }),
            );
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

/// The solo bundle's work seat hired from the office `plain` (opus, at
/// `effort`), which declares no local tools, under a bare review gate.
fn agent_backed_solo(operator: &Operator, effort: &str) {
    write(
        operator.root(),
        "agents/plain.json",
        &json!({
            "description": "an office that declares no local tools",
            "charter": "charters/searcher.md",
            "models": ["opus"],
            "efforts": {"opus": effort},
        }),
    );
    one_inline_seat(operator, &["driver"]);
    write(
        operator.root(),
        "solo/bundle.json",
        &json!({"name": "solo", "policy": "policy.json", "seats": {
            "work": {"results": ["complete"], "agent": "plain"},
            "review": {"results": ["clean"], "role": "roles/role.md",
                       "driver": {"command": ["driver"]}}}}),
    );
}

/// The shipped Claude adapter with its driver ending at the terminator, so
/// its agent-backed seat emits no permission template behind the verb and
/// the interim agent arm seals `none` (rebuild unit 5c-fix).
fn untemplated_claude(root: &Path) {
    edit_adapter(root, "claude", |adapter| {
        adapter["driver"] = json!(["{brokkr}", "driver", "claude", "--"]);
    });
}

/// Rebuild unit 5c-fix-b (chief R1, scenarios 2 and 3): a model or effort
/// pin never carries a permission control. On a Claude adapter whose driver
/// emits no template, an adapter that declares its `model_flag` or
/// `effort_flag` as a permission control — split or `=`-joined — refuses
/// the compile naming the field and the control's canonical spelling and
/// never the value it would pin; a model or effort VALUE that spells one,
/// and a flag that is not the harness's model or effort option, refuse
/// naming the contribution and a fixed cause. The legitimate `--model` and
/// `--effort` pins compile, seal `none` and reach the final command.
#[test]
fn an_adapter_whose_model_or_effort_pin_carries_a_permission_control_refuses_the_compile() {
    let operator = Operator::new();
    let context = CapabilityContext::no_grants("private", operator.root());
    let declared = |field: &str, control: &str| {
        format!(
            "bundle: seat 'work': the 'claude' adapter declares its {field} as the permission control \
             '{control}'; a model or effort pin names a model or an effort and never carries a \
             permission mode, so the declaration is refused rather than composed (operator ruling \
             1 of 2026-09-23; rebuild unit 5c-fix-b)"
        )
    };
    let contribution = |segment: usize, fault: &str| {
        format!(
            "bundle: seat 'work': the 'claude' adapter's composition carries a template contribution \
             (segment {segment}) behind its driver template that {fault}; only a model or effort \
             pin may follow the driver template, and its tokens are not echoed because they can \
             carry a value (operator ruling 1 of 2026-09-23; rebuild unit 5c-fix-b)"
        )
    };
    type Edit = Box<dyn Fn(&mut Value)>;
    let rows: Vec<(&str, &str, Edit, String)> = vec![
        (
            "the legitimate pins",
            "high",
            Box::new(|_| {}),
            format!(
                "{:?} {} launched {:?}",
                Ok::<(), String>(()),
                json!({"kind": "none"}),
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
                    "--disallowedTools",
                    "WebFetch,WebSearch"
                ]
            ),
        ),
        (
            "model_flag a permission mode, the model bypassPermissions",
            "high",
            Box::new(|adapter| {
                adapter["model_flag"] = json!("--permission-mode");
                adapter["models"]["opus"] = json!("bypassPermissions");
            }),
            declared("model_flag", "--permission-mode"),
        ),
        (
            "model_flag a joined permission mode",
            "high",
            Box::new(|adapter| adapter["model_flag"] = json!("--permission-mode=plan")),
            declared("model_flag", "--permission-mode"),
        ),
        (
            "effort_flag a permission mode, the effort plan",
            "plan",
            Box::new(|adapter| {
                adapter["effort_flag"] = json!("--permission-mode");
                adapter["efforts"] = json!(["plan"]);
            }),
            declared("effort_flag", "--permission-mode"),
        ),
        (
            "effort_flag a bypass switch",
            "high",
            Box::new(|adapter| adapter["effort_flag"] = json!("--dangerously-skip-permissions")),
            declared("effort_flag", "--dangerously-skip-permissions"),
        ),
        (
            "a model value that spells a permission mode",
            "high",
            Box::new(|adapter| {
                adapter["models"]["opus"] = json!("--permission-mode=bypassPermissions")
            }),
            contribution(2, "spells a permission control"),
        ),
        (
            "effort_flag a loading option",
            "high",
            Box::new(|adapter| adapter["effort_flag"] = json!("--settings")),
            contribution(3, "is not its harness's model or effort option"),
        ),
        (
            "model_flag a loading option",
            "high",
            Box::new(|adapter| adapter["model_flag"] = json!("--settings")),
            contribution(2, "is not its harness's model or effort option"),
        ),
        // The returned R1: the whole specified inventory, on a DORMANT
        // effort_flag (the office pins no effort, so no effort segment is
        // ever composed for a later check to see) and on emitted ones.
        (
            "the legitimate model pin with no effort pinned",
            "",
            Box::new(|_| {}),
            format!(
                "{:?} {} launched {:?}",
                Ok::<(), String>(()),
                json!({"kind": "none"}),
                [
                    "claude",
                    "-p",
                    "--output-format",
                    "stream-json",
                    "--verbose",
                    "--model",
                    "route/claude-opus-5-5",
                    "--disallowedTools",
                    "WebFetch,WebSearch"
                ]
            ),
        ),
        (
            "a dormant effort_flag: additional directories",
            "",
            Box::new(|adapter| adapter["effort_flag"] = json!("--add-dir")),
            declared("effort_flag", "--add-dir"),
        ),
        (
            "a dormant effort_flag: the bypass alias",
            "",
            Box::new(|adapter| adapter["effort_flag"] = json!("--yolo")),
            declared("effort_flag", "--dangerously-bypass-approvals-and-sandbox"),
        ),
        (
            "an emitted model_flag: the permission prompt tool",
            "high",
            Box::new(|adapter| adapter["model_flag"] = json!("--permission-prompt-tool")),
            declared("model_flag", "--permission-prompt-tool"),
        ),
        (
            "an emitted effort_flag: approve for me",
            "high",
            Box::new(|adapter| adapter["effort_flag"] = json!("--approve-for-me")),
            declared("effort_flag", "--approve-for-me"),
        ),
        (
            "an emitted effort_flag: ignore rules",
            "high",
            Box::new(|adapter| adapter["effort_flag"] = json!("--ignore-rules")),
            declared("effort_flag", "--ignore-rules"),
        ),
    ];
    assert_eq!(rows.len(), 14);
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(|(label, effort, edit, expected)| {
            let adapters = copied_adapters();
            let root = std::fs::canonicalize(adapters.path()).unwrap();
            untemplated_claude(&root);
            edit_adapter(&root, "claude", |adapter| edit(adapter));
            agent_backed_solo(&operator, effort);
            if effort.is_empty() {
                // The office pins no effort: its model resolves to an
                // effortless route, so no effort segment is composed.
                edit_adapter(&root, "claude", |adapter| {
                    adapter["effortless_routes"] = json!({"route": "a fixture route"});
                    adapter["models"]["opus"] = json!("route/claude-opus-5-5");
                });
                write(
                    operator.root(),
                    "agents/plain.json",
                    &json!({
                        "description": "an office that declares no local tools",
                        "charter": "charters/searcher.md",
                        "models": ["opus"],
                    }),
                );
            }
            let observed = match solo_bundle(&operator, &root, &context) {
                Ok(bundle) => {
                    let (spawn, sealing) = sealing(&bundle, "work", 0, &bundle.sites["work"]);
                    let final_command = match try_launch(&bundle, "work", 0) {
                        Ok(argv) => format!("launched {argv:?}"),
                        Err(refusal) => format!("the driver said: {refusal}"),
                    };
                    format!(
                        "{sealing:?} {} {final_command}",
                        spawn.launch_record()["expected"]["template"]
                    )
                }
                Err(refusal) => refusal,
            };
            (observed != expected)
                .then(|| format!("row {label}:\n  left:  {observed}\n  right: {expected}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Rebuild unit 5c-fix-b (the second returned R1): Codex's permission and
/// sandbox configuration assignments are permission controls a model or
/// effort flag may not be either. On a Codex route whose office pins no
/// effort, the effort flag is DORMANT — no effort segment is composed for a
/// later check to see — and an adapter declaring it as a `-cKEY=V`,
/// `-c=KEY=V` or `--config=KEY=V` assignment into or under
/// `approval_policy`, `sandbox_mode` or `sandbox_workspace_write` refuses
/// the compile naming the field and the table, never the value; so does
/// the same flag where an office pins an effort and it would be EMITTED.
/// The legitimate model pin, with no effort and with one, compiles, seals
/// `none` and reaches the final command with native search switched off.
#[test]
fn a_codex_adapter_whose_effort_flag_assigns_a_permission_table_refuses_the_compile() {
    let operator = Operator::new();
    let context = CapabilityContext::no_grants("private", operator.root());
    let declared = |field: &str, control: &str| {
        format!(
            "bundle: seat 'work': the 'codex' adapter declares its {field} as the permission control \
             '{control}'; a model or effort pin names a model or an effort and never carries a \
             permission mode, so the declaration is refused rather than composed (operator ruling \
             1 of 2026-09-23; rebuild unit 5c-fix-b)"
        )
    };
    let launched = |pins: &[&str]| {
        let mut argv = vec!["codex", "exec", "--json", "-C", "/w"];
        argv.extend(pins);
        argv.extend(["-c", "web_search=\"disabled\""]);
        format!(
            "{:?} {} launched {argv:?}",
            Ok::<(), String>(()),
            json!({"kind": "none"})
        )
    };
    let rows: [(&str, Option<&str>, &str, String); 8] = [
        (
            "the legitimate model pin, no effort",
            None,
            "",
            launched(&["--model", "route/gpt-6-astra"]),
        ),
        (
            "the legitimate model and effort pins",
            None,
            "high",
            launched(&[
                "-c",
                "model_reasoning_effort=\"high\"",
                "--model",
                "gpt-6-astra",
            ]),
        ),
        (
            "dormant: an attached approval policy",
            Some("-capproval_policy=never"),
            "",
            declared("effort_flag", "--config approval_policy"),
        ),
        (
            "dormant: an equals-joined sandbox mode",
            Some("-c=sandbox_mode=\"danger-full-access\""),
            "",
            declared("effort_flag", "--config sandbox_mode"),
        ),
        (
            "dormant: a long workspace-write descendant",
            Some("--config=sandbox_workspace_write.network_access=true"),
            "",
            declared("effort_flag", "--config sandbox_workspace_write"),
        ),
        (
            "emitted: an attached approval policy",
            Some("-capproval_policy=never"),
            "high",
            declared("effort_flag", "--config approval_policy"),
        ),
        (
            "emitted: an equals-joined sandbox mode",
            Some("-c=sandbox_mode=\"danger-full-access\""),
            "high",
            declared("effort_flag", "--config sandbox_mode"),
        ),
        (
            "emitted: a long workspace-write descendant",
            Some("--config=sandbox_workspace_write.network_access=true"),
            "high",
            declared("effort_flag", "--config sandbox_workspace_write"),
        ),
    ];
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(|(label, flag, effort, expected)| {
            let adapters = copied_adapters();
            let root = std::fs::canonicalize(adapters.path()).unwrap();
            edit_adapter(&root, "codex", |adapter| {
                adapter["effortless_routes"] = json!({"route": "a fixture route"});
                adapter["models"]["astra"] = json!("route/gpt-6-astra");
                if let Some(flag) = flag {
                    adapter["effort_flag"] = json!(flag);
                }
            });
            agent_backed_solo(&operator, effort);
            let mut office = json!({
                "description": "an office that declares no local tools",
                "charter": "charters/searcher.md",
                "models": ["astra"],
            });
            if !effort.is_empty() {
                // An effort pinned on an effortless route still reaches the
                // declaration check first: it is refused whichever it pins.
                edit_adapter(&root, "codex", |adapter| {
                    adapter["models"]["astra"] = json!("gpt-6-astra");
                });
                office["efforts"] = json!({"astra": effort});
            }
            write(operator.root(), "agents/plain.json", &office);
            let observed = match solo_bundle(&operator, &root, &context) {
                Ok(bundle) => {
                    let (spawn, sealing) = sealing(&bundle, "work", 0, &bundle.sites["work"]);
                    let final_command = match try_launch(&bundle, "work", 0) {
                        Ok(argv) => format!("launched {argv:?}"),
                        Err(refusal) => format!("the driver said: {refusal}"),
                    };
                    format!(
                        "{sealing:?} {} {final_command}",
                        spawn.launch_record()["expected"]["template"]
                    )
                }
                Err(refusal) => refusal,
            };
            (observed != expected)
                .then(|| format!("row {label}:\n  left:  {observed}\n  right: {expected}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Rebuild unit 5c-fix-b (chief R1, scenario 1): EVERY template
/// contribution of an agent-backed seat is judged, before the expectation
/// and at the seal, not only its driver template. On a Claude adapter whose
/// driver emits no template, the seat compiles and seals `none`. Then, one
/// fact moved per row: a permission mode ADDED behind the pins, a pin
/// ALTERED into a permission control, or the driver made to CONTRADICT
/// the `none` expectation — in the composed spawn after the expectation,
/// or in the candidate's composition before it — is refused at the seal,
/// against the template the composition records from its adapter's
/// declaration (rebuild unit 5c-fix2, which removed the interim agent-arm
/// refusal); so is that declaration moved while the emission is not. A
/// legitimate pin moved is not a template. Last, the OMISSION: an
/// expectation that records a template the spawn does not emit refuses,
/// and every refusal seals nothing.
#[test]
fn every_template_contribution_of_an_agent_backed_seat_is_judged_before_the_seal() {
    use brokkr_protocol::native_controls::{flatten, Origin, Segment, TemplateExpectation};
    use brokkr_runtime::agents::Lowering;
    use brokkr_runtime::bundle::SiteFacts;
    use brokkr_runtime::engine::SiteSpawn;
    let operator = Operator::new();
    agent_backed_solo(&operator, "high");
    let context = CapabilityContext::no_grants("private", operator.root());
    let adapters = copied_adapters();
    let root = std::fs::canonicalize(adapters.path()).unwrap();
    untemplated_claude(&root);
    let bundle = solo_bundle(&operator, &root, &context).unwrap();
    let site = &bundle.sites["work"];
    let strings = |parts: &[&str]| {
        parts
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>()
    };
    let bypass = strings(&["--permission-mode", "bypassPermissions"]);
    let acceptance = strings(&["--permission-mode", "acceptEdits"]);
    // The compiled spawn's segments, so every row is read against them.
    let (compiled, _) = sealing(&bundle, "work", 0, site);
    assert_eq!(
        json!(
            compiled
                .segments
                .iter()
                .map(|segment| json!([segment.origin.word(), segment.argv]))
                .collect::<Vec<_>>()[1..]
        ),
        json!([
            ["template", ["--model", "claude-opus-5-5"]],
            ["template", ["--effort", "high"]],
        ])
    );
    // Replace segment `at` of the spawn, its argv with it.
    let replaced = |at: usize, segment: Segment| {
        move |spawn: &mut SiteSpawn| {
            let start: usize = spawn.segments[..at]
                .iter()
                .map(|segment| segment.argv.len())
                .sum();
            let end = start + spawn.segments[at].argv.len();
            spawn.argv.splice(start..end, segment.argv.iter().cloned());
            spawn.segments[at] = segment;
        }
    };
    // The site's facts with the first candidate's composition moved, its
    // flat argv following it as the resolver would have produced it.
    let composed = |edit: &dyn Fn(&mut Vec<Segment>)| {
        let mut facts: SiteFacts = site.clone();
        let link = &mut facts.chain[0];
        let Lowering::Composed(composition) = &mut link.lowering else {
            panic!("the plain office composes");
        };
        edit(&mut composition.segments);
        link.argv = flatten(&composition.segments);
        facts
    };
    let driver_with = |tail: &[String]| {
        let mut argv = compiled.segments[0].argv.clone();
        argv.extend(tail.iter().cloned());
        Segment::new(Origin::Template, &argv)
    };
    type Row = (
        &'static str,
        SiteFacts,
        Box<dyn FnOnce(&mut SiteSpawn)>,
        Result<(), String>,
    );
    let contradicted = || Err(TEMPLATE_CONTRADICTED.to_string());
    // The site's facts with the first candidate's recorded declaration
    // moved and its composed segments untouched.
    let declaring = |template: TemplateExpectation| {
        let mut facts: SiteFacts = site.clone();
        let Lowering::Composed(composition) = &mut facts.chain[0].lowering else {
            panic!("the plain office composes");
        };
        composition.template = template;
        facts
    };
    let rows: Vec<Row> = vec![
        ("as compiled", site.clone(), Box::new(|_| {}), Ok(())),
        (
            "addition: a permission mode appended to the spawn",
            site.clone(),
            Box::new({
                let bypass = bypass.clone();
                move |spawn: &mut SiteSpawn| {
                    spawn.argv.extend(bypass.iter().cloned());
                    spawn.segments.push(Segment::new(Origin::Template, &bypass));
                }
            }),
            contradicted(),
        ),
        (
            "addition: a permission mode appended to the composition",
            composed(&|segments| segments.push(Segment::new(Origin::Template, &bypass))),
            Box::new(|_| {}),
            contradicted(),
        ),
        (
            "alteration: the model pin's flag made a permission mode in the spawn",
            site.clone(),
            Box::new(replaced(
                1,
                Segment::new(Origin::Template, &strings(&["--permission-mode", "plan"])),
            )),
            contradicted(),
        ),
        (
            "alteration: the effort pin's value made a permission mode in the composition",
            composed(&|segments| {
                segments[2] = Segment::new(
                    Origin::Template,
                    &strings(&["--effort", "--permission-mode=plan"]),
                )
            }),
            Box::new(|_| {}),
            contradicted(),
        ),
        (
            "alteration: a legitimate pin's model moved in the spawn is not a template",
            site.clone(),
            Box::new(replaced(
                1,
                Segment::new(Origin::Template, &strings(&["--model", "claude-sonnet-5"])),
            )),
            Ok(()),
        ),
        (
            "contradiction: the driver emits the shipped template in the spawn",
            site.clone(),
            Box::new(replaced(0, driver_with(&acceptance))),
            contradicted(),
        ),
        (
            "contradiction: the driver emits the shipped template in the composition",
            composed(&|segments| segments[0] = driver_with(&acceptance)),
            Box::new(|_| {}),
            contradicted(),
        ),
        (
            "contradiction: the composition's recorded declaration made the shipped template",
            declaring(TemplateExpectation::Declared(acceptance.clone())),
            Box::new(|_| {}),
            contradicted(),
        ),
    ];
    assert_eq!(rows.len(), 9);
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(|(label, facts, moved, expected)| {
            let (spawn, observed) = sealing_moved(&bundle, "work", 0, &facts, moved);
            let recorded = match &expected {
                Ok(()) => json!({"kind": "none"}),
                Err(_) => Value::Null,
            };
            let record = spawn.launch_record();
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

    // Omission: an expectation recording a template this spawn does not
    // emit refuses, and the refusal clears the record sealed before it.
    let (mut spawn, sealed) = sealing(&bundle, "work", 0, site);
    let mut expected = spawn.record.clone().map(|record| record.expected).unwrap();
    expected.template = TemplateExpectation::Declared(acceptance.clone());
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

/// Rebuild unit 5c-fix2 (operator ruling 2 of 2026-09-23; the ruling of
/// 2026-09-24, item 2): an agent-backed seat on the SHIPPED Claude adapter,
/// whose composition emits `--permission-mode acceptEdits` behind its verb,
/// seals with exactly that declaration recorded, and the whole final Claude
/// command carries it. Then, one fact moved per row, the seal refuses whole
/// and seals nothing: the template OMITTED from the spawn's driver segment
/// or RELABELLED out of the engine's origin, ALTERED to another mode, a
/// second mode ADDED behind the pins, or the composition's recorded
/// declaration moved to `none` or to another mode while the emission is not.
#[test]
fn an_agent_backed_claude_seat_seals_its_declared_template_and_refuses_a_contradiction() {
    use brokkr_protocol::native_controls::{Origin, Segment, TemplateExpectation};
    use brokkr_runtime::agents::Lowering;
    use brokkr_runtime::bundle::SiteFacts;
    use brokkr_runtime::engine::SiteSpawn;
    let operator = Operator::new();
    agent_backed_solo(&operator, "high");
    let context = CapabilityContext::no_grants("private", operator.root());
    let adapters = copied_adapters();
    let root = std::fs::canonicalize(adapters.path()).unwrap();
    let bundle = solo_bundle(&operator, &root, &context).unwrap();
    let site = &bundle.sites["work"];
    let strings = |parts: &[&str]| {
        parts
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>()
    };
    let acceptance = strings(&["--permission-mode", "acceptEdits"]);
    let bypass = strings(&["--permission-mode", "bypassPermissions"]);
    assert_eq!(
        launch(&bundle, "work", 0),
        [
            "claude",
            "-p",
            "--output-format",
            "stream-json",
            "--verbose",
            "--permission-mode",
            "acceptEdits",
            "--model",
            "claude-opus-5-5",
            "--effort",
            "high",
            "--disallowedTools",
            "WebFetch,WebSearch"
        ]
    );
    // The compiled spawn's driver segment: the verb, then the template.
    let (compiled, _) = sealing(&bundle, "work", 0, site);
    let driver = compiled.segments[0].argv.clone();
    let (verb, template) = driver.split_at(driver.len() - 2);
    assert_eq!(
        (compiled.segments[0].origin, &verb[1..], template),
        (
            Origin::Template,
            &strings(&["driver", "claude", "--"])[..],
            &acceptance[..]
        )
    );
    // Replace segment `at` of the spawn, its argv with it.
    let replaced = |at: usize, segment: Segment| {
        move |spawn: &mut SiteSpawn| {
            let start: usize = spawn.segments[..at]
                .iter()
                .map(|segment| segment.argv.len())
                .sum();
            let end = start + spawn.segments[at].argv.len();
            spawn.argv.splice(start..end, segment.argv.iter().cloned());
            spawn.segments[at] = segment;
        }
    };
    // The site's facts with the candidate's recorded declaration moved.
    let declaring = |template: TemplateExpectation| {
        let mut facts: SiteFacts = site.clone();
        let Lowering::Composed(composition) = &mut facts.chain[0].lowering else {
            panic!("the plain office composes");
        };
        composition.template = template;
        facts
    };
    let contradicted = || Err(TEMPLATE_CONTRADICTED.to_string());
    type Row = (
        &'static str,
        SiteFacts,
        Box<dyn FnOnce(&mut SiteSpawn)>,
        Result<(), String>,
    );
    let rows: Vec<Row> = vec![
        ("as compiled", site.clone(), Box::new(|_| {}), Ok(())),
        (
            "omission: the template dropped from the spawn's driver segment",
            site.clone(),
            Box::new(replaced(0, Segment::new(Origin::Template, verb))),
            contradicted(),
        ),
        (
            "omission: the driver segment relabelled out of the engine's origin",
            site.clone(),
            Box::new(replaced(0, Segment::new(Origin::Authored, &driver))),
            contradicted(),
        ),
        (
            "alteration: the template's mode changed in the spawn",
            site.clone(),
            Box::new(replaced(
                0,
                Segment::new(Origin::Template, &[verb, &bypass[..]].concat()),
            )),
            contradicted(),
        ),
        (
            "addition: a second permission mode appended to the spawn",
            site.clone(),
            Box::new({
                let bypass = bypass.clone();
                move |spawn: &mut SiteSpawn| {
                    spawn.argv.extend(bypass.iter().cloned());
                    spawn.segments.push(Segment::new(Origin::Template, &bypass));
                }
            }),
            contradicted(),
        ),
        (
            "contradiction: the recorded declaration made none",
            declaring(TemplateExpectation::None),
            Box::new(|_| {}),
            contradicted(),
        ),
        (
            "contradiction: the recorded declaration made another mode",
            declaring(TemplateExpectation::Declared(bypass.clone())),
            Box::new(|_| {}),
            contradicted(),
        ),
    ];
    assert_eq!(rows.len(), 7);
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(|(label, facts, moved, expected)| {
            let (spawn, observed) = sealing_moved(&bundle, "work", 0, &facts, moved);
            let recorded = match &expected {
                Ok(()) => json!({"kind": "declared", "argv": acceptance}),
                Err(_) => Value::Null,
            };
            let record = spawn.launch_record();
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
    let never_declared = "dispatch refused: the inline site's lowered restriction carries no \
                          recorded declaration of its adapter's permission template, so no launch \
                          record can be sealed for this site; a record is sealed from typed facts \
                          and never repaired into a default one (decision 0065 slice one, design \
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

/// Rebuild unit 5d: a solo bundle whose work seat and gate are both inline
/// Codex commands pinned to the shipped model and effort, with no authored
/// sandbox, declaring `work` and `gate` as their typed sandbox classes.
fn inline_codex_seats(operator: &Operator, work: &str, gate: &str) {
    one_inline_seat(operator, &CODEX_SEAT[..8]);
    let path = operator.root().join("solo/bundle.json");
    let mut bundle: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    bundle["seats"]["work"]["tools"] = json!({"sandbox": work});
    bundle["seats"]["review"] = json!({"results": ["clean"], "role": "roles/role.md",
        "class": "gate", "driver": {"command": &CODEX_SEAT[..8]},
        "tools": {"sandbox": gate}});
    write(operator.root(), "solo/bundle.json", &bundle);
}

/// One compiled inline site of `class`, composed from `facts` as dispatch
/// composes it, with `moved` applied to the spawn before it is sealed as
/// dispatch seals it; beside the seal, the record the driver is handed, the
/// final Codex command the shipped driver composes from that spawn, and the
/// result door the engine selects for the site.
fn inline_codex_sealing(
    bundle: &Bundle,
    label: &str,
    class: brokkr_runtime::SeatClass,
    facts: &brokkr_runtime::bundle::SiteFacts,
    moved: impl FnOnce(&mut brokkr_runtime::engine::SiteSpawn),
) -> (
    Result<(), String>,
    Value,
    Result<Vec<String>, String>,
    String,
) {
    inline_codex_launching(bundle, label, class, facts, moved, |_| {})
}

/// [`inline_codex_sealing`], with `handed` applied to the input the
/// dispatch door is handed after the engine wrote it (rebuild unit
/// 5d-fix-b): the seat, the native plan, the result path and the door, as
/// dispatch writes them.
fn inline_codex_launching(
    bundle: &Bundle,
    label: &str,
    class: brokkr_runtime::SeatClass,
    facts: &brokkr_runtime::bundle::SiteFacts,
    moved: impl FnOnce(&mut brokkr_runtime::engine::SiteSpawn),
    handed_as: impl FnOnce(&mut Value),
) -> (
    Result<(), String>,
    Value,
    Result<Vec<String>, String>,
    String,
) {
    use brokkr_runtime::engine::{expected_state, result_door, verify_record, LAUNCH_RECORD};
    let outcome = &facts.capabilities.as_ref().unwrap().outcomes[0];
    let SeatBody::Single { command, .. } = &bundle.seats[label].body else {
        panic!("{label} is a single seat");
    };
    let mut spawn = brokkr_runtime::engine::compose_site_at(
        Some(facts),
        brokkr_runtime::engine::BuiltBoundary::Harness,
        class,
        command.clone(),
        None,
        None,
        Path::new("/w"),
        &[],
        "/w/result.json",
        None,
    );
    assert_eq!(spawn.refusal, None, "{label}");
    moved(&mut spawn);
    let sealing =
        expected_state(outcome, None, Some(facts)).and_then(|expected| spawn.seal(expected));
    let record = spawn.launch_record();
    let gate = class == brokkr_runtime::SeatClass::Gate;
    let door = result_door(Boundary::Harness, gate, Some(facts), None).word();
    // Handed as dispatch hands it: the seat, the native plan, the result
    // path the spawn was composed with, and the door `mark_delivery` writes
    // where it is the capture.
    let mut handed = json!({LAUNCH_RECORD: record, "seat": label,
                            "native_controls": outcome.controls(),
                            "result_path": "/w/result.json"});
    if door == "last-message" {
        handed["result_delivery"] = json!(door);
    }
    handed_as(&mut handed);
    let launched = verify_record(&spawn, &handed).and_then(|()| {
        let argv = &spawn.argv;
        let extra = &argv[argv.iter().position(|part| part == "--").unwrap() + 1..];
        // The driver is handed what the door admitted — the record, the
        // result path and door, and a plan admitted as absent stays absent
        // (unit 5d-fix-c1) — so it judges the final command it composes
        // (unit 5d-fix-c2).
        let mut input = handed.clone();
        input["workdir"] = json!("/w");
        input["launch_arguments"] = spawn.launch_arguments();
        brokkr_protocol::adapters::codex_command("codex", extra, "/w", None, &input)
    });
    (sealing, record, launched, door.to_string())
}

/// Rebuild unit 5d (operator ruling of 2026-09-25, "narrow"; rulings 1 and 2
/// of 2026-09-23): an inline Codex work seat declaring `workspace-write` and
/// an inline Codex gate declaring `read-only`, neither with an authored
/// sandbox, compile on the shipped adapters. The engine appends the
/// adapter's fragment for each seat's class behind the authored command as
/// its own `local` segment, the gate's with the result path filled in; the
/// sealed record carries both origins and the typed class; and the whole
/// final Codex command carries the engine's class beside the native OFF.
/// The gate's result reaches the engine through the last-message door —
/// the harness's capture of the final message into the result path — and
/// the work seat's through the file it writes.
#[test]
fn an_inline_codex_work_seat_and_gate_reach_their_final_commands_with_the_engines_class() {
    use brokkr_runtime::SeatClass;
    let operator = Operator::new();
    inline_codex_seats(&operator, "workspace-write", "read-only");
    let context = CapabilityContext::no_grants("private", operator.root());
    let bundle = solo_bundle(&operator, &workspace().join("adapters"), &context).unwrap();
    let observed = |label: &str, class: SeatClass| {
        let (sealing, record, launched, door) =
            inline_codex_sealing(&bundle, label, class, &bundle.sites[label], |_| {});
        json!({
            "sealed": format!("{sealing:?}"),
            "segments": record["segments"],
            "local": record["expected"]["local"],
            "template": record["expected"]["template"],
            "final": launched.map_err(|refusal| format!("refused: {refusal}")),
            "door": door,
        })
    };
    let pins = ["--model", "gpt-6-astra", "--effort", "high"];
    let expected = |segment: &[&str], class: &str, last: &[&str], door: &str| {
        let mut last_argv = vec![
            "codex",
            "exec",
            "--json",
            "-C",
            "/w",
            "-c",
            "model_reasoning_effort=\"high\"",
            "--model",
            "gpt-6-astra",
        ];
        last_argv.extend(last);
        last_argv.extend(OFF);
        json!({
            "sealed": "Ok(())",
            "segments": [{"origin": "authored", "argv": pins},
                         {"origin": "local", "argv": segment}],
            "local": {"allow": {"kind": "unspecified"}, "sandbox": {"kind": class},
                      "application": {"kind": "unrestricted"}},
            "template": {"kind": "none"},
            "final": {"Ok": last_argv},
            "door": door,
        })
    };
    let gate_fragment = [
        "--sandbox",
        "read-only",
        "--output-last-message",
        "/w/result.json",
    ];
    assert_eq!(
        json!({"work": observed("work", SeatClass::Work),
               "review": observed("review", SeatClass::Gate)}),
        json!({
            "work": expected(
                &["--sandbox", "workspace-write"],
                "workspace-write",
                &["--sandbox", "workspace-write"],
                "file",
            ),
            "review": expected(&gate_fragment, "read-only", &gate_fragment, "last-message"),
        })
    );
}

/// Rebuild unit 5d (operator ruling 2 of 2026-09-23): at the
/// production-compiled inline Codex work seat, the expected state records
/// the class the site declared from the compiler's typed facts, and the
/// seal reads the engine's own `local` segments back under the codex
/// grammar: a class altered, omitted, added or relabelled on its way into
/// the command refuses whole and seals nothing, as does a contradiction
/// between the declared and the lowered class.
#[test]
fn an_inline_codex_seal_whose_emitted_class_contradicts_the_declared_one_refuses() {
    use brokkr_protocol::native_controls::{Origin, Segment};
    use brokkr_runtime::agents::Sandbox;
    use brokkr_runtime::bundle::SiteFacts;
    use brokkr_runtime::engine::SiteSpawn;
    let operator = Operator::new();
    inline_codex_seats(&operator, "workspace-write", "read-only");
    let context = CapabilityContext::no_grants("private", operator.root());
    let bundle = solo_bundle(&operator, &workspace().join("adapters"), &context).unwrap();
    let site = &bundle.sites["work"];
    let facts = |edit: &dyn Fn(&mut SiteFacts)| {
        let mut facts = site.clone();
        edit(&mut facts);
        facts
    };
    let lowering = |edit: &dyn Fn(&mut brokkr_runtime::bundle::InlineSandbox)| {
        facts(&|facts| edit(facts.inline_sandbox.as_mut().unwrap()))
    };
    let argv = |parts: &[&str]| {
        parts
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>()
    };
    let contradicted = || {
        Err(
            "dispatch refused: the sandbox class this spawn's own `local` segments express is not \
             the one its expected state records from the site's typed declaration; a class \
             omitted, altered or added on its way into the command is never sealed as the \
             engine's (operator ruling 2 of 2026-09-23; ruling of 2026-09-25, inline Codex \
             sandbox classes; rebuild unit 5d)"
                .to_string(),
        )
    };
    let unlowered = || {
        Err(
            "dispatch refused: the inline site declares a typed local restriction no inline \
             command lowers, so no launch record can be sealed for this site; a record is sealed \
             from typed facts and never repaired into a default one (decision 0065 slice one, \
             design D5.7)"
                .to_string(),
        )
    };
    type Row = (
        &'static str,
        SiteFacts,
        Box<dyn FnOnce(&mut SiteSpawn)>,
        Result<(), String>,
    );
    let rows: Vec<Row> = vec![
        ("as compiled", site.clone(), Box::new(|_| {}), Ok(())),
        (
            "the emitted class altered",
            lowering(&|lowered| lowered.segment.argv = argv(&["--sandbox", "read-only"])),
            Box::new(|_| {}),
            contradicted(),
        ),
        (
            "the emitted class widened",
            lowering(&|lowered| lowered.segment.argv = argv(&["--sandbox", "danger-full-access"])),
            Box::new(|_| {}),
            contradicted(),
        ),
        (
            "the emitted segment emptied",
            lowering(&|lowered| lowered.segment.argv = Vec::new()),
            Box::new(|_| {}),
            contradicted(),
        ),
        (
            "the emitted segment relabelled as authored",
            lowering(&|lowered| lowered.segment.origin = Origin::Authored),
            Box::new(|_| {}),
            contradicted(),
        ),
        (
            "the local segment relabelled as authored in the spawn",
            site.clone(),
            Box::new(|spawn: &mut SiteSpawn| spawn.segments[1].origin = Origin::Authored),
            contradicted(),
        ),
        (
            "a second local class added to the spawn",
            site.clone(),
            Box::new(move |spawn: &mut SiteSpawn| {
                let added = Segment::new(Origin::Local, &argv(&["-s", "read-only"]));
                spawn.argv.extend(added.argv.iter().cloned());
                spawn.segments.push(added);
            }),
            Err(
                "dispatch refused: the engine's own `local` segments of this spawn cannot be \
                 read under the 'codex' grammar (argument 3: it repeats option '--sandbox', \
                 which the grammar admits once; a CLI that resolves a duplicate last-wins would \
                 resolve it against the control the engine composed), so the sandbox class they \
                 express cannot be checked against its expected state; an unreadable contribution \
                 is never sealed as the engine's (operator ruling 2 of 2026-09-23; rebuild unit \
                 5d)"
                .to_string(),
            ),
        ),
        (
            "the lowering recording another class",
            lowering(&|lowered| lowered.class = Sandbox::ReadOnly),
            Box::new(|_| {}),
            unlowered(),
        ),
        (
            "the lowering never recorded",
            facts(&|facts| facts.inline_sandbox = None),
            Box::new(|_| {}),
            unlowered(),
        ),
    ];
    assert_eq!(rows.len(), 9);
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(|(label, facts, moved, expected)| {
            let (observed, record, _, _) = inline_codex_sealing(
                &bundle,
                "work",
                brokkr_runtime::SeatClass::Work,
                &facts,
                moved,
            );
            // A refused seal leaves no record behind; an admitted one
            // records the declared class.
            let recorded = match &expected {
                Ok(()) => json!({"kind": "workspace-write"}),
                Err(_) => Value::Null,
            };
            let observed_record = match record.is_null() {
                true => Value::Null,
                false => record["expected"]["local"]["sandbox"].clone(),
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
}

/// Rebuild unit 5d-fix (chief F1 and F2 of run 0065-rebuild-unit-5d-see-the-uni-5b7d59c1;
/// operator ruling 2 of 2026-09-23; ruling of 2026-09-25): at the final
/// launch of the production-compiled inline Codex seats, every contribution
/// is read back under the codex grammar — a template the compiler recorded,
/// the author's command as it reaches the spawn and the engine's own
/// fragment — and every competing sandbox, approval, root, load or
/// configuration effect beside the engine's one class refuses, in every
/// spelling. The gate's capture is held to exactly the result path handed
/// over, in the engine's fragment, and a work seat to none. Rebuild unit
/// 5d-fix-b moved this judgment whole to the dispatch door, where the native
/// plan is known, so each refusal is the launch's, and bound the door to the
/// admitted class: file delivery at a gate refuses with or without its
/// capture (chief F4).
#[test]
fn an_inline_codex_launch_refuses_every_competing_contribution_and_every_misbound_capture() {
    use brokkr_protocol::native_controls::{Origin, Segment, TemplateExpectation};
    use brokkr_runtime::agents::ResultDoor;
    use brokkr_runtime::bundle::{InlineSandbox, SiteFacts};
    use brokkr_runtime::engine::SiteSpawn;
    use brokkr_runtime::SeatClass;
    let operator = Operator::new();
    inline_codex_seats(&operator, "workspace-write", "read-only");
    let context = CapabilityContext::no_grants("private", operator.root());
    let bundle = solo_bundle(&operator, &workspace().join("adapters"), &context).unwrap();
    let argv = |parts: &[&str]| {
        parts
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>()
    };
    let facts = |label: &str, edit: &dyn Fn(&mut SiteFacts)| {
        let mut facts = bundle.sites[label].clone();
        edit(&mut facts);
        facts
    };
    // A template the compiler would have recorded from an adapter's driver
    // tail: declared and emitted alike, so the template seal agrees.
    let templated = |tail: &[&str]| {
        let tail = argv(tail);
        facts("work", &|facts| {
            facts.inline_template = Some(Segment::new(Origin::Template, &tail));
            facts.declared_template = Some(TemplateExpectation::Declared(tail.clone()));
        })
    };
    let lowered = |label: &str, edit: &dyn Fn(&mut InlineSandbox)| {
        facts(label, &|facts| edit(facts.inline_sandbox.as_mut().unwrap()))
    };
    // Tokens the author's command carries into the spawn, behind its pins.
    let authored = |tokens: &[&str]| -> Box<dyn FnOnce(&mut SiteSpawn)> {
        let tokens = argv(tokens);
        Box::new(move |spawn: &mut SiteSpawn| {
            let at = spawn.segments[0].argv.len();
            spawn.segments[0].argv.extend(tokens.iter().cloned());
            spawn.argv.splice(at..at, tokens);
        })
    };
    let untouched = || -> Box<dyn FnOnce(&mut SiteSpawn)> { Box::new(|_| {}) };
    let rest = "the launch admits only the engine's one sandbox fragment of the site's class, at \
                a gate the engine's one capture into the result path it owns, and configuration \
                on a closed allowlist, so every other effect is refused rather than reconciled or \
                ordered, whoever composed it";
    let permission = "a permission control, which sets, lifts or replaces the sandbox or its \
                      approvals";
    let root = "a root selector, which moves the root the sandbox class is measured from";
    let capture = |cause: String| {
        format!(
            "launch refused: dispatch refused: {cause} (decision 0046 ruling 4; operator ruling \
             2 of 2026-09-23; ruling of 2026-09-25; rebuild unit 5d-fix-b)"
        )
    };
    let carrying = |seat: &str, origin: &str, canonical: &str, at: usize, effect: &str| {
        capture(format!(
            "the inline Codex launch of seat '{seat}' carries '{canonical}' (argument {at}) in \
             its `{origin}` contribution, {effect}; {rest}"
        ))
    };
    let competing = |origin: &str, canonical: &str, at: usize, effect: &str| {
        carrying("work", origin, canonical, at, effect)
    };
    let table = |name: &str| {
        format!(
            "a configuration assignment that assigns into the '{name}' configuration, which is \
             outside the closed set of keys an inline Codex launch admits"
        )
    };
    let unreadable = |at: usize, label: &str, cause: &str| {
        capture(format!(
            "the inline Codex launch of seat 'work' cannot be read whole under the 'codex' \
             grammar (argument {at}, {label}: it {cause}), so none of its effects can be judged; \
             an unclassified option is refused, never passed through"
        ))
    };
    let repeated = "repeats option '--sandbox', which the grammar admits once; a CLI that \
                    resolves a duplicate last-wins would resolve it against the control the \
                    engine composed";
    let elsewhere = "a result capture other than the engine's own into exactly the result path \
                     it owns, a harness write path outside the result sink";
    let misdirected = carrying("review", "local", "--output-last-message", 7, elsewhere);
    let unowned = competing(
        "local",
        "--output-last-message",
        7,
        "a result capture at a work seat, whose result is the file the seat writes; a capture \
         the engine does not own is a harness write path outside the result sink",
    );
    let uncaptured = capture(
        "the inline Codex launch of seat 'review' is a gate's, and no contribution carries the \
         engine's capture into the result path it owns, which the last-message door needs, so \
         the gate's result could not be delivered"
            .to_string(),
    );
    let door = |seat: &str, class: &str, kind: &str, named: &str| {
        capture(format!(
            "the inline Codex launch of seat '{seat}' is admitted '{class}', a {kind}, but its \
             input names the {named} result door; the door follows the admitted class, a gate's \
             result reaching the engine only through the last-message door and a work seat's \
             only through the file it writes"
        ))
    };
    type Row = (
        &'static str,
        &'static str,
        SiteFacts,
        Box<dyn FnOnce(&mut SiteSpawn)>,
        String,
    );
    type Moved = Box<dyn FnOnce(&mut SiteSpawn)>;
    let work = |facts: SiteFacts, moved: Moved, expected: String| ("work", facts, moved, expected);
    let rows: Vec<Row> = vec![
        (
            "work as compiled",
            work(facts("work", &|_| {}), untouched(), "launched".into()),
        ),
        (
            "template --dangerously-bypass-approvals-and-sandbox",
            work(
                templated(&["--dangerously-bypass-approvals-and-sandbox"]),
                untouched(),
                competing(
                    "template",
                    "--dangerously-bypass-approvals-and-sandbox",
                    5,
                    permission,
                ),
            ),
        ),
        (
            "template --full-auto",
            work(
                templated(&["--full-auto"]),
                untouched(),
                competing("template", "--full-auto", 5, permission),
            ),
        ),
        (
            "template --add-dir",
            work(
                templated(&["--add-dir", "/x"]),
                untouched(),
                competing(
                    "template",
                    "--add-dir",
                    5,
                    "a writable root beyond the sandbox class's reach",
                ),
            ),
        ),
        (
            "template -a",
            work(
                templated(&["-a", "never"]),
                untouched(),
                competing("template", "--ask-for-approval", 5, permission),
            ),
        ),
        (
            "template -c sandbox_mode",
            work(
                templated(&["-c", "sandbox_mode=\"danger-full-access\""]),
                untouched(),
                competing("template", "--config", 5, &table("sandbox_mode")),
            ),
        ),
        (
            "template -c attached sandbox_workspace_write",
            work(
                templated(&["-csandbox_workspace_write.network_access=true"]),
                untouched(),
                competing("template", "--config", 5, &table("sandbox_workspace_write")),
            ),
        ),
        (
            "template --config= approval_policy",
            work(
                templated(&["--config=approval_policy=\"never\""]),
                untouched(),
                competing("template", "--config", 5, &table("approval_policy")),
            ),
        ),
        (
            "template --cd",
            work(
                templated(&["--cd", "/elsewhere"]),
                untouched(),
                competing("template", "--cd", 5, root),
            ),
        ),
        (
            "authored -C",
            work(
                facts("work", &|_| {}),
                authored(&["-C", "/elsewhere"]),
                competing("authored", "--cd", 5, root),
            ),
        ),
        (
            "template -s beside the engine's class",
            work(
                templated(&["-s", "read-only"]),
                untouched(),
                unreadable(7, "'--sandbox'", repeated),
            ),
        ),
        (
            "authored --full-auto",
            work(
                facts("work", &|_| {}),
                authored(&["--full-auto"]),
                competing("authored", "--full-auto", 5, permission),
            ),
        ),
        (
            "authored --add-dir=",
            work(
                facts("work", &|_| {}),
                authored(&["--add-dir=/x"]),
                competing(
                    "authored",
                    "--add-dir",
                    5,
                    "a writable root beyond the sandbox class's reach",
                ),
            ),
        ),
        (
            "authored -p",
            work(
                facts("work", &|_| {}),
                authored(&["-p", "x"]),
                competing(
                    "authored",
                    "--profile",
                    5,
                    "a configuration document the engine cannot see into, which can set the \
                     sandbox",
                ),
            ),
        ),
        (
            "authored -c unbounded",
            work(
                facts("work", &|_| {}),
                authored(&["-c", "unmodelled.key=1"]),
                competing(
                    "authored",
                    "--config",
                    5,
                    "a configuration assignment that assigns a key outside the closed set an \
                     inline Codex launch admits",
                ),
            ),
        ),
        (
            "authored --sandbox= beside the engine's class",
            work(
                facts("work", &|_| {}),
                authored(&["--sandbox=read-only"]),
                unreadable(6, "'--sandbox'", repeated),
            ),
        ),
        (
            "authored bare word",
            work(
                facts("work", &|_| {}),
                authored(&["stray"]),
                unreadable(
                    5,
                    "a positional argument, whose text is not echoed",
                    "is a bare word, and no positional argument is part of the supported shape",
                ),
            ),
        ),
        (
            "local -a beside the class",
            work(
                lowered("work", &|lowered| {
                    lowered.segment.argv = argv(&["--sandbox", "workspace-write", "-a", "never"])
                }),
                untouched(),
                competing("local", "--ask-for-approval", 7, permission),
            ),
        ),
        (
            "work, a capture in the fragment",
            work(
                lowered("work", &|lowered| {
                    lowered.segment.argv =
                        argv(&["--sandbox", "workspace-write", "-o", "{result_path}"])
                }),
                untouched(),
                unowned.clone(),
            ),
        ),
        (
            "gate as compiled",
            (
                "review",
                facts("review", &|_| {}),
                untouched(),
                "launched".into(),
            ),
        ),
        (
            "gate, capture elsewhere",
            (
                "review",
                lowered("review", &|lowered| {
                    lowered.segment.argv = argv(&["--sandbox", "read-only", "-o", "/elsewhere"])
                }),
                untouched(),
                misdirected.clone(),
            ),
        ),
        (
            "gate, capture retargeted in the spawn",
            (
                "review",
                facts("review", &|_| {}),
                Box::new(|spawn: &mut SiteSpawn| {
                    let at = spawn.argv.len() - 1;
                    spawn.argv[at] = "/w/other.json".to_string();
                    let local = spawn.segments.last_mut().unwrap();
                    *local.argv.last_mut().unwrap() = "/w/other.json".to_string();
                }) as Moved,
                misdirected,
            ),
        ),
        (
            "gate, no capture",
            (
                "review",
                lowered("review", &|lowered| {
                    lowered.segment.argv = argv(&["--sandbox", "read-only"])
                }),
                untouched(),
                uncaptured,
            ),
        ),
        (
            "gate, the capture moved into the template",
            (
                "review",
                facts("review", &|facts| {
                    let tail = argv(&["-o", "/w/result.json"]);
                    facts.inline_template = Some(Segment::new(Origin::Template, &tail));
                    facts.declared_template = Some(TemplateExpectation::Declared(tail));
                    facts.inline_sandbox.as_mut().unwrap().segment.argv =
                        argv(&["--sandbox", "read-only"]);
                }),
                untouched(),
                carrying("review", "template", "--output-last-message", 5, elsewhere),
            ),
        ),
        (
            "gate, file delivery",
            (
                "review",
                lowered("review", &|lowered| lowered.door = ResultDoor::File),
                untouched(),
                door("review", "read-only", "gate", "file"),
            ),
        ),
        // Rebuild unit 5d-fix-b (chief F4): the door and the capture changed
        // together still contradict the admitted gate class.
        (
            "gate, file delivery and no capture",
            (
                "review",
                lowered("review", &|lowered| {
                    lowered.door = ResultDoor::File;
                    lowered.segment.argv = argv(&["--sandbox", "read-only"]);
                }),
                untouched(),
                door("review", "read-only", "gate", "file"),
            ),
        ),
    ]
    .into_iter()
    .map(|(label, (site, facts, moved, expected))| (label, site, facts, moved, expected))
    .collect();
    assert_eq!(rows.len(), 26);
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(|(label, site, facts, moved, expected)| {
            let class = match site {
                "review" => SeatClass::Gate,
                _ => SeatClass::Work,
            };
            let (sealing, record, launched, _) =
                inline_codex_sealing(&bundle, site, class, &facts, moved);
            let observed = match (sealing, launched) {
                (Err(reason), _) => format!("sealing refused: {reason}"),
                (Ok(()), Ok(_)) => "launched".to_string(),
                (Ok(()), Err(reason)) => format!("launch refused: {reason}"),
            };
            // A refused seal leaves no record behind.
            let sealed = !record.is_null();
            let expected_sealed = !expected.starts_with("sealing refused");
            (observed != expected || sealed != expected_sealed).then(|| {
                format!(
                    "row {label}:\n  left:  {observed} (record: {sealed})\n  right: {expected} \
                     (record: {expected_sealed})"
                )
            })
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Rebuild unit 5d-fix-b (chief F1–F5 of run 0065-rebuild-unit-5d-fix-see-the-569be761;
/// operator ruling 2 of 2026-09-23; ruling of 2026-09-25): the dispatch door
/// judges the whole launch the driver is handed, the native plan the input
/// carries included, by the admission's own judgment, and binds the result
/// door to the admitted class. Native sandbox, approval and capture effects,
/// profile and profiles configuration, a key off the allowlist, an option
/// the grammar cannot place, an unreadable plan and a door the class does
/// not admit each refuse before any provider work, naming the seat bounded;
/// the compiled work seat and gate, as handed, launch.
#[test]
fn an_inline_codex_launch_judges_the_native_plan_and_binds_the_door_to_the_class() {
    use brokkr_runtime::SeatClass;
    let operator = Operator::new();
    inline_codex_seats(&operator, "workspace-write", "read-only");
    let context = CapabilityContext::no_grants("private", operator.root());
    let bundle = solo_bundle(&operator, &workspace().join("adapters"), &context).unwrap();
    let rest = "the launch admits only the engine's one sandbox fragment of the site's class, at \
                a gate the engine's one capture into the result path it owns, and configuration \
                on a closed allowlist, so every other effect is refused rather than reconciled or \
                ordered, whoever composed it";
    let refused = |cause: String| {
        format!(
            "refused: dispatch refused: {cause} (decision 0046 ruling 4; operator ruling 2 of \
             2026-09-23; ruling of 2026-09-25; rebuild unit 5d-fix-b)"
        )
    };
    let native_as = |named: &str, canonical: &str, at: usize, effect: &str| {
        refused(format!(
            "the inline Codex launch of seat {named} carries '{canonical}' (argument {at}) in \
             its `native` contribution, {effect}; {rest}"
        ))
    };
    let native = |seat: &str, canonical: &str, at: usize, effect: &str| {
        native_as(&format!("'{seat}'"), canonical, at, effect)
    };
    let unreadable = |seat: &str, at: usize, label: &str, cause: &str| {
        refused(format!(
            "the inline Codex launch of seat '{seat}' cannot be read whole under the 'codex' \
             grammar (argument {at}, {label}: it {cause}), so none of its effects can be judged; \
             an unclassified option is refused, never passed through"
        ))
    };
    let repeats = |option: &str| {
        format!(
            "repeats option '{option}', which the grammar admits once; a CLI that resolves a \
             duplicate last-wins would resolve it against the control the engine composed"
        )
    };
    let table = |name: &str| {
        format!(
            "a configuration assignment that assigns into the '{name}' configuration, which is \
             outside the closed set of keys an inline Codex launch admits"
        )
    };
    let permission = "a permission control, which sets, lifts or replaces the sandbox or its \
                      approvals";
    // The OFF pair the compiled plan carries, then `extra` behind it.
    let plan = |extra: &[&str]| -> Box<dyn FnOnce(&mut Value)> {
        let mut argv = vec!["-c".to_string(), "web_search=\"disabled\"".to_string()];
        argv.extend(extra.iter().map(|part| part.to_string()));
        Box::new(move |handed: &mut Value| handed["native_controls"]["argv"] = json!(argv))
    };
    let handed = |key: &'static str, value: Value| -> Box<dyn FnOnce(&mut Value)> {
        Box::new(move |handed: &mut Value| handed[key] = value)
    };
    type Row = (
        &'static str,
        &'static str,
        Box<dyn FnOnce(&mut Value)>,
        String,
    );
    let rows: Vec<Row> = vec![
        ("work as handed", "work", plan(&[]), "launched".into()),
        ("gate as handed", "review", plan(&[]), "launched".into()),
        (
            "native --sandbox",
            "work",
            plan(&["--sandbox", "danger-full-access"]),
            unreadable("work", 9, "'--sandbox'", &repeats("--sandbox")),
        ),
        (
            "native -c sandbox_mode",
            "work",
            plan(&["-c", "sandbox_mode=\"danger-full-access\""]),
            native("work", "--config", 9, &table("sandbox_mode")),
        ),
        (
            "native -a",
            "work",
            plan(&["-a", "never"]),
            native("work", "--ask-for-approval", 9, permission),
        ),
        (
            "native --dangerously-bypass-approvals-and-sandbox at a gate",
            "review",
            plan(&["--dangerously-bypass-approvals-and-sandbox"]),
            native(
                "review",
                "--dangerously-bypass-approvals-and-sandbox",
                11,
                permission,
            ),
        ),
        (
            "native -o at a work seat",
            "work",
            plan(&["-o", "/elsewhere"]),
            native(
                "work",
                "--output-last-message",
                9,
                "a result capture at a work seat, whose result is the file the seat writes; a \
                 capture the engine does not own is a harness write path outside the result sink",
            ),
        ),
        (
            "native -o at a gate",
            "review",
            plan(&["-o", "/w/result.json"]),
            unreadable(
                "review",
                11,
                "'--output-last-message'",
                &repeats("--output-last-message"),
            ),
        ),
        (
            "native -c profile",
            "work",
            plan(&["-c", "profile=x"]),
            native("work", "--config", 9, &table("profile")),
        ),
        (
            "native -c profiles sandbox_mode",
            "work",
            plan(&["-c", "profiles.x.sandbox_mode=\"danger-full-access\""]),
            native("work", "--config", 9, &table("profiles")),
        ),
        (
            "native -c profiles approval_policy",
            "review",
            plan(&["-c", "profiles.x.approval_policy=\"never\""]),
            native("review", "--config", 11, &table("profiles")),
        ),
        (
            "native -c unknown key",
            "work",
            plan(&["-c", "unmodelled.key=1"]),
            native(
                "work",
                "--config",
                9,
                "a configuration assignment that assigns a key outside the closed set an inline \
                 Codex launch admits",
            ),
        ),
        (
            "native, an unclassified option",
            "work",
            plan(&["--frobnicate"]),
            unreadable("work", 9, "'--frobnicate'", "names no option"),
        ),
        (
            "native plan null",
            "work",
            handed("native_controls", Value::Null),
            // Rebuild unit 5d-fix-c1 (F3): one fixed cause, the reader's
            // never echoed.
            unread_plan("'work'"),
        ),
        (
            "work, the last-message door",
            "work",
            handed("result_delivery", json!("last-message")),
            refused(
                "the inline Codex launch of seat 'work' is admitted 'workspace-write', a work \
                 seat, but its input names the last-message result door; the door follows the \
                 admitted class, a gate's result reaching the engine only through the \
                 last-message door and a work seat's only through the file it writes"
                    .to_string(),
            ),
        ),
        (
            "gate, the door removed",
            "review",
            Box::new(|handed: &mut Value| {
                handed.as_object_mut().unwrap().remove("result_delivery");
            }),
            refused(
                "the inline Codex launch of seat 'review' is admitted 'read-only', a gate, but its \
                 input names the file result door; the door follows the admitted class, a gate's \
                 result reaching the engine only through the last-message door and a work seat's \
                 only through the file it writes"
                    .to_string(),
            ),
        ),
        (
            "a seat that is not a plain label",
            "work",
            Box::new(|handed: &mut Value| {
                handed["seat"] = json!("work seat");
                handed["native_controls"]["argv"] = json!(["-a", "never"]);
            }),
            // Rebuild unit 5d-fix-c1 (F4): named in admission's bounded
            // representation.
            native_as(
                "'work…' (9 bytes, not echoed in full)",
                "--ask-for-approval",
                7,
                permission,
            ),
        ),
    ];
    assert_eq!(rows.len(), 17);
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(|(label, seat, handed_as, expected)| {
            let class = match seat {
                "review" => SeatClass::Gate,
                _ => SeatClass::Work,
            };
            let (sealing, _, launched, _) = inline_codex_launching(
                &bundle,
                seat,
                class,
                &bundle.sites[seat],
                |_| {},
                handed_as,
            );
            let observed = match (sealing, launched) {
                (Err(reason), _) => format!("sealing refused: {reason}"),
                (Ok(()), Ok(_)) => "launched".to_string(),
                (Ok(()), Err(reason)) => format!("refused: {reason}"),
            };
            (observed != expected)
                .then(|| format!("row {label}:\n  left:  {observed}\n  right: {expected}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The dispatch door's refusal of an inline Codex launch, as the launch
/// helpers above report it.
fn door_refused(cause: &str) -> String {
    format!(
        "refused: dispatch refused: {cause} (decision 0046 ruling 4; operator ruling 2 of \
         2026-09-23; ruling of 2026-09-25; rebuild unit 5d-fix-b)"
    )
}

/// Rebuild unit 5d-fix-c1 (F3): the one fixed refusal of a null or
/// unreadable native plan, naming the seat `named` as rendered.
fn unread_plan(named: &str) -> String {
    door_refused(&format!(
        "the native plan of seat {named} is null or cannot be read, so the launch has no \
         capability authority; the reader's cause is not echoed, because it can carry the plan's \
         own text (rebuild unit 5d-fix-c1)"
    ))
}

/// Rebuild unit 5d-fix-c1 (chief F1, F3 and F4 of run
/// 0065-rebuild-unit-5d-fix-b-see-t-8067eebc): where the sealed expectation
/// denies a native power, the dispatch door requires the engine's plan — a
/// missing `native_controls` key and a plan with no argv each refuse — and
/// proves that the delivered launch itself expresses each sealed denial, so a
/// plan whose OFF argv was replaced by another admitted assignment refuses.
/// An unmeasured inventory seals no denial, and its launch without a plan
/// stands. An unreadable plan refuses with one fixed cause, and neither a
/// newline nor a long value it carries reaches the reason. The seat is named
/// in admission's bounded representation: a dotted label keeps its identity,
/// and a long label with a newline is named by its lead and length.
#[test]
fn an_inline_codex_launch_requires_its_native_plan_and_proves_each_sealed_denial() {
    use brokkr_runtime::capabilities::NativePlan;
    use brokkr_runtime::SeatClass;
    let operator = Operator::new();
    inline_codex_seats(&operator, "workspace-write", "read-only");
    let context = CapabilityContext::no_grants("private", operator.root());
    let bundle = solo_bundle(&operator, &workspace().join("adapters"), &context).unwrap();
    let mut unmeasured = bundle.sites["work"].clone();
    unmeasured.capabilities.as_mut().unwrap().outcomes[0].native = NativePlan::Unmeasured {
        declaration: None,
        reason: "unmeasured".to_string(),
    };
    let sentinel = format!("x\n{}", "s".repeat(4096));
    let long_seat = format!("work\n{}", "w".repeat(100));
    let unplanned = |named: &str, what: &str| {
        door_refused(&format!(
            "the inline Codex launch of seat {named} {what}, while its sealed expectation denies \
             1 native power(s); only the plan's OFF argv expresses a denial, so without it the \
             harness would run at its own defaults (decision 0065 ruling 4; rebuild unit \
             5d-fix-c1)"
        ))
    };
    let undenied = |named: &str| {
        door_refused(&format!(
            "the inline Codex launch of seat {named} does not express its sealed native denial \
             of 'web-search'; a denial is something the launch proves, so a plan whose OFF argv \
             was removed or changed is refused, never trusted by its claim (decision 0066; \
             rebuild unit 5d-fix-c1)"
        ))
    };
    let no_plan = || -> Box<dyn FnOnce(&mut Value)> {
        Box::new(|handed: &mut Value| {
            handed.as_object_mut().unwrap().remove("native_controls");
        })
    };
    let argv = |argv: &[&str]| -> Box<dyn FnOnce(&mut Value)> {
        let argv = json!(argv);
        Box::new(move |handed: &mut Value| handed["native_controls"]["argv"] = argv)
    };
    type Row<'a> = (
        &'static str,
        &'static str,
        &'a brokkr_runtime::bundle::SiteFacts,
        Box<dyn FnOnce(&mut Value)>,
        String,
    );
    let (work, review) = (&bundle.sites["work"], &bundle.sites["review"]);
    let effort = ["-c", "model_reasoning_effort=\"high\""];
    let rows: Vec<Row> = vec![
        (
            "work as handed",
            "work",
            work,
            Box::new(|_| {}),
            "launched".into(),
        ),
        (
            "gate as handed",
            "review",
            review,
            Box::new(|_| {}),
            "launched".into(),
        ),
        (
            "work, no plan",
            "work",
            work,
            no_plan(),
            unplanned("'work'", "carries no native plan"),
        ),
        (
            "gate, no plan",
            "review",
            review,
            no_plan(),
            unplanned("'review'", "carries no native plan"),
        ),
        (
            "work, a plan with no argv",
            "work",
            work,
            argv(&[]),
            unplanned("'work'", "carries a native plan with no argv"),
        ),
        (
            "work, the OFF replaced by an admitted effort",
            "work",
            work,
            argv(&effort),
            undenied("'work'"),
        ),
        (
            "gate, the OFF replaced by an admitted effort",
            "review",
            review,
            argv(&effort),
            undenied("'review'"),
        ),
        (
            "unmeasured, no plan",
            "work",
            &unmeasured,
            no_plan(),
            "launched".into(),
        ),
        (
            "an unreadable plan carrying a newline and a long value",
            "work",
            work,
            {
                let sentinel = sentinel.clone();
                Box::new(move |handed: &mut Value| {
                    handed["native_controls"]["inventory"] = json!(sentinel)
                })
            },
            unread_plan("'work'"),
        ),
        (
            "a dotted seat",
            "work",
            work,
            Box::new(|handed: &mut Value| {
                handed["seat"] = json!("work.v1");
                handed["native_controls"]["argv"] = json!(["-a", "never"]);
            }),
            door_refused(
                "the inline Codex launch of seat 'work.v1' carries '--ask-for-approval' \
                 (argument 7) in its `native` contribution, a permission control, which sets, \
                 lifts or replaces the sandbox or its approvals; the launch admits only the \
                 engine's one sandbox fragment of the site's class, at a gate the engine's one \
                 capture into the result path it owns, and configuration on a closed allowlist, \
                 so every other effect is refused rather than reconciled or ordered, whoever \
                 composed it",
            ),
        ),
        (
            "a long seat with a newline",
            "work",
            work,
            {
                let long_seat = long_seat.clone();
                Box::new(move |handed: &mut Value| {
                    handed["seat"] = json!(long_seat);
                    handed.as_object_mut().unwrap().remove("native_controls");
                })
            },
            unplanned(
                "'work…' (105 bytes, not echoed in full)",
                "carries no native plan",
            ),
        ),
    ];
    assert_eq!(rows.len(), 11);
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(|(label, seat, facts, handed_as, expected)| {
            let class = match seat {
                "review" => SeatClass::Gate,
                _ => SeatClass::Work,
            };
            let (sealing, _, launched, _) =
                inline_codex_launching(&bundle, seat, class, facts, |_| {}, handed_as);
            let observed = match (sealing, launched) {
                (Err(reason), _) => format!("sealing refused: {reason}"),
                (Ok(()), Ok(_)) => "launched".to_string(),
                (Ok(()), Err(reason)) => format!("refused: {reason}"),
            };
            let leaked = observed.contains('\n') || observed.contains("ssssssss");
            (observed != expected || leaked).then(|| {
                format!("row {label}:\n  left:  {observed}\n  right: {expected} (leaked: {leaked})")
            })
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Rebuild unit 5d-fix-c2 (chief F1 of run
/// 0065-rebuild-unit-5d-fix-c1-see--b800f52a): at the production-compiled
/// inline Codex seats, a native plan whose OFF key is double- or
/// single-quoted — a key the harness reads literally, leaving search on
/// (codex-cli rust-v0.154.0 config_override.rs and overrides.rs, per the
/// chief) — refuses at the dispatch door, at the work seat and at the gate,
/// and so does a partly quoted dotted key; the canonical OFF launches.
/// Chief F2: the driver judges the final command it composes, so the
/// door-admitted `--effort ultra` refuses once it is translated, and an
/// authored `--json` beside the driver's own refuses as the repeat it is.
#[test]
fn an_inline_codex_launch_reads_its_keys_as_the_harness_does_and_is_judged_as_composed() {
    use brokkr_runtime::engine::SiteSpawn;
    use brokkr_runtime::SeatClass;
    let operator = Operator::new();
    inline_codex_seats(&operator, "workspace-write", "read-only");
    let context = CapabilityContext::no_grants("private", operator.root());
    let bundle = solo_bundle(&operator, &workspace().join("adapters"), &context).unwrap();
    let rest = "the launch admits only the engine's one sandbox fragment of the site's class, at \
                a gate the engine's one capture into the result path it owns, and configuration \
                on a closed allowlist, so every other effect is refused rather than reconciled or \
                ordered, whoever composed it";
    let noncanonical = "a configuration assignment that assigns through a key not spelled \
                        canonically: the harness splits an assignment at its first '=', trims it \
                        and splits the key at every '.', reading a quote or an escape as part of \
                        the name (codex-cli rust-v0.154.0, \
                        codex-rs/utils/cli/src/config_override.rs and \
                        codex-rs/config/src/overrides.rs), so only dot-separated bare names of \
                        ASCII letters, digits, '_' and '-', with nothing around the '=', are read \
                        as the key they spell";
    let at_door = |seat: &str, at: usize| {
        door_refused(&format!(
            "the inline Codex launch of seat '{seat}' carries '--config' (argument {at}) in its \
             `native` contribution, {noncanonical}; {rest}"
        ))
    };
    let composed = |cause: String| {
        format!(
            "refused: refusing to invoke the agent CLI: the final command of this inline Codex \
             launch {cause} (decision 0046 ruling 4; operator ruling of 2026-09-25; rebuild unit \
             5d-fix-c2)"
        )
    };
    let argv = |argv: &[&str]| -> Box<dyn FnOnce(&mut Value)> {
        let argv = json!(argv);
        Box::new(move |handed: &mut Value| handed["native_controls"]["argv"] = argv)
    };
    let as_handed = || -> Box<dyn FnOnce(&mut Value)> { Box::new(|_| {}) };
    // The author's pinned level, or tokens behind its pins, as the spawn
    // carries them.
    let start = |spawn: &SiteSpawn| {
        spawn.argv.len()
            - spawn
                .segments
                .iter()
                .map(|segment| segment.argv.len())
                .sum::<usize>()
    };
    let level = move |level: &'static str| -> Box<dyn FnOnce(&mut SiteSpawn)> {
        Box::new(move |spawn: &mut SiteSpawn| {
            let pin = spawn.segments[0].argv.len() - 1;
            assert_eq!(spawn.segments[0].argv[pin - 1..], ["--effort", "high"]);
            let at = start(spawn) + pin;
            spawn.argv[at] = level.to_string();
            spawn.segments[0].argv[pin] = level.to_string();
        })
    };
    let behind = move |token: &'static str| -> Box<dyn FnOnce(&mut SiteSpawn)> {
        Box::new(move |spawn: &mut SiteSpawn| {
            let at = start(spawn) + spawn.segments[0].argv.len();
            spawn.segments[0].argv.push(token.to_string());
            spawn.argv.insert(at, token.to_string());
        })
    };
    let unmoved = || -> Box<dyn FnOnce(&mut SiteSpawn)> { Box::new(|_| {}) };
    type Row = (
        &'static str,
        &'static str,
        Box<dyn FnOnce(&mut SiteSpawn)>,
        Box<dyn FnOnce(&mut Value)>,
        String,
    );
    let rows: Vec<Row> = vec![
        (
            "work as handed",
            "work",
            unmoved(),
            as_handed(),
            "launched".into(),
        ),
        (
            "gate as handed",
            "review",
            unmoved(),
            as_handed(),
            "launched".into(),
        ),
        (
            "work, the OFF key double-quoted",
            "work",
            unmoved(),
            argv(&["-c", "\"web_search\"=\"disabled\""]),
            at_door("work", 7),
        ),
        (
            "gate, the OFF key double-quoted",
            "review",
            unmoved(),
            argv(&["-c", "\"web_search\"=\"disabled\""]),
            at_door("review", 9),
        ),
        (
            "work, the OFF key single-quoted",
            "work",
            unmoved(),
            argv(&["-c", "'web_search'=\"disabled\""]),
            at_door("work", 7),
        ),
        (
            "gate, the OFF key single-quoted",
            "review",
            unmoved(),
            argv(&["-c", "'web_search'=\"disabled\""]),
            at_door("review", 9),
        ),
        (
            "work, a partly quoted dotted key",
            "work",
            unmoved(),
            argv(&[
                "-c",
                "web_search=\"disabled\"",
                "-c",
                "web_search.\"mode\"=1",
            ]),
            at_door("work", 9),
        ),
        (
            "gate, the canonical OFF joined",
            "review",
            unmoved(),
            argv(&["--config=web_search=disabled"]),
            "launched".into(),
        ),
        (
            "work, --effort ultra admitted at the door and translated",
            "work",
            level("ultra"),
            as_handed(),
            composed(format!(
                "carries '--config' (argument 2) in its `authored` contribution, a configuration \
                 assignment that assigns 'model_reasoning_effort' a value outside the bounded \
                 ones its declaration admits; {rest}"
            )),
        ),
        (
            "work, an authored --json beside the driver's",
            "work",
            behind("--json"),
            as_handed(),
            composed(
                "cannot be read whole as the harness receives it under the 'codex' grammar \
                 (argument 9, '--json': it repeats option '--json', which the grammar admits \
                 once; a CLI that resolves a duplicate last-wins would resolve it against the \
                 control the engine composed), so none of its effects can be judged"
                    .to_string(),
            ),
        ),
    ];
    assert_eq!(rows.len(), 10);
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(|(label, seat, moved, handed_as, expected)| {
            let class = match seat {
                "review" => SeatClass::Gate,
                _ => SeatClass::Work,
            };
            let (sealing, _, launched, _) =
                inline_codex_launching(&bundle, seat, class, &bundle.sites[seat], moved, handed_as);
            let observed = match (sealing, launched) {
                (Err(reason), _) => format!("sealing refused: {reason}"),
                (Ok(()), Ok(_)) => "launched".to_string(),
                (Ok(()), Err(reason)) => format!("refused: {reason}"),
            };
            (observed != expected)
                .then(|| format!("row {label}:\n  left:  {observed}\n  right: {expected}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
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
/// naming the seat — `--search` and the OFF pair itself alike. Operator
/// ruling 1 of 2026-09-23 (rebuild unit 12): the refusal names the
/// canonical option and its position, never the value. The inline seat
/// now declares its class, whose lowering judges its command first, so the
/// authored control is planted on the boxed inline seat, which declares
/// none (fixture migration of 2026-09-26).
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
        (vec!["-c", "web_search=\"disabled\""], "--config"),
        (vec!["-cweb_search=\"disabled\""], "--config"),
        (vec!["-c=web_search=\"disabled\""], "--config"),
        (vec!["--config", "web_search=\"disabled\""], "--config"),
        (vec!["--config=web_search=\"disabled\""], "--config"),
    ] {
        // The sound bundle compiles; then one authored control is added.
        operator
            .compile(&context, Boundary::Namespace, None, None)
            .unwrap();
        let mut bundle: Value = serde_json::from_slice(
            &std::fs::read(operator.root().join("bundle/bundle.json")).unwrap(),
        )
        .unwrap();
        let command = bundle["seats"]["boxed"]["driver"]["command"]
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
                "bundle: seat 'boxed' (office 'boxed') in realm 'private': its arguments carry \
                 '{written}' (argument 5), a capability-bearing option of harness 'codex'. A \
                 recipe authors no capability-bearing option, whatever its value, polarity or \
                 grant: tools come from typed declarations and the realm's grant, composed by \
                 the engine alone (operator ruling 1 of 2026-09-23)"
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
    bundle["seats"]["boxed"]["driver"]["command"]
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
        "bundle: seat 'boxed' (office 'boxed') in realm 'private': its arguments do not \
         parse: the 'codex' command grammar cannot place argument 5 ('--enable'): it names no \
         option. A harness brokkr launches is parsed against a model of its options, and a \
         token that grammar cannot place is refused rather than passed through, because a \
         control nobody can read is a control nobody can rule on (decision 0066 ruling 6)"
    );
}

/// Rebuild unit 12-fix-f, the council's C-E1 (design D6): the site a
/// compiled capability refusal opens with is typed, so an author's seat
/// label is an identity and never spelled. The reviewer's seat
/// `/private/REVIEW_SENTINEL\nwork` was named twice, as seat and office,
/// in a 417-scalar line — through the composition refusal and through the
/// runtime's own refusals alike.
#[test]
fn a_compiled_capability_refusal_names_an_unplain_seat_without_echoing_it() {
    const LABEL: &str = "/private/REVIEW_SENTINEL\nwork";
    const SITE: &str = "seat '…' (29 bytes, not echoed in full) (office '…' (29 bytes, not \
                        echoed in full)) in realm 'private'";
    let operator = Operator::new();
    let context = CapabilityContext::no_grants("private", operator.root());
    operator
        .compile(&context, Boundary::Namespace, None, None)
        .unwrap();
    let sound: Value =
        serde_json::from_slice(&std::fs::read(operator.root().join("bundle/bundle.json")).unwrap())
            .unwrap();
    let policy = std::fs::read_to_string(operator.root().join("bundle/policy.json")).unwrap();
    std::fs::write(
        operator.root().join("bundle/policy.json"),
        policy.replace("\"boxed\"", &serde_json::to_string(LABEL).unwrap()),
    )
    .unwrap();
    let refused = |authored: &[&str], asks: Option<Value>| {
        let mut bundle = sound.clone();
        let mut seat = bundle["seats"]
            .as_object_mut()
            .unwrap()
            .remove("boxed")
            .unwrap();
        seat["driver"]["command"]
            .as_array_mut()
            .unwrap()
            .extend(authored.iter().map(|part| json!(part)));
        if let Some(asks) = asks {
            seat["capabilities"] = asks;
        }
        bundle["seats"][LABEL] = seat;
        write(operator.root(), "bundle/bundle.json", &bundle);
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
        .to_string()
    };
    assert_eq!(
        refused(&["--search"], None),
        format!(
            "bundle: {SITE}: its arguments carry '--search' (argument 5), a \
             capability-bearing option of harness 'codex'. A recipe authors no \
             capability-bearing option, whatever its value, polarity or grant: tools come \
             from typed declarations and the realm's grant, composed by the engine alone \
             (operator ruling 1 of 2026-09-23)"
        )
    );
    assert_eq!(
        refused(&[], Some(json!({"web-search": "requires"}))),
        format!(
            "bundle: {SITE}: requires capability 'web-search' but the realm does not grant it \
             to this office"
        )
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

/// Rebuild unit 6 (task 6.1): the five inline Claude seats of the shipped
/// `fast`, `node` and `preflight` recipes author no capability flag and
/// declare a typed allow instead. Each one, compiled from the shipped
/// directory in a realm that grants nothing and composed and sealed by the
/// engine's own functions, holds nothing; the engine emits the adapter's
/// `acceptEdits` template and the list it lowers, every concrete prefix the
/// recipe used to write — `.venv/bin/pytest` included — in the order it
/// wrote them; and the whole ordered final command ends in Claude's native
/// denial and nothing wider. The literals are the recipes' former authored
/// values, written out here, not derived from the adapter.
#[test]
fn the_shipped_claude_recipes_seat_their_typed_allow_as_the_engines_exact_local_limits() {
    let operator = Operator::new();
    let context = CapabilityContext::no_grants("private", operator.root());
    let fast = (
        ["cargo", "git", "python3", "pytest", "ls", "rg", "mkdir"],
        "Bash(cargo:*),Bash(git:*),Bash(python3:*),Bash(.venv/bin/pytest:*),Bash(ls:*),\
         Bash(rg:*),Bash(mkdir:*)",
    );
    let node = (
        ["npm", "npx", "node", "git", "ls", "rg", "mkdir"],
        "Bash(npm:*),Bash(npx:*),Bash(node:*),Bash(git:*),Bash(ls:*),Bash(rg:*),Bash(mkdir:*)",
    );
    let preflight = (
        &["cargo", "git", "ls", "rg"][..],
        "Bash(cargo:*),Bash(git:*),Bash(ls:*),Bash(rg:*)",
    );
    let seats = [
        ("fast", "implement", &fast.0[..], fast.1),
        ("fast", "review", &fast.0[..], fast.1),
        ("node", "implement", &node.0[..], node.1),
        ("node", "review", &node.0[..], node.1),
        ("preflight", "review", preflight.0, preflight.1),
    ];
    let (mut observed, mut expected) = (serde_json::Map::new(), serde_json::Map::new());
    for (recipe, seat, names, list) in seats {
        let bundle = Bundle::compile_with_capabilities(
            &workspace().join("recipes").join(recipe),
            &workspace().join("agents"),
            &workspace().join("adapters"),
            Some("private"),
            None,
            Boundary::Harness,
            &context,
        )
        .unwrap_or_else(|refusal| panic!("{recipe} must compile: {refusal}"));
        let SeatBody::Single { command, .. } = &bundle.seats[seat].body else {
            panic!("{recipe}/{seat} is a single seat");
        };
        let held: Vec<usize> = bundle.sites[seat]
            .capabilities
            .as_ref()
            .unwrap()
            .outcomes
            .iter()
            .map(|outcome| outcome.held.len())
            .collect();
        let (spawn, input) = sealed(&bundle, seat, 0);
        let record = &input["launch_record"];
        observed.insert(
            format!("{recipe}/{seat}"),
            json!({
                "authored command": command[1..],
                "held": held,
                "spawn": spawn.argv[1..],
                "segments": record["segments"],
                "local": record["expected"]["local"],
                "native": record["expected"]["native"],
                "template": record["expected"]["template"],
                "final": try_launch(&bundle, seat, 0),
            }),
        );
        let limits: Vec<&str> = list.split(',').collect();
        let pins = ["--model", "claude-fable-5-1", "--effort", "high"];
        expected.insert(
            format!("{recipe}/{seat}"),
            json!({
                "authored command": ["driver", "claude", "--",
                                     "--model", "claude-fable-5-1", "--effort", "high"],
                "held": [0],
                "spawn": ["driver", "claude", "--", "--model", "claude-fable-5-1",
                          "--effort", "high", "--permission-mode", "acceptEdits",
                          "--allowedTools", list],
                "segments": [
                    {"origin": "authored", "argv": pins},
                    {"origin": "template", "argv": ["--permission-mode", "acceptEdits"]},
                    {"origin": "local", "argv": ["--allowedTools", list]},
                ],
                "local": {"allow": {"kind": "listed", "names": names},
                          "sandbox": {"kind": "unspecified"},
                          "application": {"kind": "direct", "limits": limits}},
                "native": {"kind": "known", "held": [], "denied": ["web-fetch", "web-search"]},
                "template": {"kind": "declared", "argv": ["--permission-mode", "acceptEdits"]},
                "final": {"Ok": ["claude", "-p", "--output-format", "stream-json", "--verbose",
                                 "--model", "claude-fable-5-1", "--effort", "high",
                                 "--permission-mode", "acceptEdits", "--allowedTools", list,
                                 "--disallowedTools", "WebFetch,WebSearch"]},
            }),
        );
    }
    assert_eq!(Value::Object(observed), Value::Object(expected));
}

/// Rebuild unit 7 (task 7.1; operator ruling of 2026-09-25, "narrow"):
/// `bundles/verify`'s inline Claude reviewer and the three inline Codex
/// seats of `recipes/standby` and `recipes/review-first` author no
/// capability flag. Each is compiled from the shipped directory in a realm
/// that grants nothing, then composed, sealed and launched by the engine's
/// own functions, and none holds anything. The reviewer's typed allow
/// lowers to its former list, both narrow gh prefixes included, never
/// unrestricted gh. Each Codex seat's typed class is the engine's own
/// `local` segment at its narrowed class: the standby implementer runs
/// `workspace-write` (it was `danger-full-access`) and delivers by file,
/// and both reviewers run `read-only` (they were `workspace-write`) and
/// deliver through the last-message door. Every final command ends in its
/// provider's native denial and nothing wider. The literals are written
/// out here, not derived from the adapters.
#[test]
fn the_shipped_verify_and_codex_recipes_seat_their_typed_restrictions_as_the_engines_own() {
    use brokkr_runtime::SeatClass;
    let operator = Operator::new();
    let context = CapabilityContext::no_grants("private", operator.root());
    let compiled = |relative: &str| {
        Bundle::compile_with_capabilities(
            &workspace().join(relative),
            &workspace().join("agents"),
            &workspace().join("adapters"),
            Some("private"),
            None,
            Boundary::Harness,
            &context,
        )
        .unwrap_or_else(|refusal| panic!("{relative} must compile: {refusal}"))
    };
    let authored = |bundle: &Bundle, seat: &str| {
        let SeatBody::Single { command, .. } = &bundle.seats[seat].body else {
            panic!("{seat} is a single seat");
        };
        command[1..].to_vec()
    };
    let held = |bundle: &Bundle, seat: &str| -> Vec<usize> {
        bundle.sites[seat]
            .capabilities
            .as_ref()
            .unwrap()
            .outcomes
            .iter()
            .map(|outcome| outcome.held.len())
            .collect()
    };
    let mut observed = serde_json::Map::new();

    let verify = compiled("bundles/verify");
    let (spawn, input) = sealed(&verify, "review", 0);
    let record = &input["launch_record"];
    observed.insert(
        "verify/review".into(),
        json!({
            "authored command": authored(&verify, "review"),
            "held": held(&verify, "review"),
            "spawn": spawn.argv[1..],
            "segments": record["segments"],
            "local": record["expected"]["local"],
            "native": record["expected"]["native"],
            "template": record["expected"]["template"],
            "final": try_launch(&verify, "review", 0),
        }),
    );

    let standby = compiled("recipes/standby");
    let review_first = compiled("recipes/review-first");
    for (name, bundle, seat, class) in [
        ("standby/implement", &standby, "implement", SeatClass::Work),
        ("standby/review", &standby, "review", SeatClass::Gate),
        (
            "review-first/review",
            &review_first,
            "review",
            SeatClass::Gate,
        ),
    ] {
        let (sealing, record, launched, door) =
            inline_codex_sealing(bundle, seat, class, &bundle.sites[seat], |_| {});
        observed.insert(
            name.into(),
            json!({
                "authored command": authored(bundle, seat),
                "held": held(bundle, seat),
                "sealed": format!("{sealing:?}"),
                "segments": record["segments"],
                "local": record["expected"]["local"],
                "native": record["expected"]["native"],
                "template": record["expected"]["template"],
                "final": launched.map_err(|refusal| format!("refused: {refusal}")),
                "door": door,
            }),
        );
    }

    let list = "Bash(cargo:*),Bash(git:*),Bash(python3:*),Bash(.venv/bin/pytest:*),Bash(ls:*),\
                Bash(rg:*),Bash(gh pr view:*),Bash(gh run view:*)";
    let claude_pins = ["--model", "claude-fable-5-1", "--effort", "high"];
    let codex_pins = ["--model", "gpt-6-astra", "--effort", "xhigh"];
    let codex = |segment: &[&str], class: &str, door: &str| {
        let mut last = vec![
            "codex",
            "exec",
            "--json",
            "-C",
            "/w",
            "-c",
            "model_reasoning_effort=\"xhigh\"",
            "--model",
            "gpt-6-astra",
        ];
        last.extend(segment);
        last.extend(OFF);
        json!({
            "authored command": ["driver", "codex", "--",
                                 "--model", "gpt-6-astra", "--effort", "xhigh"],
            "held": [0],
            "sealed": "Ok(())",
            "segments": [{"origin": "authored", "argv": codex_pins},
                         {"origin": "local", "argv": segment}],
            "local": {"allow": {"kind": "unspecified"}, "sandbox": {"kind": class},
                      "application": {"kind": "unrestricted"}},
            "native": {"kind": "known", "held": [], "denied": ["web-search"]},
            "template": {"kind": "none"},
            "final": {"Ok": last},
            "door": door,
        })
    };
    let gate = [
        "--sandbox",
        "read-only",
        "--output-last-message",
        "/w/result.json",
    ];
    let expected = json!({
        "verify/review": {
            "authored command": ["driver", "claude", "--",
                                 "--model", "claude-fable-5-1", "--effort", "high"],
            "held": [0],
            "spawn": ["driver", "claude", "--", "--model", "claude-fable-5-1",
                      "--effort", "high", "--permission-mode", "acceptEdits",
                      "--allowedTools", list],
            "segments": [
                {"origin": "authored", "argv": claude_pins},
                {"origin": "template", "argv": ["--permission-mode", "acceptEdits"]},
                {"origin": "local", "argv": ["--allowedTools", list]},
            ],
            "local": {"allow": {"kind": "listed",
                                "names": ["cargo", "git", "python3", "pytest", "ls", "rg",
                                          "gh-pr-view", "gh-run-view"]},
                      "sandbox": {"kind": "unspecified"},
                      "application": {"kind": "direct",
                                      "limits": list.split(',').collect::<Vec<_>>()}},
            "native": {"kind": "known", "held": [], "denied": ["web-fetch", "web-search"]},
            "template": {"kind": "declared", "argv": ["--permission-mode", "acceptEdits"]},
            "final": {"Ok": ["claude", "-p", "--output-format", "stream-json", "--verbose",
                             "--model", "claude-fable-5-1", "--effort", "high",
                             "--permission-mode", "acceptEdits", "--allowedTools", list,
                             "--disallowedTools", "WebFetch,WebSearch"]},
        },
        "standby/implement": codex(&["--sandbox", "workspace-write"], "workspace-write", "file"),
        "standby/review": codex(&gate, "read-only", "last-message"),
        "review-first/review": codex(&gate, "read-only", "last-message"),
    });
    assert_eq!(Value::Object(observed), expected);
}

/// Rebuild unit 8 (task 8.1; operator ruling of 2026-09-25, "narrow"):
/// `recipes/wager-harness`'s inline Codex implementer authors no sandbox.
/// Its typed class is the engine's own `local` segment at
/// `workspace-write` (it was `danger-full-access`, which no path admits),
/// it delivers by file, and its final command ends in the native denial.
/// It is compiled from the shipped directory in a realm that grants
/// nothing, and the literals are written out here.
#[test]
fn the_shipped_wager_harness_seats_its_typed_sandbox_narrowed_as_the_engines_own() {
    let operator = Operator::new();
    let context = CapabilityContext::no_grants("private", operator.root());
    let bundle = Bundle::compile_with_capabilities(
        &workspace().join("recipes/wager-harness"),
        &workspace().join("agents"),
        &workspace().join("adapters"),
        Some("private"),
        None,
        Boundary::Harness,
        &context,
    )
    .unwrap_or_else(|refusal| panic!("recipes/wager-harness must compile: {refusal}"));
    let SeatBody::Single { command, .. } = &bundle.seats["implement"].body else {
        panic!("implement is a single seat");
    };
    let held: Vec<usize> = bundle.sites["implement"]
        .capabilities
        .as_ref()
        .unwrap()
        .outcomes
        .iter()
        .map(|outcome| outcome.held.len())
        .collect();
    let (sealing, record, launched, door) = inline_codex_sealing(
        &bundle,
        "implement",
        brokkr_runtime::SeatClass::Work,
        &bundle.sites["implement"],
        |_| {},
    );
    let observed = json!({
        "authored command": command[1..],
        "held": held,
        "sealed": format!("{sealing:?}"),
        "segments": record["segments"],
        "local": record["expected"]["local"],
        "native": record["expected"]["native"],
        "template": record["expected"]["template"],
        "final": launched.map_err(|refusal| format!("refused: {refusal}")),
        "door": door,
    });
    let pins = ["--model", "gpt-6-sol", "--effort", "medium"];
    let segment = ["--sandbox", "workspace-write"];
    let mut last = vec![
        "codex",
        "exec",
        "--json",
        "-C",
        "/w",
        "-c",
        "model_reasoning_effort=\"medium\"",
        "--model",
        "gpt-6-sol",
    ];
    last.extend(segment);
    last.extend(OFF);
    let expected = json!({
        "authored command": ["driver", "codex", "--",
                             "--model", "gpt-6-sol", "--effort", "medium"],
        "held": [0],
        "sealed": "Ok(())",
        "segments": [{"origin": "authored", "argv": pins},
                     {"origin": "local", "argv": segment}],
        "local": {"allow": {"kind": "unspecified"}, "sandbox": {"kind": "workspace-write"},
                  "application": {"kind": "unrestricted"}},
        "native": {"kind": "known", "held": [], "denied": ["web-search"]},
        "template": {"kind": "none"},
        "final": {"Ok": last},
        "door": "file",
    });
    assert_eq!(observed, expected);
}

/// Rebuild unit 8's inventory (task 8.1): after units 6, 7 and 8, no
/// shipped recipe, bundle or agent authors a capability-bearing option in
/// any driver command. Every option any authored command carries is a pin
/// the engine does not compose (`--model`, `--effort`), the driver
/// separator, or research-dsh's route-only `--patch` overlay. Every
/// `bundle.json` and every file that authors a command is listed exactly,
/// so the sweep is shown to have read them; the agent-backed `gpt-flash`,
/// `release` and `triage` author none, and no agent file does.
#[test]
fn no_shipped_driver_command_authors_a_capability_bearing_option() {
    fn commands<'a>(value: &'a Value, found: &mut Vec<&'a str>) {
        match value {
            Value::Object(map) => {
                for (key, inner) in map {
                    match (key.as_str(), inner) {
                        ("command", Value::Array(argv)) => {
                            found.extend(argv.iter().filter_map(Value::as_str))
                        }
                        _ => commands(inner, found),
                    }
                }
            }
            Value::Array(items) => items.iter().for_each(|inner| commands(inner, found)),
            _ => {}
        }
    }
    fn sweep(dir: &Path, out: &mut std::collections::BTreeMap<String, Vec<String>>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                sweep(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "json") {
                let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap())
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                let mut argv = Vec::new();
                commands(&value, &mut argv);
                if !argv.is_empty() || path.ends_with("bundle.json") {
                    let relative = path.strip_prefix(workspace()).unwrap();
                    let options = argv
                        .iter()
                        .filter(|part| part.starts_with('-'))
                        .filter(|part| !matches!(**part, "--" | "--model" | "--effort"))
                        .map(|part| part.to_string())
                        .collect();
                    out.insert(relative.display().to_string(), options);
                }
            }
        }
    }
    let mut observed = std::collections::BTreeMap::new();
    for dir in ["recipes", "bundles", "agents"] {
        sweep(&workspace().join(dir), &mut observed);
    }
    let mut expected: std::collections::BTreeMap<String, Vec<String>> = [
        "bundles/self",
        "bundles/verify",
        "recipes/fast",
        "recipes/gpt-flash",
        "recipes/landing",
        "recipes/night-shift",
        "recipes/node",
        "recipes/panel-review",
        "recipes/preflight",
        "recipes/release",
        "recipes/research",
        "recipes/research-dsh",
        "recipes/review-first",
        "recipes/standby",
        "recipes/triage",
        "recipes/wager-harness",
        "recipes/wager-harness-dsh",
        "recipes/wager-harness-muse",
    ]
    .iter()
    .map(|dir| (format!("{dir}/bundle.json"), Vec::new()))
    .collect();
    expected.insert(
        "recipes/research-dsh/bundle.json".into(),
        vec!["--patch".into()],
    );
    assert_eq!(observed, expected);
}

/// A panel member and a sequence step are sites exactly as a seat is: each
/// resolves its own asks under its own label, and a request written on the
/// CONTAINER — which executes nothing — is refused rather than dropped.
#[test]
fn a_panel_member_and_a_sequence_step_resolve_under_their_own_labels() {
    let operator = Operator::new();
    let site = |asks: Option<Value>| {
        // No authored `--sandbox` (fixture migration of 2026-09-26).
        let mut site = json!({"role": "roles/role.md", "driver": {"command": [
            "{brokkr}", "driver", "codex", "--", "--model", "gpt-6-astra", "--effort", "high"]}});
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
    // Under the canonicalised temporary base, so every path a fixture
    // writes, loads and expects is the one spelling (macOS `/var`).
    let copied = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
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

/// An inline Codex work seat's pins, with no authored sandbox: a recipe
/// authors no `--sandbox` (operator ruling 1 of 2026-09-23; fixture
/// migration of 2026-09-26).
const CODEX_SEAT: [&str; 8] = [
    "{brokkr}",
    "driver",
    "codex",
    "--",
    "--model",
    "gpt-6-astra",
    "--effort",
    "high",
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
    // Operator ruling 1 of 2026-09-23 (rebuild unit 12): refused by origin,
    // naming the canonical option and its position, never its value.
    let refusal = |site: &str, written: &str, provider: &str| {
        format!(
            "bundle: seat '{site}' (office '{site}') in realm 'private': its arguments carry \
             '{written}' (argument 5), a capability-bearing option of harness '{provider}'. A \
             recipe authors no capability-bearing option, whatever its value, polarity or \
             grant: tools come from typed declarations and the realm's grant, composed by the \
             engine alone (operator ruling 1 of 2026-09-23)"
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
            "--config",
            "codex",
        ),
        (
            codex(&["--config=mcp_servers={ungranted={command=\"npx\"}}"]),
            "--config",
            "codex",
        ),
        // Second council H1, verbatim: the ATTACHED spelling the first
        // repair's scanner passed straight through into the final
        // `codex exec` command earns the same realm-only refusal.
        (
            codex(&["-cmcp_servers.ungranted.command=\"/bin/false\""]),
            "--config",
            "codex",
        ),
        (
            codex(&["-c=mcp_servers.ungranted.command=\"/bin/false\""]),
            "--config",
            "codex",
        ),
        (
            codex(&["--config", "mcp_servers.ungranted.command=\"/bin/false\""]),
            "--config",
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
            "--allowedTools",
            "claude",
        ),
        // Counterfeit hands: the engine's server name, authored.
        (
            codex(&["-c", "mcp_servers.brokkr.command=\"{brokkr}\""]),
            "--config",
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
            "--allowedTools",
            "claude",
        ),
        (claude(&["--allowed-tools=*"]), "--allowedTools", "claude"),
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
            refusal(site, "--config", "codex"),
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

/// Rebuild unit 12 (operator ruling 1 of 2026-09-23; tasks 12.1 and 12.2):
/// an inline seat of every harness brokkr drives that AUTHORS a
/// capability-bearing option is refused at compile under every grant state
/// — no map, a realm that grants nothing, a grant its office asks for and
/// one it does not — whatever the option's polarity: a list agreeing with
/// the engine's managed denial, an explicit empty restriction, the ON a
/// grant would compose, and the engine's own OFF bytes. The same seat
/// without the option compiles in every state, and where it holds nothing
/// its final command carries the engine's managed denial alone.
#[test]
fn an_authored_capability_option_refuses_the_compile_under_every_grant_state() {
    let operator = Operator::new();
    let adapters = workspace().join("adapters");
    let seat = |harness: &str, model: &str, extra: &[&str]| {
        let mut command = vec!["{brokkr}", "driver", harness, "--", "--model", model];
        command.extend(["--effort", "high"]);
        [command, extra.to_vec()]
            .concat()
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>()
    };
    let refused = |realm: &str, option: &str, harness: &str| {
        format!(
            "bundle: seat 'work' (office 'work') in realm '{realm}': its arguments carry \
             '{option}' (argument 5), a capability-bearing option of harness '{harness}'. A \
             recipe authors no capability-bearing option, whatever its value, polarity or \
             grant: tools come from typed declarations and the realm's grant, composed by the \
             engine alone (operator ruling 1 of 2026-09-23)"
        )
    };
    type Rows = Vec<(&'static [&'static str], &'static str)>;
    let harnesses: [(&str, &str, &str, Rows); 3] = [
        (
            "claude",
            "claude-opus-5",
            "claude-native-search",
            vec![
                (
                    &["--disallowedTools", "WebSearch,WebFetch"],
                    "--disallowedTools",
                ),
                (&["--tools", ""], "--tools"),
                (&["--allowed-tools=WebSearch"], "--allowedTools"),
            ],
        ),
        (
            "lanetally",
            "claude-opus-5",
            "claude-native-search",
            vec![
                (&["--disallowed-tools=WebFetch"], "--disallowedTools"),
                (&["--tools="], "--tools"),
            ],
        ),
        (
            "codex",
            "gpt-6-astra",
            "codex-native-search",
            vec![
                (&["-c", "web_search=\"disabled\""], "--config"),
                (&["--search"], "--search"),
                (&["-sread-only"], "--sandbox"),
            ],
        ),
    ];
    for (harness, model, dialect, rows) in &harnesses {
        let granted = json!({"web-search": {"dialect": dialect}});
        let states = [
            (
                CapabilityContext::no_grants("<unmapped>", operator.root()),
                None,
            ),
            (
                operator.context(json!({})),
                Some(json!({"web-search": "wants"})),
            ),
            (
                operator.context(granted.clone()),
                Some(json!({"web-search": "wants"})),
            ),
            (operator.context(granted), None),
        ];
        for (context, asks) in &states {
            let realm = context.realm.as_str();
            let plant = |command: &[String]| {
                let parts: Vec<&str> = command.iter().map(String::as_str).collect();
                one_inline_seat(&operator, &parts);
                if let Some(asks) = asks {
                    let path = operator.root().join("solo/bundle.json");
                    let mut bundle: Value =
                        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                    bundle["seats"]["work"]["capabilities"] = asks.clone();
                    write(operator.root(), "solo/bundle.json", &bundle);
                }
            };
            for (extra, option) in rows {
                plant(&seat(harness, model, extra));
                assert_eq!(
                    solo(&operator, &adapters, context).as_str(),
                    refused(realm, option, harness),
                    "{harness} {extra:?} in {realm} asking {asks:?}"
                );
            }
            // The migrated positive: the same seat, nothing authored.
            plant(&seat(harness, model, &[]));
            let bundle = solo_bundle(&operator, &adapters, context)
                .unwrap_or_else(|refusal| panic!("{harness} in {realm}: {refusal}"));
            let outcome = &bundle.sites["work"].capabilities.as_ref().unwrap().outcomes[0];
            if !outcome.held.is_empty() || *harness == "lanetally" {
                continue;
            }
            let launched = try_launch(&bundle, "work", 0)
                .unwrap_or_else(|refusal| panic!("{harness} in {realm}: {refusal}"));
            let denial: &[&str] = match *harness {
                "codex" => &OFF,
                _ => &["--disallowedTools", "WebFetch,WebSearch"],
            };
            assert_eq!(
                &launched[launched.len() - 2..],
                denial,
                "{harness} in {realm}: {launched:?}"
            );
        }
    }
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
        ],
    );
    // No authored `--permission-mode` (fixture migration of 2026-09-26):
    // the inline seat declares no typed allow, so no template is emitted.
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
                "--disallowedTools",
                "WebFetch,WebSearch"
            ]
        )
    );
    // A form no launch consumes is refused where it is compiled: Codex
    // takes no tool selection. The adapter's own invocation dispatches no
    // modelled grammar, so the declaration loads (rebuild unit 11) and the
    // inline seat's codex launch is what refuses it.
    let selecting = copied_adapters();
    edit_adapter(selecting.path(), "codex", |adapter| {
        adapter["driver"] = json!(["codex"]);
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

/// Rebuild unit 12, second review F1 (design D6; NCT "Admission and
/// restriction cannot erase each other"; task 12.2), along the whole chain:
/// Claude's search OFF declared as an explicit include list is a hard
/// limit. Holding nothing, the inline seat's final command carries the
/// limit as declared, empty or not, beside the independent WebFetch
/// denial. Requiring a realm-granted fetch the limit does not name, the
/// compile refuses the whole conflict instead of launching
/// `--tools WebFetch,Read`; wanting it, the fetch drops with its denial
/// composed (unit 12-fix; CQ1). A limit that names WebFetch reaches its
/// literal command with the admission, so always-OFF is no grant support.
#[test]
fn an_explicit_include_list_an_adapter_declares_is_never_widened_by_a_grant() {
    let operator = Operator::new();
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
    let head = [
        "claude",
        "-p",
        "--output-format",
        "stream-json",
        "--verbose",
        "--model",
        "claude-opus-5",
        "--effort",
        "high",
    ];
    let launched = |tail: &[&str]| format!("launched {:?}", [&head[..], tail].concat());
    let refused = |limit: &str| {
        format!(
            "bundle: seat 'work' (office 'work') in realm 'private': the capability plan's \
             explicit '--tools' restriction for provider 'claude' (naming {limit}) does not name \
             tool 'WebFetch', which the plan admits for native capability 'web-fetch'; an \
             explicit tool list is a hard limit that nothing widens, so the conflict is refused \
             whole rather than unioned (design D6)"
        )
    };
    let dialect = "dialects/tools/claude-native-fetch.json";
    let body: Value =
        serde_json::from_slice(&std::fs::read(workspace().join(dialect)).unwrap()).unwrap();
    write(operator.root(), dialect, &body);
    let nothing = operator.context(json!({}));
    let fetch = operator.context(json!({"web-fetch": {"dialect": "claude-native-fetch"}}));
    // Third review F1: fetch ON declared as an argv switch, or as a
    // measured default that writes nothing, is held all the same.
    let switch = json!({"argv": ["--allowedTools", "WebFetch"]});
    let default = json!({"default": "fetch is on unless a list removes it"});
    // Unit 12-fix (CQ1): a REQUIRED holding the limit excludes refuses the
    // whole conflict; a WANTED one drops, its native control OFF.
    // Unit 12-fix-b (I1): a limit only bounds, and the list it calls for is
    // filled from the holdings alone, so Read, which only the limit names,
    // never reaches the command.
    let off_launch = || launched(&["--tools", "", "--disallowedTools", "WebFetch"]);
    let rows: Vec<(Value, Option<&Value>, &CapabilityContext, &str, String)> = vec![
        (
            json!(["--tools", "Read"]),
            None,
            &nothing,
            "wants",
            off_launch(),
        ),
        (json!(["--tools="]), None, &nothing, "wants", off_launch()),
        (
            json!(["--tools", "Read"]),
            None,
            &fetch,
            "requires",
            refused("Read"),
        ),
        (
            json!(["--tools="]),
            None,
            &fetch,
            "requires",
            refused("no tool"),
        ),
        (
            json!(["--tools", "Read"]),
            None,
            &fetch,
            "wants",
            off_launch(),
        ),
        (json!(["--tools="]), None, &fetch, "wants", off_launch()),
        (
            json!(["--tools", "Read,WebFetch"]),
            None,
            &fetch,
            "wants",
            launched(&["--tools", "WebFetch", "--allowedTools", "WebFetch"]),
        ),
        (
            json!(["--tools", "Read"]),
            Some(&switch),
            &fetch,
            "requires",
            refused("Read"),
        ),
        (
            json!(["--tools", "Read"]),
            Some(&default),
            &fetch,
            "requires",
            refused("Read"),
        ),
        (
            json!(["--tools", "Read,WebFetch"]),
            Some(&switch),
            &fetch,
            "wants",
            launched(&["--tools", "WebFetch", "--allowedTools", "WebFetch"]),
        ),
        (
            json!(["--tools", "Read,WebFetch"]),
            Some(&default),
            &fetch,
            "wants",
            launched(&["--tools", "WebFetch"]),
        ),
    ];
    for (off, on, context, strength, expected) in rows {
        let adapters = copied_adapters();
        edit_adapter(adapters.path(), "claude", |adapter| {
            let known = &mut adapter["native_capabilities"]["known"];
            known["web-search"]["off"] = json!({"argv": off.clone()});
            if let Some(on) = on {
                known["web-fetch"]["on"] = on.clone();
            }
        });
        let path = operator.root().join("solo/bundle.json");
        let mut bundle: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        bundle["seats"]["work"]["capabilities"] = json!({"web-fetch": strength});
        write(operator.root(), "solo/bundle.json", &bundle);
        assert_eq!(
            solo(&operator, adapters.path(), context),
            expected,
            "{off} {strength} under {}",
            context.realm
        );
    }
}

/// Write the shipped Claude fetch dialect into the operator's directory,
/// with `tools` as its tool set, under `name`.
fn fetch_dialect(operator: &Operator, name: &str, tools: Value) {
    let shipped = "dialects/tools/claude-native-fetch.json";
    let mut body: Value =
        serde_json::from_slice(&std::fs::read(workspace().join(shipped)).unwrap()).unwrap();
    body["name"] = json!(name);
    body["tools"] = tools;
    write(
        operator.root(),
        &format!("dialects/tools/{name}.json"),
        &body,
    );
}

/// Rebuild unit 12-fix, chief R1 and the hands case (design D6): a held
/// capability admits exactly the tools of the ONE inventory entry its
/// holding binds. Beside the realm-granted fetch, a second, unselected
/// entry serving the same capability declares Bash and an OFF that writes
/// `--tools Bash,WebFetch --allowedTools Bash`: the boxed Claude link is
/// refused, never launched with Bash admitted beside the hands. The same
/// entry's include list alone is only a limit, and the box's hands are
/// the base a limit bounds: the held fetch fills the hands' empty list,
/// and no tool the limit names but nothing holds enters the box. A limit
/// that excludes the wanted fetch drops it, its denial composed. The hands
/// tool is bounded like every allowance (rebuild unit 12-fix-c, I1): a
/// limit that does not name it refuses the whole conflict, at compile.
#[test]
fn a_boxed_holding_admits_only_its_bound_entry_and_the_hands_only_fill_the_limit() {
    let operator = Operator::new();
    fetch_dialect(&operator, "claude-native-fetch", json!(["WebFetch"]));
    let fetch = operator.context(json!({"web-fetch": {"dialect": "claude-native-fetch"}}));
    // Both offices want fetch; the Claude link of `fallback` holds it.
    for office in ["searcher", "fallback"] {
        let path = operator.root().join(format!("agents/{office}.json"));
        let mut agent: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        agent["capabilities"] = json!({"web-fetch": "wants"});
        write(operator.root(), &format!("agents/{office}.json"), &agent);
    }
    let second = |off: Value| {
        move |adapter: &mut Value| {
            let known = &mut adapter["native_capabilities"]["known"];
            let mut entry = known["web-fetch"].clone();
            entry["tools"] = json!(["Bash"]);
            entry["on"] = json!({"argv": ["--allowedTools", "Bash"]});
            entry["off"] = json!({"argv": off});
            known["web-fetch-second"] = entry;
        }
    };
    let search_off = |off: Value| {
        move |adapter: &mut Value| {
            adapter["native_capabilities"]["known"]["web-search"]["off"] = json!({"argv": off});
        }
    };
    let launched_by =
        |edit: &dyn Fn(&mut Value),
         launch: fn(&Bundle, &str, usize) -> Result<Vec<String>, String>| {
            let adapters = copied_adapters();
            edit_adapter(adapters.path(), "claude", edit);
            operator
                .compile_against(adapters.path(), &fetch, Boundary::Namespace, None, None)
                .and_then(|bundle| launch(&bundle, "chain", 0))
                .map(|argv| {
                    // The hands server's document names this test binary.
                    let mut tail =
                        argv[argv.iter().position(|part| part == "--tools").unwrap()..].to_vec();
                    tail[4] = "<hands>".into();
                    tail
                })
        };
    let boxed = |edit: &dyn Fn(&mut Value)| launched_by(edit, sealed_launch);
    let outside = |names: &str| {
        Err(format!(
            "bundle: seat 'chain' (office 'fallback') in realm 'private': the capability plan's \
             explicit '--tools' restriction for provider 'claude' (naming {names}) does not name \
             tool 'mcp__brokkr__workspace', which the site's typed hands admit; an explicit tool \
             list is a hard limit that nothing widens, so the conflict is refused whole rather \
             than unioned (design D6)"
        ))
    };
    let tail = |tools: &str, allow: &str, deny: &[&str]| {
        Ok([
            &[
                "--tools",
                tools,
                "--strict-mcp-config",
                "--mcp-config",
                "<hands>",
                "--allowedTools",
                allow,
            ][..],
            deny,
        ]
        .concat()
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>())
    };
    assert_eq!(
        boxed(&second(json!([
            "--tools",
            "Bash,WebFetch",
            "--allowedTools",
            "Bash"
        ]))),
        Err(
            "bundle: seat 'chain' (office 'fallback') in realm 'private': the capability plan \
             admits tool 'Bash' for provider 'claude', which no realm holding admits; a tool is \
             admitted only through the one adapter entry a holding binds, narrowed by its grant \
             (design D6)"
                .to_string()
        )
    );
    assert_eq!(
        boxed(&second(json!(["--tools", "Bash,WebFetch"]))),
        outside("Bash, WebFetch")
    );
    assert_eq!(
        boxed(&second(json!([
            "--tools",
            "Bash,WebFetch,mcp__brokkr__workspace"
        ]))),
        tail(
            "WebFetch",
            "mcp__brokkr__workspace,WebFetch",
            &["--disallowedTools", "WebSearch"]
        )
    );
    // The same launch checked whole, as rebuild unit 14 serves it: the
    // unselected entry's OFF for the held fetch is no denial of WebFetch
    // (operator ruling (1) of 2026-09-27; rebuild unit 14a4b).
    assert_eq!(
        launched_by(
            &second(json!(["--tools", "Bash,WebFetch,mcp__brokkr__workspace"])),
            checked_launch
        ),
        tail(
            "WebFetch",
            "mcp__brokkr__workspace,WebFetch",
            &["--disallowedTools", "WebSearch"]
        )
    );
    assert_eq!(
        boxed(&search_off(json!(["--tools", "Read,WebFetch"]))),
        outside("Read, WebFetch")
    );
    assert_eq!(
        boxed(&search_off(json!([
            "--tools",
            "Read,WebFetch,mcp__brokkr__workspace"
        ]))),
        tail("WebFetch", "mcp__brokkr__workspace,WebFetch", &[])
    );
    // The wanted fetch drops first; the hands tool is still outside.
    assert_eq!(
        boxed(&search_off(json!(["--tools", "Read"]))),
        outside("Read")
    );
    assert_eq!(
        boxed(&search_off(json!([
            "--tools",
            "Read,mcp__brokkr__workspace"
        ]))),
        tail(
            "",
            "mcp__brokkr__workspace",
            &["--disallowedTools", "WebFetch"]
        )
    );
}

/// Rebuild unit 12-fix, chief R2 and R3 (design D6; CQ1), unboxed. R2: a
/// grant narrowed to WebFetch from an entry of [WebFetch, Read] admits
/// WebFetch alone, so a limit naming only WebFetch is compatible and
/// reaches its literal command with Read denied — the entry's unselected
/// Read is no admission. R3: every restrictive list is a hard limit, the
/// adapter template's included: behind a template ending `--tools
/// Read,Bash` a required fetch refuses the whole conflict, naming the
/// template, and a wanted one drops with its denial composed — never
/// `--tools Read,Bash,WebFetch`. A template limit that names the held tool
/// holds it within.
#[test]
fn a_narrowed_grant_admits_its_subset_and_a_template_limit_is_never_widened() {
    let operator = Operator::new();
    fetch_dialect(&operator, "claude-fetch-read", json!(["WebFetch", "Read"]));
    fetch_dialect(&operator, "claude-native-fetch", json!(["WebFetch"]));
    let narrowed = operator.context(json!({"web-fetch": {"dialect": "claude-fetch-read",
                                                         "tools": ["WebFetch"]}}));
    let fetch = operator.context(json!({"web-fetch": {"dialect": "claude-native-fetch"}}));
    // R2: the inline seat, the fetch entry widened to [WebFetch, Read].
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
    let path = operator.root().join("solo/bundle.json");
    let mut bundle: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    bundle["seats"]["work"]["capabilities"] = json!({"web-fetch": "requires"});
    write(operator.root(), "solo/bundle.json", &bundle);
    let inline = |limit: Option<Value>| {
        let adapters = copied_adapters();
        edit_adapter(adapters.path(), "claude", |adapter| {
            let known = &mut adapter["native_capabilities"]["known"];
            let both = json!(["WebFetch", "Read"]);
            known["web-fetch"]["tools"] = both.clone();
            known["web-fetch"]["on"] =
                json!({"selection": {"include": both, "allow": both, "deny": []}});
            known["web-fetch"]["off"] =
                json!({"selection": {"include": [], "allow": [], "deny": both}});
            if let Some(limit) = limit {
                known["web-search"]["off"] = json!({"argv": limit});
            }
        });
        solo(&operator, adapters.path(), &narrowed)
    };
    let launched =
        |tail: &[&str]| launched_as(&[&["--model", "claude-opus-5", "--effort", "high"], tail]);
    assert_eq!(
        inline(None),
        launched(&[
            "--allowedTools",
            "WebFetch",
            "--disallowedTools",
            "Read,WebSearch"
        ])
    );
    assert_eq!(
        inline(Some(json!(["--tools", "WebFetch"]))),
        launched(&[
            "--tools",
            "WebFetch",
            "--allowedTools",
            "WebFetch",
            "--disallowedTools",
            "Read"
        ])
    );
    // R3: an agent-backed seat, typed allow [ls], behind a template limit.
    write(
        operator.root(),
        "solo/bundle.json",
        &json!({"name": "solo", "policy": "policy.json", "seats": {
            "work": {"results": ["complete"], "agent": "templated"},
            "review": {"results": ["clean"], "role": "roles/role.md",
                       "driver": {"command": ["driver"]}}}}),
    );
    let copied = copied_adapters();
    let limited = copied.path().to_path_buf();
    let templated = |limit: &str, strength: &str| {
        write(
            operator.root(),
            "agents/templated.json",
            &json!({
                "description": "an office with one local command",
                "charter": "charters/searcher.md",
                "models": ["opus"],
                "efforts": {"opus": "high"},
                "tools": {"allow": ["ls"]},
                "capabilities": {"web-fetch": strength},
            }),
        );
        edit_adapter(&limited, "claude", |adapter| {
            adapter["driver"] = json!([
                "{brokkr}",
                "driver",
                "claude",
                "--",
                "--permission-mode",
                "acceptEdits",
                "--tools",
                limit
            ]);
        });
        solo(&operator, &limited, &fetch)
    };
    let template =
        |tail: &[&str]| launched_as(&[&["--permission-mode", "acceptEdits", "--tools"][..], tail]);
    assert_eq!(
        templated("Read,Bash", "requires"),
        "bundle: seat 'work' (office 'templated') in realm 'private': the adapter template's \
         explicit '--tools' restriction for provider 'claude' (naming Read, Bash) does not name \
         tool 'WebFetch', which the plan admits for native capability 'web-fetch'; an explicit \
         tool list is a hard limit that nothing widens, so the conflict is refused whole rather \
         than unioned (design D6)"
    );
    // Dropped, the fetch leaves nothing held: the template's list is
    // written in place naming the typed local permission's Bash alone, Read
    // being a name only the template gives (unit 12-fix-b, I1; unit 13-fix,
    // F6).
    assert_eq!(
        templated("Read,Bash", "wants"),
        template(&[
            "Bash",
            "--model",
            "claude-opus-5-5",
            "--effort",
            "high",
            "--allowedTools",
            "Bash(ls:*)",
            "--disallowedTools",
            "WebFetch,WebSearch"
        ])
    );
    // The seat is told why, and that the native power stays OFF.
    let bundle = solo_bundle(&operator, &limited, &fetch).unwrap();
    let outcome = &bundle.sites["work"].capabilities.as_ref().unwrap().outcomes[0];
    assert_eq!(
        outcome.not_held["web-fetch"],
        "the adapter template's explicit '--tools' restriction for provider 'claude' (naming \
         Read, Bash) does not name its tool 'WebFetch'; native capability remains OFF"
    );
}

/// Rebuild unit 12-fix-b, the chief's four reproductions compiled and
/// launched (design D6; CQ1). R1: a restrictive list names no power for
/// the command — a fetch OFF written `--tools WebFetch` with nothing held
/// launches an empty list, and an ON written `--tools WebFetch,Read` for a
/// holding admitting WebFetch launches WebFetch alone. R2: under `harness`
/// the adapter's managed `hands.harness.work` list is a limit, never the
/// base: a required fetch it does not name refuses at compile, naming it,
/// and a wanted one drops with its denial and leaves the list empty. R3:
/// in the box a held fetch fills the hands' list from the holding, whether
/// its ON is an argv with its own list, an allow-only argv or a measured
/// default beside a compatible limit — never `--tools ""`. A limit is
/// compatible only where it names the hands tool too (rebuild unit
/// 12-fix-c, I1); one that does not refuses at compile. Every launch goes
/// through the seal and `verify_record` to the command builder (C3).
#[test]
fn every_chief_reproduction_composes_inside_the_holdings_and_every_limit() {
    let operator = Operator::new();
    fetch_dialect(&operator, "claude-native-fetch", json!(["WebFetch"]));
    let nothing = operator.context(json!({}));
    let fetch = operator.context(json!({"web-fetch": {"dialect": "claude-native-fetch"}}));
    // R1, on the inline Claude seat.
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
    let inline = |fetch_entry: &str, value: Value, context: &CapabilityContext, asks: Value| {
        let path = operator.root().join("solo/bundle.json");
        let mut bundle: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        bundle["seats"]["work"]["capabilities"] = asks;
        write(operator.root(), "solo/bundle.json", &bundle);
        let adapters = copied_adapters();
        edit_adapter(adapters.path(), "claude", |adapter| {
            adapter["native_capabilities"]["known"]["web-fetch"][fetch_entry] = value;
        });
        solo_sealed(&operator, adapters.path(), context)
    };
    let pins = ["--model", "claude-opus-5", "--effort", "high"];
    assert_eq!(
        inline(
            "off",
            json!({"argv": ["--tools", "WebFetch"]}),
            &nothing,
            json!({})
        ),
        launched_as(&[&pins, &["--tools", "", "--disallowedTools", "WebSearch"]])
    );
    assert_eq!(
        inline(
            "on",
            json!({"argv": ["--tools", "WebFetch,Read"]}),
            &fetch,
            json!({"web-fetch": "requires"})
        ),
        launched_as(&[
            &pins,
            &["--tools", "WebFetch", "--disallowedTools", "WebSearch"]
        ])
    );
    // R2: an agent seat with hands under `harness`, whose Claude adapter
    // appends a managed `--tools Read,Bash` for a work seat.
    write(
        operator.root(),
        "solo/bundle.json",
        &json!({"name": "solo", "policy": "policy.json", "seats": {
            "work": {"results": ["complete"], "agent": "handed"},
            "review": {"results": ["clean"], "role": "roles/role.md",
                       "driver": {"command": ["driver"]}}}}),
    );
    let managed = copied_adapters();
    edit_adapter(managed.path(), "claude", |adapter| {
        adapter["hands"]["harness"] = json!({"work": ["--tools", "Read,Bash"]});
    });
    let handed = |strength: &str| {
        write(
            operator.root(),
            "agents/handed.json",
            &json!({
                "description": "an office with hands",
                "charter": "charters/searcher.md",
                "models": ["opus"],
                "efforts": {"opus": "high"},
                "hands": {"kind": "workspace", "network": false, "binds": []},
                "capabilities": {"web-fetch": strength},
            }),
        );
        solo_sealed(&operator, managed.path(), &fetch)
    };
    assert_eq!(
        handed("requires"),
        "bundle: seat 'work' (office 'handed') in realm 'private': the adapter's managed \
         boundary fragment's explicit '--tools' restriction for provider 'claude' (naming Read, \
         Bash) does not name tool 'WebFetch', which the plan admits for native capability \
         'web-fetch'; an explicit tool list is a hard limit that nothing widens, so the conflict \
         is refused whole rather than unioned (design D6)"
    );
    let wants = launched_as(&[&[
        "--permission-mode",
        "acceptEdits",
        "--model",
        "claude-opus-5-5",
        "--effort",
        "high",
        "--tools",
        "",
        "--disallowedTools",
        "WebFetch,WebSearch",
    ]]);
    assert_eq!(handed("wants"), wants);
    // The same launch checked whole, as rebuild unit 14 serves it: under
    // `harness` the seat's hands are the managed fragment alone, with no
    // workspace fragment beside it (operator ruling (2) of 2026-09-27;
    // rebuild unit 14a4b).
    let bundle = solo_bundle(&operator, managed.path(), &fetch).unwrap();
    assert_eq!(
        checked_launch(&bundle, "work", 0).map(|argv| format!("launched {argv:?}")),
        Ok(wants)
    );
    // R3: the boxed Claude link of `fallback`, which requires fetch, so
    // its Codex link is left out of the chain.
    let path = operator.root().join("agents/fallback.json");
    let mut agent: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    agent["capabilities"] = json!({"web-fetch": "requires"});
    agent["models"] = json!(["opus"]);
    agent["efforts"] = json!({"opus": "high"});
    write(operator.root(), "agents/fallback.json", &agent);
    let boxed = |on: Value, search_off: Option<Value>| {
        let adapters = copied_adapters();
        edit_adapter(adapters.path(), "claude", |adapter| {
            let known = &mut adapter["native_capabilities"]["known"];
            known["web-fetch"]["on"] = on;
            if let Some(off) = search_off {
                known["web-search"]["off"] = json!({"argv": off});
            }
        });
        operator
            .compile_against(adapters.path(), &fetch, Boundary::Namespace, None, None)
            .and_then(|bundle| sealed_launch(&bundle, "chain", 0))
            .map(|argv| {
                // The hands server's document names this test binary.
                let mut tail =
                    argv[argv.iter().position(|part| part == "--tools").unwrap()..].to_vec();
                tail[4] = "<hands>".into();
                tail
            })
    };
    let outside = |names: &str| {
        Err(format!(
            "bundle: seat 'chain' (office 'fallback') in realm 'private': the capability plan's \
             explicit '--tools' restriction for provider 'claude' (naming {names}) does not name \
             tool 'mcp__brokkr__workspace', which the site's typed hands admit; an explicit tool \
             list is a hard limit that nothing widens, so the conflict is refused whole rather \
             than unioned (design D6)"
        ))
    };
    let tail = |allow: &str, deny: &[&str]| {
        Ok([
            &[
                "--tools",
                "WebFetch",
                "--strict-mcp-config",
                "--mcp-config",
                "<hands>",
                "--allowedTools",
                allow,
            ][..],
            deny,
        ]
        .concat()
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>())
    };
    let both = "mcp__brokkr__workspace,WebFetch";
    let bounded = "WebFetch,mcp__brokkr__workspace";
    assert_eq!(
        boxed(
            json!({"argv": ["--tools", "WebFetch", "--allowedTools", "WebFetch"]}),
            None
        ),
        outside("WebFetch")
    );
    assert_eq!(
        boxed(
            json!({"argv": ["--tools", bounded, "--allowedTools", "WebFetch"]}),
            None
        ),
        tail(both, &["--disallowedTools", "WebSearch"])
    );
    assert_eq!(
        boxed(
            json!({"argv": ["--allowedTools", "WebFetch"]}),
            Some(json!(["--tools", "WebFetch"]))
        ),
        outside("WebFetch")
    );
    assert_eq!(
        boxed(
            json!({"argv": ["--allowedTools", "WebFetch"]}),
            Some(json!(["--tools", bounded]))
        ),
        tail(both, &[])
    );
    assert_eq!(
        boxed(
            json!({"default": "fetch is on unless a list removes it"}),
            Some(json!(["--tools", "WebFetch"]))
        ),
        outside("WebFetch")
    );
    assert_eq!(
        boxed(
            json!({"default": "fetch is on unless a list removes it"}),
            Some(json!(["--tools", bounded]))
        ),
        tail("mcp__brokkr__workspace", &[])
    );
}

/// Rebuild unit 14a4c (operator ruling (2) of 2026-09-27; 14a4b's F1): one
/// work agent with workspace hands and no grants, whose Claude adapter
/// declares an empty `hands.harness.work`. Compiled under `harness`, the
/// seat's hands are that empty fragment and the command checks. The same
/// fixture under `open` seals the same fragments and the same command, and
/// refuses: only the sealed boundary tells the two apart. Each is sealed by
/// the engine itself (14a4c's review return, F1): a run started in a world
/// whose realm declares the boundary, its dispatch seam sealing the site's
/// composed spawn, and the command checked against what that seam wrote.
#[test]
fn an_empty_harness_fragment_is_the_hands_under_harness_and_refused_under_open() {
    use brokkr_protocol::native_controls::{SealedServing, SERVING_INPUTS};
    let operator = Operator::new();
    let nothing = operator.context(json!({}));
    one_inline_seat(&operator, &["driver"]);
    write(
        operator.root(),
        "solo/bundle.json",
        &json!({"name": "solo", "policy": "policy.json", "seats": {
            "work": {"results": ["complete"], "agent": "handed"},
            "review": {"results": ["clean"], "role": "roles/role.md",
                       "driver": {"command": ["driver"]}}}}),
    );
    write(
        operator.root(),
        "agents/handed.json",
        &json!({
            "description": "an office with hands",
            "charter": "charters/searcher.md",
            "models": ["opus"],
            "efforts": {"opus": "high"},
            "hands": {"kind": "workspace", "network": false, "binds": []},
        }),
    );
    let adapters = copied_adapters();
    edit_adapter(adapters.path(), "claude", |adapter| {
        adapter["hands"]["harness"] = json!({"work": []});
    });
    let checked = |boundary: Boundary| {
        let bundle = Bundle::compile_with_capabilities(
            &operator.root().join("solo"),
            &operator.root().join("agents"),
            adapters.path(),
            Some(nothing.realm.as_str()).filter(|realm| *realm != "<unmapped>"),
            None,
            boundary,
            &nothing,
        )
        .unwrap();
        let root = operator.root();
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        write(
            root,
            "realms.json",
            &json!({"schema": "forge.realms/v6", "journal": "forge.db", "realms": [
                {"name": "private", "path": repo, "default_branch": "main",
                 "boundary": boundary.word(), "capabilities": {}}]}),
        );
        let world = brokkr_runtime::World::load(&root.join("realms.json")).unwrap();
        let store = brokkr_store::Store::open(&root.join(format!("{}.db", boundary.word())));
        let engine = brokkr_runtime::Engine::start_in_world(
            store.unwrap(),
            bundle,
            "probe",
            Some(repo),
            Some(world),
        )
        .unwrap();
        let (bundle, facts) = (&engine.bundle, &engine.bundle.sites["work"]);
        let (mut spawn, _) = sealing(bundle, "work", 0, facts);
        let mut input = json!({});
        engine.mark_capabilities("work", facts.chain.first(), Some(&mut spawn), &mut input);
        assert_eq!(spawn.refusal, None);
        let sealed = SealedServing::decode(input.get(SERVING_INPUTS)).unwrap();
        (
            input[SERVING_INPUTS]["dialect"]["stands"].clone(),
            checked_against(bundle, "work", 0, &sealed),
        )
    };
    let (stands, harness) = checked(Boundary::Harness);
    assert_eq!(stands, json!({"kind": "harness"}));
    assert_eq!(
        harness,
        Ok([
            "claude",
            "-p",
            "--output-format",
            "stream-json",
            "--verbose",
            "--permission-mode",
            "acceptEdits",
            "--model",
            "claude-opus-5-5",
            "--effort",
            "high",
            "--disallowedTools",
            "WebFetch,WebSearch",
        ]
        .map(String::from)
        .to_vec())
    );
    let (stands, open) = checked(Boundary::Open);
    assert_eq!(stands, json!({"kind": "open"}));
    assert_eq!(
        open,
        Err(
            "the final command of harness 'claude' is sealed with hands that are not the \
             engine's workspace hands; a complete command is parsed back before its spawn and \
             must express exactly the capability state its sealed plan records, so it is \
             refused rather than spawned (operator ruling 2 of 2026-09-23; design D6)"
                .to_string()
        )
    );
}

/// Rebuild unit 12-fix-c, the four review positions' reproductions compiled
/// and launched (design D6). Provenance is the engine's typed record, never
/// argv text. Under `harness` an adapter's managed `hands.harness.work` that
/// names the hands tool is still a limit, and the hands tool it allows is
/// refused by name, required or wanted, where it was once taken for the
/// box's hands and widened to `--tools WebFetch` (S1, C1, SC-2). A managed
/// allow list no typed contribution made is refused, alone or beside a
/// limit (S2). A template's `Bash(ls:*)` the site's typed allow did not
/// lower is refused (SC-1); lowered, it is admitted, and still bounded by
/// the template's limit. Accepted launches go through the seal.
#[test]
fn every_position_reproduction_refuses_by_provenance_and_typed_origins_launch() {
    let operator = Operator::new();
    fetch_dialect(&operator, "claude-native-fetch", json!(["WebFetch"]));
    let nothing = operator.context(json!({}));
    let fetch = operator.context(json!({"web-fetch": {"dialect": "claude-native-fetch"}}));
    one_inline_seat(&operator, &["driver"]);
    let seat = |office: &str| {
        write(
            operator.root(),
            "solo/bundle.json",
            &json!({"name": "solo", "policy": "policy.json", "seats": {
                "work": {"results": ["complete"], "agent": office},
                "review": {"results": ["clean"], "role": "roles/role.md",
                           "driver": {"command": ["driver"]}}}}),
        );
    };
    let refused = |office: &str, cause: &str| {
        format!("bundle: seat 'work' (office '{office}') in realm 'private': {cause}")
    };
    let untyped = |owner: &str, tool: &str| {
        format!(
            "{owner} '--allowedTools' allow list names tool '{tool}' for provider 'claude', \
             which no realm holding admits, the site's typed hands do not carry and its typed \
             'tools.allow' did not lower; an allowance is admitted by the typed contribution \
             that made it, never by its spelling or by the list it stands in (design D6)"
        )
    };
    let managed = "the adapter's managed boundary fragment's";
    // The hands office under `harness`, with the adapter's work fragment.
    seat("handed");
    let handed = |fragment: Value, asks: Value, context: &CapabilityContext| {
        write(
            operator.root(),
            "agents/handed.json",
            &json!({
                "description": "an office with hands",
                "charter": "charters/searcher.md",
                "models": ["opus"],
                "efforts": {"opus": "high"},
                "hands": {"kind": "workspace", "network": false, "binds": []},
                "capabilities": asks,
            }),
        );
        let adapters = copied_adapters();
        edit_adapter(adapters.path(), "claude", |adapter| {
            adapter["hands"]["harness"] = json!({"work": fragment});
        });
        solo_sealed(&operator, adapters.path(), context)
    };
    let counterfeit = json!([
        "--tools",
        "Read,Bash",
        "--allowedTools",
        "mcp__brokkr__workspace"
    ]);
    for strength in ["requires", "wants"] {
        assert_eq!(
            handed(counterfeit.clone(), json!({"web-fetch": strength}), &fetch),
            refused("handed", &untyped(managed, "mcp__brokkr__workspace")),
            "{strength}"
        );
    }
    assert_eq!(
        handed(json!(["--allowedTools", "Bash"]), json!({}), &nothing),
        refused("handed", &untyped(managed, "Bash"))
    );
    assert_eq!(
        handed(
            json!(["--tools", "Read", "--allowedTools", "Bash"]),
            json!({}),
            &nothing
        ),
        refused("handed", &untyped(managed, "Bash"))
    );
    // A template's allowance, and the site's typed allow beside a limit.
    seat("templated");
    let templated = |template: &[&str], allow: Option<Value>| {
        let mut office = json!({
            "description": "an office with one local command",
            "charter": "charters/searcher.md",
            "models": ["opus"],
            "efforts": {"opus": "high"},
        });
        if let Some(allow) = allow {
            office["tools"] = json!({"allow": allow});
        }
        write(operator.root(), "agents/templated.json", &office);
        let adapters = copied_adapters();
        edit_adapter(adapters.path(), "claude", |adapter| {
            let mut driver = json!([
                "{brokkr}",
                "driver",
                "claude",
                "--",
                "--permission-mode",
                "acceptEdits"
            ]);
            for part in template {
                driver.as_array_mut().unwrap().push(json!(part));
            }
            adapter["driver"] = driver;
        });
        solo_sealed(&operator, adapters.path(), &nothing)
    };
    // Named by its tool alone, never its permission payload, inside the
    // 512 scalar values D6 bounds the cause to — here the longest
    // grammar-valid pattern, once spelled whole (the returned review's
    // finding), its tool cut to what the cause's bound leaves it (rebuild
    // unit 12-fix-d).
    let longest = format!("B{}", "a".repeat(127));
    let secret = format!("{longest}(/{}/REVIEW_SENTINEL:*)", "s".repeat(230));
    let cause = untyped("the adapter template's", &format!("B{}…", "a".repeat(112)));
    assert_eq!(
        templated(&["--allowedTools", &secret], None),
        refused("templated", &cause)
    );
    assert!(cause.chars().count() <= 512, "{cause}");
    assert_eq!(
        templated(&["--allowedTools", "Bash(ls:*)"], None),
        refused("templated", &untyped("the adapter template's", "Bash"))
    );
    assert_eq!(
        templated(&["--tools", "Read"], Some(json!(["ls"]))),
        refused(
            "templated",
            "the adapter template's explicit '--tools' restriction for provider 'claude' \
             (naming Read) does not name tool 'Bash', which the local permissions of the site's \
             typed 'tools.allow' admit; an explicit tool list is a hard limit that nothing \
             widens, so the conflict is refused whole rather than unioned (design D6)"
        )
    );
    // The limit is named by bounded identities, never a pattern's payload,
    // and the rest counted past 48 scalar values (the second return's R1).
    let outside_local = |naming: &str| {
        format!(
            "the adapter template's explicit '--tools' restriction for provider 'claude' \
             (naming {naming}) does not name tool 'Bash', which the local permissions of the \
             site's typed 'tools.allow' admit; an explicit tool list is a hard limit that nothing \
             widens, so the conflict is refused whole rather than unioned (design D6)"
        )
    };
    let names = ["A", "B", "C"].map(|c| format!("{c}{}", "a".repeat(127)));
    for (limit, naming) in [
        ("Read(/private/REVIEW_SENTINEL)".to_string(), "Read(…)"),
        (names.join(","), "3 tools"),
    ] {
        let cause = outside_local(naming);
        assert_eq!(
            templated(&["--tools", &limit], Some(json!(["ls"]))),
            refused("templated", &cause)
        );
        assert!(cause.chars().count() <= 512, "{cause}");
    }
    assert_eq!(
        templated(&["--tools", "Read,Bash"], Some(json!(["ls"]))),
        launched_as(&[&[
            "--permission-mode",
            "acceptEdits",
            "--tools",
            "Bash",
            "--model",
            "claude-opus-5-5",
            "--effort",
            "high",
            "--allowedTools",
            "Bash(ls:*)",
            "--disallowedTools",
            "WebFetch,WebSearch"
        ]])
    );
}

/// Rebuild unit 12-fix-d, the chief's SC-1 and SC-2 compiled (design D6):
/// a realm-granted entry whose tool and ON allowance are
/// `Bash(/private/…:*)`, or the longest grammar-valid pattern, is refused
/// naming its tool alone; a
/// required 200-scalar capability whose 128-scalar tool the template's
/// limit excludes is refused with the capability cut to what keeps the
/// cause within 438 scalar values, so the compiler's whole line — its
/// `bundle: `, the site and the cause — stays within 512 (the second
/// return). The carried sibling, the template's own allowance, renders
/// through the same function.
#[test]
fn a_compiled_conflict_is_refused_in_bounded_identities() {
    const SENTINEL: &str = "REVIEW_SENTINEL";
    let operator = Operator::new();
    one_inline_seat(&operator, &["driver"]);
    write(
        operator.root(),
        "solo/bundle.json",
        &json!({"name": "solo", "policy": "policy.json", "seats": {
            "work": {"results": ["complete"], "agent": "bounded"},
            "review": {"results": ["clean"], "role": "roles/role.md",
                       "driver": {"command": ["driver"]}}}}),
    );
    // One realm-granted entry serving `capability` with `tool`, whose ON
    // allows `on`, required by the office under the template's `template`.
    let compiled = |capability: &str, tool: &str, on: &str, template: &[&str]| {
        fetch_dialect(&operator, "bounded", json!([tool]));
        let path = operator.root().join("dialects/tools/bounded.json");
        let mut dialect: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        dialect["serves"] = json!(capability);
        dialect["adapter_key"] = json!("bounded");
        write(operator.root(), "dialects/tools/bounded.json", &dialect);
        write(
            operator.root(),
            &format!("capabilities/{capability}.json"),
            &json!({"name": capability, "classes": ["reads", "egress"]}),
        );
        write(
            operator.root(),
            "agents/bounded.json",
            &json!({
                "description": "an office with one bounded capability",
                "charter": "charters/searcher.md",
                "models": ["opus"],
                "efforts": {"opus": "high"},
                "capabilities": {capability: "requires"},
            }),
        );
        let adapters = copied_adapters();
        edit_adapter(adapters.path(), "claude", |adapter| {
            let known = &mut adapter["native_capabilities"]["known"];
            let mut entry = known["web-fetch"].clone();
            entry["capability"] = json!(capability);
            entry["tools"] = json!([tool]);
            entry["on"] = json!({"argv": ["--allowedTools", on]});
            entry["off"] = json!({"argv": ["--disallowedTools", tool]});
            known["bounded"] = entry;
            let driver = adapter["driver"].as_array_mut().unwrap();
            driver.extend(template.iter().map(|part| json!(part)));
        });
        let context = operator.context(json!({capability: {"dialect": "bounded"}}));
        solo_sealed(&operator, adapters.path(), &context)
    };
    let refused = |cause: &str| {
        let line = format!("bundle: seat 'work' (office 'bounded') in realm 'private': {cause}");
        assert!(!line.contains(SENTINEL), "{line}");
        assert!(line.chars().count() <= 512, "{line}");
        line
    };
    let unheld = |tool: &str| {
        format!(
            "the capability plan admits tool '{tool}' for provider 'claude', which no realm \
             holding admits; a tool is admitted only through the one adapter entry a holding \
             binds, narrowed by its grant (design D6)"
        )
    };
    let tool = format!("T{}", "a".repeat(127));
    let payload = format!("{tool}(/{}/{SENTINEL}:*)", "s".repeat(230));
    let sentinel = format!("Bash(/private/{SENTINEL}:*)");
    assert_eq!(
        compiled("web-fetch", &sentinel, &sentinel, &[]),
        refused(&unheld("Bash"))
    );
    assert_eq!(
        compiled("web-fetch", &payload, &payload, &[]),
        refused(&unheld(&tool))
    );
    let capability = format!("c{}", "a".repeat(199));
    assert_eq!(
        compiled(&capability, &tool, &tool, &["--tools", "Read"]),
        refused(&format!(
            "the adapter template's explicit '--tools' restriction for provider 'claude' (naming \
             Read) does not name tool '{tool}', which the plan admits for native capability \
             'c{}…'; an explicit tool list is a hard limit that nothing widens, so the conflict \
             is refused whole rather than unioned (design D6)",
            "a".repeat(24)
        ))
    );
    assert_eq!(
        compiled(
            "web-fetch",
            "WebFetch",
            "WebFetch",
            &["--allowedTools", &payload]
        ),
        refused(&format!(
            "the adapter template's '--allowedTools' allow list names tool 'T{}…' for provider \
             'claude', which no realm holding admits, the site's typed hands do not carry and its \
             typed 'tools.allow' did not lower; an allowance is admitted by the typed \
             contribution that made it, never by its spelling or by the list it stands in \
             (design D6)",
            "a".repeat(112)
        ))
    );
}

/// Rebuild unit 12-fix-e, the chief's C1 (design D6): on a composed bundle
/// the compiler's whole line — `bundle: ` twice, the site, SC-2's
/// 438-scalar cause and the composition note — is 533 scalar values, and
/// renders through the one sink cut to 512, ending in `…`, still a
/// capability refusal.
#[test]
fn a_composed_capability_refusal_renders_as_one_bounded_line() {
    let operator = Operator::new();
    one_inline_seat(&operator, &["driver"]);
    write(
        operator.root(),
        "solo/bundle.json",
        &json!({"name": "solo", "policy": "policy.json", "seats": {
            "work": {"results": ["complete"], "agent": "bounded"},
            "review": {"results": ["clean"], "role": "roles/role.md",
                       "driver": {"command": ["driver"]}}}}),
    );
    write(
        operator.root(),
        "derived/bundle.json",
        &json!({"name": "derived", "extends": "solo"}),
    );
    let capability = format!("c{}", "a".repeat(199));
    let tool = format!("T{}", "a".repeat(127));
    fetch_dialect(&operator, "bounded", json!([tool]));
    let path = operator.root().join("dialects/tools/bounded.json");
    let mut dialect: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    dialect["serves"] = json!(capability);
    dialect["adapter_key"] = json!("bounded");
    write(operator.root(), "dialects/tools/bounded.json", &dialect);
    write(
        operator.root(),
        &format!("capabilities/{capability}.json"),
        &json!({"name": capability, "classes": ["reads", "egress"]}),
    );
    write(
        operator.root(),
        "agents/bounded.json",
        &json!({
            "description": "an office with one bounded capability",
            "charter": "charters/searcher.md",
            "models": ["opus"],
            "efforts": {"opus": "high"},
            "capabilities": {capability.as_str(): "requires"},
        }),
    );
    let adapters = copied_adapters();
    edit_adapter(adapters.path(), "claude", |adapter| {
        let known = &mut adapter["native_capabilities"]["known"];
        let mut entry = known["web-fetch"].clone();
        entry["capability"] = json!(capability);
        entry["tools"] = json!([tool]);
        entry["on"] = json!({"argv": ["--allowedTools", tool]});
        entry["off"] = json!({"argv": ["--disallowedTools", tool]});
        known["bounded"] = entry;
        let driver = adapter["driver"].as_array_mut().unwrap();
        driver.extend([json!("--tools"), json!("Read")]);
    });
    let context = operator.context(json!({capability.as_str(): {"dialect": "bounded"}}));
    let refusal = Bundle::compile_with_capabilities(
        &operator.root().join("derived"),
        &operator.root().join("agents"),
        adapters.path(),
        Some("private"),
        None,
        Boundary::Harness,
        &context,
    )
    .unwrap_err();
    let whole = format!(
        "bundle: bundle: seat 'work' (office 'bounded') in realm 'private': the adapter \
         template's explicit '--tools' restriction for provider 'claude' (naming Read) does not \
         name tool '{tool}', which the plan admits for native capability 'c{}…'; an explicit \
         tool list is a hard limit that nothing widens, so the conflict is refused whole rather \
         than unioned (design D6) (composed: derived -> solo)",
        "a".repeat(24)
    );
    assert_eq!(whole.chars().count(), 533);
    let line = refusal.to_string();
    assert_eq!(
        line,
        format!("{}…", whole.chars().take(511).collect::<String>())
    );
    assert_eq!(line.chars().count(), 512);
    assert!(matches!(
        refusal,
        brokkr_runtime::bundle::CompileError::Capability(_)
    ));
}

/// Rebuild unit 12-fix-d, the second return (design D6): a realm-granted
/// entry that admits `Bash` and a `Bash(/private/…:*)` its ON selection both
/// allows and denies is refused at compile by its tool name alone, on
/// Claude and LaneTally; the longest grammar-valid pattern, at a site whose
/// realm is 300 scalar values long, is refused with that realm named as a
/// bounded identity (rebuild unit 12-fix-f), the compiler's whole line
/// within 512.
#[test]
fn a_compiled_tool_both_admitted_and_denied_is_refused_by_its_bounded_identity() {
    const SENTINEL: &str = "REVIEW_SENTINEL";
    let tool = format!("T{}", "a".repeat(127));
    let sentinel = format!("Bash(/private/{SENTINEL}:*)");
    let payload = format!("{tool}(/{}/{SENTINEL}:*)", "s".repeat(230));
    let compiled = |driver: &str, pattern: &str, realm: &str| {
        let operator = Operator::new();
        one_inline_seat(
            &operator,
            &[
                "{brokkr}",
                "driver",
                driver,
                "--",
                "--model",
                "claude-opus-5-5",
                "--effort",
                "high",
            ],
        );
        let path = operator.root().join("solo/bundle.json");
        let mut bundle: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        bundle["seats"]["work"]["capabilities"] = json!({"web-fetch": "requires"});
        write(operator.root(), "solo/bundle.json", &bundle);
        let name = brokkr_protocol::native_controls::grammar::tool_name(pattern);
        fetch_dialect(&operator, "bounded", json!([name, pattern]));
        let path = operator.root().join("dialects/tools/bounded.json");
        let mut dialect: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        dialect["provider"] = json!(driver);
        write(operator.root(), "dialects/tools/bounded.json", &dialect);
        let adapters = copied_adapters();
        let claude: Value =
            serde_json::from_slice(&std::fs::read(adapters.path().join("claude.json")).unwrap())
                .unwrap();
        edit_adapter(adapters.path(), driver, |adapter| {
            adapter["native_capabilities"] = claude["native_capabilities"].clone();
            let fetch = &mut adapter["native_capabilities"]["known"]["web-fetch"];
            fetch["tools"] = json!([name, pattern]);
            fetch["on"] =
                json!({"selection": {"include": [], "allow": [pattern], "deny": [pattern]}});
        });
        let mut context = operator.context(json!({"web-fetch": {"dialect": "bounded"}}));
        context.realm = realm.to_string();
        solo_sealed(&operator, adapters.path(), &context)
    };
    let refused = |site: &str, named: &str, driver: &str| {
        let line = format!(
            "bundle: {site}: the capability plan carries tool '{named}' both admitted and denied \
             for provider '{driver}', which its launch does not consume; a control that cannot \
             reach the final command is refused rather than recorded and dropped (decision 0066 \
             ruling 3)"
        );
        assert!(!line.contains(SENTINEL), "{line}");
        assert!(line.chars().count() <= 512, "{line}");
        line
    };
    let long = "r".repeat(300);
    // The realm is a bounded identity (rebuild unit 12-fix-f): its lead
    // and its length, whole beside either cause. LaneTally's name is three
    // scalar values longer than Claude's.
    for (driver, scalars) in [("claude", 475), ("lanetally", 478)] {
        assert_eq!(
            compiled(driver, &sentinel, "private"),
            refused(
                "seat 'work' (office 'work') in realm 'private'",
                "Bash",
                driver
            ),
            "{driver}"
        );
        let line = compiled(driver, &payload, &long);
        assert_eq!(
            line,
            refused(
                &format!(
                    "seat 'work' (office 'work') in realm '{}…' (300 bytes, not echoed in full)",
                    "r".repeat(32)
                ),
                &tool,
                driver
            ),
            "{driver}"
        );
        assert_eq!(line.chars().count(), scalars, "{driver}");
    }
}

/// `launched […]` for the Claude head followed by `parts`, concatenated.
fn launched_as(parts: &[&[&str]]) -> String {
    let head: &[&str] = &[
        "claude",
        "-p",
        "--output-format",
        "stream-json",
        "--verbose",
    ];
    format!(
        "launched {:?}",
        std::iter::once(head)
            .chain(parts.iter().copied())
            .flatten()
            .collect::<Vec<_>>()
    )
}
