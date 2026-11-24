//! Decision 0033's executable documentation: the sixty-second guide is
//! the recipe library rendered as one table, and the contribution gate
//! keeps its declaration, evidence, head-binding, and visible escape
//! hatch in repository-owned platform data.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::doc_samples::hermetic;
use crate::workflow;

#[path = "contributing/docs_lane.rs"]
mod docs_lane;
#[path = "contributing/inline_copies.rs"]
mod inline_copies;
#[path = "contributing/job_lines.rs"]
mod job_lines;
use inline_copies::{
    assert_first_words_listed, assert_inline_copies, assert_inline_only_used, text_reads_as_command,
};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn guide_rows(guide: &str) -> Vec<(String, String, String, String)> {
    let table = guide
        .split_once("<!-- recipe-table:start -->")
        .expect("guide starts its recipe table")
        .1
        .split_once("<!-- recipe-table:end -->")
        .expect("guide ends its recipe table")
        .0;
    table
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
            assert_eq!(cells.len(), 4, "one four-column recipe row: {line}");
            (
                cells[0].trim_matches('`').to_string(),
                cells[1].to_string(),
                cells[2].to_string(),
                cells[3].to_string(),
            )
        })
        .collect()
}

/// Brokkr's own bundles, in the library since #359 and listed there, but
/// the operator's tools rather than a contributor's choice: `self`
/// delivers this repository (decision 0005) and `verify` judges a slice
/// already delivered.
const OPERATORS_OWN: [&str; 2] = ["self", "verify"];

#[test]
fn the_sixty_second_table_is_the_recipe_library() {
    let root = workspace();
    let guide = std::fs::read_to_string(root.join("CONTRIBUTING.md")).unwrap();
    assert!(
        guide.lines().count() < 120,
        "the guide stopped being one screen"
    );
    assert_eq!(
        guide
            .matches("| Recipe | When to use it | What it seats | Rough cost |")
            .count(),
        1,
        "the short guide has one recipe table"
    );
    let documented = guide_rows(&guide);

    // A config home of its own, so no host's personal shelf (#697) adds rows.
    let config = tempfile::tempdir().unwrap();
    let mut list = crate::built::brokkr(["library", "recipes", "list"]);
    list.current_dir(&root);
    hermetic(&mut list, config.path());
    let output = list.output().unwrap();
    assert!(
        output.status.success(),
        "recipe listing failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let listing = String::from_utf8(output.stdout).unwrap();
    let listed: BTreeMap<String, (String, String, String)> = listing
        .lines()
        .filter(|line| !line.starts_with("warning:"))
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            (fields.len() == 8).then(|| {
                (
                    fields[0].to_string(),
                    (
                        fields[5].to_string(),
                        fields[3].to_string(),
                        fields[4].to_string(),
                    ),
                )
            })
        })
        .collect();

    let mut library = Vec::new();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root.join("recipes"))
        .unwrap()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.join("bundle.json").is_file())
        .collect();
    dirs.sort();
    for dir in dirs {
        // Read as composed (#359): an overlay inherits its base's cost.
        let bundle = brokkr_runtime::bundle::compose::resolve(&dir)
            .unwrap()
            .document;
        let name = bundle["name"].as_str().unwrap();
        if OPERATORS_OWN.contains(&name) {
            continue;
        }
        let description = bundle["description"].as_str().unwrap();
        let cost = bundle["cost"].as_str().unwrap();
        assert!(
            !description.is_empty() && !description.contains(['\r', '\n']),
            "{name} needs a one-line description"
        );
        let (listed_description, seats, listed_cost) = listed
            .get(name)
            .unwrap_or_else(|| panic!("brokkr library recipes list omitted {name}"));
        assert_eq!(listed_description, description, "{name} description");
        assert_eq!(listed_cost, cost, "{name} cost");
        library.push((
            name.to_string(),
            description.to_string(),
            seats.clone(),
            cost.to_string(),
        ));
    }
    assert_eq!(documented, library, "guide table drifted from recipe data");
}

#[test]
fn the_platform_gate_carries_every_part_of_the_ruling() {
    let root = workspace();
    let template = std::fs::read_to_string(root.join(".github/pull_request_template.md")).unwrap();
    assert!(template.lines().any(|line| line == "Brokkr-Run: <run id>"));

    let workflow = std::fs::read_to_string(root.join(".github/workflows/pr-gates.yml")).unwrap();
    for binding in [
        "name: delivered by brokkr",
        "github.event.pull_request.base.sha",
        "fetch-depth: 0",
        "scripts/delivered-by-brokkr.sh",
        "join(github.event.pull_request.labels.*.name, ',')",
        // Decision 0038 ruling 4: the label re-runs the gate, no reopen,
        // and so does a body edit (#1295).
        "types: [opened, synchronize, reopened, edited, labeled, unlabeled]",
    ] {
        assert!(workflow.contains(binding), "CI lost binding: {binding}");
    }

    // The gate itself is repository-owned data too (0033 ruling 4, 0038
    // rulings 2, 3 and 6): every check the ruling names is in the script.
    let gate = std::fs::read_to_string(root.join("scripts/delivered-by-brokkr.sh")).unwrap();
    for binding in [
        "refs/heads/brokkr-runs/${run}",
        "journal verify \"$work/${run}.ndjson\"",
        ".payload.from == \"ship\" and .payload.result == \"shipped\"",
        ".seq == $seq and .journal_head_hash == $journal",
        "patch-id --verbatim",
        ".repo_head == $head",
        "Brokkr-Preflight",
        ".payload.from == \"review\" and .payload.next == \"done\"",
        ".classes.docs.paths",
        "by-hand label",
        "the tier would have been",
        // Decision 0046 ruling 3: the boundary is read off every
        // `effect/started` entry after the journal verifies, and the
        // adjective is the script's rendering of the plain word.
        "boundary_suffix()",
        ".payload.boundary",
        "IN(\"harness\", \"open\")",
        " · unboxed",
        " · boundary not recorded",
        "has(\"member\")",
    ] {
        assert!(gate.contains(binding), "the gate lost binding: {binding}");
    }

    // 0038 ruling 3: the docs class is data, not a pattern in a workflow.
    let classes: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".github/delivery-classes.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(classes["schema"], "forge.delivery-classes/v1");
    let docs = classes["classes"]["docs"]["paths"]
        .as_array()
        .expect("docs paths");
    assert!(!docs.is_empty());
    assert!(template.contains("Brokkr-Preflight: <run id>"));
}

/// One leg of a workflow job: the check it reports (one per `matrix.os`
/// entry when the job's name carries the matrix), its runner, and the
/// steps that runner takes.
struct WorkflowLeg {
    check: String,
    id: String,
    file: &'static str,
    runner: &'static str,
    steps: Vec<Step>,
}

/// One step a leg takes: the variables it sets (its job's first), and
/// every line it runs: its one-line command, or each code line of its
/// multi-line block.
#[derive(Clone)]
struct Step {
    env: Vec<(String, String)>,
    lines: Vec<String>,
}

impl WorkflowLeg {
    /// The leg's commands as a local run writes them, read against the
    /// lines `LEG_LINES` holds for it, in order: a line its row writes
    /// carries its step's variables that `LOCAL_ENV` carries, in their
    /// local form, and any other line must be the one the table holds in
    /// its place. A line added, dropped or moved fails here.
    fn local_commands(&self) -> Vec<String> {
        let (_, held) = LEG_LINES
            .iter()
            .find(|(check, _)| *check == self.check)
            .unwrap_or_else(|| panic!("LEG_LINES does not hold {}", self.check));
        let mut held = held.iter();
        let mut commands = Vec::new();
        for step in &self.steps {
            for line in &step.lines {
                match held.next() {
                    Some(Line::Written) => commands.push(format!("{}{line}", self.prefix(step))),
                    Some(Line::Unwritten(text)) => {
                        assert_eq!(
                            line, text,
                            "{} runs a line LEG_LINES does not hold",
                            self.check
                        );
                    }
                    None => panic!(
                        "{} runs `{line}`, past the lines LEG_LINES holds",
                        self.check
                    ),
                }
            }
        }
        let missing: Vec<&Line> = held.collect();
        assert!(
            missing.is_empty(),
            "{} no longer runs {missing:?}",
            self.check
        );
        commands
    }

    /// The variables `step` sets that a local run carries, as a prefix.
    fn prefix(&self, step: &Step) -> String {
        let mut prefix = String::new();
        for (name, value) in &step.env {
            let carried = LOCAL_ENV
                .iter()
                .find(|(listed, _)| listed == name)
                .unwrap_or_else(|| {
                    panic!(
                        "LOCAL_ENV does not rule on {name}, which {} sets",
                        self.check
                    )
                })
                .1;
            // An omitted variable never reaches the guide's line, so its
            // value needs no local form (#647's step output is one).
            if matches!(carried, Local::Omitted) {
                continue;
            }
            let value = local_value(value, self.runner);
            if !value.is_empty() {
                prefix.push_str(&format!("{name}={value} "));
            }
        }
        prefix
    }
}

