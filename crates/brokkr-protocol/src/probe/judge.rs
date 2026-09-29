//! What the facts imply: decision 0075 ruling 4's seat eligibility, the
//! adapter's declared fields beside the ones the facts imply, and the
//! drift since the report this run replaces. Pure.

use std::collections::BTreeSet;

use super::facts::{Agreement, DriftRow, Eligibility, Fact, Facts, FieldRow, Report, Verdict};
use super::Declared;

/// The status a CLI gives an MCP server it started and reached.
const CONNECTED: &str = "connected";

/// A fact read as settled yes or no: a measured value tested, a refused
/// mechanism a no, and an unmeasured fact not settled at all.
fn settled<T: serde::Serialize>(fact: &Fact<T>, test: impl FnOnce(&T) -> bool) -> Option<bool> {
    match fact {
        Fact::Measured { value, .. } => Some(test(value)),
        Fact::Unsupported { .. } => Some(false),
        Fact::Unmeasured { .. } => None,
    }
}

/// Whether the CLI can stand behind the box (decisions 0043 and 0046):
/// its own tools switched off, and the hands MCP server reached.
fn boxable(facts: &Facts) -> Option<bool> {
    let emptied = settled(&facts.boxed_tools, Vec::is_empty)?;
    let served = settled(&facts.mcp_server, |status| status == CONNECTED)?;
    Some(emptied && served)
}

/// Ruling 4, in order: a harness not shown to keep a user-scope MCP
/// server out of its turn (#467) is refused; one that empties its own tools and reaches an
/// MCP server may hold boxed offices; one whose native egress is absent or
/// switched off may hold unboxed offices; and one whose egress has no
/// measured off switch may hold only offices that give it no tools
/// (decision 0065 ruling 4). An unmeasured fact never admits more. An
/// office outside the box launches the plain turn, so a plain turn not
/// shown to keep the planted server out holds a boxable harness to boxed
/// offices and refuses any other.
pub(crate) fn eligibility(facts: &Facts) -> Eligibility {
    let (verdict, reason) = if facts.config_isolation.value() != Some(&true) {
        (
            Verdict::Refused,
            format!(
                "its user-scope configuration is not shown to be isolated: {}",
                facts.config_isolation.account()
            ),
        )
    } else if boxable(facts) == Some(true) {
        let reason = "its own tools switch off and the hands MCP server connects";
        match plain_leak(facts) {
            None => (Verdict::Boxed, reason.to_string()),
            Some(leak) => (
                Verdict::BoxedOnly,
                format!("{reason}, but {leak}, so it may hold boxed offices only"),
            ),
        }
    } else {
        outside_the_box(facts)
    };
    Eligibility { verdict, reason }
}

/// The rungs below the box: each launches the plain turn, so a plain
/// turn not shown to keep the planted server out refuses the harness.
fn outside_the_box(facts: &Facts) -> (Verdict, String) {
    let (verdict, reason) = if facts.egress_off.value() == Some(&true) {
        (
            Verdict::UnboxedOnly,
            format!(
                "it is not shown to stand behind the box ({}; {}), and {}",
                facts.boxed_tools.account(),
                facts.mcp_server.account(),
                facts.egress_off.account()
            ),
        )
    } else {
        (
            Verdict::ToolLessOnly,
            format!(
                "its native egress has no measured off switch: {}",
                facts.egress_off.account()
            ),
        )
    };
    match plain_leak(facts) {
        None => (verdict, reason),
        Some(leak) => (
            Verdict::Refused,
            format!("{leak}, and it may hold no boxed office: {reason}"),
        ),
    }
}

/// How the plain turn, the launch an office outside the box uses, failed
/// to show the planted user-scope server kept out (#467); `None` when it
/// listed its servers without it.
fn plain_leak(facts: &Facts) -> Option<String> {
    let account = facts.user_mcp_unboxed.account();
    let outside = "its plain turn, the launch an office outside the box uses,";
    match facts.user_mcp_unboxed.value() {
        Some(false) => None,
        Some(true) => Some(format!(
            "{outside} loaded the planted user-scope MCP server (#467): {account}"
        )),
        None => Some(format!(
            "{outside} is not shown to keep the planted user-scope MCP server out (#467): \
             {account}"
        )),
    }
}

fn row(field: String, declared: String, implied: Option<(String, bool)>) -> FieldRow {
    let (implied, agreement) = match implied {
        Some((value, true)) => (value, Agreement::Agrees),
        Some((value, false)) => (value, Agreement::Differs),
        None => ("unmeasured".to_string(), Agreement::NotCompared),
    };
    FieldRow {
        field,
        declared,
        implied,
        agreement,
    }
}

fn efforts_row(declared: &Declared, efforts: &Fact<Vec<String>>) -> FieldRow {
    let implied = efforts.value().map(|levels| {
        let same = levels.iter().collect::<BTreeSet<_>>()
            == declared.efforts.iter().collect::<BTreeSet<_>>();
        (levels.join(", "), same)
    });
    row("efforts".to_string(), declared.efforts.join(", "), implied)
}

fn hands_row(declared: &Declared, facts: &Facts) -> FieldRow {
    let word = |supported: bool| {
        if supported {
            "supported"
        } else {
            "unsupported"
        }
    };
    let declared_hands = declared.hands.is_some();
    let implied =
        boxable(facts).map(|boxable| (word(boxable).to_string(), boxable == declared_hands));
    row(
        "hands".to_string(),
        word(declared_hands).to_string(),
        implied,
    )
}

/// A resume shape's measured identity beside the version the CLI reports.
fn version_row(shape: &str, declared: &str, version: &Fact<String>) -> FieldRow {
    let implied = version.value().map(|reported| {
        let same = reported
            .split(|c: char| c.is_whitespace() || "()/".contains(c))
            .any(|token| token == declared);
        (reported.clone(), same)
    });
    row(
        format!("resume.{shape}.identity.version"),
        declared.to_string(),
        implied,
    )
}

/// Each adapter field the probe can speak to, beside what it implies.
pub(crate) fn adapter_fields(
    declared: &Declared,
    facts: &Facts,
    version: &Fact<String>,
) -> Vec<FieldRow> {
    let mut rows = vec![
        efforts_row(declared, &facts.efforts),
        hands_row(declared, facts),
    ];
    rows.extend(
        declared
            .resume_versions
            .iter()
            .map(|(shape, identity)| version_row(shape, identity, version)),
    );
    rows
}

fn readings(report: &Report) -> Vec<(&'static str, String)> {
    let mut readings = vec![("cli.version", report.cli.version.reading())];
    readings.extend(report.facts.readings());
    readings.push((
        "eligibility",
        serde_json::to_string(&report.eligibility.verdict).unwrap_or_default(),
    ));
    readings
}

/// Every reading that moved between two reports of one harness.
pub(crate) fn drift(previous: &Report, current: &Report) -> Vec<DriftRow> {
    readings(previous)
        .into_iter()
        .zip(readings(current))
        .filter(|((_, before), (_, after))| before != after)
        .map(|((fact, before), (_, after))| DriftRow {
            fact: fact.to_string(),
            before,
            after,
        })
        .collect()
}
