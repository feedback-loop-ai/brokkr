//! GP1 (decision 0065 slice two, U3a) at every executable site a compile
//! resolves: each judged at its own canonical class and stable office,
//! once per provider candidate, whatever contains or relocates it. Beside
//! it, the bundle's binding minimum (MB4, U5a2) as a compile meets it:
//! the MCP fence unchanged, and native holdings untouched.

use super::*;
use crate::capabilities::CapabilityContext;

impl AgentFixture {
    /// The fixture with `charters/data.md` and the inline role
    /// `roles/data.md`, whose one paragraph declares every capability an
    /// office or an inline site here asks for (GP2, U3b and U3c). The parent
    /// suite shares it, sitting over its own line ceiling.
    pub(super) fn declaring() -> AgentFixture {
        let fixture = AgentFixture::new();
        crate::agents::charter_data::write_declaring(&fixture.library().join("charters/data.md"));
        crate::agents::charter_data::write_declaring(&fixture.bundle().join("roles/data.md"));
        fixture
    }
}

/// Office `judge` on codex, asking `web-search` at `strength`, served by
/// the chain `models`.
pub(super) fn hire_judge(fixture: &AgentFixture, strength: &str, models: &[&str]) {
    let efforts: Map<String, Value> = models
        .iter()
        .map(|m| (m.to_string(), json!("high")))
        .collect();
    fixture.write(
        "agents/judge.json",
        json!({"description": "a judge", "charter": "charters/data.md", "models": models,
               "efforts": efforts, "capabilities": {"web-search": strength}}),
    );
    let mut adapter = codex();
    adapter["models"]["sol"] = json!("gpt-6-sol");
    adapter["judges"] = json!(["astra", "sol"]);
    fixture.write("adapters/codex.json", adapter);
}

/// `grant_web_search`'s realm with the grant's offices list replaced.
fn offices(fixture: &AgentFixture, offices: Option<&[&str]>) -> CapabilityContext {
    let mut context = grant_web_search(fixture, None);
    let grant = context.grants.get_mut("web-search").unwrap();
    grant.offices = offices.map(|named| named.iter().map(ToString::to_string).collect());
    context
}

/// The staged fixture compiled in realm `private` under `namespace`.
fn compiled(
    fixture: &AgentFixture,
    dialect: Option<&Dialect>,
    context: &CapabilityContext,
) -> Result<Bundle, CompileError> {
    Bundle::compile_with_capabilities(
        &fixture.bundle(),
        &fixture.library(),
        &fixture.adapters(),
        Some("private"),
        dialect,
        Boundary::Namespace,
        context,
    )
}

/// GP1's egress cause for `office`, pinned once for this module.
fn egress(office: &str) -> String {
    format!(
        "a gate's egress capability requires the realm grant to name office '{office}' explicitly"
    )
}

fn refused(site: &str, office: &str) -> String {
    format!(
        "bundle: seat '{site}' (office '{office}') in realm 'private': requires capability \
         'web-search' through dialect 'codex-search', but {}",
        egress(office)
    )
}

fn notice(site: &str) -> String {
    format!(
        "seat '{site}' (office 'judge') in realm 'private': dropped wanted capability \
         'web-search' through dialect 'codex-search' because {}; native capability \
         remains OFF",
        egress("judge")
    )
}

/// Per candidate of `label`: what it holds and the notices it carries.
fn held(result: Result<Bundle, CompileError>, label: &str) -> String {
    match result {
        Ok(bundle) => {
            let site = bundle.sites[label].capabilities.as_ref().unwrap();
            let outcomes = site.outcomes.iter().map(|outcome| {
                let held: Vec<&String> = outcome.held.keys().collect();
                format!("{held:?} {:?}", outcome.notices)
            });
            outcomes.collect::<Vec<_>>().join(" | ")
        }
        Err(error) => error.to_string(),
    }
}

