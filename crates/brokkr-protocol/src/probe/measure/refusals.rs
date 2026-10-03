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

/// How one deliberate mistake was refused, whatever the refusal says.
fn refusal(trial: &Trial) -> Fact<Refusal> {
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

/// The words a refusal of a flag or a config key is put in, folded to
/// lowercase.
const CONTROL_REFUSALS: [&str; 9] = [
    "unknown option",
    "unknown key",
    "unknown argument",
    "unknown flag",
    "unexpected argument",
    "unrecognized",
    "unrecognised",
    "invalid option",
    "not supported",
];

/// The refusal classes a line marks by its words alone: an auth failure,
/// a rate limit or an outage.
#[derive(Clone, Copy, PartialEq)]
enum Class {
    Auth,
    RateLimit,
    Outage,
}

/// Each class's words, folded to lowercase, and its HTTP statuses, read
/// as whole words.
const CLASSES: [(Class, &[&str], &[&str]); 3] = [
    (
        Class::Auth,
        &[
            "unauthorized",
            "api key",
            "api_key",
            "x-api-key",
            "login",
            "logged in",
        ],
        &["401", "403"],
    ),
    (Class::RateLimit, &["rate limit", "quota"], &["429"]),
    (
        Class::Outage,
        &["overloaded", "unavailable", "timeout"],
        &["500", "502", "503", "504", "529"],
    ),
];

/// Every class whose mark `line` carries.
fn marked(line: &str) -> Vec<Class> {
    let lower = line.to_ascii_lowercase();
    let words: Vec<&str> = lower.split(|c: char| !c.is_ascii_alphanumeric()).collect();
    CLASSES
        .iter()
        .filter(|(_, marks, statuses)| {
            marks.iter().any(|mark| lower.contains(mark))
                || words.iter().any(|word| statuses.contains(word))
        })
        .map(|(class, _, _)| *class)
        .collect()
}

/// Whether `line` is itself the unknown model's refusal: it names the
/// model in a model refusal's words, and carries no class's mark.
fn refuses_the_model(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    line.contains(NO_SUCH_MODEL)
        && MODEL_REFUSALS.iter().any(|refusal| lower.contains(refusal))
        && marked(line).is_empty()
}

/// Whether `line` is itself an auth refusal: it carries an auth
/// refusal's mark and no other class's.
fn refuses_the_credentials(line: &str) -> bool {
    marked(line) == [Class::Auth]
}

/// Every line a launch printed, stderr's first.
fn lines(observation: &Observation) -> impl Iterator<Item = &str> {
    let stderr = observation.stderr.text.lines();
    stderr.chain(observation.stdout.text.lines())
}

/// The flags and `-c` keys of `controls`, by which a refusal names them.
fn control_words(controls: &[String]) -> Vec<&str> {
    let words = controls
        .iter()
        .filter_map(|part| match part.strip_prefix('-') {
            Some(_) => Some(part.as_str()),
            None => part.split_once('=').map(|(key, _)| key),
        });
    words.collect()
}

/// Whether a line the launch printed is itself the refusal of one of
/// `controls`' flags or keys: it names one in a control refusal's words,
/// and carries no class's mark (#484). A line that only echoes a control,
/// or names it beside an outage or the account, refuses nothing.
pub(super) fn refuses_a_control(observation: &Observation, controls: &[String]) -> bool {
    let named = control_words(controls);
    lines(observation).any(|line| {
        let lower = line.to_ascii_lowercase();
        let mut words = line.split(|c: char| !(c.is_ascii_alphanumeric() || "-_.".contains(c)));
        words.any(|word| named.contains(&word))
            && CONTROL_REFUSALS
                .iter()
                .any(|refusal| lower.contains(refusal))
            && marked(line).is_empty()
    })
}

/// How a deliberate mistake was refused, measured only when a line the
/// launch printed is itself that refusal by `is_one`, since a line that
/// echoes the mistake, or a failure of another class, does not show the
/// mistake was what it refused (decision 0071 ruling 3); otherwise
/// unmeasured, `not_shown` saying why.
fn classed(trial: &Trial, is_one: fn(&str) -> bool, not_shown: &str) -> Fact<Refusal> {
    let read = refusal(trial);
    match (trial, read.value()) {
        (Trial::Observed(observation), Some(_)) if !lines(observation).any(is_one) => {
            Fact::unmeasured(format!("{not_shown}: {}", exit_and_excerpt(observation)))
        }
        _ => read,
    }
}

/// How the unknown model was refused, by [`refuses_the_model`].
pub(super) fn config_refusal(trial: &Trial) -> Fact<Refusal> {
    let not_shown = format!(
        "no line of the refusal names the model {NO_SUCH_MODEL} in a model refusal's words and \
         no other class's, so it is not shown to be the configuration's"
    );
    classed(trial, refuses_the_model, &not_shown)
}

/// How the launch without credentials was refused, by
/// [`refuses_the_credentials`]: removing them is the trigger, not proof
/// of the class (#484).
pub(super) fn auth_refusal(trial: &Trial) -> Fact<Refusal> {
    let not_shown = "no line of the refusal carries an auth refusal's mark and no other \
                     class's, so it is not shown to be an auth failure";
    classed(trial, refuses_the_credentials, not_shown)
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
