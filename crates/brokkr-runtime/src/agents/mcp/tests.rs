//! U1b (decision 0065 slice two; SI1, SI2, MB2): adapter MCP facts are
//! typed through the real adapter loader, each axis and shape on its own.

use super::*;
use crate::agents::{Adapters, LibraryError};
use serde_json::json;

/// Load `body` through the real loader as the only adapter, in a fresh
/// canonicalised temporary root, so no row sees another row's file.
fn load(body: &Value) -> Result<Adapters, LibraryError> {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let file = format!("{}.json", body["provider"].as_str().unwrap());
    std::fs::write(root.join(file), serde_json::to_vec_pretty(body).unwrap()).unwrap();
    Adapters::load(&root)
}

fn support(body: &Value) -> McpSupport {
    let adapters = load(body).unwrap();
    let mcp = adapters.providers().next().unwrap().mcp.clone();
    mcp
}

fn problem(body: &Value) -> McpError {
    match load(body) {
        Err(LibraryError::Mcp(McpRefusal { problem, .. })) => problem,
        other => panic!("expected an 'mcp' refusal, got {other:?}"),
    }
}

/// An adapter for `provider`, run by harness `harness`, whose binary is
/// `binary`, declaring `mcp`. Boxed hands are declared, harness hands and
/// resume shapes are not.
fn adapter(provider: &str, harness: &str, binary: &str, mcp: Value) -> Value {
    json!({
        "provider": provider, "binary": binary,
        "driver": ["{brokkr}", "driver", harness, "--"],
        "models": {}, "model_flag": "unsupported", "efforts": [], "effort_flag": "unsupported",
        "tool_permissions": "unsupported", "mcp": mcp,
        "hands": {"workspace": ["--mcp-config", "{hands_mcp_json}"]},
    })
}

fn claude(mcp: Value) -> Value {
    adapter("claude", "claude", "claude", mcp)
}

/// Measured under `harness` on `binary`, version 2.1.287, on Linux.
fn on(harness: &str, binary: &str) -> Value {
    json!({"harness": harness, "binary": binary, "version": "2.1.287", "host": "linux"})
}

/// One shape entry, measured `on`, with its four axes.
fn entry(invocation: Value, hands: &str, on: Value, axes: [Value; 4]) -> Value {
    let [ambient, native_write, store_read, process_read] = axes;
    json!({
        "invocation": invocation, "hands": hands, "measured_on": on,
        "ambient": ambient, "native_write": native_write,
        "store_read": store_read, "process_read": process_read,
    })
}

fn measured(evidence: &str) -> Value {
    json!({ "measured": evidence })
}

fn axes() -> [Value; 4] {
    [measured("a"), measured("b"), measured("c"), measured("d")]
}

fn facts(shapes: Value) -> Value {
    json!({"carriage": measured("C05 engine sentinel called"), "shapes": shapes})
}

fn shape(invocation: McpInvocation, hands: McpHands) -> McpShape {
    McpShape {
        invocation,
        hands,
        host: McpHost::Linux,
    }
}

fn proved(evidence: &str) -> McpAxis {
    McpAxis::Measured {
        evidence: evidence.to_string(),
    }
}

/// The same axis on all four, built here and not by the module.
fn every(axis: &McpAxis) -> McpIsolation {
    McpIsolation {
        ambient: axis.clone(),
        native_write: axis.clone(),
        store_read: axis.clone(),
        process_read: axis.clone(),
    }
}

