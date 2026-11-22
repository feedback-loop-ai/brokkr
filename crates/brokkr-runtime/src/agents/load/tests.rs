//! The order the loaders refuse in, step against step (#288 slice L).
//! Each row writes one definition that fails two adjacent steps of
//! `parse_adapter`, `resume_assessment` or `parse_agent`, and the earlier
//! step's refusal is the one reported. A row's text names the file's own
//! description as `{what}`. Inside each step the reads are bound the same
//! way (#288 slice N): a row fails two adjacent reads of one step, and
//! the earlier read's refusal is the one reported.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::{Adapters, Library, LibraryError};
use crate::agents::RootShapeUnmet;

/// Each row's patch written over `loadable` as `file`, alone in a fresh
/// canonical directory beside a charter that declares nothing, and
/// loaded: the refusals observed, beside each row's text with `{what}`
/// read as `described` makes it of the file's path.
fn refusals<T: std::fmt::Debug>(
    rows: &[(Value, &str)],
    (file, loadable): (&str, &Value),
    load: fn(&Path) -> Result<T, LibraryError>,
    described: fn(&Path) -> String,
) -> (Vec<String>, Vec<String>) {
    rows.iter()
        .map(|(patch, expected)| {
            let dir = tempfile::tempdir().unwrap();
            let root: PathBuf = dir.path().canonicalize().unwrap();
            let mut body = loadable.clone();
            for (key, value) in patch.as_object().unwrap() {
                body[key] = value.clone();
            }
            std::fs::create_dir(root.join("charters")).unwrap();
            std::fs::write(root.join("charters/plain.md"), "# office\n").unwrap();
            std::fs::write(root.join(file), body.to_string()).unwrap();
            let refused = load(&root).unwrap_err().to_string();
            (
                refused,
                expected.replace("{what}", &described(&root.join(file))),
            )
        })
        .unzip()
}

/// Each row's refusal from `Adapters::load`, over an adapter that loads.
/// The smallest claude adapter that loads.
fn loadable_adapter() -> Value {
    json!({
        "effort_flag": "--effort", "efforts": ["high"], "model_flag": "--model",
        "models": {"opus": "claude-opus-5"}, "mcp": "unsupported", "provider": "claude",
        "tool_permissions": "unsupported", "binary": "claude",
        "driver": ["{brokkr}", "driver", "claude", "--"],
    })
}

fn adapter_refusals(rows: &[(Value, &str)]) -> (Vec<String>, Vec<String>) {
    let loadable = loadable_adapter();
    refusals(rows, ("claude.json", &loadable), Adapters::load, |path| {
        format!("adapter 'claude' ({})", path.display())
    })
}

/// A typed MCP shape measured on this adapter's own harness and binary,
/// which admission refuses when the adapter declares no boxed hands.
fn boxed_mcp() -> Value {
    let measured = json!({"measured": "m"});
    json!({"carriage": measured, "shapes": [{
        "invocation": "cold", "hands": "boxed", "ambient": measured,
        "native_write": measured, "store_read": measured, "process_read": measured,
        "measured_on": {"harness": "claude", "binary": "claude", "version": "1.0.0",
                        "host": "linux"},
    }]})
}

