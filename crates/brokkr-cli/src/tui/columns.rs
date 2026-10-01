//! The fleet's columns beside its list (#503): which of the run
//! dashboard (`d`) and the live or findings column (`f`) a frame holds,
//! how they share its whole width, and what each draws about the selected
//! run. Every fact is `brokkr-view`'s (decision 0013); this lays it out,
//! each column wrapping its text at its own width.

use super::*;

/// A column of the fleet frame, left to right.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FleetColumn {
    List,
    Dashboard,
    Live,
}

/// The columns beside the list the operator has asked for, `d` and `f`:
/// both, until a key says otherwise, wherever the width holds them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Toggles {
    pub dashboard: bool,
    pub live: bool,
}

impl Default for Toggles {
    fn default() -> Toggles {
        Toggles {
            dashboard: true,
            live: true,
        }
    }
}

/// The narrowest frame that holds the list beside the dashboard.
pub(super) const DASHBOARD_FROM: u16 = LIST_COLUMNS + DASHBOARD_MIN;

/// The narrowest frame that holds the list beside the live column alone.
pub(super) const LIVE_FROM: u16 = LIST_COLUMNS + LIVE_MIN;

/// The narrowest frame that holds all three.
pub(super) const THREE_FROM: u16 = LIST_COLUMNS + DASHBOARD_MIN + LIVE_MIN;

/// `share` hundredths of `width`.
fn part(width: u16, share: u32) -> u16 {
    u16::try_from(u32::from(width) * share / 100).unwrap_or(u16::MAX)
}

/// The columns a frame `width` wide draws under `toggles`, each with its
/// width, sharing the whole frame: about 35/35/30 for three and 45/55 for
/// two, the list never narrower than a whole title. A column the width
/// cannot hold is not drawn; the live column gives way to the dashboard.
pub(super) fn layout(toggles: Toggles, width: u16) -> Vec<(FleetColumn, u16)> {
    let list = |share| part(width, share).max(LIST_COLUMNS);
    match (toggles.dashboard, toggles.live) {
        (true, true) if width >= THREE_FROM => {
            let list = list(35);
            let rest = width - list;
            let dashboard = u16::try_from(u32::from(rest) * 35 / 65).unwrap_or(rest);
            vec![
                (FleetColumn::List, list),
                (FleetColumn::Dashboard, dashboard),
                (FleetColumn::Live, rest - dashboard),
            ]
        }
        (true, _) if width >= DASHBOARD_FROM => {
            let list = list(45);
            vec![
                (FleetColumn::List, list),
                (FleetColumn::Dashboard, width - list),
            ]
        }
        (false, true) if width >= LIVE_FROM => {
            let list = list(45);
            vec![(FleetColumn::List, list), (FleetColumn::Live, width - list)]
        }
        _ => vec![(FleetColumn::List, width)],
    }
}

/// The width the live column needs under `toggles`: beside the dashboard
/// when that is asked for too, else beside the list alone.
pub(super) fn live_from(toggles: Toggles) -> u16 {
    match toggles.dashboard {
        true => THREE_FROM,
        false => LIVE_FROM,
    }
}

/// The fleet frame's columns as the shell measured it: the list alone
/// until a run is selected for the others to show.
pub(super) fn fleet_columns(tui: &Tui, views: &Views) -> Vec<(FleetColumn, u16)> {
    match selected_run(tui, views) {
        Some(_) => layout(tui.toggles, tui.width),
        None => vec![(FleetColumn::List, tui.width)],
    }
}

/// The fleet column the keys move: the one `Tab` focused, or the list
/// once a narrowed frame or a toggle took that one away.
pub(super) fn focused(tui: &Tui, views: &Views) -> FleetColumn {
    let columns = fleet_columns(tui, views);
    columns
        .get(tui.pane)
        .map_or(FleetColumn::List, |(column, _)| *column)
}

/// The text width inside a column `width` wide.
pub(super) fn inside(width: u16) -> usize {
    usize::from(width.saturating_sub(2))
}

/// The width of the fleet column `column` on this frame, inside its
/// borders, or `None` when it is not on it.
pub(super) fn width_of_column(tui: &Tui, views: &Views, column: FleetColumn) -> Option<usize> {
    let columns = fleet_columns(tui, views);
    let found = columns.into_iter().find(|(shown, _)| *shown == column);
    found.map(|(_, width)| inside(width))
}

/// What the third column is called for `row`: what its seat is doing
/// while it runs, and what it found once it has stopped.
pub(super) fn third(row: Option<&RunRow>) -> &'static str {
    match row.map(|row| row.verdict.standing) {
        Some(Standing::Running) | None => "live",
        Some(
            Standing::Quarantined
            | Standing::Parked
            | Standing::Shipped
            | Standing::Stopped
            | Standing::OperatorStopped,
        ) => "findings",
    }
}

