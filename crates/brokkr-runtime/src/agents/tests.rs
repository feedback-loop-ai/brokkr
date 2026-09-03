use super::*;
use std::path::Path;

/// A throwaway library + adapters tree. Every test writes exactly the
/// data it is about, so a rejection message can be asserted verbatim.
struct Tree {
    dir: tempfile::TempDir,
}

impl Tree {
    fn new() -> Tree {
        let tree = Tree {
            dir: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(tree.library_root().join("charters")).unwrap();
        std::fs::create_dir_all(tree.adapters_root()).unwrap();
        std::fs::write(tree.library_root().join("charters/c.md"), "# charter\n").unwrap();
        tree
    }

    fn library_root(&self) -> PathBuf {
        self.dir.path().join("agents")
    }

    fn adapters_root(&self) -> PathBuf {
        self.dir.path().join("adapters")
    }

    fn write(&self, relative: &str, body: &Value) {
        std::fs::write(
            self.dir.path().join(relative),
            serde_json::to_vec_pretty(body).unwrap(),
        )
        .unwrap();
    }

    fn raw(&self, relative: &str, body: &str) {
        std::fs::write(self.dir.path().join(relative), body).unwrap();
    }

    fn library(&self) -> Library {
        Library::load(&self.library_root()).unwrap()
    }

    fn adapters(&self) -> Adapters {
        Adapters::load(&self.adapters_root()).unwrap()
    }

    fn library_error(&self) -> String {
        Library::load(&self.library_root()).unwrap_err().to_string()
    }

    fn adapters_error(&self) -> String {
        Adapters::load(&self.adapters_root())
            .unwrap_err()
            .to_string()
    }
}

fn agent_body() -> Value {
    json!({
        "description": "a test agent",
        "charter": "charters/c.md",
        "models": ["opus", "sonnet"],
        "efforts": {"opus": "high", "sonnet": "medium"},
        "tools": {"allow": ["cargo", "git"], "mcp": []},
        "limits": {"max_attempts": 2, "timeout_seconds": 60},
    })
}

fn claude_body() -> Value {
    json!({
        "provider": "claude",
        "binary": "claude",
        "driver": ["{brokkr}", "driver", "claude", "--"],
        "models": {"opus": "claude-opus-5", "sonnet": "claude-sonnet-5"},
        "model_flag": "--model",
        "efforts": ["low", "medium", "high"],
        "effort_flag": "--effort",
        "tool_permissions": {
            "flag": "--allowedTools",
            "separator": ",",
            "names": {"cargo": "Bash(cargo:*)", "git": "Bash(git:*)"},
        },
        "mcp": {"flag": "--mcp-config", "servers": {"github": "/etc/github.json"}},
    })
}

/// The common shape: one agent, one provider that can serve it.
fn ready() -> Tree {
    let tree = Tree::new();
    tree.write("agents/tester.json", &agent_body());
    tree.write("adapters/claude.json", &claude_body());
    tree
}

fn resolved(tree: &Tree, availability: &Availability) -> Resolution {
    resolve(&tree.library(), &tree.adapters(), availability, "tester").unwrap()
}

fn refusal(tree: &Tree, name: &str) -> String {
    resolve(
        &tree.library(),
        &tree.adapters(),
        &Availability::unspecified(),
        name,
    )
    .unwrap_err()
    .to_string()
}

// ------------------------------------------------------------- purity

/// AC-1's anti-drift half: the resolver module names no filesystem, no
/// environment, no process and no clock. Purity is checked as a property
/// of the source, not left to review discipline.
#[test]
fn the_resolver_module_reaches_for_nothing_outside_its_arguments() {
    let source =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/agents.rs"))
            .unwrap();
    // The module doc names these in prose; strip comment lines first.
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "std::fs",
        "std::env",
        "std::process",
        "Command",
        "SystemTime",
        "OffsetDateTime",
        "Instant",
    ] {
        assert!(
            !code.contains(forbidden),
            "agents.rs must not name '{forbidden}': resolution is a pure \
             function of (library, adapters, availability)"
        );
    }
}

/// AC-1: same inputs, byte-identical output including key order.
#[test]
fn resolution_is_deterministic_to_the_byte() {
    let tree = ready();
    let first = resolved(&tree, &Availability::unspecified());
    let second = resolved(&tree, &Availability::unspecified());
    assert_eq!(
        serde_json::to_string(&first.record).unwrap(),
        serde_json::to_string(&second.record).unwrap()
    );
    assert_eq!(first.candidates, second.candidates);
}

#[test]
fn names_obey_one_grammar() {
    assert!(valid_name("chief-architect"));
    assert!(valid_name("gpt-5"));
    assert!(!valid_name(""));
    assert!(!valid_name("Chief"));
    assert!(!valid_name("chief_architect"));
}

#[test]
fn availability_defaults_to_unknown_and_records_what_was_probed() {
    let mut availability = Availability::unspecified();
    assert_eq!(availability.presence("claude"), Presence::Unknown);
    availability.record("claude", Presence::Available);
    availability.record("codex", Presence::Unavailable);
    assert_eq!(availability.presence("claude"), Presence::Available);
    assert_eq!(availability.presence("codex"), Presence::Unavailable);
}

// --------------------------------------------------------- composition

/// AC-5's building block: the composed argv is a lookup and a join, in
/// the agent's declared tool order, with `{brokkr}` still a literal.
#[test]
fn composition_is_a_lookup_and_a_join() {
    let tree = ready();
    let resolution = resolved(&tree, &Availability::unspecified());
    assert_eq!(
        resolution.candidates[0].argv,
        vec![
            "{brokkr}",
            "driver",
            "claude",
            "--",
            "--model",
            "claude-opus-5",
            // The other half of the hire, composed from the same data by
            // the same lookup: decision 0035 ruling 5.
            "--effort",
            "high",
            "--allowedTools",
            "Bash(cargo:*),Bash(git:*)",
        ]
    );
    assert_eq!(resolution.candidates[0].model, "opus");
    assert_eq!(resolution.candidates[0].effort.as_deref(), Some("high"));
    // Per candidate, not per agent: the chain's second link is hired at
    // its own level and the record of it says so.
    assert_eq!(resolution.candidates[1].effort.as_deref(), Some("medium"));
    assert_eq!(resolution.candidates[0].provider, "claude");
    assert_eq!(resolution.candidates.len(), 2, "the whole chain, in order");
    assert_eq!(resolution.limits.unwrap().max_attempts, 2);
    assert!(resolution.inputs.is_none());
    assert!(resolution.notices.is_empty());
}

