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

#[test]
fn an_agent_driver_object_may_not_amend_even_one_field() {
    assert!(refuse_amendments("work", &json!({"driver": {}})).is_ok());
    let refusal = error(refuse_amendments(
        "work",
        &json!({"driver": {"command": []}}),
    ));
    assert!(refusal.contains("driver.command"), "{refusal}");
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
///
/// `root` is the temporary directory canonicalised ONCE at creation
/// (decision 0063; SC8's canonical-root scenario): every write and every
/// expected path is derived from it, so a diagnostic that names a path
/// compares equal on a host whose temp root is reached through an alias.
struct AgentFixture {
    dir: tempfile::TempDir,
    root: PathBuf,
}

impl AgentFixture {
    fn new() -> AgentFixture {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let fixture = AgentFixture { dir, root };
        std::fs::create_dir_all(fixture.library().join("charters")).unwrap();
        std::fs::create_dir_all(fixture.adapters()).unwrap();
        std::fs::create_dir_all(fixture.bundle().join("roles")).unwrap();
        std::fs::write(fixture.library().join("charters/work.md"), "# work\n").unwrap();
        // An inline seat's role stands inside its own bundle, where the
        // file map pins it (decision 0066 ruling 5). Here it is a link to
        // the agent's charter — pinned by content under the link's own
        // name — so an inline seat can be the same seat an agent resolves to.
        std::os::unix::fs::symlink(
            "../../agents/charters/work.md",
            fixture.bundle().join("roles/work.md"),
        )
        .unwrap();
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
        self.root.join("agents")
    }
    fn adapters(&self) -> PathBuf {
        self.root.join("adapters")
    }
    fn bundle(&self) -> PathBuf {
        self.root.join("bundle")
    }

    fn write(&self, relative: &str, body: Value) {
        std::fs::write(
            self.root.join(relative),
            serde_json::to_vec_pretty(&body).unwrap(),
        )
        .unwrap();
    }

    /// The same compile under a stated boundary (decision 0046 ruling 1).
    fn compile_under(&self, config: Value, boundary: Boundary) -> Result<Bundle, CompileError> {
        self.stage(&config, &policy());
        Bundle::compile_under(&self.bundle(), &self.library(), &self.adapters(), boundary)
    }

    /// The same compile with a policy of the test's own.
    fn compile_with_policy(&self, config: Value, table: &Value) -> Result<Bundle, CompileError> {
        self.stage(&config, table);
        Bundle::compile_with(&self.bundle(), &self.library(), &self.adapters())
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
                    "role": "roles/work.md",
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
        // Claude Code is KNOWN to carry web search and web fetch, so a
        // seat is compiled on it only where its adapter says how each is
        // denied (decision 0066 ruling 1): an adapter written before the
        // ruling refuses, and this fixture is not one.
        "native_capabilities": claude_native(),
    })
}

