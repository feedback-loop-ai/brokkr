//! The fleet view of #491, headless: the sections and the count line
//! the older runs fold into, `a`, the filter over every run, the rows
//! that print a title and never a feature, and the detail pane a wide
//! frame gains beside the list.

use super::tests::{
    cell_at, drawn, frame_of, lines_of, script, state_of, test_ops, NOW, T0, TERMINAL,
};
use super::*;
use brokkr_core::fold::{Cursor, ParkReason, Run, RunState, Status};
use brokkr_view::{Quarantine, ResidualFinding};
use ratatui::backend::TestBackend;
use serde_json::{json, Value};

/// Two days before the fixture clock: a run the fleet folds away.
pub(super) const OLDER: &str = "2025-12-30T00:07:03Z";

/// The run the detail tests select: parked on a ruling, with a residual.
pub(super) const HELD: &str = "cargo-exemption-hold-5b6c7d8e";

pub(super) fn ruled(status: Status, phase: &str, decision: Value) -> RunState {
    let mut state = state_of(status);
    state.phase = Some(phase.to_string());
    state.last_decision = Some(serde_json::from_value(decision).unwrap());
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
pub(super) const HELD_FEATURE: &str ="#362 cargo exemption\n\
\n\
The review held the change on an unverified security residual: the exemption lets cargo deny pass a crate whose licence nobody read, and the reviewer could not verify the audit the implementer cited.\n\
\n\
What was asked:\n\
- keep the exemption narrow\n\
- cite the audit\n\
- name the follow-up";

/// A run in review after `decision`, its second attempt in flight with
/// the reviewer (#503).
fn reviewing(decision: Value) -> RunState {
    let mut review = ruled(Status::Running, "review", decision);
    review.run = Run::Running(Cursor::EffectInFlight {
        effect_id: "e9".to_string(),
        attempt_id: "a9".to_string(),
        seat: "reviewer".to_string(),
        failed_attempts: 1,
        stop_rides: false,
    });
    review
}

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
    let review = reviewing(decided("IMPL-OK", "implement", Some("review")));
    let mut held = ruled(
        Status::AwaitingOperator,
        "review",
        decided("REVIEW-UNVERIFIED-SECURITY", "review", None),
    );
    held.run = Run::AwaitingOperator {
        reason: ParkReason::read("REVIEW-UNVERIFIED-SECURITY for (review, clean)".to_string()),
    };
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
    let refusal = "event 12: event after terminal status";
    broken.detail = Some(Quarantine::DoesNotFold.in_words(refusal));
    let mut entries = [
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
    // The review's journal moved two minutes ago (#503).
    entries[6].last_recorded_at = Some("2026-01-01T00:05:00Z");
    let mut views = Views::empty();
    views.now = NOW.to_string();
    views.runs = brokkr_view::run_rows(&entries);
    views
}

/// The run the operator found (#503): its journal folds to running and
/// has not moved for 116 hours, so nothing drives it.
pub(super) const DEAD: &str = "landing-pr-420-of-the-fleet-9d8c7b6a";

/// [`fleet_to_act_on`] with [`DEAD`] newest, as a fleet read lists it.
pub(super) fn fleet_with_a_dead_run() -> Views {
    let mut landing = state_of(Status::Running);
    landing.phase = Some("land".to_string());
    let dead = brokkr_view::RunEntry {
        run_id: DEAD,
        feature: "LANDING PR #420: the fleet lists who must act first",
        created_at: "2025-12-25T00:00:00Z",
        last_recorded_at: Some("2025-12-27T04:00:00Z"),
        state: Some(&landing),
        detail: None,
        residuals: &[],
    };
    let mut views = fleet_to_act_on();
    let row = brokkr_view::run_rows(&[dead]).runs.remove(0);
    views.runs.runs.insert(0, row);
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

/// The dashboard's lines for `row`, at a width that wraps none of them.
fn dashboard_of(views: &Views, row: &RunRow) -> Vec<String> {
    let lines = dashboard_lines(&Tui::new(None), views, row, 200);
    lines.iter().map(Line::to_string).collect()
}

/// The dashboard lines under `heading`, to the blank line that ends them.
pub(super) fn under(lines: &[String], heading: &str) -> Vec<String> {
    let start = lines.iter().position(|line| line == heading).unwrap() + 1;
    let section = lines[start..].iter().take_while(|line| !line.is_empty());
    section.cloned().collect()
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
        "2 older runs",
    ]
    .map(|text| row_of(&lines, text));
    assert!(rows.is_sorted(), "{rows:?}:\n{}", lines.join("\n"));
    assert_eq!(rows[10], rows[9] + 1, "the count is the list's last line");
    assert!(
        lines[rows[10]].contains("2 older runs: press a to show"),
        "{}",
        lines[rows[10]]
    );
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
    assert!(!frame.contains("2 older runs"), "{frame}");
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
        !frame.contains("older runs"),
        "a filter folds nothing:\n{frame}"
    );
    tui.filter = "second line".to_string();
    assert_eq!(listed(&tui, &views), Vec::<String>::new());
}

// --------------------------------------------------------------- the rows

