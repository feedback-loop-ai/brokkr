//! What the facts imply: decision 0075 ruling 4's seat eligibility, the
//! adapter's declared fields beside the ones the facts imply, and the
//! drift since the report this run replaces. Pure.

use std::collections::BTreeSet;

use super::facts::{
    Agreement, Capability, DriftRow, Eligibility, Fact, Facts, FieldRow, Report, Verdict,
};
use super::measure::Unread;
use super::{Declared, DeclaredOff, Native, NativePower};

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

/// Ruling 4's verdict, admitted only on complete evidence (#484): a
/// verdict that admits the harness anywhere stands only when every line
/// both turns captured was read, on stdout, in a transcript or on stderr
/// and however the turn exited, none of them undecoded, cut short or
/// joined; every value their listings hold was named; and every fact that
/// verdict rests on was measured. A fact the CLI refused counts as
/// measured, its refusal being what the probe read; an unmeasured one
/// never does. Otherwise the harness is refused, and the reason names each
/// unread line, each unread value and each unmeasured fact.
pub(crate) fn eligibility(facts: &Facts, unread: &[Unread]) -> Eligibility {
    let proposed = verdict(facts);
    let Some((admits, unmeasured)) = rests_on(proposed.verdict, facts) else {
        return proposed;
    };
    let gaps: Vec<String> = unread
        .iter()
        .map(ToString::to_string)
        .chain(unmeasured)
        .collect();
    if gaps.is_empty() {
        return proposed;
    }
    Eligibility {
        verdict: Verdict::Refused,
        reason: format!(
            "the evidence for {admits} is not complete: {}",
            gaps.join("; ")
        ),
    }
}

/// `fact`, named `name`, when it is unmeasured: why, under its name.
fn gap<T: serde::Serialize>(name: &str, fact: &Fact<T>) -> Option<String> {
    match fact {
        Fact::Unmeasured { why } => Some(format!("{name} is unmeasured: {why}")),
        Fact::Measured { .. } | Fact::Unsupported { .. } => None,
    }
}

/// What a verdict admits the harness to, and each fact it rests on that
/// is unmeasured; `None` for a refusal, which admits nothing. The box
/// rests on the plain turn's tool inventory too, since its own tools are
/// shown switched off only against the tools it has; a seat outside the
/// box rests on each native capability's off switch, a grant on those it
/// lacks and an unboxed office on those it has.
fn rests_on(verdict: Verdict, facts: &Facts) -> Option<(&'static str, Vec<String>)> {
    let plain = || gap("user_mcp_unboxed", &facts.user_mcp_unboxed);
    let boxed_tools = || gap("boxed_tools", &facts.boxed_tools);
    let server = || gap("mcp_server", &facts.mcp_server);
    let switches = || {
        let each = facts.capabilities.value().into_iter().flatten();
        let offs = each.map(|capability| {
            let name = format!("{}'s off switch", capability.capability);
            gap(&name, &capability.off)
        });
        let listed = gap("capabilities", &facts.capabilities);
        std::iter::once(listed).chain(offs).collect::<Vec<_>>()
    };
    let (admits, rested_on) = match verdict {
        Verdict::Refused => return None,
        Verdict::Boxed => ("boxed offices", vec![plain(), boxed_tools(), server()]),
        Verdict::BoxedOnly => ("boxed offices only", vec![boxed_tools(), server()]),
        Verdict::GrantingRealmsOnly => (
            "a seat in a realm that grants its capabilities",
            [vec![plain()], switches()].concat(),
        ),
        Verdict::UnboxedOnly => {
            let egress = gap("native_egress", &facts.native_egress);
            let off = gap("egress_off", &facts.egress_off);
            (
                "unboxed offices",
                [vec![plain(), egress, off], switches()].concat(),
            )
        }
    };
    let isolated = gap("config_isolation", &facts.config_isolation);
    let inventory = gap("tools", &facts.tools);
    let unmeasured = [isolated, inventory].into_iter().chain(rested_on);
    Some((admits, unmeasured.flatten().collect()))
}

