//! Realm-wide validation of each granted tool dialect.

use std::collections::BTreeMap;

use brokkr_core::realms::GrantDialect;
use serde_json::Value;

use super::{
    sorted, violation, Authority, CapabilityContext, Definition, Definitions, McpFence,
    ToolDialect, ABSENT_EGRESS_MINIMUM,
};

impl Authority {
    /// Every grant of the operated realm validated BEFORE any seat is
    /// looked at (design D4 steps 1 and 2): the definition exists, the
    /// dialect loads, serves the capability and agrees on its classes, the
    /// tool subset is a subset, the restriction keys pass the dialect's
    /// schema. What [`Authority::load`] refuses realm-wide follows it.
    pub(super) fn validated(context: CapabilityContext) -> Result<Authority, String> {
        let realm = &context.realm;
        let definitions = Definitions::load(&context.root)?;
        let mut dialects = BTreeMap::new();
        for (capability, grant) in &context.grants {
            let definition = definitions.get(capability).ok_or_else(|| {
                format!(
                    "realm '{realm}': capability '{capability}' has no abstract definition at \
                     '{}' in the operator configuration; declare its classes before granting it",
                    Definition::source_of(capability)
                )
            })?;
            // Which keys the grant reserves is its map version's (SC2).
            let name = match &grant.dialect {
                GrantDialect::Single(name) => name,
                GrantDialect::PerProvider(_) => return Err(format!(
                    "realm '{realm}': capability '{capability}': per-provider tool dialects are not yet supported"
                )),
            };
            let dialect = ToolDialect::load(&context.root, name)
                .and_then(|dialect| {
                    let reserved = dialect.reserves(grant.retention);
                    reserved
                        .map(|()| dialect)
                        .map_err(|collision| collision.to_string())
                })
                .map_err(|problem| {
                    format!("realm '{realm}': capability '{capability}': {problem}")
                })?;
            if dialect.serves != *capability {
                return Err(format!(
                    "realm '{realm}' grants capability '{capability}' through dialect '{}', \
                     which serves '{}'",
                    dialect.name, dialect.serves
                ));
            }
            if let Some(classes) = &dialect.classes {
                if sorted(classes) != sorted(&definition.classes) {
                    return Err(format!(
                        "realm '{realm}': capability '{capability}' in dialect '{}' declares \
                         classes [{}], conflicting with abstract definition '{}' classes [{}]",
                        dialect.name,
                        classes.join(", "),
                        definition.source,
                        definition.classes.join(", ")
                    ));
                }
            }
            for tool in grant.tools.iter().flatten() {
                if !dialect.tools.contains(tool) {
                    return Err(format!(
                        "realm '{realm}' grants capability '{capability}' tool '{tool}', which \
                         dialect '{}' does not name; its tools are [{}]",
                        dialect.name,
                        dialect.tools.join(", ")
                    ));
                }
            }
            violation(
                &dialect.restrictions,
                &Value::Object(grant.restrictions.clone()),
            )
            .map_err(|problem| {
                format!(
                    "realm '{realm}' grants capability '{capability}' through dialect '{}' with \
                     an invalid restriction {problem}",
                    dialect.name
                )
            })?;
            dialects.insert(capability.clone(), dialect);
        }
        Ok(Authority {
            context,
            definitions,
            dialects,
            minimum: ABSENT_EGRESS_MINIMUM,
            fence: McpFence::current(),
        })
    }
}
