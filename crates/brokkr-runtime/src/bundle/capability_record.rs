//! The arms of [`record_capabilities`](super::record_capabilities) (#288,
//! slice I): decision 0065's capability record of one composed seat, by the
//! kind of site it is — an agent-backed site, a single inline site, a
//! dialect step — and the sites a panel, sequence or select nests.
//! `record_capabilities` reads as their sequence and keeps the walk.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{
    admit_inline_launch, admit_native_sandbox, command_parts, compose, dispatch_driver,
    expand_command, mcp, pins, site_asks, site_facts, step_label, CapabilityAdapters, CompileError,
    McpIntent, Opened, SeatClass, SiteFacts,
};
use crate::agents::Library;
use crate::argv::Executable;
use brokkr_core::realms::Boundary;
use brokkr_protocol::native_controls::{Origin, Segment};

/// What one capability walk reads at every site it visits, in one
/// parameter object (decision 0071 ruling 10): the authority it resolves
/// against, the agent library and the adapters the seats opened, the
/// realm's boundary, the engine executable a `{brokkr}` expands to, and the
/// composition's layer roots with the layer that wrote each select case.
pub(super) struct CapabilityWalk<'a> {
    pub(super) authority: &'a crate::capabilities::Authority,
    pub(super) library: Option<&'a Library>,
    pub(super) adapters: CapabilityAdapters<'a>,
    pub(super) boundary: Boundary,
    pub(super) engine: &'a Executable,
    pub(super) roots: &'a [PathBuf],
    pub(super) case_origin: &'a BTreeMap<String, usize>,
}

impl<'a> CapabilityWalk<'a> {
    /// The walk over `resolved`'s seats, through what `agents` opened.
    pub(super) fn of(
        authority: &'a crate::capabilities::Authority,
        agents: &'a Opened<'_>,
        (boundary, engine): (Boundary, &'a Executable),
        resolved: &'a compose::Resolved,
    ) -> Self {
        CapabilityWalk {
            authority,
            library: agents.library(),
            adapters: agents.capability_adapters(),
            boundary,
            engine,
            roots: &resolved.roots,
            case_origin: &resolved.case_origin,
        }
    }
}

/// One site as its capability arm reads it: the authority it resolves
/// against, the adapters, the site's label and its declaration.
pub(super) struct CapabilitySite<'a> {
    pub(super) authority: &'a crate::capabilities::Authority,
    pub(super) adapters: CapabilityAdapters<'a>,
    pub(super) what: &'a str,
    pub(super) raw: &'a Value,
}

/// An agent-backed site: its asks are its agent's minus what the site
/// subtracts, resolved against every candidate of its chain, and a wanted
/// capability a candidate lost is a notice in the site's agent record.
pub(super) fn record_agent_capabilities(
    at: &CapabilitySite<'_>,
    library: Option<&Library>,
    boundary: Boundary,
    name: &str,
    sites: &mut BTreeMap<String, SiteFacts>,
) -> Result<(), CompileError> {
    let CapabilitySite {
        authority,
        adapters,
        what,
        raw,
    } = *at;
    let agent = library
        .and_then(|library| library.agent(name))
        .expect("the seat loop resolved this agent reference");
    let asks = site_asks(what, raw, Some((name, &agent.capabilities)))?;
    let chain = site_facts(sites, what).chain.clone();
    // Under the harness boundary a hands site's candidates are composed
    // with the `hands.harness.*` fragment of the seat's class.
    let class = match (boundary, &agent.hands) {
        (Boundary::Harness, Some(_)) => Some(asks.class),
        _ => None,
    };
    let site = mcp::candidate_capabilities(authority, adapters, asks, &chain, class)?;
    let facts = site_facts(sites, what);
    // The EFFECTIVE class, an inherited office class included, as the
    // local admission recorded it (design D5.6).
    if let Some(requested) = facts.local.as_ref().and_then(|local| local.sandbox) {
        admit_native_sandbox(what, requested, &site)?;
    }
    let notices = facts
        .record
        .as_mut()
        .and_then(|record| record.get_mut("notices"))
        .and_then(Value::as_array_mut)
        .expect("an agent-backed site carries its resolution record");
    for (candidate, outcome) in chain.iter().zip(&site.outcomes) {
        for (capability, message) in &outcome.notices {
            notices.push(
                crate::agents::Notice {
                    agent: candidate.agent.clone(),
                    provider: candidate.provider.clone(),
                    model: candidate.model.clone(),
                    capability: "capability".to_string(),
                    item: capability.clone(),
                    message: message.clone(),
                }
                .value(),
            );
        }
    }
    facts.seal(site, pins::agents(adapters, &chain));
    Ok(())
}

