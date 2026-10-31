//! Decision 0082 rulings 1 and 2, beside the ledger they read: a contract
//! version is minted only under an accepted ruling, an addendum by its own
//! exact status line, and a superseded ruling keeps a minting only when
//! exactly one successor its pointer names carries the keep line. The files
//! already minted on pending or absent authority stand, marked, on a closed
//! list.
//!
//! The cell's grammar lives once, in `contract_lifecycle::minted_form`; this
//! module reads the ruling a cell names.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::{header, Decision, Ledger, Pointer, Status, DECISIONS};
use crate::contract_lifecycle::{self, Minted};

/// The files decision 0082 ruling 2 records as standing without an accepted
/// ruling, with the exact cell each row writes. The list only shrinks.
///
/// `realms.v7` left the list when decision 0075 was accepted on 2026-10-12.
/// `panel-carry.v1`, minted under decision 0093 after this rule was
/// accepted, was never on it: the operator accepted 0093 on 2026-10-30
/// rather than widen a list that only shrinks.
const PENDING: [(&str, &str); 8] = [
    (
        "contracts/seat-record.v5.schema.json",
        "0056, enacted, pending acceptance",
    ),
    (
        "contracts/realms.v5.schema.json",
        "0057, enacted, pending acceptance",
    ),
    (
        "contracts/run-manifest.v10.schema.json",
        "0057, enacted, pending acceptance",
    ),
    (
        "contracts/phase-entered-head.v1.schema.json",
        "0039, enacted, pending acceptance",
    ),
    (
        "contracts/effect-cleanup.v1.schema.json",
        "0006 (2026-09-27 addendum), enacted, pending acceptance",
    ),
    (
        "contracts/run-manifest.v13.schema.json",
        "operator ruling 2026-10-10 (#430, PR #621), enacted, pending acceptance",
    ),
    (
        "contracts/dispatch-envelope.v2.schema.json",
        "no decision (PR #28), enacted, pending acceptance",
    ),
    (
        "contracts/run-manifest.v2.schema.json",
        "no decision (PR #28), enacted, pending acceptance",
    ),
];

/// What an addendum's own status line says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AddendumStatus {
    Accepted,
    Proposed,
    Withdrawn,
}

/// The first non-blank line under `## Addendum — <date>`, read whole.
fn read_addendum(decision: &Decision, date: &str) -> Result<AddendumStatus, String> {
    let heading = format!("## Addendum — {date}");
    let headings: Vec<usize> = decision
        .text
        .lines()
        .enumerate()
        .filter(|(_, line)| line.starts_with(&heading))
        .map(|(at, _)| at)
        .collect();
    if headings.len() != 1 {
        return Err(format!(
            "{DECISIONS}/{}: names {date} as its addendum, and the file has {} headings for it",
            decision.file,
            headings.len()
        ));
    }
    let at = headings[0];
    let Some((line, text)) = decision
        .text
        .lines()
        .enumerate()
        .skip(at + 1)
        .find(|(_, line)| !line.trim().is_empty())
    else {
        return Err(format!(
            "{DECISIONS}/{}:{}: the addendum carries no status line",
            decision.file,
            at + 1
        ));
    };
    let line = line + 1;
    if let Some(accepted) = text.strip_prefix("Addendum status: accepted (operator, ") {
        // The heading's date selects the addendum; the status line's own
        // date is read for its form only, and may follow the heading.
        if accepted
            .strip_suffix(')')
            .is_some_and(contract_lifecycle::date_form)
        {
            return Ok(AddendumStatus::Accepted);
        }
    }
    if text.starts_with("Addendum status: proposed") {
        return Ok(AddendumStatus::Proposed);
    }
    if text.starts_with("Addendum status: withdrawn") {
        return Ok(AddendumStatus::Withdrawn);
    }
    Err(format!(
        "{DECISIONS}/{}:{line}: '{text}' is not an addendum status line",
        decision.file
    ))
}

/// The `Keeps the minting of: contracts/<file>` lines in a decision's header.
fn keeps(decision: &Decision) -> Vec<(usize, String)> {
    header(&decision.text)
        .zip(1..)
        .filter_map(|(line, at)| {
            let rest = line.strip_prefix("Keeps the minting of: ")?;
            Some((at, rest.trim().to_string()))
        })
        .collect()
}

