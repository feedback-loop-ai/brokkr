//! The console's key vocabulary, its one translation from crossterm, and
//! the pure state machine `apply` that a key drives: the ladder `Enter`
//! climbs and `Esc` descends.

use super::*;

// --------------------------------------------------------------- the keys

/// Our own key vocabulary. Translation happens once, at the boundary.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Key {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Tab,
    Enter,
    Escape,
    Backspace,
    Char(char),
    Quit,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Flow {
    Continue,
    Quit,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Step {
    Up,
    Down,
    Top,
    Bottom,
    PageUp,
    PageDown,
}

/// Mouse, paste and focus events are ignored by **named** arms: a
/// wildcard here is an untested claim about what a terminal can send.
pub(crate) fn from_crossterm(event: Event) -> Option<Key> {
    match event {
        Event::Key(key) => from_key(key),
        Event::Mouse(_) => None,
        Event::Paste(_) => None,
        Event::FocusGained => None,
        Event::FocusLost => None,
        Event::Resize(_, _) => None,
    }
}

/// Windows delivers key **release** events too; a handler matching on
/// `KeyCode` alone would process every keystroke twice on the exact CI
/// leg crossterm exists to make survivable. `Ctrl+C` quits alongside
/// `q`, because raw mode disables SIGINT.
pub(super) fn from_key(key: KeyEvent) -> Option<Key> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Key::Quit);
    }
    from_code(key.code)
}

/// A pressed key's code in our vocabulary. Anything the console binds no
/// key to is `None`.
fn from_code(code: KeyCode) -> Option<Key> {
    match code {
        KeyCode::Up => Some(Key::Up),
        KeyCode::Down => Some(Key::Down),
        KeyCode::Left => Some(Key::Left),
        KeyCode::Right => Some(Key::Right),
        KeyCode::PageUp => Some(Key::PageUp),
        KeyCode::PageDown => Some(Key::PageDown),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::BackTab => Some(Key::Tab),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Esc => Some(Key::Escape),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Char(character) => Some(Key::Char(character)),
        _ => None,
    }
}

// ---------------------------------------------------------- the ladder

/// `Enter` pushes one rung, `Esc` pops one. At the RUN level the rungs
/// are `unscoped → scoped → descended`, which is how "Enter descends"
/// and "selecting a phase or a participant scopes the run level" are
/// both honoured without inventing a key the ruling does not name.
pub(super) fn enter(tui: &mut Tui, views: &Views) {
    if tui.typing {
        tui.typing = false;
        return;
    }
    match tui.level {
        // The checkpoint pane is a paragraph, not a door. The
        // transcript is a list of TURNS, and a long turn clamped by
        // its pane is not readable evidence — so Enter opens the
        // selected turn whole in the trail's own reader (the operator
        // re-ruled the paragraph contract). With NO turn selected the
        // same key opens the WHOLE transcript, because a stream
        // clamped by its pane is no more readable than one long turn
        // (the operator's ruling, superseding the inert pin). The
        // per-turn reader stays the drilldown; a pane holding no
        // transcript at all still holds no door.
        Level::Participant => {
            if tui.pane == 1 {
                // A refused or unavailable read has no reading door: the
                // pane carries the reason, and Enter must not reopen a
                // former snapshot's prose.
                let Some(read) = views.transcript.as_ref().filter(|read| read.is_readable()) else {
                    return;
                };
                match selected_turn(tui, views) {
                    Some((index, turn)) => {
                        tui.reading = Some(turn_overlay_text(index + 1, turn, read));
                        tui.reading_transcript = true;
                        tui.read_offset = 0;
                    }
                    None => {
                        tui.reading = Some(transcript_text(read));
                        tui.reading_transcript = true;
                        tui.read_offset = 0;
                    }
                }
            }
        }
        // The list's run, whether the list or its detail pane has focus.
        Level::Runs => {
            if let Some(row) = selected_run(tui, views) {
                tui.assign_run(row.run_id.clone());
            }
        }
        Level::Run => {
            let Some(key) = selected(tui, views) else {
                return;
            };
            match tui.pane {
                0 => tui.scope = Some(render::Scope::Phase(key)),
                1 => {
                    if scoped_seat(tui) == Some(key.as_str()) {
                        tui.level = Level::Participant;
                        tui.seat = Some(key);
                        tui.pane = 0;
                        tui.offset = 0;
                        // A turn cursor belongs to the transcript it
                        // was moved over, never to the next seat's.
                        tui.turn = None;
                        tui.force = true;
                    } else {
                        tui.scope = Some(render::Scope::Seat(key));
                    }
                }
                // The trail is evidence — and evidence you cannot read is not
                // evidence, so Enter opens the row's full text rather than
                // descending.
                _ => {
                    let row = views
                        .run
                        .as_ref()
                        .and_then(|view| view.journal.iter().find(|row| row.seq.to_string() == key))
                        .expect("a selected trail key resolves against the same view");
                    tui.reading = Some(format!(
                        "seq {}  {}  {}\n\n{}\n\npayload\n{}",
                        row.seq,
                        safe(&row.event_type),
                        safe(&row.recorded_at),
                        safe(&row.what.text),
                        safe(&row.payload_json),
                    ));
                    tui.reading_transcript = false;
                    tui.read_offset = 0;
                }
            }
        }
    }
}

