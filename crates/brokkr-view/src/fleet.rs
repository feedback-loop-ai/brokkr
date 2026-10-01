//! The fleet's rows (#491): a run as a listing shows it, with the three
//! facts the fleet is read by — what the run is called, how it stands
//! and was ruled, and which section it belongs to. Derived here once, so
//! the TUI, `brokkr runs` and the console can read them alike (decision
//! 0013); a renderer lays them out and derives none of them.

use brokkr_core::fold::{Cursor, RunState, Status};
use brokkr_core::policy::Severity;
use serde::Serialize;
use serde_json::Value;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    decided, fmt_dur, js, status_str, FleetView, RealmRuns, ResidualFinding, RunEntry, RunsView,
    ABSENT, KNOWN_STATUS, VIEW_VERSION,
};

#[derive(Serialize)]
pub struct RunRow {
    pub run_id: String,
    pub status: Option<String>,
    /// The status is one of the four the surfaces have a colour for.
    pub status_known: bool,
    pub phase: Option<String>,
    pub seq: Option<u64>,
    pub created_at: String,
    /// When the run's journal last moved, as its entry states it (#491).
    pub last_recorded_at: Option<String>,
    /// The **full** feature: the model stays terminal-agnostic and
    /// `--json` stays lossless. Clamping is the renderer's job.
    pub feature: String,
    /// Why this row carries no status, when it carries none: the fold
    /// error, verbatim. A quarantined run reads as `?` plus this line
    /// on every surface instead of vanishing from the fleet.
    pub detail: Option<String>,
    /// This run's residual findings, each carrying the operator's
    /// supersede annotation when one closes it (decision 0047 ruling
    /// 3): `brokkr runs --json` is a surface that prints a residual
    /// finding, so it prints the mark. The text table stays a digest —
    /// clamping and omission are the renderer's job, losslessness is
    /// the model's.
    pub residuals: Vec<ResidualFinding>,
    /// What the fleet calls the run: the feature's first line, clamped
    /// at a word to 60 display columns (#491). `feature` stays whole
    /// beside it.
    pub title: String,
    /// How the run stands and how it was last ruled (#491).
    pub verdict: Verdict,
    /// The seat the run's current effect is hired to, and which attempt
    /// of it, while an effect is requested or in flight (#503).
    pub hire: Option<Hire>,
}

/// A seat at work on a run's current effect, as the fold's cursor
/// names it (#503).
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Hire {
    pub seat: String,
    /// The attempt in flight, or the next one to start: one more than
    /// the attempts that have failed.
    pub attempt: u64,
}

/// The widest a title may be, in display columns: a wide character
/// counts two, a combining mark none. A renderer that sizes a column for
/// a whole title sizes it by this.
pub const TITLE_COLUMNS: usize = 60;

/// The widest a verdict cell may be, in display columns. A renderer
/// sizes its verdict column by it.
pub const VERDICT_COLUMNS: usize = 14;

/// A finished run stays in the fleet's recent section for a day.
const RECENT_MILLIS: i64 = 24 * 60 * 60 * 1000;

/// The longest deadline a seat this repository ships gives one attempt,
/// in seconds. The journal records no attempt's deadline, so the
/// staleness bound is stated against the longest one, and brokkr-cli's
/// `the_staleness_bound_is_the_longest_shipped_deadline_and_an_hour`
/// holds every recipe, agent and bundle in the repository to it.
const ATTEMPT_DEADLINE_SECONDS: i64 = 2 * 60 * 60;

/// How long past that deadline a running run's journal may stay silent:
/// time for the engine to end a timed-out attempt's process tree and
/// journal its failure, and for the next attempt to start.
const STALE_MARGIN_SECONDS: i64 = 60 * 60;

/// A running run whose journal has not moved for longer than this has
/// no engine at work on it (#503): a live attempt ends by its deadline,
/// and the engine journals that ending.
const STALE_AFTER_SECONDS: i64 = ATTEMPT_DEADLINE_SECONDS + STALE_MARGIN_SECONDS;

/// The terminal phase the engine concludes a run from as `stopped` —
/// the policy table's hard stop. A run stopped anywhere else was stopped
/// by an operator: the engine appends `run/stopped` only there and on an
/// accepted operator stop.
const STOP_PHASE: &str = "stop";

