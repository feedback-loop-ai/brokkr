//! The seat-class mark a boxed site's hands carry (#896).

use brokkr_core::realms::{Boundary, BOUNDARIES};
use brokkr_protocol::adapters::SEAT_CLASS;
use brokkr_protocol::hands::HandsSpec;
use serde_json::{json, Value};

use super::super::tests::{engine, set_site_hands, single_body};
use super::super::Engine;
use crate::bundle::{HandsState, SiteFacts};

/// The class `engine` marks site `work` with for a `gate` or not, over an
/// input that carried a stale one.
fn mark(engine: &Engine, gate: bool) -> Option<Value> {
    let mut input = json!({SEAT_CLASS: "stale", "context": {}});
    engine.marks().class("work", gate, &mut input);
    input.get(SEAT_CLASS).cloned()
}

/// A boxed site with hands is marked `gate` or `work`; a site without
/// hands, unknown or unregistered, or one Brokkr does not box, carries no
/// class, and the stale carrier is gone.
#[test]
fn only_a_boxed_site_with_hands_carries_its_seat_class() {
    let (_dir, mut engine) = engine(single_body(vec!["d".into()]));
    let mut rows = Vec::new();
    for facts in [None, Some(HandsState::Unknown), Some(HandsState::NoHands)] {
        match facts {
            None => {
                engine.bundle.sites.remove("work");
            }
            Some(hands) => {
                let facts = SiteFacts {
                    hands,
                    ..Default::default()
                };
                engine.bundle.sites.insert("work".into(), facts);
            }
        }
        for boundary in BOUNDARIES {
            engine.boundary = boundary;
            rows.push([mark(&engine, true), mark(&engine, false)]);
        }
    }
    assert_eq!(rows, vec![[None, None]; 3 * BOUNDARIES.len()]);

    set_site_hands(&mut engine.bundle, "work", HandsSpec::default());
    for boundary in BOUNDARIES {
        engine.boundary = boundary;
        let expected = match boundary {
            Boundary::Namespace | Boundary::Seatbelt | Boundary::Container => {
                [Some(json!("gate")), Some(json!("work"))]
            }
            Boundary::Harness | Boundary::Open => [None, None],
        };
        let marked = [mark(&engine, true), mark(&engine, false)];
        assert_eq!(marked, expected, "{boundary}");
    }
    // Every model site's spawn-time marks carry it.
    engine.boundary = Boundary::Namespace;
    let mut input = json!({"context": {}});
    engine.marks().site("work", true, None, &mut input);
    assert_eq!(input[SEAT_CLASS], json!("gate"));
}
