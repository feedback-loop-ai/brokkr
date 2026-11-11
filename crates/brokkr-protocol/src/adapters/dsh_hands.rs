//! A dsh seat's hands (#896; decisions 0043 and 0069): the rows the seat's
//! one `--patch` overlay carries where its office declares hands a
//! boundary boxes, so Brokkr's workspace tool is the seat's only
//! effectful tool.
//!
//! dsh 0.2 composes every model-facing tool from its own profile row, and
//! an overlay row `disabled: true` removes it, as the headless profile's
//! `credentials` row is removed. Measured on 2026-10-26 on dsh 0.2.1-rc.2
//! and 0.2.1-rc.3 (headless, the local `spark-glm` route): with the ten
//! rows below disabled, the model's own tool list went from 25 tools to
//! nine bookkeeping tools, and `--dump-config` showed each row
//! `disabled: true` (docs/evidence/adapters/slice-two-dsh-hands.md).
//! Measured again on 2026-11-11 on dsh 0.2.2-rc.1 (npm `latest`): with the
//! same ten rows disabled the list is eight bookkeeping tools, and that
//! release adds a disabled programmatic executor the overlay also pins
//! (docs/evidence/adapters/dsh-hands-0-2-2.md). Each measured release has
//! its own row set and composed tree, so a seat on any other release is
//! refused before it starts rather than served a tool row nobody measured.
//!
//! Beside them the overlay sets dsh's own sandbox, defence in depth under
//! the box: `read-only` for an office that reviews and writes no worktree
//! file, `workspace-write` for one that writes, from the access the office
//! declares ([`ACCESS`], [`crate::native_controls::Access`]). And it inserts
//! Brokkr's workspace server, `brokkr hands serve` bound as the final
//! check binds it, as the engine's `dsh-mcp-client` row (U0 D03, U0b D09).
//!
//! The row carries `transport: stdio` and `failOnStartupError: true`: on
//! 0.2.1-rc.2 a row without `transport` never started its server and dsh
//! exited 0 with no warning at all, and `failOnStartupError` alone does not
//! refuse a failed server either (docs/evidence/adapters/dsh-hands-round2).
//! So the driver also reads the startup it ran: a hand that did not
//! activate fails the seat closed ([`report`]), and every launch first
//! composes the seat's own tree with `--dump-config` and refuses any row
//! outside the observed release's measured set ([`composition`]).

use serde_json::Value;
use thiserror::Error;

use super::mcp::{self, McpRefusal};
use super::route_overlay::{self, Refused};
use super::stderr::{CapturedStderr, StderrEnd};
use super::{dsh_sandbox_row_for, observed_version};
use crate::dsh_sandbox::{self, GitScope, SeatGitStore};
use crate::hands::{GitFacts, SERVER_NAME};
use crate::native_controls::Access;

mod composition;
mod report;

pub(crate) use composition::Composition;
pub(crate) use report::{Enforcement, ReportRefusal};

/// The driver-input key the engine marks a boxed site's hands access under,
/// `read-only` or `writes`, beside its `hands` marker: a spawn-time fact of
/// the site, written by the engine alone from the office's declaration
/// (#896 round 2).
pub const ACCESS: &str = "seat_access";

/// The exact version strings the measured release families cover, each in
/// one home: [`MEASURED_VERSIONS`], [`Release::versions`] and
/// [`Release::label`] all derive from these (decision 0071 ruling 5).
const V0_2_1_RC_2: &str = "0.2.1-rc.2";
const V0_2_1_RC_3: &str = "0.2.1-rc.3";
const V0_2_2_RC_1: &str = "0.2.2-rc.1";

/// The dsh releases the hands rows were measured on: 0.2.1-rc.2 and
/// 0.2.1-rc.3 on 2026-10-26, and 0.2.2-rc.1 on 2026-11-11
/// (docs/evidence/adapters/dsh-hands-0-2-2.md).
const MEASURED_VERSIONS: [&str; 3] = [V0_2_1_RC_2, V0_2_1_RC_3, V0_2_2_RC_1];

/// One measured dsh release family. The set is closed: a version the
/// controller did not measure is refused by name, never matched by prefix
/// or range (decision 0071 rulings 2 and 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Release {
    /// 0.2.1-rc.2 and 0.2.1-rc.3, measured on 2026-10-26.
    V0_2_1,
    /// 0.2.2-rc.1, measured on 2026-11-11 (npm `latest`).
    V0_2_2,
}

impl Release {
    /// Every measured release, in the order the refusal names them.
    const ALL: [Release; 2] = [Release::V0_2_1, Release::V0_2_2];

