//! Unit tests for the tool-grant derivation in `init.rs` — the half of
//! the feature that turns the detection tables' command text into the
//! `Bash(...)` grants a scaffold writes. The end-to-end proofs (per-fixture
//! adapter maps, per-class allowances, resolved argv) live in
//! `tests/init_stacks.rs`, where a reader sees the whole scaffold; these
//! cover the derivation's own corners — the deduplication, the class
//! split, and the refusals for a runner no table names — that only a
//! synthetic `Detected` can reach.

use super::{
    agent_json, allowance, command_tools, detect, dialect_files, grants, host, leading_word,
    realms_json, runner_tools, stack_readme, tools_for, verify_script, write_files, AgentSpec,
    Boundary, Claims, Class, Cli, Detected, DialectDetection, Effort, Host, Tool, NO_TOOL_MAP,
    OPENSPEC, PRE_APPROVAL, REVIEW_GATE, SEATS,
};
use std::panic::{catch_unwind, AssertUnwindSafe};

use brokkr_protocol::native_controls::Access;

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
fn dialect_writes_surface_a_target_that_becomes_unwritable() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("dialects/openspec.json")).unwrap();
    assert!(write_files(dir.path(), &dialect_files(OPENSPEC)).is_err());
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
/// narrower `Bash(.venv/bin/pytest:*)` expression.
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

/// No recognized stack means no runner, and the base set alone: the work
/// seats' four and the gate's read trio, which the adapter map carries
/// (#717: a no-marker implement seat was refused `git add`).
#[test]
fn an_unrecognized_stack_grants_the_base_set_alone() {
    let grant = grants(None);
    assert_eq!(names(&grant.work), ["git", "ls", "rg", "mkdir"]);
    assert_eq!(names(&grant.gate), ["git", "ls", "rg"]);
}