/// An agent that declares no restriction gets none composed, and the
/// record says so by carrying no tool flag at all.
#[test]
fn an_agent_without_tools_allow_declares_no_restriction() {
    let tree = Tree::new();
    let mut body = agent_body();
    body.as_object_mut().unwrap().remove("tools");
    tree.write("agents/tester.json", &body);
    tree.write("adapters/claude.json", &claude_body());
    let resolution = resolved(&tree, &Availability::unspecified());
    assert_eq!(
        resolution.candidates[0].argv,
        vec![
            "{brokkr}",
            "driver",
            "claude",
            "--",
            "--model",
            "claude-opus-5",
            "--effort",
            "high"
        ]
    );
}

/// A named MCP server the provider declares is composed onto the command
/// line; matching is per named item.
#[test]
fn a_declared_mcp_server_reaches_the_command_line() {
    let tree = Tree::new();
    let mut body = agent_body();
    body["tools"]["mcp"] = json!([{"server": "github"}]);
    tree.write("agents/tester.json", &body);
    tree.write("adapters/claude.json", &claude_body());
    let resolution = resolved(&tree, &Availability::unspecified());
    assert!(resolution.candidates[0]
        .argv
        .windows(2)
        .any(|pair| pair == ["--mcp-config", "/etc/github.json"]));
}

// ------------------------------------------------------- honesty rules

/// AC-2: a restriction the provider cannot express fails compilation,
/// and the message names the agent, the provider and the capability.
#[test]
fn a_restriction_the_provider_cannot_express_is_a_hard_failure() {
    let tree = Tree::new();
    tree.write("agents/tester.json", &agent_body());
    let mut adapter = claude_body();
    adapter["tool_permissions"] = json!("unsupported");
    tree.write("adapters/claude.json", &adapter);
    let message = refusal(&tree, "tester");
    assert!(message.contains("agent 'tester'"), "{message}");
    assert!(message.contains("provider 'claude'"), "{message}");
    assert!(
        message.contains("tool_permissions unsupported"),
        "{message}"
    );
    assert!(message.contains("MORE power"), "{message}");
}

/// The same refusal, from a provider that MEASURED its gap: the
/// attempt still fails — a reason is not a capability — but the message
/// now names the restriction axis the provider does have, so the reader
/// learns what to do about it instead of only that it cannot.
#[test]
fn a_measured_gap_refuses_exactly_as_a_bare_unsupported_does() {
    let tree = Tree::new();
    tree.write("agents/tester.json", &agent_body());
    let mut adapter = claude_body();
    adapter["tool_permissions"] = json!({
        "unsupported": "restricts by sandbox CLASS, not by tool name: --sandbox \
                        read-only|workspace-write|danger-full-access"
    });
    tree.write("adapters/claude.json", &adapter);
    let message = refusal(&tree, "tester");
    // Everything the bare-`"unsupported"` arm asserts still holds …
    assert!(message.contains("agent 'tester'"), "{message}");
    assert!(message.contains("provider 'claude'"), "{message}");
    assert!(
        message.contains("tool_permissions unsupported"),
        "{message}"
    );
    assert!(message.contains("MORE power"), "{message}");
    // … plus the measured axis, which is the whole point of declaring it.
    assert!(message.contains("--sandbox"), "{message}");
    assert!(message.contains("sandbox CLASS"), "{message}");
}

/// A declared gap is not a back door: an adapter cannot smuggle a
/// working allow-list in beside the reason it says it has none.
#[test]
fn a_declared_gap_admits_no_other_key() {
    let tree = Tree::new();
    let mut adapter = claude_body();
    adapter["tool_permissions"] = json!({
        "unsupported": "no per-tool flag",
        "flag": "--allowedTools",
    });
    tree.write("adapters/claude.json", &adapter);
    let error = tree.adapters_error();
    assert!(
        error.contains("'tool_permissions' has unknown key"),
        "{error}"
    );
}

/// An empty reason is the bare `"unsupported"` wearing a costume — it
/// records nothing, so it is refused at load rather than read back as
/// evidence that someone looked.
#[test]
fn a_declared_gap_needs_an_actual_reason() {
    let tree = Tree::new();
    let mut adapter = claude_body();
    adapter["tool_permissions"] = json!({"unsupported": ""});
    tree.write("adapters/claude.json", &adapter);
    let error = tree.adapters_error();
    assert!(
        error.contains("needs a non-empty string 'unsupported'"),
        "{error}"
    );
}

/// Per named item, never per class: the provider expresses tool
/// permissions, just not this one.
#[test]
fn a_tool_the_provider_does_not_name_is_a_hard_failure() {
    let tree = Tree::new();
    tree.write("agents/tester.json", &agent_body());
    let mut adapter = claude_body();
    adapter["tool_permissions"]["names"] = json!({"cargo": "Bash(cargo:*)"});
    tree.write("adapters/claude.json", &adapter);
    let message = refusal(&tree, "tester");
    assert!(
        message.contains("maps no tool permission named 'git'"),
        "{message}"
    );
}

/// A provider that serves the model but cannot be told which model would
/// run its own default and let the run claim the pinned one.
#[test]
fn a_provider_that_cannot_pin_the_model_is_a_hard_failure() {
    let tree = Tree::new();
    tree.write("agents/tester.json", &agent_body());
    let mut adapter = claude_body();
    adapter["model_flag"] = json!("unsupported");
    tree.write("adapters/claude.json", &adapter);
    let message = refusal(&tree, "tester");
    assert!(message.contains("model_flag unsupported"), "{message}");
    assert!(message.contains("default would run"), "{message}");
}

