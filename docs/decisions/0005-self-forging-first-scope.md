# 0005 — Self-forging first: the initial implementation scope

**Status**: accepted (operator ruling, 2026-08-23; scope enumeration drafted
by Claude under that ruling and amendable by the operator)

## Ruling

The first implementation target is the smallest engine that can drive its
own further development in this repository (delivery-sequence step 10,
pulled forward): the full polyrepo-shaped machinery is not built until the
loop is proven here. The operator's ruling: the polyrepo-shaped phases —
live stack verification above all — do not make sense for a one-repo engine
build, and nothing that does not make sense here gets built now.

In scope for the first cut:

- `forge-core`: event envelope, fold, and the strict evaluator (decision
  0004), differential-tested against the Python oracle's committed corpus.
- `forge-store`: bundled-SQLite journal — append-only, hash-chained,
  update/delete rejected by triggers — plus NDJSON export and offline
  verification.
- `forge-runtime`: the durable effect loop (request → commit → execute →
  terminal attempt fact → validate → evaluate → decide), crash recovery,
  indeterminate parking, and a single-writer file lease.
- `forge-protocol`: `forge-driver/v1` over NDJSON stdio, a fake driver for
  machine proof, and one real driver: headless Claude Code.
- `forge-cli`: `init · compile · run · resume · inspect · replay · export ·
  verify-run`. No UI.
- `bundles/self/`: the self-delivery bundle — a trimmed linear table
  (intake → implement → verify → review → ship, with the constitutional
  review/security invariants intact), one seat per phase, result schemas,
  and role charters. Verification is `cargo test` + the differential suite,
  not a stack boot.

Deferred, all additive behind the frozen v1 contracts: the embedded UI
(acceptance criteria already require the engine to be UI-independent),
container and remote runners, the signing/anchoring service, multi-seat
topologies (`parallel`/`join`/`loop`/`submachine`), Codex/Cordis drivers,
LaneTally integration, and the origin-workspace profile.

## Why

- Every trimmed item extends data (bundles) or adds a driver behind a
  versioned protocol; none changes the reducer, the journal, or the policy
  semantics. Deferring them costs nothing structural.
- A one-repo, one-seat delivery exercises every constitutional invariant
  that matters — pure decisions, parked unknowns, operator gates, replay —
  at a fraction of the machinery.
- Self-hosting is the fastest honest feedback: the engine's first user is
  its own development, with the human retaining review and merge authority
  exactly as delivery-sequence step 10 prescribes.

## Consequences

- The repository becomes a mixed workspace: Rust crates under `crates/`,
  the Python oracle retained as executable policy specification.
- The self bundle's phase table is bundle data, not a fork of the imported
  production table; `policy/phase-machine.json` remains untouched upstream
  parity material.
- The origin-workspace vertical slice (delivery-sequence step 6 as originally
  ordered) follows once the self-loop has delivered real changes here.
