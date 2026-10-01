//! The graph's painter: it walks a [`Plan`] and writes cells through the
//! one sanitized constructor, and computes no geometry of its own.

use super::*;

/// One frame's cells. A cell holding `None` is the second column of a
/// double-width glyph, already paid for by its neighbour — which is why
/// every width here is ratatui's own measurement rather than a count.
pub(super) type Cells = Vec<Vec<Option<(String, Style)>>>;

/// Write sanitized text at a cell position. Bounded by the grid itself
/// rather than by a branch: the plan fits, and the zip is what makes
/// that structural instead of asserted.
pub(super) fn put(cells: &mut Cells, x: usize, row: usize, text: &str, style: Style) {
    let glyphs: Vec<Option<(String, Style)>> = safe(text)
        .chars()
        .flat_map(|character| {
            let glyph = character.to_string();
            let wide = width_of(&glyph) > 1;
            let mut pair = vec![Some((glyph, style))];
            pair.resize(1 + usize::from(wide), None);
            pair
        })
        .collect();
    for line in cells.iter_mut().skip(row).take(1) {
        for (cell, glyph) in line.iter_mut().skip(x).zip(glyphs.iter()) {
            cell.clone_from(glyph);
        }
    }
}

/// What is already painted at a cell — bounded by the grid the same way
/// `put` is, by iteration rather than by a branch, so an off-grid read
/// answers with the empty string instead of a panic or an `Option` the
/// caller would have to unwrap.
pub(super) fn under(cells: &Cells, x: usize, row: usize) -> String {
    cells
        .iter()
        .skip(row)
        .take(1)
        .flat_map(|line| line.iter().skip(x).take(1))
        .flatten()
        .map(|(glyph, _)| glyph.as_str())
        .collect()
}

/// The spine at the fork and at the rejoin, drawn top to bottom in one
/// pass so no glyph can overwrite another's arms: a corner at the
/// outermost lane, the tee the rail wears at the trunk, and a tee for
/// every lane in between.
pub(super) fn spine(cells: &mut Cells, join: &Join, rail_row: usize) {
    let up = join
        .rows
        .iter()
        .copied()
        .min()
        .unwrap_or(rail_row)
        .min(rail_row);
    let down = join
        .rows
        .iter()
        .copied()
        .max()
        .unwrap_or(rail_row)
        .max(rail_row);
    for row in up..=down {
        let (left, right) = match (row == up, row == down, row == rail_row, join.on_rail) {
            (true, _, _, _) => ("┌", "┐"),
            (_, true, _, _) => ("└", "┘"),
            (_, _, true, true) => ("┼", "┼"),
            (_, _, true, false) => ("┤", "├"),
            _ => ("├", "┤"),
        };
        put(cells, join.x0, row, left, plain());
        put(cells, join.x1, row, right, plain());
    }
}

