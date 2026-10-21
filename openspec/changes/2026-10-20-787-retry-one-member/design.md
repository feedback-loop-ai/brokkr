# Design — a retry by member re-runs the named members and carries the rest (#787)

Decision 0093 carries the rulings. This design carries what the code does,
the evidence behind each choice, and the council's positions reconciled.
The proposal's record (S1 to S31) holds the clarify history, and the lists,
orders and texts a refusal or a record carries are the specs' own
(`panel-member-retry` R1 to R5, `panel-attempt-record` A1 to A4,
`working-tree-reading` W1 and W2), stated there once and pinned once in the
tests D13 names. A decision is stated once here and cited from there; a
record of the proposal is cited only where this design reversed it.

## Context

On 2026-10-19 one reviewer of a four-member panel ran 141 turns and wrote no
verdict; `panel_outcome` parked the attempt and the plain retry re-ran all
four members, about forty minutes for one stuck seat. The proposal cleared
clarify with four operator rulings (the head at both ends of every panel
attempt; the pinned bundle compiled under `resume`'s manifest gate; the
three-way test deciding the parked attempt; the reader's tests in a new
module) and the operator's assistant's settlements under the standing ruling
for epic #790. The council sat three times, robustness and simplicity each
time, both run-local: at S22, after clarify's sixth pass cleared the record
(S27), and after tasks returned the change upstream (S28). This design adopts,
rejects or combines each claim below, on the code as it stands.

The pieces the design builds on, as read on this branch:

- `execute_panel` (`crates/brokkr-runtime/src/engine/panel.rs:37`) derives
  one `MemberRun` per member (`member_runs`, `:117`), runs them, journals one
  `panel-member-finished` marker each (`:275`) and joins the reports by
  `panel_outcome` (`:448`), a pure function over the members'
  `settled_outcome()`s.
- `member_runs` builds a member's input from the seat input and nothing the
  journal does not already hold: `feature`, `phase`, `workdir`,
  `allowed_results`, `house_rules` and `context` copied from the seat input;
  `seat` from the seat and member names; `role_path` and `result_path` from
  the seat input's own `members` entry; the dialect prose from the bundle's
  `dialect_prompts` or the seat input (`marks.rs:239`); the secret-binding
  facts copied from the seat input; the site marks, the launch arguments and
  the serving facts from the pinned bundle's site facts and the selected
  candidate (`marks.rs:44`, `engine.rs:1222`); and `resume_context` from the
  member's own resume plan (`resume.rs:720`: the assessment, the originating
  harness, the owned session target).
- `seat_input` (`engine/input.rs:49`) is `Engine`'s: "a pure function of
  (journal, pinned bundle, feature)" over `self.bundle`, `self.repo`,
  `self.world` and the run-local results directory, which `execute`
  (`engine/dispatch.rs:41`) rebuilds for this effect and refuses when its
  digest moved (`refuse_changed_input`, `:131`: an attempt on `driver:
  "none"` with a terminal fact that says why), before it plans the attempt
  and appends `effect/started`.
- `launch::resume` (`crates/brokkr-runtime/src/launch.rs:255`) builds an
  `Engine` and drives nothing: it reads the folder's state, opens the store,
  compiles the pinned bundle (`compile_from_manifest`, refused through
  `unreproducible`), refuses a host that cannot box it, and calls
  `Engine::resume` (`engine.rs:680`), which refuses an unbuilt bundle,
  compares the manifest to the pin (`ManifestMismatch`), verifies a Looper
  dispatch's bounds, rehydrates the world from the manifest, folds the
  journal through `replay.caught_up` and binds the charters as `run/started`
  recorded them (`charters_as_started`, `bundle.rs:4177`); then it fences
  the crossings (decision 0057). None of it writes the journal or spawns a
  process.
- `disposed` (`engine/operator.rs:364`) folds the run, writes
  `operator/commanded` with `"args": {}`, re-folds and fences the acceptance
  on that fold's head, re-deciding on a moved head up to `FENCE_ATTEMPTS`
  and refusing `LostFence` after; `standing_refusal` (`:330`) refuses
  `LostFence` whenever the fold's conditions have come to hold or another
  command took the pending place. `operator_stop_reason` (`:120`) is the
  precedent for the engine finding an accepted command by its acceptance's
  `command_id`; it reads no `args`.
- `Engine::append` (`engine.rs:1023`) runs `finish_gate_head_check`
  (`:1044`) before any terminal fact and, when the gate's head moved, writes
  the `GATE-MOVED-HEAD` indeterminate in the terminal fact's place; the
  sequence does the same at each step's end (`sequence.rs:254`).
- The fold's `close` arm (`crates/brokkr-core/src/fold.rs:629`) reads an
  unmarked indeterminate reason to
  `ParkReason::Stated("effect <id> indeterminate: <detail>")`, and the `Park`
  cursor writes `run/parked` with that text and `{}` evidence
  (`engine.rs:1003`). Every readout paints a stated reason today.
- `git_dirty` (`engine.rs:3458`) reads only what `git status --porcelain`
  printed, with a failed spawn as dirty. `git_head` (`:3446`) reads
  `rev-parse HEAD` and answers nothing when git fails.
- `SiteKey::tag` (`bundle/site_key.rs:175`) spells a member once,
  `<member>` in a seat's panel and `<step>:<member>` in a panel step; every
  per-member record of an attempt keys by it, and the view's participant
  label is `<seat>:<tag>` (`brokkr-view/src/lib.rs:960`).
- The view's scan (`crates/brokkr-view/src/lib.rs:1036`) reads
  `effect/started.provenance` and `.boundary` per site tag and the member
  marker's outcome into one `Build` per `effect:member`, from which
  `participants` (`:1774`) derives each `Participant`.
  `select_transcript_participant` (`brokkr-cli/src/lib.rs:1029`) selects by
  exact key, else by a unique label, and lists every key of an ambiguous
  label, participants without a transcript included.
- `Selection` (`engine.rs:1830`) is "everything this attempt decided per
  site before anything spawned", built by `select_candidates` inside
  `plan_attempt` (`dispatch.rs:199`), handed to `execute_panel` and
  `execute_sequence` by `run_body` (`:255`), held by `SequenceRun` and read
  by `panel_step` through `run.selection`; its `Default` is
  `Selection::new()`, which every direct test caller passes, and nothing
  builds it by a struct literal.
- The direct test callers, by function: `execute_panel` at twelve sites in
  six files (`engine/tests.rs` 2107, 2140, 2168, 3607, 3629, 3651, 6073;
  `boundary_tests.rs` 3485; `checkpoints/tests.rs` 100;
  `contention_tests.rs` 1065; `capability_tests.rs` 1778;
  `capability_tests/call_tests.rs` 419), each spelling four scalar
  arguments on four lines; `panel_outcome` at five sites in three
  (`engine/tests.rs` 1975, 1979, 1983; `boundary_tests.rs` 3935;
  `cleanup_tests.rs` 159); `execute_sequence` at thirty-one in five
  (`engine/tests.rs` twenty-four, `boundary_tests.rs` two,
  `capability_tests.rs` three, `agent_tests.rs` one,
  `boundary_tests/sealed_tests/executable_tests.rs` one). `engine/tests.rs`
  (6,556) and `boundary_tests.rs` (4,207) stand at their
  `quality/file-lines.txt` baselines and may not grow; the ratchet
  (`quality/ratchet.sh`, `files`) lets a file grow to the higher of its
  ceiling and its baseline, so `brokkr-view/src/lib.rs` (2,716) may reach
  2,789. `clippy.toml` sets `too-many-arguments-threshold = 7` and clippy
  counts `self`, which is why `execute_sequence`, seven parameters beside
  `self`, carries its expectation (`sequence.rs:184`).
- #802 (`origin/recovery/789-environment-blocks-park`, measured
  2026-10-21): in `crates/brokkr-runtime` it edits `engine/decide.rs` and
  `engine/tests.rs` alone, the latter in one hunk at lines 2347 to 2380 of
  `decide_covers_schema_no_rule_review_head_and_ship_drift`; it does not
  touch `engine.rs`, `dispatch.rs`, `panel.rs` or `sequence.rs`.

## Goals and non-goals

Goals, the issue's three acceptance lines and the proposal's rulings:

1. `brokkr operator retry --member <tag>` (repeatable) on a parked panel
   runs the named members only, and the attempt's aggregate is the bytes a
   full re-run produces over the same verdicts.
2. A verdict is carried only when the reviewed head, the working tree, the
   pinned bundle and the member's inputs are what it judged; all four facts
   are asked at the command, which refuses by name and writes nothing, and
   again at the drive, which fails closed.
3. Every carried verdict is named in the journal, typed, and in every
   readout that paints participants.

Non-goals, each named with its reason:

- The ship gate's reading of the working tree does not change (D10). Both
  council positions refused the proposal's tightening as a second feature
  under one decision; the hole it would have closed is older than this
  change and goes to a decision of its own.
- No step-level carry, no carry of a panel step after another step, no carry
  across phase visits (D4).
- The Looper bridge and the console keep the plain retry.
- No per-member judged-input digest (D5): the seat input's digest, the
  manifest gate, the charter binding and the head and tree already hold what
  a member judged, through the one function that builds every member's
  input.
- No second spelling of a member's name (D4): the tag is the one the journal
  and every readout use.
- No change to the transcript verb's selection (D9).
- No typed park reason for a refused carry (D8).
- The fold is not edited: no event type, no cursor, no arm, no field it
  reads (decision 0050 ruling 6).
- No field on `Engine` and no eighth parameter on `execute_sequence`: the
  plan rides the `Selection` every body function already takes, and the
  panel observes its own span (D1, D7).
