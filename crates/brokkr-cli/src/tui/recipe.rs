//! A recipe's delivery graph in the run rail's grammar (#1145). `brokkr
//! graph` lays the recipe's states on the rail [`plan`] lays a run's phases
//! on, deals its roads back onto arc rows with the one [`nest`], paints it
//! with the one painter, and writes the traffic the view counted over
//! those cells: a recipe and a run read alike, drawn by one renderer
//! (decision 0071 ruling 5). Every figure is the view's own (ruling 7);
//! this file places them and counts nothing.
//!
//! An arrow is a link the range took: two neighbours no taken link joins
//! stand apart on the rail with no road between them. Every taken back
//! link is a return arc, nested under another where their spans meet, and
//! cut at the fold's marks `›` and `‹` where a fold put its states on two
//! rails. A taken rule into a terminal beneath the row is a road down
//! from the state it leaves. No count is written over or against another.
//! What the rail still cannot draw is listed under it, never dropped: a
//! rule taken between states that are not neighbours, a count the width
//! leaves no room for, and a park.

use std::collections::BTreeSet;
use std::ops::Range;

use brokkr_view::{GraphView, ModelAtBoundary, Place, RuleKind};

use super::*;

/// The columns the drawing takes when stdout is not a terminal: plain
/// text that pipes and snapshots alike on every host.
pub(crate) const PIPED_WIDTH: usize = 100;

/// The narrowest terminal the drawing fits: one that reports fewer
/// columns (a pty with no size set answers 0) is drawn at [`PIPED_WIDTH`]
/// as a pipe is.
pub(crate) const NARROWEST: usize = 40;

/// The rows one rail is planned in: two for the counts over the rail, the
/// rail, the names, and the selection box's row the plan reserves (no
/// state is selected here, so it stays blank and is dropped). [`nest`]
/// adds the arc rows under them.
const RAIL_ROWS: usize = 5;

/// The drawing's legend, so a bare number on the rail says what it counts;
/// each line fits 80 columns, and a narrower drawing wraps it.
const LEGEND: [&str; 4] = [
    "nodes: runs entered · median stay · ∙ a live run · · never entered",
    "arrows: runs moved · ↻ to itself · ×n back · none where no run moved",
    "⊗ a terminal · ╰─ n ─ᐳ ⊗ the road down to one beneath the row",
    "› ‹ a rail the next continues, with its arrows and arcs",
];

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

/// States of the view in their order, and the fold that puts them on
/// rails.
struct Folded<'r> {
    states: &'r [usize],
    rails: Vec<Range<usize>>,
}

impl Folded<'_> {
    /// The states on rail `index`.
    fn rail(&self, index: usize) -> &[usize] {
        &self.states[self.rails[index].clone()]
    }

    /// The rail the state named `name` stands on, and its place on that
    /// rail, when it is one of these states.
    fn spot(&self, view: &GraphView, name: &str) -> Option<(usize, usize)> {
        let at = (self.states.iter()).position(|at| view.states[*at].name == name);
        at.map(|at| {
            let rail = (self.rails.iter())
                .take_while(|rail| rail.end <= at)
                .count();
            (rail, at - self.rails[rail].start)
        })
    }
}

/// What an arc a rail draws carries, written over it once it is painted.
enum Carried {
    /// A back link's `×n`.
    Back(usize),
    /// A road down's runs, and the terminal it reaches, written from its
    /// head.
    Down {
        link: usize,
        head: usize,
        tag: String,
    },
}

/// The counts written over the rows above the rail, each kept a blank
/// column from every other on its row, so no count is written over or
/// against another (review 2's R2).
#[derive(Default)]
struct Labels {
    /// `(row, first column, one past the last)`.
    taken: Vec<(usize, usize, usize)>,
}

impl Labels {
    /// `text` written at `x` on the first of `rows` where it stands clear,
    /// when it ends by `limit`; false, with nothing written, when it does
    /// not or no row has room.
    fn write(
        &mut self,
        cells: &mut Cells,
        rows: &[usize],
        (x, limit): (usize, usize),
        text: &str,
    ) -> bool {
        let last = x + width_of(text);
        let clear = |row: &usize| {
            (self.taken.iter()).all(|(other, first, end)| other != row || last < *first || *end < x)
        };
        let Some(row) = rows.iter().copied().find(clear).filter(|_| last <= limit) else {
            return false;
        };
        put(cells, x, row, text, plain());
        self.taken.push((row, x, last));
        true
    }
}

/// The view, drawn at `width` columns.
struct Drawing<'a> {
    view: &'a GraphView,
    width: usize,
}