/// Acceptance 1 and 2 on a 4K-class frame: a row prints its title and no
/// more of the feature, and no id is cut inside its hash. With no pane
/// beside it, the title takes the width the frame gives it (#503).
#[test]
fn a_row_prints_its_title_and_an_id_whose_hash_is_whole() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(None);
    tui.all = true;
    let frame = frame_of(&tui, &views, 320, 80);
    let title =
        "#403 macOS fix, round 9: PR #483's test (macos-latest) job fails four protocol tests";
    assert!(frame.contains(title), "{frame}");
    // The title column takes what the other cells leave, so the age and
    // the id stand at the list's right edge and it leaves no width empty.
    let row = frame.lines().find(|line| line.contains(title)).unwrap();
    assert!(row.ends_with("7m03s   fix-4…3c1f9a02│"), "{row}");
    for past in ["symlink", "second line", "What was asked"] {
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

/// Acceptance 2 at every width the TUI draws: from [`MIN_WIDTH`] to the
/// list the detail pane stands beside, each row paints its verdict and
/// its id whole, and the count line its count and its key. The columns
/// that give way are the age and the residual, never a cell's tail.
#[test]
fn every_width_from_the_minimum_draws_each_verdict_and_id_whole() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(None);
    for width in MIN_WIDTH..=LIST_COLUMNS {
        let lines = lines_of(drawn(&tui, &views, width, 30).backend().buffer());
        let count = &lines[row_of(&lines, "2 older runs")];
        assert!(count.contains("press a to show"), "at {width}: {count}");
    }
    tui.all = true;
    for width in MIN_WIDTH..=LIST_COLUMNS {
        let lines = lines_of(drawn(&tui, &views, width, 30).backend().buffer());
        for row in &views.runs.runs {
            let line = &lines[row_of(&lines, &short_id(&row.run_id))];
            assert!(line.contains(&row.verdict.text), "at {width}: {line}");
        }
    }
}

/// Acceptance 1 against the sanitizer: a family emoji is one 2-column
/// sequence only while its joiners hold, and the TUI strips joiners, so
/// thirty of them measure 60 columns in the view and paint 240 here.
/// The row paints its title clamped as drawn: on a 160-column frame,
/// whose title column holds 97, 48 emoji and an ellipsis.
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
    let frame = frame_of(&Tui::new(None), &views, 160, 48);
    let painted = frame
        .chars()
        .filter(|c| family.contains(*c) && *c != '\u{200D}');
    assert_eq!(painted.count(), 48, "{frame}");
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

/// Acceptance 5's frame, asked of its cells: the dashboard stands
/// beside the list from [`DASHBOARD_FROM`] columns, and only over a
/// selection; the list keeps the frame below it and with nothing
/// selected. Each frame is drawn at the width the shell measured for it.
#[test]
fn the_detail_pane_needs_the_width_and_a_selection() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(None);
    tui.width = 320;
    assert!(!frame_of(&tui, &views, 320, 80).contains("rule      "));
    tui.cursor[0] = Some(HELD.to_string());
    tui.width = DASHBOARD_FROM - 1;
    assert!(!frame_of(&tui, &views, DASHBOARD_FROM - 1, 30).contains(HELD));
    tui.width = DASHBOARD_FROM;
    let frame = frame_of(&tui, &views, DASHBOARD_FROM, 30);
    assert!(frame.contains(&format!("│{HELD}")), "{frame}");
    // The list beside the dashboard holds a whole title, sixty columns of it.
    let title = "#403 macOS fix, round 9: PR #483's test (macos-latest) job…";
    assert!(frame.contains(&format!("{title}  ")), "{frame}");
    let top = frame.lines().next().unwrap();
    let list_edge = usize::from(LIST_COLUMNS) - 1;
    assert_eq!(
        top.chars().position(|c| c == '┐'),
        Some(list_edge),
        "{frame}"
    );
    tui.width = 320;
    let frame = frame_of(&tui, &views, 320, 80);
    let wanted = [
        HELD,
        "parked · review · 7m03s",
        "verdict   UNVERIFIED-SE… · residual high",
        "rule      REVIEW-UNVERIFIED-SECURITY",
        "reason    REVIEW-UNVERIFIED-SECURITY for (review, clean)",
        "cargo-exemption-hold-5b6c7d8e seq 40 · review · max_residual_severity: high",
        "#362 cargo exemption",
    ];
    for text in wanted {
        assert!(frame.contains(text), "{text}:\n{frame}");
    }
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

/// `Tab` reaches the dashboard only while it is on the frame; there the
/// list keys scroll it a drawn line at a time, the run's own lines first,
/// `Enter` still opens a run whose view holds no notes, and moving the
/// list reads the next run from the top.
#[test]
fn tab_focuses_the_detail_pane_which_scrolls_and_enter_still_opens_the_run() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(None);
    tui.cursor[0] = Some(HELD.to_string());
    tui.width = DASHBOARD_FROM - 1;
    apply(&mut tui, &views, Key::Tab);
    assert_eq!(tui.pane, 0, "no dashboard, nothing to tab to");
    tui.width = DASHBOARD_FROM;
    let footer = footer_for(&tui, &views);
    assert!(
        footer.contains("Enter open run · Tab dashboard · d hide dashboard"),
        "{footer}"
    );
    apply(&mut tui, &views, Key::Tab);
    assert_eq!(tui.pane, 1);
    assert!(footer_for(&tui, &views).starts_with("↑↓/jk scroll · Enter open run · Tab list"));
    // The title, the id, how it stands, and its ending's heading.
    for _ in 0..5 {
        apply(&mut tui, &views, Key::Char('j'));
    }
    assert_eq!((tui.offset, tui.cursor[0].as_deref()), (5, Some(HELD)));
    // The title stays on the list's row; the body has scrolled past the
    // run's own lines to its verdict.
    let frame = frame_of(&tui, &views, DASHBOARD_FROM, 30);
    assert!(frame.contains("│verdict   UNVERIFIED-SE…"), "{frame}");
    assert!(!frame.contains(&format!("││{HELD}  ")), "{frame}");
    assert_eq!(frame.matches("#362 cargo exemption").count(), 2, "{frame}");
    apply(&mut tui, &views, Key::Char('G'));
    // Nineteen lines of the run's, its graph unread (#508), and its
    // commission folded to four.
    assert_eq!(tui.offset, 22, "the last line drawn");
    tui.width = DASHBOARD_FROM - 1;
    assert!(
        footer_for(&tui, &views).starts_with("↑↓/jk move"),
        "narrowed, the list's"
    );
    tui.width = DASHBOARD_FROM;
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
    let mut terminal = Terminal::new(TestBackend::new(DASHBOARD_FROM, 30)).unwrap();
    // The fleet opens on its first row (#503); one `Down` is the second.
    script(&[Key::Down, Key::Tab, Key::Quit]);
    let mut source = |_: Ask| Ok(Some(fleet_to_act_on()));
    let mut tui = Tui::new(None);
    drive(&mut terminal, &test_ops(), &mut source, &mut tui, 9).unwrap();
    assert_eq!((tui.width, tui.pane), (DASHBOARD_FROM, 1));
    assert_eq!(tui.cursor[0].as_deref(), Some(HELD));
}

