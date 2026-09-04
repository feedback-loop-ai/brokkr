//! Agent references at compile time (decision 0016): the fourth
//! alternative beside role+driver, panel and sequence.

use super::*;
use serde_json::json;

fn error<T>(result: Result<T, CompileError>) -> String {
    match result {
        Ok(_) => panic!("expected compilation to fail"),
        Err(error) => error.to_string(),
    }
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

/// A bundle fixture that also owns an `agents/` and an `adapters/` tree,
/// so a compile can run against a library written by the test — which is
/// what makes "adapters are data" (AC-9) an executable claim rather than
/// a promise.
struct AgentFixture {
    dir: tempfile::TempDir,
}

impl AgentFixture {
    fn new() -> AgentFixture {
        let fixture = AgentFixture {
            dir: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(fixture.library().join("charters")).unwrap();
        std::fs::create_dir_all(fixture.adapters()).unwrap();
        std::fs::create_dir_all(fixture.bundle()).unwrap();
        std::fs::write(fixture.library().join("charters/work.md"), "# work\n").unwrap();
        fixture.write(
            "agents/worker.json",
            json!({
                "description": "the worker",
                "charter": "charters/work.md",
                "models": ["opus"],
                "efforts": {"opus": "high"},
                "tools": {"allow": ["cargo"]},
                "limits": {"max_attempts": 3, "timeout_seconds": 77},
            }),
        );
        fixture.write("adapters/claude.json", claude());
        fixture
    }

    fn library(&self) -> PathBuf {
        self.dir.path().join("agents")
    }
    fn adapters(&self) -> PathBuf {
        self.dir.path().join("adapters")
    }
    fn bundle(&self) -> PathBuf {
        self.dir.path().join("bundle")
    }

    fn write(&self, relative: &str, body: Value) {
        std::fs::write(
            self.dir.path().join(relative),
            serde_json::to_vec_pretty(&body).unwrap(),
        )
        .unwrap();
    }

    fn stage(&self, config: &Value, table: &Value) {
        std::fs::write(
            self.bundle().join("bundle.json"),
            serde_json::to_vec(config).unwrap(),
        )
        .unwrap();
        std::fs::write(
            self.bundle().join("policy.json"),
            serde_json::to_vec(table).unwrap(),
        )
        .unwrap();
    }

    fn compile(&self, config: Value) -> Result<Bundle, CompileError> {
        self.stage(&config, &policy());
        Bundle::compile_with(&self.bundle(), &self.library(), &self.adapters())
    }

    /// Two seats: `work` references an agent, `review` inlines.
    fn config(&self) -> Value {
        json!({
            "name": "fixture",
            "policy": "policy.json",
            "seats": {
                "work": {"results": ["complete"], "agent": "worker"},
                "review": {
                    "results": ["clean"],
                    "role": "../agents/charters/work.md",
                    "driver": {"command": ["driver"]},
                },
            },
        })
    }
}

fn claude() -> Value {
    json!({
        "provider": "claude",
        // The fixture grants bindings so the seats here that declare
        // them still compile (decision 0021 ruling 4); the tier stays
        // undeclared, which is untrusted — no seat here is a gate.
        "binding_grant": true,
        "binary": "claude",
        "driver": ["{brokkr}", "driver", "claude", "--"],
        "models": {"opus": "claude-opus-5"},
        "model_flag": "--model",
        "efforts": ["low", "medium", "high"],
        "effort_flag": "--effort",
        "tool_permissions": {
            "flag": "--allowedTools",
            "separator": ",",
            "names": {"cargo": "Bash(cargo:*)"},
        },
        "mcp": "unsupported",
    })
}

/// AC-5's compile half: a seat that names an agent produces the charter
/// as its role, the composed argv as its command, and the agent's 0006
/// bounds as its limits — and the resolution is pinned in the manifest
/// under the invocation site.
#[test]
fn an_agent_reference_resolves_into_an_ordinary_seat_and_pins_itself() {
    let fixture = AgentFixture::new();
    let bundle = fixture.compile(fixture.config()).unwrap();
    let seat = &bundle.seats["work"];
    assert_eq!(seat.limits.max_attempts, 3);
    assert_eq!(seat.limits.timeout_seconds, 77);
    let SeatBody::Single {
        role_path,
        command,
        candidates,
        ..
    } = &seat.body
    else {
        panic!("an agent reference resolves to a single seat");
    };
    assert!(role_path.ends_with(Path::new("charters").join("work.md")));
    assert_eq!(
        &command[1..],
        [
            "driver",
            "claude",
            "--",
            "--model",
            "claude-opus-5",
            "--effort",
            "high",
            "--allowedTools",
            "Bash(cargo:*)"
        ]
    );
    assert!(!command[0].contains('{'), "the legacy token is expanded");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].model, "opus");
    assert_eq!(candidates[0].effort.as_deref(), Some("high"));

    let record = &bundle.manifest["agents"]["work"];
    assert_eq!(record["agent"], "worker");
    assert_eq!(record["model"], "opus");
    assert_eq!(record["provider"], "claude");
    assert_eq!(record["chosen_index"], 0);
    // The manifest key is the pin that replaces the `manifest.files`
    // entry a charter loses by living outside the bundle.
    assert_eq!(record["charter_digest"].as_str().unwrap().len(), 64);
}

