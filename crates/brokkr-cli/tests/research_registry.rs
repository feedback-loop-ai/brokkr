//! Decision 0044: the research registry is a record with receipts.
//!
//! `docs/research/` holds one numbered file per article the product has
//! read; each finding in it is one row that says what Brokkr does about
//! it and where to check. This test holds three things the prose cannot:
//! the index in `docs/research/README.md` is exactly the entries in the
//! directory, in order, with their status (rulings 1 and 3); every
//! classification is one of the closed vocabulary (ruling 1); and every
//! classification that claims something carries a citation that resolves
//! — a decision that exists, a path that exists, or an issue number
//! (ruling 2). A row that says `implemented` and points nowhere is an
//! opinion, and this directory is not for opinions.

use std::path::PathBuf;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

const CLASSES: [&str; 5] = [
    "implemented",
    "alternative",
    "declined",
    "planned",
    "not-planned",
];

struct Entry {
    number: String,
    file: String,
    status: String,
    findings: Vec<Finding>,
}

struct Finding {
    index: usize,
    class: String,
    citation: String,
}

fn header(contents: &str, key: &str, file: &str) -> String {
    contents
        .lines()
        .take(12)
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix(':'))
        })
        .map(|rest| rest.trim().to_string())
        .unwrap_or_else(|| panic!("{file} carries no `{key}:` line in its header"))
}

fn is_date(text: &str) -> bool {
    text.len() == 10
        && text.chars().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        })
}

fn entries() -> Vec<Entry> {
    let dir = workspace().join("docs/research");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("docs/research")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            name.len() > 5 && name[..4].chars().all(|c| c.is_ascii_digit()) && name.ends_with(".md")
        })
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|file| {
            let contents = std::fs::read_to_string(dir.join(&file)).unwrap();
            let source = header(&contents, "Source", &file);
            assert!(
                source.starts_with("https://") || source.starts_with("http://"),
                "{file}: Source is not a link: {source}"
            );
            assert!(
                is_date(&header(&contents, "Read", &file)),
                "{file}: Read is not a YYYY-MM-DD date"
            );
            let status_line = header(&contents, "Status", &file);
            let status = status_line
                .split(|c: char| !c.is_ascii_alphabetic())
                .next()
                .unwrap_or_default()
                .to_string();
            match status.as_str() {
                "proposed" => assert_eq!(
                    status_line, "proposed",
                    "{file}: a proposed status carries nothing else"
                ),
                "ruled" => {
                    let date = status_line
                        .strip_prefix("ruled (")
                        .and_then(|rest| rest.strip_suffix(')'))
                        .unwrap_or_else(|| panic!("{file}: a ruling reads `ruled (<date>)`"));
                    assert!(is_date(date), "{file}: the ruling date is not YYYY-MM-DD");
                }
                other => panic!("{file}: status `{other}` is neither proposed nor ruled"),
            }
            header(&contents, "Intake", &file);
            let findings: Vec<Finding> = contents
                .lines()
                .filter(|line| {
                    line.starts_with("| ")
                        && line[2..].chars().next().is_some_and(|c| c.is_ascii_digit())
                })
                .map(|line| {
                    let cells: Vec<&str> =
                        line.trim_matches('|').split('|').map(str::trim).collect();
                    assert_eq!(
                        cells.len(),
                        4,
                        "{file}: a finding row has four cells: {line}"
                    );
                    assert!(
                        !cells[1].is_empty(),
                        "{file}: a finding row names its finding"
                    );
                    Finding {
                        index: cells[0].parse().unwrap_or_else(|_| {
                            panic!("{file}: `{}` is not a finding number", cells[0])
                        }),
                        class: cells[2].to_string(),
                        citation: cells[3].to_string(),
                    }
                })
                .collect();
            assert!(
                !findings.is_empty(),
                "{file}: an entry records at least one finding"
            );
            Entry {
                number: file[..4].to_string(),
                file,
                status,
                findings,
            }
        })
        .collect()
}

