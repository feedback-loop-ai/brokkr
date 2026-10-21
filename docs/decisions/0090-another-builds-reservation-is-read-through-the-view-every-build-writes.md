# 0090 — Another build's reservation is read through the view every build writes

Status: proposed
Date: 2026-10-20

Built: built
Amends: 0065

## Context

Issue #772. On 2026-10-19 a run from a build of main parked at its first
seat: dispatch was refused because "the protected inventory holds an
unknown participant" at another realm's run's reservation. That run was a
lanetally work order, alive, started by an older Brokkr binary on `PATH`,
with its own journal and worktree. While any such run stands, no run of
the newer build can dispatch, not even an exec gate.

Decision 0065 slice two (D6; U8a2a) keeps every managed writer on the host
in one protected inventory, `HOME/.local/state/brokkr/capabilities/`, one
reservation per run. Admission reads every reservation in it, and compares
the incoming attempt's writers with every standing attempt's. Whatever it
cannot read is "incomplete, inaccessible or ambiguous participation" (D6),
and refuses with MB3's identity cause.

Admission read every reservation, every realm's, whole: as `Reservation`,
with `deny_unknown_fields` throughout. The record's top level has been the
same since U8a2a's first build (`repo`, `run`, `workdir`, `artifacts`,
`journal`, `attempts`, `settled`; each attempt's `protects`, `writers`,
`cleanup`, and since U8a2b `serves`, which defaults). Only one field has
changed shape. Before U8a2c (`b6475fda`, #756), a boxed writer's
`reach.boxed.sealed` was a list of `{path, identity}`; since then it is one
record, `hands::Sealed {home, worktree, git, binds}`. Each side refuses the
other's record whole, so the skew already runs both ways: while a newer
build's run stands, the older build's runs refuse too.

What admission reads of another run has never included `sealed`, nor
`readable` or `workdir`. The lease, the comparison, the sources'
comparison and recovery read only the run's key and id, its artifact root
and journal, its settled attempts, and each standing attempt's
`protects`, `cleanup`, `serves` and writers, each by its site and its
reach (unbounded, or boxed to its `writable` roots beside its `sources`).
`sealed` is what this run's own door rechecks and its boxes are built
from, so it matters only in this run's own record.

The alternatives weighed:

- **Scope the inventory to this run's realm or worktree.** Refused. The
  inventory's security purpose is every live managed writer on the host:
  another realm's writer can still reach this run's artifact root or the
  coordination root.
- **Drop `deny_unknown_fields` from `Reservation`.** Refused. It loosens
  this run's own record, which the door rechecks and its boxes are built
  from, and it still could not read a field whose shape changed.
- **A tolerant view of other runs' reservations.** Taken (rulings 1 to 3).
  It reads only what admission needs of another run, and ignores the rest.
- **Version the whole file.** Weighed in ruling 4: a format number names
  the view, not the whole record.

## Rulings

1. **This run's own reservation is read whole, strictly.** The run's
   directory named by the claim's worktree key and run id holds this run's
   own reservation. It is read only as `Reservation`, with
   `deny_unknown_fields`, as before. A record there that this build cannot
   read whole (a field it does not know, or an older `sealed` list,
   written by an earlier attempt of the same run under another build)
   refuses by its name (`Unknown::Participant`). The join, the sources'
   extension, the door's recheck, the release and every mark read and
   seal only this record.

2. **Every other run's reservation is read through the participation
   view.** It is read whole where this build can read it whole, so a
   record of this build's own format is compared, recovered and sealed
   again exactly as before. Otherwise it is read through the view: `repo`,
   `run`, `artifacts`, `journal`, `settled`, and for each standing attempt
   its `protects`, `cleanup`, `serves` (default none) and each writer's
   `site` and `reach`, unbounded or boxed to `writable` beside `sources`.
   Every other field is ignored. A value of `protects` this build does not
   know reads as evidence at stake. A value of `cleanup` it does not know
   reads as not proven settled. A missing field the view needs, a reach of
   a kind it does not know, or a `serves` entry it does not know refuses
   that file by name (`Unknown::Participant`): with no writable roots,
   there is nothing to compare. The inventory's directory-level refusals
   (owner-only, no link, a worktree key, UTF-8 names, stray entries, an
   attempt directory the record does not name, a record naming another
   run) are unchanged. The view never widens what this run may do. Every
   writer the record reserves is still compared, as D6 and the operator's
   ruling of 2026-10-18 require. Only the record's other fields are
   tolerated.

3. **A record read only through the view is never sealed again.**
   Recovery proves another build's attempt settled from the same durable
   facts as any other attempt: its positive settlement fact (`cleanup:
   settled`), and a certified end in its journal, opened read-only, that
   no participant's writer can reach (M1, R2). The attempt is then
   released for that admission alone, and nothing is written: sealing the
   record in this build's format would drop fields the writing build
   needs, and refuse that build's own door recheck. Each later admission
   proves it again from the same facts. Until then, an attempt that is not
   proven keeps refusing whatever it would. The operator's path is the
   build that wrote it: resume or conclude that run with it, and its own
   release settles the record durably.

4. **The view carries a format number, and this build writes none.** The
   view reads an optional `format`. Absent, or `1`, it is the view of
   ruling 2, as U8a2a first wrote its fields. Any other value refuses that
   file by name and format (`Unknown::Formatted`), and the refusal tells
   the operator to conclude that run with the build that wrote it, or to
   use a build that reads it. A later build names a new format only where
   it changes what a field of the view means, or drops one. An additive
   change, or a change to a field outside the view (as `sealed` changed),
   needs none, because readers ignore it. This build's own record is
   format 1's, so it writes no `format`. Writing one now would make every
   build from U8a2c to this one refuse this build's records under
   `deny_unknown_fields`, which is the issue's failure in reverse, for no
   gain.

   What an older engine does with a newer file: a build before this
   decision reads every reservation whole and refuses any it cannot, as an
   unknown participant. Builds since U8a2c read this build's records whole,
   because the schema did not change. Builds before U8a2c refuse them, as
   they already refuse every record main writes, until they are retired or
   updated. A build carrying this decision reads every format-1 record of
   any build since U8a2a, and refuses a later format by name.

5. **Refusals name what they can.** An unreadable record refuses as
   before, by its path below the coordination root, which names its
   worktree key and run. A conflict with a writer of a record read through
   the view names that record's run as any other does (`Conflict::Held`,
   `Unboxed`, `Reaches`, `Replaceable`; MB3's `Serving`, `Unconfined`). The
   refusal does not add the other run's journal path: it would put another
   realm's host path into this run's journal, and the run id already
   identifies it.

## Enforcement

- Rulings 1 and 2: `crates/brokkr-runtime/src/engine/broker/inventory.rs`
  reads this run's own directory whole and every other through
  `inventory/standing.rs`. The comparison
  (`exclusion.rs`), the sources' comparison (`sources.rs`) and recovery
  (`settlement.rs`) consume only the view's `Standing`. Tests in
  `admission/tests/foreign.rs`: `a_foreign_reservation_kept_apart_is_admitted`,
  `a_foreign_writer_reaching_what_this_run_protects_refuses_by_its_run`,
  `an_unreadable_foreign_reservation_refuses_by_name`,
  `this_runs_own_reservation_is_read_whole`, and
  `an_unknown_participant_refuses_before_anything_is_sealed` in
  `admission/tests.rs`.
- Ruling 3: `settlement::recovered` seals only a record the inventory
  read whole.
  `a_foreign_reservation_proven_settled_is_released_but_never_sealed`
  asserts the planted bytes after every admission.
- Ruling 4: the view's `FORMAT`, and
  `an_unreadable_foreign_reservation_refuses_by_name`, which pins
  `Unknown::Formatted`'s text.
- Ruling 5 is judgment guidance for later refusals: no gate holds it.

## Consequences

- A run of this build dispatches beside another build's live run, as long
  as that run's writers are kept from what this run protects. The issue's
  park no longer happens.
- A reservation from a build before U8a2c, still standing on the host,
  is compared, never recovered into this build's format. Its attempts
  that are proven settled release in memory at every admission.
- The older build still refuses this build's records while one of this
  build's runs stands (ruling 4). Updating the binary on `PATH` that
  lanetally runs is the operator's remedy. Nothing in this repository can
  change what an installed older build reads.
- A later change to what admission compares must keep the view's fields,
  or name a new format, in a decision.
