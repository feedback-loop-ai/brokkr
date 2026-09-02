//! Unit tests for the scaffold writer: the per-stack tool table's corner
//! branches — the python-row special case, the loud refusal for a runner
//! this file does not know — and the JSON writers against a Detected
//! built by hand. The stack-by-stack end-to-end assertions (charters,
//! adapter map, per-class agent allowances, resolved argv) live in
//! `tests/init_stacks.rs`, where every fixture drives the real binary.

use super::*;

fn detected(name: &str, build: &str, test: &str, lint: &str) -> Detected {
    Detected {
        name: name.to_string(),
        evidence: String::new(),
        build: build.to_string(),
        test: test.to_string(),
        lint: lint.to_string(),
        note: None,
    }
}

fn rust() -> Detected {
    detected(
        "rust",
        "cargo build --workspace",
        "cargo test --workspace",
        "cargo clippy --workspace --all-targets -- -D warnings",
    )
}

fn python() -> Detected {
    detected(
        "python",
        "python3 -m build",
        "python3 -m pytest",
        "python3 -m ruff check .",
    )
}

/// The plain-python row's runner is `python3` (the interpreter its
/// commands run through) plus `pytest` (the venv's own suite binary).
#[test]
fn a_python_rows_runners_are_python3_and_pytest() {
    assert_eq!(runner_tools(&python()), vec![PYTHON3, PYTEST]);
    assert_eq!(tools_for_token(&python(), "python3"), vec![PYTHON3, PYTEST]);
}

/// The special case fires only for the python row: another stack whose
/// command happens to lead with a python3 token is refused loudly rather
/// than granted a tool the table never wrote for it.
#[test]
fn python3_tokens_are_granted_only_to_the_python_stack() {
    // python stack, token that is not python3: falls through to the table.
    assert_eq!(tools_for_token(&python(), "cargo"), vec![CARGO]);
    // rust stack: dedup means one cargo, and no python tools anywhere.
    assert_eq!(runner_tools(&rust()), vec![CARGO]);
}

/// A runner token no row of the two detection tables can produce — a bug
/// in this file, not a stack choice — refuses loudly instead of silently
/// scaffolding a seat that can run nothing.
#[test]
#[should_panic(expected = "has no Bash grant")]
fn an_unknown_runner_token_refuses_loudly() {
    tools_for_token(&rust(), "definitely-not-a-tool");
}

/// The class split, in one assertion: the gate allowance is the read-only
/// subset — the stack's runner plus git, ls and rg — and the work
/// allowance is exactly that plus mkdir.
#[test]
fn the_gate_allowance_is_the_work_allowance_without_mkdir() {
    let set = toolset(&rust());
    assert_eq!(set.gate, vec![CARGO, GIT, LS, RG]);
    assert_eq!(set.work, vec![CARGO, GIT, LS, RG, MKDIR]);

    let python_set = toolset(&python());
    assert_eq!(python_set.gate, vec![PYTHON3, PYTEST, GIT, LS, RG]);
    assert_eq!(python_set.work, vec![PYTHON3, PYTEST, GIT, LS, RG, MKDIR]);
}

/// A stack whose commands the table does not know grants nothing: the
/// adapter's names map stays empty and the agents declare no `tools`
/// restriction at all — the only reading an empty map can honestly serve.
#[test]
fn no_stack_means_an_empty_names_map_and_agents_without_tools() {
    let adapter: serde_json::Value = serde_json::from_str(&adapter_json(None)).unwrap();
    assert!(adapter["tool_permissions"]["names"]
        .as_object()
        .expect("a names map")
        .is_empty());

    let implementer = &SEAT_AGENTS[1];
    let without_tools: serde_json::Value =
        serde_json::from_str(&agent_json(implementer, None)).unwrap();
    assert!(without_tools.get("tools").is_none(), "{without_tools}");

    let rust_set = toolset(&rust());
    let with_tools: serde_json::Value =
        serde_json::from_str(&agent_json(implementer, Some(&rust_set.work))).unwrap();
    assert_eq!(
        with_tools["tools"]["allow"],
        serde_json::json!(["cargo", "git", "ls", "rg", "mkdir"])
    );
}

/// The adapter's names map is the union of every allowance the scaffold
/// wrote — for a recognized stack, the work set, each name backed by its
/// Bash(...) expression.
#[test]
fn the_adapter_names_map_is_the_work_allowance() {
    let adapter: serde_json::Value =
        serde_json::from_str(&adapter_json(Some(&toolset(&python())))).unwrap();
    let names = adapter["tool_permissions"]["names"]
        .as_object()
        .expect("a names map");
    assert_eq!(names.len(), 6, "{names:?}");
    assert_eq!(names["python3"], "Bash(python3:*)");
    assert_eq!(names["pytest"], "Bash(.venv/bin/pytest:*)");
    assert_eq!(names["mkdir"], "Bash(mkdir:*)");
}

