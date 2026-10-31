//! The typed serving inputs a spawn is sealed with (rebuild unit 14a2) and
//! the MCP isolation intent among them (decision 0065 slice two, U1g2),
//! moved here from `engine.rs`, which is frozen at its baseline, when the
//! intent gained the brokered set (U7d2; MB1, MB4, SI2).
//!
//! A serving whose own intended set holds the box's hands and brokers MCP
//! holdings ([`super::marks::brokered_at`], the one projection its plans
//! were provisioned from) is sealed as [`SealedServers::Brokered`]: each
//! [`SealedBroker`] is the one its attempt's provisioning wrote for that
//! holding at that site, carrying the plan's locator and digest as the
//! inventory pins them and the dialect's secret names the harness is
//! served without ([`super::broker`]). The brokers are carried from the
//! attempt's provisioning, never rebuilt here, and a serving whose brokered
//! holdings are not exactly the ones provisioned for its site refuses its
//! spawn: nothing is reconciled.

use std::path::Path;

use brokkr_core::realms::Boundary;
use brokkr_protocol::hands::Sealed;
use brokkr_protocol::native_controls::{
    flatten, Origin, SealedAssessment, SealedBoundary, SealedBroker, SealedDialect, SealedFence,
    SealedIsolation, SealedServers, SealedServing, Transport,
};
use thiserror::Error;

use super::marks::{brokered_at, outcome_at};
use super::{composed, Selection, SiteSpawn};
use crate::agents::{Candidate, McpAxis, McpInvocation};
use crate::argv::Executable;
use crate::bundle::{HandsState, McpIntent, SeatClass, SiteFacts};

impl Selection {
    /// The brokers this attempt's provisioning sealed for the site at
    /// `label`, none where it provisioned none there.
    pub(super) fn brokers(&self, label: &str) -> &[SealedBroker] {
        self.brokers.get(label).map_or(&[], Vec::as_slice)
    }
}

/// The typed serving inputs of the site a spawn serves (rebuild unit
/// 14a2), carried from where they were chosen (rebuild unit 14a1) and
/// never read back from any argv: the selected candidate's composition, or
/// an inline site's recorded dialect and hands. Of the two boundary
/// fragments a composition carries, the spawn's class selects the one
/// sealed, as [`super::compose_segments`] selects the one appended; a spawn
/// with no class refuses, as every composition classes its own (operator
/// ruling of 2026-09-30, unit 26c). The run's `boundary` is
/// sealed beside them as the typed fact the site stood under, and none for
/// a site without hands (rebuild unit 14a4c), so a declared-empty fragment
/// pair under `harness` never reads as no boundary. No plan was provisioned
/// here, so a serving that brokers a holding refuses; dispatch seals with
/// its attempt's own brokers.
pub fn serving_inputs(
    link: Option<&Candidate>,
    facts: Option<&SiteFacts>,
    class: Option<SeatClass>,
    boundary: Boundary,
) -> Result<SealedServing, String> {
    serving_with(link, facts, class, boundary, &[]).map_err(|unsealed| unsealed.to_string())
}

/// The serving inputs `spawn` is sealed with at the site `facts` describe
/// ([`serving_inputs`]), with the brokers the attempt's provisioning sealed
/// for this site, `provisioned`, and its hands segment expanded beside them
/// under the engine executable, `workdir`, the run's artifact root and the
/// reach its box was built from ([`beside`]; U7d2, U8a, U8a2c).
pub(super) fn served(
    spawn: &mut SiteSpawn,
    (link, facts): (Option<&Candidate>, Option<&SiteFacts>),
    boundary: Boundary,
    worktree: (&Executable, &Path, &Path, Option<&Sealed>),
    provisioned: &[SealedBroker],
) -> Result<SealedServing, Unsealed> {
    let serving = serving_with(link, facts, spawn.class, boundary, provisioned)?;
    beside(spawn, &serving, worktree, provisioned)?;
    Ok(serving)
}

/// The isolation intent dispatch seals beside a launch at the site `facts`
/// describe, for the serving `link` selects, with no broker provisioned:
/// `None` where it seals none, the site's capability plan holding no
/// outcome for that serving (`outcome_at`), so the launch is served as a
/// driver run by hand is, or where it refuses to seal one. Doctor models a
/// dsh seat's home from it through the driver's own admission (#717).
pub fn sealed_isolation(
    link: Option<&Candidate>,
    facts: Option<&SiteFacts>,
) -> Option<SealedIsolation> {
    outcome_at(facts, link)?;
    isolation_intent(link, facts, &[]).ok()
}

