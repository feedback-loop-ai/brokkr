//! What the frame says about itself: the footer's keys for the current
//! context, the status line's breadcrumb, and the help text.

use super::*;

// ------------------------------------------------------ footer and help

/// Discoverability is a requirement, not a nicety: the footer names the
/// keys available in the CURRENT context, and differs per (level, pane,
/// typing, help) so a constant footer cannot pass its test.
pub(crate) fn footer_for(tui: &Tui, views: &Views) -> String {
    if tui.help {
        return "? or Esc close help · q quit".to_string();
    }
    if tui.reading.is_some() {
        return "↑↓/jk scroll · PgUp/PgDn page · g top · Esc or Enter close · q quit".to_string();
    }
    if tui.typing {
        let mut line = String::from("/");
        line.push_str(safe(&tui.filter).as_str());
        line.push_str("▏ filtering · Enter keep · Esc clear · ⌫ delete");
        return line;
    }
    let tail = "· / filter · r refresh · ? help · q quit";
    match tui.level {
        Level::Runs => runs_footer(tui, views, tail),
        Level::Run => run_footer(tui, views, tail),
        Level::Participant => participant_footer(tui, views, tail),
    }
}

/// The fleet's footer: the list's, or the detail pane's while it has
/// the focus. The tab keys are said where they are bound, and only
/// there: a one-hearth world's footer never names them.
fn runs_footer(tui: &Tui, views: &Views, tail: &str) -> String {
    let all = match tui.all {
        true => "a recent only",
        false => "a all runs",
    };
    if detail_focused(tui, views) {
        return format!("↑↓/jk scroll · Enter open run · Tab list · {all} {tail}");
    }
    let realm = match tabbed(tui) {
        true => " · [ ] 1-9 realm",
        false => "",
    };
    let detail = match detail_row(tui, views) {
        Some(_) => " · Tab detail",
        None => "",
    };
    format!("↑↓/jk move · Enter open run{realm}{detail} · {all} · g/G top/bottom {tail}")
}

/// The run level's footer, one per pane.
fn run_footer(tui: &Tui, views: &Views, tail: &str) -> String {
    match tui.pane {
        // The lane cursor scopes, so the footer must say so where it
        // happens (decision 0014's discoverability rule): an operator
        // who watches the seats and the trail narrow under `↑↓` should
        // read WHY on the same line, named by the seat's own label —
        // what the seats pane displays — rather than by its raw key.
        // It says so only while that member IS the scope: `Enter` and
        // `j`/`k` both re-scope the phase and leave the lane cursor
        // standing, and a footer naming a seat the panes are not
        // filtered to would contradict the status line above it.
        0 => {
            let named =
                lane_member(tui, views).filter(|part| scoped_seat(tui) == Some(part.key.as_str()));
            let lanes = match named {
                Some(part) => format!("↑↓ lanes · scoped to {}", safe(&part.label)),
                None => "↑↓ lanes".to_string(),
            };
            format!("←→ rail · {lanes} · Enter scope phase · Tab pane · Esc back {tail}")
        }
        1 => {
            let verb = match (scoped_seat(tui), tui.cursor[1].as_deref()) {
                (Some(scoped), Some(cursor)) if scoped == cursor => "Enter open seat",
                _ => "Enter scope seat",
            };
            format!("↑↓/jk move · {verb} · Tab pane · Esc back {tail}")
        }
        2 => format!("↑↓/jk move · Enter read row · Tab pane · Esc back {tail}"),
        _ => format!("↑↓/jk move · Tab pane · Esc back {tail}"),
    }
}

/// The participant level's footer, one per pane.
fn participant_footer(tui: &Tui, views: &Views, tail: &str) -> String {
    match tui.pane {
        // Two doors, so two footers: the key that opens the turn under
        // the cursor, and the key that opens the whole transcript —
        // each named where it is the one Enter does, with `Esc
        // unselect` naming the way back to the other.
        1 => {
            // A refused or unavailable read has both doors disabled, so
            // the footer must not advertise one. A `None` transcript is
            // the old no-subject pane, unchanged.
            if views
                .transcript
                .as_ref()
                .is_some_and(|read| !read.is_readable())
            {
                format!("↑↓/jk move · no reading door · Tab pane · Esc back {tail}")
            } else if tui.turn.is_some() {
                format!("↑↓/jk move · Enter read turn · Esc unselect · g/G top/bottom · Tab pane {tail}")
            } else {
                format!(
                    "↑↓/jk move · Enter read whole transcript · g/G top/bottom · Tab pane · Esc back {tail}"
                )
            }
        }
        _ => format!("↑↓/jk scroll · g/G top/bottom · Tab pane · Esc back {tail}"),
    }
}

/// The breadcrumb, or the sentence a transient store error earns. One
/// line, always drawn: an operator must be able to see WHY a frame is
/// standing still.
pub(super) fn status_line(tui: &Tui) -> String {
    if let Some(status) = &tui.status {
        return status.clone();
    }
    let mut line = String::from("runs");
    // Which hearth's runs, in a world that has more than one to be at.
    // `tab` is in range by construction: [`Tui::over`] clamps it and
    // [`switch`] refuses an index the world does not have.
    if tabbed(tui) {
        line.push_str(" · realm ");
        line.push_str(safe(&tui.tabs[tui.tab]).as_str());
    }
    if let Some(run) = &tui.run {
        line.push_str(" · run ");
        line.push_str(safe(run).as_str());
    }
    if let Some(seat) = &tui.seat {
        line.push_str(" · seat ");
        line.push_str(safe(seat).as_str());
    }
    match &tui.scope {
        Some(render::Scope::Phase(name)) => {
            line.push_str(" · phase ");
            line.push_str(safe(name).as_str());
        }
        Some(render::Scope::Seat(key)) => {
            line.push_str(" · scoped ");
            line.push_str(safe(key).as_str());
        }
        None => {}
    }
    line
}

pub(super) const HELP: [&str; 13] = [
    "brokkr tui — a read-only console over the same models as",
    "brokkr inspect, brokkr watch and brokkr ui. It issues no",
    "operator commands and writes nothing to the journal.",
    "",
    "↑ ↓ j k     move          Enter   descend / scope",
    "← →         the graph rail        ↑ ↓ its lanes",
    "Esc         back          ⌫       back (keeps the scope)",
    "Tab         next pane     g G     top / bottom",
    "PgUp PgDn   page          /       filter this list",
    "r           refresh       ?       this help",
    "q Ctrl+C    quit          a       all runs, older ones too",
    "",
    "Selecting a phase or a seat scopes the run level; Esc clears it.",
];