/// A REQUIRED MCP server the provider cannot serve fails, whether the
/// provider lacks MCP entirely or merely lacks that server.
#[test]
fn a_required_mcp_grant_the_provider_cannot_serve_is_a_hard_failure() {
    for (adapter_mcp, expected) in [
        (json!("unsupported"), "declares mcp unsupported"),
        (
            json!({"flag": "--mcp-config", "servers": {}}),
            "declares no MCP server named 'github'",
        ),
    ] {
        let tree = Tree::new();
        let mut body = agent_body();
        body["tools"]["mcp"] = json!([{"server": "github"}]);
        tree.write("agents/tester.json", &body);
        let mut adapter = claude_body();
        adapter["mcp"] = adapter_mcp;
        tree.write("adapters/claude.json", &adapter);
        let message = refusal(&tree, "tester");
        assert!(message.contains(expected), "{message}");
    }
}

/// AC-3: an OPTIONAL grant gap warns rather than failing, and the
/// warning is a value that reaches the manifest record — never a print.
#[test]
fn an_optional_mcp_grant_gap_becomes_a_notice_in_the_record() {
    for adapter_mcp in [
        json!("unsupported"),
        json!({"flag": "--mcp-config", "servers": {}}),
    ] {
        let tree = Tree::new();
        let mut body = agent_body();
        body["tools"]["mcp"] = json!([{"server": "github", "optional": true}]);
        tree.write("agents/tester.json", &body);
        let mut adapter = claude_body();
        adapter["mcp"] = adapter_mcp;
        tree.write("adapters/claude.json", &adapter);
        let resolution = resolved(&tree, &Availability::unspecified());
        // Two chain entries, both on the same gapped provider.
        assert_eq!(resolution.notices.len(), 2);
        let notice = &resolution.notices[0];
        assert_eq!(notice.capability, "mcp");
        assert_eq!(notice.item, "github");
        assert!(notice.message.contains("less power"), "{notice:?}");
        let recorded = resolution.record["notices"].as_array().unwrap();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0]["item"], "github");
        assert_eq!(recorded[0]["agent"], "tester");
        assert_eq!(recorded[0]["provider"], "claude");
    }
}

/// The pinch of salt made mechanical: a gap on a NON-CHOSEN entry fails
/// exactly as loudly as one on the chosen entry, because a chain that
/// would widen the agent's blast radius on fallback is a design-time
/// error, not a 2am surprise.
#[test]
fn a_capability_gap_on_a_later_chain_entry_fails_just_as_loudly() {
    let tree = Tree::new();
    tree.write("agents/tester.json", &agent_body());
    let mut claude = claude_body();
    claude["models"] = json!({"opus": "claude-opus-5"});
    tree.write("adapters/claude.json", &claude);
    tree.write(
        "adapters/codex.json",
        &json!({
            "provider": "codex",
            "binary": "codex",
            "driver": ["{brokkr}", "driver", "codex", "--"],
            "models": {"sonnet": "gpt-x"},
            "model_flag": "--model",
            "efforts": ["low", "medium", "high"],
            "effort_flag": "--effort",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        }),
    );
    let message = refusal(&tree, "tester");
    assert!(message.contains("provider 'codex'"), "{message}");
    assert!(message.contains("model 'sonnet'"), "{message}");
}

