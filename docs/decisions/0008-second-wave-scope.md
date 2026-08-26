# 0008 — Second wave: the 0005 deferrals, delivered and bounded

**Status**: accepted (operator goal directive, 2026-08-23 — "deliver what
was deliberately left out, slice by slice, verified by the forge's
verify agents; all drivers delivered")

## Delivered

Every slice landed through PR + CI and was examined by the forge's own
verification bundle (`bundles/verify`: a verify seat plus a strictly
read-only review seat with security riding along — its rulings are
journaled runs like any other):

1. **`bundles/verify`** — the verification agents themselves; first
   verified by their own first use.
2. **The driver fleet** — `driver_common.py` (one protocol lib),
   `claude_driver.py`, `codex_driver.py`, and `exec_driver.py` for any
   template-shaped harness: dsh/Surface profiles, ssh-carried remote
   execution (the protocol is pure stdio), or any prompt-in /
   result-file-out CLI. One parameterized conformance suite over all
   adapters; `forge doctor` detects claude, codex, and dsh; `forge init`
   embeds the fleet.
3. **Parallel panels** — `SeatBody::Panel`: members fan out INSIDE one
   effect and join as a barrier in declared order (decision 0002 kept:
   concurrency inside, serialization at the boundary). Aggregates are a
   closed vocabulary: `unanimous-pass`, `review-panel`
   (worst-member-wins; severity max; security/fixes OR-ed). Member
   outcomes are journaled checkpoint evidence; an indeterminate member
   parks; a failed member is 0006-retryable.
4. **The embedded read-only UI** — `forge ui`: loopback, std-only HTTP
   shell, one embedded page, SSE updates; submits no commands, writes
   nothing (constitutional boundary 6). The journal gained full causal
   threading (`causation_id` on every engine append) to feed its
   timeline.
5. **Journal anchoring** — `refs/forge/<run>` commit chains recording
   seq + head hash; auto-anchored at drive end; `forge anchor --check`
   detects a moved or rewritten journal. Tamper-EVIDENCE, not
   tamper-proofing: the ref is unsigned.
6. **Container confinement** — `driver.confine {image, network, mounts}`
   maps the policy-confined trust class onto `docker run` wrapping for
   any seat or panel member; proven live in machine proof.
7. **The LaneTally join surface** — `forge costs`: per-seat attempts,
   turns, and USD from journal checkpoints, keyed by stable seat ids.

## Still deferred, by name

- **The signing service**: anchors are unsigned; key distribution,
  the signing-key allowlist, and external anchoring (the referee-era
  lore in `reference/handoff-protocol.md`) wait for an operator key
  decision.
- **`loop` / `gate` / `submachine` topology primitives**: panels cover
  parallel+join; the rest of the inner-topology language follows need,
  not symmetry.
- **A remote-runner transport beyond ssh**: the target architecture
  itself defers this choice; the exec driver's ssh pattern covers the
  known case.
- **Deep LaneTally integration** (default-deny per-seat credentials
  fetched at spawn): lands on the driver-command boundary as bundle
  data when LaneTally's key API is pinned.
- **The origin-workspace profile and vertical slice**: lives in its own
  workspace; it is the next campaign, not a gap in this one.
- **Looper as the dispatch layer above the forge**: the composition is
  designed (Looper decides what and when a human is needed; the forge
  alone decides how far; LaneTally decides who pays) and the forge-side
  surfaces exist (typed exits, parks, SSE, `forge runs`, `forge
  costs`); the Looper-side wiring is its own delivery.
