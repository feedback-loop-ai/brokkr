//! Every tool that decides a verdict is pinned (issue #340).
//!
//! A tag or a branch can move under a green check with no change in this
//! repository: `dtolnay/rust-toolchain@stable` let a new stable Clippy
//! turn `main` red, and `taiki-e/install-action@cargo-llvm-cov` let the
//! measuring tool of the exact gate change its own ignore regex. So every
//! `uses:` names a commit, with its release in a trailing comment for the
//! reviewer and for Dependabot, and every Rust toolchain a workflow
//! installs is an exact version this repository records once. This rides
//! the required `test` checks, so it needs no job of its own.
//!
//! The files are read by a line scanner, not a YAML parser, so the scanner
//! fails closed: a line it cannot read as a plain `key: value` or a plain
//! scalar is refused by name rather than guessed at.

use std::path::{Path, PathBuf};

const BUBBLEWRAP_ACTION: &str = ".github/actions/setup-bubblewrap/action.yml";
const BUBBLEWRAP: &str = "uses: ./.github/actions/setup-bubblewrap";
const CARGO_AUDIT_ACTION: &str = ".github/actions/setup-cargo-audit/action.yml";
const NIGHTLY_INPUT: &str = "${{ steps.nightly.outputs.toolchain }}";
const LLVM_COV_INPUT: &str = "${{ steps.nightly.outputs.cargo_llvm_cov }}";

/// A step a job must carry line for line, indentation aside, with `keys`
/// keys after its `- ` line and nothing but its script after them.
struct Step {
    text: &'static [&'static str],
    keys: usize,
}

/// The one step whose outputs may feed [`NIGHTLY_INPUT`] and
/// [`LLVM_COV_INPUT`]: it reads the two pin files and writes nothing else.
const NIGHTLY: Step = Step {
    text: &[
        "- name: the pinned coverage toolchain and its measuring tool",
        "id: nightly",
        "run: |",
        r#"echo "toolchain=$(tr -d '[:space:]' < rust-nightly-version.txt)" >> "$GITHUB_OUTPUT""#,
        r#"echo "cargo_llvm_cov=$(tr -d '[:space:]' < cargo-llvm-cov-version.txt)" >> "$GITHUB_OUTPUT""#,
        r#"echo "RUSTUP_TOOLCHAIN=$(tr -d '[:space:]' < rust-nightly-version.txt)" >> "$GITHUB_ENV""#,
    ],
    keys: 2,
};

/// The step that installs the digest-pinned cargo-audit, with no `if:` or
/// `continue-on-error:` that could leave `rustsec/audit-check` to build
/// the latest one from source.
const SETUP_CARGO_AUDIT: Step = Step {
    text: &[
        "- name: cargo-audit, pinned by digest",
        "uses: ./.github/actions/setup-cargo-audit",
    ],
    keys: 1,
};

