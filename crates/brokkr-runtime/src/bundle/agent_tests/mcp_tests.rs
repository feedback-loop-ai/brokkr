//! The MCP server set each serving intends (decision 0065 slice two, U1f;
//! SI2 and MB1), carried beside the serving inputs an inline site's
//! composition keeps: a candidate's from its own composition's typed hands,
//! an inline site's recorded in its facts at its own execution label, and
//! neither from the bytes either emits.

use super::*;
use crate::bundle::mcp::{InlineServing, McpIntent};
use brokkr_protocol::native_controls::HandsIntent;

/// Rebuild unit 14a1: an inline site's composition carries the typed
/// serving inputs the final check rebuilds its command from, each equal to
/// its adapter's declaration where its typed declaration lowered: the
/// permission flag and separator a Claude allow lowered onto, and the
/// `hands.harness` fragment a Codex class lowered onto, as declared, its
/// `{result_path}` unfilled. An inline site has no pins or boundary
/// fragment of its adapter's, and carries the typed hands it resolved;
/// beside them, it carries its adapter's `hands.workspace` fragment as
/// declared through the compile's expansion (operator ruling (B) of
/// 2026-09-27). An agent-backed site has no inline composition.
#[test]
fn an_inline_sites_composition_carries_each_serving_input_as_its_adapter_declares_it() {
    use crate::agents::{DeclaredDialect, ServingInputs};
    use brokkr_protocol::native_controls::ListFlag;
    let fixture = AgentFixture::new();
    let workspace = with_schema(&CODEX_WORKSPACE);
    let mut codex = codex();
    codex["hands"]["workspace"] = json!(workspace);
    fixture.write("adapters/codex.json", codex);
    let serving = |site: &str, command: Value, extra: Value| {
        let mut config = fixture.config();
        config["seats"]["review"]["driver"]["command"] = command;
        for (key, value) in extra.as_object().unwrap() {
            config["seats"]["review"][key] = value.clone();
        }
        match fixture.compile(config) {
            Ok(bundle) => format!("{:?}", bundle.sites[site].inline_serving()),
            Err(error) => error.to_string(),
        }
    };
    let dialect = |permissions: Option<(&str, &str)>, sandbox: &[&str]| DeclaredDialect {
        permissions: permissions.map(|(flag, separator)| ListFlag {
            flag: flag.to_string(),
            separator: separator.to_string(),
        }),
        sandbox: sandbox.iter().map(|part| part.to_string()).collect(),
        ..DeclaredDialect::default()
    };
    let carried = |dialect: DeclaredDialect, spec: Option<HandsSpec>| {
        format!(
            "{:?}",
            Some(ServingInputs {
                dialect,
                pins: Vec::new(),
                spec,
            })
        )
    };
    let hands = json!({"kind": "workspace", "network": false, "binds": []});
    let exec = json!(["{brokkr}", "driver", "exec", "--", "bash", "x.sh"]);
    let rows: Vec<Row<String>> = vec![
        (
            "claude allow".into(),
            serving(
                "review",
                claude_inline("claude", &[]),
                json!({"tools": {"allow": ["cargo"]}}),
            ),
            carried(dialect(Some(("--allowedTools", ",")), &[]), None),
        ),
        (
            "codex gate, read-only".into(),
            serving(
                "review",
                codex_inline(&[]),
                json!({"class": "gate", "tools": {"sandbox": "read-only"}}),
            ),
            carried(dialect(None, &CODEX_GATE), None),
        ),
        (
            "codex work, workspace-write".into(),
            serving(
                "review",
                codex_inline(&[]),
                json!({"tools": {"sandbox": "workspace-write"}}),
            ),
            carried(dialect(None, &CODEX_WORK), None),
        ),
        (
            "claude, nothing lowered".into(),
            serving("review", claude_inline("claude", &[]), json!({})),
            carried(dialect(None, &[]), None),
        ),
        (
            "exec with hands".into(),
            serving("review", exec, json!({"hands": hands})),
            carried(dialect(None, &[]), Some(HandsSpec::parse(&hands).unwrap())),
        ),
        (
            "codex with boxed hands".into(),
            serving("review", codex_inline(&[]), json!({"hands": hands})),
            carried(
                DeclaredDialect {
                    hands: workspace.clone(),
                    ..DeclaredDialect::default()
                },
                Some(HandsSpec::parse(&hands).unwrap()),
            ),
        ),
        (
            "codex without hands".into(),
            serving("review", codex_inline(&[]), json!({})),
            carried(dialect(None, &[]), None),
        ),
        (
            "an agent-backed site".into(),
            serving("work", claude_inline("claude", &[]), json!({})),
            format!("{:?}", None::<ServingInputs>),
        ),
    ];
    each_row(rows);
}

