//! Task 46.1 (U9b1b; MB3, MB4, CR5): what a server's box is kept from,
//! through real compile and dispatch. Every mode and direction of a seat's
//! reach over the server's package, its resolution ancestors, the store's
//! empty and missing forms, each network and the retained response's way back
//! out through `inspect`.

use std::path::Path;

use brokkr_protocol::broker::Refusal;
use serde_json::{json, Value};

use super::matrix::{marking, refused_unread, STARTED};
use super::{answering, put, World, TOKEN};

/// A seat's boxed hands, binding `path` as `mode`.
fn binding(path: &Path, mode: &str) -> Value {
    json!({"kind": "workspace", "network": false, "binds": [{"path": path, "mode": mode}]})
}

/// The refusal of `world`'s run before its driver started, by the source lease's
/// identity cause, naming what its seat can write: no lookup, no harness and no
/// server. Ruling F6's order: this cause answers first, ahead of MB3's "launch
/// resolves inside seat-writable reach", which stands behind it.
fn leased_off(world: &World, names: &Path) {
    let reads = world.store_reads();
    let ended = world.run();
    let cause = ended.cause.clone().unwrap_or_default();
    let identity = format!("dispatch refused: {}: ", Refusal::Identity);
    assert_eq!(
        (
            ended.status.as_str(),
            cause.starts_with(&identity),
            cause.contains(names.to_str().unwrap()),
        ),
        ("awaiting_operator", true, true),
        "{ended:?}"
    );
    let unstarted = (
        reads.lookups(),
        world.starts(),
        world.base.join(STARTED).exists(),
    );
    assert_eq!(unstarted, (0, 0, false));
}

/// `refused` over a world whose seat binds the server's package as `mode`, at,
/// above and below it, each direction by itself, handed what it binds.
fn each_direction(mode: &str, refused: impl Fn(&World, &Path)) {
    for at in ["opt/docs", "opt", "opt/docs/bin"] {
        let Some(world) = World::new() else {
            return;
        };
        marking(&world);
        let bound = world.base.join(at);
        world.worker(Some(binding(&bound, mode)), "requires");
        refused(&world, &bound);
    }
}

/// A seat writing at, above or below the server's package reaches it: the
/// source lease refuses it before its driver starts, naming what it can write.
#[test]
fn a_writable_bind_at_above_or_below_the_package_reaches_it() {
    each_direction("rw", leased_off);
}

/// A read-only or overlay bind is no writer's, so the lease lets it through,
/// and it still overlaps the box's source in either direction: MB3's bind
/// cause, each mode by itself, with no lookup and no server.
#[test]
fn a_read_only_or_overlay_bind_over_the_package_overlaps_the_box() {
    for mode in ["ro", "overlay"] {
        each_direction(mode, |world, _| {
            let reads = world.store_reads();
            refused_unread(world, reads, &Refusal::BindOverlapsReach.to_string());
        });
    }
}

/// MB3's resolution ancestors: a program spelled through a link the seat can
/// rewrite, whose target is out of its reach, is refused by what the seat can
/// write: the link itself, named by the lease that keeps it.
#[test]
fn a_launch_resolved_through_a_link_in_reach_refuses_with_no_lookup_or_start() {
    let Some(world) = World::new() else {
        return;
    };
    marking(&world);
    let writable = world.base.join("writable");
    std::fs::create_dir(&writable).unwrap();
    super::chmod(&writable, 0o755);
    let link = writable.join("current");
    std::os::unix::fs::symlink(world.base.join("opt/docs"), &link).unwrap();
    world.dialect(&json!({"connection": {"argv": [link.join("bin/docs-mcp")]}}));
    world.worker(Some(binding(&writable, "rw")), "requires");
    leased_off(&world, &link);
}

/// A program inside the worktree itself is in every boxed seat's reach.
#[test]
fn a_program_in_the_worktree_refuses_with_no_lookup_or_start() {
    let Some(world) = World::new() else {
        return;
    };
    marking(&world);
    let program = world.repo.join("tools/docs-mcp");
    put(&program, std::fs::read(world.program()).unwrap());
    super::chmod(&program, 0o755);
    for at in [".", "tools"] {
        super::chmod(&world.repo.join(at), 0o755);
    }
    world.dialect(&json!({"connection": {"argv": [&program]}}));
    leased_off(&world, &program);
}

/// A singleton system entry: the server is the system's own `/bin/sh`, given
/// its script as an argument, served from its own system set with the host's
/// ordinary hard links intact, beside no package of its own.
#[test]
fn a_singleton_system_entry_is_served_beside_the_systems_hard_links() {
    let Some(world) = World::new() else {
        return;
    };
    let system = std::fs::canonicalize("/bin/sh").unwrap();
    let script = answering("").replacen("#!/bin/sh\n", "", 1);
    world.dialect(&json!({"connection": {"argv": [system, "-c", script]}}));
    let ended = world.completes();
    assert_eq!(world.calls(&ended.run).len(), 1);
    assert_eq!(world.broker_code(), Some(0));
}

