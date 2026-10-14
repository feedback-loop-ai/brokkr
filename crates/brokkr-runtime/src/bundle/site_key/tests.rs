//! `site_key.rs`'s own unit tests: the strings it renders and the digests
//! it is journaled by.

use super::*;

/// Every topology renders the flattened label the census keys on and the
/// tag the engine's maps key on. Two structurally different sites may
/// share one label, which is the aliasing the census refuses, and they
/// never share a digest.
#[test]
fn every_site_renders_the_label_and_tag_the_census_spells() {
    let sites = [
        (SiteKey::single("work", None), "work", None),
        (SiteKey::single("work", Some("engine")), "work:engine", None),
        (
            SiteKey::panel_member("work", None, "alpha", 0),
            "work:alpha",
            Some("alpha"),
        ),
        (
            SiteKey::sequence_step("work", Some("default"), "author", 0),
            "work:default:author",
            Some("author"),
        ),
        (
            SiteKey::sequence_panel_member("work", None, "review", 1, "alpha", 0),
            "work:review:alpha",
            Some("review:alpha"),
        ),
    ];
    for (key, label, tag) in sites {
        assert_eq!(key.label(), label);
        assert_eq!(key.tag().as_deref(), tag);
    }
    let left = SiteKey::sequence_panel_member("work", None, "a:b", 0, "c", 0);
    let right = SiteKey::sequence_panel_member("work", None, "a", 0, "b:c", 0);
    assert_eq!(left.label(), "work:a:b:c");
    assert_eq!(right.label(), "work:a:b:c");
    assert_ne!(left.digest(), right.digest());
}

/// A site's digest is journaled as each checkpoint's `site_ref`, which is
/// what lets a fresh engine rejoin a session, so its bytes are frozen:
/// measured on main at f6429ec4, before `SiteKey` left `engine/resume.rs`.
#[test]
fn every_site_key_digests_as_it_did() {
    let sites = [
        (
            SiteKey::single("work", None),
            "b5e9a3fd0e0551a8649ac15b2eede3c3cbe7e8a206050ec328b6aa15c55b59fe",
        ),
        (
            SiteKey::single("work", Some("engine")),
            "23e4aedc417e7f60d4b5dab25ed4d27e6a5c05cf4295e4c80a665ec2fbe81e85",
        ),
        (
            SiteKey::panel_member("work", None, "alpha", 0),
            "18ce7b38766167e315a44182773c3cf69c7660a81be9bfa8c65995da56a45160",
        ),
        (
            SiteKey::sequence_step("work", None, "author", 0),
            "fe6bf2c81b064b55c2884dd7f552ccdc637260d415c5fe1e5b62d8d8ee7b0a41",
        ),
        (
            SiteKey::sequence_panel_member("work", None, "review", 1, "alpha", 0),
            "8f2a8ce35febee4e2f2ac663ac08b54d72a4f264e54f5dd2d89e4f5058892c4a",
        ),
    ];
    for (key, digest) in sites {
        assert_eq!(key.digest(), digest);
    }
}
