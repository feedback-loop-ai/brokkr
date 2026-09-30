//! The graph's layout: [`plan`] is pure owned integer geometry over the
//! models and the rect, and fits by construction.

use super::*;

/// A node on the rail or in a lane.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Mark {
    pub(super) x: usize,
    pub(super) row: usize,
    pub(super) class: Class,
    /// The pulse gate, read from a model field — `summary.status` for a
    /// phase's own node, `Node.state` for one inside it.
    pub(super) live: bool,
    pub(super) label: String,
    pub(super) selected: bool,
}

/// A fork: the lanes leave the rail at `x0` and rejoin it at `x1`.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Join {
    pub(super) x0: usize,
    pub(super) x1: usize,
    /// The rows the lanes run on, in draw order.
    pub(super) rows: Vec<usize>,
    /// True when the member count is odd and one member rides the rail
    /// row itself — the `┼` case.
    pub(super) on_rail: bool,
    /// The step's own name, centred on the rail row when no member is
    /// there to occupy it.
    pub(super) label: Option<String>,
}

/// A return arc: the road back, under the name baseline, from beneath
/// the phase whose ruling sent the run back to beneath the phase it
/// landed in. Absolute plan-level columns, like `Plan.rail` and
/// `Plan.edges` and unlike a `Join` — an arc spans from one segment's
/// rail to another's, with whole phases possibly between them.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Arc {
    /// The landing column, where the mirror head points: beneath the
    /// target phase's own rail content.
    pub(super) to: usize,
    /// The departing column, beneath the source phase's.
    pub(super) from: usize,
}

/// One phase: a span of the rail, the marks and forks on it, and the
/// name it wears on the shared baseline.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Seg {
    /// `Phase.name` — the cursor key, and the reason `brokkr-view` pins
    /// name uniqueness with a test of its own.
    pub(super) key: String,
    /// The inclusive span of this phase's own rail content — where the
    /// connector from the phase before lands, and where the one to the
    /// phase after leaves from.
    pub(super) rail: (usize, usize),
    /// The inclusive span of everything the phase DRAWS: its rail
    /// extent and its name, whichever reaches further. A name wider
    /// than the rail hangs off its node into the gaps, so this is a
    /// union and not a box the name widened.
    pub(super) x0: usize,
    pub(super) x1: usize,
    /// Already clamped, and already carrying its `▸` and its `×N`.
    pub(super) name: String,
    pub(super) name_x: usize,
    /// `Some` only for the CURRENT phase, which takes the run's own
    /// colour on the baseline exactly as `ui.html` does — so the current
    /// phase is distinguished even where it has no single rail node of
    /// its own to fill. Selection is a different channel entirely.
    pub(super) class: Option<Class>,
    pub(super) selected: bool,
    pub(super) marks: Vec<Mark>,
    pub(super) joins: Vec<Join>,
}

/// One frame's geometry: owned integers, no borrow of a model, nothing
/// that varies with `tick`.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Plan {
    pub(super) mode: Mode,
    pub(super) width: usize,
    pub(super) rows: usize,
    pub(super) rail_row: usize,
    pub(super) name_row: usize,
    /// The inclusive span of the rail line, absent when nothing is drawn.
    pub(super) rail: Option<(usize, usize)>,
    /// The reserved row for the selection box's lower edge.
    pub(super) box_row: Option<usize>,
    /// The reserved row the return arcs run on, under everything else.
    /// Absent when the journal recorded no return, and absent when the
    /// pane cannot hold the row — half an arc is the box's own lesson.
    pub(super) arc_row: Option<usize>,
    /// The roads back, at most one per `(from, to)` pair however many
    /// times it was taken: repeats ride the `×N` marker.
    pub(super) arcs: Vec<Arc>,
    /// Where the `→` heads sit.
    pub(super) edges: Vec<usize>,
    pub(super) segments: Vec<Seg>,
    pub(super) left_elided: bool,
    pub(super) right_elided: bool,
}

