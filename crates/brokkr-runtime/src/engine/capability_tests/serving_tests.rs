//! The typed serving inputs sealed beside a launch record (rebuild unit
//! 14a2), moved here from `capability_tests.rs`, and the MCP isolation
//! intent dispatch seals among them (decision 0065 slice two, U1g2).

use super::*;

/// The intent dispatch seals for a serving whose shapes carry no SI2
/// record, under the standing fence, naming `servers`.
fn standing(servers: &str) -> Value {
    json!({"servers": servers, "cold": "unmeasured", "replacement": "unmeasured",
           "resume": {}, "fence": "standing"})
}

/// The dispatch door's refusal of serving inputs that decode but differ.
const NOT_SEALED: &str = "dispatch refused: the serving inputs handed over are not the ones the \
                          engine sealed beside this spawn's record; a dialect, pin or hands \
                          declaration that differs is never trusted by its shape (rebuild unit \
                          14a2; design D5.7, D6)";

/// Rebuild unit 14a2: the typed serving inputs the final check rebuilds a
/// command from are sealed beside the record from the selected link's own
/// composition — or the inline site's recorded dialect and hands — with the
/// boundary fragment its class selects, written into the input, and
/// admitted by the dispatch door exactly as sealed. A missing, malformed or
/// altered copy, one planted before sealing and one handed where none was
/// sealed each refuse with the whole reason; a site whose inputs were never
/// recorded, and a spawn no composition classed, seal nothing.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn the_serving_inputs_are_sealed_beside_the_record_and_admitted_only_as_sealed() {
    use crate::agents::{BoundaryFragments, DeclaredDialect, ServingInputs};
    let (_dir, mut engine) = two_candidate_engine();
    let spec = HandsSpec::parse(&json!({"kind": "workspace", "network": true,
        "binds": [{"path": "~/.cargo", "mode": "ro", "mask": ["credentials.toml"]}]}))
    .unwrap();
    let carrying = |mut link: Candidate, serving: ServingInputs| {
        if let Lowering::Composed(composition) = &mut link.lowering {
            *composition.serving = serving;
        }
        link
    };
    let flag = |flag: &str| {
        Some(brokkr_protocol::native_controls::ListFlag {
            flag: flag.into(),
            separator: ",".into(),
        })
    };

    // The Codex primary's boxed hands: its workspace fragment unexpanded,
    // its pin and its typed hands.
    let primary = carrying(
        codex_primary(),
        ServingInputs {
            dialect: DeclaredDialect {
                hands: strings(&["-c", "mcp_servers.brokkr.args={hands_args_toml}"]),
                ..DeclaredDialect::default()
            },
            pins: strings(&["--model", "gpt-6-astra"]),
            spec: Some(spec.clone()),
        },
    );
    let (spawn, sealed) = marked(&engine, &primary, json!({}));
    assert_eq!(spawn.refusal, None);
    assert_eq!(
        sealed[SERVING_INPUTS],
        json!({
            "dialect": {
                "permissions": {"kind": "none"},
                "sandbox": [],
                "hands": ["-c", "mcp_servers.brokkr.args={hands_args_toml}"],
                "boundary": [],
                "stands": {"kind": "none"},
            },
            "pins": ["--model", "gpt-6-astra"],
            "spec": {"kind": "typed", "declaration": {"kind": "workspace", "network": true,
                "binds": [{"path": "~/.cargo", "mode": "ro", "mask": ["credentials.toml"]}]}},
            "isolation": standing("hands"),
        })
    );
    assert_eq!(
        spawn.serving,
        Some(SealedServing {
            dialect: SealedDialect {
                hands: strings(&["-c", "mcp_servers.brokkr.args={hands_args_toml}"]),
                ..SealedDialect::default()
            },
            pins: strings(&["--model", "gpt-6-astra"]),
            spec: Some(spec.clone()),
            isolation: serde_json::from_value(standing("hands")).unwrap(),
        })
    );
    assert_eq!(verify_record(&spawn, &sealed), Ok(()));

    // The DSH fallback carries its own: its permission flag, both boundary
    // fragments, and no hands. The spawn's class selects the one sealed.
    let fallback = carrying(
        dsh_fallback(),
        ServingInputs {
            dialect: DeclaredDialect {
                permissions: flag("--allowedTools"),
                boundary: BoundaryFragments {
                    gate: strings(&["--gate", "{result_path}"]),
                    work: strings(&["--work", "{brokkr}"]),
                },
                ..DeclaredDialect::default()
            },
            pins: strings(&["--model", "flash"]),
            spec: None,
        },
    );
    let fallback_serving = |boundary: &[&str]| {
        json!({
            "dialect": {
                "permissions": {"kind": "flag", "flag": "--allowedTools", "separator": ","},
                "sandbox": [],
                "hands": [],
                "boundary": boundary,
                "stands": {"kind": "none"},
            },
            "pins": ["--model", "flash"],
            "spec": {"kind": "none"},
            "isolation": standing("empty"),
        })
    };
    let (fallen, input) = marked(&engine, &fallback, json!({}));
    assert_eq!(
        input[SERVING_INPUTS],
        fallback_serving(&["--work", "{brokkr}"])
    );
    assert_eq!(verify_record(&fallen, &input), Ok(()));
    let mut gate = compose_site(
        BuiltBoundary::Open,
        SeatClass::Gate,
        fallback.argv.clone(),
        None,
        Some(&fallback),
        Path::new("/w"),
        &[],
        "/w/result.json",
        None,
    );
    let mut gated = json!({});
    engine.mark_capabilities("work", Some(&fallback), Some(&mut gate), &mut gated);
    assert_eq!(
        gated[SERVING_INPUTS],
        fallback_serving(&["--gate", "{result_path}"])
    );
    assert_eq!(verify_record(&gate, &gated), Ok(()));
    // The other class's fragment is another input.
    let mut exchanged = input.clone();
    exchanged[SERVING_INPUTS]["dialect"]["boundary"] = json!(["--gate", "{result_path}"]);
    let not_sealed = NOT_SEALED;
    assert_eq!(
        verify_record(&fallen, &exchanged),
        Err(not_sealed.to_string())
    );
    // A spawn no composition classed selects no fragment and seals nothing.
    let mut unclassed = SiteSpawn::inherit(fallback.argv.clone());
    let mut input = json!({});
    engine.mark_capabilities("work", Some(&fallback), Some(&mut unclassed), &mut input);
    assert_eq!(
        unclassed.refusal.as_deref(),
        Some(
            "dispatch refused: the spawn was composed for no seat class, so none of the boundary \
             fragments its serving inputs carry can be selected and sealed; a fragment is never \
             chosen by default (decision 0046 ruling 4; rebuild unit 14a2)"
        )
    );
    assert_eq!((&unclassed.record, &unclassed.serving), (&None, &None));
    assert_eq!(
        (input.get(LAUNCH_RECORD), input.get(SERVING_INPUTS)),
        (None, None)
    );
    // Nor is one whose inputs carry no fragment to select (unit 26c): every
    // composition classes its spawn, so a classless one is never sealed.
    let mut bare = SiteSpawn::inherit(primary.argv.clone());
    engine.mark_capabilities("work", Some(&primary), Some(&mut bare), &mut json!({}));
    assert_eq!(
        (&bare.refusal, &bare.record, &bare.serving),
        (&unclassed.refusal, &None, &None)
    );

    // Missing, then malformed: the strict reader's own whole causes.
    let decoded = |path: &str, problem: &str| {
        Err(format!(
            "refusing the sealed serving inputs: '{path}' {problem}; the inputs a final command \
             is rebuilt from are never repaired into empty or default ones, nor recovered from \
             its argv (rebuild unit 14a2; design D5.7, D6)"
        ))
    };
    let mut missing = sealed.clone();
    missing.as_object_mut().unwrap().remove(SERVING_INPUTS);
    assert_eq!(
        verify_record(&spawn, &missing),
        decoded("serving", "is missing")
    );
    let mut malformed = sealed.clone();
    malformed[SERVING_INPUTS]["spec"]["declaration"] = json!("workspace");
    assert_eq!(
        verify_record(&spawn, &malformed),
        decoded(
            "serving.spec.declaration",
            "is not a hands declaration's canonical form"
        )
    );
    // Well-formed but altered: a pin, the hands and the typed declaration.
    for (pointer, value) in [
        ("/pins/1", json!("gpt-6-luna")),
        ("/dialect/hands/1", json!("mcp_servers.brokkr.args=[]")),
        ("/spec/declaration/network", json!(false)),
    ] {
        let mut altered = sealed.clone();
        *altered[SERVING_INPUTS].pointer_mut(pointer).unwrap() = value;
        assert_eq!(
            verify_record(&spawn, &altered),
            Err(not_sealed.to_string()),
            "{pointer}"
        );
    }

    // Planted before sealing — even as the very inputs the engine seals —
    // refuses the spawn, and the engine's own are still what is written.
    let planted = json!({SERVING_INPUTS: sealed[SERVING_INPUTS].clone()});
    let (overridden, input) = marked(&engine, &primary, planted);
    assert_eq!(
        overridden.refusal.as_deref(),
        Some(
            "dispatch refused: the input arrived carrying sealed serving inputs \
             ('serving_inputs') before the engine sealed any; a recipe, a result or a context \
             cannot supply them, even ones equal to the engine's (rebuild unit 14a2; design D5.7)"
        )
    );
    assert_eq!(input[SERVING_INPUTS], sealed[SERVING_INPUTS]);

    // Handed where nothing was sealed.
    let unsealed = SiteSpawn::inherit(strings(&["/bin/brokkr", "driver", "exec", "--"]));
    assert_eq!(
        verify_record(&unsealed, &json!({SERVING_INPUTS: Value::Null})),
        Err(
            "dispatch refused: the input carries sealed serving inputs ('serving_inputs') the \
             engine sealed none for, and they are never accepted from anything but the dispatch \
             that sealed them (rebuild unit 14a2; design D5.7)"
                .to_string()
        )
    );

    // An inline site: its recorded dialect, no pins, and no hands.
    lowered_inline(&mut engine, "work", "cargo");
    engine.bundle.sites.get_mut("work").unwrap().inline_dialect = Some(DeclaredDialect {
        permissions: flag("--allowedTools"),
        ..DeclaredDialect::default()
    });
    let inline = |engine: &Engine| {
        compose_site_at(
            engine.bundle.sites.get("work"),
            BuiltBoundary::Open,
            SeatClass::Work,
            strings(&["/bin/brokkr", "driver", "dsh", "--"]),
            None,
            None,
            Path::new("/w"),
            &[],
            "/w/result.json",
            None,
        )
    };
    let mut spawn = inline(&engine);
    assert_eq!(spawn.class, Some(SeatClass::Work));
    let mut input = json!({});
    engine.mark_capabilities("work", None, Some(&mut spawn), &mut input);
    assert_eq!(spawn.refusal, None);
    assert_eq!(
        input[SERVING_INPUTS],
        json!({
            "dialect": {
                "permissions": {"kind": "flag", "flag": "--allowedTools", "separator": ","},
                "sandbox": [],
                "hands": [],
                "boundary": [],
                "stands": {"kind": "none"},
            },
            "pins": [],
            "spec": {"kind": "none"},
            "isolation": standing("empty"),
        })
    );
    assert_eq!(verify_record(&spawn, &input), Ok(()));

    // An inline site whose dialect was never recorded seals nothing.
    engine.bundle.sites.get_mut("work").unwrap().inline_dialect = None;
    let mut spawn = inline(&engine);
    let mut input = json!({});
    engine.mark_capabilities("work", None, Some(&mut spawn), &mut input);
    assert_eq!(
        spawn.refusal.as_deref(),
        Some(
            "dispatch refused: the site's typed serving inputs were never recorded, so none can \
             be sealed beside its launch record; they are carried from the composition and never \
             recovered from its argv (rebuild unit 14a2; design D5.7, D6)"
        )
    );
    assert_eq!((spawn.record, spawn.serving), (None, None));
    assert_eq!(
        (input.get(LAUNCH_RECORD), input.get(SERVING_INPUTS)),
        (None, None)
    );
}

