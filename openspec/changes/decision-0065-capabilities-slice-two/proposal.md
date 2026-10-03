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

This design visit adopts the committed change at 1c96ca2c. The supplied
returned_from is clarify's clear review of the R-G/R-H repairs; preserve that
answer and its scenarios. Both current council positions were read whole and
reconciled against checked source in design D1. Robustness's startup-environment
collision is a construction and proof obligation under existing MB3: protect
the child environment through secret injection, using its existing refusal.
Simplicity's discovery work moves into U7c/U7d without losing tests or task IDs.
The owning scenario precedes the revised design and tasks. Design D11 records
the current analysis; historical upstream findings and their adoption retain
their dated scope. No unresolved earlier-owner defect was found this visit.

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
Council amendments: one qualifying DATA declaration per capability replaces
the every-mention rule (GP2); private begin/terminal evidence yields one public
settled checkpoint (CC2/SC4/CR4). MB2–MB4 add measured secret-read isolation,
protected startup inputs and protocol correlation. CR2–CR4 require protection
across managed writers, closed ledger lifecycle, commit-confirmed settlement
and aggregate retention bounds. These repair earlier specification omissions,
not implementation exceptions. Design D1 records each claim and reason.
Specify disposition, 2026-10-03: adopt those owning-spec repairs. Clarify the
bounded acceptance point in CC2/CR3 and the request-identity edge in MB3/SC4;
record the returned finding's closure in SD1 and design D11. Repair 0077's
missing Built issue marker using its existing open strictness prerequisite
#467; the complete remaining work is this change's tasks, not closed slice-one
PR #319. The operator alone accepts decision 0077 and the completed plan.

Council disposition, 2026-10-03, reviewed 7f36e922: adopt typed private
session closure and conservative unsafe-response refusal, not the legacy numeric
masking residual. Retain the existing masker and canonicalizer with a broker
edge check. Adopt the four bounded PR combinations and two module cuts with
all proof ownership preserved; that design contained 44 PRs. These proposals
claim no runtime protection or acceptance of 0077 (0071 rulings 2–9).

Specify disposition of b310e094, 2026-10-03: adopt R-H's conservative refusal
and R-G's durable failed-session closure after rechecking secret.rs:637–668,
secrets.md:50–52, process.rs:499–520 and engine/checkpoints.rs:169–248.
The exact cause is "MCP response cannot be safely masked"; unsafe forwarded
responses have a failed call, no digest and no delivered or staged body.
Retain the numeric/string/control and unsafe-correlation scenarios and the
independent engine-failure proofs. Refuse the alternatives of inheriting the
numeric residual, changing scalar values, treating precision as secrecy or
treating harness success as broker success (0071 rulings 3, 5, 8, 9).
The specify result is drafted with inputs.change; all 100 implementation and
measurement tasks remain open and 0077 remains proposed.

Design disposition, 2026-10-03, reviewed 1c96ca2c: adopt R-I at MB3 with
independent HOME/TMPDIR collision controls before store lookup or spawn.
Reserve the broker builder's fixed environment keys without changing decision
0012's shared name grammar or adding a plaintext accessor. Combine discovery
selection with U7c and rendering with U7d because those rows already own the
required files; retain the full proof inventory in 43 PRs and all 100 open
tasks. R-G/R-H, R1–R5, contract reservations and U9b-only enablement stand
(0071 rulings 2–6, 8–10). This is a proposed design, not operator acceptance.
