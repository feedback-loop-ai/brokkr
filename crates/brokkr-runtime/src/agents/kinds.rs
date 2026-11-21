//! The kinds a seat record names (decision 0075 ruling 2, seat-record v8):
//! a transcript kind and a root session kind are declared by the harness's
//! adapter, not listed in a frozen contract. The schema admits the
//! grammar; the engine's writer, here, refuses a kind the seat's own
//! harness does not declare. Readers judge a record by the schema alone.
//!
//! A seat's harness is its adapter (the selected candidate's provider, or
//! the adapter an inline built-in driver names) and the built-in driver
//! that adapter runs. Each adapter is judged apart: loading another
//! adapter changes no seat's admission (the controller's ruling Q1 of
//! 2026-11-21).

use std::collections::BTreeMap;

use brokkr_protocol::adapters::AdapterKind;
use brokkr_protocol::{driver_kind, driver_kinds, AttemptOutcome, AttemptReport};
use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::Value;

use super::{Adapter, Adapters, ResumeStatus};
use crate::argv::Dispatch;

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

/// Which place of a seat record names a kind. These are every place
/// seat-record v8 admits a `transcript` or a `root_session`:
///
/// - a checkpoint's `transcript` and `root_session` (`$defs/checkpoint`),
///   and a successful result's `transcript` (`$defs/result`), the top
///   level of the record either way;
/// - a checkpoint's nested `result`: `$defs/result` beside `authored`,
///   which admits a `transcript`, and otherwise any object, which admits
///   both.
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

    /// Whether the schema judges this place's kind by v8's grammar. The
    /// nested result beside no `authored` is any object, so there the
    /// writer is the only judge, and refuses a kind outside the grammar
    /// too.
    fn schema_judged(self) -> bool {
        match self {
            Member::Transcript | Member::RootSession => true,
            Member::ResultTranscript | Member::ResultRootSession => false,
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
    pub(crate) root_session: Option<String>,
}

/// What one seat's harness declares: its adapter's `record_kinds`, the
/// built-in driver it runs, and whether its adapter declares a `resume`
/// shape for a work site that is not `unsupported`. A seat with neither a
/// declaration nor a built-in driver — a raw command no adapter declares
/// kinds for — keeps v7's closed vocabulary, which is what it could write
/// before v8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SeatKinds {
    declared: Option<RecordKinds>,
    driver: Option<AdapterKind>,
    rejoins_work: bool,
}

/// The harness of a site no plan covers, which spawns no driver of a
/// harness: v7's closed vocabulary.
pub(crate) static UNDECLARED: SeatKinds = SeatKinds {
    declared: None,
    driver: None,
    rejoins_work: false,
};

impl Default for SeatKinds {
    fn default() -> Self {
        UNDECLARED.clone()
    }
}

impl SeatKinds {
    fn of(adapter: &Adapter) -> SeatKinds {
        let shapes = adapter.resume.0.values();
        SeatKinds {
            declared: adapter.record_kinds.clone(),
            driver: built_in(&adapter.driver),
            rejoins_work: shapes
                .filter(|shape| shape.status != ResumeStatus::Unsupported)
                .any(|shape| shape.classes.iter().any(|class| class == "work")),
        }
    }

    fn declares(&self, member: Member, kind: &str) -> bool {
        if member.is_transcript() && kind == NONE {
            return true;
        }
        let Some(declared) = &self.declared else {
            return match self.driver {
                Some(driver) => driver_kind(driver) == Some(kind),
                None => driver_kinds().contains(&kind),
            };
        };
        let named = match member.is_transcript() {
            true => Some(declared.transcript.as_str()),
            false => declared.root_session.as_deref(),
        };
        named == Some(kind) || self.driver.and_then(driver_kind) == Some(kind)
    }

    /// Whether a work site of this harness may be offered a root of
    /// `kind` (the controller's ruling Q3 of 2026-11-21): one of the three
    /// kinds a built-in driver writes, as today; otherwise only the root
    /// session kind this harness's adapter declares, where that adapter
    /// declares a resume for a work site that is not `unsupported`.
    pub(crate) fn rejoins(&self, kind: &str) -> bool {
        if driver_kinds().contains(&kind) {
            return true;
        }
        let declared = self.declared.as_ref();
        self.rejoins_work && declared.and_then(|kinds| kinds.root_session.as_deref()) == Some(kind)
    }

