//! Recipe library proof: listing over valid and broken recipes, add from
//! a local path and from a git URL (a local `file://` clone — no
//! network), cleanup of a non-compiling add, and a full delivery driven
//! through `--recipe`. Bundles reuse the machine-proof pattern: the
//! scripted fake driver over the real protocol.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

const POLICY: &str = r#"{
  "schema": "forge.phase-machine/v1",
  "phases": ["intake", "implement", "verify", "review", "ship", "done", "stop"],
  "initial": "intake",
  "terminal": ["done", "stop"],
  "shippable_from": ["review"],
  "rules": [
    {"id": "INTAKE-OK", "from": "intake", "result": "resolved", "next": "implement",
     "reason": "Task framed and recorded."},
    {"id": "IMPL-BROKEN-TWICE", "from": "implement", "result": "broken",
     "when": {"consecutive_failures_gte": 2}, "next": "stop", "severity": "hard",
     "reason": "Two consecutive broken implement runs; stop rather than thrash."},
    {"id": "IMPL-BROKEN-RETRY", "from": "implement", "result": "broken",
     "next": "implement", "reason": "First broken run; one re-run permitted."},
    {"id": "IMPL-BLOCKED", "from": "implement", "result": "blocked", "next": "stop",
     "severity": "hard", "reason": "Implementer blocked; report, never silently continue."},
    {"id": "IMPL-OK", "from": "implement", "result": "complete", "next": "verify",
     "reason": "Implementation complete and committed."},
    {"id": "VERIFY-FAIL", "from": "verify", "result": "fail", "next": "stop",
     "severity": "hard", "reason": "Verification failed; not shippable."},
    {"id": "VERIFY-PASS", "from": "verify", "result": "pass", "next": "review",
     "reason": "Suite green; reviewers read verified code."},
    {"id": "REVIEW-SECURITY-HOLD", "from": "review", "result": "security-hold",
     "next": "stop", "severity": "hard",
     "reason": "Unresolved security findings. NEVER ship."},
    {"id": "REVIEW-RESIDUAL-ABOVE-MEDIUM", "from": "review", "result": "residual",
     "when": {"max_residual_severity_above": "medium"}, "next": "stop",
     "severity": "hard", "reason": "Residual severity above medium; not shippable."},
    {"id": "REVIEW-RESIDUAL-SECURITY", "from": "review", "result": "residual",
     "when": {"has_security_residual": true}, "next": "stop", "severity": "hard",
     "reason": "Security residuals never take the tracked-debt path."},
    {"id": "REVIEW-RESIDUAL-OK", "from": "review", "result": "residual", "next": "ship",
     "severity": "flagged",
     "reason": "Non-security residuals at or below medium proceed as tracked debt."},
    {"id": "REVIEW-CLEAN-NO-FIXES", "from": "review", "result": "clean",
     "when": {"fixes_applied": false}, "next": "ship",
     "reason": "Clean with no code changed; verification evidence stands."},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "verify",
     "reason": "Clean but fixes applied; re-verify before shipping."},
    {"id": "SHIP-DRIFT", "from": "ship", "result": "ready",
     "when": {"drift_detected": true}, "next": "review", "severity": "flagged",
     "reason": "HEAD moved after review; re-arm a scoped review."},
    {"id": "SHIP-DIRTY", "from": "ship", "result": "ready",
     "when": {"dirty_worktrees": true}, "next": "stop", "severity": "hard",
     "reason": "Dirty tree at ship time is a defect."},
    {"id": "SHIP-READY", "from": "ship", "result": "ready", "next": "ship",
     "reason": "Gates passed and ledger written; confirm close-out and report shipped."},
    {"id": "SHIPPED-DRIFT", "from": "ship", "result": "shipped",
     "when": {"drift_detected": true}, "next": "review", "severity": "flagged",
     "reason": "HEAD moved between ready and close-out; re-arm a scoped review."},
    {"id": "SHIPPED-DIRTY", "from": "ship", "result": "shipped",
     "when": {"dirty_worktrees": true}, "next": "stop", "severity": "hard",
     "reason": "Dirty tree at close-out is a defect."},
    {"id": "SHIP-COMPLETE", "from": "ship", "result": "shipped", "next": "done",
     "reason": "Close-out confirmed: clean, reviewed, verified; done."}
  ]
}"#;

