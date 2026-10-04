//! A message, as Claude Code's stream and its transcripts both carry it,
//! and the usage it reports (#484): its text, the model it names, its
//! counts, and the tools the provider ran for it, by count or by its
//! stop. Read for `claude` and `claude_log`, as `claude` reads a key:
//! consumed, or inert only when no value of it can contradict a fact the
//! verdict rests on.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::claude::texts;
use super::forms::Origin;
use super::read::{Counted, Empty, Given, Null, Said};
use super::Fault;

/// A message, as claude's stream and its transcripts carry it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Message {
    /// The message the usage counts.
    #[serde(default)]
    id: Given<String>,
    content: Vec<Block>,
    #[serde(default)]
    usage: Given<Usage>,
    #[serde(default)]
    model: Given<String>,
    /// Inert: a tag, of one value.
    #[serde(default, rename = "type")]
    _kind: Given<MessageTag>,
    /// Inert: a tag, of one value.
    #[serde(default, rename = "role")]
    _role: Given<Role>,
    /// Why the message ended, `null` while it streams: a tool use is a
    /// tool run.
    stop_reason: Option<Ending>,
    /// Inert: the sequence that ended it, which runs nothing.
    #[serde(rename = "stop_sequence")]
    _stop_sequence: Option<String>,
    #[serde(rename = "stop_details")]
    _stop_details: Null,
    #[serde(rename = "container")]
    _container: Null,
    #[serde(rename = "diagnostics")]
    _diagnostics: Null,
    #[serde(rename = "context_management")]
    _context_management: Null,
    #[serde(default, rename = "input_transformations")]
    _input_transformations: Empty,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum MessageTag {
    Message,
}

/// Why a message or a turn ended.
#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Ending {
    EndTurn,
    StopSequence,
    ToolUse,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Role {
    Assistant,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Block {
    /// Read whole, as the reply or a refusal.
    Text { text: String },
}

/// Every key may be absent, so the struct defaults each one; those the
/// recordings show `null` are `Option`s.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Usage {
    input_tokens: Given<u64>,
    cache_creation_input_tokens: Given<u64>,
    cache_read_input_tokens: Given<u64>,
    output_tokens: Given<u64>,
    /// Inert: `cache_creation_input_tokens` split by lifetime.
    #[serde(rename = "cache_creation")]
    _cache_creation: Given<CacheCreation>,
    /// The searches and fetches the provider ran: a tool run.
    server_tool_use: Given<ServerToolUse>,
    /// Inert: the token counts restated per request, none of them a use.
    #[serde(rename = "iterations")]
    _iterations: Option<Vec<Iteration>>,
    /// Inert: part of `output_tokens`.
    #[serde(rename = "output_tokens_details")]
    _output_tokens_details: Option<OutputDetails>,
    /// Inert: a billing tag.
    #[serde(rename = "service_tier")]
    _service_tier: Option<String>,
    /// Inert: a billing tag.
    #[serde(rename = "inference_geo")]
    _inference_geo: Option<String>,
    /// Inert: a billing tag.
    #[serde(rename = "speed")]
    _speed: Option<String>,
    #[serde(rename = "fallback_credit")]
    _fallback_credit: Null,
}

/// Inert, every key: `cache_creation_input_tokens` by lifetime.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CacheCreation {
    #[serde(rename = "ephemeral_1h_input_tokens")]
    _hour: u64,
    #[serde(rename = "ephemeral_5m_input_tokens")]
    _minutes: u64,
}

/// How many searches and fetches the provider ran for the turn.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ServerToolUse {
    web_search_requests: u64,
    web_fetch_requests: u64,
}

/// Inert, every key: the usage restated for one request.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Iteration {
    #[serde(rename = "input_tokens")]
    _input: u64,
    #[serde(rename = "output_tokens")]
    _output: u64,
    #[serde(rename = "cache_read_input_tokens")]
    _cache_read: u64,
    #[serde(rename = "cache_creation_input_tokens")]
    _cache_creation: u64,
    #[serde(rename = "cache_creation")]
    _by_lifetime: CacheCreation,
    #[serde(rename = "type")]
    _kind: MessageTag,
}

/// Inert, every key: part of `output_tokens`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputDetails {
    #[serde(rename = "thinking_tokens")]
    _thinking: u64,
}

