//! `resume.rs`'s own unit tests, beside it so the production file holds
//! its line ceiling.

use super::*;

fn site(site: Option<&str>, key: SiteKey) -> StructuralSite {
    StructuralSite {
        site: site.map(str::to_string),
        key,
        class: SeatClass::Work,
        command: vec!["driver".into()],
    }
}

/// The refusal names both sites, in each shape a site can take. The
/// engine's own compile-time check exercises the nested-member pair;
/// these are the other three, and a refusal that could not name a
/// step or a single body would send an operator hunting.
#[test]
fn a_collision_names_both_sites_whatever_shape_they_are() {
    for (left, right, expected) in [
        (
            SiteKey::single("work", None),
            SiteKey::panel_member("work", None, "alpha", 0),
            "the single body of seat 'work' and panel member 'alpha' of seat 'work'",
        ),
        (
            SiteKey::sequence_step("work", None, "author", 0),
            SiteKey::sequence_panel_member("work", None, "review", 1, "alpha", 0),
            "step 'author' of seat 'work' and member 'alpha' of step 'review' of seat 'work'",
        ),
    ] {
        let (label, both) =
            flat_address_collision(&[site(Some("a"), left), site(Some("a"), right)])
                .expect("two different sites under one label collide");
        assert_eq!(label, "a");
        assert_eq!(both, expected);
    }

    // The same site listed twice under one label is not a collision:
    // one site cannot alias itself, and nothing selects wrongly.
    assert!(flat_address_collision(&[
        site(Some("a"), SiteKey::single("work", None)),
        site(Some("a"), SiteKey::single("work", None)),
    ])
    .is_none());
    // Nor are distinct labels, whatever the sites are.
    assert!(flat_address_collision(&[
        site(None, SiteKey::single("work", None)),
        site(Some("a"), SiteKey::panel_member("work", None, "a", 0)),
    ])
    .is_none());
}

/// The stamps are for RECORDS, and a record is an object. Every fold
/// in this tree emits one, but the store's fence judges third-party
/// driver checkpoints too — so a non-object passes through untouched
/// rather than being wrapped into something that looks stamped.
#[test]
fn a_non_object_checkpoint_is_neither_stamped_nor_reshaped() {
    let context = SiteContext::new("a".repeat(64), "b".repeat(64), SeatClass::Work);
    for record in [json!("prose"), json!(7), json!([1, 2]), Value::Null] {
        assert_eq!(context.stamp(record.clone()), record);
        assert_eq!(unstamped(record.clone()), record);
    }
    // And an object that names no model carries neither stamp, even
    // when a driver wrote one itself.
    let forged = json!({"step": "seat-turn", "site_ref": "c".repeat(64)});
    assert_eq!(context.stamp(forged.clone()), json!({"step": "seat-turn"}));
    assert_eq!(unstamped(forged), json!({"step": "seat-turn"}));
}

/// The offered root's history at a site (U4f; CC1) is every attempt whose
/// row there confirmed that root, and every call id those attempts' rows
/// there carry. Another root's attempt, another site's row and an
/// unowned row add nothing, and a site offered nothing has no history.
#[test]
fn an_offered_roots_history_is_its_own_attempts_calls_at_its_own_site() {
    let (site, other) = ("a".repeat(64), "c".repeat(64));
    let row = |attempt: Option<&str>, site: &str, fields: Value| {
        let mut checkpoint = json!({"step": "seat-turn", "site_ref": site});
        let fields = fields.as_object().unwrap().clone();
        checkpoint.as_object_mut().unwrap().extend(fields);
        let payload = json!({"checkpoint": checkpoint});
        crate::envelope_builder::EnvelopeBuilder::new(EventType::EffectCheckpointed, payload)
            .attempt(attempt)
            .at("2026-10-06T00:00:00Z")
            .hash("a".repeat(64))
            .build()
    };
    let root = |id: &str| json!({"root_session": {"id": id, "persistent": true}});
    let call = |id: &str| json!({"call_id": id});
    let events = [
        row(Some("first"), &site, root("r")),
        row(Some("first"), &site, call("n-first")),
        row(Some("second"), &site, root("r")),
        row(Some("second"), &site, call("n-second")),
        row(Some("other-root"), &site, root("s")),
        row(Some("other-root"), &site, call("n-other-root")),
        row(Some("elsewhere"), &other, root("r")),
        row(Some("first"), &other, call("n-other-site")),
        row(None, &site, call("n-unowned")),
    ];
    let offer = ResumeTarget {
        provider_id: "r".into(),
        persistence_locator: None,
        persistence_home: None,
    };
    let context = SiteContext::new(site, "b".repeat(64), SeatClass::Work);
    let history = RootHistory {
        attempts: BTreeSet::from(["first".into(), "second".into()]),
        calls: BTreeSet::from(["n-first".into(), "n-second".into()]),
    };
    let offered = context.clone().offered(&events, Some(&offer));
    assert_eq!(offered.history, history);
    assert_eq!(
        context.offered(&events, None).history,
        RootHistory::default()
    );
}

