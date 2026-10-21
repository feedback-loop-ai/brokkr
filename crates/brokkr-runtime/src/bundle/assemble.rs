//! The phases of [`Bundle::assemble`](super::Bundle) after the table
//! (#288, slice F; the table's own is `phases::table`): what the seats
//! open, the seat loop and each seat's results, driver class, limits and
//! inputs, the model pins, the capability pass, the dialect `verify`
//! wrapper, and the manifest's projections of the one canonical site
//! family. `assemble` reads as their sequence.

use std::collections::BTreeMap;
use std::path::Path;

use brokkr_core::policy::{is_declarable, Machine};
use brokkr_core::realms::Boundary;
use brokkr_protocol::hands::HandsSpec;
use serde_json::{json, Map, Value};

use super::parse::{SeatScope, SiteCompiler};
use super::site_form::{classify, SEAT};
use super::{
    body_manifest, claim_address, compose, dialect_gate_site, dialect_results, enforce_model_pins,
    enforce_model_policy, fold_driver_facts, is_engine_owned, owner_index, parse_limits,
    parse_secrets, phases, pins, record_capabilities, record_hands, record_judged_tools,
    refuse_aliasing_sites, refuse_class_without_a_driver, refuse_global_aliasing,
    relocate_verify_facts, resolved_hands, site_facts, CapabilityWalk, CharterPins, Charters,
    CompileError, Dialect, DialectExecution, ExecutableBody, Invocation, LibraryRead, Limits,
    Opened, ResolvedSeat, Seat, SeatBody, SeatClass, SequenceStep, SiteFacts, SiteKey, SiteLaw,
    StepBody,
};
use crate::agents::EgressClass;
use crate::argv::Executable;
use crate::capabilities::{Authority, Definitions};
use crate::dialect::Command;
use crate::phase_name;

/// One authoring owner the wrapper moves: its label, its destination under
/// `checks`, and its key.
type Moved = (String, String, SiteKey);

/// The bundle document's top-level `key` text, empty when it writes none.
pub(super) fn config_text(config: &Value, key: &str) -> String {
    config
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// What the COMPOSED seats open, each when a site first needs it: a base
/// may be what carries the agent reference. A dialect phase's wrapper and
/// steps consult the adapters, so they open here.
pub(super) fn open_agents<'a>(
    roots: (&'a Path, &'a Path),
    definitions: &'a Definitions,
    egress_minimum: EgressClass,
    provisional_offices: Vec<String>,
    uses_dialect: bool,
) -> Result<Opened<'a>, CompileError> {
    let mut agents = Opened::new(roots, definitions, egress_minimum, provisional_offices);
    if uses_dialect {
        agents.adapters()?;
    }
    Ok(agents)
}

/// The dialect's `verify` wrapper, applied only AFTER the authoring
/// census (design D10 F2). Wrapping renames the wrapped seat's whole
/// subtree, so a collision it creates or disguises must be decided against
/// every authoring owner before any fact moves.
pub(super) fn wrapped_verify(uses_dialect: bool, dialect: Option<&Dialect>) -> Option<&Command> {
    if uses_dialect {
        dialect.and_then(|dialect| match &dialect.verify {
            crate::dialect::CommandOrUnsupported::Command(command) => Some(command),
            crate::dialect::CommandOrUnsupported::Unsupported(_) => None,
        })
    } else {
        None
    }
}