- No new seam: this change adds no trait. `resume_with` is a `*_with`
  function the house uses to invert one dependency (decision 0071 ruling 2),
  and the engine is the one observer of the carry's facts.

## The council, reconciled

Each claim is adopted, rejected or combined on evidence, with the ruling of
decision 0071 it turns on. The first visit's table is kept short, since its
reasons stand in the decisions below; the second visit's is in full.

### The first visit (S22)

Adopted from both positions: one pure rule over typed facts (D4); the
drive-time recheck (D7), since a head moved between the command and the
drive would otherwise be carried, a fail-open on the review gate rated high;
a complete, typed plan with one entry per declared member (D4); only a
settled success as carry evidence, its whole result value, never a run-local
file (D6); the tree at both ends and at both times (D5); the selection's
causal scope (D3); a first-step panel only (D4); the read-through of
`carried` (D4); the records parsed once at the edge, a present field that
does not read refusing by name (D4); and the ship gate's semantics kept
(D10), with the operator's fourth ruling honoured by the reader's tests in a
new module.

Adopted from simplicity alone: no marker, codec or `ParkReason` variant for
a refused carry (D8); every member named as the stated short-circuit (D4);
`resume`'s argument defaulting reused (D2); each decision stated once.

Rejected from simplicity: falling through to the plain path when the fold
refuses (D2: a run that parks between a read-only fold and the fenced one
would be accepted with no carry check); one text for the four shape facts
(D4: the operator's remedy differs by fact); `tree` as a boolean (D10: the
reading is the fact the refusal names); one capability instead of three
(each archives as a living spec of its own).

Rejected from robustness: `effect_id` beside `attempt_id` on a `carried`
entry (a second home, ruling 5); decision number 0092 (D12).

Combined: a carried participant's origin derived apart from its status (D9);
the crash scenarios answered one by one (D7, D13).

### The second visit (S27)

From robustness:

- **The identity proof, and the inputs fact at the command: adopted, by
  making the engine the one observer** (D2, D5). Robustness held that D5
  proved the source reconstructible, not the destination member's input
  equal to it, and that the command refused nothing of the inputs. The first
  half is answered by writing the derivation (D5): every fact `member_runs`
  adds to the seat input comes from the pinned bundle, the effect id or the
  serving candidate, and the seat input is built by one function over the
  same context for the source effect and the carrying one, so the content
  judged is equal when the source's digest holds; a second projection with
  `result_path` struck out by hand would be a second definition of the input
  (ruling 5), and the per-member digest stays refused. The second half is
  adopted: `launch::resume` builds the engine without driving or writing
  anything, so the verb builds it as `resume` does and asks the inputs fact
  through the same `seat_input` the drive asks it with. One observer at two
  times replaces the two observers of S22 (ruling 5), and the planned
  `launch::pinned_bundle` is not written. The cost is `Engine::resume`'s
  admission at the command, which refuses there what `brokkr resume` would
  refuse at the drive, in `resume`'s words: intended.
- **The acceptance bound to the fold it was validated against: adopted as
  the smaller structure** (D2). The fence already closes the window:
  `standing_refusal` refuses `LostFence` when the run moved so that the
  fold's conditions hold or another command took the pending place, and an
  attempt record can change only through an acceptance and a drive, which
  that catches; the head, the tree and the inputs are asked again at the
  drive; the acceptance carries the tags alone. So no stale proof reaches
  the journal. The member form now hands `disposed` its own fold, so the run
  is folded once and the one refusal after the write is the race's own word,
  `lost_fence`. A typed race refusal of the carry's own was not needed: the
  fold's word is the house's (ruling 6).
- **A complete plan that refuses a duplicate report: adopted** (D7). The
  join refuses a report twice for one tag as it refuses a report for no
  `Fresh` entry: `EngineError::Invariant`.
- **`Value` in `settled_members` named as a deliberate exception: adopted**
  (D6). The result is the recipe's, opaque to the engine, the same `Value`
  `AttemptOutcome::Succeeded { result }` holds today, typed by the aggregate
  at its own edge; every other field of the record is typed.
- **No settlement path is bypassed: answered with evidence** (D7). The carry
  reads only attempts whose panel ended, each member through
  `settled_outcome`, which reads `refused` and `cleanup`; an attempt in
  flight at a restart is settled by the engine's own `EffectInFlight` arm
  as today, records no `tree`, and is passed over. The carry adds no path by
  which a process is proven over.
- **Decision number 0092: rejected** (D12). Verified on 2026-10-21 against
  the remote refs.
- **The binding cases: adopted into D13**: the inputs refused at the
  command; the carrying effect's own result paths leaving the source's
  digest whole; the race before the disposition; the duplicate report;
  both `Aggregate` kinds; the rest were already there.

From simplicity:

