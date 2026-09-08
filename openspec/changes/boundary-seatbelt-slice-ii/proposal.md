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
- Resolve 0046's deliberately unruled git-hooks question through a concrete
  council design and adversarial macOS execution, including linked
  worktrees. Protect the host after the seat exits as well as during a call.
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

- `boundary-availability`: Offer implemented Seatbelt only on a capable
  macOS host; refuse wrong hosts, missing/unusable tools and unsupported
  policies consistently at the CLI and runtime entries; retain slice III.
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

- **Authority and scope.** Adopt accepted 0046, including its unbuilt-boundary
  addendum and seat-record v4 erratum, with 0043 and 0049. Decision 0048
  remains proposed; its recorded limitations and enacted integrity checks
  are preserved. Decisions 0004, 0005 and 0009 govern this Rust engine change.
  This specify visit authors the proposal first and capability deltas next;
  council design and tasks are subsequent artifacts, not preempted here.
- **The unresolved ruling is not delegated to a fallback.** Council design
  must name the actual git protection mechanism and alternatives, then bind
  its claims to adversarial measurements. The precise question is whether
  it satisfies 0043 ruling 6's hidden hooks and immutable config while
  permitting unsigned linked-worktree commits, including effective
  `core.hooksPath`, include files and filesystem aliases. A deny-only hooks
  view, if proposed instead of the empty directory view, must be named as
  an observable difference for an explicit ruling. A Git flag alone is
  refused as a security answer because commands can bypass it.
- **Overlay and mask compatibility is a separate question.** If the design
  needs relocated snapshots or denied reads instead of same-path overlays
  and `/dev/null` masks, it must state precisely which declared operations
  change and why. Locator-only support for cargo does not establish support
  for arbitrary binds. Semantic changes must carry a focused decision
  document with status `proposed`; only the operator accepts a new ruling.
  Until then the original guarantees govern and an unsatisfied policy
  refuses. If an accepted requirement itself must change, the finding
  belongs upstream; it cannot be hidden in implementation or a guide.
- **Preparation is not macOS evidence.** This controller and author seat
  are Linux. Actual Mac measurements and host exact coverage are pending,
  owned by the controller until real run results identify the candidate.
  The author seat additionally has no `cargo`, `rustc` or `rustup` on PATH;
  specification validation is available through OpenSpec 1.12.0, while
  Rust checks require a toolchain-equipped seat or controller. Implemented
  code may be prepared and committed here without declaring the overall
  slice complete. Neither Linux tests nor generated profile strings can
  settle the Mac security questions.

Validation of this specify visit: `openspec validate
boundary-seatbelt-slice-ii --strict --no-interactive` passes, and OpenSpec
reports proposal and specs done with design still outstanding. Attempts to
run format, clippy, all-features locked workspace tests and both bundle
compiles report cargo unavailable in the workspace tool environment; none
is claimed to have passed. No Mac or host exact-coverage result exists for
this preparation.
