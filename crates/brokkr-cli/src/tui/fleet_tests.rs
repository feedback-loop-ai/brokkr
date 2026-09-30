//! The fleet view of #491, headless: the sections and the count line
//! the older runs fold into, `a`, the filter over every run, the rows
//! that print a title and never a feature, and the detail pane a wide
//! frame gains beside the list.

use super::tests::{drawn, frame_of, lines_of, script, state_of, test_ops, NOW, T0, TERMINAL};
use super::*;
use brokkr_core::fold::{RunState, Status};
use brokkr_view::ResidualFinding;
use ratatui::backend::TestBackend;
use serde_json::{json, Value};

/// Two days before the fixture clock: a run the fleet folds away.
const OLDER: &str = "2025-12-30T00:07:03Z";

/// The run the detail tests select: parked on a ruling, with a residual.
const HELD: &str = "cargo-exemption-hold-5b6c7d8e";

fn ruled(status: Status, phase: &str, decision: Value) -> RunState {
    let mut state = state_of(status);
    state.phase = Some(phase.to_string());
    state.last_decision = Some(decision);
    state
}

fn finding(run_id: &str, value: &str) -> ResidualFinding {
    ResidualFinding {
        run_id: run_id.to_string(),
        seq: 40,
        phase: "review".to_string(),
        rule_id: "REVIEW-RESIDUAL-OK".to_string(),
        input: "max_residual_severity".to_string(),
        value: value.to_string(),
        line: format!("{run_id} seq 40 · review · max_residual_severity: {value}"),
        superseded: None,
    }
}

/// The feature the held run was commissioned with: a title line, a
/// paragraph longer than the pane wraps at, and a list.
const HELD_FEATURE: &str = "#362 cargo exemption\n\
\n\
The review held the change on an unverified security residual: the exemption lets cargo deny pass a crate whose licence nobody read, and the reviewer could not verify the audit the implementer cited.\n\
\n\
What was asked:\n\
- keep the exemption narrow\n\
- cite the audit\n\
- name the follow-up";

/// A fleet with a run in every place the view can put one: two that
/// need the operator, two running, three finished today and two older
/// ones the list folds into its count line. Listed oldest first, as the
/// store lists them.
pub(super) fn fleet_to_act_on() -> Views {
    let decided = |rule: &str, from: &str, next: Option<&str>| json!({"rule_id": rule, "from": from, "next": next, "result": "ruled"});
    let shipped = ruled(
        Status::Completed,
        "done",
        decided("SHIP-COMPLETE", "ship", Some("done")),
    );
    let blocked = ruled(
        Status::Stopped,
        "stop",
        decided("IMPL-BLOCKED", "implement", Some("stop")),
    );
    let exhausted = ruled(
        Status::Stopped,
        "stop",
        decided("VERIFY-FAIL-EXHAUSTED", "verify", Some("stop")),
    );
    let triage = ruled(
        Status::Running,
        "triage",
        decided("INTAKE-OK", "intake", Some("triage")),
    );
    let review = ruled(
        Status::Running,
        "review",
        decided("IMPL-OK", "implement", Some("review")),
    );
    let mut held = ruled(
        Status::AwaitingOperator,
        "review",
        decided("REVIEW-UNVERIFIED-SECURITY", "review", None),
    );
    held.park_reason = Some("REVIEW-UNVERIFIED-SECURITY for (review, clean)".to_string());
    let low = [finding("landing-4-of-the-fleet-2a3b4c5d", "low")];
    let high = [finding(HELD, "high")];
    let entry = |run_id, feature, created_at, state, residuals| brokkr_view::RunEntry {
        run_id,
        feature,
        created_at,
        last_recorded_at: None,
        state,
        detail: None,
        residuals,
    };
    let mut broken = entry(
        "journal-that-broke-7f8e9d0c",
        "A journal that does not fold",
        T0,
        None,
        &[],
    );
    broken.detail = Some("event 12: event after terminal status");
    let entries = [
        entry("landing-2-of-the-fleet-0a1b2c3d", "Landing 2 of the fleet view", OLDER, Some(&shipped), &[]),
        entry(
            "unit-18-of-the-rebuild-4e5f6a7b",
            "Unit 18 of the rebuild: the smith hit a permission wall\n\nsecond line",
            OLDER,
            Some(&blocked),
            &[],
        ),
        entry("0065-rebuild-unit-19-8c9d0e1f", "0065 rebuild unit 19", T0, Some(&exhausted), &[]),
        entry("landing-4-of-the-fleet-2a3b4c5d", "Landing 4 of the fleet view", T0, Some(&shipped), &low),
        entry("landing-5-of-the-fleet-6e7f8a9b", "Landing 5 of the fleet view", T0, Some(&shipped), &[]),
        entry("0065-rebuild-unit-20-1c2d3e4f", "0065 rebuild unit 20", T0, Some(&triage), &[]),
        entry(
            "fix-403-on-macos-round-9-3c1f9a02",
            "#403 macOS fix, round 9: PR #483's test (macos-latest) job fails four protocol tests\n\
             \n\
             The runner's temporary directory is a symlink.",
            T0,
            Some(&review),
            &[],
        ),
        entry(HELD, HELD_FEATURE, T0, Some(&held), &high),
        broken,
    ];
    let mut views = Views::empty();
    views.now = NOW.to_string();
    views.runs = brokkr_view::run_rows(&entries);
    views
}

