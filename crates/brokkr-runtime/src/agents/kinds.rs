//! The kinds a seat record names (decision 0075 ruling 2, seat-record v8):
//! a transcript kind and a root session kind are declared by the harness's
//! adapter, not listed in a frozen contract. The schema admits the
//! grammar; the engine's writer, here, refuses a kind the seat's own
//! harness does not declare, and anything at a kind place that is not a
//! declared kind, on every road, whether or not a schema judges it after.
//! A verifier judges a record by the schema alone; a reader acts on a kind
//! only at a seat-record place and only where it was judged, as item 3 of
//! decision 0075's addendum of 2026-11-21 states once.
//!
//! A seat's harness is the built-in driver its own command runs and the
//! adapter its site answers to (the selected candidate's provider, or the
//! adapter the compile pins for an inline model-bearing built-in driver).
//! Each adapter is judged apart: loading another adapter changes no
//! seat's admission (the controller's rulings Q1, R1 and R3 of
//! 2026-11-21).

use std::collections::BTreeMap;

use brokkr_protocol::adapters::AdapterKind;
use brokkr_protocol::{driver_kind, driver_kinds, AttemptOutcome, AttemptReport};
use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::Value;

use super::{Adapter, Adapters, ResumeShape, ResumeStatus};
use crate::argv::Dispatch;
use crate::bundle::built_in_model_driver;

/// The longest kind seat-record v8 admits.
const KIND_LIMIT: usize = 40;

/// The transcript kind that locates nothing, and needs no declaration.
const NONE: &str = "none";

/// Whether `text` is one kind of seat-record v8's grammar,
/// `^[a-z][a-z0-9]*(-[a-z0-9]+)*$`, of at most [`KIND_LIMIT`] bytes. A byte
/// check, held to the contract's pattern by
/// `kinds::tests::the_kind_grammar_is_the_contracts`.
pub(crate) fn kind_word(text: &str) -> bool {
    let lower = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    text.len() <= KIND_LIMIT
        && text.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && (text.split('-')).all(|part| !part.is_empty() && part.bytes().all(lower))
}

/// Whether `kind` is one of v7's closed vocabulary: a kind a built-in
/// driver writes, or `none`. A reader acts on these as it did before v8.
pub(crate) fn v7_kind(kind: &str) -> bool {
    kind == NONE || driver_kinds().contains(&kind)
}

/// Which place of a seat record names a kind. Seat-record v8 defines
/// three: a checkpoint's `transcript` and `root_session`
/// (`$defs/checkpoint`), and a result's `transcript` (`$defs/result`),
/// whether the result stands alone or is nested in a checkpoint beside
/// `authored`. A checkpoint's nested `result` is otherwise any object,
/// which the schema judges for nothing, so the writer judges both of its
/// places there too; a nested `root_session` is refused by the closed
/// `$defs/result` wherever the schema judges one.
///
/// `$defs/record_fields` names a `transcript` too, and no definition
/// refers to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Member {
    Transcript,
    RootSession,
    ResultTranscript,
    ResultRootSession,
}

impl Member {
    fn is_transcript(self) -> bool {
        match self {
            Member::Transcript | Member::ResultTranscript => true,
            Member::RootSession | Member::ResultRootSession => false,
        }
    }
}

impl std::fmt::Display for Member {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Member::Transcript => "transcript",
            Member::RootSession => "root session",
            Member::ResultTranscript => "nested result's transcript",
            Member::ResultRootSession => "nested result's root session",
        })
    }
}

/// An adapter's `record_kinds`: the transcript kind its harness writes,
/// and the root session kind where the harness confirms one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordKinds {
    pub(crate) transcript: String,
    pub(crate) root_session: Option<RootKind>,
}

/// A declared root session kind, and `resume_shape`, the one shape of
/// the adapter's `resume` its seats rejoin by (the controller's ruling
/// R14 of 2026-11-22).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RootKind {
    pub(crate) kind: String,
    pub(crate) resume_shape: String,
}