/// Each shape reads its own four axes; a shape no entry names — another
/// invocation, the same invocation under other hands or on the other
/// host — reads `Unmeasured(Absent)` on every axis, never a neighbour's.
#[test]
fn each_shape_reads_its_own_axes_and_an_absent_shape_is_unmeasured() {
    let support = support(&claude(facts(json!([entry(
        json!("cold"),
        "boxed",
        on("claude", "claude"),
        [
            measured("C10 ambient"),
            json!({"unsupported": "C10 write outside the worktree"}),
            measured("C10 store absent in the box"),
            json!({"unmeasured": "no process canary planted"}),
        ],
    )]))));
    let cold = shape(McpInvocation::Cold, McpHands::Boxed);
    let macos = McpShape {
        host: McpHost::Macos,
        ..cold.clone()
    };
    let absent = every(&McpAxis::Unmeasured(McpUnmeasured::Absent));
    let observed = [
        support.isolation(&cold),
        support.isolation(&shape(McpInvocation::Cold, McpHands::NoHands)),
        support.isolation(&shape(McpInvocation::Replacement, McpHands::Boxed)),
        support.isolation(&shape(
            McpInvocation::Resume("work-site".to_string()),
            McpHands::Boxed,
        )),
        support.isolation(&macos),
    ];
    let expected = [
        McpIsolation {
            ambient: proved("C10 ambient"),
            native_write: McpAxis::Unsupported {
                reason: "C10 write outside the worktree".to_string(),
            },
            store_read: proved("C10 store absent in the box"),
            process_read: McpAxis::Unmeasured(McpUnmeasured::Declared(
                "no process canary planted".to_string(),
            )),
        },
        absent.clone(),
        absent.clone(),
        absent.clone(),
        absent,
    ];
    assert_eq!(observed, expected);
    assert_eq!(support.carriage(), proved("C05 engine sentinel called"));
}

/// SI1's identity survives loading: each entry keeps the harness version
/// it was measured on, and the host is part of the shape, so a Linux and
/// a macOS measurement of one invocation are two facts and two adapters
/// differing only in version are not the same declaration.
#[test]
fn the_measured_version_and_host_survive_loading() {
    let mut mac = on("claude", "claude");
    mac["host"] = json!("macos");
    mac["version"] = json!("2.1.290");
    let both = facts(json!([
        entry(json!("cold"), "boxed", on("claude", "claude"), axes()),
        entry(json!("cold"), "boxed", mac, axes()),
    ]));
    let isolation = McpIsolation {
        ambient: proved("a"),
        native_write: proved("b"),
        store_read: proved("c"),
        process_read: proved("d"),
    };
    let measurement = |version: &str| McpMeasurement {
        version: version.to_string(),
        isolation: isolation.clone(),
    };
    let cold = shape(McpInvocation::Cold, McpHands::Boxed);
    let macos = McpShape {
        host: McpHost::Macos,
        ..cold.clone()
    };
    assert_eq!(
        support(&claude(both)),
        McpSupport::Declared {
            carriage: proved("C05 engine sentinel called"),
            shapes: BTreeMap::from([
                (cold, measurement("2.1.287")),
                (macos, measurement("2.1.290")),
            ]),
        }
    );
    let mut newer = on("claude", "claude");
    newer["version"] = json!("2.1.290");
    let declared = |on: Value| {
        support(&claude(facts(json!([entry(
            json!("cold"),
            "none",
            on,
            axes()
        )]))))
    };
    assert_ne!(declared(on("claude", "claude")), declared(newer));
}

/// MB2's control: ambient exclusion and read-only write confinement are
/// both measured, yet the read axes keep their own failed and missing
/// results. A read axis left out is a decoding refusal, never inferred.
#[test]
fn write_confinement_and_ambient_exclusion_never_supply_a_secret_read_proof() {
    let axes = [
        measured("C09b ambient excluded"),
        measured("X08 read-only sandbox denies writes outside the workspace"),
        json!({"unsupported": "X08 native shell reads the 0600 store"}),
        json!({"unmeasured": "no process canary"}),
    ];
    let claude_on = || on("claude", "claude");
    let support = support(&claude(facts(json!([entry(
        json!("cold"),
        "none",
        claude_on(),
        axes.clone()
    )]))));
    assert_eq!(
        support.isolation(&shape(McpInvocation::Cold, McpHands::NoHands)),
        McpIsolation {
            ambient: proved("C09b ambient excluded"),
            native_write: proved("X08 read-only sandbox denies writes outside the workspace"),
            store_read: McpAxis::Unsupported {
                reason: "X08 native shell reads the 0600 store".to_string()
            },
            process_read: McpAxis::Unmeasured(McpUnmeasured::Declared(
                "no process canary".to_string()
            )),
        }
    );
    let mut missing = entry(json!("cold"), "none", claude_on(), axes);
    missing.as_object_mut().unwrap().remove("store_read");
    assert_eq!(
        problem(&claude(facts(json!([missing])))),
        McpError::Decode("missing field `store_read`".to_string())
    );
}

