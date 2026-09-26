//! Every tool that decides a verdict is pinned (issue #340).
//!
//! A tag or a branch can move under a green check with no change in this
//! repository: `dtolnay/rust-toolchain@stable` let a new stable Clippy
//! turn `main` red, and `taiki-e/install-action@cargo-llvm-cov` let the
//! measuring tool of the exact gate change its own ignore regex. So every
//! `uses:` names a commit, with its release in a trailing comment for the
//! reviewer and for Renovate, and every Rust toolchain a workflow
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

/// The one home of the Renovate image, by release and digest: the Renovate
/// workflow runs it and ci.yml validates the config from it.
const RENOVATE_IMAGE_FILE: &str = ".github/renovate-image.txt";
const RENOVATE_IMAGE_INPUT: &str = "${{ steps.renovate-image.outputs.ref }}";

/// The one step whose output may feed [`RENOVATE_IMAGE_INPUT`]: it reads
/// [`RENOVATE_IMAGE_FILE`] and writes nothing else.
const RENOVATE_IMAGE: Step = Step {
    text: &[
        "- name: the pinned Renovate image",
        "id: renovate-image",
        "run: |",
        r#"echo "ref=$(tr -d '[:space:]' < .github/renovate-image.txt)" >> "$GITHUB_OUTPUT""#,
    ],
    keys: 2,
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
    /// A commit whose trailing comment does not name its release, `# v27`
    /// or `# v4.4.0`, or, for `dtolnay/rust-toolchain`, which tags no
    /// releases, the branch `# master` its commit was read from.
    Unlabelled(String),
    /// A `./` action that is not one of the repository's own actions, a
    /// `.github/actions/<name>/` directory this scan also reads.
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
    /// A Go or Node runtime a setup action installs that is not an exact
    /// release, or one read from a file this scan does not judge.
    Runtime(String),
    /// A Renovate image named anywhere but its one home: a literal, an
    /// output no step here reads from that home, a `renovate-version`, or a
    /// `renovatebot/github-action` step that names none and so runs the
    /// action's floating default.
    Image(String),
    /// A `taiki-e/install-action` step without `fallback: none`: a tool
    /// version missing from the action's manifest would then install
    /// through cargo-binstall, with no manifest checksum at all.
    Fallback(String),
}

#[derive(Debug, PartialEq, Eq)]
struct Offense {
    line: usize,
    what: Unpinned,
}

/// How many dot-separated decimal numbers `version` is, or `None` when a
/// part is empty or not all digits. The one home of what a version is.
fn numbers(version: &str) -> Option<usize> {
    version.split('.').try_fold(0, |count, part| {
        (!part.is_empty() && part.chars().all(|c| c.is_ascii_digit())).then_some(count + 1)
    })
}

/// An exact `MAJOR.MINOR.PATCH` release, never a channel or `latest`.
fn is_release(version: &str) -> bool {
    numbers(version) == Some(3)
}

/// An image named by exact release and digest: `name:X.Y.Z@sha256:<64 hex>`.
fn is_pinned_image(image: &str) -> bool {
    image
        .split_once('@')
        .and_then(|(reference, digest)| Some((reference.rsplit_once(':')?.1, digest)))
        .is_some_and(|(tag, digest)| {
            is_release(tag)
                && digest.strip_prefix("sha256:").is_some_and(|hex| {
                    hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit())
                })
        })
}

/// A commit's label: `v` and one to three numbers, or `master` for the one
/// action that tags no releases.
fn is_label(action: &str, comment: &str) -> bool {
    let comment = comment.trim();
    let branch_pinned = action
        .to_ascii_lowercase()
        .starts_with("dtolnay/rust-toolchain@");
    (branch_pinned && comment == "master")
        || comment
            .strip_prefix('v')
            .and_then(numbers)
            .is_some_and(|parts| parts <= 3)
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
        // Only a directory directly under `.github/actions/` is one of the
        // actions [`local_actions`] lists, and so one this scan reads.
        let own = local
            .strip_prefix(".github/actions/")
            .is_some_and(|name| !name.is_empty() && !name.contains('/') && !name.starts_with('.'));
        let present = own && action_file(&root.join(local)).is_some();
        return (!present).then(|| Unpinned::LocalAction(action.to_string()));
    }
    let reference = action
        .split_once('@')
        .map_or("", |(_, reference)| reference);
    if !is_commit(reference) {
        return Some(Unpinned::Reference(action.to_string()));
    }
    (!is_label(action, comment)).then(|| Unpinned::Unlabelled(action.to_string()))
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

