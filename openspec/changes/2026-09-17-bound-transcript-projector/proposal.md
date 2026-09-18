Status: proposed

## Why

Issue #277 is the unstarted availability residual from #222: bounded DSH
source input can expand into millions of retained projector events, affecting
the operator's console on every read and refresh. The operator's 2026-09-17
ruling authorizes both packed-member merging and a fixed structural charge;
the reported memory-pressure incidents make that combined repair urgent.

## What Changes

- **BREAKING:** Collapse consecutive surviving text or reasoning members of
  one validated version-zero DSH packed row into a displayed chunk. Keep
  member-level citation identity and suppression; a suppressed member splits
  a run. The reading delta defines the rule once, and CLI/TUI deltas consume
  it. Ordinary rows keep their current turn boundaries.
- **BREAKING:** Keep `DISPLAY_CAP` at 4,000,000, but state its unit as display
  accounting bytes: 512 structural bytes (`DISPLAY_EVENT_COST`) per final
  content-bearing event/turn plus every emitted block's UTF-8 text bytes.
  Enforce bounded event retention during projection as well as in the
  returned document, without ending classification or association early.
- Release blockless DSH events after recording their diagnostic and sequence
  facts. Preserve duplicate-sequence ambiguity and all suppression outcomes.
- Strengthen the Codex shared-record-id regression proof so it observes the
  association pass while its keys are alive, covering the defect repaired by
  `5aee618`, as well as the existing before/after checks.
- Enforce the existing DSH discovery limit at a newline-less EOF: 65,536
  header bytes fit and 65,537 refuse with `discovery-limit`.
- Require reproducible before/after allocation evidence on ordinary
  one-member-per-token streaming data and deliberate removal proofs for each
  bound and both low findings. Arithmetic from the issue is motivation,
  not a measurement result.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `transcript-reading`: packed chunk coalescing, structural display accounting,
  bounded projector retention, unchanged blockless-event semantics and exact
  header boundary scenarios.
- `transcript-command`: selectable indices address the coalesced, capped
  projection instead of individual packed members.
- `transcript-tui`: both reading doors and refresh consume those same chunks,
  indices and notices.

## Impact

Production implementation is confined to `crates/brokkr-view/` and the named
DSH header boundary defect in `crates/brokkr-cli/src/ui.rs`. CLI and TUI
behavior changes through the shared projection; no new reader interface or
serialized field is introduced. A later house decision must carry status
`proposed` and supplement decision 0055 ruling 3's member-per-turn and
text-only rules; the operator alone accepts decisions.

### D0 return: dependent test scope requires an explicit fence amendment

The design returned `upstream` at `555ad9f8` because the original Impact
inventory omitted a necessary CLI test correction. This specify visit adopts
that finding. At the unchanged production base, the existing
`crates/brokkr-cli/tests/transcript_command.rs::packed_dsh_members_have_separate_selectable_turns`
projects an uncited packed `["a", "b", "c"]` row followed by user text `q`.
It requires four turns, `b` at index two/stamp `999`, `q` at index four,
and packed/ordinary index-two equality. The adopted reading rule instead
requires `abc` at index one/stamp `1000`, then `q` at index two/stamp `2000`.
The command delta now pins those exact inputs and outputs, including the
ordinary-row control and the reason-bearing refusal at packed index three.

The following is the concrete **test-only scope amendment** to the
commission's fence. The second-visit commission for #277 authorizes it
explicitly: `crates/brokkr-cli/tests/transcript_command.rs`,
`crates/brokkr-cli/tests/transcript_surfaces.rs` and
`crates/brokkr-cli/src/tui/tests.rs` are in scope, and correcting their
packed-turn assertions to the merged shape is part of the work. The
amendment grants no production change beyond the named `ui.rs` header
repair.

| Dependent test file | Narrowly required correction or proof |
|---|---|
| `crates/brokkr-cli/tests/transcript_command.rs` | Replace the contradictory packed-member selection expectations with the two-turn packed result and four-turn ordinary control; cover whole/selected JSON and the retained 7,797 versus refused 7,798 boundary with the exact reason and notices. |
| `crates/brokkr-cli/tests/transcript_surfaces.rs` | Derive a real synthetic DSH source through the shared reader and compare coalesced whole/selected CLI output, TUI pane indices, both doors and structural-cap notices against that same result. |
| `crates/brokkr-cli/src/tui/tests.rs` | Prove the packed text/reasoning two-chunk doors and the `abcd` to `a`, `c`, assembly refresh using projected source snapshots; keep generic six-turn navigation coverage without presenting injected turns as packed decoding evidence. |
| `crates/brokkr-cli/src/ui/tests.rs` | Extend only the already commissioned header repair's exact-limit and cap-plus-one cases, with and without newline, under both admitted versions; assert `DiscoveryLimit` and its metadata. |

The first three test files are the requested extension; the last makes the
existing header-test dependency explicit. This adds no CLI/TUI production
work beyond the named `ui.rs` repair and no exception to the exclusions below.
The existing commission's tests-required rule explains why these dependencies
must be resolved, but cannot silently erase its express file fence.

Reasoned refusal: keeping the old four-turn packed assertion, undoing merging
in the command, skipping the contradictory test, or weakening workspace
validation would violate the adopted shared-projection contract. This visit
therefore records the upstream scope dependency rather than disguising it as
an implementation task. The owning design decision D0 and its migration plan
retain that authorization prerequisite. No test or production file is changed
by this specification return.

