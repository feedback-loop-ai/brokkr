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
not depend on U1's strictness implementation. Resumed history SHALL be
distinguished from a new call by measured event semantics, never restamped
as current-attempt activity merely because its tool name matches.

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

Driver observation emission SHALL remain in its valid legacy shape until
the v6 append/export/verify consumer and engine attribution for single, panel,
sequence, fallback and resumed paths are installed. Private observations SHALL
be consumed and removed at the engine edge; only complete engine-owned SC4
groups or valid ordinary legacy records may reach the journal. Every
intermediate merge SHALL preserve native compile-to-journal operation; the
MCP compile fence supplies no protection for native calls.

#### Scenario: A preparation merge preserves legacy native checkpoints

- **WHEN** a native-only fixture compiles and runs through the production driver/process/engine/store path at any merge before new observation emission
- **THEN** its legacy checkpoint appends, exports and verifies with exact existing fields and no private observation or partial attribution group
- **AND** ordinary, inline, fallback, panel, sequence and eligible resumed/replacement paths retain valid records; no schema weakening or premature metadata emission is allowed

#### Scenario: Native emission starts only with complete consumers

- **WHEN** normalized native emission activates after the store fence and all engine attribution consumers are installed
- **THEN** real native compile-to-journal proofs cover ordinary, inline, fallback, panel, sequence and eligible resumed/replacement paths with exact selected capability/dialect/tool/call identity and observed state
- **AND** private observation fields and forged driver stamps never reach the store; historical replay yields no new call, repeated new calls remain distinct and start/completion count once
- **AND** bypassing engine consumption or emitting metadata on a preceding legacy-only path independently fails its append/export/verify proof; MCP remains fenced

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

Every valid bounded broker tools/call request accepted under CR3, including
locally refused, failed and interrupted calls, SHALL have a broker-generated
monotonically increasing sequence within its attempt/capability ledger.
Acceptance means durable Started after request validation and reservation of
ledger capacity, before local tool/budget refusal or external forwarding.
Unread over-capacity frames and invalid request identities are not accepted
calls; they SHALL NOT acquire invented checkpoints. Failure to persist Started
SHALL block forwarding and successful completion, preserving the verified
ledger prefix for CR4 recovery. Each accepted call's stable call_id SHALL be derived from
engine-owned attempt and broker identity plus that sequence, never from model
content or a reusable JSON-RPC id alone. A durable started record SHALL
precede forwarding an admitted call. Its terminal outcome SHALL distinguish
succeeded, failed, refused and interrupted. Native observation uses
call_state observed; it need not fabricate a response that telemetry did not
supply.

After the owned process tree settles, and before any terminal attempt result
is admitted, the engine SHALL fold exactly one settled seat-record v6
checkpoint per accepted broker call, with
the same capability/dialect/tool fields as CC1, and SHALL be the only journal
writer. Harness MCP telemetry SHALL be normalized using U0 evidence for
diagnostics/discovery but SHALL not produce a second attributed broker call:
the bound ledger is authoritative for that call. Broker metadata SHALL carry
no request arguments, URL, command or response body into the journal.
Denied tool attempts SHALL be marked refused, not represented as granted
holdings or successful uses.

#### Scenario: Two telemetry events still describe one call

- **WHEN** a broker call emits a start and completion in its ledger and the harness also emits item.started and item.completed
- **THEN** the journal has exactly one settled checkpoint for that call_id and no duplicate attributed harness call
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

#### Scenario: The ledger limit bounds acceptance rather than erasing calls

- **WHEN** 4,096 accepted calls have durable records and another request waits unread at the broker's input
- **THEN** the broker ends with "broker ledger exceeds the attempt limit" without accepting or forwarding a 4,097th call
- **AND** settlement preserves exactly the 4,096 recorded call outcomes and the failed-attempt cause; it fabricates no checkpoint for unread input
- **AND** a valid in-budget request denied for an ungranted tool or exhausted retention share instead has one Started, one refused Terminal and one settled refused checkpoint

### Requirement: CC3 v6 adds attribution without rewriting historical records

Implementation SHALL publish seat-record.v6.schema.json beside v5 and update
the store's append/export/verify dispatch consistently. The contract delta
defines new field shapes and dependencies. Historical records SHALL retain
absence, never backfilled from today's grants or inferred from a tool name.
The engine-line boundary SHALL follow the store's existing additive version
convention, with exact old/new tests and an explicit boundary chosen at U4
against then-current main; adding the schema requires no package-version change.

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
Started/terminal records remain private; the public checkpoint has only the
settled outcome. This keeps durable evidence private
(0071 rulings 3, 5, 7). Authoritative MCP rows appear at settlement; live
telemetry remains diagnostic and cannot certify completion. Native observed
checkpoints deliberately claim no completion.
The engine's journal fence remains single-writer; a broker opening forge.db
is rejected as an alternative.

Malformed JSON-RPC or an invalid tool identifier is a protocol refusal before
it denotes a capability call; it carries MB3's safe protocol cause, never raw
invalid identifiers in checkpoint fields. This does not excuse dropping any
well-formed denied call or a granted call whose child fails.

One public settled row satisfies 0065 ruling 8; private lifecycle records
supply crash evidence (0071 rulings 3, 5, 7–9). Durable acceptance bounds the
promise: a bounded reader cannot record unread input. Accepted local refusals
remain evidence; persistence failure cannot be declared successful.

Delay emission until complete consumers exist. v6 accepts a complete
group, not a private observation; forwarding new driver fields unchanged
would fail its closed fence. Preserve the fence and every valid legacy row
through preparation (0071 rulings 3 and 9).