/// One answer to whether the detail pane is on the frame: the shell
/// measures the frame before it draws it, so the very first frame at a
/// wide width draws the pane and its footer names the key to reach it.
#[test]
fn the_first_wide_frame_and_its_footer_agree_on_the_detail_pane() {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let mut terminal = Terminal::new(TestBackend::new(DASHBOARD_FROM, 30)).unwrap();
    script(&[]);
    let mut source = |_: Ask| Ok(Some(fleet_to_act_on()));
    let mut tui = Tui::new(None);
    tui.cursor[0] = Some(HELD.to_string());
    drive(&mut terminal, &test_ops(), &mut source, &mut tui, 1).unwrap();
    let frame = lines_of(terminal.backend().buffer()).join("\n");
    assert!(frame.contains(&format!("│{HELD}")), "{frame}");
    assert!(frame.contains("· Tab dashboard ·"), "{frame}");
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
    tui.width = DASHBOARD_FROM;
    tui.pane = 1;
    // `c` opens the commission the dashboard folds to its first lines.
    apply(&mut tui, &views, Key::Char('c'));
    let frame = frame_of(&tui, &views, DASHBOARD_FROM, 30);
    assert!(!frame.contains("the last words"), "{frame}");
    apply(&mut tui, &views, Key::Char('G'));
    // Sixteen lines of the run's, its graph unread (#508), the 44 its
    // commission draws, and the key that folds it.
    assert_eq!(
        tui.offset, 60,
        "the run's lines, then the commission's drawn"
    );
    apply(&mut tui, &views, Key::Char('k'));
    let frame = frame_of(&tui, &views, DASHBOARD_FROM, 30);
    assert!(frame.contains("the last words"), "{frame}");
    assert!(frame.contains("│c folds it"), "{frame}");
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

// --------------------------------------------------------------- #503

/// The frame the shell draws on the operator's terminal with no key
/// pressed, and the state it leaves.
pub(super) fn opened_on(views: fn() -> Views, width: u16, height: u16) -> (Tui, String) {
    let _serialized = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    script(&[]);
    let mut tui = Tui::new(None);
    let mut source = |_: Ask| Ok(Some(views()));
    drive(&mut terminal, &test_ops(), &mut source, &mut tui, 1).unwrap();
    (tui, terminal.backend().to_string())
}

/// Item 1: the fleet opens on its first row — the first run that needs
/// the operator — so a wide frame draws the detail pane before any key
/// is pressed; with nobody to act on, it opens on the first running run.
#[test]
fn the_fleet_opens_on_the_first_run_that_needs_you_with_its_detail_drawn() {
    let (tui, frame) = opened_on(fleet_with_a_dead_run, 330, 60);
    assert_eq!(tui.cursor[0].as_deref(), Some(DEAD));
    assert!(frame.contains(&format!("││{DEAD}")), "{frame}");
    assert!(frame.contains("· Tab dashboard ·"), "{frame}");
    let mut views = fleet_to_act_on();
    views
        .runs
        .runs
        .retain(|row| row.status.as_deref() == Some("running"));
    let mut tui = Tui::new(None);
    settle(&mut tui, &views);
    let first = Some("fix-403-on-macos-round-9-3c1f9a02");
    assert_eq!(tui.cursor[0].as_deref(), first);
    // A cursor the operator moved is theirs: a refresh never re-seeds it.
    tui.cursor[0] = Some(HELD.to_string());
    settle(&mut tui, &views);
    assert_eq!(tui.cursor[0].as_deref(), Some(HELD));
}

/// Item 3: a run that folds to running and whose journal has not moved
/// for 116 hours is listed as needing the operator, stale, never as
/// running: its row says since when and what to do, its detail names
/// the commands, and the brand mark does not pulse for it.
#[test]
fn a_dead_running_run_needs_you_as_stale_and_never_reads_as_live() {
    let views = fleet_with_a_dead_run();
    let tui = Tui::new(None);
    assert_eq!(listed(&tui, &views)[0], DEAD);
    let terminal = drawn(&tui, &views, 160, 48);
    let lines = lines_of(terminal.backend().buffer());
    let row = row_of(&lines, "landi…9d8c7b6a");
    assert!(lines[row].contains("! stale"), "{}", lines[row]);
    let (column, at) = cell_at(&lines, row, "! stale");
    let worn = terminal.backend().buffer()[(column, at)].modifier;
    assert_eq!(worn, Modifier::DIM, "quiet, never the running bold");
    let prompt = "stale: no event since 116h07m; resume or conclude";
    assert!(lines[row + 1].contains(prompt), "{}", lines[row + 1]);
    assert!(row < row_of(&lines, "│running"), "{}", lines.join("\n"));
    let detail = dashboard_of(&views, &views.runs.runs[0]);
    assert_eq!(detail[2], "stale · land · 168h07m");
    let wanted = [
        "stale     no event since 116h07m, past the 3h bound: an attempt's 2h deadline and a 1h \
         margin"
            .to_string(),
        format!("way out   brokkr resume --run {DEAD} --bundle <its-bundle>"),
        "          drives it again under its pinned bundle (or --recipe <name>);".to_string(),
        format!("          or brokkr conclude --run {DEAD} --reason <why> closes it"),
    ];
    assert_eq!(under(&detail, "WAY OUT"), wanted, "{detail:#?}");
    let mut dead_only = fleet_with_a_dead_run();
    dead_only.runs.runs.truncate(1);
    assert!(!fleet_live(&dead_only), "a stale run is not forging");
    assert!(fleet_live(&views), "the other two are");
}

/// Review F3: every command a way out names is one `brokkr` parses once
/// its placeholders are filled, and one the engine admits on a run that
/// stands as that one does. A stale run folds to running, which
/// `operator retry` refuses and `resume` and `conclude` admit. Review M1:
/// a run whose journal the store does not load names no `export`, since
/// the store refuses it that too; either quarantine names `operator
/// acknowledge` (#511).
#[test]
fn every_way_out_names_a_command_brokkr_parses_and_the_run_admits() {
    use clap::Parser;
    let mut views = fleet_with_a_dead_run();
    let unloaded = brokkr_view::RunEntry {
        run_id: "journal-that-will-not-load-1a2b3c4d",
        feature: "A journal whose chain does not verify",
        created_at: T0,
        last_recorded_at: None,
        state: None,
        detail: Some(Quarantine::DoesNotLoad.in_words("hash chain broken at seq 3")),
        residuals: &[],
    };
    views
        .runs
        .runs
        .extend(brokkr_view::run_rows(&[unloaded]).runs);
    let mut named = Vec::new();
    for row in &views.runs.runs {
        let Some(need) = brokkr_view::need(row, &views.now) else {
            continue;
        };
        for command in need.commands(&row.run_id) {
            let argv = command
                .split(' ')
                .map(|word| if word.starts_with('<') { "x" } else { word });
            let cli = crate::Cli::try_parse_from(argv)
                .unwrap_or_else(|error| panic!("{command}: {error}"));
            let read = read_as(row);
            assert!(admits(read, &cli.command), "{command} on {read:?}");
            named.push(need.label());
        }
    }
    assert_eq!(
        named,
        [
            "stale",
            "stale",
            "quarantined",
            "quarantined",
            "quarantined"
        ]
    );
}

/// How the store and the fold read a run standing so: the status it
/// folds to, or what refused its journal.
fn read_as(row: &RunRow) -> Result<Status, Quarantine> {
    match row.verdict.standing {
        Standing::Quarantined => Err(row.quarantine.expect("a quarantined row names it")),
        Standing::Parked => Ok(Status::AwaitingOperator),
        Standing::Running | Standing::Stopping => Ok(Status::Running),
        Standing::Shipped => Ok(Status::Completed),
        Standing::Stopped | Standing::OperatorStopped => Ok(Status::Stopped),
    }
}

/// Whether the engine admits `command` on a run its journal reads as
/// `read`, by its own rules: `operator` by the fold's acceptance,
/// `resume` drives a run that runs or waits, `conclude` folds the journal
/// before it appends, so it refuses one that does not fold
/// (`a_broken_chain_refuses_the_whole_conclusion`) and one already
/// ended, and `export` loads the journal before it writes it, so it
/// refuses one the store does not load and writes any other as written.
fn admits(read: Result<Status, Quarantine>, command: &crate::Cmd) -> bool {
    use brokkr_core::fold::{acceptance_refusal, OperatorCommand};
    let status = read.ok();
    match command {
        // It reads no fold, and refuses a run whose journal folds (#511).
        crate::Cmd::Operator(args) if args.command == "acknowledge" => status.is_none(),
        crate::Cmd::Operator(args) => match (status, OperatorCommand::parse(&args.command)) {
            (Some(status), Some(word)) => acceptance_refusal(&state_of(status), word).is_none(),
            _ => false,
        },
        crate::Cmd::Resume(_) => matches!(status, Some(Status::Running | Status::AwaitingOperator)),
        crate::Cmd::Conclude(_) => {
            status.is_some_and(|status| !matches!(status, Status::Completed | Status::Stopped))
        }
        crate::Cmd::Export(_) => read != Err(Quarantine::DoesNotLoad),
        _ => panic!("a way out names a verb this test does not know"),
    }
}

/// Review F1: a run opened from the fleet and left again is the fleet's
/// selection, so a wide frame draws its detail beside the list again.
#[test]
fn leaving_a_run_lands_back_on_it_with_its_detail_drawn() {
    let views = fleet_with_a_dead_run();
    let (mut tui, _) = opened_on(fleet_with_a_dead_run, 330, 60);
    apply(&mut tui, &views, Key::Char('j'));
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(tui.run.as_deref(), Some("journal-that-broke-7f8e9d0c"));
    tui.cursor[0] = Some("design".to_string());
    apply(&mut tui, &views, Key::Escape);
    assert_eq!(tui.level, Level::Runs);
    let shown = detail_row(&tui, &views).map(|row| row.run_id.as_str());
    assert_eq!(shown, Some("journal-that-broke-7f8e9d0c"));
    let frame = frame_of(&tui, &views, 330, 60);
    assert!(frame.contains("││journal-that-broke-7f8e9d0c"), "{frame}");
}

/// Review F2: `/` finds a run by a word its title paints past the
/// sixtieth column, as wide a list as the frame gives it.
#[test]
fn the_filter_finds_a_word_painted_past_the_sixtieth_column() {
    let views = fleet_with_a_dead_run();
    let mut tui = Tui::new(None);
    let frame = frame_of(&tui, &views, 220, 50);
    assert!(frame.contains("job fails four protocol tests"), "{frame}");
    apply(&mut tui, &views, Key::Char('/'));
    for character in "protocol".chars() {
        apply(&mut tui, &views, Key::Char(character));
    }
    apply(&mut tui, &views, Key::Enter);
    assert_eq!(listed(&tui, &views), ["fix-403-on-macos-round-9-3c1f9a02"]);
}

/// Item 4: a quarantined run's row says what to do, and its detail
/// names the fold's refusal and the operator's way out.
#[test]
fn a_quarantined_run_says_export_and_inspect_and_how() {
    let views = fleet_to_act_on();
    let lines = lines_of(drawn(&Tui::new(None), &views, 160, 48).backend().buffer());
    let row = row_of(&lines, "journ…7f8e9d0c");
    let prompt = "quarantined: export and inspect";
    assert!(lines[row + 1].contains(prompt), "{}", lines[row + 1]);
    let broken = views.runs.runs.iter().find(|row| row.detail.is_some());
    let detail = dashboard_of(&views, broken.unwrap());
    let refusal = "journal   event 12: event after terminal status".to_string();
    assert!(
        under(&detail, "WHAT IT NEEDS").contains(&refusal),
        "{detail:#?}"
    );
    let run = "journal-that-broke-7f8e9d0c";
    let wanted = [
        format!("way out   brokkr export --run {run}"),
        "          and inspect the journal it writes".to_string(),
        format!("          then brokkr operator acknowledge --run {run} --reason <why> lists it under older"),
    ];
    assert_eq!(under(&detail, "WAY OUT"), wanted, "{detail:#?}");
}

/// #511: an acknowledged quarantine leaves "needs you" for the older runs
/// the list folds, keeping its `?` and its refusal, and says who answered
/// it on its row and in its detail.
#[test]
fn an_acknowledged_quarantine_folds_into_older_keeping_its_mark_and_refusal() {
    let mut views = fleet_to_act_on();
    let run = "journal-that-broke-7f8e9d0c";
    let broken = views.runs.runs.iter_mut().find(|row| row.run_id == run);
    broken.unwrap().acknowledgement = Some(brokkr_view::Acknowledgement {
        seq: 12,
        event_hash: "ab12".to_string(),
        operator: "valentin".to_string(),
        reason: "history from an engine since changed".to_string(),
        recorded_at: T0.to_string(),
    });
    let mut tui = Tui::new(None);
    assert_eq!(listed(&tui, &views)[0], HELD, "it needs no one now");
    assert!(!listed(&tui, &views).contains(&run.to_string()), "folded");
    let frame = frame_of(&tui, &views, 160, 48);
    assert!(frame.contains("3 older runs"), "{frame}");

    apply(&mut tui, &views, Key::Char('a'));
    let lines = lines_of(drawn(&tui, &views, 160, 48).backend().buffer());
    let row = row_of(&lines, "journ…7f8e9d0c");
    assert!(row > row_of(&lines, "│older"), "{}", lines.join("\n"));
    assert!(lines[row].contains('?'), "{}", lines[row]);
    let answered = "acknowledged by valentin at 2026-01-01T00:00:00Z, event 12: history";
    assert!(lines[row + 1].contains(answered), "{}", lines[row + 1]);
    let acknowledged = views.runs.runs.iter().find(|row| row.run_id == run);
    let detail = dashboard_of(&views, acknowledged.unwrap());
    let ended = under(&detail, "WHY IT ENDED");
    for fact in [
        "journal   event 12: event after terminal status".to_string(),
        format!("operator  {answered} from an engine since changed"),
    ] {
        assert!(ended.contains(&fact), "{fact}: {detail:#?}");
    }
    assert_eq!(under(&detail, "WAY OUT"), ["—"], "{detail:#?}");
}

/// Item 6: `→`, `l` and Space open a running row in place — its phase,
/// its seat and attempt, how long it has run and its last event — and
/// `←` and `h` fold it; `Enter` still opens the run. A row that is not
/// running has nothing to open, and away from the fleet `l`, `h` and
/// Space bind nothing.
#[test]
fn a_running_row_opens_in_place_and_folds_again() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(None);
    let fix = "fix-403-on-macos-round-9-3c1f9a02";
    tui.cursor[0] = Some(fix.to_string());
    let opened = [
        "phase review · seat reviewer · attempt 2",
        "elapsed 7m03s · last event 2m03s ago, seq 18",
    ];
    let shows = |tui: &Tui| {
        let frame = frame_of(tui, &views, 160, 48);
        opened.map(|text| frame.contains(text))
    };
    assert_eq!(shows(&tui), [false, false]);
    assert!(footer_for(&tui, &views).contains("Enter open run · → expand ·"));
    for (open, fold) in [
        (Key::Right, Key::Left),
        (Key::Char('l'), Key::Char('h')),
        (Key::Char(' '), Key::Left),
    ] {
        apply(&mut tui, &views, open);
        assert_eq!(shows(&tui), [true, true], "{open:?}");
        assert!(footer_for(&tui, &views).contains("Enter open run · ← fold ·"));
        apply(&mut tui, &views, fold);
        assert_eq!(shows(&tui), [false, false], "{fold:?}");
    }
    tui.cursor[0] = Some(HELD.to_string());
    apply(&mut tui, &views, Key::Right);
    assert!(tui.expanded.is_empty(), "a parked run has nothing to open");
    assert!(!footer_for(&tui, &views).contains("expand"));
    tui.cursor[0] = Some(fix.to_string());
    apply(&mut tui, &views, Key::Right);
    apply(&mut tui, &views, Key::Enter);
    assert_eq!((tui.level, tui.run.as_deref()), (Level::Run, Some(fix)));
    let held = tui.expanded.clone();
    for key in [Key::Char('l'), Key::Char('h'), Key::Char(' ')] {
        apply(&mut tui, &views, key);
    }
    assert_eq!(tui.expanded, held, "the run level binds none of them");
}

