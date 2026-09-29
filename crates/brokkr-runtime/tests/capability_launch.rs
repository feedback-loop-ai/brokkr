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
        None => inline_command(bundle, label),
    };
    // Composed by the engine's own function, so the boundary's fragment —
    // the box's tokens, or the harness's own sandbox under `harness` — is
    // in the argv exactly as it is at a real spawn, and an inline site's
    // lowered allow is the engine's own segment, as dispatch composes it.
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
        facts.chain.get(candidate),
        Path::new("/w"),
        &[],
        "/w/result.json",
        None,
    );
    // The plan AND the argv's two parts, each exactly as the engine writes
    // it: the driver refuses a launch whose provenance it cannot reassemble.
    let mut input = json!({"workdir": "/w", "seat": label, "native_controls": outcome.controls(),
                           "launch_arguments": spawn.launch_arguments()});
    // Beside the plan, the record and serving inputs sealed exactly as
    // dispatch seals them (`mark_capabilities`), or dispatch's refusal
    // where it seals none (rebuild unit 15-fix-a).
    seal_as_dispatch(bundle, label, candidate, &mut spawn, &mut input)?;
    let argv = &spawn.argv;
    let extra = &argv[argv.iter().position(|part| part == "--").unwrap() + 1..];
    match outcome.provider.as_str() {
        "codex" => brokkr_protocol::adapters::codex_command("codex", extra, "/w", None, &input),
        _ => brokkr_protocol::adapters::claude_command("claude", extra, None, &input),
    }
}

/// Seal `spawn`'s launch record and the serving inputs beside it into
/// `input`, through the engine's own `expected_state` and `serving_inputs`
/// in `mark_capabilities`' order, or answer the refusal dispatch sets on a
/// spawn it cannot seal.
fn seal_as_dispatch(
    bundle: &Bundle,
    label: &str,
    candidate: usize,
    spawn: &mut brokkr_runtime::engine::SiteSpawn,
    input: &mut Value,
) -> Result<(), String> {
    use brokkr_protocol::native_controls::SERVING_INPUTS;
    use brokkr_runtime::engine::{expected_state, serving_inputs, LAUNCH_RECORD};
    let facts = &bundle.sites[label];
    let outcome = &facts.capabilities.as_ref().unwrap().outcomes[candidate];
    let link = facts.chain.get(candidate);
    let expected = expected_state(outcome, link, Some(facts))?;
    let serving = serving_inputs(link, Some(facts), spawn.class, bundle.boundary)?;
    spawn.seal(expected)?;
    input[LAUNCH_RECORD] = spawn.launch_record();
    input[SERVING_INPUTS] = serving.value();
    spawn.serving = Some(serving);
    Ok(())
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
    use brokkr_protocol::native_controls::SERVING_INPUTS;
    use brokkr_runtime::engine::{verify_record, LAUNCH_RECORD};
    let facts = &bundle.sites[label];
    let (mut spawn, sealing) = sealing(bundle, label, candidate, facts);
    assert_eq!(sealing, Ok(()), "{label}[{candidate}]");
    let serving = seal_serving(bundle, &mut spawn, facts, facts.chain.get(candidate));
    let input = json!({LAUNCH_RECORD: spawn.launch_record(), SERVING_INPUTS: serving});
    assert_eq!(
        verify_record(&spawn, &input),
        Ok(()),
        "{label}[{candidate}]"
    );
    (spawn, input)
}