/// U1g2: an inline site whose MCP server set the compile never recorded
/// seals nothing and refuses its spawn; the set is never derived again from
/// its argv, and the same site with its set recorded seals the empty one.
#[test]
fn an_inline_site_with_no_recorded_server_set_seals_nothing() {
    let (_dir, mut engine) = two_candidate_engine();
    lowered_inline(&mut engine, "work", "cargo");
    let mark = |engine: &Engine| {
        let spawn = compose_site_at(
            engine.bundle.sites.get("work"),
            BuiltBoundary::Open,
            SeatClass::Work,
            strings(&["/bin/brokkr", "driver", "claude", "--"]),
            None,
            None,
            Path::new("/w"),
            &[],
            "/w/result.json",
            None,
        );
        let (mut spawn, mut input) = (spawn, json!({}));
        engine.mark_capabilities("work", None, Some(&mut spawn), &mut input);
        (
            spawn.refusal,
            spawn.serving.map(|serving| serving.isolation.servers),
        )
    };
    use brokkr_protocol::native_controls::SealedServers;
    assert_eq!(mark(&engine), (None, Some(SealedServers::Empty)));
    engine.bundle.sites.get_mut("work").unwrap().inline_mcp = None;
    let unrecorded = "dispatch refused: the site's MCP server set was never recorded, so no \
                      isolation intent is sealed beside its launch (decision 0065 slice two, U1g2)";
    assert_eq!(mark(&engine), (Some(unrecorded.to_string()), None));
}

