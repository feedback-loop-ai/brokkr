//! `brokkr agents` — the agent library, mirroring `brokkr recipes`.
//!
//! `list` warns on a broken entry and keeps listing: one bad file is
//! information, not a fatal error for the whole library, which is the
//! reason the library is one file per agent rather than one index.
//! `show` prints the definition as written plus a `resolution` block
//! computed by the SAME pure function the compiler calls — machine
//! readable without a `--json` flag, and unable to drift from the data
//! because there is no second formatter to drift from.

use std::path::Path;

use anyhow::{bail, Result};
use brokkr_runtime::agents::{report, ChainEntry, Presence, Report};
use brokkr_runtime::capabilities::Definitions;
use brokkr_runtime::{Adapters, Availability, Library};
use serde::Serialize;
use serde_json::{json, Value};

/// How the machine asking sees a provider. `unknown` is what compile
/// sees and what `show` reports unless a caller probed.
pub(crate) fn presence_word(presence: Presence) -> &'static str {
    match presence {
        Presence::Available => "available",
        Presence::Unavailable => "unavailable",
        Presence::Unknown => "unknown",
    }
}

/// One chain entry of the `resolution` block. Each struct of the block
/// declares its fields in byte order, the order its keys were printed in
/// when the block was a `json!` map (#346).
#[derive(Serialize)]
pub(crate) struct EntryView<'a> {
    model: &'a str,
    notices: Vec<Value>,
    presence: &'static str,
    /// Written only by an entry with a gap.
    #[serde(skip_serializing_if = "Option::is_none")]
    problem: Option<String>,
    provider: Option<&'a str>,
    status: &'static str,
}

/// The `resolution` block `show` prints: every entry, and the one chosen,
/// null where none is.
#[derive(Serialize)]
struct ResolutionView<'a> {
    chain: Vec<EntryView<'a>>,
    chosen: Option<Chosen<'a>>,
}

/// The entry [`ResolutionView`] chose.
#[derive(Serialize)]
struct Chosen<'a> {
    index: usize,
    model: &'a str,
    provider: Option<&'a str>,
}

/// One chain entry, as a readout prints it: the provider that maps it or
/// `unmapped`, and the capability check's verdict. A blocked entry is
/// REPORTED here rather than thrown, so a reader sees the whole chain
/// and can act on the link that is wrong.
pub(crate) fn entry_value(entry: &ChainEntry) -> EntryView<'_> {
    EntryView {
        model: &entry.model,
        notices: entry
            .notices
            .iter()
            .map(brokkr_runtime::agents::Notice::value)
            .collect(),
        presence: presence_word(entry.presence),
        problem: entry.gap.as_ref().map(ToString::to_string),
        provider: entry.provider.as_deref(),
        status: match (&entry.provider, &entry.gap) {
            (None, _) => "unmapped",
            (Some(_), Some(_)) => "blocked",
            (Some(_), None) => "ok",
        },
    }
}

fn resolution_value(walked: &Report) -> ResolutionView<'_> {
    ResolutionView {
        chain: walked.entries.iter().map(entry_value).collect(),
        chosen: walked.chosen.map(|index| Chosen {
            index,
            model: &walked.entries[index].model,
            provider: walked.entries[index].provider.as_deref(),
        }),
    }
}

/// The operator's abstract capability definitions (decision 0065 ruling
/// 1), read from the operator's configuration directory — never from the
/// library's own, which an `--agents-dir` override may point anywhere.
fn definitions(operator_root: &Path) -> Result<Definitions> {
    Definitions::load(operator_root).map_err(anyhow::Error::msg)
}

/// One tab-separated line per agent — `name ⇥ chain ⇥ description` — and
/// a warning line per definition that does not parse, and per capability
/// an agent asks for that the operator has not defined: semantic lint,
/// which parsing the request map alone is not. Nothing aborts the listing.
pub(crate) fn list(library_root: &Path, operator_root: &Path) -> Result<()> {
    let (library, mut problems) = Library::scan(library_root)?;
    problems.extend(definitions(operator_root)?.lint(&library));
    for problem in &problems {
        println!("warning: {problem}");
    }
    for agent in library.agents() {
        println!(
            "{}\t{}\t{}",
            agent.name,
            agent.models.join(" → "),
            agent.description
        );
    }
    Ok(())
}

/// The definition as written, plus its per-entry resolution. An unknown
/// name errors naming the known set, so the next command is obvious. A
/// library asking for a capability the operator has not defined is
/// refused as a library that does not load is: `show` prints what the
/// compiler would resolve, and the compiler refuses that ask.
pub(crate) fn show(
    name: &str,
    library_root: &Path,
    adapters_root: &Path,
    operator_root: &Path,
) -> Result<()> {
    let library = Library::load(library_root)?;
    if let Some(problem) = definitions(operator_root)?.lint(&library).first() {
        bail!("{problem}");
    }
    let adapters = Adapters::load(adapters_root)?;
    let walked = match report(&library, &adapters, &Availability::unspecified(), name) {
        Ok(walked) => walked,
        Err(error) => bail!("{error}"),
    };
    let mut out = walked.agent.source.clone();
    out["name"] = Value::String(walked.agent.name.clone());
    out["agent_digest"] = Value::String(walked.agent.digest.clone());
    out["charter_digest"] = Value::String(walked.agent.charter_digest.clone());
    out["resolution"] = json!(resolution_value(&walked));
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

#[cfg(test)]
mod tests;
