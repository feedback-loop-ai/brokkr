use super::*;
use serde_json::json;
use std::collections::BTreeMap;

fn error<T>(result: Result<T, CompileError>) -> String {
    match result {
        Ok(_) => panic!("expected compilation to fail"),
        Err(error) => error.to_string(),
    }
}

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("roles")).unwrap();
        std::fs::write(dir.path().join("roles/role.md"), "# role").unwrap();
        Self { dir }
    }

    fn policy() -> Value {
        json!({
            "phases": ["work", "review", "done"],
            "initial": "work",
            "terminal": ["done"],
            "rules": [
                {"id":"WORK", "from":"work", "result":"complete", "next":"review", "reason":"work"},
                {"id":"REVIEW", "from":"review", "result":"clean", "next":"done", "reason":"review"},
            ],
        })
    }

    fn config() -> Value {
        json!({
            "name": "fixture",
            "policy": "policy.json",
            "seats": {
                "work": {
                    "results": ["complete"],
                    "role": "roles/role.md",
                    "driver": {"command": ["driver"]},
                },
                "review": {
                    "results": ["clean"],
                    "role": "roles/role.md",
                    "driver": {"command": ["driver"]},
                },
            },
        })
    }

    fn compile(&self, config: &Value, policy: &Value) -> Result<Bundle, CompileError> {
        std::fs::write(
            self.dir.path().join("bundle.json"),
            serde_json::to_vec(config).unwrap(),
        )
        .unwrap();
        std::fs::write(
            self.dir.path().join("policy.json"),
            serde_json::to_vec(policy).unwrap(),
        )
        .unwrap();
        Bundle::compile(self.dir.path())
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn dialect_config(verify: Value) -> (Value, Value) {
    let policy = json!({
        "phases":["design","verify","review","done"], "initial":"design", "terminal":["done"],
        "rules":[
            {"id":"D","from":"design","result":"drafted","next":"verify","reason":"drafted"},
            {"id":"DF","from":"design","result":"fail","next":"design","reason":"retry"},
            {"id":"V","from":"verify","result":"pass","next":"review","reason":"pass"},
            {"id":"VF","from":"verify","result":"fail","next":"verify","reason":"retry"},
            {"id":"R","from":"review","result":"clean","next":"done","reason":"clean"}
        ]
    });
    let config = json!({
        "name":"dialect-fixture", "policy":"policy.json", "protected_phase":"review",
        "seats":{
            "design":{"results":["drafted","fail"],"sequence":[
                {"name":"author","results":["drafted"],"role":"roles/role.md","driver":{"command":["driver"]}},
                {"name":"validate","dialect":"validate"}
            ]},
            "verify":verify,
            "review":{"results":["clean"],"role":"roles/role.md","driver":{"command":["driver"]}}
        }
    });
    (config, policy)
}

fn compile_dialect_fixture(
    fixture: &Fixture,
    config: &Value,
    policy: &Value,
    dialect: Option<&Dialect>,
) -> Result<Bundle, CompileError> {
    std::fs::write(
        fixture.dir.path().join("bundle.json"),
        serde_json::to_vec(config).unwrap(),
    )
    .unwrap();
    std::fs::write(
        fixture.dir.path().join("policy.json"),
        serde_json::to_vec(policy).unwrap(),
    )
    .unwrap();
    let root = workspace_root();
    Bundle::compile_with_realm(
        fixture.dir.path(),
        &root.join("agents"),
        &root.join("adapters"),
        None,
        dialect,
        Boundary::Namespace,
    )
}

#[test]
fn dialect_sites_and_verify_composition_cover_every_body_boundary() {
    let fixture = Fixture::new();
    let root = workspace_root();
    let dialect = Dialect::load(&root.join("dialects/openspec.json"))
        .unwrap()
        .0;
    let single = json!({"results":["pass","fail"],"class":"gate","hands":"workspace","role":"roles/role.md","driver":{"command":["{brokkr}","driver","exec","--","true"]}});
    let (config, policy) = dialect_config(single.clone());
    let compiled = compile_dialect_fixture(&fixture, &config, &policy, Some(&dialect)).unwrap();
    assert!(matches!(
        compiled.seats["verify"].body,
        SeatBody::Sequence { .. }
    ));
    let (mut extended_config, mut extended_policy) = dialect_config(single.clone());
    extended_config["seats"]["verify"]["results"] = json!(["pass", "fail", "manual"]);
    extended_policy["rules"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"VM", "from":"verify", "result":"manual", "next":"review", "reason":"manual"
        }));
    let extended =
        compile_dialect_fixture(&fixture, &extended_config, &extended_policy, Some(&dialect))
            .unwrap();
    let SeatBody::Sequence { steps } = &extended.seats["verify"].body else {
        panic!("dialect verification is synthesized as a sequence");
    };
    assert_eq!(steps.last().unwrap().results, ["pass", "fail"]);
    assert_eq!(
        body_manifest(&SeatBody::Sequence {
            steps: vec![SequenceStep {
                name: "dialect".into(),
                results: vec!["drafted".into()],
                class: SeatClass::Gate,
                body: StepBody::Dialect {
                    execution: DialectExecution {
                        argv: vec!["validate".into()],
                        state: None,
                    },
                },
            }],
        })["sequence"][0]["body"]["dialect"]["argv"][0],
        "validate"
    );
    let (mut invalid_hands, invalid_policy) = dialect_config(single.clone());
    invalid_hands["seats"]["verify"]["hands"] = json!({"kind":"unknown"});
    assert!(
        compile_dialect_fixture(&fixture, &invalid_hands, &invalid_policy, Some(&dialect)).is_err()
    );

    let work_verify =
        json!({"results":["pass","fail"],"role":"roles/role.md","driver":{"command":["driver"]}});
    let (config, policy) = dialect_config(work_verify);
    assert!(compile_dialect_fixture(&fixture, &config, &policy, Some(&dialect)).is_ok());

    let panel = json!({
        "results":["pass","fail"], "aggregate":"unanimous-pass", "panel":{
            "one":{"role":"roles/role.md","driver":{"command":["driver"]}},
            "two":{"role":"roles/role.md","driver":{"command":["driver"]}}
        }
    });
    let (config, policy) = dialect_config(panel);
    assert!(compile_dialect_fixture(&fixture, &config, &policy, Some(&dialect)).is_ok());

    let sequence = json!({
        "results":["pass","fail"], "sequence":[
            {"name":"one","results":["pass"],"role":"roles/role.md","driver":{"command":["driver"]}},
            {"name":"two","role":"roles/role.md","driver":{"command":["driver"]}}
        ]
    });
    let (config, policy) = dialect_config(sequence);
    assert!(error(compile_dialect_fixture(
        &fixture,
        &config,
        &policy,
        Some(&dialect)
    ))
    .contains("requires a single or panel verify seat"));
}

