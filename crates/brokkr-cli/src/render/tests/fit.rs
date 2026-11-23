//! #1167's tests of `brokkr stats graph`'s width detection, its cell fitting and
//! its controlling-terminal proof, split from render/tests.rs mechanically
//! so that file stays under the 2000-line test ceiling once #1174's tests
//! sit beside them on main. No test changed.

use super::*;
use std::ffi::OsString;

use crate::tui::width_of;

/// The width `width` sizes to, under `columns`, where stdout is a
/// terminal as `tty` says and measures `stdout` columns.
fn detected_width(width: Width, tty: bool, columns: Option<&str>, stdout: Option<u16>) -> usize {
    let env = |name: &str| match name {
        "COLUMNS" => columns.map(OsString::from),
        _ => None,
    };
    Style::detect_in(tty, env, width, || stdout).width
}

/// `brokkr stats graph`'s width (#1167): a whole, nonzero `COLUMNS`, else the
/// columns of the stdout it writes to, else 80. An 80-cell controlling
/// terminal never reaches the seam: a 60-cell stdout sizes to 60.
#[test]
fn the_stdout_width_is_columns_else_stdouts_own_else_the_default() {
    let stdout = |columns, measured| detected_width(Width::Stdout, true, columns, measured);
    assert_eq!(stdout(Some("120"), Some(60)), 120, "COLUMNS wins");
    assert_eq!(stdout(None, Some(60)), 60, "stdout's own, not the tty's");
    for ignored in ["abc", "", "0", "-3", "1.5"] {
        assert_eq!(stdout(Some(ignored), Some(60)), 60, "{ignored:?}");
        assert_eq!(stdout(Some(ignored), None), 80, "{ignored:?}");
    }
    assert_eq!(stdout(None, Some(5)), 20, "clamped up");
    assert_eq!(stdout(Some("100000"), Some(60)), 1000, "clamped down");
    assert_eq!(stdout(None, Some(0)), 80, "a window of no columns");
    assert_eq!(stdout(None, None), 80, "it will not say");
    let off = detected_width(Width::Stdout, false, None, Some(60));
    assert_eq!(off, 80, "stdout is no terminal");
}

/// Every other readout keeps main's width: `COLUMNS` where it is set,
/// 0 clamped up as before, else 80, whatever stdout measures.
#[test]
fn the_columns_width_never_reads_the_terminal() {
    let columns = |set| detected_width(Width::Columns, true, set, Some(60));
    assert_eq!(columns(None), 80);
    assert_eq!(columns(Some("abc")), 80);
    assert_eq!(columns(Some("0")), 20);
    assert_eq!(columns(Some("120")), 120);
}

/// A redirected stdout is measured by its own descriptor, which says
/// nothing, whatever terminal controls the process: never /dev/tty.
#[test]
fn a_descriptor_that_is_no_terminal_measures_no_columns() {
    let file = tempfile::tempfile().unwrap();
    assert_eq!(window_columns(&file), None);
    let detected = Style::detect_for(Width::Stdout).width;
    assert!((20..=1000).contains(&detected), "{detected}");
}

/// `text` cut to `max` cells by [`width_of`]: what it keeps and the cells
/// that takes.
fn fitted(text: &str, max: usize) -> (String, usize) {
    let table::Fitted { text, cells } = table::fit_by(text, max, &mut width_of);
    (text.into_owned(), cells)
}

/// A counting [`width_of`]: what it measured, in scalars, is added to
/// `measured`.
fn counting(measured: &std::cell::Cell<usize>) -> impl FnMut(&str) -> usize + '_ {
    |text: &str| {
        measured.set(measured.get() + text.chars().count());
        width_of(text)
    }
}

/// A long run of combining marks before a suffix that must be cut costs
/// no cell, and the cut is made in one walk: the whole text is measured
/// once, each grapheme alone and beside the one before it, and the kept
/// text once, never a prefix again (#1167).
#[test]
fn a_cell_is_cut_in_one_walk_however_many_marks_it_carries() {
    let marks = "\u{301}".repeat(1000);
    let text = format!("a{marks}bcdefgh");
    assert_eq!(width_of(&text), 8);
    assert_eq!(fitted(&text, 4), (format!("a{marks}bc…"), 4));
    let measured = std::cell::Cell::new(0);
    let cut = table::fit_by(&text, 4, &mut counting(&measured));
    assert_eq!(cut.text, format!("a{marks}bc…"));
    // The whole text (1008); `a` and its marks as the first pair and
    // alone (2002), `b` beside them and alone (1003), `c` likewise (3),
    // the `cd` that passes the budget of 3 (2): nothing after the cut;
    // the kept `a`, its marks, `bc` (1003) and the ellipsis (1).
    assert_eq!(measured.get(), 1008 + (2002 + 1003 + 3 + 2) + (1003 + 1));
    assert_eq!(fitted("abc", 3), ("abc".to_string(), 3), "it fits");
}