/// One execution site's intended sets: its label, an inline site's own
/// fact, and an agent-backed site's provider and intent per candidate in
/// chain order.
type Intents = (String, Option<McpIntent>, Vec<(String, Option<McpIntent>)>);

/// Every executable site's intended sets, by execution label.
fn intents(bundle: &Bundle) -> Vec<Intents> {
    bundle
        .sites
        .iter()
        .filter(|(_, facts)| facts.capabilities.is_some())
        .map(|(label, facts)| {
            let chain = facts.chain.iter();
            let chain = chain.map(|link| (link.provider.clone(), McpIntent::of_candidate(link)));
            (label.clone(), facts.inline_mcp, chain.collect())
        })
        .collect()
}

/// An inline site's expected row, or with `chain`, an agent-backed one's.
fn site(label: &str, inline: Option<McpIntent>, chain: &[(&str, McpIntent)]) -> Intents {
    let chain = chain.iter();
    let chain = chain.map(|(provider, intent)| (provider.to_string(), Some(*intent)));
    (label.to_string(), inline, chain.collect())
}

/// Fixture Claude with a boxed hands fragment, beside the fixture Codex, and
/// three offices: `boxed` on opus then astra with hands, `bare` on the same
/// chain without, and `harnessed`, boxed on astra alone.
fn two_providers(fixture: &AgentFixture) {
    let mut claude = claude();
    claude["hands"] =
        json!({"workspace": ["--strict-mcp-config", "--mcp-config", "{hands_mcp_json}"]});
    fixture.write("adapters/claude.json", claude);
    fixture.write("adapters/codex.json", codex());
    fixture.write(
        "agents/boxed.json",
        boxed_agent(&["opus", "astra"], json!({})),
    );
    fixture.write("agents/harnessed.json", boxed_agent(&["astra"], json!({})));
    let mut bare = boxed_agent(&["opus", "astra"], json!({}));
    bare.as_object_mut().unwrap().remove("hands");
    fixture.write("agents/bare.json", bare);
}

/// SI2 and MB1 at each candidate: primary and fallback each intend the set
/// their own composition recorded from its typed hands and boundary, on
/// their own provider and in their unchanged order. A hands-less office
/// intends the explicitly empty set on both links. Under the `harness`
/// boundary the harness's own sandbox carries the office's hands, so no
/// hands server is launched and the set is strictly empty, while the typed
/// hands stay required.
#[test]
fn each_candidate_intends_its_own_set_from_its_typed_hands_never_its_emitted_fragment() {
    let fixture = AgentFixture::new();
    two_providers(&fixture);
    let chain = |agent: &str, boundary: Boundary| {
        let mut config = fixture.config();
        config["seats"]["work"]["agent"] = json!(agent);
        match fixture.compile_under(config, boundary) {
            Ok(bundle) => format!(
                "{:?}",
                bundle.sites["work"]
                    .chain
                    .iter()
                    .map(|link| (
                        link.provider.as_str(),
                        McpIntent::of_candidate(link),
                        hands(link),
                        link.hands_fragment.len()
                    ))
                    .collect::<Vec<_>>()
            ),
            Err(error) => error.to_string(),
        }
    };
    let expected = |links: &[(&str, McpIntent, HandsIntent, usize)]| {
        format!(
            "{:?}",
            links
                .iter()
                .map(|(provider, intent, hands, emitted)| {
                    (*provider, Some(*intent), Some(*hands), *emitted)
                })
                .collect::<Vec<_>>()
        )
    };
    use HandsIntent::{None as Bare, Required};
    each_row(vec![
        (
            "boxed, namespace".into(),
            chain("boxed", Boundary::Namespace),
            expected(&[
                ("claude", McpIntent::Hands, Required, 3),
                ("codex", McpIntent::Hands, Required, 8),
            ]),
        ),
        (
            "bare, namespace".into(),
            chain("bare", Boundary::Namespace),
            expected(&[
                ("claude", McpIntent::Empty, Bare, 0),
                ("codex", McpIntent::Empty, Bare, 0),
            ]),
        ),
        (
            "harnessed, harness".into(),
            chain("harnessed", Boundary::Harness),
            expected(&[("codex", McpIntent::Empty, Required, 0)]),
        ),
    ]);
}

