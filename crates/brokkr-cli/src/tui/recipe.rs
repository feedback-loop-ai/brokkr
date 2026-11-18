//! A recipe's delivery graph in the run rail's grammar (#1145). `brokkr
//! graph` lays the recipe's states on the rail [`plan`] lays a run's phases
//! on, paints it with the one painter, and writes the traffic the view
//! counted over those cells: a recipe and a run read alike, drawn by one
//! renderer (decision 0071 ruling 5). Every figure is the view's own
//! (ruling 7); this file places them and counts nothing.
//!
//! An arrow is a link the range took: two neighbours no taken link joins
//! stand apart on the rail with no road between them. What the rail
//! cannot draw is listed under it, never dropped: a rule taken between
//! states that are not neighbours on one rail, a return arc that would
//! overlap another, a rule into a terminal beneath the row, and a park.

use std::collections::BTreeSet;
use std::fmt;

use brokkr_view::{GraphView, ModelAtBoundary, Road};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::*;

/// The columns the drawing takes when stdout is not a terminal: plain
/// text that pipes and snapshots alike on every host.
pub(crate) const PIPED_WIDTH: usize = 100;

/// The narrowest terminal the drawing fits: one that reports fewer
/// columns (a pty with no size set answers 0) is drawn at [`PIPED_WIDTH`]
/// as a pipe is.
pub(crate) const NARROWEST: usize = 40;

/// The rows one rail is planned in: the counts' row, the rail, the names
/// and the return arcs' row, with the selection box's row the plan
/// reserves left blank (no state is selected here) and dropped.
const RAIL_ROWS: usize = 5;

/// The drawing's legend, so a bare number on the rail says what it counts;
/// each line fits 80 columns.
const LEGEND: [&str; 3] = [
    "nodes: runs entered · median stay · ∙ a live run · · never entered",
    "arrows: runs moved · ↻ to itself · ×n back · none where no run moved",
    "⊗ a terminal: those beneath the row are drawn under it, their rules listed",
];

/// Where the view places a state (design D3), read at the drawing's edge
/// from the view's wire word into this file's own type: the view's
/// `Place` is not named outside `brokkr-view`, and naming it grows that
/// crate's public API, which is the operator's to rule.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
enum Placed {
    Row,
    End,
    Beneath,
}

/// How a link moves the run (design D3), read as [`Placed`] is, from the
/// view's `RuleKind`.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
enum Moved {
    Forward,
    #[serde(rename = "self")]
    SelfLoop,
    Back,
    Terminal,
}

/// Why the view could not be drawn (decision 0071 ruling 8).
#[derive(Debug)]
pub(crate) enum DrawError {
    /// The view carries a word this file has not learned: a place or a
    /// kind the view grew.
    Unread {
        what: &'static str,
        source: serde_json::Error,
    },
}

impl fmt::Display for DrawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DrawError::Unread { what, source } => {
                write!(
                    f,
                    "the drawing cannot read the graph view's {what}: {source}"
                )
            }
        }
    }
}

impl std::error::Error for DrawError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DrawError::Unread { source, .. } => Some(source),
        }
    }
}

/// `value` as the view's wire writes it, read into the drawing's `U`.
fn read_as<T: Serialize, U: DeserializeOwned>(
    what: &'static str,
    value: T,
) -> Result<U, DrawError> {
    serde_json::to_value(value)
        .and_then(serde_json::from_value)
        .map_err(|source| DrawError::Unread { what, source })
}

/// A state's node carries no served model: the rail's node shape asks for
/// one, and the recipe's graph has none to show.
fn unserved() -> ModelAtBoundary {
    let empty = || brokkr_view::Cell {
        text: String::new(),
        absent: true,
        note: None,
    };
    ModelAtBoundary {
        model: empty(),
        boundary: empty(),
    }
}

/// A return arc one rail draws: its landing and departure as positions on
/// the rail, and the link it carries.
#[derive(Clone, Copy)]
struct Return {
    to: usize,
    from: usize,
    link: usize,
}

/// The view, its places and kinds read once, at `width` columns.
struct Drawing<'a> {
    view: &'a GraphView,
    places: Vec<Placed>,
    kinds: Vec<Moved>,
    width: usize,
}

impl<'a> Drawing<'a> {
    fn new(view: &'a GraphView, width: usize) -> Result<Self, DrawError> {
        let places = (view.states.iter())
            .map(|state| read_as("place", state.place))
            .collect::<Result<_, _>>()?;
        let kinds = (view.links.iter())
            .map(|link| read_as("rule kind", link.kind))
            .collect::<Result<_, _>>()?;
        Ok(Drawing {
            view,
            places,
            kinds,
            width,
        })
    }