/// A line a checked leg runs.
#[derive(Clone, Copy, Debug)]
enum Line {
    /// A command its row writes.
    Written,
    /// A line its row leaves out, held word for word: output, exits and
    /// shell structure, a fetch of the pull request's base (a local clone
    /// has it), the ratchets job's check that its checkout is a pull
    /// request's merge commit, the macOS host report and its check for
    /// `/usr/bin/sandbox-exec`, the coverage job's pin reads and its
    /// cargo-public-api install (`the_by_hand_tool_versions_are_the_pins`
    /// holds its version), the suite's verdict over its machines (a local
    /// run is one machine), and every line of `delivered by brokkr`, which
    /// has no local form: it judges a pull request's own evidence with its
    /// base branch's verifier.
    Unwritten(&'static str),
}

/// The coverage jobs' reads of their pins, which each machine and the gate
/// make.
const PIN_READS: [Line; 3] = [
    Line::Unwritten(
        r#"echo "toolchain=$(tr -d '[:space:]' < rust-nightly-version.txt)" >> "$GITHUB_OUTPUT""#,
    ),
    Line::Unwritten(
        r#"echo "cargo_llvm_cov=$(tr -d '[:space:]' < cargo-llvm-cov-version.txt)" >> "$GITHUB_OUTPUT""#,
    ),
    Line::Unwritten(
        r#"echo "RUSTUP_TOOLCHAIN=$(tr -d '[:space:]' < rust-nightly-version.txt)" >> "$GITHUB_ENV""#,
    ),
];

/// The suite's verdict over its machines, or the docs lane (#1295).
const VERDICT_LINES: [Line; 2] = [
    Line::Unwritten(r#"test "$CLASSIFIED" = success"#),
    Line::Unwritten(
        r#"case "$LANE" in docs) test "$DOCS" = success ;; full) test "$MACHINES" = success ;; *) exit 1 ;; esac"#,
    ),
];

/// Every line each of the twelve checks' legs runs, in order: a skipped
/// line such as an early `exit 0`, or a command moved into a branch that
/// never runs, fails the test like a command the row left out.
const LEG_LINES: [(&str, &[Line]); 12] = [
    (
        "delivered by brokkr",
        &[
            Line::Unwritten(
                r#"git -C pr fetch --no-tags "$BASE_REPO" "${BASE_REF}:refs/remotes/base/${BASE_REF}""#,
            ),
            Line::Unwritten(
                r#"cargo build --locked --manifest-path "base-verifier/Cargo.toml" -p brokkr-cli"#,
            ),
            Line::Unwritten("if [ ! -f base-verifier/scripts/delivered-by-brokkr.sh ]; then"),
            Line::Unwritten(r#"case ",${LABELS}," in"#),
            Line::Unwritten("*,by-hand,*)"),
            Line::Unwritten(
                r#"echo "delivered by brokkr: skipped — operator applied the by-hand label; the base branch carries no gate script yet""#,
            ),
            Line::Unwritten("exit 0 ;;"),
            Line::Unwritten("esac"),
            Line::Unwritten(
                r#"echo "delivered by brokkr: the base branch carries no gate script" >&2"#,
            ),
            Line::Unwritten("exit 1"),
            Line::Unwritten("fi"),
            Line::Unwritten("bash base-verifier/scripts/delivered-by-brokkr.sh"),
        ],
    ),
    ("MSRV (1.89)", &[Line::Written]),
    (
        "format, clippy, contracts",
        &[
            Line::Written,
            Line::Written,
            Line::Unwritten(r#"git fetch --no-tags --depth=1 origin "$BROKKR_SUPPRESSION_BASE""#),
            Line::Written,
            Line::Written,
            Line::Written,
        ],
    ),
    (
        "test (ubuntu-latest)",
        &[
            Line::Written,
            Line::Written,
            Line::Written,
            VERDICT_LINES[0],
            VERDICT_LINES[1],
        ],
    ),
    (
        "test (macos-latest)",
        &[
            Line::Written,
            Line::Written,
            Line::Unwritten("sw_vers"),
            Line::Unwritten("uname -m"),
            Line::Unwritten("test -x /usr/bin/sandbox-exec"),
            Line::Written,
            Line::Written,
            Line::Written,
            Line::Written,
            Line::Written,
            VERDICT_LINES[0],
            VERDICT_LINES[1],
        ],
    ),
    (
        "exact coverage gate",
        &[
            // A machine: its pin reads and its measurement.
            PIN_READS[0],
            PIN_READS[1],
            PIN_READS[2],
            Line::Written,
            // The report: its pin reads, the report over every machine's
            // profile, and the ratchets.
            PIN_READS[0],
            PIN_READS[1],
            PIN_READS[2],
            Line::Written,
            Line::Written,
            Line::Unwritten("cargo install --locked cargo-public-api --version 0.52.0"),
            Line::Written,
            // The gate: its verdict over the report, or the docs lane.
            Line::Unwritten(r#"test "$CLASSIFIED" = success"#),
            Line::Unwritten(
                r#"case "$LANE" in docs) test "$DOCS" = success ;; full) test "$REPORT" = success ;; *) exit 1 ;; esac"#,
            ),
        ],
    ),
    ("dependency licenses (cargo-deny)", &[Line::Written]),
    (
        "non-Rust lints",
        &[
            Line::Written,
            Line::Written,
            Line::Written,
            Line::Written,
            Line::Written,
            Line::Written,
        ],
    ),
    (
        "baseline ratchets",
        &[
            Line::Written,
            Line::Written,
            Line::Written,
            Line::Unwritten("git rev-parse --verify --quiet 'HEAD^2' > /dev/null || {"),
            Line::Unwritten(
                r#"echo "ratchet refusal: the checkout is not the pull request's merge commit" >&2"#,
            ),
            Line::Unwritten("exit 1"),
            Line::Unwritten("}"),
            Line::Written,
        ],
    ),
    ("RustSec dependency audit", &[Line::Written]),
    ("release binary artifact", &[Line::Written, Line::Written]),
    ("mutants in the diff: brokkr-core", &[Line::Written]),
];

/// Whether a local run sets a variable a checked step sets.
#[derive(Clone, Copy)]
enum Local {
    Carried,
    Omitted,
}

/// Every variable a step sets whose lines a row writes, ruled on once: a
/// variable no row has ruled on fails the test rather than dropping out of
/// the guide. A step no row writes, such as each of `delivered by
/// brokkr`'s, has its variables held word for word by `JOB_LINES`.
const LOCAL_ENV: [(&str, Local); 10] = [
    ("BROKKR_REQUIRE_BOUNDARY_EVIDENCE", Local::Carried),
    ("BROKKR_SUPPRESSION_BASE", Local::Carried),
    ("PR_BODY", Local::Carried),
    // The mutants gate's base: its command names it as an argument.
    ("BASE", Local::Omitted),
    // CI's runner has two cores; unset, cargo-mutants runs one job.
    ("MUTANTS_JOBS", Local::Omitted),
    // `npm ci --ignore-scripts` runs no download script either way.
    ("PUPPETEER_SKIP_DOWNLOAD", Local::Omitted),
    // The protected set's directory exists only where CI's setup step
    // prepared it in the background (#647); a local run waits on nothing.
    ("BROKKR_SYSTEM_SET", Local::Omitted),
    // The machine a test job's runner is; a local run, naming none, runs
    // every machine's shards.
    ("BROKKR_TEST_MACHINE", Local::Omitted),
    // Where a coverage machine leaves its profile and the gate reads every
    // machine's; a local run measures and reports in one.
    ("BROKKR_COVERAGE_PROFILES", Local::Omitted),
    // CI requires a real-delivery test's PID namespace where it installed
    // bubblewrap (#1137h); a local run, which may be a box that cannot nest
    // one (decision 0054), leaves it unset and such a test skips aloud.
    ("BROKKR_REQUIRE_PIDNS", Local::Omitted),
];

/// A variable's value on a local run of `runner`'s leg: a literal
/// unquoted, a test of the runner decided, the pull request's base as
/// `origin/main` and its body as a placeholder. Any other expression
/// fails the test.
fn local_value(value: &str, runner: &str) -> String {
    if let Some(test) = value.strip_prefix("${{ runner.os == '") {
        let (os, arms) = test.split_once("' && ").expect("a runner test");
        let (then, other) = arms
            .trim_end_matches(" }}")
            .split_once(" || ")
            .expect("both arms of a runner test");
        return if os == runner { then } else { other }
            .trim_matches('\'')
            .to_string();
    }
    match value {
        "${{ github.event.pull_request.base.sha }}" => "origin/main".to_string(),
        "${{ github.event.pull_request.body }}" => "\"<your pull request's body>\"".to_string(),
        _ if !value.contains("${{") => value.trim_matches('\'').to_string(),
        _ => panic!("no local form for `{value}`"),
    }
}

/// The `NAME: value` pairs of the `env:` map among `lines`, if any.
fn env_map(lines: &[&str]) -> Vec<(String, String)> {
    let indent = |line: &str| line.len() - line.trim_start().len();
    let Some(at) = lines.iter().position(|line| line.trim() == "env:") else {
        return Vec::new();
    };
    lines[at + 1..]
        .iter()
        .filter(|line| !line.trim_start().starts_with('#'))
        .take_while(|line| indent(line) > indent(lines[at]))
        .map(|line| {
            let (name, value) = line.trim().split_once(": ").expect("a NAME: value pair");
            (name.to_string(), value.to_string())
        })
        .collect()
}

/// The required checks that are verdicts over other jobs, as (the
/// verdict's job, the jobs it judges on a change that is not documentation
/// alone, in the order they run): a verdict's leg runs every step those
/// jobs run, on its system, and then its own, so its row writes what a
/// local run of the whole suite runs. The docs lane (#1295) is not among
/// them: a local run is the whole suite.
const VERDICTS: [(&str, &[&str]); 2] = [
    ("engine", &["engine-machines"]),
    ("coverage-gate", &["coverage-machines", "coverage"]),
];

fn workflow_legs(root: &Path, file: &'static str) -> Vec<WorkflowLeg> {
    let text = std::fs::read_to_string(root.join(".github/workflows").join(file)).unwrap();
    let mut legs: Vec<WorkflowLeg> = workflow::jobs(&text)
        .into_iter()
        .flat_map(|(id, body)| job_legs(file, &id, &body.lines().collect::<Vec<_>>()))
        .collect();
    for (verdict, judged) in VERDICTS {
        for runner in ["Linux", "macOS"] {
            if !legs
                .iter()
                .any(|leg| leg.id == verdict && leg.runner == runner)
            {
                continue;
            }
            let ran: Vec<Step> = judged
                .iter()
                .flat_map(|job| {
                    legs.iter()
                        .find(|leg| leg.id == *job && leg.runner == runner)
                        .unwrap_or_else(|| panic!("{file}'s {job} has no {runner} machine"))
                        .steps
                        .clone()
                })
                .collect();
            for leg in legs
                .iter_mut()
                .filter(|leg| leg.id == verdict && leg.runner == runner)
            {
                leg.steps.splice(0..0, ran.iter().cloned());
            }
        }
    }
    legs
}

fn job_legs(file: &'static str, id: &str, body: &[&str]) -> Vec<WorkflowLeg> {
    let field = |key: &str| body.iter().find_map(|line| line.strip_prefix(key));
    let name = field("    name: ")
        .unwrap_or_else(|| panic!("{file}'s {id} has no name"))
        .trim_matches('\'');
    let runs_on = field("    runs-on: ").unwrap_or_else(|| panic!("{file}'s {id} has no runner"));
    // A matrix lists its systems, or names one per included machine.
    let mut included: Vec<&str> = body
        .iter()
        .filter_map(|line| line.trim().strip_prefix("- os: "))
        .collect();
    included.dedup();
    let oses: Vec<&str> = body
        .iter()
        .find_map(|line| line.trim().strip_prefix("os: ["))
        .map_or_else(
            || match included.is_empty() {
                true => vec![runs_on],
                false => included.clone(),
            },
            |list| list.trim_end_matches(']').split(", ").collect(),
        );
    let steps = job_steps(body);
    let job_env: Vec<&str> = body
        .iter()
        .copied()
        .take_while(|line| *line != "    steps:")
        .collect();
    let job_env = env_map(&job_env);
    oses.into_iter()
        .map(|os| {
            let runner = if os.starts_with("macos") {
                "macOS"
            } else {
                "Linux"
            };
            // A line that names a runner holds only on that runner's leg.
            let on_leg = |line: &str| {
                !line.contains("runner.os ==") || line.contains(&format!("runner.os == '{runner}'"))
            };
            let taken: Vec<&Vec<&str>> = steps
                .iter()
                .filter(|step| {
                    step.iter()
                        .filter(|line| line.trim().trim_start_matches("- ").starts_with("if: "))
                        .all(|line| on_leg(line))
                })
                .collect();
            WorkflowLeg {
                check: name.replace("${{ matrix.os }}", os),
                id: id.to_string(),
                file,
                runner,
                steps: taken
                    .into_iter()
                    .map(|step| Step {
                        env: job_env.iter().cloned().chain(env_map(step)).collect(),
                        lines: step_lines(step),
                    })
                    .collect(),
            }
        })
        .collect()
}

/// A job's steps, each its own lines, comments dropped.
fn job_steps<'a>(body: &[&'a str]) -> Vec<Vec<&'a str>> {
    let mut steps: Vec<Vec<&str>> = Vec::new();
    for &line in body
        .iter()
        .skip_while(|line| **line != "    steps:")
        .skip(1)
    {
        if line.starts_with("      - ") {
            steps.push(vec![line]);
        } else if let Some(step) = steps
            .last_mut()
            .filter(|_| !line.trim_start().starts_with('#'))
        {
            step.push(line);
        }
    }
    steps
}

/// What one step runs: its one-line command, or every code line of its
/// multi-line block (continuations joined), a call to a function the block
/// defines standing for that function's body with `"$@"` replaced by the
/// call's arguments. A pull request's base stands as `origin/main`, the
/// base a local clone has.
fn step_lines(step: &[&str]) -> Vec<String> {
    let mut commands = Vec::new();
    for (at, line) in step.iter().enumerate() {
        let key = line.trim_start().trim_start_matches("- ");
        if key == "run: |" {
            let indent = line.len() - key.len();
            let block: Vec<&str> = step[at + 1..]
                .iter()
                .take_while(|line| {
                    line.trim().is_empty() || line.len() - line.trim_start().len() > indent
                })
                .map(|line| line.trim())
                .collect();
            commands.extend(block_lines(&block.join("\n").replace(" \\\n", " ")));
        } else if let Some(run) = key.strip_prefix("run: ") {
            commands.push(run.replace("\"$BASE\"", "origin/main"));
        } else if let Some(deny) = key.strip_prefix("command: ") {
            commands.push(format!("cargo deny {deny}"));
        }
    }
    commands
}

/// The code lines of one `run: |` block, its functions expanded.
fn block_lines(block: &str) -> Vec<String> {
    let mut functions: Vec<(&str, Vec<&str>)> = Vec::new();
    let mut commands = Vec::new();
    let mut lines = block.lines();
    while let Some(line) = lines.next() {
        if let Some(name) = line.strip_suffix("() {") {
            functions.push((
                name,
                lines.by_ref().take_while(|line| *line != "}").collect(),
            ));
        } else if let Some((body, args)) = functions.iter().find_map(|(name, body)| {
            line.strip_prefix(name)
                .and_then(|rest| rest.strip_prefix(' '))
                .map(|args| (body, args))
        }) {
            commands.extend(body.iter().map(|call| call.replace("\"$@\"", args)));
        } else if !line.is_empty() {
            commands.push(line.replace("'HEAD^1'", "origin/main"));
        }
    }
    commands
}

/// The commands a row writes: its cell's code spans, and every line of a
/// code block in a section it links.
fn written_commands(check: &str, guide: &str, cell: &str) -> Vec<String> {
    cell.split('`')
        .skip(1)
        .step_by(2)
        .chain(
            links(check, guide, cell)
                .into_iter()
                .flat_map(|link| code_lines(link.section)),
        )
        .map(str::to_string)
        .collect()
}

/// One link in a row's command cell: its text, and the section it points at.
struct Link<'a> {
    text: &'a str,
    anchor: &'a str,
    section: &'a str,
}

/// The guide sections `check`'s row links to from its command cell. A
/// link this cannot read, or one to a section the guide does not have,
/// fails the test with the row and the link.
fn links<'a>(check: &str, guide: &'a str, cell: &'a str) -> Vec<Link<'a>> {
    let parts: Vec<&str> = cell.split("](#").collect();
    parts
        .windows(2)
        .map(|pair| {
            let anchor = pair[1]
                .split_once(')')
                .unwrap_or_else(|| panic!("{check}'s row leaves a link to #{} open", pair[1]))
                .0;
            let text = pair[0]
                .rsplit_once('[')
                .unwrap_or_else(|| {
                    panic!("{check}'s row links #{anchor} from text this test cannot read")
                })
                .1;
            let section = guide
                .split("\n#")
                .find(|section| heading_anchor(section) == anchor)
                .unwrap_or_else(|| {
                    panic!("{check}'s row links #{anchor}, a section the guide does not have")
                });
            Link {
                text,
                anchor,
                section,
            }
        })
        .collect()
}

/// A link whose text is a command is a promise that the section writes
/// that command: each such link's command is a line of a code block in the
/// section it points at, so deleting the block that "this same command"
/// stands beside fails here. Link text that reads as a command but is not
/// one code span is refused as unreadable rather than taken for prose.
fn assert_linked_blocks(check: &str, cell: &str, guide: &str) {
    for link in links(check, guide, cell) {
        let unreadable = || {
            panic!(
                "{check}'s row links [{}] to #{}, which this test cannot read",
                link.text, link.anchor
            )
        };
        let Some(command) = link.text.strip_prefix('`') else {
            if text_reads_as_command(link.text) {
                unreadable();
            }
            continue;
        };
        let command = command
            .strip_suffix('`')
            .filter(|command| !command.contains('`'))
            .unwrap_or_else(unreadable);
        assert!(
            code_lines(link.section).contains(&command),
            "{check}'s row links `{command}` to #{}, whose code blocks do not write it",
            link.anchor
        );
    }
}

/// Every line of every fenced code block in `section`.
fn code_lines(section: &str) -> Vec<&str> {
    section
        .split("```")
        .skip(1)
        .step_by(2)
        .flat_map(str::lines)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
}

/// Code-block lines in a linked section that no CI leg runs, each with
/// the reason it is there: a local extra, not a check a job makes.
const LOCAL_ONLY: [&str; 3] = [
    // "If you touched a recipe … compile that one too": CI compiles the
    // two shipped bundles; a contributor compiles the one they changed.
    "cargo run --locked -p brokkr-cli -- library compile --bundle recipes/<name>",
    // The directory the coverage script keeps its warm build in: a path
    // the section names, not a command.
    "${BROKKR_COVERAGE_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}}/brokkr-coverage-cache",
    // The same gate with its temporary directories moved off a small tmpfs.
    "TMPDIR=/var/tmp BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 bash scripts/coverage-exact.sh --parallel-heaps",
];

/// The anchor GitHub gives the heading a section starts with.
fn heading_anchor(section: &str) -> String {
    let heading = section.lines().next().unwrap_or_default();
    heading
        .trim_start_matches('#')
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

/// One row of the by-hand guide's check table.
#[derive(Debug)]
struct CheckRow {
    number: String,
    check: String,
    job: String,
    file: &'static str,
    command: String,
}

fn check_rows(guide: &str) -> Vec<CheckRow> {
    let section = guide
        .split_once("\n## The twelve checks\n")
        .expect("the guide has its checks section")
        .1
        .split_once("\n## ")
        .expect("a section follows the checks")
        .0;
    section
        .lines()
        .filter(|line| line.starts_with("| ") && line.as_bytes()[2].is_ascii_digit())
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
            assert_eq!(cells.len(), 4, "one four-column check row: {line}");
            let (job, file) = ["mutants.yml", "pr-gates.yml"]
                .into_iter()
                .find_map(|file| {
                    cells[2]
                        .strip_suffix(&format!(" ({file})"))
                        .map(|job| (job, file))
                })
                .unwrap_or((cells[2], "ci.yml"));
            CheckRow {
                number: cells[0].to_string(),
                check: cells[1].trim_matches('`').to_string(),
                job: job.trim_matches('`').to_string(),
                file,
                command: cells[3].to_string(),
            }
        })
        .collect()
}

