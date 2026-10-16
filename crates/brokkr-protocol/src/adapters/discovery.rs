//! The capability servers a seat is told it is given (decision 0065 slice
//! two, U7d; SD3, CC2). The engine's private `capability_servers` carrier,
//! which it writes from the serving candidate's own selected set and clears
//! on every selection (U7c), is read here once into a closed type and
//! rendered as fixed guidance inside the existing capability contract.
//! Only selected identifiers reach it: each server, its capability and
//! exact tools, and the discovery tool the serving adapter's measured
//! `hands.notice` declares (decision 0069). No plan, mount, store or ledger
//! locator is read, and no provider name decides what it says.

use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::Value;

use crate::native_controls::SealedBroker;

/// The driver-input key the engine writes the carrier under.
const CARRIER: &str = "capability_servers";

/// The carrier as the engine writes it: every server it names to the seat,
/// and the discovery tool, `null` where the serving adapter declares none.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selected {
    servers: Vec<Server>,
    // Present, as `null` or a name: never read as absent.
    #[serde(deserialize_with = "Option::deserialize")]
    discovery_tool: Option<String>,
}

/// One brokered holding as the seat is told of it.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Server {
    server: String,
    capability: String,
    tools: Vec<String>,
}

impl Selected {
    /// The carrier `input` holds, `None` where it holds none or one that is
    /// not exactly the engine's: no server, a server not named
    /// `cap-<capability>` for one bare capability or named twice, a holding
    /// with no tool or a tool that is not one identifier, or a discovery
    /// tool outside decision 0069's grammar. Like the hands notice's, an
    /// absent or malformed carrier supplies nothing.
    fn read(input: &Value) -> Option<Selected> {
        let selected = Selected::deserialize(input.get(CARRIER)?).ok()?;
        let mut named = BTreeSet::new();
        let servers = selected.servers.iter().all(|server| {
            SealedBroker::server(&server.capability).as_ref() == Some(&server.server)
                && named.insert(&server.server)
                && !server.tools.is_empty()
                && server.tools.iter().all(|tool| identifier(tool))
        });
        let discovery = (selected.discovery_tool.as_deref())
            .is_none_or(|tool| super::tool_identifier(tool).is_ok());
        (!selected.servers.is_empty() && servers && discovery).then_some(selected)
    }

    /// The fixed guidance: each capability by the server that serves it
    /// and the exact names its tools are called by, then, where the serving
    /// adapter declares one, the tool that loads a deferred one.
    fn guidance(&self) -> String {
        let mut text = String::new();
        for Server {
            server,
            capability,
            tools,
        } in &self.servers
        {
            let called: Vec<String> = (tools.iter())
                .map(|tool| format!("`{}`", called(server, tool)))
                .collect();
            text.push_str(&format!(
                "\nYou reach `{capability}` through its server `{server}`, whose tools you call as \
                 {}.",
                called.join(", ")
            ));
        }
        if let Some(discovery) = &self.discovery_tool {
            text.push_str(&format!(
                "\nIf one of these tools is not listed, use `{discovery}` to load it before \
                 calling it."
            ));
        }
        text
    }
}

/// The name a harness calls `server`'s `tool` by, `mcp__<server>__<tool>`.
/// Its other home is `capability_calls.rs`'s `MCP_PREFIX` and
/// `MCP_SEPARATOR`, which read the same name back from a call; the two are
/// held together by `discovery_tests`'s
/// `the_guidance_names_a_tool_as_its_observed_call_shows_it`.
fn called(server: &str, tool: &str) -> String {
    format!("mcp__{server}__{tool}")
}

/// Whether `tool` renders as one identifier inside the guidance's code
/// span: ASCII graphic characters other than the backtick, so no tool can
/// close its span, break its line or carry prose of its own.
fn identifier(tool: &str) -> bool {
    !tool.is_empty()
        && tool
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b'`')
}

/// The existing capability contract
/// ([`crate::native_controls::capabilities_paragraph`]) with the selected
/// servers' guidance inside it: only where that contract is rendered and
/// the input carries the engine's own carrier, so prose elsewhere in the
/// input can neither add the guidance nor suppress it.
pub(super) fn capabilities_contract(input: &Value) -> String {
    let mut contract = crate::native_controls::capabilities_paragraph(input);
    if let Some(selected) = Selected::read(input).filter(|_| !contract.is_empty()) {
        contract.push_str(&selected.guidance());
    }
    contract
}
