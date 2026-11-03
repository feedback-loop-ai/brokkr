//! U10a2's repairs of the scenario map's row 11 through real dispatch (MB3):
//! the source walk's depth and its resolution's hops, each bound by itself,
//! a package or a launch just past it refused by the identity cause with no
//! harness, lookup or start, and one just inside it served.

use std::path::{Path, PathBuf};

use brokkr_protocol::broker::Refusal;
use serde_json::json;

use super::{chmod, World};

/// `world`'s run: served where `cause` is none, or refused by it before its
/// harness, reading no store and starting no server.
fn served_or_refused(world: &World, cause: Option<String>) {
    let served = cause.is_none();
    let (looked, started) = (world.store_reads(), world.server_reads());
    let ended = world.run();
    let status = match served {
        true => "completed",
        false => "awaiting_operator",
    };
    assert_eq!((ended.status.as_str(), ended.cause), (status, cause));
    let touched = (world.starts(), looked.lookups() > 0, started.lookups() > 0);
    assert_eq!(touched, (usize::from(served), served, served));
}

/// MB3's depth bound of 64: a package holding a chain of `levels` directories.
#[test]
fn a_package_past_the_walks_depth_bound_refuses_and_one_within_it_is_served() {
    for (levels, served) in [(70, false), (60, true)] {
        let Some(world) = World::new() else {
            return;
        };
        let mut at = world.base.join("opt/docs/share");
        for _ in 0..levels {
            std::fs::create_dir_all(&at).unwrap();
            chmod(&at, 0o755);
            at.push("d");
        }
        let cause = (!served).then(|| Refusal::Identity.to_string());
        served_or_refused(&world, cause);
    }
}

/// A chain of `links` symbolic links outside every reach, its last naming
/// the program, its first returned.
fn chain(world: &World, links: usize) -> PathBuf {
    let dir = world.base.join("links");
    std::fs::create_dir(&dir).unwrap();
    chmod(&dir, 0o755);
    let name = |at: usize| dir.join(format!("l{at}"));
    for at in 0..links {
        let target = match at + 1 == links {
            true => world.program(),
            false => name(at + 1),
        };
        std::os::unix::fs::symlink(&target, name(at)).unwrap();
    }
    name(0)
}

/// MB3's bound of 40 hops a resolution takes, the kernel's own: a launch
/// named through a chain of `links` symbolic links, which the host resolves
/// within the bound and not past it. Past it, the source lease refuses the
/// route by name before the box is observed.
#[test]
fn a_launch_past_the_resolutions_hop_bound_refuses_and_one_within_it_is_served() {
    for (links, served) in [(45, false), (30, true)] {
        let Some(world) = World::new() else {
            return;
        };
        let launch = chain(&world, links);
        assert_eq!(Path::new(&launch).exists(), served);
        world.dialect(&json!({"connection": {"argv": [&launch]}}));
        let cause = (!served).then(|| {
            format!(
                "dispatch refused: {}: the route of '{}', which an MCP server's box is built \
                 from, meets more than 40 symlinks",
                Refusal::Identity,
                launch.display()
            )
        });
        served_or_refused(&world, cause);
    }
}