    /// The states the view places at `place`, in its order.
    fn placed(&self, place: Placed) -> Vec<usize> {
        (0..self.places.len())
            .filter(|at| self.places[*at] == place)
            .collect()
    }

    /// The link that took runs from state `from` to state `next`, the view
    /// holding at most one per pair.
    fn taken(&self, from: usize, next: usize) -> Option<usize> {
        let (from, next) = (&self.view.states[from].name, &self.view.states[next].name);
        (0..self.view.links.len()).find(|at| {
            let link = &self.view.links[*at];
            &link.from == from && &link.next == next && link.count > 0
        })
    }

    /// One state as a rail phase: its node labelled with its entered count
    /// and median, a terminal's name marked `⊗`, lit as live when a run
    /// stands in it, and `returns` as the roads its return arcs draw.
    fn phase_of(&self, at: usize, returns: Vec<Road>) -> Phase {
        let state = &self.view.states[at];
        let live =
            (self.view.live.iter()).any(|run| run.state.as_deref() == Some(state.name.as_str()));
        let (word, class) = match (live, state.entered > 0) {
            (true, _) => ("active", "in-active"),
            (false, true) => ("finished", "on-phosphor"),
            (false, false) => ("", ""),
        };
        let name = match self.places[at] {
            Placed::Row => state.name.clone(),
            Placed::End | Placed::Beneath => format!("⊗ {}", state.name),
        };
        Phase {
            name,
            strategy: None,
            visits: 0,
            current: false,
            plain: false,
            columns: vec![Column {
                label: Some(format!("{} · {}", state.entered, state.median.text)),
                nodes: vec![Node {
                    label: String::new(),
                    key: state.name.clone(),
                    state: word.to_string(),
                    state_class: class.to_string(),
                    served: unserved(),
                }],
            }],
            returns,
        }
    }

    /// The return arcs `rail` draws: the back links taken between two of
    /// its states, in the view's order, each kept only when its span
    /// shares no column with one kept before it. The arcs ride one row,
    /// so an overlapping one would merge into its neighbour and its `×n`
    /// fall on the other's; it is listed instead.
    fn returns(&self, rail: &[usize]) -> Vec<Return> {
        let position = |name: &str| (rail.iter()).position(|at| self.view.states[*at].name == name);
        let mut kept: Vec<Return> = Vec::new();
        for (link, kind) in self.kinds.iter().enumerate() {
            let road = &self.view.links[link];
            if *kind != Moved::Back || road.count == 0 {
                continue;
            }
            let (Some(to), Some(from)) = (position(&road.next), position(&road.from)) else {
                continue;
            };
            let apart = |other: &Return| from < other.to || to > other.from;
            if to < from && kept.iter().all(apart) {
                kept.push(Return { to, from, link });
            }
        }
        kept
    }

    /// `rail`'s states as phases, each carrying the arcs that land on it.
    fn phases(&self, rail: &[usize], arcs: &[Return]) -> Vec<Phase> {
        (rail.iter().enumerate())
            .map(|(position, at)| {
                let roads = (arcs.iter())
                    .filter(|arc| arc.to == position)
                    .map(|arc| Road {
                        from: self.view.states[rail[arc.from]].name.clone(),
                        taken: self.view.links[arc.link].count as u64,
                        rules: Vec::new(),
                    });
                self.phase_of(*at, roads.collect())
            })
            .collect()
    }

    /// The roads between neighbours: a taken link's runs above its arrow,
    /// and no road at all where none was taken. Each pair drawn goes into
    /// `drawn`.
    fn roads(&self, cells: &mut Cells, plan: &Plan, rail: &[usize], drawn: &mut Drawn) {
        let above = plan.rail_row - 1;
        for (pair, segs) in rail.windows(2).zip(plan.segments.windows(2)) {
            let Some(link) = self.taken(pair[0], pair[1]) else {
                for x in segs[0].rail.1 + 1..segs[1].rail.0 {
                    put(cells, x, plan.rail_row, " ", plain());
                }
                continue;
            };
            let text = self.view.links[link].count.to_string();
            let head = segs[1].rail.0 - 1;
            let x = head.saturating_sub(width_of(&text));
            put(cells, x, above, &text, plain());
            drawn.insert(link);
        }
        for (at, seg) in rail.iter().zip(&plan.segments) {
            if let Some(link) = self.taken(*at, *at) {
                let text = format!("↻{}", self.view.links[link].count);
                put(cells, seg.rail.0, above, &text, plain());
                drawn.insert(link);
            }
        }
    }