/// The typed hands a candidate's composition keeps, `None` where it has
/// none.
fn hands(link: &Candidate) -> Option<HandsIntent> {
    match &link.lowering {
        Lowering::Composed(composition) => Some(composition.intent.hands),
        Lowering::Refused(_) | Lowering::Unavailable => None,
    }
}

/// Each boundary decides whether an office's required hands are served by
/// the engine's hands server: only the three that box them do, and under
/// `harness` (the harness's own sandbox) or `open` (nothing) the set is
/// strictly empty. No hands intend the empty set under every boundary.
#[test]
fn only_a_boxing_boundary_turns_required_hands_into_the_hands_server() {
    let rows: Vec<Row<String>> = [
        (Boundary::Open, McpIntent::Empty),
        (Boundary::Harness, McpIntent::Empty),
        (Boundary::Namespace, McpIntent::Hands),
        (Boundary::Seatbelt, McpIntent::Hands),
        (Boundary::Container, McpIntent::Hands),
    ]
    .into_iter()
    .map(|(boundary, required)| {
        let set = |hands| McpIntent::composed(hands, boundary);
        (
            boundary.to_string(),
            format!("{:?}", (set(HandsIntent::Required), set(HandsIntent::None))),
            format!("{:?}", (required, McpIntent::Empty)),
        )
    })
    .collect();
    each_row(rows);
}

/// Equality of bytes never supplies the hands origin (MB1): a fallback whose
/// own composition records the empty set types none of its fragment as the
/// box's hands, although its fragment is byte for byte its primary's, and
/// the primary keeps its own count. At an inline site only an intended hands
/// server, or exec's surface-less box, types the fragment; an empty or
/// unrecorded intent types none of it.
#[test]
fn a_fallback_types_its_own_hands_and_equal_bytes_never_supply_their_origin() {
    use crate::capabilities::{Authority, CapabilityContext, SiteAsks};
    let fixture = AgentFixture::new();
    two_providers(&fixture);
    let mut codex = codex();
    codex["models"]["sol"] = json!("gpt-6-sol");
    fixture.write("adapters/codex.json", codex);
    fixture.write(
        "agents/boxed.json",
        boxed_agent(&["astra", "sol"], json!({})),
    );
    let mut config = sandbox_seat(&fixture, None);
    config["seats"]["review"]["driver"]["command"] = codex_inline(&[]);
    config["seats"]["review"]["hands"] =
        json!({"kind": "workspace", "network": false, "binds": []});
    let bundle = fixture.compile(config).unwrap();
    let mut chain = bundle.sites["work"].chain.clone();
    let Lowering::Composed(fallback) = &mut chain[1].lowering else {
        panic!("the fallback composes")
    };
    fallback.mcp = McpIntent::Empty;
    assert_eq!(chain[1].hands_fragment, chain[0].hands_fragment);
    let adapters = Adapters::load(&fixture.adapters()).unwrap();
    let adapters = CapabilityAdapters::loaded(&adapters);
    let authority =
        Authority::load(CapabilityContext::no_grants("private", &fixture.root)).unwrap();
    let asks = || SiteAsks::at(SeatClass::Work, "work", None, None).unwrap();
    let typed = |site: crate::capabilities::SiteCapabilities| -> Vec<Value> {
        let outcomes = site.outcomes.iter();
        outcomes
            .map(|outcome| outcome.controls()["hands"].clone())
            .collect()
    };
    let candidates = mcp::candidate_capabilities(&authority, adapters, asks(), &chain, None);
    assert_eq!(typed(candidates.unwrap()), vec![json!(8), json!(0)]);
    let argv: Vec<String> = serde_json::from_value(codex_inline(&[])).unwrap();
    let hands = bundle.sites["review"].inline_hands.clone().unwrap().argv;
    let inline = |intent: Option<McpIntent>| {
        let serving = InlineServing {
            driver: Some("codex"),
            argv: &argv,
            local: &[],
            hands: &hands,
            intent,
        };
        typed(mcp::inline_capabilities(&authority, adapters, asks(), &serving).unwrap())
    };
    let rows: Vec<Row<Vec<Value>>> = [
        (Some(McpIntent::Hands), 8),
        (Some(McpIntent::NoModelSurface), 8),
        (Some(McpIntent::Empty), 0),
        (None, 0),
    ]
    .into_iter()
    .map(|(intent, count)| (format!("{intent:?}"), inline(intent), vec![json!(count)]))
    .collect();
    each_row(rows);
}