/// U1g2: the intent sealed with each serving is its own — the set its own
/// composition recorded and each shape's SI2 record beside its own outcome,
/// so the DSH fallback never borrows the Codex primary's assessments — and
/// names the fence it was dispatched under. A shape with no record, or one
/// recorded unmeasured, is unmeasured.
#[test]
fn each_serving_seals_its_own_isolation_intent_under_the_fence() {
    use crate::agents::{McpAxis, McpHands, McpHost, McpInvocation, McpShape, McpUnmeasured};
    let (_dir, mut engine) = canonical_engine(single_body(vec!["driver".into()]));
    let shape = |invocation| McpShape {
        invocation,
        hands: McpHands::NoHands,
        host: McpHost::Linux,
    };
    let measured = || McpAxis::Measured {
        evidence: "U0".into(),
    };
    let unsupported = McpAxis::Unsupported {
        reason: "project MCP".into(),
    };
    let mut site = two_candidates();
    site.strict = vec![
        vec![
            (shape(McpInvocation::Cold), unsupported),
            (shape(McpInvocation::Replacement), measured()),
        ],
        vec![
            (shape(McpInvocation::Cold), measured()),
            (
                shape(McpInvocation::Resume("work-site".into())),
                McpAxis::Unmeasured(McpUnmeasured::Absent),
            ),
            (shape(McpInvocation::Resume("headless".into())), measured()),
        ],
    ];
    engine
        .bundle
        .sites
        .entry("work".into())
        .or_default()
        .capabilities = Some(site);
    let intent = |link: Candidate| {
        let (spawn, input) = marked(&engine, &link, json!({}));
        assert_eq!(spawn.refusal, None);
        input[SERVING_INPUTS]["isolation"].clone()
    };
    assert_eq!(
        intent(codex_primary()),
        json!({"servers": "hands", "cold": {"unsupported": "project MCP"},
               "replacement": "measured", "resume": {}, "fence": "standing"})
    );
    // Each resume shape is sealed under its own name, an unmeasured one
    // recorded first never standing in for a measured one after it.
    assert_eq!(
        intent(dsh_fallback()),
        json!({"servers": "empty", "cold": "measured", "replacement": "unmeasured",
               "resume": {"work-site": "unmeasured", "headless": "measured"},
               "fence": "standing"})
    );
    let _lifted = crate::capabilities::McpFence::lift();
    assert_eq!(intent(dsh_fallback())["fence"], json!("lifted"));
}

