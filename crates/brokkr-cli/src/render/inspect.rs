//! The sections of the human run readout that [`super::inspect()`] paints
//! from the run's summary and its ruling (#288): each is a list of lines
//! the readout prints in the order it always has.

use brokkr_view::{Ruling, Standing, Summary};

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
            if let Some(detail) = &summary.detail {
                push_line(out, &format!("stop  {}", Safe::new(detail).as_str()));
            }
            if let Some(ruled) = policy_reason(summary) {
                push_line(out, &format!("stop  {}", Safe::new(&ruled).as_str()));
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

#[cfg(test)]
mod tests;