/// AC-5, stated as the equality it claims: a resolved seat and the
/// equivalent inline seat produce the same body, element for element.
#[test]
fn a_resolved_seat_equals_the_equivalent_inline_seat() {
    let fixture = AgentFixture::new();
    let resolved = fixture.compile(fixture.config()).unwrap();
    let mut inline = fixture.config();
    inline["seats"]["work"] = json!({
        "results": ["complete"],
        "role": "../agents/charters/work.md",
        "limits": {"max_attempts": 3, "timeout_seconds": 77},
        "driver": {"command": [
            "{brokkr}", "driver", "claude", "--",
            "--model", "claude-opus-5",
            "--effort", "high",
            "--allowedTools", "Bash(cargo:*)",
        ]},
    });
    let inline = fixture.compile(inline).unwrap();

    let describe = |bundle: &Bundle| {
        let seat = &bundle.seats["work"];
        let SeatBody::Single {
            role_path, command, ..
        } = &seat.body
        else {
            unreachable!("single seat")
        };
        (
            role_path.canonicalize().unwrap(),
            command.clone(),
            seat.limits.max_attempts,
            seat.limits.timeout_seconds,
            seat.inputs.clone(),
        )
    };
    assert_eq!(describe(&resolved), describe(&inline));
}

/// AC-21: the agent reference is total. Every key that states what the
/// agent IS is refused beside it, by name. Limits bound this invocation,
/// so the strategy may narrow the roster default without amending the office.
#[test]
fn an_agent_reference_refuses_every_key_that_would_amend_it() {
    let fixture = AgentFixture::new();
    for (key, value) in [
        ("role", json!("../agents/charters/work.md")),
        ("inputs", json!(["fixes_applied"])),
    ] {
        let mut config = fixture.config();
        config["seats"]["work"][key] = value;
        let message = error(fixture.compile(config));
        assert!(
            message.contains(&format!("combines 'agent' with '{key}'")),
            "{message}"
        );
    }
    // `driver.command` states what the agent IS; `driver.confine` is the
    // seat's own trust-class binding, so the refusal names the exact key.
    let mut config = fixture.config();
    config["seats"]["work"]["driver"] = json!({"command": ["driver"]});
    assert!(error(fixture.compile(config)).contains("combines 'agent' with 'driver.command'"));

    let mut config = fixture.config();
    config["seats"]["work"]["driver"] = json!("not an object");
    assert!(error(fixture.compile(config)).contains("driver must be an object"));
}

#[test]
fn a_seat_limit_narrows_the_agent_default() {
    let fixture = AgentFixture::new();
    let mut config = fixture.config();
    config["seats"]["work"]["limits"] = json!({"max_attempts": 1, "timeout_seconds": 19});
    let bundle = fixture.compile(config).unwrap();
    assert_eq!(bundle.seats["work"].limits.max_attempts, 1);
    assert_eq!(bundle.seats["work"].limits.timeout_seconds, 19);
}

#[test]
fn a_seat_is_exactly_one_of_role_agent_panel_or_sequence() {
    let fixture = AgentFixture::new();
    let mut config = fixture.config();
    config["seats"]["work"]["panel"] = json!({});
    assert!(error(fixture.compile(config))
        .contains("exactly one of role+driver, agent, panel, sequence, or select"));

    let mut config = fixture.config();
    config["seats"]["work"]["agent"] = json!("");
    assert!(error(fixture.compile(config)).contains("agent must be a non-empty string"));

    let mut config = fixture.config();
    config["seats"]["work"]["agent"] = json!("nobody");
    assert!(error(fixture.compile(config)).contains("is not in the library"));
}