const ELLIPSIS: char = '…';

/// How a run stands, as the fleet reads it. The fold's four statuses,
/// with a quarantined run beside them and a stop split by who made it.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Standing {
    /// The journal does not fold; the row carries the fold's refusal.
    Quarantined,
    /// Waiting on the operator.
    Parked,
    Running,
    /// Completed.
    Shipped,
    /// The policy table ruled a hard stop.
    Stopped,
    /// An operator stopped or concluded the run.
    OperatorStopped,
}

impl Standing {
    /// The word every surface prints for it.
    pub fn label(self) -> &'static str {
        match self {
            Standing::Quarantined => "quarantined",
            Standing::Parked => "parked",
            Standing::Running => "running",
            Standing::Shipped => "shipped",
            Standing::Stopped | Standing::OperatorStopped => "stopped",
        }
    }
}

/// A run's verdict: how it stands, the ruling that settled it, and the
/// worst residual it carries, as a (structured value, rendered text)
/// pair like every displayed scalar in this crate.
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Verdict {
    pub standing: Standing,
    /// The rule id that settled the run, whole: the one that shipped or
    /// stopped it, or the one that parked it when a ruling routed
    /// nowhere. `None` for a run still working, one an operator stopped,
    /// and a park no ruling made.
    pub rule: Option<String>,
    /// The highest severity an open residual finding names. A finding
    /// the operator has superseded (decision 0047) is closed, and one
    /// valued `none` names no residual.
    pub residual: Option<Severity>,
    /// Why the run waits, verbatim, while it is parked.
    pub reason: Option<String>,
    /// The fleet's verdict cell, at most [`VERDICT_COLUMNS`] wide: the
    /// rule that shipped, stopped or parked the run without its own
    /// phase's prefix, and nothing yet for a running one. The residual
    /// is its own cell beside it.
    pub text: String,
}

/// Who must act on a run: the fleet's sections, listed in [`Section::ORDER`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    /// Parked, quarantined because the journal does not fold, or stale:
    /// whatever [`need`] names.
    NeedsYou,
    Running,
    /// Finished within the last 24 hours, by when its journal last
    /// moved: a run that waited days on a ruling and ended today is
    /// today's news.
    Recent,
    /// Finished and older.
    Older,
}

impl Section {
    pub const ORDER: [Section; 4] = [
        Section::NeedsYou,
        Section::Running,
        Section::Recent,
        Section::Older,
    ];

    /// The heading every surface prints for it.
    pub fn label(self) -> &'static str {
        match self {
            Section::NeedsYou => "needs you",
            Section::Running => "running",
            Section::Recent => "last 24h",
            Section::Older => "older",
        }
    }
}

/// The fleet's rows by section, every section in [`Section::ORDER`] and
/// each keeping the rows' own order. `now` is a parameter: a run whose
/// age cannot be read is never hidden among the older ones.
pub fn sections<'a>(rows: &'a [RunRow], now: &str) -> Vec<(Section, Vec<&'a RunRow>)> {
    Section::ORDER
        .iter()
        .map(|section| {
            let within = rows.iter().filter(|row| section_of(row, now) == *section);
            (*section, within.collect())
        })
        .collect()
}

fn section_of(row: &RunRow, now: &str) -> Section {
    if need(row, now).is_some() {
        return Section::NeedsYou;
    }
    if row.verdict.standing == Standing::Running {
        return Section::Running;
    }
    let moved = row.last_recorded_at.as_deref().unwrap_or(&row.created_at);
    match (js::parse_millis(moved), js::parse_millis(now)) {
        (Some(moved), Some(now)) if now - moved >= RECENT_MILLIS => Section::Older,
        _ => Section::Recent,
    }
}

// ------------------------------------------------------- what it needs

/// What a run in "needs you" asks of the operator (#503).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Need {
    /// Parked on a ruling or a limit; its verdict says which and why.
    Parked,
    /// Its journal does not fold; the row's `detail` is the refusal.
    Quarantined,
    /// Its journal folds to running and has not moved for longer than
    /// the staleness bound, so nothing drives it. `silent` is how long
    /// it has been quiet, rendered.
    Stale { silent: String },
}