/// The MCP fence this engine stands under, as an intent seals it.
fn fence() -> SealedFence {
    match crate::capabilities::McpFence::current() {
        #[cfg(any(test, not(target_os = "linux")))]
        crate::capabilities::McpFence::Standing => SealedFence::Standing,
        #[cfg(any(test, target_os = "linux"))]
        crate::capabilities::McpFence::Lifted => SealedFence::Lifted,
    }
}

/// [`serving_inputs`] with the brokers the attempt's provisioning sealed
/// for this site, `provisioned` (U7d2).
fn serving_with(
    link: Option<&Candidate>,
    facts: Option<&SiteFacts>,
    class: Option<SeatClass>,
    boundary: Boundary,
    provisioned: &[SealedBroker],
) -> Result<SealedServing, Unsealed> {
    let stands = facts
        .filter(|facts| matches!(facts.hands, HandsState::Hands(_)))
        .and_then(|_| SealedBoundary::named(boundary.word()));
    let serving = |link| composed(link).map(|composition| (*composition.serving).clone());
    let carried = link.map_or_else(|| facts.and_then(SiteFacts::inline_serving), serving);
    let crate::agents::ServingInputs {
        dialect,
        pins,
        spec,
    } = carried.ok_or(Unsealed::Uncarried)?;
    let boundary = match class {
        Some(SeatClass::Gate) => dialect.boundary.gate,
        Some(SeatClass::Work) => dialect.boundary.work,
        None => return Err(Unsealed::Unclassed),
    };
    Ok(SealedServing {
        dialect: SealedDialect {
            permissions: dialect.permissions,
            sandbox: dialect.sandbox,
            hands: dialect.hands,
            boundary,
            stands,
        },
        pins,
        spec,
        isolation: isolation_intent(link, facts, provisioned)?,
    })
}

/// `spawn`'s hands segment expanded beside `brokers`, the ones its
/// attempt provisioned for its site (U7d2; MB1): the measured fragment
/// `serving` seals, which the spawn carries expanded for the box's hands
/// alone exactly once, is expanded again by the set's one builder
/// ([`Transport::expand_beside`]) with the brokers beside the hands, under
/// the executable, `workdir`, artifact root and sealed reach it was
/// composed with. A spawn without brokers keeps its every byte; one whose
/// hands segment is not that expansion, or that was composed with no
/// sealed reach, refuses, and nothing is rebuilt around another.
pub(super) fn beside(
    spawn: &mut SiteSpawn,
    serving: &SealedServing,
    (engine, workdir, artifacts, sealed): (&Executable, &Path, &Path, Option<&Sealed>),
    brokers: &[SealedBroker],
) -> Result<(), Unsealed> {
    if brokers.is_empty() {
        return Ok(());
    }
    let spec = serving.spec.as_ref().ok_or(Unsealed::Unexpanded)?;
    let hands = Transport {
        brokkr: engine.or_empty(),
        workdir,
        artifacts,
        sealed: sealed.ok_or(Unsealed::Unexpanded)?,
        spec,
    };
    let fragment = &serving.dialect.hands;
    let (Some(alone), Some(whole)) = (
        hands.expand(fragment),
        hands.expand_beside(fragment, brokers),
    ) else {
        return Err(Unsealed::Unexpanded);
    };
    let mut expanded = (spawn.segments.iter_mut())
        .filter(|segment| segment.origin == Origin::Hands && segment.argv == alone);
    let (Some(segment), None) = (expanded.next(), expanded.next()) else {
        return Err(Unsealed::Unexpanded);
    };
    segment.argv = whole;
    spawn.argv = flatten(&spawn.segments);
    Ok(())
}

/// Why no serving inputs are sealed beside a site's launch, each in the
/// operator's words its spawn refuses with.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum Unsealed {
    /// No composition and no inline site recorded the site's typed serving
    /// inputs (rebuild unit 14a2).
    #[error(
        "dispatch refused: the site's typed serving inputs were never recorded, so none can be \
         sealed beside its launch record; they are carried from the composition and never \
         recovered from its argv (rebuild unit 14a2; design D5.7, D6)"
    )]
    Uncarried,
    /// The spawn was composed for no seat class (rebuild unit 14a2).
    #[error(
        "dispatch refused: the spawn was composed for no seat class, so none of the boundary \
         fragments its serving inputs carry can be selected and sealed; a fragment is never \
         chosen by default (decision 0046 ruling 4; rebuild unit 14a2)"
    )]
    Unclassed,
    /// The compile never recorded the site's MCP server set (U1g2).
    #[error(
        "dispatch refused: the site's MCP server set was never recorded, so no isolation intent \
         is sealed beside its launch (decision 0065 slice two, U1g2)"
    )]
    Unrecorded,
    /// The serving brokers holdings whose plans this attempt did not
    /// provision for its site, or was handed brokers it does not hold
    /// (U7d2): each list names capabilities, in order.
    #[error(
        "dispatch refused: the site's serving candidate brokers the MCP capabilities [{held}], \
         but this attempt provisioned plans for [{provisioned}] at its site, so no broker set is \
         sealed beside its launch; a broker is sealed only from its own provisioned plan \
         (decision 0065 slice two, U7d2)"
    )]
    Unprovisioned { held: String, provisioned: String },
    /// The spawn's hands segment is not its measured fragment's hands-only
    /// expansion exactly once, or the set cannot be expanded (U7d2).
    #[error(
        "dispatch refused: the spawn does not carry its measured hands fragment expanded for the \
         box's hands exactly once, so the brokers its attempt provisioned cannot be expanded \
         beside them; no document is rebuilt around another (decision 0065 slice two, U7d2)"
    )]
    Unexpanded,
}

