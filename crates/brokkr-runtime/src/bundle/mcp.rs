//! Capability-relevant candidate composition (decision 0065 slice two, U1f;
//! requirements SI2 and MB1): each serving a site can launch, an agent's
//! candidate or an inline site's one command, composed for the capability
//! pass beside the MCP server set the engine intends it to launch with.
//!
//! The intended set is a typed fact, read where the site is compiled and
//! never from the bytes it emits. A candidate's is recorded in its own
//! composition from its typed hands and the boundary that boxes them or
//! not, so a fallback never borrows its primary's. An inline site's
//! is recorded in its facts from the hands it resolved and the driver its
//! command dispatches, and relocates with them. The set is the hands server
//! or none, and no hands means an explicitly empty set, never an absent one.
//! Exec has no model MCP surface (SI2): it intends no set, and nothing here
//! stands in for a measurement. Authored bytes equal to an engine fragment
//! stay authored: a serving whose intended set holds no hands server types
//! none of its fragment as the box's hands.
//!
//! What a serving contributes is materialized from that set and the typed
//! MCP holdings its own outcome seals (U7b; MB1, MB2): one `cap-<capability>`
//! broker per holding beside the hands server, judged at that serving
//! alone against its own adapter's measured carriage, which a `requires`
//! ask refuses on and a `wants` drops by. An adapter's legacy server map is
//! no carriage and names no server. No MCP holding is admissible while the
//! compile fence stands (macOS), so there every set contributes itself.
//!
//! SI2's strict admission is decided here for every serving (U1g1): each
//! shape it can be served as, with what its adapter's validated evidence
//! measures of ambient exclusion there, is recorded beside its outcome for
//! U9a's doctor, and refuses past the lifted MCP compile fence, on Linux
//! since U9b1 (operator rulings of 2026-10-07, 2026-10-08 and 2026-10-18).

use brokkr_core::realms::Boundary;
use brokkr_protocol::adapters::{AdapterKind, StrictCause};
use brokkr_protocol::hands::HandsSpec;
use brokkr_protocol::native_controls::{Application, HandsIntent, Provenance, Site};
use thiserror::Error;

use super::{dispatch_driver, CapabilityAdapters, CompileError, SeatClass, SiteFacts};
use crate::agents::ResumeStatus;
use crate::agents::{Adapter, Candidate, Composition, Lowering, Uncarried};
use crate::agents::{McpAxis, McpHands, McpHost, McpInvocation, McpShape, McpUnmeasured};
use crate::capabilities::{
    Authority, Implementation, McpFence, NativeInventory, Outcome, Requests, Serving, SiteAsks,
    SiteCapabilities, Strength, OPAQUE_HARNESS,
};

/// The host this build serves on (decision 0063): a shape measured on the
/// other qualifies nothing here.
#[cfg(target_os = "macos")]
const HOST: McpHost = McpHost::Macos;
#[cfg(not(target_os = "macos"))]
const HOST: McpHost = McpHost::Linux;

impl McpFence {
    /// The fence every load stands under (SC5), by host and never by a
    /// knob. U9b1 lifts it on Linux, where the namespace path is proved:
    /// an `mcp` grant loads and a serving's SI2 cause refuses. On macOS it
    /// stands until boxing exists there (operator ruling 1 of 2026-10-18):
    /// `Authority::load` refuses every `mcp` grant and a serving's SI2
    /// record is kept but never refused.
    #[cfg(all(not(test), target_os = "linux"))]
    pub(crate) fn current() -> McpFence {
        McpFence::Lifted
    }

    /// [`McpFence::current`] off Linux: standing.
    #[cfg(all(not(test), not(target_os = "linux")))]
    pub(crate) fn current() -> McpFence {
        McpFence::Standing
    }

    /// The fence on this thread: standing, as macOS's, unless a [`Lift`]
    /// is held, as Linux's.
    #[cfg(test)]
    pub(crate) fn current() -> McpFence {
        FENCE.get()
    }

    /// The test-only override (operator ruling of 2026-10-08): the fence is
    /// lifted on this thread until the returned [`Lift`] is dropped.
    #[cfg(test)]
    pub(crate) fn lift() -> Lift {
        FENCE.set(McpFence::Lifted);
        Lift(())
    }