/// The README for a repository no row recognizes says the map was
/// scaffolded EMPTY and why, in the words an operator can act on.
#[test]
fn the_no_stack_readme_says_the_map_is_empty_in_so_many_words() {
    let readme = readme(None);
    assert!(readme.contains("NO STACK WAS RECOGNIZED"), "{readme}");
    assert!(readme.contains("scaffolded EMPTY"), "{readme}");
    assert!(readme.contains("does not invent tool names"), "{readme}");
}

/// The five agents of the roster are the five seats of `bundle.json`,
/// each with the bounds the inline starter declared on its seat, and the
/// two classes both present.
#[test]
fn the_roster_is_five_agents_across_both_classes() {
    assert_eq!(SEAT_AGENTS.len(), 5);
    assert_eq!(SEAT_AGENTS[0].name, "intake");
    assert_eq!(SEAT_AGENTS[0].allowance, Allowance::Work);
    assert_eq!(SEAT_AGENTS[1].name, "implementer");
    assert_eq!(SEAT_AGENTS[1].allowance, Allowance::Work);
    for gate in &SEAT_AGENTS[2..] {
        assert_eq!(gate.allowance, Allowance::Gate);
    }
    let names: Vec<&str> = SEAT_AGENTS.iter().map(|agent| agent.name).collect();
    assert!(
        names.contains(&"verifier") && names.contains(&"reviewer") && names.contains(&"shipper")
    );
}

/// The full writer, called in-process: a recognized repo scaffolds the
/// whole tree — policy, bundle, README, adapter, the five agents and
/// their five charters — and the digest it returns is the digest of a
/// bundle that compiles under the tree it wrote.
#[test]
fn init_writes_the_whole_tree_and_returns_a_compiling_digest() {
    let dir = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(repo.path().join("Cargo.toml"), "# marker\n").unwrap();

    let bundle = dir.path().join("bundle");
    let digest = init(&bundle, repo.path()).expect("init succeeds");
    assert!(!digest.is_empty());

    for expected in [
        "policy.json",
        "bundle.json",
        "README.md",
        "adapters/claude.json",
        "agents/intake.json",
        "agents/implementer.json",
        "agents/verifier.json",
        "agents/reviewer.json",
        "agents/shipper.json",
        "agents/charters/intake.md",
        "agents/charters/implementer.md",
        "agents/charters/verifier.md",
        "agents/charters/reviewer.md",
        "agents/charters/shipper.md",
    ] {
        assert!(
            bundle.join(expected).is_file(),
            "{} was not scaffolded",
            expected
        );
    }

    // init proved its own output; prove it again against the same roots.
    let compiled = Bundle::compile_with(
        &bundle,
        &bundle.join(DEFAULT_AGENTS_DIR),
        &bundle.join(DEFAULT_ADAPTERS_DIR),
    )
    .expect("the scaffold compiles under its own adapters/ and agents/");
    assert_eq!(compiled.manifest_digest(), digest);

    // The generic repo, same flow, empty tool map.
    let generic_repo = tempfile::tempdir().unwrap();
    let generic = dir.path().join("generic");
    init(&generic, generic_repo.path()).expect("init succeeds on an unknown stack");
    let adapter: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(generic.join("adapters/claude.json")).unwrap(),
    )
    .unwrap();
    assert!(adapter["tool_permissions"]["names"]
        .as_object()
        .expect("a names map")
        .is_empty());
}

/// The overwrite guards fire before anything is written — the bundle.json
/// guard and the trust-declaration guard, on the same terms as the
/// integration suite asserts through the binary.
#[test]
fn init_refuses_to_overwrite_a_bundle_or_a_trust_declaration() {
    let dir = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();

    std::fs::write(bundle.join("bundle.json"), "{}").unwrap();
    let error = init(&bundle, repo.path()).unwrap_err();
    assert!(
        error.to_string().contains("refusing to overwrite"),
        "{error}"
    );
    std::fs::remove_file(bundle.join("bundle.json")).unwrap();

    std::fs::create_dir_all(bundle.join("adapters")).unwrap();
    std::fs::write(
        bundle.join("adapters/claude.json"),
        "{\"trust_tier\": \"untrusted\"}",
    )
    .unwrap();
    let error = init(&bundle, repo.path()).unwrap_err();
    assert!(
        error.to_string().contains("refusing to overwrite"),
        "{error}"
    );
    assert!(
        !bundle.join("bundle.json").exists(),
        "nothing else was written"
    );
}

/// The scaffolded charters of a recognized stack embed the stack's own
/// commands — the sentence `roles`-as-charters built for a repo with a
/// Cargo.toml.
#[test]
fn a_recognized_charters_text_names_the_stacks_commands() {
    let implementer_text = implementer(Some(&rust()));
    assert!(implementer_text.contains("cargo build --workspace"));
    assert!(implementer_text.contains("cargo test --workspace"));
    let verifier_text = verifier(Some(&rust()));
    assert!(verifier_text.contains("cargo test --workspace"));
    assert!(!verifier_text.contains("NO STACK WAS RECOGNIZED"));
}
