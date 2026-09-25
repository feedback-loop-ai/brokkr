//! What the TUI actually draws, pinned before #288 moves `tui.rs`
//! (issue #343). `tests.rs` asserts on state and on substrings; these
//! snapshots hold every cell of every pane and level, so a layout
//! regression cannot pass unseen.
//!
//! Every frame is drawn through `TestBackend` at the two fixed sizes in
//! [`SIZES`] over the existing fixture journals of `tests.rs`, at the
//! fixture clock, with the pulse still. Every input is a fixture literal:
//! no frame carries a clock read, the environment, a temporary directory
//! or this checkout's path, so a frame is the same on Linux and macOS.
//! The text snapshots drop colour, so the status colours are asserted on
//! the buffer's own cells beside them.
//!
//! insta (decision 0014's dependency ruling of 2026-09-25) refuses to
//! write a snapshot and fails a mismatch under `CI=true` or
//! `INSTA_UPDATE=no`. A new or changed frame is written locally with
//! `INSTA_UPDATE=always`, read in its `.snap` file, and committed.

use super::tests::{
    adopting_views, at_run, at_seats, at_transcript, boxed_views, claude_reference, panel_views,
    read_of, refused_read, state_of, turns_of, views, NOW, T0,
};
use super::*;
use brokkr_core::fold::Status;
use brokkr_view::transcript::Unavailable;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

/// The issue's two fixed terminals: the common 80×20 and a wide 160×48.
const SIZES: [(u16, u16); 2] = [(80, 20), (160, 48)];

// --------------------------------------------------------------- helpers

/// One frame drawn at a fixed size: its backend displays the text the
/// snapshots hold, and its buffer keeps the styles that text drops.
fn drawn(tui: &Tui, views: &Views, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| draw(frame, tui, views)).unwrap();
    terminal
}

/// The snapshot directory by the crate's own root and the bare names,
/// so the split of #288 moving this file or its module path renames no
/// snapshot.
fn settings() -> insta::Settings {
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(concat!(env!("CARGO_MANIFEST_DIR"), "/src/tui/snapshots"));
    settings.set_prepend_module_to_snapshot(false);
    settings.set_omit_expression(true);
    settings
}

/// `name` at both [`SIZES`], as `TestBackend` displays the frame.
fn snapshot(name: &str, tui: &Tui, views: &Views) {
    for (width, height) in SIZES {
        let frame = drawn(tui, views, width, height).backend().to_string();
        let name = format!("{name}_{width}x{height}");
        settings().bind(|| insta::assert_snapshot!(name, frame));
    }
}

/// The participant level for the intake seat, as two `Enter`s from its
/// seat row leave it: the first scopes the seat, the second descends.
fn at_participant(views: &Views) -> Tui {
    let mut tui = at_seats("eff-i");
    apply(&mut tui, views, Key::Enter);
    apply(&mut tui, views, Key::Enter);
    assert_eq!(tui.level, Level::Participant);
    tui
}

fn with_read(read: TranscriptRead) -> Views {
    let mut views = views();
    views.transcript = Some(read);
    views
}

/// The participant level's checkpoint pane over one transcript read.
fn participant_snapshot(name: &str, read: TranscriptRead) {
    let views = with_read(read);
    snapshot(name, &at_participant(&views), &views);
}

/// A fleet holding one run in each of the four statuses a row can wait
/// on.
fn fleet_of_every_status() -> Views {
    let states = [
        ("run-running", state_of(Status::Running)),
        ("run-parked", state_of(Status::AwaitingOperator)),
        ("run-completed", state_of(Status::Completed)),
        ("run-stopped", state_of(Status::Stopped)),
    ];
    let entries: Vec<brokkr_view::RunEntry> = states
        .iter()
        .map(|(run_id, state)| brokkr_view::RunEntry {
            run_id,
            feature: "a run in one status",
            created_at: T0,
            state: Some(state),
            detail: None,
            residuals: &[],
        })
        .collect();
    let mut views = Views::empty();
    views.now = NOW.to_string();
    views.runs = brokkr_view::run_rows(&entries);
    views
}

// ------------------------------------------------------------ the fleet

#[test]
fn the_fleet_list_is_pinned() {
    let views = views();
    let mut tui = Tui::new(None);
    tui.cursor[0] = Some("run-7".to_string());
    snapshot("fleet", &tui, &views);
    snapshot(
        "fleet_of_every_status",
        &Tui::new(None),
        &fleet_of_every_status(),
    );
    let tabbed = Tui::over(None, vec!["alpha".to_string(), "beta".to_string()], 1);
    snapshot("fleet_tabbed", &tabbed, &views);
    tui.filter = "old".to_string();
    tui.typing = true;
    snapshot("fleet_filtered", &tui, &views);
}

// ------------------------------------------------------- the run level

#[test]
fn every_pane_of_the_run_level_is_pinned() {
    let views = views();
    let mut tui = at_run();
    snapshot("run_graph", &tui, &views);
    tui.pane = 1;
    tui.cursor[1] = Some("eff-i".to_string());
    snapshot("run_seats", &tui, &views);
    tui.pane = 2;
    tui.cursor[2] = Some(keys_for(&tui, &views)[0].clone());
    snapshot("run_trail", &tui, &views);
    apply(&mut tui, &views, Key::Enter);
    snapshot("run_trail_reader", &tui, &views);
    let panel = panel_views();
    let mut stopped = Tui::new(Some("run-stopped".to_string()));
    apply(&mut stopped, &panel, Key::Right);
    snapshot("run_stopped_panel", &stopped, &panel);
}

// ------------------------------------------------ the participant level

