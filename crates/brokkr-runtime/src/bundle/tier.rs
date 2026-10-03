//! The provisional tier (proposed decision 0075 ruling 5): a model its
//! adapter declares `"tier": "provisional"` holds only the offices the
//! operator lists in `realms.json`'s `provisional_offices`
//! (`forge.realms/v7`), and never a gate. The check is a lookup and a
//! comparison at compile, before any journal is opened, on every link of
//! a site's chain: a fallback that could reach a provisional model at run
//! time is refused as the first link would be.
//!
//! An office is an agent, by name. An inline command names no agent, so
//! it holds no office: a provisional model pinned inline is seatable
//! nowhere, and an inline pin that cannot be read as one id, or no pin at
//! all, is refused wherever its adapter declares a provisional model.
//!
//! The check reads the adapters the compile opened for an agent, a gate,
//! a secret or typed tools, and otherwise [`inline_context`] opens the
//! optional read decision 0066 ruling 1 gives every inline built-in model
//! driver, so a work seat in a bundle that names no agent, seats no gate
//! and binds no secret is judged all the same. Where neither read loaded
//! nothing is declared, so nothing is provisional: an absent root compiles
//! as it always did, and a present one that does not load is refused by
//! decision 0069's own check, after the capability pass has had its say.

use brokkr_core::realms::Realm;
use serde_json::Value;
use thiserror::Error;

use super::{
    command_parts, dispatch_driver, inline_route_pin, parse_class, route_pin, AgentContext,
    Boundary, CompileError, ModelPin, SeatClass, MODEL_FLAG,
};
use crate::agents::{Adapter, Adapters, Candidate, EgressClass};
use crate::realms::World;

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

impl RealmLaw {
    /// The law of `realm` in `world`: its boundary, or `namespace` for a
    /// repository no map names, and the world's provisional offices, or
    /// none where there is no map. A resume passes the world its run
    /// pinned, so it is judged by the list the run started under.
    pub fn of(world: Option<&World>, realm: Option<&Realm>) -> RealmLaw {
        RealmLaw {
            boundary: realm.map_or(Boundary::Namespace, Realm::boundary),
            provisional_offices: world
                .map(|world| world.map.provisional_offices.clone())
                .unwrap_or_default(),
        }
    }
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
/// Each names the seat, the adapter that declares the model provisional,
/// and which rule fired.
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
    #[error(
        "seat '{seat}' pins its model on {}, which this compiler cannot read as one concrete \
         id, and adapter '{adapter}' declares a provisional model the pin may reach; a pin the \
         tier cannot read is refused, never read as promoted (proposed decision 0075 ruling 5)",
        flags_named(.flags)
    )]
    Unreadable {
        seat: String,
        adapter: String,
        flags: Vec<String>,
    },
    #[error(
        "seat '{seat}' pins no model, and adapter '{adapter}' declares a provisional model its \
         own default may be; pin a promoted model's id on the adapter's model flag, because a \
         seat whose model the tier cannot read is refused, never read as promoted (proposed \
         decision 0075 ruling 5)"
    )]
    Unpinned { seat: String, adapter: String },
    #[error(
        "seat '{seat}' pins '{value}' on {}, which is the id of no model adapter '{adapter}' \
         declares, and the adapter declares a provisional model; an alias or an undeclared id \
         may reach that model, so a pin the tier cannot map to a declared id is refused, never \
         read as promoted (proposed decision 0075 ruling 5)",
        flags_named(.flags)
    )]
    Undeclared {
        seat: String,
        adapter: String,
        flags: Vec<String>,
        value: String,
    },
}

fn office_named(office: Option<&str>) -> String {
    match office {
        Some(office) => format!("office '{office}'"),
        None => "no office (an inline command names no agent)".to_string(),
    }
}

fn flags_named(flags: &[String]) -> String {
    let named: Vec<String> = flags.iter().map(|flag| format!("'{flag}'")).collect();
    named.join(" and ")
}

/// One link of a site's chain that reaches a provisional model.
struct Link<'a> {
    number: usize,
    office: Option<&'a str>,
    adapter: &'a Adapter,
    model: &'a str,
}

