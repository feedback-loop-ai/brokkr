//! The frame and the fleet and run panes: the layout of one frame, the
//! reader and help overlays, the tab bar, the runs table, and how the
//! run level deals its rows between its three panes.

use super::*;

pub(crate) fn draw(frame: &mut Frame, tui: &Tui, views: &Views) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small(frame, area);
        return;
    }
    let [body, status, footer] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    match (tui.level, views.run.as_ref(), seat_of(tui, views)) {
        (Level::Participant, _, Some(part)) => draw_participant(frame, body, tui, views, part),
        (Level::Run, Some(view), _) => draw_run(frame, body, tui, views, view),
        _ => draw_runs(frame, body, tui, views),
    }
    // The forging beacon — the terminal's answer to the console's
    // pulsing favicon. Whenever ANY run in the fleet is live it pulses
    // in the status line's right corner, at every level, so an
    // operator browsing an old run still knows the machine is at work.
    // It rides the fleet the shell already refreshes and the tick the
    // pulse already uses: no store read, no extra poll.
    // The corner 'forging' text retired when the brand mark landed on
    // the graph pane (operator's ruling): ONE forging signal, the
    // logo's pulsing rail node, visible at every level.
    frame.render_widget(Paragraph::new(line(&status_line(tui), plain())), status);
    frame.render_widget(
        Paragraph::new(line(
            &footer_for(tui, views),
            Style::new().add_modifier(Modifier::REVERSED),
        )),
        footer,
    );
    if let Some(text) = &tui.reading {
        draw_reader(frame, area, text, tui.read_offset);
    }
    if tui.help {
        draw_help(frame, area);
    }
}

/// A row opened for reading: the full text, wrapped and scrollable, over
/// the whole frame. Every line is already sanitized at the call site
/// that filled `reading`.
pub(super) fn draw_reader(frame: &mut Frame, area: Rect, text: &str, offset: usize) {
    let lines: Vec<Line> = text.lines().map(|text| line(text, plain())).collect();
    // Clamp so scrolling past the end cannot leave an empty frame with
    // no way back.
    let last = lines.len().saturating_sub(1);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((u16::try_from(offset.min(last)).unwrap_or(u16::MAX), 0))
            .block(pane("row · Esc closes", true)),
        area,
    );
}

/// A terminal resized below the minimum draws this, and keeps its
/// session: an operator dragging a window edge does not lose their
/// place, and no frame is ever corrupted.
pub(super) fn draw_too_small(frame: &mut Frame, area: Rect) {
    let lines = vec![
        line("this terminal is too small for brokkr tui", header_style()),
        line(
            "try `brokkr inspect --run <id>` or `brokkr watch --run <id>`,",
            plain(),
        ),
        line("or make the window bigger. q or Ctrl+C quits.", plain()),
    ];
    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(lines), area);
}

pub(super) fn draw_help(frame: &mut Frame, area: Rect) {
    let lines: Vec<Line> = HELP.iter().map(|text| line(text, plain())).collect();
    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(lines).block(pane("help", true)), area);
}

/// The runs pane's tab bar: one entry per hearth, numbered so the number
/// keys are discoverable, the active one marked. Drawn only in a world
/// that has more than one hearth — see [`draw_runs`].
pub(super) fn tab_bar(tui: &Tui) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (index, label) in tui.tabs.iter().enumerate() {
        let style = match index == tui.tab {
            true => Style::new().add_modifier(Modifier::BOLD | Modifier::REVERSED),
            false => Style::new().add_modifier(Modifier::DIM),
        };
        // Through the same sanitizer every other operator-written string
        // on this frame crosses. A realm name is already held to a closed
        // charset by the loader, so nothing here changes today — which is
        // exactly why it must not be the one string that skips the gate.
        spans.push(span(
            &format!(" {} {} ", index + 1, safe(label).as_str()),
            style,
        ));
        spans.push(span(" ", plain()));
    }
    Line::from(spans)
}