    /// Each kept arc's `×n`, centred on it. Two states' nodes stand at
    /// least a labelled node and a connector apart, so an arc is always
    /// wider than its ` ×n `, and the kept arcs share no column.
    fn arc_counts(&self, cells: &mut Cells, plan: &Plan, arcs: &[Return], drawn: &mut Drawn) {
        let Some(row) = plan.arc_row else {
            return;
        };
        for arc in arcs {
            let text = format!(" ×{} ", self.view.links[arc.link].count);
            let to = centre_of(&plan.segments[arc.to]);
            let from = centre_of(&plan.segments[arc.from]);
            put(
                cells,
                (to + from) / 2 - width_of(&text) / 2,
                row,
                &text,
                plain(),
            );
            drawn.insert(arc.link);
        }
    }

    /// One rail of the fold, painted and counted, as plain text: no colour,
    /// trailing blanks trimmed and blank rows dropped. A rail the fold
    /// continues wears the TUI's own elision marks, `›` and `‹`.
    fn rail_lines(&self, rail: &[usize], folded: (bool, bool), drawn: &mut Drawn) -> Vec<String> {
        let arcs = self.returns(rail);
        let plan = planned(&self.phases(rail, &arcs), self.width);
        let mut cells = paint_cells(&plan, 0, false);
        self.roads(&mut cells, &plan, rail, drawn);
        self.arc_counts(&mut cells, &plan, &arcs, drawn);
        if folded.0 {
            put(&mut cells, 0, plan.rail_row, "‹", plain());
        }
        if folded.1 {
            let x = self.width.saturating_sub(1);
            put(&mut cells, x, plan.rail_row, "›", plain());
        }
        (cells.iter())
            .map(|row| {
                let text: String = (row.iter().flatten())
                    .map(|(glyph, _)| glyph.as_str())
                    .collect();
                text.trim_end().to_string()
            })
            .filter(|text| !text.is_empty())
            .collect()
    }

    /// `states` folded into rails at the drawing's width, each painted.
    fn folded(&self, states: &[usize], drawn: &mut Drawn) -> Vec<String> {
        let rails = folds(&self.phases(states, &[]), self.width);
        let mut lines = Vec::new();
        for (index, rail) in rails.iter().enumerate() {
            let folded = (index > 0, index + 1 < rails.len());
            lines.extend(self.rail_lines(&states[rail.clone()], folded, drawn));
        }
        lines
    }

    /// A rule taken that no rail drew, in the rail's own glyphs: a back
    /// rule as its arc's head and `×n`, any other to its next state, a
    /// terminal marked `⊗`.
    fn listed(&self, at: usize) -> String {
        let link = &self.view.links[at];
        match self.kinds[at] {
            Moved::Back => format!("  {} ᐸ── {}  ×{}", link.next, link.from, link.count),
            Moved::Terminal => format!("  {} ──ᐳ ⊗ {}  {}", link.from, link.next, link.count),
            Moved::Forward | Moved::SelfLoop => {
                format!("  {} ──ᐳ {}  {}", link.from, link.next, link.count)
            }
        }
    }

    /// The lines under the rails: the taken links no rail drew, the parks,
    /// the rules that never fired, and the legend.
    fn under(&self, drawn: &Drawn) -> Vec<String> {
        let view = self.view;
        let unseen =
            (0..view.links.len()).filter(|at| view.links[*at].count > 0 && !drawn.contains(at));
        let mut lines: Vec<String> = unseen.map(|at| self.listed(at)).collect();
        let parks = (view.rules.iter()).filter(|rule| rule.next.is_none() && rule.count > 0);
        lines.extend(
            parks.map(|rule| format!("  {} ⊙ park  {} · {}", rule.from, rule.count, rule.id)),
        );
        let dormant = (view.rules.iter()).filter(|rule| rule.count == 0);
        let dormant: Vec<&str> = dormant.map(|rule| rule.id.as_str()).collect();
        lines.push(format!(
            "never fired {}  {}",
            view.never_fired.text,
            dormant.join("  ")
        ));
        lines.extend(LEGEND.map(str::to_string));
        lines
    }
}

/// The links a rail drew, by their index in the view.
type Drawn = BTreeSet<usize>;

fn planned(phases: &[Phase], width: usize) -> Plan {
    plan(phases, None, "running", None, None, width, RAIL_ROWS)
}

/// The fold: each rail takes as many states as fit `width` whole, from
/// the first, and a state that does not is the next rail's first. A rail
/// never scrolls a state away as the TUI's window does, so every state is
/// drawn; one state too wide alone clamps its name with `…`.
fn folds(phases: &[Phase], width: usize) -> Vec<std::ops::Range<usize>> {
    let mut rails = Vec::new();
    let mut start = 0;
    while start < phases.len() {
        let mut end = start + 1;
        while end < phases.len() {
            // No state is current, so the plan's window anchors on the
            // rail's first and can only elide at the right: a left elision
            // would drop a state, which the 80-column test's every-name
            // check (`at_80_columns_a_wide_recipe_folds_whole_states…`)
            // catches.
            if planned(&phases[start..=end], width).right_elided {
                break;
            }
            end += 1;
        }
        rails.push(start..end);
        start = end;
    }
    rails
}

