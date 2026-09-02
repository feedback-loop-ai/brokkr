//! `brokkr init` looks before it scaffolds: the repository it is invoked
//! from is read for the manifests and lockfiles at its root, the two
//! charters that tell a seat to build and to prove name that stack's own
//! commands, and the seats' tool allowances are sized to exactly the
//! binaries those commands run through. What is asserted here is what an
//! operator would read in `agents/charters/` and in the scaffolded
//! `agents/*.json` + `adapters/claude.json` tool data, per stack — and,
//! for a repository carrying no marker init knows, that the charter says
//! so in those words instead of dressing a placeholder as a choice, and
//! that the tool map stayed EMPTY rather than inventing names.
//!
//! Every scaffold is made in a tempdir the fixture's markers are COPIED
//! into. Never in the checked-in fixture itself: `init` writes, and a
//! test that wrote into the committed tree would be a test that edited
//! the repository to pass.

use std::path::{Path, PathBuf};
use std::process::Command;

use brokkr_runtime::bundle::{DEFAULT_ADAPTERS_DIR, DEFAULT_AGENTS_DIR};
use brokkr_runtime::{Bundle, SeatBody};

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
/// read-only subset: the stack's runner plus git, ls and rg, never the
/// write tools). Written out again rather than imported, like every other
/// table here: a grant silently widened or narrowed in `init.rs` fails
/// here.
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
fn expr(name: &str) -> &'static str {
    match name {
        "cargo" => "Bash(cargo:*)",
        "bun" => "Bash(bun:*)",
        "bunx" => "Bash(bunx:*)",
        "pnpm" => "Bash(pnpm:*)",
        "yarn" => "Bash(yarn:*)",
        "npm" => "Bash(npm:*)",
        "npx" => "Bash(npx:*)",
        "uv" => "Bash(uv:*)",
        "python3" => "Bash(python3:*)",
        "pytest" => "Bash(.venv/bin/pytest:*)",
        "go" => "Bash(go:*)",
        "make" => "Bash(make:*)",
        "git" => "Bash(git:*)",
        "ls" => "Bash(ls:*)",
        "rg" => "Bash(rg:*)",
        "mkdir" => "Bash(mkdir:*)",
        other => panic!("no expression written out for '{other}'"),
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
/// `agents/charters/` inside the library root the compiler resolves it
/// against (decision 0016).
fn charter(bundle: &Path, name: &str) -> String {
    std::fs::read_to_string(bundle.join(DEFAULT_AGENTS_DIR).join("charters").join(name)).unwrap()
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// One scaffolded agent definition.
fn agent(bundle: &Path, name: &str) -> serde_json::Value {
    read_json(&bundle.join(DEFAULT_AGENTS_DIR).join(format!("{name}.json")))
}

/// The tool names one scaffolded agent's `tools.allow` lists, in order.
fn allow_of(definition: &serde_json::Value) -> Vec<&str> {
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
fn names_map(bundle: &Path) -> Vec<(String, String)> {
    let adapter = read_json(&bundle.join(DEFAULT_ADAPTERS_DIR).join("claude.json"));
    adapter["tool_permissions"]["names"]
        .as_object()
        .expect("a names map")
        .iter()
        .map(|(name, permission)| {
            (
                name.clone(),
                permission.as_str().expect("a permission").to_string(),
            )
        })
        .collect()
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
        let verifier = charter(&bundle, "verifier.md");

        // The seat that builds is told to build and to test; the seat
        // that proves is told to test and to lint.
        assert_eq!(commands(&implementer), vec![*build, *test], "{fixture}");
        assert_eq!(commands(&verifier), vec![*test, *lint], "{fixture}");

        // Named in the stack's own vocabulary, with the evidence quoted
        // back so the guess can be checked — and not marked generic.
        for text in [&implementer, &verifier] {
            assert!(text.contains(&format!("a {name} project")), "{text}");
            assert!(!text.contains("NO STACK WAS RECOGNIZED"), "{text}");
        }

        // Nobody else's tooling arrived with it.
        let named: Vec<&str> = commands(&implementer)
            .into_iter()
            .chain(commands(&verifier))
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
    let verifier = charter(&bundle, "verifier.md");

    assert_eq!(
        commands(&implementer),
        vec![
            "<this project's build command>",
            "<this project's test command>"
        ]
    );
    assert_eq!(
        commands(&verifier),
        vec![
            "<this project's test command>",
            "<this project's lint command>"
        ]
    );
    for text in [&implementer, &verifier] {
        assert!(text.contains("NO STACK WAS RECOGNIZED"), "{text}");
        assert!(text.contains("GENERIC placeholders"), "{text}");
        for (_, _, build, test, lint) in RECOGNIZED.iter().chain(MONOREPOS) {
            for command in [build, test, lint] {
                assert!(!text.contains(command), "unrecognized, yet names {command}");
            }
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
    let verifier = charter(&bundle, "verifier.md");

    assert_eq!(
        commands(&implementer),
        vec!["cargo build --workspace", "cargo test --workspace"]
    );
    assert_eq!(
        commands(&verifier),
        vec![
            "cargo test --workspace",
            "cargo clippy --workspace --all-targets -- -D warnings"
        ]
    );
    for text in [&implementer, &verifier] {
        for command in ["make build", "make test", "make lint"] {
            assert!(!commands(text).contains(&command), "{text}");
        }
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
    let verifier = charter(&bundle, "verifier.md");

    assert_eq!(
        commands(&implementer),
        vec!["bun install --frozen-lockfile", "bun run test"]
    );
    assert_eq!(
        commands(&verifier),
        vec!["bun run test", "bun run typecheck"]
    );
    for text in [&implementer, &verifier] {
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
    let verifier = charter(&bundle, "verifier.md");

    assert_eq!(commands(&implementer), vec!["uv sync", "uv run pytest"]);
    assert_eq!(
        commands(&verifier),
        vec!["uv run pytest", "uv run ruff check ."]
    );
    for text in [&implementer, &verifier] {
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
        let verifier = charter(&bundle, "verifier.md");

        assert_eq!(commands(&implementer), vec![*build, *test], "{fixture}");
        assert_eq!(commands(&verifier), vec![*test, *lint], "{fixture}");

        for text in [&implementer, &verifier] {
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
        for role in ["implementer.md", "verifier.md"] {
            let text = charter(&bundle, role);
            assert!(text.contains(claim), "{fixture}/{role}: {text}");
        }
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
        for role in ["implementer.md", "verifier.md"] {
            let text = charter(&bundle, role);
            assert!(!text.contains(claim), "{fixture}/{role} claims {claim}");
        }
    }
}

/// The tool-grant half of the table: every recognized stack scaffolds an
/// adapter whose `tool_permissions.names` maps exactly the granted names
/// to their `Bash(...)` expressions, and agents sized by decision 0021
/// ruling 1's classes — the two work agents (intake, implement) carry the
/// full set, the three gate agents (verify, review, ship) carry the
/// read-only subset and never the write tools. The test style is the
/// file's own: the table above is written out again, so a grant changed
/// in `init.rs` fails here.
#[test]
fn each_stack_grants_its_own_tools_by_seat_class() {
    for (fixture, work, gate) in TOOLS {
        let (_repo, bundle) = scaffold_from(fixture);

        // The adapter map is the union of every allowance: the work set,
        // each name mapped to the Bash(...) expression the CLI needs.
        // (An object's keys print sorted; compare the two as maps.)
        let mut mapped = names_map(&bundle);
        mapped.sort();
        let mut expected: Vec<(String, String)> = work
            .iter()
            .map(|name| (name.to_string(), expr(name).to_string()))
            .collect();
        expected.sort();
        assert_eq!(mapped, expected, "{fixture}");

        // The two work-class agents carry the full set...
        for name in ["intake", "implementer"] {
            assert_eq!(
                allow_of(&agent(&bundle, name)),
                work.to_vec(),
                "{fixture}/{name}"
            );
        }
        // ... and the three gate-class agents carry the read-only subset:
        // git, ls, rg and the stack's runner, never the write tools.
        for name in ["verifier", "reviewer", "shipper"] {
            assert_eq!(
                allow_of(&agent(&bundle, name)),
                gate.to_vec(),
                "{fixture}/{name}"
            );
        }

        // Every seat's allowance is exactly expressible: the scaffold
        // compiles under its own adapters/ and agents/ (init proved the
        // same when it printed its digest).
        compiles(&bundle);
    }
}

/// A repository no row recognizes is granted nothing BY NAME: the
/// adapter's names map stays empty and no agent declares a `tools.allow`
/// (an empty map can express no name, and the loader reads an absent
/// `tools` as "no restriction" — the only honest reading). The README
/// says so in those words rather than letting the silence pass for a
/// choice, and the scaffold still compiles.
#[test]
fn an_unrecognized_stack_scaffolds_an_empty_map_and_a_readme_that_says_so() {
    let (_repo, bundle) = scaffold_from("generic");

    assert_eq!(
        names_map(&bundle),
        Vec::new(),
        "generic grants were invented"
    );
    for name in ["intake", "implementer", "verifier", "reviewer", "shipper"] {
        let definition = agent(&bundle, name);
        assert!(
            definition.get("tools").is_none(),
            "generic agent {name} declares a tools restriction the empty map cannot express"
        );
    }

    let readme = std::fs::read_to_string(bundle.join("README.md")).unwrap();
    assert!(readme.contains("NO STACK WAS RECOGNIZED"), "{readme}");
    assert!(
        readme.contains("tool map was scaffolded EMPTY") || readme.contains("scaffolded EMPTY"),
        "{readme}"
    );
    assert!(readme.contains("does not invent tool names"), "{readme}");

    // A scaffold that declares nothing still compiles.
    compiles(&bundle);
}

/// The README of a recognized stack names the stack, the evidence, and
/// the two class-sized grants — so the sentence an operator reads agrees
/// with the data the seats actually run under.
#[test]
fn a_recognized_stack_readme_names_the_stack_and_its_grants() {
    for (fixture, work, gate) in TOOLS {
        let (_repo, bundle) = scaffold_from(fixture);
        let readme = std::fs::read_to_string(bundle.join("README.md")).unwrap();
        assert!(
            !readme.contains("NO STACK WAS RECOGNIZED"),
            "{fixture}: the README says no stack was recognized: {readme}"
        );
        for name in *work {
            assert!(
                readme.contains(expr(name)),
                "{fixture}: README does not name the work grant {name}: {readme}"
            );
        }
        for name in *gate {
            assert!(
                readme.contains(expr(name)),
                "{fixture}: README does not name the gate grant {name}: {readme}"
            );
        }
        assert!(readme.contains("read-only subset"), "{fixture}: {readme}");
        assert!(readme.contains("work-class seats"), "{fixture}");
    }
}

/// The implement seat's resolved argv — the command the engine would
/// spawn for the seat that builds — carries the expected `--allowedTools`
/// list, composed from the implementer agent's `tools.allow` through the
/// adapter's names map (decision 0016). node/bun and rust/cargo are the
/// two the adoption run asked for; every row of the grant table gets the
/// same proof.
#[test]
fn the_implement_seats_resolved_argv_carries_the_expected_allowed_tools() {
    for (fixture, work, _gate) in TOOLS {
        let (_repo, bundle) = scaffold_from(fixture);
        let compiled = compiles(&bundle);
        let SeatBody::Single { command, .. } = &compiled.seats["implement"].body else {
            panic!("implement is a single-agent seat")
        };
        let at = command
            .iter()
            .position(|part| part == "--allowedTools")
            .unwrap_or_else(|| panic!("{fixture}: no --allowedTools in {command:?}"));
        assert_eq!(&command[at], "--allowedTools");
        let expected: Vec<String> = work.iter().map(|name| expr(name).to_string()).collect();
        assert_eq!(
            &command[at + 1],
            &expected.join(","),
            "{fixture}: {command:?}"
        );
    }
}

/// Every scaffolded recipe — each recognized stack and the fallback
/// alike — still compiles, and what it compiles to still carries decision
/// 0021 ruling 1's division: the three judging seats declare `gate`, the
/// two working seats declare `work`, and every seat's resolution in the
/// compiled manifest pins the adapter that answered for it. The manifest
/// no longer shows a `drivers` map — since the seats reference agents,
/// each site is recorded under `agents` with the adapter digest that
/// authorised it (decision 0016) — so "the gates are still gates" is
/// asserted the way the machine enforces it: the scaffold compiles only
/// while its claude adapter holds the trusted tier, and the demotion
/// tests below prove the refusal.
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

        // Every seat resolved as an agent against the scaffold's own
        // library, each pinning the adapter that expressed its tools.
        let compiled: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let resolved = compiled["manifest"]["agents"]
            .as_object()
            .unwrap_or_else(|| panic!("{fixture}: no seat resolved as an agent: {stdout}"));
        assert_eq!(resolved.len(), 5, "{fixture}: {stdout}");
        for record in resolved.values() {
            assert_eq!(record["provider"], "claude", "{fixture}: {stdout}");
            assert!(
                record["adapter_digest"].is_string(),
                "{fixture}: no adapter pin: {stdout}"
            );
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
    let adapter = bundle.join("adapters/claude.json");
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
    let adapter_path = bundle.join("adapters/claude.json");
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
