//! Movement over the navigable lists: every list is a `Vec` of stable
//! keys, and one `move_to` walks all of them, the graph's rail and lanes
//! included.

use super::*;

// ----------------------------------------------------------- the movement

/// Every navigable list is a `Vec` of stable keys, filtered over
/// **sanitized** labels: the filter is matched against exactly the text
/// the operator can see.
pub(super) fn keys_for(tui: &Tui, views: &Views) -> Vec<String> {
    labels_for(tui, views)
        .into_iter()
        .filter(|(_, label)| matches(&tui.filter, label))
        .map(|(key, _)| key)
        .collect()
}

/// Whether a list's `label` holds what `filter` searches for.
fn matches(filter: &str, label: &str) -> bool {
    let needle = Safe::new(filter).as_str().to_lowercase();
    label.to_lowercase().contains(&needle)
}

/// A fleet row as the list labels it, found by id and by the title line
/// the list paints as much of as its column holds.
fn fleet_label(row: &RunRow) -> String {
    format!("{} {}", safe(&row.run_id), listed_title(row))
}

/// Whether the fleet's list keeps `row` under `filter`: the one test the
/// list's keys and the reader that pages it under a filter both make.
pub(crate) fn keeps(filter: &str, row: &RunRow) -> bool {
    filter.is_empty() || matches(filter, &fleet_label(row))
}

pub(super) fn labels_for(tui: &Tui, views: &Views) -> Vec<(String, String)> {
    match (tui.level, views.run.as_ref()) {
        // The fleet in the order it is listed, found by id and by the
        // title line the list paints as much of as its column holds.
        (Level::Runs, _) => fleet_sections(tui, views)
            .0
            .into_iter()
            .flat_map(|(_, rows)| rows)
            .map(|row| (key_of(row).to_string(), fleet_label(row)))
            .collect(),
        (Level::Run, Some(view)) => run_labels(tui, view, lens_of(tui, views).as_ref()),
        // The PARTICIPANT panes are paragraphs with an offset, and a run
        // that will not load has no lists to move over.
        _ => Vec::new(),
    }
}

pub(super) fn run_labels(
    tui: &Tui,
    view: &RunView,
    lens: Option<&brokkr_view::Lens>,
) -> Vec<(String, String)> {
    match tui.pane {
        // The graph is the SELECTOR: it lists every phase the run
        // visited, scoped or not, because a pane that hid the phases
        // you might scope next could never replace one scope with
        // another. The lens marks; it does not hide, here.
        0 => view
            .phases
            .iter()
            .map(|phase| (phase.name.clone(), safe(&phase.name)))
            .collect(),
        1 => view
            .participants
            .iter()
            .filter(|part| brokkr_view::keeps_participant(lens, part))
            .map(|part| (part.key.clone(), safe(&part.label)))
            .collect(),
        _ => view
            .journal
            .iter()
            .filter(|row| row.in_trail && brokkr_view::keeps_row(lens, row))
            .map(|row| (row.seq.to_string(), safe(&row.what.text)))
            .collect(),
    }
}

/// The fleet's sections as the list shows them (#491), in the model's
/// order, and how many runs the list folds into its count line: the
/// older ones, listed whole when `a` asked for them or a filter is
/// searching every run, and otherwise as many as the list has room for
/// in the frame the shell measured (#503).
pub(super) type Listed<'a> = (Vec<(Section, Vec<&'a RunRow>)>, usize);

pub(super) fn fleet_sections<'a>(tui: &Tui, views: &'a Views) -> Listed<'a> {
    let every = tui.all || !tui.filter.is_empty();
    let (mut kept, folded): (Vec<_>, Vec<_>) = brokkr_view::sections(&views.runs.runs, &views.now)
        .into_iter()
        .partition(|(section, _)| every || *section != Section::Older);
    let older: Vec<&RunRow> = folded.into_iter().flat_map(|(_, rows)| rows).collect();
    let room = older_room(tui, views, &kept, older.len());
    let (drawn, rest) = older.split_at(room);
    if !drawn.is_empty() {
        kept.push((Section::Older, drawn.to_vec()));
    }
    (kept, rest.len() + views.unread.older)
}

