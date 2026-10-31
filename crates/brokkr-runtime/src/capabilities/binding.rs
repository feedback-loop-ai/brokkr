//! What a granted capability is bound to, projected from its dialect's
//! kind (decision 0065 slice two, SC5). The kind is the one home of the
//! binding: a native dialect names its provider and adapter key, and an
//! `mcp` or reserved `hands` dialect names none. Nothing here indexes a
//! map, expects a native pair, invents an empty provider or reads another
//! kind as native. Every caller gets the native pair or the typed reason
//! there is none. The resolver's reading of an `mcp` binding, with the
//! bound on its identity, its egress against the bundle's binding minimum
//! and what a holding retains (SC1, SC4, MB4, CR1) are read from the same
//! kind. What that comparison admitted is sealed once, as the holding's
//! [`Admitted`] facts (U7c0).

use brokkr_core::canonical::{sha256_bytes, to_bytes};
use brokkr_core::realms::{CapabilityGrant, GrantRetention};
use brokkr_protocol::broker::{Clearance, Network};
use serde_json::{Map, Value};

use super::{
    gates, Authority, CapabilityContext, Cause, Connection, DialectKind, Implementation, McpFence,
    McpServer, Requests, SiteAsks, ToolDialect, Unheld, ADAPTER_SEAT,
};
use crate::agents::EgressClass;
use crate::bundle::{CompileError, SeatClass};

/// The binding minimum of a bundle that declares no `egress_minimum`
/// (decision 0036 ruling 4): what the superseded `binding_grant: true`
/// meant, so every bundle on disk keeps its behaviour and its digest.
pub(crate) const ABSENT_EGRESS_MINIMUM: EgressClass = EgressClass::Contracted;

/// The byte bound on an `mcp` holding's capability or dialect name, and
/// on its concrete tool name, that a call's attribution carries whole
/// (SC4).
const NAME_BYTES: usize = 128;
const TOOL_BYTES: usize = 256;

/// Why a granted capability has no native `(provider, adapter key)`. The
/// text of each is the compile's: `Authority::load` refuses an `mcp` and a
/// `hands` grant with it, and the resolver and `brokkr doctor` give the
/// same words where they meet one.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Unbound {
    /// A valid `mcp` dialect where the fence stands, as it does on macOS:
    /// its server is a broker's to carry, and no box stands there for a
    /// broker to carry it in. Past the fence, on Linux, the grant loads and
    /// no compile refuses by this cause; `Authority::binding` still gives it,
    /// as such a grant has no native binding for doctor to compare.
    #[error(
        "realm '{realm}' grants capability '{capability}' through dialect '{dialect}' of kind \
         'mcp', whose broker support is not implemented until decision 0065 slice two"
    )]
    Mcp {
        realm: String,
        capability: String,
        dialect: String,
    },
    /// The reserved `hands` kind, never a realm grant.
    #[error(
        "realm '{realm}' grants capability '{capability}' through dialect '{dialect}' of kind \
         'hands', which is reserved: the workspace tool stays governed by decisions 0043 and \
         0046 and is not a realm grant"
    )]
    Hands {
        realm: String,
        capability: String,
        dialect: String,
    },
    /// No dialect was loaded for the capability, so nothing binds it.
    #[error(
        "realm '{realm}' has no loaded dialect for capability '{capability}', so it is bound to \
         no provider"
    )]
    Missing { realm: String, capability: String },
}

/// A granted capability's native binding, beside the dialect that names it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Native<'a> {
    pub dialect: &'a ToolDialect,
    pub provider: &'a str,
    pub adapter_key: &'a str,
}

/// Why a candidate cannot hold a granted capability through its binding,
/// beside the binding's own [`Unbound`] (design D3 step 6).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum Unserved {
    #[error(transparent)]
    Unbound(#[from] Unbound),
    /// A held name a call's attribution could carry only truncated (SC4).
    #[error("capability call identity cannot be represented by seat-record v6")]
    Identity,
    /// A declared reference is valid data and never substituted (SC1).
    #[error(
        "MCP connection argv cannot contain secret references; declare environment bindings in \
         secrets"
    )]
    ArgvReference,
    #[error("MCP URL connections are not implemented in decision 0065 slice two")]
    Url,
    #[error(transparent)]
    Below(#[from] Below),
    /// The broker carries only the empty restriction (design D3 step 6;
    /// D11): a nonempty one is never discarded alone.
    #[error("MCP nonempty restrictions are deferred to the restriction-transport slice")]
    Restricted,
    /// MB4: a secret the dialect binds is a name the serving harness reads
    /// to authenticate.
    #[error("MCP secret binding '{0}' collides with harness authentication")]
    Collides(String),
}

