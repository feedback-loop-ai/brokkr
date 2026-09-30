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
    // A frame wide enough holds the selected run beside the list; any
    // other draws the list alone, exactly where it always stood.
    match detail_row(tui, views, area.width) {
        Some(row) => {
            let [list, detail] =
                Layout::horizontal([Constraint::Length(LIST_COLUMNS), Constraint::Min(1)])
                    .areas(area);
            draw_fleet(frame, list, tui, views, tui.pane == 0);
            draw_detail(frame, detail, tui, views, row);
        }
        None => draw_fleet(frame, area, tui, views, true),
    }
}

/// The width of a run id in the list. An id longer than this is
/// shortened in the middle, and its minted hash — the last
/// [`ID_HASH_CHARS`] characters — is never cut: it is the only part
/// that tells two runs of one commission apart.
const ID_COLUMNS: usize = 14;

/// The hash the engine mints at the end of every run id.
const ID_HASH_CHARS: usize = 8;

/// A run id in [`ID_COLUMNS`], its head shortened and its hash whole.
pub(super) fn short_id(id: &str) -> String {
    let count = id.chars().count();
    if count <= ID_COLUMNS {
        return id.to_string();
    }
    let head: String = id.chars().take(ID_COLUMNS - ID_HASH_CHARS - 1).collect();
    let hash: String = id.chars().skip(count - ID_HASH_CHARS).collect();
    format!("{head}…{hash}")
}

/// The fleet list (#491): a heading over each section that lists a run,
/// each run on one row by its title and never by its feature, and one
/// line counting the older runs `a` would show. Every cell is a model
/// field; the cursor's row is kept in view however long the list.
fn draw_fleet(frame: &mut Frame, area: Rect, tui: &Tui, views: &Views, focused: bool) {
    let keys = keys_for(tui, views);
    let cursor = tui.cursor[0].as_deref();
    let (sections, folded) = fleet_sections(tui, views);
    let mut rows: Vec<Row> = Vec::new();
    let mut selected = None;
    for (section, members) in sections {
        let listed: Vec<&RunRow> = members
            .into_iter()
            .filter(|row| keys.contains(&row.run_id))
            .collect();
        if listed.is_empty() {
            continue;
        }
        rows.push(Row::new([cell(section.label(), header_style())]));
        for row in listed {
            let chosen = cursor == Some(row.run_id.as_str());
            if chosen {
                selected = Some(rows.len());
            }
            rows.push(fleet_row(row, &views.now).style(selected_style(chosen)));
        }
    }
    if folded > 0 {
        let count = format!("{folded} hidden · a shows them");
        let dim = Style::new().add_modifier(Modifier::DIM);
        rows.push(Row::new([
            cell(Section::Older.label(), header_style()),
            cell("", plain()),
            cell(&count, dim),
        ]));
    }
    let widths = [
        Constraint::Length(13),
        Constraint::Length(u16::try_from(brokkr_view::VERDICT_COLUMNS).unwrap_or(u16::MAX)),
        Constraint::Min(10),
        Constraint::Length(8),
        Constraint::Length(u16::try_from(ID_COLUMNS).unwrap_or(u16::MAX)),
    ];
    let mut state = TableState::default().with_selected(selected);
    let table = Table::new(rows, widths).block(pane("runs", focused));
    frame.render_stateful_widget(table, area, &mut state);
}

/// One run's row: how it stands (its phase while it runs), its verdict,
/// its title, its age and its id. A run whose journal does not fold
/// keeps its row and its absence marks.
fn fleet_row(row: &RunRow, now: &str) -> Row<'static> {
    let standing = row.verdict.standing;
    let word = match standing {
        Standing::Running => row.phase.as_deref().unwrap_or(brokkr_view::ABSENT),
        Standing::Quarantined
        | Standing::Parked
        | Standing::Shipped
        | Standing::Stopped
        | Standing::OperatorStopped => standing.label(),
    };
    let status = row.status.as_deref().unwrap_or("?");
    let age = brokkr_view::age(&row.created_at, now).unwrap_or(brokkr_view::ABSENT.to_string());
    Row::new([
        cell(
            &format!("{} {word}", standing_glyph(standing)),
            tone_style(status),
        ),
        cell(&row.verdict.text, plain()),
        cell(&row.title, plain()),
        cell(&age, plain()),
        cell(&short_id(&row.run_id), plain()),
    ])
}

/// The one glyph each standing wears, so a row reads without colour.
fn standing_glyph(standing: Standing) -> &'static str {
    match standing {
        Standing::Quarantined => "?",
        Standing::Parked => "●",
        Standing::Running => "▶",
        Standing::Shipped => "✓",
        Standing::Stopped => "✗",
        Standing::OperatorStopped => "■",
    }
}

/// The selected run, whole (#491): its full id; how it stands, where
/// and since when; its verdict and every residual finding; and the full
/// feature, wrapped at [`DETAIL_TEXT_COLUMNS`] and scrolled a line at a
/// time from `offset`.
fn draw_detail(frame: &mut Frame, area: Rect, tui: &Tui, views: &Views, row: &RunRow) {
    let block = pane(&row.title, tui.pane == 1);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let text = Rect {
        width: inner.width.min(DETAIL_TEXT_COLUMNS),
        ..inner
    };
    let lines = detail_lines(row, &views.now, tui.offset);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), text);
}

fn detail_lines(row: &RunRow, now: &str, offset: usize) -> Vec<Line<'static>> {
    let verdict = &row.verdict;
    let absent = brokkr_view::ABSENT;
    let phase = row.phase.as_deref().unwrap_or(absent);
    let age = brokkr_view::age(&row.created_at, now).unwrap_or(absent.to_string());
    let standing = format!("{} · {phase} · {age}", verdict.standing.label());
    let status = row.status.as_deref().unwrap_or("?");
    let text = Some(verdict.text.as_str()).filter(|text| !text.is_empty());
    let mut lines = vec![
        line(&row.run_id, header_style()),
        line(&standing, tone_style(status)),
        line("", plain()),
        line(&format!("verdict   {}", text.unwrap_or(absent)), plain()),
        line(
            &format!("rule      {}", verdict.rule.as_deref().unwrap_or(absent)),
            plain(),
        ),
        line(
            &format!(
                "residual  {}",
                verdict.residual.as_deref().unwrap_or("none")
            ),
            plain(),
        ),
    ];
    let notes = [("parked", &verdict.reason), ("journal", &row.detail)];
    for (label, note) in notes {
        if let Some(note) = note {
            lines.push(line(&format!("{label:<9} {note}"), plain()));
        }
    }
    lines.extend(
        row.residuals
            .iter()
            .map(|finding| line(&finding.line, plain())),
    );
    lines.push(line("", plain()));
    lines.extend(
        row.feature
            .lines()
            .skip(offset)
            .map(|text| line(text, plain())),
    );
    lines
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