    /// A serving's first SI2 cause, which refuses only past the fence.
    fn admit(self, cause: Option<StrictCause>) -> Result<(), StrictCause> {
        match (self, cause) {
            #[cfg(any(test, not(target_os = "linux")))]
            (McpFence::Standing, _) => Ok(()),
            #[cfg(any(test, target_os = "linux"))]
            (McpFence::Lifted, None) => Ok(()),
            #[cfg(any(test, target_os = "linux"))]
            (McpFence::Lifted, Some(cause)) => Err(cause),
        }
    }
}

#[cfg(test)]
thread_local! {
    static FENCE: std::cell::Cell<McpFence> = const { std::cell::Cell::new(McpFence::Standing) };
}

/// A held lift of the fence ([`McpFence::lift`]): standing again once
/// dropped, unwinding included.
#[cfg(test)]
pub(crate) struct Lift(());

#[cfg(test)]
impl Drop for Lift {
    fn drop(&mut self) {
        FENCE.set(McpFence::Standing);
    }
}

/// One serving's SI2 record: each shape it can be served as, with the
/// ambient exclusion its adapter measures there, read by the adapter's own
/// isolation reader and never from a name. Every serving is served cold; a
/// work site, which may be offered a rejoin, also as the cold replacement
/// and as each resume shape its adapter declares `supported` under the
/// same hands, while a gate is never offered one. An `unmeasured` or
/// `unsupported` resume is never performed, so it is no served shape
/// (operator ruling (2) of 2026-10-28). The hands are the intended set's: the
/// box's where it holds the hands server, the harness's own sandbox where
/// `harnessed` hands were composed, and none otherwise. Exec's set, which
/// serves no model, and a candidate that composed nothing have no shape.
fn strictness(
    intent: Option<McpIntent>,
    harnessed: bool,
    class: SeatClass,
    adapter: Option<&Adapter>,
) -> Vec<(McpShape, McpAxis)> {
    let hands = match intent {
        Some(McpIntent::Hands | McpIntent::Brokered) => McpHands::Boxed,
        Some(McpIntent::Empty) if harnessed => McpHands::Harness,
        Some(McpIntent::Empty) => McpHands::NoHands,
        Some(McpIntent::NoModelSurface) | None => return Vec::new(),
    };
    let mut invocations = vec![McpInvocation::Cold];
    match class {
        SeatClass::Work => {
            invocations.push(McpInvocation::Replacement);
            let declared = adapter.into_iter().flat_map(|adapter| &adapter.resume.0);
            let declared = declared.filter(|(_, shape)| {
                shape.status == ResumeStatus::Supported && shape.hands == hands.word()
            });
            invocations.extend(declared.map(|(name, _)| McpInvocation::Resume(name.clone())));
        }
        SeatClass::Gate => {}
    }
    invocations
        .into_iter()
        .map(|invocation| {
            let shape = McpShape {
                invocation,
                hands,
                host: HOST,
            };
            let ambient = match adapter {
                Some(adapter) => adapter.mcp.isolation(&shape).ambient,
                None => McpAxis::Unmeasured(McpUnmeasured::Absent),
            };
            (shape, ambient)
        })
        .collect()
}

/// The MCP server set the engine intends one serving to launch with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpIntent {
    /// No hands and no holding: strictly no server at all.
    Empty,
    /// The engine's hands server alone.
    Hands,
    /// The hands server and, beside it, one broker per MCP holding the
    /// serving's outcome brokers (U7d2; MB1): never composed, only read by
    /// the engine from a hands set against its holdings
    /// ([`McpIntent::brokering`]).
    Brokered,
    /// The exec harness, which serves no model: no set is intended and none
    /// is measured.
    NoModelSurface,
}

