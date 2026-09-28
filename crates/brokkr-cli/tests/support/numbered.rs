//! The numbered documents a registry directory holds, and the linked rows
//! of the index that lists them, for the test binaries that hold a
//! registry's index to its files (`decisions_index.rs`,
//! `research_registry.rs`). Each includes this through `#[path]`, so the
//! listing and the row reader have one home.

use std::path::Path;

/// `(file name, contents)` for every `NNNN-*.md` in `dir`, in name order.
pub(crate) fn numbered_files(dir: &Path) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        // An entry the listing cannot read fails the test rather than
        // leaving its file out of the ledger. No fixture can make readdir
        // fail on one entry, so no test drives this arm (ruling 9).
        .map(|entry| entry.unwrap_or_else(|error| panic!("{}: {error}", dir.display())))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            name.len() > 5 && name[..4].chars().all(|c| c.is_ascii_digit()) && name.ends_with(".md")
        })
        .map(|name| {
            let text = std::fs::read_to_string(dir.join(&name)).unwrap();
            (name, text)
        })
        .collect();
    files.sort();
    files
}

/// An index row whose first cell links a numbered file: the number, the
/// linked file name and every cell, once the row is shown to hold
/// `count` cells and a title.
pub(crate) fn linked_row(line: &str, count: usize) -> (String, String, Vec<String>) {
    let cells: Vec<String> = line
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect();
    assert_eq!(cells.len(), count, "an index row has {count} cells: {line}");
    let (number, file) = cells[0]
        .trim_start_matches('[')
        .split_once("](")
        .map(|(number, rest)| (number.to_string(), rest.trim_end_matches(')').to_string()))
        .expect("a linked number");
    assert!(!cells[1].is_empty(), "row {number} has no title");
    (number, file, cells)
}