- **One spelling for a member, the tag: adopted** (D4; rulings 5 and 6).
  `SiteKey::tag` spells a member once, every journal record keys by it and
  the participant label prints it, so a bare name on a sequence panel was a
  second spelling whose only consumer was convenience, and whose collision
  with a tag needed a fact of its own (C8) to fail closed. With the tag the
  only spelling an exact match cannot read as two members, since member keys
  are unique within a panel and the step prefix is fixed: `Names`, the
  resolution step, `AmbiguousMember` and their tests go, and the names are
  two facts. C8's settlement stands in effect. The cost accepted: `--member
  correctness` on the shipped review sequence is refused once, naming the
  four `positions:` tags, and the operator retypes what `inspect` prints.
- **The transcript verb's skip: adopted** (D9; ruling 6).
  `select_transcript_participant` lists every participant of an ambiguous
  label today, those with no transcript included, an attempt refused
  admission among them, and refuses `no-reference` when such a key is
  chosen. Skipping a carried participant alone would be a second rule for
  one kind of participant without a transcript; the general cut is a change
  of its own. The skip, its new CLI test module and its spec clause go, and
  the readout stays truthful.
- **Cite the spec, do not restate it: adopted** (this file). D4 and D9 name
  the facts and point at the spec for the lists, orders and texts, and the
  clarify tags are gone from the decisions except where one was reversed.
- **`CarriedFrom` by the ordinal alone: rejected** (D9). The painters lead
  with the ordinal, and the id is the journal's one name for the attempt,
  the key `inspect --json` and the console's data join on; an ordinal names
  nothing in the journal. The id's consumer is that join.
### The third visit (tasks returned, 2026-10-21)

The tasks seat returned the change: D1 and D7 changed two signatures that
existing tests call directly, while D12, D13 and task 1.2 said
`engine/tests.rs` is not edited, so task 5.1 and task 1.2's verify step
could not both hold. It offered in-place edits at the named call sites (a)
or keeping both signatures with the plan reaching the panel some other way
(b), asked D1 to list every test file a signature change touches, and asked
for one figure for `brokkr-view/src/lib.rs`. Both positions sat again on
that finding. Each claim, with the ruling it turns on:

- **The finding: adopted, as (a) narrowed by (b)** (D1, D7, D12, D13;
  rulings 4 and 6). The plan reaches the panel without any new signature
  (below), so `execute_sequence`, `SequenceRun` and `panel_outcome` keep
  theirs and their thirty-six direct callers are not touched;
  `execute_panel` must change because its body changes (the join and the
  record), and ruling 4 then holds a changed function to seven parameters,
  so its twelve callers are edited in place and shrink. D1 lists them; D12
  restricts the edit to those hunks, none in #802's; the figure for
  `lib.rs` is the ratchet's 2,789, since the gate is the authority.
- **Simplicity: the plan's home during the drive is `Selection`: adopted**
  (D7; rulings 5 and 6). `Selection` is already "everything this attempt
  decided per site before anything spawned", it already reaches
  `execute_panel`, `execute_sequence` through `SequenceRun` and
  `panel_step` through `run.selection`, and its `Default` is the plain
  attempt. One optional field gives the plan one home that every consumer
  already holds; `Dispatch` does not also hold it. The eighth parameter
  task 5.3's wording would have forced on `execute_sequence`, a function
  already carrying its expectation, with thirty-one test callers, is not
  written.
- **Simplicity: `panel_outcome` keeps its signature and the rule moves
  below it: adopted** (D7; rulings 5 and 6). Today's three arms become
  `outcome_over` over `(name, AttemptOutcome)` pairs, the one rule;
  `panel_outcome(aggregate, reports)` is that rule over the reports'
  settled outcomes, the plain path's, with its five direct callers
  unchanged; a carrying attempt reaches the same rule through
  `CarryPlan::join`, which yields the pairs. The `Joined` type and its two
  views go: a `Vec` of pairs and one free function over the fresh reports
  (`settled_members`) serve both callers, and a plain attempt does not
  join at all, so the argument that it "joins under `CarryPlan::fresh` and
  keeps its bytes" and its mutation test go with it. The two paths end in
  one function, and the acceptance's first test compares them byte for
  byte (D13).
- **Simplicity: `execute_panel` reaches six parameters beside `self`, by
  `PanelAt`: adopted** (D1; rulings 4 and 10). D1's "nine become seven"
  did not clear the lint, since clippy counts `self`; `PanelAt {
  effect_id, attempt_id, seat_name, gate }` with a positional constructor,
  the `StepAt` precedent, brings it to six beside `self`, and the
  expectation goes. Each of the twelve test callers folds four argument
  lines into one, so both baseline files shrink. The constructor
  production calls is the builder ruling 9 asks for; no test-support
  module is added.
- **Robustness: choose (a), with the inventory in D1 and the existing
  assertions kept: adopted** (D1, D13; ruling 9). The inventory is D1's;
  the assertions do not move; the coordination with #802 is a restriction
  on overlapping hunks, not on the file.
- **Robustness: an all-fresh builder in the existing test-support home:
  rejected as unneeded** (ruling 6). `Selection::new()` is already the
  plain attempt, so no existing test builds a plan.
- **Robustness: the checked join's output the only input to the changed
  `panel_outcome`: combined** (D7). The one rule, `outcome_over`, takes
  pairs; on a carrying attempt those pairs come from the checked join
  alone, on a plain attempt from the reports' settled outcomes, since
  there is no plan to check them against. The narrowed body and the join
  read one plan, so a filtered panel cannot reach the rule without its
  carried entries.
- **Robustness: the post-digest identity written as a classification, with
  a binding scenario: adopted** (D5, D13; spec R3). D5 now names which
  facts `member_runs` adds after the digest, why none of them can change
  the reviewed subject, and where the source's serving facts stay bound;
  the spec gains the scenario `A chain fallback between the attempts`, and
  D13 its test.
- **Robustness: clear a carried entry at a later `effect/started` of the
  same effect, and test the scan across successive attempts: adopted**
  (D9, D13; ruling 7). The scan clears `boundary` and `stamp` at every
  start today; `carried` is cleared with them.
- **The start readings' home, which neither position settled** (D7;
  rulings 5 and 10). Simplicity kept them on `Dispatch`, which reaches
  `execute_panel` through `PanelAt` but cannot reach the sequence's
  terminal writer without the parameter it refused; robustness did not
  address them; an `Engine` field is the `active_gate_head` precedent,
  mutable state that needs clearing and that `engine.rs` has no room for.
  Settled: the panel observes its own span, `execute_panel` and
  `panel_step` reading the tree and the head at their top, before
  `run_panel` spawns a member, and before their terminal fact; the
  sequence's panel step hands its opened span and its settled verdicts to
  the terminal writer on `StepRun`, the struct it already returns. This is
  the house's shape for a sequence step's gate-head observation, armed in
  `sequence_step` at the step's own start. `Dispatch` is not edited. A1,
  ruling 4 of 0093 and the proposal's "the attempt's start" are made
  precise: the panel's start, before any member of it spawned, which for a
  seat's panel and a sequence's first step is the attempt's start, and for
  a later panel step, never carried, the step's own.


### The analyze pass after the third visit (2026-10-21)

The analyze judge measured the record after S28 against the code: the
deterministic check was clean, and six findings came back, the earliest in
the proposal, with one note. The proposal's S29 answers each; this design
takes them where they bind:

- **The secrets store (F1, medium): adopted** (D1, D2, D5, D11, D13,
  Risks). The member form took no `--secrets-file`, yet `seat_input` embeds
  the operator-side store's path in the input of a seat that declares
  secrets (decision 0012; `engine/input.rs:83-85`), and the digest covers
  it; a run driven with a store would have been refused `InputMoved` at the
  command while the drive carried. The member form takes `--secrets-file`
  as `resume` takes it, so the engine at the command is built from what the
  engine at the drive is built from, which is what one observer at two
  times means (ruling 5).
- **The head's shape has one home (F2, low): adopted** (D5, D6). `Head`
  parses by `is_commit_id`, not by a second predicate.
- **The view's functions over the ceiling (F3, low): adopted** (D1, D9,
  D13). The helpers are named, in `started.rs`, so `scan_participants`
  shrinks and `participants` gains no branch; the sentence a carried
  participant shows is derived once, as its activity cell (ruling 7).
- **`disposed`'s two folds (F4, low): reworded** (tasks 7.2). Only the
  deciding fold before the write moves to the caller; the fence loop folds
  as today.
- **A figure (F5, low): corrected** (tasks 9.1).
- **A mixed panel does not compile (F6, low): corrected** (proposal S7, spec
  R3's scenario). `refuse_mixed_panel` refuses it; an all-work panel alone
  is unobserved by the gate-head observation.
- **The index fixture (note): adopted** (D13, tasks 1.3, three specs). The
  fixture corrupts the index rather than changing its mode, which a box
  running as root reads regardless.
- **Also made coherent:** D11, D13 and decision 0093 ruling 1's binding name
  the handler's test module as S20's pin, where tasks T5 moved it.

## Decisions

### D1. Where the code lands, against the ceilings

The ceilings of decision 0071 ruling 4 fix the modules. `engine.rs` (3,523
lines, at its baseline), `engine/operator.rs` (800), `engine/tests.rs`
(6,556, at its baseline), `boundary_tests.rs` (4,207, at its baseline) and
`crates/brokkr-cli/tests/machine_proof.rs` (3,449 of 3,451) may not grow;
`brokkr-view/src/lib.rs` (2,716) may reach its 2,789 baseline, the one
figure this change uses for it; `cli_args.rs` has seven lines of headroom.
Clippy's `too_many_arguments` counts `self` against its threshold of seven.

- `engine.rs`: `Selection` gains `carry: Option<carry::CarryPlan>`, one
  field with its doc line, `None` by `Default`, so every existing caller of
  `Selection::new()` runs the plain attempt (D7); `git_head` and
  `git_dirty` move into `worktree.rs` whole, `git_dirty` keeping its name
  and its boolean and becoming one `matches!` over the reading (D10), with
  `pub use worktree::git_head;` keeping the one public reader `brokkr
  realms` reads (ruling 5) and `use worktree::git_dirty;` for the ship
  gate's caller; `mod worktree;` and `mod carry;` are added. The two
  functions' lines pay for the lines gained, so the file shrinks (task
  1.2). No field is added to `Engine`.
- `engine/worktree.rs`, with `engine/worktree/tests.rs`: the one reader
  (D10).
- `engine/carry.rs`, with `carry/record.rs`, `carry/facts.rs` and
  `carry/tests.rs`: the rule and its typed refusal (D4); `CarryPlan` and
  its constructor; `CarryPlan::join`, the one join of carried and fresh
  verdicts, yielding the pairs the rule takes; `outcome_under`, the one
  function both panel callers use (D7); the `panel-carry.v1` types that
  both the writer and the reader use, and `settled_members`, the free
  function over the fresh reports that fills the record (D6); and
  `Engine::carry_facts`, the one observer (D4), which fills the facts from
  the journal's typed records, the body the engine would run, git and the
  engine's own `seat_input`, raising `RecordUnread` for a record that does
  not read. Observing is apart from deciding (ruling 10), and the observer
  is one method called at two times (ruling 5).
- `engine/panel.rs`: `PanelAt<'a> { effect_id, attempt_id, seat_name, gate
  }` with a positional `PanelAt::new`, the `StepAt` precedent (ruling 10:
  a long list becomes a parameter object); `execute_panel(&mut self, at:
  &PanelAt<'_>, members, aggregate, panel_input, deadline, selection)`,
  six parameters beside `self`, its `too_many_arguments` expectation
  removed; `run_body` builds the `PanelAt` from its `Dispatch`.
  `panel_outcome(aggregate, reports)` keeps its signature; today's three
  arms move below it into `outcome_over(aggregate, pairs)`, the one rule
  (D7). `execute_panel` reads its span's ends and stamps the record (D6,
  D7); `member_runs` is not edited.
- `engine/sequence.rs`: `panel_step` reaches the outcome by
  `outcome_under` with the plan `run.selection.carry` holds for the first
  step and `None` for any later panel step; `StepRun` gains the panel
  step's opened span and settled verdicts, which `step_result` stamps on
  the terminal fact when the panel step's outcome ends the attempt (D7).
  `execute_sequence`, its expectation and `SequenceRun` are not edited.
- `engine/dispatch.rs`: the drive-time check (D7) before `effect/started`;
  the body narrowed once at the seat's clone; `plan_attempt` takes the
  plan, sets it on the `Selection` it returns and writes the `carried`
  entries on `started` (D7). `Dispatch` is not edited.
- `engine/operator/retry.rs`: `Engine::retry_members`, the verb's member
  form (D2), returning the typed `MemberRetryOutcome`, beside
  `conclusion.rs`, `receipt.rs` and `supersede.rs`; `disposed` in
  `operator.rs` takes a `Commanded` parameter object and the deciding fold
  (ruling 10: a long list becomes a parameter object), its own line count
  unchanged within the file's ceiling.
- `launch.rs`: `resume` splits into the store's open and
  `resume_with(store, request, run)`, every gate of today's `resume` over a
  store the caller opened; `operator_retry_members(request, run, names,
  operator, reason)` opens the store, folds, reads the records, calls
  `resume_with` and the engine's `retry_members` (D2). `Engine::resume` and
  `engine.rs` are not edited for it, and no gate is written twice
  (ruling 5).
- `crates/brokkr-cli/src/cli_args/member_retry.rs`: the member form's
  arguments, `--member`, `--repo` and the `DeliveryArgs` `resume` reads
  (`--bundle`, `--recipe`, `--recipes-dir`, `--secrets-file`) flattened
  whole, the one home of the four (ruling 5), the struct flattened into
  `OperatorArgs` by one line; `verbs/delivery/member_tests.rs`: the
  handler's tests (D13; tasks T5).
- `crates/brokkr-cli/tests/machine_proof/panel_carry.rs`: the acceptance
  proofs, over a workdir the proof `git init`s, with `mod panel_carry;`
  after `mod output_edge;`.
- `crates/brokkr-view/src/started.rs`, a child module of `lib.rs`: the
  per-site readers of `effect/started` and what they decide (D9).
  `boundary_entries` and `BoundaryEntry` move there from `lib.rs`;
  `carried_entries`, `sites`, `status_of`, `carried_from` and `activity` are
  added beside them, and the scan's `EffectStarted` arm and its status loop
  shrink onto the first two, so `lib.rs` shrinks by more than the lines
  `Build`, `Participant` and `participants` gain and stays under 2,789
  either way, and `scan_participants` and `participants`, over ruling 4's
  ceiling, gain no branch.
- `crates/brokkr-core`: nothing. `crates/brokkr-cli/src/lib.rs`: nothing.

