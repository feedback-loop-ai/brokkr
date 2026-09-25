//! Unit tests for the tool-grant derivation in `init.rs` — the half of
//! the feature that turns the detection tables' command text into the
//! `Bash(...)` grants a scaffold writes. The end-to-end proofs (per-fixture
//! adapter maps, per-class allowances, resolved argv) live in
//! `tests/init_stacks.rs`, where a reader sees the whole scaffold; these
//! cover the derivation's own corners — the deduplication, the class
//! split, and the refusals for a runner no table names — that only a
//! synthetic `Detected` can reach.

use super::{
    allowance, command_tools, detect, grants, host, leading_word, realms_json, relative_realm_path,
    runner_tools, stack_readme, tools_for, write_dialect, AgentSpec, Boundary, Class, Cli,
    Detected, DialectDetection, Tool, NO_TOOL_MAP, OPENSPEC, SEATS,
};
use std::panic::{catch_unwind, AssertUnwindSafe};

#[test]
fn dialect_detection_covers_each_marker_combination() {
    for (speckit, openspec, expected) in [
        (true, false, "speckit"),
        (false, true, "openspec"),
        (true, true, "ambiguous"),
        (false, false, "absent"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        if speckit {
            std::fs::create_dir(dir.path().join(".specify")).unwrap();
        }
        if openspec {
            std::fs::create_dir(dir.path().join("openspec")).unwrap();
            std::fs::write(
                dir.path().join("openspec/config.yaml"),
                "schema: spec-driven\n",
            )
            .unwrap();
        }
        let detection = detect(dir.path());
        let actual = match detection.dialect {
            DialectDetection::Detected(choice, dialect) => {
                assert_eq!(choice.name, dialect.name);
                choice.name
            }
            DialectDetection::Ambiguous => "ambiguous",
            DialectDetection::Absent => "absent",
        };
        assert_eq!(actual, expected);
    }
}

#[test]
fn a_realm_path_is_relative_when_roots_are_related_and_absolute_when_they_are_not() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    std::fs::create_dir(&bundle).unwrap();
    assert_eq!(
        relative_realm_path(&bundle, dir.path()),
        std::path::Path::new("..")
    );
    assert_eq!(
        relative_realm_path(dir.path(), dir.path()),
        std::path::Path::new(".")
    );
    let child = dir.path().join("child");
    std::fs::create_dir(&child).unwrap();
    assert_eq!(
        relative_realm_path(dir.path(), &child),
        std::path::Path::new("child")
    );
    assert_eq!(
        relative_realm_path(
            std::path::Path::new("brokkr-certainly-absent-left"),
            std::path::Path::new("brokkr-certainly-absent-right")
        ),
        std::path::Path::new("brokkr-certainly-absent-right")
    );
}

#[test]
fn dialect_writes_surface_a_target_that_becomes_unwritable() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("dialects/openspec.json")).unwrap();
    assert!(write_dialect(dir.path(), OPENSPEC).is_err());
}

/// The leading word is the binary that runs: whole command, flag-heavy
/// command, or a bare single-token command.
#[test]
fn the_leading_word_is_the_binary_that_runs() {
    assert_eq!(leading_word("cargo build --workspace"), "cargo");
    assert_eq!(leading_word("pnpm exec turbo run test"), "pnpm");
    assert_eq!(leading_word("make"), "make");
}

/// Every runner the two detection tables can put first is granted by its
/// own name, as the `Bash(<bin>:*)` prefix the claude CLI reads. The one
/// word with two tools is the plain-python row's `python3`: beside the
/// interpreter, pytest — the venv's suite binary — is granted with the
/// narrower `Bash(.venv/bin/pytest:*)` expression the shipped adapters
/// already carry.
#[test]
fn every_runner_the_tables_name_has_a_bash_grant() {
    for token in [
        "cargo", "bun", "bunx", "pnpm", "yarn", "npm", "npx", "uv", "go", "make",
    ] {
        let granted = tools_for(token);
        assert_eq!(granted.len(), 1, "{token}");
        assert_eq!(granted[0].name, token);
        assert_eq!(granted[0].permission, format!("Bash({token}:*)"));
    }
    let python = tools_for("python3");
    assert_eq!(python, vec![python3(), pytest()]);
    assert_eq!(pytest().permission, "Bash(.venv/bin/pytest:*)");
}