/// The one line "needs you" counts the runs behind their summary by,
/// past those a refresh reads whole (decision 0089 ruling 6), when any
/// are: none is placed by its stale row, and none is dropped silently.
pub(super) fn behind_line(views: &Views) -> Option<String> {
    brokkr_view::behind_line(views.unread.behind)
}

/// The list lines that count takes: its own, and the heading of "needs
/// you" when no run listed there draws it.
fn behind_lines(kept: &[(Section, Vec<&RunRow>)], views: &Views) -> usize {
    let headed = kept
        .iter()
        .any(|(section, rows)| *section == Section::NeedsYou && !rows.is_empty());
    match behind_line(views) {
        None => 0,
        Some(_) if headed => 1,
        Some(_) => 2,
    }
}

/// The window of the fleet the list asks to be read (decision 0089 ruling
/// 6): from the key past the last page it scrolled beyond, the page there
/// and the one ahead, kept by the filter typed over the fleet, `a` or no
/// `a`. However far it scrolls, a frame reads two pages.
pub(crate) fn asked_window(tui: &Tui) -> Window {
    let filter = match tui.level {
        Level::Runs => tui.filter.clone(),
        Level::Run | Level::Participant => String::new(),
    };
    let scrolled = (tui.scrolled.filter == filter).then(|| tui.scrolled.past.last());
    Window {
        after: scrolled.flatten().cloned(),
        filter,
    }
}

/// The list's window follows its cursor by keyset: onto the page ahead
/// once the cursor reaches it, back a page when the cursor steps up off
/// the window's first row, to the top once it steps back past the
/// [`SCROLL_BACK`](super::state::SCROLL_BACK) pages it keeps keys for,
/// and on `g` or a wrap down past its last. It moves over the frame of
/// the window it asked for alone, so a key pressed before that frame
/// arrives moves it once. The fleet's last
/// page by the reverse order is a later slice's: `G` and a wrap up reach
/// the window's last row, a page further each time.
fn follow(tui: &mut Tui, views: &Views, (keys, from): (&[String], Option<usize>), step: Step) {
    let window = asked_window(tui);
    if views.window != window {
        return;
    }
    let scrolled = window.after.is_some();
    let wrapped = step == Step::Down && from.is_some_and(|at| at + 1 == keys.len());
    match step {
        Step::Top | Step::Down if scrolled && (step == Step::Top || wrapped) => {
            tui.scrolled = Scrolled::default();
            // Seeded on the first row of the top's own frame.
            tui.cursor[0] = None;
        }
        Step::Up | Step::PageUp if scrolled && from == Some(0) => {
            tui.scrolled.past.pop();
            tui.cursor[0] = keys.first().cloned();
        }
        _ => {
            let cursor = tui.cursor[0].as_deref();
            let at = views
                .runs
                .runs
                .iter()
                .position(|row| Some(key_of(row)) == cursor);
            let reached = at >= Some(brokkr_view::PAGE_ROWS);
            let Some(ahead) = views.ahead.as_ref().filter(|_| reached) else {
                return;
            };
            if tui.scrolled.filter != window.filter {
                tui.scrolled = Scrolled {
                    filter: window.filter,
                    past: Vec::new(),
                };
            }
            tui.scrolled.passed(ahead.clone());
        }
    }
}

/// How many of `older` runs the list draws below the sections it keeps:
/// its height inside its borders and under the tab bar, less each kept
/// section's heading and lines, the older runs' heading, and the count
/// line when some still fold.
fn older_room(tui: &Tui, views: &Views, kept: &[(Section, Vec<&RunRow>)], older: usize) -> usize {
    let inside = usize::from(tui.height).saturating_sub(4 + usize::from(tabbed(tui)));
    let lines = |rows: &[&RunRow]| -> usize {
        let drawn: usize = rows
            .iter()
            .map(|row| title_lines(tui, views, row).len())
            .sum();
        usize::from(!rows.is_empty()) + drawn
    };
    let used: usize = kept.iter().map(|(_, rows)| lines(rows)).sum();
    let used = used + behind_lines(kept, views);
    let left = inside.saturating_sub(used + 1);
    match older <= left {
        true => older,
        false => left.saturating_sub(1),
    }
}

