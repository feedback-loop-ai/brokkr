//! `forge doctor` — verify tools, drivers, the workspace database, and
//! (optionally) a bundle, without executing any agent. Required tools
//! fail the check; optional ones warn. Acceptance criterion: a user can
//! see what is missing before a run wastes a model session on it.

use std::path::{Path, PathBuf};
use std::process::Command;

use forge_runtime::Bundle;
use forge_store::Store;

pub struct Report {
    pub healthy: bool,
    lines: Vec<String>,
}

impl Report {
    fn ok(&mut self, what: &str, detail: String) {
        self.lines.push(format!("ok       {what}: {detail}"));
    }
    fn warn(&mut self, what: &str, detail: String) {
        self.lines.push(format!("warn     {what}: {detail}"));
    }
    fn missing(&mut self, what: &str, detail: String) {
        self.healthy = false;
        self.lines.push(format!("MISSING  {what}: {detail}"));
    }
    pub fn render(&self) -> String {
        self.lines.join("\n")
    }
}

fn tool_version(program: &str) -> Option<String> {
    let out = Command::new(program).arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_string(),
    )
}

pub fn doctor(bundle: Option<&Path>, db: &PathBuf) -> Report {
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };

    // The pinned contract versions this binary was built against.
    report.ok(
        "contracts",
        format!(
            "engine {}, event_schema {}, database_schema {}, driver_protocol {}",
            forge_runtime::ENGINE_VERSION,
            forge_runtime::bundle::EVENT_SCHEMA,
            forge_store::DATABASE_SCHEMA,
            forge_runtime::bundle::DRIVER_PROTOCOL,
        ),
    );

    // Required: the engine's own effects use git (drift/dirty gates).
    match tool_version("git") {
        Some(v) => report.ok("git", v),
        None => report.missing("git", "required for worktree, drift, and dirty gates".into()),
    }
    // Required by the bundled Claude Code driver.
    match tool_version("python3") {
        Some(v) => report.ok("python3", v),
        None => report.missing("python3", "required by the claude-code driver".into()),
    }
    // Optional: each agent CLI matters only to bundles whose seats use
    // its driver.
    for (tool, driver) in [
        ("claude", "claude-code"),
        ("codex", "codex"),
        ("dsh", "exec (dsh/Surface profiles)"),
    ] {
        match tool_version(tool) {
            Some(v) => report.ok(tool, v),
            None => report.warn(
                tool,
                format!("not found — seats using the {driver} driver will fail to spawn"),
            ),
        }
    }

    match Store::open(db) {
        Ok(_) => report.ok("database", format!("{} opens (WAL, append-only triggers)", db.display())),
        Err(e) => report.missing("database", format!("{}: {e}", db.display())),
    }

    if let Some(dir) = bundle {
        match Bundle::compile(dir) {
            Ok(bundle) => report.ok(
                "bundle",
                format!("'{}' compiles, digest {}", bundle.name, bundle.manifest_digest()),
            ),
            Err(e) => report.missing("bundle", format!("{}: {e}", dir.display())),
        }
    }

    report
}
