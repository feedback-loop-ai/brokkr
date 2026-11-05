//! The cause an operator names when a run ends (decision 0064 ruling 4):
//! one of the six operator-owned endings, the citation its word requires,
//! and the cited run checked in the journal it names. This is the one home
//! of those rules (decision 0071 ruling 5), and `stop`, `conclude` and
//! `classify` each call it; every refusal here comes before anything is
//! written, and names the verb it refused.
//!
//! `stop` is here too, beside the cause it carries: its `operator/commanded`
//! writes the cause in `args` under `operator-ending.v1`, and its sentence
//! stays on the acceptance, where the stop has always written it.

use std::fmt;

use brokkr_core::ending::{Citation, Ended, Ending, UnknownEnding};
use brokkr_core::fold::OperatorCommand;
use brokkr_store::{Store, StoreError};
use serde_json::{json, Value};
use thiserror::Error;

use super::disposition::{disposed, Commanded, Disposition};
use super::{folded, EngineError, FencedCommandOutcome};

/// The verb a cause is named on. Each refusal names it, so the operator
/// reads back the command they typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    /// `brokkr operator stop`.
    Stop,
    /// `brokkr conclude`.
    Conclude,
    /// `brokkr operator classify`.
    Classify,
}

impl fmt::Display for Verb {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Verb::Stop => "stop",
            Verb::Conclude => "conclude",
            Verb::Classify => "classify",
        })
    }
}

/// The cause as the operator typed it: the ending, parsed once where it
/// was typed by [`named_cause`] (decision 0071 ruling 3), and the three
/// citation flags. Whether they fit one another each verb checks before it
/// writes anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cause<'a> {
    pub ending: Ending,
    /// The realm the cited run was read in, or `None` for the workspace
    /// journal — decision 0026 ruling 3's key, on a citation.
    pub by_realm: Option<&'a str>,
    /// The run that replaced this one; `superseded` only.
    pub by_run: Option<&'a str>,
    /// The commit or pull request the work landed by; `delivered-elsewhere`
    /// only, recorded as written and not resolved.
    pub by_ref: Option<&'a str>,
}

/// What an operator says when they name how a run ended, on any of the
/// three verbs: the cause, why, and who named it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Named<'a> {
    pub cause: Cause<'a>,
    pub reason: &'a str,
    pub operator: &'a str,
}

/// A citation flag a cause may refuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    ByRun,
    ByRealm,
    ByRef,
}

impl Flag {
    /// The flag as it is typed.
    const fn word(self) -> &'static str {
        match self {
            Flag::ByRun => "--by-run",
            Flag::ByRealm => "--by-realm",
            Flag::ByRef => "--by-ref",
        }
    }

    /// What the flag belongs to, as a refusal on `cause` names it.
    fn owner(self, cause: Ending) -> &'static str {
        match (self, cause) {
            (Flag::ByRealm, Ending::DeliveredElsewhere) => "the realm --by-run is read in",
            (Flag::ByRun | Flag::ByRealm, _) => "superseded",
            (Flag::ByRef, _) => "delivered-elsewhere",
        }
    }
}

