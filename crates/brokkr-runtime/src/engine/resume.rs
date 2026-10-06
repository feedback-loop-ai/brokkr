//! Which session, if any, one model invocation site may rejoin
//! (proposed decision 0056, generalising decision 0030).
//!
//! Three private values and one pure query. `SiteKey` says WHICH site
//! this is — structurally, from the compiled body walk, never by
//! splitting a display tag on `:`. `InstanceKey` says WHOSE it is — the
//! adapter instance that would run it. `ConfirmedSession` is the
//! provider-confirmed root a rejoin stands on, which is a different
//! thing from decision 0032's retained transcript locator: a storage
//! directory is not a provider handle.
//!
//! The query reads this run's durable evidence and nothing else. It is a
//! derivation of the journal, the pinned bundle and the local origin —
//! never a policy input, never a new store, and never a model's claim
//! about its own identity.

use std::collections::{BTreeMap, BTreeSet};

use brokkr_core::{EventEnvelope, EventType};
use serde_json::{json, Map, Value};

use brokkr_core::realms::Boundary;

use crate::agents::Candidate;
use crate::bundle::{ExecutableBody, SeatClass, StepBody};

/// The domain separators. Two digests over two different structures must
/// never collide into each other, and a domain string is the cheapest
/// way to say so.
const SITE_DOMAIN: &str = "brokkr.resume-site/v1";
const INSTANCE_DOMAIN: &str = "brokkr.resume-instance/v1";

/// Where inside its seat one invocation site sits. The ancestry is
/// carried in distinct fields with their own indices, taken off the
/// compiled walk: `a:b`/`c` and `a`/`b:c` are two different values here,
/// which is exactly what the flattened display tag cannot say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SitePath {
    Single,
    PanelMember {
        member: String,
        member_index: usize,
    },
    SequenceStep {
        step: String,
        step_index: usize,
    },
    SequencePanelMember {
        step: String,
        step_index: usize,
        member: String,
        member_index: usize,
    },
}

impl SitePath {
    fn value(&self) -> Value {
        match self {
            SitePath::Single => json!({"kind": "single"}),
            SitePath::PanelMember {
                member,
                member_index,
            } => json!({
                "kind": "panel-member",
                "member": member,
                "member_index": member_index,
            }),
            SitePath::SequenceStep { step, step_index } => json!({
                "kind": "sequence-step",
                "step": step,
                "step_index": step_index,
            }),
            SitePath::SequencePanelMember {
                step,
                step_index,
                member,
                member_index,
            } => json!({
                "kind": "sequence-panel-member",
                "step": step,
                "step_index": step_index,
                "member": member,
                "member_index": member_index,
            }),
        }
    }
}

/// One structural invocation site: the outer seat, the selected case
/// (explicitly absent for a body with no selector), the body kind and
/// the full path. The run supplies its own scope — the journal this is
/// asked of is one run's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SiteKey {
    seat: String,
    /// `None` is an unselected body, and it is NOT the same key as a
    /// case literally named "default": absence is its own value here.
    case: Option<String>,
    body: &'static str,
    path: SitePath,
}

impl SiteKey {
    pub(super) fn single(seat: &str, case: Option<&str>) -> SiteKey {
        SiteKey {
            seat: seat.to_string(),
            case: case.map(str::to_string),
            body: "single",
            path: SitePath::Single,
        }
    }

    pub(super) fn panel_member(
        seat: &str,
        case: Option<&str>,
        member: &str,
        member_index: usize,
    ) -> SiteKey {
        SiteKey {
            seat: seat.to_string(),
            case: case.map(str::to_string),
            body: "panel",
            path: SitePath::PanelMember {
                member: member.to_string(),
                member_index,
            },
        }
    }

    pub(super) fn sequence_step(
        seat: &str,
        case: Option<&str>,
        step: &str,
        step_index: usize,
    ) -> SiteKey {
        SiteKey {
            seat: seat.to_string(),
            case: case.map(str::to_string),
            body: "sequence",
            path: SitePath::SequenceStep {
                step: step.to_string(),
                step_index,
            },
        }
    }

