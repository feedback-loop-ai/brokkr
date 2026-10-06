//! What one candidate's selected native holdings project onto its adapter
//! inventory (decision 0065 slice two, CC1 and SC4; design D8): the sealed
//! expectation of each known power, and the reverse attribution index of
//! each held tool.
//!
//! The inventory names the tools a harness is known to have
//! ([`NativeInventory::capability_of`]): it identifies a tool and grants
//! none. Only a holding the realm granted and this candidate selected puts
//! a tool in the reverse [`Attribution`] index, under the exact name the
//! holding admits after the grant narrowed it — never a prefix, a substring
//! or a clamped form, and never a neighbour's holding. A tool two held
//! capabilities both claim, or a held name that seat-record v6 could carry
//! only truncated, refuses the compile.
//!
//! [`NativeInventory::capability_of`]: super::NativeInventory::capability_of

use std::collections::BTreeMap;

use brokkr_protocol::native_controls::{HeldPower, NativeExpectation};

use super::binding::{representable, Unserved};
use super::{Authority, Holding, Implementation, NativeCapability};

/// What a known plan answers for, sealed from typed inputs before any
/// control is rendered (design D5.7): each known power is held where the
/// realm's holding reaches its adapter key, with that holding's admitted
/// tools and restriction object as written, and denied otherwise. A
/// measured default ON is held here though it emits no argument.
pub(super) fn expected(
    known: &BTreeMap<String, NativeCapability>,
    keys: &BTreeMap<String, String>,
    held: &BTreeMap<String, Holding>,
) -> NativeExpectation {
    NativeExpectation::Known {
        held: known
            .iter()
            .filter_map(|(key, native)| {
                keys.get(key).map(|capability| HeldPower {
                    capability: native.capability.clone(),
                    tools: held[capability].tools.clone(),
                    restrictions: held[capability].restrictions.clone(),
                })
            })
            .collect(),
        denied: known
            .iter()
            .filter(|(key, _)| !keys.contains_key(*key))
            .map(|(_, native)| native.capability.clone())
            .collect(),
    }
}

/// The held capability, and the dialect it is held through, that one
/// concrete tool is attributed to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Attributed {
    pub(crate) capability: String,
    pub(crate) dialect: String,
}

/// One candidate's reverse attribution index: every tool its native
/// holdings admit, by its exact name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Attribution(BTreeMap<String, Attributed>);

impl Attribution {
    /// The holding `tool` is attributed to, by exact name; `None` for a
    /// tool no selected holding admits, whatever the inventory knows of it.
    pub(crate) fn of(&self, tool: &str) -> Option<&Attributed> {
        self.0.get(tool)
    }
}

/// Why a candidate's native holdings cannot be attributed. Each text is
/// the compile's, under the site prefix.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Refusal {
    /// CC1: one tool, two held capabilities. Neither is chosen.
    #[error("tool '{tool}' maps to more than one held capability for provider '{provider}'")]
    Ambiguous { tool: String, provider: String },
    /// SC4: a held name v6 could carry only truncated. The tool is not
    /// echoed; the capability and dialect bound where it was held.
    #[error(
        "{identity} (capability '{capability}' through dialect '{dialect}')",
        identity = Unserved::Identity
    )]
    Unrepresentable { capability: String, dialect: String },
}

impl Authority {
    /// The reverse attribution index of the native holdings `provider`'s
    /// candidate selected, after every wanted drop: each holding's tools
    /// are bounded identifiers, judged as an `mcp` holding's are, and no
    /// tool is claimed twice. An `mcp` tool is told apart by its `cap-`
    /// server instead (CC1), so it never enters this index.
    pub(super) fn attribution(
        &self,
        provider: &str,
        held: &BTreeMap<String, Holding>,
    ) -> Result<Attribution, Refusal> {
        let mut index = Attribution::default();
        for (capability, holding) in held {
            match &holding.implementation {
                Implementation::Native { .. } => {}
                Implementation::Mcp { .. } => continue,
            }
            let dialect = self
                .dialect(capability)
                .expect("a held capability's dialect is loaded");
            representable(capability, dialect, &holding.tools).map_err(|_| {
                Refusal::Unrepresentable {
                    capability: capability.clone(),
                    dialect: holding.dialect.clone(),
                }
            })?;
            for tool in &holding.tools {
                if index.of(tool).is_some() {
                    return Err(Refusal::Ambiguous {
                        tool: tool.clone(),
                        provider: provider.to_string(),
                    });
                }
                let attributed = Attributed {
                    capability: capability.clone(),
                    dialect: holding.dialect.clone(),
                };
                index.0.insert(tool.clone(), attributed);
            }
        }
        Ok(index)
    }
}
