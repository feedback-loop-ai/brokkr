//! Decision 0038 ruling 5: the decision index is derived. The table in
//! `docs/decisions/README.md` names every decision file once, in number
//! order, with the status its `Status:` line carries — so the file can be
//! union-merged (`.gitattributes`) and an appended row is never a
//! conflict, while a duplicated, missing or stale row fails here instead
//! of blocking the merge.
//!
//! Titles are not compared: a decision's heading is never edited (the
//! rename guard lets history keep its old names), while the index row is
//! living prose and carries the current name.

use std::path::PathBuf;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// `(number, file name, status)` for every `NNNN-*.md` in the directory.
fn decision_files() -> Vec<(String, String, String)> {
    let dir = workspace().join("docs/decisions");
    let mut files: Vec<(String, String, String)> = std::fs::read_dir(&dir)
        .expect("docs/decisions")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            name.len() > 5 && name[..4].chars().all(|c| c.is_ascii_digit()) && name.ends_with(".md")
        })
        .map(|name| {
            let contents = std::fs::read_to_string(dir.join(&name)).unwrap();
            let status = contents
                .lines()
                .take(12)
                .find_map(|line| {
                    let line = line.trim_start_matches('*');
                    line.strip_prefix("Status").map(|rest| {
                        rest.trim_start_matches('*')
                            .trim_start_matches(':')
                            .trim()
                            .trim_start_matches('*')
                            .split(|c: char| !c.is_ascii_alphabetic())
                            .next()
                            .unwrap_or_default()
                            .to_string()
                    })
                })
                .unwrap_or_else(|| panic!("{name} carries no Status line"));
            (name[..4].to_string(), name, status)
        })
        .collect();
    files.sort();
    files
}

/// `(number, file name, status)` for every row of the index table.
fn index_rows() -> Vec<(String, String, String)> {
    let readme = std::fs::read_to_string(workspace().join("docs/decisions/README.md")).unwrap();
    readme
        .lines()
        .filter(|line| line.starts_with("| ["))
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
            assert_eq!(cells.len(), 4, "an index row has four cells: {line}");
            let (number, rest) = cells[0]
                .trim_start_matches('[')
                .split_once("](")
                .expect("a linked number");
            let file = rest.trim_end_matches(')');
            assert!(!cells[1].is_empty(), "row {number} has no title");
            (number.to_string(), file.to_string(), cells[3].to_string())
        })
        .collect()
}

#[test]
fn the_index_is_exactly_the_decision_files_in_order_with_their_status() {
    let files = decision_files();
    let rows = index_rows();
    assert!(files.len() > 30, "the walk found too few decisions");
    let mut sorted = rows.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        rows, sorted,
        "the index must list each decision once, in number order"
    );
    assert_eq!(
        rows, files,
        "docs/decisions/README.md drifted from the decision files: a row is missing, duplicated, or carries a status its file does not"
    );
}

#[test]
fn the_index_is_union_merged_so_an_appended_row_is_never_a_conflict() {
    let attributes = std::fs::read_to_string(workspace().join(".gitattributes")).unwrap();
    assert!(
        attributes
            .lines()
            .any(|line| line.trim() == "docs/decisions/README.md merge=union"),
        ".gitattributes lost the union merge for the decision index"
    );
}