/// A seat may still declare the bindings it owns beside `agent:`.
#[test]
fn a_resolved_seat_keeps_the_bindings_the_seat_itself_provides() {
    let fixture = AgentFixture::new();
    let mut config = fixture.config();
    config["seats"]["work"]["secrets"] = json!(["TOKEN"]);
    let bundle = fixture.compile(config).unwrap();
    assert_eq!(bundle.seats["work"].secrets, vec!["TOKEN".to_string()]);
    assert_eq!(bundle.seats["work"].results, vec!["complete".to_string()]);
}

/// Panel members and sequence steps may each name an agent, and the
/// resolution is recorded under the invocation site the engine already
/// uses — `seat:member`, `seat:step`, `seat:step:member`.
#[test]
fn panel_members_and_sequence_steps_may_name_agents() {
    let fixture = AgentFixture::new();
    fixture.write(
        "agents/member.json",
        json!({
            "description": "a member",
            "charter": "charters/work.md",
            "models": ["opus"],
            "efforts": {"opus": "high"},
            "tools": {"allow": ["cargo"]},
        }),
    );
    let mut config = fixture.config();
    config["seats"]["work"] = json!({
        "results": ["complete"],
        "limits": {"max_attempts": 2, "timeout_seconds": 5},
        "sequence": [
            {"name": "first", "aggregate": "unanimous-pass", "panel": {
                "a": {"agent": "member"},
                "b": {"role": "../agents/charters/work.md",
                      "driver": {"command": ["driver"]}},
            }},
            {"name": "second", "agent": "member"},
        ],
    });
    let bundle = fixture.compile(config).unwrap();
    let records = bundle.manifest["agents"].as_object().unwrap();
    assert_eq!(
        records.keys().cloned().collect::<Vec<_>>(),
        vec!["work:first:a".to_string(), "work:second".to_string()]
    );
    // The inline member keeps an EMPTY candidate list, which is what
    // keeps the execute path unchanged for inline seats.
    let SeatBody::Sequence { steps } = &bundle.seats["work"].body else {
        unreachable!("sequence")
    };
    let StepBody::Single { candidates, .. } = &steps[1].body else {
        unreachable!("single step")
    };
    assert_eq!(candidates.len(), 1);
    let StepBody::Panel { members, .. } = &steps[0].body else {
        unreachable!("panel step")
    };
    assert!(!members[0].candidates.is_empty());
    assert!(members[1].candidates.is_empty());
}

/// A member or step has no 0006 bounds and no 0007 declaration of its
/// own, so an agent carrying either cannot be referenced there: the
/// declaration could only be discarded silently.
#[test]
fn an_agent_with_limits_or_inputs_cannot_be_referenced_from_a_step() {
    let fixture = AgentFixture::new();
    fixture.write(
        "agents/inputful.json",
        json!({
            "description": "declares inputs",
            "charter": "charters/work.md",
            "models": ["opus"],
            "efforts": {"opus": "high"},
            "tools": {"allow": ["cargo"]},
            "inputs": ["fixes_applied"],
        }),
    );
    for (agent, key) in [("worker", "limits"), ("inputful", "inputs")] {
        let mut config = fixture.config();
        config["seats"]["work"] = json!({
            "results": ["complete"],
            "sequence": [
            {"name": "first", "results": ["complete"], "agent": agent},
                {"name": "second", "role": "../agents/charters/work.md",
                 "driver": {"command": ["driver"]}},
            ],
        });
        let message = error(fixture.compile(config));
        assert!(
            message.contains(&format!("which declares '{key}'")),
            "{message}"
        );
    }
    let mut config = fixture.config();
    config["seats"]["work"] = json!({
        "results": ["complete"],
        "sequence": [
            {"name": "first", "results": ["complete"], "agent": "worker", "panel": {}},
            {"name": "second", "role": "../agents/charters/work.md",
             "driver": {"command": ["driver"]}},
        ],
    });
    assert!(error(fixture.compile(config))
        .contains("exactly one of role+driver, agent, panel, or dialect"));

    let mut config = fixture.config();
    config["seats"]["work"] = json!({
        "results": ["complete"],
        "sequence": [
            {"name": "first", "results": ["complete"], "agent": "worker", "role": "x"},
            {"name": "second", "role": "../agents/charters/work.md",
             "driver": {"command": ["driver"]}},
        ],
    });
    assert!(error(fixture.compile(config)).contains("combines 'agent' with 'role'"));
}