/// The tables are closed data in `init.rs`; a runner no row names is a
/// coding error, and the derivation refuses loudly rather than inventing
/// a `Bash(...)` expression no seat asked for.
#[test]
fn a_runner_no_table_names_is_refused_not_invented() {
    let refused = catch_unwind(AssertUnwindSafe(|| tools_for("deno")));
    assert!(refused.is_err(), "an unknown runner must refuse");
    // The same refusal greets an empty command: no leading word, no grant.
    let refused = catch_unwind(AssertUnwindSafe(|| command_tools("")));
    assert!(refused.is_err(), "an empty command must refuse");
}

/// The runners a detected stack earns are the leading binaries of its
/// build, test and lint commands, deduplicated and in that order — a
/// command's binary is listed once however many of the three invoke it.
#[test]
fn runner_tools_deduplicates_across_the_three_commands() {
    let d = detected(
        "cargo build --workspace",
        "cargo test --workspace",
        "cargo clippy --workspace --all-targets -- -D warnings",
    );
    assert_eq!(
        runner_tools(&d),
        vec![Tool {
            name: "cargo",
            permission: "Bash(cargo:*)"
        }]
    );

    // A synthetic row whose lint line runs through a second binary still
    // lists each once, build order preserved.
    let d = detected("make build", "bun run test", "make lint");
    assert_eq!(
        runner_tools(&d)
            .iter()
            .map(|tool| tool.name)
            .collect::<Vec<_>>(),
        ["make", "bun"]
    );
}

/// The class split of decision 0021 ruling 1, on one stack: the work set
/// is the whole grant — every runner, the read trio, and `mkdir`; the
/// gate set is the test runner's tools and the read trio, never the
/// write tool. For a stack whose build line runs through a different
/// binary than its test line, that difference is visible: the gate keeps
/// only what proves.
#[test]
fn grants_split_the_full_set_from_the_gate_subset() {
    // rust/cargo: one binary everywhere.
    let d = detected(
        "cargo build --workspace",
        "cargo test --workspace",
        "cargo clippy --workspace --all-targets -- -D warnings",
    );
    let grant = grants(Some(&d));
    assert_eq!(names(&grant.work), ["cargo", "git", "ls", "rg", "mkdir"]);
    assert_eq!(names(&grant.gate), ["cargo", "git", "ls", "rg"]);

    // The plain-python row: interpreter and pytest on both classes.
    let d = detected(
        "python3 -m build",
        "python3 -m pytest",
        "python3 -m ruff check .",
    );
    let grant = grants(Some(&d));
    assert_eq!(
        names(&grant.work),
        ["python3", "pytest", "git", "ls", "rg", "mkdir"]
    );
    assert_eq!(names(&grant.gate), ["python3", "pytest", "git", "ls", "rg"]);

    // A build-only binary stays out of the gate set.
    let d = detected("make build", "bun run test", "bun run typecheck");
    let grant = grants(Some(&d));
    assert_eq!(
        names(&grant.work),
        ["make", "bun", "git", "ls", "rg", "mkdir"]
    );
    assert_eq!(names(&grant.gate), ["bun", "git", "ls", "rg"]);
}

/// No recognized stack means no grant at all: both allowances empty, so
/// the adapter map is written empty and no agent names a tool.
#[test]
fn an_unrecognized_stack_grants_nothing() {
    let grant = grants(None);
    assert!(grant.work.is_empty());
    assert!(grant.gate.is_empty());
}

