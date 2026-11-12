//! The composed-tree check (#896 round 2, D1): what dsh actually composes
//! under a seat's own home, read before the session starts.
//!
//! `dsh --profile headless --patch <the seat's overlay> --dump-config` runs
//! under the seat's own environment and home (0.06–0.07 s, about 64 MB, no
//! model call). Every row it names must be one the controller measured for
//! the release it observed: the 99 rows of 0.2.1-rc.2 and rc.3
//! (`docs/evidence/adapters/dsh-hands-round2/dump-stdio.txt`), or
//! 0.2.2-rc.1's 100, that set minus `tool-subagent-list-agents` and plus
//! `tool-executor-native` and `tool-executor-ptc`
//! (`docs/evidence/adapters/dsh-hands-0-2-2.md`). The one
//! `dsh-mcp-client` row must be Brokkr's `brokkr-hands`. An extra row — an
//! operator home's own `dsh-mcp-client` insert, any `dsh-tool-*` a profile
//! patch adds, or another measured release's row — refuses by its id and
//! package, and so does a second MCP client, a repeated id, a hands row
//! left disabled or a dump that cannot be read. A hands seat is never
//! served a composition nobody measured.

use std::process::{Command, Stdio};

use thiserror::Error;

use super::{Release, HANDS_ID, MCP_CLIENT};

/// A dump larger than this is not the measured shape, and is refused
/// before it is parsed.
const MAX_DUMP_BYTES: usize = 1 << 20;

/// Why a seat's composed dsh tree is not the measured one.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum Composition {
    #[error(
        "refusing to invoke the dsh driver: the composed dsh tree carries row '{id}' of package \
         '{name}', which the measured hands composition (dsh {release}) does not name, so a \
         seat would be served a row nobody measured (#896 D1)"
    )]
    Unmeasured {
        id: String,
        name: String,
        release: &'static str,
    },
    #[error(
        "refusing to invoke the dsh driver: the composed dsh tree's release cannot be read as \
         one the hands rows were measured on ({}), so the seat is admitted on no measured \
         composition (#896 D1)",
        super::MEASURED_VERSIONS.join(", ")
    )]
    UnknownRelease,
    #[error(
        "refusing to invoke the dsh driver: the composed dsh tree carries no '{HANDS_ID}' row, \
         so the seat's one effectful tool would not be served (#896 D1)"
    )]
    NoHands,
    #[error(
        "refusing to invoke the dsh driver: the composed dsh tree carries a second \
         dsh-mcp-client row '{id}', so the seat would be served a second effectful server \
         beside Brokkr's (#896 D1)"
    )]
    SecondClient { id: String },
    #[error(
        "refusing to invoke the dsh driver: the composed dsh tree carries row '{id}' enabled, \
         which the hands rows disable, so the seat would keep an effectful tool beside the box \
         (#896 D1)"
    )]
    Enabled { id: String },
    #[error(
        "refusing to invoke the dsh driver: the composed dsh tree carries row '{id}' disabled, \
         so the seat's one effectful tool would not be served (#896 D1)"
    )]
    HandsDisabled { id: String },
    #[error(
        "refusing to invoke the dsh driver: the composed dsh tree names row '{id}' twice, so no \
         row it reads can be trusted (#896 D1)"
    )]
    Repeated { id: String },
    #[error(
        "refusing to invoke the dsh driver: the composed dsh tree cannot be read as the one \
         sequence of id and package rows dsh writes, so the seat is admitted on no measured \
         composition (#896 D1)"
    )]
    Unreadable,
    #[error(
        "refusing to invoke the dsh driver: `dsh --dump-config` failed ({status}), so the \
         composed tree could not be read and the seat is not launched (#896 D1)"
    )]
    DumpFailed { status: String },
}

/// One top-level row of the dump.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Row {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) disabled: bool,
}

/// The release the composed tree's own binary answers, through the same
/// bounded `--version` probe the launch used. The gate reads it again
/// rather than threading it through the launch's mode carrier, which is
/// outside this unit's files. An unreadable answer is no release; the gate
/// still runs the dump first, so a driver the probe cannot start keeps
/// today's unreadable-tree refusal.
pub(super) fn release(command: &Command) -> Option<Release> {
    let program = command.get_program().to_string_lossy().into_owned();
    let observed = super::super::observed_version(&[program, "--version".to_string()]);
    observed.as_deref().and_then(Release::named)
}