/// Decision 0035 ruling 5's three refusals, each tripped on its own. A
/// model pin without an effort pin is half a hire, and the half it
/// withholds is the half that moves the bill — so the resolver refuses
/// where the provider, and therefore the vocabulary, is known, and every
/// refusal names the repair rather than the rule alone.
#[test]
fn an_effort_that_cannot_be_pinned_as_asked_is_refused_with_its_vocabulary() {
    // 1. The provider takes an effort and this candidate pins none.
    let unpinned = Tree::new();
    let mut body = agent_body();
    body["models"] = json!(["sonnet"]);
    body["efforts"] = json!({});
    unpinned.write("agents/tester.json", &body);
    let mut claude = claude_body();
    claude["models"] = json!({"sonnet": "claude-sonnet-5"});
    unpinned.write("adapters/claude.json", &claude);
    let message = refusal(&unpinned, "tester");
    assert!(
        message.contains("takes an effort and this candidate pins none"),
        "{message}"
    );
    // The repair, in the vocabulary this driver's adapter declares.
    assert!(message.contains(r#""efforts": {"sonnet""#), "{message}");
    assert!(message.contains("low, medium, high"), "{message}");

    // 2. A level outside the vocabulary the provider declares. A pin the
    //    harness would reject at 2am is a design-time error here.
    let unknown = Tree::new();
    let mut body = agent_body();
    body["models"] = json!(["opus"]);
    body["efforts"] = json!({"opus": "xhigh"});
    unknown.write("agents/tester.json", &body);
    let mut claude = claude_body();
    claude["models"] = json!({"opus": "claude-opus-5"});
    unknown.write("adapters/claude.json", &claude);
    let message = refusal(&unknown, "tester");
    assert!(message.contains("declares no effort 'xhigh'"), "{message}");
    assert!(message.contains("low, medium, high"), "{message}");

    // 3. The mirror image: the agent names an effort the provider has no
    //    way to be TOLD, so the provider's own default would run. That is
    //    the silent-substitution case ruling 1 exists to refuse, and it
    //    fails compilation rather than degrading quietly.
    let effortless = Tree::new();
    let mut body = agent_body();
    body["models"] = json!(["opus"]);
    body["efforts"] = json!({"opus": "high"});
    effortless.write("agents/tester.json", &body);
    let mut claude = claude_body();
    claude["models"] = json!({"opus": "claude-opus-5"});
    claude["efforts"] = json!([]);
    claude["effort_flag"] = json!("unsupported");
    effortless.write("adapters/claude.json", &claude);
    let message = refusal(&effortless, "tester");
    assert!(
        message.contains("declares effort_flag unsupported"),
        "{message}"
    );
    assert!(
        message.contains("provider's own default would run"),
        "{message}"
    );
}

/// An effort named for a candidate the chain does not contain is a
/// typo the loader catches, where the whole chain is in view. Keyed by
/// candidate rather than positionally on purpose: a chain reordered in
/// review must not silently re-hire every seat at a different level.
#[test]
fn an_effort_for_a_candidate_outside_the_chain_is_refused_by_name() {
    let tree = Tree::new();
    let mut body = agent_body();
    body["efforts"] = json!({"opus": "high", "haiku": "low"});
    tree.write("agents/tester.json", &body);
    tree.write("adapters/claude.json", &claude_body());
    let message = tree.library_error();
    assert!(
        message.contains("'efforts' names an effort for 'haiku'"),
        "{message}"
    );
    assert!(message.contains("opus, sonnet"), "{message}");
}

/// The operator's own example, run: a tool-restricted agent whose chain
/// reaches providers that cannot express restrictions does not compile,
/// and the message tells the reader exactly which link is the problem.
#[test]
fn the_operators_literal_chain_fails_for_a_tool_restricted_agent() {
    let tree = Tree::new();
    let mut body = agent_body();
    body["models"] = json!(["fable", "qwen-max", "gpt-sol"]);
    body["efforts"] = json!({"fable": "high", "qwen-max": "high", "gpt-sol": "high"});
    tree.write("agents/chief-architect.json", &body);
    let mut claude = claude_body();
    claude["models"] = json!({"fable": "claude-fable-5"});
    tree.write("adapters/claude.json", &claude);
    tree.write(
        "adapters/dsh.json",
        &json!({
            "provider": "dsh",
            "binary": "dsh",
            "driver": ["{brokkr}", "driver", "dsh", "--"],
            "models": {"qwen-max": "qwen-max"},
            "model_flag": "--model",
            "efforts": ["low", "medium", "high"],
            "effort_flag": "--effort",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        }),
    );
    let message = refusal(&tree, "chief-architect");
    assert!(message.contains("agent 'chief-architect'"), "{message}");
    assert!(message.contains("provider 'dsh'"), "{message}");
    assert!(message.contains("MORE power"), "{message}");
}

// ------------------------------------------------------------ mapping

/// AC-1: a model no adapter maps refuses, naming the model and the
/// adapter files a reader must edit.
#[test]
fn a_model_no_adapter_maps_names_the_files_consulted() {
    let tree = Tree::new();
    tree.write("agents/tester.json", &agent_body());
    let mut claude = claude_body();
    claude["models"] = json!({"opus": "claude-opus-5"});
    tree.write("adapters/claude.json", &claude);
    let message = refusal(&tree, "tester");
    assert!(message.contains("model 'sonnet'"), "{message}");
    assert!(message.contains("claude.json"), "{message}");
}

/// AC-1: one model name, one provider — a duplicate mapping names both
/// files rather than inventing a tiebreak.
#[test]
fn a_model_mapped_by_two_adapters_names_both_files() {
    let tree = ready();
    let mut second = claude_body();
    second["provider"] = json!("lanetally");
    second["binary"] = json!("claude-lanetally");
    tree.write("adapters/lanetally.json", &second);
    let message = tree.adapters_error();
    assert!(message.contains("mapped by two adapters"), "{message}");
    assert!(message.contains("claude.json"), "{message}");
    assert!(message.contains("lanetally.json"), "{message}");
}

#[test]
fn an_unknown_agent_names_the_known_set() {
    let tree = ready();
    let message = refusal(&tree, "nobody");
    assert!(
        message.contains("agent 'nobody' is not in the library"),
        "{message}"
    );
    assert!(message.contains("tester"), "{message}");
}

// ------------------------------------------------------- availability

/// AC-1: a later entry is chosen only when an earlier one is unavailable,
/// and the skip is recorded with a closed-vocabulary reason.
#[test]
fn an_unavailable_provider_is_skipped_and_recorded() {
    let tree = Tree::new();
    tree.write("agents/tester.json", &agent_body());
    let mut claude = claude_body();
    claude["models"] = json!({"opus": "claude-opus-5"});
    tree.write("adapters/claude.json", &claude);
    let mut second = claude_body();
    second["provider"] = json!("lanetally");
    second["binary"] = json!("claude-lanetally");
    second["models"] = json!({"sonnet": "claude-sonnet-5"});
    tree.write("adapters/lanetally.json", &second);

    let mut availability = Availability::unspecified();
    availability.record("claude", Presence::Unavailable);
    availability.record("lanetally", Presence::Available);
    let resolution = resolved(&tree, &availability);
    assert_eq!(resolution.record["chosen_index"], 1);
    assert_eq!(resolution.record["model"], "sonnet");
    assert_eq!(resolution.record["provider"], "lanetally");
    assert_eq!(
        resolution.record["skipped"],
        json!([{"model": "opus", "reason": "unavailable"}])
    );
    assert_eq!(resolution.candidates.len(), 1);

    availability.record("lanetally", Presence::Unavailable);
    let message = resolve(&tree.library(), &tree.adapters(), &availability, "tester")
        .unwrap_err()
        .to_string();
    assert!(message.contains("no available candidate"), "{message}");
    assert!(message.contains("opus, sonnet"), "{message}");
}

/// The `report` half: an unmapped or blocked entry is reported rather
/// than thrown, so `brokkr agents show` can print the whole chain.
#[test]
fn report_walks_the_whole_chain_without_refusing() {
    let tree = Tree::new();
    tree.write("agents/tester.json", &agent_body());
    let mut claude = claude_body();
    claude["models"] = json!({"opus": "claude-opus-5"});
    claude["tool_permissions"]["names"] = json!({"cargo": "Bash(cargo:*)"});
    tree.write("adapters/claude.json", &claude);
    let walked = report(
        &tree.library(),
        &tree.adapters(),
        &Availability::unspecified(),
        "tester",
    )
    .unwrap();
    assert_eq!(walked.entries.len(), 2);
    assert_eq!(walked.entries[0].provider.as_deref(), Some("claude"));
    assert!(walked.entries[0].gap.is_some());
    assert!(walked.entries[1].provider.is_none());
    // Blocked, but mapped, so still the chosen entry a readout shows.
    assert_eq!(walked.chosen, Some(0));
    assert!(report(
        &tree.library(),
        &tree.adapters(),
        &Availability::unspecified(),
        "nobody"
    )
    .is_err());
}

// -------------------------------------------------------- the record

/// T8: the record carries names and digests only — never argv, whose
/// `{brokkr}` expansion is a machine-local absolute path.
#[test]
fn the_record_carries_names_and_digests_and_moves_with_its_inputs() {
    let tree = ready();
    let first = resolved(&tree, &Availability::unspecified());
    let record = first.record.as_object().unwrap();
    assert_eq!(
        record.keys().cloned().collect::<Vec<_>>(),
        vec![
            "adapter_digest",
            "agent",
            "agent_digest",
            "chain",
            "charter_digest",
            "chosen_index",
            "model",
            "notices",
            "provider",
            "skipped",
        ]
    );
    assert_eq!(record["chain"], json!(["opus", "sonnet"]));
    assert!(!serde_json::to_string(&first.record)
        .unwrap()
        .contains("{brokkr}"));

    std::fs::write(tree.library_root().join("charters/c.md"), "# changed\n").unwrap();
    let after_charter = resolved(&tree, &Availability::unspecified());
    assert_ne!(
        record["charter_digest"],
        after_charter.record["charter_digest"]
    );
    assert_eq!(
        record["adapter_digest"],
        after_charter.record["adapter_digest"]
    );

    let mut adapter = claude_body();
    adapter["binary"] = json!("claude-2");
    tree.write("adapters/claude.json", &adapter);
    let after_adapter = resolved(&tree, &Availability::unspecified());
    assert_ne!(
        after_charter.record["adapter_digest"],
        after_adapter.record["adapter_digest"]
    );
}

// ------------------------------------------------------- strict parsing

/// T5/AC-20: every rejection names the file and the key, so an operator
/// can act on it without reading the loader.
#[test]
fn the_library_loader_names_the_file_and_the_key_it_refuses() {
    let cases: Vec<(Value, &str)> = vec![
        (json!([]), "must be a JSON object"),
        (json!({"invented": 1}), "unknown key 'invented'"),
        (json!({"description": ""}), "non-empty string 'description'"),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": "x"}),
            "'models' as an array of strings",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": [1]}),
            "'models' must hold strings only",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": []}),
            "'models' is empty",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["Opus"]}),
            "does not match ^[a-z][a-z0-9-]*$",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "tools": []}),
            "'tools' must be a JSON object",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "tools": {"invented": 1}}),
            "'tools' has unknown key 'invented'",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "tools": {"allow": []}}),
            "ambiguous between 'no restriction' and 'restrict to nothing'",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "tools": {"allow": ["Cargo"]}}),
            "'tools.allow' names 'Cargo'",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "tools": {"mcp": "no"}}),
            "'tools.mcp' must be an array",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "tools": {"mcp": [{"invented": 1}]}}),
            "'tools.mcp' entry has unknown key",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "tools": {"mcp": [{"server": "GitHub"}]}}),
            "'tools.mcp.server' names 'GitHub'",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "limits": []}),
            "'limits' must be a JSON object",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "limits": {"invented": 1}}),
            "'limits' has unknown key",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "limits": {"max_attempts": 0}}),
            "'limits.max_attempts' must be an integer >= 1",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "inputs": [1]}),
            "'inputs' must hold strings only",
        ),
        (
            json!({"description": "d", "charter": "../escape.md", "models": ["opus"]}),
            "charter '../escape.md'",
        ),
    ];
    for (body, expected) in cases {
        let tree = Tree::new();
        tree.write("agents/tester.json", &body);
        let message = tree.library_error();
        assert!(
            message.contains(expected),
            "expected {expected:?} in {message:?}"
        );
        assert!(message.contains("tester"), "{message}");
    }
}