/// The allowance an agent is written with follows the class of the seat
/// that names it, a stack recognized or not.
#[test]
fn an_agents_allowance_follows_its_seats_class() {
    let work = AgentSpec {
        agent: "implementer",
        class: Class::Work,
        description: "",
        models: &[("opus", Effort::High)],
        codex: &[("sol", Effort::Medium)],
        dsh: Some(&[("pro", Effort::High)]),
        access: None,
        max_attempts: 2,
        timeout_seconds: 5400,
    };
    let gate = AgentSpec {
        agent: "verifier",
        class: Class::Gate,
        description: "",
        models: &[("sonnet", Effort::High)],
        codex: &[("sol", Effort::Medium)],
        dsh: None,
        access: None,
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

    let base = grants(None);
    assert_eq!(
        names(claude(&work, &base).unwrap()),
        ["git", "ls", "rg", "mkdir"]
    );
    assert_eq!(names(claude(&gate, &base).unwrap()), ["git", "ls", "rg"]);

    // codex and dsh cannot express a tool name: no allowance, whatever
    // the stack granted.
    for cli in [Cli::Codex, Cli::Dsh] {
        assert!(allowance(&work, cli, &grant).is_none());
        assert!(allowance(&gate, cli, &grant).is_none());
    }
}

/// The scaffold's access is the loader's typed vocabulary, not a literal:
/// the reviewer's `Access::ReadOnly` serializes to the one word
/// `read-only`, and every access the scaffold writes decodes back through
/// the loader's own `Access` decode.
#[test]
fn the_scaffold_writes_the_loaders_typed_access_vocabulary() {
    let reviewer = SEATS
        .iter()
        .find(|spec| spec.agent == "reviewer")
        .expect("the scaffold seats a reviewer");
    assert_eq!(reviewer.access, Some(Access::ReadOnly));
    let definition: serde_json::Value =
        serde_json::from_str(&agent_json(reviewer, reviewer.models, None, None))
            .expect("an agent definition parses");
    assert_eq!(definition["access"], serde_json::json!("read-only"));

    for spec in SEATS.iter().filter(|spec| spec.access.is_some()) {
        let definition: serde_json::Value =
            serde_json::from_str(&agent_json(spec, spec.models, None, None))
                .expect("an agent definition parses");
        let word = definition
            .get("access")
            .unwrap_or_else(|| panic!("{} writes an access", spec.agent))
            .clone();
        assert_eq!(
            serde_json::from_value::<Access>(word).ok(),
            Some(spec.access.expect("a filtered seat has an access")),
            "{}",
            spec.agent
        );
    }
}

/// Each seat's hire under each CLI: its own chain, except that dsh never
/// holds the gate, which a dsh scaffold hires from claude. Every link
/// carries its effort as data: high, except sol, whose scale sits a step
/// lower (decision 0045's addendum).
#[test]
fn a_dsh_scaffold_hires_its_gate_from_claude() {
    const H: Effort = Effort::High;
    const M: Effort = Effort::Medium;
    let hires = |cli| {
        SEATS
            .iter()
            .map(|spec| (spec.agent, spec.hire(cli)))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        hires(Cli::Claude),
        [
            ("intake", (Cli::Claude, &[("sonnet", H), ("opus", H)][..])),
            (
                "implementer",
                (Cli::Claude, &[("opus", H), ("sonnet", H)][..])
            ),
            ("reviewer", (Cli::Claude, &[("fable", H), ("opus", H)][..])),
        ]
    );
    assert_eq!(
        hires(Cli::Codex),
        [
            ("intake", (Cli::Codex, &[("sol", M), ("terra", H)][..])),
            ("implementer", (Cli::Codex, &[("sol", M), ("terra", H)][..])),
            ("reviewer", (Cli::Codex, &[("sol", M)][..])),
        ]
    );
    assert_eq!(
        hires(Cli::Dsh),
        [
            ("intake", (Cli::Dsh, &[("flash", H), ("pro", H)][..])),
            ("implementer", (Cli::Dsh, &[("pro", H), ("flash", H)][..])),
            ("reviewer", (Cli::Claude, &[("fable", H), ("opus", H)][..])),
        ]
    );
}

/// The scaffold's README names the adapters it wrote, and a scaffold
/// with no claude-hired seat says it has no tool map rather than
/// describing claude's.
#[test]
fn the_readme_names_the_adapters_written_and_no_tool_map_without_claude() {
    let cargo = detected("cargo build", "cargo test", "cargo clippy");
    let codex = stack_readme(Some(&cargo), &[Cli::Codex], claims(Some(Boundary::Harness)));
    assert!(
        codex.contains("- `adapters/codex.json` and `adapters/exec.json` — the model and"),
        "{codex}"
    );
    assert!(codex.ends_with(NO_TOOL_MAP), "{codex}");
    assert!(!codex.contains("claude.json"), "{codex}");
    let dsh = stack_readme(Some(&cargo), &[Cli::Dsh, Cli::Claude], claims(None));
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

/// The README says the work seats' grant pre-approves and removes no
/// tool, never that a seat may run only what its charter names (#366),
/// and that only the native powers the realm does not grant are denied
/// (decision 0065).
#[test]
fn the_readme_says_the_work_grant_pre_approves_and_removes_no_tool() {
    let cargo = detected("cargo build", "cargo test", "cargo clippy");
    let readme = stack_readme(Some(&cargo), &[Cli::Claude], claims(None));
    let work = readme
        .split("\n\n")
        .find(|paragraph| paragraph.starts_with("Work-class seats"))
        .expect("a work-class paragraph");
    assert_eq!(
        work,
        "Work-class seats (intake, implement) are pre-approved for the whole set:\n\
         cargo, git, ls, rg, mkdir. Pre-approval removes no tool: an unboxed seat keeps the\n\
         harness's other defaults and your own permission settings. Only\n\
         `WebSearch` and `WebFetch` are denied, by name, because\n\
         `realms.json` grants neither."
    );
}

/// No README the scaffold writes calls a `tools` list a restriction: a
/// list pre-approves and removes no tool (#366).
#[test]
fn no_scaffold_readme_calls_a_tool_list_a_restriction() {
    let cargo = detected("cargo build", "cargo test", "cargo clippy");
    for readme in [
        stack_readme(Some(&cargo), &[Cli::Claude], claims(None)),
        stack_readme(None, &[Cli::Claude], claims(None)),
        stack_readme(Some(&cargo), &[Cli::Codex], claims(Some(Boundary::Harness))),
    ] {
        assert_eq!(
            readme.matches("restriction").collect::<Vec<_>>(),
            Vec::<&str>::new(),
            "{readme}"
        );
    }
}

/// What the scaffold says of the boundary `boundary`, from a claude host.
fn claims(boundary: Option<Boundary>) -> Claims {
    let host = Host {
        cli: Cli::Claude,
        boundary,
        notes: Vec::new(),
    };
    host.claims()
}

/// The README and the note a passing verify journals claim the box and a
/// denied network only under `namespace`. Under every other boundary the
/// scripts run under no box of Brokkr's, and a note saying "network
/// denied" would record a control that was not applied (#366).
#[test]
fn the_scaffold_claims_the_box_and_a_denied_network_only_under_namespace() {
    let cargo = detected("cargo build", "cargo test", "cargo clippy");
    let said = |boundary: Option<Boundary>| {
        let readme = stack_readme(Some(&cargo), &[Cli::Claude], claims(boundary));
        let script = verify_script(Some(&cargo), None, claims(boundary));
        let lines = |text: &str, with: &str, count: usize| {
            let at = text.lines().position(|line| line.contains(with));
            let at = at.expect(with);
            text.lines()
                .skip(at)
                .take(count)
                .collect::<Vec<_>>()
                .join("\n")
        };
        [
            lines(&readme, "- `bundle.json`", 2),
            lines(&readme, "own commands", 1),
            lines(&readme, "Verify and ship", 2),
            lines(&script, " passed ", 1),
        ]
    };
    let boxed = [
        "- `bundle.json` — three model offices plus the verify and ship exec\n  \
         gates, which `namespace` boxes,",
        "names this repository's own commands and runs in Brokkr's box, without network.",
        "cargo, git, ls, rg. Verify and ship are exec scripts with no model grant,\n\
         and `namespace` boxes them.",
        "printf '%s and %s passed boxed under namespace, with no network' \
         \"$test_command\" \"$lint_command\" > \"$notes\"",
    ];
    let unboxed = [
        "- `bundle.json` — three model offices plus the verify and ship exec\n  \
         gates, which no box of Brokkr's holds,",
        "names this repository's own commands and runs under no box of Brokkr's, \
         with no network denial confirmed.",
        "cargo, git, ls, rg. Verify and ship are exec scripts with no model grant,\n\
         and no box of Brokkr's holds them.",
        "printf '%s and %s passed unboxed, with no network denial confirmed' \
         \"$test_command\" \"$lint_command\" > \"$notes\"",
    ];
    for namespace in [None, Some(Boundary::Namespace)] {
        assert_eq!(said(namespace), boxed, "{namespace:?}");
    }
    for other in [
        Boundary::Harness,
        Boundary::Open,
        Boundary::Seatbelt,
        Boundary::Container,
    ] {
        assert_eq!(said(Some(other)), unboxed, "{other:?}");
    }
}

/// Words that claim a box, a network, a read-only surface or a tool
/// bound. Outside the sentences `claims.rs` gives, no generated prose
/// may carry one.
const CLAIM_WORDS: [&str; 5] = ["box", "network", "read-only", "sandbox", "restrict"];

/// Every host arm `init` can take — the `namespace` default, `harness` on
/// macOS, a codex scaffold and a dsh one on either host — writes each
/// sentence its claims give, once, into the whole `agents/README.md`, the
/// scripts and the notes `init` prints; with those sentences taken out, no
/// claim word is left in the README or the scripts. So no literal in `init`
/// restates a claim, and one written beside the claims fails here on
/// the arm it is false for (#366). Each arm names its claim set itself,
/// so an arm of `claims()` that hands a host the wrong set fails too.
#[test]
fn every_host_arm_says_its_claims_and_no_other() {
    use super::claims::{CODEX, MACOS, NAMESPACE};
    let root = tempfile::tempdir().unwrap();
    let harness = Some(Boundary::Harness);
    let arms = [
        ("namespace", "claude", "linux", Cli::Claude, None, NAMESPACE),
        ("macos", "claude", "macos", Cli::Claude, harness, MACOS),
        ("codex", "codex", "linux", Cli::Codex, None, NAMESPACE),
        ("codex-macos", "codex", "macos", Cli::Codex, harness, CODEX),
        ("dsh", "dsh", "linux", Cli::Dsh, None, NAMESPACE),
        ("dsh-macos", "dsh", "macos", Cli::Dsh, harness, MACOS),
    ];
    let mut wrong = Vec::new();
    for (arm, binary, os, cli, boundary, claims) in arms {
        let bins = root.path().join(format!("{arm}-bin"));
        std::fs::create_dir(&bins).unwrap();
        std::fs::write(bins.join(binary), "").unwrap();
        // Each arm's own repository root, a cargo workspace.
        let repo = root.path().join(arm);
        std::fs::create_dir(&repo).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[workspace]\n").unwrap();
        let scaffold = super::init(&repo, bins.as_os_str(), os, root.path(), None).unwrap();
        if super::claims::claims(cli, boundary) != claims {
            wrong.push(format!("{arm}: claims() hands this host another set"));
        }
        let read = |path: &str| std::fs::read_to_string(repo.join(".brokkr").join(path)).unwrap();
        let mut said = vec![claims.gates, claims.runs];
        if cli != Cli::Codex {
            said.extend([claims.scripts, PRE_APPROVAL, REVIEW_GATE]);
        }
        said.extend(claims.realm);
        let files = [
            (read("agents/README.md"), said),
            (read("scripts/verify-seat.sh"), vec![claims.note]),
            (read("scripts/ship-seat.sh"), Vec::new()),
            (scaffold.notes.join("\n\n"), Vec::from_iter(claims.realm)),
        ];
        for (text, said) in files {
            wrong.extend(
                unsaid(&text, &said)
                    .into_iter()
                    .map(|at| format!("{arm}: {at}")),
            );
        }
    }
    assert_eq!(wrong, Vec::<String>::new());
}

/// What is wrong with one generated text against the claim sentences it
/// must say: each sentence not said exactly once, then each claim word
/// left once those sentences are taken out.
fn unsaid(text: &str, said: &[&str]) -> Vec<String> {
    let mut rest = text.to_string();
    let mut wrong = Vec::new();
    for sentence in said {
        if rest.matches(sentence).count() != 1 {
            wrong.push(format!("not said once: {sentence:?}"));
        }
        rest = rest.replace(sentence, "");
    }
    let rest = rest.to_lowercase();
    let left = CLAIM_WORDS.iter().filter(|word| rest.contains(*word));
    wrong.extend(left.map(|word| format!("{word:?} outside claims.rs")));
    wrong
}

/// The agent CLI is the first of claude, codex and dsh on PATH, claude
/// when none is; the boundary is `harness` on macOS, and left to the
/// `namespace` default otherwise, a codex scaffold's included (#717).
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
        let host = host(path, os, None);
        (host.cli, host.boundary)
    };
    assert_eq!(picked(&all, "linux"), (Cli::Claude, None));
    assert_eq!(
        picked(&all, "macos"),
        (Cli::Claude, Some(Boundary::Harness))
    );
    assert_eq!(picked(&codex, "linux"), (Cli::Codex, None));
    assert_eq!(
        picked(&codex, "macos"),
        (Cli::Codex, Some(Boundary::Harness))
    );
    assert_eq!(picked(&dsh, "linux"), (Cli::Dsh, None));
    assert_eq!(picked(&dsh, "macos"), (Cli::Dsh, Some(Boundary::Harness)));
    assert_eq!(picked(&none, "linux"), (Cli::Claude, None));

    assert_eq!(
        host(&all, "linux", None).notes,
        ["Scaffolded for `claude`, the first of claude, codex and dsh found on PATH."]
    );
    let fallback = host(&none, "macos", None).notes;
    assert_eq!(fallback.len(), 2, "{fallback:?}");
    assert!(
        fallback[0].starts_with("No agent CLI (claude, codex or dsh) was found on PATH"),
        "{fallback:?}"
    );
    assert!(
        fallback[1].contains("bubblewrap 0.11 or newer, which is Linux-only"),
        "{fallback:?}"
    );
    let dsh_notes = host(&dsh, "linux", None).notes;
    assert_eq!(dsh_notes.len(), 3, "{dsh_notes:?}");
    assert!(
        dsh_notes[2].starts_with("The review gate cannot be held by dsh"),
        "{dsh_notes:?}"
    );
    assert_eq!(host(&codex, "linux", None).notes.len(), 2);
    let codex_notes = host(&codex, "macos", None).notes;
    assert_eq!(codex_notes.len(), 3, "{codex_notes:?}");
    assert!(
        codex_notes[2].contains("codex holds each seat's hands under its own sandbox"),
        "{codex_notes:?}"
    );
}

