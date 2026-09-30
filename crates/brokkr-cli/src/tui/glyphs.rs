//! The graph's vocabulary: node classes and their glyphs, the pulse,
//! the one width measurement, label clamping, and the degradation modes.

use super::*;

// --------------------------------------------------------------- the graph
//
// The console's grammar (`ui.html::renderLoops`), drawn in characters:
// ONE horizontal rail; sequential steps joined by arrowed edges that
// read as *then*; a panel FORKING into vertically symmetric lanes that
// REJOIN the rail before the next step — the rejoin *is* the join
// dependency, and it is the one thing the 0013 tree could not say;
// phase names on one shared baseline with `×N` revisit markers; and the
// active node pulsing while the run is live.
//
// **A character grid, and deliberately not ratatui's plotting surface.**
// Colour on that surface is stored per cell, last writer wins, so a node
// circle drawn over a rail recolours the rail's cells — which makes a
// fixed per-node colour vocabulary structurally unsatisfiable wherever a
// node shares a cell with an edge, and at terminal densities that is
// most nodes. Its own text call is a third path into a buffer whose
// apparent safety is borrowed from ratatui's undocumented filtering
// rather than from `Safe`'s enumerated table. And at five inner rows
// there is no sub-cell resolution to win: a `●` at one-cell scale is a
// truer circle than any dot-matrix approximation of one. What is
// conceded is stroke fidelity; the grammar being matched is topological
// — rail, arrow, fork, rejoin, baseline — and every element of it
// survives at cell resolution. The ruling is held by a test over this
// file's own source, not by memory.
//
// **Two functions, one boundary.** [`plan`] is pure owned integer
// geometry over the models and the rect; [`paint`] walks that plan and
// emits spans through the one sanitized constructor. The plan fits by
// construction, so the painter has no clipping branch — a painter that
// clips is a painter whose failure mode is invisible.

/// The arrowed edge between two steps: `──→`, the console's *then*.
pub(super) const ARROW_WIDTH: usize = 3;

/// How the selection box breathes: this many columns from a wall to the
/// nearest glyph of the phase it holds, on EVERY side. Two, because one
/// is not breathing room and because a wall two columns out can never
/// land on the arrowhead that flies into the node — the head sits one
/// column off the rail content, inside the boundary where it belongs.
pub(super) const BOX_PAD: usize = 2;

/// A label clamps here before anything structural gives way.
pub(super) const LABEL_MAX: usize = 14;

/// A corrupt fold must not put twenty digits on the name baseline.
pub(super) const VISITS_MAX: u64 = 99;

/// The node vocabulary: `ui.html`'s `NODE_CLASS` allowlist plus its
/// `phase.current × summary.status` branch, transliterated. Seven
/// classes — deliverable 2's six, plus the one fallback arm Rust obliges.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Class {
    Visited,
    Current,
    Park,
    Failed,
    Finished,
    Active,
    Unknown,
}

/// One classification, one rendering of it: a **(colour, marker ramp)**
/// pair per class. The ramp is the pulse's four frames; classes that do
/// not pulse repeat one glyph, so the frame index is inert for them and
/// costs no branch. `Active`'s still glyph differs from `Finished`'s in
/// the GLYPH channel, so the distinction survives a terminal with no
/// colour and an operator with no animation.
///
/// This is deliberately **not** `render::tone`: `tone` maps
/// `awaiting_operator` to `Quiet`, while the graph needs **park** as a
/// distinct yellow class, and widening `tone` would move `brokkr runs`'
/// colour. One classification per question, not one table for two.
/// Node glyphs are calibrated BY THE OPERATOR'S EYE, like the
/// arrowhead: `⏺` (U+23FA) is the filled node whose centre sits on the
/// dash axis in their font, and the math operators `⊗`/`⊙` share that
/// axis by design. The geometric shapes `●○◉◎` do not — they are the
/// reason this table exists. Every live class pulses on the SAME ramp,
/// so all live nodes breathe in phase — and the ramp STARTS on `∙`, so
/// a live node differs from a finished `⏺` in the glyph channel even
/// with colour off and animation frozen (the property the vocabulary
/// test pins).
pub(super) const LIVE_RAMP: [&str; 4] = ["∙", "⏺", "∙", "·"];