    pub(super) fn sequence_panel_member(
        seat: &str,
        case: Option<&str>,
        step: &str,
        step_index: usize,
        member: &str,
        member_index: usize,
    ) -> SiteKey {
        SiteKey {
            seat: seat.to_string(),
            case: case.map(str::to_string),
            body: "sequence",
            path: SitePath::SequencePanelMember {
                step: step.to_string(),
                step_index,
                member: member.to_string(),
                member_index,
            },
        }
    }

    /// Whether this site is the seat's own single body — the one shape
    /// decision 0030's legacy codex evidence can answer for, because a
    /// composite row's ancestry was never journaled.
    fn is_single(&self) -> bool {
        matches!(self.path, SitePath::Single)
    }

    fn value(&self) -> Value {
        json!({
            "domain": SITE_DOMAIN,
            "seat": self.seat,
            "case": self.case,
            "body": self.body,
            "path": self.path.value(),
        })
    }

    /// 64 lowercase hex, through the same canonical SHA-256 every other
    /// digest in this realm goes through.
    pub(super) fn digest(&self) -> String {
        brokkr_core::canonical::sha256_hex(&self.value())
    }
}

/// Everything about the adapter instance that would run this site, and
/// nothing about the invocation that reconstructs its values.
///
/// What is IN: the hire (agent, provider, model, effort, chain index),
/// the driver it spawns and the pinned unexpanded command template, the
/// adapter declaration the manifest pinned for this site, the engine,
/// the pinned bundle, the class, the boundary and the hands declaration.
///
/// What is deliberately OUT: this attempt's result path, the expanded
/// MCP settings filename and the capability token inside it. Their
/// DECLARATION is pinned — it is in the manifest, which is in `bundle`
/// below — while every invocation reconstructs the value, so comparing
/// the values would deny every offer. No resolved credential, no
/// provider-home content and no private argv enters this identity at
/// all: it is a comparison key, and a comparison key that carried a
/// secret would put it in a digest nobody can un-say.
pub(super) struct InstanceKey {
    value: Value,
}

impl InstanceKey {
    /// Build from the facts the engine holds at dispatch. `candidate` is
    /// `None` for an inline site, which uses its own pinned command and
    /// compiled facts instead of an agent chain.
    #[expect(clippy::too_many_arguments, reason = "baseline 2026-09, #288")]
    pub(super) fn new(
        candidate: Option<&Candidate>,
        chain_index: Option<usize>,
        command: &[String],
        manifest: &Value,
        site_label: &str,
        engine_version: &str,
        class: SeatClass,
        boundary: Option<Boundary>,
    ) -> InstanceKey {
        InstanceKey {
            value: json!({
                "domain": INSTANCE_DOMAIN,
                "agent": candidate.map(|c| c.agent.clone()),
                "provider": candidate.map(|c| c.provider.clone()),
                "model": candidate.map(|c| c.model.clone()),
                "effort": candidate.and_then(|c| c.effort.clone()),
                "chain_index": chain_index,
                // The driver this site spawns, normalized to the program
                // it names. `{brokkr}` is still a literal token here, so
                // this compares the TEMPLATE, not one machine's path.
                "driver": command.first().cloned(),
                "command": brokkr_core::canonical::sha256_hex(&json!(command)),
                // The adapter declaration the manifest pinned for this
                // site, where one was consulted (decision 0021). Absent
                // sites are covered by `bundle` below, which pins the
                // whole manifest.
                "adapter": manifest.pointer(&pointer("drivers", site_label)).cloned(),
                "engine": engine_version,
                "bundle": brokkr_core::canonical::sha256_hex(manifest),
                "class": match class {
                    SeatClass::Gate => "gate",
                    SeatClass::Work => "work",
                },
                "boundary": boundary.map(|word| word.word().to_string()),
                // The hands DECLARATION, from the manifest's pinned map —
                // not the fragment one invocation expanded.
                "hands": manifest.pointer(&pointer("hands", site_label)).cloned(),
            }),
        }
    }

    pub(super) fn digest(&self) -> String {
        brokkr_core::canonical::sha256_hex(&self.value)
    }
}