/// This file lives at `crates/brokkr-cli/tests/`.
fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = workspace().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The entries of a `.github` directory, relative to the workspace, sorted.
fn listing(directory: &str) -> Vec<String> {
    let path = workspace().join(directory);
    let entries =
        std::fs::read_dir(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut names: Vec<String> = entries
        .map(|entry| {
            let name = entry.expect(directory).file_name();
            format!("{directory}/{}", name.to_str().expect("a UTF-8 name"))
        })
        .collect();
    names.sort();
    names
}

/// Every workflow GitHub would run, read from the directory rather than
/// named here, so a new file is judged the day it arrives.
fn workflows() -> Vec<String> {
    listing(".github/workflows")
        .into_iter()
        .filter(|file| file.ends_with(".yml") || file.ends_with(".yaml"))
        .collect()
}

/// The metadata file of a local action directory, as GitHub looks it up.
fn action_file(directory: &Path) -> Option<PathBuf> {
    ["action.yml", "action.yaml"]
        .map(|file| directory.join(file))
        .into_iter()
        .find(|file| file.is_file())
}

/// Every local action's metadata file, one per `.github/actions/<name>/`.
fn local_actions() -> Vec<String> {
    let root = workspace();
    listing(".github/actions")
        .into_iter()
        .filter_map(|action| action_file(&root.join(action)))
        .map(|file| {
            let relative = file.strip_prefix(&root).expect("under the workspace");
            relative.to_str().expect("a UTF-8 path").to_string()
        })
        .collect()
}

/// The value of a `key = "value"` or `key: value` line, first match wins.
fn field(text: &str, key: &str) -> String {
    text.lines()
        .find_map(|line| {
            let rest = line.trim().strip_prefix(key)?.trim_start();
            let rest = rest.strip_prefix(['=', ':'])?;
            Some(rest.trim().trim_matches('"').to_string())
        })
        .unwrap_or_else(|| panic!("no {key} in:\n{text}"))
}

/// The exact toolchains a workflow may name: rust-toolchain.toml's stable
/// channel and the MSRV Cargo.toml declares. The coverage nightly is
/// [`NIGHTLY_INPUT`], and only in a job that carries [`NIGHTLY`].
fn pinned_toolchains(root: &Path) -> [String; 2] {
    let file = |name: &str| std::fs::read_to_string(root.join(name)).expect(name);
    [
        field(&file("rust-toolchain.toml"), "channel"),
        format!("{}.0", field(&file("Cargo.toml"), "rust-version")),
    ]
}

#[derive(Debug, PartialEq, Eq)]
enum Unpinned {
    /// A `uses:` whose ref is a tag or a branch rather than a commit.
    Reference(String),
    /// A commit with no `# <release>` comment naming what it is.
    Unlabelled(String),
    /// A `./` action that is not in the repository.
    LocalAction(String),
    /// A `dtolnay/rust-toolchain` step with no `toolchain:` input: the
    /// pinned master ref installs nothing without one.
    ToolchainMissing,
    /// A toolchain that is not one of the repository's recorded pins.
    Toolchain(String),
    /// An install-action `tool:` entry that names no exact release, which
    /// installs whatever is newest.
    Tool(String),
    /// A `rustsec/audit-check` with no [`SETUP_CARGO_AUDIT`] before it in
    /// its job: the action would build the newest cargo-audit from source.
    CargoAuditMissing,
    /// A line the scanner cannot read as a plain `key: value` or a plain
    /// scalar — a quoted, anchored, tagged, merge or complex key, an
    /// alias, or a pinned value folded onto the next line.
    Unread(String),
    /// A flow collection, `{…}` or `[…]`, that names an action or a tool
    /// or escapes a character: its keys are not read, so it is refused.
    Flow(String),
}

#[derive(Debug, PartialEq, Eq)]
struct Offense {
    line: usize,
    what: Unpinned,
}

/// An exact `MAJOR.MINOR.PATCH` release, never a channel or `latest`.
fn is_release(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

fn is_commit(reference: &str) -> bool {
    reference.len() == 40
        && reference
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// A quoted scalar's text, when its quotes hold no escape; anything else
/// is returned as written, quotes included, and so matches no pin.
fn unquote(value: &str) -> &str {
    ['"', '\'']
        .into_iter()
        .find_map(|quote| {
            let inner = value.strip_prefix(quote)?.strip_suffix(quote)?;
            (!inner.contains(['\\', quote])).then_some(inner)
        })
        .unwrap_or(value)
}

/// A plain value with its trailing comment and its quotes removed.
fn scalar(value: &str) -> &str {
    unquote(
        value
            .split_once(" #")
            .map_or(value, |(value, _)| value)
            .trim(),
    )
}

fn judge_uses(root: &Path, value: &str) -> Option<Unpinned> {
    let (action, comment) = value.split_once(" # ").unwrap_or((value, ""));
    let action = unquote(action.trim());
    if let Some(local) = action.strip_prefix("./") {
        let present = action_file(&root.join(local)).is_some();
        return (!present).then(|| Unpinned::LocalAction(action.to_string()));
    }
    let reference = action
        .split_once('@')
        .map_or("", |(_, reference)| reference);
    if !is_commit(reference) {
        return Some(Unpinned::Reference(action.to_string()));
    }
    comment
        .trim()
        .is_empty()
        .then(|| Unpinned::Unlabelled(action.to_string()))
}

/// The install-action `tool:` entries that do not name an exact release:
/// each comma-separated entry is `name@MAJOR.MINOR.PATCH`, or, in a job
/// that carries [`NIGHTLY`], the cargo-llvm-cov version it read.
fn floating_tools(value: &str, reads_the_pins: bool) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|entry| {
            let version = entry.split_once('@').map_or("", |(_, version)| version);
            !is_release(version) && !(reads_the_pins && version == LLVM_COV_INPUT)
        })
        .map(str::to_string)
        .collect()
}

/// What one line of YAML structure holds, read only as far as this check
/// needs, and refused where reading on would mean guessing.
enum Entry<'a> {
    /// A plain `key: value`, the key lowercased: GitHub reads an action's
    /// inputs without regard to case.
    Pair(String, &'a str),
    /// A sequence item or a scalar with no key.
    Scalar(&'a str),
    /// See [`Unpinned::Unread`].
    Unread,
    /// See [`Unpinned::Flow`].
    Flow,
}

/// One line of structure. The body of a block scalar (a `run: |` script)
/// is text, not structure, and yields no node.
struct Node<'a> {
    /// The line's index in the file.
    index: usize,
    /// The column of its first character, any `- ` included.
    indent: usize,
    /// The column of its key or scalar, after any `- `.
    column: usize,
    entry: Entry<'a>,
}

impl Node<'_> {
    fn key(&self) -> Option<&str> {
        match &self.entry {
            Entry::Pair(key, _) => Some(key),
            _ => None,
        }
    }
}

