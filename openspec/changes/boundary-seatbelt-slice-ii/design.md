## Context

The operator accepted R1–R4 observables on 2026-09-09 and requires demonstrated
guarantees. Section A of #253 now has verified native baseline evidence on
macOS 26.6.2 arm64, Rust 1.98.1, source bb0bc39520f064ff7abe62ec517f52922bab6b53.
The archive checksum and five committed logs match. That baseline proves
launcher readiness and unavailable-boundary refusal, not native containment.

## Goals / Non-Goals

First settle R3 feasibility with a bounded, reproducible Rust experiment.
Preserve the unbuilt production fence. This preparation does not implement
Seatbelt, claim a viable complete mechanism, or enable an alternate grade.

## Decisions

1. **Accepted semantics govern the deltas.** Replace stale same-path overlay,
   readable-empty mask and unruled hooks scenarios with the accepted 0046
   snapshot-locator, denied-read and conditional private-hooks observables.
   Their protection requirements and native proof obligations remain intact.
2. **Falsify the process-group candidate first.** The fixture independently
   varies ordinary/setsid/double-fork topology and timeout/cancel/parent-exit/
   supervisor-death. Apple's documented group semantics motivate this test;
   no source-level inference is native proof. launchd's documented group
   cleanup is not assumed to catch a different session. No undocumented
   SBPL operation is assumed to prevent detachment.
3. **Observe outside the candidate.** Two known live helper identities and a
   moving heartbeat precede each trigger. Check process state and heartbeat
   after the teardown bound; stopped processes remain live. A no-cleanup
   negative control must be detected. Observer cleanup occurs only after
   the verdict and cannot erase a residual. Retain the group leader's wait
   status until group signalling, avoiding reused group identifiers.
4. **Bound the fixture separately.** Controlled helpers watch an observer-only
   cleanup marker and self-expire after 30 seconds. Expiry before measurement
   invalidates it. Cleanup cooperation is not offered as a production lifetime
   guarantee. No arbitrary user command, host process scan/kill, service
   installation, global configuration change or real secret belongs here.
5. **Keep evidence attributable.** The native runner requires a clean checkout,
   records revision/host/toolchain/source and binary digests, preserves logs
   on failure, and returns nonzero with R3 OPEN. Portable Linux self-tests
   validate the harness and label native=false. This standalone experiment
   is outside production Cargo artifacts and does not alter exact coverage.
6. **Do not design full enforcement around an unproven lifetime mechanism.**
   The explicit overlay locator transport, alias-safe snapshots, native mask/
   hook policy and hands integration remain dependent design tasks. A
   filesystem snapshot alone cannot resolve R3. Raw process polling has
   tracking/race problems and is not credited as arbitrary-child containment.
   If this candidate fails, select and measure a stronger native mechanism;
   absent one, return the residual without weakening the accepted guarantee.

## Risks / Trade-offs

The initial profile allows default operations and measures lifetime only;
it supplies no R1/R2/R4 or network evidence. Controlled helpers are not an
arbitrary-code adversary. Retained output pipes, hostile fork races and both
hands integration remain necessary for a replacement mechanism. Full peer
status cannot follow from this experiment alone, including an unexpected
all-clean observation on a future macOS version.

## Validation and handoff

Run standalone Rust unit tests and the full portable matrix on Linux, plus
shell syntax/refusal checks and OpenSpec validation. Run workspace gates as
required by the house; any existing unrelated failures are reported, not
silently attributed to new code or bypassed. Publish a reachable candidate
SHA through the controller's delivery workflow, then the operator runs:

`bash scripts/validate-seatbelt-macos.sh --lifetime`

Attach its archive to #253. Reconcile each measurement with SEATBELT-R3;
R1/R2/R4 remain OPEN. No production activation or full-slice completion is
part of this preparation.