/// The SOLID box around the selected phase — the terminal's answer to
/// the console's selection ring. The vocabulary is `─ │ ╭ ╮ ╰ ╯ ┼` and
/// nothing else: the dashed set `╌ ┆` left the graph by the operator's
/// ruling of 2026-08-31, because those glyphs are drawn with a gap at
/// every cell boundary BY DESIGN, so no amount of correct geometry can
/// make their edges touch the corners in any font. The geometry was
/// never the fault; the vocabulary was.
///
/// Lower edge on the reserved box row, walls on EVERY row between the
/// corners — and where a wall crosses the rail, the cell is `┼`, a true
/// junction, so the wall and the rail both read continuous through it.
/// That satisfies the unbroken-rail ruling rather than fighting it. A
/// pane too short to reserve the box row draws none: half a box is two
/// floating lines, which is what this replaced.
pub(super) fn selection_box(cells: &mut Cells, seg: &Seg, plan: &Plan) {
    let Some(bottom) = plan.box_row else {
        return;
    };
    // The box spans the lane envelope OF THIS PHASE — one row above the
    // topmost row the phase actually occupies. A plain phase gets a
    // snug box with no empty `│    │` air rows; a fork's grows to hold
    // its own lanes. The shape is a function of the selected phase,
    // never of the pane, so moving the selection between two plain
    // phases cannot change it (operator's ruling, superseding the fixed
    // full-envelope box that made every plain phase wear a fork's).
    let top = seg
        .marks
        .iter()
        .map(|mark| mark.row)
        .chain(seg.joins.iter().flat_map(|join| join.rows.iter().copied()))
        .min()
        .unwrap_or(plan.rail_row)
        .saturating_sub(1);
    // `x0`/`x1` are already everything the segment draws, so padding
    // them by the same `BOX_PAD` is even breathing BY CONSTRUCTION: a
    // name can never sit flush against one wall while the other floats.
    // Two columns out also puts every wall clear of the arrowheads —
    // the head that lands on this phase's node is one column off its
    // rail content, and so stays inside the boundary it points into.
    // A wall does still CROSS the rail's dashes at the rail row, and
    // deliberately: the rail is one unbroken line by an earlier ruling,
    // so the only alternative is a hole in the track at every
    // selection. Crossing a line is a boundary; standing on an arrow's
    // point was the accident.
    let (left, right) = (seg.x0.saturating_sub(BOX_PAD), seg.x1 + BOX_PAD);
    for x in left..=right {
        put(cells, x, top, "─", plain());
        put(cells, x, bottom, "─", plain());
    }
    for row in top + 1..bottom {
        for column in [left, right] {
            // A junction may only replace a DASH: where the wall crosses
            // live rail the cell becomes `┼` and both lines read through
            // it, but a wall standing where there is no rail — the outer
            // side of the first or the last phase — is a plain wall.
            // Reading the cell rather than trusting `plan.rail` is what
            // keeps the junction honest about what is actually painted.
            let wall = match (row == plan.rail_row, under(cells, column, row) == "─") {
                (true, true) => "┼",
                _ => "│",
            };
            put(cells, column, row, wall, plain());
        }
    }
    put(cells, left, top, "╭", plain());
    put(cells, right, top, "╮", plain());
    put(cells, left, bottom, "╰", plain());
    put(cells, right, bottom, "╯", plain());
}

/// The road back. A reforging is a road, and a road is drawn: without
/// this the rail says `review` finished and `implement` lit up again,
/// and an operator watching a live run reads teleportation instead of a
/// loop.
///
/// The vocabulary is the SOLID set — `╰ ─ ╯` from the same rounded
/// corners the selection box uses, plus the mirror head `ᐸ` (U+1438),
/// the sibling of the rail's own operator-calibrated `ᐳ`. Never the
/// dashed set `╌ ┆`: those glyphs are drawn with a gap at every cell
/// boundary BY DESIGN, so no amount of correct geometry makes them
/// touch a corner — the operator's ruling of 2026-08-31, learned once
/// on the box and applied here from the start rather than after.
///
/// The corners rise toward the phases they belong to, so the arc reads
/// as one road leaving the rail and returning to it. Only the LANDING
/// wears a head, matching the rail's own asymmetry: `ᐳ` marks arrival,
/// never departure. The head sits inside the landing corner, pointing
/// into it.
pub(super) fn arc(cells: &mut Cells, arc: &Arc, row: usize) {
    for x in arc.to..=arc.from {
        put(cells, x, row, "─", plain());
    }
    put(cells, arc.to, row, "╰", plain());
    put(cells, arc.from, row, "╯", plain());
    put(cells, arc.to + 1, row, "ᐸ", plain());
}