impl Drawing<'_> {
    /// The states the view places at `place`, in its order.
    fn placed(&self, place: Place) -> Vec<usize> {
        (0..self.view.states.len())
            .filter(|at| self.view.states[*at].place == place)
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
    /// and median, a terminal's name marked `⊗`, and lit as live when a run
    /// stands in it.
    fn phase_of(&self, at: usize) -> Phase {
        let state = &self.view.states[at];
        let live =
            (self.view.live.iter()).any(|run| run.state.as_deref() == Some(state.name.as_str()));
        let (word, class) = match (live, state.entered > 0) {
            (true, _) => ("active", "in-active"),
            (false, true) => ("finished", "on-phosphor"),
            (false, false) => ("", ""),
        };
        let name = match state.place {
            Place::Row => state.name.clone(),
            Place::End | Place::Beneath => format!("⊗ {}", state.name),
        };
        let label = format!("{} · {}", state.entered, state.median.text);
        Phase {
            name,
            strategy: None,
            visits: 0,
            current: false,
            plain: false,
            columns: vec![Column {
                label: Some(label),
                nodes: vec![Node {
                    label: String::new(),
                    key: state.name.clone(),
                    state: word.to_string(),
                    state_class: class.to_string(),
                    served: unserved(),
                }],
            }],
            returns: Vec::new(),
        }
    }

    /// The arcs rail `index` of `row` draws, each with what it carries:
    /// the taken back links ([`Drawing::back`]) and the roads down
    /// ([`Drawing::down`]). The links landing furthest right come first,
    /// so of two arcs of one width (two halves leaving one state for an
    /// earlier rail) [`nest`] deals the shallower row to the one whose
    /// landing half rides shallower too.
    fn arcs(&self, row: &Folded, index: usize, plan: &Plan) -> Vec<(Arc, Carried)> {
        let mut links: Vec<usize> = (0..self.view.links.len()).collect();
        links.sort_by_key(|at| std::cmp::Reverse(row.spot(self.view, &self.view.links[*at].next)));
        let mut arcs = Vec::new();
        for link in links {
            let road = &self.view.links[link];
            let arc = match (road.kind, road.count > 0) {
                (RuleKind::Back, true) => self.back(row, index, link, plan),
                (RuleKind::Terminal, true) => self.down(row, index, link, plan),
                (RuleKind::Forward | RuleKind::SelfLoop, _)
                | (RuleKind::Back | RuleKind::Terminal, false) => None,
            };
            arcs.extend(arc);
        }
        arcs
    }

    /// A taken back link as rail `index` draws it: whole when both its
    /// states stand on this rail, its landing half (out to `›`) or its
    /// departing half (in from `‹`) when the fold put the other on
    /// another rail, and nothing when neither is here or it does not run
    /// leftward along the row.
    fn back(&self, row: &Folded, index: usize, link: usize, plan: &Plan) -> Option<(Arc, Carried)> {
        let road = &self.view.links[link];
        let to = row.spot(self.view, &road.next)?;
        let from = row.spot(self.view, &road.from).filter(|from| to < *from)?;
        let centre = |(_, at): (usize, usize)| centre_of(&plan.segments[at]);
        let half = |to, from, reach| Arc {
            to,
            from,
            level: 0,
            reach,
        };
        let arc = match (to.0 == index, from.0 == index) {
            (true, true) => Arc::whole(centre(to), centre(from)),
            (true, false) => half(centre(to), self.width - 1, Reach::FromLater),
            (false, true) => half(0, centre(from), Reach::ToEarlier),
            (false, false) => return None,
        };
        Some((arc, Carried::Back(link)))
    }

    /// A taken terminal link from a state of rail `index` into a terminal
    /// beneath the row, as its road down: `╰─ n ─ᐳ ⊗ stop`, the name
    /// clamped with `…` to the columns left, and nothing when the width
    /// leaves no room for the head and a letter.
    fn down(&self, row: &Folded, index: usize, link: usize, plan: &Plan) -> Option<(Arc, Carried)> {
        let road = &self.view.links[link];
        let next = (self.view.states.iter()).find(|state| state.name == road.next)?;
        let (rail, at) = row.spot(self.view, &road.from)?;
        let beneath = match next.place {
            Place::Beneath => rail == index,
            Place::Row | Place::End => false,
        };
        let to = centre_of(&plan.segments[at]);
        // `╰─ n ─`, then the head.
        let head = to + 5 + width_of(&road.count.to_string());
        let room = self.width.saturating_sub(head + 4);
        if !beneath || room == 0 {
            return None;
        }
        let tag = format!("ᐳ ⊗ {}", clamp(&road.next, room));
        let arc = Arc {
            to,
            from: head + width_of(&tag) - 1,
            level: 0,
            reach: Reach::Down,
        };
        Some((arc, Carried::Down { link, head, tag }))
    }

    /// `link`'s runs over the arrow whose head is at `head`, ending a
    /// column before it, on the first counts row with room; into `drawn`
    /// when written.
    fn count_before(&self, paint: &mut Paint, head: usize, link: usize, drawn: &mut Drawn) {
        let text = self.view.links[link].count.to_string();
        let x = head.saturating_sub(width_of(&text));
        if paint
            .labels
            .write(paint.cells, &paint.rows, (x, head), &text)
        {
            drawn.insert(link);
        }
    }

    /// The roads between neighbours: a taken link's runs above its arrow,
    /// and no road at all where none was taken; then each state's runs to
    /// itself, `↻n` from its node, on the counts row that keeps it clear
    /// of every count before it.
    fn roads(&self, paint: &mut Paint, plan: &Plan, rail: &[usize], drawn: &mut Drawn) {
        for (pair, segs) in rail.windows(2).zip(plan.segments.windows(2)) {
            let Some(link) = self.taken(pair[0], pair[1]) else {
                for x in segs[0].rail.1 + 1..segs[1].rail.0 {
                    put(paint.cells, x, plan.rail_row, " ", plain());
                }
                continue;
            };
            self.count_before(paint, segs[1].rail.0 - 1, link, drawn);
        }
        for (at, seg) in rail.iter().zip(&plan.segments) {
            if let Some(link) = self.taken(*at, *at) {
                let text = format!("↻{}", self.view.links[link].count);
                let x = seg.rail.0;
                if paint
                    .labels
                    .write(paint.cells, &paint.rows, (x, self.width), &text)
                {
                    drawn.insert(link);
                }
            }
        }
    }

    /// The link across the fold between the last state of one rail and
    /// the first of the next, when the range took it: its road runs out
    /// to the one rail's `›`, and in from the next's `‹` to an arrow with
    /// its runs above it.
    fn crossing(
        &self,
        paint: &mut Paint,
        plan: &Plan,
        row: &Folded,
        index: usize,
        drawn: &mut Drawn,
    ) {
        let rail = row.rail(index);
        let out = (row.rails.get(index + 1))
            .and_then(|next| self.taken(rail[rail.len() - 1], row.states[next.start]));
        if out.is_some() {
            let last = plan.segments[plan.segments.len() - 1].rail.1;
            for x in last + 1..self.width - 1 {
                put(paint.cells, x, plan.rail_row, "─", plain());
            }
        }
        let before = (index.checked_sub(1)).map(|before| row.states[row.rails[before].end - 1]);
        if let Some(link) = before.and_then(|before| self.taken(before, rail[0])) {
            let head = plan.segments[0].rail.0 - 1;
            for x in 1..head {
                put(paint.cells, x, plan.rail_row, "─", plain());
            }
            put(paint.cells, head, plan.rail_row, "ᐳ", plain());
            self.count_before(paint, head, link, drawn);
        }
    }

    /// A back arc's ` ×n ` on its own dashes, as near its middle as they
    /// stand clear of its head, its corners and any deeper arc's end;
    /// false when no stretch of them is wide enough.
    fn arc_count(&self, cells: &mut Cells, arc: &Arc, row: usize, link: usize) -> bool {
        let text = format!(" ×{} ", self.view.links[link].count);
        let wide = width_of(&text);
        let middle = (arc.to + arc.from) / 2;
        let mut starts: Vec<usize> = (arc.to + 1..arc.from).collect();
        starts.sort_by_key(|x| (x + wide / 2).abs_diff(middle));
        let clear = |x: &usize| (*x..*x + wide).all(|column| under(cells, column, row) == "─");
        let Some(x) = starts
            .into_iter()
            .filter(|x| x + wide <= arc.from)
            .find(clear)
        else {
            return false;
        };
        put(cells, x, row, &text, plain());
        true
    }

    /// What each arc carries, written over its painted row: a back arc's
    /// count, and a road down's runs and the terminal it reaches.
    fn arc_counts(&self, cells: &mut Cells, plan: &Plan, carried: &[Carried], drawn: &mut Drawn) {
        let row = plan.arc_row.unwrap_or(plan.rows);
        for (arc, carried) in plan.arcs.iter().zip(carried) {
            let at = row + arc.level;
            match carried {
                Carried::Back(link) => {
                    if self.arc_count(cells, arc, at, *link) {
                        drawn.insert(*link);
                    }
                }
                Carried::Down { link, head, tag } => {
                    let runs = format!(" {} ", self.view.links[*link].count);
                    put(cells, arc.to + 2, at, &runs, plain());
                    put(cells, *head, at, tag, plain());
                    drawn.insert(*link);
                }
            }
        }
    }

    /// Rail `index` of `row`, painted and counted, as plain text: no
    /// colour, trailing blanks trimmed and blank rows dropped. A rail the
    /// fold continues wears the TUI's own elision marks, `›` and `‹`.
    fn rail_lines(&self, row: &Folded, index: usize, drawn: &mut Drawn) -> Vec<String> {
        let rail = row.rail(index);
        let phases: Vec<Phase> = rail.iter().map(|at| self.phase_of(*at)).collect();
        let mut plan = planned(&phases, self.width);
        let (arcs, carried): (Vec<Arc>, Vec<Carried>) =
            self.arcs(row, index, &plan).into_iter().unzip();
        plan.arcs = arcs;
        nest(&mut plan);
        let mut cells = paint_cells(&plan, 0, false);
        let mut paint = Paint {
            cells: &mut cells,
            labels: Labels::default(),
            rows: [plan.rail_row - 1, plan.rail_row - 2],
        };
        self.roads(&mut paint, &plan, rail, drawn);
        self.crossing(&mut paint, &plan, row, index, drawn);
        self.arc_counts(&mut cells, &plan, &carried, drawn);
        if index > 0 {
            put(&mut cells, 0, plan.rail_row, "‹", plain());
        }
        if index + 1 < row.rails.len() {
            put(&mut cells, self.width - 1, plan.rail_row, "›", plain());
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

    /// `states` folded into rails at the drawing's width.
    fn fold<'r>(&self, states: &'r [usize]) -> Folded<'r> {
        let phases: Vec<Phase> = states.iter().map(|at| self.phase_of(*at)).collect();
        Folded {
            states,
            rails: folds(&phases, self.width),
        }
    }

    /// A rule taken that no rail drew, in the rail's own glyphs: a back
    /// rule as its arc's head and `×n`, any other to its next state, a
    /// terminal marked `⊗`.
    fn listed(&self, at: usize) -> String {
        let link = &self.view.links[at];
        match link.kind {
            RuleKind::Back => format!("  {} ᐸ── {}  ×{}", link.next, link.from, link.count),
            RuleKind::Terminal => format!("  {} ──ᐳ ⊗ {}  {}", link.from, link.next, link.count),
            RuleKind::Forward | RuleKind::SelfLoop => {
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

/// One rail's cells as its counts are written over them: the counts rows
/// over the rail, and what each already holds.
struct Paint<'c> {
    cells: &'c mut Cells,
    labels: Labels,
    rows: [usize; 2],
}

/// The links a rail drew, by their index in the view.
type Drawn = BTreeSet<usize>;

fn planned(phases: &[Phase], width: usize) -> Plan {
    plan(phases, None, "running", None, None, width, RAIL_ROWS)
}

/// The fold: each rail takes as many states as fit `width` whole, from
/// the first, and a state that does not is the next rail's first. A rail
/// never scrolls a state away as the TUI's window does, so every state is
/// drawn; at the narrowest a rail holds one state.
fn folds(phases: &[Phase], width: usize) -> Vec<Range<usize>> {
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
pub(crate) fn recipe_graph(view: &GraphView, width: usize) -> String {
    drawn_links(view, width).0
}

/// [`recipe_graph`], with the links its rails drew: every other taken
/// link is listed under them.
pub(crate) fn drawn_links(view: &GraphView, width: usize) -> (String, Drawn) {
    let mut drawn = Drawn::new();
    if view.states.is_empty() {
        return (String::new(), drawn);
    }
    let drawing = Drawing { view, width };
    let mut row = drawing.placed(Place::Row);
    row.extend(drawing.placed(Place::End));
    let beneath = drawing.placed(Place::Beneath);
    let mut lines = Vec::new();
    for states in [&row, &beneath] {
        let folded = drawing.fold(states);
        for index in 0..folded.rails.len() {
            lines.extend(drawing.rail_lines(&folded, index, &mut drawn));
        }
    }
    let under = drawing.under(&drawn);
    lines.extend(under.iter().flat_map(|line| fitted(line, width)));
    let text = (lines.iter())
        .map(|line| format!("{line}\n"))
        .collect::<String>()
        + "\n";
    (text, drawn)
}

#[cfg(test)]
mod tests {
    use super::*;

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
