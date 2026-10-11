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
//! all, is refused wherever a loaded adapter declares a provisional model.
//! The tier belongs to a concrete id, so a link is judged against every
//! loaded adapter's provisional ids, not only those of the adapter that
//! serves it.
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
        "seat '{seat}' pins no model, and adapter '{adapter}' declares a provisional model the \
         harness's own default may be; pin a promoted model's id on the adapter's model flag, \
         because a seat whose model the tier cannot read is refused, never read as promoted \
         (proposed decision 0075 ruling 5)"
    )]
    Unpinned { seat: String, adapter: String },
    #[error(
        "seat '{seat}' pins '{value}' on {}, which is the id of no model the seat's own adapter \
         declares, and adapter '{adapter}' declares a provisional model; an alias or an \
         undeclared id may reach that model, so a pin the tier cannot map to a declared id is \
         refused, never read as promoted (proposed decision 0075 ruling 5)",
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
            let (serving, id) = adapters
                .serving(&candidate.model)
                .expect("resolution mapped every link of the chain");
            declaring(adapters, Some(serving), id).map(|(adapter, model)| Link {
                number: index + 1,
                office: Some(candidate.agent.as_str()),
                adapter,
                model,
            })
        })
        .collect())
}

/// The adapter that declares the concrete `id` provisional, and the name
/// it declares it under, the seat's own adapter read first. The tier is
/// the id's, not the entry's: a model one adapter declares provisional is
/// provisional whichever adapter serves it and under whichever name, a
/// sibling's tierless entry or a second alias of the same id included.
fn declaring<'a>(
    adapters: &'a Adapters,
    own: Option<&'a Adapter>,
    id: &str,
) -> Option<(&'a Adapter, &'a str)> {
    own.into_iter()
        .chain(adapters.providers())
        .find_map(|adapter| {
            adapter
                .provisional
                .iter()
                .find(|name| adapter.models[name.as_str()] == id)
                .map(|name| (adapter, name.as_str()))
        })
}

/// The flag a harness reads a second model from, the one it falls to at
/// run time: claude's grammar admits it as inert, so the model policy
/// walks past it, and the tier reads it as link 2 of an inline chain.
pub(super) const FALLBACK_FLAG: &str = "--fallback-model";

/// An inline command's provisional models: the ones any loaded adapter
/// declares provisional at the concrete ids the command pins, link 1 read
/// on the flags the model policy reads (decision 0040 ruling 1) and link 2
/// on [`FALLBACK_FLAG`]. The tier is the id's, so once any loaded adapter
/// declares a provisional model every inline model seat is judged, not
/// only those its own adapter serves: a sibling adapter, or a second file
/// over the same driver kind, may route to the same id. A pin those flags
/// cannot read as one id, pinned twice or illegible, may name the
/// provisional model as well as any other, so it is refused naming the
/// flags read; the tier fails closed on what it cannot read. No primary
/// pin at all leaves the model to the harness's own default, which may be
/// the provisional one, so it is refused the same way: the tier does not
/// lean on another rule to demand a pin. No fallback pin is no second
/// link. A pin that is the id of no model the seat's own adapter declares,
/// an alias the harness resolves itself or an id the file never names, may
/// be the provisional model too, so it is refused naming the flags its
/// link is read on; a driver no adapter declares declares no id. Each
/// refusal names the adapter that declares a provisional model, the
/// seat's own where it does. A driver that declares no model and can be
/// told none, exec's shape, seats nothing the tier can name: what an
/// arbitrary command runs is decisions 0043 and 0046's.
fn inline_link<'a>(
    what: &str,
    raw: &Value,
    adapters: &'a Adapters,
) -> Result<Vec<Link<'a>>, ProvisionalRefusal> {
    let Some(driver) = dispatch_driver(&command_parts(raw)) else {
        return Ok(Vec::new());
    };
    let own = adapters.adapter(&driver);
    let declares = |adapter: &&Adapter| !adapter.provisional.is_empty();
    let Some(declarer) = own
        .filter(declares)
        .or_else(|| adapters.providers().find(declares))
    else {
        return Ok(Vec::new());
    };
    if own.is_some_and(|own| own.models.is_empty() && own.model_flag.is_none()) {
        return Ok(Vec::new());
    }
    let pins = inline_pins(what, raw, own, &declarer.provider)?;
    let mut links = Vec::new();
    for (index, (flags, concrete)) in pins.into_iter().enumerate() {
        if !own.is_some_and(|own| own.models.values().any(|id| *id == concrete)) {
            return Err(ProvisionalRefusal::Undeclared {
                seat: what.to_string(),
                adapter: declarer.provider.clone(),
                flags,
                value: concrete,
            });
        }
        links.extend(
            declaring(adapters, own, &concrete).map(|(adapter, model)| Link {
                number: index + 1,
                office: None,
                adapter,
                model,
            }),
        );
    }
    Ok(links)
}

/// An inline command's pins in link order, each with the flags it was
/// read on: the primary, which must be there, and the fallback, where
/// there is one. A pin that cannot be read, or no primary, is refused
/// naming `declarer`, the adapter whose provisional model it may reach.
fn inline_pins(
    what: &str,
    raw: &Value,
    own: Option<&Adapter>,
    declarer: &str,
) -> Result<Vec<(Vec<String>, String)>, ProvisionalRefusal> {
    let unreadable = |flags| ProvisionalRefusal::Unreadable {
        seat: what.to_string(),
        adapter: declarer.to_string(),
        flags,
    };
    let primary = match inline_route_pin(raw, own) {
        ModelPin::Concrete(concrete) => concrete,
        ModelPin::Unreadable(flags) => return Err(unreadable(flags)),
        ModelPin::Absent => {
            return Err(ProvisionalRefusal::Unpinned {
                seat: what.to_string(),
                adapter: declarer.to_string(),
            });
        }
    };
    let fallback = match route_pin(raw, FALLBACK_FLAG) {
        ModelPin::Concrete(concrete) => Some(concrete),
        ModelPin::Unreadable(flags) => return Err(unreadable(flags)),
        ModelPin::Absent => None,
    };
    let primary_flags = own
        .and_then(|own| own.model_flag.as_deref())
        .filter(|flag| *flag != MODEL_FLAG)
        .into_iter()
        .chain([MODEL_FLAG]);
    let pins = [
        Some((primary_flags.map(str::to_string).collect(), primary)),
        fallback.map(|concrete| (vec![FALLBACK_FLAG.to_string()], concrete)),
    ];
    Ok(pins.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests;
