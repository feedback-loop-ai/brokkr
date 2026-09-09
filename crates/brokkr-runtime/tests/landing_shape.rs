//! `recipes/landing`'s shape, made a test rather than a comment
//! (decision 0051).
//!
//! The recipe is `fast` entered at a classify gate: the branch already
//! exists, so there is no first implement; a seconds-long exec seat reads
//! the branch's class against the repository's docs class and routes
//! prose to `review` and code to `verify`; the smith is the remediation
//! seat the verify failure and the reforging edge return to. Everything
//! else — every rule, every seat, the protected phase — is `fast`'s, so a
//! landing is judged exactly as a delivery is. That is a structural
//! claim, so it is asserted structurally, and the classify script is
//! driven over a real repository so its two answers are pinned too.

use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Command;

use brokkr_runtime::Bundle;
use serde_json::Value;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// The pinned digest of `policy.json`, wherever the manifest files it.
fn policy_digest(manifest: &Value) -> String {
    fn walk(value: &Value) -> Option<String> {
        match value {
            Value::Object(map) => map
                .get("policy.json")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| map.values().find_map(walk)),
            Value::Array(items) => items.iter().find_map(walk),
            _ => None,
        }
    }
    walk(manifest).expect("the manifest pins policy.json")
}

fn compile(path: &Path) -> Bundle {
    let root = workspace();
    Bundle::compile_with(path, &root.join("agents"), &root.join("adapters"))
        .unwrap_or_else(|error| panic!("{} compiles: {error}", path.display()))
}

#[test]
fn landing_is_fast_entered_at_classify_and_nothing_else() {
    let root = workspace();
    let landing = compile(&root.join("recipes/landing"));
    let fast = compile(&root.join("recipes/fast"));

    assert_eq!(
        landing.machine.initial, "classify",
        "a landing classifies before it writes"
    );
    assert_eq!(fast.machine.initial, "implement", "fast still writes first");
    let mut phases = fast.machine.phases.clone();
    phases.push("classify".into());
    assert_eq!(
        landing.machine.phases, phases,
        "the phases are fast's plus classify"
    );
    assert_eq!(landing.machine.terminal, fast.machine.terminal);
    assert_eq!(
        landing.protected_phase, fast.protected_phase,
        "review is the protected phase"
    );

    let ids = |bundle: &Bundle| -> Vec<String> {
        bundle
            .machine
            .rules
            .iter()
            .map(|rule| rule.id.clone())
            .collect()
    };
    let mut expected = vec!["CLASSIFY-DOCS".to_string(), "CLASSIFY-CODE".to_string()];
    expected.extend(ids(&fast));
    assert_eq!(
        ids(&landing),
        expected,
        "landing adds the two classify arms before fast's rules and nothing else"
    );

    let mut seats: Vec<&String> = fast.seats.keys().collect();
    let classify = "classify".to_string();
    seats.push(&classify);
    seats.sort();
    assert_eq!(
        landing.seats.keys().collect::<Vec<_>>(),
        seats,
        "the seats are fast's plus classify"
    );
    assert!(landing.seats["classify"].has_gate, "classify is a gate");

    assert_eq!(
        landing
            .chain
            .iter()
            .map(|ancestor| ancestor.name.as_str())
            .collect::<Vec<_>>(),
        ["fast"],
        "landing extends fast and nothing between"
    );
    assert_ne!(
        policy_digest(&landing.manifest),
        policy_digest(&fast.manifest),
        "the table moved, so the policy digest is its own"
    );
}

/// The remediation edges must stand from the initial phase's two routes,
/// and no route reaches a terminal without passing review.
#[test]
fn prose_is_judged_and_code_is_built_and_both_pass_review() {
    let landing = compile(&workspace().join("recipes/landing"));
    let rules = &landing.machine.rules;
    let leads = |from: &str, result: &str, next: &str| {
        rules.iter().any(|rule| {
            rule.from == from && rule.result == result && rule.next.as_deref() == Some(next)
        })
    };
    assert!(
        leads("classify", "docs", "review"),
        "a docs-only branch is judged, not built"
    );
    assert!(
        leads("classify", "code", "verify"),
        "code is verified before it is judged"
    );
    assert!(
        leads("verify", "fail", "implement"),
        "a failed verify returns to the smith"
    );
    assert!(
        leads("review", "residual", "implement"),
        "a residual above low returns to the smith"
    );
    assert!(
        leads("implement", "complete", "verify"),
        "the smith's work is re-verified"
    );
    assert!(
        !rules
            .iter()
            .any(|rule| rule.from == "classify"
                && matches!(rule.next.as_deref(), Some("ship" | "done"))),
        "classify never reaches ship or done directly"
    );
}

