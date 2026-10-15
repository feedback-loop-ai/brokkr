//! Issue #365, asks 1 and 6: a sample a living doc shows is what the
//! command it shows prints, and the reforging ladder's tables are the
//! policy files they describe.
//!
//! Every fenced block in a living doc, read by `doc_truth`'s one reader,
//! that holds a `$ brokkr` prompt line is either [`SAMPLED`] or in a doc
//! [`ALLOWED`] names with its reason; a prompt in neither is refused, and
//! so is an allowance that covers nothing. A sampled block holds one
//! prompt, its first line, and every line after it is what the built
//! binary prints to stdout for that argv, run from the workspace root;
//! stderr is not compared. `…` is the one wildcard, as trycmd reads it: a
//! line that is `…` alone stands for any number of whole lines, and a `…`
//! inside a line for any run of characters within it. Everything else
//! matches byte for byte, tabs included.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use serde::Deserialize;
use serde_json::Value;

use crate::doc_truth::{readable_docs, Block, Doc};
use crate::workspace_root::{read, workspace};

/// What opens a prompt line, after its indent and its blockquote.
const PROMPT: &str = "$ brokkr";

/// The wildcard: any run of whole lines, or of characters within one.
const ELIDED: &str = "…";

/// Why a doc's prompts wait for #634 slice C to land.
const AFTER_701: &str =
    "#634 slice C (#701) rewrites its prompts; #365's next slice samples them once it merges";

/// Docs whose `$ brokkr` prompts are shown but not sampled, one row per
/// doc, never per line, each with its reason. An allowance that covers
/// nothing fails, so none outlives its prompts.
const ALLOWED: [(&str, &str); 12] = [
    ("docs/guides/quickstart.md", AFTER_701),
    ("docs/guides/starters/bun.md", AFTER_701),
    ("docs/guides/starters/go.md", AFTER_701),
    ("docs/guides/starters/node.md", AFTER_701),
    ("docs/guides/starters/python.md", AFTER_701),
    ("docs/guides/starters/rust.md", AFTER_701),
    ("docs/guides/secrets.md", AFTER_701),
    ("docs/guides/adopting-a-node-repo.md", AFTER_701),
    ("docs/guides/read-surfaces.md", AFTER_701),
    (
        "docs/guides/journal-and-verification.md",
        "a run's narrative, which needs a driven fake-driver run as its fixture: #365's next slice",
    ),
    (
        "docs/guides/provider-adapters.md",
        "doctor's provider lines are the operator's host",
    ),
    (
        "docs/guides/versioning.md",
        "doctor's contracts line, whose one version doc_truth's version rule holds",
    ),
];

/// A sampled block: the doc, the prompt line that opens the block, and
/// the argv the prompt runs.
struct Sample {
    doc: &'static str,
    prompt: &'static str,
    argv: &'static [&'static str],
}

const SAMPLED: [Sample; 3] = [
    Sample {
        doc: "docs/guides/agent-library.md",
        prompt: "$ brokkr agents list",
        argv: &["agents", "list"],
    },
    Sample {
        doc: "docs/guides/recipe-authoring.md",
        prompt: "$ brokkr recipes list",
        argv: &["recipes", "list"],
    },
    Sample {
        doc: "docs/guides/recipe-authoring.md",
        prompt: "$ brokkr compile --bundle recipes/night-shift",
        argv: &["compile", "--bundle", "recipes/night-shift"],
    },
];

/// Whether a line of a fenced block is a `$ brokkr` prompt.
fn is_prompt(line: &str) -> bool {
    line.trim_start().starts_with(PROMPT)
}

/// Each prompt in `docs` that neither opens a block a `sampled` row names
/// nor stands in a doc `allowed` names, and each allowance that covered
/// nothing.
fn unsampled(docs: &[Doc], sampled: &[Sample], allowed: &[(&str, &str)]) -> Vec<String> {
    let mut found = Vec::new();
    let mut used = vec![false; allowed.len()];
    for doc in docs {
        let allowance = allowed.iter().position(|(path, _)| *path == doc.path);
        for block in doc.blocks.iter().filter(|block| block.fenced) {
            for (at, line) in block.text.lines().enumerate() {
                let opens = |sample: &Sample| sample.doc == doc.path && sample.prompt == line;
                if !is_prompt(line) || (at == 0 && sampled.iter().any(opens)) {
                    continue;
                }
                match allowance {
                    Some(index) => used[index] = true,
                    None => found.push(format!(
                        "{}:{}: `{}` is neither sampled nor in an allowed doc",
                        doc.path,
                        block.first + at,
                        line.trim()
                    )),
                }
            }
        }
    }
    found.extend(
        allowed
            .iter()
            .zip(&used)
            .filter(|(_, used)| !**used)
            .map(|((path, _), _)| {
                format!("{path}: allowed, but covers nothing: no `{PROMPT}` prompt is left")
            }),
    );
    found
}

