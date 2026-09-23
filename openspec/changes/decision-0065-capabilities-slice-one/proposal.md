Status: proposed specification of accepted decision 0065, slice one.
Change: decision-0065-capabilities-slice-one.
Adopted: slice-0065-capabilities at 44430402 and specification draft a84197cd;
every commit is retained. This design visit amends that draft with reasons.
Authority: [operator ruling, 2026-09-23](operator-ruling-2026-09-23.md).

## Why

Three councils exposed adjacent failures in reconciling recipe-authored harness
flags with engine controls. The operator has ruled “refuse, never reconcile”;
this highest-priority revision corrects the specification and plans the rebuild.

## What Changes

- **BREAKING:** Compilation refuses authored capability-bearing options for
  Claude, Codex, DSH and LaneTally's Claude path, regardless of grant, value,
  polarity, alias or spelling. “Nothing is merged.” Tools come only from typed
  agent/seat declarations and the realm grant, composed by the engine.
  The complete refusal catalogue and spelling rules live in the realm delta.
- Parse every adapter-declared ON and OFF argv at load, including unused
  dispositions. Parse the complete composed command back before each launch;
  it must express exactly the planned capability state or refuse. Proposed
  decision 0066 now makes its launch-proof principle total.
- Refuse consumed inputs whose filesystem-resolved target escapes the recipe's
  declaring tree. “It is not pinned and admitted.” Revoke the pinned outward
  link permission identified by third-council R3/C7; require regular, readable,
  identity-bound input bytes and owner-bound charter verification at use.
- Doctor submits the whole plan to the launch composer and reports its
  admission or refusal, including interactions, without inventing a partial
  plan or promising live enforcement from static composition.
- Carry private argument origins through runtime dispatch, and consume a checked
  final command without further argv edits. Bind containment checks to the file
  supplying the consumed bytes; a path check followed by an unchecked reopen
  is insufficient. These clarify enforcement boundaries, without a new public
  contract or subsystem.
- Migrate shipped inline tool and permission flags to typed declarations
  before refusal lands. The file-by-file inventory is in design's Migration
  Plan, including preflight, fast, node, verify and Codex recipe modes.
- Preserve 0065's no-grandfathering, off-by-default and realm-only rules,
  requires/wants resolution, strict source decoding, loaded-library lint,
  additive tool-dialect v1 / realms v6 / run-manifest v11 contracts, capability
  identity, data-only prompts and slice-two MCP refusal. This realm grants none.

## Capabilities

### New Capabilities

- `tool-dialect-contract`: abstract definitions and concrete dialect contracts.
- `realm-capability-grants`: realm authority and authored-option refusal.
- `seat-capability-resolution`: typed requests, subtraction and migration.
- `native-capability-controls`: load validation and exact final-command proof.
- `capability-manifest-and-prompts`: identity, containment and verified inputs.
- `capability-doctor`: whole-plan admission reporting and evidence limits.

### Modified Capabilities

None; these six deltas remain the adopted slice's new capabilities.

## Impact

This visit changes only this proposal, six deltas, design, tasks, evidence and
proposed decision 0066. No code, tests, shipped data or witness bytes change.
Design ends with one dependency-ordered list of one-visit rebuild units; unit 1
rebases onto current origin/main and re-pins measured identities. Implementation
and its test/coverage gates remain owed. Supported hosts are Linux and macOS
(decision 0063). Frozen contracts, fixtures, policy and reference stay frozen.

There is no returned_from in the supplied run context. The explicit operator
ruling and all three completed third-council positions supply this visit's
findings; their individual dispositions are recorded under design Decisions.
Earlier chief rulings remain evidence, with superseded remedies identified.
The robustness and simplicity design positions are explicitly reconciled in
design Decisions; their useful safeguards share the existing bounded machinery.
The security hold remains unresolved; this documentation is not a repair pass.
Only the operator can accept 0066. No archive, push or release is authorized.
