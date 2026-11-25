//! Realm-wide validation of each granted tool dialect.

use std::collections::BTreeMap;
use std::path::Path;

use brokkr_core::realms::{CapabilityGrant, GrantDialect};
use serde_json::Value;
use thiserror::Error;

use super::{
    sorted, violation, Authority, CapabilityContext, Definition, Definitions, DialectKind,
    McpFence, ToolDialect, ABSENT_EGRESS_MINIMUM,
};

#[derive(Debug, Error)]
enum Refusal {
    #[error("realm '{realm}': capability '{capability}': {problem}")]
    Load {
        realm: String,
        capability: String,
        problem: String,
    },
    #[error(
        "realm '{realm}' grants capability '{capability}' through dialect '{dialect}', \
         which serves '{serves}'"
    )]
    Serves {
        realm: String,
        capability: String,
        dialect: String,
        serves: String,
    },
    #[error(
        "realm '{realm}' grants capability '{capability}' through dialect '{dialect}' under \
         provider key '{key}', but the dialect binds provider '{provider}'"
    )]
    Provider {
        realm: String,
        capability: String,
        dialect: String,
        key: String,
        provider: String,
    },
    #[error(
        "realm '{realm}': capability '{capability}' in dialect '{dialect}' declares \
         classes [{declared}], conflicting with abstract definition '{source_path}' classes [{defined}]"
    )]
    Classes {
        realm: String,
        capability: String,
        dialect: String,
        declared: Box<str>,
        source_path: Box<str>,
        defined: Box<str>,
    },
    #[error(
        "realm '{realm}' grants capability '{capability}' tool '{tool}', which \
         dialect '{dialect}' does not name; its tools are [{named}]"
    )]
    Tool {
        realm: String,
        capability: String,
        dialect: String,
        tool: String,
        named: String,
    },
    #[error(
        "realm '{realm}' grants capability '{capability}' through dialect '{dialect}' with \
         an invalid restriction {problem}"
    )]
    Restriction {
        realm: String,
        capability: String,
        dialect: String,
        problem: String,
    },
}

/// A grant and the abstract definition against which all its dialects are judged.
struct Judgment<'a> {
    root: &'a Path,
    realm: &'a str,
    capability: &'a str,
    definition: &'a Definition,
    grant: &'a CapabilityGrant,
}

impl Judgment<'_> {
    fn load(&self, name: &str, key: Option<&str>) -> Result<ToolDialect, Refusal> {
        let Self {
            root,
            realm,
            capability,
            definition,
            grant,
        } = *self;
        // Which keys the grant reserves is its map version's (SC2).
        let failed = |problem| Refusal::Load {
            realm: realm.into(),
            capability: capability.into(),
            problem,
        };
        let dialect = ToolDialect::load(root, name).map_err(failed)?;
        dialect
            .reserves(grant.retention)
            .map_err(|collision| failed(collision.to_string()))?;
        if dialect.serves != capability {
            return Err(Refusal::Serves {
                realm: realm.into(),
                capability: capability.into(),
                dialect: dialect.name,
                serves: dialect.serves,
            });
        }
        if let Some((key, provider)) = key.zip(native_provider(&dialect)) {
            if key != provider {
                return Err(Refusal::Provider {
                    realm: realm.into(),
                    capability: capability.into(),
                    dialect: name.into(),
                    key: key.into(),
                    provider: provider.into(),
                });
            }
        }
        if let Some(classes) = &dialect.classes {
            if sorted(classes) != sorted(&definition.classes) {
                return Err(Refusal::Classes {
                    realm: realm.into(),
                    capability: capability.into(),
                    dialect: dialect.name,
                    declared: classes.join(", ").into_boxed_str(),
                    source_path: definition.source.as_str().into(),
                    defined: definition.classes.join(", ").into_boxed_str(),
                });
            }
        }
        for tool in grant.tools.iter().flatten() {
            if !dialect.tools.contains(tool) {
                return Err(Refusal::Tool {
                    realm: realm.into(),
                    capability: capability.into(),
                    dialect: dialect.name,
                    tool: tool.clone(),
                    named: dialect.tools.join(", "),
                });
            }
        }
        violation(
            &dialect.restrictions,
            &Value::Object(grant.restrictions.clone()),
        )
        .map_err(|problem| Refusal::Restriction {
            realm: realm.into(),
            capability: capability.into(),
            dialect: dialect.name.clone(),
            problem,
        })?;
        Ok(dialect)
    }
}

fn native_provider(dialect: &ToolDialect) -> Option<&str> {
    match &dialect.kind {
        DialectKind::Native { provider, .. } => Some(provider),
        DialectKind::Mcp(_) | DialectKind::Hands => None,
    }
}

impl Authority {
    /// Every grant of the operated realm validated BEFORE any seat is
    /// looked at (design D4 steps 1 and 2). What [`Authority::load`]
    /// refuses realm-wide follows it.
    pub(super) fn validated(context: CapabilityContext) -> Result<Authority, String> {
        let realm = &context.realm;
        let definitions = Definitions::load(&context.root)?;
        let mut dialects = BTreeMap::new();
        let mut provider_dialects = BTreeMap::new();
        for (capability, grant) in &context.grants {
            let definition = definitions.get(capability).ok_or_else(|| {
                format!(
                    "realm '{realm}': capability '{capability}' has no abstract definition at \
                     '{}' in the operator configuration; declare its classes before granting it",
                    Definition::source_of(capability)
                )
            })?;
            let judgment = Judgment {
                root: &context.root,
                realm,
                capability,
                definition,
                grant,
            };
            let display = |refusal: Refusal| refusal.to_string();
            match &grant.dialect {
                GrantDialect::Single(name) => {
                    dialects.insert(
                        capability.clone(),
                        judgment.load(name, None).map_err(display)?,
                    );
                }
                GrantDialect::PerProvider(names) => {
                    let loaded = names
                        .iter()
                        .map(|(provider, name)| {
                            judgment
                                .load(name, Some(provider))
                                .map(|dialect| (provider.clone(), dialect))
                        })
                        .collect::<Result<_, _>>()
                        .map_err(display)?;
                    provider_dialects.insert(capability.clone(), loaded);
                }
            }
        }
        Ok(Authority {
            context,
            definitions,
            dialects,
            provider_dialects,
            minimum: ABSENT_EGRESS_MINIMUM,
            fence: McpFence::current(),
        })
    }

    /// Every capability and dialect the compiled realm grants, across both forms.
    pub(super) fn all_dialects(&self) -> impl Iterator<Item = (&str, &ToolDialect)> {
        let single = self
            .dialects
            .iter()
            .map(|(capability, dialect)| (capability.as_str(), dialect));
        single.chain(
            self.provider_dialects
                .iter()
                .flat_map(|(capability, dialects)| {
                    dialects
                        .values()
                        .map(move |dialect| (capability.as_str(), dialect))
                }),
        )
    }
}