/// A fork and its rejoin. **The rejoin is drawn always** — it is the
/// join dependency, and the whole reason a fork is not two steps.
pub(super) fn fork(cells: &mut Cells, join: &Join, rail_row: usize) {
    // Between the fork and its rejoin the rail gives way to the lanes,
    // unless the member count is odd and one member rides the rail row.
    // A labelled fork parts the rail to make room for its name; an
    // UNLABELLED one (a panel with no step name) must not leave a
    // gap — a rail that stops for eighteen columns reads as broken
    // track, not as parallelism.
    let filler = match join.label {
        Some(_) => " ",
        None => "─",
    };
    for x in join.x0 + 1..join.x1 {
        put(cells, x, rail_row, filler, plain());
    }
    for row in &join.rows {
        for x in join.x0 + 1..join.x1 {
            put(cells, x, *row, "─", plain());
        }
    }
    spine(cells, join, rail_row);
    if let Some(label) = &join.label {
        let inner = join.x1 - join.x0 - 3;
        let x = join.x0 + 2 + inner.saturating_sub(width_of(label)) / 2;
        put(cells, x, rail_row, label, plain());
    }
}

/// Adjacent cells sharing a style become one span, and a span is the
/// ONE sanitized constructor — so the graph opens no third path into a
/// buffer and the discipline test stays unamended.
pub(super) fn row_line(row: &[Option<(String, Style)>]) -> Line<'static> {
    let mut runs: Vec<(String, Style)> = Vec::new();
    for (glyph, style) in row.iter().flatten() {
        match runs.last_mut() {
            Some((text, last)) if last == style => text.push_str(glyph),
            _ => runs.push((glyph.clone(), *style)),
        }
    }
    Line::from(
        runs.iter()
            .map(|(text, style)| span(text, *style))
            .collect::<Vec<Span<'static>>>(),
    )
}

/// One phase's nodes, their labels, its selection box and its name, in
/// that order, so the box and the name are drawn over the rail.
fn paint_seg(cells: &mut Cells, seg: &Seg, plan: &Plan, tick: usize, animate: bool) {
    for mark in &seg.marks {
        let (style, ramp) = look(mark.class);
        let glyph = ramp[pulse(tick, mark.live, animate)];
        put(cells, mark.x, mark.row, glyph, style);
        if !mark.label.is_empty() {
            // One clear cell between a node and its own text, so the
            // rail does not read as part of the word.
            put(cells, mark.x + 1, mark.row, " ", plain());
            put(
                cells,
                mark.x + 2,
                mark.row,
                &mark.label,
                selected_style(mark.selected),
            );
        }
    }
    // The selected phase sits in a solid box — the terminal's
    // answer to the console's selection ring. It hugs the rows THIS
    // phase occupies and pads what it draws evenly on both sides,
    // and where a wall crosses the rail it wears the junction `┼`,
    // so the rail is SEEN to pass through the boundary rather than
    // stopping at it — an arrow head is never overwritten.
    if seg.selected {
        selection_box(cells, seg, plan);
    }
    let current = match seg.class {
        Some(class) => look(class).0,
        None => plain(),
    };
    put(
        cells,
        seg.name_x,
        plan.name_row,
        &seg.name,
        current.patch(selected_style(seg.selected)),
    );
}