impl Need {
    /// The word a row's standing cell prints for it.
    pub fn label(&self) -> &'static str {
        match self {
            Need::Parked => Standing::Parked.label(),
            Need::Quarantined => Standing::Quarantined.label(),
            Need::Stale { .. } => "stale",
        }
    }

    /// The line a row prints under its title: what to do, in a few
    /// words. A park's verdict already says why it waits, so it has none.
    pub fn prompt(&self) -> Option<String> {
        match self {
            Need::Parked => None,
            Need::Quarantined => Some("quarantined: conclude or inspect".to_string()),
            Need::Stale { silent } => Some(format!(
                "stale: no event since {silent}; retry, resume or conclude"
            )),
        }
    }

    /// The detail's lines for run `run_id`: why it needs the operator and
    /// the commands that answer it. `conclude` folds the journal it
    /// closes, so it is named for a quarantined run only where that
    /// journal folds; `export` writes it for reading wherever it does not.
    pub fn way_out(&self, run_id: &str) -> Vec<String> {
        let conclude = format!("brokkr conclude --run {run_id} --reason <why>");
        match self {
            Need::Parked => Vec::new(),
            Need::Quarantined => vec![
                format!("way out   {conclude}"),
                format!("          where its journal folds; or brokkr export --run {run_id}"),
                "          and inspect the journal it writes".to_string(),
            ],
            Need::Stale { silent } => vec![
                format!(
                    "stale     no event since {silent}, past the {}h bound: an attempt's {}h \
                     deadline and a {}h margin",
                    STALE_AFTER_SECONDS / 3600,
                    ATTEMPT_DEADLINE_SECONDS / 3600,
                    STALE_MARGIN_SECONDS / 3600,
                ),
                format!("way out   brokkr operator retry --run {run_id} --reason <why>,"),
                format!("          then brokkr resume --run {run_id};"),
                format!("          or {conclude}"),
            ],
        }
    }
}

/// What run `row` needs of the operator at `now`, if anything: a park,
/// a journal that does not fold, or a running journal silent for longer
/// than the bound — the longest attempt deadline a shipped seat holds
/// and a margin past it, against the time of the run's last event. A
/// run is stale only on that evidence: a clock or a last event whose
/// time does not read never makes one.
pub fn need(row: &RunRow, now: &str) -> Option<Need> {
    match row.verdict.standing {
        Standing::Quarantined => Some(Need::Quarantined),
        Standing::Parked => Some(Need::Parked),
        Standing::Running => silence(row, now).map(|silent| Need::Stale { silent }),
        Standing::Shipped | Standing::Stopped | Standing::OperatorStopped => None,
    }
}

/// How long the run's journal has been silent, rendered, when that is
/// past the staleness bound.
fn silence(row: &RunRow, now: &str) -> Option<String> {
    let last = row.last_recorded_at.as_deref()?;
    let silent = js::parse_millis(now)? - js::parse_millis(last)?;
    match silent > STALE_AFTER_SECONDS * 1000 {
        true => fmt_dur(last, now),
        false => None,
    }
}

// ----------------------------------------------------------- the title

/// The feature's first line that says anything, clamped at a word
/// boundary to [`TITLE_COLUMNS`] display columns with an ellipsis. A
/// line with no space to cut at is cut where it stops fitting. A
/// renderer that strips characters widens what it measures here (a
/// family emoji loses its joiners), so it clamps its sanitized title
/// again with this, and paints what it measured.
pub fn title(feature: &str) -> String {
    title_within(feature, TITLE_COLUMNS)
}

/// [`title`] clamped to `columns` instead (#503): a list wider than the
/// one beside the detail pane gives its titles the width it has.
pub fn title_within(feature: &str, columns: usize) -> String {
    let first = feature
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    if first.width() <= columns {
        return first.to_string();
    }
    let cut = break_at(first, columns.saturating_sub(1));
    format!("{}{ELLIPSIS}", first[..cut].trim_end())
}

/// `text` wrapped to `columns` display columns, a line per entry: each
/// source line broken at its last space that fits, a word wider than the
/// width cut where it stops fitting, and a blank line kept. A pane that
/// scrolls a wrapped text scrolls by these lines.
pub fn wrap(text: &str, columns: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for line in text.lines() {
        let mut rest = line.trim_end();
        // At least one character a line, so a width narrower than a wide
        // character still ends.
        while rest.width() > columns && rest.chars().nth(1).is_some() {
            let first = rest.char_indices().nth(1).map_or(rest.len(), |(at, _)| at);
            let cut = break_at(rest, columns).max(first);
            lines.push(rest[..cut].trim_end().to_string());
            rest = rest[cut..].trim_start();
        }
        lines.push(rest.to_string());
    }
    lines
}

