//! Terminal rendering for `brokkr-view`'s models: pure string functions,
//! golden-tested. The derivation is not here — every value below is a
//! field of a model the console paints from too, which is the whole
//! point of decision 0013.
//!
//! **Terminal safety.** The journal is seat- and driver-authored: feature
//! text, seat labels, result tokens, park reasons, error strings.
//! Printed straight to a tty, `\x1b]0;…\x07` retitles the operator's
//! terminal, `\x1b[2J` clears the frame that was supposed to be
//! evidence, and `\r` plus spaces overwrites the line above — so a
//! hostile result token can forge a ruling line, continuously under
//! `watch`. Every journal string reaching stdout goes through [`Safe`],
//! whose only constructor strips control characters **and** the bidi and
//! zero-width formatting characters that `char::is_control()` does not
//! cover — U+202E RIGHT-TO-LEFT OVERRIDE and its neighbours visually
//! reorder the rest of a rendered line, which forges the same ruling
//! line by another route. Sanitization happens **before** any width
//! arithmetic so an escape sequence cannot smuggle invisible width. This
//! is the terminal twin of the console's fixed class allowlists, and it
//! serves all three surfaces (decision 0014's TUI included).
//!
//! **Width.** `COLUMNS` with a sane default, all column arithmetic
//! saturating, all truncation on `char` boundaries. Without a
//! Unicode-width dependency — forbidden by the ruling — CJK and emoji
//! columns will misalign. That is stated, not pretended away.

use std::io::IsTerminal;

use brokkr_view::{JournalRow, Participant, Phase, RunView, RunsView};

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";

const DEFAULT_WIDTH: usize = 80;
const MIN_WIDTH: usize = 20;
const MAX_WIDTH: usize = 1000;
/// Below this many remaining columns the feature is omitted rather than
/// mangled into two characters and an ellipsis.
const FEATURE_FLOOR: usize = 8;

/// A string safe to interpolate into a terminal frame. The field is
/// private and the only constructor sanitizes: this is enforced by
/// construction, not by discipline.
pub struct Safe(String);

/// The formatting characters that reorder or hide a line without being
/// control characters: the zero-width and bidi marks (U+200B–U+200F),
/// the embedding and override controls (U+202A–U+202E), the invisible
/// operators (U+2060–U+2064), the directional isolates (U+2066–U+2069)
/// and the byte-order mark (U+FEFF). Enumerated explicitly — a
/// Unicode-properties dependency is not on the table.
fn reorders(character: char) -> bool {
    matches!(character,
        '\u{200B}'..='\u{200F}'
        | '\u{202A}'..='\u{202E}'
        | '\u{2060}'..='\u{2064}'
        | '\u{2066}'..='\u{2069}'
        | '\u{FEFF}')
}

impl Safe {
    pub fn new(text: &str) -> Safe {
        Safe(
            text.chars()
                .filter(|c| !c.is_control() && !reorders(*c))
                .collect(),
        )
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Display width in `char`s — computed on the sanitized text, so a
    /// stripped escape sequence cannot claim invisible columns.
    pub fn width(&self) -> usize {
        self.0.chars().count()
    }

    pub fn padded(&self, width: usize) -> String {
        let mut out = self.0.clone();
        for _ in self.width()..width {
            out.push(' ');
        }
        out
    }
}

/// How the terminal wants to be written to. Both facts come from the
/// environment; the rules that read them are pure.
pub struct Style {
    pub color: bool,
    pub width: usize,
}

impl Style {
    pub fn detect() -> Style {
        Style {
            color: color_enabled(
                std::io::stdout().is_terminal(),
                std::env::var_os("NO_COLOR").is_some(),
                std::env::var_os("TERM") == Some(std::ffi::OsString::from("dumb")),
            ),
            width: width_from(std::env::var("COLUMNS").ok().as_deref()),
        }
    }

