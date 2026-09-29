//! The participant level: the seat's head, its checkpoints, its
//! transcript pane, and the text both transcript doors open onto.

use super::*;

/// The whole turn, composed for the reader: a one-based number in the
/// command's own displayed sequence, a header naming the role and the
/// timestamp, then every block in order — prose in full, tool blocks as
/// the same `⚙ name · target` marker the console shows. Every part is
/// seat-authored, so every part passes through [`safe`].
pub(super) fn turn_text(number: usize, turn: &Turn) -> String {
    let mut text = format!("#{number} {}  {}\n", safe(&turn.role), safe(&turn.ts));
    for block in &turn.blocks {
        text.push('\n');
        if block.kind == BlockKind::Tool {
            text.push_str("⚙ ");
        }
        text.push_str(safe(&block.text).as_str());
    }
    text
}

/// The reader's ordered notices, each sanitized, as one block. They are
/// the reader's own strings, never re-derived here, so the pane and both
/// doors cannot invent a suffix (decision 0055; T2).
pub(super) fn notices_text(read: &TranscriptRead) -> Vec<String> {
    read.notices.iter().map(|notice| safe(notice)).collect()
}

/// The selected turn plus the whole read's notices, so a door onto one
/// turn cannot hide a source cap, a malformed-line count or an
/// unrecognized-record count that the read still carries.
pub(super) fn turn_overlay_text(number: usize, turn: &Turn, read: &TranscriptRead) -> String {
    let mut parts = vec![turn_text(number, turn).trim_end_matches('\n').to_string()];
    parts.extend(notices_text(read));
    parts.join("\n\n")
}

/// The WHOLE transcript, composed for the same reader: every turn in
/// stream order, each composed by [`turn_text`] itself — reused, never
/// re-derived, so the two doors cannot drift — a blank line between
/// turns, then the reader's own notices and full-session line. A readable
/// zero-turn result keeps an openable empty or capped explanation rather
/// than an apparently missing session. Every part is [`safe`] because
/// [`turn_text`] and [`notices_text`] are.
pub(super) fn transcript_text(read: &TranscriptRead) -> String {
    let mut parts: Vec<String> = if read.turns.is_empty() {
        vec![if read.truncated {
            "no readable turns — transcript truncated (size cap)".to_string()
        } else {
            "no readable turns".to_string()
        }]
    } else {
        read.turns
            .iter()
            .enumerate()
            // The separator owns the blank line, so a turn with no blocks
            // cannot smuggle a second one in on its header's newline.
            .map(|(index, turn)| {
                turn_text(index + 1, turn)
                    .trim_end_matches('\n')
                    .to_string()
            })
            .collect()
    };
    parts.extend(notices_text(read));
    if let Some(hint) = &read.full_session {
        parts.push(safe(hint));
    }
    parts.join("\n\n")
}

/// The exact pane keys, one selected-turn door and the whole-transcript
/// door for one shared read, through the same renderers the TUI paints.
/// The 11.2 cross-surface proof compares these against the command and
/// the HTTP body without a second renderer.
#[doc(hidden)]
pub fn transcript_surfaces_for_test(
    read: &TranscriptRead,
    selected: Option<usize>,
) -> (Vec<String>, String, String) {
    let selected = selected
        .filter(|index| *index < read.turns.len())
        .map(|index| turn_overlay_text(index + 1, &read.turns[index], read))
        .unwrap_or_default();
    (turn_keys_of(Some(read)), selected, transcript_text(read))
}

pub(super) fn draw_participant(
    frame: &mut Frame,
    area: Rect,
    tui: &Tui,
    views: &Views,
    part: &Participant,
) {
    let [head, stream, transcript] = Layout::vertical([
        Constraint::Length(8),
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .areas(area);
    frame.render_widget(
        Paragraph::new(head_lines(views, part)).block(pane("seat", false)),
        head,
    );
    frame.render_widget(
        Paragraph::new(checkpoint_lines(part))
            .scroll((offset_for(tui, 0), 0))
            .block(pane("checkpoints", tui.pane == 0)),
        stream,
    );
    let (lines, scroll) = transcript_pane(tui, views);
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((u16::try_from(scroll).unwrap_or(u16::MAX), 0))
            .block(pane("transcript", tui.pane == 1)),
        transcript,
    );
}

/// The seat pane: who the seat is, where its transcript lives, what
/// selected it, and what it claims to have run on.
fn head_lines(views: &Views, part: &Participant) -> Vec<Line<'static>> {
    // The shared full-session value is rendered verbatim when the reader
    // produced one and no line at all when it is null — the TUI never
    // chooses a command or invents a hint of its own.
    let pair = render::served_text(&part.served);
    let mut lines = vec![
        Line::from(vec![
            span(&part.label, header_style()),
            span(" · ", plain()),
            span(&part.status, tone_style(&part.status)),
        ]),
        line(&format!("terminal  {}", part.terminal_line.text), plain()),
        line(
            &format!("transcript  {}", part.transcript_cell.text),
            plain(),
        ),
    ];
    if let Some(hint) = views
        .transcript
        .as_ref()
        .and_then(|read| read.full_session.as_deref())
    {
        lines.push(line(hint, plain()));
    }
    lines.extend([
        line(
            &match &part.provenance {
                Some(provenance) => format!("selected by  {}", provenance.line),
                None => format!("selected by  {}", brokkr_view::ABSENT),
            },
            plain(),
        ),
        // `model` is the provider's CLAIM, not proof (decision 0035
        // ruling 2), and the two efforts beside it are CONFIGURATION —
        // what the plan pinned and what the harness echoed as applied,
        // kept apart because ruling 6 keeps them apart. None of the
        // three measures what the model did; the reasoning count in the
        // tokens line below is the only figure here that does. The
        // boundary beside the model is the plain word its hands stood
        // behind (decision 0046 ruling 3), read through the pair helper.
        line(
            &format!(
                "model     {} (claimed) · boundary {}",
                pair.model.as_str(),
                pair.boundary.as_str()
            ),
            plain(),
        ),
        line(
            &format!(
                "effort    pinned {} · applied {} (configuration)",
                part.effort_pin.text, part.effort.text
            ),
            plain(),
        ),
        line(
            &format!(
                "attempts {} · turns {} · cost {} · tokens {}",
                part.attempts, part.turns_cell.text, part.cost_cell.text, part.usage_cell.text
            ),
            plain(),
        ),
    ]);
    lines
}

