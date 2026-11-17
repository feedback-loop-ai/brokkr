//! A park's cause, typed and closed (decision 0106 ruling 1): which of the
//! four things ended the run's movement, and the facts that say so. It
//! rides as an optional object on `run/parked` at `event_schema: 1`,
//! published as `park-cause/v1`. `fold` never reads it; the summary, the
//! view and every readout read it here once, and an absent cause reads
//! the reader's fifth word, `unknown`, which is never written.
//!
//! A present object that does not read refuses by name, as a typed
//! [`ParkCauseError`]: the edge is [`parse`], a hand-written reader that holds
//! each kind's closed shape and each form the contract states field by field
//! ([`forms`]), so a refusal is a variant a reader matches, not serde's text
//! (decision 0071 rulings 3 and 8). `Serialize` is derived, and the one
//! `Deserialize` is the reader's own, which calls [`parse`].

use serde::{Deserialize, Serialize};

use crate::envelope::EventEnvelope;
use crate::summary::PARK_REASON_BYTES;
use crate::ProviderRefusal;

mod error;
mod forms;
mod parse;
mod reading;
mod record;
mod until;

pub use error::{FormError, ParkCauseError};
pub(crate) use forms::RULE_ID_MAX;
pub use reading::CauseRead;

/// The four words a writer states for why a run parked (decision 0106
/// ruling 1). `unknown` is a reader's word, not a variant: it is what an
/// absent `cause` reads, and no writer may journal it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ParkCause {
    /// Policy or engine refused: a rule park, an unmatched result, a
    /// schema refusal, a missing seat default, or a moved gate head.
    Ruled {
        /// Present exactly where a policy rule matched.
        #[serde(skip_serializing_if = "Option::is_none")]
        rule_id: Option<String>,
        detail: String,
    },
    /// 0006's attempt cap was reached.
    AttemptCap {
        failure_class: FailureClass,
        detail: String,
    },
    /// An indeterminate end, an engine restart in flight, or contention.
    Infrastructure { retryable: bool, detail: String },
    /// A typed rate-limit refusal parked below the cap.
    ProviderLimit {
        provider: String,
        detail: String,
        refusal: ProviderLimitRefusal,
    },
}

/// One `run/parked` write: the reason an operator reads and the cause a
/// script reads, stated together so a park cannot be written with one and
/// not the other (decision 0106 ruling 1). Every writer builds this and
/// nothing else, and [`Park::payload`] is the one place a `run/parked`
/// payload is built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Park {
    reason: String,
    cause: ParkCause,
}

impl Park {
    /// A park with its own cause, whose sentence is bounded here, where it
    /// is built, to [`PARK_REASON_BYTES`]: the bound the summary row keeps
    /// the reason within. The reason stays whole in the journal, so a cause
    /// is never longer than a row can hold it, and every readout paints the
    /// same bytes of it.
    pub fn new(reason: impl Into<String>, cause: ParkCause) -> Park {
        Park {
            reason: reason.into(),
            cause: cause.bounded(PARK_REASON_BYTES),
        }
    }

    /// A policy or engine refusal that matched no rule, its detail the
    /// reason itself, bounded as [`Park::new`] bounds it.
    pub fn ruled(reason: impl Into<String>) -> Park {
        let reason = reason.into();
        let cause = ParkCause::ruled(None, reason.clone());
        Park::new(reason, cause)
    }

    /// The reason, for an operator. A park prints as its reason.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// The cause, for a script.
    pub fn cause(&self) -> &ParkCause {
        &self.cause
    }

    /// The `run/parked` payload, with `evidence` beside the reason and the
    /// cause (the moved head's observations, or an empty object).
    pub fn payload(&self, evidence: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "reason": self.reason,
            "evidence": evidence,
            "cause": self.cause,
        })
    }
}

impl std::fmt::Display for Park {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reason)
    }
}

/// The last attempt's failure class at 0006's cap (decision 0106 ruling
/// 1). Closed, and derived from typed journal facts, never from the
/// reason's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FailureClass {
    /// A typed rate-limit refusal.
    ProviderLimit,
    /// A typed refusal that is not a rate limit.
    ProviderRefusal,
    /// A pre-session failure to start (decision 0053).
    StartFailure,
    /// A determinate attempt failure.
    Determinate,
}