/// The workflows the required checks live in, in their order: the pull
/// request's metadata gates (#1295), the suite, and the mutants gate.
const REQUIRED_WORKFLOWS: [&str; 3] = ["pr-gates.yml", "ci.yml", "mutants.yml"];

/// The checks branch protection on main requires (operator rulings,
/// 2026-09-26), as (context, job id, workflow), in the workflows' order.
/// Branch protection lives outside the tree; its checked-in snapshot,
/// `.github/branch-protection.json`, holds these contexts, and
/// `contributing_lists_the_checks_branch_protection_requires` holds this
/// list to it.
const MAIN_REQUIRES: [(&str, &str, &str); 12] = [
    ("delivered by brokkr", "delivered-by-brokkr", "pr-gates.yml"),
    ("baseline ratchets", "ratchets", "pr-gates.yml"),
    ("MSRV (1.89)", "msrv", "ci.yml"),
    ("format, clippy, contracts", "quality", "ci.yml"),
    ("test (ubuntu-latest)", "engine", "ci.yml"),
    ("test (macos-latest)", "engine", "ci.yml"),
    ("exact coverage gate", "coverage-gate", "ci.yml"),
    (
        "dependency licenses (cargo-deny)",
        "license-compliance",
        "ci.yml",
    ),
    ("non-Rust lints", "lint-non-rust", "ci.yml"),
    ("RustSec dependency audit", "dependency-audit", "ci.yml"),
    ("release binary artifact", "release-binary", "ci.yml"),
    (
        "mutants in the diff: brokkr-core",
        "core-gate",
        "mutants.yml",
    ),
];