/// The facts a column placement needs that are not the column itself.
struct Ink<'a> {
    status: &'a str,
    node: Option<&'a str>,
    rail_row: usize,
    lane_span: usize,
    lanes: bool,
    budget: usize,
}

/// One phase, laid out relative to its own rail start.
struct Built {
    rail_width: usize,
    /// How far the name spills past its own rail content, left and
    /// right. The name centres on the rail's centre and is NEVER moved
    /// off it; what a wide name claims it claims from the gaps beside
    /// the phase, not as padding inside the rail.
    lead: usize,
    trail: usize,
    name: String,
    /// Where the name starts, from the segment's leftmost drawn column.
    name_off: usize,
    marks: Vec<Mark>,
    joins: Vec<Join>,
    edges: Vec<usize>,
    /// The phase continues past the pane's edge.
    truncated: bool,
}

/// Where a name sits against the rail content it names. The name's own
/// start is the rail's centre less half the name, so its overhang is
/// that subtraction's shortfall on the left (`lead`) and its remainder
/// past the rail's last column on the right (`trail`); `off` is the
/// same placement measured from the segment's leftmost drawn column,
/// which is where the layout finally needs it. One computation, three
/// readers: the segment's drawn extent, the connector, and the box.
struct Named {
    lead: usize,
    trail: usize,
    off: usize,
}

fn name_place(rail_width: usize, name_width: usize) -> Named {
    let centre = rail_width.saturating_sub(1) / 2;
    let half = name_width / 2;
    let lead = half.saturating_sub(centre);
    // The name's last column from the rail's start: its centre plus the
    // half that rounds up. An empty name ends where it begins and
    // overhangs nothing, which needs no branch of its own.
    let end = centre + (name_width - half).saturating_sub(1);
    Named {
        lead,
        trail: end.saturating_sub(rail_width.saturating_sub(1)),
        off: centre + lead - half,
    }
}

/// One column, laid out from `x`. Placement and measurement are the same
/// code, so the two cannot disagree about where the next column starts.
fn place_column(
    column: &Column,
    x: usize,
    ink: &Ink,
    marks: &mut Vec<Mark>,
    joins: &mut Vec<Join>,
) -> usize {
    let members = column.nodes.len();
    if !ink.lanes || members < 2 {
        // One node on the rail with its label beside it — and, where a
        // parallel column could not be drawn as lanes, the `⑂n` that
        // keeps it reading as parallel rather than as a sequential step.
        let chosen = column.nodes.get(worst(&column.nodes));
        let base = match members > 1 {
            true => format!("⑂{members}"),
            false => column_label(column).to_string(),
        };
        // The label is the work's name and nothing else. The served
        // model is a leaf's fact, carried by the seats table and the
        // seat pane; the rail is the abstraction over which work
        // followed which, and a per-seat field on it both crowds the
        // 14-column budget and says nothing about the shape.
        let label = clamp(&base, LABEL_MAX);
        marks.push(Mark {
            x,
            row: ink.rail_row,
            class: chosen.map_or(Class::Unknown, |node| class_for_node(&node.state_class)),
            live: chosen.is_some_and(|node| node.state == "active" && ink.status == "running"),
            selected: chosen.is_some_and(|node| ink.node == Some(node.key.as_str())),
            label: label.clone(),
        });
        return 1 + label_span(&label);
    }
    // A fork: it leaves the rail at `x` and REJOINS it, symmetric about
    // the rail, member `k` of `n` at row offset `lane_offset(k, n)`.
    let mut rows = Vec::new();
    let mut lanes: Vec<Mark> = Vec::new();
    let mut on_rail = false;
    let mut overflow = 0usize;
    for (k, member) in column.nodes.iter().enumerate() {
        let offset = lane_offset(k, members);
        if offset.unsigned_abs() > ink.lane_span {
            overflow += 1;
            continue;
        }
        on_rail = on_rail || offset == 0;
        let row = ink.rail_row.saturating_add_signed(offset);
        rows.push(row);
        lanes.push(Mark {
            x: x + 2,
            row,
            class: class_for_node(&member.state_class),
            live: member.state == "active" && ink.status == "running",
            label: clamp(&member.label, LABEL_MAX),
            selected: ink.node == Some(member.key.as_str()),
        });
    }
    // The members the lane budget could not hold are counted on the
    // outermost lane that was drawn: honest, never silently dropped.
    for lane in lanes.iter_mut().rev().take(overflow.min(1)) {
        lane.label.push_str(&format!(" +{overflow}"));
    }
    let mut inner = 0usize;
    for lane in &lanes {
        inner = inner.max(1 + label_span(&lane.label));
    }
    let label = match (on_rail, &column.label) {
        (false, Some(text)) => Some(clamp(text, LABEL_MAX)),
        _ => None,
    };
    if let Some(text) = &label {
        inner = inner.max(width_of(text));
    }
    marks.append(&mut lanes);
    joins.push(Join {
        x0: x,
        x1: x + inner + 3,
        rows,
        on_rail,
        label,
    });
    inner + 4
}

