//! The deliberate mistakes' refusals, and the accepted efforts one of
//! them lists. Pure, like `measure`.

use super::{excerpt, exit_and_excerpt};
use crate::probe::facts::{Fact, Refusal};
use crate::probe::observe::{Observation, Trial};
use crate::probe::plan::{NO_SUCH_EFFORT, NO_SUCH_MODEL};

/// What refusals list the accepted levels after: clap's, commander's and
/// serde's wording.
const LEVEL_MARKERS: [&str; 3] = ["possible values:", "Allowed choices are", "expected one of"];

/// The launch that refused a deliberate mistake, or why there is no
/// refusal to read: `accepted` when it exited 0. A launch that ended with
/// no exit code, by a signal or the probe's deadline, refused nothing.
fn refused<T: serde::Serialize>(trial: &Trial, accepted: String) -> Result<&Observation, Fact<T>> {
    let observation = match trial {
        Trial::Untried(why) => return Err(Fact::unmeasured(why.clone())),
        Trial::Observed(observation) => observation,
    };
    match observation.exit {
        Some(0) => Err(Fact::unmeasured(accepted)),
        None => Err(Fact::unmeasured(format!(
            "no refusal was read: {}",
            exit_and_excerpt(observation)
        ))),
        Some(_) => Ok(observation),
    }
}

/// How one deliberate mistake was refused.
pub(super) fn refusal(trial: &Trial) -> Fact<Refusal> {
    let accepted = "the CLI exited 0, so there was no refusal to read".to_string();
    match refused(trial, accepted) {
        Ok(observation) => Fact::measured(
            Refusal {
                exit: observation.exit,
                excerpt: excerpt(observation),
            },
            exit_and_excerpt(observation),
        ),
        Err(fact) => fact,
    }
}

/// How the unknown model was refused: measured only when what the launch
/// printed names the model, since that alone shows the model was what it
/// refused, and not an outage or the account (decision 0071 ruling 3).
pub(super) fn config_refusal(trial: &Trial) -> Fact<Refusal> {
    let read = refusal(trial);
    match (trial, read.value()) {
        (Trial::Observed(observation), Some(_))
            if !observation.stderr.text.contains(NO_SUCH_MODEL)
                && !observation.stdout.text.contains(NO_SUCH_MODEL) =>
        {
            Fact::unmeasured(format!(
                "the refusal does not name the model {NO_SUCH_MODEL}, so it is not shown to be \
                 the configuration's: {}",
                exit_and_excerpt(observation)
            ))
        }
        _ => read,
    }
}

/// The accepted levels a refusal lists after one of its markers.
fn accepted_levels(text: &str) -> Option<Vec<String>> {
    let rest = text.lines().find_map(|line| {
        LEVEL_MARKERS
            .iter()
            .find_map(|marker| line.split_once(marker).map(|(_, rest)| rest))
    })?;
    let levels: Vec<String> = rest
        .replace(" or ", ",")
        .split(',')
        .filter_map(|item| item.split_whitespace().next())
        .map(|word| {
            word.trim_matches(|c: char| !c.is_ascii_alphanumeric())
                .to_string()
        })
        .filter(|level| !level.is_empty())
        .collect();
    Some(levels).filter(|levels| !levels.is_empty())
}

pub(super) fn efforts(trial: &Trial) -> Fact<Vec<String>> {
    let accepted = format!(
        "the CLI accepted the unknown effort '{NO_SUCH_EFFORT}' and exited 0, so no refusal \
         lists its levels"
    );
    let observation = match refused(trial, accepted) {
        Ok(observation) => observation,
        Err(fact) => return fact,
    };
    let text = format!("{}\n{}", observation.stderr.text, observation.stdout.text);
    match accepted_levels(&text) {
        Some(levels) => Fact::measured(levels, exit_and_excerpt(observation)),
        None => Fact::unmeasured(format!(
            "the refusal names no accepted levels: {}",
            exit_and_excerpt(observation)
        )),
    }
}
