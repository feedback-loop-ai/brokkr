//! The policy input vocabulary, once (#345): every input a table may
//! read or a decision may record, the kind of value it holds and who
//! supplies it. The lists the loader, the compiler and the engine read
//! are projections of [`INPUTS`], so an input is added by one row.

use super::{DRIFT_PHASES, REALM_FACTS, REVIEWED_HEADS, STRATEGIES};

/// The consecutive-failure counter (decision 0006): journal-computed,
/// and the one counter the table's `_gte` conditions name by itself.
pub const CONSECUTIVE_FAILURES: &str = "consecutive_failures";

/// The journal-computed strategy input shared by policy and the fold.
pub(crate) const STRATEGY: &str = "strategy";

/// What blocked a seat that returns `blocked` (decision 0092): its
/// environment (a signing key, a tool, a capability its box refused),
/// which the operator fixes before retrying the seat, or the work (a
/// framing or specification problem), which routes as it always did.
pub(crate) const BLOCKERS: [&str; 2] = ["environment", "work"];

/// What an input's value is, which decides the conditions that read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputKind {
    /// A boolean, read by the condition keyed by its own name.
    Flag,
    /// A count, read by `<name>_gte`.
    Counter,
    /// A severity word, read by `<name>_above` and `<name>_at_most`.
    Severity,
    /// A word from a closed vocabulary, read by the condition `key` as
    /// the list of words it allows.
    Choice {
        key: &'static str,
        vocabulary: &'static [&'static str],
    },
    /// A handoff identity: data passed to effects, which policy may
    /// never branch on.
    Identifier,
    /// A record a decision keeps beside the facts, which no condition
    /// reads.
    Record,
}

/// Who supplies an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Owner {
    /// Journal-computed truth, never accepted from a caller (README law
    /// 2): a seat may neither declare nor claim it.
    Engine,
    /// A seat's declared fact (decision 0007).
    Seat,
}

/// One input of the vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Input {
    pub(crate) name: &'static str,
    pub(crate) kind: InputKind,
    pub(crate) owner: Owner,
}

const fn input(name: &'static str, kind: InputKind, owner: Owner) -> Input {
    Input { name, kind, owner }
}

/// The vocabulary. The phase-visit family (`visits_<phase>`, decision
/// 0022) closes over each table's own phases, so it is a prefix, not a
/// row. A projection keeps table order, which the loader's refusals
/// print.
pub(crate) const INPUTS: [Input; 16] = [
    input(CONSECUTIVE_FAILURES, InputKind::Counter, Owner::Engine),
    input("skip_verify", InputKind::Flag, Owner::Seat),
    input("fixes_applied", InputKind::Flag, Owner::Seat),
    // Decision 0041: a review panel may return a defective specification
    // to design. Seat-declarable; unlike visit counts, the judge observes it.
    input("spec_defect", InputKind::Flag, Owner::Seat),
    input("has_security_residual", InputKind::Flag, Owner::Seat),
    input("high_risk_uncovered", InputKind::Flag, Owner::Seat),
    input("drift_detected", InputKind::Flag, Owner::Engine),
    input("dirty_worktrees", InputKind::Flag, Owner::Engine),
    // Decision 0039: every commit the protected phase added since it
    // was entered lies in the repository's docs class, read from the
    // tree at the protected phase's ruling.
    input("fixes_docs_only", InputKind::Flag, Owner::Engine),
    input("max_residual_severity", InputKind::Severity, Owner::Seat),
    input("change", InputKind::Identifier, Owner::Seat),
    // The fold remembers the last successful triage result. A seat may
    // neither declare nor overwrite the class that governs its run.
    input(
        STRATEGY,
        InputKind::Choice {
            key: "strategy_in",
            vocabulary: &STRATEGIES,
        },
        Owner::Engine,
    ),
    // Artifact ownership reported by the read-only analysis judge.
    input(
        "drift_in",
        InputKind::Choice {
            key: "drift_in",
            vocabulary: &DRIFT_PHASES,
        },
        Owner::Seat,
    ),
    // Decision 0092: a blocked seat names what blocked it, so a table
    // may park an environment block for the operator.
    input(
        "blocked_by",
        InputKind::Choice {
            key: "blocked_by_in",
            vocabulary: &BLOCKERS,
        },
        Owner::Seat,
    ),
    input(REVIEWED_HEADS, InputKind::Record, Owner::Engine),
    // The same repository facts, keyed by realm (decision 0023).
    input(REALM_FACTS, InputKind::Record, Owner::Engine),
];

impl Input {
    /// A choice input's condition key and vocabulary.
    pub(crate) const fn choice(&self) -> Option<(&'static str, &'static [&'static str])> {
        match self.kind {
            InputKind::Choice { key, vocabulary } => Some((key, vocabulary)),
            InputKind::Flag
            | InputKind::Counter
            | InputKind::Severity
            | InputKind::Identifier
            | InputKind::Record => None,
        }
    }
}

/// Which rows a projection keeps.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Pick {
    Flags,
    Counters,
    Severities,
    Identifiers,
    EngineOwned,
}

const fn keeps(pick: Pick, input: &Input) -> bool {
    match pick {
        Pick::Flags => matches!(input.kind, InputKind::Flag),
        Pick::Counters => matches!(input.kind, InputKind::Counter),
        Pick::Severities => matches!(input.kind, InputKind::Severity),
        Pick::Identifiers => matches!(input.kind, InputKind::Identifier),
        Pick::EngineOwned => matches!(input.owner, Owner::Engine),
    }
}

/// How many rows `pick` keeps: a projection's length.
pub(crate) const fn count(pick: Pick) -> usize {
    let (mut kept, mut row) = (0, 0);
    while row < INPUTS.len() {
        if keeps(pick, &INPUTS[row]) {
            kept += 1;
        }
        row += 1;
    }
    kept
}

/// The names of the rows `pick` keeps, in table order.
pub(crate) const fn names<const N: usize>(pick: Pick) -> [&'static str; N] {
    let mut kept = [""; N];
    let (mut at, mut row) = (0, 0);
    while row < INPUTS.len() {
        if keeps(pick, &INPUTS[row]) {
            kept[at] = INPUTS[row].name;
            at += 1;
        }
        row += 1;
    }
    kept
}

/// May a seat declare `name`? Exactly the seat-owned rows: an
/// engine-owned input, or a word outside the vocabulary, is refused.
pub fn is_declarable(name: &str) -> bool {
    INPUTS
        .iter()
        .any(|input| input.name == name && input.owner == Owner::Seat)
}

/// The input a choice condition `key` reads, and its vocabulary.
pub(crate) fn choice_keyed(key: &str) -> Option<(&'static str, &'static [&'static str])> {
    INPUTS.iter().find_map(|input| {
        let (written, vocabulary) = input.choice()?;
        (written == key).then_some((input.name, vocabulary))
    })
}

/// Every choice condition, as the loader's unknown-key refusal lists
/// them: each condition key over its vocabulary, in table order.
pub(crate) fn choice_conditions() -> String {
    let conditions: Vec<String> = INPUTS
        .iter()
        .filter_map(Input::choice)
        .map(|(key, vocabulary)| format!("{key} over {vocabulary:?}"))
        .collect();
    conditions.join(", ")
}

/// The closed vocabulary a choice input's value is drawn from.
pub(crate) fn vocabulary(name: &str) -> &'static [&'static str] {
    INPUTS
        .iter()
        .find(|input| input.name == name)
        .and_then(Input::choice)
        .map_or(&[], |(_, vocabulary)| vocabulary)
}

#[cfg(test)]
mod tests;
