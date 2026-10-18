//! The steps of one adapter definition, each a group of its fields, in
//! the order `parse_adapter` reads them: the invocation, the served
//! models, the tool permissions, the MCP carriage, the pinning flags, the
//! hands and the fields the definition settles last. Every refusal stays
//! in the order it was written, so a file failing several checks reports
//! the same first one.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use brokkr_core::canonical::sha256_bytes;
use serde_json::{Map, Value};

use super::{
    adapter_egress, auth, capability, hands_notice, harness_hands, invalid, models, name_map,
    named, no_notice_without_workspace, object, only_keys, pin_flag, route_name, routes, string,
    string_array, trust_tier, EgressClass, HarnessHands, LibraryError, McpSupport, ToolPermissions,
    TrustTier, ROUTE_GRAMMAR,
};
use crate::agents::mcp::McpDecoded;

/// Who the provider is and how it is invoked, with the native inventory
/// its driver's harness judges.
pub(super) struct Invocation {
    pub(super) native: crate::capabilities::NativeInventory,
    pub(super) provider: String,
    pub(super) binary: String,
    pub(super) driver: Vec<String>,
}

pub(super) fn invocation(
    name: &str,
    map: &Map<String, Value>,
    path: &Path,
    what: &str,
) -> Result<Invocation, LibraryError> {
    // Every adapter is authority data: its native capabilities (decision
    // 0065 ruling 4) and its models' tiers (decision 0075 ruling
    // 5) alike, so a key written twice anywhere in the file is a refusal
    // rather than whichever copy came second.
    let text = std::fs::read_to_string(path)?;
    brokkr_core::canonical::parse_strict(&text)
        .map_err(|problem| LibraryError::Invalid(format!("{what}: {problem}")))?;
    let native = crate::capabilities::NativeInventory::parse(what, map.get("native_capabilities"))
        .map_err(LibraryError::Invalid)?;
    let provider = string(map, "provider", what)?;
    if provider != name {
        return invalid(format!(
            "{what} declares provider '{provider}' but the file is named \
             '{name}.json'; the file name is the provider name"
        ));
    }
    let binary = string(map, "binary", what)?;
    let driver = string_array(map, "driver", what)?;
    if driver.is_empty() {
        return invalid(format!("{what} 'driver' is empty; it is the invocation"));
    }
    // Rebuild unit 11: both declared halves parse under the grammar of the
    // harness this adapter dispatches, here, whether or not a realm uses
    // them (operator ruling 2).
    native
        .check_declared(what, crate::capabilities::harness_of(&driver))
        .map_err(LibraryError::Invalid)?;
    Ok(Invocation {
        native,
        provider,
        binary,
        driver,
    })
}

/// The abstract models the adapter serves, the ones it marks
/// provisional, and the judges among them.
pub(super) struct Served {
    pub(super) models: BTreeMap<String, String>,
    pub(super) provisional: BTreeSet<String>,
    pub(super) judges: Vec<String>,
}

pub(super) fn served(map: &Map<String, Value>, what: &str) -> Result<Served, LibraryError> {
    let (models, provisional) = models::models(map, what)?;
    let judges = match map.get("judges") {
        Some(_) => string_array(map, "judges", what)?,
        None => Vec::new(),
    };
    named(&judges, "judges", what)?;
    for judge in &judges {
        if !models.contains_key(judge) {
            return invalid(format!(
                "{what} declares judge '{judge}' but maps no model with that abstract name"
            ));
        }
    }
    Ok(Served {
        models,
        provisional,
        judges,
    })
}

/// The per-tool allow-list mapping, or the measured gap where there is
/// none.
pub(super) fn tool_permissions(
    map: &Map<String, Value>,
    what: &str,
) -> Result<(Option<ToolPermissions>, Option<String>), LibraryError> {
    // Three legal shapes, two outcomes. `"unsupported"` and a declared
    // gap BOTH yield `None` — the refusal in `compose` is identical, so
    // no seat gains power by the reason existing. What the reason buys
    // is that a future reader learns the gap was MEASURED against the
    // provider's CLI, not defaulted by whoever wrote the file first.
    let (tool_permissions, tool_permissions_gap) = match capability(map, "tool_permissions", what)?
    {
        None => (None, None),
        Some(value) => {
            let raw = object(value, &format!("{what} 'tool_permissions'"))?;
            if raw.contains_key("unsupported") {
                only_keys(raw, &["unsupported"], &format!("{what} 'tool_permissions'"))?;
                (
                    None,
                    Some(string(
                        raw,
                        "unsupported",
                        &format!("{what} 'tool_permissions'"),
                    )?),
                )
            } else {
                only_keys(
                    raw,
                    &["flag", "separator", "names"],
                    &format!("{what} 'tool_permissions'"),
                )?;
                (
                    Some(ToolPermissions {
                        flag: string(raw, "flag", &format!("{what} 'tool_permissions'"))?,
                        separator: string(raw, "separator", &format!("{what} 'tool_permissions'"))?,
                        names: name_map(raw, "names", &format!("{what} 'tool_permissions'"))?,
                    }),
                    None,
                )
            }
        }
    };
    Ok((tool_permissions, tool_permissions_gap))
}

/// The MCP carriage, decoded against the driver; admitted once the
/// hands and the resume assessment are read.
pub(super) fn mcp(
    map: &Map<String, Value>,
    what: &str,
    driver: &[String],
) -> Result<McpDecoded, LibraryError> {
    Ok(McpSupport::decode(
        what,
        capability(map, "mcp", what)?,
        driver,
    )?)
}

