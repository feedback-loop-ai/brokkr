//! The steps of [`resolve_reference`](super::resolve_reference) (#288,
//! slice H): the agent's name, the hands law on a mapped chain before its
//! capability gaps resolve, the resolved chain as the site's candidates,
//! and the charter the site will be told, bound to its library's read.
//! `resolve_reference` reads as their sequence.

use std::collections::BTreeMap;
use std::path::Path;

use brokkr_core::canonical::sha256_bytes;
use brokkr_core::realms::Boundary;
use serde_json::Value;

use super::opened::Opened;
use super::{
    enforce_model_policy, expand_command, expand_lowering, library_refusal, lint_secret_refs,
    missing_clause, owned_input, refuse_permission_pins, CharterOwner, CharterPin, CompileError,
    InputFault, LibraryRead, SiteFacts, SiteLaw,
};
use crate::agents::{Adapters, Candidate, Report, Resolution};
use crate::argv::Executable;

/// One agent reference as its site wrote it: the engine executable and the
/// directory of the layer that declared the site, which its `{brokkr}` and
/// `./` tokens expand to, the site and its declaration, the secrets it
/// binds, the boundary the bundle compiles under, and the agent it names.
#[derive(Clone, Copy)]
pub(super) struct ReferenceAt<'s> {
    pub(super) engine: &'s Executable,
    pub(super) dir: &'s Path,
    pub(super) what: &'s str,
    pub(super) raw: &'s Value,
    pub(super) secrets: &'s [String],
    pub(super) boundary: Boundary,
    pub(super) name: &'s str,
}

/// The site's `agent`: a non-empty string.
pub(super) fn agent_name<'v>(what: &str, raw: &'v Value) -> Result<&'v str, CompileError> {
    raw.get("agent")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            CompileError::Invalid(format!("seat '{what}' agent must be a non-empty string"))
        })
}

/// The hands law and the route policy on the reported chain, where the
/// agent declares hands and every entry is mapped.
pub(super) fn judge_mapped_hands(
    at: ReferenceAt<'_>,
    report: &Report,
    agents: &mut Opened<'_>,
    sites: &mut BTreeMap<String, SiteFacts>,
) -> Result<(), CompileError> {
    let ReferenceAt {
        dir,
        what,
        raw,
        secrets,
        boundary,
        name,
        ..
    } = at;
    // D33: judge every mapped hands link before resolving capability gaps.
    // In particular dsh/LaneTally earn the tier refusal under namespace,
    // and the missing harness.gate refusal under harness. An unmapped
    // chain still receives the resolver's precise unmapped-model error.
    if report.agent.hands.is_some() && report.entries.iter().all(|entry| entry.provider.is_some()) {
        let candidates: Vec<Candidate> = report
            .entries
            .iter()
            .map(|entry| Candidate {
                agent: name.to_string(),
                model: entry.model.clone(),
                effort: entry.effort.clone(),
                provider: entry.provider.clone().expect("mapped above"),
                argv: entry.argv.clone(),
                hands_fragment: entry.hands_fragment.clone(),
                harness: entry.harness.clone(),
                // The hands law reads argv, class and boundary; the
                // resume assessment and the discovery notice are not its
                // terms, and this projection is discarded after that
                // judgment.
                resume: Default::default(),
                hands_notice: None,
                // The entry's own lowering, refused or composed, never a
                // valid empty one standing in for it (design D5.7).
                lowering: entry.lowering.clone(),
            })
            .collect();
        enforce_model_policy(
            what,
            raw,
            &candidates,
            secrets,
            agents,
            SiteLaw {
                boundary,
                dir,
                agent_hands: report.agent.hands.as_ref(),
            },
            sites,
        )?;
    }
    Ok(())
}

/// The resolved chain as the site's candidates, each composed argv judged
/// as an inline one is.
pub(super) fn resolved_candidates(
    at: ReferenceAt<'_>,
    resolution: &Resolution,
    adapters: &Adapters,
) -> Result<Vec<Candidate>, CompileError> {
    let ReferenceAt {
        engine,
        dir,
        what,
        secrets,
        ..
    } = at;
    // The composed argv faces the SAME secret-reference lint as an inline
    // one (decision 0012), and `{brokkr}` is expanded by the same
    // function, so a resolved seat is an inline seat by construction.
    let mut candidates = Vec::with_capacity(resolution.candidates.len());
    for candidate in &resolution.candidates {
        refuse_permission_pins(what, candidate, adapters)?;
        lint_secret_refs(what, &candidate.argv, secrets)?;
        candidates.push(Candidate {
            agent: candidate.agent.clone(),
            model: candidate.model.clone(),
            effort: candidate.effort.clone(),
            provider: candidate.provider.clone(),
            argv: expand_command(engine, dir, &candidate.argv),
            hands_fragment: candidate.hands_fragment.clone(),
            harness: candidate.harness.clone(),
            resume: candidate.resume.clone(),
            hands_notice: candidate.hands_notice.clone(),
            lowering: expand_lowering(engine, dir, &candidate.lowering),
        });
    }
    Ok(candidates)
}

/// The charter this site will be told, pinned to the read of its library
/// that supplied the bytes the library record pins, and that read, held
/// until the seal.
pub(super) fn bind_charter(
    what: &str,
    resolution: &Resolution,
) -> Result<(CharterPin, LibraryRead), CompileError> {
    // Second council H6: the charter this site will be told, bound to the
    // digest its library record pins, so the dispatch door can compare the
    // bytes it is about to hand over against the bytes the compile read.
    // Rebuild unit 17: its owner is the library the office was loaded
    // from, with that library's own root, whether it stands outside the
    // recipe or inside it.
    // Rebuild unit 18-fix-b (council F1, F2): the pin is the binding of a
    // bound read from the library's directory, which must supply the bytes
    // the library record pins; a charter no such read binds is refused here
    // rather than compiled into a seat every dispatch refuses.
    let source = &resolution.charter_source;
    let refused =
        |clause: String| library_refusal(what, &resolution.agent, &source.reference, &clause);
    let bound = match owned_input(&source.library, &source.reference) {
        Ok(bound) if sha256_bytes(&bound.bytes) == source.digest => bound,
        Ok(_) => {
            return Err(refused(
                "whose bytes changed after its library was loaded".into(),
            ))
        }
        Err(InputFault::Missing(error)) => return Err(refused(missing_clause(&error))),
        Err(InputFault::Place(place)) => return Err(refused(place.to_string())),
    };
    let owner = CharterOwner::Library {
        agent: resolution.agent.clone(),
        root: source.library.clone(),
    };
    let pin = CharterPin::of(owner, &source.reference, resolution.charter.clone(), &bound);
    // Rebuild unit 18-fix-b return (F2): the read is held until the seal,
    // which checks it still stands, its owner included.
    let read = LibraryRead {
        site: what.to_string(),
        agent: resolution.agent.clone(),
        reference: source.reference.clone(),
        digest: source.digest.clone(),
        bytes: bound.bytes.len() as u64,
        held: bound.held,
    };
    Ok((pin, read))
}
