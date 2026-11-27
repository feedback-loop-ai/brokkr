use serde_json::{json, Value};

use super::*;
use crate::native_controls::managed;

pub(crate) const OFF_ROW: &str = "- id: tool-web\n  config:\n    fetch: false\n    search: false\n";

pub(crate) fn plan(fetch: bool, search: bool) -> Value {
    let mut held = Vec::new();
    let mut denied = Vec::new();
    let mut admits = serde_json::Map::new();
    let overlay = [Overlay::DshWebFetch(fetch), Overlay::DshWebSearch(search)];
    for (key, tool) in overlay.into_iter().zip(["web_fetch", "web_search"]) {
        if key.enabled() {
            held.push(key.capability());
            admits.insert(key.capability().into(), json!([tool]));
        } else {
            denied.push(key.capability());
        }
    }
    json!({"inventory": "known", "provider": "dsh", "harness": "dsh",
           "on": held, "off": denied, "admits": admits, "argv": [], "guards": [],
           "overlay": overlay})
}

pub(crate) fn denied() -> Controls {
    managed(&json!({"native_controls": plan(false, false)}))
        .unwrap()
        .unwrap()
}

#[test]
fn only_a_complete_consistent_dsh_overlay_composes() {
    use crate::native_controls::compose_for_provider;

    for overlay in [
        vec![],
        vec![Overlay::DshWebSearch(false); 2],
        vec![Overlay::DshWebFetch(true), Overlay::DshWebSearch(false)],
    ] {
        let controls = Controls {
            overlay,
            ..denied()
        };
        assert_eq!(
            compose_for_provider("dsh", &[], &[], &controls),
            Err(unanswered())
        );
    }
    assert_eq!(
        compose_for_provider("exec", &[], &[], &denied()),
        Err(unconsumed("exec", "a DSH overlay"))
    );
}

#[test]
fn an_overlay_with_an_unknown_key_is_refused_at_the_plan_edge() {
    let raw = json!([{"web-search-deepseek": true}]);
    assert_eq!(
        decode(Some(&raw)).unwrap_err().classify(),
        serde_json::error::Category::Data
    );
}