/// A width that hangs on the grapheme before (a Lisu tone letter after
/// another draws no cell of its own) is read as the TUI's measure reads
/// the whole name: eight cells fit an eight-cell column whole, and the
/// number after it stays aligned (#1167).
#[test]
fn a_width_that_hangs_on_its_neighbour_is_read_as_the_whole_name_reads() {
    let name = "abcdefg\u{A4F9}\u{A4FC}";
    assert_eq!((width_of(name), width_of("\u{A4FC}")), (8, 1));
    assert_eq!(fitted(name, 8), (name.to_string(), 8));
    let columns = [table::Column::left("STATE"), table::Column::right("N")];
    let rows = vec![
        vec![Safe::new(name), Safe::new("1")],
        vec![Safe::new("ab"), Safe::new("22")],
    ];
    let drawn = table::table(&columns, &rows, &Style::plain(12));
    let want = format!("STATE      N\n{name}   1\nab        22\n");
    assert_eq!(drawn, want);
}

/// Tifinagh `ⵏ⵿` draws two cells and `ⵏ⵿ⴾ` one: the joiner joins the
/// pair, so the width falls as the name goes on.
const JOINED: &str = "\u{2D4F}\u{2D7F}\u{2D3E}";

/// A name that fits its column is never cut, though a prefix of it is
/// wider than the whole: `abcdefgⵏ⵿ⴾ` is eight cells in eight (#1167).
#[test]
fn a_cell_that_fits_is_never_cut_where_its_width_falls() {
    let name = format!("abcdefg{JOINED}");
    assert_eq!(
        (width_of(&name), width_of("abcdefg\u{2D4F}\u{2D7F}")),
        (8, 9)
    );
    assert_eq!(fitted(&name, 8), (name, 8));
}

/// Two names, nine cells and eight whose width falls, in a column with
/// room for both: the next column starts at the same cell in each row.
#[test]
fn the_next_column_starts_where_it_does_after_a_width_that_falls() {
    let name = format!("abcdefg{JOINED}");
    let columns = [
        table::Column::left("STATE"),
        table::Column::right("ENTERED"),
    ];
    let rows = vec![
        vec![Safe::new("abcdefghi"), Safe::new("1")],
        vec![Safe::new(&name), Safe::new("2")],
    ];
    let drawn = table::table(&columns, &rows, &Style::plain(80));
    let want = format!("STATE      ENTERED\nabcdefghi        1\n{name}         2\n");
    assert_eq!(drawn, want);
}

/// A cut reports the cells of the text it prints, measured, never the
/// walk's sum: where the kept text ends before or after a width that
/// falls, the padding reads what the terminal draws.
#[test]
fn a_cut_reports_the_cells_of_the_text_it_prints() {
    let text = format!("ab{JOINED}cdef{JOINED}XYZ");
    assert_eq!(fitted(&text, 8), (format!("ab{JOINED}cdef…"), 8));
    assert_eq!(fitted(&text, 9), (format!("ab{JOINED}cdef…"), 8));
    for max in 1..width_of(&text) {
        let (printed, cells) = fitted(&text, max);
        assert_eq!(cells, width_of(&printed), "at {max}: {printed}");
    }
}

/// Tifinagh joins that overlap, `ⵏ⵿ⴾ⵿ⵏ`: the walk counts each pair's
/// fall, and the run falls less than their sum.
const OVERLAPPED: &str = "\u{2D4F}\u{2D7F}\u{2D3E}\u{2D7F}\u{2D4F}";