/// How an adapter's `record_kinds` fails to name the one shape its root
/// session kind rejoins by (the controller's ruling R14 of 2026-11-22).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RootShapeUnmet {
    #[error("declares a 'root_session' kind and no 'resume_shape' it rejoins by")]
    Unnamed,
    #[error("names a 'resume_shape' and declares no 'root_session' kind to rejoin")]
    Rootless,
    #[error("names 'resume_shape' '{0}', which its 'resume' does not declare")]
    Undeclared(String),
}

/// What one seat's harness declares (the controller's rulings R1 to R3 of
/// 2026-11-21): the adapter its site answers to, where it answers to one,
/// and the built-in driver the seat's own command runs, read from the
/// argv the site spawns and never from an adapter. A site that answers to
/// no adapter — an inline raw command, an inline exec site, a site no
/// plan covers — keeps v7's closed vocabulary, which is what it could
/// write before v8: no adapter exists to hold a declaration for it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SeatKinds {
    adapter: Option<AdapterKinds>,
    driver: Option<AdapterKind>,
}

/// What one adapter adds to a seat's harness: its `record_kinds`, and the
/// one shape of its `resume` its root session kind names as the shape its
/// seats rejoin by, kept only where its status is `supported`, the only
/// status a driver's resume gate enables (decision 0056 ruling 5: any
/// other is `unsupported-resume`). No other shape buys an offer (the
/// controller's ruling R14). Never a driver.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AdapterKinds {
    declared: Option<RecordKinds>,
    rejoin: Option<ResumeShape>,
}

impl AdapterKinds {
    fn of(adapter: &Adapter) -> AdapterKinds {
        let root = (adapter.record_kinds.as_ref()).and_then(|kinds| kinds.root_session.as_ref());
        let named = root.and_then(|root| adapter.resume.shape(&root.resume_shape));
        AdapterKinds {
            declared: adapter.record_kinds.clone(),
            rejoin: (named.filter(|shape| shape.status == ResumeStatus::Supported)).cloned(),
        }
    }
}

/// The harness of a site no plan covers, which spawns no driver of a
/// harness and answers to no adapter: v7's closed vocabulary.
pub(crate) static UNDECLARED: SeatKinds = SeatKinds {
    adapter: None,
    driver: None,
};

/// What stands at one site, in the words the engine writes into its
/// driver's input as `boundary` and `hands` (decision 0046 ruling 3,
/// decision 0043) and the driver's resume gate matches against the shape
/// it rejoins under. A site whose boundary or hands the engine cannot
/// state has no standing, and is offered no root of a new kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Standing {
    pub(crate) boundary: &'static str,
    pub(crate) hands: &'static str,
}

impl SeatKinds {
    /// Whether this harness may write `kind` at `member`: the check every
    /// write takes, and the one a stamped offer's transcript takes too
    /// (the controller's ruling R32).
    pub(crate) fn declares(&self, member: Member, kind: &str) -> bool {
        if member.is_transcript() && kind == NONE {
            return true;
        }
        let Some(adapter) = &self.adapter else {
            return driver_kinds().contains(&kind);
        };
        let declared = adapter.declared.as_ref();
        let named = declared.and_then(|declared| match member.is_transcript() {
            true => Some(declared.transcript.as_str()),
            false => (declared.root_session.as_ref()).map(|root| root.kind.as_str()),
        });
        named == Some(kind) || self.driver.and_then(driver_kind) == Some(kind)
    }

    /// Whether this harness's adapter declares its kinds (`record_kinds`),
    /// so its only road to an offer is its declared root session kind and
    /// never decision 0030's flat session id (the controller's ruling R5).
    pub(crate) fn declares_its_kinds(&self) -> bool {
        (self.adapter.as_ref()).is_some_and(|adapter| adapter.declared.is_some())
    }