/// The install hint, and how a pin reaches the provider: the model flag,
/// the effort vocabulary and the routes that refuse every effort.
pub(super) struct Pinning {
    pub(super) hint: Option<String>,
    pub(super) model_flag: Option<String>,
    pub(super) efforts: Vec<String>,
    pub(super) effortless_routes: BTreeMap<String, String>,
}

pub(super) fn pinning(map: &Map<String, Value>, what: &str) -> Result<Pinning, LibraryError> {
    let hint = match map.get("hint") {
        None => None,
        Some(_) => Some(string(map, "hint", what)?),
    };
    let model_flag = pin_flag(map, "model_flag", what)?;
    // The effort vocabulary this provider declares (decision 0035
    // ruling 5), read beside the flag that expresses it. Required, and
    // required even when empty, for the reason `model_flag` is: an
    // absent declaration would read as "effortless" and quietly excuse
    // every seat this provider serves from the pin — which is the
    // implicit default ruling 1 exists to refuse.
    let efforts = string_array(map, "efforts", what)?;
    named(&efforts, "efforts", what)?;
    Ok(Pinning {
        hint,
        model_flag,
        efforts,
        effortless_routes: effortless_routes(map, what)?,
    })
}

/// Decision 0035 addendum 2026-09-11: routes measured as refusing
/// every reasoning level, mapped to the measurement that listed
/// them. Absent is no such route — an absent declaration excuses
/// nothing, which is the implicit-default refusal ruling 1 exists
/// for. Keys obey the route grammar, like `routes` and
/// `credentials`; values are non-empty evidence strings.
fn effortless_routes(
    map: &Map<String, Value>,
    what: &str,
) -> Result<BTreeMap<String, String>, LibraryError> {
    let effortless_routes = match map.get("effortless_routes") {
        None => BTreeMap::new(),
        Some(declared) => {
            let Some(entries) = declared.as_object() else {
                return invalid(format!(
                    "{what} 'effortless_routes' must be an object of route name → measurement"
                ));
            };
            let mut out = BTreeMap::new();
            for (route, value) in entries {
                if !route_name(route) {
                    return invalid(format!(
                        "{what} 'effortless_routes' names '{route}', which does not match {ROUTE_GRAMMAR}"
                    ));
                }
                match value.as_str() {
                    Some(text) if !text.is_empty() => {
                        out.insert(route.clone(), text.to_string());
                    }
                    _ => {
                        return invalid(format!(
                            "{what} 'effortless_routes.{route}' must be a non-empty string naming the measurement"
                        ))
                    }
                }
            }
            out
        }
    };
    Ok(effortless_routes)
}

/// How the provider puts its hands in the box, or why it cannot.
pub(super) struct Hands {
    pub(super) hands: Option<Vec<String>>,
    pub(super) hands_gap: Option<String>,
    pub(super) harness: HarnessHands,
    pub(super) hands_notice: Option<brokkr_protocol::adapters::HandsNotice>,
}

pub(super) fn hands(map: &Map<String, Value>, what: &str) -> Result<Hands, LibraryError> {
    // Decision 0043: how the provider puts its hands in the box, or the
    // measured reason it cannot. The same three legal shapes as
    // `tool_permissions`, for the same reason.
    // Absent is legal here, unlike `tool_permissions`: the key arrived
    // with decision 0043, and an adapter written before it — including
    // every `brokkr init` scaffold in the field — must keep compiling.
    // Absent reads as unsupported with no reason, fail-closed.
    let (hands, hands_gap, harness, hands_notice) =
        match map.get("hands").map(|_| capability(map, "hands", what)) {
            None | Some(Ok(None)) => (None, None, HarnessHands::default(), None),
            Some(Err(error)) => return Err(error),
            Some(Ok(Some(value))) => {
                let raw = object(value, &format!("{what} 'hands'"))?;
                if raw.contains_key("unsupported") {
                    no_notice_without_workspace(raw, what)?;
                    only_keys(raw, &["unsupported"], &format!("{what} 'hands'"))?;
                    (
                        None,
                        Some(string(raw, "unsupported", &format!("{what} 'hands'"))?),
                        HarnessHands::default(),
                        None,
                    )
                } else {
                    only_keys(
                        raw,
                        &["workspace", "harness", "notice"],
                        &format!("{what} 'hands'"),
                    )?;
                    let workspace = string_array(raw, "workspace", &format!("{what} 'hands'"))?;
                    let notice = hands_notice(raw, &workspace, what)?;
                    (Some(workspace), None, harness_hands(raw, what)?, notice)
                }
            }
        };
    Ok(Hands {
        hands,
        hands_gap,
        harness,
        hands_notice,
    })
}

/// The fields the definition settles after MCP admission, in the order
/// the adapter's fields read them: the trust tier, the egress class and
/// routes, the harness's authentication, the effort flag and the file's
/// digest.
pub(super) struct Settled {
    pub(super) trust_tier: TrustTier,
    pub(super) egress: EgressClass,
    pub(super) routes: BTreeMap<String, EgressClass>,
    pub(super) harness_auth: Vec<String>,
    pub(super) effort_flag: Option<String>,
    pub(super) digest: String,
}

pub(super) fn settled(
    map: &Map<String, Value>,
    credentials: &BTreeMap<String, String>,
    path: &Path,
    what: &str,
) -> Result<Settled, LibraryError> {
    Ok(Settled {
        trust_tier: trust_tier(map, what)?,
        egress: adapter_egress(map, what)?,
        routes: routes(map, what)?,
        harness_auth: auth::harness_auth(map, credentials, what)?,
        effort_flag: pin_flag(map, "effort_flag", what)?,
        digest: sha256_bytes(&std::fs::read(path)?),
    })
}