/// `--cli` chooses in place of PATH's order (#717). Named and found, the
/// note says `--cli` chose; a named dsh beside claude says the gate's
/// claude is there. Named and absent, the scaffold is still written for
/// it, its note naming the binary to install, and nothing is run.
#[test]
fn a_named_cli_is_chosen_over_path_order_and_said_so() {
    let bins = tempfile::tempdir().unwrap();
    let (all, none) = (bins.path().join("all"), bins.path().join("none"));
    for dir in [&all, &none] {
        std::fs::create_dir(dir).unwrap();
    }
    for binary in ["claude", "codex", "dsh"] {
        std::fs::write(all.join(binary), "").unwrap();
    }
    let named = host(all.as_os_str(), "linux", Some(Cli::Dsh));
    assert_eq!((named.cli, named.boundary), (Cli::Dsh, None));
    assert_eq!(
        named.notes,
        [
            "Scaffolded for `dsh`, as --cli named.",
            "`dsh` cannot express a tool name, so no `dsh`-hired agent carries a tool \
             allowance; where this README speaks of tool grants, it speaks of the \
             claude-hired seats only.",
            "The review gate cannot be held by dsh: its adapter is untrusted and names no \
             judges, and a gate seat judges only on a trusted adapter's judge (decision \
             0021). So intake and implement are hired from dsh and `agents/reviewer.json` \
             is hired from claude.",
        ]
    );
    let codex = host(all.as_os_str(), "linux", Some(Cli::Codex));
    assert_eq!((codex.cli, codex.boundary), (Cli::Codex, None));
    assert_eq!(codex.notes[0], "Scaffolded for `codex`, as --cli named.");
    let missing = host(none.as_os_str(), "linux", Some(Cli::Dsh));
    assert_eq!((missing.cli, missing.notes.len()), (Cli::Dsh, 3));
    assert_eq!(
        missing.notes[0],
        "Scaffolded for `dsh`, as --cli named, but no `dsh` was found on PATH. \
         Install `dsh` before the first run."
    );
    assert!(
        missing.notes[2].ends_with(
            "hired from claude, which is not on PATH — install claude before a run \
             reaches review."
        ),
        "{:?}",
        missing.notes
    );
}

