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
            &workspace().join("adapters"),
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