fn happy_script() -> Value {
    json!({"seats": {
        "intake": [{"behavior": "succeed", "result": {"result": "resolved"}}],
        "implement": [{"behavior": "succeed", "result": {"result": "complete"}}],
        "verify": [{"behavior": "succeed", "result": {"result": "pass"}}],
        "review:correctness": [{"behavior": "succeed",
            "result": {"result": "clean", "inputs": {"fixes_applied": false}}}],
        "review:security": [{"behavior": "succeed",
            "result": {"result": "clean", "inputs": {"fixes_applied": false}}}],
        "ship": [
            {"behavior": "succeed", "result": {"result": "ready"}},
            {"behavior": "succeed", "result": {"result": "shipped"}},
        ],
    }})
}

struct Ws {
    dir: tempfile::TempDir,
}

impl Ws {
    fn new() -> Ws {
        let ws = Ws {
            dir: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(ws.path().join("state")).unwrap();
        std::fs::write(
            ws.path().join("script.json"),
            serde_json::to_string(&happy_script()).unwrap(),
        )
        .unwrap();
        ws
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    /// A minimal compiling bundle at `at`, review as a two-member panel
    /// so the listing's `review[correctness+security]` rendering is
    /// exercised. Drivers are the scripted fake driver.
    fn write_recipe(&self, at: &Path) {
        std::fs::create_dir_all(at.join("roles")).unwrap();
        std::fs::write(at.join("policy.json"), POLICY).unwrap();
        std::fs::write(at.join("roles/role.md"), "# test role\n").unwrap();
        let driver = json!({"command": [
            brokkr_bin(), "fake-driver",
            "--script", self.path().join("script.json").to_string_lossy(),
            "--state", self.path().join("state").to_string_lossy(),
        ]});
        let seat = |results: Vec<&str>| -> Value {
            json!({"role": "roles/role.md", "results": results, "driver": driver})
        };
        let config = json!({
            "name": "recipe-proof",
            "policy": "policy.json",
            "seats": {
                "intake": seat(vec!["resolved"]),
                "implement": seat(vec!["complete", "broken", "blocked"]),
                "verify": seat(vec!["pass", "fail"]),
                "review": {
                    "results": ["clean", "residual", "security-hold"],
                    "aggregate": "review-panel",
                    "panel": {
                        "correctness": {"role": "roles/role.md", "driver": driver},
                        "security": {"role": "roles/role.md", "driver": driver},
                    },
                },
                "ship": seat(vec!["ready", "shipped"]),
            }
        });
        std::fs::write(
            at.join("bundle.json"),
            serde_json::to_string_pretty(&config).unwrap(),
        )
        .unwrap();
    }

    /// A recipe that extends `base` (a sibling DIRECTORY name) and
    /// states one difference: a different review panel. `marker` is the
    /// resolver-owned `override` block — omit it and the collision is a
    /// compile error, which is the point of decision 0017's merge rules.
    fn write_derived(&self, at: &Path, name: &str, base: &str, marker: bool) {
        std::fs::create_dir_all(at.join("roles")).unwrap();
        std::fs::write(at.join("roles/role.md"), "# derived role\n").unwrap();
        let driver = json!({"command": [
            brokkr_bin(), "fake-driver",
            "--script", self.path().join("script.json").to_string_lossy(),
            "--state", self.path().join("state").to_string_lossy(),
        ]});
        let mut config = json!({
            "name": name,
            "extends": base,
            "seats": {
                "review": {
                    "results": ["clean", "residual", "security-hold"],
                    "aggregate": "review-panel",
                    "panel": {
                        "adversarial": {"role": "roles/role.md", "driver": driver},
                        "security": {"role": "roles/role.md", "driver": driver},
                    },
                },
            },
        });
        if marker {
            config["override"] = json!({"seats": ["review"]});
        }
        std::fs::write(
            at.join("bundle.json"),
            serde_json::to_string_pretty(&config).unwrap(),
        )
        .unwrap();
    }

    /// A bundle directory that fails to compile: bundle.json without
    /// 'policy'.
    fn write_broken(&self, at: &Path) {
        std::fs::create_dir_all(at).unwrap();
        std::fs::write(at.join("bundle.json"), r#"{"name": "broken"}"#).unwrap();
    }

    fn brokkr(&self, args: &[&str]) -> (Option<i32>, String, String) {
        let out = Command::new(brokkr_bin())
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap();
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    fn recipes_dir(&self) -> PathBuf {
        self.path().join("recipes")
    }
}

fn git(cwd: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.email=forge@test",
            "-c",
            "user.name=forge",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn list_prints_valid_recipes_and_warns_on_broken_ones() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    ws.write_recipe(&rdir.join("good"));
    ws.write_broken(&rdir.join("broken"));

    // The digest the listing must abbreviate.
    let (code, stdout, stderr) =
        ws.brokkr(&["compile", "--bundle", rdir.join("good").to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let compiled: Value = serde_json::from_str(&stdout).unwrap();
    let digest = compiled["digest"].as_str().unwrap();

    let (code, stdout, stderr) = ws.brokkr(&["recipes", "list", "--dir", rdir.to_str().unwrap()]);
    assert_eq!(code, Some(0), "a broken recipe must not abort: {stderr}");

    let line = stdout
        .lines()
        .find(|l| l.starts_with("good\t"))
        .expect("listing line for the valid recipe");
    let cols: Vec<&str> = line.split('\t').collect();
    assert_eq!(cols.len(), 5, "name, digest, phases, seats, path: {line}");
    assert_eq!(
        cols[1],
        &digest[..12],
        "short digest is the manifest digest prefix"
    );
    assert!(cols[1].chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(cols[2], "7 phases");
    assert!(
        cols[3].contains("review[correctness+security]"),
        "seats: {}",
        cols[3]
    );
    assert!(cols[3].contains("implement"), "seats: {}", cols[3]);
    assert!(cols[4].ends_with("good"), "source path: {}", cols[4]);

    let warning = stdout
        .lines()
        .find(|l| l.starts_with("warning:") && l.contains("broken"))
        .expect("warning line for the broken recipe");
    assert!(
        warning.contains("missing 'policy'"),
        "warning names the error: {warning}"
    );

    // Built-ins don't exist under the temp CWD: warnings, not errors.
    assert!(stdout.contains("warning: self"), "stdout: {stdout}");
    assert!(stdout.contains("warning: verify"), "stdout: {stdout}");
}

#[test]
fn add_from_local_path_installs_and_lists() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    let src = ws.path().join("src-bundle");
    ws.write_recipe(&src);

    let (code, _, stderr) = ws.brokkr(&[
        "recipes",
        "add",
        src.to_str().unwrap(),
        "--name",
        "mine",
        "--dir",
        rdir.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(stderr.contains("added recipe 'mine'"), "stderr: {stderr}");
    assert!(rdir.join("mine/bundle.json").is_file());

    let (code, _, stderr) =
        ws.brokkr(&["compile", "--bundle", rdir.join("mine").to_str().unwrap()]);
    assert_eq!(code, Some(0), "installed copy compiles: {stderr}");

    let (code, stdout, _) = ws.brokkr(&["recipes", "list", "--dir", rdir.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert!(
        stdout.lines().any(|l| l.starts_with("mine\t")),
        "stdout: {stdout}"
    );
}

#[test]
fn add_from_git_url_clones_and_installs_without_network() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();

    // Fixture repo with the bundle in a subdirectory: exercises the
    // clone-root-without-bundle.json resolution. The `file://` scheme is
    // classified as a git source (see recipes::is_git_source).
    let repo = ws.path().join("fixture-repo");
    ws.write_recipe(&repo.join("bundle"));
    git(&repo, &["init"]);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-m", "fixture bundle"]);
    let url = format!("file://{}", repo.display());

    let (code, _, stderr) = ws.brokkr(&[
        "recipes",
        "add",
        &url,
        "--name",
        "cloned",
        "--dir",
        rdir.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(rdir.join("cloned/bundle.json").is_file());
    assert!(
        !rdir.join("cloned/.git").exists(),
        "recipes are plain files, not repos"
    );

    let (code, _, stderr) =
        ws.brokkr(&["compile", "--bundle", rdir.join("cloned").to_str().unwrap()]);
    assert_eq!(code, Some(0), "cloned copy compiles: {stderr}");
}

#[test]
fn add_of_a_non_compiling_bundle_cleans_up_and_fails() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    let src = ws.path().join("bad-bundle");
    ws.write_broken(&src);

    let (code, _, stderr) = ws.brokkr(&[
        "recipes",
        "add",
        src.to_str().unwrap(),
        "--name",
        "bad",
        "--dir",
        rdir.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains("missing 'policy'"),
        "prints the compile error: {stderr}"
    );
    assert!(!rdir.join("bad").exists(), "the rejected copy is removed");
}

#[test]
fn add_refuses_the_ext_transport_that_would_execute_a_command() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    // Ends with `.git` so it classifies as a git source; the ext
    // transport would run `sh -c` if git were allowed to use it.
    let marker = ws.path().join("pwned");
    let url = format!("ext::sh -c \"touch {}\" x.git", marker.display());

    let (code, _, _) = ws.brokkr(&[
        "recipes",
        "add",
        &url,
        "--name",
        "evil",
        "--dir",
        rdir.to_str().unwrap(),
    ]);
    assert_ne!(code, Some(0), "ext transport must be refused");
    assert!(!marker.exists(), "the ext command must never execute");
    assert!(!rdir.join("evil").exists());
}

#[cfg(unix)]
#[test]
fn add_refuses_symlinks_and_removes_the_partial_copy() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    let src = ws.path().join("sneaky");
    ws.write_recipe(&src);
    let secret = ws.path().join("secret.txt");
    std::fs::write(&secret, "not for the library").unwrap();
    std::os::unix::fs::symlink(&secret, src.join("zz-link")).unwrap();

    let (code, _, stderr) = ws.brokkr(&[
        "recipes",
        "add",
        src.to_str().unwrap(),
        "--name",
        "sneaky",
        "--dir",
        rdir.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(stderr.contains("symlink"), "names the refusal: {stderr}");
    assert!(!rdir.join("sneaky").exists(), "the partial copy is removed");
}

#[test]
fn run_recipe_completes_a_delivery_and_arg_group_holds() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    let src = ws.path().join("src-bundle");
    ws.write_recipe(&src);
    let (code, _, stderr) = ws.brokkr(&[
        "recipes",
        "add",
        src.to_str().unwrap(),
        "--name",
        "good",
        "--dir",
        rdir.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "stderr: {stderr}");

    let db = ws.path().join("forge.db");
    let (code, stdout, stderr) = ws.brokkr(&[
        "run",
        "--recipe",
        "good",
        "--recipes-dir",
        rdir.to_str().unwrap(),
        "--feature",
        "recipe feature",
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let summary: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(summary["status"], "completed");
    assert_eq!(summary["phase"], "done");

    // Exactly one of --bundle/--recipe: both is a usage error...
    let (code, _, stderr) = ws.brokkr(&[
        "run",
        "--bundle",
        src.to_str().unwrap(),
        "--recipe",
        "good",
        "--feature",
        "f",
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_ne!(code, Some(0));
    assert!(stderr.contains("cannot be used with"), "stderr: {stderr}");

    // ...and neither is too.
    let (code, _, stderr) = ws.brokkr(&["run", "--feature", "f", "--db", db.to_str().unwrap()]);
    assert_ne!(code, Some(0));
    assert!(stderr.contains("required"), "stderr: {stderr}");
}

#[test]
fn compile_and_show_print_the_resolved_result_and_its_provenance() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    ws.write_recipe(&rdir.join("good"));
    ws.write_derived(&rdir.join("paranoid"), "paranoid", "good", true);

    // A bundle that composed nothing prints exactly what it always did:
    // no `composed_from` member at all.
    let (code, plain, stderr) =
        ws.brokkr(&["compile", "--bundle", rdir.join("good").to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let base: Value = serde_json::from_str(&plain).unwrap();
    assert!(base.get("composed_from").is_none(), "{plain}");

    // A composed one prints the RESOLVED result and its provenance.
    let (code, composed, stderr) = ws.brokkr(&[
        "compile",
        "--bundle",
        rdir.join("paranoid").to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let view: Value = serde_json::from_str(&composed).unwrap();
    assert_eq!(view["bundle"], "paranoid");
    let seats: Vec<&str> = view["seats"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert_eq!(
        seats,
        vec!["implement", "intake", "review", "ship", "verify"],
        "the resolved bundle carries every inherited seat"
    );
    let chain = view["composed_from"].as_array().unwrap();
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0]["recipe"], "recipe-proof");
    assert_eq!(chain[0]["digest"], base["digest"]);
    assert!(chain[0]["dir"].as_str().unwrap().ends_with("good"));
    assert_eq!(
        view["manifest"]["files"]["@compose/0000/recipe-proof@good"], base["digest"],
        "the chain pins the run through the digested manifest, naming BOTH the \
         declared name and the library directory it was extended by — recording \
         only one lets a directory answer to a name it does not declare"
    );

    // `recipes show` is the same renderer, reached by name, for both.
    let (code, shown, stderr) = ws.brokkr(&[
        "recipes",
        "show",
        "paranoid",
        "--dir",
        rdir.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(shown, composed);
    let (code, shown, _) = ws.brokkr(&["recipes", "show", "good", "--dir", rdir.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert_eq!(shown, plain);

    let (code, _, stderr) =
        ws.brokkr(&["recipes", "show", "absent", "--dir", rdir.to_str().unwrap()]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("recipe 'absent' not found"), "{stderr}");
}

#[test]
fn a_merge_conflict_names_the_file_and_the_conflicting_key() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    ws.write_recipe(&rdir.join("good"));
    ws.write_derived(&rdir.join("clash"), "clash", "good", false);

    let (code, _, stderr) =
        ws.brokkr(&["compile", "--bundle", rdir.join("clash").to_str().unwrap()]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("redefines seat 'review'"), "{stderr}");
    // Windows spells the same path with backslashes; the assertion is
    // about which files are named, not about the separator.
    let named = stderr.replace('\\', "/");
    assert!(named.contains("clash/bundle.json"), "{stderr}");
    assert!(named.contains("good/bundle.json"), "{stderr}");
    assert!(stderr.contains("override.seats"), "{stderr}");
}

#[test]
fn list_warns_when_a_derived_recipes_base_is_missing() {
    let ws = Ws::new();
    let rdir = ws.recipes_dir();
    ws.write_recipe(&rdir.join("good"));
    let orphan = rdir.join("orphan");
    std::fs::create_dir_all(&orphan).unwrap();
    std::fs::write(
        orphan.join("bundle.json"),
        json!({"name": "orphan", "extends": "absent"}).to_string(),
    )
    .unwrap();

    let (code, stdout, stderr) = ws.brokkr(&["recipes", "list", "--dir", rdir.to_str().unwrap()]);
    assert_eq!(code, Some(0), "a missing base must not abort: {stderr}");
    assert!(
        stdout.lines().any(|l| l.starts_with("good\t")),
        "everything else still lists: {stdout}"
    );
    let warning = stdout
        .lines()
        .find(|l| l.starts_with("warning:") && l.contains("orphan"))
        .expect("warning line for the orphan");
    assert!(warning.contains("extends 'absent'"), "{warning}");
    assert!(warning.contains("is not a recipe in"), "{warning}");
}