This return amends the adopted proposal, dependent command/TUI scenarios and
the existing design's D0 status in dependency order. The reading delta already
owns the correct merge rule and is retained unchanged. Creation metadata is
adopted, not recreated. Tasks, the proposed house decision, implementation,
living-spec synchronization and measurement evidence remain subsequent work.

The scope excludes `adapters/codex.json`,
`crates/brokkr-protocol/src/adapters*`,
`crates/brokkr-runtime/tests/witness_digests.rs` and
`openspec/changes/2026-09-09-226-session-resumption/`. Frozen contracts,
`policy/phase-machine.json`, `policy/schemas/`, `reference/` and `fixtures/`
remain unchanged; synthetic measurement data belongs outside the frozen
corpus. Rust-only decision 0009 remains in force. Nothing changes the
journal, provider execution, session resumption or source ownership.

## Evidence and completion criteria

Read at base `5ef4a842`: `README.md`, decisions 0004, 0005, 0009, 0055 and
0061, the three living transcript capabilities, the projector and its tests,
the header reader, and commits `b809e02` and `5aee618`. Current code constructs
one event per nonempty packed member; `display_cap` charges text only and
runs after the event/turn vectors are built. Ordinary DSH collection retains
blockless events although their sequences have already been observed. The
Codex regression checks shared identity before and after association but
never while association keys exist. The header reader admits cap-plus-one
bytes at EOF when its separate overflow flag is false.

The proposed 512-byte charge rounds above the issue's approximately 330-byte
per-event audit estimate, allowing structural overhead without lowering the
cap. It is a portable accounting charge, not a claim that all allocations,
source parsing, association metadata or the process RSS fit in four MB.
The implementation evidence must measure those costs rather than present
that estimate as an observation.

Before implementation can be reported complete:

- Generate a valid version-zero DSH session with one packed member per token.
  Record its exact byte size, token/member/row counts and generator parameters.
  Measure the unchanged base and completed projector on identical bytes,
  naming the allocation instrument or defensible peak-memory proxy, measured
  scope, build profile, commands and limitations. Record actual before/after
  numbers for peak and retained projection size/count; separate fixture
  construction and source parsing from projected-event retention where the
  method allows. Add tiny-call and blockless-row probes for shapes (b)/(c).
- For every test claiming a bound, remove the particular protection it claims,
  run the targeted test, record its relevant failing assertion, restore it and
  rerun successfully. Cover merging independently of structural charging,
  structural charging independently of merging, transient event retention,
  and the blockless guard. Reintroduce `5aee618`'s association-time id copying
  while retaining row-level sharing, observe the strengthened test fail during
  the pass, restore and pass. Restore the header off-by-one to show its
  regression fails on the reason-bearing `discovery-limit` assertion. A build
  error or an unrelated refusal is not a removal proof.
- Preserve suppression, source order, complete-prefix diagnostic/refusal
  precedence, both admitted DSH versions, same-snapshot surface agreement,
  exact equality/overflow and UTF-8 boundaries. Cover every new production
  line, branch and function; never lower or exclude from the coverage gate.
- Run `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
  `cargo test --workspace --all-features --locked`, both `bundles/self` and
  `bundles/verify` compiles, and strict OpenSpec validation. Use writable
  disk-backed `TMPDIR` outside the repository; unset `GIT_CONFIG_COUNT` and
  `GIT_CONFIG_VALUE_0` for git, including git launched by tests. Stage any
  executable beside its target and rename it into place before execution.
- Report available in-box exact-coverage numerators and denominators honestly.
  Boundary namespace skips make equality unattainable in this box (#286);
  the controller's host exact-coverage measurement and final-head remote CI
  remain pending until their results exist. No seat pushes, merges or starts
  another Brokkr run.


## Specification validation

The initial specify visit (`b82d2479`) passed strict change validation; the
subsequent design visit (`555ad9f8`) passed all 15 OpenSpec items. On this
return, the existing proposal/specs/design are adopted and tasks remain
unauthored. Existing modified requirement blocks and scenario identities are
preserved. The command example is made concrete using the contradictory test's
source; the TUI example requires the actual shared projection. Neither changes
the reading delta's merge or budget semantics. The budget examples remain
specification arithmetic: 7,797 one-byte turns cost 3,999,861 accounting bytes,
and the next costs 4,000,374 in total, not measured allocations.

Implementation landing (second visit for #277): the controller authorized the
D0 test-only amendment, widened the fence to the three named CLI/TUI test
files and verified that `cargo` resolves in the warmed box. The packed merge,
the shared `512`-byte structural charge, the bounded two-pass DSH projection
and the blockless guard are implemented in `crates/brokkr-view/src/transcript.rs`
and proved by the view, command, surfaces and TUI suites; the Codex
association-sharing and header true-EOF repairs are preserved with their
during-pass and reason-bearing assertions. Each bound was removed, its
targeted test observed to fail for the intended assertion, and restored to
pass. Design D8's `/usr/bin/time -v` process proxy measured the unchanged
production base `5ef4a842` and this candidate on identical generated bytes;
the numbers are recorded in the delivery notes. In-box formatting, clippy and
the workspace suites pass; in-box exact-coverage totals are reported with the
known #286 namespace skips. Living-spec synchronization, host exact coverage
and final-head remote CI remain controller/closure work.