/// The allowance an agent is written with follows the class of the seat
/// that names it — and, when nothing was granted, every agent omits the
/// `tools` restriction rather than name a tool the empty map cannot back.
#[test]
fn an_agents_allowance_follows_its_seats_class() {
    let work = AgentSpec {
        agent: "implementer",
        class: Class::Work,
        description: "",
        models: ["opus", "sonnet"],
        codex: ["sol", "terra"],
        dsh: Some(["pro", "flash"]),
        max_attempts: 2,
        timeout_seconds: 5400,
    };
    let gate = AgentSpec {
        agent: "verifier",
        class: Class::Gate,
        description: "",
        models: ["sonnet", "opus"],
        codex: ["astra", "sol"],
        dsh: None,
        max_attempts: 2,
        timeout_seconds: 3600,
    };

    let d = detected(
        "cargo build --workspace",
        "cargo test --workspace",
        "cargo clippy --workspace --all-targets -- -D warnings",
    );
    let grant = grants(Some(&d));
    let claude = |spec, grant| allowance(spec, Cli::Claude, grant);
    assert_eq!(names(claude(&work, &grant).unwrap()), names(&grant.work));
    assert_eq!(names(claude(&gate, &grant).unwrap()), names(&grant.gate));
    assert_eq!(claude(&work, &grant).unwrap().len(), 5);

    let nothing = grants(None);
    assert!(claude(&work, &nothing).is_none());
    assert!(claude(&gate, &nothing).is_none());

    // codex and dsh cannot express a tool name: no allowance, whatever
    // the stack granted.
    for cli in [Cli::Codex, Cli::Dsh] {
        assert!(allowance(&work, cli, &grant).is_none());
        assert!(allowance(&gate, cli, &grant).is_none());
    }
}

/// Each seat's hire under each CLI: its own chain, except that dsh never
/// holds the gate, which a dsh scaffold hires from claude.
#[test]
fn a_dsh_scaffold_hires_its_gate_from_claude() {
    let hires = |cli| {
        SEATS
            .iter()
            .map(|spec| (spec.agent, spec.hire(cli)))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        hires(Cli::Claude),
        [
            ("intake", (Cli::Claude, ["sonnet", "opus"])),
            ("implementer", (Cli::Claude, ["opus", "sonnet"])),
            ("reviewer", (Cli::Claude, ["fable", "opus"])),
        ]
    );
    assert_eq!(
        hires(Cli::Codex),
        [
            ("intake", (Cli::Codex, ["sol", "terra"])),
            ("implementer", (Cli::Codex, ["sol", "terra"])),
            ("reviewer", (Cli::Codex, ["astra", "sol"])),
        ]
    );
    assert_eq!(
        hires(Cli::Dsh),
        [
            ("intake", (Cli::Dsh, ["flash", "pro"])),
            ("implementer", (Cli::Dsh, ["pro", "flash"])),
            ("reviewer", (Cli::Claude, ["fable", "opus"])),
        ]
    );
}

/// The scaffold's README names the adapters it wrote, and a scaffold
/// with no claude-hired seat says it has no tool map rather than
/// describing claude's.
#[test]
fn the_readme_names_the_adapters_written_and_no_tool_map_without_claude() {
    let cargo = detected("cargo build", "cargo test", "cargo clippy");
    let codex = stack_readme(Some(&cargo), &[Cli::Codex]);
    assert!(
        codex.contains("- `adapters/codex.json` and `adapters/exec.json` — the model and"),
        "{codex}"
    );
    assert!(codex.ends_with(NO_TOOL_MAP), "{codex}");
    assert!(!codex.contains("claude.json"), "{codex}");
    let dsh = stack_readme(Some(&cargo), &[Cli::Dsh, Cli::Claude]);
    assert!(
        dsh.contains(
            "- `adapters/dsh.json`, `adapters/claude.json` and `adapters/exec.json` — the model and"
        ),
        "{dsh}"
    );
    assert!(
        dsh.contains("`adapters/claude.json` maps each tool"),
        "{dsh}"
    );
}

