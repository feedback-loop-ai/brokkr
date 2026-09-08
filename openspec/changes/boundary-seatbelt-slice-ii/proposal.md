# Change: Seatbelt on macOS — decision 0046 slice (ii)

## Why

Slice I (PR #230, `7b53e92`) names and pins `seatbelt`, but this engine
still refuses it even when `sandbox-exec` exists. macOS needs a real
execution boundary for workspace calls and boxed exec seats, with the
security guarantees of decision 0043 and honest evidence of what was
measured, as accepted decision 0046 requires.

## What Changes

- Implement the `seatbelt` path for both the workspace MCP tool and whole
  boxed exec dispatches. Preserve the hands policy: cleared environment,
  private call HOME/tmp, controlled toolchain access, network denied unless
  declared, writable worktree, `ro`/`rw`/`overlay` binds, credential masks,
  protected git metadata, bounded output and process lifetime.
- Prepare a concrete council design and adversarial macOS execution,
  including linked worktrees, for the operator to settle 0046's deliberately
  unruled git-hooks question and its consequence for Seatbelt peer status.
  Protect the host after the seat exits as well as during a call.
  Environment overrides or a successful unsigned commit alone do not prove
  that a hostile command cannot plant a host hook.
- Make overlay persistence and masks explicit compatibility obligations.
  A writable host cache, a read-only substitute for an overlay, or a private
  copy reachable only through a special locator does not silently satisfy
  the declared bind. Document any proposed difference in paths, visibility,
  or lifetime before asking the operator to rule on it.
- Thread the compiled realm boundary through runtime composition, both
  hands CLI verbs and adapter MCP configurations, entry refusals and doctor.
  Test the existing manifest, effect, seat-record and readout contracts
  against real Seatbelt dispatches. Handle the engine binary and declaring
  bundle layers without assuming Linux's `/runtime/bundle` mount exists.
- Add required behavioral tests on the existing macOS CI runner, alongside
  host-independent policy/composition tests and Linux regressions. Update
  the guides with the actual mechanism, restrictions, invocation examples
  and the evidence status; distinguish implemented code from measured
  acceptance of the whole slice.

This run owns slice II only. `container` stays unbuilt and refuses as
slice III, including when Docker or Podman exists; `driver.confine` stays
retired. The repository's default boundary, accepted decision statuses,
frozen contracts, production policy, reference material and evaluator
corpus are not rewritten. Any required wire change is an additive version.
No provider roster or global agent configuration change belongs here; the
controller already selected the Codex-led control library. Delivery and
all remote actions remain the controller's; no nested Brokkr run is needed.

## Capabilities

### New Capabilities

- `seatbelt-execution`: Enforce the declared hands policy through real
  `sandbox-exec` execution, including bind and git adversaries, child
  lifecycle, pinned inputs, and the evidence required to accept slice II.

### Modified Capabilities

- `boundary-availability`: Activate Seatbelt only after the policy and peer
  prerequisites below are settled and both paths implement them; define the
  system-launcher probe, host/tool refusals and macOS init advice; retain
  slice III. Refusing the shipped overlay users is not this deliverable.
- `realm-boundary`: Keep compilation machine-independent and pin the
  realm's Seatbelt word while making the start refusal about availability.
- `gate-boundary-policy`: Route boxed model, exec and dialect sites through
  Seatbelt; retain the gate law and the documented unboxed script limits.
- `boundary-guides`: Describe the actual macOS boundary and its evidence,
  preserving platform classes and the distinctions between boundaries.

## Impact

The implementation reaches `crates/brokkr-protocol/src/hands.rs` and its
process/driver integration, `crates/brokkr-runtime/src/engine.rs`, agent
resolution and composition, `crates/brokkr-cli` hands commands and boundary
probes, their tests, `.github/workflows/ci.yml`, and the hands, quickstart,
provider, driver, journal and read-surface guides. The existing
`boundary-manifest-pin`, `boundary-record` and `boundary-readouts`
requirements remain authoritative; new scenarios exercise them end to end.
Witness and compose pins move only for measured identity changes, with the
reason recorded. No engine version bump is commissioned.

## Decisions

This visit **adopts** change `boundary-seatbelt-slice-ii` from `c01132d`,
answers the seven findings in `returned_from` (clarify, `ambiguous`), and
amends proposal before deltas. Only these specify artifacts are authored;
design and tasks do not yet exist. The dialect's instructions and artifact
state were read directly with `openspec instructions` and `openspec status`;
no workflow runner was invoked.

Accepted authority remains 0046 and its addenda, 0043, 0049, 0004, 0005 and
0009. Decision 0048 remains proposed; its enacted checks and limitations
stand. This return does not enact a weaker policy or accept a decision.

### Returned findings and their disposition

| Finding | Disposition and reason | Owning delta/scenario |
|---|---|---|
| R1 — overlay and shipped-library deliverable | Accept the conflict; retain full declared policy as the deliverable. Refusal is a safety outcome, not an alternative definition of built. Overlay realization needs an upstream answer. | `boundary-availability`: Shipped overlay users cannot be refused into a built claim; `seatbelt-execution`: Locator-only copies do not pass as arbitrary overlays |
| R2 — readable-empty masks | Accept the circular assertion; the present-mask target is now an explicit successful zero-byte read at the declared path. Denial is a different observable and needs an upstream ruling. | `seatbelt-execution`: Present masks have a readable-empty view |
| R3 — detached descendants | Accept that process-group termination alone does not meet the requirement. Remove the per-command unsupported-policy escape hatch; no declaration distinguishes benign commands from commands that detach. Require a demonstrated mechanism or an upstream change of guarantee. | `seatbelt-execution`: Session detachment is part of the ordinary command threat model |
| R4 — hooks view and peer status | Accept; a private hooksPath is a candidate for ordinary Git, not an accepted peer ruling or a raw-write defense. Keep activation fenced until the operator resolves the observable difference and grade. No harness-grade execution branch is commissioned by this draft. | `seatbelt-execution`: An unresolved hooks ruling cannot produce a peer gate; `gate-boundary-policy`: Compile admission is not a peer ruling |
| R5 — system launcher | Resolve: only the literal `/usr/bin` entry of the supplied PATH admits `/usr/bin/sandbox-exec`, with executable/ownership checks and a bounded real allow/deny probe. Lookalikes are never executed. Namespace lookup is unchanged. | `boundary-availability`: The system pin ignores an earlier lookalike; A no-op launcher fails the denial probe |
| R6 — exact coverage seam | Resolve the design constraint: shared Rust policy, composition and lifecycle decisions accept injected host/probe/process facts and are compiled and exercised on Linux. No target-gated Seatbelt implementation may hide these branches from the exact gate. Native enforcement remains separate evidence. | `seatbelt-execution`: Linux exercises both policy arms without claiming native enforcement |
| R7 — init warning | Resolve by adding a macOS arm: explain the default namespace, name Seatbelt with its actual activation/tool status and doctor check, and retain the explicit unboxed harness alternative. Do not change the realm automatically. | `boundary-availability`: init on macOS describes the available Seatbelt road; init does not recommend an unbuilt boundary as usable |

R1's universal inventory is corrected by the files: `bundles/self`'s ship
seat has no binds; `recipes/node` overlays `~/.npm`, not `~/.cargo`;
`agents/release-manager.json` also declares the masked cargo overlay. This
does not refute the consequential finding: the self and verify **verify**
seats and the named review agents require masked overlays, so denying those
policies stops those bundles at start. Changing their declarations would
change policy and identity, not repair a launcher. The existing declaration
inventory must remain the acceptance input unless the operator commissions
an explicit amendment.

### Proposed decision for the upstream return: retain policy until its realization is ruled

**Status: proposed.** This focused return proposal is recorded at the
dialect's decisions place; it asserts no new accepted security semantics.
Its recommendation is to retain the current guarantees and keep Seatbelt
unbuilt until an enforceable realization is identified. If the operator
instead chooses any semantic difference below, record that exact choice
in a focused `docs/decisions/` document with status `proposed`, then return
to this same change to encode the ruled observables before dependent design
or implementation enables them. Only the operator can accept that document.

The four precise questions carried upstream are independent:

1. **R1, 0043 ruling 2 and 0046 ruling 6(ii):** Must an arbitrary overlay
   remain usable at its declared absolute path with lower-layer semantics,
   or may Seatbelt expose a relocated seat-private snapshot only through
   an explicit locator, with direct source-path writes refused and later
   host-source changes invisible? The latter needs a locator contract for
   arbitrary binds, link/mask behavior and identity consequences, not just
   `CARGO_HOME`. If neither realization is commissioned, does the operator
   explicitly reduce slice II to refusing overlay users or amend named
   shipped declarations, and does that reduced scope qualify as built?
   This proposal's answer remains **no reduction without that ruling**.
2. **R2, 0043 ruling 2:** May a present masked file return a permission
   error on open/read on Seatbelt, instead of a successful empty read at
   that same path? The current answer is **readable-empty or refuse**;
   denied-read support alone does not complete mask support. Either choice
   must keep credential contents and host bytes protected through aliases.
3. **R3, 0043 ruling 1 and this commission's time limits:** Supply a
   named native lifetime mechanism that prevents or terminates descendants
   after `setsid`, double-fork and supervisor death, or explicitly rule
   whether a residual detached descendant is permitted and what that means
   for deadlines, cancellation, scratch lifetime and boundary grade. This
   proposal retains **no surviving payload**. It does not assert that all
   possible macOS mechanisms are impossible; none is identified or measured
   in this change, and a process-group kill is insufficient evidence.
4. **R4, 0043 ruling 6 and 0046's deliberately unruled consequence:** Does
   denying access to host hooks plus directing ordinary Git to an empty
   private `core.hooksPath`, with independent protection against all raw
   hook/config/routing writes, satisfy the empty-hooks obligation and admit
   Seatbelt as a full peer after the primary/linked-worktree adversaries
   pass? The original hooks directory would return denial, not an empty
   listing. If the operator instead rules harness-grade, the upstream
   amendment must settle gate admission, `hands: boxed`, readouts and
   delivery summaries together; the current peer-only deltas cannot enable
   that branch. A Git environment override alone is refused on either road.

**Evidence and limit.** `hands.rs` builds same-path `--overlay` entries,
`/dev/null` masks and a `--tmpfs` hooks view, and its timeout kills the
namespace launcher. `hands/tests.rs`'s linked-worktree test checks the empty
hooks view. There is no Seatbelt implementation in that file, and
`boundary.rs` plus the runtime's built-boundary fence still refuse it.
[Apple's setsid manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/setsid.2.html)
describes creation of a new process group;
[its kill manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/kill.2.html)
defines group-directed signals by current group membership. The inference
is that a signal to the original group is not a complete descendant kill;
this is source reasoning, not a native Seatbelt experiment.

A refusal-only implementation is therefore **not** this slice's successful
outcome, even if a bind-free smoke test runs. Positive execution, gate,
record and guide scenarios in these deltas describe the target after
`boundary-availability`'s activation conditions hold. Until they do, the
0046 unbuilt refusal remains the behavior, with no successful Seatbelt
seat to imply peer status. An upstream harness-grade ruling would require
new deltas for `boundary-record` and `boundary-readouts` as well as changes
to gate policy and guides before activation; frozen files would still need
additive versions if their meaning or shape changes.

### Validation and evidence ownership

R5's probe establishes launch readiness only; it cannot settle R1–R4. R6's
injected facts prove the shared program decisions, not macOS enforcement.
The controller owns actual Mac measurements and host exact coverage on the
candidate. Both remain pending, and neither pending result licenses a
semantic downgrade. The existing macOS CI runner must execute the native
suite without missing-tool or zero-test success. The exact gate retains
literal nonzero 100% line/branch/function equality and
`rust-nightly-version.txt`, consumed by CI, release coverage and the local
script.

This returned specification is **upstream**, not ready for implementation
under an inferred ruling. The controller can resume the same change with
recorded answers to R1–R4 or a demonstrated mechanism satisfying their
unchanged guarantees. R5–R7 and the other policy/test obligations are
prepared for that return. Rust validation is unavailable in this author
seat (`cargo`, `rustc` and `rustup` are absent); OpenSpec and artifact checks
are recorded after the revision below. No Mac measurement, host coverage
result, implemented Seatbelt path or completed slice is claimed.

Returned-visit checks: `openspec validate boundary-seatbelt-slice-ii
--strict --no-interactive` and `git diff --check` pass. Artifact checks find
five matching capability deltas, 23 added/modified requirements and 138
scenarios, all with WHEN/THEN; modified/removed requirement names match the
base specs, and all seven findings have a disposition. OpenSpec reports
proposal/specs present and design/tasks unauthored; that is artifact state,
not acceptance of R1–R4. Exactly the six specify artifacts are changed, and
the frozen trees have no diff from the adopted `c01132d` commit.

Attempts to run `cargo fmt --all -- --check`, locked all-targets/all-features
clippy with `-D warnings`, `cargo test --workspace`, the all-features locked
tests, and locked compiles of self and verify all report **cargo unavailable**
in the workspace tool. None passed or ran tests. The controller must supply
a toolchain-equipped environment for those checks, actual sandbox-exec
execution on the Mac CI runner, and `bash scripts/coverage-exact.sh` outside
the workspace box. Native tests and host exact coverage remain pending;
this specify-only commit contains no implementation or measured digest
change requiring a repin.
