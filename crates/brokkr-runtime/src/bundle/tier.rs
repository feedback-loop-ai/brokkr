//! The provisional tier (proposed decision 0075 ruling 5): a model its
//! adapter declares `"tier": "provisional"` holds only the offices the
//! operator lists in `realms.json`'s `provisional_offices`, and never a
//! gate. The check is a lookup and a comparison at compile, before any
//! journal is opened, on every link of a site's chain — a fallback that
//! could reach a provisional model at run time is refused as the first
//! link would be.
//!
//! An office is an agent, by name. An inline command names no agent, so
//! it holds no office: a provisional model pinned inline is seatable
//! nowhere. The check reads the adapters the compile opened, and
//! [`opens`] opens them wherever an inline site dispatches a driver and
//! the adapter data exists, whatever the site's class: a bundle that
//! names no agent, seats no gate and binds no secret is judged all the
//! same.

use std::path::Path;

use serde_json::{Map, Value};
use thiserror::Error;

use super::{
    command_parts, dispatch_driver, inline_route_pin, needs_adapters, parse_class, AgentContext,
    Boundary, CompileError, ModelPin, SeatClass,
};
use crate::agents::{Adapter, Adapters, Candidate};

/// What a realm rules over a compile: the boundary its boxed hands stand
/// behind (decision 0046 ruling 1) and the offices a provisional model may
/// hold. A bare boundary lists none, so every compile outside a map, and
/// every map that does not name the list, seats a provisional model
/// nowhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealmLaw {
    pub boundary: Boundary,
    pub provisional_offices: Vec<String>,
}

impl From<Boundary> for RealmLaw {
    fn from(boundary: Boundary) -> RealmLaw {
        RealmLaw {
            boundary,
            provisional_offices: Vec::new(),
        }
    }
}

/// Why a site may not seat the provisional model one of its links names.
/// Each names the seat, the link, the model, the adapter that declares it
/// provisional, and which rule fired.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProvisionalRefusal {
    #[error(
        "seat '{seat}' is gate class but link {link} seats model '{model}', which adapter \
         '{adapter}' declares provisional; a provisional model never holds a gate, whatever \
         provisional_offices lists (proposed decision 0075 ruling 5)"
    )]
    Gate {
        seat: String,
        link: usize,
        model: String,
        adapter: String,
    },
    #[error(
        "seat '{seat}' link {link} seats model '{model}', which adapter '{adapter}' declares \
         provisional, in {}, which realms.json does not list in provisional_offices; a \
         provisional model holds only the offices the operator lists, and an absent or empty \
         list names none (proposed decision 0075 ruling 5)",
        office_named(.office.as_deref())
    )]
    Unlisted {
        seat: String,
        link: usize,
        model: String,
        adapter: String,
        office: Option<String>,
    },
}

fn office_named(office: Option<&str>) -> String {
    match office {
        Some(office) => format!("office '{office}'"),
        None => "no office (an inline command names no agent)".to_string(),
    }
}

/// Does the compile open the adapters? Where [`needs_adapters`] says so,
/// and, for the tier, wherever an inline site dispatches a driver other
/// than `exec`, which pins no model, while the adapter data exists: the
/// model that command pins may be one its adapter declares provisional,
/// and a work seat is judged as a gate is. With no adapter data nothing
/// is declared, so nothing is provisional and the compile needs no
/// `adapters/` in sight.
pub(super) fn opens(seats: &Map<String, Value>, adapters_root: &Path) -> bool {
    fn dispatches(value: &Value) -> bool {
        match value {
            Value::Object(map) => {
                !matches!(
                    dispatch_driver(&command_parts(value)).as_deref(),
                    None | Some("exec")
                ) || map.values().any(dispatches)
            }
            Value::Array(items) => items.iter().any(dispatches),
            _ => false,
        }
    }
    seats.values().any(needs_adapters) || (adapters_root.exists() && seats.values().any(dispatches))
}

/// One link of a site's chain that reaches a provisional model.
struct Link<'a> {
    number: usize,
    office: Option<&'a str>,
    adapter: &'a Adapter,
    model: &'a str,
}

/// The site's class, once every link of its chain has passed the tier
/// check. Called where the model policy reads the class, so every site
/// the policy judges is judged here too.
pub(super) fn admitted(
    what: &str,
    raw: &Value,
    candidates: &[Candidate],
    agents: Option<&AgentContext>,
) -> Result<SeatClass, CompileError> {
    let class = parse_class(what, raw)?;
    let Some(context) = agents else {
        return Ok(class);
    };
    for link in provisional_links(raw, candidates, &context.adapters) {
        let (seat, model, adapter) = (
            what.to_string(),
            link.model.to_string(),
            link.adapter.provider.clone(),
        );
        if class == SeatClass::Gate {
            return Err(CompileError::Provisional(ProvisionalRefusal::Gate {
                seat,
                link: link.number,
                model,
                adapter,
            }));
        }
        if !link
            .office
            .is_some_and(|office| context.provisional_offices.iter().any(|o| o == office))
        {
            return Err(CompileError::Provisional(ProvisionalRefusal::Unlisted {
                seat,
                link: link.number,
                model,
                adapter,
                office: link.office.map(str::to_string),
            }));
        }
    }
    Ok(class)
}

/// The links of a site's chain whose model is provisional: every link of
/// an agent's resolved chain, or the one model an inline command pins.
fn provisional_links<'a>(
    raw: &Value,
    candidates: &'a [Candidate],
    adapters: &'a Adapters,
) -> Vec<Link<'a>> {
    if candidates.is_empty() {
        return inline_link(raw, adapters).into_iter().collect();
    }
    candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            let (adapter, _) = adapters
                .serving(&candidate.model)
                .expect("resolution mapped every link of the chain");
            adapter
                .provisional
                .contains(&candidate.model)
                .then_some(Link {
                    number: index + 1,
                    office: Some(candidate.agent.as_str()),
                    adapter,
                    model: candidate.model.as_str(),
                })
        })
        .collect()
}

/// An inline command's provisional model: the one its adapter maps to
/// the concrete id the command pins, read on the flags the model policy
/// reads (decision 0040 ruling 1).
fn inline_link<'a>(raw: &Value, adapters: &'a Adapters) -> Option<Link<'a>> {
    let adapter = adapters.adapter(&dispatch_driver(&command_parts(raw))?)?;
    let ModelPin::Concrete(concrete) = inline_route_pin(raw, Some(adapter)) else {
        return None;
    };
    let model = adapter
        .provisional
        .iter()
        .find(|model| adapter.models[model.as_str()] == concrete)?;
    Some(Link {
        number: 1,
        office: None,
        adapter,
        model,
    })
}

#[cfg(test)]
mod tests;
