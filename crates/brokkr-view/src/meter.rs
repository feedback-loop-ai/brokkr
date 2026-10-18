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

/// One checkpoint's metering, read once: the tokens it carries, whether
/// it is one of its seat's turns, and whether, as a Claude session record
/// reporting output, it settles the output its seat's turns streamed.
#[derive(Clone)]
pub(crate) struct Meter {
    turn: bool,
    settles: bool,
    usage: Option<TokenUsage>,
}

/// Which of a seat's checkpoints meter it: its turns when any of them
/// carries tokens, else its finishing `session` record alone. It is
/// decided over all the seat's checkpoints, so a window holding only some
/// of them never reads both, which would count the same work twice.
///
/// A Claude seat's turns are `Settled`: their input and cache figures are
/// final, but a streamed line carries its message's output before it is
/// final, and Claude reports what it spent thinking in its results alone.
/// Its session record sums the results (each its own query's), so the
/// seat's output and reasoning are that record's. Batch 2's implement
/// seat streamed 2k output and its results reported 103,479.
#[derive(Clone, Copy)]
pub(crate) enum Metering {
    Turns,
    Settled,
    Session,
}

impl Metering {
    /// How a seat with `meters` and the session record `session` is read.
    pub(crate) fn of<'a>(
        meters: impl IntoIterator<Item = &'a Meter>,
        session: Option<&Meter>,
    ) -> Metering {
        let turns = meters
            .into_iter()
            .any(|meter| meter.turn && meter.usage.is_some());
        match (turns, session.is_some_and(|session| session.settles)) {
            (true, true) => Metering::Settled,
            (true, false) => Metering::Turns,
            (false, _) => Metering::Session,
        }
    }
}

/// Each message metered once. Claude Code streams an assistant message
/// as a line per content block, every line repeating the message's usage,
/// and its driver journaled a turn with that usage for each line until it
/// learned to carry it on the message's first line alone: a Claude seat's
/// turns summed as written read its tokens two and a half times over. A
/// turn whose usage is the one the seat's last metered turn carried is
/// that message again, and meters nothing; a turn that carries none, as
/// the tool uses after a message's first, is passed over. Two messages of
/// one session do not report the same counts, since each reads the
/// context the one before it grew. `meters` are one seat's, in order.
pub(crate) fn once<'a>(meters: impl IntoIterator<Item = &'a mut Meter>) {
    let mut last = None;
    let metered = meters
        .into_iter()
        .filter(|meter| meter.turn && meter.usage.is_some());
    for meter in metered {
        if meter.usage == last {
            meter.usage = None;
        } else {
            last.clone_from(&meter.usage);
        }
    }
}

impl Meter {
    pub(crate) fn of(checkpoint: &Value) -> Meter {
        let step = checkpoint.get("step").and_then(Value::as_str);
        let usage = token_usage(std::iter::once(checkpoint));
        // `claude-code-` and `claude-lanetally-session-finished`: the
        // driver's own step, the one mark a journal already written bears.
        let claude = step
            .is_some_and(|step| step.starts_with("claude-") && step.ends_with("-session-finished"));
        Meter {
            turn: matches!(step, Some("seat-turn" | "turn-completed")),
            settles: claude
                && usage
                    .as_ref()
                    .is_some_and(|usage| usage.output_tokens.is_some()),
            usage,
        }
    }

    /// The tokens this checkpoint adds to its seat metered `by`;
    /// `session` says whether it is the seat's session record.
    pub(crate) fn counted(&self, by: Metering, session: bool) -> Option<TokenUsage> {
        let usage = self.usage.clone();
        match by {
            Metering::Turns => usage.filter(|_| self.turn),
            Metering::Session => usage.filter(|_| session),
            Metering::Settled if self.turn => usage.map(|usage| TokenUsage {
                output_tokens: None,
                reasoning_output_tokens: None,
                ..usage
            }),
            Metering::Settled => usage.filter(|_| session).map(|usage| TokenUsage {
                output_tokens: usage.output_tokens,
                reasoning_output_tokens: usage.reasoning_output_tokens,
                ..TokenUsage::default()
            }),
        }
    }
}
