# 0105 — An operator retires a historical evidence root, with a record, once its evidence is archived or discarded

Status: accepted — operator ruled in chat 2026-11-08, "go with all recommended" (https://github.com/feedback-loop-ai/brokkr/issues/1020#issuecomment-6719129582)
Date: 2026-11-07

Built: unbuilt (#1020)

## Context

Decision 0065's slice two protects retained capability evidence on the host.
Its design (D6, and the U8a2a ruling of 2026-10-18) records an artifact root
that once protected evidence as **historical**. A historical root "stays
protected after every attempt that wrote it has settled, from every later
writer of any worktree on this host" (`engine/broker/historical.rs`), and
"no settlement removes the record". Since the operator's ruling of 2026-10-29
on U9b1, unboxed-writer refusals are keyed to evidence at stake. A historical
root is enough to put evidence at stake, so while one exists, every writer
Brokkr cannot box refuses at dispatch, on every run of the host
(`exclusion::guarded`, `exclusion::staked`).

**What happened.**
- On 2026-11-07 the house's engine canary was refused at dispatch: "site
  'implement' of run … writes outside a workspace box, which cannot be kept
  from protected evidence or the coordination root". The canary was current
  main, with an unboxed dsh implement seat that held no grant and had an empty
  artifact root.
- The cause was seven `historical-*.json` records written on 2026-10-31 and
  11-01 by real-harness measurement worlds under `/var/tmp`. Those worlds had
  run against the host's real coordination root.
- Every unboxed writer of every project on the host had been refused since
  then, on any engine with writer admission. The house lanes missed it only
  because their engine predates writer admission.
- The operator approved deleting the seven records by hand, and that cleared
  it.

**The gap.** Nothing retires a historical root: there is no verb, no
settlement and no repair. One real retained grant therefore turns a host into
one where every unboxed writer refuses forever. That is close to the
"refuse every run" clause the operator superseded on 2026-10-29 ("security in
balance, but Brokkr must be able to run, with capabilities").

**Ruled.** The operator, 2026-11-07, in chat, on #1020: **(a)**, a journaled
operator verb that retires a historical root once its evidence is archived or
discarded. The alternatives were refused:
- (b), comparing historical roots only against boxed writers, would let an
  unboxed writer reach the evidence the record protects;
- (c), boxing the house's implement seats, costs cargo in the box and seat
  signing for a scratch leftover.

This decision chooses how (a) is built. It is docs only.

## What main holds at HEAD

Measured at be0b6a871.

- **The record.**
  - `crates/brokkr-runtime/src/engine/broker/historical.rs`:
    `historical-<sha256 of the artifact root's path>.json`, sealed
    (`Root::seal`) in its worktree's directory of the coordination root, and
    holding `{ "artifacts": <path> }`.
  - `recorded` writes it under the admission lock at the first admission
    whose root protects evidence.
  - `verified` checks every `sha256/<hex>` artifact against its name, within
    a deadline, refusing with CR2's cause.
- **The read.** `crates/brokkr-runtime/src/engine/broker/exclusion.rs`:
  - `guarded()` adds every historical root to the coordination root and the
    protecting artifact roots;
  - `staked()` is `guarded().len() > 1`;
  - `judged()` refuses an unboxed writer with `Conflict::Unboxed`.
- **The documentation.** `docs/security-model.md` (about :426-439) states the
  rule and the refusals.
- **The nearest verbs.** `brokkr backup` and `restore` (decision 0100,
  `crates/brokkr-cli/src/verbs/backup.rs`): one consistent copy with a
  versioned inventory, refusing a destination that holds any file. `brokkr
  doctor` reads the capability state (`doctor/capabilities/`).

## Rulings

Each ruling gives the options and the recommended one. The operator may
accept all or pick.

### 1. The verb

**Recommended: (a) `brokkr evidence`, with `list` and `retire`.**

- **(a)** A noun of its own, because a historical root belongs to a host
  worktree, not to one run.
  - `brokkr evidence list` prints each historical root: its path, whether it
    still exists, its digest count and bytes, and any retirement.
  - `brokkr evidence retire --root <artifact root> (--archived-to <dir> |
    --discarded) --reason <words>` retires one.
  - `--json` on both.
- **(b) `brokkr operator retire --root …`.** Refused: `operator` commands
  name a run (`--run` is required), and this names none.
- **(c) A `brokkr doctor --repair` step.** Refused: doctor repairs derived
  state, and this is an operator's decision about evidence.

### 2. What may be retired

**Recommended: all of the following, checked under the admission lock
before anything is written.**

- The root has a historical record, and no retirement of it exists.
- **No standing reservation protects it:** no standing attempt's
  `artifacts` is this root.
- **No standing attempt's writer can reach it:** the same comparison
  admission makes. A root still in use is refused by name: "evidence root …
  is protected by run …'s standing attempt".
- **`--archived-to <dir>`:**
  - The destination does not exist, or is an empty directory, as `backup`
    requires.
  - Every artifact is first checked with `historical::verified`'s digest
    check, then copied to `<dir>/sha256/<hex>` with each copy's digest
    checked. An inventory, `brokkr.evidence-archive.v1`, is written last:
    the root, every digest and its bytes, the total, the host's engine
    version, and when.
  - A digest that does not match refuses the retirement. The evidence is
    not what its name says, and archiving it would launder that.
- **`--discarded`:** the operator gives the evidence up.
  - If the root still exists, the verb removes it in the same locked step,
    by a walk that follows no link and crosses no filesystem. No unprotected
    evidence stays under a name a reader could trust.
  - A root that is already absent is recorded as absent.

### 3. The record of a retirement

**Recommended: (a) a sealed `retired-<key>.json` beside the historical record,
which stays.**

- **(a)** The fields are `artifacts`, `cause` (`archived` or `discarded`),
  `archive` and `inventory_sha256` when archived, `operator`, `reason`,
  `retired_at`, and `engine`.
  - It is sealed as reservations are, in the same directory, under the same
    lock.
  - Admission reads a historical root that has a retirement as not guarded.
  - A retirement that does not read refuses with CR2's cause, never read as
    absent and never as retired.
  - The historical record is never deleted, so the host keeps its history.
- **(b) Delete the historical record.** Refused. The history is lost, and a
  crash between removing the evidence and deleting the record leaves the
  host holding neither.

### 4. "Journaled"

**Recommended: (a) the sealed retirement is the record of record, with no
journal event.**

- **(a)** A historical root belongs to a worktree's run-local directory,
  which many runs share, and the coordination root has no journal. The
  sealed record in the root is what admission reads, and `brokkr evidence
  list` prints it.
- **(b) Also append an operator event to the journal the operator names.**
  It is visible in that journal's readers, but it is a second home a reader
  could find disagreeing with the first. Not recommended.

### 5. After retirement

**Recommended:**

- `brokkr inspect --run <run> --capability-call <id>` reports a retained
  response whose root was retired as "archived at <dir>" or "discarded",
  read from the retirement, and never as corrupt.
- Reading the archived bytes back is not part of this decision.
- `brokkr doctor` names a historical root whose artifact root is absent. It
  still stakes the host until retired, so the operator sees why unboxed
  writers refuse.

### 6. Keeping scratch out of the host's root

**Recommended: documentation and doctor, no new refusal.**

- `docs/security-model.md` says that a measurement or scratch run that may
  retain evidence gives the run its own `XDG_STATE_HOME`.
- `brokkr evidence list` and doctor show each root's path, so a root under a
  temporary directory is easy to see.
- A refusal keyed to "a root under /tmp" is not proposed. A heuristic over
  paths is not a security property.

## Ruled (operator, in chat, 2026-11-08)

The operator answered "go with all recommended" (#1020,
https://github.com/feedback-loop-ai/brokkr/issues/1020#issuecomment-6719129582), and each ruling above was taken as recommended:
1. 1(a), `brokkr evidence list` and `brokkr evidence retire --root …
   (--archived-to <dir> | --discarded) --reason …`, with `--json` on both.
2. A root may be retired under the admission lock only when it has a
   historical record and no standing attempt protects or can reach it;
   archiving verifies every digest before and after the copy, and
   discarding removes the root by a no-follow walk.
3. 3(a), a sealed `retired-<key>.json` beside the historical record,
   which stays.
4. 4(a), the sealed retirement is the record, with no journal event.
5. `inspect` names archived or discarded evidence, never corrupt, and
   doctor names a stale historical root.
6. Documentation and doctor keep scratch runs out of the host's root,
   with no new refusal.

The acceptance mints the inventory, `brokkr.evidence-archive.v1`, under
decision 0082 when its slice lands.

## What stays out

- **Reading archived evidence back,** or restoring it into a root.
- **Retiring by age or by count:** a retirement is one root, by name, by an
  operator.
- **The protection itself.** D6, the coordination comparison and the
  2026-10-29 ruling are unchanged; this adds only a way out of them for one
  root at a time.

## The slices

One slice.

- **Files:**
  - `crates/brokkr-runtime/src/engine/broker/historical.rs` (the retirement
    record, its read, and the archive copy's digest check);
  - `broker/exclusion.rs` (`guarded` skips a retired root);
  - the inventory's read of retirements;
  - a CLI verb `brokkr evidence` (`crates/brokkr-cli/src/verbs/evidence.rs`,
    with its args);
  - doctor's lines;
  - the `brokkr.evidence-archive.v1` schema and its lifecycle row;
  - `docs/security-model.md`.
- **Tests:**
  - a root with a historical record stakes the host;
  - after `retire --archived-to`, it does not, the archive verifies, and the
    inventory names every digest;
  - `--discarded` removes the root and records it;
  - a root a standing attempt protects or can reach refuses;
  - a non-empty destination refuses;
  - a mismatched digest refuses with nothing written;
  - a retirement that does not read refuses admission;
  - a link at the root, or a root on another filesystem, refuses the
    discard;
  - `list` and doctor print every root, its existence and its retirement.
- **Queue order:** after decision 0103's slice 4c1 merges, which is editing
  `broker/admission` and its neighbours.

## Consequences

A host that once ran a retained grant can return to admitting unboxed writers,
one root at a time, by an operator's recorded act that keeps the evidence or
says it was given up. Nothing becomes unprotected silently. The price is one
verb, one record kind and one minted inventory.

## Evidence

The 2026-11-07 refusal, the seven records and their deletion are recorded on
#1020 and in the controller's session.