impl Unserved {
    /// How the resolver holds this against an ask through `dialect`: SC4's
    /// unrepresentable identity refuses the compile whatever the strength;
    /// MB4's egress comparison, which names its dialect, and D3's deferred
    /// MCP restriction close both of GP1's forms on their own words, with
    /// no incompatibility suffix and no native tail (U5a2 F1); and every
    /// other cause is the dialect's incompatibility, which the ask's
    /// strength settles.
    pub(super) fn unheld(self, dialect: &str) -> Unheld {
        match self {
            Unserved::Identity => Unheld::Identity,
            Unserved::Below(_) | Unserved::Restricted => {
                Unheld::Cause(Cause::Ungranted(self.to_string()))
            }
            Unserved::Unbound(_)
            | Unserved::ArgvReference
            | Unserved::Url
            | Unserved::Collides(_) => {
                Unheld::Cause(Cause::incompatible(dialect, self.to_string()))
            }
        }
    }
}

/// MB4: the dialect's own route does not meet the bundle's binding
/// minimum, whatever clearance the harness's route has ([`admitted`]).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "MCP dialect '{dialect}' has egress '{}' below binding minimum '{}'",
    .egress.name(),
    .minimum.name()
)]
pub struct Below {
    pub(super) dialect: String,
    pub(super) egress: EgressClass,
    pub(super) minimum: EgressClass,
}

/// What a dialect binds its capability to: one arm per bindable kind.
enum Binding<'a> {
    Native {
        provider: &'a str,
        adapter_key: &'a str,
    },
    Mcp(&'a McpServer),
}

/// What `dialect` binds `capability` to in `realm`, or that its kind is
/// the reserved `hands`: one arm per kind, with no wildcard.
fn bound<'a>(
    realm: &str,
    capability: &str,
    dialect: &'a ToolDialect,
) -> Result<Binding<'a>, Unbound> {
    match &dialect.kind {
        DialectKind::Native {
            provider,
            adapter_key,
        } => Ok(Binding::Native {
            provider,
            adapter_key,
        }),
        DialectKind::Mcp(server) => Ok(Binding::Mcp(server)),
        DialectKind::Hands => Err(Unbound::Hands {
            realm: realm.into(),
            capability: capability.into(),
            dialect: dialect.name.clone(),
        }),
    }
}

/// The native `(provider, adapter key)` `dialect` binds `capability` to in
/// `realm`, or why it binds none: `brokkr doctor`'s reading, and the
/// standing compile fence's, under which an `mcp` binding has no broker.
pub(super) fn native<'a>(
    realm: &str,
    capability: &str,
    dialect: &'a ToolDialect,
) -> Result<(&'a str, &'a str), Unbound> {
    match bound(realm, capability, dialect)? {
        Binding::Native {
            provider,
            adapter_key,
        } => Ok((provider, adapter_key)),
        Binding::Mcp(_) => Err(unbuilt(realm, capability, &dialect.name)),
    }
}

fn unbuilt(realm: &str, capability: &str, dialect: &str) -> Unbound {
    Unbound::Mcp {
        realm: realm.into(),
        capability: capability.into(),
        dialect: dialect.into(),
    }
}

/// Whether `fence` stands, so an `mcp` binding meets the unbuilt kind's
/// own words (macOS; SC5, operator ruling 1 of 2026-10-18), or is lifted,
/// so its broker carries it (Linux, U9b1).
fn stands(fence: McpFence) -> bool {
    match fence {
        #[cfg(any(test, not(target_os = "linux")))]
        McpFence::Standing => true,
        #[cfg(any(test, target_os = "linux"))]
        McpFence::Lifted => false,
    }
}

/// Does this argument name a decision 0012 secret? The loader refused a
/// malformed or undeclared reference, so what remains is a declared one.
fn references(part: &str) -> bool {
    brokkr_protocol::secret::scan_secret_refs(part).is_ok_and(|names| !names.is_empty())
}