/// One composed argv's shape: what it runs (`driver`, or `hands` for
/// Brokkr's box), each sandbox class it hands codex, and each key of the
/// brokkr MCP server it configures.
fn shape(argv: &[String]) -> (String, Vec<String>, Vec<String>) {
    let classes = argv.windows(2).filter(|pair| pair[0] == "--sandbox");
    let served = argv
        .iter()
        .filter_map(|part| part.strip_prefix("mcp_servers.brokkr."));
    (
        argv[1].clone(),
        classes.map(|pair| pair[1].clone()).collect(),
        served
            .map(|part| part.split('=').next().unwrap().to_string())
            .collect(),
    )
}

/// On Linux a codex scaffold stands under `namespace` (#717). Compiled and
/// composed under the boundary its own `realms.json` declares, each codex
/// seat — the review gate as well as intake and implement — runs codex
/// read-only with Brokkr's workspace tool served, the adapter's
/// `hands.workspace` shape, and never codex's `workspace-write` class,
/// which keeps `.git` read-only; verify and ship run in Brokkr's box.
#[test]
fn a_codex_scaffold_on_linux_seats_codex_on_brokkrs_hands_and_boxes_its_scripts() {
    codex_seats_on_linux(&["codex"], None);
}

/// The same scaffold when `--cli codex` names it on a PATH that holds
/// claude, codex and dsh, whose own order would choose claude (#717).
#[test]
fn a_named_codex_beside_claude_seats_codex_on_brokkrs_hands_on_linux() {
    codex_seats_on_linux(&["claude", "codex", "dsh"], Some(Cli::Codex));
}