/// Where joins overlap, a cut still fits its column and reports the
/// cells it prints, so the next column starts where it does after a
/// plain name (#1167).
#[test]
fn a_cut_where_joins_overlap_fits_its_column() {
    let text = format!("abcd{OVERLAPPED}efXYZ");
    assert_eq!(fitted(&text, 8), ("abcd\u{2D4F}\u{2D7F}…".to_string(), 7));
    for max in 1..width_of(&text) {
        let (printed, cells) = fitted(&text, max);
        let measured = (cells, cells <= max);
        assert_eq!(measured, (width_of(&printed), true), "at {max}: {printed}");
    }
    let columns = [
        table::Column::left("STATE"),
        table::Column::right("ENTERED"),
    ];
    let rows = vec![
        vec![Safe::new("abcdefghij"), Safe::new("1")],
        vec![Safe::new(&text), Safe::new("2")],
    ];
    let drawn = table::table(&columns, &rows, &Style::plain(16));
    let want = "STATE    ENTERED\nabcdefg…       1\nabcd\u{2D4F}\u{2D7F}…        2\n";
    assert_eq!(drawn, want);
}

/// A run of joins whose walk runs far past its column, as each `ⵏ⵿`
/// after the first costs it nothing, is cut to fit in work linear in the
/// text: the whole text, the walk, and each halving of the kept graphemes
/// are measured once each, never a prefix grapheme by grapheme (#1167).
#[test]
fn a_cut_where_joins_overlap_is_made_in_linear_work() {
    let text = format!("ab{}XYZ", "\u{2D4F}\u{2D7F}".repeat(200));
    let measured = std::cell::Cell::new(0);
    let cut = table::fit_by(&text, 8, &mut counting(&measured));
    let kept = format!("ab{}…", "\u{2D4F}\u{2D7F}".repeat(2));
    assert_eq!(
        (width_of(&text), cut.text, cut.cells),
        (205, kept.into(), 5)
    );
    // Of 405 scalars in 1205 bytes: the whole text (405); the walk keeps
    // all 205 graphemes, each beside the one before (4 × 200 + 9) and
    // alone (405); the kept text whole, then `ab` and the joins within
    // 602, 301, 148, 73, 34 and 16 bytes, 100, 49, 24, 11, 5 and 2 joins
    // (405 + 202 + 100 + 50 + 24 + 12 + 6); the ellipsis (1).
    let work = 405 + (809 + 405) + (405 + 202 + 100 + 50 + 24 + 12 + 6) + 1;
    assert_eq!(measured.get(), work);
}

/// A long grapheme kept before the joins is measured in only the tries
/// whose bytes it fits within: halving the kept bytes, not the kept
/// graphemes, drops it once half the text is shorter, so its marks are
/// never measured once per try and the work stays linear (#1167).
#[test]
fn a_cut_after_a_long_grapheme_where_joins_overlap_is_made_in_linear_work() {
    let marks = "\u{301}".repeat(1000);
    let joins = "\u{2D4F}\u{2D7F}".repeat(1000);
    let text = format!("a{marks}b{joins}XYZ");
    let measured = std::cell::Cell::new(0);
    let cut = table::fit_by(&text, 8, &mut counting(&measured));
    // Of 3005 scalars in 8005 bytes: the whole text (3005); the walk
    // keeps every grapheme, `a` and its marks beside nothing and alone
    // (2002), `b` (1003), the first join (5), the other 999 (6 each),
    // `XYZ` (10); the kept text whole (3005), then within 4002 bytes `a`,
    // its marks, `b` and 333 joins (1668), then within 2000 bytes none,
    // as `a` and its marks take 2001: the marks are measured in two
    // tries, not in one per halving; the ellipsis (1).
    let walk = 2002 + 1003 + 5 + 999 * 6 + 10;
    assert_eq!(measured.get(), 3005 + walk + (3005 + 1668) + 1);
    assert_eq!((cut.text, cut.cells), ("…".into(), 1));
}

/// A whole table is measured once per cell to size it, and once more to
/// set it, with one walk and one measure of the kept text where it is
/// cut: the padding reads the cells the cut returned, so the kept text is
/// never walked again (#1167).
#[test]
fn a_row_is_padded_by_the_cells_its_cut_returned() {
    let columns = [table::Column::left("NAME"), table::Column::right("N")];
    let rows = vec![vec![Safe::new(&"x".repeat(100)), Safe::new("1")]];
    let measured = std::cell::Cell::new(0);
    let style = Style::plain(20);
    let drawn = table::table_by(&columns, &rows, &style, &mut counting(&measured));
    let kept = "x".repeat(17);
    assert_eq!(drawn, format!("NAME               N\n{kept}… 1\n"));
    // Sizing, each cell whole: `NAME` 4, `N` 1, the hundred `x`s 100,
    // `1` 1. Setting: `NAME` 4, `N` 1; the `x`s whole (100), walked to
    // the eighteenth that passes the budget of 17 (52), the seventeen
    // kept (17) and the ellipsis (1); `1` 1.
    assert_eq!(measured.get(), (4 + 1 + 100 + 1) + (4 + 1 + 170 + 1));
}

