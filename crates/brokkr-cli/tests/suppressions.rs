//! Every lint suppression is counted, and none is added without a ruling
//! (issue #337, decision 0071 ruling 4).
//!
//! Today's offenders against the ceilings carry `#[expect(.., reason)]`,
//! which `unfulfilled_lint_expectations` forces out as each is fixed. This
//! test holds the rest of the ratchet: `quality/suppressions.txt` records
//! how many `#[expect]` and `#[allow]` attributes name each lint, and the
//! tree must match it exactly, so a count moves only by a reviewed edit of
//! that file. On a pull request, CI also runs the ignored
//! [`an_added_suppression_of_a_ratcheted_lint_names_a_ruling`] against the
//! base commit: a new suppression of a ratcheted lint in production code
//! must name a ruling (an issue `#N` or a `decision NNNN`) in its reason.
//!
//! Attributes are read by a lexer that skips comments and every string and
//! char literal, so `#[expect(` inside a string is not an attribute, and a
//! multi-line attribute, a `cfg_attr` and an inner `#![expect]` all count.
//! Text it cannot read is refused rather than skipped.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

#[path = "support/test_paths.rs"]
mod test_paths;
#[path = "support/workspace.rs"]
mod workspace_root;

use test_paths::is_gate_test_path as is_test_path;
use workspace_root::{read, workspace};

const BASELINE: &str = "quality/suppressions.txt";

/// The lints #337 ratchets: suppressing one in production needs a ruling.
const RATCHETED: &[&str] = &[
    "clippy::too_many_lines",
    "clippy::excessive_nesting",
    "clippy::too_many_arguments",
    "clippy::struct_excessive_bools",
    "clippy::fn_params_excessive_bools",
    "clippy::allow_attributes",
    "clippy::allow_attributes_without_reason",
    "clippy::dbg_macro",
    "clippy::todo",
    "unreachable_pub",
    "unfulfilled_lint_expectations",
];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Suppression {
    level: &'static str,
    lints: Vec<String>,
    reason: Option<String>,
    predicate: Option<String>,
}

/// Advances past one string or char literal starting at `at`, or returns
/// `at` unchanged when there is none (a lifetime is not a literal).
fn skip_literal(s: &[char], at: usize) -> Result<usize, String> {
    if at >= s.len() {
        return Ok(at);
    }
    let mut i = at;
    if s[i] == 'b' && i + 1 < s.len() && matches!(s[i + 1], '"' | '\'' | 'r') {
        i += 1;
    }
    if s[i] == 'r' && i + 1 < s.len() && matches!(s[i + 1], '"' | '#') {
        let hashes = s[i + 1..].iter().take_while(|&&c| c == '#').count();
        let open = i + 1 + hashes;
        if s.get(open) != Some(&'"') {
            return Ok(at);
        }
        let close: String = std::iter::once('"')
            .chain("#".repeat(hashes).chars())
            .collect();
        let rest: String = s[open + 1..].iter().collect();
        let end = rest.find(&close).ok_or("an unterminated raw string")?;
        return Ok(open + 1 + rest[..end].chars().count() + close.chars().count());
    }
    match s[i] {
        '"' => {
            let mut j = i + 1;
            while j < s.len() && s[j] != '"' {
                j += if s[j] == '\\' { 2 } else { 1 };
            }
            if j >= s.len() {
                return Err("an unterminated string".into());
            }
            Ok(j + 1)
        }
        '\'' if s.get(i + 1) == Some(&'\\') => {
            let end = s
                .get(i + 3..)
                .and_then(|rest| rest.iter().position(|&c| c == '\''));
            Ok(i + 4 + end.ok_or("an unterminated char literal")?)
        }
        '\'' if s.get(i + 2) == Some(&'\'') => Ok(i + 3),
        _ => Ok(at),
    }
}

/// Advances past a comment at `at`, or returns `at` when there is none.
fn skip_comment(s: &[char], at: usize) -> Result<usize, String> {
    let Some(&first) = s.get(at) else {
        return Ok(at);
    };
    match (first, s.get(at + 1)) {
        ('/', Some('/')) => Ok(s[at..]
            .iter()
            .position(|&c| c == '\n')
            .map_or(s.len(), |p| at + p)),
        ('/', Some('*')) => {
            let (mut depth, mut i) = (1, at + 2);
            while depth > 0 {
                match (s.get(i), s.get(i + 1)) {
                    (Some('/'), Some('*')) => (depth, i) = (depth + 1, i + 2),
                    (Some('*'), Some('/')) => (depth, i) = (depth - 1, i + 2),
                    (Some(_), _) => i += 1,
                    (None, _) => return Err("an unterminated block comment".into()),
                }
            }
            Ok(i)
        }
        _ => Ok(at),
    }
}

