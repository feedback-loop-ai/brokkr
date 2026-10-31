//! The fleet's rows (#491): a run as a listing shows it, with the three
//! facts the fleet is read by — what the run is called, how it stands
//! and was ruled, and which section it belongs to. Derived here once, so
//! the TUI, `brokkr runs` and the console can read them alike (decision
//! 0013); a renderer lays them out and derives none of them.

use brokkr_core::fold::{RunState, Status};
use brokkr_core::page::newest_first;
use brokkr_core::policy::Severity;
use brokkr_core::residual;
use brokkr_core::stamp::parse_millis;
use serde::Serialize;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    ruled, FleetView, RealmRuns, ResidualFinding, RunEntry, RunsView, ABSENT, KNOWN_STATUS,
    VIEW_VERSION,
};

#[derive(Serialize, Clone)]
pub struct RunRow {
    pub run_id: String,
    pub status: Option<String>,
    /// The status is one of the four the surfaces have a colour for.
    pub status_known: bool,
    /// The word a readout prints, derived once from the standing (0102).
    pub status_word: String,
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
    /// What refused the journal, when the row carries no status (#503):
    /// a way out names `export` only for a journal the store loads.
    pub quarantine: Option<Quarantine>,
    /// The operator's acknowledgement standing on a quarantined run
    /// (#511): it lists under "older" while the journal stays where it was
    /// acknowledged, keeping its `?` and its refusal.
    pub acknowledgement: Option<Acknowledgement>,
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
    /// Where the row lists within its section (decision 0089 ruling 6):
    /// the milliseconds of its last stamp, or of `created_at` while the
    /// run has no event, by the one parse, and `i64::MAX` where that does
    /// not read. Its section and its staleness read it too, so a page read
    /// from the summary's index and a whole read list a run alike.
    #[serde(skip)]
    pub order_at: i64,
    /// Where a page read the row, where its own facts place it elsewhere:
    /// a run read whole at its summary's place, or an id listed cut at its
    /// whole one (decision 0089 rulings 5 and 6). Cursors compare it, and
    /// a cut id's stays on the page: the key a surface selects it by.
    #[serde(skip)]
    pub paged_at: Option<(i64, String)>,
}

impl RunRow {
    /// The word a readout prints; the field carries [`Standing`]'s answer.
    pub fn status_word(&self) -> &str {
        &self.status_word
    }
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

/// What refused a quarantined run's journal (#503). The store verifies a
/// journal's chain and records as it loads it, and every verb that reads
/// a run loads it first, so a journal it refuses is refused by `export`
/// too; one that loads and does not fold still exports as written.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum Quarantine {
    /// The store refuses the journal. A quarantine that names no refusal
    /// is read as this one, the refusal no verb gets past, so no way out
    /// names a command the journal would refuse.
    #[default]
    DoesNotLoad,
    /// The journal loads, and the fold refuses it.
    DoesNotFold,
}

impl Quarantine {
    /// This refusal, in `words`, acknowledged by no one.
    pub fn in_words(self, words: &str) -> Refusal<'_> {
        Refusal {
            quarantine: self,
            words,
            acknowledgement: None,
        }
    }
}

/// Why a listed run carries no state: what refused its journal, and the
/// refusal's own words.
#[derive(Clone, Copy, Debug)]
pub struct Refusal<'a> {
    pub quarantine: Quarantine,
    pub words: &'a str,
    /// The operator's acknowledgement standing at the journal's last
    /// event, when one does (#511).
    pub acknowledgement: Option<&'a Acknowledgement>,
}

/// An operator's acknowledgement of a quarantined run (#511): who, when
/// and why, at the event that was the journal's last. The journal is not
/// touched; the store keeps it beside the chain, and it lapses when the
/// journal moves.
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Acknowledgement {
    pub seq: u64,
    pub event_hash: String,
    pub operator: String,
    pub reason: String,
    pub recorded_at: String,
}

impl Acknowledgement {
    /// The line every surface prints under the refusal.
    pub fn line(&self) -> String {
        format!(
            "acknowledged by {} at {}, event {}: {}",
            self.operator, self.recorded_at, self.seq, self.reason
        )
    }
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
pub(crate) const STOP_PHASE: &str = "stop";

const ELLIPSIS: char = '…';

/// How a run stands, as the fleet reads it. The fold's four statuses,
/// with a quarantined run beside them, an accepted stop that no ending
/// closed, and a stop split by who made it.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Standing {
    /// The journal does not fold; the row carries the fold's refusal.
    Quarantined,
    /// Waiting on the operator.
    Parked,
    Running,
    /// A running run whose operator stop was accepted and whose ending
    /// has not been written: stopping, not running.
    Stopping,
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
            Standing::Stopping => "stopping",
            Standing::Shipped => "shipped",
            Standing::Stopped | Standing::OperatorStopped => "stopped",
        }
    }

    /// The word a readout prints for a run whose fold status is `status`:
    /// the fold's own word, except that an accepted stop with no ending
    /// stands as stopping. One home for the rule every row and summary
    /// paints (decision 0071 ruling 5).
    pub fn status_word(self, status: Option<&str>) -> &str {
        match self {
            Standing::Stopping => self.label(),
            Standing::Quarantined
            | Standing::Parked
            | Standing::Running
            | Standing::Shipped
            | Standing::Stopped
            | Standing::OperatorStopped => status.unwrap_or("?"),
        }
    }
}

