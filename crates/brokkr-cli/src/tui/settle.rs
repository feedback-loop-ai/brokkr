//! The console's settlement: resolving the movement the keys parked
//! against the frame that answers it, and clearing selections whose
//! subject left the frame. Split from `state.rs` by responsibility, so the
//! state file stays within its ceiling (decision 0071 ruling 4).

use super::*;

/// Resolve the end key's request into the rank the read found: `G` asks
/// for the stream's absolute end, the read fetches its last window and
/// says how deep the stream is, and the pane then scrolls at that rank
/// (M3). The focused participant offset is kept when the pane is not the
/// one displayed.
pub(crate) fn settle_end(tui: &mut Tui, views: &Views) {
    // A `PageUp` across a window asks for the drawn lines above it,
    // including those still owed when it began inside the window; the
    // read now holds the rows above, so resolve the whole request at the
    // column's actual width (R1, R3).
    if let Some(cross) = tui.live_cross.take() {
        settle_live_page(tui, views, cross);
    }
    // A move that crossed into a wrapped row asked for its foot, not its
    // first continuation: the read now holds that row, so resolve the
    // request at the column's actual width (R5, decision 0071 ruling 5).
    if tui.live_line == usize::MAX {
        tui.live_line = last_live_line(tui, views);
    }
    // A read through the pages answers the stream's end; a whole read in
    // the same coordinate answers it from the laid column, so an absolute
    // end request on either wraps to the same row (R1).
    let end = match views.end {
        Some(end) => Some(end),
        None if tui.level == Level::Runs && tui.live_offset == usize::MAX => {
            held_live_end(tui, views)
        }
        None => None,
    };
    let Some(end) = end else {
        return;
    };
    let at = end.saturating_sub(1);
    match tui.level {
        Level::Runs if tui.live_offset == usize::MAX => {
            tui.live_offset = at;
            tui.live_line = last_live_line(tui, views);
        }
        Level::Participant if tui.pane == 0 && tui.offset == usize::MAX => {
            // The logical rank is kept; the pane narrows it for display
            // only, as today's renderer did, so movement from the end stays
            // in the stream's own coordinate (M4, R2).
            tui.offset = at;
        }
        // A page step past a stream whose end the read just discovered
        // resolves to that end's last row, never a blank page beyond it
        // (F3).
        Level::Participant if tui.pane == 0 => {
            tui.offset = tui.offset.min(at);
        }
        _ => {}
    }
}

/// The live column's layout for the held fleet frame: the selected row
/// laid at the column's actual width, when the frame draws the detail.
/// The one lookup the three live-column reads share (L2, ruling 5).
fn laid_live(tui: &Tui, views: &Views) -> Option<Laid> {
    let row = detail_row(tui, views)?;
    let width = width_of_column(tui, views, FleetColumn::Live).unwrap_or_default();
    Some(live_layout(views, row, width))
}

/// The rank one past the live column's last held checkpoint row: the
/// whole read's stream end, from the laid lines the column paints.
fn held_live_end(tui: &Tui, views: &Views) -> Option<usize> {
    let laid = laid_live(tui, views)?;
    if !laid.streamed {
        return None;
    }
    Some(views.start + laid.rows.len().saturating_sub(1))
}

/// Resolve a step that crossed the held window: `cross.row` is the stream
/// row the move anchored at, and `cross.lines` is how many drawn lines
/// above (negative) or below (positive) that row's first line it named.
/// The read holds the rows around the anchor, so the whole request is laid
/// at the column's actual width and translated once back into a row and a
/// line within it (R1, decision 0071 ruling 5). The drawn-line destination
/// is bounded to the last line the read holds before it is translated, so
/// a step past a discovered end lands on that end's last line instead of
/// a line the stream never drew (R6).
fn settle_live_page(tui: &mut Tui, views: &Views, cross: LiveCross) {
    let Some(laid) = laid_live(tui, views) else {
        return;
    };
    if !laid.streamed || laid.rows.len() < 2 {
        return;
    }
    let rows = laid.rows.len() - 1;
    let drawn = laid.rows[rows];
    if drawn == 0 {
        return;
    }
    let anchor = cross.row.saturating_sub(views.start).min(rows);
    let at = (laid.rows[anchor] as isize)
        .saturating_add(cross.lines)
        .clamp(0, drawn.saturating_sub(1) as isize) as usize;
    place_live(tui, views, &laid, at);
}

/// Write `at`, a drawn line in the held window, as the row and the line
/// within it the live pane scrolls at: the one translation both a move
/// within the window and a step resolved after its read make (ruling 5).
pub(super) fn place_live(tui: &mut Tui, views: &Views, laid: &Laid, at: usize) {
    let rows = laid.rows.len() - 1;
    let row = laid
        .rows
        .partition_point(|start| *start <= at)
        .saturating_sub(1)
        .min(rows - 1);
    tui.live_offset = views.start + row;
    tui.live_line = at - laid.rows[row];
}

/// The last drawn line of the live column's checkpoint row at
/// `tui.live_offset`, at the column's actual width: the end key reaches
/// the row's foot, and a step up across a window reaches the preceding
/// wrapped row's last line, not its first continuation (M2, R5).
fn last_live_line(tui: &Tui, views: &Views) -> usize {
    let Some(laid) = laid_live(tui, views) else {
        return 0;
    };
    if !laid.streamed {
        return 0;
    }
    let rows = laid.rows.len().saturating_sub(1);
    if rows == 0 {
        return 0;
    }
    let at = tui.live_offset.saturating_sub(views.start).min(rows - 1);
    laid.rows[at + 1]
        .saturating_sub(laid.rows[at])
        .saturating_sub(1)
}

/// Selection clears itself when its subject disappears — the rule the
/// console already follows. A **filter** never clears a scope: absence
/// from a filtered list is a display fact, and this runs against the
/// unfiltered models.
#[cfg(test)]
pub(super) fn settle(tui: &mut Tui, views: &Views) {
    settle_when(tui, views, true);
}

/// [`settle`], resolving the movement a page request parked only when the
/// held frame answers that request (`answered`). A read still out is no
/// answer to the keys pressed against the window it will replace, so a
/// crossing waits for its page instead of being consumed against the old
/// one and lost (R4, decision 0071 ruling 5).
pub(crate) fn settle_when(tui: &mut Tui, views: &Views, answered: bool) {
    if answered {
        settle_end(tui, views);
    }
    if lens_of(tui, views).is_none() {
        tui.scope = None;
    }
    seed_cursor(tui, views);
    if seat_of(tui, views).is_none() {
        tui.seat = None;
        if tui.level == Level::Participant {
            tui.level = Level::Run;
            tui.pane = 1;
        }
    }
    // The fleet's rows are a window of pages read by keyset (decision 0089
    // ruling 6), and a writer moves a run out of it, so a run missing from
    // them has not gone on that account. Only the run level reads the open
    // run, and there the frame says what it found: a view of it, or, when
    // the store refused it, its row in the store's words (ruling 5). A
    // fleet that lists runs while the frame holds neither has lost it. An
    // empty fleet is the unreadable-journal frame, which is not evidence
    // that the run went away.
    let vanished = match &tui.run {
        Some(run) if tui.level != Level::Runs && views.run.is_none() => {
            !views.runs.runs.is_empty() && !views.runs.runs.iter().any(|row| key_of(row) == run)
        }
        Some(_) | None => false,
    };
    if vanished {
        tui.run = None;
        tui.level = Level::Runs;
        tui.pane = 0;
        tui.scope = None;
        tui.seat = None;
        tui.cursor = [None, None, None];
    }
}