/// Every executable form holding the judge as a gate under a work
/// container, beside a work sibling — or, in a panel, which never mixes
/// classes, beside a `neighbour` gate office that asks nothing. The
/// container's class, the sibling, a neighbour the grant names and the
/// execution label lend it nothing; its own office named explicitly admits.
#[test]
fn every_executable_form_is_judged_at_its_own_class_and_office() {
    let fixture = AgentFixture::declaring();
    hire_judge(&fixture, "requires", &["astra"]);
    fixture.write(
        "agents/neighbour.json",
        json!({"description": "a neighbour", "charter": "charters/data.md",
               "models": ["astra"], "efforts": {"astra": "high"}}),
    );
    let sibling = json!({"role": "roles/work.md", "driver": {"command": ["driver"]}});
    let forms = |class: &str| {
        let judge = json!({"agent": "judge", "class": class});
        let neighbour = json!({"agent": "neighbour", "class": class});
        let step = json!({"name": "first", "results": ["complete"], "agent": "judge",
                          "class": class});
        [
            (
                "work",
                json!({"results": ["complete"], "agent": "judge", "class": class}),
                policy(),
            ),
            (
                "work:a",
                json!({"results": ["pass", "fail"], "aggregate": "unanimous-pass",
                              "panel": {"a": judge, "b": neighbour}}),
                panel_policy(),
            ),
            (
                "work:first",
                json!({"results": ["complete"], "sequence": [step,
                {"name": "second", "role": "roles/work.md", "driver": {"command": ["driver"]}}]}),
                policy(),
            ),
            (
                "work:engine",
                json!({"results": ["complete"], "select": {"on": "strategy",
                "cases": {"engine": judge}, "default": sibling}}),
                policy(),
            ),
            (
                "work:default",
                json!({"results": ["complete"], "select": {"on": "strategy",
                "cases": {}, "default": judge}}),
                policy(),
            ),
        ]
    };
    let compile = |seat: &Value, table: &Value, context: &CapabilityContext| {
        let mut config = fixture.config();
        config["seats"]["work"] = seat.clone();
        fixture.stage(&config, table);
        compiled(&fixture, None, context)
    };
    let mut rows: Vec<Row<String>> = Vec::new();
    for ((label, gate, table), (_, work, _)) in forms("gate").into_iter().zip(forms("work")) {
        let unnamed = offices(&fixture, None);
        rows.push((
            format!("{label} unnamed"),
            held(compile(&gate, &table, &unnamed), label),
            refused(label, "judge"),
        ));
        let labelled = offices(&fixture, Some(&[label, "neighbour"]));
        rows.push((
            format!("{label} names its label and a neighbour"),
            held(compile(&gate, &table, &labelled), label),
            format!(
                "bundle: seat '{label}' (office 'judge') in realm 'private': requires capability \
                 'web-search' but the realm grants it only to offices [{label}, neighbour], not \
                 to this office"
            ),
        ));
        let named = offices(&fixture, Some(&["judge"]));
        rows.push((
            format!("{label} named"),
            held(compile(&gate, &table, &named), label),
            r#"["web-search"] []"#.to_string(),
        ));
        rows.push((
            format!("{label} as work"),
            held(compile(&work, &table, &unnamed), label),
            r#"["web-search"] []"#.to_string(),
        ));
    }
    assert_eq!(rows.len(), 20);
    each_row(rows);
}

/// Each provider candidate is resolved on the site's own class: a wanted
/// egress is dropped on the primary and on the fallback alike, each
/// recording the same notice in its own outcome.
#[test]
fn every_fallback_candidate_drops_a_gates_unnamed_egress() {
    let fixture = AgentFixture::declaring();
    hire_judge(&fixture, "wants", &["astra", "sol"]);
    let mut config = fixture.config();
    config["seats"]["work"] = json!({"results": ["complete"], "agent": "judge", "class": "gate"});
    fixture.stage(&config, &policy());
    let compiled = compiled(&fixture, None, &offices(&fixture, None));
    let entry = ("web-search".to_string(), notice("work"));
    assert_eq!(
        held(compiled, "work"),
        format!("[] {:?} | [] {:?}", [&entry], [&entry])
    );
}

/// An inline codex site of `class` requiring `web-search`, its adapter
/// trusted to seat a gate and its role declaring the ask.
fn inline_codex(fixture: &AgentFixture, class: &str) -> Value {
    let mut adapter = codex();
    adapter["trust_tier"] = json!("trusted");
    fixture.write("adapters/codex.json", adapter);
    let command = json!([
        "{brokkr}",
        "driver",
        "codex",
        "--",
        "--model",
        "gpt-6-astra",
        "--effort",
        "high"
    ]);
    json!({"results": ["complete"], "class": class, "role": "roles/data.md",
           "driver": {"command": command}, "capabilities": {"web-search": "requires"}})
}