#[test]
fn an_adapter_failing_two_adjacent_steps_reports_the_earlier() {
    let rows = [
        // pinning, then hands; and inside pinning, efforts, then effortless routes.
        (
            json!({"efforts": ["Xhigh"], "hands": "maybe"}),
            "{what} 'efforts' names 'Xhigh', which does not match ^[a-z][a-z0-9-]*$",
        ),
        (
            json!({"efforts": ["Xhigh"], "effortless_routes": []}),
            "{what} 'efforts' names 'Xhigh', which does not match ^[a-z][a-z0-9-]*$",
        ),
        // hands, then resume.
        (
            json!({"hands": "maybe", "resume": true}),
            "{what} 'hands' is the string 'maybe'; the only legal string here is \
             \"unsupported\" — a provider's missing capability is declared, never inferred",
        ),
        // resume, then credentials.
        (
            json!({"resume": true, "credentials": []}),
            "{what} 'resume' must be an object of named execution shapes; a bare value \
             cannot say WHICH shape was measured, and a shape nobody named enables nothing",
        ),
        // credentials, then MCP admission.
        (
            json!({"credentials": [], "mcp": boxed_mcp()}),
            "{what} 'credentials' must be an object of route name → environment variable name",
        ),
        // MCP admission, then the fields settled after it.
        (
            json!({"mcp": boxed_mcp(), "trust_tier": "trusetd"}),
            "{what} 'mcp' shape 'cold' with hands 'boxed' on linux names hands this adapter \
             does not declare; a measurement supplies no hands",
        ),
    ];
    let (observed, expected) = adapter_refusals(&rows);
    assert_eq!(observed, expected);
}

#[test]
fn an_adapter_step_failing_two_adjacent_reads_reports_the_earlier() {
    let rows = [
        // invocation: the native inventory, then the provider.
        (
            json!({"native_capabilities": 7, "provider": "codex"}),
            "{what} 'native_capabilities' at '': it does not satisfy '/type'",
        ),
        // the native inventory, then the provider's own string read.
        (
            json!({"native_capabilities": 7, "provider": 7}),
            "{what} 'native_capabilities' at '': it does not satisfy '/type'",
        ),
        // the provider's file name, then the binary.
        (
            json!({"provider": "codex", "binary": 7}),
            "{what} declares provider 'codex' but the file is named 'claude.json'; the file \
             name is the provider name",
        ),
        // the binary, then the driver.
        (
            json!({"binary": 7, "driver": 7}),
            "{what} needs a non-empty string 'binary'",
        ),
        // served: the models, then the judges.
        (
            json!({"models": 7, "judges": 7}),
            "{what} needs 'models' as an object of strings",
        ),
        // the judges' names, then whether each is served.
        (
            json!({"judges": ["Opus"]}),
            "{what} 'judges' names 'Opus', which does not match ^[a-z][a-z0-9-]*$",
        ),
        // tool permissions: the gap's keys, then its reason.
        (
            json!({"tool_permissions": {"unsupported": 7, "unmeasured": 1}}),
            "{what} 'tool_permissions' has unknown key 'unmeasured'; known keys: unsupported",
        ),
        // the mapping's keys, then its flag.
        (
            json!({"tool_permissions": {"flag": 7, "x": 1}}),
            "{what} 'tool_permissions' has unknown key 'x'; known keys: flag, separator, names",
        ),
        // the flag, then the separator.
        (
            json!({"tool_permissions": {"flag": 7, "separator": 7}}),
            "{what} 'tool_permissions' needs a non-empty string 'flag'",
        ),
        // the separator, then the names.
        (
            json!({"tool_permissions": {"flag": "--allow", "separator": 7, "names": 7}}),
            "{what} 'tool_permissions' needs a non-empty string 'separator'",
        ),
        // pinning: the hint, then the model flag.
        (
            json!({"hint": 7, "model_flag": 7}),
            "{what} needs a non-empty string 'hint'",
        ),
        // the model flag, then the efforts.
        (
            json!({"model_flag": 7, "efforts": 7}),
            "{what} needs 'model_flag': the flag this provider takes, or the explicit string \
             \"unsupported\"",
        ),
        // an effortless route's name, then its measurement.
        (
            json!({"effortless_routes": {"us/east": ""}}),
            "{what} 'effortless_routes' names 'us/east', which does not match \
             ^[A-Za-z0-9._:-]+$",
        ),
        // hands: a notice beside the gap, then the gap's keys.
        (
            json!({"hands": {"unsupported": "measured", "notice": 7}}),
            "{what} 'hands.notice' needs a supported, non-empty 'hands.workspace' fragment; a \
             discovery notice names the workspace tool that fragment serves (decision 0069)",
        ),
        // the gap's keys, then its reason.
        (
            json!({"hands": {"unsupported": 7, "unmeasured": 1}}),
            "{what} 'hands' has unknown key 'unmeasured'; known keys: unsupported",
        ),
        // the fragment's keys, then the workspace.
        (
            json!({"hands": {"workspace": 7, "x": 1}}),
            "{what} 'hands' has unknown key 'x'; known keys: workspace, harness, notice",
        ),
        // the notice, then the harness.
        (
            json!({"hands": {"workspace": [], "notice": 7, "harness": 7}}),
            "{what} 'hands.notice' needs a supported, non-empty 'hands.workspace' fragment; a \
             discovery notice names the workspace tool that fragment serves (decision 0069)",
        ),
        // settled: the trust tier, then the egress class.
        (
            json!({"trust_tier": "trusetd", "egress": "lokal"}),
            "{what} 'trust_tier' is \"trusetd\"; the vocabulary is closed — \"trusted\" or \
             \"untrusted\" — and an absent tier is untrusted",
        ),
        // the egress class, then the routes.
        (
            json!({"egress": "lokal", "routes": []}),
            "{what} 'egress' is \"lokal\"; the egress vocabulary is closed — \"local\", \
             \"contracted\" or \"uncontracted\" — and an absent class is uncontracted (decision 0036 ruling 1)",
        ),
        // the routes, then the harness's authentication.
        (
            json!({"routes": [], "harness_auth": 7}),
            "{what} 'routes' must be an object of route name → egress class",
        ),
        // the harness's authentication, then the effort flag.
        (
            json!({"harness_auth": 7, "effort_flag": 7}),
            "{what} 'harness_auth' must be an object of exactly 'names' and a non-empty \
             'measured' (it is not an object)",
        ),
    ];
    let (observed, expected) = adapter_refusals(&rows);
    assert_eq!(observed, expected);
}