/// The painter: it walks the plan and writes cells, and computes no
/// geometry of its own. Selection is `REVERSED` on the name or the node
/// label — the TUI's existing selection idiom, identical to the runs
/// table and the trail; current is the filled, coloured rail glyph.
/// Different attribute, different cell, different axis.
pub(super) fn paint(plan: &Plan, tick: usize, animate: bool) -> Vec<Line<'static>> {
    let blank = Some((" ".to_string(), plain()));
    let mut cells: Cells = vec![vec![blank; plan.width]; plan.rows];
    // One rail, from the first segment's first node to the last one's.
    if let Some((from, to)) = plan.rail {
        for x in from..=to {
            put(&mut cells, x, plan.rail_row, "─", plain());
        }
    }
    for seg in &plan.segments {
        for join in &seg.joins {
            fork(&mut cells, join, plan.rail_row);
        }
    }
    for head in &plan.edges {
        // `ᐳ` (U+1433), chosen BY THE OPERATOR'S EYE from a rendered
        // specimen of seven candidates: `→`'s stem falls short of the
        // box-drawing stroke, `►` and `>` sit a pixel below the cell
        // midline in the operator's font, and the syllabics glyph is
        // the one whose point sits dead-centre on the dash axis.
        // Runners-up, recorded for the next font that disagrees: `⟩`
        // (close second), and box-drawing `╼`, which cannot misalign
        // by construction but reads as a heavy tip, not a point.
        put(&mut cells, *head, plan.rail_row, "ᐳ", plain());
    }
    for seg in &plan.segments {
        paint_seg(&mut cells, seg, plan, tick, animate);
    }
    // The roads back run on their own reserved row, under the names and
    // under the box's lower edge: nothing else is drawn there, so the
    // arcs overwrite nothing and are overwritten by nothing.
    if let Some(row) = plan.arc_row {
        for road in &plan.arcs {
            arc(&mut cells, road, row);
        }
    }
    if plan.left_elided {
        put(&mut cells, 0, plan.rail_row, "‹", plain());
    }
    if plan.right_elided {
        let x = plan.width.saturating_sub(1);
        put(&mut cells, x, plan.rail_row, "›", plain());
    }
    cells.iter().map(|row| row_line(row)).collect()
}

/// Where the rail's cursor stands on a graph: the selected phase, and
/// the lane node inside it.
pub(super) struct Rail<'a> {
    pub(super) phase: Option<&'a str>,
    pub(super) node: Option<&'a str>,
}

/// The graph of `view` in `cells`, columns by rows: the run-level
/// notices, then the rail planned into whatever rows remain. The run
/// level's graph pane and the fleet's dashboard (#508) both draw these
/// lines, so one picture is drawn at two widths by one renderer
/// (decision 0013).
pub(super) fn graph_lines(
    tui: &Tui,
    view: &RunView,
    lens: Option<&render::Lens>,
    rail: &Rail,
    (width, height): (usize, usize),
) -> Vec<Line<'static>> {
    // Run-level notices first — a fallback selection or an optional
    // capability gap is a fact an operator must SEE, not find (decision
    // 0016) — then the graph, planned into whatever rows remain.
    let mut lines: Vec<Line<'static>> = Vec::new();
    // The run header's boundary line (decision 0046 ruling 3): the
    // model's rendered text, adjective included, printed and never
    // composed. A run that boxes nothing carries no line at all.
    if !view.boundary.text.is_empty() {
        lines.push(line(&format!("boundary  {}", view.boundary.text), plain()));
    }
    for notice in &view.notices {
        lines.push(line(
            &format!("note  {} — {}", notice.kind, notice.text),
            tone_style("working"),
        ));
    }
    let status = view
        .summary
        .as_ref()
        .map_or("", |summary| summary.status.as_str());
    let plan = plan(
        &view.phases,
        lens,
        status,
        rail.phase,
        rail.node,
        width,
        height.saturating_sub(lines.len()),
    );
    lines.extend(paint(&plan, tui.ticks, tui.animate));
    lines
}

pub(super) fn draw_graph(
    frame: &mut Frame,
    area: Rect,
    tui: &Tui,
    views: &Views,
    view: &RunView,
    lens: Option<&render::Lens>,
) {
    let rail = Rail {
        phase: tui.cursor[0].as_deref(),
        node: tui.node.as_deref(),
    };
    let cells = (
        usize::from(area.width.saturating_sub(2)),
        usize::from(area.height.saturating_sub(2)),
    );
    let lines = graph_lines(tui, view, lens, &rail, cells);
    frame.render_widget(
        Paragraph::new(lines).block(
            pane("graph", tui.pane == 0)
                .title_top(brand(fleet_live(views), tui.ticks, tui.animate).right_aligned()),
        ),
        area,
    );
}
