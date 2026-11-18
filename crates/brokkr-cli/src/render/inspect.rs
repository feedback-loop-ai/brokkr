//! The sections of the human run readout that [`super::inspect()`] paints
//! from the run's summary and its ruling (#288): each is a list of lines
//! the readout prints in the order it always has.

use brokkr_view::{GateFinding, GateRecord, Ruling, Standing, Summary};

use super::ending::{cited, code, painted_as};
use super::{push_line, tint, Safe, Style};

/// The header: the run, its status, phase and seq, and the park reason
/// where the run parked. A journal that does not fold says so instead.
pub(super) fn summary_lines(out: &mut String, summary: Option<&Summary>, style: &Style) {
    match summary {
        Some(summary) => {
            push_line(
                out,
                &format!("run  {}", Safe::new(&summary.run_id).as_str()),
            );
            let phase = match &summary.phase {
                Some(phase) => phase.clone(),
                None => "-".to_string(),
            };
            push_line(
                out,
                &format!(
                    "     {} · phase {} · seq {}",
                    standing_words(summary, style),
                    Safe::new(&phase).as_str(),
                    summary.seq
                ),
            );
            if let Some(reason) = &summary.park_reason {
                push_line(out, &format!("park  {}", Safe::new(reason).as_str()));
            }
            if let Some(cause) = &summary.park_cause {
                push_line(out, &format!("cause {}", Safe::new(&cause.line()).as_str()));
            }
            if let Some(detail) = &summary.detail {
                push_line(out, &format!("stop  {}", Safe::new(detail).as_str()));
            }
            if let Some(ruled) = policy_reason(summary) {
                push_line(out, &format!("stop  {}", Safe::new(&ruled).as_str()));
            }
            if let Some(tail) = &summary.late_tail {
                push_line(
                    out,
                    &format!(
                        "late  {}",
                        Safe::new(&brokkr_view::late_line(tail.count(), &tail.operator)).as_str()
                    ),
                );
                for event in &tail.events {
                    push_line(
                        out,
                        &format!(
                            "      seq {} · {}",
                            event.seq,
                            Safe::new(event.event_type.as_str()).as_str()
                        ),
                    );
                }
            }
        }
        None => push_line(out, "run  — this journal does not fold"),
    }
}

/// How the run stands, shaded by what it waits on (decision 0064 ruling
/// 6): both words once it has ended, `status stopped · ending superseded
/// by <run>`, and the one word a live run prints otherwise.
fn standing_words(summary: &Summary, style: &Style) -> String {
    let (shade, _) = painted_as(summary.ending, summary.wants, summary.standing);
    let painted = |word: &str| tint(Safe::new(word).as_str(), code(shade), style);
    let Some(ending) = summary.ending else {
        return painted(summary.status_word());
    };
    let citation = match &summary.ending_cited {
        Some(citation) => format!(" {}", Safe::new(&cited(citation)).as_str()),
        None => String::new(),
    };
    format!(
        "status {} · ending {}{citation}",
        Safe::new(&summary.status).as_str(),
        painted(ending.word())
    )
}

/// The reason a policy ending reads (ruling 6): the rule that stopped the
/// run, the phase it ruled from, and the problem the ruling recorded.
fn policy_reason(summary: &Summary) -> Option<String> {
    if summary.standing != Standing::Stopped {
        return None;
    }
    let ruling = summary.last_decision.as_ref()?;
    let rule = ruling.rule_id.present().map_or("?", String::as_str);
    let problem = match ruling.problem.present() {
        Some(problem) => format!(" — {problem}"),
        None => String::new(),
    };
    Some(format!("ruled by {rule} from {}{problem}", ruling.from))
}

/// The ruling line, its problem, and the operator's supersession mark.
pub(super) fn ruling_lines(out: &mut String, ruling: &Ruling) {
    let result = match &ruling.result {
        Some(result) => format!(" · {}", Safe::new(result).as_str()),
        None => String::new(),
    };
    push_line(
        out,
        &format!(
            "ruling  {}  {} → {}{result}",
            Safe::new(&ruling.rule_id).as_str(),
            Safe::new(&ruling.from).as_str(),
            Safe::new(&ruling.next).as_str()
        ),
    );
    if let Some(problem) = &ruling.problem {
        push_line(out, &format!("        {}", Safe::new(problem).as_str()));
    }
    // The operator's word that the residual this ruling recorded is
    // closed elsewhere (decision 0047 ruling 3). The ruling above is
    // printed exactly as it was ruled: a mark is not a correction.
    if let Some(mark) = &ruling.superseded {
        push_line(
            out,
            &format!(
                "        superseded at seq {} by {} · {} · {}",
                mark.seq,
                Safe::new(&brokkr_view::cited(&mark.by)).as_str(),
                Safe::new(&mark.operator).as_str(),
                Safe::new(&mark.reason).as_str()
            ),
        );
    }
}

/// The block after the readout's frame that says what each gate verdict
/// recorded beside its word (decision 0109 slice 0109a1): its covered head
/// and each finding, one line each, the finding's key as its id and the
/// first twelve hex of its site. Empty for a run whose results recorded
/// neither, so an older run reads exactly as it did.
pub(crate) fn gates(records: &[GateRecord]) -> String {
    let mut out = String::new();
    for record in records {
        let phase = Safe::new(&record.phase);
        if let Some(covered) = &record.covered {
            let covered = Safe::new(covered);
            push_line(
                &mut out,
                &format!("  {} · covered {}", phase.as_str(), covered.as_str()),
            );
        }
        for finding in &record.findings {
            push_line(&mut out, &finding_line(phase.as_str(), finding));
        }
    }
    match out.is_empty() {
        true => out,
        false => format!("\nverdicts\n{out}"),
    }
}

fn finding_line(phase: &str, finding: &GateFinding) -> String {
    let at = match finding.line {
        Some(line) => format!("{}:{line}", finding.file),
        None => finding.file.clone(),
    };
    let security = if finding.security { " · security" } else { "" };
    format!(
        "  {phase} · finding {} · site {} · {}{security} · {} · {}",
        Safe::new(&finding.id).as_str(),
        finding.site.get(..12).unwrap_or(&finding.site),
        finding.severity,
        Safe::new(&at).as_str(),
        Safe::new(&finding.summary).as_str()
    )
}

#[cfg(test)]
mod tests;
