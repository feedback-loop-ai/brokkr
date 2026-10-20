# Design — a retry by member re-runs the named members and carries the rest (#787)

Decision 0093 carries the rulings. This design carries what the code does,
the evidence behind each choice, and the council's two positions reconciled.
The proposal's record (S1 to S22) holds the clarify history; a decision is
stated once here and cited there.

## Context

On 2026-10-19 one reviewer of a four-member panel ran 141 turns and wrote no
verdict; `panel_outcome` parked the attempt and the plain retry re-ran all
four members, about forty minutes for one stuck seat. The proposal cleared
eight clarify passes with four operator rulings (the head at both ends of
every panel attempt; the pinned bundle compiled under `resume`'s manifest
gate; the three-way test deciding the parked attempt; the reader's tests in a
new module) and two rulings by the operator's assistant under the standing
ruling for epic #790 (S20's text, S21's text). The council sat with two
positions, robustness and simplicity, both run-local; this design adopts,
rejects or combines each of their claims below, on the code as it stands.

The pieces the design builds on, as read on this branch:

- `execute_panel` (`crates/brokkr-runtime/src/engine/panel.rs:37`) derives
  one `MemberRun` per member (`member_runs`, `:117`), runs them, journals one
  `panel-member-finished` marker each (`:275`) and joins the reports by
  `panel_outcome` (`:448`), a pure function over `(name, result)` pairs.
- `execute` (`engine/dispatch.rs:41`) rebuilds the seat input and refuses a
  digest that moved (`refuse_changed_input`, `:131`: an attempt on
  `driver: "none"` with a terminal fact that says why), then plans the
  attempt and appends `effect/started` before anything spawns.
- `disposed` (`engine/operator.rs:364`) folds the run, writes
  `operator/commanded` with `"args": {}`, re-folds under the journal-head
  fence and accepts or refuses; `operator_stop_reason` (`:120`) is the
  precedent for the engine reading an accepted command's `args` back.
- The fold's `close` arm (`crates/brokkr-core/src/fold.rs:629`) reads an
  unmarked indeterminate reason to
  `ParkReason::Stated("effect <id> indeterminate: <detail>")`, and the `Park`
  cursor writes `run/parked` with that text and `{}` evidence
  (`engine.rs:1003`). Every readout paints a stated reason today.
- `git_dirty` (`engine.rs:3458`) reads only what `git status --porcelain`
  printed, with a failed spawn as dirty. `git_head` (`:3446`) reads
  `rev-parse HEAD`.
- `Engine::resume` (`engine.rs:696`) refuses a compiled bundle whose manifest
  is not the pinned one (`ManifestMismatch`), after `launch::resume`
  (`launch.rs:262`) compiled it through `compile_from_manifest` (`:419`).
- The view's scan (`crates/brokkr-view/src/lib.rs:1036`) reads
  `effect/started.provenance` and `.boundary` per site tag and the member
  marker's outcome into one `Build` per `effect:member`, from which
  `participants` (`:1774`) derives each `Participant`.

## Goals and non-goals

Goals, the issue's three acceptance lines and the proposal's rulings:

1. `brokkr operator retry --member <name>` (repeatable) on a parked panel
   runs the named members only, and the attempt's aggregate is the bytes a
   full re-run produces over the same verdicts.
2. A verdict is carried only when the reviewed head, the working tree, the
   pinned bundle and the member's inputs are what it judged; a carry that
   does not hold is refused by name at the command and fails closed at the
   drive.
3. Every carried verdict is named in the journal, typed, and in every
   readout.

Non-goals, each named with its reason:

- The ship gate's reading of the working tree does not change (D10). Both
  council positions refused the proposal's tightening as a second feature
  under one decision; the hole it would have closed is older than this
  change and goes to a decision of its own.
- No step-level carry, no carry of a panel step after another step, no carry
  across phase visits (S8, S10).
- The Looper bridge and the console keep the plain retry.
- No per-member judged-input digest (D5).
- No typed park reason for a refused carry (D8).
- The fold is not edited: no event type, no cursor, no arm, no field it
  reads (decision 0050 ruling 6).

## The council, reconciled

Each claim is adopted, rejected or combined on evidence, with the ruling of
decision 0071 it turns on.