/// A running run between effects, whose journal states no last event,
/// opens to the absence marks; with nothing selected `→` opens nothing;
/// and the runs a hearth opened are its own, parked with it (decision
/// 0026 ruling 2).
#[test]
fn an_opened_row_marks_what_it_lacks_and_belongs_to_its_hearth() {
    let views = fleet_to_act_on();
    let mut tui = Tui::over(None, vec!["alpha".to_string(), "beta".to_string()], 0);
    apply(&mut tui, &views, Key::Right);
    assert!(tui.expanded.is_empty(), "nothing is selected");
    tui.cursor[0] = Some("0065-rebuild-unit-20-1c2d3e4f".to_string());
    apply(&mut tui, &views, Key::Right);
    let frame = frame_of(&tui, &views, 160, 48);
    for text in [
        "phase triage · seat — · attempt —",
        "elapsed 7m03s · last event —",
    ] {
        assert!(frame.contains(text), "{text}:\n{frame}");
    }
    switch(&mut tui, 1);
    assert!(tui.expanded.is_empty(), "beta opened nothing");
    switch(&mut tui, 0);
    assert_eq!(tui.expanded.len(), 1, "alpha's opened run is returned");
}

/// Item 5: the fleet's footer names `Enter`, `a` and `/` at every width
/// the TUI draws, `Tab` wherever a column stands beside the list, and
/// drops its lesser keys to fit; a frame too narrow for the dashboard
/// says how wide it needs to be (#503).
#[test]
fn the_footer_names_enter_tab_a_and_filter_at_every_width() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(None);
    tui.cursor[0] = Some(HELD.to_string());
    for width in MIN_WIDTH..=400 {
        tui.width = width;
        let lines = lines_of(drawn(&tui, &views, width, 30).backend().buffer());
        let footer = lines.last().unwrap().trim_end();
        let (tab, d) = match width >= DASHBOARD_FROM {
            true => ("Tab dashboard ·", "d hide dashboard ·"),
            false => ("Enter open run ·", "d dashboard ≥195 ·"),
        };
        for key in ["Enter open run ·", tab, d, "a all runs ·", "/ filter"] {
            assert!(footer.contains(key), "at {width}: {footer}");
        }
        let whole = footer_within(&tui, &views, usize::from(width));
        assert_eq!(footer, whole, "at {width}, nothing is cut");
    }
    tui.width = 0;
    assert_eq!(
        footer_within(&tui, &views, 60),
        "Enter open run · d dashboard ≥195 · a all runs · / filter"
    );
    assert_eq!(
        footer_within(&tui, &views, 80),
        "Enter open run · d dashboard ≥195 · f findings ≥257 · a all runs · / filter"
    );
}

