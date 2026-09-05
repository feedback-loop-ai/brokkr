# 0047 — The finding is closed by name: a stopped run's residual is superseded only by an operator command that cites the run that closed it

Status: accepted (operator ruled in chat, 2026-09-06)
Date: 2026-09-06

## Context

On 2026-09-05 Muninn read a fleet of 139 runs and queued fifteen
residual findings. The first in its queue was the `REVIEW-SECURITY-HOLD`
that stopped `decision-0040-the-model-s-hands--96398324` at seq 190: a
high security residual on the review agents' read-write bind of the
operator's cargo home, and a high correctness residual on git inside a
worktree box. Both were fixed. The later run on the same feature,
`decision-0040-the-model-s-hands--415a7840`, shipped at head 77c3099;
on today's main the review agents bind `~/.cargo` in overlay mode and
the box binds a worktree's external git directory with its hooks hidden
and its config read-only, under decision 0043 ruling 6.

The operator then asked the question this decision answers: if these
are fixed, and Muninn runs one day from now, is there a record they are
fixed? There is not. Read against the tree:

- **A residual finding is derived from its own run's journal and
  nothing else.** `brokkr_view::residual_findings` walks one run's
  `transition/decided` events ruled from `verify` or `review` and lifts
  the structured rule inputs — `max_residual_severity`,
  `has_security_residual`, `high_risk_uncovered` — into findings, each
  cited by the sequence number of the ruling. A stopped run's journal
  ends at the ruling that stopped it. The finding stands as long as the
  journal does, which is forever.
- **No run references another.** `rerun` says so on its own help line:
  "No stored linkage." The run that shipped the fix carries no fact
  naming the run whose findings it closed, and the two share feature
  text by how they were started, not by any recorded relation.
- **Muninn does not read its own record back.** Each invocation
  rebuilds the dossier from the journals (decision 0020 ruling 1). Its
  2026-09-05 report noticed, for one entry, that "later runs on the same
  feature text completed and shipped", and ranked that finding at the
  bottom of its group for it. That was the seat reading feature-text
  similarity in the dossier and saying so in prose. It is advice, not a
  fact, and the next reading starts from zero.
- **The operator verbs cannot say it either.** `brokkr operator` admits
  `retry` and `stop`; `conclude` appends a stop conclusion to a parked or
  stopped run. Neither touches the ruling event the finding is derived
  from, so a concluded run's residual surfaces unchanged.
- **The journal already admits the annotation this decision needs.**
  `fold` refuses every event after a terminal status with one exception,
  stated in its own comment: "Terminal runs accept only operator
  annotations that change nothing" — `operator/commanded` and
  `operator/rejected` fold to `Ok(())` on a completed or stopped run and
  move no state. The v1 event vocabulary carries `operator/commanded`
  with `{ command_id, command, args, operator }`, where `command` is an
  open string and `args` an object the reducer never reads. A new
  command word with structured `args` is a v1 event byte for byte: no
  new event type, no new payload field, no extension schema, no
  manifest digest moved.

So the missing record is one operator command on the stopped run,
carrying the citation, folded as the no-op annotation the journal
already allows, and read by the dossier. That is the shape below.

Alternatives weighed:

- **Head-based staleness, derived and not recorded.** Every review
  ruling records `reviewed_heads`; the dossier could state, per
  finding, whether that head is an ancestor of the realm's default
  branch and whether a later run shipped on the realm since. Rejected as
  the answer, kept as a consequence: it says *stale*, never *fixed*, and
  a stale finding on an unfixed defect is exactly the finding Muninn
  must keep raising. It is cheap and derivable, so a later slice may
  add it as a dossier fact beside this one; it does not replace the
  operator's word.
- **Muninn reads its own record.** Feeding prior proposals into the
  dossier records that a finding was queued three times and never acted
  on. That is attention, not closure, and decision 0020 ruling 3 already
  keeps the proposals for a reader who asks. Rejected.
- **The shipping run cites the run it supersedes.** The engine cannot
  know it: a rerun has no stored linkage by decision, and a fresh run
  on a rebased branch has none to store. Only the operator knows that
  run B closed run A's finding, and only the operator may say so
  (decision 0020 ruling 2, the two-step law of 0005). Rejected.
- **A new event type, `finding/superseded`.** Rejected on the contract:
  the `type` enum is closed under `additionalProperties: false`, so a
  new type is a v2 event and moves every manifest digest for a fact one
  readout needs. The annotation the fold already admits is enough.
- **Delete or edit the finding.** Never. The journal is append-only and
  the run engine is its single writer (decisions 0001, 0020 ruling 3).
  A superseded finding stays in the journal and in the dossier, marked;
  it leaves only Muninn's queue.

## Ruling — 2026-09-06, operator: accepted as proposed

Accepted in chat the day it was proposed ("accept 0047"), without
amendment. The six rulings and their enforcement bindings stand as
written; the verb, the view field, the dossier mark and the charter
rule are the slice this decision names. The three items the
consequences leave open stay open.

## Rulings

1. **`supersede` is an operator command, and it is the only record that
   a residual finding is closed.** `brokkr operator supersede` appends
   one `operator/commanded` event to the run that carries the finding,
   with `command: "supersede"` and `args` of exactly this shape:

   ```
   { "findings": [<seq>, ...],
     "by": { "realm": <realm|null>, "run_id": <run>, "seq": <seq> },
     "reason": <text> }
   ```

   `findings` names the sequence numbers of the rulings on this run
   whose residuals are closed. `by` names the run and the ruling that
   closed them, keyed by realm as decision 0026 ruling 3 keys every
   fleet fact, `null` in a one-hearth world. `reason` is the operator's
   own sentence. `operator` and `command_id` are as they are today. No
   `operator/accepted` follows: there is nothing to execute, and the fold
   refuses an acceptance on a terminal run by design. The event is the
   record.

   **Enforcement binding:** `brokkr operator` gains the `supersede`
   verb with `--findings`, `--by-run`, `--by-seq`, and optionally
   `--by-realm`, beside the existing `--reason`. The `args` shape is
   published as `operator-supersede.v1.schema.json`, a payload schema
   over `args` for `command: "supersede"` and nothing else; the v1 event
   envelope is unchanged and the byte-identity witnesses do not move.
   A fold test folds a stopped run with a `supersede` annotation
   appended and asserts the state is byte-identical to the fold without
   it.