/// One phase's rail content and its name, relative to its own origin.
fn build(phase: &Phase, lens: Option<&render::Lens>, ink: &Ink) -> Built {
    let mut marks = Vec::new();
    let mut joins = Vec::new();
    let mut edges = Vec::new();
    let mut truncated = false;
    let mut x = 0usize;
    match phase.columns.is_empty() {
        // A plain phase is one node on the rail: the console's r=7 for
        // the current phase against r=5.5 for a visited one, expressed
        // in the only axis a cell has.
        true => {
            marks.push(Mark {
                x: 0,
                row: ink.rail_row,
                class: class_for_phase(phase.current, ink.status),
                live: phase.current && ink.status == "running",
                label: String::new(),
                selected: false,
            });
            x = 1;
        }
        false => {
            for (index, column) in phase.columns.iter().enumerate() {
                let gap = ARROW_WIDTH * usize::from(index > 0);
                let mut column_marks = Vec::new();
                let mut column_joins = Vec::new();
                let width =
                    place_column(column, x + gap, ink, &mut column_marks, &mut column_joins);
                if index > 0 && x + gap + width > ink.budget {
                    // The rest of this phase continues past the pane's
                    // edge, and `›` says so.
                    truncated = true;
                    break;
                }
                if let Some(origin) = (index > 0).then_some(x + gap) {
                    edges.push(origin - 1);
                }
                marks.append(&mut column_marks);
                joins.append(&mut column_joins);
                x += gap + width;
            }
        }
    }
    let name = clamp(&name_text(phase, lens), ink.budget);
    // A wide name no longer widens its segment. It used to, and the
    // rail filled the padding with dashes indistinguishable from the
    // gap, so the connector after a wide label read `────ᐳ` where its
    // neighbours read `──ᐳ` — an accident of arithmetic on the one
    // line whose rhythm is the grammar. The name now hangs off its own
    // node and the frame carries ONE connector length (`connector_of`),
    // so the dash run between any two phases is the same. The name is
    // still centred on its node, which is what the earlier attempt at
    // this got wrong: it clamped names into segment boxes and cascaded
    // dense ones away from the nodes they name.
    let place = name_place(x, width_of(&name));
    Built {
        rail_width: x,
        lead: place.lead,
        trail: place.trail,
        name,
        name_off: place.off,
        marks,
        joins,
        edges,
        truncated,
    }
}

