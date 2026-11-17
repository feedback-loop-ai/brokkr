//! The workspace box's environment, moved out of `hands.rs` by decision
//! 0065 slice two, U8a, to make room for the evidence protection beside
//! the box's argv builder (operator ruling of 2026-10-09).

use std::path::Path;

use super::{expand_home, namespace_join, namespace_path, HandsSpec, SANDBOX_HOME};

#[cfg(test)]
pub(crate) mod tests;

/// Every seat's git environment, the one home the workspace box, the
/// unboxed dispatch and the dsh harness read (#1109): `commit.gpgsign`
/// is `false`, so a seat's commits are never signed (CONTRIBUTING).
///
/// The setting rides `GIT_CONFIG_PARAMETERS`, the form `git -c` itself
/// exports to its children, rather than the numbered
/// `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_0`/`GIT_CONFIG_VALUE_0` triple. A
/// harness's credential scrub (dsh's `/KEY|PASSWORD|SECRET|TOKEN/i`)
/// drops `GIT_CONFIG_KEY_0` and keeps the other two, and git refuses
/// every command over the dangling count; this name matches no such
/// pattern, so the setting reaches the seat's shell whole under any
/// harness, not only the one whose own explicit layer could carry it.
pub(crate) fn seat_git_environment() -> (&'static str, &'static str) {
    ("GIT_CONFIG_PARAMETERS", "'commit.gpgsign'='false'")
}

/// The workspace box's environment, in the order it is set: the fixed
/// switches, then each toolchain home its own bind declares.
pub(super) fn box_environment(spec: &HandsSpec, home: &Path) -> Vec<(&'static str, String)> {
    let cargo_home = home.join(".cargo");
    let rustup_home = home.join(".rustup");
    let npm_cache = home.join(".npm");
    let sandbox_path = format!(
        "/runtime:{}:/usr/local/bin:/usr/bin:/bin",
        namespace_join(&namespace_path(&cargo_home), Path::new("bin"))
    );
    let mut environment = vec![
        ("HOME", SANDBOX_HOME.to_string()),
        ("USER", "runner".to_string()),
        ("LOGNAME", "runner".to_string()),
        ("TMPDIR", "/tmp".to_string()),
        ("PATH", sandbox_path),
        ("LANG", "C.UTF-8".to_string()),
        ("LC_ALL", "C.UTF-8".to_string()),
        ("CI", "true".to_string()),
        ("DISABLE_AUTOUPDATER", "1".to_string()),
        ("DISABLE_TELEMETRY", "1".to_string()),
    ];
    // Seat commits are unsigned (CONTRIBUTING); the signing wrapper and
    // its key are not in the box, and the repository config that names
    // them is outranked by this environment entry.
    let (git_name, git_value) = seat_git_environment();
    environment.push((git_name, git_value.to_string()));
    // On rustup installations cargo and rustc are proxies below
    // ~/.cargo/bin. A declared toolchain bind must therefore be both
    // executable and discoverable; the private HOME must not redirect the
    // proxy to an empty ~/.rustup inside the box.
    if spec
        .binds
        .iter()
        .any(|bind| expand_home(&bind.path, home) == cargo_home)
    {
        environment.push(("CARGO_HOME", namespace_path(&cargo_home)));
    }
    if spec
        .binds
        .iter()
        .any(|bind| expand_home(&bind.path, home) == rustup_home)
    {
        environment.push(("RUSTUP_HOME", namespace_path(&rustup_home)));
    }
    if spec
        .binds
        .iter()
        .any(|bind| expand_home(&bind.path, home) == npm_cache)
    {
        environment.push(("NPM_CONFIG_CACHE", namespace_path(&npm_cache)));
    }
    environment
}
