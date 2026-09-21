Status: proposed specification of accepted decision 0065, slice one.
Authority: operator ruling and controller-adopted cut, 2026-09-21.
Change: decision-0065-capabilities-slice-one.

## Why

Codex can search from the provider's servers even when a seat's workspace has
no network; existing realms neither authorize nor disclose that egress.
Accepted decision 0065 makes capabilities the realm's to grant, with no
grandfathering, and the operator has admitted closing this defect as the
highest-priority first slice.

## What Changes

- **BREAKING:** Everything beyond the existing hands contract is denied unless
  held through an applicable realm grant, including provider-native tools.
  Realms v1–v5 continue loading and grant nothing.
- Add tool-dialect.v1 and forge.realms/v6 beside the frozen contracts. A tool
  dialect serves an abstract capability; the realm selects the dialect,
  tools subset, office scope and dialect-schema-validated restrictions.
  Operator-owned capabilities/<name>.json definitions supply the abstract
  reads/writes/egress class sets independently of requests and serving
  dialects; defining a name grants nothing. Dialect egress uses decision
  0036's separate local/contracted/uncontracted vocabulary.
- Agents and every executable seat form request requires/wants capabilities.
  The compiler resolves office asks minus seat subtractions, intersected with
  realm grants to that office. Requirements refuse with named reasons;
  optional drops become manifest notices. Schema-valid restrictions that a
  native binding cannot express make that binding incompatible: requires
  refuses, wants drops with native OFF, and an unused grant enables nothing.
  Invalid grant data and impossible native denial still refuse independently.
  Legacy server and native-tool declarations cannot bypass that authorization.
- Add adapter-owned native ON/OFF declarations and explicit unsupported or
  unmeasured reasons. Ship provider-native dialect files only. Codex denial
  composes the measured -c web_search="disabled" on cold and eligible resume
  argv, boxed and unboxed, without weakening resume eligibility or its config
  guard. A native capability that cannot be disabled refuses when ungranted.
- Report grants, dialects, office scope, ungranted installed native
  capabilities and measurement gaps per realm in doctor. Render each seat's
  held and not-held capabilities; capability results are DATA, never
  instruction, including in charters of offices that use them.
- Add run-manifest.v11 (v10 is the inspected latest version) for per-seat
  holdings, abstract-definition identity/digest, dialect identity/digest,
  tools, restrictions and notices. Changes to consulted abstract definitions
  or realm grants, including unused realm grants, move the bundle identity.
  Re-pin witness, compose and affected charter digests only from actual bytes
  and compiles, with history reasons.
- Define the complete mcp kind in the schema, but refuse every realm grant
  selecting it with a reason naming the absent second slice.

## Capabilities

### New Capabilities

- tool-dialect-contract: implementation kinds, abstraction classes, disclosure
  metadata, restriction schemas and versioned contract boundaries.
- realm-capability-grants: realm-only authority, legacy empty grants, office
  scope, tool subsets and uninterpreted restriction pass-through.
- seat-capability-resolution: portable asks, subtraction, strict lints,
  compiler refusals, optional notices and fallback authorization.
- native-capability-controls: adapter evidence, native enable/disable
  composition, Codex cold/resume denial and explicit measurement limits.
- capability-doctor: per-realm inventory of grants, installed native
  capabilities, denials, unsupported controls and unmeasured guarantees.
- capability-manifest-and-prompts: pinned capability identity, notices,
  seat-visible holdings, data-only charters and measured digest migration.

### Modified Capabilities

None. Existing boundary and hands requirements remain intact; capability
authorization is an additional axis, not a boundary change.

## Impact

Implementation will touch Rust realm representation, agent/adapter loaders,
bundle resolution, launch/prompt integration and doctor under crates/, plus
adapters/, affected agents/charters, new capabilities/ and dialects/tools/
data, additive contracts and focused operator guides. The expected files and existing proof
suites are inventoried in .forge/tasks/0065-capabilities-slice-one.md.
No dependency is proposed; any later addition needs a stated justification.
Supported hosts remain Linux and macOS under decision 0063.