    /// The exact versions this release family covers, each matched whole.
    /// Each variant names its own versions, so a version appended to
    /// [`MEASURED_VERSIONS`] alone is measured nowhere and refuses.
    fn versions(self) -> &'static [&'static str] {
        match self {
            Release::V0_2_1 => &[V0_2_1_RC_2, V0_2_1_RC_3],
            Release::V0_2_2 => &[V0_2_2_RC_1],
        }
    }

    /// The release the exact version `version` is, `None` for any other.
    fn named(version: &str) -> Option<Release> {
        Release::ALL
            .into_iter()
            .find(|release| release.versions().contains(&version))
    }

    /// The release as a composition refusal names it: 0.2.1's first measured
    /// version, the label today's refusals carry.
    fn label(self) -> &'static str {
        match self {
            Release::V0_2_1 => V0_2_1_RC_2,
            Release::V0_2_2 => V0_2_2_RC_1,
        }
    }
}

/// Every model-facing tool row of the measured releases that can read,
/// write or run anything, or reach the network, or spawn an agent: shell
/// (`bash`, `pwsh`, the `job_*` tools), files (`read`, `write`, `edit`,
/// `read_image`, `glob`, `grep`), skills, subagent and fork spawns,
/// workflows and the web (`web_fetch`, `web_search`).
pub(super) const DISABLED_ROWS: [&str; 10] = [
    "tool-bash",
    "tool-pwsh",
    "tool-jobs",
    "tool-fs",
    "tool-fs-search",
    "tool-skill",
    "tool-subagent",
    "tool-subagent-fork",
    "tool-workflow",
    "tool-web",
];

/// The programmatic tool-calling executor 0.2.2-rc.1 adds: disabled by
/// default there, and pinned disabled by the hands overlay so a profile
/// that enables it cannot give a hands seat a code-running executor. It
/// is not a 0.2.1 row, so 0.2.1's overlay does not name it.
pub(super) const PTC_ROW: &str = "tool-executor-ptc";

/// Whether `id` is one of the effectful rows `release`'s hands overlay
/// disables. 0.2.1 is the ten common rows; 0.2.2-rc.1 adds the
/// programmatic executor. Its native executor is the tool calling the
/// model is still offered, so it stays enabled.
pub(super) fn disabled(release: Release, id: &str) -> bool {
    match release {
        Release::V0_2_1 => DISABLED_ROWS.contains(&id),
        Release::V0_2_2 => DISABLED_ROWS.contains(&id) || id == PTC_ROW,
    }
}

/// The row whose `mode` is dsh's own sandbox.
pub(super) const SANDBOX_POLICY: &str = "sandbox-policy";

/// The one row Brokkr inserts: the overlay writer writes it, the composed
/// tree's judge requires it, and dsh's startup report names it. One
/// spelling, so the three cannot drift (decision 0071 ruling 5).
pub(super) const HANDS_ID: &str = "brokkr-hands";

/// The dsh package that serves one MCP server per inserted row.
pub(super) const MCP_CLIENT: &str = "@deepseek-ai/dsh-mcp-client";

/// The scoped runner a linked worktree's native commits need (decision
/// 0054), its row first.
type Staged = (String, GitScope, SeatGitStore);

/// dsh's own sandbox mode under the box: the two of its three modes a seat
/// with hands may run under. `danger-full-access` is never one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    ReadOnly,
    WorkspaceWrite,
}

impl Mode {
    /// The mode as dsh's `sandbox-policy` row spells it, and as the seat
    /// record's `sandbox` field admits it.
    fn word(self) -> &'static str {
        match self {
            Mode::ReadOnly => "read-only",
            Mode::WorkspaceWrite => "workspace-write",
        }
    }

    /// The mode `word` spells, `None` for any other word.
    pub(super) fn named(word: &str) -> Option<Mode> {
        [Mode::ReadOnly, Mode::WorkspaceWrite]
            .into_iter()
            .find(|mode| mode.word() == word)
    }

    /// The mode the office's declared access selects: an office that
    /// reviews and writes no worktree file runs read-only, one that writes
    /// runs workspace-write. The typed carrier is decoded here, once, at
    /// the driver edge; a missing or unknown value is refused, never
    /// guessed from the seat's gate class (decision 0071 rulings 2 and 3).
    fn of(input: &Value) -> Result<Mode, Refusal> {
        let access: Access = input
            .get(ACCESS)
            .cloned()
            .ok_or(Refusal::NoAccess)
            .and_then(|value| serde_json::from_value(value).map_err(|_| Refusal::NoAccess))?;
        match access {
            Access::ReadOnly => Ok(Mode::ReadOnly),
            Access::Writes => Ok(Mode::WorkspaceWrite),
        }
    }
}

