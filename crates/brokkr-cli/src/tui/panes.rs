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
            &footer_within(tui, views, usize::from(footer.width)),
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
    // other draws the list alone, exactly where it always stood. Wide
    // enough is the width the shell measured for this frame, the one the
    // footer and the keys read too.
    match detail_row(tui, views) {
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

/// The fleet list (#491, #503): a heading over each section that lists
/// a run, each run by its title and never by its feature, and one line
/// under the list saying how many older runs `a` would show. Every cell
/// is a model field; the cursor's row is kept in view however long the
/// list. The title column is as wide as the widest line it draws, up to
/// what the list leaves it, so the age and the id stand beside the
/// titles however wide the frame.
fn draw_fleet(frame: &mut Frame, area: Rect, tui: &Tui, views: &Views, focused: bool) {
    let block = pane("runs", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let keys = keys_for(tui, views);
    let cursor = tui.cursor[0].as_deref();
    let (sections, folded) = fleet_sections(tui, views);
    let mut listed: Vec<Drawn> = Vec::new();
    for (section, members) in sections {
        let members: Vec<_> = members
            .into_iter()
            .filter(|row| keys.contains(&row.run_id))
            .map(|row| (row, title_lines(tui, views, row)))
            .collect();
        if !members.is_empty() {
            listed.push((section, members));
        }
    }
    let mut widths = fleet_widths(area.width);
    let drawn = listed.iter().flat_map(|(_, members)| members);
    let widest = drawn
        .flat_map(|(_, lines)| lines)
        .map(|text| width_of(text));
    let widest = u16::try_from(widest.max().unwrap_or(0)).unwrap_or(u16::MAX);
    widths[3] = widest.clamp(TITLE_MIN_COLUMNS, widths[3].max(TITLE_MIN_COLUMNS));
    let columns = usize::from(widths[3]);
    let mut rows: Vec<Row> = Vec::new();
    let mut selected = None;
    let mut height = 0u16;
    for (section, members) in listed {
        rows.push(heading(tui, section));
        height += 1;
        for (row, lines) in members {
            let chosen = cursor == Some(row.run_id.as_str());
            if chosen {
                selected = Some(rows.len());
            }
            let lines = lines
                .iter()
                .map(|text| brokkr_view::title_within(text, columns));
            let lines: Vec<String> = lines.collect();
            height += lines.len() as u16;
            rows.push(fleet_row(row, &views.now, &lines).style(selected_style(chosen)));
        }
    }
    let count = u16::from(folded > 0);
    let [list, below] = Layout::vertical([
        Constraint::Length(height.min(inner.height.saturating_sub(count))),
        Constraint::Length(count),
    ])
    .areas(inner);
    let mut state = TableState::default().with_selected(selected);
    let table = Table::new(rows, widths.map(Constraint::Length));
    frame.render_stateful_widget(table, list, &mut state);
    let older = format!("{folded} older runs: press a to show");
    frame.render_widget(Paragraph::new(line(&older, header_style())), below);
}

/// A section the list draws, each of its runs with its title cell's
/// lines.
type Drawn<'a> = (Section, Vec<(&'a RunRow, Vec<String>)>);

/// A section's heading: its name, and over the older runs `a` showed,
/// in the verdict's column, the key that folds them again (#503).
fn heading(tui: &Tui, section: Section) -> Row<'static> {
    let fold = match section == Section::Older && tui.all {
        true => "a folds them",
        false => "",
    };
    let dim = Style::new().add_modifier(Modifier::DIM);
    Row::new([cell(section.label(), header_style()), cell(fold, dim)])
}

/// The fleet list's column widths in a list `width` wide (#491). A
/// column is drawn whole or not at all: a list too narrow for every
/// column and a title of [`TITLE_MIN_COLUMNS`] folds the age away, then
/// the residual, so the standing, the verdict and the id's hash are
/// whole from [`MIN_WIDTH`] up. The title's entry is what the rest
/// leaves it: a whole title in the list the detail pane stands beside,
/// which is exactly that wide.
fn fleet_widths(width: u16) -> [u16; 6] {
    let [standing, verdict, mut residual, _, mut age, id] = FLEET_COLUMNS;
    let fixed = |residual: u16, age: u16| 2 + 5 + standing + verdict + residual + age + id;
    if fixed(residual, age) + TITLE_MIN_COLUMNS > width {
        age = 0;
    }
    if fixed(residual, age) + TITLE_MIN_COLUMNS > width {
        residual = 0;
    }
    let title = width.saturating_sub(fixed(residual, age));
    [standing, verdict, residual, title, age, id]
}

