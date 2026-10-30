# 0093 — A retry by member re-runs the named panel members and carries the others' settled verdicts

Status: accepted — operator ruled in chat 2026-10-30, "go with recommended" (#361, https://github.com/feedback-loop-ai/brokkr/issues/361#issuecomment-6522822871)
Date: 2026-10-21

Built: built

## Context

Issue #787, part of epic #790 (cheaper recovery). `panel_outcome`
(`crates/brokkr-runtime/src/engine/panel.rs`) parks the whole attempt when
any member of a panel is indeterminate, and `brokkr operator retry` re-runs
the phase as a new attempt with every member. The verdicts that settled are
discarded although nothing they judged has changed. On 2026-10-19 one
reviewer of a four-member panel ran 141 turns and wrote no verdict, and
recovering it re-ran all four, about forty minutes for one stuck seat.

The pieces already exist. The engine reads the repository's head
(`git_head`) and whether the tree is dirty (`git_dirty`); the journal records
the digest of every effect's requested input (`effect/requested.input_digest`)
and the engine rebuilds and compares it before it executes; the journal pins
the run's manifest at `run/started` and `brokkr resume` refuses a bundle that
does not reproduce it (`ManifestMismatch`); `operator/commanded` carries an
`args` object the fold never reads (decision 0047); `effect/started` carries
per-member entries keyed by the member's tag (`provenance`, decision 0016;
`boundary`, decision 0046 ruling 3); a sequence journals a
`sequence-step-finished` marker per step; an attempt on `driver: "none"` is
how the engine refuses to run something other than what was requested; the
fold reads an unmarked indeterminate reason to a stated park reason; and the
aggregate is a pure function of the members' result values.

What is missing is a verb that names the members to run, a record of the
verdicts that settled in an attempt that ended short of success and of the
head and the tree they judged at both ends of the attempt, a rule that says
when such a verdict may be carried, a reading of the tree that trusts git's
exit, and a readout that names a carried verdict.

Weighed and set aside, each recorded in the change's proposal (S1 to S32):

- **The verb alone deciding the carry.** A head moved between the command
  and the drive would be carried: a fail-open on the review gate.
- **The engine alone deciding it.** The operator would learn of a refusal
  two commands later; the issue asks the command to refuse. The verb
  therefore builds the engine as `brokkr resume` builds it, which drives
  nothing, and asks every fact through it, the member's inputs included.
- **Running the whole panel when a carry no longer holds at the drive.**
  Silent, and it spends exactly what the operator asked not to spend.
- **Carrying a panel step after another step.** Its members read
  `context.prior_results`, which the requested digest does not cover. A
  step-level carry is a later slice of the epic.
- **A per-member judged-input digest recorded at settlement.** A second
  definition of "the input" (decision 0071 ruling 5); the source effect's
  recorded digest, rebuilt by the one function that builds every member's
  input, the manifest gate, the charter binding and the head and tree
  already hold what a member judged.
- **The serving candidate or the session as an identity fact.** They name
  who judged and how it was run, never what was judged: a chain fallback
  between attempts moves a fresh member's candidate alone, and the
  candidate that served a carried verdict stands in its source attempt's
  provenance, where every readout names it.
- **A second spelling of a member's name.** The bare name of a sequence
  step's member beside its tag; the tag is the one spelling the journal keys
  by and every readout prints, and a second spelling needed a fact of its own
  to fail closed where the two collided (decision 0071 rulings 5 and 6).
- **Skipping a carried participant in the transcript verb's selection.**
  The verb lists every participant of an ambiguous label today, those with
  no transcript included, and refuses `no-reference` when such a key is
  chosen; one kind skipped would be a second rule.
- **The head alone as the identity.** An uncommitted edit changes what a
  fresh member judges without moving the head.
- **The carrying attempt's own ends held by the record alone, for the next
  carry to refuse.** A succeeded attempt records nothing, so a head moved
  before the gate-head observation was armed, a tree dirtied without a
  commit while an all-gate panel judged, or a fresh work member's commit
  inside an all-work panel would have joined the carried verdicts with a
  fresh one on another subject and published the aggregate; the panel asks
  its own two ends before it publishes (ruling 3).
- **Tightening the ship gate's reading of a repository git refuses to
  read.** A second feature under one decision, with no consumer in #787;
  named for a decision of its own.
- **A marker, a codec and a typed park reason for a refused carry.** No
  consumer in state but a painter; the sentence is the record.