This returned specify visit adopts decision-0065-capabilities-slice-one and
amends only this proposal and its six specification deltas, in that order.
Design, tasks, code, new contracts and digest updates are subsequent
artifacts/work, not claims made by this draft. The checkout
already names slice-0065-capabilities; this return starts at e40a716a, the
initial specification commit on top of 5347c667, which accepts 0065. The
existing accepted decision is adopted as authority, not re-authored or demoted; any additional semantic ruling needs a separate proposed decision.

## Decisions

The controller's three-slice cut is adopted as given, not re-triaged. Slice
two owns MCP brokers outside the box, gate-class capability policy,
capability/dialect checkpoint names and retained responses. Slice three owns
comparison/wager reporting and capabilities: equal. Reserved hands neither
grants nor moves the workspace tool. The data-only instruction is in this
slice even though the remainder of ruling 7's gate policy is deferred.

Answers and reasoned refutations of specification ambiguities are encoded as
scenarios in the owning deltas: a realm grant alone is not a seat holding;
an optional MCP grant still refuses; a native adapter key is not a grant;
DSH/LaneTally unknown inventories are not empty inventories; and a composed
Codex resume control is not live proof. This return answers clarify findings
CQ1 and CQ2 below and in the owning scenarios; no council positions were
supplied. The accepted decision and controller cut remain unchanged.

An explicit capabilities map on an agent-backed seat is the requested subset
of its office's map; omission inherits, and an empty map subtracts everything.
This uses the commissioned requires/wants vocabulary without a second grant
syntax. Strength changes are refused because subtraction removes an ask; it
does not rewrite it. A referenced agent name identifies its office; an inline
site uses its stable site label. The realm/subtraction scenarios own these
answers, so design must preserve their observable authority boundaries.

Reasoned refusals: preserving legacy concrete MCP requests as a second launch
path would let an agent choose a server; treating existing native-tool
permissions as grants would grandfather undeclared egress. The owning
resolution delta instead requires explicit migration. Making the new Codex
OFF pair force every otherwise eligible resume cold would evade the required
resume proof; the native-controls scenarios explicitly reject that outcome.

CQ1 — restriction compatibility follows decision 0065 ruling 5. Validate
all selected-realm grants first: malformed restrictions and the unbuilt MCP
kind refuse even for wants or unused grants. For a schema-valid native
restriction the serving binding cannot express, a remaining requires refuses,
a wants is dropped with its full reason and supported native OFF, and an
unused grant stays pinned but inactive. The independent impossible-OFF
refusal still wins over an optional drop. Rejecting every valid but unusable
optional binding would contradict ruling 5; discarding only its restriction
would violate rulings 3–5. The realm delta replaces its contradictory blanket
refusal with this precedence and complete diagnostic scenarios. Existing
local tool-permission restrictions retain their unconditional refusal.

CQ2 — abstract classes have one operator-owned declaration source, independent
of grant selection: capabilities/<name>.json under the active realms map's
configuration directory, or the operated repository root when no map exists.
Each file defines its abstract name and classes; neither an agent, a recipe,
a provider nor a dialect supplies a missing definition. The tool-dialect
scenarios give the complete operator-library-docs example, lookup/consistency
scope and missing/conflicting metadata refusals. Request grammar stays
name-to-requires/wants; semantic library lint resolves those names before
optional dropping or provider choice. A valid definition without a grant
still yields a named required refusal or optional drop. No-definition legacy
inputs that request and grant nothing still deny known native tools; absence
is never evidence of an empty native inventory. Consulted definitions are
pinned even for dropped requests. This is the representation of ruling 1's
abstract classes, not a new grant authority or an implementation catalogue
inside a recipe. Provider-derived classes were rejected because they cannot
classify an ungranted request and could change the abstraction when a dialect
changes. All answers remain proposed specification choices for the later
design phase, not amendments to the operator's accepted decision.