/// The first row of `lines` that shows `text`.
fn row_of(lines: &[String], text: &str) -> usize {
    lines
        .iter()
        .position(|line| line.contains(text))
        .unwrap_or_else(|| panic!("no row shows {text}:\n{}", lines.join("\n")))
}

/// The fleet's own list, in the order `j` walks it.
fn listed(tui: &Tui, views: &Views) -> Vec<String> {
    keys_for(tui, views)
}

// ----------------------------------------------------------- the sections

/// Acceptance 3: the three sections render in order — who needs you,
/// what runs, what finished today — and the older runs fold into one
/// line that counts them. `j` walks the same order the frame shows.
#[test]
fn the_sections_render_in_order_and_the_older_runs_fold_into_a_count() {
    let views = fleet_to_act_on();
    let tui = Tui::new(None);
    let lines = lines_of(drawn(&tui, &views, 100, 30).backend().buffer());
    let rows = [
        "needs you",
        "journ…7f8e9d0c",
        "cargo…5b6c7d8e",
        "running",
        "fix-4…3c1f9a02",
        "0065-…1c2d3e4f",
        "last 24h",
        "landi…6e7f8a9b",
        "landi…2a3b4c5d",
        "0065-…8c9d0e1f",
        "2 hidden · a shows them",
    ]
    .map(|text| row_of(&lines, text));
    assert!(rows.is_sorted(), "{rows:?}:\n{}", lines.join("\n"));
    assert_eq!(rows[10], rows[9] + 1, "the count is the list's last line");
    let frame = lines.join("\n");
    for hidden in ["0a1b2c3d", "4e5f6a7b"] {
        assert!(!frame.contains(hidden), "{hidden} is folded:\n{frame}");
    }
    assert_eq!(
        listed(&tui, &views),
        [
            "journal-that-broke-7f8e9d0c",
            HELD,
            "fix-403-on-macos-round-9-3c1f9a02",
            "0065-rebuild-unit-20-1c2d3e4f",
            "landing-5-of-the-fleet-6e7f8a9b",
            "landing-4-of-the-fleet-2a3b4c5d",
            "0065-rebuild-unit-19-8c9d0e1f",
        ]
    );
}

/// Acceptance 3: `a` lists the older runs under their own heading and
/// says how to fold them again; a second `a` folds them. Away from the
/// fleet `a` is a character nothing binds.
#[test]
fn a_shows_the_older_runs_and_a_second_a_folds_them_again() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(None);
    assert!(footer_for(&tui, &views).contains("· a all runs ·"));
    apply(&mut tui, &views, Key::Char('a'));
    assert!(tui.all);
    assert!(footer_for(&tui, &views).contains("· a recent only ·"));
    let frame = frame_of(&tui, &views, 100, 30);
    assert!(!frame.contains("hidden ·"), "{frame}");
    let older = frame.lines().position(|line| line.contains("│older"));
    let first = frame
        .lines()
        .position(|line| line.contains("unit-…4e5f6a7b"));
    assert_eq!(older.map(|row| row + 1), first, "{frame}");
    assert!(frame.contains("✗ stopped     BLOCKED"), "{frame}");
    assert_eq!(listed(&tui, &views).len(), 9);
    apply(&mut tui, &views, Key::Char('a'));
    assert_eq!(listed(&tui, &views).len(), 7);
    let mut run = Tui::new(Some("run-7".to_string()));
    apply(&mut run, &views, Key::Char('a'));
    assert!(!run.all, "the run level binds no `a`");
}

