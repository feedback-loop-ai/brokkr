//! U10a2's repairs of the scenario map's confinement rows through real
//! dispatch, each read from inside the running server's own box (MB3, MB4,
//! CR2; rows 4, 13, 27 and 29): a module its program sources from outside
//! its package is not there, though one inside is; of the host's `/etc/ssl`
//! only the public certificates are; a local route reaches neither the
//! host's loopback nor its abstract sockets, which a contracted one reaches;
//! and none of the attempt's evidence, the store's folder or the host HOME is
//! there while its call is retained.

use std::io::ErrorKind;
use std::os::linux::net::SocketAddrExt;
use std::path::Path;

use brokkr_protocol::broker::ledger::Failure;
use serde_json::json;

use super::{answering, chmod, put, World};

/// The words the server's stderr relays, cut of the relay's framing.
fn relayed(world: &World) -> Vec<String> {
    let said = std::fs::read_to_string(world.bin("broker.stderr")).unwrap();
    let lines = said
        .lines()
        .filter_map(|line| line.strip_prefix("MCP server stderr: "));
    lines.map(str::to_string).collect()
}

/// Row 4 (MB3, H2, H3): the program sources a module before it says a word
/// of MCP. Inside its package, under the box's read-only bind of it, the
/// module loads and the call settles; beside the package, outside every bind,
/// it is absent inside the box, so the server fails its initialization with
/// the protocol's own cause past the store's lookup, and none of the module's
/// bytes reach anything the broker relays. No argument or source adds a
/// mount.
#[test]
fn a_module_outside_the_package_is_absent_from_the_box_and_one_inside_loads() {
    for inside in [true, false] {
        let Some(world) = World::new() else {
            return;
        };
        let module = match inside {
            true => world.base.join("opt/docs/lib/module.sh"),
            false => world.base.join("opt/shared/module.sh"),
        };
        put(&module, ": u10a2-module-bytes\n");
        chmod(module.parent().unwrap(), 0o755);
        let rest = answering("").replacen("#!/bin/sh\n", "", 1);
        world.server(&format!("#!/bin/sh\n. '{}'\n{rest}", module.display()));
        let looked = world.store_reads();
        let ended = world.run();
        let (status, cause, calls) = match inside {
            true => ("completed", None, 1),
            false => ("awaiting_operator", Some(Failure::Protocol.to_string()), 0),
        };
        assert_eq!(
            (
                ended.status.as_str(),
                ended.cause,
                world.calls(&ended.run).len()
            ),
            (status, cause, calls)
        );
        assert!(looked.lookups() >= 1, "past the store's lookup");
        let said = std::fs::read_to_string(world.bin("broker.stderr")).unwrap();
        assert!(!said.contains("u10a2-module-bytes"), "{said}");
    }
}

/// Row 13 (MB3): of the host's `/etc/ssl`, the box holds the certificates
/// and not the private material beside them, which it neither mounts nor
/// fails on, and the call settles.
#[test]
fn the_box_holds_the_public_certificates_and_not_the_private_tls_material() {
    for host in ["/etc/ssl/certs", "/etc/ssl/private"] {
        if !Path::new(host).exists() {
            super::skip(&format!("this host has no {host}"));
            return;
        }
    }
    let Some(world) = World::new() else {
        return;
    };
    let probe = r#"[ -n "$(ls /etc/ssl/certs)" ] && echo certs >&2
[ -e /etc/ssl/private ] && echo private >&2 || echo no-private >&2"#;
    world.server(&answering(probe));
    let ended = world.completes();
    assert_eq!(world.calls(&ended.run).len(), 1);
    assert_eq!(relayed(&world), ["certs", "no-private"]);
}

/// Row 27 (MB4): a sentinel on the host's loopback and one on an abstract
/// Unix socket, each the test's own. A local route's box reaches neither; a
/// contracted one's reaches both, its seat's hands network-false throughout.
#[test]
fn a_local_route_reaches_no_host_sentinel_and_a_contracted_one_reaches_both() {
    if !Path::new("/usr/bin/python3").exists() {
        super::skip("this host has no /usr/bin/python3 to reach an abstract socket with");
        return;
    }
    for (egress, reached) in [("local", false), ("contracted", true)] {
        let Some(world) = World::new() else {
            return;
        };
        let tcp = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        tcp.set_nonblocking(true).unwrap();
        let name = format!("brokkr-u10a2-{}", std::process::id());
        let address = std::os::unix::net::SocketAddr::from_abstract_name(&name).unwrap();
        let abstract_socket = std::os::unix::net::UnixListener::bind_addr(&address).unwrap();
        abstract_socket.set_nonblocking(true).unwrap();
        let port = tcp.local_addr().unwrap().port();
        let probe = format!(
            r#"if (exec 3<>/dev/tcp/127.0.0.1/{port}) 2>/dev/null; then echo tcp >&2; fi
python3 -c 'import socket; socket.socket(socket.AF_UNIX).connect("\0{name}")' 2>/dev/null && echo abstract >&2
echo probed >&2"#
        );
        world.dialect(&json!({"egress": egress}));
        let script = answering(&probe).replacen("#!/bin/sh\n", "#!/bin/bash\n", 1);
        world.server(&script);
        world.completes();
        let heard = |accepted: std::io::Result<()>| match accepted {
            Ok(()) => true,
            Err(error) if error.kind() == ErrorKind::WouldBlock => false,
            Err(error) => panic!("{error}"),
        };
        let seen = (
            heard(tcp.accept().map(drop)),
            heard(abstract_socket.accept().map(drop)),
        );
        let expected: &[&str] = match reached {
            true => &["tcp", "abstract", "probed"],
            false => &["probed"],
        };
        let expected: Vec<String> = expected.iter().map(|line| line.to_string()).collect();
        assert_eq!(
            (relayed(&world), seen),
            (expected, (reached, reached)),
            "{egress}"
        );
        let config = std::fs::read_to_string(world.bin("config.json")).unwrap();
        assert!(config.contains(r#"\"network\":false"#), "{config}");
    }
}

/// Row 29 (CR2): the server probes the attempt's coordination root, where its
/// plan, ledger and stage lie, the host HOME, the store's folder and the
/// worktree, then answers. None of them is in its box; its answer is
/// retained, staged and published outside it.
#[test]
fn a_boxed_server_finds_none_of_its_evidence_and_its_answer_is_retained() {
    let Some(world) = World::new() else {
        return;
    };
    let root = world.base.join("home/.local/state/brokkr/capabilities");
    let places = [
        root,
        world.base.join("home"),
        world.base.join("secrets"),
        world.repo.clone(),
    ];
    let places: Vec<String> = places
        .iter()
        .map(|at| format!("'{}'", at.display()))
        .collect();
    let probe = format!(
        "for at in {}; do [ -e \"$at\" ] && echo \"present $at\" >&2; done\necho probed >&2",
        places.join(" ")
    );
    world.server(&answering(&probe));
    let ended = world.completes();
    assert_eq!(relayed(&world), ["probed"]);
    let rows = world.calls(&ended.run);
    assert_eq!(
        rows.iter()
            .map(|(_, state, digest)| (state.as_str(), digest.is_some()))
            .collect::<Vec<_>>(),
        [("succeeded", true)]
    );
}