/// The by-hand guide's check table is exactly the twelve checks main
/// requires, and every row is a check the workflows define: its name and
/// job as the job states them, in the workflows' order, and, line for
/// line, the commands its leg of the job runs with the variables they run
/// under, written in the row's code spans and the sections it links.
#[test]
fn the_by_hand_checks_are_the_workflows_checks() {
    let root = workspace();
    let guide = std::fs::read_to_string(root.join("docs/guides/contributing-by-hand.md")).unwrap();
    let rows = check_rows(&guide);
    let listed: Vec<(String, String, &str)> = rows
        .iter()
        .map(|row| (row.check.clone(), row.job.clone(), row.file))
        .collect();
    let required: Vec<(String, String, &str)> = MAIN_REQUIRES
        .iter()
        .map(|&(check, job, file)| (check.to_string(), job.to_string(), file))
        .collect();
    assert_eq!(
        listed, required,
        "the guide lists the twelve checks main requires"
    );
    let legs: Vec<WorkflowLeg> = REQUIRED_WORKFLOWS
        .into_iter()
        .flat_map(|file| workflow_legs(&root, file))
        .collect();
    let defined: Vec<(String, String, &str)> = legs
        .iter()
        .map(|leg| (leg.check.clone(), leg.id.clone(), leg.file))
        .filter(|check| listed.contains(check))
        .collect();
    assert_eq!(
        listed, defined,
        "a row names a check its workflow does not define"
    );
    let runs: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            legs.iter()
                .find(|leg| (&leg.check, &leg.id, leg.file) == (&row.check, &row.job, row.file))
                .expect("a defined leg")
                .local_commands()
        })
        .collect();
    assert_first_words_listed(&root, &runs);
    assert_inline_only_used(&rows, &guide);
    let mut every_written = Vec::new();
    for (at, (row, runs)) in rows.iter().zip(&runs).enumerate() {
        let check = &row.check;
        assert_eq!(
            row.number,
            (at + 1).to_string(),
            "{check} is numbered in order"
        );
        assert_linked_blocks(check, &row.command, &guide);
        assert_inline_copies(&root, row, &guide, runs);
        let written = written_commands(check, &guide, &row.command);
        for run in runs {
            assert!(
                written.contains(run),
                "{check}'s row does not write `{run}`"
            );
        }
        // And the other way: a command the row writes is one its leg runs,
        // so a step deleted from the workflow cannot leave the guide
        // promising it.
        for line in &written {
            assert!(
                LOCAL_ONLY.contains(&line.as_str()) || runs.contains(line),
                "{check}'s row writes `{line}`, which its job does not run"
            );
        }
        every_written.extend(written);
    }
    // A local extra is a copy too: deleting the block that writes one
    // fails, as a stale allowance does.
    for line in LOCAL_ONLY {
        assert!(
            every_written.iter().any(|written| written == line),
            "no row writes `{line}`, which LOCAL_ONLY holds as a local extra"
        );
    }
    let contributing = std::fs::read_to_string(root.join("CONTRIBUTING.md")).unwrap();
    assert!(contributing.contains("preserves the twelve exact checks"));
    assert!(guide.contains("twelve required checks"));
}

/// `.github/branch-protection.json`: the checked-in snapshot of what
/// branch protection on `main` requires. A test cannot call the GitHub
/// API offline, so the settings are read from this file, and a key it
/// does not know is refused.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Protection {
    branch: String,
    observed: String,
    source: String,
    required_status_checks: StatusChecks,
    required_signatures: bool,
    required_approving_review: bool,
    enforce_admins: bool,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusChecks {
    strict: bool,
    contexts: Vec<String>,
}

/// "requires {what}" or "does not require {what}".
fn requires(yes: bool, what: &str) -> String {
    match yes {
        true => format!("requires {what}"),
        false => format!("does not require {what}"),
    }
}

/// CONTRIBUTING's required-check paragraph: the required contexts in the
/// workflows' order, then each other setting the snapshot records.
fn required_checks_paragraph(snapshot: &Protection, ordered: &[&str]) -> String {
    let (last, rest) = ordered.split_last().expect("main requires a check");
    let rest: Vec<String> = rest.iter().map(|check| format!("`{check}`")).collect();
    format!(
        "`{branch}`'s branch protection, as observed on {observed} \
         ([snapshot](.github/branch-protection.json)), requires {count} checks: {rest}, and \
         `{last}`. It {signatures}, {strict}, {review}, and {admins} administrators.\n",
        branch = snapshot.branch,
        observed = snapshot.observed,
        count = ordered.len(),
        rest = rest.join(", "),
        signatures = requires(snapshot.required_signatures, "signed commits"),
        strict = requires(
            snapshot.required_status_checks.strict,
            "a branch to be up to date with `main`"
        ),
        review = requires(snapshot.required_approving_review, "an approving review"),
        admins = match snapshot.enforce_admins {
            true => "applies to",
            false => "does not apply to",
        },
    )
}