/// Judge what `run` makes of `command` under `release`. `run` is the one
/// substitutable function — a closure seam (decision 0071 ruling 10) — so
/// a test reads the exact argv and environment the seat's tree is composed
/// under, and feeds rows to judge, without spawning anything. The command
/// itself is built by the one spawn builder, so the probe and the session
/// share their working directory, environment, secret bindings and
/// removals.
pub(super) fn composed_with(
    mut command: Command,
    release: Option<Release>,
    run: impl FnOnce(&mut Command) -> std::io::Result<std::process::Output>,
) -> Result<(), Composition> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = run(&mut command).map_err(|_| Composition::Unreadable)?;
    if !output.status.success() {
        return Err(Composition::DumpFailed {
            status: output
                .status
                .code()
                .map_or_else(|| "by signal".to_string(), |code| format!("exit {code}")),
        });
    }
    if output.stdout.len() > MAX_DUMP_BYTES {
        return Err(Composition::Unreadable);
    }
    let rows = rows(&output.stdout)?;
    judge(release.ok_or(Composition::UnknownRelease)?, &rows)
}

/// Every row of a `--dump-config` sequence. dsh writes `- id:` at column
/// zero and the row's `name:` directly beneath it; any other shape is not
/// read as a row.
pub(crate) fn rows(bytes: &[u8]) -> Result<Vec<Row>, Composition> {
    let text = std::str::from_utf8(bytes).map_err(|_| Composition::Unreadable)?;
    let mut rows = Vec::new();
    let mut awaiting: Option<Row> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("- id: ") {
            if let Some(row) = awaiting.take() {
                rows.push(row);
            }
            awaiting = Some(Row {
                id: unquote(rest.trim()),
                name: String::new(),
                disabled: false,
            });
            continue;
        }
        let Some(row) = awaiting.as_mut() else {
            continue;
        };
        if let Some(rest) = line.strip_prefix("  name: ") {
            if !row.name.is_empty() {
                return Err(Composition::Unreadable);
            }
            row.name = unquote(rest.trim());
        } else if let Some(rest) = line.strip_prefix("  disabled: ") {
            row.disabled = rest.trim() == "true";
        } else if !line.is_empty() && !line.starts_with('#') && !line.starts_with("  ") {
            return Err(Composition::Unreadable);
        }
    }
    if let Some(row) = awaiting.take() {
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(Composition::Unreadable);
    }
    for row in &rows {
        if row.name.is_empty() {
            return Err(Composition::Unreadable);
        }
    }
    Ok(rows)
}

/// One YAML single-quoted scalar's body, or the bare text as written.
fn unquote(value: &str) -> String {
    match value
        .strip_prefix('\'')
        .and_then(|rest| rest.strip_suffix('\''))
    {
        Some(body) => body.replace("''", "'"),
        None => value.to_string(),
    }
}

/// Judge the rows against `release`'s measured composition.
pub(crate) fn judge(release: Release, rows: &[Row]) -> Result<(), Composition> {
    let mut seen: Vec<&str> = Vec::new();
    for row in rows {
        if seen.contains(&row.id.as_str()) {
            return Err(Composition::Repeated { id: row.id.clone() });
        }
        seen.push(&row.id);
        if row.name == MCP_CLIENT {
            if row.id != HANDS_ID {
                return Err(Composition::SecondClient { id: row.id.clone() });
            }
            if row.disabled {
                return Err(Composition::HandsDisabled { id: row.id.clone() });
            }
        } else if !measured(release, &row.id, &row.name) {
            return Err(Composition::Unmeasured {
                id: row.id.clone(),
                name: row.name.clone(),
                release: release.label(),
            });
        }
        if super::disabled(release, &row.id) && !row.disabled {
            return Err(Composition::Enabled { id: row.id.clone() });
        }
    }
    match seen.contains(&HANDS_ID) {
        true => Ok(()),
        false => Err(Composition::NoHands),
    }
}

/// Whether `(id, name)` is one of `release`'s measured top-level rows.
/// 0.2.2-rc.1's set is 0.2.1's 99 minus the removed row and plus the two
/// executor rows; the delta tables keep the 99 measured once (decision
/// 0071 ruling 5).
fn measured(release: Release, id: &str, name: &str) -> bool {
    match release {
        Release::V0_2_1 => MEASURED_COMPOSITION.contains(&(id, name)),
        Release::V0_2_2 => {
            ADDED_0_2_2.contains(&(id, name))
                || (MEASURED_COMPOSITION.contains(&(id, name))
                    && !REMOVED_0_2_2.contains(&(id, name)))
        }
    }
}

/// The rows 0.2.2-rc.1 adds to 0.2.1's measured set (2026-11-11): its
/// native tool executor, enabled, and the programmatic tool-calling
/// executor, disabled by default.
pub(crate) const ADDED_0_2_2: [(&str, &str); 2] = [
    (
        "tool-executor-native",
        "@deepseek-ai/dsh-tool-executor-native",
    ),
    ("tool-executor-ptc", "@deepseek-ai/dsh-tool-executor-ptc"),
];

