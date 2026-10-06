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

/// A compile's outcome as one string, whichever way it went, so a table
/// row that unexpectedly COMPILES is reported beside its expected refusal
/// rather than aborting the table at that row.
fn outcome(result: Result<Bundle, CompileError>) -> String {
    match result {
        Ok(bundle) => format!(
            "compiled: {:?}",
            bundle
                .sites
                .iter()
                .map(|(label, facts)| (label.clone(), facts.local.clone()))
                .collect::<Vec<_>>()
        ),
        Err(error) => error.to_string(),
    }
}

/// One table row: a label, what was observed and what was expected.
type Row<T> = (String, T, T);

/// Every row of a table reaches its own exact assertion: the rows are all
/// computed first, then every mismatch is reported together, so a mutation
/// that touches several rows names each of them, and a first failing row
/// hides no later one (SC8; tasks 2.1.4 to 2.1.6).
#[track_caller]
fn each_row<T: PartialEq + std::fmt::Debug>(rows: Vec<Row<T>>) {
    let failures: Vec<String> = rows
        .iter()
        .filter(|(_, observed, expected)| observed != expected)
        .map(|(label, observed, expected)| {
            format!("row {label}:\n  left:  {observed:?}\n  right: {expected:?}")
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
        // file map pins it (decision 0066 ruling 5; operator ruling 3 of
        // 2026-09-23: a consumed input that resolves outside the tree is
        // refused, not pinned). It is the SAME bytes as the agent's
        // charter — pinned by content under its own name — so an inline
        // seat can be the same seat an agent resolves to, without a link
        // that leaves the bundle (design D5.4; review return F6).
        std::fs::write(fixture.bundle().join("roles/work.md"), "# work\n").unwrap();
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
        "egress": "contracted",
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
    // A recipe authors no `--allowedTools` (operator ruling 1 of
    // 2026-09-23): the equivalent inline seat declares the office's typed
    // allow, which the engine lowers as its own segment (rebuild unit 5b;
    // fixture migration of 2026-09-26).
    let mut inline = fixture.config();
    inline["seats"]["work"] = json!({
        "results": ["complete"],
        "role": "roles/work.md",
        "limits": {"max_attempts": 3, "timeout_seconds": 77},
        "tools": {"allow": ["cargo"]},
        "driver": {"command": [
            "{brokkr}", "driver", "claude", "--",
            "--model", "claude-opus-5",
            "--effort", "high",
        ]},
    });
    let inline = fixture.compile(inline).unwrap();

    // The role is the same TEXT under two names — the agent's charter in
    // the library, the inline seat's role inside its bundle — so the
    // bodies are compared on the bytes each path holds, not on a link
    // from one tree into the other (review return F6). The command is
    // compared with the inline site's lowered segment behind it, as
    // dispatch composes it; the fixture's adapter declares no template.
    let describe = |bundle: &Bundle| {
        let seat = &bundle.seats["work"];
        let SeatBody::Single {
            role_path, command, ..
        } = &seat.body
        else {
            unreachable!("single seat")
        };
        let facts = bundle.sites.get("work");
        assert_eq!(facts.and_then(|facts| facts.inline_template.as_ref()), None);
        let lowered = facts
            .and_then(|facts| facts.inline_local.as_ref())
            .map(|local| local.segment.argv.clone())
            .unwrap_or_default();
        (
            std::fs::read(role_path).unwrap(),
            [command.clone(), lowered].concat(),
            seat.limits.max_attempts,
            seat.limits.timeout_seconds,
            seat.inputs.clone(),
        )
    };
    assert_eq!(describe(&resolved), describe(&inline));
    let role_of = |bundle: &Bundle| match &bundle.seats["work"].body {
        SeatBody::Single { role_path, .. } => role_path.clone(),
        _ => unreachable!("single seat"),
    };
    assert_eq!(
        role_of(&resolved),
        fixture.library().join("charters/work.md")
    );
    assert_eq!(role_of(&inline), fixture.bundle().join("roles/work.md"));
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
            "native_capabilities": {"known": {}},
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
           "charter": "charters/data.md", "models": ["opus"],
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
        let fixture = AgentFixture::declaring();
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
    let fixture = AgentFixture::declaring();
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
    let fixture = AgentFixture::declaring();
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
const CODEX_WORKSPACE: [&str; 8] = [
    "--sandbox",
    "read-only",
    "-c",
    "mcp_servers.brokkr.command=\"{brokkr}\"",
    "-c",
    "mcp_servers.brokkr.args={hands_args_toml}",
    "-c",
    "mcp_servers.brokkr.default_tools_approval_mode=\"approve\"",
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

/// The complete inline sandbox refusal at a site: no inline command lowers
/// a typed sandbox class (design D5.3).
fn inline_sandbox_refusal(site: &str) -> String {
    format!(
        "bundle: seat '{site}' declares 'tools.sandbox' on a site whose command no office \
         composes; the engine does not yet lower a typed local sandbox into an authored \
         command, so the restriction would be recorded and not delivered — it is kept exactly \
         and refused rather than run unrestricted, until decision 0065 slice one's lowering and \
         origin transport prove its delivery (design D5.3); an authored flag cannot stand in for \
         it"
    )
}

/// The complete refusal of a typed allow at an inline site whose command
/// `dispatches` a driver the engine lowers no allow into (rebuild unit 5b).
fn undelivered_allow(site: &str, dispatches: &str) -> String {
    format!(
        "bundle: seat '{site}' declares 'tools.allow' on an inline site whose command \
         {dispatches}; the engine lowers a typed local allow into an inline command only for the \
         claude and lanetally drivers, whose adapters map it onto their tool permissions, so here \
         the restriction would be recorded and not delivered — it is kept exactly and refused \
         rather than run unrestricted (decision 0065 slice one, design D5.3); an authored flag \
         cannot stand in for it"
    )
}

/// An inline Claude command pinned as every inline built-in must be, with
/// `extra` authored behind the pins.
fn claude_inline(driver: &str, extra: &[&str]) -> Value {
    let mut command = vec![
        "{brokkr}",
        "driver",
        driver,
        "--",
        "--model",
        "claude-opus-5",
        "--effort",
        "high",
    ];
    command.extend(extra);
    json!(command)
}

/// The complete refusal of a typed allow at an inline site whose authored
/// command carries the capability-bearing option `canonical` at argument
/// `at` (operator ruling 1; rebuild units 5b-fix and 5b-fix2).
fn carries(canonical: &str, at: usize, kind: &str) -> String {
    format!(
        "bundle: seat 'review' declares 'tools.allow' while its authored command carries \
         '{canonical}' (argument {at}), {kind}; the engine composes the typed list as its own \
         contribution and a recipe authors no capability-bearing option beside it, so the site \
         is refused rather than reconciled (operator ruling 1 of 2026-09-23; decision 0065 slice \
         one, design D5.3)"
    )
}

/// The complete refusal of a typed allow at an inline site whose authored
/// command the `driver` grammar cannot place, which names the argument, a
/// bounded label and the cause, and never the token (rebuild unit 5b-fix2).
fn unreadable_inline(driver: &str, at: usize, label: &str, cause: &str) -> String {
    format!(
        "bundle: seat 'review' declares 'tools.allow' while its authored command cannot be read: \
         the '{driver}' command grammar cannot place argument {at} ({label}), whose token is not \
         echoed because it can carry a value: it {cause}. A control nobody can read is a control \
         nobody can rule on, so it is refused rather than passed through (decision 0066 ruling 6; \
         operator ruling 1 of 2026-09-23)"
    )
}

/// The fixture's one Claude mapping, lowered as the engine's own segment.
fn cargo_lowering() -> Option<crate::agents::LocalLowering> {
    Some(crate::agents::LocalLowering {
        segment: Segment::new(
            brokkr_protocol::native_controls::Origin::Local,
            &["--allowedTools".to_string(), "Bash(cargo:*)".to_string()],
        ),
        limits: vec!["Bash(cargo:*)".to_string()],
    })
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
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
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
    let mut inherited: Vec<Row<(Option<LocalTools>, Vec<String>)>> = Vec::new();
    for tools in [None, Some(json!({})), Some(json!({"mcp": []}))] {
        let bundle = compiled(tools.clone()).unwrap();
        inherited.push((
            format!("{tools:?}"),
            (
                bundle.sites["work"].local.clone(),
                command_of(&bundle, "work")[8..].to_vec(),
            ),
            (
                Some(local(Some(&["cargo", "git"]), None)),
                vec![
                    "--allowedTools".to_string(),
                    "Bash(cargo:*),Bash(git:*)".to_string(),
                ],
            ),
        ));
    }
    each_row(inherited);
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
    let refusals: Vec<(Value, String)> = vec![
        (
            json!({"allow": ["git", "make"]}),
            widening(
                "work",
                "office",
                "allow",
                "names 'make', which the office's 'tools.allow' [\"cargo\", \"git\"] does not; \
                 a site subtracts from its office and never adds to it",
            ),
        ),
        (
            json!({"allow": []}),
            empty_refusal("work", "office", "claude", "opus"),
        ),
        (
            json!({"sandbox": "read-only"}),
            "bundle: seat 'work' requests 'tools.sandbox' 'read-only' without hands; no engine \
             path expresses a sandbox class for a site without hands, so the class would be \
             recorded and not delivered — refused under the `namespace` boundary until \
             decision 0065 slice one's lowering proves it (design D5.3)"
                .to_string(),
        ),
        (
            json!({"allow": null}),
            "bundle: seat 'work' 'tools' needs 'allow' as an array of strings".to_string(),
        ),
        (
            json!(null),
            "bundle: seat 'work' 'tools' must be a JSON object".to_string(),
        ),
        (
            json!([]),
            "bundle: seat 'work' 'tools' must be a JSON object".to_string(),
        ),
        (
            json!({"allow": ["git"], "sandbox": "loose"}),
            "bundle: seat 'work' 'tools.sandbox' is 'loose', which is not one of read-only, \
             workspace-write, danger-full-access"
                .to_string(),
        ),
        (
            json!({"allow": ["git", "git"]}),
            "bundle: seat 'work' 'tools.allow' names 'git' twice; a local allow list is \
             duplicate-free"
                .to_string(),
        ),
    ];
    each_row(
        refusals
            .into_iter()
            .map(|(tools, expected)| {
                (
                    tools.to_string(),
                    outcome(compiled(Some(tools.clone()))),
                    expected,
                )
            })
            .collect(),
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
    let mut rows: Vec<Row<String>> = [None, Some(json!({})), Some(json!({"mcp": []}))]
        .into_iter()
        .map(|tools| {
            (
                format!("{tools:?}"),
                outcome(compiled(tools)),
                "compiled: [(\"review\", Some(LocalTools { allow: None, sandbox: None })), \
                 (\"work\", Some(LocalTools { allow: Some([\"cargo\"]), sandbox: None }))]"
                    .to_string(),
            )
        })
        .collect();
    // Shape is judged before representation: a malformed field names its
    // own cause; a well-formed nonempty field names the missing lowering —
    // this site's command dispatches no driver the engine lowers into.
    let opaque = "dispatches no built-in driver";
    let refusals: Vec<(Value, String)> = vec![
        (json!({"allow": []}), undelivered_allow("review", opaque)),
        (
            json!({"allow": ["cargo"]}),
            undelivered_allow("review", opaque),
        ),
        (
            json!({"sandbox": "read-only"}),
            inline_sandbox_refusal("review"),
        ),
        (
            json!({"sandbox": "workspace-write"}),
            inline_sandbox_refusal("review"),
        ),
        (
            json!({"sandbox": "danger-full-access"}),
            inline_sandbox_refusal("review"),
        ),
        // Both fields present: the allow is judged first and names itself;
        // neither is dropped for the other.
        (
            json!({"allow": ["cargo"], "sandbox": "read-only"}),
            undelivered_allow("review", opaque),
        ),
        (
            json!({"allow": ["Bash(cargo:*)"]}),
            "bundle: seat 'review' 'tools.allow' names 'Bash(cargo:*)', which does not match \
             ^[a-z][a-z0-9-]*$"
                .to_string(),
        ),
        (
            json!({"invented": 1}),
            "bundle: seat 'review' 'tools' has unknown key 'invented'; known keys: allow, \
             sandbox, mcp"
                .to_string(),
        ),
        (
            json!({"sandbox": "loose"}),
            "bundle: seat 'review' 'tools.sandbox' is 'loose', which is not one of read-only, \
             workspace-write, danger-full-access"
                .to_string(),
        ),
    ];
    for (tools, expected) in refusals {
        rows.push((
            tools.to_string(),
            outcome(compiled(Some(tools.clone()))),
            expected,
        ));
    }
    each_row(rows);
}

/// Rebuild unit 5b (design D5.3, D5.7): an inline site whose command
/// dispatches the claude or lanetally driver has its typed allow lowered by
/// unit 3's own lowering onto its adapter's tool permissions and recorded
/// as the engine's `local` segment — the authored command untouched — and
/// every other inline shape refuses with its own complete cause.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn an_inline_claude_or_lanetally_site_lowers_its_allow_and_every_other_shape_refuses() {
    let fixture = AgentFixture::new();
    let compiled = |command: Value, tools: Value, hands: bool| {
        let mut config = fixture.config();
        config["seats"]["review"]["driver"]["command"] = command;
        config["seats"]["review"]["tools"] = tools;
        if hands {
            config["seats"]["review"]["hands"] =
                json!({"kind": "workspace", "network": false, "binds": []});
        }
        fixture.compile(config)
    };
    // The lowered facts, and the command the author wrote, exactly.
    let lowered = |result: Result<Bundle, CompileError>| match result {
        Ok(bundle) => format!(
            "{:?} {:?} {:?}",
            bundle.sites["review"].local,
            bundle.sites["review"].inline_local,
            command_of(&bundle, "review")[1..].to_vec()
        ),
        Err(error) => error.to_string(),
    };
    let authored = |driver: &str| {
        format!(
            "{:?} {:?} {:?}",
            Some(local(Some(&["cargo"]), None)),
            cargo_lowering(),
            [
                "driver",
                driver,
                "--",
                "--model",
                "claude-opus-5",
                "--effort",
                "high"
            ]
        )
    };
    let mut lanetally = claude();
    lanetally["provider"] = json!("lanetally");
    lanetally["driver"] = json!(["{brokkr}", "driver", "lanetally", "--"]);
    lanetally["models"] = json!({"opus-tallied": "claude-opus-5"});
    let allow = json!({"allow": ["cargo"]});
    let mut rows: Vec<Row<String>> = vec![(
        "claude".to_string(),
        lowered(compiled(claude_inline("claude", &[]), allow.clone(), false)),
        authored("claude"),
    )];
    // No adapter declares lanetally yet: nothing maps the list.
    rows.push((
        "lanetally without an adapter".to_string(),
        outcome(compiled(
            claude_inline("lanetally", &[]),
            allow.clone(),
            false,
        )),
        "bundle: seat 'review' declares 'tools.allow' for driver 'lanetally', which no loaded \
         adapter declares; with no tool permission mapping the restriction cannot be \
         expressed, so it is refused rather than run unrestricted (decision 0065 slice one, \
         design D5.3)"
            .to_string(),
    ));
    fixture.write("adapters/lanetally.json", lanetally.clone());
    rows.push((
        "lanetally".to_string(),
        lowered(compiled(
            claude_inline("lanetally", &[]),
            allow.clone(),
            false,
        )),
        authored("lanetally"),
    ));
    // Every other driver refuses, naming what its command dispatches.
    for (driver, command) in [
        ("codex", claude_inline("codex", &[])),
        ("dsh", claude_inline("dsh", &[])),
        (
            "exec",
            json!(["{brokkr}", "driver", "exec", "--", "bash", "x.sh"]),
        ),
    ] {
        rows.push((
            driver.to_string(),
            outcome(compiled(command, allow.clone(), false)),
            undelivered_allow("review", &format!("dispatches the '{driver}' driver")),
        ));
    }
    let site = |cause: &str| format!("bundle: seat 'review': {cause}");
    let refusals: Vec<(&str, Value, Value, bool, String)> = vec![
        (
            "explicit empty",
            claude_inline("claude", &[]),
            json!({"allow": []}),
            false,
            site(
                "the effective 'tools.allow' is explicitly empty, and no serving path yet \
                 expresses an empty local allow set as a delivered restriction (joining no names \
                 into an empty flag value proves nothing); the declaration is kept exactly and \
                 refused rather than run unrestricted, until decision 0065 slice one's lowering \
                 proves its delivery (design D5.3)",
            ),
        ),
        (
            "beside a sandbox",
            claude_inline("claude", &[]),
            json!({"allow": ["cargo"], "sandbox": "read-only"}),
            false,
            inline_sandbox_refusal("review"),
        ),
        (
            "beside hands",
            claude_inline("claude", &[]),
            allow.clone(),
            true,
            "bundle: seat 'review' declares 'tools.allow' beside the site's own hands; hands \
             replace the harness's tools, so a direct local list at an inline site would stand \
             beside the box's restriction rather than express it — it is kept exactly and \
             refused (decision 0065 slice one, design D5.3)"
                .to_string(),
        ),
        (
            "unmapped name",
            claude_inline("claude", &[]),
            json!({"allow": ["git"]}),
            false,
            site("the provider maps no tool permission named 'git'"),
        ),
        (
            "unreadable authored argv",
            claude_inline("claude", &["stray"]),
            allow.clone(),
            false,
            unreadable_inline(
                "claude",
                5,
                "a bare word",
                "is a bare word, and no positional argument is part of the supported shape",
            ),
        ),
    ];
    for (label, command, tools, hands, expected) in refusals {
        rows.push((
            label.to_string(),
            outcome(compiled(command, tools, hands)),
            expected,
        ));
    }
    // An authored capability-bearing option of any kind, in any spelling,
    // is refused beside the typed list rather than merged with or ordered
    // against it, named by its canonical option and position and never by
    // its value (operator ruling 1; rebuild unit 5b-fix, finding S1).
    let loads = "which loads or configures a server, a plugin or a settings document";
    for (driver, written, canonical, at, kind) in [
        (
            "claude",
            vec!["--allowedTools", "Bash(git:*)"],
            "--allowedTools",
            5,
            "a tool list",
        ),
        (
            "claude",
            vec!["--allowed-tools=Bash(git:*)"],
            "--allowedTools",
            5,
            "a tool list",
        ),
        (
            "claude",
            vec!["--tools", "Bash"],
            "--tools",
            5,
            "a tool list",
        ),
        (
            "claude",
            vec!["--disallowedTools", "WebSearch"],
            "--disallowedTools",
            5,
            "a tool list",
        ),
        (
            "claude",
            vec!["--permission-mode", "bypassPermissions"],
            "--permission-mode",
            5,
            "a permission mode",
        ),
        (
            "claude",
            vec!["--permission-mode=bypassPermissions"],
            "--permission-mode",
            5,
            "a permission mode",
        ),
        (
            "lanetally",
            vec!["--verbose", "--permission-mode", "bypassPermissions"],
            "--permission-mode",
            6,
            "a permission mode",
        ),
        (
            "claude",
            vec!["--mcp-config", "servers.json"],
            "--mcp-config",
            5,
            loads,
        ),
        (
            "claude",
            vec!["--mcp-config=servers.json"],
            "--mcp-config",
            5,
            loads,
        ),
        (
            "claude",
            vec!["--strict-mcp-config"],
            "--strict-mcp-config",
            5,
            "an MCP configuration control",
        ),
        (
            "claude",
            vec!["--plugin-dir", "plugins"],
            "--plugin-dir",
            5,
            loads,
        ),
        ("claude", vec!["--settings=s.json"], "--settings", 5, loads),
        (
            "claude",
            vec!["--agents", "agents.json"],
            "--agents",
            5,
            loads,
        ),
    ] {
        rows.push((
            format!("{driver} {}", written.join(" ")),
            outcome(compiled(
                claude_inline(driver, &written),
                allow.clone(),
                false,
            )),
            carries(canonical, at, kind),
        ));
    }
    // Rebuild unit 5b-fix2 (chief S1 and the sweep): an additional
    // directory grants file access, a session selector rejoins a saved
    // working directory, a background session runs under a supervisor the
    // engine does not launch, and a streamed input format carries control
    // messages — each refused in both drivers, in every spelling the
    // grammar places, split, `=`-joined, variadic and repeated, and named
    // by its first occurrence without its value.
    let added = "an additional directory, which grants file access";
    let session = "a session selector, and a rejoined session restores its saved working directory";
    for driver in ["claude", "lanetally"] {
        for (written, canonical, at, kind) in [
            (vec!["--add-dir", "SENTINEL-dir"], "--add-dir", 5, added),
            (vec!["--add-dir=SENTINEL-dir"], "--add-dir", 5, added),
            (
                vec!["--verbose", "--add-dir", "SENTINEL-a", "SENTINEL-b"],
                "--add-dir",
                6,
                added,
            ),
            (
                vec![
                    "--add-dir=SENTINEL-a",
                    "--verbose",
                    "--add-dir",
                    "SENTINEL-b",
                ],
                "--add-dir",
                5,
                added,
            ),
            (vec!["--resume", "SENTINEL-id"], "--resume", 5, session),
            (vec!["-r", "SENTINEL-id"], "--resume", 5, session),
            (vec!["--resume=SENTINEL-id"], "--resume", 5, session),
            (vec!["--continue"], "--continue", 5, session),
            (vec!["-c"], "--continue", 5, session),
            (vec!["--session-id=SENTINEL-id"], "--session-id", 5, session),
            (
                vec!["--bg"],
                "--bg",
                5,
                "a background session, which runs under a supervisor the engine does not launch",
            ),
            (
                vec!["--input-format", "stream-json"],
                "--input-format",
                5,
                "an input format, whose streamed input can carry control messages the engine \
                 does not compose",
            ),
            (
                vec!["--input-format=text"],
                "--input-format",
                5,
                "an input format, whose streamed input can carry control messages the engine \
                 does not compose",
            ),
        ] {
            rows.push((
                format!("{driver} {}", written.join(" ")),
                outcome(compiled(
                    claude_inline(driver, &written),
                    allow.clone(),
                    false,
                )),
                carries(canonical, at, kind),
            ));
        }
    }
    // Every option the sweep judged inert is admitted beside the list, in
    // one command, and lowered exactly as the bare pins are.
    let inert = [
        "--print",
        "--verbose",
        "--no-session-persistence",
        "--fork-session",
        "--output-format=stream-json",
        "--fallback-model",
        "claude-sonnet-5",
        "--system-prompt",
        "SENTINEL-prompt",
        "--append-system-prompt=SENTINEL-append",
        "--system-prompt-snapshot",
        "SENTINEL-snapshot",
        "--max-turns",
        "3",
    ];
    rows.push((
        "every inert option".to_string(),
        match compiled(claude_inline("claude", &inert), allow.clone(), false) {
            Ok(bundle) => format!(
                "{:?} {:?}",
                bundle.sites["review"].inline_local,
                command_of(&bundle, "review")[8..].to_vec()
            ),
            Err(error) => error.to_string(),
        },
        format!("{:?} {:?}", cargo_lowering(), inert),
    ));
    // A switch that bypasses permissions is no option the grammar models,
    // so it never parses; a web option reaches Claude only as a tool name.
    // No grammar failure echoes its token (rebuild unit 5b-fix2, S2): a
    // duplicate, a malformed or an unknown option carrying a value, and a
    // value that reads as an option, are named by a bounded label.
    let repeats = |name: &str| {
        format!(
            "repeats option '{name}', which the grammar admits once; a CLI that resolves a \
             duplicate last-wins would resolve it against the control the engine composed"
        )
    };
    for (driver, written, at, label, cause) in [
        (
            "claude",
            vec!["--dangerously-skip-permissions"],
            5,
            "an option the 'claude' grammar does not model".to_string(),
            "names no option".to_string(),
        ),
        (
            "claude",
            vec!["--settings=SENTINEL-a", "--settings=SENTINEL-b"],
            6,
            "'--settings'".to_string(),
            repeats("--settings"),
        ),
        (
            "lanetally",
            vec![
                "--permission-mode=SENTINEL-a",
                "--permission-mode=SENTINEL-b",
            ],
            6,
            "'--permission-mode'".to_string(),
            repeats("--permission-mode"),
        ),
        (
            "claude",
            vec!["--web-search=SENTINEL"],
            5,
            "an option the 'claude' grammar does not model".to_string(),
            "names no option, or names one that has no equals-joined spelling".to_string(),
        ),
        (
            "lanetally",
            vec!["--strict-mcp-config=SENTINEL"],
            5,
            "'--strict-mcp-config'".to_string(),
            "names no option, or names one that has no equals-joined spelling".to_string(),
        ),
        (
            "claude",
            vec!["--system-prompt", "--SENTINEL=x"],
            6,
            "an option the 'claude' grammar does not model".to_string(),
            "stands where the value of '--system-prompt' belongs but reads as an option, so \
             which of the two it is cannot be told"
                .to_string(),
        ),
        (
            "claude",
            vec!["-"],
            5,
            "a bare word".to_string(),
            "is a bare word, and no positional argument is part of the supported shape".to_string(),
        ),
    ] {
        rows.push((
            format!("{driver} {}", written.join(" ")),
            outcome(compiled(
                claude_inline(driver, &written),
                allow.clone(),
                false,
            )),
            unreadable_inline(driver, at, &label, &cause),
        ));
    }
    // The rest of the accepted Claude catalogue (realm-capability-grants,
    // "Authored provider configuration cannot supply capability authority")
    // is no option the grammar models, so each refuses bare and joined,
    // under the fixed unknown-option label, for both drivers.
    for name in [
        "--allow-dangerously-skip-permissions",
        "--permission-prompt-tool",
        "--setting-sources",
        "--agent",
        "--web",
        "--web-fetch",
        "--search",
    ] {
        for driver in ["claude", "lanetally"] {
            for (token, cause) in [
                (name.to_string(), "names no option"),
                (
                    format!("{name}=SENTINEL"),
                    "names no option, or names one that has no equals-joined spelling",
                ),
            ] {
                rows.push((
                    format!("{driver} {token}"),
                    outcome(compiled(
                        claude_inline(driver, &[&token]),
                        allow.clone(),
                        false,
                    )),
                    unreadable_inline(
                        driver,
                        5,
                        &format!("an option the '{driver}' grammar does not model"),
                        cause,
                    ),
                ));
            }
        }
    }
    // A modelled alias is named by its canonical option however the
    // grammar fails on it (rebuild unit 5b-fix3, R3): a dangling `-r`, a
    // malformed `-c=`, and a duplicate written through an alias.
    for driver in ["claude", "lanetally"] {
        for (written, at, canonical, cause) in [
            (
                vec!["-r"],
                5,
                "--resume",
                "takes a value and is the last argument, so it has none".to_string(),
            ),
            (
                vec!["-c=SENTINEL"],
                5,
                "--continue",
                "names no option, or names one that has no equals-joined spelling".to_string(),
            ),
            (
                vec!["--allowed-tools=SENTINEL-a", "--allowed-tools=SENTINEL-b"],
                6,
                "--allowedTools",
                repeats("--allowedTools"),
            ),
        ] {
            rows.push((
                format!("{driver} {}", written.join(" ")),
                outcome(compiled(
                    claude_inline(driver, &written),
                    allow.clone(),
                    false,
                )),
                unreadable_inline(driver, at, &format!("'{canonical}'"), &cause),
            ));
        }
    }
    // An effort is judged by a fixed classification of its value, never
    // by the adapter's declaration (rebuild unit 5b-fix3, R2): adapters
    // that declare `ultracode` and a 2055-character name beside the plain
    // levels still see `ultracode` refused, in either spelling and case,
    // for both drivers, and an effort they do not declare is refused under
    // the same fixed cause, which names neither the value nor the adapter's
    // vocabulary (R1) — while the reference's plain levels the adapters do
    // not declare stand beside the list. (An authored value past the effort
    // pin's 40-scalar bound never reaches this check: the pin refuses it.)
    let long = "u".repeat(2055);
    let declared = json!(["low", "medium", "high", "ultracode", long]);
    let mut declaring = claude();
    declaring["efforts"] = declared.clone();
    fixture.write("adapters/claude.json", declaring);
    let mut declaring = lanetally.clone();
    declaring["efforts"] = declared;
    fixture.write("adapters/lanetally.json", declaring);
    let effort = |driver: &str, written: &[&str]| {
        let mut command = vec![
            "{brokkr}",
            "driver",
            driver,
            "--",
            "--model",
            "claude-opus-5",
        ];
        command.extend(written);
        json!(command)
    };
    let unplain = "an effort other than the reference's plain levels (low, medium, high, xhigh, \
                   max), which can turn on more than effort";
    for driver in ["claude", "lanetally"] {
        for written in [
            ["--effort", "ultracode"].as_slice(),
            &["--effort=ultracode"],
            &["--effort", "unknown"],
            &["--effort=unknown"],
            &["--effort", "ULTRACODE"],
        ] {
            rows.push((
                format!("{driver} {}", written.join(" ")),
                outcome(compiled(effort(driver, written), allow.clone(), false)),
                carries("--effort", 3, unplain),
            ));
        }
        for level in ["xhigh", "max"] {
            let joined = format!("--effort={level}");
            rows.push((
                format!("{driver} {joined}"),
                match compiled(effort(driver, &[&joined]), allow.clone(), false) {
                    Ok(bundle) => format!("{:?}", bundle.sites["review"].inline_local),
                    Err(error) => error.to_string(),
                },
                format!("{:?}", cargo_lowering()),
            ));
        }
    }
    fixture.write("adapters/claude.json", claude());
    fixture.write("adapters/lanetally.json", lanetally.clone());
    // The adapter's own gaps: a mapped native alias, and no mapping at all.
    let mut aliased = claude();
    aliased["tool_permissions"]["names"]["webfetch"] = json!("WebFetch");
    aliased["tool_permissions"]["names"]["tool"] = json!("./bin/tool");
    fixture.write("adapters/claude.json", aliased);
    // A bundle-relative mapping is expanded in the engine's segment as an
    // agent's composition is, and kept as mapped in the limits.
    rows.push((
        "bundle-relative mapping".to_string(),
        lowered(compiled(
            claude_inline("claude", &[]),
            json!({"allow": ["tool"]}),
            false,
        )),
        format!(
            "{:?} {:?} {:?}",
            Some(local(Some(&["tool"]), None)),
            Some(crate::agents::LocalLowering {
                segment: Segment::new(
                    brokkr_protocol::native_controls::Origin::Local,
                    &[
                        "--allowedTools".to_string(),
                        fixture.bundle().join("bin/tool").display().to_string(),
                    ],
                ),
                limits: vec!["./bin/tool".to_string()],
            }),
            [
                "driver",
                "claude",
                "--",
                "--model",
                "claude-opus-5",
                "--effort",
                "high"
            ]
        ),
    ));
    rows.push((
        "native alias".to_string(),
        outcome(compiled(
            claude_inline("claude", &[]),
            json!({"allow": ["webfetch"]}),
            false,
        )),
        site(
            "tool permission 'webfetch' maps to 'WebFetch', a tool of the provider's native \
             capability 'web-fetch'; a legacy allow entry cannot authorize a capability, so \
             request 'web-fetch' by name under 'capabilities' and let the realm grant it through \
             a tool dialect (decision 0065 ruling 3)",
        ),
    ));
    lanetally["tool_permissions"] = json!("unsupported");
    fixture.write("adapters/lanetally.json", lanetally);
    rows.push((
        "no tool permissions".to_string(),
        outcome(compiled(claude_inline("lanetally", &[]), allow, false)),
        site(
            "the provider declares tool_permissions unsupported, so the site's restriction to \
             [\"cargo\"] cannot be expressed and the site would run with MORE power than it \
             declares",
        ),
    ));
    assert_eq!(rows.len(), 109);
    each_row(rows);
}

/// Rebuild unit 5c (operator ruling of 2026-09-24, "the permission template
/// at inline sites"): where an inline Claude or LaneTally site's typed allow
/// lowers, the compiler records the permission template its adapter declares
/// behind the driver verb as the engine's own `template` segment, beside the
/// lowered list and with the authored command untouched. An adapter that
/// declares none gives none; a site whose allow does not lower records none;
/// an adapter whose own driver does not dispatch the site's driver refuses;
/// and an authored permission mode — the template's own bytes included —
/// and hands keep their refusals. Rebuild unit 5c-fix: beside the emitted
/// segment, the adapter's declaration is recorded as its own typed fact —
/// `Declared`, or `None` for an adapter declaring nothing — exactly where
/// the allow lowers, expanded as the segment is.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn an_inline_site_records_its_adapters_permission_template_only_where_its_allow_lowers() {
    let fixture = AgentFixture::new();
    let compiled = |command: Value, tools: Option<Value>, hands: bool| {
        let mut config = fixture.config();
        config["seats"]["review"]["driver"]["command"] = command;
        if let Some(tools) = tools {
            config["seats"]["review"]["tools"] = tools;
        }
        if hands {
            config["seats"]["review"]["hands"] =
                json!({"kind": "workspace", "network": false, "binds": []});
        }
        fixture.compile(config)
    };
    // The template, its declaration as a typed fact (rebuild unit 5c-fix),
    // the lowered list and the command the author wrote.
    let recorded = |result: Result<Bundle, CompileError>| match result {
        Ok(bundle) => format!(
            "{:?} {:?} {:?} {:?}",
            bundle.sites["review"].inline_template,
            bundle.sites["review"].declared_template,
            bundle.sites["review"].inline_local,
            command_of(&bundle, "review")[1..].to_vec()
        ),
        Err(error) => error.to_string(),
    };
    use brokkr_protocol::native_controls::TemplateExpectation;
    let owned = |argv: &[&str]| argv.iter().map(|part| part.to_string()).collect::<Vec<_>>();
    let declared = |argv: &[&str]| Some(TemplateExpectation::Declared(owned(argv)));
    let expected = |template: Option<&[&str]>,
                    declared: Option<TemplateExpectation>,
                    lowering,
                    driver: &str| {
        format!(
            "{:?} {:?} {:?} {:?}",
            template.map(|argv| Segment::new(
                brokkr_protocol::native_controls::Origin::Template,
                &owned(argv)
            )),
            declared,
            lowering,
            [
                "driver",
                driver,
                "--",
                "--model",
                "claude-opus-5",
                "--effort",
                "high"
            ]
        )
    };
    let accept: &[&str] = &["--permission-mode", "acceptEdits"];
    let with_driver = |provider: &str, driver: Value| {
        let mut adapter = claude();
        adapter["provider"] = json!(provider);
        adapter["driver"] = driver;
        if provider == "lanetally" {
            adapter["models"] = json!({"opus-tallied": "claude-opus-5"});
        }
        fixture.write(&format!("adapters/{provider}.json"), adapter);
    };
    let allow = || Some(json!({"allow": ["cargo"]}));
    let mut rows: Vec<Row<String>> = Vec::new();
    for driver in ["claude", "lanetally"] {
        with_driver(
            driver,
            json!([
                "{brokkr}",
                "driver",
                driver,
                "--",
                "--permission-mode",
                "acceptEdits"
            ]),
        );
        rows.push((
            format!("{driver} with a template"),
            recorded(compiled(claude_inline(driver, &[]), allow(), false)),
            expected(Some(accept), declared(accept), cargo_lowering(), driver),
        ));
    }
    rows.push((
        "no typed allow".to_string(),
        recorded(compiled(claude_inline("claude", &[]), None, false)),
        expected(None, None, None, "claude"),
    ));
    rows.push((
        "an unspecified declaration".to_string(),
        recorded(compiled(
            claude_inline("claude", &[]),
            Some(json!({})),
            false,
        )),
        expected(None, None, None, "claude"),
    ));
    // Rebuild unit 5c-fix: a declaration naming a bundle-relative path is
    // expanded as the emitted segment is, so the two agree on this machine.
    with_driver(
        "claude",
        json!([
            "{brokkr}",
            "driver",
            "claude",
            "--",
            "--permission-mode",
            "./modes/accept"
        ]),
    );
    let relative = fixture.bundle().join("modes/accept");
    let relative = ["--permission-mode", relative.to_str().unwrap()];
    rows.push((
        "a template naming a bundle-relative path".to_string(),
        recorded(compiled(claude_inline("claude", &[]), allow(), false)),
        expected(
            Some(&relative),
            declared(&relative),
            cargo_lowering(),
            "claude",
        ),
    ));
    for written in [
        &["--permission-mode", "acceptEdits"][..],
        &["--permission-mode=acceptEdits"][..],
    ] {
        rows.push((
            format!("an authored {written:?}"),
            outcome(compiled(claude_inline("claude", written), allow(), false)),
            carries("--permission-mode", 5, "a permission mode"),
        ));
    }
    rows.push((
        "beside hands".to_string(),
        outcome(compiled(claude_inline("claude", &[]), allow(), true)),
        "bundle: seat 'review' declares 'tools.allow' beside the site's own hands; hands \
         replace the harness's tools, so a direct local list at an inline site would stand \
         beside the box's restriction rather than express it — it is kept exactly and \
         refused (decision 0065 slice one, design D5.3)"
            .to_string(),
    ));
    let elsewhere = |driver: &str| {
        format!(
            "bundle: seat 'review': the '{driver}' adapter's own driver does not dispatch the \
             '{driver}' driver, so the permission template it declares cannot be placed behind \
             an inline '{driver}' command; the engine emits an adapter's template only as that \
             adapter's agents receive it, and the site is refused rather than launched without \
             it (operator ruling of 2026-09-24, the permission template at inline sites)"
        )
    };
    for (label, driver) in [
        (
            "a driver dispatching another kind",
            json!([
                "{brokkr}",
                "driver",
                "lanetally",
                "--",
                "--permission-mode",
                "acceptEdits"
            ]),
        ),
        (
            "an opaque driver",
            json!(["claude-wrapper", "--permission-mode", "acceptEdits"]),
        ),
    ] {
        with_driver("claude", driver);
        rows.push((
            label.to_string(),
            outcome(compiled(claude_inline("claude", &[]), allow(), false)),
            elsewhere("claude"),
        ));
    }
    for (label, driver) in [
        (
            "no template behind the terminator",
            json!(["{brokkr}", "driver", "claude", "--"]),
        ),
        (
            "no terminator and no template",
            json!(["{brokkr}", "driver", "claude"]),
        ),
    ] {
        with_driver("claude", driver);
        rows.push((
            label.to_string(),
            recorded(compiled(claude_inline("claude", &[]), allow(), false)),
            expected(
                None,
                Some(TemplateExpectation::None),
                cargo_lowering(),
                "claude",
            ),
        ));
    }
    assert_eq!(rows.len(), 12);
    each_row(rows);
}

/// An inline Codex command pinned as every inline built-in must be, with
/// `extra` authored behind the pins (rebuild unit 5d).
fn codex_inline(extra: &[&str]) -> Value {
    let mut command = vec![
        "{brokkr}",
        "driver",
        "codex",
        "--",
        "--model",
        "gpt-6-astra",
        "--effort",
        "high",
    ];
    command.extend(extra);
    json!(command)
}

/// The complete refusal of a typed sandbox `class` at an inline Codex seat
/// of `site_kind`, where only `admitted` is (rebuild unit 5d).
fn narrowed(class: &str, site_kind: &str, admitted: &str) -> String {
    format!(
        "bundle: seat 'review' declares 'tools.sandbox' '{class}' at {site_kind}, where only \
         '{admitted}' is admitted: a gate changes no files, so it runs read-only and delivers its \
         result through the last-message door, a work seat runs workspace-write, and \
         danger-full-access is admitted nowhere (operator ruling of 2026-09-25, inline Codex \
         sandbox classes are narrowed; design D5.3)"
    )
}

/// The complete refusal of a typed sandbox at an inline Codex seat whose
/// authored command carries `canonical` at argument `at` (rebuild unit 5d).
fn codex_carries(class: &str, canonical: &str, at: usize, kind: &str) -> String {
    format!(
        "bundle: seat 'review' declares 'tools.sandbox' '{class}' while its authored command \
         carries '{canonical}' (argument {at}), {kind}; the engine composes the typed class as \
         its own contribution and a recipe authors no capability-bearing option beside it, so \
         the site is refused rather than reconciled (operator ruling 1 of 2026-09-23; decision \
         0065 slice one, design D5.3)"
    )
}

/// Rebuild unit 5d (operator ruling of 2026-09-25, "narrow"; design D5.3):
/// an inline seat whose command dispatches the codex driver has its typed
/// sandbox lowered onto the fragment its adapter declares for the seat's
/// class — `workspace-write` at a work seat through `hands.harness.work`,
/// `read-only` at a gate through `hands.harness.gate`, which opens the
/// last-message door — recorded as the engine's own `local` segment beside
/// the adapter's (absent) template, the authored command untouched. Every
/// other class at either kind of seat, and every other shape, refuses with
/// its own complete, value-free cause.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn an_inline_codex_seat_lowers_its_sandbox_by_class_and_every_other_shape_refuses() {
    use crate::agents::ResultDoor;
    use brokkr_protocol::native_controls::Origin;
    let fixture = AgentFixture::new();
    let compiled = |class: Option<&str>, extra: &[&str], sandbox: &str, adapter: Value| {
        fixture.write("adapters/codex.json", adapter);
        let mut config = fixture.config();
        config["seats"]["review"]["driver"]["command"] = codex_inline(extra);
        config["seats"]["review"]["tools"] = json!({"sandbox": sandbox});
        if let Some(class) = class {
            config["seats"]["review"]["class"] = json!(class);
        }
        let result = fixture.compile(config);
        match result {
            Ok(bundle) => {
                let facts = &bundle.sites["review"];
                format!(
                    "{:?} {:?} {:?} {:?}",
                    facts.inline_sandbox,
                    facts.declared_template,
                    facts.inline_template,
                    facts.inline_local
                )
            }
            Err(error) => error.to_string(),
        }
    };
    let lowered = |class: Sandbox, argv: &[&str], door: ResultDoor| {
        let argv: Vec<String> = argv.iter().map(|part| part.to_string()).collect();
        format!(
            "{:?} {:?} {:?} {:?}",
            Some(InlineSandbox {
                class,
                segment: Segment::new(Origin::Local, &argv),
                door,
            }),
            Some(TemplateExpectation::None),
            None::<Segment>,
            None::<crate::agents::LocalLowering>
        )
    };
    let (work_seat, gate) = ("an inline Codex work seat", "an inline Codex gate");
    let unmatched = |class: &str, site_kind: &str, part: &str| {
        format!(
            "bundle: seat 'review' declares 'tools.sandbox' '{class}' at {site_kind}, but the \
             codex adapter's `hands.harness.{part}` fragment does not express exactly that class; \
             a missing or different fragment is not a representation, and a fragment is neither \
             called narrower nor clamped — refused (design D5.3)"
        )
    };
    let mut work_reads = codex();
    work_reads["hands"]["harness"]["work"] = json!(["--sandbox", "read-only"]);
    let mut no_gate = codex();
    no_gate["hands"]["harness"]
        .as_object_mut()
        .unwrap()
        .remove("gate");
    let mut elsewhere = codex();
    elsewhere["driver"] = json!(["{brokkr}", "driver", "claude", "--"]);
    // Its declared OFF loads under the grammar its driver dispatches
    // (rebuild unit 11), so the template refusal below is still reached.
    elsewhere["native_capabilities"]["known"]["web-search"]["off"] =
        json!({"argv": ["--disallowedTools", "web_search"]});
    let long = long_payload();
    let mut rows: Vec<Row<String>> = vec![
        (
            "work, workspace-write".into(),
            compiled(None, &[], "workspace-write", codex()),
            lowered(Sandbox::WorkspaceWrite, &CODEX_WORK, ResultDoor::File),
        ),
        (
            "gate, read-only".into(),
            compiled(Some("gate"), &[], "read-only", codex()),
            lowered(Sandbox::ReadOnly, &CODEX_GATE, ResultDoor::LastMessage),
        ),
        (
            "work, read-only".into(),
            compiled(None, &[], "read-only", codex()),
            narrowed("read-only", work_seat, "workspace-write"),
        ),
        (
            "work, danger-full-access".into(),
            compiled(None, &[], "danger-full-access", codex()),
            narrowed("danger-full-access", work_seat, "workspace-write"),
        ),
        (
            "gate, workspace-write".into(),
            compiled(Some("gate"), &[], "workspace-write", codex()),
            narrowed("workspace-write", gate, "read-only"),
        ),
        (
            "gate, danger-full-access".into(),
            compiled(Some("gate"), &[], "danger-full-access", codex()),
            narrowed("danger-full-access", gate, "read-only"),
        ),
        (
            "authored --sandbox".into(),
            compiled(None, &["--sandbox", "workspace-write"], "workspace-write", codex()),
            codex_carries(
                "workspace-write",
                "--sandbox",
                5,
                "a sandbox class, which the typed declaration alone supplies",
            ),
        ),
        (
            "authored -C".into(),
            compiled(None, &["-C", "/elsewhere"], "workspace-write", codex()),
            codex_carries(
                "workspace-write",
                "--cd",
                5,
                "a root selector, which moves the root the sandbox class is measured from",
            ),
        ),
        (
            "authored -o at a gate".into(),
            compiled(Some("gate"), &["-o", "/elsewhere"], "read-only", codex()),
            codex_carries(
                "read-only",
                "--output-last-message",
                5,
                "a result capture, which the engine's gate control owns as the last-message door",
            ),
        ),
        (
            "authored --full-auto".into(),
            compiled(None, &["--full-auto"], "workspace-write", codex()),
            codex_carries(
                "workspace-write",
                "--full-auto",
                5,
                "which bears a capability the realm grants and the engine composes",
            ),
        ),
        (
            "authored long unassigned -c".into(),
            compiled(None, &["-c", &long], "workspace-write", codex()),
            codex_carries(
                "workspace-write",
                "--config",
                5,
                "a configuration assignment with no bounded meaning",
            ),
        ),
        (
            "authored bare word".into(),
            compiled(None, &["stray"], "workspace-write", codex()),
            "bundle: seat 'review' declares 'tools.sandbox' 'workspace-write' while its authored \
             command cannot be read: the 'codex' command grammar cannot place argument 5 (a bare \
             word), whose token is not echoed because it can carry a value: it is a bare word, \
             and no positional argument is part of the supported shape. A control nobody can read \
             is a control nobody can rule on, so it is refused rather than passed through \
             (decision 0066 ruling 6; operator ruling 1 of 2026-09-23)"
                .to_string(),
        ),
        (
            "a work fragment of another class".into(),
            compiled(None, &[], "workspace-write", work_reads),
            unmatched("workspace-write", work_seat, "work"),
        ),
        (
            "no gate fragment".into(),
            compiled(Some("gate"), &[], "read-only", no_gate),
            unmatched("read-only", gate, "gate"),
        ),
        (
            "an adapter whose driver is not codex's".into(),
            compiled(None, &[], "workspace-write", elsewhere),
            "bundle: seat 'review': the 'codex' adapter's own driver does not dispatch the 'codex' \
             driver, so the permission template it declares cannot be placed behind an inline \
             'codex' command; the engine emits an adapter's template only as that adapter's \
             agents receive it, and the site is refused rather than launched without it \
             (operator ruling of 2026-09-24, the permission template at inline sites)"
                .to_string(),
        ),
        (
            "a native OFF carrying --sandbox".into(),
            compiled(
                None,
                &[],
                "workspace-write",
                codex_off(&["--sandbox", "workspace-write"]),
            ),
            "bundle: seat 'review' declares 'tools.sandbox' 'workspace-write', but the inline \
             Codex launch of seat 'review' cannot be read whole under the 'codex' grammar \
             (argument 9, '--sandbox': it repeats option '--sandbox', which the grammar admits \
             once; a CLI that resolves a duplicate last-wins would resolve it against the control \
             the engine composed), so none of its effects can be judged; an unclassified option \
             is refused, never passed through (operator ruling of 2026-09-25; rebuild unit \
             5d-fix-b; design D5.3)"
                .to_string(),
        ),
    ];
    // Beside hands, and on a claude command, the class keeps its refusal.
    let mut config = fixture.config();
    fixture.write("adapters/codex.json", codex());
    config["seats"]["review"]["driver"]["command"] = codex_inline(&[]);
    config["seats"]["review"]["tools"] = json!({"sandbox": "workspace-write"});
    config["seats"]["review"]["hands"] =
        json!({"kind": "workspace", "network": false, "binds": []});
    rows.push((
        "hands".into(),
        outcome(fixture.compile(config)),
        "bundle: seat 'review' declares 'tools.sandbox' 'workspace-write' beside the site's own \
         hands; hands replace the harness's tools, so an inline sandbox would stand beside the \
         box's restriction rather than express it — it is kept exactly and refused (decision \
         0065 slice one, design D5.3)"
            .to_string(),
    ));
    let mut config = fixture.config();
    config["seats"]["review"]["driver"]["command"] = claude_inline("claude", &[]);
    config["seats"]["review"]["tools"] = json!({"sandbox": "workspace-write"});
    rows.push((
        "claude".into(),
        outcome(fixture.compile(config)),
        inline_sandbox_refusal("review"),
    ));
    // With no codex adapter loaded, nothing declares the control.
    std::fs::remove_file(fixture.adapters().join("codex.json")).unwrap();
    let mut config = fixture.config();
    config["seats"]["review"]["driver"]["command"] = codex_inline(&[]);
    config["seats"]["review"]["tools"] = json!({"sandbox": "workspace-write"});
    rows.push((
        "no codex adapter".into(),
        outcome(fixture.compile(config)),
        "bundle: seat 'review' declares 'tools.sandbox' 'workspace-write' for driver 'codex', \
         which no loaded adapter declares; with no sandbox control the class cannot be \
         expressed, so it is refused rather than run unrestricted (decision 0065 slice one, \
         design D5.3)"
            .to_string(),
    ));
    assert_eq!(rows.len(), 19);
    each_row(rows);
}

/// Rebuild unit 14a1: an inline site's composition carries the typed
/// serving inputs the final check rebuilds its command from, each equal to
/// its adapter's declaration where its typed declaration lowered: the
/// permission flag and separator a Claude allow lowered onto, and the
/// `hands.harness` fragment a Codex class lowered onto, as declared, its
/// `{result_path}` unfilled. An inline site has no pins or boundary
/// fragment of its adapter's, and carries the typed hands it resolved;
/// beside them, it carries its adapter's `hands.workspace` fragment as
/// declared through the compile's expansion (operator ruling (B) of
/// 2026-09-27). An agent-backed site has no inline composition.
#[test]
fn an_inline_sites_composition_carries_each_serving_input_as_its_adapter_declares_it() {
    use crate::agents::{DeclaredDialect, ServingInputs};
    use brokkr_protocol::native_controls::ListFlag;
    let fixture = AgentFixture::new();
    let workspace = with_schema(&CODEX_WORKSPACE);
    let mut codex = codex();
    codex["hands"]["workspace"] = json!(workspace);
    fixture.write("adapters/codex.json", codex);
    let serving = |site: &str, command: Value, extra: Value| {
        let mut config = fixture.config();
        config["seats"]["review"]["driver"]["command"] = command;
        for (key, value) in extra.as_object().unwrap() {
            config["seats"]["review"][key] = value.clone();
        }
        match fixture.compile(config) {
            Ok(bundle) => format!("{:?}", bundle.sites[site].inline_serving()),
            Err(error) => error.to_string(),
        }
    };
    let dialect = |permissions: Option<(&str, &str)>, sandbox: &[&str]| DeclaredDialect {
        permissions: permissions.map(|(flag, separator)| ListFlag {
            flag: flag.to_string(),
            separator: separator.to_string(),
        }),
        sandbox: sandbox.iter().map(|part| part.to_string()).collect(),
        ..DeclaredDialect::default()
    };
    let carried = |dialect: DeclaredDialect, spec: Option<HandsSpec>| {
        format!(
            "{:?}",
            Some(ServingInputs {
                dialect,
                pins: Vec::new(),
                spec,
            })
        )
    };
    let hands = json!({"kind": "workspace", "network": false, "binds": []});
    let exec = json!(["{brokkr}", "driver", "exec", "--", "bash", "x.sh"]);
    let rows: Vec<Row<String>> = vec![
        (
            "claude allow".into(),
            serving(
                "review",
                claude_inline("claude", &[]),
                json!({"tools": {"allow": ["cargo"]}}),
            ),
            carried(dialect(Some(("--allowedTools", ",")), &[]), None),
        ),
        (
            "codex gate, read-only".into(),
            serving(
                "review",
                codex_inline(&[]),
                json!({"class": "gate", "tools": {"sandbox": "read-only"}}),
            ),
            carried(dialect(None, &CODEX_GATE), None),
        ),
        (
            "codex work, workspace-write".into(),
            serving(
                "review",
                codex_inline(&[]),
                json!({"tools": {"sandbox": "workspace-write"}}),
            ),
            carried(dialect(None, &CODEX_WORK), None),
        ),
        (
            "claude, nothing lowered".into(),
            serving("review", claude_inline("claude", &[]), json!({})),
            carried(dialect(None, &[]), None),
        ),
        (
            "exec with hands".into(),
            serving("review", exec, json!({"hands": hands})),
            carried(dialect(None, &[]), Some(HandsSpec::parse(&hands).unwrap())),
        ),
        (
            "codex with boxed hands".into(),
            serving("review", codex_inline(&[]), json!({"hands": hands})),
            carried(
                DeclaredDialect {
                    hands: workspace.clone(),
                    ..DeclaredDialect::default()
                },
                Some(HandsSpec::parse(&hands).unwrap()),
            ),
        ),
        (
            "codex without hands".into(),
            serving("review", codex_inline(&[]), json!({})),
            carried(dialect(None, &[]), None),
        ),
        (
            "an agent-backed site".into(),
            serving("work", claude_inline("claude", &[]), json!({})),
            format!("{:?}", None::<ServingInputs>),
        ),
    ];
    each_row(rows);
}

/// Rebuild unit 14a4a (operator ruling (B) of 2026-09-27): an inline site
/// with hands is served like an agent, so its plan types the whole
/// `hands.workspace` fragment its driver's adapter declares as the box's
/// hands, the count a boxed agent-backed site's plan types for the same
/// adapter, and the engine's segment for them is that fragment through the
/// compile's expansion. An inline site without hands has neither.
#[test]
fn an_inline_sites_plan_types_its_hands_as_an_agent_backed_sites_plan_does() {
    use brokkr_protocol::native_controls::{Origin, Segment};
    let fixture = AgentFixture::new();
    let workspace = with_schema(&CODEX_WORKSPACE);
    let mut codex = codex();
    codex["hands"]["workspace"] = json!(workspace);
    fixture.write("adapters/codex.json", codex);
    declare_sandbox(&fixture, "read-only");
    let typed = |hands: Option<Value>| {
        let mut config = sandbox_seat(&fixture, None);
        config["seats"]["review"]["driver"]["command"] = codex_inline(&[]);
        if let Some(hands) = hands {
            config["seats"]["review"]["hands"] = hands;
        }
        let bundle = fixture.compile(config).unwrap();
        let count = |label: &str| {
            bundle.sites[label].capabilities.as_ref().unwrap().outcomes[0].controls()["hands"]
                .clone()
        };
        (
            count("review"),
            count("work"),
            bundle.sites["review"].inline_hands.clone(),
        )
    };
    let hands = json!({"kind": "workspace", "network": false, "binds": []});
    let expanded = expand_command(&fixture.bundle(), &workspace);
    assert_eq!(
        typed(Some(hands)),
        (
            json!(10),
            json!(10),
            Some(Segment::new(Origin::Hands, &expanded))
        )
    );
    assert_eq!(workspace.len(), 10);
    assert_ne!(expanded, workspace);
    assert_eq!(typed(None), (json!(0), json!(10), None));
}

/// Review return F1 of rebuild unit 14a4a: an INHERITED inline site's
/// hands expand against the layer that wrote the seat, as an inherited
/// agent-backed site's hands segment does, never against the leaf that
/// merely extends it. The fragment ends in `--output-schema ./schema.json`,
/// so the two directories are told apart.
#[test]
fn an_inherited_inline_sites_hands_expand_against_its_owning_layer_as_an_agents_do() {
    use brokkr_protocol::native_controls::{Origin, Segment};
    let fixture = AgentFixture::new();
    let workspace = with_schema(&CODEX_WORKSPACE);
    let mut codex = codex();
    codex["hands"]["workspace"] = json!(workspace);
    fixture.write("adapters/codex.json", codex);
    declare_sandbox(&fixture, "read-only");
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
    let mut config = sandbox_seat(&fixture, None);
    config["name"] = json!("base");
    config["seats"]["review"]["driver"]["command"] = codex_inline(&[]);
    config["seats"]["review"]["hands"] =
        json!({"kind": "workspace", "network": false, "binds": []});
    std::fs::write(
        base.join("bundle.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    std::fs::write(
        fixture.bundle().join("bundle.json"),
        serde_json::to_vec(&json!({"name": "fixture", "extends": "base"})).unwrap(),
    )
    .unwrap();
    let bundle =
        Bundle::compile_with(&fixture.bundle(), &fixture.library(), &fixture.adapters()).unwrap();
    let agent = match &bundle.sites["work"].chain[0].lowering {
        crate::agents::Lowering::Composed(composition) => composition
            .segments
            .iter()
            .find(|segment| segment.origin == Origin::Hands)
            .cloned(),
        other => panic!("work composes: {other:?}"),
    };
    let expanded = expand_command(&base, &workspace);
    assert_eq!(
        (bundle.sites["review"].inline_hands.clone(), agent),
        (
            Some(Segment::new(Origin::Hands, &expanded)),
            Some(Segment::new(Origin::Hands, &expanded))
        )
    );
    assert_ne!(expanded, expand_command(&fixture.bundle(), &workspace));
}

/// Second review return of rebuild unit 14a4a (C1): a select case the leaf
/// overrides by `override.cases` is owned by the leaf while its seat and
/// default stay the base's. The leaf's inline case expands its hands
/// against the leaf, as the leaf's agent-backed case does, and the
/// inherited inline default against the base that wrote it.
#[test]
fn a_mixed_origin_selects_inline_hands_expand_against_each_cases_owning_layer() {
    use brokkr_protocol::native_controls::{Origin, Segment};
    let fixture = AgentFixture::new();
    let workspace = with_schema(&CODEX_WORKSPACE);
    let mut codex = codex();
    codex["hands"]["workspace"] = json!(workspace);
    fixture.write("adapters/codex.json", codex);
    declare_sandbox(&fixture, "read-only");
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
    let inline = json!({
        "role": "roles/work.md",
        "driver": {"command": codex_inline(&[])},
        "hands": {"kind": "workspace", "network": false, "binds": []},
    });
    let mut config = sandbox_seat(&fixture, None);
    config["name"] = json!("base");
    config["seats"]["review"] = json!({
        "results": ["clean"],
        "select": {
            "on": "strategy",
            "cases": {"feature": inline, "chore": inline},
            "default": inline,
        },
    });
    std::fs::write(
        base.join("bundle.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    std::fs::write(
        fixture.bundle().join("bundle.json"),
        serde_json::to_vec(&json!({
            "name": "fixture",
            "extends": "base",
            "override": {"cases": ["review:feature", "review:chore"]},
            "seats": {"review": {"select": {"cases": {
                "feature": inline,
                "chore": {"agent": "boxed"},
            }}}},
        }))
        .unwrap(),
    )
    .unwrap();
    let bundle =
        Bundle::compile_with(&fixture.bundle(), &fixture.library(), &fixture.adapters()).unwrap();
    let agent = match &bundle.sites["review:chore"].chain[0].lowering {
        crate::agents::Lowering::Composed(composition) => composition
            .segments
            .iter()
            .find(|segment| segment.origin == Origin::Hands)
            .cloned(),
        other => panic!("review:chore composes: {other:?}"),
    };
    let (leaf, inherited) = (
        expand_command(&fixture.bundle(), &workspace),
        expand_command(&base, &workspace),
    );
    assert_eq!(
        (
            bundle.sites["review:feature"].inline_hands.clone(),
            agent,
            bundle.sites["review:default"].inline_hands.clone(),
        ),
        (
            Some(Segment::new(Origin::Hands, &leaf)),
            Some(Segment::new(Origin::Hands, &leaf)),
            Some(Segment::new(Origin::Hands, &inherited)),
        )
    );
    assert_ne!(leaf, inherited);
}

/// The fixture codex's fragments followed by `--output-schema
/// ./schema.json`: an inert option whose bundle-relative value the
/// compile's expansion rewrites, so a carrier expanded with its segment is
/// told apart from one carried as declared (rebuild unit 14a1).
fn with_schema(fragment: &[&str]) -> Vec<String> {
    [fragment, &["--output-schema", "./schema.json"]]
        .concat()
        .into_iter()
        .map(String::from)
        .collect()
}

/// Rebuild unit 14a1 (chief F2 of run 0065-rebuild-unit-14-see-the-uni-5642ffd7):
/// an agent-backed site's compiled composition keeps every serving input as
/// its adapter declares it, through the compile's expansion, while the
/// segments beside it carry the same values expanded: the permission flag,
/// the model and effort pins, the `hands.workspace` fragment where boxed
/// hands compose, both `hands.harness` fragments under the harness
/// boundary, and the agent's typed hands. The adapters name a
/// bundle-relative model and schema, so an expanded carrier cannot pass.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn an_agent_backed_sites_compiled_composition_keeps_each_serving_input_as_declared() {
    use crate::agents::{BoundaryFragments, DeclaredDialect, ServingInputs};
    use brokkr_protocol::native_controls::ListFlag;
    let fixture = AgentFixture::new();
    let root = fixture.bundle();
    let owned = |argv: &[&str]| argv.iter().map(|part| part.to_string()).collect::<Vec<_>>();
    let (workspace, gate, work) = (
        with_schema(&CODEX_WORKSPACE),
        with_schema(&CODEX_GATE),
        with_schema(&CODEX_WORK),
    );
    let mut codex = codex();
    codex["models"]["astra"] = json!("./models/gpt-6-astra");
    codex["hands"]["workspace"] = json!(workspace);
    codex["hands"]["harness"]["gate"] = json!(gate);
    codex["hands"]["harness"]["work"] = json!(work);
    fixture.write("adapters/codex.json", codex);
    let mut claude = claude();
    claude["models"]["opus"] = json!("./models/claude-opus-5");
    fixture.write("adapters/claude.json", claude);
    // The `work` site's first candidate: its carried serving inputs, and
    // every token its segments emit behind the driver's own.
    let compiled = |config: Value, boundary: Boundary| match fixture.compile_under(config, boundary)
    {
        Ok(bundle) => match &bundle.sites["work"].chain[0].lowering {
            crate::agents::Lowering::Composed(composition) => (
                format!("{:?}", composition.serving),
                format!(
                    "{:?}",
                    brokkr_protocol::native_controls::flatten(&composition.segments[1..])
                ),
            ),
            other => (format!("{other:?}"), String::new()),
        },
        Err(error) => (error.to_string(), String::new()),
    };
    let expected = |serving: ServingInputs, emitted: Vec<String>| {
        (
            format!("{:?}", Box::new(serving)),
            format!("{:?}", expand_command(&root, &emitted)),
        )
    };
    let (claude_pins, codex_pins) = (
        owned(&["--model", "./models/claude-opus-5", "--effort", "high"]),
        owned(&["--model", "./models/gpt-6-astra", "--effort", "high"]),
    );
    let spec = Some(HandsSpec::parse(&json!("workspace")).unwrap());
    let codex_serving = |dialect: DeclaredDialect| ServingInputs {
        dialect,
        pins: codex_pins.clone(),
        spec: spec.clone(),
    };
    let mut rows: Vec<Row<String>> = Vec::new();
    let mut row = |label: &str, (serving, emitted), (declared, expanded)| {
        rows.push((format!("{label}, carried"), serving, declared));
        rows.push((format!("{label}, emitted"), emitted, expanded));
    };

    row(
        "claude allow",
        compiled(fixture.config(), Boundary::Namespace),
        expected(
            ServingInputs {
                dialect: DeclaredDialect {
                    permissions: Some(ListFlag {
                        flag: "--allowedTools".into(),
                        separator: ",".into(),
                    }),
                    ..DeclaredDialect::default()
                },
                pins: claude_pins.clone(),
                spec: None,
            },
            [
                claude_pins.clone(),
                owned(&["--allowedTools", "Bash(cargo:*)"]),
            ]
            .concat(),
        ),
    );
    declare_sandbox(&fixture, "read-only");
    row(
        "codex boxed",
        compiled(sandbox_seat(&fixture, None), Boundary::Namespace),
        expected(
            codex_serving(DeclaredDialect {
                hands: workspace.clone(),
                ..DeclaredDialect::default()
            }),
            [codex_pins.clone(), workspace.clone()].concat(),
        ),
    );
    let harness = || DeclaredDialect {
        boundary: BoundaryFragments {
            gate: gate.clone(),
            work: work.clone(),
        },
        ..DeclaredDialect::default()
    };
    row(
        "codex harness gate",
        compiled(sandbox_seat(&fixture, Some("gate")), Boundary::Harness),
        expected(codex_serving(harness()), codex_pins.clone()),
    );
    declare_sandbox(&fixture, "workspace-write");
    row(
        "codex harness work",
        compiled(sandbox_seat(&fixture, None), Boundary::Harness),
        expected(codex_serving(harness()), codex_pins.clone()),
    );
    assert_eq!(rows.len(), 8);
    each_row(rows);
}

/// Rebuild unit 14a1 (chief F1 and F2 of run 0065-rebuild-unit-14-see-the-uni-5642ffd7):
/// an inline Codex seat carries the fragment its class lowered onto as its
/// adapter declares it, beside and never read back from the segment the
/// engine emits, which the compile expands: `./schema.json` stays
/// bundle-relative in the carrier and names the bundle's path in the
/// segment, and `{result_path}`, filled at dispatch, stays in both.
#[test]
fn an_inline_codex_class_carries_its_declared_fragment_beside_its_expanded_emission() {
    let fixture = AgentFixture::new();
    let (gate, work) = (with_schema(&CODEX_GATE), with_schema(&CODEX_WORK));
    let mut codex = codex();
    codex["hands"]["harness"]["gate"] = json!(gate);
    codex["hands"]["harness"]["work"] = json!(work);
    fixture.write("adapters/codex.json", codex);
    // The review site's carried class fragment, and its emitted segment.
    let lowered = |class: Option<&str>, sandbox: &str| {
        let mut config = fixture.config();
        config["seats"]["review"]["driver"]["command"] = codex_inline(&[]);
        config["seats"]["review"]["tools"] = json!({"sandbox": sandbox});
        if let Some(class) = class {
            config["seats"]["review"]["class"] = json!(class);
        }
        match fixture.compile(config) {
            Ok(bundle) => {
                let site = &bundle.sites["review"];
                (
                    format!("{:?}", site.inline_serving().map(|s| s.dialect.sandbox)),
                    format!("{:?}", site.inline_sandbox.as_ref().map(|s| &s.segment)),
                )
            }
            Err(error) => (error.to_string(), String::new()),
        }
    };
    let expected = |declared: &[String]| {
        (
            format!("{:?}", Some(declared)),
            format!(
                "{:?}",
                Some(Segment::new(
                    Origin::Local,
                    &expand_command(&fixture.bundle(), declared)
                ))
            ),
        )
    };
    let mut rows: Vec<Row<String>> = Vec::new();
    for (label, observed, (declared, emitted)) in [
        ("gate", lowered(Some("gate"), "read-only"), expected(&gate)),
        ("work", lowered(None, "workspace-write"), expected(&work)),
    ] {
        rows.push((format!("{label}, carried"), observed.0, declared));
        rows.push((format!("{label}, emitted"), observed.1, emitted));
    }
    each_row(rows);
}

/// Rebuild unit 5d-fix (chief F1 and F2 of run 0065-rebuild-unit-5d-see-the-uni-5b7d59c1;
/// operator ruling of 2026-09-25; design D5.3): the engine's fragment is the
/// only sandbox-bearing element of an inline Codex launch. Every contribution
/// — the adapter's template, the engine's own fragment and the resolved
/// native plan, beside the authored command already judged — is read under
/// the codex grammar, and an option it classifies as a permission control, a
/// writable root, a load or sandbox, approval or unbounded configuration
/// refuses in every spelling. A gate delivers through the last-message door
/// alone, captured into exactly the engine-owned result path by the engine's
/// fragment; file delivery, a missing capture, a capture elsewhere and a
/// capture at a work seat each refuse. Every cause is complete and names no
/// value.
///
/// Rebuild unit 5d-fix-b: the whole launch is one judgment, run once the
/// native plan is resolved, over a closed set of admitted effects. The
/// native plan's sandbox, approval and capture effects, profile and
/// profiles configuration, every key off the allowlist (and an allowlisted
/// key with another value) and an option the grammar cannot place refuse,
/// each naming the seat and the option.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn an_inline_codex_seat_refuses_every_competing_sandbox_contribution_and_every_misbound_capture() {
    let fixture = AgentFixture::new();
    let compiled = |class: Option<&str>, sandbox: &str, adapter: Value| {
        fixture.write("adapters/codex.json", adapter);
        let mut config = fixture.config();
        config["seats"]["review"]["driver"]["command"] = codex_inline(&[]);
        config["seats"]["review"]["tools"] = json!({"sandbox": sandbox});
        if let Some(class) = class {
            config["seats"]["review"]["class"] = json!(class);
        }
        outcome(fixture.compile(config))
    };
    let templated = |tail: &[&str]| {
        let mut adapter = codex();
        let mut driver = vec!["{brokkr}", "driver", "codex", "--"];
        driver.extend(tail);
        adapter["driver"] = json!(driver);
        adapter
    };
    let fragment = |adapter: Value, part: &str, argv: &[&str]| {
        let mut adapter = adapter;
        adapter["hands"]["harness"][part] = json!(argv);
        adapter
    };
    let rest = "the launch admits only the engine's one sandbox fragment of the site's class, at \
                a gate the engine's one capture into the result path it owns, and configuration \
                on a closed allowlist, so every other effect is refused rather than reconciled or \
                ordered, whoever composed it";
    let capture = |class: &str, cause: String| {
        format!(
            "bundle: seat 'review' declares 'tools.sandbox' '{class}', but {cause} (operator \
             ruling of 2026-09-25; rebuild unit 5d-fix-b; design D5.3)"
        )
    };
    let competing = |class: &str, origin: &str, canonical: &str, at: usize, effect: &str| {
        capture(
            class,
            format!(
                "the inline Codex launch of seat 'review' carries '{canonical}' (argument {at}) \
                 in its `{origin}` contribution, {effect}; {rest}"
            ),
        )
    };
    let unreadable = |class: &str, at: usize, label: &str, cause: &str| {
        capture(
            class,
            format!(
                "the inline Codex launch of seat 'review' cannot be read whole under the 'codex' \
                 grammar (argument {at}, {label}: it {cause}), so none of its effects can be \
                 judged; an unclassified option is refused, never passed through"
            ),
        )
    };
    let permission = "a permission control, which sets, lifts or replaces the sandbox or its \
                      approvals";
    let table = |name: &str| {
        format!(
            "a configuration assignment that assigns into the '{name}' configuration, which is \
             outside the closed set of keys an inline Codex launch admits"
        )
    };
    let work = |origin: &str, canonical: &str, at: usize, effect: &str| {
        competing("workspace-write", origin, canonical, at, effect)
    };
    let misdirected = |origin: &str, at: usize| {
        competing(
            "read-only",
            origin,
            "--output-last-message",
            at,
            "a result capture other than the engine's own into exactly the result path it owns, \
             a harness write path outside the result sink",
        )
    };
    let at_work = "a result capture at a work seat, whose result is the file the seat writes; a \
                   capture the engine does not own is a harness write path outside the result \
                   sink";
    let off_key = "a configuration assignment that assigns a key outside the closed set an \
                   inline Codex launch admits";
    let compiled_as = |review: &str| {
        format!(
            "compiled: [(\"review\", Some(LocalTools {{ allow: None, sandbox: Some({review}) }})), \
             (\"work\", Some(LocalTools {{ allow: Some([\"cargo\"]), sandbox: None }}))]"
        )
    };
    let repeats = |option: &str| {
        format!(
            "repeats option '{option}', which the grammar admits once; a CLI that resolves a \
             duplicate last-wins would resolve it against the control the engine composed"
        )
    };
    let gate_reads = &["--sandbox", "read-only"];
    let mut file_door = codex();
    file_door["hands"]["harness"]["result"] = json!("file");
    let rows: Vec<Row<String>> =
        vec![
        (
            "template --dangerously-bypass-approvals-and-sandbox".into(),
            compiled(
                None,
                "workspace-write",
                templated(&["--dangerously-bypass-approvals-and-sandbox"]),
            ),
            work(
                "template",
                "--dangerously-bypass-approvals-and-sandbox",
                5,
                permission,
            ),
        ),
        (
            "template --full-auto".into(),
            compiled(None, "workspace-write", templated(&["--full-auto"])),
            work("template", "--full-auto", 5, permission),
        ),
        (
            "template --add-dir".into(),
            compiled(None, "workspace-write", templated(&["--add-dir", "/x"])),
            work(
                "template",
                "--add-dir",
                5,
                "a writable root beyond the sandbox class's reach",
            ),
        ),
        (
            "template -a".into(),
            compiled(None, "workspace-write", templated(&["-a", "never"])),
            work("template", "--ask-for-approval", 5, permission),
        ),
        (
            "template -c sandbox_mode".into(),
            compiled(
                None,
                "workspace-write",
                templated(&["-c", "sandbox_mode=\"danger-full-access\""]),
            ),
            work("template", "--config", 5, &table("sandbox_mode")),
        ),
        (
            "template -c attached sandbox_workspace_write".into(),
            compiled(
                None,
                "workspace-write",
                templated(&["-csandbox_workspace_write.network_access=true"]),
            ),
            work("template", "--config", 5, &table("sandbox_workspace_write")),
        ),
        (
            "template --config= approval_policy".into(),
            compiled(
                None,
                "workspace-write",
                templated(&["--config=approval_policy=\"never\""]),
            ),
            work("template", "--config", 5, &table("approval_policy")),
        ),
        (
            "template -c unbounded".into(),
            compiled(
                None,
                "workspace-write",
                templated(&["-c", "unmodelled.key=1"]),
            ),
            work("template", "--config", 5, off_key),
        ),
        (
            "template -C".into(),
            compiled(None, "workspace-write", templated(&["-C", "/elsewhere"])),
            work(
                "template",
                "--cd",
                5,
                "a root selector, which moves the root the sandbox class is measured from",
            ),
        ),
        (
            "template -c inert effort stands".into(),
            compiled(
                None,
                "workspace-write",
                templated(&["-c", "model_reasoning_effort=\"high\""]),
            ),
            compiled_as("WorkspaceWrite"),
        ),
        (
            "template -p".into(),
            compiled(None, "workspace-write", templated(&["-p", "x"])),
            work(
                "template",
                "--profile",
                5,
                "a configuration document the engine cannot see into, which can set the sandbox",
            ),
        ),
        (
            "template -s beside the fragment's class".into(),
            compiled(
                None,
                "workspace-write",
                templated(&["-s", "workspace-write"]),
            ),
            unreadable("workspace-write", 7, "'--sandbox'", &repeats("--sandbox")),
        ),
        (
            "fragment -a beside its class".into(),
            compiled(
                None,
                "workspace-write",
                fragment(codex(), "work", &["--sandbox", "workspace-write", "-a", "never"]),
            ),
            work("local", "--ask-for-approval", 7, permission),
        ),
        // Rebuild unit 11 re-plant: a declared approval value has no
        // bounded set, so the load refuses it; a value-free permission
        // switch still reaches this refusal.
        (
            "native OFF --dangerously-bypass-approvals-and-sandbox".into(),
            compiled(
                None,
                "workspace-write",
                codex_off(&["--dangerously-bypass-approvals-and-sandbox"]),
            ),
            work(
                "native",
                "--dangerously-bypass-approvals-and-sandbox",
                9,
                permission,
            ),
        ),
        (
            "gate, file delivery".into(),
            compiled(Some("gate"), "read-only", file_door),
            "bundle: seat 'review' declares 'tools.sandbox' 'read-only' at an inline Codex gate, \
             but the codex adapter declares its `hands.harness.result` door as 'file'; a gate \
             delivers only through the last-message door, the harness's capture of its final \
             message into the engine-owned result path, so file delivery is refused (decision \
             0046 ruling 4; operator ruling of 2026-09-25; rebuild unit 5d-fix)"
                .to_string(),
        ),
        (
            "gate, no capture".into(),
            compiled(Some("gate"), "read-only", fragment(codex(), "gate", gate_reads)),
            capture(
                "read-only",
                "the inline Codex launch of seat 'review' is a gate's, and no contribution \
                 carries the engine's capture into the result path it owns, which the \
                 last-message door needs, so the gate's result could not be delivered"
                    .to_string(),
            ),
        ),
        (
            "gate, capture elsewhere".into(),
            compiled(
                Some("gate"),
                "read-only",
                fragment(
                    codex(),
                    "gate",
                    &["--sandbox", "read-only", "-o", "/elsewhere"],
                ),
            ),
            misdirected("local", 7),
        ),
        (
            "gate, capture in the template".into(),
            compiled(
                Some("gate"),
                "read-only",
                fragment(templated(&["-o", "{result_path}"]), "gate", gate_reads),
            ),
            misdirected("template", 5),
        ),
        (
            "work, a capture".into(),
            compiled(
                None,
                "workspace-write",
                fragment(
                    codex(),
                    "work",
                    &["--sandbox", "workspace-write", "-o", "{result_path}"],
                ),
            ),
            work("local", "--output-last-message", 7, at_work),
        ),
        // Rebuild unit 5d-fix-b (chief F1–F3 of run
        // 0065-rebuild-unit-5d-fix-see-the-569be761): the native plan, profile
        // configuration and every key off the closed allowlist, each paired
        // with its valid control.
        (
            "work as compiled".into(),
            compiled(None, "workspace-write", codex()),
            compiled_as("WorkspaceWrite"),
        ),
        (
            "gate as compiled".into(),
            compiled(Some("gate"), "read-only", codex()),
            compiled_as("ReadOnly"),
        ),
        (
            "native OFF --full-auto".into(),
            compiled(None, "workspace-write", codex_off(&["--full-auto"])),
            work("native", "--full-auto", 9, permission),
        ),
        // Rebuild unit 11 (the second review's F1): a declared assignment
        // is read at load by the same bounded reader, so an off-allowlist
        // native OFF meets the load's refusal and never reaches this launch.
        (
            "native OFF -c sandbox_mode".into(),
            compiled(
                None,
                "workspace-write",
                codex_off(&["-c", "sandbox_mode=\"danger-full-access\""]),
            ),
            off_config_refusal(&fixture, &off_allowlist("sandbox_mode")),
        ),
        (
            "native OFF -c approval_policy".into(),
            compiled(
                None,
                "workspace-write",
                codex_off(&["-c", "approval_policy=\"never\""]),
            ),
            off_config_refusal(&fixture, &off_allowlist("approval_policy")),
        ),
        (
            "native OFF -s at a gate".into(),
            compiled(Some("gate"), "read-only", codex_off(&["-s", "read-only"])),
            unreadable("read-only", 11, "'--sandbox'", &repeats("--sandbox")),
        ),
        (
            "native OFF -o at a work seat".into(),
            compiled(None, "workspace-write", codex_off(&["-o", "/elsewhere"])),
            work("native", "--output-last-message", 9, at_work),
        ),
        (
            "native OFF -o at a gate".into(),
            compiled(Some("gate"), "read-only", codex_off(&["-o", "/elsewhere"])),
            unreadable(
                "read-only",
                11,
                "'--output-last-message'",
                &repeats("--output-last-message"),
            ),
        ),
        (
            "native OFF -o at a gate whose fragment captures nothing".into(),
            compiled(
                Some("gate"),
                "read-only",
                fragment(codex_off(&["-o", "{result_path}"]), "gate", gate_reads),
            ),
            misdirected("native", 9),
        ),
        (
            "native OFF web_search, another value".into(),
            compiled(
                None,
                "workspace-write",
                codex_off(&["-c", "web_search=\"live\""]),
            ),
            off_config_refusal(
                &fixture,
                "assigns 'web_search' a value outside the bounded ones its declaration admits",
            ),
        ),
        (
            "native OFF web_search, a descendant key".into(),
            compiled(
                None,
                "workspace-write",
                codex_off(&["-c", "web_search.mode=\"disabled\""]),
            ),
            off_config_refusal(&fixture, &off_allowlist("web_search")),
        ),
        (
            "template -c profile".into(),
            compiled(None, "workspace-write", templated(&["-c", "profile=x"])),
            work("template", "--config", 5, &table("profile")),
        ),
        (
            "template -c profiles sandbox_mode".into(),
            compiled(
                None,
                "workspace-write",
                templated(&["-c", "profiles.x.sandbox_mode=\"danger-full-access\""]),
            ),
            work("template", "--config", 5, &table("profiles")),
        ),
        (
            "template -c profiles approval_policy".into(),
            compiled(
                None,
                "workspace-write",
                templated(&["--config=profiles.x.approval_policy=\"never\""]),
            ),
            work("template", "--config", 5, &table("profiles")),
        ),
        (
            "native OFF -c profile".into(),
            compiled(None, "workspace-write", codex_off(&["-cprofile=x"])),
            off_config_refusal(&fixture, &off_allowlist("profile")),
        ),
        (
            "template -c effort outside its levels".into(),
            compiled(
                None,
                "workspace-write",
                templated(&["-c", "model_reasoning_effort=\"ultra\""]),
            ),
            work(
                "template",
                "--config",
                5,
                "a configuration assignment that assigns 'model_reasoning_effort' a value outside \
                 the bounded ones its declaration admits",
            ),
        ),
        (
            "template -c no assignment".into(),
            compiled(None, "workspace-write", templated(&["-c", "bare"])),
            work(
                "template",
                "--config",
                5,
                "a configuration assignment that is not a KEY=VALUE configuration assignment",
            ),
        ),
        (
            "template --search".into(),
            compiled(None, "workspace-write", templated(&["--search"])),
            work(
                "template",
                "--search",
                5,
                "a capability-bearing control outside the closed set an inline Codex launch \
                 admits",
            ),
        ),
        (
            "template, an unclassified option".into(),
            compiled(None, "workspace-write", templated(&["--frobnicate"])),
            unreadable("workspace-write", 5, "'--frobnicate'", "names no option"),
        ),
    ];
    assert_eq!(rows.len(), 38);
    each_row(rows);
}

/// Rebuild unit 5d-fix-b: the whole-launch judgment requires exactly the
/// engine's own fragment of the admitted class, whatever composed the rest.
/// The compile and the seal each refuse a missing or different fragment
/// first, so the judgment's own requirement is shown on segments directly.
#[test]
fn the_inline_codex_launch_judgment_requires_the_engines_fragment_of_the_class() {
    use brokkr_protocol::native_controls::SandboxIntent;
    let segment = |origin: Origin, argv: &[&str]| {
        Segment::new(
            origin,
            &argv.iter().map(|part| part.to_string()).collect::<Vec<_>>(),
        )
    };
    // Rebuild unit 5d-fix-c1: the judgment is the grammar's, and the site is
    // named by the one wrapper both boundaries use.
    let judged_as = |class: SandboxIntent, segments: &[Segment]| {
        format!(
            "{:?}",
            brokkr_protocol::native_controls::grammar::judge_inline_codex_launch(
                class, segments, None
            )
            .map_err(|cause| inline_codex_refusal("work", &cause))
        )
    };
    let judged = |segments: &[Segment]| judged_as(Sandbox::WorkspaceWrite.intent(), segments);
    let rows: Vec<Row<String>> = vec![
        (
            "the fragment".into(),
            judged(&[segment(Origin::Local, &["--sandbox", "workspace-write"])]),
            "Ok(())".into(),
        ),
        (
            "no fragment".into(),
            judged(&[segment(Origin::Authored, &["--model", "m"])]),
            "Err(\"the inline Codex launch of seat 'work' carries no sandbox fragment of the \
             site's 'workspace-write' class in the engine's `local` contribution, so the class \
             it was admitted with would not reach the harness\")"
                .into(),
        ),
        (
            "the class in another contribution".into(),
            judged(&[segment(Origin::Template, &["--sandbox", "workspace-write"])]),
            "Err(\"the inline Codex launch of seat 'work' carries '--sandbox' (argument 1) in \
             its `template` contribution, a permission control, which sets, lifts or replaces \
             the sandbox or its approvals; the launch admits only the engine's one sandbox \
             fragment of the site's class, at a gate the engine's one capture into the result \
             path it owns, and configuration on a closed allowlist, so every other effect is \
             refused rather than reconciled or ordered, whoever composed it\")"
                .into(),
        ),
        (
            "a local fragment of another class".into(),
            judged(&[segment(Origin::Local, &["--sandbox", "read-only"])]),
            "Err(\"the inline Codex launch of seat 'work' carries '--sandbox' (argument 1) in \
             its `local` contribution, a permission control, which sets, lifts or replaces the \
             sandbox or its approvals; the launch admits only the engine's one sandbox fragment \
             of the site's class, at a gate the engine's one capture into the result path it \
             owns, and configuration on a closed allowlist, so every other effect is refused \
             rather than reconciled or ordered, whoever composed it\")"
                .into(),
        ),
        // Rebuild unit 5d-fix-c1: a judgment with no class to judge refuses
        // before any fragment could stand for one.
        (
            "no class".into(),
            judged_as(
                SandboxIntent::Unspecified,
                &[segment(Origin::Local, &["--sandbox", "workspace-write"])],
            ),
            "Err(\"the inline Codex launch of seat 'work' is judged with no sandbox class, so no \
             fragment of the engine's could express the class it was admitted with\")"
                .into(),
        ),
    ];
    assert_eq!(rows.len(), 5);
    each_row(rows);
}

/// Rebuild unit 5d-fix-c1 (chief F4 of run 0065-rebuild-unit-5d-fix-b-see-t-8067eebc):
/// admission names the seat in the one bounded representation the dispatch
/// door uses, at both places its refusal names it. A dotted label keeps its
/// identity; a long label with a newline is named by its lead and length.
#[test]
fn an_inline_codex_admission_names_its_seat_bounded_and_keeps_a_dotted_identity() {
    let fixture = AgentFixture::new();
    // Re-planted by rebuild unit 11: a declared approval value no longer
    // loads, and a value-free permission switch reaches the same refusal.
    fixture.write("adapters/codex.json", codex_off(&["--full-auto"]));
    let long = format!("work\n{}", "w".repeat(100));
    let refused = |named: &str| {
        format!(
            "bundle: seat {named} declares 'tools.sandbox' 'workspace-write', but the inline Codex \
             launch of seat {named} carries '--full-auto' (argument 9) in its `native` \
             contribution, a permission control, which sets, lifts or replaces the sandbox or its \
             approvals; the launch admits only the engine's one sandbox fragment of the site's \
             class, at a gate the engine's one capture into the result path it owns, and \
             configuration on a closed allowlist, so every other effect is refused rather than \
             reconciled or ordered, whoever composed it (operator ruling of 2026-09-25; rebuild \
             unit 5d-fix-b; design D5.3)"
        )
    };
    let rows: Vec<Row<String>> = [
        ("plain", "work", refused("'work'")),
        ("dotted", "work.v1", refused("'work.v1'")),
        (
            "long, with a newline",
            long.as_str(),
            refused("'work…' (105 bytes, not echoed in full)"),
        ),
    ]
    .into_iter()
    .map(|(label, seat, expected)| {
        // The seat stands in the policy's first phase, under its own name.
        let mut config = fixture.config();
        config["seats"].as_object_mut().unwrap().remove("work");
        config["seats"][seat] = json!({
            "results": ["complete"],
            "role": "roles/work.md",
            "driver": {"command": codex_inline(&[])},
            "tools": {"sandbox": "workspace-write"},
        });
        let mut table = policy();
        table["phases"][0] = json!(seat);
        table["initial"] = json!(seat);
        table["rules"][0]["from"] = json!(seat);
        (
            label.into(),
            outcome(fixture.compile_with_policy(config, &table)),
            expected,
        )
    })
    .collect();
    assert_eq!(rows.len(), 3);
    each_row(rows);
}

/// Rebuild unit 5c-fix2 (operator ruling 2 of 2026-09-23; the ruling of
/// 2026-09-24, item 2): an agent-backed site's compiled composition carries
/// its adapter's declared permission template as a typed fact, expanded as
/// its driver segment is, so the two agree on this machine: `acceptEdits`
/// for the shipped Claude and LaneTally shapes, a bundle-relative path
/// expanded exactly as the segment's, and `none` for a driver that declares
/// nothing behind its verb.
#[test]
fn an_agent_backed_sites_composition_records_its_adapters_template_expanded_as_its_segment() {
    use brokkr_protocol::native_controls::TemplateExpectation;
    let fixture = AgentFixture::new();
    let owned = |argv: &[&str]| argv.iter().map(|part| part.to_string()).collect::<Vec<_>>();
    // The declared template beside the driver segment behind the engine's
    // own path, for the compiled `work` site's first candidate.
    let recorded = |provider: &str, driver: Value| {
        let mut adapter = claude();
        adapter["provider"] = json!(provider);
        adapter["driver"] = driver;
        fixture.write(&format!("adapters/{provider}.json"), adapter);
        match fixture.compile(fixture.config()) {
            Ok(bundle) => match &bundle.sites["work"].chain[0].lowering {
                crate::agents::Lowering::Composed(composition) => format!(
                    "{:?} {:?}",
                    composition.template,
                    composition.segments[0].argv[1..].to_vec()
                ),
                other => format!("{other:?}"),
            },
            Err(error) => error.to_string(),
        }
    };
    let expected = |template: TemplateExpectation, driver: &[&str]| {
        format!("{template:?} {:?}", owned(driver))
    };
    let accept = ["--permission-mode", "acceptEdits"];
    let relative = fixture.bundle().join("modes/accept");
    let relative = ["--permission-mode", relative.to_str().unwrap()];
    let mut rows: Vec<Row<String>> = Vec::new();
    for provider in ["claude", "lanetally"] {
        if provider == "lanetally" {
            // Only one adapter serves the office's model at a time.
            std::fs::remove_file(fixture.adapters().join("claude.json")).unwrap();
        }
        rows.push((
            format!("the shipped {provider} shape"),
            recorded(
                provider,
                json!([
                    "{brokkr}",
                    "driver",
                    provider,
                    "--",
                    "--permission-mode",
                    "acceptEdits"
                ]),
            ),
            expected(
                TemplateExpectation::Declared(owned(&accept)),
                &["driver", provider, "--", accept[0], accept[1]],
            ),
        ));
    }
    std::fs::remove_file(fixture.adapters().join("lanetally.json")).unwrap();
    rows.push((
        "a template naming a bundle-relative path".to_string(),
        recorded(
            "claude",
            json!([
                "{brokkr}",
                "driver",
                "claude",
                "--",
                "--permission-mode",
                "./modes/accept"
            ]),
        ),
        expected(
            TemplateExpectation::Declared(owned(&relative)),
            &["driver", "claude", "--", relative[0], relative[1]],
        ),
    ));
    rows.push((
        "nothing behind the terminator".to_string(),
        recorded("claude", json!(["{brokkr}", "driver", "claude", "--"])),
        expected(TemplateExpectation::None, &["driver", "claude", "--"]),
    ));
    assert_eq!(rows.len(), 4);
    each_row(rows);
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
    let context = "the adapter data is where a driver's model mapping (decision 0016) and its \
                   trust tier and binding grant (decision 0021) are declared, and this bundle \
                   names an agent, seats a gate, declares a secret binding or declares typed \
                   tools";
    let missing = (
        "missing adapters directory".to_string(),
        outcome(fixture.compile(config.clone())),
        format!(
            "bundle: adapters {}: No such file or directory (os error 2); {context}",
            fixture.adapters().display()
        ),
    );
    // A malformed adapter file is the loader's refusal, wrapped the same
    // way — the context is obtained fallibly, never swallowed.
    std::fs::create_dir_all(fixture.adapters()).unwrap();
    std::fs::write(fixture.adapters().join("claude.json"), "{").unwrap();
    let malformed = (
        "malformed adapter file".to_string(),
        outcome(fixture.compile(config)),
        format!(
            "bundle: {}: EOF while parsing an object at line 1 column 1; {context}",
            fixture.adapters().join("claude.json").display()
        ),
    );
    each_row(vec![missing, malformed]);
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
    let rows: Vec<Row<String>> = cases
        .into_iter()
        .map(|(text, key, value)| {
            fixture.stage(&json!({}), &policy());
            std::fs::write(fixture.bundle().join("bundle.json"), &text).unwrap();
            // The parser stands one past the token that closed the SECOND
            // copy of the value.
            let first = text.find(value).unwrap();
            let second = first + 1 + text[first + 1..].find(value).unwrap();
            let column = second + value.len() + 1;
            (
                format!("repeated '{key}'"),
                outcome(Bundle::compile_with(
                    &fixture.bundle(),
                    &fixture.library(),
                    &fixture.adapters(),
                )),
                format!(
                    "bundle: {}: key '{key}' is written twice at line 1 column {column}",
                    fixture.bundle().join("bundle.json").display()
                ),
            )
        })
        .collect();
    each_row(rows);
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
    let mut rows: Vec<Row<String>> = Vec::new();
    let mut config = fixture.config();
    config["seats"]["work"] = panel(json!({}));
    rows.push((
        "panel at a seat".to_string(),
        outcome(fixture.compile_with_policy(config, &panel_policy())),
        container_refusal("work"),
    ));
    let mut config = fixture.config();
    config["seats"]["work"] = json!({
        "results": ["complete"], "tools": {"allow": ["cargo"]},
        "sequence": [
            {"name": "first", "results": ["complete"], "role": "roles/work.md",
             "driver": {"command": ["driver"]}},
            {"name": "second", "role": "roles/work.md", "driver": {"command": ["driver"]}},
        ],
    });
    rows.push((
        "sequence at a seat".to_string(),
        outcome(fixture.compile(config)),
        container_refusal("work"),
    ));
    let mut config = fixture.config();
    config["seats"]["work"] = json!({
        "results": ["complete"], "tools": {},
        "select": {"on": "strategy", "cases": {}, "default": member},
    });
    rows.push((
        "select at a seat".to_string(),
        outcome(fixture.compile(config)),
        container_refusal("work"),
    ));
    // A selected body that is itself a panel.
    let mut config = fixture.config();
    config["seats"]["work"] = json!({
        "results": ["pass", "fail"],
        "select": {"on": "strategy", "cases": {},
                   "default": {"aggregate": "unanimous-pass",
                               "panel": {"a": member, "b": member}, "tools": {}}},
    });
    rows.push((
        "panel as a selected body".to_string(),
        outcome(fixture.compile_with_policy(config, &panel_policy())),
        container_refusal("work:default"),
    ));
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
    rows.push((
        "panel as a sequence step".to_string(),
        outcome(fixture.compile(config)),
        container_refusal("work:first"),
    ));
    each_row(rows);
}

/// SCM "Each executable body owns its local declaration", the executable
/// half: the same subset, explicit empty and widening at an ordinary seat,
/// a panel member, a sequence step, a selected case, a selected default and
/// an inherited body each yield the same effective value or the complete
/// owning-site refusal.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn every_executable_form_owns_its_local_declaration() {
    let fixture = AgentFixture::new();
    write_office(&fixture);
    let inline = json!({"role": "roles/work.md", "driver": {"command": ["driver"]}});
    // (label, config builder, policy). The builder takes the site's
    // `tools` value, or `None` for a site that OMITS the key, so omission
    // is its own row and not a spelling of `{}`.
    type Form<'a> = (&'a str, Box<dyn Fn(Option<Value>) -> Value + 'a>, Value);
    let forms: Vec<Form<'_>> = vec![
        (
            "work",
            Box::new(|tools| {
                with_tools(json!({"results": ["complete"], "agent": "office"}), tools)
            }),
            policy(),
        ),
        (
            "work:a",
            Box::new(|tools| {
                json!({"results": ["pass", "fail"], "aggregate": "unanimous-pass",
                       "panel": {"a": with_tools(json!({"agent": "office"}), tools),
                                 "b": inline.clone()}})
            }),
            panel_policy(),
        ),
        (
            "work:first",
            Box::new(|tools| {
                json!({"results": ["complete"], "sequence": [
                    with_tools(json!({"name": "first", "results": ["complete"],
                                      "agent": "office"}), tools),
                    {"name": "second", "role": "roles/work.md",
                     "driver": {"command": ["driver"]}}]})
            }),
            policy(),
        ),
        (
            "work:engine",
            Box::new(|tools| {
                json!({"results": ["complete"], "select": {"on": "strategy",
                    "cases": {"engine": with_tools(json!({"agent": "office"}), tools)},
                    "default": inline.clone()}})
            }),
            policy(),
        ),
        (
            "work:default",
            Box::new(|tools| {
                json!({"results": ["complete"], "select": {"on": "strategy", "cases": {},
                    "default": with_tools(json!({"agent": "office"}), tools)}})
            }),
            policy(),
        ),
    ];
    /// The effective value and the composed argv after the dispatch, model
    /// and effort tokens, as one observed row.
    fn effective(result: Result<Bundle, CompileError>, label: &str) -> String {
        match result {
            Ok(bundle) => format!(
                "{:?} {:?}",
                bundle.sites[label].local,
                &bundle.sites[label].chain[0].argv[8..]
            ),
            Err(error) => error.to_string(),
        }
    }
    let mut rows: Vec<Row<String>> = Vec::new();
    for (label, seat, table) in &forms {
        let compiled = |tools: Option<Value>| {
            let mut config = fixture.config();
            config["seats"]["work"] = seat(tools);
            fixture.compile_with_policy(config, table)
        };
        // Omission inherits the office whole (review return F5).
        rows.push((
            format!("{label} omission"),
            effective(compiled(None), label),
            format!(
                "{:?} {:?}",
                Some(local(Some(&["cargo", "git"]), None)),
                ["--allowedTools", "Bash(cargo:*),Bash(git:*)"]
            ),
        ));
        rows.push((
            format!("{label} subset"),
            effective(compiled(Some(json!({"allow": ["git"]}))), label),
            format!(
                "{:?} {:?}",
                Some(local(Some(&["git"]), None)),
                ["--allowedTools", "Bash(git:*)"]
            ),
        ));
        rows.push((
            format!("{label} explicit empty"),
            outcome(compiled(Some(json!({"allow": []})))),
            empty_refusal(label, "office", "claude", "opus"),
        ));
        rows.push((
            format!("{label} widening"),
            outcome(compiled(Some(json!({"allow": ["make"]})))),
            widening(
                label,
                "office",
                "allow",
                "names 'make', which the office's 'tools.allow' [\"cargo\", \"git\"] does not; \
                 a site subtracts from its office and never adds to it",
            ),
        ));
        rows.push((
            format!("{label} malformed"),
            outcome(compiled(Some(json!({"allow": [1]})))),
            format!("bundle: seat '{label}' 'tools' 'allow' must hold strings only"),
        ));
    }
    // An inherited body: the base layer declares the narrowing and the leaf
    // declares nothing, so the effective value comes from the layer that
    // wrote it — and so does a refusal, at the same owning site (review
    // return P2: an inherited explicit empty, malformed or widening
    // declaration refuses with the complete cause the leaf form gives).
    // These rows stand in the same table as the forms above, so a failure
    // in the table does not hide them (review return F4).
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
    std::fs::write(
        fixture.bundle().join("bundle.json"),
        serde_json::to_vec(&json!({"name": "fixture", "extends": "base"})).unwrap(),
    )
    .unwrap();
    let inherited = |tools: Value| {
        let mut base_config = fixture.config();
        base_config["name"] = json!("base");
        base_config["seats"]["work"] =
            json!({"results": ["complete"], "agent": "office", "tools": tools});
        std::fs::write(
            base.join("bundle.json"),
            serde_json::to_vec(&base_config).unwrap(),
        )
        .unwrap();
        Bundle::compile_with(&fixture.bundle(), &fixture.library(), &fixture.adapters())
    };
    let subset = inherited(json!({"allow": ["git"]}));
    rows.push((
        "inherited body, work".to_string(),
        match &subset {
            Ok(bundle) => format!("{:?}", bundle.sites["work"].local),
            Err(error) => error.to_string(),
        },
        format!("{:?}", Some(local(Some(&["git"]), None))),
    ));
    rows.push((
        "inherited body, review".to_string(),
        match &subset {
            Ok(bundle) => format!("{:?}", bundle.sites["review"].local),
            Err(error) => error.to_string(),
        },
        format!("{:?}", Some(LocalTools::unspecified())),
    ));
    // A composed bundle's refusal is the leaf form's complete cause, with
    // the chain it was composed from appended once (`Resolved::chain_note`).
    let composed = |cause: String| format!("bundle: {cause} (composed: fixture -> base)");
    rows.push((
        "inherited body, explicit empty".to_string(),
        outcome(inherited(json!({"allow": []}))),
        composed(empty_refusal("work", "office", "claude", "opus")),
    ));
    rows.push((
        "inherited body, widening".to_string(),
        outcome(inherited(json!({"allow": ["make"]}))),
        composed(widening(
            "work",
            "office",
            "allow",
            "names 'make', which the office's 'tools.allow' [\"cargo\", \"git\"] does not; a \
             site subtracts from its office and never adds to it",
        )),
    ));
    rows.push((
        "inherited body, malformed".to_string(),
        outcome(inherited(json!({"allow": [1]}))),
        composed("bundle: seat 'work' 'tools' 'allow' must hold strings only".to_string()),
    ));
    assert_eq!(rows.len(), 30);
    each_row(rows);
}