/// What a column is called in the footer.
pub(super) fn name_of(column: FleetColumn, row: Option<&RunRow>) -> &'static str {
    match column {
        FleetColumn::List => "list",
        FleetColumn::Dashboard => "dashboard",
        FleetColumn::Live => third(row),
    }
}

/// The selected run's view, when the frame holds the view of that run:
/// the shell reads it a frame behind a new selection, and a view of
/// another run is never painted as this one's.
pub(super) fn view_of<'a>(views: &'a Views, row: &RunRow) -> Option<&'a RunView> {
    let view = views.run.as_ref()?;
    (view.dashboard.run_id.as_deref() == Some(row.run_id.as_str())).then_some(view)
}

/// The selected run's last seat's notes, whole, when its view holds any.
pub(super) fn notes_of(views: &Views, row: &RunRow) -> Option<(String, String)> {
    let seat = view_of(views, row)?.dashboard.last_seat.as_ref()?;
    Some((seat.seat.clone(), seat.notes.clone()?))
}

/// Lines to draw: each text with its style, before it is wrapped.
type Texts = Vec<(String, Style)>;

/// `texts` wrapped at `columns`, sanitized before they are measured so a
/// line is as wide as what it draws; a blank line stays one line.
pub(super) fn wrapped(texts: Texts, columns: usize) -> Vec<Line<'static>> {
    texts
        .into_iter()
        .flat_map(|(text, style)| {
            let parts = brokkr_view::wrap(&safe(&text), columns);
            let blank = parts.is_empty().then(String::new);
            parts
                .into_iter()
                .chain(blank)
                .map(move |part| line(&part, style))
        })
        .collect()
}

/// A heading over a dashboard section, after a blank line.
fn heading_of(title: &str) -> Texts {
    vec![
        (String::new(), plain()),
        (title.to_string(), header_style()),
    ]
}

/// `text`'s first `count` lines at `columns`, and how many it has. Each
/// of its own lines is sanitized apart, so the sanitizer never joins
/// two, and a blank one stays one line.
fn first_lines(text: &str, columns: usize, count: usize) -> (Texts, usize) {
    let lines: Vec<String> = text
        .lines()
        .flat_map(|line| match brokkr_view::wrap(&safe(line), columns) {
            parts if parts.is_empty() => vec![String::new()],
            parts => parts,
        })
        .collect();
    let total = lines.len();
    let kept = lines.into_iter().take(count).map(|text| (text, plain()));
    (kept.collect(), total)
}

/// The lines the dashboard folds a long text to.
const FOLDED_LINES: usize = 3;

/// Every line the dashboard holds for `row`, wrapped at `columns`: what
/// it draws and what its scroll counts are one list (#503). How the run
/// stands; why it ended or what it needs; its path, its seats and its way
/// out; and its commission, folded to its first lines until `c`.
pub(super) fn dashboard_lines(
    tui: &Tui,
    views: &Views,
    row: &RunRow,
    columns: usize,
) -> Vec<Line<'static>> {
    let view = view_of(views, row);
    let need = brokkr_view::need(row, &views.now);
    let mut texts = standing_texts(row, need.as_ref(), &views.now, columns);
    texts.extend(ending_texts(row, view, need.as_ref(), columns));
    texts.extend(heading_of("PATH"));
    let path = view.map(|view| view.dashboard.path_text());
    texts.push((
        path.filter(|path| !path.is_empty()).unwrap_or(absent()),
        plain(),
    ));
    texts.extend(heading_of("SEATS"));
    let seats: Vec<String> = view
        .map(|view| view.participants.iter().map(brokkr_view::seat_summary))
        .into_iter()
        .flatten()
        .collect();
    texts.extend(or_absent(seats).into_iter().map(|text| (text, plain())));
    texts.extend(heading_of("WAY OUT"));
    let way_out = need
        .map(|need| need.way_out(&row.run_id))
        .unwrap_or_default();
    texts.extend(or_absent(way_out).into_iter().map(|text| (text, plain())));
    texts.extend(heading_of("COMMISSION"));
    texts.extend(commission(tui, row, columns));
    wrapped(texts, columns)
}

fn absent() -> String {
    brokkr_view::ABSENT.to_string()
}

/// `lines`, or the absence mark where there are none.
fn or_absent(lines: Vec<String>) -> Vec<String> {
    match lines.is_empty() {
        true => vec![absent()],
        false => lines,
    }
}

