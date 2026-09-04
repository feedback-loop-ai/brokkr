//! `brokkr init` looks before it scaffolds: the repository it is invoked
//! from is read for the manifests and lockfiles at its root, and the two
//! charter and verifier script that tell seats to build and prove name that
//! stack's own commands. What is asserted here is what an operator would
//! read in the generated files — and, for a repository carrying no marker
//! init knows, that the charter says so in those words instead of dressing
//! a placeholder as a choice.
//!
//! The same table decides what the seats may RUN: the binary each command
//! invokes is written into the scaffold's adapter as a tool permission and
//! granted to the scaffolded agents — the whole set to the work seats, the
//! read-only subset to the gates — and what is asserted is the resolved
//! argv the compiler composes for the implement seat, because a grant that
//! never reached `--allowedTools` is no grant.
//!
//! Every scaffold is made in a tempdir the fixture's markers are COPIED
//! into. Never in the checked-in fixture itself: `init` writes, and a
//! test that wrote into the committed tree would be a test that edited
//! the repository to pass.

use std::path::{Path, PathBuf};
use std::process::Command;

use brokkr_runtime::bundle::{DEFAULT_ADAPTERS_DIR, DEFAULT_AGENTS_DIR};
use brokkr_runtime::{Bundle, SeatBody};
use serde_json::Value;

/// Fixture, the name the charters call the stack, then build, test, lint.
/// The same shape as the table in `init.rs`, written out again rather
/// than imported, so a command silently changed there fails here.
const RECOGNIZED: &[(&str, &str, &str, &str, &str)] = &[
    (
        "rust",
        "rust",
        "cargo build --workspace",
        "cargo test --workspace",
        "cargo clippy --workspace --all-targets -- -D warnings",
    ),
    (
        "node-bun",
        "node/bun",
        "bun install --frozen-lockfile",
        "bun run test",
        "bun run typecheck",
    ),
    (
        "node-pnpm",
        "node/pnpm",
        "pnpm build",
        "pnpm test",
        "pnpm lint",
    ),
    (
        "node-yarn",
        "node/yarn",
        "yarn build",
        "yarn test",
        "yarn lint",
    ),
    (
        "node-npm",
        "node/npm",
        "npm run build",
        "npm test",
        "npm run lint",
    ),
    (
        "python-uv",
        "python/uv",
        "uv sync",
        "uv run pytest",
        "uv run ruff check .",
    ),
    // The plain-python row runs through `python3`, the interpreter a fresh
    // project actually resolves (the shipped adapters grant it that name).
    (
        "python",
        "python",
        "python3 -m build",
        "python3 -m pytest",
        "python3 -m ruff check .",
    ),
    (
        "go",
        "go",
        "go build ./...",
        "go test ./...",
        "go vet ./...",
    ),
    ("make", "make", "make build", "make test", "make lint"),
];

/// The orchestrator half of the same table: fixture, the name the
/// charters call it, then build, test, lint — with the package-manager
/// prefix already resolved from whichever lockfile the fixture carries.
/// Same shape, same reason for being written out again rather than
/// imported: a run-template quietly changed in `init.rs` fails here.
const MONOREPOS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "turbo-pnpm",
        "node/turbo",
        "pnpm exec turbo run build",
        "pnpm exec turbo run test",
        "pnpm exec turbo run lint",
    ),
    (
        "turbo-bun",
        "node/turbo",
        "bunx turbo run build",
        "bunx turbo run test",
        "bunx turbo run lint",
    ),
    // No lockfile at all: `npx` is what is left, and it resolves the
    // repository's own local install before it reaches for the registry.
    (
        "turbo-plain",
        "node/turbo",
        "npx turbo run build",
        "npx turbo run test",
        "npx turbo run lint",
    ),
    (
        "nx-yarn",
        "node/nx",
        "yarn exec nx run-many -t build",
        "yarn exec nx run-many -t test",
        "yarn exec nx run-many -t lint",
    ),
];