/// The agent CLI is the first of claude, codex and dsh on PATH, claude
/// when none is; the boundary is `harness` for a codex scaffold and on
/// macOS, and left to the `namespace` default otherwise.
#[test]
fn the_host_picks_the_cli_on_path_and_the_boundary_the_os_can_build() {
    let bins = tempfile::tempdir().unwrap();
    let dir = |name: &str, present: &[&str]| {
        let dir = bins.path().join(name);
        std::fs::create_dir(&dir).unwrap();
        for binary in present {
            std::fs::write(dir.join(binary), "").unwrap();
        }
        dir.into_os_string()
    };
    let all = dir("all", &["dsh", "codex", "claude"]);
    let codex = dir("codex", &["dsh", "codex"]);
    let dsh = dir("dsh", &["dsh"]);
    let none = dir("none", &[]);

    let picked = |path: &std::ffi::OsString, os| {
        let host = host(path, os);
        (host.cli, host.boundary)
    };
    assert_eq!(picked(&all, "linux"), (Cli::Claude, None));
    assert_eq!(
        picked(&all, "macos"),
        (Cli::Claude, Some(Boundary::Harness))
    );
    assert_eq!(
        picked(&codex, "linux"),
        (Cli::Codex, Some(Boundary::Harness))
    );
    assert_eq!(picked(&dsh, "linux"), (Cli::Dsh, None));
    assert_eq!(picked(&dsh, "macos"), (Cli::Dsh, Some(Boundary::Harness)));
    assert_eq!(picked(&none, "linux"), (Cli::Claude, None));

    assert_eq!(
        host(&all, "linux").notes,
        ["Scaffolded for `claude`, the first of claude, codex and dsh found on PATH."]
    );
    let fallback = host(&none, "macos").notes;
    assert_eq!(fallback.len(), 2, "{fallback:?}");
    assert!(
        fallback[0].starts_with("No agent CLI (claude, codex or dsh) was found on PATH"),
        "{fallback:?}"
    );
    assert!(
        fallback[1].contains("bubblewrap 0.10 or newer, which is Linux-only"),
        "{fallback:?}"
    );
    let dsh_notes = host(&dsh, "linux").notes;
    assert_eq!(dsh_notes.len(), 3, "{dsh_notes:?}");
    assert!(
        dsh_notes[2].starts_with("The review gate cannot be held by dsh"),
        "{dsh_notes:?}"
    );
    let codex_notes = host(&codex, "linux").notes;
    assert_eq!(codex_notes.len(), 3, "{codex_notes:?}");
    assert!(
        codex_notes[2].contains("codex holds each seat's hands under its own sandbox"),
        "{codex_notes:?}"
    );
}

/// A declared boundary is v4's one addition; without one the map stays
/// at v3, byte for byte what it was.
#[test]
fn a_declared_boundary_writes_a_v4_map() {
    let path = std::path::Path::new(".");
    let absent = DialectDetection::Absent;
    let v3: serde_json::Value = serde_json::from_str(&realms_json(&absent, path, None)).unwrap();
    assert_eq!(v3["schema"], "forge.realms/v3");
    assert!(v3["realms"][0].get("boundary").is_none(), "{v3}");
    let v4: serde_json::Value =
        serde_json::from_str(&realms_json(&absent, path, Some(Boundary::Harness))).unwrap();
    assert_eq!(v4["schema"], "forge.realms/v4");
    assert_eq!(v4["realms"][0]["boundary"], "harness");
}

/// A tiny `Detected` for the derivation's own tests: the identity fields
/// are prose this module never reads.
fn detected(build: &str, test: &str, lint: &str) -> Detected {
    Detected {
        name: "test".to_string(),
        evidence: "`Cargo.toml`".to_string(),
        build: build.to_string(),
        test: test.to_string(),
        lint: lint.to_string(),
        note: None,
    }
}

fn names(tools: &[Tool]) -> Vec<&str> {
    tools.iter().map(|tool| tool.name).collect()
}

fn python3() -> Tool {
    Tool {
        name: "python3",
        permission: "Bash(python3:*)",
    }
}

fn pytest() -> Tool {
    Tool {
        name: "pytest",
        permission: "Bash(.venv/bin/pytest:*)",
    }
}