/// A JSON pointer into a manifest map keyed by site label. Labels carry
/// `:` and may in principle carry `/` or `~`, which are the two
/// characters RFC 6901 escapes.
fn pointer(map: &str, label: &str) -> String {
    format!("/{map}/{}", label.replace('~', "~0").replace('/', "~1"))
}

/// The engine's owned resume target: the provider root ID plus the
/// retained persistence locator the SAME confirmed checkpoint recorded
/// beside it (design D6). Only `provider_id` crosses the negotiated
/// `Body::Resume`; `persistence_locator` and the selected assessment
/// travel in the private `Start.input` context, where a provider like DSH
/// that needs both coordinates can read them and no prompt ever sees
/// them.
///
/// The two fields are read together because they are one fact: a locator
/// from a different row than the root would let a seat rejoin a directory
/// in which the offered ID was never opened. The locator is `None` for a
/// provider whose confirmed row carries no transcript reference; such a
/// target still offers its provider ID, and a planner that requires the
/// locator declines rather than guessing one. The same is true of the
/// recorded home: it is read off the same row, and a home-requiring
/// planner that finds none declines instead of borrowing another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResumeTarget {
    pub(super) provider_id: String,
    pub(super) persistence_locator: Option<String>,
    /// The admitted harness home the SAME checkpoint recorded beside the
    /// locator. It is the third coordinate of a DSH offer's complete
    /// address: a provider that must reopen the retained store compares
    /// this recorded home against the current admitted home before reading
    /// anything (design D6, Pass B completion).
    pub(super) persistence_home: Option<String>,
}

/// The harness facts the offered root was opened under, read off the same
/// confirmed checkpoint the offer came from. The version is what a
/// planner compares with its measured identity; the wrapper digest is the
/// optional composite identity a wrapper-shaped provider (DSH) records,
/// which an offered root without one can never match (design D6). The
/// locator is not here: it travels on the `ResumeTarget` the same row
/// produced, and nothing reads it twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginatingRoot {
    pub(super) harness_version: Option<String>,
    pub(super) wrapper_digest: Option<String>,
}

/// Read the offered root's own facts from the NEWEST checkpoint that
/// carries confirmed root evidence for this exact site. This is the same
/// row `eligible_offer` judges, scanned once more so the version and the
/// optional wrapper digest describe one session rather than whichever
/// checkpoint happened to be newest per field.
pub(super) fn originating_root(
    events: &[EventEnvelope],
    site_ref: &str,
) -> Option<OriginatingRoot> {
    let checkpoint = events
        .iter()
        .rev()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .find_map(|event| {
            let checkpoint = event.payload.get("checkpoint")?;
            if checkpoint.get("site_ref").and_then(Value::as_str) != Some(site_ref) {
                return None;
            }
            checkpoint.get("root_session")?;
            Some(checkpoint)
        })?;
    Some(OriginatingRoot {
        harness_version: checkpoint
            .pointer("/root_session/harness_version")
            .and_then(Value::as_str)
            .map(str::to_string),
        wrapper_digest: checkpoint
            .pointer("/root_session/wrapper_digest")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

/// The provider-confirmed root a launch stood on, as it reaches the
/// journal under seat-record v5. Read back here, never invented: a row
/// without one supplies no offer, whatever else it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConfirmedSession {
    pub(super) id: String,
    pub(super) persistent: bool,
}

impl ConfirmedSession {
    fn read(root: &Value) -> Option<ConfirmedSession> {
        Some(ConfirmedSession {
            id: root.get("id").and_then(Value::as_str)?.to_string(),
            // Absent persistence is not "probably persistent". A root
            // whose shape never established the answer cannot be
            // offered.
            persistent: root.get("persistent").and_then(Value::as_bool)?,
        })
    }
}

/// One executing model site's identity, carried from where the site is
/// composed to where its checkpoints are journaled and its offer is
/// decided. Panel workers hand the key beside their checkpoints to the
/// single journal writer for stamping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SiteContext {
    pub(super) site_ref: String,
    pub(super) instance_ref: String,
    /// Work sites may be offered a session; gate sites never are
    /// (decision 0042 ruling 2, bounding decision 0030's continuity).
    pub(super) class: SeatClass,
    /// The offered root's history at this site; empty unless offered.
    pub(super) history: RootHistory,
}

/// What the offered root's earlier attempts journaled at one site
/// (decision 0065 slice two, U4f; CC1): each attempt whose stamped row
/// there confirmed that root, and every native call id those attempts'
/// rows there carry. A call id is owned by its attempt, so the consumer
/// recomputes a replayed call's id under each of these attempts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct RootHistory {
    pub(super) attempts: BTreeSet<String>,
    pub(super) calls: BTreeSet<String>,
}

