//! Decision 0033's executable documentation: the sixty-second guide is
//! the recipe library rendered as one table, and the contribution gate
//! keeps its declaration, evidence, head-binding, and visible escape
//! hatch in repository-owned platform data.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

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

    let output = Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(["recipes", "list"])
        .current_dir(&root)
        .output()
        .unwrap();
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
            (fields.len() == 7).then(|| {
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
        let raw = std::fs::read_to_string(dir.join("bundle.json")).unwrap();
        let bundle: Value = serde_json::from_str(&raw).unwrap();
        let name = bundle["name"].as_str().unwrap();
        let description = bundle["description"].as_str().unwrap();
        let cost = bundle["cost"].as_str().unwrap();
        assert!(
            !description.is_empty() && !description.contains(['\r', '\n']),
            "{name} needs a one-line description"
        );
        let (listed_description, seats, listed_cost) = listed
            .get(name)
            .unwrap_or_else(|| panic!("brokkr recipes list omitted {name}"));
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

    let workflow = std::fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    for binding in [
        "name: delivered by brokkr",
        "github.event.pull_request.base.sha",
        "fetch-depth: 0",
        "scripts/delivered-by-brokkr.sh",
        "join(github.event.pull_request.labels.*.name, ',')",
        // Decision 0038 ruling 4: the label re-runs the gate, no reopen.
        "types: [opened, synchronize, reopened, labeled, unlabeled]",
    ] {
        assert!(workflow.contains(binding), "CI lost binding: {binding}");
    }

    // The gate itself is repository-owned data too (0033 ruling 4, 0038
    // rulings 2, 3 and 6): every check the ruling names is in the script.
    let gate = std::fs::read_to_string(root.join("scripts/delivered-by-brokkr.sh")).unwrap();
    for binding in [
        "refs/heads/brokkr-runs/${run}",
        "verify-run \"$work/${run}.ndjson\"",
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

    let manual = std::fs::read_to_string(root.join("docs/guides/contributing-by-hand.md")).unwrap();
    assert!(
        manual.lines().count() >= 600,
        "the old handbook was not preserved whole"
    );
    for preserved in [
        "## The twelve checks",
        "## The coverage gate, practically",
        "## Commits, signing, and how your PR actually lands",
        "## The decision culture",
        "## What is frozen",
    ] {
        assert!(manual.contains(preserved), "manual guide lost {preserved}");
    }
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
/// every one-line command and every command line of a multi-line block it
/// runs.
struct Step {
    env: Vec<(String, String)>,
    commands: Vec<String>,
}

impl WorkflowLeg {
    /// The leg's commands as a local run writes them: each carries its
    /// step's variables that `LOCAL_ENV` carries, in their local form.
    fn local_commands(&self) -> Vec<String> {
        let mut commands = Vec::new();
        for step in &self.steps {
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
                let value = local_value(value, self.runner);
                if matches!(carried, Local::Carried) && !value.is_empty() {
                    prefix.push_str(&format!("{name}={value} "));
                }
            }
            commands.extend(step.commands.iter().map(|run| format!("{prefix}{run}")));
        }
        commands
    }
}

/// Whether a local run sets a variable a checked step sets.
#[derive(Clone, Copy)]
enum Local {
    Carried,
    Omitted,
}

/// Every variable a checked step sets, ruled on once: a variable no row
/// has ruled on fails the test rather than dropping out of the guide.
const LOCAL_ENV: [(&str, Local); 6] = [
    ("BROKKR_REQUIRE_BOUNDARY_EVIDENCE", Local::Carried),
    ("BROKKR_SUPPRESSION_BASE", Local::Carried),
    ("PR_BODY", Local::Carried),
    // The mutants gate's base: its command names it as an argument.
    ("BASE", Local::Omitted),
    // CI's runner has two cores; unset, cargo-mutants runs one job.
    ("MUTANTS_JOBS", Local::Omitted),
    // `npm ci --ignore-scripts` runs no download script either way.
    ("PUPPETEER_SKIP_DOWNLOAD", Local::Omitted),
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

fn workflow_legs(root: &Path, file: &'static str) -> Vec<WorkflowLeg> {
    let text = std::fs::read_to_string(root.join(".github/workflows").join(file)).unwrap();
    let jobs = text.split_once("\njobs:\n").expect("a jobs map").1;
    let mut blocks: Vec<(&str, Vec<&str>)> = Vec::new();
    for line in jobs.lines() {
        match line
            .strip_prefix("  ")
            .and_then(|key| key.strip_suffix(':'))
        {
            Some(id) if !id.starts_with([' ', '#']) => blocks.push((id, Vec::new())),
            // A comment above the first job belongs to no job.
            _ => blocks.last_mut().map_or((), |(_, body)| body.push(line)),
        }
    }
    blocks
        .into_iter()
        .flat_map(|(id, body)| job_legs(file, id, &body))
        .collect()
}

fn job_legs(file: &'static str, id: &str, body: &[&str]) -> Vec<WorkflowLeg> {
    let field = |key: &str| body.iter().find_map(|line| line.strip_prefix(key));
    let name = field("    name: ")
        .unwrap_or_else(|| panic!("{file}'s {id} has no name"))
        .trim_matches('\'');
    let runs_on = field("    runs-on: ").unwrap_or_else(|| panic!("{file}'s {id} has no runner"));
    let oses: Vec<&str> = body
        .iter()
        .find_map(|line| line.trim().strip_prefix("os: ["))
        .map_or_else(
            || vec![runs_on],
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
                        commands: step_commands(step),
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

/// What one step runs: its one-line command, or every command line of its
/// multi-line block (continuations joined), a call to a function the block
/// defines standing for that function's body with `"$@"` replaced by the
/// call's arguments. A pull request's base stands as `origin/main`, the
/// base a local clone has. Only scaffolding is skipped, by `scaffolding`.
fn step_commands(step: &[&str]) -> Vec<String> {
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
            commands.extend(block_commands(&block.join("\n").replace(" \\\n", " ")));
        } else if let Some(run) = key.strip_prefix("run: ") {
            commands.push(run.replace("\"$BASE\"", "origin/main"));
        } else if let Some(deny) = key.strip_prefix("command: ") {
            commands.push(format!("cargo deny {deny}"));
        }
    }
    commands.retain(|run| !run.starts_with("cargo install "));
    commands
}

/// The command lines of one `run: |` block, its functions expanded.
fn block_commands(block: &str) -> Vec<String> {
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
        } else if !scaffolding(line) {
            commands.push(line.replace("'HEAD^1'", "origin/main"));
        }
    }
    commands
}

/// A block line that does no check of its own, so no row carries it:
/// - blank lines and comments;
/// - output and exits: `echo`, `printf`, `exit`;
/// - shell structure: `if`/`then`/`else`/`elif`/`fi`, `case`/`esac` and
///   their arms, and braces (a `set` line is a command: it decides whether
///   a failure stops the block);
/// - a `git fetch` of the pull request's base, which a local clone has;
/// - the ratchets job's check that its checkout is a merge commit
///   (`git rev-parse --verify --quiet 'HEAD^2'`), which only a pull
///   request's merge ref is;
/// - the macOS leg's host report (`sw_vers`, `uname -m`) and its check
///   that the runner carries `/usr/bin/sandbox-exec`.
fn scaffolding(line: &str) -> bool {
    let word = line.split_whitespace().next().unwrap_or_default();
    line.is_empty()
        || line.starts_with('#')
        || matches!(
            word,
            "echo"
                | "printf"
                | "exit"
                | "if"
                | "then"
                | "else"
                | "elif"
                | "fi"
                | "case"
                | "esac"
                | ";;"
                | "{"
                | "}"
                | "sw_vers"
        )
        || line.ends_with(')') && !line.contains(' ')
        || line.starts_with("git fetch ")
        || line.starts_with("git -C pr fetch ")
        || line.starts_with("git rev-parse --verify --quiet 'HEAD^2'")
        || line == "uname -m"
        || line == "test -x /usr/bin/sandbox-exec"
}

/// The commands a row writes: its cell's code spans, and every line of a
/// code block in a section it links.
fn written_commands(guide: &str, cell: &str) -> Vec<String> {
    cell.split('`')
        .skip(1)
        .step_by(2)
        .chain(
            linked_sections(guide, cell)
                .into_iter()
                .flat_map(code_lines),
        )
        .map(str::to_string)
        .collect()
}

/// The guide sections a row's command cell links to.
fn linked_sections<'a>(guide: &'a str, cell: &str) -> Vec<&'a str> {
    cell.split("](#")
        .skip(1)
        .map(|link| {
            let anchor = link.split_once(')').expect("a closed link").0;
            guide
                .split("\n#")
                .find(|section| heading_anchor(section) == anchor)
                .unwrap_or_else(|| panic!("the guide has no section #{anchor}"))
        })
        .collect()
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
    "cargo run --locked -p brokkr-cli -- compile --bundle recipes/<name>",
    // The directory the coverage script keeps its warm build in: a path
    // the section names, not a command.
    "${BROKKR_COVERAGE_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}}/brokkr-coverage-cache",
    // The same gate with its temporary directories moved off a small tmpfs.
    "TMPDIR=/var/tmp BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 bash scripts/coverage-exact.sh",
];

