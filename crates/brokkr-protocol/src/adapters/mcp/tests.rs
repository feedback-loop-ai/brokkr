use super::super::tests::{
    claude_plan, enabled_input, executable, sealed_pair, version_preamble, Seal,
};
use super::super::{
    claude_launch, codex_command, dsh_argv, dsh_launch_with, CLAUDE_SHAPE, LANETALLY_SHAPE,
};
use super::*;
use serde_json::json;

/// dsh's engine-only home and its routes (U1c2), in their own file.
#[cfg(unix)]
mod dsh_home;

fn s(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

/// The stream shape every Claude-grammar launch leads with.
const HEAD: [&str; 4] = ["-p", "--output-format", "stream-json", "--verbose"];

/// The empty set's strict configuration, as an independent literal: the
/// flag and the document U0 measured (cell C03).
const EMPTY: [&str; 3] = [
    "--strict-mcp-config",
    "--mcp-config",
    r#"{"mcpServers":{}}"#,
];

/// The reason U0 measured for Codex, and the one SI2's scenarios use.
const CODEX_REASON: &str = "project, system and managed MCP configuration cannot be excluded";
const PROJECT_REASON: &str = "project MCP configuration cannot be excluded";

/// SI2's two site causes, as a launch refusal carries them.
fn unsupported_cause(provider: &str, reason: &str) -> McpRefusal {
    McpRefusal::Strict(StrictCause::Unsupported {
        provider: provider.into(),
        reason: reason.into(),
    })
}

fn unmeasured_cause(provider: &str, shape: &str) -> McpRefusal {
    McpRefusal::Strict(StrictCause::Unmeasured {
        provider: provider.into(),
        shape: shape.into(),
    })
}

/// `input` carrying the engine's isolation intent for `servers`, its cold,
/// replacement and resume assessments in that order.
fn intended(mut input: Value, servers: &str, [cold, replacement, resume]: [Value; 3]) -> Value {
    input[MCP_ISOLATION] = json!({
        "servers": servers, "cold": cold, "replacement": replacement, "resume": resume
    });
    input
}

fn by_hand(servers: &str, assessments: [Value; 3]) -> Value {
    intended(json!({"workdir": "/w"}), servers, assessments)
}

/// A cold shape measured and nothing else.
fn cold_only() -> [Value; 3] {
    [json!("measured"), json!("unmeasured"), json!("unmeasured")]
}

/// A Claude launch's input as dispatch seals it by `seal`: the engine's plan
/// denying both known powers and lowering no local list, and `extra` wholly
/// the engine's, its last `hands` arguments the box's hands.
fn sealed_claude(extra: &[String], hands: usize, seal: Seal) -> Value {
    let mut plan = claude_plan(&[], &[], &["WebSearch", "WebFetch"]);
    plan["local"] = json!([]);
    plan["hands"] = json!(hands);
    let mut input = json!({"workdir": "/w", "native_controls": plan});
    input["launch_arguments"] = json!({"authored": [], "managed": extra});
    sealed_pair(input, extra, seal)
}

/// `bin` followed by the stream head, then `rest`.
fn command(bin: &str, rest: &[Vec<String>]) -> Vec<String> {
    [vec![bin.to_string()], s(&HEAD), rest.concat()].concat()
}

/// A Claude shim that reports `version`, never the operator's installed CLI.
fn claude_shim(dir: &Path, version: &str) -> String {
    let banner = version_preamble(&format!("{version} (Claude Code)"));
    let bin = executable(dir, "claude", &format!("#!/bin/sh\n{banner}exit 1\n"));
    bin.to_str().unwrap().to_string()
}

/// The empty set is served cold as the seat's composed arguments followed by
/// exactly the strict flag and the empty document, for Claude and for the
/// LaneTally wrapper alike; the seat's model, effort and permission mode
/// stay where they stood. A launch carrying no intent is served as before.
#[test]
fn the_empty_set_is_served_cold_with_exactly_the_strict_flag_and_the_empty_document() {
    let extra = s(&[
        "--permission-mode",
        "acceptEdits",
        "--model",
        "haiku",
        "--effort",
        "low",
    ]);
    let input = by_hand("empty", cold_only());
    for (bin, shape) in [("claude", CLAUDE_SHAPE), ("lanetally", LANETALLY_SHAPE)] {
        let launch = claude_launch(bin, &extra, None, &input, shape, None).unwrap();
        assert_eq!(
            (launch.command, launch.rejoining, launch.refusal),
            (command(bin, &[extra.clone(), s(&EMPTY)]), None, None),
            "{bin}"
        );
    }
    let legacy = claude_launch(
        "claude",
        &extra,
        None,
        &json!({"workdir": "/w"}),
        CLAUDE_SHAPE,
        None,
    );
    assert_eq!(
        legacy.map(|launch| launch.command),
        Ok(command("claude", &[extra]))
    );
}

/// A rejoin keeps the isolated configuration and adds only its selector,
/// where its resume shape is measured. Where it is not, the offer is
/// declined before any probe and the launch is the cold replacement,
/// recorded with `restrictions-unavailable`, never as resume support. An
/// offered launch is judged by its replacement assessment, which every offer
/// can end as, and a launch offered nothing by its cold one: neither borrows
/// the other's measurement.
#[test]
fn a_rejoin_keeps_the_isolation_only_where_its_resume_shape_is_measured() {
    let dir = tempfile::tempdir().unwrap();
    let bin = claude_shim(&dir.path().canonicalize().unwrap(), "2.1.287");
    let enabled = enabled_input(CLAUDE_SHAPE, "2.1.287", Path::new("/w"));
    let extra = s(&["--model", "haiku"]);
    let session = "019c4b7e-0000-7000-8000-000000000001";
    let launch_offered = |offered: Option<&str>, replacement: Value, resume: Value| {
        let assessments = [json!("unmeasured"), replacement, resume];
        let input = intended(enabled.clone(), "empty", assessments);
        claude_launch(&bin, &extra, offered, &input, CLAUDE_SHAPE, None)
    };
    let launch = |replacement, resume| launch_offered(Some(session), replacement, resume);
    let warm = launch(json!("measured"), json!("measured")).unwrap();
    assert_eq!(
        (warm.command, warm.rejoining.as_deref(), warm.refusal),
        (
            command(&bin, &[extra.clone(), s(&EMPTY), s(&["--resume", session])]),
            Some(session),
            None
        )
    );
    for resume in [json!("unmeasured"), json!({"unsupported": PROJECT_REASON})] {
        let cold = launch(json!("measured"), resume.clone()).unwrap();
        assert_eq!(
            (
                cold.command,
                cold.rejoining,
                cold.refusal,
                cold.harness_version
            ),
            (
                command(&bin, &[extra.clone(), s(&EMPTY)]),
                None,
                Some("restrictions-unavailable"),
                None
            ),
            "{resume}"
        );
    }
    let refused = |offered, replacement, refusal: McpRefusal| {
        assert_eq!(
            launch_offered(offered, replacement, json!("measured")).map(|plan| plan.command),
            Err(refusal.at_launch())
        );
    };
    refused(
        Some(session),
        json!("unmeasured"),
        unmeasured_cause("claude", "replacement"),
    );
    refused(
        Some(session),
        json!({"unsupported": PROJECT_REASON}),
        unsupported_cause("claude", PROJECT_REASON),
    );
    refused(None, json!("measured"), unmeasured_cause("claude", "cold"));
}

/// Every launch builder reads the intent before any provider work: a
/// measured limitation is its own cause with its reason, missing evidence
/// names the shape, and a measured claim for a harness the engine builds no
/// mechanism for — Codex, which no U0 candidate qualified — is missing
/// evidence, never Claude's result and never ambient configuration. dsh's
/// shapes are judged the same way, ahead of its route (U1c2).
#[test]
fn each_unqualified_shape_refuses_with_its_exact_cause() {
    let unsupported = |reason: &str| json!({"unsupported": reason});
    let refused = |provider: &'static str, cold: Value| -> Result<(), String> {
        let input = by_hand("empty", [cold, json!("unmeasured"), json!("unmeasured")]);
        let extra = s(&["--model", "m"]);
        match provider {
            "codex" => codex_command("codex", &extra, "/w", None, &input).map(drop),
            "dsh" => dsh_argv(&extra, &input, false).map(drop),
            "lanetally" => {
                claude_launch("lanetally", &extra, None, &input, LANETALLY_SHAPE, None).map(drop)
            }
            _ => claude_launch("claude", &extra, None, &input, CLAUDE_SHAPE, None).map(drop),
        }
    };
    let unmeasured = |provider| unmeasured_cause(provider, "cold");
    for (provider, cold, refusal) in [
        (
            "claude",
            unsupported(PROJECT_REASON),
            unsupported_cause("claude", PROJECT_REASON),
        ),
        ("claude", json!("unmeasured"), unmeasured("claude")),
        ("lanetally", json!("unmeasured"), unmeasured("lanetally")),
        (
            "codex",
            unsupported(CODEX_REASON),
            unsupported_cause("codex", CODEX_REASON),
        ),
        ("codex", json!("measured"), unmeasured("codex")),
        ("dsh", json!("unmeasured"), unmeasured("dsh")),
        (
            "dsh",
            unsupported(PROJECT_REASON),
            unsupported_cause("dsh", PROJECT_REASON),
        ),
    ] {
        assert_eq!(
            refused(provider, cold.clone()),
            Err(refusal.at_launch()),
            "{provider}, {cold}"
        );
    }
    // An offered Codex or dsh launch is judged by its replacement shape, not
    // by its measured cold one.
    let extra = s(&["--model", "m"]);
    let codex = by_hand(
        "empty",
        [
            json!("measured"),
            unsupported(CODEX_REASON),
            json!("measured"),
        ],
    );
    assert_eq!(
        codex_command("codex", &extra, "/w", Some("019c"), &codex).map(drop),
        Err(unsupported_cause("codex", CODEX_REASON).at_launch())
    );
    // Through the launch itself: the refusal precedes the composite, the
    // route claim, the home's staging and any probe.
    for (replacement, refusal) in [
        (json!("unmeasured"), unmeasured_cause("dsh", "replacement")),
        (
            unsupported(PROJECT_REASON),
            unsupported_cause("dsh", PROJECT_REASON),
        ),
    ] {
        let dsh = by_hand(
            "empty",
            [json!("measured"), replacement.clone(), json!("measured")],
        );
        let launch = dsh_launch_with("dsh", &extra, "/w", Some("019c"), &dsh, |_| {
            unreachable!("the composite is read after the refusal")
        });
        assert_eq!(launch.map(drop), Err(refusal.at_launch()), "{replacement}");
    }
}

