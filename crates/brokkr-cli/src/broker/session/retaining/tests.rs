//! A retaining plan's share is counted from the sealed inventory alone
//! (U8b; CR3, operator ruling 2026-10-15): every pinned plan beside it,
//! fallback slots included, read as the bound one is.

use std::ffi::OsString;
use std::os::fd::OwnedFd;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::{Arc, Barrier};

use brokkr_core::canonical;
use brokkr_protocol::broker::ledger::{recover, Denial, Disposition, Ledger, Outcome};
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

/// The answer the broker of the plan pinned `at` stages for its call.
fn answer(at: usize) -> Vec<u8> {
    format!(r#"{{"result":{{"content":[{{"text":"{at}","type":"text"}}]}}}}"#).into_bytes()
}

/// How one broker's two calls fared: reserved for, or refused before they
/// could be forwarded.
type Reserved = (Result<(), Denial>, Result<(), Denial>);

/// What the broker of the plan pinned `at`, in the attempt directory `dir`
/// at `attempt`, reserved for each of its two calls, once every broker
/// beside it has opened its ledger there (`start`): a call reserved for is
/// answered with [`answer`] and staged, and one that is not is refused for
/// the budget; then its ledger is closed.
fn brokered(at: usize, (dir, attempt): (&OwnedFd, &Path), start: &Barrier) -> Reserved {
    let plan: Plan = serde_json::from_value(sealed(true)).unwrap();
    let Ok(Some((retaining, staging))) = retention(&plan, dir, attempt) else {
        panic!("a retaining plan of a sealed attempt has its retention");
    };
    let name = OsString::from(format!("cap-{at}.json"));
    let pinned = digest();
    let mut ledger = Ledger::open(dir, &name, (&plan.owner, pinned.as_str()), &[]).unwrap();
    ledger.bind(&[]).unwrap();
    ledger.retain(staging, &name, retaining);
    start.wait();
    let mut called = || {
        let mut call = ledger.room().unwrap().begin("t").unwrap();
        let reserved = ledger.reserve(&mut call);
        let bytes = answer(at);
        let digest = canonical::sha256_bytes(&bytes);
        match reserved {
            Ok(()) => ledger.answered(call, Outcome::Succeeded, (&bytes, &digest)),
            Err(denial) => ledger.end(call, Outcome::Refused(denial)),
        }
        .unwrap();
        reserved
    };
    let reserved = (called(), called());
    assert_eq!(ledger.close(), Ok(Disposition::Clean));
    reserved
}

/// The digest every retaining plan is pinned and its ledger opened by.
fn digest() -> String {
    canonical::sha256_bytes(sealed(true).to_string().as_bytes())
}

#[test]
fn each_broker_of_one_sealed_inventory_spends_its_own_share_and_none_of_anothers() {
    // CR3 (U8b; operator ruling 2026-10-15): one attempt's inventory pins
    // 32 retaining plans, every brokered slot's, fallback or not, beside
    // one that retains nothing. Every plan's broker, all at once, derives
    // its share from that inventory, of exactly one reservation, and
    // stages in the one attempt directory: its first call is reserved for
    // and staged; its second finds its own share spent and is refused for
    // the budget before it could be forwarded, whatever the others did;
    // and once all have closed, each stage is still as it was staged and
    // cited. Pinned 33, a share is under one reservation, and every
    // broker's every call is refused, nothing staged.
    for (retaining, first) in [(32, Ok(())), (33, Err(Denial::Budget))] {
        let mut retained = vec![true; retaining];
        retained.push(false);
        let (root, dir, _) = attempt(&retained, |_, pin| pin);
        let path = root.path().canonicalize().unwrap();
        let start = Arc::new(Barrier::new(retaining));
        let brokers: Vec<_> = (0..retaining)
            .map(|at| {
                let (dir, path) = (dir.try_clone().unwrap(), path.clone());
                let start = Arc::clone(&start);
                std::thread::spawn(move || brokered(at, (&dir, &path), &start))
            })
            .collect();
        let reserved: Vec<Reserved> = brokers.into_iter().map(|b| b.join().unwrap()).collect();
        let budget = Err(Denial::Budget);
        assert_eq!(reserved, vec![(first, budget); retaining]);
        let outcome = |reserved: Result<(), Denial>| match reserved {
            Ok(()) => Outcome::Succeeded,
            Err(denial) => Outcome::Refused(denial),
        };
        let calls = [first, budget].map(|reserved| ("t".to_string(), Some(outcome(reserved))));
        let plan: Plan = serde_json::from_value(sealed(true)).unwrap();
        for at in 0..retaining {
            let read = |suffix: &str| std::fs::read(path.join(format!("cap-{at}.json.{suffix}")));
            let (staged, ledger) = (read("1.response").ok(), read("ledger").unwrap());
            let (lifecycle, invalid) = recover(&ledger, &plan.owner, &digest());
            let line = ledger.split(|byte| *byte == b'\n').nth(2).unwrap();
            let terminal: Value = serde_json::from_slice(line).unwrap();
            let cited = staged.as_ref().map(
                |bytes| json!({"sha256": canonical::sha256_bytes(bytes), "bytes": bytes.len()}),
            );
            let proved = (lifecycle.calls(), lifecycle.closed(), invalid);
            let found = (proved, staged.clone(), terminal.get("response").cloned());
            let clean = (&calls[..], Some(&Disposition::Clean), None);
            let expected = (clean, first.ok().map(|()| answer(at)), cited);
            assert_eq!((retaining, at, found), (retaining, at, expected));
        }
    }
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