    /// Whether a work site of this harness standing at `standing` may be
    /// offered a root of `kind` (the controller's rulings R4 and R11 of
    /// 2026-11-21): one of the three kinds a built-in driver writes, as
    /// today; otherwise only the root session kind this harness's adapter
    /// declares, where the one shape it names as the shape it rejoins by
    /// (`resume_shape`) is `supported` and covers the site, so the engine
    /// offers no wider than the driver's resume gate, asked about that
    /// shape, would rejoin (the controller's ruling R14 of 2026-11-22).
    /// Unknown standing declines.
    pub(crate) fn rejoins(&self, kind: &str, standing: Option<Standing>) -> bool {
        if driver_kinds().contains(&kind) {
            return true;
        }
        let (Some(adapter), Some(standing)) = (&self.adapter, standing) else {
            return false;
        };
        let declared = adapter.declared.as_ref();
        let root = declared.and_then(|kinds| kinds.root_session.as_ref());
        root.is_some_and(|root| root.kind == kind)
            && (adapter.rejoin.as_ref())
                .is_some_and(|shape| shape.covers("work", standing.boundary, standing.hands))
    }

    /// `record`, which seat `seat` wrote, refused unless each of its kind
    /// places is absent or names a kind this harness declares (the
    /// controller's ruling R13 of 2026-11-22). A place outside the
    /// grammar — a string the grammar refuses, a value that is no string,
    /// a place that is not the object the grammar gives — is refused here
    /// on every road, whether or not a schema judges that road after, and
    /// so is a record that is not an object, whose places cannot be read.
    pub(crate) fn admit(&self, record: &Value, seat: &str) -> Result<(), UndeclaredKind> {
        let seat = || seat.to_string();
        let read = record.is_object().then(|| Named::deserialize(record).ok());
        let Some(named) = read.flatten() else {
            return Err(UndeclaredKind::Unread { seat: seat() });
        };
        let nested = match named.result {
            Nested::Members(nested) => nested,
            Nested::Sequence(_) | Nested::Other(_) | Nested::Absent => Members::default(),
        };
        // A nested result's root session is no place seat-record v8
        // defines: the closed result of an authored row refuses it, and
        // the open object of any other row is judged for nothing. The
        // check stays, and fails closed (the controller's ruling R30).
        let places = [
            (Member::Transcript, named.transcript),
            (Member::RootSession, named.root_session),
            (Member::ResultTranscript, nested.transcript),
            (Member::ResultRootSession, nested.root_session),
        ];
        for (member, place) in places {
            let kind = match place.recorded() {
                None => continue,
                Some(Recorded::Word(word)) if self.declares(member, &word) => continue,
                Some(kind) => kind,
            };
            let seat = seat();
            return Err(UndeclaredKind::Named { seat, member, kind });
        }
        Ok(())
    }

    /// `report` with its successful result refused, as a refused
    /// checkpoint's is (decision 0034 ruling 6), when the result names a
    /// kind this harness does not declare. A refusal already in place
    /// stands, and keeps its words. Either way the received result is
    /// judged too, and one naming an undeclared kind is received as the
    /// kind refusal in its place, so no evidence an unresolved cleanup
    /// journals carries the kind (the controller's ruling R10 of
    /// 2026-11-21). A result naming declared kinds is received as it came.
    pub(crate) fn refused(&self, seat: &str, mut report: AttemptReport) -> AttemptReport {
        if let AttemptOutcome::Succeeded { result } = &report.outcome {
            if let Err(refusal) = self.admit(result, seat) {
                let error = refusal.to_string();
                let refusal = AttemptOutcome::Failed { error };
                report.refused.get_or_insert_with(|| refusal.clone());
                report.outcome = refusal;
            }
        }
        report
    }
}

/// The built-in driver `argv` dispatches, if it names one.
fn built_in(argv: &[String]) -> Option<AdapterKind> {
    match Dispatch::of(argv) {
        Dispatch::Driver { kind, .. } => AdapterKind::parse(kind),
        Dispatch::Raw => None,
    }
}

/// What each loaded adapter adds to a seat's harness, by provider. The
/// built-in drivers' kinds are declared by the drivers that write them
/// ([`driver_kind`]) and are never held in an adapter, so no shipped
/// adapter carries them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeclaredKinds(BTreeMap<String, AdapterKinds>);

impl DeclaredKinds {
    /// Every adapter's declarations; nothing when no library was loaded.
    pub(crate) fn of(adapters: Option<&Adapters>) -> DeclaredKinds {
        let adapters = adapters.into_iter().flat_map(Adapters::providers);
        DeclaredKinds(
            adapters
                .map(|adapter| (adapter.provider.clone(), AdapterKinds::of(adapter)))
                .collect(),
        )
    }