impl SiteContext {
    /// A site's identity, before any offer is decided for it.
    pub(super) fn new(site_ref: String, instance_ref: String, class: SeatClass) -> SiteContext {
        SiteContext {
            site_ref,
            instance_ref,
            class,
            history: RootHistory::default(),
        }
    }

    /// This context carrying the history of `offer`'s root at this site,
    /// read from this run's journal alone. A site offered nothing, or a
    /// root with no stamped row here (decision 0030's legacy evidence),
    /// has none: its every call is new.
    pub(super) fn offered(self, events: &[EventEnvelope], offer: Option<&ResumeTarget>) -> Self {
        let Some(offer) = offer else {
            return self;
        };
        let rows = || {
            events
                .iter()
                .filter(|event| event.event_type == EventType::EffectCheckpointed)
                .filter_map(|event| {
                    let checkpoint = event.payload.get("checkpoint")?;
                    let site = checkpoint.get("site_ref").and_then(Value::as_str)?;
                    let attempt = event.attempt_id.as_deref()?;
                    (site == self.site_ref).then_some((attempt, checkpoint))
                })
        };
        let attempts: BTreeSet<String> = rows()
            .filter(|(_, row)| {
                row.pointer("/root_session/id").and_then(Value::as_str)
                    == Some(offer.provider_id.as_str())
            })
            .map(|(attempt, _)| attempt.to_string())
            .collect();
        let calls = rows()
            .filter(|(attempt, _)| attempts.contains(*attempt))
            .filter_map(|(_, row)| row.get("call_id").and_then(Value::as_str))
            .map(str::to_string)
            .collect();
        let history = RootHistory { attempts, calls };
        SiteContext { history, ..self }
    }

    /// The stamp (proposed decision 0056 ruling 1), the same shape
    /// `stamp_boundary` has: the engine's two structural facts replace
    /// whatever a driver wrote, on a record that names a model. Every
    /// driver Brokkr ships names one on every row it forwards — the
    /// served model, `not reported` before a harness echoes one, `not
    /// applicable` on an exec row — so every driver row of a planned site
    /// carries both stamps, the launch row with its confirmed root among
    /// them. A record that names none is a third-party driver's or the
    /// engine's own; it carries neither, and a driver's value on such a
    /// record is dropped.
    pub(super) fn stamp(&self, record: Value) -> Value {
        match record {
            Value::Object(mut object) => {
                if object.contains_key("model") {
                    object.insert("site_ref".into(), Value::String(self.site_ref.clone()));
                    object.insert(
                        "instance_ref".into(),
                        Value::String(self.instance_ref.clone()),
                    );
                } else {
                    object.remove("site_ref");
                    object.remove("instance_ref");
                }
                Value::Object(object)
            }
            other => other,
        }
    }
}

/// Strip the two stamps from a record no site context owns — an
/// aggregate the engine writes itself, or an exec row. An aggregate
/// never establishes a member's session ownership, so it must not carry
/// a member's stamp either.
pub(super) fn unstamped(record: Value) -> Value {
    match record {
        Value::Object(mut object) => {
            object.remove("site_ref");
            object.remove("instance_ref");
            Value::Object(object)
        }
        other => other,
    }
}