/// What a run's title cell draws, sanitized and whole, for the list to
/// measure and clamp: its title, what it needs of the operator (#503),
/// and, opened in place, where a running run stands.
fn title_lines(tui: &Tui, views: &Views, row: &RunRow) -> Vec<String> {
    let mut lines = vec![brokkr_view::title_within(&row.feature, usize::MAX)];
    lines.extend(brokkr_view::need(row, &views.now).and_then(|need| need.prompt()));
    if opened(tui, row) {
        lines.extend(opened_lines(row, &views.now));
    }
    lines.iter().map(|text| safe(text)).collect()
}

/// A running run opened in place (#503): its phase, the seat at work
/// and which attempt, how long it has run, and when its journal last
/// moved.
fn opened_lines(row: &RunRow, now: &str) -> [String; 2] {
    let absent = brokkr_view::ABSENT;
    let phase = row.phase.as_deref().unwrap_or(absent);
    let (seat, attempt) = match &row.hire {
        Some(hire) => (hire.seat.as_str(), hire.attempt.to_string()),
        None => (absent, absent.to_string()),
    };
    let elapsed = brokkr_view::age(&row.created_at, now).unwrap_or(absent.to_string());
    let moved = row.last_recorded_at.as_deref();
    let last = match (moved.and_then(|at| brokkr_view::age(at, now)), row.seq) {
        (Some(ago), Some(seq)) => format!("{ago} ago, seq {seq}"),
        _ => absent.to_string(),
    };
    [
        format!("phase {phase} · seat {seat} · attempt {attempt}"),
        format!("elapsed {elapsed} · last event {last}"),
    ]
}

/// One run's row: how it stands (its phase while it runs), its verdict,
/// its worst open residual, its title lines, its age and its id. A run
/// whose journal does not fold keeps its row and its absence marks.
fn fleet_row(row: &RunRow, now: &str, lines: &[String]) -> Row<'static> {
    let need = brokkr_view::need(row, now);
    let word = standing_word(row, need.as_ref());
    let age = brokkr_view::age(&row.created_at, now).unwrap_or(brokkr_view::ABSENT.to_string());
    Row::new([
        cell(
            &format!("{} {word}", glyph_of(row.verdict.standing, need.as_ref())),
            tone_of(row, need.as_ref()),
        ),
        cell(&row.verdict.text, plain()),
        cell(row.verdict.residual.map_or("", Severity::name), plain()),
        cell_lines(lines, plain()),
        cell(&age, plain()),
        cell(&short_id(&row.run_id), plain()),
    ])
    .height(lines.len() as u16)
}

/// The run's title as the TUI paints it in `columns`: sanitized, then
/// clamped again, so the columns it measures are the columns it draws.
fn painted_title(row: &RunRow, columns: usize) -> String {
    let first = brokkr_view::title_within(&row.feature, columns);
    brokkr_view::title_within(&safe(&first), columns)
}

/// The one glyph each standing wears, so a row reads without colour. A
/// running run's is not a disclosure triangle (#503): nothing about it
/// says it opens. A stale one wears the alarm, never the running mark.
fn glyph_of(standing: Standing, need: Option<&Need>) -> &'static str {
    if let Some(Need::Stale { .. }) = need {
        return "!";
    }
    match standing {
        Standing::Quarantined => "?",
        Standing::Parked => "●",
        Standing::Running => "◐",
        Standing::Shipped => "✓",
        Standing::Stopped => "✗",
        Standing::OperatorStopped => "■",
    }
}