/// The legacy forms load and grant nothing — a server map naming a server
/// included — and only the exec harness may declare no MCP surface.
#[test]
fn legacy_maps_grant_nothing_and_only_exec_is_inapplicable() {
    let any = shape(McpInvocation::Cold, McpHands::Boxed);
    let legacy = McpAxis::Unmeasured(McpUnmeasured::Legacy);
    let inapplicable = McpAxis::Inapplicable {
        reason: "no model MCP surface".to_string(),
    };
    let exec = adapter(
        "exec",
        "exec",
        "sh",
        json!({"inapplicable": "no model MCP surface"}),
    );
    let observed = [
        claude(json!({"flag": "--mcp-config", "servers": {"github": "/etc/github.json"}})),
        claude(json!("unsupported")),
        exec,
    ]
    .map(|body| {
        let support = support(&body);
        (support.clone(), support.carriage(), support.isolation(&any))
    });
    let inapplicable_support = McpSupport::Inapplicable {
        reason: "no model MCP surface".to_string(),
    };
    assert_eq!(
        observed,
        [
            (
                McpSupport::Legacy {
                    flag: Some("--mcp-config".to_string())
                },
                legacy.clone(),
                every(&legacy)
            ),
            (
                McpSupport::Legacy { flag: None },
                legacy.clone(),
                every(&legacy)
            ),
            (
                inapplicable_support,
                inapplicable.clone(),
                every(&inapplicable)
            ),
        ]
    );
    let lanetally = adapter(
        "lanetally",
        "lanetally",
        "claude-lanetally",
        json!({"inapplicable": "no model MCP surface"}),
    );
    assert_eq!(
        problem(&lanetally),
        McpError::ModelHarness("lanetally".to_string())
    );
}

/// The legacy form keeps the values it always accepted — any non-empty
/// flag and server path, however long — and refuses in its old words.
#[test]
fn legacy_values_keep_their_old_bounds_and_refusals() {
    let flag = format!("--{}", "m".repeat(450));
    let path = format!("/{}", "p".repeat(1_000));
    assert_eq!(
        support(&claude(json!({"flag": flag, "servers": {"github": path}}))),
        McpSupport::Legacy { flag: Some(flag) }
    );
    let observed = [
        json!({"flag": "", "servers": {}}),
        json!({"servers": {}}),
        json!({"flag": "--mcp-config"}),
        json!({"flag": "--mcp-config", "servers": ["github"]}),
        json!({"flag": "--mcp-config", "servers": {"github": ""}}),
        json!({"flag": "--mcp-config", "servers": {"github": 1}}),
    ]
    .map(|mcp| problem(&claude(mcp)));
    assert_eq!(
        observed,
        [
            McpError::LegacyFlag,
            McpError::LegacyFlag,
            McpError::LegacyServers,
            McpError::LegacyServers,
            McpError::LegacyServerPath("github".to_string()),
            McpError::LegacyServerPath("github".to_string()),
        ]
    );
}

/// A harness, wrapper, binary, hands mode or resume shape the adapter does
/// not itself declare cannot be supplied by a measurement: each refuses by
/// variant. The same binary under another driver is another harness.
#[test]
fn a_wrapper_hands_or_resume_shape_cannot_borrow_a_measurement() {
    let cold = |hands: &str, on: Value| entry(json!("cold"), hands, on, axes());
    let claude_on = || on("claude", "claude");
    let resume = |hands: &str| entry(json!({"resume": "work-site"}), hands, claude_on(), axes());
    let no_hands = |mcp: Value| {
        let mut body = claude(mcp);
        body.as_object_mut().unwrap().remove("hands");
        body
    };
    let mut boxed_resume = claude(facts(json!([resume("boxed")])));
    boxed_resume["hands"]["harness"] = json!({"work": ["--permission-mode", "acceptEdits"]});
    boxed_resume["resume"] = json!({"work-site": {"status": "unmeasured",
        "identity": {"unknown": "not remeasured"}, "classes": ["work"],
        "boundaries": ["harness"], "hands": "harness", "reason": "not remeasured"}});
    let lanetally = |binary: &str, on: Value| {
        adapter(
            "lanetally",
            "lanetally",
            binary,
            facts(json!([cold("boxed", on)])),
        )
    };
    let observed = [
        problem(&lanetally("claude", claude_on())),
        problem(&lanetally("claude-lanetally", on("lanetally", "claude"))),
        problem(&no_hands(facts(json!([cold("boxed", claude_on())])))),
        problem(&claude(facts(json!([cold("harness", claude_on())])))),
        problem(&claude(facts(json!([resume("boxed")])))),
        problem(&boxed_resume),
    ];
    let boxed = shape(McpInvocation::Cold, McpHands::Boxed);
    let work_site = shape(
        McpInvocation::Resume("work-site".to_string()),
        McpHands::Boxed,
    );
    assert_eq!(
        observed,
        [
            McpError::BorrowedHarness {
                shape: boxed.clone(),
                measured: "claude".to_string(),
                harness: "lanetally".to_string(),
            },
            McpError::BorrowedBinary {
                shape: boxed.clone(),
                measured: "claude".to_string(),
                binary: "claude-lanetally".to_string(),
            },
            McpError::UndeclaredHands(boxed),
            McpError::UndeclaredHands(shape(McpInvocation::Cold, McpHands::Harness)),
            McpError::UndeclaredResume(work_site.clone()),
            McpError::ResumeHands {
                shape: work_site,
                declared: "harness".to_string(),
            },
        ]
    );
}