Adopted from both positions:

- **The ship gate's semantics stay** (simplicity cut 1; robustness's
  "incidental ship-policy change"). `git_dirty` is
  `!stdout.is_empty()` with a failed spawn as dirty; a four-reading enum
  reproduces that boolean exactly as `dirty | unrun` (D10). #787 has no
  consumer for a changed ship gate (ruling 6), and a change to a hard
  stop's reading is a decision of its own. The operator's fourth ruling,
  the reader's tests in a new module, stands: the reader still lands in
  `engine/worktree.rs` with `engine/worktree/tests.rs`.
- **One pure rule, two observers** (S1; both). The verb and dispatch fill
  the same `CarryFacts` and decide by the same function (rulings 5, 10).
- **The drive-time recheck** (both). A head moved between the command and
  `brokkr resume` would otherwise be carried across: a fail-open on the
  review gate, rated high.
- **A complete, typed plan** (robustness invariant 2; simplicity's
  `CarryPlan`). One entry per declared member, `Fresh` or `Carried`, built
  by a constructor that refuses a member twice, a member missing or a tag
  outside the panel (D4).
- **Only a settled success is carry evidence** (robustness invariant 3; S6).
  `settled_members` holds the member's whole result object from its
  post-fence `settled_outcome`, so a `residual` or `fail` verdict carries
  as the finding it is; a run-local result file is never read (D6).
- **The tree at both ends, and at the command and the drive** (S12; both).
- **The selection has causal scope** (robustness; S11): the members apply
  to the one effect the acceptance requested and its bounded attempts, and
  to no later effect (D3).
- **A first-step panel only** (S10; both). A later panel step reads
  `context.prior_results`, which the requested digest does not cover.
- **The read-through of `carried`** (S17; both): one lookup, and an entry
  names the attempt the verdict settled in, never one that only carried it.
- **The compile at the verb through `resume`'s own gate** (the operator's
  second ruling; both insist it is one function, not a copy).
- **The journal's records are parsed once, at the edge** (robustness
  invariant 5). A `panel-carry.v1` field that is present and does not read
  refuses by name; it never disappears through a lenient read (D4).

Adopted from simplicity alone:

- **No marker, codec or `ParkReason` variant for a refused carry** (cut 2;
  D8). The typed refusal's only consumer in state would be a painter that
  turns it back into the sentence it was encoded from (ruling 6). The raw
  JSON the first clarify pass objected to existed only because S3 wrote
  JSON into the reason. With the sentence as the reason, the `close` arm
  and every readout already do what S14 asked for. Robustness's "typed
  carry evidence" is kept where it binds: one `thiserror` enum in the
  runtime whose `Display` is the verb's text and the journaled reason.
- **Every member named needs no special case** (cut 7). The identity facts
  are asked of the attempts a verdict is read from or through; with none,
  they are vacuous, which is S18 stated as the rule rather than beside it.
  The spec's order and scenarios already say this; no text moves.
- **`resume`'s argument defaulting is reused** (section 1). The member form
  builds its `LaunchRequest` through the same `request`
  (`crates/brokkr-cli/src/verbs/delivery.rs:79`) `resume` uses, with
  `secrets_file` absent (ruling 5).
- **The design states each decision once** (cut 8). This file.

Rejected from simplicity:

- **Falling through to the plain path when the fold refuses** (cut 3,
  against S21). `disposed` folds, writes the command, then re-folds under
  the fence and accepts whatever the second fold admits. A member form that
  handed its command to that path after a read-only fold refused it would
  have its carry facts asked by nobody when the run parked between the two
  folds: an accepted member retry with no carry check, a fail-open. Asking
  the facts under the fence instead means compiling a bundle and reading
  git while a fence is held, a second mechanism. S21's one function, with
  one consumer (ruling 6 is met), is the smaller and safer structure, and
  the operator's assistant ruled it.
- **Merging the four shape facts and the unnamed members into one
  `NoVerdict`** (cut 4). The operator's remedy differs by fact: a park at a
  later step admits the all-named retry; a succeeded attempt's members did
  settle, so "no verdict to carry" would misname what happened; "no attempt
  its panel ended" says the visit holds no evidence at all. The refusal is
  the one place the operator learns which (ruling 7). The variants are
  grouped under closed `why` enums (ruling 2) so no text is pinned twice.
