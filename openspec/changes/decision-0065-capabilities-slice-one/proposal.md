Status: proposed specification of accepted decision 0065, slice one.
Change: decision-0065-capabilities-slice-one.
Adopted: slice-0065-capabilities at 44430402, specification draft a84197cd
and design revision 3c5402be; every commit is retained, including the replay
and unit 1b through 5a47b090, unit 2 through 6a7044e5 and unit 2-fix through
b4839426. This specification visit adopts that history and specifies only
rebuild unit 3: typed lowering and private origins (3.1; the prerequisite
portion of 4.2). D5.3/D5.5/D5.6 and the existing refusal requirements remain
authoritative; earlier implementation and evidence keep their recorded scope.
Authority: [operator ruling, 2026-09-23](operator-ruling-2026-09-23.md).

## Why

Three councils exposed adjacent failures in reconciling recipe-authored harness
flags with engine controls. The operator has ruled “refuse, never reconcile”;
the rebuild follows that ruling. Unit 3 supplies the typed lowering and private
origin primitives needed to distinguish engine contributions from authored
bytes. The current two-part hands boundary cannot distinguish adapter templates
and generated local permissions. Expected state must survive independently of
the argv that later units will check.

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
- Specify unit 3's local lowering with exact mapped values, ordered private
  authored/template/local/hands/native segments, and expected state derived
  from typed inputs before serialization. Define fallible private decoding and
  exact reassembly, including identical bytes with different supplying origins.
  Unit 4 owns dispatch wiring; unit 12 owns authored-refusal activation. Existing
  unsupported-path guards stay until their delivery dependencies are proved.
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
seat-capability-resolution and native-capability-controls deltas, then the
unit 3 tasks/evidence notes. The accepted Rebuild units order and other deltas
remain adopted. Clarification answers and rejected alternatives are recorded
in the owning deltas' scenarios and Decisions; no council is reconvened.

Unit 3's production ceiling is exactly `crates/brokkr-runtime/src/agents.rs`,
`crates/brokkr-runtime/src/capabilities.rs` and
`crates/brokkr-protocol/src/native_controls.rs`. Tests belong only in runtime
`agents/tests.rs` and protocol `native_controls/tests.rs`. No new module,
public contract, shipped mapping, bundle/engine wiring, migration or refusal
activation enters this unit. A required edit beyond this ceiling must produce
a split before implementation; it cannot be hidden in a test or data file.

This specification closes no implementation task. Task 3.1 awaits exact-value
and full-reason tests, observed baseline reds, independent compiling mutations,
restored passes and applicable gates; 4.2 stays open until unit 4. Existing
unit 2 admission guards cannot be removed merely because a lowering primitive
exists. Final serving-command and counterfeit-origin proofs retain their later
owners. No `returned_from` is present in this run, and no upstream defect in
the accepted unit order was found.

External exact coverage, macOS and remote CI remain pending until their actual
results exist. Supported hosts are Linux and macOS (decision 0063); grants and
frozen contracts, fixtures, policy, reference and extensions do not change.
Decision 0066 stays proposed; only the operator can accept it. No archive,
push or release is authorized.
