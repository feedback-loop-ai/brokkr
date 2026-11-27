//! SHA-256 digest pin guard over `.github/actions/` and `.github/workflows/`
//! (#1417, operator ruling 2026-11-27).
//!
//! Every file under `.github/actions/` and `.github/workflows/` is pinned by
//! byte digest in `.github/workflow-digests.txt`. Every path component under
//! those two directories must match `[A-Za-z0-9._-]+` and not be `.` or `..`;
//! any other name is refused with its bytes escaped before anything is hashed
//! or written. Any edit to a pinned file, any missing or extra entry, any
//! symlink or non-regular entry, any format deviation, or any local `uses: ./…`
//! resolving outside the two directories fails this guard with a typed
//! [`DigestProblem`] (decision 0071 rulings 3, 8 and 9).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use yaml_rust2::{Yaml, YamlLoader};

use crate::json5::{parse_json5, Json5Error, Json5Value};

pub(crate) const DIGEST_FILE: &str = ".github/workflow-digests.txt";
const PINNED_DIRS: [&str; 2] = [".github/actions", ".github/workflows"];
const REFRESH_COMMAND: &str = "bash scripts/refresh-workflow-digests.sh";
const CHECKSUM_REFRESH_COMMAND: &str = "bash scripts/refresh-pin-checksums.sh";
const DIGEST_PINNED_GROUP: &str = "CI tools pinned by digest";
const DIGEST_PINNED_DEPS: [&str; 5] =
    ["bubblewrap", "cargo-audit", "actionlint", "lychee", "jscpd"];
const ALLOWED_GLOBAL_COMMANDS: [&str; 2] = [
    r"^bash scripts/refresh-pin-checksums\.sh$",
    r"^bash scripts/refresh-workflow-digests\.sh$",
];

/// Typed failure returned by [`check_workflow_digests`] (decision 0071 ruling 8).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum DigestProblem {
    #[error("{path}: digest file could not be read: {detail}")]
    UnreadableDigestFile { path: String, detail: String },

    #[error("{path}: digest file is empty; run `{REFRESH_COMMAND}`")]
    EmptyDigestFile { path: String },

    #[error("{path}: digest file is missing its final newline; run `{REFRESH_COMMAND}`")]
    MissingFinalNewline { path: String },

    #[error("{path}:{line}: blank line in digest file")]
    BlankLine { path: String, line: usize },

    #[error("{path}:{line}: comment line in digest file")]
    CommentLine { path: String, line: usize },

    #[error("{path}:{line}: digest must be 64 lowercase hex digits, found `{found}`")]
    UppercaseHex {
        path: String,
        line: usize,
        found: String,
    },

    #[error("{path}:{line}: expected two spaces between digest and path")]
    WrongSeparator { path: String, line: usize },

    #[error("{path}:{line}: malformed digest line `{content}`")]
    MalformedLine {
        path: String,
        line: usize,
        content: String,
    },

    #[error(
        "{path}:{line}: entry `{entry}` lies outside `.github/actions/` and `.github/workflows/`"
    )]
    PathOutsidePinnedDirs {
        path: String,
        line: usize,
        entry: String,
    },

    #[error("{path}: path component must match `[A-Za-z0-9._-]+` and not be `.` or `..`")]
    InvalidPathName { path: String },

    #[error("{path}:{line}: duplicate entry `{entry}`")]
    DuplicateEntry {
        path: String,
        line: usize,
        entry: String,
    },

    #[error("{path}:{line}: entry `{entry}` is out of byte order after `{previous}`")]
    OutOfOrder {
        path: String,
        line: usize,
        entry: String,
        previous: String,
    },

    #[error("{path}: pinned directory is missing or unreadable: {detail}")]
    UnreadablePinnedDir { path: String, detail: String },

    #[error("{path}: entry is neither a regular file nor a directory (symlinks are refused)")]
    NonRegularEntry { path: String },

    #[error(
        "{path}: file under pinned directory has no entry in `{DIGEST_FILE}`; run `{REFRESH_COMMAND}`"
    )]
    UnpinnedFile { path: String },

    #[error("{path}: recorded in `{DIGEST_FILE}` but does not exist; run `{REFRESH_COMMAND}`")]
    MissingPinnedFile { path: String },

    #[error(
        "{path}: digest mismatch (recorded {recorded}, actual {actual}); run `{REFRESH_COMMAND}`"
    )]
    DigestMismatch {
        path: String,
        recorded: String,
        actual: String,
    },

    #[error("{path}: unreadable YAML ({detail}); cannot verify local `uses:` references")]
    UnreadableYaml { path: String, detail: String },

    #[error(
        "{path}: local `uses: {uses}` resolves to `{resolved}`, outside `.github/actions/` and `.github/workflows/`"
    )]
    LocalUsesOutsidePinnedDirs {
        path: String,
        uses: String,
        resolved: String,
    },
}

/// Typed failure returned by [`check_renovate_digest_wiring`] (decision 0071 ruling 8).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RenovateConfigProblem {
    #[error("{file}: invalid JSON5: {err}")]
    InvalidJson5 { file: &'static str, err: Json5Error },

    #[error("{file}: expected root JSON5 object")]
    RootNotObject { file: &'static str },

    #[error(".github/renovate.json5: missing top-level `postUpgradeTasks`")]
    MissingTopLevelPostUpgradeTasks,

    #[error(".github/renovate.json5: top-level `postUpgradeTasks` does not match expected digest refresh task")]
    WrongTopLevelPostUpgradeTasks,

    #[error(".github/renovate.json5: `packageRules` is missing or not an array of objects")]
    InvalidPackageRules,

    #[error(".github/renovate.json5: expected exactly one package rule with `postUpgradeTasks`, found {count}")]
    WrongOverrideRuleCount { count: usize },

    #[error(".github/renovate.json5: `postUpgradeTasks` override rule is not `{DIGEST_PINNED_GROUP}` with expected dependencies")]
    WrongOverrideRuleTarget,

    #[error(".github/renovate.json5: `{DIGEST_PINNED_GROUP}` rule `postUpgradeTasks` must run checksum refresh then digest refresh with both fileFilters in branch mode")]
    WrongOverridePostUpgradeTasks,

    #[error(".github/renovate.json5: `jscpd` is matched by a later package rule after `{DIGEST_PINNED_GROUP}`")]
    JscpdOverriddenAfterDigestGroup,

    #[error(".github/renovate-global.json5: `allowedCommands` must allow exactly the two anchored refresh commands")]
    WrongGlobalAllowedCommands,
}

fn workspace() -> PathBuf {
    crate::workspace_root::workspace()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn is_allowed_component_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-')
}

fn is_valid_pinned_component(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes != b"."
        && bytes != b".."
        && bytes.iter().copied().all(is_allowed_component_byte)
}

fn escape_component_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        if is_allowed_component_byte(b) {
            out.push(b as char);
        } else {
            let _ = write!(out, "\\x{b:02x}");
        }
    }
    out
}