/// `Compressed`: one node per phase, wearing the phase's own name and
/// the `⑂n` of its widest parallel column. Today's information on one
/// line — never a blank pane and never a refusal, because losing the
/// graph while dragging a window edge is worse than a compact one.
fn compressed(phase: &Phase, lens: Option<&render::Lens>, ink: &Ink) -> Built {
    let mut label = name_text(phase, lens);
    let widest = phase
        .columns
        .iter()
        .map(|column| column.nodes.len())
        .filter(|count| *count > 1)
        .max();
    if let Some(count) = widest {
        label.push_str(&format!(" ⑂{count}"));
    }
    let label = clamp(&label, ink.budget.saturating_sub(2));
    let width = 1 + label_span(&label);
    Built {
        rail_width: width,
        lead: 0,
        trail: 0,
        name: String::new(),
        name_off: 0,
        marks: vec![Mark {
            x: 0,
            row: ink.rail_row,
            class: class_for_phase(phase.current, ink.status),
            live: phase.current && ink.status == "running",
            label,
            selected: false,
        }],
        joins: Vec::new(),
        edges: Vec::new(),
        truncated: false,
    }
}

/// The window is derived from the cursor **every frame** — there is no
/// scroll-offset field, so there is no second piece of state that can
/// desynchronise from the selection.
pub(super) fn anchor_of(phases: &[Phase], cursor: Option<&str>) -> usize {
    phases
        .iter()
        .position(|phase| Some(phase.name.as_str()) == cursor)
        .or_else(|| phases.iter().position(|phase| phase.current))
        .unwrap_or(0)
}

/// ONE connector length for the whole frame: the arrow, plus whatever
/// the names on either side of the hungriest gap claim from it. Every
/// pair then joins with the same dash run, which is the operator's
/// ruling — a connector whose length varies with a neighbour's label
/// reads as arithmetic, not as rhythm.
fn connector_of(built: &[Built]) -> usize {
    built
        .windows(2)
        .map(|pair| pair[0].trail + pair[1].lead + ARROW_WIDTH)
        .max()
        .unwrap_or(ARROW_WIDTH)
}

/// The columns a run of segments claims: the rail extents, one
/// connector between each pair, and the overhang the outermost names
/// carry past the rail at either end. Interior overhang is already paid
/// for by the connector, which is what makes this a sum and not a walk.
fn span_of(built: &[Built], start: usize, end: usize, connector: usize) -> usize {
    built[start..=end]
        .iter()
        .map(|item| item.rail_width)
        .sum::<usize>()
        + connector * (end - start)
        + built[start].lead
        + built[end].trail
}

/// Every return the rail could draw, as `(landing, departure)` pairs of
/// PHASE INDICES. The pairs themselves are `Phase.returns` — derived
/// once in `brokkr-view` from the transition that caused the revisit —
/// so nothing here reads a journal, infers backwardness from `visits`,
/// or names a phase.
///
/// Two pairs are dropped, both for want of geometry and neither as a
/// judgement about the journal: a departure naming no phase on this
/// rail has no column to leave from, and a landing sitting LATER on the
/// rail than its departure is not a road drawn leftward — the arc's
/// head has one direction, and the rail's own `ᐳ` is not the arc's to
/// borrow.
pub(super) fn returns_of(phases: &[Phase]) -> Vec<(usize, usize)> {
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for (to, phase) in phases.iter().enumerate() {
        for source in &phase.returns {
            match phases.iter().position(|other| other.name == *source) {
                Some(from) if from > to => pairs.push((to, from)),
                _ => {}
            }
        }
    }
    pairs
}

/// Where a phase's road meets the arc row: the centre of its rail
/// content, so an arc's end sits under the phase's own node rather than
/// under the gap beside it. Two segments' rails are disjoint and a
/// connector apart, so two ends are never the same column and never
/// closer than the head plus its corner.
pub(super) fn centre_of(seg: &Seg) -> usize {
    (seg.rail.0 + seg.rail.1) / 2
}

/// The visible run of segments: the anchor, then grow right, then left,
/// while the budget holds. The console's answer to width was horizontal
/// scrolling; ours is this window, walked by the arrow keys.
fn window(built: &[Built], anchor: usize, connector: usize, budget: usize) -> (usize, usize) {
    if built.is_empty() {
        return (0, 0);
    }
    let mut start = anchor;
    let mut end = anchor;
    loop {
        if end + 1 < built.len() && span_of(built, start, end + 1, connector) <= budget {
            end += 1;
            continue;
        }
        if start > 0 && span_of(built, start - 1, end, connector) <= budget {
            start -= 1;
            continue;
        }
        return (start, end + 1);
    }
}