- **`tree` as a boolean** (section 2). The reading is the fact the refusal
  names: `dirty` is the operator's edit, `refused` is git's, `unrun` is the
  host's. A closed four-word vocabulary is the typed form (ruling 3), and
  robustness asks for exactly that distinction.
- **One capability instead of three** (cut 6). Each capability has one
  fact to hold and will archive into `openspec/specs` as a living spec of
  its own: the verb and the rule; the journal's record and its readouts;
  the reader, whose four readings the record, the carry and the ship gate
  all consume. `working-tree-reading` shrinks to the reader and the ship
  gate's unchanged decision over it.

Rejected from robustness:

- **A per-member judged-input projection, digested at settlement** (D5).
  A member's input is the panel's seat input plus facts the pinned bundle
  and the effect id fix (`role_path`, `result_path`, the dialect and site
  marks), the secret-binding facts copied from the seat input, and the
  serving and resume facts, which say who judged, not what. The content a
  member judged is held by the seat input's recorded digest, the manifest
  gate and the head and tree. A second definition of "the input", with
  `result_path` excluded by hand, is what ruling 5 forbids; the whole-panel
  digest refuses more than a per-member one would, never less.
- **`effect_id` beside `attempt_id` on a `carried` entry.** The attempt's
  own `effect/started` names its effect, and attempt ids are unique in a
  journal. A second copy would be a second home (ruling 5).
- **Decision number 0092.** The commission's rule is "the next after
  #802's". `origin/main` took 0091 for #717 (`d7907e93`), #802 renumbered
  to 0092 (`68f8560f` on `origin/recovery/789-environment-blocks-park`),
  so the rule gives 0093; writing 0092 would collide at merge (D12).

Combined:

- **The origin and the status of a carried participant** (robustness:
  derive the origin apart from the verdict). A participant's `status` is
  the outcome word (`succeeded`, `failed`, `indeterminate`, `working`),
  never the verdict: a fresh member that reports `residual` stands at
  `succeeded`. A carried member stands at `carried`, which implies a
  settled success, and `carried_from` names its source; both derive from
  the one `carried` entry in the scan, so they cannot disagree (D9). No
  finding is hidden, because no status shows one today; the verdicts live
  in the aggregate's notes, which stay the fresh panel's.
- **Crash scenarios** (robustness). After acceptance, after request, after
  `effect/started` and after a partial settlement are each answered in D7
  and the tests of D13; a partial settlement is lost to the engine's
  restart indeterminate, fail-closed.

## Decisions

### D1. Where the code lands, against the ceilings

The ceilings of decision 0071 ruling 4 fix the modules. `engine.rs` (3,523
lines), `engine/operator.rs` (800), `brokkr-view/src/lib.rs` (2,716) and
`crates/brokkr-cli/tests/machine_proof.rs` may not grow; `cli_args.rs` has
seven lines of headroom; `engine/tests.rs` has none against
`quality/file-lines.txt`.

- `engine/worktree.rs`, with `engine/worktree/tests.rs`: the one reader
  (D10). `git_dirty` keeps its name and place in `engine.rs` and becomes one
  match over the reading, so the spawn it loses pays for the `mod` line.
- `engine/carry.rs`, with `carry/record.rs`, `carry/facts.rs` and
  `carry/tests.rs`: the rule (D4), the `panel-carry.v1` types that both the
  writer and the reader use (D6), and the observers that fill the facts from
  the journal and git. Observing is apart from deciding (ruling 10).
- `engine/operator/retry.rs`: the verb's member form (D2), beside
  `conclusion.rs`, `receipt.rs` and `supersede.rs`.
- `launch.rs`: `pinned_bundle(request, run)`, the compile and the manifest
  comparison as one function `resume` and the verb both call; the
  comparison moves out of `Engine::resume` into it, so `engine.rs` shrinks
  and neither caller copies it (ruling 5).
- `engine/dispatch.rs`: the drive-time check, the narrowed body, the start
  readings and the `carried` entries (D7). `Dispatch` gains the attempt's
  panel span, so no field is added to `Engine`.