/// Why a dsh seat whose office declares hands is not launched. Each text is
/// the seat's launch refusal.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum Refusal {
    #[error(
        "refusing to invoke the dsh driver: the engine names no hands access for a seat whose \
         office declares hands, so dsh's own sandbox mode cannot be chosen (#896)"
    )]
    NoAccess,
    #[error(
        "refusing to invoke the dsh driver: {observed} is not a dsh release its hands rows were \
         measured on ({}), and a tool row nobody measured could stay enabled beside the box, so \
         the seat is not served its native tools (#896)",
        MEASURED_VERSIONS.join(", ")
    )]
    Unmeasured { observed: String },
    #[error(
        "refusing to invoke the dsh driver: the box's hands server path spans more than one \
         line, so it cannot be written as one overlay scalar"
    )]
    Multiline,
    #[error(transparent)]
    Rows(#[from] Refused),
    #[error("{}", .0.at_launch())]
    Unbound(McpRefusal),
    /// The scoped runner's refusal, in its own words (decision 0054).
    #[error("{0}")]
    Runner(String),
}

/// The launch refusal is still text in `adapters.rs`, so the conversion is
/// the boundary's.
impl From<Refusal> for String {
    fn from(refusal: Refusal) -> Self {
        refusal.to_string()
    }
}

/// The sandbox rows one dsh seat runs under, and the runner it stages. A
/// seat whose office declares boxed hands runs no native command, so it
/// stages no runner; any other seat's rows are the scoped runner's. The
/// two are exclusive and the choice is the enum's, so "has hands" is read
/// from actual boxed hands and never from whether some overlay row exists
/// (#896 round 3).
pub(super) enum Selection {
    Hands { rows: String, mode: Mode },
    Runner(Option<Box<Staged>>),
}

impl Selection {
    /// The rows the seat's one overlay carries: the hands rows for a hands
    /// seat, the scoped runner's staged row for a runner seat, and `None`
    /// where no scoped runner is staged, exactly as main wrote it (#896
    /// round 4). The conditions a runner is staged under are
    /// [`super::dsh_git_runner_scope`]'s, its one home (decision 0071
    /// ruling 5).
    pub(super) fn rows(&self) -> Option<&str> {
        match self {
            Selection::Hands { rows, .. } => Some(rows),
            Selection::Runner(staged) => staged.as_ref().map(|staged| staged.0.as_str()),
        }
    }

    /// The private store a runner seat stages and the mode a hands seat
    /// runs under; the two are never both present.
    pub(super) fn into_parts(self) -> (Option<Staged>, Option<Mode>) {
        match self {
            Selection::Hands { mode, .. } => (None, Some(mode)),
            Selection::Runner(staged) => (staged.map(|staged| *staged), None),
        }
    }
}

/// Brokkr's sandbox rows for one dsh seat and the runner they stage. A seat
/// whose office declares boxed hands runs no native command, so it stages
/// no runner and its rows are the hands rows ([`hands_rows`]), once the
/// installed release is one they were measured on. Any other seat's rows
/// are the scoped runner's a linked worktree needs, under the mode
/// `DSH_PERMISSION_MODE` names, exactly as before (decision 0054).
pub(super) fn sandbox(
    bin: &str,
    workdir: &str,
    facts: &GitFacts,
    input: &Value,
) -> Result<Selection, Refusal> {
    let Some((command, args)) = mcp::dsh_hands(input, workdir).map_err(Refusal::Unbound)? else {
        let mode = std::env::var("DSH_PERMISSION_MODE").unwrap_or_default();
        let staged = dsh_sandbox_row_for(workdir, facts, &mode).map_err(Refusal::Runner)?;
        return Ok(Selection::Runner(staged.map(Box::new)));
    };
    let mode = Mode::of(input)?;
    let release = measured(observed_version(&[
        bin.to_string(),
        "--version".to_string(),
    ]))?;
    Ok(Selection::Hands {
        rows: hands_rows(release, mode, &command, &args)?,
        mode,
    })
}

/// The version drift check: the observed release must be one the rows were
/// measured on, matched whole. An unreadable answer is no release at all.
fn measured(observed: Option<String>) -> Result<Release, Refusal> {
    match observed {
        Some(version) => Release::named(&version).ok_or_else(|| Refusal::Unmeasured {
            observed: format!("dsh '{version}'"),
        }),
        None => Err(Refusal::Unmeasured {
            observed: "an unreadable dsh version".into(),
        }),
    }
}

