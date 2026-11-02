//! U10a2's repairs of the scenario map's store rows through real dispatch
//! (MB4; rows 20 and 21, and H6): a store under the server's own read-only
//! package bind, with bindings and secret-free; a store a seat's read-only
//! bind exposes; and a secret-free dialect's store, made by the engine where
//! it is missing and refused by the broker where it goes missing after. Each
//! refusal is the broker's, past its harness's start: it starts no server
//! and reads no store.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use brokkr_protocol::broker::Refusal;
use serde_json::json;

use super::{ended, Ended, World};

/// `world`'s run with its store at `store`: how it ended, and the server's
/// starts.
fn run_with(world: &World, store: &Path) -> (Ended, usize) {
    let started = world.server_reads();
    let words = ["run", "--feature", "enabled", "--secrets-file"];
    let mut args: Vec<&str> = words.to_vec();
    args.push(store.to_str().unwrap());
    let ended = ended(world.brokkr(&args).output().unwrap());
    (ended, started.lookups())
}

/// `world`'s run with its store at `store`, refused by `cause` at its broker:
/// the harness started once, and no lookup, server or call.
fn refused_unread(world: &World, store: &Path, cause: &str) {
    let looked = super::Reads::on(store.to_path_buf());
    let (ended, started) = run_with(world, store);
    assert_eq!(
        (ended.status.as_str(), ended.cause.as_deref()),
        ("awaiting_operator", Some(cause)),
        "{ended:?}"
    );
    assert_eq!((looked.lookups(), world.starts(), started), (0, 1, 0));
    assert_eq!(world.calls(&ended.run), []);
}

/// Row 20 (MB4, H6): a store inside the server's package lies under the box's
/// read-only bind of it, so it would be mounted in the box: refused by that
/// cause before any lookup or start, a secret-free dialect's as well.
#[test]
fn a_store_under_the_packages_read_only_bind_would_be_in_the_box() {
    for secrets in [json!(["DOCS_TOKEN"]), json!([])] {
        let Some(world) = World::new() else {
            return;
        };
        world.dialect(&json!({"secrets": secrets}));
        let store = world.base.join("opt/docs/share/secrets.env");
        super::put(&store, format!("DOCS_TOKEN={}\n", super::TOKEN));
        super::chmod(&store, 0o600);
        super::chmod(&world.base.join("opt/docs/share"), 0o755);
        refused_unread(&world, &store, &Refusal::StoreInBox.to_string());
    }
}

/// H6: a store a seat's read-only bind exposes is readable by its hands,
/// though no hand can write it: the hands-reach cause, before any lookup.
#[test]
fn a_store_under_a_seats_read_only_bind_is_reachable_by_its_hands() {
    let Some(world) = World::new() else {
        return;
    };
    let binds = json!([{"path": world.base.join("secrets"), "mode": "ro"}]);
    let hands = json!({"kind": "workspace", "network": false, "binds": binds});
    world.worker(Some(hands), "requires");
    let cause = Refusal::StoreReachable.to_string();
    refused_unread(&world, &world.secrets(), &cause);
}

/// Row 21 (MB4): a secret-free dialect still admits its store's identity. A
/// store missing when the run starts is made by the engine, empty and its
/// owner's alone (operator ruling 2 of 2026-10-23), and the seat is served
/// over it; one that goes missing between the engine's admission and the
/// broker's refuses by MB3's identity cause, before any server starts, and is
/// not made again.
#[test]
fn a_secret_free_dialects_store_is_admitted_made_where_missing_and_refused_once_gone() {
    for gone in [false, true] {
        let Some(world) = World::new() else {
            return;
        };
        world.dialect(&json!({"secrets": []}));
        std::fs::remove_file(world.secrets()).unwrap();
        if gone {
            let store = world.secrets();
            world.before_broker(&format!("rm '{}'", store.display()));
        }
        let (ended, started) = run_with(&world, &world.secrets());
        let (status, cause, calls) = match gone {
            true => ("awaiting_operator", Some(Refusal::Identity.to_string()), 0),
            false => ("completed", None, 1),
        };
        assert_eq!(
            (
                ended.status.as_str(),
                ended.cause,
                world.calls(&ended.run).len()
            ),
            (status, cause, calls)
        );
        let made = std::fs::metadata(world.secrets()).ok();
        let made = made.map(|made| (made.len(), made.permissions().mode() & 0o777));
        assert_eq!((started > 0, made), (!gone, (!gone).then_some((0, 0o600))));
    }
}
