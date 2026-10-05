//! Decision 0038 ruling 5: the decision index is derived. The table in
//! `docs/decisions/README.md` names every decision file once, in number
//! order, with the status its `Status:` line carries — so the file can be
//! union-merged (`.gitattributes`) and an appended row is never a
//! conflict, while a duplicated, missing or stale row fails here instead
//! of blocking the merge.
//!
//! Issue #363 makes the index a ledger of what is in force. A decision's
//! header, the lines above its first `## ` heading, carries a status from
//! a closed vocabulary, one `Built:` marker, and a pointer for every
//! decision it amends or supersedes, answered by a back-pointer in that
//! decision. The index row repeats the status and renders the marker, and
//! a number with no file is a gap the index explains. Each rule is one
//! function in `CHECKS`, returning the findings it refuses, each named by
//! its file and, where one line is at fault, that line.
//!
//! Titles are not compared: a decision's heading is never edited (the
//! rename guard lets history keep its old names), while the index row is
//! living prose and carries the current name.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use crate::numbered;

use numbered::{linked_row, numbered_files};

/// Where an issue number in the index's `Built` column links to.
const ISSUES: &str = "https://github.com/feedback-loop-ai/brokkr/issues/";

/// The registry directory, from the workspace root.
const DECISIONS: &str = "docs/decisions";

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// The closed status vocabulary `docs/decisions/README.md` states.
/// Partial supersession is a pointer, not a status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Proposed,
    Accepted,
    Superseded,
    Withdrawn,
}

impl Status {
    const ALL: [Status; 4] = [
        Status::Proposed,
        Status::Accepted,
        Status::Superseded,
        Status::Withdrawn,
    ];

    fn word(self) -> &'static str {
        match self {
            Status::Proposed => "proposed",
            Status::Accepted => "accepted",
            Status::Superseded => "superseded",
            Status::Withdrawn => "withdrawn",
        }
    }

    fn parse(word: &str) -> Option<Status> {
        Status::ALL.into_iter().find(|status| status.word() == word)
    }
}

/// How much of a decision the tree carries, kept apart from its status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Build {
    Built,
    Partial,
    Unbuilt,
}

impl Build {
    const ALL: [Build; 3] = [Build::Built, Build::Partial, Build::Unbuilt];

    fn word(self) -> &'static str {
        match self {
            Build::Built => "built",
            Build::Partial => "partial",
            Build::Unbuilt => "unbuilt",
        }
    }

    fn parse(word: &str) -> Option<Build> {
        Build::ALL.into_iter().find(|build| build.word() == word)
    }
}

/// A `Built:` marker: the state, and the issues that carry the rest.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Built {
    build: Build,
    issues: Vec<u32>,
}

impl Built {
    /// `built`, `partial (#429)` or `unbuilt (#317, #319)`, each
    /// optionally followed by ` — ` and a note the index does not carry.
    fn parse(value: &str) -> Option<Built> {
        let head = value.split_once(" — ").map_or(value, |(head, _)| head);
        let (word, issues) = match head.split_once(" (") {
            None => (head, Vec::new()),
            Some((word, list)) => (word, issue_list(list.strip_suffix(')')?)?),
        };
        Some(Built {
            build: Build::parse(word)?,
            issues,
        })
    }

    /// The index cell: the state word, and each issue as a link.
    fn cell(&self) -> String {
        let links: Vec<String> = self
            .issues
            .iter()
            .map(|issue| format!("[#{issue}]({ISSUES}{issue})"))
            .collect();
        if links.is_empty() {
            self.build.word().to_string()
        } else {
            format!("{} ({})", self.build.word(), links.join(", "))
        }
    }
}

/// `#429, #430`: every item a `#` and digits, nothing else.
fn issue_list(list: &str) -> Option<Vec<u32>> {
    list.split(", ")
        .map(|item| digits(item.strip_prefix('#')?))
        .collect()
}

/// A run of ASCII digits and nothing else (`str::parse` admits a `+`).
fn digits(text: &str) -> Option<u32> {
    let all_digits = !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    all_digits.then(|| text.parse().ok()).flatten()
}

/// A four-digit decision number.
fn decision_number(text: &str) -> Option<u16> {
    (text.len() == 4)
        .then(|| digits(text))
        .flatten()
        .and_then(|number| u16::try_from(number).ok())
}

/// A pointer one decision declares at another. The decision that amends
/// or supersedes carries the forward key, the decision it acts on carries
/// the back key, and neither stands without the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Pointer {
    Amends,
    AmendedBy,
    SupersedesInPart,
    SupersededInPartBy,
    Supersedes,
    SupersededBy,
}

impl Pointer {
    const ALL: [Pointer; 6] = [
        Pointer::Amends,
        Pointer::AmendedBy,
        Pointer::SupersedesInPart,
        Pointer::SupersededInPartBy,
        Pointer::Supersedes,
        Pointer::SupersededBy,
    ];

    fn key(self) -> &'static str {
        match self {
            Pointer::Amends => "Amends",
            Pointer::AmendedBy => "Amended by",
            Pointer::SupersedesInPart => "Supersedes in part",
            Pointer::SupersededInPartBy => "Superseded in part by",
            Pointer::Supersedes => "Supersedes",
            Pointer::SupersededBy => "Superseded by",
        }
    }

    /// The key the other decision must carry back.
    fn counterpart(self) -> Pointer {
        match self {
            Pointer::Amends => Pointer::AmendedBy,
            Pointer::AmendedBy => Pointer::Amends,
            Pointer::SupersedesInPart => Pointer::SupersededInPartBy,
            Pointer::SupersededInPartBy => Pointer::SupersedesInPart,
            Pointer::Supersedes => Pointer::SupersededBy,
            Pointer::SupersededBy => Pointer::Supersedes,
        }
    }
}

/// What a decision's text says it does to another, read from the verb that
/// opens the clause; each is declared by its own pointers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Verb {
    Amends,
    Supersedes,
}

impl Verb {
    fn parse(word: &str) -> Option<Verb> {
        match bare(word).as_str() {
            "amends" | "supplements" => Some(Verb::Amends),
            "supersedes" | "overturns" | "replaces" | "retires" => Some(Verb::Supersedes),
            _ => None,
        }
    }

    fn word(self) -> &'static str {
        match self {
            Verb::Amends => "amends",
            Verb::Supersedes => "supersedes",
        }
    }

    /// The header pointers that declare this verb's target.
    fn declared_by(self) -> &'static [Pointer] {
        match self {
            Verb::Amends => &[Pointer::Amends],
            Verb::Supersedes => &[Pointer::Supersedes, Pointer::SupersedesInPart],
        }
    }
}

/// A header marker, for the line it was read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Marker {
    Status,
    Built,
    Pointer(Pointer),
}

/// One decision file, read.
#[derive(Debug)]
struct Decision {
    number: u16,
    file: String,
    /// The status word as written; the vocabulary check reads it.
    status: String,
    built: Option<Built>,
    pointers: BTreeMap<Pointer, Vec<u16>>,
    /// The line each marker was read from, counted from 1.
    lines: BTreeMap<Marker, usize>,
    text: String,
}

