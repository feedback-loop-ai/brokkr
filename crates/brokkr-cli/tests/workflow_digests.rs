//! SHA-256 digest pin guard over `.github/actions/` and `.github/workflows/`
//! (#1417, operator ruling 2026-11-27).
//!
//! Every file under `.github/actions/` and `.github/workflows/` is pinned by
//! byte digest in `.github/workflow-digests.txt`. Any edit to a pinned file,
//! any missing or extra entry, any symlink or non-regular entry, any format
//! deviation, or any local `uses: ./…` resolving outside the two directories
//! fails this guard with a typed [`DigestProblem`] (decision 0071 rulings 3, 8
//! and 9).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use yaml_rust2::{Yaml, YamlLoader};

pub(crate) const DIGEST_FILE: &str = ".github/workflow-digests.txt";
const PINNED_DIRS: [&str; 2] = [".github/actions", ".github/workflows"];
const REFRESH_COMMAND: &str = "bash scripts/refresh-workflow-digests.sh";

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

fn workspace() -> PathBuf {
    crate::workspace_root::workspace()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn is_pinned_rel_path(entry: &str) -> bool {
    if entry.contains(['\\', '\r', '\0']) {
        return false;
    }
    let segments: Vec<&str> = entry.split('/').collect();
    if segments.len() < 3
        || segments
            .iter()
            .any(|s| s.is_empty() || *s == "." || *s == "..")
    {
        return false;
    }
    matches!(
        (segments[0], segments[1]),
        (".github", "actions") | (".github", "workflows")
    )
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
    if !is_pinned_rel_path(entry) {
        return Err(DigestProblem::PathOutsidePinnedDirs {
            path: DIGEST_FILE.to_string(),
            line: line_no,
            entry: entry.to_string(),
        });
    }
    Ok((hex.to_string(), entry.to_string()))
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
    if !require_non_symlink_kind(&root.join(".github"), ".github", true, problems) {
        return None;
    }
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
    entries.sort_by(|a, b| {
        a.file_name()
            .as_encoded_bytes()
            .cmp(b.file_name().as_encoded_bytes())
    });
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
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
    let Some(recorded) = read_digest_file(root, &mut problems) else {
        return problems;
    };
    let (found_files, non_regular) = collect_pinned_tree(root, &mut problems);
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

    fn check_raw_digest_file(raw: &str) -> Vec<DigestProblem> {
        let tree = PlantedTree::new();
        tree.put(".github/actions/sample/action.yml", SAMPLE_ACTION);
        tree.put(".github/workflows/sample.yml", SAMPLE_WF);
        tree.put(DIGEST_FILE, raw);
        check_workflow_digests(tree.path())
    }

    #[test]
    fn digest_file_framing_and_line_format_errors_are_refused_by_type() {
        let digest = sha256_hex(SAMPLE_WF.as_bytes());
        let upper = digest.to_ascii_uppercase();
        let a_digest = sha256_hex(SAMPLE_ACTION.as_bytes());

        assert_eq!(
            check_raw_digest_file(""),
            vec![DigestProblem::EmptyDigestFile {
                path: DIGEST_FILE.to_string(),
            }]
        );
        assert_eq!(
            check_raw_digest_file(&format!(
                "{a_digest}  .github/actions/sample/action.yml\n{digest}  .github/workflows/sample.yml"
            )),
            vec![DigestProblem::MissingFinalNewline {
                path: DIGEST_FILE.to_string(),
            }]
        );
        assert_eq!(
            check_raw_digest_file(&format!(
                "{a_digest}  .github/actions/sample/action.yml\n\n{digest}  .github/workflows/sample.yml\n"
            )),
            vec![DigestProblem::BlankLine {
                path: DIGEST_FILE.to_string(),
                line: 2,
            }]
        );
        assert_eq!(
            check_raw_digest_file(&format!(
                "# generated\n{a_digest}  .github/actions/sample/action.yml\n{digest}  .github/workflows/sample.yml\n"
            )),
            vec![DigestProblem::CommentLine {
                path: DIGEST_FILE.to_string(),
                line: 1,
            }]
        );
        assert_eq!(
            check_raw_digest_file(&format!("{upper}  .github/workflows/sample.yml\n")),
            vec![DigestProblem::UppercaseHex {
                path: DIGEST_FILE.to_string(),
                line: 1,
                found: upper,
            }]
        );
        assert_eq!(
            check_raw_digest_file(&format!("{digest} .github/workflows/sample.yml\n")),
            vec![DigestProblem::WrongSeparator {
                path: DIGEST_FILE.to_string(),
                line: 1,
            }]
        );
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
    fn digest_file_path_duplicate_and_ordering_errors_are_refused_by_type() {
        let digest = sha256_hex(SAMPLE_WF.as_bytes());
        let a_digest = sha256_hex(SAMPLE_ACTION.as_bytes());

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

    #[test]
    fn renovate_config_allows_and_runs_workflow_digest_refresh() {
        let root = workspace();
        let global =
            std::fs::read_to_string(root.join(".github/renovate-global.json5")).expect("global");
        assert!(
            global.contains("'^bash scripts/refresh-workflow-digests\\\\.sh$'"),
            "renovate-global.json5 must allow scripts/refresh-workflow-digests.sh"
        );
        let config = std::fs::read_to_string(root.join(".github/renovate.json5")).expect("config");
        assert_eq!(
            config
                .matches("'bash scripts/refresh-workflow-digests.sh'")
                .count(),
            4,
            "every Renovate rule that can touch a pinned file must refresh workflow digests"
        );
        assert_eq!(
            config.matches("'.github/workflow-digests.txt'").count(),
            4,
            "each digest-refreshing Renovate rule must include .github/workflow-digests.txt in fileFilters"
        );
        let checksums_pos = config
            .find("'bash scripts/refresh-pin-checksums.sh'")
            .expect("pin checksums command");
        let after_checksums = &config[checksums_pos..];
        let digest_after = after_checksums
            .find("'bash scripts/refresh-workflow-digests.sh'")
            .expect("digest refresh after pin checksums");
        let rule_end = after_checksums.find("executionMode:").expect("rule end");
        assert!(
            digest_after < rule_end,
            "digest refresh must run after refresh-pin-checksums.sh in the same postUpgradeTasks"
        );
    }
}
