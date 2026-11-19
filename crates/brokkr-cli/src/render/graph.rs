//! The phase graph's one classification and its one colour choice, read
//! by both surfaces that draw the graph: the plain frame below paints a
//! node's class in ANSI, and the console (`tui/glyphs.rs::look`) paints
//! the same class in ratatui (#1168). A class names its colour in the
//! small vocabulary here, so the two renderings cannot drift apart.

use brokkr_view::{keeps_phase, Lens, Phase, RunView};

use super::{push_line, tint, Safe, Style, BOLD, DIM, GREEN, RED};

const MAGENTA: &str = "\x1b[35m";
const YELLOW: &str = "\x1b[33m";

/// The node vocabulary: `ui.html`'s `NODE_CLASS` allowlist plus its
/// `phase.current × summary.status` branch, transliterated. Seven
/// classes — deliverable 2's six, plus the one fallback arm Rust obliges.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Class {
    Visited,
    Current,
    Park,
    Failed,
    Finished,
    Active,
    Unknown,
}

/// The graph's colours, named once for both surfaces.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Hue {
    Magenta,
    Green,
    Yellow,
    Red,
}

/// The graph's intensity beside its hue.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Weight {
    Normal,
    Dim,
    Bold,
}

/// A class's colour: a hue, or the terminal's own where there is none,
/// and a weight.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Paint {
    pub(crate) hue: Option<Hue>,
    pub(crate) weight: Weight,
}

/// Every live class pulses on the SAME ramp, so all live nodes breathe
/// in phase — and the ramp STARTS on `∙`, so a live node differs from a
/// finished `⏺` in the glyph channel even with colour off and animation
/// frozen. The plain frame prints that first, still frame.
pub(crate) const LIVE_RAMP: [&str; 4] = ["∙", "⏺", "∙", "·"];

/// One classification, one rendering of it: a **(paint, marker ramp)**
/// pair per class. The ramp is the pulse's four frames; classes that do
/// not pulse repeat one glyph.
///
/// This is deliberately **not** `render::tone`: `tone` maps
/// `awaiting_operator` to `Quiet`, while the graph needs **park** as a
/// distinct yellow class, and widening `tone` would move `brokkr runs`'
/// colour. One classification per question, not one table for two.
/// Node glyphs are calibrated BY THE OPERATOR'S EYE, like the
/// arrowhead: `⏺` (U+23FA) is the filled node whose centre sits on the
/// dash axis in their font, and the math operators `⊗`/`⊙` share that
/// axis by design. The geometric shapes `●○◉◎` do not.
pub(crate) fn mark(class: Class) -> (Paint, [&'static str; 4]) {
    let paint = |hue, weight| Paint { hue, weight };
    match class {
        Class::Visited => (paint(Some(Hue::Magenta), Weight::Dim), ["⏺"; 4]),
        Class::Current => (paint(Some(Hue::Green), Weight::Normal), LIVE_RAMP),
        Class::Park => (paint(Some(Hue::Yellow), Weight::Normal), ["⊙"; 4]),
        Class::Failed => (paint(Some(Hue::Red), Weight::Normal), ["⊗"; 4]),
        Class::Finished => (paint(Some(Hue::Green), Weight::Normal), ["⏺"; 4]),
        Class::Active => (paint(Some(Hue::Magenta), Weight::Bold), LIVE_RAMP),
        Class::Unknown => (paint(None, Weight::Dim), ["·"; 4]),
    }
}

/// A phase's own rail node. An unrecognised `summary.status` renders
/// **quiet, never live** — a deliberate, named divergence from
/// `ui.html`, which falls through to green. The terminal declines to
/// guess, in the manner of `render::tone`'s own `_ => Tone::Quiet`.
pub(crate) fn class_for_phase(current: bool, status: &str) -> Class {
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
pub(crate) fn class_for_node(state_class: &str) -> Class {
    match state_class {
        "on-phosphor" => Class::Finished,
        "in-active" => Class::Active,
        "on-park" => Class::Park,
        "on-halt" => Class::Failed,
        _ => Class::Unknown,
    }
}

/// The plain frame's rendering of a paint: its ANSI codes.
pub(crate) fn ansi(paint: Paint) -> String {
    let hue = match paint.hue {
        Some(Hue::Magenta) => MAGENTA,
        Some(Hue::Green) => GREEN,
        Some(Hue::Yellow) => YELLOW,
        Some(Hue::Red) => RED,
        None => "",
    };
    let weight = match paint.weight {
        Weight::Normal => "",
        Weight::Dim => DIM,
        Weight::Bold => BOLD,
    };
    format!("{hue}{weight}")
}

/// `text` in `class`'s paint, when the terminal takes colour.
fn painted(text: &str, class: Class, style: &Style) -> String {
    tint(text, &ansi(mark(class).0), style)
}

/// A node's still glyph in its class's paint.
fn node_glyph(state_class: &str, style: &Style) -> String {
    let class = class_for_node(state_class);
    painted(mark(class).1[0], class, style)
}

/// The phase graph as a terminal tree: `⑂` precedes parallel members and
/// `→` precedes a sequential step, both nested under their phase. The
/// markers are content, not colour, and are therefore unconditional —
/// the models already emit `Σ`, `↓`, `…` and `—` in pre-baked text, so
/// an ASCII mode would need a second derivation of every one of them.
/// Nodes read as their class's still glyph, name and state, the glyph in
/// the console's colour for that class, and the current phase's name in
/// its phase class's colour, as the console paints it. The served model
/// is a seat's field, printed by the seats table and the seat block,
/// never by the rail — the graph is the abstraction over order and shape.
pub(super) fn graph_block(view: &RunView, lens: Option<&Lens>, style: &Style) -> String {
    let phases: Vec<&Phase> = view
        .phases
        .iter()
        .filter(|phase| keeps_phase(lens, phase))
        .collect();
    if phases.is_empty() {
        return String::new();
    }
    let status = view
        .summary
        .as_ref()
        .map_or("", |summary| summary.status_word());
    let mut out = String::from("graph\n");
    for phase in phases {
        let name = Safe::new(&phase.name);
        let (name, current) = match phase.current {
            true => (
                painted(name.as_str(), class_for_phase(true, status), style),
                "  ←current",
            ),
            false => (name.as_str().to_string(), ""),
        };
        push_line(
            &mut out,
            &format!(
                "  {name}{} ×{}{current}",
                phase
                    .strategy
                    .as_ref()
                    .map(|strategy| format!(" · {}", Safe::new(strategy).as_str()))
                    .unwrap_or_default(),
                phase.visits
            ),
        );
        for column in &phase.columns {
            if column.nodes.len() == 1 {
                let node = &column.nodes[0];
                let label = match &column.label {
                    Some(label) => label.as_str(),
                    None => node.label.as_str(),
                };
                push_line(
                    &mut out,
                    &format!(
                        "    → {} {} · {}",
                        node_glyph(&node.state_class, style),
                        Safe::new(label).as_str(),
                        Safe::new(&node.state).as_str()
                    ),
                );
            } else {
                let label = match &column.label {
                    Some(label) => format!(" {}", Safe::new(label).as_str()),
                    None => String::new(),
                };
                push_line(&mut out, &format!("    ⑂{label}"));
                for node in &column.nodes {
                    push_line(
                        &mut out,
                        &format!(
                            "      {} {} · {}",
                            node_glyph(&node.state_class, style),
                            Safe::new(&node.label).as_str(),
                            Safe::new(&node.state).as_str()
                        ),
                    );
                }
            }
        }
    }
    out
}
