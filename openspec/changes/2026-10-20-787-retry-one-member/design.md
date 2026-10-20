# Design — a retry by member re-runs the named members and carries the rest (#787)

Decision 0093 carries the rulings. This design carries what the code does,
the evidence behind each choice, and the council's two positions reconciled.
The proposal's record (S1 to S25) holds the clarify history; a decision is
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
  precedent for the engine finding an accepted command by its
  acceptance's `command_id`; it reads no `args`.
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
  `settled_members` holds the member's whole result value from its
  post-fence `settled_outcome`, any JSON value as its driver reported it, so a `residual` or `fail` verdict carries
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
- **Every member named is R4 item 4's stated short-circuit** (cut 7,
  corrected on clarify's pass after S24, C6). It does not fall out of the
  identity facts as vacuous: with no verdict carried, the head now's shape,
  the tree now and item 5's four shape facts about the parked attempt
  (`NoAttemptEnded`, `Succeeded`, `EndedAtLaterStep`, `NoRecord`) would
  still be asked and would refuse a dirty tree, a park at the chief and a
  parked attempt with no record, which the spec's scenarios `Every member
  named` and `Every member named on a run parked at a later step` admit.
  So once the names resolve to every member, the rule returns the
  all-fresh plan and asks nothing below item 4 (S18, task 4.2). The visit's
  records are read by the reader before the compile and before any name
  (D2, D4), so a record that does not read refuses the all-named retry as
  it refuses any member retry, at the command and at the drive (S24, C1);
  the all-named case is not exempted from the reader, since that would
  move the reader after the names and reverse S23's order.
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
  `carry/tests.rs`: the rule (D4), the one join of carried and fresh
  verdicts (D7), the `panel-carry.v1` types that both the writer and the
  reader use (D6), and the observers that fill the facts from the journal
  and git, the journal's reader refusing a record that does not read (D4).
  Observing is apart from deciding (ruling 10).
- `engine/operator/retry.rs`: the verb's member form (D2), returning the
  typed `MemberRetryOutcome`, beside `conclusion.rs`, `receipt.rs` and
  `supersede.rs`.
- `launch.rs`: `pinned_bundle(request, run)`, the compile and the manifest
  comparison as one function `resume` and the verb both call; the
  comparison moves out of `Engine::resume` into it, so `engine.rs` shrinks
  and neither caller copies it (ruling 5).
- `engine/dispatch.rs`: the drive-time check, the narrowed body, the start
  readings and the `carried` entries (D7). `Dispatch` gains the attempt's
  panel span, so no field is added to `Engine`.
- `engine/panel.rs`: `execute_panel` takes a parameter object and the plan;
  its nine parameters become seven and its `too_many_arguments` expectation
  goes (ruling 10: a long list becomes a parameter object); `panel_outcome`
  takes the join in place of the reports (D7).
- `engine/sequence.rs`: the panel step joins by the one join, under the
  attempt's plan when it is the first step and the all-fresh plan otherwise
  (D7), and the record on the sequence's terminal fact when the panel step's
  outcome ends it.
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

1. Fold the run. A fold refusal (`acceptance_refusal`'s word) is returned
   as `MemberRetryOutcome::FoldRefused(Refusal)` and nothing is written
   (decision 0047 ruling 2); the CLI handler prints
   `brokkr_view::member_retry_refusal(word)`, S21's text.
2. Read the parked phase's visit into typed records (`carry/facts.rs`; D4,
   D6), whatever names are given, every member named included. A
   `panel-carry.v1` field that does not read is
   `CarryRefusal::RecordUnread`, returned as
   `MemberRetryOutcome::CarryRefused` before the bundle is compiled and
   before any name is resolved, so a drifted bundle beside a malformed
   record reports the record (spec R4), and an all-named retry beside a
   malformed record is refused for it (S24).
3. Compile the pinned bundle through `launch::pinned_bundle`; a refusal is
   `resume`'s own `LaunchError`, the verb's `Err`, printed as `resume`
   prints it.
4. Fill the rest of `CarryFacts` at the command: the selected body's
   `PanelShape`, read from the compiled bundle's body for the parked phase
   under the case its latest `phase/entered` records (D4), and the facts of
   D5; then decide by `carry_plan` over the typed names as given
   (`Names::Given`), which asks the two panel facts and then the names
   (S15 items 3 and 4). The verb decides nothing of its own: it observes the
   body and hands the shape to the rule (S25, C7). A refusal is
   `MemberRetryOutcome::CarryRefused`, whose `Display` the CLI prints byte
   for byte, and nothing is written.
5. Dispose through `disposed` with `args: {"members": [<tags>]}`, returned
   as `MemberRetryOutcome::Disposed(FencedCommandOutcome)`, which the CLI
   prints as it prints the plain verb's. `disposed` takes a
   `Commanded { command, operator, reason, args }` parameter object so the
   bridge and the plain verb pass `{}` and nothing else changes. After the
   command is written only the fence's race refusal can follow, journaled
   as today.

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

`LaunchError` because the compile's refusal is `resume`'s own and already
wraps `EngineError`; `EngineError` gains no variant, so `engine.rs` does not
grow for it. The runtime's tests match the variants (D13).

Alternatives: journaling carry refusals as `operator/rejected` (S2; the
reason word is a closed vocabulary that cannot carry two heads); the fold's
refusal falling through to the plain path (rejected above, a race).

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
command's `args` are read first, before the visit's records: `args`
that do not read as `RetryArgs`, a key other than `members`, a `members`
that is not a list of strings, or an empty `members` list, which the verb
never writes since the plain retry journals `{}`, are
`CarryRefusal::RecordUnread` naming the command by its `operator/commanded`
sequence number and the field `args`, refused through `refuse_carry` (D8),
and never read as the plain retry (S24, C5). Its command's `args.members`
are the attempt's selection, handed to `carry_plan` as `Names::Journaled`
(D4), which matches each tag exactly against the panel's tags and resolves
no name again, after the rule has asked the two panel facts of the body the
engine runs (S25, C7). Every bounded attempt of the effect reads the same
acceptance; the next effect of the phase, after implement and verify, has no
acceptance before it and runs the whole panel. The fold reads nothing of
this (decision 0050 ruling 6; `RunState` is unchanged).

### D4. The rule

```text
carry_plan(facts: &CarryFacts, names: Names<'_>) -> Result<CarryPlan, CarryRefusal>

pub enum PanelShape {
    /// The body is a panel, or a sequence whose first step is a panel.
    Panel { step: Option<String>, tags: Vec<Tag> },
    /// The body is a single seat or a sequence with no panel step.
    NoPanel { phase: String },
    /// The body is a sequence whose first panel step is not its first step.
    StepFollows { step: String, first: String },
}

pub enum Names<'a> {
    /// The verb's names as the operator typed them: bare, or a tag whole.
    Given(&'a [String]),
    /// The drive's `args.members`: tags, matched exactly, resolved no more.
    Journaled(&'a [Tag]),
}
```

`CarryFacts` is what was observed: the selected body's `PanelShape`, read
from the body the observer has (the verb: the compiled bundle's body for the
parked phase under the case its latest `phase/entered` records; the drive:
the body the engine runs for this effect), with a panel's tags in declared
order; the
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
- `AmbiguousMember { name, tags }`; `UnknownMember { name, members }`;
  `MemberTwice { tag }`
- `RecordUnread { record, field, error }`, `record` a closed enum, an
  attempt by its id or, for the accepted retry's `args`, the command by its
  `operator/commanded` sequence number (D3), raised by the journal's reader
  (`carry/facts.rs`, `carry/record.rs`) and never by `carry_plan`
- `Shape { attempt, why }` with `why` one of `NoAttemptEnded { visit }`,
  `Succeeded { park_reason }`, `EndedAtLaterStep { step }`, `NoRecord`
- `Unsettled { members }`
- `NoHead { repo }`; `HeadMoved { ends: Vec<RecordedHead>, now }`
- `TreeNotClean { observations: Vec<Observation>, now }`
- `InputMoved { effects }` (drive time only)

`RecordUnread` is the robustness position's distinction between missing and
malformed evidence: a `panel-carry.v1` field present on an attempt that does
not parse, a `head` end outside the commit-id shape included (D6), refuses
naming the attempt and the field, as a journal that does not fold refuses
before anything is written; the accepted retry's `args` that do not read
refuse the drive the same way, naming the command (D3). The reader raises
it where the records are read, after the fold and before the bundle is
compiled at the verb (D2) and after the digest check at the drive (D7),
whatever names are given, so `carry_plan` takes typed facts alone. Its
first facts are the two panel facts, decided from the shape: `NoPanel`,
then `PanelStepFollows`; only a `PanelShape::Panel` reaches the names. So
item 3's two panel facts have one home, the rule, for both observers, and
the drive asks them before `args.members` are matched (S25, C7). The names
are then three facts in this order, each asked over the names in the order
given and refusing on the first name that fails it: `AmbiguousMember`, a
name that is one member's tag and another member's bare name (in a step
`p` with members `x` and `p:x`, the name `p:x` is `x`'s tag and `p:x`'s
bare name), naming both tags; `UnknownMember`, a name no member has, bare
or as its tag; `MemberTwice`, two names that resolve to one member. Member
names are object keys the compile does not constrain (`parse_panel`,
`bundle/parse.rs`), and `flat_address_collision` (`engine/resume.rs`) refuses only two sites
that flatten to one label, so such a step compiles; the rule fails closed
rather than choose a reading (S25, C8). `Names::Journaled` asks
`AmbiguousMember` never: a journaled tag is matched exactly against the
tags, so `p:x` there is `x`'s tag alone.
The order of the facts, their texts and the lists each refusal names are the
spec's; `Display` is the one home of each text, printed by the verb and
written by the engine (D8). The handler's two argument refusals and S21's
text are not variants: the first are the CLI's, the second the fold's word.

### D5. The four identity facts

- **The head.** `git_head` of the repository `--repo` names at the command,
  and of `self.repo` at the drive. The reading must be forty lowercase hex
  digits, else `NoHead`; each recorded end of the parked attempt and of
  every source attempt is compared as a string, an absent end unequal, a
  present end outside the shape refused by the reader before this (D6), and
  every unequal end is named in the journal's order with the
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
Ends<Option<Head>>, settled_members: Option<BTreeMap<Tag, Value>> }`, each
value any JSON value, an object or not, and
`CarriedEntry { member: Tag, attempt_id }`, serialised with
`deny_unknown_fields` and read back by the same types. `Head` parses forty
lowercase hex digits and nothing else, one shape with no second reading, so
no field is doubly optional (ruling 3): `head` is written absent when
neither end holds one and reads back as two absent ends. The writer stamps
`git_head`'s answer only when it is in that shape, else the end is absent,
as `phase-entered-head.v1`'s reader treats an answer outside its pattern (a
SHA-256 repository answers sixty-four digits and records no head). A `head`
end present outside the shape, or a `settled_members` or `head` beside no
`tree`, is a record the engine never wrote, and the reader refuses it as
`RecordUnread` (D4). `PanelRecord::stamp`
merges the fields into the panel's `effect/indeterminate` or `effect/failed`
payload beside `reason`, `unresolved_members` and the start-failure fields,
which keep their bytes; `settled_members` holds each fresh member's
`AttemptOutcome::Succeeded { result }` value, the same value
`panel_outcome` feeds to the aggregate, whatever JSON the member's driver
reported, since the adapter forwards whatever the seat's result file
parses to and repairs nothing (decision 0001), and is absent when no member
settled. The reader reads any JSON value under a tag, so the engine never
writes a record its own reader refuses, and a carried value that is no
object poisons the aggregate exactly as the same value in a fresh panel
would (S24, C4). The sequence stamps the same record on its own terminal fact when a
panel step's outcome ended the attempt (S6), and nothing on any other
terminal fact. `carried` rides `effect/started` beside `provenance` and
`boundary`, absent when nothing is carried, so a plain retry's attempt is
byte-identical to today's.

`contracts/panel-carry.v1.schema.json` publishes the five fields (`tree`,
`head`, `settled_members`, `carried`, `args.members` for `command: "retry"`),
each pinned to the shape the reader reads, the `head` ends to
`^[0-9a-f]{40}$` as `phase-entered-head.v1` pins its own, each value of
`settled_members` to any JSON value, and `args.members` to a non-empty list
of tags, under the amended
additive rule of `contracts/README.md`, registered in
`crates/brokkr-runtime/tests/frozen_contracts.rs` with its paragraph in the
README. The fold reads none of them. It is minted under 0093 and lands on
`main` only once 0093 is accepted (decision 0082 ruling 1; S13).

### D7. The carrying attempt

In `execute`, after the digest check: the selection is read from the
accepted retry's `args` (D3), whose failure to read is `RecordUnread`
naming the command, refused as below; when the selection names members,
every member named included, the attempts before this effect's request are
read into typed records (a field that does not read is `RecordUnread`,
refused as below), the facts are filled at the drive, the body this effect
runs read to its `PanelShape` before `args.members` are matched, and the
rule decides over `Names::Journaled`, asking `NoPanel` and
`PanelStepFollows` first (D4; S25, C7). A refusal becomes
`refuse_carry` (D8). A plan narrows the body once: `ExecutableBody::Panel`
over the fresh members, or a sequence whose first step is that narrowed
panel. `Dispatch` carries the narrowed body and the plan, so admission,
provisioning, `select_candidates`, the driver label, the session offers,
`provenance`, `boundary` and the member markers cover the fresh members and
no other, with no second rule. Before `effect/started` the attempt's start
readings (tree, head) are taken for every attempt whose body is a panel or a
sequence with a panel step, and held on `Dispatch`; the plan's entries are
written as `carried` on `started`.

`execute_panel` and the sequence's panel step (`sequence.rs`, `panel_step`)
run `member_runs` over the narrowed members, journal their markers, and
join by one function, `CarryPlan::join(reports) -> Joined` in `carry.rs`:
in the panel's declared order, a `Fresh` entry takes its report's
`settled_outcome()` and a `Carried` entry stands as
`AttemptOutcome::Succeeded { result }` from the plan, so no `AttemptReport`
is fabricated for a member that did not run (S4). `panel_outcome(aggregate,
&joined)` is today's rule over the joined `(name, AttemptOutcome)` pairs,
its parameter moved from the reports to the join and its indeterminate,
failed and aggregate arms unchanged, so the result, `inputs` and `notes` are
a fresh panel's bytes (acceptance 1); `Unproven::panel` and
`start_failure_sites` read the fresh reports as today. A plain attempt's
plan is every member `Fresh` (`CarryPlan::fresh(members)`), so the plain
path joins to the fresh reports alone and keeps its bytes; the sequence
joins under the attempt's plan for its first step and under the all-fresh
plan for any later panel step (S10), `SequenceRun` carrying the plan. The
join is written once and called twice (ruling 5); a `Fresh` entry with no
report, or a report for no `Fresh` entry, is a broken engine invariant
(`EngineError::Invariant`), never a member's failure, since the narrowed
body and the join read one plan. The end readings are taken before the
terminal fact and stamped with `joined.settled_fresh()`, the record's
`settled_members`, when the outcome is short of success (D6). A fresh indeterminate parks the attempt and a fresh failure
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
The surfaces that paint participants, `inspect`, `seats`, `watch`, the TUI,
`ui.html` and the console, paint the carried participant (ruling 7);
`brokkr runs` and the fleet readouts paint `brokkr_view::run_rows` and
`fleet_rows`, whose run rows hold no participant, so they show nothing of a
carried verdict and keep only a refused carry's stated park reason (D8;
S24, C3). A carried participant ran no seat, so it has no session,
transcript, cost or provenance of its own, and its `working` is false
(its status is `carried`, never the scan's initial `working`). The readers
of participants that paint no status read it as such:
`brokkr_view::compare::resolutions` (`compare.rs:356-366`) collects the
participants into a map by label, the last of a label standing, so it skips
a carried participant (`carried_from` present) and a label keeps the
resolution of its last participant that ran; muninn's cost rows
(`muninn.rs:321-326`) need no change, since they sum only participants with
no member and a seat's row sums only its members' reported costs
(`own_or_members`), of which a carried member has none;
`select_transcript_participant` (`crates/brokkr-cli/src/lib.rs:1029-1060`)
matches a carried participant neither by label nor among an ambiguous
label's keys, since it has no session to open, while an exact key naming
one reaches `read_local` and is refused `no-reference`, as a participant
with no recorded transcript is today; and `working_checkpoints`
(`dashboard.rs:341`) needs no change, since it keeps only participants at
work and a carried participant is not at work and has no checkpoints; `render.rs` tones `carried` as
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
match variants (ruling 8) and pin each text once. `MemberRetryOutcome` is
matched by variant in the runtime's tests, and the CLI handler's tests
assert that it prints `brokkr_view::member_retry_refusal`'s output for the
word, against the function and not a second copy of the text, so S21's text
has one pin.

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
- **The `Built:` marker** moves with the index row: the file's
  `Built: unbuilt (#787)` becomes `Built: built` as the row's cell does,
  since the index gate renders the cell from the marker (task 12.2).

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
- the two panel facts from the shape: `NoPanel` and `PanelStepFollows`
  before any name, for `Names::Given` and `Names::Journaled` alike;
  mutation: the shape's arms folded into the names step (an unknown name
  then refuses where the phase runs no panel);
