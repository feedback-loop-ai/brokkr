//! Decision 0038 rulings 2, 3 and 6, driven through the real gate: a real
//! run (fake-driver seats, real engine, real anchor) delivers a slice,
//! the branch is rebased, edited, and labelled, and
//! `scripts/delivered-by-brokkr.sh` cuts the tier the ruling names each
//! time. Unix only: the gate is a shell script, because CI is.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn commit(repo: &Path, path: &str, contents: &str, message: &str) -> String {
    let file = repo.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, contents).unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", message]);
    git(repo, &["rev-parse", "HEAD"])
}

/// A delivery table with every phase seated by the fake driver, and a
/// preflight table (verify, review, done) in the same shape.
const DELIVERY_POLICY: &str = r#"{
  "schema": "forge.phase-machine/v1",
  "phases": ["implement", "verify", "review", "ship", "done", "stop"],
  "initial": "implement",
  "terminal": ["done", "stop"],
  "shippable_from": ["review"],
  "rules": [
    {"id": "IMPL-OK", "from": "implement", "result": "complete", "next": "verify", "reason": "r"},
    {"id": "VERIFY-FAIL", "from": "verify", "result": "fail", "next": "stop", "severity": "hard", "reason": "r"},
    {"id": "VERIFY-PASS", "from": "verify", "result": "pass", "next": "review", "reason": "r"},
    {"id": "REVIEW-CLEAN-NO-FIXES", "from": "review", "result": "clean", "when": {"fixes_applied": false}, "next": "ship", "reason": "r"},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "verify", "reason": "r"},
    {"id": "SHIP-DIRTY", "from": "ship", "result": "ready", "when": {"dirty_worktrees": true}, "next": "stop", "severity": "hard", "reason": "r"},
    {"id": "SHIP-READY", "from": "ship", "result": "ready", "next": "ship", "reason": "r"},
    {"id": "SHIP-COMPLETE", "from": "ship", "result": "shipped", "next": "done", "reason": "r"}
  ]
}"#;

const PREFLIGHT_POLICY: &str = r#"{
  "schema": "forge.phase-machine/v1",
  "phases": ["verify", "review", "done", "stop"],
  "initial": "verify",
  "terminal": ["done", "stop"],
  "shippable_from": ["review"],
  "rules": [
    {"id": "VERIFY-FAIL", "from": "verify", "result": "fail", "next": "stop", "severity": "hard", "reason": "r"},
    {"id": "VERIFY-PASS", "from": "verify", "result": "pass", "next": "review", "reason": "r"},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "when": {"fixes_applied": false}, "next": "done", "reason": "r"},
    {"id": "REVIEW-CLEAN-FIXED", "from": "review", "result": "clean", "next": "stop", "severity": "hard", "reason": "r"}
  ]
}"#;

/// Drive one real run over `repo` at its current HEAD and return its id.
/// The bundle, journal and driver state live beside the repository, so
/// the tree the ship seat inspects stays clean.
fn deliver(side: &Path, repo: &Path, name: &str, policy: &str, script: Value) -> String {
    let bundle = side.join(name).join("bundle");
    std::fs::create_dir_all(bundle.join("roles")).unwrap();
    std::fs::create_dir_all(side.join(name).join("state")).unwrap();
    std::fs::write(bundle.join("policy.json"), policy).unwrap();
    let script_path = side.join(name).join("script.json");
    std::fs::write(&script_path, serde_json::to_string(&script).unwrap()).unwrap();
    std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
    let seat = |results: Vec<&str>| {
        json!({
            "role": "roles/role.md",
            "results": results,
            "driver": {"command": [
                brokkr_bin(), "fake-driver",
                "--script", script_path.to_string_lossy(),
                "--state", side.join(name).join("state").to_string_lossy(),
            ]}
        })
    };
    let mut seats = serde_json::Map::new();
    if policy.contains("\"implement\"") {
        seats.insert("implement".into(), seat(vec!["complete"]));
        seats.insert("ship".into(), seat(vec!["ready", "shipped"]));
    }
    seats.insert("verify".into(), seat(vec!["pass", "fail"]));
    seats.insert("review".into(), seat(vec!["clean"]));
    std::fs::write(
        bundle.join("bundle.json"),
        serde_json::to_string_pretty(
            &json!({"name": name, "policy": "policy.json", "seats": seats}),
        )
        .unwrap(),
    )
    .unwrap();
    let out = Command::new(brokkr_bin())
        .args([
            "run",
            "--bundle",
            bundle.to_str().unwrap(),
            "--repo",
            repo.to_str().unwrap(),
            "--db",
            side.join("forge.db").to_str().unwrap(),
            "--feature",
            name,
        ])
        .current_dir(side)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{name}: {stderr}");
    let run_id = stderr
        .lines()
        .find_map(|line| line.strip_prefix("run started: "))
        .expect("run id")
        .trim()
        .to_string();
    // What CONTRIBUTING.md has the contributor push: the anchor ref,
    // published under refs/heads/brokkr-runs/<run>.
    git(
        repo,
        &[
            "update-ref",
            &format!("refs/heads/brokkr-runs/{run_id}"),
            &format!("refs/forge/{run_id}"),
        ],
    );
    run_id
}