The existing tests a signature change touches, every one edited in place
with its assertions unchanged (ruling 9), each `execute_panel` call folding
its four scalar arguments into `&PanelAt::new(effect, attempt, seat, gate)`
so the call is three lines shorter and the file shrinks:

| File | `execute_panel` calls | In #802's hunk? |
|---|---|---|
| `engine/tests.rs` | 2107, 2140, 2168, 3607, 3629, 3651, 6073 | no; its one hunk is 2347 to 2380 |
| `boundary_tests.rs` | 3485 | no; #802 does not touch the file |
| `checkpoints/tests.rs` | 100 | no |
| `contention_tests.rs` | 1065 | no |
| `capability_tests.rs` | 1778 | no |
| `capability_tests/call_tests.rs` | 419 | no |

`panel_outcome`'s five callers (`engine/tests.rs` 1975, 1979, 1983;
`boundary_tests.rs` 3935; `cleanup_tests.rs` 159) and `execute_sequence`'s
thirty-one are not edited: neither signature moves. Every new test lands in
a new module (D13).

### D2. The verb

`brokkr operator retry --member <tag>` is the plain verb with a member form.
The handler (`verbs/delivery.rs:167`) asks the two argument facts first,
before its `supersede` branch: `--member` with any command word but `retry`;
any of `--repo`, `--bundle`, `--recipe`, `--recipes-dir` and `--secrets-file`
without `--member`.
Then today's refusals in today's order, with S20's one text on `retry`. With
members typed, the handler builds a `LaunchRequest` through `request`
(`verbs/delivery.rs:79`, the one `resume` uses) from every argument
`resume`'s request is built from, `--secrets-file` read and defaulted as
`resume` reads it (D5), and calls `launch::operator_retry_members`, which
runs in R4's order:

1. Open the store and fold the run once: `decided` over the loaded events,
   the deciding fold, which also carries the account the verb's writes
   need (decision 0089 ruling 3). `acceptance_refusal`'s word is returned
   as `MemberRetryOutcome::FoldRefused(Refusal)` and nothing is written
   (decision 0047 ruling 2); the CLI handler prints
   `brokkr_view::member_retry_refusal(word)`.
2. Read the parked phase's visit into typed records (`carry/facts.rs`; D4,
   D6), whatever names are given, every member named included. A
   `panel-carry.v1` field that does not read is
   `CarryRefusal::RecordUnread`, returned as
   `MemberRetryOutcome::CarryRefused` before the engine is built and before
   any name is matched, so a drifted bundle beside a malformed record
   reports the record, and an all-named retry beside a malformed record is
   refused for it (R4).
3. Build the engine through `launch::resume_with(store, request, run)`,
   `resume`'s own admission over the store of step 1: a refusal is
   `resume`'s own `LaunchError`, the verb's `Err`, printed as `resume`
   prints it. The engine spawns nothing and writes nothing here.
4. Observe the rest of the facts through the engine (`Engine::carry_facts`,
   D4): the shape of the body it would run for the parked phase, the head
   and the tree of its repository, and each attempt's `input_holds`; then
   decide by `carry_plan` over the names as tags. A refusal is
   `MemberRetryOutcome::CarryRefused`, whose `Display` the CLI prints byte
   for byte, and nothing is written.
5. Dispose through `disposed` on the engine's store with the fold of step 1
   and `Commanded { command: Retry, operator, reason, args: {"members":
   [<tags>]} }`, the tags being the returned plan's `fresh()`, in the
   panel's declared order whatever order the names were given in. `disposed`
   decides on the fold it is handed, writes the command, and fences the
   acceptance as today: after the write only the race's refusal,
   `lost_fence`, can follow, journaled as the plain retry's race is. The
   plain verb and the bridge fold for themselves and pass `{}`, so their
   bytes do not move. Returned as
   `MemberRetryOutcome::Disposed(FencedCommandOutcome)`, which the CLI
   prints as it prints the plain verb's.

Why the fold is the member form's: the facts of step 4 are read against the
state of step 1, and `disposed` decides the acceptance against that same
fold. An attempt record of the visit can change only through an acceptance
and a drive, which make the run `running` or take the pending place, and
`standing_refusal` refuses either as `lost_fence` under the fence. So an
acceptance lands only on a journal whose attempt records are the ones the
plan read, and what lies outside the journal, the head, the tree and the
inputs, the drive asks again (D7). The acceptance carries the tags and no
fact of the carry.

The outcome is typed because the runtime cannot speak the fold's text:
`brokkr-runtime` names `brokkr-view` as a dev-dependency alone
(`crates/brokkr-runtime/Cargo.toml`) and the view's dependency set is
closed, so the text's one consumer is the CLI handler
(`crates/brokkr-cli/src/verbs/delivery.rs`), as it is for
`operator_refusal` over `FencedCommandOutcome::Rejected` today.

```text
pub enum MemberRetryOutcome {
    /// The fold refused before anything was written; the word is the view's to paint.
    FoldRefused(brokkr_core::Refusal),
    /// The carry refused before anything was written; `Display` is the sentence.
    CarryRefused(CarryRefusal),
    /// Written under the fence: accepted, or the race refusal journaled as today.
    Disposed(FencedCommandOutcome),
}
pub fn operator_retry_members(request: LaunchRequest, run: &str, names: &[String],
                              operator: &str, reason: &str)
    -> Result<MemberRetryOutcome, LaunchError>
```

`LaunchError` because `resume`'s admission is its own and already wraps
`EngineError`; `EngineError` gains no variant, so `engine.rs` does not grow
for it. `CarryRefusal` is a public newtype over the crate-private
`thiserror` enum, with `Display` alone on the wire, so the public API gains
one type and the runtime's tests match the variants (ruling 6: prefer
`pub(crate)`; the CLI consumes the text and nothing else).

Alternatives: journaling carry refusals as `operator/rejected` (the reason
word is a closed vocabulary that cannot carry two heads); the fold's
refusal falling through to the plain path (rejected above, a race);
`launch::pinned_bundle`, a compile and manifest gate extracted for the verb
(superseded: the engine `resume_with` builds asks the inputs fact too, and
no gate moves out of `Engine::resume`).

### D3. The selection's scope

`carry/record.rs` parses `operator/commanded.args` of a `retry` into
`RetryArgs { members: Option<Vec<Tag>> }` with `deny_unknown_fields`, a
`members` absent reading as `None` and a present one read only as a
non-empty list of strings; an absent `args` or `{}` is the plain retry. At
`execute`, the engine finds the retry that caused this effect: the latest
`operator/accepted` before this effect's `effect/requested` whose accepted
command, the `operator/commanded` bearing its `command_id`, is a `retry`,
provided no other `effect/requested` stands between that acceptance and
this effect's; that command's `args` are the ones read. No such acceptance
selects nothing, and the effect runs the whole panel. `operator_stop_reason`
(`engine/operator.rs:120-147`) is cited for what it shares alone: it takes
the latest `operator/accepted` and finds the command it accepts by
`command_id`; it reads no `args`, asks no command word of the acceptance
and bounds nothing by an effect's request, which this search does. The
command's `args` are read first, before the visit's records: `args` that do
not read as `RetryArgs`, a key other than `members`, a `members` that is
not a list of strings, or an empty `members` list, which the verb never
writes since the plain retry journals `{}`, are `CarryRefusal::RecordUnread`
naming the command by its `operator/commanded` sequence number and the
field `args`, refused through `refuse_carry` (D8), and never read as the
plain retry. Its command's `args.members` are the attempt's selection,
handed to `carry_plan` as the tags they are and matched exactly, as the
verb matched them, after the rule has asked the two panel facts of the body
the engine runs. Every bounded attempt of the effect reads the same
acceptance; the next effect of the phase, after implement and verify, has no
acceptance before it and runs the whole panel. The fold reads nothing of
this (decision 0050 ruling 6; `RunState` is unchanged).

### D4. The rule

```text
carry_plan(facts: &CarryFacts, names: &[Tag]) -> Result<CarryPlan, CarryRefusal>

pub(crate) enum PanelShape {
    /// The body is a panel, or a sequence whose first step is a panel.
    Panel { step: Option<String>, tags: Vec<Tag> },
    /// The body is a single seat or a sequence with no panel step.
    NoPanel { phase: String },
    /// The body is a sequence whose first panel step is not its first step.
    StepFollows { step: String, first: String },
}
```