/// Every composed seat, each through [`compile_seat`], with the resolved
/// agent hands the wrapped verify seat carried.
pub(super) fn compile_seats(
    compiler: &mut SiteCompiler<'_>,
    machine: &Machine,
    resolved: &compose::Resolved,
    wrapped_verify: Option<&Command>,
) -> Result<(BTreeMap<String, Seat>, Option<HandsSpec>), CompileError> {
    // The resolved agent hands the wrapped verify seat carried, kept for
    // the synthetic validator's hands law in the wrapper pass. Only
    // `verify` is ever wrapped, so one slot answers.
    let mut verify_agent_hands: Option<HandsSpec> = None;

    let mut seats = BTreeMap::new();
    for (phase, raw) in &resolved.seats {
        // An inherited seat's `role` and `./`-prefixed argv resolve
        // against the layer that WROTE them, found by name — the
        // resolver never looked inside the seat to learn this.
        let dir = &resolved.roots[resolved.seat_origin[phase]];
        if !machine.phases.contains(phase) {
            return Err(CompileError::Invalid(format!(
                "seat '{phase}' names a phase the policy does not have"
            )));
        }
        let seat = compile_seat(
            compiler,
            machine,
            dir,
            phase,
            raw,
            wrapped_verify,
            &mut verify_agent_hands,
        )?;
        seats.insert(phase.clone(), seat);
    }
    Ok((seats, verify_agent_hands))
}

/// Decision 0091 refusal 4, where a folder supplied a seat: the recipe's
/// own declaration of it, which the folder's driver and hands replaced
/// before any parser read it, compiles as a compile with no folder
/// compiles it, by the same seat reading ([`compile_seat`]) under the same
/// boundary, adapters and engine, on a scratch compiler whose facts are
/// dropped. Its class, results and limits are the seat's and compiled
/// already, so what this refuses is its driver or its hands, with refusal
/// 4's text. The passes after the seat loop read a model's candidates or a
/// site's asks, and such a seat names no model, as it writes no `role` and
/// a role-less site runs `exec` alone (bound for a model driver by
/// `a_folder_seat_whose_own_driver_a_compile_with_no_folder_refuses_refuses`),
/// and asks what its class asks, as the folder's declaration of the same
/// class does in this compile.
pub(super) fn hold_own_drivers(
    compiler: &SiteCompiler<'_>,
    machine: &Machine,
    resolved: &compose::Resolved,
    wrapped_verify: Option<&Command>,
) -> Result<(), CompileError> {
    for (phase, own) in &resolved.own_seats {
        let dir = &resolved.roots[own.origin];
        let (mut scratch, mut hands) = (compiler.scratch(), None);
        let compiled = compile_seat(
            &mut scratch,
            machine,
            dir,
            phase,
            &own.raw,
            wrapped_verify,
            &mut hands,
        );
        if compiled.is_err() {
            let seat = phase.clone();
            return Err(CompileError::Folder(
                compose::FolderSeatRefusal::NoOwnDriver { seat },
            ));
        }
    }
    Ok(())
}

/// One seat of a phase the policy has: its form, results, body, driver
/// class, limits and inputs.
fn compile_seat(
    compiler: &mut SiteCompiler<'_>,
    machine: &Machine,
    dir: &Path,
    phase: &str,
    raw: &Value,
    wrapped_verify: Option<&Command>,
    verify_agent_hands: &mut Option<HandsSpec>,
) -> Result<Seat, CompileError> {
    let shape = classify(phase, raw, &SEAT)?;
    let results = seat_results(machine, phase, raw)?;
    let secrets = parse_secrets(phase, raw)?;
    let at = SeatScope {
        dir,
        results: &results,
        secrets: &secrets,
    };
    let (agent_seat, body, sequence_gate) = compiler.parse_seat_body(at, phase, raw, shape)?;
    // The dialect wrapper is applied after this loop, once every
    // authoring owner is registered (design D10 F2). Record the
    // agent hands the wrapped seat resolved so the synthetic
    // validator's hands law sees the same site it always did.
    if phase == phase_name::VERIFY && wrapped_verify.is_some() {
        *verify_agent_hands = agent_seat.as_ref().and_then(resolved_hands);
    }
    judge_seat_driver(compiler, at, phase, raw, &body, &agent_seat, wrapped_verify)?;
    let limits = seat_limits(phase, raw, &agent_seat)?;
    let inputs = seat_inputs(machine, phase, raw, &agent_seat)?;
    Ok(Seat {
        has_gate: sequence_gate.unwrap_or_else(|| body.is_gate()),
        results,
        limits,
        inputs,
        secrets,
        body,
    })
}