- the names in a step `p` with members `x` and `p:x`: `Names::Given`
  with `p:x` refuses `AmbiguousMember` naming `p:x` and `p:p:x`, `x`
  resolves to `p:x` and `p:p:x` to itself, and `Names::Journaled` with the
  tag `p:x` matches `x` alone; mutation: the ambiguity check removed, so
  `p:x` resolves to one member silently;
- every member named: on a dirty tree, a moved head and a park at the
  chief, the rule returns the all-fresh plan and asks nothing below item 4;
  mutation: the short-circuit removed, so the dirty tree (or the park at
  the chief) refuses;
- `RecordUnread` on a `settled_members` that is not an object, on a
  `tree` with a fifth word and on a `head` end outside the shape, raised by
  the reader before any fact; mutation: the parse falling back to "no
  record" or to "no head";
- the join: a plan with one `Carried` entry joins the fresh reports to a
  fresh panel's pairs byte for byte, and the all-fresh plan joins to the
  reports alone; mutation: the carried entry dropped from the join;
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
  written; mutation: the reader pinned to objects.

`crates/brokkr-runtime/src/engine/worktree/tests.rs`: the four readings over
a clean repository, an edited one, a repository whose index is unreadable
and a missing directory, and `git_dirty`'s `false`, `true`, `false`, `true`
over them; mutation: `Refused` folded into `Dirty`. The existing `git_dirty`
test in `engine/tests.rs` (lines 2043 to 2058) holds unchanged.