/// The MCP isolation intent sealed with a site's serving inputs (decision
/// 0065 slice two, U1g2): the set the compile composed the selected serving
/// with ([`McpIntent::of_candidate`], or an inline site's own), its brokers
/// the ones `provisioned` for its brokered holdings ([`servers`]), each
/// shape's SI2 record beside that serving's own outcome, a resume shape's
/// by name, and the MCP fence this engine stands under; a shape unrecorded
/// is unmeasured.
fn isolation_intent(
    link: Option<&Candidate>,
    facts: Option<&SiteFacts>,
    provisioned: &[SealedBroker],
) -> Result<SealedIsolation, Unsealed> {
    let inline = facts.and_then(|facts| facts.inline_mcp);
    let intent = link.map_or(inline, McpIntent::of_candidate);
    let intent = intent.ok_or(Unsealed::Unrecorded)?;
    let intent = intent.brokering(outcome_at(facts, link));
    let servers = servers(intent, facts, link, provisioned)?;
    let site = facts.and_then(|facts| facts.capabilities.as_ref());
    let record = site.and_then(|site| {
        let outcome = site.serving(link.map(|link| (&*link.provider, &*link.model)))?;
        let at = (site.outcomes.iter()).position(|each| std::ptr::eq(each, outcome))?;
        site.strict.get(at)
    });
    let mut sealed = SealedIsolation {
        servers,
        fence: fence(),
        ..SealedIsolation::default()
    };
    for (shape, axis) in record.into_iter().flatten() {
        let assessed = match axis {
            McpAxis::Measured { .. } => SealedAssessment::Measured,
            McpAxis::Unsupported { reason } => SealedAssessment::Unsupported(reason.clone()),
            McpAxis::Unmeasured(_) | McpAxis::Inapplicable { .. } => SealedAssessment::Unmeasured,
        };
        match &shape.invocation {
            McpInvocation::Cold => sealed.cold = assessed,
            McpInvocation::Replacement => sealed.replacement = assessed,
            McpInvocation::Resume(name) => drop(sealed.resume.insert(name.clone(), assessed)),
        }
    }
    Ok(sealed)
}

/// The server set `intent`, as the engine reads it against the serving's
/// holdings ([`McpIntent::brokering`]), seals for the serving `link`
/// selects at the site `facts` describe (U7d2; MB1): the brokered set
/// seals the brokers `provisioned` for the holdings it brokers
/// ([`brokered_at`]). The holdings brokered and the brokers provisioned
/// must name the same capabilities in the same order, none for a set that
/// brokers none; anything else refuses.
fn servers(
    intent: McpIntent,
    facts: Option<&SiteFacts>,
    link: Option<&Candidate>,
    provisioned: &[SealedBroker],
) -> Result<SealedServers, Unsealed> {
    let held: Vec<&str> = (brokered_at(facts, link).iter())
        .map(|held| held.capability)
        .collect();
    let sealed: Vec<&str> = (provisioned.iter())
        .map(|broker| broker.capability.as_str())
        .collect();
    if held != sealed {
        let named = |names: &[&str]| {
            (names.iter().map(|name| format!("'{name}'")))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let (held, provisioned) = (named(&held), named(&sealed));
        return Err(Unsealed::Unprovisioned { held, provisioned });
    }
    Ok(match intent {
        McpIntent::Empty => SealedServers::Empty,
        McpIntent::Hands | McpIntent::NetworkedHands => SealedServers::Hands,
        McpIntent::Brokered => SealedServers::Brokered(provisioned.to_vec()),
        McpIntent::NoModelSurface => SealedServers::NoModelSurface,
    })
}
