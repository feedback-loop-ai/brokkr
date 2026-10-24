# 0096 — Driver evidence belongs to the current effect and attempt

Status: proposed
- **Status:** proposed
Date: 2026-10-24
Built: not built

## Context

The driver v1 schema already requires effect_id and attempt_id in Accepted, Checkpoint and Result. The effect survives retries, whereas each execution has its own attempt. The shared receive loop currently compares effect_id alone. A foreign attempt can therefore supply a session, a checkpoint stamped as current by the runtime, or a clean review receipt authorizing ship. The supplied reproduction and static code inspection justify engine/security classification; this proposal does not claim executed implementation proof.

This repairs enforcement of the existing identity boundary under [0005](0005-self-forging-first-scope.md) and the strict evaluation of [0004](0004-strict-condition-evaluation.md). It touches fail-closed admission semantics, so operator acceptance is required for this proposed ruling; no frozen contract is edited or versioned. [The specification](../../specs/attempt-correlation/spec.md) defines testable criteria and [the plan](../../specs/attempt-correlation/plan.md) resolves the two design positions.

## Proposed rulings

1. Each Accepted, Checkpoint and Result is admitted only when both IDs exactly match the immutable pair sent in Start for that invocation. The shared protocol transport checks effect first, then attempt, before acceptance/session mutation, checkpoint retention/callback or result conversion. A resumed session conveys no authority for its earlier attempt. No duplicate rule lives in engine, panel or view (0071 rulings 1–3, 5 and 7).
2. A mismatch after Start uses the existing Unread -> Indeterminate path, not determinate failure, ignored traffic, a successful transition or automatic retry. Preserve existing reader-limit, shutdown and cleanup-uncertainty precedence. A prior valid acceptance and journaled legitimate checkpoints remain facts; returned broken session/checkpoint fields are cleared as today. The foreign message contributes no admitted evidence. A legitimate callback remains subject to the existing checkpoint sink retries and settlement; callback delivery alone is not a durable commit, and stranded checkpoints still prevent success.
3. Mismatch diagnostics describe foreign bytes only by the existing length, first-byte class and SHA-256 prefix. They never quote foreign identifiers or payloads. The engine-owned expected identifier may be quoted as today. Existing foreign-effect reasons remain byte-identical and win if both identifiers differ. Attempt reasons are exactly `driver accepted a different attempt`, `checkpoint for foreign attempt`, and `result for foreign attempt`, followed by the established mismatch rendering and post-Start suffix. Reuse typed outcomes and the single evidence implementation (0071 rulings 3, 5 and 8).
4. Proof must independently bind each of the three call sites, preserve matching traffic and foreign-effect behavior, retain prior legitimate checkpoint callbacks, reject foreign failed Results as indeterminate, and show that a foreign clean review Result cannot advance review to ship immediately or on journal replay. Paired matching controls establish fixture reachability. Every new test carries an executed compiling removal control; all existing house gates and exact coverage remain unchanged (0071 rulings 4 and 9).

## Alternatives and costs

A Result-only fix leaves acceptance/session and checkpoint evidence corruptible. An engine-only fix runs after shared consumers or live checkpoint callbacks have already admitted foreign data. Discard-and-continue conceals mixed executions; determinate failure can retry uncertain effects. All are rejected as weaker refusals.

A global attempt registry, identity framework, new wire version, authentication mechanism or ordering rewrite adds unrelated state and proof obligations. One private comparison rule using existing typed fields is sufficient (0071 rulings 2, 5, 6 and 10). The accepted limit is that a producer which copies both current IDs is not authenticated by equality checks.

The stronger proof suite wins over the smallest possible test matrix: prior legitimate evidence, failed foreign Results and reopened-journal behavior cost extra bounded test executions but expose distinct misattribution and authorization failures. No extra production mechanism follows from those tests.

## Compatibility, migration and status

Conforming drivers need no migration. Drivers sending stale attempts now park; correcting those drivers is the compatibility cost. No journal rewrite, retroactive certification, automatic replay or exactly-once external execution guarantee is introduced. Frozen contracts, fixtures, reference and production policy remain unchanged. Linux and macOS are the supported hosts; production remains Rust.

This proposal grants no new capability, accepts no other decision and reports no implementation validation. The operator alone accepts it. The user's explicit downstream reviewer pins apply; decision 0058's proposed top-level status remains factual. Outside-box coverage and reviewer results remain pending until obtained. No gate, denominator or refusal may be weakened to finish this repair.

## Design return disposition

The preceding design checks rejected the declared proposal status because the pinned design checker requires a list-prefixed `- **Status:** proposed` line. Brokkr's decision index reads the native `Status: proposed` line above; the two lines state the same proposed status and do not accept this decision. Preserve both forms and decision 0096. Commit `519a3ec3` corrected that mismatch. Continuation `triage-and-continue-the-epic-192-3ee4b8fb`, after `continue-the-brokkr-attempt-corr-6ca9c613` stopped on origin DNS, preserves that correction, this proposal's scope and all four proposed rulings; it does not repeat the repair or allocate a replacement number. The retained predecessor check records a design pass at `2ddf906b` after the correction; the earlier failed records are historical and do not imply decision acceptance. The final design commit must contain the linked proposal and all three specification artifacts, clean at HEAD; the sequence check owns acceptance of that artifact shape.

Current synthesis for `triage-and-continue-the-epic-192-c8405236` adopts both positions' invocation-local transport rule and mandatory per-arm, privacy and durable replay proof. Their scope agrees; no artificial panel conflict requires a new ruling. Current-run evidence is isolated under `.forge/artifacts/triage-and-continue-the-epic-192-c8405236/`; prior design validation cannot certify the implementation. At candidate `9f5c53b7`, FETCH_HEAD and origin/main both match the supplied successful-fetch receipt `54d5a14e8401bb0feb3fce02b9183396a8adfe9f`, also the merge-base. The predecessor DNS stop is historical and its continuation prerequisite is discharged by that receipt check. Recheck before implementation and reconcile drift before editing, as the specification scenarios require. The latest retained predecessor design check passed at `9f5c53b7`; this is artifact evidence only. This continuation changes neither the four proposed rulings nor their acceptance status.
