//! What a seat's checkpoints metered, by the one rule a seat's tokens are
//! read by: its turns, else its finishing session record, never both. The
//! run view's participants and the attempt walk the analytics readout and
//! the recipe's graph share both read it here (decision 0071 ruling 5).

use serde_json::Value;

use crate::{add_token, finish_usage, token_usage, TokenUsage};

/// The sum of `usages`, by the rule above [`add_token`]: each one's
/// `total_tokens` is not read but made again from the summed parts.
pub(crate) fn sum_usage(usages: impl Iterator<Item = TokenUsage>) -> Option<TokenUsage> {
    let mut sum = TokenUsage::default();
    for usage in usages {
        add_token(&mut sum.input_tokens, usage.input_tokens);
        add_token(&mut sum.output_tokens, usage.output_tokens);
        add_token(&mut sum.cache_read_tokens, usage.cache_read_tokens);
        add_token(&mut sum.cache_write_tokens, usage.cache_write_tokens);
        add_token(
            &mut sum.reasoning_output_tokens,
            usage.reasoning_output_tokens,
        );
    }
    finish_usage(sum)
}

/// One checkpoint's metering, read once: the tokens it carries, and
/// whether it is one of its seat's turns.
#[derive(Clone)]
pub(crate) struct Meter {
    turn: bool,
    usage: Option<TokenUsage>,
}

/// Which of a seat's checkpoints meter it: its turns when any of them
/// carries tokens, else its finishing `session` record alone. It is
/// decided over all the seat's checkpoints, so a window holding only some
/// of them never reads both, which would count the same work twice.
#[derive(Clone, Copy)]
pub(crate) enum Metering {
    Turns,
    Session,
}

impl Metering {
    pub(crate) fn of<'a>(meters: impl IntoIterator<Item = &'a Meter>) -> Metering {
        match meters
            .into_iter()
            .any(|meter| meter.turn && meter.usage.is_some())
        {
            true => Metering::Turns,
            false => Metering::Session,
        }
    }
}

impl Meter {
    pub(crate) fn of(checkpoint: &Value) -> Meter {
        Meter {
            turn: matches!(
                checkpoint.get("step").and_then(Value::as_str),
                Some("seat-turn" | "turn-completed")
            ),
            usage: token_usage(std::iter::once(checkpoint)),
        }
    }

    /// The tokens this checkpoint adds to its seat metered `by`;
    /// `session` says whether it is the seat's session record.
    pub(crate) fn counted(&self, by: Metering, session: bool) -> Option<TokenUsage> {
        let counts = match by {
            Metering::Turns => self.turn,
            Metering::Session => session,
        };
        self.usage.clone().filter(|_| counts)
    }
}