/// The tool-grant table, asserted per stack: fixture, the WORK allowance
/// (intake, implement — the full set: the stack's runners plus git, ls,
/// rg and mkdir), then the GATE allowance (verify, review, ship — the
/// read-only subset: the test runner's tools plus git, ls and rg, never
/// the write tool). Written out again rather than imported, like every
/// other table here: a grant silently widened or narrowed in `init.rs`
/// fails here.
const TOOLS: &[(&str, &[&str], &[&str])] = &[
    (
        "rust",
        &["cargo", "git", "ls", "rg", "mkdir"],
        &["cargo", "git", "ls", "rg"],
    ),
    (
        "node-bun",
        &["bun", "git", "ls", "rg", "mkdir"],
        &["bun", "git", "ls", "rg"],
    ),
    (
        "node-pnpm",
        &["pnpm", "git", "ls", "rg", "mkdir"],
        &["pnpm", "git", "ls", "rg"],
    ),
    (
        "node-yarn",
        &["yarn", "git", "ls", "rg", "mkdir"],
        &["yarn", "git", "ls", "rg"],
    ),
    (
        "node-npm",
        &["npm", "git", "ls", "rg", "mkdir"],
        &["npm", "git", "ls", "rg"],
    ),
    (
        "python-uv",
        &["uv", "git", "ls", "rg", "mkdir"],
        &["uv", "git", "ls", "rg"],
    ),
    // The plain-python row: the interpreter the commands run through plus
    // pytest, the venv's own suite binary.
    (
        "python",
        &["python3", "pytest", "git", "ls", "rg", "mkdir"],
        &["python3", "pytest", "git", "ls", "rg"],
    ),
    (
        "go",
        &["go", "git", "ls", "rg", "mkdir"],
        &["go", "git", "ls", "rg"],
    ),
    (
        "make",
        &["make", "git", "ls", "rg", "mkdir"],
        &["make", "git", "ls", "rg"],
    ),
    // A monorepo runs through whichever runner its lockfile picked.
    (
        "turbo-pnpm",
        &["pnpm", "git", "ls", "rg", "mkdir"],
        &["pnpm", "git", "ls", "rg"],
    ),
    (
        "turbo-bun",
        &["bunx", "git", "ls", "rg", "mkdir"],
        &["bunx", "git", "ls", "rg"],
    ),
    (
        "turbo-plain",
        &["npx", "git", "ls", "rg", "mkdir"],
        &["npx", "git", "ls", "rg"],
    ),
    (
        "nx-yarn",
        &["yarn", "git", "ls", "rg", "mkdir"],
        &["yarn", "git", "ls", "rg"],
    ),
];

/// The Bash expression each granted tool name stands for — the same
/// vocabulary the shipped adapters carry, written out again so an
/// expression silently changed in `init.rs` fails here.
fn expr(name: &str) -> String {
    match name {
        "pytest" => "Bash(.venv/bin/pytest:*)".to_string(),
        other => format!("Bash({other}:*)"),
    }
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/init-stacks")
}

fn brokkr(args: &[&str], cwd: &Path) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Copy one fixture's marker files into a fresh repository and scaffold
/// from inside it — which is how `init` is used: the recipe lands in a
/// subdirectory, the repository it describes is the one you stand in.
/// The `TempDir` comes back with the path so the tree outlives the call.
fn scaffold_from(fixture: &str) -> (tempfile::TempDir, PathBuf) {
    let repo = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(fixtures().join(fixture)).unwrap() {
        let marker = entry.unwrap().path();
        std::fs::copy(&marker, repo.path().join(marker.file_name().unwrap())).unwrap();
    }
    let bundle = repo.path().join("bundle");
    let (code, _, stderr) = brokkr(&["init", bundle.to_str().unwrap()], repo.path());
    assert_eq!(code, Some(0), "{fixture}: {stderr}");
    (repo, bundle)
}

/// A charter is the agent of the same name's text, scaffolded under
/// `agents/charters/` — inside the library root the compiler resolves it
/// against (decision 0016).
fn charter(bundle: &Path, name: &str) -> String {
    std::fs::read_to_string(bundle.join(DEFAULT_AGENTS_DIR).join("charters").join(name)).unwrap()
}

fn verifier(bundle: &Path) -> String {
    std::fs::read_to_string(bundle.join("scripts/verify-seat.sh")).unwrap()
}