/// Item 5: the older runs' line reads as an action, and the heading of
/// the older runs `a` showed says how to fold them again.
#[test]
fn the_older_runs_say_how_to_show_them_and_how_to_fold_them() {
    let views = fleet_to_act_on();
    let mut tui = Tui::new(None);
    let frame = frame_of(&tui, &views, 100, 30);
    assert!(frame.contains("│2 older runs: press a to show"), "{frame}");
    apply(&mut tui, &views, Key::Char('a'));
    let frame = frame_of(&tui, &views, 100, 30);
    assert!(frame.contains("│older         a folds them"), "{frame}");
    assert!(!frame.contains("needs you     a"), "{frame}");
}

/// `count` runs of the fixture's parked one, `more-00` onward, in a frame
/// read for `window` whose first page ends at `ahead`.
fn more_runs(count: usize, window: Window, ahead: Option<&brokkr_view::Cursor>) -> Views {
    let mut views = fleet_to_act_on();
    let held = views.runs.runs.iter().find(|row| row.run_id == HELD);
    let held = held.unwrap().clone();
    views.runs.runs = (0..count)
        .map(|at| RunRow {
            run_id: format!("more-{at:02}"),
            ..held.clone()
        })
        .collect();
    (views.window, views.ahead) = (window, ahead.cloned());
    views
}