fn escape_rel_path_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        if is_allowed_component_byte(b) || b == b'/' {
            out.push(b as char);
        } else {
            let _ = write!(out, "\\x{b:02x}");
        }
    }
    out
}

enum PinnedEntryPathCheck {
    Valid,
    OutsidePinnedDirs,
    InvalidComponent(String),
}

fn check_pinned_rel_path(entry: &str) -> PinnedEntryPathCheck {
    if entry.contains(['\\', '\r', '\0']) {
        return PinnedEntryPathCheck::OutsidePinnedDirs;
    }
    let segments: Vec<&str> = entry.split('/').collect();
    if segments.len() < 3
        || segments
            .iter()
            .any(|s| s.is_empty() || *s == "." || *s == "..")
        || !matches!(
            (segments[0], segments[1]),
            (".github", "actions") | (".github", "workflows")
        )
    {
        return PinnedEntryPathCheck::OutsidePinnedDirs;
    }
    if segments[2..]
        .iter()
        .all(|s| is_valid_pinned_component(s.as_bytes()))
    {
        PinnedEntryPathCheck::Valid
    } else {
        PinnedEntryPathCheck::InvalidComponent(escape_rel_path_bytes(entry.as_bytes()))
    }
}

fn parse_digest_line(line_no: usize, line: &str) -> Result<(String, String), DigestProblem> {
    if line.trim().is_empty() {
        return Err(DigestProblem::BlankLine {
            path: DIGEST_FILE.to_string(),
            line: line_no,
        });
    }
    if line.trim_start().starts_with('#') {
        return Err(DigestProblem::CommentLine {
            path: DIGEST_FILE.to_string(),
            line: line_no,
        });
    }
    if line.contains('\r') {
        return Err(DigestProblem::MalformedLine {
            path: DIGEST_FILE.to_string(),
            line: line_no,
            content: line.to_string(),
        });
    }
    let Some((hex, rest)) = line.split_once(' ') else {
        return Err(DigestProblem::MalformedLine {
            path: DIGEST_FILE.to_string(),
            line: line_no,
            content: line.to_string(),
        });
    };
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(DigestProblem::MalformedLine {
            path: DIGEST_FILE.to_string(),
            line: line_no,
            content: line.to_string(),
        });
    }
    if hex.bytes().any(|b| b.is_ascii_uppercase()) {
        return Err(DigestProblem::UppercaseHex {
            path: DIGEST_FILE.to_string(),
            line: line_no,
            found: hex.to_string(),
        });
    }
    let Some(entry) = rest.strip_prefix(' ') else {
        return Err(DigestProblem::WrongSeparator {
            path: DIGEST_FILE.to_string(),
            line: line_no,
        });
    };
    if entry.is_empty() || entry.starts_with([' ', '\t']) {
        return Err(DigestProblem::WrongSeparator {
            path: DIGEST_FILE.to_string(),
            line: line_no,
        });
    }
    match check_pinned_rel_path(entry) {
        PinnedEntryPathCheck::Valid => Ok((hex.to_string(), entry.to_string())),
        PinnedEntryPathCheck::OutsidePinnedDirs => Err(DigestProblem::PathOutsidePinnedDirs {
            path: DIGEST_FILE.to_string(),
            line: line_no,
            entry: entry.to_string(),
        }),
        PinnedEntryPathCheck::InvalidComponent(escaped) => {
            Err(DigestProblem::InvalidPathName { path: escaped })
        }
    }
}

fn require_non_symlink_kind(
    path: &Path,
    label: &str,
    expect_dir: bool,
    problems: &mut Vec<DigestProblem>,
) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            let ft = meta.file_type();
            let ok = if expect_dir {
                ft.is_dir()
            } else {
                ft.is_file()
            };
            if !ok {
                problems.push(DigestProblem::NonRegularEntry {
                    path: label.to_string(),
                });
            }
            ok
        }
        Err(err) => {
            problems.push(DigestProblem::UnreadableDigestFile {
                path: DIGEST_FILE.to_string(),
                detail: err.to_string(),
            });
            false
        }
    }
}

fn read_digest_file(
    root: &Path,
    problems: &mut Vec<DigestProblem>,
) -> Option<BTreeMap<String, String>> {
    let digest_path = root.join(DIGEST_FILE);
    if !require_non_symlink_kind(&digest_path, DIGEST_FILE, false, problems) {
        return None;
    }
    let bytes = match std::fs::read(&digest_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            problems.push(DigestProblem::UnreadableDigestFile {
                path: DIGEST_FILE.to_string(),
                detail: err.to_string(),
            });
            return None;
        }
    };
    if bytes.is_empty() {
        problems.push(DigestProblem::EmptyDigestFile {
            path: DIGEST_FILE.to_string(),
        });
        return None;
    }
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(err) => {
            problems.push(DigestProblem::UnreadableDigestFile {
                path: DIGEST_FILE.to_string(),
                detail: err.to_string(),
            });
            return None;
        }
    };
    parse_digest_contents(&text, problems)
}