/// The strict parse of the bytes read, which a definition alone cannot
/// fail beside a field, is refused ahead of the native inventory; and the
/// digest, of those same bytes, is no read of its own, so an absent file
/// leaves the effort flag's refusal standing (the controller's rulings R9
/// and R37). Each step is called with a map and bytes that disagree.
#[test]
fn an_adapter_step_failing_a_field_and_its_file_reports_the_earlier() {
    let dir = tempfile::tempdir().unwrap();
    let twice = br#"{"native_capabilities": 7, "native_capabilities": 7}"#;
    let (_, problem) = super::json_of(twice, &dir.path().join("claude.json")).unwrap();
    let native = json!({"native_capabilities": 7});
    let invocation =
        super::adapter::invocation("claude", native.as_object().unwrap(), problem, "w");
    let effort = json!({"effort_flag": 7});
    let absent = dir.path().join("absent.json");
    let settled = super::adapter::settled(
        effort.as_object().unwrap(),
        &BTreeMap::new(),
        (&absent, b""),
        "w",
        &[],
    );
    assert_eq!(
        [
            invocation.err().map(|e| e.to_string()),
            settled.err().map(|e| e.to_string())
        ],
        [
            Some("w: key 'native_capabilities' is written twice at line 1 column 52".to_string()),
            Some(
                "w needs 'effort_flag': the flag this provider takes, or the explicit string \
                 \"unsupported\""
                    .to_string()
            ),
        ]
    );
}

