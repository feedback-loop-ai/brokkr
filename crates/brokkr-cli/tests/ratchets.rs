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
/// generator, the ruled ceilings and the exact gate (whose test vocabulary
/// `lib.sh` reads), copied from this tree.
fn scratch() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for name in ["lib.sh", "ratchet.sh", "measure.sh", "ceilings.json"] {
        write(
            dir.path(),
            &format!("quality/{name}"),
            &read(&format!("quality/{name}")),
        );
    }
    let gate = "scripts/coverage-exact.sh";
    write(dir.path(), gate, &read(gate));
    git(dir.path(), &["init", "-q"]);
    dir
}

/// Run `quality/ratchet.sh`, with `PR_BODY` set only when a body is given.
fn ratchet(repo: &Path, args: &[&str], pr_body: Option<&str>) -> Output {
    ratchet_on(repo, args, pr_body, None)
}

/// Run `quality/ratchet.sh` with a directory of stub tools first on `PATH`
/// and the stub's report mode in its environment, when given.
fn ratchet_on(
    repo: &Path,
    args: &[&str],
    pr_body: Option<&str>,
    stubs: Option<(&Path, &str)>,
) -> Output {
    let mut command = Command::new("bash");
    if let Some((stubs, report)) = stubs {
        let path = std::env::var("PATH").unwrap_or_default();
        command.env("PATH", format!("{}:{path}", stubs.display()));
        command.env("JSCPD_STUB_REPORT", report);
    }
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

    let misfiled = FILE_LINES
        .replace("   900 crates/demo/src/big.rs\n", "")
        .replace("# test\n", "# test\n   900 crates/demo/src/big.rs\n");
    write(at, "quality/file-lines.txt", &misfiled);
    assert_refused(
        &ratchet(at, &["files"], None),
        &["quality/file-lines.txt:5: crates/demo/src/big.rs is production code, listed under # test"],
    );
    write(at, "quality/file-lines.txt", "# production\n# test\n");
    assert_refused(
        &ratchet(at, &["files"], None),
        &["quality/file-lines.txt: no baseline entry parsed"],
    );
}

/// A stub jscpd, first on PATH, that exits 0 having written the report
/// `JSCPD_STUB_REPORT` names: nothing (`empty`), or a well-formed report
/// of a scan that read no file (`nothing-read`).
const JSCPD_STUB: &str = r#"#!/bin/sh
while [ $# -gt 0 ]; do
  if [ "$1" = --output ]; then
    mkdir -p "$2"
    case "$JSCPD_STUB_REPORT" in
      empty) : > "$2/jscpd-report.json" ;;
      nothing-read) echo '{"duplicates":[],"statistics":{"total":{"sources":0}}}' > "$2/jscpd-report.json" ;;
    esac
  fi
  shift
done
exit 0
"#;