    /// The shape every golden runs in: colour is a post-processing wrap,
    /// so the goldens prove content and exactly one test proves colour.
    #[cfg(test)]
    pub fn plain(width: usize) -> Style {
        Style {
            color: false,
            width,
        }
    }
}

fn color_enabled(is_terminal: bool, no_color: bool, term_is_dumb: bool) -> bool {
    is_terminal && !no_color && !term_is_dumb
}

fn width_from(value: Option<&str>) -> usize {
    match value.and_then(|text| text.parse::<usize>().ok()) {
        Some(width) => width.clamp(MIN_WIDTH, MAX_WIDTH),
        None => DEFAULT_WIDTH,
    }
}

/// How a status reads, independent of how a surface paints it. One
/// classification in the crate; three renderings of it (decision 0013's
/// invariant, held across the TUI's arrival).
pub(crate) enum Tone {
    Good,
    Bad,
    Live,
    Quiet,
}

/// The fixed status table, the same four the console colours. A status
/// outside the known four falls to the quiet arm; it is never guessed
/// into one of the four (decision 0001).
pub(crate) fn tone(status: &str) -> Tone {
    match status {
        "completed" | "succeeded" => Tone::Good,
        "stopped" | "failed" => Tone::Bad,
        "running" | "working" => Tone::Live,
        _ => Tone::Quiet,
    }
}

fn status_code(status: &str) -> &'static str {
    match tone(status) {
        Tone::Good => GREEN,
        Tone::Bad => RED,
        Tone::Live => BOLD,
        Tone::Quiet => DIM,
    }
}

fn tint(text: &str, code: &'static str, style: &Style) -> String {
    if style.color {
        format!("{code}{text}{RESET}")
    } else {
        text.to_string()
    }
}

fn push_line(out: &mut String, line: &str) {
    out.push_str(line.trim_end());
    out.push('\n');
}

// ----------------------------------------------------------- brokkr runs

/// One clamped line per run, newest first: id, status, phase, seq, age,
/// feature. Columns are sized to the widest value in the batch; the
/// feature takes what is left.
pub fn runs(view: &RunsView, now: &str, style: &Style) -> String {
    let mut rows: Vec<[Safe; 6]> = Vec::new();
    let mut codes: Vec<&'static str> = Vec::new();
    let mut details: Vec<Option<Safe>> = Vec::new();
    for run in &view.runs {
        let status = match &run.status {
            Some(status) => status.clone(),
            None => "?".to_string(),
        };
        let phase = match &run.phase {
            Some(phase) => phase.clone(),
            None => "-".to_string(),
        };
        let seq = match run.seq {
            Some(seq) => format!("seq {seq}"),
            None => "seq -".to_string(),
        };
        let age = match brokkr_view::age(&run.created_at, now) {
            Some(age) => age,
            None => brokkr_view::ABSENT.to_string(),
        };
        codes.push(status_code(&status));
        details.push(run.detail.as_deref().map(Safe::new));
        rows.push([
            Safe::new(&run.run_id),
            Safe::new(&status),
            Safe::new(&phase),
            Safe::new(&seq),
            Safe::new(&age),
            Safe::new(&run.feature),
        ]);
    }
    let mut widths = [0usize; 5];
    for row in &rows {
        for (index, width) in widths.iter_mut().enumerate() {
            *width = (*width).max(row[index].width());
        }
    }
    let used = widths.iter().sum::<usize>() + widths.len();
    let remaining = style.width.saturating_sub(used);
    let mut out = String::new();
    for ((row, code), detail) in rows.iter().zip(codes).zip(&details) {
        let mut line = String::new();
        for (index, width) in widths.iter().enumerate() {
            let piece = row[index].padded(*width);
            line.push_str(&if index == 1 {
                tint(&piece, code, style)
            } else {
                piece
            });
            line.push(' ');
        }
        if remaining >= FEATURE_FLOOR {
            line.push_str(&brokkr_view::clamp(row[5].as_str(), remaining));
        }
        push_line(&mut out, &line);
        // A quarantined row says why underneath itself, the way a park
        // reason does on the run readout: `?` alone tells an operator
        // nothing they can act on.
        if let Some(detail) = detail {
            push_line(&mut out, &format!("  fold  {}", detail.as_str()));
        }
    }
    out
}

// -------------------------------------------------------- brokkr inspect

/// The console's exclusive scoping, as the verbs a terminal has.
pub enum Scope {
    Phase(String),
    Seat(String),
}

/// A resolved scope: which phases survive, and — for `--seat` — which
/// participant keys. Membership itself is a model field on both sides,
/// so this never re-implements the predicate.
pub struct Lens {
    phases: Vec<String>,
    keys: Vec<String>,
    by_key: bool,
}