/// The keys whose value is a map of a step's inputs or environment. A flow
/// collection under one is never read pair by pair, so it is refused
/// whatever it holds: `with: { node-version: 22 }` would otherwise hide a
/// floating runtime from the judged keys below.
const INPUT_MAPS: [&str; 2] = ["with", "env"];

/// Whether a flow collection could hide a pin: it names `uses`, a `tool`,
/// a `version`, a `renovate` input or a `fallback` in any case, or escapes
/// a character that could spell one.
fn hides_a_pin(flow: &str) -> bool {
    let lower = flow.to_ascii_lowercase();
    ["uses", "tool", "version", "renovate", "fallback"]
        .iter()
        .any(|word| lower.contains(word))
        || flow.contains('\\')
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
    if !is_plain_key(key) {
        return Entry::Unread;
    }
    let key = key.to_ascii_lowercase();
    if is_flow(value) && (hides_a_pin(value) || INPUT_MAPS.contains(&key.as_str())) {
        return Entry::Flow;
    }
    // A value behind an anchor (`&a`) or a tag (`!!str`) is not the text
    // the per-action rules match, and an alias (`*a`) is text read from
    // elsewhere: none is read, all are refused. (A flow collection behind
    // either is judged as the flow it is, above.)
    if value.starts_with(['*', '&', '!']) {
        return Entry::Unread;
    }
    Entry::Pair(key, value)
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

    /// The value the step whose `uses:` is the node at `at` passes as its
    /// input `input`. The step's nodes are the ones indented at least as
    /// deep as that key; its keys sit exactly at it, and the input counts
    /// only under the `with:` key, not under `env:` or any other.
    fn step_input(&self, at: usize, input: &str) -> Option<&str> {
        let column = self.nodes[at].column;
        let step = self.nodes[at + 1..]
            .iter()
            .take_while(|node| node.indent >= column);
        let mut in_with = false;
        for node in step {
            if node.indent == column {
                in_with = node.key() == Some("with");
            } else if let (true, Entry::Pair(key, value)) = (in_with, &node.entry) {
                if key == input {
                    return Some(scalar(value));
                }
            }
        }
        None
    }

    fn step_has_input(&self, at: usize, input: &str) -> bool {
        self.step_input(at, input).is_some()
    }

    fn judge_uses(&self, at: usize, value: &str) -> Option<Unpinned> {
        let action = scalar(value).to_ascii_lowercase();
        let toolchain_missing =
            action.starts_with("dtolnay/rust-toolchain@") && !self.step_has_input(at, "toolchain");
        let image_missing = action.starts_with("renovatebot/github-action@")
            && !self.step_has_input(at, "renovate-image");
        let audit_unpinned = action.starts_with("rustsec/audit-check@")
            && !self.job_has_step(at, &SETUP_CARGO_AUDIT, at);
        let falls_back = action.starts_with("taiki-e/install-action@")
            && self.step_input(at, "fallback") != Some("none");
        // A setup action with no version input installs the runner image's
        // own runtime, which floats with the image. (A *-version-file is
        // refused where it is named.)
        let runtime_missing = [("actions/setup-node@", "node"), ("actions/setup-go@", "go")]
            .into_iter()
            .find(|(prefix, _)| action.starts_with(prefix))
            .filter(|(_, runtime)| {
                !self.step_has_input(at, &format!("{runtime}-version"))
                    && !self.step_has_input(at, &format!("{runtime}-version-file"))
            })
            .map(|(_, runtime)| Unpinned::Runtime(format!("{action} with no {runtime}-version")));
        judge_uses(self.root, value)
            .or_else(|| toolchain_missing.then_some(Unpinned::ToolchainMissing))
            .or_else(|| {
                image_missing.then(|| Unpinned::Image(format!("{action} with no renovate-image")))
            })
            .or_else(|| audit_unpinned.then_some(Unpinned::CargoAuditMissing))
            .or_else(|| {
                falls_back.then(|| Unpinned::Fallback(format!("{action} without fallback: none")))
            })
            .or(runtime_missing)
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
        let judged = matches!(
            key,
            "uses"
                | "toolchain"
                | "tool"
                | "go-version"
                | "node-version"
                | "go-version-file"
                | "node-version-file"
                | "renovate-image"
                | "renovate-version"
        );
        let folded = self
            .nodes
            .get(at + 1)
            .is_some_and(|next| next.indent > node.column);
        if judged && folded {
            return vec![Unpinned::Unread(line)];
        }
        // The reading step must run first: an output read before its step
        // is empty, and the action behind it falls to its own default.
        let reads_the_pins = || self.job_has_step(at, &NIGHTLY, at);
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
            "go-version" | "node-version" => {
                let version = scalar(value);
                (!is_release(version))
                    .then(|| Unpinned::Runtime(version.to_string()))
                    .into_iter()
                    .collect()
            }
            "go-version-file" | "node-version-file" => {
                vec![Unpinned::Runtime(scalar(value).to_string())]
            }
            "renovate-image" => {
                let image = scalar(value);
                let from_home =
                    image == RENOVATE_IMAGE_INPUT && self.job_has_step(at, &RENOVATE_IMAGE, at);
                (!from_home)
                    .then(|| Unpinned::Image(image.to_string()))
                    .into_iter()
                    .collect()
            }
            // The image pins the version; this input would float it.
            "renovate-version" => vec![Unpinned::Image(format!(
                "renovate-version: {}",
                scalar(value)
            ))],
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
    // The measuring tool and the packager are exact releases too, judged
    // by the same rule; packaging.rs holds that both workflows read them.
    let llvm_cov = read("cargo-llvm-cov-version.txt");
    assert!(
        is_release(llvm_cov.trim()),
        "{llvm_cov} is not MAJOR.MINOR.PATCH"
    );
    let nfpm = read("packaging/nfpm-version.txt");
    let nfpm = nfpm.trim();
    assert!(
        nfpm.strip_prefix('v')
            .is_some_and(|version| version.starts_with("2.") && is_release(version)),
        "{nfpm} is not a pinned nfpm v2.MINOR.PATCH release"
    );

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
    let fallback = format!("taiki-e/install-action@{sha} without fallback: none");
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
            // None of these install-action steps sets `fallback: none`.
            at(10, Unpinned::Fallback(fallback.clone())),
            at(12, Unpinned::Tool("cargo-llvm-cov".into())),
            // No step here reads the pin files, so their outputs are
            // no pin at all.
            at(17, Unpinned::Toolchain(NIGHTLY_INPUT.into())),
            at(18, Unpinned::Fallback(fallback.clone())),
            at(20, Unpinned::Tool("cargo-llvm-cov@latest".into())),
            at(21, Unpinned::Fallback(fallback.clone())),
            at(23, Unpinned::Tool("cargo-llvm-cov@stable".into())),
            at(24, Unpinned::Fallback(fallback.clone())),
            at(26, Unpinned::Tool("cargo-nextest".into())),
            at(27, Unpinned::Fallback(fallback)),
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
      - uses: actions/setup-node@{sha} # v4.4.0
        with: {{ node-version: 22 }}
      - uses: actions/setup-go@{sha} # v5.6.0
        with: {{ go-version: stable }}
      - uses: actions/setup-node@{sha} # v4.4.0
        with: {{ node-version-file: .nvmrc }}
      - uses: actions/setup-go@{sha} # v5.6.0
        env: {{ GOTOOLCHAIN: local }}
        with: [cache]
      - uses: &x taiki-e/install-action@{sha} # v2.87.20
        with:
          tool: cargo-llvm-cov@0.9.1
      - uses: !!str actions/checkout@{sha} # v4.4.0
      - uses: *x
"
    );
    let at = |line, what| Offense { line, what };
    let unread = |line: &str| Unpinned::Unread(line.into());
    let flow = |line: &str| Unpinned::Flow(line.into());
    let fallback = format!("taiki-e/install-action@{sha} without fallback: none");
    let no_version = |runtime: &str| {
        Unpinned::Runtime(format!(
            "actions/setup-{runtime}@{sha} with no {runtime}-version"
        ))
    };
    assert_eq!(
        offenses_in(&root, &planted),
        [
            at(1, Unpinned::Reference("actions/checkout@v4".into())),
            at(2, flow("- {uses: actions/checkout@v4}")),
            at(3, unread("- \"uses\": actions/checkout@v4")),
            // A flow `with:` is never read, so no `fallback: none` is seen.
            at(4, Unpinned::Fallback(fallback.clone())),
            at(5, flow("with: {tool: cargo-llvm-cov}")),
            at(8, Unpinned::Toolchain("stable".into())),
            at(9, unread("- &step uses: actions/checkout@v4")),
            at(10, unread("- <<: *step")),
            at(11, unread("- ? uses")),
            at(12, unread(": actions/checkout@v4")),
            at(13, unread("- *step")),
            at(14, Unpinned::Fallback(fallback)),
            at(16, unread("tool: cargo-llvm-cov@0.9.1")),
            at(18, flow("- name: &pinned { uses: actions/checkout@v4 }")),
            // A flow under `with:` or `env:` is refused whatever it holds,
            // and the version it hides is never seen by its step.
            at(24, no_version("node")),
            at(25, flow("with: { node-version: 22 }")),
            at(26, no_version("go")),
            at(27, flow("with: { go-version: stable }")),
            at(28, no_version("node")),
            at(29, flow("with: { node-version-file: .nvmrc }")),
            at(30, no_version("go")),
            at(31, flow("env: { GOTOOLCHAIN: local }")),
            at(32, flow("with: [cache]")),
            // An anchored, tagged or aliased value is never read: the
            // anchored install-action step's own rules never ran on it.
            at(
                33,
                unread(&format!(
                    "- uses: &x taiki-e/install-action@{sha} # v2.87.20"
                ))
            ),
            at(
                36,
                unread(&format!("- uses: !!str actions/checkout@{sha} # v4.4.0"))
            ),
            at(37, unread("- uses: *x")),
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
          fallback: none
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
          fallback: none
      - uses: rustsec/audit-check@{sha} # v2.0.0
{audit}  forward:
    steps:
      - uses: dtolnay/rust-toolchain@{sha} # master
        with:
          toolchain: {NIGHTLY_INPUT}
{nightly}"
    );
    let at = |line, what| Offense { line, what };
    assert_eq!(
        offenses_in(&root, &planted),
        [
            at(31, Unpinned::Toolchain(NIGHTLY_INPUT.into())),
            at(35, Unpinned::CargoAuditMissing),
            at(
                40,
                Unpinned::Tool(format!("cargo-llvm-cov@{LLVM_COV_INPUT}"))
            ),
            at(42, Unpinned::CargoAuditMissing),
            // The pins are read after the step that uses them: its output
            // is still empty when the toolchain installs.
            at(49, Unpinned::Toolchain(NIGHTLY_INPUT.into())),
        ]
    );
}

/// A commit's label names a release, a runtime is an exact release, and a
/// `./` action is one of the repository's own. The local cases are planted
/// in a scratch workspace, beside real action files outside
/// `.github/actions/<name>/` that the scan would otherwise never read.
#[test]
fn a_loose_label_a_floating_runtime_or_a_stray_local_action_is_refused() {
    let root = std::env::temp_dir().join(format!("workflow-pins-{}", std::process::id()));
    let actions = [
        ".github/actions/own",
        ".github/actions/own/nested",
        "tools/stray",
    ];
    for action in actions {
        std::fs::create_dir_all(root.join(action)).expect(action);
        std::fs::write(root.join(action).join("action.yml"), "name: planted\n").expect(action);
    }
    for pin in ["rust-toolchain.toml", "Cargo.toml"] {
        std::fs::copy(workspace().join(pin), root.join(pin)).expect(pin);
    }
    let sha = "11d5960a326750d5838078e36cf38b85af677262";
    let planted = format!(
        "      - uses: ./.github/actions/own
      - uses: ./tools/stray
      - uses: ./.github/actions/own/nested
      - uses: ./.github/actions/../../tools/stray
      - uses: actions/checkout@{sha} # v27
      - uses: actions/checkout@{sha} # junk
      - uses: actions/checkout@{sha} # master
      - uses: actions/checkout@{sha} # v4.4.0 or so
      - uses: actions/setup-go@{sha} # v5.6.0
        with:
          go-version: stable
      - uses: actions/setup-node@{sha} # v4.4.0
        with:
          node-version: 22
          NODE-VERSION: '22.23.3'
      - uses: actions/setup-node@{sha} # v4.4.0
        with:
          node-version-file: .nvmrc
      - uses: actions/setup-node@{sha} # v4.4.0
      - uses: actions/setup-go@{sha} # v5.6.0
        with:
          cache: false
"
    );
    let offenses = offenses_in(&root, &planted);
    std::fs::remove_dir_all(&root).expect("scratch workspace");
    let at = |line, what| Offense { line, what };
    let checkout = format!("actions/checkout@{sha}");
    assert_eq!(
        offenses,
        [
            at(2, Unpinned::LocalAction("./tools/stray".into())),
            at(
                3,
                Unpinned::LocalAction("./.github/actions/own/nested".into())
            ),
            at(
                4,
                Unpinned::LocalAction("./.github/actions/../../tools/stray".into())
            ),
            at(6, Unpinned::Unlabelled(checkout.clone())),
            at(7, Unpinned::Unlabelled(checkout.clone())),
            at(8, Unpinned::Unlabelled(checkout)),
            at(11, Unpinned::Runtime("stable".into())),
            at(14, Unpinned::Runtime("22".into())),
            at(18, Unpinned::Runtime(".nvmrc".into())),
            // No version input at all: the runner image's own runtime.
            at(
                19,
                Unpinned::Runtime(format!("actions/setup-node@{sha} with no node-version"))
            ),
            at(
                20,
                Unpinned::Runtime(format!("actions/setup-go@{sha} with no go-version"))
            ),
        ]
    );
}

/// The Renovate image has one home, [`RENOVATE_IMAGE_FILE`], naming an
/// exact release and its digest. Every workflow reaches it through that
/// file: the Renovate step through [`RENOVATE_IMAGE`]'s output in its own
/// job, ci.yml's validation by reading the file. A literal image, even a
/// pinned one, is a second home and is refused.
#[test]
fn the_renovate_image_has_one_home_pinned_by_release_and_digest() {
    let home = read(RENOVATE_IMAGE_FILE);
    assert!(
        is_pinned_image(home.trim()),
        "{home} is not name:X.Y.Z@sha256:<64 hex>"
    );
    for file in workflows().iter().chain(&local_actions()) {
        assert!(
            !read(file).contains("renovatebot/renovate:"),
            "{file} names the Renovate image outside {RENOVATE_IMAGE_FILE}"
        );
    }
    let renovate = read(".github/workflows/renovate.yml");
    assert!(renovate.contains(&format!("renovate-image: {RENOVATE_IMAGE_INPUT}\n")));
    let jobs = ci_jobs();
    let (_, lint) = jobs
        .iter()
        .find(|(id, _)| id == "lint-non-rust")
        .expect("a lint-non-rust job");
    assert!(
        lint.contains(r#"image="$(tr -d '[:space:]' < .github/renovate-image.txt)""#),
        "lint-non-rust does not validate from the pinned image"
    );

    let root = workspace();
    let sha = "f3a31a786096ba6b40d0f0ffe11a494ef73bfa7c";
    let digest = "2327f790a2faf49aafc42cb3b5233f4f52b18fd3081c850f1adc17d2a9af584e";
    let planted = format!(
        "jobs:
  reads:
    steps:
{reads}      - uses: renovatebot/github-action@{sha} # v46.3.4
        with:
          renovate-image: {RENOVATE_IMAGE_INPUT}
  literal:
    steps:
      - uses: renovatebot/github-action@{sha} # v46.3.4
        with:
          renovate-image: ghcr.io/renovatebot/renovate:44.115.9@sha256:{digest}
      - uses: renovatebot/github-action@{sha} # v46.3.4
        with:
          token: t
      - uses: renovatebot/github-action@{sha} # v46.3.4
        with:
          renovate-image: {RENOVATE_IMAGE_INPUT}
          renovate-version: 44.115.9
  forward:
    steps:
      - uses: renovatebot/github-action@{sha} # v46.3.4
        with:
          renovate-image: {RENOVATE_IMAGE_INPUT}
{reads}",
        reads = step_text(&RENOVATE_IMAGE)
    );
    let at = |line, what| Offense { line, what };
    assert_eq!(
        offenses_in(&root, &planted),
        [
            at(
                15,
                Unpinned::Image(format!(
                    "ghcr.io/renovatebot/renovate:44.115.9@sha256:{digest}"
                ))
            ),
            at(
                16,
                Unpinned::Image(format!(
                    "renovatebot/github-action@{sha} with no renovate-image"
                ))
            ),
            // No step in this job reads the home, so its output is no pin.
            at(21, Unpinned::Image(RENOVATE_IMAGE_INPUT.into())),
            at(22, Unpinned::Image("renovate-version: 44.115.9".into())),
            // The home is read after the step that needs it: the output is
            // still empty there, and the action would run its default.
            at(27, Unpinned::Image(RENOVATE_IMAGE_INPUT.into())),
        ]
    );
}

/// Every install-action step disables its fallback: a tool version missing
/// from the pinned manifest must fail, not install through cargo-binstall
/// with no checksum. Only `fallback: none` under `with:` counts.
#[test]
fn an_install_action_step_without_fallback_none_is_refused() {
    let root = workspace();
    let sha = "9983c65e42da123ff25d1f78505eb6de315aa172";
    let planted = format!(
        "      - uses: taiki-e/install-action@{sha} # v2.87.20
        with:
          tool: typos@1.50.2
          fallback: none
      - uses: taiki-e/install-action@{sha} # v2.87.20
        with:
          tool: typos@1.50.2
          fallback: cargo-binstall
      - uses: taiki-e/install-action@{sha} # v2.87.20
        env:
          fallback: none
        with:
          tool: typos@1.50.2
      - uses: taiki-e/install-action@{sha} # v2.87.20
        with:
          tool: typos@1.50.2
"
    );
    let at = |line, what| Offense { line, what };
    let open = || {
        Unpinned::Fallback(format!(
            "taiki-e/install-action@{sha} without fallback: none"
        ))
    };
    assert_eq!(
        offenses_in(&root, &planted),
        [at(5, open()), at(9, open()), at(14, open())]
    );
}

/// Renovate is the one bumper: every tool pinned by version and digest has
/// a Renovate manager moving its version, and the post-upgrade script that
/// re-measures its digest resolves the very release its action downloads.
#[test]
fn renovate_moves_every_digest_pinned_tool_and_its_script_finds_each_release() {
    let root = workspace();
    assert!(
        !root.join(".github/dependabot.yml").exists(),
        "Dependabot is back beside Renovate"
    );
    let config = read(".github/renovate.json5");
    let output = std::process::Command::new("bash")
        .current_dir(&root)
        .args(["scripts/refresh-pin-checksums.sh", "--print-urls"])
        .output()
        .expect("bash");
    assert!(output.status.success(), "{output:?}");
    let printed = String::from_utf8(output.stdout).expect("UTF-8");
    let urls: Vec<(&str, &str)> = printed
        .lines()
        .map(|line| line.split_once(' ').expect("an action and its URL"))
        .collect();
    let mut digested = Vec::new();
    for action in local_actions() {
        let text = read(&action);
        let Some(key) = text.lines().find_map(|line| {
            let (key, _) = line.trim().split_once(": ")?;
            key.ends_with("_SHA256").then(|| key.to_string())
        }) else {
            continue;
        };
        let version_key = key.replace("_SHA256", "_VERSION");
        let version = field(&text, &version_key);
        let (_, url) = urls
            .iter()
            .find(|(file, _)| *file == action)
            .unwrap_or_else(|| panic!("the script does not refresh {action}"));
        assert!(
            url.starts_with("https://github.com/") && !url.contains("${") && url.contains(&version),
            "{action}: {url}"
        );
        assert!(
            config.contains(&format!("{version_key}: (?<currentValue>")),
            "no Renovate manager moves {action}'s {version_key}"
        );
        let directory = action
            .strip_suffix("/action.yml")
            .and_then(|path| path.strip_prefix(".github/actions/"))
            .expect("an action directory");
        assert!(
            config.contains(&format!("{directory}/action\\\\.yml$/")),
            "no Renovate manager reads {action}"
        );
        digested.push(action);
    }
    assert_eq!(
        digested,
        [
            ".github/actions/setup-actionlint/action.yml",
            BUBBLEWRAP_ACTION,
            CARGO_AUDIT_ACTION,
            ".github/actions/setup-jscpd/action.yml",
            ".github/actions/setup-lychee/action.yml",
        ]
    );
    assert_eq!(urls.len(), digested.len(), "{printed}");
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

/// actionlint and lychee, which rule on the workflows and the links, are
/// installed from one action each, from a release verified by digest.
#[test]
fn the_lint_tools_are_installed_from_releases_verified_by_digest() {
    for (action, digest, download, unpack, expected) in [
        (
            ".github/actions/setup-actionlint/action.yml",
            "ACTIONLINT_SHA256",
            "rhysd/actionlint/releases/download",
            r#"tar xzf "$tarball""#,
            vec![(".github/workflows/ci.yml", 1)],
        ),
        (
            ".github/actions/setup-jscpd/action.yml",
            "JSCPD_SHA256",
            "kucherenko/jscpd/releases/download",
            r#"tar xzf "$tarball""#,
            vec![(".github/workflows/ci.yml", 1)],
        ),
        (
            ".github/actions/setup-lychee/action.yml",
            "LYCHEE_SHA256",
            "lycheeverse/lychee/releases/download",
            r#"tar xzf "$tarball""#,
            vec![
                (".github/workflows/ci.yml", 1),
                (".github/workflows/links-weekly.yml", 1),
            ],
        ),
    ] {
        let uses = format!(
            "uses: ./{}",
            action.strip_suffix("/action.yml").expect("an action file")
        );
        let sites: Vec<(String, usize)> = workflows()
            .into_iter()
            .map(|file| {
                let count = read(&file).matches(&uses).count();
                (file, count)
            })
            .filter(|(_, count)| *count > 0)
            .collect();
        let expected: Vec<(String, usize)> = expected
            .into_iter()
            .map(|(file, count)| (file.to_string(), count))
            .collect();
        assert_eq!(sites, expected, "{action}");
        assert_fetched_by_digest(action, digest, download, unpack);
    }
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
            "lint-non-rust",
            "ratchets",
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

/// The non-Rust half of the tree is linted by one job, each check at the
/// threshold issue #339 set: dropping a step, or softening its flags,
/// fails here.
#[test]
fn the_non_rust_lints_job_runs_every_check() {
    let jobs = ci_jobs();
    let (_, job) = jobs
        .iter()
        .find(|(id, _)| id == "lint-non-rust")
        .expect("a lint-non-rust job");
    for step in [
        "uses: ./.github/actions/setup-actionlint\n",
        "uses: ./.github/actions/setup-lychee\n",
        "fallback: none\n",
        "SHELLCHECK_OPTS: -S warning\n        run: actionlint\n",
        "run: git ls-files -z .github/workflows .github/actions | xargs -0 zizmor --offline\n",
        "run: git ls-files -z '*.sh' | xargs -0 shellcheck -S warning\n",
        "run: typos --hidden\n",
        "run: bash scripts/shellcheck-actions.sh\n",
        "run: git ls-files -z '*.md' | xargs -0 lychee --offline --include-fragments --no-progress\n",
        "npm ci --prefix .github/lint --ignore-scripts --no-audit --no-fund\n",
        "bash scripts/lint-diagrams.sh\n",
        "--entrypoint renovate-config-validator \"$image\" --strict \"$@\"\n",
        "validate --no-global .github/renovate.json5\n",
        "validate .github/renovate-global.json5\n",
    ] {
        assert!(job.contains(step), "lint-non-rust does not run {step:?}");
    }
    assert!(!job.contains("continue-on-error"), "lint-non-rust");
    // The versions have their homes (the tool: line, the lockfile) and
    // Renovate moves them there; this holds the shape, so a bump never
    // reddens it. The pin scan holds each tool: entry to an exact release.
    let tools: Vec<&str> = job
        .lines()
        .find_map(|line| line.trim().strip_prefix("tool: "))
        .expect("an install-action tool: line")
        .split(',')
        .map(|entry| entry.split_once('@').map_or(entry, |(name, _)| name))
        .collect();
    assert_eq!(tools, ["shellcheck", "typos", "zizmor"]);
    let lock = read(".github/lint/package-lock.json");
    let mermaid = lock
        .split_once(
            r#""node_modules/@mermaid-js/mermaid-cli": {
      "version": ""#,
        )
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(version, _)| version)
        .expect("the lockfile pins mermaid-cli");
    assert!(is_release(mermaid), "mermaid-cli {mermaid} is not exact");
    // Renovate runs from its pinned image, never from this install.
    assert!(
        !lock.contains("node_modules/renovate"),
        "Renovate is back in .github/lint"
    );
}