#[cfg(unix)]
fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        // The fixture must not inherit a host's `commit.gpgsign=true`,
        // which cannot sign where no agent is reachable; every other git
        // fixture in the tree disables signing for the same reason.
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "landing")
        .env("GIT_AUTHOR_EMAIL", "landing@example")
        .env("GIT_COMMITTER_NAME", "landing")
        .env("GIT_COMMITTER_EMAIL", "landing@example")
        // Seat commits are unsigned (CONTRIBUTING); a host whose global
        // config signs would otherwise fail this fixture before the
        // engine ever runs.
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
        .env("GIT_CONFIG_VALUE_0", "false")
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

#[cfg(unix)]
fn classify(repo: &Path) -> Value {
    let results = repo.join(".forge/results");
    std::fs::create_dir_all(&results).unwrap();
    let result_path = results.join("classify.json");
    let prompt = repo.join("prompt.txt");
    std::fs::write(
        &prompt,
        format!(
            "Write the result to exactly this file:\n\n    {}\n",
            result_path.display()
        ),
    )
    .unwrap();
    let script = workspace().join("recipes/landing/scripts/classify-seat.sh");
    let status = Command::new("bash")
        .arg(&script)
        .arg(&prompt)
        .current_dir(repo)
        .status()
        .expect("the classify script runs");
    assert!(status.success());
    serde_json::from_str(&std::fs::read_to_string(&result_path).unwrap()).expect("a JSON result")
}

/// The classify script over a real repository: a docs-only branch
/// answers `docs`, one code path turns the whole branch `code`, and a
/// repository without a docs class lands everything as code.
///
/// Unix only, like every test that drives a seat script through `bash`
/// (`delivered_by_brokkr.rs`, `driver_conformance.rs`): the seat runs in
/// the box on the platforms that have one, and on Windows the result
/// path the prompt carries is a drive path the script's `/…json` reader
/// does not recognise. Decision 0049 ruling 3: named as not holding
/// there, not pursued.
#[cfg(unix)]
#[test]
fn the_classify_gate_reads_the_repositorys_own_docs_class() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path();
    git(repo, &["init", "-q", "-b", "main"]);
    std::fs::create_dir_all(repo.join(".github")).unwrap();
    std::fs::copy(
        workspace().join(".github/delivery-classes.json"),
        repo.join(".github/delivery-classes.json"),
    )
    .unwrap();
    std::fs::write(repo.join("README.md"), "# base\n").unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", "base"]);
    git(repo, &["switch", "-q", "-c", "branch"]);

    std::fs::create_dir_all(repo.join("docs/decisions")).unwrap();
    std::fs::write(
        repo.join("docs/decisions/0099-a-ruling.md"),
        "# 0099\n\nStatus: proposed\n",
    )
    .unwrap();
    std::fs::write(repo.join("README.md"), "# base\n\nand a sentence\n").unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", "prose"]);
    let docs = classify(repo);
    assert_eq!(docs["result"], "docs", "{docs}");
    assert!(
        docs["notes"].as_str().unwrap().contains("2 path(s)"),
        "{docs}"
    );
    assert_eq!(
        docs.as_object().unwrap().len(),
        2,
        "result and notes, nothing else: {docs}"
    );

    std::fs::create_dir_all(repo.join("src")).unwrap();
    std::fs::write(repo.join("src/lib.rs"), "pub fn landed() {}\n").unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", "code"]);
    let code = classify(repo);
    assert_eq!(code["result"], "code", "{code}");
    assert!(
        code["notes"].as_str().unwrap().contains("src/lib.rs"),
        "{code}"
    );

    git(repo, &["rm", "-q", ".github/delivery-classes.json"]);
    git(repo, &["commit", "-q", "-m", "no class"]);
    let unclassed = classify(repo);
    assert_eq!(unclassed["result"], "code", "{unclassed}");
    assert!(
        unclassed["notes"].as_str().unwrap().contains("absent"),
        "{unclassed}"
    );
}