/// Where `text` breaks to fit `columns`: at its last space within them
/// that follows a word, or where the next character stops fitting when
/// there is none.
fn break_at(text: &str, columns: usize) -> usize {
    let end = fitting(text, columns);
    match text[end..].starts_with(char::is_whitespace) {
        true => end,
        false => text[..end]
            .rfind(char::is_whitespace)
            .filter(|at| !text[..*at].trim().is_empty())
            .unwrap_or(end),
    }
}

/// `text` clamped to `columns` display columns, with an ellipsis when
/// anything was cut.
fn clamp_columns(text: &str, columns: usize) -> String {
    if text.width() <= columns {
        return text.to_string();
    }
    format!("{}{ELLIPSIS}", &text[..fitting(text, columns - 1)])
}

/// The byte length of the longest prefix of `text` within `columns`
/// display columns, on a char boundary. Chars are summed first, a
/// control char at the one column a string's measure gives it; then the
/// prefix steps back while the string's own measure is still wider, as
/// it is where a presentation selector widens the character before it.
fn fitting(text: &str, columns: usize) -> usize {
    let mut end = 0;
    let mut used = 0;
    for (index, character) in text.char_indices() {
        used += character.width().unwrap_or(1);
        if used > columns {
            break;
        }
        end = index + character.len_utf8();
    }
    while text[..end].width() > columns {
        end = text[..end]
            .char_indices()
            .next_back()
            .map_or(0, |(index, _)| index);
    }
    end
}

// --------------------------------------------------------- the verdict

fn verdict(state: Option<&RunState>, residuals: &[ResidualFinding]) -> Verdict {
    let residual = open_severity(residuals);
    let Some(state) = state else {
        return Verdict {
            standing: Standing::Quarantined,
            rule: None,
            residual,
            reason: None,
            text: "does not fold".to_string(),
        };
    };
    let standing = standing_of(state);
    let rule = match standing {
        Standing::Shipped | Standing::Stopped => last_rule(state),
        Standing::Parked if ruling_parked(state) => last_rule(state),
        Standing::Parked
        | Standing::Running
        | Standing::OperatorStopped
        | Standing::Quarantined => None,
    };
    let text = match standing {
        Standing::Running => String::new(),
        Standing::OperatorStopped => "by operator".to_string(),
        Standing::Shipped | Standing::Parked | Standing::Stopped | Standing::Quarantined => {
            rule.map_or(ABSENT.to_string(), |(rule, from)| abbreviate(rule, from))
        }
    };
    Verdict {
        standing,
        rule: rule.map(|(rule, _)| rule.to_string()),
        residual,
        reason: state.park_reason.clone(),
        text,
    }
}

fn standing_of(state: &RunState) -> Standing {
    match state.status {
        Status::Running => Standing::Running,
        Status::AwaitingOperator => Standing::Parked,
        Status::Completed => Standing::Shipped,
        Status::Stopped if state.phase.as_deref() == Some(STOP_PHASE) => Standing::Stopped,
        Status::Stopped => Standing::OperatorStopped,
    }
}

/// The last ruling's rule id and the phase that ruled, when it names a
/// rule at all.
fn last_rule(state: &RunState) -> Option<(&str, Option<&str>)> {
    let ruled = decided(state.last_decision.as_ref()?)?;
    Some((ruled.rule.as_str()?, ruled.from.and_then(Value::as_str)))
}

/// The last ruling routed nowhere, so it is what parked the run — the
/// fold's own test for a rule-driven park. A park on an exhausted
/// attempt limit or a lost attempt follows a ruling that routed on.
fn ruling_parked(state: &RunState) -> bool {
    state
        .last_decision
        .as_ref()
        .and_then(decided)
        .is_some_and(|ruled| ruled.next.and_then(Value::as_str).is_none())
}