fn parse_digest_contents(
    text: &str,
    problems: &mut Vec<DigestProblem>,
) -> Option<BTreeMap<String, String>> {
    let before = problems.len();
    let body = match text.strip_suffix('\n') {
        Some(stripped) => stripped,
        None => {
            problems.push(DigestProblem::MissingFinalNewline {
                path: DIGEST_FILE.to_string(),
            });
            text
        }
    };
    let mut recorded = BTreeMap::new();
    let mut prev: Option<String> = None;
    for (idx, line) in body.split('\n').enumerate() {
        let line_no = idx + 1;
        match parse_digest_line(line_no, line) {
            Ok((digest, entry)) => {
                if recorded.contains_key(&entry) {
                    problems.push(DigestProblem::DuplicateEntry {
                        path: DIGEST_FILE.to_string(),
                        line: line_no,
                        entry,
                    });
                    continue;
                }
                if let Some(previous) = prev.as_deref() {
                    if entry.as_bytes() < previous.as_bytes() {
                        problems.push(DigestProblem::OutOfOrder {
                            path: DIGEST_FILE.to_string(),
                            line: line_no,
                            entry: entry.clone(),
                            previous: previous.to_string(),
                        });
                    }
                }
                prev = Some(entry.clone());
                recorded.insert(entry, digest);
            }
            Err(problem) => problems.push(problem),
        }
    }
    (problems.len() == before).then_some(recorded)
}

fn walk_pinned_dir(
    root: &Path,
    rel_dir: &str,
    files: &mut BTreeSet<String>,
    non_regular: &mut BTreeSet<String>,
    problems: &mut Vec<DigestProblem>,
) {
    let dir_path = root.join(rel_dir);
    let mut entries: Vec<_> = match std::fs::read_dir(&dir_path) {
        Ok(iter) => match iter.collect::<Result<Vec<_>, _>>() {
            Ok(list) => list,
            Err(err) => {
                problems.push(DigestProblem::UnreadablePinnedDir {
                    path: rel_dir.to_string(),
                    detail: err.to_string(),
                });
                return;
            }
        },
        Err(err) => {
            problems.push(DigestProblem::UnreadablePinnedDir {
                path: rel_dir.to_string(),
                detail: err.to_string(),
            });
            return;
        }
    };
    entries.sort_by(|a, b| a.file_name().as_bytes().cmp(b.file_name().as_bytes()));
    for entry in entries {
        let os_name = entry.file_name();
        let name_bytes = os_name.as_bytes();
        if !is_valid_pinned_component(name_bytes) {
            problems.push(DigestProblem::InvalidPathName {
                path: format!("{rel_dir}/{}", escape_component_bytes(name_bytes)),
            });
            continue;
        }
        let name =
            std::str::from_utf8(name_bytes).expect("validated ASCII path component is valid UTF-8");
        let rel_path = format!("{rel_dir}/{name}");
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(err) => {
                problems.push(DigestProblem::UnreadablePinnedDir {
                    path: rel_path,
                    detail: err.to_string(),
                });
                continue;
            }
        };
        if ft.is_dir() {
            walk_pinned_dir(root, &rel_path, files, non_regular, problems);
        } else if ft.is_file() {
            files.insert(rel_path);
        } else {
            non_regular.insert(rel_path.clone());
            problems.push(DigestProblem::NonRegularEntry { path: rel_path });
        }
    }
}

fn collect_pinned_tree(
    root: &Path,
    problems: &mut Vec<DigestProblem>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut files = BTreeSet::new();
    let mut non_regular = BTreeSet::new();
    for dir in PINNED_DIRS {
        match std::fs::symlink_metadata(root.join(dir)) {
            Ok(meta) if meta.file_type().is_dir() => {
                walk_pinned_dir(root, dir, &mut files, &mut non_regular, problems);
            }
            Ok(_) => {
                non_regular.insert(dir.to_string());
                problems.push(DigestProblem::NonRegularEntry {
                    path: dir.to_string(),
                });
            }
            Err(err) => {
                problems.push(DigestProblem::UnreadablePinnedDir {
                    path: dir.to_string(),
                    detail: err.to_string(),
                });
            }
        }
    }
    (files, non_regular)
}

fn resolve_local_uses(uses: &str) -> Result<Vec<&str>, ()> {
    if uses.contains(['\\', '\0', '\n', '\r']) {
        return Err(());
    }
    let rest = uses.strip_prefix("./").ok_or(())?;
    let mut stack = Vec::new();
    for seg in rest.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                stack.pop().ok_or(())?;
            }
            other => stack.push(other),
        }
    }
    Ok(stack)
}

fn check_yaml_map_entry(file: &str, k: &Yaml, v: &Yaml, problems: &mut Vec<DigestProblem>) {
    if k.as_str() == Some("<<") {
        problems.push(DigestProblem::UnreadableYaml {
            path: file.to_string(),
            detail: "YAML merge key `<<` is refused".to_string(),
        });
        return;
    }
    if k.as_str() == Some("uses") {
        if let Some(uses) = v.as_str().filter(|s| s.starts_with("./")) {
            validate_local_uses_target(file, uses, problems);
        }
    }
    check_yaml_local_uses(file, k, problems);
    check_yaml_local_uses(file, v, problems);
}

fn check_yaml_local_uses(file: &str, node: &Yaml, problems: &mut Vec<DigestProblem>) {
    match node {
        Yaml::Alias(_) | Yaml::BadValue => {
            problems.push(DigestProblem::UnreadableYaml {
                path: file.to_string(),
                detail: "YAML alias or unreadable value".to_string(),
            });
        }
        Yaml::Array(items) => {
            for item in items {
                check_yaml_local_uses(file, item, problems);
            }
        }
        Yaml::Hash(map) => {
            for (k, v) in map {
                check_yaml_map_entry(file, k, v, problems);
            }
        }
        _ => {}
    }
}

