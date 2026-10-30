//! The hands-access mark a boxed site carries (#896 round 2).

use brokkr_core::realms::{Boundary, BOUNDARIES};
use brokkr_protocol::adapters::ACCESS;
use brokkr_protocol::hands::HandsSpec;
use brokkr_protocol::native_controls::Access;
use serde_json::{json, Value};

use super::super::tests::{engine, set_site_hands, single_body};
use super::super::Engine;
use crate::bundle::{HandsState, SiteFacts};

/// The access `engine` marks site `work` with for a `gate` or not, over an
/// input that carried a stale one.
fn mark(engine: &Engine, gate: bool) -> Option<Value> {
    let mut input = json!({ACCESS: "stale", "context": {}});
    engine.marks().access("work", gate, &mut input);
    input.get(ACCESS).cloned()
}

/// A boxed site with hands is marked from the access its office declared,
/// or from the gate class where no office declared one; a site without
/// hands, unknown or unregistered, or one Brokkr does not box, carries no
/// access, and the stale carrier is gone.
#[test]
fn only_a_boxed_site_with_hands_carries_its_access() {
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
                [Some(json!("read-only")), Some(json!("writes"))]
            }
            Boundary::Harness | Boundary::Open => [None, None],
        };
        let marked = [mark(&engine, true), mark(&engine, false)];
        assert_eq!(marked, expected, "{boundary}");
    }
    // Every model site's spawn-time marks carry the office's own access,
    // not the class: a read-only office hired at work class is read-only.
    engine.boundary = Boundary::Namespace;
    engine.bundle.sites.get_mut("work").unwrap().access = Some(Access::ReadOnly);
    let mut input = json!({"context": {}});
    engine.marks().site("work", false, None, &mut input);
    assert_eq!(input[ACCESS], json!("read-only"));
    engine.bundle.sites.get_mut("work").unwrap().access = Some(Access::Writes);
    let mut input = json!({"context": {}});
    engine.marks().site("work", true, None, &mut input);
    assert_eq!(input[ACCESS], json!("writes"));
}
