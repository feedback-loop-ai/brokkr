# 0097 — What ships is what verification passed and review judged

Status: proposed
Date: 2026-10-26
Built: built

## Context

Three checksum-pinned engine patches from swarmail (#893) close gaps in
how a review verdict and a verification reach the policy table and the
ship gate, all under the no-repair rule of [0001](0001-no-llm-repair-of-control-plane.md)
and the strict evaluation of [0004](0004-strict-condition-evaluation.md):

- A review panel joined its members' `max_residual_severity` and
  `has_security_residual` only when the worst member said `residual`
  or named a severity above none, and read a member's missing or
  malformed flag as `false`. A member that omitted its facts could
  therefore lend a panel the `has_security_residual: false` the table's
  tracked-debt rule asks for, and a judges-never-fix charter was journaled
  as `fixes_applied: false` whether or not any judge said so.
- A singleton review could answer `clean` while reporting a security
  residual or a severity above none in its own inputs, and the table
  ruled on the word.
- The ship gate compared HEAD against the head review recorded, but
  nothing bound the tree verification had passed: a commit between
  verify and review, or a file hidden from `git status` by the
  repository's own setting, reached review and ship as verified code.

Upstreaming the patches unchanged (#893) found four compatibility faults
in the third, which this decision rules on rather than carries: a recipe
whose protected phase is `verify` itself (`research`) parked every verify
ruling for want of a witness only that ruling could write; a run the
table routes past verify, decision [0039](0039-the-docs-tier-inside-the-run.md)'s
docs tier among them, parked at review for the same want; a ruling
that already took no transition had its own reason overwritten; and a
returned implement whose delta lay wholly in the repository's docs class,
which decision [0041](0041-one-office-per-seat.md) ruling 5e routes to
review without verify in every recipe built on the `fast` table, parked
at review and at ship as drift from the candidate.

## Rulings

1. **A panel joins safety unknowns as unknown, and scope flags as
   witnessed or false.** A member's `has_security_residual` and
   `max_residual_severity` are read as a boolean and as the severity
   vocabulary, or not at all. A witnessed `true` dominates; the joined
   security flag is `false` only when every residual member supplied
   `false`; otherwise it is `null`, which no `when` condition matches. A
   residual member that omits a readable severity leaves the joined
   severity `null`, unless another member witnessed one above medium,
   which the table's first-match stop must still see. The worst
   residual's severity and security flag reach the inputs whenever any
   member witnessed one, whatever the worst result word. `fixes_applied`
   and `spec_defect` are `true` when any member witnessed `true` and
   `false` otherwise: judges never fix ([0041](0041-one-office-per-seat.md)
   ruling 5), ruling 3 parks a judge that committed code anyway, and the
   frozen production table's no-fixes shortcut reads `false`; a `null`
   there sent every clean panel review back through verify without
   bound.
2. **A clean verdict carries no residual.** A review result whose word is
   `clean` and whose inputs name `has_security_residual: true` or a
   `max_residual_severity` above `none` is malformed. So is a clean
   verdict whose inputs name `has_security_residual` as anything but a
   boolean, or `max_residual_severity` as anything but a word of the
   severity vocabulary, `null` included: a clean word over a safety fact
   the engine cannot read is refused, never read as the absence that
   would admit it. A fact the inputs do not name is absent, and a clean
   verdict needs neither. A panel whose every member is clean folds any
   member's malformed verdict to the member-schema refusal, naming the
   member and the problem; a hold or a residual beside such a member
   dominates and keeps the table's own stop, as it did. The engine
   refuses a malformed clean verdict at the protected phase's decision
   boundary, singleton and panel alike, as a schema failure parked with
   the raw evidence (0001). No other result word is checked: the table
   rules on `residual` and `clean-unverified`.
3. **A passing verification binds the candidate; the judge and the ship
   are held to it.** `verified_candidate` is an engine-owned record input
   (`{head, tree}`) a seat can neither declare nor supply. When a bundle
   hires a `verify` seat, verify's ruling that hands the run to the
   protected phase, or that advances at all where verify is itself the
   protected phase, reads HEAD and `HEAD^{tree}` from a worktree `git
   status --untracked-files=all` reports clean and writes them into the
   decision's inputs; a tree with anything uncommitted, or whose status
   git refuses, parks the ruling instead. At the protected phase and at
   ship, a ruling that takes a transition of severity below hard is
   parked unless the worktree reads clean and its HEAD and tree are the
   pair verify's latest ruling journaled, or its HEAD differs from that
   pair's only by paths in the docs class the repository declared at the
   verified head, read as the smith's docs return reads it (0039 ruling
   3, 0041 ruling 5e): prose over a verified candidate is what the table
   already admits past verify, and a code path in the delta is drift. A
   verify ruling with no readable witness (a failed verification, or a
   journal written before this decision) parks as missing. A worktree
   that names no commit, no
   repository, an empty one or no git, binds nothing and is held to
   nothing, as `reviewed_heads` is recorded only where a repository
   answers (decision 0023) and every machine proof runs outside one. A
   run whose journal holds no verify ruling is the table's to route, as
   it was. A ruling that already takes no transition, or a hard one,
   admits nothing and keeps its own words.

   **The limit of the reading.** The candidate is what the host's git
   answers over the run's own worktree: `rev-parse` and `status` read
   the worktree's git directory and the common directory, which a seat
   writes so that it can commit at all. The hands box binds both
   writable but for `hooks`, hidden, and `config`, read-only (decision
   [0043](0043-the-hands-are-one-tool.md) ruling 6; the write set is the
   one decision [0054](0054-the-dsh-harness-sandbox-reaches-a-linked-worktree-s-git-metadata.md)'s
   consequences record), and the dsh sandbox serves the per-worktree
   directory, index and `HEAD` among it, as the seat's own over a private
   common directory (0054). So a seat cannot make the engine's git run a
   program, and nothing holds the index or, in the hands box, the exclude
   file. A seat that sets a tracked path's `assume-unchanged` or
   `skip-worktree` bit in the index, or lists an untracked path in the
   common directory's `info/exclude` where that directory is the shared
   one, holds a modification or a file out of the status the engine
   reads, and a status read clean then binds a HEAD and tree the
   worktree's bytes do not match. The hold therefore proves what git
   says of the worktree at verify and again at the judge and the ship,
   and that nothing was committed between; it does not prove the
   worktree's bytes against the seat-writable index. The engine holds no
   copy of the repository a seat cannot write, so no reading in this
   decision comes from such state. Closing the limit
   takes an engine-owned reading that consults no seat-writable index or
   exclude file, such as a fresh index written over the worktree and
   compared to `HEAD^{tree}` with excludes read from the committed
   `.gitignore` alone, or the dsh sandbox's reclaimed store for every
   adapter; that is a follow-up, not this decision.
4. **Observations stay in one home.** The status and `rev-parse` readers
   under `engine/worktree.rs` serve the carry, ship and candidate
   decisions alike; the candidate's reading differs from the ship gate's
   only in counting every untracked file. The docs classification of a
   delta between two commits is one function, which the smith's return
   and the candidate's hold both call. The witness is parsed once into
   a typed record and compared as one (0071 rulings 3 and 5).

## Consequences

- A reviewer that commits code parks the run at its own ruling: the
  house's judges never fix ([0041](0041-one-office-per-seat.md) ruling 5),
  and a table path that re-verifies after review fixes is reached only by
  resuming from verify. The ship gate's `drift_detected` re-review is
  reached only for a docs-class drift under a verify seat, since any other
  drift from the reviewed head is drift from the verified candidate; the
  engine insists on re-verification where the table alone would have
  re-reviewed. The smith's docs return (0041 ruling 5e) reaches review
  and ship as it did, held to the verified candidate under its prose.
- A table that reads the panel's `fixes_applied: false` shortcut, the
  frozen production table and the machine proofs among them, sees the
  value it always did from judges that do not report fixes; a judge that
  reports `true` is no longer overwritten. A member's malformed
  `has_security_residual` or `max_residual_severity` now leaves the
  panel's fact `null` where it once lent the panel `false`; what a
  `null` does to a `residual` is the table's. Under `fast`, `landing`
  over it, `verify` and `preflight`, every residual route is earned by
  a value (`max_residual_severity_above: none`, and in `verify` and
  `preflight` `has_security_residual: false`), so a `null` matches no
  residual rule and the pair parks unmatched under 0004's strict
  evaluation. The frozen production table's `REVIEW-RESIDUAL-OK` carries
  no `when`: a `residual` whose facts are `null` passes its two stops,
  which only a witnessed `true` or a ranked severity above medium match,
  and proceeds to regression as flagged debt, exactly as it did when the
  facts read `false`. The production table is frozen and does not
  change here: under it the word `residual` alone reaches the
  tracked-debt route, and only a witnessed `true` or a ranked severity
  above medium stops it, so the join's `null` neither lends nor
  withholds anything there.
- A clean verdict's safety facts are read or refused (ruling 2): a
  singleton or an all-clean panel that writes `has_security_residual:
  "true"`, `1` or `null`, or a `max_residual_severity` outside the
  vocabulary, parks as a schema failure with the raw evidence where it
  once reached the table's clean route as if the facts were absent. A
  clean verdict that names neither fact is unchanged.
- A journal resumed across this change whose verify ruled before it
  parks at review or ship with the witness missing; `--resume --from
  verify` writes one.
- The engine's own writes under the folder's `state/`, which `init`
  ignores from inside the folder (decision 0083), must stay ignored, and
  an implementer must commit before it reports: a repository that leaves
  either untracked parked at the ship gate's dirty rule already and now
  parks at verify, naming the candidate dirty. Two machine proofs that
  verified uncommitted writes now ignore the state and commit the source.
- No frozen contract, fixture, reference schema or the production table
  changes: `transition/decided.inputs` is an open object, and the new
  input joins `reviewed_heads` there. The public API of `brokkr-core`
  changes only in the length of `ENGINE_OWNED_INPUTS`.

## Alternatives

- Holding every run to a witness whether or not verify ruled would make
  the docs tier and the research recipe unrunnable; the table's routing
  past verification is a signed choice of the bundle, not an evasion.
- Re-reading HEAD and cleanliness after the tree to close the race
  between reads adds branches no test reaches, which the exact-coverage
  gate refuses; a writer racing the single pass yields a pair no later
  clean pass reproduces, so the race parks and never admits.
- A sidecar file written by the verify seat was the shape swarmail
  replaced: a seat-writable witness is no witness.
