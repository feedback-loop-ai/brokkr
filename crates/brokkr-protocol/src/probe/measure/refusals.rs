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

/// The words a model refusal is put in, folded to lowercase.
const MODEL_REFUSALS: [&str; 5] = [
    "not found",
    "does not exist",
    "unknown model",
    "invalid model",
    "not supported",
];

/// The words that mark a line as another class's refusal, an auth
/// failure, a rate limit or an outage, folded to lowercase.
const OTHER_CLASSES: [&str; 9] = [
    "unauthorized",
    "api key",
    "x-api-key",
    "login",
    "rate limit",
    "quota",
    "overloaded",
    "unavailable",
    "timeout",
];

/// The HTTP statuses of another class's refusal, read as whole words.
const OTHER_STATUSES: [&str; 8] = ["401", "403", "429", "500", "502", "503", "504", "529"];

/// Whether `line` is itself the unknown model's refusal: it names the
/// model in a model refusal's words, and carries no other class's mark.
fn refuses_the_model(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let mut words = lower.split(|c: char| !c.is_ascii_alphanumeric());
    line.contains(NO_SUCH_MODEL)
        && MODEL_REFUSALS.iter().any(|refusal| lower.contains(refusal))
        && !OTHER_CLASSES.iter().any(|mark| lower.contains(mark))
        && !words.any(|word| OTHER_STATUSES.contains(&word))
}

/// How the unknown model was refused: measured only when a line the
/// launch printed is itself that refusal, by [`refuses_the_model`], since
/// a line that merely echoes the model, or names it beside an outage or
/// the account, does not show the model was what it refused (decision
/// 0071 ruling 3).
pub(super) fn config_refusal(trial: &Trial) -> Fact<Refusal> {
    let read = refusal(trial);
    match (trial, read.value()) {
        (Trial::Observed(observation), Some(_))
            if !observation
                .stderr
                .text
                .lines()
                .chain(observation.stdout.text.lines())
                .any(refuses_the_model) =>
        {
            Fact::unmeasured(format!(
                "no line of the refusal names the model {NO_SUCH_MODEL} in a model refusal's \
                 words and no other class's, so it is not shown to be the configuration's: {}",
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
