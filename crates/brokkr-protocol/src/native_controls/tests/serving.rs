//! The sealed serving inputs written as closed JSON and read back (rebuild
//! unit 14a2), with the MCP isolation intent sealed among them (decision
//! 0065 slice two, U1g2). Moved from `native_controls/tests.rs`.

use super::*;

/// Sealed serving inputs carrying every member: a permission flag, each
/// fragment with its tokens unexpanded, an empty-string argument and a
/// repeat kept as written, a typed hands declaration with a bind, and an
/// intent other than the default in every member.
fn full_serving() -> SealedServing {
    SealedServing {
        isolation: SealedIsolation {
            servers: SealedServers::Hands,
            cold: SealedAssessment::Measured,
            replacement: SealedAssessment::Unsupported("project MCP cannot be excluded".into()),
            resume: BTreeMap::from([
                ("boxed-workspace".into(), SealedAssessment::Unmeasured),
                ("work-site".into(), SealedAssessment::Measured),
            ]),
            fence: SealedFence::Lifted,
        },
        dialect: SealedDialect {
            permissions: Some(ListFlag {
                flag: "--allowedTools".into(),
                separator: ",".into(),
            }),
            sandbox: argv(&["--sandbox", "read-only", "-o", "{result_path}"]),
            hands: argv(&[
                "--mcp-config",
                "{hands_mcp_json}",
                "",
                "--strict-mcp-config",
            ]),
            boundary: argv(&[
                "--sandbox",
                "workspace-write",
                "--sandbox",
                "workspace-write",
                "-c",
                "mcp_servers.brokkr.command=\"{brokkr}\"",
            ]),
            stands: Some(SealedBoundary::Harness),
        },
        pins: argv(&["--model", "claude-opus-5", "--effort", "high"]),
        spec: Some(
            crate::hands::HandsSpec::parse(&json!({"kind": "workspace", "network": true,
                "binds": [{"path": "~/.cargo", "mode": "overlay", "mask": ["credentials.toml"]}]}))
            .unwrap(),
        ),
    }
}

/// Rebuild unit 14a2: the typed serving inputs sealed beside a launch
/// record are written as closed JSON and read back byte for byte — every
/// member, both optionals in each state, and nothing defaulted, sorted,
/// deduplicated or expanded.
#[test]
fn sealed_serving_inputs_round_trip_byte_exactly() {
    let full = full_serving();
    let written = full.value();
    assert_eq!(
        written,
        json!({
            "dialect": {
                "permissions": {"kind": "flag", "flag": "--allowedTools", "separator": ","},
                "sandbox": ["--sandbox", "read-only", "-o", "{result_path}"],
                "hands": ["--mcp-config", "{hands_mcp_json}", "", "--strict-mcp-config"],
                "boundary": ["--sandbox", "workspace-write", "--sandbox", "workspace-write",
                             "-c", "mcp_servers.brokkr.command=\"{brokkr}\""],
                "stands": {"kind": "harness"},
            },
            "pins": ["--model", "claude-opus-5", "--effort", "high"],
            "spec": {"kind": "typed", "declaration": {"kind": "workspace", "network": true,
                "binds": [{"path": "~/.cargo", "mode": "overlay", "mask": ["credentials.toml"]}]}},
            "isolation": {"servers": "hands", "cold": "measured",
                "replacement": {"unsupported": "project MCP cannot be excluded"},
                "resume": {"boxed-workspace": "unmeasured", "work-site": "measured"},
                "fence": "lifted"},
        })
    );
    let empty = SealedServing::default();
    assert_eq!(
        empty.value(),
        json!({
            "dialect": {
                "permissions": {"kind": "none"},
                "sandbox": [], "hands": [], "boundary": [], "stands": {"kind": "none"},
            },
            "pins": [],
            "spec": {"kind": "none"},
            "isolation": {"servers": "empty", "cold": "unmeasured", "replacement": "unmeasured",
                "resume": {}, "fence": "standing"},
        })
    );
    // Rebuild unit 14a4c: each boundary is sealed under its own word.
    for (word, stands) in [
        ("namespace", SealedBoundary::Namespace),
        ("seatbelt", SealedBoundary::Seatbelt),
        ("container", SealedBoundary::Container),
        ("harness", SealedBoundary::Harness),
        ("open", SealedBoundary::Open),
    ] {
        let sealed = SealedServing {
            dialect: SealedDialect {
                stands: Some(stands),
                ..SealedDialect::default()
            },
            ..SealedServing::default()
        };
        let written = sealed.value();
        assert_eq!(
            written["dialect"]["stands"],
            json!({"kind": word}),
            "{word}"
        );
        assert_eq!(SealedServing::decode(Some(&written)), Ok(sealed), "{word}");
    }
    for sealed in [full, empty] {
        let bytes = serde_json::to_vec(&sealed.value()).unwrap();
        let read = SealedServing::decode(Some(&serde_json::from_slice(&bytes).unwrap()));
        assert_eq!(read.as_ref(), Ok(&sealed));
        assert_eq!(serde_json::to_vec(&read.unwrap().value()).unwrap(), bytes);
    }
}