/// A charter that escapes the library root is refused even when it
/// exists: containment is checked after canonicalisation.
#[test]
fn a_charter_outside_the_library_root_is_refused() {
    let tree = Tree::new();
    std::fs::write(tree.dir.path().join("escape.md"), "# outside\n").unwrap();
    let mut body = agent_body();
    body["charter"] = json!("../escape.md");
    tree.write("agents/tester.json", &body);
    let message = tree.library_error();
    assert!(message.contains("outside the library root"), "{message}");
}

#[test]
fn a_file_name_outside_the_grammar_is_refused() {
    let tree = Tree::new();
    tree.write("agents/Tester.json", &agent_body());
    let message = tree.library_error();
    assert!(message.contains("Tester"), "{message}");
    assert!(
        message.contains("file name must match ^[a-z][a-z0-9-]*$"),
        "{message}"
    );

    let tree = Tree::new();
    tree.write("agents/tester.json", &agent_body());
    tree.write("adapters/Claude.json", &claude_body());
    assert!(tree.adapters_error().contains("file name must match"));
}

#[test]
fn unparseable_and_missing_trees_are_refused_by_name() {
    let tree = Tree::new();
    tree.raw("agents/tester.json", "{not json");
    assert!(tree.library_error().contains("tester.json"));

    let missing = Tree::new();
    let message = Library::load(&missing.dir.path().join("absent"))
        .unwrap_err()
        .to_string();
    assert!(message.contains("agent library"), "{message}");
    let message = Adapters::load(&missing.dir.path().join("absent"))
        .unwrap_err()
        .to_string();
    assert!(message.contains("adapters"), "{message}");
}

/// AC-11: a secrets store anywhere under either tree is refused, exactly
/// as `manifest_for` refuses one inside a bundle.
#[test]
fn a_secrets_store_in_either_tree_is_refused() {
    let tree = ready();
    std::fs::write(tree.library_root().join("charters/secrets.env"), "T=v\n").unwrap();
    let message = tree.library_error();
    assert!(
        message.contains("agent library tree contains a secrets store"),
        "{message}"
    );
    std::fs::remove_file(tree.library_root().join("charters/secrets.env")).unwrap();

    std::fs::write(tree.adapters_root().join("secrets.env"), "T=v\n").unwrap();
    assert!(tree
        .adapters_error()
        .contains("adapters tree contains a secrets store"));
}

/// A file that is not `.json` is not a definition; only `.json` files
/// are read, so a README beside the library is not a broken agent.
#[test]
fn non_json_files_are_not_definitions() {
    let tree = ready();
    std::fs::write(tree.library_root().join("README.md"), "# library\n").unwrap();
    assert_eq!(tree.library().names(), vec!["tester".to_string()]);
}

/// `scan` warns; `load` refuses. The listing contract is *warn on a
/// broken entry, never abort*, and the compiler's contract is the
/// opposite, so both exist and neither is a flag on the other.
#[test]
fn scan_collects_problems_where_load_refuses_them() {
    let tree = ready();
    tree.raw("agents/broken.json", "{");
    let (library, problems) = Library::scan(&tree.library_root()).unwrap();
    assert_eq!(library.names(), vec!["tester".to_string()]);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("broken.json"));
    assert!(Library::load(&tree.library_root()).is_err());
}