/// One model's share of the turn, in a result and in a transcript's cost
/// row: its searches, and, inert, the counts and cost `usage` and
/// `total_cost_usd` restate, and the model's limits, which the result
/// alone gives. None of them is a use.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ModelUsage {
    web_search_requests: u64,
    #[serde(rename = "inputTokens")]
    _input: u64,
    #[serde(rename = "outputTokens")]
    _output: u64,
    #[serde(rename = "cacheReadInputTokens")]
    _cache_read: u64,
    #[serde(rename = "cacheCreationInputTokens")]
    _cache_creation: u64,
    #[serde(rename = "costUSD")]
    _cost: f64,
    #[serde(rename = "thinkingTokens")]
    _thinking: u64,
    #[serde(default, rename = "contextWindow")]
    _context_window: Given<u64>,
    #[serde(default, rename = "maxOutputTokens")]
    _max_output: Given<u64>,
    #[serde(default, rename = "canonicalModel")]
    _canonical_model: Given<String>,
    #[serde(default, rename = "provider")]
    _provider: Given<String>,
    #[serde(default, rename = "costBasis")]
    _cost_basis: Given<String>,
}

/// The searches each model of `usage`, a `modelUsage`, ran, as tool
/// runs.
pub(super) fn searched(usage: &BTreeMap<String, ModelUsage>) -> Vec<Said> {
    let ran = usage.values().filter(|model| model.web_search_requests > 0);
    ran.map(|_| Said::Ran {
        tool: Some(SEARCH),
        at: "/modelUsage/*/webSearchRequests",
    })
    .collect()
}

/// Claude Code's names for the tools its provider runs.
const SEARCH: &str = "WebSearch";
const FETCH: &str = "WebFetch";

/// Where an event carries usage, the provider's counts of searches and
/// fetches within it, and why it stopped.
pub(super) struct UsageAt {
    usage: &'static str,
    searches: &'static str,
    fetches: &'static str,
    pub(super) stop: &'static str,
}

const IN_MESSAGE: UsageAt = UsageAt {
    usage: "/message/usage",
    searches: "/message/usage/server_tool_use/web_search_requests",
    fetches: "/message/usage/server_tool_use/web_fetch_requests",
    stop: "/message/stop_reason",
};

pub(super) const IN_RESULT: UsageAt = UsageAt {
    usage: "/usage",
    searches: "/usage/server_tool_use/web_search_requests",
    fetches: "/usage/server_tool_use/web_fetch_requests",
    stop: "/stop_reason",
};

/// Usage, counted, and each search and fetch it counts as a tool run.
pub(super) fn usage_said(at: &UsageAt, message: Option<String>, usage: Usage) -> Vec<Said> {
    let counted = Counted::of(
        at.usage,
        message,
        [
            ("input_tokens", usage.input_tokens.0),
            (
                "cache_creation_input_tokens",
                usage.cache_creation_input_tokens.0,
            ),
            ("cache_read_input_tokens", usage.cache_read_input_tokens.0),
            ("output_tokens", usage.output_tokens.0),
        ],
    );
    let used = usage.server_tool_use.0.map(|used| {
        [
            (used.web_search_requests, SEARCH, at.searches),
            (used.web_fetch_requests, FETCH, at.fetches),
        ]
    });
    let ran = used.into_iter().flatten().filter(|(count, ..)| *count > 0);
    let ran = ran.map(|(_, tool, at)| Said::Ran {
        tool: Some(tool),
        at,
    });
    std::iter::once(counted).chain(ran).collect()
}

/// A stop for a tool use, as a tool run at `at`.
pub(super) fn stopped(stop: Option<&Ending>, at: &'static str) -> Option<Said> {
    (stop == Some(&Ending::ToolUse)).then_some(Said::Ran { tool: None, at })
}

/// What a message says: its text read whole, unless `failed` says the
/// event already states its failure, which no text can undo; the model
/// it names; its usage; and the tools it ran.
pub(super) fn message_said(message: Message, failed: bool) -> Result<Vec<Said>, Fault> {
    let mut said = Vec::new();
    for Block::Text { text } in message.content.iter().filter(|_| !failed) {
        said.extend(texts(text, Origin::Answer)?);
    }
    said.extend(message.model.0.map(Said::Model));
    if let Some(usage) = message.usage.0 {
        said.extend(usage_said(&IN_MESSAGE, message.id.0, usage));
    }
    said.extend(stopped(message.stop_reason.as_ref(), IN_MESSAGE.stop));
    Ok(said)
}