The repository's realms.json must continue granting nothing. Protected
policy/phase-machine.json, policy/schemas/, fixtures/, reference/, extensions/,
the event-envelope schema and issue-226 task ledger remain untouched.
Existing contract versions and historical evidence remain byte-for-byte.

## Evidence and delivery obligations

Read decision 0065 in full beside 0043, 0046, 0036, 0012 and 0016; README,
0004, 0005, 0009, 0063 and the realm house; dialects/openspec.json and its
specify/return instructions; dialect.v3; current realm/agent/adapter loaders,
Codex launch and resume guards, doctor, manifest generation and digest tests.
Current researcher tool permissions name webfetch/websearch: migration must
remove that concrete-tool authorization path, not silently preserve it.

The controller's .forge/tasks/controller-codex-web-search-switch-2026-09-21.json
establishes only codex-cli 0.154.0 cold exec: the default searched, while
-c web_search="disabled" removed the tool. The following remain controller
measurements owed, and delivery notes must report their actual status:

| Evidence gap | What is and is not established |
| --- | --- |
| Codex resumed web search | Compose and deterministically prove the OFF argv on an actual eligible resume; live enforcement on a resumed session is unmeasured. |
| Codex explicit ON values and other versions | Default-ON cold behavior was measured; no other configuration value or version is thereby verified. |
| Claude native controls | Empty native tools under strict MCP configuration is adapter data, not a live denial/enablement measurement. |
| DSH native inventory and controls | Unsupported mcp/tool_permissions establishes neither absence of native egress nor a measured OFF control; declare unmeasured with that reason. |
| LaneTally native inventory and controls | Wrapper/native controls are unverified; Claude data or wrapper argv forwarding does not measure them; declare unmeasured. |

Implementation acceptance includes full reason/notice assertions and removal
proofs for compiler enforcement, boxed/unboxed Codex cold and eligible resume
argv in both grant states, strict library/schema tests, per-realm doctor and
prompt tests, and actual grant/dialect/restriction digest movement. Each
removal proof must fail at the intended assertion, then pass with enforcement
restored; a compile error or unrelated failure is not proof.

Required validation remains cargo fmt --all -- --check; cargo clippy
--workspace --all-targets --all-features --locked -- -D warnings; each of the
seven crate suites, crate-scoped; cargo test --workspace and the all-features
locked workspace suite; the bundles/self compile and every re-pinned compile;
openspec validate --all --strict; and the unchanged literal-100% exact coverage
gate, including every added production line. Host/CI coverage evidence remains
pending until it exists where nested boundaries can execute. Notes describe
observations and never waive, instruct or substitute for a gate. There is no
release, publication, profile update or push in this commission.

## Specification validation

The initial specify visit passed strict validation of this change and
openspec validate --all --strict --no-interactive: 16 items passed, zero
failed. OpenSpec reports proposal and specs complete; design and tasks remain
subsequent phase work. Existing informational archive notices for issue-226
spec deltas were reported by the validator and those artifacts were unchanged.

On the initial visit, Rust formatting, clippy, both workspace test commands,
all seven crate-scoped test commands, the bundles/self compile and scripts/coverage-exact.sh were
attempted through the workspace hands. Every attempt stopped with cargo:
command not found (exit 127); no Rust suite, compilation or coverage result is
claimed. These are environment limitations in this specification visit, not
observed implementation failures or changes to the required gates.

This return passed strict validation of the amended change and strict
all-item validation: 16 passed, zero failed. Proposal and all six deltas are
complete; design and tasks remain later-phase work. git diff --check is clean.
The validator also reports informational long-requirement notices and the
same unrelated issue-226 archive notices; neither is a validation failure.

Return attempts of formatting, clippy, both workspace test commands, the
seven actual crate suites and bundles/self compilation could not start
because cargo is absent from the workspace PATH. The exact-coverage script
also stopped at cargo with exit 127. No Rust, compile or coverage pass is
claimed. No implementation, provider measurement, dependency change or digest
re-pin is performed by this specification return; all controller measurement
obligations above remain outstanding.
