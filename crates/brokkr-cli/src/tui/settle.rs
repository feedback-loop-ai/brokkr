//! The console's settlement: resolving the movement the keys parked
//! against the frame that answers it, and clearing selections whose
//! subject left the frame. Split from `state.rs` by responsibility, so the
//! state file stays within its ceiling (decision 0071 ruling 4).

use super::*;

/// Settle the console against the frame that answered: resolve an
/// absolute-end request into the rank the read found, apply the held keys
/// in order against the window it holds, and resolve an end a held `G`
/// asked for (M3, decision 0071 ruling 5).
pub(crate) fn settle_end(tui: &mut Tui, views: &Views) {
    resolve_end(tui, views);
    // The answer's window settles the keys pressed against it, in order;
    // a key that crosses it asks the next page and keeps its place.
    flush_held(tui, views);
    // A held `G` applied above asks the absolute end it now needs.
    resolve_end(tui, views);
}

/// Resolve an absolute-end request into the rank the read answered, and
/// leave every other position where the keys put it.
fn resolve_end(tui: &mut Tui, views: &Views) {
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
    if let Some(end) = end {
        let at = end.saturating_sub(1);
        match tui.level {
            Level::Runs if tui.live_offset == usize::MAX => {
                tui.live_offset = at;
                tui.live_line = last_live_line(tui, views);
            }
            Level::Participant if tui.pane == 0 && tui.offset == usize::MAX => {
                // The logical rank is kept; the pane narrows it for display
                // only, as today's renderer did, so movement from the end
                // stays in the stream's own coordinate (M4, R2).
                tui.offset = at;
            }
            // A page step past a stream whose end the read just discovered
            // resolves to that end's last row, never a blank page beyond
            // it (F3).
            Level::Participant if tui.pane == 0 => {
                tui.offset = tui.offset.min(at);
            }
            _ => {}
        }
    }
}

/// The live column's layout for the held fleet frame: the selected row
/// laid at the column's actual width, when the frame draws the detail.
/// The one lookup the three live-column reads share (L2, ruling 5).
pub(super) fn laid_live(tui: &Tui, views: &Views) -> Option<Laid> {
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

/// Whether the focused pane is a page of a longer stream: the live column
/// at work, or the participant's checkpoint pane. Only those move against
/// a window the console may not yet hold.
pub(super) fn paged_window(tui: &Tui, views: &Views) -> bool {
    match tui.level {
        Level::Participant => tui.pane == 0,
        Level::Runs => {
            focused(tui, views) == FleetColumn::Live
                && laid_live(tui, views).is_some_and(|laid| laid.streamed)
        }
        Level::Run => false,
    }
}

/// Whether `step` lands inside the held window, so it can be applied there
/// exactly. A step that does not is held and asks the page that covers it,
/// the keys behind it waiting (decision 0071 ruling 5).
pub(super) fn can_move(tui: &Tui, views: &Views, step: Step) -> bool {
    match tui.level {
        Level::Runs if paged_window(tui, views) => live_can(tui, views, step),
        // The participant level moves in the stream's own integer
        // coordinate, so a step is exact even when the pane displays a
        // narrowed page: it never needs the window it has not yet read.
        _ => true,
    }
}

pub(super) fn live_can(tui: &Tui, views: &Views, step: Step) -> bool {
    let Some(laid) = laid_live(tui, views) else {
        return true;
    };
    if !laid.streamed || laid.rows.len() < 2 {
        return true;
    }
    let end = laid.rows[laid.rows.len() - 1] as isize;
    let Some(at) = live_position(tui, views, &laid) else {
        return false;
    };
    match step {
        Step::Top | Step::Bottom => true,
        Step::Up => at > 0 || views.start == 0,
        Step::Down => at + 1 < end || !views.more,
        Step::PageUp => at >= PAGE as isize || views.start == 0,
        Step::PageDown => at + (PAGE as isize) < end || !views.more,
    }
}

/// The rank the next read must hold for `step`, asked from the cursor in
/// the direction the step goes: the window that answers holds the cursor
/// and the destination, and no drawn line of either is assumed (ruling 5).
fn ask_want(tui: &Tui, step: Step) -> usize {
    let height = usize::from(tui.height);
    match step {
        Step::Up | Step::PageUp => tui.live_offset.saturating_add(1).saturating_sub(height),
        Step::Down | Step::PageDown => tui
            .live_offset
            .saturating_add(PAGE)
            .saturating_add(1)
            .saturating_sub(height),
        Step::Top => 0,
        Step::Bottom => usize::MAX,
    }
}

/// Hold `step` until the page it needs is answered. A run of the same key
/// is one entry with a count; past [`HELD_BOUND`] runs the oldest is
/// dropped and the status line says so.
pub(crate) fn hold(tui: &mut Tui, step: Step) {
    // Only the head asks a page; a key behind it waits for that answer.
    if !tui.pending && tui.held.is_empty() {
        tui.want = Some(ask_want(tui, step));
    }
    if let Some(last) = tui.held.last_mut() {
        if last.step == step {
            last.count += 1;
            return;
        }
    }
    if tui.held.len() >= HELD_BOUND {
        tui.held.remove(0);
        tui.status = Some("held movement keys dropped: the queue is full".to_string());
    }
    tui.held.push(Held { step, count: 1 });
}

/// Whether the cursor is in the answered window, so the next held key can
/// be applied against it. A move that leaves it asks the next read.
fn in_window(tui: &Tui, views: &Views) -> bool {
    match tui.level {
        Level::Participant if tui.pane == 0 => seat_of(tui, views).is_none_or(|part| {
            tui.offset >= views.start
                && tui.offset < views.start.saturating_add(part.checkpoints.len())
        }),
        Level::Runs => laid_live(tui, views)
            .is_none_or(|laid| !laid.streamed || live_position(tui, views, &laid).is_some()),
        _ => true,
    }
}

/// Apply the held keys, in order, against the answered window. A key that
/// would cross it asks the next page and keeps its place, and every key
/// behind it waits (decision 0071 ruling 5).
pub(super) fn flush_held(tui: &mut Tui, views: &Views) {
    while let Some(&Held { step: next, .. }) = tui.held.first() {
        if !can_move(tui, views, next) {
            tui.want = Some(ask_want(tui, next));
            return;
        }
        step(tui, views, next);
        if let Some(entry) = tui.held.first_mut() {
            entry.count -= 1;
            if entry.count == 0 {
                tui.held.remove(0);
            }
        }
        if !in_window(tui, views) {
            tui.want = None;
            return;
        }
    }
    tui.want = None;
}

/// Selection clears itself when its subject disappears — the rule the
/// console already follows. A **filter** never clears a scope: absence
/// from a filtered list is a display fact, and this runs against the
/// unfiltered models.
#[cfg(test)]
pub(super) fn settle(tui: &mut Tui, views: &Views) {
    settle_when(tui, views, true);
}

/// [`settle`], settling the held movement only when the read answers
/// (`answered`). A read still out is no answer to the keys pressed
/// against the window it will replace, so they wait for it instead of
/// being consumed against the old one and lost (R4, decision 0071 ruling
/// 5).
pub(crate) fn settle_when(tui: &mut Tui, views: &Views, answered: bool) {
    if answered {
        tui.pending = false;
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