impl McpIntent {
    /// A model serving's set, from its typed hands intent and whether the
    /// boundary it is composed under boxes them: only boxed hands are served
    /// by the engine's hands server. Under `harness` the harness's own
    /// sandbox carries an office's hands and under `open` nothing does, so
    /// either intends strictly no server, whatever the office declares.
    pub(crate) fn composed(hands: HandsIntent, boundary: Boundary) -> McpIntent {
        match (hands, boundary.is_boxed()) {
            (HandsIntent::Required, true) => McpIntent::Hands,
            (HandsIntent::Required, false) | (HandsIntent::None, _) => McpIntent::Empty,
        }
    }

    /// An inline site's set: the driver kind its command dispatches decides
    /// whether it has a model surface at all, and its resolved hands decide
    /// the set. Those hands are boxed: the hands law refuses an inline model
    /// harness's hands unboxed (decision 0046 ruling 4). An opaque command,
    /// which no adapter answers for, still intends the set its hands give.
    pub(super) fn inline(driver: Option<&str>, hands: Option<&HandsSpec>) -> McpIntent {
        match driver.and_then(AdapterKind::parse) {
            Some(AdapterKind::Exec) => McpIntent::NoModelSurface,
            Some(
                AdapterKind::Claude
                | AdapterKind::Lanetally
                | AdapterKind::Codex
                | AdapterKind::Dsh,
            )
            | None => match hands {
                Some(_) => McpIntent::Hands,
                None => McpIntent::Empty,
            },
        }
    }

    /// One candidate's set, the one its own composition recorded where its
    /// boundary was read, and `None` where nothing was composed: no adapter
    /// mapped the model, or the entry was refused and launches nothing.
    /// Crate-visible, as the type is through `crate::bundle`, so dispatch
    /// reads the set this pass composed with and never derives it again.
    pub(crate) fn of_candidate(candidate: &Candidate) -> Option<McpIntent> {
        match &candidate.lowering {
            Lowering::Composed(composition) => Some(composition.mcp),
            Lowering::Refused(_) | Lowering::Unavailable => None,
        }
    }

    /// The servers this set contributes beside `outcome`'s typed MCP
    /// holdings (U7b; MB1): each holding's own `cap-<capability>` server,
    /// in the outcome's order, beside the box's hands, and nothing else.
    /// Without an MCP holding the set contributes itself. A set without the
    /// hands server, or exec's, which serves no model, carries no broker
    /// (MB2): it refuses by its first holding, whatever that asked. The
    /// brokered set is the hands set read against holdings, so it
    /// contributes as the hands set does.
    pub(crate) fn contribution(self, outcome: &Outcome) -> Result<McpContribution<'_>, Unservable> {
        let mut servers = (outcome.held.iter())
            .filter_map(|(capability, holding)| match &holding.implementation {
                Implementation::Mcp { server, .. } => Some((capability, server.as_str())),
                Implementation::Native { .. } => None,
            })
            .peekable();
        let first = servers.peek().map(|(capability, _)| capability.to_string());
        match (self, first) {
            (McpIntent::Empty, None) => Ok(McpContribution::Empty),
            (McpIntent::Hands | McpIntent::Brokered, None) => Ok(McpContribution::Hands),
            (McpIntent::NoModelSurface, None) => Ok(McpContribution::NoModelSurface),
            (McpIntent::Hands | McpIntent::Brokered, Some(_)) => Ok(McpContribution::Brokered(
                servers.map(|(_, server)| server).collect(),
            )),
            (McpIntent::Empty, Some(first)) => Err(Unservable::NoHands(first)),
            (McpIntent::NoModelSurface, Some(first)) => Err(Unservable::NoModelChannel(first)),
        }
    }

    /// The set the engine reads this one as at a serving whose compiled
    /// outcome is `outcome` (U7d2; MB1): the brokered set where its
    /// holdings contribute brokers beside the hands, and itself otherwise.
    /// A set its holdings cannot be served beside was refused at compile
    /// ([`McpSite::contribute`]) and brokers nothing here.
    pub(crate) fn brokering(self, outcome: Option<&Outcome>) -> McpIntent {
        match outcome.map(|outcome| self.contribution(outcome)) {
            Some(Ok(McpContribution::Brokered(_))) => McpIntent::Brokered,
            Some(Ok(
                McpContribution::Empty | McpContribution::Hands | McpContribution::NoModelSurface,
            ))
            | Some(Err(_))
            | None => self,
        }
    }
}