/// The detail a stopping run's row prints under its title: the accepted
/// stop's seq, and the one command that closes the run it no longer
/// drives. One function, so every surface says it alike (decision 0071
/// ruling 5).
pub fn stopping_detail(seq: u64) -> String {
    format!(
        "stop accepted at seq {seq}; no ending written. If no driver is running it, \
         `brokkr conclude` closes it."
    )
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
    /// Why the run stands as it does when the standing alone does not say
    /// it: a stopping run's accepted stop and its way out.
    pub detail: Option<String>,
}

/// Who must act on a run: the fleet's sections, listed in [`Section::ORDER`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    /// Parked, quarantined because the journal does not load or fold and
    /// not acknowledged where it stands, stopping because a stop was
    /// accepted and no ending was written, or stale: whatever [`need`]
    /// names.
    NeedsYou,
    Running,
    /// Finished within the last 24 hours, by when its journal last
    /// moved: a run that waited days on a ruling and ended today is
    /// today's news.
    Recent,
    /// Finished and older, and every quarantined run the operator has
    /// acknowledged (#511), however recent: it is answered until its
    /// journal moves.
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

/// The section `row` lists under at `now`, by its own facts: where every
/// read places a row, from a page or whole.
pub fn section_of(row: &RunRow, now: &str) -> Section {
    if need(row, now).is_some() {
        return Section::NeedsYou;
    }
    if row.verdict.standing == Standing::Running {
        return Section::Running;
    }
    if row.acknowledgement.is_some() {
        return Section::Older;
    }
    match parse_millis(now) {
        Some(now) if row.order_at <= now.saturating_sub(RECENT_MILLIS) => Section::Older,
        _ => Section::Recent,
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

/// What a verdict is derived from, whichever read holds the run: its fold
/// read whole, or its summary row, which states these as the fold does
/// (decision 0089 ruling 2), so the two derive one verdict.
#[derive(Clone, Copy)]
pub(crate) struct Facts<'a> {
    pub(crate) status: Status,
    pub(crate) phase: Option<&'a str>,
    /// The last ruling that names its rule, present or null: the rule id
    /// where present, the phase that ruled, and whether it routed on.
    pub(crate) ruling: Option<(Option<&'a str>, &'a str, bool)>,
    pub(crate) park_reason: Option<&'a str>,
    /// The seq an operator stop was accepted at, while no ending closed it.
    pub(crate) stop_accepted_seq: Option<u64>,
}

impl<'a> Facts<'a> {
    pub(crate) fn of(state: &'a RunState) -> Facts<'a> {
        Facts {
            status: state.run.status(),
            phase: state.phase.as_deref(),
            ruling: ruled(state.last_decision.as_ref()).map(|decision| {
                let routes = decision.next.present().is_some();
                let rule = decision.rule_id.present().map(String::as_str);
                (rule, decision.from.as_str(), routes)
            }),
            park_reason: state.run.park_reason().map(|reason| reason.as_str()),
            stop_accepted_seq: state.stopping_seq(),
        }
    }

    pub(crate) fn standing(self) -> Standing {
        match self.status {
            Status::Running if self.stop_accepted_seq.is_some() => Standing::Stopping,
            Status::Running => Standing::Running,
            Status::AwaitingOperator => Standing::Parked,
            Status::Completed => Standing::Shipped,
            Status::Stopped if self.phase == Some(STOP_PHASE) => Standing::Stopped,
            Status::Stopped => Standing::OperatorStopped,
        }
    }

    /// The last ruling's rule id and the phase that ruled, when it names a
    /// rule at all.
    fn last_rule(self) -> Option<(&'a str, Option<&'a str>)> {
        let (rule, from, _) = self.ruling?;
        Some((rule?, Some(from)))
    }

    /// The last ruling routed nowhere, so it is what parked the run — the
    /// fold's own test for a rule-driven park. A park on an exhausted
    /// attempt limit or a lost attempt follows a ruling that routed on.
    fn ruling_parked(self) -> bool {
        self.ruling.is_some_and(|(_, _, routes)| !routes)
    }
}

fn verdict(state: Option<&RunState>, residuals: &[ResidualFinding]) -> Verdict {
    verdict_of(state.map(Facts::of), open_severity(residuals))
}

/// The verdict `facts` and the highest open residual derive.
pub(crate) fn verdict_of(facts: Option<Facts<'_>>, residual: Option<Severity>) -> Verdict {
    let Some(facts) = facts else {
        return Verdict {
            standing: Standing::Quarantined,
            rule: None,
            residual,
            reason: None,
            text: "does not fold".to_string(),
            detail: None,
        };
    };
    let standing = facts.standing();
    let rule = match standing {
        Standing::Shipped | Standing::Stopped => facts.last_rule(),
        Standing::Parked if facts.ruling_parked() => facts.last_rule(),
        Standing::Parked
        | Standing::Running
        | Standing::Stopping
        | Standing::OperatorStopped
        | Standing::Quarantined => None,
    };
    let text = match standing {
        Standing::Running => String::new(),
        Standing::Stopping => String::new(),
        Standing::OperatorStopped => "by operator".to_string(),
        Standing::Shipped | Standing::Parked | Standing::Stopped | Standing::Quarantined => {
            rule.map_or(ABSENT.to_string(), |(rule, from)| abbreviate(rule, from))
        }
    };
    let detail = match standing {
        Standing::Stopping => facts.stop_accepted_seq.map(stopping_detail),
        _ => None,
    };
    Verdict {
        standing,
        rule: rule.map(|(rule, _)| rule.to_string()),
        residual,
        reason: facts.park_reason.map(str::to_string),
        text,
        detail,
    }
}

/// The last ruling's rule id and the phase that ruled, when it names a
/// rule at all.
pub(crate) fn last_rule(state: &RunState) -> Option<(&str, Option<&str>)> {
    Facts::of(state).last_rule()
}

/// Whether the last ruling, naming its rule, routed nowhere: what parked
/// the run ([`Facts::ruling_parked`]).
pub(crate) fn ruling_parked(state: &RunState) -> bool {
    Facts::of(state).ruling_parked()
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

/// The highest severity any open residual finding names, by core's one
/// reading ([`residual::open_severity`]). Only the severity input carries
/// a [`Severity`] by its name; the boolean flags carry none and stay in
/// the findings themselves.
fn open_severity(residuals: &[ResidualFinding]) -> Option<Severity> {
    residual::open_severity(residuals.iter().map(|finding| {
        (
            Severity::named(&finding.value),
            finding.superseded.is_some(),
        )
    }))
}

// ------------------------------------------------------------ run rows

/// The row of one run read whole.
pub fn run_row(entry: &RunEntry) -> RunRow {
    let status = entry
        .state
        .map(|state| state.run.status().as_str().to_string());
    let status_known = status
        .as_deref()
        .is_some_and(|word| KNOWN_STATUS.contains(&word));
    let verdict = verdict(entry.state, entry.residuals);
    let status_word = verdict.standing.status_word(status.as_deref()).to_string();
    RunRow {
        run_id: entry.run_id.to_string(),
        status,
        status_known,
        status_word,
        phase: entry.state.and_then(|state| state.phase.clone()),
        seq: entry.state.map(|state| state.seq),
        created_at: entry.created_at.to_string(),
        last_recorded_at: entry.last_recorded_at.map(str::to_string),
        feature: entry.feature.to_string(),
        detail: entry.detail.map(|refusal| refusal.words.to_string()),
        quarantine: entry.state.is_none().then(|| {
            entry
                .detail
                .map(|refusal| refusal.quarantine)
                .unwrap_or_default()
        }),
        residuals: entry.residuals.to_vec(),
        acknowledgement: entry
            .detail
            .and_then(|refusal| refusal.acknowledgement.cloned()),
        title: title(entry.feature),
        verdict,
        hire: entry.state.and_then(hire),
        order_at: order_at(entry.last_recorded_at, entry.created_at),
        paged_at: None,
    }
}

/// The seat the fold's cursor has hired for the current effect, by
/// core's one reading of it ([`brokkr_core::fold::Cursor::hire`]).
fn hire(state: &RunState) -> Option<Hire> {
    let (seat, failed_attempts) = state.run.cursor()?.hire()?;
    Some(Hire {
        seat: seat.to_string(),
        attempt: failed_attempts + 1,
    })
}

/// Run rows, by last activity, newest first, then run id (decision 0089
/// ruling 6): the order a page lists each section in, by the same
/// function, so a paged read and a whole read list a run alike. Ordering
/// is a derivation rule, not something each surface reverses for itself.
pub fn run_rows(entries: &[RunEntry]) -> RunsView {
    let mut runs: Vec<RunRow> = entries.iter().map(run_row).collect();
    runs.sort_by(|a, b| newest_first(a.place()).cmp(&newest_first(b.place())));
    RunsView::of(runs)
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

mod need;
mod page;
pub use need::{at_work, need, Need};
use page::order_at;
pub use page::{
    behind_line, page, ranges, section_of_rows, summary_row, window, Cursor, CursorError, Page,
    Paged, AHEAD_ROWS, PAGE_ROWS,
};

#[cfg(test)]
pub(crate) mod tests;