/// Whether one output line fits one sample line: equal, or each `…` in
/// the sample standing for any run of characters within the line.
fn line_fits(sample: &str, line: &str) -> bool {
    let mut pieces = sample.split(ELIDED);
    let Some(mut rest) = pieces.next().and_then(|first| line.strip_prefix(first)) else {
        return false;
    };
    let pieces: Vec<&str> = pieces.collect();
    let Some((last, middle)) = pieces.split_last() else {
        return rest.is_empty();
    };
    for piece in middle {
        match rest.find(piece) {
            Some(at) => rest = &rest[at + piece.len()..],
            None => return false,
        }
    }
    rest.ends_with(last)
}

/// The sample line no reading of `expected` gets past, or `None` when it
/// matches `actual` whole: the deepest line any alignment reached, and
/// `expected.len()` when the output runs on past the sample's end.
fn unmatched(expected: &[&str], actual: &[&str]) -> Option<usize> {
    let mut deepest = 0;
    let mut failed = BTreeSet::new();
    (!walk(expected, actual, (0, 0), &mut deepest, &mut failed)).then_some(deepest)
}

/// Whether `expected[i..]` matches `actual[j..]`, recording the deepest
/// sample line reached and each pair already known to fail.
fn walk(
    expected: &[&str],
    actual: &[&str],
    (i, j): (usize, usize),
    deepest: &mut usize,
    failed: &mut BTreeSet<(usize, usize)>,
) -> bool {
    *deepest = (*deepest).max(i);
    if failed.contains(&(i, j)) {
        return false;
    }
    let held = match expected.get(i) {
        None => j == actual.len(),
        Some(&ELIDED) => {
            (j..=actual.len()).any(|k| walk(expected, actual, (i + 1, k), deepest, failed))
        }
        Some(line) => {
            actual.get(j).is_some_and(|got| line_fits(line, got))
                && walk(expected, actual, (i + 1, j + 1), deepest, failed)
        }
    };
    if !held {
        failed.insert((i, j));
    }
    held
}

/// What a command printed to stdout, or why it did not run clean.
type Ran = Result<String, String>;

