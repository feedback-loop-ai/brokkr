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

impl ReceiptReason {
    /// The word the receipt carries.
    pub const fn word(self) -> &'static str {
        match self {
            ReceiptReason::Refusal(refusal) => refusal.word(),
            ReceiptReason::ReservedCommand => "reserved_command",
        }
    }
}