/// Declared harness hands and a resume shape declared under them are the
/// adapter's own, and their entries load under their exact keys.
#[test]
fn declared_harness_hands_and_resume_shapes_are_admitted() {
    let claude_on = || on("claude", "claude");
    let mut body = claude(facts(json!([
        entry(
            json!({"resume": "work-site"}),
            "harness",
            claude_on(),
            axes()
        ),
        entry(json!("replacement"), "boxed", claude_on(), axes()),
    ])));
    body["hands"]["harness"] = json!({"work": ["--permission-mode", "acceptEdits"]});
    body["resume"] = json!({"work-site": {"status": "unmeasured",
        "identity": {"unknown": "not remeasured"}, "classes": ["work"],
        "boundaries": ["harness"], "hands": "harness", "reason": "not remeasured"}});
    let McpSupport::Declared { shapes, .. } = support(&body) else {
        panic!("typed facts decode as declared");
    };
    assert_eq!(
        shapes.keys().cloned().collect::<Vec<_>>(),
        vec![
            shape(McpInvocation::Replacement, McpHands::Boxed),
            shape(
                McpInvocation::Resume("work-site".to_string()),
                McpHands::Harness
            ),
        ]
    );
}

/// Closed decoding: every malformed declaration is refused with its exact
/// variant and never read as unmeasured.
#[test]
fn closed_decoding_refuses_each_malformed_fact_by_variant() {
    let cold = || entry(json!("cold"), "boxed", on("claude", "claude"), axes());
    let mut stray = cold();
    stray["invented"] = json!(1);
    let mut proved_word = cold();
    proved_word["ambient"] = json!({"proved": "x"});
    let mut empty = cold();
    empty["ambient"] = measured("");
    let mut long = cold();
    long["ambient"] = measured(&"x".repeat(401));
    let mut two_lines = cold();
    two_lines["ambient"] = measured("a\nb");
    let mut windows = cold();
    windows["measured_on"]["host"] = json!("windows");
    let bounded = "a measured fact must be a bounded non-empty line of at most 400 characters";
    let rows = [
        (
            json!({"invented": 1}),
            McpError::UnknownKey("invented".to_string()),
        ),
        (
            json!({"flag": "--mcp-config", "servers": {}, "carriage": measured("x")}),
            McpError::MixedForms,
        ),
        (
            json!({"carriage": measured("x"), "shapes": [], "inapplicable": "x"}),
            McpError::MixedForms,
        ),
        (
            json!({"flag": "--mcp-config", "servers": {"GitHub": "/etc/g.json"}}),
            McpError::LegacyServer("GitHub".to_string()),
        ),
        (
            facts(json!([stray])),
            McpError::Decode(
                "unknown field `invented`, expected one of `invocation`, `hands`, \
                 `measured_on`, `ambient`, `native_write`, `store_read`, `process_read`"
                    .to_string(),
            ),
        ),
        (
            facts(json!([proved_word])),
            McpError::Decode(
                "unknown variant `proved`, expected one of `measured`, `unsupported`, \
                 `unmeasured`"
                    .to_string(),
            ),
        ),
        (facts(json!([empty])), McpError::Decode(bounded.to_string())),
        (facts(json!([long])), McpError::Decode(bounded.to_string())),
        (
            facts(json!([two_lines])),
            McpError::Decode(bounded.to_string()),
        ),
        (
            facts(json!([windows])),
            McpError::Decode("unknown variant `windows`, expected `linux` or `macos`".to_string()),
        ),
        (
            facts(json!([cold(), cold()])),
            McpError::DuplicateShape(shape(McpInvocation::Cold, McpHands::Boxed)),
        ),
    ];
    let (observed, expected): (Vec<_>, Vec<_>) = rows
        .into_iter()
        .map(|(mcp, expected)| (problem(&claude(mcp)), expected))
        .unzip();
    assert_eq!(observed, expected);
}