/// Whether `from` reaches `target` through its supersession pointers.
fn reaches(ledger: &Ledger, from: u16, target: u16) -> bool {
    let mut seen = BTreeSet::new();
    let mut todo = vec![from];
    while let Some(number) = todo.pop() {
        if !seen.insert(number) {
            continue;
        }
        let Some(decision) = ledger.decision(number) else {
            continue;
        };
        for successor in successors(decision) {
            if successor == target {
                return true;
            }
            todo.push(successor);
        }
    }
    false
}

/// `numbers` as four-digit text, joined by `separator`.
fn numbers(numbers: &[u16], separator: &str) -> String {
    numbers
        .iter()
        .map(|number| format!("{number:04}"))
        .collect::<Vec<_>>()
        .join(separator)
}

/// What ruling 1's walk and keeper chain refuse, in the words the failing
/// test prints. The one home for each refusal's diagnostic text.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Refusal {
    MetTwice {
        file: String,
        number: u16,
    },
    NotInLedger {
        file: String,
        number: u16,
    },
    Proposed {
        file: String,
        number: u16,
    },
    Withdrawn {
        file: String,
        number: u16,
    },
    UnknownStatus {
        file: String,
        number: u16,
        status: String,
    },
    NoSuccessor {
        file: String,
        number: u16,
    },
    NoKeeper {
        file: String,
        old: u16,
        named: Vec<u16>,
    },
    TwoKeepers {
        file: String,
        old: u16,
        keepers: Vec<u16>,
    },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::MetTwice { file, number } => {
                write!(f, "{file}: the supersession walk met {number:04} twice")
            }
            Refusal::NotInLedger { file, number } => {
                write!(
                    f,
                    "{file}: minted under {number:04}, which is not in the ledger"
                )
            }
            Refusal::Proposed { file, number } => write!(
                f,
                "{file}: minted under {number:04}, which is proposed and mints nothing"
            ),
            Refusal::Withdrawn { file, number } => write!(
                f,
                "{file}: minted under {number:04}, which is withdrawn and was never accepted"
            ),
            Refusal::UnknownStatus {
                file,
                number,
                status,
            } => write!(
                f,
                "{file}: minted under {number:04}, whose status '{status}' is not in the ledger's vocabulary"
            ),
            Refusal::NoSuccessor { file, number } => write!(
                f,
                "{file}: minted under {number:04}, which is superseded and names no successor"
            ),
            Refusal::NoKeeper { file, old, named } => {
                let named = numbers(named, ", ");
                write!(
                    f,
                    "{file}: minted under {old:04}, which is superseded by {named}, and {named} does not keep its minting"
                )
            }
            Refusal::TwoKeepers {
                file,
                old,
                keepers,
            } => write!(
                f,
                "{file}: minted under {old:04}, and {} each keep its minting; exactly one may",
                numbers(keepers, " and ")
            ),
        }
    }
}

/// Every decision a supersession pointer names, in the order read, once per
/// occurrence. Nothing here deduplicates: the graph walk's `seen` set is
/// what refuses a number met twice.
fn successors(decision: &Decision) -> Vec<u16> {
    let mut successors = Vec::new();
    for pointer in [Pointer::SupersededBy, Pointer::SupersededInPartBy] {
        if let Some(numbers) = decision.pointers.get(&pointer) {
            successors.extend_from_slice(numbers);
        }
    }
    successors
}

/// Ruling 1's graph walk: every decision the supersession pointers reach,
/// with one `seen` set for the whole walk. A number met twice refuses,
/// whether a cycle, two branches meeting at one decision, or one successor
/// named twice; a pointer naming no decision in the ledger refuses by the
/// default.
fn graph_walk(
    ledger: &Ledger,
    number: u16,
    file: &str,
    seen: &mut BTreeSet<u16>,
) -> Result<(), Refusal> {
    if !seen.insert(number) {
        return Err(Refusal::MetTwice {
            file: file.to_string(),
            number,
        });
    }
    let decision = ledger
        .decision(number)
        .ok_or_else(|| Refusal::NotInLedger {
            file: file.to_string(),
            number,
        })?;
    for successor in successors(decision) {
        graph_walk(ledger, successor, file, seen)?;
    }
    Ok(())
}