- `engine/panel.rs`: `execute_panel` takes a parameter object and the plan;
  its nine parameters become seven and its `too_many_arguments` expectation
  goes (ruling 10: a long list becomes a parameter object).
- `engine/sequence.rs`: the first step's plan and the record on the
  sequence's terminal fact when the panel step's outcome ends it.
- `crates/brokkr-cli/src/cli_args/member_retry.rs`: the member form's
  arguments, flattened into `OperatorArgs` by one line.
- `crates/brokkr-cli/tests/machine_proof/panel_carry.rs`: the acceptance
  proofs, over a workdir the proof `git init`s, with `mod panel_carry;`
  after `mod output_edge;`.
- `crates/brokkr-view/src/started.rs`: the per-site readers of
  `effect/started`. `boundary_entries` and `BoundaryEntry` move there from
  `lib.rs` and `carried_entries` is added beside them, so `lib.rs` shrinks
  by more than the lines `Build`, `Participant` and `participants` gain.
- `crates/brokkr-core`: nothing.

### D2. The verb

`brokkr operator retry --member <name>` is the plain verb with a member form.
The handler (`verbs/delivery.rs:167`) asks the two argument facts first,
before its `supersede` branch: `--member` with any command word but `retry`;
any of `--repo`, `--bundle`, `--recipe`, `--recipes-dir` without `--member`.
Then today's refusals in today's order, with S20's one text on `retry`. With
members typed, the handler builds a `LaunchRequest` through `request` with
`secrets_file` absent, and calls `operator_retry_members` in
`engine/operator/retry.rs`, which runs in this order (S15):