impl Decision {
    fn points(&self, pointer: Pointer, number: u16) -> bool {
        self.pointers
            .get(&pointer)
            .is_some_and(|numbers| numbers.contains(&number))
    }

    fn place(&self, line: Option<usize>) -> Place {
        Place {
            file: self.file.clone(),
            line,
        }
    }

    /// The marker's line; a marker the header lacks names the file alone.
    fn at(&self, marker: Marker) -> Place {
        self.place(self.lines.get(&marker).copied())
    }
}

/// One row of the index table.
#[derive(Debug)]
struct Row {
    number: String,
    file: String,
    status: String,
    built: String,
    line: usize,
}

/// One row of the gap table: a number no decision holds. Its reason is
/// asserted present when the row is read.
#[derive(Debug)]
struct Gap {
    number: u16,
    line: usize,
}

/// Where a finding stands: a file in `docs/decisions`, and the line at
/// fault when one is (a missing marker or gap row has none to name).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Place {
    file: String,
    line: Option<usize>,
}

impl Place {
    fn index(line: Option<usize>) -> Place {
        Place {
            file: "README.md".into(),
            line,
        }
    }

    fn holds(self, finding: Finding) -> Located {
        Located {
            place: self,
            finding,
        }
    }
}

/// A finding and the place that holds it, as the failing test prints it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Located {
    place: Place,
    finding: Finding,
}

impl fmt::Display for Located {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Place { file, line } = &self.place;
        match line {
            Some(line) => write!(f, "{DECISIONS}/{file}:{line}: {}", self.finding),
            None => write!(f, "{DECISIONS}/{file}: {}", self.finding),
        }
    }
}

/// What the ledger refuses, in the words the failing test prints.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Finding {
    UnknownStatus {
        decision: u16,
        word: String,
    },
    NoBuilt {
        decision: u16,
    },
    WithoutIssue {
        decision: u16,
    },
    IssueOnBuilt {
        decision: u16,
    },
    MalformedMarker {
        decision: u16,
        line: String,
    },
    UnknownMarker {
        decision: u16,
        line: String,
    },
    RepeatedMarker {
        decision: u16,
        key: &'static str,
    },
    DanglingPointer {
        decision: u16,
        key: &'static str,
        number: u16,
    },
    MissingCounterpart {
        decision: u16,
        key: &'static str,
        number: u16,
        counterpart: &'static str,
    },
    UndeclaredProse {
        decision: u16,
        verb: Verb,
        number: u16,
    },
    UnreadableTarget {
        decision: u16,
        verb: Verb,
        word: String,
    },
    SupersededStatus {
        decision: u16,
    },
    DuplicateNumber {
        number: u16,
    },
    UnexplainedGap {
        number: u16,
    },
    MalformedGap {
        line: String,
    },
    NotAGap {
        number: u16,
    },
    BuiltCell {
        number: u16,
        cell: String,
        expected: String,
    },
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Finding::UnknownStatus { decision, word } => write!(
                f,
                "{decision:04}: status '{word}' is not one of proposed, accepted, superseded, withdrawn"
            ),
            Finding::NoBuilt { decision } => write!(f, "{decision:04}: no 'Built:' marker in its header"),
            Finding::WithoutIssue { decision } => write!(
                f,
                "{decision:04}: a partial or unbuilt decision names the issue that carries the rest"
            ),
            Finding::IssueOnBuilt { decision } => write!(
                f,
                "{decision:04}: a built decision names no issue; its marker reads 'built' alone"
            ),
            Finding::MalformedMarker { decision, line } => {
                write!(f, "{decision:04}: the marker '{line}' does not parse")
            }
            Finding::UnknownMarker { decision, line } => {
                write!(f, "{decision:04}: '{line}' is not a ledger marker")
            }
            Finding::RepeatedMarker { decision, key } => {
                write!(f, "{decision:04}: '{key}:' appears more than once")
            }
            Finding::DanglingPointer { decision, key, number } => {
                write!(f, "{decision:04}: '{key}: {number:04}' names no other decision")
            }
            Finding::MissingCounterpart { decision, key, number, counterpart } => write!(
                f,
                "{decision:04}: '{key}: {number:04}' has no '{counterpart}: {decision:04}' in {number:04}"
            ),
            Finding::UndeclaredProse { decision, verb, number } => {
                let keys: Vec<String> =
                    verb.declared_by().iter().map(|pointer| format!("'{}:'", pointer.key())).collect();
                write!(
                    f,
                    "{decision:04}: its text {} {number:04}, and its header declares no {} pointer to it",
                    verb.word(),
                    keys.join(" or ")
                )
            }
            Finding::UnreadableTarget { decision, verb, word } => write!(
                f,
                "{decision:04}: its text {} '{word}', which is not one decision number; name each decision on its own",
                verb.word()
            ),
            Finding::SupersededStatus { decision } => write!(
                f,
                "{decision:04}: status 'superseded' and a 'Superseded by:' pointer go together"
            ),
            Finding::DuplicateNumber { number } => write!(f, "{number:04}: two decision files hold this number"),
            Finding::UnexplainedGap { number } => {
                write!(f, "{number:04}: no decision holds this number and the index does not say why")
            }
            Finding::MalformedGap { line } => {
                write!(f, "the gap row '{line}' is not one decision number and one note")
            }
            Finding::NotAGap { number } => write!(
                f,
                "{number:04}: the gap table names a number that is not a gap below the last decision"
            ),
            Finding::BuiltCell { number, cell, expected } => write!(
                f,
                "{number:04}: the index's Built cell reads '{cell}', and the file's marker renders '{expected}'"
            ),
        }
    }
}

/// The lines above a decision's first `## ` heading.
fn header(text: &str) -> impl Iterator<Item = &str> {
    text.lines().take_while(|line| !line.starts_with("## "))
}

/// A header's `Status` line, read.
enum StatusLine {
    /// The first word after the colon.
    Word(String),
    /// A `Status` line without its colon, as written.
    NoColon(String),
}

/// The header's `Status` line, bold or plain, and its line number: the
/// first word after its colon, up to the first space (`accepted-ish` is
/// read whole, so the vocabulary refuses it). A line without the colon is
/// not read.
fn status_line(text: &str) -> Option<(usize, StatusLine)> {
    header(text).zip(1..).find_map(|(line, at)| {
        let rest = line.trim_start_matches('*').strip_prefix("Status")?;
        let Some(value) = rest.trim_start_matches('*').strip_prefix(':') else {
            return Some((at, StatusLine::NoColon(line.to_string())));
        };
        let word = value
            .trim()
            .trim_start_matches('*')
            .split_whitespace()
            .next()
            .unwrap_or_default();
        Some((at, StatusLine::Word(word.to_string())))
    })
}

/// The edit distance between two words, for the near-miss net.
fn edit_distance(from: &str, to: &str) -> usize {
    let to: Vec<char> = to.chars().collect();
    let mut row: Vec<usize> = (0..=to.len()).collect();
    for (i, a) in from.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, b) in to.iter().enumerate() {
            let substitution = previous + usize::from(a != *b);
            previous = row[j + 1];
            row[j + 1] = substitution.min(row[j] + 1).min(previous + 1);
        }
    }
    row[to.len()]
}

