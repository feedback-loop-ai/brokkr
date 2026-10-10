use serde_json::{json, Map, Value};

use super::admit;
use crate::bundle::compose_tests::{base_bundle, base_policy, derived, said, Library};
use crate::bundle::{Bundle, CompileError};

fn root(forced_crew: Option<Value>) -> Map<String, Value> {
    let mut document = json!({"name": "hedge", "extends": "fast"})
        .as_object()
        .unwrap()
        .clone();
    if let Some(value) = forced_crew {
        document.insert("forced_crew".to_string(), value);
    }
    document
}

/// The module's refusal, pinned once.
fn refusal(layer: &str) -> String {
    format!(
        "recipe {layer} declares 'forced_crew' without a reason; a forced crew names the \
         decision that forces it, as a non-empty string (decision 0041's addendum of \
         2026-10-07)"
    )
}

/// A root that forces nothing, or names its decision, is admitted.
#[test]
fn a_forced_crew_is_admitted_with_its_reason_or_absent() {
    for document in [
        root(None),
        root(Some(json!(
            "decision 0041 ruling 7: a wager forces its arm"
        ))),
    ] {
        assert_eq!(admit("hedge", &document).map_err(|e| e.to_string()), Ok(()));
    }
}

/// A `forced_crew` with no reason in it — empty, blank, or not a string —
/// is refused naming the layer, and the value is never echoed.
#[test]
fn a_forced_crew_without_a_reason_is_refused() {
    for value in [json!(""), json!("  \n"), json!(true), json!(["0041"])] {
        let refused = admit("hedge", &root(Some(value.clone())));
        let Err(CompileError::Invalid(reason)) = refused else {
            panic!("{value} must be refused, got {refused:?}");
        };
        assert_eq!(reason, refusal("'hedge'"), "{value}");
    }
}

/// The compile reaches the admission at a layer's root: a derived recipe
/// whose `forced_crew` states nothing is refused by name, and its control,
/// the same layer with a reason, compiles.
#[test]
fn the_compile_admits_forced_crew_at_a_layer_root() {
    let library = Library::new();
    library.recipe("base", &base_bundle(), Some(&base_policy()));
    let leaf = library.recipe("derived", &derived(json!({"forced_crew": " "})), None);
    assert_eq!(said(&leaf), format!("bundle: {}", refusal("'derived'")));
    let reason = json!({"forced_crew": "decision 0041 ruling 7"});
    library.recipe("derived", &derived(reason), None);
    let compiled = Bundle::compile(&leaf).unwrap_or_else(|e| panic!("the control compiles: {e}"));
    assert_eq!(compiled.name, "derived");
}
