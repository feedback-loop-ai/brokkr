//! What the frame says about itself: the footer's keys for the current
//! context, the status line's breadcrumb, and the help text.

use super::*;

// ------------------------------------------------------ footer and help

/// Discoverability is a requirement, not a nicety: the footer names the
/// keys available in the CURRENT context, and differs per (level, pane,
/// typing, help) so a constant footer cannot pass its test. This is the
/// footer however wide; a frame draws [`footer_within`] its width.
#[cfg(test)]
pub(crate) fn footer_for(tui: &Tui, views: &Views) -> String {
    footer_within(tui, views, usize::MAX)
}

/// [`footer_for`] on a footer `width` columns wide: the fleet's drops
/// its lesser keys until it fits (#503), and every other level's is
/// drawn as it stands.
pub(crate) fn footer_within(tui: &Tui, views: &Views, width: usize) -> String {
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
        Level::Runs => fit(runs_footer(tui, views), width),
        Level::Run => run_footer(tui, views, tail),
        Level::Participant => participant_footer(tui, views, tail),
    }
}

/// One key a footer names, and how readily it gives way on a narrow
/// frame: `None` never does, and a higher rank goes first.
type Part = (String, Option<u8>);

/// The fleet's footer, at every width (#503): it always names `Enter`,
/// `Tab`, `a` and `/`. The list's, or the detail pane's while it has
/// the focus. `Tab` says where the pane is when the frame is too narrow
/// for it. The tab keys are said where they are bound, and only there:
/// a one-hearth world's footer never names them.
fn runs_footer(tui: &Tui, views: &Views) -> Vec<Part> {
    let all = match tui.all {
        true => "a recent only",
        false => "a all runs",
    };
    let tab = match (detail_focused(tui, views), tui.width >= DETAIL_MIN_WIDTH) {
        (true, _) => "Tab list".to_string(),
        (false, true) => "Tab detail".to_string(),
        (false, false) => format!("Tab detail ≥{DETAIL_MIN_WIDTH}"),
    };
    let movement = match detail_focused(tui, views) {
        true => "↑↓/jk scroll",
        false => "↑↓/jk move",
    };
    let mut parts: Vec<Part> = vec![
        (movement.to_string(), Some(3)),
        ("Enter open run".to_string(), None),
    ];
    parts.extend(in_place(tui, views).map(|keys| (keys.to_string(), Some(2))));
    parts.extend(tabbed(tui).then(|| ("[ ] 1-9 realm".to_string(), Some(5))));
    parts.extend([
        (tab, None),
        (all.to_string(), None),
        ("g/G top/bottom".to_string(), Some(7)),
        ("/ filter".to_string(), None),
        ("r refresh".to_string(), Some(6)),
        ("? help".to_string(), Some(1)),
        ("q quit".to_string(), Some(4)),
    ]);
    parts
}

/// What `←→` do to the selected row, when it is a running run.
fn in_place(tui: &Tui, views: &Views) -> Option<&'static str> {
    let row = selected_run(tui, views)?;
    match (row.verdict.standing, opened(tui, row)) {
        (Standing::Running, true) => Some("← fold"),
        (Standing::Running, false) => Some("→ expand"),
        (
            Standing::Quarantined
            | Standing::Parked
            | Standing::Shipped
            | Standing::Stopped
            | Standing::OperatorStopped,
            _,
        ) => None,
    }
}

/// `parts` joined, dropping the most readily dropped until the line
/// fits `width`. The parts that never give way stay however narrow.
fn fit(mut parts: Vec<Part>, width: usize) -> String {
    loop {
        let words: Vec<&str> = parts.iter().map(|(words, _)| words.as_str()).collect();
        let joined = words.join(" · ");
        let first = parts.iter().filter_map(|(_, rank)| *rank).max();
        match first.filter(|_| width_of(&joined) > width) {
            Some(first) => parts.retain(|(_, rank)| *rank != Some(first)),
            None => return joined,
        }
    }
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

pub(super) const HELP: [&str; 14] = [
    "brokkr tui — a read-only console over the same models as",
    "brokkr inspect, brokkr watch and brokkr ui. It issues no",
    "operator commands and writes nothing to the journal.",
    "",
    "↑ ↓ j k     move          Enter   descend / scope",
    "← →         the graph rail        ↑ ↓ its lanes",
    "→ l Space   open a running run in the fleet    ← h  fold it",
    "Esc         back          ⌫       back (keeps the scope)",
    "Tab         next pane     g G     top / bottom",
    "PgUp PgDn   page          /       filter this list",
    "r           refresh       ?       this help",
    "q Ctrl+C    quit          a       all runs, older ones too",
    "",
    "Selecting a phase or a seat scopes the run level; Esc clears it.",
];
