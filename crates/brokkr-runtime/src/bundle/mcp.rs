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
//! command dispatches, and relocates with them. No MCP holding is admissible
//! while the compile fence stands (U9b), so the set is the hands server or
//! none, and no hands means an explicitly empty set, never an absent one.
//! Exec has no model MCP surface (SI2): it intends no set, and nothing here
//! stands in for a measurement. Authored bytes equal to an engine fragment
//! stay authored: a serving whose intended set holds no hands server types
//! none of its fragment as the box's hands.
//!
//! SI2's strict admission is decided here for every serving (U1g1): each
//! shape it can be served as, with what its adapter's validated evidence
//! measures of ambient exclusion there, is recorded beside its outcome for
//! U9a's doctor, and refuses only once U9b lifts the MCP compile fence
//! (operator rulings of 2026-10-07 and 2026-10-08).

use brokkr_core::realms::Boundary;
use brokkr_protocol::adapters::{AdapterKind, StrictCause};
use brokkr_protocol::hands::HandsSpec;
use brokkr_protocol::native_controls::{Application, HandsIntent, Provenance};

use super::{dispatch_driver, CapabilityAdapters, CompileError, SeatClass, SiteFacts};
use crate::agents::{Adapter, Candidate, Composition, Lowering};
use crate::agents::{McpAxis, McpHands, McpHost, McpInvocation, McpShape, McpUnmeasured};
use crate::capabilities::{
    Authority, McpFence, NativeInventory, Serving, SiteAsks, SiteCapabilities, OPAQUE_HARNESS,
};

/// The host this build serves on (decision 0063): a shape measured on the
/// other qualifies nothing here.
#[cfg(target_os = "macos")]
const HOST: McpHost = McpHost::Macos;
#[cfg(not(target_os = "macos"))]
const HOST: McpHost = McpHost::Linux;

impl McpFence {
    /// The fence every load stands under (SC5): standing, `Authority::load`
    /// refuses every `mcp` grant and a serving's SI2 record is kept but
    /// never refused. Only U9b lifts it, and before then only a test does,
    /// on its own thread ([`Lift`]); there is no runtime knob.
    #[cfg(not(test))]
    pub(crate) fn current() -> McpFence {
        McpFence::Standing
    }

    /// The fence on this thread: standing unless a [`Lift`] is held.
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
            (McpFence::Standing, _) => Ok(()),
            #[cfg(test)]
            (McpFence::Lifted, None) => Ok(()),
            #[cfg(test)]
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
/// and as each resume shape its adapter declares under the same hands,
/// while a gate is never offered one. The hands are the intended set's: the
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
        Some(McpIntent::Hands) => McpHands::Boxed,
        Some(McpIntent::Empty) if harnessed => McpHands::Harness,
        Some(McpIntent::Empty) => McpHands::NoHands,
        Some(McpIntent::NoModelSurface) | None => return Vec::new(),
    };
    let mut invocations = vec![McpInvocation::Cold];
    match class {
        SeatClass::Work => {
            invocations.push(McpInvocation::Replacement);
            let declared = adapter.into_iter().flat_map(|adapter| &adapter.resume.0);
            let declared = declared.filter(|(_, shape)| shape.hands == hands.word());
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
}

/// How much of `fragment` a plan types as the box's hands: all of it where
/// the intended set holds the hands server or the serving has no model
/// surface (exec's fragment is its box's, carried by no MCP server), and
/// none where the set is empty or no intent was recorded, whatever bytes the
/// fragment holds.
fn typed_hands(intent: Option<McpIntent>, fragment: &[String]) -> usize {
    match intent {
        Some(McpIntent::Hands | McpIntent::NoModelSurface) => fragment.len(),
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

/// An agent-backed site's sealed capability facts (decision 0065 ruling 5):
/// its asks resolved once per candidate of its chain. Under the harness
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
) -> Result<SiteCapabilities, CompileError> {
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
            }
        })
        .collect();
    // SI2 at each candidate, by its own composition and adapter: a fallback
    // never borrows its primary's record.
    let strict = chain
        .iter()
        .map(|candidate| {
            let intent = McpIntent::of_candidate(candidate);
            let adapter = adapter(adapters, &candidate.provider);
            strictness(intent, class.is_some(), asks.class, adapter)
        })
        .collect();
    resolved(authority, asks, &servings, strict)
}

/// An inline site's sealed capability facts: its asks resolved for the one
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
) -> Result<SiteCapabilities, CompileError> {
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
    };
    // Inline hands are boxed or none: the hands law refuses an inline model
    // harness's hands unboxed (decision 0046 ruling 4).
    let strict = strictness(inline.intent, false, asks.class, adapter(adapters, harness));
    resolved(
        authority,
        asks,
        std::slice::from_ref(&serving),
        vec![strict],
    )
}

/// `asks` resolved against each serving in order, each then judged by its
/// SI2 record in `strict`, or the first refusal; the records are kept
/// beside the outcomes, which way the fence stands.
fn resolved(
    authority: &Authority,
    asks: SiteAsks,
    servings: &[Serving<'_>],
    strict: Vec<Vec<(McpShape, McpAxis)>>,
) -> Result<SiteCapabilities, CompileError> {
    let mut outcomes = Vec::with_capacity(servings.len());
    for (serving, record) in servings.iter().zip(&strict) {
        outcomes.push(
            authority
                .resolve(&asks, serving)
                .map_err(CompileError::Capability)?,
        );
        let cause = record
            .iter()
            .find_map(|(shape, ambient)| ambient.strict(serving.provider, shape));
        authority.fence.admit(cause).map_err(|cause| {
            CompileError::Capability(format!("{}: {cause}", authority.who(&asks)))
        })?;
    }
    Ok(SiteCapabilities {
        asks,
        outcomes,
        strict,
    })
}