    /// The harness of a site that spawns `argv`, for `provider` where a
    /// candidate was selected (the controller's rulings R1 and R3): the
    /// built-in driver `argv` runs, beside the selected provider's
    /// adapter, or, at an inline site, the adapter the compile pins for
    /// that site's model-bearing built-in driver ([`built_in_model_driver`])
    /// and no other. Inline exec and an inline raw command answer to none.
    pub(crate) fn seat(&self, provider: Option<&str>, argv: &[String]) -> SeatKinds {
        let pinned = provider.or_else(|| built_in_model_driver(argv));
        SeatKinds {
            adapter: pinned.and_then(|provider| self.0.get(provider)).cloned(),
            driver: built_in(argv),
        }
    }
}

/// The kind places of one record, read once as types. A place that does
/// not read as a kind is read as what it is, refused by [`Place::recorded`],
/// and never keeps its siblings from being read.
#[derive(Deserialize)]
struct Named {
    #[serde(default)]
    transcript: Place,
    #[serde(default)]
    root_session: Place,
    #[serde(default)]
    result: Nested,
}

/// A record's nested `result`: an object whose kind places are read, or
/// anything else, which names no kind place. An array is held apart so it
/// is never read as an object's members by position.
#[derive(Default, Deserialize)]
#[serde(untagged)]
enum Nested {
    Sequence(Sequence),
    Members(Members),
    Other(IgnoredAny),
    #[default]
    Absent,
}

/// Any array, read as nothing but that it is one.
struct Sequence;

impl<'de> Deserialize<'de> for Sequence {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<IgnoredAny>::deserialize(deserializer).map(|_| Sequence)
    }
}

#[derive(Default, Deserialize)]
struct Members {
    #[serde(default)]
    transcript: Place,
    #[serde(default)]
    root_session: Place,
}

/// One kind place as it read: absent, an object naming a string `kind`,
/// or anything else (an array first, so it is never read as an object by
/// position). `Absent` is only ever the default of a missing place:
/// `Other` reads every present value the variants before it do not,
/// `null` included.
#[derive(Default, Deserialize)]
#[serde(untagged)]
enum Place {
    Sequence(Sequence),
    Kinded {
        kind: String,
    },
    Other(IgnoredAny),
    #[default]
    Absent,
}

impl Place {
    /// What this place records: nothing where it is absent, its kind
    /// where that is one word of v8's grammar, and otherwise a kind
    /// outside the grammar, whatever it holds.
    fn recorded(self) -> Option<Recorded> {
        match self {
            Place::Absent => None,
            Place::Kinded { kind } if kind_word(&kind) => Some(Recorded::Word(kind)),
            Place::Kinded { .. } | Place::Sequence(_) | Place::Other(_) => Some(Recorded::Outside),
        }
    }
}

/// A kind a record names, as a refusal may echo it: one word of v8's
/// grammar, quoted, or a place outside the grammar, which is never
/// echoed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Recorded {
    Word(String),
    Outside,
}

impl std::fmt::Display for Recorded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Recorded::Word(word) => write!(f, "'{word}'"),
            Recorded::Outside => f.write_str("outside seat-record v8's grammar"),
        }
    }
}

/// A record that names a kind the seat's harness does not declare, or
/// whose kind places cannot be read, refused, naming the seat that wrote
/// it (decision 0075 ruling 2; the controller's ruling R13 of 2026-11-22).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum UndeclaredKind {
    #[error(
        "seat '{seat}' recorded {member} kind {kind}, which the seat's harness does not \
         declare (decision 0075 ruling 2)"
    )]
    Named {
        seat: String,
        member: Member,
        kind: Recorded,
    },
    /// A record that is not an object: its root is the place, outside the
    /// grammar, in [`Recorded::Outside`]'s words (the controller's ruling
    /// R21 of 2026-11-22).
    #[error(
        "seat '{seat}' recorded its record's root {}, so the kinds it names cannot be read \
         (decision 0075 ruling 2)",
        Recorded::Outside
    )]
    Unread { seat: String },
}

#[cfg(test)]
mod tests;
