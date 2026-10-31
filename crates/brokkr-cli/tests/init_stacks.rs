//! `brokkr init` looks before it scaffolds: the repository it writes its
//! folder into is read for the manifests and lockfiles at its root, and the two
//! charter and verifier script that tell seats to build and prove name that
//! stack's own commands. What is asserted here is what an operator would
//! read in the generated files — and, for a repository carrying no marker
//! init knows, that the charter says so in those words instead of dressing
//! a placeholder as a choice.
//!
//! The same table decides what the seats are pre-approved to run: the
//! binary each command invokes is written into the scaffold's adapter as a
//! tool permission and granted to the scaffolded agents — the whole set to
//! the work seats, the smaller set to the review gate — and what is asserted
//! is the resolved argv the compiler composes for the implement seat,
//! because a grant that never reached `--allowedTools` is no grant.
//! Pre-approval removes no tool.
//!
//! Every scaffold is made in a tempdir the fixture's markers are COPIED
//! into. Never in the checked-in fixture itself: `init` writes, and a
//! test that wrote into the committed tree would be a test that edited
//! the repository to pass.

use std::path::{Path, PathBuf};
use std::process::Command;

use brokkr_runtime::bundle::{DEFAULT_ADAPTERS_DIR, DEFAULT_AGENTS_DIR};
use brokkr_runtime::{Bundle, Invocation, SeatBody};
use serde_json::Value;

use crate::brokkr_folder::{repository, Repo};

