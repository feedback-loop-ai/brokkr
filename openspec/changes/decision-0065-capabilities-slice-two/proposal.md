Status: proposed specification and design; implementation awaits the operator.
Change: decision-0065-capabilities-slice-two.
Adopted: the twelve staged first-visit documents at main 2a23488b19f6dea271264f6a85b40d46a84af39e, on slice-0065-two-spec.
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
  Unmeasured isolation refuses, even with no requests.
- MCP holdings require workspace hands under `namespace`, network false,
  strict configuration and supported carriage. Exact safety refusals and
  requires/wants compatibility outcomes remain distinct.
- Gates never hold writes, and hold egress only when the realm explicitly
  names their office, for native and MCP capabilities. Requesting charters
  carry the DATA rule in the paragraph naming the capability.
- Every granted call gains capability/dialect/tool attribution. MCP calls
  pass through a protected durable ledger; the engine alone appends journal
  checkpoints. Opted-in, non-vetoed responses are masked, content-addressed at
  `.forge/artifacts/sha256/<hex>`, and openable through inspect.
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

This visit adopts proposal then deltas, clarifies their scenarios, writes
design.md and tasks.md, and analyzes their consistency using the dialect's
own instructions. Design records the unit order, every planned file, Hot
files, alternatives and analysis. No council position files or returned_from
were supplied; no council vote is invented.

Only these documents, the verbatim ruling record, proposed decision 0077,
its index row and 0065's amendment pointer are committed. No production,
test, contract, adapter, fixture, policy or reference byte changes. No U0
experiment or implementation gate is claimed. In-box gates are
`git diff --check` and `openspec validate --all --strict`; plain unsigned
`git commit` is authorized. The operator runs format/typos outside the box
and signs the squash before pushing.

## Decisions

Adopt the first draft's realm-only broker authority, hard namespace refusal,
native gate protection, engine-only journal, masked retention and independent
native protections. Reconcile three draft choices with reasons:

1. Replace the draft's realms v7 veto with **the next realms version after
   v7**: #487 owns v7. Preserve its provisional-office meaning when adding the
   new version. Never reserve a future merge number by editing a frozen file.
2. Reject mandatory tool-dialect v2: v1 already has every required field,
   environment-bound names and no promised runnable MCP implementation.
   Unsupported URL/argv-reference execution has an exact compatibility
   refusal, not a reinterpretation of old bytes (SC1 scenarios).
3. Replace the first visit's unavailable signing/format/typos obligations
   with this commission's unsigned document commit and external handoff
   (SD4 scenario). Its earlier failures stay historical, not current blockers.

URL execution, native response retention, wider boundaries, slice-three
comparisons and D11 nonempty restriction transport remain outside scope.
The operator alone accepts decision 0077 and the completed plan.
