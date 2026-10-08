use super::super::tests::{
    claude_plan, enabled_input, executable, sealed_pair, version_preamble, Seal,
};
use super::super::{
    claude_launch, codex_command, dsh_argv, dsh_launch_with, CLAUDE_SHAPE, DSH_SHAPE,
    LANETALLY_SHAPE,
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

/// `input` whose serving inputs, its own or the default ones, seal the
/// engine's isolation intent for `servers`, its cold, replacement and resume
/// assessments in that order, the last for each resume shape a launch here
/// rejoins as, under the fence `fence` (U1g2).
fn fenced(mut input: Value, servers: &str, assessments: [Value; 3], fence: &str) -> Value {
    let [cold, replacement, resume] = assessments;
    if input.get(SERVING_INPUTS).is_none() {
        input[SERVING_INPUTS] = SealedServing::default().value();
    }
    let resume: serde_json::Map<String, Value> = [CLAUDE_SHAPE, LANETALLY_SHAPE, DSH_SHAPE]
        .into_iter()
        .map(|shape| (shape.to_string(), resume.clone()))
        .collect();
    input[SERVING_INPUTS]["isolation"] = json!({
        "servers": servers, "cold": cold, "replacement": replacement, "resume": resume,
        "fence": fence,
    });
    input
}

/// [`fenced`] past the lifted fence, where SI2 refuses.
fn intended(input: Value, servers: &str, assessments: [Value; 3]) -> Value {
    fenced(input, servers, assessments, "lifted")
}

/// The serving inputs dispatch seals for a fixture's `dialect`, `pins` and
/// `spec` (U1g2): its intent names the set they declare, measured on no
/// shape, under the standing fence, so the launch is served as before.
pub(in crate::adapters) fn dispatched(
    dialect: crate::native_controls::SealedDialect,
    pins: Vec<String>,
    spec: Option<crate::hands::HandsSpec>,
) -> Value {
    let mut sealed = SealedServing {
        dialect,
        pins,
        spec,
        isolation: SealedIsolation::default(),
    };
    sealed.isolation.servers = sealed_servers(&sealed);
    sealed.value()
}

/// Sealed serving inputs declaring the default typed hands under `stands`.
fn hands_under(stands: SealedBoundary) -> Value {
    let dialect = crate::native_controls::SealedDialect {
        stands: Some(stands),
        ..Default::default()
    };
    dispatched(
        dialect,
        Vec::new(),
        Some(crate::hands::HandsSpec::default()),
    )
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
    for provider in ["claude", "lanetally"] {
        let isolated = isolated(provider, &Edge::new(&input), &extra, false).unwrap();
        assert_eq!(
            (isolated.argv, isolated.isolation.is_some()),
            ([extra.clone(), s(&EMPTY)].concat(), true),
            "{provider}"
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
    // A sealed launch, so its final check rebuilds the isolated command.
    let mut enabled = sealed_claude(&[], 0, Seal::authored(0));
    let assessment = enabled_input(CLAUDE_SHAPE, "2.1.287", Path::new("/w"));
    for key in ["boundary", "hands", "resume_context"] {
        enabled[key] = assessment[key].clone();
    }
    let (extra, denied) = (Vec::new(), s(&["--disallowedTools", "WebSearch,WebFetch"]));
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
            command(
                &bin,
                &[denied.clone(), s(&EMPTY), s(&["--resume", session])]
            ),
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
                command(&bin, &[denied.clone(), s(&EMPTY)]),
                None,
                Some("restrictions-unavailable"),
                None
            ),
            "{resume}"
        );
    }
    // A rejoin is judged by its own resume shape's entry: another shape
    // recorded beside it neither declines a measured rejoin nor lends a
    // declined one its measurement.
    let mixed = |own: &str, other: &str| {
        let assessments = [json!("unmeasured"), json!("measured"), json!("measured")];
        let mut input = intended(enabled.clone(), "empty", assessments);
        input[SERVING_INPUTS]["isolation"]["resume"] =
            json!({"work-site": other, CLAUDE_SHAPE: own});
        let plan = claude_launch(&bin, &extra, Some(session), &input, CLAUDE_SHAPE, None).unwrap();
        (plan.rejoining, plan.refusal)
    };
    assert_eq!(
        mixed("measured", "unmeasured"),
        (Some(session.to_string()), None)
    );
    assert_eq!(
        mixed("unmeasured", "measured"),
        (None, Some("restrictions-unavailable"))
    );
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
/// without sealed hands, the empty set beside the box's sealed hands, and an
/// empty set whose arguments already carry a strict flag or a document each
/// refuse, whichever way the fence stands; inputs that cannot be read carry
/// no intent at all. The sealed hands set is served exactly as the same
/// launch dispatched under the standing fence, its document the adapter
/// fragment's.
#[test]
fn a_server_set_its_sealed_inputs_do_not_declare_is_refused() {
    let isolated_as = |servers: &str, extra: &[&str], inputs: Option<Value>| {
        let mut base = json!({"workdir": "/w"});
        if let Some(inputs) = inputs {
            base[SERVING_INPUTS] = inputs;
        }
        let input = intended(base, servers, cold_only());
        isolated("claude", &Edge::new(&input), &s(extra), false).map(|isolated| isolated.argv)
    };
    let boxed = hands_under(SealedBoundary::Namespace);
    let not_sealed = Err(McpRefusal::NotSealed { provider: "claude" });
    let unreadable = json!({"workdir": "/w", SERVING_INPUTS: "garbage"});
    assert_eq!(
        isolated("claude", &Edge::new(&unreadable), &[], false).map(|isolated| isolated.argv),
        Err(McpRefusal::Unreadable)
    );
    for (case, servers, extra, inputs) in [
        ("hands, none sealed", "hands", &[][..], None),
        ("no model surface", "no-model-surface", &[][..], None),
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
    // Tampering refuses under the standing fence as past the lifted one.
    for (servers, inputs) in [("hands", None), ("empty", Some(boxed_inputs()))] {
        let mut input = json!({"workdir": "/w"});
        if let Some(inputs) = inputs {
            input[SERVING_INPUTS] = inputs;
        }
        let standing = fenced(input, servers, cold_only(), "standing");
        assert_eq!(
            isolated("claude", &Edge::new(&standing), &[], false).map(|isolated| isolated.argv),
            not_sealed,
            "{servers}"
        );
    }
}

/// Sealed serving inputs declaring the box's hands, as dispatch seals a
/// site with hands under `namespace`.
pub(super) fn boxed_inputs() -> Value {
    hands_under(SealedBoundary::Namespace)
}

/// The intent is read closed: an unknown member or word and an absent member
/// are unreadable, and a measured reason must be one bounded line. Serving
/// inputs that carry no intent are unreadable too, never served as before.
#[test]
fn the_intent_is_read_closed_and_its_reasons_are_bounded() {
    let read = |intent: Option<Value>| {
        let mut input = json!({SERVING_INPUTS: SealedServing::default().value()});
        match intent {
            Some(intent) => input[SERVING_INPUTS]["isolation"] = intent,
            None => drop(
                input[SERVING_INPUTS]
                    .as_object_mut()
                    .unwrap()
                    .remove("isolation"),
            ),
        }
        intent_of(&input).map(|isolation| isolation.is_some())
    };
    let with_reason = |at: &str, reason: String| {
        let mut intent = json!({
            "servers": "empty", "cold": "measured", "replacement": "measured",
            "resume": {"work-site": "measured"}, "fence": "standing"
        });
        *intent.pointer_mut(at).unwrap() = json!({"unsupported": reason});
        read(Some(intent))
    };
    let all = |servers: &str, cold: Value| json!({"servers": servers, "cold": cold, "replacement": "measured", "resume": {"work-site": "measured"}, "fence": "lifted"});
    let mut extended = all("empty", json!("measured"));
    extended["brokers"] = json!([]);
    let (mut unfenced, mut opened) = (all("empty", json!("measured")), all("empty", json!("x")));
    unfenced.as_object_mut().unwrap().remove("fence");
    opened["cold"] = json!("measured");
    opened["fence"] = json!("open");
    // A resume assessment names its shape: one unnamed, or a named one in
    // an unknown word, is unreadable.
    let (mut unnamed, mut inherited) = (opened.clone(), opened.clone());
    for intent in [&mut unnamed, &mut inherited] {
        intent["fence"] = json!("lifted");
    }
    unnamed["resume"] = json!("measured");
    inherited["resume"]["work-site"] = json!("inherited");
    assert_eq!(read(None), Err(McpRefusal::Unreadable));
    for intent in [
        extended,
        all("brokers", json!("measured")),
        json!({"servers": "empty", "cold": "measured", "resume": {}, "fence": "lifted"}),
        all("empty", json!("inherited")),
        all("empty", json!({"measured": "x"})),
        unfenced,
        opened,
        unnamed,
        inherited,
    ] {
        assert_eq!(
            read(Some(intent.clone())),
            Err(McpRefusal::Unreadable),
            "{intent}"
        );
    }
    // Each shape's reason is held to the bound on its own.
    for at in ["/cold", "/replacement", "/resume/work-site"] {
        for reason in [String::new(), "r".repeat(401), "two\nlines".into()] {
            assert_eq!(
                with_reason(at, reason.clone()),
                Err(McpRefusal::UnboundedReason),
                "{at}: {reason:?}"
            );
        }
        assert_eq!(with_reason(at, "r".repeat(400)), Ok(true), "{at}");
    }
    assert_eq!(intent_of(&json!({})), Ok(None));
}

/// The intent [`intent`] reads from `input`, `None` where it seals none.
fn intent_of(input: &Value) -> Result<Option<SealedIsolation>, McpRefusal> {
    intent(&Edge::new(input)).map(|sealed| sealed.map(|(_, isolation)| isolation.clone()))
}

/// U1g2: a sealed launch is served the empty set its intent built, and its
/// final check rebuilds that configuration behind the composition, cold and
/// on a rejoin. A sealed launch whose inputs carry no intent refuses before
/// any provider work, and a driver run by hand with none is served as before.
#[test]
fn a_sealed_empty_set_is_served_as_its_final_check_rebuilds_it() {
    let sealed = sealed_claude(&[], 0, Seal::authored(0));
    let denied = s(&["--disallowedTools", "WebSearch,WebFetch"]);
    let measured = [json!("measured"), json!("measured"), json!("measured")];
    let input = intended(sealed.clone(), "empty", measured);
    let launch = claude_launch("claude", &[], None, &input, CLAUDE_SHAPE, None);
    assert_eq!(
        launch.map(|plan| plan.command),
        Ok(command("claude", &[denied.clone(), s(&EMPTY)]))
    );
    let session = "019c4b7e-0000-7000-8000-000000000001";
    let edge = Edge::new(&input);
    let rejoin = isolated("claude", &edge, &[], true).map(|isolated| isolated.argv);
    assert_eq!(rejoin, Ok(s(&EMPTY)));
    let chosen = crate::native_controls::Serving {
        program: "claude",
        workdir: "/w",
        session: Some(session),
        ..Default::default()
    };
    let rejoined = command("claude", &[denied, s(&EMPTY), s(&["--resume", session])]);
    assert_eq!(
        served("claude", rejoined.clone(), &[], &edge, chosen),
        Ok(rejoined)
    );
    let mut absent = sealed;
    absent[SERVING_INPUTS]
        .as_object_mut()
        .unwrap()
        .remove("isolation");
    let launch = claude_launch("claude", &[], None, &absent, CLAUDE_SHAPE, None);
    assert_eq!(
        launch.map(|plan| plan.command),
        Err(McpRefusal::Unreadable.at_launch())
    );
    let by_hand = claude_launch("claude", &[], None, &json!({}), CLAUDE_SHAPE, None);
    assert_eq!(by_hand.map(|plan| plan.command), Ok(command("claude", &[])));
}

/// SI2's final configuration check at the serving boundary (U1g2): a final
/// command whose strict flag or document was removed, beside which another
/// MCP source was appended, or whose built document changed refuses with
/// SI2's exact cause before any server or model starts, for the empty set
/// and for the box's hands set alike.
#[test]
fn a_final_command_whose_mcp_configuration_departs_from_the_built_one_refuses() {
    let chosen = crate::native_controls::Serving {
        program: "claude",
        workdir: "/w",
        ..Default::default()
    };
    let not_sealed = Err(McpRefusal::NotSealed { provider: "claude" }.at_launch());
    let denied = s(&["--disallowedTools", "WebSearch,WebFetch"]);
    let input = intended(
        sealed_claude(&[], 0, Seal::authored(0)),
        "empty",
        cold_only(),
    );
    let edge = Edge::new(&input);
    assert_eq!(
        isolated("claude", &edge, &[], false).map(|i| i.argv),
        Ok(s(&EMPTY))
    );
    let serve = |rest: &[&[&str]]| {
        let rest: Vec<Vec<String>> = rest.iter().map(|part| s(part)).collect();
        served(
            "claude",
            command("claude", &[denied.clone(), rest.concat()]),
            &[],
            &edge,
            chosen,
        )
    };
    let ambient = ["--mcp-config", "/home/operator/.claude.json"];
    for (case, rest) in [
        ("strict removed", &[&EMPTY[1..]][..]),
        ("document removed", &[&EMPTY[..1]][..]),
        ("all removed", &[][..]),
        ("another source appended", &[&EMPTY[..], &ambient[..]][..]),
        (
            "document changed",
            &[&EMPTY[..2], &[r#"{"mcpServers":{"x":{"command":"x"}}}"#]][..],
        ),
    ] {
        assert_eq!(serve(rest), not_sealed, "{case}");
    }
    assert_eq!(
        serve(&[&EMPTY]),
        Ok(command("claude", &[denied.clone(), s(&EMPTY)]))
    );
    // The box's hands set: its strict flag and document are the adapter
    // fragment's, which the same check requires.
    let mut seal = Seal::authored(0);
    let fragment = [
        "--tools",
        "",
        "--strict-mcp-config",
        "--mcp-config",
        "{hands_mcp_json}",
        "--allowedTools",
        "mcp__brokkr__workspace",
    ];
    let extra = seal.hands(&fragment);
    let input = intended(
        sealed_claude(&extra, extra.len(), seal),
        "hands",
        cold_only(),
    );
    let launch = claude_launch("claude", &extra, None, &input, CLAUDE_SHAPE, None);
    let served_hands = launch.map(|plan| plan.command).unwrap();
    let edge = Edge::new(&input);
    isolated("claude", &edge, &extra, false).unwrap();
    assert_eq!(
        served("claude", served_hands.clone(), &extra, &edge, chosen),
        Ok(served_hands.clone())
    );
    let mut stripped = served_hands.clone();
    stripped.retain(|part| part != "--strict-mcp-config");
    let mut appended = served_hands.clone();
    appended.extend(s(&ambient));
    for (case, command) in [("strict removed", stripped), ("appended", appended)] {
        assert_eq!(
            served("claude", command, &extra, &edge, chosen),
            not_sealed,
            "{case}"
        );
    }
}

/// Under `harness` the harness's own sandbox carries an office's hands and
/// under `open` nothing does, so typed hands sealed under either are served
/// the empty set, its strict flag and empty document, and a hands set there
/// is not the sealed one (U1g2; the U1f council's carried case). Under a
/// boundary that boxes them, the same hands are the hands set.
#[test]
fn hands_sealed_unboxed_are_served_the_empty_set() {
    use SealedBoundary::{Container, Harness, Namespace, Open, Seatbelt};
    let isolated_as = |stands: SealedBoundary, servers: &str| {
        let input = json!({"workdir": "/w", SERVING_INPUTS: hands_under(stands)});
        let input = intended(input, servers, cold_only());
        isolated("claude", &Edge::new(&input), &s(&["--model", "m"]), false)
            .map(|isolated| isolated.argv)
    };
    let not_sealed = Err(McpRefusal::NotSealed { provider: "claude" });
    for stands in [Harness, Open] {
        let empty = Ok([s(&["--model", "m"]), s(&EMPTY)].concat());
        assert_eq!(isolated_as(stands, "empty"), empty, "{stands:?}");
        assert_eq!(isolated_as(stands, "hands"), not_sealed, "{stands:?}");
    }
    for stands in [Namespace, Seatbelt, Container] {
        assert_eq!(
            isolated_as(stands, "hands"),
            Ok(s(&["--model", "m"])),
            "{stands:?}"
        );
        assert_eq!(isolated_as(stands, "empty"), not_sealed, "{stands:?}");
    }
}

/// While the MCP compile fence stands (U1g1's switch, which U9b lifts), a
/// shape SI2 does not admit is served as before rather than refused: no
/// isolated configuration is built, a rejoin is offered as it was, and dsh
/// is served from the operator's home. Past the lifted fence the same
/// intents refuse with SI2's exact causes
/// ([`each_unqualified_shape_refuses_with_its_exact_cause`]).
#[test]
fn a_standing_fence_serves_an_unadmitted_shape_as_before() {
    let extra = s(&["--model", "m"]);
    let unsupported = json!({"unsupported": PROJECT_REASON});
    let standing = |cold: Value| {
        let assessments = [cold, json!("unmeasured"), json!("unmeasured")];
        fenced(json!({"workdir": "/w"}), "empty", assessments, "standing")
    };
    for (provider, cold) in [
        ("claude", json!("unmeasured")),
        ("claude", unsupported.clone()),
        ("lanetally", json!("unmeasured")),
        ("codex", json!("measured")),
        ("codex", unsupported),
        ("dsh", json!("unmeasured")),
    ] {
        let isolated = isolated(provider, &Edge::new(&standing(cold.clone())), &extra, false);
        let isolated = isolated.unwrap();
        assert_eq!(
            (isolated.argv, isolated.isolation),
            (extra.clone(), None),
            "{provider}, {cold}"
        );
    }
    // A rejoin offered under an unmeasured replacement is offered as before.
    let input = standing(json!("measured"));
    let offered = isolated("claude", &Edge::new(&input), &extra, true).unwrap();
    assert_eq!(
        (
            offered.argv.clone(),
            offered.resumes("claude", CLAUDE_SHAPE)
        ),
        (extra.clone(), true)
    );
    // An admitted dsh shape on an unmeasured route, or pinning none, is
    // served from the operator's home while the fence stands.
    let measured = [json!("measured"), json!("measured"), json!("measured")];
    let input = fenced(json!({}), "empty", measured, "standing");
    let built = isolated("dsh", &Edge::new(&input), &[], false).unwrap();
    for route in [Some("spark"), None] {
        assert_eq!(
            built.dsh(route, false),
            Ok(DshIsolation::Operator),
            "{route:?}"
        );
    }
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