fn validate_local_uses_target(file: &str, uses: &str, problems: &mut Vec<DigestProblem>) {
    match resolve_local_uses(uses) {
        Ok(segments)
            if matches!(
                segments.as_slice(),
                [".github", "actions", ..] | [".github", "workflows", ..]
            ) => {}
        Ok(segments) => {
            problems.push(DigestProblem::LocalUsesOutsidePinnedDirs {
                path: file.to_string(),
                uses: uses.to_string(),
                resolved: segments.join("/"),
            });
        }
        Err(()) => {
            problems.push(DigestProblem::LocalUsesOutsidePinnedDirs {
                path: file.to_string(),
                uses: uses.to_string(),
                resolved: "<escapes repository root>".to_string(),
            });
        }
    }
}

fn is_yaml_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("yml") || ext.eq_ignore_ascii_case("yaml"))
}

fn verify_yaml_uses_in_bytes(file: &str, bytes: &[u8], problems: &mut Vec<DigestProblem>) {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(err) => {
            problems.push(DigestProblem::UnreadableYaml {
                path: file.to_string(),
                detail: err.to_string(),
            });
            return;
        }
    };
    let docs = match YamlLoader::load_from_str(text) {
        Ok(docs) => docs,
        Err(err) => {
            problems.push(DigestProblem::UnreadableYaml {
                path: file.to_string(),
                detail: err.to_string(),
            });
            return;
        }
    };
    for doc in &docs {
        check_yaml_local_uses(file, doc, problems);
    }
}

fn verify_pinned_file_contents(
    root: &Path,
    path: &str,
    expected: Option<&String>,
    problems: &mut Vec<DigestProblem>,
) {
    let bytes = match std::fs::read(root.join(path)) {
        Ok(bytes) => bytes,
        Err(err) => {
            problems.push(DigestProblem::UnreadablePinnedDir {
                path: path.to_string(),
                detail: err.to_string(),
            });
            return;
        }
    };
    if let Some(recorded) = expected {
        let actual = sha256_hex(&bytes);
        if actual != *recorded {
            problems.push(DigestProblem::DigestMismatch {
                path: path.to_string(),
                recorded: recorded.clone(),
                actual,
            });
        }
    }
    if is_yaml_path(path) {
        verify_yaml_uses_in_bytes(path, &bytes, problems);
    }
}

/// Check `.github/workflow-digests.txt` against `.github/actions/` and
/// `.github/workflows/` under `root`.
pub(crate) fn check_workflow_digests(root: &Path) -> Vec<DigestProblem> {
    let mut problems = Vec::new();
    if !require_non_symlink_kind(&root.join(".github"), ".github", true, &mut problems) {
        return problems;
    }
    let mut tree_problems = Vec::new();
    let (found_files, non_regular) = collect_pinned_tree(root, &mut tree_problems);
    let invalid_names: Vec<DigestProblem> = tree_problems
        .iter()
        .filter(|p| matches!(p, DigestProblem::InvalidPathName { .. }))
        .cloned()
        .collect();
    if !invalid_names.is_empty() {
        return invalid_names;
    }
    let Some(recorded) = read_digest_file(root, &mut problems) else {
        return problems;
    };
    problems.extend(tree_problems);
    for path in &found_files {
        if !recorded.contains_key(path) {
            problems.push(DigestProblem::UnpinnedFile { path: path.clone() });
        }
    }
    for path in recorded.keys() {
        let covered_non_regular = non_regular.contains(path)
            || non_regular
                .iter()
                .any(|dir| path.starts_with(&format!("{dir}/")));
        if !found_files.contains(path) && !covered_non_regular {
            problems.push(DigestProblem::MissingPinnedFile { path: path.clone() });
        }
    }
    for path in &found_files {
        verify_pinned_file_contents(root, path, recorded.get(path), &mut problems);
    }
    problems
}

fn post_upgrade_tasks_matches(
    value: &Json5Value,
    expected_commands: &[&str],
    expected_filters: &[&str],
) -> bool {
    let Some(obj) = value.as_object() else {
        return false;
    };
    if obj.len() != 3 {
        return false;
    }
    obj.get("commands")
        .and_then(Json5Value::as_string_list)
        .is_some_and(|cmds| cmds == expected_commands)
        && obj
            .get("fileFilters")
            .and_then(Json5Value::as_string_list)
            .is_some_and(|filters| filters == expected_filters)
        && obj.get("executionMode").and_then(Json5Value::as_str) == Some("branch")
}

fn check_renovate_package_rules(
    root: &BTreeMap<String, Json5Value>,
    problems: &mut Vec<RenovateConfigProblem>,
) {
    let Some(rules_slice) = root.get("packageRules").and_then(Json5Value::as_array) else {
        problems.push(RenovateConfigProblem::InvalidPackageRules);
        return;
    };
    let mut rule_objs = Vec::with_capacity(rules_slice.len());
    for item in rules_slice {
        let Some(obj) = item.as_object() else {
            problems.push(RenovateConfigProblem::InvalidPackageRules);
            return;
        };
        rule_objs.push(obj);
    }
    let override_indices: Vec<usize> = rule_objs
        .iter()
        .enumerate()
        .filter_map(|(idx, obj)| obj.contains_key("postUpgradeTasks").then_some(idx))
        .collect();
    if override_indices.len() != 1 {
        problems.push(RenovateConfigProblem::WrongOverrideRuleCount {
            count: override_indices.len(),
        });
        return;
    }
    let idx = override_indices[0];
    let rule = rule_objs[idx];
    let right_target = rule.get("groupName").and_then(Json5Value::as_str)
        == Some(DIGEST_PINNED_GROUP)
        && rule
            .get("matchDepNames")
            .and_then(Json5Value::as_string_list)
            .is_some_and(|deps| deps == DIGEST_PINNED_DEPS);
    if !right_target {
        problems.push(RenovateConfigProblem::WrongOverrideRuleTarget);
    }
    let tasks = rule
        .get("postUpgradeTasks")
        .expect("filtered by contains_key");
    if !post_upgrade_tasks_matches(
        tasks,
        &[CHECKSUM_REFRESH_COMMAND, REFRESH_COMMAND],
        &[".github/actions/*/action.yml", DIGEST_FILE],
    ) {
        problems.push(RenovateConfigProblem::WrongOverridePostUpgradeTasks);
    }
    let later_matches_jscpd = rule_objs[idx + 1..].iter().any(|later| {
        later
            .get("matchDepNames")
            .and_then(Json5Value::as_string_list)
            .is_some_and(|deps| deps.contains(&"jscpd"))
    });
    if later_matches_jscpd {
        problems.push(RenovateConfigProblem::JscpdOverriddenAfterDigestGroup);
    }
}