/// CONTRIBUTING's required-check list is generated from the workflows and
/// the branch-protection snapshot (#366): every context the snapshot
/// requires is a check a workflow defines, listed in the workflows'
/// order, and `MAIN_REQUIRES` names the same contexts in that order.
#[test]
fn contributing_lists_the_checks_branch_protection_requires() {
    let root = workspace();
    let snapshot: Protection = serde_json::from_str(
        &std::fs::read_to_string(root.join(".github/branch-protection.json")).unwrap(),
    )
    .expect("the branch-protection snapshot parses");
    assert!(
        snapshot
            .source
            .starts_with("GET /repos/feedback-loop-ai/brokkr/branches/main/protection"),
        "the snapshot names the API it transcribes"
    );
    let contexts = &snapshot.required_status_checks.contexts;
    let legs: Vec<WorkflowLeg> = REQUIRED_WORKFLOWS
        .into_iter()
        .flat_map(|file| workflow_legs(&root, file))
        .collect();
    let mut ordered: Vec<&str> = legs
        .iter()
        .map(|leg| leg.check.as_str())
        .filter(|check| contexts.iter().any(|context| context == check))
        .collect();
    ordered.dedup();
    let mut defined = ordered.clone();
    defined.sort_unstable();
    let mut required: Vec<&str> = contexts.iter().map(String::as_str).collect();
    required.sort_unstable();
    assert_eq!(
        defined, required,
        "branch protection requires a check no workflow defines, or names one twice"
    );
    let listed: Vec<&str> = MAIN_REQUIRES.iter().map(|&(check, _, _)| check).collect();
    assert_eq!(
        listed, ordered,
        "MAIN_REQUIRES drifted from the branch-protection snapshot"
    );

    let paragraph = required_checks_paragraph(&snapshot, &ordered);
    let contributing = std::fs::read_to_string(root.join("CONTRIBUTING.md")).unwrap();
    let written = contributing
        .split_once("<!-- required-checks:start -->\n")
        .expect("CONTRIBUTING opens its required-check list")
        .1
        .split_once("<!-- required-checks:end -->")
        .expect("CONTRIBUTING closes its required-check list")
        .0;
    assert_eq!(
        written, paragraph,
        "CONTRIBUTING's required checks drifted; the rendering is:\n{paragraph}"
    );
}

/// What a job or a step does on `merge_group`, the event the merge queue
/// raises for the commit it would land: `main` plus the queued pull
/// requests (#451).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InTheQueue {
    /// Nothing stops it, so it judges the combined tree.
    Runs,
    /// It judges a pull request's own head or description, and skips. A
    /// skipped job or step reports success, which satisfies a required
    /// check, so each is named with its reason.
    PullRequestOnly,
    /// It runs on its schedule or by hand, never in the queue.
    Scheduled,
    /// It runs on the push of a commit to `main` alone, a commit the queue
    /// already judged, and saves what a pull request's run restores
    /// (#1295).
    PushOnly,
}

/// The other events a condition lets its job or step run on: a pull
/// request's, and the push of a commit to `main` (#1295).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fires {
    pull_request: bool,
    push: bool,
}

/// Every event but the merge queue's, which `InTheQueue` holds.
const EVERY: Fires = Fires {
    pull_request: true,
    push: true,
};

/// A pull request's events, and no push.
const NOT_ON_PUSH: Fires = Fires {
    pull_request: true,
    push: false,
};

/// The `if:` conditions this test reads, what each does in a merge group,
/// and the other events it runs on. Any other condition fails the test
/// rather than being guessed at.
const CONDITIONS: [(&str, InTheQueue, Fires); 18] = [
    // The docs lane (#1295): the classifier calls a merge group, and every
    // event but a pull request's, full, so the machines run there and the
    // docs lane does not.
    (
        "github.event_name != 'push' && needs.classify.outputs.lane != 'docs'",
        InTheQueue::Runs,
        NOT_ON_PUSH,
    ),
    (
        "needs.classify.outputs.lane != 'docs'",
        InTheQueue::Runs,
        EVERY,
    ),
    (
        "needs.classify.outputs.lane == 'docs'",
        InTheQueue::PullRequestOnly,
        NOT_ON_PUSH,
    ),
    (
        "github.event_name == 'pull_request'",
        InTheQueue::PullRequestOnly,
        NOT_ON_PUSH,
    ),
    (
        "github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'",
        InTheQueue::Scheduled,
        Fires {
            pull_request: false,
            push: false,
        },
    ),
    // The push of a commit the queue already judged runs only what nothing
    // else feeds (#1295).
    ("github.event_name != 'push'", InTheQueue::Runs, NOT_ON_PUSH),
    (
        "always() && github.event_name != 'push'",
        InTheQueue::Runs,
        NOT_ON_PUSH,
    ),
    (
        "github.event_name == 'push'",
        InTheQueue::PushOnly,
        Fires {
            pull_request: false,
            push: true,
        },
    ),
    // An exact cache hit leaves nothing to save, so its build is skipped.
    (
        "steps.cache.outputs.cache-hit != 'true'",
        InTheQueue::Runs,
        EVERY,
    ),
    // Both runner kinds execute the same checks in the queue. Only their
    // provisioning and cache steps differ.
    (
        "runner.environment == 'self-hosted'",
        InTheQueue::Runs,
        EVERY,
    ),
    (
        "runner.environment != 'self-hosted'",
        InTheQueue::Runs,
        EVERY,
    ),
    (
        "runner.os == 'Linux' && runner.environment != 'self-hosted'",
        InTheQueue::Runs,
        EVERY,
    ),
    ("runner.os == 'Linux'", InTheQueue::Runs, EVERY),
    ("runner.os == 'macOS'", InTheQueue::Runs, EVERY),
    ("always()", InTheQueue::Runs, EVERY),
    ("always() && runner.os == 'macOS'", InTheQueue::Runs, EVERY),
    // The macOS startup gate runs on one of the suite's machines, the
    // first; the queue runs every machine.
    (
        "runner.os == 'macOS' && matrix.machine == 'macos/1'",
        InTheQueue::Runs,
        EVERY,
    ),
    (
        "always() && runner.os == 'macOS' && matrix.machine == 'macos/1'",
        InTheQueue::Runs,
        EVERY,
    ),
];

/// Every job of the workflows the required checks live in, in file order,
/// and what it does in a merge group.
const JOBS_IN_THE_QUEUE: [(&str, &str, InTheQueue); 26] = [
    // It judges the run that the pull request's own description names for
    // its own head (decisions 0033, 0038), and a merge group carries no
    // description, labels or head repository. The queue lands that head's
    // patch, which the judgment bound.
    (
        "pr-gates.yml",
        "delivered-by-brokkr",
        InTheQueue::PullRequestOnly,
    ),
    ("pr-gates.yml", "ratchets", InTheQueue::Runs),
    ("ci.yml", "msrv", InTheQueue::Runs),
    ("ci.yml", "quality", InTheQueue::Runs),
    // The console plugin's tests (decision 0085 ruling 5) test the landed
    // bytes like the Rust suite does.
    ("ci.yml", "plugin-tests", InTheQueue::Runs),
    // It calls a merge group full (#1295), the queue being the last check
    // before main and able to hold several pull requests.
    ("ci.yml", "classify", InTheQueue::Runs),
    // In no required check: the tests that read documentation, on a pull
    // request's docs change alone. A merge group is full, so the verdicts
    // read the machines there.
    ("ci.yml", "docs-lane", InTheQueue::PullRequestOnly),
    ("ci.yml", "engine-machines", InTheQueue::Runs),
    // The suite's verdicts run whatever the machines ended with.
    ("ci.yml", "engine", InTheQueue::Runs),
    ("ci.yml", "seatbelt-lifetime", InTheQueue::Runs),
    ("ci.yml", "coverage-machines", InTheQueue::Runs),
    ("ci.yml", "coverage", InTheQueue::Runs),
    // The gate runs whatever the report ended with.
    ("ci.yml", "coverage-gate", InTheQueue::Runs),
    ("ci.yml", "license-compliance", InTheQueue::Runs),
    ("ci.yml", "lint-non-rust", InTheQueue::Runs),
    ("ci.yml", "dependency-audit", InTheQueue::Runs),
    ("ci.yml", "bootstrap-budgets", InTheQueue::Runs),
    ("ci.yml", "bootstrap-budgets-macos", InTheQueue::Runs),
    ("ci.yml", "packaging", InTheQueue::Runs),
    ("ci.yml", "flake", InTheQueue::Runs),
    ("ci.yml", "release-binary", InTheQueue::Runs),
    // In no required check: it saves, from main, the caches a pull
    // request's jobs restore, which a merge group's cache cannot feed.
    ("ci.yml", "pr-caches", InTheQueue::PushOnly),
    // It judges the mutants of the pull request's own diff against its
    // base, and the queue lands that diff. A miss two pull requests make
    // only together is left to the weekly run.
    ("mutants.yml", "core-gate", InTheQueue::PullRequestOnly),
    // In no required check: brokkr-protocol's report on the pull
    // request's own diff.
    ("mutants.yml", "in-diff", InTheQueue::PullRequestOnly),
    ("mutants.yml", "weekly", InTheQueue::Scheduled),
    ("mutants.yml", "weekly-report", InTheQueue::Scheduled),
];

