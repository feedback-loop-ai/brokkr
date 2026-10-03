//! The native capabilities as decision 0065 declares them (#484): keyed
//! as a realm grants them, switched off by the adapter's declared OFF
//! controls, which the probe launches, and compared against the
//! declaration, with each difference named. The configuration refusal
//! is measured only when its text names the unknown model.

use serde_json::{json, Value};

use super::*;

/// A plain turn listing Claude Code's two network tools beside its shell,
/// which no MCP server reached.
const PLAIN_WEB: &str = r#"tools='"Bash","WebSearch","WebFetch"'"#;

/// The native facts a report shows, and its native rows and verdict.
fn native_view(report: &Value) -> Value {
    let rows = report["adapter_fields"].as_array().unwrap();
    json!({
        "capabilities": report["facts"]["capabilities"],
        "egress_off": report["facts"]["egress_off"],
        "rows": rows[3..],
        "eligibility": report["eligibility"],
    })
}

/// A Claude-like CLI whose box keeps its tools, whose plain turn lists
/// [`PLAIN_WEB`], and which does `under_off` under the declared OFF
/// controls in place of honouring them.
fn kept_by_the_box(world: &World, name: &str, under_off: &str) -> Value {
    let script =
        claude_with("9.9.9", PLAIN_WEB, Boxed::Keeps.shell()).replace(OFF_HONOURED, under_off);
    let cli = world.fake(name, &script);
    native_view(&probe(AdapterKind::Claude, &cli, &claude_declared(), world))
}

const ALL_TOOLS: &str = "the system/init event on line 1 of stdout listed tools: 3";
const INVENTORY_AGREES: [&str; 3] = ["WebFetch, WebSearch", "WebFetch, WebSearch", "agrees"];
const BOXED_KEEPS: &str = "the system/init event on line 1 of stdout listed tools: 3; the \
                           system/init event on line 1 of stdout listed mcp_servers: 1";

fn inventory_row(cells: [&str; 3]) -> Value {
    field("native_capabilities.tools", cells[0], cells[1], cells[2])
}

fn off_row(key: &str, declared: &str, implied: &str, agreement: &str) -> Value {
    field(
        &format!("native_capabilities.known.{key}.off"),
        declared,
        implied,
        agreement,
    )
}

#[test]
fn the_declared_off_controls_are_launched_and_a_box_keeping_a_tool_says_nothing_of_them() {
    if !in_its_own_engine(
        "probe::tests::native::the_declared_off_controls_are_launched_and_a_box_keeping_a_tool_says_nothing_of_them",
    ) {
        return;
    }
    let world = world();
    let honoured = kept_by_the_box(&world, "claude-honoured", OFF_HONOURED);
    assert_eq!(
        honoured,
        json!({
            "capabilities": measured(
                claude_switched_off("the system/init event on line 1 of stdout listed tools: 1"),
                ALL_TOOLS,
            ),
            "egress_off": measured(json!(true), "the declared OFF controls removed WebSearch, WebFetch"),
            "rows": [
                inventory_row(INVENTORY_AGREES),
                off_row("web-fetch", "switched off", CLAUDE_SWITCHED, "agrees"),
                off_row("web-search", "switched off", CLAUDE_SWITCHED, "agrees"),
            ],
            "eligibility": {
                "verdict": "unboxed-only",
                "reason": format!(
                    "it is not shown to stand behind the box ({BOXED_KEEPS}), and the declared \
                     OFF controls removed WebSearch, WebFetch"
                ),
            },
        })
    );
    let left = |tool: &str| {
        unsupported(&format!(
            "the declared OFF controls left {tool}: {ALL_TOOLS}"
        ))
    };
    assert_eq!(
        kept_by_the_box(&world, "claude-ignored", "true"),
        json!({
            "capabilities": measured(
                json!([
                    capability("web-fetch", "WebFetch", left("WebFetch")),
                    capability("web-search", "WebSearch", left("WebSearch")),
                ]),
                ALL_TOOLS,
            ),
            "egress_off": measured(json!(false), "the declared OFF controls left WebSearch, WebFetch"),
            "rows": [
                inventory_row(INVENTORY_AGREES),
                off_row("web-fetch", "switched off", "unsupported", "differs"),
                off_row("web-search", "switched off", "unsupported", "differs"),
            ],
            "eligibility": {
                "verdict": "granting-realms-only",
                "reason": format!(
                    "no off switch exists for its native capabilities web-fetch, web-search, \
                     {GRANTING_REALMS}"
                ),
            },
        })
    );
}