/// A store that stands but binds nothing the plan declares, and one that does
/// not stand, are the one absence to the binding: decision 0012's own cause,
/// naming the binding and the store, before any server starts.
#[test]
fn an_empty_or_missing_store_starts_no_server() {
    for missing in [false, true] {
        let Some(world) = World::new() else {
            return;
        };
        marking(&world);
        let store = world.secrets();
        match missing {
            true => std::fs::remove_file(&store).unwrap(),
            false => world.store(""),
        }
        let cause = format!(
            "secret 'DOCS_TOKEN' is not in the store at {} (brokkr secrets set DOCS_TOKEN)",
            store.display()
        );
        let ended = world.run();
        assert_eq!(
            (ended.status.as_str(), ended.cause.as_deref()),
            ("awaiting_operator", Some(cause.as_str())),
            "{ended:?}"
        );
        assert!(!world.base.join(STARTED).exists(), "no server start");
        assert_eq!(world.calls(&ended.run), []);
    }
}

/// Each egress's network, as the running server's own namespace shows it: a
/// `local` route is its own, holding only loopback; a `contracted` one is the
/// host's. The seat's hands stay network-false in both.
#[test]
fn a_local_route_is_isolated_a_contracted_one_shared_and_the_hands_stay_off_the_network() {
    let host = std::fs::read_link("/proc/self/ns/net").unwrap();
    let probe = r#"readlink /proc/$$/ns/net >&2
cut -d: -f1 /proc/net/dev | tail -n +3 | tr -d ' ' | sort | tr '\n' ' ' >&2
echo >&2"#;
    for (egress, shared) in [("local", false), ("contracted", true)] {
        let Some(world) = World::new() else {
            return;
        };
        world.dialect(&json!({"egress": egress}));
        world.server(&answering(probe));
        let ended = world.completes();
        let said = std::fs::read_to_string(world.bin("broker.stderr")).unwrap();
        let mut lines = said
            .lines()
            .map(|line| line.strip_prefix("MCP server stderr: ").unwrap_or(line));
        let (space, devices) = (lines.next().unwrap(), lines.next().unwrap());
        assert_eq!(space == host.to_str().unwrap(), shared, "{egress}: {said}");
        if !shared {
            assert_eq!(devices.trim(), "lo", "{said}");
        }
        let config = std::fs::read_to_string(world.bin("config.json")).unwrap();
        assert!(config.contains(r#"\"network\":false"#), "{config}");
        assert_eq!(world.calls(&ended.run).len(), 1);
    }
}

/// A declared loading name reaches the server at its confined exec: the
/// server's names are MB4's fixed ones and its binding. The host's own loader
/// environment is `boxes::the_host_loader_environment_stays_out_of_the_box`'s.
#[test]
fn a_declared_loading_binding_reaches_the_server() {
    let Some(world) = World::new() else {
        return;
    };
    world.store("PYTHONPATH=/runtime/lib\n");
    world.dialect(&json!({"secrets": ["PYTHONPATH"]}));
    let probe = r#"tr "\0" "\n" < /proc/$$/environ | cut -d= -f1 | sort | tr "\n" " " >&2
echo >&2"#;
    world.server(&answering(probe));
    world.completes();
    let said = std::fs::read_to_string(world.bin("broker.stderr")).unwrap();
    let names = "BROKKR_HANDS_BOX HOME LANG LC_ALL LOGNAME PATH PYTHONPATH TMPDIR USER ";
    assert!(
        said.starts_with(&format!("MCP server stderr: {names}\n")),
        "{said}"
    );
}

/// CR5 on the real path: the retained response of a served call opens through
/// `inspect --capability-call`, verified against its row, and it holds what
/// the server said with its bound value masked.
#[test]
fn a_real_calls_retained_response_opens_through_inspect_masked() {
    let Some(world) = World::new() else {
        return;
    };
    let answer = format!(
        r#"echo '{{"jsonrpc":"2.0","id":3,"result":{{"content":[{{"type":"text","text":"key {TOKEN} end"}}]}}}}'"#
    );
    world.server(&format!("{}{answer}\nread -r line\n", super::STARTS));
    let ended = world.completes();
    let view = world.opened(&ended.run);
    let said = json!({"result": {"content": [
        {"text": "key [secret:DOCS_TOKEN] end", "type": "text"}]}});
    let digest = brokkr_core::canonical::sha256_bytes(
        br#"{"result":{"content":[{"text":"key [secret:DOCS_TOKEN] end","type":"text"}]}}"#,
    );
    assert_eq!(
        (
            &view["response"],
            &view["call"]["state"],
            &view["call"]["response_sha256"]
        ),
        (&said, &json!("succeeded"), &json!(digest)),
        "{view}"
    );
}