/// A key that reads like a ledger marker without being one: it carries a
/// marker's stem, or is two edits or fewer from a marker key.
fn near_marker(key: &str) -> bool {
    let key = key.to_lowercase();
    ["built", "amend", "supers"]
        .iter()
        .any(|stem| key.contains(stem))
        || Pointer::ALL
            .map(Pointer::key)
            .into_iter()
            .chain(["Built"])
            .any(|marker| edit_distance(&key, &marker.to_lowercase()) <= 2)
}

/// Markdown a key may be dressed in (`**Built**`, `Supersedes_in_part`,
/// `` `Amends` ``, `Amends/Supersedes`). A dressed key is never a marker,
/// and the near-miss net reads it undressed.
const DRESSING: [char; 4] = ['*', '_', '`', '/'];

/// `text` with its markdown dressing taken out.
fn undress(text: &str) -> String {
    text.chars().filter(|c| !DRESSING.contains(c)).collect()
}

/// A header line shaped `Key: value` whose key, undressed, is words alone:
/// the key as written, the key undressed, and the value.
fn key_value(line: &str) -> Option<(&str, String, &str)> {
    let (key, value) = line.split_once(':')?;
    let undressed = undress(key);
    undressed
        .chars()
        .all(|c| c.is_ascii_alphabetic() || c == ' ' || c == '-')
        .then_some((key, undressed, value.trim()))
}

/// A header line that opens on `Status`, undressed and in any case, with
/// or without its colon: the reader takes the first, so any other is a
/// second status.
fn is_status(line: &str) -> bool {
    undress(line)
        .trim_start()
        .to_lowercase()
        .starts_with("status")
}

/// A header line that is not `Key: value` yet opens, undressed and in any
/// case, on a ledger marker's key as a whole word: `Built built` or
/// `Amends 0001` is a marker written without its colon.
fn is_colonless_marker(line: &str) -> bool {
    let words = undress(line).trim_start().to_lowercase();
    Pointer::ALL
        .map(Pointer::key)
        .into_iter()
        .chain(["Built"])
        .any(|key| {
            words
                .strip_prefix(&key.to_lowercase())
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
        })
}

/// The ledger markers in one decision's header, each with the line it was
/// read from. A key `near_marker` reads as a misspelt or dressed marker is
/// refused, never skipped, and so are a marker without its colon and a
/// second `Status` line.
fn read_markers(decision: &mut Decision, findings: &mut Vec<Located>) {
    let number = decision.number;
    let status_at = decision.lines[&Marker::Status];
    let lines: Vec<String> = header(&decision.text).map(str::to_string).collect();
    for (line, at) in lines.into_iter().zip(1..) {
        let place = decision.place(Some(at));
        let malformed = || Finding::MalformedMarker {
            decision: number,
            line: line.clone(),
        };
        if at != status_at && is_status(&line) {
            findings.push(place.holds(Finding::RepeatedMarker {
                decision: number,
                key: "Status",
            }));
            continue;
        }
        let Some((key, undressed, value)) = key_value(&line) else {
            if is_colonless_marker(&line) {
                findings.push(place.holds(malformed()));
            }
            continue;
        };
        if key == "Built" {
            match (decision.built.is_some(), Built::parse(value)) {
                (true, _) => findings.push(place.holds(Finding::RepeatedMarker {
                    decision: number,
                    key: "Built",
                })),
                (false, None) => findings.push(place.holds(malformed())),
                (false, built) => {
                    decision.built = built;
                    decision.lines.insert(Marker::Built, at);
                }
            }
        } else if let Some(pointer) = Pointer::ALL
            .into_iter()
            .find(|pointer| pointer.key() == key)
        {
            let numbers: Option<Vec<u16>> = value.split(", ").map(decision_number).collect();
            match (decision.pointers.contains_key(&pointer), numbers) {
                (true, _) => findings.push(place.holds(Finding::RepeatedMarker {
                    decision: number,
                    key: pointer.key(),
                })),
                (false, None) => findings.push(place.holds(malformed())),
                (false, Some(numbers)) => {
                    decision.pointers.insert(pointer, numbers);
                    decision.lines.insert(Marker::Pointer(pointer), at);
                }
            }
        } else if near_marker(&undressed) {
            findings.push(place.holds(Finding::UnknownMarker {
                decision: number,
                line: line.clone(),
            }));
        }
    }
}

/// Read one decision file; what its header cannot say becomes a finding.
fn read_decision(file: &str, text: &str, findings: &mut Vec<Located>) -> Decision {
    let number = decision_number(&file[..4]).expect("a decision file starts with its number");
    let (at, status) = status_line(text).unwrap_or_else(|| panic!("{file} carries no Status line"));
    let mut decision = Decision {
        number,
        file: file.to_string(),
        status: String::new(),
        built: None,
        pointers: BTreeMap::new(),
        lines: BTreeMap::from([(Marker::Status, at)]),
        text: text.to_string(),
    };
    match status {
        StatusLine::Word(word) => decision.status = word,
        StatusLine::NoColon(line) => {
            findings.push(decision.place(Some(at)).holds(Finding::MalformedMarker {
                decision: number,
                line,
            }))
        }
    }
    read_markers(&mut decision, findings);
    decision
}

/// An index row: `| [0001](0001-….md) | title | rules | status | built |`,
/// on line `at` of the index.
fn read_row((line, at): (&str, usize)) -> Row {
    let (number, file, cells) = linked_row(line, 5);
    Row {
        number,
        file,
        status: cells[3].clone(),
        built: cells[4].clone(),
        line: at,
    }
}

/// A gap row: `| 0024 | why the number is empty |`, on line `at`. A row
/// that is not one number and one note is refused at its line, and so is
/// a number whose note is empty.
fn read_gap((line, at): (&str, usize), findings: &mut Vec<Located>) -> Option<Gap> {
    let place = Place::index(Some(at));
    let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
    let Some(number) = (cells.len() == 2)
        .then(|| decision_number(cells[0]))
        .flatten()
    else {
        findings.push(place.holds(Finding::MalformedGap {
            line: line.to_string(),
        }));
        return None;
    };
    if cells[1].is_empty() {
        findings.push(place.holds(Finding::UnexplainedGap { number }));
    }
    Some(Gap { number, line: at })
}

/// Every decision, the index's rows and gaps, and what reading them found.
struct Ledger {
    decisions: Vec<Decision>,
    rows: Vec<Row>,
    gaps: Vec<Gap>,
    read: Vec<Located>,
}