/// Ruling 1's keeper chain: from the minting decision, while it is
/// superseded, exactly one successor must keep the file; move to that
/// keeper, and end at an accepted decision nothing supersedes. A branch
/// the chain does not take is walked for repeats only and is never asked
/// to keep the file.
fn keeper_chain(ledger: &Ledger, number: u16, file: &str) -> Result<(), Refusal> {
    let mut current = number;
    loop {
        let decision = ledger
            .decision(current)
            .ok_or_else(|| Refusal::NotInLedger {
                file: file.to_string(),
                number: current,
            })?;
        match Status::parse(&decision.status) {
            Some(Status::Accepted | Status::Superseded) => {}
            Some(Status::Proposed) => {
                return Err(Refusal::Proposed {
                    file: file.to_string(),
                    number: current,
                })
            }
            Some(Status::Withdrawn) => {
                return Err(Refusal::Withdrawn {
                    file: file.to_string(),
                    number: current,
                })
            }
            None => {
                return Err(Refusal::UnknownStatus {
                    file: file.to_string(),
                    number: current,
                    status: decision.status.clone(),
                })
            }
        }
        let pointed = successors(decision);
        if pointed.is_empty() {
            return if Status::parse(&decision.status) == Some(Status::Accepted) {
                Ok(())
            } else {
                Err(Refusal::NoSuccessor {
                    file: file.to_string(),
                    number: current,
                })
            };
        }
        let keepers: Vec<u16> = pointed
            .iter()
            .copied()
            .filter(|successor| {
                ledger
                    .decision(*successor)
                    .is_some_and(|decision| keeps(decision).iter().any(|(_, kept)| kept == file))
            })
            .collect();
        match keepers.as_slice() {
            [one] => current = *one,
            [] => {
                return Err(Refusal::NoKeeper {
                    file: file.to_string(),
                    old: current,
                    named: pointed,
                });
            }
            many => {
                return Err(Refusal::TwoKeepers {
                    file: file.to_string(),
                    old: current,
                    keepers: many.to_vec(),
                });
            }
        }
    }
}

/// Whether `number` mints `file`: one graph walk for repeats, then the
/// keeper chain for the file.
fn admits(
    ledger: &Ledger,
    number: u16,
    file: &str,
    seen: &mut BTreeSet<u16>,
) -> Result<(), Refusal> {
    graph_walk(ledger, number, file, seen)?;
    keeper_chain(ledger, number, file)
}

/// What a listed row's recorded authority reads today.
enum Listed {
    /// The cell names no decision: `no decision (PR #28)` or an operator
    /// ruling held in the pull request.
    NoDecision,
    Status(Status),
    Unreadable(String),
    Missing(u16),
}

/// Read the ruling a listed cell names, from the cell as recorded.
fn listed_authority(ledger: &Ledger, cell: &str) -> Listed {
    let first = cell.split([',', ' ']).next().unwrap_or_default();
    let Some(number) = contract_lifecycle::decision_number(first) else {
        return Listed::NoDecision;
    };
    let Some(decision) = ledger.decision(number) else {
        return Listed::Missing(number);
    };
    let addendum = cell
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once(' '))
        .map(|(date, _)| date.to_string())
        .filter(|date| contract_lifecycle::date_form(date));
    match addendum {
        Some(date) => match read_addendum(decision, &date) {
            Ok(AddendumStatus::Accepted) => Listed::Status(Status::Accepted),
            Ok(AddendumStatus::Proposed) => Listed::Status(Status::Proposed),
            Ok(AddendumStatus::Withdrawn) => Listed::Status(Status::Withdrawn),
            Err(why) => Listed::Unreadable(why),
        },
        None => match Status::parse(&decision.status) {
            Some(status) => Listed::Status(status),
            None => Listed::Unreadable(format!(
                "{DECISIONS}/{}: status '{}' is not in the ledger's vocabulary",
                decision.file, decision.status
            )),
        },
    }
}

/// The recorded cell with ruling 2's marker replaced by the withdrawn word.
fn withdrawn_cell(recorded: &str) -> String {
    recorded.replace(", enacted, pending acceptance", ", withdrawn")
}

/// The decision number a row names, for the unused-keep check.
fn authority_number(row: &contract_lifecycle::Row) -> Option<u16> {
    match contract_lifecycle::minted_form(&row.minted)? {
        Minted::Decision { number, .. } => Some(number),
        Minted::Withdrawn(_) => {
            let (_, recorded) = PENDING.iter().find(|(file, _)| *file == row.file)?;
            let first = recorded.split([',', ' ']).next().unwrap_or_default();
            contract_lifecycle::decision_number(first)
        }
        Minted::Listed(_) => None,
    }
}