/// A seat's non-empty `results`, each covered by a rule from its phase.
fn seat_results(machine: &Machine, phase: &str, raw: &Value) -> Result<Vec<String>, CompileError> {
    let results = raw
        .get("results")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|r| !r.is_empty())
        .ok_or_else(|| {
            CompileError::Invalid(format!("seat '{phase}' needs non-empty 'results'"))
        })?;
    for result in &results {
        let covered = machine
            .rules
            .iter()
            .any(|rule| rule.from == *phase && rule.result == *result);
        // Decision 0041 reserves the implementer's `oversized`
        // verdict before ruling 6 seats triage. Until that phase
        // exists there can be no edge to it; the unmatched verdict
        // parks fail-closed. Once triage exists, ordinary coverage
        // is mandatory and the roster test pins both bounded arms.
        let reserved_oversized = phase == phase_name::IMPLEMENT
            && result == "oversized"
            && !machine
                .phases
                .iter()
                .any(|known| known == phase_name::TRIAGE);
        if !covered && !reserved_oversized {
            return Err(CompileError::Invalid(format!(
                "seat '{phase}' may emit '{result}' but no rule covers it; \
                 result variants without an outer rule are rejected"
            )));
        }
    }
    Ok(results)
}

/// Decision 0021, at the seat's own driver-bearing site. A panel or a
/// sequence has none: its members and steps were each checked where they
/// were built.
fn judge_seat_driver(
    compiler: &mut SiteCompiler<'_>,
    at: SeatScope<'_>,
    phase: &str,
    raw: &Value,
    body: &SeatBody,
    agent_seat: &Option<ResolvedSeat>,
    wrapped_verify: Option<&Command>,
) -> Result<(), CompileError> {
    match body {
        SeatBody::Invocation(Invocation::Single { candidates, .. }) => {
            let hands = agent_seat.as_ref().and_then(resolved_hands);
            compiler.judge(at, phase, raw, candidates, hands)?;
        }
        SeatBody::Select { .. } => refuse_class_without_a_driver(phase, raw)?,
        // A to-be-wrapped seat's class belongs to its members or
        // its synthetic validator, exactly as it did when the
        // wrapper ran inline; the wrapper pass applies the same
        // refusal for a shape it cannot wrap.
        _ if phase == phase_name::VERIFY && wrapped_verify.is_some() => {}
        _ => refuse_class_without_a_driver(phase, raw)?,
    }
    Ok(())
}

/// Decision 0006 bounds belong to the strategy seat, not the office it
/// hires. A roster entry supplies the default, while an explicit seat
/// limit (night-shift's one-attempt gates) wins.
fn seat_limits(
    phase: &str,
    raw: &Value,
    agent_seat: &Option<ResolvedSeat>,
) -> Result<Limits, CompileError> {
    let limits = if raw.get("limits").is_some() {
        parse_limits(phase, raw)?
    } else {
        match agent_seat.as_ref().and_then(|agent_seat| agent_seat.limits) {
            Some(limits) => limits,
            None => parse_limits(phase, raw)?,
        }
    };
    Ok(limits)
}