/// Rebuild unit 14a2: sealed serving inputs that are missing, null,
/// mistyped, unknown-tagged, carry an unknown member or a hands
/// declaration in any but its canonical form each refuse at decode with
/// the complete bounded cause — a fixed path and numeric positions, never
/// the supplied value — and none becomes an empty or default input.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn sealed_serving_inputs_refuse_each_tampered_member_with_its_full_cause() {
    let sentinel = format!("SENTINEL\n{}ü", "x".repeat(600));
    let valid = full_serving().value();
    let cause = |path: &str, problem: &str| {
        format!(
            "refusing the sealed serving inputs: '{path}' {problem}; the inputs a final command \
             is rebuilt from are never repaired into empty or default ones, nor recovered from \
             its argv (rebuild unit 14a2; design D5.7, D6)"
        )
    };
    // Replace the member at `pointer`, or remove it where `None`.
    let edit = |pointer: &str, replacement: Option<Value>| {
        let mut inputs = valid.clone();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        match (replacement, inputs.pointer_mut(parent).unwrap()) {
            (Some(value), Value::Array(items)) => items[key.parse::<usize>().unwrap()] = value,
            (Some(value), parent) => parent[key] = value,
            (None, parent) => drop(parent.as_object_mut().unwrap().remove(key)),
        }
        Some(inputs)
    };
    let unknown = |pointer: &str| {
        let mut inputs = valid.clone();
        inputs.pointer_mut(pointer).unwrap()[sentinel.as_str()] = json!(sentinel);
        Some(inputs)
    };
    let (missing, null, unknown_member) = ("is missing", "is null", "carries an unknown member");
    let rows: Vec<Malformed> = vec![
        ("absent inputs", None, cause("serving", missing)),
        ("null inputs", Some(Value::Null), cause("serving", null)),
        (
            "inputs not an object",
            Some(json!([sentinel])),
            cause("serving", "is not an object"),
        ),
        (
            "inputs unknown member",
            unknown(""),
            cause("serving", unknown_member),
        ),
        (
            "dialect missing",
            edit("/dialect", None),
            cause("serving.dialect", missing),
        ),
        (
            "pins missing",
            edit("/pins", None),
            cause("serving.pins", missing),
        ),
        (
            "spec null",
            edit("/spec", Some(Value::Null)),
            cause("serving.spec", null),
        ),
        (
            "dialect unknown member",
            unknown("/dialect"),
            cause("serving.dialect", unknown_member),
        ),
        (
            "boundary missing",
            edit("/dialect/boundary", None),
            cause("serving.dialect.boundary", missing),
        ),
        (
            "permissions unknown kind",
            edit("/dialect/permissions/kind", Some(json!(sentinel))),
            cause("serving.dialect.permissions.kind", "names no known kind"),
        ),
        (
            "permissions flag missing",
            edit("/dialect/permissions/flag", None),
            cause("serving.dialect.permissions.flag", missing),
        ),
        (
            "permissions flag not a string",
            edit("/dialect/permissions/flag", Some(json!([sentinel]))),
            cause("serving.dialect.permissions.flag", "is not a string"),
        ),
        (
            "permissions separator not a string",
            edit("/dialect/permissions/separator", Some(json!([sentinel]))),
            cause("serving.dialect.permissions.separator", "is not a string"),
        ),
        (
            "none permissions carrying a flag",
            edit(
                "/dialect/permissions",
                Some(json!({"kind": "none", "flag": "--allowedTools"})),
            ),
            cause("serving.dialect.permissions", unknown_member),
        ),
        (
            "spec unknown kind",
            edit("/spec/kind", Some(json!(sentinel))),
            cause("serving.spec.kind", "names no known kind"),
        ),
        (
            "typed spec without its declaration",
            edit("/spec/declaration", None),
            cause("serving.spec.declaration", missing),
        ),
        (
            "declaration not a hands declaration",
            edit("/spec/declaration/kind", Some(json!(sentinel))),
            cause("serving.spec.declaration", "is not a hands declaration"),
        ),
        (
            "declaration in its short spelling",
            edit("/spec/declaration", Some(json!("workspace"))),
            cause(
                "serving.spec.declaration",
                "is not a hands declaration's canonical form",
            ),
        ),
        (
            "declaration with an omitted member",
            edit("/spec/declaration/binds/0/mask", None),
            cause(
                "serving.spec.declaration",
                "is not a hands declaration's canonical form",
            ),
        ),
        (
            "sandbox not an array",
            edit("/dialect/sandbox", Some(json!(sentinel))),
            cause("serving.dialect.sandbox", "is not an array"),
        ),
        (
            "hands argument not a string",
            edit("/dialect/hands/2", Some(json!({"x": sentinel}))),
            cause("serving.dialect.hands[2]", "is not a string"),
        ),
        (
            "boundary argument not a string",
            edit("/dialect/boundary/5", Some(json!(1))),
            cause("serving.dialect.boundary[5]", "is not a string"),
        ),
        (
            "boundary not an array",
            edit("/dialect/boundary", Some(json!({"gate": [], "work": []}))),
            cause("serving.dialect.boundary", "is not an array"),
        ),
        // Rebuild unit 14a4c: the boundary the site stood under is a
        // closed word, never recovered from its fragments.
        (
            "stands missing",
            edit("/dialect/stands", None),
            cause("serving.dialect.stands", missing),
        ),
        (
            "stands a bare word",
            edit("/dialect/stands", Some(json!("harness"))),
            cause("serving.dialect.stands", "is not an object"),
        ),
        (
            "stands an unknown boundary",
            edit("/dialect/stands/kind", Some(json!("not applicable"))),
            cause("serving.dialect.stands.kind", "names no known kind"),
        ),
        (
            "stands carrying a fragment",
            edit(
                "/dialect/stands",
                Some(json!({"kind": "harness", "work": []})),
            ),
            cause("serving.dialect.stands", unknown_member),
        ),
        (
            "pin not a string",
            edit("/pins/3", Some(Value::Bool(true))),
            cause("serving.pins[3]", "is not a string"),
        ),
    ];
    assert_eq!(rows.len(), 28);
    // Every row reaches its own exact assertion; no cause carries the
    // sentinel it was handed.
    let failures: Vec<String> =
        rows.iter()
            .filter_map(|(label, inputs, expected)| {
                let observed = SealedServing::decode(inputs.as_ref());
                let bounded = observed.as_ref().err().is_some_and(|cause| {
                    cause.chars().count() <= 512 && !cause.contains("SENTINEL")
                });
                (observed.as_ref() != Err(expected) || !bounded)
                    .then(|| format!("row {label}:\n  left:  {observed:?}\n  right: {expected:?}"))
            })
            .collect();
    assert!(
        failures.is_empty(),
        "{} of {} rows failed:\n{}",
        failures.len(),
        rows.len(),
        failures.join("\n")
    );
}

