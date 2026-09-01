# 0028 — Keep-refs: the journal's exhibits outlive the branch

Status: proposed
Date: 2026-09-01

## Context

A journal cites git SHAs. The protected phase records the head it
reviewed (`transition/decided.inputs.reviewed_heads`); ship records each
realm's head (`.inputs.realm_facts.<realm>.head`). Those citations are
the run's exhibits: the objects a reader goes to when they ask what,
exactly, was reviewed and what, exactly, shipped.

The chain that proves the journal is hash-based, not git-based. So the
ordinary landing flow — squash-merge the PR, delete the branch, let `git
gc` run — takes the cited commits with it and the journal never notices.
It still verifies. It still names commit `a1b2c3…`. The repository no
longer has it. The evidence is a citation to a shelf that was emptied.

This is not hypothetical: branch cleanup is routine, and it is the
operator's own routine, done long after the run concluded.

## Decision

1. **The shape.** For each distinct object a run's journal cites, plant
   `refs/forge/keep/<run-id>/<sha>` pointing directly at that object. No
   synthetic commit, no chain — unlike the anchor (`anchor.rs`), which
   chains because it records a sequence, a keep-ref's whole job is to be
   a root. Naming the ref after the object it holds is what buys the
   three properties the mechanism needs:

   - **idempotent** — replanting writes the same name to the same value,
     which `git update-ref` treats as a no-op;
   - **cheap to list** — one `for-each-ref` over the namespace answers
     which runs hold which exhibits, never one `rev-parse` per ref;
   - **deliberately deletable** — `refs/forge/keep/<run-id>/*` is exactly
     one run's exhibits, so releasing them is one command and one run.

2. **Planted automatically, at run conclusion.** Alongside the existing
   anchor call in `Engine::drive`, on every conclusion the anchor covers
   (completed, stopped, awaiting_operator), best-effort: a gap is
   printed, never fatal, and never fails a run.

   The second determinism law — *state is derived, never mutated* — is
   satisfied, not strained: the set of refs is a pure function of the
   journal (`brokkr_core::keep_refs::cited_shas`), computed by fold, and
   planting is the same category of derived side effect the anchor
   already is. Nothing is written back into the journal; replanting a
   journal that has not moved produces exactly the refs already there.

   The operator-owns-push/merge division is likewise untouched. A
   keep-ref is a local ref in the engine's own `refs/forge/` namespace.
   It is not a branch, it is not pushed, it changes no index, no working
   tree and no checked-out branch. It cannot merge, cannot land, cannot
   travel to a remote.

   Automatic is the *stronger* reading of the division, not a weaker one.
   The operator owns cleanup; a mechanism that requires the operator to
   remember a verb *before* cleaning up hands them a trap, because the
   moment of forgetting is silent, unrecoverable and only discovered by
   the reader who later needs the exhibit. Planting at conclusion means
   cleanup stays entirely the operator's — and stays safe.

3. **A manual verb as well, not instead.** `brokkr keep-refs plant --run
   <selector>` calls the identical function the engine calls. It covers
   the runs that concluded before this mechanism existed, and the
   deliberate restoration of exhibits released earlier. `brokkr keep-refs
   list [--run <selector>]` reports the namespace; the ref's *target* is
   reported rather than the SHA in its name, because the name is a claim
   and the target is the fact.

4. **Deletion is the operator's alone.** `brokkr keep-refs delete --run
   <selector>` removes that run's keep-refs and nothing else: the
   operator saying these exhibits may go. Nothing in the engine ever
   deletes a keep-ref, and no keep-ref expires. This mirrors the anchor's
   posture — refs record, they do not police.

5. **Collected by fold, never by grep.** The cited set is folded from the
   journal's `transition/decided` events through the event vocabulary's
   own types: every head under `reviewed_heads` (per-realm, and the
   legacy unkeyed `repo` shape the contract still accepts) and every
   `realm_facts.<realm>.head`. Deliberately not the fold's
   `RunState.reviewed_heads`, which keeps only the latest value — state
   is not history, and a reforged run (0022) revisiting review cites a
   different head each time. Every one of those was named by the journal,
   so every one of them is an exhibit. Strings that are not object names
   (40 or 64 hex) are not citations and never reach git.

## Consequences

- A run's exhibits survive branch deletion and `git gc` unless the
  operator releases them.
- The `refs/forge/keep/` namespace grows by one ref per distinct cited
  object per run — two or three refs for a typical run. Trivial, and
  bounded by the journal.
- Cited objects that this repository does not hold — another realm's
  head, or an object already collected before anything was planted — are
  reported as a gap rather than silently dropped. Silence there is the
  failure mode this decision exists to end.
- A cited commit stays reachable, so "delete the branch and let gc run"
  stops being a way to make one disappear. If a reviewed head has to go
  — a secret committed and rewritten away, say — releasing the run's
  keep-refs (`brokkr keep-refs delete --run <id>`) is now part of that
  removal, alongside every other reference the repository holds. The set
  is bounded and listable, which is the point: an exhibit the operator
  cannot see is one they cannot release.
- Keep-refs outlive journals, so the reading and releasing verbs take a
  literal run id when the workspace database is gone — the refs
  themselves name the run. `latest` is the exception and is refused
  there: it is a question for the run table, not a name, and answering
  it literally would report a listing (or a release) of nothing as
  though it were the answer.
- Pushing keep-refs to a remote is deliberately out of scope: a remote's
  own gc policy is a separate, undesigned question.
- Signing keep-refs stays deferred with the signing service (0008), as
  it is for anchors.
