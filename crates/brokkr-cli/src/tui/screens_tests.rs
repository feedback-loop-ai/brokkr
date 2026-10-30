//! Every screen the console paints of one run (#753; decision 0089 slice
//! 3), by its own `draw`: the fleet level beside the run's row, its
//! dashboard and its live or findings column; the run level; and each
//! participant's level. The console's parity paints them over the run
//! read through its pages and over the run read whole, and compares them.

use super::*;
use ratatui::backend::TestBackend;

/// The terminals each screen is painted at: the snapshots' two, and the
/// 4K-class terminal the fleet's detail pane is drawn on (#491).
const SIZES: [(u16, u16); 3] = [(80, 20), (160, 48), (320, 80)];

/// The screens of `run` the console paints over `views`, and over `views`
/// with `other` as its run, each pair as the text their cells show, at
/// every size. Away from a participant, the fleet level on the run's row,
/// its dashboard and its live column scrolled to each of `offsets`, `0` its
/// top and `usize::MAX` its end, and the run level; at the participant
/// keyed `at`, its level, its checkpoints from the top and scrolled to the
/// last. A table whose fitted
/// columns overflow its pane is laid out by ratatui's solver, which this
/// workspace builds without its layout cache, and two solves of the same
/// table can shrink a different column by one: the same view painted back
/// to back differs so. So the other screen is painted again, up to
/// [`TRIES`] times, until it is the first, and a screen whose text differs
/// never is.
pub(crate) fn screens(
    views: &mut Views,
    other: RunView,
    run: &str,
    at: Option<&str>,
    offsets: &[usize],
) -> Vec<[String; 2]> {
    let mut tuis = Vec::new();
    match at {
        None => {
            for &offset in offsets {
                let mut fleet = Tui::over(None, Vec::new(), 0);
                fleet.cursor[0] = Some(run.to_string());
                (fleet.offset, fleet.live_offset) = (offset, offset);
                tuis.push(fleet);
            }
            tuis.push(Tui::over(Some(run.to_string()), Vec::new(), 0));
        }
        Some(key) => {
            let parts = views.run.iter().flat_map(|view| &view.participants);
            let shown = parts.filter(|part| part.key == key);
            let last = shown.map(|part| part.checkpoints.len().saturating_sub(1));
            for offset in [0, last.max().unwrap_or_default()] {
                let mut level = Tui::over(Some(run.to_string()), Vec::new(), 0);
                level.level = Level::Participant;
                (level.seat, level.offset) = (Some(key.to_string()), offset);
                tuis.push(level);
            }
        }
    }
    let mut other = Some(other);
    let mut painted = Vec::new();
    for tui in tuis {
        painted.extend(paint(tui, views, &mut other));
    }
    painted
}

/// The run level alone, painted over `views` and over `other`, at every
/// size: the RUN screen's parity, where the fleet's live column (which a
/// run-level read does not hold) is no screen of its own (R6).
pub(crate) fn run_level_screens(views: &mut Views, other: RunView, run: &str) -> Vec<[String; 2]> {
    let mut other = Some(other);
    paint(
        Tui::over(Some(run.to_string()), Vec::new(), 0),
        views,
        &mut other,
    )
}

/// [`screens`]' paint-and-compare, one terminal size at a time.
fn paint(mut tui: Tui, views: &mut Views, other: &mut Option<RunView>) -> Vec<[String; 2]> {
    let mut painted = Vec::new();
    for (width, height) in SIZES {
        (tui.width, tui.height) = (width, height);
        let paint = |views: &Views| {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| draw(frame, &tui, views)).unwrap();
            terminal.backend().to_string()
        };
        let first = paint(views);
        std::mem::swap(&mut views.run, other);
        let mut second = paint(views);
        for _ in 1..TRIES {
            if second == first {
                break;
            }
            second = paint(views);
        }
        std::mem::swap(&mut views.run, other);
        painted.push([first, second]);
    }
    painted
}

/// The offsets a fleet level's columns are painted at: their tops and
/// their ends.
pub(crate) const ENDS: [usize; 2] = [0, usize::MAX];

/// How many times a screen is painted to meet the one beside it.
const TRIES: usize = 32;