    /// `record`, which seat `seat` wrote, refused when one of its places
    /// names a kind this harness does not declare. A kind outside the
    /// grammar at a place the schema judges is left to the schema, which
    /// refuses it without echoing it.
    pub(crate) fn admit(&self, record: &Value, seat: &str) -> Result<(), UndeclaredKind> {
        let Ok(named) = Named::deserialize(record) else {
            return Ok(());
        };
        let nested = match named.result {
            Some(Read::Read(nested)) => nested,
            _ => Members::default(),
        };
        let places = [
            (Member::Transcript, named.transcript),
            (Member::RootSession, named.root_session),
            (Member::ResultTranscript, nested.transcript),
            (Member::ResultRootSession, nested.root_session),
        ];
        for (member, kinded) in places {
            let Some(Read::Read(Kinded { kind })) = kinded else {
                continue;
            };
            let left = member.schema_judged() && !kind_word(&kind);
            if !left && !self.declares(member, &kind) {
                let seat = seat.to_string();
                return Err(UndeclaredKind { seat, member, kind });
            }
        }
        Ok(())
    }

    /// `report` with its successful result refused, as a refused
    /// checkpoint's is (decision 0034 ruling 6), when the result names a
    /// kind this harness does not declare. A report already refused stands.
    pub(crate) fn refused(&self, seat: &str, mut report: AttemptReport) -> AttemptReport {
        if let (None, AttemptOutcome::Succeeded { result }) = (&report.refused, &report.outcome) {
            if let Err(refusal) = self.admit(result, seat) {
                let error = refusal.to_string();
                report.refused = Some(AttemptOutcome::Failed { error });
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

/// Each loaded adapter's harness, by provider. The built-in drivers' kinds
/// are declared by the drivers that write them ([`driver_kind`]) and are
/// never held in an adapter, so no shipped adapter carries them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeclaredKinds(BTreeMap<String, SeatKinds>);

impl DeclaredKinds {
    /// Every adapter's harness; nothing when no library was loaded.
    pub(crate) fn of(adapters: Option<&Adapters>) -> DeclaredKinds {
        let adapters = adapters.into_iter().flat_map(Adapters::providers);
        DeclaredKinds(
            adapters
                .map(|adapter| (adapter.provider.clone(), SeatKinds::of(adapter)))
                .collect(),
        )
    }

    /// The harness of a site that spawns `argv`, for `provider` where a
    /// candidate was selected: that provider's adapter, else the adapter
    /// an inline built-in driver names, else the built-in driver alone.
    pub(crate) fn seat(&self, provider: Option<&str>, argv: &[String]) -> SeatKinds {
        let driver = built_in(argv);
        let named = provider.or_else(|| match Dispatch::of(argv) {
            Dispatch::Driver { kind, .. } => driver.map(|_| kind),
            Dispatch::Raw => None,
        });
        match named.and_then(|provider| self.0.get(provider)) {
            Some(kinds) => kinds.clone(),
            None => SeatKinds {
                driver,
                ..SeatKinds::default()
            },
        }
    }
}

/// The kind members of one record, read once as types. A member that does
/// not read is the schema's to refuse, never this check's, and it never
/// keeps its siblings from being read.
#[derive(Deserialize)]
struct Named {
    transcript: Option<Read<Kinded>>,
    root_session: Option<Read<Kinded>>,
    result: Option<Read<Members>>,
}

#[derive(Default, Deserialize)]
struct Members {
    transcript: Option<Read<Kinded>>,
    root_session: Option<Read<Kinded>>,
}

/// A member as it read: its type, or anything else.
#[derive(Deserialize)]
#[serde(untagged)]
enum Read<T> {
    Read(T),
    Other(IgnoredAny),
}

#[derive(Deserialize)]
struct Kinded {
    kind: String,
}

/// A record whose kind the seat's harness does not declare, refused at
/// append, naming the seat that wrote it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "seat '{seat}' recorded {member} kind {}, which the seat's harness does not declare \
     (decision 0075 ruling 2)",
    shown(.kind)
)]
pub(crate) struct UndeclaredKind {
    pub(crate) seat: String,
    pub(crate) member: Member,
    pub(crate) kind: String,
}

/// A kind as a refusal may echo it: quoted where it is one word of v8's
/// grammar, and never otherwise.
fn shown(kind: &str) -> String {
    match kind_word(kind) {
        true => format!("'{kind}'"),
        false => "outside seat-record v8's grammar".to_string(),
    }
}

#[cfg(test)]
mod tests;
