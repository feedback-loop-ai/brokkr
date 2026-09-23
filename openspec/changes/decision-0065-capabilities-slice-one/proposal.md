Status: proposed specification of accepted decision 0065, slice one.
Change: decision-0065-capabilities-slice-one.
Adopted: slice-0065-capabilities at 44430402, specification draft a84197cd
and design revision 3c5402be; every commit is retained, including the replay
and unit 1b through 5a47b090, and unit 2 through 6a7044e5. This specification
visit adopts that history and records only rebuild unit 2-fix under task 2.1.
The third review left S1/A1 implementation omissions; their residual is not
accepted. D5.3/D5.5 and the existing refusal requirements remain authoritative.
Authority: [operator ruling, 2026-09-23](operator-ruling-2026-09-23.md).

## Why

Three councils exposed adjacent failures in reconciling recipe-authored harness
flags with engine controls. The operator has ruled “refuse, never reconcile”;
the rebuild follows that ruling. Unit 2-fix repairs the remaining native-control
and root-selector admission omissions against the existing specification.

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
- Specify unit 2's strict `tools.allow` / `tools.sandbox` decoding for agents
  and executable sites: distinguish omission from empty, admit only local
  narrowing, and preserve realm/boundary authority with complete refusal causes.
  These are the D5 prerequisites for the later lowering and migration units.
  Unit 2-fix applies their existing no-competing-control rule to actual resolved
  native ON/OFF/restriction contributions and canonical root-changing options
  (`--cd` / `-C`), preserving valid native denial controls at compile admission.
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

The complete adopted change retains this proposal, six deltas, design, tasks,
evidence and proposed decision 0066. This specify visit amends the proposal,
the owning seat-capability-resolution delta's acceptance scenarios and Decisions,
and tasks/evidence for **2-fix**. No specification defect or new semantic ruling
was found. The other deltas and accepted Rebuild units order remain adopted.

Unit 2-fix's production scope is `crates/brokkr-runtime/src/bundle.rs` and,
only if exposing resolved native contributions requires it, `capabilities.rs`
in that src root. Tests stay in `bundle/agent_tests.rs`. Preserve the existing
decoder, narrowing, authority checks and matching sandbox positives. Record
each of the chief's eleven defect cases as an exact refusal regression with
its own compiling mutation and restored pass. Reopen task 2.1's aggregate and
its admission/proof/fresh-gate portions only for this repair; no implementation
or proof is claimed by this specification visit. Exceeding these files or this
scope requires a split before implementation. Units 13–15 cannot absorb it.

The current run has no `returned_from`; it carries the chief's supplied S1/A1
findings from run `build-decision-0065-slice-one-re-e332dc8d`. No council is
reconvened here. Historical passes retain their tested scope, while external
exact coverage, macOS and remote CI remain pending. Supported hosts are Linux
and macOS (decision 0063); grants and frozen contracts, fixtures, policy,
reference and extensions do not change. Decision 0066 stays proposed; only
the operator can accept it. No archive, push or release is authorized.
