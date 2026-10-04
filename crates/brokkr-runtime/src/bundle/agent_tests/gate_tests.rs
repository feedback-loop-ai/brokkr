//! GP1 (decision 0065 slice two, U3a) at every executable site a compile
//! resolves: each judged at its own canonical class and stable office,
//! once per provider candidate, whatever contains or relocates it.

use super::*;
use crate::capabilities::CapabilityContext;

/// Office `judge` on codex, asking `web-search` at `strength`, served by
/// the chain `models`.
fn hire_judge(fixture: &AgentFixture, strength: &str, models: &[&str]) {
    let efforts: Map<String, Value> = models
        .iter()
        .map(|m| (m.to_string(), json!("high")))
        .collect();
    fixture.write(
        "agents/judge.json",
        json!({"description": "a judge", "charter": "charters/work.md", "models": models,
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
    let fixture = AgentFixture::new();
    hire_judge(&fixture, "requires", &["astra"]);
    fixture.write(
        "agents/neighbour.json",
        json!({"description": "a neighbour", "charter": "charters/work.md",
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
    let fixture = AgentFixture::new();
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

/// An inline gate is judged at its own class, as the office its label is:
/// GP1 refuses its egress unless the grant names that label, and the same
/// site at work holds it unnamed.
#[test]
fn an_inline_gate_is_judged_at_its_own_class_and_label() {
    let fixture = AgentFixture::new();
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
    let compile = |class: &str, named: Option<&[&str]>| {
        let mut config = fixture.config();
        config["seats"]["work"] = json!({"results": ["complete"], "class": class,
            "role": "roles/work.md", "driver": {"command": command},
            "capabilities": {"web-search": "requires"}});
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
fn wrapped_verify(fixture: &AgentFixture) -> Result<Bundle, CompileError> {
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
    let fixture = AgentFixture::new();
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
    let fixture = AgentFixture::new();
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
