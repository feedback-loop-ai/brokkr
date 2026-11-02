//! U10a2's repairs of the scenario map's link rows through real dispatch
//! (MB3; row 7, and H2's native alias): a second link of a package's sibling
//! module, of the engine's own bootstrap, and one a native surface makes of
//! the program inside the seat's writable reach once the engine has admitted
//! it. Each is the program tree's multiply-linked cause, with no lookup and
//! no server start; a package's directories, every one of them multiply
//! linked by its entries, are no cause (the healthy control's tree).

use std::path::Path;

use brokkr_protocol::broker::Refusal;

use super::{chmod, put, World};

/// How `world`'s run ended once refused: its cause, how often its harness
/// started, and whether its store was read or its server started.
fn refusal(world: &World, run: impl FnOnce() -> super::Ended) -> (Option<String>, usize, bool) {
    let (looked, started) = (world.store_reads(), world.server_reads());
    let ended = run();
    assert_eq!(ended.status, "awaiting_operator", "{ended:?}");
    assert_eq!(world.calls(&ended.run), []);
    let touched = (looked.lookups() > 0) | (started.lookups() > 0);
    (ended.cause, world.starts(), touched)
}

fn linked() -> Option<String> {
    Some(Refusal::Linked.to_string())
}

/// Row 7: a module the program sources, beside it in its package, given a
/// second link outside every reach, refuses at admission, before its harness.
#[test]
fn a_multiply_linked_sibling_module_refuses_before_its_harness() {
    let Some(world) = World::new() else {
        return;
    };
    let module = world.base.join("opt/docs/lib/module.sh");
    put(&module, ":\n");
    chmod(&world.base.join("opt/docs/lib"), 0o755);
    world.server(&super::answering(&format!(". '{}'", module.display())));
    std::fs::hard_link(&module, world.base.join("module.again")).unwrap();
    assert_eq!(refusal(&world, || world.run()), (linked(), 0, false));
}

/// Row 7: the engine's own executable, the bootstrap every box starts on,
/// given a second link: the same cause, for the bootstrap as for the program.
#[test]
fn a_multiply_linked_bootstrap_refuses_before_its_harness() {
    let Some(world) = World::new() else {
        return;
    };
    let own = world.base.join("engine");
    std::fs::create_dir(&own).unwrap();
    chmod(&own, 0o755);
    let engine = own.join("brokkr");
    std::fs::copy(crate::capability_broker::brokkr(), &engine).unwrap();
    std::fs::hard_link(&engine, own.join("brokkr.again")).unwrap();
    let run = || {
        let args = world.run_args();
        let mut command = world.in_world(std::process::Command::new(&engine));
        super::ended(command.args(&args).output().unwrap())
    };
    assert_eq!(refusal(&world, run), (linked(), 0, false));
}

/// H2: a native surface makes a writable alias of an admitted source. The
/// harness, once the engine has admitted and sealed the server's program,
/// links it into the worktree, its seat's writable reach, before its broker
/// starts: the broker refuses by the program tree's cause.
#[test]
fn a_native_alias_of_the_program_made_after_admission_refuses_at_the_broker() {
    let Some(world) = World::new() else {
        return;
    };
    let alias = world.repo.join("docs-mcp");
    world.before_broker(&format!(
        "ln '{}' '{}'",
        world.program().display(),
        alias.display()
    ));
    assert_eq!(refusal(&world, || world.run()), (linked(), 1, false));
    assert!(same(&alias, &world.program()), "the alias stood");
}

/// Whether `one` and `other` are one file.
fn same(one: &Path, other: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let identity = |path: &Path| std::fs::metadata(path).map(|meta| (meta.dev(), meta.ino()));
    identity(one).ok() == identity(other).ok()
}
