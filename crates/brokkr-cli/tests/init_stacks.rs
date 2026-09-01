//! `brokkr init` looks before it scaffolds: the repository it is invoked
//! from is read for the manifests and lockfiles at its root, and the two
//! charters that tell a seat to build and to prove name that stack's own
//! commands. What is asserted here is what an operator would read in
//! `roles/`, per stack — and, for a repository carrying no marker init
//! knows, that the charter says so in those words instead of dressing a
//! placeholder as a choice.
//!
//! Every scaffold is made in a tempdir the fixture's markers are COPIED
//! into. Never in the checked-in fixture itself: `init` writes, and a
//! test that wrote into the committed tree would be a test that edited
//! the repository to pass.

use std::path::{Path, PathBuf};
use std::process::Command;

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
        "python",
        "python",
        "python -m build",
        "python -m pytest",
        "python -m ruff check .",
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

fn charter(bundle: &Path, name: &str) -> String {
    std::fs::read_to_string(bundle.join("roles").join(name)).unwrap()
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
        for (other, _, other_build, other_test, other_lint) in RECOGNIZED {
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
        for (_, _, build, test, lint) in RECOGNIZED {
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

/// Introspection rewrote prose, not the roster. Every scaffold — each
/// recognized stack and the fallback alike — still compiles, and what it
/// compiles to still carries decision 0021 ruling 1's division: the
/// three judging seats declare `gate`, the two working seats declare
/// `work`, and the compiled manifest pins the adapter that authorised a
/// judgement for exactly those three. That pin is the gate class made
/// visible in the output — a work seat never earns one — so a class left
/// to default would show up here as a roster of nobody.
#[test]
fn every_scaffolded_recipe_compiles_with_its_gates_still_gates() {
    let fixtures = RECOGNIZED
        .iter()
        .map(|(fixture, ..)| *fixture)
        .chain(["generic", "rust-and-make"]);
    for fixture in fixtures {
        let (_repo, bundle) = scaffold_from(fixture);
        let (code, stdout, stderr) = brokkr(&["compile", "--bundle", "."], &bundle);
        assert_eq!(code, Some(0), "{fixture}: {stderr}");
        assert!(stdout.contains("\"starter\""), "{fixture}: {stdout}");

        let compiled: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let judged: Vec<&String> = compiled["manifest"]["drivers"]
            .as_object()
            .unwrap_or_else(|| panic!("{fixture}: no seat compiled as a gate: {stdout}"))
            .keys()
            .collect();
        assert_eq!(judged, ["review", "ship", "verify"], "{fixture}: {stdout}");

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