/// Design D10's second verify refusal, isolated from the sequence guard.
/// A wrapped verify seat may legally be a strategy `Select` whose EMPTY
/// case map defers to a single default. Resolving that default yields a
/// `Single`, so the first `selected(None)` guard admits it; the refusal
/// under test is the outer-body match, which sees a `SeatBody::Select`
/// and refuses to clone it into the wrapper's `StepBody`. The existing
/// sequence control above proves the first guard independently.
#[test]
fn a_dialect_wrapped_verify_select_reaches_the_single_or_panel_refusal() {
    let fixture = Fixture::new();
    let root = workspace_root();
    let dialect = Dialect::load(&root.join("dialects/openspec.json"))
        .unwrap()
        .0;
    let verify = json!({
        "results":["pass","fail"],
        "select": {
            "on":"strategy",
            "cases": {},
            "default": {"role":"roles/role.md","driver":{"command":["driver"]}}
        }
    });
    let (config, policy) = dialect_config(verify);
    let message = error(compile_dialect_fixture(
        &fixture,
        &config,
        &policy,
        Some(&dialect),
    ));
    assert!(
        message.contains("dialect verify currently requires a single or panel verify seat"),
        "{message}"
    );
}

/// Design D10's selected-agent cause. An empty `agent` reference inside a
/// legal named selector case must reach `resolve_reference` and retain
/// its own diagnostic, selector-qualified, rather than being swallowed by
/// a later load step or replaced by an arbitrary compile error. The
/// default is valid and the outer seat owns the results, so no earlier
/// shape or key refusal can mask the continuation under test.
#[test]
fn a_selected_agent_case_keeps_its_empty_reference_cause() {
    let fixture = Fixture::new();
    let policy = Fixture::policy();
    let mut config = Fixture::config();
    let work = config["seats"]["work"].as_object_mut().unwrap();
    work.remove("role");
    work.remove("driver");
    work.insert(
        "select".into(),
        json!({
            "on":"strategy",
            "cases": {"engine": {"agent": ""}},
            "default": {"role":"roles/role.md","driver":{"command":["driver"]}}
        }),
    );
    let message = error(compile_dialect_fixture(&fixture, &config, &policy, None));
    assert!(
        message.contains("seat 'work:engine' agent must be a non-empty string"),
        "{message}"
    );
}

#[test]
fn dialect_site_vocabulary_and_support_fail_closed() {
    let fixture = Fixture::new();
    let root = workspace_root();
    let dialect = Dialect::load(&root.join("dialects/openspec.json"))
        .unwrap()
        .0;
    let verify =
        json!({"results":["pass","fail"],"role":"roles/role.md","driver":{"command":["driver"]}});
    for (site, expected) in [
        (json!(3), "must be 'validate'"),
        (json!("check"), "needs 'validate'"),
    ] {
        let (mut config, policy) = dialect_config(verify.clone());
        config["seats"]["design"]["sequence"][1]["dialect"] = site;
        assert!(error(compile_dialect_fixture(
            &fixture,
            &config,
            &policy,
            Some(&dialect)
        ))
        .contains(expected));
    }
    let (config, policy) = dialect_config(verify.clone());
    assert!(
        error(compile_dialect_fixture(&fixture, &config, &policy, None))
            .contains("needs a realm dialect")
    );

    let mut value: Value =
        serde_json::from_slice(&std::fs::read(root.join("dialects/openspec.json")).unwrap())
            .unwrap();
    value["phases"]["design"]["validate"] = json!({"unsupported":"no validator"});
    let mut unsupported = Dialect::parse("unsupported.json", &value.to_string())
        .unwrap()
        .0;
    unsupported.render(&root.join("dialects")).unwrap();
    let (config, policy) = dialect_config(verify);
    assert!(error(compile_dialect_fixture(
        &fixture,
        &config,
        &policy,
        Some(&unsupported)
    ))
    .contains("validate unsupported"));

    let loop_policy = json!({
        "phases":["clarify","review","done"], "initial":"clarify", "terminal":["done"],
        "rules":[
            {"id":"C","from":"clarify","result":"clear","next":"review","reason":"clear"},
            {"id":"CA","from":"clarify","result":"ambiguous","next":"clarify","reason":"retry"},
            {"id":"R","from":"review","result":"clean","next":"done","reason":"clean"}
        ]
    });
    let loop_config = json!({
        "name":"loop", "policy":"policy.json", "protected_phase":"review", "seats":{
            "clarify":{"results":["clear","ambiguous"],"sequence":[
                {"name":"count","results":["clear"],"dialect":"check"},
                {"name":"judge","role":"roles/role.md","driver":{"command":["driver"]}}
            ]},
            "review":{"results":["clean"],"role":"roles/role.md","driver":{"command":["driver"]}}
        }
    });
    assert!(compile_dialect_fixture(&fixture, &loop_config, &loop_policy, Some(&dialect)).is_ok());
}

#[test]
fn a_synthetic_verifier_is_subject_to_its_own_binding_policy() {
    let fixture = Fixture::new();
    let root = workspace_root();
    let dialect = Dialect::load(&root.join("dialects/openspec.json"))
        .unwrap()
        .0;
    let (config, policy) = dialect_config(json!({
        "results":["pass","fail"], "secrets":["TOKEN"], "role":"roles/role.md",
        "driver":{"command":["{brokkr}","driver","claude","--","--model","claude-sonnet-4","--effort","high"]}
    }));
    let adapters = fixture.dir.path().join("adapters");
    std::fs::create_dir(&adapters).unwrap();
    std::fs::copy(
        root.join("adapters/claude.json"),
        adapters.join("claude.json"),
    )
    .unwrap();
    std::fs::write(fixture.dir.path().join("bundle.json"), config.to_string()).unwrap();
    std::fs::write(fixture.dir.path().join("policy.json"), policy.to_string()).unwrap();
    let refusal = error(Bundle::compile_with_realm(
        fixture.dir.path(),
        &root.join("agents"),
        &adapters,
        None,
        Some(&dialect),
        Boundary::Namespace,
    ));
    assert!(
        refusal.contains("seat 'verify:dialect-verify' declares secret bindings"),
        "{refusal}"
    );
    assert!(refusal.contains("driver 'exec'"), "{refusal}");
}