1. Fold the run. A fold refusal (`acceptance_refusal`'s word) is reported
   by `brokkr_view::member_retry_refusal(word)`, S21's text, and nothing is
   written (decision 0047 ruling 2).
2. Compile the pinned bundle through `launch::pinned_bundle`; a refusal is
   `resume`'s own error, printed as `resume` prints it.
3. Fill `CarryFacts` at the command (D4, D5) and decide; a refusal prints
   the enum's `Display` and writes nothing.
4. Dispose through `disposed` with `args: {"members": [<tags>]}`. `disposed`
   takes a `Commanded { command, operator, reason, args }` parameter object
   so the bridge and the plain verb pass `{}` and nothing else changes.
   After the command is written only the fence's race refusal can follow,
   journaled as today.

Alternatives: journaling carry refusals as `operator/rejected` (S2; the
reason word is a closed vocabulary that cannot carry two heads); the fold's
refusal falling through to the plain path (rejected above, a race).

### D3. The selection's scope

`carry/record.rs` parses `operator/commanded.args` of a `retry` into
`RetryArgs { members: Option<Vec<Tag>> }` with `deny_unknown_fields`; `{}`
is the plain retry. At `execute`, the engine finds the `operator/accepted`
that caused this effect: the latest acceptance before this effect's
`effect/requested` with no other `effect/requested` between them, as
`operator_stop_reason` finds a stop's. Its command's `args.members` are the
attempt's selection, exactly matched against the panel's tags and never
resolved again. Every bounded attempt of the effect reads the same
acceptance; the next effect of the phase, after implement and verify, has no
acceptance before it and runs the whole panel. The fold reads nothing of
this (decision 0050 ruling 6; `RunState` is unchanged).

### D4. The rule

```text
carry_plan(facts: &CarryFacts, names: &[Tag]) -> Result<CarryPlan, CarryRefusal>
```

`CarryFacts` is what was observed: the panel (its tags in declared order,
from the compiled bundle: the body, or the sequence's first step), the
visit's attempts as typed `PanelAttempt` records in journal order (each with
its terminal kind, its `settled_members`, its `carried`, its `tree` and
`head` at both ends, and for a sequence the steps whose markers stand), the
head now, the tree now, and at drive time the source effects whose rebuilt
input no longer digests to its record. `Tag` is a newtype spelled once from
`(step, member)` by the function the markers use, with `bare()` for the
aggregate, so the plan, the journal and the view spell a member one way
(ruling 5).

The parked attempt is the latest of the visit's attempts, before the cut-off
(the command: every attempt; the drive: the attempts before this effect's
request), that bears one of three marks: `effect/succeeded`; a terminal fact
recording `tree`; the panel step's `sequence-step-finished` marker. Every
other attempt is passed over (S11).

`CarryPlan` holds one `Contribution` per declared tag, `Fresh` or
`Carried(SettledVerdict { tag, result, source: AttemptId })`, built by a
constructor that refuses a duplicate, a missing member or a tag outside the
panel; `fresh()` and `carried()` are its two views. `CarryRefusal` is one
`thiserror` enum, one variant per fact of S15, grouped where the facts are
siblings so each `why` is a closed enum matched exhaustively (ruling 2):

- `NoPanel { phase }`; `PanelStepFollows { step, first }`
- `UnknownMember { name, members }`; `MemberTwice { tag }`
- `Shape { attempt, why }` with `why` one of `NoAttemptEnded { visit }`,
  `Succeeded { park_reason }`, `EndedAtLaterStep { step }`, `NoRecord`,
  `RecordUnread { field, error }`
- `Unsettled { members }`
- `NoHead { repo }`; `HeadMoved { ends: Vec<RecordedHead>, now }`
- `TreeNotClean { observations: Vec<Observation>, now }`
- `InputMoved { effects }` (drive time only)

`RecordUnread` is the robustness position's distinction between missing and
malformed evidence: a `panel-carry.v1` field present on an attempt that does
not parse refuses naming the attempt and the field, before any fact is
asked, as a journal that does not fold refuses before anything is written.
The order of the facts, their texts and the lists each refusal names are the
spec's; `Display` is the one home of each text, printed by the verb and
written by the engine (D8). The handler's two argument refusals and S21's
text are not variants: the first are the CLI's, the second the fold's word.

### D5. The four identity facts

- **The head.** `git_head` of the repository `--repo` names at the command,
  and of `self.repo` at the drive. The reading must be forty lowercase hex
  digits, else `NoHead`; each recorded end of the parked attempt and of
  every source attempt is compared as a string, an absent or malformed end
  unequal, and every unequal end is named in the journal's order with the
  head now last. `phase/entered.head` and the gate-head observation are not
  read (S7).
- **The tree.** The reader's `clean` at those same ends, at the command and
  at the drive; every other observation named with its reading (S12).
- **The bundle.** `pinned_bundle` at the verb; `resume` at the drive. The
  engine does not ask it again: the bundle it runs is the one `resume`
  compiled under the gate.
- **The inputs.** At the drive only: for each source effect, `seat_input`
  rebuilt under that effect's own id, digested and compared with its
  `effect/requested.input_digest`, exactly as `execute` compares this
  effect's. The incident's park writes no `transition/decided`, so the
  rebuilt context is the source's and the digest holds; a rule-driven park
  after a succeeded attempt moves `last_decision`, but that attempt is
  refused as `Succeeded` before the inputs are asked. The verb builds no
  seat input: that needs an `Engine`, and the ruling that brought the
  compile to the verb stopped there.

### D6. The record

`carry/record.rs` holds `PanelRecord { tree: Ends<TreeReading>, head:
Option<Ends<Head>>, settled_members: Option<BTreeMap<Tag, Value>> }` and
`CarriedEntry { member: Tag, attempt_id }`, serialised with
`deny_unknown_fields` and read back by the same types. `PanelRecord::stamp`
merges the fields into the panel's `effect/indeterminate` or `effect/failed`
payload beside `reason`, `unresolved_members` and the start-failure fields,
which keep their bytes; `settled_members` holds each fresh member's
`AttemptOutcome::Succeeded { result }` value, the same object
`panel_outcome` feeds to the aggregate, and is absent when no member
settled. The sequence stamps the same record on its own terminal fact when a
panel step's outcome ended the attempt (S6), and nothing on any other
terminal fact. `carried` rides `effect/started` beside `provenance` and
`boundary`, absent when nothing is carried, so a plain retry's attempt is
byte-identical to today's.

`contracts/panel-carry.v1.schema.json` publishes the five fields (`tree`,
`head`, `settled_members`, `carried`, `args.members` for `command: "retry"`)
under the amended additive rule of `contracts/README.md`, registered in
`crates/brokkr-runtime/tests/frozen_contracts.rs` with its paragraph in the
README. The fold reads none of them. It is minted under 0093 and lands on
`main` only once 0093 is accepted (decision 0082 ruling 1; S13).

### D7. The carrying attempt

In `execute`, after the digest check: when the selection names members, the
facts are filled at the drive and the rule decides. A refusal becomes
`refuse_carry` (D8). A plan narrows the body once: `ExecutableBody::Panel`
over the fresh members, or a sequence whose first step is that narrowed
panel. `Dispatch` carries the narrowed body and the plan, so admission,
provisioning, `select_candidates`, the driver label, the session offers,
`provenance`, `boundary` and the member markers cover the fresh members and
no other, with no second rule. Before `effect/started` the attempt's start
readings (tree, head) are taken for every attempt whose body is a panel or a
sequence with a panel step, and held on `Dispatch`; the plan's entries are
written as `carried` on `started`.

`execute_panel` runs `member_runs` over the fresh members, journals their
markers, then joins in declared order: each `Contribution` is `Fresh`, taken
from the reports, or `Carried`, its result object; `panel_outcome` is the
same function over the joined pairs, so the result, `inputs` and `notes` are
a fresh panel's bytes (acceptance 1). The end readings are taken before the
terminal fact and stamped with `settled_members` when the outcome is short
of success. A fresh indeterminate parks the attempt and a fresh failure
fails it; the bounded retry reads the same acceptance and the same parked
attempt (D3), so it carries the same entries and runs the named members
again, never a verdict that settled in the failed attempt.

Crashes: after the acceptance and before the request, resume requests the
effect and reads the selection; after the request, resume rebuilds the
digest and the plan from the journal; after `effect/started`, the engine's
restart indeterminate carries no `tree`, so the attempt is passed over and
the next member retry carries from the attempt before it; after a partial
settlement, the fresh verdicts that settled are lost with that attempt,
fail-closed, and the named members run again.

### D8. A carry refused at drive time

The attempt starts on `driver: "none"` and ends `effect/indeterminate` with
`reason` equal to the refusal's `Display`, the sentence the verb would have
printed. The `close` arm reads it, as it reads every unmarked reason, to
`Stated("effect <id> indeterminate: <sentence>")`; `run/parked` carries that
text with `{}` evidence; every readout paints it as it paints every stated
park. Nothing is auto-retried (decision 0006 keeps an indeterminate for the
operator), nothing of the panel spawned, and the next member retry passes
the attempt over (S11).

Alternatives: S14's marker, codec and `ParkReason` variant (rejected: no
consumer, a public-API move in `brokkr-core`, a bytes/sentence split and a
timeline decode for a painter); running the whole panel (rejected: silent,
spends what the operator asked not to spend); a `run/parked` with no attempt
(rejected: a requested effect with no terminal fact).