/// U1g2: an isolation intent that is well formed but not the one sealed —
/// another server set, any shape's assessment, a resume shape's entry or
/// the fence — is refused at the dispatch door with its whole reason, before
/// any launch, exactly as an altered pin or hands declaration is.
#[test]
fn an_altered_isolation_intent_is_refused_at_the_door() {
    let (_dir, engine) = two_candidate_engine();
    let (spawn, sealed) = marked(&engine, &codex_primary(), json!({}));
    assert_eq!(
        (spawn.refusal.as_deref(), verify_record(&spawn, &sealed)),
        (None, Ok(()))
    );
    for (pointer, value) in [
        ("/servers", json!("empty")),
        ("/cold", json!("measured")),
        ("/replacement", json!({"unsupported": "project MCP"})),
        ("/resume", json!({"work-site": "measured"})),
        ("/fence", json!("lifted")),
    ] {
        let mut altered = sealed.clone();
        *altered[SERVING_INPUTS]["isolation"]
            .pointer_mut(pointer)
            .unwrap() = value;
        assert_eq!(
            verify_record(&spawn, &altered),
            Err(NOT_SEALED.to_string()),
            "{pointer}"
        );
    }
}

/// The sealed boundary's boxed vocabulary, which the serving edge declares
/// a hands set by, is core's `Boundary::is_boxed`, word for word (ruling 5).
#[test]
fn a_sealed_boundary_boxes_exactly_where_core_rules_it() {
    for boundary in brokkr_core::realms::BOUNDARIES {
        let sealed = SealedBoundary::named(boundary.word()).unwrap();
        assert_eq!(sealed.boxes(), boundary.is_boxed(), "{boundary}");
    }
}