/// Every step that does not run in a merge group, as (workflow, job,
/// step): the rest of its job judges the combined tree.
const STEPS_OUT_OF_THE_QUEUE: [(&str, &str, &str, InTheQueue); 2] = [
    // It reads the `Ruling:` line of the pull request's description, which
    // a merge group does not carry. The file-size and duplication ratchets
    // before it judge the combined tree against the baselines.
    (
        "pr-gates.yml",
        "ratchets",
        "a raised baseline names its ruling (quality/)",
        InTheQueue::PullRequestOnly,
    ),
    // It reads the suppressions the pull request's own diff adds against
    // its base. The tree's counts, which two pull requests could raise
    // together, ride the test jobs (quality/suppressions.txt).
    (
        "ci.yml",
        "quality",
        "an added suppression names a ruling",
        InTheQueue::PullRequestOnly,
    ),
];

/// What `condition`, an `if:` at `site`, does in a merge group, and the
/// other events it runs on.
fn read_condition(condition: Option<&str>, site: &str) -> (InTheQueue, Fires) {
    condition.map_or((InTheQueue::Runs, EVERY), |condition| {
        CONDITIONS
            .iter()
            .find(|(known, _, _)| *known == condition)
            .map(|&(_, queue, fires)| (queue, fires))
            .unwrap_or_else(|| panic!("{site} runs `if: {condition}`, which this test cannot read"))
    })
}

/// What `condition`, an `if:` at `site`, does in a merge group.
fn in_the_queue(condition: Option<&str>, site: &str) -> InTheQueue {
    read_condition(condition, site).0
}

/// The jobs a job's body `needs:`, one id or a `[a, b]` list; an entry
/// this cannot read fails the test.
fn needs_of(body: &str, site: &str) -> Vec<String> {
    let Some(needed) = body
        .lines()
        .find_map(|line| line.strip_prefix("    needs:"))
        .map(str::trim)
    else {
        return Vec::new();
    };
    let list = needed
        .strip_prefix('[')
        .and_then(|list| list.strip_suffix(']'))
        .unwrap_or(needed);
    list.split(", ")
        .map(|id| {
            assert!(
                !id.is_empty()
                    && id
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "{site} needs `{needed}`, which this test cannot read"
            );
            id.to_string()
        })
        .collect()
}

/// The job `id` of `jobs` as a site to name, its body, and its own `if:`.
fn job_and_condition<'a>(
    file: &str,
    jobs: &'a [(String, String)],
    id: &str,
) -> (String, &'a str, Option<&'a str>) {
    let body = jobs
        .iter()
        .find_map(|(job, body)| (job == id).then_some(body.as_str()))
        .unwrap_or_else(|| panic!("{file} has no {id} job"));
    let own = body.lines().find_map(|line| line.strip_prefix("    if: "));
    (format!("{file}'s {id}"), body, own)
}

/// Whether the job `id` of `jobs` runs on the event `on` picks: its own
/// `if:`, and then each job it `needs:`, whose skip it carries unless its
/// own condition is `always()`.
fn job_fires(file: &str, jobs: &[(String, String)], id: &str, on: fn(Fires) -> bool) -> bool {
    let (site, body, own) = job_and_condition(file, jobs, id);
    let fires = on(read_condition(own, &site).1);
    match fires && !own.is_some_and(|own| own.starts_with("always()")) {
        true => needs_of(body, &site)
            .iter()
            .all(|needed| job_fires(file, jobs, needed, on)),
        false => fires,
    }
}

/// What the job `id` of `jobs` does in a merge group: its own `if:`, and
/// then each job it `needs:`, whose skip carries into it unless its own
/// condition is `always()`.
fn job_in_the_queue(file: &str, jobs: &[(String, String)], id: &str) -> InTheQueue {
    let (site, body, condition) = job_and_condition(file, jobs, id);
    let own = in_the_queue(condition, &site);
    let needs = needs_of(body, &site);
    if own != InTheQueue::Runs || condition.is_some_and(|own| own.starts_with("always()")) {
        return own;
    }
    needs
        .iter()
        .map(|needed| job_in_the_queue(file, jobs, needed))
        .find(|upstream| *upstream != InTheQueue::Runs)
        .unwrap_or(own)
}

/// The steps of the job `id` that do not run in a merge group, each named
/// by its `name:`, or by its first line when it has none.
fn steps_out_of_the_queue(
    file: &'static str,
    id: &str,
    body: &str,
) -> Vec<(&'static str, String, String, InTheQueue)> {
    let site = format!("a step of {file}'s {id}");
    job_steps(&body.lines().collect::<Vec<_>>())
        .into_iter()
        .filter_map(|step| {
            let key = |key: &str| {
                step.iter().find_map(|line| {
                    line.strip_prefix(&format!("      - {key}: "))
                        .or_else(|| line.strip_prefix(&format!("        {key}: ")))
                })
            };
            let queue = in_the_queue(key("if"), &site);
            let name = key("name").unwrap_or_else(|| step[0].trim());
            (queue != InTheQueue::Runs).then(|| (file, id.to_string(), name.to_string(), queue))
        })
        .collect()
}

/// The lines under `event` in a workflow's `on:` map, comments dropped.
fn event_lines<'a>(workflow: &'a str, event: &str) -> Vec<&'a str> {
    let (_, on) = workflow.split_once("\non:\n").expect("an on: map");
    let key = format!("  {event}:");
    on.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .take_while(|line| line.starts_with("  "))
        .skip_while(|line| *line != key)
        .skip(1)
        .take_while(|line| line.starts_with("    "))
        .collect()
}

/// The merge queue re-runs the required checks on the commit that lands
/// (#451): each workflow a required check lives in runs on `merge_group`,
/// every job and step of it does in a queue what `JOBS_IN_THE_QUEUE` and
/// `STEPS_OUT_OF_THE_QUEUE` hold, each skip with its reason, and no
/// required check's job waits on a schedule that a queue never raises.
#[test]
fn every_job_behaves_deliberately_in_a_merge_group() {
    let root = workspace();
    let mut files: Vec<&str> = MAIN_REQUIRES.iter().map(|&(_, _, file)| file).collect();
    files.dedup();
    assert_eq!(files, REQUIRED_WORKFLOWS);
    let (mut jobs_seen, mut steps_seen) = (Vec::new(), Vec::new());
    for file in files {
        let text = std::fs::read_to_string(root.join(".github/workflows").join(file)).unwrap();
        assert_eq!(
            event_lines(&text, "merge_group"),
            ["    types: [checks_requested]"],
            "{file} does not run on the merge queue's event"
        );
        let jobs = workflow::jobs(&text);
        for (id, body) in &jobs {
            jobs_seen.push((file, id.clone(), job_in_the_queue(file, &jobs, id)));
            steps_seen.extend(steps_out_of_the_queue(file, id, body));
        }
    }
    let jobs_held: Vec<(&str, String, InTheQueue)> = JOBS_IN_THE_QUEUE
        .iter()
        .map(|&(file, id, queue)| (file, id.to_string(), queue))
        .collect();
    assert_eq!(
        jobs_seen, jobs_held,
        "a job's part in the merge queue changed: re-read JOBS_IN_THE_QUEUE's reasons"
    );
    let steps_held: Vec<(&str, String, String, InTheQueue)> = STEPS_OUT_OF_THE_QUEUE
        .iter()
        .map(|&(file, id, step, queue)| (file, id.to_string(), step.to_string(), queue))
        .collect();
    assert_eq!(
        steps_seen, steps_held,
        "a step's part in the merge queue changed: re-read STEPS_OUT_OF_THE_QUEUE's reasons"
    );
    for (check, id, file) in MAIN_REQUIRES {
        let queue = jobs_seen
            .iter()
            .find_map(|(seen, job, queue)| (*seen == file && job == id).then_some(*queue))
            .unwrap_or_else(|| panic!("{file} has no {id} job"));
        assert!(
            matches!(queue, InTheQueue::Runs | InTheQueue::PullRequestOnly),
            "{check} neither runs nor skips deliberately in a merge group: {queue:?}"
        );
    }
}

/// The jobs of ci.yml that run on the push of a commit to `main` (#1295),
/// in file order: the fast gates, so the README's badge reads a verdict
/// on main, and the builds that save the caches a merge group's and a
/// pull request's run restore. The coverage machines and gate are among
/// them because only their measurement makes the instrumented build their
/// caches hold, with the classifier they need, which calls a push full,
/// and the gate over their report. Everything else that commit already
/// passed in the queue.
const JOBS_ON_PUSH: [&str; 11] = [
    "msrv",
    "quality",
    "classify",
    "coverage-machines",
    "coverage",
    "coverage-gate",
    "license-compliance",
    "lint-non-rust",
    "dependency-audit",
    "release-binary",
    "pr-caches",
];