/// A site object with `tools` set to `value`, or with the key absent.
fn with_tools(mut site: Value, value: Option<Value>) -> Value {
    if let Some(tools) = value {
        site["tools"] = tools;
    }
    site
}

/// SCM "Each executable body owns its local declaration" and "Omitted and
/// empty local permissions are distinct" at the NESTED inline forms: an
/// inline panel member, sequence step, selected case and selected default
/// each record the checked unspecified value for omission and `{}`, and
/// each refuses a nonempty or malformed field with its own site named
/// (review return F5).
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn every_inline_executable_form_records_or_refuses_its_own_declaration() {
    let fixture = AgentFixture::new();
    fixture.write("adapters/codex.json", codex());
    // The command of the body under test; its siblings keep the opaque one.
    let command = std::cell::RefCell::new(json!(["driver"]));
    let inline = |tools: Option<Value>| {
        let command = match tools {
            Some(_) => command.borrow().clone(),
            None => json!(["driver"]),
        };
        with_tools(
            json!({"role": "roles/work.md", "driver": {"command": command}}),
            tools,
        )
    };
    type Form<'a> = (&'a str, Box<dyn Fn(Option<Value>) -> Value + 'a>, Value);
    let forms: Vec<Form<'_>> = vec![
        (
            "work:b",
            Box::new(|tools| {
                json!({"results": ["pass", "fail"], "aggregate": "unanimous-pass",
                       "panel": {"a": inline(None), "b": inline(tools)}})
            }),
            panel_policy(),
        ),
        (
            "work:second",
            Box::new(|tools| {
                let mut second = inline(tools);
                second["name"] = json!("second");
                json!({"results": ["complete"], "sequence": [
                    {"name": "first", "results": ["complete"], "role": "roles/work.md",
                     "driver": {"command": ["driver"]}},
                    second]})
            }),
            policy(),
        ),
        (
            "work:engine",
            Box::new(|tools| {
                json!({"results": ["complete"], "select": {"on": "strategy",
                    "cases": {"engine": inline(tools)}, "default": inline(None)}})
            }),
            policy(),
        ),
        (
            "work:default",
            Box::new(|tools| {
                json!({"results": ["complete"], "select": {"on": "strategy", "cases": {},
                    "default": inline(tools)}})
            }),
            policy(),
        ),
    ];
    let mut rows: Vec<Row<String>> = Vec::new();
    for (label, seat, table) in &forms {
        let compiled = |tools: Option<Value>| {
            let mut config = fixture.config();
            config["seats"]["work"] = seat(tools);
            fixture.compile_with_policy(config, table)
        };
        let recorded = |result: Result<Bundle, CompileError>| match result {
            Ok(bundle) => format!("{:?}", bundle.sites[*label].local),
            Err(error) => error.to_string(),
        };
        let checked = format!("{:?}", Some(LocalTools::unspecified()));
        rows.push((
            format!("{label} omission"),
            recorded(compiled(None)),
            checked.clone(),
        ));
        rows.push((
            format!("{label} {{}}"),
            recorded(compiled(Some(json!({})))),
            checked,
        ));
        rows.push((
            format!("{label} allow [cargo]"),
            outcome(compiled(Some(json!({"allow": ["cargo"]})))),
            undelivered_allow(label, "dispatches no built-in driver"),
        ));
        rows.push((
            format!("{label} allow []"),
            outcome(compiled(Some(json!({"allow": []})))),
            undelivered_allow(label, "dispatches no built-in driver"),
        ));
        rows.push((
            format!("{label} sandbox"),
            outcome(compiled(Some(json!({"sandbox": "workspace-write"})))),
            inline_sandbox_refusal(label),
        ));
        // Rebuild unit 5d lowers a class only at a seat, whose own class
        // rules it: a nested codex body keeps the refusal.
        *command.borrow_mut() = codex_inline(&[]);
        rows.push((
            format!("{label} codex sandbox"),
            outcome(compiled(Some(json!({"sandbox": "workspace-write"})))),
            inline_sandbox_refusal(label),
        ));
        // Rebuild unit 5b: the same body dispatching the claude driver has
        // its allow lowered, recorded at its own label and nowhere else.
        let claude = |result: Result<Bundle, CompileError>| match result {
            Ok(bundle) => format!(
                "{:?} {:?}",
                bundle.sites[*label].local, bundle.sites[*label].inline_local
            ),
            Err(error) => error.to_string(),
        };
        *command.borrow_mut() = claude_inline("claude", &[]);
        rows.push((
            format!("{label} claude allow [cargo]"),
            claude(compiled(Some(json!({"allow": ["cargo"]})))),
            format!(
                "{:?} {:?}",
                Some(local(Some(&["cargo"]), None)),
                cargo_lowering()
            ),
        ));
        *command.borrow_mut() = json!(["driver"]);
        rows.push((
            format!("{label} malformed"),
            outcome(compiled(Some(json!({"allow": [1]})))),
            format!("bundle: seat '{label}' 'tools' 'allow' must hold strings only"),
        ));
        rows.push((
            format!("{label} unknown key"),
            outcome(compiled(Some(json!({"invented": 1})))),
            format!(
                "bundle: seat '{label}' 'tools' has unknown key 'invented'; known keys: allow, \
                 sandbox, mcp"
            ),
        ));
    }
    assert_eq!(rows.len(), 36);
    each_row(rows);
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
    type Facts = (Option<LocalTools>, Vec<String>, Value, Value);
    let mut rows: Vec<Row<Facts>> = Vec::new();
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
            rows.push((
                format!("{label} with a={first:?} b={second:?}"),
                (
                    facts.local.clone(),
                    facts.chain[0].argv[8..].to_vec(),
                    facts.record.as_ref().unwrap()["agent_digest"].clone(),
                    facts.record.as_ref().unwrap()["agent"].clone(),
                ),
                (
                    Some(local(Some(&names), None)),
                    vec![
                        "--allowedTools".to_string(),
                        format!("Bash({}:*)", names[0]),
                    ],
                    json!(digest),
                    json!("office"),
                ),
            ));
        }
        // The container itself was never visited by the local pass.
        assert!(bundle
            .sites
            .get("work")
            .is_none_or(|facts| facts.local.is_none()));
    }
    each_row(rows);
}