/// The engine-owned MCP servers one serving contributes (U7b; MB1), from
/// its intended set and its own typed holdings, never from an emitted
/// command or an adapter's server map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum McpContribution<'a> {
    Empty,
    Hands,
    /// The box's hands and, beside them, each held server by its exact name.
    Brokered(Vec<&'a str>),
    NoModelSurface,
}

/// Why a serving cannot be given the servers its holdings ask (U7b; MB1,
/// MB2), each after the bounded site prefix.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum Unservable {
    #[error("MCP capability '{0}' requires workspace hands at this site")]
    NoHands(String),
    #[error("MCP capability '{0}' requires a model harness with an engine-owned MCP channel")]
    NoModelChannel(String),
    #[error("requires capability '{capability}' through dialect '{dialect}', but {cause}")]
    Uncarried {
        capability: String,
        dialect: String,
        cause: Uncarried,
    },
    #[error("MCP server name '{server}' is not representable by provider '{provider}'")]
    Unrepresentable { server: String, provider: String },
}

/// Whether `server` is one bare name, ASCII letters, digits, `_` and `-`:
/// the only server name the engine's MCP builder writes into any
/// provider's configuration (U7a). Nothing is shortened into one.
fn representable(server: &str) -> bool {
    (server.chars()).all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// One serving as the capability pass judges its MCP facts: the set it
/// intends, `None` where it composed nothing and launches nothing; the
/// adapter answering for its provider, `None` for an opaque command; and
/// its SI2 record ([`strictness`]).
pub(super) struct McpSite<'a> {
    pub(super) intent: Option<McpIntent>,
    pub(super) adapter: Option<&'a Adapter>,
    pub(super) strict: Vec<(McpShape, McpAxis)>,
}

impl McpSite<'_> {
    /// MB1 and MB2 at this serving alone (U7b), once `outcome` is resolved
    /// for it: an MCP holding needs the hands server and a model channel
    /// whatever its strength; then its provider's own measured carriage,
    /// which a `requires` ask refuses on and a `wants` drops by, the dropped
    /// holding contributing nothing; then each server's exact name. The
    /// adapter, set and outcome are this serving's own, so a fallback never
    /// borrows another's holdings, carriage or set.
    pub(super) fn contribute(
        &self,
        who: Site<'_>,
        asks: &Requests,
        outcome: &mut Outcome,
    ) -> Result<(), Unservable> {
        let Some(intent) = self.intent else {
            return Ok(());
        };
        let held: Vec<(String, String)> = (outcome.held.iter())
            .filter(|(_, holding)| matches!(holding.implementation, Implementation::Mcp { .. }))
            .map(|(capability, holding)| (capability.clone(), holding.dialect.clone()))
            .collect();
        let Some((first, _)) = held.first() else {
            return Ok(());
        };
        intent.contribution(outcome)?;
        let Some(adapter) = self.adapter else {
            return Err(Unservable::NoModelChannel(first.clone()));
        };
        if let Err(cause) = adapter.mcp.carries(&outcome.provider) {
            for (capability, dialect) in held {
                match asks.get(&capability) {
                    Some(Strength::Requires) => {
                        let cause = cause.clone();
                        return Err(Unservable::Uncarried {
                            capability,
                            dialect,
                            cause,
                        });
                    }
                    Some(Strength::Wants) | None => {
                        let notice = format!(
                            "{who}: dropped wanted capability '{capability}' through dialect \
                             '{dialect}' because {cause}"
                        );
                        outcome.held.remove(&capability);
                        outcome
                            .not_held
                            .insert(capability.clone(), cause.to_string());
                        outcome.notices.push((capability, notice));
                    }
                }
            }
        }
        if let McpContribution::Brokered(servers) = intent.contribution(outcome)? {
            if let Some(server) = servers.into_iter().find(|server| !representable(server)) {
                let (server, provider) = (server.to_string(), outcome.provider.clone());
                return Err(Unservable::Unrepresentable { server, provider });
            }
        }
        Ok(())
    }
}