/// A clone scan that measured nothing passes nothing: a report left empty,
/// and a well-formed report of a scan that read no file, are each refused
/// though the tool exited 0.
#[test]
fn the_clone_ratchet_refuses_a_scan_that_measured_nothing() {
    let repo = scratch();
    let at = repo.path();
    for scope in ["prod", "tests", "data"] {
        let path = format!("quality/jscpd-baseline-{scope}.json");
        write(at, &path, r#"{"version":1,"fingerprints":{}}"#);
    }
    write(at, "stubs/jscpd", JSCPD_STUB);
    let stub = at.join("stubs/jscpd");
    let mode = std::os::unix::fs::PermissionsExt::from_mode(0o755);
    std::fs::set_permissions(&stub, mode).unwrap();
    let stubs = at.join("stubs");
    for (report, refusal) in [
        ("empty", "jscpd wrote an empty report for prod"),
        ("nothing-read", "jscpd read no file for prod"),
    ] {
        let output = ratchet_on(at, &["clones"], None, Some((&stubs, report)));
        assert_refused(&output, &[refusal]);
    }
}

/// A tree with no test file is measured, not refused for its empty section.
#[test]
fn the_file_ratchet_reads_a_tree_with_no_test_file() {
    let repo = scratch();
    let at = repo.path();
    write(
        at,
        "quality/file-lines.txt",
        "# production\n    10 crates/demo/src/small.rs\n# test\n",
    );
    write(at, "crates/demo/src/small.rs", &rust_lines(10));
    git(at, &["add", "-A"]);
    assert_holds(&ratchet(at, &["files"], None), "ratchet: file size holds");
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
    write(at, "empty.json", "");
    assert_refused(
        &ratchet(at, &["crap-judge", "empty.json"], None),
        &["the cargo-crap report at empty.json is empty"],
    );
    write(at, "list.json", "[]");
    assert_refused(
        &ratchet(at, &["crap-judge", "list.json"], None),
        &["the cargo-crap report at list.json is not a JSON object"],
    );
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

const TOO_MANY_LINES: &str = "# produced by quality/measure.sh
# production
 120 crates/demo/src/lib.rs:1 big
# test
 150 crates/demo/tests/t.rs:3 long_test
";

const MISS: &str = "crates/demo/src/lib.rs:1:1: replace f with ()\n";

const VIEW_API: &str = "# produced by quality/measure.sh
pub fn brokkr_view::a()
pub fn brokkr_view::b()
";

const SUPPRESSIONS: &str = "# by lint
# production
    2 expect clippy::too_many_lines
# test
    5 expect clippy::too_many_lines
";

/// A budget file (#342): ceilings its own gate holds the tree to.
const BUDGETS: &str = r#"{"budgets":{"a/x":10,"a/y":20}}"#;

/// Commit a set of baselines as the base a pull request is compared with.
fn baselined(at: &Path) {
    write(at, "quality/crap-baseline.json", &crap_baseline(20, 3, &[]));
    write(at, "quality/file-lines.txt", FILE_LINES);
    for scope in ["prod", "tests", "data"] {
        let fingerprints = r#"{"version":1,"fingerprints":{"aa":1,"cc":1}}"#;
        write(
            at,
            &format!("quality/jscpd-baseline-{scope}.json"),
            fingerprints,
        );
    }
    write(at, "quality/public-api/brokkr-core.txt", &VALUE.repeat(2));
    write(at, "quality/public-api/brokkr-view.txt", VIEW_API);
    write(at, "quality/too-many-lines.txt", TOO_MANY_LINES);
    write(at, "quality/suppressions.txt", SUPPRESSIONS);
    write(at, "quality/duplicate-skips.txt", "syn@1.0.0\nsyn@2.0.0\n");
    write(at, "quality/mutants/brokkr-core.missed.txt", MISS);
    write(at, "quality/mutants/brokkr-view.missed.txt", "");
    write(at, "quality/prompt-bytes.json", BUDGETS);
    git(at, &["add", "-A"]);
    git(at, &["commit", "-q", "-m", "base"]);
}

/// Raise every baseline once, and return the line each raise is named by.
fn raise_everything(at: &Path) -> Vec<&'static str> {
    let crap = crap_baseline(21, 14, &[("fresh", 16), ("tiny", 4)]);
    write(at, "quality/crap-baseline.json", &crap);
    let big = FILE_LINES.replace(
        "   900 crates/demo/src/big.rs",
        "   950 crates/demo/src/big.rs",
    );
    write(at, "quality/file-lines.txt", &big);
    let long = TOO_MANY_LINES.replace(
        " 120 crates/demo/src/lib.rs:1 big",
        " 125 crates/demo/src/lib.rs:1 big",
    ) + " 101 crates/demo/src/lib.rs:40 fresh\n";
    write(at, "quality/too-many-lines.txt", &long);
    let clones = r#"{"version":1,"fingerprints":{"aa":1,"bb":1}}"#;
    write(at, "quality/jscpd-baseline-tests.json", clones);
    write(at, "quality/public-api/brokkr-core.txt", &VALUE.repeat(3));
    write(
        at,
        "quality/public-api/brokkr-view.txt",
        &(VIEW_API.to_owned() + "pub fn brokkr_view::c()\n"),
    );
    write(
        at,
        "quality/public-api/brokkr-new.txt",
        "pub fn brokkr_new::c()\n",
    );
    std::fs::remove_file(at.join("quality/jscpd-baseline-data.json")).unwrap();
    let suppressions =
        SUPPRESSIONS.replace("    2 expect", "    3 expect") + "    1 allow  dead_code\n";
    write(at, "quality/suppressions.txt", &suppressions);
    write(
        at,
        "quality/duplicate-skips.txt",
        "syn@1.0.0\nsyn@2.0.0\nsyn@3.0.0\n",
    );
    let misses = MISS.to_owned() + "crates/demo/src/lib.rs:9:1: replace g with ()\n";
    write(at, "quality/mutants/brokkr-core.missed.txt", &misses);
    let budgets = r#"{"budgets":{"a/x":11,"a/y":20,"a/z":1}}"#;
    write(at, "quality/prompt-bytes.json", budgets);
    write(at, "quality/heap-bytes.json", r#"{"budgets":{"k":5}}"#);
    vec![
        "crap-baseline.json: ./crates/demo/src/lib.rs big #0 at CC 21 (was 20)",
        "crap-baseline.json: ./crates/demo/src/lib.rs fresh #0 at CC 16 (was absent)",
        "file-lines.txt: crates/demo/src/big.rs at 950 lines (was 900)",
        "too-many-lines.txt: crates/demo/src/lib.rs big #0 at 125 lines (was 120)",
        "too-many-lines.txt: crates/demo/src/lib.rs fresh #0 at 101 lines (was absent)",
        "jscpd-baseline-tests.json: new clone bb",
        "public-api/brokkr-core.txt: 3 public items (was 2)",
        "public-api/brokkr-view.txt: 3 public items (was 2)",
        "public-api/brokkr-new.txt: 1 public items (was absent)",
        "public-api/brokkr-core.txt: 3 items name serde_json::Value (was 2)",
        "quality/jscpd-baseline-data.json: removed",
        "suppressions.txt: production expect clippy::too_many_lines at 3 (was 2)",
        "suppressions.txt: test allow dead_code at 1 (was 0)",
        "duplicate-skips.txt: new skip syn@3.0.0",
        "quality/mutants/brokkr-core.missed.txt: new miss crates/demo/src/lib.rs:9:1: replace g with ()",
        "prompt-bytes.json: a/x at 11 (was 10)",
        "prompt-bytes.json: a/z at 1 (was absent)",
        "heap-bytes.json: k at 5 (was absent)",
    ]
}

/// Every way a baseline can be raised is named, and the set passes only
/// when the pull request carries a line of its own naming the ruling.
#[test]
fn a_raised_baseline_passes_only_with_a_named_ruling() {
    let repo = scratch();
    let at = repo.path();
    baselined(at);
    let raised = raise_everything(at);
    let refused = ratchet(at, &["baselines", "HEAD"], None);
    assert_refused(&refused, &raised);
    let text = stderr(&refused);
    assert!(!text.contains("small") && !text.contains("tiny"), "{text}");
    let passing = Some("Mentions Ruling: in passing only");
    assert_refused(&ratchet(at, &["baselines", "HEAD"], passing), &raised);
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

/// A web form's CRLF body is read with its carriage returns dropped: a
/// Ruling line that names nothing is refused, and one that names a ruling
/// passes.
#[test]
fn the_ruling_line_must_name_something_whatever_its_line_ends() {
    let repo = scratch();
    let at = repo.path();
    baselined(at);
    let raised = raise_everything(at);
    let blank = Some("## What changed\r\n\r\nRuling: \r\n");
    assert_refused(&ratchet(at, &["baselines", "HEAD"], blank), &raised);
    let named = Some("## What changed\r\n\r\nRuling: #338\r\n");
    assert_holds(
        &ratchet(at, &["baselines", "HEAD"], named),
        "allowed by: Ruling: #338",
    );
}

/// A baseline this check cannot read is refused, and no ruling passes it:
/// a listing with no entry, and a file listed under the section its path
/// does not belong to.
#[test]
fn a_baseline_this_check_cannot_read_is_refused_under_any_ruling() {
    let repo = scratch();
    let at = repo.path();
    baselined(at);
    let ruled = Some("Ruling: #338\n");
    write(at, "quality/file-lines.txt", "# production\n# test\n");
    assert_refused(
        &ratchet(at, &["baselines", "HEAD"], ruled),
        &[
            "file-lines.txt: no baseline entry parsed",
            "no ruling passes it",
        ],
    );
    let misfiled = FILE_LINES
        .replace("   900 crates/demo/src/big.rs\n", "")
        .replace("# test\n", "# test\n  1900 crates/demo/src/big.rs\n");
    write(at, "quality/file-lines.txt", &misfiled);
    assert_refused(
        &ratchet(at, &["baselines", "HEAD"], ruled),
        &["file-lines.txt:5: crates/demo/src/big.rs is production code, listed under # test"],
    );
    write(at, "quality/file-lines.txt", FILE_LINES);
    write(
        at,
        "quality/too-many-lines.txt",
        "# production\nnot an entry\n",
    );
    assert_refused(
        &ratchet(at, &["baselines", "HEAD"], ruled),
        &["too-many-lines.txt:2 is not \"<lines> <path>:<line> <name>\""],
    );
    write(at, "quality/too-many-lines.txt", TOO_MANY_LINES);
    write(at, "quality/crap-baseline.json", "");
    assert_refused(
        &ratchet(at, &["baselines", "HEAD"], ruled),
        &["quality/crap-baseline.json: empty at HEAD or here"],
    );
    unreadable_shapes_are_refused(at, ruled);
}

/// Each JSON and text baseline shape this check cannot read, planted one
/// at a time on a baselined repository, is refused under a Ruling line.
fn unreadable_shapes_are_refused(at: &Path, ruled: Option<&str>) {
    let no_cc = crap_baseline(21, 3, &[]).replace(r#""cyclomatic":21"#, r#""complexity":21"#);
    let not_counts = r#"{"version":1,"fingerprints":{"aa":"one"}}"#;
    let cases: [(&str, &str, &str); 8] = [
        (
            "quality/crap-baseline.json",
            &no_cc,
            "without file, function, line and cyclomatic (here)",
        ),
        (
            "quality/jscpd-baseline-prod.json",
            not_counts,
            "no fingerprints object of counts (here)",
        ),
        (
            "quality/public-api/brokkr-view.txt",
            "# produced by quality/measure.sh\n",
            "brokkr-view.txt: no public item parsed (here)",
        ),
        (
            "quality/public-api/brokkr-view.txt",
            "",
            "brokkr-view.txt: no public item parsed (here)",
        ),
        (
            "quality/public-api/brokkr-core.txt",
            "# produced by quality/measure.sh\n",
            "brokkr-core.txt: no public item parsed (here)",
        ),
        (
            "quality/suppressions.txt",
            "# production\n    two expect clippy::x\n",
            "suppressions.txt:2 is not \"<count> <expect|allow> <lint>\" (here)",
        ),
        (
            "quality/duplicate-skips.txt",
            "syn 2.0.0\n",
            "duplicate-skips.txt:1 is not \"<crate>@<version>\" (here)",
        ),
        (
            "quality/prompt-bytes.json",
            r#"{"budgets":{"a/x":10.5,"a/y":20}}"#,
            "prompt-bytes.json: no budgets object of whole numbers (here)",
        ),
    ];
    for (path, text, refusal) in cases {
        let committed = read_at(at, path);
        write(at, path, text);
        assert_refused(
            &ratchet(at, &["baselines", "HEAD"], ruled),
            &[refusal, "no ruling passes it"],
        );
        write(at, path, &committed);
    }
}

fn read_at(repo: &Path, path: &str) -> String {
    std::fs::read_to_string(repo.join(path)).unwrap()
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
        r#"{"version":1,"fingerprints":{"aa":1}}"#,
    );
    write(at, "quality/public-api/brokkr-core.txt", VALUE);
    let fewer = VIEW_API.replace("pub fn brokkr_view::b()\n", "");
    write(at, "quality/public-api/brokkr-view.txt", &fewer);
    let shorter = TOO_MANY_LINES.replace(
        " 120 crates/demo/src/lib.rs:1 big",
        " 110 crates/demo/src/lib.rs:1 big",
    );
    write(at, "quality/too-many-lines.txt", &shorter);
    write(
        at,
        "quality/suppressions.txt",
        &SUPPRESSIONS.replace("    5 expect", "    4 expect"),
    );
    write(at, "quality/duplicate-skips.txt", "syn@2.0.0\n");
    write(at, "quality/prompt-bytes.json", r#"{"budgets":{"a/x":9}}"#);
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

/// Every listing `baselines` reads, enumerated from the script's own table
/// (`ratchet.sh listings`), is planted with a head that parses to zero
/// entries: JSON as `{}`, text as its comment lines only. Each is refused
/// under a Ruling line wherever its base had entries; a listing empty at the
/// base (brokkr-view's allow-list here) stays allowed. Dropping the shared
/// guard turns every row red.
#[test]
fn a_listing_that_reads_zero_entries_is_refused_under_any_ruling() {
    let repo = scratch();
    let at = repo.path();
    baselined(at);
    let listings = stdout(&ratchet(at, &["listings"], None));
    let ruled = Some("Ruling: #338\n");
    let mut counters = std::collections::BTreeSet::new();
    let mut unrefused = Vec::new();
    for row in listings.lines() {
        let fields: Vec<&str> = row.split_whitespace().collect();
        assert_ne!(fields.get(1), Some(&"unknown"), "{row}");
        let [path, "listing", counter, _] = fields[..] else {
            continue;
        };
        counters.insert(counter.to_owned());
        let committed = read_at(at, path);
        write(at, path, &zero_entries(path, &committed));
        let output = ratchet(at, &["baselines", "HEAD"], ruled);
        write(at, path, &committed);
        let text = stderr(&output);
        let held = output.status.code() == Some(1)
            && text.contains(&format!("{path}: no entry parsed here, where HEAD had"))
            && text.contains("no ruling passes it");
        // A listing empty at the base may stay empty; every other row is held
        // by the shared guard, by its own message.
        let as_ruled = if committed.is_empty() {
            output.status.success()
        } else {
            held
        };
        if !as_ruled {
            unrefused.push(format!("{path}: {text}"));
        }
    }
    assert!(
        unrefused.is_empty(),
        "rows not held by the shared guard:\n{}",
        unrefused.join("\n")
    );
    let table = stdout(&ratchet(at, &["table"], None));
    let every: std::collections::BTreeSet<String> = table
        .lines()
        .filter_map(|row| match row.split_whitespace().collect::<Vec<_>>()[..] {
            [_, "listing", counter, _] => Some(counter.to_owned()),
            _ => None,
        })
        .collect();
    assert_eq!(
        counters, every,
        "the fixture plants a file for every listing row"
    );
    write(at, "quality/new-baseline.json", "{}");
    assert_refused(
        &ratchet(at, &["baselines", "HEAD"], ruled),
        &[
            "quality/new-baseline.json: not in ratchet.sh's table of baselines",
            "no ruling passes it",
        ],
    );
}

/// A head that parses to zero entries: JSON as an object with nothing in it,
/// text as its comment lines only.
fn zero_entries(path: &str, committed: &str) -> String {
    if path.ends_with(".json") {
        return "{}".to_owned();
    }
    committed
        .lines()
        .filter(|line| line.starts_with('#'))
        .map(|line| format!("{line}\n"))
        .collect()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
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
