//! How a row is painted by what it waits on (decision 0064 ruling 6):
//! its shade and its gutter mark, the group the console and the filters
//! read, the dim line under a terminal row, and what a listing counts as
//! it lists. A terminal row's tone and mark are its ending's, by
//! brokkr-core's total functions; a row with no ending is painted by what
//! it wants, as the addendum's choice 2 (a) rules. Colour never says
//! whether a run was good.

use std::collections::BTreeMap;

use brokkr_core::ending::{Citation, Ending, Tone, Wants};
use brokkr_view::{RunRow, Standing};

use super::{push_line, tint, Safe, Style, BOLD, DIM, GREEN, RED};

const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";

/// The six tone classes a row is painted in: an ending's five, and a
/// live run's bold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shade {
    Toned(Tone),
    Live,
}

/// Which of the console's three groups a row lists under, and what the
/// filters read: it wants the operator (`!` or `>`), it is live, or it is
/// settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Group {
    WantsYou,
    Live,
    Settled,
}

/// `row`'s shade and gutter mark: its ending's tone and mark when it has
/// ended; else yellow and `!` while it wants a ruling (a park), cyan and
/// `>` while it wants an action (a stopping run, an unanswered
/// quarantine), bold and no mark while it runs (a cool-down included),
/// and dim otherwise.
pub(crate) fn painted(row: &RunRow) -> (Shade, char) {
    painted_as(row.ending, row.wants, row.verdict.standing)
}

/// [`painted`], of a run that ended as `ending`, wants `wants` and stands
/// as `standing`: a row's facts, or a run summary's.
pub(crate) fn painted_as(
    ending: Option<Ending>,
    wants: Option<Wants>,
    standing: Standing,
) -> (Shade, char) {
    if let Some(ending) = ending {
        return (Shade::Toned(ending.tone()), ending.gutter());
    }
    match wants {
        Some(Wants::Ruling) => (Shade::Toned(Tone::Yellow), '!'),
        Some(Wants::Action) => (Shade::Toned(Tone::Cyan), '>'),
        None => match standing {
            Standing::Running => (Shade::Live, ' '),
            Standing::Quarantined
            | Standing::Parked
            | Standing::Stopping
            | Standing::Shipped
            | Standing::Stopped
            | Standing::OperatorStopped => (Shade::Toned(Tone::Dim), ' '),
        },
    }
}

/// The group `row` lists under: the `!` and `>` rows want the operator,
/// a bold row is live, and every other row is settled.
pub(crate) fn group(row: &RunRow) -> Group {
    match painted(row) {
        (_, '!' | '>') => Group::WantsYou,
        (Shade::Live, _) => Group::Live,
        (Shade::Toned(_), _) => Group::Settled,
    }
}

/// The ANSI code `shade` is written in.
pub(crate) fn code(shade: Shade) -> &'static str {
    match shade {
        Shade::Toned(Tone::Green) => GREEN,
        Shade::Toned(Tone::Yellow) => YELLOW,
        Shade::Toned(Tone::Cyan) => CYAN,
        Shade::Toned(Tone::Red) => RED,
        Shade::Toned(Tone::Dim) => DIM,
        Shade::Live => BOLD,
    }
}

/// The tone class `tone` is named by where a surface names it rather than
/// writes it: the console's `tone-<class>`.
pub(crate) fn tone_class(tone: Tone) -> &'static str {
    match tone {
        Tone::Green => "green",
        Tone::Yellow => "yellow",
        Tone::Cyan => "cyan",
        Tone::Red => "red",
        Tone::Dim => "dim",
    }
}

/// Every ending's word and the tone class brokkr-core's `Ending::tone`
/// gives it, as a JSON object: the table the console's run list paints an
/// ended run from, written into the page as `brokkr ui` serves it, so the
/// console restates no ending's tone (decision 0064 ruling 6).
pub(crate) fn ending_tones() -> String {
    let tones: serde_json::Map<String, serde_json::Value> = Ending::ALL
        .iter()
        .map(|ending| (ending.word().to_string(), tone_class(ending.tone()).into()))
        .collect();
    serde_json::Value::Object(tones).to_string()
}

/// What an ending cites, as a reader follows it: `by` the run, keyed by
/// its realm where one is named, or the ref it landed by.
pub(crate) fn cited(citation: &Citation) -> String {
    match citation {
        Citation::Run {
            realm: Some(realm),
            run_id,
        } => format!("by {realm}/{run_id}"),
        Citation::Run {
            realm: None,
            run_id,
        } => format!("by {run_id}"),
        Citation::Ref(reference) => format!("by {reference}"),
    }
}

/// The dim line under a terminal row: its ending's citation, or the rule
/// that ruled a policy ending.
pub(crate) fn ended_line(row: &RunRow, style: &Style) -> Option<String> {
    row.ending?;
    let said = match &row.ending_cited {
        Some(citation) => Some(cited(citation)),
        None => match row.verdict.standing {
            Standing::Stopped => row.verdict.rule.clone(),
            Standing::Quarantined
            | Standing::Parked
            | Standing::Running
            | Standing::Stopping
            | Standing::Shipped
            | Standing::OperatorStopped => None,
        },
    };
    said.map(|said| format!("      {}", tint(Safe::new(&said).as_str(), DIM, style)))
}

/// What a listing counts of the rows it lists: how many, how many want a
/// ruling and an action (every `refused` run among them, as choice 6 (a)
/// rules), and how many ended each way.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Tally {
    pub(crate) runs: usize,
    rulings: usize,
    actions: usize,
    endings: BTreeMap<Ending, usize>,
}

impl Tally {
    /// `row` counted.
    pub(crate) fn count(&mut self, row: &RunRow) {
        self.runs += 1;
        match row.wants {
            Some(Wants::Ruling) => self.rulings += 1,
            Some(Wants::Action) => self.actions += 1,
            None => {}
        }
        if let Some(ending) = row.ending {
            *self.endings.entry(ending).or_default() += 1;
        }
    }

    /// The header's counts: `N runs · N want a ruling · N wants an action`.
    pub(crate) fn counts(&self) -> String {
        let runs = match self.runs {
            1 => "1 run".to_string(),
            runs => format!("{runs} runs"),
        };
        format!(
            "{runs} · {} · {}",
            wanting(self.rulings, "a ruling"),
            wanting(self.actions, "an action")
        )
    }

    /// The closing line: how many listed runs ended each way, in ruling
    /// 2's order; nothing where none has.
    pub(crate) fn closing(&self) -> String {
        let counted: Vec<String> = Ending::ALL
            .iter()
            .filter_map(|ending| {
                let count = self.endings.get(ending)?;
                Some(format!("{count} {}", Safe::new(ending.word()).as_str()))
            })
            .collect();
        let mut out = String::new();
        if !counted.is_empty() {
            push_line(&mut out, &format!("endings  {}", counted.join(" · ")));
        }
        out
    }
}

/// `count` runs wanting `what`, the verb agreeing with the count.
fn wanting(count: usize, what: &str) -> String {
    match count {
        1 => format!("1 wants {what}"),
        count => format!("{count} want {what}"),
    }
}

#[cfg(test)]
pub(crate) mod tests;
