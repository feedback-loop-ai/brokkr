//! The engine's MCP set a qoder launch is served (#1178), read from the
//! sealed inputs as dsh's hands are ([`super::dsh_hands`]) and handed to
//! Qoder's arm of the set's builder ([`crate::native_controls::mcp::qoder`]).
//! A child module because its parent stands at the production file ceiling.

use std::path::Path;

use serde_json::Value;

use super::{
    engine_executable, hands_transport, sealed_reach, sealed_servers, Edge, McpRefusal, StrictCause,
};
use crate::native_controls::mcp::qoder::{served, Served};
use crate::native_controls::SealedServers;

/// What the engine's set gives a qoder launch in `workdir`: the empty set
/// where `input` carries no sealed inputs, a driver run by hand, or they
/// seal no boxed hands, and the box's hands where they do. Inputs that
/// cannot be read refuse, and so does a brokered set, whose grant Qoder
/// has not been measured with, and hands no document value binds exactly.
pub(in crate::adapters) fn qoder_set(input: &Value, workdir: &str) -> Result<Served, McpRefusal> {
    qoder_set_at(input, workdir, &engine_executable())
}

/// [`qoder_set`] for the engine's executable at `brokkr`, so a path no
/// document value represents is a plain test.
pub(super) fn qoder_set_at(
    input: &Value,
    workdir: &str,
    brokkr: &Path,
) -> Result<Served, McpRefusal> {
    let edge = Edge::new(input);
    let reach = sealed_reach(input);
    let hands = match edge.sealed() {
        None => None,
        Some(sealed) => {
            let sealed = sealed.as_ref().map_err(|_| McpRefusal::Unreadable)?;
            if !sealed.isolation.servers.brokers().is_empty() {
                return Err(McpRefusal::Strict(StrictCause::Unmeasured {
                    provider: "qoder".into(),
                    shape: "brokers".into(),
                }));
            }
            let boxed = sealed_servers(sealed) == SealedServers::Hands;
            hands_transport(sealed, (brokkr, workdir), input, &reach).filter(|_| boxed)
        }
    };
    served(hands).ok_or(McpRefusal::NotSealed { provider: "qoder" })
}
