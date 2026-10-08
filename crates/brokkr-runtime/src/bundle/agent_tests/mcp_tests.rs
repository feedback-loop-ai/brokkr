//! The MCP server set each serving intends (decision 0065 slice two, U1f;
//! SI2 and MB1), carried beside the serving inputs an inline site's
//! composition keeps: a candidate's from its own composition's typed hands,
//! an inline site's recorded in its facts at its own execution label, and
//! neither from the bytes either emits.

use super::*;
use crate::agents::{McpAxis, McpHands, McpHost, McpInvocation, McpShape, McpUnmeasured};
use crate::bundle::mcp::{InlineServing, McpIntent};
use crate::capabilities::McpFence;
use brokkr_protocol::native_controls::HandsIntent;

/// The fixture compiled with its inline `review` seat running `command`,
/// each of `extra`'s keys written on that seat.
fn inline_review(
    fixture: &AgentFixture,
    command: Value,
    extra: Value,
) -> Result<Bundle, CompileError> {
    let mut config = fixture.config();
    config["seats"]["review"]["driver"]["command"] = command;
    for (key, value) in extra.as_object().unwrap() {
        config["seats"]["review"][key] = value.clone();
    }
    fixture.compile(config)
}

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
    let serving =
        |site: &str, command: Value, extra: Value| match inline_review(&fixture, command, extra) {
            Ok(bundle) => format!("{:?}", bundle.sites[site].inline_serving()),
            Err(error) => error.to_string(),
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

/// The host word and host this build serves on (decision 0063), read here
/// independently of the compile pass's own.
fn host() -> (&'static str, McpHost) {
    match cfg!(target_os = "macos") {
        true => ("macos", McpHost::Macos),
        false => ("linux", McpHost::Linux),
    }
}

/// One synthetic shape measured under `harness` on `binary`: plumbing
/// evidence these suites write to drive the compile pass, never U0's
/// qualification of a harness.
fn synthetic_shape(harness: &str, invocation: Value, hands: &str, ambient: Value) -> Value {
    let unprobed = json!({"unmeasured": "synthetic fixture: not probed"});
    json!({
        "invocation": invocation, "hands": hands,
        "measured_on": {"harness": harness, "binary": harness, "version": "synthetic", "host": host().0},
        "ambient": ambient, "native_write": unprobed, "store_read": unprobed, "process_read": unprobed,
    })
}

/// A resume shape declared under `hands` and never measured.
fn declared_resume(hands: &str) -> Value {
    json!({
        "status": "unmeasured", "identity": {"unknown": "synthetic fixture"}, "classes": ["work"],
        "boundaries": ["namespace", "not applicable"], "hands": hands, "evidence": {},
        "limitations": [],
    })
}

/// The fixture Claude with the box's hands and typed MCP facts, `shapes`,
/// declaring two resume shapes: `rejoin` without hands, `boxed-rejoin` boxed.
fn strict_claude(shapes: Vec<Value>) -> Value {
    let mut claude = claude();
    claude["hands"] =
        json!({"workspace": ["--strict-mcp-config", "--mcp-config", "{hands_mcp_json}"]});
    claude["mcp"] = json!({"carriage": {"measured": "synthetic carriage"}, "shapes": shapes});
    claude["resume"] =
        json!({"rejoin": declared_resume("none"), "boxed-rejoin": declared_resume("boxed")});
    claude
}

/// The fixture Claude's shapes: hands-less cold, replacement and `rejoin`
/// as given, and the box's cold measured unsupported.
fn claude_shapes(cold: Value, replacement: Value, rejoin: Value) -> Vec<Value> {
    let unsupported = json!({"unsupported": "synthetic: the box's exclusion fails"});
    vec![
        synthetic_shape("claude", json!("cold"), "none", cold),
        synthetic_shape("claude", json!("cold"), "boxed", unsupported),
        synthetic_shape("claude", json!("replacement"), "none", replacement),
        synthetic_shape("claude", json!({"resume": "rejoin"}), "none", rejoin),
    ]
}

/// One recorded shape on this host, with its ambient exclusion.
fn entry(invocation: McpInvocation, hands: McpHands, ambient: McpAxis) -> (McpShape, McpAxis) {
    let host = host().1;
    let shape = McpShape {
        invocation,
        hands,
        host,
    };
    (shape, ambient)
}

fn measured(evidence: &str) -> McpAxis {
    McpAxis::Measured {
        evidence: evidence.to_string(),
    }
}

const LEGACY: McpAxis = McpAxis::Unmeasured(McpUnmeasured::Legacy);
const ABSENT: McpAxis = McpAxis::Unmeasured(McpUnmeasured::Absent);

/// A site's SI2 records, one per outcome, as the compile kept them.
fn records(bundle: &Bundle, site: &str) -> String {
    let site = bundle.sites[site].capabilities.as_ref().unwrap();
    assert_eq!(
        site.strict.len(),
        site.outcomes.len(),
        "one record per outcome"
    );
    format!("{:?}", site.strict)
}

/// SI2 at each candidate (U1g1): primary and fallback each record, per
/// shape they can be served as, the ambient exclusion their own adapter
/// measures there. The hands are the intended set's: the box's where it
/// holds the hands server, the harness's own sandbox under `harness`, and
/// none otherwise. A work site is judged cold, as its replacement and as
/// each resume shape declared under the same hands. A declared-unmeasured,
/// absent or legacy fact is recorded as what it is, never a measurement.
#[test]
fn each_candidate_records_its_own_adapters_support_per_shape_and_hands() {
    let fixture = AgentFixture::new();
    two_providers(&fixture);
    let shapes = claude_shapes(
        json!({"measured": "synthetic cold"}),
        json!({"unmeasured": "synthetic: replacement not probed"}),
        json!({"measured": "synthetic rejoin"}),
    );
    fixture.write("adapters/claude.json", strict_claude(shapes));
    let record = |agent: &str, boundary: Boundary| {
        let mut config = fixture.config();
        config["seats"]["work"]["agent"] = json!(agent);
        match fixture.compile_under(config, boundary) {
            Ok(bundle) => records(&bundle, "work"),
            Err(error) => error.to_string(),
        }
    };
    use McpHands::{Boxed, Harness, NoHands};
    use McpInvocation::{Cold, Replacement, Resume};
    let unprobed = McpUnmeasured::Declared("synthetic: replacement not probed".into());
    let unsupported = McpAxis::Unsupported {
        reason: "synthetic: the box's exclusion fails".into(),
    };
    each_row(vec![
        (
            "bare, namespace".into(),
            record("bare", Boundary::Namespace),
            format!(
                "{:?}",
                [
                    vec![
                        entry(Cold, NoHands, measured("synthetic cold")),
                        entry(Replacement, NoHands, McpAxis::Unmeasured(unprobed)),
                        entry(
                            Resume("rejoin".into()),
                            NoHands,
                            measured("synthetic rejoin")
                        ),
                    ],
                    vec![
                        entry(Cold, NoHands, LEGACY),
                        entry(Replacement, NoHands, LEGACY)
                    ],
                ]
            ),
        ),
        (
            "boxed, namespace".into(),
            record("boxed", Boundary::Namespace),
            format!(
                "{:?}",
                [
                    vec![
                        entry(Cold, Boxed, unsupported),
                        entry(Replacement, Boxed, ABSENT),
                        entry(Resume("boxed-rejoin".into()), Boxed, ABSENT),
                    ],
                    vec![
                        entry(Cold, Boxed, LEGACY),
                        entry(Replacement, Boxed, LEGACY)
                    ],
                ]
            ),
        ),
        (
            "harnessed, harness".into(),
            record("harnessed", Boundary::Harness),
            format!(
                "{:?}",
                [vec![
                    entry(Cold, Harness, LEGACY),
                    entry(Replacement, Harness, LEGACY)
                ]]
            ),
        ),
    ]);
}

/// SI2 at an inline site, by its class and driver: its record is its own
/// driver's adapter's, and a gate, never offered a session, is judged cold
/// alone. Exec serves no model and records no shape, while an opaque
/// command no adapter answers for records every shape absent. A candidate
/// that composed nothing launches nothing and records no shape either.
#[test]
fn each_inline_site_records_its_drivers_support_and_a_gate_is_judged_cold_alone() {
    let fixture = AgentFixture::new();
    two_providers(&fixture);
    let shapes = claude_shapes(
        json!({"measured": "synthetic cold"}),
        json!({"measured": "synthetic replacement"}),
        json!({"measured": "synthetic rejoin"}),
    );
    fixture.write("adapters/claude.json", strict_claude(shapes));
    let record = |command: Value, extra: Value| match inline_review(&fixture, command, extra) {
        Ok(bundle) => records(&bundle, "review"),
        Err(error) => error.to_string(),
    };
    use McpHands::{Boxed, NoHands};
    use McpInvocation::{Cold, Replacement, Resume};
    let hands = json!({"hands": {"kind": "workspace", "network": false, "binds": []}});
    let exec = json!(["{brokkr}", "driver", "exec", "--", "bash", "x.sh"]);
    let expected = |record: Vec<(McpShape, McpAxis)>| format!("{:?}", [record]);
    each_row(vec![
        (
            "claude, work".into(),
            record(claude_inline("claude", &[]), json!({})),
            expected(vec![
                entry(Cold, NoHands, measured("synthetic cold")),
                entry(Replacement, NoHands, measured("synthetic replacement")),
                entry(
                    Resume("rejoin".into()),
                    NoHands,
                    measured("synthetic rejoin"),
                ),
            ]),
        ),
        (
            "codex gate".into(),
            record(codex_inline(&[]), json!({"class": "gate"})),
            expected(vec![entry(Cold, NoHands, LEGACY)]),
        ),
        (
            "codex, boxed hands".into(),
            record(codex_inline(&[]), hands.clone()),
            expected(vec![
                entry(Cold, Boxed, LEGACY),
                entry(Replacement, Boxed, LEGACY),
            ]),
        ),
        ("exec".into(), record(exec, hands), expected(Vec::new())),
        (
            "opaque".into(),
            record(json!(["driver"]), json!({})),
            expected(vec![
                entry(Cold, NoHands, ABSENT),
                entry(Replacement, NoHands, ABSENT),
            ]),
        ),
    ]);
    let bundle = fixture.compile(sandbox_seat(&fixture, None)).unwrap();
    let mut chain = bundle.sites["work"].chain.clone();
    chain[1].lowering = Lowering::Unavailable;
    let adapters = Adapters::load(&fixture.adapters()).unwrap();
    let authority = crate::capabilities::Authority::load(
        crate::capabilities::CapabilityContext::no_grants("private", &fixture.root),
    )
    .unwrap();
    let asks = crate::capabilities::SiteAsks::at(SeatClass::Work, "work", None, None).unwrap();
    let site = mcp::candidate_capabilities(
        &authority,
        CapabilityAdapters::loaded(&adapters),
        asks,
        &chain,
        None,
    )
    .unwrap();
    assert_eq!(
        site.strict,
        vec![
            vec![
                entry(
                    Cold,
                    Boxed,
                    McpAxis::Unsupported {
                        reason: "synthetic: the box's exclusion fails".into()
                    }
                ),
                entry(Replacement, Boxed, ABSENT),
                entry(Resume("boxed-rejoin".into()), Boxed, ABSENT),
            ],
            Vec::new(),
        ]
    );
}

/// The compile of `config` against the fixture's library, `Ok` as
/// "compiles", with the fence lifted where `lifted`.
fn compiled(fixture: &AgentFixture, config: Value, lifted: bool) -> String {
    let lift = lifted.then(McpFence::lift);
    let compiled = fixture.compile(config);
    drop(lift);
    match compiled {
        Ok(_) => "compiles".to_string(),
        Err(error) => error.to_string(),
    }
}

/// SI2's measured unsupported cause after the bounded site prefix.
fn cannot_exclude(seat: &str, office: &str, provider: &str, reason: &str) -> String {
    format!(
        "bundle: seat '{seat}' (office '{office}') in realm '<unmapped>': provider '{provider}' \
         cannot exclude ambient MCP configuration ({reason})"
    )
}

/// SI2's unmeasured cause after the bounded site prefix.
fn unmeasured(seat: &str, office: &str, provider: &str, shape: &str) -> String {
    format!(
        "bundle: seat '{seat}' (office '{office}') in realm '<unmapped>': provider '{provider}' \
         has no measured strict MCP configuration for '{shape}'"
    )
}

/// Past the lifted fence (a test's override, never a runtime knob), SI2's
/// two exact causes refuse the first serving whose record carries one,
/// whatever it asks: a measured unsupported exclusion with its measured
/// reason, and an absent, declared-unmeasured or legacy shape by its bare
/// invocation word — `cold`, `replacement` or a resume shape's own name. A
/// fallback is judged on its own record. A site measured on every shape it
/// is served as compiles, and with the fence standing every one of them
/// compiles too.
#[test]
fn past_the_lifted_fence_each_si2_cause_refuses_exactly_and_the_standing_fence_compiles() {
    let fixture = AgentFixture::new();
    two_providers(&fixture);
    let compile = |shapes: Vec<Value>, agent: &str, lifted: bool| {
        fixture.write("adapters/claude.json", strict_claude(shapes));
        let mut config = fixture.config();
        config["seats"]["work"]["agent"] = json!(agent);
        config["seats"]["review"]["driver"]["command"] = claude_inline("claude", &[]);
        compiled(&fixture, config, lifted)
    };
    let measured = |text: &str| json!({"measured": text});
    let qualified = || {
        claude_shapes(
            measured("synthetic cold"),
            measured("synthetic replacement"),
            measured("synthetic rejoin"),
        )
    };
    let unsupported = json!({"unsupported": "project MCP configuration cannot be excluded"});
    let mut no_cold = qualified();
    no_cold.remove(0);
    let mut no_rejoin = qualified();
    no_rejoin.pop();
    let rows: Vec<(&str, Vec<Value>, &str, String)> = vec![
        (
            "unsupported cold",
            claude_shapes(unsupported, measured("r"), measured("j")),
            "worker",
            cannot_exclude(
                "review",
                "review",
                "claude",
                "project MCP configuration cannot be excluded",
            ),
        ),
        (
            "absent cold",
            no_cold,
            "worker",
            unmeasured("review", "review", "claude", "cold"),
        ),
        (
            "declared-unmeasured replacement",
            claude_shapes(
                measured("c"),
                json!({"unmeasured": "not probed"}),
                measured("j"),
            ),
            "worker",
            unmeasured("review", "review", "claude", "replacement"),
        ),
        (
            "absent rejoin",
            no_rejoin,
            "worker",
            unmeasured("review", "review", "claude", "rejoin"),
        ),
        (
            "a fallback's legacy cold",
            qualified(),
            "bare",
            unmeasured("work", "bare", "codex", "cold"),
        ),
    ];
    let mut table: Vec<Row<String>> = vec![(
        "measured on every shape, lifted".into(),
        compile(qualified(), "worker", true),
        "compiles".into(),
    )];
    for (label, shapes, agent, refusal) in rows {
        table.push((
            format!("{label}, lifted"),
            compile(shapes.clone(), agent, true),
            refusal,
        ));
        table.push((
            format!("{label}, standing"),
            compile(shapes, agent, false),
            "compiles".into(),
        ));
    }
    each_row(table);
}

/// A fresh dsh scaffold's shape (U1f2's starter), compiled whole in-crate
/// against the shipped `dsh`, `claude` and `exec` adapters init copies its
/// MCP facts from: intake and implement hired from dsh, the review gate from
/// claude, and verify and ship exec scripts with boxed hands. `dsh` and
/// `claude` stand in for the shipped MCP facts where given.
fn dsh_scaffold(fixture: &AgentFixture, dsh: Option<Value>, claude: Option<Value>) -> Value {
    let shipped = |name: &str| -> Value {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../adapters/{name}.json"));
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    };
    for (name, mcp) in [("dsh", dsh), ("claude", claude), ("exec", None)] {
        let mut adapter = shipped(name);
        if let Some(mcp) = mcp {
            adapter["mcp"] = mcp;
        }
        fixture.write(&format!("adapters/{name}.json"), adapter);
    }
    let agent = |models: [&str; 2]| {
        json!({
            "description": "a scaffolded office", "charter": "charters/work.md", "models": models,
            "efforts": {models[0]: "high", models[1]: "high"},
        })
    };
    fixture.write("agents/intake.json", agent(["flash", "pro"]));
    fixture.write("agents/implementer.json", agent(["pro", "flash"]));
    fixture.write("agents/reviewer.json", agent(["fable", "opus"]));
    let hands = json!({"kind": "workspace", "network": false, "binds": []});
    let script = |name: &str| json!(["{brokkr}", "driver", "exec", "--", "bash", name]);
    json!({
        "name": "starter", "policy": "policy.json",
        "seats": {
            "intake": {"agent": "intake", "class": "work", "results": ["complete"]},
            "implement": {"agent": "implementer", "class": "work", "results": ["complete"]},
            "verify": {"class": "gate", "results": ["clean"], "hands": hands,
                       "role": "roles/work.md", "driver": {"command": script("verify.sh")}},
            "review": {"agent": "reviewer", "class": "gate", "results": ["clean"]},
            "ship": {"class": "gate", "results": ["clean"], "hands": hands,
                     "role": "roles/work.md", "driver": {"command": script("ship.sh")}},
        },
    })
}

/// The scaffold's table: each phase hands to the next.
fn scaffold_policy() -> Value {
    let phases = ["intake", "implement", "verify", "review", "ship", "done"];
    let rules: Vec<Value> = phases
        .windows(2)
        .map(|pair| {
            let result = match pair[0] {
                "intake" | "implement" => "complete",
                _ => "clean",
            };
            json!({"id": pair[0].to_uppercase(), "from": pair[0], "result": result,
                   "next": pair[1], "reason": pair[0]})
        })
        .collect();
    json!({"phases": phases, "initial": "intake", "terminal": ["done"], "rules": rules})
}

/// The whole fresh-scaffold-shaped bundle compiled, with the fence lifted
/// where `lifted`.
fn scaffold_compile(
    fixture: &AgentFixture,
    dsh: Option<Value>,
    claude: Option<Value>,
    lifted: bool,
) -> Result<Bundle, CompileError> {
    let config = dsh_scaffold(fixture, dsh, claude);
    fixture.stage(&config, &scaffold_policy());
    let lift = lifted.then(McpFence::lift);
    let compiled = Bundle::compile_with(&fixture.bundle(), &fixture.library(), &fixture.adapters());
    drop(lift);
    compiled
}

/// One site's records as words: each shape's bare invocation and whether
/// the shipped facts measure it or name it at all.
fn scaffold_words(bundle: &Bundle, site: &str) -> Vec<Vec<String>> {
    let site = bundle.sites[site].capabilities.as_ref().unwrap();
    let word = |(shape, ambient): &(McpShape, McpAxis)| {
        let verdict = match ambient {
            McpAxis::Measured { .. } => "measured",
            McpAxis::Unmeasured(McpUnmeasured::Absent) => "absent",
            other => panic!("an unexpected shipped fact: {other:?}"),
        };
        let invocation = match &shape.invocation {
            McpInvocation::Cold => "cold",
            McpInvocation::Replacement => "replacement",
            McpInvocation::Resume(name) => name,
        };
        format!("{invocation} {verdict}")
    };
    let records = site.strict.iter();
    records
        .map(|record| record.iter().map(word).collect())
        .collect()
}

/// SI2 over a whole fresh-scaffold-shaped compile in-crate, on the shipped
/// facts: the standing fence compiles it and keeps each model site's
/// record, exec's empty, while lifted it refuses the first dsh work site,
/// because U0 measured dsh's hands-less cold shape on Linux alone and no
/// replacement or resume shape.
#[test]
fn a_fresh_scaffold_on_shipped_facts_compiles_standing_and_refuses_its_first_dsh_site_lifted() {
    let fixture = AgentFixture::new();
    let standing = scaffold_compile(&fixture, None, None, false).unwrap();
    let cold = match host().1 {
        McpHost::Linux => "cold measured",
        McpHost::Macos => "cold absent",
    };
    let dsh_work = [cold, "replacement absent", "headless-work absent"].map(String::from);
    let none = Vec::<String>::new();
    let rows: Vec<Row<Vec<Vec<String>>>> = [
        ("intake", vec![dsh_work.to_vec(), dsh_work.to_vec()]),
        ("implement", vec![dsh_work.to_vec(), dsh_work.to_vec()]),
        (
            "review",
            vec![vec![cold.to_string()], vec![cold.to_string()]],
        ),
        ("verify", vec![none.clone()]),
        ("ship", vec![none]),
    ]
    .into_iter()
    .map(|(site, expected)| (site.to_string(), scaffold_words(&standing, site), expected))
    .collect();
    each_row(rows);
    let first = match host().1 {
        McpHost::Linux => "replacement",
        McpHost::Macos => "cold",
    };
    assert_eq!(
        scaffold_compile(&fixture, None, None, true)
            .unwrap_err()
            .to_string(),
        unmeasured("implement", "implementer", "dsh", first)
    );
}

/// Once the scaffold's dsh work sites are qualified, by synthetic plumbing
/// evidence that is never U0's, the separately hired Claude reviewer is the
/// first failing site for each SI2 cause with the fence lifted, and compiles
/// where its cold shape is measured; standing, every one compiles.
#[test]
fn past_qualified_dsh_work_sites_the_scaffolds_claude_reviewer_refuses_each_cause_lifted() {
    let fixture = AgentFixture::new();
    let shape = |harness: &str, invocation: Value, ambient: Value| {
        synthetic_shape(harness, invocation, "none", ambient)
    };
    let measured = json!({"measured": "synthetic plumbing, not U0"});
    let dsh = json!({"carriage": {"measured": "synthetic carriage"}, "shapes": [
        shape("dsh", json!("cold"), measured.clone()),
        shape("dsh", json!("replacement"), measured.clone()),
        shape("dsh", json!({"resume": "headless-work"}), measured.clone()),
    ]});
    let claude = |cold: Option<Value>| {
        let shapes: Vec<Value> = cold
            .into_iter()
            .map(|ambient| shape("claude", json!("cold"), ambient))
            .collect();
        json!({"carriage": {"measured": "synthetic carriage"}, "shapes": shapes})
    };
    let unsupported = json!({"unsupported": "project MCP configuration cannot be excluded"});
    let compile = |claude: &Value, lifted: bool| match scaffold_compile(
        &fixture,
        Some(dsh.clone()),
        Some(claude.clone()),
        lifted,
    ) {
        Ok(_) => "compiles".to_string(),
        Err(error) => error.to_string(),
    };
    let mut table: Vec<Row<String>> = Vec::new();
    for (label, claude, refusal) in [
        (
            "measured",
            claude(Some(measured.clone())),
            "compiles".to_string(),
        ),
        (
            "unsupported",
            claude(Some(unsupported)),
            cannot_exclude(
                "review",
                "reviewer",
                "claude",
                "project MCP configuration cannot be excluded",
            ),
        ),
        (
            "unmeasured",
            claude(None),
            unmeasured("review", "reviewer", "claude", "cold"),
        ),
    ] {
        table.push((format!("{label}, lifted"), compile(&claude, true), refusal));
        let standing = compile(&claude, false);
        table.push((format!("{label}, standing"), standing, "compiles".into()));
    }
    each_row(table);
}
