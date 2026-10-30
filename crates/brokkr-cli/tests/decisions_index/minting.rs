//! Decision 0082 rulings 1 and 2, beside the ledger they read: a contract
//! version is minted only under an accepted ruling, an addendum by its own
//! exact status line, and a superseded ruling keeps a minting only when
//! exactly one successor its pointer names carries the keep line. The files
//! already minted on pending or absent authority stand, marked, on a closed
//! list.
//!
//! The cell's grammar lives once, in `contract_lifecycle::minted_form`; this
//! module reads the ruling a cell names.

use std::collections::BTreeSet;

use super::{header, Decision, Ledger, Pointer, Status, DECISIONS};
use crate::contract_lifecycle::{self, Minted};

/// The files decision 0082 ruling 2 records as standing without an accepted
/// ruling, with the exact cell each row writes. The list only shrinks.
///
/// `realms.v7` left the list when decision 0075 was accepted on 2026-10-12.
/// `panel-carry.v1` is the one file 0082 did not see: it landed on `main`
/// on 2026-10-21 under proposed decision 0093, after the ruling was
/// accepted, and it is the ninth pending row until the operator rules 0093.
/// The build's pull request names it.
const PENDING: [(&str, &str); 9] = [
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
    (
        "contracts/panel-carry.v1.schema.json",
        "0093, enacted, pending acceptance",
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
    if text == format!("Addendum status: accepted (operator, {date})") {
        return Ok(AddendumStatus::Accepted);
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
        for pointer in [Pointer::SupersededBy, Pointer::SupersededInPartBy] {
            if let Some(numbers) = decision.pointers.get(&pointer) {
                if numbers.contains(&target) {
                    return true;
                }
                todo.extend(numbers);
            }
        }
    }
    false
}

/// The exact refusal ruling 1 gives when a successor keeps no minting.
fn not_kept(file: &str, old: u16, new: &str) -> String {
    format!("{file}: minted under {old:04}, which is superseded by {new}, and {new} does not keep its minting")
}

/// Whether `number` mints `file`, walking every supersession pointer.
fn admits(
    ledger: &Ledger,
    number: u16,
    file: &str,
    seen: &mut BTreeSet<u16>,
) -> Result<(), String> {
    if !seen.insert(number) {
        return Err(format!(
            "{file}: the supersession walk met {number:04} twice"
        ));
    }
    let decision = ledger
        .decision(number)
        .ok_or_else(|| format!("{file}: minted under {number:04}, which is not in the ledger"))?;
    match Status::parse(&decision.status) {
        Some(Status::Accepted | Status::Superseded) => {}
        Some(Status::Proposed) => {
            return Err(format!(
                "{file}: minted under {number:04}, which is proposed and mints nothing"
            ))
        }
        Some(Status::Withdrawn) => {
            return Err(format!(
                "{file}: minted under {number:04}, which is withdrawn and was never accepted"
            ))
        }
        None => {
            return Err(format!(
            "{file}: minted under {number:04}, whose status '{}' is not in the ledger's vocabulary",
            decision.status
        ))
        }
    }
    let mut successors: BTreeSet<u16> = BTreeSet::new();
    for pointer in [Pointer::SupersededBy, Pointer::SupersededInPartBy] {
        if let Some(numbers) = decision.pointers.get(&pointer) {
            successors.extend(numbers);
        }
    }
    if successors.is_empty() {
        return if Status::parse(&decision.status) == Some(Status::Accepted) {
            Ok(())
        } else {
            Err(format!(
                "{file}: minted under {number:04}, which is superseded and names no successor"
            ))
        };
    }
    let keepers: Vec<u16> = successors
        .iter()
        .copied()
        .filter(|successor| {
            ledger
                .decision(*successor)
                .is_some_and(|decision| keeps(decision).iter().any(|(_, kept)| kept == file))
        })
        .collect();
    match keepers.as_slice() {
        [one] => admits(ledger, *one, file, seen),
        [] => {
            let named = successors
                .iter()
                .map(|number| format!("{number:04}"))
                .collect::<Vec<_>>()
                .join(", ");
            Err(not_kept(file, number, &named))
        }
        many => Err(format!(
            "{file}: minted under {number:04}, and {} each keep its minting; exactly one may",
            many.iter()
                .map(|number| format!("{number:04}"))
                .collect::<Vec<_>>()
                .join(" and ")
        )),
    }
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
                    return vec![format!(
                        "{file}: minted under {number:04}, which is not in the ledger"
                    )];
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
            if let Err(finding) = admits(ledger, number, file, &mut seen) {
                findings.push(finding);
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
