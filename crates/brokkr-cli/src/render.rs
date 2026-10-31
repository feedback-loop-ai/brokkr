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

use brokkr_view::transcript::TranscriptRead;
use brokkr_view::{
    keeps_participant, keeps_phase, keeps_row, JournalRow, Lens, ModelAtBoundary, Participant,
    Phase, RunRow, RunView, RunsView, ScopeError,
};

mod inspect;

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
pub(crate) struct Safe(String);

/// The formatting characters that reorder or hide a line without being
/// control characters: the Arabic letter mark (U+061C), the zero-width
/// and bidi marks (U+200B–U+200F), the embedding and override controls
/// (U+202A–U+202E), the invisible operators (U+2060–U+2064), the
/// directional isolates (U+2066–U+2069) and the byte-order mark
/// (U+FEFF). Enumerated explicitly — a Unicode-properties dependency is
/// not on the table.
fn reorders(character: char) -> bool {
    matches!(character,
        '\u{061C}'
        | '\u{200B}'..='\u{200F}'
        | '\u{202A}'..='\u{202E}'
        | '\u{2060}'..='\u{2064}'
        | '\u{2066}'..='\u{2069}'
        | '\u{FEFF}')
}

impl Safe {
    pub(crate) fn new(text: &str) -> Safe {
        Safe(
            text.chars()
                .filter(|c| !c.is_control() && !reorders(*c))
                .collect(),
        )
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Display width in `char`s — computed on the sanitized text, so a
    /// stripped escape sequence cannot claim invisible columns.
    pub(crate) fn width(&self) -> usize {
        self.0.chars().count()
    }

    pub(crate) fn padded(&self, width: usize) -> String {
        let mut out = self.0.clone();
        for _ in self.width()..width {
            out.push(' ');
        }
        out
    }
}

/// How the terminal wants to be written to. Both facts come from the
/// environment; the rules that read them are pure.
pub(crate) struct Style {
    pub color: bool,
    pub width: usize,
}

impl Style {
    pub(crate) fn detect() -> Style {
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
    pub(crate) fn plain(width: usize) -> Style {
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
        "completed" | "succeeded" | "carried" => Tone::Good,
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

// ------------------------------------------------- the model, its boundary

/// The served-model pair as a terminal surface places it: the model
/// cell and the boundary cell, sanitized, and whether the journal named
/// a model at all — the one fact a trail row branches on.
pub(crate) struct ServedText {
    pub model: Safe,
    pub boundary: Safe,
    /// True when the model cell is present. A trail row prints the pair
    /// only beside a model it names; a table prints both marks.
    pub named: bool,
}

/// The pair helper's text face (decision 0046 ruling 3; design DD12):
/// the ONE place a renderer reads `served.model`, so it cannot take the
/// model without the boundary beside it. Every terminal surface — the
/// seats table, the trail, the TUI's table, detail pane, checkpoint and
/// journal rows — places what this returns and reads neither cell
/// itself, which `tests/boundary_readouts.rs` pins by source.
pub(crate) fn served_text(served: &ModelAtBoundary) -> ServedText {
    ServedText {
        model: Safe::new(&served.model.text),
        boundary: Safe::new(&served.boundary.text),
        named: !served.model.absent,
    }
}

// ----------------------------------------------------------- brokkr runs

/// One clamped line per run, newest first: id, status, phase, seq, age,
/// feature. Columns are sized to the widest value in the batch; the
/// feature takes what is left.
pub(crate) fn runs(view: &RunsView, now: &str, style: &Style) -> String {
    let mut widths = [0usize; 5];
    widen(&mut widths, &view.runs, now);
    lines_within(&view.runs, &widths, now, style)
}

/// A run's six cells: id, status, phase, seq, age and feature.
fn run_cells(run: &RunRow, now: &str) -> [Safe; 6] {
    let status = run.status_word().to_string();
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
    [
        Safe::new(&run.run_id),
        Safe::new(&status),
        Safe::new(&phase),
        Safe::new(&seq),
        Safe::new(&age),
        Safe::new(&run.feature),
    ]
}

/// `widths` widened to the widest of each sized column in `runs`: a
/// listing written a page at a time sizes its columns over every page
/// first, so every page lines up as one batch would.
pub(crate) fn widen(widths: &mut [usize; 5], runs: &[RunRow], now: &str) {
    for run in runs {
        let cells = run_cells(run, now);
        for (index, width) in widths.iter_mut().enumerate() {
            *width = (*width).max(cells[index].width());
        }
    }
}

/// The lines `runs` render to, each sized column `widths` wide; the
/// feature takes what is left.
pub(crate) fn lines_within(
    runs: &[RunRow],
    widths: &[usize; 5],
    now: &str,
    style: &Style,
) -> String {
    let used = widths.iter().sum::<usize>() + widths.len();
    let remaining = style.width.saturating_sub(used);
    let mut out = String::new();
    for run in runs {
        let row = run_cells(run, now);
        let code = status_code(run.status_word());
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
        for under in under_row(run) {
            push_line(&mut out, &under);
        }
    }
    out
}

/// The lines a row prints underneath itself. A quarantined row says why,
/// the way a park reason does on the run readout: `?` alone tells an
/// operator nothing they can act on. An acknowledged one says who
/// answered it, when and why, under the refusal it keeps (#511).
fn under_row(run: &RunRow) -> Vec<String> {
    let refusal = run
        .detail
        .as_deref()
        .map(|detail| format!("  fold  {}", Safe::new(detail).as_str()));
    let acknowledged = run
        .acknowledgement
        .as_ref()
        .map(|acknowledgement| format!("  {}", Safe::new(&acknowledgement.line()).as_str()));
    let stopping = run
        .verdict
        .detail
        .as_deref()
        .map(|detail| format!("  {}", Safe::new(detail).as_str()));
    refusal
        .into_iter()
        .chain(acknowledged)
        .chain(stopping)
        .collect()
}

/// The heading of the `index`th realm's section of a many-hearth world's
/// fleet: its runs are listed under it rather than interleaved with
/// another realm's (decision 0026 ruling 3), a blank line parting it from
/// the section before. It names the realm, how many runs it holds and the
/// journal they were read from. A one-hearth world never reaches here —
/// `brokkr runs` renders it exactly as it always has.
pub(crate) fn realm_head(
    index: usize,
    (realm, journal): (&str, &str),
    count: usize,
    detail: Option<&str>,
    style: &Style,
) -> String {
    let mut out = String::new();
    if index > 0 {
        out.push('\n');
    }
    let count = match count {
        1 => "1 run".to_string(),
        count => format!("{count} runs"),
    };
    let head = format!(
        "{} · {count} · {}",
        Safe::new(realm).as_str(),
        Safe::new(journal).as_str()
    );
    push_line(&mut out, &tint(&head, BOLD, style));
    // A hearth whose journal would not open says so where its runs
    // would have been; the rest of the world is still listed.
    if let Some(detail) = detail {
        push_line(
            &mut out,
            &format!("  journal  {}", Safe::new(detail).as_str()),
        );
    }
    out
}

// -------------------------------------------------------- brokkr inspect

/// A scope that matched nothing, as the operator's tty receives it. The
/// view names the scope and what the run offers in the journal's own
/// words; error paths honour the module invariant too, since this text
/// reaches the terminal through anyhow, so every name passes through
/// [`Safe`] before the view folds the participants.
pub(crate) fn scope_refusal(error: &ScopeError) -> String {
    error
        .named_with(|name| Safe::new(name).as_str().to_string())
        .to_string()
}

/// The seats block alone, as `brokkr seats` prints it (decision 0046
/// ruling 3; design DD11): the same block `inspect` renders, from the
/// same view, unscoped.
pub(crate) fn seats(view: &RunView, style: &Style) -> String {
    seats_block(view, None, style)
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
    // `model` is the provider's claim; `effort` is the level it was
    // configured with, as the harness echoed it back — configuration,
    // never a measurement of what the model did (decision 0035 rulings
    // 2 and 3). The pin the plan asked for is the seat's own detail
    // view, one fact per column here. `boundary` stands beside `model`
    // wherever a model is named (decision 0046 ruling 3): the plain word
    // the seat's hands stood behind, never composed here.
    let header = [
        "participant",
        "status",
        "attempts",
        "turns",
        "cost",
        "tokens",
        "model",
        "boundary",
        "effort",
    ];
    let mut rows: Vec<[Safe; 10]> = Vec::new();
    for part in &seats {
        let pair = served_text(&part.served);
        rows.push([
            Safe::new(&part.label),
            Safe::new(&part.status),
            Safe::new(&part.attempts.to_string()),
            Safe::new(&part.turns_cell.text),
            Safe::new(&part.cost_cell.text),
            Safe::new(&part.usage_cell.text),
            pair.model,
            pair.boundary,
            Safe::new(&part.effort.text),
            Safe::new(&part.activity.text),
        ]);
    }
    let mut widths = [0usize; 9];
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
    // Ruling 2's sentence, at the human readout that labels the field.
    // A served model is testimony from the party being audited, so this
    // surface names it a claim rather than printing it as settled; the
    // two effort cells beside it are configuration, and the only metered
    // fact in the row is the token count.
    push_line(
        &mut out,
        "  model is the provider's claim, not proof · effort is configuration",
    );
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
        line.push_str(&brokkr_view::clamp(row[9].as_str(), remaining));
        push_line(&mut out, &line);
        push_line(
            &mut out,
            &format!(
                "    transcript  {}",
                Safe::new(&part.transcript_cell.text).as_str()
            ),
        );
        // Which agent resolution selected this seat (decision 0016).
        // The sentence comes from the single
        // derivation; this surface only indents it.
        if let Some(provenance) = &part.provenance {
            push_line(
                &mut out,
                &format!("    {}", Safe::new(&provenance.line).as_str()),
            );
            // The pin the plan asked for, beside the applied level in
            // the row above (decision 0035 ruling 6). Both are
            // configuration; neither is filled from the other, so an
            // old journal shows two absences rather than one borrowed
            // answer.
            push_line(
                &mut out,
                &format!(
                    "    effort pinned {} · applied {} (configuration)",
                    Safe::new(&part.effort_pin.text).as_str(),
                    Safe::new(&part.effort.text).as_str()
                ),
            );
        }
    }
    out
}

/// The suffix a trail row carries when the journal names a model there:
/// ` · model <x> · boundary <y>`, and nothing at all otherwise. Shared
/// with the TUI's trail so the two spell the pair the same way.
pub(crate) fn trail_pair(pair: &ServedText) -> String {
    if pair.named {
        format!(
            " · model {} · boundary {}",
            pair.model.as_str(),
            pair.boundary.as_str()
        )
    } else {
        String::new()
    }
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
        let model = trail_pair(&served_text(&row.served));
        let what_width = remaining.saturating_sub(model.chars().count());
        let line = format!(
            "  {:>seq_width$} {} {}{}",
            row.seq,
            Safe::new(&row.event_type).padded(type_width),
            brokkr_view::clamp(Safe::new(&row.what.text).as_str(), what_width),
            model,
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
/// Nodes read as name and state only: the served model is a seat's
/// field, printed by the seats table and the seat block, never by the
/// rail — the graph is the abstraction over order and shape.
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
                "  {}{} ×{}{current}",
                Safe::new(&phase.name).as_str(),
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
pub(crate) fn inspect(view: &RunView, lens: Option<&Lens>, trail: bool, style: &Style) -> String {
    let mut out = String::new();
    inspect::summary_lines(&mut out, view.summary.as_ref(), style);
    // The run-level boundary line (decision 0046 ruling 3): the model's
    // rendered text, adjective included, printed here and composed
    // nowhere. Empty for a run that boxes nothing, so such a run reads
    // exactly as it did before the boundary was named.
    if !view.boundary.text.is_empty() {
        push_line(
            &mut out,
            &format!("     boundary {}", Safe::new(&view.boundary.text).as_str()),
        );
    }
    if let Some(ruling) = &view.ruling {
        inspect::ruling_lines(&mut out, ruling);
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

/// One local transcript as text: identity, the confirmed source and the
/// shared hint/notices, then one-based turn numbers, roles, stamps and
/// every retained block in order. Content, references, paths, stamps,
/// hints and explanations all pass through [`Safe`]; the matching JSON
/// document keeps its escaped originals.
pub(crate) fn transcript(
    run: &str,
    seat: &str,
    read: &TranscriptRead,
    turn: Option<u64>,
    _style: &Style,
) -> String {
    let mut out = String::new();
    push_line(&mut out, &format!("run   {}", Safe::new(run).as_str()));
    push_line(&mut out, &format!("seat  {}", Safe::new(seat).as_str()));
    if let Some(kind) = read.kind {
        push_line(&mut out, &format!("kind  {}", kind.as_str()));
    }
    let source = match &read.path {
        Some(path) => Safe::new(path).as_str().to_string(),
        None => "-".to_string(),
    };
    push_line(&mut out, &format!("source {source}"));
    if let Some(hint) = &read.full_session {
        push_line(&mut out, &format!("hint  {}", Safe::new(hint).as_str()));
    }
    for notice in &read.notices {
        push_line(&mut out, &format!("notice {}", Safe::new(notice).as_str()));
    }
    if read.turns.is_empty() {
        if read.truncated {
            push_line(
                &mut out,
                "no readable turns — transcript truncated (size cap)",
            );
        } else {
            push_line(&mut out, "no readable turns");
        }
        return out;
    }
    for (position, entry) in read.turns.iter().enumerate() {
        let number = match turn {
            Some(requested) => requested,
            None => position as u64 + 1,
        };
        push_line(
            &mut out,
            &format!(
                "turn {} · {} · {}",
                number,
                Safe::new(&entry.role).as_str(),
                Safe::new(&entry.ts).as_str()
            ),
        );
        for block in &entry.blocks {
            push_line(
                &mut out,
                &format!(
                    "  {}: {}",
                    block.kind.as_str(),
                    Safe::new(&block.text).as_str()
                ),
            );
        }
    }
    out
}

/// `text` as one shell word, so a printed command can be pasted as it
/// reads: bare where every character is plain, else single-quoted, each
/// `'` written `'\''` (#820).
pub(crate) fn shell_word(text: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "._-/+=:,@%".contains(c);
    match !text.is_empty() && text.chars().all(plain) {
        true => text.to_string(),
        false => format!("'{}'", text.replace('\'', r"'\''")),
    }
}

#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
mod member_tests;