/// Decision 0075 ruling 2: `record_kinds` is an object of a `transcript`
/// kind and an optional `root_session` kind, each one seat-record v8 kind
/// and never `none`; anything else refuses by the rule it broke.
#[test]
fn a_record_kinds_declaration_refuses_every_other_shape() {
    let kind_rule = "must be one kind of at most 40 bytes matching \
                     ^[a-z][a-z0-9]*(-[a-z0-9]+)*$, and not 'none'";
    let rows = [
        (
            json!({"record_kinds": "trellis-trail"}),
            "{what} 'record_kinds' must be a JSON object".to_string(),
        ),
        (
            json!({"record_kinds": {"transcript": "trellis-trail", "home": "/h"}}),
            "{what} 'record_kinds' has unknown key 'home'; known keys: transcript, root_session, \
             resume_shape"
                .to_string(),
        ),
        (
            json!({"record_kinds": {"root_session": "trellis-thread"}}),
            "{what} 'record_kinds' needs a non-empty string 'transcript'".to_string(),
        ),
        (
            json!({"record_kinds": {"transcript": "Trellis"}}),
            format!("{{what}} 'record_kinds' 'transcript' {kind_rule}"),
        ),
        (
            json!({"record_kinds": {"transcript": "none"}}),
            format!("{{what}} 'record_kinds' 'transcript' {kind_rule}"),
        ),
        (
            json!({"record_kinds": {"transcript": "trellis-trail", "root_session": "none"}}),
            format!("{{what}} 'record_kinds' 'root_session' {kind_rule}"),
        ),
    ];
    let rows: Vec<(Value, &str)> = (rows.iter())
        .map(|(patch, text)| (patch.clone(), text.as_str()))
        .collect();
    let (observed, expected) = adapter_refusals(&rows);
    assert_eq!(observed, expected);
}

/// A root session kind with no `resume` member at all is refused by its
/// own variant, naming the adapter: a rejoin is declared, never defaulted
/// (decision 0075 ruling 1; the controller's ruling R6). Its text is
/// pinned once, here.
#[test]
fn a_root_session_kind_without_a_resume_is_refused_by_its_own_variant() {
    let dir = tempfile::tempdir().unwrap();
    let mut adapter = loadable_adapter();
    adapter["record_kinds"] =
        json!({"transcript": "trellis-trail", "root_session": "trellis-thread"});
    let path = dir.path().join("claude.json");
    std::fs::write(&path, adapter.to_string()).unwrap();
    let refusal = Adapters::load(dir.path()).unwrap_err();
    let named = format!("adapter 'claude' ({})", path.display());
    assert!(
        matches!(&refusal, LibraryError::RootWithoutResume { adapter } if *adapter == named),
        "{refusal:?}"
    );
    assert_eq!(
        refusal.to_string(),
        format!(
            "{named} 'record_kinds' declares a 'root_session' kind and the adapter declares no \
             'resume'; a rejoin is declared, never defaulted (decision 0075 ruling 1)"
        )
    );
}