/// The built binary run with `argv` from the workspace root.
fn brokkr(argv: &[&str]) -> Ran {
    let output = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(argv)
        .current_dir(workspace())
        .output()
        .map_err(|error| format!("did not start: {error}"))?;
    let stdout = String::from_utf8(output.stdout).map_err(|error| format!("printed {error}"))?;
    match output.status.success() {
        true => Ok(stdout),
        false => Err(format!(
            "exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )),
    }
}

/// Each sample in `sampled` that is not what `run` prints for its argv,
/// or whose block is not one command with its output.
fn sample_failures(docs: &[Doc], sampled: &[Sample], run: impl Fn(&[&str]) -> Ran) -> Vec<String> {
    let mut found = Vec::new();
    for sample in sampled {
        let spelled = format!("{PROMPT} {}", sample.argv.join(" "));
        if sample.prompt != spelled {
            found.push(format!(
                "{}: the sample `{}` runs `{spelled}`",
                sample.doc, sample.prompt
            ));
            continue;
        }
        let blocks: Vec<&Block> = docs
            .iter()
            .filter(|doc| doc.path == sample.doc)
            .flat_map(|doc| &doc.blocks)
            .filter(|block| block.fenced && block.text.lines().next() == Some(sample.prompt))
            .collect();
        match blocks[..] {
            [block] => found.extend(sample_failure(sample, block, &run)),
            _ => found.push(format!(
                "{}: {} fenced blocks open with `{}`; a sample is exactly one",
                sample.doc,
                blocks.len(),
                sample.prompt
            )),
        }
    }
    found
}

/// Why one sampled block is not what `run` prints, if it is not.
fn sample_failure(sample: &Sample, block: &Block, run: impl Fn(&[&str]) -> Ran) -> Option<String> {
    let at = |index: usize| format!("{}:{}", sample.doc, block.first + index);
    let expected: Vec<&str> = block.text.lines().skip(1).collect();
    if let Some(index) = expected
        .iter()
        .position(|line| line.trim_start().starts_with("$ "))
    {
        return Some(format!(
            "{}: `{}` is a second `$ ` line; a sample runs one command",
            at(index + 1),
            expected[index].trim()
        ));
    }
    if expected
        .iter()
        .all(|line| line.trim().is_empty() || *line == ELIDED)
    {
        return Some(format!(
            "{}: `{}` shows no output but `{ELIDED}`",
            at(0),
            sample.prompt
        ));
    }
    let actual = match run(sample.argv) {
        Ok(actual) => actual,
        Err(why) => return Some(format!("{}: `{}` {why}", at(0), sample.prompt)),
    };
    let index = unmatched(&expected, &actual.lines().collect::<Vec<_>>())?;
    Some(match expected.get(index) {
        Some(line) => format!(
            "{}: `{line}` is not what `{}` printed:\n{actual}",
            at(index + 1),
            sample.prompt
        ),
        None => format!(
            "{}: the sample ends before what `{}` printed:\n{actual}",
            at(index),
            sample.prompt
        ),
    })
}

#[test]
fn every_brokkr_prompt_in_a_living_doc_is_sampled_or_allowed() {
    assert_eq!(
        unsampled(&readable_docs(), &SAMPLED, &ALLOWED),
        Vec::<String>::new(),
        "a living doc shows a `$ brokkr` prompt: sample it in SAMPLED, or allow its doc with a reason"
    );
}

#[test]
fn every_sample_is_what_the_binary_prints() {
    assert_eq!(
        sample_failures(&readable_docs(), &SAMPLED, brokkr),
        Vec::<String>::new(),
        "a sampled block is not what its command prints: paste the output, eliding with `…`"
    );
}

/// A doc of the given fenced blocks, each with the line its text starts
/// on.
fn fenced(path: &str, blocks: &[(usize, &str)]) -> Doc {
    Doc {
        path: path.to_string(),
        blocks: blocks
            .iter()
            .map(|&(first, text)| Block {
                first,
                text: text.to_string(),
                fenced: true,
            })
            .collect(),
    }
}

#[test]
fn a_prompt_neither_sampled_nor_allowed_is_refused() {
    let sampled = [Sample {
        doc: "docs/a.md",
        prompt: "$ brokkr runs",
        argv: &["runs"],
    }];
    let docs = [
        fenced(
            "docs/a.md",
            &[
                (3, "$ brokkr runs\nrun-1"),
                (9, "out\n$ brokkr runs"),
                (14, "$ brokkr runs --json"),
            ],
        ),
        fenced("docs/b.md", &[(2, "  $ brokkr doctor\n$ brokkr realms")]),
        fenced("docs/d.md", &[(7, "out\n  $ brokkr realms")]),
        fenced("docs/c.md", &[(5, "brokkr runs\n$ cargo test")]),
    ];
    let allowed = [("docs/b.md", "the host"), ("docs/c.md", "stale")];
    assert_eq!(
        unsampled(&docs, &sampled, &allowed),
        [
            "docs/a.md:10: `$ brokkr runs` is neither sampled nor in an allowed doc",
            "docs/a.md:14: `$ brokkr runs --json` is neither sampled nor in an allowed doc",
            "docs/d.md:8: `$ brokkr realms` is neither sampled nor in an allowed doc",
            "docs/c.md: allowed, but covers nothing: no `$ brokkr` prompt is left",
        ]
    );
}

#[test]
fn the_wildcard_reads_as_trycmd_does() {
    let reads = |expected: &str, actual: &str| {
        unmatched(
            &expected.lines().collect::<Vec<_>>(),
            &actual.lines().collect::<Vec<_>>(),
        )
    };
    assert_eq!(reads("a\n…\nb", "a\nb"), None);
    assert_eq!(reads("a\n…\nb", "a\nx\ny\nb"), None);
    assert_eq!(reads("a\n…", "a\nx\ny"), None);
    assert_eq!(reads("a\t…\tc", "a\tsome digest\tc"), None);
    assert_eq!(reads("…/recipes/fast", "/home/x/recipes/fast"), None);
    assert_eq!(reads("a…b…c", "a-c-b-c"), None);
    assert_eq!(reads("xy…yz", "xyz"), Some(0));
    assert_eq!(reads("a\tb", "a    b"), Some(0));
    assert_eq!(reads("a\n…\nb\nc", "a\nb\nx\nb\nd"), Some(3));
    assert_eq!(reads("a\nb", "a\nb\nc"), Some(2));
    assert_eq!(reads("a\nb", "a"), Some(1));
}

#[test]
fn a_sample_is_one_command_and_its_output() {
    let sample = |prompt: &'static str, argv: &'static [&'static str]| Sample {
        doc: "docs/a.md",
        prompt,
        argv,
    };
    let docs = [fenced(
        "docs/a.md",
        &[
            (3, "$ brokkr runs\nrun-1\n…\nrun-9"),
            (9, "$ brokkr realms\n…\n\n…"),
            (15, "$ brokkr seats\nanalyst\n$ brokkr doctor\nok"),
            (21, "$ brokkr costs\n…"),
            (26, "$ brokkr costs\ntotal"),
            (30, "$ brokkr ledger\ntotal"),
            (34, "$ brokkr inspect\none\ntwo"),
        ],
    )];
    let sampled = [
        sample("$ brokkr runs", &["runs"]),
        sample("$ brokkr realms", &["realms"]),
        sample("$ brokkr seats", &["seats"]),
        sample("$ brokkr costs", &["costs"]),
        sample("$ brokkr doctor", &["doctor"]),
        sample("$ brokkr  runs", &["runs"]),
        sample("$ brokkr ledger", &["ledger"]),
        sample("$ brokkr inspect", &["inspect"]),
    ];
    let run = |argv: &[&str]| match argv {
        ["runs"] => Ok("run-1\nrun-2\nrun-8\n".to_string()),
        ["ledger"] => Err("exited 1: no journal".to_string()),
        _ => Ok("one\ntwo\nthree\n".to_string()),
    };
    assert_eq!(
        sample_failures(&docs, &sampled, run),
        [
            "docs/a.md:6: `run-9` is not what `$ brokkr runs` printed:\nrun-1\nrun-2\nrun-8\n",
            "docs/a.md:9: `$ brokkr realms` shows no output but `…`",
            "docs/a.md:17: `$ brokkr doctor` is a second `$ ` line; a sample runs one command",
            "docs/a.md: 2 fenced blocks open with `$ brokkr costs`; a sample is exactly one",
            "docs/a.md: 0 fenced blocks open with `$ brokkr doctor`; a sample is exactly one",
            "docs/a.md: the sample `$ brokkr  runs` runs `$ brokkr runs`",
            "docs/a.md:30: `$ brokkr ledger` exited 1: no journal",
            "docs/a.md:36: the sample ends before what `$ brokkr inspect` printed:\none\ntwo\nthree\n",
        ]
    );
}

/// A policy table as the ladder's tables read it. A rule key these tables
/// do not render is refused by its parser rather than left out of a row.
#[derive(Deserialize)]
struct Policy {
    phases: Vec<String>,
    rules: Vec<Rule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    id: String,
    from: String,
    result: String,
    #[serde(default)]
    when: BTreeMap<String, Value>,
    next: Option<String>,
    #[serde(default)]
    park: bool,
    severity: Option<String>,
    reason: String,
}

/// The two tables the ladder renders, each with the recipe it is.
const POLICIES: [(&str, &str); 2] = [
    ("fast", "recipes/fast/policy.json"),
    ("triage", "recipes/triage/policy.json"),
];

fn policy(path: &str) -> Policy {
    serde_json::from_str(&read(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// Whether a condition counts entries to a phase: `visits_<phase>_gte`.
fn is_bound(key: &str) -> bool {
    key.starts_with("visits_") && key.ends_with("_gte")
}

/// A table cell, its `|` escaped.
fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

/// A rule's conditions `keep` selects, each `` `key: value` ``, or
/// `none`.
fn conditions(rule: &Rule, keep: impl Fn(&str) -> bool) -> String {
    let kept: Vec<String> = rule
        .when
        .iter()
        .filter(|(key, _)| keep(key))
        .map(|(key, value)| format!("`{key}: {value}`"))
        .collect();
    match kept.is_empty() {
        true => "none".to_string(),
        false => kept.join(", "),
    }
}

/// A Markdown table of `header` and `rows`, each cell escaped.
fn table(header: &[&str], rows: &[Vec<String>]) -> String {
    let line = |cells: Vec<String>| format!("| {} |", cells.join(" | "));
    let mut lines = vec![
        line(header.iter().map(|title| cell(title)).collect()),
        format!("|{}", "---|".repeat(header.len())),
    ];
    lines.extend(
        rows.iter()
            .map(|row| line(row.iter().map(|text| cell(text)).collect())),
    );
    lines.join("\n")
}

/// Every bounded return in `policies`: a rule whose `next` its table
/// lists before its `from`, below an exhaustion arm, a rule with its
/// `from` and `result` whose conditions count entries to a phase.
fn bounded_returns(policies: &[(&str, Policy)]) -> String {
    let mut rows = Vec::new();
    for (recipe, policy) in policies {
        let place = |phase: &str| {
            policy
                .phases
                .iter()
                .position(|listed| listed == phase)
                .unwrap_or_else(|| panic!("{recipe}: `{phase}` is not a phase"))
        };
        for (at, rule) in policy.rules.iter().enumerate() {
            let Some(next) = rule.next.as_deref() else {
                continue;
            };
            let arms: Vec<&Rule> = policy.rules[..at]
                .iter()
                .filter(|arm| arm.from == rule.from && arm.result == rule.result)
                .filter(|arm| arm.when.keys().any(|key| is_bound(key)))
                .collect();
            if place(next) >= place(&rule.from) || arms.is_empty() {
                continue;
            }
            let bounds: BTreeSet<String> =
                arms.iter().map(|arm| conditions(arm, is_bound)).collect();
            let ids: Vec<String> = arms.iter().map(|arm| format!("`{}`", arm.id)).collect();
            rows.push(vec![
                format!("`{}`", rule.id),
                format!("`{recipe}`"),
                format!("`{}` → `{next}` on `{}`", rule.from, rule.result),
                conditions(rule, |key| !is_bound(key)),
                bounds.into_iter().collect::<Vec<_>>().join(", "),
                ids.join(", "),
            ]);
        }
    }
    table(
        &[
            "Rule",
            "Recipe",
            "Return",
            "Also requires",
            "Bound",
            "Exhaustion arms",
        ],
        &rows,
    )
}

/// Review's exhaustion arms in `fast`, in table order: each rule from
/// `review` that counts implement visits, with what else it requires, its
/// outcome, and its reason verbatim.
fn review_exhaustion(fast: &Policy) -> String {
    const BOUND: &str = "visits_implement_gte";
    let rows: Vec<Vec<String>> = fast
        .rules
        .iter()
        .filter(|rule| rule.from == "review" && rule.when.contains_key(BOUND))
        .map(|rule| {
            let outcome = match (&rule.next, rule.park, &rule.severity) {
                (Some(next), false, None) => format!("`next: {next}`"),
                (Some(next), false, Some(severity)) => {
                    format!("`next: {next}`, severity `{severity}`")
                }
                (None, true, None) => "`park: true`".to_string(),
                _ => panic!("{}: an outcome the ladder cannot render", rule.id),
            };
            vec![
                format!("`{}`", rule.id),
                conditions(rule, |key| key != BOUND),
                outcome,
                rule.reason.clone(),
            ]
        })
        .collect();
    table(&["Rule", "Also requires", "Outcome", "Reason"], &rows)
}

/// The guide's text between `<!-- {name}:start -->` and
/// `<!-- {name}:end -->`, trimmed.
fn between<'a>(guide: &'a str, name: &str) -> &'a str {
    let start = format!("<!-- {name}:start -->");
    let end = format!("<!-- {name}:end -->");
    let (_, after) = guide
        .split_once(&start)
        .unwrap_or_else(|| panic!("the guide lost {start}"));
    let (inside, _) = after
        .split_once(&end)
        .unwrap_or_else(|| panic!("the guide lost {end}"));
    inside.trim()
}

const GUIDE: &str = "docs/guides/recipe-authoring.md";

#[test]
fn the_bounded_returns_table_is_the_policy_files() {
    let policies: Vec<(&str, Policy)> = POLICIES
        .iter()
        .map(|&(recipe, path)| (recipe, policy(path)))
        .collect();
    let rendered = bounded_returns(&policies);
    assert_eq!(
        between(&read(GUIDE), "bounded-returns"),
        rendered,
        "{GUIDE}'s bounded returns drifted from the policy files; the table they render:\n\n{rendered}\n"
    );
}

#[test]
fn the_review_exhaustion_table_is_fasts_policy() {
    let rendered = review_exhaustion(&policy(POLICIES[0].1));
    assert_eq!(
        between(&read(GUIDE), "review-exhaustion"),
        rendered,
        "{GUIDE}'s review exhaustion arms drifted from fast's policy; the table it renders:\n\n{rendered}\n"
    );
}
