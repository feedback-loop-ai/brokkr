//! The TUI frame the CPU budgets measure (#342) draws what the console
//! draws, from any journal the benchmark hands it.

use brokkr_core::envelope::EventEnvelope;

const FIXTURE: &str =
    include_str!("../../../fixtures/journals/tui-graph-the-selection-box-gets-80f98deb.ndjson");

fn fixture() -> Vec<EventEnvelope> {
    FIXTURE
        .lines()
        .map(|line| serde_json::from_str(line).expect("the fixture is an exported journal"))
        .collect()
}

/// The frame's rows, as the terminal would show them, right-trimmed.
fn lines(buffer: &ratatui::buffer::Buffer) -> Vec<String> {
    let area = buffer.area;
    (0..area.height)
        .map(|row| {
            let line: String = (0..area.width)
                .map(|column| buffer[(column, row)].symbol())
                .collect();
            line.trim_end().to_string()
        })
        .collect()
}

#[test]
fn the_budget_frame_opens_the_run_level_of_the_journal_it_is_handed() {
    let frame = brokkr_cli::run_frame_for_budget(&fixture(), "2026-09-26T00:00:00Z", 160, 48);
    assert_eq!((frame.area.width, frame.area.height), (160, 48));
    let lines = lines(&frame);
    assert_eq!(
        lines[46],
        "runs · run tui-graph-the-selection-box-gets-80f98deb"
    );
    assert!(
        lines[2].contains("⏺ intake──ᐳ⏺ implement──ᐳ⏺ verify"),
        "{}",
        lines[2]
    );
    assert!(
        lines[10].starts_with("│implement   succeeded 1        76"),
        "{}",
        lines[10]
    );
}

#[test]
fn a_journal_that_does_not_fold_still_draws_a_frame() {
    let frame = brokkr_cli::run_frame_for_budget(&[], "2026-09-26T00:00:00Z", 80, 20);
    assert_eq!((frame.area.width, frame.area.height), (80, 20));
    let lines = lines(&frame);
    assert_eq!(lines[18], "runs · run");
    assert_eq!(
        lines[7],
        "│participant status attempts turns cost tokens model boundary activity         │"
    );
    assert_eq!(lines[8], format!("└{}┘", "─".repeat(78)));
}
