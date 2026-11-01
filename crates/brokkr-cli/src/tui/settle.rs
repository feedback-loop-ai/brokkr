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
    // A queue pressed on another view is dropped first: its head's
    // unresolved move is cancelled, not resolved against a frame it was
    // never pressed on (M1).
    if tui.held_for.as_ref() != Some(&held_view(tui, views)) {
        drop_held(tui);
    } else {
        // An unresolved marker a previous answer left is resolved against
        // this answer first. Resolution consumes the held head that owns
        // the marker, so the flush below starts from that head's one
        // application instead of repeating it (M1). A direct `G` owns its
        // own marker and consumes nothing.
        resolve_end(tui, views);
        // The answer's window settles the keys pressed against it, in
        // order; a key that crosses it asks the next page and keeps its
        // place.
        flush_held(tui, views);
    }
    // A directly applied `G` keeps its marker until its own answer; a
    // cancelled or consumed held move has already left the cursor where
    // the operator put it.
    resolve_end(tui, views);
}

/// Resolve an absolute-end marker into the rank the read answered, and
/// leave every other position where the keys put it. The resolution is
/// the marker owner's one application: a held head that asked for the end
/// leaves the queue here, while a direct `G` owns its own marker and
/// consumes nothing. Every path that resolves a marker comes through this
/// one function (M1, decision 0071 ruling 5).
fn resolve_end(tui: &mut Tui, views: &Views) -> bool {
    // A read through the pages answers the stream's end; a whole read in
    // the same coordinate answers it from the laid column, so an absolute
    // end request on either wraps to the same row (R1).
    let end = match views.end {
        Some(end) => Some(end),
        None if tui.level == Level::Runs && tui.live_offset == usize::MAX && !views.more => {
            held_live_end(tui, views)
        }
        None => None,
    };
    let Some(end) = end else {
        return false;
    };
    let at = end.saturating_sub(1);
    let resolved = match tui.level {
        Level::Runs if tui.live_offset == usize::MAX => {
            tui.live_offset = at;
            tui.live_line = last_live_line(tui, views);
            true
        }
        Level::Participant if tui.pane == 0 && tui.offset == usize::MAX => {
            // The logical rank is kept; the pane narrows it for display
            // only, as today's renderer did, so movement from the end
            // stays in the stream's own coordinate (M4, R2).
            tui.offset = at;
            true
        }
        // A page step past a stream whose end the read just discovered
        // resolves to that end's last row, never a blank page beyond
        // it (F3).
        Level::Participant if tui.pane == 0 => {
            tui.offset = tui.offset.min(at);
            false
        }
        _ => false,
    };
    if !resolved {
        return false;
    }
    // The held head whose marker this is has now had its one application.
    // A direct `G` set its marker with no held head, so nothing leaves the
    // queue (M1).
    if tui.held.first().is_some_and(|held| held.end.is_some()) {
        consumed(tui);
    }
    true
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
    last_line_of(&laid, tui, views)
}

/// [`last_live_line`] over a laid window: the last drawn line of the row
/// the cursor is on, none for a screen that is not a stream or a window
/// with no row.
pub(super) fn last_line_of(laid: &Laid, tui: &Tui, views: &Views) -> usize {
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
        Level::Runs if focused(tui, views) == FleetColumn::Live => match laid_live(tui, views) {
            Some(laid) if laid.streamed => live_can(tui, views, &laid, step),
            _ => true,
        },
        Level::Runs => true,
        // The participant level moves in the stream's own integer
        // coordinate, so a step is exact even when the pane displays a
        // narrowed page: it never needs the window it has not yet read.
        Level::Run | Level::Participant => true,
    }
}

pub(super) fn live_can(tui: &Tui, views: &Views, laid: &Laid, step: Step) -> bool {
    if laid.rows.len() < 2 {
        return true;
    }
    let end = laid.rows[laid.rows.len() - 1] as isize;
    // The top and the end are the stream's, not the held window's: they
    // need no saved continuation, and a resize that removed one must not
    // hold `g` or `G` (M3). Every other step needs the cursor's place in
    // the laid window and refuses to move without it.
    let at = live_position(tui, views, laid);
    match step {
        Step::Top | Step::Bottom => true,
        Step::Up => at.is_some_and(|at| at > 0 || views.start == 0),
        Step::Down => at.is_some_and(|at| at + 1 < end || !views.more),
        Step::PageUp => at.is_some_and(|at| at >= PAGE as isize || views.start == 0),
        Step::PageDown => at.is_some_and(|at| at + (PAGE as isize) < end || !views.more),
    }
}

/// The rank the next read must hold for `step`, asked from the cursor in
/// the direction the step goes: the window that answers holds the cursor
/// and the destination, and no drawn line of either is assumed (ruling 5).
pub(super) fn ask_want(tui: &Tui, step: Step) -> usize {
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
/// dropped, with any unresolved end application it still owns, and the
/// status line says so.
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
        // The oldest run leaves with the cursor change it still owns:
        // cancel it first, so the replacement head is not the one an
        // answer consumes, and no resolved marker is left behind (M1).
        if let Some(origin) = tui.held.remove(0).end {
            cancel_end(tui, origin);
            tui.want = None;
        }
        tui.status = Some("held movement keys dropped: the queue is full".to_string());
    }
    tui.held.push(Held {
        step,
        count: 1,
        end: None,
    });
}