/// A single inline site, by the same two keys `has_single` reads: its map
/// is its own office's asks, served as its driver serves it. `engine` and
/// `dir` are what its command was expanded against.
pub(super) fn record_inline_capabilities(
    at: &CapabilitySite<'_>,
    (engine, dir): (&Executable, &Path),
    sites: &mut BTreeMap<String, SiteFacts>,
) -> Result<(), CompileError> {
    let CapabilitySite {
        authority,
        adapters,
        what,
        raw,
    } = *at;
    let asks = site_asks(what, raw, None)?;
    let parts = command_parts(raw);
    let driver = dispatch_driver(&parts);
    let facts = site_facts(sites, what);
    // Operator ruling (B) of 2026-09-27: an inline site with hands is
    // served like an agent, so its dialect carries the `hands.workspace`
    // fragment its driver's adapter declares, as `agents::compose`
    // carries an agent's. Its hands are boxed: the hands law refuses an
    // inline model harness's hands unboxed (decision 0046 ruling 4).
    let declared = adapters
        .adapters
        .zip(driver.as_deref())
        .and_then(|(adapters, driver)| adapters.adapter(driver))
        .and_then(|adapter| adapter.hands.as_ref())
        .filter(|_| facts.hands_spec().is_some());
    if let (Some(dialect), Some(fragment)) = (facts.inline_dialect.as_mut(), declared) {
        dialect.hands = fragment.clone();
        // Rebuild unit 14a4a: the engine emits that fragment, expanded
        // as an agent's `hands` segment is, and the plan types it.
        facts.inline_hands = Some(Segment::new(
            Origin::Hands,
            &expand_command(engine, dir, fragment),
        ));
    }
    // U1f: the MCP set this serving intends, never read from its bytes.
    facts.inline_mcp = Some(McpIntent::inline(driver.as_deref(), facts.hands_spec()));
    let serving = mcp::InlineServing::of(facts, driver.as_deref(), &parts);
    let site = mcp::inline_capabilities(authority, adapters, asks, &serving)?;
    // Rebuild unit 5d-fix-b: an inline class the engine lowered is
    // judged with its whole launch, the resolved native plan included.
    admit_inline_launch(what, &parts, facts, &site)?;
    facts.seal(site, pins::inline(raw, adapters));
    Ok(())
}

/// A dialect step runs the realm dialect's own validator through the
/// exec driver: no author wrote its command and it asks for nothing,
/// and it still gets an explicit outcome — "no capability" is a
/// recorded fact, never a missing one (design D5). Only where the
/// dialect supplied a validator: an unsupported `check` compiles to no
/// site at all, and none is invented for it here.
pub(super) fn record_dialect_capabilities(
    at: &CapabilitySite<'_>,
    sites: &mut BTreeMap<String, SiteFacts>,
) -> Result<(), CompileError> {
    let CapabilitySite {
        authority,
        adapters,
        what,
        ..
    } = *at;
    if sites.contains_key(what) {
        // The validator judges: a gate, as its compiled step's `class`.
        let asks = crate::capabilities::SiteAsks::at(SeatClass::Gate, what, None, None)
            .map_err(CompileError::Invalid)?;
        let facts = site_facts(sites, what);
        facts.inline_mcp = Some(McpIntent::inline(Some("exec"), facts.hands_spec()));
        let serving = mcp::InlineServing {
            driver: Some("exec"),
            argv: &[],
            local: &[],
            hands: &[],
            intent: facts.inline_mcp,
        };
        let site = mcp::inline_capabilities(authority, adapters, asks, &serving)?;
        facts.seal(site, pins::exec());
    }
    Ok(())
}

/// The sites a panel, sequence or select nests, each with its label and
/// the directory it is walked in: a panel member and a sequence step in
/// the seat's own, a select case in the layer that wrote it, from `roots`
/// by `case_origin`.
pub(super) fn nested_sites<'v>(
    what: &str,
    raw: &'v Value,
    (dir, roots, case_origin): (&'v Path, &'v [PathBuf], &BTreeMap<String, usize>),
) -> Vec<(String, &'v Value, &'v Path)> {
    let mut nested: Vec<(String, &Value, &Path)> = Vec::new();
    if let Some(panel) = raw.get("panel").and_then(Value::as_object) {
        nested.extend(
            panel
                .iter()
                .map(|(member, raw)| (format!("{what}:{member}"), raw, dir)),
        );
    }
    if let Some(sequence) = raw.get("sequence").and_then(Value::as_array) {
        for (index, step) in sequence.iter().enumerate() {
            nested.push((format!("{what}:{}", step_label(index, step)), step, dir));
        }
    }
    if let Some(select) = raw.get("select").and_then(Value::as_object) {
        // Each case in the layer that wrote it (second review return C1);
        // the default stays with the seat's owner.
        nested.extend(
            select
                .get("cases")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .map(|(case, raw)| {
                    let label = format!("{what}:{case}");
                    let owner = case_origin
                        .get(&label)
                        .map_or(dir, |&index| roots[index].as_path());
                    (label, raw, owner)
                }),
        );
        if let Some(body) = select.get("default") {
            nested.push((format!("{what}:default"), body, dir));
        }
    }
    nested
}