/// SC4: an `mcp` holding's identity, which a broker call's attribution
/// names, is carried whole or refused, never truncated — the capability
/// and dialect names within [`NAME_BYTES`], and each tool a bounded
/// identifier in seat-record v5's tool vocabulary. A native dialect's
/// tools are harness patterns (`Bash(git:*)`), and its holding is judged
/// as slice one judged it.
pub(super) fn representable(
    capability: &str,
    dialect: &ToolDialect,
    tools: &[String],
) -> Result<(), Unserved> {
    let tool = |tool: &String| {
        let mut chars = tool.chars();
        tool.len() <= TOOL_BYTES
            && chars
                .next()
                .is_some_and(|first| first.is_ascii_alphanumeric())
            && chars.all(|next| next.is_ascii_alphanumeric() || "._:/-".contains(next))
    };
    match capability.len() <= NAME_BYTES
        && dialect.name.len() <= NAME_BYTES
        && tools.iter().all(tool)
    {
        true => Ok(()),
        false => Err(Unserved::Identity),
    }
}

/// What a holding retains (CR1): the dialect's declaration beside what
/// the grant's map version says of it. Only an `mcp` dialect declares;
/// native retention is unsupported, so a native one declares false.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retention {
    pub declared: bool,
    pub realm: GrantRetention,
}

impl Retention {
    /// What `grant` makes of `dialect`'s declaration: the holding's, and
    /// what `brokkr doctor` names on an `mcp` grant's line (U9a2).
    pub fn of(dialect: &ToolDialect, grant: &CapabilityGrant) -> Retention {
        let declared = match &dialect.kind {
            DialectKind::Mcp(server) => server.retained,
            DialectKind::Native { .. } | DialectKind::Hands => false,
        };
        Retention {
            declared,
            realm: grant.retention,
        }
    }

    /// True exactly where the dialect retains and the realm does not
    /// veto; a realm can never require retention. Manifest v12's effective
    /// retention reads it (`manifest.rs`, `Retention::value`), and so does
    /// the engine's plan.
    pub(crate) fn effective(self) -> bool {
        match self.realm {
            GrantRetention::Veto => false,
            GrantRetention::Unreserved | GrantRetention::Inherit => self.declared,
        }
    }
}

/// One held capability, fully attributable (ruling 8).
#[derive(Debug, Clone, PartialEq)]
pub struct Holding {
    pub classes: Vec<String>,
    pub dialect: String,
    pub dialect_sha256: String,
    pub definition_sha256: String,
    pub tools: Vec<String>,
    /// The realm's restriction object, exactly as written.
    pub restrictions: Map<String, Value>,
    /// What carries it (SC3), from the dialect's kind.
    pub(crate) implementation: Implementation,
    /// The dialect's declaration beside the grant's veto (CR1).
    pub retention: Retention,
    /// What MB4's comparison sealed where it admitted an `mcp` holding;
    /// none for a native one, which that comparison never judges. Not a
    /// fact of manifest identity.
    pub admitted: Option<Admitted>,
}

/// What MB4's comparison earned an `mcp` holding it admitted (U7c0, D5):
/// the network its pinned dialect's egress projects to, and the receipt
/// of the comparison. The engine provisions its plan from these and never
/// reloads a grant or a dialect to recover them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    pub network: Network,
    pub clearance: Clearance,
}

impl Holding {
    /// The native `(provider, adapter key)` a provider must carry this
    /// holding of `capability` in `realm` by, or `None` where the engine's
    /// broker carries it instead: an `mcp` holding, sealed with what MB4's
    /// comparison admitted, past the lifted `fence`. While the fence
    /// stands, such a holding meets the fence's own words.
    pub(super) fn native(
        &self,
        realm: &str,
        capability: &str,
        fence: McpFence,
    ) -> Result<Option<(&str, &str)>, Unheld> {
        match &self.implementation {
            Implementation::Native {
                provider,
                adapter_key,
            } => Ok(Some((provider, adapter_key))),
            Implementation::Mcp { .. } if stands(fence) => {
                Err(Unserved::from(unbuilt(realm, capability, &self.dialect)).unheld(&self.dialect))
            }
            Implementation::Mcp { .. } => match self.restrictions.is_empty() {
                true => Ok(None),
                false => Err(Unserved::Restricted.unheld(&self.dialect)),
            },
        }
    }

    /// MB4 at the serving candidate: the first secret this `mcp` holding
    /// binds that is one of `auth`, the names the candidate's harness reads
    /// to authenticate. A native holding binds no secret.
    pub(super) fn collision(&self, auth: &[String]) -> Result<(), Unserved> {
        match &self.implementation {
            Implementation::Mcp { secrets, .. } => {
                match secrets.iter().find(|s| auth.contains(s)) {
                    Some(name) => Err(Unserved::Collides(name.clone())),
                    None => Ok(()),
                }
            }
            Implementation::Native { .. } => Ok(()),
        }
    }
}

