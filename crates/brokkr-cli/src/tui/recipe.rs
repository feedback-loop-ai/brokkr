//! A recipe's delivery graph in the run rail's grammar (#1145). `brokkr
//! graph` lays the recipe's states on the rail [`plan`] lays a run's phases
//! on, paints it with the one painter, and writes the traffic the view
//! counted over those cells: a recipe and a run read alike, drawn by one
//! renderer (decision 0071 ruling 5). Every figure is the view's own
//! (ruling 7); this file places them and counts nothing.
//!
//! What the rail cannot draw is listed under it, never dropped: a rule
//! taken between states that are not neighbours on the rail, or that a
//! fold put on two rails, a rule into a terminal beneath the row, and a
//! park.

use std::collections::BTreeSet;

use brokkr_view::{GraphView, ModelAtBoundary, Road};

use super::*;

/// The columns the drawing takes when stdout is not a terminal: plain
/// text that pipes and snapshots alike on every host.
pub(crate) const PIPED_WIDTH: usize = 100;

/// The rows one rail is planned in: the counts' row, the rail, the names
/// and the return arcs' row, with the selection box's row the plan
/// reserves left blank (no state is selected here) and dropped.
const RAIL_ROWS: usize = 5;

/// The drawing's legend, so a bare number on the rail says what it counts.
const LEGEND: [&str; 2] = [
    "nodes: runs entered · median stay · ∙ a live run · · never entered",
    "arrows: runs moved · ↻ to itself · ×n back · ⊗ a terminal beneath the row",
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

/// Where the view places state `at`, in the words its wire carries: `row`,
/// `end` or `beneath`. The view's `Place`, like its `State`, `Link` and
/// `RuleKind`, is not named outside `brokkr-view`, and naming it would
/// grow that crate's public API, which is the operator's to rule; so the
/// drawing reads states and links by index and these two by their words.
fn place_of(view: &GraphView, at: usize) -> String {
    let word = serde_json::to_value(view.states[at].place).ok();
    word.and_then(|word| word.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// The runs the range moved from `from` to `next`: at most one count, as
/// the view holds one link per pair, and none when it took none.
fn taken<'a>(
    view: &'a GraphView,
    from: &'a str,
    next: &'a str,
) -> impl Iterator<Item = usize> + 'a {
    (view.links.iter())
        .filter(move |link| link.from == from && link.next == next && link.count > 0)
        .map(|link| link.count)
}

/// One state as a rail phase: its node labelled with its figures, lit as
/// live when a run stands in it, and the back links taken into it as the
/// roads its return arcs draw.
fn phase_of(view: &GraphView, at: usize, label: String) -> Phase {
    let state = &view.states[at];
    let live = (view.live.iter()).any(|run| run.state.as_deref() == Some(state.name.as_str()));
    let (word, class) = match (live, state.entered > 0) {
        (true, _) => ("active", "in-active"),
        (false, true) => ("finished", "on-phosphor"),
        (false, false) => ("", ""),
    };
    let back = (view.links.iter()).filter(|link| link.kind.word() == "back");
    Phase {
        name: state.name.clone(),
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
        returns: back
            .filter(|link| link.next == state.name && link.count > 0)
            .map(|link| Road {
                from: link.from.clone(),
                taken: link.count as u64,
                rules: Vec::new(),
            })
            .collect(),
    }
}

/// The rail's phases: the row's states in the view's order, then the
/// state that ends the row. A terminal beneath the row is no node on it.
fn railed(view: &GraphView) -> Vec<Phase> {
    let placed =
        |place: &'static str| (0..view.states.len()).filter(move |at| place_of(view, *at) == place);
    let row = placed("row").map(|at| {
        let state = &view.states[at];
        phase_of(
            view,
            at,
            format!("{} · {}", state.entered, state.median.text),
        )
    });
    let end =
        placed("end").map(|at| phase_of(view, at, format!("{} · end", view.states[at].entered)));
    row.chain(end).collect()
}

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

/// The segments whose road meets the arc row at `x`.
fn seg_at(plan: &Plan, x: usize) -> impl Iterator<Item = &Seg> {
    (plan.segments.iter()).filter(move |seg| centre_of(seg) == x)
}

/// The counts written over one painted rail: a forward rule's above its
/// arrow, a self rule's above its node, a back rule's on its arc. Each
/// pair it draws goes into `drawn`.
fn counted(
    cells: &mut Cells,
    plan: &Plan,
    view: &GraphView,
    drawn: &mut BTreeSet<(String, String)>,
) {
    let above = plan.rail_row - 1;
    for pair in plan.segments.windows(2) {
        for count in taken(view, &pair[0].key, &pair[1].key) {
            let text = count.to_string();
            let head = pair[1].rail.0 - 1;
            put(
                cells,
                head.saturating_sub(width_of(&text)),
                above,
                &text,
                plain(),
            );
            drawn.insert((pair[0].key.clone(), pair[1].key.clone()));
        }
    }
    for seg in &plan.segments {
        for count in taken(view, &seg.key, &seg.key) {
            put(cells, seg.rail.0, above, &format!("↻{count}"), plain());
            drawn.insert((seg.key.clone(), seg.key.clone()));
        }
    }
    let Some(row) = plan.arc_row else {
        return;
    };
    // Two states' nodes stand at least a labelled node and a connector
    // apart, so an arc is always wider than its ` ×n ` and the count sits
    // inside it, centred.
    for arc in &plan.arcs {
        for (from, to) in seg_at(plan, arc.from).zip(seg_at(plan, arc.to)) {
            for count in taken(view, &from.key, &to.key) {
                let text = format!(" ×{count} ");
                let x = (arc.to + arc.from) / 2 - width_of(&text) / 2;
                put(cells, x, row, &text, plain());
                drawn.insert((from.key.clone(), to.key.clone()));
            }
        }
    }
}

/// One rail of the fold, painted and counted, as plain text: no colour,
/// trailing blanks trimmed and blank rows dropped. A rail the fold
/// continues wears the TUI's own elision marks, `›` and `‹`.
fn rail_lines(
    view: &GraphView,
    phases: &[Phase],
    width: usize,
    folded: (bool, bool),
    drawn: &mut BTreeSet<(String, String)>,
) -> Vec<String> {
    let plan = planned(phases, width);
    let mut cells = paint_cells(&plan, 0, false);
    counted(&mut cells, &plan, view, drawn);
    if folded.0 {
        put(&mut cells, 0, plan.rail_row, "‹", plain());
    }
    if folded.1 {
        put(&mut cells, width - 1, plan.rail_row, "›", plain());
    }
    (cells.iter())
        .map(|row| {
            let text: String = row
                .iter()
                .flatten()
                .map(|(glyph, _)| glyph.as_str())
                .collect();
            text.trim_end().to_string()
        })
        .filter(|text| !text.is_empty())
        .collect()
}

/// A rule taken that no rail drew, in the rail's own glyphs: a back rule
/// as its arc's head and `×n`, any other to its next state, a terminal
/// beneath the row marked `⊗`.
fn listed(view: &GraphView, at: usize) -> String {
    let link = &view.links[at];
    let beneath = (0..view.states.len())
        .any(|state| view.states[state].name == link.next && place_of(view, state) == "beneath");
    let mark = ["", "⊗ "][usize::from(beneath)];
    match link.kind.word() == "back" {
        true => format!("  {} ᐸ── {}  ×{}", link.next, link.from, link.count),
        false => format!("  {} ──ᐳ {mark}{}  {}", link.from, link.next, link.count),
    }
}

/// `words` after `head`, two spaces apart, a line broken before a word
/// that would pass `width`, each further line indented.
fn wrapped(head: String, words: &[&str], width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = head;
    for word in words {
        if width_of(&line) + 2 + width_of(word) > width {
            lines.push(std::mem::replace(&mut line, "   ".to_string()));
        }
        line.push_str("  ");
        line.push_str(word);
    }
    lines.push(line);
    lines
}

/// `brokkr graph`'s drawing of `view` at `width` columns: the rails, what
/// they could not draw, the rules that never fired, and the legend, then
/// a blank line. Nothing at all when the view has no state to draw: a
/// recipe that does not compile and that no run carries.
pub(crate) fn recipe_graph(view: &GraphView, width: usize) -> String {
    if view.states.is_empty() {
        return String::new();
    }
    let phases = railed(view);
    let rails = folds(&phases, width);
    let mut drawn = BTreeSet::new();
    let mut lines = Vec::new();
    for (index, rail) in rails.iter().enumerate() {
        let folded = (index > 0, index + 1 < rails.len());
        lines.extend(rail_lines(
            view,
            &phases[rail.clone()],
            width,
            folded,
            &mut drawn,
        ));
    }
    let unseen = (0..view.links.len()).filter(|at| {
        let link = &view.links[*at];
        link.count > 0 && !drawn.contains(&(link.from.clone(), link.next.clone()))
    });
    lines.extend(unseen.map(|at| listed(view, at)));
    let parks = (view.rules.iter()).filter(|rule| rule.next.is_none() && rule.count > 0);
    lines.extend(parks.map(|rule| format!("  {} ⊙ park  {} · {}", rule.from, rule.count, rule.id)));
    let dormant: Vec<&str> = (view.rules.iter())
        .filter(|rule| rule.count == 0)
        .map(|rule| rule.id.as_str())
        .collect();
    let head = format!("never fired {}", view.never_fired.text);
    lines.extend(wrapped(head, &dormant, width));
    lines.extend(LEGEND.map(str::to_string));
    lines
        .iter()
        .map(|line| format!("{line}\n"))
        .collect::<String>()
        + "\n"
}