/// One `provider-limit` cause's refusal variant (decision 0106 ruling 1):
/// one per 0103 cause, each holding exactly the fields it has. The
/// provider's words stay in the park's reason and in the typed `refused`
/// record; this object never copies them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProviderLimitRefusal {
    /// The provider's route could not be resolved, so no route-wide hold
    /// was recorded.
    RouteUnresolved,
    /// The host file could not be read or is not a valid host file.
    HostUnreadable { route: String },
    /// An earlier wait of the same effect did not read.
    EpisodeUnreadable { route: String },
    /// The wait is past the declared maximum.
    PastMaximum {
        route: String,
        #[serde(with = "until")]
        until: time::OffsetDateTime,
        seconds: u64,
        max_seconds: u64,
    },
    /// The journal cannot spell the wait's end.
    NoEnd {
        route: String,
        seconds: u64,
        max_seconds: u64,
    },
}

impl ParkCause {
    /// A policy or engine refusal, naming the rule where one matched.
    pub fn ruled(rule_id: Option<String>, detail: impl Into<String>) -> ParkCause {
        ParkCause::Ruled {
            rule_id,
            detail: detail.into(),
        }
    }

    /// 0006's cap, with the last attempt's typed failure class.
    pub fn attempt_cap(failure_class: FailureClass, detail: impl Into<String>) -> ParkCause {
        ParkCause::AttemptCap {
            failure_class,
            detail: detail.into(),
        }
    }

    /// An indeterminate end, a restart in flight or contention. `retryable`
    /// is true only on decision 0106 ruling 4's positive non-execution
    /// evidence.
    pub fn infrastructure(retryable: bool, detail: impl Into<String>) -> ParkCause {
        ParkCause::Infrastructure {
            retryable,
            detail: detail.into(),
        }
    }

    /// A provider's refusal in its one typed variant, the detail the
    /// engine's own sentence for it ([`ProviderLimitRefusal::detail`]).
    pub fn refusal(provider: impl Into<String>, refusal: ProviderLimitRefusal) -> ParkCause {
        let provider = provider.into();
        ParkCause::ProviderLimit {
            detail: refusal.detail(&provider),
            provider,
            refusal,
        }
    }

    /// A typed rate-limit refusal parked below the cap, its one 0103
    /// variant carrying exactly the fields that record has. `until` is the
    /// computed end a `past-maximum` refusal records in its wait. The
    /// detail is the engine's own sentence for the variant: the provider's
    /// words stay in the park's reason and the typed `refused` record.
    ///
    /// A record that contradicts its own cause (a public value built by
    /// hand: a resolved refusal with no route, a computed one with no
    /// length) is not a panic: the park is the engine's own refusal,
    /// `ruled` and never retried, naming what the record lacks.
    pub fn provider_limit(
        refused: &ProviderRefusal,
        until: Option<time::OffsetDateTime>,
    ) -> ParkCause {
        match record::variant(refused, until) {
            Ok(refusal) => ParkCause::refusal(refused.provider.clone(), refusal),
            Err(error) => ParkCause::ruled(
                None,
                format!("a typed refusal record that contradicts itself: {error}"),
            ),
        }
    }

    /// This cause with its sentence cut at a character boundary to at most
    /// `bytes` bytes, the cut the summary row's reason takes. A writer cuts
    /// it as it builds the park ([`Park::new`]) and the reader cuts what a
    /// journal holds ([`ParkCause::read`]), so the row never cuts it again.
    /// Every other field is bounded by the schema's forms.
    pub(crate) fn bounded(self, bytes: usize) -> ParkCause {
        let cut = |detail: String| crate::summary::cut(&detail, bytes).to_string();
        match self {
            ParkCause::Ruled { rule_id, detail } => ParkCause::Ruled {
                rule_id,
                detail: cut(detail),
            },
            ParkCause::AttemptCap {
                failure_class,
                detail,
            } => ParkCause::AttemptCap {
                failure_class,
                detail: cut(detail),
            },
            ParkCause::Infrastructure { retryable, detail } => ParkCause::Infrastructure {
                retryable,
                detail: cut(detail),
            },
            ParkCause::ProviderLimit {
                provider,
                detail,
                refusal,
            } => ParkCause::ProviderLimit {
                provider,
                detail: cut(detail),
                refusal,
            },
        }
    }

    /// The cause's own sentence, whichever word it is.
    fn detail(&self) -> &str {
        match self {
            ParkCause::Ruled { detail, .. }
            | ParkCause::AttemptCap { detail, .. }
            | ParkCause::Infrastructure { detail, .. }
            | ParkCause::ProviderLimit { detail, .. } => detail,
        }
    }