#[test]
fn a_malformed_default_dialect_is_a_compile_refusal() {
    assert!(!needs_adapters(&json!({"plain":true})));
    assert!(needs_adapters(&json!({"dialect":"validate"})));
    let fixture = Fixture::new();
    let policy = Fixture::policy();
    let config = Fixture::config();
    std::fs::write(
        fixture.dir.path().join("bundle.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    std::fs::write(
        fixture.dir.path().join("policy.json"),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
    let library = tempfile::tempdir().unwrap();
    std::fs::create_dir(library.path().join("agents")).unwrap();
    std::fs::create_dir(library.path().join("dialects")).unwrap();
    std::fs::write(library.path().join("dialects/openspec.json"), "{").unwrap();
    let message = error(Bundle::compile_with(
        fixture.dir.path(),
        &library.path().join("agents"),
        &workspace_root().join("adapters"),
    ));
    assert!(message.contains("malformed"), "{message}");
}

#[test]
fn an_ordinary_verify_sequence_does_not_gain_a_dialect_check() {
    let fixture = Fixture::new();
    let policy = json!({
        "phases":["verify","review","done"], "initial":"verify", "terminal":["done"],
        "rules":[
            {"id":"V","from":"verify","result":"pass","next":"review","reason":"pass"},
            {"id":"VF","from":"verify","result":"fail","next":"verify","reason":"retry"},
            {"id":"R","from":"review","result":"clean","next":"done","reason":"clean"}
        ]
    });
    let config = json!({
        "name":"plain-verify", "policy":"policy.json", "protected_phase":"review", "seats":{
            "verify":{"results":["pass","fail"],"sequence":[
                {"name":"one","results":["pass"],"role":"roles/role.md","driver":{"command":["driver"]}},
                {"name":"two","role":"roles/role.md","driver":{"command":["driver"]}}
            ]},
            "review":{"results":["clean"],"role":"roles/role.md","driver":{"command":["driver"]}}
        }
    });
    assert!(fixture.compile(&config, &policy).is_ok());
}

#[test]
fn compiler_refuses_invalid_phase_seat_limit_and_input_shapes() {
    let fixture = Fixture::new();
    let policy = Fixture::policy();

    assert!(Bundle::compile(&fixture.dir.path().join("missing")).is_err());

    let mut config = Fixture::config();
    config.as_object_mut().unwrap().remove("name");
    assert!(error(fixture.compile(&config, &policy)).contains("missing 'name'"));

    let mut config = Fixture::config();
    config.as_object_mut().unwrap().remove("seats");
    assert!(error(fixture.compile(&config, &policy)).contains("missing 'seats'"));

    let mut config = Fixture::config();
    config["protected_phase"] = json!("missing");
    assert!(error(fixture.compile(&config, &policy)).contains("policy has no"));

    let mut config = Fixture::config();
    config["seats"]["unknown"] = config["seats"]["work"].clone();
    assert!(error(fixture.compile(&config, &policy)).contains("policy does not have"));

    let mut config = Fixture::config();
    config["seats"]["work"]["results"] = json!([]);
    assert!(error(fixture.compile(&config, &policy)).contains("non-empty 'results'"));

    let mut config = Fixture::config();
    config["seats"]["work"]["panel"] = json!({});
    assert!(error(fixture.compile(&config, &policy)).contains("exactly one"));

    for limits in [
        json!("bad"),
        json!({"max_attempts": 0}),
        json!({"invented": 1}),
    ] {
        let mut config = Fixture::config();
        config["seats"]["work"]["limits"] = limits;
        assert!(fixture.compile(&config, &policy).is_err());
    }

    let mut config = Fixture::config();
    config["seats"]["work"]["inputs"] = json!("bad");
    assert!(error(fixture.compile(&config, &policy)).contains("array of strings"));

    let mut config = Fixture::config();
    config["seats"]["work"]
        .as_object_mut()
        .unwrap()
        .remove("role");
    assert!(error(fixture.compile(&config, &policy)).contains("missing 'role'"));

    let mut config = Fixture::config();
    config["seats"].as_object_mut().unwrap().remove("work");
    assert!(error(fixture.compile(&config, &policy)).contains("has no seat"));
}

#[test]
fn select_parses_every_case_and_refuses_closed_vocabulary_defects_by_case() {
    let fixture = Fixture::new();
    let policy = Fixture::policy();
    let body = json!({"role":"roles/role.md", "driver":{"command":["driver"]}});
    let mut config = Fixture::config();
    config["seats"]["work"]
        .as_object_mut()
        .unwrap()
        .remove("role");
    config["seats"]["work"]
        .as_object_mut()
        .unwrap()
        .remove("driver");
    config["seats"]["work"]["select"] = json!({
        "on":"strategy",
        "cases": {
            "chore": body.clone(),
            "feature": body.clone(),
            "design": body.clone(),
            "engine": body
        }
    });
    let compiled = fixture.compile(&config, &policy).unwrap();
    assert!(compiled.manifest["select"]["work"].get("engine").is_some());
    assert!(matches!(
        compiled.seats["work"].body,
        SeatBody::Select { .. }
    ));
    assert!(compiled.seats["work"]
        .body
        .selected(Some("chore"))
        .is_some());
    assert!(compiled.seats["work"].body.selected(None).is_none());
    assert!(!compiled.seats["work"].body.selected_is_gate(None, false));
    assert!(!is_gate_class(
        &json!({"select":{"on":"strategy", "cases":{}}})
    ));
    assert!(is_gate_class(
        &json!({"select":{"on":"strategy", "cases":{},
        "default":{"class":"gate", "role":"roles/role.md", "driver":{"command":["driver"]}}}})
    ));
    let nested = SeatBody::Select {
        cases: BTreeMap::from([("chore".into(), compiled.seats["work"].body.clone())]),
        default: None,
        case_gates: BTreeMap::new(),
        default_gate: false,
    };
    assert!(nested.selected(Some("chore")).is_none());

    let mut not_an_object = config.clone();
    not_an_object["seats"]["work"]["select"] = json!("strategy");
    assert!(error(fixture.compile(&not_an_object, &policy)).contains("select must be an object"));

    let mut unknown_key = config.clone();
    unknown_key["seats"]["work"]["select"]["invented"] = json!(true);
    assert!(error(fixture.compile(&unknown_key, &policy)).contains("unknown key 'invented'"));

    let mut missing_on = config.clone();
    missing_on["seats"]["work"]["select"]
        .as_object_mut()
        .unwrap()
        .remove("on");
    assert!(error(fixture.compile(&missing_on, &policy)).contains("unknown 'on' <missing>"));

    let mut unknown = config.clone();
    unknown["seats"]["work"]["select"]["on"] = json!("weather");
    let refusal = error(fixture.compile(&unknown, &policy));
    assert!(refusal.contains("unknown 'on' weather") && refusal.contains("known: strategy"));

    let mut bad_cases = config.clone();
    bad_cases["seats"]["work"]["select"]["cases"] = json!([]);
    assert!(error(fixture.compile(&bad_cases, &policy)).contains("select.cases must be an object"));

    let mut unknown_case = config.clone();
    unknown_case["seats"]["work"]["select"]["cases"]["escalate"] =
        json!({"role":"roles/role.md", "driver":{"command":["driver"]}});
    assert!(
        error(fixture.compile(&unknown_case, &policy)).contains("unknown strategy case 'escalate'")
    );

    let mut missing = config.clone();
    missing["seats"]["work"]["select"]["cases"]
        .as_object_mut()
        .unwrap()
        .remove("engine");
    let refusal = error(fixture.compile(&missing, &policy));
    assert!(refusal.contains("class 'engine'") && refusal.contains("seat 'work'"));

    let mut with_default = config.clone();
    let default = with_default["seats"]["work"]["select"]["cases"]["chore"].clone();
    with_default["seats"]["work"]["select"]["cases"] = json!({"chore": default.clone()});
    with_default["seats"]["work"]["select"]["default"] = default;
    let compiled = fixture.compile(&with_default, &policy).unwrap();
    assert!(compiled.manifest["select"]["work"].get("default").is_some());
    assert_eq!(
        compiled.seats["work"].body.selected(None).unwrap().1,
        Some("default")
    );

    let mut bad = config;
    bad["seats"]["work"]["select"]["cases"]["feature"]["panel"] = json!({});
    let refusal = error(fixture.compile(&bad, &policy));
    assert!(refusal.contains("work:feature"), "{refusal}");
}

#[test]
fn panel_and_sequence_parsers_refuse_every_ambiguous_shape() {
    let fixture = Fixture::new();
    let dir = fixture.dir.path();
    let results = vec!["pass".to_string(), "fail".to_string()];

    for raw in [
        json!({}),
        json!({"panel": {}}),
        json!({"panel": {"a": {}, "b": {}}}),
        json!({"panel": {"a": {}, "b": {}}, "aggregate": "invented"}),
    ] {
        assert!(parse_panel(
            dir,
            "review",
            &raw,
            &results,
            &[],
            &mut None,
            &mut BTreeMap::new(),
            Boundary::Namespace,
        )
        .is_err());
    }
    let panel = json!({
        "panel": {
            "a": {"role":"roles/role.md", "driver":{"command":["driver"]}},
            "b": {"role":"roles/role.md", "driver":{"command":["driver"]}},
        },
        "aggregate": "unanimous-pass",
    });
    assert!(error(parse_panel(
        dir,
        "review",
        &panel,
        &[],
        &[],
        &mut None,
        &mut BTreeMap::new(),
        Boundary::Namespace,
    ))
    .contains("does not declare"));
    assert_eq!(
        parse_panel(
            dir,
            "review",
            &panel,
            &results,
            &[],
            &mut None,
            &mut BTreeMap::new(),
            Boundary::Namespace,
        )
        .unwrap()
        .0
        .len(),
        2
    );

    for raw in [
        json!({"sequence": "bad"}),
        json!({"sequence": []}),
        json!({"sequence": [{}, {"name":"two"}]}),
        json!({"sequence": [
            {"name":"same", "role":"roles/role.md", "driver":{"command":["driver"]}},
            {"name":"SAME", "role":"roles/role.md", "driver":{"command":["driver"]}},
        ]}),
        json!({"sequence": [
            {"name":"one", "role":"roles/role.md", "driver":{"command":["driver"]}, "panel":{}},
            {"name":"two", "role":"roles/role.md", "driver":{"command":["driver"]}},
        ]}),
        json!({"sequence": [
            {"name":"one", "results":[], "role":"roles/role.md", "driver":{"command":["driver"]}},
            {"name":"two", "role":"roles/role.md", "driver":{"command":["driver"]}},
        ]}),
    ] {
        assert!(parse_sequence(
            dir,
            "review",
            &raw,
            &mut None,
            &mut BTreeMap::new(),
            BodyCompile {
                results: &results,
                secrets: &[],
                dialect: None,
                boundary: Boundary::Namespace,
            }
        )
        .is_err());
    }

    let mismatch = json!({"sequence": [
        {"name":"panel", "results":["clean"], "aggregate":"unanimous-pass",
         "panel": {
            "a":{"role":"roles/role.md", "driver":{"command":["driver"]}},
            "b":{"role":"roles/role.md", "driver":{"command":["driver"]}}
         }},
        {"name":"final", "role":"roles/role.md", "driver":{"command":["driver"]}}
    ]});
    let refusal = error(parse_sequence(
        dir,
        "review",
        &mismatch,
        &mut None,
        &mut BTreeMap::new(),
        BodyCompile {
            results: &results,
            secrets: &[],
            dialect: None,
            boundary: Boundary::Namespace,
        },
    ));
    assert!(refusal.contains("can emit 'pass'"), "{refusal}");

    let final_vocabulary = json!({"sequence": [
        {"name":"first", "results":["drafted"],
         "role":"roles/role.md", "driver":{"command":["driver"]}},
        {"name":"final", "results":["invented"],
         "role":"roles/role.md", "driver":{"command":["driver"]}}
    ]});
    let refusal = error(parse_sequence(
        dir,
        "review",
        &final_vocabulary,
        &mut None,
        &mut BTreeMap::new(),
        BodyCompile {
            results: &results,
            secrets: &[],
            dialect: None,
            boundary: Boundary::Namespace,
        },
    ));
    assert!(
        refusal.contains("final and receives the seat's results"),
        "{refusal}"
    );

    let valid = json!({"sequence": [
        {"name":"first", "results":["drafted"],
         "role":"roles/role.md", "driver":{"command":["driver"]}},
        {"name":"final", "role":"roles/role.md", "driver":{"command":["driver"]}}
    ]});
    let steps = parse_sequence(
        dir,
        "review",
        &valid,
        &mut None,
        &mut BTreeMap::new(),
        BodyCompile {
            results: &results,
            secrets: &[],
            dialect: None,
            boundary: Boundary::Namespace,
        },
    )
    .unwrap();
    let pinned = body_manifest(&SeatBody::Sequence { steps });
    assert_eq!(pinned["sequence"][0]["results"], json!(["drafted"]));
    assert_eq!(pinned["sequence"][1]["results"], json!(results));
}

#[test]
fn role_secret_command_and_confinement_boundaries_are_explicit() {
    let fixture = Fixture::new();
    let dir = fixture.dir.path();
    assert!(parse_role(dir, "work", &json!({"role":"missing.md"})).is_err());

    for raw in [
        json!({"secrets": "bad"}),
        json!({"secrets": [2]}),
        json!({"secrets": ["TOKEN", "TOKEN"]}),
    ] {
        assert!(parse_secrets("work", &raw).is_err());
    }

    assert!(parse_command(dir, "work", &json!({}), &[]).is_err());
    let command = parse_command(
        dir,
        "work",
        &json!({"driver":{"command":["{brokkr}", "./tool", "plain"]}}),
        &[],
    )
    .unwrap();
    assert_eq!(command.len(), 3);
    assert!(command[1].ends_with("tool"));
    assert_eq!(command[2], "plain");

    // Decision 0019: the old token is the same token for one more
    // release. It resolves to exactly what `{brokkr}` resolves to — the
    // note it earns is said once by the one latch `legacy` owns, which
    // is where that property is pinned.
    let old = parse_command(
        dir,
        "work",
        &json!({"driver":{"command":["{forge}", "./tool", "plain"]}}),
        &[],
    )
    .unwrap();
    assert_eq!(old, command);
    // Read twice: an old token resolves the same every time, never once
    // and then differently.
    assert_eq!(
        expand_command(dir, &["{forge}".to_string(), "{brokkr}".to_string()]),
        vec![command[0].clone(), command[0].clone()]
    );

    // Decision 0046 ruling 5: the field is refused by name, in every
    // shape, and a site without it is untouched.
    for raw in [
        json!({"driver":{"confine":"bad"}}),
        json!({"driver":{"confine":{}}}),
        json!({"driver":{"confine":{"image":"img", "network":true, "mounts":["/x"]}}}),
    ] {
        let refusal = error(refuse_confine("work", &raw));
        assert!(
            refusal.contains("seat 'work' declares driver.confine"),
            "{refusal}"
        );
        assert!(refusal.contains("`container` boundary"), "{refusal}");
        assert!(refusal.contains("slice (iii)"), "{refusal}");
        assert!(refusal.contains("decision 0046 ruling 5"), "{refusal}");
    }
    refuse_confine("work", &json!({"driver":{"command":["x"]}})).unwrap();

    assert!(referenced_seat_inputs(&json!({}), "work").is_empty());
}

/// Decision 0046 ruling 5, at the compiler's front door: an inline seat
/// with `driver.confine` is refused naming the site, the `container`
/// boundary and the ruling; an agent seat with the same key is refused
/// the same way, and the field is no longer the one `driver` key legal
/// beside `agent:`. No shipped bundle declares it, and every shipped
/// bundle still compiles.
#[test]
fn driver_confine_is_refused_by_name_in_a_bundle_and_beside_an_agent() {
    let fixture = Fixture::new();
    let mut config = Fixture::config();
    config["seats"]["work"]["driver"] =
        json!({"command": ["./drivers/x"], "confine": {"image": "img"}});
    let refusal = error(fixture.compile(&config, &Fixture::policy()));
    assert!(
        refusal.contains("seat 'work' declares driver.confine"),
        "{refusal}"
    );
    assert!(refusal.contains("`container` boundary"), "{refusal}");
    assert!(refusal.contains("decision 0046 ruling 5"), "{refusal}");

    // Beside `agent:` the same words, and before the amendment lint that
    // would otherwise call it an amendment of the agent: pinned in
    // `agent_tests.rs`, which owns an agent library.

    // A sequence step is a site too, refused by its own label.
    let mut config = Fixture::config();
    config["seats"]["work"] = json!({
        "results": ["complete"],
        "sequence": [
            {"name": "first", "results": ["done"], "role": "roles/role.md",
             "driver": {"command": ["d"], "confine": {"image": "img"}}},
            {"name": "second", "role": "roles/role.md", "driver": {"command": ["d"]}}
        ],
    });
    let refusal = error(fixture.compile(&config, &Fixture::policy()));
    assert!(
        refusal.contains("seat 'work:first' declares driver.confine"),
        "{refusal}"
    );

    // Every shipped bundle: no site declares the field, and all compile.
    let root = workspace_root();
    let mut dirs = Vec::new();
    for parent in ["recipes", "bundles"] {
        for entry in std::fs::read_dir(root.join(parent)).unwrap() {
            let path = entry.unwrap().path();
            if path.join("bundle.json").is_file() {
                dirs.push(path);
            }
        }
    }
    assert!(dirs.len() >= 5);
    for dir in dirs {
        let text = std::fs::read_to_string(dir.join("bundle.json")).unwrap();
        assert!(
            !text.contains("\"confine\""),
            "{} declares confine",
            dir.display()
        );
        Bundle::compile_with(&dir, &root.join("agents"), &root.join("adapters"))
            .unwrap_or_else(|e| panic!("{} must compile: {e}", dir.display()));
    }
}

/// Decision 0057 ruling 1: a bundle never names a crossing. `publishes`
/// and `consumes` are the realm's two words, and a site that writes
/// either is told where they live — naming the seat, `realms.json` as
/// their home and the ruling — rather than that its key is unknown, on
/// exactly the terms `boundary` gets below and at exactly the same sites.
#[test]
fn a_bundle_never_names_a_crossing() {
    let fixture = Fixture::new();
    let policy = Fixture::policy();
    let home = "a crossing is declared by the realm (realms.json, forge.realms/v5) and never \
                by a bundle, because the file one realm publishes and the digest another \
                pins it at is the realm's fact and not a recipe's (decision 0057 ruling 1)";
    let inline = json!({"role": "roles/role.md", "driver": {"command": ["driver"]}});

    // A seat publishing: the word a realm uses for the file it owns.
    let mut config = Fixture::config();
    config["seats"]["work"]["publishes"] =
        json!([{"name": "orders.api", "path": "contracts/orders.v1.schema.json"}]);
    assert_eq!(
        error(fixture.compile(&config, &policy)),
        format!("bundle: seat 'work' declares publishes; {home}")
    );

    // A seat consuming: the word a realm uses for the pin it carries.
    let mut config = Fixture::config();
    config["seats"]["work"]["consumes"] =
        json!([{"name": "orders.api", "realm": "alpha", "sha256": "0".repeat(64)}]);
    assert_eq!(
        error(fixture.compile(&config, &policy)),
        format!("bundle: seat 'work' declares consumes; {home}")
    );

    // A sequence step, named as its own site — the same walk `boundary`
    // is refused on, so no site can claim a realm's fact by nesting.
    let mut step = inline.clone();
    step["name"] = json!("first");
    step["results"] = json!(["done"]);
    step["consumes"] = json!([]);
    let mut second = inline.clone();
    second["name"] = json!("second");
    let mut config = Fixture::config();
    config["seats"]["work"] = json!({"results": ["complete"], "sequence": [step, second]});
    let refusal = error(fixture.compile(&config, &policy));
    assert!(
        refusal.starts_with("bundle: seat 'work:first' declares consumes; "),
        "{refusal}"
    );
    assert!(refusal.contains(home), "{refusal}");

    // A panel member.
    let mut member = inline.clone();
    member["publishes"] = json!([]);
    let mut config = Fixture::config();
    config["seats"]["work"] = json!({
        "results": ["pass", "fail"],
        "aggregate": "unanimous-pass",
        "panel": {"one": member, "two": inline.clone()},
    });
    let mut panel_policy = Fixture::policy();
    panel_policy["rules"] = json!([
        {"id":"WP", "from":"work", "result":"pass", "next":"review", "reason":"pass"},
        {"id":"WF", "from":"work", "result":"fail", "next":"review", "reason":"fail"},
        {"id":"REVIEW", "from":"review", "result":"clean", "next":"done", "reason":"review"},
    ]);
    let refusal = error(fixture.compile(&config, &panel_policy));
    assert!(
        refusal.starts_with("bundle: seat 'work:one' declares publishes; "),
        "{refusal}"
    );

    // A selected case body.
    let mut case = inline.clone();
    case["consumes"] = json!([]);
    let mut config = Fixture::config();
    config["seats"]["work"] = json!({
        "results": ["complete"],
        "select": {"on": "strategy", "cases": {
            "chore": case,
            "feature": inline.clone(),
            "design": inline.clone(),
            "engine": inline,
        }},
    });
    let refusal = error(fixture.compile(&config, &policy));
    assert!(
        refusal.starts_with("bundle: seat 'work:chore' declares consumes; "),
        "{refusal}"
    );

    // And a bundle that names neither compiles exactly as it always did.
    fixture.compile(&Fixture::config(), &policy).unwrap();
}

/// Decision 0046 ruling 1: a bundle never names the boundary. A `boundary`
/// key at any site — a seat, a panel member, a sequence step, a selected
/// case body — is refused naming the site, the realm map as the field's
/// home and the ruling, and never as an unknown key; inside `hands` it is
/// refused as a misplaced field and not as an unknown `hands` key. The
/// agent-file half is pinned in `agent_tests.rs`, which owns a library.
#[test]
fn a_bundle_never_names_the_boundary() {
    let fixture = Fixture::new();
    let policy = Fixture::policy();
    let home = "the boundary is declared by the realm (realms.json, forge.realms/v4) and never \
                by a bundle or an agent, because the machine a realm runs on is the realm's \
                fact (decision 0046 ruling 1)";
    let inline = json!({"role": "roles/role.md", "driver": {"command": ["driver"]}});

    // A seat, beside its hands.
    let mut config = Fixture::config();
    config["seats"]["work"]["hands"] = json!("workspace");
    config["seats"]["work"]["boundary"] = json!("harness");
    let refusal = error(fixture.compile(&config, &policy));
    assert_eq!(
        refusal,
        format!("bundle: seat 'work' declares boundary; {home}")
    );

    // A panel member: the aggregate's own vocabulary is checked before
    // the members are read, so the table covers `pass` and `fail` too.
    let mut member = inline.clone();
    member["boundary"] = json!("open");
    let mut config = Fixture::config();
    config["seats"]["work"] = json!({
        "results": ["pass", "fail"],
        "aggregate": "unanimous-pass",
        "panel": {"one": member, "two": inline.clone()},
    });
    let mut panel_policy = Fixture::policy();
    panel_policy["rules"] = json!([
        {"id":"WP", "from":"work", "result":"pass", "next":"review", "reason":"pass"},
        {"id":"WF", "from":"work", "result":"fail", "next":"review", "reason":"fail"},
        {"id":"REVIEW", "from":"review", "result":"clean", "next":"done", "reason":"review"},
    ]);
    let refusal = error(fixture.compile(&config, &panel_policy));
    assert!(
        refusal.starts_with("bundle: seat 'work:one' declares boundary; "),
        "{refusal}"
    );
    assert!(refusal.contains(home), "{refusal}");

    // A sequence step.
    let mut step = inline.clone();
    step["name"] = json!("first");
    step["results"] = json!(["done"]);
    step["boundary"] = json!("namespace");
    let mut second = inline.clone();
    second["name"] = json!("second");
    let mut config = Fixture::config();
    config["seats"]["work"] = json!({"results": ["complete"], "sequence": [step, second]});
    let refusal = error(fixture.compile(&config, &policy));
    assert!(
        refusal.starts_with("bundle: seat 'work:first' declares boundary; "),
        "{refusal}"
    );
    assert!(refusal.contains(home), "{refusal}");

    // A selected case body.
    let mut case = inline.clone();
    case["boundary"] = json!("seatbelt");
    let mut config = Fixture::config();
    config["seats"]["work"] = json!({
        "results": ["complete"],
        "select": {"on": "strategy", "cases": {
            "chore": case,
            "feature": inline.clone(),
            "design": inline.clone(),
            "engine": inline.clone(),
        }},
    });
    let refusal = error(fixture.compile(&config, &policy));
    assert!(
        refusal.starts_with("bundle: seat 'work:chore' declares boundary; "),
        "{refusal}"
    );
    assert!(refusal.contains(home), "{refusal}");

    // Inside hands: a misplaced field, never an unknown `hands` key.
    let mut config = Fixture::config();
    config["seats"]["work"]["hands"] = json!({"kind": "workspace", "boundary": "open"});
    let refusal = error(fixture.compile(&config, &policy));
    assert!(
        refusal.starts_with(&format!(
            "bundle: seat 'work' hands: hands names 'boundary'; {home}"
        )),
        "{refusal}"
    );
    assert!(!refusal.contains("unknown key"), "{refusal}");
    // Every refusal above named the realm rather than an unknown key.
    let mut config = Fixture::config();
    config["seats"]["work"]["boundary"] = json!("open");
    assert!(!error(fixture.compile(&config, &policy)).contains("unknown key"));
    // And the pure check reads the key alone: a site without it is untouched.
    refuse_boundary_key("work", &json!({"hands": "workspace"})).unwrap();
}

/// The `docker run` wrapper is gone with the field (decision 0046 ruling
/// 5): the runtime and cli sources name neither the wrapper nor the type
/// outside prose that explains the retirement, and a site that never
/// declared the field composes exactly what it composed — the identity
/// `hands_command` returns for a site without hands.
#[test]
fn the_docker_wrapper_is_gone_from_the_sources() {
    let root = workspace_root();
    let mut sources = Vec::new();
    for crate_dir in ["crates/brokkr-runtime/src", "crates/brokkr-cli/src"] {
        let mut stack = vec![root.join(crate_dir)];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    sources.push(path);
                }
            }
        }
    }
    assert!(sources.len() > 20);
    for path in sources {
        let text = std::fs::read_to_string(&path).unwrap();
        for (line_number, line) in text.lines().enumerate() {
            let prose = line.trim_start().starts_with("//");
            if prose {
                continue;
            }
            // Spelled in halves so this test's own source passes its
            // own scan.
            for banned in [
                concat!("confined_", "command"),
                concat!("\"doc", "ker run"),
                concat!("\"doc", "ker\".to_string()"),
                concat!("--network", "=none"),
                concat!("struct Con", "fine"),
                concat!("Con", "fine {"),
            ] {
                assert!(
                    !line.contains(banned),
                    "{}:{}: {banned} survives the retirement",
                    path.display(),
                    line_number + 1
                );
            }
        }
    }
}