/// An inline gate is judged at its own class, as the office its label is:
/// GP1 refuses its egress unless the grant names that label, and the same
/// site at work holds it unnamed.
#[test]
fn an_inline_gate_is_judged_at_its_own_class_and_label() {
    let fixture = AgentFixture::declaring();
    let compile = |class: &str, named: Option<&[&str]>| {
        let mut config = fixture.config();
        config["seats"]["work"] = inline_codex(&fixture, class);
        fixture.stage(&config, &policy());
        held(compiled(&fixture, None, &offices(&fixture, named)), "work")
    };
    let held = r#"["web-search"] []"#;
    each_row(vec![
        (
            "unnamed gate".into(),
            compile("gate", None),
            refused("work", "work"),
        ),
        (
            "named gate".into(),
            compile("gate", Some(&["work"])),
            held.into(),
        ),
        ("unnamed work".into(), compile("work", None), held.into()),
    ]);
}

/// The `openspec` dialect wrapping `verify`, seated with the judge as a
/// gate: `design` is a dialect phase, a draft and then the dialect's own
/// `validate` step, compiled in a realm whose grant names no office.
pub(super) fn wrapped_verify(fixture: &AgentFixture) -> Result<Bundle, CompileError> {
    let dialect = openspec_with_exec(fixture);
    let table = json!({
        "phases": ["design", "verify", "review", "done"], "initial": "design",
        "terminal": ["done"],
        "rules": [
            {"id":"DD", "from":"design", "result":"drafted", "next":"verify", "reason":"draft"},
            {"id":"DX", "from":"design", "result":"fail", "next":"design", "reason":"redo"},
            {"id":"VP", "from":"verify", "result":"pass", "next":"review", "reason":"pass"},
            {"id":"VR", "from":"verify", "result":"fail", "next":"verify", "reason":"retry"},
            {"id":"RC", "from":"review", "result":"clean", "next":"done", "reason":"clean"},
        ],
    });
    let mut config = fixture.config();
    let inline = config["seats"]["review"].clone();
    let mut draft = inline.clone();
    draft["name"] = json!("draft");
    draft["results"] = json!(["drafted"]);
    config["protected_phase"] = json!("review");
    config["seats"] = json!({
        "design": {"results": ["drafted", "fail"],
                   "sequence": [draft, {"name": "validate", "dialect": "validate"}]},
        "verify": {"results": ["pass", "fail"], "agent": "judge", "class": "gate"},
        "review": inline,
    });
    fixture.stage(&config, &table);
    compiled(fixture, Some(&dialect), &offices(fixture, None))
}

/// A dialect-wrapped verify is judged before the wrapper relocates it, as
/// the office its agent is — never as the label the wrapper moves it to —
/// and its outcome travels with it to `verify:checks`.
#[test]
fn a_relocated_verify_keeps_its_class_and_stable_office() {
    let fixture = AgentFixture::declaring();
    hire_judge(&fixture, "wants", &["astra"]);
    let entry = ("web-search".to_string(), notice("verify"));
    assert_eq!(
        held(wrapped_verify(&fixture), "verify:checks"),
        format!("[] {:?}", [&entry])
    );
    hire_judge(&fixture, "requires", &["astra"]);
    let refusal = held(wrapped_verify(&fixture), "verify:checks");
    assert_eq!(refusal, refused("verify", "judge"));
}

/// A dialect step writes no class and asks for nothing, and its record
/// still carries its canonical class (GP1) — the gate its compiled step
/// is — never a default: the step and its record read one fact.
#[test]
fn a_dialect_step_is_recorded_at_the_class_its_step_compiles_to() {
    let fixture = AgentFixture::declaring();
    hire_judge(&fixture, "wants", &["astra"]);
    let bundle = wrapped_verify(&fixture).unwrap();
    let SeatBody::Sequence { steps } = &bundle.seats["design"].body else {
        panic!("design compiles to a sequence");
    };
    let record = bundle.sites["design:validate"].capabilities.as_ref();
    let classes = (
        steps[1].name.as_str(),
        steps[1].class,
        record.unwrap().asks.class,
    );
    assert_eq!(classes, ("validate", SeatClass::Gate, SeatClass::Gate));
}

/// The bundle's binding minimum, written as `minimum` or left absent.
fn at_minimum(fixture: &AgentFixture, minimum: Option<&str>) -> Value {
    let mut config = fixture.config();
    if let Some(minimum) = minimum {
        config["egress_minimum"] = json!(minimum);
    }
    config
}