### D9. Readouts

The scan reads `carried` where it reads `provenance` and `boundary`: each
entry ensures the member's `Build` under the carrying effect and sets
`carried: Some(entry)`. `participants` derives from it `status: "carried"`
with class `completed`, and `carried_from: Some(CarriedFrom { attempt_id,
ordinal })`, where `ordinal` is the source attempt's place among the phase's
attempts in journal order, every visit counted, so a painter leads with
"carried from review attempt 1" and keeps the id for linking. A carried
participant has no model, session, cost or turns of its own under the
carrying effect; its evidence stands under the effect it settled in.
`inspect`, `seats`, `watch`, the TUI, `ui.html`, the fleet readouts and the
console paint the participant (ruling 7); `render.rs` tones `carried` as
good; `VIEW_VERSION` moves from 23 to 24. The console offers no member
retry.

### D10. The working-tree reader

```text
pub(super) enum TreeReading { Clean, Dirty, Refused, Unrun }
pub(super) fn read_tree(repo: &Path) -> TreeReading
```

`Dirty` when git printed anything, whatever its exit; `Clean` when it exited
zero and printed nothing; `Refused` when it exited other than zero and
printed nothing; `Unrun` when it could not be run. `git_dirty` is
`matches!(read_tree(repo), Dirty | Unrun)`, today's boolean byte for byte,
so `ship_inputs`, the ship gate's payloads and the machine proofs over a
bare directory do not move. The carry and the record take the reading; the
carry needs `Clean`. The hole `git_dirty` keeps, a repository with a commit
whose index git cannot read shipping as clean, is older than this change and
is put to the operator as its own decision (open question 2).

