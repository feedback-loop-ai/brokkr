//! The kinds a seat record names (decision 0075 ruling 2, seat-record v8):
//! a transcript kind and a root session kind are declared by the harness's
//! adapter, not listed in a frozen contract. The schema admits the
//! grammar; the engine's writer, here, refuses a kind no loaded adapter
//! declares. Readers judge a record by the schema alone.

use std::collections::BTreeSet;

use brokkr_protocol::{AttemptOutcome, AttemptReport};
use serde::Deserialize;
use serde_json::Value;

use super::Adapters;

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

/// Which member of a seat record names a kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Member {
    Transcript,
    RootSession,
}

impl std::fmt::Display for Member {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Member::Transcript => "transcript",
            Member::RootSession => "root session",
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

/// The kinds the loaded adapters declare. The built-in drivers' kinds are
/// declared by the drivers that write them ([`brokkr_protocol::driver_kinds`])
/// and are never held here, so no shipped adapter carries them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeclaredKinds {
    transcript: BTreeSet<String>,
    root_session: BTreeSet<String>,
}

/// The two kind members of one record, read once as a type. A record whose
/// members do not read is the schema's to refuse, never this check's.
#[derive(Deserialize)]
struct Named {
    transcript: Option<Kinded>,
    root_session: Option<Kinded>,
}

#[derive(Deserialize)]
struct Kinded {
    kind: String,
}

/// A record whose kind no loaded adapter and no built-in driver declares,
/// refused at append, naming the seat that wrote it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "seat '{seat}' recorded {member} kind '{kind}', which no loaded adapter declares \
     (decision 0075 ruling 2)"
)]
pub(crate) struct UndeclaredKind {
    pub(crate) seat: String,
    pub(crate) member: Member,
    pub(crate) kind: String,
}

impl DeclaredKinds {
    /// Every kind `adapters` declare; nothing when no library was loaded.
    pub(crate) fn of(adapters: Option<&Adapters>) -> DeclaredKinds {
        let mut declared = DeclaredKinds::default();
        let kinds = adapters.into_iter().flat_map(Adapters::providers);
        for kinds in kinds.filter_map(|adapter| adapter.record_kinds.as_ref()) {
            declared.transcript.insert(kinds.transcript.clone());
            declared.root_session.extend(kinds.root_session.clone());
        }
        declared
    }

    fn declares(&self, member: Member, kind: &str) -> bool {
        let held = match member {
            Member::Transcript => kind == NONE || self.transcript.contains(kind),
            Member::RootSession => self.root_session.contains(kind),
        };
        held || brokkr_protocol::driver_kinds().contains(&kind)
    }

    /// `record`, which seat `seat` wrote, refused when it names a kind in
    /// v8's grammar that nothing declares. A kind outside the grammar is
    /// left to the schema, which refuses it without echoing it.
    pub(crate) fn admit(&self, record: &Value, seat: &str) -> Result<(), UndeclaredKind> {
        let Ok(named) = Named::deserialize(record) else {
            return Ok(());
        };
        let members = [
            (Member::Transcript, named.transcript),
            (Member::RootSession, named.root_session),
        ];
        for (member, kinded) in members {
            let Some(Kinded { kind }) = kinded else {
                continue;
            };
            if kind_word(&kind) && !self.declares(member, &kind) {
                let seat = seat.to_string();
                return Err(UndeclaredKind { seat, member, kind });
            }
        }
        Ok(())
    }

    /// `report` with its successful result refused, as a refused
    /// checkpoint's is (decision 0034 ruling 6), when the result names a
    /// kind nothing declares. A report already refused stands.
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

#[cfg(test)]
mod tests;
