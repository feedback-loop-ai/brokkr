//! Decision 0038 rulings 2, 3 and 6, driven through the real gate: a real
//! run (fake-driver seats, real engine, real anchor) delivers a slice,
//! the branch is rebased, edited, and labelled, and
//! `scripts/delivered-by-brokkr.sh` cuts the tier the ruling names each
//! time. Decision 0046 ruling 3, through the same gate: a run whose boxed
//! gate stood under `harness` is rendered `unboxed`, a `namespace` run
//! carries no adjective, and a journal that recorded no boundary — or a
//! malformed one — reads `boundary not recorded`. Unix only: the gate is
//! a shell script, because CI is.

#![cfg(unix)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::EventEnvelope;
use serde_json::{json, Value};

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

/// A real namespace can be built here: bubblewrap answers, and this
/// process is not itself inside a box (`BROKKR_HANDS_BOX`), where a
/// box-building test skips explicitly rather than nesting.
fn can_create_namespace() -> bool {
    if std::env::var_os(brokkr_protocol::hands::HANDS_BOX_ENV).is_some() {
        return false;
    }
    Command::new("bwrap")
        .args(["--ro-bind", "/", "/", "--", "true"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
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

/// The bundle's own pinned verify script: reads the result path the
/// prompt states and writes `pass`, and nothing else. Under `harness` it
/// runs unboxed, in the fixed environment; under `namespace`, in the box.
const VERIFY_SCRIPT: &str = r#"#!/usr/bin/env bash
set -u
prompt_file="${1:-}"
[ -f "$prompt_file" ] || { echo "verify: prompt file missing" >&2; exit 2; }
result_path=""
while IFS= read -r line; do
    trimmed="${line#"${line%%[![:space:]]*}"}"
    trimmed="${trimmed%"${trimmed##*[![:space:]]}"}"
    case "$trimmed" in /*.json) result_path="$trimmed" ;; esac
done < "$prompt_file"
[ -n "$result_path" ] || { echo "verify: result path missing from prompt" >&2; exit 2; }
mkdir -p "$(dirname "$result_path")"
printf '{"result": "pass", "notes": "the pinned script ran"}\n' > "$result_path"
"#;

/// Drive one real run over `repo` at its current HEAD and return its id.
/// The bundle, journal and driver state live beside the repository, so
/// the tree the ship seat inspects stays clean.
fn deliver(side: &Path, repo: &Path, name: &str, policy: &str, script: Value) -> String {
    deliver_in(side, repo, name, policy, script, false, None)
}

/// [`deliver`], with the verify seat optionally a boxed exec gate running
/// the bundle's own pinned script (decision 0043 ruling 3), and the run
/// started under the named realms map — whose realm for `repo` declares
/// the boundary the gate stands under (decision 0046 ruling 1).
fn deliver_in(
    side: &Path,
    repo: &Path,
    name: &str,
    policy: &str,
    script: Value,
    boxed_verify: bool,
    realms: Option<&Path>,
) -> String {
    let bundle = delivery_bundle(side, name, policy, script, boxed_verify);
    let extra = realms
        .map(|path| vec!["--realms", path.to_str().unwrap()])
        .unwrap_or_default();
    let (code, stderr) = drive_delivery(side, repo, &bundle, name, &extra);
    assert_eq!(code, Some(0), "{name}: {stderr}");
    let run = started_run(&stderr);
    publish_anchor(repo, &run);
    run
}

fn delivery_bundle(
    side: &Path,
    name: &str,
    table: &str,
    script: Value,
    boxed_verify: bool,
) -> PathBuf {
    let bundle = side.join(name).join("bundle");
    std::fs::create_dir_all(bundle.join("roles")).unwrap();
    std::fs::create_dir_all(side.join(name).join("state")).unwrap();
    std::fs::write(bundle.join("policy.json"), table).unwrap();
    let script_path = side.join(name).join("script.json");
    std::fs::write(&script_path, serde_json::to_string(&script).unwrap()).unwrap();
    std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
    // Each delivery's seats hire an office of its own beside the run (U9b1).
    let office = format!("{name}-office");
    let driver = crate::fake_office::fake_driver(&script_path, &side.join(name).join("state"));
    crate::fake_office::hire_driver(side, [&office, &format!("{name}-fake")], driver);
    let seat = |results: Vec<&str>| json!({"results": results, "agent": office});
    let mut seats = serde_json::Map::new();
    if table.contains("\"implement\"") {
        seats.insert("implement".into(), seat(vec!["complete"]));
        seats.insert("ship".into(), seat(vec!["ready", "shipped"]));
    }
    if boxed_verify {
        // A gate seat opens the workspace adapters (decision 0021), so
        // the exec adapter stands beside the run exactly as it does in
        // the repository.
        std::fs::create_dir_all(side.join("adapters")).unwrap();
        std::fs::copy(
            workspace().join("adapters/exec.json"),
            side.join("adapters/exec.json"),
        )
        .unwrap();
        std::fs::create_dir_all(bundle.join("scripts")).unwrap();
        std::fs::write(bundle.join("scripts/verify.sh"), VERIFY_SCRIPT).unwrap();
        seats.insert(
            "verify".into(),
            json!({
                "role": "roles/role.md",
                "results": ["pass", "fail"],
                "class": "gate",
                "hands": "workspace",
                "driver": {"command": [
                    "{brokkr}", "driver", "exec", "--", "bash", "./scripts/verify.sh", "{prompt_file}",
                ]}
            }),
        );
    } else {
        seats.insert("verify".into(), seat(vec!["pass", "fail"]));
    }
    seats.insert("review".into(), seat(vec!["clean"]));
    std::fs::write(
        bundle.join("bundle.json"),
        serde_json::to_string_pretty(
            &json!({"name": name, "policy": "policy.json", "seats": seats}),
        )
        .unwrap(),
    )
    .unwrap();
    bundle
}

fn drive_delivery(
    side: &Path,
    repo: &Path,
    bundle: &Path,
    name: &str,
    extra: &[&str],
) -> (Option<i32>, String) {
    let mut args = vec![
        "run".to_string(),
        "--bundle".to_string(),
        bundle.to_str().unwrap().to_string(),
        "--repo".to_string(),
        repo.to_str().unwrap().to_string(),
        "--db".to_string(),
        side.join("forge.db").to_str().unwrap().to_string(),
        "--feature".to_string(),
        name.to_string(),
    ];
    args.extend(extra.iter().map(|arg| (*arg).to_string()));
    crate::engine::library(side);
    let out = Command::new(brokkr_bin())
        .args(&args)
        .env("HOME", crate::engine::home_in(side))
        .current_dir(side)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn started_run(stderr: &str) -> String {
    stderr
        .lines()
        .find_map(|line| line.strip_prefix("run started: "))
        .expect("run id")
        .trim()
        .to_string()
}

fn publish_anchor(repo: &Path, run_id: &str) {
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
    fn accepted(&self, body: &str, head: &str) -> String {
        let (code, log) = self.judge(body, head, "");
        assert_eq!(code, 0, "{log}");
        log
    }

    fn local(repo: &Path) -> Self {
        Self {
            repo: repo.into(),
            evidence: repo.into(),
        }
    }

    fn delivering(side: &Path, repo: &Path, name: &str) -> (String, Self) {
        let run = deliver(side, repo, name, DELIVERY_POLICY, delivery_script());
        (
            run,
            Self {
                repo: repo.into(),
                evidence: repo.into(),
            },
        )
    }

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

/// A fresh repository with `main` checked out, ready to commit into.
fn init_repo(side: &Path) -> PathBuf {
    let repo = side.join("repo");
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
    // Engine scratch is runtime state. Instrumented drivers whose fixed
    // environment clears LLVM_PROFILE_FILE also leave their default profile.
    std::fs::write(repo.join(".git/info/exclude"), ".forge/\n*.profraw\n").unwrap();
    repo
}

/// A `forge.realms/v4` map naming `repo` as its one realm, under the
/// given boundary word (decision 0046 ruling 1): the one place the word
/// is declared, and never the bundle.
fn write_realms(side: &Path, repo: &Path, boundary: &str) -> PathBuf {
    let map = side.join("realms.json");
    std::fs::write(
        &map,
        json!({
            "schema": "forge.realms/v4",
            "realms": [{
                "name": "judged",
                "path": repo.to_str().unwrap(),
                "default_branch": "main",
                "boundary": boundary,
            }],
            "journal": "forge.db",
        })
        .to_string(),
    )
    .unwrap();
    map
}

/// The line of the gate's log that starts with the given prefix, whole.
fn line_starting(log: &str, prefix: &str) -> String {
    log.lines()
        .find(|line| line.starts_with(prefix))
        .unwrap_or_else(|| panic!("no line starts with {prefix:?}:\n{log}"))
        .to_string()
}

/// The published journal of one run, as the gate fetches it.
fn published_journal(repo: &Path, run: &str) -> Vec<EventEnvelope> {
    git(
        repo,
        &[
            "show",
            &format!("refs/heads/brokkr-runs/{run}:{run}.ndjson"),
        ],
    )
    .lines()
    .filter(|line| !line.trim().is_empty())
    .map(|line| serde_json::from_str(line).unwrap())
    .collect()
}

/// Re-publish one run's evidence with its journal edited: every event
/// re-sealed and re-chained so `verify-run` still verifies it, the
/// anchor re-written to name the new journal head over the same tree
/// otherwise. What this engine never writes — a boxed manifest with no
/// `boundary`, an entry outside the vocabulary — is exactly what an
/// older or foreign engine could have, and the gate reads the journal,
/// not the engine that wrote it.
fn republish(repo: &Path, run: &str, edit: impl FnOnce(&mut Vec<EventEnvelope>)) {
    let evidence_ref = format!("refs/heads/brokkr-runs/{run}");
    let mut events = published_journal(repo, run);
    edit(&mut events);
    let mut previous = ZERO_HASH.to_string();
    for event in &mut events {
        event.previous_hash = previous.clone();
        event.event_hash = event.compute_hash();
        previous = event.event_hash.clone();
    }
    let ndjson = events
        .iter()
        .map(|event| serde_json::to_string(event).unwrap() + "\n")
        .collect::<String>();
    let blob = git_with_stdin(repo, &["hash-object", "-w", "--stdin"], &ndjson);
    let listing = git(repo, &["ls-tree", &evidence_ref])
        .lines()
        .map(|line| {
            if line.ends_with(&format!("\t{run}.ndjson")) {
                format!("100644 blob {blob}\t{run}.ndjson")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let tree = git_with_stdin(repo, &["mktree"], &listing);
    let mut anchor: Value =
        serde_json::from_str(&git(repo, &["log", "-1", "--format=%B", &evidence_ref])).unwrap();
    anchor["journal_head_hash"] = json!(previous);
    let rewritten = git(repo, &["commit-tree", &tree, "-m", &anchor.to_string()]);
    git(repo, &["update-ref", &evidence_ref, &rewritten]);
}

fn git_with_stdin(repo: &Path, args: &[&str], input: &str) -> String {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// The `effect/started` entries of the run's one boxed site, from the
/// published journal: the plain word, never the adjective.
fn boundary_entries(events: &[EventEnvelope]) -> Vec<Value> {
    events
        .iter()
        .filter(|event| event.event_type == brokkr_core::envelope::EventType::EffectStarted)
        .filter_map(|event| event.payload.get("boundary").cloned())
        .collect()
}

#[test]
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
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

    let (run, gate) = Gate::delivering(side.path(), &repo, "delivery");
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
    // A run that boxes nothing is neither boxed nor unboxed (decision
    // 0046 ruling 3; design DD13): both lines carry no adjective at all.
    assert_plain_vouch(&log, &run, &rebased);

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
    let preflight = deliver_preflight(side.path(), &repo);
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

    let gate = Gate::local(&repo);
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

/// Decision 0046 ruling 3, through the real gate: a run whose boxed exec
/// gate stood under `harness` — declared by the realm, compiled and
/// started under that word, the pinned script run unboxed — is rendered
/// `unboxed` on the tier line and the vouch line, and on the docs tier
/// on the delivery line and the preflight's own line. The journal
/// carries the plain word; the adjective is the script's. The same
/// run's evidence, re-published with what this engine never writes — a
/// boxed manifest without `boundary`, an entry outside the vocabulary,
/// an entry without its tag — reads `boundary not recorded`, never
/// `unboxed` and never nothing (design DD13, DD14), and so does a
/// manifest carrying both `hands` and `boundary` whose journal holds no
/// entry yet (proposal D34).
#[test]
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn a_harness_judged_run_reads_unboxed_and_an_unrecorded_boundary_says_so() {
    let side = tempfile::tempdir().unwrap();
    let repo = init_repo(side.path());
    commit(&repo, "src/lib.txt", "one\n", "base code");
    commit(&repo, "README.md", "# app\n", "base readme");
    git(&repo, &["checkout", "-q", "-b", "slice"]);
    let judged = commit(&repo, "src/lib.txt", "one\ntwo\n", "the slice");
    let realms = write_realms(side.path(), &repo, "harness");

    let run = deliver_in(
        side.path(),
        &repo,
        "delivery",
        DELIVERY_POLICY,
        delivery_script(),
        true,
        Some(&realms),
    );

    // The record: the manifest pins the word per boxed site, and the
    // verify attempt's `effect/started` names it for the gate site.
    let events = published_journal(&repo, &run);
    let manifest = &events[0].payload["manifest"];
    assert_eq!(
        manifest["boundary"],
        json!({"verify": "harness"}),
        "{manifest}"
    );
    assert!(manifest["hands"].get("verify").is_some(), "{manifest}");
    let entries = boundary_entries(&events);
    assert_eq!(
        entries,
        vec![json!([{"member": null, "boundary": "harness", "gate": true}])]
    );
    let ndjson = git(
        &repo,
        &[
            "show",
            &format!("refs/heads/brokkr-runs/{run}:{run}.ndjson"),
        ],
    );
    assert!(
        !ndjson.contains("unboxed"),
        "the record carries the adjective"
    );

    let gate = Gate::local(&repo);
    let body = format!("## What changed\n\nBrokkr-Run: {run}\n");
    let (code, log) = gate.judge(&body, &judged, "");
    assert_eq!(code, 0, "{log}");
    assert_eq!(
        line_starting(&log, "delivered by brokkr: tier"),
        "delivered by brokkr: tier vouched · delta since the judgment: [] · unboxed"
    );
    assert_eq!(
        line_starting(&log, "delivered by brokkr: run"),
        format!("delivered by brokkr: run {run} vouches for {judged} · unboxed")
    );

    // The docs tier: the delivering run and the preflight each read on
    // their own line, each under its own boundary.
    let page = commit(&repo, "docs/page.md", "# page\n", "a page");
    let preflight = deliver_in(
        side.path(),
        &repo,
        "preflight",
        PREFLIGHT_POLICY,
        preflight_script(),
        true,
        Some(&realms),
    );
    let with_preflight = format!("{body}Brokkr-Preflight: {preflight}\n");
    let (code, log) = gate.judge(&with_preflight, &page, "");
    assert_eq!(code, 0, "{log}");
    assert_eq!(
        line_starting(&log, "delivered by brokkr: tier"),
        "delivered by brokkr: tier docs · delta since the judgment: [\"docs/page.md\"] · unboxed"
    );
    assert_eq!(
        line_starting(&log, "delivered by brokkr: run"),
        format!("delivered by brokkr: run {run} delivered the slice · unboxed")
    );
    assert_eq!(
        line_starting(&log, "delivered by brokkr: preflight"),
        format!("delivered by brokkr: preflight {preflight} judged {page} · unboxed")
    );

    // An old journal: the manifest declares hands and no boundary.
    let original = git(
        &repo,
        &["rev-parse", &format!("refs/heads/brokkr-runs/{run}")],
    );
    republish(&repo, &run, |events| {
        events[0].payload["manifest"]
            .as_object_mut()
            .unwrap()
            .remove("boundary")
            .expect("the manifest pinned a boundary");
    });
    let (code, log) = gate.judge(&body, &judged, "");
    assert_eq!(code, 0, "{log}");
    assert_eq!(
        line_starting(&log, "delivered by brokkr: tier"),
        "delivered by brokkr: tier vouched · delta since the judgment: [] · boundary not recorded"
    );
    assert_eq!(
        line_starting(&log, "delivered by brokkr: run"),
        format!("delivered by brokkr: run {run} vouches for {judged} · boundary not recorded")
    );

    // A malformed entry: a word outside the six, then a missing tag —
    // each read as not recorded, never as unboxed for the harness word
    // that still stands beside it.
    let started = events
        .iter()
        .position(|event| event.payload.get("boundary").is_some())
        .expect("one effect/started carries boundary");
    for malform in [
        (|entry: &mut Value| entry["boundary"] = json!("chroot")) as fn(&mut Value),
        |entry: &mut Value| {
            entry.as_object_mut().unwrap().remove("member");
        },
    ] {
        git(
            &repo,
            &[
                "update-ref",
                &format!("refs/heads/brokkr-runs/{run}"),
                &original,
            ],
        );
        republish(&repo, &run, |events| {
            let entries = events[started].payload["boundary"].as_array_mut().unwrap();
            let mut fresh = entries[0].clone();
            malform(&mut fresh);
            entries.insert(0, fresh);
        });
        let (code, log) = gate.judge(&body, &judged, "");
        assert_eq!(code, 0, "{log}");
        assert_eq!(
            line_starting(&log, "delivered by brokkr: tier"),
            "delivered by brokkr: tier vouched · delta since the judgment: [] · boundary not recorded"
        );
        assert_eq!(
            line_starting(&log, "delivered by brokkr: run"),
            format!("delivered by brokkr: run {run} vouches for {judged} · boundary not recorded")
        );
    }

    // A manifest that carries both `hands` and `boundary` but a journal
    // with no `effect/started.boundary` entry yet: unreachable for a
    // completed anchored run over any shipped bundle — each runs a hands
    // gate before `done` — but the view renders `no boundary recorded`
    // for it, and the script says the same rather than nothing (D34).
    git(
        &repo,
        &[
            "update-ref",
            &format!("refs/heads/brokkr-runs/{run}"),
            &original,
        ],
    );
    republish(&repo, &run, |events| {
        for event in events.iter_mut() {
            if event.event_type == brokkr_core::envelope::EventType::EffectStarted {
                event.payload.as_object_mut().unwrap().remove("boundary");
            }
        }
    });
    let (code, log) = gate.judge(&body, &judged, "");
    assert_eq!(code, 0, "{log}");
    assert_eq!(
        line_starting(&log, "delivered by brokkr: tier"),
        "delivered by brokkr: tier vouched · delta since the judgment: [] · boundary not recorded"
    );
    assert_eq!(
        line_starting(&log, "delivered by brokkr: run"),
        format!("delivered by brokkr: run {run} vouches for {judged} · boundary not recorded")
    );
}

/// The same bundle under `namespace`: the exec gate runs in decision
/// 0043's box, the journal names the word, and the gate renders no
/// adjective at all — a boxed run is the unmarked case.
#[test]
fn a_namespace_judged_run_carries_no_adjective() {
    if !can_create_namespace() {
        brokkr_protocol::hands::skip_boundary_proof(
            brokkr_protocol::hands::boundary_evidence_required(),
            "no namespace can be built here",
        );
        return;
    }
    let side = tempfile::tempdir().unwrap();
    let repo = init_repo(side.path());
    commit(&repo, "src/lib.txt", "one\n", "base code");
    git(&repo, &["checkout", "-q", "-b", "slice"]);
    let judged = commit(&repo, "src/lib.txt", "one\ntwo\n", "the slice");
    let realms = write_realms(side.path(), &repo, "namespace");
    let run = deliver_in(
        side.path(),
        &repo,
        "delivery",
        DELIVERY_POLICY,
        delivery_script(),
        true,
        Some(&realms),
    );
    let events = published_journal(&repo, &run);
    assert_eq!(
        events[0].payload["manifest"]["boundary"],
        json!({"verify": "namespace"})
    );
    assert_eq!(
        boundary_entries(&events),
        vec![json!([{"member": null, "boundary": "namespace", "gate": true}])]
    );

    let gate = Gate::local(&repo);
    let body = format!("## What changed\n\nBrokkr-Run: {run}\n");
    let log = gate.accepted(&body, &judged);
    assert_plain_vouch(&log, &run, &judged);
    assert!(!log.contains("unboxed"), "{log}");
    assert!(!log.contains("boundary not recorded"), "{log}");
}

#[test]
fn every_malformed_entry_record_refuses_the_vouch() {
    let side = tempfile::tempdir().unwrap();
    let repo = init_repo(side.path());
    commit(&repo, "README.md", "synthetic base\n", "base");
    git(&repo, &["checkout", "-qb", "slice"]);
    let head = commit(&repo, "src/slice.rs", "// synthetic\n", "slice");
    let (run, gate) = Gate::delivering(side.path(), &repo, "entry-gate");
    let body = format!("Brokkr-Run: {run}\n");
    for (_, raw) in crate::entry_records::invalid_records() {
        republish(&repo, &run, |events| {
            events[0].payload["entry"] = raw.clone();
        });
        let (code, log) = gate.judge(&body, &head, "");
        assert_eq!(code, 1, "{raw}: {log}");
        assert!(
            log.contains(&format!("run {run}: unreadable entry")),
            "{raw}: {log}"
        );
    }
}

fn assert_plain_vouch(log: &str, run: &str, head: &str) {
    assert_eq!(
        line_starting(log, "delivered by brokkr: tier"),
        "delivered by brokkr: tier vouched · delta since the judgment: []"
    );
    assert_eq!(
        line_starting(log, "delivered by brokkr: run"),
        format!("delivered by brokkr: run {run} vouches for {head}")
    );
}

#[test]
fn the_gate_and_the_typed_reader_accept_the_same_bounded_names() {
    let source =
        std::fs::read_to_string(workspace().join("scripts/delivered-by-brokkr.sh")).unwrap();
    let program = source
        .split("entry_suffix() {")
        .nth(1)
        .unwrap()
        .split("  jq -rs '")
        .nth(1)
        .unwrap()
        .split("' \"$work/")
        .next()
        .unwrap();
    let phases = [
        "verify".into(),
        "".into(),
        "v".repeat(64),
        "v".repeat(65),
        "-v".into(),
        "Verify".into(),
        "v v".into(),
        "verify\n".into(),
        "é".into(),
    ];
    let ids = [
        "-12345678".into(),
        format!("{}-abcdef01", "a".repeat(32)),
        format!("{}-abcdef01", "a".repeat(33)),
        "story-120-0a1b2c3d".into(),
        "a-1234567".into(),
        "a-ABCDEF01".into(),
        "A-0a1b2c3d".into(),
        "a-12345678\n".into(),
    ];
    let records = phases
        .into_iter()
        .map(|phase| {
            let valid = brokkr_core::entry::is_entry_name(&phase);
            (json!({"phase":phase,"continues":[]}), valid)
        })
        .chain(ids.into_iter().map(|id| {
            let valid = brokkr_core::entry::is_native_run_id(&id);
            (
                json!({"phase":"verify","continues":[{"run_id":id,"feature":"synthetic"}]}),
                valid,
            )
        }));
    for (entry, valid) in records {
        let input = json!({"type":"run/started","payload":{"entry":entry}}).to_string();
        let mut child = Command::new("jq")
            .args(["-rs", program])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        assert_eq!(child.wait().unwrap().success(), valid, "{input}");
    }
}

fn gate_classes(side: &Path, bundle: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let wrapper = side.join("synthetic-judge");
    std::fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nshift 4\nexec '{}' fake-driver \"$@\"\n",
            brokkr_bin()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::create_dir_all(side.join("adapters")).unwrap();
    let mut adapter: Value = serde_json::from_str(
        &std::fs::read_to_string(workspace().join("adapters/codex.json")).unwrap(),
    )
    .unwrap();
    adapter["provider"] = json!("judge");
    adapter["driver"] = json!(["{brokkr}", "driver", "judge", "--"]);
    adapter["binary"] = json!("judge");
    adapter["efforts"] = json!([]);
    adapter["effort_flag"] = json!("unsupported");
    adapter["models"] = json!({"judge":"judge-1"});
    adapter["judges"] = json!(["judge"]);
    let adapter = crate::fake_office::measured(adapter);
    std::fs::write(side.join("adapters/judge.json"), adapter.to_string()).unwrap();
    let path = bundle.join("bundle.json");
    let mut config: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    // The delivery's own script and state, beside its bundle.
    let dir = bundle.parent().unwrap();
    let fake = crate::fake_office::fake_driver(&dir.join("script.json"), &dir.join("state"));
    for phase in ["verify", "review", "ship"] {
        let mut command = vec![
            json!(wrapper),
            json!("driver"),
            json!("judge"),
            json!("--model"),
            json!("judge-1"),
        ];
        command.extend(fake.as_array().unwrap()[2..].iter().cloned());
        let results = config["seats"][phase]["results"].clone();
        config["seats"][phase] = json!({"role": "roles/role.md", "results": results,
                                         "driver": {"command": command}, "class": "gate"});
    }
    std::fs::write(path, config.to_string()).unwrap();
}

const CONTINUED_FEATURE: &str = "a\u{1b}[2J\u{1b}[H";

#[test]
fn an_entered_delivery_continues_two_runs_and_vouches_without_fetching_them() {
    let side = tempfile::tempdir().unwrap();
    let repo = init_repo(side.path());
    commit(&repo, "README.md", "synthetic base\n", "base");
    git(&repo, &["checkout", "-qb", "slice"]);
    let predecessors = stopped_predecessors(side.path(), &repo);
    let mut policy: Value = serde_json::from_str(DELIVERY_POLICY).unwrap();
    let fail = policy["rules"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["id"] == "VERIFY-FAIL")
        .unwrap();
    fail["next"] = json!("implement");
    fail["severity"] = json!("flagged");
    let mut script = delivery_script();
    script["seats"]["verify"] = json!([
        {"behavior":"succeed","result":{"result":"fail"}},
        {"behavior":"succeed","result":{"result":"pass"}}
    ]);

    let (a, b) = (&predecessors[0], &predecessors[1]);
    let mut vouched = None;
    for (name, phase, extra) in [
        (
            "c",
            "verify",
            vec!["--enter", "verify", "--continues", a, "--continues", b],
        ),
        ("review-entry", "review", vec!["--enter", "review"]),
    ] {
        let bundle = delivery_bundle(
            side.path(),
            name,
            &policy.to_string(),
            script.clone(),
            false,
        );
        gate_classes(side.path(), &bundle);
        let (code, stderr) = drive_delivery(side.path(), &repo, &bundle, name, &extra);
        assert_eq!(code, Some(0), "{stderr}");
        let run = started_run(&stderr);
        publish_anchor(&repo, &run);
        let events = published_journal(&repo, &run);
        assert_entered_journal(&events, phase, a, b);
        let head = git(&repo, &["rev-parse", "HEAD"]);
        let body = format!("Brokkr-Run: {run}\n");
        let log = Gate::local(&repo).accepted(&body, &head);
        let line = if phase == "verify" {
            assert_eq!(
                line_starting(&stderr, "entered at"),
                format!("entered at verify, continuing a[2J[H ({a}), b ({b})")
            );
            vouched = Some((body, run.clone()));
            format!("entered at verify, continuing {a}, {b}")
        } else {
            "entered at review, continuing no earlier run".into()
        };
        assert_eq!(
            line_starting(&log, "delivered by brokkr: tier"),
            "delivered by brokkr: tier vouched · delta since the judgment: []"
        );
        assert_eq!(
            line_starting(&log, "delivered by brokkr: run"),
            format!("delivered by brokkr: run {run} vouches for {head} · {line}")
        );
    }
    assert_unpublished_predecessors(&repo, a, b);
    let (body, run) = vouched.unwrap();
    let changed = commit(
        &repo,
        "src/after.rs",
        "// after judgment\n",
        "after judgment",
    );
    let (code, log) = Gate::local(&repo).judge(&body, &changed, "");
    assert_eq!(code, 1, "{log}");
    assert_eq!(
        line_starting(&log, "delivered by brokkr: tier"),
        "delivered by brokkr: tier code · delta since the judgment: [\"src/after.rs\"]"
    );
    git(&repo, &["checkout", "-q", &format!("{changed}~1")]);
    assert_entered_docs_delivery(side.path(), &repo, &body, &run, a, b);
}

fn assert_unpublished_predecessors(repo: &Path, a: &str, b: &str) {
    for predecessor in [a, b] {
        assert_eq!(
            git(
                repo,
                &[
                    "for-each-ref",
                    &format!("refs/heads/brokkr-runs/{predecessor}")
                ]
            ),
            ""
        );
    }
}

fn deliver_preflight(side: &Path, repo: &Path) -> String {
    deliver(
        side,
        repo,
        "preflight",
        PREFLIGHT_POLICY,
        preflight_script(),
    )
}

fn assert_entered_docs_delivery(side: &Path, repo: &Path, body: &str, run: &str, a: &str, b: &str) {
    let head = commit(
        repo,
        "docs/after.md",
        "# synthetic page\n",
        "docs after judgment",
    );
    let preflight = deliver_preflight(side, repo);
    let log = Gate::local(repo).accepted(&format!("{body}Brokkr-Preflight: {preflight}\n"), &head);
    assert_eq!(
        line_starting(&log, "delivered by brokkr: tier"),
        "delivered by brokkr: tier docs · delta since the judgment: [\"docs/after.md\"]"
    );
    assert_eq!(
        line_starting(&log, "delivered by brokkr: run"),
        format!("delivered by brokkr: run {run} delivered the slice · entered at verify, continuing {a}, {b}")
    );
    assert_eq!(
        line_starting(&log, "delivered by brokkr: preflight"),
        format!("delivered by brokkr: preflight {preflight} judged {head}")
    );
}

fn stopped_predecessors(side: &Path, repo: &Path) -> Vec<String> {
    let mut predecessors = Vec::new();
    for feature in ["a", "b"] {
        commit(
            repo,
            &format!("src/{feature}.rs"),
            "// synthetic slice\n",
            feature,
        );
        let mut script = delivery_script();
        script["seats"]["verify"][0]["result"]["result"] = json!("fail");
        let bundle = delivery_bundle(side, feature, DELIVERY_POLICY, script, false);
        let title = if feature == "a" {
            CONTINUED_FEATURE
        } else {
            feature
        };
        let (code, stderr) = drive_delivery(side, repo, &bundle, title, &[]);
        assert_eq!(code, Some(3), "{stderr}");
        let run = started_run(&stderr);
        assert_eq!(
            git(
                repo,
                &["for-each-ref", &format!("refs/heads/brokkr-runs/{run}")]
            ),
            ""
        );
        predecessors.push(run);
    }
    predecessors
}

fn assert_entered_journal(events: &[EventEnvelope], phase: &str, a: &str, b: &str) {
    let continues = if phase == "verify" {
        json!([{"run_id":a,"feature":CONTINUED_FEATURE},{"run_id":b,"feature":"b"}])
    } else {
        json!([])
    };
    assert_eq!(
        events[0].payload["entry"],
        json!({"phase":phase,"continues":continues})
    );
    let phases: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == brokkr_core::EventType::PhaseEntered)
        .map(|e| e.payload["phase"].as_str().unwrap())
        .collect();
    let expected = if phase == "verify" {
        vec![
            "verify",
            "implement",
            "verify",
            "review",
            "ship",
            "ship",
            "done",
        ]
    } else {
        vec!["review", "ship", "ship", "done"]
    };
    assert_eq!(phases, expected);
}