/// The deepest fork's lane pairs: how many rows the rail needs on EACH
/// side of itself to draw every lane in full.
pub(super) fn lane_pairs(phases: &[Phase]) -> usize {
    phases
        .iter()
        .flat_map(|phase| phase.columns.iter())
        .map(|column| column.nodes.len() / 2)
        .max()
        .unwrap_or(0)
}

/// The rows a full drawing of these phases takes, and not one more:
/// the lanes either side of the rail, the rail itself, the row the
/// selection box's upper edge rides above the topmost lane, the name
/// baseline, the box's lower edge, and the road back when the journal
/// recorded one. [`plan`] deals its rows from the bottom and calls the
/// rest headroom; this is the height at which that headroom is zero,
/// so [`estate`] can give the graph its drawing and nothing else.
pub(super) fn rows_wanted(phases: &[Phase]) -> usize {
    2 * lane_pairs(phases) + 4 + usize::from(!returns_of(phases).is_empty())
}

/// The pure geometry: models and a rect in, owned integers out. It calls
/// `render::keeps_phase` rather than reimplementing the scope predicate,
/// lists **every** phase whether scoped or not — the lens marks, it does
/// not hide, here — and returns a layout that fits by construction.
pub(super) fn plan(
    phases: &[Phase],
    lens: Option<&render::Lens>,
    status: &str,
    cursor: Option<&str>,
    node: Option<&str>,
    width: usize,
    height: usize,
) -> Plan {
    let needed = lane_pairs(phases);
    let mode = mode_for(height, needed);
    // Row allocation is fixed and ordered: the name baseline is the last
    // row, the rail sits one row above the deepest lane it needs, and
    // the lanes spread symmetrically outward from the rail. Anything
    // left over is headroom, so the names stay under their own graph.
    // One row under the names is reserved for the selection box's
    // lower edge, so the box the operator draws around a phase can be
    // symmetric: an upper edge with no lower edge is two floating
    // lines, not a boundary. Reserved only when there is room.
    //
    // Under THAT, one row for the roads back — the last row of the
    // pane, so an arc passes beneath the box rather than through it and
    // the two can never meet by arithmetic. The row exists only for a
    // run whose journal recorded a return (`returns_of` is a fact about
    // the model, not about the layout, so it is safe to ask before the
    // rows are dealt) and only where the pane can hold it: a pane too
    // short omits the arc WHOLE, which is the box's own ruling — half
    // an arc is worse than none.
    let pairs = returns_of(phases);
    let arc_row = (!pairs.is_empty() && height >= 5).then(|| height - 1);
    let box_row = (height >= 4).then(|| height - 1 - usize::from(arc_row.is_some()));
    let name_row =
        height.saturating_sub(1 + usize::from(box_row.is_some()) + usize::from(arc_row.is_some()));
    let lane_span = match mode {
        Mode::Full => needed.min(name_row.saturating_sub(1) / 2),
        _ => 0,
    };
    let ink = Ink {
        status,
        node,
        rail_row: match mode {
            Mode::Compressed => 0,
            _ => name_row.saturating_sub(1 + lane_span),
        },
        lane_span,
        lanes: mode == Mode::Full,
        // Column 0 and the last column are the elision marks' own, and
        // one `BOX_PAD` inside each of those is the selection box's
        // walls — reserved like the box ROW is, so the box breathes the
        // same at the frame's edge as it does in the middle of it.
        budget: width.saturating_sub(2 * (1 + BOX_PAD)),
    };
    let built: Vec<Built> = phases
        .iter()
        .map(|phase| match mode {
            Mode::Compressed => compressed(phase, lens, &ink),
            _ => build(phase, lens, &ink),
        })
        .collect();
    let connector = connector_of(&built);
    let (start, end) = window(&built, anchor_of(phases, cursor), connector, ink.budget);
    let Laid {
        segments,
        edges,
        rail,
    } = lay(
        &built[start..end],
        &phases[start..end],
        connector,
        status,
        cursor,
    );
    // Both ends or neither. A pair reaching a phase the window scrolled
    // away would have to land on the elision mark's own column, so it is
    // not drawn at all — the arithmetic is the window's own bounds, and
    // the painter is told nothing. The ROW stays reserved either way, so
    // walking the rail never lifts the baseline under the operator's eye.
    let arcs: Vec<Arc> = match arc_row {
        None => Vec::new(),
        Some(_) => pairs
            .iter()
            .filter(|(to, from)| *to >= start && *from < end)
            .map(|(to, from)| Arc {
                to: centre_of(&segments[to - start]),
                from: centre_of(&segments[from - start]),
            })
            .collect(),
    };
    Plan {
        mode,
        width,
        rows: height,
        rail_row: ink.rail_row,
        name_row,
        rail,
        box_row,
        arc_row,
        arcs,
        edges,
        segments,
        left_elided: start > 0,
        right_elided: end < phases.len() || built[start..end].iter().any(|item| item.truncated),
    }
}

