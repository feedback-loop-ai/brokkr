//! Row 25 of the boxed-server scenario map, through real dispatch (U10a3;
//! MB4, "Fresh private directories and exact environment are per server"):
//! a replacement attempt's server starts in a box of its own, whose HOME and
//! TMPDIR hold nothing its failed predecessor left there. `boxes`'s
//! `a_second_servers_private_directories…` is the same claim for two servers
//! of one attempt.

use serde_json::json;

use super::{put, World};

/// What the server says of its HOME and TMPDIR as it starts, then again
/// once it has written a file in each.
const LISTING: &str = r#"ls -A "$TMPDIR" "$HOME" | tr "\n" " " >&2
echo >&2
touch "$TMPDIR/mine" "$HOME/mine"
ls -A "$TMPDIR" "$HOME" | tr "\n" " " >&2
echo >&2"#;

/// The seat's first attempt is served, its server writing a file in each
/// private directory and its one call settling, and then its harness fails;
/// the bounded retry's replacement attempt is served afresh. Its server finds
/// both directories empty as it starts, as the first did. Two harness starts,
/// two server starts and two settled calls, one of each attempt; the store is
/// looked up.
#[test]
fn a_replacement_attempts_server_finds_nothing_its_predecessor_left() {
    let Some(world) = World::new() else {
        return;
    };
    world.server(&super::answering(LISTING));
    put(&world.bin("fail-served"), "");
    let limits = json!({"max_attempts": 2, "timeout_seconds": 90});
    world.chain(&["opus"], Some(limits));
    let (looked, started) = (world.store_reads(), world.server_reads());
    let ended = world.completes();
    let empty = "/runtime/home:  /tmp: ";
    let written = "/runtime/home: mine  /tmp: mine ";
    assert_eq!(
        super::boxes::relayed(&world, "broker.stderr"),
        [empty, written, empty, written],
        "the first attempt's two lines, then its replacement's"
    );
    assert_eq!((world.starts(), world.broker_code()), (2, Some(0)));
    assert!(looked.lookups() > 0, "the store was looked up");
    assert!(started.lookups() > 0, "the server's start is observed");
    let object = r#"{"result":{"content":[{"text":"resolved","type":"text"}]}}"#;
    let digest = brokkr_core::canonical::sha256_bytes(object.as_bytes());
    let settled = ("resolve".into(), "succeeded".into(), Some(digest));
    assert_eq!(world.calls(&ended.run), [settled.clone(), settled]);
}
