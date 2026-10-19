//! The fleet a page at a time (decision 0089 ruling 6): which ranges of
//! the summary's index each section reads, a summary row as the fleet
//! shows it, the cursor a page ends at, and the page itself, filled from
//! the sections in [`Section::ORDER`] by whichever read hands it their
//! rows. Derived here once, so the console, `brokkr runs` and the browser
//! page a fleet alike; the reads live with their stores.

use std::fmt;

use brokkr_core::page::{Bound, Copied, Lane, Place, Range, Row, Whole};
use brokkr_core::stamp::parse_millis;
use brokkr_core::summary::Bounded;
use serde::{Serialize, Serializer};

use super::{
    title, verdict_of, Acknowledgement, Facts, Hire, Quarantine, RunRow, Section, KNOWN_STATUS,
    RECENT_MILLIS, STALE_AFTER_SECONDS,
};
use crate::stated_finding;

/// The rows of a page a screen shows.
pub const PAGE_ROWS: usize = 50;

/// The visible page and the one ahead: what the console reads, and the
/// most runs behind their summary a refresh reads whole (ruling 6).
pub const AHEAD_ROWS: usize = 2 * PAGE_ROWS;

impl RunRow {
    /// The row's place in its section's order, by
    /// [`brokkr_core::page::newest_first`].
    pub fn place(&self) -> Place<'_> {
        (self.order_at, &self.run_id)
    }
}

/// The key a run read whole lists by, as the writer keeps it.
pub(super) fn order_at(last_recorded_at: Option<&str>, created_at: &str) -> i64 {
    parse_millis(last_recorded_at.unwrap_or(created_at)).unwrap_or(i64::MAX)
}

/// The ranges `section` reads at `now`, beside the runs behind their
/// summary, which a reader reads whole and places itself. Each is the
/// range [`super::sections`] places a row of that lane and time in, so a
/// page lists what the whole read lists (`every_page_is_its_sections`).
/// A `now` that does not read bounds nothing, as it stales and ages
/// nothing there.
pub fn ranges(section: Section, now: &str) -> Vec<Range> {
    let now = parse_millis(now);
    let stale = now.map(|now| now.saturating_sub(STALE_AFTER_SECONDS * 1000));
    let day = now.map(|now| now.saturating_sub(RECENT_MILLIS));
    let range = |lane, bound| Range { lane, bound };
    match section {
        Section::NeedsYou => {
            let mut held = vec![range(Lane::Refused, None), range(Lane::Parked, None)];
            held.extend(stale.map(|at| range(Lane::Running, Some(Bound::Below(at)))));
            held
        }
        Section::Running => vec![range(Lane::Running, stale.map(Bound::AtOrAbove))],
        Section::Recent => vec![range(Lane::Finished, day.map(Bound::Above))],
        Section::Older => {
            let older = day.map(|at| range(Lane::Finished, Some(Bound::AtOrBelow(at))));
            let mut held: Vec<Range> = older.into_iter().collect();
            held.push(range(Lane::Acknowledged, None));
            held
        }
    }
}

/// A bounded value as a row shows it: whole where it was read whole.
fn shown(value: &Bounded) -> &str {
    value.whole.as_deref().unwrap_or(&value.cut)
}

/// A run's row from its summary row (decision 0089 ruling 2), derived as
/// [`super::run_row`] derives one from the run read whole. `whole` is what
/// the row keeps beyond its bounds, read by its key: a listing that prints
/// a run's values whole reads it, and then the row is the whole read's.
/// A page reads none, so its row lists each value within its bound, the
/// feature as its first line and no finding.
pub fn summary_row(row: &Row, whole: Option<&Whole>) -> RunRow {
    let facts = row.status.map(|status| Facts {
        status,
        phase: row.phase.as_ref().map(shown),
        ruling: row.ruled.as_ref().map(|ruled| {
            let rule = ruled.rule_id.as_ref().map(shown);
            (rule, shown(&ruled.from), ruled.routes)
        }),
        park_reason: row.park_reason.as_ref().map(shown),
    });
    let feature = whole.map_or(row.feature_line.as_str(), |whole| &whole.feature);
    let copied = whole.map_or(row.acknowledged.as_ref(), |whole| {
        whole.acknowledged.as_ref()
    });
    let refused = row.refused.as_ref();
    let status = row.status.map(|status| status.as_str().to_string());
    RunRow {
        run_id: row.run_id.clone(),
        status_known: status
            .as_deref()
            .is_some_and(|status| KNOWN_STATUS.contains(&status)),
        status,
        phase: facts.and_then(|facts| facts.phase).map(str::to_string),
        seq: facts.map(|_| row.folded.0),
        created_at: shown(&row.created_at).to_string(),
        last_recorded_at: row
            .last_recorded_at
            .as_ref()
            .map(|at| shown(at).to_string()),
        feature: feature.to_string(),
        detail: refused.map(|refused| shown(&refused.words).to_string()),
        quarantine: refused.map(|_| Quarantine::DoesNotFold),
        acknowledgement: refused
            .and(copied)
            .map(|copied| acknowledgement(row, copied)),
        residuals: whole.map_or_else(Vec::new, |whole| {
            let stated = whole.residuals.iter().cloned();
            stated
                .map(|stated| stated_finding(&row.run_id, stated))
                .collect()
        }),
        title: title(feature),
        verdict: verdict_of(facts, row.residual_open),
        hire: row.hired.as_ref().map(|hired| Hire {
            seat: shown(&hired.seat).to_string(),
            attempt: hired.failed_attempts + 1,
        }),
        order_at: row.order_at,
    }
}