/// The word a row's standing cell prints: a stale run's need, a running
/// run's phase, and every other standing's own word.
fn standing_word<'a>(row: &'a RunRow, need: Option<&Need>) -> &'a str {
    match (need, row.verdict.standing) {
        (Some(stale @ Need::Stale { .. }), _) => stale.label(),
        (Some(Need::Parked | Need::Quarantined) | None, Standing::Running) => {
            row.phase.as_deref().unwrap_or(brokkr_view::ABSENT)
        }
        (
            Some(Need::Parked | Need::Quarantined) | None,
            Standing::Quarantined
            | Standing::Parked
            | Standing::Shipped
            | Standing::Stopped
            | Standing::OperatorStopped,
        ) => row.verdict.standing.label(),
    }
}

/// The tone a run's standing wears: its status's, except that a stale
/// run is quiet, never live.
fn tone_of(row: &RunRow, need: Option<&Need>) -> Style {
    let status = match need {
        Some(stale @ Need::Stale { .. }) => stale.label(),
        Some(Need::Parked | Need::Quarantined) | None => row.status.as_deref().unwrap_or("?"),
    };
    tone_style(status)
}

/// The selected run, whole (#491): its full id; how it stands, where
/// and since when; its verdict and every residual finding; and the full
/// feature. The pane scrolls from `offset` by the lines it draws.
fn draw_detail(frame: &mut Frame, area: Rect, tui: &Tui, views: &Views, row: &RunRow) {
    let block = pane(
        &painted_title(row, brokkr_view::TITLE_COLUMNS),
        tui.pane == 1,
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let lines: Vec<Line> = detail_lines(row, &views.now)
        .into_iter()
        .skip(tui.offset)
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Every line the detail pane holds, each already wrapped at
/// [`DETAIL_TEXT_COLUMNS`]: what the pane draws and what its scroll
/// counts are one list. A run that needs the operator names the way out
/// after its notes (#503).
pub(super) fn detail_lines(row: &RunRow, now: &str) -> Vec<Line<'static>> {
    let verdict = &row.verdict;
    let absent = brokkr_view::ABSENT;
    let need = brokkr_view::need(row, now);
    let phase = row.phase.as_deref().unwrap_or(absent);
    let age = brokkr_view::age(&row.created_at, now).unwrap_or(absent.to_string());
    let stands = match &need {
        Some(stale @ Need::Stale { .. }) => stale.label(),
        Some(Need::Parked | Need::Quarantined) | None => verdict.standing.label(),
    };
    let standing = format!("{stands} · {phase} · {age}");
    let text = Some(verdict.text.as_str()).filter(|text| !text.is_empty());
    let rule = verdict.rule.as_deref().unwrap_or(absent);
    let residual = verdict.residual.map_or("none", Severity::name);
    let mut texts = vec![
        (row.run_id.clone(), header_style()),
        (standing, tone_of(row, need.as_ref())),
        (String::new(), plain()),
        (format!("verdict   {}", text.unwrap_or(absent)), plain()),
        (format!("rule      {rule}"), plain()),
        (format!("residual  {residual}"), plain()),
    ];
    let notes = [("parked", &verdict.reason), ("journal", &row.detail)];
    for (label, note) in notes {
        if let Some(note) = note {
            texts.push((format!("{label:<9} {note}"), plain()));
        }
    }
    let way_out = need.map(|need| need.way_out(&row.run_id));
    texts.extend(way_out.into_iter().flatten().map(|text| (text, plain())));
    let findings = row.residuals.iter().map(|finding| finding.line.clone());
    texts.extend(findings.map(|text| (text, plain())));
    texts.push((String::new(), plain()));
    texts.extend(row.feature.lines().map(|text| (text.to_string(), plain())));
    let columns = usize::from(DETAIL_TEXT_COLUMNS);
    texts
        .into_iter()
        .flat_map(|(text, style)| {
            // Sanitized before it is measured, so a line is as wide as
            // what it draws; a blank line stays one line.
            let parts = brokkr_view::wrap(&safe(&text), columns);
            let blank = parts.is_empty().then(String::new);
            parts
                .into_iter()
                .chain(blank)
                .map(move |part| line(&part, style))
        })
        .collect()
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