#[test]
fn compiler_accepts_nondefault_positive_limits() {
    let fixture = Fixture::new();
    let mut config = Fixture::config();
    config["seats"]["work"]["limits"] = json!({"max_attempts": 2, "timeout_seconds": 5});
    let bundle = fixture.compile(&config, &Fixture::policy()).unwrap();
    assert_eq!(bundle.seats["work"].limits.max_attempts, 2);
    assert_eq!(bundle.seats["work"].limits.timeout_seconds, 5);
}

#[test]
fn explicit_inputs_suffixes_and_manifest_nonfiles_are_deterministic() {
    assert_eq!(
        brokkr_executable(Err(std::io::Error::other("missing"))),
        "brokkr"
    );
    assert_eq!(
        brokkr_executable(Ok(PathBuf::from("/tmp/brokkr"))),
        "/tmp/brokkr"
    );
    assert!(declarable_input("fixes_applied"));
    assert!(declarable_input("max_residual_severity"));
    assert!(!declarable_input("dirty_worktrees"));
    assert!(!declarable_input("invented"));

    let table = json!({"rules": [
        {"from":"work", "when": {
            "max_residual_severity_gte":"low",
            "max_residual_severity_above":"none",
            "fixes_applied":true,
            "dirty_worktrees":false
        }},
        {"from":"review", "when":{"has_security_residual":false}},
        {"from":"work"}
    ]});
    assert_eq!(
        referenced_seat_inputs(&table, "work"),
        vec!["fixes_applied", "max_residual_severity"]
    );

    let fixture = Fixture::new();
    let mut config = Fixture::config();
    config["seats"]["work"]["inputs"] = json!(["fixes_applied"]);
    let bundle = fixture.compile(&config, &Fixture::policy()).unwrap();
    assert_eq!(bundle.seats["work"].inputs, vec!["fixes_applied"]);

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            fixture.dir.path().join("absent"),
            fixture.dir.path().join("dangling"),
        )
        .unwrap();
        assert!(manifest_for(
            fixture.dir.path(),
            "fixture",
            &[],
            None,
            None,
            &BTreeMap::new(),
            &serde_json::Map::new(),
            Boundary::Namespace,
        )
        .is_ok());
    }
}

