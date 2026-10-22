//! The operator's choices for one run, derived once (#568, design D2):
//! the four commands Brokkr has, each admitted or refused with the words
//! the verb would give, and the actions a console shows disabled, each
//! with its fixed reason. A surface offers what this says and decides
//! nothing itself (decision 0013); the engine never admits by it.

use brokkr_core::fold::{
    acceptance_refusal, concluded, concluded_text, OperatorCommand, Refusal, RunState, Status,
};
use brokkr_core::residual::ResidualPhase;
use serde::{Serialize, Serializer};

use crate::clarify::Clarify;
use crate::ResidualFinding;

/// One of the four commands Brokkr has for a run.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    /// `brokkr operator retry`: re-runs the phase the run parked in, or
    /// would, had it entered one.
    Retry { phase: Option<String> },
    /// `brokkr operator stop`.
    Stop,
    /// `brokkr resume`: drives the run again under its pinned bundle.
    Resume,
    /// `brokkr conclude`: closes the run from its journal alone.
    Conclude,
}

impl Command {
    /// The operator command this is, for the two `brokkr operator` takes.
    fn operator(&self) -> Option<OperatorCommand> {
        match self {
            Command::Retry { .. } => Some(OperatorCommand::Retry),
            Command::Stop => Some(OperatorCommand::Stop),
            Command::Resume | Command::Conclude => None,
        }
    }
}

/// Why a command would be refused now. Each variant is one verb's rule,
/// read where that rule lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandRefusal {
    /// `retry` or `stop`, refused by the fold's one rule
    /// ([`acceptance_refusal`]).
    Operator {
        command: OperatorCommand,
        refusal: Refusal,
    },
    /// `conclude` on a run that already has its conclusion ([`concluded`]).
    Concluded(Status),
    /// `resume` on a parked run: it is driven again only once a command
    /// is accepted.
    Parked,
    /// `resume` on a run that already has its conclusion.
    Ended(Status),
}

impl CommandRefusal {
    /// The word the refusal is known by: the journal's word for the fold's
    /// refusals and the status's for the rest.
    pub(crate) fn word(self) -> &'static str {
        match self {
            CommandRefusal::Operator { refusal, .. } => refusal.word(),
            CommandRefusal::Concluded(_) => "already_concluded",
            CommandRefusal::Parked => Status::AwaitingOperator.as_str(),
            CommandRefusal::Ended(_) => Refusal::AfterTerminal.word(),
        }
    }

    /// The words the verb gives for it, on run `run_id`.
    pub(crate) fn text(self, run_id: &str) -> String {
        match self {
            CommandRefusal::Operator { command, refusal } => {
                operator_refusal(command.as_str(), refusal.word(), run_id)
            }
            CommandRefusal::Concluded(status) => concluded_text(run_id, status),
            CommandRefusal::Parked => format!(
                "run '{run_id}' awaits the operator; resume drives it again only once a \
                 retry is accepted"
            ),
            CommandRefusal::Ended(status) => format!(
                "run '{run_id}' is already concluded ({}); resume drives nothing",
                status.as_str()
            ),
        }
    }
}

impl Serialize for CommandRefusal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.word())
    }
}

/// What `brokkr operator` prints when the journal refuses `command` with
/// the word `reason`, on run `run_id`. The verb prints this and the view
/// offers it, so the two never say it two ways.
pub fn operator_refusal(command: &str, reason: &str, run_id: &str) -> String {
    format!(
        "refused operator {command} ({reason}): the run is not in a state this command can \
         apply to; the refusal is journaled. Read it with: brokkr inspect --run {run_id}"
    )
}

/// A member retry refused before a command was journaled.
pub fn member_retry_refusal(word: &str) -> String {
    format!("refused operator retry --member ({word}): the run is not in a state a retry can apply to; nothing was journaled")
}

/// Whether Brokkr would take a command now.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "admission", rename_all = "snake_case")]
pub enum Admission {
    Admitted,
    Refused {
        refusal: CommandRefusal,
        text: String,
    },
}

/// One command and whether Brokkr would take it now.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Offered {
    #[serde(flatten)]
    pub command: Command,
    #[serde(flatten)]
    pub admission: Admission,
}

impl Offered {
    fn new(command: Command, refusal: Option<CommandRefusal>, run_id: &str) -> Offered {
        let admission = match refusal {
            None => Admission::Admitted,
            Some(refusal) => Admission::Refused {
                text: refusal.text(run_id),
                refusal,
            },
        };
        Offered { command, admission }
    }
}

/// The fold's rule for `command`, as a [`CommandRefusal`].
fn operator_rule(state: &RunState, command: OperatorCommand) -> Option<CommandRefusal> {
    acceptance_refusal(state, command).map(|refusal| CommandRefusal::Operator { command, refusal })
}