pub(super) fn draw_runs(frame: &mut Frame, area: Rect, tui: &Tui, views: &Views) {
    // A one-hearth world draws no bar and loses no row to one: the whole
    // area is the table, exactly as it was.
    let area = match tabbed(tui) {
        false => area,
        true => {
            let [bar, rest] =
                Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(area);
            frame.render_widget(Paragraph::new(tab_bar(tui)), bar);
            rest
        }
    };
    let keys = keys_for(tui, views);
    let cursor = tui.cursor[0].as_deref();
    let header = Row::new(
        ["id", "status", "phase", "seq", "age", "feature"]
            .iter()
            .map(|name| cell(name, header_style()))
            .collect::<Vec<Cell>>(),
    );
    let mut rows: Vec<Row> = Vec::new();
    for row in &views.runs.runs {
        if !keys.iter().any(|key| key == &row.run_id) {
            continue;
        }
        // Every cell is a model field; the absence marks are the
        // model's, and a run whose journal does not fold keeps its row.
        let status = match &row.status {
            Some(status) => status.clone(),
            None => "?".to_string(),
        };
        let phase = match &row.phase {
            Some(phase) => phase.clone(),
            None => "-".to_string(),
        };
        let seq = match row.seq {
            Some(seq) => seq.to_string(),
            None => "-".to_string(),
        };
        let age = match brokkr_view::age(&row.created_at, &views.now) {
            Some(age) => age,
            None => brokkr_view::ABSENT.to_string(),
        };
        rows.push(
            Row::new(vec![
                cell(&row.run_id, plain()),
                cell(&status, tone_style(&status)),
                cell(&phase, plain()),
                cell(&seq, plain()),
                cell(&age, plain()),
                cell(&row.feature, plain()),
            ])
            .style(selected_style(cursor == Some(row.run_id.as_str()))),
        );
    }
    let widths = [
        Constraint::Length(24),
        Constraint::Length(9),
        Constraint::Length(12),
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Min(10),
    ];
    frame.render_widget(
        Table::new(rows, widths)
            .header(header)
            .block(pane("runs", true)),
        area,
    );
}

pub(super) fn draw_run(frame: &mut Frame, area: Rect, tui: &Tui, views: &Views, view: &RunView) {
    let lens = lens_of(tui, views);
    let [graph, seats, trail] =
        Layout::vertical(estate(view, lens.as_ref(), usize::from(area.height))).areas(area);
    draw_graph(frame, graph, tui, views, view, lens.as_ref());
    draw_seats(frame, seats, tui, view, lens.as_ref());
    draw_trail(frame, trail, tui, view, lens.as_ref());
}

/// The graph may take at most this share of the run level, however
/// deep its forks: the seats and the trail are where a run is read.
pub(super) const GRAPH_SHARE_MAX: usize = 45;

/// How the run level's rows are dealt between its three panes: each
/// pane gets the rows its content needs, and the trail gets the rest.
///
/// Three equal thirds was the first cut, and it read badly the moment
/// a real run was under it: a one-lane rail floated in a third of the
/// screen of air while the seats below it clipped at the bottom and
/// the trail showed a dozen of its hundreds of rows. The graph's need
/// is exact — [`rows_wanted`] is the same arithmetic [`plan`] deals
/// its rows by — and the seats' need is one row per line it lists.
/// Both are capped, the graph at [`GRAPH_SHARE_MAX`] and the seats at
/// half of what the graph leaves, so a deep fork or a long roster
/// degrades the way the thirds did rather than starving the trail.
pub(super) fn estate(
    view: &RunView,
    lens: Option<&render::Lens>,
    height: usize,
) -> [Constraint; 3] {
    let head = usize::from(!view.boundary.text.is_empty()) + view.notices.len();
    let graph = (2 + head + rows_wanted(&view.phases)).min(height * GRAPH_SHARE_MAX / 100);
    let seats = (3 + seat_lines(view, lens)).min(height.saturating_sub(graph) / 2);
    [
        Constraint::Length(u16::try_from(graph).unwrap_or(u16::MAX)),
        Constraint::Length(u16::try_from(seats).unwrap_or(u16::MAX)),
        Constraint::Min(0),
    ]
}

/// The lines the seats pane lists under its header: one per seat the
/// lens keeps, and one more under each seat that carries a provenance
/// sentence.
pub(super) fn seat_lines(view: &RunView, lens: Option<&render::Lens>) -> usize {
    view.participants
        .iter()
        .filter(|part| render::keeps_participant(lens, part))
        .map(|part| 1 + usize::from(part.provenance.is_some()))
        .sum()
}