/// The D5.3 admission table, row by row, under empty realm grants. Only
/// actual codex dispatch with hands and an engine fragment that expresses
/// exactly the requested class admits; every other shape refuses with the
/// site, link, class, boundary and complete cause, and every standing
/// refusal keeps its precedence. Admitted fixtures keep their holdings,
/// native OFF, hands and boundary unchanged.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
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
    /// The facts an admitted fixture keeps unchanged under empty grants:
    /// nothing held, the one known power not held with its OFF planned.
    fn unchanged(bundle: &Bundle) {
        let outcome = &bundle.sites["work"].capabilities.as_ref().unwrap().outcomes[0];
        assert!(outcome.held.is_empty());
        assert_eq!(outcome.not_held.keys().collect::<Vec<_>>(), ["web-search"]);
        assert_eq!(outcome.manifest()["native"]["inventory"], json!("known"));
        assert_eq!(outcome.manifest()["native"]["on"], json!([]));
        assert_eq!(outcome.manifest()["native"]["off"], json!(["web-search"]));
        assert_eq!(bundle.hands["work"], HandsSpec::default());
    }
    let mut rows: Vec<Row<String>> = Vec::new();

    // Row: boxed, read-only matches `hands.workspace`.
    declare("read-only");
    let boxed = fixture.compile(seat(None)).unwrap();
    assert_eq!(
        boxed.sites["work"].local,
        Some(local(Some(&["cargo"]), Some(Sandbox::ReadOnly)))
    );
    assert_eq!(boxed.sites["work"].chain[0].hands_fragment, CODEX_WORKSPACE);
    assert_eq!(boxed.boundary, Boundary::Namespace);
    unchanged(&boxed);
    for class in ["workspace-write", "danger-full-access"] {
        declare(class);
        rows.push((
            format!("boxed {class}"),
            outcome(fixture.compile(seat(None))),
            mismatch(class, "hands.workspace", "namespace", "read-only"),
        ));
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
    assert_eq!(gate.boundary, Boundary::Harness);
    unchanged(&gate);
    for class in ["workspace-write", "danger-full-access"] {
        declare(class);
        rows.push((
            format!("gate {class}"),
            outcome(fixture.compile_under(seat(Some("gate")), Boundary::Harness)),
            mismatch(class, "hands.harness.gate", "harness", "read-only"),
        ));
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
    assert!(work.sites["work"].chain[0].hands_fragment.is_empty());
    unchanged(&work);
    for class in ["read-only", "danger-full-access"] {
        declare(class);
        rows.push((
            format!("work {class}"),
            outcome(fixture.compile_under(seat(None), Boundary::Harness)),
            mismatch(class, "hands.harness.work", "harness", "workspace-write"),
        ));
    }

    // Row: open work has no fragment; an open gate keeps its standing refusal.
    declare("read-only");
    rows.push((
        "open work".to_string(),
        outcome(fixture.compile_under(seat(None), Boundary::Open)),
        "bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only' under the `open` \
         boundary, where a work seat runs at the harness's own default and no engine fragment \
         expresses a class; a presumed provider default is not a representation — refused \
         (design D5.3)"
            .to_string(),
    ));
    rows.push((
        "open gate keeps its standing refusal".to_string(),
        outcome(fixture.compile_under(seat(Some("gate")), Boundary::Open)),
        "bundle: seat 'work' is a gate with hands under the `open` boundary, where nothing at \
         all stands between a model's hands and the machine; `open` never holds a model gate \
         (decision 0046 ruling 4)"
            .to_string(),
    ));

    // A missing gate or work fragment keeps its standing refusal, before
    // any class; a DECLARED empty fragment reaches admission and expresses
    // nothing.
    let mut gateless = codex();
    gateless["hands"]["harness"] = json!({"work": CODEX_WORK, "result": "last-message"});
    fixture.write("adapters/codex.json", gateless);
    rows.push((
        "missing gate fragment keeps its standing refusal".to_string(),
        outcome(fixture.compile_under(seat(Some("gate")), Boundary::Harness)),
        "bundle: seat 'work' gate link 1 resolves to provider 'codex', which declares no \
         `hands.harness.gate` fragment; under the `harness` boundary a model may judge only \
         under its harness's own read-only sandbox as the adapter addresses it (decision 0046 \
         ruling 4)"
            .to_string(),
    ));
    let mut workless = codex();
    workless["hands"]["harness"] = json!({"gate": CODEX_GATE, "result": "last-message"});
    fixture.write("adapters/codex.json", workless);
    declare("workspace-write");
    rows.push((
        "missing work fragment keeps its standing refusal".to_string(),
        outcome(fixture.compile_under(seat(None), Boundary::Harness)),
        "bundle: seat 'work' link 1 resolves to provider 'codex', which declares no \
         `hands.harness.work` fragment: a capability gap — under the `harness` boundary a work \
         seat with hands writes the tree only under the harness's own writable sandbox as the \
         adapter addresses it (decision 0046 rulings 1 and 4)"
            .to_string(),
    ));
    let mut empty_gate = codex();
    empty_gate["hands"]["harness"]["gate"] = json!([]);
    fixture.write("adapters/codex.json", empty_gate);
    declare("read-only");
    rows.push((
        "declared empty gate fragment".to_string(),
        outcome(fixture.compile_under(seat(Some("gate")), Boundary::Harness)),
        "bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only' under the `harness` \
         boundary, but provider 'codex' supplies no `hands.harness.gate` fragment to express \
         it; a missing fragment is not a representation — refused (design D5.3)"
            .to_string(),
    ));

    // The judges list keeps its precedence over admission: the class
    // mismatches the gate fragment too, so an admission that ran first
    // would name the mismatch instead.
    let mut unjudged = codex();
    unjudged["judges"] = json!([]);
    fixture.write("adapters/codex.json", unjudged);
    declare("workspace-write");
    rows.push((
        "judges list keeps its precedence".to_string(),
        outcome(fixture.compile_under(seat(Some("gate")), Boundary::Harness)),
        "bundle: seat 'work' gate link 1 names model 'astra', which driver 'codex' does not \
         declare in 'judges' (decision 0041 ruling 3 — an absent declaration is empty)"
            .to_string(),
    ));
    declare("read-only");

    // A fragment naming no class, and one the grammar cannot read.
    let mut classless = codex();
    classless["hands"]["workspace"] = json!(["-c", "mcp_servers.brokkr.args={hands_args_toml}"]);
    fixture.write("adapters/codex.json", classless);
    rows.push((
        "classless fragment".to_string(),
        outcome(fixture.compile(seat(None))),
        "bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only', but the \
         `hands.workspace` fragment the engine selects for provider 'codex' under the \
         `namespace` boundary names no `--sandbox` class at all; a fragment that expresses \
         nothing is not a representation — refused (design D5.3)"
            .to_string(),
    ));
    let mut bogus = codex();
    bogus["hands"]["workspace"] = json!(["--sandbox", "read-only", "--bogus"]);
    fixture.write("adapters/codex.json", bogus);
    rows.push((
        "unreadable fragment".to_string(),
        outcome(fixture.compile(seat(None))),
        unreadable(1, "`hands.workspace` fragment", 3, "names no option"),
    ));
    // The same control through the configuration door.
    let mut configured = codex();
    configured["hands"]["workspace"] =
        json!(["--sandbox", "read-only", "-c", "sandbox_mode=\"read-only\""]);
    fixture.write("adapters/codex.json", configured);
    rows.push((
        "configuration door".to_string(),
        outcome(fixture.compile(seat(None))),
        "bundle: seat 'work' link 1 requests a typed 'tools.sandbox', but the `hands.workspace` \
         fragment assigns 'sandbox_mode' through the harness's configuration, a second door to \
         the same control that no typed class can be checked against — refused (design D5.3)"
            .to_string(),
    ));

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
    rows.push((
        "authored competing class".to_string(),
        outcome(fixture.compile(seat(None))),
        "bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only', but the authored \
         command of provider 'codex' already carries `--sandbox` 'danger-full-access', a \
         competing control the selected `hands.workspace` fragment would stand beside; authored \
         bytes cannot supply or contest a typed representation — refused under the `namespace` \
         boundary (design D5.3)"
            .to_string(),
    ));

    // A provider labelled codex that dispatches another harness.
    let mut shim = codex();
    shim["driver"] = json!(["{brokkr}", "driver", "codex-shim", "--"]);
    fixture.write("adapters/codex.json", shim);
    rows.push((
        "codex label, other harness".to_string(),
        outcome(fixture.compile(seat(None))),
        "bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only' but dispatches the \
         'codex-shim' harness through provider 'codex'; only the codex harness's own `--sandbox` \
         fragments express a sandbox class today, and a provider label, a permission mode or an \
         unmodelled driver is not evidence of one — refused under the `namespace` boundary \
         (design D5.3)"
            .to_string(),
    ));
    fixture.write("adapters/codex.json", codex());

    // Claude with hands: a permission mode is not a sandbox class.
    let mut claude = claude();
    claude["hands"] = json!({"workspace": [
        "--tools",
        "",
        "--strict-mcp-config",
        "--mcp-config",
        "{hands_mcp_json}"
    ]});
    fixture.write("adapters/claude.json", claude);
    fixture.write(
        "agents/boxed.json",
        boxed_agent(&["opus"], json!({"sandbox": "read-only"})),
    );
    rows.push((
        "claude with hands".to_string(),
        outcome(fixture.compile(seat(None))),
        "bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only' but dispatches the \
         'claude' harness through provider 'claude'; only the codex harness's own `--sandbox` \
         fragments express a sandbox class today, and a provider label, a permission mode or an \
         unmodelled driver is not evidence of one — refused under the `namespace` boundary \
         (design D5.3)"
            .to_string(),
    ));

    // A later candidate cannot hide behind an admitted primary.
    fixture.write(
        "agents/boxed.json",
        boxed_agent(&["astra", "opus"], json!({"sandbox": "read-only"})),
    );
    rows.push((
        "later candidate".to_string(),
        outcome(fixture.compile(seat(None))),
        "bundle: seat 'work' link 2 requests 'tools.sandbox' 'read-only' but dispatches the \
         'claude' harness through provider 'claude'; only the codex harness's own `--sandbox` \
         fragments express a sandbox class today, and a provider label, a permission mode or an \
         unmodelled driver is not evidence of one — refused under the `namespace` boundary \
         (design D5.3)"
            .to_string(),
    ));
    // D5.3's admission TABLE is independent of adapter data (review return
    // F1): a fragment that expresses the requested class exactly does not
    // admit a class the path does not hold. Each pair below matches the
    // fragment to the request and is refused by the table alone.
    let table = |class: &str, site_kind: &str, boundary: &str, admitted: &str, part: &str| {
        format!(
            "bundle: seat 'work' link 1 requests 'tools.sandbox' '{class}' at {site_kind} under \
             the `{boundary}` boundary, where decision 0065 slice one admits only '{admitted}' \
             (design D5.3: a box and a harness gate hold read-only, harness work holds \
             workspace-write); the `{part}` fragment of provider 'codex' expresses '{class}' \
             too, but adapter data is a representation and not an authority, so a fragment \
             cannot widen that table — refused"
        )
    };
    for class in ["workspace-write", "danger-full-access"] {
        let mut wide = codex();
        wide["hands"]["workspace"] = json!([
            "--sandbox",
            class,
            "-c",
            "mcp_servers.brokkr.args={hands_args_toml}"
        ]);
        wide["hands"]["harness"]["gate"] =
            json!(["--sandbox", class, "--output-last-message", "{result_path}"]);
        fixture.write("adapters/codex.json", wide);
        declare(class);
        rows.push((
            format!("boxed fragment {class}, request {class}"),
            outcome(fixture.compile(seat(None))),
            table(
                class,
                "a boxed site",
                "namespace",
                "read-only",
                "hands.workspace",
            ),
        ));
        rows.push((
            format!("gate fragment {class}, request {class}"),
            outcome(fixture.compile_under(seat(Some("gate")), Boundary::Harness)),
            table(
                class,
                "a harness gate",
                "harness",
                "read-only",
                "hands.harness.gate",
            ),
        ));
    }
    for class in ["read-only", "danger-full-access"] {
        let mut wide = codex();
        wide["hands"]["harness"]["work"] = json!(["--sandbox", class]);
        fixture.write("adapters/codex.json", wide);
        declare(class);
        rows.push((
            format!("work fragment {class}, request {class}"),
            outcome(fixture.compile_under(seat(None), Boundary::Harness)),
            table(
                class,
                "a harness work seat",
                "harness",
                "workspace-write",
                "hands.harness.work",
            ),
        ));
    }

    // An OPAQUE contribution (review return F2): a profile load, in either
    // spelling, and a configuration assignment outside the established
    // keys, in the selected fragment and in the authored command alike.
    let opaque = |part: &str, cause: &str| {
        format!(
            "bundle: seat 'work' link 1 requests a typed 'tools.sandbox', but the {part} {cause}"
        )
    };
    let profile = "carries `--profile`, which loads an opaque configuration document the engine \
                   cannot see into and that can set the same control, so no typed class can be \
                   checked against it — refused (design D5.3)";
    let unestablished = |at: usize| {
        format!(
            "assigns configuration at argument {at} outside the keys an existing fragment is \
             established to write (the hands transport under 'mcp_servers.brokkr' and the \
             effort 'model_reasoning_effort'); an unqualified assignment could reach the same \
             control, so no typed class can be checked against it — refused (design D5.3)"
        )
    };
    declare("read-only");
    let mut loaded = codex();
    loaded["hands"]["workspace"] = json!(["--sandbox", "read-only", "--profile=ci"]);
    fixture.write("adapters/codex.json", loaded);
    rows.push((
        "workspace fragment loads a profile".to_string(),
        outcome(fixture.compile(seat(None))),
        opaque("`hands.workspace` fragment", profile),
    ));
    let mut loaded = codex();
    loaded["driver"] = json!(["{brokkr}", "driver", "codex", "--", "-p", "ci"]);
    fixture.write("adapters/codex.json", loaded);
    rows.push((
        "authored command loads a profile".to_string(),
        outcome(fixture.compile(seat(None))),
        opaque("authored command", profile),
    ));
    let mut assigned = codex();
    assigned["hands"]["workspace"] = json!([
        "--sandbox",
        "read-only",
        "-c",
        "mcp_servers.brokkr.args={hands_args_toml}",
        "-c",
        "approval_policy=\"never\""
    ]);
    fixture.write("adapters/codex.json", assigned);
    rows.push((
        "workspace fragment assigns unestablished configuration".to_string(),
        outcome(fixture.compile(seat(None))),
        opaque("`hands.workspace` fragment", &unestablished(4)),
    ));
    let mut assigned = codex();
    assigned["driver"] = json!([
        "{brokkr}",
        "driver",
        "codex",
        "--",
        "-c",
        "approval_policy=\"never\""
    ]);
    fixture.write("adapters/codex.json", assigned);
    rows.push((
        "authored command assigns unestablished configuration".to_string(),
        outcome(fixture.compile(seat(None))),
        opaque("authored command", &unestablished(0)),
    ));
    // The control: the shipped transport and the effort assignment are
    // established, and a fragment carrying all of them admits exactly.
    let established = [
        "--sandbox",
        "read-only",
        "-c",
        "mcp_servers.brokkr.command=\"{brokkr}\"",
        "-c",
        "mcp_servers.brokkr.args={hands_args_toml}",
        "-c",
        "mcp_servers.brokkr.default_tools_approval_mode=\"approve\"",
        "-c",
        "model_reasoning_effort=\"high\"",
    ];
    let mut shipped = codex();
    shipped["hands"]["workspace"] = json!(established);
    fixture.write("adapters/codex.json", shipped);
    rows.push((
        "established transport and effort admit".to_string(),
        match fixture.compile(seat(None)) {
            Ok(bundle) => format!(
                "{:?} {:?}",
                bundle.sites["work"].local, bundle.sites["work"].chain[0].hands_fragment
            ),
            Err(error) => error.to_string(),
        },
        format!(
            "{:?} {:?}",
            Some(local(Some(&["cargo"]), Some(Sandbox::ReadOnly))),
            established
        ),
    ));
    fixture.write("adapters/codex.json", codex());

    // The other harnesses brokkr drives, and a dispatch it does not model,
    // each with hands in the box and a requested class (task 2.1.5;
    // review return F5): LaneTally, DSH, exec and a bare program. None
    // expresses a sandbox class, whatever its provider label says.
    let other = |harness: &str, provider: &str| {
        format!(
            "bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only' but dispatches the \
             '{harness}' harness through provider '{provider}'; only the codex harness's own \
             `--sandbox` fragments express a sandbox class today, and a provider label, a \
             permission mode or an unmodelled driver is not evidence of one — refused under the \
             `namespace` boundary (design D5.3)"
        )
    };
    let harness_adapter = |provider: &str, driver: Vec<&str>, model: &str, fragment: Value| {
        json!({
            "provider": provider,
            "binary": provider,
            "driver": driver,
            "models": {model: format!("{model}-concrete")},
            "model_flag": "--model",
            "efforts": ["high"],
            "effort_flag": "--effort",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
            "native_capabilities": claude_native(),
            "hands": {"workspace": fragment},
        })
    };
    let others = [
        (
            "lanetally",
            "lanetally",
            vec!["{brokkr}", "driver", "lanetally", "--"],
            "lane",
            json!(["--tools", "", "--mcp-config", "{hands_mcp_json}"]),
        ),
        (
            "dsh",
            "dsh",
            vec!["{brokkr}", "driver", "dsh", "--"],
            "flash",
            json!(["--tools", "", "--mcp-config", "{hands_mcp_json}"]),
        ),
        (
            "exec",
            "exec",
            vec!["{brokkr}", "driver", "exec", "--"],
            "script",
            json!([]),
        ),
        ("<custom>", "custom", vec!["codex"], "bare", json!([])),
    ];
    for (harness, provider, driver, model, fragment) in others {
        let mut adapter = harness_adapter(provider, driver, model, fragment);
        if harness == "dsh" {
            // The dsh grammar reads no tool list, so a selection cannot
            // load (rebuild unit 11); its inventory is unmeasured, as the
            // shipped dsh adapter's is.
            adapter["native_capabilities"] = json!({"unmeasured": "no inventory is measured"});
        }
        fixture.write(&format!("adapters/{provider}.json"), adapter);
        fixture.write(
            "agents/boxed.json",
            boxed_agent(&[model], json!({"sandbox": "read-only"})),
        );
        rows.push((
            format!("{provider} with hands"),
            outcome(fixture.compile(seat(None))),
            other(harness, provider),
        ));
        std::fs::remove_file(fixture.adapters().join(format!("{provider}.json"))).unwrap();
    }
    assert_eq!(rows.len(), 34);
    each_row(rows);

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

/// D5.3 as D5.5 makes it explicit: a matching `--sandbox` is insufficient
/// beside a control that lifts or replaces the sandbox. Each of
/// `--full-auto`, `--dangerously-bypass-approvals-and-sandbox`,
/// configuration under `sandbox_mode`, configuration under
/// `sandbox_workspace_write` and `--add-dir` in both its split and its
/// `=` spelling (review return S1: a filesystem root added beside the
/// class is a competing control on the same reach) is paired
/// independently with the authored command and with each selected engine
/// fragment (workspace, gate, work), under a MATCHING requested class so a
/// mismatch cannot hide the competing control. Every row refuses with the
/// full cause; the same fixtures without the control admit with their
/// exact facts.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn a_competing_control_beside_a_matching_sandbox_refuses_in_either_contribution() {
    let fixture = AgentFixture::new();
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
    let switch = |name: &str| {
        format!(
            "carries `{name}`, a switch that lifts or replaces the sandbox a `--sandbox` class \
             would express, so no typed class can be checked against it — refused (design D5.3)"
        )
    };
    let door = |table: &str| {
        format!(
            "assigns '{table}' through the harness's configuration, a second door to the same \
             control that no typed class can be checked against — refused (design D5.3)"
        )
    };
    let added = || {
        "carries `--add-dir`, which adds a filesystem root the `--sandbox` class would not \
         reach, a competing control on the same reach that no typed class can be checked \
         against — refused (design D5.3)"
            .to_string()
    };
    let controls: Vec<(&str, Vec<&str>, String)> = vec![
        ("--full-auto", vec!["--full-auto"], switch("--full-auto")),
        (
            "--dangerously-bypass-approvals-and-sandbox",
            vec!["--dangerously-bypass-approvals-and-sandbox"],
            switch("--dangerously-bypass-approvals-and-sandbox"),
        ),
        (
            "sandbox_mode",
            vec!["-c", "sandbox_mode=\"danger-full-access\""],
            door("sandbox_mode"),
        ),
        (
            "sandbox_workspace_write",
            vec!["-c", "sandbox_workspace_write.network_access=true"],
            door("sandbox_workspace_write"),
        ),
        // A root added to the sandbox, in the two spellings the codex
        // grammar accepts; the path is never echoed.
        ("--add-dir split", vec!["--add-dir", "/srv/shared"], added()),
        ("--add-dir equals", vec!["--add-dir=/srv/shared"], added()),
    ];
    let refusal = |part: &str, cause: &str| {
        format!(
            "bundle: seat 'work' link 1 requests a typed 'tools.sandbox', but the {part} {cause}"
        )
    };
    let mut rows: Vec<Row<String>> = Vec::new();
    for (label, tokens, cause) in &controls {
        // The authored command carries the control after brokkr's own
        // dispatch tokens; the selected fragment matches the class.
        let mut authored = codex();
        let mut driver = vec!["{brokkr}", "driver", "codex", "--"];
        driver.extend(tokens.iter().copied());
        authored["driver"] = json!(driver);
        fixture.write("adapters/codex.json", authored);
        declare("read-only");
        rows.push((
            format!("authored {label}"),
            outcome(fixture.compile(seat(None))),
            refusal("authored command", cause),
        ));
        // The selected fragment itself carries the control beside its
        // matching class: engine provenance is not an exemption.
        let mut with = |fragment: &[&str],
                        set: &dyn Fn(&mut Value, Value),
                        class: &str,
                        seat_class: Option<&str>,
                        boundary: Boundary,
                        part: &str| {
            let mut adapter = codex();
            let mut argv: Vec<&str> = fragment.to_vec();
            argv.extend(tokens.iter().copied());
            set(&mut adapter, json!(argv));
            fixture.write("adapters/codex.json", adapter);
            declare(class);
            rows.push((
                format!("{part} {label}"),
                outcome(fixture.compile_under(seat(seat_class), boundary)),
                refusal(part, cause),
            ));
        };
        with(
            &CODEX_WORKSPACE,
            &|adapter, argv| adapter["hands"]["workspace"] = argv,
            "read-only",
            None,
            Boundary::Namespace,
            "`hands.workspace` fragment",
        );
        with(
            &CODEX_GATE,
            &|adapter, argv| adapter["hands"]["harness"]["gate"] = argv,
            "read-only",
            Some("gate"),
            Boundary::Harness,
            "`hands.harness.gate` fragment",
        );
        with(
            &CODEX_WORK,
            &|adapter, argv| adapter["hands"]["harness"]["work"] = argv,
            "workspace-write",
            None,
            Boundary::Harness,
            "`hands.harness.work` fragment",
        );
    }
    assert_eq!(rows.len(), 24);
    each_row(rows);

    // The controls: the same three fragments without a competing control
    // admit the matching class with exact facts, and the supported hands
    // and effort configuration keep their established meaning.
    fixture.write("adapters/codex.json", codex());
    declare("read-only");
    let boxed = fixture.compile(seat(None)).unwrap();
    assert_eq!(
        boxed.sites["work"].local,
        Some(local(Some(&["cargo"]), Some(Sandbox::ReadOnly)))
    );
    // `{brokkr}` is expanded to the engine's own path at compile; the
    // dispatch and every token after it are exact.
    assert_eq!(boxed.sites["work"].chain[0].argv.len(), 16);
    assert_eq!(
        boxed.sites["work"].chain[0].argv[1..],
        [
            "driver",
            "codex",
            "--",
            "--model",
            "gpt-6-astra",
            "--effort",
            "high",
            "--sandbox",
            "read-only",
            "-c",
            "mcp_servers.brokkr.command=\"{brokkr}\"",
            "-c",
            "mcp_servers.brokkr.args={hands_args_toml}",
            "-c",
            "mcp_servers.brokkr.default_tools_approval_mode=\"approve\""
        ]
    );
    let gate = fixture
        .compile_under(seat(Some("gate")), Boundary::Harness)
        .unwrap();
    assert_eq!(
        gate.sites["work"].local,
        Some(local(Some(&["cargo"]), Some(Sandbox::ReadOnly)))
    );
    declare("workspace-write");
    let work = fixture
        .compile_under(seat(None), Boundary::Harness)
        .unwrap();
    assert_eq!(
        work.sites["work"].local,
        Some(local(Some(&["cargo"]), Some(Sandbox::WorkspaceWrite)))
    );
}

// ------------------------------- unit 2-fix: root selectors and native argv

/// The four spellings the codex grammar reads as its one root selector,
/// canonical `--cd` (A1): split, `=`, the short alias split and attached.
const ROOT_SPELLINGS: [(&str, &[&str]); 4] = [
    ("--cd /", &["--cd", "/"]),
    ("--cd=/", &["--cd=/"]),
    ("-C /", &["-C", "/"]),
    ("-C/", &["-C/"]),
];

/// The contribution a site's resolved native plan is judged as.
const NATIVE: &str = "resolved native control argv";

/// The complete root-selector refusal at `work` for one contribution,
/// written here rather than derived: it names canonical `--cd` and never
/// the path it selects.
fn root_refusal(link: usize, part: &str) -> String {
    format!(
        "bundle: seat 'work' link {link} requests a typed 'tools.sandbox', but the {part} \
         carries `--cd`, which selects the root the `--sandbox` class is measured from, a \
         competing root control that no typed class can be checked against whatever its value \
         or position — refused (design D5.3)"
    )
}

/// The complete refusal of a competing control at `work`, link 1.
fn competing(part: &str, cause: &str) -> String {
    format!("bundle: seat 'work' link 1 requests a typed 'tools.sandbox', but the {part} {cause}")
}

/// The cause of a codex switch that lifts or replaces the sandbox.
fn switch_cause(name: &str) -> String {
    format!(
        "carries `{name}`, a switch that lifts or replaces the sandbox a `--sandbox` class would \
         express, so no typed class can be checked against it — refused (design D5.3)"
    )
}

/// The cause of an added filesystem root; the path is never echoed.
const ADDED_ROOT_CAUSE: &str = "carries `--add-dir`, which adds a filesystem root the \
    `--sandbox` class would not reach, a competing control on the same reach that no typed \
    class can be checked against — refused (design D5.3)";

/// The cause of a profile load, whose document the engine cannot see into.
const LOAD_CAUSE: &str = "carries `--profile`, which loads an opaque configuration document \
    the engine cannot see into and that can set the same control, so no typed class can be \
    checked against it — refused (design D5.3)";

/// The complete refusal of a contribution the codex grammar cannot place,
/// at `work`: the argument by position and the grammar's fixed cause, and
/// never the token, which can carry a value (unit 2-fix review return S2).
fn unreadable(link: usize, part: &str, argument: usize, cause: &str) -> String {
    format!(
        "bundle: seat 'work' link {link} requests a typed 'tools.sandbox', but the {part} it \
         would be judged against cannot be read: the 'codex' command grammar cannot place \
         argument {argument}, whose token is not echoed because it can carry a value: it \
         {cause} — refused (design D5.3)"
    )
}

/// The grammar's fixed cause for a second `--cd`.
const REPEATED_ROOT: &str = "repeats option '--cd', which the grammar admits once; a CLI that \
    resolves a duplicate last-wins would resolve it against the control the engine composed";

/// The grammar's fixed cause for a `--cd` whose split value reads as an
/// option.
const ROOT_VALUE_READS_AS_OPTION: &str = "stands where the value of '--cd' belongs but reads as \
    an option, so which of the two it is cannot be told";

/// The load refusal of a declared native OFF whose second assignment the
/// bounded configuration reader refuses (rebuild unit 11, the second
/// review's F1): no such declaration reaches a compile's D5.3 judgment.
fn off_config_refusal(fixture: &AgentFixture, cause: &str) -> String {
    format!(
        "bundle: adapter 'codex' ({}) 'native_capabilities' key 'web-search' OFF argv cannot be \
         composed: '--config' value 1 {cause}; the adapter data is where a driver's model mapping \
         (decision 0016) and its trust tier and binding grant (decision 0021) are declared, and \
         this bundle names an agent, seats a gate, declares a secret binding or declares typed \
         tools",
        fixture.adapters().join("codex.json").display()
    )
}

/// The bounded reader's cause for an assignment into a capability table
/// off the allowlist.
fn off_allowlist(table: &str) -> String {
    format!(
        "assigns into the '{table}' configuration, which is outside the closed set of keys an \
         inline Codex launch admits"
    )
}

/// The complete refusal of any `--sandbox` in a resolved native plan.
fn native_sandbox_refusal(link: usize, class: &str) -> String {
    format!(
        "bundle: seat 'work' link {link} requests 'tools.sandbox' '{class}', but the resolved \
         native control argv of provider 'codex' carries `--sandbox`, a second sandbox control \
         beside the selected hands fragment; only that fragment represents a typed class, so \
         even a matching native class competes — refused (design D5.3)"
    )
}

/// The `work` seat on agent `boxed`, of `class` where one is written.
fn sandbox_seat(fixture: &AgentFixture, class: Option<&str>) -> Value {
    let mut config = fixture.config();
    config["seats"]["work"] = json!({"results": ["complete"], "agent": "boxed"});
    if let Some(class) = class {
        config["seats"]["work"]["class"] = json!(class);
    }
    config
}

/// Agent `boxed` on astra, allowing `cargo` and requesting `sandbox`.
fn declare_sandbox(fixture: &AgentFixture, sandbox: &str) {
    fixture.write(
        "agents/boxed.json",
        boxed_agent(&["astra"], json!({"allow": ["cargo"], "sandbox": sandbox})),
    );
}

/// The fixture codex whose web-search OFF is the measured denial followed
/// by `extra`: the legitimate control first, so a scan that stopped at a
/// valid denial would never reach what follows it.
fn codex_off(extra: &[&str]) -> Value {
    let mut adapter = codex();
    let mut off = vec!["-c", "web_search=\"disabled\""];
    off.extend(extra);
    adapter["native_capabilities"]["known"]["web-search"]["off"] = json!({"argv": off});
    adapter
}

/// The full resolved native argv of one link of `work`.
fn native_argv(bundle: &Bundle, link: usize) -> Value {
    bundle.sites["work"].capabilities.as_ref().unwrap().outcomes[link].controls()["argv"].clone()
}

/// A long value of non-ASCII scalars with a newline inside it, within the
/// grammar's input (it sets no length limit): the refusal is the same
/// value-free sentence as for `/`.
fn long_payload() -> String {
    format!("/{}\n{}", "ü".repeat(300), "ж".repeat(300))
}

/// Unit 2-fix A1: canonical `--cd` in every spelling, beside a matching
/// workspace-write class at a harness work seat, refuses in the authored
/// command and in the `hands.harness.work` fragment — for any value, so
/// the current workspace too, and whichever occurrence a harness would
/// honour. The refusal names `--cd` and never the path. A long non-ASCII
/// value with a newline yields the identical sentence; an open gate keeps
/// its standing refusal first; the same fixture without the selector
/// admits with its exact facts.
#[test]
fn a_root_selector_beside_a_matching_sandbox_refuses_in_the_authored_command_and_the_fragment() {
    let fixture = AgentFixture::new();
    declare_sandbox(&fixture, "workspace-write");
    let work = || fixture.compile_under(sandbox_seat(&fixture, None), Boundary::Harness);
    let authored = |tokens: &[&str]| {
        let mut adapter = codex();
        let mut driver = vec!["{brokkr}", "driver", "codex", "--"];
        driver.extend(tokens);
        adapter["driver"] = json!(driver);
        fixture.write("adapters/codex.json", adapter);
    };
    let fragment = |tokens: &[&str]| {
        let mut adapter = codex();
        let mut argv = CODEX_WORK.to_vec();
        argv.extend(tokens);
        adapter["hands"]["harness"]["work"] = json!(argv);
        fixture.write("adapters/codex.json", adapter);
    };
    let mut rows: Vec<Row<String>> = Vec::new();
    for (label, tokens) in ROOT_SPELLINGS {
        authored(tokens);
        rows.push((
            format!("A1 authored {label}"),
            outcome(work()),
            root_refusal(1, "authored command"),
        ));
        fragment(tokens);
        rows.push((
            format!("A1 hands.harness.work {label}"),
            outcome(work()),
            root_refusal(1, "`hands.harness.work` fragment"),
        ));
    }
    // The current workspace is a root selector as much as `/` is.
    authored(&["--cd", "."]);
    rows.push((
        "authored --cd .".to_string(),
        outcome(work()),
        root_refusal(1, "authored command"),
    ));
    // A long non-ASCII value with a newline: the identical sentence.
    let long = long_payload();
    authored(&[&format!("--cd={long}")]);
    rows.push((
        "authored --cd=<long>".to_string(),
        outcome(work()),
        root_refusal(1, "authored command"),
    ));
    fragment(&[&format!("-C{long}")]);
    rows.push((
        "hands.harness.work -C<long>".to_string(),
        outcome(work()),
        root_refusal(1, "`hands.harness.work` fragment"),
    ));
    // An open gate's standing refusal precedes the root selector.
    authored(&["--cd", "/"]);
    declare_sandbox(&fixture, "read-only");
    rows.push((
        "open gate keeps its standing refusal".to_string(),
        outcome(fixture.compile_under(sandbox_seat(&fixture, Some("gate")), Boundary::Open)),
        "bundle: seat 'work' is a gate with hands under the `open` boundary, where nothing at \
         all stands between a model's hands and the machine; `open` never holds a model gate \
         (decision 0046 ruling 4)"
            .to_string(),
    ));
    assert_eq!(rows.len(), 12);
    each_row(rows);

    // The control: without a selector the same seat admits, exactly.
    fixture.write("adapters/codex.json", codex());
    declare_sandbox(&fixture, "workspace-write");
    let admitted = work().unwrap();
    assert_eq!(
        admitted.sites["work"].local,
        Some(local(Some(&["cargo"]), Some(Sandbox::WorkspaceWrite)))
    );
    assert_eq!(
        admitted.sites["work"].chain[0].harness.work.as_deref(),
        Some(CODEX_WORK.map(String::from).as_slice())
    );
    assert_eq!(
        admitted.sites["work"].chain[0].argv[1..],
        [
            "driver",
            "codex",
            "--",
            "--model",
            "gpt-6-astra",
            "--effort",
            "high"
        ]
    );
}

/// Unit 2-fix S1: the RESOLVED native plan is judged too, after resolution
/// and before its facts are published. Beside the legitimate web-search
/// denial, the chief's three cases (S1.1 an added root at harness work,
/// S1.2 the sandbox bypass at a harness gate, S1.3 a workspace-write
/// network assignment at harness work), every root-selector spelling, the
/// other competing switches, tables, an opaque load, an unqualified or
/// descendant assignment and any native `--sandbox` — even the matching
/// class — refuse with the complete bounded cause, under the matching
/// requested class so no mismatch can hide them. A competing control
/// BEFORE the denial refuses as well as one after it. Since rebuild unit
/// 11 (the second review's F1), an assignment the bounded configuration
/// reader refuses — a sandbox or feature table, a descendant key — is
/// refused where the adapter loads.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn a_resolved_native_off_contribution_cannot_compete_with_a_matching_sandbox() {
    let fixture = AgentFixture::new();
    let long = format!("--add-dir={}", long_payload());
    /// A label, the OFF argv after the denial, the seat class, the
    /// requested class and the expected outcome.
    type Case<'a> = (String, Vec<&'a str>, Option<&'a str>, &'a str, String);
    let cases: Vec<Case<'_>> = vec![
        (
            "S1.1 --add-dir=/srv/shared at harness work".to_string(),
            vec!["--add-dir=/srv/shared"],
            None,
            "workspace-write",
            competing(NATIVE, ADDED_ROOT_CAUSE),
        ),
        (
            "S1.2 bypass at a harness gate".to_string(),
            vec!["--dangerously-bypass-approvals-and-sandbox"],
            Some("gate"),
            "read-only",
            competing(
                NATIVE,
                &switch_cause("--dangerously-bypass-approvals-and-sandbox"),
            ),
        ),
        (
            "S1.3 sandbox_workspace_write.network_access at harness work".to_string(),
            vec!["-c", "sandbox_workspace_write.network_access=true"],
            None,
            "workspace-write",
            off_config_refusal(&fixture, &off_allowlist("sandbox_workspace_write")),
        ),
        (
            "--full-auto at harness work".to_string(),
            vec!["--full-auto"],
            None,
            "workspace-write",
            competing(NATIVE, &switch_cause("--full-auto")),
        ),
        (
            "sandbox_mode at a harness gate".to_string(),
            vec!["-c", "sandbox_mode=\"danger-full-access\""],
            Some("gate"),
            "read-only",
            off_config_refusal(&fixture, &off_allowlist("sandbox_mode")),
        ),
        (
            "opaque profile load at harness work".to_string(),
            vec!["--profile", "ci"],
            None,
            "workspace-write",
            competing(NATIVE, LOAD_CAUSE),
        ),
        (
            "--add-dir split at a harness gate".to_string(),
            vec!["--add-dir", "/srv/shared"],
            Some("gate"),
            "read-only",
            competing(NATIVE, ADDED_ROOT_CAUSE),
        ),
        (
            "--add-dir=<long> at harness work".to_string(),
            vec![long.as_str()],
            None,
            "workspace-write",
            competing(NATIVE, ADDED_ROOT_CAUSE),
        ),
        (
            "unqualified config at harness work".to_string(),
            vec!["-c", "features.web_search_request=true"],
            None,
            "workspace-write",
            off_config_refusal(&fixture, &off_allowlist("features")),
        ),
        (
            "web_search descendant at harness work".to_string(),
            vec!["-c", "web_search.mode=\"live\""],
            None,
            "workspace-write",
            off_config_refusal(&fixture, &off_allowlist("web_search")),
        ),
        (
            "matching --sandbox at harness work".to_string(),
            vec!["--sandbox", "workspace-write"],
            None,
            "workspace-write",
            native_sandbox_refusal(1, "workspace-write"),
        ),
        (
            "matching --sandbox at a harness gate".to_string(),
            vec!["-s", "read-only"],
            Some("gate"),
            "read-only",
            native_sandbox_refusal(1, "read-only"),
        ),
    ];
    let mut rows: Vec<Row<String>> = Vec::new();
    for (label, extra, seat_class, class, expected) in cases {
        fixture.write("adapters/codex.json", codex_off(&extra));
        declare_sandbox(&fixture, class);
        rows.push((
            label,
            outcome(fixture.compile_under(sandbox_seat(&fixture, seat_class), Boundary::Harness)),
            expected,
        ));
    }
    for (label, tokens) in ROOT_SPELLINGS {
        fixture.write("adapters/codex.json", codex_off(tokens));
        declare_sandbox(&fixture, "workspace-write");
        rows.push((
            format!("native OFF {label}"),
            outcome(fixture.compile_under(sandbox_seat(&fixture, None), Boundary::Harness)),
            root_refusal(1, NATIVE),
        ));
    }
    // A competing control BEFORE the legitimate denial.
    let mut adapter = codex();
    adapter["native_capabilities"]["known"]["web-search"]["off"] =
        json!({"argv": ["--add-dir=/srv/shared", "-c", "web_search=\"disabled\""]});
    fixture.write("adapters/codex.json", adapter);
    declare_sandbox(&fixture, "workspace-write");
    rows.push((
        "--add-dir before the denial".to_string(),
        outcome(fixture.compile_under(sandbox_seat(&fixture, None), Boundary::Harness)),
        competing(NATIVE, ADDED_ROOT_CAUSE),
    ));
    assert_eq!(rows.len(), 17);
    each_row(rows);
}

