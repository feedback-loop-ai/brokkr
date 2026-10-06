//! The deliberate mistakes' refusals, read as the typed forms the
//! harness's reader recognises, and the accepted efforts one of them
//! lists. Pure, like `measure`.

use super::read::{Class, Harness, Levels, Refused, Said};
use super::{exit_and_excerpt, exit_text, said_in, Stream, EXCERPT_CHARS};
use crate::probe::facts::{Fact, Refusal};
use crate::probe::observe::{Observation, Trial};
use crate::probe::plan::{NO_SUCH_EFFORT, NO_SUCH_MODEL};

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

/// Everything a line the launch printed states, stderr's lines first,
/// each beside its line, as `harness`'s reader reads it; a line the
/// reader does not read states nothing.
fn said(observation: &Observation, harness: Harness) -> Vec<(&str, Said)> {
    let stderr = observation.stderr.text.lines();
    let lines = stderr.chain(observation.stdout.text.lines());
    let said = lines.flat_map(|line| harness.said(line).into_iter().map(move |said| (line, said)));
    said.collect()
}

/// Every refusal a line the launch printed states, each beside its line:
/// a refusal is a form `harness`'s reader recognises, which names its
/// class and its object (#484).
fn refusals(observation: &Observation, harness: Harness) -> Vec<(&str, Refused)> {
    let refused = said(observation, harness)
        .into_iter()
        .filter_map(|(line, said)| {
            let Said::Refusal(refused) = said else {
                return None;
            };
            Some((line, refused))
        });
    refused.collect()
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

/// Whether a line of a turn's `streams` is the CLI's refusal of one of
/// `controls`' flags or keys: a control refusal whose object is one of
/// them, as the turn's reader read it. A line that echoes a control, or
/// names it in any other words, refuses nothing (#484).
pub(super) fn refuses_a_control(streams: &[Stream], controls: &[String]) -> bool {
    let named = control_words(controls);
    let said = streams.iter().flat_map(said_in);
    said.into_iter().any(|(_, said)| {
        matches!(said, Said::Refusal(Refused { class: Class::Control, object })
            if named.contains(&object.as_str()))
    })
}

/// How a deliberate mistake was refused, measured only when a line the
/// launch printed is a refusal `is_one` takes, its excerpt being that
/// line, since a line that echoes the mistake, or a failure of another
/// class, does not show the mistake was what it refused (decision 0071
/// ruling 3); otherwise unmeasured, `not_shown` saying why.
fn classed(
    trial: &Trial,
    harness: Harness,
    is_one: fn(&Refused) -> bool,
    not_shown: &str,
) -> Fact<Refusal> {
    let accepted = "the CLI exited 0, so there was no refusal to read".to_string();
    let observation = match refused(trial, accepted) {
        Ok(observation) => observation,
        Err(fact) => return fact,
    };
    let refusals = refusals(observation, harness);
    match refusals.iter().find(|(_, refused)| is_one(refused)) {
        Some((line, _)) => {
            let excerpt: String = line.trim().chars().take(EXCERPT_CHARS).collect();
            let evidence = format!("{}: {excerpt}", exit_text(observation.exit));
            Fact::measured(
                Refusal {
                    exit: observation.exit,
                    excerpt,
                },
                evidence,
            )
        }
        None => Fact::unmeasured(format!("{not_shown}: {}", exit_and_excerpt(observation))),
    }
}

/// How the unknown model was refused: by a configuration refusal whose
/// object is that model.
pub(super) fn config_refusal(trial: &Trial, harness: Harness) -> Fact<Refusal> {
    let not_shown = format!(
        "no line of the refusal is a configuration refusal of the model {NO_SUCH_MODEL}, so it \
         is not shown to be the configuration's"
    );
    let is_one =
        |refused: &Refused| refused.class == Class::Config && refused.object == NO_SUCH_MODEL;
    classed(trial, harness, is_one, &not_shown)
}

/// How the launch without credentials was refused: by an auth refusal,
/// since removing them is the trigger, not proof of the class (#484).
pub(super) fn auth_refusal(trial: &Trial, harness: Harness) -> Fact<Refusal> {
    let not_shown = "no line of the refusal is an auth refusal, so it is not shown to be an \
                     auth failure";
    classed(
        trial,
        harness,
        |refused| refused.class == Class::Auth,
        not_shown,
    )
}

/// The levels the unknown effort's launch was refused with: measured
/// only when a line is `harness`'s typed effort refusal whose object is
/// that effort, its evidence being that line, and its levels the list
/// that form reads (#484); a line that lists levels in any other words,
/// or refuses another level, lists none.
pub(super) fn efforts(trial: &Trial, harness: Harness) -> Fact<Vec<String>> {
    let accepted = format!(
        "the CLI accepted the unknown effort '{NO_SUCH_EFFORT}' and exited 0, so no refusal \
         lists its levels"
    );
    let observation = match refused(trial, accepted) {
        Ok(observation) => observation,
        Err(fact) => return fact,
    };
    let listed = said(observation, harness)
        .into_iter()
        .find_map(|(line, said)| {
            let Said::Levels(Levels { refused, accepted }) = said else {
                return None;
            };
            (refused == NO_SUCH_EFFORT).then_some((line, accepted))
        });
    match listed {
        Some((line, levels)) => {
            let excerpt: String = line.trim().chars().take(EXCERPT_CHARS).collect();
            Fact::measured(
                levels,
                format!("{}: {excerpt}", exit_text(observation.exit)),
            )
        }
        None => Fact::unmeasured(format!(
            "no line of the refusal is an effort refusal of {NO_SUCH_EFFORT} listing the levels \
             accepted: {}",
            exit_and_excerpt(observation)
        )),
    }
}