/// The closed network `egress` projects to: a `local` route is isolated,
/// and any route that leaves the host shares its network.
fn network(egress: EgressClass) -> Network {
    match egress {
        EgressClass::Local => Network::Isolated,
        EgressClass::Contracted | EgressClass::Uncontracted => Network::Shared,
    }
}

/// The seat [`Authority::assess`]'s plan is resolved for: [`ADAPTER_SEAT`]
/// of `office`, asking `asks`. A seatless hypothesis has no class to read:
/// judged as work, it answers no gate's holding (GP1) and authorizes no
/// launch.
pub(super) fn hypothesis(office: &str, asks: Requests) -> SiteAsks {
    SiteAsks {
        label: ADAPTER_SEAT.to_string(),
        office: office.to_string(),
        asks,
        subtracted: Vec::new(),
        class: SeatClass::Work,
    }
}

/// MB4's one comparison of `dialect`'s egress against the bundle's binding
/// `minimum`: below it, the cause; at or above it, what the comparison
/// sealed. The receipt binds the dialect's digest and the sha256 of the
/// canonical bytes of exactly the minimum it read (operator ruling
/// 2026-10-15). The resolver's, and what `brokkr doctor` names on an `mcp`
/// grant's line (U9a2).
pub fn admitted(dialect: &ToolDialect, minimum: EgressClass) -> Result<Admitted, Below> {
    if dialect.egress < minimum {
        return Err(Below {
            dialect: dialect.name.clone(),
            egress: dialect.egress,
            minimum,
        });
    }
    let policy = sha256_bytes(&to_bytes(&Value::from(minimum.name())));
    Ok(Admitted {
        network: network(dialect.egress),
        clearance: Clearance {
            dialect: dialect.sha256.clone(),
            policy,
        },
    })
}

impl Authority {
    /// Validate every grant of the operated realm BEFORE any seat is
    /// looked at, then the realm-wide refusals, which hold whether or not
    /// any seat asks: a `hands` grant, and an `mcp` grant while the compile
    /// fence (`McpFence`) stands. None of these can become an optional drop.
    pub fn load(context: CapabilityContext) -> Result<Authority, String> {
        let authority = Authority::validated(context)?;
        for (capability, dialect) in &authority.dialects {
            let realm = &authority.context.realm;
            let carried = match bound(realm, capability, dialect) {
                Ok(Binding::Mcp(_)) if stands(authority.fence) => {
                    Err(unbuilt(realm, capability, &dialect.name))
                }
                Ok(Binding::Native { .. } | Binding::Mcp(_)) => Ok(()),
                Err(unbound) => Err(unbound),
            };
            carried.map_err(|unbound| unbound.to_string())?;
        }
        Ok(authority)
    }

    /// Every grant validated as [`Authority::load`] validates it, with
    /// neither realm-wide refusal applied, refused as a compile refuses an
    /// invalid one. What `brokkr doctor` reads an `mcp` grant through, and
    /// what a survey of a bundle's seats resolves against (U9a2); no
    /// compile or launch is given an authority, so neither can use it.
    pub fn survey(context: CapabilityContext) -> Result<Authority, CompileError> {
        Authority::validated(context).map_err(CompileError::Capability)
    }

    /// This authority, judging every `mcp` dialect's egress against the
    /// bundle's binding `minimum` (MB4) rather than the absent default.
    pub(crate) fn with_minimum(mut self, minimum: EgressClass) -> Authority {
        self.minimum = minimum;
        self
    }

    /// The binding minimum this authority judges by, the one home a
    /// compile's route policy reads it from too (decision 0036 ruling 4).
    pub(crate) fn minimum(&self) -> EgressClass {
        self.minimum
    }