impl Ledger {
    /// `files` are `(file name, contents)` for every `NNNN-*.md`.
    fn parse(mut files: Vec<(String, String)>, readme: &str) -> Ledger {
        files.sort();
        let mut read = Vec::new();
        let decisions = files
            .iter()
            .map(|(file, text)| read_decision(file, text, &mut read))
            .collect();
        let rows = readme
            .lines()
            .zip(1..)
            .filter(|(line, _)| line.starts_with("| ["))
            .map(read_row)
            .collect();
        // A gap row opens on its bare number; every other table row opens
        // on a link, a `#` header or a `---` rule.
        let gaps = readme
            .lines()
            .zip(1..)
            .filter(|(line, _)| {
                line.strip_prefix("| ")
                    .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
            })
            .filter_map(|row| read_gap(row, &mut read))
            .collect();
        Ledger {
            decisions,
            rows,
            gaps,
            read,
        }
    }

    fn read() -> Ledger {
        let dir = workspace().join(DECISIONS);
        let readme = std::fs::read_to_string(dir.join("README.md")).unwrap();
        Ledger::parse(numbered_files(&dir), &readme)
    }

    fn decision(&self, number: u16) -> Option<&Decision> {
        self.decisions
            .iter()
            .find(|decision| decision.number == number)
    }

    /// What reading found, then what every rule in `CHECKS` refuses.
    fn findings(&self) -> Vec<Located> {
        let mut findings = self.read.clone();
        findings.extend(CHECKS.iter().flat_map(|check| check(self)));
        findings
    }
}

/// One rule of the ledger: the findings it refuses, each where it stands.
type Check = fn(&Ledger) -> Vec<Located>;

/// Every rule, in the order the failing test prints them. A new rule is
/// one function here.
const CHECKS: [Check; 9] = [
    status_is_in_the_vocabulary,
    built_is_declared,
    pointers_resolve,
    pointers_are_paired,
    superseded_names_its_successor,
    prose_amendments_are_declared,
    each_number_names_one_decision,
    every_gap_is_explained,
    built_cells_render_the_marker,
];

fn status_is_in_the_vocabulary(ledger: &Ledger) -> Vec<Located> {
    ledger
        .decisions
        .iter()
        .filter(|decision| Status::parse(&decision.status).is_none())
        .map(|decision| {
            decision.at(Marker::Status).holds(Finding::UnknownStatus {
                decision: decision.number,
                word: decision.status.clone(),
            })
        })
        .collect()
}

fn built_is_declared(ledger: &Ledger) -> Vec<Located> {
    ledger
        .decisions
        .iter()
        .filter_map(|this @ Decision { number, built, .. }| {
            let decision = *number;
            let finding = match built {
                None => Finding::NoBuilt { decision },
                Some(built) => match (built.build, built.issues.is_empty()) {
                    (Build::Built, false) => Finding::IssueOnBuilt { decision },
                    (Build::Partial | Build::Unbuilt, true) => Finding::WithoutIssue { decision },
                    (Build::Built, true) | (Build::Partial | Build::Unbuilt, false) => return None,
                },
            };
            Some(this.at(Marker::Built).holds(finding))
        })
        .collect()
}

/// `(decision, pointer, target)` for every pointer every decision declares.
fn every_pointer(ledger: &Ledger) -> impl Iterator<Item = (&Decision, Pointer, u16)> {
    ledger.decisions.iter().flat_map(|decision| {
        decision
            .pointers
            .iter()
            .flat_map(move |(pointer, numbers)| {
                numbers
                    .iter()
                    .map(move |number| (decision, *pointer, *number))
            })
    })
}

fn pointers_resolve(ledger: &Ledger) -> Vec<Located> {
    every_pointer(ledger)
        .filter(|(decision, _, number)| {
            *number == decision.number || ledger.decision(*number).is_none()
        })
        .map(|(decision, pointer, number)| {
            decision
                .at(Marker::Pointer(pointer))
                .holds(Finding::DanglingPointer {
                    decision: decision.number,
                    key: pointer.key(),
                    number,
                })
        })
        .collect()
}

fn pointers_are_paired(ledger: &Ledger) -> Vec<Located> {
    every_pointer(ledger)
        .filter_map(|(decision, pointer, number)| {
            let other = ledger
                .decision(number)
                .filter(|other| other.number != decision.number)?;
            let counterpart = pointer.counterpart();
            (!other.points(counterpart, decision.number)).then(|| {
                decision
                    .at(Marker::Pointer(pointer))
                    .holds(Finding::MissingCounterpart {
                        decision: decision.number,
                        key: pointer.key(),
                        number,
                        counterpart: counterpart.key(),
                    })
            })
        })
        .collect()
}

fn superseded_names_its_successor(ledger: &Ledger) -> Vec<Located> {
    ledger
        .decisions
        .iter()
        .filter(|decision| {
            let superseded = Status::parse(&decision.status) == Some(Status::Superseded);
            superseded != decision.pointers.contains_key(&Pointer::SupersededBy)
        })
        .map(|decision| {
            decision
                .at(Marker::Status)
                .holds(Finding::SupersededStatus {
                    decision: decision.number,
                })
        })
        .collect()
}

/// A word without its punctuation, lower-cased.
fn bare(word: &str) -> String {
    word.trim_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_lowercase()
}

/// Whether a word opens an amendment clause.
fn is_amending(word: &str) -> bool {
    Verb::parse(word).is_some()
}

/// Whether a word closes its sentence or clause (`0047.`, `part:`).
fn closes_clause(word: &str) -> bool {
    word.trim_end_matches([')', '"', '\'', '*', '`'])
        .ends_with(['.', ';', ':', '?', '!'])
}

/// What one word of an amending clause names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Target {
    /// A decision number, bare or possessive (`0008`, `0008's`).
    Number(u16),
    /// A word that opens on four digits and is not one number (`0041–0043`,
    /// `0041/0043`, `00081`): refused, never skipped.
    Unreadable(String),
}

/// The target a word names, if it reads like a decision number at all.
fn target(word: &str) -> Option<Target> {
    let word = word.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    let number = ["'s", "’s"]
        .iter()
        .find_map(|possessive| word.strip_suffix(possessive))
        .unwrap_or(word);
    if let Some(number) = decision_number(number) {
        return Some(Target::Number(number));
    }
    let opens_on_a_number = word.len() >= 4 && word.as_bytes()[..4].iter().all(u8::is_ascii_digit);
    opens_on_a_number.then(|| Target::Unreadable(word.to_string()))
}

/// The targets one clause names, from the word after its verb: every word
/// that reads like a decision number up to the clause's end or the next
/// verb, save a number written bare directly before that verb, which is
/// its subject (`the way 0021 amends a tier`). A number its punctuation
/// ends is a target even when a verb follows (`0046; supersedes`,
/// `0004, supersedes`), and a word that is not one number is never a
/// subject: it is refused wherever it stands. Each target comes with its
/// word's index in `words`.
fn clause_targets(words: &[&str]) -> Vec<(usize, Target)> {
    let mut targets = Vec::new();
    for (at, word) in words.iter().enumerate() {
        if is_amending(word) {
            break;
        }
        let subject = word.ends_with(|c: char| c.is_ascii_alphanumeric())
            && words.get(at + 1).is_some_and(|next| is_amending(next));
        match target(word) {
            Some(Target::Number(_)) if subject => {}
            Some(target) => targets.push((at, target)),
            None => {}
        }
        if closes_clause(word) {
            break;
        }
    }
    targets
}