/// The eligibility question, answered from durable evidence alone.
///
/// The order is the order the checks fail closed in:
///
/// 1. a gate site is never offered a session, whatever its journal says;
/// 2. the run must have been started on this installation, and the
///    bundle this attempt runs under must be the one the run pinned;
/// 3. the NEWEST checkpoint carrying confirmed root evidence for this
///    exact `site_ref` is the only candidate — an incompatible or
///    ambiguous newest owner denies the offer, and the query never
///    searches behind it for an older matching owner;
/// 4. that row's owner must be this instance, its root must persist, and
///    its attempt must resolve to this run's requested seat.
///
/// Only when every one of those holds does the handle travel; the last
/// resort is decision 0030's narrow legacy codex path, and only for a
/// single work seat whose ancestry was never in question.
pub(super) fn eligible_offer(
    events: &[EventEnvelope],
    key: &SiteKey,
    context: &SiteContext,
    seat: &str,
    started_here: bool,
    pinned_bundle_holds: bool,
    legacy: impl Fn(&EventEnvelope) -> bool,
) -> Option<ResumeTarget> {
    if context.class == SeatClass::Gate || !started_here || !pinned_bundle_holds {
        return None;
    }
    let seat_effects = effects_of_seat(events, seat);
    let mut stamped = events
        .iter()
        .rev()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .filter(|event| in_effects(event, &seat_effects))
        .filter_map(|event| {
            let checkpoint = event.payload.get("checkpoint")?;
            let site = checkpoint.get("site_ref").and_then(Value::as_str)?;
            (site == context.site_ref).then_some((event, checkpoint))
        })
        .peekable();
    if stamped.peek().is_some() {
        // A site with stamped evidence is judged on that evidence alone:
        // it never falls back to decision 0030's unstamped rows, however
        // the judgment below turns out.
        //
        // The candidate is the NEWEST stamped row that carries a root.
        // Every driver reports a model on every row it emits, so the
        // seat-turns and the finishing record that follow a launch are
        // stamped too; they are the attempt's telemetry, not a change of
        // owner, and the launch row behind them is what the offer stands
        // on (design D3). A site whose stamped rows carry no root at all
        // has no offer.
        let (event, checkpoint, root) = stamped.find_map(|(event, checkpoint)| {
            checkpoint
                .get("root_session")
                .map(|root| (event, checkpoint, root))
        })?;
        // The newest owner is the only owner asked. A session opened by
        // a candidate this site has since moved past is not resurrected
        // by looking further back, and an unstamped owner is ambiguous
        // rather than absent.
        if checkpoint.get("instance_ref").and_then(Value::as_str) != Some(&context.instance_ref) {
            return None;
        }
        let root = ConfirmedSession::read(root)?;
        if !root.persistent {
            return None;
        }
        // The attempt this row belongs to must be one of this seat's own
        // durable starts. `in_effects` already bound the effect to the
        // seat; this binds the row to a start that actually happened,
        // rather than to a checkpoint whose start never reached the
        // journal.
        let attempt = event.attempt_id.as_deref()?;
        started_attempt(events, attempt)?;
        // The locator comes off the SAME checkpoint as the root: a
        // provider that needs both coordinates (DSH) may only rejoin the
        // directory in which this exact ID was opened. A codex/claude row
        // that confirms from its own locator needs none, and a row that
        // carries none still offers its provider ID.
        return Some(ResumeTarget {
            provider_id: root.id,
            persistence_locator: checkpoint
                .pointer("/transcript/locator")
                .and_then(Value::as_str)
                .filter(|locator| !locator.is_empty())
                .map(str::to_string),
            persistence_home: checkpoint
                .pointer("/transcript/home")
                .and_then(Value::as_str)
                .filter(|home| !home.is_empty())
                .map(str::to_string),
        });
    }
    // Decision 0030's evidence, kept readable (design D8). Only an
    // unambiguous local single WORK seat qualifies: a composite row's
    // ancestry was never journaled, so no colon tag is split to guess
    // it, and a dsh transcript directory is not a provider handle.
    if !key.is_single() {
        return None;
    }
    legacy_offer(events, &seat_effects, &legacy).map(|provider_id| ResumeTarget {
        // The legacy shape's locator IS the session it hands over
        // (decision 0032's codex-thread locator, or the old flat id), so
        // the two coordinates coincide — and no transcript home was ever
        // recorded, so a home-requiring planner declines rather than
        // borrowing the current environment's.
        persistence_locator: Some(provider_id.clone()),
        provider_id,
        persistence_home: None,
    })
}