/// Check one row's `Minted under` cell against the rule.
fn minted_row(ledger: &Ledger, row: &contract_lifecycle::Row) -> Vec<String> {
    let file = &row.file;
    let Some(minted) = contract_lifecycle::minted_form(&row.minted) else {
        return vec![format!(
            "{file}: the Minted under cell '{}' is not a closed form",
            row.minted
        )];
    };
    let listed = PENDING
        .iter()
        .find(|(pending, _)| *pending == file.as_str());
    match minted {
        Minted::Listed(cell) => match listed {
            Some((_, recorded)) if *recorded == cell => {
                match listed_authority(ledger, &cell) {
                    Listed::NoDecision | Listed::Status(Status::Proposed) => Vec::new(),
                    Listed::Status(status) => vec![format!(
                        "{file}: minted under '{cell}', and its ruling now reads '{}'; the marker drops",
                        status.word()
                    )],
                    Listed::Unreadable(why) => vec![format!("{file}: {why}")],
                    Listed::Missing(number) => vec![format!(
                        "{file}: minted under '{cell}', and {number:04} is not in the ledger"
                    )],
                }
            }
            _ => vec![format!(
                "{file}: minted under '{cell}', which is outside ruling 2's closed list"
            )],
        },
        Minted::Withdrawn(cell) => match listed {
            Some((_, recorded)) if withdrawn_cell(recorded) == cell => {
                match listed_authority(ledger, recorded) {
                    Listed::Status(Status::Withdrawn) => Vec::new(),
                    Listed::Status(status) => vec![format!(
                        "{file}: reads '{cell}', and its ruling reads '{}'; only a withdrawn ruling admits it",
                        status.word()
                    )],
                    Listed::NoDecision => vec![format!(
                        "{file}: reads '{cell}', and a cell naming no decision has no withdrawn form"
                    )],
                    Listed::Unreadable(why) => vec![format!("{file}: {why}")],
                    Listed::Missing(number) => vec![format!(
                        "{file}: reads '{cell}', and {number:04} is not in the ledger"
                    )],
                }
            }
            _ => vec![format!(
                "{file}: the withdrawn form is only a listed file's recorded cell with the marker replaced"
            )],
        },
        Minted::Decision { number, addendum } => {
            let mut findings = Vec::new();
            if let Some((_, recorded)) = listed {
                match listed_authority(ledger, recorded) {
                    Listed::Status(Status::Proposed) => {
                        findings.push(format!(
                            "{file}: is on ruling 2's list and must carry '{recorded}' while {number:04} is proposed"
                        ));
                        return findings;
                    }
                    Listed::Status(Status::Withdrawn) => {
                        findings.push(format!(
                            "{file}: is on ruling 2's list and must read '{}'",
                            withdrawn_cell(recorded)
                        ));
                        return findings;
                    }
                    Listed::Unreadable(why) => findings.push(format!("{file}: {why}")),
                    _ => {}
                }
            }
            if let Some(date) = addendum.as_deref() {
                let Some(decision) = ledger.decision(number) else {
                    return vec![Refusal::NotInLedger {
                        file: file.clone(),
                        number,
                    }
                    .to_string()];
                };
                match read_addendum(decision, date) {
                    Ok(AddendumStatus::Accepted) => {}
                    Ok(AddendumStatus::Proposed) => findings.push(format!(
                        "{file}: minted under {number:04} ({date} addendum), which is proposed and mints nothing"
                    )),
                    Ok(AddendumStatus::Withdrawn) => findings.push(format!(
                        "{file}: minted under {number:04} ({date} addendum), which is withdrawn and was never accepted"
                    )),
                    Err(why) => findings.push(format!("{file}: {why}")),
                }
            }
            let mut seen = BTreeSet::new();
            if let Err(refusal) = admits(ledger, number, file, &mut seen) {
                findings.push(refusal.to_string());
            }
            findings
        }
    }
}

/// A keep line no row walks through is refused as unused.
fn unused_keep_lines(ledger: &Ledger, rows: &[contract_lifecycle::Row]) -> Vec<String> {
    let mut findings = Vec::new();
    for decision in &ledger.decisions {
        for (at, file) in keeps(decision) {
            let used = rows.iter().any(|row| {
                row.file == file
                    && authority_number(row)
                        .is_some_and(|number| reaches(ledger, number, decision.number))
            });
            if !used {
                findings.push(format!(
                    "{DECISIONS}/{}:{at}: {:04} keeps the minting of {file}, which no row mints under a decision it supersedes",
                    decision.file, decision.number
                ));
            }
        }
    }
    findings
}