/// The line after the `- ` of each sequence item it opens.
fn strip_items(mut rest: &str) -> &str {
    while let Some(item) = rest
        .strip_prefix('-')
        .filter(|item| item.is_empty() || item.starts_with(char::is_whitespace))
    {
        rest = item.trim_start();
    }
    rest
}

/// A key GitHub reads as written: no quotes, escapes, anchors or tags.
fn is_plain_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// A flow collection, after any anchor or tag in front of it.
fn is_flow(mut text: &str) -> bool {
    while text.starts_with(['&', '!']) {
        text = text
            .split_once(char::is_whitespace)
            .map_or("", |(_, rest)| rest)
            .trim_start();
    }
    text.starts_with(['{', '['])
}

/// Whether a flow collection could hide a pin: it names `uses` or a
/// `tool` in any case, or escapes a character that could spell one.
fn hides_a_pin(flow: &str) -> bool {
    let lower = flow.to_ascii_lowercase();
    lower.contains("uses") || lower.contains("tool") || flow.contains('\\')
}

fn read_entry(rest: &str) -> Entry<'_> {
    if is_flow(rest) {
        return if hides_a_pin(rest) {
            Entry::Flow
        } else {
            Entry::Scalar(rest)
        };
    }
    if rest.starts_with(['?', '*']) {
        return Entry::Unread;
    }
    let separator = rest.char_indices().find(|&(at, c)| {
        c == ':'
            && rest[at + 1..]
                .chars()
                .next()
                .is_none_or(char::is_whitespace)
    });
    let Some((at, _)) = separator else {
        return Entry::Scalar(rest);
    };
    let (key, value) = (rest[..at].trim_end(), rest[at + 1..].trim());
    if !is_plain_key(key) || value.starts_with('*') {
        return Entry::Unread;
    }
    if is_flow(value) && hides_a_pin(value) {
        return Entry::Flow;
    }
    Entry::Pair(key.to_ascii_lowercase(), value)
}

/// A block scalar's header, `|` or `>` with its indicators.
fn is_block_header(value: &str) -> bool {
    let header = value.split_once(" #").map_or(value, |(header, _)| header);
    header
        .trim()
        .strip_prefix(['|', '>'])
        .is_some_and(|indicators| {
            indicators
                .chars()
                .all(|c| matches!(c, '1'..='9' | '+' | '-'))
        })
}

/// The structure of a file, line by line: comments, blank lines and the
/// lines indented under a block scalar's header are skipped.
fn structure<'a>(lines: &[&'a str]) -> Vec<Node<'a>> {
    let mut nodes = Vec::new();
    let mut body_under: Option<usize> = None;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if body_under.is_some_and(|column| trimmed.is_empty() || indent > column) {
            continue;
        }
        body_under = None;
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let rest = strip_items(trimmed);
        let column = line.len() - rest.len();
        let entry = read_entry(rest);
        if let Entry::Pair(_, text) | Entry::Scalar(text) = &entry {
            if is_block_header(text) {
                body_under = Some(column);
            }
        }
        nodes.push(Node {
            index,
            indent,
            column,
            entry,
        });
    }
    nodes
}