/// Acceptance 3: the filter searches every run, the folded ones too, by
/// title or by id — never by the feature past its title.
#[test]
fn the_filter_finds_a_folded_run_by_its_title_or_its_id() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(None);
    tui.filter = "permission wall".to_string();
    assert_eq!(listed(&tui, &views), ["unit-18-of-the-rebuild-4e5f6a7b"]);
    tui.filter = "0a1b2c3d".to_string();
    assert_eq!(listed(&tui, &views), ["landing-2-of-the-fleet-0a1b2c3d"]);
    let frame = frame_of(&tui, &views, 100, 30);
    assert!(frame.contains("landi…0a1b2c3d"), "{frame}");
    assert!(
        !frame.contains("hidden ·"),
        "a filter folds nothing:\n{frame}"
    );
    tui.filter = "second line".to_string();
    assert_eq!(listed(&tui, &views), Vec::<String>::new());
}

// --------------------------------------------------------------- the rows

/// Acceptance 1 and 2 on a 4K-class frame: a row prints its title and no
/// more of the feature, and no id is cut inside its hash.
#[test]
fn a_row_prints_its_title_and_an_id_whose_hash_is_whole() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(None);
    tui.all = true;
    let frame = frame_of(&tui, &views, 320, 80);
    let title = "#403 macOS fix, round 9: PR #483's test (macos-latest) job…";
    assert!(frame.contains(title), "{frame}");
    for past in ["fails four", "symlink", "second line", "What was asked"] {
        assert!(!frame.contains(past), "{past} is past a title:\n{frame}");
    }
    for row in &views.runs.runs {
        let hash = &row.run_id[row.run_id.len() - 8..];
        assert!(
            frame.contains(hash),
            "{} keeps {hash}:\n{frame}",
            row.run_id
        );
    }
}

/// Acceptance 1 against the sanitizer: a family emoji is one 2-column
/// sequence only while its joiners hold, and the TUI strips joiners, so
/// thirty of them measure 60 columns in the view and paint 240 here.
/// The row paints its title clamped as drawn: 29 emoji and an ellipsis.
#[test]
fn a_title_is_clamped_as_it_is_painted_once_its_joiners_are_stripped() {
    let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
    let feature = family.repeat(30);
    let state = state_of(Status::Running);
    let entry = brokkr_view::RunEntry {
        run_id: "a-family-of-emoji-5d6e7f80",
        feature: &feature,
        created_at: T0,
        last_recorded_at: None,
        state: Some(&state),
        detail: None,
        residuals: &[],
    };
    let mut views = Views::empty();
    views.now = NOW.to_string();
    views.runs = brokkr_view::run_rows(&[entry]);
    assert_eq!(views.runs.runs[0].title, feature, "the view measures 60");
    let frame = frame_of(&Tui::new(None), &views, 320, 80);
    let painted = frame
        .chars()
        .filter(|c| family.contains(*c) && *c != '\u{200D}');
    assert_eq!(painted.count(), 29, "{frame}");
    assert!(frame.contains('…'), "{frame}");
}

#[test]
fn a_long_id_is_shortened_in_its_head_and_never_in_its_hash() {
    assert_eq!(short_id("run-unfoldable"), "run-unfoldable");
    assert_eq!(
        short_id("implement-story-491-the-tui-s-fl-b7707183"),
        "imple…b7707183"
    );
    assert_eq!(short_id("run-unfoldable-1"), "run-u…ldable-1");
}

// ---------------------------------------------------------- the detail pane

/// Acceptance 5's frame, asked of its cells: the detail pane stands
/// beside the list from [`DETAIL_MIN_WIDTH`] columns, and only over a
/// selection; the list keeps the frame below it and with nothing
/// selected. Each frame is drawn at the width the shell measured for it.
#[test]
fn the_detail_pane_needs_the_width_and_a_selection() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(None);
    tui.width = 320;
    assert!(!frame_of(&tui, &views, 320, 80).contains("rule      "));
    tui.cursor[0] = Some(HELD.to_string());
    tui.width = DETAIL_MIN_WIDTH - 1;
    assert!(!frame_of(&tui, &views, DETAIL_MIN_WIDTH - 1, 30).contains(HELD));
    tui.width = DETAIL_MIN_WIDTH;
    let frame = frame_of(&tui, &views, DETAIL_MIN_WIDTH, 30);
    let wanted = [
        HELD,
        "parked · review · 7m03s",
        "verdict   UNVERIFIED-SE…",
        "rule      REVIEW-UNVERIFIED-SECURITY",
        "residual  high",
        "parked    REVIEW-UNVERIFIED-SECURITY for (review, clean)",
        "cargo-exemption-hold-5b6c7d8e seq 40 · review · max_residual_severity: high",
        "What was asked:",
    ];
    for text in wanted {
        assert!(frame.contains(text), "{text}:\n{frame}");
    }
    // The list beside the pane holds a whole title, sixty columns of it.
    let title = "#403 macOS fix, round 9: PR #483's test (macos-latest) job…";
    assert!(frame.contains(&format!("{title}  ")), "{frame}");
    let top = frame.lines().next().unwrap();
    let list_edge = usize::from(LIST_COLUMNS) - 1;
    assert_eq!(
        top.chars().position(|c| c == '┐'),
        Some(list_edge),
        "{frame}"
    );
    tui.cursor[0] = Some("journal-that-broke-7f8e9d0c".to_string());
    tui.width = 320;
    let frame = frame_of(&tui, &views, 320, 80);
    assert!(
        frame.contains("journal   event 12: event after terminal status"),
        "{frame}"
    );
    assert!(frame.contains("quarantined · — · 7m03s"), "{frame}");
    assert!(frame.contains("verdict   does not fold"), "{frame}");
}