/// What no step on the push to `main` may run: the suite, the CPU and
/// bootstrap benches, or a test that is not `--no-run`.
const NOT_ON_PUSH_RUNS: [&str; 3] = [
    "scripts/test-shards.sh",
    "cargo bench",
    "scripts/bootstrap-bench.sh",
];

/// The jobs whose steps may run the coverage measurement on the push to
/// `main`: the coverage machines and gate, which save its caches.
const COVERAGE_ON_PUSH: [&str; 2] = ["coverage-machines", "coverage"];

/// The push to `main` after each merge runs only what it uniquely feeds
/// (#1295): `JOBS_ON_PUSH`, none of whose steps on `push` runs a test,
/// Callgrind or bootstrap measure, and only the coverage jobs the coverage
/// measurement. And every required check's job still runs on a pull
/// request, and runs or reports in a merge group.
#[test]
fn the_push_to_main_runs_only_what_it_uniquely_feeds() {
    let root = workspace();
    let text = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    let jobs = workflow::jobs(&text);
    let on_push: Vec<&str> = jobs
        .iter()
        .map(|(id, _)| id.as_str())
        .filter(|id| job_fires("ci.yml", &jobs, id, |fires| fires.push))
        .collect();
    assert_eq!(on_push, JOBS_ON_PUSH, "ci.yml's jobs on the push to main");
    for (id, body) in jobs.iter().filter(|(id, _)| on_push.contains(&id.as_str())) {
        let site = format!("a step of ci.yml's {id}");
        for step in job_steps(&body.lines().collect::<Vec<_>>()) {
            let condition = step.iter().find_map(|line| {
                line.strip_prefix("      - if: ")
                    .or_else(|| line.strip_prefix("        if: "))
            });
            if !read_condition(condition, &site).1.push {
                continue;
            }
            for line in step {
                assert!(
                    !NOT_ON_PUSH_RUNS.iter().any(|run| line.contains(run))
                        && (!line.contains("cargo test") || line.contains("--no-run"))
                        && (!line.contains("scripts/coverage-exact.sh")
                            || COVERAGE_ON_PUSH.contains(&id.as_str())),
                    "{id} runs `{}` on the push to main",
                    line.trim()
                );
            }
        }
    }
    for (check, id, file) in MAIN_REQUIRES {
        let text = std::fs::read_to_string(root.join(".github/workflows").join(file)).unwrap();
        let jobs = workflow::jobs(&text);
        assert!(
            job_fires(file, &jobs, id, |fires| fires.pull_request),
            "{check} does not run on a pull request"
        );
        assert!(
            matches!(
                job_in_the_queue(file, &jobs, id),
                InTheQueue::Runs | InTheQueue::PullRequestOnly
            ),
            "{check} neither runs nor reports in a merge group"
        );
    }
}

/// Where `pr-caches` fails to warm the very cache a job restores, one
/// problem a line (#1295). Its matrix keys and its `case` arms are the same
/// set; its rust-cache step is keyed by the entry's key; and each key names
/// a ci.yml job on the entry's system, on the same toolchain, keyed by its
/// own id (no other `shared-key`), whose build, `--no-run` aside, is the
/// arm's: in its own steps, or in scripts/test-shards.sh, which the test
/// machines run.
fn pr_caches_unbound(ci: &str, shards: &str) -> Vec<String> {
    let jobs = workflow::jobs(ci);
    let body = |id: &str| {
        jobs.iter()
            .find(|(seen, _)| seen == id)
            .map(|(_, body)| body.as_str())
    };
    let Some(warm) = body("pr-caches") else {
        return vec!["ci.yml has no pr-caches job".to_string()];
    };
    let lines: Vec<&str> = warm.lines().map(str::trim).collect();
    let entries: Vec<(&str, &str)> = lines
        .windows(2)
        .filter_map(|pair| {
            Some((
                pair[0].strip_prefix("- key: ")?,
                pair[1].strip_prefix("os: ")?,
            ))
        })
        .collect();
    let arms: BTreeMap<&str, &str> = lines
        .iter()
        .filter_map(|line| {
            let (key, command) = line.strip_suffix(" ;;")?.split_once(") ")?;
            (key != "*").then_some((key, command))
        })
        .collect();
    let toolchain = |text: &str| -> Vec<String> {
        text.lines()
            .filter_map(|line| line.trim().strip_prefix("toolchain: "))
            .map(String::from)
            .collect()
    };
    let mut problems = Vec::new();
    if !lines.contains(&"shared-key: ${{ matrix.key }}") {
        problems.push("pr-caches' rust-cache is not keyed by its entry's key".to_string());
    }
    let keys: std::collections::BTreeSet<&str> = entries.iter().map(|(key, _)| *key).collect();
    if keys != arms.keys().copied().collect() {
        problems.push(format!(
            "pr-caches warms {keys:?} and builds {:?}",
            arms.keys()
        ));
    }
    for (key, os) in entries {
        let Some(job) = body(key) else {
            problems.push(format!("pr-caches warms {key}, and ci.yml has no such job"));
            continue;
        };
        if !job.contains("Swatinem/rust-cache@") {
            problems.push(format!("{key} restores no rust-cache"));
        }
        let mut shared = job
            .lines()
            .filter_map(|line| line.trim().strip_prefix("shared-key: "));
        if let Some(other) = shared.find(|shared| *shared != key) {
            problems.push(format!(
                "{key}'s rust-cache is keyed {other}, not by its id"
            ));
        }
        if !job.contains(os) {
            problems.push(format!(
                "pr-caches warms {key} on {os}, and {key} does not run there"
            ));
        }
        if toolchain(job) != toolchain(warm) {
            problems.push(format!("{key}'s toolchain is not pr-caches' toolchain"));
        }
        let build = arms
            .get(key)
            .map(|command| command.replace(" --no-run", ""));
        let source = if job.contains("scripts/test-shards.sh") {
            format!("{job}{shards}")
        } else {
            job.to_string()
        };
        if !build.is_some_and(|build| source.contains(&build)) {
            problems.push(format!(
                "pr-caches' build for {key} is not what {key} builds"
            ));
        }
    }
    problems
}

/// `pr-caches` warms the very key and build each job it names restores
/// (#1295), so a renamed job, another `shared-key` or a changed build
/// cannot leave a cache unwarmed unseen. Two altered copies are the
/// control: an entry naming no job, and a build that is not the job's.
#[test]
fn pr_caches_warms_each_jobs_own_cache() {
    let root = workspace();
    let ci = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    let shards = std::fs::read_to_string(root.join("scripts/test-shards.sh")).unwrap();
    assert_eq!(pr_caches_unbound(&ci, &shards), Vec::<String>::new());
    let altered = [
        (
            "          - key: packaging\n",
            "          - key: packages\n",
        ),
        (
            "bootstrap-budgets-macos) cargo build --release --locked -p brokkr-cli ;;",
            "bootstrap-budgets-macos) cargo build --release --locked -p brokkr-core ;;",
        ),
    ];
    for (from, to) in altered {
        assert_eq!(ci.matches(from).count(), 1, "{from}");
        let problems = pr_caches_unbound(&ci.replace(from, to), &shards);
        assert!(!problems.is_empty(), "{to} passes");
    }
}

/// A label or a body change starts no suite run (#1295): ci.yml runs on a
/// pull request's new head alone, and pr-gates.yml, which holds the two
/// required checks that read its labels and body, runs on those changes
/// too, and on the merge queue's event so both still report there.
#[test]
fn a_label_or_body_change_reruns_only_the_metadata_gates() {
    let root = workspace();
    let read =
        |file: &str| std::fs::read_to_string(root.join(".github/workflows").join(file)).unwrap();
    let (ci, gates) = (read("ci.yml"), read("pr-gates.yml"));
    assert_eq!(
        event_lines(&ci, "pull_request"),
        ["    types: [opened, synchronize, reopened]"],
        "ci.yml starts on a label or a body change"
    );
    assert_eq!(
        event_lines(&gates, "pull_request"),
        ["    types: [opened, synchronize, reopened, edited, labeled, unlabeled]"],
    );
    assert_eq!(
        event_lines(&gates, "merge_group"),
        ["    types: [checks_requested]"],
        "pr-gates.yml does not run on the merge queue's event"
    );
    let ids: Vec<String> = workflow::jobs(&gates)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let held: Vec<&str> = MAIN_REQUIRES
        .iter()
        .filter(|&&(_, _, file)| file == "pr-gates.yml")
        .map(|&(_, id, _)| id)
        .collect();
    assert_eq!(ids, held, "pr-gates.yml holds the metadata gates alone");
}