/// The job each node belongs to, as the index of the node naming it. A
/// file with no top-level `jobs:` (a local action) is one scope.
fn scopes(nodes: &[Node]) -> Vec<Option<usize>> {
    let Some(jobs) = nodes
        .iter()
        .position(|node| node.indent == 0 && node.key() == Some("jobs"))
    else {
        return vec![Some(0); nodes.len()];
    };
    let job_indent = nodes.get(jobs + 1).map(|node| node.indent);
    let mut job = None;
    nodes
        .iter()
        .enumerate()
        .map(|(at, node)| {
            if at <= jobs || node.indent == 0 {
                job = None;
            } else if Some(node.indent) == job_indent {
                job = Some(at);
            }
            job
        })
        .collect()
}

/// One workflow or action file, as the judge reads it.
struct Scan<'a> {
    root: &'a Path,
    lines: Vec<&'a str>,
    nodes: Vec<Node<'a>>,
    scopes: Vec<Option<usize>>,
    toolchains: [String; 2],
}

impl Scan<'_> {
    /// Whether the node at `at` opens `step`, verbatim: its keys belong to
    /// that step, and after them come only the lines of its script, then
    /// the next step, with nothing added.
    fn is_step(&self, at: usize, step: &Step) -> bool {
        let item = &self.nodes[at];
        let end = item.index + step.text.len();
        let verbatim = self.lines.get(item.index..end).is_some_and(|text| {
            text.iter()
                .map(|line| line.trim())
                .eq(step.text.iter().copied())
        });
        let keys = self.nodes.get(at + 1..=at + step.keys).is_some_and(|keys| {
            keys.iter()
                .zip(1..)
                .all(|(key, offset)| key.index == item.index + offset && key.indent == item.column)
        });
        let next = self.nodes.get(at + step.keys + 1);
        let body_ends = next.map_or(self.lines.len(), |node| node.index);
        let closed = next.is_none_or(|node| node.indent <= item.indent)
            && body_ends >= end
            && self.lines[end..body_ends].iter().all(|line| {
                let text = line.trim_start();
                text.is_empty() || (text.starts_with('#') && line.len() - text.len() <= item.column)
            });
        verbatim && keys && closed
    }

    /// Whether the job of the node at `at` carries `step` before `before`.
    fn job_has_step(&self, at: usize, step: &Step, before: usize) -> bool {
        let job = self.scopes[at];
        job.is_some()
            && (0..before).any(|other| self.scopes[other] == job && self.is_step(other, step))
    }

    /// Whether the step whose `uses:` is the node at `at` passes a
    /// `toolchain:` input. The step's nodes are the ones indented at least
    /// as deep as that key; its keys sit exactly at it, and the input
    /// counts only under the `with:` key, not under `env:` or any other.
    fn step_names_a_toolchain(&self, at: usize) -> bool {
        let column = self.nodes[at].column;
        let step = self.nodes[at + 1..]
            .iter()
            .take_while(|node| node.indent >= column);
        let mut in_with = false;
        for node in step {
            if node.indent == column {
                in_with = node.key() == Some("with");
            } else if in_with && node.key() == Some("toolchain") {
                return true;
            }
        }
        false
    }

    fn judge_uses(&self, at: usize, value: &str) -> Option<Unpinned> {
        let action = scalar(value).to_ascii_lowercase();
        let toolchain_missing =
            action.starts_with("dtolnay/rust-toolchain@") && !self.step_names_a_toolchain(at);
        let audit_unpinned = action.starts_with("rustsec/audit-check@")
            && !self.job_has_step(at, &SETUP_CARGO_AUDIT, at);
        judge_uses(self.root, value)
            .or_else(|| toolchain_missing.then_some(Unpinned::ToolchainMissing))
            .or_else(|| audit_unpinned.then_some(Unpinned::CargoAuditMissing))
    }

    /// Every unpinned tool the node at `at` names.
    fn judge(&self, at: usize) -> Vec<Unpinned> {
        let node = &self.nodes[at];
        let line = self.lines[node.index].trim().to_string();
        let (key, value) = match &node.entry {
            Entry::Pair(key, value) => (key.as_str(), *value),
            Entry::Scalar(_) => return Vec::new(),
            Entry::Unread => return vec![Unpinned::Unread(line)],
            Entry::Flow => return vec![Unpinned::Flow(line)],
        };
        let judged = matches!(key, "uses" | "toolchain" | "tool");
        let folded = self
            .nodes
            .get(at + 1)
            .is_some_and(|next| next.indent > node.column);
        if judged && folded {
            return vec![Unpinned::Unread(line)];
        }
        let reads_the_pins = || self.job_has_step(at, &NIGHTLY, self.nodes.len());
        match key {
            "uses" => self.judge_uses(at, value).into_iter().collect(),
            "toolchain" => {
                let toolchain = scalar(value);
                let pinned = self.toolchains.iter().any(|pin| pin == toolchain)
                    || (toolchain == NIGHTLY_INPUT && reads_the_pins());
                (!pinned)
                    .then(|| Unpinned::Toolchain(toolchain.to_string()))
                    .into_iter()
                    .collect()
            }
            "tool" => floating_tools(scalar(value), reads_the_pins())
                .into_iter()
                .map(Unpinned::Tool)
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// Every unpinned tool in one workflow or action file.
fn offenses_in(root: &Path, text: &str) -> Vec<Offense> {
    let lines: Vec<&str> = text.lines().collect();
    let nodes = structure(&lines);
    let scan = Scan {
        root,
        scopes: scopes(&nodes),
        toolchains: pinned_toolchains(root),
        lines,
        nodes,
    };
    (0..scan.nodes.len())
        .flat_map(|at| {
            let line = scan.nodes[at].index + 1;
            scan.judge(at)
                .into_iter()
                .map(move |what| Offense { line, what })
        })
        .collect()
}

#[test]
fn every_workflow_pins_each_action_by_commit_and_each_toolchain_by_version() {
    // The stable pin is an exact release: a toolchain bump is a reviewed
    // pull request that edits this file, never a release upstream.
    let toolchain = read("rust-toolchain.toml");
    let channel = field(&toolchain, "channel");
    assert!(is_release(&channel), "{channel} is not MAJOR.MINOR.PATCH");
    assert_eq!(field(&toolchain, "components"), r#"["clippy", "rustfmt"]"#);

    // The directories are listed, not named, so a workflow or an action
    // added later is judged too; the known files prove the listing read.
    let workflows = workflows();
    let actions = local_actions();
    for known in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        assert!(workflows.iter().any(|file| file == known), "{workflows:?}");
    }
    for known in [BUBBLEWRAP_ACTION, CARGO_AUDIT_ACTION] {
        assert!(actions.iter().any(|file| file == known), "{actions:?}");
    }

    let root = workspace();
    for file in workflows.iter().chain(&actions) {
        assert_eq!(offenses_in(&root, &read(file)), [], "{file}");
    }
}

/// The refusal itself bites: each way a pin can float is planted and
/// named, with the line it is on.
#[test]
fn a_floating_action_or_toolchain_is_refused_by_name() {
    let root = workspace();
    let sha = "11d5960a326750d5838078e36cf38b85af677262";
    let planted = format!(
        "      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@{sha}
      - uses: dtolnay/rust-toolchain@{sha} # master
      - name: next step
        uses: ./.github/actions/not-there
      - uses: dtolnay/rust-toolchain@{sha} # master
        with:
          toolchain: nightly
      - uses: taiki-e/install-action@{sha} # v2.87.20
        with:
          tool: cargo-llvm-cov
      - uses: actions/checkout@{sha} # v4.4.0
      - uses: ./.github/actions/setup-bubblewrap
      - uses: dtolnay/rust-toolchain@{sha} # master
        with:
          toolchain: {NIGHTLY_INPUT}
      - uses: taiki-e/install-action@{sha} # v2.87.20
        with:
          tool: cargo-llvm-cov@latest
      - uses: taiki-e/install-action@{sha} # v2.87.20
        with:
          tool: cargo-llvm-cov@stable
      - uses: taiki-e/install-action@{sha} # v2.87.20
        with:
          tool: cargo-llvm-cov@0.9.1, cargo-nextest, cargo-deny@0.20.2
      - uses: taiki-e/install-action@{sha} # v2.87.20
        with:
          tool: cargo-llvm-cov@{LLVM_COV_INPUT}
      - uses: dtolnay/rust-toolchain@{sha} # master
        env:
          toolchain: 1.98.0
        with:
          components: clippy
"
    );
    let at = |line, what| Offense { line, what };
    assert_eq!(
        offenses_in(&root, &planted),
        [
            at(1, Unpinned::Reference("actions/checkout@v4".into())),
            at(
                2,
                Unpinned::Reference("dtolnay/rust-toolchain@stable".into())
            ),
            at(
                3,
                Unpinned::Unlabelled(format!("Swatinem/rust-cache@{sha}"))
            ),
            at(4, Unpinned::ToolchainMissing),
            at(
                6,
                Unpinned::LocalAction("./.github/actions/not-there".into())
            ),
            at(9, Unpinned::Toolchain("nightly".into())),
            at(12, Unpinned::Tool("cargo-llvm-cov".into())),
            // No step here reads the pin files, so their outputs are
            // no pin at all.
            at(17, Unpinned::Toolchain(NIGHTLY_INPUT.into())),
            at(20, Unpinned::Tool("cargo-llvm-cov@latest".into())),
            at(23, Unpinned::Tool("cargo-llvm-cov@stable".into())),
            at(26, Unpinned::Tool("cargo-nextest".into())),
            at(
                29,
                Unpinned::Tool(format!("cargo-llvm-cov@{LLVM_COV_INPUT}"))
            ),
            at(30, Unpinned::ToolchainMissing),
        ]
    );
}

/// The scanner fails closed: each spelling YAML allows for a key it does
/// not read plainly is refused, and what it does read (quotes, a script's
/// body) is read as GitHub reads it.
#[test]
fn a_line_the_scanner_cannot_read_plainly_is_refused() {
    let root = workspace();
    let sha = "11d5960a326750d5838078e36cf38b85af677262";
    let planted = format!(
        "      - uses : actions/checkout@v4
      - {{uses: actions/checkout@v4}}
      - \"uses\": actions/checkout@v4
      - uses: taiki-e/install-action@{sha} # v2.87.20
        with: {{tool: cargo-llvm-cov}}
      - uses: dtolnay/rust-toolchain@{sha} # master
        With:
          Toolchain: stable
      - &step uses: actions/checkout@v4
      - <<: *step
      - ? uses
        : actions/checkout@v4
      - *step
      - uses: taiki-e/install-action@{sha} # v2.87.20
        with:
          tool: cargo-llvm-cov@0.9.1
            cargo-nextest
      - name: &pinned {{ uses: actions/checkout@v4 }}
      - uses: \"actions/checkout@{sha}\" # v4.4.0
      - run: |
          uses: actions/checkout@v4
          awk '{{ print }}'
      - uses: ./.github/actions/setup-cargo-audit
"
    );
    let at = |line, what| Offense { line, what };
    let unread = |line: &str| Unpinned::Unread(line.into());
    let flow = |line: &str| Unpinned::Flow(line.into());
    assert_eq!(
        offenses_in(&root, &planted),
        [
            at(1, Unpinned::Reference("actions/checkout@v4".into())),
            at(2, flow("- {uses: actions/checkout@v4}")),
            at(3, unread("- \"uses\": actions/checkout@v4")),
            at(5, flow("with: {tool: cargo-llvm-cov}")),
            at(8, Unpinned::Toolchain("stable".into())),
            at(9, unread("- &step uses: actions/checkout@v4")),
            at(10, unread("- <<: *step")),
            at(11, unread("- ? uses")),
            at(12, unread(": actions/checkout@v4")),
            at(13, unread("- *step")),
            at(16, unread("tool: cargo-llvm-cov@0.9.1")),
            at(18, flow("- name: &pinned { uses: actions/checkout@v4 }")),
        ]
    );
}

/// The lines of `step` laid out as a job's step.
fn step_text(step: &Step) -> String {
    step.text
        .iter()
        .enumerate()
        .map(|(at, line)| match at {
            0 => format!("      {line}\n"),
            at if at <= step.keys => format!("        {line}\n"),
            _ => format!("          {line}\n"),
        })
        .collect()
}

/// The pin files' outputs and the pinned cargo-audit count only in the
/// job that carries their step verbatim, and before audit-check runs.
#[test]
fn a_pin_read_or_an_audit_set_up_counts_only_in_its_own_job() {
    let root = workspace();
    let sha = "11d5960a326750d5838078e36cf38b85af677262";
    let (nightly, audit) = (step_text(&NIGHTLY), step_text(&SETUP_CARGO_AUDIT));
    let planted = format!(
        "\
jobs:
  reads:
    steps:
{nightly}      - uses: dtolnay/rust-toolchain@{sha} # master
        with:
          toolchain: {NIGHTLY_INPUT}
      - uses: taiki-e/install-action@{sha} # v2.87.20
        with:
          tool: cargo-llvm-cov@{LLVM_COV_INPUT}
{audit}      - uses: rustsec/audit-check@{sha} # v2.0.0
  extended:
    steps:
{nightly}          echo \"toolchain=nightly\" >> \"$GITHUB_OUTPUT\"
      - uses: dtolnay/rust-toolchain@{sha} # master
        with:
          toolchain: {NIGHTLY_INPUT}
      - name: cargo-audit, pinned by digest
        if: false
        uses: ./.github/actions/setup-cargo-audit
      - uses: rustsec/audit-check@{sha} # v2.0.0
  borrows:
    steps:
      - uses: taiki-e/install-action@{sha} # v2.87.20
        with:
          tool: cargo-llvm-cov@{LLVM_COV_INPUT}
      - uses: rustsec/audit-check@{sha} # v2.0.0
{audit}"
    );
    let at = |line, what| Offense { line, what };
    assert_eq!(
        offenses_in(&root, &planted),
        [
            at(30, Unpinned::Toolchain(NIGHTLY_INPUT.into())),
            at(34, Unpinned::CargoAuditMissing),
            at(
                39,
                Unpinned::Tool(format!("cargo-llvm-cov@{LLVM_COV_INPUT}"))
            ),
            at(40, Unpinned::CargoAuditMissing),
        ]
    );
}

/// A tool CI fetches is unpacked only after its tarball matched the
/// digest its one action records, and no workflow fetches it around that
/// action.
fn assert_fetched_by_digest(action_file: &str, digest: &str, download: &str, unpack: &str) {
    for file in workflows() {
        assert!(
            !read(&file).contains(download),
            "{file} fetches {download} itself"
        );
    }
    let action = read(action_file);
    let sha256 = field(&action, digest);
    assert!(
        sha256.len() == 64 && sha256.chars().all(|c| c.is_ascii_hexdigit()),
        "{sha256} is not a sha256"
    );
    let at = |needle: &str| {
        action
            .find(needle)
            .unwrap_or_else(|| panic!("no {needle} in:\n{action}"))
    };
    let fetch = at("curl -sSfL --retry 3 -o \"$tarball\"");
    let check = at(&format!(
        "echo \"${{{digest}}}  ${{tarball}}\" | sha256sum -c -"
    ));
    let unpack = at(unpack);
    assert!(fetch < check && check < unpack, "{action}");
    assert!(action.contains("set -euo pipefail"), "{action}");
}

/// Bubblewrap decides every boxed verdict, so it is built from one place,
/// and only from a tarball whose digest matched before it was unpacked.
#[test]
fn bubblewrap_is_built_once_from_a_tarball_verified_by_digest() {
    let sites: Vec<(String, usize)> = workflows()
        .into_iter()
        .map(|file| {
            let count = read(&file).matches(BUBBLEWRAP).count();
            (file, count)
        })
        .filter(|(_, count)| *count > 0)
        .collect();
    assert_eq!(
        sites,
        [
            (".github/workflows/ci.yml".to_string(), 3),
            (".github/workflows/mutants.yml".to_string(), 2),
            (".github/workflows/release.yml".to_string(), 2),
            (".github/workflows/research-weekly.yml".to_string(), 1),
        ]
    );
    assert_fetched_by_digest(
        BUBBLEWRAP_ACTION,
        "BWRAP_SHA256",
        "bubblewrap/releases/download",
        r#"tar xJf "$tarball""#,
    );
}

/// cargo-audit is installed from one place, from a release verified by
/// digest; every audit-check follows it in its job, which the pin scan
/// above holds as [`Unpinned::CargoAuditMissing`].
#[test]
fn cargo_audit_is_installed_once_from_a_release_verified_by_digest() {
    let sites: Vec<(String, usize)> = workflows()
        .into_iter()
        .map(|file| {
            let count = read(&file).matches("uses: rustsec/audit-check@").count();
            (file, count)
        })
        .filter(|(_, count)| *count > 0)
        .collect();
    assert_eq!(
        sites,
        [
            (".github/workflows/ci.yml".to_string(), 1),
            (".github/workflows/release.yml".to_string(), 1),
        ]
    );
    assert_fetched_by_digest(
        CARGO_AUDIT_ACTION,
        "CARGO_AUDIT_SHA256",
        "rustsec/rustsec/releases/download",
        r#"tar xzf "$tarball""#,
    );
}

/// The job bodies of ci.yml, keyed by job id, in file order.
fn ci_jobs() -> Vec<(String, String)> {
    let workflow = read(".github/workflows/ci.yml");
    let (_, jobs) = workflow.split_once("\njobs:\n").expect("a jobs map");
    let mut parsed: Vec<(String, String)> = Vec::new();
    for line in jobs.lines() {
        let id = line
            .strip_prefix("  ")
            .filter(|rest| !rest.starts_with([' ', '#']))
            .and_then(|rest| rest.strip_suffix(':'));
        match (id, parsed.last_mut()) {
            (Some(id), _) => parsed.push((id.to_string(), String::new())),
            (None, Some((_, body))) => {
                body.push_str(line);
                body.push('\n');
            }
            (None, None) => {}
        }
    }
    parsed
}

/// The rest of issue #340's CI shape: superseded pull request runs are
/// cancelled, every job is bounded, the Linux release binary is built
/// once and shared, and the reporting Gate B probe is off the required
/// macOS job's critical path.
#[test]
fn ci_cancels_superseded_runs_bounds_every_job_and_builds_once() {
    let workflow = read(".github/workflows/ci.yml");
    assert!(
        workflow.contains(
            "concurrency:\n  group: ${{ github.workflow }}-${{ github.event_name == 'pull_request' && github.ref || github.sha }}\n  cancel-in-progress: ${{ github.event_name == 'pull_request' }}\n"
        ),
        "ci.yml does not cancel a superseded pull request run, or groups a main one with another commit"
    );

    let jobs = ci_jobs();
    let ids: Vec<&str> = jobs.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "delivered-by-brokkr",
            "msrv",
            "quality",
            "engine",
            "seatbelt-lifetime",
            "coverage",
            "license-compliance",
            "dependency-audit",
            "bootstrap-budgets",
            "bootstrap-budgets-macos",
            "packaging",
            "flake",
            "release-binary",
        ]
    );
    for (id, body) in &jobs {
        assert!(body.contains("    timeout-minutes: "), "{id} is unbounded");
    }

    let job = |wanted: &str| {
        jobs.iter()
            .find(|(id, _)| id == wanted)
            .map(|(_, body)| body.as_str())
            .unwrap_or_else(|| panic!("no {wanted} job"))
    };
    let builders: Vec<&str> = jobs
        .iter()
        .filter(|(_, body)| body.contains("cargo build --release"))
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(builders, ["bootstrap-budgets-macos", "release-binary"]);
    for consumer in ["bootstrap-budgets", "packaging"] {
        let body = job(consumer);
        assert!(body.contains("    needs: release-binary\n"), "{consumer}");
        assert!(
            body.contains("          name: brokkr-linux-x86_64\n"),
            "{consumer}"
        );
        assert!(
            body.contains("chmod +x target/release/brokkr"),
            "{consumer}"
        );
    }

    let gate_b = "native_lifetime_feasibility_probe";
    assert!(!job("engine").contains(gate_b), "Gate B is back in engine");
    assert!(!job("engine").contains("continue-on-error"), "engine");
    assert!(job("seatbelt-lifetime").contains(gate_b));
    assert!(job("seatbelt-lifetime").contains("    runs-on: macos-latest\n"));
}