fn delivery_script() -> Value {
    json!({"seats": {
        "implement": [{"behavior": "succeed", "result": {"result": "complete"}}],
        "verify": [{"behavior": "succeed", "result": {"result": "pass"}}],
        "review": [{"behavior": "succeed", "result": {"result": "clean", "inputs": {"fixes_applied": false}}}],
        "ship": [
            {"behavior": "succeed", "result": {"result": "ready"}},
            {"behavior": "succeed", "result": {"result": "shipped"}},
        ],
    }})
}

fn preflight_script() -> Value {
    json!({"seats": {
        "verify": [{"behavior": "succeed", "result": {"result": "pass"}}],
        "review": [{"behavior": "succeed", "result": {"result": "clean", "inputs": {"fixes_applied": false}}}],
    }})
}

struct Gate {
    repo: PathBuf,
    evidence: PathBuf,
}

impl Gate {
    /// Run the gate script exactly as CI does, with the given body, head
    /// and labels. Returns (exit code, stdout + stderr).
    fn judge(&self, body: &str, head: &str, labels: &str) -> (i32, String) {
        let out = Command::new("bash")
            .arg(workspace().join("scripts/delivered-by-brokkr.sh"))
            .env("PR_BODY", body)
            .env("PR_HEAD", head)
            .env("PR_BASE", "main")
            .env("REPO", &self.repo)
            .env("EVIDENCE", &self.evidence)
            .env("VERIFIER", brokkr_bin())
            .env("LABELS", labels)
            .env("CLASSES", workspace().join(".github/delivery-classes.json"))
            .output()
            .unwrap();
        let log = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        (out.status.code().unwrap_or(-1), log)
    }
}