/// Ruling 4, in order: a harness not shown to keep a user-scope MCP
/// server out of its turn (#467) is refused; one that empties its own
/// tools and reaches an MCP server may hold boxed offices; one with a
/// native capability whose off switch is absent or unread may be seated
/// only in a realm that grants it (operator ruling A, 2026-09-29, reading
/// decision 0065 ruling 4), whatever its egress reads; and one whose
/// native egress is absent or switched off may hold unboxed offices. An
/// unmeasured fact never admits more, and [`eligibility`] admits what
/// this proposes only on complete evidence, so an off switch left unread
/// refuses the harness. An office outside the box launches
/// the plain turn, so a plain turn not shown to keep the planted server
/// out holds a boxable harness to boxed offices (operator ruling B,
/// 2026-09-29) and refuses any other.
fn verdict(facts: &Facts) -> Eligibility {
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
/// turn not shown to keep the planted server out refuses the harness. A
/// native capability without a measured off switch demands a grant
/// whatever the egress reading says.
fn outside_the_box(facts: &Facts) -> (Verdict, String) {
    let (verdict, reason) = if let Some(ungranted) = without_off_switch(facts) {
        (
            Verdict::GrantingRealmsOnly,
            format!(
                "{ungranted}, so it may be seated only in a realm that grants them (decision \
                 0065 ruling 4)"
            ),
        )
    } else if facts.egress_off.value() == Some(&true) {
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
            Verdict::Refused,
            format!(
                "its native egress has no measured off switch ({}), and no native capability \
                 is named for a realm to grant: {}",
                facts.egress_off.account(),
                facts.capabilities.account()
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

/// The native capabilities without a measured off switch, those measured
/// to have none named apart from those whose off switch was not read,
/// when the plain turn's were read and at least one lacks one.
fn without_off_switch(facts: &Facts) -> Option<String> {
    let mut absent = Vec::new();
    let mut unread = Vec::new();
    for capability in facts.capabilities.value()? {
        match capability.off {
            Fact::Measured { .. } => {}
            Fact::Unsupported { .. } => absent.push(capability.capability.as_str()),
            Fact::Unmeasured { .. } => unread.push(capability.capability.as_str()),
        }
    }
    let mut named = Vec::new();
    if !absent.is_empty() {
        named.push(format!(
            "no off switch exists for its native capabilities {}",
            absent.join(", ")
        ));
    }
    if !unread.is_empty() {
        named.push(format!(
            "the off switch was not read for its native capabilities {}",
            unread.join(", ")
        ));
    }
    Some(named.join(", and ")).filter(|named| !named.is_empty())
}

/// How the plain turn, the launch an office outside the box uses, failed
/// to show the planted user-scope server kept out (#467); `None` when it
/// showed no MCP server reached it.
fn plain_leak(facts: &Facts) -> Option<String> {
    let account = facts.user_mcp_unboxed.account();
    let outside = "its plain turn, the launch an office outside the box uses,";
    match facts.user_mcp_unboxed.value() {
        Some(false) => None,
        Some(true) => Some(format!(
            "{outside} loaded an MCP server the probe did not give it (#467): {account}"
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

fn joined(tools: &BTreeSet<&str>) -> String {
    tools.iter().copied().collect::<Vec<_>>().join(", ")
}

/// The tools the declared native capabilities map, beside the plain
/// turn's native egress tools (decision 0065 ruling 4).
fn inventory_row(native: &Native, egress: &Fact<Vec<String>>) -> FieldRow {
    let declared: Option<BTreeSet<&str>> = match native {
        Native::Known { powers, .. } => Some(
            powers
                .iter()
                .flat_map(|power| &power.tools)
                .map(String::as_str)
                .collect(),
        ),
        Native::Unmeasured(_) => None,
    };
    let implied = egress.value().map(|listed| {
        let listed: BTreeSet<&str> = listed.iter().map(String::as_str).collect();
        (joined(&listed), declared.as_ref() == Some(&listed))
    });
    let shown = match &declared {
        Some(tools) => joined(tools),
        None => "unmeasured".to_string(),
    };
    row("native_capabilities.tools".to_string(), shown, implied)
}

/// One power's declared OFF beside what the probe measured switching it
/// off, compared only where the probe read a switch or its absence.
fn off_row(power: &NativePower, off: Option<&Fact<Vec<String>>>) -> FieldRow {
    let declared = match power.off {
        DeclaredOff::Switched => "switched off",
        DeclaredOff::Unsupported => "unsupported",
        DeclaredOff::Unmeasured => "unmeasured",
    };
    let implied = match off {
        Some(Fact::Measured { value, .. }) => Some((
            format!("switched off by {}", value.join(" ")),
            power.off == DeclaredOff::Switched,
        )),
        Some(Fact::Unsupported { .. }) => Some((
            "unsupported".to_string(),
            power.off == DeclaredOff::Unsupported,
        )),
        Some(Fact::Unmeasured { .. }) | None => None,
    };
    row(
        format!("native_capabilities.known.{}.off", power.key),
        declared.to_string(),
        implied,
    )
}

/// Each declared power's OFF row, read against the capability measured
/// for it: `tools::capabilities` measures them in the declared order.
fn off_rows(native: &Native, capabilities: &Fact<Vec<Capability>>) -> Vec<FieldRow> {
    let Native::Known { powers, .. } = native else {
        return Vec::new();
    };
    let measured = capabilities.value();
    powers
        .iter()
        .enumerate()
        .map(|(index, power)| {
            let off = measured
                .and_then(|each| each.get(index))
                .map(|each| &each.off);
            off_row(power, off)
        })
        .collect()
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
    rows.push(inventory_row(&declared.native, &facts.native_egress));
    rows.extend(off_rows(&declared.native, &facts.capabilities));
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