/// U1g2: the engine's MCP isolation intent is a closed member of the sealed
/// inputs. An absent one is never served as the default, and one missing a
/// member, naming an unknown word or carrying an unknown member refuses with
/// its fixed path, echoing nothing it was handed.
#[test]
fn a_sealed_isolation_intent_is_read_closed_and_never_defaulted() {
    let valid = full_serving().value();
    let refused = |problem: &str| {
        Err(format!(
            "refusing the sealed serving inputs: 'serving.isolation' {problem}; the inputs a final \
             command is rebuilt from are never repaired into empty or default ones, nor recovered \
             from its argv (rebuild unit 14a2; design D5.7, D6)"
        ))
    };
    let unread = refused("is not an MCP isolation intent");
    let edit = |change: fn(&mut Value)| {
        let mut inputs = valid.clone();
        change(&mut inputs["isolation"]);
        SealedServing::decode(Some(&inputs))
    };
    let mut absent = valid.clone();
    absent.as_object_mut().unwrap().remove("isolation");
    assert_eq!(SealedServing::decode(Some(&absent)), refused("is missing"));
    type Change = fn(&mut Value);
    let cases: [(&str, Change); 4] = [
        ("fence missing", |intent| {
            drop(intent.as_object_mut().unwrap().remove("fence"))
        }),
        ("unknown set", |intent| {
            intent["servers"] = json!("SENTINEL")
        }),
        ("unknown member", |intent| intent["SENTINEL"] = json!(true)),
        ("unknown assessment", |intent| {
            intent["cold"] = json!({"measured": "SENTINEL"})
        }),
    ];
    for (case, change) in cases {
        assert_eq!(edit(change), unread, "{case}");
    }
}