/// `decision NNNN` tokens the citation names, each checked to exist.
fn cited_decisions(citation: &str) -> Vec<String> {
    let decisions = workspace().join("docs/decisions");
    citation
        .split("decision ")
        .skip(1)
        .map(|rest| rest.chars().take(4).collect::<String>())
        .filter(|number| number.len() == 4 && number.chars().all(|c| c.is_ascii_digit()))
        .inspect(|number| {
            let exists = std::fs::read_dir(&decisions)
                .unwrap()
                .flatten()
                .any(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(&format!("{number}-"))
                });
            assert!(exists, "decision {number} does not exist: {citation}");
        })
        .collect()
}

/// Backticked paths the citation names, each checked to exist in the tree.
fn cited_paths(citation: &str) -> Vec<String> {
    let root = workspace();
    citation
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .inspect(|path| {
            assert!(
                root.join(path).exists(),
                "cited path does not exist: {path}"
            );
        })
        .collect()
}

fn cited_issues(citation: &str) -> Vec<String> {
    citation
        .split('#')
        .skip(1)
        .map(|rest| {
            rest.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
        })
        .filter(|number| !number.is_empty())
        .collect()
}

#[test]
fn every_finding_is_classified_from_the_vocabulary_and_its_citation_resolves() {
    for entry in entries() {
        for (position, finding) in entry.findings.iter().enumerate() {
            let file = &entry.file;
            assert_eq!(
                finding.index,
                position + 1,
                "{file}: findings are numbered from 1 in order"
            );
            assert!(
                CLASSES.contains(&finding.class.as_str()),
                "{file} finding {}: `{}` is not in the vocabulary {CLASSES:?}",
                finding.index,
                finding.class
            );
            let citation = &finding.citation;
            let decisions = cited_decisions(citation);
            let paths = cited_paths(citation);
            let issues = cited_issues(citation);
            match finding.class.as_str() {
                "implemented" | "alternative" => assert!(
                    !decisions.is_empty() || !paths.is_empty(),
                    "{file} finding {}: `{}` cites no decision and no path",
                    finding.index,
                    finding.class
                ),
                "declined" => assert!(
                    !decisions.is_empty()
                        || paths
                            .iter()
                            .any(|p| p.starts_with("agents/charters/") || p.ends_with("README.md")),
                    "{file} finding {}: `declined` cites neither the decision nor the charter it conflicts with",
                    finding.index
                ),
                "planned" => assert!(
                    !issues.is_empty(),
                    "{file} finding {}: `planned` cites no issue number",
                    finding.index
                ),
                "not-planned" => assert!(
                    citation.is_empty(),
                    "{file} finding {}: `not-planned` carries no citation; a reason belongs in the summary",
                    finding.index
                ),
                _ => unreachable!(),
            }
        }
    }
}

#[test]
fn the_index_is_exactly_the_entries_in_order_with_their_status() {
    let entries = entries();
    let readme = std::fs::read_to_string(workspace().join("docs/research/README.md")).unwrap();
    let rows: Vec<(String, String, usize, String)> = readme
        .lines()
        .filter(|line| line.starts_with("| ["))
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
            assert_eq!(cells.len(), 5, "an index row has five cells: {line}");
            let (number, rest) = cells[0]
                .trim_start_matches('[')
                .split_once("](")
                .expect("a linked number");
            assert!(!cells[1].is_empty(), "row {number} has no title");
            assert!(
                cells[2].contains("](http"),
                "row {number} has no source link"
            );
            (
                number.to_string(),
                rest.trim_end_matches(')').to_string(),
                cells[3]
                    .parse()
                    .unwrap_or_else(|_| panic!("row {number}: findings is a count")),
                cells[4].to_string(),
            )
        })
        .collect();
    let expected: Vec<(String, String, usize, String)> = entries
        .iter()
        .map(|e| {
            (
                e.number.clone(),
                e.file.clone(),
                e.findings.len(),
                e.status.clone(),
            )
        })
        .collect();
    assert_eq!(rows, expected, "docs/research/README.md is derived from the entries: one row per file, in order, with its findings count and status");
    let mut numbers: Vec<&str> = entries.iter().map(|e| e.number.as_str()).collect();
    numbers.dedup();
    assert_eq!(numbers.len(), entries.len(), "two entries share a number");
}
