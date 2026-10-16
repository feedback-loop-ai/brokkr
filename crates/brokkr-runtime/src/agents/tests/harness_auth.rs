//! MB4's `harness_auth` through the real adapter loader (decision 0065
//! slice two, U7d3): the protocol's one parser reads the declaration, and
//! the loader joins its names to each route's `credentials` variable.

use super::*;

/// What a claude adapter declaring `auth` and `credentials` loads as: its
/// authentication names or the loader's typed refusal, beside the file's
/// path that refusal is written against.
fn loaded(
    auth: Option<Value>,
    credentials: Option<Value>,
) -> (Result<Vec<String>, LibraryError>, String) {
    let tree = Tree::new();
    let mut adapter = claude_body();
    if let Some(auth) = auth {
        adapter["harness_auth"] = auth;
    }
    if let Some(credentials) = credentials {
        adapter["credentials"] = credentials;
    }
    tree.write("adapters/claude.json", &adapter);
    let root = tree.adapters_root();
    let path = root.join("claude.json").display().to_string();
    let outcome = Adapters::load(&root)
        .map(|adapters| adapters.adapter("claude").unwrap().harness_auth.clone());
    (outcome, path)
}

/// The names a loadable declaration reads.
fn accepted(auth: Option<Value>, credentials: Option<Value>) -> Vec<String> {
    loaded(auth, credentials).0.unwrap()
}

/// The diagnostic a malformed declaration is refused with, its path
/// written `<path>`; any variant but `LibraryError::Invalid` fails.
fn invalid(auth: Value) -> String {
    match loaded(Some(auth), None) {
        (Err(LibraryError::Invalid(text)), path) => text.replace(&path, "<path>"),
        (other, _) => panic!("expected LibraryError::Invalid, got {other:?}"),
    }
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
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
            accepted(Some(declared.clone()), Some(routes.clone())),
            accepted(Some(declared), None),
            accepted(None, Some(routes)),
            accepted(Some(empty), None),
            accepted(None, None),
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

/// A malformed declaration refuses the adapter as `LibraryError::Invalid`
/// with the parser's own cause behind the file and key it was read from.
#[test]
fn a_malformed_declaration_refuses_the_adapter_with_the_parsers_cause() {
    let refused = |cause: &str| format!("adapter 'claude' (<path>) 'harness_auth' {cause}");
    assert_eq!(
        [
            invalid(json!({"names": ["claude_key"], "measured": "m"})),
            invalid(json!({"names": [], "measured": ""})),
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