/// Proposed decision 0056 ruling 1 (design D2): two structurally
/// different sites of one selected body may not flatten to one address.
///
/// The hazard is not the resume digest — that is derived from structure
/// and cannot alias. It is everything that already keys on the FLAT
/// label: `select_candidates` inserts it into a `BTreeMap`, `argv_for`
/// selects by it, and `site_boundary` and `mark_hands` read
/// `bundle.hands` under it. Step `a:b` with member `c` and step `a` with
/// member `b:c` both spell `a:b:c`, so one site would answer for the
/// other's candidate, hands identity and boundary. An ambiguous bundle
/// fails before spawn, naming both.
#[test]
fn two_sites_that_flatten_to_one_address_are_refused_at_compile_time() {
    // A panel of two, twice: only the FIRST step declares its own
    // vocabulary — the final step is the seat boundary and receives the
    // seat's.
    let step = |name: &str, first: &str, second: &str, results: Option<Value>| {
        let mut step = json!({
            "name": name, "aggregate": "unanimous-pass",
            "panel": {
                first: {"role": "roles/role.md", "driver": {"command": ["driver"]}},
                second: {"role": "roles/role.md", "driver": {"command": ["driver"]}},
            },
        });
        if let Some(results) = results {
            step["results"] = results;
        }
        step
    };
    let seat = |steps: Value| {
        let mut config = Fixture::config();
        config["seats"]["work"] = json!({"results": ["pass", "fail"], "sequence": steps});
        config
    };
    let mut policy = Fixture::policy();
    policy["rules"][0]["result"] = json!("pass");
    policy["rules"].as_array_mut().unwrap().push(json!({
        "id":"WORK-FAIL", "from":"work", "result":"fail", "next":"review",
        "reason":"the panel did not agree"
    }));

    let fixture = Fixture::new();
    let config = seat(json!([
        step("a:b", "c", "other", Some(json!(["pass", "fail"]))),
        step("a", "b:c", "another", None),
    ]));
    let message = error(fixture.compile(&config, &policy));
    assert!(
        message.contains("addresses two different sites as 'a:b:c'"),
        "{message}"
    );
    assert!(message.contains("member 'c' of step 'a:b'"), "{message}");
    assert!(message.contains("member 'b:c' of step 'a'"), "{message}");

    // The ordinary case the check must NOT touch: the same member name
    // under two different steps. `review:alpha` and `design:alpha` are
    // two distinct strings, and nothing about repeated member names,
    // chain progression or the historical meaning of a display tag
    // moves.
    let fixture = Fixture::new();
    let config = seat(json!([
        step("review", "alpha", "beta", Some(json!(["pass", "fail"]))),
        step("design", "alpha", "beta", None),
    ]));
    assert!(fixture.compile(&config, &policy).is_ok());
}