/// Every row's `Minted under` cell, and every keep line, against the ledger.
pub(super) fn check() {
    let ledger = Ledger::read();
    let rows = contract_lifecycle::rows();
    assert!(rows.len() > 40, "the table was not read");
    let mut findings = Vec::new();
    for row in &rows {
        findings.extend(minted_row(&ledger, row));
    }
    findings.extend(unused_keep_lines(&ledger, &rows));
    assert_eq!(
        findings,
        Vec::<String>::new(),
        "a contract version is not minted under an accepted ruling"
    );
}

/// A decision the minting rule can walk, with `header` above its body.
fn fixture(number: u16, status: &str, pointers: &[(Pointer, &[u16])], header: &str) -> Decision {
    Decision {
        number,
        file: format!("{number:04}-fixture.md"),
        status: status.to_string(),
        built: None,
        pointers: pointers
            .iter()
            .map(|(pointer, numbers)| (*pointer, numbers.to_vec()))
            .collect(),
        lines: BTreeMap::new(),
        text: format!("{header}\n\n## Body\n"),
    }
}

/// The file every fixture in this module mints.
const FILE: &str = "contracts/chief.schema.json";

/// The row's minting decision 0001, superseded by the keeper 0002 and the
/// non-keeper 0003.
fn first() -> Decision {
    fixture(
        1,
        "superseded",
        &[(Pointer::SupersededBy, &[2, 3])],
        "# 0001",
    )
}

/// A decision that keeps the fixture file, with its own status and
/// supersession pointer.
fn keeper(number: u16, status: &str, next: &[u16]) -> Decision {
    let pointers: Vec<(Pointer, &[u16])> = if next.is_empty() {
        Vec::new()
    } else {
        vec![(Pointer::SupersededBy, next)]
    };
    fixture(
        number,
        status,
        &pointers,
        &format!("# {number:04}\nKeeps the minting of: {FILE}"),
    )
}

/// The non-keeper 0003, superseded by `next` and keeping nothing.
fn non_keeper(next: u16) -> Decision {
    fixture(
        3,
        "superseded",
        &[(Pointer::SupersededBy, &[next])],
        "# 0003",
    )
}

/// A ledger of fixtures, with no rows, gaps or reading findings.
fn fixture_ledger(decisions: Vec<Decision>) -> Ledger {
    Ledger {
        decisions,
        rows: Vec::new(),
        gaps: Vec::new(),
        read: Vec::new(),
    }
}

/// Assert that the row minted under 0001 refuses `number` met twice.
fn refuses_a_number_met_twice(ledger: &Ledger, number: u16) {
    let mut seen = BTreeSet::new();
    let refused = admits(ledger, 1, FILE, &mut seen).expect_err("the walk must meet it twice");
    assert_eq!(
        refused,
        Refusal::MetTwice {
            file: FILE.to_string(),
            number
        }
    );
}

/// 0082 ruling 1: a supersession walk meets every decision a pointer
/// names. 0001 names 0002, which keeps the file, and 0003, which cycles
/// back to 0001; walking only the keeper would admit it.
#[test]
fn chief_non_keeper_cycle_must_refuse() {
    let ledger = fixture_ledger(vec![first(), keeper(2, "accepted", &[]), non_keeper(1)]);
    let mut seen = BTreeSet::new();
    let refused = admits(&ledger, 1, FILE, &mut seen).expect_err("the walk must meet 0001 twice");
    assert_eq!(
        refused,
        Refusal::MetTwice {
            file: FILE.to_string(),
            number: 1
        }
    );
}

/// 0082 ruling 1: one walk carries one `seen` set. 0001 names 0002, which
/// keeps the file, and 0003, which does not; both point to accepted 0004,
/// which keeps it. Two branches meeting at one decision are a number met
/// twice, so the row refuses even though a keeper chain exists.
#[test]
fn review_shared_successor_must_refuse_twice() {
    let ledger = fixture_ledger(vec![
        first(),
        keeper(2, "superseded", &[4]),
        non_keeper(4),
        keeper(4, "accepted", &[]),
    ]);
    let mut seen = BTreeSet::new();
    let refused = admits(&ledger, 1, FILE, &mut seen).expect_err("both branches must meet 0004");
    assert_eq!(
        refused,
        Refusal::MetTwice {
            file: FILE.to_string(),
            number: 4
        }
    );
}