/// The acknowledgement standing at the row's head, in `copied`'s words.
fn acknowledgement(row: &Row, copied: &Copied) -> Acknowledgement {
    Acknowledgement {
        seq: row.folded.0,
        event_hash: row.folded.1.clone(),
        operator: copied.operator.clone(),
        reason: copied.reason.clone(),
        recorded_at: copied.at.clone(),
    }
}

/// Where a page ended: the section its last row listed under, and that
/// row's place in the section's order. A reader asks for the page after
/// it, typed from its text at the edge (`--after`, `?after=`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub section: Section,
    pub order_at: i64,
    pub run_id: String,
}

/// The word a cursor names each section by.
fn cursor_word(section: Section) -> &'static str {
    match section {
        Section::NeedsYou => "needs-you",
        Section::Running => "running",
        Section::Recent => "recent",
        Section::Older => "older",
    }
}

/// Why a cursor's text names no place on a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorError {
    /// Not `<section>:<order>:<run id>`.
    Shape(String),
    /// A section no page lists.
    Section(String),
    /// An order that is not a whole number of milliseconds.
    Order(String),
}

impl fmt::Display for CursorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&match self {
            CursorError::Shape(text) => format!(
                "cursor '{text}' is not <section>:<order>:<run id>, as a page's `next` states it"
            ),
            CursorError::Section(word) => {
                format!("cursor section '{word}' is none of needs-you, running, recent or older")
            }
            CursorError::Order(word) => {
                format!("cursor order '{word}' is not a whole number of milliseconds")
            }
        })
    }
}

impl std::str::FromStr for Cursor {
    type Err = CursorError;

    fn from_str(text: &str) -> Result<Cursor, CursorError> {
        let mut parts = text.splitn(3, ':');
        let (Some(word), Some(order), Some(run_id)) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(CursorError::Shape(text.to_string()));
        };
        if run_id.is_empty() {
            return Err(CursorError::Shape(text.to_string()));
        }
        let section = Section::ORDER
            .into_iter()
            .find(|section| cursor_word(*section) == word)
            .ok_or_else(|| CursorError::Section(word.to_string()))?;
        let order_at = order
            .parse()
            .map_err(|_| CursorError::Order(order.to_string()))?;
        Ok(Cursor {
            section,
            order_at,
            run_id: run_id.to_string(),
        })
    }
}

impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = cursor_word(self.section);
        f.write_str(&format!("{word}:{}:{}", self.order_at, self.run_id))
    }
}

impl Serialize for Cursor {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// What a listing of one page states beside its rows: where the next
/// page starts, or `None` at the last.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Paged {
    pub next: Option<Cursor>,
}

/// One page of the fleet: its rows in section order, each with the
/// section it lists under, and where the next page starts.
pub struct Page {
    pub rows: Vec<(Section, RunRow)>,
    pub next: Option<Cursor>,
}

/// The page after `after`, or the first: up to `limit` rows (at least
/// one), the sections in [`Section::ORDER`] from the cursor's on, each
/// handed by `read` its rows below the cursor's place in its own order,
/// at most as many as asked. One row past the page is asked for, so a
/// page that ends where the fleet does states no next page.
pub fn page<E>(
    after: Option<&Cursor>,
    limit: usize,
    mut read: impl FnMut(Section, Option<Place<'_>>, usize) -> Result<Vec<RunRow>, E>,
) -> Result<Page, E> {
    let limit = limit.max(1);
    let first = after.map_or(0, |after| {
        let at = Section::ORDER
            .iter()
            .position(|section| *section == after.section);
        at.expect("every section is in the order")
    });
    let mut rows: Vec<(Section, RunRow)> = Vec::new();
    for section in &Section::ORDER[first..] {
        let wanted = limit.saturating_add(1) - rows.len();
        if wanted == 0 {
            break;
        }
        let place = after
            .filter(|after| after.section == *section)
            .map(|after| (after.order_at, after.run_id.as_str()));
        let read = read(*section, place, wanted)?;
        rows.extend(read.into_iter().take(wanted).map(|row| (*section, row)));
    }
    let more = rows.len() > limit;
    rows.truncate(limit);
    let next = rows.last().filter(|_| more).map(|(section, row)| Cursor {
        section: *section,
        order_at: row.order_at,
        run_id: row.run_id.clone(),
    });
    Ok(Page { rows, next })
}

/// Up to `wanted` of `rows`, listed whole and in their order, that list
/// under `section` at `now` and below `after`: a section of a fleet read
/// whole, for a reader of a journal no summary pages (ruling 8).
pub fn section_of_rows(
    rows: &[RunRow],
    section: Section,
    now: &str,
    after: Option<Place<'_>>,
    wanted: usize,
) -> Vec<RunRow> {
    rows.iter()
        .filter(|row| after.is_none_or(|after| row.place() < after))
        .filter(|row| super::section_of(row, now) == section)
        .take(wanted)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests;