/// Unit 2-fix review return S2 and SC1: a contribution the codex grammar
/// cannot place — a duplicate root selector, a root selector whose split
/// value reads as an option, a bare word, a trailing option without its
/// value — refuses by argument position and the grammar's fixed cause, in
/// the authored command, the `hands.harness.work` fragment and the
/// resolved native OFF argv alike. The unplaceable token is an attached
/// `-C` carrying a long non-ASCII value with a newline, and it is never
/// echoed: every compile refusal stays within 512 Unicode scalars. Since
/// rebuild unit 11, a native OFF the grammar cannot place is refused where
/// the adapter loads, by the same position and cause and a bounded label.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn an_unreadable_contribution_refuses_by_position_without_echoing_its_token() {
    let fixture = AgentFixture::new();
    declare_sandbox(&fixture, "workspace-write");
    let attached = format!("-C{}", long_payload());
    let duplicate = ["--cd=/", attached.as_str()];
    let malformed = ["--cd", attached.as_str()];
    let work = || outcome(fixture.compile_under(sandbox_seat(&fixture, None), Boundary::Harness));
    let authored = |tokens: &[&str]| {
        let mut adapter = codex();
        let mut driver = vec!["{brokkr}", "driver", "codex", "--"];
        driver.extend(tokens);
        adapter["driver"] = json!(driver);
        fixture.write("adapters/codex.json", adapter);
    };
    let fragment = |tokens: &[&str]| {
        let mut adapter = codex();
        let mut argv = CODEX_WORK.to_vec();
        argv.extend(tokens);
        adapter["hands"]["harness"]["work"] = json!(argv);
        fixture.write("adapters/codex.json", adapter);
    };
    let native = |tokens: &[&str]| fixture.write("adapters/codex.json", codex_off(tokens));
    // Rebuild unit 11 parses each declared half where the adapter loads, so
    // a native OFF the grammar cannot place meets the load's refusal, by
    // position and a bounded label.
    let unloadable = |argument: usize, label: &str, cause: &str| {
        format!(
            "bundle: adapter 'codex' ({}) 'native_capabilities' key 'web-search' OFF argv cannot \
             be composed: the 'codex' command grammar cannot place argument {argument} \
             ({label}): it {cause}. A harness brokkr launches is parsed against a model of its \
             options, and a token that grammar cannot place is refused rather than passed \
             through, because a control nobody can read is a control nobody can rule on \
             (decision 0066 ruling 6); the adapter data is where a driver's model mapping \
             (decision 0016) and its trust tier and binding grant (decision 0021) are declared, \
             and this bundle names an agent, seats a gate, declares a secret binding or declares \
             typed tools",
            fixture.adapters().join("codex.json").display()
        )
    };
    let mut rows: Vec<Row<String>> = Vec::new();
    authored(&duplicate);
    rows.push((
        "authored duplicate --cd".to_string(),
        work(),
        unreadable(1, "authored command", 2, REPEATED_ROOT),
    ));
    authored(&malformed);
    rows.push((
        "authored malformed --cd".to_string(),
        work(),
        unreadable(1, "authored command", 2, ROOT_VALUE_READS_AS_OPTION),
    ));
    fragment(&duplicate);
    rows.push((
        "hands.harness.work duplicate --cd".to_string(),
        work(),
        unreadable(1, "`hands.harness.work` fragment", 4, REPEATED_ROOT),
    ));
    fragment(&malformed);
    rows.push((
        "hands.harness.work malformed --cd".to_string(),
        work(),
        unreadable(
            1,
            "`hands.harness.work` fragment",
            4,
            ROOT_VALUE_READS_AS_OPTION,
        ),
    ));
    native(&duplicate);
    rows.push((
        "native OFF duplicate --cd".to_string(),
        work(),
        unloadable(4, "'--cd'", REPEATED_ROOT),
    ));
    native(&malformed);
    rows.push((
        "native OFF malformed --cd".to_string(),
        work(),
        unloadable(4, "'--cd'", ROOT_VALUE_READS_AS_OPTION),
    ));
    native(&["stray"]);
    rows.push((
        "native OFF bare word".to_string(),
        work(),
        unloadable(
            3,
            "a positional argument, whose text is not echoed",
            "is a bare word, and no positional argument is part of the supported shape",
        ),
    ));
    native(&["--profile"]);
    rows.push((
        "native OFF trailing --profile".to_string(),
        work(),
        unloadable(
            3,
            "'--profile'",
            "takes a value and is the last argument, so it has none",
        ),
    ));
    assert_eq!(rows.len(), 8);
    // The bound is the engine's own refusal's; a load refusal also names
    // the adapter's path, and its exact text above echoes no token.
    for (label, _, expected) in &rows[..4] {
        assert!(
            expected.chars().count() <= 512,
            "row {label}: the expected refusal is {} scalars",
            expected.chars().count()
        );
    }
    each_row(rows);
}