/// The one 0.2.1 measured row 0.2.2-rc.1 no longer carries.
pub(crate) const REMOVED_0_2_2: [(&str, &str); 1] = [(
    "tool-subagent-list-agents",
    "@deepseek-ai/dsh-tool-subagent-control/list-agents",
)];

/// The 99 top-level rows measured on 0.2.1-rc.2 and rc.3, in the
/// controller's dump order (`dsh-hands-round2/dump-stdio.txt`). The
/// 0.2.2-rc.1 set is this table minus [`REMOVED_0_2_2`] plus
/// [`ADDED_0_2_2`].
pub(crate) const MEASURED_COMPOSITION: [(&str, &str); 99] = [
    (
        "tool-plugin-manager",
        "@deepseek-ai/dsh-plugin-manager/tools",
    ),
    ("plugin-manager", "@deepseek-ai/dsh-plugin-manager"),
    ("timer", "@deepseek-ai/cordis-plugin-timer"),
    ("hmr", "@deepseek-ai/dsh-hmr"),
    ("llm", "@deepseek-ai/dsh-llm"),
    (
        "deepseek-llm-api-extensions",
        "@deepseek-ai/dsh-deepseek-llm-api-extensions",
    ),
    ("session", "@deepseek-ai/dsh-session"),
    (
        "session-log-deepseek",
        "@deepseek-ai/dsh-session-log-deepseek",
    ),
    ("typert", "@deepseek-ai/dsh-typert-registry"),
    ("typert-loader", "@deepseek-ai/dsh-typert-loader"),
    ("typert-gateway", "@deepseek-ai/dsh-api-gateway"),
    ("remote-access", "@deepseek-ai/dsh-api-remote-access"),
    ("session-title", "@deepseek-ai/dsh-session-title"),
    (
        "session-title-llm",
        "@deepseek-ai/dsh-session-title-first-prompt-llm",
    ),
    ("user-questions", "@deepseek-ai/dsh-user-questions"),
    ("agent", "@deepseek-ai/dsh-agent"),
    (
        "plugin-package-inventory-deepseek",
        "@deepseek-ai/dsh-plugin-package-inventory-deepseek",
    ),
    (
        "agent-default-model",
        "@deepseek-ai/dsh-agent-default-model",
    ),
    ("jobs", "@deepseek-ai/dsh-jobs-local"),
    ("llm-retry", "@deepseek-ai/dsh-llm-retry"),
    ("config-editor", "@deepseek-ai/dsh-config-editor"),
    ("settings", "@deepseek-ai/dsh-settings"),
    ("authorization", "@deepseek-ai/dsh-authorization"),
    (
        "deepseek-account",
        "@deepseek-ai/dsh-deepseek-account-platform",
    ),
    ("credentials", "@deepseek-ai/dsh-credentials-local"),
    ("llm-pi-ai", "@deepseek-ai/dsh-llm-pi-ai"),
    (
        "session-persistence-jsonl",
        "@deepseek-ai/dsh-session-persistence-jsonl",
    ),
    ("attachment-local", "@deepseek-ai/dsh-attachment-local"),
    (
        "session-query-sqlite",
        "@deepseek-ai/dsh-session-query-sqlite",
    ),
    ("working-directory", "@deepseek-ai/dsh-working-directory"),
    (
        "tool-working-directory",
        "@deepseek-ai/dsh-tool-working-directory",
    ),
    ("session-projection", "@deepseek-ai/dsh-session-projection"),
    ("storage", "@deepseek-ai/dsh-storage"),
    ("storage-json", "@deepseek-ai/dsh-storage-json"),
    ("storage-domain", "@deepseek-ai/dsh-storage-domain"),
    (
        "session-projection-cache",
        "@deepseek-ai/dsh-session-projection-cache",
    ),
    ("otel", "@deepseek-ai/dsh-otel"),
    (
        "session-telemetry-otel",
        "@deepseek-ai/dsh-session-telemetry-otel",
    ),
    ("subprocess", "@deepseek-ai/dsh-subprocess-local"),
    ("sandbox", "@deepseek-ai/dsh-sandbox-local"),
    ("sandbox-policy", "@deepseek-ai/dsh-sandbox-policy"),
    ("bash-sandbox", "@deepseek-ai/dsh-bash-sandbox"),
    ("pwsh-sandbox", "@deepseek-ai/dsh-pwsh-sandbox"),
    ("approval", "@deepseek-ai/dsh-user-approval"),
    ("permission", "@deepseek-ai/dsh-permission-presets"),
    ("shell-env", "@deepseek-ai/dsh-shell-env"),
    ("tool-bash", "@deepseek-ai/dsh-tool-bash"),
    ("tool-pwsh", "@deepseek-ai/dsh-tool-pwsh"),
    ("tool-jobs", "@deepseek-ai/dsh-tool-jobs"),
    (
        "fs-observation-policy",
        "@deepseek-ai/dsh-fs-observation-policy",
    ),
    ("tool-fs", "@deepseek-ai/dsh-tool-fs"),
    ("tool-fs-search", "@deepseek-ai/dsh-tool-fs-search"),
    ("agent-instructions", "@deepseek-ai/dsh-agent-instructions"),
    ("skill", "@deepseek-ai/dsh-skill"),
    ("skill-filesystem", "@deepseek-ai/dsh-skill-filesystem"),
    ("tool-skill", "@deepseek-ai/dsh-tool-skill"),
    ("commands", "@deepseek-ai/dsh-commands"),
    ("command-feedback", "@deepseek-ai/dsh-command-feedback"),
    ("goal", "@deepseek-ai/dsh-goal"),
    ("goal-round-driver", "@deepseek-ai/dsh-goal-round-driver"),
    ("command-goal", "@deepseek-ai/dsh-command-goal"),
    ("plan-mode", "@deepseek-ai/dsh-plan-mode"),
    ("token-meter", "@deepseek-ai/dsh-token-meter"),
    ("compaction-basic", "@deepseek-ai/dsh-compaction-basic"),
    ("command-compact", "@deepseek-ai/dsh-command-compact"),
    ("subagent", "@deepseek-ai/dsh-subagent"),
    (
        "subagent-spawn-in-process",
        "@deepseek-ai/dsh-subagent-spawn-in-process",
    ),
    (
        "subagent-fork-in-process",
        "@deepseek-ai/dsh-subagent-fork-in-process",
    ),
    (
        "tool-subagent-control",
        "@deepseek-ai/dsh-tool-subagent-control",
    ),
    (
        "tool-subagent-list-agents",
        "@deepseek-ai/dsh-tool-subagent-control/list-agents",
    ),
    ("tool-subagent", "@deepseek-ai/dsh-tool-subagent"),
    ("tool-subagent-fork", "@deepseek-ai/dsh-tool-subagent"),
    ("ptc-runtime", "@deepseek-ai/dsh-ptc-runtime-node"),
    ("workflow-ptc", "@deepseek-ai/dsh-workflow-ptc"),
    ("tool-workflow", "@deepseek-ai/dsh-tool-workflow"),
    (
        "timeout-policy",
        "@deepseek-ai/dsh-tool-call-timeout-policy",
    ),
    ("spill-local", "@deepseek-ai/dsh-spill-local"),
    ("spill-policy", "@deepseek-ai/dsh-spill-policy"),
    (
        "session-checkpoint-policy",
        "@deepseek-ai/dsh-session-checkpoint-policy",
    ),
    (
        "tool-result-pruner",
        "@deepseek-ai/dsh-compaction-tool-result-pruner",
    ),
    ("image-offload", "@deepseek-ai/dsh-compaction-image-offload"),
    ("tool-todo", "@deepseek-ai/dsh-tool-todo"),
    ("tool-goal", "@deepseek-ai/dsh-tool-goal"),
    (
        "repeat-tool-reminder",
        "@deepseek-ai/dsh-repeat-tool-reminder",
    ),
    ("web", "@deepseek-ai/dsh-web"),
    (
        "web-search-deepseek",
        "@deepseek-ai/dsh-web-search-deepseek",
    ),
    ("web-deepseek-api", "@deepseek-ai/dsh-web-deepseek-api"),
    ("web-fetch-http", "@deepseek-ai/dsh-web-fetch-http"),
    ("tool-web", "@deepseek-ai/dsh-tool-web"),
    ("mcp-resources", "@deepseek-ai/dsh-mcp-resources"),
    ("tools", "@deepseek-ai/dsh-tools"),
    ("system-prompt", "@deepseek-ai/dsh-system-prompt"),
    ("agent-loop", "@deepseek-ai/dsh-agent-loop"),
    ("fs-sandbox", "@deepseek-ai/dsh-fs-sandbox"),
    ("llm-deepseek", "@deepseek-ai/dsh-llm-deepseek-api-key"),
    (
        "llm-deepseek-account",
        "@deepseek-ai/dsh-llm-deepseek-account",
    ),
    ("headless-startup", "@deepseek-ai/dsh-headless/startup"),
    ("headless-runner", "@deepseek-ai/dsh-headless"),
    // The measured dump is data, so its hands row keeps the words measured
    // verbatim; `the_writer_the_composition_and_the_report_spell_one_hands_identity`
    // pins them equal to `HANDS_ID` and `MCP_CLIENT`.
    ("brokkr-hands", "@deepseek-ai/dsh-mcp-client"),
];
