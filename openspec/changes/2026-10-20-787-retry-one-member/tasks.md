Requirements served, by the key each task cites:

- **R1** `panel-member-retry`: "A retry names the members that run"
- **R2** `panel-member-retry`: "The carry applies to one panel"
- **R3** `panel-member-retry`: "A verdict is carried only when nothing it judged has changed"
- **R4** `panel-member-retry`: "What is not carried refuses by name, in one order, before anything is written"
- **R5** `panel-member-retry`: "The carrying attempt runs the named members and aggregates as a fresh panel"
- **A1** `panel-attempt-record`: "An attempt that ran its panel and ended short of success records the head and the tree it judged and each verdict that settled in it"
- **A2** `panel-attempt-record`: "An attempt names every verdict it carries"
- **A3** `panel-attempt-record`: "Every readout that paints participants names a carried verdict as carried"
- **A4** `panel-attempt-record`: "A carry refused at drive time parks as a stated indeterminate"
- **W1** `working-tree-reading`: "The working tree is read once, by git's exit and its words"
- **W2** `working-tree-reading`: "The ship gate's decision over the reading does not change"

Ceilings the groups are built against (`quality/file-lines.txt`, decision 0071 ruling 4; a file may grow to the higher of its ceiling and its baseline), measured on this branch: `crates/brokkr-runtime/src/engine.rs` 3,523 and `engine/tests.rs` 6,556 at their baselines and `engine/boundary_tests.rs` 4,207 at its baseline may not grow; `engine/operator.rs` stands at its 800 ceiling and may not grow; `engine/sequence.rs` 714 of 800; `cli_args.rs` 793 of 800; `crates/brokkr-cli/src/tests.rs` 4,592 of 4,593; `crates/brokkr-cli/tests/machine_proof.rs` 3,449 of 3,451; `crates/brokkr-cli/src/render.rs` 774 of 800; `brokkr-view/src/lib.rs` 2,716 may reach its 2,789 baseline, the one figure used for it. Every new test lands in a new module (design D1, D13's opening rule). `engine/tests.rs` and `engine/boundary_tests.rs` are edited only at their direct `execute_panel` calls (task 5.1), in place, each call shorter than before, so both shrink. New and changed functions stay within 100 lines, nesting depth 5 and seven parameters counting `self`. Cyclomatic complexity follows the operator's 2026-09-26 ruling, which the crap ratchet (`quality/crap-baseline.json`) gates: a new function stays within 15, an existing one may change up to 15, and one already over 15 may only shrink or hold, never gain a branch. Of the functions this change touches, `scan_participants` (39), `participants` (21) and `activity_for` (23) in `brokkr-view/src/lib.rs` and `inspect` (17) in `crates/brokkr-cli/src/render.rs` are over it: the view's three gain no branch and the first shrinks (group 8), `inspect` is not edited, since it paints participants through `seats_block` (11), and `Engine::execute` in `engine/dispatch.rs` gains one call and no branch of its own (task 6.2).

## Where these tasks make the design precise

Each point below fills in a detail the design leaves to the build, measured against the code on this branch. None of them changes a decision, a requirement or a text.

- **T1, `resume_with`'s arguments** (D1, D2). `launch::resume` reads `state_of` before `Store::open`, and `Store::open` creates the journal's file and its parent. So `resume_with` takes the state read as well as the store, `resume_with(store, state, request, run)`, and both callers keep today's order: `state_of`, then the store's open. Splitting `resume` at the store alone would leave an empty journal behind a refused state read.
- **T2, how the verb reaches the fold** (D2). `decided` and `Head` are private to `engine::operator`, and `launch.rs` is not a descendant of it. So steps 1 and 2 of D2, the fold and the record read, are an associated function on `Engine` in `engine/operator/retry.rs`. It returns an opaque crate-private value that holds the fold, the events it read and the visit's attempts, or else the refusing outcome. `launch::operator_retry_members` calls it, then calls `resume_with` and the engine's `retry_members`. No private item becomes public, and `Head` stays private.
- **T3, `disposed`'s callers** (D1, D2). Besides `operator_command_racing`, `disposed`'s other caller is `command_stop` (`operator.rs:548`, `conclude`'s stop). The Looper bridge's path (`apply_fenced_*`, `fenced_commanded`) never calls `disposed` and is not edited. `operator.rs` stands at 800, so if the parameter object would push it past the ceiling, `Disposition`, `Commanded` and `disposed` move whole into `engine/operator/disposition.rs` (D1 already moves code for a ceiling in the same way).
- **T4, `engine.rs`'s budget** (D1). The member form's two public types have to be re-exported from `engine.rs` for the CLI to reach them, so the lines `git_dirty`'s spawn frees do not cover what `engine.rs` gains. D1's fallback is therefore taken from the start: `git_head` and `git_dirty` move into `engine/worktree.rs` whole, with `pub use worktree::git_head;` and `use worktree::git_dirty;`, so `crate::git_head` (`brokkr realms`) and every `super::git_head` / `super::git_dirty` import (`decide.rs`, `dispatch.rs`, `sequence.rs`) resolve unchanged.
- **T5, where S20 is pinned** (D11, D13). Adding the member form's field to `OperatorArgs` touches nine struct literals, three of them in `crates/brokkr-cli/src/tests.rs`, which has one line of headroom. S20's longer text also wraps one more line. So D1's opening rule (every new test lands in a new module, and a file at its ceiling may not grow) governs D11's and D13's sentence that puts S20's pin and the argument tests in `tests.rs`. Today's retry pin (`tests.rs:4338-4353`) moves into the handler's new test module, together with S20 on `retry` with and without `--member` and today's text on `stop`. S20 is still pinned once, and `tests.rs` shrinks.
- **T6, the record read once** (D4). The reader `read_visit` in `carry/facts.rs` reads the journal's records, and `Engine::carry_facts` takes the attempts it returned instead of reading the events a second time. At the command the reader runs before the engine exists (D2 step 2), and at the drive it runs after the digest check (D7). So each record is parsed once, and `carry_facts` decides nothing.
- **T7, the command lookup in one place** (D3). Finding the `operator/commanded` that an acceptance's `command_id` names becomes one function, which both `operator_stop_reason` and the carry's scope call (ruling 5). The command word check and the request bound remain the carry's alone.
- **T8, `members: null`** (D3, D13). serde reads `null` as `None` for an `Option` field, which would turn a `members: null` into a plain retry. So `RetryArgs.members` reads through a deserializer that refuses `null`, and the record is unread rather than taken for a plain retry.

## 1. The working-tree reader

*Serves: W1, W2. Design: D10, D1; T4. Decision 0093 ruling 6.*

- [ ] 1.1 [W1] Add `crates/brokkr-runtime/src/engine/worktree.rs`: `TreeReading { Clean, Dirty, Refused, Unrun }`, serialised as the four lowercase words, and `read_tree(repo)`, which runs `git status --porcelain` and reads `Dirty` on any output whatever the exit, `Clean` on exit zero and no output, `Refused` on a non-zero exit and no output, and `Unrun` when git cannot be spawned (a missing directory included). It reads nothing of stderr. `git_head` and `git_dirty` move here whole from `engine.rs` (T4), `git_head`'s doc comment and its `pub` with it. Verify: `cargo build -p brokkr-runtime`.
- [ ] 1.2 [W2] `git_dirty` keeps its name and becomes `matches!(read_tree(repo), TreeReading::Dirty | TreeReading::Unrun)`. `engine.rs` takes these edits and no others: the two functions leave; `mod worktree;`, `mod carry;`, `pub use worktree::git_head;` and `use worktree::git_dirty;` are added; `Selection` gains its `carry` field (task 6.3); and the public re-exports of task 7.3 are added. The file shrinks. `ship_inputs` and `engine/decide.rs` are not edited. Verify: `wc -l crates/brokkr-runtime/src/engine.rs` ≤ 3,523; `git diff --stat main -- crates/brokkr-runtime/src/engine/decide.rs` is empty; `git diff main -- crates/brokkr-runtime/src/engine/tests.rs` holds only the seven `execute_panel` hunks task 5.1 names, none between lines 2347 and 2380, and that file's `wc -l` ≤ 6,556; `cargo test -p brokkr-runtime realms` passes, since it reads `crate::git_head`.
- [ ] 1.3 [W1, W2] Add `engine/worktree/tests.rs` covering:
  - the four readings, over a clean repository, an edited one, a repository with a commit whose index file is corrupted (`.git/index` overwritten with bytes that are no index, so git refuses it at exit 128 printing nothing whatever user the box runs as; a 0o000 index reads fine as root), and a missing directory;
  - a directory that is no repository reads `Refused`;
  - an ignored file edited reads `Clean`;
  - an empty repository reads by what git printed;
  - `git_dirty` is `false`, `true`, `false`, `true` over the first four.

  Verify: `cargo test -p brokkr-runtime worktree`, and the existing `git_dirty` test in `engine/tests.rs` (lines 2043 to 2058) passes unchanged.

## 2. The verb builds the engine as `resume` does

*Serves: R2, R3, R4 (item 3). Design: D1, D2, D5; T1. Proposal S1, S27.*

- [ ] 2.1 [R2, R3] In `launch.rs`, `resume` keeps `state_of` and `Store::open` in today's order and hands both to a new private `resume_with(store, state, request, run)` (T1). `resume_with` holds every gate after the open, in today's order:
  1. `store.manifest`;
  2. the bundle's resolve;
  3. `compile_from_manifest`, refused through `unreproducible`;
  4. `refuse_unboxable`;
  5. `Engine::resume`, with its `ManifestMismatch`, dispatch bounds and charter binding;
  6. `fence_crossings`;
  7. the `secrets_file` and `state` fields.

  `resume`'s behaviour and texts do not move, no gate is written twice, and `Engine::resume` is not edited (ruling 5). Verify: `cargo test -p brokkr-runtime resume` and `cargo test -p brokkr-runtime launch`; the existing resume, `ManifestMismatch` and charter tests pass with their texts unmoved.

## 3. The record and its contract

*Serves: A1, A2, R1 (`args.members`). Design: D6, D3; T8. Proposal S6, S13, S17.*

- [ ] 3.1 [A1, A2, R1] Add `engine/carry.rs` with `engine/carry/record.rs`, holding these types:
  - `Tag`: a newtype spelled from `(step, member)` by the function the member markers use, with `bare()`. The verb's names and `args.members` become `Tag`s as written.
  - `Ends<T> { at_start, at_end }`.
  - `Head`: parsed by `is_commit_id` (`engine.rs:3388`), the one home of the shape `phase-entered-head.v1` pins, reached as a private item of the ancestor module; no second predicate is written (ruling 5). The writer stamps `git_head`'s answer only when `is_commit_id` admits it, and otherwise leaves the end absent, as `phase-entered-head.v1` records none.
  - `PanelSpan { tree: Ends<TreeReading>, head: Ends<Option<Head>> }`, the panel's two readings (task 5.1), and `PanelRecord { span: PanelSpan, settled_members }` with the span flattened, so the record's fields read `tree`, `head` and `settled_members`, `head` absent when neither end holds one. `PanelRecord::stamp` merges it into an `effect/indeterminate` or `effect/failed` payload beside the fields that payload keeps. `settled_members` maps each tag to any JSON value, the member's result value as its driver reported it, whether an object or not. This is the one deliberate `Value` of this change, named in design D6.
  - `CarriedEntry { member, attempt_id }`.
  - `RetryArgs { members: Option<Vec<Tag>> }`: an absent `members` reads as `None`; a present one reads only as a non-empty list of strings, through a deserializer that refuses `null` (T8); an absent `args` or `{}` is the plain retry.

  Each type is `deny_unknown_fields` and is read back by the same types. Two kinds of input that do not parse are `RecordUnread`, never an absent record nor the plain retry (design D3, D6):
  - a record field that does not parse, including a `head` end outside the shape and a `settled_members` or `head` beside no `tree`, names the attempt and the field;
  - `args` that do not parse as `RetryArgs` (a key other than `members`, a `members` that is not a list of strings with `null` included, or an empty list) name the command by its `operator/commanded` sequence number, and the field `args`.

  Verify: `cargo build -p brokkr-runtime`.
- [ ] 3.2 [A2] Add `contracts/panel-carry.v1.schema.json`. It publishes `tree`, `head`, `settled_members`, `carried` and `args.members` (on `command: "retry"`) as optional extension fields at `event_schema: 1`, each pinned to the shape its reader reads:
  - `tree`: ends holding one of the four readings;
  - `head`: ends matching `^[0-9a-f]{40}$`;
  - `settled_members`: an object keyed by tag whose values are any JSON value;
  - `carried`: a list of `member` and `attempt_id` entries;
  - `args.members`: a non-empty list of tags.

  Give it a paragraph and a row in `contracts/README.md` naming 0093 as what minted it, and register it in `crates/brokkr-runtime/tests/frozen_contracts.rs`. No frozen byte moves. Verify: `cargo test -p brokkr-runtime --test it frozen_contracts`; `git diff --stat main -- contracts/` shows only the new file and the README.
- [ ] 3.3 [A2] In `crates/brokkr-core/src/fold/tests.rs` (1,250 lines, under its ceiling), add a test that a journal carrying every new field folds to the same state, byte for byte, as the same journal without them. No production file of `brokkr-core` is edited. Verify: `cargo test -p brokkr-core fold`; `git diff --stat main -- crates/brokkr-core/src` names only the test file.

## 4. The carry rule and the one observer

*Serves: R2, R3, R4, R5. Design: D4, D5, D7; T6. Proposal S5, S7, S9-S12, S15, S17, S18, S27, S32.*

- [ ] 4.1 [R3, R4] Build the reader and the observer in `engine/carry/facts.rs` (T6).
  - **The reader.** `read_visit(events, state, cutoff) -> Result<Vec<PanelAttempt>, CarryRefusal>` is a free function, callable before the engine exists. It returns the visit's attempts before the cut-off as typed `PanelAttempt` records in journal order. Each record holds its terminal kind, `settled_members`, `carried`, `tree` and `head`, its `park_reason` (the `reason` of the first `run/parked` after its terminal fact and before the cut-off, absent where none follows; the one source of `Shape::Succeeded`'s reason at the command and at the drive, never the folded state's), and, for a sequence, the steps whose markers stand. Each record is read once through group 3's types, and a field that does not read raises `CarryRefusal::RecordUnread`. The cut-off is every attempt at the command, and the attempts before this effect's `effect/requested` at the drive.
  - **The facts.** `CarryFacts` holds:
    - the selected body's `PanelShape`, the closed enum of design D4 (a panel's tags in declared order, the phase running no panel, or a panel step after the first step);
    - those attempts, each with its `input_holds`;
    - the head now;
    - the tree now.
  - **The observer.** `Engine::carry_facts(events, state, attempts)` fills them:
    - the shape from `self.bundle`, by the same `selected` call `execute` makes;
    - `input_holds` by `self.seat_input` rebuilt under each attempt's effect id and compared with that effect's `requested_digest`;
    - the head by `git_head(&self.repo)`, kept only in the commit-id shape;
    - the tree by `read_tree(&self.repo)`.

  The observer decides nothing, and `carry_plan` takes typed facts alone. Verify: `cargo build -p brokkr-runtime`.
- [ ] 4.2 [R3, R4, R5] In `engine/carry.rs`, build the refusal, the plan, the join and the rule.
  - **`CarryRefusal`**: a public newtype over a crate-private `thiserror` enum, with one variant per fact of R4 and the closed `why` enums of D4:
    - `NoPanel`;
    - `PanelStepFollows`;
    - `UnknownMember { name, tags }`;
    - `MemberTwice`;
    - `RecordUnread { record, field, error }`, where `record` is an attempt, or the accepted retry's command by its `operator/commanded` sequence number;
    - `Shape { NoAttemptEnded, Succeeded, EndedAtLaterStep, NoRecord }`;
    - `Unsettled`;
    - `NoHead`;
    - `HeadMoved`;
    - `TreeNotClean`;
    - `InputMoved`;
    - `SubjectMoved { attempt, head, tree, admitted }`, the fact the panel alone asks of its own span (design D7), raised by `outcome_under` and never by `carry_plan`.

    Its `Display` is the one home of each text; the texts and the lists each names are R1 to R5's.
  - **`CarryPlan`**: derives `Debug`, since `Selection` does. It holds one `Contribution` per declared tag, `Fresh` or `Carried(SettledVerdict { tag, result, source })`.
    - Its constructor refuses a duplicate, a missing member or a foreign tag.
    - `CarryPlan::fresh(members)` is the all-fresh plan the all-named retry runs under; a plain attempt holds no plan.
    - It carries `subject: Option<Head>`, the head the carry was admitted on: `Some` exactly when an entry is `Carried`, set by the constructor from the facts' head now, and `None` for `CarryPlan::fresh`. The span check reads it and nothing else does.
    - Its views `fresh()` and `carried()` are each in the panel's declared order. `fresh()` is the selection the verb journals as `args.members` (the named tags, or every tag for the all-named short-circuit), and the drive reads it back.
  - **`CarryPlan::join(reports) -> Result<Vec<(String, AttemptOutcome)>, EngineError>`**: the one join of design D7. It yields pairs in declared order: a `Fresh` entry takes its report's `settled_outcome()` under the member's bare name, and a `Carried` entry stands as `AttemptOutcome::Succeeded { result }`. A report twice for one tag, a report for no `Fresh` entry, or a `Fresh` entry with no report is `EngineError::Invariant`.
  - **`outcome_under(aggregate, plan: Option<&CarryPlan>, reports, span: &PanelSpan)`**: the one function both panel callers use. Under `None` it is `panel_outcome(aggregate, reports)`. Under `Some` with `subject: Some(admitted)` it asks the span first, `head.at_start` and `head.at_end` each equal to `admitted` and `tree` `Clean` at both ends, else it is `AttemptOutcome::Indeterminate` with `SubjectMoved`'s `Display` as its reason, before the join; then it is `outcome_over(aggregate, plan.join(reports)?)` (task 5.1). `PanelSpan { tree: Ends<TreeReading>, head: Ends<Option<Head>> }` lives in `carry/record.rs`, `PanelSpan::open(repo)` reading the start and `close(repo)` the end, and `PanelRecord` holds it flattened, so the record's bytes are design D6's.
  - **`settled_members(reports, prefix)`**, in `carry/record.rs`: the free function over the fresh reports that fills the record for both callers.
  - **`carry_plan(facts, names: &[Tag])`**: asks the facts in R4's order, as one call per fact, each helper within ruling 4's ceilings:
    1. the two panel facts from `facts`' `PanelShape`, `NoPanel` then `PanelStepFollows`;
    2. the names, two facts each asked over the names in the order given: `UnknownMember` (an exact match against the panel's tags and nothing else, naming the panel's tags in declared order), then `MemberTwice`. When every member is named, every member is `Fresh` and nothing below is asked;
    3. the parked attempt, found by the three-way test (`effect/succeeded`, a terminal fact recording `tree`, the panel step's marker), and its four shape facts;
    4. the unnamed members;
    5. the head: its shape first, then every unequal end in journal order, with the head now last;
    6. the tree: every non-`clean` observation in journal order, with the tree now last;
    7. the inputs: every source attempt whose `input_holds` is false, each effect named once, in first-named order.

  Verify: `cargo build -p brokkr-runtime`; `cargo clippy -p brokkr-runtime --all-targets -- -D warnings`.
- [ ] 4.3 [R3, R4, R5] Add `engine/carry/tests.rs`. It runs over synthetic journals from a builder and a synthetic panel bundle, and covers the cases of design D13's first list:
  - the incident's plan;
  - the aggregate byte for byte under both `Aggregate` kinds, with a carried `residual` and a carried `fail` kept;
  - every `CarryRefusal` variant by its fact and in R4's order, with the lists each names;
  - the three-way test over every passed-over and every marked kind;
  - the succeeded attempt's park reason from the `run/parked` after it: a rule parks after a succeeded attempt, a plain retry's effect is then refused admission `max_attempts` times and parks `effect <id> failed 2 of 2 attempt(s); …`, and `Shape::Succeeded` names the rule's reason, at the command's cut-off and at the drive's;
  - the read-through;
  - the two panel facts asked before any name;
  - the tags in a step `p` with members `x` and `p:x`: `p:x` matches `x` alone, `p:p:x` matches `p:x` alone, and `x` refuses `UnknownMember` naming `p:p:x` then `p:x`;
  - `fresh()` in declared order over names given out of it;
  - every member named asks nothing below the names, on a dirty tree, a moved head, a moved input and a park at the chief;
  - `RecordUnread` on a non-object `settled_members`, a fifth tree word and a `head` end outside the shape, raised by the reader before any fact;
  - a string-valued `settled_members` entry read back as written and joining to `__member-schema-invalid__`;
  - the join, its all-fresh form and its three invariants;
  - `outcome_under` driving the same verdicts through `None` and through `Some(plan)` to the same bytes;
  - the span check: under a plan whose `subject` is `Some(H0)`, a span at `H0` and `Clean` at both ends joins as before; an end head `H1`, an absent start head, and `Dirty` at either end are each `Indeterminate` with `SubjectMoved`'s sentence naming the attempt, both ends and `H0`, before the join; under `None` and under `CarryPlan::fresh` the same spans change nothing;
  - the scope's two cases (task 6.1);
  - `RetryArgs` refusing an unknown field, a `members` that is not a list of strings (`null` included) and an empty list as `RecordUnread` naming the command, with `{}` and an absent `args` the plain retry.

  Verify: `cargo test -p brokkr-runtime carry::tests`.

## 5. The panel and the sequence write the record

*Serves: A1, R5. Design: D6, D7, D1. Proposal S6, S32.*

- [ ] 5.1 [R5] Narrow `execute_panel`'s signature and route its outcome through the one function.
  - **The signature.** `execute_panel` takes `at: &PanelAt<'_>` beside `members`, `aggregate`, `panel_input`, `deadline` and `selection`: six parameters beside `self`. `PanelAt { effect_id, attempt_id, seat_name, gate }` lives in `panel.rs` with a positional `PanelAt::new`, the `StepAt` precedent. Its `too_many_arguments` expectation is removed, since clippy counts `self` against the threshold of seven. `run_body` (`dispatch.rs`) builds the `PanelAt` from its `Dispatch`.
  - **The twelve direct test calls** are edited in place. Each folds its four scalar arguments into one `&PanelAt::new(…)` line, with its assertions unchanged:
    - `engine/tests.rs` 2107, 2140, 2168, 3607, 3629, 3651 and 6073;
    - `engine/boundary_tests.rs` 3485;
    - `engine/checkpoints/tests.rs` 100;
    - `engine/contention_tests.rs` 1065;
    - `engine/capability_tests.rs` 1778;
    - `engine/capability_tests/call_tests.rs` 419.
  - **The rule.** `panel_outcome(aggregate, reports)` keeps its signature and its five direct test callers. Its three arms move below it into `outcome_over(aggregate, pairs)`, and `panel_outcome` becomes that rule over the reports' settled outcomes.
  - **The body.** `execute_panel`:
    - runs `member_runs` over the members it is handed (the narrowed body's, task 6.3);
    - opens its span, the tree and the head, at its top, before `run_panel` spawns a member, and closes it after the markers, on every attempt;
    - reaches the outcome through `outcome_under(aggregate, selection.carry.as_ref(), reports, &span)?` (group 4), so the result, `inputs` and `notes` are a fresh panel's bytes, no report is fabricated for a carried member, and a carrying attempt whose span moved is indeterminate before the join.

    `Unproven::panel` and `start_failure_sites` read the fresh reports as today. Markers are journaled only for the members that ran. `member_runs` is not edited. `execute_panel` stays within ruling 4's ceilings, with the record's assembly in `carry/record.rs`.

  Verify: `cargo test -p brokkr-runtime panel`, and the six test files above pass with their assertions unchanged, the plain path's bytes with them; `wc -l` of `engine/tests.rs` ≤ 6,556 and of `engine/boundary_tests.rs` ≤ 4,207, both lower than before.
- [ ] 5.2 [A1] When the panel's outcome ends the attempt short of success, the span check's indeterminate included:
  1. take the span the panel opened at its top and closed after the markers (task 5.1), both readings;
  2. stamp `tree`, `head` and, when a member settled fresh, `settled_members` onto the `effect/indeterminate` or `effect/failed`. `settled_members` comes from `settled_members(reports, prefix)` (group 4): each fresh success's result value, whatever JSON its driver reported, keyed by tag.

  The stamped fields sit beside `reason`, `unresolved_members` and the start-failure fields, which keep their bytes. `effect/succeeded` is unchanged. Verify: the drive tests of 6.4 (`tree` and `head` on the terminal fact).
- [ ] 5.3 [A1, R2] In `engine/sequence.rs` (714 of its 800 ceiling):
  - `panel_step` opens its span at its top, before `run_panel` spawns a member, and closes it after the markers, on every attempt.
  - It reaches the outcome through the same `outcome_under`, with the step's span and the plan `run.selection.carry` holds when the step is the first (`at.index == 0`) and `None` for any later panel step, so neither the join nor the span check is written a second time (ruling 5).
  - `StepRun` gains the panel step's closed span and its settled verdicts (`settled_members` over the step's reports under the `<step>:` prefix).
  - `step_result` takes the `StepRun` whole (`settle_step` keeps the copy of its start failures that the fence's refusal needs). It stamps the record, from the `StepRun`'s span and settled verdicts, only on the terminal fact of a failed or indeterminate panel step, and on nothing that a later step, the vocabulary check, the marker's fence or the change check writes.

  `execute_sequence`, its expectation, `SequenceRun` and their thirty-one direct test callers are not edited. If the stamp would pass the file's ceiling, it lives in `carry/record.rs` and the sequence calls it (design, Risks). Verify: `cargo test -p brokkr-runtime sequence`; `wc -l` within 800; `git diff main -- crates/brokkr-runtime/src/engine/sequence.rs` shows no change to `execute_sequence`'s signature.

## 6. The drive

*Serves: R1, R2, R3, R4, R5, A2, A4. Design: D3, D7, D8; T6, T7. Proposal S3, S11, S14, S32.*

- [ ] 6.1 [R1] The selection's scope.
  - **The lookup.** Finding an `operator/commanded` by an acceptance's `command_id` becomes one function in `operator.rs` (`pub(super)`). `operator_stop_reason` (`engine/operator.rs:120-147`) calls it in place of its inline find, and its text does not move (T7).
  - **The search.** At `execute`, find the retry that caused this effect: the latest `operator/accepted` before its `effect/requested` whose accepted command, found by that lookup, is a `retry`, with no other `effect/requested` between.
  - **The parse.** Parse that command's `args` as `RetryArgs` first, before the visit's records. No acceptance, an absent `args` or `{}` selects the whole panel. `args` that do not parse (a key other than `members`, a `members` that is not a list of strings, `null` included, or an empty list, none of which the verb writes) are `RecordUnread`, naming the command by its `operator/commanded` sequence number and the field `args`. They are refused through `refuse_carry` (6.2), never read as the whole panel.

  Verify: in `carry::tests`, an acceptance before an earlier effect's request selects nothing for a later effect, and an accepted retry followed by another command's acceptance before the request still selects the retry's members; 6.4's unreadable-args case; `cargo test -p brokkr-runtime operator` passes unchanged.
- [ ] 6.2 [R3, R4, A4] In `engine/dispatch.rs`, after the digest check and before `effect/started`, make the drive-time check. It is one `Engine` method in `carry.rs`, `carry_selection(events, state, effect_id) -> Result<Option<CarryPlan>, CarryRefusal>`, so `execute` gains one match and stays within ruling 4. With members selected, every member named included, it:
  1. reads the attempts through `read_visit` with the cut-off at this effect's request; a field that does not read is `RecordUnread`, refused as below;
  2. fills `CarryFacts` through `Engine::carry_facts`: the `PanelShape` of the body this effect runs, the head and tree of `self.repo`, and each attempt's `input_holds`;
  3. decides by `carry_plan` over the journaled tags, which asks `NoPanel` and `PanelStepFollows` before it matches a tag.

  A refusal is `refuse_carry`, beside `refuse_changed_input`: an attempt on `driver: "none"` with no `carried`, ended `effect/indeterminate` with the refusal's `Display` as its `reason`, no marker and no JSON. `brokkr-core` is not edited; the fold's `close` arm reads the reason as a stated reason. Verify: 6.4's moved-head case.
- [ ] 6.3 [R5, A2] A plan narrows the body once, at the seat's clone `execute` already takes: the panel's `members` are kept to the plan's `fresh()`, or the sequence's first step's panel is kept to them.
  - `selected` then yields the narrowed `ExecutableBody`, and `Dispatch` holds it as it holds every body. Admission, provisioning, `select_candidates`, the driver label, the session offers, `provenance`, `boundary` and the markers cover the fresh members alone.
  - `plan_attempt` takes the plan and writes its `carried()` entries on `started` in declared order, absent when nothing is carried, so a plain retry's `effect/started` is byte-identical to today's.
  - `plan_attempt` sets `selection.carry = Some(plan)` on the `Selection` it returns: the plan's one home during the drive. `Selection` gains `carry: Option<CarryPlan>` in `engine.rs`, `None` by `Default`. A first attempt, a plain retry, an acceptance with `{}` and an effect with no acceptance before it hold `None`.

  No field is added to `Engine` or `Dispatch`; the start readings are the panel's own (tasks 5.1, 5.3). Verify: 6.4.
- [ ] 6.4 [R2, R3, R5, A1, A2, A4] Add `engine/carry/drive_tests.rs`, using the fake driver, covering:
  - **Who runs.** Only the named member spawns (counted). `provenance`, `boundary`, the driver label, one offer, one admission claim and one marker cover it alone.
  - **What the attempt records.** `carried` names the source by `attempt_id`. The aggregate equals a fresh panel's over the same verdicts byte for byte, with a carried `residual` kept. The start and end readings stand on the terminal fact.
  - **The moved head.** A head moved between acceptance and drive parks on `driver: "none"` with the sentence as `effect/indeterminate.reason`. The folded park reason is `effect <id> indeterminate: <sentence>`, with `run/parked.evidence` `{}`.
  - **Inputs and files.** On a seat that declares a secret, an acceptance given under `--secrets-file /keys/ops.env` and driven by `brokkr resume --secrets-file /keys/other.env` parks naming the source effect, the store path being a byte of its rebuilt input (`input.rs:83-86`). Result files deleted after the park change nothing.
  - **The bounded retry.** After a start failure it carries the same entries, and it does not carry a verdict that settled in the failed attempt.
  - **Old journals and restarts.** A journal from before this change refuses `NoAttemptEnded`. The restart indeterminate carries no record and is passed over.
  - **Retry arguments.** An accepted retry whose `args` carry an unknown key, a `members` that is not a list of strings (`null` included) or an empty list parks on `driver: "none"`, naming the command by its sequence number. One whose `args` are `{}` runs every member.
  - **Unreadable records.** An all-named acceptance beside a record that does not read parks, naming the attempt and the field.
  - **The panel facts.** An accepted retry whose `args.members` meet a body with no panel, and one whose panel step follows another step, each park on `driver: "none"` naming the fact (`NoPanel`, `PanelStepFollows`), with no member spawned.
  - **Sequences.** The sequence's first step carries under `positions:` tags, and `chief` runs on the aggregate. A sequence whose panel step ends it carries `tree` and `head` from the step's own two ends; one ended at the chief carries none.
  - **A chain fallback between the attempts.** The named member's driver fails to start in the carrying effect's first attempt, under a chain of two candidates. The second attempt selects the second candidate, the four facts hold, and it carries the same verdicts from the same source attempts. Its `provenance` names the fresh member under the second candidate alone.
  - **The carrying attempt's own span.** In a synthetic all-work panel, the fresh member's fake driver commits inside the carrying attempt: the attempt ends `effect/indeterminate` with `SubjectMoved`'s sentence naming its start at `H0`, its end at `H1` and the admitted `H0`, no `effect/succeeded` is journaled, the terminal fact records `head` at both ends, `tree` and the fresh verdict under `settled_members`, the stated park reason carries the sentence, the next member retry is refused at the head fact naming the ends unequal to the head now, and the plain retry is admitted. In a carrying all-gate panel, the fresh member's fake driver edits a file and leaves it: the same, naming the tree `dirty` at the end. In a carrying all-gate panel, the fresh member's fake driver commits: `GATE-MOVED-HEAD` in the terminal fact's place as today, no record, and the attempt passed over by the next member retry. A plain retry's attempt in the all-work panel whose member commits: the aggregate as today.
  - **The plain retry.** Its `effect/started` matches today's bytes.

  Verify: `cargo test -p brokkr-runtime carry::drive_tests`.

## 7. The verb

*Serves: R1, R2, R4. Design: D2, D11; T1, T2, T3, T5. Proposal S2, S15, S18, S20, S21, S27.*

- [ ] 7.1 [R4] In `brokkr-view`, add `member_retry_refusal(word)` to `commands.rs` beside `operator_refusal`. It returns `refused operator retry --member (<word>): the run is not in a state a retry can apply to; nothing was journaled`, and is re-exported on the existing `pub use commands::{…}` line. Its text is pinned once in `commands/tests.rs`, for each of the three words. Verify: `cargo test -p brokkr-view commands`.
- [ ] 7.2 [R1, R4] `disposed` takes a `Commanded { command, operator, reason, args }` parameter object and the deciding fold `(RunState, Head)` the caller read, in place of the fold it reads for itself before the command's write today (`folded` at `operator.rs:375`: the `asked` state and the head the command is appended after). The fence loop after the write is not edited: it folds again per attempt under `FENCE_ATTEMPTS`, since the acceptance is fenced on the head the command's own write moved, and re-folds after a lost fence as today.
  - **Its callers.** It has two today, `operator_command_racing` and `command_stop` (T3). Each folds through `folded` and passes `args: {}`, so the plain verb's and `conclude`'s bytes, refusals and fence do not move. The bridge's path does not call `disposed` and is not edited.
  - **Visibility.** `retry.rs`, a child module of `operator`, reaches `decided`, `Head` and `disposed` as private items of its parent, so no visibility widens.
  - **The ceiling.** If `operator.rs` would pass 800, `Disposition`, `Commanded` and `disposed` move whole into `engine/operator/disposition.rs`. They become `pub(super)` there, and `operator.rs` gains one `mod` line.

  Verify: `cargo test -p brokkr-runtime operator`, with the existing operator and conclude tests passing unchanged; `wc -l crates/brokkr-runtime/src/engine/operator.rs` ≤ 800.
- [ ] 7.3 [R1, R2, R3, R4] Add `engine/operator/retry.rs` (`operator.rs` stays ≤ 800), holding:
  - `pub enum MemberRetryOutcome { FoldRefused(brokkr_core::Refusal), CarryRefused(CarryRefusal), Disposed(FencedCommandOutcome) }`.
  - An associated function on `Engine` that runs D2's steps 1 and 2 on the store before the engine exists (T2):
    1. Fold once through `decided`. A refusal of `acceptance_refusal` is `FoldRefused` with the fold's word, and nothing is written.
    2. Read the visit's records through `read_visit`. A field that does not read is `CarryRefused(RecordUnread)`.

    It returns either that refusing outcome, or an opaque crate-private value holding the fold, the events it read and the attempts.
  - `Engine::retry_members(&mut self, read, names, operator, reason)`, which runs steps 4 and 5:
    - **Step 4.** Observe through `Engine::carry_facts` at the command's cut-off, against the engine's repository and `seat_input`, then decide by `carry_plan` over the names as `Tag`s. A refusal is `CarryRefused`.
    - **Step 5.** Dispose through `disposed`, on the engine's store, with the fold of step 1 and `args.members` set to the returned plan's `fresh()` tags, in the panel's declared order whatever order the names were given in. The result is `Disposed`.
  - `Engine::retry_members_racing(&mut self, read, names, operator, reason, before_command)`, the twin `retry_members` delegates to with a no-op, as `operator_command_racing` is to the plain verb: `before_command: impl FnMut(&mut Store)` is called once, after step 4's facts are decided and before step 5's `disposed` writes `operator/commanded`, the window in which a peer's acceptance makes the member form decide on a fold the journal has moved past. Production passes a no-op; the tests pass a store append.

  In `launch.rs`, `operator_retry_members(request, run, names, operator, reason) -> Result<MemberRetryOutcome, LaunchError>` runs the steps in order:
  1. `state_of`;
  2. `Store::open`;
  3. the associated function; a refusing outcome is returned as `Ok`, with the engine unbuilt;
  4. `resume_with(store, state, request, run)`, whose refusal is the `Err`, `resume`'s own (step 3 of D2);
  5. the engine's `retry_members`.

  `MemberRetryOutcome` and `CarryRefusal` are re-exported from `engine.rs` and `lib.rs`. They are the public items this change adds, beside `operator_retry_members`. The verb decides no fact of its own and resolves and orders no name. The runtime prints nothing and names no `brokkr-view` text. Verify: 7.4; `cargo clippy -p brokkr-runtime --all-targets -- -D warnings` (no `private_interfaces` warning).
- [ ] 7.4 [R1, R2, R3, R4] Add `engine/operator/retry/tests.rs`, using the fake driver and matching `MemberRetryOutcome` by variant, covering the cases of design D13's verb list:
  - `args.members` journaled in declared order over names given out of it;
  - each fold word returning `FoldRefused` and writing nothing;
  - a record that does not read beside a drifted bundle returning `CarryRefused(RecordUnread)` with the engine unbuilt, and every member named beside such a record giving the same;
  - a drifted bundle on a sound record returning `Err` with `resume`'s text, before any panel fact;
  - a run driven with `brokkr resume --secrets-file /keys/ops.env` on a seat that declares a secret, retried by member with `--secrets-file /keys/other.env`, returning `CarryRefused(InputMoved)` naming the source effect, and writing nothing;
  - a run driven with `brokkr resume --secrets-file` on a seat that declares a secret: the member form given the same store admitted, and given none returning `CarryRefused(InputMoved)` naming the source effect;
  - the carrying effect's own result paths leaving the command admitted;
  - a phase with no panel returning `CarryRefused(NoPanel)`, and a panel step after the first step returning `CarryRefused(PanelStepFollows)`;
  - `--member correctness` on the review sequence returning `CarryRefused(UnknownMember)` naming the four `positions:` tags, and `--member positions:correctness` admitted, journaling that tag;
  - in a step `p` with members `x` and `p:x`, `--member p:x` journaling `p:x`, and `--member x` refused;
  - a carry refusal returning `CarryRefused` and writing nothing;
  - every member named carrying nothing, a sequence parked at `chief` included;
  - a peer's plain retry, its `operator/commanded` and `operator/accepted`, appended through `retry_members_racing`'s `before_command` hook, after the facts and before the command is written, giving `Disposed(Rejected)` with `lost_fence`, journaled, with no `operator/accepted` carrying the tags (the case task 10.1's `disposed` mutation must fail);
  - the same peer appended through `disposed`'s existing `between` hook, after the command is written, giving `Disposed(Rejected)` with `lost_fence` as the plain retry's race does today, reached by calling `disposed` directly from this module, a descendant of `operator`, with the member form's `Commanded`, the deciding fold the test read and a `between` that appends the peer; under 10.1's nearest-mutation clause a fence mutation in `disposed`'s loop binds it.

  No test here reads a `brokkr-view` text. Verify: `cargo test -p brokkr-runtime operator::retry`.
- [ ] 7.5 [R1] The CLI.
  - **The arguments.** Add `crates/brokkr-cli/src/cli_args/member_retry.rs` with `MemberRetryArgs { member: Vec<String>, repo: Option<PathBuf>, delivery: DeliveryArgs }`: `--member` repeatable, `--repo`, and `DeliveryArgs` (`--bundle`, `--recipe`, `--recipes-dir`, `--secrets-file`, the four `resume` reads through `request`) flattened whole, so the four have one home (ruling 5); `DeliveryArgs` gains `Default` and the struct derives it. It is flattened into `OperatorArgs` as one field, with its `mod` line in `cli_args.rs`.
  - **The nine `OperatorArgs` struct literals** gain `member: MemberRetryArgs::default()`, one line each, with no assertion edited:
    - `verbs/acknowledge/tests.rs` 22, 192 and 202;
    - `verbs/tests.rs` 129, 157 and 216;
    - `src/tests.rs` 362, 4208 and 4315.
  - **The handler order.** In `verbs/delivery.rs`'s `operator` handler, before the `supersede` branch:
    1. ask `--member` on any command word but `retry`;
    2. ask any member-form argument without `--member`, naming every one typed in one refusal;
    3. then make today's refusals in today's order. The supersede-only flags' text on `retry` reads S20's sentence, and on every other word today's.
  - **The member form** is its own function beside the handler, so `operator` stays within ruling 4. It resolves the run and builds the `LaunchRequest` through `resolved`, `resume`'s path, from `delivery` whole, so `--secrets-file` is read and defaulted as `resume` reads it (design D5). It calls `launch::operator_retry_members` and prints the outcome:
    - `FoldRefused(word)` as `brokkr_view::member_retry_refusal(word.word())`, with the failed exit;
    - `CarryRefused(refusal)` as the refusal's `Display` alone, the sentence byte for byte, with the failed exit;
    - `Disposed(outcome)` as the plain verb prints a `FencedCommandOutcome`;
    - an `Err` as `resume`'s errors are printed.

  Verify: `cargo build -p brokkr-cli`; `wc -l crates/brokkr-cli/src/cli_args.rs` ≤ 800.
- [ ] 7.6 [R1] Add `crates/brokkr-cli/src/verbs/delivery/member_tests.rs`, the handler's new test module, with `#[cfg(test)] mod member_tests;` in `delivery.rs`.
  - **S20's pin moves here (T5).** The retry refusal pin at `src/tests.rs:4338-4353` leaves that file, and this module pins S20 on `retry` with and without `--member`, and today's text on `stop`.
  - **The argument-order tests:**
    - `stop --member x --findings 1`;
    - `bogus --member x`;
    - `retry --member x --findings 1`;
    - `retry --repo …`;
    - `retry --recipe … --repo …`, named once;
    - `retry --secrets-file …` without `--member`, refused naming it;
    - `stop --findings 1` keeping today's text;
    - `--repo` defaulting to the folder's root from a subdirectory.
  - **The outcome's painting.** Over a synthetic journal with a running run, the handler prints `brokkr_view::member_retry_refusal`'s output for the fold's word, asserted against the function and pinned nowhere else. A carry refusal prints its `Display` alone.

  Verify: `cargo test -p brokkr-cli member_tests`, and `cargo test -p brokkr-cli` for the edited literals; `wc -l crates/brokkr-cli/src/tests.rs` ≤ 4,593, lower than before.

## 8. The readouts

*Serves: A3, A4. Design: D9. Proposal S14, S27.*

- [ ] 8.1 [A3] Add `crates/brokkr-view/src/started.rs` as a child module of `lib.rs` (`mod started;`), so it reaches `Scan`, `Build` and `ensure` as its parent's private items. Move `BoundaryEntry` and `boundary_entries` there from `lib.rs`; add `CarriedEntry { member, attempt_id }` and `carried_entries`, which drops an entry lacking either key as `boundary_entries` drops a malformed one; and add `sites(scan, slot, effect_id, payload)`, the one reader of an `effect/started`'s per-site lists: it clears `boundary`, `stamp` and `carried` on every part of the effect, then reads `provenance`, `boundary` and `carried`, each entry ensuring its member's `Build` under the carrying effect and setting its field. `Build` gains `carried: Option<CarriedEntry>`. The scan's `EffectStarted` arm calls `sites` once in place of its three loops, so `scan_participants` (cyclomatic 39, over the ceiling) loses three branches there and gains none. Verify: `cargo build -p brokkr-view`; `lib.rs` within its 2,789 baseline; the crap ratchet shows `scan_participants` below 39.
- [ ] 8.2 [A3] In `started.rs`, add the three helpers that decide from the entry, so `participants` (cyclomatic 21, over the ceiling) and `activity_for` (23) gain no branch:
  - `status_of(part, effect) -> (&'static str, &'static str)`: the derivation the scan's last loop makes today (the effect's open or terminal state, then a member's marker), moved whole, with one arm before them: a part with a `carried` entry is `("carried", "completed")` whatever its effect's state, since nothing of it ran. The loop becomes one assignment per part, so `scan_participants` loses five more branches.
  - `carried_from(events, scan, part) -> Option<CarriedFrom { attempt_id, ordinal }>`: `ordinal` is the source attempt's place among the phase's `effect/started`s in journal order, every visit counted, and `attempt_id` the journal's name for the attempt, which the JSON readouts join on.
  - `activity(events, scan, part, member_count, carried_from) -> Activity`: a carried participant's cell is the one sentence `carried from <phase> attempt <ordinal>`, with the attempt id as its note; every other participant's is `activity_for`'s, unchanged.

  `Participant` gains `carried_from`, and `participants` fills `carried_from` and `activity` by one call each. A carried participant has no model, session, cost or turns under the carrying effect, and its `working` is false. `VIEW_VERSION` moves from 23 to 24, and its pin moves with it. `crates/brokkr-cli/src/render.rs` tones `carried` as good (`tone`, cyclomatic 5). Verify: `cargo test -p brokkr-view`; the crap ratchet shows `scan_participants` and `participants` at or below 39 and 21.
- [ ] 8.3 [A3, A4] In `started.rs`'s test module, cover:
  - the scan over `carried`;
  - status, class and `carried_from`;
  - a carried participant under an effect still open is `carried`, not `working`;
  - the activity cell's sentence, with the attempt id as its note;
  - no model or cost of its own, and `working` false;
  - the ordinal across two effects;
  - `carried` across two attempts of one effect: the second start's entries replace the first's, and a start without `carried` clears it;
  - a plain retry's view, unchanged;
  - a drive-time refused carry's stated reason painted as every stated park is, with a `GATE-MOVED-HEAD` park unchanged.

  Verify: `cargo test -p brokkr-view started`.
- [ ] 8.4 [A3] The surfaces that paint participants, `inspect` and `seats` (through `seats_block` in `render.rs`, ≤ 800 lines), `watch`, the TUI (`crates/brokkr-cli/src/tui/seats.rs`), `ui.html` and the console (`extensions/dsh/brokkr-console/lib/client.js`), paint the status word and the activity cell as they do today, so each shows `carried` and the sentence, leading with the ordinal, and composes no line of its own; `carried_from` rides the JSON they read. The painters that key a tone on the status word gain `carried` as good: `render.rs`'s `tone`, and the console's palette where it keys on the word; a painter keyed on `status_class` (`ui.html`'s badge, the TUI's class tones) needs nothing. `inspect` (cyclomatic 17, over the ceiling) is not edited. `brokkr runs` and the fleet readouts, whose run rows hold no participant, are not edited and show nothing of a carried verdict. Each Rust painter's new lines are covered by a test in a module beside it (the exact-coverage gate), and the console's by a case in its `tests/readouts.test.js`. Verify: `cargo test -p brokkr-cli`; `node --test` in `extensions/dsh/brokkr-console`, recorded pending in the notes where the box refuses node.
- [ ] 8.5 [A3] The readers that paint no status:
  - `brokkr_view::compare::resolutions` (`compare.rs:360`) skips a participant whose `carried_from` is present, pinned in `crates/brokkr-view/src/compare/tests.rs`: a carried label keeps its source participant's resolution.
  - `select_transcript_participant` (`crates/brokkr-cli/src/lib.rs:1029`) is not edited and takes no task: the transcript verb lists a carried participant's key as it lists every participant's, and refuses `no-reference` when that key is chosen.
  - muninn's cost rows (`muninn.rs:321-326`) are not edited and take no task: they sum only participants with no member, and a seat's row sums only its members' reported costs.
  - `working_checkpoints` (`dashboard.rs:341`) is not edited and takes no task: it keeps only participants at work (design D9).

  Verify: `cargo test -p brokkr-view compare`; `git diff --stat main -- crates/brokkr-cli/src/lib.rs` is empty.

## 9. The acceptance, end to end

*Serves: R1, R3, R5, A2, A3 (the issue's three acceptance lines). Design: D13.*

- [ ] 9.1 Add `crates/brokkr-cli/tests/machine_proof/panel_carry.rs`, with `mod panel_carry;` after `mod output_edge;` in `machine_proof.rs` (3,449 of 3,451). It runs over a workdir the proof `git init`s and commits in, with synthetic fixtures only, and proves:
  - **The same aggregate.** A panel with one indeterminate member, retried by member, reaches the aggregate a full re-run reaches on the same verdicts.
  - **The moved head.** A carried verdict is refused at the command once the head moves.
  - **Naming.** Every carried verdict is named in the journal, and in `inspect` and `seats` with its ordinal.
  - **The plain retry.** Its `effect/started` is byte-identical to today's.
  - **The routed residual** (decision 0093 ruling 3; R5's scenario `A residual from the retried member routes as today`):
    1. Under the proof's reforging table (`REFORGING_POLICY`, decision 0022), the review seat is made a panel of `correctness` and `security` (`make_panel`).
    2. `security` vanishes in the first review attempt, and the run parks.
    3. `brokkr operator retry --member security` is accepted. The resumed attempt carries `correctness` and runs `security`, which reports `residual` with `has_security_residual` true at `low`.
    4. `REVIEW-REFORGE` routes the run to implement, then verify.
    5. The re-entered review's attempt runs both members fresh with no `carried`, both clean, and the run ships, its rule ids read in order.

  Verify: `cargo test -p brokkr-cli --test it machine_proof::panel_carry`.

## 10. Binding

*Serves: every requirement above (decision 0071 ruling 9). Design: D13.*

- [ ] 10.1 For each new test, make the compiling mutation D13 names (or the nearest one that removes the behaviour), watch the test fail, restore, and record the mutation and the failing test in the result's notes. The mutations:
  - **The reader and the records:**
    - `Refused` folded into `Dirty`;
    - the parse falling back to "no record";
    - the reader's `RecordUnread` folded to "no record";
    - the `settled_members` reader pinned to objects;
    - the all-named retry exempted from the record reader.
  - **Retry arguments and scope:**
    - `deny_unknown_fields` removed;
    - the empty-list check on `args.members` dropped;
    - the `null` refusal dropped, so serde's `Option` reads `members: null` as the plain retry (T8);
    - the "no request between" condition dropped;
    - the `retry` command-word check dropped from the acceptance's lookup.
  - **The plan and the join:**
    - the plan constructor accepting a missing member;
    - the join over fresh reports alone;
    - the join's duplicate check removed;
    - the carried entries dropped from the join (the `None` and `Some` paths then differ);
    - the join emitting the reports' order in place of the plan's.
  - **The rule:**
    - each refusal fact removed, and two facts swapped;
    - the marker dropped from the three-way test;
    - the folded state's park reason used for `Shape::Succeeded` in place of the `run/parked` after the attempt (the later admission park's reason is then named);
    - the entry's source replaced by the carrying attempt;
    - the all-named short-circuit removed (the dirty tree or the park at the chief then refuses);
    - the tag match widened to the bare name (`p:x` then matches two members);
    - `fresh()` in the order given rather than the panel's declared order.
  - **The verb:**
    - the fold step skipped;
    - `input_holds` dropped from the command's facts (a member retry naming another secrets store than the source effect's is admitted);
    - the source rebuilt under the carrying effect's id (every carry refuses);
    - `disposed` folding for itself after the facts (with the peer appended through `before_command`, the race's word becomes `run_not_awaiting_operator`);
    - the store dropped from the member form's request (a run driven with `--secrets-file` then refuses `InputMoved` at the command).
  - **The drive:**
    - the drive-time check removed;
    - the drive's call of `carry_plan` over the body's `PanelShape` removed (the body with no panel and the following panel step then reach the names);
    - the narrowing removed;
    - the end reading removed;
    - the span check removed from `outcome_under` (a fresh work member's commit inside the carrying attempt then publishes the mixed aggregate as `effect/succeeded`);
    - the stamp applied whatever step ended a sequence (the chief's park then carries a record);
    - the fresh member's candidate compared between the attempts as an identity fact (the chain fallback then refuses).
  - **The readouts:**
    - the `carried` scan removed;
    - the clearing of `carried` at a later `effect/started` removed;
    - the carried arm of `status_of` removed (a carried participant under an open effect reads `working`);
    - `activity` yielding `activity_for`'s cell for a carried part;
    - the carried skip removed from `resolutions`.
  - **The CLI:**
    - the handler printing a line of its own in place of the view's;
    - S21's and S20's texts changed.

  Verify: each mutation's failing test is named in the notes.

## 11. Docs and the API

*Serves: R1, A3. Design: Migration, D12. Proposal Impact.*

- [ ] 11.1 Name the member form, the tag it takes, the store a run driven with `--secrets-file` is retried with, and the carry in `docs/guides/quickstart.md`'s operator-commands paragraph (not #802's parks list) and in `ARCHITECTURE.md`'s panel, operator-verb and ship-gate prose (not the state diagram). Regenerate `docs/reference/cli.md` by the command `cli_reference_tests` prints. Verify: `cargo test -p brokkr-cli cli_reference`.
- [ ] 11.2 Re-measure `quality/public-api/brokkr-view.txt` and `brokkr-runtime.txt` on the pinned nightly (`rust-nightly-version.txt`). `brokkr-runtime` gains `MemberRetryOutcome`, `CarryRefusal` and `launch::operator_retry_members`, and `brokkr-core.txt` must not move. If the box cannot run the nightly, record the measurement pending for the operator, whose ruling the pull request cites. Verify: `bash quality/ratchet.sh api`, or the pending record.

## 12. Gates and the archive

*Serves: every requirement above. House rules; decision 0081.*

- [ ] 12.1 Run the gates:
  - `cargo fmt --all -- --check`;
  - `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
  - `cargo test --all-features --locked`, per crate (re-run a `brokkr-protocol` or `brokkr-runtime` timing test alone before calling it a failure);
  - `cargo run --locked -p brokkr-cli -- compile --bundle recipes/self` and `recipes/verify`;
  - the ratchets (`quality/ratchet.sh`);
  - the exact-coverage gate;
  - the console's `node --test`.

  Record each in the notes as pending where the box refuses it; never lower a gate. Verify: each exits clean, or its output is in the notes.
- [ ] 12.2 Move decision 0093 to built in both places the gate reads together: the file's header marker `Built: unbuilt (#787)` becomes `Built: built`, and the index row's cell `unbuilt (#787)` becomes `built`, since the gate renders the cell from the marker and refuses a disagreement. Keep `Status: proposed` in the header and the row, and keep the 0091 and 0092 gap rows until each lands. Verify: `cargo test -p brokkr-cli --test it decisions_index`.
- [ ] 12.3 Fold the three ADDED deltas into `openspec/specs/panel-member-retry/spec.md`, `panel-attempt-record/spec.md` and `working-tree-reading/spec.md`, each with a `## Purpose` and its requirements. Then `git mv` the change into `openspec/changes/archive/`, last. Verify: `cargo test -p brokkr-cli --test it openspec_retention`, and the commit is made with a clean tree.