/// The effect ids this seat requested, newest last.
fn effects_of_seat(events: &[EventEnvelope], seat: &str) -> Vec<String> {
    events
        .iter()
        .filter(|event| event.event_type == EventType::EffectRequested)
        .filter(|event| event.payload.get("seat").and_then(Value::as_str) == Some(seat))
        .filter_map(|event| {
            event
                .payload
                .get("effect_id")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect()
}

fn in_effects(event: &EventEnvelope, effects: &[String]) -> bool {
    event
        .payload
        .get("effect_id")
        .and_then(Value::as_str)
        .is_some_and(|effect_id| effects.iter().any(|known| known == effect_id))
}

fn started_attempt<'a>(events: &'a [EventEnvelope], attempt: &str) -> Option<&'a Value> {
    events
        .iter()
        .rev()
        .filter(|event| event.event_type == EventType::EffectStarted)
        .find(|event| event.attempt_id.as_deref() == Some(attempt))
        .map(|event| &event.payload)
}

/// The narrow compatibility path: the last checkpoint of this seat that
/// carries an old-shaped session locator AND no site stamp, offered only
/// when the caller's predicate accepts the attempt that wrote it. The
/// predicate is decision 0030's instance comparison — the `driver` label
/// and the `provenance` of the opening attempt against this one's
/// (`legacy_instance_holds`); it holds no kind or version test, because
/// the evidence this path reads predates both. The two shapes it reads
/// are decision 0032's `codex-thread` locator and the flat `session_id`
/// a run opened before that ruling wrote — the flat id names no kind,
/// exactly as it did not when the shipped engine offered it. A row that
/// carries a site stamp is NEW evidence and was already judged above; it
/// may not fall back to legacy fields to evade the confirmation it
/// failed.
fn legacy_offer(
    events: &[EventEnvelope],
    seat_effects: &[String],
    legacy: &impl Fn(&EventEnvelope) -> bool,
) -> Option<String> {
    let (attempt, session) = events
        .iter()
        .rev()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .filter(|event| in_effects(event, seat_effects))
        .find_map(|event| {
            let checkpoint = event.payload.get("checkpoint")?;
            if checkpoint.get("site_ref").is_some() {
                return None;
            }
            // Decision 0032's common shape first, then the old flat id a
            // run opened before that ruling still carries.
            let session = checkpoint
                .pointer("/transcript/locator")
                .and_then(Value::as_str)
                .filter(|locator| !locator.is_empty())
                .filter(|_| {
                    checkpoint
                        .pointer("/transcript/kind")
                        .and_then(Value::as_str)
                        == Some("codex-thread")
                })
                .or_else(|| checkpoint.get("session_id").and_then(Value::as_str))?;
            let attempt = event.attempt_id.clone()?;
            legacy(event).then(|| (attempt, session.to_string()))
        })?;
    started_attempt(events, &attempt).map(|_| session)
}

/// The shipped instance comparison decision 0030 wrote, kept for the
/// legacy path alone: the `driver` label and the `provenance` this
/// attempt is about to journal against the ones the attempt that opened
/// the session did. New evidence uses `instance_ref`, which does not
/// move when a SIBLING member's chain moves.
pub(super) fn legacy_instance_holds(opened_by: &Value, started: &Value) -> bool {
    ["driver", "provenance"]
        .iter()
        .all(|field| opened_by.get(field) == started.get(field))
}

