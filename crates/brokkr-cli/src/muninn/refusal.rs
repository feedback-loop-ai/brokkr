//! Why a seat's report is not usable. Every refusal names what was wrong
//! with which entry, and its text is what the operator reads on stderr;
//! the tests match the variant and pin the text once (decision 0071
//! rulings 8 and 9).

use super::queue;

/// One refusal of a report, raised by the first check that fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Unusable {
    /// The seat ended on a result other than `proposed`; `reached` is the
    /// result it named, or `brokkr_view::ABSENT` when it named none.
    Unreached { reached: String },
    /// The report carries no `inputs` object.
    NoInputs,
    /// The report's inputs lack the array `key`.
    NoArray { key: &'static str },
    /// `what` lacks a non-empty string at `key`.
    NoText {
        what: &'static str,
        key: &'static str,
    },
    /// `what` lacks a numeric `seq`.
    NoSeq { what: &'static str },
    /// `what` cites a run fact the dossier does not state.
    Unstated {
        what: &'static str,
        run_id: String,
        seq: u64,
    },
    /// `what` cites a stated fact in a realm other than the one the
    /// dossier states it in; `stated` is that realm, or none.
    OtherRealm {
        what: &'static str,
        run_id: String,
        seq: u64,
        claimed: String,
        stated: Option<String>,
    },
    /// `what` cites a crossing the dossier does not state as a finding.
    UnstatedCrossing {
        what: &'static str,
        realm: String,
        crossing: String,
    },
    /// `what` suggests a command the dossier does not state the run admits.
    NotAdmitted {
        what: &'static str,
        command: String,
        run_id: String,
    },
    /// A work-queue entry cites a run whose work is settled.
    Settled(queue::Refusal),
}

impl From<queue::Refusal> for Unusable {
    fn from(refusal: queue::Refusal) -> Self {
        Unusable::Settled(refusal)
    }
}

impl std::fmt::Display for Unusable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unusable::Unreached { reached } => write!(
                formatter,
                "the seat reached '{reached}', not '{}'",
                super::PROPOSED
            ),
            Unusable::NoInputs => formatter.write_str("the report carries no 'inputs' object"),
            Unusable::NoArray { key } => {
                write!(formatter, "the report is missing the '{key}' array")
            }
            Unusable::NoText { what, key } => {
                write!(formatter, "{what} is missing a non-empty '{key}'")
            }
            Unusable::NoSeq { what } => write!(formatter, "{what} is missing a numeric 'seq'"),
            Unusable::Unstated { what, run_id, seq } => write!(
                formatter,
                "{what} cites {run_id} seq {seq}, which the dossier does not state"
            ),
            Unusable::OtherRealm {
                what,
                run_id,
                seq,
                claimed,
                stated,
            } => {
                let stated = match stated {
                    Some(realm) => format!("realm '{realm}'"),
                    None => "no realm".to_string(),
                };
                write!(
                    formatter,
                    "{what} cites {run_id} seq {seq} in realm '{claimed}'; the dossier states \
                     that fact in {stated}"
                )
            }
            Unusable::UnstatedCrossing {
                what,
                realm,
                crossing,
            } => write!(
                formatter,
                "{what} cites crossing '{crossing}' in realm '{realm}', which the dossier \
                 does not state as a finding"
            ),
            Unusable::NotAdmitted {
                what,
                command,
                run_id,
            } => write!(
                formatter,
                "{what} suggests '{command}' for {run_id}, which is not an operator \
                 command the dossier states for it"
            ),
            Unusable::Settled(refusal) => refusal.fmt(formatter),
        }
    }
}

impl std::error::Error for Unusable {}