/// The intent's server set must be the one the sealed inputs declare, and
/// the empty set is served beside no other MCP configuration: the hands set
/// without sealed hands, the empty set beside sealed hands or unreadable
/// inputs, and an empty set whose arguments already carry a strict flag or
/// a document each refuse. The sealed hands set is served exactly as the
/// same launch without an intent, its document the adapter fragment's.
#[test]
fn a_server_set_its_sealed_inputs_do_not_declare_is_refused() {
    let isolated_as = |servers: &str, extra: &[&str], inputs: Option<Value>| {
        let mut input = by_hand(servers, cold_only());
        if let Some(inputs) = inputs {
            input[SERVING_INPUTS] = inputs;
        }
        isolated("claude", &Edge::new(&input), &s(extra), false).map(|isolated| isolated.argv)
    };
    let boxed = SealedServing {
        spec: Some(crate::hands::HandsSpec::default()),
        ..Default::default()
    }
    .value();
    let not_sealed = Err(McpRefusal::NotSealed { provider: "claude" });
    for (case, servers, extra, inputs) in [
        ("hands, none sealed", "hands", &[][..], None),
        (
            "hands, unreadable",
            "hands",
            &[][..],
            Some(json!("garbage")),
        ),
        (
            "empty, unreadable",
            "empty",
            &[][..],
            Some(json!("garbage")),
        ),
        ("empty, hands sealed", "empty", &[][..], Some(boxed.clone())),
        ("empty, strict", "empty", &["--strict-mcp-config"][..], None),
        (
            "empty, document",
            "empty",
            &["--mcp-config=a.json"][..],
            None,
        ),
    ] {
        assert_eq!(isolated_as(servers, extra, inputs), not_sealed, "{case}");
    }
    assert_eq!(
        isolated_as("hands", &["--model", "m"], Some(boxed)),
        Ok(s(&["--model", "m"]))
    );
    // The empty set's flag and document must be placed as options of their
    // own: after a terminator, as a dangling option's value or after an
    // argument the grammar does not model, they would not be.
    for extra in [
        &["--model", "m", "--"][..],
        &["--model"][..],
        &["--frobnicate"][..],
    ] {
        assert_eq!(
            isolated_as("empty", extra, None),
            Err(McpRefusal::Unplaced { provider: "claude" }),
            "{extra:?}"
        );
    }
    assert_eq!(
        isolated_as("empty", &["--model", "m"], None),
        Ok([s(&["--model", "m"]), s(&EMPTY)].concat())
    );
    let terminated = by_hand("empty", cold_only());
    assert_eq!(
        claude_launch("claude", &s(&["--"]), None, &terminated, CLAUDE_SHAPE, None)
            .map(|plan| plan.command),
        Err(McpRefusal::Unplaced { provider: "claude" }.at_launch())
    );

    let mut seal = Seal::authored(0);
    let extra = seal.hands(&[
        "--tools",
        "",
        "--strict-mcp-config",
        "--mcp-config",
        "{hands_mcp_json}",
        "--allowedTools",
        "mcp__brokkr__workspace",
    ]);
    let input = sealed_claude(&extra, extra.len(), seal);
    let launch = |input: &Value| {
        claude_launch("claude", &extra, None, input, CLAUDE_SHAPE, None).map(|plan| plan.command)
    };
    let served = launch(&intended(input.clone(), "hands", cold_only()));
    assert_eq!(served, launch(&input));
    // The document as an independent literal (NC6), only the test's own
    // executable substituted.
    let exe = std::env::current_exe().unwrap();
    let serve = r#"["hands","serve","--workdir","/w","--spec","{\"binds\":[],\"kind\":\"workspace\",\"network\":false}"]"#;
    let document = format!(
        r#"{{"mcpServers":{{"brokkr":{{"args":{serve},"command":"{}"}}}}}}"#,
        exe.to_str().unwrap()
    );
    let mcp = s(&["--strict-mcp-config", "--mcp-config", &document]);
    assert_eq!(
        served.map(|argv| argv[HEAD.len() + 3..HEAD.len() + 6].to_vec()),
        Ok(mcp)
    );
}

