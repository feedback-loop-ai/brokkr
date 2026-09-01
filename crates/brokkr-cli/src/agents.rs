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
use brokkr_runtime::{Adapters, Availability, Library};
use serde_json::{json, Value};

/// How the machine asking sees a provider. `unknown` is what compile
/// sees and what `show` reports unless a caller probed.
pub fn presence_word(presence: Presence) -> &'static str {
    match presence {
        Presence::Available => "available",
        Presence::Unavailable => "unavailable",
        Presence::Unknown => "unknown",
    }
}

/// One chain entry, as a readout prints it: the provider that maps it or
/// `unmapped`, and the capability check's verdict. A blocked entry is
/// REPORTED here rather than thrown, so a reader sees the whole chain
/// and can act on the link that is wrong.
pub fn entry_value(entry: &ChainEntry) -> Value {
    let mut value = json!({
        "model": entry.model,
        "provider": entry.provider.clone().map(Value::String).unwrap_or(Value::Null),
        "presence": presence_word(entry.presence),
        "status": match (&entry.provider, &entry.gap) {
            (None, _) => "unmapped",
            (Some(_), Some(_)) => "blocked",
            (Some(_), None) => "ok",
        },
        "notices": entry
            .notices
            .iter()
            .map(brokkr_runtime::agents::Notice::value)
            .collect::<Vec<_>>(),
    });
    if let Some(gap) = &entry.gap {
        value["problem"] = Value::String(gap.to_string());
    }
    value
}

fn resolution_value(walked: &Report) -> Value {
    let chosen = match walked.chosen {
        None => Value::Null,
        Some(index) => json!({
            "index": index,
            "model": walked.entries[index].model,
            "provider": walked.entries[index].provider,
        }),
    };
    json!({
        "chain": walked.entries.iter().map(entry_value).collect::<Vec<_>>(),
        "chosen": chosen,
    })
}

/// One tab-separated line per agent — `name ⇥ chain ⇥ description` — and
/// a warning line per definition that does not parse. Nothing aborts the
/// listing.
pub fn list(library_root: &Path) -> Result<()> {
    let (library, problems) = Library::scan(library_root)?;
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
/// name errors naming the known set, so the next command is obvious.
pub fn show(name: &str, library_root: &Path, adapters_root: &Path) -> Result<()> {
    let library = Library::load(library_root)?;
    let adapters = Adapters::load(adapters_root)?;
    let walked = match report(&library, &adapters, &Availability::unspecified(), name) {
        Ok(walked) => walked,
        Err(error) => bail!("{error}"),
    };
    let mut out = walked.agent.source.clone();
    out["name"] = Value::String(walked.agent.name.clone());
    out["agent_digest"] = Value::String(walked.agent.digest.clone());
    out["charter_digest"] = Value::String(walked.agent.charter_digest.clone());
    out["resolution"] = resolution_value(&walked);
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

#[cfg(test)]
mod tests;