/// A root session kind names the one shape of the adapter's `resume` its
/// seats rejoin by (the controller's ruling R14): a `root_session` without
/// a `resume_shape`, a `resume_shape` without a `root_session`, and a
/// `resume_shape` naming no shape the adapter declares are each refused
/// by their own cause, naming the adapter; a named declared shape loads.
#[test]
fn a_root_session_kind_names_the_one_declared_shape_it_rejoins_by() {
    let shape = json!({"status": "unsupported", "identity": {"unknown": "fixture"},
        "classes": ["work"], "boundaries": ["not applicable"], "hands": "none",
        "reason": "fixture: cannot rejoin"});
    let loaded = |kinds: Value| {
        let dir = tempfile::tempdir().unwrap();
        let mut adapter = loadable_adapter();
        adapter["resume"] = json!({"headless": shape});
        adapter["record_kinds"] = kinds;
        let path = dir.path().join("claude.json");
        std::fs::write(&path, adapter.to_string()).unwrap();
        let named = format!("adapter 'claude' ({})", path.display());
        (Adapters::load(dir.path()), named)
    };
    let (trail, thread) = ("trellis-trail", "trellis-thread");
    for (kinds, unmet, text) in [
        (
            json!({"transcript": trail, "root_session": thread}),
            RootShapeUnmet::Unnamed,
            "declares a 'root_session' kind and no 'resume_shape' it rejoins by",
        ),
        (
            json!({"transcript": trail, "resume_shape": "headless"}),
            RootShapeUnmet::Rootless,
            "names a 'resume_shape' and declares no 'root_session' kind to rejoin",
        ),
        (
            json!({"transcript": trail, "root_session": thread, "resume_shape": "interactive"}),
            RootShapeUnmet::Undeclared("interactive".into()),
            "names 'resume_shape' 'interactive', which its 'resume' does not declare",
        ),
    ] {
        let (refusal, named) = loaded(kinds);
        let refusal = refusal.unwrap_err();
        assert!(
            matches!(&refusal, LibraryError::RootShape { adapter, unmet: found }
                if *adapter == named && *found == unmet),
            "{refusal:?}"
        );
        let text = format!("{named} 'record_kinds' {text} (decision 0075 ruling 1)");
        assert_eq!(refusal.to_string(), text);
    }
    let kinds = json!({"transcript": trail, "root_session": thread, "resume_shape": "headless"});
    let (adapters, _) = loaded(kinds);
    let adapters = adapters.unwrap();
    let declared = adapters.adapter("claude").unwrap().record_kinds.clone();
    let root = crate::agents::kinds::RootKind {
        kind: thread.into(),
        resume_shape: "headless".into(),
    };
    assert_eq!(declared.unwrap().root_session, Some(root));
}

/// One read (the controller's ruling R9 of 2026-11-21): an adapter is
/// what the bytes it was read from declare, strictly parsed, and its
/// digest, the witness its site pins, is of those bytes, whatever the
/// file at its path holds by then — here a key written twice and another
/// transcript kind.
#[test]
fn an_adapter_declares_and_digests_the_one_read_of_its_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("claude.json");
    let mut read = loadable_adapter();
    read["record_kinds"] = json!({"transcript": "trellis-trail"});
    let mut later = loadable_adapter();
    later["record_kinds"] = json!({"transcript": "other-trail"});
    let later = format!("{{\"provider\": \"claude\", {}", &later.to_string()[1..]);
    std::fs::write(&path, later).unwrap();
    let bytes = read.to_string().into_bytes();
    let what = super::models::described("claude", &path);
    let (adapter, _) = super::adapter_from("claude", &path, &what, &bytes).unwrap();
    assert_eq!(adapter.digest, brokkr_core::canonical::sha256_bytes(&bytes));
    let declared = crate::agents::kinds::RecordKinds {
        transcript: "trellis-trail".to_string(),
        root_session: None,
    };
    assert_eq!(adapter.record_kinds, Some(declared));
}