/// The serving inputs dispatch seals beside `spawn`'s record (rebuild unit
/// 14a2), set on the spawn so its door admits them and returned as the
/// driver is handed them. They are sealed by the engine's own
/// `serving_inputs` under the bundle's boundary, as dispatch seals them.
fn seal_serving(
    bundle: &Bundle,
    spawn: &mut brokkr_runtime::engine::SiteSpawn,
    facts: &brokkr_runtime::bundle::SiteFacts,
    link: Option<&brokkr_runtime::agents::Candidate>,
) -> Value {
    let sealed =
        brokkr_runtime::engine::serving_inputs(link, Some(facts), spawn.class, bundle.boundary)
            .expect("the site's serving inputs");
    let value = sealed.value();
    spawn.serving = Some(sealed);
    value
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

/// The authored command of the inline site `label`, at whatever site shape
/// the seat's body holds it ([`body_command`]).
fn inline_command(bundle: &Bundle, label: &str) -> Vec<String> {
    let (seat, tag) = label.split_once(':').unwrap_or((label, ""));
    body_command(&bundle.seats[seat].body, tag)
        .unwrap_or_else(|| panic!("{label} is no inline site"))
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
    let work = brokkr_runtime::SeatClass::Work;
    sealing_at(bundle, label, candidate, facts, work, "/w", moved)
}

/// [`sealing_moved`] at `class` in `workdir`, with the result file inside
/// it, under the boundary `compose_at` builds: none of its own for a site
/// without hands, the bundle's for one with them.
fn sealing_at(
    bundle: &Bundle,
    label: &str,
    candidate: usize,
    facts: &brokkr_runtime::bundle::SiteFacts,
    class: brokkr_runtime::SeatClass,
    workdir: &str,
    moved: impl FnOnce(&mut brokkr_runtime::engine::SiteSpawn),
) -> (brokkr_runtime::engine::SiteSpawn, Result<(), String>) {
    use brokkr_runtime::engine::{expected_state, BuiltBoundary};
    let outcome = &facts.capabilities.as_ref().unwrap().outcomes[candidate];
    let link = facts.chain.get(candidate);
    let argv: Vec<String> = match link {
        Some(link) => link.argv.clone(),
        None => inline_command(bundle, label),
    };
    let built = match (bundle.hands.get(label), bundle.boundary) {
        (None, _) | (_, Boundary::Open) => BuiltBoundary::Open,
        (_, boxed) if boxed.is_boxed() => BuiltBoundary::Namespace,
        _ => BuiltBoundary::Harness,
    };
    let mut spawn = brokkr_runtime::engine::compose_site_at(
        Some(facts),
        built,
        class,
        argv,
        bundle.hands.get(label),
        link,
        Path::new(workdir),
        &[],
        &format!("{workdir}/result.json"),
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
        None => (
            inline_command(bundle, label),
            facts.inline_resume.clone().expect("an inline codex seat"),
            "not applicable",
        ),
    };
    // Composed from the site's facts, as dispatch composes it, so an inline
    // seat's lowered class is the engine's own segment (rebuild unit 5d).
    let mut spawn = brokkr_runtime::engine::compose_site_at(
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
    let mut input = json!({
        "workdir": "/w", "seat": label, "boundary": boundary, "hands": "none",
        "resume_context": {"assessment": assessment},
        "native_controls": outcome.controls(),
        "launch_arguments": spawn.launch_arguments(),
    });
    seal_as_dispatch(bundle, label, 0, &mut spawn, &mut input)
        .unwrap_or_else(|refusal| panic!("{label} unsealed: {refusal}"));
    let argv = &spawn.argv;
    let extra = &argv[argv.iter().position(|part| part == "--").unwrap() + 1..];
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
    // Rebuild unit 20: beside its hands, each boxed site is told the
    // charter the compile selected for it — the inline seat its layer's
    // role, the fallback its office's library charter.
    use brokkr_runtime::bundle::CharterOwner;
    let told = |label: &str| {
        let pin = bundle.sites[label].charter.as_ref().unwrap();
        (
            pin.owner.clone(),
            pin.reference.clone(),
            pin.path.clone(),
            pin.digest.clone(),
        )
    };
    let (recipe, library) = (
        operator.root().join("bundle"),
        operator.root().join("agents"),
    );
    assert_eq!(
        [told("boxed"), told("chain")],
        [
            (
                CharterOwner::Layer {
                    dir: recipe.clone(),
                    key: "roles/role.md".into(),
                },
                "roles/role.md".to_string(),
                recipe.join("roles/role.md"),
                brokkr_core::canonical::sha256_bytes(b"# role\n"),
            ),
            (
                CharterOwner::Library {
                    agent: "fallback".into(),
                    root: library.clone(),
                },
                "charters/searcher.md".to_string(),
                library.join("charters/searcher.md"),
                brokkr_core::canonical::sha256_bytes(b"# searcher\n"),
            ),
        ]
    );
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
    // Rebuild unit 20 (review return R2): beside its checked command, each
    // boxed member is told the charter the compile selected for it — the
    // inline member its layer's role, the agent member its office's
    // library charter.
    use brokkr_runtime::bundle::CharterOwner;
    let told = |label: &str| {
        let pin = bundle.sites[label].charter.as_ref().unwrap();
        (
            pin.owner.clone(),
            pin.reference.clone(),
            pin.path.clone(),
            pin.digest.clone(),
        )
    };
    let (recipe, library) = (
        operator.root().join("bundle"),
        operator.root().join("agents"),
    );
    assert_eq!(
        [told("judges:inline"), told("judges:agent")],
        [
            (
                CharterOwner::Layer {
                    dir: recipe.clone(),
                    key: "roles/role.md".into(),
                },
                "roles/role.md".to_string(),
                recipe.join("roles/role.md"),
                brokkr_core::canonical::sha256_bytes(b"# role\n"),
            ),
            (
                CharterOwner::Library {
                    agent: "searcher".into(),
                    root: library.clone(),
                },
                "charters/searcher.md".to_string(),
                library.join("charters/searcher.md"),
                brokkr_core::canonical::sha256_bytes(b"# searcher\n"),
            ),
        ]
    );
}

/// [`sealed_launch`] with the input dispatch writes changed by `tamper`
/// before the driver is handed it, and the extras by `extra`.
fn tampered_launch(
    bundle: &Bundle,
    label: &str,
    candidate: usize,
    tamper: impl FnOnce(&mut Value, &mut Vec<String>),
) -> Result<Vec<String>, String> {
    let (spawn, mut input) = sealed(bundle, label, candidate);
    let outcome = &bundle.sites[label].capabilities.as_ref().unwrap().outcomes[candidate];
    input["workdir"] = json!("/w");
    input["seat"] = json!(label);
    input["native_controls"] = outcome.controls();
    input["launch_arguments"] = spawn.launch_arguments();
    let argv = &spawn.argv;
    let mut extra = argv[argv.iter().position(|part| part == "--").unwrap() + 1..].to_vec();
    tamper(&mut input, &mut extra);
    match outcome.provider.as_str() {
        "codex" => brokkr_protocol::adapters::codex_command("codex", &extra, "/w", None, &input),
        _ => brokkr_protocol::adapters::claude_command("claude", &extra, None, &input),
    }
}

/// Rebuild unit 14 (task 14.1): at the Codex and Claude cold seams, a
/// compiled, sealed launch is served only as the final check returns it,
/// written out here independently of any composer; and the driver refuses
/// a command that departs from its sealed inputs. The plan's OFF dropped,
/// a sealed fragment's separator changed, and an authored option that
/// repeats one the driver composes (a cross-origin duplicate) each refuse
/// with the check's whole reason. The command is never read back from the
/// argv, so none of them is reconciled.
#[test]
fn a_compiled_cold_command_is_served_only_as_its_final_check_returns_it() {
    let operator = Operator::new();
    let context = CapabilityContext::no_grants("private", operator.root());
    let boxed = operator
        .compile(&context, Boundary::Namespace, None, Some(json!({})))
        .unwrap();
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
    let unboxed = solo_bundle(&operator, &workspace().join("adapters"), &context).unwrap();
    // An authored option inserted first, as the recipe's own word: in the
    // extras, the argv's authored part and the record's authored segment.
    type Tamper = Box<dyn FnOnce(&mut Value, &mut Vec<String>)>;
    let authored = |option: &'static str| -> Tamper {
        Box::new(move |input, extra| {
            extra.insert(0, option.into());
            for part in [
                "/launch_arguments/authored",
                "/launch_record/segments/0/argv",
            ] {
                let part = input.pointer_mut(part).unwrap();
                part.as_array_mut().unwrap().insert(0, json!(option));
            }
        })
    };
    let refused = |harness: &str, problem: &str| {
        Err(format!(
            "refusing to invoke the agent CLI: the final command of harness '{harness}' \
             {problem}; a complete command is parsed back before its spawn and must express \
             exactly the capability state its sealed plan records, so it is refused rather than \
             spawned (operator ruling 2 of 2026-09-23; design D6)"
        ))
    };
    let repeats = |at: usize, option: &str| {
        format!(
            "cannot be read whole (argument {at}, '{option}': it repeats option '{option}', \
             which the grammar admits once; a CLI that resolves a duplicate last-wins would \
             resolve it against the control the engine composed)"
        )
    };
    // The shipped Claude adapter's `hands.workspace` fragment, expanded for
    // the box as `boxed_hands` expands Codex's.
    let adapter: Value =
        serde_json::from_slice(&std::fs::read(workspace().join("adapters/claude.json")).unwrap())
            .unwrap();
    let fragment: Vec<String> =
        serde_json::from_value(adapter["hands"]["workspace"].clone()).unwrap();
    let claude_hands = brokkr_protocol::native_controls::Transport {
        brokkr: &std::env::current_exe().unwrap(),
        workdir: Path::new("/w"),
        spec: &boxed.hands["chain"],
    }
    .expand(&fragment)
    .unwrap();
    let words = |words: &[&str]| {
        words
            .iter()
            .map(|word| word.to_string())
            .collect::<Vec<_>>()
    };
    let claude = |mode: &[&str], hands: &[String], deny: &[&str]| {
        let lead = [
            "claude",
            "-p",
            "--output-format",
            "stream-json",
            "--verbose",
        ];
        let pins = ["--model", "claude-opus-5-5", "--effort", "high"];
        Ok([
            words(&lead),
            words(mode),
            words(&pins),
            hands.to_vec(),
            words(deny),
        ]
        .concat())
    };
    let deny = ["--disallowedTools", "WebFetch,WebSearch"];
    let mode = ["--permission-mode", "acceptEdits"];
    let codex = Ok(checked_codex(&boxed_hands(&boxed, "boxed")));
    type Row<'a> = (
        &'static str,
        &'a Bundle,
        &'static str,
        usize,
        Tamper,
        Result<Vec<String>, String>,
    );
    let rows: Vec<Row> = vec![
        (
            "claude as sealed",
            &unboxed,
            "work",
            0,
            Box::new(|_, _| {}),
            claude(&[], &[], &deny),
        ),
        (
            "boxed claude as sealed",
            &boxed,
            "chain",
            0,
            Box::new(|_, _| {}),
            claude(&mode, &claude_hands, &deny),
        ),
        (
            "codex as sealed",
            &boxed,
            "chain",
            1,
            Box::new(|_, _| {}),
            codex.clone(),
        ),
        (
            "inline codex as sealed",
            &boxed,
            "boxed",
            0,
            Box::new(|_, _| {}),
            codex,
        ),
        (
            "claude, the OFF dropped",
            &unboxed,
            "work",
            0,
            Box::new(|input, _| input["native_controls"]["selection"]["deny"] = json!([])),
            refused(
                "claude",
                "leaves tool 'WebFetch' available, which its plan denies as native capability \
                 'web-fetch'",
            ),
        ),
        // Behind `--tools ""` neither tool is available, so the denial is
        // still what the command expresses.
        (
            "boxed claude, the OFF dropped behind an empty tool list",
            &boxed,
            "chain",
            0,
            Box::new(|input, _| input["native_controls"]["selection"]["deny"] = json!([])),
            claude(&mode, &claude_hands, &[]),
        ),
        (
            "claude, the denial's separator changed",
            &unboxed,
            "work",
            0,
            Box::new(|input, _| {
                input["native_controls"]["selection"]["flags"]["deny"]["separator"] = json!(":")
            }),
            refused(
                "claude",
                "cannot be read: it carries '--disallowedTools' (argument 9), whose value names \
                 a tool that is not a plain name of ASCII letters, digits and '_' leading with a \
                 letter, within 128 bytes",
            ),
        ),
        (
            "codex, the OFF dropped",
            &boxed,
            "chain",
            1,
            Box::new(|input, _| input["native_controls"]["argv"] = json!([])),
            refused(
                "codex",
                "carries no measured OFF for native capability 'web-search', which its plan \
                 denies",
            ),
        ),
        (
            "claude, an authored --verbose beside the driver's",
            &unboxed,
            "work",
            0,
            authored("--verbose"),
            refused("claude", &repeats(5, "--verbose")),
        ),
        (
            "inline codex, an authored --json beside the driver's",
            &boxed,
            "boxed",
            0,
            authored("--json"),
            refused("codex", &repeats(7, "--json")),
        ),
    ];
    assert_eq!(rows.len(), 10);
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(|(label, bundle, site, candidate, tamper, expected)| {
            let observed = tampered_launch(bundle, site, candidate, tamper);
            (observed != expected)
                .then(|| format!("row {label}:\n  left:  {observed:?}\n  right: {expected:?}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
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
/// nothing else. Provider-free: the wrapper is a [`recording_harness`],
/// and the driver is this test binary re-entered ([`serve_driver`]),
/// because the serving branch is reached only through the driver's own
/// stdin protocol.
#[cfg(unix)]
#[test]
fn an_inline_lanetally_seats_typed_allow_reaches_the_wrappers_final_command_with_native_off() {
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
    let wrapper = recording_harness(root, "lanetally");
    let recorded = root.join("lanetally-argv");

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
        "serving_inputs": sealed_input["serving_inputs"],
    });
    let home = root.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let serve = |input: &Value| {
        let env = [
            ("BROKKR_LANETALLY_BIN", wrapper.as_path()),
            ("HOME", home.as_path()),
        ];
        serve_driver("lanetally", &extra, &env, None, input)
    };

    // Rebuild unit 14: the same sealed launch with its plan's denial
    // dropped is refused at the wrapper's cold seam, and the wrapper is
    // never spawned.
    let mut undenied = input.clone();
    undenied["native_controls"]["selection"]["deny"] = json!([]);
    let said = serve(&undenied);
    assert_eq!(
        result_error(&said),
        "refusing to invoke the agent CLI: the final command of harness 'lanetally' leaves \
         tool 'WebFetch' available, which its plan denies as native capability 'web-fetch'; a \
         complete command is parsed back before its spawn and must express exactly the \
         capability state its sealed plan records, so it is refused rather than spawned \
         (operator ruling 2 of 2026-09-23; design D6)",
        "the driver said: {said}"
    );
    assert!(!recorded.exists(), "the wrapper was spawned: {said}");

    let said = serve(&input);
    let spawned = recorded_argv(&recorded)
        .unwrap_or_else(|| panic!("the wrapper was never spawned; the driver said: {said}"));
    assert_eq!(
        spawned,
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
    use brokkr_protocol::native_controls::SERVING_INPUTS;
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
    let serving = seal_serving(bundle, &mut spawn, facts, None);
    let gate = class == brokkr_runtime::SeatClass::Gate;
    let door = result_door(Boundary::Harness, gate, Some(facts), None).word();
    // Handed as dispatch hands it: the seat, the native plan, the result
    // path the spawn was composed with, and the door `mark_delivery` writes
    // where it is the capture.
    let mut handed = json!({LAUNCH_RECORD: record, SERVING_INPUTS: serving, "seat": label,
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
/// An unmeasured inventory seals no denial, and the door admits its launch
/// without a plan; since rebuild unit 14 the driver then refuses it, because
/// a sealed cold command is served only once it is checked against its plan.
/// An unreadable plan refuses with one fixed cause, and neither a
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
        provenance: Default::default(),
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
            "refused: refusing to invoke the agent CLI: the input carries a sealed launch record \
             or sealed serving inputs without the capability plan and the record they are sealed \
             beside, so its final command cannot be checked; a sealed launch is never served \
             unchecked (rebuild unit 14; design D6)"
                .into(),
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
        // Rebuild unit 20: each rejoined site is told the charter the
        // compile selected for it — the inline seat its layer's role, the
        // agent-backed seat its library's charter.
        use brokkr_runtime::bundle::CharterOwner;
        let told = |label: &str| {
            let pin = bundle.sites[label].charter.as_ref().unwrap();
            (
                pin.owner.clone(),
                pin.reference.clone(),
                pin.path.clone(),
                pin.digest.clone(),
            )
        };
        let (recipe, library) = (
            operator.root().join("bundle"),
            operator.root().join("agents"),
        );
        assert_eq!(
            [told("inline"), told("agent")],
            [
                (
                    CharterOwner::Layer {
                        dir: recipe.clone(),
                        key: "roles/role.md".into(),
                    },
                    "roles/role.md".to_string(),
                    recipe.join("roles/role.md"),
                    brokkr_core::canonical::sha256_bytes(b"# role\n"),
                ),
                (
                    CharterOwner::Library {
                        agent: "searcher".into(),
                        root: library.clone(),
                    },
                    "charters/searcher.md".to_string(),
                    library.join("charters/searcher.md"),
                    brokkr_core::canonical::sha256_bytes(b"# searcher\n"),
                ),
            ],
            "{case}"
        );
    }
}

/// A stand-in `claude` answering the version probe, as [`codex_reporting`]
/// answers Codex's.
#[cfg(unix)]
fn claude_reporting(dir: &Path, version: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let staged = dir.join("claude.staged");
    std::fs::write(
        &staged,
        format!("#!/bin/sh\nprintf '{version} (Claude Code)\\n'\n"),
    )
    .unwrap();
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).unwrap();
    let shim = dir.join("claude");
    std::fs::rename(&staged, &shim).unwrap();
    shim
}

/// [`tampered_launch`] offered `session`: the site's spawn sealed as
/// dispatch seals it and the input the door admits, with the site's
/// confinement markers and a resume `assessment` beside it, changed by
/// `tamper` and handed to its provider's driver run as `bin`.
#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn sealed_rejoin(
    bundle: &Bundle,
    label: &str,
    candidate: usize,
    bin: &Path,
    session: &str,
    markers: [&str; 2],
    assessment: Value,
    tamper: impl FnOnce(&mut Value),
) -> Result<Vec<String>, String> {
    use brokkr_protocol::native_controls::SERVING_INPUTS;
    use brokkr_runtime::engine::{verify_record, LAUNCH_RECORD};
    let facts = &bundle.sites[label];
    let (mut spawn, sealing) = sealing(bundle, label, candidate, facts);
    assert_eq!(sealing, Ok(()), "{label}[{candidate}]");
    let serving = seal_serving(bundle, &mut spawn, facts, facts.chain.get(candidate));
    let outcome = &facts.capabilities.as_ref().unwrap().outcomes[candidate];
    let mut input = json!({LAUNCH_RECORD: spawn.launch_record(), SERVING_INPUTS: serving,
                           "workdir": "/w", "seat": label, "native_controls": outcome.controls(),
                           "launch_arguments": spawn.launch_arguments()});
    assert_eq!(
        verify_record(&spawn, &input),
        Ok(()),
        "{label}[{candidate}]"
    );
    input["boundary"] = json!(markers[0]);
    input["hands"] = json!(markers[1]);
    input["resume_context"] = json!({"assessment": assessment});
    tamper(&mut input);
    let argv = &spawn.argv;
    let extra = &argv[argv.iter().position(|part| part == "--").unwrap() + 1..];
    let bin = bin.to_str().unwrap();
    match outcome.provider.as_str() {
        "codex" => {
            brokkr_protocol::adapters::codex_command(bin, extra, "/w", Some(session), &input)
        }
        _ => brokkr_protocol::adapters::claude_command(bin, extra, Some(session), &input),
    }
}

/// Rebuild unit 15 (tasks 15.1 and 15.2): a compiled, sealed launch offered
/// its session is served only as the final check returns it, checked with
/// the session it rejoins, and written out here independently of any
/// composer. An eligible Codex rejoin, inline and agent-backed, is an actual
/// `exec resume` that carries the OFF exactly where search is not held; an
/// eligible Claude rejoin, unboxed and boxed, is its cold command then
/// `--resume <id>`, the prompt staying on stdin. The selected fallback, a
/// boxed Codex lane its shipped assessment does not admit, is declined and
/// served its checked cold command with its OFF, which is no resume. A
/// dropped OFF, an unreadable sealed input, a record missing beside its
/// serving inputs, and a record that counterfeits an origin (the engine's
/// segment as the recipe's, or the recipe's words as the engine's) each
/// refuse the rejoin with the whole reason; so does a record emptied,
/// reversed or grown by a hands token, which no longer reassembles the
/// arguments the driver was handed, on a rejoin and on the fallback served
/// cold alike. The Codex
/// assessments are the ones the bundle compiled from the shipped adapter;
/// Claude's shipped shape is unmeasured, so its rows are handed a supported
/// one to reach the rejoin at all.
#[cfg(unix)]
#[test]
fn a_compiled_rejoin_is_served_only_as_its_final_check_returns_it() {
    let operator = Operator::new();
    let codex = codex_reporting(operator.root(), "0.154.0");
    let claude = claude_reporting(operator.root(), "2.1.266");
    let denied_context = CapabilityContext::no_grants("private", operator.root());
    let wants = Some(json!({"web-search": "wants"}));
    let denied = operator
        .compile(&denied_context, Boundary::Harness, wants.clone(), None)
        .unwrap();
    let held = operator
        .compile(
            &operator.context(json!({"web-search": {"dialect": "codex-native-search"}})),
            Boundary::Harness,
            wants,
            None,
        )
        .unwrap();
    let boxed = operator
        .compile(&denied_context, Boundary::Namespace, None, Some(json!({})))
        .unwrap();
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
    let unboxed = solo_bundle(&operator, &workspace().join("adapters"), &denied_context).unwrap();
    let session = "019c4b7e-0000-7000-8000-000000000001";
    let words = |program: &Path, words: &[&str]| {
        [
            vec![program.to_str().unwrap().to_string()],
            words.iter().map(|word| word.to_string()).collect(),
        ]
        .concat()
    };
    let refused = |harness: &str, problem: &str| {
        Err(format!(
            "refusing to invoke the agent CLI: the final command of harness '{harness}' \
             {problem}; a complete command is parsed back before its spawn and must express \
             exactly the capability state its sealed plan records, so it is refused rather than \
             spawned (operator ruling 2 of 2026-09-23; design D6)"
        ))
    };

    // The Codex rejoin, with and without the OFF before its positionals.
    let resumed = |off: bool| {
        let mut argv = words(
            &codex,
            &[
                "exec",
                "resume",
                "--json",
                "-c",
                "sandbox_mode=\"workspace-write\"",
                "-c",
                "model_reasoning_effort=\"high\"",
                "--model",
                "gpt-6-astra",
            ],
        );
        if off {
            argv.extend(OFF.map(String::from));
        }
        argv.extend([THREAD.to_string(), "-".to_string()]);
        Ok(argv)
    };
    let compiled = |bundle: &Bundle, label: &str, candidate: usize| match bundle.sites[label]
        .chain
        .get(candidate)
    {
        Some(link) => link.resume.value(),
        None => bundle.sites[label].inline_resume.clone().unwrap(),
    };
    // The box's hands as independent literals, never production's own
    // expansion (NC6): only the test's executable, the canonical fixture
    // value `{brokkr}` binds, is substituted.
    let exe = std::env::current_exe().unwrap();
    let exe = exe.to_str().unwrap();
    let serve = r#"["hands","serve","--workdir","/w","--spec","{\"binds\":[],\"kind\":\"workspace\",\"network\":false}"]"#;
    // The selected fallback's cold command, its hands expanded for the box.
    let fallback: Vec<String> = [
        codex.to_str().unwrap(),
        "exec",
        "--json",
        "-C",
        "/w",
        "-c",
        "model_reasoning_effort=\"high\"",
        "--model",
        "gpt-6-astra",
        "--sandbox",
        "read-only",
        "-c",
        &format!("mcp_servers.brokkr.command=\"{exe}\""),
        "-c",
        &format!("mcp_servers.brokkr.args={serve}"),
        "-c",
        "mcp_servers.brokkr.default_tools_approval_mode=\"approve\"",
        "-c",
        "web_search=\"disabled\"",
    ]
    .map(String::from)
    .to_vec();

    // The Claude rejoin: the cold command, then the session it rejoins.
    let enabled = |boundary: &str, hands: &str| {
        json!({"boxed-workspace": {
            "status": "supported",
            "identity": {"version": "2.1.266", "applies_to": "2.1.266"},
            "classes": ["work"], "boundaries": [boundary], "hands": hands,
            "evidence": {"interface": "i", "restrictions": "r", "root": "o", "accounting": "a"},
            "limitations": [], "reason": null}})
    };
    let claude_hands: Vec<String> = [
        "--tools",
        "",
        "--strict-mcp-config",
        "--mcp-config",
        &format!(r#"{{"mcpServers":{{"brokkr":{{"args":{serve},"command":"{exe}"}}}}}}"#),
        "--allowedTools",
        "mcp__brokkr__workspace",
    ]
    .map(String::from)
    .to_vec();
    let rejoined_claude = |mode: &[&str], hands: &[String]| {
        Ok([
            words(
                &claude,
                &["-p", "--output-format", "stream-json", "--verbose"],
            ),
            mode.iter().map(|word| word.to_string()).collect(),
            ["--model", "claude-opus-5-5", "--effort", "high"]
                .map(String::from)
                .to_vec(),
            hands.to_vec(),
            [
                "--disallowedTools",
                "WebFetch,WebSearch",
                "--resume",
                session,
            ]
            .map(String::from)
            .to_vec(),
        ]
        .concat())
    };
    let mode = ["--permission-mode", "acceptEdits"];

    // A record that no longer reassembles the arguments the driver was
    // handed, refused before any of its origins is read.
    let unassembled = |first: usize, recorded: usize, supplied: usize| {
        Err(format!(
            "refusing the private launch record: its segments do not reassemble the arguments \
             supplied; they first differ at argument {first} ({recorded} recorded, {supplied} \
             supplied), and an argument whose origin is not recorded is never trusted by its \
             bytes (decision 0065 slice one, design D5.7)"
        ))
    };
    let segments = |tamper: fn(&mut Vec<Value>)| -> Tamper {
        Box::new(move |input| tamper(input["launch_record"]["segments"].as_array_mut().unwrap()))
    };
    let grown = |segments: &mut Vec<Value>| {
        segments.push(json!({"origin": "hands", "argv": ["mcp__brokkr__workspace"]}))
    };

    type Tamper = Box<dyn FnOnce(&mut Value)>;
    type Row<'a> = (
        &'static str,
        &'a Bundle,
        &'static str,
        usize,
        [&'static str; 2],
        Tamper,
        Result<Vec<String>, String>,
    );
    let untouched = || -> Tamper { Box::new(|_| {}) };
    let rows: Vec<Row> = vec![
        (
            "codex inline, denied",
            &denied,
            "inline",
            0,
            ["not applicable", "none"],
            untouched(),
            resumed(true),
        ),
        (
            "codex agent, denied",
            &denied,
            "agent",
            0,
            ["harness", "none"],
            untouched(),
            resumed(true),
        ),
        (
            "codex inline, held",
            &held,
            "inline",
            0,
            ["not applicable", "none"],
            untouched(),
            resumed(false),
        ),
        (
            "codex agent, held",
            &held,
            "agent",
            0,
            ["harness", "none"],
            untouched(),
            resumed(false),
        ),
        (
            "codex agent, the OFF dropped",
            &denied,
            "agent",
            0,
            ["harness", "none"],
            Box::new(|input| input["native_controls"]["argv"] = json!([])),
            refused(
                "codex",
                "carries no measured OFF for native capability 'web-search', which its plan \
                 denies",
            ),
        ),
        (
            "codex inline, its serving inputs unreadable",
            &denied,
            "inline",
            0,
            ["not applicable", "none"],
            Box::new(|input| input["serving_inputs"]["dialect"]["sandbox"] = json!("x\nsecret")),
            Err(
                "refusing the sealed serving inputs: 'serving.dialect.sandbox' is not an array; the \
                 inputs a final command is rebuilt from are never repaired into empty or \
                 default ones, nor recovered from its argv (rebuild unit 14a2; design D5.7, D6)"
                    .to_string(),
            ),
        ),
        (
            "codex agent, its engine segment counterfeited as the recipe's",
            &denied,
            "agent",
            0,
            ["harness", "none"],
            Box::new(|input| {
                let segments = input["launch_record"]["segments"].as_array_mut().unwrap();
                segments.last_mut().unwrap()["origin"] = json!("authored");
            }),
            refused(
                "codex",
                "is served with the recipe's words or its adapter's pins carrying what cannot be \
                 read, a session or a capability-bearing effect, which only its sealed plan \
                 composes",
            ),
        ),
        (
            "codex inline, the recipe's words counterfeited as the engine's",
            &denied,
            "inline",
            0,
            ["not applicable", "none"],
            Box::new(|input| input["launch_record"]["segments"][0]["origin"] = json!("local")),
            refused(
                "codex",
                "departs at argument 7 from the complete command its sealed inputs and the \
                 engine's serving choices rebuild: missing, extra, reordered and respelled \
                 arguments are refused alike",
            ),
        ),
        (
            "the selected fallback, declined and served cold",
            &boxed,
            "chain",
            1,
            ["namespace", "boxed"],
            untouched(),
            Ok(fallback),
        ),
        (
            "the selected fallback, the OFF dropped",
            &boxed,
            "chain",
            1,
            ["namespace", "boxed"],
            Box::new(|input| input["native_controls"]["argv"] = json!([])),
            refused(
                "codex",
                "carries no measured OFF for native capability 'web-search', which its plan \
                 denies",
            ),
        ),
        (
            "claude, rejoined",
            &unboxed,
            "work",
            0,
            ["harness", "none"],
            untouched(),
            rejoined_claude(&[], &[]),
        ),
        (
            "boxed claude, rejoined",
            &boxed,
            "chain",
            0,
            ["namespace", "boxed"],
            untouched(),
            rejoined_claude(&mode, &claude_hands),
        ),
        (
            "claude, the OFF dropped",
            &unboxed,
            "work",
            0,
            ["harness", "none"],
            Box::new(|input| input["native_controls"]["selection"]["deny"] = json!([])),
            refused(
                "claude",
                "leaves tool 'WebFetch' available, which its plan denies as native capability \
                 'web-fetch'",
            ),
        ),
        (
            "claude, its record missing beside its serving inputs",
            &unboxed,
            "work",
            0,
            ["harness", "none"],
            Box::new(|input| {
                input.as_object_mut().unwrap().remove("launch_record");
            }),
            Err(
                "refusing to invoke the agent CLI: the input carries a sealed launch record or \
                 sealed serving inputs without the capability plan and the record they are \
                 sealed beside, so its final command cannot be checked; a sealed launch is never \
                 served unchecked (rebuild unit 14; design D6)"
                    .to_string(),
            ),
        ),
        (
            "codex agent, its record emptied",
            &denied,
            "agent",
            0,
            ["harness", "none"],
            segments(|segments| segments.clear()),
            unassembled(0, 0, 6),
        ),
        (
            "codex agent, its record reversed",
            &denied,
            "agent",
            0,
            ["harness", "none"],
            segments(|segments| segments.reverse()),
            unassembled(0, 6, 6),
        ),
        (
            "codex agent, its record grown by a hands token",
            &denied,
            "agent",
            0,
            ["harness", "none"],
            segments(grown),
            unassembled(6, 7, 6),
        ),
        (
            "the selected fallback, its record grown by a hands token",
            &boxed,
            "chain",
            1,
            ["namespace", "boxed"],
            segments(grown),
            unassembled(12, 13, 12),
        ),
        (
            "boxed claude, its record reversed",
            &boxed,
            "chain",
            0,
            ["namespace", "boxed"],
            segments(|segments| segments.reverse()),
            unassembled(0, 13, 13),
        ),
    ];
    assert_eq!(rows.len(), 19);
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(
            |(label, bundle, site, candidate, markers, tamper, expected)| {
                let provider =
                    &bundle.sites[site].capabilities.as_ref().unwrap().outcomes[candidate].provider;
                let (bin, offered, assessment) = match provider.as_str() {
                    "codex" => (&codex, THREAD, compiled(bundle, site, candidate)),
                    _ => (&claude, session, enabled(markers[0], markers[1])),
                };
                let observed = sealed_rejoin(
                    bundle, site, candidate, bin, offered, markers, assessment, tamper,
                );
                (observed != expected)
                    .then(|| format!("row {label}:\n  left:  {observed:?}\n  right: {expected:?}"))
            },
        )
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
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
/// Since rebuild unit 14 the Claude driver runs the same check at its cold
/// seam, so under `open` it is the driver that refuses.
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
            "refusing to invoke the agent CLI: the final command of harness 'claude' is sealed \
             with hands that are not the engine's workspace hands; a complete command is \
             parsed back before its spawn and must express exactly the capability state its \
             sealed plan records, so it is refused rather than spawned (operator ruling 2 of \
             2026-09-23; design D6)"
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

// ------------------------------------------------- rebuild unit 20

/// The authored command of the inline site `tag` inside `body`, labelled
/// as the engine labels its sites: a single seat's (no tag), a panel
/// member's, a sequence step's or a member of a panel step, and a select
/// case's or the default's.
fn body_command(body: &SeatBody, tag: &str) -> Option<Vec<String>> {
    use brokkr_runtime::bundle::StepBody;
    let (name, rest) = tag.split_once(':').unwrap_or((tag, ""));
    match body {
        SeatBody::Single { command, .. } if tag.is_empty() => Some(command.clone()),
        SeatBody::Panel { members, .. } => members
            .iter()
            .find(|member| member.name == tag)
            .map(|member| member.command.clone()),
        SeatBody::Sequence { steps } => match &steps.iter().find(|step| step.name == name)?.body {
            StepBody::Single { command, .. } if rest.is_empty() => Some(command.clone()),
            StepBody::Panel { members, .. } => members
                .iter()
                .find(|member| member.name == rest)
                .map(|member| member.command.clone()),
            _ => None,
        },
        SeatBody::Select { cases, default, .. } => match name {
            "default" => body_command(default.as_deref()?, rest),
            case => body_command(cases.get(case)?, rest),
        },
        _ => None,
    }
}

/// The resume assessment the bundle COMPILED for one site and candidate —
/// the selected link's (the shipped adapter's), or the inline site's —
/// never a test's.
fn assessment(bundle: &Bundle, label: &str, candidate: usize) -> Value {
    let facts = &bundle.sites[label];
    match facts.chain.get(candidate) {
        Some(link) => link.resume.value(),
        None => facts.inline_resume.clone().unwrap_or(Value::Null),
    }
}

/// Rebuild unit 20: one compiled site and candidate of `class`, composed,
/// sealed and verified as dispatch does it ([`dispatched`]), and handed to
/// its provider's own command builder, offered a session where `offer`
/// names the harness binary and the session, under the resume assessment
/// the bundle compiled for the site. The final command the driver would
/// spawn, or its whole refusal.
fn served_as(
    bundle: &Bundle,
    label: &str,
    candidate: usize,
    class: brokkr_runtime::SeatClass,
    offer: Option<(&Path, &str)>,
) -> Result<Vec<String>, String> {
    let outcome = &bundle.sites[label].capabilities.as_ref().unwrap().outcomes[candidate];
    let (extra, mut input) = dispatched(bundle, label, candidate, class, "/w")?;
    let (bin, session) = match offer {
        Some((bin, session)) => {
            input["resume_context"] = json!({"assessment": assessment(bundle, label, candidate)});
            (bin.to_str().unwrap().to_string(), Some(session))
        }
        None => (outcome.provider.clone(), None),
    };
    match outcome.provider.as_str() {
        "codex" => brokkr_protocol::adapters::codex_command(&bin, &extra, "/w", session, &input),
        "claude" => brokkr_protocol::adapters::claude_command(&bin, &extra, session, &input),
        other => panic!("{label}[{candidate}] is served by {other}, which is not read here"),
    }
}

/// [`served_as`]'s dispatch half: the site's spawn composed at `class` in
/// `workdir` and sealed as dispatch does it ([`sealing_at`]), its record
/// verified at the door, and what the driver is handed — the extras after
/// the driver's `--` and the input, with the record, the serving inputs,
/// the plan, the result path and the door dispatch selects for the class,
/// the confinement markers `mark_hands` writes for the site, and the
/// provenance of the arguments.
fn dispatched(
    bundle: &Bundle,
    label: &str,
    candidate: usize,
    class: brokkr_runtime::SeatClass,
    workdir: &str,
) -> Result<(Vec<String>, Value), String> {
    use brokkr_protocol::native_controls::SERVING_INPUTS;
    use brokkr_runtime::bundle::HandsState;
    use brokkr_runtime::engine::{result_door, verify_record, LAUNCH_RECORD};
    let facts = &bundle.sites[label];
    let outcome = &facts.capabilities.as_ref().unwrap().outcomes[candidate];
    let link = facts.chain.get(candidate);
    let (mut spawn, sealing) = sealing_at(bundle, label, candidate, facts, class, workdir, |_| {});
    sealing?;
    let serving = seal_serving(bundle, &mut spawn, facts, link);
    let gate = class == brokkr_runtime::SeatClass::Gate;
    let door = result_door(bundle.boundary, gate, Some(facts), link).word();
    let mut input = json!({LAUNCH_RECORD: spawn.launch_record(), SERVING_INPUTS: serving,
                           "seat": label, "native_controls": outcome.controls(),
                           "result_path": format!("{workdir}/result.json")});
    if door == "last-message" {
        input["result_delivery"] = json!(door);
    }
    verify_record(&spawn, &input)?;
    let (boundary, hands) = match facts.hands {
        HandsState::Hands(_) => (
            bundle.boundary.word(),
            match bundle.boundary.is_boxed() {
                true => "boxed",
                false => "none",
            },
        ),
        HandsState::NoHands => ("not applicable", "none"),
        HandsState::Unknown => panic!("{label} is resolved"),
    };
    input["boundary"] = json!(boundary);
    input["hands"] = json!(hands);
    input["workdir"] = json!(workdir);
    input["launch_arguments"] = spawn.launch_arguments();
    let extra = spawn.argv[spawn.argv.iter().position(|part| part == "--").unwrap() + 1..].to_vec();
    Ok((extra, input))
}

/// The variable that turns [`driver_serving_child`] into the built-in
/// driver it names, carrying `{"kind": …, "extra": […]}` as JSON.
const SERVE_DRIVER: &str = "BROKKR_TEST_SERVE_DRIVER";

/// Not a test of its own: the driver half of [`serve_driver`], run only
/// when that helper re-enters this binary with [`SERVE_DRIVER`] set. It
/// serves the driver protocol on this process's stdin and stdout, as
/// `brokkr driver <kind> -- <extras>` does, then exits before the test
/// harness can report on it.
#[test]
fn driver_serving_child() {
    use brokkr_protocol::adapters::AdapterKind;
    let Ok(served) = std::env::var(SERVE_DRIVER) else {
        return;
    };
    let served: Value = serde_json::from_str(&served).unwrap();
    let kind = match served["kind"].as_str().unwrap() {
        "lanetally" => AdapterKind::Lanetally,
        "dsh" => AdapterKind::Dsh,
        other => panic!("no serving child for {other}"),
    };
    let extra: Vec<String> = serde_json::from_value(served["extra"].clone()).unwrap();
    brokkr_protocol::adapters::serve(kind, extra).unwrap();
    std::process::exit(0);
}

/// The built-in driver `kind` — this binary re-entered as
/// [`driver_serving_child`], because a driver's serving branch is reached
/// only through its own stdin protocol — handed `extra` after its `--`,
/// with `env` set, then sent the protocol's hello, a resume offer of
/// `session` where one is made, the start carrying `input`, and shutdown.
/// What the driver said on its stdout.
#[cfg(unix)]
fn serve_driver(
    kind: &str,
    extra: &[String],
    env: &[(&str, &Path)],
    session: Option<&str>,
    input: &Value,
) -> String {
    use std::io::Write;
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "driver_serving_child",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(
            SERVE_DRIVER,
            json!({"kind": kind, "extra": extra}).to_string(),
        )
        .env_remove("FORGE_LANETALLY_BIN")
        .env_remove("FORGE_DSH_BIN")
        .env("PATH", "/usr/bin:/bin");
    for (name, value) in env {
        command.env(name, value);
    }
    let mut child = command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let offered = session.map(|session| {
        json!({"proto": "forge-driver/v1", "msg_id": "m2", "type": "resume",
               "effect_id": "fx", "attempt_id": "a1", "session_ref": session})
    });
    let messages = [
        Some(
            json!({"proto": "forge-driver/v1", "msg_id": "m1", "type": "hello",
                    "engine_version": "test"}),
        ),
        offered,
        Some(
            json!({"proto": "forge-driver/v1", "msg_id": "m3", "type": "start",
                    "effect_id": "fx", "attempt_id": "a1", "seat": input["seat"],
                    "input": input}),
        ),
        Some(json!({"proto": "forge-driver/v1", "msg_id": "m4", "type": "shutdown"})),
    ];
    for message in messages.into_iter().flatten() {
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The error of the one result message the driver said, whole.
fn result_error(said: &str) -> String {
    said.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|message| message["type"] == "result")
        .and_then(|result| result["error"].as_str().map(String::from))
        .unwrap_or_else(|| format!("no result error: {said}"))
}

/// A recording stand-in for the harness binary `name` under `root`: it
/// answers the version probe, writes every other argv to `<name>-argv`,
/// each argument NUL-terminated, and keeps a DSH launch's staged overlay
/// as it was when spawned in `<name>-overlay`. Staged beside its name and
/// renamed in, so the file is never open for writing when it is executed.
#[cfg(unix)]
fn recording_harness(root: &Path, name: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let staged = root.join(format!("{name}.staged"));
    std::fs::write(
        &staged,
        [
            "#!/bin/sh\n",
            "case \"$1\" in --version) printf '2.1.266 (Claude Code)\\n'; exit 0;; esac\n",
            "printf '%s\\0' \"$0\" \"$@\" > '",
            root.join(format!("{name}-argv")).to_str().unwrap(),
            "'\n",
            "if [ \"$1\" = --profile ]; then cat \"$4\" > '",
            root.join(format!("{name}-overlay")).to_str().unwrap(),
            "'; fi\n",
        ]
        .concat(),
    )
    .unwrap();
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).unwrap();
    let harness = root.join(format!("{name}-harness"));
    std::fs::rename(&staged, &harness).unwrap();
    harness
}

/// What [`recording_harness`] recorded at `path`, one argument per
/// NUL-terminated field: only the last terminator is dropped, so an empty
/// argument stays an argument. `None` where it was never spawned.
fn recorded_argv(path: &Path) -> Option<Vec<String>> {
    let bytes = std::fs::read(path).ok()?;
    let fields = bytes
        .strip_suffix(&[0])
        .unwrap_or_else(|| panic!("an unterminated record: {bytes:?}"));
    Some(
        fields
            .split(|byte| *byte == 0)
            .map(|field| String::from_utf8_lossy(field).into_owned())
            .collect(),
    )
}

/// Rebuild unit 20: one compiled LaneTally or DSH site and candidate served
/// by its REAL driver ([`serve_driver`]), handed the extras and input
/// [`dispatched`] seals, with the charter text the door reads, and offered
/// `session` where one is named, under the assessment the bundle compiled
/// for the site; the harness is a [`recording_harness`]. What the harness
/// was spawned with, or the driver's result error where it spawned
/// nothing; beside it, the input the driver was handed.
#[cfg(unix)]
fn driver_spawned(
    bundle: &Bundle,
    label: &str,
    candidate: usize,
    root: &Path,
    route: Option<&str>,
    session: Option<&str>,
) -> (Result<Vec<String>, String>, Value) {
    let provider = &bundle.sites[label].capabilities.as_ref().unwrap().outcomes[candidate].provider;
    // A seat's `--patch` route overlay resolves against the run's working
    // directory, which is here the compiled layer that holds it.
    let workdir = match route {
        Some(_) => bundle.dir.clone(),
        None => root.join("work"),
    };
    std::fs::create_dir_all(&workdir).unwrap();
    let (extra, mut input) = dispatched(
        bundle,
        label,
        candidate,
        brokkr_runtime::SeatClass::Work,
        workdir.to_str().unwrap(),
    )
    .unwrap_or_else(|refusal| panic!("{label}[{candidate}] unsealed: {refusal}"));
    let charter = bundle.sites[label]
        .charter
        .as_ref()
        .expect("a chartered site");
    input["feature"] = json!("serving");
    input["phase"] = json!(label);
    input["role_path"] = json!(charter.path);
    input["role_text"] = json!(std::fs::read_to_string(&charter.path).unwrap());
    input["allowed_results"] = json!(["complete"]);
    input["context"] = json!({});
    // The start context dispatch writes: the compiled assessment, and for
    // a `--patch` that resolves inside the compiled layer the binding
    // `route_overlay_binding` computes — the value as authored and the
    // digest the compiled manifest records for that file.
    if session.is_some() || route.is_some() {
        input["resume_context"] = json!({"assessment": assessment(bundle, label, candidate)});
    }
    if let Some(value) = route {
        input["resume_context"]["route_overlay"] =
            json!({"value": value, "digest": bundle.manifest["files"][value]});
    }
    let harness = recording_harness(root, provider);
    let recorded = root.join(format!("{provider}-argv"));
    let _ = std::fs::remove_file(&recorded);
    let home = root.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let variable = match provider.as_str() {
        "lanetally" => "BROKKR_LANETALLY_BIN",
        _ => "BROKKR_DSH_BIN",
    };
    let said = serve_driver(
        provider,
        &extra,
        &[(variable, &harness), ("HOME", &home), ("DSH_HOME", &home)],
        session,
        &input,
    );
    (
        recorded_argv(&recorded).ok_or_else(|| result_error(&said)),
        input,
    )
}

/// The carriers of rebuild unit 20's matrix: what a site is seated with —
/// an inline command of one harness, or an agent office — and whether it
/// may hold a gate (decision 0021 ruling 2 keeps LaneTally and DSH off
/// one).
const CARRIERS: [(&str, bool); 9] = [
    ("claude", true),
    ("codex", true),
    ("lanetally", false),
    ("dsh", false),
    ("pair", true),
    ("pair-claude", true),
    ("pair-tally", false),
    ("pair-flash", false),
    ("boxed", true),
];

/// The site labels one carrier is seated at in [`every_shape`], each with
/// its class and whether the recipe inherits it from `base`. The inline
/// Claude gate is the protected `review` every policy keeps.
fn carrier_sites(carrier: &str, gate: bool) -> Vec<(String, brokkr_runtime::SeatClass, bool)> {
    use brokkr_runtime::SeatClass::{Gate, Work};
    let mut sites = vec![(carrier.to_string(), Work, false)];
    match (gate, carrier) {
        (true, "claude") => sites.push(("review".to_string(), Gate, false)),
        (true, _) => sites.push((format!("{carrier}-gate"), Gate, false)),
        (false, _) => {}
    }
    for tag in [
        "panel:member",
        "panel:peer",
        "steps:step",
        "steps:next",
        "pick:engine",
        "pick:default",
    ] {
        sites.push((format!("{carrier}-{tag}"), Work, false));
    }
    sites.push((format!("{carrier}-base"), Work, true));
    sites
}

/// Rebuild unit 20's recipe, `matrix`, extending `base`: every executable
/// site shape of every harness, compiled on the shipped adapters under
/// `harness` in a realm that grants nothing. Each of the [`CARRIERS`] is
/// seated at a single work seat, at a gate where it may hold one, at a
/// panel member, a sequence step, a select case and the default, and at a
/// work seat `base` declares ([`carrier_sites`]). Beside them, two single
/// seats with a typed or routed declaration: `claude-typed` (a typed
/// `tools.allow`) and `dsh-route` (a `--patch` route overlay of this
/// layer). The single inline Codex work seat and gate declare their typed
/// sandbox classes; a nested inline site declares none (D5.3).
///
/// Each inline site's charter is its own file of its layer, `# <label>\n`,
/// and each office's is `# <office>\n`, so a site bound to a neighbour's
/// charter fails by name. The offices: `pair` (Codex, falling back to
/// Claude), `pair-claude` (the reverse), `pair-tally` (LaneTally, falling
/// back to DSH), `pair-flash` (the reverse), and `boxed` (Codex alone,
/// with workspace hands, which `harness` serves with the harness's own
/// sandbox).
fn every_shape(operator: &Operator) -> Bundle {
    compile_every_shape(operator, None)
        .unwrap_or_else(|refusal| panic!("the matrix compiles: {refusal}"))
}

/// [`every_shape`], with a typed LaneTally allow seated beside it as the
/// work seat `lanetally-typed`: inline (`Some("inline")`), or through the
/// office `tally-typed` (`Some("agent")`), a LaneTally agent whose own
/// `tools.allow` is `[cargo]`. The compile's own words either way.
fn compile_every_shape(operator: &Operator, tally: Option<&str>) -> Result<Bundle, String> {
    let context = CapabilityContext::no_grants("private", operator.root());
    compile_every_shape_on(
        operator,
        tally,
        &workspace().join("adapters"),
        &context,
        None,
    )
}

/// [`compile_every_shape`] against a stated adapters root and realm
/// context, with `asks` written on every inline site and every office
/// where it is given (rebuild unit 21-fix-b).
fn compile_every_shape_on(
    operator: &Operator,
    tally: Option<&str>,
    adapters: &Path,
    context: &CapabilityContext,
    asks: Option<&Value>,
) -> Result<Bundle, String> {
    let root = operator.root();
    let command = |harness: &str| -> Value {
        let model = match harness {
            "codex" => "gpt-6-astra",
            "dsh" => "deepseek-v4-flash",
            _ => "claude-opus-5-5",
        };
        json!(["{brokkr}", "driver", harness, "--", "--model", model, "--effort", "high"])
    };
    for (office, models) in [
        ("pair", ["astra", "opus"].as_slice()),
        ("pair-claude", &["opus", "astra"]),
        ("pair-tally", &["opus-tallied", "flash"]),
        ("pair-flash", &["flash", "opus-tallied"]),
        ("boxed", &["astra"]),
    ] {
        std::fs::write(
            root.join(format!("agents/charters/{office}.md")),
            format!("# {office}\n"),
        )
        .unwrap();
        let efforts: serde_json::Map<String, Value> = models
            .iter()
            .map(|model| (model.to_string(), json!("high")))
            .collect();
        let mut agent = json!({"description": "an office",
            "charter": format!("charters/{office}.md"), "models": models, "efforts": efforts});
        if office == "boxed" {
            agent["hands"] = json!({"kind": "workspace", "network": false, "binds": []});
        }
        if let Some(asks) = asks {
            agent["capabilities"] = asks.clone();
        }
        write(root, &format!("agents/{office}.json"), &agent);
    }
    // One carrier seated at one site: its own charter file for an inline
    // command, its office otherwise.
    let seated = |layer: &str, carrier: &str, label: &str| -> Value {
        match CARRIERS.iter().position(|(name, _)| *name == carrier) {
            Some(at) if at < 4 => {
                let role = format!("roles/{}.md", label.replace(':', "-"));
                std::fs::create_dir_all(root.join(layer).join("roles")).unwrap();
                std::fs::write(root.join(layer).join(&role), format!("# {label}\n")).unwrap();
                let mut site = json!({"role": role, "driver": {"command": command(carrier)}});
                if let Some(asks) = asks {
                    site["capabilities"] = asks.clone();
                }
                site
            }
            _ => json!({"agent": carrier}),
        }
    };
    let mut phases: Vec<(String, &str)> = Vec::new();
    let (mut base, mut matrix) = (serde_json::Map::new(), serde_json::Map::new());
    for (carrier, gate) in CARRIERS {
        for (label, class, inherited) in carrier_sites(carrier, gate) {
            let layer = if inherited { "base" } else { "matrix" };
            let site = seated(layer, carrier, &label);
            let (seat, tag) = label.split_once(':').unwrap_or((&label, ""));
            let (results, body) = match (tag, class) {
                ("", brokkr_runtime::SeatClass::Gate) => {
                    let mut body = site;
                    body["class"] = json!("gate");
                    (json!(["clean"]), body)
                }
                ("", _) => (json!(["complete"]), site),
                ("member", _) => (
                    json!(["pass", "fail"]),
                    json!({"aggregate": "unanimous-pass", "panel": {"member": site,
                           "peer": seated(layer, carrier, &format!("{seat}:peer"))}}),
                ),
                ("step", _) => {
                    let mut step = site;
                    step["name"] = json!("step");
                    step["results"] = json!(["complete"]);
                    let mut next = seated(layer, carrier, &format!("{seat}:next"));
                    next["name"] = json!("next");
                    (json!(["complete"]), json!({"sequence": [step, next]}))
                }
                ("engine", _) => (
                    json!(["complete"]),
                    json!({"select": {"on": "strategy", "cases": {"engine": site},
                          "default": seated(layer, carrier, &format!("{seat}:default"))}}),
                ),
                // The peer is seated beside its member, the next step
                // behind its first, the default beside its case.
                _ => continue,
            };
            let mut body = body;
            body["results"] = results.clone();
            if (carrier, tag, inherited) == ("codex", "", false) {
                body["tools"] = json!({"sandbox": match class {
                    brokkr_runtime::SeatClass::Gate => "read-only",
                    brokkr_runtime::SeatClass::Work => "workspace-write",
                }});
            }
            for result in ["complete", "clean", "pass", "fail"] {
                if results.as_array().unwrap().contains(&json!(result)) {
                    phases.push((seat.to_string(), result));
                }
            }
            match inherited {
                true => base.insert(seat.to_string(), body),
                false => matrix.insert(seat.to_string(), body),
            };
        }
    }
    let mut typed = seated("matrix", "claude", "claude-typed");
    typed["results"] = json!(["complete"]);
    typed["tools"] = json!({"allow": ["cargo"]});
    matrix.insert("claude-typed".to_string(), typed);
    phases.push(("claude-typed".to_string(), "complete"));
    if let Some(form) = tally {
        let mut body = match form {
            "inline" => {
                let mut body = seated("matrix", "lanetally", "lanetally-typed");
                body["tools"] = json!({"allow": ["cargo"]});
                body
            }
            _ => {
                std::fs::write(
                    root.join("agents/charters/tally-typed.md"),
                    "# tally-typed\n",
                )
                .unwrap();
                write(
                    root,
                    "agents/tally-typed.json",
                    &json!({"description": "an office", "charter": "charters/tally-typed.md",
                            "models": ["opus-tallied"], "efforts": {"opus-tallied": "high"},
                            "tools": {"allow": ["cargo"]}}),
                );
                json!({"agent": "tally-typed"})
            }
        };
        body["results"] = json!(["complete"]);
        matrix.insert("lanetally-typed".to_string(), body);
        phases.push(("lanetally-typed".to_string(), "complete"));
    }
    // The shipped route-only overlay, as a file of this layer.
    std::fs::create_dir_all(root.join("matrix/drivers")).unwrap();
    std::fs::copy(
        workspace().join("recipes/research-dsh/drivers/research-web.yml"),
        root.join("matrix/drivers/route.yml"),
    )
    .unwrap();
    let mut routed = seated("matrix", "dsh", "dsh-route");
    routed["results"] = json!(["complete"]);
    routed["driver"]["command"] = json!([
        "{brokkr}",
        "driver",
        "dsh",
        "--",
        "--model",
        "dashscope/qwen3.8-max",
        "--effort",
        "xhigh",
        "--patch",
        "drivers/route.yml"
    ]);
    matrix.insert("dsh-route".to_string(), routed);
    phases.push(("dsh-route".to_string(), "complete"));

    // One phase per seat, in order, every result stepping to the next.
    let mut names: Vec<String> = Vec::new();
    for (seat, _) in &phases {
        if !names.contains(seat) {
            names.push(seat.clone());
        }
    }
    let rules: Vec<Value> = phases
        .iter()
        .enumerate()
        .map(|(at, (seat, result))| {
            let next = names
                .iter()
                .position(|name| name == seat)
                .and_then(|at| names.get(at + 1))
                .map_or("done", String::as_str);
            json!({"id": format!("R{at}"), "from": seat, "result": result, "next": next,
                   "reason": "r"})
        })
        .collect();
    names.push("done".to_string());
    write(
        root,
        "base/policy.json",
        &json!({"phases": names, "initial": "claude", "terminal": ["done"], "rules": rules}),
    );
    write(
        root,
        "base/bundle.json",
        &json!({"name": "base", "policy": "policy.json", "seats": base}),
    );
    write(
        root,
        "matrix/bundle.json",
        &json!({"name": "matrix", "extends": "base", "seats": matrix}),
    );
    Bundle::compile_with_capabilities(
        &root.join("matrix"),
        &root.join("agents"),
        adapters,
        Some("private"),
        None,
        Boundary::Harness,
        context,
    )
    .map_err(|refusal| refusal.to_string())
}

/// The whole prompt the DSH driver renders for a site of [`every_shape`]
/// on the shipped adapter in a realm that grants nothing, written out: the
/// charter the door read (`# <charter>\n`), then the task, the result
/// contract and what the seat is told of its hands and capabilities. Only
/// the fixture's variable values are substituted.
fn dsh_prompt(charter: &str, phase: &str, workdir: &str) -> String {
    format!("# {charter}\n\n\n---\n## Task\n\nFeature: serving\nPhase: {phase} (you are this phase's only seat)\nWorking directory: {workdir}\n\nRun context (journal-derived, read-only):\n```json\n{{}}\n```\n\n## Result contract — MANDATORY\n\nWhen your work is finished, write a JSON object to exactly this file:\n\n    {workdir}/result.json\n\nwith the shape:\n\n    {{\"result\": \"<one of: complete>\",\n      \"inputs\": {{ ...optional typed facts for the phase machine... }},\n      \"notes\": \"<short human summary of what you did and why>\"}}\n\nThe file is the ONLY channel the engine reads. Printing the JSON instead of writing the file counts as producing no result. The object carries exactly these top-level keys — result, inputs, notes — and nothing else: a typed fact goes INSIDE inputs, and a record with any other top-level key is refused where it is sealed (decision 0034), which loses the whole attempt. You never decide the next phase — the engine's policy table rules on your typed result.\n")
}

/// Rebuild unit 20 (tasks 20.1 and 21.3): EVERY COMPILED SITE SHAPE OF
/// EVERY HARNESS IS SERVED ITS WHOLE COMMAND BESIDE ITS SELECTED CHARTER.
/// One production compile ([`every_shape`]) seats Claude, Codex,
/// LaneTally and DSH, inline and agent-backed, at every executable site
/// shape — a single work seat, a gate where the harness may hold one, a
/// panel member, a sequence step, a select case and default, and a seat
/// the recipe inherits — each agent's primary and its selected fallback,
/// and a Codex office with hands under `harness`, in a realm that grants
/// nothing. The rows are every compiled site and candidate of that
/// bundle, no more and no fewer.
///
/// For each row this asserts, as independent literals: the charter the
/// compile selected for the site (the declaring layer and the key its walk
/// pins, or the library and the agent; the reference as written, the path
/// the seat is told and the digest of the text written); the whole cold
/// command; and, at a work site, the whole command served when the site is
/// offered a session under the assessment the bundle compiled. Claude and
/// Codex are composed by their drivers' own command builders; LaneTally
/// and DSH are spawned by their REAL drivers against a recording harness,
/// whose argv is read whole, empty arguments included, and a DSH prompt is
/// the literal [`dsh_prompt`]. Only a staged DSH overlay's temporary path
/// is named by shape, and its bytes are read: the layer's route where one
/// is authored, and none of it elsewhere.
///
/// Only two shipped rejoins are supported: a Codex work site under its
/// typed `workspace-write` class and a Codex site with hands under
/// `harness`, each an actual `exec resume`. Every other offer — a Codex
/// link without hands (`sandbox-unavailable`), and every Claude, LaneTally
/// and DSH shape, whose shipped resume is unmeasured — is declined and
/// served its cold command. A gate is offered no session.
///
/// The typed LaneTally allow is the one full refusal, and it is the
/// compile's (operator ruling of 2026-09-29, R5; D5.3): seated inline or
/// through a LaneTally office, it meets LaneTally's unmeasured native
/// plan, and the same matrix refuses whole, naming the seat, its office,
/// the harness and the cause, then the adapter's own reason. No bundle
/// exists, so nothing is spawned cold. A pinned `brokkr resume` of such a
/// run is NOT proved here: it recompiles through this compiler, but the
/// CLI renders the refusal through its own manifest-mismatch door, so its
/// line differs and is owed to a brokkr-cli suite (review return R1 of
/// unit 20-fix). Composition evidence only: what a provider honours is the
/// controller's to measure. The restriction rows are unit 21's, below:
/// the managed Read/empty rows in
/// [`a_managed_read_or_empty_limit_is_served_whole_cold_and_on_an_actual_eligible_resume`]
/// and
/// [`a_managed_read_limit_keeps_prompt_values_authored_lists_and_lanetallys_inventory_apart`],
/// and CQ1's in
/// [`a_restricted_grant_reaches_only_cq1s_outcomes_cold_and_on_an_actual_eligible_resume`];
/// this matrix's own site shapes carry them in
/// [`a_managed_read_or_empty_limit_reaches_every_compiled_claude_site_shape`]
/// and
/// [`a_restricted_grant_reaches_only_cq1s_outcomes_at_every_compiled_codex_site_shape`],
/// and boxed under `namespace` in
/// [`a_restricted_grant_reaches_only_cq1s_outcomes_at_every_boxed_codex_site_shape`].
#[cfg(unix)]
#[test]
fn every_compiled_site_shape_of_every_harness_is_served_its_whole_command_beside_its_charter() {
    use brokkr_runtime::bundle::CharterOwner;
    use brokkr_runtime::SeatClass::{Gate, Work};
    let operator = Operator::new();
    // The line is the compiler's one bounded refusal (512 scalar values).
    // The cause is whole before the adapter's own reason, and only that
    // reason's tail is cut, where the office's label leaves it: its first
    // sentence is whole in both forms.
    let forms = [
        ("inline", "lanetally-typed", "Cla…"),
        ("agent", "tally-typed", "Claude'…"),
    ];
    assert_eq!(
        forms.map(|(form, ..)| {
            let compiled = compile_every_shape(&operator, Some(form));
            (form, compiled.map(|bundle| bundle.sites.len()))
        }),
        forms.map(|(form, office, cut)| (
            form,
            Err(format!(
                "bundle: bundle: seat 'lanetally-typed' (office '{office}') in realm 'private': \
                 its typed 'tools.allow' refuses at compile, as harness 'lanetally' of provider \
                 'lanetally' has native controls its adapter declares unmeasured (ruling R5 of \
                 2026-09-29; design D5.3): the LaneTally wrapper forwards argv to claude, and \
                 forwarding is not confinement: whether Claude Code's native WebSearch and \
                 WebFetch controls hold through the wrapper, which owns its own per-session \
                 settings layer, has not been verified. {cut}"
            ))
        ))
    );
    let bundle = every_shape(&operator);
    let codex = codex_reporting(operator.root(), "0.154.0");
    let claude = claude_reporting(operator.root(), "2.1.266");
    let session = "019c4b7e-0000-7000-8000-000000000002";
    let root = operator.root();
    let library = root.join("agents");
    let work = root.join("work");
    let work = work.to_str().unwrap();

    type Charter = (CharterOwner, String, PathBuf, String);
    // The charter each site is told: its owner, the reference as written,
    // the path and the digest of the text this test wrote.
    let layer = |dir: &str, label: &str| -> Charter {
        let dir = root.join(dir);
        let reference = format!("roles/{}.md", label.replace(':', "-"));
        (
            CharterOwner::Layer {
                dir: dir.clone(),
                key: reference.clone(),
            },
            reference.clone(),
            dir.join(&reference),
            brokkr_core::canonical::sha256_bytes(format!("# {label}\n").as_bytes()),
        )
    };
    let office = |agent: &str| -> Charter {
        let reference = format!("charters/{agent}.md");
        (
            CharterOwner::Library {
                agent: agent.to_string(),
                root: library.clone(),
            },
            reference.clone(),
            library.join(&reference),
            brokkr_core::canonical::sha256_bytes(format!("# {agent}\n").as_bytes()),
        )
    };

    // The whole commands, written out: the program, the driver's lead, the
    // pins the recipe or the adapter wrote, the engine's own segments, the
    // denial.
    let words = |program: &str, parts: &[&[&str]]| -> Vec<String> {
        std::iter::once(program)
            .chain(parts.iter().flat_map(|part| part.iter().copied()))
            .map(String::from)
            .collect()
    };
    let lead = ["-p", "--output-format", "stream-json", "--verbose"];
    let claude_pins = ["--model", "claude-opus-5-5", "--effort", "high"];
    let denial = ["--disallowedTools", "WebFetch,WebSearch"];
    let template = ["--permission-mode", "acceptEdits"];
    let lowered = ["--allowedTools", "Bash(cargo:*)"];
    let codex_lead = ["exec", "--json", "-C", "/w"];
    let codex_pins = [
        "-c",
        "model_reasoning_effort=\"high\"",
        "--model",
        "gpt-6-astra",
    ];
    let work_class = ["--sandbox", "workspace-write"];
    let gate_class = [
        "--sandbox",
        "read-only",
        "--output-last-message",
        "/w/result.json",
    ];
    let resumed = ["exec", "resume", "--json"];
    let rejoined_class = ["-c", "sandbox_mode=\"workspace-write\""];
    let at = [THREAD, "-"];
    let lanetally_bin = root.join("lanetally-harness");
    let dsh_bin = root.join("dsh-harness");
    let (lanetally_bin, dsh_bin) = (lanetally_bin.to_str().unwrap(), dsh_bin.to_str().unwrap());
    let (claude_shim, codex_shim) = (claude.to_str().unwrap(), codex.to_str().unwrap());

    // The cold command of a site seated with `carrier`, served by
    // `provider`, at `class`, on `program`.
    let cold = |carrier: &str, label: &str, provider: &str, program: &str, charter: &str| {
        let inline = CARRIERS[..4].iter().any(|(name, _)| *name == carrier);
        let gate = label.ends_with("-gate");
        match (provider, inline) {
            ("claude", true) if label == "claude-typed" => words(
                program,
                &[&lead, &claude_pins, &template, &lowered, &denial],
            ),
            ("claude", true) => words(program, &[&lead, &claude_pins, &denial]),
            ("claude", false) => words(program, &[&lead, &template, &claude_pins, &denial]),
            ("codex", _) if label == "codex" => {
                words(program, &[&codex_lead, &codex_pins, &work_class, &OFF])
            }
            ("codex", _) if label == "codex-gate" || (carrier == "boxed" && gate) => {
                words(program, &[&codex_lead, &codex_pins, &gate_class, &OFF])
            }
            ("codex", _) if carrier == "boxed" => {
                words(program, &[&codex_lead, &codex_pins, &work_class, &OFF])
            }
            ("codex", _) => words(program, &[&codex_lead, &codex_pins, &OFF]),
            ("lanetally", true) => words(lanetally_bin, &[&lead, &claude_pins]),
            ("lanetally", false) => words(lanetally_bin, &[&lead, &template, &claude_pins]),
            ("dsh", _) => {
                let workdir = match label {
                    "dsh-route" => bundle.dir.to_str().unwrap(),
                    _ => work,
                };
                words(
                    dsh_bin,
                    &[&[
                        "--profile",
                        "headless",
                        "--patch",
                        "<overlay>",
                        &dsh_prompt(charter, label, workdir),
                    ]],
                )
            }
            other => panic!("no row for {other:?}"),
        }
    };

    // (site, candidate, class, charter, served cold, served when offered)
    type Row = (
        String,
        usize,
        brokkr_runtime::SeatClass,
        Charter,
        Result<Vec<String>, String>,
        Option<Result<Vec<String>, String>>,
    );
    let mut rows: Vec<Row> = Vec::new();
    let mut seated: Vec<(String, &str, brokkr_runtime::SeatClass, bool)> = Vec::new();
    for (carrier, gate) in CARRIERS {
        for (label, class, inherited) in carrier_sites(carrier, gate) {
            seated.push((label, carrier, class, inherited));
        }
    }
    seated.push(("claude-typed".into(), "claude", Work, false));
    seated.push(("dsh-route".into(), "dsh", Work, false));
    for (label, carrier, class, inherited) in &seated {
        let class = *class;
        let inline = CARRIERS[..4].iter().any(|(name, _)| name == carrier);
        let (charter, told) = match inline {
            true => (
                layer(if *inherited { "base" } else { "matrix" }, label),
                label.as_str(),
            ),
            false => (office(carrier), *carrier),
        };
        let providers: &[&str] = match *carrier {
            "pair" => &["codex", "claude"],
            "pair-claude" => &["claude", "codex"],
            "pair-tally" => &["lanetally", "dsh"],
            "pair-flash" => &["dsh", "lanetally"],
            "boxed" => &["codex"],
            harness => &[harness],
        };
        for (candidate, provider) in providers.iter().enumerate() {
            let cold_on = |program: &str| cold(carrier, label, provider, program, told);
            let served: Result<Vec<String>, String> = Ok(cold_on(match *provider {
                "claude" => "claude",
                _ => "codex",
            }));
            // A Codex work site under its typed class, or with hands under
            // `harness`, is rejoined; every other work site is declined and
            // served cold on the binary it was offered on.
            let offered = match (class, *provider) {
                (Gate, _) => None,
                (_, "codex") if label == "codex" || *carrier == "boxed" => Some(Ok(words(
                    codex_shim,
                    &[&resumed, &rejoined_class, &codex_pins, &OFF, &at],
                ))),
                (_, "codex") => Some(Ok(cold_on(codex_shim))),
                (_, "claude") => Some(Ok(cold_on(claude_shim))),
                _ => Some(served.clone()),
            };
            rows.push((
                label.clone(),
                candidate,
                class,
                charter.clone(),
                served,
                offered,
            ));
        }
    }
    assert_eq!(rows.len(), 113);
    // The rows are every compiled site and candidate, and nothing else.
    let compiled: std::collections::BTreeSet<(String, usize)> = bundle
        .sites
        .iter()
        .flat_map(|(label, facts)| {
            (0..facts.chain.len().max(1)).map(move |candidate| (label.clone(), candidate))
        })
        .collect();
    let written: std::collections::BTreeSet<(String, usize)> = rows
        .iter()
        .map(|(label, candidate, ..)| (label.clone(), *candidate))
        .collect();
    assert_eq!(written, compiled);

    let overlay = std::env::temp_dir().join("brokkr-dsh-seat-");
    let overlay = overlay.to_str().unwrap();
    let route = std::fs::read(root.join("matrix/drivers/route.yml")).unwrap();
    let failures: Vec<String> = rows
        .into_iter()
        .filter_map(|(label, candidate, class, charter, cold, offered)| {
            let label = label.as_str();
            let pin = bundle.sites[label].charter.as_ref().map(|pin| {
                (
                    pin.owner.clone(),
                    pin.reference.clone(),
                    pin.path.clone(),
                    pin.digest.clone(),
                )
            });
            let provider =
                &bundle.sites[label].capabilities.as_ref().unwrap().outcomes[candidate].provider;
            let routed = (label == "dsh-route").then_some("drivers/route.yml");
            // What a wrapper or DSH driver spawned: a staged DSH overlay's
            // temporary path by shape, and whether its bytes begin with
            // the layer's route.
            let spawned = |offer: Option<&str>| {
                let (spawned, _) = driver_spawned(&bundle, label, candidate, root, routed, offer);
                let staged = root.join("dsh-overlay");
                let overlaid = std::fs::read(&staged)
                    .ok()
                    .map(|bytes| bytes.starts_with(&route));
                let _ = std::fs::remove_file(&staged);
                let spawned = spawned.map(|argv| {
                    argv.into_iter()
                        .enumerate()
                        .map(|(at, part)| match at {
                            4 if provider == "dsh"
                                && part.starts_with(overlay)
                                && part.ends_with(".yml") =>
                            {
                                "<overlay>".into()
                            }
                            _ => part,
                        })
                        .collect()
                });
                (spawned, overlaid)
            };
            let observed = match provider.as_str() {
                "codex" | "claude" => {
                    let offer = match provider.as_str() {
                        "codex" => (codex.as_path(), THREAD),
                        _ => (claude.as_path(), session),
                    };
                    (
                        pin,
                        served_as(&bundle, label, candidate, class, None),
                        (class == Work)
                            .then(|| served_as(&bundle, label, candidate, class, Some(offer))),
                        None,
                    )
                }
                _ => {
                    let (cold, overlaid) = spawned(None);
                    let (offered, reoverlaid) = spawned(Some(session));
                    assert_eq!(overlaid, reoverlaid, "{label}[{candidate}]");
                    (pin, cold, Some(offered), overlaid)
                }
            };
            let overlaid = (provider == "dsh" && cold.is_ok()).then_some(label == "dsh-route");
            let expected = (Some(charter), cold, offered, overlaid);
            (observed != expected).then(|| {
                format!("row {label}[{candidate}]:\n  left:  {observed:?}\n  right: {expected:?}")
            })
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Rebuild unit 20-fix (operator ruling of 2026-09-29, R5): an unmeasured
/// plan hands the driver the provenance it was served — here two hands
/// arguments and one local limit, a shape the compiler itself never seals
/// (a lowered allow refuses there) — and the driver reads back exactly
/// that, never a default. A plan that types nothing writes no member, as
/// before.
#[test]
fn an_unmeasured_plan_hands_its_driver_the_provenance_it_was_served() {
    use brokkr_protocol::native_controls::{managed, Provenance};
    use brokkr_runtime::capabilities::NativePlan;
    let plan = |hands: usize, local: &[&str]| NativePlan::Unmeasured {
        declaration: None,
        reason: "never probed".to_string(),
        provenance: Provenance {
            hands,
            local: local.iter().map(|limit| limit.to_string()).collect(),
        },
    };
    let controls = plan(2, &["Bash(ls:*)"]).controls("dsh", "dsh");
    assert_eq!(
        controls,
        json!({"inventory": "unmeasured", "provider": "dsh", "harness": "dsh",
               "reason": "never probed", "hands": 2, "local": ["Bash(ls:*)"]})
    );
    let read = managed(&json!({ "native_controls": controls }))
        .unwrap()
        .unwrap();
    assert_eq!(
        read.provenance,
        Provenance {
            hands: 2,
            local: vec!["Bash(ls:*)".to_string()],
        }
    );
    assert_eq!(
        plan(0, &[]).controls("dsh", "dsh"),
        json!({"inventory": "unmeasured", "provider": "dsh", "harness": "dsh",
               "reason": "never probed"})
    );
}

/// Rebuild unit 20-fix, review return R3: the carry above made by the
/// COMPILER. A LaneTally office with workspace hands, compiled boxed on a
/// copy of the shipped adapters whose LaneTally file declares a workspace
/// fragment in place of its unsupported hands. (DSH's grammar places no
/// hands fragment at all.) LaneTally's unmeasured plan carries the box's
/// typed hands into the compose, which reads its count from the plan. A
/// fragment without its strict MCP configuration is refused by that count.
/// The whole transport is refused too, because the tool the hands admit
/// makes a final tool list that an unmeasured plan has no mapping to
/// write. So no compiled unmeasured site carries hands, and nothing is
/// spawned.
#[test]
fn a_compiled_unmeasured_site_carries_its_typed_hands_into_its_compose() {
    let operator = Operator::new();
    one_inline_seat(&operator, &["driver"]);
    write(
        operator.root(),
        "agents/handed-tally.json",
        &json!({"description": "an office with hands", "charter": "charters/searcher.md",
                "models": ["opus-tallied"], "efforts": {"opus-tallied": "high"},
                "hands": {"kind": "workspace", "network": false, "binds": []}}),
    );
    write(
        operator.root(),
        "solo/bundle.json",
        &json!({"name": "solo", "policy": "policy.json", "seats": {
            "work": {"results": ["complete"], "agent": "handed-tally"},
            "review": {"results": ["clean"], "role": "roles/role.md",
                       "driver": {"command": ["driver"]}}}}),
    );
    let compiled = |fragment: &[&str]| {
        let adapters = copied_adapters();
        edit_adapter(adapters.path(), "lanetally", |adapter| {
            adapter["hands"] = json!({ "workspace": fragment });
        });
        Bundle::compile_with_capabilities(
            &operator.root().join("solo"),
            &operator.root().join("agents"),
            adapters.path(),
            Some("private"),
            None,
            Boundary::Namespace,
            &CapabilityContext::no_grants("private", operator.root()),
        )
        .map(|bundle| bundle.sites.len())
        .map_err(|refusal| refusal.to_string())
    };
    let who = "bundle: seat 'work' (office 'handed-tally') in realm 'private'";
    assert_eq!(
        compiled(&["--mcp-config", "{hands_mcp_json}"]),
        Err(format!(
            "{who}: the capability plan for provider 'lanetally' types 2 arguments of the \
             engine's fragment as the box's hands, but they carry no strict MCP configuration \
             ('--strict-mcp-config'); the box's hands are delivered whole, so the launch is \
             refused rather than composed without them (decision 0043; design D6)"
        ))
    );
    assert_eq!(
        compiled(&["--strict-mcp-config", "--mcp-config", "{hands_mcp_json}"]),
        Err(format!(
            "{who}: the capability plan carries a final tool list with no selection mapping to \
             write it into, for provider 'lanetally', which its launch does not consume; a \
             control that cannot reach the final command is refused rather than recorded and \
             dropped (decision 0066 ruling 3)"
        ))
    );
}

/// Rebuild unit 20 (task 20.1; operator ruling 1 of 2026-09-23): AN
/// AUTHORED CAPABILITY-BEARING OPTION IS REFUSED AT EVERY SITE SHAPE OF
/// EVERY HARNESS. One recipe, `authored`, extending `authored-base`, seats
/// one harness inline at every site shape a recipe writes a command for: a
/// work seat, a gate, a panel member, a sequence step, a select case and
/// its default, and a seat inherited from the base. Each compile plants one
/// capability-bearing option at ONE site and the production compile
/// refuses it whole, naming that site, its office, the canonical option and
/// the harness, never the value; no driver or provider runs. With no site
/// poisoned the same recipe compiles, so each refusal is the planted
/// option's. An agent-backed site has no recipe-authored command: its
/// argv is the adapter's data, judged at load (unit 11).
#[test]
fn an_authored_capability_option_refuses_every_site_shape_of_every_harness() {
    let operator = Operator::new();
    let root = operator.root();
    let sites = [
        "work",
        "review",
        "judges:member",
        "steps:draft",
        "pick:engine",
        "pick:default",
        "inherited",
    ];
    for layer in ["authored", "authored-base"] {
        std::fs::create_dir_all(root.join(layer).join("roles")).unwrap();
        std::fs::write(root.join(layer).join("roles/role.md"), "# role\n").unwrap();
    }
    write(
        root,
        "authored-base/policy.json",
        &json!({"phases": ["work", "review", "judges", "steps", "pick", "inherited", "done"],
            "initial": "work", "terminal": ["done"], "rules": [
                {"id": "A", "from": "work", "result": "complete", "next": "review", "reason": "r"},
                {"id": "B", "from": "review", "result": "clean", "next": "judges", "reason": "r"},
                {"id": "C", "from": "judges", "result": "pass", "next": "steps", "reason": "r"},
                {"id": "D", "from": "judges", "result": "fail", "next": "steps", "reason": "r"},
                {"id": "E", "from": "steps", "result": "complete", "next": "pick", "reason": "r"},
                {"id": "F", "from": "pick", "result": "complete", "next": "inherited",
                 "reason": "r"},
                {"id": "G", "from": "inherited", "result": "complete", "next": "done",
                 "reason": "r"}]}),
    );
    let compile = |harness: &str, model: &str, poisoned: Option<(&str, &[&str])>| {
        let command = |site: &str| {
            // Neither DSH nor the LaneTally wrapper holds the trusted tier,
            // so no recipe seats either as a gate (decision 0021 ruling
            // 2): beside them the gate is Claude's.
            let (harness, model) = match (harness, site) {
                ("dsh" | "lanetally", "review") => ("claude", "claude-opus-5-5"),
                seated => (seated.0, model),
            };
            let mut command = vec!["{brokkr}", "driver", harness, "--", "--model", model];
            command.extend(["--effort", "high"]);
            if let Some((at, extra)) = poisoned.filter(|(at, _)| *at == site) {
                assert_eq!(at, site);
                command.extend(extra);
            }
            json!({"command": command})
        };
        let inline = |site: &str| json!({"role": "roles/role.md", "driver": command(site)});
        write(
            root,
            "authored-base/bundle.json",
            &json!({"name": "authored-base", "policy": "policy.json", "seats": {
                "inherited": {"results": ["complete"], "role": "roles/role.md",
                              "driver": command("inherited")}}}),
        );
        let mut draft = inline("steps:draft");
        draft["name"] = json!("draft");
        draft["results"] = json!(["complete"]);
        let mut last = inline("steps:last");
        last["name"] = json!("last");
        write(
            root,
            "authored/bundle.json",
            &json!({"name": "authored", "extends": "authored-base", "seats": {
                "work": {"results": ["complete"], "role": "roles/role.md",
                         "driver": command("work")},
                "review": {"results": ["clean"], "class": "gate", "role": "roles/role.md",
                           "driver": command("review")},
                "judges": {"results": ["pass", "fail"], "aggregate": "unanimous-pass",
                           "panel": {"member": inline("judges:member"),
                                     "peer": inline("judges:peer")}},
                "steps": {"results": ["complete"], "sequence": [draft, last]},
                "pick": {"results": ["complete"], "select": {"on": "strategy",
                    "cases": {"engine": inline("pick:engine")},
                    "default": inline("pick:default")}}}}),
        );
        Bundle::compile_with_capabilities(
            &root.join("authored"),
            &root.join("agents"),
            &workspace().join("adapters"),
            Some("private"),
            None,
            Boundary::Harness,
            &CapabilityContext::no_grants("private", root),
        )
        .map(|bundle| format!("compiled {} sites", bundle.sites.len()))
        .map_err(|refusal| refusal.to_string())
    };
    // DSH's closed grammar models no `--profile` at all, so the planted
    // profile is refused as a token it cannot place (decision 0066 ruling
    // 6), before any catalogue is read: the arbitrary-profile refusal of
    // the realm-capability-grants delta.
    let refused = |site: &str, option: &str, harness: &str| {
        if harness == "dsh" {
            return Err(format!(
                "bundle: bundle: seat '{site}' (office '{site}') in realm 'private': its \
                 arguments do not parse: the 'dsh' command grammar cannot place argument 5 \
                 ('{option}'): it names no option. A harness brokkr launches is parsed against a \
                 model of its options, and a token that grammar cannot place is refused rather \
                 than passed through, because a control nobody can read is a control nobody can \
                 rule on (decision 0066 ruling 6) (composed: authored -> authored-base)"
            ));
        }
        Err(format!(
            "bundle: bundle: seat '{site}' (office '{site}') in realm 'private': its arguments \
             carry '{option}' (argument 5), a capability-bearing option of harness '{harness}'. \
             A recipe authors no capability-bearing option, whatever its value, polarity or \
             grant: tools come from typed declarations and the realm's grant, composed by the \
             engine alone (operator ruling 1 of 2026-09-23) (composed: authored -> \
             authored-base)"
        ))
    };
    let harnesses: [(&str, &str, &[&str], &str); 4] = [
        (
            "claude",
            "claude-opus-5-5",
            &["--allowedTools", "Read"],
            "--allowedTools",
        ),
        (
            "lanetally",
            "claude-opus-5-5",
            &["--mcp-config=m.json"],
            "--mcp-config",
        ),
        (
            "codex",
            "gpt-6-astra",
            &["-c", "mcp_servers.x.command=\"y\""],
            "--config",
        ),
        (
            "dsh",
            "deepseek-v4-flash",
            &["--profile", "web"],
            "--profile",
        ),
    ];
    let mut failures = Vec::new();
    for (harness, model, extra, option) in harnesses {
        let clean = compile(harness, model, None);
        if clean != Ok("compiled 9 sites".to_string()) {
            failures.push(format!("{harness} unpoisoned: {clean:?}"));
        }
        let untrusted = matches!(harness, "dsh" | "lanetally");
        for site in sites
            .into_iter()
            .filter(|site| !(untrusted && *site == "review"))
        {
            let observed = compile(harness, model, Some((site, extra)));
            let expected = refused(site, option, harness);
            if observed != expected {
                failures.push(format!(
                    "{harness} at {site}:\n  left:  {observed:?}\n  right: {expected:?}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// ------------------------------------------------- rebuild unit 21

/// Rebuild unit 21: a copy of the shipped adapters whose Claude web-search
/// OFF is declared as the argv `off`, or left as shipped where it is `None`.
fn claude_search_off(off: Option<&Value>) -> tempfile::TempDir {
    let adapters = copied_adapters();
    if let Some(off) = off {
        edit_adapter(adapters.path(), "claude", |adapter| {
            adapter["native_capabilities"]["known"]["web-search"]["off"] = json!({"argv": off});
        });
    }
    adapters
}

/// Rebuild unit 21: an unboxed Claude office with no hands and no asks,
/// whose own charter tells its pin apart from the recipe's role.
fn reader_office(operator: &Operator) {
    std::fs::write(
        operator.root().join("agents/charters/reader.md"),
        "# reader\n",
    )
    .unwrap();
    write(
        operator.root(),
        "agents/reader.json",
        &json!({"description": "an office that reads", "charter": "charters/reader.md",
                "models": ["opus"], "efforts": {"opus": "high"}}),
    );
}

/// Rebuild unit 21: the solo bundle's work seat, an inline Claude command
/// with `authored` after its pins, or the office `reader` ([`reader_office`]).
fn claude_work_seat(operator: &Operator, agent: bool, authored: &[&str]) {
    let mut command = vec![
        "{brokkr}",
        "driver",
        "claude",
        "--",
        "--model",
        "claude-opus-5-5",
        "--effort",
        "high",
    ];
    command.extend(authored);
    one_inline_seat(operator, &command);
    if agent {
        let path = operator.root().join("solo/bundle.json");
        let mut bundle: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        bundle["seats"]["work"] = json!({"results": ["complete"], "agent": "reader"});
        write(operator.root(), "solo/bundle.json", &bundle);
    }
}

/// The charter the compile selected for `label`: its owner, the reference
/// as written, the path and the digest.
fn charter_of(
    bundle: &Bundle,
    label: &str,
) -> (
    brokkr_runtime::bundle::CharterOwner,
    String,
    PathBuf,
    String,
) {
    let pin = bundle.sites[label].charter.as_ref().unwrap();
    (
        pin.owner.clone(),
        pin.reference.clone(),
        pin.path.clone(),
        pin.digest.clone(),
    )
}

/// Rebuild unit 21: a compiled Claude site dispatched as the engine
/// dispatches it ([`dispatched`]) and offered `session` under a SUPPORTED
/// assessment qualified for the markers dispatch wrote. The shipped Claude
/// assessment is unmeasured and declines every offer, so an actual rejoin
/// is handed one, as rebuild unit 15's Claude rows are. The final command
/// the driver would spawn, or its whole refusal.
#[cfg(unix)]
fn claude_rejoined(
    bundle: &Bundle,
    label: &str,
    bin: &Path,
    session: &str,
) -> Result<Vec<String>, String> {
    claude_rejoined_at(bundle, label, 0, bin, session)
}

/// [`claude_rejoined`] for the site's candidate `candidate`, a primary or
/// a selected fallback (rebuild unit 21-fix-b).
#[cfg(unix)]
fn claude_rejoined_at(
    bundle: &Bundle,
    label: &str,
    candidate: usize,
    bin: &Path,
    session: &str,
) -> Result<Vec<String>, String> {
    let work = brokkr_runtime::SeatClass::Work;
    let (extra, mut input) = dispatched(bundle, label, candidate, work, "/w")?;
    input["resume_context"] = json!({"assessment": {"boxed-workspace": {
        "status": "supported",
        "identity": {"version": "2.1.266", "applies_to": "2.1.266"},
        "classes": ["work"], "boundaries": [input["boundary"].clone()],
        "hands": input["hands"].clone(),
        "evidence": {"interface": "i", "restrictions": "r", "root": "o", "accounting": "a"},
        "limitations": [], "reason": null}}});
    brokkr_protocol::adapters::claude_command(bin.to_str().unwrap(), &extra, Some(session), &input)
}

/// The per-site manifest record with each candidate's native declaration
/// digest taken out, and the digests beside it: the digest identifies the
/// adapter bytes a test edited, so it is compared across rows rather than
/// spelled.
fn undigested(site: &Value) -> (Value, Vec<String>) {
    let mut site = site.clone();
    let mut digests = Vec::new();
    for candidate in site["candidates"].as_array_mut().unwrap() {
        let native = candidate["native"].as_object_mut().unwrap();
        if let Some(Value::String(digest)) = native.remove("declaration") {
            digests.push(digest);
        }
    }
    (site, digests)
}

/// Claude's two known powers as its shipped adapter guards them, in the
/// plan the driver is handed.
fn claude_guards() -> Value {
    let guard = |capability: &str, tool: &str| {
        json!({"capability": capability, "config_flags": [], "config_keys": [],
               "feature_flags": [], "features": [], "flags": [],
               "list_flags": ["--tools", "--allowedTools", "--allowed-tools"],
               "tools": [tool],
               "value_flags": ["--model", "--effort", "--permission-mode", "--mcp-config"]})
    };
    json!([
        guard("web-fetch", "WebFetch"),
        guard("web-search", "WebSearch")
    ])
}

/// The compiler's one bounded refusal line (design D6): whole up to 512
/// scalar values, otherwise its first 511 and `…`.
fn bounded(line: String) -> String {
    match line.chars().count() > 512 {
        true => line.chars().take(511).chain(['…']).collect(),
        false => line,
    }
}

/// Rebuild unit 21 (task 21.1; NCT "Second H4"; NCC; NC6), through real
/// realm, dialect and candidate resolution. Only the shipped Claude
/// web-search OFF is changed, to the split `--tools Read`, the joined
/// `--tools=Read` or the joined explicit empty `--tools=`. An inline and an
/// agent-backed unboxed Claude work seat that hold nothing each compile,
/// and three commands are served beside the site's selected charter:
///
/// - the checked cold command;
/// - an ACTUAL eligible rejoin, the offered session after `--resume`;
/// - the offer declined under the compiled (unmeasured) assessment and
///   served cold. That replacement is its own outcome and proves no resume.
///
/// Each carries the effective empty include list (a limit is filled from
/// the holdings alone, unit 12-fix-b I1) beside the independent WebFetch
/// denial, so WebSearch is excluded by the list; a command with only
/// `--disallowedTools WebFetch` fails here. The `--disallowedTools
/// WebSearch` OFF and the shipped selection deny both searches by name.
/// The split explicit empty `--tools ""` is not a supported spelling: its
/// empty argument refuses the adapter load, whole. Boxed, the agent-backed
/// Claude link's hands tool is outside an empty limit, and the conflict
/// refuses rather than weakening the limit. The manifest record, the
/// prompt and the plan hold and claim nothing; the plan is compared whole,
/// and `adapters/tests.rs`'s
/// `a_compiled_managed_read_limit_is_served_whole_cold_and_on_an_eligible_resume`
/// serves the same plan literal.
///
/// Composition evidence only: whether Claude honours the list live is
/// unmeasured and owed to the controller.
#[cfg(unix)]
#[test]
fn a_managed_read_or_empty_limit_is_served_whole_cold_and_on_an_actual_eligible_resume() {
    use brokkr_runtime::bundle::CharterOwner;
    let operator = Operator::new();
    reader_office(&operator);
    let claude = claude_reporting(operator.root(), "2.1.266");
    let session = "019c4b7e-0000-7000-8000-000000000021";
    let context = CapabilityContext::no_grants("private", operator.root());
    let words =
        |parts: &[&str]| -> Vec<String> { parts.iter().map(|part| part.to_string()).collect() };
    let limited = ["--tools", "", "--disallowedTools", "WebFetch"];
    let both = ["--disallowedTools", "WebFetch,WebSearch"];
    let off_argv = |off: &[&str]| json!(off);
    // A row: the case, the OFF declared, the tail served (none where the
    // declaration refuses), and the plan's argv and deny list.
    type Row<'a> = (&'a str, Option<Value>, Option<&'a [&'a str]>, Value, Value);
    let rows: Vec<Row> = vec![
        (
            "split Read",
            Some(off_argv(&["--tools", "Read"])),
            Some(&limited),
            json!(["--tools", "Read"]),
            json!(["WebFetch"]),
        ),
        (
            "joined Read",
            Some(off_argv(&["--tools=Read"])),
            Some(&limited),
            json!(["--tools=Read"]),
            json!(["WebFetch"]),
        ),
        (
            "joined explicit empty",
            Some(off_argv(&["--tools="])),
            Some(&limited),
            json!(["--tools="]),
            json!(["WebFetch"]),
        ),
        (
            "split explicit empty",
            Some(off_argv(&["--tools", ""])),
            None,
            Value::Null,
            Value::Null,
        ),
        (
            "the deny-list positive control",
            Some(off_argv(&["--disallowedTools", "WebSearch"])),
            Some(&both),
            json!(["--disallowedTools", "WebSearch"]),
            json!(["WebFetch"]),
        ),
        (
            "shipped, no include list",
            None,
            Some(&both),
            json!([]),
            json!(["WebFetch", "WebSearch"]),
        ),
    ];
    assert_eq!(rows.len(), 6);
    let switched_off = "provider 'claude' has it natively, the realm does not grant it to this \
                        seat, and it is switched off";
    let not_held = json!({"web-fetch": switched_off, "web-search": switched_off});
    let solo = operator.root().join("solo");
    let library = operator.root().join("agents");
    let mut failures = Vec::new();
    let mut digests: Vec<(String, Vec<String>)> = Vec::new();
    for (case, off, tail, argv, deny) in rows {
        let adapters = claude_search_off(off.as_ref());
        let adapter = adapters.path().join("claude.json");
        let adapter = adapter.display();
        for agent in [false, true] {
            let form = match agent {
                true => "agent-backed",
                false => "inline",
            };
            claude_work_seat(&operator, agent, &[]);
            let compiled = solo_bundle(&operator, adapters.path(), &context);
            let Some(tail) = tail else {
                let unloadable = format!(
                    "adapter 'claude' ({adapter}) 'native_capabilities' at \
                     '/known/web-search/off/argv/1': it does not satisfy \
                     '/definitions/disposition/properties/argv/items/minLength'"
                );
                let expected = match agent {
                    false => bounded(format!(
                        "bundle: seat 'work' (office 'work') in realm 'private': provider 'claude' \
                         is known to carry native capability 'web-search', which this seat does \
                         not hold, and no valid control denies it: the adapter data could not be \
                         loaded ({unloadable}). A known native power is launched only with a \
                         delivered denial, never on what absence implies; repair the adapter data \
                         (decision 0066 ruling 1)"
                    )),
                    true => format!(
                        "bundle: {unloadable}; the adapter data is where a driver's model mapping \
                         (decision 0016) and its trust tier and binding grant (decision 0021) are \
                         declared, and this bundle names an agent, seats a gate, declares a secret \
                         binding or declares typed tools"
                    ),
                };
                let observed = compiled.map(|bundle| bundle.sites.len());
                if observed != Err(expected.clone()) {
                    failures.push(format!(
                        "{case}, {form}:\n  left:  {observed:?}\n  right: {expected:?}"
                    ));
                }
                continue;
            };
            let bundle =
                compiled.unwrap_or_else(|refusal| panic!("{case}, {form} refused: {refusal}"));
            let (office, template, model, charter) = match agent {
                false => (
                    "work",
                    &[][..],
                    None,
                    (
                        CharterOwner::Layer {
                            dir: solo.clone(),
                            key: "roles/role.md".into(),
                        },
                        "roles/role.md".to_string(),
                        solo.join("roles/role.md"),
                        brokkr_core::canonical::sha256_bytes(b"# role\n"),
                    ),
                ),
                true => (
                    "reader",
                    &["--permission-mode", "acceptEdits"][..],
                    Some("opus"),
                    (
                        CharterOwner::Library {
                            agent: "reader".into(),
                            root: library.clone(),
                        },
                        "charters/reader.md".to_string(),
                        library.join("charters/reader.md"),
                        brokkr_core::canonical::sha256_bytes(b"# reader\n"),
                    ),
                ),
            };
            let site = bundle.sites["work"].capabilities.as_ref().unwrap();
            let outcome = &site.outcomes[0];
            let mut candidate = json!({"held": {}, "not_held": not_held, "notices": [],
                "native": {"inventory": "known", "off": ["web-fetch", "web-search"], "on": []},
                "provider": "claude"});
            if let Some(model) = model {
                candidate["model"] = json!(model);
            }
            let (record, digest) = undigested(&site.manifest());
            digests.push((format!("{case}, {form}"), digest));
            let command = |program: &str, resumed: bool| {
                let mut argv = [
                    vec![program.to_string()],
                    words(&["-p", "--output-format", "stream-json", "--verbose"]),
                    words(template),
                    words(&["--model", "claude-opus-5-5", "--effort", "high"]),
                    words(tail),
                ]
                .concat();
                if resumed {
                    argv.extend(words(&["--resume", session]));
                }
                argv
            };
            let shim = claude.to_str().unwrap();
            let observed = json!({
                "charter": format!("{:?}", charter_of(&bundle, "work")),
                "site": record,
                "prompt": outcome.prompt(),
                "plan": outcome.controls(),
                "cold": format!("{:?}", served_as(&bundle, "work", 0,
                    brokkr_runtime::SeatClass::Work, None)),
                "rejoined": format!("{:?}", claude_rejoined(&bundle, "work", &claude, session)),
                "declined": format!("{:?}", served_as(&bundle, "work", 0,
                    brokkr_runtime::SeatClass::Work, Some((&claude, session)))),
            });
            let expected = json!({
                "charter": format!("{charter:?}"),
                "site": {"asks": {}, "candidates": [candidate], "office": office,
                         "subtracted": []},
                "prompt": {"held": {}, "not_held": not_held},
                "plan": {"admits": {}, "argv": argv, "guards": claude_guards(), "hands": 0,
                         "harness": "claude", "inventory": "known", "local": [],
                         "off": ["web-fetch", "web-search"], "on": [], "provider": "claude",
                         "selection": {"allow": [], "deny": deny, "include": [],
                             "flags": {"allow": {"flag": "--allowedTools", "separator": ","},
                                       "deny": {"flag": "--disallowedTools", "separator": ","},
                                       "include": {"flag": "--tools", "separator": ","}}}},
                "cold": format!("{:?}", Ok::<_, String>(command("claude", false))),
                "rejoined": format!("{:?}", Ok::<_, String>(command(shim, true))),
                "declined": format!("{:?}", Ok::<_, String>(command(shim, false))),
            });
            if observed != expected {
                failures.push(format!(
                    "{case}, {form}:\n  left:  {observed:#}\n  right: {expected:#}"
                ));
            }
        }
    }
    // One declaration each, the same for both forms of a row, and every
    // edited declaration a different one from the shipped declaration.
    let declarations: Vec<&String> = digests.iter().map(|(_, digest)| &digest[0]).collect();
    assert_eq!(declarations.len(), 10, "{digests:?}");
    for pair in declarations.chunks(2) {
        assert_eq!(pair[0], pair[1], "{digests:?}");
    }
    let mut distinct: Vec<&String> = declarations.iter().step_by(2).copied().collect();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), 5, "{digests:?}");

    // Boxed, the agent-backed Claude primary of the office `fallback` holds
    // the box's hands tool, which an empty limit does not name.
    let empty = claude_search_off(Some(&json!(["--tools="])));
    let boxed = operator
        .compile_against(empty.path(), &context, Boundary::Namespace, None, None)
        .map(|bundle| bundle.sites.len());
    let refused = "bundle: seat 'chain' (office 'fallback') in realm 'private': the capability \
                   plan's explicit '--tools' restriction for provider 'claude' (naming no tool) \
                   does not name tool 'mcp__brokkr__workspace', which the site's typed hands \
                   admit; an explicit tool list is a hard limit that nothing widens, so the \
                   conflict is refused whole rather than unioned (design D6)";
    if boxed != Err(refused.to_string()) {
        failures.push(format!(
            "boxed, joined explicit empty:\n  left:  {boxed:?}\n  right: {refused:?}"
        ));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Rebuild unit 21 (task 21.1; NCP; RGR; NC6): beside the same managed
/// `--tools Read` OFF, what the recipe writes stays the recipe's, and
/// LaneTally's inventory stays its own.
///
/// - Prompt integrity. A joined prompt value that reads like the list is
///   one inert argument: the cold command and the actual eligible rejoin
///   keep it whole, with the managed empty list and WebFetch denial in
///   their own positions after it. The split spelling, whose value's
///   boundary is ambiguous, refuses at compile.
/// - Authored lists. The recipe's own `--tools=Read` refuses at compile,
///   naming the option and never its value (operator ruling 1).
/// - LaneTally shares Claude's composition path but not its declaration.
///   An inline LaneTally seat on the same copy keeps its shipped
///   unmeasured inventory: its plan and manifest claim nothing, and its
///   real driver spawns the wrapper with no managed list. Its typed allow
///   still refuses at compile with that unmeasured cause (ruling R5), so
///   no LaneTally shape is served a managed limit.
#[cfg(unix)]
#[test]
fn a_managed_read_limit_keeps_prompt_values_authored_lists_and_lanetallys_inventory_apart() {
    use brokkr_runtime::bundle::CharterOwner;
    let operator = Operator::new();
    let claude = claude_reporting(operator.root(), "2.1.266");
    let session = "019c4b7e-0000-7000-8000-000000000121";
    let context = CapabilityContext::no_grants("private", operator.root());
    let adapters = claude_search_off(Some(&json!(["--tools", "Read"])));
    let words =
        |parts: &[&str]| -> Vec<String> { parts.iter().map(|part| part.to_string()).collect() };
    let mut failures = Vec::new();
    let mut check = |case: &str, observed: String, expected: String| {
        if observed != expected {
            failures.push(format!("{case}:\n  left:  {observed}\n  right: {expected}"));
        }
    };

    // The joined prompt value, served cold and on an actual rejoin.
    claude_work_seat(&operator, false, &["--append-system-prompt=--tools Read"]);
    let bundle = solo_bundle(&operator, adapters.path(), &context).unwrap();
    let command = |program: &str| {
        [
            vec![program.to_string()],
            words(&[
                "-p",
                "--output-format",
                "stream-json",
                "--verbose",
                "--model",
                "claude-opus-5-5",
                "--effort",
                "high",
                "--append-system-prompt=--tools Read",
                "--tools",
                "",
                "--disallowedTools",
                "WebFetch",
            ]),
        ]
        .concat()
    };
    let solo = operator.root().join("solo");
    check(
        "the joined prompt value, its charter",
        format!("{:?}", charter_of(&bundle, "work")),
        format!(
            "{:?}",
            (
                CharterOwner::Layer {
                    dir: solo.clone(),
                    key: "roles/role.md".into(),
                },
                "roles/role.md".to_string(),
                solo.join("roles/role.md"),
                brokkr_core::canonical::sha256_bytes(b"# role\n"),
            )
        ),
    );
    check(
        "the joined prompt value, cold",
        format!(
            "{:?}",
            served_as(&bundle, "work", 0, brokkr_runtime::SeatClass::Work, None)
        ),
        format!("{:?}", Ok::<_, String>(command("claude"))),
    );
    let mut rejoined = command(claude.to_str().unwrap());
    rejoined.extend(words(&["--resume", session]));
    check(
        "the joined prompt value, rejoined",
        format!("{:?}", claude_rejoined(&bundle, "work", &claude, session)),
        format!("{:?}", Ok::<_, String>(rejoined)),
    );

    // What the recipe writes and the compile refuses.
    for (case, authored, expected) in [
        (
            "the split prompt value",
            &["--append-system-prompt", "--tools", "Read"][..],
            bounded(
                "bundle: seat 'work' (office 'work') in realm 'private': its arguments do not \
                 parse: the 'claude' command grammar cannot place argument 6 ('--tools'): it \
                 stands where the value of '--append-system-prompt' belongs but reads as an \
                 option, so which of the two it is cannot be told. A harness brokkr launches is \
                 parsed against a model of its options, and a token that grammar cannot place is \
                 refused rather than passed through, because a control nobody can read is a \
                 control nobody can rule on (decision 0066 ruling 6)"
                    .to_string(),
            ),
        ),
        (
            "an authored list",
            &["--tools=Read"][..],
            "bundle: seat 'work' (office 'work') in realm 'private': its arguments carry \
             '--tools' (argument 5), a capability-bearing option of harness 'claude'. A recipe \
             authors no capability-bearing option, whatever its value, polarity or grant: tools \
             come from typed declarations and the realm's grant, composed by the engine alone \
             (operator ruling 1 of 2026-09-23)"
                .to_string(),
        ),
    ] {
        claude_work_seat(&operator, false, authored);
        check(
            case,
            format!(
                "{:?}",
                solo_bundle(&operator, adapters.path(), &context).map(|bundle| bundle.sites.len())
            ),
            format!("{:?}", Err::<usize, _>(expected)),
        );
    }

    // LaneTally, inline, on the same copy: its own unmeasured inventory.
    let tally: Value =
        serde_json::from_slice(&std::fs::read(adapters.path().join("lanetally.json")).unwrap())
            .unwrap();
    let reason = tally["native_capabilities"]["unmeasured"]
        .as_str()
        .unwrap()
        .to_string();
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
    let bundle = solo_bundle(&operator, adapters.path(), &context).unwrap();
    let site = bundle.sites["work"].capabilities.as_ref().unwrap();
    let (record, declaration) = undigested(&site.manifest());
    // The Claude edit does not move LaneTally's declaration: it is the one
    // the shipped adapters compile to.
    let shipped = solo_bundle(&operator, &workspace().join("adapters"), &context).unwrap();
    let (_, shipped) = undigested(
        &shipped.sites["work"]
            .capabilities
            .as_ref()
            .unwrap()
            .manifest(),
    );
    check(
        "lanetally, its declaration",
        format!("{declaration:?}"),
        format!("{shipped:?}"),
    );
    check(
        "lanetally, its manifest record and plan",
        json!({"site": record, "plan": site.outcomes[0].controls(),
               "prompt": site.outcomes[0].prompt()})
        .to_string(),
        json!({
            "site": {"asks": {}, "office": "work", "subtracted": [], "candidates": [
                {"held": {}, "not_held": {}, "notices": [], "provider": "lanetally",
                 "native": {"inventory": "unmeasured", "reason": reason}}]},
            "plan": {"inventory": "unmeasured", "provider": "lanetally", "harness": "lanetally",
                     "reason": reason},
            "prompt": {"held": {}, "not_held": {}, "native": format!(
                "Provider 'lanetally' declares its native capabilities unmeasured ({reason}); \
                 nothing is claimed about what it can reach on its own")},
        })
        .to_string(),
    );
    let (spawned, _) = driver_spawned(&bundle, "work", 0, operator.root(), None, None);
    let harness = operator.root().join("lanetally-harness");
    check(
        "lanetally, cold through its real driver",
        format!("{spawned:?}"),
        format!(
            "{:?}",
            Ok::<_, String>(
                [
                    vec![harness.to_str().unwrap().to_string()],
                    words(&[
                        "-p",
                        "--output-format",
                        "stream-json",
                        "--verbose",
                        "--model",
                        "claude-opus-5-5",
                        "--effort",
                        "high",
                    ]),
                ]
                .concat()
            )
        ),
    );
    typed_allow(&operator, json!(["git"]));
    check(
        "lanetally, its typed allow",
        format!(
            "{:?}",
            solo_bundle(&operator, adapters.path(), &context).map(|bundle| bundle.sites.len())
        ),
        format!(
            "{:?}",
            Err::<usize, _>(bounded(format!(
                "bundle: seat 'work' (office 'work') in realm 'private': its typed 'tools.allow' \
                 refuses at compile, as harness 'lanetally' of provider 'lanetally' has native \
                 controls its adapter declares unmeasured (ruling R5 of 2026-09-29; design \
                 D5.3): {reason}"
            )))
        ),
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Rebuild unit 21: the dialect `codex-search-hosts` — the shipped Codex
/// search dialect with a schema-valid `allow.hosts` restriction — written
/// into the operator's configuration, and the v6 grant of `web-search`
/// through it restricted to `a.example`: the grant as written, and the
/// realm context that carries it.
fn hosts_grant(operator: &Operator) -> (Value, CapabilityContext) {
    let root = operator.root();
    let mut hosts: Value = serde_json::from_slice(
        &std::fs::read(root.join("dialects/tools/codex-native-search.json")).unwrap(),
    )
    .unwrap();
    hosts["name"] = json!("codex-search-hosts");
    hosts["restrictions"] = json!({"type": "object", "additionalProperties": false,
        "properties": {"allow": {"type": "object", "additionalProperties": false,
            "properties": {"hosts": {"type": "array", "items": {"type": "string"}}}}}});
    write(root, "dialects/tools/codex-search-hosts.json", &hosts);
    let grant = json!({"dialect": "codex-search-hosts", "allow": {"hosts": ["a.example"]}});
    let context = operator.context(json!({"web-search": grant}));
    (grant, context)
}

/// Rebuild unit 21 (task 21.3's CQ1 rows; RG4 CQ1; NCC "H3 a nonempty
/// restriction is never delivered in slice one"; design D11), through a
/// real v6 realm grant, a schema-valid dialect that declares an
/// `allow.hosts` restriction, and candidate resolution on the shipped
/// Codex adapter. An inline and an agent-backed unboxed Codex work seat
/// reach only CQ1's outcomes:
///
/// - requires refuses the compile with the complete cause;
/// - wants drops with its exact notice and is served its native OFF;
/// - a grant no seat uses stays pinned, inactive, with no notice.
///
/// The realm's grant is pinned exactly as written, and no holding, prompt
/// or plan carries the restriction. Each served outcome's cold command
/// and ACTUAL eligible rejoin (`exec resume`, the offered thread, under
/// the assessment the bundle compiled from the shipped adapter) is a whole
/// literal beside the site's charter. Where the adapter declares a
/// restriction transport, the same restricted requires refuses and the want
/// drops, each naming the deferral, an unused grant stays pinned and
/// inactive as above, and an unrestricted grant is held with the empty
/// restriction and no transport argument (rebuild unit 21-fix-b, R2). Every
/// other site shape is
/// [`a_restricted_grant_reaches_only_cq1s_outcomes_at_every_compiled_codex_site_shape`]'s.
#[cfg(unix)]
#[test]
fn a_restricted_grant_reaches_only_cq1s_outcomes_cold_and_on_an_actual_eligible_resume() {
    use brokkr_runtime::bundle::CharterOwner;
    let operator = Operator::new();
    let shim = codex_reporting(operator.root(), "0.154.0");
    let (grant, restricted) = hosts_grant(&operator);
    let unrestricted = operator.context(json!({"web-search": {"dialect": "codex-search-hosts"}}));
    let shipped = workspace().join("adapters");
    let transported = copied_adapters();
    edit_adapter(transported.path(), "codex", |adapter| {
        adapter["native_capabilities"]["known"]["web-search"]["restrictions"] =
            json!({"argv": ["--image", "{restrictions_json}"]});
    });
    let office_asks = |strength: &str| {
        let path = operator.root().join("agents/searcher.json");
        let mut agent: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        agent["capabilities"] = json!({"web-search": strength});
        write(operator.root(), "agents/searcher.json", &agent);
    };
    let mut failures = Vec::new();

    // Requires refuses, inline and through the office.
    let inexpressible = "provider 'codex' cannot express restriction 'allow.hosts'";
    let requires = |seat: &str, office: &str| {
        format!(
            "bundle: seat '{seat}' (office '{office}') in realm 'private': requires capability \
             'web-search' through dialect 'codex-search-hosts', but {inexpressible}; the \
             capability cannot be held under this grant"
        )
    };
    // Over a declared transport, requires refuses naming the deferral
    // (rebuild unit 21-fix-b, R2).
    let requires_deferred = |seat: &str, office: &str| {
        format!(
            "bundle: seat '{seat}' (office '{office}') in realm 'private': requires capability \
             'web-search' through dialect 'codex-search-hosts', but {inexpressible} through its \
             declared transport, which carries only the empty restriction until a provider \
             restriction transport is measured (operator ruling of 2026-09-25); the capability \
             cannot be held under this grant"
        )
    };
    for (adapters, transport) in [(shipped.as_path(), false), (transported.path(), true)] {
        let inline_requires = operator
            .compile_against(
                adapters,
                &restricted,
                Boundary::Harness,
                Some(json!({"web-search": "requires"})),
                None,
            )
            .map(|bundle| bundle.sites.len());
        office_asks("requires");
        let office_requires = operator
            .compile_against(adapters, &restricted, Boundary::Harness, None, None)
            .map(|bundle| bundle.sites.len());
        office_asks("wants");
        let expected = match transport {
            false => [requires("inline", "inline"), requires("agent", "searcher")],
            true => [
                requires_deferred("inline", "inline"),
                requires_deferred("agent", "searcher"),
            ],
        };
        for (observed, expected) in [inline_requires, office_requires].into_iter().zip(expected) {
            if observed != Err(expected.clone()) {
                failures.push(format!(
                    "requires, transport {transport}:\n  left:  {observed:?}\n  right: \
                     {expected:?}"
                ));
            }
        }
    }

    // The served outcomes. Codex's two sites share one command shape: the
    // inline seat's lowered class and the office's are both
    // `workspace-write` under `harness`.
    let command = |off: bool| {
        let mut argv: Vec<String> = [
            "codex",
            "exec",
            "--json",
            "-C",
            "/w",
            "-c",
            "model_reasoning_effort=\"high\"",
            "--model",
            "gpt-6-astra",
            "--sandbox",
            "workspace-write",
        ]
        .map(String::from)
        .to_vec();
        if off {
            argv.extend(OFF.map(String::from));
        }
        argv
    };
    let resumed = |off: bool| {
        let mut argv: Vec<String> = [
            shim.to_str().unwrap(),
            "exec",
            "resume",
            "--json",
            "-c",
            "sandbox_mode=\"workspace-write\"",
            "-c",
            "model_reasoning_effort=\"high\"",
            "--model",
            "gpt-6-astra",
        ]
        .map(String::from)
        .to_vec();
        if off {
            argv.extend(OFF.map(String::from));
        }
        argv.extend([THREAD.to_string(), "-".to_string()]);
        argv
    };
    let recipe = operator.root().join("bundle");
    let library = operator.root().join("agents");
    let charter = |label: &str| match label {
        "inline" => (
            CharterOwner::Layer {
                dir: recipe.clone(),
                key: "roles/role.md".into(),
            },
            "roles/role.md".to_string(),
            recipe.join("roles/role.md"),
            brokkr_core::canonical::sha256_bytes(b"# role\n"),
        ),
        _ => (
            CharterOwner::Library {
                agent: "searcher".into(),
                root: library.clone(),
            },
            "charters/searcher.md".to_string(),
            library.join("charters/searcher.md"),
            brokkr_core::canonical::sha256_bytes(b"# searcher\n"),
        ),
    };
    let held = json!({"web-search": {
        "classes": ["reads", "egress"],
        "definition_sha256": brokkr_core::canonical::sha256_bytes(
            &std::fs::read(operator.root().join("capabilities/web-search.json")).unwrap()),
        "dialect": "codex-search-hosts",
        "dialect_sha256": brokkr_core::canonical::sha256_bytes(
            &std::fs::read(operator.root().join("dialects/tools/codex-search-hosts.json"))
                .unwrap()),
        "restrictions": {}, "tools": ["web_search"]}});
    let deferred = format!(
        "{inexpressible} through its declared transport, which carries only the empty \
         restriction until a provider restriction transport is measured (operator ruling of \
         2026-09-25)"
    );
    let dropped = format!("{inexpressible}; native capability remains OFF");
    let dropped_deferred = format!("{deferred}; native capability remains OFF");
    let notice = |label: &str, office: &str, but: &str| {
        format!(
            "seat '{label}' (office '{office}') in realm 'private': dropped wanted capability \
             'web-search' through dialect 'codex-search-hosts' because {but}"
        )
    };
    // The realm grants web-search; the inline seat asks nothing (operator
    // ruling of 2026-09-29, rebuild unit 21-fix-a, R3).
    let unasked = "provider 'codex' has it natively, granted, but this seat does not request \
                   it, and it is switched off";
    let subtracted = "this seat subtracted it from its office's asks";
    struct Served<'a> {
        case: &'a str,
        adapters: &'a Path,
        context: &'a CapabilityContext,
        asks: Option<Value>,
        seat: Option<Value>,
        grant: Value,
    }
    let rows = [
        Served {
            case: "wants",
            adapters: &shipped,
            context: &restricted,
            asks: Some(json!({"web-search": "wants"})),
            seat: None,
            grant: grant.clone(),
        },
        Served {
            case: "unused",
            adapters: &shipped,
            context: &restricted,
            asks: None,
            seat: Some(json!({})),
            grant: grant.clone(),
        },
        Served {
            case: "wants, over a declared transport",
            adapters: transported.path(),
            context: &restricted,
            asks: Some(json!({"web-search": "wants"})),
            seat: None,
            grant: grant.clone(),
        },
        Served {
            case: "unused, over a declared transport",
            adapters: transported.path(),
            context: &restricted,
            asks: None,
            seat: Some(json!({})),
            grant: grant.clone(),
        },
        Served {
            case: "the empty restriction, over a declared transport",
            adapters: transported.path(),
            context: &unrestricted,
            asks: Some(json!({"web-search": "wants"})),
            seat: None,
            grant: json!({"dialect": "codex-search-hosts"}),
        },
    ];
    for row in rows {
        let bundle = operator
            .compile_against(
                row.adapters,
                row.context,
                Boundary::Harness,
                row.asks.clone(),
                row.seat.clone(),
            )
            .unwrap_or_else(|refusal| panic!("{} refused: {refusal}", row.case));
        let pinned = &bundle.manifest["capabilities"]["grants"]["web-search"];
        if *pinned != row.grant {
            failures.push(format!(
                "{}, the pinned grant:\n  left:  {pinned}\n  right: {}",
                row.case, row.grant
            ));
        }
        for (label, office) in [("inline", "inline"), ("agent", "searcher")] {
            let site = bundle.sites[label].capabilities.as_ref().unwrap();
            let (record, _) = undigested(&site.manifest());
            let (asks, subtracted_names, holding, not_held, notices, on, off) =
                match (row.case, label) {
                    ("unused" | "unused, over a declared transport", "inline") => (
                        json!({}),
                        json!([]),
                        json!({}),
                        json!({"web-search": unasked}),
                        json!([]),
                        json!([]),
                        true,
                    ),
                    ("unused" | "unused, over a declared transport", _) => (
                        json!({}),
                        json!(["web-search"]),
                        json!({}),
                        json!({"web-search": subtracted}),
                        json!([]),
                        json!([]),
                        true,
                    ),
                    ("the empty restriction, over a declared transport", _) => (
                        json!({"web-search": "wants"}),
                        json!([]),
                        held.clone(),
                        json!({}),
                        json!([]),
                        json!(["web-search"]),
                        false,
                    ),
                    (case, _) => {
                        let but = match case {
                            "wants" => &dropped,
                            _ => &dropped_deferred,
                        };
                        (
                            json!({"web-search": "wants"}),
                            json!([]),
                            json!({}),
                            json!({"web-search": but}),
                            json!([notice(label, office, but)]),
                            json!([]),
                            true,
                        )
                    }
                };
            let mut candidate = json!({"held": holding, "not_held": not_held,
                "notices": notices, "provider": "codex",
                "native": {"inventory": "known", "on": on,
                           "off": if off { json!(["web-search"]) } else { json!([]) }}});
            if label == "agent" {
                candidate["model"] = json!("astra");
            }
            let tools = match off {
                true => json!({}),
                false => json!({"web-search": {"tools": ["web_search"]}}),
            };
            let observed = json!({
                "charter": format!("{:?}", charter_of(&bundle, label)),
                "site": record,
                "prompt": site.outcomes[0].prompt(),
                "cold": format!("{:?}", served_as(&bundle, label, 0,
                    brokkr_runtime::SeatClass::Work, None)),
                "rejoined": format!("{:?}", served_as(&bundle, label, 0,
                    brokkr_runtime::SeatClass::Work, Some((&shim, THREAD)))),
            });
            let expected = json!({
                "charter": format!("{:?}", charter(label)),
                "site": {"asks": asks, "candidates": [candidate], "office": office,
                         "subtracted": subtracted_names},
                "prompt": {"held": tools, "not_held": not_held},
                "cold": format!("{:?}", Ok::<_, String>(command(off))),
                "rejoined": format!("{:?}", Ok::<_, String>(resumed(off))),
            });
            if observed != expected {
                failures.push(format!(
                    "{}, {label}:\n  left:  {observed:#}\n  right: {expected:#}",
                    row.case
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Rebuild unit 21-fix-b: one seat `x` of `shape` — a single work seat, a
/// gate, a panel (`member` and `peer`), a sequence (`step` then `next`), a
/// select (case `engine` and the default) or a single work seat the recipe
/// inherits from its base — every site of it seated with `site`, compiled
/// under `boundary` against `adapters` in the realm `context`.
fn one_shape(
    operator: &Operator,
    adapters: &Path,
    boundary: Boundary,
    context: &CapabilityContext,
    shape: &str,
    site: &Value,
) -> Result<Bundle, String> {
    let root = operator.root();
    // The first step names its results; the last receives the seat's.
    let step = |name: &str| {
        let mut step = site.clone();
        step["name"] = json!(name);
        if name == "step" {
            step["results"] = json!(["complete"]);
        }
        step
    };
    let (results, mut body) = match shape {
        "gate" => {
            let mut body = site.clone();
            body["class"] = json!("gate");
            (json!(["clean"]), body)
        }
        "panel" => (
            json!(["pass", "fail"]),
            json!({"aggregate": "unanimous-pass", "panel": {"member": site, "peer": site}}),
        ),
        "sequence" => (
            json!(["complete"]),
            json!({"sequence": [step("step"), step("next")]}),
        ),
        "select" => (
            json!(["complete"]),
            json!({"select": {"on": "strategy", "cases": {"engine": site}, "default": site}}),
        ),
        _ => (json!(["complete"]), site.clone()),
    };
    body["results"] = results.clone();
    let mut rules: Vec<Value> = results
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(at, result)| {
            json!({"id": format!("S{at}"), "from": "x", "result": result, "next": "review",
                   "reason": "r"})
        })
        .collect();
    rules.push(
        json!({"id": "R", "from": "review", "result": "clean", "next": "done",
                      "reason": "r"}),
    );
    // The protected review gate every layered policy keeps, on a plain
    // custom driver that composes nothing.
    let review = json!({"results": ["clean"], "role": "roles/x.md",
                        "driver": {"command": ["driver"]}});
    for layer in ["shape", "shape-base"] {
        std::fs::create_dir_all(root.join(layer).join("roles")).unwrap();
        std::fs::write(root.join(layer).join("roles/x.md"), "# x\n").unwrap();
    }
    write(
        root,
        "shape-base/policy.json",
        &json!({"phases": ["x", "review", "done"], "initial": "x", "terminal": ["done"],
                "rules": rules}),
    );
    let (base, top) = match shape {
        "inherited" => (json!({"x": body, "review": review}), json!({})),
        _ => (json!({"review": review}), json!({"x": body})),
    };
    write(
        root,
        "shape-base/bundle.json",
        &json!({"name": "shape-base", "policy": "policy.json", "seats": base}),
    );
    write(
        root,
        "shape/bundle.json",
        &json!({"name": "shape", "extends": "shape-base", "seats": top}),
    );
    Bundle::compile_with_capabilities(
        &root.join("shape"),
        &root.join("agents"),
        adapters,
        Some("private"),
        None,
        boundary,
        context,
    )
    .map_err(|refusal| refusal.to_string())
}

/// The charter a site of rebuild unit 20's matrix is told, as it wrote it:
/// an inline site's own file of `layer` (`# <label>\n`), or its office's
/// (`# <office>\n`). The owner, the reference as written, the path and
/// the digest.
fn matrix_charter(
    root: &Path,
    seated: Result<(&str, &str), &str>,
) -> (
    brokkr_runtime::bundle::CharterOwner,
    String,
    PathBuf,
    String,
) {
    use brokkr_runtime::bundle::CharterOwner;
    match seated {
        Ok((layer, label)) => {
            let dir = root.join(layer);
            let reference = format!("roles/{}.md", label.replace(':', "-"));
            (
                CharterOwner::Layer {
                    dir: dir.clone(),
                    key: reference.clone(),
                },
                reference.clone(),
                dir.join(&reference),
                brokkr_core::canonical::sha256_bytes(format!("# {label}\n").as_bytes()),
            )
        }
        Err(agent) => {
            let library = root.join("agents");
            let reference = format!("charters/{agent}.md");
            (
                CharterOwner::Library {
                    agent: agent.to_string(),
                    root: library.clone(),
                },
                reference.clone(),
                library.join(&reference),
                brokkr_core::canonical::sha256_bytes(format!("# {agent}\n").as_bytes()),
            )
        }
    }
}

/// Rebuild unit 21-fix-b (tasks 21.1 and 21.3; unit 21's review return R2;
/// NCT "Second H4"; NCC; NC6; MPL): THE MANAGED READ/EMPTY LIMIT AT EVERY
/// COMPILED CLAUDE SITE SHAPE. Rebuild unit 20's matrix
/// ([`compile_every_shape_on`]) is compiled through real realm, dialect and
/// candidate resolution on a copy of the shipped adapters whose only change
/// is Claude's web-search OFF — the split `--tools Read`, or the joined
/// explicit empty `--tools=` — in a realm that grants nothing. The rows
/// are every compiled Claude candidate, no more and no fewer: the inline
/// Claude work seat, the protected gate `review`, a panel member and its
/// peer, a sequence step and the next, a select case and the default, and
/// the seat the recipe inherits from `base`; at each of those shapes the
/// office `pair-claude`'s Claude primary and the office `pair`'s selected
/// Claude FALLBACK; and the typed `claude-typed`.
///
/// Each row asserts, beside the site's selected charter, its manifest
/// candidate record (nothing held, both powers OFF with their exact
/// reasons, no notice), its prompt, its whole plan, and whole commands:
/// cold, and at a work site the ACTUAL eligible rejoin
/// ([`claude_rejoined_at`]) and the offer declined under the compiled
/// (unmeasured) assessment, served cold. Every served command carries
/// `--tools "" --disallowedTools WebFetch`. `claude-typed`'s lowered
/// `Bash(cargo:*)` is outside the empty limit, so each of its commands is
/// the driver's whole D6 refusal (I1; rebuild unit 12-fix-c). A gate is
/// offered no session.
///
/// Boxed (`namespace`), a site's typed hands put `mcp__brokkr__workspace`
/// outside the limit. Every shape ([`one_shape`]) seated inline with hands,
/// through a Claude office with hands, or through an office whose Claude
/// link is the fallback, refuses the compile whole, naming the site and
/// its office; on the shipped adapters the same shapes compile, each site
/// told the charter its walk selected — the layer's own `roles/x.md` inline
/// (the base's for the inherited seat), or its office's. Under
/// `harness` a Claude site with hands is refused by decision 0046 before
/// any restriction applies, so it is no restriction row.
///
/// Composition evidence only: whether Claude honours the list live is
/// unmeasured and owed to the controller.
#[cfg(unix)]
#[test]
fn a_managed_read_or_empty_limit_reaches_every_compiled_claude_site_shape() {
    use brokkr_runtime::SeatClass::Work;
    let operator = Operator::new();
    let root = operator.root();
    let claude = claude_reporting(root, "2.1.266");
    let shim = claude.to_str().unwrap();
    let session = "019c4b7e-0000-7000-8000-000000000221";
    let context = CapabilityContext::no_grants("private", root);
    let hands = json!({"kind": "workspace", "network": false, "binds": []});
    for (office, models) in [
        ("boxed-claude", &["opus"][..]),
        ("boxed-pair", &["astra", "opus"]),
    ] {
        std::fs::write(
            root.join(format!("agents/charters/{office}.md")),
            format!("# {office}\n"),
        )
        .unwrap();
        let efforts: serde_json::Map<String, Value> = models
            .iter()
            .map(|model| (model.to_string(), json!("high")))
            .collect();
        write(
            root,
            &format!("agents/{office}.json"),
            &json!({"description": "an office", "charter": format!("charters/{office}.md"),
                    "models": models, "efforts": efforts, "hands": hands}),
        );
    }
    let words = |parts: &[&[&str]]| -> Vec<String> {
        parts
            .iter()
            .flat_map(|part| part.iter().map(|word| word.to_string()))
            .collect()
    };
    let lead = ["-p", "--output-format", "stream-json", "--verbose"];
    let pins = ["--model", "claude-opus-5-5", "--effort", "high"];
    let template = ["--permission-mode", "acceptEdits"];
    let limited = ["--tools", "", "--disallowedTools", "WebFetch"];
    let switched_off = "provider 'claude' has it natively, the realm does not grant it to this \
                        seat, and it is switched off";
    let not_held = json!({"web-fetch": switched_off, "web-search": switched_off});

    // (site, candidate, class, where its charter is, through an office)
    type Row<'a> = (
        String,
        usize,
        brokkr_runtime::SeatClass,
        Result<(&'a str, String), &'a str>,
    );
    let mut rows: Vec<Row> = Vec::new();
    for (carrier, candidate) in [("claude", 0), ("pair-claude", 0), ("pair", 1)] {
        for (label, class, inherited) in carrier_sites(carrier, true) {
            let layer = if inherited { "base" } else { "matrix" };
            let seated = match carrier {
                "claude" => Ok((layer, label.clone())),
                office => Err(office),
            };
            rows.push((label, candidate, class, seated));
        }
    }
    rows.push((
        "claude-typed".into(),
        0,
        Work,
        Ok(("matrix", "claude-typed".into())),
    ));
    assert_eq!(rows.len(), 28);

    let mut failures = Vec::new();
    for (case, off, naming) in [
        ("split Read", json!(["--tools", "Read"]), "naming Read"),
        (
            "joined explicit empty",
            json!(["--tools="]),
            "naming no tool",
        ),
    ] {
        let adapters = claude_search_off(Some(&off));
        let bundle = compile_every_shape_on(&operator, None, adapters.path(), &context, None)
            .unwrap_or_else(|refusal| panic!("{case}: the matrix compiles: {refusal}"));
        // The rows are every compiled Claude candidate, and nothing else.
        let compiled: std::collections::BTreeSet<(String, usize)> = bundle
            .sites
            .iter()
            .flat_map(|(label, facts)| {
                let outcomes = &facts.capabilities.as_ref().unwrap().outcomes;
                (0..outcomes.len())
                    .filter(|at| outcomes[*at].provider == "claude")
                    .map(move |at| (label.clone(), at))
            })
            .collect();
        let written: std::collections::BTreeSet<(String, usize)> = rows
            .iter()
            .map(|(label, candidate, ..)| (label.clone(), *candidate))
            .collect();
        assert_eq!(written, compiled, "{case}");
        let typed = format!(
            "refusing to invoke the agent CLI: the capability plan's explicit '--tools' \
             restriction for provider 'claude' ({naming}) does not name tool 'Bash', which the \
             local permissions of the site's typed 'tools.allow' admit; an explicit tool list \
             is a hard limit that nothing widens, so the conflict is refused whole rather than \
             unioned (design D6)"
        );
        for (label, candidate, class, seated) in &rows {
            let (label, candidate, class) = (label.as_str(), *candidate, *class);
            let office = seated.is_err();
            let site = bundle.sites[label].capabilities.as_ref().unwrap();
            let outcome = &site.outcomes[candidate];
            let served = |program: &str, resumed: bool| -> Result<Vec<String>, String> {
                if label == "claude-typed" {
                    return Err(typed.clone());
                }
                let mut argv = match office {
                    true => words(&[&[program], &lead, &template, &pins, &limited]),
                    false => words(&[&[program], &lead, &pins, &limited]),
                };
                if resumed {
                    argv.extend(words(&[&["--resume", session]]));
                }
                Ok(argv)
            };
            let mut record = json!({"held": {}, "not_held": not_held, "notices": [],
                "native": {"inventory": "known", "off": ["web-fetch", "web-search"], "on": []},
                "provider": "claude"});
            if office {
                record["model"] = json!("opus");
            }
            let local = match label {
                "claude-typed" => json!(["Bash(cargo:*)"]),
                _ => json!([]),
            };
            let seated = match seated {
                Ok((layer, label)) => Ok((*layer, label.as_str())),
                Err(office) => Err(*office),
            };
            let offered = class == Work;
            let observed = json!({
                "charter": format!("{:?}", charter_of(&bundle, label)),
                "record": undigested(&site.manifest()).0["candidates"][candidate],
                "prompt": outcome.prompt(),
                "plan": outcome.controls(),
                "cold": format!("{:?}", served_as(&bundle, label, candidate, class, None)),
                "rejoined": offered.then(|| format!("{:?}",
                    claude_rejoined_at(&bundle, label, candidate, &claude, session))),
                "declined": offered.then(|| format!("{:?}",
                    served_as(&bundle, label, candidate, class, Some((&claude, session))))),
            });
            let expected = json!({
                "charter": format!("{:?}", matrix_charter(root, seated)),
                "record": record,
                "prompt": {"held": {}, "not_held": not_held},
                "plan": {"admits": {}, "argv": off, "guards": claude_guards(), "hands": 0,
                         "harness": "claude", "inventory": "known", "local": local,
                         "off": ["web-fetch", "web-search"], "on": [], "provider": "claude",
                         "selection": {"allow": [], "deny": ["WebFetch"], "include": [],
                             "flags": {"allow": {"flag": "--allowedTools", "separator": ","},
                                       "deny": {"flag": "--disallowedTools", "separator": ","},
                                       "include": {"flag": "--tools", "separator": ","}}}},
                "cold": format!("{:?}", served("claude", false)),
                "rejoined": offered.then(|| format!("{:?}", served(shim, true))),
                "declined": offered.then(|| format!("{:?}", served(shim, false))),
            });
            if observed != expected {
                failures.push(format!(
                    "{case}, {label}[{candidate}]:\n  left:  {observed:#}\n  right: {expected:#}"
                ));
            }
        }

        // Boxed: every shape, inline with hands, through a Claude office
        // with hands, and through one whose Claude link is the fallback.
        let inline = json!({"role": "roles/x.md", "hands": hands, "driver": {"command": [
            "{brokkr}", "driver", "claude", "--", "--model", "claude-opus-5-5", "--effort",
            "high"]}});
        for (shape, label) in [
            ("single", "x"),
            ("gate", "x"),
            ("panel", "x:member"),
            ("sequence", "x:step"),
            ("select", "x:engine"),
            ("inherited", "x"),
        ] {
            let layer = match shape {
                "inherited" => "shape-base",
                _ => "shape",
            };
            for (site, office, seated) in [
                (&inline, label, Ok((layer, "x"))),
                (
                    &json!({"agent": "boxed-claude"}),
                    "boxed-claude",
                    Err("boxed-claude"),
                ),
                (
                    &json!({"agent": "boxed-pair"}),
                    "boxed-pair",
                    Err("boxed-pair"),
                ),
            ] {
                let refused = format!(
                    "bundle: bundle: seat '{label}' (office '{office}') in realm 'private': the \
                     capability plan's explicit '--tools' restriction for provider 'claude' \
                     ({naming}) does not name tool 'mcp__brokkr__workspace', which the site's \
                     typed hands admit; an explicit tool list is a hard limit that nothing \
                     widens, so the conflict is refused whole rather than unioned (design D6) \
                     (composed: shape -> shape-base)"
                );
                // On the shipped adapters the same shape compiles, and the
                // site is told the charter its walk selected (review return
                // R2 of rebuild unit 21-fix-b).
                let shipped = workspace().join("adapters");
                let observed = [adapters.path(), shipped.as_path()].map(|adapters| {
                    one_shape(
                        &operator,
                        adapters,
                        Boundary::Namespace,
                        &context,
                        shape,
                        site,
                    )
                    .map(|bundle| format!("{:?}", charter_of(&bundle, label)))
                });
                let expected = [
                    Err(refused),
                    Ok(format!("{:?}", matrix_charter(root, seated))),
                ];
                if observed != expected {
                    failures.push(format!(
                        "{case}, boxed {shape} through {office}:\n  left:  {observed:?}\n  \
                         right: {expected:?}"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Rebuild unit 21-fix-b (task 21.3's CQ1 rows; unit 21's review return
/// R2; RG4 CQ1; NCC; NC6; MPL; design D11): CQ1 AT EVERY COMPILED CODEX
/// SITE SHAPE. Rebuild unit 20's matrix ([`compile_every_shape_on`]) is
/// compiled on the shipped adapters in a realm granting `web-search`
/// through `codex-search-hosts` restricted to `a.example`
/// ([`hosts_grant`]), with every inline site and every office asking for
/// it. The rows are every compiled Codex candidate, no more and no fewer:
/// the inline Codex work seat (its typed `workspace-write`), its gate
/// (typed `read-only`), a panel member and its peer, a sequence step and
/// the next, a select case and the default, and the seat inherited from
/// `base`; at each of those shapes the office `pair`'s Codex primary, the
/// office `pair-claude`'s selected Codex FALLBACK, and the office `boxed`,
/// whose hands `harness` serves with the harness's own sandbox.
///
/// Wants drops at every one: each row asserts, beside the site's selected
/// charter, its manifest candidate record (nothing held, the want's exact
/// not-held reason and its one notice naming the site and office), its
/// prompt, and whole commands that carry the measured OFF: cold, and at a
/// work site the offered session under the assessment the bundle compiled
/// — an ACTUAL `exec resume` for the typed work seat and the boxed office,
/// declined and served cold for every other shape. The grant is pinned as
/// written and no site's record carries its host. Requires refuses the same
/// matrix whole, at the first site it resolves. The office `boxed` is not
/// boxed here: `harness` is no boxed boundary. The boxed rows, under
/// `namespace`, are
/// [`a_restricted_grant_reaches_only_cq1s_outcomes_at_every_boxed_codex_site_shape`]'s.
#[cfg(unix)]
#[test]
fn a_restricted_grant_reaches_only_cq1s_outcomes_at_every_compiled_codex_site_shape() {
    use brokkr_runtime::SeatClass::{Gate, Work};
    let operator = Operator::new();
    let root = operator.root();
    let codex = codex_reporting(root, "0.154.0");
    let shim = codex.to_str().unwrap();
    let (grant, restricted) = hosts_grant(&operator);
    let shipped = workspace().join("adapters");
    let mut failures = Vec::new();

    let requires = compile_every_shape_on(
        &operator,
        None,
        &shipped,
        &restricted,
        Some(&json!({"web-search": "requires"})),
    )
    .map(|bundle| bundle.sites.len());
    let refused = "bundle: bundle: seat 'boxed' (office 'boxed') in realm 'private': requires \
                   capability 'web-search' through dialect 'codex-search-hosts', but provider \
                   'codex' cannot express restriction 'allow.hosts'; the capability cannot be \
                   held under this grant (composed: matrix -> base)";
    if requires != Err(refused.to_string()) {
        failures.push(format!(
            "requires:\n  left:  {requires:?}\n  right: {refused:?}"
        ));
    }

    let bundle = compile_every_shape_on(
        &operator,
        None,
        &shipped,
        &restricted,
        Some(&json!({"web-search": "wants"})),
    )
    .unwrap_or_else(|refusal| panic!("the wanting matrix compiles: {refusal}"));
    let pinned = &bundle.manifest["capabilities"]["grants"]["web-search"];
    if *pinned != grant {
        failures.push(format!(
            "the pinned grant:\n  left:  {pinned}\n  right: {grant}"
        ));
    }

    // (site, candidate, class, where its charter is, the office)
    type Row<'a> = (
        String,
        usize,
        brokkr_runtime::SeatClass,
        Result<(&'a str, String), &'a str>,
    );
    let mut rows: Vec<Row> = Vec::new();
    for (carrier, candidate) in [("codex", 0), ("pair", 0), ("pair-claude", 1), ("boxed", 0)] {
        for (label, class, inherited) in carrier_sites(carrier, true) {
            let layer = if inherited { "base" } else { "matrix" };
            let seated = match carrier {
                "codex" => Ok((layer, label.clone())),
                office => Err(office),
            };
            rows.push((label, candidate, class, seated));
        }
    }
    assert_eq!(rows.len(), 36);
    let compiled: std::collections::BTreeSet<(String, usize)> = bundle
        .sites
        .iter()
        .flat_map(|(label, facts)| {
            let outcomes = &facts.capabilities.as_ref().unwrap().outcomes;
            (0..outcomes.len())
                .filter(|at| outcomes[*at].provider == "codex")
                .map(move |at| (label.clone(), at))
        })
        .collect();
    let written: std::collections::BTreeSet<(String, usize)> = rows
        .iter()
        .map(|(label, candidate, ..)| (label.clone(), *candidate))
        .collect();
    assert_eq!(written, compiled);

    let words = |program: &str, parts: &[&[&str]]| -> Vec<String> {
        std::iter::once(program)
            .chain(parts.iter().flat_map(|part| part.iter().copied()))
            .map(String::from)
            .collect()
    };
    let lead = ["exec", "--json", "-C", "/w"];
    let pins = [
        "-c",
        "model_reasoning_effort=\"high\"",
        "--model",
        "gpt-6-astra",
    ];
    let work_class = ["--sandbox", "workspace-write"];
    let gate_class = [
        "--sandbox",
        "read-only",
        "--output-last-message",
        "/w/result.json",
    ];
    let resumed = [
        "exec",
        "resume",
        "--json",
        "-c",
        "sandbox_mode=\"workspace-write\"",
    ];
    let at = [THREAD, "-"];
    let dropped = "provider 'codex' cannot express restriction 'allow.hosts'; native capability \
                   remains OFF";
    for (label, candidate, class, seated) in &rows {
        let (label, candidate, class) = (label.as_str(), *candidate, *class);
        let site = bundle.sites[label].capabilities.as_ref().unwrap();
        let outcome = &site.outcomes[candidate];
        let (office, boxed) = match seated {
            Ok(_) => (label, false),
            Err(office) => (*office, *office == "boxed"),
        };
        let cold = |program: &str| match (label, class) {
            ("codex", _) => words(program, &[&lead, &pins, &work_class, &OFF]),
            ("codex-gate" | "boxed-gate", _) => words(program, &[&lead, &pins, &gate_class, &OFF]),
            (_, Work) if boxed => words(program, &[&lead, &pins, &work_class, &OFF]),
            _ => words(program, &[&lead, &pins, &OFF]),
        };
        let offered = match class {
            Gate => None,
            Work if label == "codex" || boxed => Some(words(shim, &[&resumed, &pins, &OFF, &at])),
            Work => Some(cold(shim)),
        };
        let mut record = json!({"held": {}, "not_held": {"web-search": dropped},
            "notices": [format!(
                "seat '{label}' (office '{office}') in realm 'private': dropped wanted capability \
                 'web-search' through dialect 'codex-search-hosts' because {dropped}")],
            "native": {"inventory": "known", "off": ["web-search"], "on": []},
            "provider": "codex"});
        if seated.is_err() {
            record["model"] = json!("astra");
        }
        let seated = match seated {
            Ok((layer, label)) => Ok((*layer, label.as_str())),
            Err(office) => Err(*office),
        };
        let record_of = undigested(&site.manifest()).0;
        let observed = json!({
            "charter": format!("{:?}", charter_of(&bundle, label)),
            "record": record_of["candidates"][candidate],
            "prompt": outcome.prompt(),
            "host": format!("{record_of}{}", outcome.prompt()).contains("a.example"),
            "cold": format!("{:?}", served_as(&bundle, label, candidate, class, None)),
            "offered": (class == Work).then(|| format!("{:?}",
                served_as(&bundle, label, candidate, class, Some((&codex, THREAD))))),
        });
        let expected = json!({
            "charter": format!("{:?}", matrix_charter(root, seated)),
            "record": record,
            "prompt": {"held": {}, "not_held": {"web-search": dropped}},
            "host": false,
            "cold": format!("{:?}", Ok::<_, String>(cold("codex"))),
            "offered": offered.map(|argv| format!("{:?}", Ok::<_, String>(argv))),
        });
        if observed != expected {
            failures.push(format!(
                "{label}[{candidate}]:\n  left:  {observed:#}\n  right: {expected:#}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Rebuild unit 21-fix-b's review return (R1; task 21.3's CQ1 rows; RG4
/// CQ1; NCC; NC6; MPL; design D11): CQ1 AT EVERY BOXED CODEX SITE SHAPE.
/// Under `namespace`, the realm's boxed boundary, every shape ([`one_shape`])
/// — a single work seat, a gate, a panel member and its peer, a sequence
/// step and the next, a select case and the default, and a seat inherited
/// from the recipe's base — is seated with typed workspace hands three ways:
/// inline, through the office `boxed-codex` (Codex alone), and through the
/// office `boxed-fallback`, whose Codex link is the selected FALLBACK of a
/// Claude primary. Each is compiled on the shipped adapters in a realm
/// granting `web-search` through `codex-search-hosts` restricted to
/// `a.example` ([`hosts_grant`]). The rows are every compiled Codex
/// candidate of each compile, no more and no fewer.
///
/// - requires refuses the compile whole, naming the first site the walk
///   resolves and its office; behind the fallback office's Claude primary,
///   the primary's own refusal comes first;
/// - wants drops: each row asserts, beside the site's selected charter, its
///   manifest candidate record (nothing held, the want's exact not-held
///   reason and its one notice), its prompt, and whole commands carrying the
///   measured OFF beside the box's hands — cold, and at a work site the
///   offered session under the assessment the bundle compiled;
/// - a grant no site asks for is unused: the same rows with the unused
///   reason and no notice.
///
/// The shipped Codex rejoin names only the `harness` and `not applicable`
/// boundaries with no hands, so every boxed offer is declined and served
/// its cold command: the replacement is the applicable offered-session row,
/// and no actual boxed Codex rejoin exists to serve. A gate is offered no
/// session. The grant is pinned as written and no site's record or prompt
/// carries its host.
#[cfg(unix)]
#[test]
fn a_restricted_grant_reaches_only_cq1s_outcomes_at_every_boxed_codex_site_shape() {
    use brokkr_runtime::SeatClass::{Gate, Work};
    let operator = Operator::new();
    let root = operator.root();
    let codex = codex_reporting(root, "0.154.0");
    let shim = codex.to_str().unwrap();
    let (grant, restricted) = hosts_grant(&operator);
    let shipped = workspace().join("adapters");
    let hands = json!({"kind": "workspace", "network": false, "binds": []});
    // The office `office` with `models`, boxed, asking for `asks`.
    let office_asks = |office: &str, models: &[&str], asks: Option<&Value>| {
        std::fs::write(
            root.join(format!("agents/charters/{office}.md")),
            format!("# {office}\n"),
        )
        .unwrap();
        let efforts: serde_json::Map<String, Value> = models
            .iter()
            .map(|model| (model.to_string(), json!("high")))
            .collect();
        let mut agent = json!({"description": "an office",
            "charter": format!("charters/{office}.md"), "models": models, "efforts": efforts,
            "hands": hands});
        if let Some(asks) = asks {
            agent["capabilities"] = asks.clone();
        }
        write(root, &format!("agents/{office}.json"), &agent);
    };

    let words = |program: &str, parts: &[&[&str]]| -> Vec<String> {
        std::iter::once(program)
            .chain(parts.iter().flat_map(|part| part.iter().copied()))
            .map(String::from)
            .collect()
    };
    let lead = ["exec", "--json", "-C", "/w"];
    let pins = [
        "-c",
        "model_reasoning_effort=\"high\"",
        "--model",
        "gpt-6-astra",
    ];
    // The box's hands as independent literals, never production's own
    // expansion (NC6): only the test's executable, which the canonical
    // fixture value `{brokkr}` binds, is substituted. Boxed, a gate and a
    // work seat are one command: the class is `read-only` and the result
    // comes back through the hands, not a last message.
    let exe = std::env::current_exe().unwrap();
    let exe = exe.to_str().unwrap();
    let command = format!("mcp_servers.brokkr.command=\"{exe}\"");
    let args = format!(
        "mcp_servers.brokkr.args={}",
        r#"["hands","serve","--workdir","/w","--spec","{\"binds\":[],\"kind\":\"workspace\",\"network\":false}"]"#
    );
    let boxed = [
        "--sandbox",
        "read-only",
        "-c",
        &command,
        "-c",
        &args,
        "-c",
        "mcp_servers.brokkr.default_tools_approval_mode=\"approve\"",
    ];
    let cold = |program: &str| words(program, &[&lead, &pins, &boxed, &OFF]);
    let inexpressible = "provider 'codex' cannot express restriction 'allow.hosts'";
    let dropped = format!("{inexpressible}; native capability remains OFF");
    let unasked = "provider 'codex' has it natively, granted, but this seat does not request \
                   it, and it is switched off";
    let shapes: [(&str, &[&str]); 6] = [
        ("single", &["x"]),
        ("gate", &["x"]),
        ("panel", &["x:member", "x:peer"]),
        ("sequence", &["x:step", "x:next"]),
        ("select", &["x:engine", "x:default"]),
        ("inherited", &["x"]),
    ];
    let mut failures = Vec::new();
    for case in ["requires", "wants", "unused"] {
        let asks = match case {
            "unused" => None,
            strength => Some(json!({"web-search": strength})),
        };
        for (carrier, candidate) in [("inline", 0), ("boxed-codex", 0), ("boxed-fallback", 1)] {
            let site = match carrier {
                "inline" => {
                    let mut site = json!({"role": "roles/x.md", "hands": hands,
                        "driver": {"command": ["{brokkr}", "driver", "codex", "--", "--model",
                                               "gpt-6-astra", "--effort", "high"]}});
                    if let Some(asks) = &asks {
                        site["capabilities"] = asks.clone();
                    }
                    site
                }
                "boxed-codex" => {
                    office_asks(carrier, &["astra"], asks.as_ref());
                    json!({"agent": carrier})
                }
                _ => {
                    office_asks(carrier, &["opus", "astra"], asks.as_ref());
                    json!({"agent": carrier})
                }
            };
            for (shape, labels) in shapes {
                let compiled = one_shape(
                    &operator,
                    &shipped,
                    Boundary::Namespace,
                    &restricted,
                    shape,
                    &site,
                );
                let office_of = |label: &str| match carrier {
                    "inline" => label.to_string(),
                    office => office.to_string(),
                };
                if case == "requires" {
                    // Behind a Claude primary, the primary refuses first:
                    // no Claude link carries a Codex binding.
                    let but = match carrier {
                        "boxed-fallback" => {
                            "provider 'claude' cannot carry a binding to provider 'codex'"
                        }
                        _ => inexpressible,
                    };
                    let refused = format!(
                        "bundle: bundle: seat '{}' (office '{}') in realm 'private': requires \
                         capability 'web-search' through dialect 'codex-search-hosts', but \
                         {but}; the capability cannot be held under this grant (composed: \
                         shape -> shape-base)",
                        labels[0],
                        office_of(labels[0])
                    );
                    let observed = compiled.map(|bundle| bundle.sites.len());
                    if observed != Err(refused.clone()) {
                        failures.push(format!(
                            "requires, {shape} through {carrier}:\n  left:  {observed:?}\n  \
                             right: {:?}",
                            Err::<usize, _>(refused)
                        ));
                    }
                    continue;
                }
                let bundle = compiled.unwrap_or_else(|refusal| {
                    panic!("{case}, {shape} through {carrier} compiles: {refusal}")
                });
                let pinned = &bundle.manifest["capabilities"]["grants"]["web-search"];
                if *pinned != grant {
                    failures.push(format!(
                        "{case}, {shape} through {carrier}, the pinned grant:\n  left:  \
                         {pinned}\n  right: {grant}"
                    ));
                }
                // The rows are every compiled Codex candidate, and nothing else.
                let compiled: std::collections::BTreeSet<(String, usize)> = bundle
                    .sites
                    .iter()
                    .flat_map(|(label, facts)| {
                        let outcomes = &facts.capabilities.as_ref().unwrap().outcomes;
                        (0..outcomes.len())
                            .filter(|at| outcomes[*at].provider == "codex")
                            .map(move |at| (label.clone(), at))
                    })
                    .collect();
                let written: std::collections::BTreeSet<(String, usize)> = labels
                    .iter()
                    .map(|label| (label.to_string(), candidate))
                    .collect();
                if written != compiled {
                    failures.push(format!(
                        "{case}, {shape} through {carrier}, the rows:\n  left:  {compiled:?}\n  \
                         right: {written:?}"
                    ));
                    continue;
                }
                let class = match shape {
                    "gate" => Gate,
                    _ => Work,
                };
                let seated = match carrier {
                    "inline" if shape == "inherited" => Ok(("shape-base", "x")),
                    "inline" => Ok(("shape", "x")),
                    office => Err(office),
                };
                for label in labels.iter().copied() {
                    let site = bundle.sites[label].capabilities.as_ref().unwrap();
                    let outcome = &site.outcomes[candidate];
                    let (reason, notices) = match case {
                        "wants" => (
                            dropped.as_str(),
                            json!([format!(
                                "seat '{label}' (office '{}') in realm 'private': dropped wanted \
                                 capability 'web-search' through dialect 'codex-search-hosts' \
                                 because {dropped}",
                                office_of(label)
                            )]),
                        ),
                        _ => (unasked, json!([])),
                    };
                    let mut record = json!({"held": {}, "not_held": {"web-search": reason},
                        "notices": notices,
                        "native": {"inventory": "known", "off": ["web-search"], "on": []},
                        "provider": "codex"});
                    if carrier != "inline" {
                        record["model"] = json!("astra");
                    }
                    let record_of = undigested(&site.manifest()).0;
                    let observed = json!({
                        "charter": format!("{:?}", charter_of(&bundle, label)),
                        "record": record_of["candidates"][candidate],
                        "prompt": outcome.prompt(),
                        "host": format!("{record_of}{}", outcome.prompt()).contains("a.example"),
                        "cold": format!("{:?}", served_as(&bundle, label, candidate, class, None)),
                        "offered": (class == Work).then(|| format!("{:?}",
                            served_as(&bundle, label, candidate, class, Some((&codex, THREAD))))),
                    });
                    let expected = json!({
                        "charter": format!("{:?}", matrix_charter(root, seated)),
                        "record": record,
                        "prompt": {"held": {}, "not_held": {"web-search": reason}},
                        "host": false,
                        "cold": format!("{:?}", Ok::<_, String>(cold("codex"))),
                        "offered": (class == Work)
                            .then(|| format!("{:?}", Ok::<_, String>(cold(shim)))),
                    });
                    if observed != expected {
                        failures.push(format!(
                            "{case}, {shape} through {carrier}, {label}[{candidate}]:\n  left:  \
                             {observed:#}\n  right: {expected:#}"
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