/// Input provenance (decision 0007): every input the phase's rules
/// reference must be engine-computed or supplied by this seat's
/// declaration; a declaration may only name known, non-engine-owned
/// evaluator inputs. An agent-resolved seat faces this lint exactly as an
/// inline one does — its declaration is the agent's, or the 0007 default.
fn seat_inputs(
    machine: &Machine,
    phase: &str,
    raw: &Value,
    agent_seat: &Option<ResolvedSeat>,
) -> Result<Vec<String>, CompileError> {
    let referenced = machine.seat_inputs(phase);
    let raw_inputs = match agent_seat
        .as_ref()
        .and_then(|agent_seat| agent_seat.inputs.clone())
    {
        Some(declared) => Some(json!(declared)),
        None => raw.get("inputs").cloned(),
    };
    let inputs = match raw_inputs {
        None => referenced.clone(),
        Some(declared) => {
            let declared = declared
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .ok_or_else(|| {
                    CompileError::Invalid(format!(
                        "seat '{phase}' inputs must be an array of strings"
                    ))
                })?;
            for name in &declared {
                if is_engine_owned(name) {
                    return Err(CompileError::Invalid(format!(
                        "seat '{phase}' declares engine-owned input '{name}'; \
                         journal-computed truth is never accepted from a seat"
                    )));
                }
                if !is_declarable(name) {
                    return Err(CompileError::Invalid(format!(
                        "seat '{phase}' declares unknown input '{name}'; known: \
                         the evaluator's closed vocabulary minus engine-owned"
                    )));
                }
            }
            for needed in &referenced {
                if !declared.contains(needed) {
                    return Err(CompileError::Invalid(format!(
                        "phase '{phase}' rules reference input '{needed}' but \
                         seat '{phase}' does not declare it; the rule could \
                         never fire from seat data"
                    )));
                }
            }
            declared
        }
    };
    Ok(inputs)
}

/// Decision 0031 over the TYPED, flattened seats before the wrapper moves
/// an address: one refusal names the whole repair set, and the maps it
/// returns seed the manifest's `drivers` pin and the resume.
fn pin_drivers(
    seats: &BTreeMap<String, Seat>,
    agents: &Opened<'_>,
    sites: &mut BTreeMap<String, SiteFacts>,
) -> Result<(), CompileError> {
    let (pin_drivers, inline_resume, resume_witness, inline_hands_notice) =
        enforce_model_pins(seats, agents.loaded())?;
    for (label, value) in pin_drivers.into_iter().chain(resume_witness) {
        site_facts(sites, &label).pin_drivers = Some(value);
    }
    for (label, value) in inline_resume {
        site_facts(sites, &label).inline_resume = Some(value);
    }
    for (label, notice) in inline_hands_notice {
        site_facts(sites, &label).inline_hands_notice = Some(notice);
    }
    Ok(())
}

/// Decision 0065 ruling 5, over the COMPOSED seats and before the wrapper
/// moves anything: every executable site — seat, member, step, selected
/// body, inherited or not — resolves office asks minus seat subtractions
/// against the realm's grants, once per provider candidate, and the
/// outcome lands in the one canonical site family so relocation carries it
/// like every other fact.
fn record_seat_capabilities(
    authority: &Authority,
    agents: &Opened<'_>,
    at: (Boundary, &Executable),
    resolved: &compose::Resolved,
    sites: &mut BTreeMap<String, SiteFacts>,
) -> Result<(), CompileError> {
    let walk = CapabilityWalk::of(authority, agents, at, resolved);
    for (phase, raw) in &resolved.seats {
        // The layer that wrote the seat, as its agent's hands
        // segment is expanded against (review return F1).
        let dir = &resolved.roots[resolved.seat_origin[phase]];
        record_capabilities(&walk, dir, phase, raw, sites)?;
    }
    Ok(())
}

/// One compile once its seats are read, in one parameter object (decision
/// 0071 ruling 10): the realm's authority and boundary, the engine
/// executable a `{brokkr}` expands to, the composition, what the sites
/// opened, the one canonical site family, the seats, and the agent hands
/// the wrapped verify seat resolved. Each phase after the seat loop is one
/// of its methods.
pub(super) struct Seated<'s, 'a> {
    pub(super) authority: &'s Authority,
    pub(super) boundary: Boundary,
    pub(super) engine: &'s Executable,
    pub(super) resolved: &'s compose::Resolved,
    pub(super) agents: Opened<'a>,
    pub(super) sites: BTreeMap<String, SiteFacts>,
    pub(super) seats: BTreeMap<String, Seat>,
    pub(super) verify_agent_hands: Option<HandsSpec>,
}

