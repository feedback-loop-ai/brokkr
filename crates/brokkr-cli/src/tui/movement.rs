//! Movement over the navigable lists: every list is a `Vec` of stable
//! keys, and one `move_to` walks all of them, the graph's rail and lanes
//! included.

use super::*;

// ----------------------------------------------------------- the movement

/// Every navigable list is a `Vec` of stable keys, filtered over
/// **sanitized** labels: the filter is matched against exactly the text
/// the operator can see.
pub(super) fn keys_for(tui: &Tui, views: &Views) -> Vec<String> {
    let needle = Safe::new(&tui.filter).as_str().to_lowercase();
    labels_for(tui, views)
        .into_iter()
        .filter(|(_, label)| label.to_lowercase().contains(&needle))
        .map(|(key, _)| key)
        .collect()
}

pub(super) fn labels_for(tui: &Tui, views: &Views) -> Vec<(String, String)> {
    match (tui.level, views.run.as_ref()) {
        // The fleet in the order it is listed, found by id and title.
        (Level::Runs, _) => fleet_sections(tui, views)
            .0
            .into_iter()
            .flat_map(|(_, rows)| rows)
            .map(|row| {
                let mut label = row.run_id.clone();
                label.push(' ');
                label.push_str(&row.title);
                (row.run_id.clone(), safe(&label))
            })
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
    lens: Option<&render::Lens>,
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
            .filter(|part| render::keeps_participant(lens, part))
            .map(|part| (part.key.clone(), safe(&part.label)))
            .collect(),
        _ => view
            .journal
            .iter()
            .filter(|row| row.in_trail && render::keeps_row(lens, row))
            .map(|row| (row.seq.to_string(), safe(&row.what.text)))
            .collect(),
    }
}

/// The fleet's sections as the list shows them (#491), in the model's
/// order, and how many runs the list folds into its count line: the
/// older ones, listed only when `a` asked for them or a filter is
/// searching every run.
pub(super) type Listed<'a> = (Vec<(Section, Vec<&'a RunRow>)>, usize);

pub(super) fn fleet_sections<'a>(tui: &Tui, views: &'a Views) -> Listed<'a> {
    let every = tui.all || !tui.filter.is_empty();
    let (kept, folded): (Vec<_>, Vec<_>) = brokkr_view::sections(&views.runs.runs, &views.now)
        .into_iter()
        .partition(|(section, _)| every || *section != Section::Older);
    (kept, folded.iter().map(|(_, rows)| rows.len()).sum())
}

/// The fleet's selection: the list cursor's run, whichever of the
/// fleet's panes has the focus.
pub(super) fn selected_run<'a>(tui: &Tui, views: &'a Views) -> Option<&'a RunRow> {
    let keys = keys_for(tui, views);
    let index = index_of(&keys, &tui.cursor[0])?;
    views.runs.runs.iter().find(|row| row.run_id == keys[index])
}

/// The run the detail pane shows on a frame `width` wide: the fleet's
/// selection, once the frame holds the pane beside the capped list.
pub(super) fn detail_row<'a>(tui: &Tui, views: &'a Views, width: u16) -> Option<&'a RunRow> {
    match width >= DETAIL_MIN_WIDTH {
        true => selected_run(tui, views),
        false => None,
    }
}

/// At the fleet level: the detail pane is on the frame the shell last
/// drew, and focused. A terminal narrowed under a focused pane hands the
/// keys back to the list.
pub(super) fn detail_focused(tui: &Tui, views: &Views) -> bool {
    tui.pane == 1 && detail_row(tui, views, tui.width).is_some()
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
        scroll(&mut tui.offset, len, step);
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
        tui.scope = tui.cursor[0].clone().map(render::Scope::Phase);
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

/// The fleet's keys move the list, or scroll the detail pane's feature
/// a line at a time while it has the focus. A new selection is read
/// from its first line.
fn fleet_step(tui: &mut Tui, views: &Views, step: Step) {
    match detail_row(tui, views, tui.width).filter(|_| tui.pane == 1) {
        Some(row) => scroll(&mut tui.offset, row.feature.lines().count(), step),
        None => {
            let keys = keys_for(tui, views);
            move_to(&keys, &mut tui.cursor[0], step);
            tui.offset = 0;
        }
    }
}

/// A graph that opens with nothing selected is a graph whose `Enter`
/// does nothing, which reads as a broken key rather than as an empty
/// selection. The rail cursor starts on the run's CURRENT phase — where
/// an operator is already looking — and only when it has none.
pub(super) fn seed_cursor(tui: &mut Tui, views: &Views) {
    if tui.level != Level::Run || tui.cursor[0].is_some() {
        return;
    }
    let Some(view) = views.run.as_ref() else {
        return;
    };
    tui.cursor[0] = view
        .phases
        .iter()
        .find(|phase| phase.current)
        // A journal that folds to no status has no current phase; the
        // last phase entered is still where an operator is looking.
        .or_else(|| view.phases.last())
        .map(|phase| phase.name.clone());
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
pub(super) fn graph_scope(tui: &Tui, views: &Views) -> Option<render::Scope> {
    match lane_member(tui, views) {
        Some(part) => Some(render::Scope::Seat(part.key.clone())),
        None => tui.cursor[0].clone().map(render::Scope::Phase),
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