/// Resolve a scope against a run, or say what the run actually offers.
/// A value matching nothing is an error rather than an empty table: an
/// empty seats table reads as "this phase did nothing", which is a claim
/// about a run this tool cannot make.
pub fn lens_for(view: &RunView, scope: Option<&Scope>) -> Result<Option<Lens>, String> {
    let Some(scope) = scope else {
        return Ok(None);
    };
    match scope {
        Scope::Phase(name) => {
            let known = view.phases.iter().any(|phase| phase.name == *name);
            if !known {
                let valid: Vec<String> = view
                    .phases
                    .iter()
                    .map(|phase| Safe::new(&phase.name).as_str().to_string())
                    .collect();
                // Error paths honour the module invariant too: these
                // strings reach the operator's tty through anyhow, so
                // the scope name and every phase name are sanitized.
                return Err(format!(
                    "no phase '{}' in this run; visited phases: {}",
                    Safe::new(name).as_str(),
                    valid.join(", ")
                ));
            }
            Ok(Some(Lens {
                phases: vec![name.clone()],
                keys: Vec::new(),
                by_key: false,
            }))
        }
        Scope::Seat(label) => {
            // ALL occurrences: a re-entered phase really did run that
            // seat twice, and hiding one is a false statement about the
            // run. An exact participant key matches too — that is what
            // the console's clicks select.
            let matched: Vec<&Participant> = view
                .participants
                .iter()
                .filter(|part| part.label == *label || part.key == *label)
                .collect();
            if matched.is_empty() {
                let mut valid: Vec<String> = view
                    .participants
                    .iter()
                    .map(|part| Safe::new(&part.label).as_str().to_string())
                    .collect();
                valid.dedup();
                return Err(format!(
                    "no seat '{}' in this run; participants: {}",
                    Safe::new(label).as_str(),
                    valid.join(", ")
                ));
            }
            let mut phases = Vec::new();
            for part in &matched {
                if let Some(phase) = &part.phase {
                    if !phases.contains(phase) {
                        phases.push(phase.clone());
                    }
                }
            }
            Ok(Some(Lens {
                phases,
                keys: matched.iter().map(|part| part.key.clone()).collect(),
                by_key: true,
            }))
        }
    }
}

/// The scope predicate for a phase, in one place: `graph_block` and the
/// TUI ask the same question and must never answer it twice.
pub(crate) fn keeps_phase(lens: Option<&Lens>, phase: &Phase) -> bool {
    match lens {
        None => true,
        Some(lens) => lens.phases.contains(&phase.name),
    }
}

pub(crate) fn keeps_participant(lens: Option<&Lens>, part: &Participant) -> bool {
    match lens {
        None => true,
        Some(lens) if lens.by_key => lens.keys.contains(&part.key),
        Some(lens) => match &part.phase {
            Some(phase) => lens.phases.contains(phase),
            None => false,
        },
    }
}

pub(crate) fn keeps_row(lens: Option<&Lens>, row: &JournalRow) -> bool {
    match lens {
        None => true,
        Some(lens) => row.phases.iter().any(|name| lens.phases.contains(name)),
    }
}

fn seats_block(view: &RunView, lens: Option<&Lens>, style: &Style) -> String {
    let seats: Vec<&Participant> = view
        .participants
        .iter()
        .filter(|part| keeps_participant(lens, part))
        .collect();
    if seats.is_empty() {
        return String::new();
    }
    let header = ["participant", "status", "attempts", "turns", "cost"];
    let mut rows: Vec<[Safe; 6]> = Vec::new();
    for part in &seats {
        rows.push([
            Safe::new(&part.label),
            Safe::new(&part.status),
            Safe::new(&part.attempts.to_string()),
            Safe::new(&part.turns_cell.text),
            Safe::new(&part.cost_cell.text),
            Safe::new(&part.activity.text),
        ]);
    }
    let mut widths = [0usize; 5];
    for (index, name) in header.iter().enumerate() {
        widths[index] = name.chars().count();
    }
    for row in &rows {
        for (index, width) in widths.iter_mut().enumerate() {
            *width = (*width).max(row[index].width());
        }
    }
    let used = widths.iter().sum::<usize>() + widths.len() + 2;
    let remaining = style.width.saturating_sub(used);
    let mut out = String::from("seats\n");
    let mut head = String::from("  ");
    for (index, name) in header.iter().enumerate() {
        head.push_str(&Safe::new(name).padded(widths[index]));
        head.push(' ');
    }
    head.push_str("activity");
    push_line(&mut out, &head);
    for (row, part) in rows.iter().zip(&seats) {
        let mut line = String::from("  ");
        for (index, width) in widths.iter().enumerate() {
            let piece = row[index].padded(*width);
            line.push_str(&if index == 1 {
                tint(&piece, status_code(&part.status), style)
            } else {
                piece
            });
            line.push(' ');
        }
        line.push_str(&brokkr_view::clamp(row[5].as_str(), remaining));
        push_line(&mut out, &line);
        // Which agent, model and provider actually served this seat
        // (decision 0016). The sentence comes from the single
        // derivation; this surface only indents it.
        if let Some(provenance) = &part.provenance {
            push_line(
                &mut out,
                &format!("    {}", Safe::new(&provenance.line).as_str()),
            );
        }
    }
    out
}