/// How much of `fragment` a plan types as the box's hands: all of it where
/// the intended set holds the hands server or the serving has no model
/// surface (exec's fragment is its box's, carried by no MCP server), and
/// none where the set is empty or no intent was recorded, whatever bytes the
/// fragment holds.
fn typed_hands(intent: Option<McpIntent>, fragment: &[String]) -> usize {
    match intent {
        Some(McpIntent::Hands | McpIntent::Brokered | McpIntent::NoModelSurface) => fragment.len(),
        Some(McpIntent::Empty) | None => 0,
    }
}

/// An inline site's one serving as the capability pass composes it: the
/// driver kind its command dispatches, `None` for an opaque one; its
/// command, wholly the author's; the limits its typed allow lowered to;
/// the hands fragment the engine appends behind it; and the set its facts
/// intend.
pub(super) struct InlineServing<'a> {
    pub(super) driver: Option<&'a str>,
    pub(super) argv: &'a [String],
    pub(super) local: &'a [String],
    pub(super) hands: &'a [String],
    pub(super) intent: Option<McpIntent>,
}

impl<'a> InlineServing<'a> {
    /// The serving `facts` record for an inline site whose command is
    /// `argv`: its typed allow lowered where its facts were recorded, and its
    /// hands fragment the one the engine appends behind its command, served
    /// like an agent's (operator ruling (B) of 2026-09-27; rebuild unit 14a4a).
    pub(super) fn of(
        facts: &'a SiteFacts,
        driver: Option<&'a str>,
        argv: &'a [String],
    ) -> InlineServing<'a> {
        InlineServing {
            driver,
            argv,
            local: facts
                .inline_local
                .as_ref()
                .map_or(&[], |lowered| lowered.limits.as_slice()),
            hands: facts
                .inline_hands
                .as_ref()
                .map_or(&[], |segment| segment.argv.as_slice()),
            intent: facts.inline_mcp,
        }
    }
}

/// The adapter serving `provider`, `None` where none answers for it.
fn adapter<'a>(adapters: CapabilityAdapters<'a>, provider: &str) -> Option<&'a Adapter> {
    adapters
        .adapters
        .and_then(|adapters| adapters.adapter(provider))
}

/// What the adapter serving `provider` declares of its native powers, with
/// the digest that pins it; `None` where no adapter answers for it.
fn native<'a>(
    adapters: CapabilityAdapters<'a>,
    provider: &str,
) -> Option<(&'a NativeInventory, &'a str)> {
    adapter(adapters, provider).map(|adapter| (&adapter.native, adapter.digest.as_str()))
}

/// The names the harness of the adapter serving `provider` reads to
/// authenticate (MB4); none where no adapter answers for it.
fn auth<'a>(adapters: CapabilityAdapters<'a>, provider: &str) -> &'a [String] {
    adapter(adapters, provider).map_or(&[], |adapter| adapter.harness_auth.as_slice())
}

/// An agent-backed site's plan (decision 0065 ruling 5; [`Planned`]): its
/// asks resolved once per candidate of its chain. Under the harness
/// boundary the engine appends each candidate's `hands.harness.*` fragment
/// for the seat's `class` behind a hands site's argv; it is composed with
/// here exactly as at the launch, so a limit it carries holds at compile,
/// and a wanted holding it excludes drops rather than refusing its spawn
/// (unit 12-fix-b, R2). `class` is `None` where no such fragment applies.
pub(super) fn candidate_capabilities(
    authority: &Authority,
    adapters: CapabilityAdapters<'_>,
    asks: SiteAsks,
    chain: &[Candidate],
    class: Option<SeatClass>,
) -> Planned {
    let resolve: Resolve<'_> = &|asks, serving| resolve(authority, asks, serving);
    candidate_capabilities_with(authority, adapters, asks, (chain, class), resolve)
}