/// Fixture, the name the charters call the stack, then build, test, lint.
/// The same shape as the table in `init.rs`, written out again rather
/// than imported, so a command silently changed there fails here.
const RECOGNIZED: &[(&str, &str, &str, &str, &str)] = &[
    (
        "rust",
        "rust",
        "cargo build --workspace",
        "cargo test --workspace --locked",
        "cargo clippy --workspace --all-targets --locked -- -D warnings",
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
/// rg and mkdir), then the GATE allowance (the review agent's — the
/// smaller set: the test runner's tools plus git, ls and rg, never
/// `mkdir`). Written out again rather than imported, like every
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

/// The base grant every scaffold carries, a stack recognized or not: the
/// work allowance, then the gate's (#717).
const BASE: (&[&str], &[&str]) = (&["git", "ls", "rg", "mkdir"], &["git", "ls", "rg"]);

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

/// Run `brokkr`. An `init` runs with a `claude` first on PATH: it
/// scaffolds for the first of claude, codex or dsh it finds, and these
/// proofs are about the claude scaffold whatever agent CLIs this host
/// carries. Every other verb sees the host's own PATH.
pub(crate) fn brokkr(args: &[&str], cwd: &Path) -> (Option<i32>, String, String) {
    brokkr_with(args, cwd, |_| {})
}

/// [`brokkr`], the command carrying what `configure` sets last.
pub(crate) fn brokkr_with(
    args: &[&str],
    cwd: &Path,
    configure: impl Fn(&mut Command),
) -> (Option<i32>, String, String) {
    let stubs = tempfile::tempdir().unwrap();
    std::fs::write(stubs.path().join("claude"), "").unwrap();
    let mut path = std::env::var_os("PATH").unwrap();
    if args.first() == Some(&"init") {
        path = std::env::join_paths(
            std::iter::once(stubs.path().to_path_buf()).chain(std::env::split_paths(&path)),
        )
        .unwrap();
    }
    // A HOME of its own, beside the stubs, unless `configure` gives it
    // another (decision 0065 slice two, U8a2a).
    let home = crate::engine::toolchain_home_in(stubs.path());
    let mut command = Command::new(env!("CARGO_BIN_EXE_brokkr"));
    command.args(args).env("PATH", path).env("HOME", home);
    command.current_dir(cwd);
    configure(&mut command);
    let out = command.output().unwrap();
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    (out.status.code(), text(&out.stdout), text(&out.stderr))
}

/// [`brokkr_with`] with the child's `HOME` and `XDG_CONFIG_HOME` under
/// `aside`, a directory beside the repository, so no host's own
/// configuration is read; an `init` runs with the agent CLI `cli` alone
/// on PATH, so it scaffolds for that CLI whatever this host carries.
pub(crate) fn brokkr_for(
    cli: &str,
    args: &[&str],
    cwd: &Path,
    aside: &Path,
) -> (Option<i32>, String, String) {
    let bins = aside.join("bins");
    for dir in [&bins, &aside.join("home"), &aside.join("config")] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(bins.join(cli), "").unwrap();
    crate::brokkr_folder::plant_git(&bins);
    brokkr_with(args, cwd, |command| {
        command
            .env("HOME", aside.join("home"))
            .env("XDG_CONFIG_HOME", aside.join("config"));
        if args.first() == Some(&"init") {
            command.env("PATH", &bins);
        }
    })
}

/// A fresh repository holding one fixture's marker files, not yet
/// scaffolded.
pub(crate) fn repository_of(fixture: &str) -> tempfile::TempDir {
    let repo = repository();
    for entry in std::fs::read_dir(fixtures().join(fixture)).unwrap() {
        let marker = entry.unwrap().path();
        std::fs::copy(&marker, repo.path().join(marker.file_name().unwrap())).unwrap();
    }
    repo
}

/// Copy one fixture's marker files into a fresh repository and scaffold
/// it — which is how `init` is used: the folder lands at the root of the
/// repository it describes (decision 0083). The `TempDir` comes back with
/// the folder's path so the tree outlives the call.
fn scaffold_from(fixture: &str) -> (tempfile::TempDir, PathBuf) {
    let repo = repository_of(fixture);
    let (code, _, stderr) = brokkr(&["init", "."], repo.path());
    assert_eq!(code, Some(0), "{fixture}: {stderr}");
    let bundle = repo.path().join(".brokkr");
    (repo, bundle)
}

/// `init <dir>` reads the repository it writes, never the directory it
/// was invoked from (the operator's ruling 2 of 2026-10-13): run from a
/// go module, it scaffolds a cargo repository as the cargo one it is.
#[test]
fn init_reads_the_repository_it_writes() {
    let (invoked, written) = (tempfile::tempdir().unwrap(), repository());
    for (fixture, into) in [("go", invoked.path()), ("rust", written.path())] {
        for entry in std::fs::read_dir(fixtures().join(fixture)).unwrap() {
            let marker = entry.unwrap().path();
            std::fs::copy(&marker, into.join(marker.file_name().unwrap())).unwrap();
        }
    }
    let (code, _, stderr) = brokkr(&["init", written.path().to_str().unwrap()], invoked.path());
    assert_eq!(code, Some(0), "{stderr}");
    let implementer = charter(&written.path().join(".brokkr"), "implementer.md");
    let named = commands(&implementer);
    assert_eq!(
        named,
        ["cargo build --workspace", "cargo test --workspace --locked"],
        "{implementer}"
    );
    assert!(!invoked.path().join(".brokkr").exists());
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

/// Proposed decision 0056 ruling 5: a scaffolded workspace declares the
/// same assessment shape every shipped adapter does, and declares it
/// honestly — unmeasured, with an explicitly unknown identity and its
/// reason. It loads, it compiles, and it enables nothing.
#[test]
fn the_scaffolded_adapter_declares_an_honest_unmeasured_resume_assessment() {
    let (_dir, bundle) = scaffold_from("rust");
    let adapter = read_json(&bundle.join(DEFAULT_ADAPTERS_DIR).join("claude.json"));
    let shape = &adapter["resume"]["boxed-workspace"];
    assert_eq!(shape["status"], "unmeasured");
    assert!(
        shape["identity"]["unknown"].is_string(),
        "the identity is explicitly unknown, not invented: {shape}"
    );
    assert!(
        shape["identity"].get("version").is_none(),
        "a scaffold has measured no version to claim: {shape}"
    );
    assert!(
        shape["reason"].as_str().is_some_and(|r| !r.is_empty()),
        "and it says what has not been measured: {shape}"
    );
    // It loads, and the seat it serves compiles and would invoke cold.
    compiles(&bundle);
}

/// Decision 0065 ruling 4: a scaffold carries the SAME native assessment
/// the shipped Claude adapter does — word for word, evidence limits
/// included — so a stranger's first workspace cannot drift into a weaker
/// declaration. And it grants nothing: every model seat it compiles holds
/// no capability and is composed with both native tools denied by name.
#[test]
fn the_scaffolded_adapter_carries_the_shipped_native_assessment_and_grants_nothing() {
    let (_dir, bundle) = scaffold_from("rust");
    let scaffolded = read_json(&bundle.join(DEFAULT_ADAPTERS_DIR).join("claude.json"));
    let shipped =
        read_json(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/claude.json"));
    assert_eq!(
        scaffolded["native_capabilities"],
        shipped["native_capabilities"]
    );

    let compiled = compiles(&bundle);
    assert_eq!(
        compiled.manifest["capabilities"]["grants"],
        serde_json::json!({})
    );
    let mut model_sites = 0;
    for (label, facts) in &compiled.sites {
        let site = facts.capabilities.as_ref().unwrap();
        for outcome in &site.outcomes {
            assert!(outcome.held.is_empty(), "{label} holds nothing");
            if outcome.provider != "claude" {
                continue;
            }
            model_sites += 1;
            assert_eq!(
                outcome.controls()["selection"]["deny"],
                serde_json::json!(["WebFetch", "WebSearch"]),
                "{label}"
            );
            assert_eq!(
                outcome.controls()["selection"]["include"],
                serde_json::json!([])
            );
        }
    }
    assert!(
        model_sites > 0,
        "the scaffold seats at least one claude site"
    );
}

/// The runner names a scaffold may map that the shipped Claude adapter
/// does not: the package managers and toolchains no shipped recipe runs.
/// Every other name a scaffold writes is shipped vocabulary, and must mean
/// byte for byte what it means there.
const SCAFFOLD_ONLY: &[(&str, &[&str])] = &[
    ("rust", &[]),
    ("node-bun", &["bun"]),
    ("node-pnpm", &["pnpm"]),
    ("node-yarn", &["yarn"]),
    ("node-npm", &[]),
    ("python-uv", &["uv"]),
    ("python", &["pytest", "python3"]),
    ("go", &["go"]),
    ("make", &["make"]),
    ("turbo-pnpm", &["pnpm"]),
    ("turbo-bun", &["bunx"]),
    ("turbo-plain", &[]),
    ("nx-yarn", &["yarn"]),
    ("generic", &[]),
];

/// Decision 0065 rebuild unit 5 (tasks 5.1 and 5.2): every stack's
/// generated typed restriction is local command data and nothing more.
/// Its names are the shipped vocabulary where the shipped Claude adapter
/// has them — `npm` and `npx` included since the migration names landed —
/// with the same prefix, and only the rows' own runners outside it. It
/// adds no grant: the manifest grants nothing, no outcome holds a
/// capability, and every Claude seat is composed with both native tools
/// denied by name. Every row is computed before any is judged.
#[test]
#[expect(
    clippy::excessive_nesting,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn every_stacks_typed_restrictions_are_shipped_vocabulary_and_add_no_grant() {
    let shipped =
        read_json(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/claude.json"));
    let shipped = shipped["tool_permissions"]["names"].as_object().unwrap();
    let mut rows = Vec::new();
    let mut expected = Vec::new();
    for (fixture, only) in SCAFFOLD_ONLY {
        let (_dir, bundle) = scaffold_from(fixture);
        let names = names_map(&bundle);
        let mut outside = Vec::new();
        let mut differing = Vec::new();
        for (name, pattern) in names.as_object().unwrap() {
            match shipped.get(name) {
                None => outside.push(name.clone()),
                Some(theirs) if theirs != pattern => {
                    differing.push((name.clone(), pattern.clone(), theirs.clone()))
                }
                Some(_) => {}
            }
        }
        let compiled = compiles(&bundle);
        let mut held = Vec::new();
        let mut claude = Vec::new();
        for (label, facts) in &compiled.sites {
            for outcome in &facts.capabilities.as_ref().unwrap().outcomes {
                if !outcome.held.is_empty() {
                    held.push(label.clone());
                }
                if outcome.provider == "claude" {
                    let selection = &outcome.controls()["selection"];
                    let pair = (selection["include"].clone(), selection["deny"].clone());
                    if !claude.contains(&pair) {
                        claude.push(pair);
                    }
                }
            }
        }
        rows.push((
            *fixture,
            outside,
            differing,
            compiled.manifest["capabilities"]["grants"].clone(),
            held,
            claude,
        ));
        expected.push((
            *fixture,
            only.iter().map(|name| name.to_string()).collect::<Vec<_>>(),
            Vec::new(),
            serde_json::json!({}),
            Vec::new(),
            vec![(
                serde_json::json!([]),
                serde_json::json!(["WebFetch", "WebSearch"]),
            )],
        ));
    }
    assert_eq!(rows, expected);
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
        SeatBody::Invocation(Invocation::Single { command, .. }) => command[1..].to_vec(),
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
        verifier.contains("no stack was recognized; fill in .brokkr/scripts/verify-seat.sh"),
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
        vec!["cargo build --workspace", "cargo test --workspace --locked"]
    );
    assert_eq!(
        verifier_commands(&verifier),
        vec![
            "cargo test --workspace --locked",
            "cargo clippy --workspace --all-targets --locked -- -D warnings"
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
        vec!["cargo build --workspace", "cargo test --workspace --locked"]
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
/// review carries the smaller set. Verify and ship are exec scripts.
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

/// A cargo stack's verifier runs `--locked`, so its implementer charter
/// says, in one sentence, to commit `Cargo.lock` with the change (#717:
/// a folder made by `cargo init` parked SHIP-DIRTY on `?? Cargo.lock`).
/// No other stack's charter says it.
#[test]
fn a_cargo_charter_says_to_commit_its_lockfile() {
    let sentence = "Commit `Cargo.lock` with the change whenever the change creates or changes it.";
    for (fixture, said) in [("rust", 1), ("node-npm", 0), ("generic", 0)] {
        let (_repo, bundle) = scaffold_from(fixture);
        let implementer = charter(&bundle, "implementer.md");
        let lines = implementer.lines().filter(|line| *line == sentence);
        assert_eq!(lines.count(), said, "{fixture}: {implementer}");
        assert_eq!(implementer.matches("Cargo.lock").count(), said, "{fixture}");
    }
}

/// The variables naming volta's, bun's and pnpm's homes, which `init`
/// reads.
pub(crate) const TOOL_HOMES: [&str; 4] =
    ["VOLTA_HOME", "BUN_INSTALL", "PNPM_HOME", "XDG_DATA_HOME"];

/// `init` run with `PATH` and `HOME` under `aside`: a claude stub in
/// `aside/bins`, then `extra` on `PATH`, and of the tools' home variables
/// only `vars`, so no host install is read.
fn init_on(repo: &Path, aside: &Path, extra: &[PathBuf], vars: &[(&str, &Path)]) -> String {
    let bins = aside.join("bins");
    std::fs::create_dir_all(&bins).unwrap();
    std::fs::write(bins.join("claude"), "").unwrap();
    crate::brokkr_folder::plant_git(&bins);
    let path = std::env::join_paths(std::iter::once(bins).chain(extra.iter().cloned())).unwrap();
    let (code, _, stderr) = brokkr_with(&["init", "."], repo, |command| {
        command.env("PATH", &path).env("HOME", aside.join("home"));
        for name in TOOL_HOMES {
            command.env_remove(name);
        }
        command.envs(vars.iter().copied());
    });
    assert_eq!(code, Some(0), "{stderr}");
    stderr
}

/// Gap 3 of #717, composed: an npm folder on a host whose npm is a volta
/// shim under `$HOME` compiles to a verify box holding the volta root
/// read-only, and its verifier puts the shim's directory at the head of
/// `PATH` and names `VOLTA_HOME`, which the box's private HOME would
/// otherwise hide. The bind is recorded in `bundle.json` at `init`.
#[test]
fn a_volta_npm_folder_binds_the_volta_root_on_verify() {
    let repo = repository_of("node-npm");
    let aside = tempfile::tempdir().unwrap();
    let home = aside.path().canonicalize().unwrap().join("home");
    let volta = home.join(".volta");
    std::fs::create_dir_all(volta.join("bin")).unwrap();
    std::fs::create_dir_all(volta.join("tools/image/node/20.11.0/bin")).unwrap();
    std::fs::write(volta.join("bin/volta-shim"), "").unwrap();
    let runs = std::os::unix::fs::PermissionsExt::from_mode(0o755);
    std::fs::set_permissions(volta.join("bin/volta-shim"), runs).unwrap();
    for shim in ["npm", "node"] {
        std::os::unix::fs::symlink("volta-shim", volta.join("bin").join(shim)).unwrap();
    }
    let stderr = init_on(repo.path(), aside.path(), &[volta.join("bin")], &[]);
    assert!(!stderr.contains("was not found on PATH when"), "{stderr}");
    let bundle = repo.path().join(".brokkr");
    let source = read_json(&bundle.join("bundle.json"));
    let binds = &source["seats"]["verify"]["hands"]["binds"];
    assert_eq!(
        binds,
        &serde_json::json!([{"path": "~/.volta", "mode": "ro"}])
    );
    let compiled = compiles(&bundle);
    let held = &compiled.hands["verify"].binds;
    let held: Vec<_> = held
        .iter()
        .map(|bind| (bind.path.as_str(), bind.mode))
        .collect();
    assert_eq!(held, [("~/.volta", brokkr_protocol::hands::BindMode::Ro)]);
    let script = verifier(&bundle);
    let exports: Vec<&str> = script
        .lines()
        .filter(|l| l.starts_with("export "))
        .collect();
    let (bin, root) = (volta.join("bin"), volta.display().to_string());
    assert_eq!(
        exports,
        [
            format!("export PATH='{}':\"$PATH\"", bin.display()),
            format!("export VOLTA_HOME='{root}'"),
        ]
    );
}

/// No npm on `PATH` when `init` ran: init says so in one line, the
/// verify box binds nothing for it, and the verifier, run, fails closed
/// with that same line before any command, as the no-stack verifier does.
#[test]
fn an_npm_folder_with_no_npm_fails_verify_closed() {
    let repo = repository_of("node-npm");
    let aside = tempfile::tempdir().unwrap();
    let stderr = init_on(repo.path(), aside.path(), &[], &[]);
    let said = "npm was not found on PATH when `brokkr init` ran; install it, or fill in \
                .brokkr/scripts/verify-seat.sh";
    fails_verify_closed(repo.path(), aside.path(), &stderr, said);
}

/// An npm installed in `~/.aws/bin`, linking into `~/.aws/lib`, beside
/// `~/.aws/credentials`, binds nothing (#717's security hold): init names
/// the credentials in one line, and the scaffold's verifier fails closed
/// with it.
#[test]
fn an_npm_beside_credentials_binds_nothing_and_fails_verify_closed() {
    let repo = repository_of("node-npm");
    let aside = tempfile::tempdir().unwrap();
    let aws = aside.path().canonicalize().unwrap().join("home/.aws");
    let cli = "lib/node_modules/npm/bin/npm-cli.js";
    std::fs::create_dir_all(aws.join(cli).parent().unwrap()).unwrap();
    std::fs::create_dir_all(aws.join("bin")).unwrap();
    std::fs::write(aws.join("credentials"), "").unwrap();
    for file in ["bin/node", cli] {
        std::fs::write(aws.join(file), "").unwrap();
        let runs = std::os::unix::fs::PermissionsExt::from_mode(0o755);
        std::fs::set_permissions(aws.join(file), runs).unwrap();
    }
    std::os::unix::fs::symlink(format!("../{cli}"), aws.join("bin/npm")).unwrap();
    let stderr = init_on(repo.path(), aside.path(), &[aws.join("bin")], &[]);
    let said = format!(
        "npm is installed under {}, which holds 'credentials', no part of a node prefix, \
         and which `brokkr init` will not bind into verify's hands; install it under a \
         directory of its own, or fill in .brokkr/scripts/verify-seat.sh",
        aws.display()
    );
    fails_verify_closed(repo.path(), aside.path(), &stderr, &said);
}

/// The chief's reproduction V of #717's third hold, through `init`: an
/// npm whose `bin` links to a volta shim under the `tools` beside it,
/// which holds `aws.creds`. Named by no variable it is no volta home but a
/// node prefix holding `tools`, refused in one line, and verify fails
/// closed with it. With `VOLTA_HOME` naming it, volta's own word, it is
/// bound whole as volta's root, `VOLTA_HOME` exported.
#[test]
fn a_volta_root_is_bound_only_where_volta_home_names_it() {
    let aside = tempfile::tempdir().unwrap();
    let v = aside.path().canonicalize().unwrap().join("opt/v");
    let shim = v.join("tools/volta/bin/volta-shim");
    std::fs::create_dir_all(shim.parent().unwrap()).unwrap();
    std::fs::create_dir_all(v.join("bin")).unwrap();
    std::fs::write(&shim, "").unwrap();
    std::fs::write(v.join("tools/aws.creds"), "").unwrap();
    let runs = std::os::unix::fs::PermissionsExt::from_mode(0o755);
    std::fs::set_permissions(&shim, runs).unwrap();
    for name in ["npm", "node"] {
        std::os::unix::fs::symlink("../tools/volta/bin/volta-shim", v.join("bin").join(name))
            .unwrap();
    }
    let repo = repository_of("node-npm");
    let stderr = init_on(repo.path(), aside.path(), &[v.join("bin")], &[]);
    let said = format!(
        "npm is installed under {}, which holds 'tools', no part of a node prefix, \
         and which `brokkr init` will not bind into verify's hands; install it under a \
         directory of its own, or fill in .brokkr/scripts/verify-seat.sh",
        v.display()
    );
    fails_verify_closed(repo.path(), aside.path(), &stderr, &said);
    let repo = repository_of("node-npm");
    let named = [("VOLTA_HOME", v.as_path())];
    let stderr = init_on(repo.path(), aside.path(), &[v.join("bin")], &named);
    assert!(!stderr.contains("will not bind"), "{stderr}");
    let bundle = repo.path().join(".brokkr");
    let source = read_json(&bundle.join("bundle.json"));
    assert_eq!(
        source["seats"]["verify"]["hands"]["binds"],
        serde_json::json!([{"path": v.display().to_string(), "mode": "ro"}])
    );
    let script = verifier(&bundle);
    let volta = format!("export VOLTA_HOME='{}'", v.display());
    assert_eq!(script.lines().filter(|line| *line == volta).count(), 1);
}

/// init said `said` in one line, the verify box binds nothing for the
/// runner, and the verifier, run, fails closed with that same line before
/// any command, as the no-stack verifier does.
fn fails_verify_closed(repo: &Path, aside: &Path, stderr: &str, said: &str) {
    assert_eq!(
        stderr.lines().filter(|line| *line == said).count(),
        1,
        "{stderr}"
    );
    let bundle = repo.join(".brokkr");
    let source = read_json(&bundle.join("bundle.json"));
    assert_eq!(
        source["seats"]["verify"]["hands"]["binds"],
        serde_json::json!([])
    );
    let result = aside.join("result.json");
    let prompt = aside.join("prompt.md");
    std::fs::write(&prompt, format!("{}\n", result.display())).unwrap();
    let ran = Command::new("bash")
        .arg(bundle.join("scripts/verify-seat.sh"))
        .arg(&prompt)
        .env("HOME", aside.join("home"))
        .current_dir(repo)
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let written = read_json(&result);
    assert_eq!(
        written,
        serde_json::json!({"result": "fail", "notes": said})
    );
}

/// The implement seat's resolved argv — the command the engine would
/// spawn for the seat that builds — carries the expected `--allowedTools`
/// list, composed from the implementer agent's `tools.allow` through the
/// adapter's names map (decision 0016). node/bun and rust/cargo are the
/// two the adoption run asked for; the review seat's argv carries the
/// smaller set, and never `mkdir`.
#[test]
fn the_implement_and_verify_seats_argv_carry_the_class_allowed_tools() {
    for (fixture, work, gate) in TOOLS {
        let (_repo, bundle) = scaffold_from(fixture);
        let compiled = compiles(&bundle);

        let SeatBody::Invocation(Invocation::Single { command, .. }) =
            &compiled.seats["implement"].body
        else {
            panic!("implement is a single-agent seat")
        };
        let allowed: Vec<String> = work.iter().map(|name| expr(name)).collect();
        assert!(
            command
                .windows(2)
                .any(|pair| pair[0] == "--allowedTools" && pair[1] == allowed.join(",")),
            "{fixture}: implement argv {command:?} lacks {allowed:?}"
        );

        let SeatBody::Invocation(Invocation::Single { command, .. }) =
            &compiled.seats["review"].body
        else {
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
            let SeatBody::Invocation(Invocation::Single { command, .. }) =
                &compiled.seats[gate_seat].body
            else {
                panic!("{gate_seat} is a single seat")
            };
            assert!(
                !command.iter().any(|part| part.contains("mkdir")),
                "{fixture}/{gate_seat}: a gate carries the write tool: {command:?}"
            );
        }
        for exec_seat in ["verify", "ship"] {
            let SeatBody::Invocation(Invocation::Single {
                command,
                candidates,
                ..
            }) = &compiled.seats[exec_seat].body
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
        let source: Value =
            serde_json::from_slice(&std::fs::read(bundle.join("bundle.json")).unwrap()).unwrap();
        assert_eq!(
            source["seats"]["verify"]["driver"]["command"][5], "./scripts/verify-seat.sh",
            "{fixture}"
        );
        assert_eq!(
            source["seats"]["ship"]["driver"]["command"][5], "./scripts/ship-seat.sh",
            "{fixture}"
        );
        let compiled = compiles(&bundle);

        let prefix = ["driver", "claude", "--", "--permission-mode", "acceptEdits"];
        let mut implement: Vec<String> = prefix.iter().map(|s| s.to_string()).collect();
        implement.extend([
            "--model".to_string(),
            "claude-opus-5-5".to_string(),
            "--effort".to_string(),
            "high".to_string(),
            "--allowedTools".to_string(),
            format!("Bash({binary}:*),Bash(git:*),Bash(ls:*),Bash(rg:*),Bash(mkdir:*)"),
        ]);
        assert_eq!(argv(&compiled, "implement"), implement, "{fixture}");

        let verify = argv(&compiled, "verify");
        assert_eq!(&verify[..4], ["driver", "exec", "--", "bash"], "{fixture}");
        // Compare canonical paths because the compiler canonicalizes bundle roots:
        // macOS temp dirs may cross `/private`, and Windows may use verbatim paths.
        let expected_script = std::fs::canonicalize(bundle.join("scripts/verify-seat.sh"))
            .expect("the scaffolded verifier script canonicalizes");
        assert_eq!(Path::new(&verify[4]), expected_script, "{fixture}");
        assert_eq!(verify[5], "{prompt_file}", "{fixture}");
    }
}

/// A repository no row recognizes earns NO runner, and the base grant
/// every seat's charter names (#717: a no-marker implement seat was
/// refused `git add`): the adapter's map carries `git`, `ls`, `rg` and
/// `mkdir`, the work agents allow all four and the reviewer the read
/// trio. The README says it granted no runner, in those words, and the
/// scaffold compiles with the base grant on each model seat.
#[test]
fn an_unrecognized_stack_scaffolds_the_base_grant_and_a_readme_that_says_so() {
    let (_repo, bundle) = scaffold_from("generic");

    let base = BASE
        .0
        .iter()
        .map(|name| (name.to_string(), expr(name).into()));
    assert_eq!(names_map(&bundle), Value::Object(base.collect()));
    for (name, allow) in [
        ("intake", BASE.0),
        ("implementer", BASE.0),
        ("reviewer", BASE.1),
    ] {
        let definition = agent(&bundle, name);
        let tools = serde_json::json!({"allow": allow, "mcp": []});
        assert_eq!(definition["tools"], tools, "{name}");
        // Everything else an agent needs is still there.
        assert_eq!(definition["charter"], format!("charters/{name}.md"));
        assert_eq!(definition["models"].as_array().unwrap().len(), 2);
    }

    let readme =
        std::fs::read_to_string(bundle.join(DEFAULT_AGENTS_DIR).join("README.md")).unwrap();
    assert!(readme.contains("NO STACK WAS RECOGNIZED"), "{readme}");
    assert!(
        readme.contains("the base set\nevery seat's charter names, and NO runner:"),
        "{readme}"
    );
    assert!(
        readme.contains("grants no runner it could not read from a manifest"),
        "{readme}"
    );

    let compiled = compiles(&bundle);
    let work = "Bash(git:*),Bash(ls:*),Bash(rg:*),Bash(mkdir:*)";
    let gate = "Bash(git:*),Bash(ls:*),Bash(rg:*)";
    for (seat, granted) in [("intake", work), ("implement", work), ("review", gate)] {
        let command = argv(&compiled, seat);
        let at = command.iter().position(|part| part == "--allowedTools");
        let listed = at.map(|at| command[at + 1].as_str());
        assert_eq!(listed, Some(granted), "{seat}: {command:?}");
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
/// `hands` as deterministic exec sites. Whatever tools a stack maps, its
/// claude and exec declarations carry their shipped MCP facts (SI2), and
/// the compile still passes on them before strict admission activates,
/// keeping each site's SI2 record as the Claude roster's (U1g1).
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
        for provider in ["claude", "exec"] {
            crate::init_doctor::assert_shipped_mcp_facts(&bundle, provider);
        }
        let (code, stdout, stderr) = brokkr(&["compile", "--bundle", "."], &bundle);
        assert_eq!(code, Some(0), "{fixture}: {stderr}");
        assert!(stdout.contains("\"starter\""), "{fixture}: {stdout}");
        assert_eq!(
            crate::init_doctor::strict_records(&compiles(&bundle)),
            crate::init_doctor::expected_records(&crate::init_doctor::CLAUDE_RECORDS),
            "{fixture}"
        );

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
    let repo = Repo::new();
    let definition = repo.write(
        ".brokkr/agents/implementer.json",
        "{\"description\": \"the operator's\"}\n",
    );

    let ran = repo.brokkr("", &["init", "."]);
    let refused = format!(
        "error: {}/.brokkr already holds agents (decision 0083); init writes a folder only where \
         none stands, or beside moved state (.gitignore and state/ alone): move or remove them\n",
        repo.root.display()
    );
    assert_eq!((ran.code, ran.stderr), (Some(1), refused));
    let kept = std::fs::read_to_string(&definition).unwrap();
    assert!(kept.contains("the operator's"), "{kept}");
    assert!(!repo.path(".brokkr/bundle.json").exists());
    assert!(!repo.path(".brokkr").join(DEFAULT_ADAPTERS_DIR).exists());
}

/// The documented primary flow is `brokkr init .` at a project's root,
/// and a project's root has a README.md of its own. The scaffold's notes
/// go under `agents/`, and the project's README keeps every byte —
/// caught at review of the wager's arms, both of which wrote over it.
#[test]
fn init_at_a_projects_root_leaves_its_own_readme_untouched() {
    let repo = repository();
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
    let notes = repo.path().join(".brokkr").join(DEFAULT_AGENTS_DIR);
    let notes = std::fs::read_to_string(notes.join("README.md")).unwrap();
    assert!(
        notes.contains("cargo"),
        "the scaffold's notes moved under agents/: {notes}"
    );
    // Its pre-approval sentence no longer says an unboxed seat keeps the
    // operator's MCP servers (SI2).
    assert_eq!(notes.matches("MCP\nservers").count(), 0, "{notes}");
}

#[test]
fn init_scaffolds_each_dialect_detection_outcome() {
    for (case, speckit, openspec, expected, notes) in [
        (
            "spec-kit",
            true,
            false,
            Some("speckit"),
            "`specify` at version `0.8.7`",
        ),
        (
            "OpenSpec",
            false,
            true,
            Some("openspec"),
            "`openspec` at version `1.12.0`",
        ),
        (
            "both",
            true,
            true,
            None,
            "Found both `.specify/` (spec-kit) and `openspec/config.yaml` (OpenSpec)",
        ),
        (
            "neither",
            false,
            false,
            None,
            "An artifact phase will need one",
        ),
    ] {
        let repo = repository();
        if speckit {
            std::fs::create_dir(repo.path().join(".specify")).unwrap();
        }
        if openspec {
            std::fs::create_dir(repo.path().join("openspec")).unwrap();
            std::fs::write(
                repo.path().join("openspec/config.yaml"),
                "schema: spec-driven\n",
            )
            .unwrap();
        }
        let bundle = repo.path().join(".brokkr");
        let (code, _, stderr) = brokkr(&["init", "."], repo.path());
        assert_eq!(code, Some(0), "{case}: {stderr}");

        let map = read_json(&bundle.join("realms.json"));
        assert_eq!(
            map["realms"][0].get("dialect").and_then(Value::as_str),
            expected,
            "{case}"
        );
        let world = brokkr_runtime::realms::World::load(&bundle.join("realms.json")).unwrap();
        assert_eq!(
            std::fs::canonicalize(world.path_of(&world.map.realms[0])).unwrap(),
            std::fs::canonicalize(repo.path()).unwrap(),
            "{case}: the scaffolded realm must be the repository detection read"
        );
        let compiled = Bundle::compile_with(
            &bundle,
            &bundle.join(DEFAULT_AGENTS_DIR),
            &bundle.join(DEFAULT_ADAPTERS_DIR),
        )
        .unwrap();
        let files = compiled.manifest["files"].as_object().unwrap();
        assert!(!files.contains_key("realms.json"), "{case}");
        assert!(
            !files.keys().any(|path| path.starts_with("dialects/")),
            "{case}"
        );
        let scaffold_notes = std::fs::read_to_string(bundle.join("agents/README.md")).unwrap();
        assert!(scaffold_notes.contains(notes), "{case}: {scaffold_notes}");

        if let Some(name) = expected {
            assert!(bundle
                .join("dialects")
                .join(format!("{name}.json"))
                .is_file());
            assert!(bundle
                .join("dialects")
                .join(name)
                .join("specify.md")
                .is_file());
        } else {
            assert!(!bundle.join("dialects").exists(), "{case}");
        }
    }
}

/// The chains `init`'s table hires codex's and dsh's offices on, each link
/// with its effort: written out again rather than read from `init.rs`,
/// like every table here. Claude's chain is the library's own,
/// `agents/<office>.json`, and dsh's reviewer stays on it, because dsh
/// cannot hold a gate (decision 0021 ruling 2).
type Chain = &'static [(&'static str, &'static str)];

const CHAINS: &[(&str, &str, Chain)] = &[
    ("codex", "intake", &[("sol", "medium"), ("terra", "high")]),
    (
        "codex",
        "implementer",
        &[("sol", "medium"), ("terra", "high")],
    ),
    ("codex", "reviewer", &[("sol", "medium")]),
    ("dsh", "intake", &[("flash", "high"), ("pro", "high")]),
    ("dsh", "implementer", &[("pro", "high"), ("flash", "high")]),
];

/// Decision 0087 ruling 1's projection of the library's portable base
/// office for a folder `init` scaffolded for `cli` on `stack`: the hiring
/// CLI's chain and efforts, a codex-hired office's hands, and a
/// claude-hired office's stack tools, or the base grant with no stack.
fn projected(cli: &str, office: &str, stack: Option<&str>) -> Value {
    let library = crate::workspace_root::workspace().join("agents");
    let mut definition = read_json(&library.join(format!("{office}.json")));
    let hired = if (cli, office) == ("dsh", "reviewer") {
        "claude"
    } else {
        cli
    };
    if let Some((.., chain)) = CHAINS.iter().find(|(c, o, _)| (*c, *o) == (hired, office)) {
        definition["models"] = chain.iter().map(|(model, _)| *model).collect();
        let efforts = chain
            .iter()
            .map(|(model, effort)| (model.to_string(), (*effort).into()));
        definition["efforts"] = Value::Object(efforts.collect());
    }
    match (hired, stack) {
        ("codex", _) => {
            let binds = match stack {
                Some("rust") => serde_json::json!([
                    {"path": "~/.cargo", "mode": "overlay", "mask": ["credentials.toml", "credentials"]},
                    {"path": "~/.rustup", "mode": "ro"},
                ]),
                _ => serde_json::json!([]),
            };
            definition["hands"] =
                serde_json::json!({"kind": "workspace", "network": false, "binds": binds});
        }
        ("claude", stack) => {
            let (work, gate) = stack.map_or(BASE, |stack| {
                let found = TOOLS.iter().find(|(fixture, ..)| *fixture == stack);
                found.map(|(_, work, gate)| (*work, *gate)).unwrap()
            });
            let allow = if office == "reviewer" { gate } else { work };
            definition["tools"] = serde_json::json!({"allow": allow, "mcp": []});
        }
        _ => {}
    }
    definition
}

/// Decision 0087 ruling 6: `init` writes the library's portable base.
/// Under claude, codex and dsh, each with no stack, a cargo stack and an
/// npm stack, every office it writes parses equal to `agents/<office>.json`
/// after ruling 1's projection. Under claude with no stack the projection
/// adds only the base grant every scaffold carries (#717, the operator's
/// ruling of 2026-10-22), and both files are `init`'s serialization byte
/// for byte: keys sorted, two-space indent, a closing newline.
#[test]
fn init_writes_the_librarys_portable_base_under_every_agent_cli() {
    let library = crate::workspace_root::workspace().join("agents");
    for cli in ["claude", "codex", "dsh"] {
        for stack in [None, Some("rust"), Some("node-npm")] {
            let repo = stack.map_or_else(repository, repository_of);
            let aside = tempfile::tempdir().unwrap();
            let (code, _, stderr) = brokkr_for(cli, &["init", "."], repo.path(), aside.path());
            assert_eq!(code, Some(0), "{cli} {stack:?}: {stderr}");
            let bundle = repo.path().join(".brokkr");
            for office in ["intake", "implementer", "reviewer"] {
                let case = format!("{cli} {stack:?} {office}");
                assert_eq!(
                    agent(&bundle, office),
                    projected(cli, office, stack),
                    "{case}"
                );
                let file = format!("{office}.json");
                if (cli, stack) == ("claude", None) {
                    let written = std::fs::read(bundle.join(DEFAULT_AGENTS_DIR).join(&file));
                    let source = std::fs::read(library.join(&file));
                    let expected = serialized(&projected(cli, office, stack));
                    assert_eq!(written.unwrap(), expected, "{case}");
                    let library_value = read_json(&library.join(&file));
                    assert_eq!(source.unwrap(), serialized(&library_value), "{case}");
                }
            }
        }
    }
}

/// `init`'s serialization of one definition: keys sorted, two-space
/// indent, a closing newline.
fn serialized(value: &Value) -> Vec<u8> {
    format!("{}\n", serde_json::to_string_pretty(value).unwrap()).into_bytes()
}