/// The intent is read closed: an unknown member or word and an absent member
/// are unreadable, and a measured reason must be one bounded line.
#[test]
fn the_intent_is_read_closed_and_its_reasons_are_bounded() {
    let read = |intent: Value| {
        let mut input = json!({});
        input[MCP_ISOLATION] = intent;
        Isolation::read(&input).map(|isolation| isolation.is_some())
    };
    let with_reason = |at: &str, reason: String| {
        let mut intent = json!({
            "servers": "empty", "cold": "measured", "replacement": "measured",
            "resume": "unmeasured"
        });
        intent[at] = json!({"unsupported": reason});
        read(intent)
    };
    let all = |servers: &str, cold: Value| json!({"servers": servers, "cold": cold, "replacement": "measured", "resume": "measured"});
    let mut extended = all("empty", json!("measured"));
    extended["brokers"] = json!([]);
    for intent in [
        extended,
        all("brokers", json!("measured")),
        json!({"servers": "empty", "cold": "measured", "resume": "measured"}),
        all("empty", json!("inherited")),
        all("empty", json!({"measured": "x"})),
    ] {
        assert_eq!(
            read(intent.clone()),
            Err(McpRefusal::Unreadable),
            "{intent}"
        );
    }
    // Each shape's reason is held to the bound on its own.
    for at in ["cold", "replacement", "resume"] {
        for reason in [String::new(), "r".repeat(401), "two\nlines".into()] {
            assert_eq!(
                with_reason(at, reason.clone()),
                Err(McpRefusal::UnboundedReason),
                "{at}: {reason:?}"
            );
        }
        assert_eq!(with_reason(at, "r".repeat(400)), Ok(true), "{at}");
    }
    assert_eq!(Isolation::read(&json!({})), Ok(None));
}