/// One invocation site of one selected body, with both its addresses:
/// the flat tag the engine's existing maps key on, and the structural
/// key the resume digest is built from.
pub(crate) struct StructuralSite {
    /// The tag `invocation_sites` produces — `None`, `<member>`,
    /// `<step>` or `<step>:<member>`. Retained for compatibility: every
    /// existing lookup, journal field and readout still keys on it.
    pub(crate) site: Option<String>,
    pub(crate) key: SiteKey,
    /// Which side of decision 0021 ruling 1 this site sits on, read
    /// where the compiler keeps it: the seat's selected class for a
    /// single or a panel member, the step's own class for a step, and
    /// the enclosing step's class for a member inside a step panel.
    pub(crate) class: SeatClass,
    /// The pinned unexpanded command template for the site, used only to
    /// build its instance identity.
    pub(crate) command: Vec<String>,
}

/// Every model invocation site of one selected body, structurally.
///
/// A dialect step is deliberately absent: it has no model turn, so it
/// receives no offer, negotiates nothing and publishes no launch
/// (proposed decision 0056 ruling 1).
pub(crate) fn structural_sites(
    body: ExecutableBody<'_>,
    seat: &str,
    case: Option<&str>,
    gate: bool,
) -> Vec<StructuralSite> {
    let seat_class = if gate {
        SeatClass::Gate
    } else {
        SeatClass::Work
    };
    match body {
        ExecutableBody::Single { command, .. } => vec![StructuralSite {
            site: None,
            key: SiteKey::single(seat, case),
            class: seat_class,
            command: command.to_vec(),
        }],
        ExecutableBody::Panel { members, .. } => members
            .iter()
            .enumerate()
            .map(|(index, member)| StructuralSite {
                site: Some(member.name.clone()),
                key: SiteKey::panel_member(seat, case, &member.name, index),
                class: seat_class,
                command: member.command.clone(),
            })
            .collect(),
        ExecutableBody::Sequence { steps } => steps
            .iter()
            .enumerate()
            .flat_map(|(step_index, step)| match &step.body {
                StepBody::Single { command, .. } => vec![StructuralSite {
                    site: Some(step.name.clone()),
                    key: SiteKey::sequence_step(seat, case, &step.name, step_index),
                    class: step.class,
                    command: command.clone(),
                }],
                StepBody::Panel { members, .. } => members
                    .iter()
                    .enumerate()
                    .map(|(member_index, member)| StructuralSite {
                        site: Some(format!("{}:{}", step.name, member.name)),
                        key: SiteKey::sequence_panel_member(
                            seat,
                            case,
                            &step.name,
                            step_index,
                            &member.name,
                            member_index,
                        ),
                        // A member inherits its enclosing panel's class,
                        // and an office name decides nothing.
                        class: step.class,
                        command: member.command.clone(),
                    })
                    .collect(),
                StepBody::Dialect { .. } => Vec::new(),
            })
            .collect(),
    }
}

/// Every execution coordinate of one body as its relative tag and
/// structural owner, INCLUDING deterministic dialect steps (design D10
/// F2). `structural_sites` deliberately omits a dialect leaf because it
/// receives no model offer; ownership must not, because a literal phase
/// can flatten to the injected validator's label and one site would then
/// answer for the other.
pub(crate) fn owner_sites(
    body: ExecutableBody<'_>,
    seat: &str,
    case: Option<&str>,
) -> Vec<(Option<String>, SiteKey)> {
    match body {
        ExecutableBody::Single { .. } => vec![(None, SiteKey::single(seat, case))],
        ExecutableBody::Panel { members, .. } => members
            .iter()
            .enumerate()
            .map(|(index, member)| {
                (
                    Some(member.name.clone()),
                    SiteKey::panel_member(seat, case, &member.name, index),
                )
            })
            .collect(),
        ExecutableBody::Sequence { steps } => steps
            .iter()
            .enumerate()
            .flat_map(|(step_index, step)| match &step.body {
                StepBody::Single { .. } => vec![(
                    Some(step.name.clone()),
                    SiteKey::sequence_step(seat, case, &step.name, step_index),
                )],
                StepBody::Panel { members, .. } => members
                    .iter()
                    .enumerate()
                    .map(|(member_index, member)| {
                        (
                            Some(format!("{}:{}", step.name, member.name)),
                            SiteKey::sequence_panel_member(
                                seat,
                                case,
                                &step.name,
                                step_index,
                                &member.name,
                                member_index,
                            ),
                        )
                    })
                    .collect(),
                StepBody::Dialect { .. } => vec![(
                    Some(step.name.clone()),
                    SiteKey::sequence_step(seat, case, &step.name, step_index),
                )],
            })
            .collect(),
    }
}

