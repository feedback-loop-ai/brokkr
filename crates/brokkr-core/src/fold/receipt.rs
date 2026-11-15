//! The word a bridge command receipt carries: every journaled refusal is
//! a [`Refusal`](super::Refusal), and the reserved annotation word's
//! refusal is receipt-only. One typed home, beside `Refusal` (decision
//! 0108 ruling 4, #440 fix round 440c4).

use super::Refusal;

/// Why a bridge command receipt was refused: every journaled refusal is a
/// [`Refusal`], and [`ReceiptReason::ReservedCommand`] is receipt-only —
/// a reserved annotation word is answered and nothing is journaled for it
/// (decision 0108 ruling 4, #440 fix round 440c4). This is the one home
/// for the receipt's reason vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptReason {
    /// A refusal `operator/rejected` journals.
    Refusal(Refusal),
    /// A reserved annotation word's refusal, never journaled.
    ReservedCommand,
}

impl Refusal {
    /// The refusal `word` names, or `None` for a word this build does not
    /// know. A receipt read from a journal turns its reason back into the
    /// typed vocabulary with this, and falls back to
    /// [`Refusal::PreviouslyRejected`] when an older binary's word is not
    /// one of ours (decision 0108 ruling 4, #440 fix round 440c4).
    pub fn parse(word: &str) -> Option<Refusal> {
        const ALL: [Refusal; 8] = [
            Refusal::CommandNotAllowed,
            Refusal::AfterTerminal,
            Refusal::RunNotAwaitingOperator,
            Refusal::NoPhaseToRetry,
            Refusal::LostFence,
            Refusal::StaleCursor,
            Refusal::IncompleteCommandReplay,
            Refusal::PreviouslyRejected,
        ];
        ALL.into_iter().find(|refusal| refusal.word() == word)
    }
}

impl ReceiptReason {
    /// The word the receipt carries.
    pub const fn word(self) -> &'static str {
        match self {
            ReceiptReason::Refusal(refusal) => refusal.word(),
            ReceiptReason::ReservedCommand => "reserved_command",
        }
    }
}