- **Journaling a refused member retry.** `operator/rejected.reason` is a
  closed vocabulary that cannot name a member or two heads.

## Rulings

1. **A retry names the members that run.** `brokkr operator retry` accepts
   `--member <name>`, repeatable, on `retry` alone, and with it `--repo`,
   `--bundle`, `--recipe`, `--recipes-dir` and `--secrets-file`, read and
   defaulted as `resume` reads them: every argument the engine `resume`
   builds is built from, since the seat input embeds the request's
   workspace and repository and, for a seat that declares secrets, its
   store's path (decision 0012). A member is named by its tag, the one
   spelling the journal keys it with and every readout labels it by,
   `<member>` in a seat's panel and `<step>:<member>` in a panel step,
   matched exactly against the tags of the one panel of ruling 2 and read
   no other way, so no name can read as two members; the tags are
   journaled on the retry's `operator/commanded` as `args.members`, in the
   panel's declared order, the order the compiled bundle holds its members,
   whatever order the names were given in: the verb journals the fresh
   members of the plan the one rule returns and resolves nothing itself. A
   name that is no member's tag is refused naming the panel's tags in
   declared order, and a tag given twice is refused naming it; the engine
   matches journaled tags by the same exact match. A retry without `--member`
   journals `args: {}` and re-runs every member, byte for byte as today, as
   do the Looper bridge and the console. The fold does not read
   `args.members`; the engine reads them for the one effect the acceptance
   requested and for every bounded attempt of that effect, and for no later
   effect; an `args` that does not read as the verb writes it refuses the
   drive by name and is never read as the plain retry (ruling 4). Naming
   every member is admitted exactly when the plain retry is admitted, the
   engine `resume` builds admits the run, the visit's `panel-carry.v1`
   records read and the names match; a record that does not read refuses
   it, at the command and at the drive, as it refuses any member retry.
   Nothing is carried.

   **Enforcement binding:** `crates/brokkr-cli/src/verbs/delivery/member_tests.rs`
   (the argument facts before every refusal the handler makes today; the supersede-only
   flags' refusal on `retry` names what `retry` takes);
   `crates/brokkr-runtime/src/engine/operator/retry/tests.rs` (the
   journaled tags in declared order when the names are given out of it, the fold's refusal writing nothing, every member named
   beside a record that does not read refused for the record);
   `crates/brokkr-runtime/src/engine/carry/tests.rs` (the selection's scope:
   no acceptance before an effect's request selects nothing; a tag matched
   exactly, `p:x` naming the member `x` alone where another member is
   named `p:x`; a bare name refused naming the panel's tags);
   `contracts/panel-carry.v1.schema.json` (`args.members`).

2. **The carry applies to one panel, and a verdict is carried only when
   nothing it judged has changed.** The panel is read from the pinned
   bundle, which the verb holds as `brokkr resume` holds it: it builds the
   engine through `resume`'s own admission, the compile, the manifest gate
   over the pin at `run/started`, the charter binding and the crossing
   fence, spawns and drives nothing, and is refused in `resume`'s own words: the parked phase's body when it is a panel, or a sequence's
   first step when that step is a panel. A verdict settled in an attempt is
   carried only when four facts hold: the repository's head, recorded at
   both ends of the parked attempt and of every attempt the verdict is read
   from or through, equals the head now, read at the command and again at
   the drive; the working tree read `clean` at those same ends, at the
   command and at the drive; the bundle is the pinned one; and the seat
   input rebuilt for the source attempt's effect, by the engine built from
   every argument `resume`'s is, the secrets store among them, digests to
   the `input_digest` that effect recorded. A member that never settled cannot
   be carried, a name the panel does not have is refused, and a verdict is
   read from the parked attempt alone: settled there, or named by that
   attempt's own `carried` entry. The parked attempt is the latest of the
   phase's current visit that its panel ended, told by one of three marks,
   `effect/succeeded`, a terminal fact recording `tree`, or the panel step's
   `sequence-step-finished` marker; an attempt bearing none is passed over,
   and the head and tree it could have changed are read again. The verb
   asks all four facts before it writes, the inputs through the engine it
   built, by the one function the drive rebuilds them with, in one order
   with one text per fact, and a refused command writes nothing; the
   command is disposed of against the fold the facts were read at, so after
   its write only the fence's own race refusal, `lost_fence`, can follow,
   and the acceptance carries the tags alone; the engine asks the same
   facts at the drive before anything spawns, and a carry that no longer
   holds starts the attempt on no driver and ends it `effect/indeterminate`
   with the refusal's own sentence as its reason, so the run parks and
   nothing is auto-retried.

   **Enforcement binding:** `crates/brokkr-runtime/src/engine/carry.rs`,
   one pure rule over the facts one engine method observes at the command
   and at the drive, deciding the two panel facts from the selected body's
   shape before the names as well as every later fact, and its `tests.rs`
   (every refusal by its fact and order; the three-way test over every
   passed-over kind; the read-through);
   `engine/operator/retry/tests.rs` (a moved input refused at the command,
   nothing written; a peer's acceptance before the disposition refused
   `lost_fence`);
   `engine/carry/drive_tests.rs` (a head moved between acceptance and drive
   parks with the sentence; a moved input names its effect; an accepted
   retry whose members meet a body with no panel, or a panel step after
   the first step, parks on no driver naming the fact); `crates/brokkr-cli/tests/machine_proof/panel_carry.rs`
   (a carried verdict is refused once the head moves).

3. **The carrying attempt aggregates as a fresh panel.** Only the named
   members run; the declared aggregate joins carried and fresh verdicts
   exactly as it joins a fresh panel's, so the attempt's result, `inputs`
   and `notes` are the bytes a full re-run produces over the same verdicts,
   and policy routes them unchanged. Every per-site record and plan of the
   attempt covers the members that run and no other: `provenance`,
   `boundary`, the driver label, the session offers, the admission claim and
   the member markers. A fresh indeterminate parks the attempt, a fresh
   failure fails it, and the bounded retry of that effect runs the named
   members again with the same carried entries. Before the aggregate is
   published, the attempt's own two ends, the panel's start and its end,
   hold the subject the carry was admitted on, the head at both and the
   tree `clean` at both; a span that does not hold ends the attempt
   `effect/indeterminate` with the refusal's own sentence naming its two
   ends and the admitted head, records what ruling 4 records, and
   publishes no aggregate, so the next member retry is refused by that
   record and the plain retry re-runs the panel; an all-gate panel whose
   head moved after the gate-head observation was armed ends
   `GATE-MOVED-HEAD` as today, and a work panel carries only when its
   fresh work leaves the subject unchanged.

   **Enforcement binding:** `engine/carry/tests.rs` (the aggregate equal
   byte for byte, a carried `residual` kept; a span whose end is not the
   admitted head, or whose tree is not clean, indeterminate before the
   join); `engine/carry/drive_tests.rs`
   (the spawn count and every per-site record over the fresh members
   alone; the bounded retry's entries; a fresh work member's commit inside
   the carrying attempt parking by name with no `effect/succeeded`); `machine_proof/panel_carry.rs` (the
   same aggregate as a full re-run, and a residual from the retried member
   routed to implement).

4. **The journal records what a panel attempt settled and what an attempt
   carries.** Every panel attempt whose outcome its panel ended short of
   success, a seat's panel body or the panel step whose outcome ended a
   sequence, records on its `effect/indeterminate` or `effect/failed`:
   `tree`, the reading when the panel started, before any member of it
   spawned, and when it ended, before the terminal fact, which for a seat's
   panel and a sequence's first step are the attempt's own two ends and for
   a later panel step the step's; `head`, the repository's head at those
   same two moments in the commit-id shape
   `phase-entered-head.v1` pins, each end absent where the repository
   answered none or answered outside that shape, which the engine reads as
   no head as `phase-entered-head.v1` reads one; and `settled_members`, the
   result value of each member that settled fresh in that attempt, any JSON
   value as its driver reported it, an object or not, keyed by the member's
   tag, absent when none did; its reader reads any JSON value per tag, so
   the engine never writes a record its own reader refuses, and a carried
   value that is no object poisons the aggregate as a fresh panel's would.
   An attempt that carries verdicts records `carried` on its
   `effect/started`: one entry per carried member with its tag and the
   attempt the verdict settled in, never one that only carried it. The
   fields and `args.members` are optional, absent-by-default extension
   fields at `event_schema: 1`, published as
   `contracts/panel-carry.v1.schema.json` under the amended rule in
   `contracts/README.md`; the fold reads none of them, no event type or
   seat-record version moves, and an attempt that carries nothing journals
   bytes identical to today's. The contract pins each field's shape, the
   head ends to `^[0-9a-f]{40}$`, and a field present on an attempt that
   does not read as published, a head end outside the shape included, is
   refused by name at the command and at the drive and never read as
   absent (ruling 2). `args.members` is pinned to a non-empty list of
   tags; at the drive, an accepted retry's `args` that do not read, a key
   other than `members`, a `members` that is not a list of strings or an
   empty list, refuse the drive as a record that does not read, naming the
   command by its `operator/commanded` sequence number, while `{}` and an
   absent `args` are the plain retry. The contract is minted under this
   decision and lands on `main` only once it is accepted (decision 0082
   ruling 1).

   **Enforcement binding:** `contracts/panel-carry.v1.schema.json`,
   registered in `crates/brokkr-runtime/tests/frozen_contracts.rs`;
   `crates/brokkr-core/src/fold/tests.rs` (a journal with every new field
   folds to the state it folds to without them); `engine/carry/drive_tests.rs`
   (the record on both terminal facts, absent on every attempt the panel did
   not end).

5. **Every readout that paints participants names a carried verdict as
   carried, and a refused carry parks as a stated indeterminate.**
   `brokkr-view` derives a carried member's participant once, at status
   `carried`, naming the attempt its verdict settled in by its id and by its place among the phase's
   attempts, with no model, session or cost of its own; the surfaces that
   paint participants, `inspect`, `seats`, `watch`, the TUI, `ui.html` and
   the console, paint it. `brokkr runs` and the fleet readouts, whose run
   rows hold no participant, show nothing of a carried verdict and keep
   only a refused carry's stated park reason. A carried participant ran no
   seat, so it has no session, transcript, cost or provenance of its own
   and is not at work; the readers of participants that paint no status
   read it as such: `compare`'s resolutions skip it, muninn's cost rows
   count no cost for it, the transcript verb lists its key as it lists
   every participant's and refuses `no-reference` when it is chosen, as it
   refuses any participant with no recorded transcript, and the
   dashboard's working checkpoints, which keep only participants at work,
   hold none of it. A carry refused
   at the drive is a stated indeterminate whose reason is the sentence the
   verb refuses with for the same fact; the fold, `run/parked` and every
   readout treat it as they treat every stated reason today, and no marker,
   codec or park-reason variant is added.

   **Enforcement binding:** the view's tests in the module that reads
   `carried` (the participant's status, source and ordinal, and that it is
   not at work; a plain retry's view unchanged; `VIEW_VERSION` moved);
   `crates/brokkr-view/src/compare/tests.rs` (a carried label keeps its
   source participant's resolution);
   `engine/carry/drive_tests.rs` (the stated park reason and `run/parked` after a drive-time refusal);
   `machine_proof/panel_carry.rs` (`inspect` and `seats` naming every
   carried verdict).

6. **The working tree is read once, to four readings, and the ship gate's
   decision does not change.** One reader runs `git status --porcelain` in
   the operated repository and reads `clean` (exited zero, printed nothing),
   `dirty` (printed anything), `refused` (exited other than zero, printed
   nothing) or `unrun` (could not be run). The carry needs `clean` at every
   observation; the record keeps the reading. `git_dirty` is the ship
   gate's decision over the reading, `true` on `dirty` and `unrun` and
   `false` on `clean` and `refused`, which is today's answer byte for byte;
   `ship_inputs` does not change. That a repository with a commit whose
   index git cannot read ships as clean is older than this decision and is
   left to a decision of its own.

   **Enforcement binding:** `crates/brokkr-runtime/src/engine/worktree.rs`
   and `engine/worktree/tests.rs` (the four readings; `git_dirty`'s
   decision over each); the existing `git_dirty` test in
   `engine/tests.rs`, unchanged; the machine proofs that ship over a bare
   directory, unchanged.

## Consequences

- One stuck reviewer costs one reviewer's run. The incident's recovery is
  `brokkr operator retry --member <tag>`, the tag the readouts print
  (`correctness` in a seat's panel, `positions:correctness` in the shipped
  review sequence), then `brokkr resume`, and the three settled verdicts
  stand, named in the journal and the readouts.
- A carry never widens silently and never carries across a moved head, a
  dirty tree, a drifted bundle or a moved input: each refuses by name at
  the command, and fails closed at the drive as a park; and a carrying
  attempt whose own span moved publishes no aggregate, parking by name.
- The plain retry, the bridge, the console, the fold, the frozen contracts,
  the ship gate's answer and every recipe and charter are as they were.
- A journal written before this decision carries nothing; its plain retry
  is unchanged.
- The ship gate's reading of a repository git refuses to read is now
  observable (`refused`) and unchanged in effect, which puts the question
  to the operator as its own decision.