/// Unit 2-fix, the valid denial (SCM "Valid native denial preserves
/// matching typed sandboxes"; NCR): under empty realm grants, a harness
/// gate requesting read-only and a harness work seat requesting
/// workspace-write compile beside the exact denial `-c`,
/// `web_search="disabled"`, with the exact class, the selected fragment,
/// nothing held, web-search OFF and the full resolved denial argv. The
/// allowance is the resolved plan's alone: the same assignment written in
/// the authored command or the selected fragment is unqualified there.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn a_valid_native_denial_keeps_a_matching_sandbox_admitted_and_only_there() {
    let fixture = AgentFixture::new();
    fixture.write("adapters/codex.json", codex());
    let denial = json!(["-c", "web_search=\"disabled\""]);
    let check = |bundle: &Bundle, class: Sandbox, fragment: &[&str]| {
        assert_eq!(
            bundle.sites["work"].local,
            Some(local(Some(&["cargo"]), Some(class)))
        );
        let chain = &bundle.sites["work"].chain[0];
        let selected = match class {
            Sandbox::ReadOnly => chain.harness.gate.as_deref(),
            _ => chain.harness.work.as_deref(),
        };
        assert_eq!(
            selected,
            Some(
                fragment
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .as_slice()
            )
        );
        assert!(chain.hands_fragment.is_empty());
        assert_eq!(bundle.boundary, Boundary::Harness);
        let outcome = &bundle.sites["work"].capabilities.as_ref().unwrap().outcomes[0];
        assert!(outcome.held.is_empty());
        assert_eq!(outcome.not_held.keys().collect::<Vec<_>>(), ["web-search"]);
        assert_eq!(outcome.manifest()["native"]["on"], json!([]));
        assert_eq!(outcome.manifest()["native"]["off"], json!(["web-search"]));
        assert_eq!(native_argv(bundle, 0), denial);
        assert_eq!(bundle.hands["work"], HandsSpec::default());
    };
    let compiled = |class: Sandbox| {
        format!(
            "compiled: {:?}",
            [
                ("review".to_string(), Some(LocalTools::unspecified())),
                (
                    "work".to_string(),
                    Some(local(Some(&["cargo"]), Some(class)))
                )
            ]
        )
    };
    let mut rows: Vec<Row<String>> = Vec::new();
    declare_sandbox(&fixture, "read-only");
    rows.push((
        "gate read-only beside the denial".to_string(),
        outcome(fixture.compile_under(sandbox_seat(&fixture, Some("gate")), Boundary::Harness)),
        compiled(Sandbox::ReadOnly),
    ));
    declare_sandbox(&fixture, "workspace-write");
    rows.push((
        "work workspace-write beside the denial".to_string(),
        outcome(fixture.compile_under(sandbox_seat(&fixture, None), Boundary::Harness)),
        compiled(Sandbox::WorkspaceWrite),
    ));
    // The allowance does not reach authored or fragment bytes.
    let mut adapter = codex();
    adapter["driver"] = json!([
        "{brokkr}",
        "driver",
        "codex",
        "--",
        "-c",
        "web_search=\"disabled\""
    ]);
    fixture.write("adapters/codex.json", adapter);
    rows.push((
        "authored web_search".to_string(),
        outcome(fixture.compile_under(sandbox_seat(&fixture, None), Boundary::Harness)),
        competing(
            "authored command",
            "assigns configuration at argument 0 outside the keys an existing fragment is \
             established to write (the hands transport under 'mcp_servers.brokkr' and the \
             effort 'model_reasoning_effort'); an unqualified assignment could reach the same \
             control, so no typed class can be checked against it — refused (design D5.3)",
        ),
    ));
    let mut adapter = codex();
    adapter["hands"]["harness"]["work"] = json!([
        "--sandbox",
        "workspace-write",
        "-c",
        "web_search=\"disabled\""
    ]);
    fixture.write("adapters/codex.json", adapter);
    rows.push((
        "hands.harness.work web_search".to_string(),
        outcome(fixture.compile_under(sandbox_seat(&fixture, None), Boundary::Harness)),
        competing(
            "`hands.harness.work` fragment",
            "assigns configuration at argument 2 outside the keys an existing fragment is \
             established to write (the hands transport under 'mcp_servers.brokkr' and the \
             effort 'model_reasoning_effort'); an unqualified assignment could reach the same \
             control, so no typed class can be checked against it — refused (design D5.3)",
        ),
    ));
    each_row(rows);

    // The full admitted facts of the two positives.
    fixture.write("adapters/codex.json", codex());
    declare_sandbox(&fixture, "read-only");
    let gate = fixture
        .compile_under(sandbox_seat(&fixture, Some("gate")), Boundary::Harness)
        .unwrap();
    check(&gate, Sandbox::ReadOnly, &CODEX_GATE);
    declare_sandbox(&fixture, "workspace-write");
    let work = fixture
        .compile_under(sandbox_seat(&fixture, None), Boundary::Harness)
        .unwrap();
    check(&work, Sandbox::WorkspaceWrite, &CODEX_WORK);
}