/// Design D10 F2: uniqueness is GLOBAL, not per selected body. A
/// selector's case and a literal phase can flatten to one address across
/// two different seats, and the engine's `select_candidates`, `argv_for`,
/// `mark_hands` and boundary readers key on that one string.
#[test]
fn a_literal_phase_that_aliases_a_selected_case_is_refused_globally() {
    let fixture = Fixture::new();
    let mut policy = Fixture::policy();
    policy["phases"] = json!(["work", "work:chore", "review", "done"]);
    policy["rules"].as_array_mut().unwrap().push(json!({
        "id":"WORKCHORE", "from":"work:chore", "result":"complete", "next":"review", "reason":"x"
    }));
    let mut config = Fixture::config();
    config["seats"]["work"] = json!({
        "results": ["complete"],
        "select": {"on": "strategy", "cases": {
            "chore": {"role": "roles/role.md", "driver": {"command": ["driver"]}},
            "feature": {"role": "roles/role.md", "driver": {"command": ["driver"]}},
            "design": {"role": "roles/role.md", "driver": {"command": ["driver"]}},
            "engine": {"role": "roles/role.md", "driver": {"command": ["driver"]}},
        }},
    });
    config["seats"]["work:chore"] = json!({
        "results": ["complete"], "role": "roles/role.md", "driver": {"command": ["driver"]},
    });
    let message = error(fixture.compile(&config, &policy));
    assert!(
        message.contains("addresses two different sites as 'work:chore'"),
        "{message}"
    );
    // Renaming only the conflicting literal phase compiles: the collision,
    // not the shape, is what is refused.
    let mut control = config.clone();
    control["seats"]["work:other"] = control["seats"]["work:chore"].clone();
    control["seats"]
        .as_object_mut()
        .unwrap()
        .remove("work:chore");
    let mut control_policy = policy.clone();
    control_policy["phases"] = json!(["work", "work:other", "review", "done"]);
    control_policy["rules"][2]["from"] = json!("work:other");
    assert!(fixture.compile(&control, &control_policy).is_ok());
}