`CarryFacts` is what the engine observed, at the command or at the drive,
by one method, `Engine::carry_facts(events, state, cutoff)`: the selected
body's `PanelShape`, read from `self.bundle` by the same `selected` call
`execute` makes, with a panel's tags in declared order; the visit's attempts
before the cut-off (the command: every attempt; the drive: the attempts
before this effect's request) as typed `PanelAttempt` records in journal
order, each with its terminal kind, its `settled_members`, its `carried`,
its `tree` and `head` at both ends, for a sequence the steps whose markers
stand, its `park_reason`, the `reason` of the first `run/parked` after its
terminal fact and before the cut-off, absent where none follows (the one
source of `Shape::Succeeded`'s reason at both times, never the folded
state's, which at the drive is running and at the command holds the latest
park; S30), and `input_holds`, whether its effect's seat input, rebuilt by
`self.seat_input` under that effect's own id, still digests to its
`effect/requested.input_digest`; the head now (`git_head` of `self.repo`,
kept only in the commit-id shape); and the tree now (`read_tree` of
`self.repo`). The journal's records are read by the reader in
`carry/facts.rs`, which raises `RecordUnread` itself, before the engine is
built at the command (D2) and after the digest check at the drive (D7), so
`carry_plan` takes typed facts alone. `Tag` is a newtype spelled once from
`(step, member)` by the function the markers use, with `bare()` for the
aggregate; the verb's names become `Tag`s as typed and the drive's
`args.members` read as `Tag`s, and both are matched by one exact comparison
against the panel's tags (ruling 5).

The parked attempt is the latest of the visit's attempts before the cut-off
that bears one of three marks: `effect/succeeded`; a terminal fact recording
`tree`; the panel step's `sequence-step-finished` marker. Every other
attempt is passed over (R4).

`CarryPlan` holds one `Contribution` per declared tag, `Fresh` or
`Carried(SettledVerdict { tag, result, source: AttemptId })`, built by a
constructor that refuses a duplicate, a missing member or a tag outside the
panel; `fresh()` and `carried()` are its two views, each in the panel's
declared order. `fresh()` is the selection: the named tags, or every tag for
the all-named short-circuit; the verb journals it as `args.members` (D2
step 5) and the drive reads the same tags back, so one order has one source
and the verb neither resolves nor orders a name.

`CarryRefusal` is one `thiserror` enum, one variant per fact of R4, grouped
where the facts are siblings so each `why` is a closed enum matched
exhaustively (ruling 2):

- `NoPanel { phase }`; `PanelStepFollows { step, first }`
- `UnknownMember { name, tags }`; `MemberTwice { tag }`
- `RecordUnread { record, field, error }`, `record` a closed enum, an
  attempt by its id or, for the accepted retry's `args`, the command by its
  `operator/commanded` sequence number (D3), raised by the journal's reader
  (`carry/facts.rs`, `carry/record.rs`) and never by `carry_plan`
- `Shape { attempt, why }` with `why` one of `NoAttemptEnded { visit }`,
  `Succeeded { park_reason }` (the parked attempt's `park_reason` as the
  reader read it, `None` named as no park following it),
  `EndedAtLaterStep { step }`, `NoRecord`
- `Unsettled { members }`
- `NoHead { repo }`; `HeadMoved { ends: Vec<RecordedHead>, now }`
- `TreeNotClean { observations: Vec<Observation>, now }`
- `InputMoved { effects }`

The rule asks the facts in R4's order: the two panel facts from the shape,
then the names (every member named returns the all-fresh plan and asks
nothing below), then the parked attempt's shape, the unnamed members, the
head, the tree and the inputs. The order, each fact's text and the list each
refusal names, with its stated source, are R1 to R4's and are pinned once in
`carry/tests.rs`; `Display` is the one home of each text, printed by the
verb and written by the engine (D8). The handler's two argument refusals and
the fold's word are not variants: the first are the CLI's, the second the
fold's. `RecordUnread` is the robustness position's distinction between
missing and malformed evidence: a present record that does not read refuses
by name, as a journal that does not fold refuses before anything is written,
and is never read as absent. `UnknownMember` names the panel's tags, so a
bare name on a sequence panel is answered with the spelling to type.

Alternatives: a `Names` enum with the operator's spelling resolved against
bare names and tags, and an `AmbiguousMember` fact for a name that read two
ways (withdrawn at S27: a second spelling with one consumer, convenience,
and a fact that existed only to guard its collision; rulings 5 and 6); two
observers, the verb over a compiled bundle without an engine and the drive
over the engine (superseded at S27 by the one method).

### D5. The four identity facts

- **The head.** `git_head` of `self.repo`, which `--repo` names at the
  command, as `resume`'s does. The reading must satisfy `is_commit_id`
  (`engine.rs:3388`), the one home of the shape `phase-entered-head.v1`
  pins, else `NoHead`; each recorded end of the parked attempt and of
  every source attempt is compared as a string, an absent end unequal, a
  present end outside the shape refused by the reader before this (D6), and
  every unequal end is named in the journal's order with the head now last.
  `phase/entered.head` and the gate-head observation are not read: a retry
  re-enters no phase, and the gate-head observation watches only a gate's
  own span.
- **The tree.** The reader's `clean` at those same ends, at the command and
  at the drive; every other observation named with its reading (D10).
- **The bundle.** `resume`'s own admission, at the command through
  `resume_with` and at the drive through `resume` itself: the compile under
  the manifest gate, the charters bound as `run/started` recorded them, the
  crossings fenced. The engine does not ask it again at `execute`: the
  bundle it runs is the one `resume` admitted.
- **The inputs.** For each attempt a verdict is read from, `seat_input`
  rebuilt under that attempt's effect id, digested and compared with its
  `effect/requested.input_digest`, exactly as `execute` compares this
  effect's own; at the command through the engine `resume_with` built, at
  the drive through the engine that runs the attempt, one function at two
  times. One function at two times holds only when the engine at the
  command is built from what the engine at the drive is built from. The
  seat input embeds the request's workspace and repository, through the
  result paths and `workdir`, and, for a seat that declares secrets, the
  operator-side store's path, `self.secrets_file` else the run-local
  `secrets.env` (decision 0012; `engine/input.rs:83-85`), which the digest
  covers. So the member form takes `--secrets-file` as `resume` takes it,
  beside `--repo`, `--bundle`, `--recipe` and `--recipes-dir`, and
  `--no-view` alone is not taken, since it rules the drive's painting and
  reaches no input. A run driven with a store and retried by member with
  none, or another, is refused at R4's item 9 naming the source effect,
  the fact `brokkr resume` without the flag refuses at the drive as a
  changed input (`refuse_changed_input`). The verb reads no secret value:
  the engine threads the path alone, and no store read exists in
  `brokkr-runtime`.

Why the whole-seat digest holds what a member judged, which robustness asked
to see written: a member's input (`member_runs`, `panel.rs:117`) is the seat
input's `feature`, `phase`, `workdir`, `allowed_results`, `house_rules` and
`context` copied whole; `role_path` and `result_path` from the seat input's
own `members` entry, so the charter's path is in the digest and its content
is held by the charter binding; the dialect prose, from the pinned bundle or
the seat input; the secret-binding facts, from the seat input; the site
marks, the launch arguments and the serving facts, which the pinned bundle's
site facts and the selected candidate fix and which say who judged and how
it was run; and `resume_context`, the member's own session plan. The seat
input itself is `seat_input(state, phase, effect_id)`, one function over
the pinned bundle, the folded state's context (`last_decision`, the earlier
phases' results, `returned_from`), the repository, the realm's house rules
as the manifest pins them (the engine `resume` builds reads its world from
the pinned manifest, `World::from_manifest`, `engine.rs:710`, and
`seat_input` embeds `world.house_for(&self.repo)`, `input.rs:88-90`), the
store path of a seat that declares secrets (`input.rs:83-86`,
`engine.rs:1126-1130`) and the effect's result paths. The source effect's digest holding now means
that function, over the context as it stands now, reproduces what the
carried members read; the carrying effect's seat input is the same function
over the same context with its own effect id, so the two differ in the
result paths alone, which name a file and judge nothing. The incident's park
writes no `transition/decided`, so the context stands; a rule-driven park
after a succeeded attempt moves `last_decision`, but that attempt is refused
as `Succeeded` before the inputs are asked; the acceptance itself moves
neither `visits` nor `last_decision`, since a retry re-enters no phase.
Hence a per-member projection with `result_path` struck out by hand would
restate this derivation as a second definition of the input (ruling 5) and
hold nothing more; and the whole-panel digest refuses more than a per-member
one would, never less: any byte of the seat input that moves, the store
path a seat that declares secrets embeds among them, refuses every carry.
An edit to the realm's house-rules file after the run started is not such
a byte: on a resumed engine the house text is the manifest's pin, so the
edit reaches neither a carried nor a fresh member and moves no digest
(the test `a_v3_world_pins_house_and_dialect_and_house_content_moves_run_identity`,
`realms/tests.rs:379`, holds the rehydrated world to the pinned text).
What `member_runs` adds after the digest, and why none of it is the subject
(robustness's third-visit question): the selected candidate
(`select_candidates`: its agent, model, effort, argv and hands), the site
marks and the serving facts composed from it, the launch arguments, the
session plan (`resume_context`: the assessment, the originating harness,
the owned session target) and the effect's own result paths. Each names who
judges, how it is run and where it writes, never what it judges: the
subject is the repository at its head and tree, the pinned bundle's site
facts and the seat input, and the four facts hold all of them. A chain
fallback between the source attempt and the carrying one moves the
candidate of a fresh member alone, since a carried member is not in the
narrowed body and selects nothing (D7); the candidate that served the
carried verdict stands in the source attempt's `provenance` and its member
marker's `model`, where every readout names it (D9), so the verdict is
carried as that reviewer's verdict on this subject, and the carrying
attempt's `provenance` does not name it. The spec's scenario `A chain
fallback between the attempts` and D13's test bind this. Widening the
identity to a serving fact would be a conscious, versioned change of the
contract, not a field added to a projection.


### D6. The record

`carry/record.rs` holds `PanelRecord { tree: Ends<TreeReading>, head:
Ends<Option<Head>>, settled_members: Option<BTreeMap<Tag, Value>> }` and
`CarriedEntry { member: Tag, attempt_id }`, serialised with
`deny_unknown_fields` and read back by the same types. `Head` parses by
`is_commit_id` (`engine.rs:3388`), the one home of the shape
`phase-entered-head.v1` pins, reached as a private item of its ancestor
module, and by nothing else, one shape with no second reading and no second
predicate (rulings 3 and 5), so no field is doubly optional: `head` is
written absent when neither end holds one and reads back as two absent
ends. The writer stamps `git_head`'s
answer only when it is in that shape, else the end is absent, as
`phase-entered-head.v1`'s reader treats an answer outside its pattern (a
SHA-256 repository answers sixty-four digits and records no head). A `head`
end present outside the shape, or a `settled_members` or `head` beside no
`tree`, is a record the engine never wrote, and the reader refuses it as
`RecordUnread` (D4). `PanelRecord::stamp` merges the fields into the panel's
`effect/indeterminate` or `effect/failed` payload beside `reason`,
`unresolved_members` and the start-failure fields, which keep their bytes;
`settled_members`, filled by `settled_members(reports, prefix)` in
`carry/record.rs`, one free function over the fresh reports that both the
panel and the panel step call, holds each fresh member's
`AttemptOutcome::Succeeded { result }` value, the same value the rule feeds to
the aggregate, whatever JSON the member's driver reported, since the adapter
forwards whatever the seat's result file parses to and repairs nothing
(decision 0001), and is absent when no member settled. The `Value` under each
tag is the one deliberate exception to ruling 3 in this change, named here:
the result is the recipe's and opaque to the engine, which carries it today as
`AttemptOutcome::Succeeded { result: Value }` and types it only at the
aggregate's edge; the record keeps that boundary where it is and types every
other field. The reader reads any JSON value under a tag, so the engine never
writes a record its own reader refuses, and a carried value that is no object
poisons the aggregate exactly as the same value in a fresh panel would. The
readings at both ends are the panel's own, taken at its top before any member
spawns and before its terminal fact (D7). The sequence stamps the same record
on its own terminal fact when a panel step's outcome ended the attempt, from
the span and the settled verdicts the step returned on `StepRun`, and nothing
on any other terminal fact. `carried` rides `effect/started` beside
`provenance` and `boundary`, absent when nothing is carried, so a plain
retry's attempt is byte-identical to today's.

`contracts/panel-carry.v1.schema.json` publishes the five fields (`tree`,
`head`, `settled_members`, `carried`, `args.members` for `command: "retry"`),
each pinned to the shape the reader reads, the `head` ends to
`^[0-9a-f]{40}$` as `phase-entered-head.v1` pins its own, each value of
`settled_members` to any JSON value, and `args.members` to a non-empty list
of tags, under the amended additive rule of `contracts/README.md`,
registered in `crates/brokkr-runtime/tests/frozen_contracts.rs` with its
paragraph in the README. The fold reads none of them. It is minted under
0093 and lands on `main` only once 0093 is accepted (decision 0082 ruling
1).

### D7. The carrying attempt

In `execute`, after the digest check: the selection is read from the
accepted retry's `args` (D3), whose failure to read is `RecordUnread`
naming the command, refused as below; when the selection names members,
every member named included, `Engine::carry_facts` reads the attempts
before this effect's request into typed records (a field that does not read
is `RecordUnread`, refused as below) and fills the rest of the facts (D4),
and `carry_plan` decides over the journaled tags, asking `NoPanel` and
`PanelStepFollows` first. A refusal becomes `refuse_carry` (D8). A plan
narrows the body once, at the seat's clone `execute` already takes: the
panel's `members` kept to the plan's `fresh()`, or the sequence's first
step's panel kept to them, so `selected` yields the narrowed
`ExecutableBody` with today's lifetimes and `Dispatch` holds it as it holds
every body. From there admission, provisioning, `select_candidates`, the
driver label, the session offers, `provenance`, `boundary` and the member
markers cover the fresh members and no other, with no second rule. The plan
is handed to `plan_attempt`, which writes its `carried()` entries on
`started` beside `provenance` and `boundary` and sets it on the `Selection`
it returns, `selection.carry`: the plan's one home during the drive, in the
value that already reaches every body function and that `Selection`'s doc
comment describes, one decision taken at one moment from one journal
(ruling 5). A first attempt, a plain retry, an acceptance with `{}` and an
effect with no acceptance before it hold `None`; a member retry holds
`Some`, the all-fresh plan of the all-named retry included, whose
`carried()` is empty and writes no `carried`. `Dispatch` and `Engine` gain
no field.