impl Seated<'_, '_> {
    /// What the seats opened, settled; their model pins; the table's seat
    /// and ruling lints; the capability pass; and the adapters' own load.
    pub(super) fn settle(
        &mut self,
        machine: &Machine,
        adapters_root: &Path,
    ) -> Result<(), CompileError> {
        self.agents.settle(&self.resolved.seats)?;
        pin_drivers(&self.seats, &self.agents, &mut self.sites)?;

        phases::seated_and_ruled(machine, &self.seats)?;

        record_seat_capabilities(
            self.authority,
            &self.agents,
            (self.boundary, self.engine),
            self.resolved,
            &mut self.sites,
        )?;
        // Decision 0069: a PRESENT adapters root that does not load is
        // refused with the loader's own words, so a malformed discovery
        // notice never passes itself off as an adapter that declares none.
        // Judged after the capability pass, which already refuses a seat
        // the failed load leaves with no valid denial, naming that same
        // cause (decision 0066 ruling 1). An absent root reads as no notice.
        if let Some(problem) = self.agents.unloadable().filter(|_| adapters_root.exists()) {
            return Err(CompileError::Invalid(problem.to_string()));
        }
        Ok(())
    }

    /// The authoring census, the dialect wrapper where the realm's dialect
    /// wraps a parsed verify seat, and the final aliasing walk.
    pub(super) fn wrap(&mut self, wrapped_verify: Option<&Command>) -> Result<(), CompileError> {
        // The authoring census (design D10 F2): every structural owner is
        // registered, and a raw collision refused, BEFORE the wrapper
        // changes an address or any destination fact is written. The
        // within-body pass keeps its narrower diagnostic first, and the
        // census then catches what wrapping would disguise or create.
        refuse_aliasing_sites(&self.seats)?;
        let census = owner_index(&self.seats)?;

        let verify = phase_name::VERIFY;
        if let Some(command) = wrapped_verify.filter(|_| self.seats.contains_key(verify)) {
            self.wrap_verify(command, census)?;
        }

        refuse_global_aliasing(&self.seats)?;
        Ok(())
    }

    /// The wrapped verify seat becomes a sequence of its prior body, at
    /// `checks`, and the dialect's validator, every address it introduces
    /// claimed against the authoring census first.
    fn wrap_verify(
        &mut self,
        command: &Command,
        census: BTreeMap<String, SiteKey>,
    ) -> Result<(), CompileError> {
        let verify = phase_name::VERIFY;
        let (prior_body, moved) = wrapped_owners(&self.seats)?;
        claim_wrapped_addresses(census, &moved)?;
        let dialect_site = "verify:dialect-verify";
        self.seat_validator(dialect_site)?;
        relocate_verify_facts(&mut self.sites, &moved);
        let prior = SequenceStep {
            name: "checks".into(),
            class: SeatClass::of(self.seats[verify].has_gate),
            results: self.seats[verify].results.clone(),
            body: prior_body,
        };
        self.seats.get_mut(verify).expect("parsed above").body = SeatBody::Sequence {
            steps: vec![
                prior,
                SequenceStep {
                    name: "dialect-verify".into(),
                    class: SeatClass::Gate,
                    results: dialect_results(verify)
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    body: StepBody::Dialect {
                        execution: DialectExecution {
                            argv: command.argv.clone(),
                            state: command.state.clone(),
                        },
                    },
                },
            ],
        };
        Ok(())
    }

