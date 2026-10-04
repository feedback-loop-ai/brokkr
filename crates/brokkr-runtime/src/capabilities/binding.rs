//! What a granted capability is bound to, projected from its dialect's
//! kind (decision 0065 slice two, SC5). The kind is the one home of the
//! binding: a native dialect names its provider and adapter key, and an
//! `mcp` or reserved `hands` dialect names none. Nothing here indexes a
//! map, expects a native pair, invents an empty provider or reads another
//! kind as native. Every caller gets the native pair or the typed reason
//! there is none. The resolver's reading of an `mcp` binding, with the
//! bound on its identity, its egress against the bundle's binding minimum
//! and what a holding retains (SC1, SC4, MB4, CR1) are read from the same
//! kind.

use brokkr_core::realms::{CapabilityGrant, GrantRetention};

use super::{Authority, Cause, Connection, DialectKind, McpServer, ToolDialect, Unheld};
use crate::agents::EgressClass;

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
    /// A valid `mcp` dialect: its server is a broker's to carry, and no
    /// broker exists until slice two enables one (U9b).
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
    /// MB4: the dialect's own route does not meet the bundle's binding
    /// minimum, whatever clearance the harness's route has.
    #[error(
        "MCP dialect '{dialect}' has egress '{}' below binding minimum '{}'",
        .egress.name(),
        .minimum.name()
    )]
    Below {
        dialect: String,
        egress: EgressClass,
        minimum: EgressClass,
    },
}

impl Unserved {
    /// How the resolver holds this against an ask through `dialect`: SC4's
    /// unrepresentable identity refuses the compile whatever the strength,
    /// and every other cause is the dialect's incompatibility, which the
    /// ask's strength settles.
    pub(super) fn unheld(self, dialect: &ToolDialect) -> Unheld {
        match self {
            Unserved::Identity => Unheld::Identity,
            Unserved::Unbound(_)
            | Unserved::ArgvReference
            | Unserved::Url
            | Unserved::Below { .. } => {
                Unheld::Cause(Cause::incompatible(&dialect.name, self.to_string()))
            }
        }
    }
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
/// `realm`, or why it binds none: the compile fence's reading, and
/// `brokkr doctor`'s, under which an `mcp` binding has no broker until
/// U9b.
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
        Binding::Mcp(_) => Err(unbuilt(realm, capability, dialect)),
    }
}

fn unbuilt(realm: &str, capability: &str, dialect: &ToolDialect) -> Unbound {
    Unbound::Mcp {
        realm: realm.into(),
        capability: capability.into(),
        dialect: dialect.name.clone(),
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
    pub(super) fn of(dialect: &ToolDialect, grant: &CapabilityGrant) -> Retention {
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
    /// veto; a realm can never require retention. Its first production
    /// reader is manifest v12's effective retention (U5f), which lifts the
    /// `cfg`: until then nothing in a build reads it (ruling 6).
    #[cfg(test)]
    pub(super) fn effective(self) -> bool {
        match self.realm {
            GrantRetention::Veto => false,
            GrantRetention::Unreserved | GrantRetention::Inherit => self.declared,
        }
    }
}

impl Authority {
    /// This authority, judging every `mcp` dialect's egress against the
    /// bundle's binding `minimum` (MB4) rather than the absent default.
    pub(crate) fn with_minimum(mut self, minimum: EgressClass) -> Authority {
        self.minimum = minimum;
        self
    }

    /// The resolver's reading of the binding of `capability`, held as
    /// `tools`: the native pair a provider must carry, or why none can. An
    /// `mcp` binding answers in design D3 step 6's order — its identity,
    /// then an unexecutable connection's SC1 cause, then its egress against
    /// the bundle's binding minimum (MB4) — and, until U9b builds its
    /// broker, the fence's own words last. A native binding is judged as
    /// slice one judged it, whatever its dialect's egress. Reached only
    /// past the compile fence, which refuses every `mcp` grant first.
    pub(super) fn carried<'a>(
        &self,
        capability: &str,
        dialect: &'a ToolDialect,
        tools: &[String],
    ) -> Result<(&'a str, &'a str), Unserved> {
        let realm = &self.context.realm;
        let server = match bound(realm, capability, dialect)? {
            Binding::Native {
                provider,
                adapter_key,
            } => return Ok((provider, adapter_key)),
            Binding::Mcp(server) => server,
        };
        representable(capability, dialect, tools)?;
        Err(match &server.connection {
            Connection::Url(_) => Unserved::Url,
            Connection::Stdio(argv) if argv.iter().any(|part| references(part)) => {
                Unserved::ArgvReference
            }
            Connection::Stdio(_) if dialect.egress < self.minimum => Unserved::Below {
                dialect: dialect.name.clone(),
                egress: dialect.egress,
                minimum: self.minimum,
            },
            Connection::Stdio(_) => unbuilt(realm, capability, dialect).into(),
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