/// Design D10 F2: the wrapper creates labels after parsing, so the final
/// global walk must reserve them too. A literal `verify:checks` phase
/// aliases the step the dialect wrapper injects for a wrapped `verify`
/// seat; the raw within-body pass cannot see it.
#[test]
fn a_literal_phase_that_aliases_the_wrapped_verify_step_is_refused() {
    let fixture = Fixture::new();
    let root = workspace_root();
    let dialect = Dialect::load(&root.join("dialects/openspec.json"))
        .unwrap()
        .0;
    let verify = json!({"results":["pass"],"role":"roles/role.md","driver":{"command":["driver"]}});
    let (mut config, mut policy) = dialect_config(verify);
    policy["phases"] = json!(["design", "verify", "verify:checks", "review", "done"]);
    policy["rules"].as_array_mut().unwrap().push(json!({
        "id":"VC", "from":"verify:checks", "result":"pass", "next":"review", "reason":"pass"
    }));
    config["seats"]["verify:checks"] = json!({
        "results":["pass"], "role":"roles/role.md", "driver":{"command":["driver"]}
    });
    let message = error(compile_dialect_fixture(
        &fixture,
        &config,
        &policy,
        Some(&dialect),
    ));
    assert!(
        message.contains("addresses two different sites as 'verify:checks'"),
        "{message}"
    );
}