    /// The injected validator's site facts: its model policy, hands,
    /// capability outcome and judged tools.
    fn seat_validator(&mut self, dialect_site: &str) -> Result<(), CompileError> {
        let verify = phase_name::VERIFY;
        let (boundary, resolved) = (self.boundary, self.resolved);
        let synthetic = dialect_gate_site(dialect_site, boundary)?;
        let verify_raw = &resolved.seats[verify];
        let secrets = parse_secrets(verify, verify_raw)?;
        let law = SiteLaw {
            boundary,
            dir: &resolved.roots[resolved.seat_origin[verify]],
            agent_hands: self.verify_agent_hands.as_ref(),
        };
        enforce_model_policy(
            dialect_site,
            &synthetic,
            &[],
            &secrets,
            &mut self.agents,
            law,
            &mut self.sites,
        )?;
        record_hands(dialect_site, &synthetic, None, &secrets, &mut self.sites)?;
        // The one site the engine generated was written by no author
        // and asks for nothing, and it still gets an explicit outcome
        // through the same walk every authored site takes: "no
        // capability" is a recorded fact, never a missing one (design
        // D5). ONLY this site is given one here — an authored site the
        // walk above missed keeps none, and a site with no outcome is
        // refused at dispatch rather than launched on its defaults.
        // And it is refused like one: an `exec` harness declared with
        // a native power it cannot switch off does not seat the
        // validator either (ruling 4).
        {
            // A dialect phase opened the adapters above, and the
            // validator's gate law read them.
            let at = (boundary, self.engine);
            let walk = CapabilityWalk::of(self.authority, &self.agents, at, resolved);
            record_capabilities(&walk, law.dir, dialect_site, &synthetic, &mut self.sites)?;
        }
        // The generated validator declares no `tools`, and like every
        // other visited executable it records that as a CHECKED
        // unspecified value rather than an unvisited one (design
        // D5.2; review return F3).
        let unspecified = crate::agents::LocalTools::unspecified();
        record_judged_tools(
            (self.engine, law.dir),
            dialect_site,
            unspecified,
            None,
            None,
            &mut self.sites,
        );
        Ok(())
    }
}

/// The wrapped seat's exact authoring owners — its own single label or its
/// own panel members, never a prefix sweep that would also drag an
/// unrelated literal phase onto a wrapper coordinate — each beside its
/// destination, and its body as the sequence's prior step.
fn wrapped_owners(seats: &BTreeMap<String, Seat>) -> Result<(StepBody, Vec<Moved>), CompileError> {
    let verify = phase_name::VERIFY;
    let body = &seats
        .get(verify)
        .expect("the wrapper is applied only to a parsed verify seat")
        .body;
    let executable = match body.selected(None) {
        Some((executable, _)) if !matches!(executable, ExecutableBody::Sequence { .. }) => {
            executable
        }
        _ => {
            return Err(CompileError::Invalid(
                "dialect verify currently requires a single or panel verify seat".into(),
            ))
        }
    };
    let moved: Vec<(String, String, SiteKey)> =
        crate::engine::resume::owner_sites(executable, verify, None)
            .into_iter()
            .map(|owner| {
                let destination = match owner.tag() {
                    None => "verify:checks".to_string(),
                    Some(tag) => format!("verify:checks:{tag}"),
                };
                (owner.label(), destination, owner)
            })
            .collect();
    let prior_body = match body {
        SeatBody::Invocation(invocation) => StepBody::Invocation(invocation.to_step()),
        _ => {
            return Err(CompileError::Invalid(
                "dialect verify currently requires a single or panel verify seat".into(),
            ))
        }
    };
    Ok((prior_body, moved))
}

/// Every address the wrapper introduces, claimed against the census.
fn claim_wrapped_addresses(
    census: BTreeMap<String, SiteKey>,
    moved: &[Moved],
) -> Result<(), CompileError> {
    let verify = phase_name::VERIFY;
    // The validator's address is claimed against the WHOLE census,
    // before any source is drained (operator ruling of 2026-09-30,
    // unit 26c): its facts are written before the wrapped body's
    // move, so a member named for it would take them along.
    claim_address(
        &census,
        verify,
        "verify:dialect-verify",
        "the injected dialect validator",
    )?;
    // Drain every source reservation before claiming any
    // destination: a member `x` beside `checks:x` has the second
    // member's source as its destination, which stays legal.
    let mut remaining = census;
    for (source, _, _) in moved {
        remaining.remove(source);
    }
    for (_, destination, owner) in moved {
        claim_address(&remaining, verify, destination, &owner.describe())?;
    }
    Ok(())
}