/// A run's title line as the fleet lists it, sanitized and whole: the
/// list paints it clamped to its title column (#503), and the filter
/// matches it, so the two read one line.
pub(super) fn listed_title(row: &RunRow) -> String {
    safe(&brokkr_view::title_within(&row.feature, usize::MAX))
}

/// The fleet's selection: the list cursor's run, whichever of the
/// fleet's panes has the focus.
pub(super) fn selected_run<'a>(tui: &Tui, views: &'a Views) -> Option<&'a RunRow> {
    let keys = keys_for(tui, views);
    let index = index_of(&keys, &tui.cursor[0])?;
    views
        .runs
        .runs
        .iter()
        .find(|row| key_of(row) == keys[index])
}

/// The key the fleet's list selects and opens a row by: its run's whole
/// id, which a row listed by the cut of an id no writer admits keeps where
/// its page read it, so two ids cut alike are two keys (decision 0089
/// ruling 6).
pub(super) fn key_of(row: &RunRow) -> &str {
    row.place().1
}

/// The run the columns beside the list show: the fleet's selection, once
/// the frame the shell measured holds a column beside the list (#503).
/// The one answer to whether one is on the frame, which the frame, its
/// footer, the keys and the shell's question all read.
pub(super) fn detail_row<'a>(tui: &Tui, views: &'a Views) -> Option<&'a RunRow> {
    let beside = layout(tui.toggles, tui.width).len() > 1;
    selected_run(tui, views).filter(|_| beside)
}

pub(super) fn index_of(keys: &[String], cursor: &Option<String>) -> Option<usize> {
    let cursor = cursor.as_deref()?;
    keys.iter().position(|key| key == cursor)
}

/// The one place wrap-around, `g`/`G` and paging exist, for every list
/// at every level. A cursor whose key is gone — the subject vanished, or
/// the filter excluded it — restarts from the top and renders no
/// highlight until it moves.
pub(super) fn move_to(keys: &[String], cursor: &mut Option<String>, step: Step) {
    if keys.is_empty() {
        *cursor = None;
        return;
    }
    let last = keys.len() - 1;
    let index = match index_of(keys, cursor) {
        Some(index) => match step {
            Step::Up if index == 0 => last,
            Step::Up => index - 1,
            Step::Down if index == last => 0,
            Step::Down => index + 1,
            Step::Top => 0,
            Step::Bottom => last,
            Step::PageUp => index.saturating_sub(PAGE),
            Step::PageDown => (index + PAGE).min(last),
        },
        None => match step {
            Step::Bottom => last,
            _ => 0,
        },
    };
    *cursor = Some(keys[index].clone());
}

/// How many lines the checkpoint pane holds — the one PARTICIPANT pane
/// that is still a paragraph with an offset rather than a cursor.
pub(super) fn stream_len(tui: &Tui, views: &Views) -> usize {
    seat_of(tui, views).map_or(0, |part| part.checkpoints.len())
}

/// The transcript pane's list: one key per turn, the turn's index in
/// the command's own displayed sequence. Live prose streaming only
/// APPENDS turns, so an index is a stable key, and the cursor survives an
/// appending refresh by the same absence of code as every other list.
pub(super) fn turn_keys(views: &Views) -> Vec<String> {
    turn_keys_of(views.transcript.as_ref())
}

/// The pane's turn keys for one read; the pane and the 11.2 cross-surface
/// comparison share this exact enumeration.
pub(super) fn turn_keys_of(read: Option<&TranscriptRead>) -> Vec<String> {
    let count = read.map_or(0, |read| read.turns.len());
    (0..count).map(|index| index.to_string()).collect()
}

/// The transcript's live selection: a turn the cursor's index still
/// names. A stale index — a transcript that shrank or was replaced —
/// selects nothing, exactly like a stale key anywhere else.
pub(super) fn selected_turn<'a>(tui: &Tui, views: &'a Views) -> Option<(usize, &'a Turn)> {
    let read = views.transcript.as_ref()?;
    let index = index_of(&turn_keys(views), &tui.turn)?;
    Some((index, &read.turns[index]))
}