/// Every attribute's inner text (`expect(..)` of `#[expect(..)]`), with
/// each comment inside it read as a space.
fn attributes(source: &str) -> Result<Vec<String>, String> {
    let s: Vec<char> = source.chars().collect();
    let (mut out, mut i) = (Vec::new(), 0);
    while i < s.len() {
        let next = skip_literal(&s, skip_comment(&s, i)?)?;
        if next != i {
            i = next;
            continue;
        }
        let bang = usize::from(s.get(i + 1) == Some(&'!'));
        if s[i] != '#' || s.get(i + 1 + bang) != Some(&'[') {
            i += 1;
            continue;
        }
        let (mut text, mut depth, mut j) = (String::new(), 1, i + 2 + bang);
        while depth > 0 {
            if j >= s.len() {
                return Err("an unterminated attribute".into());
            }
            let past_comment = skip_comment(&s, j)?;
            if past_comment != j {
                text.push(' ');
                j = past_comment;
                continue;
            }
            let past_literal = skip_literal(&s, j)?;
            if past_literal != j {
                text.extend(&s[j..past_literal]);
                j = past_literal;
                continue;
            }
            depth += match s[j] {
                '[' => 1,
                ']' => -1,
                _ => 0,
            };
            text.push(s[j]);
            j += 1;
        }
        text.pop();
        out.push(text);
        i = j;
    }
    Ok(out)
}

/// Splits `text` at commas outside brackets and strings.
fn top_level(text: &str) -> Result<Vec<String>, String> {
    let s: Vec<char> = text.chars().collect();
    let (mut parts, mut depth, mut from, mut i) = (Vec::new(), 0i32, 0, 0);
    while i < s.len() {
        let next = skip_literal(&s, i)?;
        if next != i {
            i = next;
            continue;
        }
        match s[i] {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(s[from..i].iter().collect::<String>().trim().to_string());
                from = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    let last: String = s[from..].iter().collect::<String>().trim().to_string();
    if !last.is_empty() {
        parts.push(last);
    }
    Ok(parts)
}

/// An `expect`, `allow` or `cfg_attr` attribute split into its name and
/// argument text, `None` for any other attribute, and refused when one of
/// those three is not `name(args)`.
fn call(attribute: &str) -> Result<Option<(&str, &str)>, String> {
    let attribute = attribute.trim();
    let end = attribute
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':'))
        .unwrap_or(attribute.len());
    let name = &attribute[..end];
    if level_of(name).is_none() && name != "cfg_attr" {
        return Ok(None);
    }
    let args = attribute[end..]
        .trim_start()
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .ok_or_else(|| format!("an unreadable {name} attribute"))?;
    Ok(Some((name, args)))
}

