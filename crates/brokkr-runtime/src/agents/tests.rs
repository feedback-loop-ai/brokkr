use super::*;
use std::path::Path;

/// A throwaway library + adapters tree. Every test writes exactly the
/// data it is about, so a rejection message can be asserted verbatim.
///
/// The tree is REACHED THROUGH A SYMLINK on every host, and `root` is that
/// alias canonicalised once, here. The loaders canonicalise what they are
/// given, so a diagnostic names the canonical place; macOS hands out
/// temporary directories under `/var`, itself a link to `/private/var`,
/// and an expectation glued from the lexical path passes on Linux and
/// fails there. With the alias the same habit fails on Linux too. Every
/// write and every expected path is derived from `root`.
struct Tree {
    /// Held for its drop: the directory lives as long as the fixture.
    _guard: tempfile::TempDir,
    root: PathBuf,
}

impl Tree {
    fn new() -> Tree {
        let guard = tempfile::tempdir().unwrap();
        std::fs::create_dir(guard.path().join("real")).unwrap();
        let alias = guard.path().join("alias");
        std::os::unix::fs::symlink("real", &alias).unwrap();
        let tree = Tree {
            root: alias.canonicalize().unwrap(),
            _guard: guard,
        };
        std::fs::create_dir_all(tree.library_root().join("charters")).unwrap();
        std::fs::create_dir_all(tree.adapters_root()).unwrap();
        std::fs::write(tree.library_root().join("charters/c.md"), "# charter\n").unwrap();
        tree
    }

    fn library_root(&self) -> PathBuf {
        self.root.join("agents")
    }

    fn adapters_root(&self) -> PathBuf {
        self.root.join("adapters")
    }

    fn write(&self, relative: &str, body: &Value) {
        std::fs::write(
            self.root.join(relative),
            serde_json::to_vec_pretty(body).unwrap(),
        )
        .unwrap();
    }

