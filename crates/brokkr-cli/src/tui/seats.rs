//! The run level's seats and trail panes.

use super::*;

pub(super) fn draw_seats(
    frame: &mut Frame,
    area: Rect,
    tui: &Tui,
    view: &RunView,
    lens: Option<&render::Lens>,
) {
    let cursor = tui.cursor[1].as_deref();
    let header = Row::new(
        SEAT_COLUMNS
            .iter()
            .map(|name| cell(name, header_style()))
            .collect::<Vec<Cell>>(),
    );
    let kept: Vec<&Participant> = view
        .participants
        .iter()
        .filter(|part| render::keeps_participant(lens, part))
        .collect();
    let texts: Vec<[String; 8]> = kept.iter().map(|part| seat_texts(part)).collect();
    let mut rows: Vec<Row> = Vec::new();
    for (part, fixed) in kept.iter().zip(&texts) {
        // `activity.text` IS the model's composition of `tool` and
        // `target_short` while a seat works, and its result-and-duration
        // once it concludes. The live/concluded distinction is a model
        // field too — `tool` is `Some` exactly while the seat works — so
        // it tints the cell rather than recomposing its text.
        let live = match part.activity.tool {
            Some(_) => tone_style("working"),
            None => plain(),
        };
        // The model and the boundary its hands stood behind, through the
        // one pair helper (decision 0046 ruling 3): two cells, one read.
        let mut cells: Vec<Cell> = fixed
            .iter()
            .enumerate()
            .map(|(index, text)| {
                cell(
                    text,
                    if index == 1 {
                        tone_style(&part.status)
                    } else {
                        plain()
                    },
                )
            })
            .collect();
        cells.push(cell(&part.activity.text, live));
        rows.push(Row::new(cells).style(selected_style(cursor == Some(part.key.as_str()))));
        // Which agent, model and provider actually served this seat
        // (decision 0016). The sentence is the model's; this pane only
        // places it, so a fallback cannot go unmentioned here while it
        // shows elsewhere.
        if let Some(provenance) = &part.provenance {
            // In the widest column, under its seat: the narrow leading
            // columns would clip the sentence, and a clipped honesty
            // rule is not one.
            rows.push(Row::new(vec![
                cell("", plain()),
                cell("", plain()),
                cell("", plain()),
                cell("", plain()),
                cell("", plain()),
                cell("", plain()),
                cell("", plain()),
                cell("", plain()),
                cell(&format!("↳ {}", provenance.line), plain()),
            ]));
        }
    }
    frame.render_widget(
        Table::new(rows, seat_widths(&texts))
            .header(header)
            .block(pane("seats", tui.pane == 1)),
        area,
    );
}

/// The seats pane's columns, in order. The first eight are fitted to
/// their content; the last takes what remains.
pub(super) const SEAT_COLUMNS: [&str; 9] = [
    "participant",
    "status",
    "attempts",
    "turns",
    "cost",
    "tokens",
    "model",
    "boundary",
    "activity",
];

/// No fitted column grows past this, so one runaway label cannot push
/// the activity column off the pane.
pub(super) const SEAT_COLUMN_MAX: usize = 40;

/// A seat's fitted cells, in [`SEAT_COLUMNS`] order minus the activity
/// column. Every text is the model's own; this pane places it.
pub(super) fn seat_texts(part: &Participant) -> [String; 8] {
    let pair = render::served_text(&part.served);
    [
        part.label.clone(),
        part.status.clone(),
        part.attempts.to_string(),
        part.turns_cell.text.clone(),
        part.cost_cell.text.clone(),
        part.usage_cell.text.clone(),
        pair.model.as_str().to_string(),
        pair.boundary.as_str().to_string(),
    ]
}

/// Each fitted column is exactly as wide as its widest cell, header
/// included, and the activity column takes the rest. Fixed widths were
/// the first cut, and a real run clipped them at once — `Σ 1.25M tok`
/// lost its unit at ten columns and `namespace, not applicable` its
/// second half at fourteen — while the activity column beside them held
/// most of a wide screen of air. Fitting is a measurement of what is
/// listed, so no cell is clipped that the pane had room to show, and
/// the provenance line under a seat keeps the widest column it needs.
pub(super) fn seat_widths(texts: &[[String; 8]]) -> Vec<Constraint> {
    let mut widths: Vec<Constraint> = (0..8)
        .map(|index| {
            let widest = texts
                .iter()
                .map(|fixed| width_of(&fixed[index]))
                .chain(std::iter::once(width_of(SEAT_COLUMNS[index])))
                .max()
                .unwrap_or(0)
                .min(SEAT_COLUMN_MAX);
            Constraint::Length(u16::try_from(widest).unwrap_or(u16::MAX))
        })
        .collect();
    widths.push(Constraint::Min(10));
    widths
}

pub(super) fn draw_trail(
    frame: &mut Frame,
    area: Rect,
    tui: &Tui,
    view: &RunView,
    lens: Option<&render::Lens>,
) {
    let cursor = tui.cursor[2].as_deref();
    let lines: Vec<Line> = view
        .journal
        .iter()
        .filter(|row| row.in_trail && render::keeps_row(lens, row))
        .map(|row| {
            let seq = row.seq.to_string();
            let model = render::trail_pair(&render::served_text(&row.served));
            line(
                &format!("{seq}  {}  {}{model}", row.event_type, row.what.text),
                selected_style(cursor == Some(seq.as_str())),
            )
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines).block(pane("trail", tui.pane == 2)),
        area,
    );
}