/// A lint is a path of identifiers (`dead_code`, `clippy::todo`).
fn is_lint_path(lint: &str) -> bool {
    lint.split("::").all(|part| {
        part.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

fn suppression(
    level: &'static str,
    args: &str,
    predicate: Option<String>,
) -> Result<Suppression, String> {
    let (mut lints, mut reason) = (Vec::new(), None);
    for arg in top_level(args)? {
        match arg.strip_prefix("reason").map(str::trim_start) {
            Some(rest) if rest.starts_with('=') => {
                reason = Some(rest[1..].trim().trim_matches('"').to_string())
            }
            _ if !arg.is_empty() => {
                let lint = arg.split_whitespace().collect::<String>();
                if !is_lint_path(&lint) {
                    return Err(format!("an unreadable lint name {lint}"));
                }
                lints.push(lint);
            }
            _ => {}
        }
    }
    Ok(Suppression {
        level,
        lints,
        reason,
        predicate,
    })
}

/// `expect` or `allow` as the level it names, or `None` for any other.
fn level_of(name: &str) -> Option<&'static str> {
    match name {
        "expect" => Some("expect"),
        "allow" => Some("allow"),
        _ => None,
    }
}

/// The suppressions inside one `cfg_attr(predicate, attr, ..)`; a nested
/// `cfg_attr` holds under both predicates.
fn cfg_attr_suppressions(args: &str) -> Result<Vec<Suppression>, String> {
    let parts = top_level(args)?;
    let predicate = parts.first().ok_or("an empty cfg_attr")?.clone();
    let mut out = Vec::new();
    for inner in &parts[1..] {
        out.extend(attribute_suppressions(inner, Some(&predicate))?);
    }
    Ok(out)
}

/// The suppressions one attribute carries under `predicate`.
fn attribute_suppressions(
    attribute: &str,
    predicate: Option<&str>,
) -> Result<Vec<Suppression>, String> {
    let Some((name, args)) = call(attribute)? else {
        return Ok(Vec::new());
    };
    if let Some(level) = level_of(name) {
        return Ok(vec![suppression(level, args, predicate.map(Into::into))?]);
    }
    let mut nested = cfg_attr_suppressions(args)?;
    if let Some(outer) = predicate {
        for found in &mut nested {
            found.predicate = found.predicate.take().map(|p| format!("all({outer}, {p})"));
        }
    }
    Ok(nested)
}

/// Every `expect` and `allow` in a source, `cfg_attr` ones included.
fn suppressions(source: &str) -> Result<Vec<Suppression>, String> {
    let mut out = Vec::new();
    for attribute in attributes(source)? {
        out.extend(attribute_suppressions(&attribute, None)?);
    }
    Ok(out)
}

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// `# production` and `# test` sections: count, level and lint, sorted.
fn render(repo: &Path) -> String {
    let mut counts: BTreeMap<(bool, String, &str), usize> = BTreeMap::new();
    for path in git(repo, &["ls-files", "*.rs"]).lines() {
        let source = std::fs::read_to_string(repo.join(path)).unwrap();
        let found = suppressions(&source).unwrap_or_else(|why| panic!("{path}: {why}"));
        for found in found {
            for lint in &found.lints {
                *counts
                    .entry((is_test_path(path), lint.clone(), found.level))
                    .or_default() += 1;
            }
        }
    }
    let mut text = String::from(
        "# Every #[expect] and #[allow] in the Rust sources, by lint (issue #337).\n\
         # Regenerate: BROKKR_REGENERATE_SUPPRESSIONS=1 cargo test -p brokkr-cli --test suppressions\n",
    );
    for (test, heading) in [(false, "# production\n"), (true, "# test\n")] {
        text.push_str(heading);
        for ((_, lint, level), count) in counts.iter().filter(|((t, _, _), _)| *t == test) {
            text.push_str(&format!("{count:5} {level:6} {lint}\n"));
        }
    }
    text
}

#[test]
fn every_suppression_in_the_tree_is_counted_in_the_baseline() {
    let root = workspace();
    let rendered = render(&root);
    if std::env::var_os("BROKKR_REGENERATE_SUPPRESSIONS").is_some() {
        std::fs::write(root.join(BASELINE), &rendered).unwrap();
    }
    let committed = read(BASELINE);
    assert_eq!(
        committed, rendered,
        "{BASELINE} no longer counts the tree's suppressions; a fixed one lowers it \
         (BROKKR_REGENERATE_SUPPRESSIONS=1), and a new one needs a ruling"
    );
}

/// A reason names a ruling when it cites an issue (`#N`) or a decision.
fn names_a_ruling(reason: &str) -> bool {
    let cites_issue = reason
        .match_indices('#')
        .any(|(at, _)| reason[at + 1..].starts_with(|c: char| c.is_ascii_digit()));
    let lower = reason.to_ascii_lowercase();
    let cites_decision = lower.match_indices("decision ").any(|(at, _)| {
        let digits = &lower[at + 9..];
        digits.len() >= 4 && digits[..4].bytes().all(|b| b.is_ascii_digit())
    });
    cites_issue || cites_decision
}

/// Suppressions of ratcheted lints the head adds to production files
/// since `base`, whose reason names no ruling.
fn unruled_additions(repo: &Path, base: &str) -> Vec<String> {
    let mut offenders = Vec::new();
    let changed = git(
        repo,
        &[
            "diff",
            "--name-only",
            "--diff-filter=AMR",
            base,
            "--",
            "*.rs",
        ],
    );
    for path in changed.lines().filter(|path| !is_test_path(path)) {
        let before = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["show", &format!("{base}:{path}")])
            .output()
            .unwrap();
        let before = if before.status.success() {
            String::from_utf8(before.stdout).unwrap()
        } else {
            String::new()
        };
        let after = std::fs::read_to_string(repo.join(path)).unwrap();
        let mut remaining =
            suppressions(&before).unwrap_or_else(|why| panic!("{base}:{path}: {why}"));
        for found in suppressions(&after).unwrap_or_else(|why| panic!("{path}: {why}")) {
            if let Some(at) = remaining.iter().position(|old| *old == found) {
                remaining.remove(at);
                continue;
            }
            let ratcheted = found
                .lints
                .iter()
                .any(|lint| RATCHETED.contains(&lint.as_str()));
            if ratcheted && !found.reason.as_deref().is_some_and(names_a_ruling) {
                offenders.push(format!(
                    "{path}: {} of {} without a ruling in its reason",
                    found.level,
                    found.lints.join(", ")
                ));
            }
        }
    }
    offenders
}

/// CI runs this on a pull request with the base commit named; it is
/// ignored in ordinary runs, and refuses to pass without its base.
#[test]
#[ignore = "run by CI on a pull request with BROKKR_SUPPRESSION_BASE set"]
fn an_added_suppression_of_a_ratcheted_lint_names_a_ruling() {
    let base = std::env::var("BROKKR_SUPPRESSION_BASE")
        .expect("BROKKR_SUPPRESSION_BASE names the base commit");
    let offenders = unruled_additions(&workspace(), &base);
    assert!(
        offenders.is_empty(),
        "suppressions added without a ruling:\n{}",
        offenders.join("\n")
    );
}

/// This file reads test paths as the exact gate declares them; if the
/// gate's vocabulary changes, this fails until the two agree again.
#[test]
fn the_test_paths_are_the_coverage_gates() {
    let gate = read("scripts/coverage-exact.sh");
    for line in [
        "test_dirs='tests|examples|benches'",
        r"test_files='tests\.rs|[^/]*[_-]tests\.rs'",
    ] {
        assert!(
            gate.lines().any(|l| l == line),
            "coverage-exact.sh no longer declares {line}"
        );
    }
    for (path, test) in [
        ("crates/a/tests/x.rs", true),
        ("crates/a/examples/x.rs", true),
        ("crates/a/benches/x.rs", true),
        ("crates/a/src/tests.rs", true),
        ("crates/a/src/x_tests.rs", true),
        ("crates/a/src/x-tests.rs", true),
        ("crates/a/src/lib.rs", false),
        ("crates/a/src/contests.rs", false),
        ("tests/support/env_guard.rs", true),
    ] {
        assert_eq!(is_test_path(path), test, "{path}");
    }
}

/// The check above runs only if CI runs it: the quality job must carry the
/// step that names the base and the test.
#[test]
fn ci_runs_the_added_suppression_check_on_every_pull_request() {
    let ci = read(".github/workflows/ci.yml");
    for needle in [
        "BROKKR_SUPPRESSION_BASE: ${{ github.event.pull_request.base.sha }}",
        "--test suppressions -- --ignored --exact an_added_suppression_of_a_ratcheted_lint_names_a_ruling",
    ] {
        assert!(ci.contains(needle), "ci.yml does not carry: {needle}");
    }
}

#[test]
fn the_lexer_reads_every_spelling_and_nothing_inside_literals() {
    let expect = |lints: &[&str], reason: &str, predicate: Option<&str>| Suppression {
        level: "expect",
        lints: lints.iter().map(|l| l.to_string()).collect(),
        reason: Some(reason.into()),
        predicate: predicate.map(Into::into),
    };
    let cases: &[(&str, Vec<Suppression>)] = &[
        (
            "#[expect(clippy::too_many_lines, reason = \"r\")]\nfn f() {}",
            vec![expect(&["clippy::too_many_lines"], "r", None)],
        ),
        (
            "#[expect(\n    clippy::too_many_lines,\n    reason = \"r\"\n)]",
            vec![expect(&["clippy::too_many_lines"], "r", None)],
        ),
        (
            "#![expect(unreachable_pub, reason = \"r\")]",
            vec![expect(&["unreachable_pub"], "r", None)],
        ),
        (
            "#[cfg_attr(not(test), expect(dead_code, reason = \"r\"))]",
            vec![expect(&["dead_code"], "r", Some("not(test)"))],
        ),
        ("let s = \"#[expect(clippy::todo)]\";", vec![]),
        ("let s = r#\"#[expect(clippy::todo)]\"#;", vec![]),
        (
            "// #[expect(clippy::todo)]\n/* #[allow(x)] /* nested */ */",
            vec![],
        ),
        (
            "let q = '\\''; #[expect(clippy::todo, reason = \"after an escaped quote\")]",
            vec![expect(&["clippy::todo"], "after an escaped quote", None)],
        ),
        (
            "let c = '#'; fn f<'a>(x: &'a str) {} #[expect(clippy::todo, reason = \"a ] in it\")]",
            vec![expect(&["clippy::todo"], "a ] in it", None)],
        ),
        (
            "#[expect(clippy::too_many_lines, reason = \"r\") /* later */]",
            vec![expect(&["clippy::too_many_lines"], "r", None)],
        ),
        (
            "#[expect(clippy::too_many_lines, reason = \"r\") // c\n]",
            vec![expect(&["clippy::too_many_lines"], "r", None)],
        ),
        (
            "#[expect(clippy::too_many_lines /* why */, reason = \"r\")]",
            vec![expect(&["clippy::too_many_lines"], "r", None)],
        ),
        (
            "#[cfg_attr(not(test), /* debt */ expect(clippy::todo, reason = \"r\"))]",
            vec![expect(&["clippy::todo"], "r", Some("not(test)"))],
        ),
        (
            "#[cfg_attr(\n    unix,\n    // debt\n    expect(clippy::todo, reason = \"r\")\n)]",
            vec![expect(&["clippy::todo"], "r", Some("unix"))],
        ),
        (
            "#[cfg_attr(unix, cfg_attr(not(test), expect(clippy::todo, reason = \"r\")))]",
            vec![expect(&["clippy::todo"], "r", Some("all(unix, not(test))"))],
        ),
        (
            "#[derive(Debug)] #[doc = \"a (b\"] #[rustfmt::skip] #[cfg_attr(unix, path = \"u.rs\")]",
            vec![],
        ),
    ];
    for (source, want) in cases {
        assert_eq!(suppressions(source).as_ref(), Ok(want), "{source}");
    }
    for (broken, why) in [
        ("#[expect(clippy::todo", "an unterminated attribute"),
        ("let s = \"open", "an unterminated string"),
        ("/* open", "an unterminated block comment"),
        ("#[expect]", "an unreadable expect attribute"),
        ("#[allow(dead_code) junk]", "an unreadable allow attribute"),
        (
            "#[cfg_attr(unix, expect = \"x\")]",
            "an unreadable expect attribute",
        ),
        (
            "#[expect(clippy::todo!, reason = \"r\")]",
            "an unreadable lint name clippy::todo!",
        ),
    ] {
        assert_eq!(suppressions(broken), Err(why.to_string()), "{broken}");
    }
}

#[test]
fn a_ruling_is_an_issue_or_a_decision() {
    for (reason, rules) in [
        ("baseline 2026-09, #288", true),
        ("decision 0071 ruling 4", true),
        ("Decision 0014's dependency ruling", true),
        ("it is long", false),
        ("see # below", false),
        ("decision pending", false),
    ] {
        assert_eq!(names_a_ruling(reason), rules, "{reason}");
    }
}

#[test]
fn the_diff_check_refuses_an_unruled_addition_and_passes_a_ruled_one() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    git(repo, &["init", "-q"]);
    std::fs::create_dir_all(repo.join("src")).unwrap();
    let write = |text: &str| std::fs::write(repo.join("src/lib.rs"), text).unwrap();
    write("#[expect(clippy::too_many_lines, reason = \"baseline 2026-09, #288\")]\nfn old() {}\n");
    git(repo, &["add", "."]);
    git(
        repo,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "base",
        ],
    );
    let old =
        "#[expect(clippy::too_many_lines, reason = \"baseline 2026-09, #288\")]\nfn old() {}\n";
    write(&format!("{old}#[cfg_attr(test,\n expect(clippy::too_many_arguments, reason = \"long\"))]\nfn new() {{}}\n"));
    assert_eq!(
        unruled_additions(repo, "HEAD"),
        ["src/lib.rs: expect of clippy::too_many_arguments without a ruling in its reason"]
    );
    write(&format!(
        "{old}#[expect(clippy::too_many_arguments, reason = \"#123 rules it\")]\nfn new() {{}}\n"
    ));
    assert!(unruled_additions(repo, "HEAD").is_empty());
    write(&format!("{old}#![allow(unreachable_pub)]\n"));
    assert_eq!(unruled_additions(repo, "HEAD").len(), 1);
}