/// AC-9: a NEW provider and a NEW model arrive as a file. There is no
/// Rust edit in this test's diff — the executable form of "adding a
/// provider must not require a release".
#[test]
fn a_brand_new_provider_and_model_arrive_as_data() {
    let fixture = AgentFixture::new();
    fixture.write(
        "adapters/invented.json",
        json!({
            "provider": "invented",
            "binary": "invented-cli",
            "driver": ["invented-cli", "run"],
            "models": {"newmodel": "invented/new-1"},
            "model_flag": "-m",
            "efforts": ["low", "medium", "high"],
            "effort_flag": "--effort",
            "tool_permissions": {
                "flag": "--tools",
                "separator": " ",
                "names": {"cargo": "cargo-everything"},
            },
            "mcp": "unsupported",
        }),
    );
    fixture.write(
        "agents/worker.json",
        json!({
            "description": "the worker",
            "charter": "charters/work.md",
            "models": ["newmodel"],
            "efforts": {"newmodel": "medium"},
            "tools": {"allow": ["cargo"]},
        }),
    );
    let bundle = fixture.compile(fixture.config()).unwrap();
    let SeatBody::Single { command, .. } = &bundle.seats["work"].body else {
        unreachable!("single seat")
    };
    assert_eq!(
        command,
        &vec![
            "invented-cli".to_string(),
            "run".to_string(),
            "-m".to_string(),
            "invented/new-1".to_string(),
            "--effort".to_string(),
            "medium".to_string(),
            "--tools".to_string(),
            "cargo-everything".to_string(),
        ]
    );
    assert_eq!(bundle.manifest["agents"]["work"]["provider"], "invented");
}

/// AC-22: resolution happens BEFORE every existing lint, so an
/// agent-resolved seat faces each of them exactly as an inline seat does.
#[test]
fn a_resolved_seat_faces_every_existing_lint() {
    let fixture = AgentFixture::new();

    // results-covered-by-a-rule.
    let mut config = fixture.config();
    config["seats"]["work"]["results"] = json!(["invented"]);
    assert!(error(fixture.compile(config)).contains("no rule covers it"));

    // Protected-phase reachability.
    let mut config = fixture.config();
    config["protected_phase"] = json!("absent");
    assert!(error(fixture.compile(config)).contains("policy has no 'absent' phase"));

    // 0007 provenance: the agent's declaration is the seat's, and it is
    // checked against the phase's rule-referenced inputs.
    fixture.write(
        "agents/underdeclared.json",
        json!({
            "description": "declares too little",
            "charter": "charters/work.md",
            "models": ["opus"],
            "efforts": {"opus": "high"},
            "tools": {"allow": ["cargo"]},
            "inputs": ["fixes_applied"],
            "limits": {"max_attempts": 1},
        }),
    );
    let mut config = fixture.config();
    config["seats"]["review"] = json!({"results": ["clean"], "agent": "underdeclared"});
    let table = json!({
        "phases": ["work", "review", "done"],
        "initial": "work",
        "terminal": ["done"],
        "rules": [
            {"id":"WORK", "from":"work", "result":"complete", "next":"review", "reason":"w"},
            {"id":"REVIEW", "from":"review", "result":"clean",
             "when": {"has_security_residual": false}, "next":"done", "reason":"r"},
        ],
    });
    fixture.stage(&config, &table);
    let message = error(Bundle::compile_with(
        &fixture.bundle(),
        &fixture.library(),
        &fixture.adapters(),
    ));
    assert!(
        message.contains("rules reference input 'has_security_residual'"),
        "{message}"
    );
}

/// AC-11: the composed argv faces the SAME undeclared-secret lint an
/// inline command faces — an adapter's driver template is not a
/// privileged place to smuggle a reference from.
#[test]
fn an_adapter_template_secret_reference_faces_the_declared_secret_lint() {
    let fixture = AgentFixture::new();
    let mut adapter = claude();
    adapter["driver"] = json!([
        "{brokkr}",
        "driver",
        "claude",
        "--",
        "--key",
        "{{secret:TOKEN}}"
    ]);
    fixture.write("adapters/claude.json", adapter);
    let message = error(fixture.compile(fixture.config()));
    assert!(message.contains("undeclared secret 'TOKEN'"), "{message}");

    let mut config = fixture.config();
    config["seats"]["work"]["secrets"] = json!(["TOKEN"]);
    let bundle = fixture.compile(config).unwrap();
    let SeatBody::Single { command, .. } = &bundle.seats["work"].body else {
        unreachable!("single seat")
    };
    assert!(command.iter().any(|part| part.contains("secret:TOKEN")));
}