/// Decision 0089 ruling 6: the list reads the page its window starts at
/// and the one ahead, by keyset. Its window moves onto the page ahead
/// when the cursor reaches it, once, however many keys land before that
/// window's frame; back a page when the cursor steps up off its first
/// row, the cursor kept; and to its top on `g`. `a` reads as it does
/// without, and a filter reads its own window from its top. The older
/// runs its pages left unread are counted with the ones it folds; and
/// the runs behind their summary past those a refresh read are one line
/// in "needs you", which heads it when no run there is listed.
#[test]
fn the_list_scrolls_by_keyset_and_counts_what_its_pages_left_unread() {
    let ahead: brokkr_view::Cursor = "needs-you:5:more-49".parse().unwrap();
    let top = more_runs(60, Window::default(), Some(&ahead));
    let mut tui = Tui::new(None);
    assert_eq!(asked_window(&tui), Window::default());
    tui.cursor[0] = Some("more-48".to_string());
    apply(&mut tui, &top, Key::Down);
    assert_eq!(asked_window(&tui), Window::default(), "the cursor's page");
    apply(&mut tui, &top, Key::Down);
    apply(&mut tui, &top, Key::Down);
    let scrolled = Window {
        after: Some(ahead.clone()),
        filter: String::new(),
    };
    assert_eq!(asked_window(&tui), scrolled);
    assert_eq!(
        tui.scrolled.past,
        std::slice::from_ref(&ahead),
        "moved once"
    );
    let below = more_runs(10, scrolled.clone(), None);
    tui.cursor[0] = Some("more-00".to_string());
    tui.all = true;
    assert_eq!(asked_window(&tui), scrolled, "`a` reads its window alike");
    apply(&mut tui, &below, Key::Up);
    assert_eq!(asked_window(&tui), Window::default());
    assert_eq!(tui.cursor[0].as_deref(), Some("more-00"));
    tui.scrolled.past.push(ahead.clone());
    apply(&mut tui, &below, Key::Char('g'));
    assert_eq!(
        (asked_window(&tui), tui.cursor[0].clone()),
        (Window::default(), None)
    );
    tui.scrolled.past.push(ahead);
    tui.filter = "more".to_string();
    let filtered = Window {
        after: None,
        filter: "more".to_string(),
    };
    assert_eq!(asked_window(&tui), filtered);
    let mut views = fleet_to_act_on();
    views.unread = Unread {
        older: 5,
        behind: 151,
    };
    let frame = frame_of(&Tui::new(None), &views, 160, 30);
    assert!(frame.contains("│7 older runs: press a to show"), "{frame}");
    // The count line, just under the heading of "needs you".
    let headed = |views: &Views| {
        let behind = "151 more runs behind their summary, unread until a writer catches them up";
        let frame = frame_of(&Tui::new(None), views, 160, 30);
        let lines: Vec<&str> = frame.lines().collect();
        let at = lines.iter().position(|line| line.contains(behind));
        at.map(|at| lines[at - 1].contains("│needs you"))
    };
    assert_eq!(headed(&views), Some(true));
    let now = views.now.clone();
    views
        .runs
        .runs
        .retain(|row| brokkr_view::need(row, &now).is_none());
    assert_eq!(headed(&views), Some(true));
}