fn verifier_commands(script: &str) -> Vec<&str> {
    script
        .lines()
        .filter_map(|line| {
            line.strip_prefix("test_command='")
                .or_else(|| line.strip_prefix("lint_command='"))
                .and_then(|command| command.strip_suffix('\''))
        })
        .collect()
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// One scaffolded agent definition.
fn agent(bundle: &Path, name: &str) -> Value {
    read_json(&bundle.join(DEFAULT_AGENTS_DIR).join(format!("{name}.json")))
}

/// The tool names one scaffolded agent's `tools.allow` lists, in order.
fn allow_of(definition: &Value) -> Vec<&str> {
    definition["tools"]["allow"]
        .as_array()
        .map(|allow| {
            allow
                .iter()
                .map(|name| name.as_str().expect("a tool name"))
                .collect()
        })
        .expect("every scaffolded agent on a recognized stack declares tools.allow")
}

/// The scaffolded `adapters/claude.json` tool map: name → Bash(...).
fn names_map(bundle: &Path) -> Value {
    read_json(&bundle.join(DEFAULT_ADAPTERS_DIR).join("claude.json"))["tool_permissions"]["names"]
        .clone()
}

/// Compile the scaffold against ITS OWN roots — the property init proves
/// when it prints its digest, asserted here from the outside.
fn compiles(bundle: &Path) -> Bundle {
    Bundle::compile_with(
        bundle,
        &bundle.join(DEFAULT_AGENTS_DIR),
        &bundle.join(DEFAULT_ADAPTERS_DIR),
    )
    .unwrap_or_else(|e| {
        panic!(
            "{} must compile under its own adapters/ and agents/: {e}",
            bundle.display()
        )
    })
}

/// One seat's resolved argv, from `driver` on — `[0]` is this machine's
/// absolute path to the brokkr executable, which is nobody's contract.
fn argv(compiled: &Bundle, seat: &str) -> Vec<String> {
    match &compiled.seats[seat].body {
        SeatBody::Single { command, .. } => command[1..].to_vec(),
        other => panic!("{seat} is not a single-agent seat: {other:?}"),
    }
}

/// The commands a charter actually names: its indented lines, whole and
/// exact. Compared as lines rather than searched for as substrings,
/// because `npm test` lives inside `pnpm test` and a substring test would
/// call the wrong stack recognized.
fn commands(charter: &str) -> Vec<&str> {
    charter
        .lines()
        .filter_map(|line| line.strip_prefix("    "))
        .map(str::trim_end)
        .collect()
}

#[test]
fn each_recognized_stack_gets_its_own_commands_and_no_others() {
    for (fixture, name, build, test, lint) in RECOGNIZED {
        let (_repo, bundle) = scaffold_from(fixture);
        let implementer = charter(&bundle, "implementer.md");
        let verifier = verifier(&bundle);

        assert!(!bundle.join("agents/verifier.json").exists());
        assert!(!bundle.join("agents/shipper.json").exists());
        assert!(!bundle.join("agents/charters/verifier.md").exists());
        assert!(!bundle.join("agents/charters/shipper.md").exists());
        assert!(bundle.join("adapters/exec.json").is_file());
        assert!(bundle.join("scripts/ship-seat.sh").is_file());

        // The seat that builds is told to build and to test; the seat
        // that proves is told to test and to lint.
        assert_eq!(commands(&implementer), vec![*build, *test], "{fixture}");
        assert_eq!(
            verifier_commands(&verifier),
            vec![*test, *lint],
            "{fixture}"
        );

        // Named in the stack's own vocabulary, with the evidence quoted
        // back so the guess can be checked — and not marked generic.
        assert!(
            implementer.contains(&format!("a {name} project")),
            "{implementer}"
        );
        assert!(
            !implementer.contains("NO STACK WAS RECOGNIZED"),
            "{implementer}"
        );

        // Nobody else's tooling arrived with it.
        let named: Vec<&str> = commands(&implementer)
            .into_iter()
            .chain(verifier_commands(&verifier))
            .collect();
        for (other, _, other_build, other_test, other_lint) in RECOGNIZED.iter().chain(MONOREPOS) {
            if other == fixture {
                continue;
            }
            for command in [other_build, other_test, other_lint] {
                assert!(
                    !named.contains(command),
                    "{fixture} charters name {other}'s {command}"
                );
            }
        }
    }
}

/// The fallback is allowed to be generic; it is not allowed to be quiet
/// about it. An operator reading a placeholder must not be able to
/// mistake it for a command chosen for their project.
#[test]
fn an_unrecognized_repository_gets_a_charter_that_says_so() {
    let (_repo, bundle) = scaffold_from("generic");
    let implementer = charter(&bundle, "implementer.md");
    let verifier = verifier(&bundle);

    assert_eq!(
        commands(&implementer),
        vec![
            "<this project's build command>",
            "<this project's test command>"
        ]
    );
    assert_eq!(verifier_commands(&verifier), ["", ""]);
    assert!(
        implementer.contains("NO STACK WAS RECOGNIZED"),
        "{implementer}"
    );
    assert!(
        implementer.contains("GENERIC placeholders"),
        "{implementer}"
    );
    assert!(
        verifier.contains("no stack was recognized; fill in scripts/verify-seat.sh"),
        "{verifier}"
    );
    for (_, _, build, test, lint) in RECOGNIZED.iter().chain(MONOREPOS) {
        for command in [build, test, lint] {
            assert!(
                !implementer.contains(command),
                "unrecognized, yet names {command}"
            );
            assert!(
                !verifier.contains(command),
                "unrecognized, yet names {command}"
            );
        }
    }
}

/// A repository carrying both a language manifest and the `Makefile` that
/// wraps it is ordinary. The manifest wins: it is the more specific
/// truth, and `make lint` in a repo whose Makefile has no `lint` target
/// is exactly the dishonest command this feature exists to stop writing.
#[test]
fn a_manifest_outranks_the_makefile_that_wraps_it() {
    let (_repo, bundle) = scaffold_from("rust-and-make");
    let implementer = charter(&bundle, "implementer.md");
    let verifier = verifier(&bundle);

    assert_eq!(
        commands(&implementer),
        vec!["cargo build --workspace", "cargo test --workspace"]
    );
    assert_eq!(
        verifier_commands(&verifier),
        vec![
            "cargo test --workspace",
            "cargo clippy --workspace --all-targets -- -D warnings"
        ]
    );
    for command in ["make build", "make test", "make lint"] {
        assert!(!commands(&implementer).contains(&command), "{implementer}");
        assert!(
            !verifier_commands(&verifier).contains(&command),
            "{verifier}"
        );
    }
}

/// The recorded regression, in one test. A bun-managed repository has
/// only `package.json` as far as the npm fallback can see, and the
/// fallback wrote `npm run build` / `npm test` into the charters of a
/// repository with no npm lockfile to install from. `bun.lock` is the
/// narrower evidence and out-votes it — and not one word of npm's
/// tooling arrives with the answer.
#[test]
fn bun_out_votes_the_npm_fallback() {
    let (_repo, bundle) = scaffold_from("node-bun");
    let implementer = charter(&bundle, "implementer.md");
    let verifier = verifier(&bundle);

    assert_eq!(
        commands(&implementer),
        vec!["bun install --frozen-lockfile", "bun run test"]
    );
    assert_eq!(
        verifier_commands(&verifier),
        vec!["bun run test", "bun run typecheck"]
    );
    {
        let text = &implementer;
        assert!(text.contains("a node/bun project"), "{text}");
        // The evidence quoted back names the lockfile that decided it.
        assert!(text.contains("`package.json` + `bun.lock`"), "{text}");
        // Not "no npm COMMAND" — no npm anywhere. A charter that so much
        // as mentions npm here is one a seat can misread at 3am.
        assert!(!text.contains("npm"), "bun charter still says npm: {text}");
    }
}

/// The same rule one language over: `uv.lock` beside `pyproject.toml`
/// out-votes the pip fallback, and the commands run inside the
/// environment uv resolved rather than whatever interpreter the seat
/// inherited.
#[test]
fn uv_out_votes_the_pip_fallback() {
    let (_repo, bundle) = scaffold_from("python-uv");
    let implementer = charter(&bundle, "implementer.md");
    let verifier = verifier(&bundle);

    assert_eq!(commands(&implementer), vec!["uv sync", "uv run pytest"]);
    assert_eq!(
        verifier_commands(&verifier),
        vec!["uv run pytest", "uv run ruff check ."]
    );
    {
        let text = &implementer;
        assert!(text.contains("a python/uv project"), "{text}");
        assert!(text.contains("`pyproject.toml` + `uv.lock`"), "{text}");
        for fallback in ["python3 -m build", "python3 -m pytest", "python3 -m ruff"] {
            assert!(!text.contains(fallback), "uv charter still says {fallback}");
        }
    }
}

/// A monorepo's orchestrator outranks any one package's manifest, and
/// which package manager runs it is read from the lockfile rather than
/// guessed. Four fixtures: pnpm, bun, no lockfile at all, and nx.
#[test]
fn a_monorepo_scaffold_names_the_orchestrators_own_commands() {
    for (fixture, name, build, test, lint) in MONOREPOS {
        let (_repo, bundle) = scaffold_from(fixture);
        let implementer = charter(&bundle, "implementer.md");
        let verifier = verifier(&bundle);

        assert_eq!(commands(&implementer), vec![*build, *test], "{fixture}");
        assert_eq!(
            verifier_commands(&verifier),
            vec![*test, *lint],
            "{fixture}"
        );

        {
            let text = &implementer;
            assert!(text.contains(&format!("a {name} project")), "{fixture}");
            // Saying so is half the deliverable: a charter that ran the
            // right command and never told the seat it was in a monorepo
            // would invite a seat to "helpfully" narrow it to one package.
            assert!(text.contains("This is a MONOREPO"), "{fixture}: {text}");
            assert!(
                text.contains("Do not substitute a single"),
                "{fixture}: {text}"
            );
            // And the single package's own scripts never appear.
            for guessed in ["npm run build", "npm test", "pnpm build", "yarn test"] {
                assert!(!commands(text).contains(&guessed), "{fixture}: {text}");
            }
        }
    }
}

/// Two of the four monorepo cases needed no new command text: `cargo
/// build --workspace` from a workspace root and `go build ./...` beside
/// a `go.work` already span every member. What was missing was the
/// charter saying so — asserted here in both directions, because a note
/// that appeared on every rust and go repository would be the same
/// dishonesty in the other direction.
#[test]
fn a_workspace_charter_says_it_is_a_workspace_and_a_lone_package_does_not() {
    let workspaces = [
        ("cargo-workspace", "This is a CARGO WORKSPACE"),
        ("go-workspace", "This is a GO WORKSPACE"),
    ];
    for (fixture, claim) in workspaces {
        let (_repo, bundle) = scaffold_from(fixture);
        let text = charter(&bundle, "implementer.md");
        assert!(text.contains(claim), "{fixture}: {text}");
    }

    // The commands did not change, because they were already right.
    let (_repo, bundle) = scaffold_from("cargo-workspace");
    assert_eq!(
        commands(&charter(&bundle, "implementer.md")),
        vec!["cargo build --workspace", "cargo test --workspace"]
    );
    let (_repo, bundle) = scaffold_from("go-workspace");
    assert_eq!(
        commands(&charter(&bundle, "implementer.md")),
        vec!["go build ./...", "go test ./..."]
    );

    // And a lone package, or a lone module, is told no such thing.
    // `rust-package`'s manifest carries the word `[workspace]` inside a
    // COMMENT: the declaration is a whole line, not a substring, and a
    // charter that read prose as a table would say this crate spans
    // members it does not have.
    for (fixture, claim) in [
        ("rust-package", "WORKSPACE"),
        ("go", "WORKSPACE"),
        ("node-npm", "MONOREPO"),
    ] {
        let (_repo, bundle) = scaffold_from(fixture);
        let text = charter(&bundle, "implementer.md");
        assert!(!text.contains(claim), "{fixture} claims {claim}");
    }
}

/// The tool-grant half of the table: every recognized stack scaffolds an
/// adapter whose `tool_permissions.names` maps exactly the granted names
/// to their `Bash(...)` expressions, and model offices sized by decision
/// 0021 ruling 1's classes — the two work agents carry the full set and
/// review carries the read-only subset. Verify and ship are exec scripts.
/// The class is read from the
/// scaffolded `bundle.json`, not assumed, so the roster the seats declare
/// and the roster the allowances were written for cannot drift apart
/// without failing here. The test style is the file's own: the table
/// above is written out again, so a grant changed in `init.rs` fails
/// here.
#[test]
fn each_stack_grants_its_own_tools_by_seat_class() {
    for (fixture, work, gate) in TOOLS {
        let (_repo, bundle) = scaffold_from(fixture);

        // The adapter map is the union of every allowance: the work set,
        // each name mapped to the Bash(...) expression the CLI needs.
        let names = names_map(&bundle);
        let mut expected = serde_json::Map::new();
        for name in *work {
            expected.insert(name.to_string(), Value::String(expr(name)));
        }
        assert_eq!(names, Value::Object(expected), "{fixture}");

        // The two work-class agents carry the full set...
        let seats = read_json(&bundle.join("bundle.json"));
        let seats = seats["seats"].as_object().unwrap();
        for (seat, declared) in seats {
            let Some(agent_name) = declared["agent"].as_str() else {
                assert!(matches!(seat.as_str(), "verify" | "ship"));
                continue;
            };
            let definition = agent(&bundle, agent_name);
            let wanted = match declared["class"].as_str().unwrap() {
                "work" => work,
                "gate" => gate,
                other => panic!("{fixture}/{seat}: unknown class {other}"),
            };
            assert_eq!(
                allow_of(&definition),
                wanted.to_vec(),
                "{fixture}/{seat} ({agent_name})"
            );
        }

        // The README names the grant in words an operator can check.
        let readme =
            std::fs::read_to_string(bundle.join(DEFAULT_AGENTS_DIR).join("README.md")).unwrap();
        assert!(!readme.contains("NO STACK WAS RECOGNIZED"), "{fixture}");
        for name in *work {
            assert!(
                readme.contains(&format!("{name} → {}", expr(name))),
                "{fixture}: README does not map {name}: {readme}"
            );
        }

        // Every allowance is expressible: the scaffold compiles under its
        // own adapters/ and agents/ (init proved the same when it printed
        // its digest).
        compiles(&bundle);
    }
}

/// The implement seat's resolved argv — the command the engine would
/// spawn for the seat that builds — carries the expected `--allowedTools`
/// list, composed from the implementer agent's `tools.allow` through the
/// adapter's names map (decision 0016). node/bun and rust/cargo are the
/// two the adoption run asked for; the gate seats' argv carries the
/// read-only subset, and never `mkdir`.
#[test]
fn the_implement_and_verify_seats_argv_carry_the_class_allowed_tools() {
    for (fixture, work, gate) in TOOLS {
        let (_repo, bundle) = scaffold_from(fixture);
        let compiled = compiles(&bundle);

        let SeatBody::Single { command, .. } = &compiled.seats["implement"].body else {
            panic!("implement is a single-agent seat")
        };
        let allowed: Vec<String> = work.iter().map(|name| expr(name)).collect();
        assert!(
            command
                .windows(2)
                .any(|pair| pair[0] == "--allowedTools" && pair[1] == allowed.join(",")),
            "{fixture}: implement argv {command:?} lacks {allowed:?}"
        );

        let SeatBody::Single { command, .. } = &compiled.seats["review"].body else {
            panic!("review is a single-agent seat")
        };
        let allowed: Vec<String> = gate.iter().map(|name| expr(name)).collect();
        assert!(
            command
                .windows(2)
                .any(|pair| pair[0] == "--allowedTools" && pair[1] == allowed.join(",")),
            "{fixture}/review: argv {command:?} lacks {allowed:?}"
        );
        for gate_seat in ["verify", "review", "ship"] {
            let SeatBody::Single { command, .. } = &compiled.seats[gate_seat].body else {
                panic!("{gate_seat} is a single seat")
            };
            assert!(
                !command.iter().any(|part| part.contains("mkdir")),
                "{fixture}/{gate_seat}: a gate carries the write tool: {command:?}"
            );
        }
        for exec_seat in ["verify", "ship"] {
            let SeatBody::Single {
                command,
                candidates,
                ..
            } = &compiled.seats[exec_seat].body
            else {
                panic!("{exec_seat} is a single exec seat")
            };
            assert!(candidates.is_empty(), "{fixture}/{exec_seat} seats a model");
            assert_eq!(&command[1..4], ["driver", "exec", "--"]);
            assert!(compiled.hands.contains_key(exec_seat));
        }
    }
}

/// The recorded regression, pinned whole: the two stacks the adoption
/// run (issue #211) was reported against, node/bun and rust/cargo. The
/// implement seat's resolved argv ends in exactly the stack's
/// `--allowedTools` list — the driver prefix, the pinned model, the
/// pinned effort, and the list, in the order the work allowance names
/// them — while verify resolves to the exec script. A grant that
/// reached the adapter map and the agent file but never the argv would
/// fail here, and so would a hire that named a model without the effort
/// decision 0035 ruling 5 pairs with it.
#[test]
fn the_implement_seats_argv_ends_in_the_expected_allowed_tools_list() {
    for (fixture, binary) in [("node-bun", "bun"), ("rust", "cargo")] {
        let (_repo, bundle) = scaffold_from(fixture);
        let compiled = compiles(&bundle);

        let prefix = ["driver", "claude", "--", "--permission-mode", "acceptEdits"];
        let mut implement: Vec<String> = prefix.iter().map(|s| s.to_string()).collect();
        implement.extend([
            "--model".to_string(),
            "claude-opus-5".to_string(),
            "--effort".to_string(),
            "high".to_string(),
            "--allowedTools".to_string(),
            format!("Bash({binary}:*),Bash(git:*),Bash(ls:*),Bash(rg:*),Bash(mkdir:*)"),
        ]);
        assert_eq!(argv(&compiled, "implement"), implement, "{fixture}");

        assert_eq!(
            argv(&compiled, "verify"),
            [
                "driver",
                "exec",
                "--",
                "bash",
                "scripts/verify-seat.sh",
                "{prompt_file}"
            ],
            "{fixture}"
        );
    }
}

/// A repository no row recognizes is granted nothing BY NAME: the
/// adapter's names map stays empty and no agent declares a `tools`
/// restriction (the loader reads an absent `tools` as "no restriction" —
/// the only honest reading an empty map can serve). The README says so in
/// those words rather than letting the silence pass for a choice, and the
/// scaffold still compiles with no `--allowedTools` on any seat.
#[test]
fn an_unrecognized_stack_scaffolds_an_empty_map_and_a_readme_that_says_so() {
    let (_repo, bundle) = scaffold_from("generic");

    assert_eq!(
        names_map(&bundle),
        serde_json::json!({}),
        "generic grants were invented"
    );
    for name in ["intake", "implementer", "reviewer"] {
        let definition = agent(&bundle, name);
        assert!(
            definition.get("tools").is_none(),
            "generic agent {name} declares a tools restriction the empty map cannot express"
        );
        // Everything else an agent needs is still there.
        assert_eq!(definition["charter"], format!("charters/{name}.md"));
        assert_eq!(definition["models"].as_array().unwrap().len(), 2);
    }

    let readme =
        std::fs::read_to_string(bundle.join(DEFAULT_AGENTS_DIR).join("README.md")).unwrap();
    assert!(readme.contains("NO STACK WAS RECOGNIZED"), "{readme}");
    assert!(readme.contains("EMPTY"), "{readme}");
    assert!(
        readme.contains("grants no tool it could not read from a manifest"),
        "{readme}"
    );

    // A scaffold that declares nothing still compiles, with no seat
    // handed a Bash grant it was never given.
    let compiled = compiles(&bundle);
    for seat in ["intake", "implement", "review"] {
        let command = argv(&compiled, seat);
        assert!(
            !command.iter().any(|part| part == "--allowedTools"),
            "{seat}: {command:?}"
        );
        assert!(
            command.contains(&"--model".to_string()),
            "{seat}: {command:?}"
        );
    }
    for seat in ["verify", "ship"] {
        let command = argv(&compiled, seat);
        assert_eq!(&command[..3], ["driver", "exec", "--"]);
        assert!(!command.iter().any(|part| part == "--model"));
    }
}

/// Introspection rewrote prose and grants, not the roster. Every
/// scaffold — each recognized stack and the fallback alike — still
/// compiles, and what it compiles to still carries decision 0021 ruling
/// 1's division: the three judging seats declare `gate`, the two working
/// seats declare `work`. The manifest pins the three model offices under
/// `agents`; verify and ship are pinned separately under `drivers` and
/// `hands` as deterministic exec sites.
#[test]
fn every_scaffolded_recipe_compiles_with_its_gates_still_gates() {
    let fixtures = RECOGNIZED
        .iter()
        .chain(MONOREPOS)
        .map(|(fixture, ..)| *fixture)
        .chain([
            "generic",
            "rust-and-make",
            "rust-package",
            "cargo-workspace",
            "go-workspace",
        ]);
    for fixture in fixtures {
        let (_repo, bundle) = scaffold_from(fixture);
        let (code, stdout, stderr) = brokkr(&["compile", "--bundle", "."], &bundle);
        assert_eq!(code, Some(0), "{fixture}: {stderr}");
        assert!(stdout.contains("\"starter\""), "{fixture}: {stdout}");

        let compiled: Value = serde_json::from_str(&stdout).unwrap();
        let records = compiled["manifest"]["agents"]
            .as_object()
            .unwrap_or_else(|| panic!("{fixture}: no seat resolved through an agent: {stdout}"));
        let seated: Vec<&String> = records.keys().collect();
        assert_eq!(
            seated,
            ["implement", "intake", "review"],
            "{fixture}: {stdout}"
        );
        for (seat, record) in records {
            assert_eq!(record["provider"], "claude", "{fixture}/{seat}: {record}");
        }

        let scaffolded = std::fs::read_to_string(bundle.join("bundle.json")).unwrap();
        assert_eq!(
            scaffolded.matches("\"class\": \"gate\"").count(),
            3,
            "{fixture}"
        );
        assert_eq!(
            scaffolded.matches("\"class\": \"work\"").count(),
            2,
            "{fixture}"
        );
    }
}

/// And the gates are gates in the only way that counts: demote the tier
/// the scaffold declared and the very next compile of a stack-aware
/// starter refuses, naming the class and the driver (decision 0021).
#[test]
fn a_stack_aware_scaffolds_gates_refuse_an_untrusted_driver() {
    let (_repo, bundle) = scaffold_from("rust");
    let adapter = bundle.join(DEFAULT_ADAPTERS_DIR).join("claude.json");
    let declared = std::fs::read_to_string(&adapter).unwrap();
    std::fs::write(
        &adapter,
        declared.replace(
            "\"trust_tier\": \"trusted\"",
            "\"trust_tier\": \"untrusted\"",
        ),
    )
    .unwrap();

    let (code, _, stderr) = brokkr(&["compile", "--bundle", "."], &bundle);
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains("gate class") && stderr.contains("claude"),
        "stderr: {stderr}"
    );
}