#[test]
fn the_gate_cuts_the_tier_by_the_delta_since_the_judgment() {
    let side = tempfile::tempdir().unwrap();
    let repo = side.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "gate@example.invalid"],
        vec!["config", "user.name", "gate"],
        vec!["config", "commit.gpgsign", "false"],
        vec!["symbolic-ref", "HEAD", "refs/heads/main"],
    ] {
        git(&repo, &args);
    }
    commit(&repo, "src/lib.txt", "one\n", "base code");
    commit(&repo, "README.md", "# app\n", "base readme");

    // The slice: one code hunk, one new page, and one page whose name
    // carries pathspec magic, a glob and a non-ASCII byte — the engine
    // and the gate must give it the same id, or the rebase below costs a
    // new run.
    git(&repo, &["checkout", "-q", "-b", "slice"]);
    std::fs::write(repo.join("src/lib.txt"), "one\ntwo\n").unwrap();
    commit(&repo, "docs/:[hliðskjálf].md", "# seen\n", "a named page");
    let judged = commit(&repo, "docs/page.md", "# page\n", "the slice");

    let run = deliver(
        side.path(),
        &repo,
        "delivery",
        DELIVERY_POLICY,
        delivery_script(),
    );
    let gate = Gate {
        repo: repo.clone(),
        evidence: repo.clone(),
    };
    let body = format!("## What changed\n\nBrokkr-Run: {run}\n");

    // The head the run named: vouched, as 0033 always read.
    let (code, log) = gate.judge(&body, &judged, "");
    assert_eq!(code, 0, "{log}");
    assert!(log.contains("tier vouched"), "{log}");

    // main moves under the slice; a clean rebase keeps the vouch (ruling 2).
    git(&repo, &["checkout", "-q", "main"]);
    commit(&repo, "CHANGELOG.md", "moved\n", "main moves");
    git(&repo, &["checkout", "-q", "slice"]);
    git(&repo, &["rebase", "-q", "main"]);
    let rebased = git(&repo, &["rev-parse", "HEAD"]);
    assert_ne!(rebased, judged);
    let (code, log) = gate.judge(&body, &rebased, "");
    assert_eq!(code, 0, "{log}");
    assert!(
        log.contains("tier vouched · delta since the judgment: []"),
        "{log}"
    );
    assert!(
        log.contains(&format!("run {run} vouches for {rebased}")),
        "{log}"
    );

    // Whitespace is content: one space added inside the judged hunk is a
    // different patch, and a code one. A space is semantic in shell, YAML
    // and Python; `patch-id --stable` alone would have stripped it and
    // kept the vouch.
    git(&repo, &["checkout", "-q", "-b", "respaced"]);
    let respaced = commit(&repo, "src/lib.txt", "one\n two\n", "one space");
    git(&repo, &["checkout", "-q", "slice"]);
    let (code, log) = gate.judge(&body, &respaced, "");
    assert_eq!(code, 1, "{log}");
    assert!(
        log.contains("tier code · delta since the judgment: [\"src/lib.txt\"]"),
        "{log}"
    );

    // A page changes after the judgment: docs tier, and it wants a preflight.
    let docs_delta = commit(&repo, "docs/page.md", "# page\n\nmore\n", "edit the page");
    let (code, log) = gate.judge(&body, &docs_delta, "");
    assert_eq!(code, 1, "{log}");
    assert!(
        log.contains("tier docs · delta since the judgment: [\"docs/page.md\"]"),
        "{log}"
    );
    assert!(log.contains("Brokkr-Preflight"), "{log}");

    // The operator's label skips every tier and the tier is logged (ruling 6).
    let (code, log) = gate.judge(&body, &docs_delta, "docs,by-hand");
    assert_eq!(code, 0, "{log}");
    assert!(log.contains("the tier would have been docs"), "{log}");

    // A preflight over the new head satisfies the docs tier (ruling 3).
    let preflight = deliver(
        side.path(),
        &repo,
        "preflight",
        PREFLIGHT_POLICY,
        preflight_script(),
    );
    let with_preflight = format!("{body}Brokkr-Preflight: {preflight}\n");
    let (code, log) = gate.judge(&with_preflight, &docs_delta, "");
    assert_eq!(code, 0, "{log}");
    assert!(
        log.contains(&format!("preflight {preflight} judged {docs_delta}")),
        "{log}"
    );

    // A preflight that judged some other head is not this head's judgment.
    let another_page = commit(&repo, "docs/other.md", "# other\n", "another page");
    let (code, log) = gate.judge(&with_preflight, &another_page, "");
    assert_eq!(code, 1, "{log}");
    assert!(
        log.contains(&format!(
            "preflight {preflight} judged {docs_delta}, not {another_page}"
        )),
        "{log}"
    );

    // A code file moved under a docs name after the judgment: a rename is
    // never paired, so the deletion stays in the delta and cuts the tier
    // to code. Paired, the delta would read as two pages.
    git(&repo, &["checkout", "-q", "-b", "moved", &docs_delta]);
    git(&repo, &["mv", "src/lib.txt", "docs/lib.md"]);
    git(&repo, &["commit", "-q", "-m", "move the code under docs"]);
    let moved = git(&repo, &["rev-parse", "HEAD"]);
    git(&repo, &["checkout", "-q", "slice"]);
    let (code, log) = gate.judge(&with_preflight, &moved, "");
    assert_eq!(code, 1, "{log}");
    assert!(
        log.contains(
            "tier code · delta since the judgment: [\"docs/lib.md\",\"docs/page.md\",\"src/lib.txt\"]"
        ),
        "{log}"
    );

    // Code changes after the judgment: a new run, or the label. The
    // delta is everything since the judgment, the pages included — one
    // code path in it is enough to cut the tier.
    let code_delta = commit(&repo, "src/lib.txt", "one\ntwo\nthree\n", "more code");
    let (code, log) = gate.judge(&body, &code_delta, "");
    assert_eq!(code, 1, "{log}");
    assert!(
        log.contains(
            "tier code · delta since the judgment: [\"docs/other.md\",\"docs/page.md\",\"src/lib.txt\"]"
        ),
        "{log}"
    );
    assert!(log.contains("A new run must vouch for"), "{log}");

    // No declaration at all: a refusal, unless the operator labelled it.
    let (code, log) = gate.judge("## What changed\n", &code_delta, "");
    assert_eq!(code, 1, "{log}");
    assert!(log.contains("expected exactly one Brokkr-Run"), "{log}");
    let (code, log) = gate.judge("## What changed\n", &code_delta, "by-hand");
    assert_eq!(code, 0, "{log}");
    assert!(log.contains("the tier would have been unknown"), "{log}");

    // A declaration the gate refuses — here, a run with no published
    // evidence — is a refusal without the label and a logged skip with it
    // (0033 ruling 5: with the label, the job succeeds without evidence).
    let unpublished = "## What changed\n\nBrokkr-Run: never-ran-00000000\n";
    let (code, log) = gate.judge(unpublished, &code_delta, "");
    assert_eq!(code, 1, "{log}");
    assert!(
        log.contains("no published evidence for run never-ran-00000000"),
        "{log}"
    );
    let (code, log) = gate.judge(unpublished, &code_delta, "by-hand");
    assert_eq!(code, 0, "{log}");
    assert!(
        log.contains("no published evidence for run never-ran-00000000"),
        "{log}"
    );
    assert!(log.contains("the tier could not be cut"), "{log}");
}