/// The title clamped to the column, the full id, and standing · phase ·
/// age. The commission below holds the title's whole line.
fn standing_texts(row: &RunRow, need: Option<&Need>, now: &str, columns: usize) -> Texts {
    let phase = row.phase.as_deref().unwrap_or(brokkr_view::ABSENT);
    let age = brokkr_view::age(&row.created_at, now).unwrap_or(absent());
    let stands = match need {
        Some(stale @ Need::Stale { .. }) => stale.label(),
        Some(Need::Parked | Need::Quarantined) | None => row.verdict.standing.label(),
    };
    vec![
        (
            brokkr_view::title_within(&listed_title(row), columns),
            header_style(),
        ),
        (row.run_id.clone(), plain()),
        (format!("{stands} · {phase} · {age}"), tone_of(row, need)),
    ]
}

/// Why the run ended, or what it needs, or where it stands: its verdict,
/// its last ruling's rule, severity and reason, its last seat's result
/// and its notes' first lines, and its residual findings.
fn ending_texts(
    row: &RunRow,
    view: Option<&RunView>,
    need: Option<&Need>,
    columns: usize,
) -> Texts {
    let title = match (need, row.verdict.standing) {
        (Some(_), _) => "WHAT IT NEEDS",
        (None, Standing::Running) => "WHERE IT STANDS",
        (
            None,
            Standing::Quarantined
            | Standing::Parked
            | Standing::Shipped
            | Standing::Stopped
            | Standing::OperatorStopped,
        ) => "WHY IT ENDED",
    };
    let verdict = &row.verdict;
    let text = Some(verdict.text.as_str()).filter(|text| !text.is_empty());
    let residual = verdict.residual.map_or("none", Severity::name);
    let mut texts = heading_of(title);
    let absent = brokkr_view::ABSENT;
    texts.push((
        format!("verdict   {} · residual {residual}", text.unwrap_or(absent)),
        plain(),
    ));
    let board = view.map(|view| &view.dashboard);
    let decision = board.and_then(|board| board.decision.as_ref());
    texts.push((rule_line(verdict, decision), plain()));
    let reason = decision.and_then(|ruled| ruled.problem.as_ref());
    if let Some(reason) = reason.or(verdict.reason.as_ref()) {
        texts.push((format!("reason    {reason}"), plain()));
    }
    let facts = [("journal", row.detail.clone()), ("at work", hired(row))];
    for (label, fact) in facts {
        texts.extend(fact.map(|fact| (format!("{label:<9} {fact}"), plain())));
    }
    texts.extend(seat_texts(
        board.and_then(|board| board.last_seat.as_ref()),
        columns,
    ));
    let findings = row.residuals.iter().map(|finding| finding.line.clone());
    texts.extend(findings.map(|text| (text, plain())));
    texts
}

/// The last ruling: its rule, its severity and where it routed, or the
/// verdict's rule where the journal is not read.
fn rule_line(verdict: &brokkr_view::Verdict, decision: Option<&brokkr_view::Decision>) -> String {
    let absent = brokkr_view::ABSENT;
    let Some(ruled) = decision else {
        return format!("rule      {}", verdict.rule.as_deref().unwrap_or(absent));
    };
    let severity = ruled
        .severity
        .map_or(absent, brokkr_view::RuleSeverity::label);
    let from = ruled.from.as_deref().unwrap_or(absent);
    let next = ruled.next.as_deref().unwrap_or(absent);
    format!("rule      {} · {severity} · {from} → {next}", ruled.rule)
}

/// The seat at work on a running run and its attempt, as the fold names it.
fn hired(row: &RunRow) -> Option<String> {
    let hire = row.hire.as_ref()?;
    Some(format!("seat {} · attempt {}", hire.seat, hire.attempt))
}

/// The last seat's result, and its notes' first lines with the key that
/// reads them whole.
fn seat_texts(seat: Option<&brokkr_view::Concluded>, columns: usize) -> Texts {
    let Some(seat) = seat else {
        return vec![(format!("seat      {}", brokkr_view::ABSENT), plain())];
    };
    let result = seat.result.as_deref().unwrap_or(brokkr_view::ABSENT);
    let said = format!(
        "seat      {} · {} · result {result}",
        seat.seat,
        seat.outcome.label()
    );
    let mut texts = vec![(said, plain())];
    let Some(notes) = &seat.notes else {
        texts.push((format!("notes     {}", brokkr_view::ABSENT), plain()));
        return texts;
    };
    let (first, total) = first_lines(notes, columns, FOLDED_LINES);
    let shown = first.len();
    let how = match total > shown {
        true => format!("notes     {shown} of {total} lines · Enter reads them all"),
        false => "notes".to_string(),
    };
    texts.push((how, plain()));
    texts.extend(first);
    texts
}

