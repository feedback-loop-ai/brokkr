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

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use brokkr_view::{GraphView, ModelAtBoundary, Place, RuleKind};

use super::*;

/// The columns the drawing takes when stdout is not a terminal, or is one
/// whose size reads as 0 or not at all: plain text that pipes and
/// snapshots alike on every host.
pub(crate) const PIPED_WIDTH: usize = 100;

/// The narrowest drawing that has rails: the plan's margins either side
/// of a node, the blank beside it, and a label clamped to three columns
/// and `…` (see [`Drawing::phase_of`]). One state to a rail fits from
/// here by construction; narrower, a state's node and name would clamp to
/// nothing, so its states are written as lines.
const RAIL_LEAST: usize = 2 * (1 + BOX_PAD) + 2 + 4;

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

/// States of the view in their order, the fold that puts them on rails,
/// and the arc row each taken back link the fold cut rides on every rail
/// it spans ([`Folded::lanes`]).
struct Folded<'r> {
    states: &'r [usize],
    rails: Vec<Range<usize>>,
    lanes: BTreeMap<usize, usize>,
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

    /// The lane of each taken back link of `view` whose states the fold
    /// put on two rails: the narrowest first, each on the shallowest lane
    /// no link already dealt holds on a rail they share. Its pieces ride
    /// that one row on every rail between its ends, so the `›` a link
    /// leaves one rail by is the `‹` it enters the next by, on the same
    /// row, and two links cut by one fold are never read as each other
    /// (review 3's F2).
    fn lanes(&mut self, view: &GraphView) {
        let mut cut: Vec<(usize, (usize, usize), usize)> = Vec::new();
        for (link, road) in view.links.iter().enumerate() {
            let (Some(to), Some(from)) = (self.spot(view, &road.next), self.spot(view, &road.from))
            else {
                continue;
            };
            let at = |(rail, place): (usize, usize)| self.rails[rail].start + place;
            let back = match road.kind {
                RuleKind::Back => road.count > 0,
                RuleKind::Forward | RuleKind::SelfLoop | RuleKind::Terminal => false,
            };
            if back && to.0 < from.0 {
                cut.push((at(from) - at(to), (to.0, from.0), link));
            }
        }
        cut.sort_unstable();
        let mut dealt: Vec<((usize, usize), usize)> = Vec::new();
        for (_, span, link) in cut {
            let shares =
                |(other, _): &&((usize, usize), usize)| span.0 <= other.1 && other.0 <= span.1;
            let held: BTreeSet<usize> =
                dealt.iter().filter(shares).map(|(_, lane)| *lane).collect();
            let lane = (0..).find(|lane| !held.contains(lane)).unwrap_or(0);
            self.lanes.insert(link, lane);
            dealt.push((span, lane));
        }
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
    /// and median, clamped with `…` where the width leaves no room for it
    /// beside the node, a terminal's name marked `⊗`, and lit as live when
    /// a run stands in it.
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
        // The plan's budget less the node and the blank beside it.
        let room = self.width.saturating_sub(2 * (1 + BOX_PAD) + 2);
        let label = format!("{} · {}", state.entered, state.median.text);
        Phase {
            name,
            strategy: None,
            visits: 0,
            current: false,
            plain: false,
            columns: vec![Column {
                label: Some(clamp(&label, room)),
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
    /// ([`Drawing::down`]), the links landing furthest right first: the
    /// order [`nest`] keeps among arcs of one width.
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
    /// states stand on this rail; when the fold put them on two, its
    /// landing half (out to `›`), its departing half (in from `‹`), or
    /// its passage under a rail between them (`‹` to `›`), each on the
    /// link's one lane ([`Folded::lanes`]); and nothing when the rail is
    /// not within its span or it does not run leftward along the row.
    fn back(&self, row: &Folded, index: usize, link: usize, plan: &Plan) -> Option<(Arc, Carried)> {
        let road = &self.view.links[link];
        let to = row.spot(self.view, &road.next)?;
        let from = row.spot(self.view, &road.from).filter(|from| to < *from)?;
        let centre = |(_, at): (usize, usize)| centre_of(&plan.segments[at]);
        let level = row.lanes.get(&link).copied().unwrap_or(0);
        let cut = |to, from, reach| Arc {
            to,
            from,
            level,
            reach,
        };
        let edge = self.width - 1;
        let arc = match (to.0.cmp(&index), from.0.cmp(&index)) {
            (Ordering::Equal, Ordering::Equal) => Arc::whole(centre(to), centre(from)),
            (Ordering::Equal, Ordering::Greater) => cut(centre(to), edge, Reach::FromLater),
            (Ordering::Less, Ordering::Equal) => cut(0, centre(from), Reach::ToEarlier),
            (Ordering::Less, Ordering::Greater) => cut(0, edge, Reach::Through),
            (Ordering::Less | Ordering::Equal | Ordering::Greater, Ordering::Less)
            | (Ordering::Greater, Ordering::Equal | Ordering::Greater) => return None,
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
        let (_, at) = (row.spot(self.view, &road.from)).filter(|(rail, _)| *rail == index)?;
        let beneath = match next.place {
            Place::Beneath => true,
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
    /// colour, trailing blanks trimmed and blank rows dropped, but for an
    /// arc lane a link holds deeper. A rail the
    /// fold continues wears the TUI's own elision marks, `›` and `‹`.
    /// Under [`RAIL_LEAST`] columns its states are written as lines
    /// instead, held to the width as every line under the rails is.
    fn rail_lines(&self, row: &Folded, index: usize, drawn: &mut Drawn) -> Vec<String> {
        let rail = row.rail(index);
        let phases: Vec<Phase> = rail.iter().map(|at| self.phase_of(*at)).collect();
        let mut plan = planned(&phases, self.width);
        if self.width < RAIL_LEAST {
            return (rail.iter().zip(&phases))
                .map(|(at, phase)| {
                    let state = &self.view.states[*at];
                    format!(
                        "  {}  {} · {}",
                        phase.name, state.entered, state.median.text
                    )
                })
                .flat_map(|line| fitted(&line, self.width))
                .collect();
        }
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
        // Every row under the rail down to the deepest lane this rail's
        // arcs hold stays, blank or not: a fold-cut link passing under a
        // rail where a shallower lane holds nothing, or whose state's name
        // paints blank, keeps its row, so its `›` and the next rail's `‹`
        // stand on one row (review 2's C1). The box row alone may go: the
        // drawing selects nothing, so it is blank under every rail alike.
        let lanes = plan.arc_row.map_or(0..0, |_| plan.rail_row + 1..plan.rows);
        let kept = |at: &usize| lanes.contains(at) && plan.box_row != Some(*at);
        (cells.iter().enumerate())
            .map(|(at, row)| {
                let text: String = (row.iter().flatten())
                    .map(|(glyph, _)| glyph.as_str())
                    .collect();
                (at, text.trim_end().to_string())
            })
            .filter(|(at, text)| !text.is_empty() || kept(at))
            .map(|(_, text)| text)
            .collect()
    }

    /// `states` folded into rails at the drawing's width.
    fn fold<'r>(&self, states: &'r [usize]) -> Folded<'r> {
        let phases: Vec<Phase> = states.iter().map(|at| self.phase_of(*at)).collect();
        let mut folded = Folded {
            states,
            rails: folds(&phases, self.width),
            lanes: BTreeMap::new(),
        };
        folded.lanes(self.view);
        folded
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

/// The columns a continued piece of a word needs: a letter between the
/// `…` it carries on from and the `…` it carries on with. Narrower, a
/// word is continued with no marks at all, a letter or two to a line,
/// since a mark would leave no room for the word.
const PIECE_LEAST: usize = 3;

/// How a line too narrow for a word continues it: the blanks a further
/// line starts with, and the mark either side of a break.
struct Carry {
    width: usize,
    carried: String,
    mark: &'static str,
}

/// `word` appended to `line`, broken where it passes the width: the piece
/// that fits ends with the mark, and the rest carries on a new line from
/// it, so a word is never cut, only continued. Each piece is held to its
/// room in display columns: where a character is too wide for the room
/// the line's blanks leave it, the blanks yield first and then the mark
/// it carries on from, and one that fits the width only without the mark
/// it carries on with stands alone. A character wider than the whole width (a
/// two-column glyph at one column) is written as `…`, the mark [`clamp`]
/// leaves for what it cut, so no line passes the width (review 2's C2).
fn broken(line: &mut String, lines: &mut Vec<String>, word: &str, carry: &Carry) {
    let mut rest = word;
    while width_of(line) + width_of(rest) > carry.width {
        let first = rest.chars().next().map_or(0, char::len_utf8);
        let glyph = &rest[..first];
        // The loop runs only on a fresh line, whose blanks and mark are
        // all it holds: `fitted` starts a word on one when it would not
        // fit after the words before it. A last glyph carries no mark on.
        let trailing = if rest.len() > first { carry.mark } else { "" };
        let needed = width_of(glyph) + width_of(trailing);
        while !line.is_empty() && width_of(line) + needed > carry.width {
            line.remove(0);
        }
        if width_of(line) + width_of(rest) <= carry.width {
            break;
        }
        let room = (carry.width).saturating_sub(width_of(line) + width_of(carry.mark));
        let fits = (rest
            .char_indices()
            .map(|(at, character)| at + character.len_utf8()))
        .take_while(|end| width_of(&rest[..*end]) <= room)
        .last();
        let cut = fits.unwrap_or(first);
        // A glyph that fits the width but not beside the mark stands alone
        // on its line, the mark yielding; only one wider than the width is
        // `…` (review 3's F1).
        let lone = if width_of(glyph) <= carry.width {
            glyph
        } else {
            "…"
        };
        line.push_str(fits.map_or(lone, |cut| &rest[..cut]));
        rest = &rest[cut..];
        if rest.is_empty() {
            return;
        }
        if width_of(line) + width_of(carry.mark) <= carry.width {
            line.push_str(carry.mark);
        }
        let next = format!("{}{}", carry.carried, carry.mark);
        lines.push(std::mem::replace(line, next));
    }
    line.push_str(rest);
}

/// The rule every line under the rails is held to: `line` broken before
/// a word that would pass `width`, each further line indented three
/// columns past the first, and a word wider than a line continued with
/// `…` (see [`broken`]). The indents yield to the width, so a piece of a
/// word always has its [`PIECE_LEAST`] columns, and a line never passes
/// `width` however narrow it is.
fn fitted(line: &str, width: usize) -> Vec<String> {
    let body = line.trim_start_matches(' ');
    let indented = line.len() - body.len();
    let (least, mark) = if width < PIECE_LEAST {
        (1, "")
    } else {
        (PIECE_LEAST, "…")
    };
    let room = width.saturating_sub(least);
    let indent = " ".repeat(indented.min(room));
    let carry = Carry {
        width,
        carried: " ".repeat((indented + 3).min(room)),
        mark,
    };
    let mut lines = Vec::new();
    let mut current = indent;
    let mut fresh = true;
    for (gap, word) in spaced(body) {
        if !fresh && width_of(&current) + width_of(gap) + width_of(word) > width {
            lines.push(std::mem::replace(&mut current, carry.carried.clone()));
            fresh = true;
        }
        if !fresh {
            current.push_str(gap);
        }
        broken(&mut current, &mut lines, word, &carry);
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
        // The indents yield to the width: a piece keeps its three columns,
        // and under three a word goes on unmarked.
        assert_eq!(fitted("  abcd ef", 4), [" ab…", " …cd", " ef"]);
        assert_eq!(fitted("  abcd", 3), ["ab…", "…cd"]);
        assert_eq!(fitted("  ab c", 1), ["a", "b", "c"]);
        // A double-width glyph is held to its room: the blanks yield, then
        // the mark it carries on from, and at one column it is `…`.
        let wide = [
            vec!["…", "…", "…"],
            vec!["界", "界", "界"],
            vec!["界…", "界…", "…界"],
            vec![" 界…", "…界…", " …界"],
            vec!["  界…", " …界…", "  …界"],
            vec!["  界…", "  …界…", "   …界"],
        ];
        for (width, lines) in (1..).zip(wide) {
            assert_eq!(fitted("  界界界", width), lines, "{width}");
        }
        // A three-column glyph as wide as the line stands alone on it, the
        // mark it would carry on with yielding; a column more, it returns.
        assert_eq!(fitted("  ៘៘៘", 3), ["៘", "៘", "៘"]);
        assert_eq!(fitted("  ៘៘៘", 4), ["៘…", "៘…", "…៘"]);
    }
}
