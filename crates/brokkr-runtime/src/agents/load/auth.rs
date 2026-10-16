//! What an adapter says its harness authenticates with: each route's
//! credential variable (decision 0036 ruling 5) and the names its
//! `harness_auth` declaration measured (decision 0065 slice two, MB4). The
//! two are read together into the one list an MCP secret binding is judged
//! against, so a route's key is written once, in `credentials`.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use super::{invalid, route_name, secret_name, LibraryError, ROUTE_GRAMMAR, SECRET_NAME_GRAMMAR};

/// A provider's declared credential variables (decision 0036 ruling 5):
/// route name → the environment variable that route's endpoint needs.
/// A NAME, never a value — the same rule the whole tree obeys, enforced
/// beside it by `refuse_secret_stores`.
pub(super) fn credentials(
    map: &Map<String, Value>,
    what: &str,
) -> Result<BTreeMap<String, String>, LibraryError> {
    let Some(declared) = map.get("credentials") else {
        return Ok(BTreeMap::new());
    };
    let Some(entries) = declared.as_object() else {
        return invalid(format!(
            "{what} 'credentials' must be an object of route name → environment \
             variable name"
        ));
    };
    let mut out = BTreeMap::new();
    for (route, value) in entries {
        if !route_name(route) {
            return invalid(format!(
                "{what} 'credentials' names '{route}', which does not match \
                 {ROUTE_GRAMMAR}"
            ));
        }
        match value.as_str().filter(|name| secret_name(name)) {
            Some(variable) => out.insert(route.clone(), variable.to_string()),
            None => {
                return invalid(format!(
                    "{what} 'credentials.{route}' is {value}; a credential is the \
                     NAME of an environment variable ({SECRET_NAME_GRAMMAR}), \
                     never a value"
                ))
            }
        };
    }
    Ok(out)
}

/// MB4: every environment variable name this adapter's harness reads to
/// authenticate, sorted and each once — the names its `harness_auth`
/// declaration measured, read by the protocol's one parser, and each
/// route's `credentials` variable. An absent declaration adds no name to
/// the credentials, as an adapter written before MB4 (every `brokkr init`
/// scaffold among them) must keep loading.
pub(super) fn harness_auth(
    map: &Map<String, Value>,
    credentials: &BTreeMap<String, String>,
    what: &str,
) -> Result<Vec<String>, LibraryError> {
    let declared = match map.get("harness_auth") {
        None => Vec::new(),
        Some(value) => brokkr_protocol::adapters::harness_auth(value)
            .map_err(|problem| LibraryError::Invalid(format!("{what} 'harness_auth' {problem}")))?,
    };
    let names: BTreeSet<String> = declared
        .into_iter()
        .chain(credentials.values().cloned())
        .collect();
    Ok(names.into_iter().collect())
}
