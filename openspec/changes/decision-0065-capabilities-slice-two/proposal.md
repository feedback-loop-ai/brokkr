Status: proposed specification and design; implementation awaits the operator.
Change: decision-0065-capabilities-slice-two.
Provenance: [evidence.md](evidence.md#scope-and-source).
Authority: [operator rulings R1–R5](operator-ruling-2026-10-03.md).

## Why

Slice one built decision 0065 rulings 1–5 and capability identity, but MCP
grants still refuse, gates lack capability class checks, and calls lack
capability attribution and retained evidence. Slice two specifies those
remaining protections before narrow implementation PRs begin.

## What Changes

- Brokkr supplies `broker serve`, launched by the harness from the engine's
  MCP configuration beside hands, one `cap-<capability>` per held MCP
  capability. Proposed decision 0077 amends 0065's "launched by the engine".
- **BREAKING:** every model serving path excludes ambient MCP configuration;
  U0 measures the applicable mechanism per harness, including Codex and dsh.
  Unmeasured isolation refuses, even with no requests. Generated Claude, Codex
  and dsh scaffolds migrate before U1g activates strict admission.
- MCP holdings require workspace hands under `namespace`, network false,
  strict configuration and supported carriage. Exact safety refusals and
  requires/wants compatibility outcomes remain distinct.
- Gates never hold writes, and hold egress only when the realm explicitly
  names their office, for native and MCP capabilities. Requesting charters
  carry the DATA rule in the paragraph naming the capability.
- Native checkpoint consumers and the additive v6 store fence land before
  drivers emit normalized observations; intermediate merges preserve valid
  legacy checkpoints. Every granted call gains capability/dialect/tool attribution. MCP calls
  pass through a protected durable ledger; the engine alone appends journal
  checkpoints, one settled record per accepted broker call. Secret-bearing
  holdings require measured read isolation; executable startup inputs and
  evidence across every managed writer require protection. Opted-in,
  non-vetoed responses have a fixed aggregate budget, are masked, content-addressed at
  `.forge/artifacts/sha256/<hex>`, and openable through inspect. Fatal broker
  session outcomes remain durable even when the harness reports success;
  responses that cannot be masked without changing their data shape refuse.
- Later implementation adds seat-record v6, run-manifest v12 and **the next
  realms version after v7** for `retain: false`. Tool-dialect v1 already
  defines connection/version/secrets/retained; retain those fields in typed
  state instead of discarding them. No new tool-dialect version is needed.
- Repair both native-binding panic assumptions before activation. Keep
  slice-one D4 precedence, D11's nonempty-restriction deferral and native OFF.
- Deliver each bounded implementation unit as a signed PR from main through
  the merge queue. U9 alone lifts MCP compile refusal; U1–U4 can protect
  existing native paths independently.

## Capabilities

### New Capabilities

- `strict-mcp-isolation`: measured exclusion of ambient servers.
- `mcp-capability-broker`: ownership, admission, filtering, secrets and cleanup.
- `gate-capability-policy`: class rules and same-paragraph DATA checks.
- `capability-call-checkpoints`: selected-attempt attribution.
- `capability-response-retention`: durable folding and verified retained bytes.
- `slice-two-contracts`: typed dialect data and additive public contracts.
- `slice-two-delivery`: staged units, proofs and operator handoff.

### Modified Capabilities

None. Slice one's deltas are unarchived at this base. These seven new deltas
continue them; the earlier MCP exclusion is lifted only by U9, and historical
slice-one artifacts/evidence are not rewritten.

## Impact

The specification touches proposal, deltas, design, tasks, proposed decision
0077 and [evidence.md](evidence.md#scope-and-source). Production, tests and frozen
contracts remain unchanged. The operator record, decision index and 0065
amendment pointer are preserved. U0 and runtime proofs remain implementation
work; document gates qualify no harness and do not accept 0077.

## Decisions

Retain realm-only broker authority, namespace refusal, native gate protection,
engine-only journal and masked retention. The next realms version after v7
preserves #487's reservation; tool-dialect v1 defines the needed fields. URL
execution, native retention, wider boundaries, slice three and D11's nonempty
restriction transport remain outside scope.

Generated scaffold declarations and instructions migrate before strict
admission, with fresh-scaffold scenarios under SI2. Driver emission waits
until the store and all attribution consumers exist, with CC1/SC4 boundary
proofs. One durable evidence record holds specification history; delivery
scenarios state behavior. Choices and rejected alternatives are in
[design Decisions](design.md#decisions) (0071 rulings 3, 5 and 9).