/// A bundle that references no agent never opens the library, so a
/// missing one is a non-event — which is why every existing recipe
/// compiles with no `agents/` directory in sight.
#[test]
fn a_bundle_without_an_agent_reference_never_opens_the_library() {
    let fixture = AgentFixture::new();
    let mut config = fixture.config();
    config["seats"]["work"] = json!({
        "results": ["complete"],
        "role": "../agents/charters/work.md",
        "driver": {"command": ["driver"]},
    });
    fixture.stage(&config, &policy());
    let bundle = Bundle::compile_with(
        &fixture.bundle(),
        Path::new("/nonexistent-library"),
        Path::new("/nonexistent-adapters"),
    )
    .unwrap();
    assert!(bundle.manifest.get("agents").is_none());
}

#[test]
fn a_missing_library_is_named_when_a_seat_needs_one() {
    let fixture = AgentFixture::new();
    fixture.stage(&fixture.config(), &policy());
    let message = error(Bundle::compile_with(
        &fixture.bundle(),
        Path::new("/nonexistent-library"),
        &fixture.adapters(),
    ));
    assert!(message.contains("nonexistent-library"), "{message}");
    let message = error(Bundle::compile_with(
        &fixture.bundle(),
        &fixture.library(),
        Path::new("/nonexistent-adapters"),
    ));
    assert!(message.contains("nonexistent-adapters"), "{message}");
}

/// The default roots are `agents` and `adapters`, resolved against the
/// working directory exactly as `--recipes-dir` is.
#[test]
fn compile_delegates_to_the_default_library_roots() {
    assert_eq!(DEFAULT_AGENTS_DIR, "agents");
    assert_eq!(DEFAULT_ADAPTERS_DIR, "adapters");
    let fixture = AgentFixture::new();
    let mut config = fixture.config();
    config["seats"]["work"] = json!({
        "results": ["complete"],
        "role": "../agents/charters/work.md",
        "driver": {"command": ["driver"]},
    });
    fixture.stage(&config, &policy());
    assert!(Bundle::compile(&fixture.bundle()).is_ok());
}

#[test]
fn mentions_agent_walks_the_whole_config() {
    assert!(mentions_agent(&json!({"seats": {"work": {"agent": "x"}}})));
    assert!(mentions_agent(&json!({"sequence": [{"agent": "x"}]})));
    assert!(!mentions_agent(&json!({"seats": {"work": {"role": "r"}}})));
    assert!(!mentions_agent(&json!("agent")));
}

/// `driver.confine` is the one `driver` key legal beside `agent:`: it is
/// the seat's own trust-class binding, not a statement about what the
/// agent is, and `brokkr agents show` never claims to show it.
#[test]
fn a_resolved_seat_may_still_declare_its_own_confinement() {
    let fixture = AgentFixture::new();
    let mut config = fixture.config();
    config["seats"]["work"]["driver"] = json!({"confine": {"image": "img", "network": true}});
    let bundle = fixture.compile(config).unwrap();
    let SeatBody::Single {
        confine,
        candidates,
        ..
    } = &bundle.seats["work"].body
    else {
        unreachable!("single seat")
    };
    let confine = confine.as_ref().expect("the seat's own confinement");
    assert_eq!(confine.image, "img");
    assert!(confine.network);
    assert_eq!(candidates.len(), 1, "the agent still resolved");
}

/// A refusal inside a panel member propagates out of the panel, and out
/// of the sequence step the panel is: one bad member is a bad bundle,
/// never a member quietly dropped.
#[test]
fn a_refusal_inside_a_panel_member_propagates_out_of_its_step() {
    let fixture = AgentFixture::new();
    let mut config = fixture.config();
    config["seats"]["work"] = json!({
        "results": ["complete"],
        "sequence": [
            {"name": "first", "aggregate": "unanimous-pass", "panel": {
                "a": {"agent": "nobody"},
                "b": {"role": "../agents/charters/work.md",
                      "driver": {"command": ["driver"]}},
            }},
            {"name": "second", "role": "../agents/charters/work.md",
             "driver": {"command": ["driver"]}},
        ],
    });
    assert!(error(fixture.compile(config)).contains("is not in the library"));
}