/// The commission, folded to its first lines until `c` opens it.
fn commission(tui: &Tui, row: &RunRow, columns: usize) -> Texts {
    let count = match tui.commission {
        true => usize::MAX,
        false => FOLDED_LINES,
    };
    let (mut texts, total) = first_lines(&row.feature, columns, count);
    let fold = match (tui.commission, total > FOLDED_LINES) {
        (_, false) => None,
        (true, true) => Some("c folds it".to_string()),
        (false, true) => Some(format!(
            "… {} more lines · c shows them",
            total - FOLDED_LINES
        )),
    };
    texts.extend(fold.map(|text| (text, header_style())));
    texts
}

/// The live or findings column for `row`: its title and its lines,
/// wrapped at `columns`. A running run streams the checkpoints of the
/// seat the fold names, newest first; a finished one shows its last
/// review's findings, or else its last seat's notes, whole.
pub(super) fn live_column(
    views: &Views,
    row: &RunRow,
    columns: usize,
) -> (String, Vec<Line<'static>>) {
    let view = view_of(views, row);
    let (title, texts) = match row.verdict.standing {
        Standing::Running => streamed(view, row),
        Standing::Quarantined
        | Standing::Parked
        | Standing::Shipped
        | Standing::Stopped
        | Standing::OperatorStopped => found(view, row),
    };
    (title, wrapped(texts, columns))
}

/// What the line says where the frame holds no view of the run yet.
const UNREAD: &str = "— the run's journal is not read";

fn streamed(view: Option<&RunView>, row: &RunRow) -> (String, Texts) {
    let Some(hire) = &row.hire else {
        let none = "— no seat at work: the fold names none".to_string();
        return ("live".to_string(), vec![(none, plain())]);
    };
    let title = format!("live · {}", hire.seat);
    let Some(view) = view else {
        return (title, vec![(UNREAD.to_string(), plain())]);
    };
    let rows = brokkr_view::working_checkpoints(view, &hire.seat);
    let lines = rows.into_iter().map(|(label, row)| {
        let text = format!(
            "{}  {label} · turn {} · {} · {}",
            row.recorded_at, row.turn.text, row.step, row.target.text
        );
        (text, plain())
    });
    let mut texts: Texts = lines.collect();
    if texts.is_empty() {
        texts.push((format!("— no checkpoint from {} yet", hire.seat), plain()));
    }
    (title, texts)
}

fn found(view: Option<&RunView>, row: &RunRow) -> (String, Texts) {
    let Some(view) = view else {
        return ("findings".to_string(), vec![(UNREAD.to_string(), plain())]);
    };
    let board = &view.dashboard;
    // A review's notes are the run's findings, read beside the residuals
    // its ruling recorded; any other seat's are its notes.
    let (kind, seat, residuals) = match (&board.last_review, &board.last_seat) {
        (Some(review), _) => ("findings", review, row.residuals.as_slice()),
        (None, Some(seat)) => ("notes", seat, [].as_slice()),
        (None, None) => {
            let none = "— no seat has concluded".to_string();
            return ("findings".to_string(), vec![(none, plain())]);
        }
    };
    let result = seat.result.as_deref().unwrap_or(brokkr_view::ABSENT);
    let mut texts = vec![(
        format!("{} · {} · result {result}", seat.seat, seat.outcome.label()),
        header_style(),
    )];
    let findings = residuals.iter().map(|finding| finding.line.clone());
    texts.extend(findings.map(|text| (text, plain())));
    texts.push((String::new(), plain()));
    let notes = seat.notes.as_deref().unwrap_or("— no notes recorded");
    texts.extend(notes.lines().map(|text| (text.to_string(), plain())));
    (format!("{kind} · {}", seat.seat), texts)
}

/// The dashboard column, scrolled to `tui.offset`.
pub(super) fn draw_dashboard(
    frame: &mut Frame,
    area: Rect,
    tui: &Tui,
    views: &Views,
    row: &RunRow,
) {
    let focused = focused(tui, views) == FleetColumn::Dashboard;
    let block = pane("dashboard", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let lines = dashboard_lines(tui, views, row, usize::from(inner.width));
    let lines: Vec<Line> = lines.into_iter().skip(tui.offset).collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

/// The live or findings column, scrolled to `tui.live_offset`.
pub(super) fn draw_live(frame: &mut Frame, area: Rect, tui: &Tui, views: &Views, row: &RunRow) {
    let inner = Block::default().borders(Borders::ALL).inner(area);
    let (title, lines) = live_column(views, row, usize::from(inner.width));
    let block = pane(&title, focused(tui, views) == FleetColumn::Live);
    frame.render_widget(block, area);
    let lines: Vec<Line> = lines.into_iter().skip(tui.live_offset).collect();
    frame.render_widget(Paragraph::new(lines), inner);
}