/// Why a cause was refused. Nothing was written: every refusal comes
/// before the verb's first append.
#[derive(Debug, Error)]
pub enum CauseRefusal {
    /// No `--cause` was typed. Decision 0064 ruling 4 makes it required,
    /// and names the vocabulary.
    #[error("{verb} needs --cause: one of {}", Ending::operator_words())]
    Missing { verb: Verb },
    /// A word outside the twelve endings.
    #[error("{unknown}")]
    Unknown { verb: Verb, unknown: UnknownEnding },
    /// An ending the policy or the reading derives, never an operator.
    #[error(
        "'{}' is not an operator-owned ending; an operator names one of {}",
        .ending.word(),
        Ending::operator_words()
    )]
    NotOperatorOwned { verb: Verb, ending: Ending },
    /// `superseded` without the run that replaced this one.
    #[error("{verb} --cause superseded needs --by-run: the run that replaced this one")]
    NoReplacement { verb: Verb },
    /// `superseded` with a ref, which is `delivered-elsewhere`'s.
    #[error(
        "{verb} --cause superseded takes no --by-ref; name the run that replaced this one with \
         --by-run"
    )]
    RefOnSuperseded { verb: Verb },
    /// `delivered-elsewhere` without the commit or pull request.
    #[error(
        "{verb} --cause delivered-elsewhere needs --by-ref: the commit or pull request it \
         landed by"
    )]
    NoRef { verb: Verb },
    /// A citation flag on a cause it does not belong to.
    #[error(
        "{verb} --cause {} takes no {flag}; {flag} belongs to '{}'",
        .cause.word(),
        .flag.owner(*.cause),
        flag = .flag.word()
    )]
    Misplaced {
        verb: Verb,
        cause: Ending,
        flag: Flag,
    },
    /// The cited run is not in the journal the citation names (ruling 4):
    /// refused, never guessed at.
    #[error("run '{by_run}' in {}: {error}", journal_named(.by_realm.as_deref()))]
    Absent {
        verb: Verb,
        by_run: String,
        by_realm: Option<String>,
        error: StoreError,
    },
}

impl CauseRefusal {
    /// The verb that refused.
    pub fn verb(&self) -> Verb {
        match self {
            CauseRefusal::Missing { verb }
            | CauseRefusal::Unknown { verb, .. }
            | CauseRefusal::NotOperatorOwned { verb, .. }
            | CauseRefusal::NoReplacement { verb }
            | CauseRefusal::RefOnSuperseded { verb }
            | CauseRefusal::NoRef { verb }
            | CauseRefusal::Misplaced { verb, .. }
            | CauseRefusal::Absent { verb, .. } => *verb,
        }
    }
}

impl From<CauseRefusal> for EngineError {
    fn from(refusal: CauseRefusal) -> EngineError {
        EngineError::CauseRefused(Box::new(refusal))
    }
}

/// The journal a citation names, as a refusal says it.
fn journal_named(by_realm: Option<&str>) -> String {
    match by_realm {
        Some(realm) => format!("realm '{realm}'"),
        None => "the workspace journal".to_string(),
    }
}

/// The cause `word` names on `verb`, parsed once where it was typed
/// (decision 0071 ruling 3): no word is [`CauseRefusal::Missing`], and a
/// word outside the twelve endings is [`CauseRefusal::Unknown`], both
/// before any journal is opened. Whether the ending is one an operator
/// may name is the verb's refusal, [`CauseRefusal::NotOperatorOwned`].
pub fn named_cause(verb: Verb, word: Option<&str>) -> Result<Ending, CauseRefusal> {
    let word = word.ok_or(CauseRefusal::Missing { verb })?;
    Ending::named(word).map_err(|unknown| CauseRefusal::Unknown { verb, unknown })
}

/// A cause whose citation fits its word, its cited run found: what the
/// verb writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Shaped<'a> {
    ending: Ending,
    /// The realm and run a `superseded` cause cites.
    by: Option<(Option<&'a str>, &'a str)>,
    by_ref: Option<&'a str>,
}

impl Shaped<'_> {
    /// The cause as `operator-ending.v1` writes it in `args`: `cause`,
    /// with `by` for `superseded` and `ref` for `delivered-elsewhere`.
    pub(super) fn args(&self) -> Value {
        let mut args = json!({"cause": self.ending.word()});
        if let Some((realm, run_id)) = self.by {
            args["by"] = json!({"realm": realm, "run_id": run_id});
        }
        if let Some(by_ref) = self.by_ref {
            args["ref"] = json!(by_ref);
        }
        args
    }

    /// The ending a reading of what the verb writes returns.
    pub(super) fn ended(&self) -> Ended {
        let run = |(realm, run_id): (Option<&str>, &str)| Citation::Run {
            realm: realm.map(str::to_string),
            run_id: run_id.to_string(),
        };
        let cited =
            (self.by.map(run)).or_else(|| self.by_ref.map(|by_ref| Citation::Ref(by_ref.into())));
        Ended {
            ending: self.ending,
            cited,
        }
    }
}

