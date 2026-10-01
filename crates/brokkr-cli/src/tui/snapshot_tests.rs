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
//! The text snapshots drop style, so the status colours, the focused
//! pane's border and the runs table's selection are asserted on the
//! buffer's own cells beside them (issue #414).
//!
//! insta (decision 0014's dependency ruling of 2026-09-25) refuses to
//! write a snapshot and fails a mismatch under `CI=true` or
//! `INSTA_UPDATE=no`. A new or changed frame is written locally with
//! `INSTA_UPDATE=always`, read in its `.snap` file, and committed.

use super::columns_tests::{
    fleet_reading, held_view, returned_view, reviewing_view, selecting, shipped_view, OPERATOR,
    REVIEWING, SHIPPED,
};
use super::fleet_tests::{fleet_to_act_on, fleet_with_a_dead_run, opened_on, HELD};
use super::tests::{
    adopting_views, at_run, at_seats, at_transcript, boxed_views, cell_at, claude_reference, drawn,
    lines_of, panel_views, read_of, refused_read, state_of, turns_of, views, NOW, T0,
};
use super::*;
use brokkr_core::fold::Status;
use brokkr_view::transcript::Unavailable;
use ratatui::buffer::Buffer;

/// The issue's two fixed terminals: the common 80×20 and a wide 160×48.
const SIZES: [(u16, u16); 2] = [(80, 20), (160, 48)];

/// A 4K-class terminal, the one #491 was measured on: wide enough for
/// the fleet's detail pane.
const WIDE: (u16, u16) = (320, 80);

// --------------------------------------------------------------- helpers

/// The snapshot directory by the crate's own root and the bare names,
/// so the split of #288 moving this file or its module path renames no
/// snapshot.
pub(super) fn settings() -> insta::Settings {
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
            last_recorded_at: None,
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

/// #491's fleet: every section, the count line, and on a 4K-class frame
/// the list alone and the list beside the selected run's columns (#503).
#[test]
fn the_fleet_by_who_must_act_is_pinned() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(None);
    snapshot("fleet_sections", &tui, &views);
    let (width, height) = WIDE;
    // The width the shell measures before it draws a frame this wide.
    tui.width = width;
    let frame = drawn(&tui, &views, width, height).backend().to_string();
    settings().bind(|| insta::assert_snapshot!("fleet_320x80", frame));
    tui.cursor[0] = Some("cargo-exemption-hold-5b6c7d8e".to_string());
    let frame = drawn(&tui, &views, width, height).backend().to_string();
    settings().bind(|| insta::assert_snapshot!("fleet_detail_320x80", frame));
}

/// #503 on the operator's terminal: the frame `brokkr tui` opens on with
/// no key pressed, the first run that needs the operator selected — at
/// 330x60 and 420x110 its dashboard and live column beside the list, at
/// 220x50 its dashboard alone, the age and id at the list's edge; and a
/// running run opened in place.
#[test]
fn the_fleet_on_the_operators_terminal_is_pinned() {
    let (_, frame) = opened_on(fleet_with_a_dead_run, 330, 60);
    settings().bind(|| insta::assert_snapshot!("fleet_open_330x60", frame));
    let views = fleet_with_a_dead_run();
    for (width, height) in [(220, 50), (420, 110)] {
        let (_, frame) = opened_on(fleet_with_a_dead_run, width, height);
        let name = format!("fleet_{width}x{height}");
        settings().bind(|| insta::assert_snapshot!(name, frame));
    }
    let mut tui = Tui::new(None);
    tui.cursor[0] = Some("fix-403-on-macos-round-9-3c1f9a02".to_string());
    apply(&mut tui, &views, Key::Right);
    snapshot("fleet_opened_in_place", &tui, &views);
}

/// #503's second round on the operator's terminal: the fleet in three
/// columns four ways — both, `d`'s dashboard only, `f`'s column only, and
/// neither — over a run parked on its review; and the third column of a
/// running run and of a finished one.
#[test]
fn the_fleet_in_three_columns_is_pinned() {
    let (width, height) = OPERATOR;
    let views = fleet_reading(held_view());
    let ways = [
        ("both", true, true),
        ("d_only", true, false),
        ("f_only", false, true),
        ("neither", false, false),
    ];
    for (way, dashboard, live) in ways {
        let mut tui = selecting(HELD);
        tui.toggles = Toggles { dashboard, live };
        let frame = drawn(&tui, &views, width, height).backend().to_string();
        let name = format!("fleet_{way}_{width}x{height}");
        settings().bind(|| insta::assert_snapshot!(name, frame));
    }
    selected_and_pinned([
        ("fleet_live", REVIEWING, reviewing_view()),
        ("fleet_findings", SHIPPED, shipped_view()),
    ]);
}

/// Each `(name, run, view)` pinned at the operator's terminal: the fleet
/// with `run` selected and `view` as the shell's read of it.
fn selected_and_pinned<const N: usize>(pins: [(&str, &str, RunView); N]) {
    let (width, height) = OPERATOR;
    for (name, run, view) in pins {
        let views = fleet_reading(view);
        let frame = drawn(&selecting(run), &views, width, height);
        let (name, frame) = (
            format!("{name}_{width}x{height}"),
            frame.backend().to_string(),
        );
        settings().bind(|| insta::assert_snapshot!(name, frame));
    }
}