/// 0082 ruling 1: a successor named twice on one pointer line is a number
/// met twice. 0001 is superseded by 0002, keeper of the file, and reads
/// `Superseded by: 0002, 0002`; the walk must count both occurrences.
#[test]
fn a_successor_named_twice_on_one_line_refuses() {
    let ledger = fixture_ledger(vec![
        fixture(
            1,
            "superseded",
            &[(Pointer::SupersededBy, &[2, 2])],
            "# 0001",
        ),
        keeper(2, "accepted", &[]),
    ]);
    refuses_a_number_met_twice(&ledger, 2);
}

/// 0082 ruling 1: a successor named by both pointer keys is a number met
/// twice. 0001 is superseded by 0002, keeper of the file, and reads both
/// `Superseded by: 0002` and `Superseded in part by: 0002`.
#[test]
fn a_successor_named_by_both_pointer_keys_refuses() {
    let ledger = fixture_ledger(vec![
        fixture(
            1,
            "superseded",
            &[
                (Pointer::SupersededBy, &[2]),
                (Pointer::SupersededInPartBy, &[2]),
            ],
            "# 0001",
        ),
        keeper(2, "accepted", &[]),
    ]);
    refuses_a_number_met_twice(&ledger, 2);
}

/// 0082 ruling 1: each walk refusal keeps the diagnostic words the ledger
/// printed before it was typed, pinned here once so a variant's text can
/// change only with this test.
#[test]
fn walk_refusals_display_the_ledgers_words() {
    let file = || FILE.to_string();
    let cases = [
        (
            Refusal::MetTwice { file: file(), number: 2 },
            format!("{FILE}: the supersession walk met 0002 twice"),
        ),
        (
            Refusal::NotInLedger { file: file(), number: 2 },
            format!("{FILE}: minted under 0002, which is not in the ledger"),
        ),
        (
            Refusal::Proposed { file: file(), number: 2 },
            format!("{FILE}: minted under 0002, which is proposed and mints nothing"),
        ),
        (
            Refusal::Withdrawn { file: file(), number: 2 },
            format!("{FILE}: minted under 0002, which is withdrawn and was never accepted"),
        ),
        (
            Refusal::UnknownStatus {
                file: file(),
                number: 2,
                status: "landed".to_string(),
            },
            format!(
                "{FILE}: minted under 0002, whose status 'landed' is not in the ledger's vocabulary"
            ),
        ),
        (
            Refusal::NoSuccessor { file: file(), number: 2 },
            format!("{FILE}: minted under 0002, which is superseded and names no successor"),
        ),
        (
            Refusal::NoKeeper {
                file: file(),
                old: 1,
                named: vec![2],
            },
            format!(
                "{FILE}: minted under 0001, which is superseded by 0002, and 0002 does not keep its minting"
            ),
        ),
        (
            Refusal::TwoKeepers {
                file: file(),
                old: 1,
                keepers: vec![2, 3],
            },
            format!(
                "{FILE}: minted under 0001, and 0002 and 0003 each keep its minting; exactly one may"
            ),
        ),
    ];
    for (refusal, words) in cases {
        assert_eq!(refusal.to_string(), words);
    }
}

/// 0082 ruling 1 and Ruled item 9: the keeper chain asks the keeper branch
/// for the file and visits a non-keeper branch for repeats only. 0001
/// names the sole keeper 0002, accepted and superseded by nothing, and the
/// non-keeper 0003, which points to accepted 0004; neither 0003 nor 0004
/// keeps the file, and the row admits.
#[test]
fn non_keeper_branch_needs_no_minting_for_unrelated_file() {
    let ledger = fixture_ledger(vec![
        first(),
        keeper(2, "accepted", &[]),
        non_keeper(4),
        fixture(4, "accepted", &[], "# 0004"),
    ]);
    let mut seen = BTreeSet::new();
    assert_eq!(admits(&ledger, 1, FILE, &mut seen), Ok(()));
}

/// 0082 ruling 1: the heading's date selects the addendum, and the status
/// line's date is read for its `YYYY-MM-DD` form only, so an acceptance
/// dated after the heading admits.
#[test]
fn chief_later_addendum_acceptance_date_must_admit() {
    let decision = fixture(
        42,
        "accepted",
        &[],
        "## Addendum — 2026-09-04\nAddendum status: accepted (operator, 2026-09-06)",
    );
    assert_eq!(
        read_addendum(&decision, "2026-09-04"),
        Ok(AddendumStatus::Accepted)
    );
}