2. **Cite or say nothing binds the operator too.** Before writing, the
   verb verifies every citation against the world it reads, and refuses
   if any fails:

   - the run being annotated has terminal status (`completed` or
     `stopped`); a `supersede` on a running or parked run is refused,
     because on those the fold would hold it as a pending command and
     `operator/accepted` would refuse it as unknown;
   - every entry in `findings` is the `seq` of a residual finding
     `brokkr_view::residual_findings` derives for that run today;
   - `by.run_id` exists in the journal `by.realm` names (or in the
     workspace journal when `null`), and `by.seq` is a
     `transition/decided` event in it;
   - the superseding run is not the annotated run.

   A refusal writes nothing. An annotation that verified at write time
   is not re-verified at read time: the journal it cites is append-only,
   so what was true stays true.

   **Enforcement binding:** the verb's refusal paths, each with a test;
   the read-time rule is stated in the `residual_findings` docs and
   tested by a journal whose superseding run is later concluded and
   whose annotation still reads as it was written.

3. **The fold does not read it; the view does.** Ruling 1's event folds
   as the no-op annotation `fold` already admits on a terminal run;
   `RunState` gains nothing. `brokkr_view::ResidualFinding` gains one
   field, `superseded: null | { seq, by: { realm, run_id, seq }, reason,
   operator, recorded_at }`, where `seq` is the annotation's own
   sequence number on the annotated run. A finding named by more than
   one `supersede` carries the last in journal order; earlier ones stay
   in the journal for a reader who asks. Every surface that prints a
   residual finding prints the mark: `inspect` and `tui` on the ruling
   line, `runs --json`, and the dossier.

   **Enforcement binding:** `residual_findings` reads
   `operator/commanded` events with `command: "supersede"` after the
   rulings; a view test plants one and asserts the field; a replay test
   proves the fold is unchanged.

4. **The dossier carries the mark, and the annotation is a citable
   fact.** Each entry of `residual_findings` in Muninn's dossier carries
   `superseded` as ruling 3 states it. The annotation's `(realm, run_id,
   seq)` joins the dossier's closed set of facts, so a report may cite
   the annotation itself, and the superseding run's `(realm, run_id,
   seq)` is stated as a fact when that run is in the world this reading
   covers. The dossier's `counts` gain `superseded_findings`.

   **Enforcement binding:** `dossier_of` in `brokkr-cli/src/muninn.rs`;
   the tests in `muninn/tests.rs` plant a superseded finding and assert
   the mark, the fact, and the count.

5. **Muninn leaves a superseded finding out of the queue, and says how
   many it left out.** `agents/charters/muninn.md` gains one rule: a
   finding the dossier marks `superseded` is not queued; the
   `fleet_summary` states how many findings were superseded and by what,
   in one clause, so the operator sees the closure without being asked
   to act on it. A superseded finding whose superseding run is itself
   stopped or has its own residual is still not queued — the superseding
   run's residuals are, on their own citation. Muninn may not propose a
   `supersede`: it admits no operator command on a terminal run
   (`operator_commands` returns none for `completed` and `stopped`), and
   proposing one would be a proposal to close the record, which is the
   operator's alone. This decision does not widen the `parked_runs`
   vocabulary.

   **Enforcement binding:** the charter text; a Muninn test whose
   dossier carries a superseded finding asserts the citation check
   still passes when the report cites the annotation and that the
   rendered queue omits the finding.

6. **The annotation is not a conclusion and does not stand in for one.**
   `supersede` says a finding is closed elsewhere. It says nothing about
   the run's status, changes no ruling, and is not a ground for any rule
   in the phase machine. A run stopped on `REVIEW-SECURITY-HOLD` stays
   stopped on it. The finding is closed; the ruling was right when it
   ruled.

## Consequences

The operator gains the sentence the raven could not write: this finding
was closed by that run, on my word, on this date. Muninn stops queueing
what has been fixed, and every reader who asks why a high residual is
missing from the queue finds the annotation, the operator, the reason,
and the run it points to, in the same journal the finding lives in.

The cost is that the record is only as good as the operator's diligence.
Nothing derives closure; the annotation is an operator's act, one
command per closed run, and an unfixed finding annotated in error is a
finding Muninn will not raise again. That is why ruling 2 refuses a
citation that does not resolve and why ruling 3 keeps every annotation
in the journal: the mistake would be visible, on the record, with a
name on it.

What this decision leaves open, stated:

- **Head-based staleness** as a derived dossier fact beside the mark, so
  a reading can say "reviewed at b24110e, main now at c1ec539, no
  supersede recorded" and rank accordingly. A later slice; it derives
  from `reviewed_heads` and `realm_facts` the journal already carries.
- **Un-superseding.** The record is append-only, so a mistaken
  annotation is answered by another `supersede` naming the correct run,
  or by a run that re-reviews the tree. A `reopen` command is not ruled
  here; if the journal shows the need, it is its own number.
- **Backfill.** The fifteen findings Muninn queued on 2026-09-05 are
  the first candidates. Each is the operator's to annotate or leave, one
  command at a time; this decision ships the verb and annotates
  nothing.