#[test]
fn the_adapter_loader_names_the_file_and_the_key_it_refuses() {
    let cases: Vec<(Value, &str)> = vec![
        (
            json!({"provider": "other"}),
            "the file name is the provider name",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": []}),
            "'driver' is empty",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {"Opus": "x"}}),
            "'models' names 'Opus'",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {"opus": ""}}),
            "'models.opus' must be a non-empty string",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": "no"}),
            "'models' as an object of strings",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "maybe"}),
            "the only legal string here is \"unsupported\"",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}}),
            "needs 'tool_permissions'",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": {"invented": 1}}),
            "'tool_permissions' has unknown key",
        ),
        // A flag with no separator cannot join two names — the argv it
        // would compose is a guess about the provider's grammar, which
        // is the one thing an adapter file may never be.
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": {"flag": "--allowedTools",
                   "names": {"cargo": "Bash(cargo:*)"}}}),
            "'tool_permissions' needs a non-empty string 'separator'",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": {"invented": 1}}),
            "'mcp' has unknown key",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported"}),
            "needs 'model_flag'",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": ""}),
            "needs 'model_flag'",
        ),
        // Decision 0035 ruling 5's half of the same rule: a provider
        // that declares no effort vocabulary would silently excuse every
        // seat it serves from the pin, so the declaration is required —
        // empty for an effortless provider, never absent.
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "--model"}),
            "needs 'efforts' as an array of strings",
        ),
        // Decision 0036's data, refused in the same style and for the
        // same reason: a misspelled class must never read as
        // `uncontracted` by accident, or a route the operator believes
        // they placed would silently not have been placed.
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "-m",
                   "efforts": [], "effort_flag": "unsupported",
                   "egress": "lokal"}),
            "'egress' is \"lokal\"; the egress vocabulary is closed",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "--model",
                   "efforts": ["Xhigh"]}),
            "'efforts' names 'Xhigh'",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "-m",
                   "efforts": [], "effort_flag": "unsupported",
                   "egress": "local", "binding_grant": true}),
            "declares both 'egress' and the superseded 'binding_grant'",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "--model",
                   "efforts": ["high"]}),
            "needs 'effort_flag'",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "-m",
                   "efforts": [], "effort_flag": "unsupported",
                   "routes": ["spark"]}),
            "'routes' must be an object of route name → egress class",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "-m",
                   "efforts": [], "effort_flag": "unsupported",
                   "routes": {"us/east": "local"}}),
            "'routes' names 'us/east', which does not match ^[A-Za-z0-9._:-]+$",
        ),
        // A route with no name at all names no prefix `resolve_route`
        // can produce either.
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "-m",
                   "efforts": [], "effort_flag": "unsupported",
                   "routes": {"": "local"}}),
            "'routes' names '', which does not match ^[A-Za-z0-9._:-]+$",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "-m",
                   "efforts": [], "effort_flag": "unsupported",
                   "routes": {"spark": "trusted"}}),
            "'routes.spark' is \"trusted\"",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "-m",
                   "efforts": [], "effort_flag": "unsupported",
                   "credentials": "SPARK_API_KEY"}),
            "'credentials' must be an object of route name → environment",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "-m",
                   "efforts": [], "effort_flag": "unsupported",
                   "credentials": {"us/east": "SPARK_API_KEY"}}),
            "'credentials' names 'us/east', which does not match \
             ^[A-Za-z0-9._:-]+$",
        ),
        // A credential is a NAME. A value-shaped one is refused where it
        // is written, beside the store refusal that guards the tree.
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "-m",
                   "efforts": [], "effort_flag": "unsupported",
                   "credentials": {"spark": "sk-live-not-a-name"}}),
            "'credentials.spark' is \"sk-live-not-a-name\"",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "-m",
                   "efforts": [], "effort_flag": "unsupported",
                   "credentials": {"spark": "SPARK-API-KEY"}}),
            "'credentials.spark' is \"SPARK-API-KEY\"",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "-m",
                   "efforts": [], "effort_flag": "unsupported",
                   "credentials": {"spark": 7}}),
            "^[A-Z][A-Z0-9_]*$",
        ),
    ];
    for (body, expected) in cases {
        let tree = Tree::new();
        tree.write("adapters/claude.json", &body);
        let message = tree.adapters_error();
        assert!(
            message.contains(expected),
            "expected {expected:?} in {message:?}"
        );
        assert!(message.contains("claude.json"), "{message}");
    }
}

// ------------------------------------------------- decision 0036: routes

/// One provider fronting three destinations — the `dsh` shape, written
/// as a fixture so no vendor's name is load-bearing. The adapter's own
/// class is `uncontracted`, one route is ruled `local` and one
/// `contracted`, and a third route is mapped but never declared.
fn many_routes() -> Value {
    json!({
        "provider": "many",
        "efforts": [],
        "effort_flag": "unsupported",
        "binary": "many",
        "driver": ["{brokkr}", "driver", "many", "--"],
        "egress": "uncontracted",
        "routes": {"nearby": "local", "partner": "contracted"},
        "credentials": {"nearby": "SPARK2_API_KEY"},
        "models": {
            "near": "nearby/small-1",
            "far": "partner/large-1",
            "unruled": "elsewhere/large-1",
            "bare": "large-1",
        },
        "model_flag": "--model",
        "tool_permissions": "unsupported",
        "mcp": "unsupported",
    })
}