pub(super) fn step(tui: &mut Tui, views: &Views, step: Step) {
    if tui.level == Level::Participant {
        // The transcript pane moves a cursor over TURNS — never over
        // wrapped lines — through the same `move_to` as every list.
        if tui.pane == 1 {
            let keys = turn_keys(views);
            move_to(&keys, &mut tui.turn, step);
            return;
        }
        let len = stream_len(tui, views);
        // The end key keeps the stream's logical rank; movement reads that
        // same coordinate, including the stream's current end, and Down
        // there wraps as the whole read's does (M4, R2, R3, R4; decision
        // 0071 ruling 5).
        scroll_paged(
            &mut tui.offset,
            views.start,
            len,
            step,
            views.more,
            views.end,
            Paging::Absolute,
        );
        return;
    }
    if tui.level == Level::Runs {
        fleet_step(tui, views, step);
        return;
    }
    let keys = keys_for(tui, views);
    let pane = tui.pane;
    move_to(&keys, &mut tui.cursor[pane], step);
    // On the graph, moving IS scoping — the console scopes on a click,
    // and an operator who has moved the rail cursor onto a phase has
    // said which phase they mean. Enter then descends; Esc clears.
    if in_graph(tui) {
        tui.scope = tui.cursor[0].clone().map(brokkr_view::Scope::Phase);
    }
}

/// A paragraph pane's offset moves through the SAME function as every
/// list, so wrap-around and paging have exactly one implementation.
fn scroll(offset: &mut usize, len: usize, step: Step) {
    let keys: Vec<String> = (0..len).map(|line| line.to_string()).collect();
    let mut cursor = Some(offset.to_string());
    move_to(&keys, &mut cursor, step);
    *offset = cursor.and_then(|line| line.parse().ok()).unwrap_or(0);
}

/// How a paragraph's page step treats the held window: the participant
/// level is a paged stream whose rows continue past the window, while the
/// findings column is held whole and stops at its last drawn line.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Paging {
    Absolute,
    Held,
}

/// A paragraph's offset, in the stream's own rows: the window holds rows
/// `start..start + len`, and movement is one arithmetic over the absolute
/// stream coordinate, never over the window's edges. A paged stream's
/// page step therefore keeps all ten rows it names when it leaves the
/// held window, and the shell reads the page the destination names (R2).
/// Where the stream's current end is known, a step past it wraps there as
/// the whole read's does, whatever the display window's bounds (R3). A
/// column held whole stops at its last drawn line, or asks one line past
/// it when a page follows, so a page can be scrolled past its oldest row
/// (decision 0089 slice 3). One coordinate, translated once into the
/// window (R1).
pub(super) fn scroll_paged(
    offset: &mut usize,
    start: usize,
    len: usize,
    step: Step,
    more: bool,
    end: Option<usize>,
    paging: Paging,
) {
    if len == 0 {
        *offset = start;
        return;
    }
    let window_end = start.saturating_add(len);
    let stream_end = stream_end_of(end, more, window_end);
    match paging {
        Paging::Absolute => scroll_absolute(offset, step, more, stream_end),
        // A column held whole is a paragraph, not a rank: its end is the
        // window's, so no read's rank narrows it.
        Paging::Held => scroll_held(offset, step, more, window_end),
    }
}

/// The stream's end in the offset's own coordinate: the read's answer
/// where it names one, else the window's end, or the absolute end when a
/// page follows the window (M3).
fn stream_end_of(end: Option<usize>, more: bool, window_end: usize) -> usize {
    end.unwrap_or(match more {
        true => usize::MAX,
        false => window_end,
    })
}

/// A step to the stream's end: its last row where the window is whole, or
/// the absolute-end marker where a page beyond it must name the end (M3).
fn end_target(more: bool, stream_end: usize) -> usize {
    match more {
        true => usize::MAX,
        false => stream_end.saturating_sub(1),
    }
}

/// A paged stream's step over its absolute coordinate. The top and the end
/// are the stream's, not the held window's, and a page step always moves
/// the ten rows it names from the absolute offset, so a crossing keeps its
/// remaining distance and never passes the stream's known end (R1, R2,
/// F2, M3).
fn scroll_absolute(offset: &mut usize, step: Step, more: bool, stream_end: usize) {
    match step {
        Step::Top => *offset = 0,
        Step::Bottom => *offset = end_target(more, stream_end),
        // Up from the stream's top is the stream's end, not the held
        // window's: ask for the absolute end and let the read say it (M1).
        Step::Up if *offset == 0 => *offset = end_target(more, stream_end),
        Step::Up => *offset = offset.saturating_sub(1),
        // Down from the stream's current end is its top, never the held
        // window's first row (M1, R3).
        Step::Down if offset.saturating_add(1) >= stream_end => *offset = 0,
        Step::Down => *offset = offset.saturating_add(1),
        Step::PageDown => {
            *offset = match stream_end {
                usize::MAX => offset.saturating_add(PAGE),
                stream_end => offset
                    .saturating_add(PAGE)
                    .min(stream_end.saturating_sub(1)),
            }
        }
        Step::PageUp => *offset = offset.saturating_sub(PAGE),
    }
}