/// The module's operator text, pinned once: the refusal names the adapter
/// file and `'mcp'`, then the problem with its shape. The legacy rows read
/// as the loader always wrote them.
#[test]
fn each_refusal_reads_as_the_operator_sees_it() {
    let resume = McpShape {
        invocation: McpInvocation::Resume("work-site".to_string()),
        hands: McpHands::NoHands,
        host: McpHost::Macos,
    };
    let cold = shape(McpInvocation::Cold, McpHands::Boxed);
    let refusal = |problem| {
        McpRefusal {
            what: "adapter 'claude'".to_string(),
            problem,
        }
        .to_string()
    };
    let observed = [
        refusal(McpError::UnknownKey("x".to_string())),
        refusal(McpError::MixedForms),
        refusal(McpError::Decode("missing field `carriage`".to_string())),
        refusal(McpError::LegacyFlag),
        refusal(McpError::LegacyServers),
        refusal(McpError::LegacyServer("X".to_string())),
        refusal(McpError::LegacyServerPath("github".to_string())),
        refusal(McpError::ModelHarness("dsh".to_string())),
        refusal(McpError::DuplicateShape(shape(
            McpInvocation::Replacement,
            McpHands::Harness,
        ))),
        refusal(McpError::BorrowedHarness {
            shape: cold.clone(),
            measured: "claude".to_string(),
            harness: "lanetally".to_string(),
        }),
        refusal(McpError::BorrowedBinary {
            shape: cold,
            measured: "claude".to_string(),
            binary: "claude-lanetally".to_string(),
        }),
        refusal(McpError::UndeclaredHands(resume.clone())),
        refusal(McpError::UndeclaredResume(resume.clone())),
        refusal(McpError::ResumeHands {
            shape: resume,
            declared: "harness".to_string(),
        }),
    ];
    let expected = [
        "adapter 'claude' 'mcp' has unknown key 'x'; known keys: flag, servers, carriage, \
         shapes, inapplicable",
        "adapter 'claude' 'mcp' mixes forms; write the legacy 'flag' and 'servers', the typed \
         'carriage' and 'shapes', or 'inapplicable' alone",
        "adapter 'claude' 'mcp' does not decode: missing field `carriage`",
        "adapter 'claude' 'mcp' needs a non-empty string 'flag'",
        "adapter 'claude' 'mcp' needs 'servers' as an object of strings",
        "adapter 'claude' 'mcp' 'servers' names 'X', which does not match ^[a-z][a-z0-9-]*$",
        "adapter 'claude' 'mcp' 'servers.github' must be a non-empty string",
        "adapter 'claude' 'mcp' is 'inapplicable', but harness 'dsh' serves a model; only the \
         exec harness has no model MCP surface",
        "adapter 'claude' 'mcp' declares shape 'replacement' with hands 'harness' on linux twice",
        "adapter 'claude' 'mcp' shape 'cold' with hands 'boxed' on linux was measured under \
         harness 'claude', not this adapter's driver harness 'lanetally'; another harness's \
         or wrapper's evidence qualifies nothing here",
        "adapter 'claude' 'mcp' shape 'cold' with hands 'boxed' on linux was measured on \
         'claude', not on this adapter's binary 'claude-lanetally'; another harness's or \
         wrapper's evidence qualifies nothing here",
        "adapter 'claude' 'mcp' shape 'resume work-site' with hands 'none' on macos names \
         hands this adapter does not declare; a measurement supplies no hands",
        "adapter 'claude' 'mcp' shape 'resume work-site' with hands 'none' on macos names a \
         resume shape this adapter's 'resume' does not declare",
        "adapter 'claude' 'mcp' shape 'resume work-site' with hands 'none' on macos names a \
         resume shape this adapter's 'resume' declares under hands 'harness'; a measurement \
         under other hands qualifies nothing",
    ];
    assert_eq!(observed, expected);
}
