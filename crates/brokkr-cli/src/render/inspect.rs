//! The sections of the human run readout that [`super::inspect()`] paints
//! from the run's summary and its ruling (#288): each is a list of lines
//! the readout prints in the order it always has.

use brokkr_view::{Ruling, Summary};

use super::{push_line, status_code, tint, Safe, Style};

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
            let status = summary.status_word();
            push_line(
                out,
                &format!(
                    "     {} · phase {} · seq {}",
                    tint(Safe::new(status).as_str(), status_code(status), style),
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
        }
        None => push_line(out, "run  — this journal does not fold"),
    }
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