`crates/brokkr-runtime/src/engine/operator/retry/tests.rs` (the verb, with
the fake driver): a member retry journals `args.members` resolved and in
order; a fold refusal returns `MemberRetryOutcome::FoldRefused` with each
of the three words and writes nothing; a record that does not read beside a
drifted bundle returns `CarryRefused(RecordUnread)` with the bundle
uncompiled, and so does every member named beside a record that does not
read, and a drifted bundle on a sound record returns `resume`'s
`LaunchError`; a carry refusal returns `CarryRefused` and writes nothing;
every member named carries nothing; a phase with no panel and a panel
step after the first step return `CarryRefused(NoPanel)` and
`CarryRefused(PanelStepFollows)` from `carry_plan`; in a step `p` with
members `x` and `p:x`, `--member p:x` returns
`CarryRefused(AmbiguousMember)` naming both tags and writes nothing; the
lost fence after the write is
`Disposed(Rejected)`, journaled as today; no runtime test reads a
`brokkr-view` text; mutation: the fold step skipped (the race refusal
appears where `FoldRefused` should).

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
restart indeterminate carries no record and is passed over; an accepted
retry whose `args` do not read parks on `driver: "none"` naming the
command, and one whose `args` are `{}` runs every member; an all-named
acceptance beside a record that does not read parks naming the attempt and
the field; an accepted retry whose `args.members` meet a body with no
panel, and one whose panel step follows another step, each park on
`driver: "none"` naming the fact. Mutations: the drive-time check removed
(the moved head is carried); the drive's call of the rule over the shape
removed (the two bodies then reach the names with no panel); the narrowing
removed
(four spawns); the end reading removed (`tree` absent).

`crates/brokkr-runtime/tests/frozen_contracts.rs`: the new schema's
registration and bytes; `crates/brokkr-core/src/fold/tests.rs`: a journal
with every new field folds to the state it folds to without them, byte for
byte (the contracts README's machine-checked clause).

`crates/brokkr-cli`, in a new test module beside `select_transcript_participant`
(`src/tests.rs` and `tests/transcript_command.rs` stand at their
baselines): a carried member's label selects the participant of that label
that ran, and an ambiguous label lists no carried key; mutation: the skip
removed. `crates/brokkr-cli/src/tests.rs`: the two argument refusals before every
refusal the handler makes today, and S20's text on `retry` with and without
`--member`, today's on `stop`; the handler's new test module: over a
synthetic journal with a running run, the handler prints
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

`crates/brokkr-view`: in the new module's tests, the scan over `carried`,
the participant's status, class and `carried_from`, no model or cost of its
own and `working` false, the ordinal across two effects, and a plain
retry's view unchanged; in `compare/tests.rs`, `resolutions` over a journal
with a carrying attempt gives each carried label its source participant's
resolution (mutation: the skip removed, so the carried participant's empty
resolution stands); muninn's cost rows and `working_checkpoints` are not
edited and take no test of this change (D9);
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