/// A grapheme whose width hangs on its sequence is measured whole: `♥`
/// alone is one cell, `♥` then U+FE0F two, so an eight-cell column cuts
/// before it and pads, and the next column stays where it is (#1167).
#[test]
fn a_cell_is_cut_and_padded_by_its_graphemes_widths() {
    let heart = "♥\u{FE0F}";
    assert_eq!((width_of("♥"), width_of(heart)), (1, 2));
    let text = format!("abcdef{heart}XYZ");
    assert_eq!(fitted(&text, 8), ("abcdef…".to_string(), 7));
    assert_eq!(fitted(&text, 9), (format!("abcdef{heart}…"), 9));
    let columns = [table::Column::left("NAME"), table::Column::right("N")];
    let rows = vec![vec![Safe::new(&text), Safe::new("1")]];
    let drawn = table::table(&columns, &rows, &Style::plain(10));
    assert_eq!(drawn, "NAME     N\nabcdef…  1\n");
    let fits = vec![vec![Safe::new(&format!("ab{heart}")), Safe::new("1")]];
    let drawn = table::table(&columns, &fits, &Style::plain(10));
    assert_eq!(
        drawn, "NAME  N\nab♥\u{FE0F}  1\n",
        "padded by its two cells"
    );
}

/// The child half of the test below names the file it reports to here.
const CONTROLLED_REPORT: &str = "BROKKR_TEST_CONTROLLED_REPORT";

/// A pseudo-terminal `columns` wide: its controller and the terminal
/// a process is handed, opened without becoming this one's.
fn pty(columns: u16) -> (std::os::fd::OwnedFd, std::os::fd::OwnedFd) {
    use rustix::fs::{open, Mode, OFlags};
    use rustix::pty::{grantpt, openpt, ptsname, unlockpt, OpenptFlags};
    let controller = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).unwrap();
    grantpt(&controller).unwrap();
    unlockpt(&controller).unwrap();
    let name = ptsname(&controller, Vec::new()).unwrap();
    let terminal = open(
        name.as_c_str(),
        OFlags::RDWR | OFlags::NOCTTY,
        Mode::empty(),
    )
    .unwrap();
    // Set on the terminal's side: macOS refuses a window size set through
    // the controller (ENOTTY), and Linux takes it from either side.
    let size = rustix::termios::Winsize {
        ws_row: 24,
        ws_col: columns,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    rustix::termios::tcsetwinsize(&terminal, size).unwrap();
    (controller, terminal)
}

/// `brokkr stats graph`'s production detector reads the stdout it writes to,
/// never the controlling terminal (#1167): this test re-runs itself in a
/// session of its own whose controlling terminal is 80 cells and whose
/// stdout is a 60-cell terminal, with no `COLUMNS`, and the child sizes
/// to 60. A lookup through /dev/tty, as crossterm's is, reads 80.
#[test]
fn a_60_cell_stdout_under_an_80_cell_controlling_terminal_sizes_to_60() {
    if let Some(report) = std::env::var_os(CONTROLLED_REPORT) {
        rustix::process::setsid().unwrap();
        rustix::process::ioctl_tiocsctty(std::io::stdin()).unwrap();
        let tty = std::fs::File::open("/dev/tty").unwrap();
        let controlling = rustix::termios::tcgetwinsize(&tty).unwrap().ws_col;
        let width = Style::detect_for(Width::Stdout).width;
        std::fs::write(report, format!("{controlling} {width}")).unwrap();
        return;
    }
    let (_stdout_controller, stdout) = pty(60);
    let (_tty_controller, tty) = pty(80);
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("width");
    let name =
        "render::tests::fit::a_60_cell_stdout_under_an_80_cell_controlling_terminal_sizes_to_60";
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--test-threads=1"])
        .env(CONTROLLED_REPORT, &report)
        .env_remove("COLUMNS")
        .stdin(std::process::Stdio::from(tty))
        .stdout(std::process::Stdio::from(stdout))
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "{status}");
    assert_eq!(std::fs::read_to_string(&report).unwrap(), "80 60");
}