/// A precedence ladder. **`Esc` never quits**, so a fat-fingered `Esc`
/// cannot kill the console.
pub(super) fn escape(tui: &mut Tui) {
    if tui.help {
        tui.help = false;
        return;
    }
    if tui.typing || !tui.filter.is_empty() {
        tui.typing = false;
        tui.filter.clear();
        return;
    }
    if tui.level == Level::Participant {
        // The transcript pane's first rung clears the TURN selection.
        // It is what keeps the whole-transcript door reachable after a
        // turn has been read — one obvious key, not a hidden
        // combination (decision 0014's discoverability rule) — and the
        // ladder ascends on the next press, as it always did.
        if tui.pane == 1 && tui.turn.is_some() {
            tui.turn = None;
            return;
        }
        ascend(tui);
        return;
    }
    if tui.scope.is_some() {
        tui.scope = None;
        return;
    }
    ascend(tui);
}

/// Rungs 3 and 5 alone: ascending without ever clearing a scope. This is
/// `Backspace` outside filter mode, and the tail of the `Esc` ladder.
pub(super) fn ascend(tui: &mut Tui) {
    // A filter belongs to the list it was typed over.
    tui.filter.clear();
    tui.typing = false;
    match tui.level {
        Level::Participant => {
            tui.level = Level::Run;
            tui.seat = None;
            // Land back on the seat you were reading: the symmetric pop.
            tui.pane = 1;
            tui.force = true;
        }
        Level::Run => {
            tui.level = Level::Runs;
            tui.pane = 0;
            tui.force = true;
        }
        Level::Runs => {}
    }
}

pub(super) fn backspace(tui: &mut Tui) {
    if tui.typing {
        // What makes `/` incremental.
        tui.filter.pop();
        return;
    }
    ascend(tui);
}

/// A character key. Bindings are read **here**, not at the crossterm
/// boundary: while a filter is being typed, `q` is a letter.
pub(super) fn typed(tui: &mut Tui, views: &Views, character: char) -> Flow {
    if tui.typing {
        // Operator input is sanitized like every other string: it is
        // echoed in the footer and a bracketed paste can carry an
        // escape sequence.
        tui.filter.push_str(safe(&character.to_string()).as_str());
        return Flow::Continue;
    }
    if hearth_key(tui, character) {
        return Flow::Continue;
    }
    match character {
        'q' => return Flow::Quit,
        'j' => step(tui, views, Step::Down),
        'k' => step(tui, views, Step::Up),
        'g' => step(tui, views, Step::Top),
        'G' => step(tui, views, Step::Bottom),
        'r' => tui.force = true,
        '/' => tui.typing = true,
        '?' => tui.help = !tui.help,
        // Bound where the fleet is the list; a character nothing binds
        // anywhere else.
        'a' if tui.level == Level::Runs => tui.all = !tui.all,
        _ => {}
    }
    Flow::Continue
}

/// The hearth keys, bound only where there are hearths to move between
/// and only at the level that has the bar. Everywhere else these are
/// characters nothing binds, exactly as they were. True when the key
/// switched, or tried to switch, the hearth.
fn hearth_key(tui: &mut Tui, character: char) -> bool {
    if !tabbed(tui) || tui.level != Level::Runs {
        return false;
    }
    let index = match character {
        '[' => tui.tab.saturating_sub(1),
        ']' => (tui.tab + 1).min(tui.tabs.len() - 1),
        // The bar numbers its tabs, so the numbers are the keys.
        '1'..='9' => character as usize - '1' as usize,
        _ => return false,
    };
    switch(tui, index);
    true
}

/// A key while the reader is open. The reader owns movement then: an
/// operator scrolling a long payload must not also be moving the list
/// behind it.
fn read_key(tui: &mut Tui, key: Key) -> Flow {
    match key {
        Key::Quit => return Flow::Quit,
        Key::Char('q') => return Flow::Quit,
        Key::Escape | Key::Backspace | Key::Enter | Key::Char('?') => {
            tui.reading = None;
            tui.reading_transcript = false;
            tui.read_offset = 0;
        }
        Key::Down | Key::Char('j') => tui.read_offset = tui.read_offset.saturating_add(1),
        Key::Up | Key::Char('k') => tui.read_offset = tui.read_offset.saturating_sub(1),
        Key::PageDown => tui.read_offset = tui.read_offset.saturating_add(10),
        Key::PageUp => tui.read_offset = tui.read_offset.saturating_sub(10),
        Key::Char('g') => tui.read_offset = 0,
        _ => {}
    }
    Flow::Continue
}

/// The pure state machine: view models plus a key, in; a flow, out. No
/// terminal, no store, no I/O.
pub(crate) fn apply(tui: &mut Tui, views: &Views, key: Key) -> Flow {
    if tui.reading.is_some() {
        return read_key(tui, key);
    }
    match key {
        Key::Quit => return Flow::Quit,
        Key::Char(character) => return typed(tui, views, character),
        Key::Enter => enter(tui, views),
        Key::Escape => escape(tui),
        Key::Backspace => backspace(tui),
        Key::Tab => tui.pane = (tui.pane + 1) % panes_at(tui, views),
        Key::Up => arrow(tui, views, Step::Up),
        Key::Down => arrow(tui, views, Step::Down),
        Key::Left => rail_move(tui, views, Step::Up),
        Key::Right => rail_move(tui, views, Step::Down),
        Key::PageUp => step(tui, views, Step::PageUp),
        Key::PageDown => step(tui, views, Step::PageDown),
    }
    Flow::Continue
}