/// `body`'s words, each with the blanks before it.
fn spaced(body: &str) -> Vec<(&str, &str)> {
    let mut words = Vec::new();
    let mut rest = body;
    loop {
        let after = rest.trim_start_matches(' ');
        let end = after.find(' ').unwrap_or(after.len());
        if end == 0 {
            return words;
        }
        words.push((&rest[..rest.len() - after.len()], &after[..end]));
        rest = &after[end..];
    }
}

/// `word` appended to `line`, broken where it passes `width`: the piece
/// that fits ends `…`, and the rest carries on a new line from `…`, so a
/// word is never cut, only continued.
fn broken(line: &mut String, lines: &mut Vec<String>, word: &str, width: usize, carried: &str) {
    let mut rest = word;
    while !rest.is_empty() && width_of(line) + width_of(rest) > width {
        let room = width.saturating_sub(width_of(line) + 1);
        let mut cut = rest.chars().next().map_or(0, char::len_utf8);
        for (at, character) in rest.char_indices().skip(1) {
            if width_of(&rest[..at + character.len_utf8()]) > room {
                break;
            }
            cut = at + character.len_utf8();
        }
        line.push_str(&rest[..cut]);
        line.push('…');
        lines.push(std::mem::replace(line, format!("{carried}…")));
        rest = &rest[cut..];
    }
    line.push_str(rest);
}

/// The rule every line under the rails is held to: `line` broken before
/// a word that would pass `width`, each further line indented three
/// columns past the first, and a word wider than a line continued with
/// `…` (see [`broken`]).
fn fitted(line: &str, width: usize) -> Vec<String> {
    let body = line.trim_start_matches(' ');
    let indent = &line[..line.len() - body.len()];
    let carried = format!("{indent}   ");
    let mut lines = Vec::new();
    let mut current = indent.to_string();
    let mut fresh = true;
    for (gap, word) in spaced(body) {
        if !fresh && width_of(&current) + width_of(gap) + width_of(word) > width {
            lines.push(std::mem::replace(&mut current, carried.clone()));
            fresh = true;
        }
        if !fresh {
            current.push_str(gap);
        }
        broken(&mut current, &mut lines, word, width, &carried);
        fresh = false;
    }
    lines.push(current);
    lines
}

/// `brokkr graph`'s drawing of `view` at `width` columns: the row's rails,
/// the terminals beneath it, what they could not draw, the rules that
/// never fired, and the legend, then a blank line. Nothing at all when the
/// view has no state to draw: a recipe that does not compile and that no
/// run carries.
pub(crate) fn recipe_graph(view: &GraphView, width: usize) -> Result<String, DrawError> {
    if view.states.is_empty() {
        return Ok(String::new());
    }
    let drawing = Drawing::new(view, width)?;
    let mut row = drawing.placed(Placed::Row);
    row.extend(drawing.placed(Placed::End));
    let mut drawn = Drawn::new();
    let mut lines = drawing.folded(&row, &mut drawn);
    lines.extend(drawing.folded(&drawing.placed(Placed::Beneath), &mut drawn));
    let under = drawing.under(&drawn);
    lines.extend(under.iter().flat_map(|line| fitted(line, width)));
    Ok(lines
        .iter()
        .map(|line| format!("{line}\n"))
        .collect::<String>()
        + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_the_drawing_has_not_learned_is_refused_with_what_it_read() {
        let read: Result<Placed, DrawError> = read_as("place", "sideways");
        let error = read.unwrap_err();
        assert!(matches!(error, DrawError::Unread { what: "place", .. }));
        assert_eq!(
            error.to_string(),
            "the drawing cannot read the graph view's place: unknown variant `sideways`, \
             expected one of `row`, `end`, `beneath`"
        );
        assert!(std::error::Error::source(&error).is_some());
        assert_eq!(
            read_as::<_, Moved>("rule kind", "self").unwrap(),
            Moved::SelfLoop
        );
    }

    #[test]
    fn a_line_breaks_before_a_word_and_a_word_wider_than_a_line_continues() {
        assert_eq!(
            fitted("  alpha beta  gamma", 12),
            ["  alpha beta", "     gamma"]
        );
        assert_eq!(
            fitted("  abcdefghijklmnop", 10),
            ["  abcdefg…", "     …hij…", "     …klm…", "     …nop"]
        );
        assert_eq!(fitted("", 10), [""]);
    }
}
