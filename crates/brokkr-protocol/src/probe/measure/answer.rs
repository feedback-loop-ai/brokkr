//! Whether a turn that exited clean answered (#484), and what the turn
//! read as its answer ran. A turn is read only when its stdout or a
//! transcript holds the reply the probe asks for, and no line of any of
//! its streams states a refusal or a failure, whatever the exit status
//! says. Pure, like `measure`.

use super::read::{Class, Levels, Refused, Said};
use super::{push_unique, said_in, Stream, Turn, STDERR};
use crate::probe::plan::REPLY;

/// What keeps a turn's streams from being its answer: the first refusal
/// or failure a line states, else the reply's absence; `None` when the
/// turn answered.
pub(super) fn unanswered(streams: &[Stream]) -> Option<String> {
    let mut replied = false;
    for stream in streams {
        for (line, said) in said_in(stream) {
            let at = || format!("line {line} of {}", stream.source);
            match said {
                Said::Refusal(refused) => return Some(format!("{} {}", at(), refusal(refused))),
                Said::Levels(Levels { refused, .. }) => {
                    return Some(format!("{} refuses the effort {refused}", at()))
                }
                Said::Failed { at: pointer } => {
                    return Some(format!("{} states a failure at {pointer}", at()))
                }
                Said::Reply => replied |= stream.source != STDERR,
                Said::Tools { .. }
                | Said::Servers { .. }
                | Said::Session { .. }
                | Said::Usage(_)
                | Said::Cost { .. }
                | Said::Ran { .. }
                | Said::Model(_) => {}
            }
        }
    }
    (!replied).then(|| format!("no line of its stdout or a transcript is the reply {REPLY}"))
}

/// The models a read turn, `base`, named, each once, as the headless
/// launch's evidence ends; nothing for a turn not read.
pub(super) fn models(base: &Turn) -> String {
    let Turn::Read(streams) = base else {
        return String::new();
    };
    let mut models = Vec::new();
    for (_, said) in streams.all.iter().flat_map(said_in) {
        if let Said::Model(model) = said {
            push_unique(&mut models, model.clone());
        }
    }
    if models.is_empty() {
        String::new()
    } else {
        format!("; it ran {}", models.join(", "))
    }
}

/// How a line refused, by its class and object.
fn refusal(refused: &Refused) -> String {
    let class = match refused.class {
        Class::Config => "refuses the model",
        Class::Auth => "refuses the credential",
        Class::Control => "refuses the flag or key",
    };
    format!("{class} {}", refused.object)
}