/// `Tab` reaches the detail pane only while it is on the frame; there
/// the list keys scroll it a drawn line at a time, the run's own lines
/// first, `Enter` still opens the run, and moving the list reads the
/// next run from the top.
#[test]
fn tab_focuses_the_detail_pane_which_scrolls_and_enter_still_opens_the_run() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(None);
    tui.cursor[0] = Some(HELD.to_string());
    tui.width = DETAIL_MIN_WIDTH - 1;
    apply(&mut tui, &views, Key::Tab);
    assert_eq!(tui.pane, 0, "no detail pane, nothing to tab to");
    tui.width = DETAIL_MIN_WIDTH;
    assert!(footer_for(&tui, &views).contains("Enter open run · Tab detail · a all runs"));
    apply(&mut tui, &views, Key::Tab);
    assert_eq!(tui.pane, 1);
    assert!(footer_for(&tui, &views).starts_with("↑↓/jk scroll · Enter open run · Tab list"));
    // Nine lines of the run's own, then the feature's first line.
    for _ in 0..10 {
        apply(&mut tui, &views, Key::Char('j'));
    }
    assert_eq!((tui.offset, tui.cursor[0].as_deref()), (10, Some(HELD)));
    // The title stays on the list's row and the pane's border; the body
    // has scrolled past the run's lines and the feature's first line.
    let frame = frame_of(&tui, &views, DETAIL_MIN_WIDTH, 30);
    assert!(frame.contains("The review held"), "{frame}");
    assert!(!frame.contains("verdict   "), "{frame}");
    assert_eq!(frame.matches("#362 cargo exemption").count(), 2, "{frame}");
    apply(&mut tui, &views, Key::Char('G'));
    assert_eq!(tui.offset, 17, "nine lines, and the feature's nine drawn");
    tui.width = DETAIL_MIN_WIDTH - 1;
    assert!(
        footer_for(&tui, &views).starts_with("↑↓/jk move"),
        "narrowed, the list's"
    );
    tui.width = DETAIL_MIN_WIDTH;
    apply(&mut tui, &views, Key::Tab);
    apply(&mut tui, &views, Key::Char('j'));
    assert_eq!(
        (tui.offset, tui.pane),
        (0, 0),
        "a new selection reads from the top"
    );
    assert_eq!(
        tui.cursor[0].as_deref(),
        Some("fix-403-on-macos-round-9-3c1f9a02")
    );
    apply(&mut tui, &views, Key::Char('k'));
    apply(&mut tui, &views, Key::Tab);
    apply(&mut tui, &views, Key::Enter);
    assert_eq!((tui.level, tui.run.as_deref()), (Level::Run, Some(HELD)));
}

/// The border is the only focus affordance (#414): with the detail pane
/// on the frame, the bold border is the list's or the pane's, never both.
#[test]
fn the_focused_fleet_pane_wears_the_bold_border() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(None);
    tui.cursor[0] = Some(HELD.to_string());
    tui.width = 320;
    let worn = [
        (0, Modifier::BOLD, Modifier::DIM),
        (1, Modifier::DIM, Modifier::BOLD),
    ];
    for (pane, list, detail) in worn {
        tui.pane = pane;
        let terminal = drawn(&tui, &views, 320, 80);
        let buffer = terminal.backend().buffer();
        let corners = (buffer[(0, 0)].modifier, buffer[(LIST_COLUMNS, 0)].modifier);
        assert_eq!(corners, (list, detail), "pane {pane}");
    }
}