    fn raw(&self, relative: &str, body: &str) {
        std::fs::write(self.root.join(relative), body).unwrap();
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

/// Decision 0065 ruling 3: an agent requests and only a realm grants, so
/// an agent no longer NAMES an MCP server — required or optional, served
/// by its adapter or not. The legacy list is refused with the migration,
/// and no server reaches a command line from agent data; the empty list
/// every shipped agent writes stays valid and composes nothing.
#[test]
fn an_agent_naming_an_mcp_server_is_refused_and_never_reaches_a_command_line() {
    for need in [
        json!([{"server": "github"}]),
        json!([{"server": "github", "optional": true}]),
        json!("github"),
    ] {
        let tree = Tree::new();
        let mut body = agent_body();
        body["tools"]["mcp"] = need;
        tree.write("agents/tester.json", &body);
        // The adapter DOES map the server: the legacy map is no authority.
        tree.write("adapters/claude.json", &claude_body());
        // The WHOLE migration reason, agent and file named (SC7): a suffix
        // would pass on a refusal that blamed the wrong definition.
        let file = tree
            .library_root()
            .canonicalize()
            .unwrap()
            .join("tester.json");
        assert_eq!(
            tree.library_error(),
            format!(
                "agent 'tester' ({}) 'tools.mcp' names an MCP server; an agent no longer names \
                 one, because a server an office could name would be a door a pulled bundle \
                 could open. Request the capability by abstract name under 'capabilities' \
                 (\"requires\" or \"wants\") and let realms.json grant it through a tool dialect \
                 (decision 0065 rulings 1 and 3)",
                file.display()
            )
        );
    }
    let tree = ready();
    let resolution = resolved(&tree, &Availability::unspecified());
    assert!(!resolution.candidates[0]
        .argv
        .iter()
        .any(|part| part == "--mcp-config"));
    assert!(resolution.notices.is_empty());
    assert_eq!(resolution.record["notices"], json!([]));
}

/// An agent asks for capabilities by ABSTRACT name, and the loader holds
/// the map to its two-word vocabulary; an absent map asks for nothing.
#[test]
fn an_agent_requests_capabilities_by_abstract_name() {
    let tree = Tree::new();
    let mut body = agent_body();
    body["capabilities"] = json!({"web-fetch": "wants", "library-docs": "requires"});
    tree.write("agents/tester.json", &body);
    let library = tree.library();
    let asks = &library.agent("tester").unwrap().capabilities;
    assert_eq!(asks["web-fetch"], crate::capabilities::Strength::Wants);
    assert_eq!(
        asks["library-docs"],
        crate::capabilities::Strength::Requires
    );

    body["capabilities"] = json!({"web-fetch": {"dialect": "fetch-mcp"}});
    tree.write("agents/tester.json", &body);
    assert_eq!(
        tree.library_error(),
        "agent 'tester' requests capability 'web-fetch' as {\"dialect\":\"fetch-mcp\"}; a \
         request is \"requires\" or \"wants\" and nothing else — a dialect, a tool list, a \
         class or a grant belongs to realms.json and the operator's definitions (decision 0065 \
         ruling 3)"
    );
    assert!(ready()
        .library()
        .agent("tester")
        .unwrap()
        .capabilities
        .is_empty());
}

/// Finding M2: an agent's requests are read from its SOURCE BYTES. An
/// ordinary JSON map keeps the last copy of a repeated key, so
/// `"web-search": "requires"` followed by `"web-search": "wants"` loaded as
/// a want — a requirement weakened before any validation saw it. Every
/// repetition is refused where the file is read: either strength order, an
/// equal repetition, and a second `capabilities` field, a later `{}`
/// included. The fixtures are raw text, because `json!` would erase the
/// duplicate before the reader met it.
#[test]
fn a_request_key_written_twice_in_an_agent_source_is_refused_from_its_bytes() {
    let tree = Tree::new();
    let path = tree.library_root().join("tester.json");
    let agent = |capabilities: &str| {
        format!(
            "{{\"description\": \"a test agent\", \"charter\": \"charters/c.md\", \
             \"models\": [\"opus\"], \"efforts\": {{\"opus\": \"high\"}}, {capabilities}}}"
        )
    };
    // The control: the same document with each key once loads, and the
    // requirement is a requirement.
    tree.raw(
        "agents/tester.json",
        &agent(r#""capabilities": {"web-search": "requires"}"#),
    );
    assert_eq!(
        tree.library().agent("tester").unwrap().capabilities["web-search"],
        crate::capabilities::Strength::Requires
    );
    for (capabilities, key) in [
        (
            r#""capabilities": {"web-search": "requires", "web-search": "wants"}"#,
            "web-search",
        ),
        (
            r#""capabilities": {"web-search": "wants", "web-search": "requires"}"#,
            "web-search",
        ),
        (
            r#""capabilities": {"web-search": "requires", "web-search": "requires"}"#,
            "web-search",
        ),
        (
            r#""capabilities": {"web-search": "requires"}, "capabilities": {}"#,
            "capabilities",
        ),
        (
            r#""capabilities": {}, "capabilities": {"web-search": "requires"}"#,
            "capabilities",
        ),
    ] {
        let text = agent(capabilities);
        tree.raw("agents/tester.json", &text);
        // Each second copy is the LAST entry of its object, and the parser
        // closes that object before it reports: it stands past the inner
        // map's brace for a repeated name, past the document's own for a
        // repeated field.
        let column = match key {
            "capabilities" => text.len(),
            _ => text.len() - 1,
        };
        // What loading said, or what it loaded: a reader that keeps the last
        // copy fails HERE, showing the strength it kept.
        let said = match Library::load(&tree.library_root()) {
            Ok(library) => format!("loaded {:?}", library.agent("tester").unwrap().capabilities),
            Err(refusal) => refusal.to_string(),
        };
        assert_eq!(
            said,
            format!(
                "{}: key '{key}' is written twice at line 1 column {column}",
                path.display()
            ),
            "{capabilities}"
        );
    }
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

/// Decision 0065, no grandfathering (SC7): `websearch` in `tools.allow`
/// used to put `WebSearch` on the harness's allowed list — a second way to
/// hold a capability only the realm grants. An allow entry that maps to a
/// tool of one of the provider's NATIVE capabilities is refused with the
/// way out, never composed and never dropped in silence; the local command
/// entries beside it keep their meaning, and so does the same alias on a
/// provider whose inventory does not own the tool.
#[test]
fn a_legacy_allow_entry_cannot_authorize_a_native_capability() {
    let tree = Tree::new();
    let mut agent = agent_body();
    agent["tools"]["allow"] = json!(["cargo", "websearch"]);
    tree.write("agents/tester.json", &agent);
    let mut adapter = claude_body();
    adapter["tool_permissions"]["names"]["websearch"] = json!("WebSearch");
    tree.write("adapters/claude.json", &adapter);

    // An unmeasured inventory owns no tool name: the mapping is an
    // ordinary permission, exactly as before.
    let plain = resolved(&tree, &Availability::unspecified());
    assert!(
        plain.candidates[0]
            .argv
            .contains(&"Bash(cargo:*),WebSearch".to_string()),
        "{:?}",
        plain.candidates[0].argv
    );

    adapter["native_capabilities"] = json!({"known": {"web-search": {
        "capability": "web-search", "tools": ["WebSearch"],
        "on": {"selection": {"include": ["WebSearch"], "allow": ["WebSearch"], "deny": []}},
        "off": {"selection": {"include": [], "allow": [], "deny": ["WebSearch"]}},
        "restrictions": {"unsupported": "no native restriction transport is established"},
        "evidence": {"source": "adapter data", "scope": "declared", "limitations": []}}},
        "selection": {"include": {"flag": "--tools", "separator": ","},
                      "allow": {"flag": "--allowedTools", "separator": ","},
                      "deny": {"flag": "--disallowedTools", "separator": ","}}});
    tree.write("adapters/claude.json", &adapter);
    assert_eq!(
        refusal(&tree, "tester"),
        "agent 'tester' cannot be served by provider 'claude' on model 'opus': tool permission \
         'websearch' maps to 'WebSearch', a tool of the provider's native capability \
         'web-search'; a legacy allow entry cannot authorize a capability, so request \
         'web-search' by name under 'capabilities' and let the realm grant it through a tool \
         dialect (decision 0065 ruling 3). A capability the provider cannot express fails \
         compilation here rather than degrading silently at run time"
    );

    // The local entries alone still compose.
    agent["tools"]["allow"] = json!(["cargo"]);
    tree.write("agents/tester.json", &agent);
    let local = resolved(&tree, &Availability::unspecified());
    assert!(local.candidates[0]
        .argv
        .contains(&"Bash(cargo:*)".to_string()));
}

/// An adapter that declares native capabilities is authority data
/// (decision 0065 ruling 4): a key written twice anywhere in the file is
/// refused rather than read as its second copy, and a selection mapping
/// that cannot be composed is refused where the adapter loads — each
/// naming the adapter file and the place.
#[test]
fn a_native_declaration_with_a_repeated_key_or_an_uncomposable_selection_is_refused() {
    let tree = Tree::new();
    let native = |selection: Value| {
        let mut adapter = claude_body();
        adapter["native_capabilities"] = json!({"known": {"web-search": {
            "capability": "web-search", "tools": ["WebSearch"],
            "on": {"selection": {"include": ["WebSearch"], "allow": ["WebSearch"], "deny": []}},
            "off": {"selection": {"include": [], "allow": [], "deny": ["WebSearch"]}},
            "restrictions": {"unsupported": "no native restriction transport is established"},
            "evidence": {"source": "adapter data", "scope": "declared", "limitations": []}}},
            "selection": selection});
        adapter
    };
    let flags = json!({"include": {"flag": "--tools", "separator": ","},
                       "allow": {"flag": "--allowedTools", "separator": ","},
                       "deny": {"flag": "--disallowedTools", "separator": ","}});
    // Sound as written; then the same bytes with ONE key repeated, deep
    // inside the declaration. `serde_json` alone would keep the second.
    let sound = serde_json::to_string(&native(flags.clone())).unwrap();
    tree.raw("adapters/claude.json", &sound);
    tree.adapters();
    let repeated = sound.replacen(
        r#""scope":"declared""#,
        r#""scope":"declared","scope":"measured live""#,
        1,
    );
    assert_ne!(repeated, sound, "the fixture repeats a key");
    tree.raw("adapters/claude.json", &repeated);
    let what = format!(
        "adapter 'claude' ({})",
        tree.adapters_root().join("claude.json").display()
    );
    // The parser stands just past the second copy's value.
    let second = r#""scope":"measured live""#;
    let column = repeated.rfind(second).unwrap() + second.len();
    assert_eq!(
        tree.adapters_error(),
        format!("{what}: key 'scope' is written twice at line 1 column {column}")
    );
    // A list flag with no separator cannot be composed into one argument.
    let mut no_separator = flags.clone();
    no_separator["deny"]
        .as_object_mut()
        .unwrap()
        .remove("separator");
    tree.write("adapters/claude.json", &native(no_separator));
    assert_eq!(
        tree.adapters_error(),
        format!(
            "{what} 'native_capabilities' at '/selection/deny': it does not satisfy \
             '/definitions/list/required'"
        )
    );
    // Nor can a mapping that names a list the harness does not have.
    let mut unknown_list = flags;
    unknown_list["exclude"] = json!({"flag": "--exclude", "separator": ","});
    tree.write("adapters/claude.json", &native(unknown_list));
    assert_eq!(
        tree.adapters_error(),
        format!(
            "{what} 'native_capabilities' at '/selection': it does not satisfy \
             '/properties/selection/additionalProperties'"
        )
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
                   "tools": {"allow": null}}),
            "'tools' needs 'allow' as an array of strings",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "tools": {"allow": ["cargo", "cargo"]}}),
            "'tools.allow' names 'cargo' twice",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "tools": {"sandbox": "loose"}}),
            "'tools.sandbox' is 'loose'",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "tools": {"allow": ["Cargo"]}}),
            "'tools.allow' names 'Cargo'",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "tools": {"mcp": "no"}}),
            "'tools.mcp' names an MCP server",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "tools": {"mcp": [{"invented": 1}]}}),
            "'tools.mcp' names an MCP server",
        ),
        (
            json!({"description": "d", "charter": "charters/c.md", "models": ["opus"],
                   "capabilities": ["web-search"]}),
            "'capabilities' must be an object",
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
    std::fs::write(tree.root.join("escape.md"), "# outside\n").unwrap();
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
    let message = Library::load(&missing.root.join("absent"))
        .unwrap_err()
        .to_string();
    assert!(message.contains("agent library"), "{message}");
    let message = Adapters::load(&missing.root.join("absent"))
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
                   "models": {}, "judges": "no"}),
            "needs 'judges' as an array of strings",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "judges": ["Opus"]}),
            "'judges' names 'Opus'",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "judges": ["opus"]}),
            "declares judge 'opus' but maps no model",
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
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "--model",
                   "efforts": ["high"], "effort_flag": "--effort",
                   "effortless_routes": ["spark"]}),
            "'effortless_routes' must be an object of route name → measurement",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "--model",
                   "efforts": ["high"], "effort_flag": "--effort",
                   "effortless_routes": {"us/east": "measured 2026-09-11"}}),
            "'effortless_routes' names 'us/east', which does not match ^[A-Za-z0-9._:-]+$",
        ),
        (
            json!({"provider": "claude", "binary": "claude", "driver": ["x"],
                   "models": {}, "tool_permissions": "unsupported",
                   "mcp": "unsupported", "model_flag": "--model",
                   "efforts": ["high"], "effort_flag": "--effort",
                   "effortless_routes": {"spark": ""}}),
            "'effortless_routes.spark' must be a non-empty string naming the measurement",
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

/// Decision 0035 addendum 2026-09-11: absent is no effortless route,
/// and only a prefixed lane on a listed route claims the standing — a
/// bare id keeps the adapter default's, and an unlisted prefix is an
/// ordinary route that takes an effort.
#[test]
fn effortless_routes_default_to_empty_and_match_prefixed_lanes_only() {
    let bare = Tree::new();
    bare.write(
        "adapters/dsh.json",
        &json!({
            "provider": "dsh",
            "binary": "dsh",
            "driver": ["{brokkr}", "driver", "dsh", "--"],
            "models": {"spark-flash": "spark/qwen3.8-flash"},
            "model_flag": "--model",
            "efforts": ["low", "medium", "high", "xhigh"],
            "effort_flag": "--effort",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        }),
    );
    let adapter = bare.adapters().adapter("dsh").unwrap().clone();
    assert!(adapter.effortless_routes.is_empty());
    assert!(!route_is_effortless(&adapter, "spark/qwen3.8-flash"));

    let listed = Tree::new();
    listed.write(
        "adapters/dsh.json",
        &json!({
            "provider": "dsh",
            "binary": "dsh",
            "driver": ["{brokkr}", "driver", "dsh", "--"],
            "models": {"spark-flash": "spark/qwen3.8-flash"},
            "model_flag": "--model",
            "efforts": ["low", "medium", "high", "xhigh"],
            "effort_flag": "--effort",
            "effortless_routes": {"spark": "dsh 0.1.5-rc.1 refuses reasoningEffort at every level"},
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        }),
    );
    let adapter = listed.adapters().adapter("dsh").unwrap().clone();
    assert_eq!(adapter.effortless_routes.len(), 1);
    assert!(route_is_effortless(&adapter, "spark/qwen3.8-flash"));
    assert!(!route_is_effortless(&adapter, "deepseek-v4-flash"));
    assert!(!route_is_effortless(&adapter, "dashscope/qwen3.8-flash"));
}

/// The same standing through an abstract hire: a candidate resolving
/// to an effortless route needs no `efforts` entry, composes no
/// `--effort` flag, and carries no effort — the route, not the seat,
/// decides.
#[test]
fn a_candidate_on_an_effortless_route_needs_no_effort_entry() {
    let tree = Tree::new();
    let mut body = agent_body();
    body["models"] = json!(["spark-flash"]);
    body["efforts"] = json!({});
    // The dsh fixture declares tool_permissions unsupported, so the
    // agent must not restrict tools either — this test is about the
    // effort axis only.
    body.as_object_mut().unwrap().remove("tools");
    tree.write("agents/tester.json", &body);
    tree.write(
        "adapters/dsh.json",
        &json!({
            "provider": "dsh",
            "binary": "dsh",
            "driver": ["{brokkr}", "driver", "dsh", "--"],
            "models": {"spark-flash": "spark/qwen3.8-flash"},
            "model_flag": "--model",
            "efforts": ["low", "medium", "high", "xhigh"],
            "effort_flag": "--effort",
            "effortless_routes": {"spark": "dsh 0.1.5-rc.1 refuses reasoningEffort at every level"},
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        }),
    );
    let resolution = resolved(&tree, &Availability::unspecified());
    assert_eq!(resolution.candidates.len(), 1);
    let candidate = &resolution.candidates[0];
    assert_eq!(candidate.effort, None);
    assert!(
        !candidate.argv.iter().any(|part| part == "--effort"),
        "{:?}",
        candidate.argv
    );
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

// ------------------------------------------------ decision 0043: hands

fn boxed_agent() -> Value {
    let mut body = agent_body();
    body["hands"] = json!({"kind": "workspace", "network": false,
        "binds": [{"path": "~/.cargo", "mode": "rw", "mask": ["credentials.toml"]}]});
    body
}

/// Ruling 2: with hands, the tool list is not consulted and the adapter's
/// hands fragment is appended instead; without a fragment the provider
/// cannot serve the agent, and the refusal says why.
#[test]
fn hands_replace_the_tool_list_with_the_adapters_fragment() {
    let tree = Tree::new();
    tree.write("agents/tester.json", &boxed_agent());
    let mut claude = claude_body();
    claude["tool_permissions"] = json!("unsupported");
    claude["hands"] = json!({"workspace": ["--tools", "", "--mcp-config", "{hands_mcp_json}"]});
    tree.write("adapters/claude.json", &claude);
    let resolution = resolved(&tree, &Availability::unspecified());
    let argv = &resolution.candidates[0].argv;
    assert!(
        argv.iter().any(|part| part == "{hands_mcp_json}"),
        "{argv:?}"
    );
    assert!(
        !argv.iter().any(|part| part == "--allowedTools"),
        "{argv:?}"
    );
    assert_eq!(
        resolution.hands.as_ref().map(|hands| hands.binds.len()),
        Some(1)
    );

    let bare = Tree::new();
    bare.write("agents/tester.json", &boxed_agent());
    bare.write("adapters/claude.json", &claude_body());
    let refusal = refusal(&bare, "tester");
    assert!(refusal.contains("declares hands unsupported"), "{refusal}");
    assert!(refusal.contains("harness's own tools"), "{refusal}");

    let measured = Tree::new();
    measured.write("agents/tester.json", &boxed_agent());
    let mut reasoned = claude_body();
    reasoned["hands"] = json!({"unsupported": "no flag swaps the tool surface"});
    measured.write("adapters/claude.json", &reasoned);
    let refusal = self::refusal(&measured, "tester");
    assert!(
        refusal.contains("no flag swaps the tool surface"),
        "{refusal}"
    );
}

/// Issue #307, P1: a provider that CAN express the tool list still takes
/// none of it from an agent with hands. The fixture maps Cargo and Git
/// through the same `--allowedTools` spelling its workspace fragment uses
/// for the one MCP grant, so the two grant sources are told apart by
/// value and origin, not by flag name.
#[test]
fn hands_take_no_tool_grant_even_where_the_fragment_shares_the_tool_flag() {
    let fragment = [
        "--tools",
        "",
        "--strict-mcp-config",
        "--mcp-config",
        "{hands_mcp_json}",
        "--allowedTools",
        "mcp__brokkr__workspace",
    ];
    let tree = Tree::new();
    tree.write("agents/tester.json", &boxed_agent());
    let mut claude = claude_body();
    claude["hands"] = json!({"workspace": fragment});
    tree.write("adapters/claude.json", &claude);
    let resolution = resolved(&tree, &Availability::unspecified());
    assert_eq!(resolution.candidates.len(), 2);
    for (candidate, concrete, effort) in [
        (&resolution.candidates[0], "claude-opus-5", "high"),
        (&resolution.candidates[1], "claude-sonnet-5", "medium"),
    ] {
        assert_eq!(candidate.hands_fragment, fragment);
        let mut expected = vec![
            "{brokkr}", "driver", "claude", "--", "--model", concrete, "--effort", effort,
        ];
        expected.extend(fragment);
        assert_eq!(candidate.argv, expected);
        let grants: Vec<&String> = candidate
            .argv
            .iter()
            .zip(candidate.argv.iter().skip(1))
            .filter(|(flag, _)| *flag == "--allowedTools")
            .map(|(_, value)| value)
            .collect();
        assert_eq!(grants, ["mcp__brokkr__workspace"], "{:?}", candidate.argv);
        for retired in ["Bash(cargo:*)", "Bash(git:*)"] {
            assert!(
                !candidate.argv.iter().any(|part| part.contains(retired)),
                "{retired} in {:?}",
                candidate.argv
            );
        }
    }
}

/// Issue #307's internal resolver control — not a loadable adapter file,
/// because the loader requires `workspace` inside a `hands` object. With
/// the parsed workspace capability gone and `hands.harness.work` still
/// declared, boxed composition refuses on the missing workspace: harness
/// support cannot rescue a namespace seat.
#[test]
fn harness_work_support_cannot_rescue_a_boxed_seat_without_a_workspace_fragment() {
    let tree = Tree::new();
    tree.write("agents/tester.json", &boxed_agent());
    let mut claude = claude_body();
    claude["tool_permissions"] = json!("unsupported");
    claude["hands"] = json!({
        "workspace": ["--mcp-config", "{hands_mcp_json}"],
        "harness": {"work": ["--writable"]},
    });
    tree.write("adapters/claude.json", &claude);
    let library = tree.library();
    let agent = library.agent("tester").unwrap();
    let mut adapter = tree.adapters().adapter("claude").unwrap().clone();
    adapter.hands = None;
    assert_eq!(
        adapter.harness.work.as_deref(),
        Some(&["--writable".to_string()][..]),
        "the control retains the harness work fragment"
    );

    let refusal = compose(agent, &adapter, "opus", "claude-opus-5", true)
        .expect_err("a boxed seat needs the workspace fragment")
        .to_string();
    assert_eq!(
        refusal,
        "agent 'tester' cannot be served by provider 'claude' on model 'opus': the provider \
         declares hands unsupported, so the agent's hands cannot be put in the box and the \
         agent would run with the harness's own tools. A capability the provider cannot \
         express fails compilation here rather than degrading silently at run time"
    );
    // Unboxed, the same adapter composes and carries neither fragment nor
    // tool list: the workspace requirement is the boxed path's alone.
    let (argv, effort, hands_fragment) = compose(agent, &adapter, "opus", "claude-opus-5", false)
        .expect("unboxed composition asks for no workspace fragment");
    assert_eq!(
        argv,
        [
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
    assert_eq!(effort.as_deref(), Some("high"));
    assert!(hands_fragment.is_empty());
}

#[test]
fn hands_declarations_are_refused_where_malformed_and_named_where_refused() {
    let tree = Tree::new();
    let mut body = agent_body();
    body["hands"] = json!({"kind": "mitten"});
    tree.write("agents/tester.json", &body);
    tree.write("adapters/claude.json", &claude_body());
    let error = tree.library_error();
    assert!(
        error.contains("'hands'") && error.contains("kind must be"),
        "{error}"
    );

    let adapters = Tree::new();
    adapters.write("agents/tester.json", &agent_body());
    let mut claude = claude_body();
    claude["hands"] = json!({"workspace": ["--tools", ""], "extra": 1});
    adapters.write("adapters/claude.json", &claude);
    let error = adapters.adapters_error();
    assert!(error.contains("'hands'"), "{error}");
    let mut claude = claude_body();
    claude["hands"] = json!("sometimes");
    adapters.write("adapters/claude.json", &claude);
    let error = adapters.adapters_error();
    assert!(error.contains("unsupported"), "{error}");
}

// ---------------------------------- decision 0046 ruling 4: hands.harness

/// An adapter whose `hands` carries the given `harness` object beside a
/// workspace fragment.
fn harness_adapter(harness: Value) -> Value {
    let mut claude = claude_body();
    claude["hands"] = json!({"workspace": ["--tools", ""], "harness": harness});
    claude
}

/// The loader's vocabulary for `hands.harness` (task 8.3): three members
/// and no other; `result` one of two doors; a workspace token in either
/// fragment refused as a fragment that would run with a literal token in
/// it; an empty fragment a legal measured declaration; `unsupported`
/// with a reason a measured gap and not a capability; and every shape
/// outside the three refused by name.
#[test]
fn an_adapters_harness_hands_are_three_members_and_two_doors() {
    let tree = Tree::new();
    tree.write("agents/tester.json", &agent_body());

    tree.write(
        "adapters/claude.json",
        &harness_adapter(json!({"judge": []})),
    );
    let error = tree.adapters_error();
    assert!(
        error.contains("'hands.harness' has unknown key 'judge'; known keys: gate, work, result"),
        "{error}"
    );

    tree.write(
        "adapters/claude.json",
        &harness_adapter(json!({"gate": [], "result": "stdout"})),
    );
    let error = tree.adapters_error();
    assert!(
        error.contains("'hands.harness'.result is 'stdout'; a gate's result reaches the engine through 'file' (the seat writes it) or 'last-message' (the harness's capture writes the final message to it) and nothing else"),
        "{error}"
    );

    for (member, token) in [
        ("gate", "{hands_mcp_json}"),
        ("work", "{hands_args_toml}"),
        ("gate", "mcp_servers.brokkr.args={hands_args_toml}"),
    ] {
        tree.write(
            "adapters/claude.json",
            &harness_adapter(json!({ member: ["--flag", token] })),
        );
        let error = tree.adapters_error();
        assert!(
            error.contains(&format!(
                "'hands.harness'.{member} names '{token}', but no workspace tool is served \
                 under the `harness` boundary — a fragment there may carry {{result_path}} \
                 and {{brokkr}} and nothing of the box's (decision 0046 ruling 4)"
            )),
            "{member} {token}: {error}"
        );
    }

    // An empty fragment: the driver argv already stands in that mode.
    tree.write(
        "adapters/claude.json",
        &harness_adapter(json!({"gate": [], "work": []})),
    );
    let adapters = tree.adapters();
    let harness = &adapters.adapter("claude").unwrap().harness;
    assert_eq!(harness.gate, Some(Vec::new()));
    assert_eq!(harness.work, Some(Vec::new()));
    assert_eq!(harness.gate_gap, None);
    assert_eq!(harness.work_gap, None);
    assert_eq!(harness.result, ResultDoor::File, "absent reads `file`");
    assert_eq!(harness.result.word(), "file");

    // A measured gap on one member, a fragment with both tokens and the
    // capture door on the other.
    tree.write(
        "adapters/claude.json",
        &harness_adapter(json!({
            "gate": ["--read-only", "--capture", "{result_path}", "--bin", "{brokkr}"],
            "work": {"unsupported": "no writable mode was measured"},
            "result": "last-message",
        })),
    );
    let adapters = tree.adapters();
    let harness = &adapters.adapter("claude").unwrap().harness;
    assert_eq!(
        harness.gate,
        Some(
            [
                "--read-only",
                "--capture",
                "{result_path}",
                "--bin",
                "{brokkr}"
            ]
            .map(String::from)
            .to_vec()
        )
    );
    assert_eq!(harness.work, None);
    assert_eq!(
        harness.work_gap.as_deref(),
        Some("no writable mode was measured")
    );
    assert_eq!(harness.result, ResultDoor::LastMessage);
    assert_eq!(harness.result.word(), "last-message");
    // The three-shape convention `tool_permissions` uses: a gap admits
    // no other key and needs an actual reason; a member is an array or
    // the gap object and nothing else.
    for (member, expected) in [
        (
            json!({"unsupported": "reason", "flag": "--x"}),
            "'hands.harness'.gate has unknown key 'flag'; known keys: unsupported",
        ),
        (
            json!({"unsupported": 1}),
            "'hands.harness'.gate needs a non-empty string 'unsupported'",
        ),
        (
            json!("--read-only"),
            "'hands.harness'.gate needs 'gate' as an array of strings",
        ),
        (
            json!([1]),
            "'hands.harness'.gate 'gate' must hold strings only",
        ),
    ] {
        tree.write(
            "adapters/claude.json",
            &harness_adapter(json!({"gate": member})),
        );
        let error = tree.adapters_error();
        assert!(error.contains(expected), "{error}");
    }
    tree.write("adapters/claude.json", &harness_adapter(json!("read-only")));
    let error = tree.adapters_error();
    assert!(error.contains("'hands.harness'"), "{error}");

    // No `harness` at all: every member undeclared, fail-closed, and the
    // door `file` — the reading of every adapter written before the ruling.
    let mut bare = claude_body();
    bare["hands"] = json!({"workspace": ["--tools", ""]});
    tree.write("adapters/claude.json", &bare);
    assert_eq!(
        tree.adapters().adapter("claude").unwrap().harness,
        HarnessHands::default()
    );
    assert_eq!(HarnessHands::default().result, ResultDoor::File);
}

/// The shipped adapter data (task 8.8): codex declares both fragments and
/// its capture door exactly as decision 0046 ruling 4 names them; claude's
/// two members are each undeclared with no reason — not yet measured, the
/// loader's fail-closed reading — and its door reads `file`; dsh and
/// lanetally declare no `hands.harness`. The candidate resolved from a
/// hands agent carries the declaration to the engine.
#[test]
fn the_shipped_adapters_declare_their_harness_as_the_record_says() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let adapters = Adapters::load(&root.join("adapters")).expect("the shipped adapters load");
    let codex = &adapters.adapter("codex").unwrap().harness;
    assert_eq!(
        codex.gate,
        Some(
            [
                "--sandbox",
                "read-only",
                "--output-last-message",
                "{result_path}"
            ]
            .map(String::from)
            .to_vec()
        )
    );
    assert_eq!(
        codex.work,
        Some(["--sandbox", "workspace-write"].map(String::from).to_vec())
    );
    assert_eq!(codex.result, ResultDoor::LastMessage);
    assert_eq!(codex.gate_gap, None);
    assert_eq!(codex.work_gap, None);

    let claude = &adapters.adapter("claude").unwrap().harness;
    match (&claude.gate, &claude.gate_gap) {
        (None, None) => {}
        (Some(fragment), None) => assert!(
            fragment.iter().any(|token| token.contains("{result_path}")),
            "a measured gate fragment names its door: {fragment:?}"
        ),
        (None, Some(reason)) => assert!(!reason.is_empty()),
        (Some(_), Some(_)) => unreachable!("a member is a fragment or a gap, never both"),
    }
    match (&claude.work, &claude.work_gap) {
        (Some(_), Some(_)) => unreachable!("a member is a fragment or a gap, never both"),
        (None, Some(reason)) => assert!(!reason.is_empty()),
        _ => {}
    }
    // As the tree stands: undeclared, pending the operator's measurement.
    assert_eq!(claude.gate, None);
    assert_eq!(claude.gate_gap, None);
    assert_eq!(claude.work, None);
    assert_eq!(claude.work_gap, None);
    assert_eq!(claude.result, ResultDoor::File);

    for quiet in ["dsh", "lanetally", "exec"] {
        assert_eq!(
            adapters.adapter(quiet).unwrap().harness,
            HarnessHands::default(),
            "{quiet} declares no hands.harness"
        );
    }

    // The shipped resume dispositions (operator ruling 2026-09-15, reconciled
    // against the 2026-09-16 live proof). Codex `work-site` preserves main's
    // harness/none AND inline/not-applicable rejoins as `supported`; both
    // identity fields now name the proof-exercised 0.154.0, with 0030's
    // historical 0.148.0 measurement and the earlier 0.153.4 applicability
    // retained in evidence. The other three are new rejoins main does not
    // perform and stay `unmeasured`.
    let codex_resume = adapters.adapter("codex").unwrap().resume.shape("work-site");
    let codex_resume = codex_resume.expect("codex declares work-site");
    assert_eq!(codex_resume.status, ResumeStatus::Supported);
    assert_eq!(
        codex_resume.identity,
        ResumeIdentity::Measured {
            version: "0.154.0".into(),
            applies_to: "0.154.0".into(),
            wrapper_digest: None,
        }
    );
    assert_eq!(
        codex_resume.boundaries,
        vec!["harness".to_string(), "not applicable".to_string()]
    );
    assert_eq!(codex_resume.hands, "none");
    for reference in [
        &codex_resume.evidence.interface,
        &codex_resume.evidence.restrictions,
        &codex_resume.evidence.root,
        &codex_resume.evidence.accounting,
    ] {
        assert!(reference.is_some(), "a supported shape names all four");
    }
    for (provider, shape) in [
        ("claude", "boxed-workspace"),
        ("dsh", "headless-work"),
        ("lanetally", "wrapper-work-site"),
    ] {
        let entry = adapters
            .adapter(provider)
            .unwrap()
            .resume
            .shape(shape)
            .unwrap_or_else(|| panic!("{provider} declares {shape}"));
        assert_eq!(
            entry.status,
            ResumeStatus::Unmeasured,
            "{provider} is not enabled by the Codex ruling"
        );
        assert!(
            entry
                .reason
                .as_deref()
                .is_some_and(|why| why.contains("main does not perform")),
            "{provider} names its non-shipping disposition"
        );
    }
    match &adapters
        .adapter("dsh")
        .unwrap()
        .resume
        .shape("headless-work")
        .unwrap()
        .identity
    {
        ResumeIdentity::Measured { wrapper_digest, .. } => assert!(wrapper_digest.is_none()),
        other => panic!("dsh stays a measured identity: {other:?}"),
    }

    let library = Library::load(&root.join("agents")).expect("the shipped library loads");
    let resolution = resolve(
        &library,
        &adapters,
        &Availability::unspecified(),
        "reviewer",
    )
    .expect("the reviewer resolves");
    let astra = resolution
        .candidates
        .iter()
        .find(|candidate| candidate.model == "astra")
        .expect("the reviewer chains astra");
    assert_eq!(astra.provider, "codex");
    assert_eq!(&astra.harness, codex);
    let fable = resolution
        .candidates
        .iter()
        .find(|candidate| candidate.model == "fable")
        .expect("the reviewer chains fable");
    assert_eq!(&fable.harness, claude);
}

// --------------------------------------------- the resume assessment

/// One well-formed measured assessment, as an adapter that HAD measured
/// its shape would declare it.
fn supported_resume() -> Value {
    json!({
        "work-site": {
            "status": "supported",
            "identity": {"version": "1.2.3", "applies_to": "1.2.3"},
            "classes": ["work"],
            "boundaries": ["namespace"],
            "hands": "boxed",
            "evidence": {
                "interface": "probe of 2026-09-09",
                "restrictions": "class re-imposition observed",
                "root": "the same root id came back, and it persists",
                "accounting": "the current-work cursor is the turn index"
            }
        }
    })
}

fn with_resume(resume: Value) -> Tree {
    let tree = Tree::new();
    tree.write("agents/tester.json", &agent_body());
    let mut adapter = claude_body();
    adapter["resume"] = resume;
    tree.write("adapters/claude.json", &adapter);
    tree
}

/// Proposed decision 0056 ruling 5: the three statuses load, and only a
/// MEASURED identity with all four evidence references can support
/// enablement.
#[test]
fn each_resume_status_loads_and_only_a_measured_supported_shape_enables_anything() {
    let tree = with_resume(supported_resume());
    let adapters = tree.adapters();
    let adapter = adapters.adapter("claude").unwrap();
    assert_eq!(
        adapter.resume.shape("work-site").map(|shape| shape.status),
        Some(ResumeStatus::Supported)
    );
    // A shape this assessment does not name is absent, and so is a
    // shape on an adapter that declares nothing at all: absent loads,
    // compiles and invokes cold.
    assert!(adapter.resume.shape("some-other-shape").is_none());
    let bare = ready();
    let bare_adapters = bare.adapters();
    let bare_adapter = bare_adapters.adapter("claude").unwrap();
    assert!(bare_adapter.resume.is_empty());
    assert!(
        bare_adapter.resume.shape("work-site").is_none(),
        "an absent `resume` key loads and enables nothing"
    );
    assert_eq!(
        bare_adapter.resume.value(),
        Value::Null,
        "and reaches the driver as an explicit absence, never as implicit support"
    );

    // The two honest unmeasured shapes an adapter must be able to write
    // BEFORE anything is measured: an unknown identity with its reason,
    // and a measured identity that does not qualify the installed
    // version. Neither is an authoring error and neither enables
    // anything.
    for identity in [
        json!({"unknown": "no wrapper measurement exists yet"}),
        json!({"version": "0.148.0", "applies_to": "0.153.4"}),
    ] {
        let tree = with_resume(json!({"work-site": {
            "status": "unmeasured",
            "identity": identity,
            "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed",
            "reason": "the installed version has not been remeasured"
        }}));
        let adapters = tree.adapters();
        assert_eq!(
            adapters
                .adapter("claude")
                .unwrap()
                .resume
                .shape("work-site")
                .map(|shape| shape.status),
            Some(ResumeStatus::Unmeasured)
        );
    }

    // An unsupported shape carries its measured reason and loads.
    let tree = with_resume(json!({"work-site": {
        "status": "unsupported",
        "identity": {"version": "0.1.2-rc.1", "applies_to": "0.1.2-rc.1"},
        "classes": ["work"], "boundaries": ["not applicable"], "hands": "none",
        "reason": "the runner mints its own agent and takes no session selector"
    }}));
    let adapters = tree.adapters();
    let shape = adapters
        .adapter("claude")
        .unwrap()
        .resume
        .shape("work-site")
        .unwrap();
    assert_eq!(shape.status, ResumeStatus::Unsupported);
    assert!(shape.reason.is_some());
}

/// The closed data the driver reads out of its private start context:
/// every field, in both identity forms, for each of the three statuses.
/// This is the whole of what crosses into the adapter — no evidence
/// database, no probe, no path by which a declaration enables itself.
#[test]
fn the_assessment_reaches_the_driver_as_closed_data_in_both_identity_forms() {
    let tree = with_resume(json!({
        "work-site": supported_resume()["work-site"],
        "wrapper-site": {
            "status": "unsupported",
            "identity": {"unknown": "no wrapper measurement exists"},
            "classes": ["work"], "boundaries": ["harness"], "hands": "none",
            "reason": "a wrapper is qualified on its own wrapper",
            "limitations": ["the underlying version is read through no measured interface"]
        }
    }));
    let adapters = tree.adapters();
    let value = adapters.adapter("claude").unwrap().resume.value();
    assert_eq!(value["work-site"]["status"], "supported");
    assert_eq!(value["work-site"]["identity"]["version"], "1.2.3");
    assert_eq!(value["work-site"]["identity"]["applies_to"], "1.2.3");
    assert_eq!(
        value["work-site"]["evidence"]["accounting"],
        "the current-work cursor is the turn index"
    );
    assert_eq!(value["work-site"]["reason"], Value::Null);
    assert_eq!(value["wrapper-site"]["status"], "unsupported");
    assert_eq!(
        value["wrapper-site"]["identity"]["unknown"],
        "no wrapper measurement exists"
    );
    assert_eq!(value["wrapper-site"]["evidence"]["root"], Value::Null);
    assert_eq!(
        value["wrapper-site"]["limitations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    // The three words are the three words, and nothing else is one.
    assert_eq!(ResumeStatus::Unmeasured.word(), "unmeasured");
    assert_eq!(ResumeStatus::Unsupported.word(), "unsupported");
    assert_eq!(ResumeStatus::Supported.word(), "supported");
}

/// The other half of the same distinction (task repairs F2 and F6):
/// data that is PRESENT and malformed is a loader refusal naming the
/// field, never a silent downgrade to `unmeasured`. An authoring error
/// must not be able to pass itself off as honest ignorance.
#[test]
fn present_and_malformed_resume_data_is_refused_and_never_read_as_unmeasured() {
    for (case, resume, expected) in [
        ("bare true", json!(true), "must be an object"),
        (
            "unknown status token",
            json!({"work-site": {"status": "probably",
                   "identity": {"unknown": "why"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed"}}),
            "needs 'status'",
        ),
        (
            "supported with an unknown identity",
            json!({"work-site": {"status": "supported",
                   "identity": {"unknown": "nobody looked"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed",
                   "evidence": {"interface": "i", "restrictions": "r",
                                "root": "o", "accounting": "a"}}}),
            "only a MEASURED identity can support enablement",
        ),
        // Each of the four, missing on its own. One does not imply the
        // others, so any one of them absent leaves the shape unsupported.
        (
            "supported missing its accounting evidence",
            json!({"work-site": {"status": "supported",
                   "identity": {"version": "1.2.3", "applies_to": "1.2.3"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed",
                   "evidence": {"interface": "i", "restrictions": "r", "root": "o"}}}),
            "must name all four evidence references",
        ),
        (
            "supported missing its root evidence",
            json!({"work-site": {"status": "supported",
                   "identity": {"version": "1.2.3", "applies_to": "1.2.3"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed",
                   "evidence": {"interface": "i", "restrictions": "r", "accounting": "a"}}}),
            "must name all four evidence references",
        ),
        (
            "supported missing its restriction evidence",
            json!({"work-site": {"status": "supported",
                   "identity": {"version": "1.2.3", "applies_to": "1.2.3"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed",
                   "evidence": {"interface": "i", "root": "o", "accounting": "a"}}}),
            "must name all four evidence references",
        ),
        (
            "supported missing its interface evidence",
            json!({"work-site": {"status": "supported",
                   "identity": {"version": "1.2.3", "applies_to": "1.2.3"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed",
                   "evidence": {"restrictions": "r", "root": "o", "accounting": "a"}}}),
            "must name all four evidence references",
        ),
        (
            "an empty limitation",
            json!({"work-site": {"status": "unmeasured", "identity": {"unknown": "why"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed",
                   "limitations": [""]}}),
            "bounded non-empty line",
        ),
        (
            "unsupported with no measured reason",
            json!({"work-site": {"status": "unsupported",
                   "identity": {"version": "1.2.3", "applies_to": "1.2.3"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed"}}),
            "needs its measured 'reason'",
        ),
        (
            "neither identity form",
            json!({"work-site": {"status": "unmeasured", "identity": {},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed"}}),
            "non-empty string 'version'",
        ),
        (
            "no identity at all",
            json!({"work-site": {"status": "unmeasured",
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed"}}),
            "is required: either the measured form",
        ),
        (
            "an identity that is not an object",
            json!({"work-site": {"status": "unmeasured", "identity": "1.2.3",
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed"}}),
            "is required: either the measured form",
        ),
        (
            "an evidence key behind the four",
            json!({"work-site": {"status": "unmeasured", "identity": {"unknown": "why"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed",
                   "evidence": {"vibes": "good"}}}),
            "unknown key 'vibes'",
        ),
        (
            "an unbounded evidence reference",
            json!({"work-site": {"status": "unmeasured", "identity": {"unknown": "why"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed",
                   "evidence": {"interface": "x".repeat(401)}}}),
            "bounded non-empty line",
        ),
        (
            "an unbounded limitation",
            json!({"work-site": {"status": "unmeasured", "identity": {"unknown": "why"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed",
                   "limitations": ["x".repeat(401)]}}),
            "bounded non-empty line",
        ),
        (
            "a class outside the grammar",
            json!({"work-site": {"status": "unmeasured", "identity": {"unknown": "why"},
                   "classes": ["Work"], "boundaries": ["namespace"], "hands": "boxed"}}),
            "does not match",
        ),
        (
            "unknown identity with no reason",
            json!({"work-site": {"status": "unmeasured", "identity": {"unknown": ""},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed"}}),
            "non-empty string 'unknown'",
        ),
        (
            "a shape name outside the grammar",
            json!({"Work Site": {"status": "unmeasured", "identity": {"unknown": "why"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed"}}),
            "does not match",
        ),
        (
            "a key behind the closed shape",
            json!({"work-site": {"status": "unmeasured", "identity": {"unknown": "why"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed",
                   "probe": ["run", "this"]}}),
            "unknown key 'probe'",
        ),
        (
            "a measured identity with a key behind the set",
            json!({"work-site": {"status": "unmeasured",
                   "identity": {"version": "1.2.3", "applies_to": "1.2.3", "probe": "x"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed"}}),
            "unknown key 'probe'",
        ),
        (
            "an unbounded reason",
            json!({"work-site": {"status": "unsupported",
                   "identity": {"version": "1", "applies_to": "1"},
                   "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed",
                   "reason": "x".repeat(401)}}),
            "bounded non-empty line",
        ),
    ] {
        let tree = with_resume(resume);
        let error = tree.adapters_error();
        assert!(
            error.contains(expected),
            "{case}: expected a refusal naming {expected:?}, got {error}"
        );
    }
}

/// Design D6 / task 8.8(a): the optional `wrapper_digest` member is
/// admitted in the measured form only, carried into the closed data the
/// driver reads, moves the adapter content digest, and is refused by name
/// when malformed or placed beside the unknown form.
#[test]
fn the_optional_wrapper_digest_member_loads_carries_and_is_refused_by_name() {
    let digest = "a".repeat(64);

    // Absent: a measured identity without the member still loads.
    let plain = with_resume(json!({
        "work-site": {"status": "unmeasured",
            "identity": {"version": "1.2.3", "applies_to": "1.2.3"},
            "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed"}
    }));
    let plain_adapters = plain.adapters();
    let shape = plain_adapters
        .adapter("claude")
        .unwrap()
        .resume
        .shape("work-site")
        .unwrap();
    assert_eq!(
        shape.identity,
        ResumeIdentity::Measured {
            version: "1.2.3".into(),
            applies_to: "1.2.3".into(),
            wrapper_digest: None,
        }
    );

    // Present and well formed: loaded and carried in the closed data.
    let wrapped = with_resume(json!({
        "work-site": {"status": "unmeasured",
            "identity": {"version": "1.2.3", "applies_to": "1.2.3", "wrapper_digest": digest},
            "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed"}
    }));
    let wrapped_adapters = wrapped.adapters();
    let shape = wrapped_adapters
        .adapter("claude")
        .unwrap()
        .resume
        .shape("work-site")
        .unwrap();
    assert_eq!(
        shape.identity,
        ResumeIdentity::Measured {
            version: "1.2.3".into(),
            applies_to: "1.2.3".into(),
            wrapper_digest: Some(digest.clone()),
        }
    );
    assert_eq!(shape.identity.value()["wrapper_digest"], json!(digest));
    // The adapter content digest moves when the member moves.
    assert_ne!(
        plain_adapters.digest("claude"),
        wrapped_adapters.digest("claude")
    );

    // A digest whose bytes are decimal digits is the other half of the
    // lowercase-hex predicate: `0` is a hex character, so it loads.
    let digits = "0".repeat(64);
    let numeric = with_resume(json!({
        "work-site": {"status": "unmeasured",
            "identity": {"version": "1.2.3", "applies_to": "1.2.3", "wrapper_digest": digits},
            "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed"}
    }));
    let numeric_adapters = numeric.adapters();
    let shape = numeric_adapters
        .adapter("claude")
        .unwrap()
        .resume
        .shape("work-site")
        .unwrap();
    assert_eq!(
        shape.identity,
        ResumeIdentity::Measured {
            version: "1.2.3".into(),
            applies_to: "1.2.3".into(),
            wrapper_digest: Some(digits.clone()),
        }
    );

    // Malformed: refused by the RESPONSIBLE reason, not merely by a
    // message that happens to carry the field's name. Uppercase, short,
    // long and non-hex are grammar refusals that quote the offending
    // value back; the last three are MISTYPED — a number, a null and an
    // object are not a digest, and none of them is coerced into one, so
    // each fails the string rule instead.
    //
    // Asserting only `contains("wrapper_digest")` could not tell those
    // two refusals apart, and would have been satisfied by a loader that
    // rejected every digest for the wrong reason (council return
    // 2026-09-19, F8).
    // The refusal's prefix names the adapter FILE, whose temporary path
    // differs on every run; everything from the shape onwards is fixed.
    let grammar = |value: &str| {
        format!(
            "'resume' 'work-site' 'identity' 'wrapper_digest' must be 64 lowercase hexadecimal \
             characters — the declared composite identity design D6 writes at enablement; \
             '{value}' is not"
        )
    };
    let mistyped =
        "'resume' 'work-site' 'identity' needs a non-empty string 'wrapper_digest'".to_string();
    for (bad, reason) in [
        (json!("A".repeat(64)), grammar(&"A".repeat(64))),
        (json!("a".repeat(63)), grammar(&"a".repeat(63))),
        (json!("a".repeat(65)), grammar(&"a".repeat(65))),
        (json!("g".repeat(64)), grammar(&"g".repeat(64))),
        (json!(0), mistyped.clone()),
        (json!(null), mistyped.clone()),
        (json!({"digest": "a".repeat(64)}), mistyped.clone()),
    ] {
        let tree = with_resume(json!({
            "work-site": {"status": "unmeasured",
                "identity": {"version": "1.2.3", "applies_to": "1.2.3", "wrapper_digest": bad},
                "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed"}
        }));
        let error = tree.adapters_error();
        assert!(error.contains(&reason), "{bad}: {error}");
    }

    // Beside `unknown`: the unknown form admits `unknown` alone.
    let tree = with_resume(json!({
        "work-site": {"status": "unmeasured",
            "identity": {"unknown": "why", "wrapper_digest": digest},
            "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed"}
    }));
    let error = tree.adapters_error();
    assert!(error.contains("unknown key 'wrapper_digest'"), "{error}");
}

/// The assessment is adapter DATA: it rides the declaration digest, so a
/// declaration edit moves every bundle identity that consults it. That
/// is what makes decision 0030 ruling 4's "an adapter edit spawns cold"
/// hold for this field too.
#[test]
fn an_edited_resume_assessment_moves_the_adapter_digest() {
    let before = ready();
    let after = with_resume(supported_resume());
    assert_ne!(
        before.adapters().digest("claude").unwrap(),
        after.adapters().digest("claude").unwrap()
    );
    // And the resolved candidate carries the assessment to the engine,
    // which holds no adapter at spawn.
    assert_eq!(
        resolved(&after, &Availability::unspecified()).candidates[0]
            .resume
            .shape("work-site")
            .map(|shape| shape.status),
        Some(ResumeStatus::Supported)
    );
    // The bare adapter declares no map at all, so the candidate carries
    // no shape — which the gate reads exactly as it reads `unmeasured`.
    assert!(
        resolved(&before, &Availability::unspecified()).candidates[0]
            .resume
            .shape("work-site")
            .is_none()
    );
}

// ------------------------------------------- typed local declarations (D5)

/// The file an agent refusal names, derived from the canonical root.
fn tester_file(tree: &Tree) -> String {
    tree.library_root()
        .join("tester.json")
        .display()
        .to_string()
}

/// Decision 0065 slice one, design D5.2 (SCM "Strict typed decoding
/// preserves exact local values"): `tools.allow` keeps its written order
/// and names, `tools.sandbox` keeps exactly the class it names, and an
/// explicit empty allow list is a value of its own — never rewritten to
/// omission, and omission never rewritten to it.
#[test]
fn typed_tools_decode_exactly_and_keep_empty_distinct_from_omission() {
    let decode = |tools: Option<Value>| -> Agent {
        let tree = Tree::new();
        let mut body = agent_body();
        match tools {
            Some(tools) => body["tools"] = tools,
            None => {
                body.as_object_mut().unwrap().remove("tools");
            }
        }
        tree.write("agents/tester.json", &body);
        tree.library().agent("tester").unwrap().clone()
    };
    let declared = decode(Some(
        json!({"allow": ["git", "cargo"], "sandbox": "read-only"}),
    ));
    assert_eq!(
        declared.allow,
        Some(vec!["git".to_string(), "cargo".to_string()])
    );
    assert_eq!(declared.sandbox, Some(Sandbox::ReadOnly));
    assert_eq!(
        declared.local(),
        LocalTools {
            allow: Some(vec!["git".to_string(), "cargo".to_string()]),
            sandbox: Some(Sandbox::ReadOnly),
        }
    );
    // Each class independently, exactly as named.
    for (word, class) in [
        ("read-only", Sandbox::ReadOnly),
        ("workspace-write", Sandbox::WorkspaceWrite),
        ("danger-full-access", Sandbox::DangerFullAccess),
    ] {
        let agent = decode(Some(json!({"allow": ["git", "cargo"], "sandbox": word})));
        assert_eq!(agent.sandbox, Some(class), "{word}");
        assert_eq!(class.name(), word);
        assert_eq!(Sandbox::parse(word), Some(class));
    }
    // Explicit empty is `Some([])`; omission and `{}` are `None`.
    let empty = decode(Some(json!({"allow": []})));
    assert_eq!(empty.allow, Some(Vec::new()));
    assert_eq!(empty.sandbox, None);
    let omitted = decode(None);
    assert_eq!(omitted.allow, None);
    assert_eq!(omitted.sandbox, None);
    assert!(omitted.local().is_unspecified());
    let braces = decode(Some(json!({})));
    assert_eq!(braces.local(), LocalTools::unspecified());
    // The harmless legacy `mcp: []` stays admitted beside both fields.
    let legacy = decode(Some(
        json!({"allow": ["cargo"], "sandbox": "workspace-write", "mcp": []}),
    ));
    assert_eq!(legacy.allow, Some(vec!["cargo".to_string()]));
    assert_eq!(legacy.sandbox, Some(Sandbox::WorkspaceWrite));
}

/// SCM "Malformed tools cannot become defaults": every malformed field
/// refuses with its complete cause, naming the agent and the file, and a
/// valid sibling never substitutes for the broken field.
#[test]
fn typed_tools_decoding_refuses_each_malformed_field_with_its_full_cause() {
    let cases: Vec<(Value, String)> = vec![
        (json!(null), "'tools' must be a JSON object".to_string()),
        (json!([]), "'tools' must be a JSON object".to_string()),
        (
            json!({"invented": 1}),
            "'tools' has unknown key 'invented'; known keys: allow, sandbox, mcp".to_string(),
        ),
        (
            json!({"allow": null}),
            "'tools' needs 'allow' as an array of strings".to_string(),
        ),
        (
            json!({"allow": "cargo"}),
            "'tools' needs 'allow' as an array of strings".to_string(),
        ),
        (
            json!({"allow": [1]}),
            "'tools' 'allow' must hold strings only".to_string(),
        ),
        (
            json!({"allow": ["cargo", "git", "cargo"]}),
            "'tools.allow' names 'cargo' twice; a local allow list is duplicate-free".to_string(),
        ),
        (
            json!({"allow": ["Bash(cargo:*)"]}),
            "'tools.allow' names 'Bash(cargo:*)', which does not match ^[a-z][a-z0-9-]*$"
                .to_string(),
        ),
        (
            json!({"sandbox": null}),
            "'tools.sandbox' must be a string naming one of read-only, workspace-write, \
             danger-full-access, got null"
                .to_string(),
        ),
        (
            json!({"sandbox": 1}),
            "'tools.sandbox' must be a string naming one of read-only, workspace-write, \
             danger-full-access, got 1"
                .to_string(),
        ),
        (
            json!({"sandbox": {"kind": "read-only"}}),
            "'tools.sandbox' must be a string naming one of read-only, workspace-write, \
             danger-full-access, got {\"kind\":\"read-only\"}"
                .to_string(),
        ),
        (
            json!({"sandbox": "loose"}),
            "'tools.sandbox' is 'loose', which is not one of read-only, workspace-write, \
             danger-full-access"
                .to_string(),
        ),
        // A valid sibling does not stand in for the broken field.
        (
            json!({"allow": ["cargo"], "sandbox": "loose"}),
            "'tools.sandbox' is 'loose', which is not one of read-only, workspace-write, \
             danger-full-access"
                .to_string(),
        ),
        (
            json!({"allow": null, "sandbox": "read-only"}),
            "'tools' needs 'allow' as an array of strings".to_string(),
        ),
        // Nonempty legacy MCP still refuses with the migration reason.
        (
            json!({"allow": ["cargo"], "sandbox": "read-only", "mcp": [{"server": "github"}]}),
            "'tools.mcp' names an MCP server; an agent no longer names one, because a server \
             an office could name would be a door a pulled bundle could open. Request the \
             capability by abstract name under 'capabilities' (\"requires\" or \"wants\") and \
             let realms.json grant it through a tool dialect (decision 0065 rulings 1 and 3)"
                .to_string(),
        ),
    ];
    for (tools, cause) in cases {
        let tree = Tree::new();
        let mut body = agent_body();
        body["tools"] = tools.clone();
        tree.write("agents/tester.json", &body);
        assert_eq!(
            tree.library_error(),
            format!("agent 'tester' ({}) {cause}", tester_file(&tree)),
            "{tools}"
        );
    }
}

/// SCM "Malformed tools cannot become defaults", last clause: a `tools`,
/// `allow` or `sandbox` key written twice in the ORIGINAL bytes refuses
/// from the strict reader, even when both copies are equal — an ordinary
/// map would keep the second and say nothing. The position is the byte the
/// parser stood on when it saw the repeat, derived from the fixture text.
#[test]
fn repeated_tools_keys_refuse_from_the_original_source_even_when_equal() {
    let head = r#"{"description":"d","charter":"charters/c.md","models":["opus"],"#;
    // The parser reports the column one past the token that closed the
    // repeated value: it has consumed that token and stands after it.
    type Column = fn(&str) -> usize;
    let cases: [(String, &str, Column); 3] = [
        (
            format!(r#"{head}"tools":{{"allow":["cargo"],"allow":["cargo"]}}}}"#),
            "allow",
            // The second `["cargo"]` closes at the last `]`.
            |text| text.rfind(']').unwrap() + 2,
        ),
        (
            format!(r#"{head}"tools":{{"allow":["cargo"]}},"tools":{{"allow":["cargo"]}}}}"#),
            "tools",
            // The second `{...}` closes at the inner `}` before the last.
            |text| text.len(),
        ),
        (
            format!(r#"{head}"tools":{{"sandbox":"read-only","sandbox":"read-only"}}}}"#),
            "sandbox",
            // The second `"read-only"` closes at its quote before `}}`.
            |text| text.len() - 1,
        ),
    ];
    for (text, key, column) in cases {
        let tree = Tree::new();
        tree.raw("agents/tester.json", &text);
        assert_eq!(
            tree.library_error(),
            format!(
                "{}: key '{key}' is written twice at line 1 column {}",
                tester_file(&tree),
                column(&text)
            )
        );
    }
    // The control: the same document with each key once loads.
    let tree = Tree::new();
    tree.raw(
        "agents/tester.json",
        &format!(r#"{head}"tools":{{"allow":["cargo"],"sandbox":"read-only"}}}}"#),
    );
    assert_eq!(
        tree.library().agent("tester").unwrap().local(),
        LocalTools {
            allow: Some(vec!["cargo".to_string()]),
            sandbox: Some(Sandbox::ReadOnly),
        }
    );
}

fn local(allow: Option<&[&str]>, sandbox: Option<Sandbox>) -> LocalTools {
    LocalTools {
        allow: allow.map(|names| names.iter().map(|name| name.to_string()).collect()),
        sandbox,
    }
}

/// SCM "Field omission inherits while an explicit empty list subtracts"
/// and "A local override cannot widen its agent", as the pure narrower
/// alone: each field inherits when unspecified, an explicit list keeps its
/// written order and must be a subset, an explicit empty list stays
/// empty, and a class may equal or reduce the office's reach — never
/// exceed it, never be clamped.
#[test]
fn narrowing_inherits_per_field_and_refuses_each_widening_exactly() {
    let office = local(Some(&["cargo", "git"]), Some(Sandbox::WorkspaceWrite));
    let ok: Vec<(LocalTools, LocalTools)> = vec![
        (local(None, None), office.clone()),
        (
            local(None, Some(Sandbox::ReadOnly)),
            local(Some(&["cargo", "git"]), Some(Sandbox::ReadOnly)),
        ),
        (
            local(Some(&[]), None),
            local(Some(&[]), Some(Sandbox::WorkspaceWrite)),
        ),
        (
            local(Some(&["git"]), None),
            local(Some(&["git"]), Some(Sandbox::WorkspaceWrite)),
        ),
        // Written order is kept; nothing is sorted or intersected.
        (
            local(Some(&["git", "cargo"]), None),
            local(Some(&["git", "cargo"]), Some(Sandbox::WorkspaceWrite)),
        ),
        // The same class is not wider.
        (
            local(Some(&["cargo"]), Some(Sandbox::WorkspaceWrite)),
            local(Some(&["cargo"]), Some(Sandbox::WorkspaceWrite)),
        ),
    ];
    for (requested, expected) in ok {
        assert_eq!(office.narrow(&requested), Ok(expected), "{requested:?}");
    }
    assert_eq!(
        office.narrow(&local(Some(&["cargo", "make"]), None)),
        Err((
            "allow".to_string(),
            "names 'make', which the office's 'tools.allow' [\"cargo\", \"git\"] does not; a \
             site subtracts from its office and never adds to it"
                .to_string()
        ))
    );
    assert_eq!(
        office.narrow(&local(None, Some(Sandbox::DangerFullAccess))),
        Err((
            "sandbox".to_string(),
            "requests 'danger-full-access', which reaches wider than the office's \
             'workspace-write'; the classes reach read-only < workspace-write < \
             danger-full-access, and a site narrows its office rather than being clamped to it"
                .to_string()
        ))
    );
    // Each field is judged independently: a valid sandbox does not
    // forgive an added name, and a valid list does not forgive a widening.
    assert_eq!(
        office
            .narrow(&local(Some(&["make"]), Some(Sandbox::ReadOnly)))
            .unwrap_err()
            .0,
        "allow"
    );
    assert_eq!(
        office
            .narrow(&local(Some(&["git"]), Some(Sandbox::DangerFullAccess)))
            .unwrap_err()
            .0,
        "sandbox"
    );
    // An empty office list permits only empty.
    let empty = local(Some(&[]), Some(Sandbox::ReadOnly));
    assert_eq!(
        empty.narrow(&local(Some(&[]), None)),
        Ok(local(Some(&[]), Some(Sandbox::ReadOnly)))
    );
    assert_eq!(
        empty.narrow(&local(Some(&["git"]), None)),
        Err((
            "allow".to_string(),
            "names 'git', which the office's 'tools.allow' [] does not; a site subtracts from \
             its office and never adds to it"
                .to_string()
        ))
    );
    // An unrestricted office may be narrowed by either field.
    let unrestricted = local(None, None);
    assert_eq!(
        unrestricted.narrow(&local(Some(&["git"]), Some(Sandbox::DangerFullAccess))),
        Ok(local(Some(&["git"]), Some(Sandbox::DangerFullAccess)))
    );
    // Every ordered pair of classes: reach read-only < workspace-write <
    // danger-full-access, compared by that order alone.
    let classes = [
        Sandbox::ReadOnly,
        Sandbox::WorkspaceWrite,
        Sandbox::DangerFullAccess,
    ];
    for (i, requested) in classes.iter().enumerate() {
        for (j, office_class) in classes.iter().enumerate() {
            let office = local(None, Some(*office_class));
            let outcome = office.narrow(&local(None, Some(*requested)));
            if i <= j {
                assert_eq!(outcome, Ok(local(None, Some(*requested))));
            } else {
                assert_eq!(outcome.unwrap_err().0, "sandbox");
            }
        }
    }
}

/// D5.2: a site's declaration narrows a PRIVATE clone of the office before
/// composition. The report's chain is composed from the effective value,
/// the office in the library keeps its own, and its source and digest are
/// untouched; a widening refuses with the site's field, the addition and
/// the agent named.
#[test]
fn report_narrowed_composes_from_a_private_clone_and_leaves_the_office_untouched() {
    let tree = ready();
    let library = tree.library();
    let adapters = tree.adapters();
    let report = report_narrowed(
        &library,
        &adapters,
        &Availability::unspecified(),
        "tester",
        brokkr_core::realms::Boundary::Namespace,
        &local(Some(&["git"]), None),
    )
    .unwrap();
    assert_eq!(report.agent.local(), local(Some(&["git"]), None));
    assert_eq!(
        report.entries[0].argv,
        vec![
            "{brokkr}",
            "driver",
            "claude",
            "--",
            "--model",
            "claude-opus-5",
            "--effort",
            "high",
            "--allowedTools",
            "Bash(git:*)"
        ]
    );
    let office = library.agent("tester").unwrap();
    assert_eq!(
        office.allow,
        Some(vec!["cargo".to_string(), "git".to_string()])
    );
    assert_eq!(report.agent.digest, office.digest);
    assert_eq!(report.agent.source, office.source);
    // The plain report is the unnarrowed one.
    let plain = report_under(
        &library,
        &adapters,
        &Availability::unspecified(),
        "tester",
        brokkr_core::realms::Boundary::Namespace,
    )
    .unwrap();
    assert_eq!(plain.agent.local(), office.local());

    let widened = report_narrowed(
        &library,
        &adapters,
        &Availability::unspecified(),
        "tester",
        brokkr_core::realms::Boundary::Namespace,
        &local(Some(&["git", "make"]), None),
    )
    .unwrap_err();
    assert_eq!(
        widened.to_string(),
        "the site's 'tools.allow' names 'make', which the office's 'tools.allow' [\"cargo\", \
         \"git\"] does not; a site subtracts from its office and never adds to it; an \
         agent-backed site only narrows the restrictions of agent 'tester' (decision 0065 \
         slice one, design D5)"
    );
    // The office declares no class, so a class may be introduced: the
    // clone carries it and composition, which lowers no class yet, is
    // unchanged. Whether a site may RUN with it is the bundle's question.
    let classed = report_narrowed(
        &library,
        &adapters,
        &Availability::unspecified(),
        "tester",
        brokkr_core::realms::Boundary::Namespace,
        &local(None, Some(Sandbox::ReadOnly)),
    )
    .unwrap();
    assert_eq!(
        classed.agent.local(),
        local(Some(&["cargo", "git"]), Some(Sandbox::ReadOnly))
    );
    assert_eq!(classed.entries[0].argv, plain.entries[0].argv);
}

/// SCM "Decoding cannot admit a runnable unrestricted command", the direct
/// explicit-empty row of D5.3: `allow: []` is kept exactly and refused at
/// composition with the full cause, on the primary and on a later link
/// alike; it is never joined into an empty flag value.
#[test]
fn an_explicit_empty_allow_set_is_kept_and_refused_until_lowering_delivers_it() {
    let tree = Tree::new();
    let mut body = agent_body();
    body["tools"] = json!({"allow": []});
    tree.write("agents/tester.json", &body);
    tree.write("adapters/claude.json", &claude_body());
    assert_eq!(
        tree.library().agent("tester").unwrap().allow,
        Some(Vec::new())
    );
    let empty_refusal = |provider: &str, model: &str| {
        format!(
            "agent 'tester' cannot be served by provider '{provider}' on model '{model}': the \
             effective 'tools.allow' is explicitly empty, and no serving path yet expresses an \
             empty local allow set as a delivered restriction (joining no names into an empty \
             flag value proves nothing); the declaration is kept exactly and refused rather \
             than run unrestricted, until decision 0065 slice one's lowering proves its \
             delivery (design D5.3). A capability the provider cannot express fails \
             compilation here rather than degrading silently at run time"
        )
    };
    assert_eq!(refusal(&tree, "tester"), empty_refusal("claude", "opus"));
    // The same through a site's narrowing of a nonempty office: the
    // effective value is empty, and the later link refuses even though the
    // primary would too — every link is judged, and the first gap names
    // itself.
    let tree = ready();
    let report = report_narrowed(
        &tree.library(),
        &tree.adapters(),
        &Availability::unspecified(),
        "tester",
        brokkr_core::realms::Boundary::Namespace,
        &local(Some(&[]), None),
    )
    .unwrap();
    assert_eq!(report.agent.allow, Some(Vec::new()));
    assert_eq!(
        report.entries[1].gap.as_ref().map(ToString::to_string),
        Some(empty_refusal("claude", "sonnet"))
    );
    assert_eq!(
        resolve_report(report, &tree.adapters())
            .unwrap_err()
            .to_string(),
        empty_refusal("claude", "opus")
    );
}

/// SCM "Existing hands semantics do not require direct-tool support": with
/// hands, the list is dormant — no direct mapping is required, no direct
/// flag is added, an empty list does not disable hands — while a mapped
/// native alias still refuses with its migration cause, on whichever link
/// maps it (whole-chain, SC8).
#[test]
fn hands_keep_their_replacement_and_still_refuse_a_mapped_native_alias() {
    let fragment = json!({"workspace": ["--tools", "", "--mcp-config", "{hands_mcp_json}"]});
    // A dormant list needs no mapping: `make` is mapped nowhere.
    let tree = Tree::new();
    let mut agent = boxed_agent();
    agent["tools"] = json!({"allow": ["cargo", "make"]});
    tree.write("agents/tester.json", &agent);
    let mut claude = claude_body();
    claude["hands"] = fragment.clone();
    tree.write("adapters/claude.json", &claude);
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
            "high",
            "--tools",
            "",
            "--mcp-config",
            "{hands_mcp_json}"
        ]
    );
    assert_eq!(
        resolution.candidates[0].hands_fragment,
        vec!["--tools", "", "--mcp-config", "{hands_mcp_json}"]
    );
    // An empty list beside hands composes the same fragment.
    agent["tools"] = json!({"allow": []});
    tree.write("agents/tester.json", &agent);
    let emptied = resolved(&tree, &Availability::unspecified());
    assert_eq!(emptied.candidates[0].argv, resolution.candidates[0].argv);
    assert!(emptied.hands.is_some());
    // The syntax check still runs beside hands.
    agent["tools"] = json!({"allow": ["cargo", "cargo"]});
    tree.write("agents/tester.json", &agent);
    assert_eq!(
        tree.library_error(),
        format!(
            "agent 'tester' ({}) 'tools.allow' names 'cargo' twice; a local allow list is \
             duplicate-free",
            tester_file(&tree)
        )
    );
    // A mapped native alias refuses even beside hands — on the fallback
    // link only, so the primary's clean composition hides nothing.
    agent["tools"] = json!({"allow": ["cargo", "websearch"]});
    tree.write("agents/tester.json", &agent);
    claude["models"] = json!({"opus": "claude-opus-5"});
    tree.write("adapters/claude.json", &claude);
    let mut second = claude_body();
    second["provider"] = json!("second");
    second["binary"] = json!("second");
    second["models"] = json!({"sonnet": "second-sonnet"});
    second["hands"] = fragment;
    second["tool_permissions"]["names"]["websearch"] = json!("WebSearch");
    second["native_capabilities"] = json!({"known": {"web-search": {
        "capability": "web-search", "tools": ["WebSearch"],
        "on": {"selection": {"include": ["WebSearch"], "allow": ["WebSearch"], "deny": []}},
        "off": {"selection": {"include": [], "allow": [], "deny": ["WebSearch"]}},
        "restrictions": {"unsupported": "no native restriction transport is established"},
        "evidence": {"source": "adapter data", "scope": "declared", "limitations": []}}},
        "selection": {"include": {"flag": "--tools", "separator": ","},
                      "allow": {"flag": "--allowedTools", "separator": ","},
                      "deny": {"flag": "--disallowedTools", "separator": ","}}});
    tree.write("adapters/second.json", &second);
    let chain = report(
        &tree.library(),
        &tree.adapters(),
        &Availability::unspecified(),
        "tester",
    )
    .unwrap();
    assert!(chain.entries[0].gap.is_none(), "{:?}", chain.entries[0].gap);
    assert_eq!(
        refusal(&tree, "tester"),
        "agent 'tester' cannot be served by provider 'second' on model 'sonnet': tool permission \
         'websearch' maps to 'WebSearch', a tool of the provider's native capability \
         'web-search'; a legacy allow entry cannot authorize a capability, so request \
         'web-search' by name under 'capabilities' and let the realm grant it through a tool \
         dialect (decision 0065 ruling 3). A capability the provider cannot express fails \
         compilation here rather than degrading silently at run time"
    );
}