/// Decision 0089 ruling 6: a step up inside a scrolled window moves the
/// cursor alone, and a wrap down past its last row reads the fleet from
/// its top; a list scrolled under one filter that reaches the page ahead
/// under another keeps that one's key alone; and the list seeds its
/// cursor on the frame of the window it asked for, never of another.
#[test]
fn the_list_scrolls_within_its_window_and_under_its_own_filter() {
    let ahead: brokkr_view::Cursor = "needs-you:5:more-49".parse().unwrap();
    let scrolled = Window {
        after: Some(ahead.clone()),
        filter: String::new(),
    };
    let below = more_runs(10, scrolled.clone(), None);
    let mut tui = Tui::new(None);
    tui.scrolled.passed(ahead.clone());
    tui.cursor[0] = Some("more-05".to_string());
    apply(&mut tui, &below, Key::Up);
    let at = |tui: &Tui| (asked_window(tui), tui.cursor[0].clone());
    assert_eq!(at(&tui), (scrolled, Some("more-04".to_string())));
    tui.cursor[0] = Some("more-09".to_string());
    apply(&mut tui, &below, Key::Down);
    assert_eq!(at(&tui), (Window::default(), None), "wrapped to the top");
    tui.scrolled.passed(ahead);
    tui.filter = "more".to_string();
    let next: brokkr_view::Cursor = "needs-you:6:more-49".parse().unwrap();
    let filtered = Window {
        after: None,
        filter: "more".to_string(),
    };
    let top = more_runs(60, filtered, Some(&next));
    tui.cursor[0] = Some("more-49".to_string());
    apply(&mut tui, &top, Key::Down);
    let kept = super::state::Scrolled {
        filter: "more".to_string(),
        past: vec![next],
    };
    assert_eq!(tui.scrolled, kept);
    let mut fresh = Tui::new(None);
    seed_cursor(&mut fresh, &below);
    assert_eq!(fresh.cursor[0], None, "a frame of another window");
    seed_cursor(&mut fresh, &more_runs(10, Window::default(), None));
    assert_eq!(fresh.cursor[0].as_deref(), Some("more-00"));
}

/// Decision 0089 rulings 6 and 9: the list keeps the keys of the last
/// [`SCROLL_BACK`](super::state::SCROLL_BACK) pages it scrolled past alone, so what it holds does not
/// grow with how far it scrolls; it steps back a page at a time over
/// them, and past them to its top.
#[test]
fn the_list_keeps_the_keys_of_its_last_pages_alone() {
    let key =
        |at: usize| -> brokkr_view::Cursor { format!("needs-you:{at}:more-49").parse().unwrap() };
    let pages = super::state::SCROLL_BACK + 3;
    let mut tui = Tui::new(None);
    for at in 0..pages {
        let views = more_runs(60, asked_window(&tui), Some(&key(at)));
        tui.cursor[0] = Some("more-49".to_string());
        apply(&mut tui, &views, Key::Down);
    }
    let kept: Vec<brokkr_view::Cursor> = (3..pages).map(key).collect();
    assert_eq!(tui.scrolled.past, kept);
    for at in (3..pages).rev() {
        assert_eq!(asked_window(&tui).after, Some(key(at)));
        let views = more_runs(10, asked_window(&tui), None);
        tui.cursor[0] = Some("more-00".to_string());
        apply(&mut tui, &views, Key::Up);
    }
    assert_eq!(asked_window(&tui), Window::default(), "past them, its top");
}

/// Every `timeout_seconds` a JSON value states, at any depth.
fn deadlines(value: &Value, found: &mut Vec<u64>) {
    match value {
        Value::Object(map) => {
            found.extend(map.get("timeout_seconds").and_then(Value::as_u64));
            map.values().for_each(|inner| deadlines(inner, found));
        }
        Value::Array(items) => items.iter().for_each(|inner| deadlines(inner, found)),
        _ => {}
    }
}

/// Every deadline the JSON files under `path` state. A file that does
/// not parse fails the test rather than going unread.
fn deadlines_under(path: &std::path::Path, found: &mut Vec<u64>) {
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            deadlines_under(&path, found);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let text = std::fs::read_to_string(&path).unwrap();
            let value = serde_json::from_str(&text)
                .unwrap_or_else(|error| panic!("{} does not parse: {error}", path.display()));
            deadlines(&value, found);
        }
    }
}

/// Item 3's bound, held to the repository: a running run is stale once
/// its journal has been silent for longer than the longest deadline any
/// recipe, agent or bundle here gives an attempt, and an hour past it —
/// and not a second sooner. The journal records no attempt's deadline,
/// so this is the one the view states.
#[test]
fn the_staleness_bound_is_the_longest_shipped_deadline_and_an_hour() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    for library in ["recipes", "agents"] {
        deadlines_under(&root.join(library), &mut found);
    }
    assert!(found.len() > 50, "the walk read the libraries: {found:?}");
    let bound = found.iter().max().unwrap() + 3600;
    assert!(bound < 12 * 3600, "the clock below is a half day: {bound}");
    let at = |silent: u64| {
        let left = 12 * 3600 - silent;
        let (hours, minutes, seconds) = (left / 3600, left / 60 % 60, left % 60);
        format!("2026-01-01T{hours:02}:{minutes:02}:{seconds:02}Z")
    };
    let mut running = state_of(Status::Running);
    running.phase = Some("land".to_string());
    let quiet = |last: &str| {
        let mut entry = brokkr_view::RunEntry {
            run_id: "quiet",
            feature: "a run",
            created_at: T0,
            last_recorded_at: None,
            state: Some(&running),
            detail: None,
            residuals: &[],
        };
        entry.last_recorded_at = Some(last);
        let row = brokkr_view::run_rows(&[entry]).runs.remove(0);
        brokkr_view::need(&row, "2026-01-01T12:00:00Z")
    };
    assert_eq!(quiet(&at(bound)), None);
    let silent = "3h00m".to_string();
    assert_eq!(quiet(&at(bound + 1)), Some(Need::Stale { silent }));
}