/// `cause` as `verb` may write it: one of ruling 4's six, the citation
/// its word requires and no other, and a `superseded` citation found in
/// `by_journal`, the journal `by_realm` names. It need not be finished,
/// because a re-commission is usually fired in the same minute.
pub(super) fn checked<'a>(
    verb: Verb,
    cause: &Cause<'a>,
    by_journal: &Store,
) -> Result<Shaped<'a>, CauseRefusal> {
    let shaped = shape(verb, cause)?;
    if let Some((by_realm, by_run)) = shaped.by {
        if let Err(error) = by_journal.load(by_run) {
            return Err(CauseRefusal::Absent {
                verb,
                by_run: by_run.to_string(),
                by_realm: by_realm.map(str::to_string),
                error,
            });
        }
    }
    Ok(shaped)
}

/// The citation `cause`'s word requires: `--by-run` for `superseded`,
/// `--by-ref` for `delivered-elsewhere`, and neither for the other four;
/// a citation that belongs to another cause is refused by name.
fn shape<'a>(verb: Verb, cause: &Cause<'a>) -> Result<Shaped<'a>, CauseRefusal> {
    let ending = cause.ending;
    let misplaced = |flags: &[(bool, Flag)]| match flags.iter().find(|(typed, _)| *typed) {
        Some(&(_, flag)) => Err(CauseRefusal::Misplaced {
            verb,
            cause: ending,
            flag,
        }),
        None => Ok(()),
    };
    let (by_run, by_realm, by_ref) = (
        (cause.by_run.is_some(), Flag::ByRun),
        (cause.by_realm.is_some(), Flag::ByRealm),
        (cause.by_ref.is_some(), Flag::ByRef),
    );
    let (by, by_ref) = match ending {
        Ending::Superseded => {
            if cause.by_ref.is_some() {
                return Err(CauseRefusal::RefOnSuperseded { verb });
            }
            let by_run = cause.by_run.ok_or(CauseRefusal::NoReplacement { verb })?;
            (Some((cause.by_realm, by_run)), None)
        }
        Ending::DeliveredElsewhere => {
            misplaced(&[by_run, by_realm])?;
            (
                None,
                Some(cause.by_ref.ok_or(CauseRefusal::NoRef { verb })?),
            )
        }
        Ending::Infrastructure
        | Ending::Exhausted
        | Ending::Withdrawn
        | Ending::OperatorStopped => {
            misplaced(&[by_run, by_realm, by_ref])?;
            (None, None)
        }
        Ending::Shipped
        | Ending::ShippedFlagged
        | Ending::Refused
        | Ending::Failed
        | Ending::Blocked
        | Ending::Unclassified => return Err(CauseRefusal::NotOperatorOwned { verb, ending }),
    };
    Ok(Shaped { ending, by, by_ref })
}

/// `brokkr operator stop` (decision 0064 ruling 4): the stop, with the
/// cause the operator names riding in its `operator/commanded` `args`
/// under `operator-ending.v1`. The cause is checked before anything is
/// written, and a refused cause writes nothing; past that the stop is
/// [`super::operator_command`]'s fenced command, disposed exactly as it
/// always was, its sentence on the acceptance.
pub fn operator_stop(
    store: &mut Store,
    by_journal: &Store,
    run_id: &str,
    named: &Named,
) -> Result<FencedCommandOutcome, EngineError> {
    operator_stop_racing(store, by_journal, run_id, named, |_| {})
}

/// [`operator_stop`], with the window its fence exists for made reachable,
/// as [`super::operator_command_racing`] makes it.
pub(in crate::engine) fn operator_stop_racing(
    store: &mut Store,
    by_journal: &Store,
    run_id: &str,
    named: &Named,
    between: impl FnMut(&mut Store),
) -> Result<FencedCommandOutcome, EngineError> {
    let shaped = checked(Verb::Stop, &named.cause, by_journal)?;
    let read = folded(store, run_id)?;
    let request = Commanded {
        command: OperatorCommand::Stop,
        operator: named.operator,
        reason: named.reason,
        args: shaped.args(),
    };
    disposed(store, run_id, request, read, between).map(Disposition::outcome)
}

#[cfg(test)]
#[path = "ending/tests.rs"]
pub(super) mod tests;