    /// The one line every readout paints: the word, what it names, and the
    /// detail. A rule id, a failure class, retryability and the provider's
    /// refusal kind are the typed facts, not a second parse of the prose.
    pub fn line(&self) -> String {
        match self {
            ParkCause::Ruled { rule_id, detail } => match rule_id {
                Some(rule) => format!("ruled {rule}: {detail}"),
                None => format!("ruled: {detail}"),
            },
            ParkCause::AttemptCap {
                failure_class,
                detail,
            } => format!("attempt-cap ({}): {detail}", failure_class.word()),
            ParkCause::Infrastructure { retryable, detail } => format!(
                "infrastructure ({}): {detail}",
                if *retryable {
                    "retryable"
                } else {
                    "not retryable"
                }
            ),
            ParkCause::ProviderLimit {
                provider,
                detail,
                refusal,
            } => format!("provider-limit ({provider}, {}): {detail}", refusal.word()),
        }
    }

    /// Read `event`'s optional `cause`: absent is no cause, a present one
    /// that does not read is refused by name. Its sentence is within
    /// [`PARK_REASON_BYTES`], as a writer states it, whatever a journal
    /// holds, so a paged row, the whole read and the kept path agree.
    pub fn read(event: &EventEnvelope) -> Result<Option<ParkCause>, ParkCauseError> {
        match event.payload.get("cause") {
            None => Ok(None),
            Some(value) => parse::cause(value).map(|cause| Some(cause.bounded(PARK_REASON_BYTES))),
        }
    }
}

impl FailureClass {
    /// The class a `failure_class` word names.
    fn parse(word: &str) -> Result<FailureClass, ParkCauseError> {
        [
            FailureClass::ProviderLimit,
            FailureClass::ProviderRefusal,
            FailureClass::StartFailure,
            FailureClass::Determinate,
        ]
        .into_iter()
        .find(|class| class.word() == word)
        .ok_or_else(|| ParkCauseError::UnknownFailureClass(word.to_string()))
    }

    /// The word the cause line paints for this class.
    pub fn word(self) -> &'static str {
        match self {
            FailureClass::ProviderLimit => "provider-limit",
            FailureClass::ProviderRefusal => "provider-refusal",
            FailureClass::StartFailure => "start-failure",
            FailureClass::Determinate => "determinate",
        }
    }
}

impl ProviderLimitRefusal {
    /// The refusal's own word, one per 0103 cause.
    pub fn word(&self) -> &'static str {
        match self {
            ProviderLimitRefusal::RouteUnresolved => "route-unresolved",
            ProviderLimitRefusal::HostUnreadable { .. } => "host-unreadable",
            ProviderLimitRefusal::EpisodeUnreadable { .. } => "episode-unreadable",
            ProviderLimitRefusal::PastMaximum { .. } => "past-maximum",
            ProviderLimitRefusal::NoEnd { .. } => "no-end",
        }
    }

    /// The engine's own sentence for this refusal, built from its typed
    /// fields and never from the provider's words, which stay in the park's
    /// reason and the typed `refused` record (decision 0106 ruling 1).
    pub fn detail(&self, provider: &str) -> String {
        match self {
            ProviderLimitRefusal::RouteUnresolved => format!(
                "{provider}'s route could not be resolved, so no route-wide hold was recorded"
            ),
            ProviderLimitRefusal::HostUnreadable { route } => {
                format!("{provider}/{route}: the host file's cool-down bounds could not be read")
            }
            ProviderLimitRefusal::EpisodeUnreadable { route } => {
                format!("{provider}/{route}: an earlier wait of this effect does not read")
            }
            ProviderLimitRefusal::PastMaximum {
                route,
                until,
                seconds,
                max_seconds,
            } => format!(
                "{provider}/{route}: a {seconds}s wait until {} goes past the declared \
                 {max_seconds}s maximum, alone or with the episode's earlier rounds",
                until::spell(until)
            ),
            ProviderLimitRefusal::NoEnd {
                route,
                seconds,
                max_seconds,
            } => format!(
                "{provider}/{route}: a {seconds}s wait has no end the journal can spell, \
                 against the declared {max_seconds}s maximum"
            ),
        }
    }
}

/// A cause reads through [`parse::cause`], the contract's closed shapes held
/// field by field; a value that does not read is a typed [`ParkCauseError`],
/// whose words are what serde reports for the same refusal.
impl<'de> Deserialize<'de> for ParkCause {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        parse::cause(&value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
