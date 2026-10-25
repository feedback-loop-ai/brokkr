//! What a participant's records told the run view, typed once (#753;
//! decision 0089 slice 2b): read from its checkpoints by the participant
//! scan, or from the facts its seat rows group into, so every derivation
//! after it, its cells, its activity, its terminal line and its node, reads
//! one shape whichever way the run view was read.

use brokkr_core::seat::meter::{self, sum_usage, Meter, Metering};
pub(crate) use brokkr_core::seat::row::Finish;
use brokkr_core::seat::row::{Member, SeatFacts};
use brokkr_core::seat::{reported_cost, TokenUsage};
use serde_json::Value;

use crate::{field, js, Build};

/// A working participant's last `seat-turn`: its tool as shown, and the
/// target it names as text that says anything.
pub(crate) struct Turn {
    pub(crate) tool: String,
    pub(crate) target: Option<String>,
}

/// A participant's own facts, never its members': a seat's members are
/// summed or joined where the view reads them.
#[derive(Default)]
pub(crate) struct Told {
    /// Every attempt's reported cost, summed in journal order.
    pub(crate) spent: Option<f64>,
    /// The last attempt's finishing cost alone: what a retry's own session
    /// reported, labelled apart from the sum so neither is read as the other.
    pub(crate) last_cost: Option<f64>,
    /// Its turns' and its session record's tokens, by the rule [`Metering`]
    /// states, each message once.
    pub(crate) usage: Option<TokenUsage>,
    /// The session its last session record names.
    pub(crate) session_id: Option<String>,
    /// A member's first and last checkpoint's time: when its clock starts,
    /// and where it ends once its effect is done. Read for a member alone.
    pub(crate) first_at: Option<String>,
    pub(crate) last_at: Option<String>,
    pub(crate) turn: Option<Turn>,
    /// A member's last panel finish.
    pub(crate) finish: Option<Finish>,
    /// The word that finish's outcome states, which a node's state reads.
    pub(crate) word: Option<String>,
}

impl Told {
    /// What `part`'s checkpoints told, read once the scan has seen them all.
    pub(crate) fn scanned(part: &Build) -> Told {
        let turn = part.last_turn.as_ref().map(|checkpoint| Turn {
            tool: js::to_display(checkpoint.get("tool")),
            target: field(checkpoint, "target")
                .filter(|target| !target.is_empty())
                .map(str::to_string),
        });
        let outcome = part.member_outcome.as_ref();
        let finish = part.member_finished_at.as_ref().map(|at| Finish {
            at: at.clone(),
            outcome: (outcome != Some(&Value::Null)).then(|| js::to_display(outcome)),
        });
        let session = part.session.as_ref();
        let at = |checkpoint: Option<&(Value, String, Option<String>)>| {
            checkpoint.map(|(_, at, _)| at.clone())
        };
        Told {
            spent: spent(part),
            last_cost: session.and_then(reported_cost),
            usage: session_usage(part),
            session_id: session
                .and_then(|session| field(session, "session_id"))
                .map(str::to_string),
            first_at: at(part.checkpoints.first()),
            last_at: at(part.checkpoints.last()),
            turn,
            finish,
            word: outcome.and_then(Value::as_str).map(str::to_string),
        }
    }

    /// What `facts` keep of a participant, its own: a seat's figures its
    /// members sum are its members' to state. `pinned` is the word its last
    /// finish pinned: the rows keep any outcome as the text it shows, so an
    /// outcome given otherwise than as a word pins none, as the scan reads.
    pub(crate) fn kept(facts: &SeatFacts, pinned: Option<String>) -> Told {
        let own = |figure: Option<f64>, summed: bool| figure.filter(|_| !summed);
        let member = facts.member != Member::Seat;
        Told {
            spent: own(facts.cost, facts.cost_aggregated),
            last_cost: facts.last_attempt_cost,
            usage: facts.usage.clone().filter(|_| !facts.usage_aggregated),
            session_id: facts.session_id.clone(),
            first_at: facts.started_at.clone().filter(|_| member),
            last_at: facts.last_at.clone(),
            turn: facts.tool.clone().map(|tool| Turn {
                tool,
                target: facts.target.clone(),
            }),
            finish: facts.finish.clone(),
            word: pinned,
        }
    }
}

/// Every attempt's reported cost, summed in journal order.
fn spent(part: &Build) -> Option<f64> {
    let mut total = None;
    for (checkpoint, _, _) in &part.checkpoints {
        if let Some(cost) = reported_cost(checkpoint) {
            total = Some(total.unwrap_or(0.0) + cost);
        }
    }
    total
}

fn session_usage(part: &Build) -> Option<TokenUsage> {
    let checkpoints = part.checkpoints.iter();
    let mut meters: Vec<Meter> = checkpoints
        .map(|(checkpoint, _, _)| Meter::of(checkpoint))
        .collect();
    meter::once(&mut meters);
    let session = part.session.as_ref().map(Meter::of);
    let by = Metering::of(&meters, session.as_ref());
    let turns = meters.iter().filter_map(|meter| meter.counted(by, false));
    sum_usage(turns.chain(session.and_then(|session| session.counted(by, true))))
}