/// The cursor a held absolute-end application replaces: the position the
/// operator left, so cancelling the run can put it back.
fn held_origin(tui: &Tui) -> HeldOrigin {
    match tui.level {
        Level::Runs => HeldOrigin::Live {
            offset: tui.live_offset,
            line: tui.live_line,
        },
        Level::Run | Level::Participant => HeldOrigin::Rank(tui.offset),
    }
}

/// Cancel a held run's unresolved absolute-end application: the marker
/// returns to the position the run replaced, so a dropped or evicted key
/// moves nothing and no later answer resolves it (M1).
pub(super) fn cancel_end(tui: &mut Tui, origin: HeldOrigin) {
    match origin {
        HeldOrigin::Live { offset, line } => {
            tui.live_offset = offset;
            tui.live_line = line;
        }
        HeldOrigin::Rank(rank) => tui.offset = rank,
    }
}

/// Whether the cursor is in the answered window, so the next held key can
/// be applied against it. A move that leaves it asks the next read.
pub(super) fn in_window(tui: &Tui, views: &Views) -> bool {
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

/// Take one application off the head run: a run of several keeps its
/// count, and the last leaves the queue. The application consumed is the
/// one an answer just resolved, so the rest of the run no longer carries
/// its end marker.
pub(super) fn consumed(tui: &mut Tui) {
    if let Some(entry) = tui.held.first_mut() {
        entry.count -= 1;
        entry.end = None;
        if entry.count == 0 {
            tui.held.remove(0);
        }
    }
}

/// Apply the held keys, in order, against the answered window. A key that
/// would cross it asks the next page and keeps its place, and every key
/// behind it waits. A queue pressed on another view — a reselected fleet,
/// a pane the frame no longer draws, a level the operator left — is
/// dropped rather than applied to what replaced it (M4, decision 0071
/// ruling 5).
pub(super) fn flush_held(tui: &mut Tui, views: &Views) {
    if tui.held.is_empty() {
        tui.want = None;
        return;
    }
    if tui.held_for.as_ref() != Some(&held_view(tui, views)) {
        drop_held(tui);
        return;
    }
    while let Some(&Held { step: next, .. }) = tui.held.first() {
        if tui.held.first().is_some_and(|held| held.end.is_some()) {
            // The head's absolute-end application already happened; only
            // an answer that names that end can resolve it, never a
            // second application of the same key (M1).
            tui.want = Some(usize::MAX);
            return;
        }
        if !can_move(tui, views, next) {
            tui.want = Some(ask_want(tui, next));
            return;
        }
        let origin = held_origin(tui);
        step(tui, views, next);
        if checkpoint_offset(tui) == usize::MAX {
            // The step asked for the stream's absolute end, so the head
            // owns the marker its one application left, whether this
            // answer names the end or a later one (M1). Until an answer
            // names that end, the key stays at the head and every key
            // behind it waits: subtracting from the unresolved marker
            // would lose a movement the whole read makes (M2). An answer
            // that names the end resolves the step now through the one
            // resolution path, consuming the owner, so the keys behind it
            // keep their order in the same flush. The loop's head is the
            // one it names, so `first_mut` always has a head; the held end's
            // application is driven by
            // `chief_landing_held_end_owns_the_movement_after_its_first_answer`.
            tui.held
                .first_mut()
                .expect("the loop's head is still held")
                .end = Some(origin);
            if views.end.is_none() {
                tui.want = Some(usize::MAX);
                return;
            }
            resolve_end(tui, views);
        } else {
            consumed(tui);
        }
        if !in_window(tui, views) {
            tui.want = None;
            return;
        }
    }
    tui.want = None;
    tui.held_for = None;
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
    } else if !tui.held.is_empty() && tui.held_for.as_ref() != Some(&held_view(tui, views)) {
        // A view that leaves the frame while its read is out takes its
        // held movement with it; coming back before the answer does not
        // revive it (M4, decision 0071 ruling 5).
        drop_held(tui);
    }
    if lens_of(tui, views).is_none() {
        tui.scope = None;
    }
    seed_cursor(tui, views);
    if seat_of(tui, views).is_none() {
        tui.seat = None;
        if tui.level == Level::Participant {
            // The seat that left takes the movement pressed on it with it:
            // its unresolved end is cancelled against the participant view
            // before the level moves, so no later key applies it to the
            // fleet (M1, decision 0071 ruling 5).
            drop_held(tui);
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
        // The run that vanished takes the movement pressed on it with it:
        // its unresolved end is cancelled against the view it was pressed
        // on before the level moves, so no later key applies it to the
        // fleet (M1, decision 0071 ruling 5).
        drop_held(tui);
        tui.run = None;
        tui.level = Level::Runs;
        tui.pane = 0;
        tui.scope = None;
        tui.seat = None;
        tui.cursor = [None, None, None];
    }
}
