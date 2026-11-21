//! A select seat's closed vocabulary, check by check through
//! `select_cases` (#353): each refusal is its exact variant, the checks
//! fire in their order, what passes returns the exact cases and default,
//! and every refusal's text is pinned once, by literals.

use super::*;
use serde_json::json;

/// A seat whose `select` is built from `(key, value)` entries, in order.
fn seat(entries: &[(&str, Value)]) -> Value {
    let select: Map<String, Value> = entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect();
    json!({ "select": select })
}

/// A `cases` object holding one empty body per named strategy.
fn cases(names: &[&str]) -> Value {
    Value::Object(
        names
            .iter()
            .map(|name| ((*name).to_owned(), json!({})))
            .collect(),
    )
}

fn on_strategy() -> (&'static str, Value) {
    ("on", json!("strategy"))
}

#[test]
fn an_absent_or_non_object_select_is_not_an_object() {
    for raw in [json!({}), json!({ "select": "strategy" })] {
        let refusal = SelectRefusal::NotObject { phase: "work" };
        assert_eq!(select_cases("work", &raw), Err(refusal));
    }
}

#[test]
fn an_unknown_key_is_named() {
    let raw = seat(&[on_strategy(), ("invented", json!(1))]);
    let refusal = SelectRefusal::UnknownKey {
        phase: "work",
        key: "invented",
    };
    assert_eq!(select_cases("work", &raw), Err(refusal));
}

#[test]
fn an_absent_non_string_or_unknown_on_is_refused() {
    let all = cases(&SELECT_STRATEGIES);
    let ons = [
        (None, "<missing>"),
        (Some(json!(7)), "<missing>"),
        (Some(json!("weather")), "weather"),
    ];
    for (on, named) in ons {
        let mut entries = vec![("cases", all.clone())];
        entries.extend(on.map(|on| ("on", on)));
        let refusal = SelectRefusal::UnknownOn {
            phase: "work",
            on: named,
        };
        assert_eq!(select_cases("work", &seat(&entries)), Err(refusal));
    }
}

#[test]
fn absent_or_non_object_cases_are_refused() {
    let absent = seat(&[on_strategy()]);
    let list = seat(&[on_strategy(), ("cases", json!([]))]);
    for raw in [absent, list] {
        let refusal = SelectRefusal::CasesNotObject { phase: "work" };
        assert_eq!(select_cases("work", &raw), Err(refusal));
    }
}

#[test]
fn an_unknown_case_is_named() {
    let raw = seat(&[on_strategy(), ("cases", cases(&["chore", "escalate"]))]);
    let refusal = SelectRefusal::UnknownCase {
        phase: "work",
        case: "escalate",
    };
    assert_eq!(select_cases("work", &raw), Err(refusal));
}

#[test]
fn the_first_unresolved_strategy_is_named_in_order() {
    let raw = seat(&[on_strategy(), ("cases", cases(&["chore"]))]);
    let refusal = SelectRefusal::Unresolved {
        phase: "work",
        strategy: "feature",
    };
    assert_eq!(select_cases("work", &raw), Err(refusal));
}

#[test]
fn the_checks_fire_in_their_order() {
    let unknown_key_without_on = seat(&[("cases", cases(&["chore"])), ("invented", json!(1))]);
    assert_eq!(
        select_cases("work", &unknown_key_without_on),
        Err(SelectRefusal::UnknownKey {
            phase: "work",
            key: "invented",
        })
    );
    let unknown_case_and_unresolved = seat(&[on_strategy(), ("cases", cases(&["escalate"]))]);
    assert_eq!(
        select_cases("work", &unknown_case_and_unresolved),
        Err(SelectRefusal::UnknownCase {
            phase: "work",
            case: "escalate",
        })
    );
}

#[test]
fn an_admitted_select_returns_its_exact_cases_and_default() {
    let every = cases(&SELECT_STRATEGIES);
    let raw = seat(&[on_strategy(), ("cases", every.clone())]);
    assert_eq!(
        select_cases("work", &raw),
        Ok((every.as_object().unwrap(), None))
    );

    let one = cases(&["design"]);
    let body = json!({ "agent": "fallback" });
    let raw = seat(&[
        on_strategy(),
        ("cases", one.clone()),
        ("default", body.clone()),
    ]);
    assert_eq!(
        select_cases("work", &raw),
        Ok((one.as_object().unwrap(), Some(&body)))
    );

    let none = cases(&[]);
    let raw = seat(&[
        on_strategy(),
        ("cases", none.clone()),
        ("default", Value::Null),
    ]);
    assert_eq!(
        select_cases("work", &raw),
        Ok((none.as_object().unwrap(), Some(&Value::Null)))
    );
}

#[test]
fn every_select_refusal_reads_as_it_always_has() {
    let phase = "work";
    let texts = [
        (
            SelectRefusal::NotObject { phase },
            "seat 'work' select must be an object",
        ),
        (
            SelectRefusal::UnknownKey { phase, key: "invented" },
            "seat 'work' select has unknown key 'invented'; known: on, cases, default",
        ),
        (
            SelectRefusal::UnknownOn { phase, on: "weather" },
            "seat 'work' select has unknown 'on' weather; known: strategy — the selection vocabulary is closed",
        ),
        (
            SelectRefusal::CasesNotObject { phase },
            "seat 'work' select.cases must be an object",
        ),
        (
            SelectRefusal::UnknownCase { phase, case: "escalate" },
            "seat 'work' select has unknown strategy case 'escalate'; known: chore, feature, design, engine",
        ),
        (
            SelectRefusal::Unresolved { phase, strategy: "engine" },
            "strategy class 'engine' resolves to nothing in seat 'work'; add that case or a default",
        ),
    ];
    for (refusal, text) in texts {
        assert_eq!(refusal.to_string(), text);
    }
}