/// A realm context granting `web-search` to office `boxed` through the
/// codex provider-native dialect `codex-search`, under `restrictions`
/// where they are given: a real temporary grant and definition, beside
/// the fixture's library, read the way the operator's map is read.
fn grant_web_search(
    fixture: &AgentFixture,
    restrictions: Option<Value>,
) -> crate::capabilities::CapabilityContext {
    define(fixture, "web-search", json!(["reads", "egress"]));
    std::fs::create_dir_all(fixture.root.join("dialects/tools")).unwrap();
    let mut dialect = json!({
        "schema": "brokkr.tool-dialect/v1", "name": "codex-search", "serves": "web-search",
        "kind": "provider-native", "provider": "codex", "adapter_key": "web-search",
        "tools": ["web_search"],
        "sends": {"description": "a query the model composes", "seat_composed": true}
    });
    let mut grant = json!({"dialect": "codex-search", "offices": ["boxed"]});
    if let Some(restrictions) = restrictions {
        dialect["restrictions"] = json!({
            "type": "object", "additionalProperties": false,
            "properties": {"allow": {"type": "object", "additionalProperties": false,
                "properties": {"hosts": {"type": "array", "items": {"type": "string"}}}}}
        });
        grant["allow"] = restrictions;
    }
    fixture.write("dialects/tools/codex-search.json", dialect);
    let map = json!({"schema": "forge.realms/v6", "journal": "forge.db", "realms": [
        {"name": "private", "path": "repo", "default_branch": "main",
         "capabilities": {"web-search": grant}}]});
    let (map, _) = brokkr_core::realms::RealmMap::of("realms.json", map).unwrap();
    crate::capabilities::CapabilityContext {
        realm: "private".into(),
        grants: map.realms[0].grants.clone(),
        root: fixture.root.clone(),
    }
}

