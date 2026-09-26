//! #338's ratchets, driven through the real script: `quality/ratchet.sh`
//! holds the tree to the committed baselines under `quality/`, lets a number
//! only shrink, and passes a raised baseline only when the pull request
//! names the ruling that allowed it. Each test builds a scratch git
//! repository carrying the script, its shared definitions and the ruled
//! ceilings, so every verdict is the script's own. The checks that run a
//! measuring tool (cargo-crap, jscpd, cargo-public-api) are bound by the
//! removal controls in their commit; the judgements here need only git, jq
//! and a POSIX shell. Unix only: the ratchets are shell, because CI is.

#![cfg(unix)]

use std::path::Path;
use std::process::{Command, Output};

#[path = "support/workspace.rs"]
mod workspace_root;
use workspace_root::read;

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=ratchet",
            "-c",
            "user.email=ratchet@example.invalid",
        ])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn write(repo: &Path, path: &str, text: &str) {
    let path = repo.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn rust_lines(count: usize) -> String {
    "// line\n".repeat(count)
}

/// A scratch repository with the ratchet, its shared definitions, the
/// generator and the ruled ceilings, copied from this tree.
fn scratch() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for name in ["lib.sh", "ratchet.sh", "measure.sh", "ceilings.json"] {
        write(
            dir.path(),
            &format!("quality/{name}"),
            &read(&format!("quality/{name}")),
        );
    }
    git(dir.path(), &["init", "-q"]);
    dir
}

