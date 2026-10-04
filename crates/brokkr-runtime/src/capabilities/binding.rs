//! What a granted capability is bound to, projected from its dialect's
//! kind (decision 0065 slice two, SC5). The kind is the one home of the
//! binding: a native dialect names its provider and adapter key, and an
//! `mcp` or reserved `hands` dialect names none. Nothing here indexes a
//! map, expects a native pair, invents an empty provider or reads another
//! kind as native. Every caller gets the native pair or the typed reason
//! there is none.

use super::{Authority, DialectKind, ToolDialect};

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

/// The native `(provider, adapter key)` `dialect` binds `capability` to in
/// `realm`, or why it binds none: one arm per kind, with no wildcard.
pub(super) fn bound<'a>(
    realm: &str,
    capability: &str,
    dialect: &'a ToolDialect,
) -> Result<(&'a str, &'a str), Unbound> {
    let (realm, capability, name) = (realm.into(), capability.into(), dialect.name.clone());
    match &dialect.kind {
        DialectKind::Native {
            provider,
            adapter_key,
        } => Ok((provider, adapter_key)),
        DialectKind::Mcp => Err(Unbound::Mcp {
            realm,
            capability,
            dialect: name,
        }),
        DialectKind::Hands => Err(Unbound::Hands {
            realm,
            capability,
            dialect: name,
        }),
    }
}

impl Authority {
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
        let (provider, adapter_key) = bound(&self.context.realm, capability, dialect)?;
        Ok(Native {
            dialect,
            provider,
            adapter_key,
        })
    }
}