#[test]
fn a_resume_shape_failing_two_adjacent_steps_reports_the_earlier() {
    let shape = |patch: Value| {
        let mut shape = json!({"status": "unmeasured", "identity": {"unknown": "u"},
                               "classes": [], "boundaries": [], "hands": "boxed"});
        for (key, value) in patch.as_object().unwrap() {
            shape[key] = value.clone();
        }
        json!({"resume": {"cold": shape}})
    };
    let rows = [
        // the shape's name, then the shape.
        (
            json!({"resume": {"Cold": 7}}),
            "{what} 'resume' names shape 'Cold', which does not match ^[a-z][a-z0-9-]*$",
        ),
        // the shape's keys, then its status.
        (
            json!({"resume": {"cold": {"invented": 1}}}),
            "{what} 'resume' 'cold' has unknown key 'invented'; known keys: status, identity, \
             classes, boundaries, hands, evidence, limitations, reason",
        ),
        // the status, then the identity.
        (
            json!({"resume": {"cold": {"status": "maybe"}}}),
            "{what} 'resume' 'cold' needs 'status': one of unmeasured, unsupported, supported",
        ),
        // the evidence, then the limitations.
        (
            shape(json!({"evidence": 7, "limitations": 7})),
            "{what} 'resume' 'cold' 'evidence' must be a JSON object",
        ),
        // the limitations, then the reason.
        (
            shape(json!({"limitations": [""], "reason": ""})),
            "{what} 'resume' 'cold' 'limitations' must be a bounded non-empty line of at most \
             400 characters",
        ),
        // the reason, then the two enablement rules.
        (
            shape(json!({"status": "supported", "reason": ""})),
            "{what} 'resume' 'cold' needs a non-empty string 'reason'",
        ),
        // the identity, then the classes.
        (
            shape(json!({"identity": 7, "classes": 7})),
            "{what} 'resume' 'cold' 'identity' is required: either the measured form \
             {\"version\": ..., \"applies_to\": ...} or the explicit \
             {\"unknown\": \"<measured reason>\"}",
        ),
        // the classes' names, then the boundaries.
        (
            shape(json!({"classes": ["Fresh"], "boundaries": 7})),
            "{what} 'resume' 'cold' 'classes' names 'Fresh', which does not match \
             ^[a-z][a-z0-9-]*$",
        ),
        // the boundaries, then the hands.
        (
            shape(json!({"boundaries": 7, "hands": 7})),
            "{what} 'resume' 'cold' needs 'boundaries' as an array of strings",
        ),
        // the hands, then the evidence.
        (
            shape(json!({"hands": 7, "evidence": 7})),
            "{what} 'resume' 'cold' needs a non-empty string 'hands'",
        ),
        // enablement: the measured identity, then the four evidence references.
        (
            shape(json!({"status": "supported"})),
            "{what} 'resume' 'cold' is 'supported' with an unknown identity; only a MEASURED \
             identity can support enablement, because the per-invocation version check needs \
             a pinned version to compare the observed one against",
        ),
    ];
    let (observed, expected) = adapter_refusals(&rows);
    assert_eq!(observed, expected);
}

#[test]
fn an_agent_failing_two_adjacent_steps_reports_the_earlier() {
    let loadable = json!({"models": ["opus"], "charter": "charters/plain.md", "description": "d"});
    let rows = [
        // capabilities, then the description and charter.
        (
            json!({"capabilities": ["web-search"], "description": ""}),
            "agent 'tester' 'capabilities' must be an object from capability name to \
             \"requires\" or \"wants\"; a request names no dialect, tool or grant, because \
             only realms.json grants (decision 0065 ruling 3)",
        ),
        // the model chain, then the tools.
        (
            json!({"models": [], "tools": []}),
            "{what} 'models' is empty; an agent names at least one model",
        ),
        // the tools, then the hands.
        (
            json!({"tools": [], "hands": 7}),
            "{what} 'tools' must be a JSON object",
        ),
        // the hands, then the limits.
        (
            json!({"hands": 7, "limits": []}),
            "{what} 'hands': hands must be \"workspace\" or an object, got 7",
        ),
        // the limits, then the inputs.
        (
            json!({"limits": [], "inputs": 7}),
            "{what} 'limits' must be a JSON object",
        ),
        // the inputs, then the charter's check of what the office asks for.
        (
            json!({"inputs": 7, "capabilities": {"web-search": "requires"}}),
            "{what} needs 'inputs' as an array of strings",
        ),
        // the office: the description, then the charter.
        (
            json!({"description": "", "charter": 7}),
            "{what} needs a non-empty string 'description'",
        ),
        // the chain: the models' names, then the efforts.
        (
            json!({"models": ["Opus"], "efforts": 7}),
            "{what} 'models' names 'Opus', which does not match ^[a-z][a-z0-9-]*$",
        ),
    ];
    let (observed, expected) = refusals(&rows, ("tester.json", &loadable), Library::load, |path| {
        format!("agent 'tester' ({})", path.display())
    });
    assert_eq!(observed, expected);
}