#[test]
fn off_controls_the_cli_refuses_are_unsupported_and_off_controls_never_composed_are_unmeasured() {
    if !in_its_own_engine(
        "probe::tests::native::off_controls_the_cli_refuses_are_unsupported_and_off_controls_never_composed_are_unmeasured",
    ) {
        return;
    }
    let world = world();
    let refused = "the CLI refused the declared OFF controls: exit 1: error: unknown option \
                   '--disallowedTools'";
    let view = kept_by_the_box(
        &world,
        "claude-refuses-off",
        r#"echo "error: unknown option '--disallowedTools'" >&2; exit 1"#,
    );
    assert_eq!(
        (&view["capabilities"], &view["egress_off"], &view["rows"][1]),
        (
            &measured(
                json!([
                    capability("web-fetch", "WebFetch", unsupported(refused)),
                    capability("web-search", "WebSearch", unsupported(refused)),
                ]),
                ALL_TOOLS,
            ),
            &unsupported(refused),
            &off_row("web-fetch", "switched off", "unsupported", "differs"),
        )
    );
    let never = "no OFF control can switch off web-search";
    let declared = Declared {
        native: Native::Known {
            powers: vec![
                NativePower {
                    off: DeclaredOff::Unmeasured,
                    ..power("web-fetch", "WebFetch")
                },
                NativePower {
                    off: DeclaredOff::Unsupported,
                    ..power("web-search", "WebSearch")
                },
            ],
            off: OffControl::Refused(never.to_string()),
        },
        ..claude_declared()
    };
    let cli = world.fake(
        "claude-never",
        &claude_with("9.9.9", PLAIN_WEB, BOXED_CLEAN),
    );
    let report = probe(AdapterKind::Claude, &cli, &declared, &world);
    let untried = format!(
        "the turn under the declared OFF controls was not read: the engine composes no OFF \
         control for a seat granted nothing: {never}"
    );
    assert_eq!(
        native_view(&report),
        json!({
            "capabilities": measured(
                json!([
                    capability("web-fetch", "WebFetch", unmeasured(&untried)),
                    capability("web-search", "WebSearch", unmeasured(&untried)),
                ]),
                ALL_TOOLS,
            ),
            "egress_off": unmeasured(&untried),
            "rows": [
                inventory_row(INVENTORY_AGREES),
                off_row("web-fetch", "unmeasured", "unmeasured", "not-compared"),
                off_row("web-search", "unsupported", "unmeasured", "not-compared"),
            ],
            "eligibility": {
                "verdict": "boxed",
                "reason": "its own tools switch off and the hands MCP server connects",
            },
        })
    );
}

/// The facts and rows of a plain turn listing `plain_tools`, whose turn
/// under the declared OFF controls listed `off_tools`, against `declared`.
fn declared_against(declared: &Declared, plain_tools: &str, off_tools: &str) -> Value {
    let init = |tools: &str| {
        format!(r#"{{"type":"system","subtype":"init","tools":[{tools}],"mcp_servers":[]}}"#)
    };
    let plan = plan::plan(AdapterKind::Claude, declared).unwrap();
    let observed = Observed {
        native_off: Trial::Observed(observation(Some(0), &init(off_tools), "")),
        ..observed(observation(Some(0), &init(plain_tools), ""))
    };
    let facts = measure::reading(&plan, &observed, &[]).facts;
    let version = Fact::measured("2.1.266".to_string(), "printed");
    let rows = judge::adapter_fields(declared, &facts, &version);
    json!({
        "capabilities": facts.capabilities,
        "rows": serde_json::to_value(&rows[3..]).unwrap(),
    })
}

#[test]
fn a_declaration_the_probe_does_not_reproduce_is_named_as_a_difference() {
    let plain = r#""Bash","WebSearch","WebFetch""#;
    let listed = "the system/init event on line 1 of stdout listed tools";
    let unsupported_off = Declared {
        native: Native::Known {
            powers: vec![
                NativePower {
                    off: DeclaredOff::Unsupported,
                    ..power("web-fetch", "WebFetch")
                },
                power("web-search", "WebSearch"),
            ],
            off: OffControl::Argv(strings(&CLAUDE_OFF)),
        },
        ..claude_declared()
    };
    assert_eq!(
        declared_against(&unsupported_off, plain, r#""Bash""#),
        json!({
            "capabilities": measured(claude_switched_off(&format!("{listed}: 1")), &format!("{listed}: 3")),
            "rows": [
                inventory_row(INVENTORY_AGREES),
                off_row("web-fetch", "unsupported", CLAUDE_SWITCHED, "differs"),
                off_row("web-search", "switched off", CLAUDE_SWITCHED, "agrees"),
            ],
        })
    );
    let renamed = Declared {
        native: Native::Known {
            powers: vec![
                power("web-fetch", "WebFetch"),
                power("web-search", "WebSearchV2"),
            ],
            off: OffControl::Argv(strings(&CLAUDE_OFF)),
        },
        ..claude_declared()
    };
    assert_eq!(
        declared_against(&renamed, plain, r#""Bash""#),
        json!({
            "capabilities": unmeasured(
                "the plain turn listed WebSearch, which no declared native capability maps, so \
                 no realm can grant it"
            ),
            "rows": [
                inventory_row(["WebFetch, WebSearchV2", "WebFetch, WebSearch", "differs"]),
                off_row("web-fetch", "switched off", "unmeasured", "not-compared"),
                off_row("web-search", "switched off", "unmeasured", "not-compared"),
            ],
        })
    );
    let unmeasured_inventory = Declared {
        native: Native::Unmeasured(DSH_NATIVE.to_string()),
        ..claude_declared()
    };
    assert_eq!(
        declared_against(&unmeasured_inventory, plain, r#""Bash""#)["rows"],
        json!([inventory_row([
            "unmeasured",
            "WebFetch, WebSearch",
            "differs"
        ])])
    );
}

#[test]
fn a_model_refusal_that_does_not_name_the_model_is_not_a_configuration_refusal() {
    let plan = plan::plan(AdapterKind::Claude, &claude_declared()).unwrap();
    let outage = "API Error: 503 provider temporarily unavailable";
    let observed = Observed {
        bad_model: Trial::Observed(observation(Some(1), "", outage)),
        ..observed(observation(Some(0), "", ""))
    };
    let refusals = measure::reading(&plan, &observed, &[]).facts.refusals;
    assert_eq!(
        (refusals.config, refusals.outage),
        (
            Fact::unmeasured(format!(
                "the refusal does not name the model brokkr-probe-no-such-model, so it is not \
                 shown to be the configuration's: exit 1: {outage}"
            )),
            Fact::unmeasured("not provoked: a provider outage cannot be caused safely"),
        )
    );
}