/// The other side of the same coin: on a recognized stack, an agent
/// allowance that names a tool the adapter map does not express is a
/// compile refusal (decision 0016) — the two files are one grant, and an
/// operator editing one side learns the coupling from the compiler, not
/// from a run that silently granted nothing.
#[test]
fn an_allowance_the_adapter_cannot_express_refuses_the_scaffolds_compile() {
    let (_repo, bundle) = scaffold_from("node-bun");
    // Remove bun from the adapter map; the work agents still list it.
    let adapter_path = bundle.join(DEFAULT_ADAPTERS_DIR).join("claude.json");
    let mut adapter = read_json(&adapter_path);
    adapter["tool_permissions"]["names"]
        .as_object_mut()
        .unwrap()
        .remove("bun");
    std::fs::write(
        &adapter_path,
        serde_json::to_string_pretty(&adapter).unwrap(),
    )
    .unwrap();

    let (code, _, stderr) = brokkr(&["compile", "--bundle", "."], &bundle);
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains("no tool permission named 'bun'"),
        "stderr: {stderr}"
    );
}

/// The library is workspace data on the trust declaration's own terms:
/// an agent an operator already defines — its grant narrowed, its chain
/// re-ordered — is theirs, and a scaffolder that wrote over it would
/// widen a permission by accident. Refused by name before anything is
/// written.
#[test]
fn init_refuses_to_overwrite_an_operators_agent_definition() {
    let repo = tempfile::tempdir().unwrap();
    let bundle = repo.path().join("bundle");
    std::fs::create_dir_all(bundle.join(DEFAULT_AGENTS_DIR)).unwrap();
    let definition = bundle.join(DEFAULT_AGENTS_DIR).join("implementer.json");
    std::fs::write(&definition, "{\"description\": \"the operator's\"}\n").unwrap();

    let (code, _, stderr) = brokkr(&["init", bundle.to_str().unwrap()], repo.path());
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(stderr.contains("refusing to overwrite"), "stderr: {stderr}");
    assert!(stderr.contains("implementer.json"), "stderr: {stderr}");
    let kept = std::fs::read_to_string(&definition).unwrap();
    assert!(kept.contains("the operator's"), "{kept}");
    assert!(!bundle.join("bundle.json").exists());
    assert!(!bundle.join(DEFAULT_ADAPTERS_DIR).exists());
}

/// The documented primary flow is `brokkr init .` at a project's root,
/// and a project's root has a README.md of its own. The scaffold's notes
/// go under `agents/`, and the project's README keeps every byte —
/// caught at review of the wager's arms, both of which wrote over it.
#[test]
fn init_at_a_projects_root_leaves_its_own_readme_untouched() {
    let repo = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(fixtures().join("rust")).unwrap() {
        let marker = entry.unwrap().path();
        std::fs::copy(&marker, repo.path().join(marker.file_name().unwrap())).unwrap();
    }
    let theirs = "# Their project\n\nThe operator's own words, byte for byte.\n";
    std::fs::write(repo.path().join("README.md"), theirs).unwrap();

    let (code, _, stderr) = brokkr(&["init", "."], repo.path());
    assert_eq!(code, Some(0), "{stderr}");

    assert_eq!(
        std::fs::read_to_string(repo.path().join("README.md")).unwrap(),
        theirs,
        "init wrote over the project's README"
    );
    let notes =
        std::fs::read_to_string(repo.path().join(DEFAULT_AGENTS_DIR).join("README.md")).unwrap();
    assert!(
        notes.contains("cargo"),
        "the scaffold's notes moved under agents/: {notes}"
    );
}