/// The events a workflow's `on:` map names, in order, comments dropped.
fn events(workflow: &str) -> Vec<&str> {
    let (_, on) = workflow.split_once("\non:\n").expect("an on: map");
    on.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .take_while(|line| line.starts_with("  "))
        .filter_map(|line| line.strip_prefix("  "))
        .filter(|line| !line.starts_with(' '))
        .map(|line| line.trim_end_matches(':'))
        .collect()
}

/// The CPU budgets run nightly and by hand alone (#1295): cpu-budgets.yml
/// starts on `schedule` and `workflow_dispatch` and on nothing else, never
/// on a pull request's or the merge queue's event.
#[test]
fn the_cpu_budgets_run_on_their_schedule_or_by_hand_alone() {
    let text =
        std::fs::read_to_string(workspace().join(".github/workflows/cpu-budgets.yml")).unwrap();
    assert_eq!(events(&text), ["schedule", "workflow_dispatch"]);
    assert_eq!(
        event_lines(&text, "workflow_dispatch")[..2],
        ["    inputs:", "      ref:"],
        "a person cannot name the ref to measure"
    );
}

/// What the CPU budgets' measure leaves a trace by: Valgrind, gungraun's
/// runner, a bench run and the limit it fails over.
const CALLGRIND_TRACES: [&str; 4] = ["valgrind", "gungraun", "cargo bench", "--callgrind-limits"];

/// The workflows among `files` that start on a pull request's or the merge
/// queue's event and measure Callgrind, each as (file, line).
fn callgrind_on_the_pull_request_path(files: &[(String, String)]) -> Vec<(String, String)> {
    files
        .iter()
        .filter(|(_, text)| {
            events(text)
                .iter()
                .any(|event| ["pull_request", "merge_group"].contains(event))
        })
        .flat_map(|(file, text)| {
            text.lines()
                .filter(|line| !line.trim_start().starts_with('#'))
                .filter(|line| {
                    CALLGRIND_TRACES
                        .iter()
                        .any(|trace| line.to_lowercase().contains(trace))
                })
                .map(|line| (file.to_string(), line.trim().to_string()))
        })
        .collect()
}

/// No pull request's or merge group's run measures Callgrind (#1295): the
/// nightly does. Every workflow is read, ci.yml among them, and a ci.yml
/// with a bench run planted back in is the control.
#[test]
fn no_pull_request_or_merge_group_run_measures_callgrind() {
    let dir = workspace().join(".github/workflows");
    let mut files: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|file| file.ends_with(".yml"))
        .map(|file| {
            let text = std::fs::read_to_string(dir.join(&file)).unwrap();
            (file, text)
        })
        .collect();
    files.sort();
    let read: Vec<&str> = files.iter().map(|(file, _)| file.as_str()).collect();
    for known in ["ci.yml", "cpu-budgets.yml", "mutants.yml", "pr-gates.yml"] {
        assert!(read.contains(&known), "{known} was not read: {read:?}");
    }
    assert_eq!(callgrind_on_the_pull_request_path(&files), []);
    let ci = files.iter_mut().find(|(file, _)| file == "ci.yml").unwrap();
    ci.1.push_str("      - run: cargo bench -p brokkr-cli --bench budgets_lib\n");
    assert_eq!(
        callgrind_on_the_pull_request_path(&files),
        [(
            "ci.yml".to_string(),
            "- run: cargo bench -p brokkr-cli --bench budgets_lib".to_string()
        )]
    );
}

/// Every version the by-hand guide states for a pinned tool is its pin:
/// each `tool: name@version` the workflows install, the MSRV toolchain,
/// the Node they set up, the cargo-public-api they build, jscpd's action, the cargo-deny the
/// licence action's image carries, and the release of each action the
/// guide names. A pin moved without its copy in the guide fails here.
#[test]
fn the_by_hand_tool_versions_are_the_pins() {
    let root = workspace();
    let guide = std::fs::read_to_string(root.join("docs/guides/contributing-by-hand.md")).unwrap();
    let mut pins = Vec::new();
    for file in [
        ".github/workflows/ci.yml",
        ".github/workflows/mutants.yml",
        ".github/actions/setup-jscpd/action.yml",
    ] {
        let text = std::fs::read_to_string(root.join(file)).unwrap();
        pins.extend(text.lines().flat_map(|line| line_pins(line.trim(), &guide)));
    }
    assert!(pins.len() >= 12, "the pins were read: {pins:?}");
    for (name, version) in &pins {
        let stated: Vec<&str> = guide
            .match_indices(&format!("{name} "))
            .map(|(at, _)| version_token(&guide[at + name.len() + 1..]))
            .filter(|token| {
                token
                    .trim_start_matches('v')
                    .starts_with(|c: char| c.is_ascii_digit())
            })
            .collect();
        assert!(!stated.is_empty(), "the guide states no version of {name}");
        for token in stated {
            assert_eq!(token, version, "the guide states {name} {token}");
        }
    }
}

/// The pins one workflow line makes, named as the guide names the tool.
fn line_pins(line: &str, guide: &str) -> Vec<(String, String)> {
    if let Some(tools) = line.strip_prefix("tool: ") {
        return tools
            .split(',')
            .filter(|tool| !tool.contains("${{"))
            .map(|tool| {
                let (name, version) = tool.split_once('@').expect("a tool@version pin");
                (name.to_string(), version.to_string())
            })
            .collect();
    }
    let action = line
        .trim_start_matches("- ")
        .strip_prefix("uses: ")
        .and_then(|uses| uses.split_once('@'))
        .map(|(action, rest)| (format!("`{action}`"), rest.split_once(" # ")))
        .filter(|(action, _)| guide.contains(action.as_str()));
    let pin = if let Some((action, release)) = action {
        Some((action, release.expect("an action's release comment").1))
    } else if let Some(msrv) = line
        .strip_prefix("toolchain: ")
        .filter(|pin| pin.contains("# the MSRV"))
    {
        Some(("Rust".to_string(), version_token(msrv)))
    } else if let Some(version) = line.strip_prefix("node-version: ") {
        Some(("Node".to_string(), version))
    } else if let Some(version) = line.strip_prefix("JSCPD_VERSION: ") {
        Some(("jscpd".to_string(), version))
    } else if let Some(built) = line.strip_prefix("run: cargo install --locked ") {
        built
            .split_once(" --version ")
            .map(|(name, version)| (name.to_string(), version))
    } else {
        line.split_once("image carries cargo-deny ")
            .map(|(_, version)| ("cargo-deny".to_string(), version_token(version)))
    };
    pin.into_iter()
        .map(|(name, version)| (name, version.to_string()))
        .collect()
}

/// The version that starts `text`, without the punctuation after it.
fn version_token(text: &str) -> &str {
    let end = text
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '.')
        .unwrap_or(text.len());
    text[..end].trim_end_matches('.')
}

/// Decision 0046's guides: no page states that the network was off under
/// `harness` or `open`, and the decision carries its one-line erratum
/// under a heading of its own. Which sections and rows a page keeps is a
/// reviewer's call since #365: a false claim fails here, a lost section
/// does not.
#[test]
fn no_boundary_guide_says_the_network_was_off_and_0046_carries_its_erratum() {
    let root = workspace();
    for page in [
        "docs/guides/provider-adapters.md",
        "docs/guides/recipe-authoring.md",
        "docs/guides/quickstart.md",
        "docs/guides/journal-and-verification.md",
        "docs/guides/repository-layout.md",
        "docs/guides/driver-authoring.md",
        "ARCHITECTURE.md",
        "contracts/README.md",
    ] {
        let raw = std::fs::read_to_string(root.join(page)).unwrap();
        // Task 12.8 (design DD15): the prefix is a narrowing the engine
        // attempts on Linux, and no page says the network was off.
        let lower = raw
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        for claim in ["network was off", "network is off", "network off"] {
            assert!(
                !lower.contains(claim),
                "{page} states the network was off: {claim}"
            );
        }
    }

    // The erratum: one heading, one line, the decision otherwise untouched.
    // The accepted 2026-09-09 Seatbelt addendum follows the erratum, so the
    // erratum section is delimited by the next `##` heading, not the file end.
    let decision =
        std::fs::read_to_string(root.join("docs/decisions/0046-the-boundary-is-named.md")).unwrap();
    assert!(decision.starts_with("# 0046 — The boundary is named"));
    assert!(decision.contains("\nStatus: accepted (operator ruled in chat, 2026-09-05)\n"));
    let erratum: Vec<&str> = decision
        .split("\n## Erratum\n")
        .nth(1)
        .expect("the decision carries its erratum heading")
        .split("\n## ")
        .next()
        .expect("the erratum section is bounded by the next heading")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(
        erratum.len(),
        1,
        "the erratum is exactly one line: {erratum:?}"
    );
    for named in [
        "`seat-record.v3`",
        "`seat-record.v4`",
        "#202",
        "decision 0034 rulings 6 and 7",
        "nothing else is renumbered",
    ] {
        assert!(
            erratum[0].contains(named),
            "the erratum does not name {named}"
        );
    }
}