/// The fixture's realm, also granting `library-docs` as `grant` through
/// the `mcp` dialect `docs-mcp`, whose egress is `egress`.
fn granting_docs(fixture: &AgentFixture, egress: &str, grant: Value) -> CapabilityContext {
    let mut context = offices(fixture, None);
    define(fixture, "library-docs", json!(["reads", "egress"]));
    fixture.write(
        "dialects/tools/docs-mcp.json",
        json!({"schema": "brokkr.tool-dialect/v1", "name": "docs-mcp", "serves": "library-docs",
               "kind": "mcp", "connection": {"argv": ["/nonexistent/docs-mcp"]},
               "version": "1.4.2", "secrets": ["DOCS_TOKEN"], "tools": ["resolve", "read"],
               "egress": egress, "sends": {"description": "a library", "seat_composed": true}}),
    );
    let map = json!({"schema": "forge.realms/v6", "journal": "forge.db", "realms": [
        {"name": "private", "path": "repo", "default_branch": "main",
         "capabilities": {"library-docs": grant}}]});
    let (map, _) = brokkr_core::realms::RealmMap::of("realms.json", map).unwrap();
    context.grants.extend(map.realms[0].grants.clone());
    context
}

/// SC5's realm-wide fence on an `mcp` grant, in its own words, until U9b.
const MCP_FENCE: &str = "bundle: realm 'private' grants capability 'library-docs' through \
                         dialect 'docs-mcp' of kind 'mcp', whose broker support is not \
                         implemented until decision 0065 slice two";

/// The fixture with a second office, `reader`, requiring `library-docs`.
fn with_reader() -> AgentFixture {
    let fixture = AgentFixture::declaring();
    fixture.write(
        "agents/reader.json",
        json!({"description": "a reader", "charter": "charters/data.md", "models": ["opus"],
               "efforts": {"opus": "high"}, "capabilities": {"library-docs": "requires"}}),
    );
    fixture
}

/// SC5's fence is unchanged by MB4 until U9b: under every binding
/// minimum, with the dialect's egress below, at or above it, an unused,
/// an office-excluded and an asked `mcp` grant — asked at an agent's
/// candidate and at an inline site — each refuse the compile with the
/// fence's own words. Comparing egress authorizes nothing.
#[test]
fn every_mcp_grant_still_refuses_the_compile_under_every_binding_minimum() {
    let fixture = with_reader();
    let fence = MCP_FENCE;
    let agent = |name: &str| json!({"results": ["complete"], "agent": name});
    let mut inline = inline_codex(&fixture, "work");
    inline["capabilities"] = json!({"library-docs": "requires"});
    let granted = json!({"dialect": "docs-mcp"});
    let grants = [
        ("unused", granted.clone(), agent("worker")),
        (
            "scoped to none",
            json!({"dialect": "docs-mcp", "offices": []}),
            agent("reader"),
        ),
        (
            "excluded",
            json!({"dialect": "docs-mcp", "offices": ["judge"]}),
            agent("reader"),
        ),
        ("asked", granted.clone(), agent("reader")),
        ("asked inline", granted, inline),
    ];
    let mut rows: Vec<Row<String>> = Vec::new();
    for minimum in [
        None,
        Some("local"),
        Some("contracted"),
        Some("uncontracted"),
    ] {
        for egress in ["local", "contracted", "uncontracted"] {
            for (label, grant, seat) in &grants {
                let mut config = at_minimum(&fixture, minimum);
                config["seats"]["work"] = seat.clone();
                fixture.stage(&config, &policy());
                let context = granting_docs(&fixture, egress, grant.clone());
                let refusal = compiled(&fixture, None, &context).map(|_| ());
                rows.push((
                    format!("{minimum:?} {egress} {label}"),
                    held_or(refusal),
                    fence.to_string(),
                ));
            }
        }
    }
    assert_eq!(rows.len(), 60);
    each_row(rows);
}

/// What a compile came to: its refusal, or that it compiled.
fn held_or(result: Result<(), CompileError>) -> String {
    result.map_or_else(|error| error.to_string(), |()| "compiled".to_string())
}

/// MB4 judges an `mcp` dialect alone: at an agent's candidate and at an
/// inline site, a native holding through the `uncontracted` dialect
/// `codex-search` is what it was under every binding minimum.
#[test]
fn a_native_holding_is_unchanged_at_every_serving_path_under_every_binding_minimum() {
    let fixture = AgentFixture::declaring();
    hire_judge(&fixture, "requires", &["astra"]);
    let inline = inline_codex(&fixture, "work");
    let mut rows: Vec<Row<String>> = Vec::new();
    for minimum in [
        None,
        Some("local"),
        Some("contracted"),
        Some("uncontracted"),
    ] {
        for (path, seat) in [
            ("agent", json!({"results": ["complete"], "agent": "judge"})),
            ("inline", inline.clone()),
        ] {
            let mut config = at_minimum(&fixture, minimum);
            config["seats"]["work"] = seat;
            fixture.stage(&config, &policy());
            rows.push((
                format!("{minimum:?} {path}"),
                held(compiled(&fixture, None, &offices(&fixture, None)), "work"),
                r#"["web-search"] []"#.to_string(),
            ));
        }
    }
    each_row(rows);
}