/// A held column's step over its window: one row past the window's last
/// asks the page that follows it, and a page step stops at the window's
/// last line or asks one line past it when a page follows (decision 0089
/// slice 3).
fn scroll_held(offset: &mut usize, step: Step, more: bool, window_end: usize) {
    match step {
        Step::Top => *offset = 0,
        Step::Bottom => *offset = end_target(more, window_end),
        Step::Up if *offset == 0 => *offset = end_target(more, window_end),
        Step::Up => *offset = offset.saturating_sub(1),
        Step::Down => held_down(offset, more, window_end),
        // A held column's page step stops at the window's last line, or
        // asks one line past it when a page follows.
        Step::PageDown => {
            *offset = match more && offset.saturating_add(PAGE) >= window_end {
                true => window_end,
                false => offset
                    .saturating_add(PAGE)
                    .min(window_end.saturating_sub(1)),
            }
        }
        Step::PageUp if *offset >= window_end => *offset = window_end.saturating_sub(1),
        Step::PageUp => *offset = offset.saturating_sub(PAGE),
    }
}

/// A held column's Down: one row past the window's last when the shell
/// says a page follows it, so the page can be asked for; at the stream's
/// current end it is the stream's top, never the held window's first row
/// (M1, R3).
fn held_down(offset: &mut usize, more: bool, window_end: usize) {
    // A paragraph's stream end is its window's: one row past the last
    // painted line asks the page that follows, or returns to the top.
    if offset.saturating_add(1) >= window_end {
        *offset = match more {
            true => window_end,
            false => 0,
        };
        return;
    }
    *offset = offset.saturating_add(1);
}

/// The live column's drawn line in the held window, when the cursor is
/// inside it: the row's first drawn line plus the line within the row.
/// A continuation the window no longer lays — a resize narrowed the row
/// it was saved in — is the whole read's position on the row's last drawn
/// line, so movement from it is the whole read's movement, never a page
/// the console keeps asking for (M1, decision 0071 ruling 5).
pub(super) fn live_position(tui: &Tui, views: &Views, laid: &Laid) -> Option<isize> {
    let rows = laid.rows.len().checked_sub(1)?;
    // `rows == 0` is already `offset - start >= rows`: the window has no
    // row to stand in, so the check below refuses it without a second arm.
    if tui.live_offset < views.start || tui.live_offset.saturating_sub(views.start) >= rows {
        return None;
    }
    let row = tui.live_offset - views.start;
    let line = tui
        .live_line
        .min(laid.rows[row + 1].saturating_sub(laid.rows[row] + 1));
    Some((laid.rows[row] + line) as isize)
}

/// The live column's position moved within the held window: `Top` and
/// `Bottom` are the stream's, a step past the window's last drawn line
/// wraps at a known end, and a step inside the window is one drawn line
/// (R1). A step that would leave it is held before this runs, so it never
/// has to carry a distance across a page (decision 0071 ruling 5).
pub(super) fn scroll_live(tui: &mut Tui, views: &Views, laid: &Laid, step: Step, more: bool) {
    let rows = laid.rows.len().saturating_sub(1);
    if rows == 0 {
        tui.live_offset = 0;
        tui.live_line = 0;
        return;
    }
    let end = laid.rows[rows] as isize;
    let at = live_position(tui, views, laid);
    match step {
        Step::Up if at == Some(0) => {
            // The held window is the stream's top (`can_move`), so `Up`
            // asks the absolute end; `settle_end` resolves it.
            tui.live_offset = usize::MAX;
            tui.live_line = 0;
            return;
        }
        Step::Down if at == Some(end - 1) => {
            // The held window is the stream's end (`can_move`), so `Down`
            // is its top, as the whole read's is.
            tui.live_offset = 0;
            tui.live_line = 0;
            return;
        }
        _ => {}
    }
    let next = match step {
        Step::Top => {
            tui.live_offset = 0;
            tui.live_line = 0;
            return;
        }
        Step::Bottom => {
            if more {
                tui.live_offset = usize::MAX;
                tui.live_line = 0;
            } else {
                place_live(tui, views, laid, (end - 1) as usize);
            }
            return;
        }
        Step::Up => at.map(|at| at - 1),
        Step::Down => at.map(|at| at + 1),
        Step::PageUp => at.map(|at| (at - PAGE as isize).max(0)),
        Step::PageDown => at.map(|at| (at + PAGE as isize).min(end - 1)),
    };
    let Some(next) = next else {
        return;
    };
    place_live(tui, views, laid, next as usize);
}