/// Ruling 2, case by case: a PREFIXED id resolves to its route's
/// declared class; an UNDECLARED route falls to `uncontracted`, because
/// ruling 1 makes an absent declaration uncontracted; an UNPREFIXED id
/// resolves to the adapter's own and NO BETTER, because it reaches
/// whatever default the harness profile resolves and the machine cannot
/// know where that is.
#[test]
fn a_route_prefix_resolves_a_class_and_an_unprefixed_id_inherits_nothing_better() {
    let tree = Tree::new();
    tree.write("adapters/many.json", &many_routes());
    let adapters = tree.adapters();
    let many = adapters.adapter("many").expect("the fixture provider");

    // Prefixed, declared: the route's own class, better AND worse than
    // the adapter's — the whole point of moving the declaration site.
    assert_eq!(
        resolve_route(many, "nearby/small-1"),
        (Some("nearby"), EgressClass::Local)
    );
    assert_eq!(
        resolve_route(many, "partner/large-1"),
        (Some("partner"), EgressClass::Contracted)
    );

    // Prefixed, undeclared: named as a route this file never places, so
    // uncontracted — ruling 1's "everything else, and the value of an
    // absent declaration". Silence about a route is not a promotion, and
    // it is not an inheritance either.
    assert_eq!(
        resolve_route(many, "elsewhere/large-1"),
        (Some("elsewhere"), EgressClass::Uncontracted)
    );

    // Unprefixed: the adapter's own class, and no better. A `local`
    // route on the same adapter lends it nothing.
    assert_eq!(resolve_route(many, "large-1"), (None, many.egress));
    let (_, bare) = resolve_route(many, "large-1");
    let (_, best) = resolve_route(many, "nearby/small-1");
    assert!(
        bare < best,
        "an unprefixed id must not inherit a better class than the adapter's own"
    );
    assert_eq!(bare, EgressClass::Uncontracted);

    // The vocabulary is closed and ordered, and the order is what
    // "meets a minimum" reads.
    assert_eq!(EgressClass::parse("local"), Some(EgressClass::Local));
    assert_eq!(
        EgressClass::parse("contracted"),
        Some(EgressClass::Contracted)
    );
    assert_eq!(
        EgressClass::parse("uncontracted"),
        Some(EgressClass::Uncontracted)
    );
    assert_eq!(EgressClass::parse("trusted"), None);
    assert!(EgressClass::Local > EgressClass::Contracted);
    assert!(EgressClass::Contracted > EgressClass::Uncontracted);
    for class in [
        EgressClass::Local,
        EgressClass::Contracted,
        EgressClass::Uncontracted,
    ] {
        assert_eq!(EgressClass::parse(class.name()), Some(class));
    }
    assert_eq!(many.credentials["nearby"], "SPARK2_API_KEY");
}

/// Ruling 2's undeclared-route case, proved where it can actually be
/// wrong: an adapter whose OWN destination the operator has ruled
/// acceptable. Every other test's fixture is uncontracted at the
/// adapter, so a fail-open there is invisible — the two readings agree
/// on the floor.
///
/// This is the shape of the decision's first rejected alternative,
/// verbatim: one binary, one ruling, three destinations. If a class
/// declared for the endpoint the file names leaked onto endpoints it
/// does not name, then "granting `dsh` the binding grant clears the
/// Alibaba and DeepSeek routes at the same stroke" would be true again
/// through the routes map instead of through the boolean, and the
/// decision would have moved the fail-open rather than closed it.
#[test]
fn a_contracted_adapter_clears_no_route_it_does_not_name() {
    let tree = Tree::new();
    let mut ruled = many_routes();
    // The operator has ruled THIS adapter's own default destination
    // acceptable, and said nothing whatever about `elsewhere`.
    ruled["egress"] = json!("contracted");
    tree.write("adapters/many.json", &ruled);
    let adapters = tree.adapters();
    let many = adapters.adapter("many").expect("the fixture provider");

    // The destination the operator ruled: contracted, as ruled.
    assert_eq!(
        resolve_route(many, "large-1"),
        (None, EgressClass::Contracted)
    );
    // A destination they did not rule, reached by the same binary: the
    // floor, and not one step of the adapter's own clearance.
    assert_eq!(
        resolve_route(many, "elsewhere/large-1"),
        (Some("elsewhere"), EgressClass::Uncontracted)
    );
    let (_, unruled) = resolve_route(many, "elsewhere/large-1");
    assert!(
        unruled < many.egress,
        "a route the adapter does not name must not inherit its clearance"
    );
    // And the routes it DOES name still stand on their own words, above
    // and below the adapter's — the reason the declaration moved here.
    assert_eq!(resolve_route(many, "nearby/small-1").1, EgressClass::Local);
    assert_eq!(
        resolve_route(many, "partner/large-1").1,
        EgressClass::Contracted
    );
}

/// Decision 0040 ruling 5: a route name is whatever a model id may begin
/// with. `resolve_route` splits a concrete id on its first `/`, and the
/// id alphabet admits `.`, `_`, `:` and upper case in that prefix — so
/// under the agent-name grammar a provider fronting `us.east` or
/// `openai_compat` had routes no operator could declare, which resolved
/// uncontracted forever with no data able to say otherwise. Ruling 1 of
/// decision 0036 makes class assignment operator DATA, and data that
/// cannot be written is not data.
#[test]
fn a_route_name_is_whatever_a_model_id_may_begin_with() {
    let tree = Tree::new();
    let mut regions = many_routes();
    regions["routes"] = json!({
        "us.east": "contracted",
        "openai_compat": "local",
        "eu:west-1": "contracted",
    });
    regions["credentials"] = json!({
        "us.east": "US_EAST_API_KEY",
        "openai_compat": "COMPAT_API_KEY",
    });
    regions["models"] = json!({"near": "openai_compat/small-1"});
    tree.write("adapters/many.json", &regions);
    let adapters = tree.adapters();
    let many = adapters.adapter("many").expect("the fixture provider");

    // Each declared route resolves through a prefixed pin, on its own
    // declared class — the thing that was unstatable before.
    assert_eq!(
        resolve_route(many, "us.east/large-1"),
        (Some("us.east"), EgressClass::Contracted)
    );
    assert_eq!(
        resolve_route(many, "openai_compat/small-1"),
        (Some("openai_compat"), EgressClass::Local)
    );
    assert_eq!(
        resolve_route(many, "eu:west-1/large-1"),
        (Some("eu:west-1"), EgressClass::Contracted)
    );
    assert_eq!(many.credentials["us.east"], "US_EAST_API_KEY");
    assert_eq!(many.credentials["openai_compat"], "COMPAT_API_KEY");

    // And a route the file still does not name is still the floor: the
    // alphabet widened, the asymmetry did not move.
    assert_eq!(
        resolve_route(many, "us.west/large-1"),
        (Some("us.west"), EgressClass::Uncontracted)
    );

    // The `/` is the one character a route may never hold, because it is
    // what separates the prefix from the rest of the id: a key carrying
    // one names a route `resolve_route` could never produce, and is
    // refused on both maps rather than sitting there matching nothing.
    for map in ["routes", "credentials"] {
        let tree = Tree::new();
        let mut split = many_routes();
        split[map] = match map {
            "routes" => json!({"us/east": "contracted"}),
            _ => json!({"us/east": "US_EAST_API_KEY"}),
        };
        tree.write("adapters/many.json", &split);
        let message = tree.adapters_error();
        assert!(
            message.contains(&format!("'{map}' names 'us/east'")),
            "{message}"
        );
        assert!(message.contains("^[A-Za-z0-9._:-]+$"), "{message}");
    }

    // The agent-name grammar is untouched, and still governs everything
    // it governed: agents, adapters and abstract model names.
    assert_eq!(NAME_GRAMMAR, "^[a-z][a-z0-9-]*$");
    assert!(!valid_name("us.east"));
    assert!(!valid_name("openai_compat"));
}

