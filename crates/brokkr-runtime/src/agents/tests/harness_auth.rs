//! MB4's `harness_auth` through the real adapter loader (decision 0065
//! slice two, U7d3): the protocol's one parser reads the declaration, and
//! the loader joins its names to each route's `credentials` variable.

use super::*;

/// What a claude adapter declaring `auth` and `credentials` loads as: its
/// authentication names, or the loader's whole refusal with the file's
/// path written `<path>`.
fn loaded(auth: Option<Value>, credentials: Option<Value>) -> Result<Vec<String>, String> {
    let tree = Tree::new();
    let mut adapter = claude_body();
    if let Some(auth) = auth {
        adapter["harness_auth"] = auth;
    }
    if let Some(credentials) = credentials {
        adapter["credentials"] = credentials;
    }
    tree.write("adapters/claude.json", &adapter);
    let path = tree.adapters_root().join("claude.json");
    match Adapters::load(&tree.adapters_root()) {
        Ok(adapters) => Ok(adapters.adapter("claude").unwrap().harness_auth.clone()),
        Err(error) => Err((error.to_string()).replace(&path.display().to_string(), "<path>")),
    }
}

fn names(names: &[&str]) -> Result<Vec<String>, String> {
    Ok(names.iter().map(|name| name.to_string()).collect())
}

/// The declared names and every route credential load as one list, sorted
/// and each once; an absent declaration is the credentials alone, and an
/// adapter declaring neither reads no name.
#[test]
fn the_declared_names_load_beside_the_route_credentials_sorted_and_each_once() {
    let declared = json!({"names": ["ZETA_TOKEN", "ALPHA_KEY"], "measured": "a fixture"});
    let routes = json!({"one": "ROUTE_KEY", "two": "ROUTE_KEY", "three": "ALPHA_KEY"});
    let empty = json!({"names": [], "measured": "a fixture"});
    assert_eq!(
        [
            loaded(Some(declared.clone()), Some(routes.clone())),
            loaded(Some(declared), None),
            loaded(None, Some(routes)),
            loaded(Some(empty), None),
            loaded(None, None),
        ],
        [
            names(&["ALPHA_KEY", "ROUTE_KEY", "ZETA_TOKEN"]),
            names(&["ALPHA_KEY", "ZETA_TOKEN"]),
            names(&["ALPHA_KEY", "ROUTE_KEY"]),
            names(&[]),
            names(&[]),
        ]
    );
}

/// A malformed declaration refuses the adapter with the parser's own cause
/// behind the file and key it was read from.
#[test]
fn a_malformed_declaration_refuses_the_adapter_with_the_parsers_cause() {
    let refused = |cause: &str| Err(format!("adapter 'claude' (<path>) 'harness_auth' {cause}"));
    assert_eq!(
        [
            loaded(
                Some(json!({"names": ["claude_key"], "measured": "m"})),
                None
            ),
            loaded(Some(json!({"names": [], "measured": ""})), None),
        ],
        [
            refused("names 'claude_key', which does not match [A-Z][A-Z0-9_]*"),
            refused(
                "must be an object of exactly 'names' and a non-empty 'measured' ('measured' is \
                 empty)"
            ),
        ]
    );
}