/// The fleet's keys move the list, or scroll the focused column a drawn
/// line at a time (#503). A new selection is read from its first line,
/// its commission folded.
fn fleet_step(tui: &mut Tui, views: &Views, step: Step) {
    let column = focused(tui, views);
    let width = width_of_column(tui, views, column).unwrap_or_default();
    match detail_row(tui, views).map(|row| (column, row)) {
        Some((FleetColumn::Dashboard, row)) => {
            let drawn = dashboard_lines(tui, views, row, width).len();
            scroll(&mut tui.offset, drawn, step);
        }
        Some((FleetColumn::Live, row)) => {
            let laid = live_layout(views, row, width);
            if laid.streamed {
                scroll_live(tui, views, &laid, step, views.more);
            } else {
                // Findings are not a stream: their offset is a drawn line
                // from the top, and no window stands before it.
                scroll_paged(
                    &mut tui.live_offset,
                    0,
                    laid.lines.len(),
                    step,
                    views.more,
                    None,
                    Paging::Held,
                );
            }
        }
        Some((FleetColumn::List, _)) | None => {
            let keys = keys_for(tui, views);
            let from = index_of(&keys, &tui.cursor[0]);
            move_to(&keys, &mut tui.cursor[0], step);
            follow(tui, views, (&keys, from), step);
            tui.offset = 0;
            tui.live_offset = 0;
            tui.live_line = 0;
            tui.commission = false;
        }
    }
}

/// A graph that opens with nothing selected is a graph whose `Enter`
/// does nothing, which reads as a broken key rather than as an empty
/// selection. The rail cursor starts on the run's CURRENT phase — where
/// an operator is already looking — and only when it has none. The
/// fleet opens on its first row (#503): the first run that needs the
/// operator, or else the first running, so a wide frame draws the
/// detail pane at once.
pub(super) fn seed_cursor(tui: &mut Tui, views: &Views) {
    if tui.cursor[0].is_some() {
        return;
    }
    tui.cursor[0] = match (tui.level, views.run.as_ref()) {
        // On the first row of the window it asked for, never of a frame
        // of another (decision 0089 ruling 6).
        (Level::Runs, _) if views.window != asked_window(tui) => None,
        (Level::Runs, _) => keys_for(tui, views).into_iter().next(),
        (Level::Run, Some(view)) => current_phase(view).map(str::to_string),
        (Level::Run | Level::Participant, _) => None,
    };
}

/// Where an operator is already looking on a run's rail: its current
/// phase. A journal that folds to no status has none, and the last phase
/// entered is still that place. The run level opens its cursor there and
/// the fleet's dashboard boxes it (#508).
pub(super) fn current_phase(view: &RunView) -> Option<&str> {
    let phase = view.phases.iter().find(|phase| phase.current);
    let phase = phase.or_else(|| view.phases.last());
    phase.map(|phase| phase.name.as_str())
}

/// Whether the row is opened in place: a running run `→` opened.
pub(super) fn opened(tui: &Tui, row: &RunRow) -> bool {
    row.verdict.standing == Standing::Running && tui.expanded.contains(&row.run_id)
}