/// A v2 anchor carries no patch identity, so it vouches only for the head
/// it names — exactly what 0033 read, for every run anchored before 0038.
#[test]
fn a_v2_anchor_vouches_for_its_head_and_nothing_else() {
    let side = tempfile::tempdir().unwrap();
    let repo = side.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "gate@example.invalid"],
        vec!["config", "user.name", "gate"],
        vec!["config", "commit.gpgsign", "false"],
        vec!["symbolic-ref", "HEAD", "refs/heads/main"],
    ] {
        git(&repo, &args);
    }
    commit(&repo, "src/lib.txt", "one\n", "base");
    git(&repo, &["checkout", "-q", "-b", "slice"]);
    let judged = commit(&repo, "src/lib.txt", "one\ntwo\n", "the slice");
    let run = deliver(
        side.path(),
        &repo,
        "delivery",
        DELIVERY_POLICY,
        delivery_script(),
    );

    // Rewrite the published anchor as the v2 shape: same tree, same
    // journal, the patch identity dropped.
    let evidence_ref = format!("refs/heads/brokkr-runs/{run}");
    let tree = git(&repo, &["rev-parse", &format!("{evidence_ref}^{{tree}}")]);
    let message: Value =
        serde_json::from_str(&git(&repo, &["log", "-1", "--format=%B", &evidence_ref])).unwrap();
    let v2 = json!({
        "anchor": "forge.journal-anchor/v2",
        "run_id": message["run_id"],
        "seq": message["seq"],
        "journal_head_hash": message["journal_head_hash"],
        "repo_head": message["repo_head"],
    })
    .to_string();
    let rewritten = {
        let out = Command::new("git")
            .args(["commit-tree", &tree, "-m", &v2])
            .current_dir(&repo)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    git(&repo, &["update-ref", &evidence_ref, &rewritten]);

    let gate = Gate {
        repo: repo.clone(),
        evidence: repo.clone(),
    };
    let body = format!("Brokkr-Run: {run}\n");
    let (code, log) = gate.judge(&body, &judged, "");
    assert_eq!(code, 0, "{log}");

    git(&repo, &["checkout", "-q", "main"]);
    commit(&repo, "CHANGELOG.md", "moved\n", "main moves");
    git(&repo, &["checkout", "-q", "slice"]);
    git(&repo, &["rebase", "-q", "main"]);
    let rebased = git(&repo, &["rev-parse", "HEAD"]);
    let (code, log) = gate.judge(&body, &rebased, "");
    assert_eq!(code, 1, "{log}");
    assert!(log.contains("tier code"), "{log}");
}