/// The shell tells the state machine how wide the frame it just drew
/// was, so `Tab` reaches the detail pane on a wide terminal.
#[test]
fn the_shell_tells_the_keys_how_wide_its_frame_was() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let mut terminal = Terminal::new(TestBackend::new(DETAIL_MIN_WIDTH, 30)).unwrap();
    script(&[Key::Down, Key::Down, Key::Tab, Key::Quit]);
    let mut source = |_: Ask| Ok(Some(fleet_to_act_on()));
    let mut tui = Tui::new(None);
    drive(&mut terminal, &test_ops(), &mut source, &mut tui, 9).unwrap();
    assert_eq!((tui.width, tui.pane), (DETAIL_MIN_WIDTH, 1));
    assert_eq!(tui.cursor[0].as_deref(), Some(HELD));
}

/// One answer to whether the detail pane is on the frame: the shell
/// measures the frame before it draws it, so the very first frame at a
/// wide width draws the pane and its footer names the key to reach it.
#[test]
fn the_first_wide_frame_and_its_footer_agree_on_the_detail_pane() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let mut terminal = Terminal::new(TestBackend::new(DETAIL_MIN_WIDTH, 30)).unwrap();
    script(&[]);
    let mut source = |_: Ask| Ok(Some(fleet_to_act_on()));
    let mut tui = Tui::new(None);
    tui.cursor[0] = Some(HELD.to_string());
    drive(&mut terminal, &test_ops(), &mut source, &mut tui, 1).unwrap();
    let frame = lines_of(terminal.backend().buffer()).join("\n");
    assert!(frame.contains(&format!("│{HELD}")), "{frame}");
    assert!(frame.contains("· Tab detail ·"), "{frame}");
}

/// Review finding 1: the pane scrolls by the lines it draws, so a feature
/// that is one long paragraph scrolls to its last wrapped line.
#[test]
fn the_detail_pane_scrolls_a_one_paragraph_feature_to_its_last_line() {
    let feature = format!(
        "One paragraph {}the last words",
        "and more words ".repeat(200)
    );
    let running = state_of(Status::Running);
    let mut views = Views::empty();
    views.now = NOW.to_string();
    views.runs = brokkr_view::run_rows(&[brokkr_view::RunEntry {
        run_id: "one-paragraph-0a1b2c3d",
        feature: &feature,
        created_at: T0,
        last_recorded_at: None,
        state: Some(&running),
        detail: None,
        residuals: &[],
    }]);
    let mut tui = Tui::new(None);
    tui.cursor[0] = Some("one-paragraph-0a1b2c3d".to_string());
    tui.width = DETAIL_MIN_WIDTH;
    tui.pane = 1;
    let frame = frame_of(&tui, &views, DETAIL_MIN_WIDTH, 30);
    assert!(!frame.contains("the last words"), "{frame}");
    apply(&mut tui, &views, Key::Char('G'));
    assert_eq!(tui.offset, 37, "seven lines of the run's, then 31 drawn");
    let frame = frame_of(&tui, &views, DETAIL_MIN_WIDTH, 30);
    assert!(frame.contains("the last words"), "{frame}");
}

/// Review finding 3: a stopped or parked run shows its worst open
/// residual beside its ruling, as a shipped one does.
#[test]
fn every_row_names_its_ruling_and_its_worst_open_residual() {
    let views = fleet_to_act_on();
    let frame = frame_of(&Tui::new(None), &views, 160, 48);
    let wanted = [
        "● parked      UNVERIFIED-SE… high     #362 cargo exemption",
        "✓ shipped     COMPLETE       low      Landing 4 of the fleet view",
        "✓ shipped     COMPLETE                Landing 5 of the fleet view",
        "✗ stopped     FAIL-EXHAUSTED          0065 rebuild unit 19",
    ];
    for row in wanted {
        assert!(frame.contains(row), "{row}:\n{frame}");
    }
}

/// Review finding 7: `a` is the hearth's own, parked with its selection
/// and returned by a switch back (decision 0026 ruling 2).
#[test]
fn a_is_a_hearths_own_and_a_switch_back_returns_it() {
    let views = fleet_to_act_on();
    let mut tui = Tui::over(None, vec!["alpha".to_string(), "beta".to_string()], 0);
    apply(&mut tui, &views, Key::Char('a'));
    switch(&mut tui, 1);
    assert!(!tui.all, "beta folds its older runs");
    switch(&mut tui, 0);
    assert!(tui.all, "alpha's `a` is returned");
}

/// Review finding 8: the participant level's checkpoint scroll does not
/// follow the operator back up to the fleet's detail pane.
#[test]
fn the_fleet_reads_its_selection_from_the_top_after_a_run() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(Some(HELD.to_string()));
    tui.offset = 5;
    apply(&mut tui, &views, Key::Backspace);
    assert_eq!((tui.level, tui.offset), (Level::Runs, 0));
}