/// Unit 2-fix, the other two resolved contributions: a real realm grant
/// holds web-search, so the ON argv is selected. Each root-selector
/// spelling in the ON argv refuses with the same bounded cause; the clean
/// ON plan admits the matching class with the holding and the exact
/// resolved argv. A validated nonempty restriction over a declared
/// transport, clean or carrying a root selector, is refused with the
/// deferral reason before any transport is composed (rebuild unit 11; the
/// operator's ruling of 2026-09-25). Synthetic transport here qualifies no
/// provider's restriction support (unit 9).
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn resolved_native_on_and_restriction_contributions_obey_the_same_refusals() {
    let fixture = AgentFixture::declaring();
    fixture.write(
        "agents/boxed.json",
        json!({
            "description": "a boxed agent",
            "charter": "charters/data.md",
            "models": ["astra"],
            "efforts": {"astra": "high"},
            "hands": "workspace",
            "tools": {"allow": ["cargo"], "sandbox": "workspace-write"},
            "capabilities": {"web-search": "requires"},
        }),
    );
    let on = |argv: &[&str], restrictions: Option<&[&str]>| {
        let mut adapter = codex();
        let known = &mut adapter["native_capabilities"]["known"]["web-search"];
        known["on"] = json!({"argv": argv});
        if let Some(template) = restrictions {
            known["restrictions"] = json!({"argv": template});
        }
        fixture.write("adapters/codex.json", adapter);
    };
    let compile = |context: &crate::capabilities::CapabilityContext| {
        fixture.stage(&sandbox_seat(&fixture, None), &policy());
        Bundle::compile_with_capabilities(
            &fixture.bundle(),
            &fixture.library(),
            &fixture.adapters(),
            Some("private"),
            None,
            Boundary::Harness,
            context,
        )
    };
    let plain = grant_web_search(&fixture, None);
    let hosts = grant_web_search(&fixture, Some(json!({"hosts": ["example.org"]})));
    // Rebuild unit 11 (the second review's F1) reads a declared value at
    // load with the bounded reader, and codex has no measured ON value or
    // restriction transport: the ON writes the admitted effort key, and
    // the synthetic transport places its slot in inert data.
    let live = ["-c", "model_reasoning_effort=\"high\""];
    let transport = ["--image", "{restrictions_json}"];
    // Slice one carries only the empty restriction (operator ruling of
    // 2026-09-25, "defer"; design D11): a held nonempty one is refused
    // before any transport is composed, whatever the template carries.
    let deferred = "bundle: seat 'work' (office 'boxed') in realm 'private': requires \
                    capability 'web-search' through dialect 'codex-search', but provider \
                    'codex' cannot express restriction 'allow.hosts' through its declared \
                    transport, which carries only the empty restriction until a provider \
                    restriction transport is measured (operator ruling of 2026-09-25); the \
                    capability cannot be held under this grant";
    let mut rows: Vec<Row<String>> = Vec::new();
    for (label, tokens) in ROOT_SPELLINGS {
        let mut argv = live.to_vec();
        argv.extend(tokens);
        on(&argv, None);
        rows.push((
            format!("native ON {label}"),
            outcome(compile(&plain)),
            root_refusal(1, NATIVE),
        ));
        let mut template = tokens.to_vec();
        template.extend(transport);
        on(&live, Some(&template));
        rows.push((
            format!("native restriction {label}"),
            outcome(compile(&hosts)),
            deferred.to_string(),
        ));
    }
    // The ON argv faces the other competing controls as well.
    on(
        &[
            "-c",
            "model_reasoning_effort=\"high\"",
            "--add-dir=/srv/shared",
        ],
        None,
    );
    rows.push((
        "native ON --add-dir".to_string(),
        outcome(compile(&plain)),
        competing(NATIVE, ADDED_ROOT_CAUSE),
    ));
    // The clean ON plan admits: the exact key the ON writes is the one a
    // resolved native plan is established to write. The clean restriction
    // plan is deferred with the transport.
    let admitted = format!(
        "compiled: {:?}",
        [
            ("review".to_string(), Some(LocalTools::unspecified())),
            (
                "work".to_string(),
                Some(local(Some(&["cargo"]), Some(Sandbox::WorkspaceWrite)))
            )
        ]
    );
    on(&live, None);
    rows.push((
        "clean native ON".to_string(),
        outcome(compile(&plain)),
        admitted,
    ));
    on(&live, Some(&transport));
    rows.push((
        "clean native restriction".to_string(),
        outcome(compile(&hosts)),
        deferred.to_string(),
    ));
    assert_eq!(rows.len(), 11);
    each_row(rows);

    // The clean ON plan: held, switched on, exactly its argv.
    on(&live, None);
    let held = compile(&plain).unwrap();
    assert_eq!(
        held.sites["work"].local,
        Some(local(Some(&["cargo"]), Some(Sandbox::WorkspaceWrite)))
    );
    let outcome = &held.sites["work"].capabilities.as_ref().unwrap().outcomes[0];
    assert_eq!(outcome.held.keys().collect::<Vec<_>>(), ["web-search"]);
    assert_eq!(outcome.manifest()["native"]["on"], json!(["web-search"]));
    assert_eq!(outcome.manifest()["native"]["off"], json!([]));
    assert_eq!(
        native_argv(&held, 0),
        json!(["-c", "model_reasoning_effort=\"high\""])
    );
}

/// Unit 2-fix, every outcome and the effective class: a valid primary
/// cannot hide a later candidate whose resolved plan competes (link 2 is
/// named); a class the office declares and the seat inherits is judged
/// as the seat's own; a site with no typed class keeps its existing
/// admission, the same native bytes compiling exactly as before.
#[test]
fn resolved_native_admission_judges_every_link_and_the_inherited_class_only_where_typed() {
    let fixture = AgentFixture::new();
    fixture.write("adapters/codex.json", codex());
    // A second codex provider serving `sol`, whose OFF adds a root.
    let mut later = codex_off(&["--add-dir=/srv/shared"]);
    later["provider"] = json!("codex-later");
    later["models"] = json!({"sol": "gpt-6-sol"});
    later["judges"] = json!(["sol"]);
    fixture.write("adapters/codex-later.json", later);
    let mut rows: Vec<Row<String>> = Vec::new();
    fixture.write(
        "agents/boxed.json",
        boxed_agent(
            &["astra", "sol"],
            json!({"allow": ["cargo"], "sandbox": "workspace-write"}),
        ),
    );
    rows.push((
        "later candidate".to_string(),
        outcome(fixture.compile_under(sandbox_seat(&fixture, None), Boundary::Harness)),
        format!(
            "bundle: seat 'work' link 2 requests a typed 'tools.sandbox', but the {NATIVE} \
             {ADDED_ROOT_CAUSE}"
        ),
    ));
    // Inherited: the office declares the class, the seat writes `{}`.
    fixture.write("adapters/codex.json", codex_off(&["-C/"]));
    fixture.write(
        "agents/boxed.json",
        boxed_agent(&["astra"], json!({"sandbox": "workspace-write"})),
    );
    let mut inherited = sandbox_seat(&fixture, None);
    inherited["seats"]["work"]["tools"] = json!({});
    rows.push((
        "inherited class".to_string(),
        outcome(fixture.compile_under(inherited, Boundary::Harness)),
        root_refusal(1, NATIVE),
    ));
    each_row(rows);

    // Untyped: the same competing native bytes, no class requested,
    // compile as they did before this repair, with the exact argv.
    fixture.write(
        "agents/boxed.json",
        boxed_agent(&["astra"], json!({"allow": ["cargo"]})),
    );
    let untyped = || fixture.compile_under(sandbox_seat(&fixture, None), Boundary::Harness);
    assert_eq!(
        outcome(untyped()),
        format!(
            "compiled: {:?}",
            [
                ("review".to_string(), Some(LocalTools::unspecified())),
                ("work".to_string(), Some(local(Some(&["cargo"]), None)))
            ]
        )
    );
    let untyped = untyped().unwrap();
    assert_eq!(
        native_argv(&untyped, 0),
        json!(["-c", "web_search=\"disabled\"", "-C/"])
    );
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
    each_row(vec![
        (
            "narrowed to read-only at harness work".to_string(),
            outcome(
                fixture.compile_under(seat(json!({"sandbox": "read-only"})), Boundary::Harness),
            ),
            "bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only', but the \
             `hands.harness.work` fragment the engine selects for provider 'codex' under the \
             `harness` boundary expresses 'workspace-write'; a fragment is neither called \
             narrower nor clamped, the typed class must match it exactly — refused (design D5.3)"
                .to_string(),
        ),
        (
            "widened to danger-full-access".to_string(),
            outcome(fixture.compile_under(
                seat(json!({"sandbox": "danger-full-access"})),
                Boundary::Harness,
            )),
            widening(
                "work",
                "boxed",
                "sandbox",
                "requests 'danger-full-access', which reaches wider than the office's \
                 'workspace-write'; the classes reach read-only < workspace-write < \
                 danger-full-access, and a site narrows its office rather than being clamped \
                 to it",
            ),
        ),
    ]);
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
    each_row(vec![
        (
            "validate allow []".to_string(),
            outcome(compile(Some(json!({"allow": []})), None)),
            undelivered_allow("design:validate", "dispatches the 'exec' driver"),
        ),
        (
            "validate allow [cargo]".to_string(),
            outcome(compile(Some(json!({"allow": ["cargo"]})), None)),
            undelivered_allow("design:validate", "dispatches the 'exec' driver"),
        ),
        (
            "validate sandbox".to_string(),
            outcome(compile(Some(json!({"sandbox": "read-only"})), None)),
            inline_sandbox_refusal("design:validate"),
        ),
        (
            "check without a supplied check".to_string(),
            outcome(compile(None, Some(json!({})))),
            "bundle: sequence step 'clarify:check' declares 'tools' on a dialect step whose \
             'check' the dialect does not supply; no executable site exists to own the \
             declaration, so it could only be discarded — refused (decision 0065 slice one, \
             design D5)"
                .to_string(),
        ),
    ]);
    // The control: without the declaration the unsupported check compiles
    // to no site at all.
    let bundle = compile(None, None).unwrap();
    assert!(!bundle.sites.contains_key("clarify:check"));
}