/// The checkpoint pane: one line per checkpoint the seat recorded.
fn checkpoint_lines(part: &Participant) -> Vec<Line<'static>> {
    part.checkpoints
        .iter()
        .map(|row| {
            let pair = render::served_text(&row.served);
            line(
                &format!(
                    "{}  {}  model {}  boundary {}  effort {}  tokens {}  {}  {}",
                    row.turn.text,
                    row.step,
                    pair.model.as_str(),
                    pair.boundary.as_str(),
                    row.effort.text,
                    row.usage.text,
                    row.target.text,
                    row.recorded_at
                ),
                plain(),
            )
        })
        .collect()
}

/// The transcript pane's lines and the scroll that keeps the selected
/// turn in view.
fn transcript_pane(tui: &Tui, views: &Views) -> (Vec<Line<'static>>, usize) {
    // A refused or unavailable read replaces the previous prose
    // atomically: the pane carries the selected reference, the reason, the
    // explanation, the retained path and the counts, and neither door is
    // active. Dispatching on `read.unavailable` removes the former
    // `is_readable` guard and its reason-less arm: a readable read is
    // exactly one whose `unavailable` is `None`.
    match &views.transcript {
        Some(read) => match read.unavailable {
            None => transcript_lines(read, selected_turn(tui, views).map(|(index, _)| index)),
            Some(reason) => (refused_lines(read, reason), 0),
        },
        None => (
            vec![line(
                "no local session transcript on this machine — the transcript line above names it",
                plain(),
            )],
            0,
        ),
    }
}

/// The scroll of the focused pane only: the other pane keeps its top.
pub(super) fn offset_for(tui: &Tui, pane: usize) -> u16 {
    match tui.pane == pane {
        true => u16::try_from(tui.offset).unwrap_or(u16::MAX),
        false => 0,
    }
}

/// Transcript prose is arbitrary text from outside the store, so it goes
/// through `Safe` like everything else, and every notice the reader
/// produced is **shown**: silently short evidence is worse than none
/// (decision 0001). The selected turn wears the same mark as every
/// selected row, and the returned scroll is that turn's own first line,
/// so the pane follows the cursor rather than holding a second offset
/// that could drift.
pub(super) fn transcript_lines(
    read: &TranscriptRead,
    selected: Option<usize>,
) -> (Vec<Line<'static>>, usize) {
    let mut lines: Vec<Line> = Vec::new();
    let mut scroll = 0usize;
    for (index, turn) in read.turns.iter().enumerate() {
        let picked = selected == Some(index);
        if picked {
            scroll = lines.len();
        }
        lines.push(line(
            &format!("#{} {} · {}", index + 1, turn.role, turn.ts),
            header_style().patch(selected_style(picked)),
        ));
        for block in &turn.blocks {
            lines.push(line(&format!("  {}", block.text), selected_style(picked)));
        }
    }
    if read.turns.is_empty() {
        let explanation = if read.truncated {
            "no readable turns — transcript truncated (size cap)"
        } else {
            "no readable turns"
        };
        lines.push(line(explanation, plain()));
    }
    for notice in &read.notices {
        lines.push(line(notice, header_style()));
    }
    (lines, scroll)
}

/// An unavailable read's own lines: the reference it kept, the reader's
/// closed reason token, its explanation, the retained path and hint, its
/// source-cap/count notices and its two counts. Nothing here is derived
/// from a previous frame, so no stale prose can survive a refusal.
pub(super) fn refused_lines(read: &TranscriptRead, reason: Unavailable) -> Vec<Line<'static>> {
    let reference = match &read.reference {
        Some(reference) => format!(
            "{} · {} · {}",
            reference.kind, reference.locator, reference.home
        ),
        None => "—".to_string(),
    };
    let mut lines = vec![line(&format!("reference  {reference}"), plain())];
    lines.push(line(
        &format!("reason     {}", reason.as_str()),
        header_style(),
    ));
    if let Some(explanation) = &read.explanation {
        lines.push(line(explanation, plain()));
    }
    lines.push(line(
        &format!("path       {}", read.path.as_deref().unwrap_or("—")),
        plain(),
    ));
    for notice in &read.notices {
        lines.push(line(notice, plain()));
    }
    if let Some(hint) = &read.full_session {
        lines.push(line(hint, plain()));
    }
    lines.push(line(
        &format!(
            "counts     skipped {} · unrecognized {}",
            read.skipped_lines, read.unrecognized_records
        ),
        plain(),
    ));
    lines
}
