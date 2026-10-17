//! The workspace box's environment, moved out of `hands.rs` by decision
//! 0065 slice two, U8a, to make room for the evidence protection beside
//! `box_argv` (operator ruling of 2026-10-09).

use std::path::Path;

use super::{expand_home, namespace_join, namespace_path, HandsSpec, SANDBOX_HOME};

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
        // Seat commits are unsigned (CONTRIBUTING); the signing wrapper
        // and its key are not in the box, and the repository config that
        // names them is outranked by this environment entry.
        ("GIT_CONFIG_COUNT", "1".to_string()),
        ("GIT_CONFIG_KEY_0", "commit.gpgsign".to_string()),
        ("GIT_CONFIG_VALUE_0", "false".to_string()),
    ];
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