fn trail_block(view: &RunView, lens: Option<&Lens>, style: &Style) -> String {
    let rows: Vec<&JournalRow> = view
        .journal
        .iter()
        .filter(|row| row.in_trail && keeps_row(lens, row))
        .collect();
    if rows.is_empty() {
        return String::new();
    }
    let mut seq_width = 0usize;
    let mut type_width = 0usize;
    for row in &rows {
        seq_width = seq_width.max(row.seq.to_string().len());
        type_width = type_width.max(row.event_type.chars().count());
    }
    let used = seq_width + type_width + 4;
    let remaining = style.width.saturating_sub(used);
    let mut out = String::from("trail\n");
    for row in &rows {
        let line = format!(
            "  {:>seq_width$} {} {}",
            row.seq,
            Safe::new(&row.event_type).padded(type_width),
            brokkr_view::clamp(Safe::new(&row.what.text).as_str(), remaining),
        );
        push_line(&mut out, &line);
    }
    out
}

/// The phase graph as a terminal tree: `⑂` precedes parallel members and
/// `→` precedes a sequential step, both nested under their phase. The
/// markers are content, not colour, and are therefore unconditional —
/// the models already emit `Σ`, `↓`, `…` and `—` in pre-baked text, so
/// an ASCII mode would need a second derivation of every one of them.
fn graph_block(view: &RunView, lens: Option<&Lens>) -> String {
    let phases: Vec<&Phase> = view
        .phases
        .iter()
        .filter(|phase| keeps_phase(lens, phase))
        .collect();
    if phases.is_empty() {
        return String::new();
    }
    let mut out = String::from("graph\n");
    for phase in phases {
        let current = if phase.current { "  ←current" } else { "" };
        push_line(
            &mut out,
            &format!(
                "  {} ×{}{current}",
                Safe::new(&phase.name).as_str(),
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
                        "    → {} · {}",
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
                            "      {} · {}",
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

/// The human readout: header, ruling, park reason, live seat activity,
/// the seats table, the decision trail, and the phase tree. A `watch`
/// frame is this without the trail.
pub fn inspect(view: &RunView, lens: Option<&Lens>, trail: bool, style: &Style) -> String {
    let mut out = String::new();
    match &view.summary {
        Some(summary) => {
            push_line(
                &mut out,
                &format!("run  {}", Safe::new(&summary.run_id).as_str()),
            );
            let phase = match &summary.phase {
                Some(phase) => phase.clone(),
                None => "-".to_string(),
            };
            push_line(
                &mut out,
                &format!(
                    "     {} · phase {} · seq {}",
                    tint(
                        Safe::new(&summary.status).as_str(),
                        status_code(&summary.status),
                        style
                    ),
                    Safe::new(&phase).as_str(),
                    summary.seq
                ),
            );
            if let Some(reason) = &summary.park_reason {
                push_line(&mut out, &format!("park  {}", Safe::new(reason).as_str()));
            }
        }
        None => push_line(&mut out, "run  — this journal does not fold"),
    }
    if let Some(ruling) = &view.ruling {
        let result = match &ruling.result {
            Some(result) => format!(" · {}", Safe::new(result).as_str()),
            None => String::new(),
        };
        push_line(
            &mut out,
            &format!(
                "ruling  {}  {} → {}{result}",
                Safe::new(&ruling.rule_id).as_str(),
                Safe::new(&ruling.from).as_str(),
                Safe::new(&ruling.next).as_str()
            ),
        );
        if let Some(problem) = &ruling.problem {
            push_line(
                &mut out,
                &format!("        {}", Safe::new(problem).as_str()),
            );
        }
    }
    // Run-level notices before the seats table: a fallback selection and
    // an optional capability gap are facts an operator must see rather
    // than find (decision 0016).
    for notice in &view.notices {
        push_line(
            &mut out,
            &format!(
                "note  {} — {}",
                Safe::new(&notice.kind).as_str(),
                Safe::new(&notice.text).as_str()
            ),
        );
    }
    for live in &view.live {
        push_line(
            &mut out,
            &format!("live  {}", Safe::new(&live.text).as_str()),
        );
    }
    for block in [
        seats_block(view, lens, style),
        if trail {
            trail_block(view, lens, style)
        } else {
            String::new()
        },
        graph_block(view, lens),
    ] {
        if !block.is_empty() {
            out.push('\n');
            out.push_str(&block);
        }
    }
    out
}

#[cfg(test)]
mod tests;