/// The two native powers of Claude Code as `adapters/claude.json` declares
/// them: switched through the harness's own tool lists.
fn claude_native() -> Value {
    let power = |capability: &str, tool: &str| {
        json!({
            "capability": capability, "tools": [tool],
            "on": {"selection": {"include": [tool], "allow": [tool], "deny": []}},
            "off": {"selection": {"include": [], "allow": [], "deny": [tool]}},
            "restrictions": {"unsupported": "no native restriction transport is established"},
            "evidence": {"source": "adapter data", "scope": "declared", "limitations": []},
            "authored": {"list_flags": ["--tools", "--allowedTools", "--allowed-tools"]},
        })
    };
    json!({
        "known": {"web-search": power("web-search", "WebSearch"),
                  "web-fetch": power("web-fetch", "WebFetch")},
        "selection": {
            "include": {"flag": "--tools", "separator": ","},
            "allow": {"flag": "--allowedTools", "separator": ","},
            "deny": {"flag": "--disallowedTools", "separator": ","},
        },
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
        "role": "roles/work.md",
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
        ("role", json!("roles/work.md")),
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
                "b": {"role": "roles/work.md",
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
                {"name": "second", "role": "roles/work.md",
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
            {"name": "second", "role": "roles/work.md",
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
            {"name": "second", "role": "roles/work.md",
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
        "--append-system-prompt",
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
        "role": "roles/work.md",
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
        "role": "roles/work.md",
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

/// `driver.confine` was the one `driver` key legal beside `agent:` until
/// decision 0046 ruling 5 retired it into the `container` boundary; a
/// resolved seat that still writes it is refused by the same words an
/// inline one is, and never read as an amendment of the agent.
#[test]
fn a_resolved_seat_may_no_longer_declare_its_own_confinement() {
    let fixture = AgentFixture::new();
    let mut config = fixture.config();
    config["seats"]["work"]["driver"] = json!({"confine": {"image": "img", "network": true}});
    let refusal = fixture.compile(config).unwrap_err().to_string();
    assert!(
        refusal.contains("seat 'work' declares driver.confine"),
        "{refusal}"
    );
    assert!(refusal.contains("`container` boundary"), "{refusal}");
    assert!(refusal.contains("decision 0046 ruling 5"), "{refusal}");
    assert!(
        !refusal.contains("an agent reference is total"),
        "{refusal}"
    );

    // Any other driver key beside `agent:` is still the amendment it was.
    let mut config = fixture.config();
    config["seats"]["work"]["driver"] = json!({"command": ["x"]});
    let refusal = fixture.compile(config).unwrap_err().to_string();
    assert!(
        refusal.contains("combines 'agent' with 'driver.command'"),
        "{refusal}"
    );
}

/// Decision 0046 ruling 1, on the agent side: an agent file whose `hands`
/// object carries `boundary` is refused when the library loads, naming
/// the agent and the realm as the field's home; and a seat that writes
/// `boundary` beside `agent:` is refused by the same words, before the
/// amendment lint would call it an amendment of the agent.
#[test]
fn an_agent_file_never_names_the_boundary_either() {
    let fixture = AgentFixture::new();
    let home = "the boundary is declared by the realm (realms.json, forge.realms/v4) and never \
                by a bundle or an agent, because the machine a realm runs on is the realm's \
                fact (decision 0046 ruling 1)";
    fixture.write(
        "agents/worker.json",
        json!({
            "description": "the worker",
            "charter": "charters/work.md",
            "models": ["opus"],
            "efforts": {"opus": "high"},
            "hands": {"kind": "workspace", "network": false, "boundary": "harness"},
        }),
    );
    let refusal = error(fixture.compile(fixture.config()));
    assert!(refusal.contains("agent 'worker'"), "{refusal}");
    assert!(
        refusal.contains(&format!("'hands': hands names 'boundary'; {home}")),
        "{refusal}"
    );
    assert!(!refusal.contains("unknown key"), "{refusal}");

    let fixture = AgentFixture::new();
    let mut config = fixture.config();
    config["seats"]["work"]["boundary"] = json!("open");
    let refusal = error(fixture.compile(config));
    assert_eq!(
        refusal,
        format!("bundle: seat 'work' declares boundary; {home}")
    );
    assert!(
        !refusal.contains("an agent reference is total"),
        "{refusal}"
    );
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
                "b": {"role": "roles/work.md",
                      "driver": {"command": ["driver"]}},
            }},
            {"name": "second", "role": "roles/work.md",
             "driver": {"command": ["driver"]}},
        ],
    });
    assert!(error(fixture.compile(config)).contains("is not in the library"));
}

/// An agent no seat names, asking for `asks`.
fn unseated(asks: Value) -> Value {
    json!({"description": "an office no seat of this bundle names",
           "charter": "charters/work.md", "models": ["opus"],
           "efforts": {"opus": "high"}, "capabilities": asks})
}

/// The operator's definition of `name`, beside the fixture's library.
fn define(fixture: &AgentFixture, name: &str, classes: Value) {
    std::fs::create_dir_all(fixture.dir.path().join("capabilities")).unwrap();
    fixture.write(
        &format!("capabilities/{name}.json"),
        json!({"name": name, "classes": classes}),
    );
}

/// Finding M3: a compile that LOADS a library lints every agent in it, not
/// only the ones a seat names (CQ2; decision 0065 ruling 1). The capability
/// walk resolves seated references, so a valid seated worker used to hide
/// an unseated researcher asking for a capability nobody defined — and the
/// CLI readouts were the only callers of the definition lint. The refusal
/// is the lint's own sentence, for a want as much as a requirement, and it
/// reaches through a composed recipe exactly as it reaches a plain one.
#[test]
fn an_unseated_loaded_agent_with_an_undefined_request_refuses_the_compile() {
    for strength in ["requires", "wants"] {
        let fixture = AgentFixture::new();
        define(&fixture, "web-search", json!(["reads", "egress"]));
        // The control: the same library compiles while every loaded
        // agent's request is defined — the unseated one's included.
        fixture.write(
            "agents/researcher.json",
            unseated(json!({"web-search": strength})),
        );
        fixture
            .compile(fixture.config())
            .unwrap_or_else(|error| panic!("{strength}: {error}"));
        fixture.write(
            "agents/researcher.json",
            unseated(json!({"web-search": strength, "library-docs": strength})),
        );
        let expected = "bundle: agent 'researcher': capability 'library-docs' has no abstract \
                        definition at 'capabilities/library-docs.json' in the operator \
                        configuration; declare its classes before requesting it";
        // What the compile said, or that it compiled: a compile that lints
        // seated agents only fails HERE.
        let said = |compiled: Result<Bundle, CompileError>| match compiled {
            Ok(bundle) => format!("compiled '{}'", bundle.name),
            Err(refusal) => refusal.to_string(),
        };
        assert_eq!(
            said(fixture.compile(fixture.config())),
            expected,
            "{strength}"
        );
        // Composed: the seat that opens the library is an ancestor's, and
        // the refusal is the same sentence inside the note every composed
        // compile failure carries.
        let derived = fixture.dir.path().join("derived");
        std::fs::create_dir_all(&derived).unwrap();
        std::fs::write(
            derived.join("bundle.json"),
            serde_json::to_vec(&json!({"name": "derived", "extends": "bundle"})).unwrap(),
        )
        .unwrap();
        assert_eq!(
            said(Bundle::compile_with(
                &derived,
                &fixture.library(),
                &fixture.adapters()
            )),
            format!("bundle: {expected} (composed: derived -> fixture)"),
            "composed, {strength}"
        );
    }
    // A bundle that names no agent never opens the library, so a broken
    // agent in it is not this compile's to refuse.
    let fixture = AgentFixture::new();
    fixture.write(
        "agents/researcher.json",
        unseated(json!({"library-docs": "requires"})),
    );
    let mut config = fixture.config();
    config["seats"]["work"] = config["seats"]["review"].clone();
    config["seats"]["work"]["results"] = json!(["complete"]);
    fixture.compile(config).unwrap();
}

/// Finding M3's identity half (design D3): the lint CONSULTS the
/// definition an unseated agent names, so that definition is pinned — an
/// edit to its bytes moves the bundle's identity — while a definition no
/// loaded agent and no grant names stays outside it.
#[test]
fn a_definition_only_an_unseated_agent_names_is_pinned_and_an_unconsulted_one_is_not() {
    let fixture = AgentFixture::new();
    define(&fixture, "web-search", json!(["reads", "egress"]));
    define(&fixture, "unasked", json!(["reads"]));
    fixture.write(
        "agents/researcher.json",
        unseated(json!({"web-search": "wants"})),
    );
    let compiled = || fixture.compile(fixture.config()).unwrap();
    assert_eq!(
        compiled().manifest["capabilities"]["definitions"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["web-search"]
    );
    let before = compiled().manifest_digest();
    assert_eq!(
        before,
        compiled().manifest_digest(),
        "identical inputs, identical identity"
    );
    // The unconsulted definition's bytes are not identity.
    define(&fixture, "unasked", json!(["reads", "egress"]));
    assert_eq!(before, compiled().manifest_digest());
    // The consulted one's are: the same classes, whitespace alone.
    std::fs::write(
        fixture.dir.path().join("capabilities/web-search.json"),
        "{\"name\": \"web-search\", \"classes\": [\"reads\", \"egress\"]}\n\n",
    )
    .unwrap();
    assert_ne!(before, compiled().manifest_digest());
}

// ------------------------------------------- typed local declarations (D5)

use crate::agents::{Library, LocalTools, Sandbox};

fn local(allow: Option<&[&str]>, sandbox: Option<Sandbox>) -> LocalTools {
    LocalTools {
        allow: allow.map(|names| names.iter().map(|name| name.to_string()).collect()),
        sandbox,
    }
}

/// The shipped codex workspace, gate and work fragments as this fixture
/// declares them: each expresses exactly one `--sandbox` class.
const CODEX_WORKSPACE: [&str; 4] = [
    "--sandbox",
    "read-only",
    "-c",
    "mcp_servers.brokkr.args={hands_args_toml}",
];
const CODEX_GATE: [&str; 4] = [
    "--sandbox",
    "read-only",
    "--output-last-message",
    "{result_path}",
];
const CODEX_WORK: [&str; 2] = ["--sandbox", "workspace-write"];

/// A fixture codex: trusted, judging `astra`, no per-tool flag, hands in
/// every shape, and the measured web-search OFF the known-power floor
/// requires (decision 0066 ruling 1).
fn codex() -> Value {
    json!({
        "provider": "codex",
        "binary": "codex",
        "driver": ["{brokkr}", "driver", "codex", "--"],
        "models": {"astra": "gpt-6-astra"},
        "model_flag": "--model",
        "efforts": ["low", "medium", "high"],
        "effort_flag": "--effort",
        "tool_permissions": "unsupported",
        "mcp": "unsupported",
        "trust_tier": "trusted",
        "judges": ["astra"],
        "hands": {
            "workspace": CODEX_WORKSPACE,
            "harness": {"gate": CODEX_GATE, "work": CODEX_WORK, "result": "last-message"},
        },
        "native_capabilities": {"known": {"web-search": {
            "capability": "web-search", "tools": ["web_search"],
            "on": {"default": "measured cold default"},
            "off": {"argv": ["-c", "web_search=\"disabled\""]},
            "restrictions": {"unsupported": "no native restriction transport is established"},
            "evidence": {"source": "adapter data", "scope": "declared", "limitations": []}}}},
    })
}

/// A boxed agent on `models`, every link at high effort, with the given
/// typed `tools`.
fn boxed_agent(models: &[&str], tools: Value) -> Value {
    let efforts: Map<String, Value> = models
        .iter()
        .map(|model| (model.to_string(), json!("high")))
        .collect();
    json!({
        "description": "a boxed agent",
        "charter": "charters/work.md",
        "models": models,
        "efforts": efforts,
        "hands": "workspace",
        "tools": tools,
    })
}

/// An office on claude declaring `allow: ["cargo", "git"]`, with the
/// fixture claude mapping both names.
fn write_office(fixture: &AgentFixture) {
    let mut claude = claude();
    claude["tool_permissions"]["names"]["git"] = json!("Bash(git:*)");
    fixture.write("adapters/claude.json", claude);
    fixture.write(
        "agents/office.json",
        json!({
            "description": "the office",
            "charter": "charters/work.md",
            "models": ["opus"],
            "efforts": {"opus": "high"},
            "tools": {"allow": ["cargo", "git"]},
        }),
    );
}

fn command_of(bundle: &Bundle, seat: &str) -> Vec<String> {
    match &bundle.seats[seat].body {
        SeatBody::Single { command, .. } => command.clone(),
        _ => panic!("seat '{seat}' is a single site"),
    }
}

/// The complete narrowing refusal at a site, as the compiler prints it.
fn widening(site: &str, agent: &str, field: &str, cause: &str) -> String {
    format!(
        "bundle: seat '{site}': the site's 'tools.{field}' {cause}; an agent-backed site only \
         narrows the restrictions of agent '{agent}' (decision 0065 slice one, design D5)"
    )
}

/// The complete explicit-empty composition refusal at a site.
fn empty_refusal(site: &str, agent: &str, provider: &str, model: &str) -> String {
    format!(
        "bundle: seat '{site}': agent '{agent}' cannot be served by provider '{provider}' on \
         model '{model}': the effective 'tools.allow' is explicitly empty, and no serving path \
         yet expresses an empty local allow set as a delivered restriction (joining no names \
         into an empty flag value proves nothing); the declaration is kept exactly and refused \
         rather than run unrestricted, until decision 0065 slice one's lowering proves its \
         delivery (design D5.3). A capability the provider cannot express fails compilation \
         here rather than degrading silently at run time"
    )
}

/// The complete inline-representation refusal at a site.
fn inline_refusal(site: &str, field: &str) -> String {
    format!(
        "bundle: seat '{site}' declares 'tools.{field}' on a site whose command no office \
         composes; the engine does not yet lower a typed local {field} into an authored \
         command, so the restriction would be recorded and not delivered — it is kept exactly \
         and refused rather than run unrestricted, until decision 0065 slice one's lowering and \
         origin transport prove its delivery (design D5.3); an authored flag cannot stand in for \
         it"
    )
}

/// The complete container refusal at a site.
fn container_refusal(site: &str) -> String {
    format!(
        "bundle: seat '{site}' declares 'tools' beside a panel, sequence or select; a local \
         declaration belongs to the site that executes — the member, step or case body — and \
         a container cannot own one, even an empty object, because it would either become a \
         shared grant or be ignored (decision 0065 slice one, design D5)"
    )
}

/// A policy whose `work` phase covers a panel's pass/fail vocabulary.
fn panel_policy() -> Value {
    json!({
        "phases": ["work", "review", "done"],
        "initial": "work",
        "terminal": ["done"],
        "rules": [
            {"id":"W-PASS", "from":"work", "result":"pass", "next":"review", "reason":"pass"},
            {"id":"W-FAIL", "from":"work", "result":"fail", "next":"review", "reason":"fail"},
            {"id":"REVIEW", "from":"review", "result":"clean", "next":"done", "reason":"review"},
        ],
    })
}

/// SCM "Field omission inherits while an explicit empty list subtracts",
/// "A local override cannot widen its agent" and "Malformed tools cannot
/// become defaults", at an ordinary agent-backed seat: the effective value
/// is recorded beside the site's facts, the composed command carries
/// exactly the narrowed mapping, the office's record and library entry
/// are untouched, and every widening, empty, classed or malformed
/// declaration refuses with the site and its complete cause.
#[test]
fn an_agent_backed_seat_narrows_its_office_per_field_and_records_the_effective_value() {
    let fixture = AgentFixture::new();
    write_office(&fixture);
    let compiled = |tools: Option<Value>| {
        let mut config = fixture.config();
        config["seats"]["work"]["agent"] = json!("office");
        if let Some(tools) = tools {
            config["seats"]["work"]["tools"] = tools;
        }
        fixture.compile(config)
    };
    // Omission and `{}` inherit the office whole.
    for tools in [None, Some(json!({})), Some(json!({"mcp": []}))] {
        let bundle = compiled(tools.clone()).unwrap();
        assert_eq!(
            bundle.sites["work"].local,
            Some(local(Some(&["cargo", "git"]), None)),
            "{tools:?}"
        );
        assert_eq!(
            command_of(&bundle, "work")[8..],
            ["--allowedTools", "Bash(cargo:*),Bash(git:*)"]
        );
    }
    // A subset keeps its written order and composes exactly itself.
    let bundle = compiled(Some(json!({"allow": ["git"]}))).unwrap();
    assert_eq!(
        bundle.sites["work"].local,
        Some(local(Some(&["git"]), None))
    );
    assert_eq!(
        command_of(&bundle, "work")[8..],
        ["--allowedTools", "Bash(git:*)"]
    );
    // The inline review seat was visited and declares nothing; the office's
    // record and library entry are the office's own.
    assert_eq!(
        bundle.sites["review"].local,
        Some(LocalTools::unspecified())
    );
    let office = Library::load(&fixture.library()).unwrap();
    let office = office.agent("office").unwrap();
    assert_eq!(
        office.allow,
        Some(vec!["cargo".to_string(), "git".to_string()])
    );
    assert_eq!(
        bundle.sites["work"].record.as_ref().unwrap()["agent_digest"],
        json!(office.digest)
    );
    // Each refusal names the site and the complete cause.
    assert_eq!(
        error(compiled(Some(json!({"allow": ["git", "make"]})))),
        widening(
            "work",
            "office",
            "allow",
            "names 'make', which the office's 'tools.allow' [\"cargo\", \"git\"] does not; a \
             site subtracts from its office and never adds to it"
        )
    );
    assert_eq!(
        error(compiled(Some(json!({"allow": []})))),
        empty_refusal("work", "office", "claude", "opus")
    );
    assert_eq!(
        error(compiled(Some(json!({"sandbox": "read-only"})))),
        "bundle: seat 'work' requests 'tools.sandbox' 'read-only' without hands; no engine \
         path expresses a sandbox class for a site without hands, so the class would be \
         recorded and not delivered — refused under the `namespace` boundary until decision \
         0065 slice one's lowering proves it (design D5.3)"
    );
    assert_eq!(
        error(compiled(Some(json!({"allow": null})))),
        "bundle: seat 'work' 'tools' needs 'allow' as an array of strings"
    );
    assert_eq!(
        error(compiled(Some(json!(null)))),
        "bundle: seat 'work' 'tools' must be a JSON object"
    );
    assert_eq!(
        error(compiled(Some(
            json!({"allow": ["git"], "sandbox": "loose"})
        ))),
        "bundle: seat 'work' 'tools.sandbox' is 'loose', which is not one of read-only, \
         workspace-write, danger-full-access"
    );
    assert_eq!(
        error(compiled(Some(json!({"allow": ["git", "git"]})))),
        "bundle: seat 'work' 'tools.allow' names 'git' twice; a local allow list is \
         duplicate-free"
    );
}

/// SCM "Omitted and empty local permissions are distinct" and "Decoding
/// cannot admit a runnable unrestricted command" at an inline site: a
/// visited site records a checked empty declaration, and every nonempty
/// field is decoded exactly and then refused with the site, field and
/// unsupported-representation cause — shape judged before representation.
#[test]
fn an_inline_site_records_a_checked_empty_declaration_and_refuses_each_nonempty_field() {
    let fixture = AgentFixture::new();
    let compiled = |tools: Option<Value>| {
        let mut config = fixture.config();
        if let Some(tools) = tools {
            config["seats"]["review"]["tools"] = tools;
        }
        fixture.compile(config)
    };
    for tools in [None, Some(json!({})), Some(json!({"mcp": []}))] {
        let bundle = compiled(tools.clone()).unwrap();
        assert_eq!(
            bundle.sites["review"].local,
            Some(LocalTools::unspecified()),
            "{tools:?}"
        );
    }
    assert_eq!(
        error(compiled(Some(json!({"allow": []})))),
        inline_refusal("review", "allow")
    );
    assert_eq!(
        error(compiled(Some(json!({"allow": ["cargo"]})))),
        inline_refusal("review", "allow")
    );
    assert_eq!(
        error(compiled(Some(json!({"sandbox": "read-only"})))),
        inline_refusal("review", "sandbox")
    );
    assert_eq!(
        error(compiled(Some(json!({"sandbox": "danger-full-access"})))),
        inline_refusal("review", "sandbox")
    );
    assert_eq!(
        error(compiled(Some(json!({"allow": ["Bash(cargo:*)"]})))),
        "bundle: seat 'review' 'tools.allow' names 'Bash(cargo:*)', which does not match \
         ^[a-z][a-z0-9-]*$"
    );
    assert_eq!(
        error(compiled(Some(json!({"invented": 1})))),
        "bundle: seat 'review' 'tools' has unknown key 'invented'; known keys: allow, sandbox, \
         mcp"
    );
}

/// A typed declaration opens the adapters through the existing fallible
/// context (design D5.2): a bundle that declares one and has no adapter
/// data refuses with the loader's own words, while the same bundle without
/// the declaration never looks for the directory.
#[test]
fn a_typed_declaration_needs_the_adapter_context_and_a_missing_one_is_named() {
    let fixture = AgentFixture::new();
    std::fs::remove_dir_all(fixture.adapters()).unwrap();
    let mut config = fixture.config();
    config["seats"]["work"] = json!({
        "results": ["complete"],
        "role": "roles/work.md",
        "driver": {"command": ["driver"]},
    });
    assert!(fixture.compile(config.clone()).is_ok());
    config["seats"]["review"]["tools"] = json!({});
    assert_eq!(
        error(fixture.compile(config)),
        format!(
            "bundle: adapters {}: No such file or directory (os error 2); the adapter data is \
             where a driver's model mapping (decision 0016) and its trust tier and binding \
             grant (decision 0021) are declared, and this bundle names an agent, seats a gate, \
             declares a secret binding or declares typed tools",
            fixture.adapters().display()
        )
    );
}

/// SCM "Malformed tools cannot become defaults", last clause, from a
/// bundle's own bytes: a `tools`, `allow` or `sandbox` key written twice
/// refuses from the strict layer reader — equal copies included — without
/// any change to the reader itself.
#[test]
fn repeated_tools_keys_in_a_bundle_refuse_from_the_original_source() {
    let fixture = AgentFixture::new();
    let review =
        r#""review":{"results":["clean"],"role":"roles/work.md","driver":{"command":["driver"]}}"#;
    let cases = [
        (
            format!(
                r#"{{"name":"fixture","policy":"policy.json","seats":{{"work":{{"results":["complete"],"agent":"worker","tools":{{"allow":["cargo"],"allow":["cargo"]}}}},{review}}}}}"#
            ),
            "allow",
            r#"["cargo"]"#,
        ),
        (
            format!(
                r#"{{"name":"fixture","policy":"policy.json","seats":{{"work":{{"results":["complete"],"agent":"worker","tools":{{"sandbox":"read-only","sandbox":"read-only"}}}},{review}}}}}"#
            ),
            "sandbox",
            r#""read-only""#,
        ),
        (
            format!(
                r#"{{"name":"fixture","policy":"policy.json","seats":{{"work":{{"results":["complete"],"agent":"worker","tools":{{}},"tools":{{}}}},{review}}}}}"#
            ),
            "tools",
            r#"{}"#,
        ),
    ];
    for (text, key, value) in cases {
        fixture.stage(&json!({}), &policy());
        std::fs::write(fixture.bundle().join("bundle.json"), &text).unwrap();
        // The parser stands one past the token that closed the SECOND copy
        // of the value.
        let first = text.find(value).unwrap();
        let second = first + 1 + text[first + 1..].find(value).unwrap();
        let column = second + value.len() + 1;
        assert_eq!(
            error(Bundle::compile_with(
                &fixture.bundle(),
                &fixture.library(),
                &fixture.adapters()
            )),
            format!(
                "bundle: {}: key '{key}' is written twice at line 1 column {column}",
                fixture.bundle().join("bundle.json").display()
            )
        );
    }
}

/// SCM "Each executable body owns its local declaration", the container
/// half: `tools` beside a panel, sequence or select — at a seat, a
/// selected body or a sequence step — refuses its placement, even as `{}`.
#[test]
fn tools_beside_a_container_refuse_at_every_container_form() {
    let fixture = AgentFixture::new();
    let member = json!({"role": "roles/work.md", "driver": {"command": ["driver"]}});
    let panel = |tools: Value| {
        json!({"results": ["pass", "fail"], "aggregate": "unanimous-pass",
               "panel": {"a": member, "b": member}, "tools": tools})
    };
    let mut config = fixture.config();
    config["seats"]["work"] = panel(json!({}));
    assert_eq!(
        error(fixture.compile_with_policy(config, &panel_policy())),
        container_refusal("work")
    );
    let mut config = fixture.config();
    config["seats"]["work"] = json!({
        "results": ["complete"], "tools": {"allow": ["cargo"]},
        "sequence": [
            {"name": "first", "results": ["complete"], "role": "roles/work.md",
             "driver": {"command": ["driver"]}},
            {"name": "second", "role": "roles/work.md", "driver": {"command": ["driver"]}},
        ],
    });
    assert_eq!(error(fixture.compile(config)), container_refusal("work"));
    let mut config = fixture.config();
    config["seats"]["work"] = json!({
        "results": ["complete"], "tools": {},
        "select": {"on": "strategy", "cases": {}, "default": member},
    });
    assert_eq!(error(fixture.compile(config)), container_refusal("work"));
    // A selected body that is itself a panel.
    let mut config = fixture.config();
    config["seats"]["work"] = json!({
        "results": ["pass", "fail"],
        "select": {"on": "strategy", "cases": {},
                   "default": {"aggregate": "unanimous-pass",
                               "panel": {"a": member, "b": member}, "tools": {}}},
    });
    assert_eq!(
        error(fixture.compile_with_policy(config, &panel_policy())),
        container_refusal("work:default")
    );
    // A sequence step that is a panel.
    let mut config = fixture.config();
    config["seats"]["work"] = json!({
        "results": ["complete"],
        "sequence": [
            {"name": "first", "aggregate": "unanimous-pass",
             "panel": {"a": member, "b": member}, "tools": {}},
            {"name": "second", "role": "roles/work.md", "driver": {"command": ["driver"]}},
        ],
    });
    assert_eq!(
        error(fixture.compile(config)),
        container_refusal("work:first")
    );
}

/// SCM "Each executable body owns its local declaration", the executable
/// half: the same subset, explicit empty and widening at an ordinary seat,
/// a panel member, a sequence step, a selected case, a selected default and
/// an inherited body each yield the same effective value or the complete
/// owning-site refusal.
#[test]
fn every_executable_form_owns_its_local_declaration() {
    let fixture = AgentFixture::new();
    write_office(&fixture);
    let inline = json!({"role": "roles/work.md", "driver": {"command": ["driver"]}});
    // (label, config builder, policy)
    type Form<'a> = (&'a str, Box<dyn Fn(Value) -> Value + 'a>, Value);
    let forms: Vec<Form<'_>> = vec![
        (
            "work",
            Box::new(|tools| json!({"results": ["complete"], "agent": "office", "tools": tools})),
            policy(),
        ),
        (
            "work:a",
            Box::new(|tools| {
                json!({"results": ["pass", "fail"], "aggregate": "unanimous-pass",
                       "panel": {"a": {"agent": "office", "tools": tools},
                                 "b": inline.clone()}})
            }),
            panel_policy(),
        ),
        (
            "work:first",
            Box::new(|tools| {
                json!({"results": ["complete"], "sequence": [
                    {"name": "first", "results": ["complete"], "agent": "office",
                     "tools": tools},
                    {"name": "second", "role": "roles/work.md",
                     "driver": {"command": ["driver"]}}]})
            }),
            policy(),
        ),
        (
            "work:engine",
            Box::new(|tools| {
                json!({"results": ["complete"], "select": {"on": "strategy",
                    "cases": {"engine": {"agent": "office", "tools": tools}},
                    "default": inline.clone()}})
            }),
            policy(),
        ),
        (
            "work:default",
            Box::new(|tools| {
                json!({"results": ["complete"], "select": {"on": "strategy", "cases": {},
                    "default": {"agent": "office", "tools": tools}}})
            }),
            policy(),
        ),
    ];
    for (label, seat, table) in &forms {
        let compiled = |tools: Value| {
            let mut config = fixture.config();
            config["seats"]["work"] = seat(tools);
            fixture.compile_with_policy(config, table)
        };
        let bundle = compiled(json!({"allow": ["git"]})).unwrap();
        assert_eq!(
            bundle.sites[*label].local,
            Some(local(Some(&["git"]), None)),
            "{label}"
        );
        assert_eq!(
            bundle.sites[*label].chain[0].argv[8..],
            ["--allowedTools", "Bash(git:*)"],
            "{label}"
        );
        assert_eq!(
            error(compiled(json!({"allow": []}))),
            empty_refusal(label, "office", "claude", "opus"),
            "{label}"
        );
        assert_eq!(
            error(compiled(json!({"allow": ["make"]}))),
            widening(
                label,
                "office",
                "allow",
                "names 'make', which the office's 'tools.allow' [\"cargo\", \"git\"] does not; \
                 a site subtracts from its office and never adds to it"
            ),
            "{label}"
        );
    }
    // An inherited body: the base layer declares the narrowing and the leaf
    // declares nothing, so the effective value comes from the layer that
    // wrote it.
    let base = fixture.root.join("base");
    std::fs::create_dir_all(base.join("roles")).unwrap();
    std::fs::copy(
        fixture.bundle().join("roles/work.md"),
        base.join("roles/work.md"),
    )
    .unwrap();
    std::fs::write(
        base.join("policy.json"),
        serde_json::to_vec(&policy()).unwrap(),
    )
    .unwrap();
    let mut base_config = fixture.config();
    base_config["name"] = json!("base");
    base_config["seats"]["work"] = json!({"results": ["complete"], "agent": "office",
                                         "tools": {"allow": ["git"]}});
    std::fs::write(
        base.join("bundle.json"),
        serde_json::to_vec(&base_config).unwrap(),
    )
    .unwrap();
    std::fs::write(
        fixture.bundle().join("bundle.json"),
        serde_json::to_vec(&json!({"name": "fixture", "extends": "base"})).unwrap(),
    )
    .unwrap();
    let inherited =
        Bundle::compile_with(&fixture.bundle(), &fixture.library(), &fixture.adapters()).unwrap();
    assert_eq!(
        inherited.sites["work"].local,
        Some(local(Some(&["git"]), None))
    );
    assert_eq!(
        inherited.sites["review"].local,
        Some(LocalTools::unspecified())
    );
}

/// SCM "Site-local narrowing cannot contaminate a shared office": two
/// members hiring one office with different subsets each keep exactly their
/// own effective fields, chain and identity, in either order of declaration,
/// and the office's record and digest are the same under both.
#[test]
fn two_sites_sharing_one_office_keep_their_own_effective_fields_in_either_order() {
    let fixture = AgentFixture::new();
    write_office(&fixture);
    let digest = Library::load(&fixture.library())
        .unwrap()
        .agent("office")
        .unwrap()
        .digest
        .clone();
    for (first, second) in [(["git"], ["cargo"]), (["cargo"], ["git"])] {
        let mut config = fixture.config();
        config["seats"]["work"] = json!({
            "results": ["pass", "fail"], "aggregate": "unanimous-pass",
            "panel": {"a": {"agent": "office", "tools": {"allow": first}},
                      "b": {"agent": "office", "tools": {"allow": second}}},
        });
        let bundle = fixture
            .compile_with_policy(config, &panel_policy())
            .unwrap();
        for (label, names) in [("work:a", first), ("work:b", second)] {
            let facts = &bundle.sites[label];
            assert_eq!(facts.local, Some(local(Some(&names), None)), "{label}");
            assert_eq!(
                facts.chain[0].argv[8..],
                [
                    "--allowedTools".to_string(),
                    format!("Bash({}:*)", names[0])
                ],
                "{label}"
            );
            assert_eq!(
                facts.record.as_ref().unwrap()["agent_digest"],
                json!(digest)
            );
            assert_eq!(facts.record.as_ref().unwrap()["agent"], json!("office"));
        }
        // The container itself was never visited by the local pass.
        assert!(bundle
            .sites
            .get("work")
            .is_none_or(|facts| facts.local.is_none()));
    }
}

/// The D5.3 admission table, row by row, under empty realm grants. Only
/// actual codex dispatch with hands and an engine fragment that expresses
/// exactly the requested class admits; every other shape refuses with the
/// site, link, class, boundary and complete cause, and every standing
/// refusal keeps its precedence. Admitted fixtures keep their holdings,
/// native OFF, hands and boundary unchanged.
#[test]
fn a_typed_sandbox_admits_only_where_an_existing_codex_fragment_expresses_it_exactly() {
    let fixture = AgentFixture::new();
    fixture.write("adapters/codex.json", codex());
    let seat = |class: Option<&str>| {
        let mut config = fixture.config();
        config["seats"]["work"] = json!({"results": ["complete"], "agent": "boxed"});
        if let Some(class) = class {
            config["seats"]["work"]["class"] = json!(class);
        }
        config
    };
    let declare = |sandbox: &str| {
        fixture.write(
            "agents/boxed.json",
            boxed_agent(&["astra"], json!({"allow": ["cargo"], "sandbox": sandbox})),
        );
    };
    let mismatch = |class: &str, part: &str, boundary: &str, found: &str| {
        format!(
            "bundle: seat 'work' link 1 requests 'tools.sandbox' '{class}', but the `{part}` \
             fragment the engine selects for provider 'codex' under the `{boundary}` boundary \
             expresses '{found}'; a fragment is neither called narrower nor clamped, the typed \
             class must match it exactly — refused (design D5.3)"
        )
    };

    // Row: boxed, read-only matches `hands.workspace`.
    declare("read-only");
    let boxed = fixture.compile(seat(None)).unwrap();
    assert_eq!(
        boxed.sites["work"].local,
        Some(local(Some(&["cargo"]), Some(Sandbox::ReadOnly)))
    );
    assert_eq!(boxed.sites["work"].chain[0].hands_fragment, CODEX_WORKSPACE);
    assert_eq!(boxed.hands["work"], HandsSpec::default());
    assert_eq!(boxed.boundary, Boundary::Namespace);
    let outcome = &boxed.sites["work"].capabilities.as_ref().unwrap().outcomes[0];
    assert!(outcome.held.is_empty());
    // The one known power is not held, and its OFF is planned.
    assert_eq!(outcome.not_held.keys().collect::<Vec<_>>(), ["web-search"]);
    assert_eq!(outcome.manifest()["native"]["inventory"], json!("known"));
    assert_eq!(outcome.manifest()["native"]["on"], json!([]));
    assert_eq!(outcome.manifest()["native"]["off"], json!(["web-search"]));
    for class in ["workspace-write", "danger-full-access"] {
        declare(class);
        assert_eq!(
            error(fixture.compile(seat(None))),
            mismatch(class, "hands.workspace", "namespace", "read-only")
        );
    }

    // Row: harness gate, read-only matches `hands.harness.gate`.
    declare("read-only");
    let gate = fixture
        .compile_under(seat(Some("gate")), Boundary::Harness)
        .unwrap();
    assert_eq!(
        gate.sites["work"].local,
        Some(local(Some(&["cargo"]), Some(Sandbox::ReadOnly)))
    );
    assert_eq!(
        gate.sites["work"].chain[0].harness.gate.as_deref(),
        Some(CODEX_GATE.map(String::from).as_slice())
    );
    assert!(gate.sites["work"].chain[0].hands_fragment.is_empty());
    for class in ["workspace-write", "danger-full-access"] {
        declare(class);
        assert_eq!(
            error(fixture.compile_under(seat(Some("gate")), Boundary::Harness)),
            mismatch(class, "hands.harness.gate", "harness", "read-only")
        );
    }

    // Row: harness work, workspace-write matches `hands.harness.work`.
    declare("workspace-write");
    let work = fixture
        .compile_under(seat(None), Boundary::Harness)
        .unwrap();
    assert_eq!(
        work.sites["work"].local,
        Some(local(Some(&["cargo"]), Some(Sandbox::WorkspaceWrite)))
    );
    assert_eq!(
        work.sites["work"].chain[0].harness.work.as_deref(),
        Some(CODEX_WORK.map(String::from).as_slice())
    );
    for class in ["read-only", "danger-full-access"] {
        declare(class);
        assert_eq!(
            error(fixture.compile_under(seat(None), Boundary::Harness)),
            mismatch(class, "hands.harness.work", "harness", "workspace-write")
        );
    }

    // Row: open work has no fragment; an open gate keeps its standing refusal.
    declare("read-only");
    assert_eq!(
        error(fixture.compile_under(seat(None), Boundary::Open)),
        "bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only' under the `open` \
         boundary, where a work seat runs at the harness's own default and no engine fragment \
         expresses a class; a presumed provider default is not a representation — refused \
         (design D5.3)"
    );
    assert_eq!(
        error(fixture.compile_under(seat(Some("gate")), Boundary::Open)),
        "bundle: seat 'work' is a gate with hands under the `open` boundary, where nothing at \
         all stands between a model's hands and the machine; `open` never holds a model gate \
         (decision 0046 ruling 4)"
    );

    // A missing gate fragment keeps its standing refusal, before any class.
    let mut gateless = codex();
    gateless["hands"]["harness"] = json!({"work": CODEX_WORK, "result": "last-message"});
    fixture.write("adapters/codex.json", gateless);
    assert_eq!(
        error(fixture.compile_under(seat(Some("gate")), Boundary::Harness)),
        "bundle: seat 'work' gate link 1 resolves to provider 'codex', which declares no \
         `hands.harness.gate` fragment; under the `harness` boundary a model may judge only \
         under its harness's own read-only sandbox as the adapter addresses it (decision 0046 \
         ruling 4)"
    );

    // A fragment naming no class, and one the grammar cannot read.
    let mut classless = codex();
    classless["hands"]["workspace"] = json!(["-c", "mcp_servers.brokkr.args={hands_args_toml}"]);
    fixture.write("adapters/codex.json", classless);
    assert_eq!(
        error(fixture.compile(seat(None))),
        "bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only', but the \
         `hands.workspace` fragment the engine selects for provider 'codex' under the \
         `namespace` boundary names no `--sandbox` class at all; a fragment that expresses \
         nothing is not a representation — refused (design D5.3)"
    );
    let mut unreadable = codex();
    unreadable["hands"]["workspace"] = json!(["--sandbox", "read-only", "--bogus"]);
    fixture.write("adapters/codex.json", unreadable);
    assert_eq!(
        error(fixture.compile(seat(None))),
        "bundle: seat 'work' link 1 requests a typed 'tools.sandbox', but the `hands.workspace` \
         fragment it would be judged against cannot be read: the 'codex' command grammar cannot \
         place argument 3 ('--bogus'): it names no option. A harness brokkr launches is parsed \
         against a model of its options, and a token that grammar cannot place is refused \
         rather than passed through, because a control nobody can read is a control nobody can \
         rule on (decision 0066 ruling 6)"
    );
    // The same control through the configuration door.
    let mut configured = codex();
    configured["hands"]["workspace"] =
        json!(["--sandbox", "read-only", "-c", "sandbox_mode=\"read-only\""]);
    fixture.write("adapters/codex.json", configured);
    assert_eq!(
        error(fixture.compile(seat(None))),
        "bundle: seat 'work' link 1 requests a typed 'tools.sandbox', but the `hands.workspace` \
         fragment assigns 'sandbox_mode' through the harness's configuration, a second door to \
         the same control that no typed class can be checked against — refused (design D5.3)"
    );

    // A competing authored control beside a matching fragment.
    let mut authored = codex();
    authored["driver"] = json!([
        "{brokkr}",
        "driver",
        "codex",
        "--",
        "--sandbox",
        "danger-full-access"
    ]);
    fixture.write("adapters/codex.json", authored);
    assert_eq!(
        error(fixture.compile(seat(None))),
        "bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only', but the authored \
         command of provider 'codex' already carries `--sandbox` 'danger-full-access', a \
         competing control the selected `hands.workspace` fragment would stand beside; authored \
         bytes cannot supply or contest a typed representation — refused under the `namespace` \
         boundary (design D5.3)"
    );

    // A provider labelled codex that dispatches another harness.
    let mut shim = codex();
    shim["driver"] = json!(["{brokkr}", "driver", "codex-shim", "--"]);
    fixture.write("adapters/codex.json", shim);
    assert_eq!(
        error(fixture.compile(seat(None))),
        "bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only' but dispatches the \
         'codex-shim' harness through provider 'codex'; only the codex harness's own `--sandbox` \
         fragments express a sandbox class today, and a provider label, a permission mode or an \
         unmodelled driver is not evidence of one — refused under the `namespace` boundary \
         (design D5.3)"
    );
    fixture.write("adapters/codex.json", codex());

    // Claude with hands: a permission mode is not a sandbox class.
    let mut claude = claude();
    claude["hands"] = json!({"workspace": ["--tools", "", "--mcp-config", "{hands_mcp_json}"]});
    fixture.write("adapters/claude.json", claude);
    fixture.write(
        "agents/boxed.json",
        boxed_agent(&["opus"], json!({"sandbox": "read-only"})),
    );
    assert_eq!(
        error(fixture.compile(seat(None))),
        "bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only' but dispatches the \
         'claude' harness through provider 'claude'; only the codex harness's own `--sandbox` \
         fragments express a sandbox class today, and a provider label, a permission mode or an \
         unmodelled driver is not evidence of one — refused under the `namespace` boundary \
         (design D5.3)"
    );

    // A later candidate cannot hide behind an admitted primary.
    fixture.write(
        "agents/boxed.json",
        boxed_agent(&["astra", "opus"], json!({"sandbox": "read-only"})),
    );
    assert_eq!(
        error(fixture.compile(seat(None))),
        "bundle: seat 'work' link 2 requests 'tools.sandbox' 'read-only' but dispatches the \
         'claude' harness through provider 'claude'; only the codex harness's own `--sandbox` \
         fragments express a sandbox class today, and a provider label, a permission mode or an \
         unmodelled driver is not evidence of one — refused under the `namespace` boundary \
         (design D5.3)"
    );

    // Without a class, the same chain compiles: the declaration alone
    // changed nothing else.
    fixture.write(
        "agents/boxed.json",
        boxed_agent(&["astra", "opus"], json!({})),
    );
    let plain = fixture.compile(seat(None)).unwrap();
    assert_eq!(plain.sites["work"].local, Some(LocalTools::unspecified()));
    assert_eq!(plain.sites["work"].chain.len(), 2);
}

/// A seat may narrow a boxed office's class and it is admitted or refused
/// on the effective value: a workspace-write office at a harness work site
/// admits, its seat narrowed to read-only refuses on the narrowed class,
/// and a seat that would widen refuses at narrowing, before any fragment.
#[test]
fn a_seat_narrows_a_boxed_office_and_admission_judges_the_effective_class() {
    let fixture = AgentFixture::new();
    fixture.write("adapters/codex.json", codex());
    fixture.write(
        "agents/boxed.json",
        boxed_agent(&["astra"], json!({"sandbox": "workspace-write"})),
    );
    let seat = |tools: Value| {
        let mut config = fixture.config();
        config["seats"]["work"] = json!({"results": ["complete"], "agent": "boxed",
                                         "tools": tools});
        config
    };
    let admitted = fixture
        .compile_under(seat(json!({})), Boundary::Harness)
        .unwrap();
    assert_eq!(
        admitted.sites["work"].local,
        Some(local(None, Some(Sandbox::WorkspaceWrite)))
    );
    assert_eq!(
        error(fixture.compile_under(seat(json!({"sandbox": "read-only"})), Boundary::Harness)),
        "bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only', but the \
         `hands.harness.work` fragment the engine selects for provider 'codex' under the \
         `harness` boundary expresses 'workspace-write'; a fragment is neither called narrower \
         nor clamped, the typed class must match it exactly — refused (design D5.3)"
    );
    assert_eq!(
        error(fixture.compile_under(
            seat(json!({"sandbox": "danger-full-access"})),
            Boundary::Harness
        )),
        widening(
            "work",
            "boxed",
            "sandbox",
            "requests 'danger-full-access', which reaches wider than the office's \
             'workspace-write'; the classes reach read-only < workspace-write < \
             danger-full-access, and a site narrows its office rather than being clamped to it"
        )
    );
    // The office's class is not the seat's boundary: under a box the
    // narrowed read-only matches the workspace fragment.
    let boxed = fixture
        .compile(seat(json!({"sandbox": "read-only"})))
        .unwrap();
    assert_eq!(
        boxed.sites["work"].local,
        Some(local(None, Some(Sandbox::ReadOnly)))
    );
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// A dialect step is an exec-generated check: it owns a checked empty
/// declaration, refuses a nonempty local field it cannot represent, and
/// where the dialect supplies no check at all there is no site to own the
/// declaration, so its presence refuses.
#[test]
fn a_dialect_step_owns_only_a_checked_empty_declaration() {
    let fixture = AgentFixture::new();
    let root = workspace_root();
    let path = root.join("dialects/openspec.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let unsupported = text.replacen(
        r#""check": {"argv": ["openspec", "validate", "{change}", "--strict", "--no-interactive"]}"#,
        r#""check": {"unsupported": "a fixture without a clarify check"}"#,
        1,
    );
    assert_ne!(unsupported, text, "the fixture rewrites the clarify check");
    let (mut dialect, _) = Dialect::parse(&path.display().to_string(), &unsupported).unwrap();
    dialect.render(path.parent().unwrap()).unwrap();
    let policy = json!({
        "phases": ["design", "clarify", "review", "done"], "initial": "design",
        "terminal": ["done"],
        "rules": [
            {"id":"D", "from":"design", "result":"drafted", "next":"clarify", "reason":"drafted"},
            {"id":"DF", "from":"design", "result":"fail", "next":"design", "reason":"retry"},
            {"id":"C", "from":"clarify", "result":"clear", "next":"review", "reason":"clear"},
            {"id":"CA", "from":"clarify", "result":"ambiguous", "next":"clarify", "reason":"again"},
            {"id":"R", "from":"review", "result":"clean", "next":"done", "reason":"clean"},
        ],
    });
    let author = |result: &str| {
        json!({"name": "author", "results": [result], "role": "roles/work.md",
               "driver": {"command": ["driver"]}})
    };
    let compile = |design_tools: Option<Value>, clarify_tools: Option<Value>| {
        let mut validate = json!({"name": "validate", "dialect": "validate"});
        if let Some(tools) = design_tools {
            validate["tools"] = tools;
        }
        let mut check = json!({"name": "check", "dialect": "check"});
        if let Some(tools) = clarify_tools {
            check["tools"] = tools;
        }
        let config = json!({
            "name": "dialect-fixture", "policy": "policy.json", "protected_phase": "review",
            "seats": {
                "design": {"results": ["drafted", "fail"],
                           "sequence": [author("drafted"), validate]},
                "clarify": {"results": ["clear", "ambiguous"],
                            "sequence": [author("clear"), check]},
                "review": {"results": ["clean"], "role": "roles/work.md",
                           "driver": {"command": ["driver"]}},
            },
        });
        fixture.stage(&config, &policy);
        Bundle::compile_with_realm(
            &fixture.bundle(),
            &root.join("agents"),
            &root.join("adapters"),
            None,
            Some(&dialect),
            Boundary::Namespace,
        )
    };
    let bundle = compile(Some(json!({})), None).unwrap();
    assert_eq!(
        bundle.sites["design:validate"].local,
        Some(LocalTools::unspecified())
    );
    assert_eq!(
        error(compile(Some(json!({"allow": []})), None)),
        inline_refusal("design:validate", "allow")
    );
    assert_eq!(
        error(compile(Some(json!({"sandbox": "read-only"})), None)),
        inline_refusal("design:validate", "sandbox")
    );
    assert_eq!(
        error(compile(None, Some(json!({})))),
        "bundle: sequence step 'clarify:check' declares 'tools' on a dialect step whose 'check' \
         the dialect does not supply; no executable site exists to own the declaration, so it \
         could only be discarded — refused (decision 0065 slice one, design D5)"
    );
    // The control: without the declaration the unsupported check compiles
    // to no site at all.
    let bundle = compile(None, None).unwrap();
    assert!(!bundle.sites.contains_key("clarify:check"));
}