/// The frames the shell reads of the journal at `db` for the console as
/// `tui` stands: forced, the fleet due, its open run and its window.
fn forced_frames(db: &std::path::Path) -> impl FnMut(&Tui) -> Views + '_ {
    let mut read = super::tests::reading(db);
    move |tui: &Tui| {
        let ask = Ask {
            run: tui.run.as_deref(),
            subject: None,
            force: true,
            fleet: true,
            tab: 0,
            window: asked_window(tui),
        };
        read(ask).unwrap().expect("a forced read draws a frame")
    }
}

/// Decision 0089 rulings 5 and 6: a run opened from the list scrolled a
/// page down stays open when a writer parks it, though its new place
/// heads "needs you", before the window the list reads, so the next
/// frame lists no row of it: the frame opened it, and it has not gone.
#[test]
fn a_run_opened_past_the_first_page_stays_open_when_a_writer_moves_it_before_the_window() {
    use brokkr_store::test_support::{generate, Generated, Standing, Width};
    use brokkr_store::Store;
    use time::format_description::well_known::Rfc3339;
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    crate::tests::running_store(&db, "opened");
    let now = time::OffsetDateTime::parse(NOW, &Rfc3339).unwrap();
    let held: Vec<Generated> = (0..110)
        .map(|n| Generated {
            run_id: format!("held-{n:03}"),
            standing: Standing::Parked,
            events: 3,
            last_at: (now - time::Duration::minutes(n)).format(&Rfc3339).unwrap(),
            width: Width::Narrow,
        })
        .collect();
    generate(&Store::open(&db).unwrap(), &held).unwrap();
    let mut frame = forced_frames(&db);
    // The list scrolled a page down, its window the second page and the
    // third: the parked runs, then the running one, 111th.
    let mut tui = Tui::new(None);
    let ahead = frame(&tui).ahead.expect("the fleet runs past a page");
    tui.scrolled.passed(ahead);
    tui.assign_run("opened".to_string());
    let views = frame(&tui);
    let listed: Vec<&str> = views
        .runs
        .runs
        .iter()
        .map(|row| row.run_id.as_str())
        .collect();
    assert_eq!(listed.iter().position(|id| *id == "opened"), Some(60));
    settle(&mut tui, &views);
    assert_eq!(
        (tui.run.as_deref(), tui.level),
        (Some("opened"), Level::Run)
    );
    // A current writer parks it, its seat's attempts spent, at the wall
    // clock, after every park listed: it heads "needs you", before the
    // window's first row.
    let effect = json!({"effect_id": "eff", "seat": "work", "phase": "work"});
    let reason = json!({"reason": "attempts exhausted"});
    for (event_type, payload) in [
        (brokkr_core::EventType::EffectRequested, effect),
        (brokkr_core::EventType::RunParked, reason),
    ] {
        let mut store = Store::open(&db).unwrap();
        let events = store.load("opened").unwrap();
        store.carry(brokkr_core::summary::Account::whole("opened", &events));
        store
            .append_next("opened", event_type, payload, None, None)
            .unwrap();
    }
    let views = frame(&tui);
    assert!(views.runs.runs.iter().all(|row| row.run_id != "opened"));
    let parked = views.run.as_ref().and_then(|run| run.summary.as_ref());
    assert_eq!(
        parked.map(|summary| summary.status.as_str()),
        Some("awaiting_operator")
    );
    settle(&mut tui, &views);
    assert_eq!(
        (tui.run.as_deref(), tui.level),
        (Some("opened"), Level::Run)
    );
    // At the fleet's level, which reads no open run, a window that no
    // longer lists the run the operator left takes nothing with it: the
    // cursor they moved stays theirs.
    let mut fleet = fleet_to_act_on();
    fleet.run = None;
    let mut tui = Tui::new(None);
    tui.run = Some("opened".to_string());
    tui.cursor[0] = Some(HELD.to_string());
    settle(&mut tui, &fleet);
    let held = (tui.run.as_deref(), tui.cursor[0].as_deref());
    assert_eq!(held, (Some("opened"), Some(HELD)));
}

/// Decision 0089 rulings 5 and 6: two run ids no writer admits, cut to
/// one shown id, are two rows the list selects apart by their whole ids,
/// and opening either reads no run: the frame is drawn, the run stays
/// open, and the fleet lists it refused in the store's words.
#[test]
fn two_refused_ids_cut_alike_are_selected_and_opened_apart() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    drop(brokkr_store::Store::open(&db).unwrap());
    let ids = ["v".repeat(200), format!("{}w", "v".repeat(199))];
    let conn = rusqlite::Connection::open(&db).unwrap();
    for run_id in &ids {
        brokkr_store::test_support::plant_run(&conn, run_id, "{}").unwrap();
    }
    let mut frame = forced_frames(&db);
    let (cut, words) = (
        format!("{}…", "v".repeat(128)),
        "its run id is past 128 bytes, which no writer admits",
    );
    let mut tui = Tui::new(None);
    let fleet = frame(&tui);
    settle(&mut tui, &fleet);
    let mut opened = Vec::new();
    for _ in &ids {
        let row = selected_run(&tui, &fleet).expect("a row is selected");
        let shown = (row.run_id.as_str(), row.detail.as_deref());
        assert_eq!(shown, (cut.as_str(), Some(words)));
        apply(&mut tui, &fleet, Key::Enter);
        let views = frame(&tui);
        settle(&mut tui, &views);
        let run = tui.run.clone().expect("the run stays open");
        let listed = views.runs.runs.iter().find(|row| key_of(row) == run);
        let listed = listed.map(|row| (row.run_id.as_str(), row.detail.as_deref()));
        assert_eq!(
            (views.run.is_none(), listed),
            (true, Some((cut.as_str(), Some(words))))
        );
        opened.push(run);
        apply(&mut tui, &views, Key::Escape);
        apply(&mut tui, &fleet, Key::Down);
    }
    opened.sort();
    assert_eq!(opened, ids);
}