/// Checks with no local form, so their rows write no command: the gate
/// judges a pull request's own evidence with its base branch's verifier.
const CI_ONLY: [&str; 1] = ["delivered by brokkr"];

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
            let file = if cells[2].ends_with(" (mutants.yml)") {
                "mutants.yml"
            } else {
                "ci.yml"
            };
            CheckRow {
                number: cells[0].to_string(),
                check: cells[1].trim_matches('`').to_string(),
                job: cells[2]
                    .trim_end_matches(" (mutants.yml)")
                    .trim_matches('`')
                    .to_string(),
                file,
                command: cells[3].to_string(),
            }
        })
        .collect()
}

/// The checks branch protection on main requires (operator rulings,
/// 2026-09-26), as (context, job id, workflow). Branch protection lives
/// outside the tree, so this list is its one home in the repository.
const MAIN_REQUIRES: [(&str, &str, &str); 12] = [
    ("delivered by brokkr", "delivered-by-brokkr", "ci.yml"),
    ("MSRV (1.88)", "msrv", "ci.yml"),
    ("format, clippy, contracts", "quality", "ci.yml"),
    ("test (ubuntu-latest)", "engine", "ci.yml"),
    ("test (macos-latest)", "engine", "ci.yml"),
    ("exact coverage gate", "coverage", "ci.yml"),
    (
        "dependency licenses (cargo-deny)",
        "license-compliance",
        "ci.yml",
    ),
    ("non-Rust lints", "lint-non-rust", "ci.yml"),
    ("baseline ratchets", "ratchets", "ci.yml"),
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
    let legs: Vec<WorkflowLeg> = ["ci.yml", "mutants.yml"]
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
    for (at, row) in rows.iter().enumerate() {
        let check = &row.check;
        assert_eq!(
            row.number,
            (at + 1).to_string(),
            "{check} is numbered in order"
        );
        let written = written_commands(&guide, &row.command);
        if CI_ONLY.contains(&check.as_str()) {
            assert_eq!(written, Vec::<String>::new(), "{check} has no local form");
            continue;
        }
        let leg = legs
            .iter()
            .find(|leg| (&leg.check, &leg.id, leg.file) == (&row.check, &row.job, row.file))
            .expect("a defined leg");
        let runs = leg.local_commands();
        for run in &runs {
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
    }
    let contributing = std::fs::read_to_string(root.join("CONTRIBUTING.md")).unwrap();
    assert!(contributing.contains("preserves the twelve exact checks"));
    assert!(guide.contains("twelve required checks"));
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

/// Decision 0046's guides (boundary-guides / The guides document the
/// boundary and never lose a section): every page keeps the sections it
/// had and gained the rows the decision commissions, no page states
/// that the network was off under `harness` or `open`, and the decision
/// carries its one-line erratum under a heading of its own.
#[test]
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn the_boundary_guides_keep_every_section_and_gained_the_rows() {
    let root = workspace();
    // (page, kept, gained) — matched on whitespace-collapsed text so a
    // reflowed paragraph is not a lost row.
    let pages: [(&str, &[&str], &[&str]); 10] = [
        (
            "docs/guides/provider-adapters.md",
            &["A provider adapter is **data**", "## Hands"],
            &[
                "ok boundaries:",
                "### `hands.harness`",
                "| `gate` |",
                "| `work` |",
                "| `result` |",
                "`{result_path}`",
                "`{hands_mcp_json}` and `{hands_args_toml}`, are refused",
                "pending and the operator's**",
                "declares **no** `hands.harness` member yet",
            ],
        ),
        (
            "docs/guides/recipe-authoring.md",
            &["$ brokkr recipes list", "## The three files"],
            &[
                "| `driver.confine` | **Refused.**",
                "decision [0046](../decisions/0046-the-boundary-is-named.md) ruling 5",
                "| `hands` | Decision 0043:",
                "`mask` **declared and not enforced**",
                "Clearing the environment confines nothing on disk",
            ],
        ),
        (
            "docs/guides/quickstart.md",
            &["### Step 1 — install", "#### Re-run under another strategy"],
            &[
                "`\"boundary\": \"harness\"`",
                "renders such a run *unboxed*",
                "`bundles/self` and `recipes/panel-review`",
                "`recipes/triage` and `recipes/night-shift`",
                "crates/brokkr-runtime/src/bundle/model_policy_tests.rs",
                "The rerun compiles the new recipe in the discovered realm",
            ],
        ),
        (
            "docs/guides/journal-and-verification.md",
            &["$ brokkr verify-run", "`brokkr costs --run <id>`"],
            &[
                "## The boundary on the record, and the word *unboxed*",
                "`effect-boundary.v1.schema.json`",
                "`no boundary recorded`",
            ],
        ),
        (
            "docs/guides/repository-layout.md",
            &["| `crates/` | The engine:"],
            &[
                "`house`, `dialect` and `boundary`",
                "`realms.v4`",
                "`run-manifest.v9`",
                "`seat-record.v4`",
                "`effect-boundary.v1`",
            ],
        ),
        (
            "docs/guides/driver-authoring.md",
            &["## Wiring it into a bundle", "## See also"],
            &[
                "under the `namespace` boundary the engine runs the whole script through `brokkr hands exec`",
                "re-walks the script's declaring layer",
                "`driver.confine`",
                "decision 0046 ruling 5",
            ],
        ),
        (
            "ARCHITECTURE.md",
            &["## Drivers", "## Verification, in layers"],
            &["the realm's **boundary** (decision 0046)", "0046 ruling 5"],
        ),
        (
            "docs/extension-model.md",
            &["**Status**: partially accepted", "| `trust` |"],
            &["The wall itself is the realm's `boundary` (decision 0046"],
        ),
        (
            "docs/target-architecture.md",
            &[
                "**Status**: implementation blueprint, accepted 2026-08-22",
                "| `policy-confined` |",
                "| `public-evidence-only` |",
            ],
            &["Decision 0046's `container` boundary", "slice (iii)"],
        ),
        (
            "contracts/README.md",
            &["## Event vocabulary (v1)", "## Fold semantics"],
            &[
                "`realms.v4.schema.json`",
                "`run-manifest.v9.schema.json`",
                "`seat-record.v4.schema.json`",
                "`effect-boundary.v1.schema.json`",
                "`effect/started.boundary`",
            ],
        ),
    ];
    for (page, kept, gained) in pages {
        let raw = std::fs::read_to_string(root.join(page)).unwrap();
        let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        for section in kept {
            assert!(text.contains(section), "{page} lost: {section}");
        }
        for row in gained {
            assert!(text.contains(row), "{page} never gained: {row}");
        }
        // Task 12.8 (design DD15): the prefix is a narrowing the engine
        // attempts on Linux, and no page says the network was off.
        let lower = text.to_lowercase();
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

/// Section and table-label inventory from the committed pre-0046 guides.
/// New sections are welcome; deleting existing guidance is not part of this slice.
/// One row left by ruling, not by drift: decision 0063 retired the quickstart's
/// `scoop` channel with the Windows host.
#[test]
fn every_original_guide_section_and_table_row_remains_available() {
    let inventory = [
        ("docs/guides/provider-adapters.md", "# Provider adapters\n## Hands", "", ""),
        ("docs/guides/recipe-authoring.md", "# Recipe authoring — bundles, policy tables, and composition\n## Recipes and composition\n## The three files\n## `bundle.json` anatomy\n### A seat\n## Seat bodies: single, panel, sequence, select\n## Composition: `extends` and `override`\n## Digest identity\n## The policy table\n### A rule\n## The condition vocabulary\n## The reforging ladder\n## Compile it\n## See also", "Recipe\n---\n[`fast`](../../recipes/fast)\n[`triage`](../../recipes/triage/README.md)\n[`night-shift`](../../recipes/night-shift/README.md)\n[`wager-harness`](../../recipes/wager-harness/README.md)\n[`wager-harness-dsh`](../../recipes/wager-harness-dsh/README.md)\n[`wager-harness-muse`](../../recipes/wager-harness-muse/README.md)\n[`node`](../../recipes/node/README.md)\n[`panel-review`](../../recipes/panel-review)\n[`preflight`](../../recipes/preflight/README.md)\nKey\n`name`\n`description`\n`cost`\n`policy`\n`protected_phase`\n`egress_minimum`\n`seats`\n`extends`\n`override`, `remove`\n`results`\n`role` + `driver`\n`agent`\n`inputs`\n`limits`\n`secrets`\n`driver.confine`\n`hands`\nAggregate\n`unanimous-pass`\n`review-panel`\n`id`\n`from`\n`result`\n`reason`\n`next`\n`park`\n`severity`\n`when`\n`requires_artifacts`\nForm\nbare name\n`strategy_in`\n`<counter>_gte`\n`visits_<phase>_gte`\n`<axis>_above`\n`<axis>_at_most`\nRule\n`…-EXHAUSTED-ABOVE-MEDIUM`\n`…-EXHAUSTED-MEDIUM`\n`…-EXHAUSTED-DEBT`\n`…-EXHAUSTED-UNFIXED`", ""),
        ("docs/guides/quickstart.md", "# Quickstart — one spine, and everything else is a diff over it\n## The spine\n### Step 1 — install\n### Step 2 — `brokkr init .`\n### Step 3 — `brokkr run`\n### Step 4 — read the journal\n## Per-stack cards\n## Flow 2 — deliver\n## Flow 3 — adopt\n## After the spine\n### Where the run wrote things\n### The escape hatches\n#### Operator commands — `retry` and `stop`\n#### Resume\n#### Conclude — closing a run whose bundle no longer compiles\n#### Re-run under another strategy\n### What it cost\n### What the budgets do not cover\n### Limits worth knowing\n## Compact first-run tour\n## Next", "\n---\n1\n2\n3\n4\nChannel\ntarball\ncargo\nnix\napt\ndnf\nbrew\nFlag\n`--repo <path>`\n`--db <path>`\n`--realms <file>`\n`--recipes-dir <path>`\n`--secrets-file <path>`\n`--dispatch <file>`\nCard\n[node](cards/node.md)\n[bun](cards/bun.md)\n[rust](cards/rust.md)\n[go](cards/go.md)\n[python](cards/python.md)", ""),
        ("docs/guides/journal-and-verification.md", "# The journal and verification\n#   <run>.redacted.ndjson — paths and usernames as stable placeholders, hashes\n#   verify only on the verbatim pair, and the manifest says so", "", ""),
        ("docs/guides/read-surfaces.md", "# The read surfaces\n### `brokkr realms` — the world\n### `brokkr runs` — the fleet\n### `brokkr inspect` — one run, explained\n### `brokkr watch` — the same, live\n### `brokkr tui` — the readouts made explorable\n### `brokkr ui` — the browser console\n### `brokkr muninn` — the fleet, read and advised on", "", ""),
        ("docs/guides/repository-layout.md", "# Repo layout", "Path\n---\n[`ARCHITECTURE.md`](../../ARCHITECTURE.md)\n[`CONTRIBUTING.md`](../../CONTRIBUTING.md)\n`crates/`\n`contracts/`\n`realms.json`\n`docs/house-rules.md`\n`bundles/`\n`recipes/`\n`agents/`\n`dialects/`\n`adapters/`\n`fixtures/`\n`policy/phase-machine.json`\n[`docs/decisions/`](../decisions/)\n[`docs/lore/`](../lore/)\n`assets/`\n`reference/`\n`scripts/coverage-exact.sh`", ""),
        ("docs/guides/driver-authoring.md", "# Driver authoring — the `forge-driver/v1` wire contract\n## Transport\n## The message family\n## The exchange, in order\n## What the engine actually sends\n## `resume` — rejoining the session you opened\n## `accepted` is the load-bearing message\n## Checkpoints\n## Results\n## The result-file contract\n## Deadlines and kills\n## A minimal driver, in prose\n## The conformance suite is the acceptance test\n## Wiring it into a bundle\n## See also", "Message\n---\n`hello`\n`capabilities`\n`start`\n`accepted`\n`checkpoint`\n`result`\n`resume`\n`cancel`\n`cancelled`\n`shutdown`\nWhat happened\nYour process exits **without** `accepted` and without a result\nYour process exits **after** `accepted` and without a result\nYou send `result` with `status: \"failed\"`\nYou violate the protocol\nField\n`input_tokens`\n`output_tokens`\n`cache_read_tokens`\n`reasoning_output_tokens`\n`cache_write_tokens`\nKey\n`inputs`\n`notes`\n`model`\n`effort`", ""),
        ("ARCHITECTURE.md", "# Architecture\n## The shape\n## The journal is the run\n## Every effect, in order\n## Policy is data\n## A bundle, resolved\n## Drivers\n## Verification, in layers\n## The operating surface", "Layer\n---\nDifferential corpus\nMachine proof\nSelf-delivery\nBrokkr verification", "**Status**: the system as implemented. The blueprint it grew from is"),
        ("docs/extension-model.md", "# Extension model — nodes, seats, and what may never be unplugged\n## Layer 1 — Phases (nodes of the outer machine)\n## Layer 2 — Seats (agents inside a phase)\n## Layer 3 — Profiles (the stack-specific bundle)\n## Resolved\n## Open questions for discussion", "Field\n---\n`role`\n`class`\n`trust`\n`result_schema`\n`driver`", "**Status**: partially accepted. Decisions 0002 and 0003 lock the outer-machine,"),
        ("docs/target-architecture.md", "# Target architecture\n## Product contract\n## System shape\n## Rust workspace and one shipped binary\n## State and control status\n## Event and effect protocol\n## SQLite and artifacts\n## Declarative bundles\n## Run manifest and versioning\n## Drivers and isolation\n## Cordis and other long-horizon harnesses\n## Local API and embedded UI\n## Audit and evaluation\n## Installation and operation\n## Delivery sequence\n## First-release acceptance criteria\n## Deferred choices\n## References", "Crate\n---\n`brokkr-core`\n`brokkr-store`\n`brokkr-runtime`\n`brokkr-protocol`\n`brokkr-api`\n`brokkr-cli`\nPrimitive\n`seat`\n`parallel`\n`join`\n`loop`\n`gate`\n`tool`\n`submachine`\n`emit-result`\nTrust\n`trusted`\n`policy-confined`\n`public-evidence-only`", "**Status**: implementation blueprint, accepted 2026-08-22 under"),
    ];
    for (path, headings, labels, statuses) in inventory {
        let text = std::fs::read_to_string(workspace().join(path)).unwrap();
        for heading in headings.lines() {
            assert!(
                text.lines().any(|line| line == heading),
                "{path} lost section {heading}"
            );
        }
        let current: Vec<&str> = text
            .lines()
            .filter(|line| line.starts_with('|'))
            .filter_map(|line| line.split('|').nth(1))
            .map(str::trim)
            .collect();
        for label in labels.lines() {
            assert!(current.contains(&label), "{path} lost row {label}");
        }
        for status in statuses.lines() {
            assert!(
                text.lines().any(|line| line == status),
                "{path} changed its status"
            );
        }
    }
}