/// Design D10 F2: a RAW authoring collision that wrapping would DISGUISE.
/// A wrapped verify panel member `alpha` and a literal phase `verify:alpha`
/// both own the authoring address `verify:alpha`; wrapping renames the
/// member to `verify:checks:alpha`, so a final-only walk would miss it and
/// the literal's facts would be served at the member's coordinate. The
/// authoring census refuses it before any fact moves.
#[test]
fn a_raw_phase_that_aliases_a_wrapped_panel_member_is_refused() {
    let fixture = Fixture::new();
    let root = workspace_root();
    let dialect = Dialect::load(&root.join("dialects/openspec.json"))
        .unwrap()
        .0;
    let panel = json!({
        "results":["pass","fail"], "aggregate":"unanimous-pass",
        "panel":{
            "alpha":{"role":"roles/role.md","driver":{"command":["driver"]}},
            "beta":{"role":"roles/role.md","driver":{"command":["driver"]}}
        }
    });
    let (mut config, mut policy) = dialect_config(panel);
    policy["phases"] = json!(["design", "verify", "verify:alpha", "review", "done"]);
    policy["rules"].as_array_mut().unwrap().push(json!({
        "id":"VA", "from":"verify:alpha", "result":"pass", "next":"review", "reason":"pass"
    }));
    config["seats"]["verify:alpha"] = json!({
        "results":["pass"], "role":"roles/role.md", "driver":{"command":["driver"]},
        "hands":"workspace"
    });
    let message = error(compile_dialect_fixture(
        &fixture,
        &config,
        &policy,
        Some(&dialect),
    ));
    assert!(
        message.contains("addresses two different sites as 'verify:alpha'"),
        "{message}"
    );
}

/// Design D10 F2: the injected deterministic validator's address is a
/// wrapper-created coordinate too, so a literal phase that flattens to it
/// is refused before the validator's facts are written there.
#[test]
fn a_literal_phase_that_aliases_the_injected_validator_is_refused() {
    let fixture = Fixture::new();
    let root = workspace_root();
    let dialect = Dialect::load(&root.join("dialects/openspec.json"))
        .unwrap()
        .0;
    let verify = json!({"results":["pass"],"role":"roles/role.md","driver":{"command":["driver"]}});
    let (mut config, mut policy) = dialect_config(verify);
    policy["phases"] = json!([
        "design",
        "verify",
        "verify:dialect-verify",
        "review",
        "done"
    ]);
    policy["rules"].as_array_mut().unwrap().push(json!({
        "id":"VDV", "from":"verify:dialect-verify", "result":"pass", "next":"review", "reason":"pass"
    }));
    config["seats"]["verify:dialect-verify"] = json!({
        "results":["pass"], "role":"roles/role.md", "driver":{"command":["driver"]}
    });
    let message = error(compile_dialect_fixture(
        &fixture,
        &config,
        &policy,
        Some(&dialect),
    ));
    assert!(
        message.contains("addresses two different sites as 'verify:dialect-verify'"),
        "{message}"
    );
}

/// Design D10 F1/F2: relocation moves only the wrapped seat's OWN owners.
/// A distinct literal `verify:bar` phase keeps its own facts instead of
/// being dragged onto `verify:checks:bar` by a prefix sweep — the defect a
/// separate-map relocation would have introduced.
#[test]
fn a_wrapped_verify_panel_leaves_an_unrelated_literal_phase_untouched() {
    let fixture = Fixture::new();
    let root = workspace_root();
    let dialect = Dialect::load(&root.join("dialects/openspec.json"))
        .unwrap()
        .0;
    let panel = json!({
        "results":["pass","fail"], "aggregate":"unanimous-pass",
        "panel":{
            "alpha":{"role":"roles/role.md","driver":{"command":["driver"]}},
            "beta":{"role":"roles/role.md","driver":{"command":["driver"]}}
        }
    });
    let (mut config, mut policy) = dialect_config(panel);
    policy["phases"] = json!(["design", "verify", "verify:bar", "review", "done"]);
    policy["rules"].as_array_mut().unwrap().push(json!({
        "id":"VB", "from":"verify:bar", "result":"pass", "next":"review", "reason":"pass"
    }));
    config["seats"]["verify:bar"] = json!({
        "results":["pass"], "role":"roles/role.md", "driver":{"command":["driver"]},
        "hands":"workspace"
    });
    let bundle = compile_dialect_fixture(&fixture, &config, &policy, Some(&dialect)).unwrap();
    assert!(
        matches!(
            bundle.sites.get("verify:bar").map(|facts| &facts.hands),
            Some(HandsState::Hands(_))
        ),
        "the unrelated literal phase keeps its own hands"
    );
    assert!(
        !bundle.sites.contains_key("verify:checks:bar"),
        "no prefix sweep manufactured a wrapper coordinate for the literal phase"
    );
    assert!(bundle.sites.contains_key("verify:checks:alpha"));
    assert!(!bundle.sites.contains_key("verify:alpha"));
}

/// Design D10 F1: a member named `a` beside one named `checks:a` (the
/// `x`/`checks:x` shape) has the second member's source as the first's
/// destination. Draining every source before inserting any destination
/// keeps both facts with their own owner instead of overwriting one with
/// the other.
#[test]
fn a_wrapped_panel_drains_overlapping_member_addresses_without_overwrite() {
    let fixture = Fixture::new();
    let root = workspace_root();
    let dialect = Dialect::load(&root.join("dialects/openspec.json"))
        .unwrap()
        .0;
    let panel = json!({
        "results":["pass","fail"], "aggregate":"unanimous-pass",
        "panel":{
            "a":{"role":"roles/role.md","driver":{"command":["driver"]},
                 "hands":{"kind":"workspace","network":true}},
            "checks:a":{"role":"roles/role.md","driver":{"command":["driver"]},
                        "hands":"workspace"},
            "x":{"role":"roles/role.md","driver":{"command":["driver"]},
                 "hands":{"kind":"workspace","network":true}},
            "checks:x":{"role":"roles/role.md","driver":{"command":["driver"]},
                        "hands":"workspace"}
        }
    });
    let (config, policy) = dialect_config(panel);
    let bundle = compile_dialect_fixture(&fixture, &config, &policy, Some(&dialect)).unwrap();
    for member in ["a", "x"] {
        assert!(
            matches!(
                bundle
                    .sites
                    .get(&format!("verify:checks:{member}"))
                    .map(|facts| &facts.hands),
                Some(HandsState::Hands(spec)) if spec.network
            ),
            "member `{member}` keeps its own network-enabled hands"
        );
        assert!(
            matches!(
                bundle
                    .sites
                    .get(&format!("verify:checks:checks:{member}"))
                    .map(|facts| &facts.hands),
                Some(HandsState::Hands(spec)) if !spec.network
            ),
            "member `checks:{member}` keeps its own default hands"
        );
        assert!(!bundle.sites.contains_key(&format!("verify:{member}")));
    }
}

/// And every bundle this tree ships walks clean, which is the other half
/// of the same check: a rule that refused a shipped recipe would be a
/// rule nobody could adopt.
#[test]
fn every_shipped_bundle_addresses_its_sites_unambiguously() {
    let root = workspace_root();
    for directory in ["recipes", "bundles"] {
        for entry in std::fs::read_dir(root.join(directory)).unwrap() {
            let path = entry.unwrap().path();
            if !path.join("bundle.json").is_file() {
                continue;
            }
            let bundle = Bundle::compile_with(&path, &root.join("agents"), &root.join("adapters"));
            // A recipe that needs a realm dialect refuses for that
            // reason, not this one; what is asserted here is that NO
            // bundle refuses for an ambiguous address.
            if let Err(error) = bundle {
                assert!(
                    !error.to_string().contains("addresses two different sites"),
                    "{}: {error}",
                    path.display()
                );
            }
        }
    }
}