`execute_panel` and the sequence's panel step (`sequence.rs`, `panel_step`)
run `member_runs` over the narrowed members, journal their markers, and
reach the one rule through one function, `outcome_under(aggregate, plan:
Option<&CarryPlan>, reports)` in `carry.rs`: under `None` it is
`panel_outcome(aggregate, reports)`, today's function with today's
signature, whose three arms have moved below it into
`outcome_over(aggregate, pairs)` over `(name, AttemptOutcome)` pairs; under
`Some(plan)` it is `outcome_over(aggregate, plan.join(reports)?)`, where
`CarryPlan::join` yields the pairs in the panel's declared order, a `Fresh`
entry taking its report's `settled_outcome()` under the member's bare name
and a `Carried` entry standing as `AttemptOutcome::Succeeded { result }`
from the plan, so no `AttemptReport` is fabricated for a member that did
not run. The rule's indeterminate, failed and aggregate arms are unchanged,
so the result, `inputs` and `notes` are a fresh panel's bytes
(acceptance 1); `Unproven::panel` and `start_failure_sites` read the fresh
reports as today. The plain path does not join, so its aggregate keeps its
bytes by construction; the two paths end in `outcome_over`, and D13's first
test drives the same verdicts through both and compares the bytes. The
join is written once and called from one function (ruling 5); a `Fresh`
entry with no report, a report for no `Fresh` entry, or two reports for one
tag is a broken engine invariant (`EngineError::Invariant`), never a
member's failure, since the narrowed body and the join read one plan.
`panel_step` hands `outcome_under` the plan `run.selection.carry` holds
when the step is the first and `None` for any later panel step (D4: a carry
applies to the first step alone).

The panel observes its own span. `execute_panel` and `panel_step` read the
tree and the head (`read_tree`, `git_head`; D10, D5) at their top, before
`run_panel` spawns a member, and again before the terminal fact; the
record's two ends are those readings (D6), which for a seat's panel and a
sequence's first step are the attempt's own ends, since nothing of the
attempt between `effect/started` and the panel's top touches the
repository (admission reserves a lease and provisioning seals brokers),
and for a later panel step, never carried, the step's own. `execute_panel`
stamps the record, with `settled_members` over its fresh reports, on its
`effect/indeterminate` or `effect/failed`; `panel_step` returns its opened
span and its settled verdicts on `StepRun`, and `step_result`, taking the
`StepRun` whole, stamps them on the terminal fact of a failed or
indeterminate panel step and on nothing else, so the vocabulary check, the
marker's fence and the change check keep their bytes (A1). This is the
shape of a sequence step's gate-head observation, armed in `sequence_step`
at the step's own start; it adds no field to `Dispatch` or `Engine`, and it
threads nothing through `execute_sequence`. The carry's own reading of the
head and the tree now, at `execute` (D4), is a second observation at an
earlier moment with another purpose, the precondition's; the record keeps
what the members judged.

A fresh indeterminate parks the attempt and a fresh failure fails it; the
bounded retry reads the same acceptance and the same parked attempt (D3),
so it carries the same entries and runs the named members again, never a
verdict that settled in the failed attempt.

Crashes: after the acceptance and before the request, resume requests the
effect and reads the selection; after the request, resume rebuilds the
digest and the plan from the journal; after `effect/started`, the engine's
`EffectInFlight` arm (`engine.rs:985`) settles the attempt indeterminate as
today, which carries no `tree`, so the attempt is passed over and the next
member retry carries from the attempt before it; after a partial
settlement, the fresh verdicts that settled are lost with that attempt,
fail-closed, and the named members run again. The carry opens no
settlement path of its own: a verdict is carry evidence only as the
`settled_outcome()` of a member in an attempt its panel ended, which reads
`refused` and `cleanup`, and an attempt in flight is settled by the
engine's arm and never read.

### D8. A carry refused at drive time

The attempt starts on `driver: "none"` and ends `effect/indeterminate` with
`reason` equal to the refusal's `Display`, the sentence the verb would have
printed. The `close` arm reads it, as it reads every unmarked reason, to
`Stated("effect <id> indeterminate: <sentence>")`; `run/parked` carries that
text with `{}` evidence; every readout paints it as it paints every stated
park. Nothing is auto-retried (decision 0006 keeps an indeterminate for the
operator), nothing of the panel spawned, and the next member retry passes
the attempt over.

Alternatives: a marker, a codec and a `ParkReason` variant (rejected at S22:
no consumer, a public-API move in `brokkr-core`, a bytes/sentence split and
a timeline decode for a painter); running the whole panel (rejected: silent,
spends what the operator asked not to spend); a `run/parked` with no attempt
(rejected: a requested effect with no terminal fact).

### D9. Readouts

`scan_participants` (cyclomatic 39) and `participants` (21) are over
ruling 4's ceiling and may only shrink, and `activity_for` (23) with them;
the crap ratchet is the gate. So the carried read, the carried status and
the carried sentence are four helpers in `started.rs`, the child module of
`lib.rs` D1 adds, which reaches `Scan`, `Build` and `ensure` as its parent's
private items, and neither function gains a branch:

- `sites(scan, slot, effect_id, payload)`: the one reader of an
  `effect/started`'s per-site lists. It clears `boundary`, `stamp` and
  `carried` on every part of the effect, as the scan clears the first two
  today, then reads `provenance`, `boundary` and `carried`, each entry
  ensuring its member's `Build` under the carrying effect and setting its
  field, so a bounded attempt's entries are its own start's and a start
  without `carried` leaves none. The scan's `EffectStarted` arm calls it
  once in place of its three loops.
- `status_of(part, effect)`: the derivation the scan's last loop makes today
  (the effect's open or terminal state, then a member's marker), moved
  whole, with one arm before them: a part with a `carried` entry is
  `carried`, class `completed`, whatever its effect's state, since nothing
  of it ran. The loop becomes one assignment per part.
- `carried_from(events, scan, part)`: `Some(CarriedFrom { attempt_id,
  ordinal })` from the entry, where `ordinal` is the source attempt's place
  among the phase's `effect/started`s in journal order, every visit counted.