/// SI2 at every execution site the capability walk visits: each inline site
/// records its own set at its own label, a panel member beside an agent in
/// the same step, a sequence step, a select case, the inherited default and
/// the case a later layer wrote, while an agent-backed site records none
/// and its candidates carry theirs. A site with no hands intends the empty
/// set, and exec's step has no model surface.
#[test]
fn each_execution_site_records_its_own_intended_set_at_its_own_label() {
    let fixture = AgentFixture::new();
    two_providers(&fixture);
    let hands = json!({"kind": "workspace", "network": false, "binds": []});
    let inline = |command: Value, hands: Option<&Value>| {
        let mut site = json!({"role": "roles/work.md", "driver": {"command": command}});
        if let Some(hands) = hands {
            site["hands"] = hands.clone();
        }
        site
    };
    let exec = json!(["{brokkr}", "driver", "exec", "--", "bash", "x.sh"]);
    let base = fixture.root.join("base");
    std::fs::create_dir_all(base.join("roles")).unwrap();
    let mut layer = fixture.config();
    layer["name"] = json!("base");
    layer["seats"]["work"] = json!({"results": ["complete"], "sequence": [
        {"name": "first", "aggregate": "unanimous-pass", "panel": {
            "a": {"agent": "boxed"},
            "b": inline(codex_inline(&[]), Some(&hands)),
        }},
        {"name": "second", "role": "roles/work.md", "driver": {"command": exec}},
    ]});
    layer["seats"]["review"] = json!({"results": ["clean"], "select": {
        "on": "strategy",
        "cases": {"feature": inline(codex_inline(&[]), Some(&hands)), "chore": {"agent": "bare"}},
        "default": inline(exec.clone(), Some(&hands)),
    }});
    for (file, body) in [
        ("roles/work.md", json!("# work\n")),
        ("policy.json", policy()),
        ("bundle.json", layer),
    ] {
        let bytes = match body {
            Value::String(text) => text.into_bytes(),
            other => serde_json::to_vec(&other).unwrap(),
        };
        std::fs::write(base.join(file), bytes).unwrap();
    }
    let leaf = json!({
        "name": "fixture", "extends": "base",
        "override": {"cases": ["review:chore"]},
        "seats": {
            "review": {"select": {"cases": {"chore": inline(claude_inline("claude", &[]), None)}}},
        },
    });
    fixture.stage(&leaf, &policy());
    let bundle =
        Bundle::compile_with(&fixture.bundle(), &fixture.library(), &fixture.adapters()).unwrap();
    use McpIntent::{Empty, Hands, NoModelSurface};
    assert_eq!(
        intents(&bundle),
        vec![
            site("review:chore", Some(Empty), &[]),
            site("review:default", Some(NoModelSurface), &[]),
            site("review:feature", Some(Hands), &[]),
            site("work:first:a", None, &[("claude", Hands), ("codex", Hands)]),
            site("work:first:b", Some(Hands), &[]),
            site("work:second", Some(NoModelSurface), &[]),
        ]
    );
}