/// `resume` drives a running run, and nothing else: a parked run moves
/// only once a command is accepted, and a concluded one never.
fn resume_rule(state: &RunState) -> Option<CommandRefusal> {
    match state.run.status() {
        Status::Running => None,
        Status::AwaitingOperator => Some(CommandRefusal::Parked),
        status @ (Status::Completed | Status::Stopped) => Some(CommandRefusal::Ended(status)),
    }
}

/// The four commands for the run as folded, in the order a console offers
/// them, each admitted or refused by the rule its verb applies.
pub(crate) fn commands(state: &RunState) -> Vec<Offered> {
    let run_id = state.run_id.as_str();
    let retry = Command::Retry {
        phase: state.phase.clone(),
    };
    vec![
        Offered::new(retry, operator_rule(state, OperatorCommand::Retry), run_id),
        Offered::new(
            Command::Stop,
            operator_rule(state, OperatorCommand::Stop),
            run_id,
        ),
        Offered::new(Command::Resume, resume_rule(state), run_id),
        Offered::new(
            Command::Conclude,
            concluded(state).map(CommandRefusal::Concluded),
            run_id,
        ),
    ]
}

/// The operator commands this run admits through the bridge's door, by
/// the word the journal records: none unless it is parked, the only state
/// that door takes one in, and then each of [`commands`]' admitted
/// `retry` and `stop`, so a run parked before any phase offers `stop`
/// alone. A surface that suggests a command suggests one the engine will
/// actually accept.
pub fn operator_commands(state: &RunState) -> Vec<String> {
    match state.run.status() {
        Status::AwaitingOperator => commands(state)
            .into_iter()
            .filter(|offered| offered.admission == Admission::Admitted)
            .filter_map(|offered| offered.command.operator())
            .map(|command| command.as_str().to_string())
            .collect(),
        Status::Running | Status::Completed | Status::Stopped => Vec::new(),
    }
}

/// An action a console shows disabled, because Brokkr cannot do it yet.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Unavailable {
    /// Supersede annotates only a finished run (decision 0047).
    DismissFinding,
    /// Gates start fresh each time (decision 0056).
    TalkToReviewer,
    /// `hold` acts on queue entries only.
    Hold,
    /// No channel carries an answer to a run.
    AnswerHere,
}

impl Unavailable {
    /// Every action, in the order a console lists them.
    pub(crate) const ALL: [Unavailable; 4] = [
        Unavailable::DismissFinding,
        Unavailable::TalkToReviewer,
        Unavailable::Hold,
        Unavailable::AnswerHere,
    ];

    /// The fixed reason a console shows beside the disabled action.
    pub(crate) const fn reason(self) -> &'static str {
        match self {
            Unavailable::DismissFinding => {
                "Brokkr only lets you dismiss findings once a run has finished."
            }
            Unavailable::TalkToReviewer => {
                "Reviews start fresh each time; your note can't reach the reviewer yet."
            }
            Unavailable::Hold => "A parked run already waits for you; it holds nothing else back.",
            Unavailable::AnswerHere => {
                "Not yet: Brokkr has no way to pass an answer to a run directly."
            }
        }
    }

    /// Whether a console shows this action for a run.
    fn shown(self, state: &RunState, open_finding: bool, clarify: bool) -> bool {
        let parked = state.run.status() == Status::AwaitingOperator;
        match self {
            // On a finished run the finding can be dismissed, by supersede.
            Unavailable::DismissFinding => open_finding && concluded(state).is_none(),
            // The review gates are the phases whose rulings carry findings.
            Unavailable::TalkToReviewer => {
                parked
                    && state
                        .phase
                        .as_deref()
                        .and_then(ResidualPhase::named)
                        .is_some()
            }
            Unavailable::Hold => parked,
            Unavailable::AnswerHere => clarify,
        }
    }
}

/// One disabled action and its reason.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Disabled {
    pub action: Unavailable,
    pub reason: &'static str,
}

/// The actions a console shows disabled for this run: dismissing an open
/// residual finding on a live run, talking to the reviewer on a run parked
/// at a review, holding a parked run, and answering a clarify question
/// the run carries.
pub(crate) fn unavailable(
    state: &RunState,
    residuals: &[ResidualFinding],
    clarify: Option<&Clarify>,
) -> Vec<Disabled> {
    let open_finding = residuals.iter().any(|finding| finding.superseded.is_none());
    Unavailable::ALL
        .into_iter()
        .filter(|action| action.shown(state, open_finding, clarify.is_some()))
        .map(|action| Disabled {
            action,
            reason: action.reason(),
        })
        .collect()
}

#[cfg(test)]
mod tests;