/// The shipped `openspec` dialect, the shipped `exec` adapter its generated
/// validator dispatches copied beside the fixture's own claude.
fn openspec_with_exec(fixture: &AgentFixture) -> Dialect {
    let (root, exec) = (workspace_root(), fixture.adapters().join("exec.json"));
    std::fs::copy(root.join("adapters/exec.json"), exec).unwrap();
    let openspec = root.join("dialects/openspec.json");
    Dialect::load(&openspec).unwrap().0
}

/// SCM "Site-local narrowing cannot contaminate a shared office", last
/// clause: wrapper relocation carries the local value with the other site
/// facts — a dialect-wrapped agent-backed `verify` keeps its effective
/// declaration at `verify:checks` and nothing at `verify` — and the
/// validator the wrapper generates records the checked unspecified value
/// like every other visited executable (review return F3).
#[test]
fn a_dialect_wrapped_verify_relocates_its_declaration_and_the_validator_records_a_checked_value() {
    let fixture = AgentFixture::new();
    write_office(&fixture);
    let dialect = openspec_with_exec(&fixture);
    let policy = json!({
        "phases": ["design", "verify", "review", "done"], "initial": "design",
        "terminal": ["done"],
        "rules": [
            {"id":"D", "from":"design", "result":"drafted", "next":"verify", "reason":"drafted"},
            {"id":"DF", "from":"design", "result":"fail", "next":"design", "reason":"retry"},
            {"id":"V", "from":"verify", "result":"pass", "next":"review", "reason":"pass"},
            {"id":"VF", "from":"verify", "result":"fail", "next":"verify", "reason":"retry"},
            {"id":"R", "from":"review", "result":"clean", "next":"done", "reason":"clean"},
        ],
    });
    let config = json!({
        "name": "dialect-fixture", "policy": "policy.json", "protected_phase": "review",
        "seats": {
            "design": {"results": ["drafted", "fail"], "sequence": [
                {"name": "author", "results": ["drafted"], "role": "roles/work.md",
                 "driver": {"command": ["driver"]}},
                {"name": "validate", "dialect": "validate"}]},
            "verify": {"results": ["pass", "fail"], "agent": "office",
                       "tools": {"allow": ["git"]}},
            "review": {"results": ["clean"], "role": "roles/work.md",
                       "driver": {"command": ["driver"]}},
        },
    });
    fixture.stage(&config, &policy);
    let bundle = Bundle::compile_with_realm(
        &fixture.bundle(),
        &fixture.library(),
        &fixture.adapters(),
        None,
        Some(&dialect),
        Boundary::Namespace,
    )
    .unwrap();
    let facts = |label: &str| {
        bundle
            .sites
            .get(label)
            .map(|facts| (facts.local.clone(), facts.chain.len()))
    };
    each_row(vec![
        (
            "verify:checks carries the effective declaration".to_string(),
            facts("verify:checks"),
            Some((Some(local(Some(&["git"]), None)), 1)),
        ),
        (
            "verify keeps nothing behind".to_string(),
            facts("verify"),
            None,
        ),
        (
            "verify:dialect-verify records a checked unspecified value".to_string(),
            facts("verify:dialect-verify"),
            Some((Some(LocalTools::unspecified()), 0)),
        ),
        (
            "design:validate records a checked unspecified value".to_string(),
            facts("design:validate"),
            Some((Some(LocalTools::unspecified()), 0)),
        ),
    ]);
    assert_eq!(
        bundle.sites["verify:checks"].chain[0].argv[8..],
        ["--allowedTools", "Bash(git:*)"]
    );
}

/// The complete refusal of an unknown key inside a site's `driver`
/// object (rebuild unit 5e), naming the key as `named` and the place a
/// capability key belongs as `place`.
pub(super) fn misplaced_in_driver(site: &str, named: &str, place: &str) -> String {
    format!(
        "bundle: seat '{site}' driver has an unknown key, {named}; known: command.{place} A key the \
         compiler does not read is a declaration that was never made — a capability placed \
         there would compile, deliver nothing and run the seat at its harness default — so it \
         is refused rather than ignored (decision 0004; decision 0065 slice one, rebuild unit 5e)"
    )
}

/// Rebuild unit 5e: unit 7's probe (evidence.md, "Unit 7 — blocked on the
/// second visit") put a seat's `tools` under its `driver`, with the authored
/// `--sandbox` pair removed, and the recipe compiled with no sandbox and no
/// restriction at all. The same shape is now refused by name.
#[test]
fn a_tools_object_misplaced_under_the_driver_is_refused_by_name() {
    let fixture = AgentFixture::new();
    fixture.write("adapters/codex.json", codex());
    let mut config = fixture.config();
    config["seats"]["review"]["driver"] = json!({
        "command": ["{brokkr}", "driver", "codex", "--", "--model", "gpt-6-astra", "--effort", "high"],
        "tools": {"sandbox": "workspace-write"},
    });
    assert_eq!(
        outcome(fixture.compile(config)),
        misplaced_in_driver(
            "review",
            "'tools'",
            " 'tools' is a site declaration, written on the seat beside its driver."
        )
    );
}

/// Rebuild unit 5e: every other key the driver object does not read is
/// refused the same way — a second capability key, the site declarations
/// `hands` and `capabilities`, a harness word — and a key that is not
/// a short name is described, never echoed, so the reason stays bounded.
#[test]
fn every_other_unknown_driver_key_is_refused_and_named_boundedly() {
    let fixture = AgentFixture::new();
    let site = " 'KEY' is a site declaration, written on the seat beside its driver.";
    let long = "k".repeat(65);
    let mut rows: Vec<Row<String>> = Vec::new();
    for (key, named, place) in [
        (
            "sandbox",
            "'sandbox'".to_string(),
            " 'sandbox' is a typed tool field, written as 'tools.sandbox' on the seat.".to_string(),
        ),
        ("hands", "'hands'".to_string(), site.replace("KEY", "hands")),
        (
            "capabilities",
            "'capabilities'".to_string(),
            site.replace("KEY", "capabilities"),
        ),
        (
            "allowed_tools",
            "'allowed_tools'".to_string(),
            String::new(),
        ),
        (
            long.as_str(),
            "one that is not a short name and is not echoed".to_string(),
            String::new(),
        ),
        (
            "tools allow",
            "one that is not a short name and is not echoed".to_string(),
            String::new(),
        ),
    ] {
        let mut config = fixture.config();
        config["seats"]["review"]["driver"] = json!({
            "command": claude_inline("claude", &[]),
            key: "workspace-write",
        });
        rows.push((
            key.chars().take(16).collect(),
            outcome(fixture.compile(config)),
            misplaced_in_driver("review", &named, &place),
        ));
    }
    each_row(rows);
}

/// Rebuild unit 5e: the refusal reads the driver object, not the seat, so
/// a `tools` declaration where it belongs still compiles and lowers.
#[test]
fn a_seat_level_tools_declaration_beside_its_driver_still_compiles() {
    let fixture = AgentFixture::new();
    let mut config = fixture.config();
    config["seats"]["review"]["driver"]["command"] = claude_inline("claude", &[]);
    config["seats"]["review"]["tools"] = json!({"allow": ["cargo"]});
    let bundle = fixture.compile(config).unwrap();
    assert_eq!(
        format!(
            "{:?} {:?}",
            bundle.sites["review"].local, bundle.sites["review"].inline_local
        ),
        format!(
            "{:?} {:?}",
            Some(local(Some(&["cargo"]), None)),
            cargo_lowering()
        )
    );
}

/// The expected binding of a charter an inline role names: the declaring
/// layer, the key its walk pins the reference under, the reference as
/// written, the path the seat is told and the canonical target read.
fn layer_pin(dir: &Path, reference: &str, target: &str) -> CharterPin {
    CharterPin {
        owner: CharterOwner::Layer {
            dir: dir.to_path_buf(),
            key: reference.to_string(),
        },
        reference: reference.to_string(),
        path: dir.join(reference),
        binding: Binding::expected(dir, reference, target),
        directory: owner_directory(dir).ok().unwrap().1,
        digest: sha256_bytes(b"# work\n"),
    }
}

/// The expected binding of an agent's charter: the library it was loaded
/// from, with that library's own root.
fn library_pin(agent: &str, root: &Path, reference: &str) -> CharterPin {
    CharterPin {
        owner: CharterOwner::Library {
            agent: agent.to_string(),
            root: root.to_path_buf(),
        },
        reference: reference.to_string(),
        path: root.join("charters/work.md"),
        binding: Binding::expected(root, reference, "charters/work.md"),
        directory: owner_directory(root).ok().unwrap().1,
        digest: sha256_bytes(b"# work\n"),
    }
}

/// One site's binding as observed: the pin it carries, the digest its
/// owner's existing identity names — the layer's file-map entry or the
/// library record — and the office and model of every candidate.
fn bound(bundle: &Bundle, label: &str) -> String {
    let facts = &bundle.sites[label];
    let existing = match facts.charter.as_ref().map(|pin| &pin.owner) {
        Some(CharterOwner::Layer { dir, key }) if dir == &bundle.dir => {
            bundle.manifest["files"][key].clone()
        }
        Some(CharterOwner::Layer { dir, key }) => bundle
            .chain
            .iter()
            .find(|ancestor| &ancestor.dir == dir)
            .map_or(Value::Null, |ancestor| ancestor.files[key].clone()),
        Some(CharterOwner::Library { .. }) => {
            bundle.manifest["agents"][label]["charter_digest"].clone()
        }
        None => Value::Null,
    };
    let chain: Vec<(&str, &str)> = facts
        .chain
        .iter()
        .map(|candidate| (candidate.agent.as_str(), candidate.model.as_str()))
        .collect();
    format!("{:?} {existing} {chain:?}", facts.charter)
}

/// Rebuild unit 17 (design D7; task 17.1): SELECT CHARTER OWNER AND SOURCE
/// AT COMPILE. Every executable site with a charter — an ordinary seat, a
/// panel member, a sequence step, a selected case and the selected default,
/// agent-backed or inline, in the leaf or an inherited layer — carries the
/// owner compiled with it, the reference as written, the path the seat is
/// told, the canonical target read and the digest its owner already pins.
/// Each charter is reached through a contained link, so reference, told
/// path and target are three facts. An agent site's fallback candidate is
/// the same office, bound once. The library stands outside the recipe.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn every_selected_site_binds_its_charter_owner_reference_target_and_digest() {
    let fixture = AgentFixture::new();
    std::os::unix::fs::symlink("work.md", fixture.library().join("charters/linked.md")).unwrap();
    std::os::unix::fs::symlink("work.md", fixture.bundle().join("roles/linked.md")).unwrap();
    // A contained link whose text climbs to its parent (rebuild unit 26b).
    std::os::unix::fs::symlink("../roles/work.md", fixture.bundle().join("roles/up.md")).unwrap();
    let mut adapter = claude();
    adapter["models"]["sonnet"] = json!("claude-sonnet-5");
    fixture.write("adapters/claude.json", adapter);
    fixture.write(
        "agents/member.json",
        json!({
            "description": "a member",
            "charter": "charters/linked.md",
            "models": ["opus", "sonnet"],
            "efforts": {"opus": "high", "sonnet": "high"},
        }),
    );
    let agent = json!({"agent": "member"});
    let linked = json!({"role": "roles/linked.md", "driver": {"command": ["driver"]}});
    let plain = json!({"role": "roles/work.md", "driver": {"command": ["driver"]}});
    let with = |value: &Value, extra: Value| {
        let mut value = value.clone();
        value
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        value
    };
    let at_library = || {
        format!(
            "{:?} {} {:?}",
            Some(library_pin(
                "member",
                &fixture.library(),
                "charters/linked.md"
            )),
            json!(sha256_bytes(b"# work\n")),
            [("member", "opus"), ("member", "sonnet")]
        )
    };
    let at_layer = |dir: &Path, reference: &str, target: &str| {
        format!(
            "{:?} {} []",
            Some(layer_pin(dir, reference, target)),
            json!(sha256_bytes(b"# work\n"))
        )
    };
    let leaf = fixture.bundle();
    // (the `work` seat, its policy, and each site label with its binding).
    type Form<'a> = (Value, Value, Vec<(&'a str, String)>);
    let forms: Vec<Form<'_>> = vec![
        (
            with(&agent, json!({"results": ["complete"]})),
            policy(),
            vec![
                ("work", at_library()),
                ("review", at_layer(&leaf, "roles/work.md", "roles/work.md")),
            ],
        ),
        (
            json!({"results": ["pass", "fail"], "aggregate": "unanimous-pass",
                   "panel": {"a": agent, "b": linked}}),
            panel_policy(),
            vec![
                ("work:a", at_library()),
                (
                    "work:b",
                    at_layer(&leaf, "roles/linked.md", "roles/work.md"),
                ),
            ],
        ),
        (
            json!({"results": ["complete"], "sequence": [
                with(&agent, json!({"name": "first", "results": ["complete"]})),
                with(&linked, json!({"name": "second"}))]}),
            policy(),
            vec![
                ("work:first", at_library()),
                (
                    "work:second",
                    at_layer(&leaf, "roles/linked.md", "roles/work.md"),
                ),
            ],
        ),
        (
            json!({"results": ["complete"], "select": {"on": "strategy",
                   "cases": {"engine": agent, "chore": plain}, "default": linked}}),
            policy(),
            vec![
                ("work:engine", at_library()),
                (
                    "work:chore",
                    at_layer(&leaf, "roles/work.md", "roles/work.md"),
                ),
                (
                    "work:default",
                    at_layer(&leaf, "roles/linked.md", "roles/work.md"),
                ),
            ],
        ),
        (
            with(
                &plain,
                json!({"results": ["complete"], "role": "roles/up.md"}),
            ),
            policy(),
            vec![("work", at_layer(&leaf, "roles/up.md", "roles/work.md"))],
        ),
    ];
    let mut rows: Vec<Row<String>> = Vec::new();
    for (seat, table, labels) in forms {
        let mut config = fixture.config();
        config["seats"]["work"] = seat;
        let bundle = fixture.compile_with_policy(config, &table).unwrap();
        for (label, expected) in labels {
            rows.push((label.to_string(), bound(&bundle, label), expected));
        }
    }
    // The dispatch map holds exactly the bindings the sites selected.
    let mut config = fixture.config();
    config["seats"]["work"] = with(&agent, json!({"results": ["complete"]}));
    let bundle = fixture.compile(config).unwrap();
    rows.push((
        "the bundle's charters".to_string(),
        format!("{:?}", bundle.charters),
        format!(
            "{:?}",
            CharterPins::from([
                (
                    fixture.library().join("charters/work.md"),
                    BTreeSet::from([library_pin(
                        "member",
                        &fixture.library(),
                        "charters/linked.md"
                    )]),
                ),
                (
                    leaf.join("roles/work.md"),
                    BTreeSet::from([layer_pin(&leaf, "roles/work.md", "roles/work.md")]),
                ),
            ])
        ),
    ));
    // An inherited layer's seats are bound to the layer that wrote them.
    let base = fixture.root.join("base");
    std::fs::create_dir_all(base.join("roles")).unwrap();
    std::fs::write(base.join("roles/work.md"), "# work\n").unwrap();
    std::os::unix::fs::symlink("work.md", base.join("roles/linked.md")).unwrap();
    std::fs::write(
        base.join("policy.json"),
        serde_json::to_vec(&policy()).unwrap(),
    )
    .unwrap();
    let mut config = fixture.config();
    config["name"] = json!("base");
    config["seats"]["work"] = with(&agent, json!({"results": ["complete"]}));
    config["seats"]["review"] = with(&linked, json!({"results": ["clean"]}));
    std::fs::write(
        base.join("bundle.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    std::fs::write(
        leaf.join("bundle.json"),
        serde_json::to_vec(&json!({"name": "fixture", "extends": "base"})).unwrap(),
    )
    .unwrap();
    let bundle = Bundle::compile_with(&leaf, &fixture.library(), &fixture.adapters()).unwrap();
    rows.push((
        "inherited work".to_string(),
        bound(&bundle, "work"),
        at_library(),
    ));
    rows.push((
        "inherited review".to_string(),
        bound(&bundle, "review"),
        at_layer(&base, "roles/linked.md", "roles/work.md"),
    ));
    each_row(rows);
}

/// Rebuild unit 17 (design D7; task 17.1): NO LONGEST-PREFIX OWNER GUESS.
/// A library nested inside the recipe stands in a tree its layer's walk
/// also pins, so the longest layer root its charter's path starts with is
/// the recipe's — a neighbouring pin. The owner is the library the site was
/// compiled against, with its own root: a changed charter is refused as
/// that library's. Where an inline role names the same file, the two
/// owners overlap and each site keeps its own; every binding must hold.
/// A path spelled with `..` onto a pinned charter is not folded onto that
/// pin, and a recipe reference out of its tree to an external library's
/// charter is refused at compile, never bound as a library pin.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn a_nested_library_owns_its_charter_and_no_recipe_path_is_reclassified() {
    let fixture = AgentFixture::new();
    let nested = fixture.bundle().join("lib");
    std::fs::create_dir_all(nested.join("charters")).unwrap();
    std::fs::write(nested.join("charters/work.md"), "# work\n").unwrap();
    std::fs::write(
        nested.join("member.json"),
        serde_json::to_vec(&json!({
            "description": "a member",
            "charter": "charters/work.md",
            "models": ["opus"],
            "efforts": {"opus": "high"},
        }))
        .unwrap(),
    )
    .unwrap();
    let charter = nested.join("charters/work.md");
    let compiled = |review: Value| {
        let mut config = fixture.config();
        config["seats"]["work"] = json!({"results": ["complete"], "agent": "member"});
        config["seats"]["review"] = review;
        fixture.stage(&config, &policy());
        Bundle::compile_with(&fixture.bundle(), &nested, &fixture.adapters())
    };
    let inline =
        |role: &str| json!({"results": ["clean"], "role": role, "driver": {"command": ["driver"]}});
    let nested_pin = library_pin("member", &nested, "charters/work.md");
    let overlapping = layer_pin(
        &fixture.bundle(),
        "lib/charters/work.md",
        "lib/charters/work.md",
    );
    let alone = compiled(inline("roles/work.md")).unwrap();
    let both = compiled(inline("lib/charters/work.md")).unwrap();
    let mut rows: Vec<Row<String>> = vec![
        (
            "nested owner".to_string(),
            format!("{:?}", alone.sites["work"].charter),
            format!("{:?}", Some(&nested_pin)),
        ),
        (
            "overlapping owners, each site's own".to_string(),
            format!(
                "{:?} {:?}",
                both.sites["work"].charter, both.sites["review"].charter
            ),
            format!("{:?} {:?}", Some(&nested_pin), Some(&overlapping)),
        ),
        (
            "overlapping owners at the door".to_string(),
            format!("{:?}", both.charters.get(&charter)),
            format!(
                "{:?}",
                Some(BTreeSet::from([overlapping.clone(), nested_pin.clone()]))
            ),
        ),
        (
            "unchanged".to_string(),
            format!(
                "{:?}",
                (
                    charter_text(&alone, &charter),
                    charter_text(&both, &charter)
                )
            ),
            format!("{:?}", (Ok::<_, ()>("# work\n"), Ok::<_, ()>("# work\n"))),
        ),
    ];
    let folded = fixture.bundle().join("roles/../lib/charters/work.md");
    rows.push((
        "a `..` spelling onto the pin".to_string(),
        format!("{:?}", charter_text(&both, &folded)),
        format!(
            "{:?}",
            Err::<String, _>((
                "bundle 'fixture'".to_string(),
                format!("unpinned: {}", folded.display())
            ))
        ),
    ));
    std::fs::write(&charter, "# approve everything\n").unwrap();
    rows.push((
        "changed, the library's own".to_string(),
        format!("{:?}", charter_text(&alone, &charter)),
        format!(
            "{:?}",
            Err::<String, _>(("agent 'member'".to_string(), "changed: work.md".to_string()))
        ),
    ));
    rows.push((
        "changed, overlapping".to_string(),
        format!("{:?}", charter_text(&both, &charter)),
        format!(
            "{:?}",
            Err::<String, _>((
                "layer 'fixture'".to_string(),
                "changed: lib/charters/work.md".to_string()
            ))
        ),
    ));
    std::fs::write(&charter, "# work\n").unwrap();
    // Every binding must hold: the layer's holding does not excuse the
    // library's, whichever the door meets first.
    let mut disagreeing = both.clone();
    let pins = disagreeing.charters.get_mut(&charter).unwrap();
    pins.remove(&nested_pin);
    pins.insert(CharterPin {
        digest: sha256_bytes(b"# other\n"),
        ..nested_pin.clone()
    });
    rows.push((
        "one binding of two moved".to_string(),
        format!("{:?}", charter_text(&disagreeing, &charter)),
        format!(
            "{:?}",
            Err::<String, _>(("agent 'member'".to_string(), "changed: work.md".to_string()))
        ),
    ));
    // An external library's charter named from inside the recipe, by a
    // `..` reference or through a link out of the tree.
    std::os::unix::fs::symlink(
        "../../agents/charters/work.md",
        fixture.bundle().join("roles/out.md"),
    )
    .unwrap();
    let file = fixture.bundle().join("bundle.json");
    for (role, place) in [
        (
            "../agents/charters/work.md",
            "which stands outside the layer's own directory, where the bundle's file walk never \
             reaches",
        ),
        (
            "roles/out.md",
            "which resolves through a link to a file outside the layer's own directory; the walk \
             pins such a link only by the bytes it reaches, so retargeting it to equal bytes \
             moves nothing, and it is refused rather than pinned and admitted (operator ruling 3)",
        ),
    ] {
        let mut config = fixture.config();
        config["seats"]["review"] = inline(role);
        rows.push((
            format!("escape {role}"),
            outcome(fixture.compile(config)),
            format!(
                "bundle: {}: seat 'review' names role '{role}', {place}. A charter there could \
                 change what the seat is told without moving the bundle's identity, so it is \
                 refused; move it to a path the bundle pins, such as 'roles/' (decision 0066 \
                 ruling 5)",
                file.display()
            ),
        ));
    }
    each_row(rows);
}
mod charter_tests;
mod gate_tests;