/// The compiled-address uniqueness check (design D2). `Selection`,
/// `argv_for` and `bundle.hands` all key on the FLATTENED site label, so
/// two structurally different sites that flatten to one string can
/// select each other's candidate, hands identity and boundary — before
/// any resume digest is built. Ordinary repeated member names under
/// different steps are untouched: `review`/`alpha` and `design`/`alpha`
/// flatten to `review:alpha` and `design:alpha`, which are distinct.
///
/// Returns the colliding label and a description of the two structural
/// sites that claim it, so the refusal names both rather than one.
pub(crate) fn flat_address_collision(sites: &[StructuralSite]) -> Option<(String, String)> {
    let mut seen: BTreeMap<&str, &SiteKey> = BTreeMap::new();
    for entry in sites {
        let label = entry.site.as_deref().unwrap_or("");
        match seen.get(label) {
            Some(first) if **first != entry.key => {
                return Some((label.to_string(), describe_pair(first, &entry.key)));
            }
            Some(_) => {}
            None => {
                seen.insert(label, &entry.key);
            }
        }
    }
    None
}

fn describe_pair(first: &SiteKey, second: &SiteKey) -> String {
    format!("{} and {}", describe(first), describe(second))
}

pub(crate) fn describe(key: &SiteKey) -> String {
    match &key.path {
        SitePath::Single => format!("the single body of seat '{}'", key.seat),
        SitePath::PanelMember { member, .. } => {
            format!("panel member '{member}' of seat '{}'", key.seat)
        }
        SitePath::SequenceStep { step, .. } => format!("step '{step}' of seat '{}'", key.seat),
        SitePath::SequencePanelMember { step, member, .. } => {
            format!("member '{member}' of step '{step}' of seat '{}'", key.seat)
        }
    }
}

/// The engine's binding of a seat's single `--patch` value to the
/// compiled bundle (design D6 mechanism 1; AS3): the argv value the seat
/// spawns and the compiled manifest's recorded digest of the leaf-layer
/// `files` member it names. The engine computes it where it builds the
/// private start context, at both `start_context` call sites, and never
/// from a hash of the file the value resolves to — so a member edited
/// since compilation still travels with the manifest's digest and the
/// adapter's required hash comparison is what refuses its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RouteOverlay {
    pub(super) value: String,
    pub(super) digest: String,
}

/// The private start context an adapter reads beside its prompt: which
/// resume assessment was selected for this site, the harness facts the
/// offered root was opened under, the owned target (provider ID plus
/// its persistence locator) that a two-coordinate provider needs, and
/// the engine's route-overlay binding where the seat's argv carries one
/// `--patch`. It rides the existing `Start.input` object under one key,
/// never the rendered prompt.
pub(super) fn start_context(
    assessment: Value,
    originating: Option<&OriginatingRoot>,
    target: Option<&ResumeTarget>,
    route_overlay: Option<&RouteOverlay>,
) -> Value {
    let mut context = Map::new();
    context.insert("assessment".into(), assessment);
    if let Some(originating) = originating {
        if let Some(version) = &originating.harness_version {
            context.insert(
                "originating_harness_version".into(),
                Value::String(version.clone()),
            );
        }
        if let Some(digest) = &originating.wrapper_digest {
            context.insert(
                "originating_wrapper_digest".into(),
                Value::String(digest.clone()),
            );
        }
    }
    if let Some(target) = target {
        context.insert(
            "owned_target".into(),
            json!({
                "provider_id": target.provider_id,
                "persistence_locator": target.persistence_locator,
                "persistence_home": target.persistence_home,
            }),
        );
    }
    if let Some(binding) = route_overlay {
        context.insert(
            "route_overlay".into(),
            json!({
                "value": binding.value,
                "digest": binding.digest,
            }),
        );
    }
    Value::Object(context)
}

#[cfg(test)]
mod tests;
