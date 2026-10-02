## Purpose

Fulfil 0065 ruling 8: "A tool call through a granted capability is checkpointed
with its capability, dialect and tool names", including native grants first.

## ADDED Requirements

### Requirement: CC1 attribution comes from the selected sealed holding

The engine SHALL attribute every observed native capability call using that
attempt's selected site/candidate holdings and adapter tool mapping.
It SHALL stamp capability and dialect, retaining the concrete tool identity,
effect/attempt ownership and existing site/instance/boundary stamps.
Native telemetry SHALL be normalized at the harness edge into typed tool
identity before any display clamp; a generic item type such as mcp_tool_call
SHALL not be treated as a concrete tool name. Native call_id SHALL use the
attempt plus the measured harness call identifier, with at most one observed
checkpoint per call even when the harness emits both start and completion.
A tool-use ordinal is permitted only for a measured format that emits exactly
one observation per call; absence of a reliable correlation refuses attribution
rather than conflating repeated calls. U4 records that format evidence and does
not depend on U1's strictness implementation.

Driver-supplied capability, dialect, retention digest and call ownership
SHALL never supply authority: the engine SHALL remove/reject such values and
derive its own. Non-capability local calls, including workspace hands, SHALL
keep ordinary checkpoints with attribution absent. No name substring,
model prose, tool result, another fallback's holding or current realm lookup
SHALL create attribution. An ambiguous reverse mapping SHALL refuse compile
with "tool '<tool>' maps to more than one held capability for provider
'<provider>'". An observed known capability call outside the holding SHALL
fail the attempt with "observed capability tool '<tool>' is not held by this
attempt"; it SHALL not be disguised as an ordinary local call.

#### Scenario: A native search call is attributed without a broker

- **WHEN** a Claude attempt holds web-search through claude-native-search and reports tool WebSearch
- **THEN** its checkpoint carries exactly capability web-search, dialect claude-native-search and tool WebSearch, with the selected attempt/site identities
- **AND** Codex's measured native web_search mapping gets its own dialect and tool identity under the same attribution contract

#### Scenario: A fallback cannot inherit primary attribution

- **WHEN** primary Codex is replaced by Claude at research, or a panel/sequence member has different holdings from its neighbors
- **THEN** every call reads only the selected candidate and actual executable site; no primary or sibling dialect is stamped
- **AND** cold replacements retain new attempt identity while eligible resumed work retains the serving site's currently pinned attribution

#### Scenario: Ordinary tools stay ordinary and forged stamps lose authority

- **WHEN** the driver reports workspace, a local shell tool, or a forged capability/dialect field beside a known tool
- **THEN** ordinary calls acquire no invented capability and forged fields do not survive engine stamping
- **AND** a known unheld native capability produces the exact attempt refusal rather than a falsely attributed checkpoint

#### Scenario: Ambiguous ownership refuses before a call

- **WHEN** two otherwise valid held native capabilities map the same provider tool to different capability/dialect pairs
- **THEN** compilation refuses with the exact ambiguous-tool cause under the site prefix
- **AND** two MCP capabilities exposing the same child tool name remain distinguishable by their distinct cap server identities

### Requirement: CC2 broker calls use the same checkpoint contract without double counting

Every syntactically valid broker tools/call request, including refused, failed and interrupted
calls, SHALL have a broker-generated monotonically increasing sequence within
its attempt/capability ledger. Its stable call_id SHALL be derived from
engine-owned attempt and broker identity plus that sequence, never from model
content or a reusable JSON-RPC id alone. A durable started record SHALL
precede forwarding an admitted call. Its terminal outcome SHALL distinguish
succeeded, failed, refused and interrupted. Native observation uses
call_state observed; it need not fabricate a response that telemetry did not
supply.

The engine SHALL fold broker records into seat-record v6 checkpoints, with
the same capability/dialect/tool fields as CC1, and SHALL be the only journal
writer. Harness MCP telemetry SHALL be normalized using U0 evidence for
diagnostics/discovery but SHALL not produce a second attributed broker call:
the bound ledger is authoritative for that call. Broker metadata SHALL carry
no request arguments, URL, command or response body into the journal.
Denied tool attempts SHALL be marked refused, not represented as granted
holdings or successful uses.

#### Scenario: Two telemetry events still describe one call

- **WHEN** a broker call emits a start and completion in its ledger and the harness also emits item.started and item.completed
- **THEN** the journal has one call_id with its started and terminal records, counted as one use, and no duplicate attributed harness call
- **AND** loss of harness completion cannot lose the durable broker terminal record

#### Scenario: An ungranted tool is visible as a refusal

- **WHEN** cap-library-docs refuses admin before forwarding
- **THEN** a terminal refused checkpoint names library-docs, its serving dialect and admin, with the safe refusal cause and no retained digest
- **AND** a reader cannot mistake it for evidence that admin was granted

#### Scenario: Missing measured fields do not become guessed names

- **WHEN** a supported harness's MCP event omits the server/tool identity U0 requires, or its shape changes
- **THEN** the parser records a typed telemetry limitation with cause "MCP telemetry does not identify its server and tool"
- **AND** it derives no holding from the string mcp_tool_call; the broker ledger still owns tool-call evidence
- **AND** native telemetry missing required attribution instead fails with "capability telemetry cannot be attributed" and does not claim complete recording

### Requirement: CC3 v6 adds attribution without rewriting historical records

Implementation SHALL publish seat-record.v6.schema.json beside v5 and update
the store's append/export/verify dispatch consistently. The contract delta
defines new field shapes and dependencies. Historical records SHALL retain
absence, never backfilled from today's grants or inferred from a tool name.
The engine-line boundary SHALL follow the store's existing additive version
convention, with exact old/new tests and an explicit boundary chosen at U4
against then-current main; this docs visit changes no package version.

#### Scenario: Old absence is honest

- **WHEN** inspect reads a valid v5 checkpoint with tool WebSearch and no capability
- **THEN** it shows attribution as unrecorded, not as web-search through the current realm's dialect
- **AND** old bytes, hash chains and frozen fixtures remain unchanged and valid

#### Scenario: New malformed attribution cannot pass the append fence

- **WHEN** a v6 record carries capability without dialect, digest without terminal retained response, a malformed digest or an invalid call state
- **THEN** the append fence rejects its typed contract violation and writes no event
- **AND** valid ordinary old-shaped records and complete new native/broker checkpoints pass

## Decisions

The selected Outcome is the authority, not NativeInventory::capability_of
alone: an inventory can name an unheld tool. Native U4 requires no broker
activation. Capability checkpoints report observed use; they do not claim
unreported native activity can be reconstructed from absent telemetry.

The broker ledger prevents dependence on each harness's response events.
Started/terminal checkpoints share call_id so crash evidence does not become
two uses. Native observed checkpoints deliberately claim no completion.
The engine's journal fence remains single-writer; a broker opening forge.db
is rejected as an alternative.

Malformed JSON-RPC or an invalid tool identifier is a protocol refusal before
it denotes a capability call; it carries MB3's safe protocol cause, never raw
invalid identifiers in checkpoint fields. This does not excuse dropping any
well-formed denied call or a granted call whose child fails.