/// A sealed launch's final check recomposes its command from the sealed
/// inputs alone, which do not yet carry the empty set (U1g threads it), so
/// the empty configuration a driver adds is refused there rather than
/// served unchecked; the same sealed launch without an intent is served.
#[test]
fn a_sealed_empty_set_is_refused_by_the_final_check_until_it_is_sealed() {
    let input = sealed_claude(&[], 0, Seal::authored(0));
    let launch = |input: &Value| {
        claude_launch("claude", &[], None, input, CLAUDE_SHAPE, None).map(|plan| plan.command)
    };
    let denied = s(&["--disallowedTools", "WebSearch,WebFetch"]);
    assert_eq!(launch(&input), Ok(command("claude", &[denied])));
    assert_eq!(
        launch(&intended(input, "empty", cold_only())),
        Err(
            "refusing to invoke the agent CLI: the final command of harness 'claude' carries \
             '--strict-mcp-config' its sealed plan does not compose, however its serving builder \
             rebuilds it; a complete command is parsed back before its spawn and must express \
             exactly the capability state its sealed plan records, so it is refused rather than \
             spawned (operator ruling 2 of 2026-09-23; design D6)"
                .to_string()
        )
    );
}

/// The module's operator text, pinned once for every refusal. SI2's two
/// site causes read the same alone, as the compile pass states them, and
/// after the driver's prefix.
#[test]
fn each_refusal_reads_as_the_operator_sees_it() {
    let lead = "refusing to invoke the agent CLI: ";
    for (cause, text) in [
        (
            crate::adapters::StrictCause::Unsupported {
                provider: "codex".into(),
                reason: CODEX_REASON.into(),
            },
            "provider 'codex' cannot exclude ambient MCP configuration (project, system and \
             managed MCP configuration cannot be excluded)",
        ),
        (
            crate::adapters::StrictCause::Unmeasured {
                provider: "claude".into(),
                shape: "boxed-workspace".into(),
            },
            "provider 'claude' has no measured strict MCP configuration for 'boxed-workspace'",
        ),
    ] {
        assert_eq!(cause.to_string(), text);
        assert_eq!(McpRefusal::from(cause).at_launch(), format!("{lead}{text}"));
    }
    for (refusal, text) in [
        (
            McpRefusal::Unreadable,
            "the engine's MCP isolation intent cannot be read, so no strict MCP configuration is \
             built",
        ),
        (
            McpRefusal::UnboundedReason,
            "the engine's MCP isolation intent carries a reason that is not one bounded line",
        ),
        (
            McpRefusal::NotSealed {
                provider: "lanetally",
            },
            "provider 'lanetally' final MCP configuration is not the engine's sealed server set",
        ),
        (
            McpRefusal::Unplaced { provider: "claude" },
            "provider 'claude' arguments do not place the engine's MCP configuration as options \
             of their own: a terminator, a dangling value or an argument the grammar does not \
             model stands before it",
        ),
        (
            McpRefusal::UnmeasuredRoute {
                route: "openai".into(),
            },
            "provider 'dsh' has no measured strict MCP configuration on route 'openai'",
        ),
        (
            McpRefusal::Unpinned,
            "provider 'dsh' has no measured strict MCP configuration for a seat that pins no \
             `--model`: the route it would run is the profile's unnamed default",
        ),
        (
            McpRefusal::RouteEntry {
                route: "deepseek-official".into(),
                row: RouteRow::Shipped,
            },
            "provider 'dsh' route 'deepseek-official' was measured on dsh-base's shipped row \
             alone, and the seat's overlay carries a provider entry for it",
        ),
        (
            McpRefusal::RouteEntry {
                route: "meta".into(),
                row: RouteRow::Overlay,
            },
            "provider 'dsh' route 'meta' was measured with its validated provider entry, and the \
             seat carries no route overlay",
        ),
    ] {
        assert_eq!(refusal.at_launch(), format!("{lead}{text}"));
    }
}