    /// The resolver's reading of the binding of `capability`, held as
    /// `tools`: what MB4's comparison admitted for an `mcp` binding, none
    /// for a native one, or why no candidate can carry it. An `mcp` binding
    /// answers in design D3 step 6's order — its identity, then an
    /// unexecutable connection's SC1 cause, then its egress against the
    /// bundle's binding minimum (MB4); past the lifted fence the broker
    /// carries its sealed holding ([`Holding::native`]), and while the fence
    /// stands its own words follow. A native binding is judged as slice one
    /// judged it, whatever its dialect's egress. A standing fence refuses
    /// every `mcp` grant before a compile reaches it; a survey (U9a2)
    /// reaches it for an `mcp` grant either way.
    fn carried(
        &self,
        capability: &str,
        dialect: &ToolDialect,
        tools: &[String],
    ) -> Result<Option<Admitted>, Unserved> {
        let server = match bound(&self.context.realm, capability, dialect)? {
            Binding::Native { .. } => return Ok(None),
            Binding::Mcp(server) => server,
        };
        representable(capability, dialect, tools)?;
        match &server.connection {
            Connection::Url(_) => Err(Unserved::Url),
            Connection::Stdio(argv) if argv.iter().any(|part| references(part)) => {
                Err(Unserved::ArgvReference)
            }
            Connection::Stdio(_) => Ok(Some(admitted(dialect, self.minimum)?)),
        }
    }

    /// The realm's grant of `capability` that reaches the site's office
    /// (design D4), or why none does: no dialect is chosen before it.
    fn reaching(&self, site: &SiteAsks, capability: &str) -> Result<&CapabilityGrant, Cause> {
        let Some(grant) = self.context.grants.get(capability) else {
            return Err(Cause::Ungranted(
                "the realm does not grant it to this office".into(),
            ));
        };
        if grant.reaches(&site.office) {
            return Ok(grant);
        }
        let scope = grant.offices.as_deref().unwrap_or_default();
        Err(Cause::Ungranted(match scope.is_empty() {
            true => "the realm grants it to no office".to_string(),
            false => format!(
                "the realm grants it only to offices [{}], not to this office",
                scope.join(", ")
            ),
        }))
    }

    /// The holding the realm's grant of `capability` seals at `site`,
    /// before any provider is asked to carry it: the grant reaching the
    /// site's office, its loaded dialect, the tools it admits, GP1's gates
    /// and the binding's own reading (design D3 steps 5 and 6). What MB4's
    /// comparison admitted for an `mcp` binding is sealed on it here, the
    /// one place a holding gets it (U7c0).
    pub(super) fn sealed(&self, site: &SiteAsks, capability: &str) -> Result<Holding, Unheld> {
        let grant = self.reaching(site, capability)?;
        let dialect = self
            .dialect(capability)
            .map_err(|missing| Cause::incompatible(&grant.dialect, missing.to_string()))?;
        let tools = grant.tools.clone().unwrap_or_else(|| dialect.tools.clone());
        if tools.is_empty() {
            let none = "the realm's grant admits no tool".to_string();
            return Err(Cause::incompatible(&dialect.name, none).into());
        }
        // GP1, before any provider carries it (design D3 steps 5 and 6).
        let definition = &self.definitions.0[capability];
        gates::check(site, definition, grant)
            .map_err(|refusal| Cause::gate(&dialect.name, refusal))?;
        let admitted = self
            .carried(capability, dialect, &tools)
            .map_err(|unserved| unserved.unheld(&dialect.name))?;
        Ok(Holding {
            classes: definition.classes.clone(),
            dialect: dialect.name.clone(),
            dialect_sha256: dialect.sha256.clone(),
            definition_sha256: definition.sha256.clone(),
            tools,
            restrictions: grant.restrictions.clone(),
            implementation: Implementation::of(capability, &dialect.kind)
                .expect("a carried binding is never the reserved hands kind"),
            retention: Retention::of(dialect, grant),
            admitted,
        })
    }

    /// The dialect the grant of `capability` selected, or that none was
    /// loaded for it.
    pub(super) fn dialect(&self, capability: &str) -> Result<&ToolDialect, Unbound> {
        self.dialects
            .get(capability)
            .ok_or_else(|| Unbound::Missing {
                realm: self.context.realm.clone(),
                capability: capability.into(),
            })
    }

    /// The native binding of a granted capability — what `brokkr doctor`
    /// compares an installed harness's native declarations against — or
    /// the typed reason it has none. A same-name grant bound to another
    /// provider covers nothing of this one's.
    pub fn binding(&self, capability: &str) -> Result<Native<'_>, Unbound> {
        let dialect = self.dialect(capability)?;
        let (provider, adapter_key) = native(&self.context.realm, capability, dialect)?;
        Ok(Native {
            dialect,
            provider,
            adapter_key,
        })
    }
}
