//! Decision 0033's executable documentation: the sixty-second guide is
//! the recipe library rendered as one table, and the contribution gate
//! keeps its declaration, evidence, head-binding, and visible escape
//! hatch in repository-owned platform data.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

#[path = "support/workflow.rs"]
mod workflow;

#[path = "contributing/inline_copies.rs"]
mod inline_copies;
use inline_copies::{assert_first_words_listed, assert_inline_copies, text_reads_as_command};

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
/// every line it runs: its one-line command, or each code line of its
/// multi-line block.
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
            let value = local_value(value, self.runner);
            if matches!(carried, Local::Carried) && !value.is_empty() {
                prefix.push_str(&format!("{name}={value} "));
            }
        }
        prefix
    }
}

/// A line a checked leg runs.
#[derive(Debug)]
enum Line {
    /// A command its row writes.
    Written,
    /// A line its row leaves out, held word for word: output, exits and
    /// shell structure, a fetch of the pull request's base (a local clone
    /// has it), the ratchets job's check that its checkout is a pull
    /// request's merge commit, the macOS host report and its check for
    /// `/usr/bin/sandbox-exec`, the coverage job's pin reads and its
    /// cargo-public-api install (`the_by_hand_tool_versions_are_the_pins`
    /// holds its version), and every line of `delivered by brokkr`, which
    /// has no local form: it judges a pull request's own evidence with its
    /// base branch's verifier.
    Unwritten(&'static str),
}

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
    ("MSRV (1.88)", &[Line::Written]),
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
        &[Line::Written, Line::Written, Line::Written],
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
        ],
    ),
    (
        "exact coverage gate",
        &[
            Line::Unwritten(
                r#"echo "toolchain=$(tr -d '[:space:]' < rust-nightly-version.txt)" >> "$GITHUB_OUTPUT""#,
            ),
            Line::Unwritten(
                r#"echo "cargo_llvm_cov=$(tr -d '[:space:]' < cargo-llvm-cov-version.txt)" >> "$GITHUB_OUTPUT""#,
            ),
            Line::Unwritten(
                r#"echo "RUSTUP_TOOLCHAIN=$(tr -d '[:space:]' < rust-nightly-version.txt)" >> "$GITHUB_ENV""#,
            ),
            Line::Written,
            Line::Written,
            Line::Unwritten("cargo install --locked cargo-public-api --version 0.52.0"),
            Line::Written,
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
    ("RustSec dependency audit", &[]),
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
    workflow::jobs(&text)
        .into_iter()
        .flat_map(|(id, body)| job_legs(file, &id, &body.lines().collect::<Vec<_>>()))
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
    "cargo run --locked -p brokkr-cli -- compile --bundle recipes/<name>",
    // The directory the coverage script keeps its warm build in: a path
    // the section names, not a command.
    "${BROKKR_COVERAGE_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}}/brokkr-coverage-cache",
    // The same gate with its temporary directories moved off a small tmpfs.
    "TMPDIR=/var/tmp BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 bash scripts/coverage-exact.sh",
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
/// 2026-09-26), as (context, job id, workflow), in the workflows' order.
/// Branch protection lives outside the tree; its checked-in snapshot,
/// `.github/branch-protection.json`, holds these contexts, and
/// `contributing_lists_the_checks_branch_protection_requires` holds this
/// list to it.
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
    let legs: Vec<WorkflowLeg> = ["ci.yml", "mutants.yml"]
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

/// Every line of each job behind the twelve checks, in `MAIN_REQUIRES`'s
/// order, as its workflow writes it, but for comments, blank lines and the
/// value of a `run:` or `command:` key, whose lines `LEG_LINES` holds. A
/// skipped step or job reports success, which satisfies a required check,
/// and a runner, an action, an input or a variable changed alters what a
/// check proves while its commands stand, so each is held word for word:
/// a `continue-on-error`, `if:`, `needs:`, `runs-on:`, `uses:`, `with:` or
/// `env:` line added, dropped or changed fails the test. A line changed
/// here is a line to re-read in the guide's row and the sections it links.
/// A line that is a key of `SHARED_STEPS` stands for that step's lines.
const JOB_LINES: [(&str, &str); 11] = [
    (
        "delivered-by-brokkr",
        r#"
    name: delivered by brokkr
    if: github.event_name == 'pull_request'
    runs-on: ubuntu-latest
    timeout-minutes: 20
    permissions:
      contents: read
      pull-requests: read
    steps:
<checkout>
          ref: ${{ github.event.pull_request.base.sha }}
          path: base-verifier
<checkout>
          repository: ${{ github.event.pull_request.head.repo.full_name }}
          ref: ${{ github.event.pull_request.head.sha }}
          fetch-depth: 0
          path: pr
      - name: the base branch, for the merge-base
        env:
          BASE_REF: ${{ github.event.pull_request.base.ref }}
          BASE_REPO: https://github.com/${{ github.repository }}.git
        run:
<stable toolchain>
      - name: build the base branch's offline verifier
        run:
      - name: the tier is cut by the delta since the judgment (decisions 0033, 0038)
        env:
          PR_BODY: ${{ github.event.pull_request.body }}
          PR_HEAD: ${{ github.event.pull_request.head.sha }}
          PR_BASE: refs/remotes/base/${{ github.event.pull_request.base.ref }}
          REPO: pr
          EVIDENCE: https://github.com/${{ github.event.pull_request.head.repo.full_name }}.git
          VERIFIER: base-verifier/target/debug/brokkr
          CLASSES: base-verifier/.github/delivery-classes.json
          LABELS: ${{ join(github.event.pull_request.labels.*.name, ',') }}
        run:
"#,
    ),
    (
        "msrv",
        r#"
    name: MSRV (1.88)
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
<checkout>
      - uses: dtolnay/rust-toolchain@02cb101ec7c40f2c49e1d9714d64511d8e1b74de # master
        with:
          toolchain: 1.88.0 # the MSRV, Cargo.toml's rust-version: moved by hand, never by Renovate
      - run:
"#,
    ),
    (
        "quality",
        r#"
    name: format, clippy, contracts
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
<checkout>
<stable toolchain>
          components: rustfmt, clippy
<rust-cache>
      - name: formatting is canonical
        run:
      - name: clippy is warning-free across every target
        run:
      - name: an added suppression names a ruling
        if: github.event_name == 'pull_request'
        env:
          BROKKR_SUPPRESSION_BASE: ${{ github.event.pull_request.base.sha }}
        run:
      - name: frozen and additive contracts compile
        run:
"#,
    ),
    (
        "engine",
        r#"
    name: test (${{ matrix.os }})
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    timeout-minutes: 20
    steps:
<checkout>
<stable toolchain>
<rust-cache>
      - name: bubblewrap for the hands tests
        if: runner.os == 'Linux'
        uses: ./.github/actions/setup-bubblewrap
      - run:
        env:
          BROKKR_REQUIRE_BOUNDARY_EVIDENCE: ${{ runner.os == 'Linux' && '1' || '' }}
      - name: R3 native startup gate
        if: runner.os == 'macOS'
        run:
      - name: R3 native diagnostics
        if: always() && runner.os == 'macOS'
        uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2
        with:
          name: r3-native-diagnostics
          path: |
            target/r3-startup.log
            target/r3-startup-report.txt
          if-no-files-found: warn
      - name: self and verify bundles compile under the constitutional lint
        run:
"#,
    ),
    (
        "coverage",
        r#"
    name: exact coverage gate
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
<checkout>
      - name: the pinned coverage toolchain and its measuring tool
        id: nightly
        run:
      - uses: dtolnay/rust-toolchain@02cb101ec7c40f2c49e1d9714d64511d8e1b74de # master
        with:
          toolchain: ${{ steps.nightly.outputs.toolchain }}
          components: llvm-tools-preview
      - uses: taiki-e/install-action@9983c65e42da123ff25d1f78505eb6de315aa172 # v2.87.20
        with:
          tool: cargo-llvm-cov@${{ steps.nightly.outputs.cargo_llvm_cov }}
          fallback: none
<rust-cache>
      - uses: actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0
        with:
          path: ~/.cache/brokkr-coverage-cache
          key: brokkr-coverage-${{ runner.os }}-${{ hashFiles('rust-nightly-version.txt', 'cargo-llvm-cov-version.txt', 'Cargo.lock', 'Cargo.toml', 'crates/*/Cargo.toml') }}
      - name: bubblewrap for the hands tests
        uses: ./.github/actions/setup-bubblewrap
      - name: prove literal nonzero 100% source-line/branch/function coverage
        run:
        env:
          BROKKR_REQUIRE_BOUNDARY_EVIDENCE: '1'
      - uses: taiki-e/install-action@9983c65e42da123ff25d1f78505eb6de315aa172 # v2.87.20
        with:
          tool: cargo-crap@0.5.0
          fallback: none
      - name: complexity ratchet (quality/crap-baseline.json)
        run:
      - name: cargo-public-api 0.52.0, pinned
        run:
      - name: public-API ratchet (quality/public-api/)
        run:
      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2
        if: always()
        with:
          name: coverage-exact
          path: target/coverage/
"#,
    ),
    (
        "license-compliance",
        r#"
    name: dependency licenses (cargo-deny)
    runs-on: ubuntu-latest
    timeout-minutes: 20
    permissions:
      contents: read
    steps:
<checkout>
      - uses: EmbarkStudios/cargo-deny-action@3c6349835b2b7b196a839186cb8b78e02f7b5f25 # v2.1.1
        with:
          command:
"#,
    ),
    (
        "lint-non-rust",
        r#"
    name: non-Rust lints
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
<checkout>
      - name: actionlint, pinned by digest
        uses: ./.github/actions/setup-actionlint
      - name: lychee, pinned by digest
        uses: ./.github/actions/setup-lychee
      - uses: taiki-e/install-action@9983c65e42da123ff25d1f78505eb6de315aa172 # v2.87.20
        with:
          tool: shellcheck@0.11.0,typos@1.50.2,zizmor@1.30.1
          fallback: none
      - name: the offline lints (scripts/lint-non-rust.sh)
        run:
      - uses: actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020 # v4.4.0
        with:
          node-version: 22.23.3
      - name: the diagrams render (mermaid-cli, from .github/lint's lockfile)
        env:
          PUPPETEER_SKIP_DOWNLOAD: '1'
        run:
      - name: Renovate's configuration is valid (renovate-config-validator, the pinned image)
        run:
"#,
    ),
    (
        "ratchets",
        r#"
    name: baseline ratchets
    runs-on: ubuntu-latest
    timeout-minutes: 15
    permissions:
      contents: read
    steps:
<checkout>
          fetch-depth: 2
<stable toolchain>
      - uses: ./.github/actions/setup-jscpd
      - uses: taiki-e/install-action@9983c65e42da123ff25d1f78505eb6de315aa172 # v2.87.20
        with:
          tool: cargo-shear@1.14.0
          fallback: none
      - name: file-size ratchet (quality/file-lines.txt)
        run:
      - name: duplication ratchet (quality/jscpd-baseline-*.json)
        run:
      - name: no unused dependency (cargo-shear)
        run:
      - name: a raised baseline names its ruling (quality/)
        if: github.event_name == 'pull_request'
        env:
          PR_BODY: ${{ github.event.pull_request.body }}
        run:
"#,
    ),
    (
        "dependency-audit",
        r#"
    name: RustSec dependency audit
    runs-on: ubuntu-latest
    timeout-minutes: 20
    permissions:
      contents: read
      checks: write
      issues: write
    steps:
<checkout>
      - name: cargo-audit, pinned by digest
        uses: ./.github/actions/setup-cargo-audit
      - uses: rustsec/audit-check@69366f33c96575abad1ee0dba8212993eecbe998 # v2.0.0
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
"#,
    ),
    (
        "release-binary",
        r#"
    name: release binary artifact
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
<checkout>
<stable toolchain>
<rust-cache>
      - run:
      - run:
      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2
        with:
          name: brokkr-linux-x86_64
          path: target/release/brokkr
"#,
    ),
    (
        "core-gate",
        r#"
    name: 'mutants in the diff: brokkr-core'
    if: github.event_name == 'pull_request'
    runs-on: ubuntu-latest
    timeout-minutes: 90
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0
        with:
          fetch-depth: 0
          persist-credentials: false
<stable toolchain>
<rust-cache>
      - uses: taiki-e/install-action@9983c65e42da123ff25d1f78505eb6de315aa172 # v2.87.20
        with:
          tool: cargo-mutants@27.1.0
          fallback: none
      - name: no miss this pull request adds to brokkr-core
        env:
          BASE: ${{ github.event.pull_request.base.sha }}
          MUTANTS_JOBS: '2'
        run:
      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2
        if: always()
        with:
          name: mutants-in-diff-brokkr-core
          path: target/mutants/mutants.out/
"#,
    ),
];

/// The `key: value` pairs written at `indent` among `lines`, comments
/// and deeper lines skipped.
fn keys_at<'a>(lines: &[&'a str], indent: &str) -> Vec<(&'a str, &'a str)> {
    lines
        .iter()
        .filter_map(|line| line.strip_prefix(indent))
        .filter(|line| !line.starts_with([' ', '#']) && !line.is_empty())
        .map(|line| {
            let (key, value) = line.split_once(':').unwrap_or((line, ""));
            (key, value.trim())
        })
        .collect()
}

/// The steps several required jobs take word for word, held once.
const SHARED_STEPS: [(&str, &str); 3] = [
    (
        "<checkout>",
        "      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0
        with:
          persist-credentials: false",
    ),
    (
        "<stable toolchain>",
        "      - uses: dtolnay/rust-toolchain@02cb101ec7c40f2c49e1d9714d64511d8e1b74de # master
        with:
          toolchain: 1.98.0",
    ),
    (
        "<rust-cache>",
        "      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2",
    ),
];

/// The lines `JOB_LINES` holds for one job, each shared step's name
/// replaced by its lines.
fn expected_lines(held: &str) -> Vec<&str> {
    held.lines()
        .skip(1)
        .flat_map(|line| {
            SHARED_STEPS
                .iter()
                .find(|(name, _)| *name == line)
                .map_or_else(|| vec![line], |(_, step)| step.lines().collect())
        })
        .collect()
}

/// A job's lines as `JOB_LINES` holds them: comments and blank lines
/// dropped, and a `run:` or `command:` key kept without its value, the
/// block under it included.
fn held_lines(body: &str) -> Vec<&str> {
    let indent = |line: &str| line.len() - line.trim_start().len();
    let mut held = Vec::new();
    let mut value_under = None;
    for line in body.lines() {
        if value_under.is_some_and(|key| line.trim().is_empty() || indent(line) > key) {
            continue;
        }
        value_under = None;
        let key = line.trim_start().trim_start_matches("- ");
        if key.is_empty() || key.starts_with('#') {
            continue;
        }
        let at = line.len() - key.len();
        match ["run:", "command:"]
            .into_iter()
            .find(|name| key.starts_with(name))
        {
            Some(name) => {
                held.push(&line[..at + name.len()]);
                value_under = Some(at);
            }
            None => held.push(line),
        }
    }
    held
}

/// A skipped step or job reports success, which satisfies a required
/// check, so the jobs behind the twelve are held whole: each workflow's
/// top-level keys (a workflow-wide `defaults:` or `env:` would reach every
/// step), and every line of each required job, as `JOB_LINES` holds it.
#[test]
fn the_required_jobs_run_every_step_unsoftened() {
    let root = workspace();
    let read =
        |file: &str| std::fs::read_to_string(root.join(".github/workflows").join(file)).unwrap();
    for (file, top) in [
        (
            "ci.yml",
            ["name", "on", "concurrency", "permissions", "jobs"],
        ),
        (
            "mutants.yml",
            ["name", "on", "permissions", "concurrency", "jobs"],
        ),
    ] {
        let text = read(file);
        let keys: Vec<&str> = keys_at(&text.lines().collect::<Vec<_>>(), "")
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        assert_eq!(keys, top, "{file}'s top-level keys");
    }
    let mut required: Vec<(&str, &str)> = MAIN_REQUIRES
        .iter()
        .map(|&(_, id, file)| (id, file))
        .collect();
    required.dedup();
    let held: Vec<&str> = JOB_LINES.iter().map(|(id, _)| *id).collect();
    let ids: Vec<&str> = required.iter().map(|(id, _)| *id).collect();
    assert_eq!(
        held, ids,
        "JOB_LINES holds each required job once, in order"
    );
    for ((id, file), (_, lines)) in required.into_iter().zip(JOB_LINES) {
        let body = workflow::jobs(&read(file))
            .into_iter()
            .find_map(|(job, body)| (job == id).then_some(body))
            .unwrap_or_else(|| panic!("{file} has no {id} job"));
        assert_eq!(
            held_lines(&body),
            expected_lines(lines),
            "a line of {file}'s {id} changed: re-read its row and the sections the row links"
        );
    }
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
}

/// The `if:` conditions this test reads, and what each does in a merge
/// group. Any other condition fails the test rather than being guessed at.
const CONDITIONS: [(&str, InTheQueue); 6] = [
    (
        "github.event_name == 'pull_request'",
        InTheQueue::PullRequestOnly,
    ),
    (
        "github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'",
        InTheQueue::Scheduled,
    ),
    ("runner.os == 'Linux'", InTheQueue::Runs),
    ("runner.os == 'macOS'", InTheQueue::Runs),
    ("always()", InTheQueue::Runs),
    ("always() && runner.os == 'macOS'", InTheQueue::Runs),
];

/// Every job of the workflows the required checks live in, in file order,
/// and what it does in a merge group.
const JOBS_IN_THE_QUEUE: [(&str, &str, InTheQueue); 20] = [
    // It judges the run that the pull request's own description names for
    // its own head (decisions 0033, 0038), and a merge group carries no
    // description, labels or head repository. The queue lands that head's
    // patch, which the judgment bound.
    ("ci.yml", "delivered-by-brokkr", InTheQueue::PullRequestOnly),
    ("ci.yml", "msrv", InTheQueue::Runs),
    ("ci.yml", "quality", InTheQueue::Runs),
    ("ci.yml", "engine", InTheQueue::Runs),
    ("ci.yml", "seatbelt-lifetime", InTheQueue::Runs),
    ("ci.yml", "coverage", InTheQueue::Runs),
    ("ci.yml", "license-compliance", InTheQueue::Runs),
    ("ci.yml", "lint-non-rust", InTheQueue::Runs),
    ("ci.yml", "ratchets", InTheQueue::Runs),
    ("ci.yml", "dependency-audit", InTheQueue::Runs),
    ("ci.yml", "bootstrap-budgets", InTheQueue::Runs),
    ("ci.yml", "bootstrap-budgets-macos", InTheQueue::Runs),
    ("ci.yml", "packaging", InTheQueue::Runs),
    ("ci.yml", "flake", InTheQueue::Runs),
    ("ci.yml", "release-binary", InTheQueue::Runs),
    // In no required check: it measures the pull request's merge against
    // the base its event names, and a rise is ruled on the pull request.
    ("ci.yml", "cpu-budgets", InTheQueue::PullRequestOnly),
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
    // It reads the suppressions the pull request's own diff adds against
    // its base. The tree's counts, which two pull requests could raise
    // together, ride the test jobs (quality/suppressions.txt).
    (
        "ci.yml",
        "quality",
        "an added suppression names a ruling",
        InTheQueue::PullRequestOnly,
    ),
    // It reads the `Ruling:` line of the pull request's description, which
    // a merge group does not carry. The file-size and duplication ratchets
    // before it judge the combined tree against the baselines.
    (
        "ci.yml",
        "ratchets",
        "a raised baseline names its ruling (quality/)",
        InTheQueue::PullRequestOnly,
    ),
];

/// What `condition`, an `if:` at `site`, does in a merge group.
fn in_the_queue(condition: Option<&str>, site: &str) -> InTheQueue {
    condition.map_or(InTheQueue::Runs, |condition| {
        CONDITIONS
            .iter()
            .find(|(known, _)| *known == condition)
            .map(|(_, queue)| *queue)
            .unwrap_or_else(|| panic!("{site} runs `if: {condition}`, which this test cannot read"))
    })
}

/// What the job `id` of `jobs` does in a merge group: its own `if:`, and
/// then the job it `needs:`, which a skip carries into it.
fn job_in_the_queue(file: &str, jobs: &[(String, String)], id: &str) -> InTheQueue {
    let site = format!("{file}'s {id}");
    let body = jobs
        .iter()
        .find_map(|(job, body)| (job == id).then_some(body))
        .unwrap_or_else(|| panic!("{file} has no {id} job"));
    let own = in_the_queue(
        body.lines().find_map(|line| line.strip_prefix("    if: ")),
        &site,
    );
    let needs = body
        .lines()
        .find_map(|line| line.strip_prefix("    needs:"))
        .map(str::trim);
    match needs {
        Some(needed) => {
            assert!(
                !needed.is_empty()
                    && needed
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "{site} needs `{needed}`, which this test cannot read"
            );
            let upstream = job_in_the_queue(file, jobs, needed);
            if own == InTheQueue::Runs {
                upstream
            } else {
                own
            }
        }
        None => own,
    }
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
    assert_eq!(files, ["ci.yml", "mutants.yml"]);
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
        ("docs/guides/quickstart.md", "# Quickstart — one spine, and everything else is a diff over it\n## The spine\n### Step 1 — install\n### Step 2 — `brokkr init .`\n### Step 3 — `brokkr run`\n### Step 4 — read the journal\n## Per-stack cards\n## Flow 2 — deliver\n## Flow 3 — adopt\n## After the spine\n### Where the run wrote things\n### The escape hatches\n#### Operator commands — `retry`, `stop` and `supersede`\n#### Resume\n#### Conclude — closing a run whose bundle no longer compiles\n#### Re-run under another strategy\n### What it cost\n### What the budgets do not cover\n### Limits worth knowing\n## Compact first-run tour\n## Next", "\n---\n1\n2\n3\n4\nChannel\ntarball\ncargo\nnix\napt\ndnf\nbrew\nFlag\n`--repo <path>`\n`--db <path>`\n`--realms <file>`\n`--recipes-dir <path>`\n`--secrets-file <path>`\n`--dispatch <file>`\nCard\n[node](cards/node.md)\n[bun](cards/bun.md)\n[rust](cards/rust.md)\n[go](cards/go.md)\n[python](cards/python.md)", ""),
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