/// Run `quality/ratchet.sh`, with `PR_BODY` set only when a body is given.
fn ratchet(repo: &Path, args: &[&str], pr_body: Option<&str>) -> Output {
    let mut command = Command::new("bash");
    command
        .arg("quality/ratchet.sh")
        .args(args)
        .current_dir(repo);
    command.env_remove("PR_BODY");
    if let Some(body) = pr_body {
        command.env("PR_BODY", body);
    }
    command.output().unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_refused(output: &Output, lines: &[&str]) {
    let text = stderr(output);
    assert_eq!(output.status.code(), Some(1), "expected a refusal:\n{text}");
    for line in lines {
        assert!(text.contains(line), "missing {line:?} in:\n{text}");
    }
}

fn assert_holds(output: &Output, what: &str) {
    assert!(output.status.success(), "{}", stderr(output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(what), "{stdout}");
}

const FILE_LINES: &str = "# produced by quality/measure.sh with git ls-files and wc -l
# production
   900 crates/demo/src/big.rs
    10 crates/demo/src/small.rs
# test
    10 crates/demo/tests/t.rs
";

/// A grandfathered file may not grow past its baseline, any other file may
/// not grow past the ruled ceiling, and a baseline it cannot read is refused.
#[test]
fn the_file_ratchet_holds_each_file_to_its_baseline_or_ceiling() {
    let repo = scratch();
    let at = repo.path();
    write(at, "quality/file-lines.txt", FILE_LINES);
    write(at, "crates/demo/src/big.rs", &rust_lines(900));
    write(at, "crates/demo/src/small.rs", &rust_lines(10));
    write(at, "crates/demo/tests/t.rs", &rust_lines(10));
    git(at, &["add", "-A"]);
    assert_holds(&ratchet(at, &["files"], None), "ratchet: file size holds");

    write(at, "crates/demo/src/big.rs", &rust_lines(901));
    write(at, "crates/demo/src/small.rs", &rust_lines(801));
    write(at, "crates/demo/tests/new.rs", &rust_lines(2001));
    git(at, &["add", "-A"]);
    assert_refused(
        &ratchet(at, &["files"], None),
        &[
            "crates/demo/src/big.rs: 901 lines, over 900",
            "crates/demo/src/small.rs: 801 lines, over 800",
            "crates/demo/tests/new.rs: 2001 lines, over 2000",
            "ratchet refusal: file size: 3 finding(s)",
        ],
    );

    write(at, "crates/demo/src/big.rs", &rust_lines(850));
    write(at, "crates/demo/src/small.rs", &rust_lines(800));
    write(at, "crates/demo/tests/new.rs", &rust_lines(2000));
    git(at, &["add", "-A"]);
    assert_holds(&ratchet(at, &["files"], None), "ratchet: file size holds");

    write(
        at,
        "quality/file-lines.txt",
        &format!("{FILE_LINES}not a count\n"),
    );
    assert_refused(
        &ratchet(at, &["files"], None),
        &["quality/file-lines.txt:7 is not \"<count> <path>\""],
    );
}

/// An existing function may grow to the ceiling or its baseline, whichever
/// is higher, and a new one only to the ceiling. A report this check cannot
/// read as cyclomatic complexity is refused, and so is an empty one.
#[test]
fn the_complexity_judge_holds_functions_to_their_allowance() {
    let repo = scratch();
    let at = repo.path();
    git(at, &["commit", "-q", "--allow-empty", "-m", "base"]);
    let entry = |function: &str, crap: i64, baseline: Option<i64>, status: &str, coverage: i64| {
        let mut entry = serde_json::json!({
            "file": "./crates/demo/src/lib.rs", "function": function, "line": 1,
            "cyclomatic": crap, "coverage": coverage, "crap": crap, "status": status,
        });
        if let Some(baseline) = baseline {
            entry["baseline_crap"] = baseline.into();
        }
        entry
    };
    let judge = |report: serde_json::Value| {
        write(at, "report.json", &report.to_string());
        ratchet(at, &["crap-judge", "report.json"], None)
    };
    let matched = serde_json::json!({"source_only": {"count": 0}, "lcov_only": {"count": 0}});
    let report = |entries: Vec<serde_json::Value>| {
        judge(serde_json::json!({ "entries": entries, "diagnostics": matched }))
    };
    assert_holds(
        &report(vec![
            entry("small_grew", 14, Some(3), "regressed", 100),
            entry("big_shrank", 30, Some(40), "improved", 100),
            entry("fresh", 15, None, "new", 100),
        ]),
        "ratchet: cyclomatic complexity holds",
    );
    assert_refused(
        &report(vec![
            entry("big_grew", 41, Some(40), "regressed", 100),
            entry("fresh", 16, None, "new", 100),
            entry("half", 2, Some(2), "unchanged", 50),
            entry("odd", 1, Some(1), "moved", 100),
            entry("lost", 1, None, "unchanged", 100),
        ]),
        &[
            "lib.rs:1 big_grew: CC 41 over 40 (regressed)",
            "lib.rs:1 fresh: CC 16 over 15 (new)",
            "lib.rs:1 half: 50% covered, so CRAP is not cyclomatic complexity",
            "lib.rs:1 odd: status moved is not one this check reads",
            "lib.rs:1 lost: matched a baseline with no CRAP score",
            "ratchet refusal: cyclomatic complexity: 5 finding(s)",
        ],
    );
    assert_refused(&report(vec![]), &["no function was measured"]);
    let one = || vec![entry("fresh", 1, None, "new", 100)];
    assert_refused(
        &judge(serde_json::json!({ "entries": one() })),
        &["the report carries no source and LCOV match counts"],
    );
    let mismatch = serde_json::json!({"source_only": {"count": 0}, "lcov_only": {"count": 2}});
    assert_refused(
        &judge(serde_json::json!({ "entries": one(), "diagnostics": mismatch })),
        &["the scan and the LCOV disagree: 0 source file(s) with no coverage, 2 covered file(s) not scored"],
    );
}

fn crap_baseline(big: i64, small: i64, extra: &[(&str, i64)]) -> String {
    let mut entries = vec![
        serde_json::json!({"file": "./crates/demo/src/lib.rs", "function": "big", "line": 1, "cyclomatic": big}),
        serde_json::json!({"file": "./crates/demo/src/lib.rs", "function": "small", "line": 9, "cyclomatic": small}),
    ];
    for (function, cc) in extra {
        entries.push(serde_json::json!({"file": "./crates/demo/src/lib.rs", "function": function, "line": 20, "cyclomatic": cc}));
    }
    serde_json::json!({ "entries": entries }).to_string()
}

const VALUE: &str = "pub fn brokkr_core::f() -> serde_json::value::Value\n";

const SUPPRESSIONS: &str = "# by lint
# production
    2 expect clippy::too_many_lines
# test
    5 expect clippy::too_many_lines
";

/// Commit a set of baselines as the base a pull request is compared with.
fn baselined(at: &Path) {
    write(at, "quality/crap-baseline.json", &crap_baseline(20, 3, &[]));
    write(at, "quality/file-lines.txt", FILE_LINES);
    for scope in ["prod", "tests", "data"] {
        let fingerprints = r#"{"version":1,"fingerprints":{"aa":1}}"#;
        write(
            at,
            &format!("quality/jscpd-baseline-{scope}.json"),
            fingerprints,
        );
    }
    write(at, "quality/public-api/brokkr-core.txt", &VALUE.repeat(2));
    write(at, "quality/suppressions.txt", SUPPRESSIONS);
    write(at, "quality/duplicate-skips.txt", "syn@2.0.0\n");
    git(at, &["add", "-A"]);
    git(at, &["commit", "-q", "-m", "base"]);
}

/// Every way a baseline can be raised is named, and the set passes only
/// when the pull request carries a line of its own naming the ruling.
#[test]
fn a_raised_baseline_passes_only_with_a_named_ruling() {
    let repo = scratch();
    let at = repo.path();
    baselined(at);
    write(
        at,
        "quality/crap-baseline.json",
        &crap_baseline(21, 14, &[("fresh", 16), ("tiny", 4)]),
    );
    write(
        at,
        "quality/file-lines.txt",
        &FILE_LINES.replace(
            "   900 crates/demo/src/big.rs",
            "   950 crates/demo/src/big.rs",
        ),
    );
    write(
        at,
        "quality/jscpd-baseline-tests.json",
        r#"{"version":1,"fingerprints":{"aa":1,"bb":1}}"#,
    );
    write(at, "quality/public-api/brokkr-core.txt", &VALUE.repeat(3));
    std::fs::remove_file(at.join("quality/jscpd-baseline-data.json")).unwrap();
    write(
        at,
        "quality/suppressions.txt",
        &(SUPPRESSIONS.replace("    2 expect", "    3 expect") + "    1 allow  dead_code\n"),
    );
    write(at, "quality/duplicate-skips.txt", "syn@2.0.0\nsyn@3.0.0\n");
    let raised = [
        "crap-baseline.json: ./crates/demo/src/lib.rs big #0 at CC 21 (was 20)",
        "crap-baseline.json: ./crates/demo/src/lib.rs fresh #0 at CC 16 (was absent)",
        "file-lines.txt: crates/demo/src/big.rs at 950 lines (was 900)",
        "jscpd-baseline-tests.json: new clone bb",
        "public-api/brokkr-core.txt: 3 items name serde_json::Value (was 2)",
        "quality/jscpd-baseline-data.json: removed",
        "suppressions.txt: production expect clippy::too_many_lines at 3 (was 2)",
        "suppressions.txt: test allow dead_code at 1 (was 0)",
        "duplicate-skips.txt: new skip syn@3.0.0",
    ];
    let refused = ratchet(at, &["baselines", "HEAD"], None);
    assert_refused(&refused, &raised);
    let text = stderr(&refused);
    assert!(!text.contains("small") && !text.contains("tiny"), "{text}");
    assert_refused(
        &ratchet(
            at,
            &["baselines", "HEAD"],
            Some("Mentions Ruling: in passing only"),
        ),
        &raised,
    );
    let allowed = ratchet(
        at,
        &["baselines", "HEAD"],
        Some("## What changed\n\nRuling: #338\n"),
    );
    assert_holds(
        &allowed,
        "ratchet: raised baselines allowed by: Ruling: #338",
    );
}

/// A lowered number needs no ruling; a changed measuring rule does, and a
/// revision that does not resolve is refused.
#[test]
fn a_lowered_baseline_needs_no_ruling_but_a_changed_rule_does() {
    let repo = scratch();
    let at = repo.path();
    baselined(at);
    write(at, "quality/crap-baseline.json", &crap_baseline(19, 2, &[]));
    write(
        at,
        "quality/jscpd-baseline-prod.json",
        r#"{"version":1,"fingerprints":{}}"#,
    );
    write(at, "quality/public-api/brokkr-core.txt", VALUE);
    write(
        at,
        "quality/suppressions.txt",
        &SUPPRESSIONS.replace("    5 expect", "    4 expect"),
    );
    write(at, "quality/duplicate-skips.txt", "");
    assert_holds(
        &ratchet(at, &["baselines", "HEAD"], None),
        "ratchet: no baseline raised since HEAD",
    );

    let ceilings = std::fs::read_to_string(at.join("quality/ceilings.json")).unwrap();
    write(
        at,
        "quality/ceilings.json",
        &ceilings.replace("\"ccNewFunction\": 15", "\"ccNewFunction\": 16"),
    );
    assert_refused(
        &ratchet(at, &["baselines", "HEAD"], None),
        &["quality/ceilings.json: the measuring rules changed"],
    );
    assert_refused(
        &ratchet(at, &["baselines", "no-such-revision"], None),
        &["cannot resolve no-such-revision"],
    );
}

/// One job's body in `.github/workflows/ci.yml`: its lines up to the next
/// key at the jobs map's indentation.
fn ci_job(id: &str) -> String {
    let workflow = read(".github/workflows/ci.yml");
    let is_job = |line: &str| {
        line.starts_with("  ") && !line.starts_with("   ") && line.trim_end().ends_with(':')
    };
    let mut lines = workflow
        .lines()
        .skip_while(|line| *line != format!("  {id}:"));
    let head = lines
        .next()
        .unwrap_or_else(|| panic!("ci.yml has no {id} job"));
    let body: Vec<&str> = lines.take_while(|line| !is_job(line)).collect();
    format!("{head}\n{}\n", body.join("\n"))
}

/// Every ratchet runs in CI, in order: complexity and the public API in
/// the required coverage job, complexity only after the exact gate wrote its
/// LCOV, and the rest in the ratchets job, a raised baseline judged against
/// the pull request's base. Dropping a step fails here.
#[test]
fn ci_runs_every_ratchet() {
    let coverage = ci_job("coverage");
    let gate = coverage
        .find("bash scripts/coverage-exact.sh")
        .expect("the exact gate");
    let crap = coverage
        .find("run: quality/ratchet.sh crap")
        .expect("the complexity ratchet");
    assert!(
        gate < crap,
        "the complexity ratchet reads the gate's LCOV, so it runs after it"
    );
    assert!(
        coverage.contains("run: quality/ratchet.sh api"),
        "the public-API ratchet"
    );
    let ratchets = ci_job("ratchets");
    for step in [
        "uses: ./.github/actions/setup-jscpd",
        "run: quality/ratchet.sh files",
        "run: quality/ratchet.sh clones",
        "run: cargo shear --deny-warnings --locked",
        "quality/ratchet.sh baselines",
        "PR_BODY: ${{ github.event.pull_request.body }}",
    ] {
        assert!(
            ratchets.contains(step),
            "the ratchets job does not run {step:?}"
        );
    }
}