/// #508 on the operator's terminal: the dashboard draws the run's graph
/// in place of its path and seats — a running run's current phase boxed,
/// and a finished run's road back drawn as the loop-back arrow.
#[test]
fn the_dashboards_graph_is_pinned() {
    selected_and_pinned([
        ("fleet_graph_running", REVIEWING, reviewing_view()),
        ("fleet_graph_returned", SHIPPED, returned_view()),
    ]);
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
    participant_snapshot("notice_empty", read_of(Vec::new(), false));
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
    // The fleet lists the run that needs you first (#491), so the second
    // row is the one this flow opens.
    flow_snapshot(
        "flow_open_a_run",
        &mut tui,
        &views,
        &[Key::Down, Key::Down, Key::Enter],
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

/// The first row of a drawn frame that shows `text`.
fn row_naming(lines: &[String], text: &str) -> usize {
    lines
        .iter()
        .position(|line| line.contains(text))
        .unwrap_or_else(|| panic!("no row names {text}"))
}

/// The style of the first cell of `text` on the row that names `run`.
/// `text` is what the 80-column status cell shows, so the one needle
/// finds it at both sizes.
fn style_of(buffer: &Buffer, run: &str, text: &str) -> Style {
    let lines = lines_of(buffer);
    let (column, row) = cell_at(&lines, row_naming(&lines, run), &format!(" {text}"));
    buffer[(column + 1, row)].style()
}

/// The text snapshots drop colour, so the colour each status row wears
/// is asserted on its cells: green for done, red only for stopped, bold
/// while it works and dim while it waits on the operator. Since #491 the
/// cell reads the run's standing, or its phase while it runs.
#[test]
fn each_status_row_wears_its_own_colour() {
    let views = fleet_of_every_status();
    for (width, height) in SIZES {
        let terminal = drawn(&Tui::new(None), &views, width, height);
        let cases = [
            ("run-running", "design", Color::Reset, Modifier::BOLD),
            ("run-parked", "parked", Color::Reset, Modifier::DIM),
            ("run-completed", "shipped", Color::Green, Modifier::empty()),
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

// ---------------------------------------------------- focus and selection

/// The modifiers the pane titled `title` wears on its top-left corner
/// and on its title's first cell: its border style and its title's.
fn border_of(buffer: &Buffer, lines: &[String], title: &str) -> (Modifier, Modifier) {
    let needle = format!("┌{title}─");
    let (column, row) = cell_at(lines, row_naming(lines, &needle), &needle);
    (
        buffer[(column, row)].modifier,
        buffer[(column + 1, row)].modifier,
    )
}

/// What every pane of a level wears with its pane `focus` focused: bold
/// on the focused one, dim on every other, and dim on each pane focus
/// never reaches.
fn borders_wanted<'a>(
    panes: &[&'a str],
    never: &[&'a str],
    focus: usize,
) -> Vec<(&'a str, (Modifier, Modifier))> {
    let worn = |focused: bool| match focused {
        true => (Modifier::BOLD, Modifier::BOLD),
        false => (Modifier::DIM, Modifier::DIM),
    };
    let focusable = panes
        .iter()
        .enumerate()
        .map(|(index, title)| (*title, worn(index == focus)));
    let dim = never.iter().map(|title| (*title, worn(false)));
    focusable.chain(dim).collect()
}

/// A level as the focus test reaches it: the state that draws it, the
/// panes its focus moves across in order, and the panes it never reaches.
type Panes = (
    fn(&Views) -> Tui,
    &'static [&'static str],
    &'static [&'static str],
);

/// The border is the only focus affordance (`pane`), and the text
/// snapshots drop it, so a level that focused the wrong pane would pass
/// them all. Every pane at every level, under each focus the level can
/// take, is asserted on the buffer's own cells at both sizes.
#[test]
fn only_the_focused_pane_wears_a_bold_border() {
    let views = views();
    let levels: [Panes; 3] = [
        (|_| Tui::new(None), &["runs"], &[]),
        (|_| at_run(), &["graph", "seats", "trail"], &[]),
        (at_participant, &["checkpoints", "transcript"], &["seat"]),
    ];
    for (level, panes, never) in levels {
        for (focus, (width, height)) in (0..panes.len()).flat_map(|f| SIZES.map(|s| (f, s))) {
            let mut tui = level(&views);
            tui.pane = focus;
            let terminal = drawn(&tui, &views, width, height);
            let buffer = terminal.backend().buffer();
            let lines = lines_of(buffer);
            let worn: Vec<_> = (panes.iter().chain(never))
                .map(|title| (*title, border_of(buffer, &lines, title)))
                .collect();
            let wanted = borders_wanted(panes, never, focus);
            assert_eq!(
                worn, wanted,
                "{:?} pane {focus} at {width}x{height}",
                tui.level
            );
        }
    }
}

/// The runs table's selection is `REVERSED` across the cursor's whole
/// row, between the pane's borders, and on no cell of any other row.
#[test]
fn the_selected_run_row_is_reversed_from_border_to_border() {
    let views = fleet_of_every_status();
    let mut tui = Tui::new(None);
    tui.cursor[0] = Some("run-parked".to_string());
    let runs = ["run-stopped", "run-completed", "run-parked", "run-running"];
    for (width, height) in SIZES {
        let terminal = drawn(&tui, &views, width, height);
        let buffer = terminal.backend().buffer();
        let lines = lines_of(buffer);
        let reversed = |run: &'static str| {
            let row = u16::try_from(row_naming(&lines, run)).unwrap();
            let inner = 1..width - 1;
            let cells =
                inner.filter(|column| buffer[(*column, row)].modifier.contains(Modifier::REVERSED));
            (run, cells.count())
        };
        let wanted = runs.map(|run| match run == "run-parked" {
            true => (run, usize::from(width - 2)),
            false => (run, 0),
        });
        assert_eq!(runs.map(reversed), wanted, "at {width}x{height}");
    }
}