/// A dialect step runs exec and has no model MCP surface; the verify seat a
/// dialect wraps keeps its candidates' intent at `verify:checks`, and every
/// inline site beside it records its own empty set.
#[test]
fn a_dialect_step_has_no_model_surface_and_a_relocated_verify_keeps_its_intent() {
    let fixture = AgentFixture::declaring();
    gate_tests::hire_judge(&fixture, "wants", &["astra"]);
    let bundle = gate_tests::wrapped_verify(&fixture).unwrap();
    use McpIntent::{Empty, NoModelSurface};
    assert_eq!(
        intents(&bundle),
        vec![
            site("design:draft", Some(Empty), &[]),
            site("design:validate", Some(NoModelSurface), &[]),
            site("review", Some(Empty), &[]),
            site("verify:checks", None, &[("codex", Empty)]),
            site("verify:dialect-verify", Some(NoModelSurface), &[]),
        ]
    );
}

/// Slice one's authored-option refusal stands at an inline site whose
/// authored bytes equal an engine fragment: the empty set's strict flag and
/// document after a Claude command, and the hands server's table after a
/// Codex command. Equal bytes are authored, never the engine's, and the
/// refusal is capability authority's, not merely its text.
#[test]
fn authored_bytes_equal_to_an_engine_fragment_stay_authored_and_refused() {
    let fixture = AgentFixture::new();
    two_providers(&fixture);
    let refusal = |command: Value| {
        let mut config = fixture.config();
        config["seats"]["review"]["driver"]["command"] = command;
        match fixture.compile(config) {
            Err(error @ CompileError::Capability(_)) => error.to_string(),
            other => format!("not a capability refusal: {}", outcome(other)),
        }
    };
    let authored = |option: &str, harness: &str| {
        format!(
            "bundle: seat 'review' (office 'review') in realm '<unmapped>': its arguments carry \
             '{option}' (argument 5), a capability-bearing option of harness '{harness}'. A recipe \
             authors no capability-bearing option, whatever its value, polarity or grant: tools \
             come from typed declarations and the realm's grant, composed by the engine alone \
             (operator ruling 1 of 2026-09-23)"
        )
    };
    each_row(vec![
        (
            "claude empty set".into(),
            refusal(claude_inline(
                "claude",
                &[
                    "--strict-mcp-config",
                    "--mcp-config",
                    r#"{"mcpServers":{}}"#,
                ],
            )),
            authored("--strict-mcp-config", "claude"),
        ),
        (
            "codex hands table".into(),
            refusal(codex_inline(&CODEX_WORKSPACE[2..])),
            authored("--config", "codex"),
        ),
    ]);
}

/// A candidate that composed nothing intends no set: neither a refused
/// entry nor a model no adapter maps yields one, so dispatch never reads
/// an intent this pass did not compose.
#[test]
fn a_candidate_that_composed_nothing_intends_no_set() {
    let fixture = AgentFixture::new();
    two_providers(&fixture);
    let bundle = fixture.compile(sandbox_seat(&fixture, None)).unwrap();
    let mut link = bundle.sites["work"].chain[0].clone();
    let Lowering::Composed(composition) = &link.lowering else {
        panic!("the primary composes")
    };
    let (composed, intent) = (composition.mcp, composition.intent.clone());
    assert_eq!(McpIntent::of_candidate(&link), Some(composed));
    link.lowering = Lowering::Refused(intent);
    assert_eq!(McpIntent::of_candidate(&link), None);
    link.lowering = Lowering::Unavailable;
    assert_eq!(McpIntent::of_candidate(&link), None);
}