#[test]
fn every_pane_of_the_participant_level_is_pinned() {
    let views = views();
    snapshot("participant_checkpoints", &at_participant(&views), &views);
    let views = with_read(read_of(turns_of(3), false));
    let mut tui = at_transcript(&views);
    snapshot("participant_transcript", &tui, &views);
    apply(&mut tui, &views, Key::Down);
    snapshot("participant_transcript_turn", &tui, &views);
    apply(&mut tui, &views, Key::Enter);
    snapshot("participant_transcript_reader", &tui, &views);
}

// ------------------------------------------------------- every notice

#[test]
fn every_notice_kind_is_pinned() {
    snapshot("notice_run_notes", &at_run(), &adopting_views());
    snapshot("notice_boundary", &at_run(), &boxed_views("harness", true));
    let mut noisy = read_of(turns_of(1), true);
    noisy.skipped_lines = 2;
    noisy.unrecognized_records = 3;
    noisy.notices = brokkr_view::transcript::notices(true, 2, 3);
    participant_snapshot("notice_transcript_counts", noisy);
    participant_snapshot("notice_truncated_empty", read_of(Vec::new(), true));
    let refused = refused_read(
        claude_reference("abcd-1234", "/home/operator/.claude/projects"),
        Unavailable::NotFound,
        "no retained transcript file was found for the selected reference",
        None,
        Some("full session: claude --resume abcd-1234"),
    );
    participant_snapshot("notice_refused", refused);
    let mut tui = at_run();
    tui.status = Some("the journal is not readable right now: locked".to_string());
    snapshot("notice_status", &tui, &views());
}

// ------------------------------------------------------------ overlays

#[test]
fn the_help_overlay_and_the_too_small_frame_are_pinned() {
    let views = views();
    let mut tui = at_run();
    tui.help = true;
    snapshot("help", &tui, &views);
    let frame = drawn(&at_run(), &views, MIN_WIDTH - 1, MIN_HEIGHT)
        .backend()
        .to_string();
    settings().bind(|| insta::assert_snapshot!("too_small", frame));
}

// ------------------------------------------------------ keyboard flows

/// One line per key: what the key returned, where the state machine
/// stands, and the status line and footer it draws. The pure `apply`
/// alone, so a flow pins navigation without a terminal.
fn flow(tui: &mut Tui, views: &Views, keys: &[Key]) -> String {
    let mut lines = vec![format!("start  {}", place(tui, views))];
    for key in keys {
        let outcome = apply(tui, views, *key);
        lines.push(format!("{key:?} -> {outcome:?}  {}", place(tui, views)));
    }
    lines.join("\n")
}

fn place(tui: &Tui, views: &Views) -> String {
    let reading = tui.reading.as_deref().and_then(|text| text.lines().next());
    format!(
        "level {:?} · pane {} · cursor {:?} · turn {:?} · reading {:?}\n    status  {}\n    footer  {}",
        tui.level,
        tui.pane,
        tui.cursor,
        tui.turn,
        reading,
        status_line(tui),
        footer_for(tui, views),
    )
}

fn flow_snapshot(name: &str, tui: &mut Tui, views: &Views, keys: &[Key]) {
    let text = flow(tui, views, keys);
    settings().bind(|| insta::assert_snapshot!(name, text));
}

#[test]
fn each_keyboard_flow_is_pinned() {
    let views = with_read(read_of(turns_of(2), false));
    let mut tui = Tui::new(None);
    flow_snapshot(
        "flow_open_a_run",
        &mut tui,
        &views,
        &[Key::Down, Key::Enter],
    );
    let drill = [Key::Tab, Key::Down, Key::Enter, Key::Enter];
    flow_snapshot("flow_drill_into_a_participant", &mut tui, &views, &drill);
    let read = [Key::Tab, Key::Down, Key::Enter, Key::Down, Key::Escape];
    flow_snapshot("flow_read_a_transcript", &mut tui, &views, &read);
    let back = [
        Key::Escape,
        Key::Escape,
        Key::Escape,
        Key::Escape,
        Key::Escape,
    ];
    flow_snapshot("flow_go_back", &mut tui, &views, &back);
}

// ------------------------------------------------------ status colours

/// The style of the first cell of `text` on the row that names `run`.
/// `text` is what the 80-column status cell shows, so the one needle
/// finds it at both sizes.
fn style_of(buffer: &Buffer, run: &str, text: &str) -> Style {
    for row in 0..buffer.area.height {
        let line: String = (0..buffer.area.width)
            .map(|column| buffer[(column, row)].symbol())
            .collect();
        if !line.contains(run) {
            continue;
        }
        let byte = line
            .find(&format!(" {text}"))
            .expect("the status is on the run's row");
        let column = u16::try_from(line[..=byte].chars().count()).unwrap();
        return buffer[(column, row)].style();
    }
    panic!("no row names {run}");
}

/// The text snapshots drop colour, so the colour each status row wears
/// is asserted on its cells: green for done, red only for stopped, bold
/// while it works and dim while it waits on the operator.
#[test]
fn each_status_row_wears_its_own_colour() {
    let views = fleet_of_every_status();
    for (width, height) in SIZES {
        let terminal = drawn(&Tui::new(None), &views, width, height);
        let cases = [
            ("run-running", "running", Color::Reset, Modifier::BOLD),
            ("run-parked", "awaiting_", Color::Reset, Modifier::DIM),
            (
                "run-completed",
                "completed",
                Color::Green,
                Modifier::empty(),
            ),
            ("run-stopped", "stopped", Color::Red, Modifier::empty()),
        ];
        for (run, status, fg, modifier) in cases {
            let style = style_of(terminal.backend().buffer(), run, status);
            assert_eq!(
                (style.fg, style.add_modifier),
                (Some(fg), modifier),
                "{run} at {width}x{height}"
            );
        }
    }
}
