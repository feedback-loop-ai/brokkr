//! The native capabilities as decision 0065 declares them (#484): keyed
//! as a realm grants them, switched off by the adapter's declared OFF
//! controls, which the probe launches, and compared against the
//! declaration, with each difference named. The configuration refusal
//! is measured only when a line is itself the unknown model's refusal.

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
    native_view(&kept_report(world, name, under_off))
}

/// The whole report of [`kept_by_the_box`]'s CLI.
fn kept_report(world: &World, name: &str, under_off: &str) -> Value {
    let script =
        claude_with("9.9.9", PLAIN_WEB, Boxed::Keeps.shell()).replace(OFF_HONOURED, under_off);
    let cli = world.fake(name, &script);
    probe(AdapterKind::Claude, &cli, &claude_declared(), world)
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

/// The chief's OFF turns on 25a0ea04 (#484): the turn under the declared
/// OFF controls, which a seat outside the box granted nothing launches,
/// is read like the plain and boxed turns. Another MCP server read
/// reaching it, by a listed tool, a listed server or a line of stderr,
/// refuses that seat, and a listing value it leaves unread is a gap.
#[test]
fn the_turn_under_the_declared_off_controls_is_read_like_the_plain_and_boxed_turns() {
    if !in_its_own_engine(
        "probe::tests::native::the_turn_under_the_declared_off_controls_is_read_like_the_plain_and_boxed_turns",
    ) {
        return;
    }
    let world = world();
    let leaked = |reach: &str| {
        json!({
            "verdict": "refused",
            "reason": format!(
                "its turn under the declared OFF controls, the launch a seat granted nothing \
                 uses, loaded an MCP server the probe did not give it (#467): {reach}\
                 {HOLDS_NO_BOX}it is not shown to stand behind the box ({BOXED_KEEPS}), and the \
                 declared OFF controls removed WebSearch, WebFetch"
            ),
        })
    };
    let string = "the system event on line 2 of stdout holds at /mcp_servers a value that is not \
                  a list: \"github\"";
    let rows = [
        (
            r#"tools='"mcp__github__search"'; servers='{"name":"github","status":"connected"}'"#,
            leaked(
                "mcp__github__search of the MCP server github was listed by the system/init \
                 event on line 1 of stdout at /tools/0",
            ),
        ),
        (
            r#"tools='"Bash"'; echo "MCP server github connected" >&2"#,
            leaked("line 1 of stderr names the MCP server github"),
        ),
        (
            r#"tools='"Bash"'; later='{"type":"system","mcp_servers":"github"}'"#,
            json!({
                "verdict": "refused",
                "reason": format!(
                    "the evidence for unboxed offices is not complete: in the OFF turn, {string}; \
                     user_mcp_off is unmeasured: no MCP server other than brokkr was read \
                     reaching the turn, but the turn's mcp_servers could not be read whole: \
                     {string}"
                ),
            }),
        ),
        (
            r#"tools='"Bash"'; servers='{"name":"brokkr-probe-user-scope","status":"connected"}'"#,
            leaked(
                "the MCP server brokkr-probe-user-scope was listed connected by the system/init \
                 event on line 1 of stdout at /mcp_servers/0",
            ),
        ),
    ];
    for (index, (under_off, eligibility)) in rows.into_iter().enumerate() {
        let report = kept_report(&world, &format!("claude-off-{index}"), under_off);
        assert_eq!(
            (under_off, &report["eligibility"]),
            (under_off, &eligibility)
        );
    }
}

/// The chief's absent baseline on 25a0ea04 (#484): with the plain, boxed
/// and OFF turns all listing `Bash` alone, no control was seen switching
/// either capability off, so neither off switch is measured, neither OFF
/// row is compared, and the harness is refused for want of them.
#[test]
fn a_capability_the_plain_turn_never_listed_has_no_measured_off_switch() {
    if !in_its_own_engine(
        "probe::tests::native::a_capability_the_plain_turn_never_listed_has_no_measured_off_switch",
    ) {
        return;
    }
    let world = world();
    let bash = r#"tools='"Bash"'"#;
    let cli = world.fake("claude-absent", &claude_with("9.9.9", bash, bash));
    let report = probe(AdapterKind::Claude, &cli, &claude_declared(), &world);
    let listed = "the system/init event on line 1 of stdout listed tools: 1";
    let never = |power: &str, tool: &str| {
        let why = format!(
            "the plain turn listed none of {tool}, so no control was seen switching it off: \
             {listed}"
        );
        (
            capability(power, tool, unmeasured(&why)),
            format!("{power}'s off switch is unmeasured: {why}"),
        )
    };
    let (fetch, fetch_gap) = never("web-fetch", "WebFetch");
    let (search, search_gap) = never("web-search", "WebSearch");
    assert_eq!(
        native_view(&report),
        json!({
            "capabilities": measured(json!([fetch, search]), listed),
            "egress_off": measured(json!(true), "the plain turn listed no native egress tool"),
            "rows": [
                inventory_row(["WebFetch, WebSearch", "", "differs"]),
                off_row("web-fetch", "switched off", "unmeasured", "not-compared"),
                off_row("web-search", "switched off", "unmeasured", "not-compared"),
            ],
            "eligibility": {
                "verdict": "refused",
                "reason": format!(
                    "the evidence for a seat in a realm that grants its capabilities is not \
                     complete: {fetch_gap}; {search_gap}"
                ),
            },
        })
    );
}

/// The chief's OFF turns on 25a0ea04 that fail for want of the provider
/// or the account (#484): a failure that names no flag or key of the
/// declared OFF controls is not the CLI refusing them, so no off switch
/// is read absent, and the harness is refused for want of one.
#[test]
fn an_off_turn_that_fails_naming_no_control_is_unread_not_a_missing_off_switch() {
    if !in_its_own_engine(
        "probe::tests::native::an_off_turn_that_fails_naming_no_control_is_unread_not_a_missing_off_switch",
    ) {
        return;
    }
    let world = world();
    for (index, line) in [
        "API Error: 503 provider temporarily unavailable",
        "Invalid API key",
    ]
    .into_iter()
    .enumerate()
    {
        let under_off = format!(r#"echo "{line}" >&2; exit 1"#);
        let view = kept_by_the_box(&world, &format!("claude-off-fails-{index}"), &under_off);
        let failed = format!(
            "the turn under the declared OFF controls failed, naming no flag or key of the \
             declared OFF controls: exit 1: {line}"
        );
        let unread = format!("the turn under the declared OFF controls was not read: {failed}");
        assert_eq!(
            view,
            json!({
                "capabilities": measured(
                    json!([
                        capability("web-fetch", "WebFetch", unmeasured(&unread)),
                        capability("web-search", "WebSearch", unmeasured(&unread)),
                    ]),
                    ALL_TOOLS,
                ),
                "egress_off": unmeasured(&unread),
                "rows": [
                    inventory_row(INVENTORY_AGREES),
                    off_row("web-fetch", "switched off", "unmeasured", "not-compared"),
                    off_row("web-search", "switched off", "unmeasured", "not-compared"),
                ],
                "eligibility": {
                    "verdict": "refused",
                    "reason": format!(
                        "the evidence for a seat in a realm that grants its capabilities is not \
                         complete: user_mcp_off is unmeasured: {failed}; web-fetch's off switch \
                         is unmeasured: {unread}; web-search's off switch is unmeasured: {unread}"
                    ),
                },
            })
        );
    }
}

/// A refusal of Codex's declared OFF control names its config key, and
/// only a failure that names it is the CLI refusing that control.
#[test]
fn an_off_refusal_is_read_by_the_config_key_it_names() {
    let plan = plan::plan(AdapterKind::Codex, &codex_declared()).unwrap();
    let off_turn = |stderr: &str| {
        let observed = Observed {
            native_off: Trial::Observed(observation(Some(1), "", stderr)),
            ..observed(observation(Some(0), "", ""))
        };
        measure::reading(&plan, &observed, &[]).facts.user_mcp_off
    };
    let named = "Error loading config: unknown key `web_search`";
    let other = "Error loading config: unknown key `web_fetch`";
    assert_eq!(
        (off_turn(named), off_turn(other)),
        (
            Fact::Unsupported {
                evidence: format!("the CLI refused the declared OFF controls: exit 1: {named}"),
            },
            Fact::unmeasured(format!(
                "the turn under the declared OFF controls failed, naming no flag or key of the \
                 declared OFF controls: exit 1: {other}"
            )),
        )
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
            Fact::unmeasured(not_the_configuration(&format!("exit 1: {outage}"))),
            Fact::unmeasured("not provoked: a provider outage cannot be caused safely"),
        )
    );
}

fn not_the_configuration(ended: &str) -> String {
    format!(
        "no line of the refusal names the model brokkr-probe-no-such-model in a model refusal's \
         words and no other class's, so it is not shown to be the configuration's: {ended}"
    )
}

/// The chief's shapes on 25a0ea04: a line that names the model beside
/// another class's mark, or an event that only echoes it, is not the
/// model's refusal; a model refusal in any case of its words is.
#[test]
fn a_model_named_beside_another_class_s_refusal_or_echoed_is_not_a_configuration_refusal() {
    let plan = plan::plan(AdapterKind::Claude, &claude_declared()).unwrap();
    let echo = r#"{"type":"system","subtype":"init","model":"brokkr-probe-no-such-model"}"#;
    let config = |stdout: &str, stderr: &str| {
        let observed = Observed {
            bad_model: Trial::Observed(observation(Some(1), stdout, stderr)),
            ..observed(observation(Some(0), "", ""))
        };
        measure::reading(&plan, &observed, &[])
            .facts
            .refusals
            .config
    };
    let rows = [
        (
            "",
            "API Error: 503 provider temporarily unavailable for model brokkr-probe-no-such-model",
        ),
        (echo, "API Error: 503 provider temporarily unavailable"),
        (echo, "API Error: 401 invalid x-api-key"),
        (
            "",
            "model brokkr-probe-no-such-model not found: the provider is overloaded",
        ),
        ("", "model brokkr-probe-no-such-model not found (HTTP 529)"),
    ];
    assert_eq!(
        rows.map(|(stdout, stderr)| config(stdout, stderr)),
        rows.map(
            |(_, stderr)| Fact::unmeasured(not_the_configuration(&format!("exit 1: {stderr}")))
        )
    );
    let refused = "Model brokkr-probe-no-such-model Not Found (HTTP 404)";
    assert_eq!(
        config("", refused),
        Fact::measured(
            facts::Refusal {
                exit: Some(1),
                excerpt: refused.to_string(),
            },
            format!("exit 1: {refused}"),
        )
    );
}