/// `→`, `l` and Space open the selected running run in place, and `←`
/// and `h` fold it (#503). Only a running run has a place to show; on
/// any other row opening changes nothing.
pub(super) fn open_in_place(tui: &mut Tui, views: &Views, open: bool) {
    let Some(row) = selected_run(tui, views) else {
        return;
    };
    match open {
        true if row.verdict.standing == Standing::Running => {
            tui.expanded.insert(row.run_id.clone());
        }
        true => {}
        false => {
            tui.expanded.remove(&row.run_id);
        }
    }
}

/// The graph is the one pane whose primary axis is horizontal, so it is
/// the one pane where `←→` and `↑↓` do different things.
pub(super) fn in_graph(tui: &Tui) -> bool {
    tui.level == Level::Run && tui.pane == 0
}

/// The selected phase's nodes in draw order — the lane cursor's list. A
/// plain phase has none, and `move_to` over an empty list sets `None`,
/// so `↑↓` there are inert by construction rather than by a special case.
pub(super) fn lane_keys(tui: &Tui, views: &Views) -> Vec<String> {
    let Some(view) = views.run.as_ref() else {
        return Vec::new();
    };
    let Some(cursor) = tui.cursor[0].as_deref() else {
        return Vec::new();
    };
    view.phases
        .iter()
        .filter(|phase| phase.name == cursor)
        .flat_map(|phase| phase.columns.iter())
        .flat_map(|column| column.nodes.iter())
        .map(|node| node.key.clone())
        .collect()
}

/// The participant the lane cursor is standing on, resolved against the
/// **fresh** models like every other selection. A fork member's node key
/// is that member's `Participant.key` and a plain step's is its seat's,
/// so this is a lookup rather than a second mapping. A structural node —
/// a finished step nobody tagged — answers `None`, and so does a lane
/// cursor that is nowhere.
pub(super) fn lane_member<'a>(tui: &Tui, views: &'a Views) -> Option<&'a Participant> {
    tui.node.as_deref().and_then(|key| participant(views, key))
}

/// What the graph's two cursors say the scope is, in one place. On the
/// rail, moving IS scoping (the standing law); in the lanes it is the
/// same law one level down — a member node scopes that seat, through the
/// same `Scope`/`lens_for` the seats pane's own `Enter` produces, never a
/// second filtering mechanism. Anything the lanes cannot resolve falls
/// back to exactly what the rail already set.
pub(super) fn graph_scope(tui: &Tui, views: &Views) -> Option<brokkr_view::Scope> {
    match lane_member(tui, views) {
        Some(part) => Some(brokkr_view::Scope::Seat(part.key.clone())),
        None => tui.cursor[0].clone().map(brokkr_view::Scope::Phase),
    }
}

/// `↑↓` walk the lanes inside the graph pane, and the focused list
/// everywhere else. Both go through the same `move_to`, so wrap-around
/// and the empty-list case are already specified and already tested.
pub(super) fn arrow(tui: &mut Tui, views: &Views, direction: Step) {
    match in_graph(tui) {
        true => {
            let keys = lane_keys(tui, views);
            move_to(&keys, &mut tui.node, direction);
            // The lane cursor landed somewhere, so it has said what the
            // operator means: this seat, or — off the members — the
            // rail's phase again.
            tui.scope = graph_scope(tui, views);
        }
        false => step(tui, views, direction),
    }
}

/// `←→` walk the rail. Everywhere else they are a **named** no-op: the
/// lists in the other panes have no horizontal axis, and a wildcard here
/// would be an untested claim about what they do with an arrow key.
pub(super) fn rail_move(tui: &mut Tui, views: &Views, direction: Step) {
    // The guard IS the naming: outside the graph pane `←→` change no
    // state at all, and a test presses them everywhere to prove it.
    if !in_graph(tui) {
        return;
    }
    let keys = keys_for(tui, views);
    move_to(&keys, &mut tui.cursor[0], direction);
    // The rail moved, so whatever lane the cursor was in is gone — and
    // with it any seat that lane had scoped. The phase under the rail is
    // what remains selected, which is what rail movement has always said.
    tui.node = None;
    tui.scope = graph_scope(tui, views);
}

/// The live selection: a cursor key still present in the current list.
/// A stale key descends into nothing.
pub(super) fn selected(tui: &Tui, views: &Views) -> Option<String> {
    let keys = keys_for(tui, views);
    index_of(&keys, &tui.cursor[tui.pane]).map(|index| keys[index].clone())
}
