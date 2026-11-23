# 0122 — What is judged and shipped is the candidate verification passed

Status: proposed
Date: 2026-11-23

Built: built

## Context

Decision [0109](0109-a-gate-s-re-visit-reads-what-changed.md) ruling 7
made a gate's head engine-stamped: a gate span reads HEAD as it begins
and as it ends, a moved head parks (`GATE-MOVED-HEAD`, decision
[0041](0041-one-office-per-seat.md) ruling 4), and the protected phase
decides on the `checked_head` its own effect journaled, which becomes
`reviewed_heads`. The ship gate compares HEAD with that head
(`drift_detected`) and reads the worktree (`dirty_worktrees`). So what
ships is what the judge read.

Nothing binds what the judge read to what verification passed:

- A commit between verify's ruling and the judge's span, or a verify that
  passed over uncommitted bytes, reaches review and ship as verified
  code. Verify's own `covered` record (0109 ruling 7) names the head its
  span checked, but says `dirty: true` on every record until 0109b's tree
  check exists, and nothing compares it with the judge's head.
- The ship gate's `dirty_worktrees` reads `git status --porcelain` as the
  repository configures it, so a repository that sets
  `status.showUntrackedFiles=no` hides an untracked source file from it.

The adopter `feedback-loop-ai/swarmail` has carried a patch for this on
top of brokkr 0.12.0 since 2026-10
(`tooling-patches/0003-bind-verified-candidate.patch`, release
`swarmail-engine-a9893b4-p4`), and asks for it upstream so it can pin a
stock release (feedback-loop-ai/swarmail#405). Pull request #899 first
proposed it under decision 0097, and its upstreaming found four
compatibility faults in the patch as written, which this record keeps
the answers to: a recipe whose protected phase is `verify` itself
(`research`) parked every verify ruling for want of a witness only that
ruling could write; a run the table routes past verify (decision
[0039](0039-the-docs-tier-inside-the-run.md)'s docs tier) parked at
review for the same want; a ruling that already took no transition had
its own reason overwritten; and a returned implement whose delta lay
wholly in the docs class, which 0041 ruling 5e routes to review without
verify, parked as drift. Porting it found a fifth: 0097 bound on any
advance where verify is protected, so a failing verification that ends
the run at a non-hard `stop` parked on a dirty tree instead of stopping;
here a ruling that ends the run binds nothing, since no judge or ship
follows it. This record is 0097 ruling 3 restated over main,
where 0109 has since built the stamped heads; 0097's rulings 1 and 2 are
decisions 0120 and 0121 and 0109 ruling 2.

## Rulings

1. **A passing verification binds the candidate; the judge and the ship
   are held to it.** `verified_candidate` is an engine-owned record input
   (`{head, tree}`) a seat can neither declare nor supply. When a bundle
   hires a `verify` seat, verify's ruling that hands the run to the
   protected phase, or, where verify is itself the protected phase, to
   any phase that does not end the run, reads HEAD and `HEAD^{tree}`
   from a worktree that
   `git status --porcelain --untracked-files=all` reports clean and
   writes them into the decision's inputs; a tree with anything
   uncommitted, or whose status git refuses, parks the ruling instead
   (`verification candidate is dirty`). At the protected phase and at
   ship, a ruling that takes a transition of severity below hard is
   parked unless the worktree reads clean the same way and its HEAD and
   tree are the pair verify's latest ruling journaled, or its HEAD
   differs from that pair's only by paths in the docs class the
   repository declared at the verified head, read by the one
   classification the smith's docs return reads (0039 ruling 3, 0041
   ruling 5e). A verify ruling with no readable witness (a failed
   verification, or a journal written before this decision) parks the
   judge as missing. A worktree that names no commit binds nothing and
   is held to nothing, as `reviewed_heads` is recorded only where a
   repository answers. A run whose journal holds no verify ruling is the
   table's to route, as it was. A ruling that already takes no
   transition, or a hard one, admits nothing and keeps its own words. A
   refusal keeps the ruling's `rule_id`, takes no transition and names
   the problem, the shape a blocked artifact gate writes.

   **Enforcement binding:** `Engine::bind_verified_candidate` and
   `hold_to_verified_candidate` in
   `crates/brokkr-runtime/src/engine/decide.rs`, over
   `git_candidate` and `read_tree_counting` in `engine/worktree.rs`; the
   row in `crates/brokkr-core/src/policy/inputs.rs` makes the input
   engine-owned, so a seat's `verified_candidate` is dropped before the
   table (decision [0007](0007-input-provenance.md)). Tests in
   `crates/brokkr-runtime/src/engine/candidate_binding_tests.rs`: a
   seat's spoofed witness is replaced by the real pair; a clean commit
   after verify parks the judge and the ship; a dirty tree or index
   parks; a missing or unreadable witness parks; a docs-class delta is
   judged and shipped and a code path in it parks; a run routed past
   verify, a bundle with no verify seat and a worktree with no commit
   are held to nothing; verify as the protected phase binds when it
   hands the run on, ship is held to it, and a ruling that ends the run
   binds nothing over a dirty tree; a failing verification binds
   nothing; a park and a hard
   hold keep their own words; the witness survives an engine restart;
   and an untracked file the repository's own status hides parks.

2. **The limit of the reading.** The candidate is what the host's git
   answers over the run's own worktree. `rev-parse` and `status` read the
   worktree's git directory and the common directory, which a seat
   writes so that it can commit at all. A seat that sets a tracked path's
   `assume-unchanged` or `skip-worktree` bit in the index, or lists an
   untracked path in a shared `info/exclude`, holds a modification out of
   the status the engine reads, and a status read clean then binds a HEAD
   and tree the worktree's bytes do not match. The hold proves what git
   says of the worktree at verify and again at the judge and the ship,
   and that nothing was committed between; it does not prove the
   worktree's bytes against the seat-writable index. 0109b's tree check,
   which builds its own index from verified objects and reads excludes
   from committed rules only, is the reading that closes this limit; when
   it lands, this hold should read the clean proof from it rather than
   from `git status`.

   **Enforcement binding:** none; this ruling states what ruling 1 does
   not prove, and names 0109b as the follow-up.

## Consequences

- A commit between verify and the judge, other than a docs-class one,
  now parks the run at the judge, where the table alone would have
  judged it as verified. The ship gate's `drift_detected` route is
  reached only for a docs-class drift under a verify seat, since any
  other drift from the reviewed head is also drift from the verified
  candidate.
- An implementer must commit before it reports: a repository that leaves
  a file uncommitted or untracked, which already parked at the ship
  gate's dirty rule, now parks at verify, naming the candidate dirty.
  The engine's own writes under the folder's `state/` must stay ignored,
  as `init` ignores them (decision 0083).
- A journal resumed across this change whose verify ruled before it
  parks at the judge with the witness missing; resuming from verify
  writes one.
- No frozen contract, fixture, reference schema or the production table
  changes: `transition/decided.inputs` is an open object, and the new
  input joins `reviewed_heads` there. The public API of `brokkr-core`
  changes only in the length of `ENGINE_OWNED_INPUTS`.

## Alternatives

- **Compare verify's `covered` head with the judge's `checked_head`.**
  Both are journaled already, and the comparison would need no fresh
  read. Rejected until 0109b: `covered.dirty` is `true` on every record,
  so it cannot say the verified head was the verified bytes.
- **Hold every run to a witness whether or not verify ruled.** It would
  make the docs tier and the research recipe unrunnable; the table's
  routing past verification is a signed choice of the bundle.
- **A sidecar file written by the verify seat.** The shape the adopter
  replaced: a seat-writable witness is no witness.
