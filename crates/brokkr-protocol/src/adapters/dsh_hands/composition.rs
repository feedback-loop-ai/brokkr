//! The composed-tree check (#896 round 2, D1): what dsh actually composes
//! under a seat's own home, read before the session starts.
//!
//! `dsh --profile headless --patch <the seat's overlay> --dump-config` runs
//! under the seat's own environment and home (0.06–0.07 s, about 64 MB, no
//! model call). Every row it names must be one the controller measured for
//! the release it observed: the 99 rows of 0.2.1-rc.2 and rc.3
//! (`docs/evidence/adapters/dsh-hands-round2/dump-stdio.txt`), or
//! 0.2.2-rc.1's 100, that set minus `tool-subagent-list-agents` and plus
//! `tool-executor-native` and `tool-executor-ptc`, or 0.2.2-rc.5's 102,
//! 0.2.2-rc.1's plus `agent-team` and `tool-agent-team` with
//! `deepseek-account` under its new package
//! (`docs/evidence/adapters/dsh-hands-0-2-2.md`). The one
//! `dsh-mcp-client` row must be Brokkr's `brokkr-hands`. An extra row — an
//! operator home's own `dsh-mcp-client` insert, any `dsh-tool-*` a profile
//! patch adds, or another measured release's row — refuses by its id and
//! package, and so does a second MCP client, a repeated id, a hands row
//! left disabled or a dump that cannot be read. A hands seat is never
//! served a composition nobody measured.
//!
//! The whole dump is read by two passes, not only the one `sandbox-policy`
//! row. First every line is refused for a break, control or tab. Then the
//! line-property closure refuses any line that leaves a quote, a non-empty
//! flow collection or an empty sequence item open, independent of the row
//! grammar and its block bookkeeping. Only then does the row grammar read
//! the dump: every row's two-space keys are closed to `name:`, `disabled:`,
//! `config:` and `inject:` each at most once, and a deeper line is admitted
//! only while `config:` or `inject:` opened a block. The closure's home is
//! [`rows::carriers`] and the grammar's one home is [`rows`];
//! [`closure`] stays a `pub(crate)` forwarder to the closure so the unit
//! tests read one seam without a second path into the module.

use std::process::Command;

use thiserror::Error;

use super::{Mode, Release, HANDS_ID, MCP_CLIENT, SANDBOX_POLICY, TEAM_TOOL_ROW};

mod rows;

pub(crate) use self::rows::rows;

/// The dump's line-property closure: refuse [`Composition::Unreadable`] if
/// any line leaves a quote, a non-empty flow collection or an empty
/// sequence item open. [`rows`] runs it after the dump-wide break, control
/// and tab rule and before the row grammar; the closure's unit tests read
/// this one seam alone.
pub(crate) fn closure(text: &str) -> Result<(), Composition> {
    rows::carriers::check(text)
}

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
        "refusing to invoke the dsh driver: the composed dsh tree carries the \
         '{id}' row disabled, so dsh's own sandbox policy is not in force and the \
         mode the gate reads is not the mode the seat runs under (#896 D1)"
    )]
    PolicyDisabled { id: String },
    #[error(
        "refusing to invoke the dsh driver: the composed dsh tree's '{SANDBOX_POLICY}' mode is \
         '{composed}', not the seat's '{expected}', so dsh's own sandbox would not be the mode \
         the office's access selected (#896 D1)"
    )]
    Mode {
        composed: String,
        expected: &'static str,
    },
    #[error(
        "refusing to invoke the dsh driver: the composed dsh tree carries no '{SANDBOX_POLICY}' \
         mode, so the seat's sandbox mode cannot be read and the seat is not launched (#896 D1)"
    )]
    NoMode,
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
    /// The `config.mode` a `sandbox-policy` row carries, `None` for any
    /// other row or a row that names no mode. Read by the same bounded
    /// parse as `name:` and `disabled:`, never by a general YAML reader.
    pub(crate) mode: Option<String>,
}

/// Judge what `run` makes of `command` under `release`. `run` is the one
/// substitutable function — a closure seam (decision 0071 ruling 10) — so
/// a test reads the exact argv and environment the seat's tree is composed
/// under, and feeds rows to judge, without spawning anything. The command
/// itself is built by the one spawn builder, so the probe and the session
/// share their working directory, environment, secret bindings and
/// removals; `run` sets its streams, as `spawned::output` does.
pub(super) fn composed_with(
    mut command: Command,
    release: Release,
    mode: Mode,
    run: impl FnOnce(&mut Command) -> std::io::Result<std::process::Output>,
) -> Result<(), Composition> {
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
    judge(release, &rows)?;
    judge_mode(&rows, mode)
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

/// Judge the composed `sandbox-policy` mode against the mode the seat's
/// office selected. A tree with no such row, or a row naming no mode, is
/// refused: an unread mode cannot be shown to be the seat's own, and
/// `danger-full-access` must never reach a hands seat (1052f's chief
/// finding, #1052 c-6821890847). This is a sibling of [`judge`] so the row
/// judge keeps its one signature and every existing exact-text test stays
/// as it was.
pub(crate) fn judge_mode(rows: &[Row], expected: Mode) -> Result<(), Composition> {
    let composed = rows
        .iter()
        .find(|row| row.id == SANDBOX_POLICY)
        .and_then(|row| row.mode.as_deref());
    match composed {
        None => Err(Composition::NoMode),
        Some(mode) if mode == expected.word() => Ok(()),
        Some(composed) => Err(Composition::Mode {
            composed: composed.to_string(),
            expected: expected.word(),
        }),
    }
}

/// Whether `(id, name)` is one of `release`'s measured top-level rows.
/// 0.2.2-rc.1's set is 0.2.1's 99 minus the removed row and plus the two
/// executor rows, and 0.2.2-rc.5's is 0.2.2-rc.1's with its own delta; the
/// delta tables keep the 99 measured once (decision 0071 ruling 5).
fn measured(release: Release, id: &str, name: &str) -> bool {
    match release {
        Release::V0_2_1 => MEASURED_COMPOSITION.contains(&(id, name)),
        Release::V0_2_2 => {
            ADDED_0_2_2.contains(&(id, name))
                || (MEASURED_COMPOSITION.contains(&(id, name))
                    && !REMOVED_0_2_2.contains(&(id, name)))
        }
        Release::V0_2_2Rc5 => {
            ADDED_0_2_2_RC5.contains(&(id, name))
                || (measured(Release::V0_2_2, id, name) && !REMOVED_0_2_2_RC5.contains(&(id, name)))
        }
    }
}

/// The rows 0.2.2-rc.5 adds to 0.2.2-rc.1's measured set (2026-11-19): the
/// agent-team host row and its tool row, both enabled by default, and the
/// `deepseek-account` row under its new package name.
pub(crate) const ADDED_0_2_2_RC5: [(&str, &str); 3] = [
    ("agent-team", "@deepseek-ai/dsh-agent-team"),
    (TEAM_TOOL_ROW, "@deepseek-ai/dsh-tool-agent-team"),
    ("deepseek-account", "@deepseek-ai/dsh-deepseek-account"),
];

/// The one 0.2.2-rc.1 row 0.2.2-rc.5 no longer carries: `deepseek-account`
/// under its old package name.
pub(crate) const REMOVED_0_2_2_RC5: [(&str, &str); 1] = [(
    "deepseek-account",
    "@deepseek-ai/dsh-deepseek-account-platform",
)];

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