/// Structurally verify `.github/renovate.json5` and `.github/renovate-global.json5`
/// wire workflow digest refreshes across all updates and the digest-pinned override.
pub(crate) fn check_renovate_digest_wiring(
    config_text: &str,
    global_text: &str,
) -> Vec<RenovateConfigProblem> {
    let mut problems = Vec::new();
    match parse_json5(config_text) {
        Err(err) => problems.push(RenovateConfigProblem::InvalidJson5 {
            file: ".github/renovate.json5",
            err,
        }),
        Ok(val) => match val.as_object() {
            None => problems.push(RenovateConfigProblem::RootNotObject {
                file: ".github/renovate.json5",
            }),
            Some(root) => {
                match root.get("postUpgradeTasks") {
                    None => problems.push(RenovateConfigProblem::MissingTopLevelPostUpgradeTasks),
                    Some(tasks) => {
                        if !post_upgrade_tasks_matches(tasks, &[REFRESH_COMMAND], &[DIGEST_FILE]) {
                            problems.push(RenovateConfigProblem::WrongTopLevelPostUpgradeTasks);
                        }
                    }
                }
                check_renovate_package_rules(root, &mut problems);
            }
        },
    }
    match parse_json5(global_text) {
        Err(err) => problems.push(RenovateConfigProblem::InvalidJson5 {
            file: ".github/renovate-global.json5",
            err,
        }),
        Ok(val) => match val.as_object() {
            None => problems.push(RenovateConfigProblem::RootNotObject {
                file: ".github/renovate-global.json5",
            }),
            Some(root) => {
                let ok = root
                    .get("allowedCommands")
                    .and_then(Json5Value::as_string_list)
                    .is_some_and(|cmds| cmds == ALLOWED_GLOBAL_COMMANDS);
                if !ok {
                    problems.push(RenovateConfigProblem::WrongGlobalAllowedCommands);
                }
            }
        },
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    struct PlantedTree {
        dir: tempfile::TempDir,
    }

    impl PlantedTree {
        fn new() -> Self {
            let dir = tempfile::tempdir().expect("tempdir");
            std::fs::create_dir_all(dir.path().join(".github/actions")).expect("actions dir");
            std::fs::create_dir_all(dir.path().join(".github/workflows")).expect("workflows dir");
            Self { dir }
        }

        fn path(&self) -> &Path {
            self.dir.path()
        }

        fn put(&self, rel: &str, content: &str) {
            let full = self.path().join(rel);
            std::fs::create_dir_all(full.parent().expect("parent")).expect("mkdir");
            std::fs::write(full, content).expect("write");
        }

        fn write_valid_digests(&self, entries: &[(&str, &str)]) {
            let mut sorted: Vec<(&str, &str)> = entries.to_vec();
            sorted.sort_by(|(a, _), (b, _)| a.as_bytes().cmp(b.as_bytes()));
            let mut out = String::new();
            for (rel, body) in sorted {
                self.put(rel, body);
                out.push_str(&format!("{}  {rel}\n", sha256_hex(body.as_bytes())));
            }
            self.put(DIGEST_FILE, &out);
        }
    }

    const SAMPLE_ACTION: &str = "name: sample\nruns:\n  using: composite\n  steps:\n    - run: echo ok\n      shell: bash\n";
    const SAMPLE_WF: &str = "name: sample\non: push\njobs:\n  ok:\n    runs-on: ubuntu-24.04\n    steps:\n      - uses: ./.github/actions/sample\n";

    #[test]
    fn workflow_digests_hold_on_repository_tree() {
        let problems = check_workflow_digests(&workspace());
        assert_eq!(problems, Vec::<DigestProblem>::new());
    }

    #[test]
    fn digest_mismatch_reports_recorded_actual_and_refresh_command() {
        let tree = PlantedTree::new();
        tree.write_valid_digests(&[
            (".github/actions/sample/action.yml", SAMPLE_ACTION),
            (".github/workflows/sample.yml", SAMPLE_WF),
        ]);
        let expected = sha256_hex(SAMPLE_WF.as_bytes());
        let mutated = format!("{SAMPLE_WF}# edited\n");
        let actual = sha256_hex(mutated.as_bytes());
        tree.put(".github/workflows/sample.yml", &mutated);

        let problems = check_workflow_digests(tree.path());
        assert_eq!(
            problems,
            vec![DigestProblem::DigestMismatch {
                path: ".github/workflows/sample.yml".to_string(),
                recorded: expected.clone(),
                actual: actual.clone(),
            }]
        );
        // Pin Display text once (decision 0071 ruling 8): must name path,
        // recorded digest, actual digest, and the refresh command.
        let rendered = problems[0].to_string();
        assert_eq!(
            rendered,
            format!(
                ".github/workflows/sample.yml: digest mismatch (recorded {expected}, actual {actual}); run `{REFRESH_COMMAND}`"
            )
        );
    }

    #[test]
    fn unpinned_file_in_either_directory_is_refused() {
        let tree = PlantedTree::new();
        tree.write_valid_digests(&[(".github/workflows/sample.yml", SAMPLE_WF)]);
        tree.put(".github/actions/nested/deep/action.yml", SAMPLE_ACTION);
        assert_eq!(
            check_workflow_digests(tree.path()),
            vec![DigestProblem::UnpinnedFile {
                path: ".github/actions/nested/deep/action.yml".to_string(),
            }]
        );

        let wf_tree = PlantedTree::new();
        wf_tree.write_valid_digests(&[(".github/actions/sample/action.yml", SAMPLE_ACTION)]);
        wf_tree.put(".github/workflows/extra.yml", SAMPLE_WF);
        assert_eq!(
            check_workflow_digests(wf_tree.path()),
            vec![DigestProblem::UnpinnedFile {
                path: ".github/workflows/extra.yml".to_string(),
            }]
        );
    }

    #[test]
    fn missing_pinned_file_is_refused() {
        let tree = PlantedTree::new();
        tree.write_valid_digests(&[
            (".github/actions/sample/action.yml", SAMPLE_ACTION),
            (".github/workflows/sample.yml", SAMPLE_WF),
        ]);
        std::fs::remove_file(tree.path().join(".github/workflows/sample.yml")).expect("rm");
        assert_eq!(
            check_workflow_digests(tree.path()),
            vec![DigestProblem::MissingPinnedFile {
                path: ".github/workflows/sample.yml".to_string(),
            }]
        );
    }

    #[test]
    fn symlink_under_either_directory_is_refused_and_never_followed() {
        let tree = PlantedTree::new();
        tree.write_valid_digests(&[(".github/workflows/sample.yml", SAMPLE_WF)]);
        tree.put("outside.yml", SAMPLE_ACTION);
        symlink(
            tree.path().join("outside.yml"),
            tree.path().join(".github/actions/link.yml"),
        )
        .expect("symlink file");
        assert_eq!(
            check_workflow_digests(tree.path()),
            vec![DigestProblem::NonRegularEntry {
                path: ".github/actions/link.yml".to_string(),
            }]
        );

        let dir_tree = PlantedTree::new();
        dir_tree.write_valid_digests(&[(".github/workflows/sample.yml", SAMPLE_WF)]);
        dir_tree.put("external_dir/hidden.yml", SAMPLE_ACTION);
        symlink(
            dir_tree.path().join("external_dir"),
            dir_tree.path().join(".github/actions/linked-dir"),
        )
        .expect("symlink dir");
        // Must report only NonRegularEntry on the symlinked dir, never walk into it.
        assert_eq!(
            check_workflow_digests(dir_tree.path()),
            vec![DigestProblem::NonRegularEntry {
                path: ".github/actions/linked-dir".to_string(),
            }]
        );
    }

    #[test]
    fn local_uses_outside_pinned_dirs_is_refused_structurally() {
        let direct = "name: bad\non: push\njobs:\n  j:\n    runs-on: ubuntu-24.04\n    steps:\n      - uses: ./scripts/unpinned-action\n";
        let tree = PlantedTree::new();
        tree.write_valid_digests(&[(".github/workflows/bad.yml", direct)]);
        assert_eq!(
            check_workflow_digests(tree.path()),
            vec![DigestProblem::LocalUsesOutsidePinnedDirs {
                path: ".github/workflows/bad.yml".to_string(),
                uses: "./scripts/unpinned-action".to_string(),
                resolved: "scripts/unpinned-action".to_string(),
            }]
        );

        let traversal =
            "name: bad\non: push\njobs:\n  j:\n    uses: ./.github/workflows/../evil.yml\n";
        let t2 = PlantedTree::new();
        t2.write_valid_digests(&[(".github/workflows/bad.yml", traversal)]);
        assert_eq!(
            check_workflow_digests(t2.path()),
            vec![DigestProblem::LocalUsesOutsidePinnedDirs {
                path: ".github/workflows/bad.yml".to_string(),
                uses: "./.github/workflows/../evil.yml".to_string(),
                resolved: ".github/evil.yml".to_string(),
            }]
        );

        let sibling_prefix = "name: bad\non: push\njobs:\n  j:\n    runs-on: ubuntu-24.04\n    steps:\n      - uses: ./.github/actions-evil/run\n";
        let t3 = PlantedTree::new();
        t3.write_valid_digests(&[(".github/workflows/bad.yml", sibling_prefix)]);
        assert_eq!(
            check_workflow_digests(t3.path()),
            vec![DigestProblem::LocalUsesOutsidePinnedDirs {
                path: ".github/workflows/bad.yml".to_string(),
                uses: "./.github/actions-evil/run".to_string(),
                resolved: ".github/actions-evil/run".to_string(),
            }]
        );

        let escape_root = "name: bad\non: push\njobs:\n  j:\n    runs-on: ubuntu-24.04\n    steps:\n      - uses: ./../../etc/passwd\n";
        let t4 = PlantedTree::new();
        t4.write_valid_digests(&[(".github/workflows/bad.yml", escape_root)]);
        assert_eq!(
            check_workflow_digests(t4.path()),
            vec![DigestProblem::LocalUsesOutsidePinnedDirs {
                path: ".github/workflows/bad.yml".to_string(),
                uses: "./../../etc/passwd".to_string(),
                resolved: "<escapes repository root>".to_string(),
            }]
        );

        // Structural YAML check: comment and `run:` text containing `uses: ./outside` pass.
        let benign = "name: ok\n# uses: ./outside\non: push\njobs:\n  j:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: |\n          echo 'uses: ./outside'\n";
        let t5 = PlantedTree::new();
        t5.write_valid_digests(&[(".github/workflows/ok.yml", benign)]);
        assert_eq!(
            check_workflow_digests(t5.path()),
            Vec::<DigestProblem>::new()
        );
    }

    /// Plant a file whose name is raw OS bytes. Compiles on Linux and macOS;
    /// returns `false` (after printing a notice) if the host filesystem refuses
    /// non-UTF-8 filenames (e.g. macOS APFS).
    fn plant_raw_name_file(dir: &Path, name_bytes: &[u8], body: &str) -> bool {
        assert!(
            dir.is_dir(),
            "parent directory must exist: {}",
            dir.display()
        );
        let path = dir.join(std::ffi::OsStr::from_bytes(name_bytes));
        match std::fs::write(&path, body) {
            Ok(()) => true,
            Err(err) => {
                assert_ne!(
                    err.kind(),
                    std::io::ErrorKind::NotFound,
                    "unexpected missing parent while planting raw-byte filename"
                );
                eprintln!(
                    "host filesystem refused non-UTF-8 filename `{}`: {err}",
                    escape_component_bytes(name_bytes)
                );
                false
            }
        }
    }

    #[test]
    fn invalid_name_non_utf8_beside_recorded_twin_is_refused() {
        let tree = PlantedTree::new();
        let twin = ".github/actions/helper-\u{FFFD}.txt";
        tree.write_valid_digests(&[
            (twin, "recorded twin\n"),
            (".github/workflows/sample.yml", SAMPLE_WF),
        ]);
        if !plant_raw_name_file(
            &tree.path().join(".github/actions"),
            b"helper-\xff.txt",
            "unrecorded sibling\n",
        ) {
            return;
        }
        assert_eq!(
            check_workflow_digests(tree.path()),
            vec![
                DigestProblem::InvalidPathName {
                    path: r".github/actions/helper-\xef\xbf\xbd.txt".to_string(),
                },
                DigestProblem::InvalidPathName {
                    path: r".github/actions/helper-\xff.txt".to_string(),
                },
            ]
        );
    }

    #[test]
    fn invalid_name_lone_non_utf8_is_refused() {
        let tree = PlantedTree::new();
        tree.write_valid_digests(&[(".github/workflows/sample.yml", SAMPLE_WF)]);
        if !plant_raw_name_file(
            &tree.path().join(".github/actions"),
            b"bad-\xff.txt",
            "raw bytes\n",
        ) {
            return;
        }
        assert_eq!(
            check_workflow_digests(tree.path()),
            vec![DigestProblem::InvalidPathName {
                path: r".github/actions/bad-\xff.txt".to_string(),
            }]
        );
    }

    #[test]
    fn invalid_name_with_space_is_refused() {
        let tree = PlantedTree::new();
        tree.write_valid_digests(&[(".github/workflows/sample.yml", SAMPLE_WF)]);
        tree.put(".github/actions/bad name.yml", SAMPLE_ACTION);
        assert_eq!(
            check_workflow_digests(tree.path()),
            vec![DigestProblem::InvalidPathName {
                path: r".github/actions/bad\x20name.yml".to_string(),
            }]
        );
    }

    #[test]
    fn invalid_name_with_non_ascii_letter_is_refused() {
        let tree = PlantedTree::new();
        tree.write_valid_digests(&[(".github/workflows/sample.yml", SAMPLE_WF)]);
        tree.put(".github/workflows/caf\u{00e9}.yml", SAMPLE_WF);
        assert_eq!(
            check_workflow_digests(tree.path()),
            vec![DigestProblem::InvalidPathName {
                path: r".github/workflows/caf\xc3\xa9.yml".to_string(),
            }]
        );
    }

    fn check_raw_digest_file(raw: &str) -> Vec<DigestProblem> {
        let tree = PlantedTree::new();
        tree.put(".github/actions/sample/action.yml", SAMPLE_ACTION);
        tree.put(".github/workflows/sample.yml", SAMPLE_WF);
        tree.put(DIGEST_FILE, raw);
        check_workflow_digests(tree.path())
    }

    #[test]
    fn digest_file_refuses_empty_file() {
        assert_eq!(
            check_raw_digest_file(""),
            vec![DigestProblem::EmptyDigestFile {
                path: DIGEST_FILE.to_string(),
            }]
        );
    }

    #[test]
    fn digest_file_refuses_missing_final_newline() {
        let digest = sha256_hex(SAMPLE_WF.as_bytes());
        let a_digest = sha256_hex(SAMPLE_ACTION.as_bytes());
        assert_eq!(
            check_raw_digest_file(&format!(
                "{a_digest}  .github/actions/sample/action.yml\n{digest}  .github/workflows/sample.yml"
            )),
            vec![DigestProblem::MissingFinalNewline {
                path: DIGEST_FILE.to_string(),
            }]
        );
    }

    #[test]
    fn digest_file_refuses_blank_line() {
        let digest = sha256_hex(SAMPLE_WF.as_bytes());
        let a_digest = sha256_hex(SAMPLE_ACTION.as_bytes());
        assert_eq!(
            check_raw_digest_file(&format!(
                "{a_digest}  .github/actions/sample/action.yml\n\n{digest}  .github/workflows/sample.yml\n"
            )),
            vec![DigestProblem::BlankLine {
                path: DIGEST_FILE.to_string(),
                line: 2,
            }]
        );
    }

    #[test]
    fn digest_file_refuses_comment_line() {
        let digest = sha256_hex(SAMPLE_WF.as_bytes());
        let a_digest = sha256_hex(SAMPLE_ACTION.as_bytes());
        assert_eq!(
            check_raw_digest_file(&format!(
                "# generated\n{a_digest}  .github/actions/sample/action.yml\n{digest}  .github/workflows/sample.yml\n"
            )),
            vec![DigestProblem::CommentLine {
                path: DIGEST_FILE.to_string(),
                line: 1,
            }]
        );
    }

    #[test]
    fn digest_file_refuses_uppercase_hex() {
        let upper = sha256_hex(SAMPLE_WF.as_bytes()).to_ascii_uppercase();
        assert_eq!(
            check_raw_digest_file(&format!("{upper}  .github/workflows/sample.yml\n")),
            vec![DigestProblem::UppercaseHex {
                path: DIGEST_FILE.to_string(),
                line: 1,
                found: upper,
            }]
        );
    }

    #[test]
    fn digest_file_refuses_wrong_separator() {
        let digest = sha256_hex(SAMPLE_WF.as_bytes());
        assert_eq!(
            check_raw_digest_file(&format!("{digest} .github/workflows/sample.yml\n")),
            vec![DigestProblem::WrongSeparator {
                path: DIGEST_FILE.to_string(),
                line: 1,
            }]
        );
    }

    #[test]
    fn digest_file_refuses_malformed_line() {
        assert_eq!(
            check_raw_digest_file("not-a-valid-line\n"),
            vec![DigestProblem::MalformedLine {
                path: DIGEST_FILE.to_string(),
                line: 1,
                content: "not-a-valid-line".to_string(),
            }]
        );
    }

    #[test]
    fn digest_file_refuses_path_outside_pinned_dirs() {
        let digest = sha256_hex(SAMPLE_WF.as_bytes());
        assert_eq!(
            check_raw_digest_file(&format!("{digest}  .github/renovate.json5\n")),
            vec![DigestProblem::PathOutsidePinnedDirs {
                path: DIGEST_FILE.to_string(),
                line: 1,
                entry: ".github/renovate.json5".to_string(),
            }]
        );
        assert_eq!(
            check_raw_digest_file(&format!("{digest}  .github/workflows/../renovate.json5\n")),
            vec![DigestProblem::PathOutsidePinnedDirs {
                path: DIGEST_FILE.to_string(),
                line: 1,
                entry: ".github/workflows/../renovate.json5".to_string(),
            }]
        );
    }

    #[test]
    fn digest_file_refuses_invalid_path_name() {
        let digest = sha256_hex(SAMPLE_WF.as_bytes());
        assert_eq!(
            check_raw_digest_file(&format!("{digest}  .github/actions/bad name.yml\n")),
            vec![DigestProblem::InvalidPathName {
                path: r".github/actions/bad\x20name.yml".to_string(),
            }]
        );
    }

    #[test]
    fn digest_file_refuses_duplicate_entry() {
        let digest = sha256_hex(SAMPLE_WF.as_bytes());
        assert_eq!(
            check_raw_digest_file(&format!(
                "{digest}  .github/workflows/sample.yml\n{digest}  .github/workflows/sample.yml\n"
            )),
            vec![DigestProblem::DuplicateEntry {
                path: DIGEST_FILE.to_string(),
                line: 2,
                entry: ".github/workflows/sample.yml".to_string(),
            }]
        );
    }

    #[test]
    fn digest_file_refuses_out_of_order() {
        let digest = sha256_hex(SAMPLE_WF.as_bytes());
        let a_digest = sha256_hex(SAMPLE_ACTION.as_bytes());
        assert_eq!(
            check_raw_digest_file(&format!(
                "{digest}  .github/workflows/sample.yml\n{a_digest}  .github/actions/sample/action.yml\n"
            )),
            vec![DigestProblem::OutOfOrder {
                path: DIGEST_FILE.to_string(),
                line: 2,
                entry: ".github/actions/sample/action.yml".to_string(),
                previous: ".github/workflows/sample.yml".to_string(),
            }]
        );
    }

    fn read_renovate_configs() -> (String, String) {
        let root = workspace();
        let config = std::fs::read_to_string(root.join(".github/renovate.json5")).expect("config");
        let global =
            std::fs::read_to_string(root.join(".github/renovate-global.json5")).expect("global");
        (config, global)
    }

    #[test]
    fn renovate_config_allows_and_runs_workflow_digest_refresh() {
        let (config, global) = read_renovate_configs();
        assert_eq!(
            check_renovate_digest_wiring(&config, &global),
            Vec::<RenovateConfigProblem>::new()
        );
    }

    #[test]
    fn renovate_wiring_guard_refuses_moved_dropped_swapped_and_unanchored_configs() {
        let (config, global) = read_renovate_configs();

        let top_block = "  postUpgradeTasks: {\n    commands: ['bash scripts/refresh-workflow-digests.sh'],\n    fileFilters: ['.github/workflow-digests.txt'],\n    executionMode: 'branch',\n  },\n";
        assert!(config.contains(top_block));
        let moved = config.replacen(top_block, "", 1).replacen(
            "      groupName: 'actions',\n",
            "      groupName: 'actions',\n      postUpgradeTasks: {\n        commands: ['bash scripts/refresh-workflow-digests.sh'],\n        fileFilters: ['.github/workflow-digests.txt'],\n        executionMode: 'branch',\n      },\n",
            1,
        );
        assert_eq!(
            check_renovate_digest_wiring(&moved, &global),
            vec![
                RenovateConfigProblem::MissingTopLevelPostUpgradeTasks,
                RenovateConfigProblem::WrongOverrideRuleCount { count: 2 },
            ]
        );

        let both_cmds = "          'bash scripts/refresh-pin-checksums.sh',\n          'bash scripts/refresh-workflow-digests.sh',\n";
        assert!(config.contains(both_cmds));
        let dropped = config.replacen(
            both_cmds,
            "          'bash scripts/refresh-pin-checksums.sh',\n",
            1,
        );
        assert_eq!(
            check_renovate_digest_wiring(&dropped, &global),
            vec![RenovateConfigProblem::WrongOverridePostUpgradeTasks]
        );

        let swapped = config.replacen(
            both_cmds,
            "          'bash scripts/refresh-workflow-digests.sh',\n          'bash scripts/refresh-pin-checksums.sh',\n",
            1,
        );
        assert_eq!(
            check_renovate_digest_wiring(&swapped, &global),
            vec![RenovateConfigProblem::WrongOverridePostUpgradeTasks]
        );

        let unanchored = global.replacen(
            "'^bash scripts/refresh-workflow-digests\\\\.sh$'",
            "'bash scripts/refresh-workflow-digests\\\\.sh$'",
            1,
        );
        assert_ne!(unanchored, global);
        assert_eq!(
            check_renovate_digest_wiring(&config, &unanchored),
            vec![RenovateConfigProblem::WrongGlobalAllowedCommands]
        );
    }
}