/// The migration, at the loader: the superseded `binding_grant` still
/// READS, and reads as exactly what decision 0036 ruling 4 says it does,
/// so no adapter file on disk is forced to change this release.
#[test]
fn the_superseded_grant_still_reads_as_a_class() {
    let tree = Tree::new();
    let mut granted = claude_body();
    granted["binding_grant"] = json!(true);
    tree.write("adapters/claude.json", &granted);
    assert_eq!(
        tree.adapters().adapter("claude").unwrap().egress,
        EgressClass::Contracted
    );

    let mut refused = claude_body();
    refused["binding_grant"] = json!(false);
    tree.write("adapters/claude.json", &refused);
    assert_eq!(
        tree.adapters().adapter("claude").unwrap().egress,
        EgressClass::Uncontracted
    );

    // Absent on both keys: uncontracted, and no routes at all — the
    // shape of an adapter that fronts a single destination.
    tree.write("adapters/claude.json", &claude_body());
    let adapters = tree.adapters();
    let claude = adapters.adapter("claude").unwrap();
    assert_eq!(claude.egress, EgressClass::Uncontracted);
    assert!(claude.routes.is_empty());
    assert!(claude.credentials.is_empty());
    assert_eq!(resolve_route(claude, "claude-opus-5").1, claude.egress);
}

/// AC-9's data half: the degenerate honest adapter. `exec` declares all
/// three capabilities unsupported and maps no model, so nothing can
/// silently select it.
#[test]
fn a_provider_may_declare_every_capability_unsupported() {
    let tree = Tree::new();
    tree.write(
        "adapters/exec.json",
        &json!({
            "provider": "exec",
            "binary": "sh",
            "driver": ["{brokkr}", "driver", "exec", "--"],
            "models": {},
            "model_flag": "unsupported",
            "efforts": [],
            "effort_flag": "unsupported",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        }),
    );
    let adapters = tree.adapters();
    let exec = adapters.providers().next().unwrap();
    assert!(exec.model_flag.is_none());
    assert!(exec.tool_permissions.is_none());
    assert!(exec.mcp.is_none());
    assert!(adapters.serving("opus").is_none());
    assert_eq!(adapters.digest("nobody"), None);
    assert!(adapters.digest("exec").is_some());
    assert_eq!(adapters.files().len(), 1);
}

/// The library exposes what `brokkr agents list` prints, in name order.
#[test]
fn the_library_lists_its_agents_in_name_order() {
    let tree = ready();
    let mut second = agent_body();
    second["description"] = json!("another");
    tree.write("agents/another.json", &second);
    let library = tree.library();
    assert_eq!(
        library
            .agents()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        vec!["another", "tester"]
    );
    assert_eq!(library.agent("tester").unwrap().description, "a test agent");
    assert!(library.agent("nobody").is_none());
}

/// An agent may declare 0007 inputs; none of the shipped sixteen does,
/// because the 0007 default already names each phase's referenced set.
#[test]
fn an_agent_may_declare_its_own_inputs() {
    let tree = Tree::new();
    let mut body = agent_body();
    body["inputs"] = json!(["fixes_applied"]);
    tree.write("agents/tester.json", &body);
    tree.write("adapters/claude.json", &claude_body());
    let resolution = resolved(&tree, &Availability::unspecified());
    assert_eq!(resolution.inputs, Some(vec!["fixes_applied".to_string()]));
}

/// An IO problem reading a definition's charter is not a "broken entry"
/// the listing warns about — it is an environment failure, and it
/// propagates rather than being folded into the per-file warnings.
#[test]
fn an_io_failure_propagates_rather_than_becoming_a_warning() {
    let tree = Tree::new();
    let mut body = agent_body();
    // A directory canonicalises and is contained, and then does not read.
    body["charter"] = json!("charters");
    tree.write("agents/tester.json", &body);
    let message = tree.library_error();
    assert!(message.contains("agent library io"), "{message}");
    assert!(Library::scan(&tree.library_root()).is_err());
}

/// A top-level key nobody knows is refused on an adapter exactly as on
/// an agent: a misspelled capability must not read as silence.
#[test]
fn an_unknown_top_level_adapter_key_is_refused() {
    let tree = Tree::new();
    let mut adapter = claude_body();
    adapter["tool_permisions"] = json!("unsupported");
    tree.write("adapters/claude.json", &adapter);
    let message = tree.adapters_error();
    assert!(
        message.contains("unknown key 'tool_permisions'"),
        "{message}"
    );
}

/// Selection SKIPS an unmapped entry rather than stopping at it, so a
/// readout can show a chain whose first link is not mapped yet and still
/// name the link that would run.
#[test]
fn selection_skips_an_unmapped_first_entry() {
    let tree = Tree::new();
    let mut body = agent_body();
    body["models"] = json!(["nowhere", "opus"]);
    body["efforts"] = json!({"nowhere": "high", "opus": "high"});
    tree.write("agents/tester.json", &body);
    tree.write("adapters/claude.json", &claude_body());
    let walked = report(
        &tree.library(),
        &tree.adapters(),
        &Availability::unspecified(),
        "tester",
    )
    .unwrap();
    assert!(walked.entries[0].provider.is_none());
    assert_eq!(walked.chosen, Some(1));
    // The compiler is stricter than the readout: an unmapped name is a
    // refusal there, because 0016 validates that a mapping EXISTS.
    assert!(refusal(&tree, "tester").contains("model 'nowhere'"));
}