pub(super) fn look(class: Class) -> (Style, [&'static str; 4]) {
    match class {
        Class::Visited => (
            Style::new().fg(Color::Magenta).add_modifier(Modifier::DIM),
            ["⏺", "⏺", "⏺", "⏺"],
        ),
        Class::Current => (Style::new().fg(Color::Green), LIVE_RAMP),
        Class::Park => (Style::new().fg(Color::Yellow), ["⊙", "⊙", "⊙", "⊙"]),
        Class::Failed => (Style::new().fg(Color::Red), ["⊗", "⊗", "⊗", "⊗"]),
        Class::Finished => (Style::new().fg(Color::Green), ["⏺", "⏺", "⏺", "⏺"]),
        Class::Active => (
            Style::new().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            LIVE_RAMP,
        ),
        Class::Unknown => (
            Style::new().add_modifier(Modifier::DIM),
            ["·", "·", "·", "·"],
        ),
    }
}

/// A phase's own rail node. An unrecognised `summary.status` renders
/// **quiet, never live** — a deliberate, named divergence from
/// `ui.html`, which falls through to green. The terminal declines to
/// guess, in the manner of `render::tone`'s own `_ => Tone::Quiet`.
pub(super) fn class_for_phase(current: bool, status: &str) -> Class {
    match (current, status) {
        (false, _) => Class::Visited,
        (true, "running" | "completed") => Class::Current,
        (true, "awaiting_operator") => Class::Park,
        (true, "stopped") => Class::Failed,
        (true, _) => Class::Unknown,
    }
}

/// A node inside a phase, from the model's closed-set key. **No journal
/// string reaches this table**, and an unlisted key is the quiet arm.
pub(super) fn class_for_node(state_class: &str) -> Class {
    match state_class {
        "on-phosphor" => Class::Finished,
        "in-active" => Class::Active,
        "on-park" => Class::Park,
        "on-halt" => Class::Failed,
        _ => Class::Unknown,
    }
}

/// The pulse: a pure, total function of a tick counter and two model
/// facts. It reads no store — there is nothing here to read *from* — and
/// it moves the GLYPH, never the colour and never a position, so a live
/// node can never look briefly parked and geometry never varies with
/// `tick`. `!live` or `!animate` is frame 0, the still frame, at every
/// tick, so an idle console costs exactly nothing extra.
pub(super) fn pulse(tick: usize, live: bool, animate: bool) -> usize {
    match live && animate {
        true => (tick / PULSE_TICKS) % PULSE_FRAMES,
        false => 0,
    }
}

/// ratatui's own measurement of the sanitized text — which is what the
/// buffer will actually draw. `Safe::width()` is a `char` count and
/// reports 6 for `設計フェーズ` where the terminal draws 12; the rail is
/// the first pane here that places its own x positions, so it is the
/// first that can be lied to. **Every width in the graph comes from
/// here**, so there is no second measurement to disagree with the first.
pub(super) fn width_of(text: &str) -> usize {
    span(text, plain()).width()
}

/// Clamp to a display width, marking the cut with `…`. Text is what
/// gives way when a segment is tight; the rail, the arrows, the fork
/// corners and the rejoin never are.
pub(super) fn clamp(text: &str, max: usize) -> String {
    let text = safe(text);
    // TWO bounds, deliberately. Display width is what the layout plans
    // in, but the buffer consumes a cell per `char`, and a zero-width
    // char (a combining mark, a variation selector) costs 0 columns and
    // 1 cell. Bounding on width alone let a label of N such marks plan
    // one column and overwrite N — erasing the rail, the arrows and a
    // neighbour's state glyph. Bounding on the char count too is
    // structural: it holds for every zero-width class, present and
    // future, where enumerating them would not.
    if width_of(&text) <= max && text.chars().count() <= max {
        return text;
    }
    if max == 0 {
        return String::new();
    }
    let mut out = String::new();
    for character in text.chars() {
        let mut wider = out.clone();
        wider.push(character);
        if width_of(&wider) + 1 > max || wider.chars().count() + 1 > max {
            break;
        }
        out = wider;
    }
    out.push('…');
    out
}

/// `×N` only when the phase was revisited — the console's rule, not
/// today's unconditional `×1` — and clamped, because `visits` is a `u64`
/// crossing `VIEW_VERSION` rather than a promise.
pub(super) fn visits_text(visits: u64) -> Option<String> {
    match visits {
        0 | 1 => None,
        2..=VISITS_MAX => Some(format!("×{visits}")),
        _ => Some(format!("×{VISITS_MAX}+")),
    }
}

/// The name baseline's text: the scope marker the crate's ONE phase
/// predicate decides — called, never reimplemented — then the name,
/// then the revisit marker.
pub(super) fn name_text(phase: &Phase, lens: Option<&render::Lens>) -> String {
    let mut text = String::new();
    if lens.is_some() && render::keeps_phase(lens, phase) {
        text.push('▸');
    }
    text.push_str(&phase.name);
    if let Some(strategy) = &phase.strategy {
        text.push_str(" · ");
        text.push_str(strategy);
    }
    if let Some(visits) = visits_text(phase.visits) {
        text.push(' ');
        text.push_str(&visits);
    }
    text
}

/// What a column drawn as one node is called: the step's own name when
/// the model gave it one, otherwise the node's, and nothing at all for a
/// column the derivation left empty.
pub(super) fn column_label(column: &Column) -> &str {
    match (&column.label, column.nodes.first()) {
        (Some(label), _) => label,
        (None, Some(node)) => &node.label,
        (None, None) => "",
    }
}

/// A label's footprint beside its node: one space and the text, or
/// nothing at all when there is no text.
pub(super) fn label_span(label: &str) -> usize {
    match label.is_empty() {
        true => 0,
        false => 1 + width_of(label),
    }
}

/// Which member speaks for a column drawn as a single node: the worst
/// state wins, so a compacted fork never reads healthier than its
/// members do, and a still-working member outranks a finished one.
pub(super) fn worst(nodes: &[Node]) -> usize {
    let rank = |node: &Node| match node.state_class.as_str() {
        "on-halt" => 0,
        "on-park" => 1,
        "in-active" => 3,
        "on-phosphor" => 4,
        _ => 2,
    };
    nodes
        .iter()
        .enumerate()
        .min_by_key(|(_, node)| rank(node))
        .map_or(0, |(index, _)| index)
}

/// Where member `k` of `n` sits, as a row offset from the rail:
/// symmetric about it, and for an even count the rail row itself is left
/// to the rail. The lane span a fork needs is therefore `n / 2`.
pub(super) fn lane_offset(k: usize, n: usize) -> isize {
    let half = (n / 2) as isize;
    let raw = k as isize - half;
    match n % 2 == 1 || raw < 0 {
        true => raw,
        false => raw + 1,
    }
}

/// Degradation's vertical axis. Lanes are vertical, and at `MIN_HEIGHT`
/// the graph pane's inner rect is about one row — no fork fits at any
/// terminal `refuse()` admits. Three named modes and one predicate;
/// every mode that cannot draw lanes still says `⑂n`, so the small forms
/// are compact rather than a lie about the shape of the run.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Mode {
    /// Rail, arrows, symmetric lanes, rejoin and the name baseline.
    Full,
    /// Rail, arrows and names; every fork collapses to one rail node
    /// bearing `⑂n` and the worst member's state.
    Rail,
    /// One row: one node per phase, its name and its `⑂n`, arrowed.
    Compressed,
}

pub(super) fn mode_for(rows: usize, lane_span: usize) -> Mode {
    if rows < 2 {
        return Mode::Compressed;
    }
    if rows < 4 || lane_span == 0 {
        return Mode::Rail;
    }
    Mode::Full
}