/// `init` on Linux under a PATH holding `present`, with `named` from
/// `--cli`, composes every seat as a codex scaffold under `namespace`.
fn codex_seats_on_linux(present: &[&str], named: Option<Cli>) {
    use brokkr_runtime::engine::{compose_site, BuiltBoundary};
    use brokkr_runtime::{argv::Executable, Invocation, SeatBody};
    let root = tempfile::tempdir().unwrap();
    let [bins, repo, home] = ["bin", "repo", "home"].map(|name| root.path().join(name));
    for dir in [&bins, &repo] {
        std::fs::create_dir(dir).unwrap();
    }
    for binary in present {
        std::fs::write(bins.join(binary), "").unwrap();
    }
    let scaffold = super::init(&repo, bins.as_os_str(), "linux", &home, named).unwrap();
    let folder = repo.join(super::FOLDER);
    let map = std::fs::read(folder.join("realms.json")).unwrap();
    let map: serde_json::Value = serde_json::from_slice(&map).unwrap();
    let (boundary, built) = match map["realms"][0]["boundary"].as_str() {
        None => (Boundary::Namespace, BuiltBoundary::Namespace),
        Some("harness") => (Boundary::Harness, BuiltBoundary::Harness),
        Some(other) => panic!("init declares no {other}"),
    };
    let (agents, adapters) = (folder.join("agents"), folder.join("adapters"));
    let compiled = brokkr_runtime::Bundle::compile_under(&folder, &agents, &adapters, boundary);
    let compiled = compiled.unwrap();
    let engine = Executable::current();
    let roots = [folder.clone()];
    let composed: Vec<_> = ["intake", "implement", "review", "verify", "ship"]
        .into_iter()
        .map(|seat| {
            let SeatBody::Invocation(Invocation::Single {
                command,
                candidates,
                class,
                ..
            }) = &compiled.seats[seat].body
            else {
                panic!("{seat} is a single seat")
            };
            let hands = compiled.hands.get(seat);
            let facts = brokkr_protocol::hands::git_facts(&repo);
            let seal = |spec| brokkr_protocol::hands::seal(spec, &home, &repo, &facts, None);
            let sealed = hands.map(seal);
            let boxed = (&engine, root.path(), sealed.as_ref());
            let (link, result) = (candidates.first(), "/r/p.json");
            let spawn = compose_site(
                built,
                *class,
                command.clone(),
                hands,
                link,
                &repo,
                boxed,
                &roots,
                result,
                None,
            );
            (seat, spawn.refusal, shape(&spawn.argv))
        })
        .collect();
    let served = ["command", "args", "default_tools_approval_mode"].map(String::from);
    let codex = || {
        (
            "driver".to_string(),
            vec!["read-only".to_string()],
            served.to_vec(),
        )
    };
    let boxed = || ("hands".to_string(), Vec::new(), Vec::new());
    assert_eq!(
        (scaffold.namespace, scaffold.hands, composed),
        (
            true,
            ["implement", "intake", "review", "ship", "verify"]
                .map(String::from)
                .to_vec(),
            vec![
                ("intake", None, codex()),
                ("implement", None, codex()),
                ("review", None, codex()),
                ("verify", None, boxed()),
                ("ship", None, boxed()),
            ]
        )
    );
}

/// A declared boundary is v4's one addition; without one the map stays
/// at v3. Either way the map stands in the folder: its realm is the
/// folder's parent and its journal in the folder's state (decision 0083
/// ruling 1).
#[test]
fn a_declared_boundary_writes_a_v4_map() {
    let absent = DialectDetection::Absent;
    let v3: serde_json::Value = serde_json::from_str(&realms_json(&absent, None)).unwrap();
    assert_eq!(v3["schema"], "forge.realms/v3");
    assert!(v3["realms"][0].get("boundary").is_none(), "{v3}");
    let v4: serde_json::Value =
        serde_json::from_str(&realms_json(&absent, Some(Boundary::Harness))).unwrap();
    assert_eq!(v4["schema"], "forge.realms/v4");
    assert_eq!(v4["realms"][0]["boundary"], "harness");
    for map in [v3, v4] {
        let placed = (&map["realms"][0]["path"], &map["journal"]);
        assert_eq!(
            placed,
            (
                &serde_json::json!(".."),
                &serde_json::json!("state/forge.db")
            )
        );
    }
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