- `activity(events, scan, part, member_count, carried_from)`: a carried
  participant's activity cell is the one sentence `carried from <phase>
  attempt <ordinal>`, with the attempt id as the cell's note; every other
  participant's is `activity_for`'s, unchanged. `participants` fills
  `carried_from` and `activity` by one call each.

Every surface that paints the activity cell, `inspect` and `seats` through
`seats_block`, `watch`, the TUI, `ui.html` and the console, so paints the
sentence, leading with the ordinal, and composes nothing (ruling 7); the id
stays because it is the journal's one name for the attempt, the key
`inspect --json` and the console's data join on, and an ordinal names
nothing in the journal. A carried participant ran no
seat, so it has no model, session, transcript, cost, turns or provenance of
its own under the carrying effect, and its `working` is false; its evidence
stands under the effect it settled in. The surfaces that paint participants,
`inspect`, `seats`, `watch`, the TUI, `ui.html` and the console, paint it
(ruling 7); `brokkr runs` and the fleet readouts paint run rows that hold no
participant. Of the readers of participants that paint no status, one changes:
`compare::resolutions` (`compare.rs:359`) collects participants into a map by
label, the last of a label standing, so it skips a carried participant, else
an empty resolution would replace the real one under its label, a correctness
fix both positions kept. `select_transcript_participant` is not edited, so a
carried participant's key is listed among an ambiguous label's keys and
refused `no-reference` when chosen, as every participant without a transcript
is today; muninn's cost rows and the dashboard's working checkpoints need no
change, for the reasons A3 gives. `render.rs` tones `carried` as good
(`tone`, cyclomatic 5), and `inspect` (17, over the ceiling) is not edited,
since it paints participants through `seats_block`; `VIEW_VERSION` moves
from 23 to 24. The console offers no member retry.

Alternatives: skipping a carried participant in the transcript verb's
selection (withdrawn at S27: a second rule for one kind of participant
without a transcript, where the general cut is a change of its own);
`CarriedFrom` by the ordinal alone (rejected: the id's consumer is the
join).

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

`CarryRefusal`'s `Display` in `carry/tests.rs`; S20's text in the handler's
test module `verbs/delivery/member_tests.rs`, where today's pin moves (tasks
T5: `crates/brokkr-cli/src/tests.rs` has one line of headroom); S21's in
`crates/brokkr-view/src/commands/tests.rs`; `resume`'s admission texts where
they are pinned today and not again. Tests match variants (ruling 8) and pin
each text once. `MemberRetryOutcome` is matched by variant in the runtime's
tests, and the CLI handler's tests assert that it prints
`brokkr_view::member_retry_refusal`'s output for the word, against the
function and not a second copy of the text, so S21's text has one pin.

### D12. Coordination

- **The number is 0093**, `Status: proposed`. Verified on 2026-10-21 against
  the remote refs: `origin/main` holds 0091 (#717's decision, `d7907e93`)
  and `origin/recovery/789-environment-blocks-park` holds 0092 (#802's), so
  the commission's rule, the next after #802's, gives 0093; the commission's
  own "0092" predates #802's renumbering. This branch holds decisions up to
  0090, so the index carries gap rows for 0091 and 0092 that leave when each
  lands; the index's gate refuses an unexplained gap and a gap row a file
  then takes. The operator may renumber at merge, a substitution over this
  change's artifacts alone.
- **#802's hunks**: this change does not edit `engine/decide.rs`, `0022`,
  the state diagram or the parks list; it edits `engine/tests.rs` only at
  its seven direct `execute_panel` calls (D1), each in place and shrinking,
  none within #802's one hunk at lines 2347 to 2380, so the two branches'
  edits to the file do not meet; its `machine_proof.rs` line is `mod
  panel_carry;`; the index row is the one meeting. Task 1.2's verify step
  reads the diff of `engine/tests.rs` as those seven hunks and nothing
  else, and `decide.rs`'s as empty.
- **The figures**: `engine/tests.rs` 6,556 and `boundary_tests.rs` 4,207
  may not grow and shrink under D1; `brokkr-view/src/lib.rs` may reach
  2,789, the ratchet's baseline, and the design uses no other figure for
  it.
- **The contract's landing** waits on 0093's acceptance (decision 0082
  ruling 1); the pull request says so.
- **The `Built:` marker** moves with the index row: the file's
  `Built: unbuilt (#787)` becomes `Built: built` as the row's cell does,
  since the index gate renders the cell from the marker (task 12.2).

### D13. Tests, and the mutation that proves each binds

Every new test lands in a new module (D1); the existing tests D1 lists
change only in the shape of their `execute_panel` call and keep their
assertions, and the existing `panel_outcome` and `execute_sequence` tests
are not touched. Each new test names the compiling mutation that must make
it fail (ruling 9); the implementer records the failing run and restores
the behaviour.

`crates/brokkr-runtime/src/engine/carry/tests.rs`, over synthetic journals
built by a builder and a synthetic panel bundle:

- the plan over the incident's journal: three `Carried` by the parked
  attempt, one `Fresh`; mutation: the constructor accepting a plan with a
  member missing;
- the same aggregate as a full re-run: the same verdicts driven through
  `outcome_under` under `None`, the full re-run's path, and under
  `Some(plan)` carrying three of them give `result`, `inputs` and `notes`
  byte for byte, under `ReviewPanel` with a `residual` carried verdict kept
  as `residual` and under `UnanimousPass` with a carried `fail` kept as
  `fail`; mutation: the carried entries dropped from the join;
- each `CarryRefusal` variant by its fact, in R4's order, with the lists
  each names (`HeadMoved` with two ends and now; `TreeNotClean` with two
  observations; `Unsettled` with two members in declared order;
  `InputMoved` with two effects once each; `UnknownMember` with the panel's
  tags); mutation per variant: the fact removed from the rule, and the order
  test's mutation: two facts swapped;
- the three-way test over every passed-over kind and every marked kind of
  R4, the carrying attempt that ended at the chief included; mutation: the
  marker's mark dropped from the test;
- the succeeded attempt's park reason, read from the `run/parked` after
  it: a rule parks after a succeeded attempt, a plain retry's effect is
  then refused admission until it parks `effect <id> failed 2 of 2
  attempt(s); …` (`engine.rs:940-964`), and `Shape::Succeeded` names the
  rule's reason at the command's cut-off and at the drive's; mutation: the
  folded state's reason used instead, so the later park's is named;
- the read-through: the same member stuck twice carries by the first
  attempt; mutation: the entry's source replaced by the carrying attempt;
- the two panel facts from the shape: `NoPanel` and `PanelStepFollows`
  before any name; mutation: the shape's arms folded into the names step
  (an unknown name then refuses where the phase runs no panel);
- the tags in a step `p` with members `x` and `p:x`: `p:x` matches `x`
  alone, `p:p:x` matches `p:x` alone, and `x` refuses `UnknownMember`
  naming `p:p:x` then `p:x`; mutation: the match widened to the bare name,
  so `p:x` matches two;
- `fresh()` over names given out of declared order (`security`,
  `correctness`) in declared order (`correctness`, `security`); mutation:
  `fresh()` in the order given;
- every member named: on a dirty tree, a moved head, a moved input and a
  park at the chief, the rule returns the all-fresh plan and asks nothing
  below the names; mutation: the short-circuit removed;
- `RecordUnread` on a `settled_members` that is not an object, on a
  `tree` with a fifth word and on a `head` end outside the shape, raised by
  the reader before any fact; mutation: the parse falling back to "no
  record" or to "no head";
- the join: a plan with one `Carried` entry joins the fresh reports to a fresh
  panel's pairs byte for byte, the all-fresh plan joins to the pairs
  `panel_outcome` reads from the reports alone, and a report twice for one
  tag, a report for no `Fresh` entry and a `Fresh` entry with no report are
  each `EngineError::Invariant`; mutations: the join emitting the reports'
  order in place of the plan's, and the duplicate check removed;
- the scope: an acceptance before an earlier effect's request selects
  nothing for a later effect, and another command's acceptance after the
  retry's and before the request does not hide the retry; mutations: the
  "no request between" condition dropped, and the `retry` command-word
  check dropped;
- `RetryArgs` with `deny_unknown_fields`, refusing an unknown key, a
  `members` that is not a list of strings, `null` included, and an empty
  list as `RecordUnread` naming the command, with `{}` and an absent `args`
  the plain retry; mutation: the attribute removed, and the empty-list
  check dropped;
- a `settled_members` whose value under a tag is a string reads back as
  written, and a plan carrying it joins to the `__member-schema-invalid__`
  aggregate a fresh panel's same value gives; mutation: the reader pinned
  to objects.

`crates/brokkr-runtime/src/engine/worktree/tests.rs`: the four readings over
a clean repository, an edited one, a repository whose index is corrupted
(`.git/index` overwritten with bytes that are no index, which git refuses at
exit 128 printing nothing whatever user the box runs as, where a 0o000 index
reads fine as root) and a missing directory, and `git_dirty`'s `false`,
`true`, `false`, `true` over them; mutation: `Refused` folded into `Dirty`.
The existing `git_dirty`
test in `engine/tests.rs` (lines 2043 to 2058) holds unchanged.

`crates/brokkr-runtime/src/engine/operator/retry/tests.rs` (the verb, with
the fake driver, matching `MemberRetryOutcome` by variant): a member retry
journals `args.members` in the panel's declared order, `--member security
--member correctness` on a panel declared `adversarial`, `correctness`,
`security`, `spec-compliance` journaling `{"members": ["correctness",
"security"]}` (mutation: the names journaled in the order given); a fold
refusal returns `FoldRefused` with each of the three words and writes
nothing (mutation: the fold step skipped, so the race refusal appears where
`FoldRefused` should); a record that does not read beside a drifted bundle
returns `CarryRefused(RecordUnread)` with the engine unbuilt, and so does
every member named beside a record that does not read; a drifted bundle on
a sound record returns `resume`'s `LaunchError`, before any panel fact; a
run driven with `brokkr resume --secrets-file /keys/ops.env` on a seat that
declares a secret, retried by member with `--secrets-file /keys/other.env`,
returns `CarryRefused(InputMoved)` naming the source effect and writes
nothing (mutation: `input_holds` dropped from the command's facts, so the
command is admitted); a run driven with `brokkr
resume --secrets-file` on a seat that declares a secret: the member form
given the same store is admitted, and given none returns
`CarryRefused(InputMoved)` naming the source effect (mutation: the store
dropped from the member form's request, so the first refuses); the carrying effect's own
result paths differing from the source's leaves the command admitted
(mutation: the source rebuilt under the carrying effect's id, so every
carry refuses); a phase with no panel and a panel step after the first step
return `CarryRefused(NoPanel)` and `CarryRefused(PanelStepFollows)`;
`--member correctness` on the review sequence returns
`CarryRefused(UnknownMember)` naming the four `positions:` tags, and
`--member positions:correctness` is admitted and journals that tag; every
member named carries nothing, a sequence parked at `chief` included; a
peer's plain retry, its command and acceptance, appended through
`retry_members_racing`'s `before_command` hook, after the facts are decided
and before `disposed` writes the command, is `Disposed(Rejected)` with
`lost_fence`, journaled, and no `operator/accepted` carries the tags
(mutation: `disposed` folding for itself after the facts, so the peer's
acceptance reads as illegal when asked and the word is
`run_not_awaiting_operator`); the same peer through `disposed`'s existing
`between` hook, after the command is written, is the plain retry's race
and refuses `lost_fence` as today, kept as a second case; `retry_members`
is `retry_members_racing` with a no-op hook, as `operator_command` is
`operator_command_racing`'s; no runtime test reads a
`brokkr-view` text.

`crates/brokkr-runtime/src/engine/carry/drive_tests.rs` (dispatch, with the
fake driver): only the named members spawn, counted; `provenance`,
`boundary`, the driver label, one offer, one admission claim and one marker
cover the fresh member alone; the start and end readings on the terminal
fact; a head moved between acceptance and drive parks with the sentence as
the stated reason and `run/parked` carries it; an acceptance given under
one secrets store and driven by `brokkr resume --secrets-file` naming
another parks naming the source effect; the bounded retry after a start failure
carries the same entries; a verdict settled in the failed bounded attempt is
not carried; result files deleted after the park change nothing; a journal
written before this change refuses `NoAttemptEnded`; the sequence's first
step carries under `positions:` tags and `chief` runs on the aggregate; a
sequence whose panel step ends it carries `tree` and `head` on its terminal
fact, from the step's own two ends, and one ended at the chief carries none
(mutation: the stamp applied whatever step ended the attempt); a chain
fallback between the attempts: the named member's driver fails to start in
the carrying effect's first attempt under a chain of two candidates, the
second attempt selects the second candidate, the four facts hold, and it
carries the same verdicts from the same source attempts, its `provenance`
naming the fresh member under the second candidate alone, while the carried
participants keep the model their source attempt's `provenance` named
(mutation: the fresh member's candidate compared between the attempts as an
identity fact, so the carry refuses); the restart indeterminate carries no
record and is passed over; an accepted
retry whose `args` do not read parks on `driver: "none"` naming the
command, and one whose `args` are `{}` runs every member; an all-named
acceptance beside a record that does not read parks naming the attempt and
the field; an accepted retry whose `args.members` meet a body with no
panel, and one whose panel step follows another step, each park on
`driver: "none"` naming the fact. Mutations: the drive-time check removed
(the moved head is carried); the drive's call of the rule over the shape
removed (the two bodies then reach the names with no panel); the narrowing
removed (four spawns); the end reading removed (`tree` absent).

`crates/brokkr-runtime/tests/frozen_contracts.rs`: the new schema's
registration and bytes; `crates/brokkr-core/src/fold/tests.rs`: a journal
with every new field folds to the state it folds to without them, byte for
byte (the contracts README's machine-checked clause).

`crates/brokkr-cli/src/verbs/delivery/member_tests.rs`, the handler's new
test module (tasks T5): the two argument refusals before every refusal the
handler makes today, `--secrets-file` without `--member` among them, S20's
text on `retry` with and without `--member` and today's on `stop`, moved
from `tests.rs:4338-4353`; and over a synthetic journal with a running run,
the handler prints
`brokkr_view::member_retry_refusal`'s output for the fold's word, asserted
against the function, and a carry refusal's `Display` alone; mutation: the
handler printing a line of its own. `crates/brokkr-cli/tests/machine_proof/
panel_carry.rs`: the three acceptance lines end to end over a committed
workdir; under decision 0022's reforging table with the review seat a panel,
the retried member's security residual at `low` aggregated as a fresh
panel's, `REVIEW-REFORGE` routing the run to implement and verify, and the
re-entered review running every member fresh with no `carried` (decision
0093 ruling 3); the plain retry's `effect/started` byte-identical to
today's; and `inspect` and `seats` naming the carried participants with
their ordinal.

`crates/brokkr-view`: in the new module's tests, the scan over `carried`, the
participant's status, class and `carried_from`, a carried participant under
an effect still open reading `carried`, not `working` (mutation: the carried
arm of `status_of` removed), the activity cell's sentence with the attempt id
as its note (mutation: `activity` yielding `activity_for`'s cell for a
carried part), no model or cost of its own and `working` false, the ordinal
across two effects, `carried` across two
attempts of one effect, the second start's entries replacing the first's and a
start without `carried` clearing it (mutation: the clearing removed), and a
plain retry's view unchanged; in `compare/tests.rs`, `resolutions` over a
journal with a carrying attempt gives each carried label its source
participant's resolution (mutation: the skip removed, so the carried
participant's empty resolution stands); muninn's cost rows,
`working_checkpoints` and the transcript verb's selection are not edited and
take no test of this change (D9); `commands/tests.rs` pins S21's text; the
`VIEW_VERSION` pin moves.

## Risks

- **Edits inside an attempt's span, reverted before its end, and edits to
  ignored files** are unobserved: the residual the gate-head observation and
  the ship gate already accept. Named, not closed.
- **A head moved and put back inside one attempt's span** is unobserved for
  the same reason. A head moved during a park and put back is refused by
  the recorded ends.
- **The whole-panel input digest** refuses every carry when any member's
  input moved (D5). Inside one run that needs a bundle drift, which the
  manifest gate already refuses, or an argument the engine is built from
  that the seat input embeds, the secrets store's path above all, given
  otherwise than the source effect was requested under, which should
  refuse. An edit to the realm's house-rules file is neither: on a resumed
  engine the house text is the manifest's pin, so the edit reaches neither
  carried nor fresh members (D5). Accepted.
- **`resume`'s admission at the command.** The member form refuses, in
  `resume`'s words, a run `brokkr resume` would refuse: a drifted bundle, a
  moved charter, a moved crossing, an expired Looper dispatch, a host that
  cannot box the bundle. The plain retry accepts such a command and the
  resume refuses it later; the member form learns it first. Intended, and
  R2 says so.
- **The journal folded twice at the command**, once by the member form's
  deciding fold and once inside `Engine::resume` (`replay.caught_up`). Both
  are reads; the cost is accepted over editing `Engine::resume`.
- **A bare name on a sequence panel** is refused once, with the tags to
  type. Accepted at S27 over a second spelling.
- **A partial settlement lost to a restart** (D7) costs the named members a
  second run. Fail-closed; the attempt before is read.
- **The fence's window.** The facts are read at the fold `disposed` decides
  on, the fence refuses a run that moved between, and the drive asks the
  world's facts again: no window admits a carry the facts do not hold for.
- **A record the engine never writes** (`RecordUnread`) refuses rather than
  carries; the plain retry stands.
- **The store at the command.** A run driven with `--secrets-file` is
  retried by member with the same store, as `brokkr resume` must be given
  it at every drive; given none, the inputs fact refuses naming the source
  effect, by the one rule, and the operator passes the store. Accepted
  over reading the store's path from the journal, which holds it only
  inside a digest.
- **`engine/sequence.rs` at 714 lines** gains the plan's read, the span and
  the stamp on `StepRun`, within its 800 ceiling; if `step_result`'s stamp
  would pass it, the stamp lives in `carry/record.rs` and the sequence calls
  it.
- **`engine.rs` stands at its baseline**, so `git_head` and `git_dirty`
  move into `worktree.rs` whole and pay for `Selection.carry`, the two
  `mod` lines and the two `use` lines; task 1.2 measures the file at or
  under 3,523 lines.
- **Two paths reach the one rule**, `None` and `Some`. Both end in
  `outcome_over`, and D13's first test drives the same verdicts through
  both and compares the bytes, so a divergence fails a binding test.
- **The record's start reading and the carry's reading at the drive** are
  two observations moments apart. The first is the evidence's, the second
  the precondition's; a tree edited between them is recorded at the
  panel's start and refuses the next carry by the record, fail-closed.
- **A positional `PanelAt::new` with a trailing `bool`.** A struct literal
  at each site grows both baseline files; the four fields are the
  `Dispatch` fields a panel reads, in their order.
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
- `--member` takes the tag the readouts print: `correctness` names a seat
  panel's member and `positions:correctness` the shipped review sequence's.
- `docs/guides/quickstart.md`'s operator commands and `ARCHITECTURE.md`'s
  panel and operator-verb prose name the member form; `docs/reference/
  cli.md` is regenerated from clap.
- The contract lands on `main` with 0093's acceptance, or the operator
  accepts at the merge.

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