/// The context an inline site's tier is judged by where no agent, gate,
/// secret or typed tools opened one: the optional inline read, where it
/// loaded, opening no library. Only inline sites reach it, and an inline
/// command holds no office, so it lists none.
pub(super) fn inline_context(
    inline: Option<&Adapters>,
    egress_minimum: EgressClass,
) -> Option<AgentContext> {
    inline.map(|adapters| AgentContext {
        library: None,
        adapters: adapters.clone(),
        egress_minimum,
        reads: Vec::new(),
        provisional_offices: Vec::new(),
    })
}

/// The site's class, once every link of its chain has passed the tier
/// check. Called where the route policy reads the class, so every site
/// the model policy judges is judged here too.
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
    let offices = &context.provisional_offices;
    let links = provisional_links(what, raw, candidates, &context.adapters)
        .map_err(CompileError::Provisional)?;
    for link in links {
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
            .is_some_and(|office| offices.iter().any(|o| o == office))
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
    what: &str,
    raw: &Value,
    candidates: &'a [Candidate],
    adapters: &'a Adapters,
) -> Result<Vec<Link<'a>>, ProvisionalRefusal> {
    if candidates.is_empty() {
        return inline_link(what, raw, adapters);
    }
    Ok(candidates
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
        .collect())
}

/// The flag a harness reads a second model from, the one it falls to at
/// run time: claude's grammar admits it as inert, so the model policy
/// walks past it, and the tier reads it as link 2 of an inline chain.
const FALLBACK_FLAG: &str = "--fallback-model";

/// An inline command's provisional models: the ones its adapter maps to
/// the concrete ids the command pins, link 1 read on the flags the model
/// policy reads (decision 0040 ruling 1) and link 2 on [`FALLBACK_FLAG`].
/// A pin those flags cannot read as one id, pinned twice or illegible,
/// may name the provisional model as well as any other, so where the
/// adapter declares one it is refused naming the flags read; the tier
/// fails closed on what it cannot read. No primary pin at all leaves the
/// model to the harness's own default, which may be the provisional one,
/// so it is refused the same way: the tier does not lean on another rule
/// to demand a pin. No fallback pin is no second link. A pin that is the
/// id of no model the adapter declares, an alias the harness resolves
/// itself or an id the file never names, may be the provisional model
/// too, so it is refused naming the flags its link is read on.
fn inline_link<'a>(
    what: &str,
    raw: &Value,
    adapters: &'a Adapters,
) -> Result<Vec<Link<'a>>, ProvisionalRefusal> {
    let Some(adapter) = dispatch_driver(&command_parts(raw))
        .and_then(|driver| adapters.adapter(&driver))
        .filter(|adapter| !adapter.provisional.is_empty())
    else {
        return Ok(Vec::new());
    };
    let unreadable = |flags| ProvisionalRefusal::Unreadable {
        seat: what.to_string(),
        adapter: adapter.provider.clone(),
        flags,
    };
    let primary = match inline_route_pin(raw, Some(adapter)) {
        ModelPin::Concrete(concrete) => concrete,
        ModelPin::Unreadable(flags) => return Err(unreadable(flags)),
        ModelPin::Absent => {
            return Err(ProvisionalRefusal::Unpinned {
                seat: what.to_string(),
                adapter: adapter.provider.clone(),
            });
        }
    };
    let fallback = match route_pin(raw, FALLBACK_FLAG) {
        ModelPin::Concrete(concrete) => Some(concrete),
        ModelPin::Unreadable(flags) => return Err(unreadable(flags)),
        ModelPin::Absent => None,
    };
    let primary_flags = adapter
        .model_flag
        .iter()
        .map(String::as_str)
        .filter(|flag| *flag != MODEL_FLAG)
        .chain([MODEL_FLAG]);
    let pins = [
        Some((primary_flags.map(str::to_string).collect(), primary)),
        fallback.map(|concrete| (vec![FALLBACK_FLAG.to_string()], concrete)),
    ];
    let mut links = Vec::new();
    for (index, (flags, concrete)) in pins.into_iter().flatten().enumerate() {
        if !adapter.models.values().any(|id| *id == concrete) {
            return Err(ProvisionalRefusal::Undeclared {
                seat: what.to_string(),
                adapter: adapter.provider.clone(),
                flags,
                value: concrete,
            });
        }
        links.extend(
            adapter
                .provisional
                .iter()
                .find(|model| adapter.models[model.as_str()] == concrete)
                .map(|model| Link {
                    number: index + 1,
                    office: None,
                    adapter,
                    model,
                }),
        );
    }
    Ok(links)
}

#[cfg(test)]
mod tests;
