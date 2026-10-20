## ADDED Requirements

### Requirement: An attempt that ran its panel and ended short of success records the tree it judged and each verdict that settled in it

A panel attempt whose terminal fact is `effect/indeterminate` or `effect/failed`, written by the panel whose outcome ended it, a seat's panel body or the panel step whose outcome ended a sequence, SHALL record `tree`, and SHALL record `settled_members` when a member settled in it. `tree` SHALL hold `at_start` and `at_end`, each `clean` or `dirty`, the working tree as `git_dirty` read it when the attempt started and when it ended: `clean` when `git status --porcelain` exited zero and printed nothing, `dirty` otherwise, a spawn that failed and an exit that was not zero included. `settled_members` SHALL hold, for each member whose settled outcome in that attempt is a success, the member's result object as its driver reported it and the aggregate reads it, keyed by the member's tag as the markers and `carried` spell it: `<member>` for a seat's panel, `<step>:<member>` for a panel step. It SHALL NOT hold a verdict the attempt only carried, and SHALL be absent when no member settled in the attempt. Both fields SHALL be absent on every attempt that ran no panel: a single-driver attempt, a sequence ended by a step that is not a panel, the `GATE-MOVED-HEAD` indeterminate a gate writes in a terminal fact's place, and an attempt that started on no driver; those payloads keep their bytes. `unresolved_members` SHALL keep its bare-name keys. `effect/succeeded` SHALL NOT change. The fold SHALL NOT read either field.

#### Scenario: Three settled, one not

- **WHEN** a four-member panel ends indeterminate because one member ran to no verdict, on a tree clean at both ends
- **THEN** its `effect/indeterminate` carries `settled_members` with the three members' results under their bare names, `tree` reading `{"at_start": "clean", "at_end": "clean"}`, and `unresolved_members` as #403 records it when any member is not proven over

#### Scenario: A panel step's keys

- **WHEN** the review sequence's `positions` step ends the attempt indeterminate with three members settled
- **THEN** `settled_members` keys them `positions:adversarial`, `positions:security` and `positions:spec-compliance`, as the attempt's markers key them

#### Scenario: A panel that failed by one member

- **WHEN** a panel attempt fails because one member's driver failed to start and the others settled
- **THEN** its `effect/failed` carries `settled_members` with the settled members' results and `tree`, beside `error` and the start-failure fields it carries today

#### Scenario: A tree edited while the panel judged

- **WHEN** a file under the operated repository is edited and left uncommitted while a panel attempt runs, and the attempt ends indeterminate
- **THEN** its terminal fact's `tree` reads `at_end` as `dirty`

#### Scenario: Git refuses status while the panel runs

- **WHEN** the operated repository's index file is made unreadable while a panel attempt runs, so that `git rev-parse HEAD` answers and `git status --porcelain` exits 128 printing nothing, and the attempt ends indeterminate
- **THEN** its terminal fact's `tree` reads `at_end` as `dirty`

#### Scenario: A carrying attempt that settles one member

- **WHEN** a member retry names `correctness` and `security`, carries the two others, `security` settles and `correctness` runs to no verdict
- **THEN** the terminal fact's `settled_members` holds `security` alone, never the carried verdicts, and `tree` records both ends

#### Scenario: The same member stuck twice

- **WHEN** a member retry names `correctness` alone, carries three verdicts, and `correctness` runs to no verdict again
- **THEN** the terminal fact carries `tree` and no `settled_members`, and the three verdicts stand only where its `effect/started.carried` names them

#### Scenario: An all-indeterminate panel

- **WHEN** every member of a panel attempt runs to no verdict
- **THEN** its terminal fact carries `tree` and no `settled_members`

#### Scenario: A single seat's attempt

- **WHEN** a single-driver attempt ends indeterminate or failed
- **THEN** its terminal fact carries neither `settled_members` nor `tree`, byte for byte as before this change

#### Scenario: A sequence ended at the chief

- **WHEN** the review sequence's `positions` step aggregates and `chief` then ends the attempt indeterminate
- **THEN** the terminal fact carries neither `settled_members` nor `tree`, and the positions' verdicts stand only inside the aggregate the step checkpointed

#### Scenario: An attempt that started on no driver

- **WHEN** a carry is refused at drive time, or a rebuilt input is refused
- **THEN** the attempt's terminal fact carries neither `settled_members` nor `tree`

### Requirement: An attempt names every verdict it carries

`effect/started` SHALL carry `carried` when the attempt carries verdicts: one entry per carried member, in the panel's declared order, with `member`, the member's tag, and `attempt_id`, the attempt the verdict settled in. An entry SHALL name the attempt the verdict settled in, never an attempt that only carried it: a verdict read through a carrying attempt's own entry keeps that entry's `attempt_id`. The field SHALL be absent on an attempt that carries nothing. A carried member SHALL appear in no other record or plan of the carrying attempt: not in `provenance`, `boundary`, the `driver` label's count, the session offers, the admission claim, nor as a `panel-member-finished` marker. `carried`, `settled_members`, `tree` and `args.members` SHALL be published as `contracts/panel-carry.v1.schema.json` under the amended rule of `contracts/README.md`: optional, absent by default, never read by the fold, moving no event type, no seat-record version and no manifest digest; the file is minted under decision 0092 and SHALL land on `main` only once 0092 is accepted (decision 0082 ruling 1).