### D11. Refusal texts, pinned once

`CarryRefusal`'s `Display` in `carry/tests.rs`; S20's text where today's is
pinned in `crates/brokkr-cli/src/tests.rs`; S21's in
`crates/brokkr-view/src/commands/tests.rs`; `resume`'s compile and
`ManifestMismatch` texts where they are pinned today and not again. Tests
match variants (ruling 8) and pin each text once.

### D12. Coordination

- **The number is 0093**, `Status: proposed`, written by this design as S19
  settled. This branch holds decisions up to 0090, so the index carries gap
  rows for 0091 (on `main`, #717) and 0092 (#802's branch) that leave when
  each lands; the index's gate refuses an unexplained gap and a gap row a
  file then takes. The operator may renumber at merge, a substitution over
  this change's artifacts alone.
- **#802's hunks** (S19): this change does not edit `engine/decide.rs`,
  `engine/tests.rs`, `0022`, the state diagram or the parks list; its
  `machine_proof.rs` line is `mod panel_carry;`; the index row is the one
  meeting.
- **The contract's landing** waits on 0093's acceptance (decision 0082
  ruling 1); the pull request says so.

### D13. Tests, and the mutation that proves each binds

Every new test lands in a new module (D1). Each names the compiling
mutation that must make it fail (ruling 9); the implementer records the
failing run and restores the behaviour.

`crates/brokkr-runtime/src/engine/carry/tests.rs`, over synthetic journals
built by a builder and a synthetic panel bundle:

- the plan over the incident's journal: three `Carried` by the parked
  attempt, one `Fresh`; mutation: the constructor accepting a plan with a
  member missing;
- the same aggregate as a full re-run, `result`, `inputs` and `notes` byte
  for byte, with a `residual` carried verdict kept as `residual`; mutation:
  joining the fresh reports alone;
- each `CarryRefusal` variant by its fact, in S15's order, with the lists
  each names (`HeadMoved` with two ends and now; `TreeNotClean` with two
  observations; `Unsettled` with two members in declared order;
  `InputMoved` with two effects once each); mutation per variant: the fact
  removed from the rule, and the order test's mutation: two facts swapped;
- the three-way test over every passed-over kind of S11 and every marked
  kind, the carrying attempt that ended at the chief included; mutation:
  the marker's mark dropped from the test;
- the read-through: the same member stuck twice carries by the first
  attempt; mutation: the entry's source replaced by the carrying attempt;
- `RecordUnread` on a `settled_members` that is not an object and on a
  `tree` with a fifth word; mutation: the parse falling back to "no
  record";
- the scope: an acceptance before an earlier effect's request selects
  nothing for a later effect; mutation: the "no request between" condition
  dropped;
- `RetryArgs` with `deny_unknown_fields`; mutation: the attribute removed.

`crates/brokkr-runtime/src/engine/worktree/tests.rs`: the four readings over
a clean repository, an edited one, a repository whose index is unreadable
and a missing directory, and `git_dirty`'s `false`, `true`, `false`, `true`
over them; mutation: `Refused` folded into `Dirty`. The existing `git_dirty`
test in `engine/tests.rs` (lines 2043 to 2058) holds unchanged.

`crates/brokkr-runtime/src/engine/operator/retry/tests.rs` (the verb, with
the fake driver): a member retry journals `args.members` resolved and in
order; a fold refusal writes nothing and prints S21's text with each of the
three words; a carry refusal writes nothing; every member named carries
nothing; the lost fence after the write is journaled as today; mutation:
the fold step skipped (the race refusal appears where S21's text should).

`crates/brokkr-runtime/src/engine/carry/drive_tests.rs` (dispatch, with the
fake driver): only the named members spawn, counted; `provenance`,
`boundary`, the driver label, one offer, one admission claim and one marker
cover the fresh member alone; the start and end readings on the terminal
fact; a head moved between acceptance and drive parks with the sentence as
the stated reason and `run/parked` carries it; a moved house-rules file
parks naming the source effect; the bounded retry after a start failure
carries the same entries; a verdict settled in the failed bounded attempt is
not carried; result files deleted after the park change nothing; a journal
written before this change refuses `NoAttemptEnded`; the sequence's first
step carries under `positions:` tags and `chief` runs on the aggregate; the
restart indeterminate carries no record and is passed over. Mutations: the
drive-time check removed (the moved head is carried); the narrowing removed
(four spawns); the end reading removed (`tree` absent).

`crates/brokkr-runtime/tests/frozen_contracts.rs`: the new schema's
registration and bytes; `crates/brokkr-core/src/fold/tests.rs`: a journal
with every new field folds to the state it folds to without them, byte for
byte (the contracts README's machine-checked clause).

`crates/brokkr-cli/src/tests.rs`: the two argument refusals before every
refusal the handler makes today, and S20's text on `retry` with and without
`--member`, today's on `stop`; `crates/brokkr-cli/tests/machine_proof/
panel_carry.rs`: the three acceptance lines end to end over a committed
workdir, the plain retry's `effect/started` byte-identical to today's, and
`inspect` and `seats` naming the carried participants with their ordinal.

`crates/brokkr-view`: in the new module's tests, the scan over `carried`,
the participant's status, class and `carried_from`, no model or cost of its
own, the ordinal across two effects, and a plain retry's view unchanged;
`commands/tests.rs` pins S21's text; the `VIEW_VERSION` pin moves.

## Risks

- **Edits inside an attempt's span, reverted before its end, and edits to
  ignored files** are unobserved (S7, S12): the residual the gate-head
  observation and the ship gate already accept. Named, not closed.
- **A head moved and put back inside one attempt's span** is unobserved for
  the same reason. A head moved during a park and put back is refused by
  the recorded ends (S7).
- **The whole-panel input digest** refuses every carry when any member's
  input moved (D5). Inside one run that needs a bundle drift, which the
  manifest gate already refuses, or an edit to a file the seat input embeds,
  which should refuse. Accepted.
- **A partial settlement lost to a restart** (D7) costs the named members a
  second run. Fail-closed; the attempt before is read.
- **The fence's window.** The facts are asked before the fence, the fence
  refuses a run that moved between, and the drive asks them again: no
  window admits a carry the facts do not hold for.
- **A record the engine never writes** (`RecordUnread`) refuses rather than
  carries; the plain retry stands.
- **`engine/sequence.rs` at 714 lines** gains the stamp and the first-step
  plan, within its ceiling; if the stamp's threading grows it past, the
  stamp lives in `carry/record.rs` and the sequence calls it.
- **The decisions index** needs its two gap rows removed at each landing;
  the index's gate catches a row left behind.

## Migration

- Journals written before this change: an attempt that bears none of the
  three marks is passed over and a visit of them refuses `NoAttemptEnded`;
  the plain retry is unchanged, so no operator action is needed.
- A reader before this change sees unknown optional fields on
  `effect/started`, `effect/indeterminate` and `effect/failed` and ignores
  them, as it ignores `provenance` and `boundary`; the frozen schemas are
  not edited, and `panel-carry.v1` is additive.
- `VIEW_VERSION` 24: the console and `ui.html` read the new participant
  field; an older plugin paints a carried participant by its status word.
- `docs/guides/quickstart.md`'s operator commands and `ARCHITECTURE.md`'s
  panel and operator-verb prose name the member form; `docs/reference/
  cli.md` is regenerated from clap.
- The contract lands on `main` with 0093's acceptance, or the operator
  accepts at the merge (S13).

## Open questions

1. **The number.** 0093 by the commission's rule; the operator confirms it
   or assigns the next free number at merge.
2. **The ship gate's `refused` reading.** A repository with a commit whose
   index git cannot read ships as clean today and after this change. The
   reader now tells it apart; ruling that the ship gate stops on it is a
   decision of its own, which this design asks the operator to open.
3. **A member retry from the console.** Not offered; the view's commands
   are unchanged. Whether the console's Retry should name members is the
   console's own later slice.