/// The hands rows for a seat on `release` under `mode` and Brokkr's
/// workspace server `command` and `args`: every tool row that release's
/// measured set disables and the sandbox mode, checked by the bounded
/// reader against the one shape they may take ([`route_overlay::hands`]),
/// then the server row. A 0.2.2-rc.1 seat also pins the programmatic
/// executor disabled; a 0.2.1 seat names no such row.
fn hands_rows(
    release: Release,
    mode: Mode,
    command: &str,
    args: &[String],
) -> Result<String, Refusal> {
    let mut rows = String::from(
        "# Written by `brokkr driver dsh` for one seat whose office declares hands:\n\
         # every effectful tool row disabled, dsh's own sandbox under the box,\n\
         # and Brokkr's workspace server as the seat's only effectful tool.\n",
    );
    for row in DISABLED_ROWS {
        rows.push_str(&format!("- id: {row}\n  disabled: true\n"));
    }
    rows.push_str(&format!(
        "- id: {SANDBOX_POLICY}\n  config:\n    mode: {}\n",
        mode.word()
    ));
    route_overlay::hands(rows.as_bytes())?;
    if release == Release::V0_2_2 {
        rows.push_str(&format!("- id: {PTC_ROW}\n  disabled: true\n"));
    }
    rows.push_str(&format!(
        "- insert:\n\
         \x20   - id: {HANDS_ID}\n\
         \x20     name: '{MCP_CLIENT}'\n\
         \x20     config:\n\
         \x20       serverName: {SERVER_NAME}\n\
         \x20       transport: stdio\n\
         \x20       failOnStartupError: true\n\
         \x20       command: '{}'\n\
         \x20       args:\n",
        scalar(command)?
    ));
    for arg in args {
        rows.push_str(&format!("          - '{}'\n", scalar(arg)?));
    }
    Ok(rows)
}

/// One single-quoted YAML scalar's body, as every row this driver writes
/// quotes a path ([`dsh_sandbox::yaml_scalar`]).
fn scalar(value: &str) -> Result<String, Refusal> {
    dsh_sandbox::yaml_scalar(value).map_err(|_| Refusal::Multiline)
}

#[cfg(test)]
mod tests;

/// The composed-tree check as the launch states it, before any child
/// spawns (#896 round 2, D1). The command is the one
/// [`harness_command`](super::harness_command) built from the launch's own
/// argv, workdir, environment, bindings and removals.
pub(crate) fn composition_gate(command: std::process::Command) -> Result<(), Composition> {
    let release = composition::release(&command);
    composition::composed_with(command, release, |command| command.output())
}

/// dsh's startup outcome, classified once from the RAW captured stderr
/// before any masking or reasoning redaction (#896 H1; decision 0098 ruling
/// 4). `Err` is the report that fails the seat closed.
pub(crate) type Startup = Result<Enforcement, ReportRefusal>;

/// Classify dsh's own report from the RAW captured stderr, once, at the
/// capture edge. The raw text goes no further than this call: the stderr
/// the journal exposes is masked and reasoning-redacted afterwards, and a
/// masking that rewrites these words cannot change the outcome (#896 H1).
/// A capture that did not reach EOF refuses before its bytes are
/// classified: a warning the reader never returned cannot be ruled out, so
/// the seat fails closed on what cannot be read (fix rounds 430s4a4 and
/// 430s4a6 H1; decision 0071 rulings 3 and 8).
pub(crate) fn startup_report(captured: &CapturedStderr) -> Startup {
    match captured.end() {
        StderrEnd::Unreadable(error) => Err(ReportRefusal::UnreadableCapture(error.clone())),
        // The bounded settle keeps a descendant-held pipe from hanging the
        // driver, but what it took is not the child's whole report: the
        // gate refuses it exactly as it refuses a failed read (fix round
        // 430s4a6 H1; decision 0098 ruling 4).
        StderrEnd::Incomplete => Err(ReportRefusal::IncompleteCapture),
        StderrEnd::Eof => report::checked(&String::from_utf8_lossy(captured.bytes())),
    }
}

/// The startup dsh reported, decided from the outcome classified before
/// masking (#896 round 2, D0 and D4; #896 H1): a failed hand or an
/// unavailable sandbox refuses, and partial enforcement lands on the row
/// the operator reads. `sandbox` keeps the MODE the seat ran under, never
/// the word `partial`; the launcher's exact line rides the
/// contract-admitted `step_name` (#896 round 3).
pub(crate) fn startup_gate(
    startup: Startup,
    meta: &mut serde_json::Map<String, Value>,
    mode: Mode,
) -> Result<(), ReportRefusal> {
    match startup? {
        Enforcement::Partial => {
            meta.insert("sandbox".into(), Value::String(mode.word().into()));
            meta.insert("step_name".into(), Value::String(report::PARTIAL.into()));
            Ok(())
        }
        Enforcement::Full => Ok(()),
    }
}