#### Scenario: Every carried verdict is named in the journal

- **WHEN** a member retry's attempt carries `adversarial`, `security` and `spec-compliance` from the parked attempt
- **THEN** its `effect/started` holds `carried` with those three tags, each naming the parked attempt's `attempt_id`, its `provenance` and `boundary` name `correctness` alone, and the attempt's markers name `correctness` alone

#### Scenario: A second park

- **WHEN** the first member retry names `correctness` and `security`, its attempt carries `adversarial` and `spec-compliance` from the first attempt, `security` settles there and `correctness` again does not, and the operator retries naming `correctness` alone
- **THEN** the third attempt's `carried` names `adversarial` and `spec-compliance` by the first attempt, the one they settled in, and `security` by the second

#### Scenario: A contract written beside the frozen ones

- **WHEN** `contracts/panel-carry.v1.schema.json` is added
- **THEN** every frozen contract's bytes are unchanged, the run manifest's digest is unchanged, a run that carries nothing journals an `effect/started` byte-identical to today's, and the contract's row names 0092 as what minted it

### Requirement: Every readout names a carried verdict as carried

`brokkr-view` SHALL derive, from the attempt's `carried`, a participant for each carried member at status `carried` with a completed class, carrying `carried_from`, the attempt its verdict settled in, and SHALL derive it once; `brokkr runs`, `inspect`, `seats`, `watch`, the TUI, `ui.html`, the fleet readouts and the console SHALL paint it and SHALL derive nothing of their own. A carried member's participant SHALL show no model, session or cost of its own for the carrying attempt. The attempt's aggregate notes SHALL stay the fresh panel's. `VIEW_VERSION` SHALL move with the participant's new field.

#### Scenario: The run view after a member retry

- **WHEN** the run view reads a journal where the retry's attempt carried three verdicts and ran one member to a verdict
- **THEN** the three carried members' participants stand at status `carried` naming the parked attempt, with no model of their own, the fresh member's at `succeeded`, and the panel's notes read one line per member as a fresh panel's would

#### Scenario: A plain retry's view

- **WHEN** the run view reads a journal where the retry's attempt carried nothing
- **THEN** no participant stands at `carried`, and every readout reads as it did before this change

### Requirement: A carry refused at drive time is read as its own park reason

The `effect/indeterminate` that ends a refused carry and the `run/parked` the engine then writes SHALL both carry, as `reason`, the marker `PANEL-CARRY-REFUSED`, one space, and the refusal as compact JSON, written and read through one codec in `brokkr-core` beside `gate_moved_head`; `run/parked.evidence` SHALL be `{}`. `ParkReason::read` SHALL decode a reason that opens with the marker and whose evidence reads as the refusal to a `ParkReason` of its own carrying the typed refusal, and the fold's `close` arm SHALL decode the indeterminate's reason the same way, so the folded state holds the typed refusal whether the journal ends at the indeterminate or at the park. A marked reason whose evidence does not read as the refusal SHALL fold as a stated reason with its text whole. The reason's journaled bytes and its sentence SHALL be told apart: the sentence SHALL be the typed refusal's own text, pinned once in `brokkr-core`, the same text `brokkr operator retry --member` refuses with, naming the fact that refused: the recorded head and the head found; the tree, and the observation at which it was dirty, the parked attempt's start or end, a source attempt's start or end naming that attempt, the command, or the drive; the effect whose input no longer rebuilds to its digest; or the shape fact or members the verb would have named. `Summary.park_reason`, the fleet's summary row (cut to decision 0089's bound, the whole kept), `brokkr compare`, muninn and every readout that shows a park reason SHALL paint that sentence. The `GATE-MOVED-HEAD` park, its `run/parked` bytes and every `Stated` reason SHALL read as they do today.

#### Scenario: The park folds through run/parked

- **WHEN** a member retry's drive ended `PANEL-CARRY-REFUSED` because the head moved, and the journal is folded past the `run/parked` the engine wrote
- **THEN** the state's park reason carries the typed refusal with both heads, `run/parked.reason` opens with the marker and carries the same JSON the `effect/indeterminate` carries, `run/parked.evidence` is `{}`, and `brokkr inspect`'s `park_reason` is the one sentence naming the recorded head and the head found

#### Scenario: The readouts after a drive-time refusal

- **WHEN** that journal is read by `brokkr runs`, `inspect`, `compare`, muninn, the fleet readouts and the console
- **THEN** each shows the one sentence naming both heads, and the journal keeps the marker and its evidence on both events

#### Scenario: The sentence is the verb's

- **WHEN** the head moved before the command and `brokkr operator retry --member correctness` refuses at the terminal
- **THEN** its refusal text is the sentence a drive-time park for the same fact paints, byte for byte

#### Scenario: A marked reason whose evidence does not read

- **WHEN** an `effect/indeterminate` reason reads `PANEL-CARRY-REFUSED` followed by text that is not the refusal's JSON
- **THEN** the run folds to a park whose reason is stated, its text whole, and every readout paints that text

#### Scenario: Every other park reads as before

- **WHEN** a run parks by `GATE-MOVED-HEAD`, by a rule, or by an attempt's stated indeterminate reason
- **THEN** its park reason, its `run/parked` bytes and every readout of it are byte for byte as before this change