/// The minimum a compile parses is the one its authority holds: the route
/// policy reads it from there, so a seat binding a secret over the
/// `contracted` claude route meets every minimum but `local`, as the
/// bundle declares it and not the authority's absent default.
#[test]
fn the_authority_holds_the_binding_minimum_the_bundle_declares() {
    let fixture = AgentFixture::declaring();
    let below = "bundle: seat 'work' declares secret bindings [\"TOKEN\"] but seats driver \
                 'claude' on its own declared destination, whose egress class is contracted; \
                 this bundle binds no secret below local (decision 0021 ruling 4 as enacted \
                 by 0036 ruling 4 — an undeclared class is uncontracted, and 'egress_minimum' \
                 is where the operator rules the bar)";
    let mut rows: Vec<Row<String>> = Vec::new();
    for (minimum, expected) in [
        (None, "compiled"),
        (Some("local"), below),
        (Some("contracted"), "compiled"),
        (Some("uncontracted"), "compiled"),
    ] {
        let mut config = at_minimum(&fixture, minimum);
        config["seats"]["work"]["secrets"] = json!(["TOKEN"]);
        fixture.stage(&config, &policy());
        let compile = compiled(&fixture, None, &offices(&fixture, None)).map(|_| ());
        rows.push((
            format!("{minimum:?}"),
            held_or(compile),
            expected.to_string(),
        ));
    }
    each_row(rows);
}

/// U1b: typed adapter MCP facts grant nothing. With the serving adapter
/// declaring every carriage and isolation axis measured, or a legacy map
/// naming the very server, an asked `mcp` grant still meets SC5's fence in
/// its own words, and a seat asking nothing composes the same argv as
/// under a bare `"unsupported"`.
#[test]
fn typed_or_legacy_adapter_mcp_facts_grant_nothing_and_the_fence_holds() {
    let fixture = with_reader();
    define(&fixture, "library-docs", json!(["reads", "egress"]));
    let measured = json!({"measured": "a sentinel measurement"});
    let typed = json!({"carriage": measured, "shapes": [{
        "invocation": "cold", "hands": "none",
        "measured_on": {"harness": "claude", "binary": "claude", "version": "2.1.287",
                        "host": "linux"},
        "ambient": measured, "native_write": measured,
        "store_read": measured, "process_read": measured}]});
    let legacy = json!({"flag": "--mcp-config", "servers": {"cap-library-docs": "/srv/docs"}});
    let argv = |mcp: Value, grant: Option<Value>, seat: &str| {
        let mut adapter = claude();
        adapter["mcp"] = mcp;
        fixture.write("adapters/claude.json", adapter);
        let mut config = fixture.config();
        config["seats"]["work"]["agent"] = json!(seat);
        fixture.stage(&config, &policy());
        let context = match grant {
            Some(grant) => granting_docs(&fixture, "local", grant),
            None => offices(&fixture, None),
        };
        compiled(&fixture, None, &context).map_or_else(
            |error| error.to_string(),
            |bundle| match &bundle.seats["work"].body {
                SeatBody::Single { command, .. } => format!("{command:?}"),
                _ => "not a single seat".to_string(),
            },
        )
    };
    let unsupported = argv(json!("unsupported"), None, "worker");
    assert!(unsupported.contains("\"claude-opus-5\""), "{unsupported}");
    let asked = json!({"dialect": "docs-mcp"});
    let rows: Vec<Row<String>> = vec![
        (
            "typed asks nothing".into(),
            argv(typed.clone(), None, "worker"),
            unsupported.clone(),
        ),
        (
            "legacy asks nothing".into(),
            argv(legacy.clone(), None, "worker"),
            unsupported,
        ),
        (
            "typed asked".into(),
            argv(typed, Some(asked.clone()), "reader"),
            MCP_FENCE.into(),
        ),
        (
            "legacy asked".into(),
            argv(legacy, Some(asked), "reader"),
            MCP_FENCE.into(),
        ),
    ];
    each_row(rows);
}
