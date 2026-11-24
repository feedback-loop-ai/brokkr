//! The one fold of a route's rows into what it stands at (fix round
//! 0106b1r14, L1), each end it shows kept with the kind of row it came from
//! (fix round 0106b1r15, L1).

use time::OffsetDateTime;

use super::Cooldown;

/// The aggregate one provider and route stands at, at an instant: the
/// latest ordinary row's end and words, and a standing probe hold's own
/// deadline and words. Within each kind the end is a monotonic MAX, so a
/// later, shorter refusal never shortens a wait, and a probe's words are
/// never shown for an ordinary row's end, nor the reverse (fix round
/// 0106b1r15, L1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CooldownAggregate {
    /// Only ordinary rows stand: the one that ends last.
    Ordinary(RowEnd),
    /// A probe's hold stands on the route (fix round 0106b1r13, H1): no
    /// time ends it, so its deadline is a readout, never an end the route
    /// opens at, and the route stays held until a read finds the hold
    /// ended. An ordinary row beside it keeps its own end and words, so a
    /// longer ordinary cool-down still ends where its row says (fix round
    /// 0106b1r14, M1).
    Held {
        probe: RowEnd,
        ordinary: Option<RowEnd>,
    },
}

/// One row's end and words, as the aggregate shows them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowEnd {
    pub until: OffsetDateTime,
    pub reason: String,
}

impl CooldownAggregate {
    /// `held`, the aggregate a route's earlier rows came to in the order
    /// the store read them (rising `seq`), joined with `row`: the one home
    /// of a route's latest end, which the engine's [`crate::Store::cooldown`]
    /// and admission's pass and claim fold alike (fix round 0106b1r14, L1).
    /// A row joins the end of its own kind only: a later, shorter row never
    /// shortens it, a row at the same end replaces the earlier one's words,
    /// and a standing probe hold among the rows is kept whichever row ends
    /// last, with its own deadline and words (fix round 0106b1r15, L1).
    pub fn joined(held: Option<CooldownAggregate>, row: &Cooldown) -> CooldownAggregate {
        let (probe, ordinary) = match held {
            None => (None, None),
            Some(CooldownAggregate::Ordinary(ordinary)) => (None, Some(ordinary)),
            Some(CooldownAggregate::Held { probe, ordinary }) => (Some(probe), ordinary),
        };
        match (row.probe, probe) {
            (true, probe) => CooldownAggregate::Held {
                probe: RowEnd::later(probe, row),
                ordinary,
            },
            (false, None) => CooldownAggregate::Ordinary(RowEnd::later(ordinary, row)),
            (false, Some(probe)) => CooldownAggregate::Held {
                probe,
                ordinary: Some(RowEnd::later(ordinary, row)),
            },
        }
    }

    /// The standing probe hold's deadline and words, where one stands.
    pub fn probe(&self) -> Option<&RowEnd> {
        match self {
            CooldownAggregate::Ordinary(_) => None,
            CooldownAggregate::Held { probe, .. } => Some(probe),
        }
    }

    /// The latest ordinary row's end and words, where one stands.
    pub fn ordinary(&self) -> Option<&RowEnd> {
        match self {
            CooldownAggregate::Ordinary(ordinary) => Some(ordinary),
            CooldownAggregate::Held { ordinary, .. } => ordinary.as_ref(),
        }
    }
}

impl RowEnd {
    /// `kept`, the end of `row`'s kind so far, or `row`'s own where it ends
    /// at or after it.
    fn later(kept: Option<RowEnd>, row: &Cooldown) -> RowEnd {
        match kept {
            Some(kept) if kept.until > row.until_at => kept,
            _ => RowEnd {
                until: row.until_at,
                reason: row.reason.clone(),
            },
        }
    }
}
