use super::*;
use serde_json::json;

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
        assert!(parse_panel(dir, "review", &raw, Some(&results), &[], &mut None).is_err());
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
        Some(&[]),
        &[],
        &mut None
    ))
    .contains("does not declare"));
    assert_eq!(
        parse_panel(dir, "review", &panel, Some(&results), &[], &mut None)
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
    ] {
        assert!(parse_sequence(dir, "review", &raw, &results, &[], &mut None).is_err());
    }
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
        &json!({"driver":{"command":["{forge}", "./tool", "plain"]}}),
        &[],
    )
    .unwrap();
    assert_eq!(command.len(), 3);
    assert!(command[1].ends_with("tool"));
    assert_eq!(command[2], "plain");

    for raw in [
        json!({"driver":{"confine":"bad"}}),
        json!({"driver":{"confine":{}}}),
        json!({"driver":{"confine":{"image":"img", "invented":true}}}),
    ] {
        assert!(parse_confine("work", &raw).is_err());
    }
    let confine = parse_confine(
        "work",
        &json!({"driver":{"confine":{"image":"img", "network":true, "mounts":["/x", 2]}}}),
    )
    .unwrap()
    .unwrap();
    assert!(confine.network);
    assert_eq!(confine.mounts, vec!["/x"]);

    assert!(referenced_seat_inputs(&json!({}), "work").is_empty());
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
        forge_executable(Err(std::io::Error::other("missing"))),
        "forge"
    );
    assert_eq!(
        forge_executable(Ok(PathBuf::from("/tmp/forge"))),
        "/tmp/forge"
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
        assert!(manifest_for(fixture.dir.path(), "fixture", None).is_ok());
    }
}