/// The manifest's capability record: the realm's authority over every ask
/// a site or the library consulted, and each site's outcome.
pub(super) fn capability_record(
    authority: &Authority,
    agents: &Opened<'_>,
    sites: &BTreeMap<String, SiteFacts>,
) -> Value {
    let capability_sites = pins::capability_sites(sites);
    let consulted: Vec<String> = sites
        .values()
        .filter_map(|facts| facts.capabilities.as_ref())
        .flat_map(|site| site.asks.asks.keys().chain(&site.asks.subtracted).cloned())
        .chain(agents.library_asks())
        .collect();
    let mut capability_record = authority.manifest(&consulted);
    capability_record["sites"] = Value::Object(capability_sites);
    capability_record
}

/// Each select seat's body, as the manifest records it.
pub(super) fn select_records(seats: &BTreeMap<String, Seat>) -> Map<String, Value> {
    seats
        .iter()
        .filter_map(|(site, seat)| match &seat.body {
            SeatBody::Select { .. } => Some((site.clone(), body_manifest(&seat.body))),
            _ => None,
        })
        .collect()
}

/// The manifest's and the bundle's projections of the site family.
pub(super) struct Projections {
    pub(super) hands: BTreeMap<String, HandsSpec>,
    pub(super) inline_resume: BTreeMap<String, Value>,
    pub(super) records: Map<String, Value>,
    pub(super) drivers: Map<String, Value>,
}

/// Decision 0035 addendum 2026-09-11: the adapter digests whose effortless
/// listings exempted inline seats ride beside decision 0021's — un-listing
/// a route moves the digest of every bundle it excused, and a bundle no
/// listing excused keeps its shape. The manifest fields are serialization
/// projections of the one canonical family table (design D10 F1), read in
/// one place and never separately mutated.
pub(super) fn projections(sites: &BTreeMap<String, SiteFacts>) -> Projections {
    let hands: BTreeMap<String, HandsSpec> = sites
        .iter()
        .filter_map(|(label, facts)| facts.hands_spec().map(|spec| (label.clone(), spec.clone())))
        .collect();
    let inline_resume: BTreeMap<String, Value> = sites
        .iter()
        .filter_map(|(label, facts)| {
            facts
                .inline_resume
                .clone()
                .map(|value| (label.clone(), value))
        })
        .collect();
    let records: Map<String, Value> = sites
        .iter()
        .filter_map(|(label, facts)| facts.record.clone().map(|record| (label.clone(), record)))
        .collect();
    let mut drivers: Map<String, Value> = Map::new();
    fold_driver_facts(&mut drivers, sites);
    Projections {
        hands,
        inline_resume,
        records,
        drivers,
    }
}

/// Design D7 (rebuild unit 16-fix-b, F3): every layer's identity is sealed
/// from the buffers it was composed from and the charters its seats were
/// bound to, ancestors first, so no consumed file is read again by a walk.
/// The leaf's walk takes its own the same way, and each input must still
/// stand as it was read or nothing seals. Rebuild unit 18-fix-b (council
/// F2): who each charter's owner is was taken by the read that bound the
/// charter, and the seal's check compares it; no later walk records
/// another. Its return (council F2): a library charter, which no layer's
/// walk pins, is checked by the read that bound it, owner included, before
/// any identity is sealed.
pub(super) fn seal_layers(
    reads: &[LibraryRead],
    resolved: &mut compose::Resolved,
    charters: Charters,
) -> Result<(), CompileError> {
    for read in reads {
        read.check()?;
    }
    resolved.seal(charters.into_inner())?;
    Ok(())
}

/// Second council H6: every charter this compile bound, kept where a
/// projection of the site facts cannot lose it, with each binding a site
/// selected for its path (rebuild unit 17).
pub(super) fn charter_pins(sites: &BTreeMap<String, SiteFacts>) -> CharterPins {
    let mut charters = CharterPins::new();
    for charter in sites.values().filter_map(|site| site.charter.clone()) {
        charters
            .entry(charter.path.clone())
            .or_default()
            .insert(charter);
    }
    charters
}