/// Every number a decision's own text says it amends or supersedes, with
/// the verb that says so (`amends decision 0001`, `Amends the read
/// surfaces of decisions 0026 and 0047 in part:`), and the line it first
/// stands on: the numbers its header must point at, each by a pointer of
/// its verb's kind. Only a full stop leaves a verb without an object
/// (`what it amends.`); a comma does not, so `amends, in part, decision
/// 0004` reads 0004, and a sentence that only mentions a number after
/// `amends,` is refused until it is split.
fn prose_targets(text: &str) -> BTreeMap<(Verb, Target), usize> {
    let (words, lines): (Vec<&str>, Vec<usize>) = text
        .lines()
        .zip(1..)
        .flat_map(|(line, at)| line.split_whitespace().map(move |word| (word, at)))
        .unzip();
    let mut targets = BTreeMap::new();
    let verbs = words
        .iter()
        .enumerate()
        .filter(|(_, word)| !word.ends_with('.'))
        .filter_map(|(at, word)| Some((at, Verb::parse(word)?)));
    for (at, verb) in verbs {
        for (offset, target) in clause_targets(&words[at + 1..]) {
            targets
                .entry((verb, target))
                .or_insert(lines[at + 1 + offset]);
        }
    }
    targets
}

fn prose_amendments_are_declared(ledger: &Ledger) -> Vec<Located> {
    ledger
        .decisions
        .iter()
        .flat_map(|decision| {
            prose_targets(&decision.text)
                .into_iter()
                .filter_map(|((verb, target), line)| {
                    let finding = match target {
                        Target::Number(number) => (!verb
                            .declared_by()
                            .iter()
                            .any(|pointer| decision.points(*pointer, number)))
                        .then_some(Finding::UndeclaredProse {
                            decision: decision.number,
                            verb,
                            number,
                        })?,
                        Target::Unreadable(word) => Finding::UnreadableTarget {
                            decision: decision.number,
                            verb,
                            word,
                        },
                    };
                    Some(decision.place(Some(line)).holds(finding))
                })
        })
        .collect()
}

fn each_number_names_one_decision(ledger: &Ledger) -> Vec<Located> {
    let mut seen = BTreeSet::new();
    ledger
        .decisions
        .iter()
        .filter(|decision| !seen.insert(decision.number))
        .map(|decision| {
            decision.place(None).holds(Finding::DuplicateNumber {
                number: decision.number,
            })
        })
        .collect()
}

fn every_gap_is_explained(ledger: &Ledger) -> Vec<Located> {
    let taken: BTreeSet<u16> = ledger
        .decisions
        .iter()
        .map(|decision| decision.number)
        .collect();
    let explained: BTreeSet<u16> = ledger.gaps.iter().map(|gap| gap.number).collect();
    let last = taken.last().copied().unwrap_or_default();
    let unexplained = (1..last)
        .filter(|number| !taken.contains(number) && !explained.contains(number))
        .map(|number| Place::index(None).holds(Finding::UnexplainedGap { number }));
    let not_gaps = ledger
        .gaps
        .iter()
        .filter(|gap| taken.contains(&gap.number) || gap.number > last)
        .map(|gap| Place::index(Some(gap.line)).holds(Finding::NotAGap { number: gap.number }));
    unexplained.chain(not_gaps).collect()
}

fn built_cells_render_the_marker(ledger: &Ledger) -> Vec<Located> {
    ledger
        .rows
        .iter()
        .filter_map(|row| {
            let decision = ledger
                .decisions
                .iter()
                .find(|decision| decision.file == row.file)?;
            let expected = decision.built.as_ref()?.cell();
            (row.built != expected).then(|| {
                Place::index(Some(row.line)).holds(Finding::BuiltCell {
                    number: decision.number,
                    cell: row.built.clone(),
                    expected,
                })
            })
        })
        .collect()
}

/// `(number, file name, status)` for every `NNNN-*.md` in the directory.
fn decision_files() -> Vec<(String, String, String)> {
    Ledger::read()
        .decisions
        .into_iter()
        .map(|decision| {
            (
                format!("{:04}", decision.number),
                decision.file,
                decision.status,
            )
        })
        .collect()
}