/// The visible phases laid on one rail, in absolute plan columns.
struct Laid {
    segments: Vec<Seg>,
    /// Where the `→` heads sit.
    edges: Vec<usize>,
    /// The inclusive span of the rail line, absent when nothing is drawn.
    rail: Option<(usize, usize)>,
}

/// Lay the window's phases, each built relative to its own origin, one
/// connector apart on the one rail.
fn lay(
    built: &[Built],
    phases: &[Phase],
    connector: usize,
    status: &str,
    cursor: Option<&str>,
) -> Laid {
    let mut segments: Vec<Seg> = Vec::new();
    let mut edges: Vec<usize> = Vec::new();
    let mut rail: Option<(usize, usize)> = None;
    // The first rail node stands far enough in that the leftmost thing
    // the frame draws — the leading name's overhang — clears the
    // elision column and the box's own wall.
    let lead = built.first().map_or(0, |item| item.lead);
    let mut rail_x = 1 + BOX_PAD + lead;
    for (item, phase) in built.iter().zip(phases) {
        // Every gap is the same `connector` columns wide, arrowhead
        // last — the rail's rhythm does not depend on who its
        // neighbours are.
        if let Some((_, previous)) = rail {
            rail_x = previous + 1 + connector;
            edges.push(rail_x - 1);
        }
        edges.extend(item.edges.iter().map(|edge| edge + rail_x));
        let last = rail_x + item.rail_width - 1;
        rail = Some(match rail {
            Some((first, _)) => (first, last),
            None => (rail_x, last),
        });
        // `x0`/`x1` are everything the segment DRAWS: its rail extent
        // and the name hanging off its node, whichever reaches further.
        // The box pads that union, so it breathes evenly by
        // construction rather than by a clamp that could collapse.
        let x0 = rail_x - item.lead;
        segments.push(Seg {
            key: phase.name.clone(),
            rail: (rail_x, last),
            x0,
            x1: last + item.trail,
            name: item.name.clone(),
            // Centred on the rail content, and nothing moves it off:
            // the overhang IS the offset, so the label and its own node
            // cannot round apart.
            name_x: x0 + item.name_off,
            class: phase
                .current
                .then(|| class_for_phase(phase.current, status)),
            selected: cursor == Some(phase.name.as_str()),
            marks: item
                .marks
                .iter()
                .map(|mark| Mark {
                    x: mark.x + rail_x,
                    ..mark.clone()
                })
                .collect(),
            joins: item
                .joins
                .iter()
                .map(|join| Join {
                    x0: join.x0 + rail_x,
                    x1: join.x1 + rail_x,
                    ..join.clone()
                })
                .collect(),
        });
    }
    Laid {
        segments,
        edges,
        rail,
    }
}
