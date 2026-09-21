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
struct Operator {
    dir: tempfile::TempDir,
}

impl Operator {
    fn new() -> Operator {
        let operator = Operator {
            dir: tempfile::tempdir().unwrap(),
        };
        let root = operator.dir.path();
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
        self.dir.path()
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
        let mut boxed = json!({"results": ["complete"], "role": "roles/role.md",
            "hands": {"kind": "workspace", "network": false, "binds": []},
            "driver": codex(&["--sandbox", "read-only", "-c",
                              "mcp_servers.brokkr.command=\"{brokkr}\""])});
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
    // in the argv exactly as it is at a real spawn.
    let built = match bundle.boundary.is_boxed() {
        true => brokkr_runtime::engine::BuiltBoundary::Namespace,
        false => brokkr_runtime::engine::BuiltBoundary::Harness,
    };
    let argv = brokkr_runtime::engine::compose_site(
        built,
        brokkr_runtime::SeatClass::Work,
        argv,
        bundle.hands.get(label),
        facts.chain.get(candidate),
        Path::new("/w"),
        &[],
        "/w/result.json",
        None,
    )
    .argv;
    let extra = &argv[argv.iter().position(|part| part == "--").unwrap() + 1..];
    let input = json!({"workdir": "/w", "native_controls": outcome.controls()});
    match outcome.provider.as_str() {
        "codex" => brokkr_protocol::adapters::codex_command("codex", extra, "/w", None, &input),
        _ => brokkr_protocol::adapters::claude_command("claude", extra, None, &input),
    }
    .unwrap_or_else(|refusal| panic!("{label}[{candidate}] refused: {refusal}"))
}

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
    let argv = brokkr_runtime::engine::compose_site(
        brokkr_runtime::engine::BuiltBoundary::Harness,
        brokkr_runtime::SeatClass::Work,
        argv,
        bundle.hands.get(label),
        facts.chain.first(),
        Path::new("/w"),
        &[],
        "/w/result.json",
        None,
    )
    .argv;
    let extra = &argv[argv.iter().position(|part| part == "--").unwrap() + 1..];
    let input = json!({
        "workdir": "/w", "seat": label, "boundary": boundary, "hands": "none",
        "resume_context": {"assessment": assessment},
        "native_controls": outcome.controls(),
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
    for (authored, written) in [
        (vec!["--search"], "--search"),
        (vec!["-c", "web_search=\"disabled\""], "-c web_search"),
        (
            vec!["--enable", "web_search_request"],
            "--enable web_search_request",
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