/// A rule id without the prefix naming the phase that ruled it —
/// `REVIEW-SECURITY-HOLD` from `review` is `SECURITY-HOLD`, `IMPL-BLOCKED`
/// from `implement` is `BLOCKED` — clamped to [`VERDICT_COLUMNS`].
fn abbreviate(rule: &str, from: Option<&str>) -> String {
    let short = match (rule.split_once('-'), from) {
        (Some((head, tail)), Some(from)) if from.starts_with(&head.to_ascii_lowercase()) => tail,
        _ => rule,
    };
    clamp_columns(short, VERDICT_COLUMNS)
}

/// The highest severity any open residual finding names. Only the
/// severity input carries a [`Severity`]; the boolean flags carry none
/// and stay in the findings themselves, and `none` is no residual.
fn open_severity(residuals: &[ResidualFinding]) -> Option<Severity> {
    residuals
        .iter()
        .filter(|finding| finding.superseded.is_none())
        .filter_map(|finding| Severity::named(&finding.value))
        .filter(|severity| *severity > Severity::None)
        .max()
}

// ------------------------------------------------------------ run rows

fn run_row(entry: &RunEntry) -> RunRow {
    let status = entry
        .state
        .map(|state| status_str(&state.status).to_string());
    let status_known = match &status {
        Some(status) => KNOWN_STATUS.contains(&status.as_str()),
        None => false,
    };
    RunRow {
        run_id: entry.run_id.to_string(),
        status,
        status_known,
        phase: entry.state.and_then(|state| state.phase.clone()),
        seq: entry.state.map(|state| state.seq),
        created_at: entry.created_at.to_string(),
        last_recorded_at: entry.last_recorded_at.map(str::to_string),
        feature: entry.feature.to_string(),
        detail: entry.detail.map(str::to_string),
        residuals: entry.residuals.to_vec(),
        title: title(entry.feature),
        verdict: verdict(entry.state, entry.residuals),
        hire: entry.state.and_then(hire),
    }
}

/// The seat the fold's cursor has hired for the current effect.
fn hire(state: &RunState) -> Option<Hire> {
    match &state.cursor {
        Cursor::ExecuteEffect {
            seat,
            failed_attempts,
            ..
        }
        | Cursor::EffectInFlight {
            seat,
            failed_attempts,
            ..
        } => Some(Hire {
            seat: seat.clone(),
            attempt: failed_attempts + 1,
        }),
        Cursor::Start
        | Cursor::EnterPhase { .. }
        | Cursor::RequestEffect
        | Cursor::Decide { .. }
        | Cursor::Park { .. }
        | Cursor::Stop
        | Cursor::Idle => None,
    }
}

/// Run rows, newest first. Ordering is a derivation rule, not something
/// each surface reverses for itself.
pub fn run_rows(entries: &[RunEntry]) -> RunsView {
    let mut runs: Vec<RunRow> = entries.iter().map(run_row).collect();
    runs.reverse();
    let count = runs.len();
    RunsView {
        view_version: VIEW_VERSION,
        runs,
        count,
    }
}

/// One hearth as a fleet reader hands it over: the realm it belongs to,
/// the journal it was read from, and either that journal's entries or
/// the words of the refusal that stopped it being read.
pub struct HearthEntries<'a> {
    pub realm: &'a str,
    pub journal: &'a str,
    pub entries: &'a [RunEntry<'a>],
    pub detail: Option<&'a str>,
}

/// The world's fleet, grouped by realm. Each hearth's rows are derived
/// by exactly the same [`run_rows`] a one-journal world uses — the
/// grouping is an arrangement of that derivation, never a second one,
/// and no fold ever crosses a journal boundary (decision 0026 ruling 5).
pub fn fleet_rows(hearths: &[HearthEntries]) -> FleetView {
    let realms: Vec<RealmRuns> = hearths
        .iter()
        .map(|hearth| {
            let view = run_rows(hearth.entries);
            RealmRuns {
                realm: hearth.realm.to_string(),
                journal: hearth.journal.to_string(),
                runs: view.runs,
                count: view.count,
                detail: hearth.detail.map(str::to_string),
            }
        })
        .collect();
    let count = realms.iter().map(|realm| realm.count).sum();
    FleetView {
        view_version: VIEW_VERSION,
        realms,
        count,
    }
}

#[cfg(test)]
mod tests;