/// `(number, file name, status)` for every row of the index table.
fn index_rows() -> Vec<(String, String, String)> {
    Ledger::read()
        .rows
        .into_iter()
        .map(|row| (row.number, row.file, row.status))
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

#[test]
fn the_ledger_in_the_tree_breaks_no_rule() {
    let findings: Vec<String> = Ledger::read()
        .findings()
        .iter()
        .map(Located::to_string)
        .collect();
    assert_eq!(
        findings,
        Vec::<String>::new(),
        "docs/decisions breaks the ledger's rules"
    );
}

/// A small well-formed ledger to plant violations in: 0001 accepted and
/// amended by 0003, 0002 a gap, 0003 proposed and partly built.
struct Fixture {
    files: Vec<(String, String)>,
    readme: String,
}

impl Fixture {
    fn new() -> Fixture {
        let one = "# 0001 — One\n\nStatus: accepted\nBuilt: built\nAmended by: 0003\n\n## Ruling\n\nText.\n";
        let three =
            "# 0003 — Three\n\nStatus: proposed\nBuilt: partial (#7) — the rest\nAmends: 0001\n\n\
                     ## Ruling\n\nThis amends decision 0001.\n";
        let readme = format!(
            "| # | Title | What it rules | Status | Built |\n|---|---|---|---|---|\n\
             | [0001](0001-one.md) | One | one | accepted | built |\n\
             | [0003](0003-three.md) | Three | three | proposed | partial ([#7]({ISSUES}7)) |\n\n\
             | # | Why the number is empty |\n|---|---|\n| 0002 | never written |\n"
        );
        Fixture {
            files: vec![
                ("0001-one.md".into(), one.into()),
                ("0003-three.md".into(), three.into()),
            ],
            readme,
        }
    }

    /// Replace `from` with `to` in the file whose name starts `prefix`.
    fn edit(mut self, prefix: &str, from: &str, to: &str) -> Fixture {
        let (_, text) = self
            .files
            .iter_mut()
            .find(|(file, _)| file.starts_with(prefix))
            .expect("a fixture file");
        assert!(text.contains(from), "the fixture holds {from:?}");
        *text = text.replace(from, to);
        self
    }

    fn edit_readme(mut self, from: &str, to: &str) -> Fixture {
        assert!(
            self.readme.contains(from),
            "the fixture index holds {from:?}"
        );
        self.readme = self.readme.replace(from, to);
        self
    }

    fn findings(self) -> Vec<Finding> {
        Ledger::parse(self.files, &self.readme)
            .findings()
            .into_iter()
            .map(|located| located.finding)
            .collect()
    }

    /// Each finding as the failing test prints it, file and line first.
    fn located(self) -> Vec<String> {
        Ledger::parse(self.files, &self.readme)
            .findings()
            .iter()
            .map(Located::to_string)
            .collect()
    }
}

#[test]
fn a_well_formed_ledger_has_no_findings() {
    assert_eq!(Fixture::new().findings(), vec![]);
}

#[test]
fn an_unknown_status_word_is_refused() {
    let findings = Fixture::new()
        .edit("0001", "Status: accepted", "Status: enacted")
        .findings();
    assert_eq!(
        findings,
        vec![Finding::UnknownStatus {
            decision: 1,
            word: "enacted".into()
        }]
    );
    let findings = Fixture::new()
        .edit("0001", "Status: accepted", "Status: accepted-ish")
        .findings();
    assert_eq!(
        findings,
        vec![Finding::UnknownStatus {
            decision: 1,
            word: "accepted-ish".into()
        }]
    );
    let findings = Fixture::new()
        .edit("0001", "Status: accepted", "Status accepted")
        .findings();
    assert_eq!(
        findings,
        vec![
            Finding::MalformedMarker {
                decision: 1,
                line: "Status accepted".into()
            },
            Finding::UnknownStatus {
                decision: 1,
                word: String::new()
            },
        ]
    );
}

#[test]
fn a_missing_back_pointer_is_refused_from_either_side() {
    let findings = Fixture::new()
        .edit("0001", "Amended by: 0003\n", "")
        .findings();
    assert_eq!(
        findings,
        vec![Finding::MissingCounterpart {
            decision: 3,
            key: "Amends",
            number: 1,
            counterpart: "Amended by"
        }]
    );
    let findings = Fixture::new().edit("0003", "Amends: 0001\n", "").findings();
    assert_eq!(
        findings,
        vec![
            Finding::MissingCounterpart {
                decision: 1,
                key: "Amended by",
                number: 3,
                counterpart: "Amends"
            },
            Finding::UndeclaredProse {
                decision: 3,
                verb: Verb::Amends,
                number: 1
            },
        ]
    );
}

#[test]
fn a_pointer_to_no_other_decision_is_refused() {
    let findings = Fixture::new()
        .edit("0003", "Amends: 0001", "Amends: 0001, 0009")
        .findings();
    assert_eq!(
        findings,
        vec![Finding::DanglingPointer {
            decision: 3,
            key: "Amends",
            number: 9
        }]
    );
    let findings = Fixture::new()
        .edit("0003", "Amends: 0001", "Amends: 0001, 0003")
        .findings();
    assert_eq!(
        findings,
        vec![Finding::DanglingPointer {
            decision: 3,
            key: "Amends",
            number: 3
        }]
    );
}

#[test]
fn an_amendment_in_the_text_needs_a_pointer_in_the_header() {
    let findings = Fixture::new()
        .edit(
            "0001",
            "Text.",
            "It supersedes\ndecision 0003, and amends nothing else.",
        )
        .findings();
    assert_eq!(
        findings,
        vec![Finding::UndeclaredProse {
            decision: 1,
            verb: Verb::Supersedes,
            number: 3
        }]
    );
    // Every number in the clause is a target, however the clause is
    // worded; a number that is the next verb's subject is not.
    let findings = Fixture::new()
        .edit(
            "0003",
            "This amends decision 0001.",
            "Amends the read surfaces of decisions 0001 and 0004 in part: \
             pins it amends by evidence the way 0009 amends a tier. \
             The rulings are what it amends. 0007 stands.",
        )
        .findings();
    assert_eq!(
        findings,
        vec![Finding::UndeclaredProse {
            decision: 3,
            verb: Verb::Amends,
            number: 4
        }]
    );
    // A number that ends its clause is a target, not the next verb's
    // subject, and every verb the table knows opens a clause.
    let findings = Fixture::new()
        .edit(
            "0003",
            "This amends decision 0001.",
            "This amends 0001 and 0004; supplements 0005; overturns 0006.",
        )
        .findings();
    let undeclared = |verb, number| Finding::UndeclaredProse {
        decision: 3,
        verb,
        number,
    };
    assert_eq!(
        findings,
        vec![
            undeclared(Verb::Amends, 4),
            undeclared(Verb::Amends, 5),
            undeclared(Verb::Supersedes, 6),
        ]
    );
}

#[test]
fn a_number_the_clause_cannot_read_is_refused_not_skipped() {
    // A possessive names its decision.
    for possessive in ["0004's", "0004’s"] {
        let findings = Fixture::new()
            .edit(
                "0003",
                "decision 0001.",
                &format!("decision 0001 and {possessive} ruling 2."),
            )
            .findings();
        assert_eq!(
            findings,
            vec![Finding::UndeclaredProse {
                decision: 3,
                verb: Verb::Amends,
                number: 4
            }]
        );
    }
    // A range, or anything else opening on four digits, is not one number.
    for word in ["0004–0006", "0004/0006", "00041"] {
        let findings = Fixture::new()
            .edit(
                "0003",
                "decision 0001.",
                &format!("decision 0001 and ({word})."),
            )
            .findings();
        assert_eq!(
            findings,
            vec![Finding::UnreadableTarget {
                decision: 3,
                verb: Verb::Amends,
                word: word.into()
            }]
        );
    }
}

#[test]
fn a_comma_leaves_the_clause_open() {
    let undeclared = |verb, number| Finding::UndeclaredProse {
        decision: 3,
        verb,
        number,
    };
    let unreadable = |word: &str| Finding::UnreadableTarget {
        decision: 3,
        verb: Verb::Amends,
        word: word.into(),
    };
    // A number before a comma is an object, not the next verb's subject;
    // a word that is not one number is refused even where a subject stands;
    // and a comma after the verb does not leave it without an object.
    let cases = [
        (
            "This amends 0004, supersedes 0005.",
            vec![undeclared(Verb::Amends, 4), undeclared(Verb::Supersedes, 5)],
        ),
        (
            "This amends 0004–0006, supersedes 0005.",
            vec![unreadable("0004–0006"), undeclared(Verb::Supersedes, 5)],
        ),
        (
            "This amends 0004/0006 amends 0001.",
            vec![unreadable("0004/0006")],
        ),
        (
            "This amends, in part, decision 0004.",
            vec![undeclared(Verb::Amends, 4)],
        ),
        (
            "It names the rulings it amends, as 0004 asks.",
            vec![undeclared(Verb::Amends, 4)],
        ),
    ];
    for (text, expected) in cases {
        let findings = Fixture::new()
            .edit("0003", "This amends decision 0001.", text)
            .findings();
        assert_eq!(findings, expected, "{text}");
    }
}

#[test]
fn a_verb_is_declared_by_a_pointer_of_its_own_kind() {
    // 0003 declares `Amends: 0001`; text that supersedes 0001 is not
    // declared by it, and a partial supersession is declared by either
    // supersession pointer.
    let findings = Fixture::new()
        .edit("0003", "This amends decision 0001.", "It supersedes 0001.")
        .findings();
    assert_eq!(
        findings,
        vec![Finding::UndeclaredProse {
            decision: 3,
            verb: Verb::Supersedes,
            number: 1
        }]
    );
    let findings = Fixture::new()
        .edit("0001", "Amended by: 0003", "Superseded in part by: 0003")
        .edit("0003", "Amends: 0001", "Supersedes in part: 0001")
        .edit(
            "0003",
            "This amends decision 0001.",
            "It retires 0001 in part.",
        )
        .findings();
    assert_eq!(findings, vec![]);
    let findings = Fixture::new()
        .edit("0001", "Amended by: 0003", "Superseded in part by: 0003")
        .edit("0003", "Amends: 0001", "Supersedes in part: 0001")
        .findings();
    assert_eq!(
        findings,
        vec![Finding::UndeclaredProse {
            decision: 3,
            verb: Verb::Amends,
            number: 1
        }]
    );
}

#[test]
fn a_superseded_status_and_its_pointer_go_together() {
    let findings = Fixture::new()
        .edit("0001", "Status: accepted", "Status: superseded")
        .findings();
    assert_eq!(findings, vec![Finding::SupersededStatus { decision: 1 }]);
    let findings = Fixture::new()
        .edit(
            "0001",
            "Amended by: 0003",
            "Amended by: 0003\nSuperseded by: 0003",
        )
        .findings();
    assert_eq!(
        findings,
        vec![
            Finding::MissingCounterpart {
                decision: 1,
                key: "Superseded by",
                number: 3,
                counterpart: "Supersedes"
            },
            Finding::SupersededStatus { decision: 1 },
        ]
    );
}

#[test]
fn every_decision_declares_how_much_is_built() {
    let findings = Fixture::new().edit("0001", "Built: built\n", "").findings();
    assert_eq!(findings, vec![Finding::NoBuilt { decision: 1 }]);
    let findings = Fixture::new()
        .edit("0003", "partial (#7)", "partial")
        .findings();
    assert_eq!(
        findings,
        vec![Finding::WithoutIssue { decision: 3 }, bad_cell("partial")]
    );
    // What is built leaves nothing for an issue to carry.
    let findings = Fixture::new()
        .edit("0001", "Built: built\n", "Built: built (#9)\n")
        .edit_readme(
            "| accepted | built |",
            &format!("| accepted | built ([#9]({ISSUES}9)) |"),
        )
        .findings();
    assert_eq!(findings, vec![Finding::IssueOnBuilt { decision: 1 }]);
    let findings = Fixture::new()
        .edit("0003", "partial (#7)", "partial (#+7)")
        .findings();
    assert_eq!(
        findings,
        vec![
            Finding::MalformedMarker {
                decision: 3,
                line: "Built: partial (#+7) — the rest".into()
            },
            Finding::NoBuilt { decision: 3 },
        ]
    );
}

/// 0003's cell as the fixture index writes it, against `expected`.
fn bad_cell(expected: &str) -> Finding {
    Finding::BuiltCell {
        number: 3,
        cell: format!("partial ([#7]({ISSUES}7))"),
        expected: expected.into(),
    }
}

#[test]
fn a_marker_is_read_once_and_exactly() {
    let findings = Fixture::new()
        .edit("0001", "Built: built\n", "Built: built\nBuilt: built\n")
        .findings();
    assert_eq!(
        findings,
        vec![Finding::RepeatedMarker {
            decision: 1,
            key: "Built"
        }]
    );
    let findings = Fixture::new()
        .edit("0001", "Amended by: 0003", "Amended-by: 0003")
        .findings();
    assert_eq!(
        findings,
        vec![
            Finding::UnknownMarker {
                decision: 1,
                line: "Amended-by: 0003".into()
            },
            Finding::MissingCounterpart {
                decision: 3,
                key: "Amends",
                number: 1,
                counterpart: "Amended by"
            },
        ]
    );
    let findings = Fixture::new()
        .edit("0003", "Amends: 0001", "Amends: 1")
        .findings();
    assert_eq!(
        findings,
        vec![
            Finding::MalformedMarker {
                decision: 3,
                line: "Amends: 1".into()
            },
            Finding::MissingCounterpart {
                decision: 1,
                key: "Amended by",
                number: 3,
                counterpart: "Amends"
            },
            Finding::UndeclaredProse {
                decision: 3,
                verb: Verb::Amends,
                number: 1
            },
        ]
    );
    let findings = Fixture::new()
        .edit("0003", "Amends: 0001\n", "Amends: 0001\nAmends: 0001\n")
        .findings();
    assert_eq!(
        findings,
        vec![Finding::RepeatedMarker {
            decision: 3,
            key: "Amends"
        }]
    );
    // A second status is refused, bold, dressed or without its colon.
    for second in [
        "Status: enacted",
        "**Status:** accepted",
        "_Status_: enacted",
        "Status enacted",
    ] {
        let findings = Fixture::new()
            .edit(
                "0001",
                "Status: accepted\n",
                &format!("Status: accepted\n{second}\n"),
            )
            .findings();
        assert_eq!(
            findings,
            vec![Finding::RepeatedMarker {
                decision: 1,
                key: "Status"
            }],
            "{second}"
        );
    }
}

#[test]
fn a_misspelt_marker_is_refused_not_skipped() {
    // A key dressed in markdown is a marker the reader would skip, so it is
    // refused as one: bold, underscored, quoted or joined by a slash.
    for key in [
        "Amens",
        "Suprsedes",
        "Superseding by",
        "**Supersedes in part**",
        "Supersedes_in_part",
        "`Amends`",
        "Amends/Supersedes",
        "**Built**",
    ] {
        let line = format!("{key}: 0001");
        let findings = Fixture::new()
            .edit("0003", "Amends: 0001", &line)
            .findings();
        assert_eq!(
            findings,
            vec![
                Finding::UnknownMarker { decision: 3, line },
                Finding::MissingCounterpart {
                    decision: 1,
                    key: "Amended by",
                    number: 3,
                    counterpart: "Amends"
                },
                Finding::UndeclaredProse {
                    decision: 3,
                    verb: Verb::Amends,
                    number: 1
                },
            ]
        );
    }
}

#[test]
fn an_unexplained_or_false_gap_is_refused() {
    let findings = Fixture::new()
        .edit_readme("| 0002 | never written |\n", "")
        .findings();
    assert_eq!(findings, vec![Finding::UnexplainedGap { number: 2 }]);
    let findings = Fixture::new()
        .edit_readme(
            "| 0002 | never written |\n",
            "| 0002 | never written |\n| 0003 | taken |\n| 0004 | ahead |\n",
        )
        .findings();
    assert_eq!(
        findings,
        vec![
            Finding::NotAGap { number: 3 },
            Finding::NotAGap { number: 4 }
        ]
    );
}

#[test]
fn two_files_with_one_number_are_refused() {
    let mut fixture = Fixture::new();
    let copy = fixture.files[0].1.clone();
    fixture.files.push(("0001-again.md".into(), copy));
    assert_eq!(
        fixture.findings(),
        vec![Finding::DuplicateNumber { number: 1 }]
    );
}

#[test]
fn the_built_cell_renders_the_files_marker() {
    let findings = Fixture::new()
        .edit("0003", "partial (#7)", "unbuilt (#7, #8)")
        .findings();
    assert_eq!(
        findings,
        vec![bad_cell(&format!(
            "unbuilt ([#7]({ISSUES}7), [#8]({ISSUES}8))"
        ))]
    );
}

#[test]
fn every_refusal_names_its_file_and_line() {
    let three = "docs/decisions/0003-three.md";
    let cases = [
        (
            Fixture::new().edit("0001", "Status: accepted", "Status: enacted"),
            vec![
                "docs/decisions/0001-one.md:3: 0001: status 'enacted' is not one of proposed, accepted, superseded, withdrawn".to_string(),
            ],
        ),
        (
            Fixture::new().edit("0001", "Built: built", "Status: enacted\nBuilt: built"),
            vec!["docs/decisions/0001-one.md:4: 0001: 'Status:' appears more than once".to_string()],
        ),
        (
            Fixture::new().edit("0001", "Built: built", "status: enacted\nBuilt: built"),
            vec!["docs/decisions/0001-one.md:4: 0001: 'Status:' appears more than once".to_string()],
        ),
        // A marker without its colon is refused at its own line.
        (
            Fixture::new().edit("0001", "Built: built", "Built built"),
            vec![
                "docs/decisions/0001-one.md:4: 0001: the marker 'Built built' does not parse".to_string(),
                "docs/decisions/0001-one.md: 0001: no 'Built:' marker in its header".to_string(),
            ],
        ),
        (
            Fixture::new().edit("0001", "Amended by: 0003", "**Amended by** 0003"),
            vec![
                "docs/decisions/0001-one.md:5: 0001: the marker '**Amended by** 0003' does not parse".to_string(),
                format!("{three}:5: 0003: 'Amends: 0001' has no 'Amended by: 0003' in 0001"),
            ],
        ),
        (
            Fixture::new().edit("0003", "partial (#7)", "partial (#+7)"),
            vec![
                format!("{three}:4: 0003: the marker 'Built: partial (#+7) — the rest' does not parse"),
                format!("{three}: 0003: no 'Built:' marker in its header"),
            ],
        ),
        (
            Fixture::new().edit("0003", "Amends: 0001", "Amends: 0001, 0009"),
            vec![format!("{three}:5: 0003: 'Amends: 0009' names no other decision")],
        ),
        (
            Fixture::new().edit_readme("| 0002 | never written |\n", ""),
            vec![
                "docs/decisions/README.md: 0002: no decision holds this number and the index does not say why".to_string(),
            ],
        ),
        (
            Fixture::new().edit_readme("| 0002 | never written |\n", "| 0002 |  |\n"),
            vec![
                "docs/decisions/README.md:8: 0002: no decision holds this number and the index does not say why".to_string(),
            ],
        ),
        (
            Fixture::new().edit_readme("| 0002 | never written |\n", "| 0002 | never | written |\n"),
            vec![
                "docs/decisions/README.md:8: the gap row '| 0002 | never | written |' is not one decision number and one note".to_string(),
                "docs/decisions/README.md: 0002: no decision holds this number and the index does not say why".to_string(),
            ],
        ),
        (
            Fixture::new().edit_readme("never written |\n", "never written |\n| 0004 | ahead |\n"),
            vec![
                "docs/decisions/README.md:9: 0004: the gap table names a number that is not a gap below the last decision".to_string(),
            ],
        ),
        // The line is the unreadable word's, not its verb's.
        (
            Fixture::new().edit("0003", "decision 0001.", "decision 0001 and\n(0004–0006)."),
            vec![format!(
                "{three}:10: 0003: its text amends '0004–0006', which is not one decision number; name each decision on its own"
            )],
        ),
        (
            Fixture::new().edit("0003", "partial (#7)", "unbuilt (#7)"),
            vec![format!(
                "docs/decisions/README.md:4: 0003: the index's Built cell reads 'partial ([#7]({ISSUES}7))', and the file's marker renders 'unbuilt ([#7]({ISSUES}7))'"
            )],
        ),
    ];
    for (fixture, expected) in cases {
        assert_eq!(fixture.located(), expected);
    }
}

#[test]
fn every_finding_reads_in_the_operators_words() {
    let texts: Vec<String> = [
        Finding::UnknownStatus {
            decision: 1,
            word: "enacted".into(),
        },
        Finding::NoBuilt { decision: 1 },
        Finding::WithoutIssue { decision: 1 },
        Finding::IssueOnBuilt { decision: 1 },
        Finding::MalformedMarker {
            decision: 1,
            line: "Built: done".into(),
        },
        Finding::UnknownMarker {
            decision: 1,
            line: "Amended-by: 0003".into(),
        },
        Finding::RepeatedMarker {
            decision: 1,
            key: "Built",
        },
        Finding::DanglingPointer {
            decision: 1,
            key: "Amends",
            number: 9,
        },
        Finding::MissingCounterpart {
            decision: 3,
            key: "Amends",
            number: 1,
            counterpart: "Amended by",
        },
        Finding::UndeclaredProse {
            decision: 3,
            verb: Verb::Amends,
            number: 1,
        },
        Finding::UndeclaredProse {
            decision: 3,
            verb: Verb::Supersedes,
            number: 1,
        },
        Finding::UnreadableTarget {
            decision: 3,
            verb: Verb::Amends,
            word: "0041–0043".into(),
        },
        Finding::SupersededStatus { decision: 1 },
        Finding::DuplicateNumber { number: 1 },
        Finding::UnexplainedGap { number: 2 },
        Finding::MalformedGap {
            line: "| 0002 |".into(),
        },
        Finding::NotAGap { number: 3 },
        Finding::BuiltCell {
            number: 3,
            cell: "built".into(),
            expected: "partial".into(),
        },
    ]
    .iter()
    .map(Finding::to_string)
    .collect();
    assert_eq!(
        texts,
        [
            "0001: status 'enacted' is not one of proposed, accepted, superseded, withdrawn",
            "0001: no 'Built:' marker in its header",
            "0001: a partial or unbuilt decision names the issue that carries the rest",
            "0001: a built decision names no issue; its marker reads 'built' alone",
            "0001: the marker 'Built: done' does not parse",
            "0001: 'Amended-by: 0003' is not a ledger marker",
            "0001: 'Built:' appears more than once",
            "0001: 'Amends: 0009' names no other decision",
            "0003: 'Amends: 0001' has no 'Amended by: 0003' in 0001",
            "0003: its text amends 0001, and its header declares no 'Amends:' pointer to it",
            "0003: its text supersedes 0001, and its header declares no 'Supersedes:' or 'Supersedes in part:' pointer to it",
            "0003: its text amends '0041–0043', which is not one decision number; name each decision on its own",
            "0001: status 'superseded' and a 'Superseded by:' pointer go together",
            "0001: two decision files hold this number",
            "0002: no decision holds this number and the index does not say why",
            "the gap row '| 0002 |' is not one decision number and one note",
            "0003: the gap table names a number that is not a gap below the last decision",
            "0003: the index's Built cell reads 'built', and the file's marker renders 'partial'",
        ]
    );
}