/// How one serving's outcome is resolved: [`resolve`], the one resolver
/// every compile uses, as a closure seam (decision 0071 ruling 2).
pub(super) type Resolve<'r> = &'r dyn Fn(&SiteAsks, &Serving<'_>) -> Result<Outcome, CompileError>;

/// [`Authority::resolve`], whose refusal is typed at this seam as the
/// capability refusal it is (decision 0071 ruling 8).
pub(super) fn resolve(
    authority: &Authority,
    asks: &SiteAsks,
    serving: &Serving<'_>,
) -> Result<Outcome, CompileError> {
    (authority.resolve(asks, serving)).map_err(CompileError::Capability)
}

/// [`candidate_capabilities`] with each candidate's outcome resolved by
/// `resolve` before its MCP contribution and SI2 record are judged.
pub(super) fn candidate_capabilities_with(
    authority: &Authority,
    adapters: CapabilityAdapters<'_>,
    asks: SiteAsks,
    (chain, class): (&[Candidate], Option<SeatClass>),
    resolve: Resolve<'_>,
) -> Planned {
    // Each serving's argv in its two parts, by who wrote them (decision
    // 0066 ruling 4): a candidate's is the agent's composed argv and then the
    // adapter's hands fragment, which `agents::compose` appended LAST and
    // recorded — so the parts are split at the length it recorded there, a
    // fact carried from where the fragment was appended, never recovered by
    // matching its text.
    //
    // The HARNESS is the driver kind the command dispatches, read off the
    // command itself: a candidate's argv opens with its adapter's `driver`,
    // which may dispatch the codex or claude driver under whatever name the
    // adapter carries.
    let harnesses: Vec<String> = chain
        .iter()
        .map(|candidate| {
            dispatch_driver(&candidate.argv).unwrap_or_else(|| OPAQUE_HARNESS.to_string())
        })
        .collect();
    // A candidate's engine fragment: the box's hands `compose` recorded,
    // then the managed boundary fragment the engine appends behind them.
    let fragments: Vec<Vec<String>> = chain
        .iter()
        .map(|candidate| {
            let managed = match class {
                Some(SeatClass::Gate) => candidate.harness.gate.as_deref(),
                Some(SeatClass::Work) => candidate.harness.work.as_deref(),
                None => None,
            };
            [candidate.parts().1, managed.unwrap_or_default()].concat()
        })
        .collect();
    // What admits a tool without a holding, by type (rebuild unit
    // 12-fix-c): the box's hands `compose` recorded from the agent's typed
    // hands, which open the fragment, counted only where the candidate's own
    // intended set holds the hands server, and the limits its typed allow
    // lowered to — never read back from the argv.
    let provenances: Vec<Provenance> = chain
        .iter()
        .map(|candidate| Provenance {
            hands: typed_hands(
                McpIntent::of_candidate(candidate),
                &candidate.hands_fragment,
            ),
            local: match &candidate.lowering {
                Lowering::Composed(Composition {
                    application: Application::Direct(limits),
                    ..
                }) => limits.clone(),
                _ => Vec::new(),
            },
        })
        .collect();
    let servings: Vec<Serving<'_>> = chain
        .iter()
        .zip(&harnesses)
        .zip(&fragments)
        .zip(&provenances)
        .map(|(((candidate, harness), fragment), provenance)| {
            let (authored, _) = candidate.parts();
            Serving {
                provider: &candidate.provider,
                harness,
                model: Some(&candidate.model),
                native: native(adapters, &candidate.provider),
                unloaded: adapters.unloaded,
                authored,
                fragment,
                provenance,
                // An agent reference is total (AC-21): the seat writes
                // no argv, and its composition is the adapter's
                // template, the engine's local permissions and hands
                // (design D5.7), none of it the recipe's words.
                written: &[],
                // Each candidate's own adapter: a fallback never
                // borrows another candidate's names.
                auth: auth(adapters, &candidate.provider),
            }
        })
        .collect();
    // SI2 and MB1 at each candidate, by its own composition and adapter: a
    // fallback never borrows its primary's record, carriage or set.
    let sites = chain
        .iter()
        .map(|candidate| {
            let intent = McpIntent::of_candidate(candidate);
            let adapter = adapter(adapters, &candidate.provider);
            McpSite {
                intent,
                adapter,
                strict: strictness(intent, class.is_some(), asks.class, adapter),
            }
        })
        .collect();
    resolved(authority, (asks, resolve), &servings, sites)
}

/// An inline site's plan ([`Planned`]): its asks resolved for the one
/// driver its command dispatches. A command that dispatches no built-in
/// driver is an opaque custom one: no adapter answers for it, so its native
/// inventory is unmeasured and it can hold nothing. An inline site's argv is
/// wholly the author's, and its typed allow and hands are the engine's,
/// served like an agent's.
pub(super) fn inline_capabilities(
    authority: &Authority,
    adapters: CapabilityAdapters<'_>,
    asks: SiteAsks,
    inline: &InlineServing<'_>,
) -> Planned {
    let harness = inline.driver.unwrap_or(OPAQUE_HARNESS);
    let provenance = Provenance {
        hands: typed_hands(inline.intent, inline.hands),
        local: inline.local.to_vec(),
    };
    let serving = Serving {
        provider: harness,
        harness,
        model: None,
        native: native(adapters, harness),
        unloaded: adapters.unloaded,
        authored: inline.argv,
        fragment: inline.hands,
        provenance: &provenance,
        written: inline.argv,
        // The names its own driver's adapter reads to authenticate, as a
        // candidate's are (MB4; U7d3 L3): none for an opaque command.
        auth: auth(adapters, harness),
    };
    // Inline hands are boxed or none: the hands law refuses an inline model
    // harness's hands unboxed (decision 0046 ruling 4).
    let adapter = adapter(adapters, harness);
    let site = McpSite {
        intent: inline.intent,
        adapter,
        strict: strictness(inline.intent, false, asks.class, adapter),
    };
    let servings = std::slice::from_ref(&serving);
    let resolve: Resolve<'_> = &|asks, serving| resolve(authority, asks, serving);
    resolved(authority, (asks, resolve), servings, vec![site])
}

/// A site's plan as [`resolved`] leaves it: its sealed capability facts or
/// the first refusal, and beside either each serving's SI2 verdict, read
/// from its own record whatever its resolution came to (U9a2): the cause
/// the fence's admission refuses it with once lifted, or `None`.
pub(super) struct Planned {
    pub(super) capabilities: Result<SiteCapabilities, CompileError>,
    pub(super) strict: Vec<Option<StrictCause>>,
}

/// Each serving's SI2 verdict, read from its `sites` entry's record before
/// any is resolved, and then the plan [`sealed`] resolves.
fn resolved(
    authority: &Authority,
    asked: (SiteAsks, Resolve<'_>),
    servings: &[Serving<'_>],
    sites: Vec<McpSite<'_>>,
) -> Planned {
    let strict: Vec<Option<StrictCause>> = (servings.iter().zip(&sites))
        .map(|(serving, site)| McpAxis::refusal(&site.strict, serving.provider))
        .collect();
    Planned {
        capabilities: sealed(authority, asked, servings, (sites, &strict)),
        strict,
    }
}

/// `asks` resolved by `resolve` against each serving in order, each outcome
/// then given its own MCP contribution by its `sites` entry and judged by
/// that serving's SI2 verdict in `strict`, or the first refusal; the
/// records are kept beside the outcomes, which way the fence stands.
fn sealed(
    authority: &Authority,
    (asks, resolve): (SiteAsks, Resolve<'_>),
    servings: &[Serving<'_>],
    (sites, strict): (Vec<McpSite<'_>>, &[Option<StrictCause>]),
) -> Result<SiteCapabilities, CompileError> {
    let who = authority.who(&asks);
    let refused =
        |cause: &dyn std::fmt::Display| CompileError::Capability(format!("{who}: {cause}"));
    let mut outcomes = Vec::with_capacity(servings.len());
    let mut records = Vec::with_capacity(sites.len());
    for ((serving, site), verdict) in servings.iter().zip(sites).zip(strict) {
        let mut outcome = resolve(&asks, serving)?;
        (site.contribute(who, &asks.asks, &mut outcome)).map_err(|cause| refused(&cause))?;
        outcomes.push(outcome);
        (authority.fence.admit(verdict.clone())).map_err(|cause| refused(&cause))?;
        records.push(site.strict);
    }
    Ok(SiteCapabilities {
        asks,
        outcomes,
        strict: records,
    })
}