/// The private context carries the two things a two-coordinate
/// provider needs and nothing it does not: the harness facts of the
/// offered root (version and the optional wrapper digest) and the
/// owned target's provider ID, persistence locator and recorded home.
/// With no offer, no target and no originating digest travel — only
/// the assessment, exactly as before.
#[test]
fn the_private_context_carries_the_owned_target_and_originating_digest() {
    let originating = OriginatingRoot {
        harness_version: Some("0.1.5-rc.1".into()),
        wrapper_digest: Some("a".repeat(64)),
    };
    let target = ResumeTarget {
        provider_id: "session-1".into(),
        persistence_locator: Some("sessions/brokkr/seat-1".into()),
        persistence_home: Some("/home/operator/.dsh".into()),
    };
    let context = start_context(
        json!({"headless-work": {"status": "supported"}}),
        Some(&originating),
        Some(&target),
        None,
    );
    assert_eq!(context["originating_harness_version"], "0.1.5-rc.1");
    assert_eq!(context["originating_wrapper_digest"], "a".repeat(64));
    assert_eq!(context["owned_target"]["provider_id"], "session-1");
    assert_eq!(
        context["owned_target"]["persistence_locator"],
        "sessions/brokkr/seat-1"
    );
    assert_eq!(
        context["owned_target"]["persistence_home"],
        "/home/operator/.dsh"
    );
    assert_eq!(
        context["assessment"]["headless-work"]["status"],
        "supported"
    );

    let cold = start_context(
        json!({"headless-work": {"status": "unmeasured"}}),
        None,
        None,
        None,
    );
    assert_eq!(cold["assessment"]["headless-work"]["status"], "unmeasured");
    assert!(cold.get("owned_target").is_none());
    assert!(cold.get("originating_harness_version").is_none());
    assert!(cold.get("originating_wrapper_digest").is_none());
    assert!(cold.get("route_overlay").is_none());

    // An offered root whose harness version was not recorded still
    // carries its wrapper digest: the absent version is not a reason to
    // drop the digest (proposed decision 0056 ruling 5).
    let versionless = OriginatingRoot {
        harness_version: None,
        wrapper_digest: Some("b".repeat(64)),
    };
    let context = start_context(
        json!({"headless-work": {"status": "supported"}}),
        Some(&versionless),
        None,
        None,
    );
    assert!(context.get("originating_harness_version").is_none());
    assert_eq!(context["originating_wrapper_digest"], "b".repeat(64));
}

/// The route-overlay binding rides the private context as exactly the
/// argv value and the compiled digest the engine handed it, and is
/// absent when no binding was supplied. It never appears elsewhere in
/// the object, `owned_target` still travels beside it, and an
/// unmeasured assessment changes nothing about it (AS3; 8.10's engine
/// list (i)).
#[test]
fn the_private_context_carries_a_supplied_route_overlay_binding() {
    let binding = RouteOverlay {
        value: "recipes/research-dsh/drivers/research-web.yml".into(),
        digest: "b".repeat(64),
    };
    let context = start_context(
        json!({"headless-work": {"status": "unmeasured"}}),
        None,
        None,
        Some(&binding),
    );
    assert_eq!(
        context["route_overlay"]["value"],
        "recipes/research-dsh/drivers/research-web.yml"
    );
    assert_eq!(context["route_overlay"]["digest"], "b".repeat(64));
    assert_eq!(
        context["assessment"]["headless-work"]["status"],
        "unmeasured"
    );

    let none = start_context(
        json!({"headless-work": {"status": "supported"}}),
        None,
        None,
        None,
    );
    assert!(none.get("route_overlay").is_none());
}
