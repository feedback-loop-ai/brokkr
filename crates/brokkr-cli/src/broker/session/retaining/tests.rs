//! A retaining plan's share is counted from the sealed inventory alone
//! (U8b; CR3, operator ruling 2026-10-15): every pinned plan beside it,
//! fallback slots included, read as the bound one is.

use std::os::fd::OwnedFd;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use brokkr_core::canonical;
use brokkr_protocol::broker::{Plan, Refusal};
use rustix::io::{fcntl_getfd, FdFlags};
use serde_json::{json, Value};

use super::retention;
use crate::broker::session::Ended;

/// A sealed plan, retaining or not: its closed shape, every digest one
/// byte repeated.
fn sealed(retained: bool) -> Value {
    let hex = |byte: char| byte.to_string().repeat(64);
    let intent = json!({
        "excluded": {"control": [], "store": "/store"},
        "bootstrap": {"digest": hex('e'), "path": "/"},
        "environment": [], "network": "isolated",
        "writers": {"privilege": "confined", "uids": [1]},
        "sources": {"digest": hex('d'), "mounts": 1, "entries": 1},
        "tree": {"kind": "system"}, "executable": "/bin/true",
        "reach": {"readable": [], "writable": []},
    });
    json!({
        "box": intent, "secrets": [], "retained": retained, "restrictions": {},
        "clearance": {"policy": hex('c'), "dialect": hex('b')},
        "tools": ["t"], "connection": {"argv": ["/bin/true"]},
        "dialect": {"version": "1", "definition": hex('a'), "digest": hex('b'), "name": "d"},
        "capability": "c", "server": "cap-c",
        "owner": {"instance": "s#0", "site": "s", "attempt": "a", "effect": "e", "run": "r",
                  "repo": hex('f')},
    })
}

/// Write `bytes` as the owner-only file `name` in `dir`.
fn owned(dir: &Path, name: &str, bytes: &[u8]) {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
}

/// An attempt directory whose inventory pins one plan of each `retained`,
/// by the name its index gives it, and as `pinned` rewrites each pin; the
/// directory held, and the first plan.
fn attempt(
    retained: &[bool],
    pinned: impl Fn(usize, Value) -> Value,
) -> (tempfile::TempDir, OwnedFd, Plan) {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().canonicalize().unwrap();
    let pins: Vec<Value> = retained
        .iter()
        .enumerate()
        .map(|(at, retained)| {
            let bytes = sealed(*retained).to_string();
            owned(&dir, &format!("cap-{at}.json"), bytes.as_bytes());
            let pin = json!({"locator": dir.join(format!("cap-{at}.json")),
                         "digest": canonical::sha256_bytes(bytes.as_bytes())});
            pinned(at, pin)
        })
        .collect();
    owned(
        &dir,
        "inventory.json",
        json!({"plans": pins}).to_string().as_bytes(),
    );
    let held = OwnedFd::from(std::fs::File::open(&dir).unwrap());
    let plan = serde_json::from_value(sealed(retained[0])).unwrap();
    (root, held, plan)
}

#[test]
fn every_retaining_pin_counts_and_the_staging_handle_is_the_brokers_own() {
    // Three plans, the second a fallback slot's, two of them retaining:
    // the share divides by two, and staging holds a handle of the attempt
    // directory of its own, closed on exec, so no server inherits it.
    let (root, dir, plan) = attempt(&[true, true, false], |_, pin| pin);
    let path = root.path().canonicalize().unwrap();
    let Ok(Some((retaining, held))) = retention(&plan, &dir, &path) else {
        panic!("a retaining plan of a sealed attempt has its retention");
    };
    assert_eq!(retaining, 2);
    assert_eq!(fcntl_getfd(&held), Ok(FdFlags::CLOEXEC));
    // A plan that retains nothing reads nothing, its inventory gone.
    std::fs::remove_file(path.join("inventory.json")).unwrap();
    let plain: Plan = serde_json::from_value(sealed(false)).unwrap();
    assert!(matches!(retention(&plain, &dir, &path), Ok(None)));
}

#[test]
fn a_pin_that_does_not_verify_beside_the_plan_leaves_it_unbound() {
    // A pin outside the attempt directory, or whose digest its plan's
    // bytes do not hash to: the share could not be the engine's.
    let elsewhere = |at, mut pin: Value| {
        if at == 1 {
            pin["locator"] = json!("/elsewhere/cap-1.json");
        }
        pin
    };
    let altered = |at, mut pin: Value| {
        if at == 1 {
            pin["digest"] = json!("0".repeat(64));
        }
        pin
    };
    for (case, (root, dir, plan)) in [
        ("elsewhere", attempt(&[true, true], elsewhere)),
        ("altered", attempt(&[true, true], altered)),
    ] {
        let path = root.path().canonicalize().unwrap();
        let found = retention(&plan, &dir, &path).map(drop);
        let unbound = Err(Ended::Refused(Refusal::Unbound));
        assert_eq!((case, found), (case, unbound));
    }
}
