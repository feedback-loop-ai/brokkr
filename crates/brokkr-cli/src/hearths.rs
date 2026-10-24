//! The decisions [`resolve_in_hearths`] takes over a world of many
//! hearths (#288): what each hearth answered a selector, then `latest`'s
//! newest answer, an ambiguity anywhere, exactly one answer, none, or
//! several.
//!
//! [`resolve_in_hearths`]: super::resolve_in_hearths

use anyhow::Result;
use brokkr_runtime::realms::Hearth;
use brokkr_store::Store;

use super::{newest_answer, render, selector};

/// What every hearth with a journal on disk answered one selector.
pub(super) struct Answers {
    refusal: Option<anyhow::Error>,
    // An ambiguous prefix in any hearth is preserved even when another
    // hearth answers the same selector uniquely: a guess there would
    // still be a guess about which run the operator meant.
    ambiguous: Option<anyhow::Error>,
    // The hearths that answered: index, the id it resolved to, and when
    // that run was created — which is what `latest` compares.
    answered: Vec<(usize, String, String)>,
}

/// Ask every hearth whose journal is on disk, read-only and in map
/// order, what `run` resolves to there. The first refusal in that order
/// is kept, whether its journal could not be listed or its selector was
/// refused.
pub(super) fn ask(hearths: &[Hearth], run: &str) -> Answers {
    let mut answers = Answers {
        refusal: None,
        ambiguous: None,
        answered: Vec::new(),
    };
    for (index, hearth) in hearths.iter().enumerate() {
        if !hearth.journal.is_file() {
            continue;
        }
        let listed = Store::open_read_only(&hearth.journal)
            .map_err(anyhow::Error::from)
            .and_then(|store| Ok(store.list_runs()?));
        match listed {
            Ok(runs) => answers.record(index, &runs, run),
            Err(error) => {
                answers.refusal.get_or_insert(error);
            }
        }
    }
    answers
}

impl Answers {
    /// What the hearth at `index`, holding `runs`, answered `run`.
    fn record(&mut self, index: usize, runs: &[(String, String, String)], run: &str) {
        let refs: Vec<selector::RunRef<'_>> = runs
            .iter()
            .map(|(run_id, _feature, created_at)| selector::RunRef { run_id, created_at })
            .collect();
        match selector::resolve(&refs, run) {
            Ok(id) => {
                let created = refs
                    .iter()
                    .find(|candidate| candidate.run_id == id)
                    .map_or(String::new(), |candidate| candidate.created_at.to_string());
                self.answered.push((index, id, created));
            }
            Err(error) => match selector::refusal_kind(&error) {
                Some(selector::Refusal::Ambiguous) => {
                    self.ambiguous.get_or_insert(error);
                }
                _ => {
                    self.refusal.get_or_insert(error);
                }
            },
        }
    }
}

/// `latest`: the newest answer in the world, by [`newest_answer`]'s
/// rule. An ambiguity does not bear on it.
pub(super) fn latest(answers: Answers, run: String) -> Result<(usize, String)> {
    match newest_answer(answers.answered) {
        Some((index, id)) => Ok((index, id)),
        None => unanswered(answers.refusal, run),
    }
}

/// A named selector: refused when ambiguous in any hearth, then the one
/// hearth that answered, none, or several refused by name.
pub(super) fn named(hearths: &[Hearth], answers: Answers, run: String) -> Result<(usize, String)> {
    if let Some(error) = answers.ambiguous {
        return Err(error);
    }
    let mut answered = answers.answered;
    match answered.len() {
        1 => {
            let (index, id, _) = answered.swap_remove(0);
            Ok((index, id))
        }
        0 => unanswered(answers.refusal, run),
        _ => Err(several(hearths, &run, &answered)),
    }
}

/// No hearth answered: the first refusal, or — where nothing refused,
/// because no hearth had a journal — the selector passed through
/// unresolved at the first hearth, for `tui::start` to refuse.
fn unanswered(refusal: Option<anyhow::Error>, run: String) -> Result<(usize, String)> {
    match refusal {
        Some(error) => Err(error),
        None => Ok((0, run)),
    }
}

/// A selector that answered in several hearths, refused by name.
fn several(hearths: &[Hearth], run: &str, answered: &[(usize, String, String)]) -> anyhow::Error {
    // These strings reach a terminal through anyhow, so the selector
    // and every realm and id named back are sanitized.
    anyhow::anyhow!(
        "'{}' matches a run in {} realms: {}; name one journal with --db",
        render::Safe::new(run).as_str(),
        answered.len(),
        answered
            .iter()
            .map(|(index, id, _)| format!(
                "{} ({})",
                render::Safe::new(&hearths[*index].label()).as_str(),
                render::Safe::new(id).as_str()
            ))
            .collect::<Vec<String>>()
            .join(", ")
    )
}
